# perch-review-core

A Perch account's verdict on an authorization entry, computed off-chain
from a ledger snapshot, before anyone signs. It runs the account's own
`__check_auth` checks and rule-core's `rule_eval::check_auth`, reports what
each chosen capped rule has left to spend, and replays attenuation
witnesses. It has no dependency on the Soroban host, so it builds to a small
wasm module with no imports. A wallet's sign dialog takes the verdict and
the budgets from it instead of re-deriving them in JavaScript.

## What the verdict means

For the ledger entries given and the ledger sequence given, assuming every
payload signer that the request does not mark unverified authenticates, the
verdict equals the account's `__check_auth` result on that state at that
ledger. The account here is `perch_smart_account::check_auth` on the fork's
`dev` branch, which `perch-account` deploys.

The verdict depends on four assumptions:

- **Signatures.** The core does not verify signatures. It accepts a
  `verified` flag for each payload signer (`no`, `yes`, or `assumed`, the
  default). If rule-eval accepts and a signer is marked `no`, the core
  refuses as the account would: `ExternalVerificationFailed` (3003) for an
  `External` signer, a host error for a `Delegated` one. A `Delegated` signer
  that has no delegate in the entry's CAP-0071 credentials fails the same
  way, because the host's delegate check finds nothing to check. Every
  verdict lists the signers it assumed (`assumed`).
- **Snapshot.** The ledger entries must include every entry `__check_auth`
  reads for this authorization, and each must be live:
  - the account's instance entry;
  - `RuleSet`;
  - `GrantTags(id)` and `SpendRecords(id)` for each chosen id;
  - `Grants(g)` for each grant those tags name.

  `Snapshot::keys` and `Snapshot::grant_keys` list these keys. The core
  treats an entry missing from the snapshot as absent from the ledger.
- **Ledger.** The verdict holds at the given ledger sequence and nowhere
  else. Expiry, the freeze and spending windows all depend on the sequence.
  Storage can also change between review and inclusion: another spend, an
  `apply_doc` or `apply_grant`, or a recovery completion that advances the
  grant generation. Any of these can change the answer.
- **Recovery completion.** The account checks a completion against its own
  record of the recovery rule, and the core ports those checks. The
  recovery controller then decides through `rcv_complete`, and that verdict
  depends on the controller's own storage. When the account's checks pass,
  the core answers `controller` and names the controllers to ask.

Out of scope:

- Mainnet's `c135aca` account. It decides with OpenZeppelin's code, not
  this gate.
- Whether the compiled wasm matches rule-core's Lean translation. Nothing
  checks that for this build either, as for the chain build.

## Checks

`gate::review` performs the account's reads and checks in the account's
order. Line numbers refer to `fork/dev` `fc0fb2b`.

| # | Check | Contract source | Error | Core |
|---|---|---|---|---|
| 1 | No context names an invoker-only hook | `perch-smart-account/src/lib.rs:1122-1128`; names at `perch-recovery-interface/src/account.rs:119` | `ReservedFunction` (1) | `gate.rs` step 1, `RESERVED_INVOKER_ONLY_FNS` |
| – | Read the recovery rule; is this one `apply_doc` on the account choosing it? | `lib.rs:1129-1130`, `is_completion` `lib.rs:1176-1193` | – | `is_completion` |
| 2 | Under the `Protected` freeze (`ledger < until`), only a completion | `lib.rs:1131-1135`, `is_frozen` `account.rs:167` | `AccountFrozen` (2) | step 2 |
| 3 | A rule with a grant tag is never chosen for the recovery controller, the ZK adapter or pool, a guardian approval or a token-admin function | `lib.rs:1142-1162`, `excluded_contract` `lib.rs:1417-1421`, `TOKEN_ADMIN_FNS` `lib.rs:1397`, `GUARDIAN_APPROVAL_FNS` `account.rs:155` | `GrantEscalation` (4) | step 3 |
| 4a | A completion is checked against the stored recovery rule: ids match contexts, the id is the recovery rule, each context is on the account while the rule is unexpired, and no signer is named. Then the controllers decide | `lib.rs:1163-1165`, `authorize_completion` `perch-smart-account/src/auth.rs:110-144` | `ContextRuleIdsLengthMismatch` (3014), `ContextRuleNotFound` (3000), `UnvalidatedContext` (3002), `UnauthorizedSigner` (3016) | `completion_review`; `Outcome::Controller` |
| – | The account stores a rule set | `lib.rs:1166` | `RuleEvalRefused` (5) | `NoRuleSet` |
| 4b | The recovery rule is not chosen outside a completion | `perch-smart-account/src/gate.rs:85-87` | `RuleEvalRefused` (5) | `RecoveryRuleChosen` |
| 5 | Each chosen id names a stored rule: in its grant's record when tagged, otherwise in `RuleSet` | `gate.rs:91`, `stored_rule` `gate.rs:173-182`, `rule_of` `gate.rs:185-195` | `ContextRuleNotFound` (3000) | `RuleNotFound`, `stored_rule`, `rule_of` |
| – | The stored rule and a capped rule's record are readable | `gate.rs:92-97`, `record` `gate.rs:198-207`, `perch-rule-eval/src/mirror.rs` | `RuleEvalRefused` (5) | `stored::rule`, `stored::record` |
| – | Every context is a call the account can read | `gate.rs:105` (`convert::calls`) | `RuleEvalRefused` (5) | `UnreadableContexts` |
| 6 | A capped rule, or a grant rule for an owner function, acts only when argument 0 is the account | `gate.rs:107-113`, `OWNER_FNS` `lib.rs:1412` | `RuleEvalRefused` (5) | `NotOwnFunds` |
| – | rule-eval's `check_auth` over the chosen rules, their records, the payload's signers, the ledger and the grant generation | `gate.rs:114-127` | `StaleGrant` (3) for `StaleGeneration`; otherwise `RuleEvalRefused` (5) | `rule_eval::check_auth`, unchanged |
| 7 | Each payload signer authenticates, in payload order (not verified here; see above) | `gate.rs:130-135`, `authenticate` `auth.rs:47-60` | `ExternalVerificationFailed` (3003), or a host error | `verified` flags and the delegate list |
| 8 | The records the charges changed are stored | `gate.rs:136-143` | – | `Review::records` |

The core reads stored values in `stored.rs`, which is
`perch_rule_eval::mirror` applied to facade values. It accepts the same
two layouts (the eight-key legacy rule and the nine-key rule with
`auth_root`), requires keys in the same order, and refuses the same
values. The checks' name lists are copies; `perch-integration-tests`
checks each copy against its source.

## What is left to spend

`remaining(record, limit, period, now)` gives the largest amount a spend
may have. For every amount `a >= 0`:

```text
check_spend(record, limit, period, now, a).is_ok()  <=>  a <= remaining(record, limit, period, now).amount
```

A spend at ledger `L` counts through ledger `L + period - 1` and leaves the
window at `L + period`. `remaining` returns `0` when the record is ahead of
`now`, when an eviction sum overflows, or when the history is full;
`blocked` says which. `tests/remaining.rs` checks the equivalence on the
window's edges by name and on a sweep of 24,000 record, cap and ledger
cases.

Each review gives, for each chosen capped rule:

- its budget before this authorization;
- the amount this authorization requests from it (charges are summed, so
  two contexts charging one rule are counted together, as rule-eval charges
  them);
- its budget after, when the account accepts.

For a subagent's grant, the bound that rule-core proves is per chain
(`spend_accepts_iff`), so what a subagent has left is the minimum over its
ancestors, not its own cap alone.

## Attenuation witnesses

`replay_witness(parent, steps, child)` replays rule-core's ten attenuation
steps from the parent, using rule-eval's own functions. It accepts exactly
when the replay reaches the child, rule for rule (the exact root
included). rule-core proves that each step narrows (`attenuate_narrower`,
`Derives.narrower`), so an accepted child allows nothing its parent does
not. The witness does not decide whether two unrelated rule sets are
ordered. `tests/witness.rs` replays all 425 attenuation cases in rule-core's
vectors.

## Interface

The pure Rust API:

- `review_entry(entry, ledger_entries, ledger, verified)`;
- `review(&Snapshot, &Request)`;
- `remaining`;
- `replay` and `replay_witness`.

The byte interface (`envelope`) takes a binary envelope and returns JSON:

- **review:** the ledger sequence, the account's `SorobanAuthorizationEntry`
  XDR, the account's `LedgerEntryData` XDRs as `getLedgerEntries` returns
  them, and the verified flags;
- **witness:** parent rules, steps and child rules, each as `ScVal` XDR;
- **budget:** a `StoredRecord`, a `StoredCap` and a ledger sequence.

The entry parser reads `Address`, `AddressV2` and `AddressWithDelegates`
credentials, plus every kind of invocation. It treats any union arm it does
not know as unreadable rather than guessing. The npm wrapper is in `npm/`.

## Wasm builds

The browser builds use `wasm32v1-none` and `cargo rustc --crate-type cdylib`
(`npm/scripts/build-wasm.mjs`). There are two modules, so the signing path
loads only what it needs:

| Module | Exports | Raw | gzip -9 | brotli -q 11 | Imports |
|---|---|---|---|---|---|
| review (`--features abi-review`) | `review`, `budget` | 46,242 | 19,077 | 16,672 | 0 |
| review, after `wasm-opt -Oz` | | 39,125 | 18,185 | 16,076 | 0 |
| witness (`--features abi-witness`) | `witness` | 27,543 | 10,840 | 9,386 | 0 |
| witness, after `wasm-opt -Oz` | | 23,760 | 10,315 | 8,976 | 0 |
| both in one module, after `wasm-opt -Oz` | all three | 50,222 | 22,262 | 19,339 | 0 |

These are rustc 1.97.1 and wasm-opt 131 sizes, with the workspace's
release profile (`opt-level = "z"`, LTO, `panic = "abort"`, overflow checks
on). Each module exports `memory` and `alloc`. `alloc` resets an arena at
the start of each call, so memory does not grow across calls.

## Placement

This is a crate in the Perch repository rather than in rule-core:

- The core ports Perch's own gate (`perch-smart-account`), reads Perch's
  storage layout (`PerchStorage` keys, `RuleRecord`, `StoredGrant`,
  `FreezeGate`, the ZK factor), and reads stored rules exactly as
  `perch-rule-eval`'s mirror does. All of these are Perch's, not rule-core's.
- rule-core stays account-agnostic: it provides `rule_eval::check_auth` and
  the facade's codec, and this crate pins both through the workspace's
  rule-core rev.
- The differential tests need the real account at the same commit.
  `perch-integration-tests` already runs rule-core's document traces through
  it (`tests/rule_core/mod.rs`), and the core's differential is built on
  that harness.

The npm wrapper lives beside the crate (`npm/`), not under `packages/`. Every
directory under `packages/` must appear in `release.yml`'s three
`NPM_PACKAGES` lists (`scripts/test-release-scripts.py`,
`test_every_list_names_every_package`), and a listed package publishes its
manifest version on the next merge (`release.yml`, "Detect released npm
packages"). This package is not published, so it stays out of `packages/`
until the decision to publish it is made.
