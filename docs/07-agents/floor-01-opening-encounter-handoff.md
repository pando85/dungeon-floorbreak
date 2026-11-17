# Local-agent handoff — Floor 01 opening encounter (integrated-scene phase)

**Status:** prerequisites merged (PR #21 combat greybox, PR #22 Character Playground, PR #23 concept references). **Ready to plan the first integrated scene with local agents.** Human gameplay/art review intentionally happens on the integrated scene rather than blocking these tooling prototypes.
**Read first:** `AGENTS.md`, `docs/02-art/floor-01-opening-catacomb-goblins.md`, `docs/02-art/character-scale-approval.md`, `docs/02-art/asset-pipeline.md`, `docs/03-tech/level-data.md`, `docs/01-design/antihero-tone.md`, `docs/04-production/rights-and-release.md`.

## 2026-11-17 integration directive: use this opening as the combat testbed

**Owner direction:** Include the [staged combat/physics probes](floor-01-integrated-combat-probes.md) in the **same first playable Floor 1 scene** rather than in an unrelated permanent combat demo. Preserve the threatening Dozer pursuit, then use the already-planned **post-chase on-foot goblin skirmish** for a basic hit, knockback into a second goblin, and—if safely supported—one authored environmental kill. Make reproducible headless/Bevy scenarios for these tests so gameplay can be tuned without replaying the entire cinematic opening.

**Important:** the separate WORK-021 greybox provides reusable melee, goblin and destruction systems, but its available bombs are **not** an excuse to grant Carl premature story gear. Pre-Guild Donut has no spells. Follow-on 1P switch / real 2P scenarios must remain **pending until implemented**. Experimenting with defeated bodies as projectiles is optional, **not a blocker** for the opening. A single integrator owns `floor01_scene` composition; do not parallelize conflicting edits with the other active scene agents.

## Dependencies and sequencing

1. **Done:** PR #21 (combat-first linear Floor 1 Bevy greybox) merged, CI passed, start-safe idle regression tested. Keep the deterministic Rust gameplay world and horizontally scrolling camera.
2. **Done:** PR #22 (standalone Bevy Character Playground) merged, CI passed, authentic 52 px Carl / 40 px Donut / 76 px Mongo atlases and fixed mirrored foot pivots verified in 640×360/640×400 render evidence.
3. **Done:** PR #23 (environment/goblin/Dozer concept references) merged with three image bytes, provenance and concept-only scope. Do not repurpose generated concept illustrations as production sprites or as new Carl/Donut references.
4. **Next:** prioritize ENV-01 and COMP-01 to compose the first calm catacomb scene, then GOB-01 / DOZ-01 and CHASE-01 as separate small PRs. Preserve the combat sandbox while adding the story-driven opening; they serve different test goals.
5. **Human review gate:** apply controller/visual acceptance to the **first integrated scene** (actual in-game camera, live combat and readable character sprites), not retroactively to the merged prototyping PRs. Reconcile new `main` schemas before implementation.

## Proposed independent work orders (IDs to assign when backlog is reconciled)

| Stage | Owner | Smallest useful PR | Evidence |
| --- | --- | --- | --- |
| **ENV-01** | Environment artist agent | Editable Aseprite brick/floor/torch/occasional arch/one pipe sources and a deliberately plain test corridor | Native 1×/2× contact sheets, reproducible export + pixel lint, side-by-side w/ clean concept |
| **COMP-01** | Bevy visual integrator | Render ENV-01 with the existing approved **actual** Carl and Donut atlases at unchanged 52/40 px heights | Bevy screenshots 640×360 and 640×400, occlusion/pivot and camera reports |
| **GOB-01** | Enemy art agent | Two *visually different* level-2 goblin riders + pot-helmet level-3 engineer, separated from vehicle; start with silhouette/idle only | Turnaround, Aseprite source/manifests, native-size view beside Carl, human review |
| **DOZ-01** | Vehicle art/animation agent | Tractor-scale tracked steam machine, large front spiked wheel with separate spin animation; smoke layer nonblocking | Source sprite layering, idle/spin/smoke previews, damage/wheel warnings; no guessed hitbox from rendered image |
| **CHASE-01** | Gameplay agent | A deterministic horizontal introductory chase, navigable junction/turn area, escape, and optional dismount transition | Seeded action replay, reachable exit, fail/retry, bounded camera and collision tests |
| **INTEGRATE-01** | Integrator/QA | One continuous intro capture and minimal playtest rubric | FPS/frame-time instrumentation, real Bevy recording, 2 gamepad test where applicable, human game-feel signoff |

Do not parallelize work that edits the same character sprite master, palette, map schema or world collision logic. A single integrator owns the composition branch.

## Spatial design hypothesis (not authored level geometry)

- **Segment A:** quiet plain brick corridor, one clear walkable line, enough room to establish motion and camera cadence.
- **Segment B:** occasional arch/pillar and turning junction, signage/ambient cues; increased tension without filling the space with props.
- **Segment C:** small localized maintenance hints (one valve/pipe and subtle lantern/lichen accent) before the Dozer enters.
- **Segment D:** widened passage where a tracked Dozer charges and can overshoot/struggle turning; clear escape path and no camera softlock.
- **Segment E:** brief dismounted goblin melee and escape toward the Guild. Avoid presenting it as a separate full mechanical workshop or a stationary boss arena.

For the first screenshot, the environment should occupy most of the view, actors read on one clean floor plane, the dozer should remain legible as a looming incoming threat. The camera can show pursuit through horizontal scrolling rather than shrinking the actor sprites to fit all content at once.

## Anatomy / equipment validation

- Common goblins (2): green, approx 4 ft by novel description, leathery rag clothing, sharp teeth, pointy ears. Their **two portraits are a visual variation**, not two approved combat classes.
- Engineer (1): kitchen-pot helmet, level 3, operates the Dozer.
- Early on-foot goblin weapon is a **pineapple on a wooden stick**; generated studded/metal club art requires correction before final production.
- Vehicle: rusted mismatched steam contraption, tank treads, huge spiked spinning front wheel, multiple smoking pipes, three riders. Approx tractor scale, but **no final pixel height/width has been approved**.
- Dozer has a large turning radius in the source encounter; chase AI/state transitions are implementation proposals pending playtest.
- Carl and Donut from generative key art are **invalid characters**. Source only `carl_h52.ase` and `donut_h40.ase` (or their approved subsequent in-repo revisions).

## Build and review protocol

- Follow existing exports and source manifests; new entities must not bypass current Aseprite/LibreSprite tooling.
- Keep authored LDtk geometry and actual mutable terrain separate; only integrate tooling versions already in the chosen Rust/Bevy stack.
- Distinguish collidable tiles from parallax/background artwork. Protect mission-critical floor and exits from demolition softlocks.
- Unit tests and reproducible seeded behavior must cover pressure/chase at fixed tick, wheel hazard timing, and exit reachability before adding presentation polish.
- For native screenshot diffs, report **original pixel sizes** and actual screenshots: composited mockups are not proof of Bevy rendering.
- Stage one novel element at a time. Opening is modest and quiet; danger grows through sound/movement then mechanical intrusion, then close-range goblin trouble.
- Document IP scope honestly; public concept art does not grant executable-distribution rights.

## Acceptance questions for the human art review

1. Does the regular passage read as an intentional clean brick catacomb, rather than a sewer or collapsed factory?
2. Can you identify Carl (52 px), Donut (40 px), two goblins, engineer and the Dozer wheel at the intended camera scale?
3. Does the chase teach through anticipation, pursuit and escape rather than simultaneously explaining multiple systems?
4. Are the special-environment details localized, leaving other Floor 1 areas room for later distinct design?

Do **not** call the opening complete until actual editable assets, Bevy capture, and owner approval exist.
