# ENEMY-GAMEPLAY-LLAMA-RUSH: the llama rush and the Infernal Corral (CORRAL-0 design sheet)

> **Status: design adopted under the owner's design-defaults directive (AGENTS.md, 2026-11-19); not implemented, not game-feel accepted.**
> **Revision 2** applies the independent review of #195. It fixes B1: a floor Donut who jumps and casts in place could hit the dock llama. A bolt-blocking bumper on the dock edge now closes that, and the gate counts jump-in-place casting as stationary (section 10). It also applies fix-now items F1 to F17 and notes N1 to N6. **Revision 3** applies the narrow re-review. The bumper is raised to **80 px**, which is what the room's 16 px collision tile layer can build; the proof was re-run at 80. Donut's values are now main's, after #186 and #197. B's leash ends at 1830. Notes NB1 and NB2 are applied.
> Every choice below has one adopted value with a `CRL-Q##` ID ([section 13](#13-adopted-defaults-crl-q01crl-q43)). The integration plan calls these rows `CORRAL-Q##`; they are the same IDs. Each value is a tuning hypothesis stored as data. The owner reviews game feel after the implementation merges and may change any value in a follow-up PR. Only [section 14](#14-genuinely-open-questions) waits for the owner.
> Design: Claude Opus (top tier). Execution: medium tier, steps CORRAL-B core, CORRAL-B wire, CORRAL-C, CORRAL-D, CORRAL-E, CORRAL-WIRE and CORRAL-F ([section 15](#15-implementation-brief-and-acceptance-fixtures)). Review: a separate top-tier agent.

**Why:** step CORRAL-0 of the Floor 01 playtest integration plan, Revision 5 ([merged in PR #190](https://github.com/pando85/dungeon-floorbreak/blob/main/docs/07-agents/f01-playtest-integration-plan.md), section 14, conflicts KC1 to KC19, items R1 and R2, plus the #190 review items D1, D2, L1, L2, F3 to F8 and F13). It turns the owner's [Infernal Corral v0.2](floor-01-llama-magic-baptism.md) (PR #181), its [work order](../07-agents/floor-01-llama-magic-workorder.md) and the [CORRAL-AUTO-01 evaluation protocol](../07-agents/floor-01-corral-autonomous-evaluation.md) into exact rules a medium-tier agent can build. Format precedents: the [Engineer sheet](enemy-gameplay-engineer.md) and the Hoarder sheet ([PR #184](https://github.com/pando85/dungeon-floorbreak/pull/184)). Code precedent for a committed floor charge: `src/engineer.rs`.

**The one-paragraph answer.** A missile stagger outlasts Donut's refire, so a ground llama in a ground Donut's line can never walk, or even rush, into reach ([section 3](#3-the-approach-under-fire-budget-d2)). No rush number fixes that. The corral therefore closes the stationary exploit with **height plus one wall**. The pen floor holds one rush llama. A raised exit dock 32 px up holds the second, behind an 80 px dock bumper. Her bolt flies level at the height she releases it. From the dock she can hit only the dock llama, since every bolt passes over the floor llama. From the floor, the bumper stops every bolt she can release, standing or **jumping**, that is low enough to touch the dock llama. So **no spot where she stays, whether she jumps in place or not, can hit both llamas**. This guarantee is geometric, not a tuning hope. Section 10 proves it in f32 against main's physics. The rush then gives each level a readable, fair close-range threat that she must interrupt, jump or bait into the timber partition. The first llama (M1) stays the legacy spitter and is meant to be an easy three-bolt win.

## 1. Authority and locks

- **Owner intent (#181), kept:** an easy first magic win at M1; a two-llama M2 that a planted, spamming Donut cannot routinely clear; one new enemy action, a short committed telegraphed rush; no HP padding, magic resistance, invulnerable phases, homing globs, extra spells, new inputs, forced co-op or offscreen threats; three winning styles (mobile Donut, aggressive Carl, environment and co-op); one optional rush-breakable partition; deterministic 60 Hz with one hit token per attack instance; existing llama fixtures never gain a rush. Deviations are in [section 2](#2-deviations-from-181-and-from-the-plan-flagged).
- **Integration plan #190 section 14:** KC1 (party targeting, opt-in), KC2 (`cf02_rules`), KC5 (`AttackKind::Rush` inside `LlamaMode::Attack`, owned by CORRAL-B), KC6 (`hurt_hero`, one token, a new source id), KC7 (no corral code in companion files), KC10 (height classes, ceiling), KC11 and KC12 (CP rows), KC13 (golden discipline), KC14 to KC16 (evaluator, starts, one geometry file), KC17 (`env_prop.rs` partition), KC18 (speed at most 240 px/s with a fairness proof), KC19 (gate threshold). The review of #190 adds D1 (room 0 grants nothing), D2 (the approach budget), L1 (fixture construction path), L2 (per-llama rush opt-in) and F3 to F8, F13.
- **[DONUT-MOBILITY-01](../03-tech/donut-mobility-01.md):** Donut runs 207 px/s (DM-Q01) and jumps at 500 px/s (DM-Q02, apex 114.92 px on tick 28, 57 air ticks); Carl runs 185 px/s and jumps at 440 px/s (apex 88.54 px, 50 air ticks); gravity 1050. Donut's hurtbox stays 20 x 24 (DM-Q25); Carl's is 24 x 52. The 12 s reflex (DM-Q12, 720 ticks) evades the DM-Q17 list; DM-Q18 makes body contacts and any future source non-evadable until a sheet opts in. DM-Q27 is the collision-only lane ceiling.
- **[DONUT-PROG-01](https://github.com/pando85/dungeon-floorbreak/blob/main/docs/03-tech/donut-prog-01-floor01-ability-progression.md) (main):** base Donut height classes 96 and 104 px (112 is a human-only route); Light on Your Feet (launch 547, apex 137.95) is granted at the refuge after the corral, never in it; DP-Q45 ceiling rule.
- **[CARL-PROG-01](https://github.com/pando85/dungeon-floorbreak/blob/main/docs/03-tech/carl-prog-01-floor01-combat-progression.md) (main) and K1:** no dynamite before the Hoarder's defeat, so none in the corral.
- **Owner draft [#187](https://github.com/pando85/dungeon-floorbreak/pull/187) (quiet setpieces):** REST-A, the fast-food refuge, begins **after** the corral's M3 threshold and carries the Light on Your Feet pickup. It says "do not grant the boost at the corral exit".
- **Code on main** (`c0e69acb`, with #186 DM-A and #197 DP-A merged): `src/llama.rs`, `src/feel.rs` (llama constants), `src/combat_scene.rs` (`step_llamas`, `step_globs`, `update_targets`, `hurt_hero`), `src/combat_scene/donut.rs` (bolt geometry, `missile_hits_llama`), `src/donut_mana.rs`, `src/engineer.rs`, `src/prototype.rs` (hero physics), `assets/data/gameplay/companion_magic_v0.json`.
- **[Antihero tone](antihero-tone.md)** and **[fun over canon](vision.md#fun-over-canon-owner-decision-2026-11-21)**. **IP-neutral engine names:** `llama_rush`, `RushProfile`, `LLAMA_RUSH_SOURCE_ID`. "Bad Llama" stays in art and display data.

## 2. Deviations from #181 and from the plan (flagged)

| ID | Source text | This sheet | Why |
| --- | --- | --- | --- |
| DV-1 | #181: rush decision envelope "~80–180 px" | Trigger **0..128 px** pivot to pivot (CRL-Q05); the llama first walks to **112 px** (CRL-Q04) | With 24 active ticks at 4 px/tick the box reaches a standing hero only out to 134 px (Donut) or 136 px (Carl) ([section 6](#6-reach-and-fairness-proof-kc18)). A rush started at 180 px ends 46 px short every time: a whiff that teaches nothing. No minimum distance: point-blank, the 32-tick tell still gives every escape ([section 6](#6-reach-and-fairness-proof-kc18)) |
| DV-2 | #181: "two optional 32 px rise shelves", the second llama at about x 1824 on the pen floor, M3 on the floor line | One shelf stays optional (the entry shelf). The other becomes the **exit dock**, which is on the forward route. The dock carries the second llama at x 1968. Its edge has a 32 px wide **dock bumper** whose top is 80 px above the floor. M3 continues at the dock level (+32) | The D2 budget shows that a planted Donut on a flat lane can only be stopped by height or facing. A level bolt cannot hit down, so on the dock she can never reach the floor llama. A floor Donut can still hit upward by jumping and casting (review B1), so the bumper blocks every bolt released from the floor that is low enough to touch the dock llama ([section 10](#10-the-corral-geometry-crl-q25-to-crl-q33-crl-q41)). Both heroes climb the 80 px bumper (exactly Carl's 80 px DP-Q18 class, the CF02 perch height), or jump past it onto the dock. Its top is a safe stall spot that threatens nothing (section 14, note 5). The dock is not a sniper point: the dock llama rushes anyone standing on it. This is a real change to the owner's layout; the owner sees it at the game-feel review |
| DV-3 | #181 Beat 6: "remaining enemies die or the encounter's stage progression policy permits forward continuation" | The M3 threshold gate opens when **both** M2 llamas are defeated (CRL-Q26) | Without it, a running hero can jump the dock llama and leave, and M2 shrinks to one llama. It is the Hoarder's door rule (#184 DV-2), with an owner kill switch |
| DV-4 | #181: spit baseline "retain" | A **corral-profile** llama spits only when the glob can hit its target at the target's current height and the glob lane is clear (CRL-Q11). Legacy and M1 llamas are unchanged | A spit that sails over a grounded Donut is a free turn for her. With the rule, the floor llama answers a grounded Donut with the rush, which is #181's point ("the short rush must threaten that same stationary ground position") |
| DV-5 | #181 asks for a review of reflex evasion | **Not evadable**, knob `crlq18_rush_evadable` (default `false`) | Review in [section 8](#8-evasion-c4-and-the-reflex). Game-feel note for the owner |
| DV-6 | Plan KC3 lever 4 (light guard on missile interrupts), owner question 3 of #190 section 13 | **Dropped.** No owner question | The missile path never reads the light guard, the lock is a walking stagger rather than a tell interrupt, and three bolts still kill ([section 3](#3-the-approach-under-fire-budget-d2)). The gate is closed by geometry instead |
| DV-7 | Plan KC9 (room 0's M3 is I-08 and grants Light on Your Feet) | **Room 0 grants nothing** (review D1). M3 is the threshold **into** the refuge; REST-A (#187) owns the refuge and the `f01_safe_room_reward_found` swap | #181 places the corral before I-08; #187 forbids the grant at the corral exit |
| DV-8 | Plan KC19; CORRAL-AUTO's `stationary_*` policies "never jump" | Refined: three more gated stationary policies (`stationary_cast_aimed`, `stationary_turret`, `stationary_jump_cast`) and a geometric invariant (no static run defeats both llamas). **Jumping in place counts as stationary** (CRL-Q42) | The strongest legal stationary schedule fires only when a target is in line, turns in place and hops to change the bolt's height. "Press Jump and Primary together" is the easiest input to find, so the gate must cover it ([section 11](#11-stationary-donut-gate-c19-and-the-corral-eval-scenarios)). This is broader than CORRAL-AUTO's literal policy, and the owner may narrow it |
| DV-9 | #181 Beat 4: "manually break the partition through existing physical attacks" | Physical hero hits break it (3 distinct melee hits); **Magic Missile does not** (bolts stop on it) | "Physical" per #181. A missile-breakable screen would make the partition a mana sink, not the rush's payoff |
| DV-10 | #181 l.65: the partition sits "**outside** the only progression path", with "one ground-level clear lane" | The partition (1392..1408, 48 px) stands **across** the pen's only floor lane. While it stands, both heroes jump it or stand on it | Room 0 is a 2D single lane, so there is no "beside" in which to place it. A 48 px screen is jumpable by both heroes (Carl's apex is 88.54) and standable, so it never blocks progress. It has to stand in the lane between B and the heroes, or the rush could never be baited into it. If CORRAL-D is cut, the partition is absent and the lane is open |

No other #181 rule is changed.

## 3. The approach-under-fire budget (D2)

All numbers are main's, at 60 Hz. Main steps Donut with the DM-A agile motion (#186, merged): run 207 (DM-Q01) and launch 500 (DM-Q02). Light on Your Feet's launch of 547 is the DP-A row `dpq14_boost_launch_px_s` (#197, merged). The section 10 proof covers launches 440 (Carl), 500 and 547.

| Quantity | Value | Source |
| --- | --- | --- |
| Missile reach, Donut pivot to llama pivot | Last tested bolt front at **+269 px** (27 spawn + 4 box ahead + 4 half box + 234 travel; the 240 px `projectile_range_px` removes the bolt before its 40th step is tested). The llama's near half-width is **20** when it faces her and **26** when it faces away (`llama_rect`, back shift 3). Reach is therefore below **289 to 295 px** pivot to pivot | `donut.rs` (`step_bolts`, `Bolt::rect`), `companion_magic_v0.json` |
| Mana | 8 charges, 1 per bolt; regen starts 30 ticks after the last release, then 1 charge per 40 ticks, and every release restarts the wait. Over 1,800 ticks that gives about 44 more bolts, so sustained fire is about 1 bolt per 70 ticks. Two M2 llamas need **6** hits. Bolts fired at nothing in reach are wasted | `donut_mana.rs`, `companion_magic_v0.json` |
| Missile time to hit at gap `g` | 4 (release) + about `(g − 55) / 6` ticks; 11 at 96 px, 17 at 128 px, 25 at 180 px, about 40 at 290 px | bolt 6 px/tick |
| Refire | 16 ticks | `CAST_TOTAL_TICKS` |
| Missile stagger | `Hurt` **18** ticks from **any** mode, plus hit-stop 3, plus `cooldown ≥ 30` (counted only in `Neutral`), knockback 60 px/s | `take_hit_split`, `MISSILE_FEEL` |
| Llama health | 3; missile damage 1 | `LLAMA_HEALTH`, Q18 knob |
| Llama walk | 40 px/s = 0.67 px/tick | `LLAMA_WALK_SPEED` |
| Rush trigger (this sheet) | 128 px | CRL-Q05 |

**Consequences.**

1. **A llama in a planted Donut's line is locked.** 18 + 3 stagger ticks against a 16-tick refire: every hit lands before the previous stagger ends. It never walks, tells or rushes again, and dies about 32 ticks after the first hit.
2. **Approach speed cannot fix it.** From the 290 px reach edge, the first bolt lands after 39 ticks at 40 px/s (gap then 264 px), 38 ticks at 60 px/s (252), 33 at 120 px/s (224) and 26 at 240 px/s (186). Even at rush speed the llama is staggered 58 px outside the trigger and locked from there.
3. **A tell inside reach is interruptible in time.** A rush tell started at 128 px is interrupted if Donut presses by attack tick 14; at 96 px by tick 20; point-blank by tick 27 ([section 6](#interrupt-windows)). A bot reacts at tick 0.
4. **Cover on one floor stalls, it does not threaten.** A solid that blocks the bolt (16..22 px up) also blocks the llama body (0..39). Behind cover, the llama can neither be hit nor reach her.
5. **The light guard is not a lever** (DV-6): `missile_hits_llama` never reads it, and even total stagger immunity leaves three bolts inside the approach time.
6. **Mana bounds the lock but is not a lever.** The first 8 charges cover both llamas (6 hits) with 2 to spare. The B1 counterexample, a jump-cast clear on the Revision 1 layout, used exactly 6. Mana stays outside the tuning authority (plan section 14), and **the gate does not rely on it**. The section 10 invariant holds with unlimited mana.
7. **Casting is not grounded-only.** `request_cast` has no grounded check, and the bolt spawns at `feet + 19` at release (cast age 4) at whatever height her feet are, then flies level. A Donut who jumps in place picks her bolt's height, from 16..22 standing up to about 131..137 at the apex. Height separation must therefore hold for **every** release height, not only the standing one (review B1).

**What can stop a planted Donut, ranked:**

| Rank | Lever | Effect on a static start | Used here |
| --- | --- | --- | --- |
| 1 | **Height plus a blocking wall.** Her bolt flies level, 16..22 px above her feet at release; a llama body spans 0..39 px above its own feet. **Downward:** from a surface 32 px or more above a llama's floor, every bolt passes over it (bottom 48 or more against a top of 39). **Upward:** jumping lets her reach any higher body, so height alone fails (B1). A solid between the levels whose top is at or above the upper llama's top (71) stops every bolt low enough to touch that llama | From the floor she hits only B; from the dock, only A; from every other surface, neither. **No static clear at all**, with or without jumps | **Yes:** pen floor, exit dock and dock bumper (DV-2, CRL-Q41) |
| 2 | **Facing and placement.** CORRAL-AUTO fixes her facing; a llama behind her is never hit | Works for one facing only; the sweep runs both facings, and `stationary_turret` turns | Reserve lever (open question 3) |
| 3 | **A reveal inside the trigger.** The llama becomes visible already within 128 px | She must react inside the interrupt window above; a bot always does, a human may not | **Yes:** the partition (floor) and the climb over the dock bumper (dock) |
| 4 | **Approach speed as data** | None under fire (point 2). It only changes the pace against Carl and against a Donut who is not firing | Kept as data at the legacy 40 px/s (CRL-Q04) |
| 5 | **The committed rule** | None against a locked llama, which never commits | Kept: it is what makes the rush a real threat to everyone who is not stunlocking it |

**Escalation risk.** The gate (no static clear of both llamas) holds by geometry for every release height, so the risk of failing it after tuning is low, and the KC3 fallback (a super-armoured llama) is not needed. The real risk is game feel: a skilled Donut can still plant once per level and stunlock that level's llama. CORRAL-AUTO reports this as `layer_static_clear` (diagnostic, not gated). If the owner finds the result too close to M1, the reserve levers are open question 3. None of them needs magic immunity.

## 4. Tactical role

**The corral llama is the same Bad Llama with one more answer. Against a hero standing in its glob's height band it spits as before; against a hero standing on its own floor within 128 px it braces, scrapes and rushes 96 px along the floor at 240 px/s, chest first. The rush is committed: a jump clears it, a step up onto a shelf escapes it, an early hit during the 32-tick brace cancels it, and a whiff leaves it standing 40 ticks with its back to you. Baited into the timber partition, it smashes through and stands dazed.**

| | Legacy llama (M1, all old fixtures) | Corral llama (M2) | Engineer |
| --- | --- | --- | --- |
| Threat | Spit at 42..48 px over the yard | Spit at 42..48 px **only when it can hit**; **rush** on its own floor | Ram on the floor |
| Punishes | Standing tall (Carl) in the open | Standing on its floor near it (anyone); standing in its glob band (Carl; Donut one step up) | Being busy at mid distance |
| Answers | Jump, cover, stay low (Donut) | Jump the rush, step onto a shelf, interrupt the brace, bait the partition; jump the glob | Jump, out-range, interrupt the point |
| Range | Stand-off 200, fire range 320 | Walks to 112 from a rush target; rush trigger 128; stand-off 200 from a spit-only target | Trigger 72, ram 80 |
| Body | 46 x 39, HP 3, leash ±160 | Same body and HP; authored leash interval | 24 x 38, HP 3 |

## 5. The rush: states and frame data (60 Hz, all data knobs)

Attack tick 0 is the tick the tell starts (the existing `attack_tick` convention: the spit releases on attack tick 36). Pixels from the llama pivot, x forward, y up from its feet. 240 px/s = 4 px per tick, so every number is exact.

| Phase (`RushPhase`) | Attack ticks | Motion | Boxes | Hits on the llama |
| --- | --- | --- | --- | --- |
| **RushTell** | 0..31 (32 ticks, CRL-Q06) | none; facing, direction and target **locked at tick 0** | body (hurtbox) | **Interruptible** like the spit tell: any hero hit (jab, kick, swat, pounce, missile) staggers it through the existing `Hurt` path and cancels the rush; `LlamaInterrupted` is logged |
| **CommittedRush** | 32..55 (24 active ticks, CRL-Q07) | 4 px forward per active tick, **no tracking, no turn** | body + **rush box** `[0, 28) x [2, 30)` (CRL-Q08) | **Committed** (the goblin smear and Engineer ram rule): damage and hit-stop only, no stagger, no push; a lethal hit puts it `Down` in place and ends the rush |
| **Recover** (whiff, hero hit, leash stop) | 40 ticks (CRL-Q09) | stops dead | body | Any hit staggers it (the legacy `Hurt` path) |
| **Bonk** (box met a solid or the partition) | 60 ticks (CRL-Q10) | stops with the box front on the face | body | Damage and hit-stop only, timer unchanged; a lethal hit kills it (the Engineer bonk rule) |
| Cooldowns | `rush_cooldown` 150 after Recover or Bonk ends; the spit `cooldown` is raised to at least 30 at the same time (CRL-Q12) | | | An interrupted tell sets `rush_cooldown ≥ 30` and the legacy `cooldown ≥ 30` (CRL-Q13) |

**One active tick, in this order (step first, then test; the Engineer order):**

1. **Step:** move 4 px. If the rush box would pass the face of an intact solid, clamp so the box front rests on the face and mark `met_solid`. If the step would cross the llama's authored leash bound, clamp to the bound and mark `leash_stop`. Remnants of 16 px or less are not solids for a llama (CRL-Q32).
2. **Hero:** the first hero whose hurtbox the box overlaps (heroes stepped earlier this tick; if both overlap on one tick, the one whose near edge is closer to the box front, ties to Carl) takes the blow. An invulnerable hero, or a guarded off-screen hero (D1 M4), is not hit and does not stop the rush. The blow goes through `hurt_hero` with `HeroBlow { source: LLAMA_RUSH_SOURCE_ID, damage 1, from_x: llama x, knock_vx: 120 in the rush direction, knock_vy: 160 }` (CRL-Q14), so that path's invulnerability (55 ticks), hurt ticks, hit-stop 4 and the solo last-HP guard apply. The llama takes hit-stop 4 (`HERO_DEFEAT_HIT_STOP` when the blow defeats the hero). One hero per rush; the rush ends → Recover; `LlamaRushEnd { end: Contact(hero) }`.
3. **Solid:** `met_solid` → Bonk; `LlamaRushEnd { end: Blocked }`. If the solid is the breakable partition, the same tick sends it the rush stimulus ([section 10](#10-the-partition-optional-corral-d)) and logs `LlamaRushEnd { end: Partition }`.
4. **Leash:** `leash_stop` → Recover; `LlamaRushEnd { end: Miss }`.
5. **Distance:** 24 active ticks elapsed → Recover; `LlamaRushEnd { end: Miss }`.

**Hits on a rushing llama, exactly (F10).** Main's `take_hit_split` always sets `Hurt` and slides by `knock_vx`. For a corral-profile llama it branches first:

| Llama state when the hit lands | Non-lethal hit | Lethal hit |
| --- | --- | --- |
| RushTell, Recover, `Neutral`, spit `Attack` | The legacy path: `Hurt` 18, the slide, `cooldown ≥ 30`. During RushTell it also sets `rush_cooldown ≥ 30` (CRL-Q13) and logs `LlamaInterrupted` | Legacy `Down` |
| **CommittedRush** | Damage, and hit-stop by the hit's split. **No** mode change, **no** `vx`, `attack_tick` and the rush phase untouched, so the rush continues on the next unfrozen tick | `Down` on the spot, with the legacy death slide (clamped, CRL-Q43). The rush ends with no `LlamaRushEnd` contact, and the box is gone that tick |
| **Bonk** | Damage and hit-stop only. The bonk timer, mode, `vx` and `attack_tick` are untouched | `Down` on the spot (clamped death slide) |

**Slides stay on the llama's ground (F5, CRL-Q43).** Main's `Llama::slide` clamps only to solids and the layout edge. For a corral-profile llama, every slide (knockback, kick push, death slide) and every rush step is also clamped to its authored leash interval. When a clamp stops the llama, `vx` becomes 0. Llamas have no gravity, so this clamp is what keeps the dock llama on the dock. `Down` and `Corpse` are clamped too, so the system-failure check "a llama outside its leash" applies in every mode.

**Instance token:** a per-`CombatState` `rush_seq` counter starts at 1 and increments at each RushTell start; mixed into the state hash only when non-zero (the Engineer's `ram_seq` precedent). One rush instance hits at most one hero once. There is no per-frame contact damage, and the llama body itself never hurts.

**The rush passes through** the other llama, goblins and lying bodies. It does not bowl anything (no CF02-B contacts in v1).

### Selection: one brain, two attacks (each tick in `Neutral`, corral profile only)

1. **Wake** (CRL-Q24, the per-llama activation state of plan R1): `Dormant` until its whole body is inside the simulation view and a living hero is within 320 px, then `Awake` for the rest of the attempt. While dormant it idles and holds its position. Each llama's activation state is instance data, hashed only while its profile is set, and `reset_attempt` restores it.
2. **Classify each living hero within 320 px** (pivot to pivot; a hero farther away is never a target, CRL-Q15). A hero is classified by her **support height**: her feet minus `feet_height_above_solid` (`prototype.rs`), which is the top of the highest intact solid under her collider. A hero mid-hop over the pen floor is therefore still a floor hero (F6).
   - `rushable`: support height within **8 px** of the llama's floor line (CRL-Q16), whether she is airborne or not. A hop keeps her a rush target, so it neither turns her into a spit target nor hides her. A tell still **starts** only while she is grounded (step 4), so the section 6 windows apply from the first tell tick.
   - `spittable`: the glob band `[llama feet + 42, llama feet + 48)` overlaps the hero's hurtbox **as if she stood at her support height**, and no intact solid lies on the glob's path between the mouth and her column in that band (CRL-Q11). A hopping floor Donut is never spat at (the glob would pass over her once she lands), and a floor Donut is never spittable by the dock llama.
3. **Target** (KC1 rule, CRL-Q15): the party's `EnemyTargetRule` (hysteresis, solo bias to the controlled hero). It looks at the rushable heroes if there are any, else the spittable heroes, else all heroes within 320 px (for facing only); with none of those, the llama holds. The target locks at the tell start.
4. **Rush** if all hold: `rush_cooldown == 0`, `cooldown == 0`, `vx == 0`, the target is rushable and **grounded**, `|dx| ≤ 128`, the **fair-rush view rule** (CRL-Q17: the llama body, the whole swept lane `[start body, start pivot + 96 + 28]` and the target are inside the simulation view), and the **rush lane** is clear: no intact solid other than the breakable partition between the llama's body front and the target's column at heights `[2, 30)` (CRL-Q19).
5. **Else spit** if `cooldown == 0`, `vx == 0`, the target is spittable, and the legacy `may_fire` holds (body in view, target within 320 px in the facing).
6. **Else walk** at the profile's approach speed (40 px/s), never past its leash interval and never into a solid: toward a rushable target until `|dx| ≤ 112`; toward a spittable-only target until `|dx| ≤ 200` (the legacy stand-off); otherwise hold. It never walks backward.

Spit and rush are mutually exclusive by construction (one `Attack` at a time) and have separate cooldowns. No script alternates them: the target's height and distance choose.

### Code shape (KC5; #190 review F3 and F8; plan R1 and R2)

- `src/llama_rush.rs` (new, Bevy-free): `RushProfile` (loaded, validated and FNV-pinned from `enemy_llama_rush_v0.json`), `RushPhase`, `RushState { phase, tick, dir, target, start_x, seq }`, `rush_box(profile, x, dir, feet) -> Rect`, `rush_motion(...)` (step, solid clamp, leash clamp), `rush_lane_clear(...)` and `glob_lane_clear(...)` with the **llama's** half-width 23 and body back shift 3. `engineer::ram_box` takes `&EngineerTuning` and `lane_clear` uses the goblin half-width, so neither is reused (F8). `LLAMA_RUSH_SOURCE_ID: u32 = 0x4C52` ("LR"). Main has no distinct-source-id test yet, so CORRAL-B wire **creates** it, covering every hero-blow source: `0x4255`, `0x5241`, `0x5348`, `0x504C`, `0x4C52`, and the index-based goblin, Engineer and glob sources. `0x4C52` collides with none of them. The constant id does not say which llama hit, so the "damage by source" diagnostic takes the llama index from `LlamaRushEnd { end: Contact(hero) }` (N5).
- `src/llama.rs`: `Llama` gains these fields (**per llama**, L2):
  - `rush: Option<RushProfileRef>`;
  - `attack: AttackKind { Spit, Rush }` and `rush_state`;
  - the authored **leash interval** `(leash_min, leash_max)`, which replaces `home_x ± LLAMA_LEASH` for this llama only;
  - the **activation state** `Dormant | Awake` (R1).

  All of these are mixed into the hash **only while `rush` is `Some`**. Inside `llama.rs`, every spit assumption becomes kind-aware (F3): `telling()` is `attack_tick < 32` for a rush; the glob is released only for `AttackKind::Spit`; `target_state()` maps RushTell → `Tell`, CommittedRush → `Impact` (so a swat guard clears on it), and Recover and Bonk → `Recovery`. `take_hit_split` follows the F10 table above, and `slide` follows the CRL-Q43 clamp. No `LlamaMode` variant is added.
- `src/combat_scene.rs`, **per-llama spawn surface** (R1): `LlamaSpawn { x, ai }` gains an optional `feet` (the authored surface height; `y = feet + 19.5`, `floor_y = feet`). It also gains an optional `opt_in` that carries the profile, the leash interval, the facing and the activation. A llama with no `feet` keeps `enemy_foot_y`, so every legacy spawn is unchanged. The load check is widened: an authored `feet` must equal the top of an intact solid under the whole body, or the floor line. A llama with no opt-in keeps every legacy rule.
- **Name replay** (plan KC15 and R2): `corral_fixtures::llama_opt_ins(name)` is applied in `reset_attempt`, following the `cf02_fixtures::shore_hits(name)` precedent. It sets party targeting and each llama's profile, spawn surface, leash, facing and activation. The opt-ins live on `CombatState`, because `reset_attempt` rebuilds the llamas from `fixture.llamas`. A retry and a name replay therefore match the builder-built run.
- **Readers outside `llama.rs`** (they still compile; behaviour listed with its owner): `combat_anim.rs`: CORRAL-B wire makes the llama frame choice kind-aware so a rush **never** shows the throat glow (idle frames plus a debug outline), CORRAL-E draws the stand-in, LLAMA-03 the art (#181 gate (c)); `combat_scene/companion.rs` (companion view): the KC7 gap, appended by CA2 ([section 9](#9-solo-2p-and-the-ai-partner-c7)); `autoplay/observation.rs` and `autoplay/diagnostics.rs`: unchanged, a rush reads as an attack with `attack_tick < 36` as "tell" (documented, since party AUTOPLAY has no llama scenario); `scene_camera.rs`: unchanged, its tell interest uses 36 (cosmetic); `death_present.rs`, `bin/floor01_scene.rs` debug: unchanged; `combat_scene/donut.rs`: reads `telling()`, which is now kind-aware.

## 6. Reach and fairness proof (KC18)

Physics: `RUN_SPEED` 185 (Carl), DM-Q01 207 (Donut); launches 440 and 500; gravity 1050; semi-implicit step (`vy -= g / 60`, then `y += vy / 60`); heroes step before llamas in a tick. Hero half-widths 12 (Carl) and 10 (Donut). The box at active tick `k` (attack tick `31 + k`) spans `[4k, 4k + 28)` ahead of the start pivot at heights 2..30. A hero is clear of it only with the feet at 30 px or more. Donut's feet are at 30 px or more on jump ticks 4..52, Carl's on 5..44 (tick 1 is the press tick).

- **Standing contact:** first `k` with `4k + 28 > d − h`. Donut is hit when `d < 134`, Carl when `d < 136`. The trigger (128) stays inside both, so every rush that starts can hit a hero who does nothing.
- **Bonk:** with a solid face `F` px ahead of the start pivot, bonk on the first `k` with `4k + 28 > F`, stopping with the pivot at `F − 28`; no bonk when `F ≥ 124`.

### Fairness table (computed from the physics above; the CORRAL-B core unit test reproduces it)

`d` is the pivot distance at the tell start. "Jump" is the window of attack ticks on which a jump in place clears. "Run" is the latest attack tick on which running straight away (and holding) still clears. Boosted Donut (Light on Your Feet, `all` loadout only) uses launch 547, with 62 air ticks and feet at 30 px or more on jump ticks 4..57. Her jump window starts at attack tick 0 at **every** `d`. Below `d = 80` it equals the base window; from `d = 80` up it is wider.

| `d` | Standing hit: active tick (attack tick) | Donut jump | Donut run, latest | Carl jump | Carl run, latest |
| --- | --- | --- | --- | --- | --- |
| 0..36 | 1 (32) | 0..29 | 17..27 | 0..28 | 11..23 |
| 48 | 3 (34) | 0..31 | 31 | 2..30 | 27 |
| 64 | 7 (38) | 0..35 | 35 | 6..34 | 32 |
| 80 | 11 (42) | 2..39 | 40 | 10..38 | 37 |
| 96 | 15 (46) | 4..43 | 44 | 12..42 | 43 |
| **112 (approach stop)** | **19 (50)** | **4..47** | **49** | **12..46** | **48** |
| 128 (trigger edge) | 23 (54) | 4..51 | 54 | 12..50 | 53 |
| 136 or more | never | any | always | any | always |

**The fairness invariant holds.** At every start distance, a hero who reacts to the tell within 15 ticks (250 ms) has a jump window of at least 14 ticks (Carl point-blank: ticks 15..28 inclusive), and usually 30 or more. Running must start early: at short range, within the first half of the tell (by attack tick 17 for Donut or 11 for Carl point-blank). Jumping is the late answer. The rush outruns both heroes (240 against 207 and 185), so a late answer goes up, not away. A hero on a 32 px shelf is above the box top (30) and is never hit (CRL-Q08).

**2P:** each hero has the windows above independently. One rush hits at most one hero, so two heroes in one lane can never both be hit. The partner can also cancel the tell from the side with any hit (Carl's jab, Donut's swat or bolt).

### Interrupt windows

Latest attack tick on which Donut, already facing the llama, can press Primary so the bolt lands before the commit (attack tick 32): 27 at 0..36 px, 25 at 64 px, 20 at 96 px, 17 at 112 px, 14 at 128 px. Carl's jab (startup 3, reach 34 px) interrupts only from point-blank.

### Punish after a clean jump

A whiff ends on attack tick 55; Recover covers 56..95. At the approach stop (`d` 112) the llama ends 96 px further on, 16 px short of where the hero stood, facing away. Carl landing from any clearing jump (lands on attack tick `J + 49`) turns and jabs by attack tick 98 at the latest jump, and by 95 for every jump up to tick 43. Donut lands at `J + 56` and her bolt spawns inside its body, so it hits on the release tick (4 ticks after the press). A missile staggers in any state anyway; Recover is the window in which the llama **cannot attack**, which is what Carl needs to close in.

## 7. Committed rule, blasts and the gauntlet (KC3, KC11, KC12)

- **CommittedRush** follows the project's committed-attack rule: damage and hit-stop only; no stagger, push, launch or knockdown; a lethal hit kills it in place (`Down`, the legacy death slide away from the attacker). RushTell and Recover stay fully interruptible. Bonk takes damage without changing its timer.
- **CP-Q26 blasts** (`all` loadout and the mastery arena only; no dynamite in the corral): the blast core is lethal to a rushing llama; the outer ring deals its damage and follows the committed rule (no stagger, no push) during CommittedRush.
- **CP-Q40 gauntlet:** whichever of CP-D and CORRAL-B wire merges second adds CommittedRush to `pierces_commit` (one test). The gauntlet arrives at the Juicer, so the corral story pin never sees it.

## 8. Evasion (C4) and the reflex

**Decision: the rush is not evadable** (`crlq18_rush_evadable = false`). The glob stays evadable (DM-Q17).

1. **Rule fit.** The rush is the llama's **body** charging, not a weapon. DM-Q18 lists body contacts as never evadable and makes every future source non-evadable until its sheet opts in. The Engineer ram is evadable because it is a pot swung at a body.
2. **Gate fit.** An armed reflex would let a planted Donut take one rush per 12 s for free. Against KC19's "at most 1 HP lost" threshold, that free pass is worth a whole hit.
3. **Behaviour (F1).** A rush contact is a `HeroHurt`, so under DM-Q13 it **resets the reflex wait** (`last_reset_tick`), as any hurt does. The reflex is then disarmed for 720 ticks. The rush never **consumes** an evade: it emits no `HeroEvaded`. MAG-16 asserts that, in the corral, only the glob is ever evaded.

This is a review result, not an escalation. The owner can set the knob to `true` after playing; the hurt path then runs the DM-Q20 check inside the rush's hero test, as `ram_on_hero` does.

## 9. Solo, 2P and the AI partner (C7)

- **Targeting** is the party rule over eligible heroes (section 5). In solo the controlled hero is preferred by `target_ai_bias_px`. Solo Donut is a real target: she is rushable on the floor and spittable one step up. #181 gate (b) holds.
- **Party targeting is state-wide** (`CombatState::with_llama_party_target()`, KC1). The M1 llama uses it too. **The rush is per llama** (L2): `Llama::rush` is set from the sidecar roster or a fixture's index list; in room 0 the M1 llama and the M2 llamas share one `CombatState`, and only the M2 llamas carry the profile.
- **Swap:** the target locks at the tell start; a swap during the tell does not retarget.
- **AI partner hook.** No corral code goes into `src/companion/**` or `src/combat_scene/companion.rs` (KC7). `llama_rush.rs` exposes a read-only accessor:

  ```rust
  pub struct RushLane { pub phase: RushPhase, pub dir: i8, pub feet: f32,
                        pub x_from: f32, pub x_to: f32,   // swept extent of the box, leash and solids applied
                        pub y_from: f32, pub y_to: f32,   // feet + 2 .. feet + 30
                        pub ticks_to_motion: u16, pub active_left: u16 }
  pub fn rush_lane(llama: &Llama) -> Option<RushLane>;
  pub fn ticks_until_box_reaches(lane: &RushLane, hero_x: f32, half_w: f32) -> Option<u16>;
  pub fn ticks_until_box_clears(lane: &RushLane, hero_x: f32, half_w: f32) -> Option<u16>;
  ```

  Both counts are ticks from now: 0 means this tick, and `reaches` gives the first tick of overlap with the hero's column.

  The CA2 step that holds the companion files after CORRAL-B wire merges (CA2-B2 or CA2-C) appends the named change "CA2 rush lane". The lane joins `lane_clear`. The AI hero **jumps** when all of these hold:
  - its hurtbox column is in `[x_from, x_to]`;
  - its feet are below `y_to`;
  - `ticks_until_box_reaches` is **3..40 for Donut** or **4..31 for Carl** (F4: the box overlaps a hero column for up to 11 ticks for Donut and 12 for Carl, and these bounds are safe for both counting conventions).

  The preferred form is equivalent and more general: jump when the airborne span, during which the feet stay at 30 px or more above the lane, covers `[reaches, clears]`. Otherwise it steps onto a shelf or holds, and it never walks into a committed lane. Until that append, the AI's rush dodge is a reported gap, covered by the D1 last-HP guard. The companion-mode (`cf02_free`) corral fixtures land in the first corral owner step after CA2-B2.

## 10. The corral geometry (CRL-Q25 to CRL-Q33, CRL-Q41)

Room 0 is the Act I prefix of the `f01_late` scene (KC8): I-05 to I-07 plus the M3 threshold. One geometry file, `assets/levels/floor01/f01_corral.room.json` (KC16), existing schema, entity arrays empty; the encounter entities live in `f01_corral.encounter.json`. x grows right in room pixels; heights are above the M0..M2 walking line.

| Part | x | Height | Content |
| --- | --- | --- | --- |
| M0 First Spark | 0..448 | floor | 1–2 goblins (CORRAL-WIRE data); the biscuit is pinned at entry |
| M1 One Burning Throat | 448..1056 | floor | **Llama M1** at x 848, legacy leash ±160 (688..1008), no rush profile, legacy spit; no cover between the approach and the llama |
| M2 entry line | 1056 | | Camera intro; M2 checkpoint at x 1072 |
| **Entry shelf** (optional) | 1120..1216 | solid, top **32** | Stone mounting block; the "safe-looking perch" |
| **Partition** (optional, CORRAL-D) | 1392..1408 | breakable prop, **48** tall | Rotten timber screen; blocks bolts (16..22) and globs (42..48) |
| **Llama B** (floor, rush) | home **1488**, facing −1 | leash **1248..1830** | Corral profile, spawned on the floor (`feet` 0). At 1830 facing −1, its body's right edge is 1856, flush with the bumper face (FN4). While the partition stands, its solid stops B at 1428 |
| Pen floor | 1056..1856 | floor | The ground lane |
| **Exit dock** | 1856..2048 | solid, top **32** | Loading dock on the route |
| **Dock bumper** (CRL-Q41) | 1856..1888 | solid, top **80** (48 above the dock; 5 tiles) | A stack of timber dock bumpers on the dock edge. It is authored as one **step or parapet-style solid** running from the floor to 80 in the room's 16 px collision tile layer (`camera_lab::layout::compose`). It is **never** a platform, which would leave an open gap underneath that leaks bolts. 72 is not buildable: the composer would quantise it to 80. Heroes climb it (80 is Carl's DP-Q18 class limit and the CF02 perch height; Donut's class is 96) and step down onto the dock, or jump past it. It blocks bolts, globs and both rushes. Hero standing positions on the dock are pivots 1900..2036 |
| **Llama A** (dock, rush) | home **1968**, facing −1 | leash **1920..2016** | Corral profile, spawned on the dock surface (`feet` 32, R1). Its body stays inside 1894..2042, clear of the bumper (1888) and the M3 gate (2048) in both facings |
| M3 gate | 2048..2064 | full height | Closed until both M2 llamas are defeated (DV-3, CRL-Q26) |
| M3 Fast-food Threshold | 2048..2304 | **32** (dock level) | Quiet; no enemy, no grant. REST-A (#187) begins at its right edge |

**Lane ceiling** `ceiling_px` **228** (CRL-Q31). Every base hero jumps a 39 px llama on its own level (the floor or the dock) without a DM-Q27 bonk (KC10):
- Donut's highest such jumps reach 214.92 from the partition top (48 + 114.92 + 52) and 198.92 from the dock.
- Carl reaches 172.54 from the dock (32 + 88.54 + 52).

Two jumps do bonk, both harmlessly:
- A base Donut jumping from the bumper top bonks at 228 (246.92 unclamped), with her feet still 96 px above the bumper.
- A boosted Donut (`all`) bonks from the partition top (237.95) and from the bumper (269.95).

No llama has to be cleared from either surface. Carl from the bumper reaches 220.54, under 228; DP-Q45 already quotes 220.5 for an 80 px perch. The bumper top (80) and the partition top (48) make the DP-Q45 ceiling mandatory under `all`; 228 is that limit. No surface is between 81 and 128 px (K8).

**Height truth table (MAG-07).** ✓ = can hit, ✗ = cannot. "Jump-cast" means a bolt released at any height a jump in place gives.

| Attacker → target | Floor Donut (0) | Floor Carl (0) | Donut on 32 (dock or shelf) | Carl on 32 | Donut on 48 (partition) | Anyone on the bumper (80) |
| --- | --- | --- | --- | --- | --- | --- |
| B's glob, band 42..48 | ✗ under it | ✓ | ✓ on the shelf; ✗ on the dock (bumper blocks) | ✓ shelf; ✗ dock | ✗ above it | ✗ |
| A's glob, band 74..80 | ✗ | ✗ | ✗ | ✓ on the dock | ✗ | ✗ (band top 80 against a hurtbox from 80; strict overlap) |
| B's rush box, 2..30 | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ |
| A's rush box, 34..62 | ✗ | ✗ | ✓ on the dock | ✓ on the dock | ✗ out of lane (A cannot leave the dock) | ✗ above it |
| **Donut's bolt** → B (body 0..39) | ✓ standing and on low jump-casts (release feet below 23) | | ✗ (bottom 48 or more passes over) | | ✗ | ✗ |
| **Donut's bolt** → A (body 32..71) | **✗ at every release height**: below 64 the bumper stops it (A is touchable only below 55); from 64 up it passes over the bumper and over A | | ✓ from the dock; ✗ from the shelf (668 px away and blocked) | | ✗ (476 px away and blocked) | ✗ (bottom 96 or more passes over) |
| Carl's melee → A | ✗ (his farthest box ends at about 1866; A's body starts at 1894 or later, behind the bumper) | | | ✓ on the dock | | |

**B1 proof (f32, main's physics).** A script replicates the hero step from `prototype.rs` (`step_hero_driven`: semi-implicit, with the landing rule) and the bolt from `donut.rs`: spawn at `pivot ± 27`, `feet + 19`; tested at spawn, then every 6 px step; removed when `travelled ≥ 240`; solids tested before targets; strict `Rect::overlaps`. It sweeps every pivot on each surface on a 0.25 px grid, both facings, and every feet height at release that a jump in place gives with launches 440, 500 and 547, from tick 0 to landing (166 or 167 heights). It runs with the partition absent, intact and broken (the 16 px remnant), and from the remnant top as a surface (NB1). The targets are every body each llama can take inside its leash, in both facings.

| Surface (pivots) | Can hit A | Can hit B |
| --- | --- | --- |
| Revision 1 layout, floor (1068..1844) | **yes**: pivots 1589.25..1844, release feet 16.88..54.17 (the review's same-tick press, feet 37.29, A at home: 1679.25 up) | yes (release feet 0..22.04) |
| **Revision 2**, floor (1068..1844) | **none** | yes (release feet 0..22.04) |
| Revision 2, entry shelf (1109..1227) | none | none |
| Revision 2, partition top (1381..1419) | none | none |
| Revision 2, dock (1900..2036) | yes (release feet 0..22.04 above the dock) | **none** |
| Revision 2, bumper top at 72 (1845..1899) | none | none |
| **Revision 3** (bumper 80, B leash 1248..1830; partition absent, intact and broken), floor (1068..1844) | **none** | yes (release feet 0..22.04) |
| Revision 3, entry shelf (1109..1227) | none | none |
| Revision 3, partition top, intact (1381..1419) | none | none |
| Revision 3, remnant top at 16, broken (1381..1419) | none | yes (release feet 0..4.58 above the remnant) |
| Revision 3, dock (1900..2036) | yes (release feet 0..22.04 above the dock) | **none** |
| Revision 3, bumper top at 80 (1845..1899) | none | none |

So in Revision 3, the set of llamas a Donut can hit from one spot is {B} on the floor and the remnant, {A} on the dock, and nothing anywhere else, whatever her jump timing. The guarantee does not depend on mana, tell timing, wake or approach speed. It needs only these four values:
- A's leash and the CRL-Q43 slide clamp, which keep A's body at 1894 or later;
- the bumper top, which must be at least A's top (71); the adopted value is 80;
- the bumper column (1856..1888), which every bolt from a floor pivot of 1844 or less must cross; the bolt box is 8 px wide and moves 6 px per step, so it cannot tunnel through;
- B's floor line.

CORRAL-C turns this sweep into a unit test with the room's real solids (machine item 2, section 15), so a quantisation change like the one that turned 72 into 80 is caught.

**Nothing moves a stationary Donut to another level.**
- On the floor, only B can hit her. B's rush knocks her with `knock_vy` 160, a rise of about 12 px. Even caught mid-hop (feet below 30), she peaks under 45 px. She can never reach the 80 px bumper, which is the only way onto the dock from the floor.
- On the dock, A's rush knocks her toward the bumper. Its contact needs her feet below 30 above the dock, so even a boosted Donut (547) caught late in a hop peaks below 72.9 above the floor, under the 80 px top. She cannot land on it. At 72 px a boosted Donut at dock pivots 1914..1921.75, caught on jump tick 58, could land on it (re-review FN3). That case was harmless, because A cannot be hit from the bumper or the floor. The 80 px bumper removes it.
- Llamas have no gravity, and the CRL-Q43 clamp keeps them inside their leashes, so neither llama changes level.

This is the invariant the gate asserts ([section 11](#11-stationary-donut-gate-c19-and-the-corral-eval-scenarios)). The B1 option is therefore closed inside the tuning authority (geometry), and no KC3 escalation is needed.

**First-llama-easy rule (CRL-Q27).** The M1 llama has no rush profile, keeps the legacy spit (which sails over a grounded Donut and teaches the glob), the legacy stand-off 200 and leash ±160, and party targeting. Its pocket is flat with no cover. A planted Donut wins with three bolts at 0 damage (positive control M1-A and M1-B). Carl wins by jumping the glob and punching.

### The partition (optional, CORRAL-D)

- **Prop:** an `env_prop.rs` prop with a 16 x 48 solid at x 1392..1408 (KC17). States `intact → broken`. Bolts and globs stop on it (they already stop on solids).
- **Break stimuli:** a **rush contact** (the box front meets its face during CommittedRush): breaks at once, same tick; **three distinct physical hero hits** (Carl jab, haymaker or kick; Donut swat or pounce; one per move instance, through the prop core's melee intake): breaks on the third. **Magic Missile and globs do not damage it** (DV-9).
- **Rush into it:** the rush lane rule treats the partition as passable for selection, so B rushes a rushable hero standing within 128 px on the other side; this is the bait. The llama Bonks at the face (60 ticks, `LlamaRushEnd { end: Partition }`), the prop logs its break event (`partition_broken` in #181's naming), its solid toggles off at the end of the tick, and nonlethal splinters are presentation only.
- **Remnant:** 16 px (the prop core's). Heroes: the remnant-lift rule. Llamas ignore remnants of 16 px or less (CRL-Q32), like slot enemies. A floor bolt (box bottom exactly 16) passes over it under the strict overlap test, which a fixture pins.
- **Near miss:** a rush that stops short of the face (distance or leash) never breaks it. Nothing breaks it on a timer or a trigger.
- **Cut path:** if CORRAL-D is cut, the room has no partition and B's leash is the same; the gate still holds (section 11). The route is never blocked: the partition is jumpable by both heroes (48 px) and standable.

## 11. STATIONARY-DONUT gate (C19) and the CORRAL-EVAL scenarios

### Policies (CORRAL-AUTO section 2, refined)

| Policy | Rule | Role |
| --- | --- | --- |
| `stationary_cast` | Zero move, no jump; a press edge on every tick a cast can start and mana permits | Gated |
| `stationary_cast_burst` | Same spot; 8 closely spaced presses, then wait for a full refill | Gated |
| `stationary_cast_aimed` (**new**) | Same spot; presses only when a living llama body intersects the bolt's line within 290 px with no solid between | Gated: the strongest legal schedule |
| `stationary_turret` (**new**) | As aimed, but may turn toward the nearest llama in line with one-tick stick taps; net displacement at most 8 px | Gated: "standing still" includes turning in place |
| `stationary_jump_cast` (**new**, B1, CRL-Q42) | Zero horizontal input except `stationary_turret`'s turn taps. Every Primary press is paired with a Jump press, and a new Jump is pressed only while grounded. Variant parameter `o` is the Primary press tick minus the Jump press tick, over −3..52; `o = 0` is the same-tick press (release on jump tick 5). Variant `aimed`: before each press, the policy picks the smallest `o` (or a grounded cast) whose release band, computed from the jump arc, overlaps a living llama body in line within 290 px with no solid between | Gated: a hop in place is not repositioning. The `aimed` variant is the strongest jump-cast schedule |
| `press_continuously` | Primary held | Reported, labelled trivial |
| `no_interaction` | Neutral input | Reported (llama readability) |
| `mobile_cast`, `carl_melee`, `coop_probe` | As CORRAL-AUTO, observable state only | Must win ([section 12](#12-acceptance-criteria)) |

### Starts (Donut pivot at tick 0, set at setup, never moved later)

The evaluator runs in solo mode with Donut controlled, so the sim camera follows her (`CameraFollow::ControlledSoloMidpointTwoPlayer`). Its corral-owned constructor wraps `PartyFixtureSim::from_parts` and sets Donut's x, y and facing through `donut_body` and `sync_donut_for_tests()` (N2). The two companion variants:
- **Decisive (`isolated`), F3:** Carl is **parked** at x **880**, through the solo drive `PartyDrive { live: true, two_player: false, controlled: Donut, bots: [false, false], tuning: PROVISIONAL_TUNING }` (without `live: true`, Donut replays Carl's trail and the gate is invalid) with a neutral input on Carl's seat on every tick (NB2). A `two_player` drive would move the sim camera to the midpoint and engage the 448 px separation wall, and a bot drive would run `follow_input`, so neither is used. He has no companion AI and no `FollowAndJumpBot`, and stands on the floor for every M2 start. That is 368 px or more from every position B can take, and the corral llamas never target a hero farther than 320 px (CRL-Q15). So he draws no attack, absorbs no rush and is never in a lane. The `corral_m2` fixture has no M1 llama.
- **M1 starts:** Carl is parked at x **300**.

Each start runs **both facings**; the listed facing is the natural one.

| ID | x | Surface (feet) | Facing | What she can hit / what can hit her | Expected terminal (gated policies) |
| --- | --- | --- | --- | --- | --- |
| M1-A | 608 | floor | +1 | M1 llama (d 240) / glob over her | **victory**, 0 damage (positive control) |
| M1-B | 560 | floor | +1 | M1 llama (d 288) / none | **victory**, 0 damage |
| M2-01 entry | 1072 | floor | +1 | B once in line (partition absent or broken) / B's rush only if it gets in, which it cannot under fire | B defeated or none; **timeout** |
| M2-02 entry shelf | 1168 | 32 | +1 | nothing / B's glob (partition absent or broken) | **timeout** or **hero_defeat** |
| M2-03 shelf edge | 1204 | 32 | +1 | nothing / as M2-02 | as M2-02 |
| M2-04 approach lane | 1248 | floor | +1 | B / as M2-01 | **timeout** (B may die) |
| M2-05 cover edge | 1376 | floor | +1 | B after the partition breaks / B's rush into the partition | **timeout** (B dies after the break) |
| M2-06 partition top | 1400 | 48 | +1 | nothing in reach / nothing | **timeout**, 0 defeats (intact only) |
| M2-07 reveal | 1424 | floor | +1 | B at d 64 / B's rush | **timeout** (+1); **hero_defeat** likely (−1, B behind her) |
| M2-08 mid pen | 1640 | floor | −1 | B (d 152) / B's rush from behind if facing +1 | **timeout** (−1); **hero_defeat** likely (+1) |
| M2-09 dock face | 1840 | floor | −1 | B (d 352) / nothing from A | **timeout** |
| M2-10 dock arrival | 1904 | 32 | +1 | A (d 64) / A's rush; B cannot reach her (the bumper blocks its glob) | **timeout** (A defeated, B alive) or **hero_defeat** |
| M2-11 dock far | 2032 | 32 | −1 | A (d 64) / A's rush | **timeout** (A defeated, B alive) or **hero_defeat** |
| M2-12 bumper top | 1872 | 80 | +1 | nothing / nothing | **timeout**, 0 defeats, 0 damage (the stall spot) |
| M2-13 B1 spot | 1728 | floor | +1 | B (behind her in the +1 facing; `stationary_turret` and `stationary_jump_cast` turn to it), then **not** A at any release height / B's rush | **timeout** (B defeated, A untouched) or **hero_defeat**. This is the review's counterexample, pinned for every policy |
| M2-G grid | floor 1072..1840, shelf 1136..1200, partition top 1392..1408 (intact only), remnant top 1392..1408 at 16 (broken only), bumper top 1856..1888, dock 1904..2032, step 16 | | both | | none may defeat both llamas |

**Variants per start:** partition `intact`, `broken` (authored pre-broken, remnant present) and `absent` (if CORRAL-D has not landed, only `absent` runs and the others are `not_run`); llama attack phase, written at setup (tick 0) as per-llama `(cooldown, rush_cooldown)`: P0 = B (0, 0) and A (45, 75), P1 = B (45, 75) and A (90, 0), P2 = B (90, 0) and A (0, 0); reflex `off` and `reflex` (`dm_q11_evasion_mode`, gate decided on `reflex`, F6); companion `isolated` (Carl parked, above; decisive) and `default` (Carl starts at 1040 on the companion AI; reported). `stationary_jump_cast` runs every `o` on the named starts. On the M2-G grid it runs only `aimed`, `o = 0` and `o = 45`. `reset_attempt()` rebuilds everything from the fixture. Seeds do not vary gameplay, so the report counts fixtures, not seeds.

**Setup writes are exempt (F11).** CORRAL-EVAL core asserts "no state writes" after setup. It exempts the tick-0 setup: Donut's x, y and facing, each llama's `(cooldown, rush_cooldown)` phase pair, Carl's parked position, and the partition's pre-broken state. Each is written once, before the first stepped tick, and is never written mid-run (plan section 14.4).

### Gate (asserted, not only printed)

1. **KC19 failure rule (kept):** fail if any start, with any gated policy, in the decisive variant (isolated, reflex on), defeats both M2 llamas within 1,800 ticks with Donut losing at most 1 HP of 4 and not defeated, in 50 % or more of its phase variants. Report the best counterexample.
2. **Geometric invariant (new, CRL-Q35):** no run of a gated policy (`stationary_jump_cast` included) ends with both M2 llamas defeated by **Donut-credited** hits (`hit_by` Donut).
   - In the `isolated` variant Carl never attacks, so this means both llamas defeated at all.
   - In the `default` variant, a llama the companion AI killed does not count against the invariant (F2). Such runs are reported.

   A failure means the geometry leaked: a llama changed level or left its leash, a reach or bumper number is wrong, or the hero was displaced to another level. It is a hard failure even if she took damage.
3. **Positive control:** M1-A and M1-B end in victory with 0 damage for `stationary_cast_aimed`.
4. **Movement success:** at least one `mobile_cast` (Donut solo, isolated Carl), one `carl_melee` and one `coop_probe` victory, each in the `intact` (if built) and `absent` partition states.
5. **System failures (hard):** a rush hit before its commit tick; a glob from a rush; an offscreen tell; a corral llama, in any mode including `Down` and `Corpse`, outside its leash or changing level (CRL-Q43); a bolt hitting A from a floor pivot; Carl-only targeting while Donut is controlled and eligible; a different event log or state hash on the in-process double run; a rush, partition or gate state not reset on retry.
6. **Diagnostics (reported, not gated):** `layer_static_clear` (a static run that defeats the llama of its own level), rush tells, commits, contacts, misses, bonks, interrupts by hero and move, partition breaks by cause, damage by source and hero, companion contribution, distance travelled.

## 12. Acceptance criteria

Machine (CI):

1. `enemy_llama_rush_v0.json` loads, validates (a load error names the field) and matches its FNV-1a pin; a completeness test proves every CRL-Q has a knob or a recorded no-knob reason. Validation: speed a positive multiple of 60 and at most 240; approach ≤ trigger; trigger < standing reach (`active × speed / 60 + box x1`); box non-empty with `y1 ≤ 39`; tell, active, recover, bonk ticks > 0.
2. The section 6 tables are reproduced by a domain unit test in `llama_rush.rs` (standing contact tick per `d`, jump-window edges for both heroes at `d` 64, 96 and 112, bonk tick per face distance, the run-away edge at `d` 96). The section 10 B1 sweep is reproduced by a unit test (`corral_fixtures.rs`, CORRAL-C, with the room's real solids and leashes). Across every surface pivot on a 1 px grid, both facings, and every release height of launches 440, 500 and 547, the reachable-llama set must be {B} on the floor, {B} on the 16 px remnant top (partition broken), {A} on the dock, and empty on the entry shelf, the intact partition top and the bumper top. A change to the bumper, the dock, a leash or the bolt geometry that breaks this fails CI.
3. Every fixture in [section 15](#fixtures-and-tests-per-step) passes and replays identically twice; pinned fixtures are pinned after DM-B (KC13).
4. **Unchanged goldens:** the 5 llama `EXPECTED_HASHES`, every `cf02_llama_*`, `cf02_donut_solo_llama`, the arena and Engineer goldens, and every other existing replay are byte-identical (no llama outside the corral has a profile or party targeting).
5. The section 11 gate passes on the final head (CORRAL-C onward).

Evidence (CORRAL-F): MAG-01 to MAG-16 statuses with `NOT RUN` where true; 640 x 360 and 640 x 400 clips at 1x of a rush tell beside a spit tell (#181 gate (c)), a jump over a rush and a punish, a point-blank interrupt, a partition break and the opened lane, a solo Donut corral clear, a Carl melee clear, a 2P clear; the CORRAL-AUTO before and after report.

Human (owner, game feel; the item stays "merged, game-feel review pending"):

6. The owner can tell a rush tell from a spit tell at 1x without being told.
7. The owner moves between levels because the room asks for it, not because a rule forces it.
8. The rush feels like a llama throwing its weight, not a teleport or a cheap shot, and the partition break feels earned.

## 13. Adopted defaults (CRL-Q01..CRL-Q43)

Knob names start with the question ID (`crlq05_trigger_px`). Profile knobs live in `assets/data/gameplay/enemy_llama_rush_v0.json` (CORRAL-B core); per-llama instance data lives in the fixture (reapplied by `corral_fixtures::llama_opt_ins(name)` in `reset_attempt`) or in the encounter sidecar. That data is the profile reference, the spawn surface `feet`, home, the leash interval, facing and activation (R1, R2, F7).

| ID | Question | Adopted value | Rationale | Knob |
| --- | --- | --- | --- | --- |
| CRL-Q01 | Role | Close-range floor charger added to the spitter; [section 4](#4-tactical-role) | One new action (#181) | none (design) |
| CRL-Q02 | Opt-in | **Per llama** (`Llama::rush: Option<_>`), from the sidecar roster or a fixture index list; hashed only while set | L2: M1 and M2 share one state | instance data |
| CRL-Q03 | Party targeting | State-wide builder `with_llama_party_target()`; the M1 llama uses it too | KC1, #181 gate (b) | none (builder) |
| CRL-Q04 | Approach | Walk **40** px/s (legacy) toward a rushable target until **112** px, toward a spittable-only target until **200** px; never backward; never past the leash | D2: speed is not a lever under fire; 112 sits 16 px inside the trigger | `crlq04_walk_px_s`, `crlq04_rush_approach_px`, `crlq04_spit_standoff_px` |
| CRL-Q05 | Trigger | Rushable target within **128** px, no minimum | DV-1, section 6 | `crlq05_trigger_px` |
| CRL-Q06 | Tell | **32** ticks, interruptible | Inside #181's 30..36; section 6 windows | `crlq06_tell_ticks` |
| CRL-Q07 | Commit | **240** px/s for **24** active ticks (**96** px); no tracking | ≤ camera cap (KC18); inside #181's 70..117 px | `crlq07_speed_px_s`, `crlq07_active_ticks` |
| CRL-Q08 | Rush box | `[0, 28) x [2, 30)` from the pivot | Chest-first, low; below the 32 px shelf; ≤ 39 px body | `crlq08_box` |
| CRL-Q09 | Recover | **40** ticks | Carl's punish lands inside it for every clearing jump up to attack tick 43 | `crlq09_recover_ticks` |
| CRL-Q10 | Bonk | **60** ticks, self-damage **0**, timer unchanged by non-lethal hits | A real punish without a free kill | `crlq10_bonk_ticks`, `crlq10_self_damage` |
| CRL-Q11 | Corral spit rule | Spit only if the glob band overlaps the target's hurtbox at her **support height** (section 5) and the glob path is clear | DV-4, F6 | `crlq11_spit_height_check` (switch) |
| CRL-Q12 | Cooldowns after a rush | `rush_cooldown` **150**; spit `cooldown ≥ 30` | #181's 2–3 s; no instant spit after a rush | `crlq12_rush_cooldown`, `crlq12_spit_gap` |
| CRL-Q13 | Interrupted tell | Legacy `Hurt` path; `rush_cooldown ≥ 30` | Pressure resumes after the stagger | `crlq13_interrupt_cooldown` |
| CRL-Q14 | Contact | `hurt_hero`, damage **1**, knock **120 / 160**, one hero per rush, llama hit-stop 4 | Engineer Q11 precedent | `crlq14_damage`, `crlq14_knock_x`, `crlq14_knock_y` |
| CRL-Q15 | Target order | Only heroes within **320** px are candidates. The hysteresis rule picks among the rushable ones, else the spittable ones, else all candidates (for facing only); with no candidate, the llama holds. The target locks at the tell start | ENG-G-Q06 precedent; F3 (a far hero draws nothing) | `crlq15_target_range_px` |
| CRL-Q16 | Rushable tolerance | Support height within **8** px of the llama's floor line, airborne or not. A tell starts only against a grounded target | ENG-G-Q06; F6 | `crlq16_floor_tolerance_px` |
| CRL-Q17 | Fair-rush view rule | Body, whole swept lane and target inside the simulation view at the tell start | F7, #181 gate (f) | none (rule) |
| CRL-Q18 | Evasion | **Not evadable** | Section 8 | `crlq18_rush_evadable` |
| CRL-Q19 | Rush lane | No intact solid between body front and target column at 2..30, except the breakable partition | Bait rule | none (rule) |
| CRL-Q20 | Committed rule | Damage and hit-stop only; lethal kills | KC3 lever 3, the existing rule | none |
| CRL-Q21 | Source id | `LLAMA_RUSH_SOURCE_ID = 0x4C52` | KC6 | none (code) |
| CRL-Q22 | Events | `LlamaRushTell { tick, llama, target }`, `LlamaRushCommit { tick, llama, dir }`, `LlamaRushEnd { tick, llama, end: Contact(hero) \| Miss \| Blocked \| Partition }`; interrupts use `LlamaInterrupted`; all appended to `CombatEvent` | #181's `charge_*` names map 1:1 | none |
| CRL-Q23 | Instance token | `rush_seq` per `CombatState`, hashed when non-zero | Engineer precedent | none |
| CRL-Q24 | Wake | Per-llama activation state `Dormant` → `Awake` when its body is in view and a hero is within 320 px; restored on retry | #181 "camera-safe sight"; plan R1 | `crlq24_wake_px` |
| CRL-Q25 | M2 layout | Section 10 table; B spawned on the floor (`feet` 0), A on the dock (`feet` 32) | D2 rank 1; R1 | room and sidecar data |
| CRL-Q26 | M3 gate | Closed until both M2 llamas are defeated; opens on the second defeat tick | DV-3 | `crlq26_exit_gate` (switch) |
| CRL-Q27 | First llama | No profile, legacy spit, legacy stand-off and leash, party targeting, flat pocket | #181 M1 | instance data |
| CRL-Q28 | Llama homes and leashes | M1 848 (688..1008); B 1488 (1248..**1830**, FN4); A 1968 (**1920..2016**) | Section 10; A's body stays clear of the bumper and the gate | instance data |
| CRL-Q29 | Shelves | Entry shelf 1120..1216 at 32; exit dock 1856..2048 at 32 (its first 32 px are the bumper, CRL-Q41) | DV-2 | room data |
| CRL-Q30 | Partition | 1392..1408, 48 tall; rush breaks at once; 3 physical hits break; missile and glob do not | DV-9 | sidecar prop data |
| CRL-Q31 | Ceiling | `ceiling_px` **228** | Section 10 | room data |
| CRL-Q32 | Remnants and llamas | Remnants of 16 px or less are not solids for llamas | Slot-enemy rule | none (rule) |
| CRL-Q33 | Grants | Room 0 grants nothing; no dynamite | DV-7, K1 | story pin |
| CRL-Q34 | Gate policies | Adds `stationary_cast_aimed`, `stationary_turret` and `stationary_jump_cast` (swept `o`, plus `aimed`) | DV-8, B1 | evaluator constant |
| CRL-Q35 | Gate invariant | No gated static run ends with both M2 llamas defeated by Donut-credited hits (in `isolated`: defeated at all) | Section 11; F2 | evaluator assertion |
| CRL-Q36 | Phase variants | P0..P2 as written in section 11 | F13: setup-time cooldowns | evaluator data |
| CRL-Q37 | Bowling | The rush does not bowl goblins or llamas in v1 | Keep v1 small; the corral has no goblins | none |
| CRL-Q38 | Presentation stand-in | Code-drawn brace, hoof scrape and dust; never the throat glow | #181 gate (c); CORRAL-E | none |
| CRL-Q39 | Art | Production rush art is **LLAMA-03** (tell, rush and recover keys; LLAMA-02 is the merged death set) | F2 | none (process) |
| CRL-Q40 | Kill switch | `crlq40_enabled` (default `true`): `false` makes every corral llama run without its profile | Feel A/B without a rebuild | `crlq40_enabled` |
| CRL-Q41 | B1 fix (adopted option) | **Dock bumper**: one step or parapet-style solid at x 1856..1888 from the floor to **80** (5 collision tiles), on the dock edge. Its top must be at least the dock llama's top (71), and every floor-to-A bolt path must cross it. 72 is not buildable in the 16 px tile layer | Section 10 proof. Rejected: a dock about 300 px deeper (lengthens the room, and the dock llama could not threaten the dock's near half); a pincer (`stationary_turret` turns, so it fails) | room data |
| CRL-Q42 | What "stationary" means for the gate | Zero net horizontal displacement (at most 8 px for turns). **Jumping in place and casting in the air count as stationary** | B1; #181 gate (a): a hop is not repositioning. The owner may narrow it (awareness note, section 14) | none (evaluator rule) |
| CRL-Q43 | Slide clamp | Every slide and every rush step of a corral-profile llama is clamped to its leash interval, in every mode, `Down` and `Corpse` included | F5: a kick or death slide would otherwise put A's body over the floor | none (rule) |

## 14. Genuinely open questions

1. **Feel of the rush on a real pad:** 32-tick brace, 240 px/s over 96 px, 40-tick recover. Does the brace read as "it is coming for me" at 1x with the stand-in?
2. **Evasion (game-feel note):** keep the rush non-evadable, or flip `crlq18_rush_evadable` to match the Engineer ram.
3. **Per-level stunlock feel:** if planting once per level still feels like M1 (the `layer_static_clear` diagnostic), the reserve levers are, in order: the second llama placed so it wakes behind the heroes' entry facing (rank 2); a shorter tell for the dock llama only; the dock arrival point moved inside A's trigger. None adds immunity.
4. **The exit gate:** keep it, or let a fast player skip the dock llama.
5. **Awareness notes (adopted defaults the owner may revert):**
   - DV-2: a mandatory dock level with the second llama, behind an 80 px bumper, and M3 at +32.
   - **The bumper top is a stall spot.** Nothing reaches a hero there: A's glob band tops out at 80, B's glob and both rushes are below it. Nothing fired from there hits a llama. A player can hide on it but cannot win from it. Neither the gate nor the invariant is affected; this is a feel question only.
   - DV-10: the partition stands on the floor route.
   - CRL-Q42: the gate counts a hop in place as stationary. That is broader than CORRAL-AUTO's literal "never jumps".

## 15. Implementation brief and acceptance fixtures

Do not edit `docs/04-production/backlog.md`; status goes in the PR body. No design decision is left to an executor; a fixture whose outcome contradicts this sheet is reported as a design defect.

| Step | Owns (from #190 section 14.3, adjusted) | Assertions active |
| --- | --- | --- |
| **CORRAL-B core** | `src/llama_rush.rs`, `enemy_llama_rush_v0.json`, unit tests (including the F10 hit table as pure functions and the CRL-Q43 clamp) | Machine items 1 and 2; the machine's phase sequence; no tracking; one hit per instance; stop on solid and leash; cooldowns; "busy" while a spit runs (the real exclusion is wire work) |
| **CORRAL-EVAL core** | `src/corral_eval/**`, `src/bin/corral_eval.rs`, `src/corral_fixtures.rs`; construction through `CombatState::new(..).with_cf02().with_donut(..)` and a corral-owned thin constructor over `PartyFixtureSim::from_parts` (L1; `from_parts` is `#[doc(hidden)]` and takes `&'static PartyFixture`, so the constructor leaks or statics the fixture and places Donut through `donut_body` and `sync_donut_for_tests()`, N2) | **Asserted:** press edges, no state writes after the exempt tick-0 setup (F11), in-process double-run equality, the M1 positive control. **Report only until CORRAL-C:** the M2 gate and the system failures (today's llamas target Carl only and have no rush) |
| **CORRAL-B wire** | `llama.rs` hook (section 5 code shape: kind-aware readers, the per-llama profile, leash interval, activation state, F10 hit table, CRL-Q43 clamp); `combat_scene.rs` (`with_llama_party_target()`, the **per-llama spawn surface** with the widened load check (R1), the per-llama opt-in stored on `CombatState`, rush contact, events); the four name-replay hooks (the `corral_` arm in `cf02::fixture_runs_cf02`, the `donut_start_x` delegate, the `PartyFixture::all()` entry, and **`corral_fixtures::llama_opt_ins(name)` in `reset_attempt`**, KC15 and R2); the **distinct-source-id test, created here** (none exists on main); the `combat_anim.rs` kind check; the isolated fixtures below | Fixtures below; unchanged goldens |
| **CORRAL-C** | `f01_corral.room.json`, `corral_m1`, `corral_m2` and the start sweep; tuning authority: the rush profile (approach and rush), the corral geometry and activation (#190 review F4), never Donut, the reflex, mana, the AI or other llamas | Section 11 gate, machine item 5 |
| **CORRAL-D** | Partition prop data, the rush-into-prop stimulus if still missing, the contact append in `combat_scene/env.rs`. **Depends on** the unmerged `env_prop.rs` and `combat_scene/env.rs` (BOIL-B and BOIL-C core and wire, #176, N4); the cut path covers a delay | Partition fixtures |
| **CORRAL-E** | Stand-in tell in `combat_anim.rs`, cue reuse in `audio.rs` | Visual evidence |
| **CORRAL-WIRE** | `f01_corral.encounter.json` (rosters with per-llama profiles, wake, checkpoints, the M3 gate), the room-0 manifest row, the corral `SceneKind`; **no zone row in `story_unlocks_v0.json`** (D1) | Route fixture, no grant in room 0 |
| **CORRAL-F** | Evidence | Section 12 evidence |

### Fixtures and tests per step

Isolated fixtures (CORRAL-B wire) run headless on the corridor kit under CF02 rules through the `corral_` prefix, party live, one llama with the corral profile at **E = 600**, facing −1, awake, cooldowns 0, the hero at `E − d` facing +1. Attack tick 0 is the llama's logged `LlamaRushTell`; input ticks are attack ticks. Expected ticks come from section 6.

| Fixture | Setup | Input | Expected |
| --- | --- | --- | --- |
| `corral_rush_hits_idle_donut` | Donut `d` 96 (x 504), Carl far | none | Tell 0, commit 32, contact active 15 (attack 46), llama at 540, Donut −1 HP, knocked 120 / 160, Recover 40, then `rush_cooldown` 150 |
| `corral_rush_hits_idle_carl` | Carl `d` 96, Donut far | none | Contact active 15 (attack 46) |
| `corral_rush_point_blank` | Donut `d` 24 | none | Contact active 1 (attack 32) |
| `corral_rush_trigger_edge` | Donut `d` 128, then 132 | none | 128: tell, contact active 23 (attack 54); 132: no tell, the llama walks to 112 first |
| `corral_rush_jump_clears_donut` | Donut `d` 96 | jump on 4, on 43; and on 3, on 44 | 4 and 43 clear (llama ends at 504, `LlamaRushEnd Miss`); 3 and 44 are hit |
| `corral_rush_jump_clears_carl` | Carl `d` 96 | jump on 12, on 42; and on 11, on 43 | 12 and 42 clear; 11 and 43 are hit |
| `corral_rush_run_away` | Donut `d` 96 | run −x from 44; from 45 | 44 clears; 45 is hit |
| `corral_rush_missile_interrupt` | Donut `d` 96, biscuit, full mana | Primary on 20; on 21 | 20: `LlamaInterrupted`, no commit; 21: the bolt lands during the commit, 1 damage, rush continues |
| `corral_rush_commit_lethal` | llama at 1 HP, Donut `d` 96 | Primary on 21 | `Down` mid-rush, no contact |
| `corral_rush_bonk_wall` | wall face `F` 84 ahead | Donut jumps on 20 | Bonk on active 15, pivot at 56, 60 ticks, `LlamaRushEnd Blocked`, no self-damage |
| `corral_rush_leash_stop` | leash bound 40 px ahead | Donut jumps on 20 | Stops at the bound, `Miss`, Recover 40 |
| `corral_rush_no_tracking` | Donut `d` 64 | jumps on 20 and lands behind | Direction unchanged; the llama never turns during the commit |
| `corral_rush_2p_one_hit` | Carl `d` 72 and Donut `d` 96 in the lane | none | Carl hit only; one `HeroHurt` |
| `corral_rush_invulnerable_pass` | Carl invulnerable at `d` 60, Donut `d` 96 | none | Passes Carl, hits Donut |
| `corral_rush_offscreen_no_tell` | lane end outside the view | none | No tell; the llama walks or holds |
| `corral_rush_no_glob` | any rush | none | No `LlamaSpit` during a rush; `telling()` false from attack 32 |
| `corral_spit_height_rule` | Donut on the floor at `d` 160; Donut on a 32 px block; Carl on the floor | none | Floor Donut: never spit at (walk to 112, rush); Donut on 32: spit; Carl: spit |
| `corral_party_target_solo_donut` | solo, Donut controlled, Carl 40 px nearer | none | Target Donut (bias 40 against switch 24) |
| `corral_m1_llama_never_rushes` | the M1 llama, no profile, Donut at `d` 96 for 600 ticks | none | Zero `LlamaRushTell` |
| `corral_rush_not_evadable` (after DM-B) | reflex armed | none | The rush hits, with no `HeroEvaded` from it; `last_reset_tick` is reset to the contact tick (DM-Q13), so the reflex is disarmed for 720 ticks |
| `corral_rush_does_not_consume_reflex` (after DM-B) | reflex armed; a glob arrives first, then a rush | none | The glob is evaded (one `HeroEvaded`); the rush then hits (`HeroHurt`), with no second `HeroEvaded` |
| `corral_spit_hop_classified_floor` | Donut on the floor at `d` 160, hopping in place (Jump whenever grounded) | Jump | Never spat at; she stays a rush target, and a tell starts only on a grounded tick |
| `corral_kick_at_leash_edge` | a dock llama at its leash minimum, facing +1; Carl on its surface kicks it toward the bound; then a lethal hit there | kick, then lethal jab | `x` stays at the bound and `vx` becomes 0; the death slide is clamped too; the body never leaves its surface |
| `corral_rush_retry_reset` | retry during a commit | restart | Rush state, `rush_seq` and cooldowns restored from the fixture |

**CORRAL-C:** `corral_m1_static_control` (M1-A, M1-B), `corral_m2_static_sweep` (section 11, the gate and the invariant, every gated policy including `stationary_jump_cast`), `corral_m2_b1_counterexample` (M2-13, Donut at 1728, every `o` and `aimed`: A is never hit and never damaged; any bolt in A's band ends on the bumper), `corral_b1_reach_sweep` (machine item 2), `corral_m2_mobile_donut`, `corral_m2_carl_melee`, `corral_m2_coop`, `corral_m2_double_run`, `corral_m2_heights` (the section 10 truth table with real bolts, globs and rushes).

**CORRAL-D:** `corral_partition_rush_break` (B rushes a Donut at 1376: break on the contact tick, Bonk, the lane opens, a bolt then reaches B), `corral_partition_near_miss` (leash stop 4 px short: intact), `corral_partition_carl_breaks` (three distinct hits), `corral_partition_missile_blocked` (eight bolts: intact), `corral_partition_glob_blocked`, `corral_partition_remnant_bolt_passes`, `corral_partition_retry_restores`.

**CORRAL-WIRE:** `corral_route_m0_m3` (both heroes reach x 2304 in solo Carl, solo Donut and 2P), `corral_room0_no_grant` (zero `CapabilityGranted` in room 0; the biscuit pin present on entry), `corral_exit_gate_opens` (closed until the second M2 defeat tick).

## 16. Horizon fit, tone and rights

- **Horizon:** consulted [the progression horizon](character-progression-horizon.md). The corral adds no hero verb, button or unlock; the rush is an enemy action; room 0 grants nothing. It fits the guide's Donut ("survivability through positioning") and Carl ("the closer"). No horizon edit is needed.
- **Tone:** the llama stays a scorched, plain dungeon beast; the rush reads as weight and effort (brace, scrape, chest first, a stumble on a miss), not spectacle. The partition break is the player's spectacle.
- **Rights:** no new art, text, audio or third-party material. LLAMA-03 follows the pixel-art pipeline and its provenance rules.
