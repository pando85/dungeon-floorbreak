# Floor03 · Skyfowl / Doomsday — Original Concept Handoff

**Status:** UNAPPROVED CONCEPT / MOOD ONLY — NOT PRODUCTION PIXEL ART, NOT TESTED GAMEPLAY. Date 2026-11-22.

## Import

Extract this archive at the repository root: it contains `assets/concept/floor-03/skyfowl-doomsday/`. Do not replace any of the existing approved Carl/Donut/Mongo source images or art masters.

The concept is a **single original environmental image** showing an idealized Skyfowl civic district, a dark service alley and a distant magical storm. The elaborate lighting and huge bird statues express place identity, not an accepted 640x360 working stage composition. The sketch is **not** an asset sheet or source of physics: the architecture, crowd and storm should remain parallax/environment art until proper scene colliders exist.

The HTML storyboard opens directly on mobile/offline in a browser and covers six authored beats S07–S12, including interactive previous/next steps, direct physical-combat opportunities and hard-stop QA notes. It requires no network access.

## Related GitHub design documents (PR #246)

- [Skyfowl city, GumGum, Krasue, Apito and Quill](https://github.com/pando85/dungeon-floorbreak/blob/design/f03-over-city-bible/docs/01-design/floor-03-skyfowl-city-and-quill-encounters.md)
- [Remex, first containment, four precursor pulses and final door](https://github.com/pando85/dungeon-floorbreak/blob/design/f03-over-city-bible/docs/01-design/floor-03-doomsday-scenario-final-act.md)
- [Art direction and native sprite constraints](https://github.com/pando85/dungeon-floorbreak/blob/design/f03-over-city-bible/docs/02-art/floor-03-skyfowl-and-doomsday-art-direction.md)

## Production rules

1. Use approved Carl / Princess Donut / Mongo character sources; **do not** derive their identities from this concept, which intentionally has no actual heroes.
2. Generate original editable pixel grid / Aseprite masters from the correct artist brief, then 1x scene composition with accepted hero scale. One high-res image downscaled is not native pixel work.
3. City safe zone, alley enemy windup, hanging lamp and office breach require **authoritative scene collision data**, not art-layer logic.
4. GumGum is already dead in the story; Miss Quill's death precedes Remex's revelation; the first explosion is contained inside a case and the second Remex blast threatens escape through four precursor pulses. These are canon anchors based on unofficial chapter summaries, pending primary editorial audit.
5. Any publication of fan-concept imagery uses the repository's **user-reported art publication** rights and asset provenance; there is no implied right to distribute a public playable game.
6. Independent silhouette/art review and source-level E2E/Playtrace tests are pending. Do not mark any outcome human-approved or render-tested because this ZIP exists.

## Included

- `f03_city_false_harmony_original_concept.png` — original high-resolution static visual mood reference (not cropped/downscaled sprite)
- `scene_storyboard_mobile.html` — self-contained mobile-friendly offline storyboard with diagrammatic encounter frames
- `PROVENANCE.json` — creative origin, tool and limitations
- `SHA256SUMS` — verification of all payloads except the checksum file itself

The generated image contains no intentionally referenced third-party artwork. It is nevertheless a fan-game concept rooted in the author's universe; visually review all asset output for rights/near-copying prior to any repository publication.
