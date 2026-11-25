# F01 room exit door evidence: rooms 2, 4, 4a, 4b and 6 (ROOM-PORTALS-01)

Prototype art (`review_status: prototype_fixture_unapproved`); nothing here approves art, tuning or game feel. Spec: [floor01-room-exit-spec.md](../../02-art/floor01-room-exit-spec.md). The independent art-director review passed (silhouette gate included); the owner's art and game-feel review are pending.

The sheet is drawn from the committed grid source by `scripts/author_danger_fx.py`, exported by `bash scripts/export_assets.sh` and rendered by the shared toolkit (`python3 -m scripts.pixelart evidence` and `review-pack`); the stills and the GIF in `scene/` are real windowed captures of `floor01_scene` (the Bevy presentation), each ending on the same state hash as its headless replay.

## Regenerate

```bash
python3 scripts/author_danger_fx.py --meta          # grids, per-grid configs, manifests (dfx_exit among them)
python3 scripts/pixel_grid_to_ase.py build assets/source/props/floor01/dfx_exit.grid.json assets/source/props/floor01/dfx_exit.ase
bash scripts/export_assets.sh                       # atlases under generated/art
c=assets/source/props/floor01/dfx_exit.pixelart.json
python3 -m scripts.pixelart evidence "$c" --out docs/evidence/f01-room-exit/dfx_exit
python3 -m scripts.pixelart review-pack "$c" --out docs/evidence/f01-room-exit/review_pack/dfx_exit
cargo build --locked --bin floor01_scene
bash scripts/make_f01_room_exit_evidence.sh         # scene/: stills at 640x360 and 1920x1080 (two passes), the room 6 clear GIF
```

## What is in this folder

| Folder | Content |
| --- | --- |
| `dfx_exit/` | `contact_sheet_x1.png`, `contact_sheet_x4.png` (both frames, keys marked), `gif_<tag>_x1.gif` and `_x3.gif`, `tell_attack_strip.png` (the state strip), `scene_640x360.png` and `scene_1920x1080.png` (both frames on the lit Floor 1 lane next to Carl and Donut; a toolkit composite), `SHA256SUMS` |
| `review_pack/dfx_exit/` | the three-image review pack (contact sheet x4, lit scene x3, state strip) with the validator report (PASS) |
| `scene/` | six in-scene stills at 640 x 360 and at 1920 x 1080, the room 6 clear GIF, `capture_lines.txt`, `SHA256SUMS` |

## Frames and the state-to-tag map

| Grid | Canvas | Pivot | Frames | Sim state -> tag |
| --- | --- | --- | --- | --- |
| `dfx_exit` | 96 x 136 | (0, 135) | 2 | rooms 4 and 6: the `defeat_roster` exit gate stands (`!roster_cleared()`) -> `exit_closed`; the roster is clear -> `exit_open`; rooms 2, 4a and 4b (no exit gate) -> `exit_open`. Opening = the sidecar exit (`{1072, 484, 64, 96}`; room 4a at x 1968), the canvas 16 px left of it on the walking line, in the Guild door's place |

Frozen keys (`tests/fixtures/f01-dfx-exit_keys.json`): frames 0 and 1.

## In-scene stills (`scene/`)

| File (`_640x360.png`, `_1920x1080.png`) | Room, pilot | Tick | Shows |
| --- | --- | --- | --- |
| `room6_closed` | 6, `mastery:carl_solo:0` | 600 | the llama yard wave with a goblin still up: the grate down over the opening at the right edge of the view |
| `room6_open` | 6, same | 700 | the roster is clear (the last stick still burning): the grate raised into the slot, the lamp-lit passage open |
| `room6_walk_in` | 6, same | 735 | Carl heading for the open exit (the pilot's exit tick is 737) |
| `room4_open` | 4, `hazards:carl_solo:0` | 745 | room 4 after its clear: the raised grate as Carl reaches the exit |
| `room2_open` | 2, `boiler:carl_solo:direct:0` | 830 | the Boiler's exit (no exit gate): open, the workshop pipe in front as before |
| `room4b_open` | 4b `ring_split`, the room's scripted walk-right smoke (no room pilot exists for the Ring rooms) | 250 | the split passage's exit (no exit gate): open, the party walking to it (it reaches the exit on tick 312) |

`room6_clear_640x360.gif`: room 6, ticks 630..738, every 3 ticks at game speed: the last roster enemies fall, the grate snaps up on the clear tick (no lift animation, EXIT-AD3) while the last goblin's defeat presentation still staggers for about 25 ticks, then Carl heads for the exit.

In every replaced room the composer's end torch behind the doorway is not drawn (EXIT-AD7): its flame tip stood on the cap.

`capture_lines.txt` holds one `ROOM_EXIT_STILL` line per still and size and one `ROOM_EXIT_GIF` line: every line ends `hash_match=yes` (`window_hash == headless_hash`) and `passes_byte_identical=yes` (two full capture passes, byte-identical files).

## Simulation unchanged (read-only presentation)

- Every capture's window run and headless replay end on the same `COMBAT_STATE` hash (`capture_lines.txt`).
- `room_exit_art::tests` steps the aligned `mastery:carl_solo:0` and `hazards:carl_solo:0` pilots for 900 ticks with and without drawing: on every tick the drawn frame is `exit_closed` exactly while the world's exit gate block is intact, both frames are seen, and the state hash and event count are equal.
- No golden, pinned hash or replay log was edited. The committed clips of rooms 2, 4 and 6 that show the exit were regenerated with their own scripts (see the PR): their capture lines are byte-identical, only the pictures changed.

## Not covered

- No aligned room 4 route shows the closed grate in view (each reaches the exit's view after its clear); room 4's closed frame is the same code path as room 6's and is held by the unit tests.
- Room 4a (`ring_court`): no scene run reaches its exit's view (no room pilot; the walk-right smoke stops at the closed shutter). Its placement is held by `room_exit_art::tests`.
- Human pad play, 2P captures and a real-GPU frame time were not run.
