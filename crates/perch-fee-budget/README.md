# Perch fee budget

A treasury-consented funding component for native Perch document rules.
The fixed executor must require the enrolled controller C before calling
`refill()`. Settlement always pays registered F and enforces the immutable
lifetime cap, irrespective of later account-rule changes.

See [the integration guide](../../docs/fee-budget.md) for the grant document,
authority boundary, tests, proof scope, and release policy. This contract does
not implement the removed OZ policy callbacks and adds no new document field.
