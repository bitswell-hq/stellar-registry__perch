# Changelog

All notable changes to this component are documented here. Contract versions
are the on-chain wasm publishes and `perch-js` versions are the npm releases of
`@stellar-registry/perch`; the format follows [Keep a Changelog](https://keepachangelog.com).

## [0.4.0] - 2026-10-15

### 🚀 Features

- Versioned JSON Schema for PolicyDoc (#120)

### 🛡️ Security

- [**breaking**] `signingDigest(account, signaturePayload, ruleIds)` takes the account and returns the account-bound `addr_oz_v1` digest, `sha256(sha256("perch/account/auth-addr-v1" || account_id || signature_payload) || xdr(rule ids))`. The 0.3.x two-argument form signed OZ's digest without the account address, which accounts built from the current wasm refuse.
- [**breaking**] `checkCapabilities`, and so `readSnapshot`, accept only accounts whose `capabilities().auth_digest` is `addr_oz_v1` and throw `UnsupportedCapability` for `oz_rule_ids`. perch-js 0.3.x throws for every account that reports `addr_oz_v1`.

## [0.3.2] - 2026-10-10

### 💼 Other

- One controller and the shared account capabilities (epic #99 WS3) (#102)
- New deployables, pin-ordered stack, manifest pipeline, release-stack harness, packages (epic #99 WS4) (#103)

## [0.3.0] - 2026-09-24

### 🚀 Features

- *(recovery)* Stage 4 — recovery schema, shared controller, compiler + client support

## [0.2.0] - 2026-09-09

### 🚀 Features

- Interpreter TS bindings (@stellar-registry/perch-interpreter) + perch-js threshold/cap parity (#77)

## [0.1.1] - 2026-09-09

### 🚀 Features

- *(perch-js)* Schema, canonical JSON + doc_hash parity, and builder (#17)
- Canonical-form spec + CANON_VERSION; own the escaper, not the serializer (#19) (#29)
- *(perch-js)* ERC-7715-shaped permission request → PolicyDoc; TS cap parity (#27)
- Publish @stellar-registry/perch to npm + verified testnet deployment map (#76)

### 💼 Other

- The `admin-root` rule → `admin` (closes #50) (#51)


