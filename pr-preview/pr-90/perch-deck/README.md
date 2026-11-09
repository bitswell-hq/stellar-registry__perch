# Perch deck

`index.html` is a 16-slide deck on Perch. It is one self-contained file with inline SVG, CSS and JS and system fonts, and it makes no network requests. Two slides frame relative pages in this tree: the sign dialog in Nido's look (`../sign-dialog/nido.html`, which loads when its slide opens) and the dual simulation (`../perch-paper/sim/index.html?embed=1`, which loads only when "Run it here" is pressed).

Controls:

- Navigation: arrows, Space or Shift+Space, PageUp/PageDown, and Home/End; swipe, or tap either side of the stage.
- `N` shows each slide's speaker notes, which carry its sources. `T` switches between light and dark; until then the deck follows the OS.
- `#n` opens slide n, and `?still=1` shows every slide's final drawing without motion. Reduced motion has the same effect.

Solid cyan in an engraved frame is how Perch works at the paper's pinned build. Dashed magenta with printer's marks is where it could go. Slides 13–15, on a zero-knowledge email proof as a recovery credential, are of that kind.

## Sources

| Slides | Source |
| --- | --- |
| 1–2 | `docs/perch-paper/source/facts/paper.json`; `source/html/abstract.html`; `source/html/sections/motivation.html` |
| 3 | `motivation.html`; `source/figures/fig-shop-grant.{a,b}.txt`; `source/facts/example.json`; `crates/perch-smart-account/src/lib.rs` (`apply_scope`, `remove_scope`) |
| 4 | `motivation.html`, `authorization.html`; `source/statements.json` (T4); `source/facts/proofs.json` (`attenuation-steps`); `source/facts/example.json` (`grant3-cap-xlm`) |
| 5 | `source/html/sections/grants.html`; `source/figures/fig-cap-window.steps.json`; `source/facts/example.json` (`cap-event-1`–`5`); T6 |
| 6 | `source/html/sections/authorization.html`; `source/figures/fig-auth-path.steps.json`; `crates/perch-smart-account/src/auth.rs` |
| 7–8 | `motivation.html`, `grants.html`, `source/html/sign-dialog.html`; `docs/sign-dialog/NOTICE.md` |
| 9–10 | `source/html/sections/simulation.html`, `conclusion.html`; `source/facts/sim.json`; `source/figures/fig-sim.run.json` (commands 2–8) |
| 11 | `source/html/sections/proofs.html`; `source/statements.json` (T1, T2, T4, T6, T8, T10; P1 is argued) |
| 12 | `simulation.html`, `repro.html`; `source/facts/sim.json`; `fig-sim.run.json` (pins) |
| 13–15 | Perch side: `crates/perch-smart-account/src/auth.rs`, `crates/perch-recovery-interface/src/{credential,config}.rs`, `crates/perch-recovery/src/contract.rs` (`submit_guardian`), `docs/recovery/statement.md`, vision paper §02 and §05. Email side (proposed for Perch): the ZK signer deck's report, citing Nido's `docs/email-recovery/spec.md` |
