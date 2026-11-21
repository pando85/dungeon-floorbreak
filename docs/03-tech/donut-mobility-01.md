# DONUT-MOBILITY-01 — Donut's agile movement and evasion

**Status:** design spec, **Revision 1** (2026-11-21), adopted defaults (orchestrator, owner directives 2026-11-19 and 2026-11-21). DM-A (the agile motion, #186) is merged and DM-B (the reflex, section 10) is implemented in its PR; DM-C (presentation) is not. Game feel stays the owner's call after merge. Revision 1 answers the independent review of PR #173 and records the owner decisions of 2026-11-21 (section 9).
**Work order:** [donut-mobility-01-workorder.md](../07-agents/donut-mobility-01-workorder.md).
**Designer tier:** top tier (a gameplay design decision with no settled spec, from a direct owner request; Revision 1 because the review found a design defect, B1). Execution: medium tier. Review: an independent top-tier agent.

## 0. Owner decisions (2026-11-21)

Recorded as given, across the request and the clarifications relayed by the orchestrator:

> "Donut must be faster all the time (human or AI). She jumps higher and further and she moves faster. That's one of their strengths. Also, she has more dodge and she is capable of avoid some attacks. [...] 15% probability for evading attacks."

The first clarifications, relayed by the orchestrator (paraphrase, not a quote):

- "15%" means a **chance to evade attacks**, not a speed value. Run speed, jump height and air distance are the designer's call.
- A 15 % passive evasion is too much when a human also dodges with her, so the evasion is bounded (one evade per arm, no invulnerability window).
- Everything is data. The same rules apply whether a human or the AI plays her.

**Evasion is a story unlock.**

- Evasion is not available from the start. Donut gains it during Floor 1, at the **biscuit event in the Tutorial Guild**, the same story unlock as Magic Missile.
- It is gated on the **existing biscuit unlock state** (`DonutMana::unlocked()`). No new flag is added. Section 5 lists what sets that state in each run.
- The faster run and the higher, longer jumps are hers **from the start**, as the horizon already says for pre-biscuit Floor 01.

**Owner decisions after the first draft (2026-11-21, relayed by the orchestrator, paraphrase):**

1. **Mode: reflex only.** "We agreed on reflex." The seeded roll mode is dropped from v1 and kept only as a rejected alternative (section 3.1).
2. **Rearm: about 10 to 15 s.** "Each 10-15s. 4s seems OP." Default **720 ticks (12 s)** without being hurt, tunable from 600 to 900 ticks. There is **no share target**: the share of evaded attacks is measured and reported, not tuned toward a number.
3. **The agile motion is Donut's default everywhere.** "Faster donut must be always. Donut is faster, dot." It is not gated to `--cf02`, and normal play has no switch that turns it off. Every golden where Donut's body moves is re-pinned on purpose (section 5). A test-only legacy motion may remain for archaeology.
4. **Unchanged:** evasion unlocks at the biscuit; mobility from the start; human and AI get the same rules.
5. After this fix round merges, DM-A, DM-B and DM-C run in order ("implement it").

**Interpretation adopted here:** "all the time" means in every state, every scene with a live party, under every controller (P1, P2, solo swap, AI companion, follower bot). Where Donut has no body of her own (the legacy trail Donut, section 1), she replays Carl's path and has no motion to change.

## 1. What exists today (verified on `origin/main` at `62686919`)

| Topic | Fact | Where |
| --- | --- | --- |
| Hero body step | One function for both heroes: `vx = move_x * RUN_SPEED` (no acceleration), jump `vy = JUMP_SPEED`, then `vy -= GRAVITY * dt`, then `y += vy * dt`. Coyote 6, jump buffer 7. No variable jump height | `src/prototype.rs` `step_hero_driven` |
| Constants | `RUN_SPEED` 185, `JUMP_SPEED` 440, `GRAVITY` 1050. Discrete apex **88.54 px**, 50 air ticks, flat air distance **154.2 px** | `src/prototype.rs` |
| Where Donut has a body | Only when the party drive is live: `set_drive` creates `donut_body` on the first live drive, and `SceneSim::step_bodies_driven` steps both bodies through `step_hero_driven` with the same constants and the same 24 x 52 collider. Live: every `PartyFixture` except the Carl-only `eng_*` ones, the combat lab, the boss scene, `--cf02` and interactive party play. **Not live:** the legacy and story walk scenarios, the demo route, CINE and the non-party autopilot, where Donut is the **trail follower** (`update_donut`) that replays Carl's path | `src/corridor.rs`, `src/party_scene.rs` `party_live()` |
| Body rebuilds | `Hero::at` builds Carl's and Donut's bodies at 7 corridor sites (build, restart, checkpoint respawn, `set_drive`) | `src/corridor.rs` |
| Per-hero movement | **None in the integrated scene.** Only the camera lab has its own Donut (run 205, jump 600, 32 x 28 body), which the integrated scene does not use | `src/camera_lab/sim.rs` |
| Open Q10 candidate | Q10 lists Donut's "faster scamper" (215 px/s) and "higher cat jump" (500 px/s, "about 119 px apex", "must also pass the HUD clearance test") as **open** rows | [party-2p-open-questions.md](../01-design/party-2p-open-questions.md) Q10, [donut-moveset-q10.md](../02-art/donut-moveset-q10.md) |
| Content built on that jump | Boss right rack: release plate on a **104 px** ledge, data-gated off (`rack.right.enabled = false`) "until Q10's higher cat jump"; no ledge collider exists while it is off. Combat lab LAB-R-Q14 (adopted 2026-11-20): a later data-only R02 variant adds a **rise-112 perch** once the cat jump lands | [floor-01-boss-design.md](../02-art/floor-01-boss-design.md), `brute_boss_f01.json`, [combat-lab-01-rooms-r00-r02.md](../01-design/combat-lab-01-rooms-r00-r02.md) |
| HUD clearance rule | The action lane starts at screen y 60 (`hud::ACTION_LANE_TOP`). At 640 x 360 the walking line is 288 px under the top (`scene_camera::walk_line_hud_px(640)`), so gameplay may reach at most **228 px** above the walking line (248 at 640 x 400). The combat lab tests heads against it (`HEAD_LIMIT` 228, Carl's collider head); the boss room uses a ceiling 224 px up | `src/hud.rs`, `src/scene_camera.rs`, `src/combat_lab/room_tests.rs`, `src/boss_scene.rs` |
| Donut's hurtbox and sprite | Hurtbox separate from her collider: `DonutHurtbox::Forgiving` **20 x 24** (Q4). Her drawn figure is about 40 px in a **72 x 44** atlas cell, inside the 24 x 52 collider. The llama's spit line (42 to 48 px) passes over a grounded Donut (CF02-Q44) | `src/party.rs`, [floor01-scene.md](floor01-scene.md) |
| "Dodge" today | No dodge verb (controller contract, CF02-Q26). The **AI** "dodge" is the follower bot's jump when a goblin tell starts within `dodge_radius_px` 40 px and no later than tell tick 20 (D1 M3). Carl's bail-out hop (Q54) and Donut's cat hop (Q55) have **no i-frames** | `src/follower.rs`, `cf02_combat_v0.json` |
| How enemy hits resolve | Each source tests its box against `CombatState::hurtbox` and calls `hurt_hero_by` / `hurt_hero`. An invulnerable hero or a guarded off-screen hero (D1 M4) is skipped. Sources: goblin swing (impact ticks 26..30, box 8..30 x 0..30), Engineer ram (box 14..24 x 14..33, `ram_on_hero` picks the nearest overlapping hero), llama glob (8 x 6 at 42..48 px), the shore block (CF02-Q42), boss blows | `src/combat_scene.rs` |
| Glob identity | `llama::Glob` has `x`, `y`, `dir`, `travelled`, `owner`, and **no spawn tick** | `src/llama.rs` |
| Follower reach | `JUMP_REACH_PX = JUMP_SPEED² / 2g - 4` = **88.19 px**, for whichever hero is the bot | `src/follower.rs` |
| Nav graph | Capability-aware: each jump or drop edge is simulated per hero, with a `HeroPhysics { jump_speed, run_speed }` per hero, driving `BodyDrive.launch` and `vx`. `NavConfig::integrated()` gives Donut Carl's physics, and navfollow always builds with it. `navfollow::approach()` sizes its last step with the shared `RUN_SPEED` | `src/companion/nav.rs`, `navfollow.rs` |
| 2P separation wall | Predicts each hero's step with the shared `RUN_SPEED` and zeroes `move_x` past `view_w - 2 * 96` (448 px at 640) | `src/corridor.rs` `apply_separation_wall` |
| Scroll camera | Pan cap `MAX_SPEED_PX` **4 px/tick** (240 px/s); vertical position fixed by the layout view | `src/scroll_camera.rs` |
| Auto camera | `--demo`, `--cf02` and the combat lab live entry. It keeps the floor at `FLOOR_SHARE` and frames each actor's top with a 56 px pad (`SAFETY_PAD_Y`); pan cap 5 px/tick | `src/camera_lab/camera_op.rs` |
| Playtrace identity | `source_fingerprint` hashes `src/`, not `assets/data`, so `stamp_manifest` adds data pins to `content_hashes` (`mana_tuning`, `cf02_tuning`). `replay.rs::live_tuning` rebuilds the party tuning from the descriptor | `src/playtrace/live.rs`, `replay.rs` |

The controller contract already states the intent: Donut's Jump is a "higher/farther cat jump", and "avoidance comes from positioning, jumps, and Donut's agility" with no dodge button ([mvp-controller-contract.md](../01-design/mvp-controller-contract.md)). This spec gives that intent numbers and adds **no input**.

## 2. Movement numbers (DM-Q01 to DM-Q09)

All values come from `assets/data/gameplay/hero_agility_v0.json`, which is new, embedded, `deny_unknown_fields`, FNV-1a pinned like `cf02_combat_v0.json`. Ticks are at 60 Hz. Every number below comes from the discrete step in the code's order (`vy = J`, then `vy -= g * dt`, then `y += vy * dt`, in `f32`), not from the continuous formula.

**Q10 reconciliation (Revision 1, B1).** DM-Q02 **adopts Q10's open Jump row (500 px/s)**, so the boss right rack and LAB-R-Q14's 112 px perch keep the margins they were designed around. DM-Q01 **supersedes Q10's open Move row** (215 px/s) with 207 px/s: nothing committed depends on 215, and +12 % keeps her visibly quicker without a different game (both are under the 240 px/s camera cap). Both Q10 rows are closed by this spec once DM-A merges.

| ID | Knob | Value | Carl (unchanged) | Why |
| --- | --- | --- | --- | --- |
| DM-Q01 | `dm_q01_run_speed` | **207 px/s** (+11.9 %, 3.45 px/tick) | 185 (3.08 px/tick) | The orchestrator's ~+12 %. A 2 s run puts her 44 px ahead of Carl. It stays under the scroll camera's 4 px/tick cap, so a controlled Donut never outruns the camera. **Invariant:** any later value must stay below 240 px/s |
| DM-Q02 | `dm_q02_jump_speed` | **500 px/s** launch (Q10's value) | 440 | Discrete apex **114.92 px** on tick 28 (continuous 119.05, Q10's "about 119"), against Carl's 88.54. She lands on a **96 px** ledge (margin 18.9), the boss rack's **104 px** ledge (10.9) and LAB-R-Q14's **112 px** perch (2.9, a narrow human route). She cannot land on 116 px or higher (8 tiles is out). Carl lands on 80 (8.5) and not on 96 |
| DM-Q03 | none (shared `GRAVITY`) | 1050, unchanged | 1050 | Same arc shape, just taller. No apex hang, no fast-fall, no variable height: the nav model (`HeroPhysics`) assumes shared gravity |
| DM-Q04 | derived, no knob | **57 air ticks**, flat air distance **196.65 px** (+27.6 %) | 50 ticks, 154.17 px | "Further" comes from the higher launch and the faster run together. Air control is Carl's rule (`vx = move_x * run`) |
| DM-Q05 | none | No acceleration or deceleration: instant velocity, as Carl | instant | It keeps the step deterministic, the nav simulation exact and the controller feel crisp. Momentum is Mongo's niche ([characters.md](../01-design/characters.md)) |
| DM-Q06 | none | Coyote 6, jump buffer 7, unchanged | same | Input semantics of the contract are untouched |
| DM-Q07 | none | **Not scaled:** swat, pounce, Magic Missile, the cat hop (Q55), Carl's bail-out, knockback, hurt motion, any `BodyDrive` launch or `vx` override. Her **collider stays 24 x 52** and her **hurtbox stays Q4 `Forgiving` 20 x 24** | n/a | v1 changes walking and the ordinary jump only |
| DM-Q08 | `dm_q08_ai_jump_margin_px` | 4, **Donut only** | Carl keeps today's constant `JUMP_REACH_PX` **88.19 px exactly** | The follower bot's "can I jump this" reach becomes per hero. Donut: her discrete apex minus the margin, **110.92 px** (computed from the JSON launch by the discrete step, not by the continuous formula, which would give 115.05 and overreach). Carl's reach does not change, so no Carl-bot decision changes (Revision 1, F2: a fix, not an intended change). The bot Donut therefore attempts 96 and 104 px ledges and not the 112 px perch, which stays a human and nav route. The nav graph gets a preset `NavConfig::agile()` with Donut `{500, 207}` and Carl unchanged; it is the preset everywhere |
| DM-Q09 | none | The 2P separation wall predicts each hero's step with **that hero's** run speed | same | Otherwise a running Donut could creep 0.37 px/tick past the wall |
| DM-Q26 | fixed rule | HUD clearance for Donut is measured on her **drawn atlas cell top** (feet + 44 px); Carl keeps the collider head (feet + 52) | n/a | The HUD covers drawn pixels. Her collider top is 8 px above her cell and never drawn. Section 2.1 |
| DM-Q27 | `ceiling_px` in the microarena scenario (new optional field, absent = none) | **228** for `CF02-ARENA` | never reached | A collision-only lane ceiling with its underside 228 px above the walking line, the boss room's pattern (section 2.1). Carl's highest head in the room is 220.5 px, so his arcs are bit-identical |

**How it is wired (Revision 1, F3: no stored motion, nothing to reset):**

- `HeroMotion { run_speed, jump_speed }` is a value, not a `Hero` field. `step_hero_motion(hero, motion, input, blocks, max_x, drive)` reads it where the step read `RUN_SPEED` and `JUMP_SPEED`. `step_hero_driven` stays as the wrapper with `HeroMotion::CARL`, so Carl's call sites, every greybox test and every Carl-only path run the same `f32` operations and stay bit-identical.
- `SceneSim::step_bodies_driven` steps Donut's body with `self.drive.tuning.donut_motion` **every tick**. No `Hero::at` site, `set_drive`, restart, respawn or swap needs to set anything, so no site can be missed.
- `PartyTuning::donut_motion` is `MotionProfile::Agile` in `PROVISIONAL_TUNING`, so every tuning built from it (`ARENA_SOLO`, `ARENA_COOP`, `ASSIST`, `V0`, the boss, the combat lab, `live_party`) is agile. No CLI flag or data switch selects legacy motion. `MotionProfile::Legacy` is constructed only by tests and by the playtrace rebuild of a recording made before DM-A (section 3.5).

### 2.1 Geometry and gate check (every space where Donut has a body)

Heights are above the walking line. "Drawn top" is DM-Q26's measure.

| Space | Check | Result |
| --- | --- | --- |
| CF02 microarena (`cf02_microarena.room.json`) | Perch slabs top at 80 px, collision underside **68 px** (the deck tile collider is 12 px tall, CF02-Q45; the JSON's 16 px `h` is the visual slab), parapet 48, end walls full height, 580 px of headroom, no feature between 88.6 and 115 px | Both heroes already reach every top, so Donut gains **no new standable surface** and no shortcut: the waves wake by position, not by a height gate. Perch to perch (gap 224 px, about 200 px of travel between supports): Carl cannot cross. **Without DM-Q27** Donut can, by a coyote jump just after walking off the left perch (simulated over every start column and jump tick: she clears the far perch's standable edge by up to about 10.4 px), which would be a new route over the parapet; **with DM-Q27** the ceiling bonk shortens her air time and she cannot (the best attempt falls 27.5 px short). Both bonk the perch underside at feet height 16 px (same collider) |
| Microarena HUD band | Jump from the 80 px perch at 640 x 360 | **Without DM-Q27:** feet reach 194.9, drawn top 238.9 (screen y 49, 11 px into the HUD band for 17 ticks), collider top 246.9, which also exceeds the auto camera's 56 px top pad. No jump value fixes this and keeps the 104 rack: the drawn top would need an apex of 104 px or less, which leaves no margin over the rack ledge. **With DM-Q27** (ceiling 228): she bonks at feet 176, drawn top 220 (y 68), collider top 228, and 228 + 56 = 284 stays inside the 288 px over the walk line, so the auto camera does not react. Carl's highest head (80 + 88.54 + 52 = 220.5) never reaches it. From the parapet (48) her collider top is 214.9: untouched |
| Engineer ram (`--cf02-engineer`) | Ram box 14..33 px high | Her feet are above 33 px on jump ticks 5..51 (Carl 6..44). The ram stays a lane threat: she still cannot duck it (hurtbox 0..24). Engineer design text is unchanged; its counterplay table gains a Donut row in DM-A's PR body, which also says the post-biscuit reflex can negate one ram per 12 s (U8) |
| Goblin swing | Box 0..30 px, impact ticks 26..30 | Her feet are above 30 px from jump tick 4 (Carl 5), so `DODGE_LATEST_TELL_TICK` 20 stays valid |
| Llama glob | 42..48 px | Unchanged on the ground (Q44). She crosses the spit line for more ticks while airborne: a fair cost of jumping |
| Floor 1 corridor (`corridor_abc_sample.json`, story run, `party_*`, `donut_*`, `companion_*` corridor fixtures) | Continuous floor, steps of at most 2 tiles (32 px), collision from deck tiles only, 640 x 360 view with the walking line at 288 | No height gate exists, so there is nothing to skip. Drawn top from a 32 px step: 190.9 (y 97). The exit is a walk-in rect. The story run has no enemies, so the guild unlock there has nothing to evade (section 5) |
| Demo hall (`demo01_hall.room.json`) | Platforms 32 px | The demo route is a trail run (section 1); interactive `--demo` / `--cf02` party play gets the agile body. Drawn top 190.9. No height gate |
| Boss room (`boss01_arena`) | Ceiling 224 px up, walls 400 px, right rack ledge absent while `rack.right.enabled = false` | Her apex from the floor (114.92, collider top 166.9) does not reach the ceiling. No new surface while the rack is off. Charge jump-over: feet above 72 px for 35 ticks (Carl 22), so with the 14-tick crossing her slack is about 21 ticks (Carl about 8). **The right rack stays off in DM-A** (section 5, "Boss right rack") |
| Combat lab R00 to R02 | Slabs rise 32 or 64; `HEAD_LIMIT` 228 on Carl's collider | From a 64 px surface her drawn top is 222.9 (inside the limit); her collider top is 230.9, which is not drawn. 230.9 + 56 = 286.9 stays inside the auto camera's 288 px. The future R02 rise-112 perch (LAB-R-Q14) needs its own HUD solution (from 112 her drawn top would be 270.9), as the lab doc already flags; it remains the lab's later data-only variant |
| Nav graph (companion AI) | Jump and drop edges are re-simulated with her agile physics | **Required test:** on the microarena (with DM-Q27), its Engineer variant, the corridor kit and the combat lab rooms, every Carl edge is identical between `integrated()` and `agile()`. Every Donut edge feasible under `integrated()` stays feasible under `agile()`, or each exception is listed with its reason (a taller jump can bonk under a low ceiling, including DM-Q27; a faster run can overshoot a narrow anchor). New Donut-only edges are listed in the PR body. `every_open_jump_and_drop_edge_replays_through_step_hero` also runs for the agile preset with an agile body |
| Scroll and auto camera | Pan caps 4 and 5 px/tick | Her run is 3.45 px/tick. In 2P the camera follows the midpoint, at most 3.45. The jump adds no horizontal speed. Vertical: the scroll camera is fixed; the auto camera is covered by the HUD rows above |
| Camera lab | Has its own lab Donut (205/600) | Untouched |
| 2P separation wall | Horizontal motion past the wall comes only from `move_x`, which the wall zeroes, and from `BodyDrive` overrides, which this spec does not scale | With DM-Q09 the agile motion adds no new way out. **Required test:** a scripted 2P run where Donut runs and makes running jumps away from a still Carl for 600 ticks; separation never exceeds `max + 0.01` px outside `BodyDrive` moves |

## 3. Evasion (DM-Q10 to DM-Q25)

### 3.1 Choice and justification

The owner asked for a **chance to evade**, and chose the readable variant: a passive **cat reflex** that needs no input.

**It runs only after the biscuit.** Evasion is active iff both hold:

- `dm_q11_evasion_mode` is `reflex` (the default; `off` exists only to isolate feel in a playtest or a test);
- `DonutMana::unlocked()` is true.

Before the biscuit, every evadable attack resolves exactly as today, and no evasion state advances or is hashed. The unlock tick starts the reflex wait (DM-Q13), so a reflex is never ready at the moment of the unlock.

**The reflex:**

- It arms after **720 ticks (12 s)** without being hurt.
- While armed, a tail-tip glint shows it is ready.
- It evades the **next evadable attack instance** outright, plays an evade response, then disarms and starts the wait again.
- It is readable (the player sees when she is covered), cannot streak, rewards play that avoids hits without replacing dodging, and replays with no RNG.

**Rejected alternatives (recorded, not built):**

- **Seeded roll** (the owner's literal "15 %"): each evadable instance would roll on a dedicated seeded stream. Rejected by the owner on 2026-11-21 in favour of the reflex: a seeded proc cannot be read in advance and feels random even when it replays exactly. Its knobs (former DM-Q14 to Q16) are retired.
- **A takeoff evade window** (i-frames on the first jump ticks) and **a tell-timed evade**: each would add a second, overlapping evasion rule. Any active evade **command** stays ruled out by CF02-Q26 unless it gets its own input-contract review.

Kept from the brief's list: **low profile**, which already exists (Q4 20 x 24 hurtbox, Q44 under the spit line). No new code.

This fits the progression horizon: Donut is "mobile, fragile and cat-shaped", with "survivability through positioning". One save per 12 s hurt-free does not change her fragility. Solo and 2P get the same rule, and so do the human and the AI.

### 3.2 Values

| ID | Knob | Value | Notes |
| --- | --- | --- | --- |
| DM-Q10 | gate | **No motion gate** (Revision 1). `PartyTuning::donut_motion` is `Agile` in `PROVISIONAL_TUNING` | Evasion's only gate is the biscuit (`DonutMana::unlocked()`) plus DM-Q11 |
| DM-Q11 | `dm_q11_evasion_mode` | `reflex` (default) / `off` | Data only; no CLI override. `roll` is not a valid value |
| DM-Q12 | `dm_q12_reflex_rearm_ticks` | **720** (owner value), valid range **600..=900**; the loader rejects values outside it | Armed iff `tick - last_reset_tick >= rearm`. No per-tick countdown |
| DM-Q13 | reset rule, no knob | See below | |
| DM-Q14, Q15, Q16 | retired | n/a | The roll mode's chance, streak cap and RNG rule. IDs kept so references stay unambiguous |
| DM-Q17 | evadable list | **Goblin swing** (every common goblin, every temperament); **Engineer ram**; **llama glob** | Single-target attacks with a readable tell, aimed at a body |
| DM-Q18 | never evadable | The **shore block** and any falling or collapsing terrain (CF02-Q42); **blasts and explosions**; **boss blows**; **hazards, pits and falls**; **body contacts**; any scripted or defeat-sequence damage. **Default for any future source: not evadable** until its own design opts it in | Area and environmental damage stays a positioning problem |
| DM-Q19 | evade outcome | Scoped to the **attack instance**, with **no time window** | See below the table; the ram check and the instance key are refined in section 10 (DM-Q19/U3-R1, DM-Q19-R1) |
| DM-Q20 | order | After the existing guards, before damage | Defeat, `invulnerable_ticks > 0`, D1 M4 off-screen immunity, the box overlap, **then** the evasion check, then damage and the D1 M1 last-HP guard. A hit already stopped by a guard consumes no arm |
| DM-Q21 | presentation | Stand-ins (section 3.4) | Never authoritative |
| DM-Q22 | events and hash | `CombatEvent::HeroEvaded { tick, hero, source }` and `CombatEvent::EvadeArmed { tick, hero }` | Hash rule in section 3.5; `HeroEvaded` also carries `kind` (section 10, DM-Q22-R1) |
| DM-Q23 | retired | n/a | The `--donut-evasion` CLI override existed only for the roll A/B |
| DM-Q24 | share | **No target.** The share is measured and reported (section 3.3) | Owner decision 2 |
| DM-Q25 | low profile | Q4 `Forgiving` 20 x 24, unchanged | Documented, no change |

**DM-Q13, the reset rule (Revision 1, U2).** The tick base is the combat scene's **attempt-local tick**, the tick carried by `CombatEvent` (a restart brings it back to 0). It counts hit-stop ticks, so hit-stop and pauses need no special case. `last_reset_tick` is set:

- at the **unlock tick** (the guild-zone grant), or to 0 for a run or fixture that starts unlocked;
- on every `HeroHurt` she takes from **any** source, evadable or not (she is rattled);
- on every evade;
- on a fixture restart (to 0, the new attempt's start) and on a checkpoint respawn (to the respawn tick).

A **solo swap does not reset it**: the swap changes who drives her body, not the body. `EvadeArmed` fires on the first tick where the armed condition holds.

**DM-Q19, what an evade does:**

- That instance does no damage, no hurt, no flinch and no knockback, and adds no `invulnerable_ticks`.
- Her running move is not cancelled.
- The same instance never hits her again: one swing, one ram, one glob.
- Any **other** instance can hit her on the next tick.
- What the instance does next:
  - **Swing:** behaves as it does today against an invulnerable Donut. If its box also overlaps the other hero, it may land on him.
  - **Ram (U3):** the evasion check runs **inside** `ram_on_hero`'s candidate loop. An evaded Donut is skipped exactly like an invulnerable one, so Carl behind her can be chosen on the same tick. The ram continues.
  - **Glob:** passes through her and keeps flying. It does not splat.
- **Instance identity:**
  - goblin or Engineer: `(enemy index, attack start tick)`; DM-B stores the tell-start tick per enemy because hit-stop freezes `attack_tick` (section 10, DM-Q19-R1);
  - glob: `(owner, spawn_tick)`. `llama::Glob` gains a `spawn_tick: u32` field (Revision 1, F8). It is **not** folded into the state hash, because it is derivable from the spit events, so adding it moves no golden.

**Human and AI.** The evasion is passive and identical for P1, P2, the solo swap, the companion and the follower bot. The AI does **not** read the armed state: no riskier play when armed. Its dodge rules (follower M3, companion profiles) are unchanged.

### 3.3 Measuring the share (no target)

The measurement lives in DM-B and is a non-golden test that prints a table.

- **Population:**
  - every `cf02_*` party fixture, overlaid (the `with_tuning` pattern of `donut_fixtures.rs`) with a `DonutSetup` that starts with the biscuit unlocked and a full pool, since evasion exists only after the biscuit. The motion is already agile everywhere;
  - plus the three solo arena pilots (`cf02_arena_direct`, `cf02_arena_swap`, `eng_solo_assist`), each at seeds `0..8`.
  - With the biscuit, the AI Donut also casts Magic Missile. That is the post-biscuit play the evasion lives in.
- **Counts:** `attempts` (evadable instances that reach the evasion check on Donut), `evades`, and, per arena pilot, the evades **per wave** and **per attempt**, plus how many of the evades took the **first** evadable hit of a wave (U1).
- **Report:** `evades / attempts` with `attempts`, at rearm 600, 720 and 900. If `attempts < 40`, the PR says "insufficient sample" and adds seeds until it reaches 40.
- **No tuning toward a share.** 720 stays the default whatever the table shows; the owner reads the table and may retune inside 600..900.
- **Known effect (U1):** walking between waves is calm, so a reflex that arms after 12 s may take the opener of many waves. The table makes that visible, and the owner handoff says it in one sentence.

### 3.4 Presentation and art (stand-ins in v1)

Presentation is read-only from sim state and events. It never feeds back.

- **Ready tell:**
  - while armed, a 2 x 2 px glint in the brightest existing Donut palette highlight sits on her tail tip, blinking 4 ticks on and 26 off;
  - on `EvadeArmed`, a 6-tick tail flick: the tail-tip pixels shift by 1 px and back.
  - The bible's tail "communicates pride, surprise and intent", so the tail is the right carrier.
- **Evade response:** on `HeroEvaded`, for 12 ticks:
  - her sprite is drawn on a cosmetic arc up to 10 px over her authoritative position, using the existing jump-rise frame;
  - two afterimages trail it at 50 % and 25 % alpha;
  - a 1-frame white streak marks where the attack passed;
  - then, if she is not in a move, the existing CF02-I `smug` overlay plays for 24 ticks.
  - Cute first, smug as payoff, no hero pose. That matches the bible's behaviour priority and the antihero tone.
- **Audio:** none in v1.
- **2P readability:** both cues are on her body, not on the HUD.
- **Run cycle and jump phases:** `anim.rs` scales frame time with speed (`frame_ms(_, speed_milli)`). DM-C checks that her run plays slip-free at 207 px/s and that her jump phases (keyed on `vy`) still sequence at a 500 launch and with a DM-Q27 ceiling bonk.
- **Production art:** a later DM-ART order, routed by the [pixel-art pipeline](../07-agents/pixel-art-pipeline.md#routing-by-difficulty): `tail_ready` flick (2 to 3 frames) and `evade` arch-and-twist (4 to 5 frames, a low-to-the-ground cat leap, never a flip or superhero pose). Top-tier keys and spec, medium-tier execution, independent top-tier review. The mode is now chosen, so it can follow DM-C.

### 3.5 Determinism, hashing and playtrace

- **Deterministic.** The reflex uses ticks only and touches no RNG stream. Rendering never changes an outcome.
- **State hash (DM-B).** The reflex state (`last_reset_tick`, the evaded instance key) and the two new events are folded into the scene hash **from the first `EvadeArmed` of the run onward**. Before that, the state is a pure function of already-hashed facts (the unlock, hurts, restarts and respawns). So DM-B moves exactly the goldens in which she arms at least once, and a run that never arms keeps its DM-A hash.
- **Motion hash (DM-A).** The body positions already carry the motion. Hashes that fold `PartyTuning` (`autoplay/party/scenario.rs` `initial_hash`, `party_tuning_hash`) fold `donut_motion` **only when it is not the default** (`Agile`), so those 7 pins do not move.
- **Playtrace (Revision 1, F5):**
  - `stamp_manifest` adds `content_hashes["agility_tuning"]`, the new JSON's FNV pin. It covers DM-Q11 and DM-Q12, so a changed rearm is a content mismatch, not a silent drift.
  - `feature_flags["donut_motion"] = "agile"` on every new recording.
  - The reset descriptor gains `donut_motion` with `#[serde(default, skip_serializing_if = ...)]`: **absent means `legacy`**, the motion of every recording made before DM-A. `replay.rs::live_tuning` rebuilds the party tuning with that motion, as it rebuilds `with_cf02_swap` from `cf02_rules`. New recordings write `agile`. This is the only non-test path that constructs `MotionProfile::Legacy`, and it is replay archaeology, not play.
  - No `evasion_mode` descriptor field: there is no CLI override, and the content hash pins the data.
  - The biscuit state already replays through the existing reset descriptor and the guild-zone unlock event.
  - `HeroEvaded` and `EvadeArmed` reach the journal through the recorder's existing event cursor.
  - DM-A states which playtrace goldens move (the manifest of a new recording gains the flag and the content hash) and keeps the existing synthetic-manifest goldens byte-identical where the field is absent.

## 4. Impact on COMPANION-AI-02 (PR #172)

PR #172 is not edited here. Its Revision 1 already depends on DM-A and records agile motion as Donut's default everywhere. Points for CA2 to absorb or confirm:

1. **Numbers.** The jump is now **500 px/s, apex 114.92 px** (not 485 / 108). CA2's vertical band (anchors at most 176 px above the leader's feet) still holds: her highest new surface is 112 px.
2. **Free zone (3.1).** She runs at 207 against his 185 and gains 0.37 px/tick. `trail_cap` (336 px) and `Z` do not change. The proof runs with both motions.
3. **Drift (3.3).** `drift` is Carl's displacement at 185/60 = 3.08 px/tick; pounce about 136 px, missile 49, swat 43; landing distances about 148 / 235 / 241 px. No credit for her speed.
4. **Catch-up (3.4).** While Carl runs she closes at 22 px/s; while he stands, at 207 px/s. A 40 px gap closes in about 109 ticks of him running. CA2's off-screen and regroup baselines are measured with the agile motion.
5. **Escort spacing.** Her step is 3.45 px/tick, below the follower's `FOLLOW_SLACK_PX` 8 (`src/follower.rs`) and below CA2-Q09's 8 px drift slack, so there is no new jitter (U5: the draft's "8 px deadbands" meant these). The solo swap pan (8 px/tick) still outruns her.
6. **Nav, Perch and `approach()` (F4).** The nav preset is `NavConfig::agile()` everywhere. `navfollow::approach()` and `steer()` take the **body's run speed** as a parameter (DM-A owns this change); CA2's go-to entry point reuses `approach()` after DM-A merges, so CA2-A and CA2-B rebase on DM-A.
7. **Evasion.** Passive, no commitment, so the drift check is unaffected. CA2 must not make utility depend on the armed state.

## 5. Scope and goldens (Revision 1)

**No motion gate.** The agile motion is Donut's default in every scene where she has a body (section 1). There is no `--cf02`-only gate, no `with_cf02_agility()`, no CLI flag and no data switch for it. A trail Donut (legacy and story walk scenarios, the demo route, CINE, the non-party autopilot) replays Carl's path, so those runs are unchanged by construction; DM-A proves it rather than assuming it.

**Evasion gate.** The biscuit state, `DonutMana::unlocked()`, set by exactly these:

- the **story run's** guild-zone event (the live corridor run's `DonutSetup` with `GUILD_PLACEHOLDER_ZONE`). The story run has no enemies, so the reflex has nothing to evade there; this is the story event only;
- `--donut-biscuit` (with or without `--cf02`) and `--boss`, which start unlocked (`DonutSetup::unlocked(8)`);
- a fixture's `DonutSetup`.

Under `--cf02` the run is a demo run, `donut_setup()` returns `DonutSetup::NONE` and there is **no guild zone**, so under `--cf02` only `--donut-biscuit` unlocks evasion (Revision 1, F6).

**Live runs:**

- `--cf02`: agile motion, evasion locked (pre-biscuit Floor 1).
- `--cf02 --donut-biscuit`: agile motion plus the reflex. This is the owner's playtest line.
- Story run: agile motion wherever the party is live; the guild unlock fires, with nothing to evade yet.
- The owner's "agile from the start, then earned at the guild" flow is exercised end to end only by the `dm_evasion_unlocks_at_guild` fixture until a scene has both the guild and enemies. The owner handoff says so.

**Boss right rack.** The boss design's default (§174) enables the rack in the PR that lands the jump. This spec **keeps `rack.right.enabled = false` in DM-A** (adopted default) so DM-A does not change boss combat beyond Donut's motion; enabling it is a follow-up order (DM-RACK). That order re-derives the boss doc's continuous numbers with the discrete ones: Carl 15.5 px under the 104 ledge, Donut 10.9 over it with an AI reach of 110.92 (6.9 over), landing on the ledge on jump tick 37, and the P3 rise/dwell/fall timing; it then un-pends `boss01_rack_donut`. Flagged here, not silently changed.

**DM-A re-pins every golden where Donut's body moves.** The PR body carries a **full before/after table**: every pinned value in the repository with its old value, its new value and one of "moved: Donut walks or jumps", "moved: other cause (named)" or "unchanged". No assertion is loosened: a test whose exact value moved is re-pinned to the new exact value, and a test that asserted "Donut moves like Carl" is replaced by its agile equivalent. Expected scope (from a code read on `origin/main`, to be confirmed by running):

| Pins | Count | Expectation |
| --- | --- | --- |
| `party_scene.rs` `GOLDENS` and its copy `autoplay/party/tests.rs` `PARTY_GOLDENS` | 7 + 7 | Move where Donut walks or the bot follows; `duo_separation_wall`, `donut_hurt_iframes` and `donut_defeat_party` may stay |
| `donut_fixtures.rs` `GOLDENS` (q10) and its copy `Q10_GOLDENS` | 8 + 7 | `swat_interrupt`, `swat_guard`, `unlock_persists` move; the pounce, dry-press and regen ones may stay |
| `donut_fixtures.rs` `CF02_GOLDENS` | 7 | All move |
| `companion_fixtures.rs` `GOLDENS` (every `companion_*`, `cf02_assist_*`, `companion_v0`) | 18 | All move |
| `cf02_fixtures.rs` `GOLDENS` (every `cf02_*`) | 61 | Most move (the solo follower bot walks and jumps); fixtures where Carl never moves and no pilot runs may stay |
| `engineer_tests.rs` `ENGINEER_GOLDENS` | 29 | `eng_solo_assist`, `cf02_arena_eng`, `cf02_arena_eng_cp2`, `eng_point_interrupt_swat`, `eng_2p_partner_in_lane` move; `eng_2p_targets_nearest` (idle Donut) and the 24 Carl-only fixtures must stay. The `eng_fixture!` macro keeps its tunings; nothing is held out of the agile motion |
| `boss_scene.rs` `GOLDENS` | 13 | All move (live party) |
| `combat_lab/room_tests.rs` `GOLDENS` | 4 x 2 | The launch hashes stay; the 600-tick hashes move (the bot Donut) |
| `autoplay/party/scenario.rs` `initial_hash`, `party_tuning_hash` | 6 + 1 | Stay (section 3.5) |
| Trail and Carl-only pins: `prototype.rs` greybox goldens, `corridor.rs` `GOLDEN_WALK_RIGHT_400`, `combat_scene.rs` `EXPECTED_HASHES` and the walk_right pins, `death_present.rs`, `autoplay/tests.rs` and `autoplay/scenario.rs`, `demo.rs` `ROUTE_HASH`, `tests/cine_fixtures.rs`, `companion/nav.rs` `GOLDEN_BUILT` / `GOLDEN_AFTER_WALL`, RNG and format pins | all | Stay. The table lists them as "unchanged" with the reason (trail Donut or no Donut) |
| Microarena kit (DM-Q27 adds a solid) | any | A pin that hashes the solid list moves for that reason; the table names it "other cause: DM-Q27". Carl's arcs are bit-identical |

**Committed replays and evidence DM-A rewrites explicitly** (the folders are append-only by convention; DM-A overwrites each moved file on purpose and says so):

- `docs/evidence/cf02/replays/headless_<fixture>.txt` (97 files), `docs/evidence/companion-ai/` (18), `docs/evidence/party-2p/` (7, plus its `SHA256SUMS` and the hash lines in its README), `docs/evidence/boss-f01/headless_*` (13), `docs/evidence/donut-q10-mana/` (8, plus `SHA256SUMS`), and `docs/evidence/audience-01/headless_audience_probe.txt` if it moves. All are reproduced by `scripts/ci_floor01_combat.sh headless`. The hash lines in `docs/evidence/cf02/README.md` follow.
- **Pinned non-golden tests** that depend on Donut's movement, re-measured and re-pinned (never loosened): `the_solo_assist_and_live_rooms_make_progress_and_log_the_ai_counts`, `cf02_arena_environment_plan_runs_under_the_default_assistant`, the `eng_point_interrupt_swat` and `eng_2p_partner_in_lane` trace tests, `in_two_player_he_points_at_the_nearer_hero_and_the_first_hero_overlapped_is_hit`, `room_tests::a6_donut_steps_like_carl_in_the_scene` (replaced by its agile equivalent), the `donut_fixtures.rs` tick pins (the guild unlock at 55, the llama swat and bolt ticks, the light-guard ticks, the bail-out landing), `q7_the_wall_lets_a_hero_move_back_toward_the_other`, `solo_swap_moves_control_and_the_swapped_hero_keeps_following`, `in_two_players_the_gate_waits_for_p2`, the `cf02_arena_*` completion, cinematic-count and swap-count tests, and the `companion_fixtures.rs` A13, solo-defeat and C11 tests. DM-A greps for further ones and lists them.
- **Evidence scripts with hard-coded tick windows:** `scripts/make_cf02_i_evidence.sh` (22 windows), `scripts/cf02_arena_captures.sh` (6), `scripts/cf02_render_match.sh` (last ticks), `scripts/ci_floor01_render_smoke.sh` (party, q10 and boss capture ticks; it runs in CI) and `scripts/combat_lab_evidence.sh`. DM-A re-measures each window so it still frames the intended state, and regenerates the committed state files the scripts produce.

**DM-B (evasion)** moves exactly the goldens in which Donut arms at least once (section 3.5): runs that start unlocked or reach the guild and then stay 720 ticks hurt-free. DM-B lists each with its old and new hash.

**New goldens (DM-A, DM-B), all `dm_` prefixed:**

- motion: `dm_run_race`, `dm_jump_ledge_96`, `dm_jump_ledge_112`, `dm_jump_ledge_116_fails`, `dm_2p_wall_running_jump`, `dm_microarena_perch_ceiling`;
- evasion, starting unlocked unless noted: `dm_reflex_evades_swing`, `dm_reflex_not_armed`, `dm_reflex_ram_passes`, `dm_reflex_glob_passes`, `dm_reflex_block_not_evadable`, `dm_ai_reflex_same_as_human`;
- biscuit gate: `dm_evasion_locked_before_biscuit` (locked, 900 hurt-free ticks, then a goblin swing lands) and `dm_evasion_unlocks_at_guild` (starts locked, her guild-zone entry fires the unlock, the reflex arms 720 ticks later and evades).

**2P:** a human Donut gets the same motion and evasion; the midpoint camera follows faster when she leads, and the separation wall holds (DM-Q09).

## 6. Risks

- **Tells stop mattering** if the reflex fires too often. Mitigations: one evade per 12 s hurt-free, no window, the measured share and per-wave report, and the CF02 stop rule ("being hit rare enough that tells stop mattering").
- **The opener of each wave may be free** (U1). Visible in the DM-B table; the owner may lengthen the rearm inside 600..900.
- **The faster run makes her harder to control on narrow perches** (128 and 96 px wide). DM-Q01 is data.
- **Human Donut with AI Carl in solo** (U4): the bot Carl (185) loses 0.37 px/tick while she runs and does not follow her onto 96 px ledges, which is intended for gates. Now that agility is the default, DM-A measures Carl's off-screen ticks and regroup count on the corridor kit with a scripted running Donut and reports them.
- **New Donut-only nav edges** let the AI take routes the camera does not frame. CA2's vertical band and `Z` filter goals; the DM-A nav diff lists the new edges.
- **The lane ceiling** (DM-Q27) truncates her perch jump by 18.9 px in one room. It is invisible (it sits inside the HUD band). It also removes the perch-to-perch coyote jump (section 2.1). If the owner dislikes the bonk, the alternative is a lower perch, which changes Carl's CF02-Q45 route; recorded, not adopted.
- **The 112 px margin is 2.9 px** (about 9 ticks above 112). Fine for a deliberate human route; the lab's later R02 variant measures its takeoff window with its room tests.

## 7. Acceptance

**Measurable tests (DM-A, DM-B):**

1. **Run ratio:** 60 ticks of full right input on flat ground gives Donut +207.0 px and Carl +185.0 px (±0.01).
2. **Jump:**
   - a standing jump's apex is Donut 114.92 ±0.05 px (tick 28) and Carl 88.54 ±0.05;
   - air ticks are 57 and 50;
   - flat running-jump distance is 196.65 ±0.05 px and 154.17 ±0.05.
3. **Ledges:** Donut lands on 96, 104 and 112 px ledges and not on 116; Carl lands on 80 and not on 96. The nav graph agrees with the body on all of them. The follower bot attempts 96 and 104 for Donut and not 112, and its reach for Carl is still exactly 88.19.
4. **Motion is always applied (F3):** Donut's body moves with the agile motion after the live drive starts (`set_drive`), after a restart, after a checkpoint respawn, after a solo swap and in 2P. A test asserts that no live or fixture path constructs `MotionProfile::Legacy`.
5. **Nav diff** on the microarena, its Engineer variant, the corridor kit and the combat lab rooms (section 2.1): Carl's edges are identical, and Donut's exceptions are listed. In the microarena no Donut edge joins the two perches without touching the floor.
6. **`approach()` (F4):** with the body's run speed, the last step lands within `ARRIVE_EPS_PX` 0.6 of the target at both 185 and 207 px/s, with no overshoot.
7. **2P wall** (section 2.1): no excess separation outside `BodyDrive` moves.
8. **HUD (DM-Q26, DM-Q27):** using the real walk line (`walk_line_hud_px`) at 640 x 360 and 640 x 400, Donut's highest drawn top from every standable surface stays at or under the limit in the microarena (with the ceiling), the corridor kit, the demo hall, the boss room and the combat lab rooms; and the auto camera's zoom does not change during any of those jumps. Carl's highest head in the microarena is unchanged (220.5).
9. **Evasion:**
   - an armed Donut evades one goblin swing: no `HeroHurt`, one `HeroEvaded`, health unchanged;
   - a second swing on the next tell lands;
   - with rearm 720, an evade happens only after 720 hurt-free ticks; the loader rejects 599 and 901;
   - the ram skips her inside its candidate loop and can hit Carl behind her on the same tick; the glob keeps flying;
   - the shore block, a blast and a boss blow are never evaded;
   - an existing guard (i-frames, off-screen) consumes no arm;
   - a solo swap does not reset the wait; a restart and a respawn do.
10. **Biscuit gate:**
    - before the unlock, the reflex never evades, no `EvadeArmed` or `HeroEvaded` event is logged and the evasion state is not hashed;
    - after the unlock (guild-zone entry, or a fixture, `--donut-biscuit` or `--boss` start) the reflex works;
    - it first arms exactly 720 ticks after the unlock tick;
    - mobility is the same before and after the unlock.
11. **Human and AI parity:** the same fixture with Donut human-driven and AI-driven, at identical body positions, gives the same evade decisions.
12. **Share report** (section 3.3) in the DM-B PR body, at 600, 720 and 900, with the per-wave and per-attempt counts.
13. **Regression:**
    - every fixture replays twice identically;
    - the before/after table of section 5 accounts for every pinned value;
    - every arena pilot still clears its room with no new softlock (no hero stuck more than 300 ticks);
    - `cargo test --locked --all-targets` is green and `scripts/ci_floor01_combat.sh headless` reproduces every committed replay.
14. **Playtrace:** a recorded `--cf02 --donut-biscuit` session verifies strict on its own build and carries `donut_motion = agile` and the `agility_tuning` hash; a session recorded before DM-A rebuilds as `legacy`.

**Evidence (DM-C), at 640 x 360, 2x for the GIFs:**

- **GIF 1:** Carl and Donut side by side, running 120 ticks from the same column. She ends 44 px ahead.
- **GIF 2:** side by side, a standing jump and a running jump at a 96 px ledge. She lands; he does not.
- **GIF 3:** the reflex. Glint while armed, a goblin swing, the evade arc and afterimages, smug, the glint gone, then rearmed later.
- **GIF 4:** the microarena perch jump with the HUD drawn, showing her under the HUD band.
- A contact sheet of the stand-in cues at gameplay scale.

**Owner game-feel review stays required.** The DM PRs merge under the delegation as "merged, game-feel review pending". The owner may retune DM-Q01, DM-Q02, DM-Q12 (inside 600..900) or set DM-Q11 to `off` in a follow-up.

## 8. Horizon fit and open questions

**Horizon fit:**

- It builds on "Donut: high/far movement and rapid positioning" and on "Never lose ... mobility-led play" ([horizon](../01-design/character-progression-horizon.md)), and on the controller contract's "higher/farther cat jump" and "avoidance from Donut's agility".
- It adds no button and no new unlock. The evasion rides on the existing biscuit unlock.
- It affects solo and 2P the same way.

**Horizon-fit note.** The Floor 01 path ("fragile agile cat -> biscuit / Magic Missile") gains one item: **at the biscuit, Donut also gains the evasion reflex** (one save per 12 s hurt-free). Pre-biscuit she keeps "high/far movement and rapid positioning", now with the agile numbers everywhere, and stays fragile. This is a material horizon change; the guide edit (one change-record row and the Floor 01 line) is the first task of DM-A ([work order](../07-agents/donut-mobility-01-workorder.md)). Until then this paragraph is the record.

**Owner-level questions (only these):**

1. After the playtest: keep the reflex at 720 ticks, retune it inside 600..900, or turn it off?
2. Does the lane ceiling in the microarena (DM-Q27) read acceptably, or should the perches drop instead?

## 9. Revision 1 (2026-11-21)

Answers the independent top-tier review of PR #173 (verdict: changes requested, one blocking design defect) and records the owner decisions of 2026-11-21 (section 0). Code facts re-verified on `origin/main` `62686919`. Numbers recomputed in `f32` in the code's step order.

**Owner decisions recorded (2026-11-21):** reflex only, the roll a rejected alternative; rearm 720 ticks (12 s) hurt-free, tunable 600..900, no share target; the agile motion is Donut's default everywhere with no off switch in normal play, and every golden where Donut moves is re-pinned on purpose; evasion stays gated on the biscuit; DM-A, DM-B and DM-C run after this PR merges.

| Finding | Fix | Where |
| --- | --- | --- |
| **B1** DM-Q02 conflicted with Q10's 500 px/s cat jump, the boss rack (104) and LAB-R-Q14 (112) | Option (a): DM-Q02 = 500 (Q10's value). Apex 114.92, 57 air ticks, 196.65 px; margins 10.9 over 104 and 2.9 over 112; AI reach 110.92. Q10's Jump row adopted, its Move row (215) superseded by DM-Q01. Q10's HUD condition checked (F7). Rack stays off in DM-A; DM-RACK follow-up re-derives the boss numbers | 1, 2, 2.1, 5 |
| Scope change (owner) | No motion gate: `Agile` in `PROVISIONAL_TUNING`; legacy motion only in tests and the playtrace rebuild of pre-DM recordings; DM-A re-pins every golden where Donut moves with a full before/after table; trail runs proven unchanged | 0, 2, 5 |
| **F1** moved-golden list incomplete; replays, pinned tests and scripts missing | Full expected scope table, committed replay folders, pinned tests and tick-window scripts listed; DM-A owns them and never loosens an assertion | 5, work order |
| **F2** legacy follower reach would change (88.19 to 84.5) | Fixed, not changed: Carl keeps `JUMP_REACH_PX` 88.19 exactly; Donut uses her discrete apex minus 4 (110.92) | 2 (DM-Q08), 7.3 |
| **F3** motion on `Hero` fragile; `set_drive` and other `Hero::at` sites missed | Motion is not stored: `step_bodies_driven` applies `donut_motion` every tick; test after live start, restart, respawn, swap and in 2P | 2, 7.4 |
| **F4** `navfollow::approach()` assumed Carl's run speed | `approach()` and `steer()` take the body's run speed; DM-A owns it; CA2 rebases on DM-A | 4, 7.6, work order |
| **F5** playtrace content identity | `agility_tuning` content hash, `donut_motion` feature flag, descriptor field absent = legacy with `skip_serializing_if`; no `evasion_mode` field since the CLI is gone | 3.5 |
| **F6** guild unlock wrongly claimed under `--cf02` | The unlock is the existing biscuit state; under `--cf02` only `--donut-biscuit`; the guild is the story event only; the earned-at-guild flow is fixture-only, said in the handoff | 5, work order |
| **F7** HUD clearance unchecked | DM-Q26 (drawn top for Donut) and DM-Q27 (microarena lane ceiling 228); per-space table; test with the real walk line and the auto camera | 2, 2.1, 7.8 |
| **F8** glob had no spawn tick | `Glob::spawn_tick`, not hashed; `src/llama.rs` owned by DM-B | 3.2, work order |
| U1 opener-free reflex | Per-wave and per-attempt report, first-hit count, handoff sentence | 3.3, 6 |
| U2 reset rules | Attempt-local tick base; swap does not reset; restart to 0; respawn to its tick | 3.2 |
| U3 ram placement | Evasion inside the `ram_on_hero` candidate loop | 3.2, 7.9 |
| U4 human Donut, AI Carl | Measured on the corridor kit in DM-A | 6 |
| U5 "8 px deadbands" | Cited as `FOLLOW_SLACK_PX` 8 and CA2-Q09 | 4 |
| U6 perch underside | Collision underside 68 px (deck collider), bonk at feet 16 px | 2.1 |
| U7 combat lab goldens | Listed; their 600-tick hashes move | 5 |
| U8 Engineer counterplay | The reflex can negate one ram per 12 s; noted in the counterplay row | 2.1 |

## 10. DM-B implementation record (reflex evasion)

Implemented as specified, with the adopted refinements below (each is a default the owner may change in a follow-up). Code: `src/agility.rs` (`Reflex`, `EvadeSource`, `EvadeKey`, `ReflexProfile`, all pure), `src/combat_scene/evade.rs` (the scene hooks), the three call sites in `src/combat_scene.rs` (`swing_on`, `ram_on_hero`, `step_globs`), `CombatEvent::{HeroEvaded, EvadeArmed}` in `src/combat.rs`, `Glob::spawn_tick` in `src/llama.rs`. Tests: `src/agility_reflex_tests.rs`, `src/combat_scene/evade/tests.rs`, `agility.rs` unit tests. Evidence: [`docs/evidence/donut-mobility-01/`](../evidence/donut-mobility-01/README.md).

| ID | Adopted | Why |
| --- | --- | --- |
| DM-Q17, DM-Q18 (owner, 2026-11-21) | The evadable list is an **allow-list in code**: the `EvadeSource` variants (`GoblinSwing`, `EngineerRam`, `LlamaGlob`) are the only sources that can reach `CombatState::evade_check`. A source with no variant is **never** evaded, so a new enemy attack (a rush, a body contact, a hazard, a blast, a Hoarder blow) opts in only by adding a variant and a call site. A test counts the call sites (3) and checks that the existing guards sit before each | The owner's decision; no data key, so `hero_agility_v0.json` and its `agility_tuning` pin do not move |
| DM-Q13 with a body or rush hit | A `HeroHurt` she takes from **any** source resets the wait, including one the reflex could not evade (shore block, boss blow, a later body contact or rush). Tested with the real `hurt_hero` path on an armed Donut: the hurt lands, nothing is evaded, the reflex disarms and re-arms exactly 720 ticks after that hurt | The spec rule, implemented exactly |
| DM-Q19 identity | A goblin swing and an Engineer ram are keyed `(enemy index, attack start tick)` where the start tick is the tick the tell began (`GoblinTell`), stored per enemy at the tell and not hashed. Hit-stop freezes `attack_tick`, so `tick - attack_tick` would drift mid-swing; the stored tell tick does not. A glob is `(owner, spawn tick)` | A stable instance key |
| U3, the ram | The evasion check runs for the hero the ram **would hit** (the nearest overlapped candidate), in the candidate loop. An evaded Donut is skipped and Carl behind her is chosen on the same tick (tested: both on tick 44). When Carl is the nearer candidate Donut is not the target and her arm is not spent | The spec's literal "inside the loop" would spend an arm on an attack that was never going to hit her; DM-Q20 says a hit that is not a hit consumes no arm |
| Reset base | `Reflex::reset` takes the attempt-local tick of the combat scene (hit-stop counts); the unlock tick comes from the logged `UnlockGranted`, every hurt from the logged `HeroHurt` of Donut, a fixture or defeat restart clears the whole state (the hash gate included), a checkpoint respawn resets at its tick | DM-Q13 |
| Hash | The reflex state (`last_reset_tick`, `armed`, the evaded key) is mixed only after the first `EvadeArmed` of the attempt; the two events carry their own tags. A run that never arms hashes exactly as in DM-A | Spec section 3.5 |
| Test tuning | `PartyTuning::donut_reflex` (`Shipped`, `Off`, `Rearm(n)`) is folded into the tuning hash only when it is not `Shipped`. `Off` and `Rearm` are constructed only by tests (a source scan enforces it); the data file stays the only play-time source of DM-Q11 and DM-Q12 | Spec F6 and the share measurement |
| `HeroEvaded` | `{ tick, hero, source, kind }`: `source` is the attacker's target id as in `HeroHurt`; `kind` names the `EvadeSource` | One field more than DM-Q22, for the log and the share table |

**Measured share (section 3.3, no target).** See the evidence README. With the biscuit at tick 0, the shipped rooms never let her arm before the first thing that reaches her: the only evadable attack that reaches the AI Donut in the microarena is the Engineer's ram, first on tick 427 to 487 of the attempt, inside the 600 ticks of the shortest rearm. So the share is 0 of 3 (fixtures) and 0 of 42 (the Engineer pilot over 48 seeds) at 600, 720 and 900; armed at tick 0 it is 3 of 3 and 42 of 42, every pilot attempt the opener of the second wave (U1). Few attempts reach her at all (the fixture rows are an insufficient sample), so the table says where the reflex sits, not how often it fires in play. 720 stays the default whatever the table shows.

**Rulings (PR #199 review, adopted defaults).** DM-Q19/U3-R1: the ram checks only the hero it would hit. DM-Q19-R1: swing and ram instances are keyed by the tell-start tick; every path into `GoblinMode::Attack` must report `started_tell` (a comment and a `debug_assert` at the stamp and the two call sites guard it). DM-Q22-R1: `HeroEvaded` carries `kind`. The measured share covers the AI-controlled Donut only; a human-Donut population is a DM-C or follow-up item.

**Deviations and gaps (stated, not hidden).**

1. Spec goldens not pinned: `dm_reflex_ram_passes` and `dm_reflex_glob_passes` are test-only (they need `cf02_rules` and the arena kit, keyed by name prefix) with events asserted; `dm_reflex_block_not_evadable` does not exist, its role is played by `the_shore_block_and_a_boss_blow_are_never_evaded` on overlaid real fixtures (the hurt source and the armed state are asserted); `DM_AI_REFLEX_SAME_AS_HUMAN` exists but is neither registered nor pinned, its test compares the two runs tick for tick instead.
2. Blast: no hero blast damage exists before CP-B1, so "a blast is never evaded" is vacuous today. A blast is non-evadable by construction (no `EvadeSource`, the call-site scan). CP-B1 adds the armed-Donut-in-a-blast test.
3. Off-screen guard (D1 M4): covered structurally by the guard-order scan; there is no behavioural test yet. Only the i-frame guard is exercised (`a_guard_consumes_no_arm`).
