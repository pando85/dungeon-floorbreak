# AREA-GYM-01: the gym kit of the boss arena (room 5)

Evidence for [the AREA-GYM-01 spec](../../02-art/area-gym-01-spec.md): the boss arena (`--scene boss_f01`, room 5 of the playtest) drawn with the gym dressing. It has mirror bays at both crash walls, the gate lintel cap, the ceiling girder, tube fixtures, the reception desk and turnstile, the weights and machine backdrop, rubber mats, and the exit door state pair. The art is not owner-approved, and the independent review is pending.

## Real Bevy captures (runtime)

`scripts/make_gym01_evidence.sh` plays room 5 with the PT-GATE-A room pilot `juicer` (`--scene f01_late --f01-room juicer --room-pilot juicer`). For each still, `scripts/room_capture.sh` renders the real window under xvfb at 1x (640 x 360, `--scale 1`) and at 1080p (1920 x 1080, `--scale 3`). Every still is captured **twice**, and the two passes must be byte-identical PNGs. Every window's final `COMBAT_STATE` hash must equal the headless run's: the dressing and the door presenter are presentation only. [`capture_lines.txt`](capture_lines.txt) holds one `GYM_STILL` line per still and size.

| Still | Tick | What it shows |
| --- | --- | --- |
| `scene_intact_*` | 60 | the room before the engage: both mirror bays intact, the gate open under its lintel cap, the exit door closed |
| `scene_left_crack_*` | 300 | the `mirror_crack` held on the left mirror after the tick-139 left-wall crash (the stun is over, the dust has gone) |
| `scene_phase2_*` | 1000 | a phase-2 exchange in the middle of the room: actors over the mats and the backdrop |
| `scene_exit_open_*` | 3230 | after the defeat (hp 0 on tick 3175): the right wall released, the exit door open on a warm lamp-lit brick passage with a floor spill (review fix 2, in the #274 `dfx_exit` language) |

## Offline sheets (deterministic, `scripts/make_gym01_sheets.py`)

| File | Content |
| --- | --- |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | every kit piece at native size; the 4x sheet is labelled |
| `hero_key_mirror_bay_x4.png` | the frozen hero key |
| `exit_states_x4.png` | the exit door pair: closed, open |
| `crop_left_crack_x4.png` | 4x crop of the runtime left crack (from `scene_left_crack_640x360.png`) |
| `composite_right_crack_x4.png` | **composite, not runtime**: no pilot crashes into the right wall, so the three held `mirror_crack` states are pasted at the right-wall pivot of the real intact capture |
| `layout_diagram_x2.png` | the schema over the real intact capture: the walking line, floor back line and ceiling collider, every dressing rectangle (the girder spans omitted), both crack zones and the left rack footprint |

## Checks

- The state hash: `gym_art::tests::the_dressing_and_the_presenter_do_not_change_the_state_hash` runs `boss01_full_clear_solo` and `boss01_party_defeat_retry` on the dressed and the plain arena. The hashes are equal on every tick, with the presenter reading the boss each tick, and the door opens on the release tick. `gym_kit::tests` cover solids, spawn, exit, views, walking line, crack zones, floor and ceiling lines, and the light budget. Every boss golden is unchanged.
- The export: two passes (the `.ase` rebuilt from the grids, then the CLI export) produce byte-identical atlases, metadata and `.ase` files.
- [`SHA256SUMS`](SHA256SUMS) is verified by `python3 scripts/check_docs.py`.

**Review round 1 fixes:** these stills were re-captured after the two design fixes (the mirror glass without the block-noise skyline, the warm lit open exit) and the fix-forward items (grey mat tape, a lighter mat base, the stronger magenta accent).

**Not refreshed here (follow-up):** other evidence folders still show room 5 before this dressing. Their hash lines are unaffected because they hash simulation state. The folders are `../boss-f01/scene/`, the room 5 gate stills in `../f01-danger-fx/`, and the Juicer clips and stills in `../f01-playtest/` and `../carl-prog-01/`.

**NOT RUN:** a 2P run, a human run, the release build, real hardware, the 640 x 400 view.

Regenerate: `cargo build --locked --bin floor01_scene && bash scripts/export_assets.sh && bash scripts/make_gym01_evidence.sh`.

## Rights and provenance

AREA-GYM-01: designed (spec, mirror-bay hero key) and executed (every other piece, the contract, the wiring, this evidence) by Claude Opus. The silhouette gate on the hero key was run by fresh Claude Opus agents: it failed in round 1 and passed in round 2. An independent art-director review by a separate Claude Opus agent is pending. All pixels are rule-drawn by `scripts/author_gym01.py` on the ENV-01 palette; nothing is traced or resampled from the concept images, and there is no text, logo or likeness. This is fan-concept art under the owner-reported scope `ART-PUBLICATION-ALL-V1`; see `docs/04-production/rights-and-release.md`. Nothing here claims approved art, a Steam build or a right to distribute a playable game.
