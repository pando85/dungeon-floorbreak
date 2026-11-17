# COMBAT-INT-01 evidence

Real Bevy captures and deterministic logs of the Floor 1 combat fixtures. **Mechanical probes only**: the goblin is a disclosed placeholder rectangle (magenta band on its head) and Carl's strike is a small pale box, because the GOB-01 and ANIM-CARL-02 art have not merged. Nothing here is a feel verdict or an art approval; hit-stop and knockback tuning come in the next PR.

| File | What it shows |
| --- | --- |
| `cp01_single_goblin_hit_tick125_1920x1080.png` | the jab's first active tick (press on 122, startup 3): the goblin flashes pale, the strike box is at the goblin's near edge |
| `cp01_single_goblin_stagger_tick129_1920x1080.png` | the staggered goblin after the 4 px push (stagger 10 ticks, no chase) |
| `cp01_single_goblin_defeated_tick142_1920x1080.png` | the second jab (hit on 139) defeated it: a flat slab, no longer a body |
| `cp01_single_goblin_ticks118-178_960x540.gif` | ticks 118..178, one frame per tick, from the same capture at half size |
| `headless_<fixture>.txt` | the headless replay: every `COMBAT_EVENT` and the final `COMBAT_STATE` hash line of `cp01_single_goblin`, `cp01_two_hits`, `cp02_two_goblins` |
| `SHA256SUMS` | checksums of the files above |

Stills and GIF are screenshots of the real Bevy window (lighting on, Xvfb with software Vulkan on the author's machine), 1920x1080 = 640x360 x 3. They are not reproduced bit for bit by CI (rendering varies by driver); the event logs and hashes are: CI diffs the headless replays against these `headless_*.txt` files and requires the real render loop to print the same events and final state.

Reproduce (from the repository root, after `bash scripts/export_assets.sh`):

```sh
cargo build --locked --bin floor01_scene
bash scripts/ci_floor01_combat.sh headless
xvfb-run -a -s '-screen 0 1920x1080x24' bash scripts/ci_floor01_combat.sh render
bash scripts/make_combat_int_evidence.sh
sha256sum -c docs/evidence/combat-int-01/SHA256SUMS   # from inside that directory
```

Final hashes: `cp01_single_goblin` `c6952df1e99c7c0a`, `cp01_two_hits` `382bcaedf5dde492`, `cp02_two_goblins` `f4e11cc71da438a1`. The story run is unchanged: `--scenario walk_right --capture-tick 250` still ends in `FLOOR1_SCENE_STATE ... hash=931868d6afa56498`.

Provenance: code by Claude Sonnet; no new art; no new third-party assets.
