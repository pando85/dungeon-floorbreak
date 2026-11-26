# LLAMA-03 rush: silhouette gate evidence (design keys)

The design-stage gate images for the Bad Llama rush ([spec](../../02-art/llama-03-rush-spec.md)). Nothing here is accepted: the independent gate review, the execution step (in-betweens, atlas, Bevy wiring, runtime captures) and the owner's game-feel review are still to come.

These are **composites, not runtime captures.** The four design keys are pasted by their pivot onto the merged CORRAL-WIRE window still (`../corral-wire/still-corral-640x360-tick360.png`, which already holds Carl and Donut as the runtime draws them), mirrored to face Carl as the runtime flip does. The goblin, the dust-trail mock and the lane band are composited too. The dust puffs use the CORRAL-E stand-in's dust tone and the lane band is a neutral white review annotation, not game art. All images regenerate byte-identically with `python3 scripts/author_llama03_rush.py gate`; `SHA256SUMS` covers the PNG files and `tests/test_art_llama03_rush.py` checks it.

| File | What it shows | How it was made |
| --- | --- | --- |
| `gate_silhouettes_1x.png` | One contact row, facing right: Carl, Donut and a GOB-01 idle, the LLAMA-01 idle, spit tell (frame 8) and spit release (frame 9), then the four rush keys (brace, scrape, run_contact, recover_skid). Colour on a dark brick tone above, black silhouettes on a pale tone below. 1x. | `scripts/llama03_rush_gate.py` `write_silhouettes` |
| `gate_silhouettes_4x.png` | The same sheet enlarged x4 by nearest neighbour, labelled | same |
| `gate_corral_strips_1x.png` | Seven 340x104 strips of the corral at 1x (crop x 300..640, y 196..300 of the still), the llama facing Carl: LLAMA-01 idle at the 112 px approach distance; spit tell at 112; brace at 112 with a goblin between them; scrape at 112; run_contact at 64 with the dust-trail mock; recover_skid at 64; the brace with the lane band | `write_corral` |
| `gate_corral_strips_3x.png` | The same strips enlarged x3, labelled | same |
| `gate_corral_640x360_brace.png` | The full 640x360 frame of the brace-with-goblin strip, 1x | same |
| `gate_corral_640x360_run.png` | The full 640x360 frame of the run_contact strip, 1x | same |

## Readings

- **Rush and spit separate by shape at 1x.** The LLAMA-01 idle is upright (top row 8) and the spit tell rears higher with the lit throat; the brace, scrape and run are long low wedges whose head sits at Carl's chest height pointing at him (top rows 24, 24, 27). The black silhouettes keep the difference with no colour.
- **The brace reads on the brick.** The tan body with its dark outline separates from the wall and the dark trough. The faint brace of the stand-in (CORRAL-E reading) is carried here by the pose itself: head down the lane, ears pinned, tail up.
- **The scrape beat reads as a paw**, not a step: the near fore hoof moves 9 px back under the chest on the same footprint.
- **The goblin stays readable** in front of the braced llama's legs (green on tan).
- **The recover reads as winded**, not as a new threat: the head pitched down below the withers, the mouth open with the tongue, no glow.
- The lane band (the box's 96 px travel plus its 28 px length, heights 2 to 30) covers Carl's legs and lower torso; the braced head points along its top edge, so the pose aims where the hit lands. The band is not proposed for the game (LL3-AD7).

## NOT RUN

- Runtime captures of the rush art in a window and the window-vs-headless hash line: the art is not wired into Bevy at this stage (execution step, `scripts/room_capture.sh corral corral_rush 736:2:50 DIR`).
- The in-betweens, the atlas, a contact sheet and a GIF of the timed animation (execution step).
- A real pad session, a human run and an owner game-feel review.
