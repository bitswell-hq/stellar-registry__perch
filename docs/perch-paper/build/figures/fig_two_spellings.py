"""fig-two-spellings: the three listing panels (plan 4.3, ASCII), with callouts <1>-<5>."""
from figlib import FIGDIR

A = [
    ('{', ''),
    ('  "version": 1,', ''),
    ('  "network": "Test SDF Network ; September 2015",', ''),
    ('  "signers": [', ''),
    ('    { "id": "app", "verifier": "C...WEBAUTHN", "key": "...65-byte passkey key, hex..." }', '4'),
    ('  ],', ''),
    ('  "rules": [', '3'),
    ('    { "name": "guess",', ''),
    ('      "scope": { "type": "contract", "address": "C...GAME" },', ''),
    ('      "principals": { "type": "all", "signers": ["app"] },', ''),
    ('      "functions": ["guess"],', ''),
    ('      "not-after-ledger": N },', '5'),
    ('    { "name": "guess-xlm",', '1'),
    ('      "scope": { "type": "contract", "address": "C...XLM" },', ''),
    ('      "principals": { "type": "all", "signers": ["app"] },', ''),
    ('      "functions": ["transfer"],', ''),
    ('      "args": [ { "index": 1, "pred": { "type": "address-eq", "address": "C...GAME" } } ],', ''),
    ('      "cap": { "limit": "20000000", "period-ledgers": 17280 },', '2'),
    ('      "not-after-ledger": N }', '5'),
    ('  ]', ''),
    ('}', ''),
]
B = [
    ('{ "calls": [', ''),
    ('  { "contract": "C...GAME", "fn": "guess" },', ''),
    ('  { "contract": "C...XLM", "fn": "transfer",', '1'),
    ('    "where": [', ''),
    ('      { "constraint": "pin", "at": [1],', ''),
    ('        "to": "C...GAME" },', ''),
    ('      { "constraint": "cap", "at": [2],', '2'),
    ('        "max": "20000000" } ] } ] }', ''),
]
C = [
    ('["and", [["eq", ["call_contract"], "C...XLM"],', ''),
    ('         ["eq", ["call_fn"], "transfer"],', ''),
    ('         ["eq", ["call_arg", 1], "C...GAME"],', ''),
    ('         ["lte", ["call_arg", 2],', '2'),
    ('                 {"i128": "20000000"}]]]', ''),
    ('', ''),
    ('{ "network": "testnet",', ''),
    ('  "smartAccount": "C...ACCOUNT",', ''),
    ('  "sourceAccount": "G...OWNER",', ''),
    ('  "intent": { "...": "(b)" },', ''),
    ('  "contract": "C...XLM",', '3'),
    ('  "signers": ["G...APP"],', '4'),
    ('  "validUntilLedger": N - 1,', '5'),
    ('  "spendingLimit": { "amount": "20000000",', '2'),
    ('                     "periodLedgers": 17280 } }', ''),
]


def panel(rows):
    col = max(len(t) for t, c in rows if c) + 1
    out = []
    for t, c in rows:
        out.append((t.ljust(col) + f'<{c}>') if c else t)
    text = '\n'.join(out) + '\n'
    assert text.isascii()
    return text, max(len(x) for x in out)


for name, rows in (('a', A), ('b', B), ('c', C)):
    text, w = panel(rows)
    (FIGDIR / f'fig-two-spellings.{name}.txt').write_text(text, encoding='ascii')
    print(name, 'max line', w)
