# ANIM-DONUT-02 evidence: Donut melee and damage reactions

Review aids for the [spec](../../02-art/donut-melee-anim-02-spec.md) (file table in its section 14). Pillow compositions, not Bevy captures; not owner-approved.

Regenerate (the Floor 1 kit atlases must exist first):

```sh
bash scripts/export_assets.sh                      # required before any of the steps below, including review-pack
python3 scripts/author_donut_melee.py --evidence   # keys-scale*.png, keys_x6.png, p0_vs_jump_start_*
python3 scripts/make_donut_melee_previews.py       # everything else, measurements.json and SHA256SUMS
python3 -m scripts.pixelart review-pack assets/source/characters/donut/donut_h40_melee.pixelart.json
```

`review-pack` and the lit scenes read the exported atlases under `generated/art/`, so they fail or go stale unless `scripts/export_assets.sh` has run. `SHA256SUMS` lists every file here; `python3 scripts/check_docs.py` verifies it.
