# Floor 01 — Minor enemy and goblin vehicle concept approvals

**Decision:** Owner approved the visual direction of the three newly explored enemies following the previously approved main bestiary and boss sheets on 2026-11-17. **This is visual-concept approval, not native gameplay sprite or mechanics approval.** The three images are new reference art; production remains with local agents.

**Sources:** Names/placement for dungeon rats, the Bomb Bard and Copper Chopper come from [the project's Book 1-based late-bestiary inventory](floor-01-late-enemy-boss-concepts.md). The PNGs themselves are newly generated concept explorations in the project conversation, *not* excerpts or screenshots from the book. Follow [existing goblin art identity](floor-01-opening-catacomb-goblins.md), [environment art atlas](floor-01-art-atlas-v1.md) and [the 1980s–1990s antihero rule](../01-design/antihero-tone.md).

## Approved concepts

### Level-2 dungeon rat

**Reference:** `assets/concept/floor-01/enemies/floor01-dungeon-rat-concept-approved.png`.

An isolated, recognizable small vermin silhouette: long wiry torso, arched mangy back, patchy gray-brown fur and pink bald patches, torn ears, yellow teeth and a whip-like tail. The one-sheet explores low stalking, sprinting, leaping, a bite and defeated poses. The creature should read against ordinary brick-and-torch catacombs rather than imply every room is an underground sewer.

**Production guardrails:** Gameplay-scale height, aggro patterns, pack AI, hit points, damage and precise leap range were **not** approved by this reference. Do not use the oversized concept proportions as proof of final pixel-scale.

### Goblin Bomb Bard

**Reference:** `assets/concept/floor-01/enemies/floor01-goblin-bomb-bard-concept-approved.png`.

A manic, comparatively slender **goblin-family** combatant with pointy ears, goggles and scarf, improvised brass horn, scavenged copper fittings and visible bomb-like gear. The musical/performance silhouette separates this archetype from the two ordinary opening goblin variants and the pot-helmet engineer. Key poses feature horn use, explosive-object handling and panic under pressure.

**Production guardrails:** Exact musical mechanics, explosion timers, area damage, self-detonation, bomb chaining and any specific instrument named in generated captions are *not* approved gameplay requirements. The Bomb Bard is a separate **later Floor 1** variant; do not arm the opening goblins or engineer with an invented loadout.

### Goblin Copper Chopper with sidecar

**Reference:** `assets/concept/floor-01/enemies/floor01-goblin-copper-chopper-concept-approved.png`.

Owner-approved **light steam-powered goblin motorcycle/sidecar direction**, distinct from the tracked Murder Dozer: copper tubes, patched sheet metal, compact boiler, smoking exhaust, crooked headlight, exposed engine and shaky sidecar. The concept shows a goblin driver and sidecar passenger and investigates side/front/back profiles, acceleration, drifting and an exaggerated spill.

**Production guardrails:** This is a vehicle encounter, not a new humanoid species. The concept's passenger weapon, exact crew count, vehicle width, speed, collision radius, crash damage and scriptable pursuits are provisional illustrative ideas. Use the existing source character/vehicle constraints rather than deriving combat rules from the illustration. Preserve the opening Dozer's established visual identity and setpiece.

## Shared visual direction and source boundaries

- All three share **THE CRAWL — NO PANTS. NO MERCY.** gritty 80s/90s action sensibility and the existing catacomb environment grammar (warm torch pools, cool recessed stone, local copper/industrial props). Not medieval fantasy or a blanket neon filter.
- **Visual concepts approved.** No automatic approval of AI-in-image numeric labels, narrative claims, level mechanics or exactly depicted weapons. Separately check them against the story and encounter requirements before implementing native sprites.
- Convert from *original editable pixel grids/`.ase`* rather than cropping illustrated panels into runtime sprites. Rat readability and Chopper width need native 640×360/640×400 review against Carl 52px and Donut 40px.
- Preserve original uploaded reference images unchanged. Record provenance and SHA-256 values in the existing [enemy reference manifest](../../assets/concept/floor-01/enemies/manifest.json).
- Publication remains limited by the owner-reported art-only permission scope in the [rights policy](../04-production/rights-and-release.md); this does not clear distribution of a playable game or standalone asset pack.

## Owner image upload — verified

These three *original byte-identical* files are committed under `assets/concept/floor-01/enemies/`:

| Image | Status |
| --- | --- |
| `floor01-dungeon-rat-concept-approved.png` | Uploaded, source identity verified |
| `floor01-goblin-bomb-bard-concept-approved.png` | Uploaded, source identity verified |
| `floor01-goblin-copper-chopper-concept-approved.png` | Uploaded, source identity verified |

The source originals were independently checked against uploaded Git blob SHA-1 identifiers, and the recorded SHA-256, file lengths and dimensions match. The image manifest now has eleven entries (previous eight preserved), and `tests/test_art_floor01_enemy_concepts.py` requires all eleven unchanged PNGs. **The art-assets check passed on the upload commit**; rerun final CI after the documentation update before merging.

## Follow-up work, deliberately excluded

The **Juicer's Gym** environment concept is **approved in mood and visual identity**, but still requires a complete standalone room study and its own separate PR: reception, turnstile, fighting space, machine placement and props. It must **not** enter this enemy PNG bundle. The Ball of Swine arena and optional unseen Goblin Chieftain also remain outside this PR.

## Acceptance

- [x] Three owner-approved minor enemy/vehicle directions accurately recorded.
- [x] References pinned by SHA-256 and byte/dimension metadata.
- [x] Existing 8 PNGs and approvals preserved.
- [x] Owner committed all three original PNGs unchanged.
- [x] Eleven-image hash/dimension validation passed on the upload commit.
- [ ] Final PR CI green.
