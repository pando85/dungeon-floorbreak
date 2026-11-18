# Doc excerpts

## docs/02-art/floor-01-minor-enemy-concepts.md#Level-2 dungeon rat

### Level-2 dungeon rat

**Reference:** `assets/concept/floor-01/enemies/floor01-dungeon-rat-concept-approved.png`.

An isolated, recognizable small vermin silhouette: long wiry torso, arched mangy back, patchy gray-brown fur and pink bald patches, torn ears, yellow teeth and a whip-like tail. The one-sheet explores low stalking, sprinting, leaping, a bite and defeated poses. The creature should read against ordinary brick-and-torch catacombs rather than imply every room is an underground sewer.

**Production guardrails:** Gameplay-scale height, aggro patterns, pack AI, hit points, damage and precise leap range were **not** approved by this reference. Do not use the oversized concept proportions as proof of final pixel-scale.


## docs/02-art/floor-01-minor-enemy-concepts.md#Shared visual direction

## Shared visual direction and source boundaries

- All three share **THE CRAWL — NO PANTS. NO MERCY.** gritty 80s/90s action sensibility and the existing catacomb environment grammar (warm torch pools, cool recessed stone, local copper/industrial props). Not medieval fantasy or a blanket neon filter.
- **Visual concepts approved.** No automatic approval of AI-in-image numeric labels, narrative claims, level mechanics or exactly depicted weapons. Separately check them against the story and encounter requirements before implementing native sprites.
- Convert from *original editable pixel grids/`.ase`* rather than cropping illustrated panels into runtime sprites. Rat readability and Chopper width need native 640×360/640×400 review against Carl 52px and Donut 40px.
- Preserve original uploaded reference images unchanged. Record provenance and SHA-256 values in the existing enemy reference manifest (../../assets/concept/floor-01/enemies/manifest.json).
- Publication remains limited by the owner-reported art-only permission scope in the rights policy (../04-production/rights-and-release.md); this does not clear distribution of a playable game or standalone asset pack.

