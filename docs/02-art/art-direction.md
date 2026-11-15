# Art direction — dark, expressive, detailed pixel art

**Status:** preferred direction selected by project owner (2026-11-15). Goal: the expressivity of large drawn characters with the discipline of genuine pixel art. **Not** Broforce-scale tiny pixel silhouettes; **not** a direct recreation of Mewgenics imagery.

## Visual statement

A dingy, dangerous, theatrical dungeon whose rooms are saturated with readable props and dark humor. Gameplay foreground is brighter and higher contrast than the moody background. Characters are large enough to read posture, expressive faces, tail motion and impacts at normal Steam Deck viewing distance. Destruction is visually legible: material chips, changing edges, falling chunks, sparks and smoke.

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
