//! The size of the ledger entry an account stores for a grant document
//! (`perch-smart-account`'s `StoredGrant`, under its `Grants(grant)` key),
//! computed from the document alone, so the compiler can refuse a grant
//! document whose entry would pass [`crate::MAX_GRANT_STORED_BYTES`].
//!
//! The entry holds, per rule, the rule-eval rule the account lowers it to
//! (rule-core's `lower_scoped`, mirrored as `perch_rule_eval::StoredRule`):
//! every signer the rule names in full, and every condition with its
//! values. Each term below is the XDR size of one `#[contracttype]` value:
//! a symbol, string or bytes is 8 bytes and its padded length, a vector or
//! map 12 bytes and its elements, an enum case a vector of its name and its
//! fields, a struct a map of its field names and values.
//! `integration-tests/tests/grant_stored_size.rs` holds the sum equal to the
//! entry the account writes, so a change to the account's record or to the
//! lowering fails there first.

use perch_ir::{ArgPred, PolicyDoc, Principals, Rule, SignerDecl, SignerMethod};

/// A value's bytes padded to four.
const fn padded(n: usize) -> usize {
    n.div_ceil(4) * 4
}

/// A symbol, string, or bytes value of `n` bytes.
const fn blob(n: usize) -> usize {
    8 + padded(n)
}

const U32: usize = 8;
const U64: usize = 12;
const I128: usize = 20;
const VOID: usize = 4;
/// A vector's or a map's header.
const HEADER: usize = 12;
/// The value's ledger entry around it: last-modified ledger, entry type,
/// extension, contract, the `Grants(u32)` key, durability, and the entry's
/// extension.
const ENTRY: usize = 92;
/// The longest string-in value the lowering keeps (rule-core's
/// `MAX_STRING_LEN`); a longer one is dropped.
const MAX_STRING_LEN: usize = 256;

/// An `Address`: a contract's 32-byte id, or an account's ed25519 key.
fn address(strkey: &str) -> usize {
    if strkey.starts_with('C') {
        40
    } else {
        44
    }
}

/// An enum case `name(fields)`.
const fn case(name: &str, fields: usize) -> usize {
    HEADER + blob(name.len()) + fields
}

/// A `StoredVal::String` of `n` bytes.
const fn string_val(n: usize) -> usize {
    case("String", blob(n))
}

/// The `Signer` a principal id names.
fn signer(decl: &SignerDecl) -> usize {
    match &decl.method {
        SignerMethod::External { verifier, key, .. } => {
            case("External", address(verifier) + blob(key.len() / 2))
        }
        SignerMethod::Delegated { address: a } => case("Delegated", address(a)),
    }
}

/// The `StoredCond` an argument predicate lowers to.
fn condition(pred: &ArgPred) -> usize {
    let arg_eq = |val: usize| case("ArgEq", U32 + val);
    match pred {
        ArgPred::IsSelf(_) => arg_eq(case("Address", 40)),
        ArgPred::AddressEq(p) => arg_eq(case("Address", address(&p.address))),
        ArgPred::U32Eq(_) => arg_eq(case("Other", blob(8))),
        ArgPred::StringIn(p) => {
            let values: usize = p
                .values
                .iter()
                .filter(|v| v.len() <= MAX_STRING_LEN)
                .map(|v| string_val(v.len()))
                .sum();
            case("ArgIn", U32 + HEADER + values)
        }
        ArgPred::StringPrefix(p) => case("ArgStrPrefix", U32 + blob(p.prefix.len())),
    }
}

/// The `StoredFunctions` a rule lowers to: a capped rule `transfer` (when
/// its functions cover it) or nothing, any other its listed functions, or
/// every function but none.
fn functions(rule: &Rule) -> usize {
    let only = |names: usize| case("Only", HEADER + names);
    match (&rule.cap, &rule.functions) {
        (Some(_), Some(fs)) if !fs.iter().any(|f| f == "transfer") => only(0),
        (Some(_), _) => only(blob("transfer".len())),
        (None, Some(fs)) => only(fs.iter().map(|f| blob(f.len())).sum()),
        (None, None) => case("AnyExcept", HEADER),
    }
}

/// The `StoredRule` a grant rule lowers to: a map of its nine fields.
fn stored_rule(rule: &Rule, signers: &[SignerDecl]) -> usize {
    let keys: usize = [
        "auth_root",
        "cap",
        "conds",
        "contract",
        "functions",
        "generation",
        "signers",
        "threshold",
        "valid_until",
    ]
    .iter()
    .map(|k| blob(k.len()))
    .sum();
    let named = match &rule.principals {
        Principals::All(p) => &p.signers,
        Principals::Threshold(p) => &p.signers,
        Principals::SelfAuthenticating(_) => return 0,
    };
    let rule_signers: usize = named
        .iter()
        .filter_map(|id| signers.iter().find(|s| &s.id == id))
        .map(signer)
        .sum();
    let mut conds: usize = rule.args.iter().flatten().map(|a| condition(&a.pred)).sum();
    let mut cap = 0;
    if rule.cap.is_some() {
        // The cap's `amount_arg <= i128::MAX` condition, and the cap.
        conds += case("ArgI128AtMost", U32 + I128);
        cap = HEADER + blob("amount_arg".len()) + I128 + blob("limit".len()) + U32;
        cap += blob("period".len()) + U32;
    }
    let contract = match &rule.scope {
        perch_ir::Scope::Contract(c) => address(&c.address),
        perch_ir::Scope::SelfAdmin(_) => 40,
    };
    let auth_root = match &rule.auth_root {
        Some(r) => {
            let root = HEADER
                + blob("contract".len())
                + address(&r.contract)
                + blob("function".len())
                + blob(r.function.len());
            HEADER + root
        }
        None => VOID,
    };
    let valid_until = if rule.not_after_ledger.is_some() {
        U32
    } else {
        VOID
    };
    HEADER
        + keys
        + auth_root
        + (HEADER + cap)
        + (HEADER + conds)
        + contract
        + functions(rule)
        + U64
        + (HEADER + rule_signers)
        + U32
        + valid_until
}

/// The account's `RuleRecord` of a grant rule: its id, name, no policies,
/// and its stored rule.
fn rule_record(rule: &Rule, signers: &[SignerDecl]) -> usize {
    let keys = blob("id".len()) + blob("name".len()) + blob("policies".len()) + blob("rule".len());
    HEADER + keys + U32 + blob(rule.name.len()) + HEADER + stored_rule(rule, signers)
}

/// The bytes of the ledger entry an account stores for grant document
/// `doc`: its `StoredGrant` (generation, `doc_hash`, index position, every
/// rule's record, the signer ids, and, when any signer has one, each
/// signer's `verifier-name`) in its entry under the `Grants` key. Exact for
/// a document `apply_grant` accepts (every rule live, no self-authenticating
/// principals).
#[must_use]
pub fn grant_stored_bytes(doc: &PolicyDoc) -> u32 {
    let keys: usize = [
        "doc_hash",
        "gen",
        "pos",
        "rules",
        "signer_ids",
        "verifier_names",
    ]
    .iter()
    .map(|k| blob(k.len()))
    .sum();
    let rules: usize = doc.rules.iter().map(|r| rule_record(r, &doc.signers)).sum();
    let ids: usize = doc.signers.iter().map(|s| blob(s.id.len())).sum();
    let names: alloc::vec::Vec<usize> = doc
        .signers
        .iter()
        .map(|s| match &s.method {
            SignerMethod::External {
                verifier_name: Some(n),
                ..
            } => n.len(),
            _ => 0,
        })
        .collect();
    let verifier_names = if names.iter().any(|&n| n > 0) {
        names.iter().map(|&n| blob(n)).sum()
    } else {
        0
    };
    let total = ENTRY
        + HEADER
        + keys
        + U64
        + blob(32)
        + U32
        + (HEADER + rules)
        + (HEADER + ids)
        + (HEADER + verifier_names);
    u32::try_from(total).unwrap_or(u32::MAX)
}
