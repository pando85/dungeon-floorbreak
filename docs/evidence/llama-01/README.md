# LLAMA-01 evidence: Bad Llama readability set (17 frames)

Review renders for the [LLAMA-01 design spec](../../02-art/llama-01-spec.md). They are composed PNGs and GIFs lit with the A1 compositor, not Bevy captures. The llama is not wired into any binary. Status: candidate; keys frozen (Claude Opus), in-betweens redrawn after design fix round 1 (Claude Sonnet), validators pass; **independent Opus review pending; owner review at the end; Bevy integration pending**.

Regenerate with:

```sh
bash scripts/export_assets.sh                                   # atlases, including the Floor 1 kit the compositor needs
python3 scripts/author_llama01.py keys                          # grid + manifest (--check verifies them)
python3 scripts/pixel_grid_to_ase.py build assets/source/characters/llama/llama-01.grid.json assets/source/characters/llama/llama-01.ase
python3 -m scripts.pixelart validate assets/source/characters/llama/llama-01.pixelart.json --out docs/evidence/llama-01/validate
python3 -m scripts.pixelart evidence assets/source/characters/llama/llama-01.pixelart.json --out docs/evidence/llama-01
python3 -m scripts.pixelart review-pack assets/source/characters/llama/llama-01.pixelart.json --out docs/evidence/llama-01/review-pack
```

`SHA256SUMS` lists the hashes of every file here except `packet/` and itself.

| File | What it shows |
| --- | --- |
| `review-pack/01_contact_sheet_x4.png` | all 17 frames at x4 with frame, tag, ticks and KEY marks, contact row 71 (cyan) and pivot (yellow) |
| `review-pack/02_lit_scene_640x360_x3.png` | Carl and Donut unobstructed on the left, a spit glob (frame 16) in flight to their right, then a llama releasing it (frame 9), an idle llama (0) and a tell llama (8); llamas are mirrored by the runtime flip, as the game will draw them (baked highlights then sit on the far side: accepted project-wide, decision 2026-11-18); lit Floor 1, shown x3 |
| `review-pack/03_tell_attack_strip.png` | tell (7, 8), release and recover (9, 10) and the spit projectile (15, 16) with the 60 Hz tick bar (8 + 22 + 6 + 14 + 4 + 4) |
| `review-pack/validator_report.json`, `validator_summary.txt` | the review-pack validator run (V01 to V15) |
| `validate/validator_report.json`, `validator_summary.txt` | the `validate` run: 13 PASS, 2 SKIPPED (V08, V10 not applicable), 0 FAIL |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | the same contact sheet at 1x and x4 |
| `tell_attack_strip.png` | the tell to attack strip (same as review-pack image 3) |
| `scene_640x360.png`, `scene_1920x1080.png` | the lit scene at 640 x 360 and 1920 x 1080 |
| `gif_<tag>_x1.gif`, `gif_<tag>_x3.gif` (7 tags: `idle`, `walk`, `attack_tell`, `attack_swing`, `hurt`, `down`, `spit`) | each tag at its 60 Hz tick table, 1x and x3; `gif_walk_x1.gif` / `gif_walk_x3.gif` is the walk loop (frames 3 to 6, 24 ticks), lifted hooves clear the floor by 4 to 5 px |
| `packet/` | the design packet (concept crops, palette candidates, scale lineup, templates) |
