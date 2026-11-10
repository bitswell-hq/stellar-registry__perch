# Perch deck

`index.html` is a 28-slide deck on Perch. It is one self-contained file with inline SVG, CSS and JS and system fonts, and it makes no network requests outside this tree. Slides 11–16 are one live session of the Nido simulation, in one frame of `../nido-sim/index.html?embed=1&present=live&preload=1` that the deck loads a moment after it opens, with stills from `nido-stills/` as the fallback. Slide 21 frames the dual simulation (`../perch-paper/sim/index.html?embed=1`), loading only when "Run it here" is pressed. The café slide links to the sign dialog demo in Nido's look (`../sign-dialog/nido.html`).

Controls:

- Navigation: arrows, Space or Shift+Space, PageUp/PageDown, and Home/End; swipe, or tap either side of the stage.
- `N` shows each slide's speaker notes, which carry its sources. `T` switches between light and dark; until then the deck follows the OS.
- On slides 11–16, "Reset demo" in the corner, or `D`, starts the Nido demo over (below).
- `#n` opens slide n, and `?still=1` shows every slide's final drawing without motion. Reduced motion has the same effect.

Solid cyan in an engraved frame is what Perch does, as the main paper presents it. Dashed magenta with printer's marks is where it could go. Slide 24, the planned agent runtime, and slides 25–27, on a zero-knowledge email proof as a recovery credential, are of that kind. Slide 22, marked "a cited study", draws another team's published experiment in plain ink and links the paper.

## Nido, live

Slides 7–10 draw how Nido creates an account, installs the grant and checks a nested payment. Slides 11–16 then run it, as one continuous session of the Nido simulation (`../nido-sim/`, 5.4 MB, offline in soroban-sim). The simulated parts are labelled in the page: the passkey is a software key, the factory is rebuilt on this stack's account code, and the EURC token, the game and the shop exist only in the simulation and demos.

The deck makes one frame and never reloads it or moves it to another parent, which would lose the session. The frame lies in a fixed layer over the current slide's slot (`[data-show]`) and is hidden, not removed, on the other slides. Once the frame posts `{type:"nido-sim:preloaded"}`, entering a slide sends `{type:"nido-sim:show", controls, step}` with that slide's controls. Going back sends the earlier one, and showing never resets or runs anything.

| Slide | `controls` | What you do |
| --- | --- | --- |
| 11 | `create` | Create the account: a simulated passkey prompt, then Continue |
| 12 | `grant` | Install the grant on its sign sheet: 2 XLM only to the game, 10 EURC only to the shop |
| 13 | `guess` | Play guess (3, a loss) |
| 14 | `coffee` | Buy a coffee |
| 15 | `direct`, `other` | Pay the game directly, or lose through `game.other`: refused; Reject |
| 16 | `overcap`, `owner` | Lose past the cap twice (after slide 13's guess, the second is refused), or lose to the game's owner: refused |

A sheet left open stays open on the next slide until you decide. Arrow and Page keys pressed in the frame arrive as `{type:"nido-sim:key"}` and move the deck, and after each change the deck takes its other keys back. "Reset demo" or `D` sends `{type:"nido-sim:reset"}` (a new ledger, no account, the simulator kept) and returns to slide 11. The SCF Build deck uses the same code.

If the frame never posts `preloaded` (a copy without `../nido-sim`, a slow device), each slide shows a still of its step and stays usable. Each slot holds four: `nido-stills/<light|dark>-<state>-<wide|narrow>.webp`, for the states `account`, `grant`, `guess`, `coffee`, `direct`, `other`, `overcap` and `owner`. That is 32 files, about 870 KB. They are screenshots of the page's `present=sheet` URL form, made like this:

1. Open `../nido-sim/index.html?embed=1&present=sheet&theme=<light|dark>&<parameters>`, with the parameters from the table in `../nido-sim/README.md` ("Embedding"); `account` uses `autostart=1&step=create`.
2. Use a wide or a narrow viewport:
   - wide, at device scale 1.5: 1300×620 CSS px for `grant`, `guess` and `coffee`, 700×620 for `account`, and 650×620 for the four refusals;
   - narrow: 360×640 at device scale 2.
3. Wait for `<html data-nido-ready>` to be set, then 700 ms, and screenshot the viewport.
4. Convert each PNG with `cwebp -quiet -q 80 -m 6`.

The current set comes from `docs/nido-sim/index.html` with SHA-256 `4f3056d0…`.

## Thumbnail

`thumbnail.svg` is a 16:9 card for the deck: the title slide's dial and the deck's two guilloché seals, drawn with the deck's own seal code (the same digests and geometry) in its dark palette. `thumbnail.py` writes it (`python3 thumbnail.py thumbnail.svg`, standard library only). `thumbnail.png` (1280×720) and `thumbnail-1920.png` (1920×1080) are Chromium renders of it at 1× and 1.5×. Its text uses the deck's system fonts, Didot and Optima; other systems substitute the deck's fallbacks.

## Sources

| Slides | Source |
| --- | --- |
| 1–2 | `docs/perch-paper/source/facts/paper.json`; `source/html/abstract.html`; `source/html/sections/motivation.html` |
| 3 | `motivation.html`; `source/figures/fig-shop-grant.{a,b}.txt`; `source/facts/example.json`; `crates/perch-smart-account/src/lib.rs` (`apply_scope`, `remove_scope`) |
| 4 | `motivation.html`, `authorization.html`; `source/statements.json` (T4, T5); `source/facts/proofs.json` (`attenuation-steps`); `source/facts/example.json` (`shop-cap-xlm`, `shop-period-ledgers`, `grant3-cap-xlm`); the attenuation model, `source/figures/fig-attenuation.text.html` and `build/figures/fig_attenuation.py` |
| 5 | `source/html/sections/grants.html`; `source/figures/fig-cap-window.steps.json`; `source/facts/example.json` (`cap-event-1`–`5`); T6 |
| 6 | `source/html/sections/authorization.html`; `source/figures/fig-auth-path.steps.json`; `crates/perch-smart-account/src/auth.rs` |
| 7 | Nido 97105a0b `contracts/factory/src/contract.rs`, `packages/frontend/src/lib/policy/docDraft.ts`, `DEPLOYED.md`; `crates/perch-smart-account/src/lib.rs` (the admin rule); `docs/nido-sim/README.md` |
| 8 | `motivation.html` (`apply_scope`, the document compiler); `docs/auth-root.md`; `docs/nido-sim/README.md` |
| 9 | `crates/integration-tests/tests/auth_root.rs`; `docs/auth-root.md`; `crates/perch-ir/src/doc.rs` (`AuthRoot`); `docs/nido-sim/README.md` (the recipient pin) |
| 10 | The simulation's coffee shop and its simulated EURC test token (`docs/nido-sim/`; simulation and demos only); the rule conditions from `docs/auth-root.md` |
| 11–16 | `docs/nido-sim/index.html`, the Nido simulation page, and its `README.md` ("Embedding": the live view, its messages and the still URLs); `docs/auth-root.md` and `crates/integration-tests/tests/auth_root.rs` (the root and recipient refusals); slides 7–10 for the facts it runs |
| 17 | `motivation.html`, `grants.html`, `source/html/sign-dialog.html`; `docs/sign-dialog/nido.html`, `NOTICE.md` |
| 18–19 | `source/html/sections/simulation.html`, `conclusion.html`; `source/facts/sim.json`; `source/figures/fig-sim.run.json` (commands 2–8) |
| 20 | `source/html/sections/proofs.html`; `source/statements.json` (T1, T2, T4, T6, T8, T10; P1 is argued) |
| 21 | `simulation.html`, `repro.html`; `source/facts/sim.json`; `fig-sim.run.json` (pins) |
| 22 | Breckenridge, Vilardell, Wang, Zhang and Juels, "I-HITL: Inalienable Human-in-the-Loop", [arXiv:2610.35835](https://arxiv.org/abs/2610.35835) (v3, 30 October 2026): p. 4 (Setup, Experiments, Key Findings), p. 5 (Results), Appendix D (p. 18). The deck does not name the ten models; the paper does. |
| 23 | `source/statements.json` (T8, grant target containment); `crates/perch-smart-account/src/lib.rs` (the admin rule, `apply_scope`, `remove_scope`); `docs/nido-sim/README.md` (step 1, the passkey); `source/facts/example.json` (the shop's cap). This is not the paper's I-HITL protocol. |
| 24 | Proposed, not built: vision paper §08 "Agents, budgets and payments (proposed)" and §09 "Cedar as an authoring language (proposed)" (`source/papers/vision/html/sections/agents.html`, `cedar.html`), for what they state: agents hold no keys, the key service, child grants derived from a root grant, x402, the account rechecking on chain, and Cedar as an authoring language; `docs/cedar-walkthrough/` (the compiler prototype); `source/statements.json` (T4, T5). The WASI packaging, the import-free Cedar engine and the every-ancestor rule are the planned runtime design, not taken from those sections |
| 25–27 | Perch side: `crates/perch-smart-account/src/auth.rs`, `crates/perch-recovery-interface/src/{credential,config}.rs`, `crates/perch-recovery/src/contract.rs` (`submit_guardian`), `docs/recovery/statement.md`, vision paper §02 and §05. Email side (proposed for Perch): the ZK signer deck's report, citing Nido's `docs/email-recovery/spec.md` |
