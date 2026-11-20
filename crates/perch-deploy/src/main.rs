//! perch-deploy: the interim signing shim for the perch smart-account
//! pipeline — it exists only because stellar-cli cannot yet sign OZ
//! smart-account approvals (AuthPayload selecting a rule id). `apply` submits
//! a policy document to the account's `apply_doc` signed by the admin key; CI
//! publishes wasm to the registry signed by the scoped CI key. Everything
//! else is the stock `stellar` CLI. Key material comes ONLY from
//! PERCH_ADMIN_KEY / PERCH_CI_KEY env variables.
//!
//! `apply` and `publish` simulate locally (soroban-sim over an explicit
//! state source, `--state-rpc-url` or `--state-snapshot`) and check the
//! simulated authorization tree against the intended call before signing;
//! `--rpc-url` then only broadcasts the signed transaction and confirms it.

use perch_deploy::{apply, compose, keys, publish, rpc, sim, verify};

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use soroban_sdk::Env;

#[derive(Parser)]
#[command(name = "perch-deploy", about, version)]
struct Cli {
    /// Soroban RPC endpoint. `apply` and `publish` only send the signed
    /// transaction there and poll it (state and simulation come from
    /// --state-rpc-url or --state-snapshot); `verify` reads from it.
    #[arg(long, env = "STELLAR_RPC_URL", global = true)]
    rpc_url: Option<String>,
    /// Network passphrase (hashed into every signature).
    #[arg(long, env = "STELLAR_NETWORK_PASSPHRASE", global = true)]
    network_passphrase: Option<String>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Expand a PolicyDoc into the reviewable JSON of the rules it lowers to
    /// on an account: each rule's context, signers, and the rule-eval rule
    /// the account stores for it. `verify`'s expectation file.
    Compose {
        /// Path to the PolicyDoc JSON.
        #[arg(long)]
        doc: PathBuf,
        /// The smart account (C…) the rules apply to.
        #[arg(long)]
        account: String,
    },
    /// Apply a policy document whole: submit its JSON bytes to the account's
    /// `apply_doc` in one transaction, signed by PERCH_ADMIN_KEY selecting
    /// the `admin` rule, at the revision it was read at. The account
    /// verifies and swaps the entire rule set atomically.
    Apply {
        #[arg(long)]
        account: String,
        /// Path to the PolicyDoc JSON — the same file the review approved.
        #[arg(long)]
        doc: PathBuf,
        /// Check the authorization and print the unsigned estimate; signs
        /// and sends nothing, and never reads PERCH_ADMIN_KEY (needs
        /// --public-key).
        #[arg(long)]
        dry_run: bool,
        /// PERCH_ADMIN_KEY's public key (G…): required with --dry-run, and
        /// accepted only with it.
        #[arg(long, requires = "dry_run")]
        public_key: Option<String>,
        #[command(flatten)]
        state: sim::StateArgs,
    },
    /// Publish wasm to the registry, signed by PERCH_CI_KEY selecting --rule-id.
    Publish(publish::PublishArgs),
    /// Read-only check that the on-chain rules match a compose output.
    Verify {
        #[arg(long)]
        account: String,
        #[arg(long)]
        rules: PathBuf,
        /// Accept an account built before the `rule_set` view: its stored
        /// rules (signers, predicates) are not compared, and a run with no
        /// mismatch prints PARTIAL and exits 3 instead of 0.
        #[arg(long)]
        legacy_no_rule_set: bool,
    },
}

impl Cli {
    fn rpc(&self) -> Result<rpc::Rpc> {
        let url = self
            .rpc_url
            .as_deref()
            .context("--rpc-url or STELLAR_RPC_URL required")?;
        Ok(rpc::Rpc::new(url))
    }

    fn passphrase(&self) -> Result<&str> {
        self.network_passphrase
            .as_deref()
            .context("--network-passphrase or STELLAR_NETWORK_PASSPHRASE required")
    }

    /// The RPC a live run broadcasts to; a dry run has none.
    fn broadcast(&self, dry_run: bool) -> Result<Option<rpc::Rpc>> {
        if dry_run {
            return Ok(None);
        }
        Ok(Some(self.rpc()?))
    }
}

/// The local simulator over `source`, announced on stderr.
fn local_sim(
    state: &sim::StateArgs,
    source: &sim::StateSource,
    passphrase: &str,
) -> Result<sim::LocalSim> {
    let sim = sim::LocalSim::start(&state.soroban_sim, source, passphrase)?;
    eprintln!("{}", sim.describe());
    Ok(sim)
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();
    match &cli.cmd {
        Cmd::Compose { doc, account } => {
            let doc_json = std::fs::read_to_string(doc)
                .with_context(|| format!("read doc {}", doc.display()))?;
            let env = Env::default();
            let out = compose::compose(&env, &doc_json, account)?;
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        Cmd::Apply {
            account,
            doc,
            dry_run,
            public_key,
            state,
        } => {
            let passphrase = cli.passphrase()?;
            let broadcast = cli.broadcast(*dry_run)?;
            let source = state.source()?;
            let sim = local_sim(state, &source, passphrase)?;
            let key = keys::signing_key("PERCH_ADMIN_KEY", public_key.as_deref(), *dry_run)?;
            apply::run(
                &sim,
                passphrase,
                key.as_ref(),
                account,
                doc,
                broadcast.as_ref().map(|b| b as &dyn rpc::Broadcast),
            )?
        }
        Cmd::Publish(args) => {
            let passphrase = cli.passphrase()?;
            let broadcast = cli.broadcast(args.dry_run)?;
            let source = args.state.source()?;
            let sim = local_sim(&args.state, &source, passphrase)?;
            let key =
                || keys::signing_key("PERCH_CI_KEY", args.public_key.as_deref(), args.dry_run);
            publish::run(
                &sim,
                passphrase,
                &key,
                args,
                broadcast.as_ref().map(|b| b as &dyn rpc::Broadcast),
            )?
        }
        Cmd::Verify {
            account,
            rules,
            legacy_no_rule_set,
        } => {
            let verdict = verify::run(&cli.rpc()?, account, rules, *legacy_no_rule_set)?;
            return Ok(ExitCode::from(verdict.exit_code()));
        }
    }
    Ok(ExitCode::SUCCESS)
}
