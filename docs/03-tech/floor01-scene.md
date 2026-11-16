# THE CRAWL — Floor 1 corridor scene (COMP-01)

**Status:** functional integration for human review; not accepted. The scene composes the owner-approved ENV-01 kit (`env-kit-v2` contract, `env-kit-layout-v2` sample layout) with the approved Carl 52 px and Donut 40 px atlases. **It renders pixel layers only: there is no lighting pass, bloom or haze yet** (COMP-02 per [ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md)). The tiles carry baked light, shadow and pool decals, so screenshots look flatter and less atmospheric than the A1 look-dev preview. That is expected, not a regression.

It is a separate binary from the combat greybox and the Character Playground; both keep working. There are no enemies, bombs, health or new mechanics here, and `attack` / `ability` inputs are ignored.

## Run

```sh
bash scripts/export_assets.sh                    # atlases into the gitignored generated/art/
python3 scripts/check_floor01_scene_contract.py   # actor pivots + kit atlases vs contract
cargo run --locked --bin floor01_scene            # 640x360 virtual, 1280x720 window
cargo run --locked --bin floor01_scene -- --deck  # 640x400 virtual, 1280x800 window
```

Run from the repository root. A missing atlas is a setup failure: the binary prints `FLOOR1_ASSET_FAILURE: ...` and exits non-zero (also if textures are not loaded within 10 s). It never substitutes placeholders. Plain `cargo run` still starts the greybox (`default-run`).

| Flag | Effect |
| --- | --- |
| `--deck` | 640x400 virtual view, 1280x800 window (Steam Deck-like 16:10) |
| `--scale <1..8>` | window = virtual size x scale (default 2; 3 for a 1080p check) |
| `--scenario <name>` | scripted input: `idle`, `walk_right` (holds right, jumps at steps, reaches the exit in 613 ticks), `turn_around` (right for 150 ticks, then left) |
| `--capture-tick <N>` | needs `--scenario`: pre-simulate N fixed ticks headlessly, then render that frozen state |
| `--freeze` | do not advance the simulation (start state, or the captured state) |
| `--debug-collision` | overlay merged solids (red), walking lines (cyan) and Carl's AABB (green) |
| `--headless-floor01-smoke` | no window or GPU: parse, derive solids, simulate, print state lines |

Controls: **A/D** or arrows or left stick/D-pad move, **Space** or South jumps, **R** or North restarts, **Esc** or Start quits. Reaching the B2 arch prints `FLOOR1_EXIT_REACHED` and stops the simulation (restart with R). Moving and jumping still show the **idle** frames: no locomotion art exists yet.

## Architecture

Pure library modules (headless, unit-tested) and one Bevy adapter:

| Module | Role |
| --- | --- |
| `env_kit` | serde structs for contract v2 and layout v2 (unknown fields, including all lighting data, are ignored); derived draw lists, the z table, parallax strips, markers and collision. Rejects v1 files. |
| `corridor` | `CorridorWorld` (Carl, Donut, solids, tick, state hash), `SceneSim` (world + camera + input), scripted scenarios. |
| `scroll_camera` | integer, deterministic camera rig |
| `actor_sprite` | pivot, flip and foot maths, idle frame from the tick |
| `prototype::step_hero` | the greybox's hero physics, extracted so both scenes share one implementation; `World::move_hero` delegates (a golden bit-trace test, added first, proves the extraction changed nothing) |
| `src/bin/floor01_scene.rs` | input adapter, sprite spawning, per-frame sync, watchdog, headless smoke |

Rendering reads simulation state and never feeds it. Lighting, parallax, decals and every piece are never read by collision.

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

Donut trails Carl by **32 px of horizontal path length**, replaying the trail of Carl's foot positions (x and foot height), including his jump arcs, so she lands on the same walking lines. She has no collision, combat, physics or influence on Carl (tested: Carl's trajectory equals the bare greybox hero's). When Carl turns around she keeps walking to the turn point before reversing, passing through him. She starts 32 px behind the spawn. She is not a playable caster.

## Determinism

The fixed 60 Hz tick drives everything simulated: hero, Donut, camera, torch and idle frames. State hashing is FNV-1a over f32 bit patterns. `FLOOR1_SCENE_STATE tick=... hero=... cam=... hash=...` is printed by the headless smoke and by the render; CI requires them to match for the frozen captures (`--scenario walk_right --capture-tick 250`). Rendered pixels are not compared bit-for-bit across GPUs.

## Known limits

1. **No lighting, bloom or haze** (COMP-02). Expect flatter frames than A1.
2. Idle frames are shown while moving and jumping; no locomotion or jump art exists.
3. No viewport letterbox adapter: at a non-integer window scale (or a size that is not a multiple of 640x360 / 640x400) sprites may show seams or uneven pixels. `--scale N` with a matching window is the supported path; letterboxing is deferred (see `camera.rs`).
4. Flip asymmetry: the flipped sprite's pivot column lands 1 px left of the unflipped one (same as the Playground). Half-pixel centres are deliberately not used.
5. Donut passes through Carl when he reverses; she does not walk around or avoid him.
6. The sample layout is flat with two low platforms (at most two tiles); it is not authored level geometry.
7. No performance data has been collected.
8. Software-Vulkan CI renders are technical checks only; they say nothing about game feel, readability or controller response. A human must play it: see the [evidence checklist](../evidence/floor01-scene/README.md).

## CI

`.github/workflows/floor01-scene.yml`: export, contract check, negative tests (`tests/test_floor01_scene_*.py`), `cargo fmt/clippy/test`, headless smoke, then Xvfb renders (desktop, deck, advanced, advanced-deck, collision) through `scripts/ci_floor01_render_smoke.sh`. Evidence artifact: `the-crawl-floor01-scene`.

**Rights:** the character art is public fan-concept art under the recorded scope in [the rights register](../04-production/rights-and-release.md); it does not establish rights to distribute a playable game.
