# ANIM-CARL-03 evidence

Renders for [the ANIM-CARL-03 spec](../../02-art/carl-dynamite-anim-03-spec.md): the frozen keys, the drawn in-betweens, the moving variant and the full throw in the lit Floor 1 corridor. All images are Pillow models composed from the committed grids with the A1 lighting model of `scripts/env_kit_light.py` (through `scripts/pixelart/scene.py`); none is a Bevy capture. Donut is the approved 40 px idle, the goblin is GOB-01 goblin A (mirrored to face Carl), the blast is the unchanged VFX-01 `burst` (centre 8 px above the walking line, drawn behind the actors). Numbers in the scene GIFs (lob speed, gravity, blast 90 ticks after the fuse is lit) are the spec section 9 placeholders, for evidence only.

**Run `bash scripts/export_assets.sh` first** (the Floor 1 kit atlases in `generated/art` are needed by the scene renderer), then:

```
python3 scripts/author_carl_dynamite.py --evidence
python3 -m scripts.pixelart evidence assets/source/characters/carl/carl_h52_dynamite.pixelart.json --out docs/evidence/anim-carl-03/carl
python3 -m scripts.pixelart evidence assets/source/vfx/dynamite/vfx_dynamite.pixelart.json --out docs/evidence/anim-carl-03/vfx
```

| File | Shows |
| --- | --- |
| `scene-throw-lit-640x360.png`, `scene-throw-lit-1920x1080.png` | Lit Floor 1 at 1x (and 3x): Donut, Carl at the release key, the stick in flight toward two goblins. |
| `scene-phases-lit-640x360.png`, `scene-phases-lit-1920x1080.png` | All nine standing frames in a row (draw, light, hold, hold flicker, wind-up, release, follow-through, flinch, settle) with Donut and two goblins, for the 1x readability check. |
| `throw-standing-scene.gif`, `throw-moving-scene.gif` | The whole throw on lit Floor 1 at 20 fps (a frame every 3 ticks): standing and running (the run frames with the baked arms), the stick flying, landing and fizzling, then the VFX-01 burst. Tick, phase and the placeholder numbers are captioned. |
| `timing_diagram.png` | The nine standing frames at 3x with pivot, stick centre, spark core, planted or raised feet, and a to-scale tick bar (fuse lit t6, spawn t41). |
| `keys-scale.png`, `keys-scale-x4.png`, `keys-scale-unlit.png` | Lit Floor 1 at 1x (and a 4x copy, and with the A1 pass off): Donut, idle Carl, the four keys, the stick in flight and fizzling, goblins and the burst. |
| `silhouettes.png` | The keys as solid shapes at 1x and 4x next to Donut, idle Carl, the ANIM-CARL-02 haymaker wind-up (b0) and goblin A; prop pixels in red. |
| `keys-x4.png` | The four Carl keys and three stick keys at 4x in colour, pivot and floor marked. |
| `move-{light,hold,wind,release}-scale.png`, `-x4.png` | Each overlay phase baked onto the six run frames on lit Floor 1. |
| `carl/` | Toolkit evidence for the Carl atlas: contact sheets (x1, x4: all 45 frames), tell-to-attack strip, per-tag GIFs at tick timing (x1, x3), scenes with the standing frames and two goblins; its own `SHA256SUMS`. |
| `vfx/` | The same for the stick atlas (flight, fizzle). |
| `SHA256SUMS` | Checksums of the PNG and GIF files in this folder, verified by `python3 scripts/check_docs.py`. |
