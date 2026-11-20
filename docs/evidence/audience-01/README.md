# AUDIENCE-01 part A: viewer icon art evidence

Mock composite, not runtime. The runtime HUD does not draw any viewer art yet; part C integrates it. The real captures in [`../hud-hybrid-a/`](../hud-hybrid-a/README.md) carry no viewer art, so these images only show where the part A pixels land.

Regenerate with `bash scripts/export_assets.sh` then `python3 scripts/make_audience_evidence.py`. Both are deterministic (the atlas PNG, JSON and `.ase` were byte-identical across two exports, and the evidence `SHA256SUMS` across two runs).

## Contents

| File | What it shows |
| --- | --- |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | The 16 `viewer_*` atlas regions at 1x and 4x, plus the code-drawn decimal point |
| `mock-composite-{360,400}-{normal,high-contrast}-1x.png` | Both plates over the real captures, with a label band appended below the image so HUD coordinates are untouched |
| `mock-composite-{360,400}-{normal,high-contrast}-plates-4x.png` | 4x crops of the Carl and Donut plates |

Base captures: `hud-REAL-normal-640x360-1x.png`, `hud-REAL-normal-640x400-1x.png`, and the existing `hud-REAL-low-hp-high-contrast-*` pair as the high-contrast base (no normal-HP high-contrast capture exists). The generator asserts the viewer field (+80..+158, +3..+14) is pure `plate_fill` in every base, which confirms the fit.

Placements follow the top-tier design review of PR #118 (section 3.1), identical at 640x360 and 640x400:

| Element | Carl (plate 8,8) | Donut (plate 468,8) |
| --- | --- | --- |
| Eye 9x7 | (100,13) | (560,13) |
| Number, left-aligned, 5x7, advance 6 | x 112, y 13 | x 572, y 13 |
| Trend glyph 5x7 | (161,13) | (621,13) |

Carl shows the worst case `999M+` (29 px, ink 112..140) with an amber number and the up arrow. Donut shows `99.9K` (the decimal case) with a `text_white` number and the down arrow. In high contrast the down arrow is `amber`; otherwise `amber_dim`.

## Decision record: the decimal point is not art

A lone pixel is not a sprite, and `lint --strict` rejects isolated pixels (even accent-keyed ones), so the review's 1x1 `.` key was **not** authored as a grid source and there is no `viewer_digit_dot` region. The orchestrator ruled that the lint is unchanged.

Integration note for part C: the HUD draws `.` as a code-placed 1x1 rect on digit row 6 (glyph top + 6), at the current cursor x, with advance 2. The brief named a `hud_ink` rect, but `hud_ink` (#120c18) on `plate_fill` (#211b27) is near invisible, so the mock draws the dot in the number's current colour (`text_white` or amber). Part C should confirm this with the reviewer before copying either choice.

## Other notes for part C

- Masks: digits are `text_white`, the up arrow `magenta_hi`, the down arrow `amber`. The mock recolours masks directly. Runtime tinting is a multiply (as for the warning icon, `amber` tinted to `amber_dim`), so an amber spike number from a `text_white` mask will come out slightly darker than `amber`. Check it in the capture, or add an amber digit variant.
- The eye pupil (`hud_ink`) is the only 1 px detail. It is keyed as a same-colour accent alias, so the pixels match the review key exactly.
- Eye region is 9x7 including its empty first and last rows, as in the review cell.
- The matching `FrameId` additions in `src/hud_draw.rs` (`ALL: [Self; 31]` to 47) wait for the boss PR to merge.
