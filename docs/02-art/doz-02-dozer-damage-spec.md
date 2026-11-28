# DOZ-02: Dozer damage states and the stop frame, design spec

**Status:** design keys drawn (section 6); silhouette gate **PASS with notes** (section 11); the remaining frames, the export, the evidence and the Bevy wiring are done in the execution step (sections 7 and 13), room 3 is at milestone `setpiece`; independent review pending; owner art and game-feel review after that.

Inputs:

- [DOZ-01 Dozer spec](doz-01-dozer-spec.md): the rig, the anchors, the palette `dozer-floor01-v0` and the frame rules this spec extends.
- [Dozer gameplay sheet](../01-design/enemy-gameplay-dozer.md): the states `impact_stunned` and `disabled`, the stop frame (section 6), the room geometry (section 5) and DZ-AD7 (front impact collider, rear glance).
- [Dozer revenge encounter](../01-design/floor-01-dozer-revenge-encounter.md): the owner's "The Machine Remembers" set piece.
- [Antihero tone](../01-design/antihero-tone.md), [art direction](art-direction.md), [pixel-art pipeline](../07-agents/pixel-art-pipeline.md).

Rights: original pixel work drawn by a deterministic script (`scripts/author_dozer_doz02.py`) on the DOZ-01 parts and palette, published under the owner-reported `ART-PUBLICATION-ALL-V1` scope ([rights and release](../04-production/rights-and-release.md)). No traced, ripped or third-party pixels. The fan-concept names stay in the replaceable layer; the atlas and contract names are IP-neutral (`dozer_*`, `prop_stop_frame*`).

## 1. Routing score

[Routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty):

| Factor | Score | Why |
| --- | --- | --- |
| New design | 1 | The stop frame, its brace, the remnant, the gate, the buffer stop and the tunnel mouth are new; the damage reads must be invented on top of an approved machine |
| Native height above 40 | 1 | The machine is h120 (156 px with the stack) |
| Vehicle or multi-part rig | 1 | Nine rig parts, pitched as columns, riders on seats |
| Special states and VFX | 1 | Damage, pitch, steam leak, glance, fall and burst |

**Score 4 of 4: Hard.** Route: the stricter route of the two the pipeline allows for a Hard asset. The design step draws the hero keys for the readability-critical parts (the jammed wheel, the impact pitch, the disabled hull overlays, the raised frame and the remnant), a separate fresh agent at the design tier runs the silhouette gate on them, the execution step draws everything else to this spec, and an independent design-tier review checks the whole set.

## 2. Canon and tone check

- **What the player must feel.** The disable is the everyman's win: Carl did not out-punch a war machine, he dropped a lump of shop iron on it at the right moment. The machine stops the way a cheap engine stops: crooked, leaking, sulking. No explosion, no heroic pose, no slow-motion.
- **The machine stays the DOZ-01 Dozer.** Same silhouette below the stack collar, same palette, same parts. Damage is added on top, never redrawn: a cracked boiler, a dented cab, a crumpled tub lip, a knocked stack, a sagging yoke, and a tie bar wedged through the wheel.
- **Dead, not lethal.** The DOZ-01 rule made the spike ring the brightest part of the machine (section 8 of DOZ-01) because it kills. The jammed wheel reverses that: the ring goes dark and dull, three spikes are gone, the glints are gone, and the brightest thing on the wheel is the rusty tie bar that stopped it. A stopped wheel reads as scrap, a spinning one as death (DV-7 of the gameplay sheet).
- **Disabled is not a hazard.** No glowing firebox (embers only), no lamp light on the yoke, no hazard stripes on the machine, smoke thin and slow. The hazard stripes belong to the stop frame, its brace and the buffer stop only, so the stripes read "the thing to release", never "the thing that hurts".
- **Crew comedy.** The riders tumble out as goblins thrown from a stalled ride: arms out, a helmet lost, no gore (the Dozer is not killed; GOB-02 death language is not used). The driver stays slumped with his pot over his eyes: humiliated, not dead.
- **Industrial, not heroic.** The stop frame is a riveted press beam on a chain sling, a shop pulley and a brace post with a cleat. Plain iron, rust and a stencilled hazard band. No glowing runes, no magic.
- **References are tone only.** 1980s–1990s action-film junkyard climaxes inform attitude (a machine stopped by improvisation), never any copied design.

## 3. Overlays on the DOZ-01 anchors

Every damaged part replaces the DOZ-01 part one-for-one in the rig of `dozer_rig_h120.json`. The rig origin O, the anchors, the z order and the mirror rule do not change. The contract `assets/source/enemies/dozer/dozer_damage_h120.json` (format `dozer-damage-v0`) records the replacements.

| Rig part | DOZ-01 atlas | DOZ-02 atlas | Tags | Frame, pivot | Anchor (rig px, y up) |
| --- | --- | --- | --- | --- | --- |
| `wheel` | `dozer_wheel_h120` | `dozer_wheel_jammed_h120` | `jammed` | 99×99, (49, 49) | (124, 49), the axle |
| `hull_body` | `dozer_hull_h120` | `dozer_hull_damage_h120` | `pitch_0`, `pitch_1`, `disabled` | 175×160, (87, 159) | (0, 0) |
| `hull_front_cab` | `dozer_front_cab_h120` | `dozer_front_cab_damage_h120` | as the hull | 60×23, (0, 22) | (−75, 85) |
| `hull_front_tub` | `dozer_front_tub_h120` | `dozer_front_tub_damage_h120` | as the hull | 65×86, (0, 81) | (19, 23) |
| `yoke` | `dozer_yoke_h120` | `dozer_yoke_damage_h120` | `bent` | as DOZ-01, (54, 30) | (124, 49), the axle |
| `tread` | `dozer_tread_h120` | unchanged; frame held still | — | — | (0, 0) |
| riders | DOZ-01 rider atlases | DOZ-01 lurch keys, then the tumbles (section 5) | — | — | seats |

The front boxes carry a 2 px (cab) and 4 px (tub, below) padding so that the pitched columns stay inside the canvas; their pivots move with the padding, so the anchors stay the DOZ-01 anchors.

**Pitch rule (impact nose-dip).** The machine is struck at the front and rocks nose-down, then settles. Each hull column at rig x moves up by `round(amp × (2 − 6 × (x + 84) / 167))` px, rounding half away from zero: +2 at the rear (x −84), −4 at the front (x +83) for amp 1. The amps are `pitch_0` 1.0, `pitch_1` 0.5, `disabled` 0.0. The frames are integer column translations of the damaged hull (no resampling, every pixel kept), so 1 px steps show along the deck bottom; that is accepted (design note N-1). The tread, the wheel and the yoke stay on the floor. The riders' seats move with the columns under them: `pitch_0` engineer +1, rider A −2, rider B −3; `pitch_1` 0, −1, −2.

**Emitters.** `steam_leak` at rig (6, 91), the boiler crack; `stack_crooked` at rig (8, 144), the knocked stack's new mouth. The DOZ-01 stack and exhaust emitters are off in `disabled`.

## 4. Frame and state table

Ticks are at 60 Hz. "Key" means drawn in this design step; "Exec" means drawn in the execution step to this spec.

### 4.1 The machine

| State | Part | Frames | Ticks | Who | Notes |
| --- | --- | --- | --- | --- | --- |
| `impact_stunned` (48 ticks) | hull, cab, tub | `pitch_0` → `pitch_1` → `disabled` | 0–5, 6–11, 12–47 | Key | Hit spark `vfx_hit_spark_metal` at the tub lip on tick 0 |
| | wheel | `jammed` | from tick 0 | Key | No spin, no smear, held still |
| | yoke | `bent` | from tick 0 | Key | Lamp dark |
| | tread | DOZ-01 frame shown at the hit | held | — | |
| | steam | 3-puff burst every 6 ticks from `steam_leak` | 0–47 | DOZ-01 smoke | |
| | stack | one black puff on tick 0, then none | 0 | DOZ-01 smoke | From `stack_crooked` |
| | riders | DOZ-01 `lurch_forward` with the seat offsets, then the tumbles from tick 12 | 0–11, 12–35 | Exec (tumbles) | Section 5 |
| `disabled` (terminal) | hull, cab, tub | `disabled` | held | Key | |
| | wheel, yoke | `jammed`, `bent` | held | Key | |
| | steam | one `steam_0..2` puff every 40 ticks | loop | DOZ-01 smoke | Thin and slow |
| | stack | one `black_3..5` wisp every 60 ticks | loop | DOZ-01 smoke | |
| | driver | `engineer_slump` (pot over the eyes), 2 frames of breathing | 30 + 30 ticks | Exec | Section 5 |

### 4.2 The stop frame

The prop is the Boiler's `weight_rig` as data (states `Supported → Weakened → Released → Falling → Landed`). Its hanging mass is 80 px wide; the room's footprint is x 640..720.

| Prop state | Atlas, tag | Frame, pivot | Ticks | Who | Notes |
| --- | --- | --- | --- | --- | --- |
| `Supported`, `Weakened` | `prop_stop_frame`, `raised` | 96×60, (8, 59) | held | Key | The beam on a V sling from a hook; the sling's chain runs up to the pulley |
| every state | `prop_stop_frame`, `pulley` | 96×60, at the raised anchor | held | Key | Sheave, housing and the haul chain leaving right |
| `Released` (30 tick warning) | `prop_stop_frame`, `warn_0`, `warn_1` | 96×60, (8, 59) | alternate every 3 ticks | Exec | The raised key shifted 1 px left, then 1 px right; dust motes (2–3 px, `5`/`4`) drop from the lugs on ticks 0, 10 and 20 |
| `Released` | floor shadow `prop_stop_frame_shadow` | 80×4 | grows over the 30 ticks: 40 → 80 px wide, 1 → 4 px tall, alpha 0.3 → 0.6 | Exec | A dark ellipse in `K`, centred on the footprint; the telegraph the AI also reads |
| `Falling` (8 ticks) | `prop_stop_frame`, `falling` | 96×60 | 8 ticks | Exec | The raised key with the sling slack: both sling legs bow 2 px outward, the hook chain hangs in a loose S; a 2-frame vertical motion streak (`3`, 1 px wide, 6 px long) above the lugs |
| `Landed` (miss or rear glance end) | `prop_stop_frame_remnant`, `landed` | 80×16, (0, 15) | held | Key | The bowed beam lies in the lane, 16 px tall (the step-over limit) |
| `Landed` after a hit | `prop_stop_frame_remnant`, `landed_hit` | 80×16, (0, 15) | held | Key | The tie bar is gone from the beam (it is in the wheel) |
| Landing tick | dust burst | DOZ-01 smoke `steam_0..2` in `5`, 4 puffs | 0–12 | Exec | At both beam ends |
| Rear glance (late miss) | `prop_stop_frame_glance`, `glance_0..2` | 88×88, pivot at the beam centre | 4 + 4 + 6 ticks | Exec | Section 4.3 |

### 4.3 The rear glance (DZ-AD7)

A late landing meets only the rear hull. The frame strikes the rear deck and tips off behind the exhaust; it is a miss. Three frames in an 88×88 atlas, the beam rotated about its centre in 15-degree steps (0, −15, −35 degrees: the left end down, toward the rear), plus one shadow frame:

- `glance_0` (4 ticks): the raised key's beam, unrotated, 2 px lower, a hit spark `vfx_hit_spark_metal` at the contact on the deck.
- `glance_1` (4 ticks): rotated −15, sliding 6 px toward the rear.
- `glance_2` (6 ticks): rotated −35, 10 px further back and 8 px down, then the remnant `landed` drops in behind the machine.
- The rotation is redrawn by hand per frame on the 80 px beam (not resampled); the rivet row stays a clean 1 px line.
- The hull shows **no** damage overlay: the machine keeps chasing, so the hull, wheel and yoke stay the DOZ-01 live parts.

### 4.4 The brace

| Prop state | Atlas, tag | Frame, pivot | Who | Notes |
| --- | --- | --- | --- | --- |
| `Supported` | `prop_stop_frame_brace`, `intact` | 16×64, (0, 63) | Key | Iron plate, copper post, cleat horn, the haul chain wrapped and running up |
| `Weakened` (one hit of two) | `prop_stop_frame_brace`, `weakened` | 16×64 | Exec | The horn bent 2 px down, one wrap of chain slipped (the wrap rows move 2 px down), a crack in `K` across the post 3 px long |
| `Released` onward | `prop_stop_frame_brace`, `snapped` | 16×64 | Exec | The horn broken off (lying at the plate foot, 4×3 px), the chain end hanging free and swinging 1 px over 2 frames every 8 ticks for the first 48 ticks, then still |

### 4.5 The room pieces

| Piece | Atlas, tags | Size | Who | Notes |
| --- | --- | --- | --- | --- |
| Rear gate | `env_dozer_gate`, `closed`, `shake_0`, `shake_1`, `burst` | tiles 32×32, stacked to the lane ceiling (228) | Exec | Riveted iron doors in `IRON` with `RUST` streaks and a cross bar; `shake_*` move the doors ±1 px and drop dust motes; `burst` shows the doors bent outward and the bar snapped; presentation only, no solid |
| Buffer stop | `env_dozer_buffer`, `idle` | 16×24 (the bottom 16 px are the block, the top 8 px a warning post) | Exec | A timber block on an iron foot with an S/0 hazard band; drawn only, never a solid (DZ-AD4) |
| Tunnel mouth and lintel | `env_dozer_lintel`, `mouth`, `fill` | tiles 32×32 | Exec | Stone lintel face at 112 px clear, a timber header with rivets, the fill above it to the ceiling; the mouth's inner shadow 8 px deep in `K` |
| Haul line | `env_dozer_haul`, `h`, `v`, `idler` | 8×8 tiles, idler 12×12 | Exec | The chain from the pulley stub to an idler above the brace, then down to the cleat; built from `CHAIN_V`/`CHAIN_LINE` in the authoring script (unoutlined) |

### 4.6 The crew (section 5)

| Actor | Tags | Frames | Ticks | Who |
| --- | --- | --- | --- | --- |
| rider A | `tumble_0..2` | 3 | 6 + 6 + 12 | Exec |
| rider B | `tumble_0..2` | 3 | 6 + 6 + 12, 4 ticks after A | Exec |
| engineer (driver) | `engineer_slump_0`, `engineer_slump_1` | 2 | 30 + 30, loop | Exec |

## 5. The crew keys

- **Tumbles.** Built from the GOB-01 goblin pixels already used for the DOZ-01 riders (no new character design). `tumble_0`: thrown forward out of the seat, arms out, the head tilted back 1 px. `tumble_1`: rotated about 90 degrees mid-air, helmet off (a 4×3 px helmet follows on its own arc). `tumble_2`: landed on the back on the lane floor in front of the tub, then the gameplay's ordinary goblin get-up takes over. The arc: up 12 px and forward 28 px (A) and 36 px (B) over 12 ticks, down to the floor over 12. No blood, no gibs; the riders are alive and about to fight.
- **The driver's slump.** The DOZ-01 `engineer_hold` key with the head dropped 2 px forward, the pot (`pot_px` of DOZ-01) tilted down over the eyes so no eye pixel shows, the arms off the levers and hanging. `slump_1` lifts the chest 1 px (breathing), so he reads as alive and dazed.

## 6. Hero keys (drawn in this step)

All keys are committed grid sources made by `scripts/author_dozer_doz02.py` (deterministic; `--check` verifies them), each with a `.ase`, a `.pixelart.json` validator config and a manifest under `assets/manifests/`.

| Key | Stem | What makes it read |
| --- | --- | --- |
| Jammed wheel | `dozer_wheel_jammed_h120` | The spike ring dulled to L* 0.30 (live 0.58), no `6`/`7` glints, rust on five spikes, three spikes broken (two bent stubs, one snapped), a rusty tie bar driven diagonally through the spokes with a shadow under it |
| Impact pitch | `dozer_hull_damage_h120`, `dozer_front_cab_damage_h120`, `dozer_front_tub_damage_h120` | `pitch_0`, `pitch_1` by the pitch rule |
| Disabled hull overlays | the same three, `disabled` | A boiler crack with the leak at its foot, a sprung rivet, a dented cab, the stack knocked crooked and leaning, the firebox down to embers, the tub lip crumpled 3 px with a crease |
| Bent yoke | `dozer_yoke_damage_h120` | The yoke sags 3 px in a V, a crease at the kink, the lamp dark with a glass shard; the axle cap is unchanged |
| Raised frame | `prop_stop_frame`, `raised` and `pulley` | An 80 px press beam (flanges, web, rivets, hazard stencils at both ends), a V sling to a hook, the chain up to the pulley |
| Remnant | `prop_stop_frame_remnant`, `landed`, `landed_hit` | The beam bowed 5 px at a kink, 16 px tall, lying flat |
| Brace, intact | `prop_stop_frame_brace`, `intact` | The release target: a hazard band, the chain wrap and the horn |

## 7. The execution step (summary; done, see section 13)

The remaining frames of section 4 (warn, falling, shadow, glance, brace weakened and snapped, the gate, the buffer, the lintel, the haul tiles, the tumbles, the slump), the export, the validators and tests, the evidence (a contact sheet, enlarged frames, the reversal and the glance at gameplay scale, a frame and state diagram), the Bevy wiring that replaces each labelled stand-in of `src/dozer_art.rs`, and the f01_late manifest's `dozer` milestone flip from `greybox` to `setpiece` with its `MANIFEST_HASH` re-pin. New frames extend the authoring script (one generator for every DOZ-02 source); the design keys of section 6 do not change without a design decision.

## 8. Palette

`dozer-floor01-v0` (DOZ-01 section 4), no new colours. The jammed wheel and the remnant use only `IRON`, `RUST`, `COPPER` and `BRASS` ramps. Hazard stencils are stripes of the DOZ-01 keys `S` (brass_4) on `0` (iron_0) and appear only on the stop frame, its brace and the buffer stop. The fire keys `j`, `k`, `n`, `q`, `z` are forbidden on every disabled part, except that the hull's firebox keeps its ember pixels `j` and `k`.

## 9. Validators and tests

- `python3 -m scripts.pixelart validate <stem>.pixelart.json`: every DOZ-02 source PASSes. Per-asset configs exist because a frame belongs to exactly one tag (no range tags; the runtime asks for `pitch_0` and `pitch_1` by name) and because the hull carries 28 colours (adopted limit 30, as the DOZ-01 hull already needs 29).
- `tests/test_art_dozer_doz02.py` (unit tests, run by the art workflow): regeneration, `--check`, no DOZ-01 path overlap, `.ase` present, manifests match the grids, palette, lint (orphans ≤ 9 per frame), validator configs, and the acceptance targets of section 10.
- New execution frames add tests to the same file; the thresholds below do not loosen.

## 10. Acceptance targets

| Target | Threshold (tested) |
| --- | --- |
| Pitch end points | +2 at x −84, −4 at x +83 (amp 1); −2 at the front for amp 0.5; 0 for amp 0 |
| Pitch frames | integer column translations of the `disabled` frame; pixel count kept |
| Hull silhouette | unchanged below the stack collar; the damage differs from DOZ-01 only near the crack, the dent and the stack |
| Firebox | embers only: none of the bright fire keys `n`, `q`, `z` in any hull damage frame |
| Yoke | none of the fire keys `j`, `k`, `n`, `q`, `z`; axle cap identical to DOZ-01; sag exactly 3 px |
| Tub | lip crumpled exactly 3 px |
| Jammed wheel | no `6`/`7` pixel; ring L* ≤ 0.35 and at least 0.20 below the live ring; the ring no longer brighter than every part + 0.1; the tie bar brighter than the spikes; the floor contact row kept; 3 spike tips gone |
| Stop frame | the mass fills its rectangle (columns 8..87, bottom row 59); the sling reaches the block; the haul chain leaves the canvas |
| Remnant | columns 0..79, bottom row 15, at most 16 px tall |
| Brace | the chain reaches the top and the bottom wrap |
| Contract | the 48-tick schedule is covered exactly; the seat offsets follow the pitch; the front anchors are the DOZ-01 anchors |
| Execution frames | warn frames differ from `raised` by a 1 px shift only; `falling` keeps the mass rectangle; the glance never overlaps the wheel's lethal circle; the gate, buffer and lintel use only the dozer palette; the slump shows no eye pixel |

## 11. Readability rules and the silhouette gate

Rules every frame must keep:

1. The jammed wheel is darker than the hull's lit iron and never the brightest thing on screen; a lethal wheel always is.
2. A disabled part shows no light source: no lamp, no fire glow, no glints.
3. Hazard stripes mark only the release mechanism (frame, brace) and the buffer stop.
4. The remnant is no taller than 16 px and reads as a fallen beam, not a step or a platform.
5. The crew tumble and the slump read as comic and alive.

**Silhouette gate (PASS with notes).** A fresh design-tier agent who did not draw the keys answers three questions from the gate pack (`docs/evidence/doz-02/`: `doz02_lit_before_640x360.png`, `doz02_lit_after_640x360.png` at 1x, `doz02_key_sheet_x4.png`, and this spec):

1. Does the disabled machine read as the DOZ-01 Dozer, broken and harmless?
2. Does the jammed wheel read as stopped and not lethal?
3. Does the remnant read as a fallen press frame?

Any "no" sends the keys back to design before execution starts. The verdict is recorded in `docs/evidence/doz-02/README.md` and here.

**Verdict: PASS with notes**, by an independent Claude Opus agent who did not draw the keys (judged at 1x first, then enlarged). 1: PASS, the DOZ-01 silhouette holds and the stack, steam, dark lamp and barred wheel read as broken and harmless; the crack, dent and tub crumple are faint at 1x. 2: PASS, the ring drops from the brightest object to a dull star and the tie bar explains the stop. 3: PASS, the bowed rust beam with the end stencils reads as the frame that fell, not a step. No blocking design defect; execution may start. The non-blocking notes G-1 to G-5 (quiet hull damage at 1x, the hub, the remnant's contact shadow, a wall-torch flame and a point light composited onto the disabled machine in the after scene, the alert stand-in driver) are listed in the evidence README and carry into the execution step.

## 12. Adopted defaults

| ID | Default | Why |
| --- | --- | --- |
| DZ2-AD1 | Separate damage atlases per part, replacing the live part one-for-one at the same anchor | Keeps the DOZ-01 atlases byte-identical; the wiring swaps a handle per part |
| DZ2-AD2 | The pitch rule of section 3, 3 hull frames (`pitch_0`, `pitch_1`, `disabled`) at 6/6/held ticks | A nose-dip sells the front hit in 12 ticks without a rotation; integer columns stay crisp |
| DZ2-AD3 | No range tags; one tag per frame | The validator rejects a frame in two tags; the runtime reads frames by name |
| DZ2-AD4 | Hull validator limit 30 colours | The damage adds no colour outside `dozer-floor01-v0`; the DOZ-01 hull already needs 29 |
| DZ2-AD5 | The frame hangs on a V sling from a hook; the chains are unoutlined | An outlined spreader read as a table and outlined chains as dark posts in the first pass |
| DZ2-AD6 | The haul line runs from the pulley along the ceiling to an idler above the brace, then down to the cleat | Shows the cause and effect: hit the brace, the chain lets go, the frame falls |
| DZ2-AD7 | The 48-tick `impact_stunned` schedule and the `disabled` emitter rates of section 4.1 | Short enough to keep the melee window, slow enough that disabled reads as dead |
| DZ2-AD8 | The tie bar of the frame ends up in the wheel (`landed_hit` remnant has no bar) | One object explains the jam |
| DZ2-AD9 | The riders' tumble arc ends at the column where the simulation's goblin lands on the disable tick (the wheel front plus `dzq20_land_dx`: A 32, B 72), not at the design's 28 / 36 px of forward travel; 12 px high, 24 ticks airborne, the poses at 6 and 24 ticks, the lying pose held until the goblin takes over (see DZ2-Q1) | The design's forward travel ends on top of the wheel; the simulation's landing is past its front, and a hand-off between them would jump 100 px |
| DZ2-AD10 | The haul tiles share a 12 x 12 atlas cell (8 x 8 of chain content, the idler fills it) and the validator allows 12 orphans per frame for them only | An atlas needs one frame size; the unoutlined chain pattern leaves 6 single-key pixels per link repeat |
| DZ2-AD11 | The gate, lintel mouth and fill tiles carry a few transparent pixels (chipped jambs, a chipped stone, a feathered shadow edge); the glance beam is redrawn column by column; the shadow frames are explicit row widths | The asset validator requires every frame to hold transparent and opaque pixels; the squeeze of a column turn drops a column here and there (a rivet column never) |

## 13. Design notes

- **N-1.** The pitch shows 1 px stair steps along the deck bottom. Accepted: integer column translation, no resampling.
- **N-2.** The DOZ-01 wheel and hull also fail the default validator config (frame in two tags, colour count); this is pre-existing and not changed here.
- **N-3.** The haul line in the lit compositions is drawn by the review script; the tile set is an execution item (section 4.5).
- **N-4.** The frame hangs 168 px above the floor (the gameplay sheet's hang height). In the room's 228 px lane the sling and pulley have 60 px; in a taller corridor the haul line must be extended, not the sling.
- **N-5.** In the lit compositions the driver in `disabled` is the DOZ-01 `engineer_hold` key, a stand-in until the slump frames exist. **Closed in execution:** `engineer_slump_0/1` exist and the lit after scene uses them.

### Execution notes (Claude Sonnet) and questions for the designer

Done: every frame of section 4 (warn, falling, the shadow, the glance, the brace states, the gate, the buffer stop, the lintel, the haul tiles, the tumbles, the slump), the contract fields (`stop_frame.warning`, `falling`, `glance`, `brace.swing`, `room_pieces`, `crew`), the export, the tests (`tests/test_art_dozer_doz02.py`, `src/dozer_art/tests.rs`), the evidence, the wiring in `src/dozer_art.rs` and the room 3 milestone flip to `setpiece` (`MANIFEST_HASH` re-pinned: the manifest file changed, nothing else moved).

- **Gate notes.** G-4: the lit scenes are restaged so no light lands on the cab, the stack, the yoke or the bar (the staging comment in the review script lists the torches and the distant lamps); the Bevy wiring draws the room's own lighting, nothing is added to the machine. G-3: the remnant and the landing dust keep a tight contact (the dust puffs sit 6 px up at the beam ends). G-5: the slump frames exist and read as alive and dazed (pot over the eyes, no eye pixel, a 1 px breath). G-1 and G-2 stay as the gate left them (quiet hull damage at 1x; the hub). The cracks still read faintly at 1x: the gate's cheap levers (a second steam puff or a darker crack outline) were not used.
- **DZ2-Q1 (tumble reach).** The design's forward travel (28 px for A, 36 for B) lands the riders on top of the wheel (the wheel spans rig x 76..172, the seats are at 38 and 66), while the simulation lands the dismounted goblin 32 and 72 px past the wheel front on the disable tick. Adopted DZ2-AD9: the arc keeps the design's height, duration and poses but ends at the simulation's landing. Designer: confirm, or move the landing data.
- **DZ2-Q2 (glance continuity), closed by the review fix.** The presenter first dropped the falling frame in front of the hull to the floor and then popped it onto the deck at the strike. Now a fall that the pure `Dozer::glance` predicate says will glance off the rear hull (the footprint meets the hull but not the front collider) ends on the rear deck and slides to the strike's start (test `a_glancing_fall_ends_on_the_deck`). There is still no Bevy window capture of the glance: only a blast releases the frame late enough, no pilot can throw one, and Carl at the brace would be run over; the composed `doz02_glance_*.gif` shows the intended sequence.
- **DZ2-Q3 (the lost helmet).** Section 5 asks for a 4 x 3 px helmet on its own arc, but riders A and B wear none (B has a headband, the pot is the driver's). Skipped: the pineapple that the GOB-01 poses already throw is the loose object.
- **DZ2-Q4 (tunnel lintel), closed by the review fix.** The room's lintel is a solid that nothing drew (the kit draws solids only under `--debug-collision`), so the execution step first left `env_dozer_lintel` unwired on a wrong claim. The review's fix wires it: `mouth` tiles along the 112 px underside and `fill` tiles up to the ceiling, over x 1040..1344, behind the Guild door piece. The door's cap and arch rise about 24 px past the 112 px clear height and overlap the lintel; the door draws in front, which reads as the door set into the tunnel mouth. Designer: confirm or ask for a clipped door.
- **Tumble frames** are the GOB-01 `hurt`, `knockback` (mirrored) and `down` poses whole (no rotation, no new pixels); the two riders overlap for about 15 ticks of the arc (A overtakes B), which reads as a tangle of thrown goblins.
- **Tumble timing, adopted default (DZ2-AD9, review F8).** The tumble poses are the GOB-01 `hurt` (6 ticks), the mirrored `knockback` (a flying pose held 18 ticks, ticks 6 to 24 of the arc, standing in for the "rotated about 90 degrees mid-air" pose) and `down` (held from tick 24), not the 6 + 6 + 12 of section 4.6; there is no separate rotated frame because the GOB-01 pixels are used whole.
- The rider landing in the presenter is the unnudged `front + land_dx`; when a hero stands on the spot the simulation's goblin lands up to 64 px further right, so the lying rider can pop that far on the disable tick.

## 14. Files

| File | Role |
| --- | --- |
| `scripts/author_dozer_doz02.py` | The generator for every DOZ-02 source and the contract |
| `assets/source/enemies/dozer/dozer_*_damage_h120.*`, `dozer_wheel_jammed_h120.*` | Machine overlays: grid, `.ase`, validator config |
| `assets/source/props/stop_frame/prop_stop_frame*.*` | The stop frame, the remnant and the brace |
| `assets/source/enemies/dozer/dozer_damage_h120.json` | The damage contract (`dozer-damage-v0`) |
| `assets/manifests/dozer_*` and `prop_stop_frame*.json` | Provenance manifests |
| `scripts/make_dozer_doz02_review.py` | The evidence renders (contact sheets, frames at x6, the key sheet, the lit scenes, the state diagram, the glance GIFs, the reversal GIFs from the Bevy frames, `SHA256SUMS`) |
| `scripts/make_dozer_doz02_evidence.sh` | The Bevy captures: the damage states of the `dozer:brace`, `dozer:retry` and `dozer:crew` pilots at 640 x 360 and 640 x 400, twice, hash-matched, and the reversal frames |
| `src/dozer_art.rs`, `src/dozer_art/tests.rs`, `src/bin/floor01_scene/dozer_art.rs` | The presenter: the damage states, the crew, the room pieces and the stop frame as atlas sprites |
| `assets/source/props/dozer_room/` | The gate, buffer stop, lintel and haul sources |
| `tests/test_art_dozer_doz02.py` | The tests |
| `docs/evidence/doz-02/` | Contact sheets, the key sheet, the lit before and after scenes, the gate verdict |

## 15. Follow-ups: DOZ-02-FU F4 to F7

Defined here on 2026-11-28 from the "Follow-up for the designer: DOZ-02-FU" section of the body of PR #291 (DOZ-02), where they were first written; until then three docs cited them and none defined them. The names F1 to F3 and F8 to F11 in that PR are review fixes, not follow-ups. All four are open, not started, owner art review pending; the group is called "faint hull damage at 1x" in the backlog although F4, F6 and F7 are about staging, layering and tiling.

| ID | Follow-up | Cheap levers named in #291 |
| --- | --- | --- |
| F4 | A wall-torch flame sits behind and above the disabled tub in the real room and reads as fire on the wreck at 1x (the G-4 issue again; the disable position is fixed). | Move that sconce, or accept it. |
| F5 | The hull crack, cab dent and tub crumple stay faint at 1x (gate note G-1). | A darker crack core, a 1 px light rim on the crumple, a missing plate. |
| F6 | The `landed_hit` remnant lies under the jammed wheel at `Z_BACK` and is about 70 % hidden (the gate judged it in front of the wheel). | Raise it above the wheel, or shift it by the wheel radius. |
| F7 | The 32 px gate tile repeats the bar and lock, so seven tiles read as a ladder and the burst stack repeats stubs. | A bar-free upper tile variant. |

Cited by [AREA-POLISH-01](area-polish-01-spec.md) (out of scope, POLISH-Q5) and the backlog. Any of them is a medium-tier art order after a top-tier note; none has an owner decision yet.
