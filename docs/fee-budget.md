# Fee funding with native Perch rules

Keep the fee key F in the wallet. A treasury grants the `perch-fee-budget`
contract a token allowance. A Perch account C authorizes a fixed executor's
zero-argument `refill`; the executor requires C and calls the budget. The
budget requires that executor, reads F's actual token balance, computes the
refill, updates lifetime spent, and transfers only to its registered F.

```
wallet passkey → native Perch rule → executor.refill()
                                      ↓
treasury SAC allowance → fee budget → registered F → network fees
```

## What is the policy?

There are two complementary checks. Perch's document says **who may invoke
this executor**. The reusable funding component says **where treasury funds
may go and how much may be spent over this deployment's lifetime**.
Native Perch dev evaluates document rules directly. This component does not
reintroduce OZ callbacks and does not add a `fee-budget` JSON keyword.

A dedicated grant uses the existing document schema:

```json
{
  "version": 1,
  "network": "<network passphrase>",
  "signers": [{"id":"owner-passkey","verifier":"<verified verifier C>","key":"<P256 public key hex>"}],
  "rules": [{
    "name":"refill-local-fee-account",
    "scope":{"type":"contract","address":"<fixed executor C>"},
    "principals":{"type":"all","signers":["owner-passkey"]},
    "functions":["refill"],
    "not-after-ledger":123456,
    "auth-root":{"contract":"<fixed executor C>","function":"refill"}
  }]
}
```

Replace placeholders, enroll the same passkey as C, and apply this with
`apply_grant` under owner authorization. Read back the grant and its allocated
rule IDs; never assume an ID is 1. The account's owner can also use
`C.execute(executor, refill, [])`. Both routes encounter the same settlement
budget. Nido's isolated example exercises this native owner route; deployment
of the dedicated grant is a separate integration step.

The executor is part of the enrollment's trusted configuration. Review its
immutable code and its C binding before approving the allowance. It must
require C before calling this budget and offer no arbitrary forwarding route.
The constructor requires treasury consent. `Config.controller` records that
binding for wallet verification; authorization at settlement is the configured
executor's. Initial funding, other deposits, and other budget deployments are
outside this budget's cap.

## Funding state

One deployment fixes treasury, token, controller, executor, F, low mark,
target, cap, and expiry. There is no recipient setter, allowance reset,
resume, or upgrade entry point. The treasury can irreversibly pause it.
Changing Perch documents, reapplying grants, or renewing the SAC allowance
cannot reset spent. Missing state fails closed. Failed transfers roll back
spent under Soroban transaction atomicity. `checks` counts successful refills,
not policy preflights or attestations.

`crates/perch-fee-budget-core/proof/` establishes fixed recipient and bounded
lifetime spending for the extracted planner. Contract tests cover actual SAC
balances/allowances, rollback, expiry, pause, and missing executor auth.
Those mocked-auth unit tests do not invoke C's `__check_auth`; Nido's real
passkey/local-ledger check must cover that integration.

## Build and release

```
cargo test -p perch-fee-budget --locked
cargo clippy -p perch-fee-budget -p perch-fee-budget-core --all-targets --locked -- -D warnings
stellar contract build --package perch-fee-budget --locked
```

The release workflow tracks this contract and its core dependency. It is
intentionally absent from auto-publish allowlists. No live deployment or
Turnkey integration is included. Turnkey can authorize the treasury's bounded
allowance without holding the local F key.
