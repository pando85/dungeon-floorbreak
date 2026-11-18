# ANIM-CARL-03 evidence (design phase)

Design-phase renders for [the ANIM-CARL-03 spec](../../02-art/carl-dynamite-anim-03-spec.md): the four frozen Carl keys, the frozen stick keys and the moving-variant rule. All images are Pillow models composed from the committed grids with the A1 lighting model of `scripts/env_kit_light.py` (through `scripts/pixelart/scene.py`); none is a Bevy capture. Donut is the approved 40 px idle, the goblin is GOB-01 goblin A (mirrored to face Carl), the blast is the unchanged VFX-01 `burst` peak (frame 5), drawn behind the actors with its centre 8 px above the walking line.

| File | Shows |
| --- | --- |
| `keys-scale.png`, `keys-scale-x4.png` | Lit Floor 1 at 1x (and a 4x copy): Donut, idle Carl, the keys L light, H hold (tell), W wind-up, R release, the stick in flight (f0, f3), the stick fizzling on the floor (z0), two goblins with the burst between them. |
| `keys-scale-unlit.png` | The same with the A1 pass off. |
| `silhouettes.png` | The keys as solid shapes at 1x and 4x next to Donut, idle Carl, the ANIM-CARL-02 haymaker wind-up (b0, for the "not a punch" check) and goblin A; prop pixels in red. |
| `keys-x4.png` | The four Carl keys and three stick keys at 4x in colour, pivot and floor marked. |
| `move-release-scale.png`, `-x4.png` | The release phase baked onto the six run frames (`dynamite_move_release`) on lit Floor 1, next to Donut and idle Carl. |
| `SHA256SUMS` | Checksums of the PNG files in this folder, verified by `python3 scripts/check_docs.py`. |

Regenerate: `bash scripts/export_assets.sh` once (the Floor 1 kit atlases in `generated/art` are needed by the scene renderer), then `python3 scripts/author_carl_dynamite.py --evidence`.
