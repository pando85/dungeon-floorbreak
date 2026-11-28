# AUD-FIN-01 evidence

Window stills (`bash scripts/make_audfin_evidence.sh`, xvfb, 640x360, scale 1), each captured twice and byte-identical, each with the window's final `COMBAT_STATE` hash equal to the headless run's of the same tick (the notices are presentation only). Status: merged, game-feel review pending (after merge).

| Still | Scene | What it shows |
| --- | --- | --- |
| `scene_intro_640x360.png` | `--scene f01_late --f01-room bos --room-pilot bos_weight`, tick 50 | AUDFIN-AD1: the Ball's intro notice `ROLL CALL!` |
| `scene_crash_640x360.png` | same, tick 470 | `WALL WINS!` after the wall crash at tick 455 |
| `scene_pinned_640x360.png` | same, tick 240 | `DEAD WEIGHT!` after the rack pins the Ball (it lands at tick 173) |
| `scene_defeat_640x360.png` | same, tick 1685 | the defeat notice `BACON BOWLED!` |
| `scene_blown_away_640x360.png` | `--fixture qa_blast_breakup`, tick 70 | AUDFIN-AD3: a goblin blown apart at point blank shows `BLOWN AWAY!`, not `FLATTENED!` |
| `floor_end_panel.txt` | `cargo test --lib the_floor_end_panel_shows_the_viewers_at_the_exit -- --nocapture` | AUDFIN-AD2: the `FLOOR 1 CLEARED` panel text with the `viewers at the exit` line |

Notes:

* **No window still of the floor-end panel.** A scripted run cannot cross a room exit (ROOM-PILOT-RUN, open), and the Ball cannot be beaten without a pilot in a live run, so the panel is shown as the text of the real `floor_end_panel` function with a carry of 8,000 and 4,345 viewers.
* **A notice can wait behind a live one.** The queue shows one notice at a time (136 ticks, then a gap). In the pilot's long fight a wall crash is on screen when the Ball falls (tick 1523), so `BACON BOWLED!` starts at about tick 1670. The Juicer's `KNOCKOUT!` has always queued the same way. The defeat still is therefore taken at tick 1685. Recorded as a risk in the PR; a boss-defeat headline that pre-empts the live one is a possible follow-up.
* The same queue delays the others: in the pilot's run `ROLL CALL!` is on screen from tick 36 to about 172, so the first crash (tick 133) is replaced by the pin (tick 173) and `DEAD WEIGHT!` starts at about 202; `WALL WINS!` shows after the crash at tick 455, when the queue is free.
* The slime and Ball death profiles and the audience room pins have no pixels: their evidence is `cargo test` (`the_slime_and_the_ball_have_their_own_profile_rows`, `every_run_room_resolves_to_its_audience_state`).
