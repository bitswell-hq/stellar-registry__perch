# OZ and Perch in soroban-sim

`index.html` is the Perch paper's dual simulation: one self-contained,
offline page (4.9 MB) that runs an OpenZeppelin v0.8.0 smart account with an
oz-policy-builder rule and OpenZeppelin's `spending_limit`, and a Perch
account with one grant, side by side in
[soroban-sim](https://github.com/theahaco/soroban-sim/tree/d02938430f0ac41f34e5e3e4c8c48d85b871bd68)
in the browser. It extends the runtime of `docs/oz-walkthrough/index.html`.

## What it runs

The rule on both accounts: the app key may move XLM out of the account with
`transfer` on the native asset contract, only to the shop. On the OZ account
the predicate policy-synth synthesized pins the recipient and bounds each
transfer at 10 XLM, and `spending_limit` adds 10 XLM per 100 ledgers. On the
Perch account, grant 1 pins the recipient and caps 10 XLM per 100 ledgers.

The reader sends payments (amount 1–100 XLM, the shop or another recipient,
signed by the app key, by another key in the app's name, or by a key not on
the rule), repeats the last one, advances the ledgers, and switches
`spending_limit` off and on for the OZ rule (the account's own
`remove_policy` / `add_policy`, authorized by the owner through rule 0). Each
request runs on both accounts' contract WebAssembly; the page shows each
side's verdict and code, the host's authorization trace, the authorization
entries signed, the window record, the ledger and a fingerprint of the
ledger entries, with running totals per design. A reference scenario of
fifteen commands is embedded as per-reply hashes of the native run; each
browser reply on that path is marked "matches the native run" or not. The
paper's record of that run is `../source/figures/fig-sim.run.json`.

## Rebuild

The page carries its build sources ("Download the build sources"). Their
`BUILD.md` fetches the pinned trees by commit and rebuilds the contracts, the
driver (native and `wasm32-wasip1`), the reference run, the page and the run
record:

```sh
./fetch-sources.sh
OZ_WASM_DIR=.sources/wasm-oz python3 build-contracts.py
POLICY_SYNTH_SRC=$PWD/.sources/oz-policy-builder/packages/policy-synth/src ./make.sh index.html fig-sim.run.json
```

## Pins

| Source | Revision |
| --- | --- |
| theahaco/soroban-sim | d02938430f0ac41f34e5e3e4c8c48d85b871bd68 (soroban-env-host 29.0.2) |
| bitswell-hq/stellar-registry__perch | f0275f41c5ab47c903bbfe9986b4bf3239c01b11 |
| bitswell/soroban-rule-core | cbabe594ae2df408991e825e56ddd618ea29541b |
| OpenZeppelin/stellar-contracts | ddc327eb92d63709436da3a8455f641c2d5476c5 (v0.8.0) |
| untangledfinance/oz-policy-builder | 43b13ed1d9c6662afd74f446006e69bfcf21259b |

Rust 1.97.1 (driver, Perch contracts), 1.98.1 (OpenZeppelin contracts),
stellar-cli 27.0.0, Docker for the interpreter's Linux build.

## Hashes

| Artifact | SHA-256 |
| --- | --- |
| perch_account.wasm | 3038f5dc50ad803acfd7567f10deabc0b24cedb2b8d5b7ea83030418b4914527 |
| perch_doc_compiler.wasm | 60f470704831852e67835379187a32058d9636b3783bf5ad9e625a29fc8da86f |
| perch_interpreter.wasm | 4191ce8271035711c7bf9822d7cb20fcd0f300bf772b8defcc2279992440da6f |
| perch_spending_limit.wasm | ccaa8f0404279aab8005791bffcbc819d8325c7abc114366cb6ef3472643f59b |
| perch_ed25519_verifier.wasm | 27a733ceb0bda62fc9f645f3a708de73afccfdde9d4b2ffb30f44a2b1b862e7c |
| verifiers_stub.wasm | 65a4168ba2123c22dee2d30e0341f420db1dae59bef7b207ba0c8da3f55f5d14 |
| oz_account_0_8_0.wasm | 8286422f205951f37f4249401b403471fc432340dde6925432d31bb8a90cce27 |
| oz_spending_limit_0_8_0.wasm | 65e70aad310bcaf7025aedc97e70cde098e1dd6c22d5153879c9cc64ea7da8e2 |
| policy_interpreter.wasm | 6704996f626904a4e33b95bda17b73963fc9cf502ffdfdbe9d85b4b10288e0d4 |
| reactor (driver, wasm32-wasip1) | a2352156fda3b91648dba2c218f883e248f20eac2c5ee1bb59cce699c13bd0da |
| reference replies minus timings (native = WASI = browser) | 88bf930eea70cb7344c912049c77e44b134b3806993c12cc7c4edabd52a38136 |
| index.html | 1056d32dee67ec94d318ef63b20188b04c7f1d61975a7362551e319802ec8796 |

The nine contract hashes equal the walkthrough's (`docs/oz-walkthrough`), the
Perch paper's Table 9 and oz-policy-builder's recorded testnet deployment.

## Embedding

- `index.html?embed=1`: the controls, running totals and request log only,
  with a one-line credit and a link to the full page.
- `?theme=light` or `?theme=dark`: the theme; otherwise `prefers-color-scheme`.
- In an iframe the page posts `{type: 'perch-sim:height', height}` to its
  parent whenever its content height changes, and accepts
  `{type: 'perch-sim:load', commands: [...]}` from its parent: it resets,
  installs the rule and runs the commands in order. Commands use the driver's
  shape and the controls' bounds: `{op: 'pay', amount_xlm: 1–100, to:
  'shop'|'mallory', signer: 'app'|'forged'|'unlisted'}`, `{op: 'advance',
  ledgers: 1–10000}`, `{op: 'oz_policy', spending_limit: 'on'|'off'}`; at most
  60, with an optional leading `{op: 'reset'}` and `{op: 'install'}`. A
  message with any other shape is ignored.
- `?scenario=<base64url of the same JSON array>` runs a scenario on load.
- Element ids: `#sim-controls`, `#sim-totals`, `#sim-log`, `#sim-reference`
  (reference table, rows `#sim-ref-0` to `#sim-ref-14`), `#sim-why`,
  `#sim-sources`, `#sim-notices`.
