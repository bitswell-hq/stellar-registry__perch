# Perch Agent Grants deck

`index.html` is a 13-slide deck for The Aha Company's SCF Build Award submission (Open Track): "Perch Agent Grants: subagent budgets enforced by Stellar smart accounts". It uses the Perch deck's engine, palette and seals and is a separate deck, so `../perch-deck/` is unchanged. Everything is in one file, with inline SVG, CSS and JS and system fonts, and the deck makes no network requests. The stellar.expert, deployment-record and GitHub links are plain links.

Controls:

- Navigation: arrows, Space or Shift+Space, PageUp/PageDown, and Home/End; swipe, or tap either side of the stage.
- `N` shows each slide's speaker notes, which carry its sources. `T` switches between light and dark; until then the deck follows the OS.
- `#n` opens slide n, and `?still=1` shows every slide's final drawing without motion. Reduced motion has the same effect.

Solid cyan in an engraved frame is what exists today. Dashed magenta with printer's marks is work the award would fund. Slide 3, marked "a cited study", draws another team's published experiment in plain ink and links the paper; slide 4 shows where Perch keeps the cap, and slide 7 the funded agent runtime, tagged with its tranches.

The deck is written as if release PR #168 has merged. Grants (`apply_grant`), `rule_eval::check_auth`, recovery and the verifiers are the merged baseline, and the award does not fund them. Mainnet still runs the earlier account design (`c135aca`). Deploying the account changes to mainnet is Tranche 3, deliverable 1.

On the Cedar slide, only two things are stated as fact: the size measurements and the fragment proof. Everything else on it is funded work.

## Sources

The source is the SCF Build submission prepared for this award (its form answers); its section names are given below.

Tranche 1 items 2 and 3 carry the titles and scope agreed on 2026-11-09. The deck states tranches, deliverables and completion dates only.

| Slides | Source |
| --- | --- |
| 1 | the SCF Build submission, Submission Title, Build Award Track, Products & Services. The dial and seal P are the Perch deck's title drawing (`../perch-deck/thumbnail.py`). |
| 2 | the SCF Build submission, Products & Services (items 1 and 2, and the funded work) |
| 3 | Breckenridge, Vilardell, Wang, Zhang and Juels, "I-HITL: Inalienable Human-in-the-Loop", [arXiv:2610.35835](https://arxiv.org/abs/2610.35835) (v3, 30 October 2026): p. 4 (Setup, Experiments, Key Findings), p. 5 (Results), Appendix D (p. 18). The paper names the ten models; the deck does not. |
| 4 | the SCF Build submission, Products & Services (items 1–3); `docs/perch-paper/source/statements.json` (T8, grant target containment); `crates/perch-smart-account/src/lib.rs` (the admin rule, `apply_grant`, `remove_grant`); `docs/nido-sim/README.md` (step 1, the passkey). This is not the paper's I-HITL protocol. |
| 5 | the SCF Build submission, Traction Evidence (`deployments/mainnet.json`, the Registry, Document compiler and Account factory on stellar.expert, Nido's factory), Products & Services, and SCF Build Tranche Deliverables ("Baseline"). Nido's counts are context reported under Nido's SCF #41 award and are not shown. |
| 6 | The Perch deck, slide 4, and its sources (`docs/perch-paper/source/statements.json` T4 and T5, `source/facts/example.json`, `source/facts/proofs.json`). The SCF Build submission, Tranche #1 item 1 and Tranche #2 item 1. |
| 7 | the SCF Build submission, Products & Services (the funded work), Tranche #1 items 1 and 3 (item 3 as agreed 2026-11-09) and Tranche #2 items 1 and 2; the vision paper, §08 "Agents, budgets and payments (proposed)" (`docs/perch-paper/source/papers/vision/html/sections/agents.html`). |
| 8 | the SCF Build submission, Tranche #1 items 2 and 3, as agreed 2026-11-09. The sizes are from `docs/cedar-walkthrough/` ("Why not upload the full Cedar evaluator?", measured 2026-11-06). The fragment proof is from the vision paper, §09 "Cedar as an authoring language" (`docs/perch-paper/source/papers/vision/html/sections/cedar.html`). |
| 9 | the SCF Build submission, Tranche #1 Deliverables and Completion Date. Items 2 and 3 are as agreed 2026-11-09. |
| 10 | the SCF Build submission, Tranche #2 Deliverables and Completion Date |
| 11 | the SCF Build submission, Tranche #3 Deliverables and Completion Date |
| 12 | the SCF Build submission, Tranche #1, #2 and #3 Deliverables and Completion Dates. Tranche 1 items 2 and 3 are as agreed 2026-11-09. |
| 13 | the SCF Build submission, Team Description and Project URL |

Nothing in this folder has been posted or submitted.
