# Perch Agent Grants deck

`index.html` is a 10-slide deck for The Aha Company's SCF Build Award submission (Open Track): "Perch Agent Grants: subagent budgets enforced by Stellar smart accounts". It uses the Perch deck's engine, palette and seals and is a separate deck, so `../perch-deck/` is unchanged. Everything is in one file, with inline SVG, CSS and JS and system fonts, and the deck makes no network requests. The stellar.expert, deployment-record and GitHub links are plain links.

Controls:

- Navigation: arrows, Space or Shift+Space, PageUp/PageDown, and Home/End; swipe, or tap either side of the stage.
- `N` shows each slide's speaker notes, which carry its sources. `T` switches between light and dark; until then the deck follows the OS.
- `#n` opens slide n, and `?still=1` shows every slide's final drawing without motion. Reduced motion has the same effect.

Solid cyan in an engraved frame is what exists today. Dashed magenta with printer's marks is work the award would fund.

The deck is written as if release PR #168 has merged. Grants (`apply_grant`), `rule_eval::check_auth`, recovery and the verifiers are the merged baseline, and the award does not fund them. Mainnet still runs the earlier account design (`c135aca`). Deploying the account changes to mainnet is Tranche 3, deliverable 1.

On the Cedar slide, only two things are stated as fact: the size measurements and the fragment proof. Everything else on it is funded work.

## Sources

The source is the SCF Build submission prepared for this award (its form answers); its section names are given below.

Agreed 2026-11-09:

- Tranche 1 items 2 and 3 have new titles and scope. Their budgets stay at $15,000 and $9,000.
- The budget is revised to $123,000: 41 engineer-weeks at $3,000 a week. The x402 payer flow is now $6,000 and the mainnet release $9,000. Tranches are $42,000, $36,000 and $45,000.

| Slides | Source |
| --- | --- |
| 1 | the SCF Build submission, Submission Title, Build Award Track, Products & Services. The dial and seal P are the Perch deck's title drawing (`../perch-deck/thumbnail.py`). |
| 2 | the SCF Build submission, Products & Services (items 1 and 2, and the funded work) |
| 3 | the SCF Build submission, Traction Evidence (`deployments/mainnet.json`, the Registry, Document compiler and Account factory on stellar.expert, Nido's factory), Products & Services, and SCF Build Tranche Deliverables ("Baseline"). Nido's counts are context reported under Nido's SCF #41 award and are not shown. |
| 4 | The Perch deck, slide 4, and its sources (`docs/perch-paper/source/statements.json` T4 and T5, `source/facts/example.json`, `source/facts/proofs.json`). the SCF Build submission, Tranche #1 item 1 and Tranche #2 item 1. |
| 5 | the SCF Build submission, Tranche #1 items 2 and 3, as agreed 2026-11-09. The sizes are from `docs/cedar-walkthrough/` ("Why not upload the full Cedar evaluator?", measured 2026-11-06). The fragment proof is from the vision paper, §09 "Cedar as an authoring language" (`docs/perch-paper/source/papers/vision/html/sections/cedar.html`). |
| 6 | the SCF Build submission, Tranche #1 Deliverables and Completion Date. Items 2 and 3 are as agreed 2026-11-09. |
| 7 | the SCF Build submission, Tranche #2 Deliverables and Completion Date. Budgets are as revised 2026-11-09. |
| 8 | the SCF Build submission, Tranche #3 Deliverables and Completion Date. Budgets are as revised 2026-11-09. |
| 9 | the SCF Build submission, Budget table and SCF Build Tranche Deliverables ("Rate and total", "Payment schedule", "Excluded costs"), with the budget as revised 2026-11-09. Also the three Completion Dates. |
| 10 | the SCF Build submission, Team Description and Project URL |

Nothing in this folder has been posted or submitted.
