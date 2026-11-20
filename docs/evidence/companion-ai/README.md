# COMPANION-AI-01 01A-wire evidence

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
| cf02_assist_swat_interrupt | companion | 0/1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| cf02_assist_swat_trailing_gap | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_swat_trailing_gap | companion | 0/1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| cf02_assist_pounce_staggered | bot | 0/0 | 0 | 3 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_pounce_staggered | companion | 0/1 | 1 | 3 | 0 | 0 | 0 | 1 | 0 |
| cf02_assist_hp_drops_midmove | bot | 0/0 | 0 | 3 | 0 | 0 | 0 | 1 | 0 |
| cf02_assist_hp_drops_midmove | companion | 0/1 | 0 | 3 | 1 | 0 | 0 | 1 | 0 |
| cf02_assist_cadence | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_cadence | companion | 0/2 | 0 | 0 | 0 | 0 | 0 | 0 | 2 |
| cf02_assist_magic_locked | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_magic_locked | companion | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_missile_reserve | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| cf02_assist_missile_reserve | companion | 0/6 | 2 | 0 | 4 | 0 | 0 | 0 | 6 |
| cf02_assist_camera_parity | bot | 0/0 | 0 | 2 | 0 | 0 | 0 | 1 | 0 |
| cf02_assist_camera_parity | companion | 0/2 | 0 | 2 | 0 | 0 | 0 | 1 | 2 |
| companion_carl_interrupts_for_donut | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| companion_carl_interrupts_for_donut | companion | 2/0 | 1 | 0 | 1 | 0 | 0 | 0 | 2 |
| companion_donut_supports_carl | bot | 0/0 | 0 | 2 | 0 | 0 | 0 | 1 | 0 |
| companion_donut_supports_carl | companion | 0/2 | 0 | 2 | 0 | 0 | 0 | 1 | 2 |
| companion_duo_no_ai | bot | 0/0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |
| companion_duo_no_ai | companion | 0/0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 |
| companion_swap_mid_move | bot | 0/0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| companion_swap_mid_move | companion | 2/1 | 1 | 0 | 1 | 0 | 0 | 0 | 2 |

Reading: `companion_duo_no_ai` shows the 2P case (the companion never runs, bot and companion are the same replay); every fixture with a companion has at most 25 % AI hit share in the two goblin fights (CF02-Q39, CA-32); no AI hit ever lands the killing blow (`AI kills` is 0 everywhere).
