# Book 1 — Odette and Meadow Lark supporting-cast concept bible

**Game:** THE CRAWL — NO PANTS. NO MERCY.  
**Status:** **OWNER-SELECTED ARTISTIC CONCEPTS**, 2026-11-18; all five original PNGs uploaded by Hermes and independently verified against the source handoff ZIP (Git blob SHA-1 / SHA-256, bytes, dimensions) on 2026-11-18. Not canon-verified production sprites, animation masters, character scales, gameplay contracts or environment redesigns.  
**Scope:** Odette (Book 1 post-Floor-01 interlude), Imani, Elle McGibbons, Brandon Andrews and Chris Andrews (Meadow Lark / Floor 01). Mordecai is already independently approved and versioned in [his separate concept bible](floor-01-mordecai-concept.md).

## Authoritative in-repository art constraints

- [Art direction](art-direction.md), [mandatory antihero tone](../01-design/antihero-tone.md), [native character scale decisions](character-scale-approval.md).
- The [Floor 01 atlas](floor-01-art-atlas-v1.md), including the Meadow Lark camp context and the distinct Tutorial Guild setting.
- [Carl's approved bible](carl-character-bible.md) and [Donut's approved bible](donut-character-bible.md), including their original art and fixed **52 px Carl / 40 px Donut** opaque heights. Never derive replacements from background illustrations.
- [Rights and publication policy](../04-production/rights-and-release.md). This document is a fan-concept design record, not an asserted adaptation license.

## Source hierarchy and visual-approval boundary

Book 1 text, where available for authorized private review, is the primary authority on physical appearance, introduction, chronology and equipment. Public fan-character indexes are *corroboration only*, not a license to invent or to import fan art. The generated images and their captions may contain mistakes, and **do not** independently establish canon. All unverified wear, accessories, silhouettes, colors, height labels, prop shapes, room floorplans and story captions remain **artistic proposals** even when the owner likes the image.

Original independent concept illustrations, not copied book pages, official artwork or existing fan artwork, are registered in `assets/concept/book-01/supporting-cast/`. Exact byte hashes and dimensions are enumerated in `manifest.json`; PNG upload and byte identity verification are complete for all five images; final CI and PR review are separate release gates.

## Odette — glamorous masked presenter, exhausted older woman in private

**Story placement:** First appearance in **Book 1 after Carl and Donut descend from Floor 01, during the television interview on Floor 02**. Her design belongs to Book 1's *post-floor interlude*, **not** a physical NPC on Floor 01 or an in-dungeon combatant.

**Book-grounded identifiers:**
- On camera: crab-taur show-host persona, human upper torso, an enormous **king-crab lower body** with red/black legs, ornate gold armor and a black **praying-mantis helmet** with very large mirrored compound eyes and long antennae.
- Off camera: a woman appearing about sixty with hair gathered in a bun, widely spaced eyes, a heart necklace and long nails. Her actual body is **legless** and she uses a **hovering disk** for mobility. The insect head is a helmet, **not her true face**.
- Strong theatrical charisma, private depth and experience as a former crawler. Do not introduce new magical or combat powers.

**Owner-selected treatment:** keep the crab legs chosen from earlier **C**, the elegant older private face chosen from **B**, and the masked mantis head in public. Do not reintroduce the rejected blond, young, feather-boa diva reinterpretations.

**Illustration qualification:** the selected board shows both the masked upper body and older private facial portraits; it does **not** fully depict the private legless disk configuration. That future composition must be verified before production. Its 'Floor 1' header is a *batch label only*, not the interview's canonical location.

## Meadow Lark — physically vulnerable people, practical staff, improvised survival

All four concepts below are **human Floor 01 identities**. Do not import later-floor class, race, gear or powers. The group should read as an unlikely cluster of elder-care workers and elderly residents, not a superhero combat team.

### Imani

- **Book anchors:** young Meadow Lark staff member; visibly frightened but protective of residents; scrubs and a sword feature in the Book 1 narrative. Keep the tension between duty, fear and emerging resolve.
- **Owner-selected visual:** tired, watchful dark-skinned young woman with practical gathered hair, teal scrubs and wounded-but-still-caring poses.
- **Artistic interpretation (not canon-certified):** ragged red cloak, detailed bandages, stethoscope, boots, accessory colors, blood marks and any exact blade design. Do not promote illustrations of abilities, loot or later transformations into Book 1 facts.
- **Scale:** `~46 px` printed on the generated reference is only a proposal.

### Elle McGibbons

- **Book anchors:** elderly Meadow Lark resident (described as extremely old, approximately 99); human and wheelchair-bound in the first-floor sequence. She has wit, defiance, emotional vulnerability and strong personhood. 
- **Owner-selected visual:** gray/white curled hair, lined expressive face, regular wheelchair, blanket and warm cardigan. Humor, tiredness, affection and grief coexist.
- **Artistic interpretation:** specific cardigan color, blanket pattern, jewelry, wheelchair branding and precise dimensions. The board's `122 cm seated` label is not a validated factual measurement.
- **Explicit exclusion:** no Frost Maiden form, costume or cold powers.

### Brandon Andrews

- **Book anchors:** Black man, stocky/shorter than his older brother Chris; warm, friendly and talkative; a Meadow Lark maintenance worker who helps protect residents. The source narrative includes a conspicuously large hammer.
- **Owner-selected visual:** compact sturdy silhouette, natural curly hair, facial hair, genuine friendly smile, practical maintenance gear with a jacket, and strongly expressive gestures.
- **Artistic interpretation:** fur-trimmed red jacket, company badge, exact trouser cargo-pockets, bag/belt contents, apparent height `163 cm`, and the hammer's precise physical construction. The generated prop could be confused with a shovel: **redraw/validate the hammer silhouette separately in native pixel art**.
- **Never depict him as a bodybuilder or a generic warrior.**

### Chris Andrews

- **Book anchors:** Black man and Brandon's **older brother**; visually similar but taller and less stocky; also a Meadow Lark maintenance worker. His comparatively reserved demeanor makes a readable contrast.
- **Owner-selected visual:** taller, leaner proportions, related facial features, shorter curly hair, serious expressions, a cautious rather than heroic stance.
- **Artistic interpretation:** matching fur-trimmed red work jacket, backpack, particular tool pouch, exact height `178–180 cm`, specific hair and beard layout. Book 1 mentions practical hauling gear; **confirm the correct equipment at the scene before freezing sprites**.
- **Explicit exclusion:** no race changes, later-floor transformations or invented abilities.

## Shared artistic review and next stages

1. **Complete:** one exact full-resolution selected concept board per character, with SHA-256, Git blob SHA-1, dimensions and original filename verified against the original handoff ZIP. Do not crop AI pseudo-sprites from the boards.
2. **Chronology gate:** distinguish floor 1 Meadow Lark survivors from Odette's post-floor interview. Avoid a shared Tutorial Guild scene that puts them together against the book.
3. **Canon gate:** verify faces, sibling relationship, shape/sizing and important equipment before exporting any canonical native-pixel master. Treat all text generated into illustrations as non-authoritative.
4. **Native scale gate:** Carl 52 px and Donut 40 px are **approved**; NPC sizes printed in these five images are *not*. Validate new 1× silhouettes in 640×360 and 640×400 frames with proper pivots.
5. **Separate production gate:** editable Aseprite / sprite metadata, independent top-tier review and in-scene evidence belong in later pixel-art work orders; this PR contains reference art and specifications only.
6. **Rights gate:** owner-reported `ART-PUBLICATION-ALL-V1` permits public-repo fan-character art as represented by the owner, not game distribution, trailer/asset-pack licensing or independent proof of rights clearance.

## Concept image register

| Image | Status |
| --- | --- |
| `01-odette-on-air-and-unmasked.png` | **Selected** B-face / C-crab-legs / black-mantis-mask direction; private hover-disk body still needs a future study |
| `02-imani-meadow-lark.png` | **Owner-liked** emotional range / practical staff identity; exact accessories require canon audit |
| `03-elle-mcgibbons-meadow-lark.png` | **Owner-liked** elderly human wheelchair concept |
| `04-brandon-andrews-meadow-lark.png` | **Owner-liked** friendly stocky younger brother; hammer shape requires later refinement |
| `05-chris-andrews-meadow-lark.png` | **Owner-liked** taller and more guarded older brother |

Only the direction and original concept paintings are approved here; not gameplay, final geometry or sprite deliverables.
