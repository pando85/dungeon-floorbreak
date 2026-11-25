# The Infernal Corral: CORRAL-WIRE and CORRAL-E evidence

Room 0 of the late-Floor-1 slice, launched with `--scene f01_late --f01-room corral` ([floor01-scene.md](../../03-tech/floor01-scene.md#the-infernal-corral-corral-wire-and-corral-e)). The room is **launchable but unlisted** (plan section 14.7): a floor run refuses it until CORRAL-F has landed and CORRAL-AUTO passes. Nothing here is accepted: game feel is the owner's call, and the rush draw is a **code-drawn stand-in** over the reused LLAMA-01 idle frames until LLAMA-03.

All stills are real `floor01_scene` window captures under Xvfb (`xvfb-run -a -s '-screen 0 1280x800x24'`), lit, with the exported atlases (`bash scripts/export_assets.sh`), at scale 1. The capture runs are scripted (no synthetic input reaches a window), so each is a function of its command line.

| File | What it shows | How it was made |
| --- | --- | --- |
| `still-corral-640x360-tick60.png` | 640x360, tick 60: the room's opening, Carl and Donut at the spawn with M0's two goblins awake | `floor01_scene --scene f01_late --f01-room corral --scale 1 --no-audio --capture-sequence 60:300:2 --capture-dir DIR` |
| `still-corral-640x360-tick360.png` | 640x360, tick 360 of the same run: the scripted walk-right (no jump) stops at the first step; the M1 llama (no rush opt-in) spits from behind | same run |
| `still-corral-640x400-tick60.png`, `still-corral-640x400-tick360.png` | The same two ticks at 640x400 (`--deck`) | the same command with `--deck` |
| `rush-stand-in-1x.png` | The rush stand-in at 1x (200x90 crops of 640x360 window captures). Top row, `corral_rush_bonk_wall`: the brace at tick 18 (lean back, the hoof scrape's flecks and dust puff behind the hind hoof), the charge at tick 40 (lean in, the dust trail behind), the bonk at tick 52 (the burst at the front of the box, rock back). Bottom row, `corral_rush_hits_idle_carl`: the brace at tick 18, the charge at tick 40, the stop at tick 60 (the trail settling). The throat glow never plays. | `floor01_scene --party-fixture NAME --scale 1 --no-audio --capture-sequence 2:2:40 --capture-dir DIR`, cropped at (420, 210)..(620, 300) |
| `rush-stand-in-x3.png` | `rush-stand-in-1x.png` enlarged x3 by nearest neighbour, for reading only | Pillow `resize(..., NEAREST)` |

## Readings

- The headless smoke prints `F01_CORRAL_ENTRY ... roster=5 goblins=2 llamas=3 m3_gate=2048..2064_on_llamas_1+2 listed=false env=live` and `F01_CORRAL_SMOKE_OK ticks=600 checkpoint=1 pin=f01_pin_corral_entry env=live slot_enemies=0 enemies=2 state_hash=0x2d847e416b946726`; `scripts/ci_floor01_combat.sh headless` replays story, `all` and seed 7 twice each, identical, and checks that `--f01-run` refuses the room.
- The window run prints `CORRAL_GATE drawn at x 2048..2064 (greybox stand-in, no art)` and `CORRAL_RUSH_STAND_IN pool 2 llamas (code-drawn stand-in, LLAMA-03 pending)`.
- **The brace is faint at 1x.** In the first capture the dark gouge and the two 1 px flecks sank into the lit floor, so the brace read as an idle llama. Each scrape beat now also kicks a 2x2 pale dust puff (the dust tone); it reads at 1x but stays small. The lean (2 px) is barely visible at 1x. This is a stand-in: LLAMA-03 carries the real brace pose.

## NOT RUN

- A window still of the M3 grate: no scripted window run reaches x 2048 (the walk-right script does not jump, and no room pilot covers the corral yet). The grate's draw rule (`!exit_gate_open()`) is the simulation's, which the fixtures test headless.
- A rush in the room itself in a window: the walk-right script stops before B's trigger. The 1x stills come from the CORRAL-B party fixtures, whose llama carries the same rush state the stand-in reads.
- A real pad session, a human run and an owner game-feel review.
