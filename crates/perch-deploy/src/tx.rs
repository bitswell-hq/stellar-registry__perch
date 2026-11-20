//! The write path: build one InvokeHostFunction transaction, simulate it in
//! recording mode to discover the smart account's auth entry, check that
//! entry's whole invocation tree against the call this tool means to make,
//! sign it with the OZ digest, then re-simulate in enforcing mode (the signed
//! auth runs verifier + policy cross-calls, so the first simulation's
//! footprint is insufficient) and sign/send/poll.
//!
//! Both simulations go to a [`Simulate`] (in `perch-deploy`, the local
//! soroban-sim over an explicit state source, [`crate::sim`]); only the
//! signed envelope goes to the [`Broadcast`] RPC, which then answers the
//! confirmation polls. A dry run stops after the tree check: it signs
//! nothing and reports the unsigned simulation's estimate.

use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use stellar_xdr::{
    DecoratedSignature, HostFunction, InvokeContractArgs, InvokeHostFunctionOp, Limits, Memo,
    MuxedAccount, Operation, OperationBody, Preconditions, ReadXdr, ScSymbol, ScVal,
    SequenceNumber, Signature, SignatureHint, SorobanAuthorizationEntry,
    SorobanAuthorizedInvocation, SorobanTransactionData, Transaction, TransactionEnvelope,
    TransactionExt, TransactionSignaturePayload, TransactionSignaturePayloadTaggedTransaction,
    TransactionV1Envelope, Uint256, WriteXdr,
};

use crate::authz::{self, IntendedAuth};
use crate::keys::SignKey;
use crate::rpc::{Broadcast, Simulate, TxStatus};
use crate::{auth, scv};

/// One contract invocation: `contract.func(args)`.
pub struct InvokeSpec {
    pub contract: String,
    pub func: String,
    pub args: Vec<ScVal>,
}

/// How the smart account's auth entry is satisfied.
pub enum AuthMode {
    /// OZ `Signer::External`: sign the rule-id-bound digest, verified by the
    /// given verifier contract.
    External { verifier: String },
    /// CAP-0071 `Signer::Delegated`: the signing key's own G-account is the
    /// delegate; the host authenticates its classic signature inside the
    /// account's single `AddressWithDelegates` entry.
    Delegated,
}

/// How to satisfy the smart account's auth entry: in which mode, selecting
/// which context rule, for which account address, over which tree. The
/// signing key is `run_signed`'s single key — it signs both the auth entry
/// and the fee envelope.
pub struct AuthSpec {
    pub mode: AuthMode,
    pub rule_id: u32,
    /// The smart account expected to be the (single) address credential.
    pub account: String,
    /// The exact sub-invocations the account authorizes beneath the call.
    /// Empty for every call this tool makes: a contract the account calls
    /// itself is authorized as the direct invoker, not through the entry.
    pub sub_invocations: Vec<SorobanAuthorizedInvocation>,
}

#[derive(Debug)]
pub struct Submitted {
    pub tx_hash: String,
    pub ledger: u32,
}

pub fn build_tx(source_pubkey: [u8; 32], seq: i64, spec: &InvokeSpec) -> Result<Transaction> {
    let op = Operation {
        source_account: None,
        body: OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
            host_function: HostFunction::InvokeContract(InvokeContractArgs {
                contract_address: scv::contract(&spec.contract)?,
                function_name: ScSymbol(spec.func.as_str().try_into()?),
                args: spec.args.clone().try_into()?,
            }),
            auth: Default::default(),
        }),
    };
    Ok(Transaction {
        source_account: MuxedAccount::Ed25519(Uint256(source_pubkey)),
        fee: 100,
        seq_num: SequenceNumber(seq),
        cond: Preconditions::None,
        memo: Memo::None,
        operations: vec![op].try_into()?,
        ext: TransactionExt::V0,
    })
}

pub fn set_auth(tx: &mut Transaction, auth: Vec<SorobanAuthorizationEntry>) -> Result<()> {
    let ops = tx.operations.to_vec();
    let mut op = ops.into_iter().next().context("transaction has no op")?;
    let OperationBody::InvokeHostFunction(ref mut ihf) = op.body else {
        bail!("operation is not InvokeHostFunction");
    };
    ihf.auth = auth.try_into()?;
    tx.operations = vec![op].try_into()?;
    Ok(())
}

pub fn envelope_b64(tx: &Transaction) -> Result<String> {
    let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
        tx: tx.clone(),
        signatures: Default::default(),
    });
    Ok(envelope.to_xdr_base64(Limits::none())?)
}

/// Sign the envelope with the fee payer. Returns (envelope base64, tx hash hex)
/// — the hash is sha256 of the TransactionSignaturePayload, i.e. the network's
/// transaction id.
pub fn sign_envelope(
    tx: &Transaction,
    passphrase: &str,
    payer: &dyn SignKey,
) -> Result<(String, String)> {
    let payload = TransactionSignaturePayload {
        network_id: stellar_xdr::Hash(auth::network_id(passphrase)),
        tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(tx.clone()),
    };
    let hash: [u8; 32] = Sha256::digest(payload.to_xdr(Limits::none())?).into();
    let sig = payer.sign(&hash)?;
    let public = payer.public();
    let decorated = DecoratedSignature {
        hint: SignatureHint(public[28..32].try_into().expect("4 bytes")),
        signature: Signature(sig.to_vec().try_into()?),
    };
    let envelope = TransactionEnvelope::Tx(TransactionV1Envelope {
        tx: tx.clone(),
        signatures: vec![decorated].try_into()?,
    });
    Ok((envelope.to_xdr_base64(Limits::none())?, hex::encode(hash)))
}

/// The full signed write flow. `broadcast` is `None` for a dry run, which
/// stops after the authorization tree is checked and never calls
/// `signer.sign`; otherwise the signed transaction is sent there and polled
/// to SUCCESS. Every read and both simulations go to `sim`.
///
/// `prior_seq` is the sequence number of a transaction this process already
/// sent from the same fee payer ([`run_upload`]'s), which `sim`'s state does
/// not reflect; this transaction takes the next one. `None` reads the fee
/// payer's sequence from `sim`.
pub fn run_signed(
    sim: &dyn Simulate,
    passphrase: &str,
    signer: &dyn SignKey,
    auth_spec: &AuthSpec,
    spec: &InvokeSpec,
    prior_seq: Option<i64>,
    broadcast: Option<&dyn Broadcast>,
) -> Result<Option<Submitted>> {
    let payer = signer.public();
    let seq = match prior_seq {
        Some(seq) => seq,
        None => sim
            .account_seq(&payer)
            .with_context(|| format!("fee payer {}", signer.account()))?,
    };
    let mut tx = build_tx(payer, seq + 1, spec)?;

    let sim1 = sim.simulate(&envelope_b64(&tx)?)?;
    if let Some(e) = sim1.error {
        bail!("simulation failed: {e}");
    }

    // The tree this tool means to authorize, from its own inputs, against
    // what simulation asks for: nothing is signed unless they are equal.
    let intended = IntendedAuth::new(
        &auth_spec.account,
        &spec.contract,
        &spec.func,
        &spec.args,
        auth_spec.sub_invocations.clone(),
    )?;
    let mut entry = authz::checked_entry(&sim1.auth, &intended)?;

    let Some(broadcast) = broadcast else {
        dry_run_report(&entry, &sim1)?;
        return Ok(None);
    };

    let expiration = sim1.latest_ledger + 120;
    match &auth_spec.mode {
        AuthMode::External { verifier } => auth::sign_auth_entry(
            &mut entry,
            passphrase,
            expiration,
            signer,
            verifier,
            auth_spec.rule_id,
        )?,
        AuthMode::Delegated => auth::sign_delegated_auth_entry(
            &mut entry,
            passphrase,
            expiration,
            signer,
            auth_spec.rule_id,
        )?,
    }
    set_auth(&mut tx, vec![entry])?;

    // Re-simulate with the signed auth: verifier + policy cross-calls inflate
    // the footprint, so only THIS simulation's resources are trustworthy.
    let sim2 = sim.simulate(&envelope_b64(&tx)?)?;
    if let Some(e) = sim2.error {
        bail!("re-simulation with signed auth failed: {e}");
    }
    let td = sim2
        .transaction_data
        .context("re-simulation returned no transactionData")?;
    let soroban_data = SorobanTransactionData::from_xdr_base64(&td, Limits::none())?;
    // 15% headroom over the simulated resource fee, rounded up.
    let fee = 100u64 + (sim2.min_resource_fee * 115).div_ceil(100);
    tx.fee = u32::try_from(fee).context("computed fee overflows u32")?;
    tx.ext = TransactionExt::V1(soroban_data);

    Ok(Some(submit(broadcast, &tx, passphrase, signer)?))
}

/// What a dry run can say without a signature: the checked authorization,
/// and the unsigned simulation's footprint and fee, which leave out the
/// signature check's own calls.
fn dry_run_report(entry: &SorobanAuthorizationEntry, sim1: &crate::rpc::Simulation) -> Result<()> {
    println!(
        "authorization tree checked: the smart account authorizes exactly {} and {} \
         sub-invocations",
        authz::describe(&entry.root_invocation),
        entry.root_invocation.sub_invocations.len()
    );
    if let Some(td) = &sim1.transaction_data {
        let data = SorobanTransactionData::from_xdr_base64(td, Limits::none())?;
        println!("unsigned footprint: {:?}", data.resources.footprint);
        println!("unsigned instructions: {}", data.resources.instructions);
    }
    println!(
        "unsigned min resource fee: {} stroops",
        sim1.min_resource_fee
    );
    println!(
        "note: this estimate is from the unsigned (recording) simulation. The live run \
         re-simulates with the signed entry, which adds the verifier and policy calls the \
         signature check makes, so its footprint, instructions, and fee differ from these. A \
         dry run holds no signing key, so it cannot produce that signed estimate."
    );
    println!("DRY RUN OK (nothing signed)");
    Ok(())
}

/// Sign a fully-prepared tx (fee + soroban data already set), send it, and poll
/// to SUCCESS. Shared by the smart-account invoke flow (`run_signed`) and the
/// plain wasm upload (`run_upload`). The broadcast RPC sees only this signed
/// envelope and the hash polled for.
pub fn submit(
    broadcast: &dyn Broadcast,
    tx: &Transaction,
    passphrase: &str,
    payer: &dyn SignKey,
) -> Result<Submitted> {
    let (envelope, hash) = sign_envelope(tx, passphrase, payer)?;
    let accepted = broadcast.send(&envelope)?;
    if accepted != hash {
        bail!("the broadcast RPC answered tx hash {accepted}, not this transaction's {hash}");
    }

    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        match broadcast.get_transaction(&hash)? {
            TxStatus::Success { ledger, .. } => {
                return Ok(Submitted {
                    tx_hash: hash,
                    ledger,
                })
            }
            TxStatus::Failed { result_xdr } => bail!("transaction {hash} FAILED: {result_xdr}"),
            TxStatus::NotFound => {
                if Instant::now() >= deadline {
                    bail!("timed out waiting for transaction {hash}");
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    }
}

/// Build a wasm-upload tx: `HostFunction::UploadContractWasm(bytes)`. Upload is
/// authorized by the source-account signature alone (no Soroban auth entry), so
/// it carries no `auth` — unlike the smart-account invoke in `build_tx`.
fn build_upload_tx(source_pubkey: [u8; 32], seq: i64, wasm: &[u8]) -> Result<Transaction> {
    let op = Operation {
        source_account: None,
        body: OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
            host_function: HostFunction::UploadContractWasm(wasm.to_vec().try_into()?),
            auth: Default::default(),
        }),
    };
    Ok(Transaction {
        source_account: MuxedAccount::Ed25519(Uint256(source_pubkey)),
        fee: 100,
        seq_num: SequenceNumber(seq),
        cond: Preconditions::None,
        memo: Memo::None,
        operations: vec![op].try_into()?,
        ext: TransactionExt::V0,
    })
}

/// Install wasm bytes on-chain via `UploadContractWasm`, paid + signed by
/// `payer`. Needed for wasm too large to inline in the registry `publish` invoke
/// (the whole binary would overflow the transaction envelope): the caller
/// uploads here, then records name→hash with `publish_hash`. `publish_hash` does
/// NOT re-check the upload, so this MUST precede it. Idempotent — re-uploading
/// identical bytes is a cheap no-op success. Returns the sequence number the
/// sent upload consumed, for the `run_signed` that follows it. A dry run
/// (`broadcast` = `None`) simulates only, signs and sends nothing, and
/// returns `None`.
pub fn run_upload(
    sim: &dyn Simulate,
    passphrase: &str,
    payer: &dyn SignKey,
    wasm: &[u8],
    broadcast: Option<&dyn Broadcast>,
) -> Result<Option<i64>> {
    let hash: [u8; 32] = Sha256::digest(wasm).into();
    let seq = sim
        .account_seq(&payer.public())
        .with_context(|| format!("fee payer {}", payer.account()))?
        + 1;
    let mut tx = build_upload_tx(payer.public(), seq, wasm)?;

    // One simulation suffices: with no auth entry to sign, the footprint doesn't
    // change under signing (unlike run_signed's verifier/policy cross-calls).
    let sim = sim.simulate(&envelope_b64(&tx)?)?;
    if let Some(e) = sim.error {
        bail!("upload simulation failed: {e}");
    }
    // An upload is authorized by the envelope signature alone; any auth
    // entry here would be authorized by that signature too.
    if !sim.auth.is_empty() {
        bail!(
            "upload simulation asks for {} authorization entries; an upload needs none",
            sim.auth.len()
        );
    }
    let td = sim
        .transaction_data
        .context("upload simulation returned no transactionData")?;
    let soroban_data = SorobanTransactionData::from_xdr_base64(&td, Limits::none())?;
    let fee = 100u64 + (sim.min_resource_fee * 115).div_ceil(100);
    tx.fee = u32::try_from(fee).context("computed upload fee overflows u32")?;
    tx.ext = TransactionExt::V1(soroban_data.clone());

    let Some(broadcast) = broadcast else {
        println!("upload footprint: {:?}", soroban_data.resources.footprint);
        println!(
            "upload fee: {} stroops (min resource fee {})",
            tx.fee, sim.min_resource_fee
        );
        println!("DRY RUN OK (upload {}, nothing signed)", hex::encode(hash));
        return Ok(None);
    };

    let submitted = submit(broadcast, &tx, passphrase, payer)?;
    println!(
        "uploaded wasm {} in tx {} at ledger {}",
        hex::encode(hash),
        submitted.tx_hash,
        submitted.ledger
    );
    Ok(Some(seq))
}

/// Outcome of a read-only simulation: a value, or a *contract* error (which
/// callers like the publish preflight treat as data, not failure). `code` is
/// the `#N` from the host's `Error(Contract, #N)` rendering — callers that
/// branch on specific contract errors MUST match the code, not just the fact
/// of a trap.
pub enum ReadOutcome {
    Value(ScVal),
    ContractError { code: Option<u32>, message: String },
}

/// Extract N from the host error rendering `… Error(Contract, #N) …`.
pub fn contract_error_code(message: &str) -> Option<u32> {
    let (_, rest) = message.split_once("Error(Contract, #")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Simulate `contract.func(args)` read-only. The source is the all-zero
/// G-account: simulation needs a well-formed envelope, not a funded account.
pub fn simulate_read(
    sim: &dyn Simulate,
    contract: &str,
    func: &str,
    args: Vec<ScVal>,
) -> Result<ReadOutcome> {
    let spec = InvokeSpec {
        contract: contract.to_string(),
        func: func.to_string(),
        args,
    };
    let tx = build_tx([0u8; 32], 0, &spec)?;
    let sim = sim.simulate(&envelope_b64(&tx)?)?;
    if let Some(e) = sim.error {
        if e.contains("Error(Contract") {
            return Ok(ReadOutcome::ContractError {
                code: contract_error_code(&e),
                message: e,
            });
        }
        bail!("read simulation of {func} failed: {e}");
    }
    let xdr = sim
        .result_xdr
        .with_context(|| format!("read simulation of {func} returned no result"))?;
    Ok(ReadOutcome::Value(ScVal::from_xdr_base64(
        &xdr,
        Limits::none(),
    )?))
}
