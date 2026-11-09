# Perch Agent Grants deck

`index.html` is a 14-slide deck for The Aha Company's SCF Build Award submission (Open Track): "Perch Agent Grants: subagent budgets enforced by Stellar smart accounts". It uses the Perch deck's engine, palette and seals and is a separate deck, so `../perch-deck/` is unchanged. Everything is in one file, with inline SVG, CSS and JS and system fonts, and the deck makes no network requests. The stellar.expert, deployment-record and GitHub links are plain links.

Controls:

- Navigation: arrows, Space or Shift+Space, PageUp/PageDown, and Home/End; swipe, or tap either side of the stage.
- `N` shows the speaker script for the slide: a few lines to say aloud, with a target time. `S` shows the slide's sources: the background and citations that used to be in the notes. `T` switches between light and dark; until then the deck follows the OS.
- `R` records a video of the deck (below); `P` pauses and resumes a recording.
- `#n` opens slide n, and `?still=1` shows every slide's final drawing without motion. Reduced motion has the same effect.

Solid cyan in an engraved frame is what exists today. Dashed magenta with printer's marks is work the award would fund. Slide 4, marked "a cited study", draws another team's published experiment in plain ink and links the paper. Slide 5 compares where agent-payment policy is enforced; it names no product and has no status mark.

The arc introduces Perch (slide 2), then the problem (slides 3–4), then the contrast (slide 5). The funded design follows, a contained policy runtime (slide 6): one Cedar policy, evaluated off chain by a contained engine and compiled into the rules the account enforces on chain. Slides 7–9 cover what exists today, and slides 10–13 the funded work. Only the account's decision function is called proved; the runtime is tested, not proven.

The deck is written as if release PR #168 has merged. Grants (`apply_grant`), `rule_eval::check_auth`, recovery and the verifiers are the merged baseline, and the award does not fund them. Mainnet still runs the earlier account design (`c135aca`). Deploying the account changes to mainnet is Tranche 3, deliverable 1.

On the contained policy runtime slide, the fragment proof and the size measurements (in its sources view) are stated as fact; everything else Cedar-related is funded work.

## Speaker script

The script is 430 words over the 14 slides. At about 150 words a minute, it runs 2:54, inside SCF's three-minute limit. Each slide's notes carry a target time (`data-sec`); the script panel shows it with the word count. The script says only what the slides say. Its citations are in the sources view (`S`), with nothing removed.

## Recording a video

The Record button in the controls, or `R`, records the deck with your voice and saves a video file. Everything happens in the browser: nothing is uploaded and there is no server.

1. Use a desktop browser. Press `R` and allow the microphone. If it is blocked, the deck says so and records nothing; allow it in the site settings and press `R` again.
2. The deck prepares the slides, then counts down 3, 2, 1. `R` or `Escape` cancels the countdown.
3. Present as usual with the arrow keys. A presenter panel, top right, shows the script for the current slide, the time against 3:00, the time on this slide against its target, and the next slide. The panel is not in the video.
4. `P` pauses and resumes. `R`, or "Stop and save", ends the recording and downloads `perch-agent-grants-<date>-<time>.mp4`, or `.webm`. The message names the format, size and length.

The video is 1920×1080 at 30 frames a second. The deck lays itself out at 1920×1080 in a hidden frame and paints each slide in its final state onto a canvas: the drawings as images, the text at the positions the browser laid out. The recording is that canvas plus the microphone, through `MediaRecorder`. Slides change with a short crossfade; the arrival animations are not recorded.

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
| 7 | the SCF Build submission, Products & Services (items 1–3); `docs/perch-paper/source/statements.json` (T8, grant target containment); `crates/perch-smart-account/src/lib.rs` (the admin rule, `apply_grant`, `remove_grant`); `docs/nido-sim/README.md` (step 1, the passkey). This is not the paper's I-HITL protocol. |
| 8 | the SCF Build submission, Traction Evidence (`deployments/mainnet.json`, the Registry, Document compiler and Account factory on stellar.expert, Nido's factory), Products & Services, and SCF Build Tranche Deliverables ("Baseline"). Nido's counts are context reported under Nido's SCF #41 award and are not shown. |
| 9 | The Perch deck, slide 4, and its sources (`docs/perch-paper/source/statements.json` T4 and T5, `source/facts/example.json`, `source/facts/proofs.json`). The SCF Build submission, Tranche #1 item 1 and Tranche #2 item 1. |
| 10 | the SCF Build submission, Tranche #1 Deliverables and Completion Date. Items 2 and 3 are as agreed 2026-11-09. |
| 11 | the SCF Build submission, Tranche #2 Deliverables and Completion Date |
| 12 | the SCF Build submission, Tranche #3 Deliverables and Completion Date |
| 13 | the SCF Build submission, Tranche #1, #2 and #3 Deliverables and Completion Dates. Tranche 1 items 2 and 3 are as agreed 2026-11-09. |
| 14 | the SCF Build submission, Team Description and Project URL |

Nothing in this folder has been posted or submitted.
