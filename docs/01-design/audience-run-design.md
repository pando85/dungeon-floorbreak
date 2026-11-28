# AUDIENCE-RUN: viewers and showcase slow motion in the Floor 01 run

> **Status: design adopted under the owner's design-defaults directive (AGENTS.md, 2026-11-19), owner decision 2026-11-25; implemented in the AUDIENCE-RUN PR (see [section 12](#12-implementation-status)), not game-feel accepted.** After the implementation PR merges, the backlog status is "merged, game-feel review pending". The owner judges the feel in the playtest; any adopted value below can change in a follow-up PR.
> Design: Claude Opus (top tier). Execution: medium tier, one implementation PR ([section 9](#9-implementation-plan-one-pr)). Review: a separate top-tier agent.

**Owner decision (2026-11-25):** "go with all owner-gated items using adopted defaults; everything must end on main and be reachable in `--scene f01_late --f01-run` for next week's playtest." The owner asked whether the audience system with per-death-kind animations exists, and whether the best kills get slow motion. AUDIENCE-RUN answers with three changes:

- **(a)** the per-hero viewer counts of [AUDIENCE-01](../02-art/individual-viewers-hud-v1.md) go live inside the Floor 01 run;
- **(b)** [CINE-01](../03-tech/cinematic-finishers-cine-01.md) slow motion also rewards **showcase kills**: a multi-kill with a thrown dynamite stick, and a llama kill;
- **(c)** the per-death-kind presentation (DEATH-D finishes, the llama's LLAMA-DEATH-WIRE set) is visible and captured in the run.

Every rule below uses only facts the authoritative event stream already carries, or a narrow adapter bridge of facts the simulation already owns. Nothing here changes `COMBAT_STATE`, `COMBAT_EVENT` lines, replay hashes or any committed golden.

## 1. What exists on main (verified on 81687b47)

| Piece | State on main | Where |
| --- | --- | --- |
| Audience domain | `AudienceState`: locked until `unlock()`, both heroes at 250 on unlock, per-target dedupe, chain payment per `(hero, target)`, decay, spike/fall cues, `Restart` restores the attempt snapshot, `Respawn` keeps counts | `src/audience.rs` |
| Audience HUD | `HudRuntime::unlock_audience`, plates show one count per hero when unlocked | `src/hud.rs` |
| Unlock in game | **None** (spec F15): only `--hud-preview viewers*` and the QA probe `--audience-probe` | `src/bin/floor01_scene.rs` |
| CINE evaluator | Primaries `CINE-PRECISION-EDGE` and `CINE-MIXED-CHAIN` (5 each, no stacking); bonuses `CINE-CONTACT-SETUP` (+2) and `CINE-EXTRA-KILL` (+1, max +2); 90-tick window, 90-tick clean window, 900-tick cooldown, 8-tick upgrade | `src/cine.rs`, `src/cine_adapter.rs` |
| Explosions in CINE | `DefeatCause::Explosion` and `Environment` map to `CineCause::Other`: never credited, never a feat (T02, BOILBC-AD4) | `cine_cause` in `src/cine.rs` |
| Boss in CINE | `BOSS_ENEMY_ID` always maps to `Other` (review F8) | `CineEvent::from_combat` |
| Death presentation | Per-species finishes, including `llama_exploded` (point-blank blast), `llama_decapitated` (edge), `llama_crushed` (block), `llama_slammed` (heavy hit), `llama_down` (routine) | `src/death.rs`, `src/death_present.rs` (#272) |
| Floor run | One room per launch (K17); a relaunch carries `--f01-room`, `--f01-carry` (ledger) and `--f01-run-tally` and keeps every other flag | `src/f01_run.rs` |

Room order **at the time of writing** (verified on 81687b47): `corral`, `rest_refuge` (REST-WIRE, a quiet room with no kill), `hoarder`, `boiler`, `dozer`, `hazards`, `ring_court`, `ring_split`, `juicer` (boss), `mastery`. Llamas live in the corral (three) and in mastery (two). Carl's dynamite (`special_thrown_charge`) is granted by the Hoarder's defeat, so it is usable from the boiler on, and in mastery through its pin.

**Current room order (refreshed 2026-11-28 against `assets/levels/f01_late/manifest.json`, 18 rooms):** 1 `act1_chase`, 2 `act1_shore`, 3 `act1_guild`, 4 `corral`, 5 `rest_refuge`, 6 `hoarder`, 7 `boiler`, 8 `dozer`, 9 `hazards`, 10 `ring_court`, 11 `ring_split`, 12 `rest_camp`, 13 `trog_run`, 14 `trog_strength`, 15 `juicer`, 16 `rest_aftermath`, 17 `mastery`, 18 `bos`. The audience is live in the 14 rooms after the corral (5 to 18); rooms 1 to 4 never show it. Rooms 5, 12 and 16 are quiet rooms with no kill. The "Unlock in game: None" row above is the state before AUDIENCE-RUN; the unlock is now implemented ([section 12](#12-implementation-status)).

## 2. The unlock beat (deliverable 1)

**Adopted (AR-AD1): the broadcast goes live when the run leaves the Infernal Corral.** The corral (the fourth room of the run since ACT1-B, after the three Act I rooms) and the Act I rooms before it (the chase, the shore and the guild) play with no viewer decoration. Every run room listed after the corral (the refuge `rest_refuge` onward, the camp `rest_camp` and the aftermath `rest_aftermath` included) has both counts live; N is the current run length, read from the manifest. The refuge, the camp and the aftermath are quiet rooms with no kill, so no award happens there and decay runs unchanged (REST-AD14; REST-Q10 asks whether it should freeze).

Why this beat:

- **It keeps "No counters at Floor 01 start".** The run starts at the corral, which is the start of Floor 01 for the playtest. Spec line 11 and acceptance item 1 of AUDIENCE-01 stay true.
- **It is "explicit" and "later".** The unlock is a single named step (`broadcast_live`, the audience's own flag, F15), announced on the corral's room-clear panel ([AR-AD2](#7-adopted-values-ar-ad1ar-ad25)). It comes after the opening material, as the spec's candidate asked: the Tutorial Guild room (`act1_guild`, the old `guild_biscuit` point) and the corral's llama baptism.
- **It stays separate from the biscuit and mana unlocks.** `broadcast_live` is not a story-ledger capability and is not granted by any capability event. The ledger, `story_order` and the room pins do not change.
- **It never lands in combat.** A room transition is a calm moment behind a confirm panel. A mid-room trigger (for example, the first llama rush tell) was rejected: danger tells outrank spectator effects (AUDIENCE-01 rule), and a counter appearing during a rush would be read as part of the threat.
- **It is reachable for the playtest.** At the time of writing eight of nine rooms showed the counts; in the current 18-room run 14 rooms do (rooms 5 to 18, the rooms after the corral), including the Juicer, mastery, where dynamite and llamas meet, and the Ball of Swine.

The beat is **positional**: the audience is live in a run room if, and only if, that room's manifest row comes after the `corral` row. The rule does not depend on how the room was reached. A direct `--f01-run --f01-room boiler` start is live at the baseline, so the owner can open any later room for a check. The corral itself never unlocks, even with an explicit `--audience=on`, so the beat cannot be bypassed by accident. If the `corral` row is missing from the manifest, nothing unlocks (fail closed, with a unit test).

This answers AUD-Q1 for the `f01_late` run only. The canonical beat for the finished game's story stays open as [AR-Q1](#8-open-questions-ar-q1ar-q9).

### 2.1 Persistence across room changes, checkpoint respawn and Restart

The run starts a new process for each room (K17), and AUDIENCE-01 keeps its state in memory (F9). The counts travel on the relaunch argument list, the same way the ledger and the tally do:

- **New per-room flag `--f01-audience C:P:D:Q`** carries Carl's current count `C`, Carl's peak `P`, Donut's current count `D` and Donut's peak `Q`, all decimal `u64`. `relaunch_argv` replaces it like `--f01-room`, `--f01-carry` and `--f01-run-tally`. A relaunch writes it only when the audience was live at the exit tick. The values are the counts on that tick (`AudienceState::count` and the peak), with the decay remainder dropped (AR-AD3).
- **Room start:** in a live room with a carry, the audience resumes at the carried counts (`AudienceState::resume`, [section 9](#9-implementation-plan-one-pr)). Without a carry, it unlocks at the baseline of 250. Either way, the room-start snapshot is taken at once. The dedupe set, the chain map, fired cues, trend, inactivity timers and the count-up start fresh in each room, because target ids are reused from room to room.
- **Restart (in a room)** restores the room-start snapshot: the counts the room began with (F9, unchanged). A restart can never farm viewers, and it never takes away viewers earned in earlier rooms.
- **Checkpoint respawn (DEMO-01 `Respawn`)** keeps the counts and the dedupe set (F9, unchanged). A party defeat costs no viewers (AUD-Q3 stays open as [AR-Q7](#8-open-questions-ar-q1ar-q9)).
- **Validation:** the carry is checked when parsed: `P >= C >= 250` and `Q >= D >= 250`. Any other value is a parse error naming the flag. A carry given to the corral, or outside `--f01-run`, is also a parse error.
- **Floor end:** the counts end with the process. Persistence past the floor end waits for a save contract ([AR-Q8](#8-open-questions-ar-q1ar-q9)).

## 3. The F15 and F10 changes (deliverable 2)

**F15 is amended as owner-adopted (2026-11-25, AUDIENCE-RUN):** Floor 01's `--f01-run` unlocks `broadcast_live` at the corral exit, as section 2 describes. Every other Floor 01 path stays locked: fixtures, scenarios, captures, pilots, `--fixture`, `--party-fixture`, the demo and direct `--scene` runs without `--f01-run`. The preview path (`--hud-preview viewers | viewers-rising | viewers-falling | viewers-max`) and the QA probe (`--audience-probe [--audience-cue-x x]`) stay exactly as they are. Floor 01 still authors **no anticipation cue** (AR-AD19).

**F10 is kept word for word.** The audience is an observer, never mixed into `COMBAT_STATE` or `COMBAT_EVENT`. While locked, no step runs. Its `AUDIENCE_STATE` digest line prints only under the probe or an explicit `--audience=on` (AR-AD22), never in a default run.

**Flag gating (AR-AD6, AR-AD7).** Both new behaviours are on by default only in a live `--f01-run`. They are off in every fixture and capture unless a flag asks, and each has an opt-out:

| Flag | Default | Explicit `on` | Explicit `off` |
| --- | --- | --- | --- |
| `--audience[=on\|off]` (bare = `on`) | on iff `--f01-run` **and** a live session (the "run default" predicate below) | only with `--f01-run` (anything else is a parse error); live in rooms after the corral | locked everywhere; the opt-out persists across relaunches because `relaunch_argv` keeps unknown flags |
| `--cine-showcase[=on\|off]` (bare = `on`) | on iff the same run default | allowed on any path, for fixtures and evidence | main's CINE rules exactly |

The **run default** predicate is `f01_run && live_input() && room_pilot.is_none() && !ring_pilot && rest_pilot.is_none() && act1_pilot.is_none() && trog_pilot.is_none()` (`run_broadcast_default` in `src/bin/floor01_scene.rs`; the rest, Act I and troglodyte pilots were added after the first version, which listed only the room and ring pilots). `live_input()` already excludes headless, replay, scenario, fixture without `--manual`, autopilot, capture, perf, view-trace, HUD preview and the audience probe. The extra terms make the default off for every scripted pilot, so every evidence script that passes `--f01-run` (`make_heat01_evidence.sh`, `make_rings01_evidence.sh`, `make_hrd_cues_evidence.sh`, the `ci_floor01_combat.sh` corral entry) stays byte-identical. An explicit flag always wins over the default.

Showcase slow motion is also live in the corral, where the audience is still locked; only the plates wait for the unlock (the corral's three llamas can trigger `CINE-SHOWCASE-KILL`).

`--cinematic-finishes` (the presentation switch) is not changed. Showcase feats are **evaluator rules**. Whether a moment actually slows time is still decided by `PresentationSettings`, which is on for live play.

## 4. Showcase feats for CINE-01 (deliverable 3)

### 4.1 What the event stream proves

| Fact | Authoritative source | Notes |
| --- | --- | --- |
| A thrown stick's charge slot | `CombatEvent::ChargeThrown { tick, charge }` (the reserved `dynamite_slot()`, after every authored charge) | Emitted on release (t41) **and** when a hurt drops the lit stick (t6..t40, `thrower_hurt` on the hurt tick) |
| The stick's owner | `DefeatCause::Explosion { charge, player: Some(owner), .. }` (`arm_slot` arms the slot with `Some(PLAYER_ONE)`, Carl's hero index 0) | The sim sets it; nothing is inferred |
| One blast's kills | Every `EnemyDefeated { cause: Explosion { charge: c } }` that one detonation causes resolves on the `Blast { tick, charge: c }` tick | Goblins, Engineers, llamas, slot enemies |
| A drop instead of a throw | `HeroHurt { hero: owner }` on the same tick as that `ChargeThrown` | The drop is armed inside the hurt path |
| A llama target | Target ids: goblins `0..g`, llamas `g..g+l` (`g` = `enemies.len()`, test `llama_target_ids_follow_the_goblins_and_the_state_hash_sees_the_llama`) | The adapter reads `g` and `l` from `CombatState`, which is sim state, not presentation |
| A llama kill's credit | `Melee` with `instigator` (Carl's moves, Donut's swat, pounce and missiles), `ShoreBlock`/`Edge`/`DropHazard` drop credit, or a credited thrown blast | Same credit rules as main |

What the stream does **not** prove, and so stays out (fail closed):

- A chained authored charge's owner (`ChargeChained` copies the owner only if none is set, "first writer keeps both") → [AR-Q4](#8-open-questions-ar-q1ar-q9).
- A rot sticker provoked by a hero (the owner may come from a flier or projectile, not a deliberate act) → AR-Q4.
- A hazard prop kill (`Environment { player }`) as a feat → [AR-Q5](#8-open-questions-ar-q1ar-q9) (BOILBC-AD4 stands).
- Who caused a llama's rush to hit the partition (the bonk does not kill and credits no one) → [AR-Q3](#8-open-questions-ar-q1ar-q9).

### 4.2 New feats (only while `CineTuning::showcase_feats` is true)

| Evidence ID | Kind | Required observable facts | Score | Base targets |
| --- | --- | --- | ---: | ---: |
| `CINE-THROWN-MULTI` | primary | On the completing tick, at least **3** (`thrown_multi_min_kills`) distinct non-boss targets (goblins, Engineers **and llamas**; never the Juicer or the Hoarder) are defeated by `Explosion { charge: c, player: Some(p) }`. `c` must have a recorded **deliberate throw** in this attempt: a `ChargeThrown { charge: c }` on a tick with no `HeroHurt { hero: p }`. All other eligibility rules apply: `p` is clean for 90 ticks, cooldown, not locked, not companion-driven (CA-29). | **5** | 3 |
| `CINE-SHOWCASE-KILL` | primary | On the completing tick, `p` is credited with the defeat of a **showcase target**: a llama in v1, flagged by the adapter bridge `ShowcaseTarget { tick, target }`. Credit uses the existing rules: melee `instigator`, verified drop credit, or a credited thrown blast as above. | **5** | 1 |
| `CINE-SHOWCASE-BONUS` | bonus | A showcase target is among `p`'s credited defeats in the scored chain, and the selected primary is **not** `CINE-SHOWCASE-KILL` | **+1** | n/a |

Rules shared by the new feats (AR-AD9 to AR-AD17):

1. **Credited thrown-blast defeats.** With showcase on, a defeat with `Explosion { charge: c, player: Some(p) }` and a recorded deliberate throw of `c` is credited to `p` as a new `DefeatKind::Blast`. Every other explosion stays uncredited, exactly as on main: authored charges, chained charges, stickers, world blasts, the QA profile, `player: None`, a dropped stick, or a charge with no recorded throw. A release on the same tick as a hurt of the owner counts as a drop (fail closed). A credited blast defeat counts toward `CINE-EXTRA-KILL` in any chain. It never counts as the melee half or the drop half of `CINE-MIXED-CHAIN`.
2. **Primaries do not stack.** If several primaries hold, the selected one is the one with the **largest base**; ties go in the fixed order THROWN-MULTI, MIXED-CHAIN, PRECISION-EDGE, SHOWCASE-KILL. The score is that primary's score, plus `CINE-EXTRA-KILL` (credited defeats in the chain beyond the base, +1 each, max +2), plus `CINE-CONTACT-SETUP` (+2, unchanged), plus `CINE-SHOWCASE-BONUS` (+1). With showcase off, the largest-base rule gives exactly main's result: MIXED-CHAIN base 2 over PRECISION-EDGE base 1.
3. **Completing tick.** The two new primaries need their defining defeats **on the tick being evaluated**: one blast's batch, or the llama's own defeat. A llama kill that was refused (for example, by the cooldown) can never resurface later as a goblin kill's moment. The existing primaries keep their 90-tick window rule.
4. **Anchor.** The moment's `target`, which is the audience dedupe key and the planner's `finish_active` target, is the lowest showcase-target id of the batch when the selected primary is SHOWCASE-KILL. Otherwise it stays main's lowest credited target of the batch.
5. **Thresholds, windows, cooldown and upgrade are unchanged:** ordinary at 5..7, legendary at 8+, 90/90/900/8 ticks. The thrown-charge record is cleared on `Restart` and on `Respawn`, together with the rest of the history.
6. **The bosses never trigger.** `BOSS_ENEMY_ID` stays `CineCause::Other`, is never a showcase target, and never counts toward the three blast kills. A boss defeated by a blast that also kills two goblins gives no moment. The Hoarder's id (`PILE_BOSS_ID`) gets the same treatment in the bridge and in the THROWN-MULTI count (AR-AD25): it lies outside the llama id range and is never blast-defeated today, so this is only a guard against later changes.
7. **Llama-only off switch.** `showcase_kill_score = 0` disables `CINE-SHOWCASE-KILL` as a primary (it is then never selected) and keeps `CINE-THROWN-MULTI` and `CINE-SHOWCASE-BONUS`. This is a tuning value (data), not a CLI flag, so the owner can switch llama slow motion off alone if AR-Q2 says it is too frequent.
8. **Legendary paths.** Without a llama, THROWN-MULTI tops out at 5 + 2 = 7 (ordinary), so F4's statement "legendary only with a proven linked contact" still holds **for goblin-only chains**. A thrown blast that kills at least four goblins **and** a llama scores 5 + 2 + 1 = **8, legendary**. That is the run's "best one": it is possible in mastery, where llamas and dynamite meet.

Worked scores:

| Chain (credited hero clean, no cooldown) | Score | Tier |
| --- | ---: | --- |
| Thrown stick kills 2 goblins | none | (no primary) |
| Thrown stick kills 3 goblins | 5 | ordinary |
| Thrown stick kills 2 goblins + 1 llama (3 targets) | 5 + 1 = 6 | ordinary |
| Thrown stick kills 5 goblins | 5 + 2 = 7 | ordinary |
| Thrown stick kills 3 goblins + 1 llama | 5 + 1 + 1 = 7 | ordinary |
| Thrown stick kills 4 goblins + 1 llama | 5 + 2 + 1 = 8 | **legendary** |
| Dropped stick (Carl hurt on the drop tick) kills 5 goblins | none | (no credit) |
| Authored or chained charge kills 5 goblins | none | (T02) |
| Carl's push kick kills a llama | 5 | ordinary |
| Donut's missile kills a llama (Donut human-driven; a companion-driven Donut never triggers, CA-29) | 5, credited to hero 1 | ordinary |
| Melee goblin, then a shore-block goblin, then a melee llama, inside 90 ticks | 5 + 1 + 1 = 7 | ordinary |
| Juicer plus 2 goblins in one thrown blast | none | (boss excluded, 2 < 3) |

### 4.3 Amended CINE-01 text

- **Rule 1** gains this sentence: "With showcase feats on (AUDIENCE-RUN), a deliberate thrown-dynamite blast that kills three or more enemies is a primary feat, and a credited llama kill is a primary feat by owner decision (a *showcase species*: rare, dangerous charging enemies), an explicit exception to "necessary but not sufficient"; any other explosion still never triggers, and the bosses (the Juicer and the Hoarder) still never do."
- **The example "Three ordinary enemies killed simultaneously by an environmental blast: no slow motion"** stays true. "Environmental" means authored or unowned. A deliberately thrown stick by a clean hero is the new exception.
- **CINE-T02 is amended to:** "three routine kills / block-only / `qa_blast_breakup` `Explosion` defeats / an authored, chained, sticker or dropped charge's blast → no candidate, **with showcase on or off**. Only a deliberate thrown charge with a recorded `ChargeThrown` and an owner (`CINE-THROWN-MULTI`, at least 3 kills) is a blast primary, and only under `showcase_feats`."
- **The "bonuses alone never qualify" sentence** stays true. SHOWCASE-BONUS and EXTRA-KILL never qualify on their own.

The CINE-01 tech doc carries these amendments (section 2, section 5, section 7) (now implemented, game feel pending).

## 5. Audience scoring hooks (deliverable 4)

No new award constants. Everything reuses `audience::TUNING` (AR-AD18):

| Event | Award (credited hero) | Source |
| --- | --- | --- |
| Any credited `EnemyDefeated`, once per target per attempt | Routine 120, heavy 500, catastrophic 1,800, by the event's authoritative `force` | `defeat()` |
| A non-melee defeat whose instigator is the credited hero (a thrown-dynamite kill, a dropped rack, a shore block, a hazard the hero set off) | +900 environmental creativity | `defeat()` |
| A helper hero of a non-melee defeat | 30% of the actor's award | `defeat()` |
| A CINE moment, including THROWN-MULTI and SHOWCASE-KILL | `min(1000, 250 * (score - 4))`: 250 at 5, 500 at 6, 750 at 7, 1,000 at 8; an upgrade pays only the difference | `chain()` via `ChainFeat` |
| Boss defeat | Force tier and environmental bonus as usual (audience is not CINE; F8 limits CINE only) | `defeat()` |
| Hero hurt, party defeat, Restart farming | Nothing (AUD-Q3 open; Restart restores) | unchanged |

A dropped stick's kills still pay audience (factual credit through `credited_hero`, with the +900 bonus), though they never trigger CINE.

For example, Carl's 4-goblin-and-llama legendary stick is worth five defeat awards, each with the +900 bonus, plus a chain award of 1,000. That is the largest single spike in the run, and so the most visible "UP" trend arrow.

**The per-death-kind link is presentation-safe as built.** The audience award and the death finish read the **same authoritative pair**, `(DefeatCause, ForceTier)`, independently. A point-blank blast is catastrophic: it pays 1,800 and shows `llama_exploded` or `goblin_exploded`. A heavy push kick is heavy: it pays 500 and shows `llama_slammed`. The audience never reads `Finish`, `RecipeKind`, the gore preset, the cosmetic override flags or anything else from `death_present`. A test pins this down: gore preset 1 and preset 4 give the same `AUDIENCE_STATE` digest (AR-T17). No finish-driven award is added.

The CINE moment and the audience award are also one-way: the evaluator is a function of events only, and it never reads audience data (AUDIENCE-01, CINE rule 5).

## 6. Per-death-kind presentation in the run (deliverable c)

No new art or wiring is needed. DEATH-D (#108), LLAMA-DEATH-WIRE (#272) and the cosmetic presenter (#97) already run in the windowed scene, the `--f01-run` rooms included. AUDIENCE-RUN only **captures** them in the run, with slow motion over them, as evidence ([section 10](#10-evidence-to-capture)). The planner's existing rule, that a moment plays only while its target's finish is active, is kept unchanged.

## 7. Adopted values (AR-AD1..AR-AD25)

| ID | Adopted value | Why |
| --- | --- | --- |
| AR-AD1 | The unlock beat is the corral exit (the refuge onward, room 4 of N, N = 18 since BOS-A, was 15 at ACT1-B: the run length is read from the manifest; 14 rooms are live). The audience is live in every `--f01-run` room whose manifest row comes after `corral`. The rule is positional and fails closed if `corral` is not listed. | Section 2 |
| AR-AD2 | The corral's room-clear panel gets one fact line, only when the audience will be live next: `broadcast: live from the next room (viewers per hero)`. No number on any panel. The plates appear at the next room's first tick, at the carried or baseline counts, with no count-up from zero. | An explicit, calm reveal that stays inside the facts-only panel rule |
| AR-AD3 | `--f01-audience C:P:D:Q` carries the current count and the peak per hero at the exit tick. The decay remainder is dropped. Validation is `peak >= current >= 250`. | The smallest carry that keeps what the player sees |
| AR-AD4 | A live room started without a carry starts at the baseline of 250 for both heroes. | Same as `unlock()` |
| AR-AD5 | Restart restores the room-start snapshot. Respawn keeps the counts. The per-room dedupe, chain, cue and trend state starts fresh. | F9, plus target ids reused per room |
| AR-AD6 | `--audience[=on\|off]`: default on iff the run default predicate. Explicit `on` is allowed only with `--f01-run`. `off` locks everywhere and persists across relaunches. | Section 3 |
| AR-AD7 | `--cine-showcase[=on\|off]`: default on iff the run default predicate. Explicit `on` is allowed on any path. `off` gives main's rules. | Section 3 |
| AR-AD8 | The live adapter and the `AudienceFeatFeed` observer use the same `CineTuning`, including `showcase_feats` and the showcase-target bridge, so the chain feats of headless and fast-forward runs match the live loop's. | One rule set |
| AR-AD9 | `CINE-THROWN-MULTI`: score 5, base 3, `thrown_multi_min_kills = 3`, all of them on one blast's tick. Llama blast defeats count toward the 3 (any distinct non-boss target). | Section 4.2 |
| AR-AD10 | Deliberate-throw proof: `ChargeThrown { charge: c }` on a tick without `HeroHurt` of the charge's owner. A drop never credits. A release on the same tick as a hurt of the owner counts as a drop. Restart and Respawn clear the record. | Fail closed |
| AR-AD11 | Authored, chained, sticker, world and QA blasts, and `player: None`, stay uncredited | Section 4.1 |
| AR-AD12 | Credited thrown-blast defeats are `DefeatKind::Blast`: they count as extra kills and never complete MIXED-CHAIN | Keeps MIXED-CHAIN's meaning |
| AR-AD13 | `CINE-SHOWCASE-KILL`: score 5, base 1, a credited llama defeat on the completing tick. The anchor is the lowest llama id of the batch. `showcase_kill_score = 0` disables it alone (data, no flag). | The owner's "llama kills": a showcase-species exception to "necessary but not sufficient" |
| AR-AD14 | `CINE-SHOWCASE-BONUS`: +1 when a llama is in the scored chain and the primary is not SHOWCASE-KILL | Makes stick-plus-llama the legendary showcase |
| AR-AD15 | Primary selection: largest base wins; ties go in the order THROWN-MULTI, MIXED-CHAIN, PRECISION-EDGE, SHOWCASE-KILL. All primaries score 5, so they never stack. | Deterministic; equal to main when showcase is off |
| AR-AD16 | The boss never: not credited, not a showcase target, never one of the 3 | CINE rule 1, F8, boss design section 5 |
| AR-AD17 | Thresholds 5 and 8, windows 90 and 90, cooldown 900 and upgrade 8 are unchanged | Reuse |
| AR-AD18 | No new audience constants. Showcase moments pay through the existing chain formula. | Reuse |
| AR-AD19 | Floor 01 authors no anticipation cue. The cue table is empty in the run. | F15 kept for cues |
| AR-AD20 | No viewer number on the room-clear or floor-end panels | Facts-only panels, AUD-Q4 |
| AR-AD21 | `CINE01_MOMENT` appends ` thrown_multi=1`, ` showcase_kill=1` and ` showcase_bonus=1`, each only when set and before ` upgrade=1`. Legacy lines stay byte-identical. | Goldens |
| AR-AD22 | `AUDIENCE_STATE` prints only under `--audience-probe` or an explicit `--audience=on`. An explicit `--cine-showcase=on` on a headless or capture path prints observer `CINE01_MOMENT` lines, so party fixtures can show their moments: the `AudienceFeatFeed` adapter runs when the audience is unlocked **or** `--cine-showcase=on` is explicit, pushes feats to the HUD only when unlocked, and keeps today's early return otherwise. Fixture evidence with an audience uses the existing `--audience-probe` (same cue for both gore presets); `--audience=on` stays tied to `--f01-run`. | Evidence without touching legacy output or F15 |
| AR-AD23 | CA-29 is unchanged: a defeat credited to a companion-driven hero never triggers CINE, and audience credit stays factual. The adapter's `is_companion_credited` names the blast owner (`CineCause::Blast { player, .. } => player`), so an AI-driven Carl's thrown blast never reaches the evaluator. | Existing rule |
| AR-AD24 | Per-death-kind presentation: no new art or wiring, only evidence; the planner's `finish_active` rule is unchanged | Section 6 |
| AR-AD25 | The Hoarder (`PILE_BOSS_ID`) is excluded like the Juicer: never a showcase target, never one of the 3 blast kills. `push_driven` also skips the `ShowcaseTarget` push when the defeat itself is filtered as companion-credited. | Review finding 11, robustness |

## 8. Open questions (AR-Q1..AR-Q9)

| ID | Question | Default until answered |
| --- | --- | --- |
| AR-Q1 | Is the corral exit the canonical broadcast beat for the finished game's story, or only for the `f01_late` playtest run? (AUD-Q1 for the full game) | Corral exit, `f01_late` run only |
| AR-Q2 | Is every clean llama kill too frequent for slow motion? (The corral can give two or three in a few minutes; the 900-tick cooldown limits it.) | Every clean, credited llama kill qualifies |
| AR-Q3 | Should a llama kill shortly after its rush ended `Miss`, `Blocked` or `Partition` earn a punish bonus? (`LlamaRushEnd` proves the timing but credits no hero for the bonk.) | No bonus |
| AR-Q4 | Should chained authored charges and provoked rot stickers carry the thrower's credit into THROWN-MULTI? | No: uncredited |
| AR-Q5 | Should a hero-credited hazard prop kill (`Environment { player }`) become a primary, or a MIXED-CHAIN partner? (BOILBC-AD4) | No |
| AR-Q6 | Should THROWN-MULTI need 2 kills instead of 3, or should a goblin-only stick reach legendary (raise the extra-kill cap)? The stick kills only in its 20 px core (the outer ring launches bodies alive), so three core kills need a tight cluster. A launched goblin that dies later is a different tick, and its kill is not counted. | 3 core kills on one tick; cap +2 |
| AR-Q7 | Does a party defeat cost viewers or become a humiliation spike? (AUD-Q3) | No change |
| AR-Q8 | Should viewers persist past the floor end or across sessions? (A save contract is needed.) | Process lifetime only |
| AR-Q9 | Should the reveal get a narrative card or System announcement beyond the panel line? (Written to the antihero tone, with no invented quotes.) | Panel line only |

AUD-Q2 (balance) and AUD-Q4 (stage-end ranking) remain open as before.

## 9. Implementation plan (one PR)

Medium tier, one implementation PR. File by file:

1. **`src/cine.rs`**
   - Add `CineCause::Blast { player: Option<u32>, charge: u32 }`. `cine_cause` maps `Explosion` to it (the boss stays `Other`). Its sort component is `8`, the same as `Other`.
   - Add `CineEvent::ChargeThrown { tick, charge }`, mapped in `from_combat`, and `CineEvent::ShowcaseTarget { tick, target }`, an adapter bridge only.
   - Renumber the sort keys so `ChargeThrown` and `ShowcaseTarget` sort after `HeroHurt` and before `Defeated`. Existing events keep their relative order.
   - Add `CineTuning` fields: `showcase_feats: bool` (false in `DEFAULT`), `thrown_multi_min_kills: u8 = 3`, `thrown_multi_score: u8 = 5`, `showcase_kill_score: u8 = 5`, `showcase_bonus: u8 = 1`. Add `CineTuning::with_showcase(bool)`. Validation: `thrown_multi_score > 0`; `showcase_kill_score == 0` (SHOWCASE-KILL off) or `>= ordinary threshold`; `thrown_multi_min_kills >= 2`.
   - Add `DefeatKind::Blast` and a `showcase: bool` field on `DefeatEntry`.
   - Add a bounded record of thrown charges: `(charge, tick, owner_hurt_mask)`, at most one entry per charge index, cleared on Restart and Respawn.
   - Extend `credit_of`, `score_player` (primary selection by base, completing-tick checks, anchor, bonus) and `FeatSet` (`thrown_multi`, `showcase_kill`, `showcase_bonus`).
2. **`src/cine_adapter.rs`**
   - Add `CineAdapter::set_showcase_targets(first: u32, count: u32)`. Before pushing a `Defeated` whose target is in range, `push` and `push_driven` push `ShowcaseTarget` for the same tick, only while `showcase_feats` is true.
   - Make `with_tuning` take the showcase tuning.
   - CA-29: in `is_companion_credited`, map `CineCause::Blast { player, .. } => player`. Skip the `ShowcaseTarget` push when the defeat is filtered. Exclude `BOSS_ENEMY_ID` and `PILE_BOSS_ID` from the bridge.
3. **`src/audience.rs`**
   - Add `AudienceState::resume(counts: [(u64, u64); 2]) -> Result<(), ResumeError>`: unlock with the given current and peak per hero, validated, then `begin_attempt()`. A second call is a no-op, like `unlock`.
4. **`src/hud.rs`**
   - Add `HudRuntime::resume_audience(counts)`, a sibling of `unlock_audience` with an empty cue table.
5. **`src/f01_run.rs`**
   - Add `FLAG_AUDIENCE_CARRY = "--f01-audience"`, `AudienceCarry { carl: (u64, u64), donut: (u64, u64) }` with `to_arg` and `parse`, and `AUDIENCE_UNLOCK_AFTER: &str = "corral"`.
   - Add `audience_live_in(manifest, room) -> bool`.
   - `relaunch_argv` also replaces `--f01-audience`, and takes `Option<AudienceCarry>`.
   - `ExitInput` gains `audience: Option<AudienceCarry>`.
   - `room_clear_panel` gains the AR-AD2 line, only when the room being left is the unlock room and the audience is enabled.
6. **`src/bin/floor01_scene.rs`**
   - Parse `--audience`, `--cine-showcase` and `--f01-audience` with strict errors, and add `Args::run_broadcast_default()`, `Args::audience_enabled()` and `Args::cine_tuning()`.
   - In `new_hud`, resume or unlock when `audience_enabled() && audience_live_in(room)`.
   - Build the CINE resource and `AudienceFeatFeed` with `cine_tuning()`.
   - Call `set_showcase_targets` from the combat state's goblin and llama counts, before each push batch (live fan-out and feed).
   - Fill the exit carry.
   - Extend `cine_moment_line`.
   - Print `AUDIENCE_STATE` per AR-AD22, and the headless observer moment lines.
   - `AudienceFeatFeed::observe`: run the adapter when the audience is unlocked **or** `cine_showcase == Some(true)`; push `ChainFeat`s to the HUD only when unlocked; print observer `CINE01_MOMENT` lines only for an explicit `--cine-showcase=on` on a headless or capture path; keep today's early return when neither holds.
7. **Fixtures:** add `carl_prog_dyn_thrown_triple` and `carl_prog_dyn_llama_pack` (party fixtures with the thrown charge held, next to `carl_prog_dyn_core_kill`), in `src/carl_prog_fixtures.rs`. Each gets its own `GOLDENS` row and its own committed replay under `docs/evidence/carl-prog-01/replays/`, and the registry grows from 32 to 34. Thrown charges kill only in the 20 px core (`cpq20_core_radius_px`), so the goblins stand tightly around the landing spot (x 511, as in `carl_prog_dyn_core_kill`). If the simulation cannot place three living bodies in the core, the implementer records that as a blocker. Changing the blast rules is not an allowed workaround.
8. **CI:** extend `scripts/ci_floor01_combat.sh cine` with showcase probes and new committed evidence files under `docs/evidence/audience-run/`.
9. **Docs:** the floor-run section of `docs/03-tech/floor01-scene.md` (flags), the owner commands in `docs/evidence/f01-playtest/README.md` and `scripts/check_f01_owner_commands.sh` (`--audience=off`, a direct `--f01-room hoarder` start), and the "implementation status" rows of both specs.

### 9.1 Tests and fixtures (each must exist and pass)

| ID | Test (name) | Expected result |
| --- | --- | --- |
| AR-T01 | `cine::ar_showcase_off_keeps_main_rules` | `showcase_feats = false`: a thrown blast killing 5 goblins and a llama gives `None`; a melee llama kill gives `None`; every pre-existing `cine.rs` test passes unchanged |
| AR-T02 | `cine::ar_thrown_triple_is_one_ordinary_moment` | `ChargeThrown { c }` at t, three `Explosion { c, Some(0) }` defeats at t+150 → one moment: score 5, ordinary, `thrown_multi`, credited 0, target = lowest id, `extra_kills` 0 |
| AR-T03 | `cine::ar_thrown_pair_is_not_a_feat` | Two kills → `None`, `Rejection::NoQualifyingFeat` |
| AR-T04 | `cine::ar_four_goblins_and_a_llama_is_legendary` | Four goblins and a `ShowcaseTarget` llama on one blast → score 8, legendary, `thrown_multi`, `showcase_bonus`, `extra_kills` 2 |
| AR-T04b | `cine::ar_a_llama_counts_toward_the_three_blast_kills` | Two goblins and a `ShowcaseTarget` llama on one blast → score 6, ordinary, `thrown_multi`, `showcase_bonus`, `extra_kills` 0 |
| AR-T05 | `cine::ar_a_dropped_stick_never_credits` | `ChargeThrown` with `HeroHurt { 0 }` on the same tick, a blast 150 ticks later killing 5 → `None`; sub-case: a t41 release with a hurt of the owner on the same tick → `None` |
| AR-T06 | `cine::ar_unthrown_authored_or_ownerless_blasts_never_credit` | A charge with no `ChargeThrown`, an authored charge index with `Some(0)`, `player: None` → `None` with showcase on (amended T02) |
| AR-T07 | `cine::ar_the_boss_never_counts` | Boss plus 2 goblins in one thrown blast → `None`; a `ShowcaseTarget` for `BOSS_ENEMY_ID` is ignored |
| AR-T08 | `cine::ar_a_hurt_in_the_clean_window_vetoes_the_blast` | `HeroHurt { 0 }` 60 ticks before the blast → `None`, `NotClean` |
| AR-T09 | `cine::ar_a_clean_credited_llama_kill_is_a_showcase_moment` | Melee llama kill (`instigator` 0) → score 5, `showcase_kill`, target = llama id; with `instigator` 1 → credited 1; with `showcase_kill_score = 0` → `None` (and a thrown triple with a llama still scores 6 via THROWN-MULTI + SHOWCASE-BONUS) |
| AR-T10 | `cine::ar_an_uncredited_llama_kill_is_nothing` | Llama killed by an authored blast or by `Environment` → `None` |
| AR-T11 | `cine::ar_cooldown_and_no_stacking_hold_for_showcase` | Llama moment, then a thrown triple 300 ticks later → second `Cooldown`; melee + shore + llama melee in one window → 7 (5 + 1 + 1), not 10 |
| AR-T12 | `cine::ar_restart_and_respawn_forget_the_throw` | `ChargeThrown`, `Restart` (or `Respawn`), then three blast kills of `c` → `None` |
| AR-T13 | `cine::ar_blast_kills_are_extras_in_a_mixed_chain` | Melee + shore + one credited thrown-blast kill → 6, `mixed_chain`, `extra_kills` 1 |
| AR-T14 | `cine::ar_showcase_choice_is_order_independent` | The same batch in shuffled orders → the same moment |
| AR-T15 | `cine_adapter::showcase_targets_are_bridged_only_when_on` | With showcase off, no `ShowcaseTarget` is pushed and the moment equals main; with it on, the llama-range defeat gets the bridge; `BOSS_ENEMY_ID` and `PILE_BOSS_ID` never do |
| AR-T15b | `cine_adapter::a_companion_driven_thrown_blast_never_reaches_the_evaluator` | `companion_driven[0] = true`, showcase on, a thrown triple through `push_driven` → `None` (CA-29) |
| AR-T16 | `audience::resume_carries_counts_and_starts_a_fresh_room` | `resume` → counts and peaks as given, empty dedupe, Restart restores the carried counts, Respawn keeps them, bad carries rejected |
| AR-T17 | `floor01_scene::audience_digest_ignores_the_gore_preset` | The same fixture with `--audience=on`-equivalent setup at gore presets 1 and 4 → identical `AUDIENCE_STATE` digests |
| AR-T18 | `f01_run::audience_carry_round_trips_and_is_replaced_on_relaunch` | `to_arg` and `parse` round-trip; `relaunch_argv` drops an old `--f01-audience` and writes the new one once |
| AR-T19 | `f01_run::the_audience_is_live_only_after_the_corral` | `corral` → false; every later row → true; a manifest without `corral` → false everywhere |
| AR-T20 | `f01_run::only_the_unlock_room_panel_announces_the_broadcast` | Corral panel with the audience enabled has the AR-AD2 line; with it disabled, and on every other panel, the lines equal main's; no panel shows a count |
| AR-T21 | `floor01_scene::broadcast_defaults_are_on_only_for_a_live_f01_run` | On: `--scene f01_late --f01-run`. Off: the same with `--headless-floor01-smoke`, `--capture-tick 90`, `--ring-pilot --f01-room ring_court`, the HRD-CUES room pilot, `--audience=off`. Parse errors: `--audience=on` without `--f01-run`, `--f01-audience` without `--f01-run`, a bad carry |
| AR-T22 | `floor01_scene::a_thrown_triple_reaches_the_audience_on_headless_and_live_paths` | The `carl_prog_dyn_thrown_triple` events through the live fan-out and through `AudienceFeatFeed` → the same `ChainFeat` (score 5) and the same digest; with the audience locked and `--cine-showcase=on` explicit, the feed still produces the moment (printed, not pushed to the HUD); with both off, the feed returns early as on main |
| AR-T23 | Fixture `carl_prog_dyn_thrown_triple` (new) | Solo, thrown charge held, three goblins inside the ring at the landing spot → one `CINE01_MOMENT ... thrown_multi=1` (score 5) on the `Blast` tick with showcase on; none with it off; own hash row |
| AR-T24 | Fixture `carl_prog_dyn_llama_pack` (new) | Four goblins and a passive llama in the ring → one legendary moment, score 8, `showcase_bonus=1`; `llama_exploded` finish; own hash row |
| AR-T25 | Existing fixtures with `--cine-showcase=on` | `carl_prog_dyn_llama_core` → `showcase_kill=1`, score 5, target = llama id; `carl_prog_dyn_core_kill` → none; `carl_prog_dyn_boss` → none; `qa_blast_breakup` → none; `cp05_llama_slam` → `showcase_kill=1` (windowed: also `CINE01_SLOWMO start`). If a fixture's log shows a hurt of the credited hero inside the clean window, the expected result is no moment (`NotClean`): record the actual output and use another existing llama fixture for the positive case. Never adjust a rule to fit a fixture. |
| AR-T26 | `thrown::a_drop_logs_the_hurt_and_the_throw_on_one_tick` | The sim fact AR-AD10 relies on: a hurt on t6..t40 logs `HeroHurt` and `ChargeThrown` on the same tick |

**Pins that may move:** none of the legacy goldens. That covers `ci_floor01_combat.sh headless`, `cine` (the B3 and pileup lines at ticks 159 and 139), `render`, every `COMBAT_STATE` hash, and every committed evidence file. The only allowed changes are **additive**: the `GOLDENS` rows, committed replays and registry count (32 to 34) for the two new `carl_prog_dyn_*` fixtures, the new probes and committed files under `docs/evidence/audience-run/`, and the `from_combat` unit test's list of mapped events (it now includes `ChargeThrown`, and its `Explosion` assertion moves from `Other` to `Blast`). Any other moved pin is a defect.

## 10. Evidence to capture

Commit everything under `docs/evidence/audience-run/`, with a README holding the exact commands and SHA-256 sums:

1. **Showcase moment lines** (`--cine-showcase=on`; windowed with `--cinematic-finishes=on --exit-after-ticks N` for `cp05_llama_slam`, headless observer lines for the party fixtures): AR-T23 to AR-T25, with the `CINE01_SLOWMO start` line where the run is windowed.
2. **Gore-preset parity:** `--party-fixture carl_prog_dyn_llama_pack --audience-probe --cine-showcase=on` at gore preset 1 versus preset 4 (the same probe cue in both), with identical `COMBAT_EVENT`, `CINE01_MOMENT` and `AUDIENCE_STATE` lines.
3. **Run reveal stills** (640x360, scale 1 and 3): the corral room-clear panel with the broadcast line; the hoarder room's first frame with both plates at 250; a later room with carried counts (a room pilot with `--audience=on` and an explicit carry, for example `--f01-run --f01-room boiler --audience=on --f01-audience 1400:1900:600:800`).
4. **Slow motion over a per-kind finish in the run:** a `capture-sequence` or room-pilot GIF in mastery or the boiler with `--audience=on --cine-showcase=on --cinematic-finishes=on`, showing a thrown multi-kill (exploded finishes) and the spike trend on Carl's plate. If no pilot throws the stick, record that and capture the `carl_prog_dyn_llama_pack` fixture instead, with `--audience-probe --cine-showcase=on --cinematic-finishes=on`. **Never fake a run capture.**
5. **Restart and respawn:** a headless log showing the carried counts restored after `Restart` and kept after `Respawn`.
6. **The opt-out:** `--audience=off --cine-showcase=off` in the run → no plates, and moment lines equal to main's.

## 11. Fit with the horizon and the tone

- **Living-horizon check** ([character-progression-horizon](character-progression-horizon.md)): this design adds no move, ability, unlock of play or co-op rule. It rewards existing moves (the thrown charge, Donut's missiles, Carl's kicks) through presentation and a cosmetic counter. **It still fits; the guide needs no update.**
- **Antihero tone** ([antihero-tone](antihero-tone.md)): the broadcast is the dungeon's cynical spectacle, not the heroes' vanity. The plates only count. The reveal line is a flat fact, with no catchphrase, no announcer quote and no invented dialogue. Slow motion stays short, so it reads as a hard-won moment rather than a power fantasy.
- **IP-neutral contracts:** the engine-side names are generic (`ShowcaseTarget`, `CINE-THROWN-MULTI`, `showcase_feats`). The llama is the v1 showcase species, chosen in the scene adapter, and can be replaced like any fan-concept layer.

## 12. Implementation status

Implemented in the AUDIENCE-RUN PR (medium tier, from this design; review by a separate top-tier agent). Status after merge: **merged, game-feel review pending**. Nothing here is game-feel accepted; the owner judges it in the playtest and any adopted value can change in a follow-up PR.

| Piece | Where | Tests |
| --- | --- | --- |
| Showcase feats, `CineCause::Blast`, `ChargeThrown` and `ShowcaseTarget` events, tuning fields and `validate` | `src/cine.rs` | AR-T01 to AR-T14 (`cine::ar_*`), the `from_combat` mapping test |
| Showcase-target bridge, CA-29 for `Blast`, boss exclusions | `src/cine_adapter.rs` | AR-T15, AR-T15b |
| `AudienceState::resume` and `carry`, `HudRuntime::resume_audience` and `audience_carry` | `src/audience.rs`, `src/hud.rs` | AR-T16 |
| `--f01-audience`, `audience_live_in`, the corral panel line | `src/f01_run.rs` | AR-T18 to AR-T20 |
| `--audience`, `--cine-showcase`, run defaults, resume at room start, feed and live adapter tuning, observer moment lines, exit carry | `src/bin/floor01_scene.rs` | AR-T17, AR-T21, AR-T22 |
| The drop-and-hurt same-tick fact | `src/combat_scene/thrown.rs` | AR-T26 |
| `carl_prog_dyn_thrown_triple` and `carl_prog_dyn_llama_pack` | `src/carl_prog_fixtures.rs` | AR-T23, AR-T24; AR-T25 through the `cine` CI probes |

Headless and capture observer `CINE01_MOMENT` lines (explicit `--cine-showcase=on`) come from the pre-simulation feed and are not tick-ordered against the `COMBAT_EVENT` lines; consumers filter by prefix. `--audience=off` together with `--f01-audience` is a parse error. Co-op watch item: a goblin last hit by a companion-driven hero and then killed by the player's credited stick is dropped by the CA-29 filter (a missed moment, never a false one); revisit with AR-Q4 if the playtest shows missed moments in co-op.

Evidence: [`docs/evidence/audience-run/`](../evidence/audience-run/README.md). Notes recorded by the implementation (AR-IMP-n) are in the PR body.

## AUD-FIN-01: the Ball of Swine, finish-aware notices and the floor-end viewers

Added after the audit of the death presentation against the audience (the Ball's room, the System notices, the floor-end panel). Design by the orchestrator's top-tier analysis, adopted under the owner's design-defaults rule; every value below is data or a named constant a follow-up PR can change. **Living-horizon check:** no move, ability, unlock or co-op rule changes (copy, a panel line, a data row and tests); it still fits [character-progression-horizon](character-progression-horizon.md) and the guide needs no update. Tone: short deadpan broadcast copy in the house style of the existing notices, no catchphrase, no quote ([antihero-tone](antihero-tone.md)).

| ID | Decision | Where |
| --- | --- | --- |
| AUDFIN-AD1 | The Ball raises its own notice set, chosen by the boss data's species (`rolling_brute`): intro `ROLL CALL!` (the gate closes), crash `WALL WINS!` (the charge ends on a wall, `BossCrash`), pin `DEAD WEIGHT!` (a rack crushes it), defeat `BACON BOWLED!`. `ROUND TWO` and `FINAL ROUND` stay shared. The Juicer's `MAIN EVENT!`, `NO SPOTTER!` and `KNOCKOUT!` are untouched (`notice_for_event` is still the Juicer's entry point). Every headline fits the 13 character frame. | `src/hud.rs` (`BossNotices`, `notice_for_event_with`), `HudRuntime::set_boss_species` |
| AUDFIN-AD2 | The finale awards viewers exactly as before (a weight drop is 1,800 + 900, a melee finish by force). The `FLOOR 1 CLEARED` (and `FLOOR 1 SLICE FINISHED`) panel gains one flat fact line, `viewers at the exit: 12,345 (Carl 8,000, Donut 4,345)`: the current counts at the exit tick, summed, with the per-hero split, shown only while the broadcast was live in the last room. This **amends AR-AD20 for the floor-end panel only**: no room-clear panel shows a viewer number, there is still no peak, no ranking and no rating (AUD-Q4 stays open), and the closing line `Facts only: no score, no rating.` stays. The Ball still has no plate, no slow motion (AR-AD20, BOS-AD10). | `src/f01_run.rs` (`viewers_line`, `floor_end_panel`) |
| AUDFIN-AD3 | A defeat notice follows the finish the death presenter chose when that finish has its own headline, and the force table (`LAUNCHED!`, `FLATTENED!`) is the fallback. Mapping: exploded `BLOWN AWAY!`; decapitated `HEADS UP!`; crushed `FLATTENED!` (the existing crush line); every other finish (knockback, slammed, `NoArt`, a species with no recipe for the cause) keeps its force notice. A boss defeat keeps its boss headline. The presenter is the only judge of the finish, so a species with no decapitation never shows `HEADS UP!`. Presentation only; nothing is hashed. | `src/death_present.rs` (`finish_notices`), `src/hud.rs` (`DefeatFinish`, `HudRuntime::set_defeat_finishes`) |
| AUDFIN-AD4 | The Ball plays its existing `down` strip through the profile path: a `rolling_brute` species row whose every recipe and the fallback select `ball_down`, which `Finish::from_id` maps to `NoArt` (the room's own `ball_art` draws the strip, BOS-ART-AD4). No new art, no gore, no CINE. | `assets/data/death/species_death_profiles.json` |
| AUDFIN-AD5 | HAZ-Q16: a `slime` species row, every recipe and the fallback selecting `slime_down` (`NoArt`: the slime sprite's own `down` frames 13 to 15, "remains, not a stain"), so the slime no longer falls to `generic_collapse`. | same file |
| AUDFIN-AD6 | Launch tests pin the audience state of every room of the run (Act I and the corral locked, the 14 later rooms live at the baseline). | `src/bin/floor01_scene.rs` |
| AUDFIN-AD7 | `CineEvent::from_combat` maps the Hoarder (`PILE_BOSS_ID`) to `CineCause::Other` like `BOSS_ENEMY_ID`, instead of relying on the planner only. A no-op for every output. | `src/cine.rs` |
| AUDFIN-AD8 | `TRY n` stays hidden in the two boss rooms. The Hybrid-A spec decided the location strip is shown "always, except while the boss strip shows" (they share the centre band) and the viewers HUD brief forbids new permanent panels; the counter shows again once the strip slides out. A test pins it. | `docs/02-art/hybrid-hud-a-native-spec.md`, `docs/02-art/individual-viewers-hud-v1.md`, `src/hud_draw.rs` |

The two new species rows are the only change to the global profile file: existing rows and the cf02 and troglodyte overlays are byte-identical. Open: whether the Ball should one day get its own plate, slow motion or a stage-end ranking stays with AUD-Q4 and BOS-C.
