# SMALL-02 evidence

Real Bevy window stills at 640 x 360 (`--scale 1`) for the presentation items of [the SMALL-02 spec](../../02-art/small-02-spec.md). Reuse only: no new art. Each still was captured twice (two full passes, byte-identical) and the window's final `COMBAT_STATE` hash equals the headless hash of the same tick (`capture_lines.txt`, `hash_match=yes`, `passes_byte_identical=yes`). Regenerate with `bash scripts/make_small02_evidence.sh`.

| Still | Command (plus `--capture-sequence`) | What it shows |
| --- | --- | --- |
| `scene_mordecai_idle_0_640x360.png` | `--scene f01_late --f01-room act1_guild --act1-pilot donut`, tick 20 | Mordecai on idle frame 0 (Carl far away) |
| `scene_mordecai_idle_2_640x360.png` | same, tick 70 | idle frame 2, the inhale hold (the loop runs) |
| `scene_mordecai_talk_640x360.png` | same, tick 100 | the talk key (frame 4, open mouth, palm up) with Carl 37 px from his column |
| `scene_boiler_plume_640x360.png` | `--f01-room boiler --room-pilot chain`, ticks 693 to 697 | the landing plume 2 ticks after the weight lands: the left plume rolls away, the right one is dropped at the flush drum (ST-OQ1) |
| `scene_hoarder_post_hit_640x360.png` | `--f01-room hoarder --room-pilot direct`, ticks 446 to 452 | the J-B post two ticks after a counted hit: the white flash and the first splinter burst frame (HC-OQ3) |

No still exists for the spit trace (HEAT-FU-1: dropped, SMALL02-AD4) or the llama ear (SMALL02-Q2: skipped). Nothing here is owner-approved art.
