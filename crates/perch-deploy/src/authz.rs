//! The authorization tree a signing flow intends, built from its own inputs,
//! and the check that simulation asked for exactly that tree. A signature
//! covers the whole `SorobanAuthorizedInvocation` tree of its entry, so the
//! check runs before any signing helper sees the entry: the contract, the
//! function, every argument, and every sub-invocation must be the ones this
//! tool means to authorize. Anything else is an error naming the first
//! difference, and nothing is signed.

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use stellar_xdr::{
    InvokeContractArgs, Limits, ReadXdr, ScAddress, ScSymbol, ScVal, SorobanAuthorizationEntry,
    SorobanAuthorizedFunction, SorobanAuthorizedInvocation, SorobanCredentials, WriteXdr,
};

use crate::scv;

/// One address-credential entry: `account` authorizes `invocation`, the whole
/// tree, sub-invocations included.
pub struct IntendedAuth {
    pub account: ScAddress,
    pub invocation: SorobanAuthorizedInvocation,
}

impl IntendedAuth {
    /// `account` authorizing `contract.func(args)`, with exactly
    /// `sub_invocations` beneath it.
    pub fn new(
        account: &str,
        contract: &str,
        func: &str,
        args: &[ScVal],
        sub_invocations: Vec<SorobanAuthorizedInvocation>,
    ) -> Result<Self> {
        Ok(Self {
            account: scv::contract(account)?,
            invocation: SorobanAuthorizedInvocation {
                function: SorobanAuthorizedFunction::ContractFn(InvokeContractArgs {
                    contract_address: scv::contract(contract)?,
                    function_name: ScSymbol(func.try_into()?),
                    args: args.to_vec().try_into()?,
                }),
                sub_invocations: sub_invocations.try_into()?,
            },
        })
    }
}

/// From a simulation's base64 auth entries, the one entry `intended` names,
/// unsigned and checked whole against it. Refuses: no entry or more than
/// one; a source-account entry (the envelope signature would authorize it);
/// an entry for any other address; an entry already carrying a signature or
/// a CAP-71 form; and any difference in the invocation tree.
pub fn checked_entry(
    simulated: &[String],
    intended: &IntendedAuth,
) -> Result<SorobanAuthorizationEntry> {
    let mut entries = Vec::with_capacity(simulated.len());
    for (i, b64) in simulated.iter().enumerate() {
        let entry = SorobanAuthorizationEntry::from_xdr_base64(b64, Limits::none())
            .with_context(|| format!("simulation auth entry {i} does not decode"))?;
        match &entry.credentials {
            SorobanCredentials::Address(c) => {
                if c.address != intended.account {
                    bail!(
                        "simulation asks {} to authorize {}; only {} authorizes this call",
                        address(&c.address),
                        describe(&entry.root_invocation),
                        address(&intended.account)
                    );
                }
                if c.signature != ScVal::Void {
                    bail!("simulation returned an auth entry that already carries a signature");
                }
            }
            SorobanCredentials::SourceAccount => bail!(
                "simulation asks the source account to authorize {}; the envelope signature \
                 would authorize it, and this call needs only {}'s authorization",
                describe(&entry.root_invocation),
                address(&intended.account)
            ),
            // Recording-mode simulation emits plain Address templates; the
            // CAP-71 form is built here (sign_delegated_auth_entry).
            SorobanCredentials::AddressV2(_) | SorobanCredentials::AddressWithDelegates(_) => {
                bail!("simulation returned a CAP-71 credential entry; expected a plain Address template")
            }
        }
        entries.push(entry);
    }
    let entry = match entries.len() {
        1 => entries.pop().expect("len checked"),
        n => bail!(
            "simulation returned {n} auth entries; this call needs exactly one, from {}",
            address(&intended.account)
        ),
    };
    compare(
        "the authorized call",
        &entry.root_invocation,
        &intended.invocation,
    )?;
    // The field-by-field walk above names the difference; this is the whole
    // check, so a field it does not walk can never slip through.
    if entry.root_invocation != intended.invocation {
        bail!("the simulated authorization tree differs from the intended one");
    }
    Ok(entry)
}

fn compare(
    at: &str,
    got: &SorobanAuthorizedInvocation,
    want: &SorobanAuthorizedInvocation,
) -> Result<()> {
    match (&got.function, &want.function) {
        (SorobanAuthorizedFunction::ContractFn(g), SorobanAuthorizedFunction::ContractFn(w)) => {
            if g.contract_address != w.contract_address {
                bail!(
                    "{at}: simulation authorizes contract {}, expected {}",
                    address(&g.contract_address),
                    address(&w.contract_address)
                );
            }
            if g.function_name != w.function_name {
                bail!(
                    "{at}: simulation authorizes function {}, expected {}",
                    g.function_name.to_utf8_string_lossy(),
                    w.function_name.to_utf8_string_lossy()
                );
            }
            if g.args.len() != w.args.len() {
                bail!(
                    "{at}: simulation authorizes {} arguments, expected {}",
                    g.args.len(),
                    w.args.len()
                );
            }
            for (i, (a, b)) in g.args.iter().zip(w.args.iter()).enumerate() {
                if a != b {
                    bail!(
                        "{at}: argument {i} differs: simulation authorizes {}, expected {}",
                        value(a),
                        value(b)
                    );
                }
            }
        }
        _ => bail!(
            "{at}: simulation authorizes {}, expected {}",
            describe(got),
            describe(want)
        ),
    }
    let (gs, ws) = (&got.sub_invocations, &want.sub_invocations);
    if gs.len() > ws.len() {
        bail!(
            "{at}: simulation authorizes an unexpected sub-invocation {}",
            describe(&gs[ws.len()])
        );
    }
    if gs.len() < ws.len() {
        bail!(
            "{at}: simulation lacks the expected sub-invocation {}",
            describe(&ws[gs.len()])
        );
    }
    for (i, (g, w)) in gs.iter().zip(ws.iter()).enumerate() {
        compare(&format!("{at}, sub-invocation {i}"), g, w)?;
    }
    Ok(())
}

/// `C….fn(N args)`, or the kind of a non-call authorization.
pub fn describe(inv: &SorobanAuthorizedInvocation) -> String {
    match &inv.function {
        SorobanAuthorizedFunction::ContractFn(c) => format!(
            "{}.{}({} args)",
            address(&c.contract_address),
            c.function_name.to_utf8_string_lossy(),
            c.args.len()
        ),
        SorobanAuthorizedFunction::CreateContractHostFn(_)
        | SorobanAuthorizedFunction::CreateContractV2HostFn(_) => "a contract creation".to_string(),
    }
}

fn address(a: &ScAddress) -> String {
    match scv::address_to_string(&ScVal::Address(a.clone())) {
        Ok(s) => s,
        Err(_) => match scv::account_to_string(a) {
            Some(g) => g,
            None => format!("{a:?}"),
        },
    }
}

/// An argument, short enough for an error line: bytes and long values by
/// size and sha256.
fn value(v: &ScVal) -> String {
    let xdr = v.to_xdr(Limits::none()).unwrap_or_default();
    match v {
        ScVal::Bytes(b) if b.len() > 32 => format!(
            "{} bytes (sha256 {})",
            b.len(),
            hex::encode(Sha256::digest(b.as_slice()))
        ),
        _ if xdr.len() > 96 => format!(
            "a {}-byte value (sha256 {})",
            xdr.len(),
            hex::encode(Sha256::digest(&xdr))
        ),
        _ => format!("{v:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::{SorobanAddressCredentials, WriteXdr};

    fn c(n: u8) -> String {
        stellar_strkey::Contract([n; 32])
            .to_string()
            .as_str()
            .into()
    }

    fn call(contract: &str, func: &str, args: Vec<ScVal>) -> SorobanAuthorizedInvocation {
        IntendedAuth::new(&c(0), contract, func, &args, vec![])
            .unwrap()
            .invocation
    }

    fn entry(account: &str, inv: SorobanAuthorizedInvocation) -> String {
        SorobanAuthorizationEntry {
            credentials: SorobanCredentials::Address(SorobanAddressCredentials {
                address: scv::contract(account).unwrap(),
                nonce: 1,
                signature_expiration_ledger: 0,
                signature: ScVal::Void,
            }),
            root_invocation: inv,
        }
        .to_xdr_base64(Limits::none())
        .unwrap()
    }

    fn intended() -> IntendedAuth {
        IntendedAuth::new(&c(1), &c(2), "publish", &[ScVal::U32(5)], vec![]).unwrap()
    }

    fn err(simulated: Vec<String>) -> String {
        format!("{:#}", checked_entry(&simulated, &intended()).unwrap_err())
    }

    #[test]
    fn the_intended_tree_passes() {
        let e = checked_entry(
            &[entry(&c(1), call(&c(2), "publish", vec![ScVal::U32(5)]))],
            &intended(),
        )
        .unwrap();
        assert_eq!(e.root_invocation, intended().invocation);
    }

    #[test]
    fn each_difference_is_named() {
        let cases = [
            (
                call(&c(3), "publish", vec![ScVal::U32(5)]),
                "authorizes contract",
            ),
            (
                call(&c(2), "upgrade", vec![ScVal::U32(5)]),
                "authorizes function upgrade",
            ),
            (
                call(&c(2), "publish", vec![ScVal::U32(6)]),
                "argument 0 differs",
            ),
            (
                call(&c(2), "publish", vec![]),
                "authorizes 0 arguments, expected 1",
            ),
            (
                SorobanAuthorizedInvocation {
                    sub_invocations: vec![call(&c(4), "transfer", vec![])].try_into().unwrap(),
                    ..call(&c(2), "publish", vec![ScVal::U32(5)])
                },
                "unexpected sub-invocation",
            ),
        ];
        for (inv, want) in cases {
            let msg = err(vec![entry(&c(1), inv)]);
            assert!(msg.contains(want), "{msg:?} lacks {want:?}");
        }
    }

    #[test]
    fn a_missing_or_nested_sub_invocation_is_named() {
        let sub = call(&c(4), "rcv_sync", vec![]);
        let want = IntendedAuth::new(&c(1), &c(2), "publish", &[ScVal::U32(5)], vec![sub.clone()])
            .unwrap();
        let bare = entry(&c(1), call(&c(2), "publish", vec![ScVal::U32(5)]));
        let msg = format!("{:#}", checked_entry(&[bare], &want).unwrap_err());
        assert!(msg.contains("lacks the expected sub-invocation"), "{msg}");

        let nested = SorobanAuthorizedInvocation {
            sub_invocations: vec![SorobanAuthorizedInvocation {
                sub_invocations: vec![call(&c(5), "drain", vec![])].try_into().unwrap(),
                ..sub
            }]
            .try_into()
            .unwrap(),
            ..call(&c(2), "publish", vec![ScVal::U32(5)])
        };
        let msg = format!(
            "{:#}",
            checked_entry(&[entry(&c(1), nested)], &want).unwrap_err()
        );
        assert!(
            msg.contains("sub-invocation 0: simulation authorizes an unexpected sub-invocation"),
            "{msg}"
        );
    }

    #[test]
    fn other_entries_are_refused() {
        let good = call(&c(2), "publish", vec![ScVal::U32(5)]);
        assert!(err(vec![]).contains("returned 0 auth entries"));
        assert!(
            err(vec![entry(&c(1), good.clone()), entry(&c(1), good.clone())])
                .contains("returned 2 auth entries")
        );
        assert!(err(vec![entry(&c(9), good.clone())]).contains("only"));
        let source = SorobanAuthorizationEntry {
            credentials: SorobanCredentials::SourceAccount,
            root_invocation: good,
        }
        .to_xdr_base64(Limits::none())
        .unwrap();
        assert!(err(vec![source]).contains("source account"));
    }
}
