# Floor 01 Camera Scenarios Lab (CAMERA-LAB)

**Status: isolated probe, not merged, awaiting the owner's playtest.** Everything here is evidence for a camera decision, not a decision. No ADR-0007 change is made or implied: the virtual resolution (640x360, Deck 640x400), integer final scaling, the full-resolution A1 lighting pass and the approved sprite heights (Carl 52 px, Donut 40 px) are untouched. The browser package this builds on is in [`tools/camera-lab/`](../../tools/camera-lab/README.md); the [review](../../tools/camera-lab/REVIEW.md) lists what was sound, what conflicted with the project rules and what changed. Evidence is in [`docs/evidence/camera-lab/`](../evidence/camera-lab/README.md).

Read with [ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md), [input and camera](input-camera.md), the [MVP controller contract](../01-design/mvp-controller-contract.md), [Floor 01 scene](floor01-scene.md) and the [Ball of Swine arena notes](../01-design/floor-01-ball-of-swine-arena-gameplay.md).

## Run it

```bash
# once: export the approved atlases (Aseprite or LibreSprite on PATH), then the SWINE-01 atlas
bash scripts/export_assets.sh
bash scripts/camera_lab_assets.sh

# play (window = output size; 1920x1080, 3840x2160 and 1280x800 are the tested outputs)
cargo run --release --bin camera_lab -- --room 6 --mode 2p --coverage auto --output 1920x1080
cargo run --release --bin camera_lab -- --room 1 --mode solo --coverage 1.5 --output 1280x800   # Deck (16:10)

# headless: the deterministic state trace of a scripted run, and the camera metrics table
./target/debug/camera_lab --room 3 --mode 2p --script --headless
./target/debug/camera_lab --metrics

# real in-engine captures under xvfb, like the repository CI scripts
xvfb-run -a -s '-screen 0 1920x1080x24' ./target/debug/camera_lab --room 3 --coverage 1.5 \
  --mode 2p --script --output 1920x1080 --capture-sequence 574:1:1 --capture-dir artifacts/camera-lab
bash scripts/camera_lab_evidence.sh shots|videos|crops|perf OUT_DIR
```

Options: `--room 1..6`, `--coverage auto|1|1.25|1.5|1.75`, `--mode solo|2p`, `--output WxH` (16:10 outputs choose the Deck view), `--script` (the scripted bot drives, same inputs every run), `--idle`, `--separate X`, `--metrics`, `--seed N`, `--start-tick N`, `--exit-after-ticks N`, `--perf-frames N`, `--capture-sequence first:step:count --capture-dir DIR`.

Controls (approved semantics; keyboard fallbacks are test-only):

| Action | Gamepad | Keyboard (player 1 / solo) | Keyboard (player 2) |
|---|---|---|---|
| Move | left stick or D-pad | A D | arrows |
| Jump | Cross / A | Space or W | Up |
| Primary (Carl jab, then haymaker; Donut swat) | Square / X | F or J | M |
| Physical (Carl push kick, **not a grab**; Donut pounce) | Triangle / Y | G or K | N |
| Switch actor (**solo only**, ignored in 2P) | L1 / LB | Q or Tab | - |
| Special | Circle / B | L | - |

The special is **story-gated**: it does nothing in the lab (no Donut magic before the biscuit, no Carl dynamite before the Guild) and is logged as `special_locked`. Hotkeys: `0` automatic camera, `1`..`4` fixed 1.0 / 1.25 / 1.5 / 1.75, `R` restart the room, `Esc` quit. In 2P, pad 1 is Carl and pad 2 is Donut; with one pad, player 2 uses the keyboard. In solo the first pad (or the keyboard) drives the active actor and the other is an AI companion.

## What was built

Isolated: a new Bevy-free module tree `src/camera_lab/` and one binary `src/bin/camera_lab.rs`. **No shared module was edited** (`floor01_scene`, `combat`, `goblin`, `feel`, `env_kit`, `lighting`, `light_pass`, `actor_sprite` are used as they are; `lib.rs` only gains `pub mod camera_lab;`). The existing fixtures keep their replay hashes (`bash scripts/ci_floor01_combat.sh headless` is unchanged).

| Module | Role |
|---|---|
| `room` | Common world schema loaded from `tools/camera-lab/scenarios.json`: render bounds (the whole drawn layout, whole 16 px tiles) separate from collision (floor, walls, platforms, authored access steps, tagged props), protected floor and exit, reachability for each actor in every prop state. |
| `layout` | Composes an `env-kit-layout-v2` document per room from existing Floor 01 kit pieces (below) and builds it with the existing `EnvKit` loader and A1 lighting contract. |
| `camera_op` | The camera operator: fixed coverage modes and the automatic shared framing. Presentation only. |
| `sim` | The deterministic simulation at the fixed 60 Hz: two actors, enemies, props, gate, projectiles, events and a state hash. |
| `script` | A bot that turns a directive per actor into inputs as a pure function of the simulation, and the six scripted runs (plus a solo variant). |
| `session` | Simulation plus camera; derives the camera input from the simulation (never the other way); frame metrics. |
| `art` | Which approved atlas and frame a state draws; reads the exported atlas JSON, never constants. |

### Reuse of the existing systems

- **Combat:** `MoveRunner`, `resolve_strike`, Carl's jab, haymaker and push kick (`CARL_MOVES`, the `feel` numbers) and `Goblin` (tell, swing, token rule, stagger, knockback, launched defeat) are used unchanged. A single `Hittable` list holds enemies then props, so one `resolve_strike` hits enemies and destructible props with the same hit-stop and per-move dedup (a side effect: a destroyed prop logs `enemy_defeated` with a target index at or above the enemy count).
- **Destruction:** props are individually tagged (`PropKind`, hit points, `breakable`), hit by the same strikes, and a **support** carries a predetermined structural link to a hanging **weight** (docs/03-tech/destruction.md L3): break the support, the weight falls under fixed gravity, hurts whatever it crosses once each and lands (`weight_fell`, `impact_damage`, `weight_landed`). Knocked-back goblins also bump props. There is no per-pixel debris with gameplay effect; the cosmetic seed of a break comes from the seeded RNG and is in the hash. The floor, steps, platforms, walls and exit are never destructible (tested through all six runs).
- **Physics:** Carl uses the same jump buffer, coyote time, sweep and landing as `prototype::step_hero`; a test proves identical positions over 900 ticks. Carl's jump peaks at 92 px, no more.
- **Input:** `InputFrame` press edges; per-player slots own one actor each.

### New in the lab (and plainly a probe)

- **2P input ownership.** No 2P system existed. The lab adds the minimum: player 1 owns Carl, player 2 owns Donut, L1/LB does nothing in 2P, a fixed horizontal leash of 760 px (a gameplay constant that does **not** depend on the camera), and a downed actor stands up beside the partner after 90 ticks. Hot-join, disconnect and AI hand-off are **not** implemented or tested.
- **Solo companion.** A deterministic follower that hops obstacles and swings at an enemy in front of it. Crude; it exists so solo switching has something to switch to.
- **Donut** (pre-biscuit): swat and pounce as `MoveSpec` data with weak reactions, a 171 px high jump, 3 health. Proposals, not approved balance.
- **Foes** other than the goblin share one generic state machine (neutral, tell, active, recover, hurt, down) driven by per-kind data: llama (ranged spit, aimed when fired, seeded jitter), engineer, scatterer, hoarder, troglodyte, juicer and the Ball of Swine (a committed charge in the direction shown by the tell, smashes weak walls, stuns on a closed gate or a wall). All timings are lab hypotheses; the atlases are the approved readability sets, none of which is integrated elsewhere.
- **Moving gate** (ROOM-06): closed ticks 0..160, lift, open to 390, lower, a 420-tick cycle, a visible warning before it closes, and it never closes on a body.

## Rooms, art and honest limits

Every room is a render layout far larger than any camera view. The wall between the cornice and the floor is brick backwall (taller `brick_upper` / `brick_mid` bands for tall rooms), with **stacked arch bays** from the floor up (2 tiers in rooms 1 to 5, 3 in ROOM-06): the ground tier opens onto the parallax gallery, upper tiers are blind arcades, each with piers, chains and torches, plus pilasters under the cornice. A test checks that **every** cell of the render bounds holds a drawn tile (or the gallery behind a ground arch) and that the lights of the widest possible view (1120x700 at every position) never exceed the A1 pass budget.

The workshop (ROOM-03), gym (ROOM-05), hoarder lair (ROOM-04) and swine vault (ROOM-06) have **no approved dedicated art**. They recompose existing pieces (copper pipe runs and a valve for the workshop, rubble for the lair, a denser arcade for the gym and the arena). Props use the approved opening-kit pieces (raking shore, loose blocks, timber set, rubble, door timber): a timber set stands for a weak wall, a stone block for a crate or a weight, so collider and art sizes differ in places. These are greybox compositions, not art proposals; no art was invented.

Other limits: the enemy-vs-enemy rebound of ROOM-01's brief is not implemented; hazards are a damage zone drawn with the kit's grit; the boss is a straight-line charge with no Tusklings; the scripted bots are crude (they lose health, skip some fights and always take the same line), so the headless numbers describe framing, not difficulty.

## Rendering at coverage above 1.0x (the chosen approach)

A coverage of `c` shows `640c x 360c` (16:9) or `640c x 400c` (16:10) world pixels. The renderer keeps ADR-0007's model and only changes how much world one frame shows:

1. The pixel layers are drawn **one texel per world pixel** with nearest sampling into the top-left region of the existing lighting slot targets. Those targets are allocated at the largest coverage (1120x630 or 1120x700); each frame the slot cameras use a viewport and a fixed projection of exactly the current coverage, so there is no resampling inside the pixel layers.
2. The existing A1 composite pass reads the slots with integer `textureLoad` (nearest, no filtering, no seams) at **output** resolution and does the light maths, haze, shadows and bloom there. Torch pools and glow radii are in world pixels, so they scale with the coverage like the scenery.
3. The camera origin is a whole world pixel and the **coverage width is a multiple of 32 px**, so the view, half of it, and the height at both aspects are whole numbers (the four candidates are exactly 640 / 800 / 960 / 1120). The automatic mode zooms in whole 32 px steps (2.5 percent each), smoothed in time. A step is a hard 2.5 percent scale change of the picture, not a continuous zoom.
4. The output size must be a window of that aspect (1920x1080 and 3840x2160 for 16:9, 1280x800 for 16:10); no letterboxing is added.

**Trade-off, measured.** When the output is an integer multiple of the coverage width every texel is exactly `k` pixels wide: no shimmer. Otherwise a column of texels is `floor(k)` or `ceil(k)` pixels wide, so some columns are one pixel wider; the pattern moves when the camera pans by a whole world pixel, which is a 1 px width shimmer on 1-pixel detail (brick joints, outlines). Nothing is blurred and nothing is resampled. The alternatives (sharp bilinear after an integer pre-scale, or letterboxing to an integer scale) either soften edges, which departs from ADR-0007's exact-sampling statement, or give up screen area; they were not built.

Effective on-screen height of the approved sprites (52 px and 40 px sources) and the texel width per output:

| Coverage | 1080p Carl / Donut px | 1080p texel px | 4K Carl / Donut px | 4K texel px | Deck 1280x800 Carl / Donut px | Deck texel px |
|---|---|---|---|---|---|---|
| 1.00x (640) | 156.0 / 120.0 | 3 exact | 312.0 / 240.0 | 6 exact | 104.0 / 80.0 | 2 exact |
| 1.25x (800) | 124.8 / 96.0 | 2.4 (2 or 3) | 249.6 / 192.0 | 4.8 (4 or 5) | 83.2 / 64.0 | 1.6 (1 or 2) |
| 1.50x (960) | 104.0 / 80.0 | 2 exact | 208.0 / 160.0 | 4 exact | 69.3 / 53.3 | 1.33 (1 or 2) |
| 1.75x (1120) | 89.1 / 68.6 | 1.71 (1 or 2) | 178.3 / 137.1 | 3.43 (3 or 4) | 59.4 / 45.7 | 1.14 (1 or 2) |

These are computed by `camera_op::effective_px` (unit-tested against the table) and match the output line the binary prints on start (`LAB_OUTPUT ... carl_px=... donut_px=...`). Only 1.0x (everywhere) and 1.5x (at 1080p and 4K) keep an integer ratio. A larger output is not a larger coverage: 4K shows the same world as 1080p, with finer pixels.

## Camera operator

`camera_op` is pure and unit-tested. Per tick it takes the two actors (body boxes in layout pixels), an optional readable threat and an impact flag, and returns a whole-pixel view.

- **Fixed modes** 1.0 / 1.25 / 1.5 / 1.75 at 16:9 and 16:10 (views 640x360 to 1120x630 and 640x400 to 1120x700).
- **Automatic shared framing.** Each actor has a safety region (96 px sideways, 56 px above, 20 px below the body). The coverage needed is the tightest width that fits both regions (and, for 16:10, the height); the target is raised at once (smoothly) when needed and lowered only after 96 ticks of lasting slack, a 72-tick dwell and a 64 px dead band, never during hit-stop or a fresh hit. The floor sits at 80 percent of the view height so the architecture above stays visible. The pan follows the framing centre with a rate and a speed cap (5 px per tick, 1.5 during an impact, 12 in an emergency when a region is outside).
- **Threat bias.** A boss tell or a llama tell shifts the view toward the threat (up to 16 percent of the width, only while every safety region still fits) and adds the charge lane to the framing, only once the tell is readable (54 ticks or fewer left). A hidden tell is ignored, so the camera never knows more than the player sees.
- **Clamp.** The view is always inside the authored camera bounds (the render bounds), so it can never show void. Safety regions are clipped to the bounds: an actor against a wall cannot demand padding beyond it.
- **No hard snap.** The pan is speed limited; the only larger per-tick move is one 32 px zoom step against a clamped room edge.
- **Presentation only.** The camera input is derived from the simulation after the tick; nothing flows back. The test `authoritative_hash_is_identical_at_every_coverage` runs each scripted room at 1.0, 1.25, 1.5, 1.75, the automatic camera at 16:9 and 16:10, and a fixed 1.5 at 16:10, and requires the **same authoritative final hash and the same event log** each time.

Separation arithmetic (both safety regions on screen): 640 px of view holds up to about 424 px between the actors, 800 up to 584, 960 up to 744, 1120 up to 904. The 2P leash is 760 px, so 1.5x keeps both **bodies** visible always and both safety regions up to 744 px; the automatic camera then takes one more step (992 px, about 1.55x).

## Tests

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets          # 288 library tests, 42 of them in camera_lab
python3 scripts/check_docs.py
bash scripts/ci_floor01_combat.sh headless  # the existing replay hashes are unchanged
./target/debug/camera_lab --room 3 --mode 2p --script --headless   # state trace, LAB_FINAL_HASH
```

What the `camera_lab` tests prove (all headless, no GPU, no window):

- **Camera operator (`camera_op`):** the four coverages are exact whole-pixel views at both aspects; effective sprite sizes at 1080p, 4K and 1280x800; fixed modes keep their size and never show void while actors walk across and beyond the room; the automatic mode never shows void for random actor positions; both safety regions stay in view once settled; no zoom pumping when the separation oscillates around a threshold; no zoom-in before the delay or during a hit-stop; pan speed limits and no zoom change in an impact hold; a readable charge biases the view but keeps both actors, a hidden or distant one is ignored; a small room is never exceeded.
- **Rooms and layout:** the data loads in English; render bounds are whole tiles and larger than the biggest view; platforms, steps and props are inside the play area; the protected exit is reachable for Carl and for Donut with every prop intact and with every prop broken; only authored props are destructible; high platforms are Donut-only; every room composes through the existing `EnvKit` loader with its A1 lighting; **every cell of the render bounds is drawn** so no view can show void; composition is deterministic; the A1 light budget holds in the widest view at every position.
- **Simulation:** Carl's body physics equals `prototype::step_hero` over 900 ticks; the jump rule; 2P ownership (player 1 never moves Donut, L1/LB does not steal); the solo switch hands Donut the controls and the AI takes Carl; the story-gated special does nothing and is logged; the leash is a fixed constant; destruction never removes the protected floor, steps, platforms or walls in any scripted run; ROOM-03 cause and effect in order (support broken, weight fell, impact damage, weight landed); the gate is open at least 40 percent of its cycle and never closes on a body; the boss tell, charge and gate stun happen in that order; reading the events does not change the hash.
- **Session:** `authoritative_hash_is_identical_at_every_coverage` (above); scripted runs reproduce exactly and different seeds diverge only through the seeded RNG; solo runs switch once; the automatic camera keeps both actors in their safety regions in every scripted run; the framing metrics are reproducible and a wider fixed coverage keeps more of the duo in view.
- **Art:** every atlas the lab draws loads with the tags the pose mapping uses (skipped with a note when the atlases are not exported).

## Evidence

All captures are real Bevy frames (xvfb, the repository's render approach; the Vulkan adapter on the capture machine was an NVIDIA RTX 4060 Ti, recorded in the evidence). The full set (180 screenshots: 6 rooms x fixed 1.0 / 1.25 / 1.5 / 1.75 and automatic x 1920x1080, 3840x2160, 1280x800 x solo scripted action and 2P separated) was produced and checked; a compact curated subset is committed. See the [evidence README](../evidence/camera-lab/README.md) for the file list, the commands and the hashes.

## Results

Framing numbers are headless and reproducible (`camera_lab --metrics`, the six scripted 2P runs). "Attacks on screen" is the share of ticks in which a foe in its tell or active frames within 700 px of an actor had its whole body in view. These numbers measure **framing**, not readability: whether a tell or a hit reads is a human verdict from the captures and the videos, and was not given.

| Room | Coverage | Both actors in view | Actor lost | Attacks on screen | Hits on screen | Auto: steps / mean | Median ms (1080p) |
|---|---|---:|---:|---:|---:|---:|---:|
| 1 goblins | 1 | 100% | 0% | 100% | 100% | - | 1.43 |
| 1 goblins | 1.25 | 100% | 0% | 100% | 100% | - | 1.75 |
| 1 goblins | 1.5 | 100% | 0% | 100% | 100% | - | 1.84 |
| 1 goblins | 1.75 | 100% | 0% | 100% | 100% | - | 1.85 |
| 1 goblins | auto | 100% | 0% | 100% | 100% | 0 / 1.00 | 1.44 |
| 2 llamas | 1 | 99% | 0% | 21% | 100% | - | 1.31 |
| 2 llamas | 1.25 | 100% | 0% | 65% | 100% | - | 1.77 |
| 2 llamas | 1.5 | 100% | 0% | 85% | 100% | - | 1.61 |
| 2 llamas | 1.75 | 100% | 0% | 95% | 100% | - | 1.61 |
| 2 llamas | auto | 100% | 0% | 71% | 100% | 17 / 1.14 | 1.79 |
| 3 workshop | 1 | 100% | 0% | 100% | 100% | - | 1.59 |
| 3 workshop | 1.25 | 100% | 0% | 100% | 100% | - | 1.37 |
| 3 workshop | 1.5 | 100% | 0% | 100% | 100% | - | 1.39 |
| 3 workshop | 1.75 | 100% | 0% | 100% | 100% | - | 1.82 |
| 3 workshop | auto | 100% | 0% | 100% | 100% | 0 / 1.00 | 1.73 |
| 4 hoarder | 1 | 100% | 0% | 100% | 100% | - | 1.76 |
| 4 hoarder | 1.25 | 100% | 0% | 100% | 100% | - | 1.74 |
| 4 hoarder | 1.5 | 100% | 0% | 100% | 100% | - | 1.89 |
| 4 hoarder | 1.75 | 100% | 0% | 100% | 100% | - | 1.90 |
| 4 hoarder | auto | 100% | 0% | 100% | 100% | 0 / 1.00 | 1.63 |
| 5 gym (2P split) | 1 | 85% | 15% | 38% | 100% | - | 1.43 |
| 5 gym (2P split) | 1.25 | 99% | 0% | 100% | 100% | - | 1.79 |
| 5 gym (2P split) | 1.5 | 100% | 0% | 100% | 100% | - | 1.88 |
| 5 gym (2P split) | 1.75 | 100% | 0% | 100% | 100% | - | 1.70 |
| 5 gym (2P split) | auto | 100% | 0% | 100% | 100% | 23 / 1.15 | 1.39 |
| 6 swine boss | 1 | 100% | 0% | 79% | 100% | - | 1.63 |
| 6 swine boss | 1.25 | 100% | 0% | 83% | 100% | - | 1.41 |
| 6 swine boss | 1.5 | 100% | 0% | 83% | 100% | - | 1.81 |
| 6 swine boss | 1.75 | 100% | 0% | 84% | 100% | - | 1.91 |
| 6 swine boss | auto | 100% | 0% | 81% | 100% | 13 / 1.07 | 1.85 |

Reading the table (agent observations from the captures and the traces, **not an owner verdict**):

- **ROOM-01 (goblins).** All three goblins and both actors stay in view at every coverage (the bots never separate far), so framing does not discriminate. In the captures the fight reads at 1.0x and 1.5x; at 1.5x the next goblin is visible earlier and the whole arch behind the fight is drawn. Whether a wider view hurts the *feel* of the jab, haymaker and kick is the open question only a hand-held playtest answers.
- **ROOM-02 (llamas, vertical).** The one clear framing result: with the spit tell on the upper gallery or across the lane, only 21 percent of attack ticks have the llama fully in view at 1.0x, 65 percent at 1.25x, 85 percent at 1.5x and 95 percent at 1.75x. A vertical ambush is not readable at 1.0x. The automatic camera as built reaches 71 percent: it frames a tell only once it is readable (a 42-tick tell, a 54-tick lookahead) and then needs time to widen; ranged foes would need earlier awareness (see the proposal).
- **ROOM-03 (workshop).** Cause and effect (support, falling weight, crushed goblin, landing dust) fit in a 640x360 view because the whole sequence is about 300 px wide; 1.5x adds context (the platform, the pipes) without hiding it. The falling weight and the dust are visible in the captures.
- **ROOM-04 (hoarder lair).** The scatterers (a 48x32 frame, 15 px of opaque insect) are the smallest threats: 45 px high on screen at 1080p and 1.0x, 30 px at 1.5x, 26 px at 1.75x, and 20 px at 1.5x on the Deck. They stay visible against the floor in the captures but at 1.75x and on the Deck they are a few pixels of contrast; the lair's rubble adds low-contrast clutter.
- **ROOM-05 (gym, separation).** In the separating run (Donut on the 240 px platform, Carl on the floor, up to the 760 px leash) both actors are fully in view for 85 percent of the ticks at 1.0x (an actor is less than half visible for 15 percent), 99 percent at 1.25x and 100 percent at 1.5x and above. The automatic camera keeps both in view for 100 percent of the ticks with 23 whole 32 px steps (roughly eight zoom transitions) and a mean coverage of 1.15x.
- **ROOM-06 (swine boss).** The charge lane is long, so only 79 to 84 percent of attack ticks have the boss fully in view at any coverage and 81 percent with the automatic camera: with both actors at the far end of a 2060 px room the boss cannot always fit together with the two safety regions. The impact on an actor is always in view (the actors are). In the separated capture the boss is cut by the right edge at 1.0x and 1.25x, mostly in view at 1.5x and fully in view at 1.75x.


### Frame time and memory

Measured on the real GPU (an NVIDIA RTX 4060 Ti on the development workstation, Vulkan), vsync off, `--release`, 300 frames after warm-up, scripted 2P runs ([full tables](../evidence/camera-lab/perf_frame_times.md)): the median frame is **1.3 to 1.9 ms at 1920x1080**, **1.7 to 2.2 ms at 3840x2160** and **1.4 to 2.0 ms at 1280x800**, p95 at most 3.0 ms and p99 at most 4.6 ms (isolated spikes up to 11.6 ms at 4K and 15.8 ms once at 1280x800, a single frame). The wider coverage costs little: for the same room the median grows by roughly 0.1 to 0.4 ms from 1.0x to 1.75x, and the automatic camera sits between 1.0x and 1.5x. The dev profile (`cargo build`, opt-level 1) is about 6 ms. Resident memory is 350 to 430 MB. The existing `floor01_scene` corridor (2864 tiles) measures 1.3 to 1.6 ms (1080p) and 1.6 ms (4K) the same way, so the lab rooms (4400 to 8900 sprites) cost about the same on this GPU. **Not a Steam Deck measurement.** Under xvfb, which presents the whole window in software, the same runs read 55 to 65 ms at 1080p independent of the coverage; that is the present path, not the scene, and is not used.

## Limitations

- Evidence is from scripted bots and a handful of captures; **no human playtest, no DualSense or Steam Deck run, no handheld legibility check** and no hands-on game-feel verdict exist. CI success is not that verdict.
- Frame times were measured under xvfb on one development machine (adapter in the evidence), with a 4K and a Deck-sized window; they are a health signal, not a Steam Deck or release claim.
- The rooms use about 4400 to 8900 sprites each (every tile is a sprite, as in `floor01_scene`); the frame-time cost is dominated by that and not by the coverage. A production level would batch or window them.
- 2P hot-join, disconnect, rejoin and controller assignment UI are not implemented.
- Donut's values, enemy timings, the boss charge and the gate cycle are hypotheses; nothing here is canon, topology, balance or an approved camera.

## RECOMMENDATION (a proposal, pending the owner's human game-feel verdict; no ADR-0007 change)

**Proposal only. It records what the evidence supports and what it cannot; the owner's hands-on verdict on a 1080p display and a Steam Deck decides.**

1. **Rest state: 1.0x.** It is the only coverage that is an exact integer scale on every supported output (3x at 1080p, 6x at 4K, 2x on the Deck), it keeps the sprites biggest (Carl 156 px, Donut 120 px at 1080p) and the smallest threat (a scatterer) at 45 px. Short brawls, kicks and hit frames stay at 1.0x. This is a reasoned default, not a measured preference for the feel.
2. **Adaptive camera, capped at 1.5x (960x540).** Let the automatic operator widen in whole 32 px steps only when it has to: the duo is more than about 424 px apart, a readable ranged or boss tell is outside the frame, never during hit-stop or a fresh hit, and zoom back in only after the situation has been calm for about 1.6 s. In the evidence this is what removes the real failures: at 1.0x an actor is lost 15 percent of a separation run and 79 percent of llama attacks are off screen, at 1.5x neither happens. It also leaves ROOM-03-style cause and effect at 1.0x. 1.5x is also the only widened coverage that is an **exact** integer scale at 1080p (2x) and 4K (4x), so it has no uneven texel columns; Carl is 104 px and Donut 80 px at 1080p, 208 and 160 at 4K.
3. **1.25x only as a transit step, not a fixed setting.** It is 2.4x at 1080p (and 4.8x at 4K, 1.6x on the Deck): texels alternate between two widths. It is acceptable for the seconds the camera spends passing through it, but it is not a default.
4. **1.75x stays exploratory.** It is 1.71x at 1080p (1 or 2 px texels), Carl 89 px, Donut 69 px, a scatterer 26 px; its only unique gain was the boss fully in view in a 2060 px room, which the room geometry, a shorter charge lane or a boss-aware framing can provide without it. Keep it as a debug and screenshot mode, or for one short authored reveal; do not ship it as gameplay without a high-confidence 1080p and Deck playtest.
5. **Steam Deck 1280x800.** Only 1.0x is an integer scale (2x). Every wider coverage gives uneven 1 or 2 px texels (50 percent width jitter) and a Carl of 83 px (1.25x) or 69 px (1.5x). Proposal: keep 1.0x as the rest state, allow the automatic widening only for separation and the boss, and **prefer reducing the 2P leash on the Deck** (about 424 px at 1.0x) over widening; or accept 1.5x there only if handheld legibility passes. The probe cannot say; it needs a Deck.
6. **Two things the probe suggests building before any policy ships:** earlier awareness of ranged foes (the automatic camera as built reaches 71 percent of llama attacks on screen against 85 percent for a fixed 1.5x, because it only reacts once the tell is readable), and a boss-aware lane framing for very long arenas. Both are camera-only work.
7. **Preserve the tall architecture regardless of the policy.** The rooms draw 400 to 700 px of masonry, arch tiers and torches above the action; the void test passes for every position and coverage, and the A1 light budget holds for the widest view.

**What to look at when playing** (record each as a note, not a pass/fail): does a widened view hurt the weight of the jab, haymaker and kick (ROOM-01, 1.0x against the auto camera); can you read the llama spit and the boss tell in time (ROOM-02, ROOM-06 at 1.5x against 1.0x); are the scatterers findable (ROOM-04 at 1.5x); do zoom steps feel like pumping (ROOM-05 auto, watch the 2.5 percent steps and the 1-pixel texel shimmer at 1.25x and 1.75x); does the separation feel recoverable with the leash (ROOM-05, 2P with two pads).

