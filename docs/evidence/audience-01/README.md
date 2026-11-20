# AUDIENCE-01 evidence: viewer icon art (part A) and the HUD integration (part C)

This directory holds two kinds of image. The `mock-composite-*` and `contact_sheet_*` files are the part A **mocks, not runtime** (the part A pixels placed on the old captures). The `hud-REAL-*`, `hud-PREVIEW-*`, `contact-sheet-*-2x` files are **real Bevy captures** of the part C runtime, described in [Part C](#part-c-runtime-captures) below.

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

## Part C: runtime captures

Real window captures of `floor01_scene` with the real exported atlas (no placeholders), made with `xvfb-run -a -s '-screen 0 1280x720x24' bash scripts/ci_floor01_render_smoke.sh audience` and `... 1280x800x24 ... audience-deck` (20 states each, lit, scale 2, NVIDIA GPU under Xvfb on a local machine, not a Deck). Each passed `scripts/check_hud_capture.py` (flat 2x2 blocks, HUD palette only in the plate rows, 10 px and 5 px caps) and its rendered simulation state equalled the headless replay with and without the flags. `python3 scripts/make_audience_hud_evidence.py` reduces them to 1x by integer nearest-neighbour (nothing is lost, the checker proves the blocks are flat), tiles the sheets and rewrites `SHA256SUMS`.

- **REAL** in the filename: a real run with the QA probe `--audience-probe` (the audience is unlocked at step 1; Floor 01 itself has no unlock, spec F15), so the counts come from real combat events through the production mapping.
- **PREVIEW** in the filename: a `--hud-preview viewers*` fixture (static values, the `PREVIEW <NAME>` tag, the real plates stay).

| File pattern | What |
| --- | --- |
| `hud-REAL-viewers-locked-<size>-1x.png` | the default: no viewer art at all (same scene as the others) |
| `hud-PREVIEW-viewers-{normal,rising,falling,max}-<size>-1x.png` | the reference-image states: 6.8K / 12.4K steady, 38.4K / 87.1K rising, falling, the widest strings (Carl `999M+`, Donut `99.9K`) |
| `hud-PREVIEW-viewers-{normal,rising,falling,max}-high-contrast-<size>-1x.png`, `...-rising-reduced-motion-...` | `--hud-high-contrast` (the down arrow is the full `amber`) and `--hud-reduced-motion` |
| `hud-REAL-viewers-donut-kill-<size>-1x.png` | `q10_pounce_hit`: Donut's pounce kill raises her count to 370 and leaves Carl at 250 (each count belongs to its own hero) |
| `hud-REAL-viewers-spectacular[-high-contrast]-<size>-1x.png` | `cp03_shore_kill`, tick 170: the real 1800 award gives Carl 2.0K with an amber number and the up arrow |
| `hud-REAL-viewers-anticipation[-high-contrast]-<size>-1x.png` | the corridor, tick 148, `--audience-cue-x 300`: an authored cue raises both heroes to 4.2K with the same cue as a spectacular gain and nothing on screen |
| `hud-REAL-viewers-falling[-high-contrast]-<size>-1x.png` | `cp03_shore_kill`, tick 1010: the same count after the decay, 1.7K with the down arrow |
| `hud-REAL-viewers-boss-<size>-1x.png`, `hud-REAL-viewers-post-mana[-high-contrast]-<size>-1x.png` | the boss strip (`boss01_crush_kill`, tick 335) and Donut's plate with her mana bar (`q10_mana_regen`, tick 60): no overlap with HP, mana, the chips or the boss strip |
| `contact-sheet-{preview,real}-<size>-2x.png` | the top HUD band of each state at 2x, labelled |
| `compare-mock-<size>-{normal,high-contrast}.txt` | per-pixel comparison of `PREVIEW viewers-max` with the part A mock composite |
| `headless_audience_probe.txt` | the committed `AUDIENCE_STATE` digests of five probe runs; `bash scripts/ci_floor01_combat.sh headless` replays them twice, diffs them and proves the probe and every viewer preview leave the plain replay's events and state lines unchanged |

`<size>` is `640x360` or `640x400` (`--deck`). The readout is identical on both canvases.

### Comparison with the reference images and the mocks

The references (`assets/references/hud/viewers-v1/`) are overlays, not runtime art; their values are used by the previews. At native scale the runtime reproduces the part A mock exactly in layout (eye, number, decimal dot, arrow positions) at `999M+` and `99.9K`, on both canvases and in high contrast: Donut's readout has **0** differing pixels, Carl's differs in 72 pixels only by the red channel of the amber number (244 against 255), the GPU-tint deviation described in the spec. The fit holds with the DONUT name (12 px gap), the HP row (+16), the mana bar and chips, the ring and the boss strip.

### Known limits

- The up arrow lasts 48 steps and the count-up 30, so the frozen captures sit inside those windows by construction (the ticks above).
- Anticipation uses an authored **test** cue; Floor 01 authors none (spec F15) and the unlock beat is AUD-Q1.
- The `REAL` captures prove the plumbing, not game feel: the numbers are tuning hypotheses (AUD-Q2). Captured on a local machine, not a Deck. No frame-time measurement was made for the readouts (a handful of extra sprites).

