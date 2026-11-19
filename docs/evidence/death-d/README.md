# DEATH-D evidence: the decapitation and the explosion in the real Bevy scene

**Status:** real `floor01_scene` captures (xvfb, the real render loop, lighting on, software Vulkan, one frame per tick) of the two DEATH-D fixtures at gore preset 4 (the default) and preset 1, at 640x360 and 1920x1080. **Nobody has judged the look or the feel yet; owner game-feel review and independent review are pending.** The charge fizzle (`vfx_dynamite`) and the blast burst (`vfx-01`) are **unreviewed prototype art** (`prototype_fixture_unapproved`), drawn here in the QA fixtures only, which approves nothing. Design and behaviour: [DEATH-D in the floor 01 scene doc](../../03-tech/floor01-scene.md#death-d-decapitation-and-explosion-finishes) and the [spec](../../03-tech/death-d-finishes-spec.md).

## Reproduce

From the repository root (repo-relative paths; needs `xvfb-run`, LibreSprite or Aseprite for the export step, and Pillow):

```
bash scripts/export_assets.sh
cargo build --locked --bin floor01_scene
python3 scripts/make_death_d_evidence.py capture   # per-tick PNG frames into artifacts/death-d (git ignored)
python3 scripts/make_death_d_evidence.py build     # stills, sheets, GIFs, SHA256SUMS
python3 scripts/make_death_d_evidence.py perf      # frame-interval probe of --death-stress
```

One capture run is, for example, `xvfb-run -a -s '-screen 0 1920x1080x24' ./target/debug/floor01_scene --scale 3 --fixture qa_blast_breakup --gore-preset 4 --capture-sequence 50:1:191 --capture-dir artifacts/death-d/blast_p4_1920x1080` (`--scale 1` for 640x360). The headless replay is `./target/debug/floor01_scene --headless-floor01-smoke --fixture cp03_block_edge_decap`.

## Replay is identical at every preset and resolution

Every capture run prints its final `COMBAT_STATE`; the script fails unless it equals the headless replay of the same tick, at both presets and both resolutions: `cp03_block_edge_decap` to tick 259 (hash `62ce22e426262a4f`) and `qa_blast_breakup` to tick 240 (hash `c3d2b3703f495284`, the committed fixture hash). Unit tests and `scripts/ci_floor01_combat.sh headless` also run both fixtures at presets 1 to 4 against the committed replays (`docs/evidence/cp01-feel/headless_cp03_block_edge_decap.txt` and `headless_qa_blast_breakup.txt`).

## What is here

| File | Content |
| --- | --- |
| `still_decap_p{4,1}_L{-2,+0,+4,+10,+40,+120}_{640x360,1920x1080}.png` | `cp03_block_edge_decap`, L = 139 (the block lands on the goblin standing on the footprint edge): the block just before the edge cuts the neck, the contact tick, the head flying, the collapse, the pool growing, the settled remains. |
| `still_blast_p{4,1}_X{-10,+0,+3,+12,+40,+180}_{640x360,1920x1080}.png` | `qa_blast_breakup`, X = 60: the fizzling charge among three goblins, the burst tick, the burst, the pieces at peak, the settle, the end. |
| `sheet_decap_p4_vs_p1.png`, `sheet_blast_p4_vs_p1.png` | the same ticks side by side, preset 4 left, preset 1 right (crops of the 1920x1080 captures, half size). |
| `gif_decap_p4_960x540.gif`, `gif_blast_p4_960x540.gif` | every tick, preset 4, downscaled from the 1920x1080 frames: L-10 to L+120 and X-10 to X+180. |
| `perf_frame_times.txt` | frame-interval probe with `--death-stress` (crush, decapitation and explosion in thirds). |
| `SHA256SUMS` | checksums of every file here (this README excluded). |

## What the captures show

* **Decapitation (preset 4):** the block comes down on the goblin's face; on the landing tick the goblin is in the `decap_contact` pose with a jet from the neck and the head leaves on the same tick, tumbling away from the stone, swapping to `head_dead` on its first floor contact; the body buckles into `headless_collapse` and the pool grows. The cause-specific additions are in the code and unit tests: a squeeze of low drops from the neck on the landing tick and a splat at the block's edge on the walking line two ticks later. At preset 1 the same poses and silhouettes play with the gore layer hidden and no blood sprite anywhere.
* **Explosion (preset 4):** the fizzling charge is drawn under the goblin standing on it until tick 60; the goblin sprite is then hidden and its ten parts, ten gibs, two rags and 32 drops leave radially from the charge, the VFX-01 burst plays under the debris, wall spatters and a medium splat appear, and pools grow from the torsos' first rest. The goblin 16 px away is thrown intact with the heavy knockback blood; the third goblin, 48 px away, is untouched. At preset 1 the burst victim keeps its sprite and plays the simulated knockback and corpse frames; only two rags fly.
* **Carl and Donut stay readable:** both are on screen in every frame and nothing in the stills or sheets covers either of them (pieces thrown to the left land between the action and Carl, and one comes to rest just left of Donut, clear of both); the draw order is unit tested: the burst layer is `actor_z - 0.45`, under every actor, so a piece that did overlap them would draw under them.
* **Judge in the clips:** the decapitation pose was drawn for a horizontal blade and the block's edge is vertical (spec section 8: an `edge_contact` key is an art follow-up if it does not read); the burst art is small beside the ten pieces; the volume of gore and the throw speeds are first values.

## Frame-interval probe

`perf_frame_times.txt`, debug binary, **software rendering under xvfb (llvmpipe)** on a machine whose load average was 35 to 41 during the run (heavily shared; **the absolute numbers are not comparable with the DEATH-C probe** and carry no GPU or Steam Deck claim). `--death-stress` keeps every pool completely full (`parts=256/256`, `floor_decals=192/192`, `wall_decals=32/32`, recycled thousands of times) with explosions in the mix. Median / p95 / max frame interval over 600 frames after warm-up:

| Resolution | baseline (no death) | stress preset 4 | stress preset 1 |
| --- | --- | --- | --- |
| 1920x1080 | 190.6 / 246.4 / 331.2 ms | 269.1 / 421.9 / 918.6 ms | 215.0 / 300.9 / 417.1 ms |
| 640x360 | 47.5 / 72.1 / 99.6 ms | 57.8 / 83.2 / 135.7 ms | 53.4 / 77.4 / 97.3 ms |

Peak live pieces in the stress rows: 256 parts, 192 floor decals and 32 wall decals at preset 4 (the pools stay inside the DEATH-E1 budget), 256 parts and no decals at preset 1. For reference, the DEATH-C probe on a quieter machine (budgets 160 / 96 / 12, crush and decapitation only) measured 76.7 ms baseline against 78.0 ms at stress, preset 4, at 1920x1080 ([DEATH-C evidence](../death-c/README.md)). The stress rows here are slower than their baseline, but the machine was far busier than in that run (the baseline alone is 2.5 times slower), so this run **does not establish a cost of the explosion** and should be repeated on a quiet machine; what it shows is that the pools stay inside their caps with explosions in the mix.

## Rights/Provenance

Presentation code and evidence by Claude (medium tier, execution of the DEATH-D spec written by a top-tier agent); no new art was drawn. Every pixel in the captures is the committed GOB-01, GOB-02-DEATH, ENV-02, Carl, Donut, Floor 1 kit, `vfx_dynamite` (ANIM-CARL-03) and `vfx-01` art under the owner-reported `ART-PUBLICATION-ALL-V1` scope, unchanged here. Independent top-tier review of this integration: pending.
