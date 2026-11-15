# Carl — approved character concept bible (fan-content layer)

**Decision:** CREATIVE-2026-11-15-CARL-01
**Status:** **Visual design APPROVED by project owner** on 2026-11-15; the owner also confirms permission to publish **concept images in the public community repository**. This does not approve production sprites, independently verify underlying IP rights, or authorize public distribution of the game.
**Scope:** The Carl fan-character design only. Runtime actor ID remains `demolitionist`; no franchise-specific artwork or name may be necessary for the core engine.

## Selected concept reference

- **Canonical concept source:** `assets/concept/carl/approved/Carl_Concept_Approved_v1.png`, **pending binary upload to the artwork-publication PR**. Read the [asset README](../../assets/concept/carl/approved/README.md) and [provenance](../../assets/concept/carl/approved/provenance.md) for the approved master and licensing boundary.
- **Source image SHA-256:** `b90fb91c3995449cd4a3c909c8407e767de3c706bcc4910c9ef1d7a69de50e31`.
- **Image size:** 1671×941 RGB; concept-art board, NOT an animation atlas, not authentic fixed-grid gameplay pixel art.
- **Provenance:** AI-generated illustration iterated and explicitly selected by the human project owner; generated tool output (model/version and full source prompt not embedded in the file).
- **Rights gate:** The owner explicitly confirmed that permission covers publishing **concept images** in the public GitHub community repository (2026-11-15). This is a user-reported permission, not independent verification; it does not cover distributing a game or other assets. See [rights register](../04-production/rights-and-release.md).
- **Illustration caveat:** Text and icon details in an image generation are not authoritative design data. The incorrect late-stage date label in the selected illustration and incidental cigarette / stubble / heroic abdominal highlights are **not** decisions.

## Locked look — no further global redesign required

**An ordinary-looking, stubborn, slightly unlucky man who earns his antihero presence, never a superhero.** 1980s–1990s action-film energy comes from the *disaster, improvisation, irritation, explosive payoffs, and stubborn humanity* rather than a Hollywood-perfect jaw or exaggerated muscle anatomy.

- **Face:** plain and recognizable; somewhat square/soft features, expressive brows and eyes, mild facial asymmetry; not conventionally glamorous, not a chiseled leading-man portrait.
- **Body:** believable tall/strong build, a little imperfect/soft early on; modest gains in practical strength, composure and endurance across stages. No six-pack presentation, broad V-torso, bodybuilding chest, or huge arms. Do not diminish Carl into a physically tiny or frail person either.
- **Hair:** brown; **same short messy silhouette** gradually becoming slightly longer and fuller, never spontaneously changing to an unrelated hairstyle. **No long hair/ponytail in the baseline four-stage lineup.**
- **Clothing anchors:** heart-patterned white boxers, battered jacket / layered equipment and appropriate early-story pink Crocs, transitioning to barefoot where consistent with the book timeline. Gear is mismatched, useful, and progressively worn; story-canon accuracy always wins over visual decoration.
- **Pose and expression:** stage I surprised / irritated, stage II alert / stubborn, stage III exhausted / pragmatic, stage IV composed / commanding but still recognizably Carl. Never universally smug or heroic.
- **Palette:** brown hair, believable skin, nearly black leather, off-white and red hearts, orange explosive highlights and cold violet dungeon contrast. Palette remains provisional until a genuine small-scale sprite test.

## Four development phases — user-approved direction

| Art stage | Loose story grouping (not a source of canon) | Hair | Face/body/attitude | Gear/environment |
| --- | --- | --- | --- | --- |
| I: Survivor | Book 1 | Short, uneven, tousled brown | Regular young adult, bewildered, irritated, still capable; slightly awkward stance | Jacket, heart-print boxers, recognizable opening items as verified by source |
| II: Anarchist | Books 2–3 | A touch fuller, spiky/untamed edges | More alert, slightly more athletic from experience; still messy and impulsive | Practical tools and gauntlet only when acquired in canon |
| III: Veteran | Books 4–5 | Just longer in the front/around temples, maintaining the same silhouette | Tired eyes, small scars, economical movement, earned confidence | Increasingly worn, repaired, asymmetrical equipment |
| IV: Reluctant leader | Books 6–8 | Shaggier/longer but **not** a baseline mane or ponytail | Slight additional functional strength, steadier stance; burdened, not glamorous | Scarred and improvised gear, increasingly strategic, still the same man |

**Hair and timing:** Hair grows within a short in-universe timespan; avoid dramatic normal hair growth. Later-book extraordinary long hair tied to a magical event (book 7) may become a **distinct lore-event/temporary variant**, not the approved baseline art progression. A later optional alternate must be separately reviewed for chapter accuracy.

**The four book groups are ART milestones, not a claim that each piece of equipment appears at their first chapter.** Maintain equipment state and model variants based on cited narrative moments.

## Animation and expression locks

**Face states:** neutral/annoyed, confused, startled, sarcastic, pain, furious, determined, exhausted, unexpected relief.

**Essential movement acting:**
- `idle`: weight on one leg, small wary glance; later stages quieter and more controlled.
- `run`: practical and slightly ungainly early; progressively economical without changing leg length or center-of-mass rules.
- `jump`/`land`: grounded takeoff, effort, occasional rough landing; avoid default three-point superhero landing.
- `attack`: trap/bomb staging and visible effort; expressive anticipation, hitstop and recovery.
- `hurt`: clear recoil and anger; wounded but still stubborn.
- `victory`: disbelief/irony more than conquering-hero chest-puffing.

No speech or facial expression should be the sole source of mandatory combat information.

## Tiny refinements remaining (production QC, not a redesign)

1. **De-emphasize sculpted abdominal definition** in later images; prioritize silhouette and actual mechanics, not muscle shading.
2. **Preserve the messy hair shape** across all stages with controlled small growth; special long-hair event excluded from the default lineup.
3. **Check props against canon.** Cigarette/cigar, stubble, decorative cloaks and non-specific equipment from the generated board are artistic noise, not automatically approved; Carl's shaving habits are a specific canon-consistency check. A role/accessory must be supported by an item/event source or tagged `noncanonical proposal`.
4. **Increase expressive asymmetry** (furrowed brows, tired eyes, awkward poses) before adding outfit complexity.

## Build-ready acceptance criteria

- A single master turnaround: front, side, back, facing-left/facing-right where asymmetry matters, consistent face/limbs.
- Master of early Carl reviewed **at 48–56 virtual pixels tall (provisional)** on a 640×360 canvas; adjust only after visual/gameplay review.
- Aseprite editable sources, deterministic export, palette list, pivot and contact-foot reference, animations with metadata; test at 1x and integer-scaled 4K (6x) and 1280×800 (640×400 base at 2x) with Steam Deck-size text.
- Two short animation tags per acceptance scope, starting `idle` and `run`, without limb jitter or unrelated AI frame drift.
- All details marked `canon verified`, `art-direction choice`, or `noncanonical gameplay proposal`, with book/chapter references for the first.
- Before a recognizable franchise image enters public Git history, record the owner-reported permission scope and asset provenance in the rights register. The concept-art publication scope is owner-confirmed; runtime builds and other distribution still require separate clearance.

## Agent instruction

**Do not generate new Carl face/body concepts unless human review reopens the decision.** Work from the selected reference and the rules above. Exclude temporary narrative transformations from baseline sprites; do not infer canon from the infographic. Preserve ordinary, unglamorous appearance and the gritty, darkly funny, 80s/90s action antihero identity.

Related: [Project tone](../01-design/antihero-tone.md), [Art direction](art-direction.md), [Character gameplay roles](../01-design/characters.md), [Rights and release](../04-production/rights-and-release.md).
