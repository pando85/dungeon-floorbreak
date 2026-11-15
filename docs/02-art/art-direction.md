# Art direction — dark, expressive, detailed pixel art

**Status:** preferred direction selected by project owner (2026-11-15). Goal: the expressivity of large drawn characters with the discipline of genuine pixel art. **Not** Broforce-scale tiny pixel silhouettes; **not** a direct recreation of Mewgenics imagery.

## Visual statement

A dingy, dangerous, theatrical dungeon whose rooms are saturated with readable props and dark humor. Gameplay foreground is brighter and higher contrast than the moody background. Characters are large enough to read posture, expressive faces, tail motion and impacts at normal Steam Deck viewing distance. Destruction is visually legible: material chips, changing edges, falling chunks, sparks and smoke.

## Project-wide antihero identity (mandatory)

All designs must obey [`the 80s/90s everyman antihero creative rule`](../01-design/antihero-tone.md). Retain the chosen **gothic/vibrant** dark pixel-art direction (cold blue-violet stone, magenta magic, hot orange explosions) while making the cast look like *flawed people improvising in a cruel action spectacle*, not fantasy superheroes. Carl starts with ordinary proportions, an unglamorous face, mild softness, awkwardness and book-accurate odd clothing. Over time give him **modest, incremental functional strength**, battle wear and earned confidence; **never** a chiseled jaw, six-pack, giant V-torso, default cape/armor, or perfect action-star posing. Keep comedic reactions and painful recovery frames. Avoid mandatory VHS grain or visual filters that harm crisp scaled pixels.

## Approved Carl design reference

Carl's four-stage appearance and evolution are **approved by the project owner**; see [`carl-character-bible.md`](carl-character-bible.md) before commissioning or generating further Carl art. Hair stays short-messy, gradually fuller/shaggier, not a default long ponytail; his face and physique remain ordinary, with earned charisma and only modest functional strength. The project owner confirms permission to publish approved **concept images** in the public community repository; the [approved Carl concept directory](../../assets/concept/carl/approved/README.md) includes the committed and source-verified image, provenance, and SHA-256 validation manifest. This does **not** authorize distribution of the game.

## Approved Princess Donut visual identity — source upload pending

The owner selected Donut's **adorable, very round, fluffy tortoiseshell Persian** identity and the eight-stage visual progression as the character's visual north star. Keep giant golden eyes, a tiny muzzle, compact paws, the huge plume tail and a small crown. The later dark/magical variants **must not** replace her cute neutral expression. Bracelets are important continuity motifs, but their names, stats and acquisition floors printed in the AI-generated sheet are **not canon-verified**. See the [Donut character bible](donut-character-bible.md), [gear audit](donut-gear-timeline.md), and [source/reference package](../../assets/concept/donut/approved/README.md). The original PNG has been committed and checked against the source identity; do not treat the miniature concept sprites as genuine gameplay pixel art.

## Approved mood references — no automatic fidelity target

The project owner reaffirmed on 2026-11-15 that the **two project-generated
visual target images** shown during the Art Slice v2 discussion capture the
desired overall direction: dense gothic-industrial arena architecture, neon
magenta theatrical machinery, cold violet stone, rusty metal catwalks, hot
amber torchlight and explosive destruction. They are the primary mood
references for future art direction reviews, **not** unrelated generic
internet-search illustrations.

- Background-only art target: `a_highly_detailed_pixel_art_background_matte.png`
  (generated in the private project conversation).
- Cinematic action-scene target:
  `a_detailed_pixel_art_game_screenshot_scene_a_vibr.png`
  (generated in the private project conversation).
- Both original reference images are now present in the repository and verified
  by the [mood reference register](../../assets/references/mood/README.md), which
  records source hashes, scope of approval and restrictions. Both are **mood
  references only**, not production-quality targets.

**Explicit exclusions:** The characters in these images are **noncanonical**
and do not replace approved character designs. In particular, the shotgun
depicted in the action image is **never Carl's weapon**. Do not repeat that
shotgun in future Carl concepts, sprite work, gameplay previews, prompts,
marketing art or equipment specifications. Generic weapons/equipment in
generated reference images must not be promoted to lore or mechanics.

**Fidelity boundary (owner clarification, 2026-11-15):** The owner approves the
**overall style only**; the depicted detail is **far from the desired eventual
result**. Do not treat object microdetail, sprite quality, density of decoration,
facial anatomy, weapon designs, pixel clustering, resolution, artistic
consistency, lighting implementation or final rendering quality as accepted.
The illustrations establish a **mood north star**, not a polished production
quality target. Final fidelity must be defined and accepted independently
through actual editable assets shown at native gameplay resolution.

**Publishing these references:** Keep them as separately labeled **reference
images**, not in runtime asset directories and not in texture atlases. They live
in `assets/references/mood/` with a README recording approved traits,
rejected details, authoring provenance and source hashes. The public GitHub
repo's rights register (`ART-PUBLICATION-ALL-V1`) records the owner-reported
publication scope for all fan-concept character art, concept and production.
That scope is user-reported, not independently verified, and grants no reuse
licence or game/asset-pack distribution right; every new asset still needs
its own provenance and checksum record.

## Initial screen and scale hypotheses

| Property | Initial target | Why |
| --- | --- | --- |
| Virtual canvas | 640×360 at 16:9, 640×400 at 16:10 | Integer 3× to 1080p; integer 2× to 1280×800 |
| Tile grid | 16×16 pixels | Convenient autotile and chunk logic |
| Carl height | ~48–56 px | Room for expressive silhouette |
| Donut height | ~28–36 px | Smaller but prominent face/tail |
| Mongo height | ~64–88 px | Distinct mass and momentum |
| Rendering | Nearest filtering + integer scaling when feasible | No blurry fractional pixels |
| Runtime FPS | 60 target, verify on actual hardware | Controller responsiveness |

These are **provisional art measurements**. Final camera framing and HUD may change virtual dimensions; prevent pixel jitter with snapped presentation coordinates while preserving subpixel simulation positions. Fixed virtual resolution is not a mandate to crop ultrawide or 16:10 displays.

## Art rules

- One transparent PNG per exported sprite sheet; keep editable `.aseprite` sources separately.
- Reserve outer/dark outline language consistently; use controlled clusters, not random pixel noise or accidental antialiasing.
- Avoid high-frequency detail behind playable figures. Desaturated backgrounds, stronger character accents.
- Object materials must be identifiable using shape/texture/pattern **and** hue; never rely on color alone.
- Fire and shock effects: high-contrast highlights and impact frames; cap lingering bloom and opaque smoke.
- Character silhouettes must remain distinct at 1× virtual resolution and during overlaps.
- Lighting can use modern additive effects, but base raster layers remain intentionally pixelated.
- No direct imitation of commercial assets; use mood/style comparisons as abstract reference only.

## Initial material palette (direction, not locked numeric colors)

Dark basalt gray, warm rust/brown wood, muted green-gray stone, cold blue metal, acidic magic accent, warm explosive amber and bright dangerous-red indicator. Establish precise palette IDs and swatches **after** a 3-character/1-room color test. Each character gets a small swatch budget with shared environmental accents.

## Character art acceptance

Review each sprite at 1×, 2× and 3× on a neutral and game background. Tests: recognizable silhouette without face details; readable anticipation and recovery; correct pivot and contact position; expressive but not distracting secondary motion. If two agents produce variants, select a canonical master and forbid silent restyling.

## Layer stack and depth

1. Background matte and distant silhouettes (low contrast).
2. Midground parallax props (avoid collisions).
3. Terrain and gameplay objects with strong edges.
4. Characters/projectiles with stable contrast.
5. Foreground debris/vignette that never hides mandatory tells.
6. Diegetic HUD/effects with adjustable flash and shake.

## Inspiration guardrail

Broforce, Mewgenics, Dead Cells and Noita serve as comparisons of **gamefeel / rendering techniques**, not permission to copy their sprites, characters, animations, palettes, font files, FX atlases or sounds.
