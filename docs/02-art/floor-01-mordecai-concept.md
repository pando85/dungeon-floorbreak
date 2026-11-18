# Floor 01 — Mordecai character concept (Rat Hooligan)

**Game:** THE CRAWL — NO PANTS. NO MERCY.  
**Status:** **OWNER-SELECTED CONCEPT DIRECTION** (2026-11-18); option **C — The Reluctant Mentor** is the character's Floor 01 identity, incorporating more natural rat anatomy from a reference the owner liked. This is concept-art approval, **not** native-pixel or animation approval.  
**Scope:** Mordecai as a supporting character in the **already selected Tutorial Guild environment**, not a new gameplay mechanic.

## Authoritative references

- Mandatory antihero tone: [antihero-tone.md](../01-design/antihero-tone.md).
- Art and scale: [art-direction.md](art-direction.md), [character-scale-approval.md](character-scale-approval.md).
- Floor 01 environment concept and location-specific approval: [floor-01-art-atlas-v1.md](floor-01-art-atlas-v1.md), especially Area 02.
- The actual approved room concept remains `assets/concept/floor-01/atlas-v1/02-tutorial-guild-old-guide.png`. **Do not replace it with any room illustration in Mordecai's character boards.**
- Carl and Donut: `assets/concept/carl/approved/`, `assets/concept/donut/approved/` and their respective character bibles. Native scale baselines are Carl **52 px** and Donut **40 px**; neither may be redesigned by this PR.
- Concept PNG intake and integrity records: `assets/concept/floor-01/mordecai/` (pending Hermes upload on this draft PR).

## Book 1 / Floor 01 anchors versus art interpretation

| Attribute | Book-grounded constraint | Selected artistic treatment / qualification |
| --- | --- | --- |
| Species on Floor 01 | Rat Hooligan; rat-like, humanoid and notably shorter than Carl | Grizzled gray/brown rat, natural muzzle and whiskers; slightly oversized ears for readability |
| Wardrobe | Black vest, blue trousers and worn sandals | Dented/torn edges, patches and muted brass buttons are artistic wear cues; exact pattern is not book-certified |
| Character | Gruff, tired, sarcastic, highly experienced guild guide; former crawler who reached Floor 11 | **C — reluctant mentor**; a guarded, responsible presence beneath the cynicism |
| Role | Tutorial Guild guide; level 50 in the context of his guild role | No combat animations, new skills, spells, inventory items or speculative gameplay abilities are approved |
| Personal history | Family photo and his mother's ashes/urn are important story elements | Intimate set dressing and subtle acting cues; no invented interactions or copyrighted dialogue |
| Future appearance | He changes form to a Bugaboo on Floor 02 (still Book 1) | May appear as a **separately labelled future reference** only; **not a Floor 01 form or an approved Floor 02 master** |

**Source boundary:** Earlier research compared the book 1 source-based Floor 01 atlas with fan reference summaries. Details should be rechecked against legally available original chapter text before production tagging. **Never commit novel text or external fan illustrations.** A third-party Rat Hooligan image admired during the design conversation influenced only the *desire for more natural rat proportions*; the image itself is **not included or licensed as a source asset**.

## Approved artistic identity — C (Reluctant Mentor)

Mordecai is a distinctly rat-like, weary and sarcastic veteran, **not** a high-fantasy wizard, superhero, comic-relief pet or combatant. His credibility comes from exhausted confidence, asymmetrical posture, practiced gestures and surprisingly human vulnerability. Keep the approved C head/eye/beard read and combine it with a somewhat more natural, less over-built rat physique.

- **Silhouette:** low shoulders, slight forward stoop, relatively broad ears, expressive long muzzle, ragged face fringe, prominent whiskers, readable tapering tail and non-heroic build. Avoid bulky human biceps, bodybuilding chest or heroic stance.
- **Face:** large tired eyelids over alert eyes; expressive eyebrows/upper fur, scruffy chin beard, slight asymmetry. No permanent sinister grin.
- **Costume:** black/brown worn vest, faded blue trousers and well-used sandals; no weapons, armor, magical staff, capes or invented class insignia.
- **Color/lighting:** ashen warm-gray fur, dusty pink skin on ears and tail, charcoal vest and indigo trousers; amber hearth edge light against blue-violet stone. Palette to be verified in genuine native pixels, not copied from the illustration.
- **Provisional native height:** around **44 px**; **NOT APPROVED** until actual 1× side-view silhouette and 640×360/640×400 cast check against the approved Carl 52 px / Donut 40 px masters.

### Expression and performance target (art only)

Show a shared identity across: weary/neutral, hungover irritation, dry sarcasm, irritated complaint, patient/exasperated explanation, skeptical pause, genuine concern, guarded grief and fleeting warmth. Body-action candidates: slumped idle, arms folded, one-handed explanatory gesture, pointing, seated exhausted pose, rubbing forehead, drinking from an ordinary vessel. **This is not authority to add dialogue or gameplay mechanics.**

### Tutorial Guild integration

Retain the approved *old guide's modest room*: hearth warmth, bed, books/shelves, small personal artifacts, and **an open central foreground**. Mordecai's gestural exchanges can be shown with **the approved** Carl and Donut identity references but new generated actor drawings are illustrative and **must never supersede the actual approved master sprites**.

The emotional trajectory is a visual-acting proposal, not a mandated scripted sequence: rude/bewildered first contact → reluctant instruction → glimpses of concern and loss. Avoid the generated board's claim that he immediately directs the duo to an arena; this is an illustrative caption, not a book-verified route or level-design decision.

## Approved source concept boards, with boundaries

1. `01-mordecai-selected-c.png` — owner-selected **character direction**, core silhouette and mood.
2. `02-mordecai-turnaround-acting.png` — supporting turnaround/expression proposal; exact pixels and scale not validated.
3. `03-mordecai-tutorial-guild-storyboard.png` — supporting performance and room composition; generated text, room changes, props and Carl/Donut likenesses are **non-authoritative**.

Original PNG bytes and SHA-256s are recorded in `assets/concept/floor-01/mordecai/manifest.json` and `SHA256SUMS` after Hermes uploads the assets. **Never extract final production sprites by cropping these images.**

## Approval and next gates

- [x] Owner selected option C and rat-anatomy refinement.
- [x] Floor 01 character direction and emotional range reviewed positively by owner, 2026-11-18.
- [ ] Import three exact concept PNGs with SHA-256 verification through Hermes (draft PR).
- [ ] Verify and merge concept-only PR after files, provenance, docs checks and review.
- [ ] Later separate work order: design *native* editable Mordecai idle/turnaround/acting keys with silhouette gate, independent art review and actual cast-scale checks.
- [ ] Later separate storyboarding and scene design: use the existing approved Tutorial Guild rather than generating a replacement environment.
- [ ] Rights and game-distribution review remains separate from this fan-concept art approval.

## Provenance and rights

The selected art is originally generated in the project's ChatGPT concept-design sessions on 2026-11-18, directed and chosen by the project owner. Exact generator settings are not embedded in the PNG files. No novel pages, copyrighted quotes, third-party Rat Hooligan references or scraped artwork are to be copied into the repository. Subject to the **owner-reported** `ART-PUBLICATION-ALL-V1` scope in [rights-and-release.md](../04-production/rights-and-release.md); this is not independently verified permission for a playable game, commercial release, marketing or a public asset pack.
