# THE CRAWL — Floor 1 corridor scene (COMP-01)

**Status:** functional integration for human review; not accepted. The scene composes the owner-approved ENV-01 kit (`env-kit-v2` contract, `env-kit-layout-v2` sample layout) with the approved Carl 52 px and Donut 40 px atlases. **COMP-02 adds the A1 lighting pass** ([ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md)): cold ambient per layer group, warm torch pools, floor pools, contact shadows, per-layer haze, additive glow and flame-only bloom, computed at output resolution on top of the integer-scaled pixel layers. It is presentation only and `--no-lighting` restores the COMP-01 look. No acceptance of the look or of performance is claimed: see [Lighting](#lighting-comp-02) for measurements and known divergences.

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
| `--perf-frames <N>` | measure N frame intervals after 120 warm-up frames with vsync off, print `FLOOR1_PERF ...` and exit |
| `--scenario <name>` | scripted input: `idle`, `walk_right` (holds right, jumps at steps, reaches the exit in 613 ticks), `turn_around` (right for 150 ticks, then left) |
| `--capture-tick <N>` | needs `--scenario`: pre-simulate N fixed ticks headlessly, then render that frozen state |
| `--freeze` | do not advance the simulation (start state, or the captured state) |
| `--debug-collision` | overlay merged solids (red), walking lines (cyan) and Carl's AABB (green) |
| `--headless-floor01-smoke` | no window or GPU: parse, derive solids, simulate, print state lines |

Controls: **A/D** or arrows or left stick/D-pad move, **Space** or South jumps, **R** or North restarts, **Esc** or Start quits. Reaching the B2 arch prints `FLOOR1_EXIT_REACHED` and stops the simulation (restart with R); torches and idle frames keep animating on a separate presentation tick. The window is not resizable (ADR-0007 integer scaling: window = virtual size x scale). Moving and jumping still show the **idle** frames: no locomotion art exists yet.

## Architecture

Pure library modules (headless, unit-tested) and one Bevy adapter:

| Module | Role |
| --- | --- |
| `env_kit` | serde structs for contract v2 and layout v2 (unknown fields, including all lighting data, are ignored); derived draw lists, the z table, parallax strips, markers and collision. Rejects v1 files. |
| `corridor` | `CorridorWorld` (Carl, Donut, solids, tick, state hash), `SceneSim` (world + camera + input), scripted scenarios. |
| `scroll_camera` | integer, deterministic camera rig |
| `actor_sprite` | pivot, flip and foot maths, idle frame from the tick |
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

The fixed 60 Hz tick drives everything simulated: hero, Donut, camera, torch and idle frames. State hashing is FNV-1a over f32 bit patterns. `FLOOR1_SCENE_STATE tick=... hero=... cam=... hash=...` is printed by the headless smoke and by the render process; CI requires the two lines to be identical for the frozen captures (`--scenario walk_right --capture-tick 250`). That proves **cross-process determinism of the simulation state** (the same scripted input gives the same state in two separate runs). It does **not** prove that the rendered pixels show that state; pixel correctness rests on the screenshot size/colour checks, the author's review and the human playtest. Rendered pixels are not compared bit-for-bit across GPUs.

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

**Colour space.** The Pillow model multiplies and mixes sRGB-encoded values, so the shader converts each texel to encoded space, does the same arithmetic, and converts back for the sRGB render target. Slot images are `Rgba8UnormSrgb`; the atlas round trip is exact.

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

Validation at load: `presentation_only` must be `true`, ranges are checked, emitter groups must be `parallax_far`, `parallax_mid` or `scene`, more than 6 slots or 16 emissive colours is an error. At most 64 lights reach the GPU per frame (the sample peaks at far fewer; a test sweeps every camera position; if ever exceeded the weakest are dropped and a warning is printed once).

### Why it is presentation only

- `lighting` and `light_pass` take shared references to the kit and to simulation state and return new values; nothing writes back. The Bevy `update_lighting` system reads `Sim` immutably.
- Collision reads only `collidable` tiles of the collision layer; lights, groups and the profile are never read by it (existing test mutates pieces and decals without changing a solid).
- Test `lighting_never_changes_the_simulation_state` runs the scripted walk with lighting derived every tick and without, and compares the state hash at checkpoints. CI also renders `--no-lighting --scenario walk_right --capture-tick 250` and requires the same `FLOOR1_SCENE_STATE` line as the lit render of the same capture.
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
| gallery | 256,734 | 20.76 | 20.53 | +0.23 | 0.5 | 0.93 |
| floor | 174,825 | 29.94 | 29.72 | +0.22 | 0.55 | 0.84 |
| actors | 19,278 | 46.04 | 45.85 | +0.19 | 0.48 | 0.93 |
| whole frame | 2,073,600 | 23.24 | 23.01 | +0.23 | 0.55 | 0.9 |

Warm share (R > B + 8): Bevy 0.4204, Pillow 0.4203. Cold share (B > R + 8): Bevy 0.3942, Pillow 0.3935. Largest per-pixel dE76 6.46; 1,574 of 2,073,600 pixels exceed dE76 3.

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

CI prints the same `FLOOR1_PERF` line from its `perf-lit` and `perf-unlit` steps under Xvfb with the runner's software Vulkan (Mesa lavapipe) at 1280x720, debug build, 100 frames; those numbers are a health signal only and are recorded in the CI log and artifact. Steam Deck: **not measured**; no Deck, Deck Verified or 60 fps claim is made.


### Known deviations from the Pillow model

- Light evaluation is analytic per output pixel; Pillow resamples a 512 x 512 falloff sprite bicubically and rounds light centres to whole output pixels.
- Pillow truncates float results to 8 bits; the GPU rounds, so the Bevy frame is about +0.2 L* brighter on average.
- Bloom: the bright-pass and gaussian run at virtual resolution on texel centres (Pillow box-downsamples the output-resolution frame by 2 first, then blurs); the blurred result is sampled bilinearly. Flames differ slightly in the halo.
- Contact shadows stay on the walking line under the actor while it jumps (the preview only had grounded actors).
- Unsupported on purpose: emitters in the `foreground` or `actors` group, more than 6 slots, 64 lights, 8 shadows or 16 emissive colours per frame.

## Known limits

1. Lighting is functional but unreviewed: no human has judged the look, the flicker, or the readability of actors. The reference comparison covers one pose and one camera window.
2. Idle frames are shown while moving and jumping; no locomotion or jump art exists.
3. No viewport letterbox adapter: at a non-integer window scale (or a size that is not a multiple of 640x360 / 640x400) the slot textures are still read with exact nearest sampling, but pixels may be uneven. `--scale N` with a matching window is the supported path; letterboxing is deferred (see `camera.rs`).
4. Flip asymmetry: the flipped sprite's pivot column lands 1 px left of the unflipped one (same as the Playground). Half-pixel centres are deliberately not used.
5. Donut passes through Carl when he reverses; she does not walk around or avoid him. Owner decision 2026-11-16: allowed for now; revisit later.
6. The sample layout is flat with two low platforms (at most two tiles); it is not authored level geometry.
7. Performance numbers are frame intervals on one desktop GPU (see Lighting > Performance); nothing was measured on a Steam Deck and no Steam Deck claim is made.
8. Software-Vulkan CI renders are technical checks only; they say nothing about game feel, readability or controller response. A human must play it: see the [evidence checklist](../evidence/floor01-scene/README.md).
9. The pass handles at most 6 slots, 64 lights, 8 contact shadows and 16 emissive colours per frame; a different layer plan or a denser layout needs the limits in `lighting.rs` and `light_pass.wgsl` raised together.

## CI

`.github/workflows/floor01-scene.yml`: export, contract check, negative tests (`tests/test_floor01_scene_*.py`), build, headless smoke, then Xvfb renders (desktop 1920x1080, desktop-720, deck, advanced, advanced-deck, nolight, nolight-advanced, reference, collision, plus two frame-interval probes) through `scripts/ci_floor01_render_smoke.sh`, which also checks the real window geometry with `xwininfo` (1920x1080 / 1280x720 / 1280x800), that the lit and unlit captures differ, and that the unlit run reports the same `FLOOR1_SCENE_STATE` as the lit run; `scripts/compare_floor01_lighting.py` compares the reference frame with the Pillow model. `cargo fmt`, `clippy` and the Rust tests (including all floor01 tests) run in `game.yml`; the three Bevy workflows share one rust-cache key. Evidence artifact: `the-crawl-floor01-scene`.

**Rights:** the character art is public fan-concept art under the recorded scope in [the rights register](../04-production/rights-and-release.md); it does not establish rights to distribute a playable game.
