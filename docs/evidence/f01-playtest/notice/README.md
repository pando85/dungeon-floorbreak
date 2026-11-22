# CP-B2-FU-NOTICE: a fresh capture of the dynamite grant notice

Window captures (xvfb, debug build, no audio) of the placeholder-zone fixture `carl_prog_story_event_grants_dynamite` (`--party-fixture`, the dynamite grant is tick 69), the frame on tick 75, with the System notice `GOBLIN BOOM` in at `--scale 1` (640x360) and `--scale 2` (1280x720). They are the owner's input for the readability verdict (plan row T40, backlog `CP-B2-FU-NOTICE`); an agent's reading is not a verdict.

Command (once per scale, after `bash scripts/export_assets.sh`):

```
xvfb-run -a -s '-screen 0 1280x720x24' ./target/debug/floor01_scene --no-audio --no-record --scale 2 \
  --party-fixture carl_prog_story_event_grants_dynamite --capture-sequence 66:3:4 --capture-dir OUT
```

Known gap: the note asks for the darkest Floor 01 scene. The only grant that raises this notice in a launchable scene today is this fixture's lit corridor (warm torch-lit brick), so the notice is shown over a bright background here, not over a near-black one. The grant fires nowhere in the dark Boiler room, so a darkest-scene capture is **NOT RUN** until a room grants there.
