# F01 room exit door, rooms 2, 4, 4a, 4b and 6 (ROOM-PORTALS-01, design spec)

**Status:** design spec, 2 frozen keys (both frames), export, manifest, validators, the presentation-only wiring and the evidence are done. Independent art-director review passed (silhouette gate included; documentation fix round applied); owner art and game-feel review pending. Evidence: [docs/evidence/f01-room-exit](../evidence/f01-room-exit/README.md).

**Scope.** The exit of the late Floor 1 rooms 2 (the Boiler), 4 (the hazard room), 4a and 4b (the Broken Ring's `ring_court` and `ring_split`) and 6 (the mastery arena). Before this, all five drew the composer's exit marker, the reused Tutorial Guild door (`guild_portal_closed`), which reads as the Guild and never opens, and the `defeat_roster` exit gate of rooms 4 and 6 (`EnvSolidKind::ExitGate`, the sidecar's 64 x 96 `exit`) was an invisible solid until the roster died (only `--debug-collision` showed it). This sheet replaces the Guild door in those five rooms with a room exit whose drop grate is **down while the exit gate stands** and **raised once it opens**. Room 1 keeps the Hoarder's own door (HOARD-02); room 5 keeps its entrance gate ([danger FX spec](floor01-danger-fx-spec.md) section 2.4). Nothing here changes the simulation, a rectangle, a hash or a golden.

**Route.** Difficulty score 2: a new design (1) above about 40 px (1); the grate reuses the approved room 5 gate's construction, so no new moving-part design. Class Medium, route B + silhouette gate. Both keys and the spec were drawn and executed in one design pass (the hero key is the closed frame, the open frame is the same canvas with the grate moved), and the silhouette gate was folded into the independent review, which passed it (adopted default EXIT-AD6). Spec and keys never name who drew them; provenance is in the manifest and the PR.

**Authorities.** Sim state: the exit gate `src/combat_scene/env.rs` (`EnvSolidKind::ExitGate`, `in_room`, the env sync `ExitGate => !cleared` with `cleared = roster_cleared()`, HAZ-Q39), the rooms' sidecars (`assets/levels/floor01/f01_boiler.encounter.json`, `f01_hazards.encounter.json`, `assets/levels/f01_late/mastery.encounter.json`: `exit` = `{1072, 484, 64, 96}`; goals `reach_exit`, `defeat_roster`, `defeat_roster`; `f01_ring_court.encounter.json` `{1968, 484, 64, 96}` and `f01_ring_split.encounter.json` `{1072, 484, 64, 96}`, both `reach_exit`), the Guild door placement `src/camera_lab/layout.rs` (the exit marker, 112 x 136, centred on the exit). Design: [danger FX spec](floor01-danger-fx-spec.md) (the room 5 grate, DFX-AD7/AD8), [art direction](art-direction.md), [antihero tone](../01-design/antihero-tone.md), the [Floor 01 kit palette](../../assets/source/terrain/floor01/floor01_catacomb_kit.json).

**Sources.** `assets/source/props/floor01/dfx_exit.grid.json` (+ `dfx_exit.ase` built from it, `dfx_exit.pixelart.json`, `assets/manifests/f01-dfx-exit.json`), written by [scripts/author_danger_fx.py](../../scripts/author_danger_fx.py) (deterministic; the room 5 grate and this one share one drawing function, `grate`, and the room 5 sheet is byte-identical; `tests/test_art_danger_fx.py` rebuilds every file byte for byte). Frozen keys: `tests/fixtures/f01-dfx-exit_keys.json` (frames 0 and 1).

## 1. Readability rules

1. **Closed reads as a barrier, open reads as a way out.** Closed: cold steel bars fill the whole opening down to the floor, teeth biting the floor row. Open: the grate hangs only 16 rows under the slot (its rail and teeth, the threat that it could drop), and a lamp-lit passage reads through the opening at actor height.
2. **The opening is the collider.** The opening is exactly the sidecar's 64 x 96 exit: in rooms 4 and 6 the `defeat_roster` exit gate's block. A hero stopped by the closed gate stands against the bars; a hero who walks out walks into the drawn opening. Another exit size keeps the Guild door (the art is not stretched).
3. **Grim, practical, no castle.** A plain dressed-stone service doorway with a riveted, rusting drop grate (the room 5 gate's steel, straps, rivets and rust); no portcullis ornaments, no sign, no text, no glow effects of its own.

## 2. Sheet

| Grid | Canvas | Pivot | Tags (frames x ticks) | Frozen keys |
| --- | --- | --- | --- | --- |
| `dfx_exit` | 96 x 136 | (0, 135) | `exit_closed` 1 held, `exit_open` 1 held | 0, 1 |

Canvas columns: jamb 0..15, opening 16..79, jamb 80..95. Rows: cap 0..4, header course 5..18, lintel and keystone 16..39 with the grate's iron slot on 36..39 across the opening, opening 40..135 (96 rows; the bottom row sits on the walking line). The canvas is the Guild door's height (136) and stands centred in its 112 px place. The composer's ground torch at the room's end (`torch_wall`, top 144 px above the walking line) stands behind it with only its flame tip above the cap, where it read as a candle on the door; the presenter does not draw that torch (its light stays, EXIT-AD7). Over the Guild door's arch the same tip showed before.

**Sim state it draws** (`room_exit_art::exit_draw`, presentation only):

| Room | Exit gate | Frame |
| --- | --- | --- |
| 4, 6 (`defeat_roster`) | standing (`!roster_cleared()`) | `exit_closed` |
| 4, 6 | open (the roster is clear) | `exit_open` |
| 2, 4a, 4b (`reach_exit`, no gate) | none | `exit_open` |

**Design.**

- **Doorway.** Warm dressed stone (`stone_02`..`stone_08`, mortar `mortar_02`), each block lit from the top left (lit top row and left column, shaded bottom row and right column, sparse pits from a fixed integer hash); jambs laid in 16-row courses up from the floor, bonded every other course; a projecting cap course, a header course and a lintel whose keystone breaks the header joint; a 1 px `stone_00` outline against the room's brick. The stone separates the exit from the warm brick wall without a new hue.
- **Slot and grooves.** A riveted iron channel (`iron_00`/`iron_02`, two `iron_06` bolts) across the top of the opening, and `iron_00` guide grooves down both inner jamb faces: the grate visibly runs in the doorway.
- **Passage.** Dark brick courses (`brick_00`..`brick_05`) lit warm by one small caged lamp (`brass_*` cage, `flame_02`..`flame_04`) on the right reveal, a shadowed left reveal (`stone_00`/`stone_01`), a lit right reveal, the lintel's soffit, and the passage floor with a lamp spill (`floor_00`..`floor_05`). The lamp sits where the Guild door's grille light was, so the kept light lands on it (EXIT-AD4).
- **Grate.** The room 5 gate's construction drawn 64 px wide: eleven 3 px cold-steel bars (`stone_cold_04` lit, `stone_cold_03`, `stone_cold_01` shade) with 3 px gaps (the passage shows between them), two riveted straps, an 8 px bottom rail and 8 rows of tapering teeth, rust runs from the copper ramp under some rivets, grime toward the floor. Closed: teeth tips on the floor row. Open: hauled up into the slot, rail and teeth on rows 40..55.

## 3. Palette

Named entries of the Floor 01 kit palette, values exact (38 colours, at most 63). Alpha is binary; the canvas is opaque but for the cap's two chamfered top corners (three pixels each), so it covers the Guild door's place.

## 4. Wiring contract (presentation only)

- `room_exit_art::exit_closed(state)` is the env sync's predicate (`any ExitGate solid && !roster_cleared()`), read through shared references; `exit_draw(state, exit, layout_h)` returns the frame and the bottom-left world position (`exit.x - 16`, the walking line). No simulation timer is added, nothing is hashed or mixed. Unit tests step the aligned `mastery:carl_solo:0` and `hazards:carl_solo:0` pilots through their clear with and without drawing: on every tick the drawn frame is `exit_closed` exactly while the world's exit gate block is intact, both frames are seen, and the state hash and event count of the drawn twin equal the plain twin's.
- The Bevy half (`src/bin/floor01_scene/room_exit_art.rs`) does not spawn the Guild door piece in rooms 2, 4, 4a, 4b and 6, nor the end torch it covers (`room_exit_art::hides_piece`: a `torch_wall` inside the canvas's columns with its bottom inside its rows), and spawns one sprite of the atlas with the Guild door's z and render layer in its place. A missing atlas prints `ROOM_EXIT missing ...` and the Guild door stays.

## 5. Acceptance targets

1. Frozen keys byte-identical to `tests/fixtures/f01-dfx-exit_keys.json`; no isolated pixels; binary alpha; palette as section 3 (toolkit validators pass).
2. The opening is 64 x 96 on the exit's columns; closed, every bar's tooth reaches the floor row; open, no grate pixel below row 55; jambs and header identical in both frames (`test_the_exit_opening_is_the_exit_gate_and_the_grate_bites_the_floor`).
3. The drawn frame is the simulation's gate on every tick of a cleared run; drawing never changes the state hash (`room_exit_art::tests`).
4. At 1x in the lit rooms (640 x 360): the closed grate reads as a barrier in front of a passage; the open doorway reads as the way out; the doorway reads as a room exit, not as the Guild door.
5. Every capture's window run ends on its headless hash; the simulation, every pinned hash and every golden are unchanged.
6. No text, logos or third-party designs; no concept traced.

## 6. Adopted defaults

| ID | Default | Why |
| --- | --- | --- |
| EXIT-AD1 | One new doorway sheet instead of stretching the 28 px room 5 grate or the 16 px Hoarder door | Both reused sheets are edge-on lane blockers of another width; integer art never stretches. The grate's design and drawing code are reused as they are |
| EXIT-AD2 | The Boiler and the Ring rooms (no exit gate) show the open frame | Their exit is open from the start (`reach_exit`); a closed grate would lie |
| EXIT-AD3 | The grate snaps from closed to open on the clear tick (no lift animation) | The simulation keeps no opening tick; a render-local memory would not survive a replay seek (the room 5 gate's DFX-AD7) |
| EXIT-AD4 | The Guild door's two light emitters stay (its grille light now falls on the passage lamp, its floor spill on the threshold) | Lighting is presentation only and is derived from the kit at load; keeping it leaves the room's light budget and look unchanged around the exit |
| EXIT-AD5 | Only rooms 2, 4, 4a, 4b and 6 swap the door; room 1 (its own door), room 5 and the gym (their own gates), the CF02 microarena (`--cf02`) and the other composed scenes keep theirs | The audit's scope (V13) plus the Ring rooms wired since; the CF02 arena shares room 6's geometry but has its own exit rule and evidence |
| EXIT-AD6 | Design and execution in one pass; the silhouette gate is folded into the independent review | Orchestrator brief; the grate was already designed |
| EXIT-AD7 | The composer's end torch behind the doorway is not drawn in the replaced rooms; its light emitter stays | Its 8 px flame tip stood on the flat cap like a candle; presentation only, the room's light is unchanged |

## 7. Follow-ups (gaps, not done here)

- A lift animation needs the simulation's opening tick (the same follow-up as the room 5 gate drop).
- No aligned room 4 route shows the closed grate in view (every route reaches the exit's view after the clear); the closed frame of room 4 is the same code path as room 6's and is held by the unit tests.
- The Guild door piece is still the composer's exit marker in the CF02 microarena, the DEMO hall and the camera lab rooms.
- Owner notes from the review: the grate snaps up on the clear tick while the last goblin's defeat presentation still staggers next to the door (about 25 ticks; a sim opening tick and a lift animation would answer it, not a render delay); at 1x the open passage is very dark and the lamp is a speck (lifting the passage's lower third and the floor spill one ramp step, same palette, would strengthen the exit pull).
