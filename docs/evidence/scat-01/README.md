# SCAT-01 evidence: Scatterer readability set (16 frames)

Review renders for the [SCAT-01 design spec](../../02-art/scat-01-spec.md). They are composed PNGs and GIFs lit with the A1 compositor, not Bevy captures. The Scatterer is not wired into any binary. Status: candidate, execution done, **independent top-tier review pending**. Design and hero key (frame 0): Claude Opus. Other keys, in-betweens and evidence: Claude Sonnet.

Regenerate with:

```sh
bash scripts/export_assets.sh                      # atlases the compositor needs (includes scat-01)
python3 scripts/author_scat01.py                   # grid, .ase and manifest
python3 -m scripts.pixelart validate assets/source/characters/scatterer/scat-01.pixelart.json
python3 -m scripts.pixelart evidence assets/source/characters/scatterer/scat-01.pixelart.json --out docs/evidence/scat-01
python3 -m scripts.pixelart review-pack assets/source/characters/scatterer/scat-01.pixelart.json --out docs/evidence/scat-01/review-pack
```

`SHA256SUMS` lists the hashes of the evidence files in this folder.

| File | What it shows |
| --- | --- |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | all 16 frames at 1x and x4 with the contact row and pivot |
| `gif_<tag>_x1.gif`, `gif_<tag>_x3.gif` (`idle`, `walk`, `attack_tell`, `attack_swing`, `hurt`, `down`) | each tag at the spec's 60 Hz tick table (GIF delays are whole centiseconds) |
| `scene_640x360.png`, `scene_1920x1080.png` | Carl, Donut and six Scatterer poses (idle, walk, tell, bite, hurt, defeat) on the lit Floor 1 brick |
| `tell_attack_strip.png` | `attack_tell` and `attack_swing` frames in order |
| `review-pack/01_contact_sheet_x4.png`, `02_lit_scene_640x360_x3.png`, `03_tell_attack_strip.png`, `validator_report.json`, `validator_summary.txt` | the review pack for the independent reviewer: exactly three images plus the validator report (V01 to V15: PASS) |
| `packet/` | the design packet (concept crops, palette candidates, scale lineup, templates) |
