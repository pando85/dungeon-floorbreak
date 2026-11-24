# The Hoarder room's presentation (FA-Q23, HRD-P; HOARD-02)

Screenshot evidence of room 1 drawn with the HOARD-02 production art: the Hoarder from the `hoard-02` atlas (29 frames) and the `floor01_hoarder_*` room kit (the sealed and opened lair mound, bank A, the J-B post and the stack hanging from it, the exit door, the entry gate and three dressing heaps). **No STAND-IN label and no greybox piece remains** for her or any wired prop. The presentation is `src/hoarder_art.rs` (pure) and `src/bin/floor01_scene/hoarder_art.rs` (Bevy). The art passed its independent art-director review (Claude Opus, 2026-11-24); it is not owner-approved, and game feel is pending.

The room pilot `--f01-room hoarder --room-pilot hoarder:carl_solo:0` plays the room (a defeating pilot, 9 of 9 seeds; seed 0 reveals her on ticks 116..205, swipes from tick 206, defeats her on tick 1235 and reaches the exit on tick 1385; HRD-B and HRD-C (#260) moved the timeline 4 ticks later). The shove and the J-B piece come from the existing EVID-ROOM plan `--room-pilot direct` (`src/room_pilots.rs` `fight_or_walk`; no sim change, no new flag), which in this room baits her shove into the J-B post: ShoveTell 404..443, the lunge from 444, jammed on the post and the stack released on tick 450, the 24-tick warning 450..472, falling 473..480, landed and she is pinned from 481, GetUp from 631 (the headless run's `COMBAT_EVENT` lines). `scripts/make_hoarder_presentation_evidence.sh` renders the real Bevy window under xvfb at 21 ticks, at 1x (640x360, `--scale 1`) and at 1080p (1920x1080, `--scale 3`), **twice**, and requires the two passes to be byte-identical PNGs. The stills were re-captured after the HOARD-02 polish (spec section 11, E6: the junk rubble, the snapped strut, the warning's shudder and sag, heap b's rim light): all 42 are byte-identical across the two passes and every window hash equals the headless hash.

| Still | Tick | What it shows |
| --- | --- | --- |
| `dormant_*` | 100 | the sealed mound (back layer behind everything, the front lip in front), bank A with its lit top, the post with the stack hanging from it, the dressing heaps, the gate open; she is under the mound |
| `reveal1_*`, `reveal2_*`, `reveal3_*` | 129, 159, 189 | the Reveal in thirds: emerge frames 26, 27, 28, drawn one ramp step darker (R7) with the lip in front of her and dust sliding off her; the gate closed |
| `reveal_last_*` | 205 | the last Reveal tick: still in shadow, lip still in front (she has no hurtbox yet) |
| `out_*`, `out_late_*` | 206, 210 | the first tick she can be hit: full value, the mound is the low `opened` spill, the 4-tick dust burst (gone by 210) |
| `tell_*` | 222 | the swipe's rear-up hold (frame 8) |
| `impact_*` | 245 | the swipe's impact frame (frame 10) |
| `recover_*` | 266 | the held recovery frame (frame 11): the free-damage window |
| `defeated_*` | 1300 | down frame 15 (held), the exit door open (jamb only), the gate open |
| `jb_full_*` | 398 (`direct`) | the whole J-B assembly in frame: the post (`intact`) with the stack hanging from its strut, clear floor under it, heap c behind |
| `shove_tell_*`, `shove_hold_*` | 410, 430 (`direct`) | the shove's wind-up (frame 16) and the low wide brace (frame 17) |
| `lunge_*` | 446 (`direct`) | the lunge's low heave (frame 18) toward the post |
| `jam_*` | 450 (`direct`) | the release tick: she jams on the post (frame 20), the post `broken`, the stack `released`, the footprint's floor shadow |
| `warning_*` | 470 (`direct`) | the warning at its full 3 px sag (and the 1 px shudder), the jam hold (frame 21) |
| `falling_*`, `fall_last_*` | 476, 480 (`direct`) | mid-fall, and the last falling tick: the snapped strut stays over the floor line |
| `pinned_*` | 520 (`direct`) | pinned (frames 22/23) under the junk rubble, drawn in front of her |
| `get_up_*` | 640 (`direct`) | the get-up (frames 24/25) behind the rubble |

[`capture_lines.txt`](capture_lines.txt) has one `HOARDER_STILL` line per still and size: the window's final `COMBAT_STATE` hash, the headless run's (they must be equal: the presentation changes no sim hash), whether the two passes were byte-identical, and the PNG's sha256. [`SHA256SUMS`](SHA256SUMS) is verified by `python3 scripts/check_docs.py`.

The state hash is also proven by `hoarder_art::tests::the_presenter_does_not_change_the_state_hash` (the pure presenter called on every tick of two room pilot runs; the hash equals the plain run's at every tick and the pinned library hash at the end) and by the render-loop replay of the nine room pilot runs (`../f01-playtest/render_replay_lines.txt`).

**Not shown at runtime:** bank A's states after `stable` (no existing plan disturbs it; the presenter draws them from the HRD-B machine, `the_piles_are_drawn_from_their_machines` tests every state, and `../hoard-02/` has offline composites, labelled "composite, not runtime"), and the post's `cracked` state (her shove releases the stack outright; unit-tested). **NOT RUN:** a 2P run, a human run, the release build, real hardware.

Regenerate: `cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh && bash scripts/make_hoarder_presentation_evidence.sh`.

## Rights and provenance

HOARD-02 and the room kit: designed by Claude Opus (keys 17, 18, 19, 20, 22, 24, 26, the mound `sealed` hero key and the spec), executed by Claude Sonnet (in-betweens, the other props and states, the wiring, this evidence); independent art-director review by a separate Claude Opus agent (passed, 2026-11-24). The HOARD-02 polish (spec section 11, E6) and these re-captured stills: designed and executed by Claude Opus; independent art-director review by a separate Claude Opus agent (passed, 2026-11-24). Fan-concept art under the owner-reported scope `ART-PUBLICATION-ALL-V1`, see `docs/04-production/rights-and-release.md`. Nothing here is a claim of approved art, a Steam build, or a playable-game distribution right.
