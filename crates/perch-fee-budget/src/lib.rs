#![no_std]
//! Treasury-consented fee funding for native Perch document rules.
//! Authorize the executor's zero-argument `refill` with a Perch grant (or
//! owner-authorized execute). Settlement always enforces the same immutable
//! recipient and lifetime cap, even when the account's rules are replaced.
//! This is a funding component, not an OZ Policy callback or new doc keyword.
use soroban_sdk::{contractclient, contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub controller: Address,
    pub executor: Address,
    pub treasury: Address,
    pub token: Address,
    pub fee: Address,
    pub cap: u64,
    pub low: u64,
    pub target: u64,
    pub expires: u32,
}
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    pub config: Config,
    pub spent: u64,
    pub paused: bool,
    pub checks: u64,
}

/// Client-only consumers do not link the policy implementation.
#[contractclient(name = "FeeBudgetClient")]
pub trait FeeBudgetInterface {
    fn status(e: Env) -> Status;
    fn refill(e: Env) -> u64;
    fn pause(e: Env);
    fn policy_version() -> u32;
}

#[cfg(feature = "contract")]
mod implementation {
    use super::*;
    use perch_fee_budget_core::{plan, Budget, Refill};
    use soroban_sdk::{
        contract, contracterror, contractevent, contractimpl, panic_with_error, token,
    };

    #[contracttype]
    #[derive(Clone)]
    enum Key {
        Config,
        Spent,
        Paused,
        Checks,
    }
    #[contracterror]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(u32)]
    pub enum Error {
        InvalidConfig = 1,
        Paused = 2,
        NoRefill = 3,
        InvalidBalance = 4,
    }
    #[contractevent]
    pub struct Refilled {
        #[topic]
        pub fee: Address,
        pub amount: u64,
        pub spent: u64,
    }
    #[contract]
    pub struct PerchFeeBudget;

    fn decision(e: &Env, s: &Status) -> Refill<Address> {
        if s.paused {
            panic_with_error!(e, Error::Paused);
        }
        let balance = u64::try_from(token::Client::new(e, &s.config.token).balance(&s.config.fee))
            .unwrap_or_else(|_| panic_with_error!(e, Error::InvalidBalance));
        plan(
            s.config.fee.clone(),
            Budget {
                cap: s.config.cap,
                spent: s.spent,
                low: s.config.low,
                target: s.config.target,
                expires: s.config.expires,
            },
            balance,
            e.ledger().sequence(),
        )
        .unwrap_or_else(|| panic_with_error!(e, Error::NoRefill))
    }

    #[contractimpl]
    impl PerchFeeBudget {
        pub fn __constructor(e: Env, config: Config) {
            config.treasury.require_auth();
            if config.low == 0
                || config.low > config.target
                || config.cap == 0
                || config.expires <= e.ledger().sequence()
                || config.fee == config.treasury
            {
                panic_with_error!(&e, Error::InvalidConfig);
            }
            e.storage().instance().set(&Key::Config, &config);
            e.storage().instance().set(&Key::Spent, &0u64);
            e.storage().instance().set(&Key::Paused, &false);
            e.storage().instance().set(&Key::Checks, &0u64);
        }
        pub fn policy_version() -> u32 {
            2
        }
        pub fn status(e: Env) -> Status {
            // Missing/archived state is never reinitialized with a zero spend.
            Status {
                config: e.storage().instance().get(&Key::Config).unwrap(),
                spent: e.storage().instance().get(&Key::Spent).unwrap(),
                paused: e.storage().instance().get(&Key::Paused).unwrap(),
                checks: e.storage().instance().get(&Key::Checks).unwrap(),
            }
        }
        pub fn refill(e: Env) -> u64 {
            Self::status(e.clone()).config.executor.require_auth();
            // Authorization may run policies; read fresh state after it returns.
            let s = Self::status(e.clone());
            let p = decision(&e, &s);
            e.storage().instance().set(&Key::Spent, &p.total);
            e.storage()
                .instance()
                .set(&Key::Checks, &s.checks.checked_add(1).unwrap());
            token::Client::new(&e, &s.config.token).transfer_from(
                &e.current_contract_address(),
                &s.config.treasury,
                &p.recipient,
                &i128::from(p.amount),
            );
            Refilled {
                fee: p.recipient,
                amount: p.amount,
                spent: p.total,
            }
            .publish(&e);
            p.amount
        }
        pub fn pause(e: Env) {
            Self::status(e.clone()).config.treasury.require_auth();
            e.storage().instance().set(&Key::Paused, &true);
        }
    }

    #[cfg(test)]
    mod test;
}
#[cfg(feature = "contract")]
pub use implementation::{Error, PerchFeeBudget};
