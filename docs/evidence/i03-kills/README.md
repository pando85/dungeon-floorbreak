# I03-KILLS PR A evidence (draft, owner review pending)

Headless traces and real Bevy frames of the goblin defeat outcomes D-01 to D-04, from `src/bin/floor01_scene.rs` fixtures on the corridor's clean ground (not the normal story route). Nothing here is a playtest or a feel approval.

| File | What it shows |
| --- | --- |
| `trace_cp02_kick_contact.txt`, `trace_cp03_shore_kill.txt`, `trace_cp03_shore_miss.txt` | `--headless-floor01-smoke --fixture <name> --combat-trace`: one `COMBAT_TRACE` row per tick (the shore rows end with `shore=<hits>/<prop>/<block>`), then the events and the final state line |
| `d01_haymaker_kill_...tick152_640x360.png` | D-01: haymaker kill, recoil frame during the lethal hit-stop |
| `d02_kick_launch_...tick140_640x360.png` | D-02: Triangle/Y kick, recoil frame during the kick's hit-stop (the launch follows) |
| `d03_contact_...tick47_640x360.png` | D-03: contact tick; the anim trace draws `g0=A/knockback_contact:26`, `g1=B/hurt:16` |
| `d04_landing_NO_SHORE_ART_...tick139_640x360.png` | D-04: landing tick; the goblin plays the temporary GOB-01 `down` frame. **No shore, block or dust is drawn yet** (the fixture scene does not load the ENV-02 opening kit), so this frame cannot show the block |

Frames: `xvfb-run ... floor01_scene --scale 1 --fixture <f> --capture-sequence <t-5>:1:6 --capture-dir <dir>`, the last of six frames (the first frames of a run can be black). Single frames, not full-speed footage.
