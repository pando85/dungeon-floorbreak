# Floor 02 — THE CRAWL / Concrete Hell — visual concepts

**Status: exploratory visual concept art only. NOT approved production sprites, animation frames, gameplay screenshots, native-scale colliders or a final character bible.**

These files accompany [Floor 02 design PR #239](https://github.com/pando85/dungeon-floorbreak/pull/239) and the floor's English design documents. They are packaged to make the existing concepts easy for the local art agents to reference, review and import.

## Inventory

| File | Purpose | Internal beat / design source |
| --- | --- | --- |
| `revup-krakaren-key-art.png` | An original key scene illustrating the sales-jug catastrophe and clone boss, shown together as one composed inspiration | S02/S03; `docs/01-design/floor-02-revup-krakaren-setpieces.md` |
| `floor-02-enemy-acting-concept-sheet.png` | Six enemy family panels with cartoon acting poses, not real editable sprite sheets | Mind Horror, Clurichaun, Laminak, Brindle Grub/Vespa, Kobold/Dingo, Ralph; `docs/01-design/floor-02-enemy-acting-bible.md` |
| `crops/*.png` | Six **unaltered-scale crops** of the same sheet for model review packets | Use as aesthetic references only; do not copy/paste the crops as game sprites |
| `floor-02-storyboard.html` | Offline mobile-friendly canvas storyboard showing all **ten internal encounter beats** | S00–S09, organized into five connected player-facing chapters |
| `provenance.json` | Per-output origin, status and copyright/review restrictions | Required art traceability |
| `SHA256SUMS` | Integrity hashes of the artwork, crops, storyboard and provenance | Repeatable audit |

## How to bring this into the repository

1. Check out the **existing** Floor 02 design branch `design/f02-last-lesson-bible` in your working repository (or start an asset-only follow-up PR based on that branch). All Markdown specs are **already in PR #239**; don't duplicate them.
2. Extract the ZIP at the repository root. This creates `assets/concept/floor-02/` (it does not replace other assets).
3. Inspect these concept references against the **approved Carl and Donut character art** and `docs/02-art/floor-02-concrete-hell-visual-direction.md` before using them for original native sprites. Make notes of the deviations below.
4. For new native pixel art, follow `docs/07-agents/pixel-art-pipeline.md`: keyed designs, 1x silhouettes beside approved heroes, editable masters, atlas export, manifested provenance/checksums, independent top-tier review and owner review as appropriate.
5. Run relevant repo asset/docs validation on the final commit; the ZIP generator did **not** run Cargo or project CI. Only claim the exact checks performed.
6. Add these assets to #239 if desired (or a linked asset PR); mark their status **exploratory** until an independent art review and human approval are recorded.

## Caveats that MUST reach the art agent

- **Design-only images, not game-scale references.** The 1672x941 compositions exaggerate proportions and detail. Carl's approved sprite is 52 opaque pixels, Donut's 40, at the established virtual game resolution. Never resize/cut a rendered panel into a shipping spritesheet. Draw new source pixel art at 1x after the silhouette/physics gate.
- **The key art depicts an approximate hero costume/silhouette.** The real approved Carl/Donut designs override it, including costume, feline identity, hero pose and hitbox. The marketing text and logos on the concept image are exploration, not final UI or legal trademark clearance.
- **Ralph** must remain genuinely tiny and still fair to hit; his **attack-warning trail can be visually large** without creating an invisible oversized physical hurtbox. The panel is not a collider specification.
- **Mind Horror** reads as a tentacled floating brain, **Laminak** as a small hovering business executive, **Clurichaun** as a moonshine rep. Treat all fine clothing/shape decisions as **to be validated against the source lore** and actual gameplay readability, rather than canon-complete renderings.
- **Stage S07's stairwell is Rage-only expulsion.** Carl and Donut continue on Floor 02; actual Floor 03 departure follows Ralph and Mongo in S09. The storyboard's stairs must not be used as an unintended early player-exit trigger.
- **The storyboard's ten entries are internal stage beats**; do not implement ten disconnected result-screen levels. Present five contiguous arcade chapters.
- **No licensing expansion**: the public-repo concept-art scope in `docs/04-production/rights-and-release.md` is based on owner-reported permission and does **not** by itself authorize shipping a playable fan game or distributing these assets independently. No third-party screenshots, music or sprites were included in this bundle.

## Production priorities

1. `Rev-Up / Krakaren` original native art **after** the S02/S03 real combat greybox demonstrates direct and environment-driven victories.
2. Clurichaun and Laminak readable silhouette keys at 1x; then Mind Horror.
3. Separately the S05 convoy, S07 Rage, S09 Ralph/Mongo production keys, preserving story order and camera safety.
4. Do not declare this bundle approved art or a source asset pack. The agent must create its **own** editable master sprites, pivots and animation tags.

**Generation note:** these two PNGs and the storyboard HTML originated from this project's 2026-11-22 ChatGPT design session. Their exact original generation prompts, source-image references (if any), and model version are not preserved with the files; don't invent such details in provenance. Crops here are deterministic lossless image excerpts made with Pillow; they introduce no new creative artwork.
