//! The per-network registry table the account and the factory compile in
//! (`crates/*/wasm/registry.networks`):
//!
//! - both crates list the same rows, and each row is perch's constructorless
//!   registry on that network, derived here from the Stellar Registry root
//!   rather than read back from the table: testnet's live
//!   `unverified/perch/constructorless`, and mainnet's `perch/constructorless`,
//!   the address the named deploys will give it;
//! - an unlisted network resolves no infra at all.
//!
//! `testnet_pins` checks the testnet row against the deployment manifest, and
//! `release_stack`'s `one_build_resolves_each_listed_network` runs one built
//! account and factory under every listed network.
use perch_account_factory::pins;
use perch_registry_resolve::registry_contract;
use perch_smart_account::{infra, stateless_registry};
use soroban_sdk::testutils::Ledger;
use soroban_sdk::{Address, Bytes, BytesN, Env};

const TESTNET: &str = "Test SDF Network ; September 2015";
const MAINNET: &str = "Public Global Stellar Network ; September 2015";
const FUTURENET: &str = "Test SDF Future Network ; October 2022";

/// The Stellar Registry root on every network is `deployer(ROOT_DEPLOYER,
/// sha256(ROOT_SALT))`: the registry CLI's hardcoded deployer and the text of
/// its `.salt` file (stellar-registry/cli, `crates/stellar-registry-build`).
const ROOT_DEPLOYER: &str = "GAMPJROHOAW662FINQ4XQOY2ULX5IEGYXCI4SMZYE75EHQBR6PSTJG3M";
const ROOT_SALT: &str = "2026-04-24T13:41:00";

// A registry's named deploy: salt = sha256(name), the registry as deployer.
registry_contract! { mod: unverified, deploy_name: "unverified" }
registry_contract! { mod: perch, deploy_name: "perch" }
registry_contract! { mod: constructorless, deploy_name: "constructorless" }

fn on(passphrase: &str) -> Env {
    let env = Env::default();
    let id = env
        .crypto()
        .sha256(&Bytes::from_slice(&env, passphrase.as_bytes()))
        .to_array();
    env.ledger().with_mut(|l| l.network_id = id);
    env
}

fn root(env: &Env) -> Address {
    let salt = env
        .crypto()
        .sha256(&Bytes::from_slice(env, ROOT_SALT.as_bytes()))
        .to_array();
    env.deployer()
        .with_address(
            Address::from_str(env, ROOT_DEPLOYER),
            BytesN::from_array(env, &salt),
        )
        .deployed_address()
}

#[test]
fn the_table_lists_perchs_registry_on_each_network() {
    let env = on(TESTNET);
    let root_testnet = root(&env);
    assert_eq!(
        root_testnet,
        Address::from_str(
            &env,
            "CAAXJETKPYAATU4HVVQUTE2FFBULNFGZNEOC3MS635U5K3GZLAY2HI4M"
        )
    );
    let testnet = constructorless::address(
        &env,
        &perch::address(&env, &unverified::address(&env, &root_testnet)),
    );
    assert_eq!(stateless_registry(&env), testnet);
    assert_eq!(pins::perch_webauthn_verifier::registry(&env), testnet);

    let env = on(MAINNET);
    let root_mainnet = root(&env);
    assert_eq!(
        root_mainnet,
        Address::from_str(
            &env,
            "CDU4M3LDIOUJJ5F3YXKJ4EJEP5VPRPG6N2LJ5HOQIMN7MNGL3NS3EGUY"
        )
    );
    let mainnet = constructorless::address(&env, &perch::address(&env, &root_mainnet));
    assert_eq!(stateless_registry(&env), mainnet);
    assert_eq!(pins::perch_webauthn_verifier::registry(&env), mainnet);

    // Those two rows and no others, the same in every module of both crates.
    assert_eq!(infra::perch_doc_compiler::NETWORKS.len(), 2);
    for table in [
        infra::perch_interpreter::NETWORKS,
        infra::perch_spending_limit::NETWORKS,
        pins::perch_webauthn_verifier::NETWORKS,
    ] {
        assert_eq!(table, infra::perch_doc_compiler::NETWORKS);
    }
}

#[test]
#[should_panic(expected = "no registry is listed for this network")]
fn the_account_resolves_no_infra_on_an_unlisted_network() {
    infra::perch_doc_compiler::address(&on(FUTURENET));
}

#[test]
#[should_panic(expected = "no registry is listed for this network")]
fn the_factory_names_no_verifier_on_an_unlisted_network() {
    pins::perch_webauthn_verifier::address(&on(FUTURENET));
}
