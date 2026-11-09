//! A capped rule meters the account's own transfers.
//!
//! A cap reads a `transfer`'s amount, so a capped rule authorizes a
//! transfer only when its `from` (argument 0) is the account itself
//! (`gate::authorize`). This holds when the account is another account's
//! CAP-0071 delegated signer, where the host runs its `__check_auth` with
//! the delegating account's contexts (`docs/recovery/cap-0071.md`, C2): its
//! rules see `transfer(other, ...)`, and its capped rule refuses it.
//!
//! Real `PerchAccount`s, the real compiler and a Stellar Asset Contract,
//! under enforcing authorization (`set_auths`), never mocked auth. A's capped
//! rule is a scoped document's: XLM `transfer`, capped at 10 XLM per 100
//! ledgers, signed by an app key; the same document's uncapped XLM rule for
//! that key authorizes B's transfer through the same delegated path.

mod support;

use perch_account::{PerchAccount, PerchAccountClient};
use soroban_sdk::token::{StellarAssetClient, TokenClient};
use soroban_sdk::xdr::{
    ScVal, SorobanAddressCredentials, SorobanAddressCredentialsWithDelegates,
    SorobanAuthorizationEntry, SorobanAuthorizedInvocation, SorobanCredentials,
    SorobanDelegateSignature, VecM,
};
use soroban_sdk::{map, vec, Address, Bytes};
use stellar_accounts::smart_account::{AuthPayload, Signer};
use support::*;

const UNIT: i128 = 10_000_000;
const FUNDS: i128 = 100 * UNIT;
const SCOPE: u32 = 1;

fn json(w: &World, s: String) -> Bytes {
    Bytes::from_slice(&w.env, s.as_bytes())
}

/// Install `account`'s authorization of `root` through its admin rule,
/// signed by its delegated owner key.
fn as_owner(w: &World, account: &Address, owner: &Address, root: SorobanAuthorizedInvocation) {
    w.env
        .set_auths(&[w.delegated_entry(account, owner, w.rule_id(account, "admin"), root)]);
}

/// B's authorization of `root` through B's rule `b_rule`, whose signer is
/// the Perch account A; A approves through its rule `a_rule`, whose signer
/// is `key` (CAP-0071 nesting, all in B's one entry).
#[allow(clippy::too_many_arguments)]
fn via_delegate(
    w: &World,
    nonce: i64,
    b: &Address,
    b_rule: u32,
    a: &Address,
    a_rule: u32,
    key: &Address,
    root: SorobanAuthorizedInvocation,
) -> SorobanAuthorizationEntry {
    let e = &w.env;
    SorobanAuthorizationEntry {
        credentials: SorobanCredentials::AddressWithDelegates(
            SorobanAddressCredentialsWithDelegates {
                address_credentials: SorobanAddressCredentials {
                    address: b.clone().into(),
                    nonce,
                    signature_expiration_ledger: w.ledger() + 1_000,
                    signature: w.sc(AuthPayload {
                        signers: map![e, (Signer::Delegated(a.clone()), Bytes::new(e))],
                        context_rule_ids: vec![e, b_rule],
                    }),
                },
                delegates: std::vec![SorobanDelegateSignature {
                    address: a.clone().into(),
                    signature: w.sc(AuthPayload {
                        signers: map![e, (Signer::Delegated(key.clone()), Bytes::new(e))],
                        context_rule_ids: vec![e, a_rule],
                    }),
                    nested_delegates: std::vec![SorobanDelegateSignature {
                        address: key.clone().into(),
                        signature: ScVal::Void,
                        nested_delegates: VecM::default(),
                    }]
                    .try_into()
                    .unwrap(),
                }]
                .try_into()
                .unwrap(),
            },
        ),
        root_invocation: root,
    }
}

#[test]
fn a_capped_rule_meters_only_the_accounts_own_transfers() {
    let w = world();
    let e = &w.env;
    let net = perch_testkit::FIXTURE_NETWORK;

    let issuer = w.new_key();
    let xlm = e
        .register_stellar_asset_contract_v2(issuer.clone())
        .address();
    let app = w.new_key();
    let sink = w.new_key();

    // A: a Perch account whose owner holds its admin rule and applies a
    // scoped document with a capped XLM rule for the app key.
    let a_owner = w.new_key();
    let a = e.register(PerchAccount, (vec![e, Signer::Delegated(a_owner.clone())],));
    let a_doc = json(
        &w,
        format!(
            r#"{{"version":1,"network":"{net}","signers":[{{"id":"owner","address":"{}"}}],"rules":[{{"name":"admin","scope":{{"type":"self-admin"}},"principals":{{"type":"all","signers":["owner"]}}}}]}}"#,
            strkey(&a_owner)
        ),
    );
    as_owner(
        &w,
        &a,
        &a_owner,
        w.invocation(
            &a,
            "apply_doc",
            std::vec![w.sc(a_doc.clone()), w.sc(0u32), w.sc(None::<u64>)],
        ),
    );
    PerchAccountClient::new(e, &a).apply_doc(&a_doc, &0, &None);
    let grant = json(
        &w,
        format!(
            r#"{{"version":1,"network":"{net}","signers":[{{"id":"app","address":"{}"}}],"rules":[{{"name":"xlm","scope":{{"type":"contract","address":"{}"}},"principals":{{"type":"all","signers":["app"]}},"functions":["transfer"],"cap":{{"limit":"{}","period-ledgers":100}}}},{{"name":"xlm-open","scope":{{"type":"contract","address":"{}"}},"principals":{{"type":"all","signers":["app"]}},"functions":["transfer"]}}]}}"#,
            strkey(&app),
            strkey(&xlm),
            10 * UNIT,
            strkey(&xlm)
        ),
    );
    as_owner(
        &w,
        &a,
        &a_owner,
        w.invocation(
            &a,
            "apply_scope",
            std::vec![w.sc(SCOPE), w.sc(grant.clone())],
        ),
    );
    PerchAccountClient::new(e, &a).apply_scope(&SCOPE, &grant);
    e.set_auths(&[]);
    let grant_rule = w.rule_id(&a, "xlm");
    let open_rule = w.rule_id(&a, "xlm-open");

    // B (the world's account): its owner names A as the signer of B's rule
    // on the XLM contract, so A is B's CAP-0071 delegated signer there.
    let b = w.account.clone();
    let b_doc = format!(
        r#"{{"version":1,"network":"{net}","signers":[{{"id":"owner","address":"{}"}},{{"id":"main","address":"{}"}}],"rules":[{{"name":"admin","scope":{{"type":"self-admin"}},"principals":{{"type":"all","signers":["owner"]}}}},{{"name":"pay","scope":{{"type":"contract","address":"{}"}},"principals":{{"type":"all","signers":["main"]}}}}]}}"#,
        strkey(&w.owner),
        strkey(&a),
        strkey(&xlm)
    );
    w.apply_bytes(&json(&w, b_doc), 0)
        .expect("B's owner names A");
    let b_rule = w.rule_id(&b, "pay");

    for to in [&a, &b] {
        e.set_auths(&[SorobanAuthorizationEntry {
            credentials: SorobanCredentials::Address(SorobanAddressCredentials {
                address: issuer.clone().into(),
                nonce: if *to == a { 1 } else { 2 },
                signature_expiration_ledger: w.ledger() + 1_000,
                signature: ScVal::Void,
            }),
            root_invocation: w.invocation(&xlm, "mint", std::vec![w.sc(to.clone()), w.sc(FUNDS)]),
        }]);
        StellarAssetClient::new(e, &xlm).mint(to, &FUNDS);
    }
    e.set_auths(&[]);
    let token = TokenClient::new(e, &xlm);
    let transfer = |from: &Address| {
        w.invocation(
            &xlm,
            "transfer",
            std::vec![w.sc(from.clone()), w.sc(sink.clone()), w.sc(UNIT)],
        )
    };

    // The capped rule authorizes A's own transfer.
    e.set_auths(&[w.delegated_entry(&a, &app, grant_rule, transfer(&a))]);
    assert!(
        token.try_transfer(&a, &sink, &UNIT).is_ok(),
        "A's capped rule spends A's own XLM"
    );
    e.set_auths(&[]);

    // It does not authorize a transfer whose `from` is B, which reaches A's
    // check as B's delegated signer.
    e.set_auths(&[via_delegate(
        &w,
        900_001,
        &b,
        b_rule,
        &a,
        grant_rule,
        &app,
        transfer(&b),
    )]);
    let from_b = token.try_transfer(&b, &sink, &UNIT);
    e.set_auths(&[]);
    assert!(
        from_b.is_err(),
        "A's capped rule authorized a transfer from B: B's balance is {} of {FUNDS}",
        token.balance(&b)
    );
    assert_eq!(token.balance(&b), FUNDS);

    // A's uncapped rule on the XLM contract authorizes the same transfer
    // through the same delegated path.
    e.set_auths(&[via_delegate(
        &w,
        900_002,
        &b,
        b_rule,
        &a,
        open_rule,
        &app,
        transfer(&b),
    )]);
    let open = token.try_transfer(&b, &sink, &UNIT);
    e.set_auths(&[]);
    assert!(open.is_ok(), "A's uncapped rule approves B's transfer");
    assert_eq!(token.balance(&b), FUNDS - UNIT);
}
