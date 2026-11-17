# GOB-01 phase 1 evidence: goblin size study and variant A keys

These are review renders for the [GOB-01 design spec](../../02-art/gob-01-goblin-spec.md). They are composed PNGs lit with the A1 compositor (`scripts/env_kit_light.py`), not Bevy captures. The goblin is not wired into any binary. Status: candidate. The size is a proposal awaiting a separate Opus decision, and the keys await an independent Opus review.

Regenerate with:

```sh
bash scripts/export_assets.sh                  # builds the Floor 1 kit atlases the compositor needs
python3 scripts/make_goblin_gob01_review.py    # writes generated/art/gob01_review/
```

The files are copied here unchanged. `SHA256SUMS` lists their hashes, and `measurements.json` records opaque bboxes, heights, areas and colour counts.

| File | What it shows |
| --- | --- |
| `size_study_sheet.png` | top: the approved Carl 52 and Donut 40 idles and the goblin block-ins at 36, 40 and 44 px at x6 on one floor, with height guides; bottom: lit Floor 1 crops at x3 for each option |
| `floor01_size_h36_*.png`, `floor01_size_h40_*.png`, `floor01_size_h44_*.png` | Donut, Carl and two goblins of one option facing him at 54 and 104 px, at 640 x 360 and an exact x3 1920 x 1080 |
| `floor01_lineup_*.png` | Donut, Carl and the three options in one corridor |
| `key_pose_sheet.png` | the five variant A keys at 1x and x6 (cyan = contact row 55, yellow = pivot 28,55), then Carl facing all five on lit Floor 1 at 1x and x3 |
| `floor01_keys_*.png` | the same scene, full frame, at 640 x 360 and 1920 x 1080 |
| `measurements.json` | per-option and per-key measurements and file hashes |

The block-ins in the size study are rasterized natively at each height by one generator (`scripts/author_goblin_gob01.py size-study`), so the three options are compared with one method. They are not production pixels.

Provenance: design and keys by Claude Opus; no tracing of the concept references; review pending (independent Opus).
