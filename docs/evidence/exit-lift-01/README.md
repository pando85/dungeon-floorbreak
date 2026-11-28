# EXIT-LIFT-01: the room exit grate lift (evidence)

Spec: [floor01-room-exit-spec.md](../../02-art/floor01-room-exit-spec.md) section 8. Status: owner art review pending, independent review pending. Not owner-approved.

The exit grate lifts over 16 ticks (EXITL-AD1) from the tick the exit gate opens, as a rigid shift of the closed frame's opening clipped by the lintel. The opening tick is an unhashed observer field of the env runtime; gameplay, every state hash and every golden are unchanged.

## Stills

Real windowed captures of `floor01_scene` under xvfb at 640 x 360, at the lift's mid-point (age 8 ticks, a 40 px rise of 80), produced by `scripts/make_f01_exit_lift_evidence.sh`:

| File | Room and plan | Opening tick | Still tick |
| --- | --- | --- | --- |
| `room6_lift_mid_640x360.png` | room 6, `mastery:carl_solo:0` (the last roster enemy falls) | 672 | 680 |
| `room0_corral_lift_mid_640x360.png` | room 0, the corral, `corral:carl_solo:0` (the second M2 llama falls, the M3 gate opens) | 1006 | 1014 |

Room 2 (the Boiler), the hazard room (room 4's exit is out of view in every aligned route) and the Ring rooms are not shown: the Boiler and the Ring rooms have no exit gate (EXITL-AD3), and room 4's exit is not in the camera's view at its opening.

`capture_lines.txt` holds one `EXIT_LIFT_STILL` line per still: every line ends `hash_match=yes` (`window_hash == headless_hash`) and `passes_byte_identical=yes` (two full capture passes, byte-identical files). `SHA256SUMS` lists the files.

## Tests

`cargo test --lib room_exit_art`: the lift offset by tick (`the_lift_offset_follows_the_age`), and, on the aligned room 4 and room 6 pilots stepped through their clear with and without drawing, no lift and no opening tick before the gate opens, the opening tick equal to the first open tick, the overlay's geometry on every lift tick (clipped by the lintel, whole opening on the opening tick, the teeth always under the lintel), the lift lasting exactly 16 ticks, and the state hash and event count of the drawn twin equal to the plain twin's on every tick. The Boiler (no gate) never lifts.

## Not done

Human pad play and a real-GPU frame time were not run. The optional passage brightness lift is not done (EXITL-AD5).
