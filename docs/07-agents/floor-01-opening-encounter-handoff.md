# Local-agent handoff — Floor 01 opening encounter (AFTER pending gameplay PRs)

**Status:** future implementation work order; **do not start production changes yet**.
**Read first:** `AGENTS.md`, `docs/02-art/floor-01-opening-catacomb-goblins.md`, `docs/02-art/character-scale-approval.md`, `docs/02-art/asset-pipeline.md`, `docs/03-tech/level-data.md`, `docs/01-design/antihero-tone.md`, `docs/04-production/rights-and-release.md`.

## Dependencies and sequencing

1. Integrate/review **PR #21**, the combat-first linear Floor 1 Bevy greybox: ensure actually green CI, headless tests, gamepad and visual evidence. Preserve its core Rust gameplay world and horizontal camera.
2. Integrate/review **PR #22**, the separate Bevy Character Playground: verify generated art imports, actual 52 px Carl / 40 px Donut scales, pivots and native 640×360 / 640×400 screenshots.
3. **Only then** schedule this opening art/gameplay composition as independent local-agent PRs. PR #21's combat sandbox and this story-driven introductory chase serve different test goals; do not silently replace one with the other.
4. Reconcile any main branch or schema changes before implementation. No branch stacking on unmerged PRs unless explicitly coordinated by the integrator.

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
