# Build-time pins (not committed)

The factory pins, at build time:

| file | pin |
|---|---|
| `perch-account.wasm` | `sha256` → the account wasm every `create` deploys |
| `perch-webauthn-verifier.wasm` | `sha256` → with `stateless.id`, the verifier's content address |
| `stateless.id` | the registry the verifier was `deploy_stateless`'d from |

The account itself pins the compiler, interpreter, and spending limit, so
`perch-account.wasm` must be built after those are published, and this crate
after the account ([`docs/deploy/README.md`](../../../docs/deploy/README.md),
"Why order matters" and "Tiers"). Populate the cache with
`scripts/fetch-infra-wasm.sh` (from a deployment manifest) or
`scripts/build-stack.sh --registry <id>` (from a fresh build;
`scripts/deploy-stack.sh` runs it). A missing file is a build error naming
it, and replacing one rebuilds the factory. After a refresh, `cargo test -p
perch-integration-tests --test testnet_pins` checks the compiled pins
against the manifest.
