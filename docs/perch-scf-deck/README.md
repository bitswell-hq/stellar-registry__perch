# Perch Agent Grants deck

`index.html` is a 17-slide deck for The Aha Company's SCF Build Award submission (Open Track): "Perch Agent Grants: subagent budgets enforced by Stellar smart accounts". It uses the Perch deck's engine, palette and seals and is a separate deck, so `../perch-deck/` is unchanged. Everything is in one file, with inline SVG, CSS and JS and system fonts, and the deck makes no network requests, except that slide 11's "Run it here" loads the sibling page `../nido-sim/index.html?embed=1` into its live frame when pressed, and nothing else. The stellar.expert, deployment-record and GitHub links are plain links.

Controls:

- Navigation: arrows, Space or Shift+Space, PageUp/PageDown, and Home/End; swipe, or tap either side of the stage.
- `N` shows the speaker script in a panel beside the slide (below it on a narrow or portrait screen), and the slide shrinks to the rest of the window at 16:9. The panel shows the lines to say aloud, a timer against 3:00 with Reset, the time on this slide against its target, and the next slide's title. The timer runs while the panel is shown. `S` shows the slide's sources: the background and citations that used to be in the notes. `T` switches between light and dark; until then the deck follows the OS.
- `R` records a video of the deck (below); `P` pauses and resumes a recording.
- `#n` opens slide n, and `?still=1` shows every slide's final drawing without motion, and turns the recorder off. Reduced motion also stops the motion; the recorder stays available.

The whole deck is the proposal, drawn in one solid style with no status coding; honesty is in the words. What is live is said as such: Perch contracts on mainnet and Nido creating Perch accounts (slides 2, 10, 11). What is merged in the repository is said as such too: grants, `apply_grant`, `rule_eval::check_auth`, recovery and the Lean proofs, with grants reaching mainnet in Tranche 3. Everything about the runtime, the Cedar compiler and engine, the examples and the tranches reads as what we will build. Slide 4, marked "a cited study", draws another team's published experiment and links the paper. Slide 5 compares where agent-payment policy is enforced and names no product. On slide 11 the Nido and npm figures and the mainnet deployment are marked "context": they are reported under Nido's SCF #41 award, not claimed for this ask. Theorem numbers appear only in the sources view; the slides, script and aria-labels say what is proven in plain words.

The arc introduces Perch (slide 2), then the problem (slides 3–4), then the contrast (slide 5). What we will build follows, a contained policy runtime (slide 6): one Cedar policy, evaluated off chain by a contained engine and compiled into the rules the account enforces on chain. Two examples follow, a mail assistant (slide 7) and a news and data reader (slide 8). Slide 9 shows the account document and the agent's grant document in Cedar-style pseudocode (the authoring form we will compile to the account's rules, not valid Cedar), and why the agent cannot raise its cap; "Show the document fields" maps each line to Perch's JSON fields. Slides 10–12 cover what exists today, including Perch in Nido with a live simulation (slide 11), and slides 13–16 the tranches. Only the account's decision function is called proved; the runtime will be tested, not proven.

The deck is written as if release PR #168 has merged. Grants (`apply_grant`), `rule_eval::check_auth`, recovery and the verifiers are the merged baseline, and the award does not fund them. Mainnet still runs the earlier account design (`c135aca`). Deploying the account changes to mainnet is Tranche 3, deliverable 1.

On the contained policy runtime slide, the fragment proof and the size measurements (in its sources view) are stated as fact; the rest of the Cedar work is what we will build.

## Speaker script

The script is 440 words over the 17 slides. At about 150 words a minute, the targets add up to 2:57, inside SCF's three-minute limit. Each slide's notes carry a target time (`data-sec`), and the script panel shows it beside a running timer. The script says only what the slides say. Its citations are in the sources view (`S`), with nothing removed.

## Recording a video

The Record button in the controls, or `R`, records the deck with your voice and saves a video file. Everything happens in the browser: nothing is uploaded and there is no server.

1. Use a desktop browser. Press `R` and allow the microphone. If it is blocked, the deck says so and records nothing; allow it in the site settings and press `R` again.
2. The deck prepares the slides, then counts down 3, 2, 1. `R` or `Escape` cancels the countdown.
3. Present as usual with the arrow keys. A presenter panel beside the slide shows the script for the current slide, the time against 3:00, the time on this slide against its target, and the next slide. The panel is not in the video.
4. `P` pauses and resumes. `R`, or "Stop and save", ends the recording and downloads `perch-agent-grants-<date>-<time>.mp4`, or `.webm`. The message names the format, size and length.

The video is 1920×1080 at 30 frames a second. The deck lays itself out at 1920×1080 in a hidden frame and paints each slide in its final state onto a canvas: the drawings as images, the text at the positions the browser laid out. The recording is that canvas plus the microphone, through `MediaRecorder`. Slides change with a short crossfade; the arrival animations are not recorded. Slide 11's live Nido simulation is a frame, which the canvas cannot draw: the video always shows that slide's drawn version, the traction facts and the simulation's own steps (create, grant, a 1 XLM guess, a 3.50 EURC coffee, a refused direct payment). That holds even while the presenter runs the live frame. Once you click inside the live frame, key presses go to the frame and the deck's keys (arrows, Space, `P`, `R`) do nothing until focus returns: click the slide outside the frame, or use the on-screen arrows, so while recording advance from slide 11 with the deck's arrow buttons.

The file is MP4 (H.264 and AAC) where the browser can write it, as in Safari and recent Chrome, and WebM (VP9 or VP8 with Opus) otherwise. YouTube and Vimeo accept both. The deck sets the duration that WebM files from `MediaRecorder` lack or leave at zero. The recording uses the theme in effect when it starts.

## Sources

The source is the SCF Build submission prepared for this award (its form answers); its section names are given below.

Tranche 1 items 2 and 3 carry the titles and scope agreed on 2026-11-09. The deck states tranches, deliverables and completion dates only.

| Slides | Source |
| --- | --- |
| 1 | the SCF Build submission, Submission Title, Build Award Track, Products & Services. The dial and seal P are the Perch deck's title drawing (`../perch-deck/thumbnail.py`). |
| 2 | the SCF Build submission, Products & Services (items 1–3), Traction Evidence, Tranche #3 item 1; `docs/perch-paper/source/statements.json` (P1). |
| 3 | the SCF Build submission, Products & Services (items 1 and 2, and the funded work) |
| 4 | Breckenridge, Vilardell, Wang, Zhang and Juels, "I-HITL: Inalienable Human-in-the-Loop", [arXiv:2610.35835](https://arxiv.org/abs/2610.35835) (v3, 30 October 2026): p. 4 (Setup, Experiments, Key Findings), p. 5 (Results), Appendix D (p. 18). The paper names the ten models; the deck does not. |
| 5 | arXiv:2610.35835, p. 4 (the emailed-code finding); the SCF Build submission, Products & Services (the funded work), Tranche #1 items 2 and 3 (as agreed 2026-11-09). The common setups are described, not named. |
| 6 | the SCF Build submission, Products & Services (items 1 and 3, and the funded work), Tranche #1 items 1–3 (items 2 and 3 as agreed 2026-11-09) and Tranche #2 items 1 and 2; `docs/cedar-walkthrough/` ("Why not upload the full Cedar evaluator?", measured 2026-11-06); the vision paper, §08 and §09 (`docs/perch-paper/source/papers/vision/html/sections/agents.html`, `cedar.html`). |
| 7 | What we will build: the SCF Build submission, Products & Services (the funded work; item 2, grants), Tranche #1 items 1 and 3; arXiv:2610.35835, p. 4; `docs/cedar-walkthrough/` (the Cedar dialect). The action and entity names are the request vocabulary we will give the runtime, and mail is modelled as web requests to two named hosts. The grant is 5 USDC per 518,400 ledgers, about 30 days. |
| 8 | What we will build: the SCF Build submission, Products & Services (the funded work), Tranche #1 items 1 and 3; `docs/cedar-walkthrough/`. No money moves, so there is no grant. |
| 9 | `crates/perch-ir/src/doc.rs` (`PolicyDoc`, `SignerDecl`, `Rule`: `scope` contract or self-admin, `principals`, `functions`, `args`/`pred`, `auth-root`, `cap {limit, period-ledgers}`, `not-after-ledger`, `RecoveryConfig`); `docs/schema.md`; `docs/account-interface.md` (Constructor, `apply_doc`, `apply_grant`, Removing grants); the grant `crates/integration-tests/tests/auth_root.rs` installs, with a shop in place of the game; `docs/perch-paper/source/statements.json` (T8, grant target containment). Cedar-style pseudocode, not valid Cedar; values are illustrative. |
| 10 | the SCF Build submission, Traction Evidence (`deployments/mainnet.json`, the Registry, Document compiler and Account factory on stellar.expert, Nido's factory), Products & Services, and SCF Build Tranche Deliverables ("Baseline"). Nido's counts are context reported under Nido's SCF #41 award and are not shown. |
| 11 | the SCF Build submission, Traction Evidence (mainnet, Nido's factory, npm 1,662 downloads 2026-10-10 to 2026-11-08, the 2026-11-05 testnet run) and its notes; the paper's section "A Nido wallet run used a grant end to end"; `docs/nido-sim/` (the live frame, `../nido-sim/index.html?embed=1`, loaded on press). |
| 12 | The Perch deck, slide 4, and its sources (`docs/perch-paper/source/statements.json` T4 and T5, `source/facts/example.json`, `source/facts/proofs.json`). The SCF Build submission, Tranche #1 item 1 and Tranche #2 item 1. |
| 13 | the SCF Build submission, Tranche #1 Deliverables and Completion Date. Items 2 and 3 are as agreed 2026-11-09. |
| 14 | the SCF Build submission, Tranche #2 Deliverables and Completion Date |
| 15 | the SCF Build submission, Tranche #3 Deliverables and Completion Date |
| 16 | the SCF Build submission, Tranche #1, #2 and #3 Deliverables and Completion Dates. Tranche 1 items 2 and 3 are as agreed 2026-11-09. |
| 17 | the SCF Build submission, Team Description and Project URL |

`REVIEW.md` reviews the sign dialog and the Nido simulation and gives a minimal-by-default spec for both.

Nothing in this folder has been posted or submitted.
