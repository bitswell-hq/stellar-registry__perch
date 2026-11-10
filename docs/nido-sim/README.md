# Nido in soroban-sim

`index.html` is one self-contained, offline page (5.3 MB; the 3.4 MB
simulator inside it is compiled only on the first press). The reader creates
a Nido account in soroban-sim the way Nido does, gives an app a Perch grant,
then plays a guessing game and buys a coffee. Each request opens an approval
sheet in Nido's look, which checks the transaction in the page's simulator
first. After an approval the page shows Perch checking every node of the
authorization tree.

## The approval sheet: minimal by default

The sheet shows only what a first-time viewer needs to decide; everything
else is behind one Details control.

- **Header:** Nido, the network chip ("Local network") and the requesting
  app. No fee line and no title line.
- **Grant install:** a short lead ("Let Guess & Coffee spend without asking
  you"), then one row per asset with the amount, the window in human time and
  the call it is allowed inside ("2 XLM · every ~8 min · in Game › guess").
  The rows describe only what the rules enforce: they name no recipient,
  because the grant's rules do not fix one. Caution chips mark the risky
  facts the rules contain ("no end date", "any function on Game"). An "Ends"
  row appears only when a rule has an end.
- **Payment or contract call:** the amount and the recipient of the signed
  transfer large ("1 XLM", "to Game"), one line for what the call does ("Play
  guess 3 on Game"), and the budget left in small type ("1 of 2 XLM left for
  ~8 min after this"; "Over the limit: …" in the danger colour). A call that
  moves no money says so.
- **Verdict:** a quiet "✓ Checked" line when the check passes. A filled box
  only on refusal or failure ("✕ Your account would refuse this: the payment
  must happen inside Game › guess"); then Reject is the filled primary and
  "Sign anyway" a text button that asks to confirm. Warnings from the
  decoder's comparison of the signed tree with the simulation's recording
  (an extra, differing or missing entry) are always shown as alerts.
- **Primary labels** name the action: "Install grant", "Play, pay 1 XLM",
  "Pay 3.50 EURC", "Pay 1 XLM", "Play".
- **Details** (collapsed for every request): what you sign (the decoded
  calls), the grant's rules in full (functions, conditions, root, cap in
  ledgers and time, recipient set by the call, end, signer, how to remove
  it), why it is allowed (the rule matched per node and the result of its
  checks), the simulation (balances, events, ledger and protocol) and the
  addresses (full ids, the signer key, the network fee).
- **Keyboard:** a native modal `<dialog>` labelled by the lead and described
  by the verdict; focus starts on Reject and returns to the button that opened
  the sheet; Details is a native `<details>` with a 44 px target; Escape
  rejects.

Windows convert ledgers to time at about 5 s per ledger (100 ledgers is about
eight minutes), as the paper and the decks say.

**Which parts are which.** The sheet is this simulation's own presentation.
Its facts come from the transaction bytes, decoded with the vendored
soroban-sign-dialog's exported decoder (`decodeEnvelope`, `decodeAuthEntry`,
`compareAuth`, `decodeEvent`; soroban-sim `fb38b9c`, unchanged,
`docs/sign-dialog/`), and from the page's simulator: the request signed with
the stand-in keys, run with enforcing authorization and, when the host
accepts it, applied on a checkpoint of the ledger, read, and rolled back. The
component's own dialog element is not used on this page, because the layout
above needs more than its public options (`strings`, slots, `::part()`,
`details`); the requests below say what it would need. The page's step
results, cards and checks are as before: a one-line result under each step
with "Show the checks", collapsed cards whose failing node opens first, and
the host trace, signed payload, sources and recorded-run comparison one level
deeper.

## Requests for the component

From a design review of this page and its dialog, for `@theahaco/soroban-sign-dialog`, as requests, not forks:

1. Reset Details to collapsed on each new request (today the open state
   carries over).
2. Name the signer's outgoing payments in the summary line ("…, paying 1
   XLM"), for example a `summaryCall` string given the call and the signer's
   transfers.
3. Summarize a grant install's rules and caps in the summary line (an
   `installGrant` string given the decoded policy).
4. Drop the short address after a labelled name in the default view, or
   expose it as a part.
5. Let the Details label differ when open and closed ("Show details" and
   "Hide details").

From the approval-screen design applied in this version (the page renders
its own sheet until the component offers these):

6. Layouts per kind of request: for a grant install, a lead and a limits
   block decoded from the policy (one row per asset: amount, window in human
   time, the root call it is allowed inside; caution chips for no end date,
   any function or any recipient); for a payment or call, the amount and the
   recipient of the signer's transfer as the lead, one line for the call, and
   the budget left as a secondary line.
7. A quiet success state (a one-line "✓ Checked", no filled box) beside the
   loud refused and failed states, for example the state reflected on the
   host (`data-state`) or a part per state.
8. On a refusal or failure, Reject as the primary button and the risky path
   as a text button behind a second confirm, with its labels as strings
   ("Sign anyway", "Confirm: sign anyway").
9. Specific primary labels per request (a string given the request kind and
   its amount: "Install grant", "Play, pay 1 XLM", "Pay 3.50 EURC").
10. An option to drop the fee line when someone other than the signer pays,
    keeping the exact fee in Details; and windows in human time from a ledger
    close time, with the ledger count in Details.

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
   the window holds 2 XLM (cap). Choose "Sign anyway" in the sheet to see
   the account refuse it (contract error 5); the page shows which node and which
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
  fits a 1300×600 or 360×650 frame without horizontal scroll; the approval
  sheet opens inside the frame as an in-page modal (a bottom sheet on a phone).
- `?theme=light` or `?theme=dark` sets `data-theme` on `<html>`; otherwise the
  page follows `prefers-color-scheme`. (The sheet keeps Nido's light look.)
- `?autostart=1`: creates the account on load (starts the simulator), so a
  frame the reader opened needs no second press. Nothing takes focus.
- `?scenario=<base64url JSON array>`: creates the account, installs the
  grant, then sends each request approved, without opening the sheet.
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
  `#nido-reference`, `#nido-sources`, `#nido-notices`, `#nido-sheet` (the
  approval sheet, a native `<dialog>`).

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
| index.html (5331561 bytes) | aa87fa18e4ebbf589280562fdb78ab8ced1624456e08091aa700ce6e850bdb8a |

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
