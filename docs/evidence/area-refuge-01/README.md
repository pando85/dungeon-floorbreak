# AREA-REFUGE-01 evidence: hearth hero key, lit check

Design spec: [area-refuge-01-spec.md](../../02-art/area-refuge-01-spec.md). Source: `assets/source/props/floor01/refuge_hearth.grid.json` (frame 0 of `burn`, frozen key).

The key is composed at 1x in the lit ENV-02 guild corridor view (`opening_route_proposal.json`, `s6_guild_640x360`, hearth centre at view x 300, base on the floor back line) with the hearth emitters of spec section 7 added to the A1 lighting pass, and with the approved Carl (52 px) and Donut (40 px) idle frames. This is a readability check against the ENV-02 brick, not the room layout.

| File | Content |
| --- | --- |
| `refuge01_hearth_lit_empty_640x360.png` | key alone, lit, 1x |
| `refuge01_hearth_lit_cast_640x360.png` | key with Carl and Donut, lit, 1x |
| `refuge01_hearth_lit_cast_1920x1080.png` | the same at full-res lighting (3x) |
| `refuge01_hearth_lit_cast_crop_4x.png` | 200 x 150 crop of the 1x frame, nearest-neighbour 4x |

Reproduce: `bash scripts/export_assets.sh && python3 scripts/author_refuge01.py && python3 scripts/author_refuge01.py scene <out_dir>`.
