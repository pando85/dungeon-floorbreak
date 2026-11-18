# ANIM-DONUT-03: evidence for the Donut Magic Missile cast and VFX

Pillow models composed from the committed grids and the A1 lighting model; none is a Bevy capture. Spec and file table: [`docs/02-art/donut-magic-missile-anim-03-spec.md`](../../02-art/donut-magic-missile-anim-03-spec.md) section 10. Integrity: `SHA256SUMS` in this folder (and in `cast/` and `vfx/`), verified by `python3 scripts/check_docs.py`.

**Prerequisite:** every scene here, `python3 -m scripts.pixelart evidence` and `python3 -m scripts.pixelart review-pack` need the Floor 1 kit atlases under `generated/art`, which are git-ignored. Run `bash scripts/export_assets.sh` once in a fresh checkout (it also exports the two Magic Missile atlases), otherwise the tools stop with "the Floor 1 kit atlases are missing". Then:

```
python3 scripts/author_donut_missile.py keys
python3 scripts/author_donut_missile.py evidence
python3 -m scripts.pixelart evidence assets/source/characters/donut/donut_h40_missile.pixelart.json --out docs/evidence/anim-donut-03/cast
python3 -m scripts.pixelart evidence assets/source/vfx/magic_missile/vfx_magic_missile.pixelart.json --out docs/evidence/anim-donut-03/vfx
```

Afterwards rewrite this folder's `SHA256SUMS` (`sha256sum *.png *.gif`).
