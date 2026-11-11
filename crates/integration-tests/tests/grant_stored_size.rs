//! The compiler's `grant_stored_bytes` is the size of the ledger entry the
//! account stores for a grant document, to the byte, and the compiler
//! refuses a grant document whose entry would pass `MAX_GRANT_STORED_BYTES`
//! (`docs/recovery/budgets.md`, "Grant-document caps"). The documents cover
//! every part the entry holds: delegated and external signers, keys of
//! several lengths, named and unnamed verifiers, every predicate, functions,
//! caps, expiries, and exact roots. A change to the account's record or to
//! rule-core's lowering fails here first.

use perch_doc_compiler::{
    grant_stored_bytes, DocCompilerError, MAX_GRANT_CANONICAL_BYTES, MAX_GRANT_STORED_BYTES,
};
use perch_testkit::delta::strkey;
use perch_testkit::{
    Bootstrap, World, ED25519_VERIFIER_NAME, FIXTURE_NETWORK, FIXTURE_VERIFIER_NAMES,
};
use soroban_sdk::xdr::{LedgerEntryData, LedgerKey, Limits, ScAddress, ScVal, WriteXdr};
use soroban_sdk::{Bytes, BytesN};

const G: &str = "GALZMP2YGMVP6N57D2E6YVKMK3AONEOSC3F2RAXPOAKRIQVTSHJOOBVH";

fn setup() -> World {
    Bootstrap::native()
        .network(FIXTURE_NETWORK)
        .admin_ed25519([9u8; 32])
        .build()
}

fn bytes(w: &World, json: &str) -> Bytes {
    Bytes::from_slice(&w.env, json.as_bytes())
}

fn doc(signers: &[String], rules: &[String]) -> String {
    format!(
        r#"{{"version":1,"network":"{FIXTURE_NETWORK}","signers":[{}],"rules":[{}]}}"#,
        signers.join(","),
        rules.join(",")
    )
}

/// The account's ledger entry whose value is the record of the grant
/// document hashing to `doc_hash`, in bytes.
fn stored_entry(w: &World, doc_hash: &BytesN<32>) -> usize {
    let me: ScAddress = w.account.clone().into();
    let hash = ScVal::Bytes(doc_hash.to_array().to_vec().try_into().unwrap());
    let snapshot = w.env.to_ledger_snapshot();
    let found: Vec<usize> = snapshot
        .ledger_entries
        .iter()
        .filter_map(|(key, (entry, _))| {
            let (LedgerKey::ContractData(k), LedgerEntryData::ContractData(d)) =
                (&**key, &entry.data)
            else {
                return None;
            };
            let ScVal::Map(Some(fields)) = &d.val else {
                return None;
            };
            let holds = fields.iter().any(|e| {
                matches!(&e.key, ScVal::Symbol(s) if s.to_string() == "doc_hash") && e.val == hash
            });
            (k.contract == me && holds).then(|| entry.to_xdr(Limits::none()).unwrap().len())
        })
        .collect();
    assert_eq!(found.len(), 1, "one record holds the document's hash");
    found[0]
}

/// Apply `json` under `grant` and check the estimate against the entry.
fn check(w: &World, grant: u32, json: &str) -> usize {
    let estimate = grant_stored_bytes(&perch_ir::from_json(json).unwrap()) as usize;
    let doc = bytes(w, json);
    assert_eq!(
        w.compiler_client().grant_stored_bytes(&doc),
        estimate as u32,
        "the view reports the estimate"
    );
    let hash = w.account_client().apply_grant(&grant, &doc);
    assert_eq!(stored_entry(w, &hash), estimate, "grant {grant}: {json}");
    estimate
}

#[test]
fn the_estimate_is_the_stored_entry() {
    let w = setup();
    let target = strkey(&w.verifier);
    let ed = strkey(&w.verifier);
    let any = strkey(&w.fixture_verifiers[0]);
    let any_name = FIXTURE_VERIFIER_NAMES[0];
    let signer = |id: &str, verifier: &str, name: Option<&str>, key: &str| {
        let name = name
            .map(|n| format!(r#","verifier-name":"{n}""#))
            .unwrap_or_default();
        format!(r#"{{"id":"{id}","verifier":"{verifier}"{name},"key":"{key}"}}"#)
    };
    let delegated = |id: &str, address: &str| format!(r#"{{"id":"{id}","address":"{address}"}}"#);
    let scope = format!(r#""scope":{{"type":"contract","address":"{target}"}}"#);

    // One delegated account signer, one bare rule: no verifier names.
    check(
        &w,
        1,
        &doc(
            &[delegated("g", G)],
            &[format!(
                r#"{{"name":"r",{scope},"principals":{{"type":"all","signers":["g"]}}}}"#
            )],
        ),
    );

    // Named ed25519 signers, a delegated contract, every predicate, listed
    // functions, an expiry, and a capped rule with an exact root.
    let c = strkey(&w.compiler);
    let keys: Vec<String> = (1..=3u8).map(|i| format!("{i:02x}").repeat(32)).collect();
    check(
        &w,
        2,
        &doc(
            &[
                signer("a", &ed, Some(ED25519_VERIFIER_NAME), &keys[0]),
                signer("bb", &ed, Some(ED25519_VERIFIER_NAME), &keys[1]),
                delegated("ccc", &c),
                signer("dddd", &ed, Some(ED25519_VERIFIER_NAME), &keys[2]),
            ],
            &[
                format!(
                    r#"{{"name":"wide",{scope},"principals":{{"type":"threshold","m":2,"signers":["a","bb","ccc"]}},"functions":["f","bbbbbbbbb"],"args":[{{"index":0,"pred":{{"type":"is-self"}}}},{{"index":1,"pred":{{"type":"address-eq","address":"{G}"}}}},{{"index":2,"pred":{{"type":"u32-eq","value":7}}}},{{"index":3,"pred":{{"type":"string-prefix","prefix":"pre"}}}},{{"index":4,"pred":{{"type":"string-in","values":["x","yy","zzzzz","123456789"]}}}},{{"index":5,"pred":{{"type":"address-eq","address":"{c}"}}}}],"not-after-ledger":4000000}}"#
                ),
                format!(
                    r#"{{"name":"capped-root",{scope},"auth-root":{{"contract":"{target}","function":"play_a_round"}},"principals":{{"type":"all","signers":["bb","dddd"]}},"functions":["transfer"],"args":[{{"index":0,"pred":{{"type":"is-self"}}}}],"cap":{{"limit":"1000","period-ledgers":17280}}}}"#
                ),
            ],
        ),
    );

    // Long keys under a named verifier and an unnamed one (once in the rule),
    // beside a delegated signer: some signers have no verifier name.
    check(
        &w,
        3,
        &doc(
            &[
                signer("k65", &any, Some(any_name), &"ab".repeat(65)),
                signer("k256", &any, Some(any_name), &"cd".repeat(256)),
                signer("raw", &ed, None, &"ef".repeat(32)),
                delegated("g", G),
            ],
            &[format!(
                r#"{{"name":"long",{scope},"principals":{{"type":"threshold","m":1,"signers":["k65","k256","raw","g"]}}}}"#
            )],
        ),
    );
}

/// A grant document of one delegated signer and one rule whose `string-in`
/// holds `n` short distinct values.
fn values_doc(w: &World, n: usize) -> String {
    let target = strkey(&w.verifier);
    let values: Vec<String> = (0..n).map(|i| format!(r#""{i:x}""#)).collect();
    doc(
        &[format!(r#"{{"id":"g","address":"{G}"}}"#)],
        &[format!(
            r#"{{"name":"v","scope":{{"type":"contract","address":"{target}"}},"principals":{{"type":"all","signers":["g"]}},"functions":["f"],"args":[{{"index":0,"pred":{{"type":"string-in","values":[{}]}}}}]}}"#,
            values.join(",")
        )],
    )
}

/// The compiler refuses a grant document whose entry would pass the bound,
/// one value past the largest it admits, which applies and stores exactly
/// its estimate, within the byte cap.
#[test]
fn the_compiler_bounds_the_entry_a_grant_stores() {
    let w = setup();
    let estimate = |n| grant_stored_bytes(&perch_ir::from_json(&values_doc(&w, n)).unwrap());
    let most = (1..)
        .take_while(|&n| estimate(n) <= MAX_GRANT_STORED_BYTES)
        .last()
        .unwrap();
    let canonical =
        |n| perch_ir::canonical_json(&perch_ir::from_json(&values_doc(&w, n)).unwrap()).len();
    assert!(
        canonical(most + 1) <= MAX_GRANT_CANONICAL_BYTES as usize,
        "within the byte cap"
    );

    let stored = check(&w, 1, &values_doc(&w, most));
    assert!(stored <= MAX_GRANT_STORED_BYTES as usize);
    assert!(matches!(
        w.compiler_client()
            .try_compile_doc(&bytes(&w, &values_doc(&w, most + 1))),
        Err(Ok(DocCompilerError::DocTooLarge))
    ));
}
