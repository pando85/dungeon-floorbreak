# THE CRAWL — Floor 1 corridor scene (COMP-01)

**Status:** functional integration for human review; not accepted. The scene composes the owner-approved ENV-01 kit (`env-kit-v2` contract, `env-kit-layout-v2` sample layout) with the approved Carl 52 px and Donut 40 px atlases. **COMP-02 adds the A1 lighting pass** ([ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md)): cold ambient per layer group, warm torch pools, floor pools, contact shadows, per-layer haze, additive glow and flame-only bloom, computed at output resolution on top of the integer-scaled pixel layers. It is presentation only and `--no-lighting` restores the COMP-01 look. No acceptance of the look or of performance is claimed: see [Lighting](#lighting-comp-02) for measurements and known divergences. **ANIM-INT-01** replaced the idle-only actor atlases with the owner-approved Carl (48 x 56) and Donut (72 x 44) locomotion atlases and a deterministic animation state machine (idle, run, jump, land); see [Animation](#animation-anim-int-01). Animation is presentation only: physics, movement, collision, camera and lighting are unchanged and the `FLOOR1_SCENE_STATE` hashes are identical to before.

It is a separate binary from the combat greybox and the Character Playground; both keep working. There are no enemies, bombs, health or new mechanics here, and `attack` / `ability` inputs are ignored.

## Run

```sh
bash scripts/export_assets.sh                    # atlases into the gitignored generated/art/
python3 scripts/check_floor01_scene_contract.py   # actor pivots + kit atlases vs contract
cargo run --locked --bin floor01_scene            # 640x360 virtual, 1280x720 window
cargo run --locked --bin floor01_scene -- --scale 3   # primary target: 1920x1080 window (3x)
cargo run --locked --bin floor01_scene -- --deck  # 640x400 virtual, 1280x800 window
```

Run from the repository root. A missing atlas is a setup failure: the binary prints `FLOOR1_ASSET_FAILURE: ...` and exits non-zero (also if textures are not loaded within 10 s). It never substitutes placeholders. Plain `cargo run` still starts the greybox (`default-run`).

| Flag | Effect |
| --- | --- |
| `--deck` | 640x400 virtual view, 1280x800 window (Steam Deck-like 16:10) |
| `--scale <1..8>` | window = virtual size x scale (default 2; **3 = 1920x1080, the primary target**) |
| `--no-lighting` | draw the COMP-01 look (pixel layers only, no pass); for regression and comparison |
| `--reference-frame` | review capture: camera pinned to layout view `abc_640x360` (`abc_640x400` with `--deck`), Carl and Donut standing at the ENV-01 preview positions (`preview.actor_check`), first idle frames, presentation tick 0, simulation untouched. Matches `docs/evidence/floor01-catacomb-kit/lit_abc_*.png` |
| `--exit-after-ticks <N>` | needs `--scenario`, live run: step the scenario for N fixed ticks in the real render loop, print `FLOOR1_SCENE_STATE` and exit (CI compares the lit and `--no-lighting` runs) |
| `--perf-frames <N>` | measure N frame intervals after 120 warm-up frames with vsync off, print `FLOOR1_PERF ...` and exit |
| `--scenario <name>` | scripted input: `idle`, `walk_right` (holds right, jumps at steps, reaches the exit in 613 ticks), `turn_around` (right for 150 ticks, then left), `run_jump` (stand 30 ticks, run right over the two low steps, a running jump at tick 170, stop at 260, a standing jump at 290; 382 ticks cover it all) |
| `--capture-tick <N>` | needs `--scenario`: pre-simulate N fixed ticks headlessly, then render that frozen state |
| `--capture-sequence <first:step:count>` `--capture-dir <dir>` | needs `--scenario`: pre-simulate `first` ticks, then in the real render loop save `count` window screenshots (`frame_NNNN_tick_TTTTT.png`) `step` ticks apart into `dir`, print a `FLOOR1_CAPTURE` line plus the scene and animation state lines for each, and exit. Used for the ANIM-INT-01 evidence |
| `--anim-trace` | with `--headless-floor01-smoke` and `--scenario`: print one `FLOOR1_ANIM_TRACE` row (tick, hero position and velocity, grounded, both actors' animation state) per simulated tick |
| `--freeze` | do not advance the simulation (start state, or the captured state) |
| `--debug-collision` | overlay merged solids (red), walking lines (cyan) and Carl's AABB (green) |
| `--headless-floor01-smoke` | no window or GPU: parse, derive solids, simulate, print state lines |

Controls: **A/D** or arrows or left stick/D-pad move, **Space** or South jumps, **R** or North restarts, **Esc** or Start quits. Reaching the B2 arch prints `FLOOR1_EXIT_REACHED` and stops the simulation (restart with R); torches keep animating on a separate presentation tick and both actors settle into idle. The window is not resizable (ADR-0007 integer scaling: window = virtual size x scale). Carl and Donut play the approved idle, run, jump and land animations ([Animation](#animation-anim-int-01)).

## Architecture

Pure library modules (headless, unit-tested) and one Bevy adapter:

| Module | Role |
| --- | --- |
| `env_kit` | serde structs for contract v2 and layout v2 (unknown fields, including all lighting data, are ignored); derived draw lists, the z table, parallax strips, markers and collision. Rejects v1 files. |
| `corridor` | `CorridorWorld` (Carl, Donut, solids, tick, state hash), `SceneSim` (world + camera + input), scripted scenarios. |
| `scroll_camera` | integer, deterministic camera rig |
| `actor_sprite` | pivot, flip and foot maths; loads each actor's frame size, pivot, tags and durations from its manifest and the exported atlas JSON (`load_actor`) |
| `anim` | the animation state machine, clip data (`ClipSet`) and run pacing rules: pure, integer clock, no Bevy types |
| `scene_anim` | derives Carl's and Donut's animation inputs from the finished simulation state and steps their animators (read-only on the world) |
| `prototype::step_hero` | the greybox's hero physics, extracted so both scenes share one implementation; `World::move_hero` delegates (two golden bit-trace tests of the greybox hero were added first and pass unchanged after the extraction; they are a coarse regression net, and direct `step_hero` tests pin coyote time, the jump buffer, wall push-out, ceiling bonk and landing individually) |
| `lighting` | pure lighting data: profile parsing and validation, slot plan, emitter derivation from the contract, per-frame `gather` (lights in view pixels, flicker from the tick), contact shadows, reference falloff curves. No Bevy types |
| `light_pass` | Bevy adapter for the lighting pass: GPU uniform packing, `LightingMaterial` (`Material2d`), camera and full-screen-quad rig, `light_pass.wgsl` |
| `src/bin/floor01_scene.rs` | input adapter, sprite spawning, per-frame sync, light update, watchdog, perf probe, headless smoke |

Rendering reads simulation state and never feeds it. Lighting, parallax, decals and every piece are never read by collision.

`env_kit` also parses the lighting data (`pieces[].lights`, layer `light_group`, the layout `lighting` profile) into `EnvKit::lighting`. It is derived last and only reads the finished kit (the walking line, once, to place floor pools).

### Coordinates and draw list

Data is top-left, y down; Bevy is y up. The y flip happens once, in `env_kit` (`world_y = layout_height - y`). Every tile and piece is one sprite using an atlas rect (`Sprite::rect`) with `Anchor::TOP_LEFT` at a whole-pixel position, and the camera sits on whole pixels, so at an integer window scale there are no sub-pixel seams. Entity counts for the sample layout (logged as `FLOOR1_ENTITY_COUNTS`): 2,864 tile sprites, 69 piece sprites, 6 parallax copies, 2 actors = **2,941 sprites** (the 8 solids are not sprites; the optional debug overlay adds about a dozen). Per-layer baked images were not needed.

Torch frames come from the contract's animation durations (120/140/100/130 ms) and the fixed tick, not wall-clock time; `frame_offset` starts a torch on that frame.

### Z table (from the contract)

| z | Layer | | z | Layer |
| --- | --- | --- | --- | --- |
| 0 | `parallax_far` | | 5 | `structure` |
| 1 | `parallax_mid` | | 6 | `prop` |
| 2 | `backwall` | | 7 / 7.1 | actors (Donut 7, Carl 7.1) |
| 3 | `backwall_decal` | | 8 | `foreground` |
| 4 | `collision` | | 10+ | debug overlay only |

Pieces add `0.001 x placement index` so placement order is the tie-break inside a layer; the bias never reaches the next layer.

### Parallax

A strip pixel at strip-local `u` is drawn at `x_origin + k*W + u - camera_x*factor`, snapped to whole pixels (`env_kit::parallax_start`, f64 maths, ties away from zero). Far strip factor 0.35, mid 0.6, both repeating forever (a strip needs at most `ceil(640/W)+1` copies; 3 for the 440 px far strip, 3 for the 344 px mid strip). The far strip shows through the arch windows because the backwall tiles are empty there.

## Collision and the walking line

- Collision comes only from `collidable` tiles of the `collision` layer. For the tile at (col, row) with `collider [cx, cy, cw, ch]` the solid is `[col*16+cx, row*16+cy, cw, ch]`. A `deck_front` tile is `[0, 4, 16, 12]`: **the collision surface is the walking line, 4 px behind the lip**; `deck_back` (the rest of the 24 px floor surface) is visual only.
- Cells are merged per row, then per identical x span, into **8 rectangles** for the whole sample layout. A test proves the merge neither adds nor loses area and that mutating or deleting the backwall, pieces and parallax changes no solid.
- Carl's AABB is the greybox's 24 x 52. **Foot sink is 0 px** (`FOOT_SINK_PX = 0`): the lowest opaque row of the actor is the row directly above the walking line. The pivot row of both atlases is the lowest opaque row (`check_floor01_scene_contract.py` proves it for all four idle frames).
- The spawn marker (`y = 307`) is the foot row, so Carl spawns with feet at 308 (the walking line). Platforms are walking lines at 292 (one tile) and 276 (two tiles).
- The hero may not leave the layout: x is clamped to `[12, width - 12]`; the left end wall also blocks.

## Camera

`scroll_camera` runs in the fixed tick and is integer-only. It follows `round(hero.x) + 24 * facing` (the pixel Carl is drawn at, so hero and camera move in lockstep and Carl does not shimmer), moves only when that point leaves a **+-40 px deadzone**, moves at most **4 px per tick**, and clamps to the layout. Vertical position is fixed by the layout views (`abc_640x360` y = 20, `abc_640x400` y = 0), so the 640x400 view shows 20 px more headroom and 20 px more floor depth than 640x360; the walking line is therefore 20 px lower on the 400 view. The simulation is identical on both views (tested); only the framing differs. Steady walking settles Carl 16 px right of centre when moving right and 16 px left when moving left.

## Donut (passive visual companion)

Donut trails Carl by **32 px of horizontal path length**, replaying the trail of Carl's foot positions (x and foot height), including his jump arcs, so she lands on the same walking lines. She has no collision, combat, physics or influence on Carl (tested: Carl's trajectory equals the bare greybox hero's). The stored trail is bounded: jumping in place adds no samples (one per path position) and a hard cap of 512 samples backs that up (tested). When Carl turns around she keeps walking to the turn point before reversing, passing through him. She starts 32 px behind the spawn. She is not a playable caster.

## Determinism

The fixed 60 Hz tick drives everything simulated: hero, Donut and camera; torch flicker and the actor animations are derived from the tick count too. State hashing is FNV-1a over f32 bit patterns. `FLOOR1_SCENE_STATE tick=... hero=... cam=... hash=...` is printed by the headless smoke and by the render process; CI requires the two lines to be identical for the frozen captures (`--scenario walk_right --capture-tick 250`). That proves **cross-process determinism of the simulation state** (the same scripted input gives the same state in two separate runs). It does **not** prove that the rendered pixels show that state; pixel correctness rests on the screenshot size/colour checks, the author's review and the human playtest. Rendered pixels are not compared bit-for-bit across GPUs. The animation state is checked the same way: `FLOOR1_ANIM_STATE tick=... carl=<tag>:<atlas frame> donut=<tag>:<atlas frame>` must be identical between the headless run and the render for every frozen and live capture, and the committed per-tick trace of the `run_jump` scenario is reproduced exactly by CI (see [Animation](#animation-anim-int-01)).

## Animation (ANIM-INT-01)

**Status:** functional integration of the owner-approved locomotion art ([Carl](../02-art/carl-locomotion-anim-01.md), [Donut](../02-art/donut-locomotion-anim-01.md)) for human review. Nobody has judged the game feel; see the known gaps below. The Character Playground is unchanged and still uses the old idle atlases (switching it is a follow-up).

### Data, not constants

| Actor | Atlas | Canvas | Pivot | Source of truth |
| --- | --- | --- | --- | --- |
| Carl | `carl_book1_h52_locomotion.png` | 48 x 56, 16 frames | (25, 55) | `assets/manifests/carl_book1_h52_locomotion.json` + exported `generated/art/carl_book1_h52_locomotion.json` |
| Donut | `donut_h40_locomotion.png` | 72 x 44, 16 frames | (46, 43) | `assets/manifests/donut_h40_locomotion.json` + exported `generated/art/donut_h40_locomotion.json` |

`actor_sprite::load_actor` reads the frame size and pivot from the manifest, the six tags (`idle`, `run`, `jump_start`, `jump_rise`, `jump_fall`, `land`) with their frame ranges and per-frame durations from the exported atlas JSON, and the playback mode of each tag (`loop`, `once`, `hold`) from the manifest's `locomotion.playback` block. Donut's manifest gained that block in this work (Carl's already had it), plus `airborne_frames: [8, 9]` (her suspension frames; the contract checker requires them to be off the pivot row). The atlas PNG name comes from the manifest's `output_atlas`. The headless smoke reads the same tags and durations from the committed pixel-grid sources instead (`FrameSource::GridSource`), so it needs no export; a test compares both sources when the export exists. The idle frames of both atlases are pixel-identical copies of the approved idle, so the `--reference-frame` capture is pixel-identical to the pre-ANIM-INT-01 one (measured: 0 differing pixels) and still passes the Pillow comparison gates.

### State machine

```text
 idle  <-- moving / stopped -->  run            instant; run restarts on its first frame

 grounded -> airborne, vy > 0    jump_start (once; starts on its toe-off frame 11 on the impulse tick)
                                   --> jump_rise (hold while vy > 0)
                                   <-> jump_fall (hold while vy <= 0; the switch is the apex)
 grounded -> airborne, vy <= 0   jump_fall (walked off a ledge: no jump_start)
 jump while airborne (coyote)    jump_fall -> jump_rise, no jump_start (intended: already up)
 airborne -> grounded            land (once) --> idle, or run when moving
                                   run cuts land short on the tick the impact frame ends
 jump while grounded             interrupts land (takeoff always wins)
 run, speed measured 0           while the actor still pushes: held on its frame for 5 ticks, idle on the 6th
                                   (stall hold); never held on a frame with no paw on the floor; a real stop idles at once
```

Inputs per fixed tick, derived from the finished simulation state (`scene_anim`): `grounded`, `vx`, `vy`. A grounded actor is "moving" above 0.1 px/s (the hero's facing threshold). `vx` is the **measured** speed (displacement per tick times 60), not the commanded one. Rules:

- **No jump delay.** The jump impulse happens on the same tick as before. `jump_start` starts on that tick **on its toe-off frame (atlas frame 11, 70 ms)** on top of the takeoff, then `jump_rise` or `jump_fall` follows by the sign of `vy` (`JUMP_START_TAKEOFF_FRAME = 1`). The anticipation crouch (frame 10, 50 ms) is never shown on a takeoff: the impulse leaves no grounded tick to show it on, and in the air it popped the pose (7 to 21 px up for 3 ticks, then the head jumped 11 px). It stays in the atlas for a future grounded anticipation. It is never cut short by an early apex (a ceiling bonk): it finishes, then falls.
- **Coyote jump.** A jump from the air right after a walk-off goes `jump_fall` to `jump_rise` with no `jump_start` (intended: the actor is already off the ground; tested).
- **Rise and fall are held frames** chosen by the sign of `vy` every tick; the apex tick is the first tick with `vy <= 0`.
- **Land plays once on touchdown** (impact 80 ms, recovery 100 ms for Carl; 80 and 110 ms for Donut). If the actor is **moving**, `run` cuts the landing short on the tick the impact frame ends (`LAND_CUT_AFTER_FRAMES = 1`, applied inside the frame advance): at 60 Hz the impact frame shows for 5 ticks and run shows on the 6th; the recovery frame is never shown for a single tick. A standing actor plays the whole landing (11 ticks) and goes to idle. A new jump interrupts the landing at any time.
- **Entering a state shows its first frame on the same tick**; time accumulates on later ticks. Idle restarts from its first frame after run or land (idle durations are the authored 420/260/560/260 ms for Carl and 320/260/180/260 ms for Donut, so the two actors do not blink in sync).
- **Stall hold** (`scene_anim::STALL_HOLD_TICKS = 6`). When a running actor's measured speed drops to zero, the run frame is held (no time passes, no restart) for 5 ticks and the 6th consecutive still tick shows idle. Donut's trail stalls for 1 to 3 ticks when Carl jumps a step or presses into a wall, and each flash of the taller idle pose restarted her run at frame 0. The hold applies **only while Carl still pushes** (his commanded speed is non-zero: a step jumped in place, a wall). A real stop (stick released, `|vx| <= 0.1`) has no hold: Carl idles on the first still tick and Donut idles on the same tick her trail stops, instead of hovering on a gallop frame. A run frame with **no paw on the floor** (the manifest's `locomotion.airborne_frames`, Donut's atlas frames 8 and 9, the gallop suspension) is never held either: a stall that lands on one goes to idle instead of freezing her mid-gallop.
- **Walls.** Carl's animation speed is his displacement, so pushing into a wall shows run for 5 ticks and then idle, not a run in place. The simulation is untouched (`vx` stays commanded; only the animation input is derived differently).
- After the exit arch is reached the simulation stops; both actors see zero speed and settle into idle.

### Clock and timing

Animation time is an integer count of 1/60000 s units (`anim::TICK_UNITS = 1000` per fixed tick, `UNITS_PER_MS = 60`), so 60 Hz ticks and whole-millisecond durations meet exactly; no float time and no wall clock. A frame advances when the accumulated units reach its duration and the remainder carries over, so a 43 ms frame lasts 3, 3, 2, 3, 3, 2, ... ticks (about 2.58 on average).

**Run frame time is speed-linked** (the authored durations of the run frames are ignored):

```text
Carl:   frame_ms = clamp( round( 8000 / |vx| ), CARL_RUN_FLOOR_MS, CARL_RUN_CEIL_MS )       (43, 90)
Donut:  frame_ms = clamp( round( w[i] * 1000 * 14.8 / |vx| ), 50, 130 )     w = [1,1,1,1,1.125,1]
```

| Constant (`anim.rs`) | Value | Note |
| --- | --- | --- |
| `CARL_RUN_GROUND_TENTHS_PX` | 80 | 8.0 px of ground per run frame (the authored stride) |
| `CARL_RUN_FLOOR_MS` | **43** | slip-free at the 185 px/s greybox `RUN_SPEED` (8000 / 185 = 43.2). The earlier 50 ms floor left about 1.3 px of foot slip per frame. **A review choice, adjustable after playtest** |
| `CARL_RUN_CEIL_MS` | 90 | slow starts look slower, not frantic |
| `DONUT_RUN_GROUND_TENTHS_PX` | 148 | 14.8 px of ground per frame |
| `DONUT_RUN_FLOOR_MS` / `CEIL_MS` | 50 / 130 | below about 114 px/s the cap applies (no walk cycle exists) |
| `DONUT_RUN_WEIGHTS` | 9/8 on run frame 4 | the gallop suspension (atlas frame 8) is held 1.125 times as long |

Integer maths (`RunPacing::frame_ms`): ground distance in tenths of a pixel, the weight as a fraction, the speed in milli-pixels per second, round half up. Resulting frame times (ms): Carl 90 at 0 px/s, 55 at 145, **43 at 185**; Donut 80, 80, 80, 80, 90, 80 at 185 px/s (the atlas values) and 102, 102, 102, 102, 115, 102 at 145. The run phase (frame and elapsed time) is kept when the speed changes; a changed speed only changes the length of the current and later frames.

### Placement

Every frame, grounded or airborne, is drawn with its pivot at the physics foot position: `actor_center(spec, foot_x, foot_y, flip)`. There is no ground snapping, so the rise and fall frames (feet drawn above the pivot row) and the gallop suspension float exactly as authored; grounded frames keep the 0 px foot sink. The documented 1 px flip asymmetry holds for the wider canvases (both widths are even; tested for 48 and 72). The contact shadow keeps the footprint (width and pivot column) of the old idle canvases (`CARL_SHADOW_FOOTPRINT` 36/19, `DONUT_SHADOW_FOOTPRINT` 46/22, pinned against the old manifests by a test), so the lighting is unchanged: the shadow does not widen for Donut's stretched run frames.

### Donut

She stays the passive trail follower, 32 px behind on Carl's path, and may overlap or pass through him. Her animation is derived from her own trail motion, not from Carl's state: `vx`, `vy` are the differences between her consecutive trail positions (x and foot height) times 60, and she is airborne when her foot is more than 0.25 px above the floor under her (the highest intact solid top below her foot at her x). She therefore starts running on the tick Carl does, takes off about 10 ticks after him (32 px at 185 px/s) and lands where he landed. Brief trail stalls are bridged by the stall hold above.

### Tests and evidence

- `anim::tests` (state machine, headless, data read from the committed manifests and grids): `clips_come_from_the_manifest_and_the_grid_source`, `run_pacing_matches_the_specs_at_the_documented_speeds` (0, 145, 185 px/s and Donut's rule), `the_anticipation_crouch_is_never_shown_on_a_takeoff`, `a_coyote_jump_after_a_walk_off_has_no_jump_start`, `hold_run_freezes_a_grounded_run_and_does_nothing_otherwise`, `idle_loops_with_the_authored_durations`, `idle_and_run_switch_on_the_move_threshold`, `run_frames_follow_the_speed_linked_time_and_loop`, `takeoff_starts_jump_start_on_the_same_tick_then_rises_then_falls`, `jump_start_is_not_cut_by_an_early_apex_but_hands_over_by_velocity`, `walking_off_a_ledge_goes_straight_to_fall`, `touchdown_plays_land_once_then_idles`, `running_cuts_land_short_only_after_the_impact_frame`, `a_new_jump_interrupts_land`, `the_same_input_trace_gives_the_same_frame_sequence`.
- `scene_anim::tests` (the real corridor world): `run_jump_plays_every_carl_state_in_order`, `carl_takes_off_on_the_impulse_tick_and_turns_at_the_apex`, `running_land_is_cut_into_run_but_the_standing_one_plays_out`, `donut_derives_her_own_states_from_her_trail`, `donut_keeps_running_through_one_tick_trail_stalls` (run_jump t41, walk_right t11), `carl_pushing_into_a_wall_settles_into_idle_after_the_stall_hold`, `the_stall_filter_holds_for_five_ticks_and_idles_on_the_sixth`, `a_real_stop_idles_donut_on_the_tick_her_trail_stops`, `a_stall_on_the_gallop_suspension_goes_to_idle_instead_of_hovering`, `a_stalled_run_is_never_held_on_a_frame_without_a_paw_on_the_floor`, `donut_holds_only_while_carl_still_pushes_and_never_past_six_ticks`, **`animation_does_not_change_the_simulation`** (the state hash is equal on every tick with and without animation, for four scenarios), `the_frame_sequence_is_deterministic`.
- `actor_sprite::tests`: placement, flip asymmetry for the new canvases, no snapping when airborne, shadow footprint.
- CI (`floor01-scene.yml`): the contract checker pins both locomotion atlases (16 frames, six tags in order, grounded frames on the pivot row, no frame below it); the exported atlas JSON must equal the committed grid sources (`REQUIRE_EXPORTED_ATLAS=1`: a missing export fails that test instead of skipping it); every frozen and live render step also compares `FLOOR1_ANIM_STATE` with the headless run; the `anim-sequence` mode captures 81 real 1920 x 1080 Bevy frames of the `run_jump` scenario (ticks 142, 145, ... 382), checks the final state against the headless run, builds the evidence with `scripts/make_anim_int_evidence.py` and also captures per-tick strips around a takeoff and a landing, and requires the committed `anim_trace.txt` and the `capture_log*.txt` files to be reproduced byte for byte.
- Evidence: [`docs/evidence/anim-int-01/`](../evidence/anim-int-01/README.md) (GIF, key stills, per-tick takeoff and landing strips, owner timeline diagram, state logs, `SHA256SUMS`).

### Known gaps

1. **Donut's hop onto a platform is not smooth.** `corridor.rs::record_trail` collapses vertical-only trail samples, so when Carl jumps a step at a wall (run_jump ticks 91-96) her replayed foot height jumps about 19 px in one tick (92, 96.9, 116.1, 122.1, 127.8, 133.2) and sits above the 108 px platform top, then snaps to 108 on touchdown. With the takeoff fix she shows `jump_start` frame 11 for that whole hop (no crouch), which reads as a hold rather than a pop, but the snap down is still visible. The cause is in the simulation trail, which this PR does not change; a later change to the trail (or a presentation-side smoothing of her foot height) must fix it.
2. **A trail stall on Donut's gallop suspension still flashes idle** (walk_right ticks 468-470: idle for 3 ticks, then the run restarts on frame 4). Holding frame 9 would hover her 4 px above the floor, so the hover rule wins over the flash rule. Stalls on a paw frame (run_jump t41, walk_right t11) are bridged.
3. **Coyote jump:** walk-off then jump shows `jump_fall` then `jump_rise` with no `jump_start`; documented as intended, not a defect.
4. No walk cycle exists for Donut below about 114 px/s, so a slow analogue stick shows a slow run (130 ms cap) whose paws drift backwards.
5. Frame times and the land cut rule are first choices, never played by a human.

### Follow-ups (deferred, not in this PR)

- **Per-tag Donut shadow footprint** (review item F3). The contact shadow keeps the old idle footprint (46 px, pivot column 22), but her run and rise frames extend 10 to 16 px behind it, so the shadow does not follow her stretched body. It needs a per-tag footprint in `ActorSpec` and a lighting comparison, so it was left out to keep the lighting byte-identical.
- Switch the Character Playground to the locomotion atlases and this state machine.
- Smooth Donut's foot height on step hops (gap 1).
- Grounded anticipation (frame 10) for a charged jump, if the design ever needs one.

## Lighting (COMP-02)

Status: functional, **not accepted**. A human must judge the look and the feel. The frames below are measurements against the owner-approved Pillow model, not an art acceptance.

### Pipeline

```text
 kit contract (pieces[].lights, layer light_group) + layout `lighting` profile
        | load (env_kit -> lighting::LightingScene): emitters, slot plan; floor pools
        |   projected once onto the walking line
        v
 per frame, read only:  view (camera left/top)  +  presentation tick  +  actor positions
        | lighting::gather -> lights in view px (flicker from the torch animation frame)
        | light_pass::pack_uniform -> one 3,920-byte uniform block
        v
 sprites (RenderLayers 1..6, one layer per slot)
   N slot cameras, Fixed 640x360 ortho, whole-pixel camera, transparent clear
        v
 6 slot images (virtual resolution, Rgba8UnormSrgb):
   parallax_far | parallax_mid | backwall+decal+collision+structure | prop | actors | foreground
        |
        +--> emissive pass  (virtual res): flame-coloured pixels, lit; nearer opaque pixels occlude; bright-pass
        |       -> blur H -> blur V   (gaussian, sigma 2.5 virtual px)
        |
        +--> composite pass (window res): textureLoad per slot (exact nearest), per slot
                 ambient + sum of pools + contact shadow, then haze; nearest opaque slot wins;
                 + additive glow (gallery halos only where the gallery shows) + bloom -> window
        v
 optional debug overlay camera (collision view), unlit, on top
```

Every stage after the slot cameras is a full-screen `Mesh2d` quad with a `LightingMaterial` seen by its own camera, ordered by `Camera::order`. One WGSL module (`src/light_pass.wgsl`) holds the three fragment entry points (`emissive`, `blur`, `composite`), chosen per material in `Material2d::specialize`.

**Colour space.** The Pillow model multiplies and mixes sRGB-encoded values, so the shader converts each texel to encoded space, does the same arithmetic, and converts back for the sRGB render target. Slot images are `Rgba8UnormSrgb`; the atlas round trip is exact. The slot cameras alpha-blend onto a transparent target, which leaves partially transparent pixels premultiplied, so the shader divides the alpha back out; the kit and actor atlases have 1-bit alpha anyway (`check_floor01_scene_contract.py` rejects partial alpha). The emissive and blur targets are `Rgba8Unorm` and hold encoded values directly, which avoids a `pow()` per blur tap.

**Integer scaling.** The composite reads the slot textures with integer `textureLoad` at `floor(pixel / (window / virtual))`, so the pixel art is never filtered and there are no seams at any window size. Only the light maths is continuous (pixel centre in virtual units). The 3x primary target is 1920x1080; 2x is 1280x720 / Deck 1280x800.

### Data flow from the contract

| Contract data | Used for |
| --- | --- |
| `pieces[].lights` (`offset`, `colour`, `radius`, `intensity`, `group`, `glow`, `flicker`, `project`, `project_dy`, `squash_y`) | emitters. Piece lights sit at `piece x/y + offset` (y down); strip lights repeat per parallax copy; `project: walk_line` moves the light to the walking line of its column plus `project_dy` (the line comes from the merged solids, read once at load) |
| `flicker` | one multiplier per animation frame; the frame is `animation_frame(durations, frame_offset, tick)`, so flicker is a pure function of the presentation tick |
| layer `light_group`, `actor_light_group`, `draw_order`, `lighting.bloom.emissive_layers` | the slot plan: contiguous draw orders with the same group and emissive flag share a slot (6 slots for the sample; a plan needing more than 6 fails the load) |
| `lighting.groups.<g>.ambient`, `light_scale`, `haze` (`colour`, `amount`, `floor_fog`) | per-slot ambient, light scale (actors 0.3, foreground 0.5), haze and ground fog |
| `lighting.actor_contact_shadow` | ellipse under each actor on the walking line, darkens the scene group only |
| `lighting.bloom` (`emissive_colours`, `threshold`, `intensity`, `radius_virtual`) | flame-only bloom mask and strength |

Validation at load: `presentation_only` must be `true`, ranges are checked, emitter groups must be `parallax_far`, `parallax_mid` or `scene`, more than 6 slots or 16 emissive colours is an error. **Budgets are proven at load.** `LightingScene::build` sweeps every camera position of every layout view and fails the load if more than 64 lights could reach one view (the sample peaks at far fewer; the figure is `LightingScene::worst_case_lights`); glow radius must not exceed the light radius, so culling by reach is exact. `pack_uniform` asserts the light, shadow and colour budgets instead of truncating. `gather` still truncates defensively, ranking lights without the flicker multiplier so the kept set cannot toggle between frames, and prints `FLOOR1_LIGHTING_WARNING` once; CI fails any run that prints it. Offsets and projections must be finite.

**One source of truth for constants.** Array lengths, flag bits and the falloff constants (`POOL_FALLOFF_K`, `GLOW_FALLOFF_K`) live in Rust and are prepended to the WGSL by `light_pass::shader_source`. A test parses and validates the final shader with `naga` (a test-only dev-dependency, already in `Cargo.lock` through wgpu) and asserts the `Params` block size and member offsets equal the Rust uniform layout. The falloff curve *shape* is written once in Rust (`pool_falloff`, `glow_falloff`, `haze_amount`) and once in WGSL; those Rust functions are not compared to the shader per pixel. What ties the shader to the reference is the gated render comparison below.

### Why it is presentation only

- `lighting` and `light_pass` take shared references to the kit and to simulation state and return new values; nothing writes back. The Bevy `update_lighting` system reads `Sim` immutably.
- Collision reads only `collidable` tiles of the collision layer; lights, groups and the profile are never read by it (existing test mutates pieces and decals without changing a solid).
- What is proven, exactly: (1) by types: `lighting` and `light_pass` take `&`-only references and return new values, and `update_lighting` reads `Sim` immutably; (2) by test `deriving_lights_leaves_the_simulation_state_untouched`: running `gather` on a live sim does not change its state hash at any of 420 ticks (this catches interior mutability or shared globals, nothing more); (3) end to end in CI: `live-lit` steps `--scenario walk_right` for 250 fixed ticks in the real render loop with the lighting systems active, `live-unlit` does the same with `--no-lighting`, and the final `FLOOR1_SCENE_STATE` lines must be equal to each other and to the pure headless simulation. The frozen `nolight-advanced` capture also matches, but that state comes from `build_sim` and is equal by construction, so it is not counted as proof.
- Screenshot evidence is captured with the pass on, as ADR-0007 requires.

### Tuning knobs (data, no code)

All in the layout `lighting` profile and the contract's piece lights; they can change without redrawing art: group `ambient`, `light_scale`, haze `colour` / `amount` / `floor_fog`, contact shadow `width_scale` / `half_height` / `strength`, bloom `threshold` / `intensity` / `radius_virtual` / `emissive_colours`, and each emitter's `radius`, `intensity`, `glow`, `flicker`, `squash_y`. Baked art and the pass must be tuned together: pools can over-brighten hot baked pixels.

### Design alternatives considered

| Alternative | Why not |
| --- | --- |
| Additive light sprites into a full-resolution light-accumulation texture, then multiply (the spike's proposal) | needs a full-resolution target and sprite overdraw per light; per-group ambient needs one accumulation per group anyway. Analytic per-pixel evaluation of about 30 lights is simpler and exact |
| One virtual-resolution target and a group id per pixel | sprites write one colour target; a group mask needs a second pass or a custom sprite pipeline. Separate slot targets are cheap (6 x 640x360) and keep the parallax gallery lit independently |
| Tinting sprites by the group ambient on the CPU | cannot express `ambient + sum(lights)` multiplied per pixel |
| Bevy's built-in HDR `Bloom` | it blooms everything bright; the owner-approved model blooms only flame colours with occlusion |
| Compute shaders or a custom render-graph node | not needed; camera-ordered `Material2d` quads use only documented, stable APIs |

### Reference comparison (Bevy vs Pillow)

Same camera window (`abc_640x360`), actors and tick as the ENV-01 lit preview, both at 1920x1080. Produced by `--reference-frame` and `scripts/compare_floor01_lighting.py`; evidence in [`docs/evidence/floor01-scene/`](../evidence/floor01-scene/README.md).

| Region | Pixels | mean L* Bevy | mean L* Pillow | delta L* | mean dE76 | p95 dE76 |
| --- | --- | --- | --- | --- | --- | --- |
| wall | 444,438 | 15.13 | 14.91 | +0.21 | 0.55 | 0.88 |
| gallery | 256,734 | 20.76 | 20.53 | +0.23 | 0.5 | 0.92 |
| floor | 174,825 | 29.94 | 29.72 | +0.22 | 0.55 | 0.84 |
| actors | 19,278 | 46.04 | 45.85 | +0.19 | 0.48 | 0.93 |
| whole frame | 2,073,600 | 23.24 | 23.01 | +0.23 | 0.55 | 0.9 |

Warm share (R > B + 8): Bevy 0.4203, Pillow 0.4203. Cold share (B > R + 8): Bevy 0.3942, Pillow 0.3935. Largest per-pixel dE76 5.97; 1,456 of 2,073,600 pixels exceed dE76 3. The script fails the CI job above its gates (see CI).

Where it diverges: the differences are tiny and concentrated at the flames, where the bloom halo is computed differently (see deviations below; the diff panel in `floor01-lighting-bevy-vs-pillow.png` lights up only at the five torches and the gallery flame). Everywhere else the Bevy frame is about 0.2 L* brighter, a rounding difference (Pillow truncates to 8 bits, the GPU rounds). One pose and one camera window only: the other corridor segments, moving torches and jumping actors were not compared numerically.

### Performance

Measured with `--perf-frames 600` (120 warm-up frames, vsync off via `PresentMode::AutoNoVsync`) on the start view, idle. The number is the **frame interval** (wall clock between frames): it is bounded by the slowest of the CPU main thread, the render thread and the GPU, not a GPU-only time. One machine, one run per row, no variance study.

Device (local, real swapchain, X11): NVIDIA GeForce RTX 4060 Ti, Vulkan, proprietary driver 615.71.09 (wgpu `AdapterInfo` line in the log). Build profiles: `release` is `cargo build --release`; `debug` is the repository's `dev` profile (opt-level 1) used by CI.

| Build | Window | Lighting | median ms | p95 ms | max ms |
| --- | --- | --- | --- | --- | --- |
| release | 1920x1080 (3x) | on | 1.28 | 1.56 | 2.75 |
| release | 1920x1080 (3x) | off | 0.71 | 0.81 | 2.83 |
| release | 1280x720 (2x) | on | 1.41 | 1.60 | 4.69 |
| release | 1280x720 (2x) | off | 0.74 | 0.81 | 2.58 |
| debug | 1920x1080 (3x) | on | 5.19 | 6.43 | 9.20 |
| debug | 1920x1080 (3x) | off | 1.36 | 1.62 | 3.29 |
| debug | 1280x720 (2x) | on | 5.35 | 6.51 | 8.65 |
| debug | 1280x720 (2x) | off | 1.37 | 1.63 | 3.85 |

Reading: the pass roughly doubles the release frame interval (about +0.6 ms) and adds about 4 ms in the unoptimised dev build. The window size barely changes it, which says these runs are CPU-side (extraction and queueing for 6 slot cameras over about 2,900 sprites plus the extra stages), not fill-rate bound on this GPU. A GPU timing was not taken. The same run under Xvfb (software X presentation, debug build, 1280x720) gave 27.2 ms lit and 22.7 ms unlit median; that is dominated by the X11 copy and says nothing about the GPU.

CI prints the same `FLOOR1_PERF` line from its report-only (`continue-on-error`) `perf-lit` and `perf-unlit` steps under Xvfb with the runner's software Vulkan (Mesa lavapipe) at 1280x720, debug build, 100 frames; those numbers are a health signal only and are recorded in the CI log and artifact. Steam Deck: **not measured**; no Deck, Deck Verified or 60 fps claim is made.


### Known deviations from the Pillow model

- Emissive and blur targets are `Rgba8Unorm` (encoded values stored directly); the reference numbers were re-measured after that change and did not move.
- Light evaluation is analytic per output pixel; Pillow resamples a 512 x 512 falloff sprite bicubically and rounds light centres to whole output pixels.
- Pillow truncates float results to 8 bits; the GPU rounds, so the Bevy frame is about +0.2 L* brighter on average.
- Bloom: the bright-pass and gaussian run at virtual resolution on texel centres (Pillow box-downsamples the output-resolution frame by 2 first, then blurs); the blurred result is sampled bilinearly. Flames differ slightly in the halo.
- Contact shadows stay on the walking line under the actor while it jumps (the preview only had grounded actors).
- Unsupported on purpose: emitters in the `foreground` or `actors` group, more than 6 slots, 64 lights, 8 shadows or 16 emissive colours (all but the contact shadows are checked at load).
- Window: the window is created with `scale_factor_override(1.0)`, so an OS scale factor cannot turn the 3x into a fractional ratio; window size equals virtual size x scale in real pixels.

## Known limits

1. Lighting is functional but unreviewed: no human has judged the look, the flicker, or the readability of actors. The reference comparison covers one pose and one camera window.
2. The animation (idle, run, jump, land) is a first integration that no human has played: see [Animation > Known gaps](#known-gaps).
3. No viewport letterbox adapter: at a non-integer window scale (or a size that is not a multiple of 640x360 / 640x400) the slot textures are still read with exact nearest sampling, but pixels may be uneven. `--scale N` with a matching window is the supported path; letterboxing is deferred (see `camera.rs`).
4. Flip asymmetry: the flipped sprite's pivot column lands 1 px left of the unflipped one (same as the Playground). Half-pixel centres are deliberately not used.
5. Donut passes through Carl when he reverses; she does not walk around or avoid him. Owner decision 2026-11-16: allowed for now; revisit later.
6. The sample layout is flat with two low platforms (at most two tiles); it is not authored level geometry.
7. Performance numbers are frame intervals on one desktop GPU (see Lighting > Performance); nothing was measured on a Steam Deck and no Steam Deck claim is made.
8. Software-Vulkan CI renders are technical checks only; they say nothing about game feel, readability or controller response. A human must play it: see the [evidence checklist](../evidence/floor01-scene/README.md).
9. The pass handles at most 6 slots, 64 lights, 8 contact shadows and 16 emissive colours per frame; a different layer plan or a denser layout needs the limits in `lighting.rs` and `light_pass.wgsl` raised together.

## CI

`.github/workflows/floor01-scene.yml`: export, contract check, negative tests (`tests/test_floor01_scene_*.py`), build, headless smoke, then Xvfb renders (desktop 1920x1080, desktop-720, deck, advanced, advanced-deck, nolight, nolight-advanced, reference, collision, plus two frame-interval probes) through `scripts/ci_floor01_render_smoke.sh`, which also checks the real window geometry with `xwininfo` (1920x1080 / 1280x720 / 1280x800), that the lit and unlit captures differ, and that the unlit run reports the same `FLOOR1_SCENE_STATE` as the lit run; `scripts/compare_floor01_lighting.py` compares the reference frame with the Pillow model and **fails** the job on divergence (whole-frame mean dE76 <= 1.5, p95 <= 3, |mean delta L*| <= 1, every region mean dE76 <= 2.5; measured 0.55 / 0.90 / 0.23 / <= 0.55, so the gates leave margin for a software rasteriser); the `live-lit` / `live-unlit` steps step the scenario live and compare final states; any `FLOOR1_LIGHTING_WARNING` fails a render step. the `anim-sequence` step captures 81 real frames of the `run_jump` scenario and builds the ANIM-INT-01 evidence (see [Animation](#animation-anim-int-01)); `cargo fmt`, `clippy` and the Rust tests (including all floor01 tests) run in `game.yml`; the three Bevy workflows share one rust-cache key. Evidence artifact: `the-crawl-floor01-scene`.

**Rights:** the character art is public fan-concept art under the recorded scope in [the rights register](../04-production/rights-and-release.md); it does not establish rights to distribute a playable game.
