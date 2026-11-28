# RING-C wire evidence: the moving shutter of `ring_court`

Spec: [docs/03-tech/ring-c-wire.md](../../03-tech/ring-c-wire.md). No new art: the stills show the existing AREA-RINGS-01 shutter frames driven by the simulation.

| File | What it shows |
| --- | --- |
| `scene_warning_640x360.png` | tick 310: the warning (armed at tick about 291 when Carl crossed `arm_x` 1008), nothing moves |
| `scene_closing_640x360.png` | tick 348: the lower edge descends |
| `scene_closed_640x360.png` | tick 420: closed, the heroes cross over the walkway |
| `scene_opening_640x360.png` | tick 532: the edge rises |
| `scene_open_640x360.png` | tick 600: open for good (one closure per attempt) |
| `scene_static_gate_closed_640x360.png` | tick 420 under `--ring-static-gate` (the RING-B gate), for comparison |
| `capture_lines.txt` | one `RINGC_STILL` line per still: the env token, the window state hash, the headless state hash, `hash_match` and `passes_byte_identical` |
| `pin_moment.txt` | the pin and the held edge as a deterministic trace (a goblin kept in the arch): the edge stops at 28 px above the goblin's head, the goblin is staggered once for 36 ticks with no damage, and the edge goes on when it leaves; the run is repeated and identical |
| `SHA256SUMS` | checksums of the files above |

Regenerate: `bash scripts/make_ring_c_wire_evidence.sh` (needs the debug `floor01_scene`, `scripts/export_assets.sh`, `xvfb-run`, ImageMagick) and `cargo test --locked --lib ring_c_pin_trace -- --ignored --nocapture`. Each still is captured in two full passes that must be byte-identical, and the window's final state hash must equal the headless run's.

Not captured: a window still of the pin moment. No scripted window run puts a goblin in the arch (the pilot's goblins stay right of it until Carl arrives), and adding a scene arm for it is out of scope; the trace in `pin_moment.txt` and the unit tests cover it. Frame time of a full cycle: NOT MEASURED in this step (the slices are Timber, so the nav updates stay incremental; the nav-equals-fresh-build test passes).
