# PTR-R1 evidence: the replay viewer

Real Bevy captures of `floor01_scene --replay-session` (Xvfb, software rendering, 3x scale = 1920x1080). Screenshots and one GIF of the rendered replay only: no session data, no manifests, no paths. The overlay (trust banner, tick, episode, who drives each hero, "presentation settings not recorded: defaults") is part of the viewer.

| File | What it shows |
| --- | --- |
| `owner_checked_tick405.png`, `owner_checked_tick585.png`, `owner_checked_tick986.png` | An owner-recorded `--cf02` session (human P1 Carl, AI companion Donut) replayed on the **sim-pinned build** (`0bffd264` + the R0/R1 commits) in checked mode. Tick 405 is a hit that interrupts a goblin, tick 585 the `BodyContact` event (the goblin body launch notice), tick 986 Donut's glance event. The orange banner reads `CHECKED CROSS-BUILD (NOT ORIGINAL-BUILD VERIFIED)`. |
| `owner_checked_t540_t730.gif` | The same session at 1x, ticks about 540 to 730 (a 20 fps screen grab of the window, 640 px wide, so a screen recording, not a frame export). |
| `pause_and_step_overlay.png` | Overlay strips at five moments of one run, driven with `xdotool`: PLAYING (tick 328), PAUSED (346, unchanged a second later), three `.` presses (349), Space again (PLAYING, 405). |
| `synthetic_strict_tick300.png` | The synthetic CF02 autopilot session of `ci_floor01_combat.sh replay` at tick 300, STRICT (`REPLAY STRICT (same source as the recording)`). |

## Commands (no local paths)

Strict precondition, on main (`b3ec13dc`, same source fingerprint as `0bffd264`):

    cargo run --locked --bin playtrace -- verify ID        # for each owner session

All eight owner sessions: `result=PASS scope=full build=source_identical_commit_differs` (8739, 1291, 1468, 307, 1171, 1216, 1894 and 1435 ticks).

Sim-pinned replay build (local, never pushed; PTR-Q16). The evidence below ran on `0bffd264` plus the R0/R1 commits, which is source-equivalent to the merged build. After the squash merge use the #167 squash commit itself (the cherry-pick onto `0bffd264` stops on the two new docs):

    git worktree add --detach <tmp> 12e43398
    cp -r generated <tmp>/generated        # exported art from the main checkout (gitignored)
    cd <tmp> && cargo run --locked --release --bin floor01_scene -- --replay-session ID --replay-compatibility checked --scale 3
    # or: scripts/replay_session_pinned.sh 12e43398 ID

(the evidence ran the dev profile of the same tree). Result lines, one per owner session, all with exit code 0:

    PLAYTRACE_REPLAY session=0487b469… result=CHECKED ticks=8739 post=adbd48b05b6e07f7
    PLAYTRACE_REPLAY session=1e48c1ed… result=CHECKED ticks=1291 post=147ec730f165209b
    PLAYTRACE_REPLAY session=2d6f7656… result=CHECKED ticks=1468 post=637fbe0fba785169
    PLAYTRACE_REPLAY session=787aa359… result=CHECKED ticks=307 post=9eb8b999394357ad
    PLAYTRACE_REPLAY session=8e8a1387… result=CHECKED ticks=1171 post=ab5180342765d9dc
    PLAYTRACE_REPLAY session=a447465c… result=CHECKED ticks=1216 post=66cc589c4dd37c27
    PLAYTRACE_REPLAY session=ab3d866b… result=CHECKED ticks=1894 post=9e7bea78e7b0b8de
    PLAYTRACE_REPLAY session=faa8f4a3… result=CHECKED ticks=1435 post=2958a3b70e83bf3d

(`ticks` is the number of ticks matched against the recording and `post` the post-state hash of the last one; the full session ids are abbreviated here.)

Stills:

    floor01_scene --replay-session ID --replay-compatibility checked --replay-exit-at-end --no-audio --scale 3 \
      --replay-capture-tick 585 --replay-capture-file owner_checked_tick585.png

## What this is not

Checked mode proves the simulation equals the recording tick by tick on a build that is not the recording build. It is **not** `playtrace verify` and not original-build verification. Game feel of the replay (animation timing around restarts, particles, audio) is still the owner's review. The captures are fan-concept material under `ART-PUBLICATION-ALL-V1` and no licence to distribute a build or a video.
