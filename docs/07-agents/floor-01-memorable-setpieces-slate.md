# FLOOR01-SETPIECES-B — Production slate and local-agent handoff

**Status:** three newly designed scenes are **design-only**, with no implementation claimed. This work follows the separately open [Goblin Boiler PR #176](https://github.com/pando85/dungeon-floorbreak/pull/176), which is an implementation work order on its own branch. **Do not merge or implement these three into the boiler PR**. The current PR is a *design review* and allows focused, separate implementation PRs when dependencies are ready.

## Intent: four distinct flavors, one coherent game

| Encounter | Stage | Emotional hook | Primary reusable mechanics | Timing hypothesis | Design |
| --- | --- | --- | --- | --- | --- |
| **The Mountain Breathes / Hoarder** | 04 | Dread, hiding monster, moving cover | Authored pile collapse/slide, real boss recovery and exposure | 2–3min | [Hoarder](../01-design/floor-01-hoarder-mountain-encounter.md) |
| **Goblin Boiler** | 05 | Player-caused industrial accident | Valve/steam, weight, blast attribution, bounded two/three-link chain | 90–180s | [Boiler #176](https://github.com/pando85/dungeon-floorbreak/pull/176) |
| **Dozer Revenge** | 05 | Return of early terror; optional victory | Physical vehicle chase/disable, stop frame, camera framing | 60–100s | [Dozer](../01-design/floor-01-dozer-revenge-encounter.md) |
| **The Floor Lies / Slimes + Rot Stickers** | 06 | Reading danger around the body, turning it against monsters | Corrosive contact, wall/ceiling movement, local detonation | 2–3min | [Hazards](../01-design/floor-01-slimes-rot-stickers-encounter.md) |

**Sequence:** the stages remain chiefly linear. These are **signature pockets**, not proof that the full 8-stage level map exists, and they should not all be mandatory closed arenas. Dozer "revenge" is explicitly an original optional adaptation rather than asserted source canon; Hoarder is a transformed human woman, not a goblin; Slimes and Rot Stickers are distinct species.

## Recommended parallelization (avoid shared-file races)

1. **Now: finalize design reviews** for the three documents in this PR, especially Hoarder's treatment and Dozer's optional defeat/canonical placement. The local agent implementing #176 can work meanwhile.
2. **Independent track:** original native enemy art/animation and simple gameplay profiles for Hoarder, Slime and Rot Sticker, each with their own approval and tests. Visual concept approval does not imply implementation approval. Do not block room geometry on final art.
3. **Shared physical interface track:** complete/reuse exactly **one** authoritative contact/prop/defeat-cause interface, preferably by extending the existing support/fall/terrain/projectile and explosion types in #176. Avoid building three conflicting falling-weight or explosion engines.
4. **Hoarder implementation PR:** start with geometry + direct encounter, then two authored pile state machines. **No dynamite pre-Hoarder**. Can proceed independently of #176 for room/design while any shared physics changes wait for merge.
5. **Slime/Rot implementation PR:** prototype each enemy/hazard in isolation, then add mixed pocket and collateral damage. Can proceed independently of Hoarder art and Dozer gameplay; coordinate blast attribution API with boiler.
6. **Dozer revenge implementation PR:** requires audited opening Dozer rig/scene and ideally one merged swept-collision/prop interface from boiler; chase/escape path first, optional counterstrike second. Do not let this delay the basic linear Stage 05 progression.
7. **Stage integration later:** connect Stage 04 → Workshop/Boiler → optional Dozer reversal → hazards → survivor camp, with checkpoint/retry continuity, story-flag persistence, and 1P/2P path proof. No new inventory or menu systems.

No local agent should take ownership of all these shared subsystems concurrently. Maintain separate branches, scoped implementation PRs, and explicit interface contracts. Review open character progression/companion and playback PRs before editing `src/bin/floor01_scene.rs`, combat adapters, terrain, camera and shared schemas.

## Shared encounter invariants

- Ordinary attacks alone must give a valid, satisfying progression; **spectacular routes are optional**.
- No level sequence may use a move before its story unlock. **Post-Guild Donut:** Magic Missile and existing cat movement; **post-Hoarder Carl:** earned dynamite; **post-Juicer only:** gauntlet and Puddle Jumper. Treat new intermediate jump upgrades as provisional until story review.
- Existing passive Donut dodge/reflect is **being implemented separately**; do not implement it again here. Hazards expose attack classifications once the canonical contract is available.
- The critical rightward route, exit, checkpoint and co-op reunion must remain reachable after all authored destruction states. No mandatory backtracking, timed two-human gate, locked-on camera that hides a threat, or offscreen projectile/vehicle hit.
- Every meaningful effect has a visual/sound warning, source ID, authoritative fixed-step event and bounded hit tokens. Camera, VFX, audio, render replay and cinematic slow-motion must not change gameplay results.
- Avoid generic HP inflation. Boss/hazard difficulty comes from positioning, legible commitment, geometry and enemy behavior.
- No unapproved new controls, enemy mechanics inferred from art labels, canonical claims inferred from generated imagery, unauthorized copy of outside game assets, or ML-based auto-approval of "fun".

## QA evidence required from each future implementation PR

1. **Path and solo/2P:** reach exit with all props untouched and again after every authored destructive state, two solo active-character modes and P1+P2 local.
2. **Baseline direct win:** clearing enemies/boss through ordinary abilities (except explicit optional Dozer bypass) without exploiting scenery.
3. **Tactical use:** one modest real collision/terrain advantage with traceable source.
4. **Spectacular use:** one optional high-skill effect requiring actual collision/contact/blast, recorded as an event chain rather than a scene-script kill.
5. **Reset:** retry mid-explosion/mid-collapse/mid-vehicle movement with identical seed and state transitions.
6. **Playback:** fixed-tick authoritative event sequence and hashes reproduce with Playtrace verification twice; record every human playtest automatically using existing PLAYTRACE policy.
7. **Evidence:** real 640×360 / 640×400 Bevy captures, at-speed clips, attack tell readability under 2P camera, no performance regression. **Owner human game-feel review remains pending until tested.**
8. **Quality judgement:** at least two meaningfully different player-observed outcomes and one truly memorable outcome per setpiece; targets are hypotheses and cannot be declared satisfied by automated scoring alone.

## Milestones

| Milestone | Deliverable | Completion gate |
| --- | --- | --- |
| SP-B0 | Three designs reviewed + topology audited for Floor 01 | Design PR merged without calling anything implemented |
| SP-B1 | Hoarder playable direct fight; Slime/Rot isolated enemy probes; Dozer escape greybox | Actual interactive scenes/recorded evidence, no fake post-unlock abilities |
| SP-B2 | Each scene's principal prop/impact behavior working | Headless deterministic tests + no-softlock paths |
| SP-B3 | Optional player-authored environmental payoff in each scene | Verified real event/collision chain and gameplay-scale footage |
| SP-B4 | Integrated Floor 01 Stage 04–06 path and session playtest | 1P + 2P progression, preserved story flags, checkpoints, human scoring |

## Design decisions deliberately left for game-feel review

- Hoarder: how frightening vs mechanically accessible? Can direct fighting hold up without collapse? How should the boss react to a falling pile without being instantly defeated?
- Dozer: optional victory vs only an environmental disable; crew combat optional; whether the returning machine is an identical instance or same model. Canon and story team approval before entering actual narrative route.
- Slimes/Rot: precise telegraphs, radius/contact damage and damage classification relative to the independently owned Donut auto-dodge system.
- All: room length, enemy count, camera opening and spectacle thresholds **require actual at-speed testing**.

The implementation agent must review `AGENTS.md`, current branch ownership, authoritative input/level/data contracts and existing production assets before coding. This document is a handoff index, **not** a substitute for the three linked encounter specifications.
