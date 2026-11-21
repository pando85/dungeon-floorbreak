# ENEMY-GAMEPLAY-ENGINEER evidence

The Goblin Engineer as a playable enemy, behind `cf02_rules` and `--cf02-engineer`. Design: [enemy-gameplay-engineer.md](../../01-design/enemy-gameplay-engineer.md) (section 19 lists what implementation found). **Game feel is not accepted; nobody has played this.** Every value is data in `assets/data/gameplay/enemy_engineer_v0.json`.

## Play it

```sh
cargo run --bin floor01_scene -- --cf02 --cf02-engineer --scale 2
```

The microarena of `--cf02` with the yard goblin at x 940 replaced by the Engineer (he guards the llama yard between the parapet, the right perch and goblin 880). `--demo-beat 1` starts from the second checkpoint. With `engq25_enabled` false in the data file the same command runs plain `cf02_arena`.

## Real Bevy 1x GIFs (640x360, no scaling)

Made with `scripts/make_cf02_i_evidence.sh` (real render loop, lighting on, `--scale 1`, one frame every 2nd tick, full 640x360 view). The `_states.txt` file holds the `COMBAT_ANIM_STATE` line of every frame (`g0=E/...` is the ENG-01 frame the animators chose); the strip shows the same frames side by side. In both clips Donut is the non-live follower standing next to Carl (the `eng_*` fixtures are Carl only).

| Clip | Fixture and ticks | What it shows |
| --- | --- | --- |
| [`eng_point_ram_jump.gif`](eng_point_ram_jump.gif) | `eng_jump_over_ram`, 4..78 | The rear (frame 9), the point (frame 10, 24 ticks), the committed launch (11), the ram (12, held) passing under Carl's jump (jump pressed on attack tick 20), the recover (13), Carl's jab on the recover (the hurt frame 14) |
| [`eng_wall_bonk.gif`](eng_wall_bonk.gif) | `eng_parapet_bonk`, 4..78 | The same attack against Carl standing flush with the parapet's face: the pot meets the face (`RamBonk` on active tick 16, scene tick 51), the hurt frame, then the dazed recover frame |

## Checks run on this head

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets` | clean; 1694 lib tests, every legacy golden and `cf02_*` fixture unchanged |
| `bash scripts/ci_floor01_combat.sh headless` | exit 0; the 29 new replays (`docs/evidence/cf02/replays/headless_eng_*.txt`, `headless_cf02_arena_eng*.txt`) replay twice identically and equal the committed files |
| `python3 scripts/check_docs.py` | passed |
| Simulation cost (`engineer_room_sim_cost`, 1800 ticks, best of 5, debug-optimised build, this machine) | `cf02_arena_direct` 1.01 us per tick, `cf02_arena_eng` 1.03 us per tick |

## Not done

* The A/B report of design section 11 item 8 (routes, seeds, AI swat cancel and AI Donut ram counts) and the other clips of item 7 (snap, bowl, perch dive, 2P point, tell next to goblin A). The fixtures that reproduce each exist and are tested; only the GIF rendering and the A/B comparison are left.
* No Engineer column in the PLAYTRACE observation rows (it would change the `autoplay-episode/1` shape); a recorded Engineer session still replays, because the replay resolves `cf02_arena_eng*` by its name.

Rights and provenance: the images show only the committed ENG-01, Carl, Donut and environment art (no new art). Model attribution: Claude Sonnet (execution and integration tier); design by the top tier in #162.
