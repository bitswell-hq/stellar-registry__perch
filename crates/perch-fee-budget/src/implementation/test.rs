use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{Client, StellarAssetClient},
};

struct Lab {
    e: Env,
    id: Address,
    token: Address,
    cfg: Config,
}
fn lab() -> Lab {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|l| l.sequence_number = 10);
    let admin = Address::generate(&e);
    let token = e.register_stellar_asset_contract_v2(admin).address();
    let cfg = Config {
        controller: Address::generate(&e),
        executor: Address::generate(&e),
        treasury: Address::generate(&e),
        token: token.clone(),
        fee: Address::generate(&e),
        cap: 200,
        low: 80,
        target: 100,
        expires: 100,
    };
    let id = e.register(PerchFeeBudget, (cfg.clone(),));
    let asset = StellarAssetClient::new(&e, &token);
    asset.mint(&cfg.treasury, &1000);
    Client::new(&e, &token).approve(&cfg.treasury, &id, &200, &100);
    Lab { e, id, token, cfg }
}

#[test]
fn actual_sac_transfer_goes_to_registered_f_and_consumes_allowance() {
    let l = lab();
    let c = FeeBudgetClient::new(&l.e, &l.id);
    let t = Client::new(&l.e, &l.token);
    let other = Address::generate(&l.e);
    assert_eq!(c.refill(), 100);
    assert_eq!(l.e.auths()[0].0, l.cfg.executor);
    assert_eq!(t.balance(&l.cfg.fee), 100);
    assert_eq!(t.balance(&other), 0);
    assert_eq!(t.balance(&l.cfg.treasury), 900);
    assert_eq!(t.allowance(&l.cfg.treasury, &l.id), 100);
    assert_eq!(c.status().spent, 100);
    assert_eq!(c.status().config, l.cfg);
}

#[test]
fn repeated_refills_stop_at_lifetime_cap() {
    let l = lab();
    let c = FeeBudgetClient::new(&l.e, &l.id);
    let t = Client::new(&l.e, &l.token);
    let sink = Address::generate(&l.e);
    assert_eq!(c.refill(), 100);
    t.transfer(&l.cfg.fee, &sink, &100);
    assert_eq!(c.refill(), 100);
    t.transfer(&l.cfg.fee, &sink, &100);
    assert!(c.try_refill().is_err());
    assert_eq!(c.status().spent, 200);
    assert_eq!(t.balance(&l.cfg.fee), 0);
    // Increasing the SAC allowance cannot reset the contract budget.
    t.approve(&l.cfg.treasury, &l.id, &999, &100);
    assert!(c.try_refill().is_err());
    assert_eq!(c.status().spent, 200);
}

#[test]
fn adequate_balance_expiry_pause_and_missing_auth_refuse() {
    let l = lab();
    let c = FeeBudgetClient::new(&l.e, &l.id);
    c.refill();
    assert!(c.try_refill().is_err());
    Client::new(&l.e, &l.token).transfer(&l.cfg.fee, &l.cfg.treasury, &100);
    l.e.ledger().with_mut(|v| v.sequence_number = 101);
    assert!(c.try_refill().is_err());
    assert_eq!(c.status().spent, 100);
    l.e.ledger().with_mut(|v| v.sequence_number = 20);
    c.pause();
    assert!(c.try_refill().is_err());
    let l = lab();
    let c = FeeBudgetClient::new(&l.e, &l.id);
    l.e.mock_auths(&[]);
    assert!(c.try_refill().is_err());
    assert_eq!(c.status().spent, 0);
}

#[test]
fn transfer_failure_rolls_back_spent() {
    let l = lab();
    let c = FeeBudgetClient::new(&l.e, &l.id);
    Client::new(&l.e, &l.token).approve(&l.cfg.treasury, &l.id, &1, &100);
    assert!(c.try_refill().is_err());
    assert_eq!(c.status().spent, 0);
    assert_eq!(Client::new(&l.e, &l.token).balance(&l.cfg.fee), 0);
}

#[test]
fn pause_preserves_spend_and_cannot_be_renewed_by_allowance() {
    let l = lab();
    let c = FeeBudgetClient::new(&l.e, &l.id);
    assert_eq!(c.refill(), 100);
    c.pause();
    Client::new(&l.e, &l.token).approve(&l.cfg.treasury, &l.id, &999, &100);
    assert!(c.try_refill().is_err());
    assert_eq!(c.status().spent, 100);
    assert_eq!(c.status().checks, 1);
    assert!(c.status().paused);
}
