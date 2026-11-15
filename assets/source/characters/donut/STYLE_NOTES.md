# Princess Donut — floor 1 scale study: style notes (round 2)

**Status:** three static native-resolution studies awaiting human artistic review. Nothing here is a locked production sprite, and this document does not decide the scale. Work order: [`docs/07-agents/donut-floor1-scale-study.md`](../../../../docs/07-agents/donut-floor1-scale-study.md). Round 2 replaces round 1 entirely. Round 1 read as a generic spotted calico: a boxy head, small eyes, Dalmatian spots, a rigid tail column and a boxy torso on visible legs.

All three sources use `pixel-grid-v1`, one frame (200 ms), tag `idle`, layers bottom to top `tail, body, head, face, gear`:

| File | Opaque height (contact row to tiara gem) | Canvas W×H | Pivot (centre of planted paws, contact row) | Opaque px |
| --- | --- | --- | --- | --- |
| `donut_h32.grid.json` | 32 | 38×36 | [17, 35] | 844 |
| `donut_h40.grid.json` | 40 | 46×44 | [22, 43] | 1305 |
| `donut_h48.grid.json` | 48 | 56×52 | [26, 51] | 1893 |

The following were measured with `scripts/pixel_grid_to_ase.py`. `lint --strict --expect-height N` exits 0 for all three: the heights are exactly 32/40/48, the contact row equals pivot y, and there are 0 orphans, 0 isolated pixels and 0 drift. Accent keys are `w`, `N`, `M` and `P`. `build` also exits 0. The highest pixel is the magenta tiara gem at every size. Three paws sit on the contact row: the near hind paw, the near fore paw and the far fore paw. All pixels are fully opaque. The generated `.ase` files are not committed with this change; rebuild them through the manifest/export flow when the study is wired up.

## How it was made (honest provenance)

- Each size was drawn independently at its own resolution. No size is a resize of another, and no pixel comes from the concept board or its tiny "gameplay sprite concept" mocks, which were not used. The board was only viewed as enlarged crops (core portrait, Floors 1–2 card, turnaround, expressions) for proportions, patch map and gear restraint.
- Method: each size was blocked in as run-length rows (one run per placement decision), then refined by direct edits to individual pixels in the text grid. Small scratch helpers did bookkeeping only: run-length expansion, splicing a run into a row, compositing layers for review, and listing orphans. Every orphan the lint found was resolved as an art decision, by pairing a highlight diagonally or merging a lone pixel into its cluster. No pixel was moved just to satisfy the rule. The helpers are not committed; the grids are the source of truth.
- Light comes from the upper left. The outline is selective: deep plum `K` on the shadow (right and bottom) side, and soft plum `j` or a lighter value of the painted colour (`c`, `o`, `W`) on lit edges. There is no pure black and no outline inside the face.

## Shared palette (22 colours, identical in all three files; limit 24)

The shadows are hue-shifted toward plum so the cat sits in the violet dungeon. The dark coat is plum-brown, not black.

| Key | Name | Hex | Role |
| --- | --- | --- | --- |
| `K` | outline_plum | `#1e1229` | outline on the shadow side, pupils, eyelid arc |
| `j` | outline_soft_plum | `#4a2846` | outline on lit edges of dark fur |
| `D` | coat_dark_shadow | `#2a1731` | dark fur in shadow, tail core shadow, fur skirt |
| `B` | coat_dark | `#45263a` | dark tortoiseshell base |
| `H` | coat_dark_light | `#6c3e52` | rim light on crown and back |
| `o` | ginger_shadow | `#c45a36` | ginger shade, ginger lit-edge outline, eye ring on the far cheek |
| `O` | ginger | `#ec8b3e` | ginger marbling |
| `G` | ginger_light | `#ffb867` | ginger highlights (forehead blaze, back) |
| `c` | cream_shadow | `#b88ca9` | cream shade (plum-shifted), chin line, toe lines, far paw |
| `C` | cream | `#f3d3c3` | muzzle, chest ruff, paws, tail fringe |
| `W` | cream_light | `#fff3e8` | whisker-pad and ruff highlights, tail fringe tufts |
| `y` | eye_amber | `#c8751c` | darker upper band of the iris |
| `Y` | eye_gold | `#ffcf3c` | iris |
| `g` | eye_gold_light | `#fff09a` | lower iris glint (40/48 three to four px, 32 two to three px) |
| `w` | catchlight_white | `#ffffff` | catchlight (accent) |
| `N` | nose_pink | `#f2708f` | nose (accent) |
| `E` | inner_ear_pink | `#d47a93` | inner ears |
| `L` | tiara_lilac | `#dbc6fb` | tiara |
| `l` | tiara_lilac_shadow | `#8a68c8` | tiara band, butterfly body (40/48) |
| `M` | gem_magenta | `#e2309f` | tiara gem, the highest pixel (accent) |
| `V` | collar_violet | `#9a6ee0` | 1 px collar, foreleg band (48), butterfly body (32) |
| `P` | charm_pink | `#ff95d3` | butterfly wings (accent) |

Changes from round 1: all coat and outline values were re-tuned toward plum, `g` was added for a luminous lower iris, and the round-1 key set is otherwise kept.

## Pose, coat and gear map (same placement at every size)

The pose is a seated three-quarter front loaf facing right (screen right), taken from the board's "3/4 Front" turnaround. Near side = viewer's left.

- **Head:** a big round "pansy" Persian head, wider than tall (head width ≈1.6× its height including cheek tufts). The cheek ruff bulges past the eyes in the lower half. The face is flat: the nose sits between the lower halves of the eyes and the whisker pads are short. The ears are small, rounded and low-set at the head corners, with pink inners and a cream tuft on the near ear.
- **Eyes:** very large gold eyes set wide and low, gazing slightly up. Each has a short dark lid arc, amber only in the top band, a large pupil touching the lid, gold below, a light-gold lower glint and the catchlight at the upper left of the pupil on both eyes. The far eye is one column narrower (three-quarter foreshortening).
- **Asymmetric tortoiseshell mask:** dark crown and dark around the near eye. A ginger blaze runs from the crown down between the eyes and turns cream at the nose bridge. A 1 px dark line separates the blaze from the far eye. The far eye sits in a ginger cheek patch with an `o` ring on its outer side, and at 48 a dark marble streak runs along the outer cheek. The near cheek has a ginger patch below and outside the eye. There is a small ginger fleck by the near ear and a ginger streak by the far ear.
- **Cream:** muzzle and whisker pads with highlights, a rounded chin closed by a `c` chin line, and a big chest ruff (mane) that flares past the chin and narrows to the forelegs, with a few diagonal fur strokes and tufted bottom edges.
- **Body:** a low, round loaf. The near side is mostly dark tortoiseshell with two irregular ginger blotches: a back blob behind the shoulder and a larger haunch blob with a dark brindle bite. A 2 px cream fleck sits on the near shoulder. The far shoulder frames the ruff in ginger and dark, with two outline fur tufts. A dark fur skirt with hanging tufts hides the legs; only small round paws with cream toes show, and the far paw is in cream shadow.
- **Tail:** an immense plume on its own layer, drawn completely, including the base hidden behind the rump, so it can sway later. It is widest through the middle and makes a C-bulge to the left with a small tip flick to the right (a mild S). The core is dark with ginger marbling in the upper half, and the outer edge has a light cream fringe with diagonal-pair tuft highlights. The side facing the body is `D`/`K`, against the body's `j`/`H` rim light, so value contrast separates tail from body. Near the tip it clears the head by 2–3 px. Through the middle it tucks behind the near cheek ruff (0 to 3 px overlap; most at 32), separated only by value: the dark tail edge against the cheek's ginger patch and soft outline.
- **Gear (restrained, provisional):** a small lilac tiara centred over the nose line, with a magenta gem as the highest pixel. A 1 px violet collar sits directly under the chin line, with a butterfly charm hanging at the centre. The foreleg band is 48 only, a 1 px violet line just above the near fore paw. There are no shoulder scales at any size.

## Per-size pixel budgets

| Feature | 48 | 40 | 32 |
| --- | --- | --- | --- |
| Head (ear tip to chin) | 25 rows (52 %) | 20 rows (50 %) | 16 rows (50 %) |
| Head width incl. cheek tufts | 41 | 34 | 28 |
| Eye near / far (W×H) | 8×8 / 7×8 | 7×7 / 6×7 | 6×6 / 5×6 |
| Pupil (incl. catchlight) | 4×4 | 3×3 | 2×4 (tall) |
| Catchlight | 2×2 per eye | 2×1 per eye | 1×2 per eye |
| Amber upper band | 4 px row under lid | 3 px row under lid | 2 corner + 2 side px per eye (no lid) |
| Nose / mouth | 3+1 px triangle / 3 px "ω" | 2×1 / 2 px | 1 px / none |
| Tiara (W×H incl. gem) | 7×6, gem 3 px | 5×5, gem 2 px | 5×4, gem 2 px |
| Collar / charm | 9 px / 5×2 butterfly | 6 px / 3×2 butterfly | 5 px / 3×1 (wings + violet body) |
| Foreleg band | 5 px | dropped | dropped |
| Tail max width / rows | 17 / 34 | 14 / 28 | 11 / 23 |
| Paws on contact row | 3 × (5 px bottom) | 3 × (4 px) | 3 × (3 px) |

What each size had to simplify:

- **48:** nothing essential is lost. It has room for the 2×2 catchlight, a triangle nose with an "ω" mouth, a three-tone tiara with a 3 px gem, a recognisable butterfly, the foreleg band, an extra dark marble streak on the far cheek and two-tone ginger on the back.
- **40:** the band is dropped (on balance it cluttered the paw), the nose is 2 px and the mouth 2 px, and the butterfly is reduced to 3×2. The amber band shrinks to a 3 px row. Everything else of the coat map survives with fewer fur strokes.
- **32:** the eyes lose the lid arc entirely. At this size a lid row plus pupil swallowed the gold and made her look sleepy or sideways, so the eye is gold against the dark fur with amber corner pixels and a tall 2 px pupil. The nose is 1 px with no mouth, the charm is 3×1, the band is dropped, the blaze is 3 px, and the tail fringe has fewer tufts. The coat map is unchanged, only simplified.

## Critique log (summary)

Each size went through more than four critique and revise cycles. Viewing was at 1×, 4× and 6× on `#191a2c` and `#2c2841`, plus a 12× gridded zoom, next to an enlarged crop of the board's core portrait at similar apparent size.

- **40 (≈10 cycles):** (1) The block-in had a wide head, but square "goggle" eyes, a flat crown, a cream slab, chevron marbling and a striped tail. (2) Rebuilt with a domed crown, mane-shaped ruff and plump plume. (3) A side-by-side eye lab on dark and ginger fur picked a short lid arc, amber band, big pupil and light-gold glint; it was the only variant that looked up rather than sleepy. (4) Fixed the frog-like cream jaw, the "eyebrow" fleck over the near eye, the slash-like blaze and row-wise tiger bands, by redrawing the flank as 2D blotches and narrowing the jaw. (5) Tail redrawn in 2D to remove a dotted "zipper" fringe. (6) Rounded the chin with a `c` line so face and chest separate. (7) Orphan pass. (8) In the cross-size check, the amber side pixels read as worried brow corners; amber was limited to the top row and the eyes became round and wide open. (9) Shoulder fur tufts and an S tip flick.
- **48 (≈6 cycles):** (1) Fresh block-in with the 40 lessons. The eyes were strong, but the far cheek was a flat orange slab and the cream jaw too wide. (2) Added a dark marble streak on the outer far cheek and narrowed the jaw. (3) Orphan pass; the foreleg band became a clean 5 px line. (4) Amber limited to the top band. (5) Shoulder tufts and tail tip.
- **32 (≈6 cycles):** (1) Block-in: the eyes read dark and sidelong. (2) Pupil narrowed and catchlight made vertical. (3) Amber removed beside the blaze, where it merged with the ginger. (4) Pupils centred. (5) In-context eye lab of four variants: the lidless eye with amber corners and a tall pupil read cutest and most open. (6) Shoulder tufts and tail tip.
- **Cross-size check** (all three at 1×, 4× and 6× beside a 52 px neutral grey bar, on both backgrounds): same facing, coat map, gear placement and eye character. All three read as a round tortoiseshell Persian with dominant gold eyes, a plume and a small tiara, and are clearly improved over round 1.

## Honest ranking (for human review, not a decision)

Relative height versus Carl's provisional 48–56 px: 32 = 57–67 %, 40 = 71–83 %, 48 = 86–100 %. The design docs' provisional Donut range is 28–36 px.

1. **40, best balance (recommended for review).** The eyes and muzzle carry every identity anchor: 7×7 eyes with amber band, catchlight and glint, a 2 px nose and mouth, a 5×5 tiara with gem, a collar and a butterfly. The round head, plume and loaf silhouette read at 1×, and the tail is clearly separate. Contact and pivot are clean (three paws, centred pivot). She is clearly smaller than Carl while staying prominent. The authoring cost is moderate (1305 px), workable for a four-frame idle and later locomotion. Cons: above the docs' 28–36 px guidance; the chest ruff is still a large cream mass with few fur strokes; no foreleg band.
2. **48, most expressive and cutest close up, heaviest.** It has the biggest, most luminous eyes, an "ω" mouth, a legible butterfly and band, the richest marbling, and the most room for blink, ear and tail acting. Cons: as tall as Carl's lower bound (86–100 %), which conflicts with "Donut smaller than Carl"; the dark forehead and the cream chest are large flat areas; it is the most expensive to animate (1893 px, about 45 % more than 40).
3. **32, cheapest and closest to the docs' range, weakest face.** The face still works at 1× (6×6 eyes with catchlight and tall pupils, 1 px nose), and the silhouette, plume and tiara remain readable. Cons: no mouth, no band, and only corner pixels of amber; the charm is a 3 px dot; the face is cramped, so expression changes (smug, calculating, furious) will have very few pixels to work with. Cost is lowest (844 px).

## Known weaknesses (all sizes unless noted)

- The chest ruff is a large, mostly flat cream area. It has a few strokes and tufted edges, but less fur texture than the board, and at 6× it can read as an apron.
- The body outline is mostly smooth: two outline tufts on the far shoulder and a tufted skirt, but no long fur wisps on the back or rump.
- The tail's S is mild (a C-bulge with a tip flick), and its repeated diagonal highlight pairs on the fringe can read as a dotted edge at 6×.
- The forehead blaze is a fairly uniform vertical stripe (most visible at 48), and the dark crown is a big flat mass at 48.
- The far fore paw in `c` reads slightly lilac-grey, like a sock.
- Where the plume tucks behind the near cheek, dark tail meets dark cheek over a few rows (32 especially). The silhouette stays readable through the tail's fringe and ginger, but the boundary there is soft.
- The pose is a seated three-quarter front loaf (board turnaround), not the side or three-quarter-side body that `walk`/`run` will need; locomotion frames will need a body turn.
- Only viewed on flat backgrounds with a floor line, not yet in the 640×360 gothic scene through `make_scale_comparison.py`. No human has judged it in a game context.
