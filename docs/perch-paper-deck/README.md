# Perch, the paper, in pictures

`index.html` is a 27-slide deck that follows the academic Perch paper (`docs/perch-paper/latex/`, built to `latex/out/perch-paper.pdf`), section by section and in the same order. It keeps the paper's claims, theorems and evaluation numbers, and explains them through one picture:

- the account is a building with a master key, the owner's;
- a grant is a keycard with its limits printed on it;
- attenuation is cutting a copy that opens fewer doors;
- the cap window is an allowance meter over the last 100 ledgers;
- the authorization path is the front desk;
- OpenZeppelin's composition is guards at a door;
- soroban-sim is a full-size test building;
- the Lean proofs are an inspector comparing a tracing of the lock with its blueprint;
- a completed recovery re-keys every earlier card.

Each slide carries an "in the paper" plaque naming its section, figure, table or theorem, using the PDF's numbering. Slide 10 carries a "beyond the paper" plaque instead: exact entry roots arrived in the stack after the paper's pin. The speaker notes (`N`) give the paper's own wording and the source files.

Brass is the card and what it allows, blue is what a theorem proves, red is a refusal, and dashed pencil is proposed.

The file is self-contained: inline SVG, CSS and JS, and system fonts. It makes no network requests. The controls are the same as the first deck's (`../perch-deck/`): arrows, Space, PageUp and PageDown, and Home/End; swipe or tap; `N` for notes; `T` for the theme; `#n` for slide n. `?still=1` shows final drawings, and reduced motion has the same effect.

## Sources

| Slides | Paper | Source |
| --- | --- | --- |
| 1 | Title, abstract | `latex/perch-paper.tex`, `sections/abstract.tex`; `source/facts/paper.json` |
| 2–4 | §1, Fig. 1, Theorems 2, 6, 8 | `sections/motivation.tex`; `source/figures/fig-shop-grant.*.txt`; `source/facts/example.json` |
| 5 | §2 | `sections/background.tex` |
| 6–9 | §3, Fig. 2, Theorems 4, 6, 8 | `sections/grants.tex`; `source/figures/fig-cap-window.steps.json`; `source/facts/example.json`, `limits.json`; Appendix B (`sections/repro.tex`) |
| 10 | Beyond the paper: the stack since c593bac (§12 lists exact roots as proposed) | `crates/integration-tests/tests/auth_root.rs`, `docs/auth-root.md`, `crates/perch-ir/src/doc.rs` (`AuthRoot`) |
| 11 | §4, Fig. 3 | `sections/authorization.tex`; `source/figures/fig-auth-path.steps.json` |
| 12–13 | §5, Table 1 | `sections/threat-model.tex`; `source/tables/tab-grantee.json` |
| 14–15 | §6, Table 2 | `sections/compare.tex`; `source/tables/tab-choose.json` |
| 16–17 | §7, Table 3 | `sections/simulation.tex`; `source/figures/tab-sim.table.json`; `source/figures/fig-sim.run.json` |
| 18–21 | §8, Theorems 1, 2, 6, 8 | `sections/proofs.tex`, Appendix B; `source/statements.json`; `source/facts/proofs.json` |
| 22 | §9 | `sections/evaluation.tex`; `source/facts/testnet.json`, `proofs.json` |
| 23–26 | §10–§13 | `sections/limits.tex`, `related.tex`, `agents.tex`, `conclusion.tex` |
| 27 | Appendix A | `sections/repro.tex` |

All paths are under `docs/perch-paper/` unless they start with `sections/`, which means `docs/perch-paper/latex/sections/`.

The charges on the Theorem 6 slide are illustrative. They are accepted only while the last 22 units stay within 10, as the theorem's rule requires.
