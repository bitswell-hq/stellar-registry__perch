//! The local simulator fails closed: no state source, a simulator that
//! cannot start or cannot reach its source, and a source that is not the
//! stated network or ledger are all errors, and nothing falls back to the
//! broadcast RPC.

mod support;

use std::path::{Path, PathBuf};
use std::process::Command;

use perch_deploy::sim::{LocalSim, StateSource};
use serde_json::json;
use support::fake_rpc::FakeRpc;

const PASSPHRASE: &str = "Test SDF Network ; September 2015";

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "perch-deploy-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A stand-in `soroban-sim` that runs `body` as a shell script.
#[cfg(unix)]
fn fake_sim(dir: &Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("soroban-sim");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn rpc_source(url: &str, ledger: Option<u32>) -> StateSource {
    StateSource::Rpc {
        url: url.to_string(),
        ledger,
    }
}

#[test]
fn a_missing_simulator_binary_is_an_error() {
    let dir = scratch("missing");
    let err = LocalSim::start(
        &dir.join("no-such-soroban-sim"),
        &rpc_source("http://127.0.0.1:1", None),
        PASSPHRASE,
    )
    .err()
    .expect("no simulator, no simulation");
    assert!(
        format!("{err:#}").contains("start the local simulator"),
        "{err:#}"
    );
}

#[cfg(unix)]
#[test]
fn an_unreachable_state_source_is_an_error() {
    // What soroban-sim serve does when it cannot read its fork's base.
    let dir = scratch("unreachable");
    let sim = fake_sim(
        &dir,
        "echo 'error: getNetwork to http://127.0.0.1:1: Connection refused' >&2; exit 2",
    );
    let err = LocalSim::start(&sim, &rpc_source("http://127.0.0.1:1", None), PASSPHRASE)
        .err()
        .expect("an unreachable source must not simulate");
    let msg = format!("{err:#}");
    assert!(msg.contains("exited"), "{msg}");
    assert!(msg.contains("Connection refused"), "{msg}");
    assert!(msg.contains("trusted RPC http://127.0.0.1:1"), "{msg}");
}

#[test]
fn a_missing_snapshot_file_is_an_error() {
    let dir = scratch("snapshot");
    let err = LocalSim::start(
        Path::new("soroban-sim"),
        &StateSource::Snapshot {
            path: dir.join("absent.json"),
            ledger: 5,
        },
        PASSPHRASE,
    )
    .err()
    .expect("no snapshot, no simulation");
    assert!(
        format!("{err:#}").contains("not a readable file"),
        "{err:#}"
    );
}

/// A stand-in simulator announcing `facade`'s URL, as soroban-sim serve's
/// banner does.
#[cfg(unix)]
fn announcing(dir: &Path, facade: &FakeRpc) -> PathBuf {
    let port = facade.url.rsplit(':').next().unwrap();
    fake_sim(
        dir,
        &format!(
            "echo 'soroban-sim RPC facade on http://127.0.0.1:{port}: a fork of testnet at ledger 50 (protocol 23)' >&2; exec sleep 30"
        ),
    )
}

#[cfg(unix)]
fn facade(name: &'static str, passphrase: &'static str, ledger: u64) -> FakeRpc {
    FakeRpc::start(move |method, _| match method {
        "fork_info" => json!({ "facade": name, "fork": { "ledger": ledger } }),
        "getNetwork" => json!({ "passphrase": passphrase }),
        _ => json!({ "error": { "code": -32601, "message": "unexpected" } }),
    })
}

#[cfg(unix)]
#[test]
fn the_simulator_is_checked_before_use() {
    let dir = scratch("checked");
    let start = |f: &FakeRpc, source: StateSource| {
        LocalSim::start(&announcing(&dir, f), &source, PASSPHRASE).map(|s| s.describe())
    };

    let ok = facade("soroban-sim 0.1.0", PASSPHRASE, 50);
    let line = start(&ok, rpc_source("http://node", Some(50))).unwrap();
    assert!(
        line.contains("trusted RPC http://node pinned at ledger 50"),
        "{line}"
    );

    let wrong_ledger = facade("soroban-sim 0.1.0", PASSPHRASE, 51);
    let err = start(&wrong_ledger, rpc_source("http://node", Some(50))).unwrap_err();
    assert!(
        format!("{err:#}").contains("not the stated ledger 50"),
        "{err:#}"
    );

    let snapshot = dir.join("snap.json");
    std::fs::write(&snapshot, "{}").unwrap();
    let err = start(
        &wrong_ledger,
        StateSource::Snapshot {
            path: snapshot,
            ledger: 50,
        },
    )
    .unwrap_err();
    assert!(
        format!("{err:#}").contains("not the stated ledger 50"),
        "{err:#}"
    );

    let other_network = facade(
        "soroban-sim 0.1.0",
        "Public Global Stellar Network ; September 2015",
        50,
    );
    let err = start(&other_network, rpc_source("http://node", None)).unwrap_err();
    assert!(
        format!("{err:#}").contains("but this command signs for"),
        "{err:#}"
    );

    let not_sim = facade("stellar-rpc", PASSPHRASE, 50);
    let err = start(&not_sim, rpc_source("http://node", None)).unwrap_err();
    assert!(
        format!("{err:#}").contains("not a soroban-sim facade"),
        "{err:#}"
    );
}

/// The binary refuses to sign without a state source, before touching the
/// key or any RPC.
#[test]
fn the_cli_requires_an_explicit_state_source() {
    let broadcast = FakeRpc::start(|_, _| json!(null));
    let dir = scratch("cli");
    let doc = dir.join("doc.json");
    std::fs::write(&doc, "{}").unwrap();
    for dry in [true, false] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_perch-deploy"));
        cmd.env_remove("PERCH_STATE_RPC_URL")
            .env_remove("PERCH_STATE_SNAPSHOT")
            .env("PERCH_ADMIN_KEY", "not-a-key")
            .args([
                "--rpc-url",
                &broadcast.url,
                "--network-passphrase",
                PASSPHRASE,
            ])
            .args([
                "apply",
                "--account",
                "CAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQC526",
            ])
            .arg("--doc")
            .arg(&doc);
        if dry {
            cmd.args([
                "--dry-run",
                "--public-key",
                "GAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQC5HR",
            ]);
        }
        let out = cmd.output().unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "dry={dry}: {stderr}");
        assert!(
            stderr.contains("needs a state source"),
            "dry={dry}: {stderr}"
        );
    }
    assert!(broadcast.methods().is_empty(), "{:?}", broadcast.methods());
}

#[test]
fn a_public_key_is_accepted_only_with_a_dry_run() {
    let account = "CAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQC526";
    let public = "GAAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQCAIBAEAQC5HR";
    let commands: [&[&str]; 2] = [
        &["apply", "--account", account, "--doc", "doc.json"],
        &[
            "publish",
            "--wasm",
            "demo.wasm",
            "--wasm-name",
            "demo",
            "--binver",
            "1.0.0",
            "--registry",
            account,
            "--author",
            account,
            "--rule-id",
            "0",
        ],
    ];
    for command in commands {
        let out = Command::new(env!("CARGO_BIN_EXE_perch-deploy"))
            .env_remove("PERCH_STATE_RPC_URL")
            .env_remove("PERCH_STATE_SNAPSHOT")
            .args(["--rpc-url", "http://127.0.0.1:1"])
            .args(["--network-passphrase", PASSPHRASE])
            .args(command)
            .args(["--public-key", public])
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{}: {stderr}", command[0]);
        assert!(stderr.contains("--dry-run"), "{}: {stderr}", command[0]);
    }
}

/// The real simulator, when one is installed (`SOROBAN_SIM`), against a
/// state RPC nothing listens on.
#[test]
fn the_real_simulator_fails_closed_on_an_unreachable_state_rpc() {
    let Some(bin) = std::env::var_os("SOROBAN_SIM") else {
        eprintln!("SOROBAN_SIM not set: skipped");
        return;
    };
    let err = LocalSim::start(
        Path::new(&bin),
        &rpc_source("http://127.0.0.1:1", None),
        PASSPHRASE,
    )
    .err()
    .expect("an unreachable source must not simulate");
    assert!(format!("{err:#}").contains("exited"), "{err:#}");
}
