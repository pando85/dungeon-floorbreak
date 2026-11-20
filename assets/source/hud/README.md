# HUD hybrid A art sources (HUD-HYBRID-A-01)

Editable sources for the native 640x360 / 640x400 hybrid HUD (plates, strips, notice frame,
button glyphs, icons, ownership tags, two hero portraits, the System CRT face). Everything exports
into **one atlas**, `generated/art/hud_hybrid_a.png` plus `hud_hybrid_a.json`
(`scripts/export_assets.sh`, manifest `assets/manifests/hud_hybrid_a_v1.json`). The manifest's
`hud.regions` list is the contract for frame names and rects (the LibreSprite JSON has one frame).

Status: prototype, review pending (`prototype_fixture_unapproved`). Not wired into Bevy by this change.

## Sources

| File | Role |
| --- | --- |
| `keys/carl_portrait.grid.json`, `keys/system_crt_face.grid.json` | Frozen top-tier keys, byte-identical copies. Not edited. |
| `hud_portrait_carl.grid.json` | The Carl key's pixels (names `carl_*`, colours subset of `carl_h52`). Pivot set to the bottom row; 11 lone 1 px details re-keyed to same-colour accent aliases (`*_detail`) so strict lint passes. Colours and positions unchanged (tested). |
| `hud_system_crt.grid.json` | The CRT key (`idle`, `blink`, `notice`), palette names aligned to the spec tokens (`metal_hi`, `crt_face_hi`), lone 1 px details re-keyed as above. Colours and positions unchanged (tested). |
| `hud_portrait_donut.grid.json` | 28x28, 1:1 crop of `assets/source/characters/donut/donut_h40.grid.json` frame 0 at x 14..41, y 4..31 (crown inside row 0, eyes on rows 12 to 17). Four edge pixels repainted to remove orphans: (0,21) O, (27,21) o, (10,27) C, (23,27) C. `donut_h40` palette only. |
| `hud_plate_frame`, `hud_strip_3slice`, `hud_notice_frame`, `hud_glyphs`, `hud_icons`, `hud_tags` `.grid.json` | Original art written by `scripts/author_hud_hybrid_a.py` from design spec sections 2, 3 and 7. |
| `hud_hybrid_a.grid.json` / `hud_hybrid_a.ase` | Combined 208x156 single-frame atlas source (one layer per asset group). `scripts/export_assets.sh` rebuilds the `.ase` from the grid and exports the atlas. |

The wells' interior (28x28 at +6,+6) and the tag area (+4,+36,32,9) of `plate_frame` are transparent
on purpose; the renderer fills the well with `code_drawn_tokens.well_carl` / `well_donut` and draws a
`tag_*` sprite. Bars, hatches, PREVIEW tag and text are code-drawn. Strip pieces sit side by side in the
atlas (left cap, 1 px mid, right cap); repeat `*_mid` to the wanted width.

## Rebuild

```sh
python3 scripts/author_hud_hybrid_a.py            # rewrites grids + manifest (add --check to verify)
for g in assets/source/hud/hud_*.grid.json; do python3 scripts/pixel_grid_to_ase.py lint "$g" --strict; done
bash scripts/export_assets.sh                      # rebuilds hud_hybrid_a.ase, exports the atlas, validates
python3 scripts/make_hud_hybrid_a_evidence.py      # docs/evidence/hud-hybrid-a-01/
python3 -m unittest discover -s tests -p 'test_art_hud_hybrid_a.py' -v
```

## Provenance

Derived from approved sprite masters and top-tier keys; no AI concept pixels copied. The HUD concept
PNGs in `assets/references/hud/hybrid-a-v1/` were never sampled, traced or cropped. Design and keys:
Claude Opus. Execution (Donut crop, frames, glyphs, icons, tags, atlas, validators, evidence): Claude
Sonnet. Independent top-tier review: pending. Rights scope: `ART-PUBLICATION-ALL-V1` (no reuse licence).

## SHA-256

```text
7d13f3d2a5b4ef4e604a3bcea2be4b558eb41b7da51e9769cd4d32361ecaf306  keys/carl_portrait.grid.json
b5ec5a51332bcb3dc6641a0dd95f08e7d2817bb812f16f4a63a9387cc8dc71a3  keys/system_crt_face.grid.json
6cf8e71880d6339bf79e4240d740b5fcac11abe399ccb5e5a12c2654ddaddd1b  hud_glyphs.grid.json
899aa74b769465a1243a603290e0f78ba150f32e77d25da30eed71f85cc08d87  hud_hybrid_a.grid.json
fbb647ff7ef6f014ea626e839571b9ac9a1bdc6cd92d3de75b90e4120308e310  hud_icons.grid.json
a11005644b0ef2b2381927ea3199d9d21f50210d2f55d116399d3260ee8c8e12  hud_notice_frame.grid.json
b1dd69d68ef6a07d8afcf02958d86ed29ec4f62bbf0c8c439b6151c84857744a  hud_plate_frame.grid.json
297ceaba67674045e10370ff5908c9adc02eb04048892cc84242673361b2f667  hud_portrait_carl.grid.json
5eb5938d6ea9af24ce8e8926a1519ff90b3c711faf8b410cd9e4520180aecb4a  hud_portrait_donut.grid.json
37cb44b9d87a91e69d8d332003d437dd7f4f9c5b2c9013541784a10039fe2e7d  hud_strip_3slice.grid.json
95cc3f2d3c15ee7389a390cf12582b14c03948c72dae372a6da459916caac550  hud_system_crt.grid.json
d00b660997b63cad53a3e42fea5227af4a1daa35027aeda25257d420e1ef8a32  hud_tags.grid.json
1e95b325589a1efa380f3d8fcd8aa51eaafe9b9d8e34062c790c7ebb9a907832  hud_hybrid_a.ase
```
(Atlas PNG: 1f73005e1955f0f90d1cbc2b73ef6c152e715b454bf79696d4e00e1e86046cf7 with the local LibreSprite 1.2-dev build; CI's pinned 1.3 is the reference. Per-region RGBA hashes are in the manifest.)

## AUDIENCE-01 viewer art (part A)

`hud_viewer_eye`, `hud_viewer_up`, `hud_viewer_down` and `hud_viewer_digit_{0..9,k,m,plus}` are single-frame sources (16 regions named `viewer_*` in the atlas), copied pixel-exact from the top-tier keys in the PR #118 design review and written by `scripts/author_hud_hybrid_a.py`. There is no decimal-point source: the HUD draws `.` as a code-placed 1x1 rect, because a lone pixel is not a sprite and fails the strict lint. See [the evidence README](../../../docs/evidence/audience-01/README.md).
