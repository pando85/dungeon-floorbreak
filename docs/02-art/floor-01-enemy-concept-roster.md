# Floor 01 — Owner-approved enemy concept roster

**Decision:** On 2026-11-17 the creative director approved the **visual concept designs for all enemy entries below**. None are merely “semi-approved” or waiting for another conceptual selection. Editable production sprites, exact dimensions, animation and Bevy integration are **separate work**, not part of this approval.

**Scope:** THE CRAWL — NO PANTS. NO MERCY. Use original editable pixel-art sources for production in Rust + Bevy; existing 640×360/640×400 native views; Carl 52 opaque px, Donut 40 opaque px.

## Approved visual roster and production status

| Character | Approved visual concept | Current production state |
| --- | --- | --- |
| Common Goblins A + B | Existing `assets/concept/floor-01/opening/02-goblin-trio-reference-crop.png`; expressive green, pointy ears, different faces and clothing | **Native 36 px A/B editable source and 27-frame sets already in main** (see [GOB-01 spec](gob-01-goblin-spec.md) and `assets/source/characters/goblin/`). Independent art review passed; production owner review and full in-engine integration are distinct |
| Goblin Engineer | Pot-helmet engineer/operator in the existing opening trio | Dedicated editable engineer/driver art and animation not yet verified as completed |
| Goblin Murder Dozer | Existing `assets/concept/floor-01/opening/03-murder-dozer-reference-crop.png`: rusted tracked steam vehicle, giant spinning spiked wheel, smoke and riders | DOZ-01 [PR #47](https://github.com/pando85/dungeon-floorbreak/pull/47) was in design/review at preparation time; inspect current state before starting duplicate work |
| Bad Llama | **C+A approved blend:** dynamic aggressive body (C), long expressive neck (A), yellow teeth, glowing-throat wind-up | Native pixel master, scale and animations pending |
| Scatterer | Small cockroach/insectoid visual direction | Native production pending |
| Scatterer Brood Guardian | Larger form of the same visual species | Native production pending |
| Rot Sticker | Small dark rounded wall/ceiling-running threat | Native production pending; don't default to a large spider silhouette |
| Green Slime | Corrosive gelatinous green creature | Native production pending |
| The Hoarder | Group concept visual direction approved by owner | Dedicated boss design-to-production interpretation pending; see important source identity warning below |

Concept approval is **not** approval of all text generated *inside* images, exact enemy sizes, gameplay behaviors, or source accuracy. Preserve user-approved PNGs **unchanged**, and document discrepancies instead of silently altering their pixels.

## Later approved enemy and boss concepts

**The Hoarder study, Troglodyte Pygmy/Basher/Virtuoso, The Juicer, The Ball of Swine and its Tusklings** have subsequently received the owner's visual concept approval. The specific images, caveats and remaining minor enemies are recorded in the [late-Floor 01 approval supplement](floor-01-late-enemy-boss-concepts.md) and three new entries in the enemy image manifest. The original five images and their approvals remain unchanged; the three newer PNGs have also been uploaded, verified against the original bytes and recorded in the eight-image manifest.

## Five original concept images — uploaded and verified

**The owner uploaded all five PNGs into `assets/concept/floor-01/enemies/`.** Each GitHub blob SHA-1 matches the blob identity independently calculated from the corresponding PNG in the original source package. The original package bytes also match the recorded SHA-256, byte size and image dimensions; these checks verify that the uploaded images are **byte-identical**, not their creative/canonical fidelity.

| Required filename | Approved content |
| --- | --- |
| `bad-llama-abc-exploration.png` | Historical A/B/C comparison; selected C+A supersedes them |
| `bad-llama-c-plus-a-approved.png` | Selected final Bad Llama concept with attack states |
| `scatterers-rot-sticker-slime-bestiary.png` | Scatterer, Brood Guardian, Rot Sticker and Green Slime visual concept direction |
| `floor01-bestiary-exploration-sheet.png` | Additional approved enemy visual exploration |
| `floor01-enemy-lineup-approved.png` | Approved enemy ensemble visual presentation including Hoarder |

The accompanying `assets/concept/floor-01/enemies/manifest.json` specifies the **exact SHA-256, bytes, dimensions and source conversation filenames** for all five. These were checked against the original ZIP supplied in this project. Do not replace them with screenshots, recompressed or cropped copies.

**Do not duplicate** the existing Goblin/Dozer reference PNGs from `assets/concept/floor-01/opening/`.

## Interpretation and source guardrails

- The early on-foot common goblins use **pineapples on wooden sticks** rather than metal maces. Common A and B are visual variants, not an approved brute class. The pot-helmet engineer is not automatically a mine-layer or combat turret.
- A generated label indicating **Scatterers explode on death** is not an approved gameplay feature.
- **Slimes split on hit** is not automatically an approved feature merely because it appears in a caption.
- Rot Stickers are **small, black, rounded wall/ceiling creatures**, not conventional large spiders; use the concept boards for mood, not every illustrative anatomical detail.
- **The Hoarder is not a goblin with a backpack.** Some ensemble imagery draws a goblin-like monster; the source identity as a transformed human requires separate narrative-aware boss production review. This does not downgrade the owner's approval of the image as visual concept direction.
- Printed scale comparisons in the generated boards are not accepted native sprite dimensions.
- No pre-Guild Donut spells, Carl shotgun, or story-premature explosive equipment because a test scene or image suggests it.

The [opening art direction](floor-01-opening-catacomb-goblins.md), [first-floor art atlas](floor-01-art-atlas-v1.md) and [combat-first encounter direction](../01-design/combat-first-encounter-design.md) remain authoritative for their respective domains.

## Follow-on production order

1. **Goblins + Dozer:** finish/review ongoing GOB-01 and DOZ-01 and synchronize with the current combat art integration PR. See [issues #40](https://github.com/pando85/dungeon-floorbreak/issues/40) and [#41](https://github.com/pando85/dungeon-floorbreak/issues/41).
2. **Bad Llama:** original native-pixel rendition of C+A, scale and glowing-throat/spit/stagger motion.
3. **Scatterer and Brood Guardian:** consistent family palette and distinguishable animations.
4. **Rot Sticker:** small native silhouette, wall/ceiling telegraph and contact readability.
5. **Green Slime:** gelatinous movement and obvious corrosion cues; no unapproved splitting.
6. **The Hoarder:** narrative-identity-aware boss art specification and scale/animation study.

For each production artifact require editable `.ase` / pixel-grid source, deterministic export manifest, relevant pixel/animation tests, real Bevy captures at 640×360 and 640×400, and **separate human acceptance of final production pixels**.

**Rights boundary:** This concept acceptance is not independent legal clearance to distribute a game or standalone asset pack. See [rights and release](../04-production/rights-and-release.md).

## PR acceptance

- [x] All enemy visual concepts recorded as **owner approved**
- [x] Images' hash and byte specifications checked against source ZIP
- [x] Current main production goblin A/B state accounted for; references not duplicated
- [x] Owner committed all five unchanged PNGs to this PR's branch
- [x] All five uploaded Git blob identities match the original ZIP bytes, SHA-256, sizes and dimensions
- [ ] Relevant CI passes on the **final PR head**
- [ ] Only then merge
