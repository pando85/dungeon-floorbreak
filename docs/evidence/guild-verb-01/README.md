# GUILD-VERB-01 evidence: the Interact verb in `act1_guild`

Four 640 x 360 window stills of the real Bevy scene (xvfb, `--scale 1`, `--scene f01_late --f01-room act1_guild --act1-pilot interact --companion off`, the debug binary, the exported atlases). The `interact` pilot walks Carl to x 316, presses Interact on the grant strongbox (accepted on tick 67, snapped to x 323), and after the verb presses it again for the guide. Each still was captured in two full passes, the two PNGs are byte-identical, and the window's final `COMBAT_STATE` hash equals the headless run's of the same tick (the art and the animation are presentation only). The lines are in `capture_lines.txt` (`GUILDV01_STILL ... hash_match=yes passes_byte_identical=yes`), the checksums in `SHA256SUMS`. The art is existing approved material (ANIM-INTERACT-01, AREA-ACT1-01); owner art and game-feel review pending. No getup still: it is not wired (GUILDV-Q3 in [interact-verb.md](../../03-tech/interact-verb.md)).

| Still | Tick | What it shows |
| --- | --- | --- |
| `scene_interact_carl_reach_640x360.png` | 71 | Carl's hand verb at clock 4: the reach (`u0`) before the grip; the strongbox is still closed |
| `scene_interact_carl_haul_640x360.png` | 81 | clock 14: the haul-down key (`u2`); the strongbox opened on the contact tick (72) |
| `scene_interact_box_open_640x360.png` | 97 | the last verb tick (`u3`, let go); the strongbox open |
| `scene_interact_guide_talks_640x360.png` | 130 | Carl stands 57 px from Mordecai's column, outside his 48 px proximity key; his `talk` key is held by the second Interact press |

Regenerate: `cargo build --locked --bin floor01_scene`, `bash scripts/export_assets.sh`, `bash scripts/make_guildverb01_evidence.sh`.
