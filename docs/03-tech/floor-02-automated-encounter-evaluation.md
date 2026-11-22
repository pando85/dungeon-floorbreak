# F02-QA-01 — Automated encounter evaluation, fairness probes and return-from-trip evidence

**Status:** design/work order, NOT a new implemented evaluation harness, trained agent, measured result or accepted fun verdict. 2026-11-22. Implements the evaluation goals of [F02 composition](../01-design/floor-02-encounter-composition-and-difficulty.md) by reusing the repository's actual [AUTOPLAY-01 contract](autoplay-01-automated-gameplay-evaluation.md), [PLAYTRACE-01](playtrace-01-session-capture.md), [current test hierarchy](testing.md), existing room fixtures, simulation hashes and F01 PT-GATE scorecards. The implementing local agent must audit the CURRENT main contracts before naming an actual CLI argument or assuming a policy/party mode already exists.

## 0. Why automated evaluation matters, and its limits

We want local agents to identify **broken, easy-to-exploit, unfair and brittle** encounters before the human returns, so that the owner spends valuable controller time judging impact, humor, readable physics and fun, not repeatedly discovering softlocks.

Machines can check:
- scene reachability and state/progression invariants;
- input ownership, authorized actions, replay, reset and real collision causality;
- whether a few *specific* baseline policies can complete an encounter;
- how many threats overlap, active damage windows, actual recovery, defeats, direct-vs-prop route viability, time/damage/action profiles;
- whether a stationary-spam baseline or perfect script trivially solves a targeted room;
- companion action counts, actual landed damage and camera/hazard evidence where exposed.

Machines **cannot** prove encounter fun, human-readable silhouettes, correct emotional pacing, enjoyable difficulty or competent local two-gamepad play from success metrics. Neither simulated performance nor PPO training policy should become a substitute for a real player.

## 1. Do not duplicate or silently extend simulation

Runtime authority is the deterministic Rust combat/party/scene simulation. AUTOPLAY-01 already specifies a Bevy-free env and existing scripted policies; PLAYTRACE-01 owns continuous real human session capture. **No second physics engine, Python-side collision model, simplified training platformer, client-side simulation, separate untracked RNG, alternate damage source or ad-hoc file-only checker that ignores real gameplay.**

When an old design doc says a workstream is pending, **audit main and merged PRs**: do not treat a 2026-11-20 historical status as live today. The Floor 02 runner may require a narrow extension of the scene/floor authoring contract and party-mode environment; it must preserve already-tested AUTOPLAY and PLAYTRACE versions and prove migration compatibility.

The currently observed AUTOPLAY technical spec notes that some early fixture seeds are hash labels, not genuine gameplay randomness; be precise: **varying a seed is not an independent scenario unless it changes real authored enemy timing/placement or a verified RNG draw**. Prefer explicit different scenario configurations and held-out encounter layouts to fake seed sweeps.

## 2. Small purposeful bot reference suite

| Policy ID (candidate) | High-level behavior | Primary coverage | What it cannot establish |
| --- | --- | --- | --- |
| F02-P01 baseline_approach | run toward nearest visible threat, attack with actually legal inputs | direct reachability and HP/time sanity | human threat recognition, competence |
| F02-P02 reactive_survivor | react to observable committed tells, back out of active lane, punish recovery | possibility of surviving a fair telegraphed pattern | real human reaction latency |
| F02-P03 stationary_magic | Donut stands in initial space; fires earned primary when legal | S01/S02 anti-camping vulnerability; detect magic turret exploit | all skilled Donut players or normal difficulty |
| F02-P04 mobile_magic | Donut moves between cover/flanks, casts at revealed targets and pounces opportunistically | benefit of mobility vs standing still | perfect routing on every map |
| F02-P05 carl_direct | only Carl physical, basic primary and legitimate jump; no optional props | direct offense and small boss contact reachability | explosive/coordination spectacle |
| F02-P06 prop_opportunist | legal actions toward **visible** jug/sign/bin and real collision outcomes | authentic environmental opportunity and missed-route recovery | a human discovering the prop affordance |
| F02-P07 risk_averse | stays behind existing cover, only attacks on safe recovery | permanent safe-spot exploit or excessive boss downtime | fun or humor |
| F02-P08 duo_fixed_slots | two simultaneous per-hero policies, **when actual PartyFixtureSim + party input path support it** | state ownership, coop contribution and camera/state consistency | actual human 2-pad enjoyment |
| F02-P09 party_solo_swap | solo policy with realistic AI companion, switch only if actual gameplay mode supports it | inactive partner action/hazard, real switching | human use of orders or preferred hero |
| F02-P10 random_legal | sample only available observations/legal actions with fixed policy RNG | robustness/replay/reset, obvious invalid hitboxes | meaningful balance |

**Policy access policy:** the observation must be player-like, bounded and free of privileged future attack direction, hidden enemy cooldowns, latent hitbox flags, omniscient line of sight or exact boss/loot RNG. One bot should not get the inner game state while another uses screen-space observations and be compared as though skill alone differs. Log supported fields and observation version.

If a policy cannot be implemented using the real current action mapping without bypassing input ownership or inventing previously unearned specials, mark UNSUPPORTED; it is not an acceptance failure. A scripted **input trace with foreknowledge** is useful for determinism but must be labeled scripted probe, never reactive player success.

## 3. Evaluation fixtures / experiments per chapter

| Probe | Scenario variant, no hidden powers | Counterfactual comparison | Failure / remediation |
| --- | --- | --- | --- |
| EQ-A1 | S01 single Horror | basic direct and reactive | attack cannot be read/reached with one hero -> adjust recovery/geometry |
| EQ-A2 | S01 Horror + rusher | reactive vs stationary Donut | standing still safe in all authored variants -> add readable approach pressure, not magic resistance |
| EQ-B1 | S02 Rep + Laminak | direct vs opportunistic physical | alarm adds unbounded foes or cannot be interrupted -> state/event fix |
| EQ-B2 | S02 jug chain | no-prop vs 1-prop vs 2-actions vs missed prop | one missile detonates all with no second cause -> invalid prop ignition |
| EQ-B3 | S03 Krakaren Clone | direct-only Carl, direct-only Donut, then prop | boss only beatable via script -> restore fair core recovery |
| EQ-C1 | S04 grubs/conveyor/Vespa | zero corpses / several corpses / no bin / bin hit | >cap actor count, no exit, Carl unable to reach Vespa |
| EQ-C2 | S05 wheelchair convoy two stops | one lead hero each, 2 slots when supported, ignore prop | convoy offscreen loss / AI partner nonparticipant / indefinite stuck |
| EQ-D1 | S06 Rage refuge | no attack vs futile attack, begin from each safe checkpoint | Rage HP altered, warning absent, sanctuary can hurt hero |
| EQ-D2 | S07 escape | launcher triggered / missed / ignored; Donut blink not equipped | no-cart direct route blocked, AI companion follows Rage, early Floor03 |
| EQ-E1 | S08 kennel | all cages closed vs optional release; rider dismount | mandatory dog ally, wrong actor ownership, invisible mounted rider |
| EQ-E2 | S09 Ralph | direct Carl, direct Donut, optional dog, no dog | Ralph unreachable by kicks, unfair tells, forced Carl finisher |
| EQ-E3 | full S00->S09 story | story loadout vs deliberately invalid future-grant configuration | early Mongo, class/skills leakage, duplicate checkpoint unlock |

Use **at least two genuinely different authored encounter configurations** when investigating balance; changing an inert fixture seed does not count. For expensive scenarios, start with one golden canonical run and two local targeted variants, not a vast uniform random sweep.

## 4. Metrics: always preserve denominators and causes

Every room report has metadata:
- game commit SHA, scenario / content manifest ID and SHA, actions/observations/rewads version, build profile, supported input mode, policy ID and version, companion profile, effective RNG provenance, actual reset configuration, repeat index;
- number of attempted episodes and number completed / failed / timeout / unsupported. Display the raw N, not only percentages;
- elapsed **sim ticks**, wall-clock training throughput separately, and per-attempt restart count;
- hero active attacks, landed attacks, misses, successful physical contact launches, interrupt count, hits received, HP damage, death cause, knockback into enemy/prop, damage ownership and genuine two-character assists;
- actual environmental activation and chain cause (root input -> actor impulse -> prop -> damaging collision -> defeat), max simultaneous active attack warnings, projectile attacks, hazard overlap, recovery-window exposure and if reachable escape volume existed;
- companion attacks and landed effective hits **while AI-controlled**; do not count mere animation or irrelevant body-block as contribution;
- seconds where the player has no meaningful active target or safe actionable interaction; label separately cinematic/safe chapter breathing time so silence does not appear as a combat bug;
- critical route passability after each authored destruction state, camera/hero/tell bounds at native 640x360 and 640x400, if render or viewport diagnostics actually supported;
- Playtrace verification status with hash checkpoints and precise mismatch; never call unverified trace reproducible.

**Use exact numerators and context.** E.g. a success rate of 100% on two scripts is N=2/2, not proof an encounter is easy. Do not attribute missing external visibility/hardware as a negative game result. A successful static scripted attack may be impossible for a non-omniscient policy.

## 5. Fairness heuristics: measurable diagnostics, not fabricated constants

### Automatic HARD failure

Any of the following fails an integrated encounter independent of human fun:
1. unsupported story ability required on critical path;
2. direct fight (ordinary or conventional boss) physically impossible without optional world prop;
3. authoritative hit before the warning can be perceived on screen (where visibility diagnostics exist) or inside protected sanctuary;
4. two hazards are simultaneously active with **no reachable legal space/counter** at known player movement envelope;
5. dead or detached enemy still emits attacks under invalid authority; duplicate collision kill/reward;
6. checkpoint resets to an in-progress lethal state, corrupts score, drops party, loses story gate or creates permanent softlock;
7. Rage direct damage reduces ordinary boss HP / awards normal kill XP; Rage-only stairwell prematurely transports heroes to Floor 03;
8. all credible AI/solo party travel paths fail because companion blocks exit or won't follow the protected route;
9. changing render timing alters physics events, or replay in fresh process mismatches a stored strict hash.

### Investigate / YELLOW, not auto-fail

These indicate a possible balance/UX problem that needs a human or deeper probe:
- stationary_magic completes **every** tested S01/S02 mixed pressure fixture without damage or repositioning;
- both reactive and basic policies die immediately after a new enemy reveal, despite intended safety route;
- no-prop direct and spectacle routes take identical time / achieve no different meaningful collision outcome;
- high number of whiffed Carl hits because the target floats too high, or Donut AI has almost no effective attacks;
- 90% of a boss fight spends time waiting for tiny core windows, not doing meaningful combat;
- one hero deals virtually every actual boss hit while the second is fully excluded;
- high warning overlap or no-progress duration around convoy, Rage or Ralph;
- an environmental chain kills all targets automatically from one ordinary input in every observed run.

Do NOT hardcode one global ratio or "fun score". Record observational patterns, propose reversible data changes and retest fixed reference scenes. Model-driven difficulty adjustment during a live playthrough is out of scope.

## 6. Difficulty measurements and confidence

Suggested **comparative**, not absolute, stats:
- paired outcomes on same authored source with no-prop vs one-prop vs full chain;
- median / distribution of elapsed ticks and health lost across each *actually distinct* scenario variant;
- stationary-vs-mobile vulnerability and how often a reactive bot takes damage during a clearly safe response;
- player-threat interaction density: landed actions and visible enemy commitments per active combat minute;
- proportion of launched-body collisions that cause another authoritative hit, not merely VFX;
- companion offensive presence, survival and at least one observable useful assist/defense in eligible scenes.

For success rates, report N, number of independent scene variants, a confidence interval when reasonable, and repeat variance; keep training/dev vs held-out rooms separate. An optimizer trained on one exact booth layout and replayed on the same layout is **not generalization evidence**.

### Human baseline contract

Before travel, owner plays Floor 01 and writes GAMEPLAY-BASELINE-V1 with actual pad findings. When returning, owner plays representative F02 scenes:
- **S02 direct vs jug chain:** is direct combat independently satisfying and the chain discoverable?
- **S03:** do Carl and Donut both have a clear active contribution?
- **S05:** do protecting people and defending at two stops create tension rather than babysitting?
- **S07:** does the player understand Rage cannot be fought and why the stair trap works?
- **S09:** does tiny Ralph read, stay fair and remain genuinely funny at speed?

Keep human ratings separate from bot metrics. Use PLAYTRACE to record each real attempt where support exists (with correct privacy, default and consent policies). A machine report can recommend what the owner tests first but never auto-approve game feel.

## 7. Anti-overfitting and negative controls

A robust lab can detect **its own false confidence**:
- Deliberately remove the Horror warning in a private test fixture and verify the visibility assertion **fails**, restoring the original data afterward.
- Remove all optional jugs and make sure S02 and Krakaren still pass direct routes; restore jugs to show real new events, not cosmetic explosion.
- Force the foreman to complete alarm and verify max one reinforcement; then interrupt with actual landed contact and observe no spawn.
- Force the Rage decoy to miss and verify fallback path. Make the stairwell hero trigger intentionally illegal and ensure progression validation catches S07->Floor03.
- Use a tiny Ralph body with an incorrect oversized physical box in a private negative fixture and verify collision-size diagnostics distinguish it from legal attack cues.
- Corrupt one Playtrace action or event and assert strict verifier fails; never silently repair corrupt evidence.
- Force 2P divergence outside camera bounds and verify the diagnostic reports NOT COVERED instead of incorrectly declaring normal 2P fun.

Negative fixtures may remain as automated tests if safe, but production scene files should not carry intentionally broken values.

## 8. Evidence and merge/report contract

Each candidate PR includes a machine-readable per-scene scorecard and one short human-readable summary:
- all exact commands that were **actually executed**, commit and manifest hashes;
- passed / failed / unsupported tests with the N/denominators; no invented runtimes;
- direct and optional player-authored routes, including actual causal event log;
- one real game-speed capture when render available, 1x sprite contact sheets, honest camera visibility status;
- proof of deterministic retry and no story progression leak;
- a **separate pending owner gameplay section** with 1–3 important questions, not a vague "polish later".

**Proposed batch order during travel**:
- QA-0 audit AUTOPLAY and PLAYTRACE current real APIs, update candidate policy support matrix, no gameplay edits;
- QA-1 run first full reference card for S01+S02+Krakaren pure scenes, add negative checks;
- QA-2 expand to corpse and convoy; measure AI participation and no-progress;
- QA-3 run Rage and Ralph path/box/fairness probes, especially Exile != Floor03;
- QA-4 E2E one true story reachable trace plus artifact index; real two-pad and enjoyment remain pending.

**No global difficulty tuning and no new ML training objective is implied**. Training a learned policy is optional research only after the baseline bot suite proves authoritative parity and scene generalization. Never let a policy that observes hidden boss states decide that human difficulty is appropriate.

## 9. Acceptance IDs for autonomous agents

EVAL-01 current code interfaces and version compatibility audited;
EVAL-02 scripted reference policies see only allowed observation, button-edge semantics preserved;
EVAL-03 distinct variants have real behavior/placement differences, seeds not falsely credited;
EVAL-04 per-scene pass/fail/retry includes cause IDs and raw counts;
EVAL-05 direct and optional mechanics can be isolated using authored fixture data;
EVAL-06 every hard fairness assertion has at least one intentionally failing negative fixture;
EVAL-07 companion contribution not misattributed to either human;
EVAL-08 floor-specific state and Rage Exile remain on Floor02 until post-Ralph+Mongo;
EVAL-09 one replay in a fresh Rust process strict verifies and corrupted replay fails;
EVAL-10 real 640x360 and 640x400 game-speed render capture where tools support;
EVAL-11 preserved Floor01 deterministic golden fixtures and no hot-file concurrent writes;
EVAL-12 human 2-controller and real game-feel acceptance clearly NOT RUN before actual owner review.
