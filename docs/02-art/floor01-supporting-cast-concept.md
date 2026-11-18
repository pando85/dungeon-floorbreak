# Book 1 — Supporting cast concept batch: Odette, Zev, Imani and Elle

**Game:** THE CRAWL — NO PANTS. NO MERCY.  
**Status:** **DRAFT COLLECTION** — Odette's latest canonical hybrid direction selected by owner (2026-11-18); Zev has **one unapproved first sketch**; Imani and Elle are **research briefs only** pending accurate concept art. This document is not an approval of all four characters, nor production sprites, gameplay code or camera.

## Scope, chronology and approved sources

Keep distinct story locations. **Imani and Elle** appear with Meadow Lark **on Floor 01** (Book 1, approximately chapter 22); **Odette and Zev** belong to the **post-Floor-01 / early Floor 02 Book 1 epilogue**, not the Tutorial Guild or a physical character encounter inside the first floor. The exact staging order must follow the first novel before scripting.

Read `AGENTS.md`, `docs/01-design/antihero-tone.md`, `docs/02-art/art-direction.md`, `docs/02-art/character-scale-approval.md`, `docs/02-art/floor-01-art-atlas-v1.md` and `docs/04-production/rights-and-release.md`. Carl 52 px, Donut 40 px and already-approved Floor 01 environments **remain unchanged**. All pixel art sizes beyond those remain hypotheses. Do not make a false floor map by placing an interview studio in the catacombs.

## Odette — selected direction (approval limited to character look)

**Book 1 anchors:**
- On camera: an ornate, theatrical **crab-taur costume**, a massive king-crab lower body with distinctly **red/black segmented legs**, gold upper-body armor, and a large **black praying-mantis helmet** with mirrored compound eyes and long antennae. This is **a helmet**, not an insect face permanently replacing her own.
- Unmasked: an elegant, clearly **older (~60) woman**, slightly wide-set eyes, hair in a bun, long painted nails, dark clothes and heart pendant; in private she has **no legs** and uses a **hover disk**. Keep the off-air form physically distinct from the on-air crab costume.
- Tone: glamorous, sharp, capable of manipulating a televised spectacle, but not merely an irredeemable villain. Distinguish show-host acting from private humanity without implying invented powers or staging.

**Owner's specific selection:** first designs' ornate **red king-crab legs (C)** plus the **older but beautiful unmasked face (B)**, preserving the mantis helmet, antennae and glamorous presentation. Final source board: `01-odette-selected-canonical-hybrid.png`; character-direction approval only. **Important production caveat:** this board shows an elderly face in close-up but does not fully show her authentic *legless hover-disk* private form. That accurate off-air view should be developed **separately before production**. Do not turn her into a blond-haired unmasked spider-woman or a humanoid with permanent crab legs.

## Zev — first exploration, not yet approved

**Book 1 anchors:** Kua-tin, a tiny fish-headed aquatic woman, approximately two feet tall, an exhausted/cautious Borant communications liaison. Her first appearance features a **water-filled globular glass helmet** on a bulky **1950s sci-fi / diving-suit** life-support outfit. She is assigned to Carl and Princess Donut after their initial media exposure.

**Sketch:** `02-zev-book1-exploration.png` explores the literal fish face, globe of water, retro suit, and corporate exhaustion. **Unapproved:** stylized fish fins, suit colors/panels, decals, props, height `24–26 px`, expression choices and imagined environmental shots. Compare at true native camera scale before selecting a master. No abilities or violence are designed.

## Imani — Book 1 / Floor 01 research brief, no approved art yet

**Book 1 anchors:** young, very slight Meadow Lark caregiver in human form, already exhausted by protecting nursing-home residents. Serious and stressed, practical clothes/scrubs and restrained body language. A sword and crawler-killer skulls occur in Book 1 context, but must not be sensationalized or confused with her personality.

**Design goals:** show a believable young care worker thrust into a catastrophe; exhausted, compassionate and alert rather than superheroic. Study `neutral / worried / protective / resolve` acting with other survivors. **Do not add butterfly wings, skull-style makeup, mystical fire effects or class costume**: those are later-floor transformations. Face details, ethnicity, specific colors and extra equipment remain **artistically exploratory unless verified from the novel**.

## Elle McGibbons — Book 1 / Floor 01 research brief, no approved art yet

**Book 1 anchors:** approximately **99 years old**, a nursing-home resident with a **wheelchair**, frail but an individual with a recognizable personality. Book 1 descriptions include an ill-fitting football cap and assisted travel with Meadow Lark. She belongs to the vulnerable group before her later changes.

**Design goals:** humanize, never caricature: strong expressive eyes, age-accurate anatomy, plain eldercare clothing, credible wheelchair dimensions and safe accessible silhouette; distinguish quiet confusion from humor or resilience. Preserve grounded first-book presentation. **Do not portray her later fairy / magical transformation, fantasy costume, combat powers or floating body on Floor 01.** Avoid copying licensed football team marks from any reference.

## Environment and narrative integration

1. **Meadow Lark Floor 01:** show Imani managing a realistically staged group including wheelchair users, with Elle as an individual, in the already established *ring/camp* catacomb mood. This is **not** a new approved exact layout.
2. **Book 1 epilogue:** Odette's controlled on-air TV studio is a **separate broadcast presentation**, with Zev as corporate contrast. Never combine these with the Tutorial Guild's interior or imply Zev walks through the first-floor corridor.
3. Keep 1980s–1990s action-film flaws, dark comic tension and human stakes; shots must not override approved Carl/Donut identities.

## Intake inventory and approval gates

| File | Status | Provenance |
| --- | --- | --- |
| `assets/concept/floor-01/supporting-cast/01-odette-selected-canonical-hybrid.png` | **Owner-selected** character direction | Original ChatGPT project conversation image; expected hash in manifest |
| `assets/concept/floor-01/supporting-cast/02-zev-book1-exploration.png` | **Exploration only** (await owner review) | Original ChatGPT project conversation image; expected hash in manifest |
| Imani concept board | **Not yet created / approved** | Primary text/physical design review first |
| Elle concept board | **Not yet created / approved** | Primary text/physical design review first |

- [x] Separate post-Floor-01 characters from Floor 01 Meadow Lark.
- [x] Record user-approved Odette anatomical assembly and form boundaries.
- [ ] Import the two exact original PNGs in this PR via Hermes, and verify SHA-256, byte lengths and Git blob IDs.
- [ ] Produce Book-1-accurate Imani and Elle individual concept boards, review with owner, and add only selected outputs to this same PR (or a later PR if scope grows).
- [ ] Review Zev and seek owner approval or specific requested changes.
- [ ] Validate documentation, image manifest, repo CI and external-IP provenance before the PR can leave draft.
- [ ] Native pixels and Bevy implementation happen in different PRs.

## Source confidence and intellectual property

Book 1-related facts were cross-checked against existing repository Floor 01 chapter-research and public spoiler-aware character records; the original licensed novel remains the source of truth for disputes. **No fan illustration, book scan, exact dialogue or offsite image** is copied into these artwork files. Generated caption text, brand labels and accidental look-alikes are **not canon**.

The repository's owner-reported `ART-PUBLICATION-ALL-V1` covers concept art in this public community repo according to its own rights register; that statement does not independently prove adaptation rights and does not cover distributing a game, trailer or standalone art pack.
