# ANIM-MONGO-01: evidence for the adult Mongo locomotion (16 frames)

Pillow renders of the committed grid `assets/source/characters/mongo/mongo_adult_h76_locomotion.grid.json`, nearest-neighbour integer enlargements only. None is a Bevy capture; the lit scenes use the Floor 1 kit contract and the A1 lighting pass of the existing compositor. Spec and execution record: [`docs/02-art/mongo-locomotion-anim-01-spec.md`](../../02-art/mongo-locomotion-anim-01-spec.md). Status: candidate, independent review passed, owner review at the end.

Regenerate from the repository root:

```
bash scripts/export_assets.sh                      # atlases, including the Floor 1 kit the lit scene needs
python3 scripts/author_mongo_locomotion.py --check-keys
python3 scripts/make_mongo_locomotion_evidence.py  # also runs the toolkit review-pack
```

The script rewrites `SHA256SUMS` (every file here including `review-pack/`, and this README), which `python3 scripts/check_docs.py` verifies.

| File | What it shows |
| --- | --- |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | all 16 frames; keys captioned in yellow, contact row 87 cyan, pivot (76, 87) yellow |
| `keys_x6.png` | the master idle and the seven design keys (4, 5, 6, 10, 12, 13, 14) at 6x |
| `scale_lineup_x1.png`, `scale_lineup_x3.png` | Carl 52 px and Donut 40 px (idle and a run frame from their locomotion grids) beside Mongo idle, run F0, run F1 and jump rise, pivots on one floor row (rise lifted 18 px) |
| `run_footfall_diagram.png` | run frames 4-9 at 5x: the planted foot on row 87 boxed (two frames per step), the airborne frames with their gap to the ground |
| `run_onion_skin_x8.png` | run frames 4-9 over one ghost: back line and feet, one tint per frame |
| `run_x1.gif`, `run_x3.gif` | the 6-frame run at 5 ticks per frame (500 ms cycle), two cycles; exact indexed palette |
| `jump_land_x1.gif`, `jump_land_x3.gif` | idle, crouch, push-off, rise, fall, impact, recover, idle at the 60 Hz tick table; rise and fall lifted for the preview only; exact palette |
| `floor01_scene_640x360.png`, `floor01_scene_1920x1080.png` | Carl and Donut with Mongo idle, run F0, rise (lifted) and impact in the lit Floor 1 catacomb, unmirrored |
| `review-pack/` | `python3 -m scripts.pixelart review-pack`: three images (contact sheet x4, lit scene x3, `03_jump_land_strip.png`: run, jump_start, jump_rise, jump_fall and land with the tick bar) plus `validator_summary.txt` and `validator_report.json` |
| `measurements.json` | per frame: duration, bounding box, height, head top-left and row-87 contact runs per leg layer |
