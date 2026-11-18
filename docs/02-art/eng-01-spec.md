# ENG-01: Goblin Engineer readability set, design spec (stub)

**Status:** packet only. No design, no keys, no execution, no review yet. This stub fixes the canon, the exclusions, the size candidates and the body-reuse plan so the design step starts from facts. No height, frame size or pivot is decided.

The sections follow `docs/02-art/bard-01-spec.md` and `docs/02-art/gob-01-goblin-spec.md`. Pose design, the palette, the animation list and the in-between list come later.

## 1. Canon and tone constraints

### Approved concept

- Only one committed concept image shows the Engineer: `assets/concept/floor-01/opening/02-goblin-trio-reference-crop.png` (1080 x 365), right panel, caption "ENGINEER / LEVEL 3". `assets/concept/floor-01/opening/README.md` (table, row `02-goblin-trio-reference-crop.png`) records it as human-approved broad visual direction. `docs/02-art/floor-01-enemy-concept-roster.md` (row "Goblin Engineer", line 12) records the pot-helmet engineer as owner-approved, with "dedicated editable engineer/driver art and animation not yet verified as completed".
- The trio image has no pixel heights printed on it. The "44 px" figure cited by other specs is second-hand (see section 3).

### Canon citations (file and line)

| Fact | Source |
| --- | --- |
| Level 3, a cooking-pot helmet, a more tool-focused silhouette, visibly operates the Dozer. A wrench is a non-canonical visual shorthand; ranged bombs, buffs, in-combat repairs and class perks need a later gameplay decision | `docs/02-art/floor-01-opening-catacomb-goblins.md:50` |
| Concept use: green goblin shapes, expressions and the engineer's pot helmet; generated weapons, classes and pixel sizes are not accepted | `docs/02-art/floor-01-opening-catacomb-goblins.md:101` |
| Engineer (1): kitchen-pot helmet, level 3, operates the Dozer | `docs/07-agents/floor-01-opening-encounter-handoff.md:46` |
| Rider identity: a level-3 engineer in a kitchen-pot helmet sits at the rear controls in an upside-down saucepan, its handle sticking out back | `docs/02-art/doz-01-dozer-spec.md:33` |
| One hand on the throttle lever, the other arm pointing at Carl; has a lever-pull key | `docs/02-art/doz-01-dozer-spec.md:34` |
| Rider sprite `dozer_riders_h36` / `engineer_*`, frame 64 x 56, seat pivot (28, 55), 7 frames (point, jab, idle 0 and 1, lever pull, lurch back, lurch forward) | `docs/02-art/doz-01-dozer-spec.md:91` |
| `engineer_point` construction: GOB-01 idle body seated (rows shifted down 9, no pineapple), saucepan 6 rows deep and 14 px wide over a 1 px lip, handle out back (fix round D7) | `docs/02-art/doz-01-dozer-spec.md:211`, `:271` |
| Other engineer frames: lever pull (line 220), lurch back (line 222, pot hops 3 px, white eyes), lurch forward (line 224, pot slides 3 px over the eyes); visible rider rows are GOB-01 pixels translated whole, apart from the pot and arms | `docs/02-art/doz-01-dozer-spec.md:239` |
| Common goblins: green, about 4 ft, leathery rag clothing, sharp teeth, pointy ears; early on-foot weapon is a pineapple on a wooden stick, not a club; tone is a scrappy, nasty, slightly ridiculous threat with a readable tell, never a heroic orc | `docs/02-art/gob-01-goblin-spec.md:11-19` |
| No bombs in the opening trio: the opening goblins use pineapples on wooden sticks; the Engineer is not automatically a mine-layer or turret | `docs/02-art/bard-01-spec.md:21`, `:224` |
| The opening goblins carry pineapples on sticks; the board's spiked club is not approved for them; the Engineer is not turned into a turret or mine-layer | `docs/02-art/chop-01-spec.md:47` |
| The engineer is distinct from the Bard's performance silhouette | `docs/02-art/bard-01-spec.md:14` |
| Chapter 15's larger goblin engineering and workshop facilities are a different, later area: do not move their clutter or machinery into the opening encounter | `docs/02-art/floor-01-opening-catacomb-goblins.md:80` |
| Production order: batch 1, "Goblin Engineer standalone (delta of GOB-01 / DOZ-01 rider)" | `docs/07-agents/pixel-art-pipeline.md:43` |

### Tone

`docs/01-design/antihero-tone.md` applies: a believable, grimy, slightly ridiculous threat with a readable tell, not a heroic mechanic and not a cute mascot. Everything on the board that reads as "class", "level" or "role" is a label, not a mechanic.

### Canon facts that must survive in pixels (reading of the concept and the rider)

- Green goblin skin with the GOB-01 head construction: pointy ears, red eyes, sharp teeth, snarl.
- An upside-down kitchen saucepan jammed on the head, handle sticking out back (to the left in the concept and in the rider).
- Tool-focused kit visible on the concept figure: a wrench, a grey-blue pauldron on the near shoulder, a wrist wrap, a belt with a buckle, a rag tunic and trousers. Which of these are canon and which are board flourish is an open question (section 6).
- Same family as GOB-01 A/B: no pineapple on a stick, no spiked club, no bombs.

## 2. NOT approved (do not derive from the boards)

- The captions "ENGINEER / LEVEL 3", "CONCEPT ONLY", "NO GAMEPLAY SPRITE" and any printed class, role or level text. The level-3 fact itself is canon from the handoff, but not a stat.
- Any printed or implied size, including the "44 px" height cited second-hand in other specs.
- The wrench as a weapon or an attack. It is "a non-canonical visual shorthand" (`floor-01-opening-catacomb-goblins.md:50`).
- Any mechanic: ranged attacks, turret or mine-layer roles, bombs, buffs, repairs in combat, class perks, drops, a standalone on-foot encounter.
- The spiked clubs held by Goblins A and B on the same board.
- The board's pose, lighting and rendering. Crops are reference for identity only, never traced or exported.

## 3. Size candidates (estimates, not approved)

Carl is 52 px (about 6 ft), Donut 40 px, GOB-01 and the DOZ-01 rider 36 px (`scale_lineup.png` in the packet shows 36, 40 and 44 px slots). The rider's opaque height seated is smaller than 36 px; the standalone sprite stands.

| Option | Idle opaque height | Ratio to Carl | Note |
| --- | --- | --- | --- |
| A | 36 px | 0.69 | Same as GOB-01 A/B. The pot adds about 6 rows to a 36 px body, so the body would be shorter than A/B or the sprite taller than 36 |
| B | 40 px | 0.77 | Body at GOB-01 height plus the pot rows. Candidate for "level 3 reads as the most senior goblin" |
| C | 44 px | 0.85 | The "printed height" quoted by `bard-01-spec.md:33`, `:57` and `swine-01-spec.md:55`. Unverified on the committed crop. Close to the Bard's 43 px cap, so the route would likely score as Medium |

No decision is taken here. The decision belongs to the design step and an independent decider.

## 4. Body-reuse plan

- **Base body:** the GOB-01 variant A idle body and head (`assets/source/characters/goblin/goblin_a_h36.*`, palette `goblin-floor01-v0` in `gob-01-goblin-spec.md` section 7). Variant A and B are visual variants, not classes. The Engineer is a third look on the same family.
- **Rider helmet and gear:** reuse the DOZ-01 rider's saucepan (`pot_px` construction, 14 px wide, 6 rows deep, 1 px lip, handle out back) and its metal ramp from `dozer_riders_h36` (palette keys in `assets/source/enemies/dozer/dozer_riders_h36.grid.json`). The standalone sprite must keep that identity: same pot, same handle side, same green, same head.
- **New for the standalone:** legs and a standing pose, plus any tool kit that the design step takes from the concept (wrench, pauldron, belt). The rider sprite only shows the seated upper body.
- **Not reused:** the pineapple stick, rider A/B poses, the Dozer hull, lever and tub parts.
- **Palette:** the packet's `palette_candidates.json` and `palette_swatches.png` are median-cut candidates (most are background navy). The final palette is chosen by eye and capped at 20 colours.
- Reference for the match: `docs/evidence/eng-01/packet/rider_reference.png` shows the 7 DOZ-01 engineer rider frames at 3x, rendered from `dozer_riders_h36.grid.json` frames 0 to 6.

## 5. Packet inputs (`docs/evidence/eng-01/packet/`)

Crops of `02-goblin-trio-reference-crop.png` (x, y, w, h in the 1080 x 365 image):

- `concept_crop_1.png` = `735,70,290,160`: the whole Engineer figure
- `concept_crop_2.png` = `840,80,110,90`: the head, pot and face row
- `concept_crop_3.png` = `920,100,100,90`: the right arm, hand and wrench
- `rider_reference.png`: the DOZ-01 engineer rider frames (not a concept crop)
- `excerpts.md`: GOB-01 section 1, DOZ-01 section 1 and section 7.1
- `scale_lineup.png`, `palette_candidates.json`, `palette_swatches.png`, `frame_pivot_template.json` (64 x 56 and pivot defaults are placeholders), `spec_template.md`

The concept has no separate action study or face-detail row. Crop 2 and 3 are detail crops of the single figure.

## 6. Open questions

1. **Printed 44 px.** The Engineer's printed height of 44 px appears only second-hand in other specs. Is it on a board that is not committed, and does it count as an approved size? Default: it does not.
2. **Standing height.** A 36, 40 or 44 px body with the pot; the pot adds rows and the 43 px cap of the Bard route is only a work-order bound for that asset.
3. **Wrench.** Keep it as a carried prop and part of the silhouette, or leave it out? It must never become a weapon or an attack with new mechanics.
4. **Pauldron, wrist wrap and belt buckle.** Canon kit or board flourish? The rider sprite has none of them; they would also break "plain rag" family identity if overdone.
5. **Behaviour set.** The readability set is idle, move, tell, attack, hurt and defeat, but the Engineer has no approved on-foot combat role. Which attack does it use without inventing a weapon (a shove, a thrown tool, the pineapple stick)? The pineapple is canon for the opening goblins but not stated for the Engineer.
6. **Where it appears.** Is the standalone Engineer a Dozer survivor on foot after a crash, a separate encounter, or a cutscene figure? The answer decides how many gameplay tags are needed.
7. **Facing and handle.** The pot handle points left (back) in the rider and the concept. Mirror rule for a left-facing sprite.
8. **Pot colour.** The concept pot is lilac-grey with warm highlights; the rider ramp is blue-grey. Which one wins under the A1 torch light?
9. **Contact row and pivot** for a standing sprite, versus the rider's seat pivot (28, 55).

## 7. Rights and provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope (see `docs/04-production/rights-and-release.md`). Crops are reference only, never traced or exported. No third-party assets, no copied designs or dialogue, no image-model raster output.

Packet: script plus Claude Sonnet. Design, execution and review: pending.
