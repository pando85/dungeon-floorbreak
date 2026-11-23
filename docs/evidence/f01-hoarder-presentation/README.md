# The Hoarder room's presentation (FA-Q23, HRD-P; HOARD-02)

Screenshot evidence of room 1 drawn with the HOARD-02 production art: the Hoarder from the `hoard-02` atlas (29 frames) and the `floor01_hoarder_*` room kit (the sealed and opened lair mound, bank A, the J-B post and the stack hanging from it, the exit door, the entry gate and three dressing heaps). **No STAND-IN label and no greybox piece remains** for her or any wired prop. The presentation is `src/hoarder_art.rs` (pure) and `src/bin/floor01_scene/hoarder_art.rs` (Bevy). The art is an unapproved candidate (independent review pending) and game feel is pending.

The room pilot `--f01-room hoarder --room-pilot hoarder:carl_solo:0` plays the room (a defeating pilot, 9 of 9 seeds; seed 0 reveals her on ticks 112..201, swipes from tick 202, defeats her on tick 1231 and reaches the exit on tick 1381). `scripts/make_hoarder_presentation_evidence.sh` renders the real Bevy window under xvfb at eleven ticks, at 1x (640x360, `--scale 1`) and at 1080p (1920x1080, `--scale 3`), **twice**, and requires the two passes to be byte-identical PNGs.

| Still | Tick | What it shows |
| --- | --- | --- |
| `dormant_*` | 100 | the sealed mound (back layer behind everything, the front lip in front), bank A with its lit top, the post with the stack hanging from it, the dressing heaps, the gate open; she is under the mound |
| `reveal1_*`, `reveal2_*`, `reveal3_*` | 125, 155, 185 | the Reveal in thirds: emerge frames 26, 27, 28, drawn one ramp step darker (R7) with the lip in front of her and dust sliding off her; the gate closed |
| `reveal_last_*` | 201 | the last Reveal tick: still in shadow, lip still in front (she has no hurtbox yet) |
| `out_*`, `out_late_*` | 202, 206 | the first tick she can be hit: full value, the mound is the low `opened` spill, the 4-tick dust burst (gone by 206) |
| `tell_*` | 218 | the swipe's rear-up hold (frame 8) |
| `impact_*` | 241 | the swipe's impact frame (frame 10) |
| `recover_*` | 262 | the held recovery frame (frame 11): the free-damage window |
| `defeated_*` | 1300 | down frame 15 (held), the exit door open (jamb only), the gate open |

[`capture_lines.txt`](capture_lines.txt) has one `HOARDER_STILL` line per still and size: the window's final `COMBAT_STATE` hash, the headless run's (they must be equal: the presentation changes no sim hash), whether the two passes were byte-identical, and the PNG's sha256. [`SHA256SUMS`](SHA256SUMS) is verified by `python3 scripts/check_docs.py`.

The state hash is also proven by `hoarder_art::tests::the_presenter_does_not_change_the_state_hash` (the pure presenter called on every tick of two room pilot runs; the hash equals the plain run's at every tick and the pinned library hash at the end) and by the render-loop replay of the nine room pilot runs (`../f01-playtest/render_replay_lines.txt`).

**Not shown at runtime:** the shove tell, lunge, jam, pinned and get-up (the room pilot kills her with swipes and never draws a shove, so those modes have no window still; their frame mapping is unit-tested and `../hoard-02/` has offline composites, labelled "composite, not runtime"), and the J-A and J-B pile states (HRD-B and HRD-C land on a parallel branch; the frames are drawn and mapped). **NOT RUN:** a 2P run, a human run, the release build, real hardware.

Regenerate: `cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh && bash scripts/make_hoarder_presentation_evidence.sh`.

## Rights and provenance

HOARD-02 and the room kit: designed by Claude Opus (keys 17, 18, 19, 20, 22, 24, 26, the mound `sealed` hero key and the spec), executed by Claude Sonnet (in-betweens, the other props and states, the wiring, this evidence); independent review of the art by a separate Claude Opus agent is pending. Fan-concept art under the owner-reported scope `ART-PUBLICATION-ALL-V1`, see `docs/04-production/rights-and-release.md`. Nothing here is a claim of approved art, a Steam build, or a playable-game distribution right.
