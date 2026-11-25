# F01-RUN and F01-HUD-ROOM evidence

Window captures (640x360, the debug build under xvfb, no audio, no recording) of the floor run of `--scene f01_late --f01-run` and of the room HUD. The behaviour is documented in [`docs/03-tech/floor01-scene.md`](../../03-tech/floor01-scene.md#the-floor-run-f01-run-and-the-room-hud-f01-hud-room) and the owner instructions are in [the playtest packet](../f01-playtest/README.md).

| File | Command | What it shows |
| --- | --- | --- |
| `hud-room-hoarder-run-start.png` | `--scene f01_late --f01-run --room-pilot hoarder --capture-sequence 600:900:2` (frame 0, tick 600) | the run starts in the first listed room (the Hoarder); the strip reads `THE HOARDER · TRY 1` and shows no TEST LOADOUT tag (the run's own start state) |
| `room-clear-panel-hoarder.png` | the same run, frame 1 (the exit tick, 1385) | the facts-only room-clear panel: ticks and clock, deaths, restarts, the dynamite granted in the room, the run so far, the next room with the carried ledger, the recording state; no score, no rating |
| `hud-room-boiler-test-loadout.png` | `--scene f01_late --f01-room boiler --f01-loadout all --room-pilot direct --capture-sequence 300:100:1` | a room launched on its own under the `all` loadout: `GOBLIN BOILER · TRY 1` and the TEST LOADOUT tag (PT-SCENE U5) |

**Capture note.** A `--capture-sequence` pre-simulates the room to the requested tick before the window draws it, and the combat clock (`CombatState::elapsed`, ticks over every attempt) keeps counting after the exit, so the panel of this capture reads 1500 ticks for an exit reached on tick 1385. In live play the panel opens on the exit tick itself (`F01_RUN_ROOM_CLEARED room=hoarder ticks=1385` in a timed window run of the same pilot). A scripted run shows the panel but never relaunches; the relaunch, the carried ledger and the floor-end panel are covered by unit tests (`f01_run`, `the_floor_run_relaunch_chain_walks_every_listed_room`, `a_carried_floor_run_room_verifies_strict_from_its_descriptor`), not by a capture. **NOT RUN:** a human-played run through all five rooms, on a pad or on real hardware. Game feel is the owner's call.
