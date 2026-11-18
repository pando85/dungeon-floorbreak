# Floor 01 — Ball of Swine arena: standalone concept

**State (2026-11-18):** The owner prefers the latest wide gameplay-layout concept, and the arena-wide camera pullback is an accepted *direction to explore*. This is concept approval, not approved native art, runtime geometry or camera behavior. Four original PNGs are available in the separate PR-specific Hermes ZIP, but binary upload to this branch remains pending.

## Design identity

One late-Floor-01 monumental *side-view* catacomb combat room, retaining the established A/B/C brick-and-stone language with unusual concentric masonry, circular grooves, oversized brass/iron mechanical gates, chains, torches and limited magenta/UV show lighting. The spectacle is exceptional to this boss. Its physical silhouette must remain intelligible with the lights and particles disabled.

The Ball of Swine is the already-approved tumbling composite of many aristocratically clothed Tusklings. The arena presents a recognizable geometry: a broad protected ground lane with enough run-up for charges, short elevated flanking routes on both sides, a hanging central impact gate, movable side segments and localized destructible supports. Concentric rings belong to the architecture; gameplay must work in **2D side view**, not a top-down maze.

Preserve 1980s–1990s gritty action and dark absurdist humor. No generic neon wallpaper throughout Floor 01, automatic full-structure collapse, fragile exits or constant untelegraphed hazard spam.

## Four image slots (upload originals unchanged)

The artwork shown to the owner is **not a native pixel sprite or an approved collider layout**. The exact binaries are included in the handoff ZIP and must be committed to the same PR branch under assets/concept/floor-01/ball-of-swine-arena/. File identities and SHA-256 appear in the ZIP manifest; do not fabricate a manifest entry in GitHub before the binaries arrive.

| Destination filename | Role | Decision scope |
| --- | --- | --- |
| 01-arena-overview.png | Empty monument, inlaid circular floor and huge gates | Previous positive arena art direction |
| 02-first-impact.png | Ball collides with a hanging gate during combat | Impact/action staging exploration; character drawings not canonical |
| 03-space-usage-exploration.png | Early space-usage infographic with combat and stage states | Historic exploratory draft |
| 04-gameplay-layout-preferred.png | Updated wide annotated layout, movement and routes, stages and camera | **Owner-preferred layout direction** (not final numbers or physics) |

The infographics contain illustrative labels and inconsistent Carl/Donut renderings. For game-scale shots **always use approved original Carl 52 px and Donut 40 px masters**; no superhero Carl, sword or invented feline model. The approved Ball of Swine boss sheet remains in the existing enemies directory.

## Spatial composition

1. **Main ground lane:** continuous and mostly open for Carl kicks/strikes, charging sphere, detached Tusklings and environmental impacts; retain a deep floor and strong read of the walking line.
2. **Left elevated flank:** an upper position with fast Donut traversal and a slower physically plausible route for Carl, rather than a Donut-only mandatory escape.
3. **Right elevated flank:** a second combat angle/retreat, useful to Donut but not permanently safe from enemies.
4. **Central suspended impact gate:** a strongly telegraphed vertical stroke and bounded collision that can create boss stagger/redirect opportunities.
5. **Side gates / breakable supports:** a few specific changing structures, with protected floor and protected/recoverable entry, exit and regroup points.
6. **Late-state destruction:** visually dramatic debris and selected missing masonry, but never a camera-obscuring wall of particles.

The foreground architecture must not cover essential character silhouettes, collision tells or the boss approach. Make space meaningful: gaps/height changes should change offensive positioning rather than force repeated mandatory platforming puzzles.

## Progressive visual study

- **Intact:** learnable rebounds, all side routes recognizable and one obvious impact gate.
- **Partly damaged:** one broken support, changed rebound, a bounded group of separated Tusklings and more combat options.
- **Finale:** selected side/background architecture cracks or fails, show lighting peaks, with a continuing accessible main lane and an earned direct or environmental finish.

These are *storyboard stages only*. No boss damage thresholds, exact timings or mandatory 3-phase system are approved.

## Next artwork / layout gates

- Produce a clean **unannotated** side view of traversable walkable surfaces, ladders/ramps, door/exits, gate sweep zones, protected critical cells and destructible zones. Avoid opaque lighting and effects for this sheet.
- Show two actual scene shots using approved source masters and the native 640×360 and 640×400 frames.
- Show normal / medium / maximum camera coverage with two players separated and the boss charging; validate legibility especially on Steam Deck.
- Verify geometry and route continuity after each destruction/gate state, and prove boss rebounds are fair and deliberate.
- For any production sprites, create editable original .ase/grid sources instead of cutting from a concept picture.
- Distinguish independent top-tier art review, actual gameplay playtest and legal publication gates from art-direction preference.

## Dependencies and rights

See [Ball of Swine combat and camera proposal](../01-design/floor-01-ball-of-swine-arena-gameplay.md), [Floor 01 atlas](floor-01-art-atlas-v1.md), [late boss concept approvals](floor-01-late-enemy-boss-concepts.md), [approved controls](../01-design/mvp-controller-contract.md), [accepted ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md), and [asset intake folder](../../assets/concept/floor-01/ball-of-swine-arena/README.md).

The four original ChatGPT-generated references come from this project conversation (2026-11-18). No novel excerpts, third-party screenshots or game assets are included. Follow the [rights and release policy](../04-production/rights-and-release.md): the owner's recorded art-publication permission is not independently verified and is not a distribution license for a playable game.

**PR state:** draft and image-dependent until Hermes uploads the originals, updates the manifest/provenance and CI passes. Separate gameplay/Bevy acceptance remains open.
