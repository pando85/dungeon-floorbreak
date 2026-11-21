# AREA-REFUGE-01 evidence: the guild-room kit

Design spec: [area-refuge-01-spec.md](../../02-art/area-refuge-01-spec.md) (execution record: section 15). Designed by Claude Opus (spec and hearth key), executed by Claude Sonnet; independent review pending. Nothing here claims art acceptance or game feel.

Captured from the real Bevy scene (`floor01_scene --scene refuge_guild`, lit by the full-resolution lighting pass, Xvfb) with `scripts/refuge01_capture.sh`; the sheets are assembled by `scripts/make_refuge01_evidence.py`. Checksums: [SHA256SUMS](SHA256SUMS) for the files here; [SOURCE_SHA256.txt](SOURCE_SHA256.txt) for the pixel grids, `.ase` sources, manifests, kit contract, layout and room file (repo-relative paths).

| File | Content |
| --- | --- |
| `refuge01_room_cast_lit_640x360.png` | Carl, Donut and Mordecai on their marks, lit, native 640x360 window |
| `refuge01_room_cast_lit_640x400.png` | same, `--deck` framing (640x400) |
| `refuge01_room_cast_lit_1920x1080.png` | same at 3x (full-resolution lighting, primary target) |
| `refuge01_empty_room_contact_sheet.png` | empty room (`--refuge-empty`), unlit and lit, 1x and 2x |
| `refuge01_props_sheet_1x.png`, `refuge01_props_sheet_3x.png` | every piece at 1x and 3x (the hearth at frame 0; soot flue variants `_s1`.. `_s3` left out of the sheet) |
| `refuge01_hearth_burn_2x.gif` | the four `burn` frames at 2x, 120 ms |
| `review-pack/` | the scripts/pixelart 3-image review pack and validator report of the hearth (`refuge_hearth.pixelart.json`, V01-V17: 11 pass, 0 fail, 6 not configured) |
| `refuge01_hearth_lit_*` | the hero-key readability check composed in the ENV-02 guild corridor view (`python3 scripts/author_refuge01.py scene <dir>`), regenerated after the five gate fixes |

Mordecai is drawn at his `idle` frame 0 (review pose; the loop is in `mordecai_h44`, see the MORDECAI-01 evidence). His mark and sprite are data in `assets/levels/floor01/refuge_guild.scene.json`.

Reproduce: `bash scripts/export_assets.sh`, `cargo build --bin floor01_scene`, then `bash scripts/refuge01_capture.sh OUT.png SCALE [--deck] [--refuge-empty] [--no-lighting]` (run from the repository root), `python3 scripts/author_refuge01.py && python3 scripts/author_refuge01_props.py && python3 scripts/author_refuge01_room.py` for the sources.
