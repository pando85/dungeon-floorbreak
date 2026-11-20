# COMBAT-LAB-01 rooms R00 to R02: layout keys and spec (LAB-01)

**Status:** design spec for LAB-01, top-tier level design (2026-11-20). **Implemented by LAB-01 (PR #150), not game-feel accepted.** The JSON, the headless asserts of section 9.1 and the live entry exist; see section 13 for what the implementation measured.
**Parent contract:** [COMBAT-LAB-01 advanced playtest](combat-lab-01-advanced-playtest.md) (rooms table, section 4 geometry contract, LAB-Q01..Q15, hard gate 1), [work order](../07-agents/combat-lab-01-workorder.md), [playtest protocol](../04-production/combat-lab-01-playtest-protocol.md).
**Owner directive (2026-11-20, speed):** `--scene combat_lab` is wired early on the **legacy rules**; `cf02_rules` variants come later. R00 to R02 must be fully playable and fun on the legacy rules with the existing goblins and the Bad Llama, in solo Carl, solo Donut and P1+P2 co-op, and must gain value later from the CF02 moves without a geometry change.
**Horizon check:** compared with [character-progression-horizon](character-progression-horizon.md). The rooms use only the Floor 01 post-biscuit kit (LAB-Q01). The ledges, the impact wall and the spit refuge are the hooks for later Down/stomp, wall slam (CF02-B) and Donut's cat jump; nothing here commits the horizon. Fits.

---

## 1. Ground truth used by this spec (legacy rules, main)

All numbers below are read from main. A medium-tier agent must re-check a cited line if the file has moved, but must not re-decide a value.

### 1.1 Room format (`.room.json`, schema `floor01-demo-hall-v1`)

| Fact | Source |
|---|---|
| Units are layout pixels, origin top-left, y down. `floor` snaps to `16n+4`; floor 580 gives `snap_dy = 0`. | `src/camera_lab/room.rs:256-258` |
| Platform JSON uses `x`, `y` (top surface) and `w`; `h` is ignored. `x`/`w` round to 16, `w` is at least 32, the rise rounds to a whole tile. Every platform is a 2-tile slab. | `src/camera_lab/room.rs:263-276` |
| The slab's collider runs from the top down 28 px (deck tile collider plus one face tile), so a rise-32 slab is solid at heights 4..32 and a rise-64 slab at 36..64. Below that is open air. | `src/camera_lab/layout.rs:392-422` |
| Access steps are generated (not authored) for every platform with rise at most 200: `STEP_W` 32, `STEP_RISE` 32, count `(rise-1)/32`, step k has height `rise-32k`, left side first, then right; a staircase is dropped if it overlaps a platform, a prop or the play bounds. Steps are full masonry from the floor. | `src/camera_lab/room.rs:30-34, 397, 405` |
| End walls are 16 px (`WALL_PX`); the playable x range is `16..width-16`. | `src/camera_lab/room.rs:28` |
| Render height is `max(ceil(height/16)*16, floor+64)`: height 660 gives 672. | `src/camera_lab/room.rs:261` |
| Exit rect is `(goal_x-24, floor-96, 64, 96)`, clamped to `play_x.1-64`. The closed guild portal art (112x136) is drawn at `exit.x-24` on the floor line, so the floor under it must stay clear. | `src/camera_lab/layout.rs:495-498` |
| `enemies` entries take `kind` (`goblin`, `llama`, ...) and `x`; `camera_intro` is not read by the parser (kept only for parity with the demo file). | `src/camera_lab/room.rs:70-96, 320-344` |
| `build_kit` composes the EnvKit; `walking_line_at(x)` returns the standing line under a column. | `src/camera_lab/layout.rs:636`, `src/env_kit.rs:558` |
| Combat enemies are Rust fixtures (`EnemySpawn{x, ai}`, `LlamaSpawn{x, ai}`) that must stand on Carl's walking line (checked when `CombatState` loads). | `src/combat_scene.rs:57-71, 862` |

### 1.2 Hero movement (Carl and Donut are identical on the legacy rules)

| Value | Source |
|---|---|
| `HERO_HALF_WIDTH` 12, `HERO_HALF_HEIGHT` 26 (body 24x52), `RUN_SPEED` 185, `JUMP_SPEED` 440, `GRAVITY` 1050, jump buffer 7 ticks, coyote 6 ticks | `src/prototype.rs:13-20` |
| Integration: no variable jump height; discrete apex 88.54 px at tick 25; a flat jump is 50 ticks and 154 px long | `src/prototype.rs:402` (`step_hero_driven`) |
| Landing windows from the floor: rise 16/32/48/64/80 is clearable while the feet are above it on ticks 3-46 / 5-44 / 8-41 / 12-37 / 17-32; rise 96 is unreachable | derived from the two rows above |
| Donut's body steps through the same `step_hero_driven` and the same 24x52 collider | `src/corridor.rs:236-262` |
| The companion nav model confirms Donut uses Carl's jump in the integrated game (the camera-lab Donut jump 600 is lab-only) | `src/companion/nav.rs:14-17, 282-288` |

**Consequence:** on the legacy rules there is **no surface Donut can reach that Carl cannot**. See LAB-R-Q09 and open question 1.

### 1.3 Moves that matter for the geometry

| Move | Box or effect | Source |
|---|---|---|
| Carl jab | box 16..22 ahead, 29..35 above his feet | `src/combat.rs:224` |
| Carl haymaker | 16..22 ahead, 27..38 above his feet | `src/combat.rs:252` |
| Carl push kick | 15..22 ahead, 18..28 above his feet, launches a goblin about 48 px (about 75 px on a lethal knock) | `src/combat.rs:277` |
| Donut Pounce | launch vx 180, vy 130, at most 20 airborne ticks, starts only when grounded (a ledge counts); box 11..25 ahead, 0..17 above her feet | `src/donut_moves.rs:115-123`, `src/combat.rs:328` |
| Donut Magic Missile | bolt spawns 27 ahead and 19 up, 8x6 box (16..22 above her feet), 6 px/tick, 240 px range, mana 8, cost 1 | `src/combat_scene/donut.rs:39-46`, `assets/data/gameplay/companion_magic_v0.json` |
| Donut hurtbox | Forgiving, 20x24 from her feet | `src/party.rs:102-120` |

### 1.4 Enemies

| Value | Source |
|---|---|
| Goblin: walk 62 px/s, sight 220 (x distance only), trigger 30 (x only), stand-off 60, tell 24 ticks, swing box 8..30 ahead and 0..30 above **its own** feet, at most 2 attackers | `src/feel.rs:168-195` |
| Goblins keep a fixed `floor_y`, never climb, and stop at any solid; all enemies start facing -1 (west) | `src/goblin.rs` |
| Llama: 3 HP, walk 40, stand-off 200, leash 160 from home, tell 36, attack 60, cooldown 90, kick push about 30 px | `src/feel.rs:223-241` |
| Glob: 150 px/s, spawns 46 ahead and 45 up, 8x6 box (42..48 above the llama's feet), range 320, stopped by any solid; fair fire needs the llama's whole body in the camera view and Carl within 320 in its facing direction | `src/feel.rs:245-254` |
| The llama aims at **Carl's x** (`scene.world.hero`), never at Donut | `src/combat_scene.rs:1773` (`step_llamas`) |
| Globs hit Carl, and Donut when the party is live | `src/combat_scene.rs:1827` (`step_globs`) |
| Respawn returns living goblins to their spawns, keeps the dead dead, does **not** reset the llama | `src/combat_scene.rs:960` |
| A room is complete when every goblin and llama is defeated | `src/demo.rs:300` |

### 1.5 Party and camera

| Value | Source |
|---|---|
| Separation wall: `max_separation_px(640)` = 640 - 2x96 = **448 px** | `src/party.rs:218`, applied in `src/corridor.rs:866` |
| Party tuning: follow 40, dodge radius 40, regroup after 120 off-screen ticks | `src/party.rs:226-251` |
| Bot regroup places the bot at the view edge on the leader's foot line | `src/corridor.rs:817-833` |
| Follower bot: jump reach = apex - 4 (about 88.2 px); obstacle probe 6x8 px at feet+1..+9 ahead; hops onto the leader's ledge within 64 px | `src/follower.rs:25-40, 111, 174` |
| **Bot trap rule:** a slab whose underside is more than 9 px above the bot's feet is invisible to the probe; the bot walks into it and stalls until regroup | `src/follower.rs:174-182` |
| Scroll views: 640x360 with the walking line 288 px below the top, 640x400 with it 308 px below | `src/demo.rs:645` |
| HUD clearance: gameplay must stay at screen y >= 60, so a hero's head may be at most **228 px** above the walking line at 360 (248 at 400) | `src/combat_scene.rs:5050` |
| Auto camera (`--demo`): width capped at 960 and never wider than the room bounds; the view is clamped inside the room, never shows void | `src/camera_lab/camera_op.rs:16-17, 226, 460-473` |

---

## 2. Rules every lab room follows

1. `schema` `floor01-demo-hall-v1`, `width` a multiple of 16, `height` 660, `floor` 580, `setting` `dry-catacomb`, `act` 1. Render size is `(width, 672)`.
2. Only the existing format: no props, `hazards` empty, `solid_backwall` empty (LAB-R-Q02).
3. Every slab has rise **32 or 64** only. Every surface is at most 64 above the floor, so the highest head (64 + 52 + 88.5 = 204.5) stays under the 228 px HUD line (LAB-R-Q03).
4. **Hollow rule:** the open air under a rise-64 slab must be closed on both sides (a step or a rise-32 slab), so no actor walks into a face that the bot probe cannot see (LAB-R-Q04).
5. **Head-bump rule:** no slab floats over a walking lane.
6. All enemies stand on the floor (`walking_line_at(x) == 580`), use `EnemyAi::Chase` and start with the default facing -1.
7. Starting x distance from Carl to the nearest enemy is more than 220 (goblin sight), so no room starts in a fight.
8. Every mandatory move is a run, a 32 px hop or a walk-off drop. A 64 px surface is always reached in two 32 px stages; a direct 64 px jump exists but is never required.

---

## 3. R00 "The Setup Cage"

**Purpose (rooms table):** can testers pick a valid profile, mode and variant from a fixed list without tutorials? **Tactic tested:** none; this is a safe warm-up that teaches the two hop heights (32 and 64 in two stages), the walk-off drop and the walk-in exit. No hostiles (LAB-Q08).

### 3.1 JSON (`assets/levels/combat_lab/r00_setup_cage.room.json`)

```json
{
  "schema": "floor01-demo-hall-v1",
  "status": "COMBAT-LAB-01 R00 greybox: geometry only, composed by camera_lab::layout from approved Floor 01 kit pieces. Lab entry only.",
  "scenarios": [
    {
      "id": "LAB-R00",
      "name": "The Setup Cage",
      "subtitle": "Dev-only selection screen and a safe warm-up",
      "act": 1,
      "setting": "dry-catacomb",
      "width": 640,
      "height": 660,
      "floor": 580,
      "spawn": { "carl": 128, "donut": 96 },
      "camera_intro": 128,
      "platforms": [
        { "x": 288, "y": 548, "w": 64, "h": 16 },
        { "x": 416, "y": 516, "w": 64, "h": 16 },
        { "x": 480, "y": 548, "w": 32, "h": 16 }
      ],
      "solid_backwall": [],
      "props": [],
      "enemies": [],
      "hazards": [],
      "goal_x": 560,
      "purpose": "Warm-up: hop 32, climb 64 in two stages, drop, walk into the exit. No hostiles.",
      "expected": "Menu confirmed, then walk to the exit. Clear = menu confirmed.",
      "coop": "Both heroes walk the same course; the exit needs both inside the exit rect."
    }
  ]
}
```

### 3.2 Derived geometry (must match exactly)

| Key | Value |
|---|---|
| Play x | 16..624 |
| P0 "Hurdle" | x 288..352, rise 32 (solid 4..32) |
| P1 "Top step" | x 416..480, rise 64 (solid 36..64) |
| P2 "Landing" | x 480..512, rise 32 (closes P1's hollow on the east) |
| Generated steps | exactly one: S0 rect (384, 548, 32, 32), x 384..416, height 32 |
| Exit | (536, 484, 64, 96); portal art x 512..624 on clear floor |
| Camera | locked: the room is 640 wide, so the scroll and the auto camera both show x 0..640 |

### 3.3 Side view (1 char = 16 px wide, 1 row = 16 px tall, heights above the walking line, `#` end wall, `=` slab, `H` step, `_` floor)

```
  96 |#......................................#|
  80 |#......................................#|
  64 |#.........................====.........#|
  48 |#.........................====.........#|
  32 |#.................====..HH....==.......#|
  16 |#.................====..HH....==.......#|
   0 |#______________________________________#|
     |      D C                          E    |
     x=0       160       320       480     640
```

### 3.4 Reachability (Carl and Donut identical, see 1.2)

| Move | Takeoff x window (running; "hold" = ticks the direction is held after the jump) | Required |
|---|---|---|
| floor -> P0 (east) | full hold 136-222; hold 10: 244-276 | yes |
| P0 -> S0 (east) | hold 10: 340-362 | yes (or drop into the 352..384 gap, then hop S0 from the gap) |
| S0 -> P1 (east) | 386-404 at any hold 6-24 | yes |
| P1 -> P2 (east) | hold 10: 470-490 | yes (or full hold: P1 -> floor, 470-490) |
| P2 -> floor (east) | walk off at 512 | yes |
| floor -> P2 (west) | hold 10: 524-556 | return |
| P2 -> P1 (west) | 492-510 at any hold | return |
| floor -> P1 direct (west) | full hold 524-600 | optional |
| P1 -> S0 (west) | hold 10: 406-434 | return |
| S0 -> P0 / floor (west) | walk off 384 | return |

Surface graph (from the stand-in): floor, P0, S0, P1, P2 all mutually reachable; maximum head 204.5 px.

### 3.5 Co-op and bot

- Width 640 allows a separation above 448 (spawn 96 to exit 560 is 464), so the wall can engage only if one hero waits at the spawn while the other stands in the exit. Expected and harmless.
- Every face is probe-visible: P0 and P2 undersides are 4 px, S0 is masonry, P1's hollow is closed by S0 and P2. The follower clears the course with 32 px hops.

### 3.6 Menu overlay (LAB-00-wire overlay, not geometry)

| Row | Values | v1 state |
|---|---|---|
| Profile | `floor01-postbiscuit-lab-v1` | fixed, shown read-only (LAB-Q01) |
| Mode | `solo_carl`, `solo_donut`, `coop_local` | selectable |
| Rules | `legacy`, `cf02_rules` | `cf02_rules` greyed out until CF02-F |
| Variant | `v1`, `v2`, `v3` | selectable; maps to the effective seed 1/2/3 (LAB-R-Q06) |
| Start room | `R01`, `R02` | later rooms greyed out until authored |

`sandbox` (including the held-out R01H) and `stress` are CLI-only and never in the menu. While the menu is open, gameplay input is ignored; after confirmation the warm-up course is playable and the exit opens.

---

## 4. R01 "Impact Pit"

**Purpose:** do jab, kick, contact and wall impacts tell a consistent story? **Tactic tested (legacy):** spacing a three-goblin brawl on a flat lane; the push kick into the second goblin (CP-02 contact: 0 damage, 24-tick stagger) and into the end wall or the nook face (stops the flier dead; CF02-B later adds a slam with no geometry change). The nook is the protected corner: a goblin cannot hit a hero standing on it (swing reaches 30, nook is 32).

### 4.1 JSON (`assets/levels/combat_lab/r01_impact_pit.room.json`, three scenarios)

Geometry is identical in the three scenarios; only `enemies` differ. A validator asserts that.

```json
{
  "schema": "floor01-demo-hall-v1",
  "status": "COMBAT-LAB-01 R01 greybox: geometry only. Three placement variants (v1..v3) selected by the effective seed 1..3. Lab entry only.",
  "scenarios": [
    {
      "id": "LAB-R01-V1",
      "name": "Impact Pit",
      "subtitle": "Three goblins on a flat lane between a nook and the impact wall (v1)",
      "act": 1,
      "setting": "dry-catacomb",
      "width": 1024,
      "height": 660,
      "floor": 580,
      "spawn": { "carl": 176, "donut": 144 },
      "camera_intro": 176,
      "platforms": [ { "x": 16, "y": 548, "w": 96, "h": 16 } ],
      "solid_backwall": [],
      "props": [],
      "enemies": [
        { "kind": "goblin", "x": 560 },
        { "kind": "goblin", "x": 588 },
        { "kind": "goblin", "x": 880 }
      ],
      "hazards": [],
      "goal_x": 944,
      "purpose": "Flat brawl lane bounded by the east impact wall, with a protected 32 px nook on the west.",
      "expected": "A pair meets the hero mid-lane; the third holds near the exit. Clear = all goblins defeated.",
      "coop": "Separation wall 448: a hero on the nook keeps the other at about x <= 548."
    },
    { "id": "LAB-R01-V2", "...": "same keys as V1 except subtitle (v2) and enemies below" },
    { "id": "LAB-R01-V3", "...": "same keys as V1 except subtitle (v3) and enemies below" }
  ]
}
```

The `"..."` lines are shorthand in this spec only; the committed file repeats every key in full.

| Variant (effective seed) | Goblin x (all `Chase`, facing -1) | Shape | Nearest start distance |
|---|---|---|---|
| V1 (seed 1) | 560, 588, 880 | a pair, then a late single near the exit | 384 |
| V2 (seed 2) | 432, 832, 860 | an early single, then a pair by the wall | 256 |
| V3 (seed 3) | 512, 640, 768 | a spread line of three, 128 apart | 336 |

### 4.2 Derived geometry

| Key | Value |
|---|---|
| Play x | 16..1008 |
| P0 "Nook" | x 16..112, rise 32, against the west wall; no generated steps (rise 32 gives count 0) |
| Floor lane | 112..1008, flat |
| Impact surfaces | east end wall at x 1008; the nook's east face at x 112 (solid 4..32) |
| Exit | (920, 484, 64, 96); portal art x 896..1008 |
| Camera | scroll; the auto camera may widen to 960 (all three goblins visible from mid-lane) |

### 4.3 Side view (V1, same scale and legend as 3.3; `1 2 3` goblins)

```
  96 |#..............................................................#|
  80 |#..............................................................#|
  64 |#..............................................................#|
  48 |#..............................................................#|
  32 |#======........................................................#|
  16 |#======........................................................#|
   0 |#______________________________________________________________#|
     |         D C                       12                  3   E    |
     x=0       160       320       480       640       800       960
```

### 4.4 Reachability

| Move | Window | Required |
|---|---|---|
| floor -> nook (west) | full hold 124-176 | optional |
| nook -> floor | walk off at 112 | optional |

Surfaces: floor and nook, mutually reachable; maximum head 172.5 px. The goblins never reach the nook top; they stop at x 124 (face at 112 plus half width 12).

### 4.5 Co-op and bot

- A hero on the nook (centre at most about 100) holds the partner at x <= 548; the exit needs both heroes in 920..984, so the wall never blocks the clear.
- The nook face has a 4 px underside: probe-visible. The follower dodges tells within 40 px and never attacks; solo mode is a human switch-through for kills.

### 4.6 Held-out geometry R01H (ML only, LAB-Q07)

File `assets/levels/combat_lab/r01h_impact_pit_holdout.room.json`, id `LAB-R01H`. A mirror of V1 with the nook moved east; reachable only through the CLI `sandbox` probe, never the menu, never the protocol trials.

| Key | Value |
|---|---|
| Platform | `{ "x": 912, "y": 548, "w": 96, "h": 16 }` (nook 912..1008) |
| Spawn | carl 848, donut 880 |
| Goblins | 464, 436, 144 (all `Chase`, facing -1, so they start facing away from Carl: an intended held-out difference) |
| goal_x | 64, giving exit (40, 484, 64, 96), portal art 16..128 |
| Reachability | floor -> nook (east) mirrors 4.4; maximum head 172.5 |

```
  32 |#........................................................======#|
  16 |#........................................................======#|
   0 |#______________________________________________________________#|
     |    E    3                 2 1                       C D        |
```

---

## 5. R02 "Broken Scaffold"

**Purpose:** do elevation, short routes, cover, aerial hits and the shared camera improve combat? **Tactics tested (legacy):**
- the mandatory 32 px climb-and-drop route through two ground pockets;
- the **bait, whiff, drop, punish** loop: a hero on a 32 px ledge is above a goblin's swing (0..30) but goblins tell on x distance alone;
- **Donut's ledge pounce:** the Pounce starts from a ledge and lands its 0..17 box on floor targets; this is the Donut-favoured position (see LAB-R-Q09);
- **spit cover:** only P1's slab (solid 36..64) crosses the glob band (42..48), so P1 is the refuge and everything east of x 832 is exposed.

### 5.1 JSON (`assets/levels/combat_lab/r02_broken_scaffold.room.json`)

```json
{
  "schema": "floor01-demo-hall-v1",
  "status": "COMBAT-LAB-01 R02 greybox: geometry only, two tiers plus a spit refuge. Lab entry only.",
  "scenarios": [
    {
      "id": "LAB-R02",
      "name": "Broken Scaffold",
      "subtitle": "Two goblin pockets, a lookout and a llama yard",
      "act": 1,
      "setting": "dry-catacomb",
      "width": 1280,
      "height": 660,
      "floor": 580,
      "spawn": { "carl": 112, "donut": 80 },
      "camera_intro": 112,
      "platforms": [
        { "x": 288, "y": 548, "w": 128, "h": 16 },
        { "x": 736, "y": 516, "w": 96, "h": 16 },
        { "x": 832, "y": 548, "w": 32, "h": 16 },
        { "x": 1088, "y": 548, "w": 64, "h": 16 }
      ],
      "solid_backwall": [],
      "props": [],
      "enemies": [
        { "kind": "goblin", "x": 560 },
        { "kind": "goblin", "x": 592 },
        { "kind": "goblin", "x": 928 },
        { "kind": "llama", "x": 1024 }
      ],
      "hazards": [],
      "goal_x": 1200,
      "purpose": "Two tiers (32, 64) and a spit refuge; goblins pinned in pockets, the llama in the yard.",
      "expected": "Drop shelf -> pocket pair -> lookout -> yard goblin and llama -> roost -> bay exit. Clear = all enemies defeated.",
      "coop": "Separation wall 448: a hero on the roost keeps the other at x >= 652."
    }
  ]
}
```

### 5.2 Derived geometry

| Key | Value |
|---|---|
| Play x | 16..1264 |
| P0 "Drop Shelf" | x 288..416, rise 32 |
| S0 (generated) | exactly one: rect (704, 548, 32, 32), x 704..736, height 32 |
| P1 "Lookout" | x 736..832, rise 64 (solid 36..64; hollow below closed by S0 and P2) |
| P2 "Broken plank" | x 832..864, rise 32 |
| P3 "Roost" | x 1088..1152, rise 32 |
| Floor segments | entry 16..288, pocket 416..704, yard 864..1088, bay 1152..1264 |
| Enemy confinement | goblins A, B: pocket, x 428..692; goblin C: yard, x 876..1076; llama: yard, x 887..1065 (inside its 160 leash) |
| Exit | (1176, 484, 64, 96); portal art x 1152..1264 (the whole bay floor, clear) |
| Camera | scroll; the auto camera widens to at most 960 |

### 5.3 Side view (same scale and legend as 3.3; `A B G` goblins, `L` llama)

```
  96 |#..............................................................................#|
  80 |#..............................................................................#|
  64 |#.............................................======...........................#|
  48 |#.............................................======...........................#|
  32 |#.................========..................HH......==..............====.......#|
  16 |#.................========..................HH......==..............====.......#|
   0 |#______________________________________________________________________________#|
     |     D C                           A B                    G     L          E    |
     x=0       160       320       480       640       800       960       1120    1280
```

### 5.4 Reachability (Carl and Donut identical)

Main path (all mandatory moves are 32 px hops or drops): entry -> P0 -> drop into pocket -> S0 -> P1 -> P2 (or run off P1 into the yard) -> yard -> P3 -> drop into bay -> exit.

| Move | Window | Required |
|---|---|---|
| entry -> P0 (east) | full hold 138-276 | yes |
| P0 -> pocket | walk off at 416 | yes |
| pocket -> S0 (east) | full hold 552-606; hold 10: 660-692 | yes |
| S0 -> P1 (east) | 706-724, hold at most 24 | yes |
| pocket -> P1 direct (east) | full hold 608-692 | optional |
| P1 -> yard (east) | run off at 832 with full hold, lands near x 976 | yes (or via P2) |
| yard -> P3 (east) | full hold 938-1022 | yes |
| P3 -> bay | walk off at 1152 | yes |
| bay -> P3 (west) | hold 10: 1164-1196 | return |
| yard -> P2 (west) | hold 10: 876-908 | return |
| P2 -> P1 (west) | 844-862 | return |
| yard -> P1 direct (west) | full hold 876-960 | optional |
| pocket -> P0 (west) | full hold 428-568 | return |

Surfaces floor, P0, S0, P1, P2, P3 form one strongly connected graph; maximum head 204.5 px.

### 5.5 Spit and threat matrix (legacy)

Glob band = 42..48 above the floor. The llama aims at Carl's x only.

| Hero position | Goblin swing (0..30) | Glob | Notes |
|---|---|---|---|
| Carl on any floor east of x 832 | hit | **exposed** | the yard and bay are the danger zone |
| Carl on the floor or a ledge west of x 832 | floor: hit; ledge: immune | **blocked** by P1's east face | the entry and the pocket are safe from spit |
| Carl on P1 (64) | immune | **refuge** | the band passes below his feet and P1 stops it |
| Carl on P2 or P3 (32) | immune | **exposed** (body 32..84 crosses the band) | |
| Donut on a floor | hit | never hit (hurtbox 0..24 under the band) | |
| Donut on P2 or P3 | immune | exposed when the glob aimed at Carl passes her | |

**No turret:** from a ledge, Carl's jab/haymaker/kick boxes (18..38 above his feet) and Donut's Missile (16..22) pass over a goblin (38 tall) and the llama (39 tall). Ledges only buy safety and a drop or pounce; they cannot kill from above.

### 5.6 Co-op and bot

- Pocket fight to yard: centres about 560 and 976 are 416 apart, under the 448 wall. A hero on the Roost (centre up to about 1100) holds the partner at x >= 652, i.e. on S0, P1 or the east end of the pocket.
- Every face is probe-visible: P0, P2, P3 undersides are 4 px; S0 is masonry; P1's 36 px underside is never approachable (S0 and P2 close it). The follower clears the main path with 32 px hops; the AI partner may still lag on the climb (01B-wire owns real platform navigation; solo R02 is a human switch-through, section 4 of the parent doc).
- Respawn is safe: the llama cannot leave the yard and P1 blocks spit toward the spawn.

---

## 6. Common room flow: checkpoints, exits, restart

| Item | Rule |
|---|---|
| Checkpoint | one per room, at Carl's spawn (`Fixture.carl_x = spawn.carl`), Donut at `spawn.donut` (32 px behind) |
| Select tap | respawn at the room checkpoint (`CombatState::respawn`): living goblins return home, the dead stay dead, the llama is **not** reset |
| Select hold (60 ticks) | restart the whole room from the JSON, llama included |
| Clear | R00: menu confirmed. R01, R02: `demo::complete` (all goblins and llamas defeated) |
| Exit | fires when the room is clear and the controlled hero (solo) or both heroes (co-op) overlap the exit rect. The lab runner evaluates the rect itself and leaves `kit.exit = None` like the demo. The exit's open/closed state is shown by the lab overlay; the portal art stays the existing closed portal |
| Next room | R00 -> chosen start room; R01 -> R02; R02 -> end of the LAB-01 set (factual panel, LAB-Q10), then back to R00 |

---

## 7. Enemy fixtures (to mirror from the JSON)

The JSON `enemies` array is authoritative. The lab fixtures (`src/combat_lab_fixtures.rs`, LAB-Q03) mirror it as `EnemySpawn { x, ai: EnemyAi::Chase }` and `LlamaSpawn { x, ai: EnemyAi::Chase }`, and a unit test asserts the fixture list equals the parsed JSON list.

| Room | Goblins | Llamas | Total (LAB-Q04 cap 4) |
|---|---|---|---|
| R00 | none | none | 0 |
| R01 V1/V2/V3 | 3 (table 4.1) | 0 | 3 |
| R01H | 3 | 0 | 3 |
| R02 | 560, 592, 928 | 1024 | 4 |

---

## 8. Seed policy

- Variant N of R01 = scenario `LAB-R01-VN` **and** the effective seed N (CF02-Q58: the fixture seed or the `--cf02-seed` override). There is no lab RNG (LAB-Q06).
- Only seeds 1, 2, 3 are valid for R01. Any other value is rejected by the lab entry with an error that lists the valid values; no hidden modulo mapping.
- R00 and R02 have one layout; their `Fixture.seed` is 1, and a `--cf02-seed` override is recorded in the trace but changes nothing in the geometry.
- On the legacy rules the variants differ **only in placement**: the parent doc's "distinct seeded temperaments" need CF02-G morale. Reports must say so.
- R01H is never selected by seed; only the CLI `sandbox` probe loads it.

---

## 9. Acceptance targets

### 9.1 Headless asserts (LAB-01 headless part, no Bevy)

1. Each file parses through `camera_lab::room::parse_rooms` with `snap_dy == 0` and render size `(width, 672)`.
2. The generated steps equal exactly: R00 `[(384,548,32,32)]`, R01 V1..V3 `[]`, R01H `[]`, R02 `[(704,548,32,32)]`.
3. Derived platform rects equal sections 3.2, 4.2, 4.6 and 5.2; the three R01 scenarios have byte-identical geometry.
4. `build_kit` succeeds and its no-void check passes.
5. `walking_line_at(x) == 580` at every spawn and every enemy column.
6. **Reachability probes** drive the real `step_hero_driven` with scripted inputs built from the windows in 3.4, 4.4, 4.6 and 5.4. They visit every surface and return to the spawn, for Carl and for Donut, keeping the head at most 228 px above the walking line on every tick.
7. **No softlock:** from every surface, the spawn surface and the exit rect are reachable (the surface graph is strongly connected).
8. Enemy count and types equal section 7; every enemy is `Chase` with facing -1 at load; at most 4 enemies are active at once.
9. **Confinement:** 3000-tick runs with Carl parked on each surface in turn (a death respawns him at the checkpoint and the run continues) keep each enemy inside the x ranges in 5.2 (R02) and 112..1008 (R01).
10. **Spit matrix:** scripted globs reproduce 5.5 (blocked west of 832; P1 refuge; Donut on the floor never hit).
11. **Co-op wall:** a 2P probe confirms the wall engages at 448 and that both heroes can stand in each exit rect together.
12. **Bot probe:** solo runs where the follower trails the main path; regroup events are **measured and reported**, not required to be zero.
13. **Clear budget:** the legacy autopilot (whatever policy the LAB-00 runner exposes) clears R01 V1..V3 and R02. Soft targets: about 2400 ticks for R01 and 3600 for R02. Record the actual values; fail only above twice the target.
14. Legacy goldens and `--demo` outputs are unchanged.

### 9.2 Hard gate 1 captures (LAB-01 live part, needs LAB-00-wire)

- Stills of each room at 640x360, 640x400 and 1080p (auto camera), with the spawn, the exit and every slab in frame at least once.
- 1x GIF clips per room and per mode (`solo_carl`, `solo_donut`, `coop_local`): an R00 warm-up, R01 V1 with a kick into the second goblin and a kick into the wall, and R02 with the bait-drop punish, a Donut ledge pounce, a llama glob blocked by P1 and a glob that hits on P3.
- A real two-pad co-op session through R00 to R02, or the capture is marked "not tested".
- Every attempt is labelled recorded or not recorded per PLAYTRACE PT-B.

---

## 10. Adopted defaults (LAB-R-Q)

| ID | Decision |
|---|---|
| LAB-R-Q01 | One file per room under `assets/levels/combat_lab/`: `r00_setup_cage`, `r01_impact_pit` (scenarios V1..V3), `r01h_impact_pit_holdout`, `r02_broken_scaffold`, all `.room.json`, schema `floor01-demo-hall-v1`, `"h": 16` on every platform for consistency. |
| LAB-R-Q02 | Existing format only: no props, no hazards, no backwall solids in R00 to R02. Hazards start in R03. |
| LAB-R-Q03 | Standing surfaces at most 64 above the floor; slab rise 32 or 64 only (HUD line 228). |
| LAB-R-Q04 | Hollow and head-bump rules (section 2, rules 4 and 5) so the follower probe never meets an invisible face. |
| LAB-R-Q05 | One checkpoint per room at the spawn; tap respawns, hold restarts the room. |
| LAB-R-Q06 | Variant N = scenario VN + effective seed N, R01 only; seeds outside 1..3 are rejected; R00/R02 seed 1. |
| LAB-R-Q07 | R01H is held out and loaded only through the CLI `sandbox` probe. |
| LAB-R-Q08 | Clear = all enemies defeated (R00: menu confirmed). The exit fires on clear plus the controlled hero (solo) or both heroes (co-op) in the exit rect; the runner evaluates it, `kit.exit` stays `None`. |
| LAB-R-Q09 | On the legacy rules the Donut-favoured position is the ledge pounce (P0, P3 and the nook), not a Donut-only height. |
| LAB-R-Q10 | Live entry uses the auto camera like `--demo`; headless captures use the scroll camera; the geometry is valid under both. |
| LAB-R-Q11 | R00 menu rows as in 3.6; `sandbox` and `stress` are CLI-only. |
| LAB-R-Q12 | All enemies spawn `Chase` on the floor with the default facing; no `Passive` enemies in R00 to R02. |
| LAB-R-Q13 | The JSON `enemies` list is authoritative; the Rust fixtures mirror it under a test. |
| LAB-R-Q14 | Open question 1 (a Donut-only route is impossible on the legacy rules) is deferred, not rewritten: a later data-only R02 variant adds the rise-112 perch after the Q10 cat jump lands (adopted 2026-11-20 with the LAB-01 implementation). The hard-gate wording of the work order stays flagged until then. |

---

## 11. Open questions (genuinely open, not blocking LAB-01)

1. **Donut-only route.** The work-order gate asks for "at least one optional offensive elevated route Carl cannot immediately take". On the legacy physics it cannot be met, because Donut's jump equals Carl's (1.2). This contradicts the gate; it is flagged, not silently rewritten. Proposal once the Q10 cat jump lands (proposed 500 px/s, apex about 119): a data-only R02 variant adds a rise-112 perch above the Lookout. That needs a second HUD check and a format change (no generated steps above it).
2. **The llama targets Carl only** (`step_llamas` uses Carl's x). In `solo_donut` the llama keeps firing at the AI Carl. A party-aware target is CF02-C1, not the lab's call.
3. **Bait-whiff-drop loop.** Goblins tell on x distance even when a hero stands 32 px above them. It may dominate R02; CF02-G morale or a height check would change it. Game-feel watch item.
4. **Respawn does not reset the llama.** Accepted for LAB-01 (the hold-restart resets it); a rules change belongs to the combat owner.
5. **Bot climbing.** The follower handles 32 px hops but has no platform graph; real AI navigation is 01B-wire. Solo R02 stays a human switch-through.
6. **Room length.** On the legacy rules R01 and R02 may run shorter than the 60-90 s aim. Measure them (LAB-Q05); do not pad. Measured: see section 13.
7. **CF02-F 1-tile perches.** The CF02-F microarena uses a 16 px slab and a 48 px parapet; main's format makes only 2-tile slabs with generated steps. The lab will not use them until the format supports them.
8. **Portal open state.** There is no open-portal art; the overlay shows the exit state. A real open-portal piece is an art work order.

---

## 12. Provenance

Designed by the top-tier model (Claude Opus) as the LAB-01 level-design step. Geometry and windows were checked with a scratch Python stand-in of the parser and `step_hero_driven`, which is **not** evidence: the headless asserts in 9.1, run on the real Rust code, are. Execution (JSON, fixtures, asserts, captures) goes to the medium tier; review goes to a separate top-tier agent.

---

## 13. Implementation errata (LAB-01, measured on the real code)

Measured by the headless asserts of section 9.1 on `main` (the legacy rules). None of these changes a geometry value. The tables above keep the stand-in window numbers (they remain the design intent); where the real integration differs, the measured value is the one below and is what the asserts check.

1. **Takeoff window edges.** The stand-in window tables are up to 2 px wider than the real integration at one edge of seven rows: floor -> P0 full hold 138..276 (not 136), floor -> P0 hold 10 246..276 (not 244), P0 -> S0 hold 10 342..362 (not 340), floor -> P2 west hold 10 524..554 (not 556), R02 pocket -> S0 full 554..606 and hold 10 662..692 (not 552 and 660), bay -> P3 west 1164..1194 (not 1196), yard -> P2 west 876..906 (not 908), pocket -> P0 west 428..566 (not 568). Every other row, including both ends of the head height (204.5 px), matches. The assert allows 8 px of edge slack and prints the measured table.
2. **R00 `P2 -> P1` (west) "at any hold".** True for holds of 6 to 24 ticks; a full hold overshoots P1 and lands on P0 or the gap (harmless, P0 is on the route).
3. **Llama range.** The llama body is shifted 3 px behind its column, so its column ranges 884..1068 where 5.2 says 887..1065 (body span). The confinement assert uses the column range.
4. **Room length (LAB-Q05).** `aim_seconds` (60 to 90 s) is a **hypothesis for the CF02 rules**, not a legacy-rules target. Measured with the headless autopilot (a perfect-spacing press every 4 ticks, so faster than any human): R01 V1, V2, V3 clear in 287, 283 and 263 ticks (about 4.5 s) and R02 in 1175 ticks (about 20 s, one death). A human will be slower, but the legacy rooms are well under the aim. Nothing is padded; the real number comes from the first recorded two-pad session. The assert 13 ceiling (twice the soft target) cannot catch "too short"; a floor needs the human baseline first.
5. **Camera.** The rooms are 660 px tall, so the vertical camera follows jumps.
6. **Live entry.** A relaunch starts the scene for the next room and a second pad must press Start again in every room (see the evidence README).
