# Floor 01 — Environment Art Atlas v1

**State:** first-floor **concept-direction baseline**, reviewed positively by the owner on 2026-11-16; **not** a production-ready tileset, game camera, actual level topology or final monster designs.
**Game:** THE CRAWL — NO PANTS. NO MERCY. (working fan-project branding)
**Stack:** Rust + Bevy. First-floor scene: [ENV-01/COMP-01 pipeline](../07-agents/floor-01-opening-encounter-handoff.md).
**Primary art constraints:** [opening corridor and goblin direction](floor-01-opening-catacomb-goblins.md), [art direction](art-direction.md), [rights and release](../04-production/rights-and-release.md).

## Enemy concept approval update (2026-11-17)

**All Floor 01 enemy visual concepts in the later owner review are approved**, including the C+A Bad Llama, Scatterers/Brood Guardian, Rot Sticker, Green Slime and The Hoarder, in addition to the existing opening goblins and Murder Dozer. The earlier sentence in this atlas saying final enemy designs await approval is superseded **for this specific approved concept batch** by the [enemy concept roster](floor-01-enemy-concept-roster.md). Original production pixels, animation, exact sprite scales, game mechanics and Bevy review still have their own gates. The owner has committed the five newly approved concept PNGs to `assets/concept/floor-01/enemies/`; their checksums and dimensions are recorded in that directory's manifest. The atlas' area layout remains only an art inventory.

## Artistic premise

**One floor; one recognizable architectural language; several memorable places.**
Floor 1 is primarily a brick and stone catacomb network with functional, fairly clean passageways. Settlements, monster territories and especially encounter rooms become distinct through **local props, ambient damage, lighting and set dressing**, not a total palette/tileset reset every thirty seconds.

**Gradual immersion is mandatory:** discovery -> quiet exploration -> foreshadowing -> encounter -> short recovery -> increasingly surprising places and escalating spectacle. Leave breathing room and strong silhouettes. Avoid constant clutter, constant neon/VHS filters, sewage, heavy industry or television spectacle at the entrance.

Our shared **A/B/C corridor grammar** remains authoritative:

- **A / ordinary:** red-brown brick, solid dry walking line, a few torches, restrained structural depth.
- **B / architectural:** occasional arches, pillars, cool blue recesses and meaningful landmarks.
- **C / local industrial:** isolated pipes, valves, railings and maintenance traces. Workshop density belongs only in a true workshop.

All area kits are extensions to these common materials. Their percent distribution is **not prescribed**.

## Area index and art direction

This numbering is an **art inventory**, not a book-accurate map or a promise that gameplay visits every area in the indicated sequence. Source-grounded elements and our creative choices are intentionally separated.

| ID | Area | Source-grounded identity (Book 1) | Selected concept treatment / adaptation | Candidate reusable kit |
| --- | --- | --- | --- | --- |
| 01 | Monumental access + ordinary corridors | Large decorated entrance followed by wide torchlit passages and brick-lined branches (Ch. 2) | One special entry, then calm A/B/C modular corridors with floor readability | **BASE:** masonry, arches, torch, entry |
| 02 | Mordecai's Tutorial Guild | Small inhabited guild room with bed, hearth, books and personal items (Ch. 3–7) | **Chosen third 'old guide's room' concept:** modest, warm and slightly lived-in; leave central floor open | **REFUGE:** door, hearth, bed, shelving, small furnishings |
| 03 | Bad Llamas district | Neighborhood populated by dangerous lava-spitting llamas (Ch. 8, 15) | **Local** heat damage and ash on otherwise ordinary catacombs; not a generic volcanic biome | **HEAT:** scorch decals, heat traces, hazard tells |
| 04 | First fast-food safe room | Blue/white restaurant, kids' tunnels/slide and ball pit, exterior windows looking onto rock (Ch. 9) | 1990s family restaurant dropped into an implausible underground context; keep it recognizable and unexpectedly clean | **PUBLIC SPACE:** tiles, booths, counter, light strips, play area |
| 05 | Scatterers district + the Hoarder's lair | Insect-infested neighborhood culminating in an enormous junk-filled lair (Ch. 10–11) | Clutter **concentrated** in the lair; entry corridors remain mainly stone/brick | **HORROR/TRASH:** piles, refuse, torn textiles, barrel fires |
| 06 | Goblin territory + workshop | A separate later machinery/workshop area with tools, steam works and Dozers (Ch. 15–18) | Dense improvised boiler works and industrial props; not the opening Murder Dozer **chase** corridor | **INDUSTRIAL:** boiler, gears, tracks, pipes, props |
| 07 | Slimes / Rot Stickers | Green corrosive slime in one area; small black wall/ceiling-running explosive Rot Stickers in another (Ch. 15, 19) | **Concept comparison only:** separate acid and parasite treatments over a shared brick substrate; **do not infer adjacent locations from the composite** | **HAZARDS:** acid surface marks, tiny ceiling tell markers, contact FX |
| 08 | Concentric maze + Meadow Lark survivor camp | Ringed, gated labyrinth and organized survivors/camp (Ch. 21–22) | Broad circular stone structure + pragmatic, temporary human refuge; the overview compresses distant spaces | **RINGS / CAMP:** gates, multi-level masonry, makeshift cloth and crates |
| 09 | Troglodyte territory + The Juicer's gym | High-ceiling gym with reception and turnstile, exercise equipment, mirrors and heavy weights (Ch. 24) | Absurd 1990s gym inside the catacomb; high contrast and some fluorescent/accented light **only here**; mirror/weight collision designed later | **GYM:** reception, mirrored wall, weight racks, movable/breakable props |
| 10 | Ball of Swine arena + floor exit | Circular/spiral encounter, patterned grooves, rising/falling walls, pig-like composite rolling boss; UV and lasers during spectacle (Ch. 26–27) | Culminating architectural machine; **purple/magenta show lighting is a late exception**, not an all-floor rule | **ARENA:** movable gates, concentric channel markers, warning lights |

### Canon accuracy and illustrative liberties

Evidence was checked in the project's user-supplied Book 1 EPUB (relevant chapters noted above). Do **not** commit novel text, excerpts or copyrighted book images. Avoid treating compositional AI labels, floor plans, equipment and character likenesses as novel facts.

- Rot Stickers are described as **small dark round/ravioli-like creatures** that run across **walls and ceilings** and explode on contact, not large tentacled oozes. Slimes are separate corrosive creatures.
- The Juicer's gym has a **reception desk, turnstile, equipment, mirrors and heavy weights**. Do not infer a specific business logo, copyrighted text signage or unverified gym gameplay.
- The Ball of Swine encounter depends on the **concentric grooves and walls changing elevation**, and its peak artificial light effects are special boss-fight staging. The arena illustration must communicate geometry even with lights switched off.
- Goblin Engineer uses a **cooking pot helmet**; basic goblins later use pineapple-tipped sticks. The opening chase does not occur in the dense later workshop.
- Carl/Donut in any wide generated concept scene are **not** the final characters. Only approved 52/40 px source masters (and approved successors) are authoritative. No shotgun.
- Art-directed contrast **does not imply** a continuous physical route among every illustrated area; topology and area transitions remain gameplay/level-design tasks.

## Committed concept references — repository inventory

The selected concept files are now **committed on PR #31** under [`assets/concept/floor-01/atlas-v1/`](../../assets/concept/floor-01/atlas-v1/README.md). They include their source provenance, byte sizes and SHA-256 values in the [manifest](../../assets/concept/floor-01/atlas-v1/manifest.json).

**File verification (2026-11-16):** all ten GitHub Git-blob SHA-1 values were checked against the original reference-package PNG bytes and match exactly. The original package's SHA-256 values match the committed manifest entries. This verifies byte identity of concept images, **not** gameplay/visual-production quality.

Area 01 remains the previously committed [opening catacomb concepts](../../assets/concept/floor-01/opening/README.md) from PR #23, without duplication.

| ID | Committed concept image | Approval boundary |
| --- | --- | --- |
| Overview | [Ten-area art atlas board](../../assets/concept/floor-01/atlas-v1/00-floor01-art-atlas-overview.png) | Generated labels and topology are not source-of-truth |
| 02 | [Tutorial Guild — old guide's room](../../assets/concept/floor-01/atlas-v1/02-tutorial-guild-old-guide.png) | Owner's preferred variant, concept mood only |
| 03 | [Bad Llamas scorched corridor](../../assets/concept/floor-01/atlas-v1/03-bad-llamas-scorched-corridor.png) | Local scorch marks, not an all-floor lava biome |
| 04 | [Fast-food refuge](../../assets/concept/floor-01/atlas-v1/04-fast-food-refuge.png) | Fictional/nonbranded public-space treatment |
| 05 | [Hoarder's lair](../../assets/concept/floor-01/atlas-v1/05-hoarder-lair.png) | Clutter localized to the special room |
| 06 | [Goblin workshop](../../assets/concept/floor-01/atlas-v1/06-goblin-workshop.png) | Later industrial workshop, not the intro chase |
| 07 | [Slimes / Rot Stickers](../../assets/concept/floor-01/atlas-v1/07-slimes-rot-stickers.png) | Two separate hazard identities; depicted adjacency not canonical |
| 08 | [Circular maze and survivor camp](../../assets/concept/floor-01/atlas-v1/08-circular-maze-survivor-camp.png) | Spatial montage, not an actual navigable map |
| 09 | [Juicer's gym — provisional atlas excerpt](../../assets/concept/floor-01/atlas-v1/09-juicer-gym-atlas-excerpt.png) | **Not a standalone concept**; reception, turnstile and gym gameplay need focused design |
| 10 | [Ball of Swine arena — provisional atlas excerpt](../../assets/concept/floor-01/atlas-v1/10-ball-of-swine-arena-atlas-excerpt.png) | **Not a standalone concept**; moving-wall and concentric-groove geometry need a dedicated study |

All visuals are generated conceptual references only. **Do not crop them into editable/source sprites, gameplay colliders, enemy designs, or actual level geometry.**

### Review boundary

The owner approved the **general direction** of this environment batch, not final native-size texturing, palette, geometry, gameplay, hazard readability, camera placement, or engine screenshots. Gym/arena have only overview-panel references, *not* approved full-scene concept art. Specific boss/enemy silhouettes need separate sprite art approval.

## Production architecture: extensions, not ten independent tile sets

Keep the **base kit** single-sourced: masonry, floor, joints, arches, pillars, torch positions, interaction/collision cells, and parallax brick recession.

Build small independent extensions: **refuge**, **heat**, **90s public space**, **trash/horror**, **goblin industry**, **acid/adhesive hazards**, **rings/camp**, **gym**, and **moving arena**. Prefer meshes/static props and material overlays to duplicating the base brick family. Keep art and physics/collision masks in separate authoritative layers; render effects may not change deterministic gameplay.

For area transitions, ramp the introduced visual novelty in stages:
1. ordinary shared masonry;
2. a small trace of the upcoming faction/hazard;
3. a recognizable local prop/lighting clue;
4. a landmark or special room with the full kit;
5. return to the common corridor language for decompression.

**Do not immediately implement every kit.** The existing [COMP-01 PR #29](https://github.com/pando85/dungeon-floorbreak/pull/29) is the first in-engine scene and remains the visual/camera baseline. Preserve its scope (pixel layers only); lighting polish and area expansion are separate PRs.

## Gates for future area implementation

- [x] Owner likes broad first-floor art direction and selected one concept treatment per region.
- [x] Areas/canon distinctions, reuse plan and caveats documented.
- [x] Commit curated PNGs, provenance README and hashes; verified all 10 Git blobs against the original local reference files.
- [ ] Review/correct gym and final arena at standalone concept composition scale.
- [ ] Approved native editable environmental pixels produced for the relevant area.
- [ ] Real Bevy screenshots at 640×360 and 640×400 with **approved Carl/Donut** and no blurred/scaled source art.
- [ ] Deterministic collision/hazard/reachability tests (only where gameplay mechanics are introduced).
- [ ] Human judgement of game feel, camera, action legibility and atmosphere **on the playable integrated scene**.
- [ ] Rights check before distribution of any playable build or standalone asset pack.

The atlas is a **production map for art direction**, not a claim that every location or encounter exists in the game.
