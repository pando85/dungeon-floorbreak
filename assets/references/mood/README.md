# Dungeon Floorbreak — approved mood references

**Status:** artistic direction approved; **gameplay-scale quality, exact objects, character designs and equipment are not approved**.

These two AI-generated illustrations are **mood boards**, not sprites, level maps,
animation sheets, evidence of playable features, or production-quality references.
They were generated in the project owner's ChatGPT conversation on 2026-11-15.
The owner approved the overall gothic-industrial atmosphere and theatrical
action presentation, **not** the visual detail or character depictions.

## Source files and byte integrity

| File | Purpose | Source SHA-256 | Repo status |
| --- | --- | --- | --- |
| `a_detailed_pixel_art_game_screenshot_scene_a_vibr.png` | Action composition: catwalks, orange explosions, magenta/violet arena | `9d6cd32fcee3d9e5646fbc85b8562da3ca842236b2a665f9b1d6804b19f39847` | Committed |
| `a_highly_detailed_pixel_art_background_matte.png` | Background-only atmosphere: arches, audience, industrial pipes and lighting | `6d1867a67a9893c46d1fc66f862a5145c1d4080bb805405ec08200b900805e72` | **Awaiting correct binary upload** |

**2026-11-15 integrity incident:** two different filenames were inadvertently
committed with the **same action-image blob** (`ba44a50a401fb401111803654f4044b5fbde348d`).
The erroneous background file was removed, rather than leaving a misleading
duplicate. The correct background-only PNG exists in the original conversation
but has not yet been transferred to this public repository.

**Restoration:** put the **original** `a_highly_detailed_pixel_art_background_matte.png`
under this directory and commit it. No conversion/re-export should be needed.
The docs check automatically confirms the image SHA-256 once the file exists.
After restoration, this status line can be updated to **Committed**.

## What to keep / what to ignore

**Keep:** rich gothic-industrial dungeon atmosphere, layered blue/violet
masonry, neon magenta entertainment machinery, copper pipes, catwalks,
crowd silhouettes, rusty utilitarian props, warm amber fire and expressive
cinematic destruction.

**Ignore:** the current level of pixel/sprite detail, exact texture, AI
typographic noise, character anatomy, guard designs, actual platform collision,
unreadable microdecoration and any unevidenced equipment. **The action image's
shotgun is NOT Carl's weapon** and must never inform a Carl-specific design.

Canonical Carl concept and its own checksum remain in
[`assets/concept/carl/approved/`](../../concept/carl/approved/README.md).
Do not crop or extract character/terrain sprites from these mood images.

## Workflow and quality gates

- Material, character and FX assets must be newly authored as editable Aseprite
  sources with explicit pivots, atlas tags and provenance.
- Real rendering and legibility must be reviewed at native 640×360 (16:9) and
  640×400 (16:10), including 1× and integer-scaled output.
- Check artistically chosen references against a **separate** fidelity target;
  user approval of atmosphere is not approval of generated detail.
- No specific franchise adaptation/distribution rights can be inferred from
  the presence of these public mood images. Review
  [rights and release](../../../docs/04-production/rights-and-release.md).

## Verification

```sh
python3 scripts/check_docs.py
sha256sum assets/references/mood/*.png
```

The docs checker verifies recognized filenames with the expected original
SHA-256, rejects duplicates and fails on unexpected substitutions. The
background source is optional **only while its original is being restored**.
