# BROOD-01 evidence: Scatterer Brood Guardian readability set (16 frames)

Review renders for the [BROOD-01 design spec](../../02-art/brood-01-spec.md). They are composed PNGs and GIFs lit with the A1 compositor, not Bevy captures. The Brood Guardian is not wired into any binary. Status: candidate, execution round 2 done (redrawn on the redesigned key 0), independent review pending. `key0-scale.png` is the silhouette-gate image (key 0 at 1x next to Carl, Donut and the Scatterer).

Regenerate with:

```sh
bash scripts/export_assets.sh                      # atlases the compositor needs
python3 scripts/author_brood01.py                  # grid, .ase and manifest
python3 -m scripts.pixelart validate assets/source/characters/scatterer/brood/brood-01.pixelart.json
python3 -m scripts.pixelart evidence assets/source/characters/scatterer/brood/brood-01.pixelart.json --out docs/evidence/brood-01
python3 -m scripts.pixelart review-pack assets/source/characters/scatterer/brood/brood-01.pixelart.json --out docs/evidence/brood-01/review-pack
```

`SHA256SUMS` lists the hashes of the evidence files in this folder.

| File | What it shows |
| --- | --- |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | all 16 frames at 1x and x4 with the contact row and pivot |
| `gif_<tag>_x1.gif`, `gif_<tag>_x3.gif` | each tag at the spec's 60 Hz tick table (GIF delays are whole centiseconds) |
| `scene_640x360.png`, `scene_1920x1080.png` | Carl, Donut and three Brood Guardian poses (idle, walk, tell) on the lit Floor 1 brick |
| `tell_attack_strip.png` | `attack_tell` and `attack_swing` frames in order |
| `review-pack/` | the three review images plus `validator_report.json` and `validator_summary.txt` (V01 to V15: PASS) |
| `key0-scale.png` | key 0 at 1x on the lit floor next to Carl, Donut and the Scatterer |
| `packet/` | the design packet |
