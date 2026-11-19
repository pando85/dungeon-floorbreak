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

- **2P input ownership.** No 2P system existed. The lab adds the minimum: player 1 owns Carl, player 2 owns Donut, L1/LB does nothing in 2P, a fixed horizontal leash of 416 px (one platform-independent constant, see the camera section), and a downed actor stands up beside the partner after 90 ticks. Hot-join, disconnect and AI hand-off are **not** implemented or tested.
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
3. The camera origin is a whole world pixel and the **coverage width is a multiple of 32 px**, so the view, half of it, and the height at both aspects are whole numbers (the four candidates are exactly 640 / 800 / 960 / 1120). The automatic mode rests only on widths that are an exact integer scale of the output and passes through the in-between 32 px widths in a short transit (see the camera section); a transit step is a hard 2.5 percent scale change of the picture, not a continuous zoom.
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

`camera_op` is pure and unit-tested. Per tick it takes the two actors (body boxes in layout pixels), a list of threats and an impact flag, and returns a whole-pixel view.

- **Fixed modes** 1.0 / 1.25 / 1.5 / 1.75 at 16:9 and 16:10 (views 640x360 to 1120x630 and 640x400 to 1120x700). These are the comparison modes; they are not limited by the cap below.
- **Automatic shared framing, resting only on integer scales.** The automatic camera has a set of **rest widths** for the output: 640 plus every 32 px width up to a **hard cap of 960 px (1.5x)** that is an exact integer scale of the output width. 1080p: {640, 960}. 4K: {640, 768, 960} (768 is exactly 5x). Steam Deck 1280x800, 720p and 1440p: {640} (nothing wider is an integer scale there). It never exceeds 960. The target is always the smallest rest width that fits the framing need; the widths in between exist **only as transit frames**, taken at a fixed pace of one 32 px step per 2 ticks (one per tick when something that must be framed is outside the view): a 640 to 960 transit is 20 ticks, 10 when urgent. It widens at once, straight to the rest width that fits, and returns only after 96 ticks of lasting slack with a 72-tick dwell and a 64 px dead band, never during hit-stop or a fresh hit.
- **What is framed.** Each actor has a safety region (96 px sideways, 56 px above, 20 px below the body). Threats are added in priority order while the width cap still holds, the lowest priority dropped first: (3) a boss in its tell or charge, a ranged foe in its tell, its shot and the start of its recovery, and a spit in flight; (2) a ranged foe that has **acquired** an actor with a clear line within 520 px, so the camera starts widening when the llama takes aim and not when the tell has already begun; (1) a large foe next to an actor (so an idle boss beside Donut is not cropped). Up to four are handed to the operator. A threat that is not yet readable is ignored.
- **Urgent pan and zoom.** When an actor's safety region or a framed threat is not fully inside the view, the pan cap rises from 5 to 12 px per tick and the zoom-out uses the one-tick step pace.
- **Floor position.** The lowest actor's feet sit at 80 percent of the view height at 1.0x and 86 percent from 1.5x, so a wider view shows more architecture and not more foundation.
- **Threat bias.** The highest-priority threat shifts the view toward it (up to 16 percent of the width, only while every safety region still fits).
- **Clamp.** The view is always inside the authored camera bounds (the render bounds), so it can never show void. Safety regions are clipped to the bounds: an actor against a wall cannot demand padding beyond it.
- **No hard snap.** The pan is speed limited; the largest per-tick move is one 32 px zoom step plus the urgent pan.
- **Presentation only.** The camera input is derived from the simulation after the tick; nothing flows back. `Threat.readable` is always true from the session (it only gates the framing of enemies the player could see); the camera's information is limited by what the session passes, not by a visibility check. The test `authoritative_hash_is_identical_at_every_coverage` runs each scripted room at 1.0, 1.25, 1.5, 1.75, the automatic camera at 16:9 and 16:10, and a fixed 1.5 at 16:10 on one output, and requires the **same authoritative final hash and the same event log** each time.

### The 2P leash is one platform-independent constant

The leash is gameplay and feeds the authoritative state, so it must not depend on the output: a Deck, a 1080p and a 4K machine must give the same state for the same inputs, and docking or resizing mid-run must change no rule. It is therefore **one constant, 416 px** (`DEFAULT_LEASH_PX`): both safety regions need `separation + 220` px (2 x 96 plus Carl's and Donut's half widths), and 416 + 220 fits a 640 px (1.0x) frame, so same-screen co-op needs no widening on any output. The documented alternative, **736 px** (`WIDE_LEASH_PX`, both regions inside the 1.5x cap), can only be selected by an explicit scenario or config choice (`--leash wide`, `Session::with_leash`), never by the output. **Owner playtest (2026-11-19):** both 416 px and 736 px play well and either is acceptable; the default stays 416 px. Test `authoritative_hash_is_identical_at_every_output_resolution`: every scripted 2P run at 1280x800, 1920x1080, 2560 wide and 3840x2160, with the automatic camera and a fixed 1.5x, ends on the same authoritative hash and event log. `leash_for_cap` remains only as the arithmetic that relates a cap to a leash (`leash + 220 <= cap`, tested).

The leash is collision safe: a tick that would widen the gap past it puts the actor whose move widened it back on its body of the start of the tick (a state that was legal), so nobody is ever placed inside geometry. A partner that stays held for 45 ticks and gives no input **catches up by walking** (the companion follower, with hops) until its player moves. **Owner decision:** the idle-partner auto-walk is a gameplay choice (it moves a human's character without input); it is kept in the probe so the leash never strands anyone, but it needs the owner's explicit approval or removal. Tests: `the_leash_is_consistent_with_the_camera_cap` and `the_leash_holds_without_snapping_through_geometry_and_the_partner_catches_up` (no tick ends inside a solid, no position jump, the idle partner catches up, at 416 and 736).

Separation arithmetic (both safety regions on screen): 640 px of view holds up to about 420 px between the actors, 768 up to 548, 960 up to 740 (1.5x), 1120 up to 900. With the 416 px default leash a 640 view always holds both safety regions; with the 736 px alternative the 960 cap does, so 1.75x has no framing job left either way.

## Forward compatibility with the open combat branches

The probe depends on the shared combat structs only through APIs that survive the open work: it keeps its own press-edge detection and its own events (`MoveStarted`, goblin tell and swing, `Crushed`) instead of `InputFrame` and `CombatEvent` constructors; builds `Reaction` and `MoveSpec` values with `..Reaction::NONE` / `..CARL_JAB`; matches `GoblinMode` with a wildcard (and treats any mode that is not neutral, attacking, down or a corpse as "flying or staggered"); and reads `CombatEvent::Hit` with `..`. The one edit to a shared file is a `Default` derive on `combat::Attacker` (`Attacker { x, y, half_height, facing, ..Default::default() }`), so a new `Attacker.player` field does not break the lab.

Verified by test-merging this branch into throwaway worktrees (not pushed): into `origin/feat/death-b-profiles` (PR #94, which stacks on #79: `Reaction.launch`, `RawInput.physical`, `CombatEvent` cause and force, `GoblinMode::Launched` / `Downed`, `Attacker.player`) and into `origin/feat/i03-goblin-defeat-modes` (PR #79): both merge with **no conflicts**, `cargo check --locked --all-targets` passes and the 51 `camera_lab` tests pass on each. Remaining rebase steps: none required for compilation. After they land: the push kick launches a goblin instead of staggering it (the lab draws a launched goblin with the knockback or down frames and lets it bump props), so the scripted runs, their hashes and the goblin-kill videos change; the lab credits all hits to player slot 0 (`Attacker.player` stays at its default); regenerate the evidence then. If a branch changes the `StrikeTarget` or `resolve_strike` signatures, `sim.rs` needs a one-line adaptation.

## Tests

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets          # the library tests, 51 of them in camera_lab
python3 scripts/check_docs.py
bash scripts/ci_floor01_combat.sh headless  # the existing replay hashes are unchanged
./target/debug/camera_lab --room 3 --mode 2p --script --headless   # state trace, LAB_FINAL_HASH
```

What the `camera_lab` tests prove (all headless, no GPU, no window):

- **Camera operator (`camera_op`):** the four coverages are exact whole-pixel views at both aspects; effective sprite sizes at 1080p, 4K and 1280x800; the rest widths per output (1080p {640, 960}, 4K {640, 768, 960}, Deck/720p/1440p {640}); fixed modes keep their size and never show void while actors walk across and beyond the room; the automatic mode never shows void and never exceeds 960 for random actor positions at three outputs; it rests only on rest widths and transits within 24 ticks; both safety regions stay in view once settled; no zoom pumping when the separation oscillates around a threshold; no zoom-in before the delay or during a hit-stop; pan speed limits; a readable charge biases the view but keeps both actors, a hidden or distant one is ignored; threats are dropped by priority when the cap cannot hold them; a threat outside the view widens at the urgent pace; the floor sits lower when wider; a small room is never exceeded.
- **Rooms and layout:** the data loads in English; render bounds are whole tiles and larger than the biggest view; platforms, steps and props are inside the play area; the protected exit is reachable for Carl and for Donut with every prop intact and with every prop broken; only authored props are destructible; high platforms are Donut-only; every room composes through the existing `EnvKit` loader with its A1 lighting; **every cell of the render bounds has a drawn tile entry** (or the gallery behind a ground arch), so no view can show void (the test checks the entry, not the opacity of its pixels, which is adequate because the kit tiles are opaque); composition is deterministic; the A1 light budget holds in the widest view at every sampled position (a 160 px grid, not every position).
- **Simulation:** Carl's body physics equals `prototype::step_hero` over 900 ticks; the jump rule; 2P ownership (player 1 never moves Donut, L1/LB does not steal); the solo switch hands Donut the controls and the AI takes Carl; the story-gated special does nothing and is logged; the leash is consistent with the cap and collision safe; destruction never removes the protected floor, steps, platforms or walls in any scripted run; ROOM-03 cause and effect in order (support broken, weight fell, impact damage, weight landed); the gate is open at least 40 percent of its cycle and never closes on a body; the boss tell, charge and gate stun happen in that order; reading the events does not change the hash.
- **Session:** `authoritative_hash_is_identical_at_every_coverage` (above); scripted runs reproduce exactly and different seeds diverge only through the seeded RNG; solo runs switch once; the automatic camera keeps both actors in their safety regions in every scripted run; **the width histogram of every room at 1080p, 4K and 1280x800: at least 95 percent of the ticks on rest widths, nothing above 960, no transit longer than 24 ticks**; ROOM-02 auto keeps at least 85 percent of the llama attack ticks on screen, ROOM-05 never loses an actor, ROOM-06 keeps the boss at least 85 percent, short brawls stay at 1.0x; the framing metrics are reproducible and a wider fixed coverage keeps more of the duo in view.
- **Art:** every atlas the lab draws loads with the tags the pose mapping uses (skipped with a note when the atlases are not exported).

## Evidence

All captures are real Bevy frames (xvfb, the repository's render approach; the Vulkan adapter on the capture machine was an NVIDIA RTX 4060 Ti, recorded in the evidence). The full set (180 screenshots: 6 rooms x fixed 1.0 / 1.25 / 1.5 / 1.75 and automatic x 1920x1080, 3840x2160, 1280x800 x solo scripted action and 2P separated) was produced and checked; a compact curated subset is committed. See the [evidence README](../evidence/camera-lab/README.md) for the file list, the commands and the hashes.

## Results

Framing numbers are headless and reproducible (`camera_lab --metrics`, the six scripted 2P runs at 1920x1080 for the 16:9 rows; the 1280x800 rows are in [framing_metrics.md](../evidence/camera-lab/framing_metrics.md)). "Attacks on screen" is the share of ticks in which a foe in its tell or active frames within 700 px of an actor had its whole body in view. These numbers measure **framing**, not readability: whether a tell or a hit reads is a human verdict from the captures and the videos, and was not given.

| Room | Coverage | Both actors in view | Actor lost | Attacks on screen | Hits on screen | Auto: rest changes / mean coverage |
|---|---|---:|---:|---:|---:|---:|
| 1 goblins | 1 | 100% | 0% | 100% | 100% | - |
| 1 goblins | 1.25 | 100% | 0% | 100% | 100% | - |
| 1 goblins | 1.5 | 100% | 0% | 100% | 100% | - |
| 1 goblins | 1.75 | 100% | 0% | 100% | 100% | - |
| 1 goblins | auto | 100% | 0% | 100% | 100% | 0 / 1.00 |
| 2 llamas | 1 | 100% | 0% | 19% | 100% | - |
| 2 llamas | 1.25 | 100% | 0% | 73% | 100% | - |
| 2 llamas | 1.5 | 100% | 0% | 94% | 100% | - |
| 2 llamas | 1.75 | 100% | 0% | 98% | 100% | - |
| 2 llamas | auto | 100% | 0% | 93% | 100% | 2 / 1.23 |
| 3 workshop | 1 | 100% | 0% | 100% | 100% | - |
| 3 workshop | 1.25 | 100% | 0% | 100% | 100% | - |
| 3 workshop | 1.5 | 100% | 0% | 100% | 100% | - |
| 3 workshop | 1.75 | 100% | 0% | 100% | 100% | - |
| 3 workshop | auto | 100% | 0% | 100% | 100% | 0 / 1.00 |
| 4 hoarder | 1 | 100% | 0% | 100% | 100% | - |
| 4 hoarder | 1.25 | 100% | 0% | 100% | 100% | - |
| 4 hoarder | 1.5 | 100% | 0% | 100% | 100% | - |
| 4 hoarder | 1.75 | 100% | 0% | 100% | 100% | - |
| 4 hoarder | auto | 100% | 0% | 100% | 100% | 0 / 1.00 |
| 5 gym (2P split) | 1 | 100% | 0% | 90% | 100% | - |
| 5 gym (2P split) | 1.25 | 100% | 0% | 100% | 100% | - |
| 5 gym (2P split) | 1.5 | 100% | 0% | 100% | 100% | - |
| 5 gym (2P split) | 1.75 | 100% | 0% | 100% | 100% | - |
| 5 gym (2P split) | auto | 100% | 0% | 100% | 100% | 2 / 1.15 |
| 6 swine boss | 1 | 100% | 0% | 81% | 100% | - |
| 6 swine boss | 1.25 | 100% | 0% | 83% | 100% | - |
| 6 swine boss | 1.5 | 100% | 0% | 90% | 100% | - |
| 6 swine boss | 1.75 | 100% | 0% | 93% | 100% | - |
| 6 swine boss | auto | 100% | 0% | 89% | 100% | 2 / 1.03 |

ROOM-05 (the separating run) with the documented wide-leash alternative (736 px; the default leash is 416 px, which keeps both actors inside a 1.0x frame on every output):

| Coverage | Both actors visible | Actor lost | Attacks on screen | Rest changes | Mean coverage | Max actor gap px |
|---|---:|---:|---:|---:|---:|---:|
| 1 | 85% | 14% | 38% | 0 | 1.00 | 734 |
| 1.25 | 100% | 0% | 100% | 0 | 1.25 | 734 |
| 1.5 | 100% | 0% | 100% | 0 | 1.50 | 734 |
| 1.75 | 100% | 0% | 100% | 0 | 1.75 | 734 |
| auto | 100% | 0% | 100% | 2 | 1.16 | 734 |

**Automatic camera: rest widths.** Ticks per view width, with the exact-integer-scale rest widths of each output (the table is generated by `camera_lab --metrics`; at least 95 percent of the ticks sit on a rest width in every room and output, tested):

| Room | Output | Rest widths | Ticks per width (w:ticks) | On rest | Longest transit (ticks) | Max width |
|---|---|---|---|---:|---:|---:|
| ROOM-01 | 1080p | 640/960 | 640:1500 | 100.0% | 0 | 640 |
| ROOM-01 | 4K | 640/768/960 | 640:1500 | 100.0% | 0 | 640 |
| ROOM-01 | 1280x800 | 640 | 640:1500 | 100.0% | 0 | 640 |
| ROOM-02 | 1080p | 640/960 | 640:782 672:3 704:3 736:3 768:3 800:3 832:3 864:3 896:3 928:3 960:691 | 98.2% | 18 | 960 |
| ROOM-02 | 4K | 640/768/960 | 640:794 672:3 704:3 736:3 768:358 800:3 832:3 864:3 896:3 928:3 960:324 | 98.4% | 10 | 960 |
| ROOM-02 | 1280x800 | 640 | 640:1500 | 100.0% | 0 | 640 |
| ROOM-03 | 1080p | 640/960 | 640:1800 | 100.0% | 0 | 640 |
| ROOM-03 | 4K | 640/768/960 | 640:1800 | 100.0% | 0 | 640 |
| ROOM-03 | 1280x800 | 640 | 640:1800 | 100.0% | 0 | 640 |
| ROOM-04 | 1080p | 640/960 | 640:1800 | 100.0% | 0 | 640 |
| ROOM-04 | 4K | 640/768/960 | 640:1800 | 100.0% | 0 | 640 |
| ROOM-04 | 1280x800 | 640 | 640:1800 | 100.0% | 0 | 640 |
| ROOM-05 | 1080p | 640/960 | 640:1468 672:4 704:4 736:4 768:4 800:4 832:4 864:4 896:4 928:4 960:596 | 98.3% | 18 | 960 |
| ROOM-05 | 4K | 640/768/960 | 640:1480 672:4 704:4 736:4 768:608 | 99.4% | 6 | 768 |
| ROOM-05 | 1280x800 | 640 | 640:2100 | 100.0% | 0 | 640 |
| ROOM-06 | 1080p | 640/960 | 640:2249 672:3 704:3 736:3 768:3 800:3 832:3 864:3 896:3 928:3 960:124 | 98.9% | 18 | 960 |
| ROOM-06 | 4K | 640/768/960 | 640:2249 672:3 704:3 736:3 768:3 800:3 832:3 864:3 896:3 928:3 960:124 | 99.0% | 10 | 960 |
| ROOM-06 | 1280x800 | 640 | 640:2400 | 100.0% | 0 | 640 |

Summary: ROOM-01, 03 and 04 stay at 640 for the whole run. ROOM-02, 05 and 06 widen to 960 (768 at 4K) and return, 98 to 99 percent of the ticks on a rest width, the longest transit 18 ticks (10 at 4K), nothing above 960. On the Deck (1280x800) the automatic camera never leaves 640 because no wider width is an integer scale there.

Reading the table (agent observations from the captures and the traces, **not an owner verdict**):

- **ROOM-01 (goblins).** All three goblins and both actors stay in view at every coverage (the bots never separate far), so framing does not discriminate; the automatic camera stays at 1.0x. In the captures the fight reads at 1.0x and 1.5x; at 1.5x the next goblin is visible earlier and the whole arch behind the fight is drawn. Whether a wider view hurts the *feel* of the jab, haymaker and kick is the open question only a hand-held playtest answers.
- **ROOM-02 (llamas, vertical).** The clearest framing result: with the spit tell on the upper gallery or across the lane, only 21 percent of attack ticks have the llama fully in view at 1.0x, 74 percent at 1.25x, 94 percent at 1.5x and 98 percent at 1.75x. A vertical ambush is not readable at 1.0x. The reworked automatic camera reaches **93 percent** (it was 71 percent): it starts widening when a llama acquires an actor, keeps the shooter in the framing set through the tell, the shot and the start of the recovery, frames the spit in flight, and pans at the urgent speed when a threat is outside. It rests at 960 for about 45 percent of the run and at 640 otherwise. On the Deck, where it cannot widen, only 19 percent of llama attacks are on screen.
- **ROOM-03 (workshop).** Cause and effect (support, falling weight, crushed goblin, landing dust) fit in a 640x360 view because the whole sequence is about 300 px wide; 1.5x adds context (the platform, the pipes) without hiding it. The automatic camera stays at 1.0x.
- **ROOM-04 (hoarder lair).** The scatterers (a 48x32 frame, 15 px of opaque insect) are the smallest threats: 45 px high on screen at 1080p and 1.0x, 30 px at 1.5x, 26 px at 1.75x, and 20 px at 1.5x on the Deck. They stay visible against the floor in the captures but at 1.75x and on the Deck they are a few pixels of contrast; the lair's rubble adds low-contrast clutter. The automatic camera stays at 1.0x.
- **ROOM-05 (gym, separation).** With the default 416 px leash both actors fit a 640 frame (a fixed 1.0x shows both for 100 percent of the ticks; the actors are held, not lost, and the trailing partner catches up). The leash itself is the separation control. With the documented 736 px alternative (Donut on the 240 px platform, Carl on the floor), both actors are fully in view for only 85 percent of the ticks at 1.0x (an actor is less than half visible for 14 percent) and for 100 percent at 1.25x and above; the automatic camera keeps both in view for 100 percent with two rest-width transitions and a mean coverage of 1.16x (table above). The wide leash trades screen coverage for freedom to split up.
- **ROOM-06 (swine boss).** The charge lane is long, so 81 to 93 percent of attack ticks have the boss fully in view in the fixed modes; the automatic camera reaches **89 percent** (it was 81 percent) by framing the charge lane and keeping a large foe next to an actor in the set. The impact on an actor is always in view (the actors are). In the separated capture (the duo 680 px apart, the boss beside Donut) the 960 cap cannot hold both safety regions and the boss: the actors win and the boss's outer edge is cropped by a few pixels. A smaller safety pad for the far actor when a threat needs the room would fix it; not built.

### Frame time and memory

Measured on the real GPU **before this revision's camera rework** (the render path did not change; not re-measured, see the limitations) (an NVIDIA RTX 4060 Ti on the development workstation, Vulkan), vsync off, `--release`, 300 frames after warm-up, scripted 2P runs ([full tables](../evidence/camera-lab/perf_frame_times.md)): the median frame is **1.3 to 1.9 ms at 1920x1080**, **1.7 to 2.2 ms at 3840x2160** and **1.4 to 2.0 ms at 1280x800**, p95 at most 3.0 ms and p99 at most 4.6 ms (isolated spikes up to 11.6 ms at 4K and 15.8 ms once at 1280x800, a single frame). The wider coverage costs little: for the same room the median grows by roughly 0.1 to 0.4 ms from 1.0x to 1.75x, and the automatic camera sits between 1.0x and 1.5x. The dev profile (`cargo build`, opt-level 1) is about 6 ms. Resident memory is 350 to 430 MB. The existing `floor01_scene` corridor (2864 tiles) measures 1.3 to 1.6 ms (1080p) and 1.6 ms (4K) the same way, so the lab rooms (4400 to 8900 sprites) cost about the same on this GPU. **Not a Steam Deck measurement.** Under xvfb, which presents the whole window in software, the same runs read 55 to 65 ms at 1080p independent of the coverage; that is the present path, not the scene, and is not used.

## Limitations

- Evidence is from scripted bots and a handful of captures; **no human playtest, no DualSense or Steam Deck run, no handheld legibility check** and no hands-on game-feel verdict exist. CI success is not that verdict.
- Frame times (below) are from one desktop GPU and were measured **before** the camera rework of this revision (the operator, the framing set and the floor position changed; the render path did not). They were **not re-measured** in this revision: a real-GPU run needs a window on a display, which was not allowed here. Under xvfb the frame time is the software present, so it is not used.
- The rooms use about 4400 to 8900 sprites each (every tile is a sprite, as in `floor01_scene`); the frame-time cost is dominated by that and not by the coverage. A production level would batch or window them.
- 2P hot-join, disconnect and rejoin, and the controller assignment UI, are not implemented. Pads keep a stable slot (first pad player 1, next pad player 2, while connected) but that is a lab binding, not the contract's logical device binding.
- **Post-biscuit Donut is not tested.** All rooms run pre-biscuit Donut (swat and pounce) and Carl without dynamite. ROOMs 04 to 06 belong to later acts where Donut's primary is Magic Missile; the readability of a ranged Donut and her missile at 1.5x, the case most relevant to a wide camera, is untested until the ANIM-DONUT-03 art has runtime support.
- Narrow Deck (1280x800) views have no integer widening, so the Deck auto camera never leaves 640: ROOM-02's llama attacks are 19 percent on screen there (see the results).
- This is a probe PR, not an art PR: no art was made, so the AGENTS.md art pipeline (design, execution and independent review tiers) does not apply.
- Donut's values, enemy timings, the boss charge and the gate cycle are hypotheses; nothing here is canon, topology, balance or an approved camera.

## RECOMMENDATION (a proposal, pending the owner's human game-feel verdict; no ADR-0007 change)

**Proposal only. It records what the evidence supports and what it cannot; the owner's hands-on verdict on a 1080p display and a Steam Deck decides.**

1. **Rest state: 1.0x.** It is the only coverage that is an exact integer scale on every supported output (3x at 1080p, 6x at 4K, 2x on the Deck), it keeps the sprites biggest (Carl 156 px, Donut 120 px at 1080p) and the smallest threat (a scatterer) at 45 px. Short brawls, kicks and hit frames stay at 1.0x: in the three brawl rooms the automatic camera never leaves 640. This is a reasoned default, not a measured preference for the feel.
2. **Adaptive camera that rests only on integer scales, capped at 1.5x (960x540).** The automatic operator now does this and the tests pin it: rest widths per output (1080p 640 and 960; 4K 640, 768 and 960; Deck, 720p and 1440p only 640), 95 percent or more of every run on a rest width, transits of about 20 ticks (10 when urgent) through the in-between widths, nothing above 960. It widens only when it has to: the duo is more than about 420 px apart, a ranged or boss threat does not fit or a llama has acquired an actor, a large foe stands next to an actor; never during hit-stop or a fresh hit; it returns after about 1.6 s calm. In the evidence this removes the real failures: at 1.0x with the wide leash an actor is lost 14 percent of the separation run, and only 21 percent of llama attack ticks are on screen; with the automatic camera (wide leash) the actor is never lost and 93 percent of the llama ticks (fixed 1.5x: 94 percent) and 89 percent of the boss ticks are on screen. 1.5x is the only widened coverage that is an **exact** integer scale at 1080p (2x) and 4K (4x); Carl is 104 px and Donut 80 px at 1080p, 208 and 160 at 4K. At 4K the exact 768 (1.2x, 5x) is an extra rest width.
3. **2P leash: one platform-independent constant, 416 px by default** (same-screen co-op at 1.0x on every output, so a Deck, a 1080p and a 4K machine play the same rules and docking changes nothing). **736 px** (both regions inside the 1.5x cap) stays a documented alternative that an explicit scenario or config flag may select, never the output. The leash is gameplay: the owner decides the value. **Owner playtest (2026-11-19):** both values play well; either is acceptable. The lab keeps 416 px as the default and 736 px behind `--leash wide`. A held partner catches up by walking, never by a jump; that **idle-partner auto-walk is an owner decision** (it moves a human's character without input) and is kept only so nobody is stranded.
4. **1.25x only as a transit step, not a fixed setting.** It is 2.4x at 1080p (and 4.8x at 4K, 1.6x on the Deck): texels alternate between two widths. It is acceptable for the fraction of a second the camera passes through it, but it is not a default.
5. **1.75x stays exploratory.** It is 1.71x at 1080p (1 or 2 px texels), Carl 89 px, Donut 69 px, a scatterer 26 px; with the leash tied to the 960 cap it has no framing job left. Keep it as a debug and screenshot mode, or for one short authored reveal; do not ship it as gameplay without a high-confidence 1080p and Deck playtest.
6. **Steam Deck 1280x800 (and 720p, 1440p): the trade-off.** Only 1.0x is an integer scale there (2x on the Deck, 4x at 1440p); every wider coverage gives uneven texels (50 percent width jitter at 1.333x on the Deck) and a 69 px Carl at 1.5x. The cost of staying at 1.0x, measured: ROOM-02's llama attacks are only **19 percent** on screen. The options, for the owner:
   - **A.** 1.0x only on those outputs (the current default). 19 percent of llama attacks on screen.
   - **B.** The automatic camera uses 960 on the Deck and 1440p (86 percent of llama attacks on screen, from the review's run), with non-integer pixels (1.333x on the Deck). **Needs an ADR-0007 change.**
   - **C.** B as an opt-in accessibility or preference setting, off by default (no ADR change for the default; the setting itself needs approval).
   - **D.** A gameplay rule: ranged foes only fire at a target that is inside a 1.0x frame of the camera's player view (so the threat is always readable at 1.0x). **Unmeasured here and needs gameplay signoff.**

   The reviewer recommends **D + C**: a fair-fire rule so that nothing unreadable ever attacks at 1.0x, plus an opt-in wider view. The probe cannot decide it; it needs a Deck and the owner's call.
7. **Preserve the tall architecture regardless of the policy.** The rooms draw 400 to 700 px of masonry, arch tiers and torches above the action; the void test passes for every position and coverage, and the A1 light budget holds in the widest view at the sampled positions.

**What to look at when playing** (record each as a note, not a pass/fail): does a widened view hurt the weight of the jab, haymaker and kick (ROOM-01, 1.0x against the auto camera); can you read the llama spit and the boss tell in time (ROOM-02, ROOM-06 at 1.5x against 1.0x); are the scatterers findable (ROOM-04 at 1.5x); do zoom steps feel like pumping (ROOM-05 auto, watch the transits and, deliberately, the 1-pixel texel shimmer at 1.25x and 1.75x); does the separation feel recoverable with the leash (ROOM-05, 2P with two pads).

