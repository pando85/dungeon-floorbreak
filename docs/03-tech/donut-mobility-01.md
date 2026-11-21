# DONUT-MOBILITY-01 — Donut's agile movement and evasion

**Status:** design spec, adopted defaults (orchestrator, owner directives 2026-11-19 and 2026-11-21). Nothing here is implemented. Game feel stays the owner's call after merge.
**Work order:** [donut-mobility-01-workorder.md](../07-agents/donut-mobility-01-workorder.md).
**Designer tier:** top tier (a gameplay design decision with no settled spec, from a direct owner request). Execution: medium tier. Review: an independent top-tier agent.

## 0. Owner decision (2026-11-21)

Recorded as given, across the request and the two clarifications relayed by the orchestrator:

> "Donut must be faster all the time (human or AI). She jumps higher and further and she moves faster. That's one of their strengths. Also, she has more dodge and she is capable of avoid some attacks. [...] 15% probability for evading attacks."

The owner's clarifications, relayed by the orchestrator (paraphrase, not a quote):

- "15%" means a **chance to evade attacks**, not a speed value. Run speed, jump height and air distance are the designer's call.
- A 15 % passive evasion is too much when a human also dodges with her. So:
  - the recommended mode rearms after about 8 s;
  - it aims at 5 to 8 % of evadable hits;
  - it evades one hit per arm, with no invulnerability window after it.
- The seeded roll defaults to 8 %, and 15 % is kept as a tunable value.
- Everything is data. The same rules apply whether a human or the AI plays her.

**Owner decision (2026-11-21, relayed by the orchestrator): evasion is a story unlock.**

- Neither evasion mode is available from the start. Donut gains the skill during Floor 1, at the **biscuit event in the Tutorial Guild**, the same story unlock as Magic Missile.
- Evasion is gated on the **existing biscuit unlock state** (`DonutMana::unlocked()`, set by the guild zone or declared by a fixture's `DonutSetup`, and by `--donut-biscuit` in CF02). No new flag is added.
- The faster run and the higher, longer jumps are hers **from the start**, as the horizon already says for pre-biscuit Floor 01.

**Interpretation adopted here:**

- "All the time" means: in every state, under every controller (P1, P2, solo swap, AI companion), whenever the agile profile is on.
- The profile is **on by default in the `--cf02` playtest profile** and off elsewhere until the owner accepts the game feel (section 5). A later flip of the default is a separate PR with a golden rebaseline.

## 1. What exists today (verified on `origin/main` at `62686919`)

| Topic | Fact | Where |
| --- | --- | --- |
| Hero body step | One function for both heroes: `vx = move_x * RUN_SPEED` (no acceleration), jump `vy = JUMP_SPEED`, then `vy -= GRAVITY * dt`, then `y += vy * dt`. Coyote 6, jump buffer 7. No variable jump height | `src/prototype.rs` `step_hero_driven` |
| Constants | `RUN_SPEED` 185, `JUMP_SPEED` 440, `GRAVITY` 1050. Discrete apex **88.5 px**, 50 air ticks, flat air distance **154 px** | `src/prototype.rs` |
| Per-hero movement | **None in the integrated scene.** The live party steps Carl and Donut through the same `step_hero_driven` with the same constants and the same 24 x 52 collider (`SceneSim::step_bodies_driven`). Only the camera lab has its own Donut (run 205, jump 600, 32 x 28 body), which the integrated scene does not use | `src/corridor.rs`, `src/camera_lab/sim.rs` |
| Donut's hurtbox | Separate from her collider: `DonutHurtbox::Forgiving` **20 x 24** (Q4). The llama's spit line (42 to 48 px) already passes over a grounded Donut (CF02-Q44) | `src/party.rs`, `CombatState::hurtbox` |
| "Dodge" today | No dodge verb (controller contract, CF02-Q26: no evade command). The **AI** "dodge" is the follower bot's jump when a goblin tell starts within `dodge_radius_px` 40 px and no later than tell tick 20 (D1 M3). Carl's bail-out hop (Q54) and Donut's cat hop (Q55) cancel move recoveries into a hop, **no i-frames** | `src/follower.rs`, `cf02_combat_v0.json` |
| How enemy hits resolve | Each source tests its box against `CombatState::hurtbox` and calls `hurt_hero_by` / `hurt_hero`. An invulnerable hero (`invulnerable_ticks > 0`) or a guarded off-screen hero (D1 M4) is skipped. Sources: goblin swing (`swing_on`, impact ticks 26..30, box 8..30 x 0..30), Engineer ram (box 14..24 x 14..33, up to 20 active ticks), llama glob (8 x 6 at 42..48 px), the shore block (CF02-Q42), boss blows (boss scene) | `src/combat_scene.rs` |
| Nav graph | Capability-aware: each jump or drop edge is simulated per hero through `step_hero_driven`, with a `HeroPhysics { jump_speed, run_speed }` per hero. Gravity is shared. `NavConfig::integrated()` gives Donut Carl's physics; `navfollow` always builds with it | `src/companion/nav.rs`, `navfollow.rs` |
| 2P separation wall | Predicts each hero's step with the shared `RUN_SPEED` and zeroes `move_x` past `view_w - 2 * 96` (448 px at 640) | `src/corridor.rs` `apply_separation_wall` |
| Scroll camera | Normal pan cap `MAX_SPEED_PX` **4 px/tick** (240 px/s) | `src/scroll_camera.rs` |
| Tuning profile | `--cf02` sets `cf02_rules` and applies `PartyTuning::with_cf02_swap()` to the live party. The arena fixtures use `ARENA_SOLO` / `ARENA_COOP`, which mirror it. Every other `cf02_*`, `cf02_assist_*` and `companion_*` fixture uses `PROVISIONAL_TUNING` | `src/party.rs`, `src/bin/floor01_scene.rs`, `src/cf02_fixtures.rs` |

The controller contract already states the intent: Donut's Jump is a "higher/farther cat jump", and "avoidance comes from positioning, jumps, and Donut's agility" with no dodge button ([mvp-controller-contract.md](../01-design/mvp-controller-contract.md)). This spec gives that intent numbers and adds **no input**.

## 2. Movement numbers (DM-Q01 to DM-Q09)

All values come from `assets/data/gameplay/hero_agility_v0.json`, which is new, embedded, `deny_unknown_fields`, FNV-1a pinned like `cf02_combat_v0.json`. They are read only when the agile profile is on (DM-Q10). Ticks are at 60 Hz. The numbers below come from the discrete step, not the continuous formula.

| ID | Knob | Value | Carl (unchanged) | Why |
| --- | --- | --- | --- | --- |
| DM-Q01 | `dm_q01_run_speed` | **207 px/s** (+11.9 %, 3.45 px/tick) | 185 (3.08 px/tick) | The orchestrator's ~+12 %. A 2 s run puts her 44 px ahead of Carl: visibly quicker, not a different game. It stays under the scroll camera's 4 px/tick cap, so a controlled Donut never outruns the camera. **Invariant:** any later value must stay below 240 px/s |
| DM-Q02 | `dm_q02_jump_speed` | **485 px/s** launch | 440 | Discrete apex **108.0 px**, against Carl's 88.5. Floor 1 uses 16 px tiles: she lands on a **96 px (6-tile) ledge** with 12 px to spare, and Carl cannot. A 112 px (7-tile) ledge stays out of reach for both. "One extra tile" exactly |
| DM-Q03 | none (shared `GRAVITY`) | 1050, unchanged | 1050 | Same arc shape, just taller. No apex hang, no fast-fall, no variable height: the nav model (`HeroPhysics`) assumes shared gravity, and a per-hero gravity would need a new nav field and new replay validation for little gain |
| DM-Q04 | derived, no knob | **55 air ticks**, flat air distance **190 px** (+23 %) | 50 ticks, 154 px | "Further" comes from the higher launch and the faster run together. Air control is Carl's rule (`vx = move_x * run`) |
| DM-Q05 | none | No acceleration or deceleration: instant velocity, as Carl | instant | It keeps the step deterministic, keeps the nav simulation exact, and keeps the controller feel crisp. Momentum is Mongo's niche ([characters.md](../01-design/characters.md): "Medium, momentum-driven") |
| DM-Q06 | none | Coyote 6, jump buffer 7, unchanged | same | Input semantics of the contract are untouched |
| DM-Q07 | none | **Not scaled:** swat, pounce, Magic Missile, the cat hop (Q55), Carl's bail-out, knockback, hurt motion, any `BodyDrive` launch or `vx` override. Her **collider stays 24 x 52** and her **hurtbox stays Q4 `Forgiving` 20 x 24** | n/a | v1 changes walking and the ordinary jump only. Move travel is tuned by its own CF02 knobs |
| DM-Q08 | `dm_q08_ai_jump_margin_px` | 4 | 4 | The AI's "can I jump this" reach becomes per hero: the discrete apex of that hero's launch minus this margin (104 px for agile Donut, 84.5 for Carl). Today `follower.rs` uses the continuous `JUMP_SPEED² / 2g - 4` for everyone. The nav graph gets a preset `NavConfig::agile()` with Donut `{485, 207}`, used whenever the profile is on |
| DM-Q09 | none | The 2P separation wall predicts each hero's step with **that hero's** run speed | same | Otherwise a running Donut could creep 0.37 px/tick past the wall |

**How it is wired (no new call-site parameter):**

- `Hero` gains a `motion: HeroMotion { run_speed, jump_speed }` field. `Hero::at` gives Carl's constants.
- `step_hero_driven` reads `hero.motion` where it read `RUN_SPEED` and `JUMP_SPEED`. With the legacy motion the arithmetic is the same `f32` operations, so legacy outcomes are bit-identical.
- The scene sets `donut_body.motion` from the profile when the party is built and on every restart or respawn.
- `motion` is **not** part of any legacy state hash (section 5).

### 2.1 Geometry and gate check

| Space | Check | Result |
| --- | --- | --- |
| CF02 microarena (`cf02_microarena.room.json`) | Perches top at 80 px (16 px slabs, underside 64), parapet 48, end walls full height, about 580 px of headroom, no 96 px feature | Both heroes already reach every top. Donut gains **no new standable surface** and no shortcut: the waves wake by position, not by a height gate. Her taller jump only bonks the perch underside at the same 12 px feet height as Carl (same collider). From the 80 px perch her sprite top peaks about 228 px over the floor, inside the view |
| Engineer ram (`--cf02-engineer`) | Ram box 14..33 px high | Her feet are above 33 px on jump ticks 5..49 (Carl 6..44), so a jump clears it one tick sooner and stays clear 5 ticks longer. The ram stays a lane threat: she still cannot duck it (hurtbox 0..24). Engineer design text is unchanged; its counterplay table gains a Donut row in DM-A's PR body only |
| Goblin swing | Box 0..30 px, impact ticks 26..30 | Her feet are above 30 px from jump tick 5 (same as today), so the AI dodge rule `DODGE_LATEST_TELL_TICK` 20 stays valid |
| Llama glob | 42..48 px | Unchanged: she stays under it on the ground (Q44). With the taller jump she crosses the spit line for more ticks while airborne, which is a fair cost of jumping |
| Nav graph (companion AI) | Jump and drop edges are re-simulated with her agile physics | **Required test:** on the microarena and its Engineer variant, every Carl edge state is identical between `integrated()` and `agile()`. Every Donut edge that is feasible under `integrated()` stays feasible under `agile()`, or each exception is listed with its reason: a taller jump can bonk under a low ceiling, and a faster run can overshoot a narrow anchor. New Donut-only edges are listed in the PR body. The existing `every_open_jump_and_drop_edge_replays_through_step_hero` test also runs for the agile preset with an agile body |
| Legacy, story, demo and Floor 1 corridor scenes | Not affected: the profile is off there | No check needed in v1. The default flip (section 5) re-runs this table on those kits |
| Camera lab | Has its own lab Donut (205/600) | Untouched. Its `donut_only` platforms were authored for a 171 px apex and are not a gate in the integrated scene |
| 2P separation wall | Horizontal motion past the wall comes only from `move_x`, which the wall zeroes, and from `BodyDrive` overrides (pounce, cat hop), which this spec does not scale | With DM-Q09 the agile profile adds no new way out. **Required test:** a scripted 2P run where Donut runs and makes running jumps away from a still Carl for 600 ticks; separation never exceeds `max + 0.01` px in ticks where she is not in a `BodyDrive` move. Any existing `BodyDrive` overshoot is measured and reported, not fixed here |

## 3. Evasion (DM-Q10 to DM-Q25)

### 3.1 Choice and justification

The owner asked for a **chance to evade**, so the core is a passive evasion that needs no input.

**It runs only after the biscuit.** Evasion is active iff all three hold:

- the agile profile is on;
- `dm_q11_evasion_mode` is not `off`;
- `DonutMana::unlocked()` is true.

Before the biscuit, every evadable attack resolves exactly as today, and no evasion state advances or is hashed. The unlock tick starts the reflex wait (DM-Q13), so a reflex is never ready at the moment of the unlock.

Two variants run behind one data switch, so the owner can A/B them:

- **(a) `reflex`, recommended.** A deterministic "cat reflex" that is visibly ready.
  - It arms after 480 ticks (8 s) without being hurt.
  - While armed, a tail-tip glint shows it is ready.
  - It evades the **next evadable attack instance** outright, plays an evade response, then disarms and starts the 480-tick wait again.
  - Why recommended:
    - It is fully readable: the player sees when she is covered.
    - It cannot streak.
    - It rewards play that avoids hits; it does not replace dodging.
    - It replays without any RNG.
- **(b) `roll`, the owner's literal idea.** Each evadable attack instance that would land on her rolls **80 per mille** (8 %) on a dedicated seeded stream, with an anti-streak cap of one.
  - The owner's original 150 per mille is kept as a recorded value of the same knob.
  - It is not recommended because it cannot be read in advance. A seeded proc can still feel random to the player, even when it replays exactly.

Kept from the brief's candidate list:

- **Low profile:** already exists and stays as is (Q4 20 x 24 hurtbox, Q44 under the spit line). No new code.

Dropped for v1, to keep it small:

- **A takeoff evade window:** i-frames on the first jump ticks.
- **A tell-timed evade:** both would add a second, overlapping evasion rule.
- Either can be proposed later if the playtest wants an active skill layer on top. Any active evade **command** stays ruled out by CF02-Q26 unless it gets its own input-contract review.

This fits the progression horizon: Donut is "mobile, fragile and cat-shaped", with "survivability through positioning". She is not an invulnerable turret. One evade per 8 s, or 8 %, does not change her fragility. Solo and 2P get the same rule, and so do the human and the AI.

### 3.2 Values

| ID | Knob | Value | Notes |
| --- | --- | --- | --- |
| DM-Q10 | `PartyTuning::donut_motion` (code knob, not in the JSON) | `MotionProfile::Legacy` in `PROVISIONAL_TUNING`. `MotionProfile::Agile` via `PartyTuning::with_cf02_agility()` | **The single gate.** Agile turns on DM-Q01..Q09 from the start. It turns on the evasion of DM-Q11..Q24 **only once the biscuit is unlocked** (`DonutMana::unlocked()`, owner decision, section 0). No new flag |
| DM-Q11 | `dm_q11_evasion_mode` | `reflex` (default) / `roll` / `off` | Read only under Agile and after the biscuit. `off` gives mobility without evasion, for isolating feel in the playtest |
| DM-Q12 | `dm_q12_reflex_rearm_ticks` | **480** | Owner value. Armed iff `world.tick - last_reset_tick >= 480`. No per-tick countdown, so hit-stop and pauses need no special case |
| DM-Q13 | `dm_q13_reflex_reset_on` | Fixed rule, no knob | `last_reset_tick` is set at the **unlock tick** (or at spawn for a fixture that starts unlocked), on every `HeroHurt` she takes from **any** source (non-evadable ones too: she is rattled), on every evade, on a fixture restart and on a checkpoint respawn. So she first arms 8 s after the biscuit |
| DM-Q14 | `dm_q14_roll_chance_milli` | **80**; recorded owner value **150** | `roll` only |
| DM-Q15 | `dm_q15_roll_max_streak` | **1** | `roll` only: after an evade, her next evadable attack instance always lands |
| DM-Q16 | fixed rule | Roll = `splitmix64(seed ^ DM_SALT ^ attempt) % 1000 < chance`, reusing the morale module's SplitMix64 | `seed` is the scene's recorded gameplay seed (the fixture seed, or `--cf02-seed`). `attempt` is a per-run counter of her evadable attempts. Counter-based, so no other system's RNG order changes. A capped attempt (DM-Q15) still advances `attempt` |
| DM-Q17 | evadable list | **Goblin swing** (every common goblin, every temperament); **Engineer ram**; **llama glob** | Single-target attacks with a readable tell, aimed at a body |
| DM-Q18 | never evadable | The **shore block** and any falling or collapsing terrain (CF02-Q42); **blasts and explosions**; **boss blows** (pound, charge: area or unblockable, and outside the cf02 arena); **hazards, pits and falls**; **body contacts**; any scripted or defeat-sequence damage. **Default for any future source: not evadable** until its own design opts it in | Area and environmental damage stays a positioning problem. The owner's "some attacks" |
| DM-Q19 | evade outcome | Scoped to the **attack instance**, with **no time window** | See the rules below the table |
| DM-Q20 | order | After the existing guards, before damage | Order of checks: defeat, `invulnerable_ticks > 0`, D1 M4 off-screen immunity, the box overlap, **then** the evasion check, then damage and the D1 M1 last-HP guard. A hit already stopped by a guard consumes no arm and no roll |
| DM-Q21 | presentation | Stand-ins (section 3.4) | Presentation only, never authoritative |
| DM-Q22 | events and hash | `CombatEvent::HeroEvaded { tick, hero, source, mode }` and, in `reflex` only, `CombatEvent::EvadeArmed { tick, hero }` | New fields are hashed only under Agile. Playtrace (section 3.5) |
| DM-Q23 | CLI override | `--donut-evasion reflex\|roll\|off`, accepted only with `--cf02` | For the owner's A/B. It overrides DM-Q11 for that run and is recorded |
| DM-Q24 | share target | 5 to 8 % of her evadable attempts (section 3.3) | Measured in DM-B. **Not yet measured:** this spec has no code |
| DM-Q25 | low profile | Q4 `Forgiving` 20 x 24, unchanged | Documented, no change |

**DM-Q19, what an evade does:**

- That instance does no damage, no hurt, no flinch and no knockback, and adds no `invulnerable_ticks`.
- Her running move is not cancelled.
- The same instance never hits her again: one swing, one ram, one glob.
- Any **other** instance can hit her on the next tick.
- What the instance does next:
  - **Swing:** behaves as it does today against an invulnerable Donut. If its box also overlaps the other hero, it may land on him.
  - **Ram:** continues, as against an invulnerable hero ("not hit and does not stop the ram").
  - **Glob:** passes through her and keeps flying. It does not splat.
- **Instance identity:**
  - goblin or Engineer: `(enemy index, attack start tick)`;
  - glob: `(owner, spawn tick)`.

**Human and AI.** The evasion is passive and identical for P1, P2, the solo swap and the companion. The AI does **not** read the armed state: no riskier play when armed. Its dodge rules (follower M3, companion profiles) are unchanged. That keeps the AI's decisions identical between modes, which makes the A/B comparison clean.

### 3.3 Tuning DM-Q12 and DM-Q14 to the 5 to 8 % target

The measurement lives in DM-B and is a non-golden test that prints a table.

- **Population:**
  - every `cf02_*` party fixture, run with the tuning overlaid by `with_cf02_agility()` (the overlay pattern of `donut_fixtures.rs` `with_tuning`) **and** a `DonutSetup` that starts with the biscuit unlocked and a full pool, since evasion exists only after the biscuit;
  - plus the three solo arena pilots (`cf02_arena_direct`, `cf02_arena_swap`, `eng_solo_assist`), with the same two overlays, each at seeds `0..8`.
  - With the biscuit, the AI Donut also casts Magic Missile. That is the post-biscuit play the evasion lives in, so the measurement includes it.
- **Counts:**
  - `attempts`: evadable instances that reach the evasion check on Donut;
  - `evades`.
- **Report:**
  - `evades / attempts` per mode, with `attempts`;
  - if `attempts < 40`, the PR says "insufficient sample" and adds seeds until it reaches 40.
- **Reflex:** sweep `rearm` over {360, 480, 600, 720}.
  - Keep **480** if its share is within 5 to 8 %.
  - Otherwise adopt the swept value closest to 6.5 %, and record it as an adopted default with the table.
  - Short fixtures rarely arm at 480, so the arena pilots dominate the sample. That is intended: they are what the owner plays.
- **Roll:** report the measured share at 80 and at 150. No tuning: the chance is the owner's value.

### 3.4 Presentation and art (stand-ins in v1)

Presentation is read-only from sim state and events. It never feeds back.

- **Ready tell, `reflex` only:**
  - while armed, a 2 x 2 px glint in the brightest existing Donut palette highlight sits on her tail tip, blinking 4 ticks on and 26 off;
  - on `EvadeArmed`, a 6-tick tail flick: the tail-tip pixels shift by 1 px and back.
  - The bible's tail "communicates pride, surprise and intent", so the tail is the right carrier.
- **Evade response, both modes:** on `HeroEvaded`, for 12 ticks:
  - her sprite is drawn on a cosmetic arc up to 10 px over her authoritative position, using the existing jump-rise frame;
  - two afterimages trail it at 50 % and 25 % alpha;
  - a 1-frame white streak marks where the attack passed;
  - then, if she is not in a move, the existing CF02-I `smug` overlay plays for 24 ticks.
  - Cute first, smug as payoff, no hero pose. That matches the bible's behaviour priority and the antihero tone (comedy, not invincibility).
- **Audio:** none in v1. No new audio asset; a cue is a follow-up through the audio pipeline.
- **2P readability:** both cues are on her body, not on the HUD, so each player sees them where they look.
- **Run cycle:** `anim.rs` already scales frame time with speed (`frame_ms(_, speed_milli)`). DM-C checks that her run plays slip-free at 207 px/s and that her jump phases (keyed on `vy`) still sequence at a 485 launch.
- **Production art:** a later DM-ART order, routed by the [pixel-art pipeline](../07-agents/pixel-art-pipeline.md#routing-by-difficulty) **after** the owner picks a mode. Keys:
  - `tail_ready` flick, 2 to 3 frames;
  - `evade` arch-and-twist, 4 to 5 frames, a low-to-the-ground cat leap, never a flip or superhero pose.
  - Top-tier keys and spec, medium-tier execution, independent top-tier review.

### 3.5 Determinism, hashing and playtrace

- **Deterministic.** `reflex` uses ticks only. `roll` uses a seeded, counter-keyed SplitMix64 and touches no other RNG stream. Rendering never changes an outcome.
- **State hash.** These are folded into the scene hash **only when `donut_motion` is Agile**, so Legacy hashes are untouched:
  - `last_reset_tick`;
  - the roll `attempt` counter and streak flag;
  - the evaded instance key;
  - the two new events.
- **Playtrace:**
  - The reset descriptor gains `donut_motion` (`legacy|agile`) and `evasion_mode` (`off|reflex|roll`). A missing field means `legacy` / `off`, so committed recordings still rebuild unchanged.
  - `playtrace/replay.rs` rebuilds the party tuning from these fields, as it rebuilds `with_cf02_swap` from `cf02_rules` today.
  - The biscuit state already replays through the existing reset descriptor and the guild-zone unlock event, so the evasion gate needs no new playtrace field.
  - `HeroEvaded` and `EvadeArmed` reach the journal through the recorder's existing event cursor.
  - The profile cannot change mid-session.

## 4. Impact on COMPANION-AI-02 (PR #172, sections 3.1 and 3.3)

PR #172 is not edited here. Its revision should absorb the following:

1. **Section 3.1, "Because she runs at his speed".** With the agile profile she runs at 207 against his 185.
   - The free-zone claim holds **a fortiori**: running with him, she gains 0.37 px/tick, about 22 px/s.
   - `trail_cap` (336 px) and `Z` do not change. The scroll camera in solo reads only the controlled body, and her speed stays under its 4 px/tick cap when she is the controlled one.
   - The proof test should run with both motions.
2. **Section 3.3, drift check.** `drift` is **Carl's** displacement, so it must name Carl's run speed (185/60 = 3.08 px/tick), not "RUN_SPEED". The numbers are unchanged:
   - pounce 44 ticks, about 136 px;
   - missile 16 ticks, about 49 px;
   - swat 14 ticks, about 43 px;
   - allowed landing distances about 148 / 235 / 241 px.
   - No recovery credit for her extra speed is recommended. The rule "every frame outside the view is involuntary" stays exact.
3. **Catch-up (section 3.4, case 3).**
   - While Carl keeps running, she now closes on him at 22 px/s. Before, she held the gap and relied on the 120-tick regroup.
   - While he stands, she closes at 207 px/s.
   - A 40 px overshoot is recovered in about 109 ticks of him running, so regroups should drop.
   - CA2's off-screen and regroup metrics need a fresh baseline with Agile on.
4. **Escort spacing (72 to 120 px):** her step is 3.45 px/tick, below the 8 px deadbands, so there is no new jitter. The solo swap pan (8 px/tick) still outruns her (case 4 unchanged).
5. **Nav and Perch:** CA2 must build its graph with `NavConfig::agile()` whenever the profile is on. Perch may use her 96 px-reachable anchors where a level authors them; the microarena has none.
6. **Evasion:** it is passive and adds no commitment, so the drift check is unaffected. CA2 must not make utility depend on the armed state (section 3.2, human and AI).

## 5. Gate and goldens

**Gate:**

- `PartyTuning::donut_motion` (DM-Q10) is `Legacy` in `PROVISIONAL_TUNING`.
- `with_cf02_agility()` sets `Agile`. It is applied:
  - by the live `--cf02` party (next to `with_cf02_swap()`);
  - by the playtrace rebuild when the descriptor says `agile`;
  - by the arena tunings `ARENA_SOLO` and `ARENA_COOP`. They mirror what the owner plays, so they adopt it (adopted default).
- The JSON file is read only through that gate. Its pin test is new, so adding the file moves no existing hash.
- `PartyTuning` fields that feed autoplay config hashes (`autoplay/party/scenario.rs`) are hashed for the new field only when it is not `Legacy`.

**Must stay byte-identical:**

- the 13 legacy, story, party (7), boss, `q10`, demo, CINE and autopilot goldens;
- `donut_fixtures` (8 + 7 `CF02_GOLDENS`);
- all 18 companion goldens, which include every `cf02_assist_*` fixture and `companion_v0` (`ASSIST` and `V0` are `PROVISIONAL_TUNING`-based);
- the **56** of the 61 `cf02_fixtures.rs` goldens that use `PROVISIONAL_TUNING`, including `cf02_arena_three_attempts` and `cf02_block_hurts_carl`;
- every Carl-only `eng_*` golden;
- the playtrace and autoplay goldens.

**Expected to move, in DM-A only (motion), 8 goldens:**

- in the `cf02_fixtures.rs` table (5):
  - `cf02_arena_direct` and `cf02_arena_swap` (`ARENA_SOLO`);
  - `cf02_arena_environment`, `cf02_arena_coop` and `cf02_block_hurts_partner` (`ARENA_COOP`);
- in the `engineer_tests.rs` table (3): `eng_solo_assist`, `cf02_arena_eng` and `cf02_arena_eng_cp2` (`ARENA_SOLO`).

All eight start with the biscuit locked (`DonutSetup::NONE`), so **DM-B (evasion) moves no existing golden**. The PR lists each old and new hash and states that no other golden changed.

**New goldens (DM-A, DM-B), all `dm_` prefixed:**

- motion: `dm_run_race`, `dm_jump_ledge_96`, `dm_jump_ledge_112`, `dm_2p_wall_running_jump`;
- evasion, all starting unlocked unless noted:
  - `dm_reflex_evades_swing`, `dm_reflex_not_armed`, `dm_reflex_ram_passes`, `dm_reflex_glob_passes`, `dm_reflex_block_not_evadable`;
  - `dm_roll_streak_cap`, `dm_ai_reflex_same_as_human`;
- biscuit gate:
  - `dm_evasion_locked_before_biscuit`: locked, 600 hurt-free ticks, then a goblin swing lands in both modes;
  - `dm_evasion_unlocks_at_guild`: starts locked, her guild-zone entry fires the unlock, the reflex arms 480 ticks later and evades.

The section 7 tests give the details of each.

**Live run:**

- `--cf02` alone gives the agile motion with evasion still locked (pre-biscuit Floor 1).
- `--cf02 --donut-biscuit` (or reaching the guild) adds the evasion.
- The owner's A/B command line is therefore `--cf02 --donut-biscuit --donut-evasion reflex|roll`.

**2P:**

- A human Donut gets the same motion and evasion.
- The midpoint camera follows faster when she leads, and the separation wall holds (DM-Q09, section 2.1).
- No companion runs in 2P, so the AI rules do not apply.

**Default flip:**

- Making Agile the default outside `--cf02` (demo, story, legacy) is a later, separate PR after the owner's game-feel review.
- It rebaselines those goldens explicitly and repeats the section 2.1 geometry table on the Floor 1 kits.

## 6. Risks

- **Tells stop mattering** if evasion fires too often. Mitigations: the 5 to 8 % target, one evade per arm, no window, and the CF02 stop rule ("being hit rare enough that tells stop mattering").
- **The roll feels random.** That is why `reflex` is the default. The A/B decides.
- **The faster run makes her harder to control on narrow perches** (128 and 96 px wide). The perch tests and owner feel will show it. DM-Q01 is data.
- **New Donut-only nav edges let the AI take routes the camera does not frame.** CA2's vertical band and `Z` already filter goals. The DM-A nav diff lists the new edges.

## 7. Acceptance

**Measurable tests (DM-A, DM-B):**

1. **Run ratio:** 60 ticks of full right input on flat ground gives Donut +207.0 px and Carl +185.0 px (±0.01). Ratio 1.119.
2. **Jump:**
   - a standing jump's apex is Donut 108.0 ±0.5 px and Carl 88.5 ±0.5;
   - air ticks are 55 ±1 and 50 ±1;
   - flat running-jump distance is 190 ±4 px and 154 ±4.
3. **Ledges:** Donut lands on a 96 px ledge and not on a 112 px ledge. Carl lands on 80 px and not on 96. The AI follower and the nav graph agree with the body on all four cases.
4. **Nav diff on the microarena and its Engineer variant** (section 2.1): Carl's edges are identical, and Donut's exceptions are listed.
5. **2P wall** (section 2.1): no excess separation outside `BodyDrive` moves.
6. **Evasion, reflex:**
   - an armed Donut evades one goblin swing: no `HeroHurt`, one `HeroEvaded`, health unchanged;
   - a second swing on the next tell lands;
   - with rearm 480, an evade happens only after 480 hurt-free ticks;
   - the ram continues past her and the glob keeps flying;
   - the shore block, a blast and a boss blow are never evaded;
   - an existing guard (i-frames, off-screen) consumes no arm.
7. **Evasion, roll:** with a seed that rolls two evades in a row, the second is capped. The share over 1000 synthetic attempts is 80 ±25 per mille.
8. **Biscuit gate:**
   - before the unlock, neither mode ever evades, no `EvadeArmed` or `HeroEvaded` event is logged and the evasion state is not hashed;
   - after the unlock (guild-zone entry, or a fixture or `--donut-biscuit` start) the selected mode works;
   - the reflex first arms exactly 480 ticks after the unlock tick, so a reflex is never ready at the moment of the unlock;
   - mobility is the same before and after the unlock.
9. **Human and AI parity:** the same fixture with Donut human-driven and AI-driven, at identical body positions, gives the same evade decisions.
10. **Share report** (section 3.3) in the DM-B PR body, with the adopted `rearm`.
11. **Regression:**
    - every `cf02_*`, `cf02_assist_*`, `companion_*` and `eng_*` fixture replays twice identically;
    - the list in section 5 is the only golden change;
    - every arena pilot fixture still clears its room with no new softlock (no hero stuck more than 300 ticks; the existing arena completion checks pass);
    - `cargo test --locked --all-targets` is green.
12. **Playtrace:**
    - a recorded `--cf02` session with each evasion mode verifies strict on its own build;
    - a committed pre-DM recording still rebuilds as `legacy`.

**Evidence (DM-C), at 640 x 360, 2x for the GIFs:**

- **GIF 1:** Carl and Donut side by side, running 120 ticks from the same column. She ends 44 px ahead.
- **GIF 2:** side by side, a standing jump and a running jump at a 96 px ledge. She lands; he does not.
- **GIF 3:** reflex. Glint while armed, a goblin swing, the evade arc and afterimages, smug, the glint gone, then rearmed later.
- **GIF 4:** roll mode, one evade and the next hit landing.
- A contact sheet of the stand-in cues at gameplay scale.

**Owner game-feel review stays required.** The DM PRs merge under the delegation as "merged, game-feel review pending". The owner picks `reflex` or `roll` (or `off`) and may retune any DM value in a follow-up.

## 8. Horizon fit and open questions

**Horizon fit:**

- It builds on "Donut: high/far movement and rapid positioning" and on "Never lose ... mobility-led play" ([horizon](../01-design/character-progression-horizon.md)). It also builds on the controller contract's "higher/farther cat jump" and "avoidance from Donut's agility".
- It adds no button and no new unlock. The evasion rides on the existing biscuit unlock.
- It affects solo and 2P the same way.

**Horizon-fit note (owner decision, section 0).** The Floor 01 path ("fragile agile cat -> biscuit / Magic Missile") gains one item: **at the biscuit, Donut also gains the evasion reflex.**

- Pre-biscuit she keeps "high/far movement and rapid positioning" (now the agile numbers) and stays fragile.
- Post-biscuit she gets Magic Missile and a bounded passive evasion. That is "survivability through positioning" plus one cat-reflex save per 8 s, still not an invulnerable turret.
- This is a material horizon change. Speed mode keeps docs PRs small, so the guide edit (one change-record row and the Floor 01 line) is the first task of DM-A ([work order](../07-agents/donut-mobility-01-workorder.md)), not part of this design PR. Until then this paragraph is the record.

**Owner-level questions (only these):**

1. After the playtest: `reflex` or `roll`, and keep evasion at all?
2. Should the agile profile become the default outside `--cf02` (demo and story), i.e. "all the time" in every mode? Until then it is `--cf02` only.
