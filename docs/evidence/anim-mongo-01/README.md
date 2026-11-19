# ANIM-MONGO-01: evidence for the adult Mongo locomotion (design stage)

Pillow renders of the committed grid `assets/source/characters/mongo/mongo_adult_h76_locomotion.grid.json`, nearest-neighbour integer enlargements only. None is a Bevy capture, and no Floor 1 lit scene is included yet (it needs the exported kit atlases). Spec: [`docs/02-art/mongo-locomotion-anim-01-spec.md`](../../02-art/mongo-locomotion-anim-01-spec.md). Frames 7, 8, 9, 11 and 15 are rig drafts, not finished art.

Regenerate from the repository root:

```
python3 scripts/author_mongo_locomotion.py --check-keys
python3 scripts/make_mongo_locomotion_evidence.py
```

The script rewrites `SHA256SUMS` (this README included), which `python3 scripts/check_docs.py` verifies.

| File | What it shows |
| --- | --- |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | all 16 frames; keys captioned in yellow, contact row 87 cyan, pivot (76, 87) yellow |
| `keys_x6.png` | the master idle and the seven design keys (4, 5, 6, 10, 12, 13, 14) at 6x |
| `scale_lineup_x1.png`, `scale_lineup_x3.png` | Carl 52 px and Donut 40 px (idle and a run frame from their locomotion grids) beside Mongo idle, run F0, run F1 and jump rise, pivots on one floor row (rise lifted 18 px) |
| `run_x1.gif`, `run_x3.gif` | the 6-frame run at 5 ticks per frame (500 ms cycle), two cycles; exact indexed palette |
| `jump_land_x3.gif` | idle, crouch, push-off (draft), rise, fall, impact, recover (draft), idle; rise and fall lifted for the preview only; exact palette |
| `measurements.json` | per frame: duration, bounding box, height, head top-left and row-87 contact runs per leg layer |
