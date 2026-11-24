# HOARD-02 execution evidence

Art evidence for the Hoarder production set (`docs/02-art/hoarder-hoard-02.md`), for the owner's final review. Regenerate with `python3 -m scripts.pixelart evidence assets/source/characters/hoarder/hoard-02.pixelart.json --out docs/evidence/hoard-02`, `python3 scripts/author_hoard02.py --sheet docs/evidence/hoard-02/keys-x3.png` and `python3 scripts/make_hoard02_evidence.py docs/evidence/hoard-02` (it needs `bash scripts/export_assets.sh`). `SHA256SUMS` lists every file.

| File | What it shows |
| --- | --- |
| `contact_sheet_x1.png`, `contact_sheet_x4.png` | all 29 frames, 1x and x4 |
| `keys-x3.png`, `keys-x3-1x.png` | the 13 frozen keys' sheet (x3 and 1x), HOARD-01 keys 0, 7, 8, 15 next to the new ones |
| `gif_*_x1.gif`, `gif_*_x3.gif` | every tag at gameplay scale (x1) and x3: `attack_tell`/`attack_swing` (swipe), `shove_tell`, `lunge`, `jammed`, `pinned`, `get_up`, `emerge`, plus idle, walk, hurt, down |
| `tell_attack_strip.png`, `scene_*.png`, `review-pack/` | the toolkit's tell/attack strip, lit scene and the three-image review pack with the validator report |
| `props_contact_x1.png`, `props_contact_x4.png` | every state of every room kit piece |
| `frame_layout_diagram.png` | the atlas frame layout, pivots, boxes, authored ticks, the sim mode to frame table, and each prop's frames with pivot and collision rectangle |
| `composite_shove_tell_*`, `composite_lunge_*`, `composite_jam_*` | the shove, lunge and jam frames facing Carl on lit Floor 1 (640x360 and 1920x1080); **composite, not runtime** |
| `composite_bank_slide_640x360.png`, `composite_bank_slide.gif` | bank A: Carl on its lit top, the shudder, the slide at 3 px/tick, the settled remnant; **composite, not runtime** (wired by HRD-B) |
| `composite_jb_*` | the J-B post and stack: supported, cracked, released (the presenter's 1 px shudder and 3 px sag), falling on the sim's drop curve with the snapped strut, the landed junk rubble (spec section 11, E6); **composite, not runtime** (wired by HRD-C; the runtime stills are in `../f01-hoarder-presentation/`) |
| `composite_pinned_*`, `composite_getup_*`, `composite_pinned_getup.gif` | pinned under the landed junk rubble (drawn in front), then the get-up; **composite, not runtime** (runtime stills: `../f01-hoarder-presentation/pinned_*`, `get_up_*`) |
| `composite_emerge_*`, `composite_emerge.gif` | the Reveal in thirds with the shadow tint and the lip in front, then the first hittable tick with the opened mound; **composite, not runtime** (the runtime stills are in `../f01-hoarder-presentation/`) |
