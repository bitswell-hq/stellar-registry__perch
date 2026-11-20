//! The signing flow's boundaries, against recording fakes: the simulated
//! authorization tree is checked before anything is signed, a dry run signs
//! nothing, and a live run sends the broadcast RPC only the signed envelope
//! and its confirmation polls.

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};

use perch_deploy::keys::{SeedKey, SignKey};
use perch_deploy::publish::{self, PublishArgs};
use perch_deploy::rpc::{Broadcast, Rpc};
use perch_deploy::scv;
use perch_deploy::tx::{run_signed, run_upload, AuthMode, AuthSpec, InvokeSpec};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use stellar_xdr::{
    AccountEntry, AccountEntryExt, AccountId, HostFunction, InvokeContractArgs, LedgerEntryData,
    LedgerFootprint, Limits, OperationBody, PublicKey, ReadXdr, ScSymbol, ScVal, SequenceNumber,
    SorobanAddressCredentials, SorobanAuthorizationEntry, SorobanAuthorizedFunction,
    SorobanAuthorizedInvocation, SorobanCredentials, SorobanResources, SorobanTransactionData,
    SorobanTransactionDataExt, Thresholds, TransactionEnvelope, TransactionSignaturePayload,
    TransactionSignaturePayloadTaggedTransaction, Uint256, WriteXdr,
};
use support::fake_rpc::FakeRpc;

const PASSPHRASE: &str = "Test SDF Network ; September 2015";

fn c(n: u8) -> String {
    stellar_strkey::Contract([n; 32])
        .to_string()
        .as_str()
        .into()
}

fn account() -> String {
    c(1)
}

/// Counts every signature it makes.
struct CountingKey {
    key: SeedKey,
    signed: AtomicUsize,
}

impl CountingKey {
    fn new() -> Self {
        Self {
            key: SeedKey::from_seed([9u8; 32]),
            signed: AtomicUsize::new(0),
        }
    }
    fn signatures(&self) -> usize {
        self.signed.load(Ordering::SeqCst)
    }
}

impl SignKey for CountingKey {
    fn public(&self) -> [u8; 32] {
        self.key.public
    }
    fn sign(&self, message: &[u8]) -> anyhow::Result<[u8; 64]> {
        self.signed.fetch_add(1, Ordering::SeqCst);
        self.key.sign(message)
    }
}

fn account_entry(pk: [u8; 32]) -> String {
    LedgerEntryData::Account(AccountEntry {
        account_id: AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(pk))),
        balance: 100_000_000,
        seq_num: SequenceNumber(41),
        num_sub_entries: 0,
        inflation_dest: None,
        flags: 0,
        home_domain: Default::default(),
        thresholds: Thresholds([1, 0, 0, 0]),
        signers: Default::default(),
        ext: AccountEntryExt::V0,
    })
    .to_xdr_base64(Limits::none())
    .unwrap()
}

fn invocation(
    contract: &str,
    func: &str,
    args: Vec<ScVal>,
    subs: Vec<SorobanAuthorizedInvocation>,
) -> SorobanAuthorizedInvocation {
    SorobanAuthorizedInvocation {
        function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
            contract_address: scv::contract(contract).unwrap(),
            function_name: ScSymbol(func.try_into().unwrap()),
            args: args.try_into().unwrap(),
        }),
        sub_invocations: subs.try_into().unwrap(),
    }
}

fn address_entry(address: &str, inv: SorobanAuthorizedInvocation) -> String {
    SorobanAuthorizationEntry {
        credentials: SorobanCredentials::Address(SorobanAddressCredentials {
            address: scv::contract(address).unwrap(),
            nonce: 7,
            signature_expiration_ledger: 0,
            signature: ScVal::Void,
        }),
        root_invocation: inv,
    }
    .to_xdr_base64(Limits::none())
    .unwrap()
}

fn tx_data() -> String {
    SorobanTransactionData {
        ext: SorobanTransactionDataExt::V0,
        resources: SorobanResources {
            footprint: LedgerFootprint {
                read_only: Default::default(),
                read_write: Default::default(),
            },
            instructions: 1,
            disk_read_bytes: 0,
            write_bytes: 0,
        },
        resource_fee: 1,
    }
    .to_xdr_base64(Limits::none())
    .unwrap()
}

/// The local simulator stand-in: the fee payer's account, and every
/// simulation answering with `auth` (the recording pass's entries).
fn simulator(payer: [u8; 32], auth: Vec<String>) -> FakeRpc {
    let acct = account_entry(payer);
    FakeRpc::start(move |method, _| match method {
        "getLedgerEntries" => json!({ "entries": [{ "xdr": acct }] }),
        "simulateTransaction" => json!({
            "latestLedger": 100,
            "minResourceFee": "1000",
            "transactionData": tx_data(),
            "results": [{ "auth": auth, "xdr": "AAAAAQ==" }],
        }),
        other => json!({ "error": { "code": -32601, "message": format!("unexpected {other}") } }),
    })
}

/// The broadcast stand-in: accepts any envelope under its real hash and
/// reports it applied.
fn network() -> FakeRpc {
    FakeRpc::start(|method, params| match method {
        "sendTransaction" => {
            let env = TransactionEnvelope::from_xdr_base64(
                params["transaction"].as_str().unwrap(),
                Limits::none(),
            )
            .unwrap();
            let TransactionEnvelope::Tx(v1) = env else {
                panic!("v1 envelope")
            };
            let payload = TransactionSignaturePayload {
                network_id: stellar_xdr::Hash(perch_deploy::auth::network_id(PASSPHRASE)),
                tagged_transaction: TransactionSignaturePayloadTaggedTransaction::Tx(v1.tx),
            };
            let hash = hex::encode(Sha256::digest(payload.to_xdr(Limits::none()).unwrap()));
            json!({ "status": "PENDING", "hash": hash })
        }
        "getTransaction" => json!({ "status": "SUCCESS", "ledger": 101, "resultXdr": "" }),
        other => json!({ "error": { "code": -32601, "message": format!("unexpected {other}") } }),
    })
}

fn intended_call() -> InvokeSpec {
    InvokeSpec {
        contract: account(),
        func: "apply_doc".into(),
        args: vec![ScVal::U32(1), ScVal::U32(0)],
    }
}

fn intended_tree() -> SorobanAuthorizedInvocation {
    invocation(
        &account(),
        "apply_doc",
        vec![ScVal::U32(1), ScVal::U32(0)],
        vec![],
    )
}

fn auth_spec(mode: AuthMode) -> AuthSpec {
    AuthSpec {
        mode,
        rule_id: 0,
        account: account(),
        sub_invocations: vec![],
    }
}

fn modes() -> [AuthMode; 2] {
    [AuthMode::External { verifier: c(2) }, AuthMode::Delegated]
}

/// Every simulated envelope's auth entries, in order.
fn simulated_auth(sim: &FakeRpc) -> Vec<Vec<SorobanAuthorizationEntry>> {
    sim.params("simulateTransaction")
        .iter()
        .map(|p| {
            let env = TransactionEnvelope::from_xdr_base64(
                p["transaction"].as_str().unwrap(),
                Limits::none(),
            )
            .unwrap();
            let TransactionEnvelope::Tx(v1) = env else {
                panic!("v1 envelope")
            };
            match &v1.tx.operations[0].body {
                OperationBody::InvokeHostFunction(op) => op.auth.to_vec(),
                _ => panic!("invoke op"),
            }
        })
        .collect()
}

#[test]
fn a_mismatched_tree_is_refused_before_signing_on_every_signing_path() {
    let stray = invocation(&c(5), "transfer", vec![], vec![]);
    let cases: Vec<(&str, Vec<String>, &str)> = vec![
        (
            "contract",
            vec![address_entry(
                &account(),
                invocation(
                    &c(3),
                    "apply_doc",
                    vec![ScVal::U32(1), ScVal::U32(0)],
                    vec![],
                ),
            )],
            "authorizes contract",
        ),
        (
            "function",
            vec![address_entry(
                &account(),
                invocation(
                    &account(),
                    "upgrade",
                    vec![ScVal::U32(1), ScVal::U32(0)],
                    vec![],
                ),
            )],
            "authorizes function upgrade, expected apply_doc",
        ),
        (
            "arguments",
            vec![address_entry(
                &account(),
                invocation(
                    &account(),
                    "apply_doc",
                    vec![ScVal::U32(1), ScVal::U32(9)],
                    vec![],
                ),
            )],
            "argument 1 differs",
        ),
        (
            "sub-invocation",
            vec![address_entry(
                &account(),
                invocation(
                    &account(),
                    "apply_doc",
                    vec![ScVal::U32(1), ScVal::U32(0)],
                    vec![stray.clone()],
                ),
            )],
            "unexpected sub-invocation",
        ),
        (
            "source-account entry",
            vec![
                address_entry(&account(), intended_tree()),
                SorobanAuthorizationEntry {
                    credentials: SorobanCredentials::SourceAccount,
                    root_invocation: stray.clone(),
                }
                .to_xdr_base64(Limits::none())
                .unwrap(),
            ],
            "source account",
        ),
        (
            "other address",
            vec![address_entry(&c(4), intended_tree())],
            "only",
        ),
    ];
    for mode in modes() {
        for (kind, auth, want) in &cases {
            let key = CountingKey::new();
            let sim = simulator(key.public(), auth.clone());
            let net = network();
            let net_rpc = Rpc::new(&net.url);
            let err = run_signed(
                &Rpc::new(&sim.url),
                PASSPHRASE,
                &key,
                &auth_spec(match &mode {
                    AuthMode::External { verifier } => AuthMode::External {
                        verifier: verifier.clone(),
                    },
                    AuthMode::Delegated => AuthMode::Delegated,
                }),
                &intended_call(),
                None,
                Some(&net_rpc as &dyn Broadcast),
            )
            .err()
            .unwrap_or_else(|| panic!("{kind}: a mismatched tree was accepted"));
            let msg = format!("{err:#}");
            assert!(msg.contains(want), "{kind}: {msg:?} lacks {want:?}");
            assert_eq!(key.signatures(), 0, "{kind}: signed before refusing");
            assert_eq!(
                sim.params("simulateTransaction").len(),
                1,
                "{kind}: re-simulated after refusing"
            );
            assert!(
                net.methods().is_empty(),
                "{kind}: reached the broadcast RPC"
            );
        }
    }
}

#[test]
fn a_dry_run_never_invokes_the_signer() {
    for mode in modes() {
        let key = CountingKey::new();
        let sim = simulator(
            key.public(),
            vec![address_entry(&account(), intended_tree())],
        );
        let out = run_signed(
            &Rpc::new(&sim.url),
            PASSPHRASE,
            &key,
            &auth_spec(mode),
            &intended_call(),
            None,
            None,
        )
        .unwrap();
        assert!(out.is_none());
        assert_eq!(key.signatures(), 0);
        // One unsigned recording simulation; no signed entry ever leaves.
        assert_eq!(simulated_auth(&sim), vec![vec![]]);
    }
}

#[test]
fn a_dry_run_still_refuses_a_mismatched_tree() {
    let key = CountingKey::new();
    let wrong = invocation(&account(), "upgrade", vec![], vec![]);
    let sim = simulator(key.public(), vec![address_entry(&account(), wrong)]);
    let err = run_signed(
        &Rpc::new(&sim.url),
        PASSPHRASE,
        &key,
        &auth_spec(AuthMode::Delegated),
        &intended_call(),
        None,
        None,
    )
    .unwrap_err();
    assert!(format!("{err:#}").contains("authorizes function upgrade"));
    assert_eq!(key.signatures(), 0);
}

#[test]
fn the_live_path_simulates_locally_and_only_broadcasts_and_confirms_remotely() {
    for mode in modes() {
        let key = CountingKey::new();
        let sim = simulator(
            key.public(),
            vec![address_entry(&account(), intended_tree())],
        );
        let net = network();
        let net_rpc = Rpc::new(&net.url);
        let submitted = run_signed(
            &Rpc::new(&sim.url),
            PASSPHRASE,
            &key,
            &auth_spec(mode),
            &intended_call(),
            None,
            Some(&net_rpc as &dyn Broadcast),
        )
        .unwrap()
        .expect("live run submits");
        assert_eq!(submitted.ledger, 101);

        // Both simulations (recording, then the signed entry under
        // enforcement) ran on the simulator, never on the broadcast RPC.
        assert_eq!(
            sim.methods(),
            [
                "getLedgerEntries",
                "simulateTransaction",
                "simulateTransaction"
            ]
        );
        assert_eq!(net.methods(), ["sendTransaction", "getTransaction"]);
        let auth = simulated_auth(&sim);
        assert!(auth[0].is_empty());
        assert_eq!(auth[1].len(), 1);
        assert_eq!(auth[1][0].root_invocation, intended_tree());
        assert!(!matches!(
            auth[1][0].credentials,
            SorobanCredentials::Address(SorobanAddressCredentials {
                signature: ScVal::Void,
                ..
            })
        ));
        // The auth entry and the envelope: two signatures, both after the check.
        assert_eq!(key.signatures(), 2);
    }
}

#[test]
fn a_broadcast_hash_that_is_not_the_transaction_is_an_error() {
    let key = CountingKey::new();
    let sim = simulator(
        key.public(),
        vec![address_entry(&account(), intended_tree())],
    );
    let net = FakeRpc::start(|method, _| match method {
        "sendTransaction" => json!({ "status": "PENDING", "hash": "00".repeat(32) }),
        _ => Value::Null,
    });
    let net_rpc = Rpc::new(&net.url);
    let err = run_signed(
        &Rpc::new(&sim.url),
        PASSPHRASE,
        &key,
        &auth_spec(AuthMode::Delegated),
        &intended_call(),
        None,
        Some(&net_rpc as &dyn Broadcast),
    )
    .unwrap_err();
    assert!(
        format!("{err:#}").contains("not this transaction's"),
        "{err:#}"
    );
    assert_eq!(net.methods(), ["sendTransaction"]);
}

#[test]
fn an_upload_that_asks_for_authorization_is_refused() {
    let key = CountingKey::new();
    let sim = simulator(
        key.public(),
        vec![address_entry(&account(), intended_tree())],
    );
    let net = network();
    let net_rpc = Rpc::new(&net.url);
    let err = run_upload(
        &Rpc::new(&sim.url),
        PASSPHRASE,
        &key,
        b"\0asm",
        Some(&net_rpc as &dyn Broadcast),
    )
    .unwrap_err();
    assert!(
        format!("{err:#}").contains("an upload needs none"),
        "{err:#}"
    );
    assert_eq!(key.signatures(), 0);
    assert!(net.methods().is_empty());
}

#[test]
fn an_upload_simulates_locally_and_only_broadcasts_and_confirms_remotely() {
    let key = CountingKey::new();
    let sim = simulator(key.public(), vec![]);
    let net = network();
    let net_rpc = Rpc::new(&net.url);
    run_upload(
        &Rpc::new(&sim.url),
        PASSPHRASE,
        &key,
        b"\0asm",
        Some(&net_rpc as &dyn Broadcast),
    )
    .unwrap();
    assert_eq!(sim.methods(), ["getLedgerEntries", "simulateTransaction"]);
    assert_eq!(net.methods(), ["sendTransaction", "getTransaction"]);
}

/// The host function and sequence number of every envelope in `calls`.
fn envelopes(calls: &[Value]) -> Vec<(HostFunction, i64)> {
    calls
        .iter()
        .map(|p| {
            let env = TransactionEnvelope::from_xdr_base64(
                p["transaction"].as_str().unwrap(),
                Limits::none(),
            )
            .unwrap();
            let TransactionEnvelope::Tx(v1) = env else {
                panic!("v1 envelope")
            };
            match &v1.tx.operations[0].body {
                OperationBody::InvokeHostFunction(op) => {
                    (op.host_function.clone(), v1.tx.seq_num.0)
                }
                _ => panic!("invoke op"),
            }
        })
        .collect()
}

/// A publish of wasm over the inline limit sends two transactions from one
/// fee payer while the simulator's state stays at the ledger it forked: the
/// `publish_hash` must take the sequence number after the upload's.
#[test]
fn a_publish_after_an_upload_takes_the_next_sequence_number() {
    let key = CountingKey::new();
    let registry = c(6);
    let wasm = b"\0asm-larger-than-the-inline-limit".to_vec();
    let wasm_hash: [u8; 32] = Sha256::digest(&wasm).into();
    let publish_hash_tree = invocation(
        &registry,
        "publish_hash",
        vec![
            scv::string("perch-demo").unwrap(),
            scv::address(&account()).unwrap(),
            scv::bytes(&wasm_hash).unwrap(),
            scv::string("1.0.0").unwrap(),
        ],
        vec![],
    );
    let entry = address_entry(&account(), publish_hash_tree);
    let acct = account_entry(key.public());
    let sim = FakeRpc::start(move |method, params| match method {
        "getLedgerEntries" => json!({ "entries": [{ "xdr": acct }] }),
        "simulateTransaction" => {
            let (function, _) = envelopes(std::slice::from_ref(params)).remove(0);
            let auth = match function {
                HostFunction::UploadContractWasm(_) => vec![],
                HostFunction::InvokeContract(call)
                    if call.function_name.0.as_slice() == b"fetch_hash" =>
                {
                    return json!({
                        "latestLedger": 100,
                        "error": "HostError: Error(Contract, #1)",
                    })
                }
                _ => vec![entry.clone()],
            };
            json!({
                "latestLedger": 100,
                "minResourceFee": "1000",
                "transactionData": tx_data(),
                "results": [{ "auth": auth, "xdr": "AAAAAQ==" }],
            })
        }
        other => json!({ "error": { "code": -32601, "message": format!("unexpected {other}") } }),
    });
    let net = network();
    let net_rpc = Rpc::new(&net.url);

    let dir = std::env::temp_dir().join(format!("perch-deploy-publish-seq-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let wasm_path = dir.join("demo.wasm");
    std::fs::write(&wasm_path, &wasm).unwrap();
    let args = PublishArgs {
        wasm: wasm_path,
        wasm_name: "perch-demo".into(),
        binver: "1.0.0".into(),
        registry,
        author: account(),
        verifier: None,
        rule_id: 0,
        dry_run: false,
        public_key: None,
        state: Default::default(),
        receipt: dir.join("receipt.json"),
        inline_max_bytes: 8,
    };
    let signer = || Ok(Box::new(SeedKey::from_seed([9u8; 32])) as Box<dyn SignKey>);
    publish::run(
        &Rpc::new(&sim.url),
        PASSPHRASE,
        &signer,
        &args,
        Some(&net_rpc as &dyn Broadcast),
    )
    .unwrap();

    // The simulator's account stays at 41 throughout; the network has
    // applied the upload at 42 by the time `publish_hash` is built.
    let sent = envelopes(&net.params("sendTransaction"));
    assert_eq!(sent.len(), 2);
    assert!(matches!(sent[0].0, HostFunction::UploadContractWasm(_)));
    assert_eq!(sent[0].1, 42);
    assert!(matches!(sent[1].0, HostFunction::InvokeContract(_)));
    assert_eq!(sent[1].1, 43);
    assert_eq!(
        net.methods(),
        [
            "sendTransaction",
            "getTransaction",
            "sendTransaction",
            "getTransaction"
        ]
    );
    assert!(dir.join("receipt.json").is_file());
}
