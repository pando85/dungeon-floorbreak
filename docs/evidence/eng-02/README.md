# ENG-02 evidence: the Goblin Engineer's get-up, ram run and finishes

Spec: [`../../02-art/eng-02-spec.md`](../../02-art/eng-02-spec.md) (design, gate rulings, executor notes in section 18). **Owner art review pending, game-feel review pending, independent review of the finished frames pending.** Nothing here approves art or feel.

## Still sheets and pose GIFs (composed from the committed sources, not Bevy captures)

Made by `python3 scripts/eng02_evidence.py` (deterministic). Pose GIFs play at game speed (60 Hz tick counts), 2x, on the Floor 1 ground tone.

| File | Shows |
| --- | --- |
| [`contact_sheet_x4.png`](contact_sheet_x4.png) | Every new frame at 4x, labelled: ENG-01 atlas 18 to 21, the six slammed cels, the six death cels |
| [`layer_split_x4.png`](layer_split_x4.png) | The slammed and death cels split by layer (`body`, `prop`, `pot`, `gore`) |
| [`crush_with_stone_x1.png`](crush_with_stone_x1.png), [`crush_with_stone_x4.png`](crush_with_stone_x4.png) | Crush frames 0 to 2 **with the ENV-02 stone drawn** (`loose_block_falling` with its underside on row 33 and 36, `loose_block_landed` for the remains), lit Floor 1, at 1x and 4x (E2-4, the gate's rule) |
| [`parts_sheet_x4.png`](parts_sheet_x4.png) | The 27 parts at 4x with the `frame0_origin` in each label, the cell pivot (yellow) and the anchors (magenta) |
| [`schema.png`](schema.png) | Frame layout, pivot (28, 55), the death anchors including `hip_back`, and the slam launch points on the slammed impact cels |
| [`getup.gif`](getup.gif), [`ram_run.gif`](ram_run.gif), [`slam_stomped.gif`](slam_stomped.gif), [`slam_wall.gif`](slam_wall.gif), [`slam_flung.gif`](slam_flung.gif), [`crush.gif`](crush.gif), [`decap.gif`](decap.gif) | The pose sequences (the launched pot and wrench are runtime parts: see the in-scene clips) |

## In-scene captures (real Bevy, `--scene f01_late` rooms 2 and 6)

Made by `bash scripts/make_eng02_evidence.sh` (debug build, xvfb, 640 x 360, 3x crops). Every clip was captured twice with `scripts/room_capture.sh`: the window's final `COMBAT_STATE` hash equals the headless run's, and the two generations are byte-identical frame for frame (`in-scene/capture_lines.txt`, 14 of 14 `match=yes generations_identical=yes`). The stills are `in-scene/<clip>_fNN.png` (frame index NN of the clip), the GIFs `in-scene/<clip>.gif` at game speed.

* **Natural:** `room2_getup` (room 2, `boiler:carl_solo:direct:0`: two dive stomps knock the Engineer down at ticks 354 and 369, the sit is on screen on ticks 406 to 415, the shove on 416 to 427), `room2_knockback_natural` (the jab kill at tick 792, the `engineer_knockback_down` finish).
* **Forced finishes (`--death-lab-engineer`, presentation only):** the Floor 1 room pilots only ever kill the Engineer with a jab. The flag makes the presenter play the named finish at that same defeat: room 2 (tick 792) `crushed`, `decapitated`, `exploded` (gore preset 4 and 2), `slammed_stomped`, `slammed_wall`, `slammed_flung`; room 6, wave 2 (`mastery:carl_solo:0`, tick 672) `crushed`, `decapitated`, `slammed_stomped`, `slammed_flung`. The pot and the wrench are parts: each clip shows the pot landing on the floor beside him. The crushed clips show the pose under no stone (the shore block is not in these rooms; the composited crush is above).
* **Ram run:** no Floor 1 pilot lets his ram run for more than a tick (a scan of the 90 aligned boiler and mastery pilots, 9 seeds each, found no frame 12, 20 or 21), so `arena_ram_run` is the CF02 arena fixture `eng_jump_over_ram` (ticks 34 to 56): the launch (11), then 20, 21, 20 and 12 (the cycle by ram age at 4 ticks a frame) under Carl's jump.

The heroes stand over the Engineer in several room 2 clips, so the Engineer is partly hidden in them; this is the pilot's choreography, not a framing choice.

## Pins that moved, with the reason

| Pin | Old | New | Why |
| --- | --- | --- | --- |
| `assets/manifests/eng_01.json`, `eng_01.grid.json`, `.ase`, `.pixelart.json`, atlas | 18 frames, 7 tags | 22 frames, tags `getup` (18, 19) and `attack_ram_run` (20, 21); `max_colors` 20 to 17; ENG-01 frames 0 to 17 byte-identical (sha256 pinned in `tests/test_art_eng01.py`/`test_art_eng02.py`) | ENG-02 appends frames 18 to 21 |
| `tests/test_art_eng01.py` | 18 frames, window sums without the new tags | 22 frames, `getup` 22 and `attack_ram_run` 8 ticks; the pot check skips frame 19; the wrench check expects it on 19 to 21 | the appended frames |
| `assets/source/pixel_hole_allowlist.json` | no ENG-02 entries | the three key pockets (design gate) and the inherited pockets of the death and parts sheets (with their GOB source frames) | V17 on the new configs |
| `engineer_ticks` / `ENGINEER_FRAMES` | 18 | 22 (frames 18 to 21 held by state) | the atlas |
| `assets/data/death/species_death_profiles_cf02.json` | `engineer_down` x6 | the five finish ids and the fallback `engineer_knockback_down` | ENG2-AD11 |
| `src/cf02_fixtures/engineer_tests.rs` | a test named `..._engineer_down` | `..._to_the_floor` | the E2-6 grep |
| Render replay lines and `SHA256SUMS` of `docs/evidence/f01-playtest` | | **not moved** | they are simulation-state hashes; the presenter reads the simulation and writes nothing back (the ci_floor01_combat replays and the 14 window-equals-headless captures above hold it) |

Not moved, as required: `MANIFEST_HASH`, `species_death_profiles.json`, the authoritative goldens and every goblin, CF02 and llama asset.

## Rights and provenance

Fan-concept art under `ART-PUBLICATION-ALL-V1`. Designed by Claude Opus; design gate (independent) Claude Opus; executed by Claude Sonnet; independent review of the finished frames pending. No image-model output, no tracing; the pixels are committed ENG-01, GOB-01 A, GOB-02 A and CF02 cels plus the two frozen keys.
