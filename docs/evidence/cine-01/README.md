# CINE-01 evidence: skill-gated cinematic finishers

Presentation-only slow motion for exceptional, player-authored defeats. This
folder holds the evidence of the implementation PR. **Game feel is pending:** every
number is the spec's adopted starting hypothesis, not an owner-approved value.
Spec: [cinematic-finishers-cine-01.md](../../03-tech/cinematic-finishers-cine-01.md).

## The B3 moment

DEMO-01 B3 (`--demo --demo-beat 3 --autopilot`): `shore_broken` at tick 123, then
at tick 159 target 4 `shore_block` and target 5 `shore_block_edge`, no hurt. The
evaluator emits `CINE-PRECISION-EDGE`, score 5 + 1 extra kill = 6, ordinary, at
**tick 159** (the haymaker kill at tick 243 sits inside the 900-tick cooldown).

| File | What it is |
| --- | --- |
| `b3_moment_as_played.gif` | Real 1920x1080 Bevy frames of ticks 150 to 210 (`--capture-sequence 150:1:61`, nearest-neighbour to 640x360 and cropped), each shown for its **as-played real duration**: 16.7 ms at 1.0x, `16.7 / speed` ms inside the moment (the spec's 35 ms in, hold at 0.35x, 90 ms out envelope). The slowdown itself is real-time behaviour; the capture sequence steps ticks by hand, so the timing is reconstructed from the planner's envelope, not recorded |
| `b3_contact_sheet.png` | Ticks 156, 158, 159, 160, 164, 170, 180 and 190 of the same sequence: the block lands, the edge shears the upright goblin's head, the crushed one lies under it |
| `windowed_b3_cinematic_on.txt`, `windowed_b3_cinematic_off.txt` | `COMBAT_EVENT`, `CINE01_MOMENT` and the final `COMBAT_STATE` of the **windowed** binary, `--cinematic-finishes=on` and `=off`: byte-identical (CINE-T09) |
| `windowed_pileup_cinematic_on.txt`, `windowed_pileup_cinematic_off.txt` | The same for `cp03_goblin_pileup` (`CINE01_MOMENT tick=139`, mixed chain, score 7) |
| `slowmo_measurements.txt` | `CINE01_SLOWMO start/end` lines of the real loop: real length, fixed ticks run, frames, worst frame and most fixed ticks any frame ran, at 640x360 and 640x400, 1x to 3x |

## Commands

```bash
cargo build --locked --bin floor01_scene
bash scripts/export_assets.sh
# CINE-T09 / T03 / T05 / T11 / T12 (real Bevy loop; needs a display)
xvfb-run -a -s '-screen 0 1920x1200x24' bash scripts/ci_floor01_combat.sh cine
# pure and fixture tests
cargo test --locked --lib cine
cargo test --locked --test cine_fixtures
# one windowed probe by hand
./target/debug/floor01_scene --scale 3 --demo --demo-beat 3 --autopilot \
  --exit-after-ticks 300 --cinematic-finishes=on --gore-preset 4
```

`ci_floor01_combat.sh cine` runs B3 and the pileup at gore presets 1 and 4 with CINE
on and off. The `COMBAT_EVENT`, `CINE01_MOMENT` and final `COMBAT_STATE` lines must be
identical across all four runs and equal the committed headless replays; with CINE on
the moment must start and end `Completed`, with CINE off no `CINE01_SLOWMO` line may
appear.

## Test coverage (CINE-T01 to T13)

| Test | Covered by | Status |
| --- | --- | --- |
| T01 | `cine_fixtures::t01_t02_*` (all 20 fixtures: routine, kicks, llamas give no moment), `cine_adapter::a_force_tier_never_decides_anything`, `cine::t01_*` | covered |
| T02 | the same fixture test (`qa_blast_breakup` `Explosion` defeats, the lone shore crush give nothing), `cine::t02_*` | covered |
| T03 | `cine_fixtures::t03_*` (`cp03_goblin_pileup`, tick 139, score 7, one moment), `cine::t03_*` | covered |
| T04 | `cine::t04_*`, `cine::f2_*` (window, clean window, unbroken block) | covered (pure) |
| T05 | `cine_fixtures::t05_*` (`cp03_block_edge_decap`, B3 tick 159, score 6), windowed script | covered |
| T06 | `cine_adapter::t06_*` (launch proof from `Hit{PushKick}` + `GoblinContact.launch`, third kill: legendary 8), `cine::t06_*` | covered (synthetic events: no fixture yields a legendary chain) |
| T07 | `cine_fixtures::t07_*` (restart, monotonic clock), `cine::restart_and_respawn_*` | covered |
| T08 | `cine::t08_*`, `cine::r3_*` (primaries do not stack) | covered |
| T09 | `cine_fixtures::t09_*` (pure parity over every fixture and B3), windowed on/off script at gore 1 and 4 | covered |
| T10 | `cine::t10_*` (Carl credited, Donut hurt, solo-swap split), `cine::f1_*`, the existing `PadSlots` latch tests; **the live two-controller check is manual** | pure covered, live **manual, not run** |
| T11 | `cine::t11_*`, `cine::f10_*` (hit-stop overlap), `cine_adapter::restart_respawn_and_party_defeat_*`, `cine_adapter::the_evaluator_runs_with_cine_off_*`; the planner cancels on the D6 pause and `Frozen` (`cine_frame`) | covered; the D6 pause and `Frozen` paths are covered at the planner level, not by a live pad unplug |
| T12 | windowed script at 640x360 and 640x400, 1x to 3x (`slowmo_measurements.txt`); `b3_contact_sheet.png` | partly: the moment completes and returns to 1.0x at every framing; the **live emphasis frames are not screenshot-captured** (the capture sequence is a locked path) |
| T13 | `cine::t13_*` (40 simultaneous defeats, bounded history), `cine_fixtures::t13_*`, `max_ticks_in_frame` in the measurements | covered; frame times are the existing 3x software-rendering cost, report only |

## Measurements (real loop, xvfb, debug profile)

At 640x360 1x: 482 ms real, 14 fixed ticks, worst frame 22 ms, at most 2 fixed ticks
in a frame. At 3x (1920x1080 lit) frames cost 50 to 100 ms in this environment (the
existing render cost, not CINE) and the loop ran at most 4 to 5 ticks in a frame, so
there is no unbounded catch-up. These numbers do not replace the owner's play on
real hardware.

## Rights/Provenance

No new art. Capture frames are real renders of the existing approved Carl, Donut and
GOB-01 assets. Designed by the spec (Claude Opus), executed on the medium tier,
independent top-tier review pending.
