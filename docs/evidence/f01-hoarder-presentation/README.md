# The Hoarder room's stand-in presentation (FA-Q23, HRD-P)

Screenshot evidence of room 1 drawn: the committed HOARD-01 frames (an unapproved candidate; the independent review is still pending) with the visible label **"STAND-IN UNTIL HOARD-02"**, and a labelled greybox door, entry gate, bank A, J-B post and lair mound (K10). **No sprite was drawn or changed**: the presentation is `src/hoarder_art.rs` (pure) and `src/bin/floor01_scene/hoarder_art.rs` (Bevy). Game feel is pending; the owner approves or rejects the stand-ins (K10).

The room pilot `--f01-room hoarder --room-pilot hoarder:carl_solo:0` plays the room (a defeating pilot, 9 of 9 seeds; seed 0 defeats her on tick 1231 and reaches the exit on tick 1381). `scripts/make_hoarder_presentation_evidence.sh` renders the real Bevy window under xvfb at six ticks, at 1x (640x360, `--scale 1`) and at 1080p (1920x1080, `--scale 3`), **twice**, and requires the two passes to be byte-identical PNGs.

| Still | Tick | What it shows |
| --- | --- | --- |
| `dormant_*` | 100 | the lair mound labelled MOUND, bank A, the post, the gate open (its faint footprint); she is under the mound |
| `reveal_*` | 150 | the reveal: she fades in on the idle frame (the stand-in of the emerge art), the gate closed (barred slab) |
| `tell_*` | 218 | the swipe's rear-up hold (HOARD-01 frame 8) with her label above her |
| `impact_*` | 241 | the swipe's impact frame (frame 10) |
| `recover_*` | 262 | the held recovery frame (frame 11): the free-damage window |
| `defeated_*` | 1300 | down frame 15 (the last, held; her label drops to just over the prone body), the exit door open (only its frame), the gate open |

[`capture_lines.txt`](capture_lines.txt) has one `HOARDER_STILL` line per still and size: the window's final `COMBAT_STATE` hash, the headless run's (they must be equal: the presentation changes no sim hash), whether the two passes were byte-identical, and the PNG's sha256. [`SHA256SUMS`](SHA256SUMS) is verified by `python3 scripts/check_docs.py`.

The state hash is also proven by `hoarder_art::tests::the_presenter_does_not_change_the_state_hash` (the pure presenter called on every tick of two room pilot runs; the hash equals the plain run's at every tick and the pinned library hash at the end) and by the render-loop replay of the nine room pilot runs (`../f01-playtest/render_replay_lines.txt`, 153 of 153 `match=yes`).

**Not shown:** the shove tell and lunge (the room pilot jumps the swipe and never draws a shove from her; their frame mapping and the 4 px wind-up are unit-tested), the jam, the J-A and J-B pile states (HRD-B and HRD-C), the stir cue before she wakes. **NOT RUN:** a GIF, a 2P run, a human run, the release build, real hardware.

Regenerate: `cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh && bash scripts/make_hoarder_presentation_evidence.sh`.

## Rights and provenance

Integration only: the HOARD-01 frames are the existing committed candidate (existing source HOARD-01: designed by Claude Opus, in-betweens and evidence by Claude Sonnet, independent review pending; fan-concept art under the owner-reported scope `ART-PUBLICATION-ALL-V1`, see `docs/04-production/rights-and-release.md`). **No new art, no sprite was drawn, resampled or altered.** The greybox pieces (the exit door, the entry gate, bank A, and, added beyond the plan's list, the J-B post and the lair mound) and the label font are code-drawn flat rectangles (K10), STAND-IN greybox, not new art; the post and the mound were accepted as such by the independent review (ruling R1) and are covered by the owner's T38 K10 approval. This PR's integration (the pure and Bevy presenters, this evidence): Claude Sonnet; independent review of the integration: Claude Opus (separate agent, PASS with fix-now items, fixed here). The independent top-tier review of HOARD-01 stays in the art lane. Nothing here is a claim of approved art, a Steam build, or a playable-game distribution right.
