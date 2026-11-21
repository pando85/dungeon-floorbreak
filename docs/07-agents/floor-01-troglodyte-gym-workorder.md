# FLOOR01-TROG-RUNUP-01 — Local agent implementation work order

**Purpose:** implement the **troglodyte territory / gym approach**, with one recognizable Pygmy pounce, one Basher smash, one Virtuoso tongue lane, a short meaningful mixed fight and a protected transition into the **existing** Juicer boss. Detailed owner-requested design: [The House of Broken Weights](../01-design/floor-01-troglodyte-gym-approach.md).

**Status:** this PR creates **design and execution instructions only**; neither troglodyte runtime AI nor the playable lead-in is claimed present. One of the essential gates is *player-run* game-feel review after real Bevy implementation. Do not treat selected art or native animation assets as proof the enemy exists in gameplay.

## Top-line implementation priorities

1. **Get the standalone new enemy types legitimately running in the existing engine**, using current `ENEMY-SLOT` ownership/adapter from merged `docs/07-agents/f01-playtest-integration-plan.md` (#182). Do **not** clone the Juicer boss controller or bypass existing combat hit/damage/event/Playtrace domains.
2. **Prove one fight per species is interesting**, with correct native animations and *actual* physical collision/tells. This precedes placing several in a single room.
3. **Stage one two-threat mixed battle** with a simple optional environment response. Basher is the heavy force, Virtuoso threatens a midheight lane, the optional bench changes **real** LoS/cover if truly contacted. A Pygmy reinforcement is conditional, not mandatory.
4. **Deliver a real gym threshold** and checkpoint to the existing `boss_f01` scene. Do not rewrite that boss, enable the unused right rack or prematurely grant Juicer rewards.
5. **Run autonomous exploit probes and actual human co-op sessions** and report results, including measured Juicer fight duration with/without the left weight rack.

## Dependencies and shared-file safety

- `AGENTS.md`, the current open PR list, recently merged branches and #182's shared hot-file queue are mandatory reading before changing code.
- **Character art:** `TROG-01`, `TROG-02`, `TROG-03` already supply source pixel frames at 40/66/66px; keep their original source/export pipeline. Do not crop the large concept painting, "upscale the boss into a Basher", invent magical effects or silently change selected silhouettes.
- **New enemy engine slot:** the runtime may not have a registered Pygmy/Basher/Virtuoso yet. If ENEMY-SLOT is not merged, make a narrow independent core/test fixture, and explicitly block the live-room spawn hookup; do not add a giant new shared enum/adapter without coordination.
- **Props:** BOIL/#176 owns shared contact/prop/hazard domain. The weak bench's **optional physical break** is gated on that existing interface; static cover is the first version. Never implement bespoke unbounded gym-rigidbody simulation.
- **Story/camera:** follow CP/DP story ledger and current DM/CA2 companion implementations. Dynamite is already story-earned by this point only if actual ledger confirms. Light on Your Feet acquired in restaurant. No gauntlet/Puddle Jumper before actual Juicer defeat. New post-boss ability playground belongs to #187; camp-to-gym entrance follows #189/#187.
- **Juicer:** `--scene boss_f01` uses its own 640px screen, independent gate, left rack and three phases. Don't move its boss into a generic `f01_late` room unless the integration owner explicitly approves. Boss tuning must be a separate review/PR.
- **Animation vs gameplay:** 24/36/30 tick troglodyte tells are authored readability animation defaults, *not* proof the AI damage/timing code exists. Keep attack profile data-tuned and validate at 1×, 640×360/400 and 1080p output.

## Phased agent deliveries

| Deliverable | What ships | Evidence / release gate |
| --- | --- | --- |
| **TR-A0 audit** | Repository status, current enemy-slot and level-entry availability, true sprite manifests/pivots, boss compatibility and source chronology | Short branch/ownership note and executable current test command |
| **TR-B1 Pygmy** | Valid `coil→committed leap→land recovery` AI profile and tests; single enemy fixture | Real collision/contact, no frame tracking, honest tell and one damage token, solo Carl/Donut |
| **TR-B2 Basher** | Valid `heavy tell→smash→20-tick spent` profile, no copy of boss charge | Safe jump/step to punish, actual hurtbox, no magic immunity or giant HP inflation |
| **TR-B3 Virtuoso** | Valid `feint→tongue lash→retract` profile with true +0/+32/+64 hurtbox matrix | Correct long tongue line, Donut low hurtbox truth, no poison or fake homing |
| **TR-C level** | Authored T0..T3 rightward scene on true level loader, visual tells, source art, no dead end | Real Bevy playable run with all three isolated, safe 2P camera, short rest |
| **TR-D mixed** | Basher + Virtuoso staged fight; static bench, optional physical break via merged prop API | Direct clear and physical outcome both reachable; never more than two initial threats |
| **TR-E gym threshold** | Reception, turnstile, checkpoint and scene-relaunch handoff to current `boss_f01` | No duplicate boss, no stage actor leak, 1P/2P party safe, story capability ledger correct |
| **TR-F evaluation** | TROG-01..17 tests, stationary exploit analysis, authentic fixed-step traces and native footage | Actual commands and measured outcomes attached; owner game-feel review explicitly pending |

Do not skip game-feel evidence by checking only that the art frame renders; it is far easier to make a player read a 36-tick animation than to make the corresponding enemy enjoyable to fight.

## Mandatory autonomous testing by the implementing agent

The user previously required an agent to **test, diagnose and iterate on its own** for [the Infernal Corral](floor-01-corral-autonomous-evaluation.md); apply the same principle here at an appropriately small scope. This means:
- Implement/extend the **real Rust party/scene** headless fixture for each new troglodyte and the T4 mixed battle, not toy scripted damage or fabricated Playtrace events. **Run those tests locally.**
- Version a **stationary Donut legal-press-edge** policy (real 16-tick missile cast and 8-charge mana), a **mobile Donut** policy (observable dodge/counter movement), **Carl melee** and one **genuine two-controller-input** probe.
- Test Pygmy leap vs moving/standing hero; Basher tell/impact/recover; Virtuoso tongue from ground and raised platforms; two-tell simultaneous warning fairness; mixed room direct and optional bench contact.
- **Search several actual reachable shot lines** and report the best consistent stationary no-risk clear. Preserve legitimate ground-Donut height advantage and first power fantasy; resolve bad dominant strategy through real enemy movement/recovery, encounter placement and sightlines, **not** magic nerfs, invisible armour or forced damage.
- Use **the current** Playtrace/party tooling only where implemented. If AUTOPLAY party evaluate/replay flags remain unavailable, write a narrowly scoped Rust evaluator/fixture and document valid commands actually executed. Every unimplemented fixture is `BLOCKED`/`NOT RUN`, never `PASS`.
- Produce run rows in JSON/CSV or equivalent stable machine-readable data, meaningful before/after comparisons if tuning changed, deterministic same-input event/hash check, and 1× game captures. Separate **companion AI damage contribution** from directly controlled Donut/Carl.
- Compare **Juicer solo and 2P time-to-clear with and without the default left rack**. If his existing 36HP combat drags, report a separate design/balance concern for owner decision, but do not modify `brute_boss_f01.json` as an unreviewed side effect.

## Specific failure conditions to detect

- Pygmy teleports to the target, follows mid-leap, hits before readable tell, or pounces into a late/offscreen P2.
- Basher permanently stunlocks the player, stays immune to normal attacks despite supposed recovery, or is trivialized by static magic from every natural position.
- Virtuoso's tongue is **visually high** but damages grounded Donut through an invisible enlarged box, hits through true solid cover, or attacks Carl exclusively.
- T4 activates two simultaneous lethal attacks without a safe response before either actor is visible; co-op gets a free hit because the camera favors the leader.
- Bench break is proximity/timer-scripted rather than actual smash/impact, seals the only forward route, or is registered more than once per token.
- Boss scene entrance causes unearned gauntlet/blink, reopens previous combat, permanently separates the party, respawns enemies without seed reset, or changes existing boss fixture hashes unexpectedly.
- "Autoplay passed" means only a test compiled or one deterministic seed no-op ended; **require a genuine successful pilot and actual failed exploit search**.

## PR completion report template

- **Actual new gameplay code / scene entry:** path, branch head, runtime rule registration
- **Data values & decisions:** original animation vs implemented AI state, all changed source art and provenance
- **Evidence:** completed TROG-01..17 matrix; commands, pass/fail/blocked, JSON/CSV, fixed input traces and replay hashes
- **Gamepad:** actual Carl/Donut solo and dual-human session, native camera/tell/animation validation and encounter pacing
- **Juicer handoff:** boss relaunch, checkpoint/story ledger receipts, fight duration with and without left rack; no default boss rewrite
- **Outstanding:** unavailable slots/props, high-vs-low hurtbox risks, game-feel issues, independent review and green final CI
- **Owner review:** explicitly human judgment pending until the actual full lead-in and Juicer transition are played

**Definition of done:** Carl and Donut can genuinely play from Meadow Lark's exit through isolated Pygmy/Basher/Virtuoso fights, a direct and optional-physical mixed gym encounter, and arrive at the **existing** Juicer boss scene with fair 1P/2P controls and correct story state. The track must not be marked implemented just because this PR's documents merge.
