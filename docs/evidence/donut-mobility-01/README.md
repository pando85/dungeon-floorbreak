# DONUT-MOBILITY-01 DM-A evidence

Real Bevy renders (`floor01_scene`, 640x360, 1x, lighting on) of the two side-by-side fixtures of
`src/agility_fixtures.rs`, cropped to the lane (640x230). They show what the motion looks like;
they are not a game-feel verdict and are not reproduced bit for bit by CI (driver dependent).
The same fixtures are pinned headlessly by `agility_fixtures::tests`.

| GIF | Fixture | What it shows |
| --- | --- | --- |
| [`dm_a_run_race_1x.gif`](dm_a_run_race_1x.gif) | `dm_run_race_2p`, ticks 4..124 | Both heroes start in Carl's column and run right for 120 ticks: Donut (207 px/s) ends 44 px ahead of Carl (185 px/s) |
| [`dm_a_jump_race_1x.gif`](dm_a_jump_race_1x.gif) | `dm_jump_race_2p`, ticks 6..170 | Donut starts 60 px behind Carl; a standing jump and then a running jump: her apex is 114.92 px against his 88.54, her flight is 57 ticks against 50 |

Recipe (from the repository root, after `bash scripts/export_assets.sh` and a debug build):

```sh
xvfb-run -a -s '-screen 0 1920x1200x24' ./target/debug/floor01_scene --scale 1 \
  --party-fixture dm_run_race_2p --capture-sequence 4:2:61 --capture-dir <scratch>/race
xvfb-run -a -s '-screen 0 1920x1200x24' ./target/debug/floor01_scene --scale 1 \
  --party-fixture dm_jump_race_2p --capture-sequence 6:2:82 --capture-dir <scratch>/jump
magick -delay 3 -loop 0 <scratch>/race/frame_*.png -crop 640x230+0+100 +repage -colors 96 \
  -fuzz 3% -layers OptimizeTransparency dm_a_run_race_1x.gif
```

The jump GIF uses the same `magick` line on its frames. Checksums: `SHA256SUMS`.

GIFs 3 and 4 of the work order (the reflex, the microarena perch jump with the HUD drawn) belong to
DM-B and DM-C.
