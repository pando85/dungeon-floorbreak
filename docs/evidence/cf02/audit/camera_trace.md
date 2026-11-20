# Camera trace table (DEMO-01 autopilot beats, measured)

Per-tick `FLOOR1_VIEW` lines of `--view-trace` (output 1280x720, slot 640x360). Pan = change of the view's left edge between ticks. Kick = the impact camera kick in px. Zoom = the view width `w` (640 at rest).

| Beat | Camera | Ticks | Max pan (px/tick) at tick | Ticks with pan >= 8 px | Max kick (px) | Ticks with a kick | Zoom w min..max | First zoom change (tick) | Cuts (pan > 40 px) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| B1 | auto | 200 | 4 at 66 | 0 | 3 | 14 | 640..640 | none | 0 |
| B1 | auto@1920x1080 | 200 | 4 at 66 | 0 | 3 | 14 | 640..640 | none | 0 |
| B1 | scroll | 200 | 4 at 195 | 0 | 3 | 14 | 640..640 | none | 0 |
| B2 | auto | 260 | 4 at 53 | 0 | 3 | 21 | 640..640 | none | 0 |
| B2 | auto@1920x1080 | 260 | 4 at 53 | 0 | 3 | 21 | 640..640 | none | 0 |
| B2 | scroll | 260 | 4 at 251 | 0 | 3 | 21 | 640..640 | none | 0 |
| B3 | auto | 300 | 4 at 216 | 0 | 3 | 7 | 640..640 | none | 0 |
| B3 | auto@1920x1080 | 300 | 4 at 216 | 0 | 3 | 7 | 640..640 | none | 0 |
| B3 | scroll | 300 | 4 at 293 | 0 | 3 | 7 | 640..640 | none | 0 |
| B4 | auto | 400 | 10 at 16 | 10 | 3 | 26 | 640..640 | none | 0 |
| B4 | auto@1920x1080 | 400 | 10 at 16 | 10 | 3 | 26 | 640..640 | none | 0 |
| B4 | scroll | 400 | 4 at 378 | 0 | 3 | 26 | 640..640 | none | 0 |
| B5 | auto | 300 | 4 at 190 | 0 | 3 | 21 | 640..640 | none | 0 |
| B5 | auto@1920x1080 | 300 | 4 at 190 | 0 | 3 | 21 | 640..640 | none | 0 |
| B5 | scroll | 300 | 4 at 191 | 0 | 3 | 21 | 640..640 | none | 0 |
| B6 | auto | 300 | 4 at 38 | 0 | 3 | 35 | 640..640 | none | 0 |
| B6 | auto@1920x1080 | 300 | 4 at 38 | 0 | 3 | 35 | 640..640 | none | 0 |
| B6 | scroll | 300 | 4 at 31 | 0 | 3 | 35 | 640..640 | none | 0 |

## Solo swap pan (authoritative scene camera, legacy rule)

The `solo_swap_continuity` party fixture keeps the heroes 40 to 70 px apart, so its swap moves the camera by 0 px (nothing to measure). The pan needs a gap: the pinned test `cf02_baseline_swap_pan_trace` (in `src/cf02_fixtures.rs`) uses the setup of the party test `q7_a_swap_pan_may_exceed_the_normal_camera_cap_but_only_briefly`: Donut far behind, Carl runs 30 ticks (hero gap 309 px), swap to Donut, then 80 idle ticks. CF02-Q20 replaces this constant pan with an eased one; the test pins the baseline so the change is deliberate.

| Tick after the swap | Camera left (px) | Step (px/tick) |
| --- | --- | --- |
| 1 | 141 | -8 |
| 2 | 133 | -8 |
| 3 | 125 | -8 |
| 4 | 117 | -8 |
| 5 | 109 | -8 |
| 6 | 101 | -8 |
| 7 | 93 | -8 |
| 8 | 85 | -8 |
| 9 | 77 | -8 |
| 10 | 69 | -8 |
| 11 | 61 | -8 |
| 12 | 53 | -8 |
| 13 | 45 | -8 |
| 14 | 37 | -8 |
| 15 | 29 | -8 |
| 16 | 21 | -8 |
| 17 | 13 | -8 |
| 18 | 5 | -8 |
| 19 | 0 | -5 |
| 20 | 0 | +0 |
| 21 | 0 | +0 |

Reading: the pan starts on the first tick at the full 8 px/tick cap (no ramp), repeats the same step for 18 ticks, ends with a shortened step and stops dead (ticks 22 to 80 are all 0). Velocity step on the first tick: 8 px/tick (0 -> 8); on arrival: 5 -> 0.
