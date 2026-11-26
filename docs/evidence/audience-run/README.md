# AUDIENCE-RUN evidence

Evidence of the AUDIENCE-RUN implementation PR: the per-hero viewer counts live inside the Floor 01 run, and the showcase slow motion of CINE-01 (a thrown-dynamite multi-kill, a llama kill). **Game feel is pending:** every number is the design's adopted starting value, not an owner-approved one. Design: [audience-run-design.md](../../01-design/audience-run-design.md) (AR-AD1 to AR-AD25, AR-T01 to AR-T26). Behaviour and flags: [floor01-scene.md](../../03-tech/floor01-scene.md#the-floor-run-f01-run-and-the-room-hud-f01-hud-room).

All commands run from the repository root with the debug build (`cargo build --bin floor01_scene`, then `bash scripts/export_assets.sh` once for the windowed runs). Window runs used `xvfb-run -a -s '-screen 0 1920x1200x24'`, `--no-audio --no-record`. SHA-256 sums of every file here are in [`SHA256SUMS`](SHA256SUMS).

## 1. Showcase moment lines (AR-T23 to AR-T25)

Headless observer lines (`--cine-showcase=on`, presentation OFF, so no `CINE01_SLOWMO` line). Each file holds the `CINE01_MOMENT` lines of one run; an empty file is a probe that must produce no moment. `scripts/ci_floor01_combat.sh cine` diffs all of them on every run.

```bash
floor01_scene --headless-floor01-smoke --party-fixture NAME --cine-showcase=on | grep '^CINE01_MOMENT'
floor01_scene --headless-floor01-smoke --fixture qa_blast_breakup --cine-showcase=on | grep '^CINE01_MOMENT'
```

| File | Result |
| --- | --- |
| `moments_carl_prog_dyn_thrown_triple.txt` | one ordinary moment, score 5, `thrown_multi=1`, on the blast tick 195 (AR-T23) |
| `moments_carl_prog_dyn_llama_pack.txt` | one **legendary** moment, score 8, `extra_kills=2 thrown_multi=1 showcase_bonus=1`: four goblins and a llama in one thrown blast (AR-T24) |
| `moments_carl_prog_dyn_llama_core.txt` | score 5, `showcase_kill=1`, target 0 (the llama's id): a llama killed point blank by the thrown stick, with no goblin around it (AR-T25) |
| `moments_cp05_llama_slam.txt` | score 5, `showcase_kill=1`: the windowed run below |
| `moments_carl_prog_dyn_core_kill.txt`, `moments_carl_prog_dyn_boss.txt`, `moments_qa_blast_breakup.txt` | empty: a single core kill, the boss fixture and the QA blast give **no** moment (AR-T25) |

The windowed llama slam, which also shows the slow motion (`cp05_llama_slam`, a heavy push kick of a clean Carl):

```bash
floor01_scene --no-audio --no-record --scale 1 --fixture cp05_llama_slam --cine-showcase=on --cinematic-finishes=on --exit-after-ticks 230
```

[`windowed_cp05_llama_slam_showcase_on.txt`](windowed_cp05_llama_slam_showcase_on.txt) has the `CINE01_MOMENT tick=163 ... showcase_kill=1` line, `CINE01_SLOWMO start` and `CINE01_SLOWMO end=Completed`. The real-time numbers of the end line (`real_ms`, `max_frame_ms`) vary from run to run; the CI compares the moment line only. [`windowed_cp05_llama_slam_showcase_off.txt`](windowed_cp05_llama_slam_showcase_off.txt) is the same run without `--cine-showcase`: no moment and no slow motion, and the **same final `COMBAT_STATE` hash** (`f5175c24c454138d`, the CINE observer never touches the simulation).

## 2. Gore preset 1 against 4

```bash
floor01_scene --headless-floor01-smoke --party-fixture carl_prog_dyn_llama_pack --audience-probe --cine-showcase=on --gore-preset 1 | grep -E '^(COMBAT_EVENT|CINE01_MOMENT|AUDIENCE_STATE)'
... --gore-preset 4 ...
```

[`gore_parity_preset1.txt`](gore_parity_preset1.txt) and [`gore_parity_preset4.txt`](gore_parity_preset4.txt) are byte-identical (`cmp` passes): the audience award and the death finish read the same authoritative pair independently (design section 5). The unit test `audience_digest_ignores_the_gore_preset` (AR-T17) checks the digest, the events and the simulation hash. The probe unlocks the audience at the fixture level; `--audience=on` stays tied to `--f01-run`.

## 3. Run reveal stills (640x360 at scale 1 and 3)

| File | Command (plus `--capture-dir`) | Shows |
| --- | --- | --- |
| `run-corral-clear-panel-640x360-x1.png`, `...-x3.png` | `--scene f01_late --f01-run --room-pilot corral --audience=on --capture-sequence 960:40:2` (frame 1, the exit tick 971) | the corral's room-clear panel with the one new line, `broadcast: live from the next room (viewers per hero)`, before `next:`; no number on the panel and no plate on the HUD (the corral is not live) |
| `run-hoarder-first-frame-640x360-x1.png`, `...-x3.png` | `--scene f01_late --f01-run --f01-room hoarder --audience=on --capture-sequence 60:30:2` (frame 0) | the room after the corral: both plates show 250 (the baseline) |
| `run-boiler-carried-counts-640x360-x1.png`, `...-x3.png` | `--scene f01_late --f01-run --f01-room boiler --room-pilot direct --audience=on --f01-audience 1400:1900:600:800 --capture-sequence 60:240:2` (frame 0) | a later room entered with a carry: Carl 1.4K, Donut 600 |
| `optout-hoarder-first-frame-640x360-x1.png` | `--scene f01_late --f01-run --f01-room hoarder --audience=off --cine-showcase=off --capture-sequence 60:30:2` | the opt-out (item 6): no plates |

A capture is a scripted path, so the run default is off there: these captures pass `--audience=on` explicitly (the room pilots and captures never switch the behaviour on by themselves, test `broadcast_defaults_are_on_only_for_a_live_f01_run`). The count of a live room with no explicit flag is the same code path.

## 4. A finish with the viewers (GIF)

No room pilot throws the dynamite stick, so this is the **fixture** capture, not a run capture (nothing here was faked as a run capture):

```bash
floor01_scene --no-audio --no-record --scale 1 --party-fixture carl_prog_dyn_llama_pack --audience-probe --cine-showcase=on --cinematic-finishes=on --capture-sequence 188:2:24
python3 scripts/room_gif.py FRAMES llama_pack_showcase_640x360.gif --step-ticks 2
```

[`llama_pack_showcase_640x360.gif`](llama_pack_showcase_640x360.gif): ticks 188 to 234, every second tick, 640x360. The stick lands among four goblins and a llama and detonates at tick 195 (the exploded finishes), and Carl's plate counts up. The legendary moment of this run is the `moments_carl_prog_dyn_llama_pack.txt` line. A capture samples ticks, so it cannot show the real-time slow-motion curve; that is what the windowed `cp05_llama_slam` run above records (`CINE01_SLOWMO`, 14 ticks over about 485 ms).

## 5. Restart and Respawn

[`restart_respawn_log.txt`](restart_respawn_log.txt): a run room resumed from the carry `1400:1900:600:800` through the real `new_hud`, then a defeat, a repeat of the same target id, a checkpoint `Respawn`, the same target again, a `Restart`, and the target once more. A checkpoint respawn keeps the counts and the per-attempt dedupe; a `Restart` restores the counts the room began with (1400, 600) and the carried peak; a restart never farms more than one award per target.

```bash
cargo test --bin floor01_scene the_carried_counts -- --nocapture | grep '^AUDIENCE_RESTART_LOG'
```

## 6. The opt-out

`--audience=off --cine-showcase=off` in the run gives no plates (`optout-hoarder-first-frame-640x360-x1.png`) and CINE-01 exactly as before: the legacy `cine` probes (the B3 moment at tick 159 and the pile-up moment at tick 139) and the committed `COMBAT_EVENT` and `COMBAT_STATE` lines are byte-identical on this head (`ci_floor01_combat.sh headless` and `cine`), the `--audience=off` and `--f01-room hoarder` owner commands launch (`scripts/check_f01_owner_commands.sh`), and the unit tests `broadcast_defaults_are_on_only_for_a_live_f01_run` and `the_run_unlocks_the_audience_after_the_corral_at_the_carried_counts` cover the parse rules and the room beat.
