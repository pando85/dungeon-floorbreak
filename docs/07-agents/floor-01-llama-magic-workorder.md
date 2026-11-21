# FLOOR01-LLAMA-MAGIC-01 — Local agent work order (Infernal Corral v0.2)

**Task:** Implement the newly approved **Infernal Corral**, not the superseded "Throat Choir". The updated [main design and MAG-01..16 acceptance matrix](../01-design/floor-01-llama-magic-baptism.md) are authoritative. This PR still initially comprises design/work-order documents only; actual playable completion requires code, tests, 1P/2P runs, native footage and user game-feel review.

## Why the redesign exists — read before coding

The original dual-llama encounter could be trivialized by Donut **standing still and repeatedly pressing Magic Missile**. A llama has 3 HP; Magic Missile is 1 damage with 240px range vs approximately 200px llama stand-off; glob can pass over grounded Donut, while missile hits can interrupt the llama's existing spit tell.

**Owner decision (2026-11-21):** preserve a deliberately simple first magical win, then make the main encounter **a mobile, readable close/mid-range fight**. Add one **short committed, telegraphed llama rush**, real repositioning and one optional destructible corral partition; **no** magic resistance, HP padding, extra spells, homing lava, mandatory precise co-op, new input, random danger or gratuitous bullet hell.

The *first llama* can be killed by just three magic bolts; only the **M2 corral** is subject to the anti-stationary-spam acceptance gate.

## First audit: what actually works on the branch?

1. Follow `AGENTS.md`, current orchestrator/workorders and shared-file ownership. Inspect current `src/llama.rs`, `src/combat_scene.rs`, `src/combat_scene/donut.rs`, `src/party.rs`, `src/prototype.rs`, live camera, room loader, active companion/Donut mobility/CF02 branches, and story unlock integration.
2. Pin the **real baseline** with a reproducible headless experiment: Donut at representative M2-like x positions, post-biscuit, pressing Primary every valid cast/recharge window while never moving. Report HP, mana, damage/kill source, number of interrupted tells and the true behavior of lava glob over Donut's hurtbox.
3. **Known potential blocker:** standard llama target selection may still be tied to Carl's x; post-Guild story mode may not have active Donut→llama hit targets outside CF02-enabled runs. Audit and coordinate narrow fixes with active owners; never make a level-only targeting cheat or mark solo Donut as supported if she cannot actually fight.
4. Check native LLAMA-01 art. There is **no already approved charge animation** in the current 17-frame llama readability set. Prototype with plainly temporary tell/debug geometry if necessary; production charge animation needs native pixel-art extension and explicit review.
5. Verify camera and jump/hurtbox heights (+0/+32/+64 shelves), actual horizontal 240px Magic Missile trajectory and current lava-glob projectile collision. The lower Donut hurtbox advantage is intentional; don't rewrite it.

## Implementation sequence

| Stage | Deliverable | Validation gate |
| --- | --- | --- |
| **C-A** | Playable Guild→M0 first bolt→M1 isolated llama→M3 restaurant-safe transition, using true earned biscuit flag and no rush in early first-lama section | MAG-01..03, real native Bevy first-cast clip, both Carl and Donut direct attacks genuinely work |
| **C-B** | **One short llama rush** in isolated fixture: explicit `RushTell`, `RushCommitted`, `RushRecovery` (names adaptable), original llama's spit unchanged, low fair collision, one hit token | MAG-04..06, deterministic no-homing collision, warning and punish-window evidence |
| **C-C** | Replace old dual-ranged-patrol M2 with **Infernal Corral** geometry: continuous ground lane, partial real cover, two staggered llamas, one active ranged threat and rush-capable close pressure | MAG-07,08,11,12; **STATIONARY-DONUT** sweep at meaningful spots and actual two-pad fairness |
| **C-D** | Optional one fragile timber partition that blocks actual projectiles and can be broken by a **real llama charge collision**; allow direct kill if never broken | MAG-09,15; only if merged material/prop interfaces allow, else clearly gated follow-up |
| **C-E** | Native-charge tell and recovery art, meaningful hoof scrape/ground dust separated from orange throat-glob and Donut magenta cast, native camera and fast-food quiet transition | Native 640×360 and 640×400 footage at 1x, 1080p scaled capture, reduced effects legibility |
| **C-F** | Playtrace, restart, state hashes, party mode, perf, independent review, human signoff | All MAG-01..16; no unsupported "implemented" assertions |

**Implementation code ownership:** do not duplicate the existing Donut mobility or **12s reflex** work, companion autonomous movement, Donut/Carl progression, opening Dozer, Boiler/Hoarder, camera or replay architecture. Minimal specialized llama attack-state extension is fine; general enemy/combat changes belong behind narrow review/coordination and goldens. Existing llama fixtures must not silently gain a rush by default; opt in only in the authored corral fixture or reviewed character progression context.

## Rush attack design parameters: proposals, not final balance

- Forward target in a **reachable 80–180px envelope**. The new behavior must close distance rather than simply hold at the old 200px standoff. A target may be Carl **or** Donut if real, visible and valid; no fixed P1 bias.
- **Windup ~30–36 ticks**, unlike orange glowing spit: body dips, hooves scrape, readable low silhouette. Do not fake tell by spamming a generic lava pose.
- **Committed 210–250px/s for 20–28 fixed ticks** with no mid-flight direction change, tracking or target correction. Stops on real solid intersection, target contact or maximum authored path.
- **Recovery ~30–42 ticks** and a meaningful attack cooldown; punish by Carl's kick or Donut Magic Missile/Pounce. Attack remains real pressure without HP immunity.
- **Damage:** one qualified low-body overlap, 1 HP initial hypothesis, proper existing invulnerability/damage-instance handling. World collider and visual footprints must agree.
- **State:** no simultaneous charge+spit from the same llama; true state transitions and event tokens all deterministic. Timing may need adaptation to existing hitstop and interrupts.
- **Targeted attack dodge policy:** explicitly review with DONUT-MOBILITY-01 owner whether rush belongs in the 12s reflex's evadable list; not automatically permitted. Run baseline with reflex disabled in isolated fixture and with integrated reflex.
- Make line-of-sight, real distance, recovery and positioning the challenge; **never solve repeated static three-bolt kills by arbitrary magic armor, an invisible phase, projectile range nerf or a hidden global speed boost**.

## Mandatory failure-mode tests

- **Static exploit:** use genuine *press edges* at each legal cast window while Donut **never moves**; avoid the false test of one long held Primary (it will not recast). Evaluate M1 vs M2 separately. Probe multiple natural shooting/perch positions, multiple seeds/configs if appropriate and measure any Carl-AI damage separately; normal static magic M1 win is allowed, M2 routine no-risk wins are **a design failure**.
- **Opponent independence:** Carl solo-direct kill; Donut mobile solo kill; P1+P2 works without a forced simultaneous move, single target restriction, companion precision save, or an offscreen glob/rush.
- **True physical cause:** if the rush hits a partition, that fixture genuinely receives the hit and changes real collision/line of sight. A near miss must not break it; never automatically destroy via timeline.
- **Existing collision truth:** grounded Donut may remain under high lava; raised Donut may be hittable; elevated horizontal Magic Missile may miss the llama. Do not homogenize all three heights with undocumented aim assists.
- **Readable recovery:** a miss lets human Carl or Donut punish before the next charge; one-shot contact token and no phantom damage after the committed movement stops.
- **Do not silently alter old demos:** new rush only when opted in; legacy goblin and llama fixtures/replays remain unchanged unless an independently approved compatibility change states otherwise.
- **Determinism:** fixed 60Hz AI and collisions; `charge_selected`, `charge_committed`, `charge_contact_or_miss`, `partition_broken`, damage and mana events where current schema supports them; preserve existing replay schema compatibility, reproducible reset, checkpoint and hash evidence.
- **Performance/visuals:** show real native frames of fully visible threats; not static concepts; compare orange throat glow, distinct low-rush prep, magenta Magic Missile, and restaurant's blue/white relief. Rare special slow-motion only when existing cinematic system actually awards a skilled kill.

## Results / completion handoff

Document evidence in the final implementation PR using this minimum matrix:

| Outcome | Evidence required |
| --- | --- |
| **Ordinary first spell** | Real 1x gameplay showing Donut hitting a goblin after Guild with exactly correct mana |
| **First llama** | Legit three-bolt easy magical clear and viable melee approach |
| **Corral static-spam probe** | Location-by-location survival, HP/MP/damage attribution, shots landed/missed, llama actions; code-generated fixed traces, **not hand-waved** |
| **Corral mobile strategy** | At-speed Donut reposition/shot/run vs charge, Carl close-punish, real 2P co-op |
| **Optional environmental payoff** | Physical rush->partition collision, real LoS change, distinct victory; if not implemented, clearly blocked/deferred |
| **Trustworthiness** | MAG-01..16 actual statuses; retry, 1P/2P, native capture, Playtrace verify twice, no-regression, perf and owner human signoff **pending until played** |

**Stop/flag for owner review** if the only way to make M2 difficult is unearned magic immunity, an invisible projectile angle change, mandatory hand-timed 2P timing or universal llama stat inflation. The desired fix is *good enemy decisions, fair movement and memorable geography*.

## Precedence / links

- [Revised owner-selected design](../01-design/floor-01-llama-magic-baptism.md)
- [Act I beats](../01-design/floor-01-act-01-playable-combat-beats.md), [combat-first](../01-design/combat-first-encounter-design.md)
- [LLAMA-01 native art](../02-art/llama-01-spec.md), [Magic Missile animation](../02-art/donut-magic-missile-anim-03-spec.md)
- [Donut mana](../01-design/donut-mana-v0-proposal.md), [DONUT-MOBILITY-01](../03-tech/donut-mobility-01.md)
- [COMBAT-LAB-01](../01-design/combat-lab-01-rooms-r00-r02.md), [Playtrace](../03-tech/playtrace-01-session-capture.md)

**Previous version superseded:** instructions to implement a "Throat Choir" with two alternating stationary spitters, without rush, are **not valid** for this PR.
