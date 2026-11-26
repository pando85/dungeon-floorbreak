# HUD-TRY-01: the room HUD's `TRY n` counts checkpoint respawns

**Status:** implemented, game-feel review pending. Display-only: no state-hash, golden or pin change.

## Behavior

Before this change the location strip of a `--scene f01_late` room showed `TRY n` with n = 1 + the room's restarts
(held-Select restart or a party-defeat retry). A death followed by the DEMO-01 checkpoint respawn (and a Select tap
respawn) kept the same number, so a room with one death still read `TRY 1` (pre-existing since #270).

**Adopted default HT-AD1:** every attempt start counts. `CombatState::hud_try` = 1 (the room's entry) + `restarts`
+ `respawns` (each checkpoint respawn, after a defeat or a Select tap). A restart does not reset the respawn count, so
the number only goes up within a room. The new `respawns` counter is not hashed and is read only by the HUD.
Unchanged: `restarts` (and with it the CF02 temperament draw), the room-clear panel's deaths and restarts,
`F01_RUN_ROOM_CLEARED`, the run tally, PLAYTRACE `combat.restarts` and the owner scorecard columns (deaths = hero
defeats, restarts = Select restarts).

## Evidence

Room 3 (`--f01-room dozer`), aligned pilot `dozer:retry` (the wheel takes Carl on tick 273, checkpoint respawn on
322), 640x360, `--scale 1`, captured with `scripts/room_capture.sh` twice per still.

| File | Tick | HUD |
| --- | --- | --- |
| `retry_down_640x360.png` | 300 | `THE MACHINE REMEMBERS · TRY 1` (down, before the respawn) |
| `retry_respawn_640x360.png` | 330 | `THE MACHINE REMEMBERS · TRY 2` (after the respawn) |
| `retry_gate_640x360.png` | 438 | `TRY 2` (the machine leaves the gate on the second try) |
| `retry_respawn_hud_before_after_2x.png` | 330 | top 40 rows at 2x: the DR-D still (`TRY 1`, top) over this PR (`TRY 2`, bottom) |

`capture_lines.txt` records, per still: window hash = headless hash, the window hash equal to the DR-D capture of
the same tick (`docs/evidence/f01-dozer-revenge/capture_lines.txt`), and both passes byte-identical. `retry_down` is
byte-identical to the DR-D still; the two post-respawn stills differ from DR-D only inside the location strip
(changed pixels bounded by a 231x16 box at x 205, y 8).

The DR-D stills `retry_respawn`, `retry_gate`, `retry_escape` (both sizes) and `retry_640x360.gif` in
`docs/evidence/f01-dozer-revenge/` were captured before this change and still show `TRY 1` after the respawn; they
are left as committed. Other room evidence captured after a checkpoint respawn has the same pre-HT-AD1 count.
