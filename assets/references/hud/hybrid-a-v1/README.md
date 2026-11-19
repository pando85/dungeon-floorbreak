# Hybrid-A HUD concept references

**Status:** four approved *concept-composition* images with a Donut mana bar. The four PNGs are committed here bit-exact; `sha256sum` of each file matches the table below.

| File | Preview state | SHA-256 (original PNG bytes) |
| --- | --- | --- |
| `01-exploration-mana.png` | Quiet exploration; mana-enabled *post-unlock* preview | `1324d6f6fa0a30d0c3f5c910a9057c24d2ff3346b6ee5aa19f10899438b1ccb4` |
| `02-combat-mana.png` | Intense combat + System message | `b47e4beabadd7c9ab13d3da118cf765f4dbf2c50c60c2ca78ac4786b5261d0c1` |
| `03-boss-mana.png` | Illustrative boss + warning bar, not a canonical boss | `eeb51c486ce2c226eccb404ca0f730e7adc6932b60eddb34dddef192d635b492` |
| `04-coop-low-mana.png` | Two-player condition differentiation with Donut low mana | `0aa351866a47f90108695368c04dd62eb99cce6b702b8f90cfad344bf024749c` |

All are 1672×941 RGB/RGBA concept renders, *not* native 640×360 game art or editable sprites. These are stylistic references for a reusable, source-owned Bevy UI, NOT runtime backgrounds or cropped production texture atlases.

**Important corrections:** mockup glyph/action text violates accepted MVP mappings; the magical abilities and mana shown in the opening corridor must be understood as **post-unlock preview overlay**, not a grant of pre-biscuit magic; numeric HP/MP, percentage buffs, name “Warlord Grizzog”, and branded award text are examples, not accepted game mechanics or franchise canon.

**Provenance:** OpenAI image generation in the project conversation on 2026-11-19, owner-selected hybrid-A visual direction and subsequent Donut-only mana design. Owner-reported public concept-art publication permission `ART-PUBLICATION-ALL-V1` (not independently verified) does not grant unrestricted reuse or game distribution. See `docs/04-production/rights-and-release.md`.

Maintain bit-exact files; do not resave or recompress them. Verify with `sha256sum *.png` in this directory against the table above (also recorded in `manifest.json`).
