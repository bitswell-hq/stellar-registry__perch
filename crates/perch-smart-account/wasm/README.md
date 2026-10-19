# Build-time pins

The account reads its shared infra from this directory **at build time**; the
account *source* hardcodes no contract ids.

| file | committed | used for |
|---|---|---|
| `registry.networks` | yes | the registry the infra was `deploy_stateless`'d from, one row per network |
| `perch-doc-compiler.sha256` | yes | the wasm hash, the content-address salt |
| `perch-interpreter.sha256` | yes | the wasm hash, the content-address salt |
| `perch-spending-limit.sha256` | yes | the wasm hash, the content-address salt |
| `<name>.wasm` | no | the fetched bytes; only tests read them |

`registry.networks` has one `<registry C-strkey> <network passphrase>` row
per line:

| network | registry | |
|---|---|---|
| testnet | `CDX2DMYM…PDA7` | `unverified/perch/constructorless`, live; `deployments/testnet.json` |
| mainnet | `CC236AK2…B7NN` | `perch/constructorless` under the mainnet Stellar Registry root, not deployed yet |

The mainnet row is a prediction. It holds if two named deploys create the
chain, each with no `deployer` argument, so each address is
`deployer(parent, sha256(name))`: `perch` via the root's
`deploy("registry", "perch", …)`, then `constructorless` via `perch`'s own
`deploy`. If either is created another way, nothing ever appears at
`CC236AK2…`. The account then resolves no infra on mainnet (it fails
closed), and the table needs a new row and a new build.
`crates/integration-tests/tests/registry_table.rs` derives both rows from
the Stellar Registry root.

Each `infra::<name>::address(env)` derives `deployer(<the row for the
ledger's network id>, <name>.sha256)` offline, and the deployed account
reports the result through its `infra()` view. On a network the table does
not list it panics, so `apply_doc` and `infra()` fail closed there; the
constructor and `__check_auth` resolve no infra and work anywhere. One build
serves every listed network, but changing any row changes the bytes for all
of them. Pinning keeps a registry republish from changing a deployed
account's behavior (`installed == reviewed`).

The pins are committed so a plain checkout builds, a cargo git dependency on
perch included, with no fetch and no network
(`scripts/check-git-dep.sh`). The table is source: edit it by hand, together
with the factory's identical copy
([`../../perch-account-factory/wasm/`](../../perch-account-factory/wasm/README.md)).
Refresh the hashes one of two ways:

- `scripts/fetch-infra-wasm.sh`: from a deployment manifest
  (`deployments/<network>.json`). Each wasm is fetched by its recorded
  address and refused unless its sha256 and content address match the
  manifest; the pins are written from the checked bytes. It refuses a
  manifest whose registry is not the table's row for its network. CI fetches
  the wasm before running the tests that need it
  (`.github/actions/fetch-infra-wasm`).
- `scripts/build-stack.sh --registry <id>`: from a fresh build, before a
  deployment (`scripts/deploy-stack.sh` runs it). `<id>` must be the table's
  row for the build's network (`--network-passphrase`); a build for another
  registry lists it in both tables first, as a dev build.

A missing pin is a build error naming it, and changing one rebuilds the
account. `scripts/check-pins.sh` checks the committed pins against
`deployments/testnet.json` offline (CI runs it), and `cargo test -p
perch-integration-tests --test testnet_pins` checks the compiled pins.
