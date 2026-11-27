# Floor 01 finale: the Ball of Swine and the floor exit in the floor run (BOS-PLAN)

**Status:** design and integration plan only. Nothing here is implemented, playtested or accepted. **BOS-A** (one Sonnet PR, section 13) builds it as a labelled playable greybox. After it merges, its status is "merged, game-feel review pending". The execution order is [the BOS work order](../07-agents/floor-01-bos-workorder.md).

**Owner decisions (cited for every adopted default below):**

- "go with all" (2026-11-24, restated 2026-11-26): every owner-gated Floor 1 item goes ahead with adopted defaults, each recorded by ID.
- "ensure everything ends on main" (2026-11-25): every room must be reachable in `--scene f01_late --f01-run` in story order. New behaviour is on by default. Unfinished items land as a labelled playable greybox.
- Budget (2026-11-27): the smallest playable greybox that ends the floor. One implementation PR. Art, cinematics and extra phases go to named follow-ups.

**Source.** This plan brings the owner draft **#192** onto main in adapted form. The draft is on branch `design/f01-ball-of-swine-finale`:

- `docs/01-design/floor-01-ball-of-swine-finale-encounter.md`: the SWINE-01 contract, the 1200 px arena with L0..L4 and the suspended gate, the Tusklings, beats E0..E6, cases BSW-01..20 and the owner questions;
- `docs/07-agents/floor-01-ball-of-swine-finale-workorder.md`: stages BSW-A..G.

#192 itself is not merged. This plan keeps its core beat: **a committed roll with a readable tell ends in an impact, and the impact opens a vulnerable recovery**. It also keeps its fairness rules (no untelegraphed hit, no mandatory phase, no immunity, no camping) and its "the exit opens only on a real defeat" rule. Everything else is either cut by ID or deferred to a named follow-up (section 12).

**Read with:**

- [the Juicer boss design](../02-art/floor-01-boss-design.md): the framework, solo fairness M1..M3, D1, restart;
- [the troglodyte integration plan](floor-01-troglodyte-integration.md): the shape this plan copies, and its review lessons;
- the art specs [SWINE-01](../02-art/swine-01-spec.md), [TUSK-01](../02-art/tusk-01-spec.md) and [the arena concept](../02-art/floor-01-ball-of-swine-arena-concept.md) (direction only; BOS-A does not need them);
- [the playtest packet](../04-production/f01-playtest-ready-01.md), which BOS-A updates (section 14).

## 1. What main holds today

- The run has 17 rooms and ends at `mastery` (`f01_mastery`, CF02 rules). After the last room, `next_room` returns `NextStep::FloorEnd`, and `floor_end_panel` shows "FLOOR 1 SLICE FINISHED". The floor has no exit room and no final boss.
- The Juicer (`brute_boss_f01`, `src/boss.rs`, `src/boss_scene.rs`) is a complete data-driven boss framework:
  - modes Dormant > Neutral > ChargeTell > Charge > Rebound > Stun, plus Pound and Roar;
  - a closing gate behind the party, a rack drop hazard and an exit wall that opens on defeat;
  - `BossData` uses `deny_unknown_fields` and is validated (tells and punishes at least 24 ticks).
  The Juicer is **frozen**: its JSON, fixtures, goldens and `COMBAT_STATE` hashes must not move.
- SWINE-01 and TUSK-01 exist as pixel sources (`assets/source/characters/swine/`). They have no runtime export and no presenter.

## 2. Decisions in one place

| ID | Decision | Why |
|---|---|---|
| BOS-AD1 | The Ball is **room 18, `bos`**, appended after `mastery` | #192 keeps mastery; "ends on main" |
| BOS-AD2 | It is a **second data file** on the Juicer framework, not a new boss system | budget; reuse |
| BOS-AD3 | It uses the **locked 640 px boss arena**, undressed (plain catacomb) | every tell is always in view |
| BOS-AD4 | It has **one phase**: `queue_roar: false` | #192 "no mandatory phases" |
| BOS-AD5 | The roll is the Juicer **charge**; the impact is the **crash**; the recovery is the **stun** | it is the core beat, with no new mode |
| BOS-AD6 | **Two arches** (abstract boss walls) with hero **alcoves** behind them | the dodge for a roll that is too tall to jump |
| BOS-AD7 | A **siege** rule against alcove camping | the degenerate turret (section 9) |
| BOS-AD8 | A **chain weight**: the Juicer's left rack, re-skinned as greybox | the player-caused environmental beat, at low cost |
| BOS-AD9 | The floor exit is the **arena's right wall**, which opens on defeat. Then the **floor-end panel** shows | reuses the Juicer's exit |
| BOS-AD10 | No Tusklings, no camera pullback, no elevated routes and no cinematics in BOS-A | deferred by ID (section 12) |

## 3. Run position

| # | id | scene | rules | milestone | story_order pins | grants |
|---|---|---|---|---|---|---|
| 17 | `mastery` | `f01_mastery` | cf02 | greybox | `f01_late_mastery` | none (unchanged) |
| **18** | **`bos`** | **`f01_ball`** | **boss** | **greybox** | **`f01_late_mastery`** | **none** |
| after | floor end | | | | | |

- **BOS-AD11:**
  - add `RoomScene::Ball` (serde `"f01_ball"`), mapped to `RoomRules::Boss`;
  - **do not reuse `RoomScene::Boss`**: TROG-R1 finds the Juicer through `scene == Boss` and requires `trog_strength` to directly precede it, so a second `Boss` room would break that rule;
  - the room title is data: `"The Ball of Swine"` (the fan-concept layer).
- **BOS-AD12:**
  - the story row is `{ id: "bos", pins: ["f01_late_mastery"], grants: [] }`;
  - the heroes arrive with exactly the mastery loadout (all Floor 1 grants, Donut's `jump_reach_boost`, dynamite if held);
  - the run adds no new pin key, so `SCOPED_PIN_KEYS` stays at 37 entries.
- **BOS-AD13:** the ledger scene for the Ball room is `SceneKind::F01Late`, as for `mastery`, never `SceneKind::Boss`. The Ball reuses `BOSS_ENEMY_ID`, and `brute_boss_f01_defeated` is `armed_in ["boss"]`. A `Boss` scene kind would re-fire the Juicer's grants. F14 asserts that it does not.
  - **Mechanism (BOS-AD33).** `story_setup` in `src/bin/floor01_scene.rs` picks `SceneKind::Boss` only for `fixture == BOSS01_SCENE`, and any other fixture falls to `carl_prog_fixtures::story_setup(name)`, which would drop the mastery loadout. The Ball therefore gets its **own fixture constant** (not `parsed.fixture = BOSS01_SCENE`), mapped to `SceneKind::F01Late` through the mastery path. The PLAYTRACE replay's scene mapping (`src/playtrace/replay.rs`) must match it.
- The manifest `status` string gets one appended BOS-A step. `MANIFEST_HASH` is re-pinned (section 10).

## 4. Arena geometry

The geometry comes from the Juicer's `arena(root, dressed=false)` with a data-selected `BossData`. It has the same floor, ceiling and 640x400 locked view, and the same gate. Every x value is in world px. The floor is y 0.

| Element | x | Notes |
|---|---|---|
| Gate solid (closes behind the party) | 0..28 | the Juicer's gate block, unchanged |
| **Left alcove** (hero refuge) | 28..110 | a hero is "in" it when pivot <= 110 (valid pivots 40..110). The 18 px gap to the arch absorbs the last charge tick's ram overreach (BOS-AD25) |
| **Left arch** (boss wall `left_wall_x`) | 128 | abstract; it does not collide with heroes |
| Chain-weight post (the rack strut) | 64..80, y 0..64 | in the left alcove; 3 melee hits drop the weight |
| Boss lane (`clamp_walk`, margin 64) | pivot 192..448 | the ball's pivot never leaves this range |
| **Right arch** (boss wall `right_wall_x`) | 512 | abstract |
| **Right alcove** | 530..608 | pivot >= 530 (valid pivots 530..596, BOS-AD25) |
| Hero right wall / floor exit | 608 (`hero_right_wall_x`) | WALL_W 32. It slides open after defeat |

- **BOS-AD14:** the arches are **not terrain**. Heroes walk freely between the lane and the alcoves.
  - The ball is held out of the alcoves only by its abstract walls (`clamp_walk` and the charge stop), which is exactly how the Juicer's walls work today.
  - BOS-A draws each arch as a non-colliding greybox outline: two 16 px posts and a lintel at y 104.
  - This adds no room-JSON solid, so the validator caps are not touched: no 48 px lane obstacle and no static solid over 80 px. The post is the rack strut, 64 px tall, which is under the 80 px static cap. It is a boss-scene object, as in the Juicer.
- **BOS-AD15:** a new optional `arena.hero_right_wall_x` (default `right_wall_x`) decouples the hero wall and the exit from the boss's right wall. The Juicer JSON does not contain it, so the Juicer's behaviour is unchanged. On the left side the heroes are already bounded by the gate solid, not by `left_wall_x`.
- **Gate (BOS-AD28):** the Juicer's values, except `engage_x` 96 > **140**, `inside_x` 40 > **112** and `regroup_x` 48 > **112**. `engage_x` is an any-hero test, but closing needs every living hero at >= `inside_x`, and the solo regroup puts the bot at `regroup_x`. So no hero can start the fight inside the left refuge.
- **Spawn:** the ball spawns at pivot 448 facing -1, Dormant. It wakes when the gate closes. The view is locked, so the ball is in view at every tick: no wake versus in-view contradiction is possible.

## 5. The boss contract (BOS-AD16, `assets/data/bosses/ball_boss_f01.json`)

- **Identity:** `id: "rolling_boss_f01"` and `species: "rolling_brute"`. These are IP-neutral. `hud.display_name: "THE BALL OF SWINE"` is the fan-concept layer and is data.
- **Schema:** every field the schema requires is present, with the Juicer's shape. The modes the ball never reaches (shockwave, double pound and roar) keep Juicer-valid values so validation passes. F15 proves that they never fire.

| Key | Value | #192 range / note |
|---|---|---|
| `hp_max` | **24** | 20..24 |
| `phase_thresholds` | [16, 8] | must validate; never used, because `queue_roar` is false |
| `phase.queue_roar` | **false** | single phase |
| `walk_px_s` | **60** | roam 45..75 |
| `turn_ticks` | Juicer value | |
| `pound` (the **shove**) | tell **30**, box x[56,96] y[0,28], damage **1**, range [40,100], recover **30** | a close-range bump, so the ball is never safe to hug; it can splash the alcove mouth (6.3) |
| `charge` (the **roll**) | tells **[36,36,36]**, speed **260** px/s, ram box x[0,64] y[0,**104**], damage **1**, launch Juicer, `min_target_px` 128, `min_room_px` **96**, cooldown **[150,150,150]** | tell >= 36 (#192 minimum); 220..280 px/s |
| `crash` (the **impact**) | rebound **24** px over **8** ticks, stun **[66,66,66]** | recovery 48..72 |
| `body` (BOS-AD30) | `standing`, `committed` and `stun` x[-60,60] y[0,124]; `pinned` x[-60,60] y[0,64]; `push_px_per_tick` 6, `min_side_room_px` 28 (Juicer) | collider radius about 60; the hurtbox is the stun box in Rebound and Stun and the pinned box when pinned |
| `rack` (the **chain weight**) | the left rack only: strut x[64,80] y[0,64], 3 hits, footprint 80 at the Juicer offset, boss damage **4**, pin **90**, hero damage 1 | the environmental beat |
| `arena` | ceiling 224, `left_wall_x` **128**, `right_wall_x` **512**, `centre_x` 320, spawn 448 / -1, `walk_margin_px` **64**, `hero_right_wall_x` **608** | |
| `gate` | Juicer values, `engage_x` **140** | |
| `ai_charge_jump_lead` | 18 (Juicer) | see section 7 |
| `exit_slide_delay` | 90 | |
| `hud` (BOS-AD31) | `display_name` "THE BALL OF SWINE", `emblem` absent (no emblem drawn) | the Juicer emblem must not show |
| `siege` (new, optional) | `warn_ticks` **60**, `after_ticks` **120**, `debris_damage` **2**, `debris_launch` [0, 120] | section 6.4 |

- **BOS-AD17 (the hurtbox):** the ball's height is 104. A 128 px ball would make the bottom 104 px the hitting part, with the curved top cleared. That is still above Carl's jump apex (about 90), so Carl's answer to the roll is the alcove, never a jump. Donut with `jump_reach_boost` (apex about 138) can sometimes clear it. This is an optional skill, not a requirement.
- Hero HP is 4. The roll, the shove and the weight's splash deal 1. Only the siege debris deals 2.

## 6. Tick-level states

Ticks are at 60 Hz. The modes are the Juicer's, and only the numbers differ. The ball's hurtbox is its `body` box for the mode (all equal except `pinned`, BOS-AD30). Damage lands in every mode, and the armoured modes (shove strike, roll) are not interrupted.

### 6.1 Roam (Neutral)

The ball walks at 1 px/tick toward the selected target, inside pivot 192..448.

- Target: a **reachable** hero. Reachable means a living hero whose pivot is in 111..529 (not in an alcove, BOS-AD25).
  - BOS-AD18: the alcove rule applies **only when `siege` is present**, so the Juicer's `reachable()` is unchanged.
- If the target is ahead within range [40,100], the ball shoves.
- If `cooldown == 0`, gap >= 128 and run room >= 96, it rolls.
- If nobody is reachable, it walks to `centre_x` and the siege counter runs (6.4).

### 6.2 Roll (ChargeTell > Charge)

| Tick | State | Rule |
|---|---|---|
| 0 | ChargeTell starts | target and heading **locked** (the Juicer's lock); the warning flag is set |
| 0..35 | tell | the ball stands; its body is solid and hurts as normal |
| 36 | Charge (commit) | moves 4.33 px/tick; `solid_body` is None; the ram box hits each hero at most once |
| arrival | the ram front reaches the arch (128 or 512) | the pivot snaps to `crash_pivot`; `BossCrash{wall}` is emitted |
| +0..7 | Rebound | 24 px back over 8 ticks |
| +8..73 | Stun (the **recovery**) | 66 ticks; no target, no hitbox; the body is the hurtbox |

**Escape feasibility (BOS-AD19).** This is the worst case allowed by `min_target_px` 128:

- the ball is at pivot 448 and Carl is at 320;
- Carl must reach the left alcove (pivot <= 110), which is 210 px at 3.083 px/tick, or **68 ticks**;
- the ram front meets the arch on charge tick 60, after the 36-tick tell, at tick **96**;
- the margin is **about 27 ticks** (BOS-AD25).

On that last charge tick `charge()` moves, tests the ram box, then snaps to the wall, so the box reaches up to 4.3 px past the arch (124..188 on the left). A hero at pivot <= 110 has a right edge <= 122 and is never touched. `charge()` stays unchanged (Juicer code); the clearance is in the alcove bounds. F17 asserts it on both sides.

The mirror case has the same margin. A Carl who reacts by tell tick 28 escapes. Every roll ends in a crash, because the arches are always on the path. So every dodged roll gives a punish window.

**Punish feasibility.** For the left arch:

- `crash_pivot` = 128 + 64 + 24 = 216, so the body's left edge is at 156;
- a hero leaving the alcove (pivot 110) at Stun start has about 30 px to cover (10 ticks) to be in jab range;
- that leaves about 56 Stun ticks for hits.

F7 asserts at least 2 hits. If BOS-A measures fewer, it raises stun to 72 (still inside #192's range) and records the change.

### 6.3 Shove (Pound)

| Phase | Ticks |
|---|---|
| tell | 30 |
| strike | the Juicer's strike length; box x[56,96] y[0,28] |
| recover | 30 |

Only the **trigger** needs a reachable target in range. The strike hits every vulnerable hero in the box (`pound_strike`), so from the lane edge (pivot 192 or 448) it reaches 96..136 and 504..544 and can splash a hero at the alcove mouth. Deeper in the alcove (left pivot <= 83, right >= 557) is shove-safe. This is accepted (BOS-AD29): the mouth is the risky spot, the back wall is safe from the shove but not from the siege.

### 6.4 Siege (new, BOS-AD20)

- **Counter.** `unreachable_ticks` counts up while the ball is in Neutral and no living hero is reachable. It resets to 0 when:
  - any hero becomes reachable;
  - a siege crash lands;
  - the fight restarts.
- **Warning.** At **60**, the ball turns toward the target alcove:
  - the target alcove is the one holding the nearest hero by pivot distance, with a tie going **left**;
  - a greybox notice shows (text as data, default "IT SNIFFS AT THE ARCH");
  - a hero who steps out now cancels the siege.
- **Stop.** At the warning the ball stops walking and keeps its x. Turn ticks count toward the counter.
- **Commit (BOS-AD27).** At **120**, the ball starts a normal ChargeTell (36 ticks, heading locked) with `siege = true`, then commits. The siege commit **bypasses** `min_target_px`, `min_room_px` and `cooldown`, and sets `cooldown` as a normal roll does. Every lane x has room: a hide directly after a left crash leaves the ball at about 276 at the warning, 60 px from the arch, and the roll still crashes (F8b).
- **Solo bot (BOS-Q11, adopted fallback).** If G-6 or G-7 fails, the counter treats an M1-protected bot as unreachable while the player-controlled hero is in an alcove, so the bot cannot be an unkillable decoy. The rule is hashed under the same siege gate.
- **Impact.** On the siege crash, every hero whose pivot is in **that** alcove takes `debris_damage` 2 with `debris_launch`. Then Rebound and Stun run as normal, so a sieged hero can still punish.
- **Bait crashes are safe.** A normal (bait) crash never deals debris. The clean beat "bait it into the arch, then punish" stays free.
- **Hashing.** `unreachable_ticks` and `siege` are mixed into `COMBAT_STATE` **only when `data.siege` is Some**. Otherwise the Juicer's hashes would move.

### 6.5 Chain weight (the left rack)

- It uses the Juicer rack exactly:
  - 3 melee hits on the post drop it at once (36 ticks of fall);
  - the footprint is centred 7 px wallward of the left `crash_pivot`, at **209** (169..249), covering the stunned body (156..276).
- If the ball is under it, it takes 4 damage and is pinned for 90 ticks. Heroes in the footprint take 1. The alcove (pivot <= 110) is outside the footprint.
- It arms only while the gate is Closed. It drops once per attempt, and a restart resets it. The AI never triggers it (M3).
- The intended play is to hit the post twice early, then bait the roll into the left arch and land the third hit during the crash.

### 6.6 Defeat

- At 0 HP the boss emits the usual defeat (`BOSS_ENEMY_ID`) and the gate is Released.
- After `exit_slide_delay` (90), the wall at 608 slides open and reveals the stairwell.

## 7. Donut and companion behaviour

- **Solo.** The follower bot is unchanged (BOS-AD21):
  - M1 applies: the AI hero takes hits but never drops below 1 HP;
  - M2 and M3 are the Juicer's, including the jump at `ai_charge_jump_lead` 18, which fails against the 104 px box unless her boosted jump happens to clear it;
  - the bot does **not** flee into alcoves, so in BOS-A it gets rammed and survives through M1. BOS-B may add alcove-fleeing (BOS-Q4);
  - the bot trails the leader by 32 px, so a player hiding at the alcove mouth leaves the bot reachable in the lane. That makes the M1 bot a possible unkillable decoy that resets the siege counter; G-6 and G-7 test it, and BOS-Q11 is the adopted fallback;
  - the bot never triggers the weight.
  - The player swaps (L1/LB) to play Donut, and M1 does not protect the hero the player controls.
- **2P:** no guards (M5). Each player uses an alcove or holds the ball's attention.
- **D1:** Donut at 0 HP is party defeat, as is Carl at 0 HP.
- **Magic Missile:**
  - from an alcove (muzzle about x 122), the ball idling at the centre (body edge 260) is inside the 240 px range. This is the turret that section 9 gates;
  - from the lane it is ordinary chip damage at 1 per hit, limited by mana (8, with 1 regen per 40 ticks).

## 8. Rescue, respawn and restart

| Event | Rule (all inherited from the Juicer unless marked) |
|---|---|
| Party defeat (D1) | after 50 ticks, both heroes are back at full HP at the arena checkpoint (inside the gate). The ball is at 24 HP, Dormant at its spawn; the weight and post reset; **the siege counter is 0** (new) |
| Quit or relaunch mid-fight | the run resumes at the start of room `bos` with the ledger at the mastery pin (the run's normal per-room relaunch) |
| Ball defeated, exit not yet taken | no respawn: the arena stays cleared until the exit |
| Exit taken | room `bos` completes, `next_room` returns FloorEnd, and the floor-end panel shows (section 9.5) |

## 9. Floor exit, gate, fixtures and eval

### 9.1 The degenerate strategy

Without the siege, a stationary Donut in an alcove casts at the idle ball with zero risk: 24 HP at about 1 per 40 ticks is about 960 ticks. The siege makes this lose:

- a full siege cycle takes about 260 ticks (120 to commit, 36 tell, travel, 8 rebound, 66 stun);
- each siege deals 2, so a 4 HP stationary Donut falls after 2 sieges (about 520 ticks), having dealt about 13 damage.

A gate is warranted.

### 9.2 Fixtures

| ID | Test | Expected | Feasibility |
|---|---|---|---|
| F1 | data validates; `builtin()` (Juicer) is equal to the pre-BOS value; every Juicer boss fixture and golden is byte-identical | pass | the new fields are optional with defaults; the siege-gated hashing |
| F2 | the manifest has 18 rooms ending in `bos`; `next_room(mastery) = Room(bos)`; `next_room(bos) = FloorEnd`; the new `MANIFEST_HASH` | pass | data plus the test retarget (section 10) |
| F3 | every ChargeTell lasts exactly 36 ticks with a locked target; the warning flag is set on exactly the tell ticks | pass | the Juicer's tell machinery |
| F4 | scripted Carl at 320, ball at 448: Carl runs left from tell tick 0 and takes 0 damage; the mirror case does the same | 0 damage | 6.2: a 28-tick margin |
| F5 | Carl stands at 320 through the roll | takes exactly 1 (ram), once | the ram hits each hero at most once |
| F6 | bait crash with the target in the alcove at pivot 110 (mirror 530) | 0 damage; `BossCrash{wall}`; Rebound 8, Stun 66 | the arch is on the path; BOS-AD25 clearance |
| F7 | punish: leave the alcove at Stun start and jab | >= 2 hits before Stun ends | 6.2: about 58 free ticks |
| F8 | siege: one hero parks in the left alcove, the other is dead or parked too | warning at 60, commit at 120, debris 2 to alcove heroes only, counter reset | 6.4 |
| F8b | F8, but the hide starts directly after a left crash (ball at 216) | the ball stops at the warning (about 276) and the siege still commits and crashes | BOS-AD27 guard bypass |
| F9 | the hero steps out between the warning and the commit | no siege; counter 0 | 6.4 |
| F10 | the weight: 3 post hits during the left-arch stun | ball -4 HP, pinned 90; alcove heroes untouched; one-shot; reset on restart | Juicer rack with data offsets |
| F11 | solo clear: `carl_bait_punish` with the bot follower | defeat in **1200..6000** ticks; Carl >= 1 HP; bot >= 1 (M1) | about 5 cycles of about 300 ticks plus shove openings |
| F12 | party defeat, then restart | 24 HP, Dormant, weight reset, siege 0, full HP | Juicer restart plus the counter |
| F13 | after defeat, wait 90, walk out at x >= 608 | room exit; then the FloorEnd panel with the new heading | Juicer exit with `hero_right_wall_x` |
| F14 | Ball defeat in the `bos` room | no grant; `brute_boss_f01_defeated` is not re-fired; `TABLE_HASH` unchanged | SceneKind F01Late (BOS-AD13) |
| F15 | over F11's run | phase stays 0; no Roar, Shockwave or DoublePound event | `queue_roar: false` |
| F16 | run smoke: `scripts/f01_run_smoke.sh` lists 18 rooms, all `status=ok`; plus an in-process `on_exit(bos)` test that reaches `floor_end_panel` | pass | the smoke runs each room separately; a `bos` pilot in `src/room_pilots.rs` |
| F17 | a hero at the alcove edge (pivot 110, mirror 530) through a bait crash on each side | 0 damage both sides | BOS-AD25: front overreach <= 4.3 px, hero edge 122 vs box 124 |
| F18 | PLAYTRACE: record and replay a `bos` run | round-trip identical, label `("boss", Some("bos"))` or a new source, Ball fixture loaded; Juicer replay goldens unchanged | `src/playtrace/replay.rs` gains the Ball source (R3) |

### 9.3 Gate `bos_eval` (STATIONARY, BOS-AD22)

The harness copies `trog_eval` and `corral_eval`. Each setup first walks both heroes past `engage_x` (so the gate is Closed), then holds its policy for 7200 ticks.

| Row | Setup | Pass condition |
|---|---|---|
| G-0 (control) | `carl_bait_punish` (F11) | it **must clear**. If it does not, the gate fails: the boss would be unbeatable, and the stationary rows would pass vacuously |
| G-1 | Donut stationary cast in the left alcove; Carl idle in the left alcove | no defeat with party HP loss <= 1 |
| G-2 | the same in the right alcove | the same |
| G-3 | split: Donut in the left alcove casting, Carl idle in the right | the same |
| G-4 | Donut stationary cast in the lane at x 200 | the same |
| G-5 | Carl stationary jab at the left alcove mouth (pivot 110) | the same |
| G-6 (solo) | the player is Donut casting from the left alcove at pivots 50, 80 and 100 (three runs); Carl is the **live follower bot** with M1 | the same |
| G-7 (solo) | the mirror of G-6 in the right alcove (pivots 590, 560, 540) | the same |

- **Validity (BOS-AD23)**, which is the troglodyte lesson. A row is **invalid, and the gate fails**, if any of these holds:
  - the gate never Closed;
  - the policy dealt fewer than 6 boss damage **and** took no ram, shove or siege hit (a setup that never engaged proves nothing);
  - a scripted position is invalid: an alcove pivot must be within 40..110 or 530..596.
- **Why the rows cannot pass vacuously.** G-0 proves that the boss can be killed. The engagement check proves that each stationary policy really traded hits.
- **Expected.** G-1 to G-3: siege debris. G-4: rolls and shoves. G-5: pivot 110 is inside the left alcove, so the ball sieges it. G-6 and G-7: either the bot's ram damage plus siege debris on Donut, or, if the bot keeps the counter at 0, BOS-A switches on BOS-Q11 and records it.

### 9.4 Flags

There are none. The behaviour is on by default ("ends on main"). The room is the run's last room, and `--scene f01_ball` runs it standalone.

### 9.5 Floor exit and floor-end panel (BOS-AD24)

- The right wall opens after defeat, and the exit rect behind x 608 completes the room.
- `next_room(bos)` returns FloorEnd. `floor_end_panel` changes:
  - the heading becomes **"FLOOR 1 CLEARED"** when the last completed room is `bos`;
  - a fact line is added, built from data: `"<hud.display_name> is down. The stairwell is open."`;
  - the existing tally, held and recording lines and the hint "Enter, A or Start: quit" do not change.
- The panel can only be reached through the exit, and the exit opens only on a real defeat. #192's "exit opens once on a real defeat" holds without a new ledger fact (see BOS-CUT9).

## 10. Pins

**BOS-A may move:**

- `MANIFEST_HASH` (currently `0x69da_957f_05d9_2834`), re-pinned with the room added and the `status` appended;
- **no** PLAYTRACE data pin: `data_pins()` carries no Juicer data pin, so the Ball gets none either (adding a key would re-stamp every recording);
- the floor-end tests that assume `mastery` is last: `next_room(mastery) == FloorEnd` and the last-room floor-end tests in `src/f01_run.rs`. These are retargeted to `bos`, not deleted.
- **test data, not hashes (BOS-AD32):**
  - the manifest order lists in the `src/f01_late.rs` tests gain `bos`;
  - the `room_index` counts in `src/f01_run.rs` tests (`(1, 17)`, `(2, 17)`) become 18;
  - the playtrace room-label tables in `tests_f01_late.rs` gain `bos`;
  - `room_exit_art/tests.rs` gains `bos` or a recorded exemption;
  - `scripts/f01_run_smoke.sh`: the room-count default 17 becomes 18, and the `F01_BOS_SMOKE_OK` marker requirement covers `bos` (the Ball emits it) or is exempted like `juicer`.

**BOS-A must not move:**

- `TABLE_HASH` (no new grant or event);
- `SCOPED_PIN_KEYS` (no new key);
- `brute_boss_f01.json` bytes;
- every Juicer fixture, golden and `COMBAT_STATE` hash;
- the CP-03 hashes;
- every other room's PLAYTRACE golden.

If one of these moves, BOS-A stops and reports. It does not re-pin.

## 11. Open questions (each with an adopted default)

| ID | Question | Adopted default |
|---|---|---|
| BOS-Q1 | Ball HP | 24 (the top of #192's range, so the fight is not over in two cycles) |
| BOS-Q2 | Roll damage 1 or 2 | 1, with launch. Carl's only dodge is the alcove, so a hit costs a quarter of his HP |
| BOS-Q3 | Siege rule, or #192's BSW-14 "search" behaviour | the siege: it is cheaper and gives an asserted anti-turret result |
| BOS-Q4 | Should the bot flee into alcoves? | no in BOS-A (M1 covers it); BOS-B adds it |
| BOS-Q5 | Floor-end heading | "FLOOR 1 CLEARED" |
| BOS-Q6 | Keep `mastery` before the Ball? | yes (#192) |
| BOS-Q7 | A `f01_ball_defeated` ledger fact or pin | deferred to BOS-B, because the exit already proves the defeat |
| BOS-Q8 | Siege warning text | data, "IT SNIFFS AT THE ARCH" (placeholder) |
| BOS-Q9 | Should the weight re-arm within one attempt? | no: once per attempt |
| BOS-Q10 | Should the arch crash hurt the ball (#192 "gate impact 3..4")? | no in BOS-A; the weight carries the environmental damage. BOS-B may add `crash.self_damage` |
| BOS-Q11 | Should the siege counter ignore the M1 bot in solo? | only if G-6 or G-7 fails: then the counter treats the M1-protected bot as unreachable while the player-controlled hero is in an alcove (6.4) |

## 12. Cuts and deferrals from #192

| ID | #192 item | In BOS-A | Goes to |
|---|---|---|---|
| BOS-CUT1 | 1200 px arena and camera pullback | the locked 640 px arena | BOS-C |
| BOS-CUT2 | the central suspended gate (Raised..Ascending) | the abstract arches plus the chain weight | BOS-C |
| BOS-CUT3 | the optional outer support | the chain weight stands in for it | BOS-C |
| BOS-CUT4 | Tusklings (at most 3 live, budget 4..6, Carl's kick-stagger) | none | **BOS-B** |
| BOS-CUT5 | L0..L4 reliefs and elevated routes | a flat floor | BOS-C |
| BOS-CUT6 | roll travel of 320..480 px | at most 256 (the arena width) | BOS-C |
| BOS-CUT7 | the six-frame native roll, tells and stun art | a greybox body with a label | **BOS-ART** |
| BOS-CUT8 | intro and exit cinematics (E0, E6) | a notice plus the panel | BOS-C |
| BOS-CUT9 | the `f01_ball_defeated` pin | the panel derives the defeat from the exit | BOS-B |
| BOS-CUT10 | gate-impact damage to the ball | none | BOS-B (BOS-Q10) |
| BOS-CUT11 | the BSW-14 search AI | the siege (6.4) | replaced by the siege; not deferred |
| BOS-CUT12 | policies `carl_environment`, `duo_coop` | G-0 and F10 cover the beats | BOS-B |

**Contradiction flagged.** #192 asks for a central gate as the main impact surface. This plan uses side arches instead. A single central arch in a 640 px arena would leave no run-up on either side of it. Restoring the central gate depends on BOS-C's wider arena.

**Second deviation.** #192 says refuges "may remain safe from direct Ball impact". They are safe from the bait roll (F6, F17) but not from the siege debris. The siege is telegraphed and on screen, which fits #192's "readable way back" and "alter approach, not nerf missiles".

## 13. Implementation split

| PR | Scope | Tier |
|---|---|---|
| **BOS-A** | everything in sections 3..10 and 17: the data file and loader; the optional fields (`hero_right_wall_x`, `siege`); the siege-gated reachability and hashing; `RoomScene::Ball`; the manifest row; arena selection by data; greybox arches, post and ball; the panel; the room-18 run, smoke and replay plumbing; F1..F18; `bos_eval` G-0..G-7; the packet update | Sonnet, one PR |
| BOS-B | Tusklings on ENEMY-SLOT; bot alcove-flee; the ledger fact; crash self-damage | Sonnet, after a top-tier mini-spec |
| BOS-ART | SWINE-01 runtime export and presenter, arch and weight props from existing kits | art pipeline |
| BOS-C | the wide arena, camera pullback, central gate, reliefs, cinematics | needs a top-tier design |

## 14. Packet update (`docs/04-production/f01-playtest-ready-01.md`)

BOS-A edits these lines (the line numbers are on main at f8f489de):

- **3:** "the 17 rooms" becomes "the 18 rooms".
- **7:** "The floor ends at the mastery arena. There is no floor exit and no Ball of Swine yet" becomes "The floor ends at the Ball of Swine arena (greybox), whose exit leads to the floor-end panel".
- **77:** the `mastery` row's "Last room: the floor ends here." becomes "Leads into the Ball of Swine."
- **A new room-table row 18** for `bos`, with what to try: bait rolls into the arches, the chain weight, the siege.
- **83:** remove section 3 item 1 (Ball of Swine and the floor exit missing), or reword it to the BOS-B/BOS-ART/BOS-C deferrals.
- **112:** the mastery checklist item about the floor-end panel moves to a new `bos` checklist item.
- **216:** the #192 row "nothing / the whole floor finale and exit" becomes "BOS-A greybox; Tusklings, art, wide arena and cinematics deferred".

## 15. Art needs (BOS-ART stub)

These reuse what exists. No new concept art is needed.

- **SWINE-01:** export the existing `swine-01.ase` (176x144 frame, pivot 88,143) to an atlas with the roll (6 native frames, never rotated or scaled), roll tell, impact and stun. Wire it as the ball's presenter, replacing the greybox. This is execution work for the medium tier. The keys exist.
- **Arches and weight:**
  - arch posts and lintels come from the catacomb kit A/B/C (`docs/02-art/floor-01-catacomb-kit.md`);
  - a chain-weight prop replaces the rack presenter (a small top-tier key plus medium-tier execution);
  - the stairwell exit is drawn behind the right wall.
- **Direction** comes from [the arena concept](../02-art/floor-01-ball-of-swine-arena-concept.md): concentric masonry, brass and iron, limited magenta UV.
- **Not needed for BOS-A:** TUSK-01 (that is BOS-B), the wide arena and camera (BOS-C).

## 16. Horizon, tone and rights

- **Horizon fit:** the plan fits [the progression horizon](character-progression-horizon.md). Item 7 there says "Ball of Swine / exit: mastery test combining known actions and geometry, **not** a final barrage of new abilities".
  - BOS-A adds no ability, input or unlock.
  - The fight uses run, jab, Magic Missile, the swap and a player-triggered environmental hazard against telegraphs.
  - It keeps "environmental attribution" and "vulnerability/risk" (the weight, and the punish after an impact).
- **Tone:** the anti-hero rule is unchanged. The ball is grotesque and absurd, not heroic. The heroes win by hiding behind a door frame and dropping a weight on it, which is a plain-guy solution.
- **Rights:**
  - "Ball of Swine", "Tusklings", Carl and Donut are the replaceable fan-concept layer. They appear only in data strings and docs.
  - Engine identifiers are neutral: `rolling_boss_f01`, `RoomScene::Ball`, `siege`.
  - No new art enters in BOS-A.

## 17. Review fixes (#315 review)

| ID | Change | Finding |
|---|---|---|
| BOS-AD25 | alcoves are pivot <= 110 and >= 530 (reachable 111..529), from the crash-tick ram overreach of ceil(4.33) = 5 px plus 1; escape margin restated as about 27 ticks; F6, F17, G-5 and the validity ranges follow | R1, R10 |
| BOS-AD26 | solo gate rows G-6 and G-7 with the live M1 bot; fallback BOS-Q11 | R2 |
| BOS-AD27 | the siege stops the walk at the warning and bypasses `min_target_px`, `min_room_px` and `cooldown`; F8b | R4 |
| BOS-AD28 | `inside_x` and `regroup_x` 112, so no hero starts the fight in the left refuge | R5 |
| BOS-AD29 | the shove's strike can splash the alcove mouth; only its trigger needs a reachable target | R6 |
| BOS-AD30 | all four body boxes, `push_px_per_tick` 6 and `min_side_room_px` 28 | R7 |
| BOS-AD31 | no Juicer emblem, sprite or death presenter for the Ball; the `rolling_brute` species has no death profile and uses the generic fallback (accepted for the greybox) | R8 |
| BOS-AD32 | test data that may move without a stop: the order lists, room counts, label tables, exit-art test and smoke script | R9 |
| BOS-AD33 | the room-18 run, smoke, replay and ledger plumbing (work order, F16, F18) | R3 |
| (nits) | weight centre 209 (R11); second #192 deviation (R12); CUT11 cell and the panel's data source (R13) | R11..R13 |
