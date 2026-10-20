# Deploying the perch stack

How the perch contracts are built, published, and pinned to each other, and
how a deployment is checked. The current testnet deployment is
[`deployments/testnet.json`](../../deployments/testnet.json); its build
provenance is recorded in the audit manifest
([`audit-scope.md`](../audit-scope.md#deployment-provenance)), and its
measured exercise is [`testnet-exercise.md`](testnet-exercise.md).

## Why order matters

Consumers pin their dependencies when they are **built**, not when they are
deployed. The account derives the compiler, interpreter, and spending-limit
addresses as `deployer(registry, sha256(wasm))` from the registry id and
hashes committed in its `wasm/` directory
(`perch_registry_resolve::registry_contract!`). The factory pins the account
wasm hash and the WebAuthn verifier the same way.
Nothing can repoint a deployed account or factory (AGENTS.md, "Infra
contracts are resolved from a build-time-pinned wasm hash"). So:

1. a dependency must exist, as exact bytes, before anything that pins it is
   built; and
2. "the account uses compiler X" is only true if the pinned hash when the
   account was built is X's.

A lookup by name can return stale bytes without failing, and a check that
an address is self-consistent does not catch it. So pins are fetched by the
address and hash the manifest records, and checked against both
(`fetch-infra-wasm.sh`, below).

## Tiers

| Tier | Contracts | Pins |
| --- | --- | --- |
| 0 | `perch-doc-compiler`, `perch-interpreter`, `perch-spending-limit`, `perch-ed25519-verifier`, `perch-webauthn-verifier`, `perch-zk-pool`, `perch-zk-adapter`, `perch-recovery` | nothing (the adapter compiles in its circuit's VK) |
| 1 | `perch-account` | compiler, interpreter, spending limit, and the registry they are under |
| 2 | `perch-account-factory` | the account wasm, the WebAuthn verifier, and the registry |

Tier-0 and tier-2 contracts are constructorless, hold no admin, and are
deployed content-addressed (`deploy_stateless`, salt = wasm hash), so an
address is a function of `(network, registry, wasm)` and is known before
anything is deployed. The account has a constructor and per-user state; it is
installed (uploaded) for the factory, never deployed on its own. Recovery
controller, adapter, and pool addresses are not build-time pins at all: the
account's applied document names them (`recovery.controller`, `adapter`,
`pool`), and the controller checks the adapter's circuit and depth against
the pool when a ZK factor is enrolled.

Every stack contract links a 64 KiB wasm stack (128 KiB for the ZK
adapter) instead of rustc's 1 MiB default (its `build.rs`): the host charges
each cross-contract call's VM its whole initial memory, and with the default
a recovery completion at the document caps exceeds the network's memory
limit ([`budgets.md`](../recovery/budgets.md), "Document caps").

The resolve macro's pins are build inputs cargo tracks: replacing a cached
wasm rebuilds the consumer (`perch-registry-resolve-macro`, `pin_file`).

## The manifest

`deployments/<network>.json` is the one record a deployment produces and
every consumer reads.

| Field | |
| --- | --- |
| `registry` | the content-addressing registry: id, its own wasm hash, admin/manager, root |
| `contracts.<name>` | `version`, `sha256`, `address` (absent for the account), `tier`, `bytes`, and `pins`: the sha256 of every contract it was built against |
| `zk` | the circuit id (the adapter's VK hash) and tree depth |
| `source`, `toolchain` | the commit the stack was built from, whether the tree was clean, and rustc / stellar-cli / scaffold versions |
| `deployed_ledger` | where an indexer starts scanning the pool's events |

## Commands

| Command | Does |
| --- | --- |
| `scripts/build-stack.sh` | Records the registry `crates/*/wasm/registry.path` derives on the build's network (`--network-passphrase`, default `$STELLAR_NETWORK_PASSPHRASE` or testnet's), and refuses a `--registry` naming another. Builds tier 0, stages its wasm as the account's pins, builds the account, stages it and the WebAuthn verifier as the factory's pins, builds the factory. `target/stack/build.json` records every hash and the pins each consumer was built against. Deployments build with `stellar scaffold build`; `--builder contract` uses core `stellar contract build`, the same code without scaffold's metadata. It overwrites the account's and factory's committed pins: run `fetch-infra-wasm.sh` (or `git checkout crates/*/wasm`) afterwards to pin the deployment again, unless this build is the new deployment. |
| `scripts/deploy-stack.sh --source <identity>` | Deploys a new instance of the registry wasm the network's perch registries already run, runs `build-stack.sh` against it, then publishes tier 0 and the factory (upload with hash check, `publish_hash`, `deploy_stateless`, derived-address and code-hash checks), installs the account, writes the manifest, and runs `verify-deployment.sh`. Idempotent. `--registry` publishes into an existing registry instead. `--author <C…>` binds every name to a smart account and signs its authorization through the `theahaco/stellar-cli` fork (source/fee stays the delegate). `--reuse "<names>"` fetches those already-published tier-0 by hash and pins them instead of rebuilding — the canonical path (see [The canonical registry](#the-canonical-registry-and-publishing-the-stack-to-it)). `--build-only` and `--prebuilt` split one run into a keyless build and a publish that compiles nothing, as the release jobs run it. |
| `scripts/verify-deployment.sh` | Read-only, by hash and address only: the registry's code; each contract at its content address, running its recorded wasm, served by the registry for its name and version; the account installed; the manifest's recorded pins equal to the deployed hashes; and what consumers resolve on-chain, from the factory's `account_wasm_hash`/`webauthn_verifier`, the adapter's `circuit_id`/`tree_depth` against the pool's `depth`, to the code hash and `infra()` of every account the exercise report lists. A report from another stack commit is skipped with a notice. |
| `scripts/fetch-infra-wasm.sh` | Fetches the account's and factory's pinned wasm from the manifest and rewrites their committed hash pins (`<name>.sha256`), refusing a manifest whose registry is not the one `registry.path` derives on its network, unless `deployments/pending-registry.json` declares that migration. Each wasm is fetched by its recorded address (the account by its hash) and refused unless its sha256 and content address match. `--stack <dir>` fetches every deployed artifact with a `build.json`. |
| `scripts/check-pins.sh` | The committed `registry.path` derives the manifest's registry id on its network (or a declared pending migration, reported as a warning), and the committed hashes equal the manifest's. A new deployment commits its pins with its manifest. |
| `scripts/check-git-dep.sh --compare` | Builds the account, the factory, and `perch-smart-account` as cargo git dependencies from cargo's clean checkout, offline and without fetching, and requires the same wasm as this checkout. |
| `cargo test -p perch-integration-tests --test testnet_pins` | The pins actually compiled into the account and the factory equal the manifest's. |
| `cargo test -p perch-integration-tests --test release_stack -- --ignored` | Every recovery flow through the stack's wasm, with real proofs (`PERCH_STACK_DIR`, default `target/stack`). |
| `cargo run -p perch-testnet` | The same flows on the deployed contracts (see [`testnet-exercise.md`](testnet-exercise.md)). |
| `scripts/bindings-contracts.sh` | Regenerates `packages/perch-contracts` from the deployed wasm and the manifest. |
| `scripts/check-packages.sh` | Packs every npm package, installs the tarballs into an empty project, imports every entry point, and runs every command they install. Publishes nothing. |

CI's `rust` job runs `fetch-infra-wasm.sh --stack`, the workspace tests
(including `testnet_pins`), `verify-deployment.sh`, and the release-stack
suite against the deployed bytes when the deployment's commit is an
ancestor of the tested commit. Otherwise the deployed bytes may run other
contract code: the job pins this source's own build (`build-stack.sh
--builder contract`), the workspace tests run against it without
`testnet_pins`, and the deployed-bytes suite skips with a notice.
`verify-deployment.sh` checks the chain against the manifest either way.
The `release-stack-source` job builds the tested source in pin order
(`build-stack.sh --builder contract`, against the manifest's registry) and
runs the release-stack suite on those fresh bytes, so a change that only a
rebuilt stack would break fails CI. The `packages` job runs
`check-packages.sh`.

## What is verified, and where

| Claim | Check |
| --- | --- |
| A consumer was built against the bytes it names | `build-stack.sh` stages exactly the built tier-0 files; `build.json` records their hashes; `release_stack.rs` refuses a stack whose recorded pins are not the hashes of the wasm it carries |
| The chain runs those bytes | `deploy-stack.sh`: uploaded hash = built hash, deployed address = offline `deployer(registry, hash)`, deployed code hash = built hash; `verify-deployment.sh` repeats all three later |
| The registry serves them | `fetch_hash(name, version)` = recorded hash |
| Consumers resolve them | factory `account_wasm_hash()` and `webauthn_verifier()`, each account's code hash and `infra()`, adapter `circuit_id()`, adapter and pool depth |
| A fresh build pins the deployment | `fetch-infra-wasm.sh` checks every fetched byte; `testnet_pins` checks what was compiled |
| The deployed bytes behave as specified | `release_stack.rs` on the fetched bytes; `perch-testnet` on the chain |
| The source reproduces them | rebuilding the stack from the deployment commit, in another directory, with the recorded toolchain gives every hash the deployment built; in canonical mode, the reused tier-0 reproduce only through the attested release build (the current deployment's record: [`audit-scope.md`](../audit-scope.md#deployment-provenance)) |

A registry name lookup is never the evidence: names only select what to
fetch, and every fetched byte is checked against a hash the manifest
records.

## The canonical registry, and publishing the stack to it

Perch has one canonical registry per network. An account or factory derives
it, and every infra address from it as `deployer(registry, sha256(wasm))`,
so everything one consumer resolves on a network must live in that one
registry. The derivation is committed, not the id:
`crates/*/wasm/registry.path` names the Stellar Registry root's deployer and
salt and the named deploys `perch/constructorless` below it, every hop
derived under the ledger's network id
([`crates/perch-smart-account/wasm/README.md`](../../crates/perch-smart-account/wasm/README.md)),
so one account build and one factory build serve every network. On testnet
that is `CDZGU4AI…BZTM`, on mainnet `CC236AK2…B7NN`; neither exists yet.

The current testnet deployment still lives in the earlier layout's
registry, `unverified/perch/constructorless` (`CDX2DMYM…PDA7`,
`vars.PERCH_REGISTRY_CONTRACT_ID`;
[`../testnet-deployment.md`](../testnet-deployment.md)), which this source
no longer derives. `deployments/pending-registry.json` declares that
migration, so the pin checks accept and report it (`scripts/manifest-registry.sh`).
Creating `perch/constructorless` under the testnet root and redeploying the
stack there is the dry run of the mainnet layout. Until then
`build-stack.sh` refuses `--registry CDX2DMYM…`, so the `deploy-stack` jobs
fail closed rather than publish an account that would not resolve its infra.
The `unverified/perch/stateless` registry is
retired: it keeps `perch-spending-limit` 0.1.1 and earlier, and nothing new is
published there.

`deploy-stack.sh` has two modes. A **rehearsal** deploys to its own fresh
instance of the same registry wasm (`4b2d9ea4…`), with a throwaway key as
admin, manager, and author — self-contained, so it builds and publishes its
own bytes and the account pins exactly those. Its fresh registry is not the
one `registry.path` derives, so `build-stack.sh` refuses it unless a dev
`registry.path` (never committed) derives it: deploy it at a derivable
address (`stellar contract deploy --salt`) and name its deployer and salt
with `path /`. The **canonical** mode
(`--registry <canonical> --author <smart account> --reuse …`) is what CI runs.

The current deployment
([`deployments/testnet.json`](../../deployments/testnet.json)) is canonical:
`release.yml`'s `deploy-stack` job published it into the `constructorless`
registry, reusing the four published tier-0 contracts.

**The same version is different bytes in the two modes.** A rehearsal's
interpreter, compiler, spending limit, and ed25519 verifier carry versions
that `release.yml` also publishes canonically. `release.yml` builds them
with stellar-registry/actions' attested release build, and `build-stack.sh`
runs its own scaffold build, so the bytes differ. In an earlier rehearsal,
interpreter 0.1.3 was `2254f17d…` against `e4d7fb8d…` canonically (observed
2026-10-09). Resolve infra by the manifest's hash or address, never by
`(name, version)` across registries.

### Authority

Three signers, the same split as the constructorless publishes
([`../testnet-deployment.md`](../testnet-deployment.md)):

| Signer | Role |
| --- | --- |
| `perch-author` smart account (`vars.PERCH_AUTHOR_ADDRESS`, `CDIFS3JD…`) | **author** of every published name. The registry has no author transfer, so the binding is permanent. |
| `ci-publish` delegate (`secrets.CI_PUBLISH_SECRET_KEY`, in the `testnet` environment) | signs the account's `publish_hash` authorization and pays the fees; a CAP-71 delegate rule, expiry-bounded. Where each secret lives: [`../testnet-deployment.md`](../testnet-deployment.md#secrets-and-environments). |
| manager (`GA327GGW…`, human-held) | the **one-time initial publish of each new name** (`#12 ManagerRequired` otherwise), which binds that name's author. |

Canonical mode mirrors `release.yml`'s constructorless publishes exactly:
`--author` is the smart account, the source/fee key is the delegate
(`STELLAR_ACCOUNT`), and the delegate signs the account's authorization through
the `theahaco/stellar-cli` fork, which discovers the account's context rule
on-chain (`STELLAR_SIGN_WITH_KEY`). No new authority is introduced.

### The `deploy-stack` jobs (`release.yml`)

On a release that bumps a recovery-stack contract, four gated jobs run
`deploy-stack.sh` against the canonical registry — SA-authored, in pin
order — attest each built wasm, and open a `deploy(<network>)` PR with the
manifest, the regenerated bindings, and the
`@stellar-registry/perch-artifacts` release that ships the manifest (below)
for a human to merge (merging the manifest PR is the deliberate human act
that flips consumers and CI's deployed-bytes checks onto the new
deployment). The jobs run only when `vars.PERCH_DEPLOY_STACK` is `true` —
see the bootstrap below.

| Job | Runs | Holds |
| --- | --- | --- |
| `deploy-stack-build` | `deploy-stack.sh --build-only`: fetches the reused tier-0, compiles the stack, computes each content address | no secret, no environment |
| `deploy-stack` | `deploy-stack.sh --prebuilt`: uploads, publishes, deploys, writes the manifest; compiles and runs no Rust | the delegate seed (`testnet` environment), in the environment only, never argv |
| `deploy-stack-attest` | attests each freshly built wasm (not the reused tier-0); checks out nothing and runs no repository code | the attestation token (`id-token`, `attestations: write`), no secret |
| `deploy-stack-record` | `verify-deployment.sh`, `bindings-contracts.sh`, the perch-artifacts bump, and the PR | no secret |

The split is by job, not by step, because a build script runs as the runner
user with `sudo`: it could replace the `stellar` binary that a later step in
the same job hands the seed to, or request an attestation token for bytes of
its choosing. For the same reason no job unpacks the build's artifact over its
checkout: `deploy-stack` downloads it outside the checkout and copies in only
the stack's wasm, `build.json`, and `addresses.json`, and `deploy-stack-record`
only the manifest and the hash pins the checkout already tracks, so nothing the
build wrote can replace a script those jobs run.

**Why it reuses the published tier-0 rather than rebuilding them.** The
account pins the compiler, interpreter, and spending-limit by the sha256 of
the exact bytes on-chain, and those (with the ed25519 verifier) are the
allow-listed tier-0 the constructorless jobs already published — built with
`stellar scaffold build` **and** `stellar contract optimize` on the reusable
build workflow's `rustup update` toolchain. `build-stack.sh` here builds with
`stellar scaffold build` on the pinned toolchain and does not optimize, so for
the same source it produces **different bytes**: `perch-doc-compiler` 0.3.2 is
`f8bc1b…` canonically but `ccea12…` from `build-stack.sh`. An account built
against the rebuild would pin a hash the registry never `deploy_stateless`'d.
So canonical mode `--reuse`s those names — fetching each by the hash the
registry serves and pinning exactly that — and publishes only the
recovery-specific names (webauthn verifier, ZK pool/adapter, controller) and
the factory. Pinning the shared `contract-release` toolchain to a concrete
rustc + stellar-cli would make the rebuilds byte-identical and remove the need
to reuse; that is a follow-up (design step 7), tracked outside this
workstream.

### One-time manager bootstrap

The registry is managed, so the **first** publish of each new name needs the
manager's signature; afterward the delegate alone (the account's `ci-publish`
rule) publishes every later version. Read read-only against `CDX2DMYM…PDA7`,
the allow-listed tier-0 are already bootstrapped (`perch-doc-compiler`,
`perch-interpreter`, `perch-spending-limit`, `perch-ed25519-verifier`) and
these five recovery-stack names are **not** (`#1 NoSuchWasmPublished`):

- `perch-webauthn-verifier`
- `perch-recovery`
- `perch-zk-pool`
- `perch-zk-adapter`
- `perch-account-factory`

(`perch-account` is install-only — uploaded, never `publish_hash`'d, because
the factory deploys it by its pinned hash — so it needs no name binding.)

Each needs the manager's one-time initial publish, authored by the perch-author
account, at a version below the first CI release (so that CI publish is
accepted as greater). It must publish a **hash-distinct placeholder** wasm, not
the real release bytes: the registry records each wasm hash once, so a second
publish of an already-published hash is refused (`#11 HashAlreadyPublished`),
and the later SA-authored CI release publishes exactly the real bytes — so
bootstrapping the real bytes here would block that release. A benign trailing
custom section keeps the wasm valid while changing its hash; the placeholder is
only `publish_hash`'d (never `deploy_stateless`'d), so its bytes never run and
only the name→author binding survives. The manager (`GA327GGW…`) is human-held;
these are the **exact commands to run by hand** — never signed by CI or
automation. Build the stack once for the wasm, then publish each name:

```sh
REG=CDX2DMYMMEYU6FGN3HPJ2GQSSL5EZHIAMEJD4SPF55FZE5LEUBPPPDA7    # constructorless
AUTHOR=CDIFS3JDMVQBVANOSHEL6AUAKRYLFZFQQVX37QKQGJGQVQXQEHJCJ6PG  # perch-author SA
MANAGER=perch-demo                                              # the manager key (GA327GGW…), human-held

scripts/build-stack.sh --out target/stack   # derives the registry from registry.path

for name in perch-webauthn-verifier perch-recovery perch-zk-pool \
            perch-zk-adapter perch-account-factory; do
  real="target/stack/$(echo "$name" | tr - _).wasm"
  # A hash-distinct PLACEHOLDER — the trailing custom section keeps the wasm
  # valid but changes its hash, so CI's later publish of the real release bytes
  # is not refused with #11 HashAlreadyPublished. Only publish_hash'd, never
  # deployed, so these bytes never run.
  ph="${real%.wasm}.bootstrap.wasm"
  cp "$real" "$ph"; printf '\x00\x09\x08bootstrp' >> "$ph"
  hash=$(stellar contract upload --network testnet --source-account "$MANAGER" --wasm "$ph")
  stellar contract invoke --network testnet --source-account "$MANAGER" --id "$REG" -- \
    publish_hash --wasm_name "$name" --author "$AUTHOR" --wasm_hash "$hash" --version 0.0.0
  rm -f "$ph"
done
```

Each name is then bound to the perch-author account, and every later version
publishes with the delegate alone. Finally set `vars.PERCH_DEPLOY_STACK=true`
so the `deploy-stack` jobs run on the next release.

### Notifying Nido (`nido-dispatch.yml`)

Nido consumes the deployment through `@stellar-registry/perch-artifacts`,
which ships `deployments/<network>.json`. So Nido is told about a deployment
only once a package version carrying its manifest is on npm:

1. The `deploy-stack` job's `deploy(<network>)` PR also bumps
   `@stellar-registry/perch-artifacts`: version, lockfile, and changelog,
   computed by the release PR's own tooling (`scripts/npm-release.sh bump`,
   git-cliff over the history with the deploy commit in it). Nothing is
   hand-edited.
2. Merging the PR changes the manifest and the package version together.
   Once CI passes on `main`, `release.yml`'s `detect-releases` tags
   `perch-artifacts-v<version>` and `npm-publish` publishes it with npm
   provenance.
3. Then `release.yml`'s `notify-nido` job calls the `Notify Nido` workflow
   with that tag. It sends a `repository_dispatch` to `nidohq/nido` for each
   network whose manifest differs from the previous perch-artifacts release's
   (`scripts/nido-dispatch-plan.sh`).

Before sending, the workflow checks the registry: the version must exist,
be published from the tagged commit (`gitHead`), carry a provenance
attestation, match its `integrity`, and ship each announced manifest with the
committed bytes. It retries for up to ten minutes while a new version
propagates, then fails without sending. A manifest that lands without a
perch-artifacts bump (a hand-committed deployment after a testnet reset, or a
deploy PR that merged after a release PR took the same version) is not
announced on its own: the next release PR proposes the bump, and Nido is told
when that release publishes.

Nido's receiving workflow opens a repin PR; a Nido maintainer merges it, and
Nido's factory redeploy follows that merge. Perch never deploys anything for
Nido and never merges on its behalf.

The event is `perch-deployed`. Its `client_payload` is the contract the
receiver consumes:

| Field | |
| --- | --- |
| `schema` | `1`. Bumped on any incompatible change; a receiver rejects a schema it does not know. |
| `repository` | `stellar-registry/perch` |
| `network` | `testnet` or `mainnet`; equals the manifest's own `network` field |
| `perch_sha` | the 40-hex `main` commit the manifest was read at — the ref to repin to; the `perch-artifacts-v<version>` tag's commit |
| `manifest_path` | `deployments/<network>.json` |
| `manifest_sha256` | sha256 of the manifest file's bytes at `perch_sha` |
| `source_commit` | the manifest's `source.commit`: the commit the stack was built from |
| `run_url` | the Perch workflow run that sent it |
| `artifacts_package` | `@stellar-registry/perch-artifacts` |
| `artifacts_version` | the published version that ships this manifest, byte for byte |

`artifacts_package` and `artifacts_version` were added without a schema bump:
a schema-1 receiver that ignores them keeps working.

The payload carries no addresses or hashes beyond the manifest's own digest:
the manifest is the one record. A receiver fetches
`https://raw.githubusercontent.com/<repository>/<perch_sha>/<manifest_path>`,
checks its sha256 against `manifest_sha256`, and reads everything else from
it, still resolving each contract by name and hash (`fetch-infra-wasm.sh`),
never by bare name. A receiver consuming the package instead installs
`artifacts_package@artifacts_version`, whose `deployments/<network>.json`
has the same sha256. The same `perch_sha` can arrive more than once (a manual
re-send), so a receiver keys its repin branch on `network` and updates an
open PR rather than opening a second.

The workflow plans in a job with no secrets, then sends from a job bound to
the GitHub environment named after the network, which holds the
`perch-nido-sync` GitHub App's credentials. That app is installed on
`nidohq/nido` only, with **Contents: read & write** and **Pull requests: read
& write** (the latter for the receiver; the dispatch itself mints a token
narrowed to `contents: write` on `nidohq/nido`, which the dispatches endpoint
requires). Each environment (`testnet`, and `mainnet` before a mainnet
manifest exists) needs two secrets:

- `PERCH_NIDO_SYNC_CLIENT_ID` — the app's Client ID (`Iv…`, from the app's settings page)
- `PERCH_NIDO_SYNC_PRIVATE_KEY` — the app's PEM private key

Without them the send step fails with an error naming the environment. To
re-send — after adding the secrets, or after a receiver failure — run the
workflow by hand (`workflow_dispatch` from `main`, choosing the network). It
re-sends that network's manifest in the latest perch-artifacts release, with
the same registry checks. If the publish itself failed, re-run the release
run's failed jobs instead: `npm-publish` and `notify-nido` re-run together.

## Mainnet

Mainnet is published by hand. No release merge reaches it: the `testnet`
publishes, the `deploy-stack` jobs, and npm run on a release merge as above,
and the separate
[`release-mainnet.yml`](../../.github/workflows/release-mainnet.yml) workflow
runs only when someone dispatches it from `main`. It publishes tags already
released on `main`, as the bytes they were released as, after the `mainnet`
environment's reviewer approves. A dispatch is a dry run unless
`dry_run` is cleared.

| Input | |
| --- | --- |
| `publish_tags` | release tags of allow-listed contracts (`perch-interpreter`, `perch-doc-compiler`, `perch-ed25519-verifier`, `perch-spending-limit`), separated by spaces |
| `stack_tag` | the release tag of a recovery-stack contract; mainnet gets the stack the testnet deployment from its commit runs |
| `dry_run` | default on: run every check, simulate each transaction a real run would send, and print it with its fee; submit nothing and open no PR |

```sh
# Dry run (the default), then the real publish:
gh workflow run release-mainnet.yml -R stellar-registry/perch --ref main \
  -f publish_tags="perch-interpreter-v0.1.4 perch-doc-compiler-v0.3.3"
gh workflow run release-mainnet.yml -R stellar-registry/perch --ref main \
  -f publish_tags="perch-interpreter-v0.1.4 perch-doc-compiler-v0.3.3" \
  -f dry_run=false
# Then the stack, once its tier-0 are on mainnet and testnet runs that stack:
gh workflow run release-mainnet.yml -R stellar-registry/perch --ref main \
  -f stack_tag=perch-account-factory-v0.1.3
```

| Job | Does | Holds |
| --- | --- | --- |
| `mainnet-plan` | derives the mainnet registry from `crates/*/wasm/registry.path` (`perch-derive-id registry`), then `scripts/mainnet-plan.sh`: each tag exists, is on `main`, and names its crate's version at its commit; for the stack, every stack contract and reused tier-0 is released at that commit, it commits `main`'s `registry.path`, and `main` records a testnet deployment built from it | nothing |
| `mainnet-publish` | downloads each tag's release asset, requires its `contract-release` attestation for the tag's commit on `main` and its `binver`, then `scripts/publish-wasm.sh` (upload, `publish_hash`, `deploy_stateless`, code-hash check), or `--plan` | the mainnet delegate (`mainnet` environment) |
| `mainnet-publish-record` | attaches `publish-receipt-mainnet.json` to each release | no secret |
| `mainnet-config` | `scripts/mainnet-settings.sh`, then hands the registry to the build | the variables only (`mainnet` environment) |
| `mainnet-deploy-stack-build` | fetches every wasm the testnet deployment from the stack commit records, by hash (`fetch-infra-wasm.sh --stack`), after checking it records the registry `registry.path` derives on testnet; requires each one but the reused tier-0 to carry that deployment's attestation (`release.yml`, the stack commit, `main`); then addresses the stack under the mainnet registry. Nothing is rebuilt | nothing |
| `mainnet-deploy-stack` | `deploy-stack.sh --prebuilt` on those bytes, or the dry run's plan of the same transactions | the mainnet delegate (`mainnet` environment) |
| `mainnet-deploy-stack-record` | checks the manifest records the derived mainnet registry (`manifest-registry.sh`), runs `verify-deployment.sh`, bumps `@stellar-registry/perch-artifacts` (`deploy-record-bump.sh`), and opens the `deploy(mainnet)` PR | no secret |

Mainnet runs exactly the testnet deployment's bytes, every contract of the
stack. The account and the factory derive their registry from the committed
`registry.path` under the ledger's network id, so one build serves both
networks, and the tier-0 pin nothing. That is why a stack must reach testnet
first: its attestations come from the testnet deployment's attest job. A stack
tag from before `registry.path` (whose account compiled in one network's
registry) is refused, and so is a testnet deployment that records another
registry than the one `registry.path` derives there, as
`deployments/pending-registry.json`'s pending testnet migration does today:
the first mainnet stack follows testnet's move to `perch/constructorless`.

The `deploy(mainnet)` PR records `deployments/mainnet.json` and the
`perch-artifacts` release that ships it; merging it notifies Nido with
`network=mainnet` ([Notifying Nido](#notifying-nido-nido-dispatchyml)). The
committed hash pins (`crates/*/wasm/*.sha256`) are the same bytes on both
networks, so the PR changes none. The `@stellar-registry/perch-contracts`
bindings are generated from one manifest and stay on testnet's.

### Settings

All in the `mainnet` environment. No mainnet job reads a testnet value, and
none of these has a default: `scripts/mainnet-settings.sh` fails on an unset
one. The registry id must be the one `crates/*/wasm/registry.path` derives
on mainnet (`CC236AK2…B7NN`), and the path must be that file's `path`. On
chain, the three addresses must be contracts, and the path must resolve from
the root, one name at a time through each registry's `fetch_contract_id`, to
exactly the registry id.

| Name | Kind | |
| --- | --- | --- |
| `CI_PUBLISH_MAINNET_SECRET_KEY` | secret | the mainnet delegate's seed: fee source, and signer of `perch-author`'s authorization on its own context rule, which must allow `publish_hash`. Never the testnet key. |
| `PERCH_MAINNET_REGISTRY_CONTRACT_ID` | variable | the registry mainnet publishes into (C…): the one the account and the factory derive on mainnet |
| `PERCH_MAINNET_REGISTRY_PATH` | variable | its path under the root, as `registry.path` commits it: `perch/constructorless` (directly under the Stellar Registry root, not `unverified/`) |
| `PERCH_MAINNET_ROOT_REGISTRY` | variable | the Stellar Registry root (C…), recorded as the manifest's `registry.root` |
| `PERCH_MAINNET_AUTHOR_ADDRESS` | variable | the mainnet `perch-author` smart account (C…), bound to every name |

### Before the first mainnet release

None of this exists yet, so no mainnet job can publish today:

1. The `perch` and `perch/constructorless` registries under the mainnet root
   (a Stellar Registry proposal), and the variables above.
2. The mainnet `perch-author` account, with a delegate rule for the mainnet
   key.
3. The manager's one-time initial publish of every name, as on testnet
   ([One-time manager bootstrap](#one-time-manager-bootstrap)).
4. The Nido app secrets in the `mainnet` environment
   ([Notifying Nido](#notifying-nido-nido-dispatchyml)).

## Release-path settings

`release.yml` keeps every release credential in a GitHub environment and out
of every job that builds, but four things it relies on live in GitHub and npm
settings, not in the repository. Each needs a repository or package admin.
Apply them in this order. Steps 1 and 2 must be done before the next npm
release merges: `npm-publish` has no token to fall back on.

### 1. npm trusted publishing, per package

For each package, on npmjs.com open the package, then **Settings → Trusted
publishing → GitHub Actions**, and enter:

| Field | Value |
| --- | --- |
| Organization or user | `stellar-registry` |
| Repository | `perch` |
| Workflow filename | `release.yml` |
| Environment name | `npm` |

The environment name is what keeps a workflow on another branch from
publishing: without it, any branch's copy of `release.yml` with `id-token:
write` would be trusted. The six packages:

| Directory | npm package |
| --- | --- |
| `packages/perch-js` | `@stellar-registry/perch` |
| `packages/perch-interpreter-js` | `@stellar-registry/perch-interpreter` |
| `packages/perch-zk` | `@stellar-registry/perch-zk` |
| `packages/perch-relay` | `@stellar-registry/perch-relay` |
| `packages/perch-contracts` | `@stellar-registry/perch-contracts` |
| `packages/perch-artifacts` | `@stellar-registry/perch-artifacts` |

Then, for each package, set **Settings → Publishing access** to **Require
two-factor authentication and disallow tokens** (trusted publishing still
works under it). Finally revoke the npm automation token that `NPM_TOKEN`
held, on npmjs.com under **Access Tokens**, and delete the secret:

```sh
gh secret delete NPM_TOKEN -R stellar-registry/perch
```

### 2. The `npm` environment

`npm-publish` runs in it. Create it with a `main`-only branch policy
(a reviewer is optional; with one, every npm release waits for approval):

```sh
gh api -X PUT repos/stellar-registry/perch/environments/npm --input - <<'JSON'
{"deployment_branch_policy": {"protected_branches": false, "custom_branch_policies": true}}
JSON
gh api -X POST repos/stellar-registry/perch/environments/npm/deployment-branch-policies \
  -f name=main -f type=branch
```

### 3. Reviewers on `testnet`, and a `mainnet` environment

A dispatch republish (`publish_tag`) runs `registry-publish`'s signing job in
the `testnet` environment, so a required reviewer there is what keeps a
dispatch from publishing unattended. The same environment serves the
CI-gated publishes and `deploy-stack`, so after this every testnet release
waits for one approval per signing job. Keep the existing `main`-only branch
policy. `<reviewer-id>` is the numeric id of each reviewer
(`gh api users/<login> --jq .id`); list one entry per reviewer:

```sh
gh api -X PUT repos/stellar-registry/perch/environments/testnet --input - <<'JSON'
{"reviewers": [{"type": "User", "id": <reviewer-id>}],
 "deployment_branch_policy": {"protected_branches": false, "custom_branch_policies": true}}
JSON
gh api repos/stellar-registry/perch/environments/testnet/deployment-branch-policies \
  --jq '.branch_policies[].name'   # expect: main
```

Create `mainnet` now, before anything names it. `nido-dispatch.yml` binds a
job to `environment: <network>`, and a job naming an environment that does
not exist creates it with no protection:

```sh
gh api -X PUT repos/stellar-registry/perch/environments/mainnet --input - <<'JSON'
{"reviewers": [{"type": "User", "id": <reviewer-id>}],
 "deployment_branch_policy": {"protected_branches": false, "custom_branch_policies": true}}
JSON
gh api -X POST repos/stellar-registry/perch/environments/mainnet/deployment-branch-policies \
  -f name=main -f type=branch
```

It holds `CI_PUBLISH_MAINNET_SECRET_KEY`. The `PERCH_MAINNET_*` variables
the mainnet jobs read ([Mainnet](#mainnet)) go into it before the first
mainnet release, and the Nido app secrets before the first mainnet manifest
merges.

### 4. Delete the repository-level `CI_PUBLISH_SECRET_KEY`

A repository secret is readable by a workflow on any branch; the `testnet`
environment's copy is not. Confirm the environment holds the seed, then
delete the repository copy:

```sh
gh secret list -R stellar-registry/perch --env testnet   # expect CI_PUBLISH_SECRET_KEY
gh secret delete CI_PUBLISH_SECRET_KEY -R stellar-registry/perch
```

Then check that `registry-publish` reads the environment's copy. Re-publish a
tag that is already on-chain; the publish step's preflight skips a same-hash
version, so nothing is written:

```sh
tag=$(git tag -l 'perch-interpreter-v*' | sort -V | tail -1)
gh workflow run release.yml -R stellar-registry/perch --ref main -f publish_tag="$tag"
```

Approve the `testnet` deployment. The `constructorless-publish` leg should log
`version … already published with the same hash — skipping`. If it fails at
**Check the signing key resolved** with `signing_key resolved to an empty
string`, the reusable job did not see the environment's copy through the
caller's mapping. In that case add the same seed to the `testnet` environment
as a secret named `SIGNING_KEY`, which GitHub documents as taking precedence
over the passed secret in a reusable workflow's environment-bound job, and run
the check again. Never restore the repository copy.

### 5. A ruleset for release tags

Release tags (`<contract>-v<version>` and `<package>-v<version>`) all match
`perch-*-v*`. `detect-releases` never re-tags a version, and a dispatch
publishes whatever asset a tag names, so a release tag must not move, vanish,
or be created by hand:

```sh
gh api -X POST repos/stellar-registry/perch/rulesets --input - <<'JSON'
{
  "name": "release tags",
  "target": "tag",
  "enforcement": "active",
  "conditions": {"ref_name": {"include": ["refs/tags/perch-*-v*"], "exclude": []}},
  "rules": [{"type": "creation"}, {"type": "update"}, {"type": "deletion"}, {"type": "non_fast_forward"}],
  "bypass_actors": [{"actor_id": 15368, "actor_type": "Integration", "bypass_mode": "always"}]
}
JSON
```

| Rule | Stops |
| --- | --- |
| `creation` | a writer tagging a branch commit with a release name; `detect-releases` still creates tags through the GitHub Actions bypass (app id 15368, the `GITHUB_TOKEN` identity) |
| `update`, `non_fast_forward` | moving an existing release tag to another commit |
| `deletion` | deleting a release tag (a failed release is recovered with a new version bump, never a re-tag) |

GitHub has refused the GitHub Actions app as a bypass actor in some
repositories (HTTP 422, "Actor GitHub Actions integration must be part of the
ruleset source or owner organization"). If it does here, create the ruleset
without the `creation` rule and with an empty `bypass_actors`: moves and
deletions stay blocked, and the `testnet` reviewer from step 3 remains the
check on a dispatch publish. The bypass is also not limited to `main`: any
workflow whose `GITHUB_TOKEN` has `contents: write` can create a release tag
through it, so the reviewer stays necessary either way.

## Follow-up: reusable-workflow integrity inputs

The reusable `stellar-registry/actions` workflows that `release.yml` pins
(`contract-release.yml` and `registry-publish.yml`, at `57bfdf4`) check less
than `release.yml`'s own jobs do. They install the stellar-cli fork and
scaffold with no checksum, though the job that installs the fork holds the
seed. Their `gh attestation verify` binds the signer workflow but not the
source ref or commit. A change to that repository adds inputs for these.
Once it is merged to that repository's `main`:

1. Re-pin both `uses:` lines in `release.yml` (marked `FOLLOW-UP`) to the
   merged commit, full SHA with a dated comment, as now. Never pin a commit
   that is not on its `main`.
2. Pass the checksums `release.yml` already pins in
   `.github/actions/setup-stellar-fork`: the fork's tarball
   `580a2d469767028e575e1c57ccf55bc5a661c0ee4032fd52aaeabb9098943685` and
   scaffold's `f24709de57e33155efd72547c9286b522d4360a379bde5af9eeb5519396f10cc`.
3. Bind the attestation check to `refs/heads/main`, the tagged commit, and the
   reusable workflow's own pinned commit, using the input names that change
   defines.
4. Keep the `testnet` reviewer: it covers the dispatch path even with the
   binding in place.

## Simulating a recovery completion

A completion is `apply_doc` of the attempt's target, authorized through the
zero-signer recovery rule. The controller recognizes it from a marker its
`enforce` writes inside the account's `__check_auth`. Recording-mode
simulation, the default way a client learns a transaction's auth entries,
never runs `__check_auth`, so `enforce` never runs, `rcv_sync` sees no
marker, and the simulation fails with `AttemptAuthorized`, as if the call
were a policy change during the window.

So a client building a completion must put the recovery-rule entry in the
transaction itself: address credentials for the account, a fresh nonce, an
`AuthPayload` with no signers selecting the recovery rule, and a root
invocation of exactly `apply_doc(target, 0, expected_revision)` (`None`,
or the revision the target was derived at). Then it simulates, in
enforcing mode. `perch-testnet` does this (`Entries::Root`). The same goes
for any check that should observe the `Protected` freeze rather than the
controller's own refusal.

## Testnet resets

Testnet resets quarterly. After a reset, run `deploy-stack.sh` with a funded
identity, then `scripts/bindings-contracts.sh` and `cargo run -p
perch-testnet`, and commit the manifest, the bindings, and the exercise
report, and update the deployment provenance in
[`audit-scope.md`](../audit-scope.md#deployment-provenance). Every address
changes. The hashes do not change unless the source does.

## Packages

| Package | Contents |
| --- | --- |
| `@stellar-registry/perch` (`packages/perch-js`) | PolicyDoc schemas, builder, canonical JSON and hashes, statement encodings, and the consumer interface ([`packages/perch-js/README.md`](../../packages/perch-js/README.md)) |
| `@stellar-registry/perch-interpreter` | interpreter bindings |
| `@stellar-registry/perch-zk` | commitments, witnesses, bb.js proving; `/indexer`: the trust-free pool indexer, serving witnesses from `PoolWitnessIndex` (one chunk of leaves per witness; node stores and a snapshot to resume) |
| `@stellar-registry/perch-relay` | the guardian-approval relay: admits an approval only once authenticated (enforcing simulation of the controller call, or a `G...` guardian's own signature with the call rebuilt from the posted statement), including CAP-0071 delegated entries; `/node` and the `perch-relay` command serve it from one process |
| `@stellar-registry/perch-contracts` | bindings for every stack contract, from the deployed wasm, and the manifests |
| `@stellar-registry/perch-artifacts` | the deployment manifests, Noir circuits and their manifest, test vectors, and document and program limits as files; `perch-fetch-infra-wasm --stack` and `perch-zk-toolchain` run the repository's scripts against them. Changing a shipped file re-releases it |

All six are published to npm by `release.yml`'s `npm-pack` and
`npm-publish` jobs: merging a bump of a package's manifest version (the
release PR proposes one from its conventional commits) tags
`<pkg>-v<version>`, `npm-pack` tests and packs it with no credential, and
`npm-publish` publishes that tarball by npm trusted publishing (OIDC, with
provenance; no npm token exists, see
[Release-path settings](#release-path-settings)). Rust crates are never published to crates.io; contract releases
are on-chain wasm publishes (`release.yml`). Hosting the indexer or the
relay, with storage, routes, and schedules, is the operator's concern. Nido's
hosted services are deployments of these packages.
