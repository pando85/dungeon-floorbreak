# DOZ-01 evidence: Goblin Murder Dozer size study, keys and construction (phase 1)

These are review renders for the [DOZ-01 design spec](../../02-art/doz-01-dozer-spec.md). They are composed PNGs lit with the A1 compositor (`scripts/env_kit_light.py`), not Bevy captures. The Dozer is not wired into any binary.

Status: **phase 1 design by Claude Opus. The size decision is pending (a separate Opus agent decides). Execution (Sonnet) and the independent Opus review are pending.** The size options are block-ins rasterized natively from one parametric construction. The h120 part keys are the design keys.

Regenerate with:

```sh
python3 scripts/author_dozer_doz01.py          # rewrites the grids, manifests and rig contract (--check verifies them)
bash scripts/export_assets.sh                  # builds the .ase, the atlases and the Floor 1 kit atlases the compositor needs
python3 scripts/make_dozer_doz01_review.py     # renders -> generated/art/doz01_review/
```

The files are copied here unchanged. `SHA256SUMS` lists their hashes. `measurements.json` records the per-option measurements (opaque size, view share, ceiling clearance, wheel and rim heights against Carl's 92 px jump apex, atlas cost) and the key frame sizes, pivots, colour counts and lint.

| File | What it shows |
| --- | --- |
| `size_study_sheet.png` | The three options (body height 96, 120 and 150 px) at x2 on one floor, next to the approved Carl 52 and Donut 40 (run frames). Guides mark Carl's height, his jump apex (feet 92 px) and his head at the apex. Below: the metrics table, then the three lit chase scenes from the 1920 x 1080 renders, scaled to one third |
| `floor01_chase_h96_*.png`, `floor01_chase_h120_*.png`, `floor01_chase_h150_*.png` | Each option chasing Carl and Donut on lit Floor 1 at 640 x 360 (native) and 1920 x 1080 (x3). Carl is 213 px (one third of the view) ahead of the wheel's front tip. h150 no longer fits the view at that gap |
| `floor01_chase_keys_h120_640x360.png`, `floor01_chase_keys_h120_1920x1080.png` | The recommended option assembled from its **part keys** with the rig contract (not the block-in), in the same chase framing |
| `key_sheet.png` | <ul><li>The assembled machine at x3 and x1.</li><li>The hull `body` and `front` keys at x3 with the pivot.</li><li>The wheel key at x4 with its 4-phase rotation plan (7.5 degree steps).</li><li>The tread key at x4 with its 3-phase belt plan (3 px steps).</li><li>The engineer and rider A keys and the derived rider B at x6, and the yoke at x4.</li></ul>The plan previews are rendered from the same geometry; they are not keys |
| `part_layout_diagram.png` | The assembled rig at x3 with part anchors (yellow), the rig origin O (green), the walking line (cyan), the proposed lethal wheel circle (red, r 44; visual tips r 48), the proposed pushing wall (blue), the occluder rows (orange), smoke emitters and light points. Also the draw-order legend, the exploded parts at x2 with frame boxes and pivots, and the proposed segment D charge timeline |
| `measurements.json` | Measurements and file hashes |

## Recommendation in one line

**h120**: the whole machine and crew fit in the chase framing, and nothing invites a jump-over (wheel 96 px, tub rim 104 px, against Carl's 92 px jump apex). It leaves 121 px for smoke under the ceiling, the 36 px riders read as a crew, and the split atlas costs about 183k px. The size is **not decided** until a separate Opus agent rules on it.

## Weakest points (for the deciding and reviewing agents)

- The cab's rust plates are the part closest in value and hue to the torch-lit brick. They stay separate because of the outline and the darker rust ramp, but a darker or cooler cab is an option.
- The wheel drum is a flat rust disc with ribs. It reads at 1x, but it has less surface interest than the hull.
- The tread interior is mostly dark. The road-wheel hubs carry little rotation cue (the belt does the work).
- The yoke arm is a plain beam.
- Rider A's pineapple stick keeps the approved GOB-01 frame-11 gap between the fist and the stick.
