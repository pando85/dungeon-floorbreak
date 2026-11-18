# RAT-01 evidence: Dungeon Rat readability set (19 frames)

Review renders for the [RAT-01 design spec](../../02-art/rat-01-spec.md). They are composed PNGs and GIFs lit with the A1 compositor (`scripts/env_kit_light.py`), not Bevy captures; the rat is not wired into any binary. Status: candidate. Keys: Claude Opus edit (frozen in `tests/fixtures/rat-01_keys.json`); in-betweens, export and evidence: Claude Sonnet; independent top-tier review passed with execution items, review fixes applied (spec section 16), re-review pending; no game-feel acceptance.

Regenerate with the commands in spec section 10. `SHA256SUMS` lists the hashes of every file here except the `packet/` inputs and the review-pack validator report. The scene tool mirrors enemies to face Carl, so the rats in the scenes face left.

| File | What it shows |
| --- | --- |
| `review-pack/01_contact_sheet_x4.png` | all 19 frames at x4 with tag, ticks, KEY or in-between, contact row 27 (cyan) and pivot (yellow) |
| `review-pack/02_lit_scene_640x360_x3.png` | the rat (idle, tell, bite, hurt, walk, down) with Carl and Donut on lit Floor 1, 640 x 360 shown x3 |
| `review-pack/03_tell_attack_strip.png` | tell onset and hold, lunge smear, bite impact and snap-back in tick order |
| `review-pack/validator_report.json`, `validator_summary.txt` | toolkit validators V01 to V15: PASS (13 pass, 0 fail, 2 skipped) |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | the same sheet at 1x and 4x |
| `gif_<tag>_x1.gif`, `gif_<tag>_x3.gif` (`idle`, `walk`, `attack_tell`, `attack_swing`, `hurt`, `down`) | each tag at the spec's 60 Hz tick table |
| `scene_640x360.png`, `scene_1920x1080.png` | the lit scene at native and 1080p size |
| `tell_attack_strip.png` | the strip outside the review pack |
| `size_study.png` | the 14, 18 and 22 px size block-ins next to the concept (decision: 18 px) |
| `packet/` | design packet inputs (concept crops, palette candidates, scale lineup, templates) |
