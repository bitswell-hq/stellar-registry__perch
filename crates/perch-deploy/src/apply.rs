//! `apply`: submit a policy document's JSON bytes to the account's
//! `apply_doc` in one signed transaction. The account does all the real
//! verification on-chain — parse, validate, network binding, compile,
//! anti-brick check, atomic whole-rule-set swap — this command only
//! pre-flights locally for a better error message, prints the canonical
//! `doc_hash` the reviewer approved, and signs the admin's approval
//! (PERCH_ADMIN_KEY selecting the `admin` rule, found by name in the
//! account's `configuration()` — the one thing the stock stellar CLI cannot
//! sign yet). The apply names the revision it read as `expected_revision`,
//! so it never overwrites a change made after that read.
//!
//! The account's configuration is read, and both simulations run, in the
//! local simulator over the configured state source; the broadcast RPC only
//! receives the signed transaction and answers the confirmation polls.

use anyhow::{bail, Context, Result};
use stellar_xdr::{ScBytes, ScVal};

use crate::keys::SignKey;
use crate::rpc::{Broadcast, Simulate};
use crate::tx::{AuthSpec, InvokeSpec};
use crate::verify::read_configuration;
use crate::{scv, tx};

/// The account's `admin` rule (self-admin, by name, in its
/// `configuration()` snapshot): its id, the verifier of its
/// `Signer::External` whose key is `pubkey`, and the snapshot's revision.
/// Rule ids move when a rule is replaced, so the id is read, never assumed.
fn onchain_admin(
    sim: &dyn Simulate,
    account: &str,
    pubkey: &[u8; 32],
) -> Result<(u32, String, u64)> {
    let (revision, rules) = read_configuration(sim, account)
        .context("read the account's configuration — is the account deployed?")?;
    let me = scv::address(account)?;
    let name = scv::string("admin")?;
    let admin = rules.iter().find(|(_, r)| {
        scv::map_get(r, "name") == Some(&name)
            && scv::map_get(r, "recovery") == Some(&ScVal::Bool(false))
            && matches!(scv::map_get(r, "context_type"),
                Some(ScVal::Vec(Some(ctx))) if ctx.get(1) == Some(&me))
    });
    let Some((id, rule)) = admin else {
        bail!("the account has no self-admin rule named admin");
    };
    let ScVal::Vec(Some(signers)) =
        scv::map_get(rule, "signers").context("the admin rule has no signers field")?
    else {
        bail!("the admin rule's signers field is not a vec");
    };
    for signer in signers.iter() {
        // Signer::External = Vec[Sym("External"), Address(verifier), Bytes(key)]
        let ScVal::Vec(Some(parts)) = signer else {
            continue;
        };
        let mut it = parts.iter();
        let (Some(ScVal::Symbol(tag)), Some(verifier), Some(ScVal::Bytes(key))) =
            (it.next(), it.next(), it.next())
        else {
            continue;
        };
        if tag.to_utf8_string_lossy() == "External" && key.as_slice() == pubkey {
            return Ok((*id, scv::address_to_string(verifier)?, revision));
        }
    }
    bail!("the signing key matches no External signer on the on-chain admin rule")
}

/// Apply the document at `doc_path` to `account`, signed by `key` selecting
/// the account's admin rule. `broadcast` is `None` for a dry run, which
/// signs nothing.
pub fn run(
    sim: &dyn Simulate,
    passphrase: &str,
    key: &dyn SignKey,
    account: &str,
    doc_path: &std::path::Path,
    broadcast: Option<&dyn Broadcast>,
) -> Result<()> {
    let doc_json = std::fs::read_to_string(doc_path)
        .with_context(|| format!("read {}", doc_path.display()))?;

    // Local pre-flight for operator-grade error messages; the account
    // re-verifies everything fail-closed on-chain.
    let doc = perch_ir::from_json(&doc_json)
        .map_err(|e| anyhow::anyhow!("document does not parse: {e:?}"))?;
    if let Err(errors) = perch_ir::validate(&doc) {
        bail!("document is invalid: {errors:?}");
    }
    match &doc.network {
        Some(net) if net == passphrase => {}
        Some(net) => bail!(
            "document names network {net:?} but this connection targets {passphrase:?} — \
             the account would reject it (WrongNetwork)"
        ),
        None => bail!("document names no network; apply_doc requires one (WrongNetwork)"),
    }
    let doc_hash = perch_ir::doc_hash_hex(&doc);
    println!("canonical doc_hash: {doc_hash}");

    let (rule_id, verifier, revision) = onchain_admin(sim, account, &key.public())?;

    // The document bytes, an `approval_valid_until` of 0, and the revision
    // the admin rule was read at: the account resolves the compiler +
    // interpreter itself through its pinned constructorless registry, the
    // freshness bound is read only for a `Protected` recovery
    // reconfiguration, which needs recorded approvals this tool does not
    // collect, and `expected_revision` refuses the apply (`StaleRevision`)
    // if the account changed since this read.
    let args = vec![
        ScVal::Bytes(ScBytes(
            doc_json
                .clone()
                .into_bytes()
                .try_into()
                .context("document too large for an ScVal bytes value")?,
        )),
        ScVal::U32(0),
        ScVal::U64(revision),
    ];

    let spec = InvokeSpec {
        contract: account.to_string(),
        func: "apply_doc".to_string(),
        args,
    };
    let auth_spec = AuthSpec {
        mode: tx::AuthMode::External { verifier },
        rule_id,
        account: account.to_string(),
        // apply_doc's own calls (compiler, recovery hooks, pool) are made by
        // the account itself and need no entry of their own.
        sub_invocations: vec![],
    };
    if let Some(submitted) = tx::run_signed(sim, passphrase, key, &auth_spec, &spec, broadcast)? {
        println!(
            "applied document {doc_hash} over revision {revision} in tx {} at ledger {}",
            submitted.tx_hash, submitted.ledger
        );
        println!(
            "confirm with the stock CLI: stellar contract invoke --id {account} -- \
             applied_doc_hash"
        );
    }
    Ok(())
}
