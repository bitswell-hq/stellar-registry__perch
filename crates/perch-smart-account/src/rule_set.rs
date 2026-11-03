//! The account's rule set as rule-core's evaluator reads it.
//!
//! `apply_doc` lowers the compiled document's model with `rule_eval::lower`,
//! drops the rules `rules::desired` drops as expired, and stores the rest as
//! `perch_rule_eval::StoredRule`s, aligned with the installed-rule records:
//! the stored rule at index `i` is what the installed rule at index `i`
//! enforces, the recovery rule last when there is one. The constructor
//! stores its admin rule the same way. Nothing evaluates the stored set yet;
//! `__check_auth` is unchanged.

use perch_doc_compiler::CompiledDoc;
use perch_rule_eval::convert::Addresses;
use perch_rule_eval::{admin_survives, doc_of_in, guarded, StoredFunctions, StoredRule};
use soroban_sdk::{Env, Vec};
use stellar_accounts::smart_account::Signer;

use crate::{rules, PerchAccountError};

/// The rule set `compiled` asks for on this account, as stored.
///
/// Refuses with `ModelInvalid` when the compiled document carries no model
/// (or more than one), a model soroban-facade cannot read, or one whose
/// rules or recovery do not match the compiled document's one for one; with
/// `UnguardedRule` when a rule other than the recovery rule asks for no
/// signer (INV-1); and with `AdminLockout` when no rule lets its signers
/// manage the account unconditionally and for good.
pub(crate) fn lowered(
    e: &Env,
    compiled: &CompiledDoc,
) -> Result<Vec<StoredRule>, PerchAccountError> {
    if compiled.model.len() != 1 {
        return Err(PerchAccountError::ModelInvalid);
    }
    let model = compiled.model.get_unchecked(0);
    if model.rules.len() != compiled.rules.len() || model.recovery.len() != compiled.recovery.len()
    {
        return Err(PerchAccountError::ModelInvalid);
    }
    // Every address a lowered rule holds is the account's or one the model
    // names, so the rules convert back to host values by lookup.
    let mut known = Addresses::default();
    let me = known
        .read(e, &e.current_contract_address())
        .ok_or(PerchAccountError::ModelInvalid)?;
    let doc = doc_of_in(e, &model, &mut known).ok_or(PerchAccountError::ModelInvalid)?;
    let documented = compiled.rules.len() as usize;

    let mut kept = alloc::vec::Vec::new();
    for (i, rule) in rule_eval::lower(&doc, &me).into_iter().enumerate() {
        if i < documented {
            if rules::expired(e, compiled.rules.get_unchecked(i as u32).valid_until) {
                continue;
            }
            if !guarded(&rule) {
                return Err(PerchAccountError::UnguardedRule);
            }
        }
        kept.push(rule);
    }
    if !admin_survives(&kept, &me) {
        return Err(PerchAccountError::AdminLockout);
    }
    let mut out = Vec::new(e);
    for rule in &kept {
        out.push_back(
            StoredRule::from_rule_in(e, rule, &known).ok_or(PerchAccountError::ModelInvalid)?,
        );
    }
    Ok(out)
}

/// The constructor's admin rule, as stored: every function of the account,
/// no condition, no cap, no last ledger, and every signer required.
pub(crate) fn admin(e: &Env, signers: &Vec<Signer>) -> StoredRule {
    StoredRule {
        contract: e.current_contract_address(),
        functions: StoredFunctions::AnyExcept(Vec::new(e)),
        conds: Vec::new(e),
        signers: signers.clone(),
        threshold: signers.len(),
        cap: Vec::new(e),
        valid_until: None,
        generation: None,
    }
}
