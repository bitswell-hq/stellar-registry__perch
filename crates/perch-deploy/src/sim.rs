//! Local simulation. Both simulations of a signing flow (recording, which
//! discovers the authorization entry, and enforcing, which meters the signed
//! entry's verifier and policy calls) run in a `soroban-sim serve` process
//! this tool starts and owns, over a ledger state source named explicitly:
//! a trusted RPC node, or a snapshot file of one stated ledger. The RPC a
//! transaction is broadcast to is never asked to simulate.
//!
//! soroban-sim executes contracts locally, but what it computes is only as
//! sound as the ledger entries it reads (contract code, instances, account
//! and rule storage), so the state source is configuration a person chooses,
//! never a default. There is no fallback: a source that is missing,
//! unreachable, or not the ledger it was said to be is an error.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use crate::rpc::{Rpc, Simulate, Simulation};

/// How long `soroban-sim serve` may take to open its fork: it reads the
/// network's settings and latest ledger, and a checkpoint fork also reads
/// every ledger since its pin.
const STARTUP: Duration = Duration::from_secs(300);
/// A simulation reads ledger entries from the state source as it runs.
const CALL_TIMEOUT: Duration = Duration::from_secs(180);

/// Where local simulation reads ledger state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateSource {
    /// A trusted RPC node, read at its latest ledger, or pinned to `ledger`
    /// (soroban-sim's checkpoint fork, which needs the node to serve every
    /// ledger since).
    Rpc { url: String, ledger: Option<u32> },
    /// A soroban-sim snapshot file, which must be of `ledger`.
    Snapshot { path: PathBuf, ledger: u32 },
}

impl std::fmt::Display for StateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateSource::Rpc { url, ledger: None } => {
                write!(f, "trusted RPC {url} at its latest ledger")
            }
            StateSource::Rpc {
                url,
                ledger: Some(l),
            } => write!(f, "trusted RPC {url} pinned at ledger {l}"),
            StateSource::Snapshot { path, ledger } => {
                write!(f, "snapshot {} of ledger {ledger}", path.display())
            }
        }
    }
}

/// The state source and simulator flags of every signing command.
#[derive(clap::Args, Debug, Default)]
pub struct StateArgs {
    /// Trusted Stellar RPC node that local simulation reads ledger state
    /// from (contract code, storage, accounts). Distinct from --rpc-url,
    /// which only broadcasts and confirms.
    #[arg(long, env = "PERCH_STATE_RPC_URL")]
    pub state_rpc_url: Option<String>,
    /// Pin --state-rpc-url's reads to this ledger.
    #[arg(long, env = "PERCH_STATE_LEDGER")]
    pub state_ledger: Option<u32>,
    /// A soroban-sim snapshot file to read ledger state from instead.
    #[arg(long, env = "PERCH_STATE_SNAPSHOT")]
    pub state_snapshot: Option<PathBuf>,
    /// The ledger --state-snapshot holds; required with it, and checked.
    #[arg(long, env = "PERCH_STATE_SNAPSHOT_LEDGER")]
    pub state_snapshot_ledger: Option<u32>,
    /// The soroban-sim binary that simulates.
    #[arg(long, env = "SOROBAN_SIM", default_value = "soroban-sim")]
    pub soroban_sim: PathBuf,
}

impl StateArgs {
    /// The one state source these flags name.
    pub fn source(&self) -> Result<StateSource> {
        match (&self.state_rpc_url, &self.state_snapshot) {
            (Some(_), Some(_)) => {
                bail!("name one state source: --state-rpc-url or --state-snapshot, not both")
            }
            (None, None) => bail!(
                "local simulation needs a state source: --state-rpc-url URL (a trusted node, \
                 optionally pinned with --state-ledger N) or --state-snapshot FILE with \
                 --state-snapshot-ledger N"
            ),
            (Some(url), None) => {
                if self.state_snapshot_ledger.is_some() {
                    bail!("--state-snapshot-ledger goes with --state-snapshot");
                }
                if !(url.starts_with("http://") || url.starts_with("https://")) {
                    bail!("--state-rpc-url must be an http(s) URL, not {url:?}");
                }
                Ok(StateSource::Rpc {
                    url: url.clone(),
                    ledger: self.state_ledger,
                })
            }
            (None, Some(path)) => {
                if self.state_ledger.is_some() {
                    bail!("--state-ledger goes with --state-rpc-url; a snapshot's is --state-snapshot-ledger");
                }
                let ledger = self.state_snapshot_ledger.context(
                    "--state-snapshot needs --state-snapshot-ledger N, the ledger the file holds",
                )?;
                Ok(StateSource::Snapshot {
                    path: path.clone(),
                    ledger,
                })
            }
        }
    }
}

/// A running `soroban-sim serve` on loopback, forked from a [`StateSource`].
/// Stopped when dropped.
pub struct LocalSim {
    child: Child,
    rpc: Rpc,
    source: StateSource,
    fork_ledger: u32,
    facade: String,
}

impl LocalSim {
    pub fn start(soroban_sim: &Path, source: &StateSource, passphrase: &str) -> Result<Self> {
        let mut cmd = Command::new(soroban_sim);
        cmd.env_remove("PERCH_ADMIN_KEY").env_remove("PERCH_CI_KEY");
        cmd.args(["serve", "--host", "127.0.0.1", "--port", "0", "--quiet"]);
        match source {
            StateSource::Rpc { url, ledger } => {
                cmd.arg("--fork")
                    .arg(url)
                    .arg("--network-passphrase")
                    .arg(passphrase);
                match ledger {
                    Some(l) => cmd
                        .args(["--fork-mode", "checkpoint", "--fork-ledger"])
                        .arg(l.to_string()),
                    None => cmd.args(["--fork-mode", "head"]),
                };
            }
            StateSource::Snapshot { path, .. } => {
                if !path.is_file() {
                    bail!("state snapshot {} is not a readable file", path.display());
                }
                cmd.arg("--snapshot").arg(path);
            }
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn().with_context(|| {
            format!(
                "start the local simulator {} (scripts/rehearse-migration.sh pins the \
                 soroban-sim revision it installs; name another binary with --soroban-sim or \
                 SOROBAN_SIM)",
                soroban_sim.display()
            )
        })?;

        // The facade announces its URL on stderr. Keep draining stderr after
        // that so it never blocks on a full pipe, holding the last lines for
        // an error message.
        let tail = Arc::new(Mutex::new(VecDeque::new()));
        let (tx, rx) = mpsc::channel();
        let stderr = child.stderr.take().expect("stderr piped");
        let lines = tail.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if let Some(url) = banner_url(&line) {
                    let _ = tx.send(url);
                }
                let mut l = lines.lock().unwrap();
                if l.len() == 20 {
                    l.pop_front();
                }
                l.push_back(line);
            }
        });
        let log = |tail: &Arc<Mutex<VecDeque<String>>>| {
            tail.lock()
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        };

        let deadline = std::time::Instant::now() + STARTUP;
        let url = loop {
            if let Ok(url) = rx.recv_timeout(Duration::from_millis(200)) {
                break url;
            }
            if let Some(status) = child.try_wait()? {
                // Give the reader a moment to collect the last lines.
                std::thread::sleep(Duration::from_millis(100));
                bail!(
                    "the local simulator exited ({status}) before serving; state source: \
                     {source}\n{}",
                    log(&tail)
                );
            }
            if std::time::Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                bail!(
                    "the local simulator did not start within {}s; state source: {source}\n{}",
                    STARTUP.as_secs(),
                    log(&tail)
                );
            }
        };

        let mut sim = Self {
            child,
            rpc: Rpc::with_timeout(&url, CALL_TIMEOUT),
            source: source.clone(),
            fork_ledger: 0,
            facade: String::new(),
        };
        let (facade, fork_ledger) = check_facade(&sim.rpc, source, passphrase)?;
        sim.facade = facade;
        sim.fork_ledger = fork_ledger;
        Ok(sim)
    }

    /// One line for the operator: what simulates, and over which state.
    pub fn describe(&self) -> String {
        format!(
            "simulating locally with {} over the {} (forked at ledger {})",
            self.facade, self.source, self.fork_ledger
        )
    }
}

impl Drop for LocalSim {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Simulate for LocalSim {
    fn simulate(&self, envelope_b64: &str) -> Result<Simulation> {
        self.rpc.simulate(envelope_b64)
    }
    fn account_seq(&self, pubkey: &[u8; 32]) -> Result<i64> {
        self.rpc.account_seq(pubkey)
    }
}

fn banner_url(line: &str) -> Option<String> {
    let rest = line.split("RPC facade on ").nth(1)?;
    let url: String = rest
        .chars()
        .take_while(|c| !c.is_whitespace())
        .collect::<String>()
        .trim_end_matches(':')
        .to_string();
    url.starts_with("http://127.0.0.1:").then_some(url)
}

/// Check the endpoint is a soroban-sim facade on `passphrase`'s network,
/// forked at the source's ledger when the source names one. Returns the
/// facade's name and version, and its fork ledger.
pub fn check_facade(rpc: &Rpc, source: &StateSource, passphrase: &str) -> Result<(String, u32)> {
    let info = rpc
        .call("fork_info", json!({}))
        .context("the local simulator does not answer fork_info")?;
    let facade = info
        .get("facade")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if !facade.starts_with("soroban-sim") {
        bail!(
            "{} is not a soroban-sim facade (fork_info.facade = {facade:?})",
            rpc.url()
        );
    }
    let network = rpc.call("getNetwork", json!({}))?;
    let got = network
        .get("passphrase")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if got != passphrase {
        bail!("the state source is network {got:?}, but this command signs for {passphrase:?}");
    }
    let fork_ledger = info
        .get("fork")
        .and_then(|f| f.get("ledger"))
        .and_then(Value::as_u64)
        .and_then(|l| u32::try_from(l).ok())
        .context("fork_info has no fork.ledger")?;
    let pinned = match source {
        StateSource::Rpc { ledger, .. } => *ledger,
        StateSource::Snapshot { ledger, .. } => Some(*ledger),
    };
    if let Some(want) = pinned {
        if fork_ledger != want {
            bail!("the state source holds ledger {fork_ledger}, not the stated ledger {want}");
        }
    }
    Ok((facade, fork_ledger))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_banner_names_the_url() {
        assert_eq!(
            banner_url(
                "soroban-sim RPC facade on http://127.0.0.1:53211: a fork of testnet at ledger 9 (protocol 23)"
            )
            .as_deref(),
            Some("http://127.0.0.1:53211")
        );
        assert_eq!(
            banner_url("soroban-sim RPC facade on http://10.0.0.1:1: x"),
            None
        );
        assert_eq!(banner_url("warning: something"), None);
    }

    fn args(
        rpc: Option<&str>,
        ledger: Option<u32>,
        snap: Option<&str>,
        snap_ledger: Option<u32>,
    ) -> StateArgs {
        StateArgs {
            state_rpc_url: rpc.map(String::from),
            state_ledger: ledger,
            state_snapshot: snap.map(PathBuf::from),
            state_snapshot_ledger: snap_ledger,
            soroban_sim: PathBuf::from("soroban-sim"),
        }
    }

    #[test]
    fn exactly_one_explicit_source() {
        let e = |a: StateArgs| a.source().unwrap_err().to_string();
        assert!(e(args(None, None, None, None)).contains("needs a state source"));
        assert!(e(args(Some("http://a"), None, Some("s.json"), Some(1))).contains("not both"));
        assert!(e(args(None, None, Some("s.json"), None)).contains("--state-snapshot-ledger"));
        assert!(e(args(Some("testnet"), None, None, None)).contains("http(s) URL"));
        assert!(
            e(args(Some("http://a"), None, None, Some(4))).contains("goes with --state-snapshot")
        );
        assert!(
            e(args(None, Some(4), Some("s.json"), Some(4))).contains("goes with --state-rpc-url")
        );
        assert_eq!(
            args(Some("https://node"), Some(7), None, None)
                .source()
                .unwrap(),
            StateSource::Rpc {
                url: "https://node".into(),
                ledger: Some(7)
            }
        );
        assert_eq!(
            args(None, None, Some("s.json"), Some(9)).source().unwrap(),
            StateSource::Snapshot {
                path: "s.json".into(),
                ledger: 9
            }
        );
    }
}
