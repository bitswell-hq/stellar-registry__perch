# Perch deck

`index.html` is a 23-slide deck on Perch. It is one self-contained file with inline SVG, CSS and JS and system fonts, and it makes no network requests. Two slides frame relative pages in this tree, each loading only when "Run it here" is pressed: the Nido simulation (`../nido-sim/index.html?embed=1`) and the dual simulation (`../perch-paper/sim/index.html?embed=1`). The café slide links to the sign dialog demo in Nido's look (`../sign-dialog/nido.html`).

Controls:

- Navigation: arrows, Space or Shift+Space, PageUp/PageDown, and Home/End; swipe, or tap either side of the stage.
- `N` shows each slide's speaker notes, which carry its sources. `T` switches between light and dark; until then the deck follows the OS.
- `#n` opens slide n, and `?still=1` shows every slide's final drawing without motion. Reduced motion has the same effect.

Solid cyan in an engraved frame is what Perch does, as the main paper presents it. Dashed magenta with printer's marks is where it could go. Slide 19, the planned agent runtime, and slides 20–22, on a zero-knowledge email proof as a recovery credential, are of that kind. Slide 17, marked "a cited study", draws another team's published experiment in plain ink and links the paper.

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
| 11 | `docs/nido-sim/index.html`, the Nido simulation page, and its `README.md` |
| 12 | `motivation.html`, `grants.html`, `source/html/sign-dialog.html`; `docs/sign-dialog/nido.html`, `NOTICE.md` |
| 13–14 | `source/html/sections/simulation.html`, `conclusion.html`; `source/facts/sim.json`; `source/figures/fig-sim.run.json` (commands 2–8) |
| 15 | `source/html/sections/proofs.html`; `source/statements.json` (T1, T2, T4, T6, T8, T10; P1 is argued) |
| 16 | `simulation.html`, `repro.html`; `source/facts/sim.json`; `fig-sim.run.json` (pins) |
| 17 | Breckenridge, Vilardell, Wang, Zhang and Juels, "I-HITL: Inalienable Human-in-the-Loop", [arXiv:2610.35835](https://arxiv.org/abs/2610.35835) (v3, 30 October 2026): p. 4 (Setup, Experiments, Key Findings), p. 5 (Results), Appendix D (p. 18). The deck does not name the ten models; the paper does. |
| 18 | `source/statements.json` (T8, grant target containment); `crates/perch-smart-account/src/lib.rs` (the admin rule, `apply_scope`, `remove_scope`); `docs/nido-sim/README.md` (step 1, the passkey); `source/facts/example.json` (the shop's cap). This is not the paper's I-HITL protocol. |
| 19 | Proposed, not built: vision paper §08 "Agents, budgets and payments (proposed)" and §09 "Cedar as an authoring language (proposed)" (`source/papers/vision/html/sections/agents.html`, `cedar.html`), for what they state: agents hold no keys, the key service, child grants derived from a root grant, x402, the account rechecking on chain, and Cedar as an authoring language; `docs/cedar-walkthrough/` (the compiler prototype); `source/statements.json` (T4, T5). The WASI packaging, the import-free Cedar engine and the every-ancestor rule are the planned runtime design, not taken from those sections |
| 20–22 | Perch side: `crates/perch-smart-account/src/auth.rs`, `crates/perch-recovery-interface/src/{credential,config}.rs`, `crates/perch-recovery/src/contract.rs` (`submit_guardian`), `docs/recovery/statement.md`, vision paper §02 and §05. Email side (proposed for Perch): the ZK signer deck's report, citing Nido's `docs/email-recovery/spec.md` |
