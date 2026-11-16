# Floor 01 opening — approved concept direction references

**Status:** human-approved *broad visual direction*, not final gameplay assets, Aseprite masters, Bevy captures, authoritatively measured character scale, or licensed distributable gameplay content.

These references accompany `docs/02-art/floor-01-opening-catacomb-goblins.md` and the local-agent handoff. Keep them in `assets/concept/` only. New environment tiles and character sprites must be authored at native pixel resolution from editable sources.

| File | Use | Exclusions / caveats |
| --- | --- | --- |
| `01-catacomb-environment-clean.png` | Warm-lit brick passages, blue depth, rare service pipe and a clear dry floor | Art direction, not a tilemap, layered parallax asset or actual physics geometry |
| `02-goblin-trio-reference-crop.png` | Expression, green complexion and recognizable silhouettes; distinguish two goblins and pot-helmet engineer | Generated RPG roles, alleged pixel heights and spiked clubs are **not approved**. The two on-foot fighters need chapter-2 pineapple-on-stick weapons |
| `03-murder-dozer-reference-crop.png` | Ram/spiked wheel, tracks, steam, improvised rusted frame and three goblin riders | Not a production sprite, collidable hull, hitbox or boss health specification |

The larger chase illustration with **wrong Carl and Donut likenesses** is intentionally excluded from this upload set. The approved source sprites remain Carl 52 px and Donut 40 px; do not extract or repaint those characters from generated illustrations.

See `manifest.json` for image byte sizes, hashes, creation provenance and reuse restrictions. Fan-project art publication permissions are owner-reported only and do **not** establish public release rights for a playable adaptation.

Check image integrity after copying:

```sh
sha256sum 01-catacomb-environment-clean.png 02-goblin-trio-reference-crop.png 03-murder-dozer-reference-crop.png
```

## Verified repository identity

All three PNGs **are committed on PR #23**. Their Git blob identities match the original local review files byte for byte:

| Image | Git blob SHA-1 |
| --- | --- |
| `01-catacomb-environment-clean.png` | `d5d1ecacfe4a36291ac15a389f8a14e52540a3fb` |
| `02-goblin-trio-reference-crop.png` | `4ae85d4ba98c993f12d7f480df94f7a946b8d4c4` |
| `03-murder-dozer-reference-crop.png` | `c2ab688581632a8b690c91ce57ae0861cb22a361` |

The full chase illustration is intentionally **not** in this folder because its Carl and Donut depictions are incorrect. Approval here covers environmental/enemy silhouette concepts only, not final pixels or native animation.
