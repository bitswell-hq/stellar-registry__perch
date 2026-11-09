# Nido in soroban-sim

`index.html` is one self-contained, offline page (5.3 MB; the 3.4 MB
simulator inside it is compiled only on the first press). The reader creates
a Nido account in soroban-sim the way Nido does, gives an app a Perch grant,
then plays a guessing game and buys a coffee. Each request goes through the
soroban-sign-dialog in Nido's look; the dialog's preview runs the request in
the page's simulator first. After an approval the page shows Perch checking
every node of the authorization tree.

## Minimal by default

The page follows a design review of its dialog and simulation UI: the default
view says what is approved and what happened; details are one click away.

- **The dialog, default view:** one plain sentence of what you approve, with
  the amount ("Guess 3 in the game, paying 1 XLM"; at install, "Let the app
  pay the game up to 2 XLM and the coffee shop up to 10 EURC per 100
  ledgers, only inside guess and order"); the budget left ("After this: 1 of
  2 XLM left in this period", or "Over the cap: …" in the danger colour); a
  one-line verdict ("Your account will accept this", or "Your account will
  refuse this: the payment must happen inside game.guess"); any warning, as
  the component raises it; "Network fee paid by Nido's relayer" (the user
  never pays here); a full-width "Details: calls, rules, simulation" control,
  collapsed for every request; Reject and Approve.
- **Behind Details**, four groups: what you authorize (the tree), why it is
  allowed (per node, the rule matched and its root and cap checks; the cap
  meter), the simulation (balance changes, events, ledger and protocol), and
  addresses and fee (full ids, the signer key, the exact fee and its payer).
- **The page:** a one-line result under the step that caused it ("✓ Applied:
  you lost 1 XLM; 1 of 2 XLM left in this period", "✕ Refused by your account:
  the payment must happen inside game.guess"), scrolled into view in the
  embed, with "Show the checks". Request cards are collapsed to that line;
  opened, the failing node is open and passing nodes read "✓ all N checks";
  the host trace, the signed payload, the sources and the comparison with the
  recorded run sit one level deeper. "Simulated in this page" is one line with
  a "What is simulated" disclosure.

The dialog is the vendored component, unchanged. The page configures it only
through its public API: `strings` (functions that read the request the page
prepared), `::part()` styles in the page's CSS (`title` kept as the
accessible name but visually hidden, `fee`, `status-reason`,
`details-summary`), the `logo` slot, and the page's preview provider, which
adds the plain refusal reason. Two host-page workarounds stand in for
component features: a fresh `<soroban-sign-dialog>` element for each request,
so Details starts collapsed; and, because the summary line and the tree's node
lines use the same strings, the page counts the string calls of each render
(fee line, one per tree node, then the summary) so that only the summary line
gets the plain sentence. The "Addresses and fee" group is appended through the
`footnote` string.

## Requests for the component

From a design review of this page and its dialog, for `@theahaco/soroban-sign-dialog`, as requests, not forks:

1. Reset Details to collapsed on each new request (today the open state
   carries over; the page creates a fresh element per request instead).
2. Name the signer's outgoing payments in the summary line ("…, paying 1
   XLM"), for example a `summaryCall` string given the call and the signer's
   transfers.
3. Summarize a grant install's rules and caps in the summary line (an
   `installGrant` string given the decoded policy).
4. Drop the short address after a labelled name in the default view, or
   expose it as a part.
5. Let the Details label differ when open and closed ("Show details" and
   "Hide details").

Requests 2 and 3 would also remove the page's string-call counting, and a
group or slot for addresses inside Details would remove the `footnote`
workaround.

## What it runs

1. **Create your Nido account.** A one-time setup key; Nido's factory
   `get_c_address(setup_pk)` gives the address before any passkey exists; a
   passkey stand-in (soroban-sim's software P-256 key) is created; the setup
   key signs `sha256("nido/bind/v1" ‖ network id ‖ factory id ‖ passkey)`;
   the relayer submits `create_account(setup_pk, passkey, setup_sig)`; the
   relayer funds the account with 10 XLM and the test issuer mints 20 test
   EURC. The account's constructor installs one rule, `admin`: the passkey
   through Perch's WebAuthn verifier, over the account itself.
2. **Install the grant.** The passkey signs `apply_scope(1, document)` under
   the admin rule. The grant: `game` (any game function), `pay-game` (XLM
   `transfer`, argument 0 is the account, `auth-root {game, guess}`, 2 XLM per
   100 ledgers), `shop` (`order`), `pay-shop` (test-EURC `transfer`, argument 0
   is the account, `auth-root {shop, order}`, 10 EURC per 100 ledgers). The app
   key is Ed25519, checked by Perch's Ed25519 verifier.
3. **Use it.** `game.guess(n, account)`: a loss is the tree `game.guess` →
   `XLM.transfer(account, game, 1 XLM)`; a win (7) is `game.guess` alone, the
   game paying the prize itself. `shop.order(account, 3.50 EURC)` →
   `EURC.transfer(account, shop, 3.50)`.
4. **Refusals.** A direct `XLM.transfer` (root is the transfer, not
   `game.guess`); a loss through `game.other` (root `game.other`); a loss once
   the window holds 2 XLM (cap). Approve in the dialog anyway to see the
   account refuse it (contract error 5); the page shows which node and which
   check failed.

Each node's checks are rule-core `5a689a5`'s own functions (`functions_match`,
`cond_holds`, `signers_met`, `auth_root_matches`, `check_spend`, and
`authorize` over the batch), run by the page's driver beside the host on the
same contexts, stored rules and spending records. The verdict shown is the
host's; the recorded run checks that rule-core's `authorize` agrees with it on
every request.

## Embedding

- `index.html?embed=1`: the steps, the account panel and the request log,
  without the page's chrome; a one-line list of what is simulated stays. It
  fits a 1300×600 or 360×650 frame without horizontal scroll; the sign dialog
  opens inside the frame as an in-page modal (a bottom sheet on a phone).
- `?theme=light` or `?theme=dark` sets `data-theme` on `<html>`; otherwise the
  page follows `prefers-color-scheme`. (The dialog keeps Nido's light look.)
- `?autostart=1`: creates the account on load (starts the simulator), so a
  frame the reader opened needs no second press. Nothing takes focus.
- `?scenario=<base64url JSON array>`: creates the account, installs the
  grant, then sends each request approved, without opening the dialog.
  Requests: `{op:"guess", number:0–9, entry?:"guess"|"other"}`,
  `{op:"order", cents:1–5000}`, `{op:"transfer", token:"xlm"|"eurc",
  cents:1–5000}`, `{op:"advance", ledgers:1–10000}`, `{op:"install_grant"}`;
  at most 40. Anything else is ignored.
- In an iframe the page posts `{type:"nido-sim:height", height}` to its parent
  when its height changes, and accepts `{type:"nido-sim:load", commands:[…]}`
  (the same requests; `commands: []` only creates the account).
- Without a parameter the page loads nothing heavy: the 3.4 MB reactor is
  compiled on the first press.
- Element ids: `#nido-controls`, `#nido-result-1` to `#nido-result-4` (each
  step's one-line result), `#nido-status` (visually hidden polite live
  region), `#nido-totals`, `#nido-log` (collapsed cards `#nido-request-N`),
  `#nido-reference`, `#nido-sources`, `#nido-notices`, `#nido-dialog` (a
  fresh element per request).

## Pins

| Source | Revision |
| --- | --- |
| theahaco/soroban-sim | d02938430f0ac41f34e5e3e4c8c48d85b871bd68 (soroban-env-host 29.0.2, protocol 29) |
| bitswell-hq/stellar-registry__perch | 2c64fe483f00e38061dd777bc9194f285b19721d (exact authorization roots) |
| bitswell/soroban-rule-core | 5a689a5760d9a94ef92e5182b15653bed21f7670 |
| nidohq/nido | 97105a0b54763d27f70d65c0d54239fe33bf777f |
| soroban-sign-dialog | theahaco/soroban-sim fb38b9c835e66dcaacf293d1f166bf2da45c1a91, `dist/soroban-sign-dialog.js` a5239ec1…72ee |

Rust 1.97.1 (Perch, driver, game, shop), 1.96.0 (Nido's factory, its
toolchain), stellar-cli 27.0.0 (`stellar contract build`).

## Hashes

| Artifact | SHA-256 |
| --- | --- |
| perch_account.wasm | a6d7604e8b24eb91007dcd0a1c51d00842be342ab096dde422e1dc2434f5a2c8 |
| perch_doc_compiler.wasm | aa5a3eaa271620df9a3ee996af66c1836c11127fb6537cebcd1da792056cb72e |
| perch_interpreter.wasm (pinned, never called) | 4191ce8271035711c7bf9822d7cb20fcd0f300bf772b8defcc2279992440da6f |
| perch_spending_limit.wasm (pinned, never called) | ccaa8f0404279aab8005791bffcbc819d8325c7abc114366cb6ef3472643f59b |
| perch_ed25519_verifier.wasm | 27a733ceb0bda62fc9f645f3a708de73afccfdde9d4b2ffb30f44a2b1b862e7c |
| perch_webauthn_verifier.wasm | bc38017e9281839ebd683341088f5e3550881b97873d350161fa4881bc3070c3 |
| nido_factory.wasm (rebuilt, account pin a6d7604e…) | d5a6aa11569db6ca2d49ea98e579d20ce71f53f309b5b5d1ea5ce121c714f2f6 |
| nido_sim_game.wasm | 7c04ff93e363811ccdd35e08f2a30d93e541b164baac96d12a34c939b6a73874 |
| nido_sim_shop.wasm | d433a64029bf2b09a86bc7b1fd18a7dbf0d1015f557684b82e21215daad6efd4 |

| reactor (driver, wasm32-wasip1) | 1d1e2094363cfa2fe5d6d944dbbb346031ec5571c7ea5bcdeb33e261d83841eb |
| reference replies minus timings (native = WASI = browser) | 9ec27d1662a8999436d0961eb5ac122c4aefb28f2d7a405bad7e02b5e5aa7db7 |
| recorded run `evidence/native-golden.jsonl` | a77586fa99248c75855a2dcf75e056c0bac8d7b88feb4c3c277ec850e9980850 |
| index.html (5312307 bytes) | 4c4b1281fd5642649192e40c8b15bc702d433ccc8cd1ede8162da17624fedea0 |

None of the Perch artifacts is a deployed contract: the account is built
from 2c64fe4 with its compiler, interpreter and spending-limit pins staged
from the same build, in `scripts/build-stack.sh` order. The interpreter,
spending limit and Ed25519 verifier have the same bytes as the f0275f4 builds
of the dual simulation (`../perch-paper/sim/`).

## Rebuild

The page carries its build sources ("Download the build sources"). Their
`BUILD.md` fetches the pinned trees by commit and rebuilds the contracts, the
driver (native and WASI), the reference run, the page and the run record
`fig-nido.run.json`, both written by `make.sh`:

```sh
./fetch-sources.sh
python3 build-contracts.py
./make.sh index.html fig-nido.run.json
```

A plain `make.sh` records a fresh run, whose wall-clock timings differ. To
rebuild this `index.html` byte for byte, keep the recorded run:
`KEEP_EVIDENCE=1 SOURCE_DATE_EPOCH=1794182400 ./make.sh index.html
fig-nido.run.json`. It still runs the scenario natively and under WASI and
checks both against the recorded run, minus timings.
