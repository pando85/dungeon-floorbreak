# LLAMA-03 rush: evidence (gate images, atlas, in-room captures)

Evidence for the Bad Llama rush ([spec](../../02-art/llama-03-rush-spec.md)): the design-stage gate images (regenerated after the gate's K1 and K2 edits), the ten-frame atlas contact sheet and GIF, and the in-room window captures of the rush art. Nothing here is accepted: the owner's art and game-feel review is still to come.

The gate images (`gate_*`) are **composites, not runtime captures.** The four design keys are pasted by their pivot onto the merged CORRAL-WIRE window still (`../corral-wire/still-corral-640x360-tick360.png`, which already holds Carl and Donut as the runtime draws them), mirrored to face Carl as the runtime flip does. The goblin, the dust-trail mock and the lane band are composited too. The dust puffs use the CORRAL-E stand-in's dust tone and the lane band is a neutral white review annotation, not game art. The gate images regenerate byte-identically with `python3 scripts/author_llama03_rush.py gate`, the contact sheet and GIF with `python3 scripts/author_llama03_rush.py evidence`; The contact sheet and GIF are rendered from the atlas grid. The `room-rush-tick*.png` stills, `captures.txt` and `f01-run-smoke.txt` are **Bevy runtime captures** of the rush art (window frames of `floor01_scene` under xvfb, unscaled). `SHA256SUMS` covers the PNG files and `tests/test_art_llama03_rush.py` checks it.

| File | What it shows | How it was made |
| --- | --- | --- |
| `gate_silhouettes_1x.png` | One contact row, facing right: Carl, Donut and a GOB-01 idle, the LLAMA-01 idle, spit tell (frame 8) and spit release (frame 9), then the four rush keys (brace, scrape, run_contact, recover_skid). Colour on a dark brick tone above, black silhouettes on a pale tone below. 1x. | `scripts/llama03_rush_gate.py` `write_silhouettes` |
| `gate_silhouettes_4x.png` | The same sheet enlarged x4 by nearest neighbour, labelled | same |
| `gate_corral_strips_1x.png` | Seven 340x104 strips of the corral at 1x (crop x 300..640, y 196..300 of the still), the llama facing Carl: LLAMA-01 idle at the 112 px approach distance; spit tell at 112; brace at 112 with a goblin between them; scrape at 112; run_contact at 64 with the dust-trail mock; recover_skid at 64; the brace with the lane band | `write_corral` |
| `gate_corral_strips_3x.png` | The same strips enlarged x3, labelled | same |
| `gate_corral_640x360_brace.png` | The full 640x360 frame of the brace-with-goblin strip, 1x | same |
| `gate_corral_640x360_run.png` | The full 640x360 frame of the run_contact strip, 1x | same |
| `contact-sheet.png` | The ten atlas frames: a 1x row, then each frame at x4 with its name and tag; the four frozen keys are marked KEY | `python3 scripts/author_llama03_rush.py evidence` (`write_atlas_evidence`) |
| `rush.gif` | The timed animation at x3: tell 32 ticks, one 24-tick run (active ticks 1 to 24), recover 40 ticks (skid 10, blown 30). Whole-frame durations merged from 60 Hz ticks (10 ms GIF units), so the loop is 1600 ms | same |
| `room-rush-tick742.png` | 640x360 window still at 1x, tick 742: the tell starts (`brace_in`) | `scripts/room_capture.sh corral corral_rush 736:2:50 OUT --f01-run`, frame of tick 742 copied unscaled |
| `room-rush-tick748.png` | tick 748: the brace (tell tick 6) | same |
| `room-rush-tick760.png` | tick 760: a scrape beat (tell ticks 16 to 19), dust and gouge at the paw | same |
| `room-rush-tick774.png` | tick 774: the commit tick, the push frame | same |
| `room-rush-tick786.png` | tick 786: mid-run | same |
| `room-rush-tick790.png` | tick 790: the last run frame before the hit (muzzle overlap, see Readings) | same |
| `room-rush-tick794.png` | tick 794: the skid (recover elapsed 3) | same |
| `room-rush-tick806.png` | tick 806: blown (recover elapsed 15), tongue out | same |
| `captures.txt` | The three `ROOM_CAPTURE` lines (with `--f01-run`, without it, and `--deck`), each `match=yes` | `scripts/room_capture.sh` |
| `f01-run-smoke.txt` | The headless `--scene f01_late --f01-run` smoke lines and the window run's log lines: `LLAMA_RUSH_ART on frames=10`, zero fallback lines | `floor01_scene --headless-floor01-smoke --scene f01_late --f01-run` |

## Readings

- **Rush and spit separate by shape at 1x.** The LLAMA-01 idle is upright (top row 8) and the spit tell rears higher with the lit throat; the brace, scrape and run are long low wedges whose head sits at Carl's chest height pointing at him (top rows 24, 24, 27). The black silhouettes keep the difference with no colour.
- **The brace reads on the brick.** The tan body with its dark outline separates from the wall and the dark trough. The faint brace of the stand-in (CORRAL-E reading) is carried here by the pose itself: head down the lane, ears pinned, tail up.
- **The scrape beat reads as a paw**, not a step: the near fore hoof moves 11 px back under the chest (x 61 to 50) while the hind hooves stay on the brace footprint (gate K2).
- **The goblin stays readable** in front of the braced llama's legs (green on tan).
- **The recover reads as winded**, not as a new threat: the head pitched down below the withers, the mouth open with the tongue, no glow.
- The lane band (the box's 96 px travel plus its 28 px length, heights 2 to 30) covers Carl's legs and lower torso; the braced head points along its top edge, so the pose aims where the hit lands. The band is not proposed for the game (LL3-AD7).

Runtime readings (the stills above, the debug build under xvfb, 1x):

- **The rush art is on by default.** `--scene f01_late --f01-run` prints `LLAMA_RUSH_ART on frames=10` (headless and window) and the window run's log has no fallback line. With the manifest removed the scene prints one `LLAMA_RUSH_ART fallback to the stand-in body` line, keeps the stand-in body and reaches the same smoke `state_hash`.
- **Gameplay is unchanged.** The headless smoke `state_hash=0x2d847e416b946726` of `--f01-run` equals the merged CORRAL-WIRE evidence, and the headless hashes at tick 794 of `corral_rush` (`aabf4ee965f78447`, deck `840e49d9179e7282`) equal the merged CORRAL-F captures. Every window capture ends on the headless hash (`match=yes`).
- **The tell reads as a coming rush.** The head is down the lane at Carl's chest height, the ears are pinned and the back mane spiked; the scrape beats put the paw back with the dirt starting at the hoof. At 1x the llama is a long low tan wedge on the brick, clear of the cast.
- **The run is a heavy low ram.** The legs flail in a four-frame cycle of three ticks a frame and the dust trail stays behind the hind hoof.
- **Contact overlap (gate N3).** At tick 790, the last run frame before the hit, the llama's pivot is at x 1416.7 and Carl's at 1376 (the plan's park position), so the muzzle (45 px ahead of the pivot, facing left) is at about x 1371.7: about 4 px past Carl's centre line and about 16 px inside his 24 px hurt box. The rush box (28 px ahead) first touches him on tick 791, which already draws the skid. At the end of range the muzzle can poke up to 17 px past the box front into a hero the box never reaches.
- **The recover reads as winded.** The skid shows the open mouth with the tongue hanging below the jaw and the head pitched below the withers; at 1x the limp near ear still reads like a collar (gate N1).
- **Silhouette.** In black the run reads as a generic shaggy quadruped, close to a hyena (gate N2); the colour, the teeth and the transition from the idle carry the llama.

## NOT RUN

- A rush that ends on a bonk (a solid or the partition) in a window: the `corral_rush` plan produces a hit, so the bonk frames (the hurt frame, then `recover_blown`) are covered by unit tests only.
- A whiff and a leash stop in a window (unit tests cover the phase map).
- A real pad session, a human run and an owner game-feel review.
