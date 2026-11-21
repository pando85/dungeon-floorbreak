# DONUT-PROG-01: Donut's Floor 01 ability progression, reconciled spec

**Status:** design adopted by recommendation under the owner's design-defaults directive (2026-11-21). Not implemented. Every value is a data knob with a question ID (DP-Q01..DP-Q44), not approved balance. Game feel needs the owner's playtest. Delivery is split into small PRs in the [work order](../07-agents/donut-prog-01-workorder.md).

**Designer tier:** top tier (reconciles a detailed owner request with the controller contract, three in-flight designs and existing code; checks the story chronology). Execution: medium tier. Review: an independent top-tier agent per PR.

**Owner request (2026-11-21), summarized:** Donut starts Floor 01 with agile movement, higher and farther jumps, the defensive swat and the pounce (0–20 %). The Tutorial Guild biscuit permanently unlocks Magic Missile and mana (~20 %), keeping the 8-charge pool, its regeneration and the primary mapping. "Light on Your Feet" permanently improves jump reach by about 20 % (~30 %), with no double jump and no new control. Then 30–70 % teaches ranged combat, mana, elevated firing, pounce finishers, environment and coordinated attacks through encounters, not abilities; 70–85 % asks for mastery against mixed enemies. After the Juicer (~85 %) Donut permanently gets Puddle Jumper: Circle/B, a short directional teleport, 112 px, 2 charges, 9 s cooldown, a valid, visible, collision-safe destination, no walls or protected gates crossed, no invulnerability, no damage, data-driven, no new button or equipment UI. The Ball of Swine tests the whole kit with no new ability. Unlocks are narrative, permanent, deterministic, persistent and checkpoint-safe; percentages are pacing targets only. The 12 s reflex evasion is DONUT-MOBILITY-01's and is not changed here. Final target: a fragile agile cat grows into a highly mobile magical fighter through positioning, ranged pressure, survival and tactical mobility, not damage scaling.

**Read with:** [MVP controller contract](../01-design/mvp-controller-contract.md) (authoritative for buttons), [character progression horizon](../01-design/character-progression-horizon.md), [Floor 01 progression bible](../01-design/floor-01-gameplay-progression-v0.md), [Act I beats](../01-design/floor-01-act-01-playable-combat-beats.md), [asymmetric traversal and gates](../01-design/asymmetric-traversal-and-gates.md), [Ball of Swine arena](../01-design/floor-01-ball-of-swine-arena-gameplay.md), [Juicer boss](../02-art/floor-01-boss-design.md), [mana v0](../01-design/donut-mana-v0-proposal.md), [Q10 moveset](../02-art/donut-moveset-q10.md), [combat lab](../01-design/combat-lab-01-advanced-playtest.md), [Donut bible](../02-art/donut-character-bible.md), [antihero tone](../01-design/antihero-tone.md), [ADR-0010](../05-decisions/ADR-0010-combat-first-progression.md).

**In-flight designs this spec conforms to (not on `main` yet, so cited by path and PR, not linked):**

| Work | PR, branch, head read | What DONUT-PROG takes from it |
| --- | --- | --- |
| DONUT-MOBILITY-01 (DM) | #173, `design/donut-mobility-01`, `66f4c84a` (Revision 1), `docs/03-tech/donut-mobility-01.md` | Base agility, always on: run 207 px/s (DM-Q01), launch 500 px/s (DM-Q02), apex 114.92 px on tick 28, 57 air ticks, 196.65 px flat; ledges 96, 104, 112 yes, 116 no; `HeroMotion` applied every tick; DM-Q08 per-hero AI reach; DM-Q26 HUD clearance on her drawn top (feet + 44); DM-Q27 the CF02 arena's 228 px lane ceiling; the 12 s reflex at the biscuit (DM-B). **Not edited, not redesigned** |
| CARL-PROG-01 (CP) | #174, `design/carl-prog-01`, `9a4b8caf`, `docs/03-tech/carl-prog-01-floor01-combat-progression.md` | The generic story-unlock ledger (section 3 there: `src/story_unlock.rs`, `CapabilityId`, `UnlockLedger`, `StoryTrigger`, `StoryPin`, `assets/data/gameplay/story_unlocks_v0.json`, `CapabilityGranted`), its restart rules (CP-Q07), flag style (CP-Q09), notices (CP-Q11), the Circle/B press rules (CP-Q12, CP-Q13), the `SpecialRejected` / `SpecialReady` events (CP-Q31), the equipped socket (CP-Q32) and the chronology anchors (section 2 there). **No competing mechanism is designed here** |
| COMPANION-AI-02 (CA2) | #172, `design/companion-ai-02-donut-autonomy`, `d745562c` (Revision 2), `docs/03-tech/companion-ai-02-donut-autonomy.md` | The goal layer (`src/companion/autonomy.rs`), the drift check, the free zone `Z`, the vertical band (CA2-Q08), Retreat. Section 9 here is a **later CA2 extension** with its own PR; #172's files are not edited |

## 1. Inventory: what exists and is reused

Verified on `origin/main` at `62686919` unless marked as an in-flight design.

| Need | Already exists | Reused as | New in DONUT-PROG-01 |
| --- | --- | --- | --- |
| Agile run and jump from the start | DM-A (in flight): 207 px/s, 500 px/s launch, every scene with a Donut body, human and AI | Unchanged; the 0–20 % band adds **no code** | Nothing |
| Defensive swat and pounce | `src/donut_moves.rs` (Q10 swat 4/3/7, 0 damage, stagger 10, light guard; pounce 10 startup, 14 flight, 1 damage, finisher on lying or staggered targets), wired in `src/combat_scene/donut.rs` | Unchanged | Nothing |
| Biscuit, Magic Missile, mana | `src/donut_mana.rs`: 8 charges, missile cost 1, regen 30-tick delay then 1 per 40 ticks, restarted only on a release tick, `ai_mana_reserve` 2, dry press at 0, `grant_unlock` idempotent, `RestartKind` (retry and checkpoint keep the unlock and refill, fixture restart restores), `UnlockTrigger` guild zone (`GUILD_PLACEHOLDER_ZONE` 520..560 in the story run), `--donut-biscuit`, `--boss` starts unlocked; tuning `companion_magic_v0.json` FNV-pinned; `route_primary` maps Square/X to swat, cast or dry | Unchanged. The biscuit stays the read-through capability `primary_ranged` (CP-Q06, CP-Q10) | Only an appended `spend` for the special cost (section 4.4) |
| Reflex evasion | DM-B (in flight), gated on `DonutMana::unlocked()` | Accounted for in pacing (section 5); never read by DONUT-PROG rules | Nothing |
| Story unlock ledger | CP-A (in flight) | Two new capability rows, their triggers and pins | Data rows only, no new mechanism |
| Circle/B input | `RawInput.ability`, `ability_pressed` (East), inert for Donut in `step_donut`; L1 + Circle is the COMPANION Regroup chord | Same input, press edge only (CP-Q12) | Donut's special step |
| HUD | Mana track (`ManaView`, 8 segments, flash), `SpecialSlot::Locked` padlock on both plates; CP-B2 adds `SpecialSlot::Equipped(Chip)` | The mana track shows the 2-charge spend unchanged; CP-B2's equipped socket | A cooldown underline in the socket and two notices |
| Jump physics | `step_hero_driven` in `src/prototype.rs` (instant velocity, coyote 6, buffer 7, no variable height); DM-A's `HeroMotion { run_speed, jump_speed }` value | Same step; Light on Your Feet only swaps the launch value | A per-tick launch selection |
| AI reach and nav | DM-A: per-hero follower reach (her discrete apex minus 4), `NavConfig::agile()` | Same rules fed with her effective launch | Rebuild on grant |
| Teleport or blink | **None.** `scene_camera::TELEPORT_PX` 160 is a render-camera cut heuristic for Carl's respawn, not a mechanic | Puddle Jumper's 112 px stays under 160, so no camera cut | `src/short_blink.rs` |
| Camera visibility in the sim | `SceneSim::is_offscreen` (sim `ScrollCamera::left()` + `view_w`), D1 M4 | The same sim camera decides "visible" | A view-inset rule |
| 2P separation | `apply_separation_wall` (max `view_w - 192`, 448 px at 640), zeroes `move_x` only | Puddle Jumper checks the same limit, since a warp is not `move_x` | One destination rule |
| Checkpoints | `RestartKind`, DEMO-01 respawn, boss restart; no save file | CP-Q07 lifetimes | Cooldown and windup cleared on restarts |
| AUTOPLAY | `src/autoplay/` (FixtureSim env, party scenarios, episode status, metrics) | Episode completion checks | `dp_*` scenarios |
| PLAYTRACE | `src/playtrace/` (reset descriptor with `donut_biscuit`, journal of combat events, strict verify, content hashes) | Same recorder and verifier | Three descriptor flags, one content hash |
| Combat lab | Profile `floor01-postbiscuit-lab-v1`; `ability.rs` lists `puddle_jumper` as not fabricated | Unchanged (DP-Q19) | Donut flags refused there |

## 2. Chronology check and adopted anchors

**Repo sources.** The chapter anchors recorded in the repo (art atlas and late-enemy notes, checked against the owner-supplied Book 1 EPUB by earlier PRs; collected in CP section 2): Tutorial Guild Ch. 3–7; rats Ch. 8 onward; Bad Llamas, rats and Scatterers Ch. 8–10; the Hoarder Ch. 11; troglodytes and the Juicer's gym Ch. 23–24; Ball of Swine and the floor exit Ch. 26–27. With the exit at Ch. 27: the Guild sits at 11–26 %, Ch. 8–9 at 30–33 %, the Juicer near 85–89 %. The controller contract (Floor 01 teaching table, "Juicer gym ... Donut earns Puddle Jumper *after* Juicer, available as an equipped special") and the horizon (Floor 01 reference path step 6) place Puddle Jumper after the Juicer; both call their sequence "design, not asserted canon timings".

**Verified against repo sources:** biscuit (Ch. 3–7) before the Juicer (Ch. 23–24) before the Ball of Swine (Ch. 26–27). The owner's ~20 % and ~85 % match.

**Unverifiable in repo sources (no claim made):**

- "Light on Your Feet" does not appear anywhere in the repository (docs, code or data). Whether Donut gains anything by that name in Book 1, and when, cannot be checked here. The name is used only in the fan layer (a notice text); the engine capability is IP-neutral. The anchor below is a **game adaptation** of the owner's ~30 %.
- The chapter in which Donut acquires Puddle Jumper, and how, has no chapter citation in the repo; the contract and horizon state the order only. The anchor is the Juicer's defeat event, as for Carl's gauntlet.

No book text is quoted and no book fact is asserted. Because every anchor is a data event id, moving one changes no code.

| ID | Adopted default | Conflict or reason |
| --- | --- | --- |
| DP-Q01 | The biscuit keeps its existing anchor: the guild zone (`UnlockTrigger`, `GUILD_PLACEHOLDER_ZONE` until the Guild room exists), read through as `primary_ranged` | Owner ~20 %, contract and Act I beat I-04 agree. Nothing changes |
| DP-Q02 | Light on Your Feet is granted on the story event `f01_ranged_lane_2_cleared`: the end of Act I beat I-07 (second Bad Llama lane cleared), about Ch. 8–9 by the repo anchors, ~30–33 %. Until I-07 exists, the story corridor run under `--donut-prog` uses a placeholder zone after the guild zone (`LIGHT_FEET_PLACEHOLDER_ZONE`, at least 640 px past the guild zone, chosen by DP-A inside the corridor kit) | Rejected anchors: the Guild itself (merges two unlocks the owner spaced apart); the I-08 refuge (the Act I doc says "No major skill unlock here unless explicitly added after separate review"); a level-up or percentage trigger (owner: no runtime percentages). Unverified as canon (above); the owner can confirm or move it against the EPUB |
| DP-Q03 | Puddle Jumper is granted on `brute_boss_f01_defeated` (`EnemyDefeated { target: BOSS_ENEMY_ID }`), on the defeat tick, the same event as Carl's gauntlet (CP-Q02). Receipts are written in capability-id order | Contract and horizon: "after Juicer". The Juicer is `boss_f01` (CP-Q03) |
| DP-Q04 | The final boss is the Ball of Swine. It pins all three Donut capabilities and holds no trigger (CP-Q03, validated at load) | Owner: no new ability in the final boss |
| DP-Q05 | Percentages are pacing targets; milestones are events (CP-Q04) | Owner rule |
| DP-Q06 | Story order is closed: a pin or a debug flag that holds a later Donut capability also holds every earlier one (`special_short_blink` implies `jump_reach_boost` implies `primary_ranged`); a pin that breaks the order fails to load; a trigger whose predecessor is not held stays armed and does not fire | Owner: "narrative unlocks"; prevents a Puddle Jumper without mana |

## 3. Unlock model: conform to CARL-PROG's ledger

DONUT-PROG adds **rows, not a mechanism**. Everything below is CP-A's design applied to Donut.

- **Capability ids (DP-Q07):** `jump_reach_boost` (Light on Your Feet) and `special_short_blink` (Puddle Jumper), beside CP's `special_thrown_charge`, `fist_reinforced` and the read-through `primary_ranged`. Display names live in data and the fan layer only.
- **Data:** the two rows, their triggers (`Event { id: "f01_ranged_lane_2_cleared" }`, the placeholder `Zone`, `Event { id: "brute_boss_f01_defeated" }`) and Donut's pins go into CP-A's `assets/data/gameplay/story_unlocks_v0.json`; DP-A re-pins that file's FNV hash (a data pin, not a golden). Light on Your Feet and Puddle Jumper knobs live in a new `assets/data/gameplay/donut_prog_v0.json` (knob names start with their question ID, `dpq14_...`), loaded, validated and FNV-pinned by a new `src/donut_prog.rs`, as `carl_prog_v0.json` is.
- **Lifetimes (CP-Q07):** a grant survives a defeat retry, a checkpoint respawn, a solo swap and a 2P join or drop; a fixture restart restores the declared pin. There is no cross-session save; "persistent" means run-persistent, and the ledger's sorted, versioned snapshot carries Donut's ids for the future campaign save.
- **Hash:** nothing while empty (CP-A); Donut's rows add nothing to an unflagged run.
- **Event:** `CombatEvent::CapabilityGranted { tick, hero, capability }` with `hero` = Donut (`DONUT_HERO` 1).

| ID | Adopted default |
| --- | --- |
| DP-Q07 | Capability ids as above |
| DP-Q08 | Grants are to Donut's body, never to a player slot: whoever controls her (P1, P2, solo swap, companion, follower bot) gets the same capability on the same tick |
| DP-Q09 | Pins under `--donut-prog`: the story corridor run holds nothing at tick 0 and has the guild zone and the Light on Your Feet placeholder zone; the opening route, DEMO-01 and the CF02 microarena hold nothing (pre-biscuit, as CP-Q08); `--scene boss_f01` holds `primary_ranged` and `jump_reach_boost` and has the Puddle Jumper event; the final-boss scene, when it exists, holds all three and has no trigger |
| DP-Q10 | Flags, the `--donut-biscuit` style: `--donut-prog` (pins and triggers), `--donut-light-feet` (grants `jump_reach_boost` at start, implies `--donut-biscuit`), `--donut-puddle` (grants `special_short_blink`, implies both). All three are refused with `--scene combat_lab`, recorded in the PLAYTRACE reset descriptor (fields default to false, so older recordings load) and ignored by a scripted fixture, which declares its own pin. A later PR may fold `--carl-prog` and `--donut-prog` into one story flag; not needed now |
| DP-Q11 | Restarts: a defeat retry, a checkpoint respawn and a fixture restart cancel a running blink windup at no cost and make the special ready (cooldown cleared), as CP-Q28; the mana refill follows the existing `RestartKind` rule |
| DP-Q12 | Grant presentation (CP-Q11): a HUD System notice from data, 13 characters at most: `LIGHT FEET` and `PUDDLE JUMPER` (owner tone check pending). At the Puddle Jumper grant, Donut's socket turns from padlock to equipped. No cutscene, no inventory screen, no equipment UI |

**Requirements on CP-A** (all already in CP-A's design; listed so DP-A can check them): idempotent grants, an event-id trigger table that maps `EnemyDefeated { target: BOSS_ENEMY_ID }`, zone triggers once-only and evaluated in the fixed tick, pins validated at load, the snapshot, and nothing hashed while empty. **One addition DP-A brings to the ledger:** the story-order check of DP-Q06 (a pin validator and a trigger guard), with a unit test. **DONUT-PROG implementation waits on CP-A.**

## 4. Light on Your Feet (`jump_reach_boost`)

### 4.1 What "jump reach" means (DP-Q13)

**Reach is vertical: the jump apex grows by 20 %.** The horizontal air distance grows as a consequence (+8.8 %).

- A ledge's height is what character gates are made of ([asymmetric traversal](../01-design/asymmetric-traversal-and-gates.md): "Donut conquers space ... higher and farther"), and Q10 and DM measure the cat jump by its apex.
- +20 % horizontal distance would need either about +45 % apex (launch about 600 px/s, apex 166 px), which puts her drawn top 242 px above the walking line from a 32 px step (the HUD limit is 228) and opens 10-tile ledges, or a separate air speed, which breaks DM-Q05 (air control is the run speed) and the nav model's `HeroPhysics { jump_speed, run_speed }`. Both are rejected.

### 4.2 Values (all discrete, `f32`, the code's step order, 60 Hz)

| ID | Knob | Value | Notes |
| --- | --- | --- | --- |
| DP-Q14 | `dpq14_light_feet_launch_px_s` | **547**; the loader accepts 501..=558 | Absolute launch while the capability is held; it **replaces** DM-Q02's 500 and is not multiplied at runtime. Apex **137.95 px** on tick 31 (DM base 114.92: **+20.0 %**), **62 air ticks** (57), flat running-jump distance **213.90 px** (196.65: +8.8 %) at DM-Q01's 207 px/s |
| DP-Q15 | stacking rule, no knob | Run speed (DM-Q01), gravity, coyote 6, buffer 7, instant velocity, collider 24 x 52, hurtbox Q4 20 x 24 unchanged; swat, pounce, cat hop, knockback and every `BodyDrive` launch unscaled (DM-Q07 extended) | **No double jump, no air jump, no variable height, no new control.** The owner's rule; [characters.md](../01-design/characters.md)'s "optional double jump" proposal is not adopted for Floor 01 (flagged, not edited) |
| DP-Q16 | derived | AI reach = effective apex − DM-Q08 margin 4 = **133.95 px** | The follower bot and nav use the launch she has now |
| DP-Q17 | wiring | The launch is chosen **every tick** from the ledger, never stored on the body | Same pattern as DM's F3: `combat_scene` passes "boost held" to the party step context; `step_bodies_driven` builds Donut's `HeroMotion` with `jump_speed = dpq14` when held, DM-Q02 otherwise. No `Hero::at` site, restart or swap can miss it |
| DP-Q18 | gate classes, level-authoring rule | Carl: up to 80 px (5 tiles). Donut base: 96 and 104 px; 112 px is a narrow human route (2.9 px margin), the AI does not attempt it. **Donut with Light on Your Feet: 128 px (8 tiles), margin 9.95 px, AI attempts it.** 144 px and above: nobody | The first Light on Your Feet gate (I-07 or later) is a 128 px ledge with a shared payoff ([asymmetric traversal](../01-design/asymmetric-traversal-and-gates.md) G1 pattern). Level metadata declares `jump_reach_boost` as the gate capability |
| DP-Q19 | combat lab | Unchanged: `floor01-postbiscuit-lab-v1` keeps the boost off; Donut flags are refused there | From a 64 px lab slab her drawn top would be 64 + 137.95 + 44 = 245.95 px, over the 228 limit. A `floor01-postlightfeet-lab-v1` profile needs a DM-Q27-style lane ceiling (later work) |

**Largest safe value (owner asked for it).** The binding limits where the boost can be held without a ceiling are, in order: the gate class (stay under a 144 px ledge, launch at most **558**, apex 143.63, +25.0 %), the auto camera (collider top + 56 px pad within 288 px from a 32 px surface, launch at most 566), the HUD band from a 32 px surface (at most about 573). **Adopted: 547 (the owner's +20 %), with 558 as the documented ceiling of the valid range.**

### 4.3 Geometry and gate check (every space where she can hold the boost)

Heights above the walking line. Drawn top = feet + 44 (DM-Q26). Limits: HUD 228 at 640 x 360 (248 at 640 x 400); auto camera collider top + 56 within 288.

| Space | Check with the boost | Result |
| --- | --- | --- |
| Floor 01 corridor and story run (steps of at most 32 px) | From 32 px: drawn top 213.95 (HUD margin 14.05), collider top 221.95 | Pass. No height gate exists there, so nothing is bypassed |
| DEMO-01 hall (platforms 32 px, auto camera under `--demo`) | 32 + 137.95 + 52 + 56 = 277.95 ≤ 288 | Pass, margin 10.05; the camera does not zoom |
| CF02 microarena (DM-Q27 ceiling: collider top ≤ 228) | Floor: collider top 189.95, no bonk. Parapet (48): bonks at feet 176 (new, invisible, inside the HUD band). Perch (80): bonks at feet 176 as with DM-A | Pass. No new standable surface (perches 80, parapet 48). **Perch to perch stays impossible:** with the bonk her air time from a perch falls from 43 to 40 ticks, about 138 px of travel against the roughly 200 px gap (DP-B re-runs DM's full start-column and jump-tick simulation) |
| Boss room `boss01_arena` (ceiling 224) | Floor: collider top 189.95, no bonk; from the 64 px rack strut she bonks; the right rack ledge (104) stays off (DM-RACK) | Pass. Charge jump-over: feet above 72 px for 43 ticks (DM base 35, Carl 22) |
| Engineer ram (14..33 px) and goblin swing (0..30 px) | Feet above 30 px from jump tick 4, as DM-A | `DODGE_LATEST_TELL_TICK` 20 stays valid. She stays above the ram box for 54 ticks (47) |
| Llama glob (42..48 px) | More airtime in the spit band when she jumps | A fair cost; grounded she still passes under it (Q44) |
| Combat lab | Not held (DP-Q19) | Excluded |
| 2P separation wall and scroll camera | The boost adds no horizontal speed; motion past the wall only comes from `move_x` | Pass; DM-Q09 unchanged |
| Ball of Swine (not built) | Elevated flanks must sit at 128 px or lower for her and give Carl a slower route | Recorded for its greybox (section 6) |

## 5. Puddle Jumper (`special_short_blink`)

### 5.1 Values

All knobs in `donut_prog_v0.json`. Ticks at 60 Hz.

| ID | Rule | Value |
| --- | --- | --- |
| DP-Q20 | Input: the **press edge** of Donut's own forwarded Circle/B `ability` action. No hold, no aim, no cycling (R2 stays reserved), no new binding. The COMPANION `L1 + Circle` Regroup chord never starts a blink (CP-Q12). Each hero resolves Circle/B to the special **that hero** holds: Carl's thrown charge, Donut's blink; one special per hero on Floor 01, so nothing to cycle | press only |
| DP-Q21 | Direction, horizontal only: `move_x >= +0.5` right, `<= -0.5` left, otherwise her facing. The stick's vertical component is ignored. No second stick, no aim cursor, D-pad and keyboard work as they move | deadzone 0.5 |
| DP-Q22 | Range: candidates from the maximum down to the minimum in fixed steps; the **farthest valid** one wins | max **112**, min 16, step 4 (25 candidates) |
| DP-Q23 | Cost: **2 charges**, checked at the press and paid on the arrival tick through `DonutMana::spend` (appended). Spending restarts the regen timer exactly as a missile release does (30-tick delay, then 1 per 40), so mana keeps one rule | 2 |
| DP-Q24 | Cooldown: **540 ticks (9 s)**, counted from the arrival tick, stored as an absolute `ready_tick` | 540 |
| DP-Q25 | Timeline: windup t0..t5 (she gathers; horizontal input ignored, `vx` 0; gravity still acts in the air), **arrival on t6** (the warp), recovery t7..t12 (walk and jump allowed; primary, physical and special presses ignored, not buffered) | 6 / 6 |
| DP-Q26 | Start conditions: capability held; not hurt, not defeated, not in hit-stop; no swat, pounce or cast running; cooldown over; mana ≥ cost; a valid destination exists **at the press**. Grounded or airborne. Otherwise `SpecialRejected { tick, hero, reason }` with `Locked`, `Busy`, `Cooldown`, `NoMana` or `NoDestination`, nothing spent, nothing buffered | — |
| DP-Q27 | Destination validity (section 5.2) | — |
| DP-Q28 | On the arrival tick the search runs again from her current position in the locked direction. No valid candidate: a **fizzle** (`BlinkFizzled`), no mana, no cooldown | — |
| DP-Q29 | A hurt during the windup cancels the blink at no cost. **No invulnerability** on any tick (the hurtbox is live through windup, arrival and recovery), **no damage** and no push to anything she passes or lands near. DM-B's reflex is untouched: a blink is not a hurt and does not reset its wait | — |
| DP-Q30 | Arrival: `x` set to the destination, `y` and `vy` unchanged (no height gained, no hang), facing = the blink direction; the normal step of that tick then runs with `vx` 0. Implementation: an appended `BodyDrive::warp_x: Option<f32>` applied before the horizontal step | — |
| DP-Q31 | Ownership: windup, cooldown and cost belong to Donut's body, not to a player slot (as mana does). A solo swap mid-windup cancels nothing; the new controller inherits the cooldown. 2P: P2's Circle/B. Same rules for human and AI | — |
| DP-Q32 | Events (appended, emitted only while the capability is held): CP's `SpecialStarted { tick, hero, capability }`, `SpecialRejected { tick, hero, reason }`, `SpecialReady { tick, hero }`, plus `BlinkResolved { tick, hero, from_x, to_x }` and `BlinkFizzled { tick, hero }`; the existing `Mana(ManaSpent)` on arrival | — |

### 5.2 Destination search (DP-Q27)

A candidate `x' = x + dir * d` is valid only if **all** hold. The body box is her 24 x 52 collider at `(x', y)`.

1. **World:** `x'` inside `[HERO_HALF_WIDTH, max_x]`.
2. **Collision-safe:** the body box at `x'` overlaps no intact solid.
3. **No wall crossed:** the swept box (the union of the start and end boxes) overlaps no intact solid. A wall between her and `x'` therefore rejects every candidate beyond it, and the search settles in front of the wall.
4. **No protected gate crossed:** the swept box overlaps no **blink barrier**. Barriers are: (a) every intact solid, including a closed boss gate collider; (b) rectangles listed in the room or scenario data under a new optional field `blink_barriers` (absent = none; the generic tag for a protected gap or a HANDOFF gate whose intended solution is not a blink); (c) every story or progression **zone** trigger not yet fired, which the blink may end inside but may not jump across (a 112 px blink would otherwise skip a 40 px zone). Position thresholds (`x >= line` wave wakes, boss engage lines) fire normally on arrival and need no barrier.
5. **Visible:** the body box lies inside the **simulation** camera's view (`ScrollCamera::left()` to `left + view_w`, the same camera as `is_offscreen`) inset by `dpq27_view_inset_px` **24** on each side. Never the render camera (the Ball of Swine's zoom is presentation and must not change outcomes). Vertical visibility holds by construction: the blink never changes `y`.
6. **Shared camera:** in 2P, `|x' - carl.x|` is at most the separation limit (`max_separation_px(view_w)`, 448 at 640), or no larger than the current separation.
7. **Support:** a downward probe from her feet at `x'` meets the top of an intact solid within `dpq27_support_probe_px` **256**. No blink over a pit or off the world. Airborne blinks keep falling normally after arrival.
8. **Hazards:** the body box overlaps no hero-hurting hazard rectangle the scene declares (the same set CA2 feeds to the nav graph).

The search is a pure function of the tick's state: fixed candidate order, `f32` in the code's order, no RNG, no hash map. Ties cannot happen (one direction, a strict order).

**Gate tagging.** Carl's breakable obstructions are solids (rule 3). Donut's height gates cannot be bypassed: the blink adds no height. Authored gaps that must keep their intended solution get a `blink_barriers` rectangle; the level-data validator of the asymmetric-traversal doc (future) treats `special_short_blink` as a capability that can cross any unbarred gap of up to 112 px plus her jump distance.

### 5.3 Mana, cooldown and the HUD

- Two charges regenerate in 30 + 80 = 110 ticks, well inside the 540-tick cooldown, so after a quiet stretch the cooldown is the limit; after heavy casting, mana is. That is the intended tension: every missile spent below 2 charges costs her the escape. The companion's `ai_mana_reserve` 2 already equals the blink cost, so an AI Donut that only casts down to its reserve always keeps one blink.
- **Socket (DP-Q33):** CP-B2's `SpecialSlot::Equipped(Chip)` with the Circle glyph and a data label (`BLINK`, 5 characters). Muted while cooling down or below 2 charges. **Cooldown cue:** a 1 px underline in the socket whose width is `ceil(remaining / 540 * socket_w)`, drawn with the existing HUD palette, and the existing `MANA_FLASH_TICKS` flash on `SpecialReady`. On a `NoMana` or `NoDestination` rejection the socket flashes once as the dry press does. Presentation reads state only.
- The mana track shows the 2-segment drop through the existing `ManaSpent` path; no cost preview.

### 5.4 Presentation, art routing and tone

- **Stand-ins in v1 (DP-Q34),** read-only from events: on `SpecialStarted` a 6-tick crouch (the existing pounce crouch frame) and a 3-ring ripple at her feet; on `BlinkResolved` the same ripple at the destination plus a 1-frame dotted streak between the two points for 2P readability, both from the existing Magic Missile palette; on `BlinkFizzled` a single shrinking ring. No aura, no glow on her body, no screen shake. Audio: reuse the missile cast cue at a lower volume, or none.
- **Production art** is a later art order through the [pixel-art pipeline](../07-agents/pixel-art-pipeline.md#routing-by-difficulty): `blink_out` and `blink_in` keys plus the ripple VFX. Top-tier keys and spec, medium-tier execution, independent top-tier review. The [Donut bible](../02-art/donut-character-bible.md) keeps controlled magic effects for later floors, so the art stays small and cat-first: a self-satisfied hop into nothing and a pop out, never a superhero teleport pose.
- **Tone ([antihero tone](../01-design/antihero-tone.md)):** she uses it to get out of trouble and look good doing it, not to duel. Smug on a clean escape is an acting choice (the existing `smug` overlay may play after a blink that dodged a tell), never a hero pose.

## 6. Pacing per band

Existing content teaches through encounters; new rooms are **later work**, not DONUT-PROG code. The 30–70 % band adds no ability: complexity grows.

| Band (pacing target) | Story anchor (repo chapters) | Donut learns | Existing content that teaches it now | New content needed (later work) |
| --- | --- | --- | --- | --- |
| **D1** 0–20 % | Entrance, Dozer, first goblins (Ch. 1–3) | Interrupt with the swat, position with the agile run, take elevated routes, finish weakened enemies with the pounce | Opening route S1–S6 with CP-01/02/03; CF02 microarena (perch, parapet, temperaments, swat interrupts, pounce finishers); Engineer `eng_point_interrupt_swat`; DM's 96/104 px ledges; DEMO-01 hall | None for this band |
| **M1** ~20 % | Tutorial Guild biscuit (Ch. 3–7) | Magic Missile on Square/X; the mana pool; the reflex evasion arrives with it (DM-B) | Story corridor guild zone; `q10_unlock_persists`; `dm_evasion_unlocks_at_guild` | The Guild room (I-04) |
| **D2** 20–30 % | I-05, I-06 (Ch. 7–8) | First ranged kills, cast while moving, firing from a modest height, mana running out (the dry press) | CF02 with `--donut-biscuit`; Bad Llama (DEMO B4, CF02-C1) | I-05 balcony passage; I-06 lane |
| **M2** ~30 % | I-07 cleared (Ch. 8–9) | Light on Your Feet: 128 px ledges, longer air, new firing perches | `--donut-light-feet` in the CF02 arena (no 128 px surface there: feel only) | I-07 with a 128 px perch as the first boost gate (DP-Q18) |
| **D3** 30–70 % | Rats, Scatterers, Hoarder, workshop, slimes and Rot Stickers, labyrinth (Ch. 8–22) | Ranged pressure under a mana budget; elevated firing positions; pounce finishers on enemies Carl downs; environmental kills (shore lure, charges Carl sets); coordinated attacks (she takes what is behind Carl, CF02-DUO A11..A14); Carl's dynamite from ~40 % (CP) creates launches she finishes | CF02 arena waves with `--donut-biscuit --donut-light-feet`; Engineer variant; CP-03 shore; CF02-DUO fixtures | Rat and Scatterer enemies (art exists: RAT-01, SCAT-01; gameplay specs needed); workshop rooms with 128 px perches and charges; labyrinth rooms with Donut-only boost routes and HANDOFF gates |
| **D4** 70–85 % | Troglodytes, Juicer approach (Ch. 23) | Mastery under mixed pressure: melee goblins, a ranged llama and a charger at once, at least two overlapping threats so the 12 s reflex cannot carry an encounter; survival with 4 HP | CF02 arena with the Engineer and llama together; `--scene boss_f01` with the boost pinned | Troglodyte encounter spec; gym approach rooms |
| **M3** ~85 % | Juicer defeated (Ch. 23–24) | Puddle Jumper granted (DP-Q03); the socket lights up | `--scene boss_f01 --donut-prog` | A short post-fight beat with a safe first blink (a gap or a cornered spot) |
| **D5** 85–100 % | Regroup, Ball of Swine, exit (Ch. 26–27) | Escape a committed charge, reposition to a flank, reach a platform she can already reach faster; then the whole kit | CF02 arena with `--donut-puddle` (playtest only) | A short blink teaching room; the Ball of Swine arena (section 7) |

**The reflex (DM-B) in encounter design:** one free evade per 12 s hurt-free. Late encounters assume it exists and do not budget around it: a room that is only dangerous because of one tell per 12 s is too easy. The blink and the reflex stack as two different answers (one chosen, one passive); no encounter may require both at once.

## 7. Final boss: the Ball of Swine tests the whole kit

No ability is added (DP-Q04); the scene pins all three and holds no trigger. When its arena is designed it must give each Donut tool a moment where it is clearly the best answer, and gate damage on none of them:

1. **Agile run and boost jump:** reach a 128 px flank in time to fire across the ball's committed line; Carl's slower route reaches the same flank (no untouchable perch).
2. **Swat:** interrupt a detached Tuskling's tell aimed at Carl.
3. **Pounce:** finish a stunned or downed Tuskling.
4. **Magic Missile and mana:** keep ranged pressure while the ball recovers, with enough charges kept for a blink.
5. **Puddle Jumper:** escape a committed charge pinned against a wall, or cross the lane to the far flank inside the view.
6. **Survival:** the reflex and positioning, not health scaling.

It stays winnable in solo with an AI partner and in 2P; every outcome is deterministic; the blink's visibility uses the simulation camera even if the presentation camera zooms (5.2 rule 5).

## 8. Gating and goldens

- **No flag, no change.** Every new rule runs only while a Donut capability is held, and Donut capabilities come only from `--donut-prog` pins and triggers, `--donut-light-feet`, `--donut-puddle` or a `donut_prog_*` fixture pin. New events, `BodyDrive::warp_x`, `DonutMana::spend` and the blink state are appended and mix nothing into the state hash while unused.
- **Byte-identical, every PR:** the 20 `EXPECTED_HASHES`, `GOLDEN_WALK_RIGHT_400`, the story hash, the party goldens, the 13 boss fixtures, the `q10_*` goldens, `demo01_*`, the CINE goldens, the whole-route autopilot hash, every `cf02_*`, `companion_*` and Engineer fixture, the ART-INT-02 anim traces, the PLAYTRACE goldens, **and** every `dm_*` (DM) and `carl_prog_*` (CP) fixture that exists on `main` when the PR is cut. The values are those on `main` after DM-A's intended re-pin; DONUT-PROG records nothing before DM-A.
- **Goldens that move on purpose: none.** New behaviour gets new `donut_prog_*` fixtures with committed final hashes, replayed twice, headless and in the render loop.
- **`--donut-prog` alone** on the opening route, DEMO-01 and the CF02 arena reproduces the unflagged hashes (nothing held, no trigger reachable). On the story corridor run it changes the run only from the placeholder zone's tick on.
- **Solo and 2P parity:** the same fixture with Donut driven by P1, by P2 and by the companion, at identical body positions and inputs, produces the same grants, launches, searches, spends and arrivals (DP-Q08, DP-Q31). Only the separation rule (5.2 rule 6) differs in 2P, by design.

## 9. CA2 extension: the AI Donut and her new abilities

A short, separate design for a later PR (DP-E) that lands **after** CA2's code PRs (CA2-A, CA2-B and the wiring) and DP-B, DP-C. It does not edit #172's spec; it extends CA2's role-neutral layer with data and one action. Human rules are unchanged: the AI uses the same press path, the same search and the same costs.

| ID | Rule | Value |
| --- | --- | --- |
| DP-Q37 | **Magic Missile:** unchanged. The director's existing rules (cast band 64 to 160 px, CA2-Q20 spacing 112, mana reserve 2) apply | — |
| DP-Q38 | **Light on Your Feet:** no new AI logic. The nav graph and follower reach use her effective launch (DP-Q16, DP-Q17); the graph is rebuilt on the grant tick (deterministic, in anchor-id order). CA2's Perch may choose a 128 px anchor if it is inside `Z` and the vertical band (CA2-Q08 max rise 176 holds); the drift check reads the edge's simulated duration, which is longer with the boost | — |
| DP-Q39 | **Puddle Jumper, AI use: escape only.** Eligible when she is companion-driven under a set with autonomy enabled, holds the capability, the blink is ready and paid for, and either (a) the Retreat goal is active, or (b) a committed attack (goblin swing tell, Engineer ram tell, a boss charge under CA2-Q35's Retreat) is aimed at her within `dpq39_ai_threat_px` 48 and its impact is due within the windup plus 6 ticks, and the follow layer's jump dodge would not clear it. Direction: away from that threat. The predicted destination (the same pure search, read-only) must gain at least `dpq39_ai_min_gain_px` 64 px of distance from it, lie inside `Z` and pass the drift check with a 12-tick commitment | 48 / 64 |
| DP-Q40 | Never: for offence, as a traversal edge (the nav graph has no blink edges in v1), to chase, in 2P, for the follower bot of non-autonomy sets, or while a human controls her. The AI may spend the mana reserve on a blink (the reserve exists for it) | — |
| DP-Q41 | Data: `autonomy.roles.skirmisher.blink_enabled` (true in `cf02_free`, absent = false elsewhere), `blink_threat_px`, `blink_min_gain_px`. The decision is part of the hashed autonomy state; no new goal kind, the blink is an action of Retreat and of the dodge layer | — |

**CA2 metrics added:** blinks per fixture, blinks that avoided the triggering hit, fizzles, frames outside the view after a blink (must be 0: the destination is inside the view by rule 5 and inside `Z`). Fixtures `cf02_free_blink_escape` (a cornered Donut blinks out of a swing), `cf02_free_blink_not_offense` (a reachable target never triggers a blink), `cf02_free_blink_reserve` (she casts down to 2 and still blinks), each solo, replayed twice.

## 10. Acceptance per milestone

Fixture names are `donut_prog_*` (`src/donut_prog_fixtures.rs`). Each asserts exact ticks and a committed final hash and replays twice.

**M0 rows and flags (DP-A)**
- Unit: the two rows load and are FNV-pinned; the story-order validator rejects a pin holding `special_short_blink` without `jump_reach_boost`; a trigger with a missing predecessor stays armed; flags imply their predecessors; flags are refused with `--scene combat_lab`; an older recording without the new descriptor fields loads as false.
- `donut_prog_flag_parity`: `--donut-prog` on the opening route, DEMO-01 and the CF02 arena reproduces the unflagged hashes.
- `donut_prog_story_unlocks`: the story corridor run under `--donut-prog`: the guild zone grants `primary_ranged` (existing tick), the placeholder zone grants `jump_reach_boost` exactly once, a defeat retry and a checkpoint respawn keep both, a fixture restart restores the empty pin.

**M1 biscuit (no new code)**
- Existing `q10_*`, `dm_evasion_unlocks_at_guild` and mana unit tests stay byte-identical; `donut_prog_primary_mapping`: before the grant Square/X swats, after it casts, at 0 mana it is dry; Circle/B does nothing before `special_short_blink` (no event, same hash as without the flag).

**M2 Light on Your Feet (DP-B)**
- Unit, discrete step: apex 137.95 ±0.05 on tick 31, 62 air ticks, flat 213.90 ±0.05 at 207 px/s; the loader rejects 500 and 559.
- `donut_prog_ledge_128` (lands only with the boost), `donut_prog_ledge_144_fails`, `donut_prog_ledge_112_margin` (lands with a wider takeoff window than DM's), `donut_prog_no_air_jump` (a second Cross press in the air does nothing).
- `donut_prog_boost_every_controller`: after the grant the boost applies under P1, P2, a solo swap, the companion and the follower bot, after a retry and after a respawn.
- HUD and camera test with the real walk line: from every standable surface of the corridor kit, DEMO-01, the microarena (with DM-Q27) and the boss room her drawn top stays within 228 (640 x 360) and 248 (640 x 400) and the auto camera does not zoom.
- Microarena perch-to-perch simulation over every start column and jump tick: no landing on the other perch.
- Nav diff with the boost: Carl's edges identical; new Donut edges listed in the PR body; `every_open_jump_and_drop_edge_replays_through_step_hero` passes with the boosted launch; the follower bot attempts 128 and not 144.

**M3 Puddle Jumper (DP-C)**
- `donut_prog_juicer_grants_blink`: under `--scene boss_f01 --donut-prog`, the boss defeat emits exactly one `CapabilityGranted { special_short_blink }` on the defeat tick, after `fist_reinforced` when CP is on.
- `donut_prog_blink_flat` (112 px, arrival on t6, mana 8 → 6, regen restarts, ready again on arrival + 540 and not on + 539), `_facing_no_stick`, `_stick_left`, `_wall_stops_short`, `_thin_wall_not_crossed` (a 16 px wall at 40 px: she lands in front of it, never behind), `_barrier_rect`, `_zone_not_skipped`, `_view_edge` (shortened to stay 24 px inside the view), `_2p_separation`, `_pit_rejected`, `_no_destination_refund` (`NoDestination`, no spend), `_fizzle_on_arrival`, `_hurt_in_windup_cancels`, `_no_iframes` (a swing landing on t7 hurts her), `_no_damage`, `_airborne_keeps_vy`, `_no_mana` (1 charge: rejected), `_cooldown_reject`, `_swap_keeps_cooldown`, `_retry_clears`, `_regroup_chord` (L1 + Circle never blinks), `_carl_circle_unchanged` (Carl's Circle/B under `--carl-dynamite` still throws in the same run).
- Determinism: each fixture headless and render loop, twice, identical; the search is fuzzed over 1,000 seeded positions in the microarena and corridor with a property test: no destination overlaps a solid, crosses a solid or barrier, leaves the inset view or lacks support.

**M4 presentation (DP-D):** HUD unit tests for locked, equipped, cooling (underline widths at 0, 270 and 539 ticks left) and ready; notices; no hash change.

**M5 CA2 extension (DP-E):** the three `cf02_free_blink_*` fixtures and the metrics of section 9; every existing CA2 fixture byte-identical with `blink_enabled` absent.

**AUTOPLAY encounter completion (DP-F):** `dp_arena_boost` (the CF02 arena run with `--donut-biscuit --donut-light-feet`, solo pilot and companion), `dp_arena_blink` (with `--donut-puddle`) and `dp_boss_boost` (`--scene boss_f01 --donut-prog`, solo with an AI partner) each reach episode success inside the existing step budgets at seeds 0..8; reported with deaths, restarts, blinks, mana spent and dry presses per episode. No new softlock (no hero stuck more than 300 ticks).

**PLAYTRACE:** a recorded `--donut-prog` story run shows `Mana(UnlockGranted)` and `CapabilityGranted { jump_reach_boost }` in the journal and verifies strict on its own build; a recorded `--cf02 --donut-puddle` session shows `SpecialStarted`, `BlinkResolved`, `ManaSpent` and `SpecialReady` and verifies strict. The manifest carries the three descriptor flags and `content_hashes["donut_prog_tuning"]`.

**GIF evidence** (real Bevy, 640 x 360 and 640 x 400, 1x and 3x): the Light on Your Feet grant notice; side by side, Donut without and with the boost at a 128 px ledge; a running jump showing the longer arc; the blink: flat, stopped by a wall, shortened at the view edge, a fizzle; the socket's cooldown underline over 9 s; a cornered escape from a swing; the 2P separation limit; one solo and one 2P Juicer recording ending in the Puddle Jumper grant; a real two-pad check or **NOT RUN**.

## 11. Horizon fit, tone and owner questions

**Horizon fit.** Consulted [the progression horizon](../01-design/character-progression-horizon.md). It still fits: "fragile agile cat -> biscuit / Magic Missile -> advanced mobility only when earned", "Puddle Jumper ... after its narrative event and requires a separate mechanical adaptation decision" (this spec is that decision), no new button (Circle/B was reserved), no inventory (ADR-0010), properties and choices instead of damage. Light on Your Feet is a **new item on the Floor 01 path** (between steps 2 and 3), so the guide needs a change-record row; it is not edited here. Paste-ready row for the orchestrator:

```text
| 2026-11-21 | DONUT-PROG-01: Donut's Floor 01 path gains a story-earned passive between the biscuit and the Hoarder, a ~20 % higher jump (Light on Your Feet, 128 px ledges, no double jump, no new control), and Puddle Jumper after the Juicer becomes a short horizontal blink on Circle/B (112 px, 2 charges, 9 s, no invulnerability, no damage, never crosses walls or protected gates). No damage scaling | Adopted defaults DP-Q01..DP-Q44, flag-gated; Light on Your Feet's canon timing unverified; game feel pending |
```

**Tone.** Donut stays a fragile show cat who wins by being somewhere else: higher, farther, gone. Nothing makes her tanky or adds raw damage.

**Owner-level question (only this):** "Light on Your Feet" is not in any repo source. Does Donut gain it in Book 1, and at which event? The adopted anchor (DP-Q02, the end of the second llama lane, ~30 %) is a game adaptation that moves without code. Game feel (DP-Q14, DP-Q22..Q25) is reviewed after merge as usual.

**Other adopted defaults:** DP-Q35 the blink barrier tag (section 5.2 rule 4), DP-Q36 the simulation camera for visibility (rule 5), DP-Q42 the reflex budgeting rule (section 6), DP-Q43 the Ball of Swine flank height ceiling 128 px (section 4.3), DP-Q44 Circle/B resolves per hero with one special each on Floor 01 (DP-Q20).

## 12. Paste-ready block for the Floor 01 progression bible

The orchestrator pastes this after CARL-PROG-01's section in `docs/01-design/floor-01-gameplay-progression-v0.md` once #174 has merged, and replaces "Donut's later unlocks remain open" in that file's section 5 with "Donut's unlock order has adopted defaults (see Donut's ability progression)". Links are written to resolve from `docs/01-design/`.

```markdown
### Donut's ability progression (DONUT-PROG-01)

Owner request of 2026-11-21, reconciled in the [DONUT-PROG-01 spec](../03-tech/donut-prog-01-floor01-ability-progression.md) (adopted defaults DP-Q01..DP-Q44) and split into PRs by the [work order](../07-agents/donut-prog-01-workorder.md). Milestones are **story events**; percentages are pacing targets from the repo chapter anchors and no code reads them. Unlocks use CARL-PROG-01's story-unlock ledger. "Light on Your Feet" is not in any repo source; its anchor is a game adaptation until the owner confirms it. DONUT-MOBILITY-01 owns her base agility (run 207 px/s, apex 114.92 px) and the 12 s reflex at the biscuit.

| Milestone | Pacing target | Story event | Donut's kit change | Taught through (existing / later) | Acceptance (spec section 10) |
| --- | --- | --- | --- | --- | --- |
| D1 base kit | 0–20 % | Entrance to the Guild (Ch. 1–3) | None: agile run and jump, swat, pounce from the start; Circle/B inert | CP-01/02/03 route, CF02 microarena, DM ledges / none | Existing `q10_*`, `cf02_*`, `dm_*` fixtures byte-identical |
| M1 biscuit | ~20 % | Guild zone (Ch. 3–7) | Magic Missile on Square/X, 8 charges (existing); DM's reflex arrives with it | Story corridor guild zone / Guild room | `donut_prog_primary_mapping`, `q10_unlock_persists` |
| M2 Light on Your Feet | ~30 % | Second llama lane cleared, `f01_ranged_lane_2_cleared` (Ch. 8–9, adaptation) | Jump apex +20 % (137.95 px), 128 px ledges; no double jump, no new control | Placeholder zone in the story run / I-07 128 px perch | `donut_prog_ledge_128`, `donut_prog_story_unlocks` |
| D3 ranged play | 30–70 % | Rats to labyrinth (Ch. 8–22) | None: encounters grow (mana budget, elevated firing, finishers, environment, coordination) | CF02 arena and Engineer variants / workshop, Scatterer, labyrinth rooms | Existing fixtures; AUTOPLAY `dp_arena_boost` |
| D4 mastery | 70–85 % | Troglodytes, gym approach (Ch. 23) | None: mixed threats, at least two overlapping | CF02 arena with Engineer and llama, boss scene / troglodyte rooms | AUTOPLAY `dp_boss_boost` |
| M3 Puddle Jumper | ~85 % | Juicer defeated, `brute_boss_f01_defeated` | Circle/B: horizontal blink, 112 px, 2 charges, 9 s cooldown, valid, visible, collision-safe, no walls or protected gates, no invulnerability, no damage | `--scene boss_f01` / post-fight beat | `donut_prog_juicer_grants_blink`, `donut_prog_blink_*` |
| D5 final mastery | 85–100 % | Ball of Swine, exit (Ch. 26–27) | **No unlock** | Ball of Swine arena (later) | Load-time check: no trigger; clears with no `CapabilityGranted` |

Light on Your Feet gate classes: Carl up to 80 px, Donut 96 to 104 px (112 narrow, human only), Donut with the boost 128 px, nobody at 144 px.
```
