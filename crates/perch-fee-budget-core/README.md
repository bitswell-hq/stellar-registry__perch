# Fee-refill decision

`plan(registered_f, budget, balance, ledger)` returns a refill only while the
balance is below the low mark and filling to the target fits the remaining
lifetime budget. The caller supplies no destination or amount.

`proof/RefillProof.lean` verifies the actual Rust function extracted by Charon
and Aeneas: accepted plans target registered F, add exactly their positive
amount to spent, stay within cap, and fill to target before expiry. A trace
lemma extends the accounting bound to committed refill sequences.

Run `bash crates/perch-fee-budget-core/proof/run.sh` from the repository root.
It requires the pinned, already-cached tool image and uses no network or paid
API. See `proof/evidence.json` for tool versions and source hashes. Only the
standard Lean axioms propext, Classical.choice, and Quot.sound are used.

This is a proof of the planner and committed-plan trace, not of the Soroban
host, token contract, key custody, deployment, or wallet application.
