# AREA-TROG-01: the troglodyte run and the Test of Strength (rooms `trog_run` and `trog_strength`)

Evidence for [the AREA-TROG-01 spec](../../02-art/area-trog-01-spec.md). At this stage it holds only the design step's silhouette gate (`gate/`): the sixteen frozen key frames, their measurements and pins, and scratch composites of both rooms dressed with the keys. Everything here is **prototype art, design gate, not owner-approved**. No piece is wired into the game yet; the execution (section 13 of the spec) adds the real window captures next to this folder.

## Gate sheets (`gate/`)

| File | Content |
| --- | --- |
| `keys_1x.png`, `keys_4x.png` | every key frame at native size and at 4x, in colour and as a black silhouette: the fist hole, the bell, the knuckle marks, the slam crack, the crater, the threshold, the mirror shards, the three bench states and the six debris frames |
| `bench_states_debris_lit_3x.png` | the bench in its three states and the debris burst frame by frame, lit by the kit's light model, at 3x |
| `gate_overview.png` | the whole gate on one sheet: the five views lit (left) and unlit (right), the key sheet in colour and silhouette, and the bench states and debris burst lit |
| `run_t1_{lit,unlit}_1x.png` | `trog_run`, view left 96: the slam crack and knuckle marks on bricked bay 1, the low refuge face, the crater and the Pygmy |
| `run_t2_{lit,unlit}_1x.png` | `trog_run`, view left 1056: the fist hole on bricked bay 4 with the Basher in front of it |
| `run_t3_{lit,unlit}_1x.png` | `trog_run`, view left 1664: the bell, the bell step face and the Virtuoso |
| `strength_entry_{lit,unlit}_1x.png` | `trog_strength`, view left 0: the spawn, the retreat shelf face and the Virtuoso |
| `strength_bench_{lit,unlit}_1x.png` | `trog_strength`, view left 384: the bench stack (intact), the threshold, the gym bay (girders, cold tube, mirror shards, rack, plate tree, mats), the Virtuoso and the Basher |
| `hero_*_lit_4x.png` | 4x crops of the lit views: the marks, crater and Pygmy; the fist hole and Basher; the bell and step; the shelf, threshold and Virtuoso; the bench, gym bay and Basher |
| `measurements.txt` | the key measurements (size, colours, luma, bounds), the SHA-256 pins of the sixteen key frames, the lit read per view (troglodyte actor L*, edge contrast, each dressing piece's lit mean L*), and the visibility of each placed piece |

**How the composites were made.** The two rooms were composed by the real Rust composer (the plain TROG-A composition), dressed through a scratch contract with `ring_kit::dress_with` (the placements, removals and bricked bays of spec sections 4 and 5), and dumped as layouts by a scratch test. A scratch script rendered each layout with the kit renderer, lit (`env_kit_light.render_lit`) and unlit. It then pasted frame 0 of each troglodyte (facing left) and of Carl (facing right) at their spawn x with the feet on y 579, and the bench at its origin. The six step and shelf faces are scratch versions built by the rule in spec section 5. Views are 640 x 360 with the top at y 292. The probe and the script are not committed. These stills are a design preview, **not runtime evidence**; the execution replaces them with real window captures.

**Key pins.** `python3 scripts/author_trogarea01.py keys` prints the sixteen pins; they must equal the `KEY` lines of `gate/measurements.txt`. `python3 scripts/author_trogarea01.py measure` prints the key measurement lines.

Status: prototype art, design gate, not owner-approved. [`gate/SHA256SUMS`](gate/SHA256SUMS) lists every gate file.
