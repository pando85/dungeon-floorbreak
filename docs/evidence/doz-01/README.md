# DOZ-01 evidence: Goblin Murder Dozer size study, keys and construction

These are review renders for the [DOZ-01 design spec](../../02-art/doz-01-dozer-spec.md). They are composed PNGs lit with the A1 compositor (`scripts/env_kit_light.py`), not Bevy captures. The Dozer is not wired into any binary.

Status:

- **Design:** Claude Opus, phases 1 and 2.
- **Size:** h120, decided by a separate Opus agent.
- **Pending:** execution (Sonnet, spec section 7.3) and an independent Opus review.

The size options are block-ins rasterized natively from one parametric construction. The h120 part keys are the design keys. Phase 2 added:

- the wheel redesign (the spike ring is now the brightest part);
- the saucepan redesign;
- `engineer_lever_pull`;
- lurch-back and lurch-forward keys for all three riders;
- smoke keys 0 and 2.

Regenerate with:

```sh
python3 scripts/author_dozer_doz01.py          # rewrites the grids, manifests and rig contract (--check verifies them)
bash scripts/export_assets.sh                  # builds the .ase, the atlases and the Floor 1 kit atlases the compositor needs
python3 scripts/make_dozer_doz01_review.py     # renders -> generated/art/doz01_review/
```

The files are copied here unchanged, and `SHA256SUMS` lists their hashes. `measurements.json` records:

- per size option: opaque size, view share, ceiling clearance, wheel and rim heights, and atlas cost under the phase 2 frame plan;
- `wheel_spike_ring_mean_lstar`;
- for each key: frame size, pivot, colour count and lint.

| File | What it shows |
| --- | --- |
| `size_study_sheet.png` | The three options (body height 96, 120 and 150 px) at x2 on one floor, next to the approved Carl 52 and Donut 40 (run frames). The guides mark Carl's height and his jump apex. Below: the metrics table, then the three lit chase scenes from the 1920 x 1080 renders, scaled to one third. h120 is marked as decided |
| `floor01_chase_h96_*.png`, `floor01_chase_h120_*.png`, `floor01_chase_h150_*.png` | Each option chasing Carl and Donut on lit Floor 1, at 640 x 360 (native) and 1920 x 1080 (x3). Carl is 213 px (one third of the view) ahead of the wheel's front tip |
| `floor01_chase_keys_h120_640x360.png`, `floor01_chase_keys_h120_1920x1080.png` | The decided option, assembled from its **part keys** with the rig contract, in the same chase framing |
| `key_sheet.png` | Every key: <ul><li>the assembled machine at x3 and x1;</li><li>the hull `body` and `front`;</li><li>the redesigned wheel at x4, with its 4-phase rotation plan;</li><li>the tread at x4, with phases 1 to 3 of the 9-phase belt plan;</li><li>the four engineer keys and the yoke;</li><li>the six rider A and B keys;</li><li>the four smoke keys at x8.</li></ul>The plan previews are rendered from the same geometry; they are not keys |
| `part_layout_diagram.png` | The assembled rig at x3 with part anchors (yellow), the rig origin O (green), the walking line (cyan), the proposed lethal wheel circle (red, r 44), the proposed pushing wall (blue), the occluder rows (orange), the smoke emitters and the light points. Also: the draw-order legend, the exploded parts at x2 with frame boxes and pivots, and the proposed segment D charge timeline (gated at a 140 px gap) |
| `measurements.json` | Measurements and file hashes |

## Weakest points (for the executor and the reviewer)

- The cab's rust plates are the part closest in value and hue to the torch-lit brick. The outline and the darker rust ramp keep them separate; a darker or cooler cab is still an option.
- The engineer's lever-pull forearms read as one slab at 1x.
- The black smoke is dark by design and needs a lit-brick background to read (spec 7.3, item 11).
- The yoke is still a plain beam; its polish is on the Sonnet list.
- Rider A keeps the gap between fist and stick from the approved GOB-01 frame 11. Sonnet closes it, here and in GOB-01 (spec 7.3, item 7).
