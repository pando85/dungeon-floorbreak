# Event log vs camera visibility (DEMO-01 autopilot, measured)

Each row is one audited event of a beat probe (`--demo --demo-beat N --autopilot`). `x` is the body's column at the event tick (`--combat-trace`); the view is the presentation camera of `--view-trace` at that tick. `inside` = at least 48 px from both view edges, `at the edge` = within 48 px, `OUTSIDE` = beyond a view edge. Carl's column is given for reference. The autopilot is not a human: a player who approaches differently sees a different view; this table says what the route's camera showed.

| Beat | Tick | Event | Target | x | Carl x | `--camera auto` (demo default) | `--camera scroll` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| B2 | 75 | B2 contact (goblin hits goblin) | 2 | 1231.4 | 1201.9 | inside (+267 px; view 858..1498, w 640) | inside (+273 px; view 864..1504, w 640) |
| B2 | 75 | B2 contact (goblin hits goblin) | 3 | 1255.2 | 1201.9 | inside (+243 px; view 858..1498, w 640) | inside (+249 px; view 864..1504, w 640) |
| B2 | 125 | B2 lethal kick (heavy) | 2 | 1244.6 | 1226.6 | inside (+279 px; view 884..1524, w 640) | inside (+284 px; view 889..1529, w 640) |
| B2 | 193 | B2 lethal kick (heavy) | 3 | 1317.9 | 1291.3 | inside (+268 px; view 946..1586, w 640) | inside (+274 px; view 952..1592, w 640) |
| B3 | 159 | B3 crush (block) | 4 | 1705.0 | 1618.7 | inside (+216 px; view 1281..1921, w 640) | inside (+218 px; view 1283..1923, w 640) |
| B3 | 159 | B3 decapitation (block edge) | 5 | 1745.0 | 1618.7 | inside (+176 px; view 1281..1921, w 640) | inside (+178 px; view 1283..1923, w 640) |
| B4 | 20 | B4 llama_tell | 17 | 2407.3 | 2131.7 | inside (+79 px; view 1846..2486, w 640) | at the edge (+29 px; view 1796..2436, w 640) |
| B4 | 56 | B4 llama_spit | 17 | 2407.3 | 2242.7 | inside (+203 px; view 1970..2610, w 640) | inside (+140 px; view 1907..2547, w 640) |
| B4 | 277 | B4 llama lethal kick (slam) | 17 | 2470.3 | 2440.9 | inside (+276 px; view 2106..2746, w 640) | inside (+272 px; view 2102..2742, w 640) |
| B5 | 82 | B5 blast (point blank) | 14 | 2735.0 | 2650.8 | inside (+218 px; view 2313..2953, w 640) | inside (+229 px; view 2324..2964, w 640) |
| B5 | 82 | B5 blast (ring) | 15 | 2763.0 | 2650.8 | inside (+190 px; view 2313..2953, w 640) | inside (+201 px; view 2324..2964, w 640) |

## Second-body response after a contact (the B2 target)

| Beat | Contact tick | Target | Track (tick offset: x, mode) |
| --- | --- | --- | --- |
| B2 | 75 | 3 | +0: x=1255.2 Hurt; +5: x=1256.2 Hurt; +10: x=1257.2 Hurt; +20: x=1257.2 Hurt; +30: x=1257.2 Neutral |

## Vertical check

The walking line (y=580) is inside every logged view of every beat and camera, so only the horizontal test above can hide a body.
