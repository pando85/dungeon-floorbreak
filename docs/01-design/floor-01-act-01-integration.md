# Floor 01 Act I in the floor run (ACT1-PLAN)

**Status:** design and integration plan. **ACT1-A** (the run plumbing, the shore and the guild) and **ACT1-B** (the chase and the opening profile) are implemented as labelled playable greybox (section 17). Nothing here is playtested or accepted. After its implementation PRs merge, the status is "merged, game-feel review pending".

**Owner decision (2026-11-25, cited for every adopted default below):** "Go with all because I could not test it until next week" and "Ensure that everything ends on main for testing all together next week". Every owner-gated Floor 1 item is authorized under adopted defaults recorded by ID. Everything must be reachable in `--scene f01_late --f01-run` in story order. New behaviour is on by default where the spec allows. Large unfinished items land as a labelled playable greybox.

**What this plan does.** It puts Act I (beats I-01 to I-04 of the [Act I combat beats](floor-01-act-01-playable-combat-beats.md)) at the **start** of the floor run, before the Infernal Corral. The rooms are:

- the entry and the Murder Dozer chase (CHASE-01, choreography from the owner draft #179, carried onto main as [the Dozer opening choreography](floor-01-dozer-opening-choreography.md));
- Carl's first revenge on the goblins, with the rotten shore ([I-03](floor-01-first-goblin-encounter.md));
- the Tutorial Guild, where Donut gets the biscuit (**GUILD-BEAT-01**).

The plan reuses the merged DOZER-CORE machine of room 3 ([Dozer gameplay note](enemy-gameplay-dozer.md)) with a second profile. It does not fork it.

**Read with:**

- [the Dozer opening choreography](floor-01-dozer-opening-choreography.md) and its [work order](../07-agents/floor-01-dozer-opening-workorder.md);
- [ENV-02](../02-art/env-02-opening-spec.md) (option R, the segment and marker baseline) and [DOZ-01 section 9](../02-art/doz-01-dozer-spec.md) (the machine's timing proposals);
- [the Dozer gameplay note](enemy-gameplay-dozer.md) section 4.4 and section 7;
- [the integration plan](../07-agents/f01-playtest-integration-plan.md) section 17;
- [the REST plan](floor-01-quiet-setpieces-and-transitions.md) section G, the pattern this section copies.

## 1. What main holds today (checked at `2aff8a70`)

- **The run.** `--f01-run` walks the listed rooms of `assets/levels/f01_late/manifest.json` in row order. It is one room per launch: on a room exit it shows a facts-only room-clear panel, and on confirm it relaunches with the next room and the carried story ledger. The first room is `manifest.rooms[0]`. Twelve rooms are listed: corral, rest_refuge, hoarder, boiler, dozer, hazards, ring_court, ring_split, rest_camp, juicer, rest_aftermath, mastery (REST-WIRE PR 2 listed the camp and the aftermath). Room and sidecar `status` strings name no run position since that PR, so inserting rooms ahead never rewrites them.
- **CORRAL-F** (`Manifest::validate`): "the corral (room 0) must be the first listed room". The corral scene runs room `corral` only. AUDIENCE-RUN goes live in every room listed **after** the corral (`AUDIENCE_UNLOCK_AFTER`). The rule is positional, so rooms listed before the corral have no audience with no code change.
- **Story-less rooms.** The corral has no story runtime (KC11). Its sidecar `story_pin` is a label with no table row. `carry_for` returns an empty ledger snapshot for the corral under the Story loadout, and `carry_story` accepts an empty snapshot in a story-less room.
- **The biscuit already exists in code.**
  - `DonutSetup { start, guild_zone }` (`src/donut_mana.rs`) holds Donut's mana start state and an optional guild zone. When Donut's pivot enters the zone's x range, `ManaEvent::UnlockGranted` fires and `primary_ranged` unlocks. The unlock survives a defeat retry; a fixture restart restores the declared start.
  - The HUD already switches Donut's prompt from the pre-biscuit swat to the missile (`DonutMagic::PreBiscuit`). Her AI never casts before the biscuit (CF02-Q17).
  - `primary_ranged` is a **read-through** capability: it is read from Donut's mana, never stored in the ledger.
  - The manifest's story order holds a **point** `guild_biscuit` that grants `primary_ranged` with `trigger: null`. `check_story_order` accepts that for a read-through capability, so `story_unlocks_v0.json` (`TABLE_HASH`) does **not** need to change.
- **The shore (CP-03).** `Fixture::shore_dx` enables the rotten-shore strike column relative to Carl's start column, with no new code (`f01_mastery` uses it). The rules are:
  - three melee hits, then a 6-tick delay;
  - a 140 px fall onto the footprint +25..+105;
  - a credited defeat for a goblin under the block;
  - the landed block does not collide.
- **DOZER-CORE** (`src/dozer.rs`, `src/combat_scene/dozer.rs`) is room 3's revenge machine.
  - Data: tuning `enemy_dozer_v0.json` with profile `revenge` and `dzq10_charge: null`.
  - States: `Dormant` → `Approaching` (rumble) → `CommittedChase` (speed band) → `Escaped(Brake, Strain, Idle)`. Also `ImpactStunned` and `Disabled` for the stop frame.
  - Contact: a lethal wheel (r 44) and a hull push box.
  - The solo rescue `SlotOut::place` uses a lane spot, then a tunnel spot (section 7.3).
  - Events are codes 1..12, each hashed.
  - The sidecar `dozer` block (`deny_unknown_fields`) holds the trigger x, the idle-wake ticks, the emergence front, `stop_x` and a required `dzq22_lintel`.
  - The encounter validator (`src/encounter/haz.rs`) enforces room 3's tunnel rules. Rule 2 says no tall solid on the chase lane; rule 4 requires a lane ceiling and the lintel.
- **Not on main.**
  - The ENV-02 route is not wired into any Bevy scene.
  - No room draws the guild interior in gameplay. The `refuge_guild` scene is a separate non-gameplay screen with marks: Donut 226, Carl 258, Mordecai 380 (`mordecai_h44`).
  - The game has no runtime audio path. `floor01_dozer_chase.ogg` and `murder_dozer_engine_sketch.ogg` exist as provisional files.

## 2. The decisions in one place

1. **Three Act I rooms precede the corral:** `act1_chase` → `act1_shore` → `act1_guild` → `corral` → … (ACT1-AD1).
   - `act1_chase` covers I-01 and I-02: ENV-02 S1 to S5's arch, route x 0..3008.
   - `act1_shore` covers I-03: S5 to S6, route x 2608..3584, re-based to local x 0.
   - `act1_guild` covers I-04: a short interior greybox.
   - All three are milestone `greybox`, run under `cf02_rules`, are story-less like the corral, and have no audience.
2. **The chase is a scripted escape room on DOZER-CORE.**
   - It uses a second profile `opening` from a **new** tuning file, so room 3's `TUNING_HASH` does not move.
   - It appends a charge sub-machine (tell, charge, overshoot brake, strain, resume), an authored safe **pocket**, a barricade **lock** that only the machine opens, and a **wedge** end state.
   - The machine is never killed or stunned in the opening. Room 3's profile, fixtures and goldens stay byte-identical (Dozer note section 4.4).
3. **The room boundary at the post-chase checkpoint.**
   - The chase and the goblin fight are separate rooms that meet at ENV-02's `checkpoint_post_chase` (route x 2640).
   - A death in the goblin fight therefore never reruns the chase (CHDIR-09). DOZER-CORE resets the machine to `dormant` on every respawn, so one room holding both could not keep a wedged machine behind a respawning party.
   - The price is one room-clear panel between the escape and the fight (ACT1-AD2, ACT1-Q1).
4. **GUILD-BEAT-01.**
   - The story-order point `guild_biscuit` becomes the room row `act1_guild`. That row grants `primary_ranged` (read-through, `trigger: null`), so `TABLE_HASH` is unchanged.
   - In the room, Donut starts locked with a guild zone in front of the guide's mark. A new lock `donut_unlocked` seals the exit until her mana is unlocked.
   - The existing swat-to-missile prompt switch is the payoff, plus a new `MAGIC MISSILE` notice (ACT1-AD12).
5. **Two implementation PRs** (section 12).
   - **ACT1-A:** the run plumbing, the shore room and the guild room.
   - **ACT1-B:** the chase room and the opening profile.
   - ACT1-A lands first and lists `act1_shore` and `act1_guild` before the corral. ACT1-B lists `act1_chase` in front of them.

## 3. Run order

| Run index (after ACT1-B) | Room id | Scene | Title (ACT1-AD3) | Milestone | Story pin at entry | Grants in play | PR |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **0** | **`act1_chase`** | **`f01_act1_chase`** | **The First Chase** | **greybox** | **none (`f01_pin_act1_chase_entry`, no table row)** | **none** | ACT1-B |
| **1** | **`act1_shore`** | **`f01_act1_shore`** | **The Rotten Shore** | **greybox** | **none (`f01_pin_act1_shore_entry`)** | **none** | ACT1-A |
| **2** | **`act1_guild`** | **`f01_act1_guild`** | **The Guild Door** | **greybox** | **none (`f01_pin_act1_guild_entry`)** | **`primary_ranged` (the biscuit, read-through)** | ACT1-A |
| 3 | `corral` | `f01_corral` | The Infernal Corral | greybox | none | none |  |
| 4 | `rest_refuge` | `f01_rest_refuge` | The Wrong Restaurant | greybox | none | `jump_reach_boost` |  |
| 5.. | hoarder, boiler, dozer, hazards, ring_court, ring_split, rest_camp, juicer, rest_aftermath, mastery | unchanged | | | | | |

Run length:

| State of main | Listed rooms |
| --- | --- |
| Today (REST-WIRE PR 2 merged) | 12 |
| After ACT1-A | 14 |
| After ACT1-B | 15 |

The pacing map of the [REST plan section D](floor-01-quiet-setpieces-and-transitions.md#d-connected-floor-01-pacing-map-adapted) already places "Opening / Murder Dozer → first goblins → Guild / biscuit" before the corral as slot S0. This plan fills slot S0 (REST-Q8, partially answered).

## 4. Rooms (greybox, room px, y down, floor 580, height 660, 16 px grid)

All three rooms:

- **Files:** `floor01-demo-hall-v1` room files plus `floor01-encounter-v1` sidecars in `assets/levels/floor01/`, composed with the existing kit (`f01_boiler::compose_kit`, the REST pattern). No new art.
- **Room fields:** `setting` `dry-catacomb`, `act` 1, `platform_slab_px` 12, protected spawn `{16, 484, 208, 96}`, and spawn Carl 112, Donut 80 unless stated.
- **Route mapping:** ENV-02 layout y maps to room y with +272 (the layout walking line 308 is room floor 580).

### 4.1 `act1_chase` (I-01 entry and I-02 chase)

Width **3008**. It is the route from S1 to the S5 arch, x 0..2944, plus a 64 px exit apron. The x values are ENV-02's.

| Piece | Data | Source / rule |
| --- | --- | --- |
| S1 false calm | x 0..640, spawn Carl 72, Donut 48 (ACT1-AD4) | ENV-02 `spawn` 72. No threat and no idle wake (ACT1-AD7) |
| S2 window | x 640..944. The machine emerges with its wheel front at **888**, the window's right edge (`pursuer_spawn` [696, 70, 192, 218]) | DZ-Q13 `dzq13_emerge_front_x` 888 |
| Checkpoint | `{984, 484, 48, 96}` (centre 1008) | ENV-02 `checkpoint_chase` 1008 |
| Trigger | `dzq11_trigger_x` **1072**, crossed by the **rearmost** living hero (ACT1-AD6) | ENV-02 `trigger_chase_start` 1072 |
| S3 low ceiling | Decor only: the soffit `{976, 272, 704, 96}` (underside y 368, 212 px clear). It is not a solid; no jump reaches it (ACT1-AD8) | ENV-02 option R |
| Timber sets | Decor plus events: `timber_a` at x 1104, `timber_b` at x 1424, each `{x, 456, 80, 104}`. Not solids | ENV-02 `timber_set_a/b` |
| Humps | Sidecar statics of kind `solid`: `hump_a` `{1168, 564, 64, 16}` and `hump_b` `{1456, 564, 64, 16}` (ACT1-AD17) | ENV-02 humps, one tile |
| Charge arm | `arm_x` **1888** (data, opening tuning) | ENV-02 charge trigger |
| Pocket ledge | Sidecar static of kind `solid`: `alcove_ledge` `{2064, 564, 96, 16}` (ACT1-AD17). Pocket rect `{2072, 460, 80, 104}` (feet on the ledge top y 564, 104 px clear) | ENV-02 `safe_pocket_alcove` |
| Alcove decor | Decor `alcove` `{2072, 460, 80, 104}`: a lit recess drawn **behind** the heroes, the brightest shape in S4, so the player reads the pocket as the way out (#179 C4) | ENV-02 `safe_pocket_alcove` |
| Barricade | Lock `barricade`: solid `{2312, 304, 80, 276}`, `opens_on: dozer_contact` (section 5.4). Drawn as the ENV-02 full-height barricade x 2304..2400 | ENV-02 blocker [2312, 32, 80, 276] |
| Wedge | `dzq14_stop_x` **2856** (the S5 arch's right edge). The machine wedges there (section 5.4) | ENV-02 wedge [2664, 70, 192, 218] |
| Exit | `{2928, 484, 64, 96}` | It lies past `stop_x` + 48 + 16 (the past-stop rescue spot, DZ-Q17) |
| Roster | One `dozer` entry, x 888, beat `I02`. No riders, no goblins (ACT1-AD9) | |
| Goal | `reach_exit`. `max_living` 1 | |
| Platforms | None: a 16 px room platform fails the lane rule (`PlatformBlocksLane`, REST-IMP-8: 52 px clear under a 12 px slab), so every low step is a lane static | |

Sidecar `dozer` block (section 5.6 has the new fields):

```json
"dozer": {
  "profile": "opening",
  "dzq11_trigger_x": 1072,
  "dzq11_trigger_by": "rearmost",
  "dzq11_idle_trigger_ticks": 216000,
  "dzq13_emerge_front_x": 888,
  "dzq14_stop_x": 2856,
  "pockets": [{ "id": "alcove", "rect": { "x": 2072, "y": 460, "w": 80, "h": 104 }, "ledge": "alcove_ledge" }],
  "timbers": [{ "id": "timber_a", "x": 1104 }, { "id": "timber_b", "x": 1424 }],
  "wedge": true
}
```

`dzq22_lintel` is absent here: the opening has no tunnel. `ledge` names the id of the `solid` static the pocket stands on. The sidecar's `statics` list holds `hump_a`, `hump_b` and `alcove_ledge`.

### 4.2 `act1_shore` (I-03, Carl's first revenge)

Width **976**. It is the route x 2608..3584 re-based by −2608 (local = route − 2608).

| Piece | Data | Source / rule |
| --- | --- | --- |
| Entry | Spawn Carl 112, Donut 80. Checkpoint `{32, 484, 48, 96}` (route 2640, `checkpoint_post_chase`) | Retrying here never replays the chase |
| Wedged machine | Decor `wedged_dozer` `{56, 440, 192, 140}` (route 2664..2856): the machine stuck in the arch, wheel spinning, not lethal, not in the sim (ACT1-AD10) | ENV-02 wedge |
| Rotten shore | `Fixture::shore_dx` = **255** (strike column local 367 = route 2975, minus Carl's spawn 112). The fall footprint is local 392..472 (route 3000..3080) | ENV-02 / CP-03 |
| Goblins | `gob_a` kind `goblin`, x 528 (route 3136, spawn A), temperament `brawler`. `gob_b` x 568 (spawn B), temperament `coward`, so it arrives late (I-03 G1). Spawn C (local 608) stays reserved (ACT1-Q4) | I-03 section 1, 2 |
| Goal | `defeat_roster`. The exit opens when both goblins are defeated, by any credited cause: melee, kick contact or the shore block. `max_living` 2 | I-03 |
| Exit | `{728, 484, 64, 96}`: the guild portal at local 760 (route 3368), inside the protected exit (local ≥ 624) | ENV-02 `exit_guild` |
| Platforms | None | |

### 4.3 `act1_guild` (I-04, GUILD-BEAT-01)

Width **640**, `ceiling_px` **228** (an interior; the REST refuge's slab rule).

| Piece | Data | Rule |
| --- | --- | --- |
| Entry | Spawn Carl 112, Donut 80. Checkpoint `{64, 484, 48, 96}` | |
| The guide | Decor `guide` `{368, 536, 24, 44}`, centred on the `refuge_guild` Mordecai mark (x 380). The binary may draw the existing `mordecai_h44` sprite there; a labelled rectangle is the fallback (ACT1-AD11) | Presentation only, never in the sim |
| Guild zone | `DonutSetup.guild_zone` = **(336, 376)**: Donut's pivot entering x 336..376 grants the biscuit | Donut's existing unlock trigger |
| Door lock | Lock `guild_door`: solid `{528, 336, 16, 244}`, `opens_on: { "event": "donut_unlocked" }` (section 6.4). Its top meets the underside of the 228 ceiling slab (y 336) on purpose, so no jump clears it. The REST staff door is `y 352, h 228`; both end at the floor 580 | Carl cannot leave without Donut's biscuit, even when the companion trails |
| Exit | `{560, 484, 64, 96}` | |
| Roster | Empty. Goal `reach_exit` (REST's empty-roster rule) | |
| Text | None. No dialogue, no quotes, no tutorial screen. The beat is the walk past the guide, the notice and the changed prompt (I-04: brief, noncombat) | Rights |

## 5. The opening Dozer: profile `opening` on DOZER-CORE

### 5.1 Data

The new file `assets/data/gameplay/enemy_dozer_opening_v0.json` holds the opening profile. It has its own `OPENING_TUNING_HASH` and pin key `dozer_opening_tuning`. The shared machine values stay in `enemy_dozer_v0.json` (wheel, push box, rumble, brake, rescue constants), whose `TUNING_HASH` does **not** move.

| Key | Value | Source |
| --- | --- | --- |
| `dzq06_cruise_px_s` | 160 | DOZ-01 section 9 |
| `dzq07_catch_up_px_s`, `dzq07_catch_up_gap_px` | 185, 260 | DOZ-01 |
| `dzq08_mercy_px_s`, `dzq08_mercy_gap_px` | 150, 96 | DOZ-01 |
| `dzq09_accel_px_s_per_tick` | 4 | DOZER-CORE |
| `dzq25_no_hero_ahead_target` | `cruise` | DOZER-CORE |
| `dzq10_charge.arm_x` | 1888 | ENV-02 |
| `dzq10_charge.min_gap_px` | 140 | DOZ-01 gate |
| `dzq10_charge.tell_ticks` | 36 | DOZ-01 |
| `dzq10_charge.speed_px_s` | 240 | DOZ-01 (K5 ≤ 185 does not apply to a charge, Dozer note section 4.4) |
| `dzq10_charge.max_ticks` | 90 | DOZ-01 |
| `dzq10_charge.brake_ticks`, `brake_slide_px` | 24, 4 | DOZ-01 overshoot |
| `dzq10_charge.strain_ticks` | 90 | DOZ-01 |
| `dzq10_charge.reach_margin_ticks` | 20 | ACT1-AD5 (ENV-02 measured 17 for the hop) |
| `dzq10_charge.charges_per_attempt` | 1 | ACT1-AD5 |

The #179 draft's "130 px/s against 145" are stale ENV-02 hypotheses. Carl runs 185 on main, and DOZ-01's values above replace them.

**Profile loading.** `Dozer::new` keeps calling `DozerTuning::active()` for room 3. The existing constructor `Dozer::with_tuning(tuning, site, exit_left, floor_y)` takes an explicit tuning, and `combat_scene/dozer.rs::build` switches to it, choosing the tuning from `site.profile`: `revenge` passes `DozerTuning::active()` exactly as `Dozer::new` does today, `opening` passes the tuning loaded from `enemy_dozer_opening_v0.json` (shared machine values from `enemy_dozer_v0.json`, opening values from the new file). Room 3's construction path, `TUNING_HASH` and fixtures are untouched.

### 5.2 States

The opening adds **no** top-level `DozerState`. `CommittedChase` gains a sub-state that exists only when the profile has a `dzq10_charge` block:

```mermaid
stateDiagram-v2
    [*] --> Dormant
    Dormant --> Approaching: rearmost hero x >= 1072 (rumble 90)
    Approaching --> Cruise: emerge at front 888
    state CommittedChase {
        Cruise --> Tell: commit rule (5.3), once per attempt
        Tell --> Charge: 36 ticks (speed held), ChargeCommitted, speed set to 240 at once
        Charge --> Overshoot: 90 ticks without BarrierImpact (unreachable while rule 4 holds)
        Overshoot --> Strain: 24 ticks, slide 4
        Strain --> Cruise: 90 ticks
    }
    CommittedChase --> Escaped_Brake: front reaches stop_x 2856 (any sub-state, a charge past BarrierImpact emits ChargeEnded here)
    Escaped_Brake --> Escaped_Strain: 24 ticks
    Escaped_Strain --> Wedged_Idle: 90 ticks (event Wedged)
```

- **Charge.** At `ChargeCommitted` the speed is set to 240 px/s **instantly**: no `dzq09` accel ramp, so rule 3's `wheel_ticks` is exact. The machine moves on a straight line. It never steers or reacquires, and it has no homing.
- **Through the barricade (ACT1-AD18).** Once `BarrierImpact` has fired, `Charge` ignores `max_ticks` and runs at the charge speed to `stop_x`. Then room 3's `Escaped(Brake → Strain → Idle)` follows, plus `Wedged`. `ChargeEnded` is emitted at `stop_x`. So a normal run has exactly one `BrakeStarted`, and the wedge beat keeps the charge's momentum (#179 C7, "cannot turn and wedges").
- **Overshoot and strain (defensive only).** When the 90 charge ticks end before `BarrierImpact`, `ChargeEnded` fires and the machine brakes and strains in the lane as DOZ-01 steps 3 to 4 describe, the wheel lethal (`dzq14_lethal_in_brake`), then resumes cruise (step 5). Rule 4 of section 5.3 makes this unreachable in the opening room; it stays for a profile whose charge has no barricade in reach, and a domain unit test (ACT1-T11) covers it.
- **`stop_x`.** Reaching it ends every sub-state. The room-3 brake and strain follow, then `Idle` with the wedge (section 5.4).
- **Barricade.** The cruise or the charge opens the lock on contact and runs on (section 5.4).
- **No idle wake (ACT1-AD7).** `dzq11_idle_trigger_ticks` 216000 (an hour) keeps S1's false calm. A party standing still is not chased. Nothing ahead of the trigger can lock, so this cannot softlock.

### 5.3 The commit rule and the fairness math

#179 named the real risk: "leader crosses x 1888" kills a trailing P2. Its preferred experiment, policy 1, is adopted: the arm is visual only, and the commit needs **every** living hero (ACT1-AD5). `ChargeArmed` fires once when the **leading** living hero first reaches `arm_x`; it drives the windup presentation only. The tell (the commit) starts on the first fixed tick, during `Cruise` and while no charge has been used in this attempt, at which **all** of these hold:

1. **Every** living hero has `x ≥ arm_x` (#179 policy 1: both are in the preparation band).
2. The gap is **≥ 140** (the DOZ-01 gate). The gap here is the **minimum body left edge over all living heroes** minus the wheel front. It is not `Dozer::gap()`, which counts only heroes ahead of the axle and returns `None` otherwise; a hero behind the front gives a negative gap and fails the rule.
3. **Every** living hero can reach the pocket before the wheel does: for each hero `h`, `walk_ticks(h) + reach_margin_ticks ≤ wheel_ticks`.
   - `walk_ticks(h)` is the hero's horizontal distance to the nearest x at which its whole body lies inside the pocket's x span, divided by that hero's own run speed per tick (its `HeroMotion`). It is 0 when the body is already inside.
   - `wheel_ticks` is the number of ticks the wheel front needs to reach the pocket's left edge: 36 tell ticks at the current speed, then 240 px/s from the first charge tick (the speed is set instantly, section 5.2).
   - A hero past the pocket (waiting at the barricade) walks back. A hero behind the wheel front makes rule 2 fail.
4. **The charge reaches the barricade:** `front + 36 × v_now / 60 + dzq10_charge.speed_px_s × max_ticks / 60 ≥ barricade.x`, where `barricade.x` is the `dozer_contact` lock solid's left edge (2312). With `v_now` 160..185 px/s this needs a front of about **1856..1841**. On the tick the front reaches `barricade.x`, the impact test runs before the `max_ticks` test, so a charge that reaches it on its last tick still counts.

The commit window is therefore `[barricade.x − reach, pocket.x − 140]`: today front **1841..1932** at 185 px/s (1856..1932 at 160, 1862..1932 at the mercy speed 150). Validation rule O9 keeps it non-empty at the slowest band speed.

If the rule never holds, **no charge happens**. The cruise continues and the pocket protects at every speed. The cruise reaches the barricade, opens it and runs on to the wedge. So there is no forced lethal charge and no softlock, and no fail-open timer is needed. This replaces ENV-02's 8 s scripted-charge guard and #179's fallback 2 (ACT1-AD5, flagged in section 15).

Worked cases. The units are px and ticks at 60 Hz. Carl runs 185 px/s (3.08 px/tick). The body half width is 12. The pocket's x span is 2072..2152.

| Case | Wheel front | `walk_ticks` | `wheel_ticks` | Result |
| --- | --- | --- | --- | --- |
| **First run.** The rearmost trigger at 1072, the emergence gap about 449, held by the 185 catch-up; both heroes run into the pocket and stop. The front is about 1623 on their arrival, then closes at 185, then 160 once the gap is under 260 | commit at about **1841..1856** (rule 4 is the last to hold) | 0 | about 66 at both speeds (rule 4 pins the tell end at about 1952) | commits with gap 216..231. The tell ends near front 1952, the wheel passes the pocket on charge tick about 30 and hits the barricade on tick 90 at the latest, then runs on to `stop_x` |
| The rearmost hero at 1888, gap exactly 140 | 1736 | | | rule 4 fails (1736 + 96 + 360 = 2192 < 2312): no commit yet. If that hero then hides, the first-run row applies |
| Carl (185 px/s) waits at the barricade (x 2300), the other hero in the pocket | 1856 | Carl 52 (to 2140, the nearest in-pocket pivot, 2152 − 12) | 66 | rule 3 fails (52 + 20 > 66) and only gets worse as the front advances: no charge, the cruise path. Carl is safe only by walking back into the pocket before the cruise front reaches him (F14 otherwise). With Donut (207 px/s) the walk is about 46 ticks, at the rule-3 boundary (46 + 20 against about 65..66), so the fixtures put Carl at the barricade |
| Both at the barricade (x 2300), front already at 2160 | 2160 | n/a | n/a | rule 3 fails, no charge. The cruise reaches them unless they went back while the front was left of the pocket |
| P2 448 px behind the leader when the leader arms (P2 at about 1440) | follows P2 | | | the trigger and the band both follow the rearmost hero, so once both hide the first-run row applies unchanged; the leader's walk-back from the barricade fails as in the third row |

A charge from gap 140 never catches a hero who keeps running on an open lane (DOZ-01: the gap shrinks to about 72 over 90 ticks). The danger is the barricade ahead, and that is what makes the pocket the answer.

### 5.4 Pocket, barricade, wedge, timbers

- **Pocket (authored depth refuge).** A hero is *in the pocket* when its body's x span lies inside the pocket rect's x span (2072..2152) and its feet are at or above the ledge top (layout y ≤ 564, y down). A hero jumping inside the pocket stays protected.
  - **Coordinates.** The test is in layout (room) px, y down. `HeroView` positions are world px, y up, so the machine converts explicitly with `layout y = layout_h − world y` (the conversion `Dozer::nav_rows_into` already uses) before comparing; a unit test pins one hero standing on the ledge (in) and one on the lane in front of it (out).
  - The wheel test, the push-out and the AI rescue (section 5.5) all skip a hero in the pocket.
  - This is not invulnerability anywhere else, and it is not a generic cover rule.
  - The event `PocketEntered { hero }` fires the first time each hero enters per attempt.
  - The pocket fits both heroes: 80 px against two 24 px bodies.
- **Barricade lock (`LockEvent::DozerContact`).** The lock is open **iff** the room's dozer reports `barrier_hit()` (ACT1-AD19).
  - `barrier_hit` is a bool in `Dozer` state. The machine sets it in its slot step on the first tick its wheel front reaches the solid's left edge (front ≥ 2312) and emits `BarrierImpact`. It is mixed by `Dozer::mix_into` and reset by `slot_respawn` and by a fixture restart.
  - **Tick order (one-tick lag).** `step_party` runs the scene step (hero physics and collision), then `step_slot_enemies` (the machine), then `step_env` (the locks). So the machine reports `BarrierImpact` in its slot step, `step_env` opens the lock in the same tick, and heroes first collide with the opened state on the **next** tick. The presentation hides the lag. The machine ignores solids, so its own motion is unaffected. No global step is reordered.
  - There is **no** env-side `opened` flag and no extra hash term. REST's surface locks keep their flags across a respawn (`EnvRuntime::reset_keeping_partitions`); this lock has none to keep, so a defeat retry, a checkpoint respawn and a fixture restart all close it by construction (the machine is `dormant` again).
  - Only the machine opens it. Heroes cannot break it or jump it (276 px tall).
  - **At the closed barricade.** A hero standing at the closed barricade is reached by the wheel first (the wheel front leads the push box by 88 px): a `WheelHit` defeat and a checkpoint respawn, never a stall. `PinnedHold` applies only to a hero above wheel height inside the push box (ACT1-F14).
- **Wedge (`wedge: true`).** When the room-3 strain at `stop_x` ends, the machine enters `Escaped(Idle)` and the event `Wedged` fires. From then on:
  - the wheel is already off at `Idle` (room 3's `wheel_lethal`); the wedge also turns the push box and the nav rows off, so the heroes, who are *behind* the machine in this room (in the pocket), walk past it to the exit;
  - its clang hurtbox stays, so melee still sparks and does no harm.
  - The presentation draws the hull in the S5 arch window. The greybox clips and darkens the side-on rig, because the end-on art is ACT1-Q3.
  - In room 3 (`wedge` absent) `Idle` is unchanged.
- **Timbers.** `TimberSmashed { id }` fires once per id and attempt, on the first tick the wheel front reaches the timber's x. It drives the existing ENV-02 splinter burst (DR-D's presentation) and nothing else.
- **Humps and the ledge.** These are 16 px lane statics of kind `solid` (ACT1-AD17). Heroes hop over them (16 px; no step-up is assumed); the machine ignores solids, and the presentation may flatten the humps; the sim does not.

### 5.5 The AI partner (solo)

- No `BossThreat` (Dozer note R-4: the bot would jump into the wheel). The layer-1 nav rows stay, cut around the pocket rect, so the planner may stand in the pocket.
- The layer-2 rescue keeps room 3's rules with three changes while the profile has a charge block (ACT1-AD14).

**Who has a rescue cause.**

- `rescue()` **skips a hero in the pocket** (section 5.4), exactly as the wheel test and the push-out do. A bot waiting in the pocket is never relocated.
- A bot whose body right edge is ≤ `o + dzq02_push_box.x0 − dzq17 zone margin` (fully behind the hull) has **no** rescue cause. `BehindAxle` applies only while the bot's body overlaps the hull span, tested with the strict `Rect::overlaps`, so a bot that `push_out` left touching the hull's rear has no cause. So a bot placed behind the hull is not re-placed on the next tick.
- Room 3 (`revenge`) keeps today's causes unchanged.

**Which spot, before `BarrierImpact`.**

| Situation (a bot hero, solo, wheel lethal, a rescue cause as above) | Spot | Value |
| --- | --- | --- |
| The pocket's left edge is still ahead of the wheel front by at least the rescue band plus a body width | `RescueSpot::Pocket` (new, appended) | Centre of the pocket, feet on the ledge |
| Otherwise (the front is within the rescue band plus a body width of the pocket's left edge, or at or past it) | `RescueSpot::Behind` (new, appended) | `o + dzq02_push_box.x0 − dzq17 zone margin − HERO_HALF_WIDTH`, on the lane |

- `Pocket` and `Behind` are **exempt** from the 120-tick spacing check.
- If the chosen spot is outside the view margin, the other of the two is used. If both are outside, `Pocket` is used: it is authored on the route and the camera follows the leader.
- `RescueSpot::Tunnel` (room 3's past-stop spot at `stop_x + 48`) is **never** used before `BarrierImpact`: it lies past the closed barricade.

**After `BarrierImpact`:** room 3's order (the lane spot, then the past-stop spot), unchanged.

- **During `Tell` and `Charge`,** a bot that is not in the pocket is rescued into it on the tell's first tick, with the cause `Charge`, a new appended `RescueCause`. This avoids a later near-wheel rescue. The bot may still walk out; then the rules above apply.
- **Append only.** `RescueSpot` and `RescueCause` keep their existing discriminants (`Lane` 0, `Tunnel` 1; `Near`, `BehindAxle`, `OffScreen`).
- **Targets.** The fixtures assert **zero** wheel contacts on a bot in solo Carl-active (ACT1-F10, F15) and Donut-active (ACT1-F10b, F15) runs, and **zero** `AiRescue` for a bot that reached the pocket before `ChargeTell` (ACT1-F10). They report the other rescue counts by spot; there is no CA2-style budget (ACT1-Q5). If zero cannot be reached with these spots, stop and escalate as a CA2-style named append (section 12.4).

### 5.6 Sidecar block and validation

`RawDozer` gains optional fields with `serde(default)`. Room 3's sidecar bytes and hash do not change.

| Field | Type | Default (room 3) | Opening |
| --- | --- | --- | --- |
| `profile` | `"revenge"` or `"opening"` | `revenge` | `opening` |
| `dzq11_trigger_by` | `"lead"` or `"rearmost"` | `lead` | `rearmost` |
| `dzq22_lintel` | rect, **now optional** | required by `revenge` | absent |
| `pockets` | list of `{ id, rect, ledge }`; `ledge` is the id of a `solid` static in the same sidecar | empty | one |
| `timbers` | list of `{ id, x }` | empty | two |
| `wedge` | bool | false | true |

`DozerSite.lintel` becomes `Option<IRect>`. The `revenge` validation errors when it is `None`. `f01_dozer` reads it as before.

Validation branches on the profile. `revenge` keeps rules 1 to 4 exactly. `opening` replaces rules 2 and 4 with O1 to O10 (rule 1, the trigger between the gate and `stop_x`, still applies):

| Rule | Check |
| --- | --- |
| O1 | The opening tuning has a `dzq10_charge` block, and the sidecar has at least one pocket and exactly one `dozer_contact` lock |
| O2 | `emerge_front_x < trigger_x < arm_x < pocket.x`, and every timber x lies in `(emerge_front_x, arm_x)`, strictly increasing, with unique ids |
| O3 | Each pocket: `ledge` names a `solid` static whose top equals the pocket rect's bottom; width ≥ 4 × `HERO_HALF_WIDTH` (two bodies); clear height ≥ 2 × `HERO_HALF_HEIGHT` + 8; the pocket lies right of `arm_x` and left of the barricade |
| O4 | The barricade lock solid lies after the pocket and at least 256 px before `stop_x` (`RUN_OUT_CLEAR_PX`); it reaches the floor and is taller than any hero jump (the gate classes) |
| O5 | `exit.x ≥ stop_x + dzq17_rescue_past_stop_px + dzq17_rescue_exit_margin_px` |
| O6 | No other solid (static, platform, prop or lock) on the lane between the emergence front and `stop_x` is taller than 16 px (`REMNANT_MAX_PX`): the hump and ledge statics pass, the barricade lock is the only exception |
| O7 | No `dzq22_lintel` and no crew (`mount`) entries in an `opening` room |
| O8 | With the barricade closed the exit is unreachable for every class and 2P; open, it is reachable (the existing lock checks) |
| O9 | The commit window is non-empty: `barricade.x − (36 × dzq08_mercy_px_s / 60 + speed_px_s × max_ticks / 60) ≤ pocket.x − min_gap_px` (rule 4 of section 5.3 at the slowest band speed; today 1862 ≤ 1932) |
| O10 | The pocket centre lies left of `barricade.x − HERO_HALF_WIDTH` (a pocket rescue never puts a body in the barricade) |

### 5.7 Events and the state hash

Appended `DozerEvent` variants: codes 13 `ChargeArmed`, 14 `ChargeTell`, 15 `ChargeCommitted`, 16 `ChargeEnded { by_ticks }`, 17 `TimberSmashed { id }`, 18 `PocketEntered { hero }`, 19 `BarrierImpact`, 20 `Wedged`. Each one has a `mix_into`.

`ChargeEnded { by_ticks }` carries the charge's tick count. It fires at `stop_x` after a `BarrierImpact`, or at `max_ticks` on the defensive overshoot path (section 5.2).

The machine mixes the charge sub-state, the used-charge flag, `barrier_hit`, the per-hero pocket flags, the smashed-timber bits and the wedge flag into the hash **only when the profile has a charge block**. Room 3's fixture hashes, DOZR replays and goldens therefore stay byte-identical, and the brief makes that a stop condition. The barricade lock is a pure function of `barrier_hit` (section 5.4), so the env runtime adds no flag and no hash term for it.

## 6. Entry and exit contracts with the run

### 6.1 Scenes and listing

- `RoomScene::{Act1Chase, Act1Shore, Act1Guild}`, with serde names `f01_act1_chase`, `f01_act1_shore` and `f01_act1_guild`.
- K17: all three run under `cf02_rules`. Each Act I scene runs only its own room id, like the corral rule.
- **CORRAL-F becomes ACT1-F.** Every listed Act I room comes before the corral, in the fixed order chase, shore, guild (any listed subset keeps that order). The corral directly follows the last listed Act I room. No Act I room is listed after the corral.
  - When no Act I room is listed, the rule reduces to today's CORRAL-F.
  - The error text names both IDs ("ACT1-F, CORRAL-F").

### 6.2 Carry

- The Act I rooms are story-less like the corral. Each sidecar `story_pin` is a label with no table row, and `launch` asserts `!has_pin(pin)` (the corral's rule).
- `carry_for(Story, scene, combat)` falls back to the empty snapshot for **every story-less scene**: corral, act1_chase, act1_shore and act1_guild (a `RoomScene::story_less()` helper).
- So the chain under Story is: chase → empty → shore → empty → guild → empty → corral → empty → refuge (unchanged). The corral now *receives* an empty carry, which `carry_story` accepts.
- Under `--f01-loadout all`, `carry_for` returns nothing, as today (F01RUN-AD2).

### 6.3 Donut's setup per loadout (ACT1-AD13)

| Launch | chase, shore | guild |
| --- | --- | --- |
| Story loadout, or flagless | `DonutSetup::NONE` (locked: the swat, no missile, her AI never casts) | locked plus `guild_zone (336, 376)` |
| `--f01-loadout all` (or any `--donut-light-feet` / `--donut-puddle`, which imply the biscuit, DP-Q06 / DP-Q10) | `DonutSetup::unlocked(8)` | `DonutSetup::unlocked(8)` (the lock is open at tick 0) |

- Carl holds his base kit under Story: no dynamite, no reinforced fist, no thrown charge (#179: nothing before the Guild). The Act I fixtures author no bomb or charge pickups.
- The corral and every later room keep their own `DonutSetup::unlocked(8)` unchanged. The biscuit is a read-through capability that each room declares, so nothing about it is carried.

### 6.4 The `donut_unlocked` lock

`LockEvent::DonutUnlocked` is appended to the `#[non_exhaustive]` enum, with sidecar `"opens_on": { "event": "donut_unlocked" }` and no target.

- **Open state.** `sync`/`step_env` reads `DonutMana::unlocked()` on every fixed tick, and the lock is open exactly when it is true. It is a pure function of state the hash already holds, so the lock needs no new hashed flag.
- **Tick order (one-tick lag).** The scene step (collision) runs before `step_env`, so the door opens in the tick of `UnlockGranted` and is effective for collision from the **next** tick. No global step is reordered.
- **Retry and restart.** A defeat retry keeps the unlock, so the door stays open. A fixture restart restores the locked start, so the door closes.
- **Validation.** Allowed only in a room whose Donut setup can unlock: the validator accepts it, and the room module asserts that the guild zone is set under the Story setup. O8's reachability checks apply.

### 6.5 Panel, HUD, audience

- **Room-clear panel.** The guild's panel lists `granted here: Donut primary_ranged`. `RoomFacts.grants` gains `(1, "primary_ranged")` when the room's combat log holds a `ManaEvent::UnlockGranted`; the ledger never holds it (ACT1-AD12). The other two Act I panels show "none".
- **Notice.** `LabelId::MagicMissile` → `"MAGIC MISSILE"`, raised by `notice_for_event` on `CombatEvent::Mana(ManaEvent::UnlockGranted)`. It is presentation only. Cut it if it moves a golden (cut order 1).
- **Hint.** `LabelId::GetIntoTheAlcove` → `"GET INTO THE ALCOVE"`, shown for 180 ticks after a respawn at the chase checkpoint, only after at least one death in this attempt (#179 section 4). It is presentation only (cut order 2).
- **Audience.** Off in all three rooms (positional rule, no code). This matches #179's "no viewers or sponsor in the opening". The broadcast line stays on the corral's panel.
- **Location strip.** It shows the manifest titles, as for every room.

## 7. Grants: GUILD-BEAT-01 and KR12

- `story_order`:
  - ACT1-A replaces the point `{ "id": "guild_biscuit", "kind": "point", … }` with `{ "id": "act1_guild", "kind": "room", "pins": [], "grants": [{ "capability": "primary_ranged", "trigger": null }] }` and inserts `{ "id": "act1_shore", "kind": "room", "pins": [], "grants": [] }` before it.
  - ACT1-B inserts `{ "id": "act1_chase", "kind": "room", "pins": [], "grants": [] }` first.
- `check_story_order` already accepts both a read-through grant with no trigger and a room row with empty pins. No pin holds a later grant: `primary_ranged` is still granted before the corral and the Hoarder's `f01_pin_pre_pile_boss`.
- **Unchanged:** `story_unlocks_v0.json` (`TABLE_HASH`), every pin and every trigger. The combat lab's `floor01_after_guild_biscuit_before_juicer` label is a lab profile name and stays.

## 8. Flags and defaults

- No new command-line flag. The Act I rooms are listed, so `--f01-run` starts in them by default. `--f01-room act1_chase|act1_shore|act1_guild` launches one directly, as for any listed room.
- `DEFAULT_ROOM` (`boiler`) is unchanged.
- An optional `--room-pilot` plan per room (the EVID-ROOM pattern) is for evidence captures only.

## 9. Determinism and pins

Fixed tick, no RNG beyond the existing seeded CF02 stream. Presentation (decor, the wedge draw, splinters, notices, hints) never enters the sim or the hash.

| Pin | ACT1-A | ACT1-B | Reason |
| --- | --- | --- | --- |
| `story_unlock::TABLE_HASH` | **unchanged** | **unchanged** | `primary_ranged` is read-through; no table row moves |
| `f01_late::MANIFEST_HASH` | re-pin (from `0xb42f_2f31_5436_e7d2`) | re-pin | rooms rows, story-order rows and a `status` sentence each; the new room `status` strings name no run position (the REST-WIRE PR 2 rule). Every committed or local late-floor PLAYTRACE recording made before a re-pin fails strict verification with a content mismatch (expected; say so in the PR) |
| `dozer::TUNING_HASH` (`enemy_dozer_v0.json`) | unchanged | **unchanged** | the opening values live in a new file |
| `OPENING_TUNING_HASH` (`dozer_opening_tuning`) | | new | new file |
| Act I room and sidecar hashes | new (4: shore and guild) | new (2: chase) | new files |
| Room 3 sidecar and room hashes | unchanged | **unchanged** | the new `dozer` fields default; the file is untouched |
| `SCOPED_PIN_KEYS` | +4 | +3 | today 25 (after REST-WIRE PR 2) → 29 → 32. `pin_applies` maps each new key to its room. `dozer::PIN_KEY` now applies to `dozer` **and** `act1_chase` (the shared machine values); the scope test's owner map takes a list |
| `playtrace::live::data_pins()` | +4 | +3 | the new pins; update the length assertions |
| Goldens, DP fixture hashes, room-3 DOZR fixture hashes, CF02 tuning, gate classes | unchanged | unchanged | asserted; a move is a stop condition |
| Act I fixture final hashes | new | new | new fixtures |

PLAYTRACE gets new descriptor rooms and replay and live dispatch arms (the REST pattern), plus `f01_act1_{chase,shore,guild}_recording_verifies` (strict, 360 and 400 view heights).

## 10. Tests and fixtures by ID

Module tests live in `src/f01_act1.rs`. The fixtures live in `src/act1_fixtures.rs`; each runs twice and asserts an identical final hash. The #179 CHDIR IDs are mapped in the last column.

| ID | Test / fixture | Expected | PR | CHDIR |
| --- | --- | --- | --- | --- |
| ACT1-T01 | `the_act1_rooms_are_listed_first_in_story_order` | listed `[act1_chase?, act1_shore, act1_guild, corral, …]`; `first_room` is the first Act I room; ACT1-F rejects a swap, an Act I room after the corral and a gap before the corral | A, B | |
| ACT1-T02 | `act1_carry_is_empty_and_accepted` | Story: each Act I exit carries the empty snapshot; the corral accepts it; All: none | A | |
| ACT1-T03 | `act1_shore_sidecar_validates`, `act1_guild_sidecar_validates`, `act1_chase_sidecar_validates` | valid; no surface above 42 px except the guild's slab; the story pins are not table rows | A, B | |
| ACT1-T04 | `act1_guild_door_seals_until_the_biscuit` | locked: exit unreachable for every class and 2P; unlocked: reachable | A | |
| ACT1-F01 | `act1_shore_solo_carl` (companion off) | both goblins defeated; exit; no `UnlockGranted`; Donut never casts | A | |
| ACT1-F02 | `act1_shore_block_kill` | three strikes at local 367 while `gob_a` stands in 392..472: a credited defeat by the block | A | |
| ACT1-F03 | `act1_shore_2p` and `act1_shore_solo_companion` (default `cf02_free`) | exit reached; Donut never casts (CF02-Q17) | A | |
| ACT1-F04 | `act1_shore_retry_from_post_chase` | a party defeat respawns at the checkpoint (route 2640); goblins restored; no chase | A | 09 |
| ACT1-F05 | `act1_guild_solo_carl` (companion off, swap to Donut), `act1_guild_solo_donut`, `act1_guild_2p` | exactly one `UnlockGranted { hero: 1 }` when Donut's pivot enters 336..376; the lock reports open in that tick and the door stops colliding from the next tick; exit; panel grants `(1, primary_ranged)`; HUD prompt switches | A | |
| ACT1-F06 | `act1_guild_companion_crossing` (default companion, Carl active) | exactly one grant; exit; Carl is never sealed in | A | |
| ACT1-F07 | `act1_guild_retry_keeps_the_biscuit` | after a defeat retry: unlocked, door open; after a fixture restart: locked, door closed; a second crossing grants once | A | |
| ACT1-F08 | `act1_guild_all_loadout_is_open` | `unlocked(8)` at tick 0, door open, no `UnlockGranted` | A | |
| ACT1-F10 | `act1_chase_solo_first_run`: Carl scripted (run right, hop into the pocket, wait, walk to the exit after `Wedged`); Donut is the follow bot (companion off) | two ordered subsequences: (a) `Rumble`, `Emerged`, `TimberSmashed a`, `TimberSmashed b`; (b) `ChargeArmed`, `ChargeTell`, `ChargeCommitted`, `PocketEntered` ×2 (either may come before `ChargeTell`), `BarrierImpact`, `ChargeEnded`, `BrakeStarted`, `Wedged`. Exactly one `BrakeStarted`. The commit front lies in 1841..1932 (rule 4). Zero `WheelHit`; `AiRescue` = 0 for the bot once it reached the pocket before `ChargeTell`; exit; no hero defeat | B | 01, 03, 04, 08 |
| ACT1-F10b | `act1_chase_solo_first_run_donut`: the Donut-active twin (Donut scripted the same way, Carl the follow bot) | the same subsequences and counts as F10 | B | 01, 06 |
| ACT1-F11 | `act1_chase_hull_cannot_be_hurt` | melee on the hull: `Clang` only, `hp()` constant, no stun, no state change | B | 02 |
| ACT1-F12 | `act1_chase_2p_together` | both in the pocket; zero wheel contacts | B | 05 |
| ACT1-F13 | `act1_chase_2p_lag_{0,96,224,448}` (P2 that far behind the leader when the leader arms; both heroes run and hide in the pocket) | zero wheel contacts; in every case the commit front lies in 1841..1932 (the trigger and the band follow the rearmost hero, so the lag only changes when the leader arrives); `BarrierImpact` within the charge; recorded commit tick, commit front and margin per hero, per case | B | 06 |
| ACT1-F14 | `act1_chase_skip_pocket` (Carl, 185 px/s, waits at the barricade; Donut is on the rule-3 boundary, section 5.3) | no charge (rule 3 fails); a `WheelHit` defeat before the front reaches the barricade (no `BarrierImpact`, no `PinnedHold`); respawn at 1008; the barricade still closed (`barrier_hit` false); the machine `dormant`; the hint shows; no softlock | B | 07, 09 |
| ACT1-F15 | `act1_chase_solo_bot_{carl,donut}_active` (default companion) | zero wheel contacts on the bot; rescue counts reported by spot | B | 06 |
| ACT1-F16 | `act1_chase_no_charge_path` (2P): P1 waits in the pocket; P2 shadows the front (body left edge 100..130 px ahead, walking at the machine's band speed) from the emergence into the pocket, so rule 2 (gap ≥ 140) never holds | no `ChargeTell`; `PocketEntered` ×2; the cruise passes the pocket; `BarrierImpact`, `Wedged`, exit; zero `WheelHit` | B | 07 |
| ACT1-F17 | `act1_chase_presentation_is_read_only` | the same inputs with presentation disabled (no decor, no notices, no splinters) give the same event log and hash | B | 12 |
| ACT1-F18 | `room3_unchanged_by_the_opening` | every DOZR fixture hash and replay equals its committed value | B | |
| ACT1-T10 | `opening_validation_rules` | O1..O10 each rejects a minimal broken sidecar (O3 with a `ledge` naming a missing or non-`solid` static); the revenge rules still reject a lintel-less `revenge` block | B | |
| ACT1-T11 | `the_charge_commit_rule` and `the_charge_runs_through_the_barricade` (domain unit tests) | rule 5.3 table rows above, including rule 4's boundary (front 1855 vs 1856 at 160 px/s), the walk-back and the pocket-behind-the-wheel case; the instant charge speed; the impact on the charge's last tick; a run past `BarrierImpact` to `stop_x`; the defensive overshoot with a synthetic tuning that has no barricade in reach; a run to `BarrierImpact`, then a party respawn (`slot_respawn`) and a fixture restart: `barrier_hit()` false, `dormant`, the `dozer_contact` solid present again; the rescue cause rules of section 5.5 (pocket skip, fully behind the hull) and the spot fallbacks; the pocket test's y conversion | B | 04, 06 |
| ACT1-T12 | `act1_run_walk` | a Story `--f01-run` walk from the first Act I room through the corral: entry lines in order, empty carries, guild grant on the panel | A, B | 11 |

CHDIR-10 (resolutions and scaling) and CHDIR-12's audio half are evidence items (section 11). CHDIR-11 is ACT1-T12 plus the PLAYTRACE verify tests.

## 11. Evidence (`docs/evidence/act1/`, a README with commands and results)

1. Bevy captures at 640×360, 640×400 and 1920×1080:
   - S1 at spawn;
   - the emergence at the S2 window;
   - a timber smash;
   - the lit `alcove` decor as the player approaches it, before the tell (the pocket must read as the way out);
   - the tell;
   - both heroes in the pocket with the wheel passing;
   - the barricade impact;
   - the wedge;
   - the shore strike and the block kill;
   - the guild zone with the notice and the changed prompt.
2. A seeded GIF of the solo chase (ACT1-F10) and one of a 2P lag-448 run (ACT1-F13), at 1x.
3. The ACT1-F13 commit table (commit tick, commit front, margin per hero, per lag) as text, plus the F10/F10b rescue counts by spot.
4. The run walk: each room's entry line from `act1_chase` to `mastery`, in order.
5. The PLAYTRACE verify lines.
6. Frame time at 640×360 for `act1_chase` (`--perf-frames`), against room 3.
7. A real two-pad run, a human first-run time (the #179 25–40 s hypothesis) and human recognition of the pocket: **NOT RUN** unless a human did it.

## 12. Implementation split (two PRs, one Sonnet implementer each)

### 12.1 ACT1-A: run plumbing, the shore and the guild

- **Data:** manifest rows `act1_shore` and `act1_guild` before `corral`; the story-order change; the `status` sentence; the `MANIFEST_HASH` re-pin. The run grows from 12 to 14 rooms.
- **Run rules:** ACT1-F in `validate`; `RoomScene::Act1Shore` and `Act1Guild`; `story_less()`; the `carry_for` generalization.
- **Room module:** `src/f01_act1.rs` with `Act1Room { Chase, Shore, Guild }` (Chase unlisted and unbuilt until ACT1-B); `LockEvent::DonutUnlocked`; the room and sidecar files.
- **Presentation:** the panel grant fact; the `MAGIC MISSILE` notice; the decor draw.
- **Plumbing and checks:** the binary and PLAYTRACE arms; the retargeted run tests (section 12.3); ACT1-F01..F08 and T01..T04, T12; evidence.

### 12.2 ACT1-B: the chase

- **Data:** the opening tuning file; the `RawDozer` fields and the optional lintel; the O1..O10 validation branch.
- **Machine:** the charge sub-machine, commit rule, pocket, `barrier_hit`, wedge, timbers and events in `src/dozer.rs`; the profile choice in `combat_scene/dozer.rs::build`; `LockEvent::DozerContact`; `RescueSpot::{Pocket, Behind}` and `RescueCause::Charge`.
- **Room:** the room and sidecar (lane statics, the alcove decor); the manifest row `act1_chase` first, the story-order row and a `MANIFEST_HASH` re-pin (the run grows from 14 to 15 rooms); `RoomScene::Act1Chase`; the binary and PLAYTRACE arms.
- **Presentation:** the DOZ-01 rig reuse (room 3's draw path); the greybox wedge; splinters; the hint.
- **Checks:** ACT1-F10..F18 and T10, T11; evidence.

**Cut order inside ACT1-B** (cut from the top; never cut the pocket, the lock, the commit rule or the zero-contact fixtures):

1. the hint;
2. the timber splinter presentation (keep the events);
3. the nav-row hole around the pocket (if the rescue alone reaches zero contacts);
4. the greybox wedge draw (a labelled rectangle instead).

Audio is not in scope (ACT1-Q2).

### 12.3 Tests that must be retargeted (ACT1-A)

- In `src/f01_run.rs`:
  - every `first_room(..).id == "corral"`;
  - `the_audience_is_live_only_after_the_corral` (`listed[0]`);
  - the `room_run` test dispatch: the Corral arm's "first room, no carry" assertion becomes "an empty carry";
  - the dynamite test's exception list (add the Act I rooms);
  - the panel counters ("ROOM n/N", "n of N listed rooms", slices `rooms[3..]`);
  - `the_corral_exit_relaunches_into_the_refuge`, which must still pass with the corral no longer first;
  - the run-walk grant table (add `act1_guild`).
- In `src/f01_late.rs`: `the_corral_is_listed_first_…` becomes ACT1-F (CORRAL-F) cases, and the listed-rooms test changes with it.
- Docs that list the run order: `docs/03-tech/floor01-scene.md` (the `--f01-room` ids and the manifest re-pin history) and `docs/01-design/audience-run-design.md` (the "Tutorial Guild point `guild_biscuit`" sentence becomes the `act1_guild` room).

### 12.4 Sequencing and stop conditions

- **Order.** ACT1-A, then ACT1-B. Each re-pins `MANIFEST_HASH` once, starting from main's value after REST-WIRE PR 2 (`0xb42f_2f31_5436_e7d2`). If another manifest PR lands in between, rebase and re-count (section 9).
- **Hot files.** `src/f01_late.rs`, `src/f01_run.rs`, the manifest, `src/encounter.rs`, `src/combat_scene/env.rs`, `src/bin/floor01_scene.rs` and the PLAYTRACE dispatch. ACT1-B also touches `src/dozer.rs`, `src/combat_scene/dozer.rs` and `src/encounter/haz.rs`.
- **Stop and report** (do not improvise) if:
  - a room-3 DOZR hash, a golden or a DP hash moves;
  - zero bot wheel contacts cannot be reached with the spots of section 5.5;
  - the lock validator cannot express `dozer_contact` or `donut_unlocked` without a design change;
  - `carry_story` rejects the corral's empty carry;
  - the Act I rooms cannot be reached first through `--f01-run`.

## 13. Adopted defaults (ACT1-AD)

All are adopted under the owner decision of 2026-11-25. The owner may change any of them after playing.

| ID | Default | Why |
| --- | --- | --- |
| ACT1-AD1 | Three rooms `act1_chase`, `act1_shore`, `act1_guild` before the corral, milestone `greybox`, `cf02_rules`, story-less | One room per launch is the run's model. The guild interior is "separate" in the Act I beats doc |
| ACT1-AD2 | The chase/shore boundary is ENV-02's post-chase checkpoint (route 2640); the standard room-clear panel shows between them | DOZER-CORE resets the machine on every respawn; a split keeps CHDIR-09 without new respawn rules |
| ACT1-AD3 | Titles "The First Chase", "The Rotten Shore", "The Guild Door" | Plain and literal, no franchise quote (antihero tone) |
| ACT1-AD4 | Spawns Carl 72 / Donut 48 in the chase (ENV-02 spawn), Carl 112 / Donut 80 elsewhere | ENV-02 marker; REST pattern |
| ACT1-AD5 | Commit rule of section 5.3 (all heroes past `arm_x`, gap ≥ 140 from the rearmost body, reach margin 20 ticks, the charge reaches the barricade), one charge per attempt, no fail-open timer | #179 policy 1; DOZ-01 gate; no forced lethal charge; rule 4 keeps #179's standout beat (the wheel passes the alcove, then the barricade breaks, C5/C6) |
| ACT1-AD6 | The chase trigger is crossed by the **rearmost** living hero | A leader alone never wakes the machine with P2 behind its emergence point |
| ACT1-AD7 | No idle wake in the opening (216000 ticks) | #179 C0 "false calm, no forced tutorial"; no softlock is possible ahead of the trigger. In 2P a human P2 who stays behind 1072 forever means no chase and a closed barricade: a stall by choice, not a softlock (QA should not file it) |
| ACT1-AD8 | The S3 soffit is decor, not a solid | 212 px clear is above every jump; the machine ignores solids |
| ACT1-AD9 | No riders on the opening machine in the greybox; the shore goblins stand in for the thrown riders | DR-C's crew is room 3's system; the riders are ACT1-Q6 |
| ACT1-AD10 | The wedged machine in the shore room is decor | The machine is resolved in the chase room |
| ACT1-AD11 | The guide is decor at the `refuge_guild` mark; the `mordecai_h44` sprite if drawable, else a labelled rectangle; no text | No invented dialogue; the beat stays brief (I-04) |
| ACT1-AD12 | The guild panel lists the read-through grant; a `MAGIC MISSILE` notice on `UnlockGranted` | Every other grant has a notice; the ledger never holds `primary_ranged` |
| ACT1-AD13 | Donut locked in the Act I rooms under Story and flagless; `unlocked(8)` under All or the light-feet / puddle flags | The flags already imply the biscuit (DP-Q06, DP-Q10) |
| ACT1-AD14 | `RescueSpot::Pocket`, `RescueSpot::Behind` and `RescueCause::Charge` appended; the solo bot is put in the pocket at the tell; no rescue cause in the pocket or fully behind the hull; the two new spots skip the spacing check and fall back to each other, never to `Tunnel` before `BarrierImpact` | K5 without Donut invincibility (Dozer note section 7); DOZ-01 "the player must never fail because of the AI partner" |
| ACT1-AD15 | The opening values live in a new tuning file with its own pin | Room 3's `TUNING_HASH` and data stay untouched (Dozer note section 4.4) |
| ACT1-AD16 | `stop_x` 2856 and the exit at 2928; the machine wedges at the S5 arch's right edge | ENV-02 wedge marker; the past-stop rescue spot fits before the exit |
| ACT1-AD17 | The humps and the alcove ledge are 16 px sidecar statics of kind `solid`, not room platforms; the pocket's `ledge` names the static | A room platform needs 52 px clear under its slab (`PlatformBlocksLane`, REST-IMP-8); a lane static up to 48 px is the validator's low-step form (Hoarder `bank_a`) |
| ACT1-AD18 | After `BarrierImpact` the charge ignores `max_ticks` and runs to `stop_x`, then brake, strain, `Idle` and `Wedged`; `ChargeEnded` at `stop_x` | One brake in a normal run; the wedge keeps the charge's momentum (#179 C7) |
| ACT1-AD19 | The barricade is open iff `Dozer::barrier_hit()`; no env flag | Every respawn and restart path closes it by construction; REST's surface locks keep their flags across a respawn |

## 14. Open questions (ACT1-Q, non-blocking)

| ID | Question | Default until answered |
| --- | --- | --- |
| ACT1-Q1 | Should the chase/shore and shore/guild boundaries skip the room-clear panel (a seamless relaunch)? | The standard panel |
| ACT1-Q2 | Runtime audio for the chase (music on the trigger, engine, whistle, impact, sudden quiet) once the game has an audio path | No audio; the provisional files stay unused |
| ACT1-Q3 | End-on wedge art (≤ 192 px wide, DOZ-02 or ENV-02's wall-breach contingency) | The greybox wedge draw |
| ACT1-Q4 | The optional third goblin at spawn C | Two goblins |
| ACT1-Q5 | A rescue budget for the opening (CA2-style), after a human session | Zero wheel contacts asserted; counts reported |
| ACT1-Q6 | Riders on the opening machine, thrown at the barricade impact into the shore room | None in the greybox |
| ACT1-Q7 | ENV-02 dressing of the three rooms (the art kit on the greybox) | Greybox kit, decor rectangles |
| ACT1-Q8 | Guild dialogue, the skip / fast advance for repeat play (I-04) and the book chronology | No text, nothing to skip |
| ACT1-Q9 | The #179 first-run duration hypothesis (25–40 s) and the reach margin, after a human session | Values above |
| ACT1-Q10 | After a hero's defeat in the chase the machine keeps running through the defeat delay and visibly breaks the barricade before the respawn closes it again (ACT1-IMP-11). Should the defeat presentation show the break, or should the camera hold on the downed hero? | The break is shown; nothing in the sim changes |

## 15. Deviations (flagged)

- **From #179.**
  - #179 kept ENV-02's 8 s scripted-charge fail-open as a last resort. This plan has no fail-open: with no charge, the cruise still opens the barricade (ACT1-AD5).
  - #179 kept chase and fight "in the same scene". This plan splits them into two run rooms at the post-chase checkpoint (ACT1-AD2).
  - Both changes keep #179's stated intent: no softlock, no forced lethal charge, fast retry without the intro.
- **From ENV-02.** "When the leading player crosses x 1888 the pursuer charges" is now arm-on-leader, commit-on-all (#179's own fix). The trigger is crossed by the rearmost hero.
- **From DOZ-01.**
  - DOZ-01's state diagram allows repeated charges and a "stopped" state when Carl is pinned. The opening allows one charge, and the room-3 pinned rule stays as DOZER-CORE built it.
  - DOZ-01 suggests "Donut exempt or guaranteed to keep up". The opening uses the rescue spots instead (no invincibility, Dozer note section 7).
  - DOZ-01's charge lasts 90 ticks, then overshoots. In the opening a charge that has broken the barricade runs on to `stop_x` and wedges (ACT1-AD18), and rule 4 only commits a charge that reaches the barricade, so the overshoot is a defensive path only.
- **From the Act I beats doc.** Its line "keep chase, CP-01, CP-02 and CP-03 in the same scene" predates the run's one-room-per-launch model. The three rooms keep ENV-02's geometry and order.

## 16. Horizon, tone and rights

- **Horizon.** Consulted [the progression horizon](character-progression-horizon.md). The plan adds no hero verb, button or ability.
  - The biscuit is the approved first Donut unlock (I-04, DP-Q02's chain), now placed in a room instead of a point.
  - The chase teaches the horizon's "a threat can be too strong to kill *yet*" with terrain as the answer.
  - The direction is unchanged, and no horizon edit is needed.
- **Tone.** The opening is panic and survival, not heroics. Carl cannot hurt the machine; the pocket is a cramped hole, not a power move. The first kill is a plain punch and a rotten beam. The guild is a quiet, modest room with no fanfare beyond a two-word notice. No slow motion, no audience, no sponsor.
- **Rights.**
  - No new art, text, audio or third-party material.
  - Every drawn piece is an existing committed asset (the DOZ-01 rig, the ENV-02 splinter burst, `mordecai_h44`) or a greybox rectangle.
  - Character names remain the replaceable fan-concept layer: room ids, events and data use neutral names (`guide`, `pocket`, `opening`).

## 17. ACT1 status and implementation notes (ACT1-A, ACT1-B)

**What landed (ACT1-A).** The run is fourteen rooms: `act1_shore`, `act1_guild`, `corral`, and the rest as before.

- **Run plumbing.**
  - `Manifest::validate` enforces ACT1-F in place of CORRAL-F.
  - `RoomScene::{Act1Shore, Act1Guild}` and `RoomScene::story_less()`.
  - `carry_for` returns the empty ledger for every story-less scene.
  - The run panel reports the biscuit as the grant `(Donut, primary_ranged)`.
  - The `MANIFEST_HASH` re-pin.
- **Rooms.**
  - `src/f01_act1.rs` with `Act1Room::{Shore, Guild}`, the room and sidecar files, and the decor tables.
  - `LockEvent::DonutUnlocked` and `EnvSolidKind::ManaLock`, with no stored flag and no hash term.
  - The appended combat event `ManaLockOpened`.
  - `LabelId::MagicMissile`, the `MAGIC MISSILE` notice (cut order 1 not needed: no golden moved).
  - The binary, PLAYTRACE and CI arms.
  - The fixtures ACT1-F01 to F08 (`src/act1_fixtures.rs`) and the tests T01 to T04 and T12.
- **Greybox.** Both rooms are flat rectangles (the wedged machine is labelled decor; the guide is the existing `mordecai_h44` sprite, ACT1-AD11), no audio, no dressing.
- **Not in ACT1-A.** The chase room, the opening profile, `LockEvent::DozerContact`, ACT1-F10 to F18 and T10, T11, `LabelId::GetIntoTheAlcove` (all ACT1-B). `Act1Room` has no `Chase` variant yet.
- **Pins.** `MANIFEST_HASH` re-pinned (`0xb42f_2f31_5436_e7d2` to `0x9f2e_dc09_b49e_0f08`); four new room and sidecar pins; `SCOPED_PIN_KEYS` 25 to 29. `TABLE_HASH`, `dozer::TUNING_HASH`, every DP, room 3 and golden hash are unchanged.

**Implementation defaults (ACT1-IMP).** Each fills a gap the design left open, with the smallest option consistent with it. The owner may change any of them.

| ID | Gap | Adopted default |
| --- | --- | --- |
| ACT1-IMP-1 | Section 4.2 names `temperament` on the shore's roster rows, but the sidecar format carries a temperament on a mounted entry only (`deny_unknown_fields`; the validator refuses it elsewhere). | The sidecar rows carry `id`, `kind`, `x`, `beat` and `delay_ticks`. The module constant `f01_act1::SHORE_TEMPERAMENTS` (brawler, coward) is applied through the new `CombatState::with_goblin_temperaments`, which overrides the seeded draw's temperament only (empty everywhere else, so no other room moves). |
| ACT1-IMP-2 | The post-chase checkpoint (centre 56) lies behind Carl's start (112), and the respawn columns must ascend. | A checkpoint behind the start is the only respawn column of the room (a retry puts the party at 56 in the shore and at 88 in the guild). A checkpoint ahead keeps the usual start then checkpoint pair. |
| ACT1-IMP-3 | The design names `goal_x` as the exit centre. | The room parser derives the exit as `goal_x - 24`, 64 px wide, so `goal_x` is 752 (shore) and 584 (guild): the exit rectangles are the design's `{728, 484, 64, 96}` and `{560, 484, 64, 96}`. |
| ACT1-IMP-4 | The design lists no sidecar form for an event with no target. | `RawOpensOn.target` defaults to the empty string. A `donut_unlocked` lock must leave it and `surface` empty; any other event with no target now fails as an unknown target instead of a missing field (still refused, a different error class). |
| ACT1-IMP-5 | The `guild_door` lock needed a log fact for "the door's open event". | A `CombatEvent::ManaLockOpened { tick, lock }` is appended and logged once per opening (first step records the state, so a room that starts unlocked logs none). It is hashed like every event and never mixed into the room state. |
| ACT1-IMP-6 | Story setup of a story-less Act I room. | `SceneKind::Corral`'s setup (no pin row, no trigger), so `story_unlock.rs` and its table are untouched. |
| ACT1-IMP-7 | The guild guide's draw. | The existing `mordecai_h44` sprite, drawn exactly as the refuge draws him (the refuge's Mordecai mark, standing at the guide decor's column x 380, facing left, with his contact shadow); presentation only. The labelled rectangle is the fallback when the atlas is not loaded. No new art. |
| ACT1-IMP-8 | ACT1-F04 "a party defeat". | The fixture calls `CombatState::respawn` (the call a party defeat makes), the REST retry fixtures' precedent, and asserts the respawn column and the restored goblins. |
| ACT1-IMP-9 | A scripted capture of the Act I rooms. | `--act1-pilot walk|donut|strike` (the `--rest-pilot` pattern); never with `--f01-run`. |

**Horizon-fit.** Unchanged from section 16: no new button, verb or ability. The biscuit is the approved first Donut unlock, now in a room.

### 17.1 ACT1-B: the chase

**What landed (ACT1-B).** The run is fifteen rooms: `act1_chase` first, then `act1_shore`, `act1_guild`, `corral` and the rest as before.

- **The opening profile on DOZER-CORE** (`src/dozer.rs`, `src/combat_scene/dozer.rs`; the machine is not forked).
  - `assets/data/gameplay/enemy_dozer_opening_v0.json` (`OPENING_TUNING_HASH`, pin key `dozer_opening_tuning`) is merged over the shared `enemy_dozer_v0.json` by `DozerTuning::opening()`; `build` picks the tuning from the site's `profile` (`revenge` passes `DozerTuning::active()` exactly as `Dozer::new` does).
  - `DozerSite` gains the profile, the rearmost trigger, `pockets`, `timbers`, `wedge`, the walking line and the barricade x, and `lintel` becomes optional. It is `Clone` and no longer `Copy`.
  - The charge sub-machine (`ChargeStage`), the pure commit rule (`commit_rule_holds`, `wheel_ticks`, `walk_ticks`), the pocket test (`Dozer::pocket_of`), `barrier_hit`, the wedge and the timbers are in `Dozer`. Everything the opening adds is mixed into the hash only when the tuning has a charge block.
  - Appended `DozerEvent` codes 13 to 20, `RescueSpot::{Pocket, Behind}` and `RescueCause::Charge`.
- **The lock.** `LockEvent::DozerContact` and `EnvSolidKind::ContactLock`: the barricade solid stands exactly while the room's dozer reports `!barrier_hit()`; no flag, no hash term.
- **Validation.** The opening branch O1 to O10 (reported as DOZER rules 11 to 20, ACT1-IMP-14).
- **The room.** `src/f01_act1.rs` `Act1Room::Chase`, the room and sidecar files, the three lane statics pushed into the kit as physical steps, the decor table, the alcove hint (`LabelId::GetIntoTheAlcove`, shown while `f01_act1::alcove_hint_live`), the binary, PLAYTRACE and CI arms, the `--act1-pilot hide|skip` evidence pilots.
- **Fixtures** (`src/act1_chase_fixtures.rs`): ACT1-F10 to F18, a 96-run robustness sweep, domain tests T10 and T11.
- **Greybox.** The machine is room 3's approved DOZ-01 rig. The window, the soffit, the timbers, the barricade and the arch are flat rectangles; the lit alcove is the brightest shape. The wedge is the rig clipped to the arch and darkened, with the console label `GREYBOX WEDGE` (ACT1-Q3). The splinter bursts of DR-D play on each timber and on the barricade.
- **Cut order.** Nothing was cut: the hint, the splinters, the nav-row hole around the pocket and the wedge draw all landed. The zero-contact fixtures hold with the nav-row hole in place; it was not tested without it.
- **Pins.** `MANIFEST_HASH` re-pinned (`0x9f2e_dc09_b49e_0f08` to `0xd213_db6d_b329_d342`); two new room pins, the opening tuning pin, `SCOPED_PIN_KEYS` 29 to 32 (`dozer::PIN_KEY` now applies to room 3 and the chase). `TABLE_HASH`, `dozer::TUNING_HASH`, room 3's room and sidecar hashes, every DP and DOZR hash and every golden are unchanged.

| ID | Gap | Adopted default |
| --- | --- | --- |
| ACT1-IMP-10 | The barricade, a timber or a charge's last tick is a float comparison of the wheel front (accumulated in f32), so a charge committed at the rule-4 boundary could miss the barricade by a rounding error. | Every "front reaches x" test and rule 4 accept a front within 0.01 px short of x (`REACH_EPS_PX`). The boundary test (front 1855 against 1856 at 160 px/s) is unchanged. |
| ACT1-IMP-11 | ACT1-F14 asserts "no `BarrierImpact`" after the wheel defeat, but a defeat respawns only after `RESTART_DELAY_TICKS`, and the machine keeps running through that delay, so it breaks the barricade about 10 ticks after the hit. | F14 asserts that the `WheelHit` precedes any `BarrierImpact`, no `PinnedHold`, no tell, and that the respawn rebuilds the machine (dormant, `barrier_hit()` false, the barricade solid standing again, the hint live). The impact before the respawn is the open-then-respawn regression ACT1-AD19 wants covered. |
| ACT1-IMP-12 | The pocket test converts layout y to world y with `layout_h - y`, but the machine step has no `layout_h`. | The site carries the room's walking line in layout px (`floor_px`, set by the loader); the ledge top is `floor_px - rect.bottom()` over the world walking line. The same conversion, taken from the line. |
| ACT1-IMP-13 | ACT1-F13 lag 448: the 2P camera leash stops the leader 447 px ahead of P2, so 448 cannot be reached. | The script holds P2 at `min(lag, 440)`; the separation at the leader's arm is recorded in the evidence table (6, 99, 230 and 444 px for the four cases). |
| ACT1-IMP-14 | The error type numbers the rules `1..=7`. | The opening's O1 to O10 are reported as DOZER rules 11 to 20 with an `O<n>:` prefix in the reason. O8's closed half (the exit is unreachable with the barricade closed) is the test `the_closed_barricade_seals_the_exit_for_every_class` on the route graphs. |
| ACT1-IMP-15 | `wheel_ticks` is "about 66" in the plan; integrated tick by tick in f32, exactly as the machine moves, it is 66 or 67. Donut waiting at the barricade (47 walk ticks) commits at 67. | The rule keeps the integrated count. The fixtures put Carl at the barricade (F14), as the plan says; Donut there is the rule-3 boundary and is not asserted. |
| ACT1-IMP-16 | The scripted hero in F10 stood at the pocket centre; the follow bot trails 48 px behind and never entered the pocket, so the tell rescued it (cause `Charge`). | The scripted hero waits at the pocket's right end (x 2136), so the trailing bot stands inside the pocket before the tell and `AiRescue` is 0. |
| ACT1-IMP-17 | The greybox wedge label has no text path (the scene draws no text for greybox decor). | The console line `F01_ACT1_WEDGE label="GREYBOX WEDGE"` and the darkened, arch-clipped rig; no on-screen text. |
| ACT1-IMP-18 | The opening's `draw` would put room 3's rear gate (x 0) and buffer stop (x 1008) into the chase. | `dozer_art::draw` skips the room pieces and the stop frame for a profile with a charge block; the emergence, timber and barricade bursts use ENV-02's splinter art at the window, the timber sets and the barricade. |
| ACT1-IMP-19 | The alcove notice lasts `NOTICE_TOTAL_TICKS` (136), shorter than the 180-tick window. | The scene raises the hint while the window is live and the notice runs its own lifetime; presentation only. |

**Findings of the fixtures.**

- The commit front is 1856 to 1859 in every run (rule 4 is the last to hold at the cruise): the "first run" row of section 5.3 holds. With the default companion as the bot the commit margin of Donut is 49 ticks (she trails), still above the 20-tick reserve.
- 96 solo runs (either hero active, the follow bot or the default companion, four start delays, six stalls) have zero wheel contacts and no respawn.
- With Carl active and the default companion, Donut was rescued into the pocket twice (cause `off_screen` before the tell, cause `charge` at the tell); every other solo run has zero rescues.
- While a defeated hero overlaps the wheel the machine's `WheelHit` repeats every tick (room 3's behaviour): ACT1-F14 logs 21 events, which are two episodes (the bot, then Carl). The evidence reports both counts.
