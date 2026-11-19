# DEMO-01 evidence (PR A: demo hall, route and respawn)

Real Bevy captures and deterministic replays of the demo hall ([plan](../../04-production/demo-01-plan.md), [Demo section of the scene doc](../../03-tech/floor01-scene.md#demo-demo-01-pr-a-hall-route-and-respawn)). **Mechanical evidence only: nothing here is a game-feel verdict.** The owner plays the demo and answers the [questionnaire](../../04-production/demo-01-questionnaire.md). Beats B4 (llama, PR B) and B5 (test charge, after DEATH-D #108) are empty hooks in this PR; the B4 image shows the hall stretch with its one goblin.

The captures are screenshots of the real Bevy window (lighting on, gore preset 4, 1920x1080 = 640x360 x 3, Xvfb with software Vulkan on the author's machine) while the deterministic autopilot plays the beat from its checkpoint. They are not reproduced bit for bit by CI (rendering varies by driver); the replays are: CI diffs the headless logs and requires the real render loop to reproduce the whole-route events and hash.

## Commands

```sh
cargo build --locked --bin floor01_scene
bash scripts/export_assets.sh

# the probe replays (what CI compares with the committed logs)
bash scripts/ci_floor01_combat.sh demo
xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/ci_floor01_combat.sh demo-render

# one probe by hand, and a single real frame at a tick
./target/debug/floor01_scene --headless-floor01-smoke --demo --demo-beat 3 --autopilot
xvfb-run -a -s '-screen 0 1920x1080x24' ./target/debug/floor01_scene --scale 3 \
  --demo --demo-beat 3 --autopilot --capture-sequence 160:1:1 --capture-dir frames

# regenerate every screenshot in this folder and SHA256SUMS
xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/demo01_evidence.sh
```

## Files

| File | What it shows |
| --- | --- |
| `b0_wakeup_1920x1080.png` | B0, tick 110 of the whole-route run's first beat: Carl running past the first ledge, Donut trailing, the left iron gate |
| `b1_first_rematch_1920x1080.png` | B1, tick 91: the jab-haymaker string kills the first goblin, the second one walks in |
| `b2_kick_contact_1920x1080.png` | B2, tick 76: a kicked goblin has just knocked into the other one (`goblin_contact`), the ledge face behind them |
| `b3_block_falling_1920x1080.png` | B3, tick 140: the shore is broken and the block falls over the two passive goblins; the cavity is cut into plain brick (`solid_backwall` keeps the ground arch away from the shore) |
| `b3_block_landed_1920x1080.png` | B3, tick 160: the block landed (`FLATTENED!` notice), one goblin crushed and one decapitated by the block's edge (DEATH-D: goblin 5 stands on the footprint's right edge), the chaser arrives |
| `b5_lit_charge_fuse_1920x1080.png`, `b5_burst_1920x1080.png` | B5 (a **QA test prop**, never Carl's ability), ticks 60 and 86 from the B5 checkpoint: the lit charge between the huddled goblins, and the burst 4 ticks after it blew (arming when Carl passes column 2640, blast 75 ticks later: one point blank burst, one ring kill thrown intact, the chaser survives and is jabbed afterwards) |
| `b4_llama_hook_1920x1080.png` | B4, tick 120: the stretch where PR B puts the llama; only its chasing goblin is there yet |
| `b6_pileup_1920x1080.png` | B6, tick 110: the pileup cluster at the right end of the hall, the portal scenery behind |
| `state_<shot>.txt` | the `COMBAT_STATE` line of each frame (tick, hit points, goblin positions and health, hash, shore state, `demo_cp`) |
| `headless_demo01_b1.txt`, `_b2`, `_b3`, `_b5`, `_b6` | every `COMBAT_EVENT` and the final `COMBAT_STATE` of each beat probe (200, 260, 300, 300, 300 ticks, started at the beat's checkpoint) |
| `headless_demo01.txt` | the whole route from B0, 2000 ticks: 14 defeats, no death, `DEMO01_COMPLETE` by tick 1506 |
| `SHA256SUMS` | SHA-256 checksums of every file in this folder except this README (`sha256sum -c SHA256SUMS`) |

## What the probes assert

* **B1:** two melee defeats, no death.
* **B2:** at least one `goblin_contact`, one lethal kick (`force=heavy`), one goblin hit twice by kicks (relaunched or downed) and one goblin stopped against the ledge face (x 1360).
* **B3:** three `shore_hit`, one `shore_broken`, one `shore_landed` and at least one `shore_block` defeat from that landing.
* **B6:** six defeats, no death; the checkpoint (2780) is 230 px before the nearest chaser, outside its 220 px sight. The corpse cap of 8 and the stains kept are asserted by `demo::tests::the_route_keeps_eight_corpses_and_every_stain`: the whole route with the death presenter defeats 14 goblins, exactly 8 corpses stay and the floor decals equal their peak (none dropped).
* **Route:** every goblin of the hall defeated, checkpoint 6, no death, one pinned final hash. Every probe gives the same events and hash at gore presets 1 to 4.
* **Respawn** (no probe: the autopilot never dies here): unit tests in `demo::tests` stand Carl still until he is knocked down, check the restart delay, full health, the checkpoint column, defeated goblins staying defeated, living ones back at their spawn and the presenter keeping its stains and corpses.

## Not shown, on purpose

The hybrid-A HUD (#103) is on `main` and draws in `--demo` automatically (the captures show it). No llama (PR B), no automatic camera (PR C). The B3 decapitation and the B5 explosion are DEATH-D (PR #108); both depend on art that is not yet reviewed (the charge fizzle and the burst). **Log re-pin after #108:** every probe's final `COMBAT_STATE` line changed (the hall gained three B5 goblins and a charge, and B3's goblin 5 moved from 1735 to the footprint edge at 1745); the events of `_b1`, `_b2` and `_b6` are unchanged, `_b3` now ends in one crush and one `shore_block_edge` decapitation on the same landing, `_b5` is new and the whole-route log changed because B5 now runs. Frame time and memory figures for B5 and B6 (plan acceptance 6) are not measured here; no performance claim is made. No Steam Deck claim.

## Rights and provenance

Hall composed at load time from the approved Floor 01 kit pieces (no new art); Carl, Donut and the goblins are the committed fan-concept art under the recorded scope in [the rights register](../../04-production/rights-and-release.md). Design: Claude Opus (the DEMO-01 plan); execution: Claude Sonnet (this PR); the independent art-director review does not apply (no new pixel art).
