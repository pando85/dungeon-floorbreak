# GOB-02B-DEATH evidence (design stage)

Spec: [gob-02b-death-spec.md](../../02-art/gob-02b-death-spec.md) (variant B of [GOB-02-DEATH](../../02-art/gob-02-death-spec.md)). Every image here is a still or GIF composed from the committed pixel-grid sources. None is a Bevy capture or a feel approval. The scenes use the same seeds and placements as the [A evidence](../gob-02-death/README.md), so the A and B sheets line up.

**Prerequisite:** the lit scenes and the toolkit evidence need the Floor 1 kit atlases and the death atlases under `generated/art`, which are git-ignored. In a fresh checkout, run `bash scripts/export_assets.sh` once. Then regenerate:

- the stills and sequence GIFs: `python3 scripts/gob02b_death_evidence.py`;
- the toolkit output: `python3 -m scripts.pixelart evidence assets/source/characters/goblin/death/goblin_b_death_h36.pixelart.json --out docs/evidence/gob-02b-death/poses`, and the same for the `parts` config with `--out .../parts`.

As for A, only the contact sheets and the pose GIFs of the toolkit output are kept.

| File | What |
| --- | --- |
| `a_vs_b_x4.png` | every frame that differs between A and B (poses 0 to 5; parts `head`, `head_dead`, `torso_upper`, `ear`) at x4, A above B; band 1 at preset 4, band 2 at preset 1 (gore hidden); a 1x strip at the bottom |
| `frames_x8.png` | the four B pose keys and the B `head` and `head_dead` parts at x8 (preset 4) |
| `preset_poses_x4.png` | every B pose frame at presets 4, 3, 2, 1 (the accessory is shown at every preset) |
| `preset_parts_x3.png` | every B part at presets 4, 3, 2, 1; parts outside a preset's class filter are dimmed and crossed out |
| `scenes_1x.png` | five 640 x 360 lit Floor 1 views at 1x with Carl and Donut: crush under the ENV-02 stone (L), the remains hold in front of the landed stone (L+70), the decapitation tick (D), the headless hold (D+60), an irregular explosion settle |
| `storyboard_crush.png`, `storyboard_decap.png` | the A section 8 timelines on the B sprites: L, L+3, L+6, L+30 and D, D+4, D+10, D+40 |
| `gif_crush_sequence_x1.gif`, `_x3.gif` | crush at preset 4: poses 0, 1, 2 and pool stages at L, L+3, L+4, L+6, L+8, L+20, L+60; unlit, exact palette |
| `gif_decap_sequence_x1.gif`, `_x3.gif` | decapitation at preset 4: D, D+2, D+4, D+7, D+10, D+14, D+40, D+100 (head flight, jet, buckle, hold, pool stages); unlit, exact palette |
| `poses/`, `parts/` | toolkit output: contact sheets at x1 and x4 (both), GIFs of the four pose tags at x1 and x3 |

A test (`tests/test_art_gob02b_death.py`) checks that every frame of every GIF here still holds the goblin skin `#7a9e36` and the blood red `#b8322e`.

Checksums: `SHA256SUMS` covers every file in this folder and its subfolders. `poses/` and `parts/` also carry the toolkit's own list.
