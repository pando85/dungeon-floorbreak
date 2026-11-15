# Princess Donut — approved visual identity and evolution bible (fan-content layer)

**Decision:** CREATIVE-2026-11-15-DONUT-01  
**Status:** **Visual identity and the eight-stage artistic evolution APPROVED in conversation** (2026-11-15). The full-resolution reference PNG must be uploaded and source-verified before this package is considered complete; the item timeline and generated captions are **not canon-approved**.  
**Character / runtime mapping:** Princess Donut (fan character) → IP-neutral `caster`.

## Selected visual reference

- **Approved visual:** Princess Donut, *Character Evolution & Gear Bible* (2026-11-15), proposed file `assets/concept/donut/approved/Princess_Donut_Evolution_Gear_Bible_v1.png`.
- **Current asset status:** **Pending upload into Git**. The reference remains in the owner's project conversation. See [asset register](../../assets/concept/donut/approved/README.md) and [provenance](../../assets/concept/donut/approved/provenance.md) for the exact source identity and upload gate.
- **Visual approval means:** faithful, irresistible Persian-cat charm; a continuous recognizable character over all eight design phases; more extravagant magic and equipment while retaining the same face and body.
- **Visual approval does NOT mean:** the image's tiny icons, labels, quoted stats, item acquisition floor, class labels or apparent in-game pixels are independently verified from the novels. AI-generated typography is not a source of lore.

## Locked anatomy and personality

**An adorable, fluffy tortoiseshell Persian show cat who happens to be proud, calculating, theatrical, manipulative, fiercely loyal and capable of spectacular anger.** Cuteness is the default; rage gains power from contrast.

- **Body:** compact, low-to-the-ground, broad fluffy outline with short legs; never a long-legged sleek cat, upright biped or large armored beast.
- **Face:** pronounced Persian short muzzle, round cheek/ruff silhouette, tiny pink nose and very large luminous yellow-golden eyes. Keep her *cute in neutral*, not in a perpetual sneer or snarl.
- **Coat:** luxurious long tortoiseshell fur, dark and warm ginger/cream irregular patches, with a pale fluffy face/chest treatment in the approved *artistic rendition*. Preserve a stable identifying patch map across views and stages; verify any literal color claims against primary book text before tagging them canon.
- **Tail:** oversized, fluffy and legible at a distance; secondary animation communicates pride, surprise and intent.
- **Personality:** smug when it serves a joke, charming when she needs something, analytical when planning, outraged when crossed and terrifying when releasing magic. Never replace the adorable base portrait with the fury portrait.
- **Identity anchors:** small purple-gem tiara, dainty butterfly charm/collar, giant eyes, plush tail and consistent coat patches. Decorative shoulder/scale protection is restrained. Jewelry never obscures face or walking silhouette.

## Eight visual phases — ART direction, not verified loot chronology

| Visual phase | Narrative label in concept | Visual treatment | Verification boundary |
| --- | --- | --- | --- |
| 1: Floors 1–2 | Princess | Same adorable round face; compact tiara, collar/charm and small foreleg band; modest scale adornment | Check exact start floor and acquisition of every prop |
| 2: Floor 3 | Star | More expressive, delighted theatrical posing; no body redesign | Do not infer class unlock date from illustrated label |
| 3: Floor 4 | Strategist | Calculating expression and optional prism-glasses overlay; stable coat and crown | Confirm which glasses/item exist and when |
| 4: Floor 5 | Spellcaster | Controlled magenta magic around paws; no extra armor; magic effects can be toggled off | Class/title and intensity not automatically canon |
| 5: Floors 6–7 | Diva | Singing/commanding animation language; bracelet visual identifiers become more important | Check every bracelet and floor independently |
| 6: Floor 8 | Summoner | Cards/phone-style abstract magical UI motifs as an optional gameplay presentation | Separate canon summons, abilities and metaphoric UI |
| 7: Floor 9 | Champion | More imposing golden aura and tiara detail, but still the same kitten-like face | Exact tiara design and stats require verification |
| 8: Floors 10–11 | Veteran | Optional darker accessory/veil and precision-oriented casting posture | Do not present generated assassin gear as confirmed canon |

The **floor labels are storyboard buckets**. If a book event, class change or item acquisition falls between visual milestones, implement the verified event in the actual item timeline instead of rewriting the source to match the board. Never silently change base anatomy or coat between phases.

## Foreleg bracelets, anklets and equipment readability

Bracelets are valuable **continuity markers**, not a reason to bury her in trinkets. The illustration depicts a progression from a discreet early foreleg band to darker, metallic, glowing bands.

- Maintain separate equip slots in documentation for `front_left`, `front_right`, `hind_left`, `hind_right`, and collar/head/back as needed; **slot names are proposals**, not a claim about the book's inventory rules.
- Display at most **one visually dominant foreleg band** in typical combat frames. A second bracelet may be indicated by a contrasting tiny highlight or inspect portrait; keep other bonuses in inventory/UI.
- Make material/material effect readable without relying on color alone: narrow polished metal versus dark faceted obsidian, with a brief spell-trigger highlight.
- Different powers are communicated primarily by effects and movement staging; avoid permanent overpowering bloom on the face.
- Confirm **source spelling, acquisition floor, relevant slot, active duration and replacement vs simultaneous equip** before encoding any game stat. See the [gear research tracker](donut-gear-timeline.md).

## Expressions and animation acting

**Face states:** adorable/curious (default), proud/smug, thoughtful/calculating, charming/manipulative, indignant, furious/magical. Extend with sleepy, startled and happy poses as required.

**Provisional gameplay sprites:** around **28–36 virtual pixels high** (not literal pixel art in the concept board). Test at 1× native 640×360 and 640×400, 2× on 1280×800, and 3× at 1920×1080. Freeze identical face proportions, ear positions, key coat patches and foot pivots across animation frames.

**First animation scope:** `idle`, `walk`, `run`, `jump`, `land`, `cast`, `hurt`, `sit_rest`, plus optional `smug_idle` / `furious_cast`. Genuine Aseprite-authored frames required; generated board sprites are illustrative only.

**Behaviour priority:** neutral cute → deliberate expression change → spell anticipation → dramatic payoff → recovery to cute. This preserves both emotional range and comedy.

## Production and QA gates

1. **Source/reference:** restore the exact approved image bytes and verify the recorded SHA-256. Do not substitute a cropped, recompressed or newly generated lookalike as the master.
2. **Lore:** audit the equipment tracker against primary book passages, recording title/chapter/floor and explicit uncertainty; keep any artistic interpretation separate.
3. **Model sheet:** validate 5 consistent views (front, 3/4, side, 3/4 back, back), fixed coat map and proportions.
4. **Sprite:** create a real editable `.aseprite` source, export deterministic atlas/JSON and test with the actual dungeon foreground/background at native scale.
5. **Acceptance:** compare F1–2 and a later gear variant in the **same gameplay camera**. Confirm she remains instantly recognizable and adorable, even in low pixel counts.
6. **Rights:** this fan-concept artwork may be tracked under the owner's reported publication scope, but public playable build / franchise license is a separate gate. See [rights policy](../04-production/rights-and-release.md).

## Agent rules

**DO NOT** regenerate Donut's base anatomy, migrate her toward an angry queen-face, change her fur patches per stage, invent inventory history from the illustration, or copy the illustrated miniature sprites into the runtime atlas.

**DO** preserve the approved adorable identity, represent bracelets in small restrained increments, cite source-verified lore, and explicitly tag entries as `canon-verified`, `art-approved`, `needs-verification`, or `noncanonical-proposal`.

Related: [Carl bible](carl-character-bible.md), [art direction](art-direction.md), [gameplay characters](../01-design/characters.md), [asset pipeline](asset-pipeline.md).
