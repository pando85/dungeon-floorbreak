# Mongo (adult / mount): native pixel scale candidates, round 2 style notes

**Status:** three redrawn scale candidates for human review. Not approved, not a production master, not animated. Round 2 replaces the round 1 grids completely (owner-authorised redo). The owner-approved references are still the concept package in `assets/concept/mongo/approved/` (adult board `Mongo_Adult_Approved_v1.png`, growth board `Mongo_Growth_Approved_v1.png`). Publication falls under `ART-PUBLICATION-ALL-V1` (owner-reported, unverified, no reuse licence).

| File | Opaque height (contact row to crest top) | Canvas | Pivot `[x, H-1]` | Silhouette width (ratio to height) | Colours used |
| --- | ---: | --- | --- | ---: | ---: |
| `mongo_adult_h64.grid.json` | 64 px | 92 x 68 | `[57, 67]` | 83 px (1.30) | 24 |
| `mongo_adult_h76.grid.json` | 76 px | 108 x 80 | `[68, 79]` | 100 px (1.32) | 24 |
| `mongo_adult_h88.grid.json` | 88 px | 126 x 92 | `[79, 91]` | 115 px (1.31) | 24 |

All three: `pixel-grid-v1`, one frame (1000 ms), tag `idle`, 4 empty rows above the crest, bottom row is the toe and claw contact row, pivot x between the centres of the two planted feet. Fully opaque pixels only. `python3 scripts/pixel_grid_to_ase.py lint --strict --expect-height N` exits 0 for each (0 orphans under the 8-neighbour rule, 0 isolated pixels, exact height) and `build` converts each to `.ase` (checked into a scratch directory only; no `.ase`, manifest or export was written to the repository).

## How round 2 was made

- Each size was drawn from scratch at its own resolution. No size is a resize of another, and no pixel of the boards (including the small 2D sprite mocks at the bottom of the adult board) was copied, traced or quantized. The boards were only viewed as enlarged crops (LATERAL view, "Neutral" and "Curioso" heads, details panel, scale panel, swatches, growth board).
- Authoring was done as hand-typed pixel runs (`row x0-x1:key`) per layer, painted back to front feather by feather. A small scratch helper only placed those runs on the canvas, wrote the grid JSON, rendered previews and listed orphan pixels. It never generated shapes, fills, shading or texture. Two honest caveats: (1) the straight shin and metatarsus rows were written as a per-row left position I chose plus a fixed skin pattern, expanded by a one-line loop; (2) for some orphan clean-up passes (mostly crest and tail edge pixels at 88) the helper proposed the majority neighbour colour, and I reviewed the list and applied it as explicit run lines, pairing highlights by hand where I wanted to keep them.
- The authoring run files are scratch tooling and are not committed. The grids are the source of truth and can be edited directly.

## Design read (from the board's LATERAL view, adapted to a compact idle)

- **Posture:** upright and compact. The neck rises in a shallow S with the head high, the body tilts forward, both legs sit under the body (near/rear leg back, far leg forward) and the tail plume counterbalances behind. This replaces round 1's flat, horizontal lizard pose.
- **Crest and mane:** the dominant silhouette feature. A big radiating fan of spiky pink feathers sweeps back from the brow and down the neck as a mane, painted as separate, overlapping, tapered feathers with a lit upper edge, a body tone and a dark underside line. The upper feather tips curl up slightly; long and short feathers alternate so the edge reads shaggy rather than combed.
- **Head:** big gold eye with a dark pupil set forward (curious, looking ahead) and a catchlight, framed by the blue face mask that wraps behind the eye and runs forward along the snout. Pink brow and snout top, cream lower jaw and cheek, closed relaxed mouth with an upturned corner and three tooth hints (accent pixels). Nostril and slightly hooked beak tip.
- **Wings (arms):** the folded forearm carries a fan of four long feathers hanging down and back. Pink coverts at the root, a blue band, a short pink band and long strong blue tips. Bands are staggered along the fan (76, 88) so they follow the feathers instead of forming horizontal stripes. A small clawed hand shows at the chest.
- **Tail:** a full layered plume. Three broad pink scimitar feathers arch up and back and droop at the end; two partly hidden feathers fill the hollows and add tips; a blue under-tail feather sits at the bottom; a blue covert band runs along the top of the tail base and continues onto the rump. The fan end shows six separate tips.
- **Legs and feet:** feathered thigh with a fringe at the knee, orange-pink scaled shin running back to a pronounced hock, metatarsus forward to the foot. Each foot has a ball pad, a lit near toe and a darker far-side toe, each ending in a dark hooked claw with a pale rim, plus a raised sickle claw arching above the near foot. Both feet are planted on the contact row.
- **Feather texture:** deliberate clusters only: per-feather shading in crest, tail and wing, a few hand-placed feather shingles on the flank and shoulder, the cream ragged edge down the throat and the knee fringe. No procedural banding.
- **Rider back:** the back between the mane and the tail coverts is a smooth, flat run with no spikes; the blue covert band rising behind it reads as a natural cantle.

## Palette (24 named colours, identical in all three files, limit 28)

Based on the board swatches (pink `#fd8db1`, hot pink `#f65087`, red `#e63757`, crimson `#ab2348`, blues `#3f74be` / `#4e88cf`, cream `#fddec1`), sampled from the board and then hue-shifted by hand. Light from the upper left (front), shadows shift toward crimson, plum and violet, never grey or black.

| Key | Name | Hex | Role |
| --- | --- | --- | --- |
| `o` | outline_plum | `#4a0f3c` | selective outline: eye rims, mouth line, under-jaw, lower body and thigh edges |
| `1` | plume_highlight | `#ffb2c6` | sheen streaks on crest, tail and thigh |
| `2` | plume_light | `#ff6b97` | lit upper edges of feathers, back |
| `3` | plume_base | `#e83a66` | dominant red-pink body colour |
| `4` | plume_shade | `#b0215a` | crimson feather undersides and form shadow |
| `5` | plume_deep | `#781650` | deep plum separation lines between feathers |
| `a` | blue_highlight | `#94c8ff` | lit edges of blue feathers and the mask |
| `b` | blue_base | `#4a8ae6` | mask, wing feathers, coverts, under-tail feather |
| `n` | blue_shade | `#3159c0` | shaded blue |
| `v` | blue_deep | `#2a3590` | blue feather tips and gaps between wing feathers |
| `C` | cream_highlight | `#fff3de` | lit throat and chest |
| `c` | cream_base | `#fbd3ac` | jaw, throat, chest |
| `t` | cream_shade | `#df9f96` | rosy cream shadow, throat feather edge |
| `S` | skin_highlight | `#ffae86` | lit back edge of shins, toe tops |
| `s` | skin_base | `#f2785c` | orange-pink legs, feet, hands |
| `z` | skin_shade | `#c44c5c` | leg shade and the far leg base |
| `Z` | skin_deep | `#86304f` | leg and foot outline, sole |
| `G` | claw_highlight | `#c2bff0` | pale rim on claws (keeps dark claws readable on dark stone) |
| `g` | claw_base | `#4a3d78` | dark claw body |
| `Y` | eye_gold | `#ffd83c` | iris |
| `O` | eye_amber | `#e88a1a` | lower iris |
| `x` | pupil_dark | `#1f0b26` | pupil (darkest value, used nowhere else) |
| `y` | eye_spark | `#fffbe8` | catchlight, `accent: true` |
| `w` | tooth | `#fff8ec` | tooth hints in the closed mouth, `accent: true` |

**Outline and background separation:** Carl and Donut use a very dark plum outline (`#1b1124`). On Mongo that value would vanish into `#191a2c` and `#2c2841`, so his outline is a lighter, saturated magenta plum (`#4a0f3c`) and it is used selectively on shadow sides only. Top and left silhouette edges are bright lit pink or blue with no outline at all, so the silhouette separates from the stone by value and saturation. Claws are dark with a pale rim for the same reason. Hue-shift direction (warm to crimson/plum/violet) and the upper-left key light match the Carl and Donut notes.

## Proportions (measured on the grids)

| Feature | 64 | 76 | 88 |
| --- | --- | --- | --- |
| Crest top above skull top | 6 rows | 7 rows | 8 rows |
| Head length (back of skull to beak tip) | 23 px | 28 px | 31 px |
| Eye centre above contact row | 0.83 H | 0.82 H | 0.82 H |
| Crest fan reach behind the head | 21 px | 25 px | 29 px |
| Back (seat) line | rows 31 to 32 | rows 36 to 37 | rows 41 to 42 |
| Belly bottom above ground | 20 px (0.31 H) | 24 px (0.32 H) | 28 px (0.32 H) |
| Tail top | row 17 | row 20 | row 22 |
| Feet centres (near / far) | x 50 / 64 | x 60 / 76 | x 69 / 88 |

## Per-size pixel budgets

| Item | 64 | 76 | 88 |
| --- | --- | --- | --- |
| Iris (inside the rim) | 5 x 3 | 5 x 4 | 6 x 5 |
| Pupil and catchlight | 1 x 2 pupil, 1 px catchlight beside it | 2 x 2 incl. catchlight | 2 x 3 incl. catchlight |
| Crest and mane feathers | 7 (one merged lower mane tuft) + 2 neck tufts | 8 + 3 neck tufts | 9 (extra small spike in the top fan) + 3 neck tufts |
| Tail | 3 pink plumes, 2 hidden fillers, 1 blue under-feather, 6 tips, 3-row covert band | same, 4-row covert band | same, 4-row covert band |
| Wing fan | 4 feathers x 3 px, one pink band on the two long feathers | 4 feathers x 4 px, staggered blue band | 4 feathers x 5 px (front one 4 px), staggered blue band |
| Toe claw | 2 px wide, 3 rows | 2 px wide, 4 rows | 2 px wide, 4 rows, larger hook |
| Sickle claw arch | 4 x 3 | 5 x 4 | 6 x 4 |
| Teeth hints | 3 | 3 | 3 |
| Flank feather shingles | 3 | 4 | 5 |
| Visible opaque pixels (composite) | 2431 | 3445 | 4601 |

## Layers (identical names and order in all sizes)

Bottom to top: `tail`, `far_leg`, `far_arm`, `body`, `near_leg`, `head`, `crest`, `near_arm`. Tail, crest (crest plus mane) and near arm (arm, hand, wing fan) are separate for later animation. Unlike round 1, layers are painted with their hidden parts where it was cheap: the body continues under the wing and the mane, the tail root continues under the rump, and the crest has a full under-layer behind its feathers. The far arm is only a few blue wing tips and the far leg is not completed under the near leg; both need more work before a run cycle.

## Simplifications

- **64:** iris 5 x 3 with a single-column pupil so the gold still dominates; lower mane merged into one tuft; wing feathers 3 px with one pink band instead of the staggered band; covert band 3 rows; shingles reduced to three marks.
- **76:** the reference design. Everything is present at a modest density.
- **88:** extra crest spike, longer and wider wing feathers, rounder eye with a 2 x 3 pupil, bigger sickle hook. The extra room was spent on feather structure rather than more texture.
- **All:** teeth are three accent pixels, not a full row; scales on the legs are not drawn (they read as noise at 1x); the far foot is a simpler, darker version of the near foot.

## Critique log (summary)

Review method each round: 1x, 3x and 6x nearest-neighbour renders on `#191a2c` and `#2c2841` next to an enlarged crop of the LATERAL view, 10x to 16x zooms of the head, crest, wing and feet, and a three-size side-by-side with grey 52 px (Carl height) and 40 px (Donut height) proxies.

| Round | Sizes | Findings | Changes |
| --- | --- | --- | --- |
| R1 first draft | 76 | Upright pose and fan already read as Mongo, but the crest interior was noise, the thigh read as a ball with a big highlight, the wing was horizontal pink/blue stripes, the throat a flat cream column, the legs thin, the tail concentric rainbow arcs. | Rebuilt the crest feather by feather with an under-layer; redrew head, rounder eye and taller brow; narrowed the cream throat into an S with a pink neck side. |
| R2 structure | 76 | Crest planks too uniform; thigh still a ball; wing a striped skirt; tail arcs with hollow dark interiors. | Tapered crest feathers with curled tips; integrated forward-leaning thigh; wing as a fan of diagonal feathers; tail redrawn as broad scimitar feathers plus hidden filler feathers so the plume is dense. |
| R3 colour and read | 76 | Too much uniform pink, blue nearly lost; wing too small; feet and claws unreadable; holes in the lower mane. | Blue covert band on the tail base and rump; longer wing fan with more blue; mane under-layer extended; claw palette lightened with a pale rim. |
| R4 detail | 76 | Wing bands lined up into stripes; claws read as purple noise; thigh highlight too strong; crest still combed. | Staggered wing bands; raised sickle arch; softened thigh; curled C3/C4 tips; first orphan pass (119 to 0, hand-decided). |
| R5 feet and flank | 76 | Flat flank; toes and claws still indistinct at 16x. | Flank and shoulder shingles; feet redrawn as ball pad, near toe and far-side toe with hooked claws. |
| R1 to R4 | 64 | First native draft: eye mostly pupil; wing pixels scattered; 47 orphans. Then: iris too small at 1x; flat flank; foot slab. | Wing rebuilt as 3 px feathers; iris widened to 5 px with a single-column pupil; shingles; foot redrawn like 76. |
| R1 to R4 | 88 | First native draft: crest feathers all the same length (comb); 78 orphans; thigh highlight blob; foot slab. | Shortened C4 and M2 so lengths alternate; orphan pass; softened thigh; shingles; foot redrawn. |
| Cross-size | all | Same pose, colour map, crest direction, mask, wing map and tail tip count; eye at 64 was weakest. | 64 eye fix, consistent foot and shingle treatment across sizes. |

## Ranking (honest, pending human game-feel review)

Reference scale: Carl 48 to 56 px (52 px proxy), Donut about 32 to 48 px (40 px proxy), 640 x 360 virtual screen.

| Size | vs Carl 48 / 52 / 56 | Seat height and mounted stack (Donut 32 to 48) | Footprint on 640 x 360 | Verdict |
| --- | --- | --- | --- | --- |
| **76 (1st)** | 1.58x / 1.46x / 1.36x | seat at 43 px, stack 75 to 91 px | 21 % of height, 100 px (16 %) wide | Best balance. At 1x the crest fan, pink plume, blue wing and gold eye in the blue mask all read; the face is charming at 3x; mass is clearly above Carl without dominating a room. Most review rounds went into this one. |
| **88 (2nd)** | 1.83x / 1.69x / 1.57x | seat at 50 px, stack 82 to 98 px | 24 % of height, 115 px (18 %) wide | Richest: best eye, most crest layering, clearest wing and claws. Best rider fit for a 40 to 48 px Donut. Cost: a quarter of the screen height, heavy in platforming, about 33 % more pixels per frame than 76. Choose if mounted play is central. |
| **64 (3rd)** | 1.33x / 1.23x / 1.14x | seat at 36 px, stack 68 to 84 px | 18 % of height, 83 px (13 %) wide | Cheapest and still identifiable at 1x (fan, plume, blue wing), but the eye is a 5 x 3 patch, claws and teeth are 2 to 3 px marks and he reads less like a mount next to a 52 px Carl. |

Authoring cost per frame, measured as visible pixels: 64 about 2.4k, 76 about 3.4k, 88 about 4.6k. A 6-frame run would scale roughly with that, and the crest and tail layering is the expensive part at every size.

## Remaining weaknesses

1. The tail is shorter and rounder than the board's long sweeping plume, because of the width cap (about 1.3x height). It risks a rooster read; a run pose can stretch it.
2. Crest feathers C3, C4 and M1 are still fairly straight and horizontal; the board's crest is shaggier with more curved, overlapping tips.
3. The crest and mane fan sweeps back over the front of the seat: at 76 the lowest mane tips come down to 1 to 2 rows above the back line around x 57 to 66, and the crest fan overhangs the seat higher up. A rider would sit in front of the lower mane tips; mount states may need the mane folded or lifted (crest is a separate layer). The clear, flat seat run is x 48 to 63 at rows 36 to 37 (body layer), suggested seat anchor (57, 36), that is (-11, -43) from the pivot; 64: (49, 31) = (-8, -36); 88: (66, 41) = (-13, -50). Measurements, not a decided socket.
4. Claws are readable at 3x and up but are only a few pixels at 1x; teeth are hints only.
5. Neck is long and upright with a straight cream throat front; the board's neck is thicker and more forward.
6. The near shin and metatarsus are plain skin ramps with no scale or tendon detail.
7. The outline colour is a deliberate deviation from Carl and Donut's near-black plum and needs a cast-wide decision in the three-character colour test.
8. Not tested in the real dungeon camera, beside real Carl and Donut sprites, against magenta magic VFX, or mounted.

## Open canon and design questions

- Toe count: one near toe and one far-side toe plus the raised sickle are drawn per foot (board interpretation). Confirm when lore research lands.
- Board size captions (18 cm, 1.2 to 1.5 m) are non-canon AI annotations; 64, 76 and 88 px are scale candidates only.
- Whether the default neutral keeps the mouth closed with tooth hints (chosen here, matching the board's Neutral/Curioso heads).

## Next steps

Human review of the three candidates in `generated/art/scale_study/` style scenes once manifests are added by the integrator (one manifest per size with `grid_source`, tag `idle`, pivot as in the table). Then regenerate the `.ase` files with `scripts/export_assets.sh`, pick one size, and complete hidden layer parts (far leg, far wing) before the `run` cycle.
