# COMPANION-AI-01 01A-wire and 01B-wire evidence

Committed replays of the `companion_*` and `cf02_assist_*` fixtures (`src/companion_fixtures.rs`): `headless_<fixture>.txt` holds the `COMBAT_EVENT`, `COMBAT_STATE` and `PARTY_STATE` lines of a headless replay. `bash scripts/ci_floor01_combat.sh headless` replays every name printed by `floor01_scene --list-fixtures companion_` and `--list-fixtures cf02_assist_` twice, compares the two runs with `cmp` and diffs them against these files; the final `PARTY_STATE` hash of each is also pinned by `GOLDENS` in `src/companion_fixtures.rs`. The same script replays `cf02_assist_camera_parity` under `--camera auto` and `--camera scroll` and requires the plain lines (CF02-Q40).

## Reproduce

```sh
cargo build --locked --bin floor01_scene
for f in $(./target/debug/floor01_scene --list-fixtures companion_) $(./target/debug/floor01_scene --list-fixtures cf02_assist_); do
  ./target/debug/floor01_scene --headless-floor01-smoke --party-fixture "$f" | grep -E '^(COMBAT_EVENT|COMBAT_STATE|PARTY_STATE)' > "docs/evidence/companion-ai/headless_$f.txt"
done
cargo test --locked --lib metrics_bot -- --nocapture   # the table below
```

## Metrics: follow-and-jump bot against the companion on the same fixtures

Attribution follows AP-Q14 (the hero-index `instigator`, `Hit` `move_id` for a hit) and who the companion drove on the tick of the event. "Assist starts C/D" are AI-started moves for Carl and Donut (a chain follow-up is not one, CA-36). "Glances" are AI hits whose damage the kill policy turned into 0 (`ca19_kill_policy = never`, CA-20). "AI indirect" is `ai_indirect_kills`. Everything here is a headless, scripted measurement: game feel is NOT RUN until the owner plays.

| fixture | mode | assist starts C/D | AI hits | human hits | glances | AI kills | AI indirect | human kills | AI interrupts |
|---|---|---|---|---|---|---|---|---|---|
| cf02_assist_swat_interrupt | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_swat_interrupt | companion | 0/2 | 2 | 0 | 0 | 1 | 0 | 0 | 0 |
| cf02_assist_swat_trailing_gap | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_swat_trailing_gap | companion | 0/3 | 2 | 0 | 0 | 1 | 0 | 0 | 1 |
| cf02_assist_pounce_staggered | bot | 0/0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_pounce_staggered | companion | 0/1 | 1 | 3 | 0 | 0 | 0 | 1 | 0 |
| cf02_assist_hp_drops_midmove | bot | 0/0 | 0 | 3 | 0 | 0 | 0 | 1 | 0 |
| cf02_assist_hp_drops_midmove | companion | 0/1 | 1 | 2 | 0 | 0 | 0 | 1 | 0 |
| cf02_assist_cadence | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_cadence | companion | 0/5 | 4 | 0 | 0 | 2 | 0 | 0 | 3 |
| cf02_assist_magic_locked | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_magic_locked | companion | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_missile_reserve | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_missile_reserve | companion | 0/3 | 3 | 0 | 0 | 1 | 0 | 0 | 3 |
| cf02_assist_camera_parity | bot | 0/0 | 0 | 2 | 0 | 0 | 0 | 1 | 0 |
| cf02_assist_camera_parity | companion | 0/5 | 4 | 0 | 0 | 2 | 0 | 0 | 1 |
| cf02_assist_donut_defeats_goblin | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_donut_defeats_goblin | companion | 0/2 | 2 | 0 | 0 | 1 | 0 | 0 | 0 |
| cf02_assist_donut_missiles_goblin | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_donut_missiles_goblin | companion | 0/2 | 2 | 0 | 0 | 1 | 0 | 0 | 0 |
| cf02_assist_donut_guards_carls_back | bot | 0/0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_donut_guards_carls_back | companion | 0/5 | 5 | 3 | 0 | 2 | 0 | 0 | 3 |
| cf02_assist_donut_flanks_two_goblins | bot | 0/0 | 0 | 2 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_donut_flanks_two_goblins | companion | 0/2 | 2 | 2 | 0 | 0 | 0 | 2 | 0 |
| companion_carl_interrupts_for_donut | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| companion_carl_interrupts_for_donut | companion | 2/0 | 1 | 0 | 1 | 0 | 0 | 0 | 2 |
| companion_donut_supports_carl | bot | 0/0 | 0 | 2 | 0 | 0 | 0 | 1 | 0 |
| companion_donut_supports_carl | companion | 0/2 | 0 | 2 | 0 | 0 | 0 | 1 | 2 |
| companion_duo_no_ai | bot | 0/0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |
| companion_duo_no_ai | companion | 0/0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |
| companion_swap_mid_move | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| companion_swap_mid_move | companion | 2/1 | 1 | 0 | 1 | 0 | 0 | 0 | 2 |
| companion_arena_nav_perch_donut | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| companion_arena_nav_perch_donut | companion | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| companion_arena_nav_perch_carl | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| companion_arena_nav_perch_carl | companion | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

Reading: `companion_duo_no_ai` shows the 2P case (the companion never runs, bot and companion are the same replay). Since CF02-DUO (owner playtest feedback 2026-11-21) the `cf02_assist_*` rows run the `cf02_assist` set with `ca19_kill_policy = factual`: AI Donut engages goblins and may finish one that Carl is not engaging (the two `cf02_assist_donut_*` fixtures have no human input at all and AI Donut defeats the goblin; where Carl fights, `cf02_assist_pounce_staggered` and `cf02_assist_camera_parity`, she holds back and the kill is Carl's, CF02-DUO-A08 / spec C-11; the offensive swat was removed by A07, so her swats are interrupts of a tell; since the A15 flank she meets the goblins with a pounce first, so each of `cf02_assist_cadence` and `cf02_assist_camera_parity` has 1 swat interrupt, was 2). The `companion_*` rows (the `companion_v0` set, unchanged) keep `never`: no AI hit lands the killing blow there (`AI kills` is 0) and the AI hit share stays at most 25 % in the two goblin fights (CF02-Q39, CA-32). The `cf02_assist` hit-share ceiling is 50 % (CF02-DUO-A03) and is measured on `cf02_arena_direct`, not on these idle-Carl fixtures. Hit counts are damaging hits; the swat deals 0. `cf02_assist_donut_guards_carls_back` (CF02-DUO round 2, A11 to A14) is the scene where Carl works a goblin ahead and a second one walks up behind him: she starts 5 pounces, lands 5 hits and defeats 2 goblins (the first is the one at Carl's back), with no last hit taken from the goblin Carl is hitting (`steals` 0 and `start_steals` 0, see the spec for both counts). `cf02_assist_donut_flanks_two_goblins` (round 3, A15) is the scene where Carl walks into two chasing goblins: she meets them on a flank spot 56 px ahead of him, lands the first hit on both (2 starts, 2 hits) and Carl finishes both (0 kills of hers, 0 steals); its 1x GIF is [`../cf02/cf02-duo2/donut_flank_pounce_1x.gif`](../cf02/cf02-duo2/donut_flank_pounce_1x.gif). Tests pinning the scenes: `c11_a_goblin_carl_hit_stays_his_across_a_walk_up`, `c11_b1_a_goblin_carl_hit_stays_his_when_he_steps_back_or_walks_through`, `c11_known_limit_a_same_tick_press_still_lets_her_take_the_last_hit`, `duo_a13_ai_donut_defeats_the_goblin_that_comes_at_carl_from_behind`, `duo_the_arena_run_measures_her_offence_and_no_steal` and `duo_a15_she_meets_the_approaching_goblins_and_carl_finishes_them`.

## 01B-wire: nav-driven follow (`01b-wire/`)

`companion_arena_nav_perch_donut` and `companion_arena_nav_perch_carl` run in the CF02 microarena (the `companion_arena_*` prefix selects the arena kit, `cf02::arena::fixture_uses_arena`) with no enemies: the heroes only move. The controlled hero jumps onto the left perch slab (top 80 px, underside 68 px, columns 544..672); the companion starts on the floor and must get up by the nav graph.

| fixture | controlled | companion | nav events | final | hash |
|---|---|---|---|---|---|
| companion_arena_nav_perch_donut | Carl | AI Donut | Begin(Jump), Arrived, tick 41 to 74 | on the slab, feet 80 px up | `5df620202ed1c92c` |
| companion_arena_nav_perch_carl | Donut | AI Carl | Begin(Jump), Arrived, tick 44 to 85 | on the slab, feet 80 px up | `69c986158efcd9d1` |

Under the follow-and-jump bot (the rollback) the AI Donut walks under the slab and bumps its head (feet 2.9 px up at the end); the bot climbs for AI Carl. Test: `cargo test --locked --lib the_rollback_bot_is_unchanged_by_the_nav -- --nocapture`.

Images (real Bevy renders at 640x360, 1x; the first, unlit warm-up frame is dropped; evidence of what was drawn, not a game-feel verdict, not reproduced bit for bit by CI): `nav_perch_donut_1x.gif`, `nav_perch_carl_1x.gif`, and `nav_perch_*_contact.png` (five frames of each: the run-up, the takeoff, the climb, the landing, the finish). The `state_nav_perch_*.txt` files hold the last `COMBAT_STATE` line.

```sh
xvfb-run -a -s '-screen 0 1920x1200x24' ./target/debug/floor01_scene --scale 1 --party-fixture companion_arena_nav_perch_donut --capture-sequence 30:2:36 --capture-dir /tmp/nav_donut
magick -delay 3 -loop 0 /tmp/nav_donut/frame_*.png -layers Optimize nav_perch_donut_1x.gif   # without frame_0000
```
