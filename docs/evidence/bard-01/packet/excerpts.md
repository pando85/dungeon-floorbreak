# Doc excerpts

## docs/02-art/floor-01-minor-enemy-concepts.md#Goblin Bomb Bard

### Goblin Bomb Bard

**Reference:** `assets/concept/floor-01/enemies/floor01-goblin-bomb-bard-concept-approved.png`.

A manic, comparatively slender **goblin-family** combatant with pointy ears, goggles and scarf, improvised brass horn, scavenged copper fittings and visible bomb-like gear. The musical/performance silhouette separates this archetype from the two ordinary opening goblin variants and the pot-helmet engineer. Key poses feature horn use, explosive-object handling and panic under pressure.

**Production guardrails:** Exact musical mechanics, explosion timers, area damage, self-detonation, bomb chaining and any specific instrument named in generated captions are *not* approved gameplay requirements. The Bomb Bard is a separate **later Floor 1** variant; do not arm the opening goblins or engineer with an invented loadout.


## docs/02-art/floor-01-minor-enemy-concepts.md#Shared visual direction

## Shared visual direction and source boundaries

- All three share **THE CRAWL — NO PANTS. NO MERCY.** gritty 80s/90s action sensibility and the existing catacomb environment grammar (warm torch pools, cool recessed stone, local copper/industrial props). Not medieval fantasy or a blanket neon filter.
- **Visual concepts approved.** No automatic approval of AI-in-image numeric labels, narrative claims, level mechanics or exactly depicted weapons. Separately check them against the story and encounter requirements before implementing native sprites.
- Convert from *original editable pixel grids/`.ase`* rather than cropping illustrated panels into runtime sprites. Rat readability and Chopper width need native 640×360/640×400 review against Carl 52px and Donut 40px.
- Preserve original uploaded reference images unchanged. Record provenance and SHA-256 values in the existing enemy reference manifest (see `../../assets/concept/floor-01/enemies/manifest.json`).
- Publication remains limited by the owner-reported art-only permission scope in the rights policy (see `../04-production/rights-and-release.md`); this does not clear distribution of a playable game or standalone asset pack.


## docs/02-art/gob-01-goblin-spec.md#7. Palette

## 7. Palette `goblin-floor01-v0` (at most 20 colours per atlas)

A uses 19 colours and B 20. The colours are tuned for the A1 lighting (warm torch light falling into cold blue ambient).

| Key | Name | Hex | Role |
| --- | --- | --- | --- |
| K | outline_plum | `#1b1124` | shared with Carl |
| a / s / m / l | skin deep / shade / mid / light | `#263a26` `#46682e` `#7a9e36` `#b2cc56` | yellow-green skin; cool shadows, warm lights |
| r / t / u | rag dark / mid / light | `#2c1e28` `#4c3436` `#73524a` | cool, dark leather that separates from the orange brick |
| e | eye_yellow (accent) | `#ffd23a` | glaring eyes |
| w | tooth_white (accent) | `#efe4c4` | teeth; eye whites in hurt, knockback and down |
| o | mouth_dark | `#5c1426` | |
| R | red_mid | `#b8322e` | tongue; variant B bandana |
| x / y | wood dark / mid | `#4a2c1c` `#8c5c32` | stick, rope belt |
| F / G / H | pineapple dark / mid / light | `#7a3c12` `#d08826` `#f4c84a` | checker-pattern fruit; H is an accent (single facet highlights, and the smear arc) |
| V / v | leaf dark / mid | `#1d4a3c` `#3e8a5c` | teal crown, kept off the skin hue |
| B | rag_red_dark (variant B only) | `#6e1a22` | bandana shadow |

