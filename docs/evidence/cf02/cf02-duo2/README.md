# CF02-DUO rounds 2 and 3 evidence

Both GIFs are 1x captures (640x360, a frame every second tick) of the real render loop replaying a golden fixture. They show the committed replays, not a game-feel verdict: nobody has played this build. The frames contain only the committed original Carl, Donut and goblin art under the recorded scope in the [rights register](../../../04-production/rights-and-release.md); no new art. The last state line of each is in its `state_*.txt`; hashes are in `SHA256SUMS` (`sha256sum -c SHA256SUMS`).

[`donut_guards_carls_back_1x.gif`](donut_guards_carls_back_1x.gif): 28 frames, ticks 184 to 238, `floor01_scene --scale 1 --party-fixture cf02_assist_donut_guards_carls_back --capture-sequence 184:2:28` (regenerated after the A15 flank and the A12-before-A11 fix: the kill on tick 205 is unchanged). Carl stands at the right of the frame; the AI Donut (profile `cf02_assist`, biscuit still locked) pounces the goblin that walked up behind him and defeats it on tick 205.

[`donut_flank_pounce_1x.gif`](donut_flank_pounce_1x.gif): 30 frames, ticks 62 to 120, `floor01_scene --scale 1 --party-fixture cf02_assist_donut_flanks_two_goblins --capture-sequence 62:2:30` (A15). Carl has stopped; the AI Donut stands on the flank spot 56 px ahead of him, pounces the first chasing goblin on tick 67 and lands the first hit on tick 83 (2 HP to 1 HP, before it reaches him); Carl steps up and jabs it (tick 110, defeat 113); her next pounce at 116 hits the second goblin on 128 and Carl finishes it on 139.

Numbers and the A11 to A15 values are in the [companion spec](../../../03-tech/companion-ai-command-system.md#e-cf02-duo-adopted-defaults-ai-donut-as-a-fighting-partner).
