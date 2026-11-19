# LLAMA-02-DEATH: Bad Llama death poses and death-parts sheet, design spec

**Status:** executor pass complete (see the [execution record](#execution-record)). Spec and hero keys frozen (poses 0, 2, 3, 5, 6, 7; parts 0, 1, 2); in-between frames 1 and 4, parts 3 to 31, the full manifest metadata, the extra evidence and the tests are built. The silhouette gate passed and its rulings are recorded in section 13. Independent review pending. Nothing here is wired into Bevy. The owner approved intensity preset 4 as the default (2026-11-18, recorded in the [death-system proposal](../03-tech/monster-death-system-proposal.md#b-suggested-data-contract-illustrative-rust-design-not-compile-ready-abi)). Gameplay acceptance of any finish, and which causes select which finish, are separate owner decisions.

Scope: the first non-goblin death set (death-system step "extend to other monsters"). It mirrors [GOB-02-DEATH](gob-02-death-spec.md) for the Floor 1 spitting llama ([LLAMA-01](llama-01-spec.md)):

1. Six contact and aftermath keys on the LLAMA-01 frame and pivot: `crush_contact`, `crushed_remains`, `decap_contact`, `headless_collapse`, `slam_contact`, `broken_heap`.
2. The llama death-parts sheet: head, neck, legs, torso chunks, wool tufts, the ruptured spit gland, teeth, bone gibs (preset 4 only), blood and molten decals, in the goblin parts manifest schema so the DEATH-C spawner consumes both species with one code path.

Sources: [`llama_death_h64.grid.json`](../../assets/source/characters/llama/death/llama_death_h64.grid.json) and [`llama_death_parts.grid.json`](../../assets/source/characters/llama/death/llama_death_parts.grid.json), written by [`scripts/author_llama02_death.py`](../../scripts/author_llama02_death.py). Evidence: [docs/evidence/llama-02-death/](../evidence/llama-02-death/README.md).

## 1. Canon and tone

- The llama is the LLAMA-01 Bad Llama: 64 px, mangy tan wool with streaky mange, a long S-curved neck, long banana ears, big ugly yellow buck teeth, and a throat that glows before it spits lava. Death art must not redesign it. Every pose and part keeps at least one 1x identity cue: the long neck, the ears, the yellow teeth or the glow.
- **Species signature: the spit gland bursts.** Whatever kills the llama, its lava gland ruptures. Crushed, it is squeezed like a tube and a molten geyser blows out of the open mouth; beheaded, the cut duct glows and dribbles down the throat; slammed, it coughs up a hot glob; at rest the gland drains into a cooling puddle that crusts. This is what makes a llama death read as a llama death at 1x, before any blood.
- [Antihero tone](../01-design/antihero-tone.md): grimy, darkly comic, brutal. The comedy is physical and animal: the squeezed-tube geyser, a dead farm animal on its back with its legs stiff in the air, a tongue lolled on the floor, X eyes, a buck tooth knocked out. The brutality is honest pixels: a hide that bursts along the back with rib ends showing, a snapped neck with a bone through the hide, a jet out of a long headless neck. Never cute, never a lingering close-up; the motion is in the first 10 ticks, the aftermath frames are holds.
- The [Death Lab](../../tools/art-references/death-lab/README.md) at intensity 4 gave the attitude only (a hard impact read, a decisive jet, a spreading pool, more pieces). None of its drawn cutouts, colours or timings were traced or sampled.
- Fan-concept layer: the llama is the replaceable fan-concept layer. The data contract (section 7) is IP-neutral (`head`, `neck`, `gland`, `molten`).

**Not approved by this spec:** which causes crush, behead, slam or explode a llama (domain eligibility), whether the throat is a weak point (it is **not**; the gland burst is presentation only), damage from the molten spray (none: cosmetic), any fragment or particle budget (set after profiling), sound, camera shake.

**Living horizon:** this is art for already-planned finishes (D-04 crush, PR D blade and earned explosive) plus a heavy-impact variant of the existing defeat. It adds no move, unlock or ability, so it does not change the [character progression horizon](../01-design/character-progression-horizon.md).

## 2. Frame, pivot, layers and anchors

**Poses atlas:** frames of **96 x 72, pivot (44, 71)**, the LLAMA-01 contract. All six tags are grounded: the lowest opaque row is 71 (tested). Authored facing right. The runtime mirrors around the pivot column (x' = 88 - x), anchors included.

**Parts atlas:** **32 x 32 cells, one part per frame**, cell pivot (16, 16), the GOB-02 contract. The cell pivot is the rotation centre. A mirrored part mirrors around x 16 (x' = 32 - x).

**Layers (both atlases), bottom to top:** `body`, `gore`, `molten`.

- `body` alone is always a **closed, outlined, bloodless sprite** (every coloured pixel touches only opaque pixels or the canvas edge; tested). A cut surface on `body` is the plain fur-and-outline cap that the rig's outline makes along the clip.
- `gore` holds every wound, blood and bone-end pixel. Keys: `o`, `B`, `b`, `P`, `e`. `B`, `b` and `P` exist on `gore` only (tested).
- `molten` holds the ruptured gland: the llama's own lava, not blood. Keys: `Y`, `O`, `R`, `r`, crust `a` and a `K` outline where it sits over empty canvas (like the LLAMA-01 spit glob). **It is shown at every preset**: it is the species signature and the cause feedback, the same class of pixel as the spit projectile that is visible in normal play.

This is the GOB-02 preset contract with one extra always-visible layer. The manifests name it: `preset_layer: "gore"`, `always_shown_layers: ["body", "molten"]`. A spawner that only knows the goblin set must treat any layer other than `gore` as always shown.

**Pose anchors (frame coordinates, facing right; in `death_poses.frames[].anchors`):**

| Pose (frame) | Anchor | xy | Use |
| --- | --- | --- | --- |
| `crush_contact` (0) | `hit_origin` | (42, 52) | the stone's anchor on its landing tick; radial origin for squeezed-out pieces |
| | `mouth`, `gland` | (90, 60), (91, 58) | blood and tooth emitter; molten drop emitter (geyser base) |
| | `squeeze_back`, `squeeze_front` | (4, 70), (76, 70) | floor squeeze-out points (splats, pool start) |
| `crushed_remains` (2) | `pool_origin`, `mouth`, `split` | (40, 71), (90, 69), (41, 58) | pool decal centre; molten puddle; the burst hide (trickle emitter) |
| `decap_contact` (3) | `neck` | (54, 22) | head part spawn: the head part's `neck` anchor goes here |
| | `hit_origin`, `jet_base`, `gland` | (54, 22), (54, 21), (56, 22) | blade line; blood jet emitter; molten emitter |
| `headless_collapse` (5) | `neck`, `gland`, `pool_origin` | (66, 44), (69, 46), (76, 71) | stump cut (trickle emitter), duct glow, pool decal centre (design follow-up D-N1) |
| `slam_contact` (6) | `hit_origin`, `mouth`, `kink`, `gland` | (42, 68), (89, 66), (69, 53), (87, 58) | impact point on the back; tooth and blood emitter; snapped neck; coughed glob |
| `broken_heap` (7) | `pool_origin`, `mouth`, `kink` | (82, 71), (90, 69), (69, 52) | pool under the head |

Frame 1 (`crush_contact` flatten): `hit_origin` (42, 55) (the stone sinks 3 px, render-only), the other anchors as frame 0. Frame 4 (`headless_collapse` buckle): `neck` (64, 36), `jet_base` (64, 35), `gland` (66, 36).

**Hazard draw order (data, gate ruling):** `hazard_draw: "over"` on the `crush_contact` frames 0 and 1 in `llama_death_h64.json` (and the `death_poses.hazard_draw` block). See section 13.1.

## 3. Palette `llama-death-floor01-v0` (20 of 20 per atlas)

The 17 `llama-floor01-v0` keys with the same hex (tested) plus the shared GOB-02 gore ramp with the same hex (tested). No new colour is invented.

| Key | Name | Hex | Role here |
| --- | --- | --- | --- |
| K | outline | `#1a1016` | outline, X eyes, pupils, the molten outline |
| a / s / m / l / L | fur deep / shade / mid / light / top | `#3a2a32` `#6e4e44` `#a37c58` `#cfab7c` `#ecd3a4` | wool ramp; on `molten`, `a` is the cooled crust |
| p | mange | `#4f2c26` | mange streaks |
| h | hoof | `#4a3c44` | hooves; a sole showing on snapped legs |
| T / t | tooth / tooth_shade | `#e6c447` (accent) `#9c7a26` | buck teeth; the `tooth` part |
| o | mouth | `#4a1020` | mouth interior; on `gore` the deepest blood (pool rims, old stain). Stands in for the goblin's `o` (`#5c1426`), one step darker, to stay at 20 |
| g | gum | `#a2404a` | gum line and the lolled tongue |
| e | eye (accent) | `#f1ead0` | eye whites; **bone ends on `gore`** (spine dot of a cut neck, rib ends, the snapped neck bone) |
| r / R / O / Y | glow deep / mid / hot / core | `#8c1e12` `#e2491c` `#ff9d2e` `#ffe680` (Y accent) | **`molten` only in this atlas** (no living throat glow on `body`): geyser, duct, coughed glob, puddle |
| B | blood_dark | `#6e1a22` | **gore only**: clotted blood, the shadow edge of a jet (GOB-02 `B`) |
| b | blood_mid | `#b8322e` | **gore only**: fresh blood and meat (GOB-02 `R`, renamed because `R` is the llama glow) |
| P | blood_wet (accent) | `#e2434b` | **gore only**: wet highlight, 1 to 3 px per blood shape, the jet core (GOB-02 `P`) |

Blood ramp, dark to light: `o` `B` `b` `P`. Molten ramp, cold to hot: `a` `r` `R` `O` `Y`. The two ramps meet only in the reds; they stay apart because molten always carries an `O`/`Y` core and blood never does, and molten has a `K` outline over empty canvas while blood has none. `P` is cooler and pinker than the brick, as on the goblin.

## 4. Pose list (poses atlas, 8 frames)

| # | Tag | Frame | Ticks (60 Hz) | Playback | Author |
| --- | --- | --- | --- | --- | --- |
| 0 | `crush_contact` | stone landing: pressed flat, geyser out of the mouth | 3 | once | **key** |
| 1 | `crush_contact` | flatten: pressed 2 px lower and wider, taller geyser | 3 | once | executor |
| 2 | `crushed_remains` | burst mat at the stone's foot, one leg snapped up | hold | hold | **key** |
| 3 | `decap_contact` | blade tick: headless recoil, jet, glowing duct | 4 | once | **key** |
| 4 | `headless_collapse` | buckle: knees folded, jet bent back | 6 | once | executor |
| 5 | `headless_collapse` | lying headless, neck stump propped, pool | hold | hold | **key** |
| 6 | `slam_contact` | heavy impact: thrown on its back, legs flung, tooth out | 6 | once | **key** |
| 7 | `broken_heap` | on its back, legs stiff in the air, neck snapped | hold | hold | **key** |

Sequences played by the renderer: `crush_contact` → `crushed_remains`; `decap_contact` → `headless_collapse`; `slam_contact` → `broken_heap`. Holds are state-driven (preview duration 100 ms).

**Which finish when (proposal for the gameplay spec, not approved):** D-04 falling stone → crush; environmental blade (PR D) → decapitation; a lethal hit with `force_tier: Heavy` (a heavy kick, a slam into a wall, a lethal fall) → slam; a `Routine` lethal hit keeps the LLAMA-01 `down` (frames 12 to 14). The slam pair gives a llama-specific heavy finish without blood jackpots on routine kills.

## 5. Intensity presets 1 to 4: derivation by data only

Presets are presentation-only. They never touch the domain, eligibility, collision or the replay hash. Same four switches as GOB-02, plus the molten rule.

| Switch | 4 (default, full) | 3 | 2 (reduced) | 1 (off) |
| --- | --- | --- | --- | --- |
| `gore` layer on poses and parts | shown | shown | shown, remapped `gore_muted` | **hidden** |
| `gore_muted` remap (gore layer only) | none | none | `b`→`B`, `P`→`B` | n/a |
| `molten` layer | shown | shown | shown | shown |
| part classes spawned | all (`gib_bone`, `gib_rib` at 4 only) | `wound`, `clean`, `molten`, `gib` (no bone), `blood` | `wound` (gore muted), `clean`, `molten`, `blood` (drops only) | `clean`, `molten`, and `head` in a decapitation |
| count scale (gibs, blood drops) | 1.0 | 0.5 | 0.25 (drops only, no gibs) | 0 |
| count scale (molten drops) | 1.0 | 1.0 | 0.5 | 0.5 |
| decals | splat s/m, pool 1 to 3, wall spatter a and b, smear, molten + crust | splat s/m, pool 1 to 2, molten + crust | splat s only (muted), molten | molten only |
| explosive finish | all body parts | head, torso_front, torso_rump, leg_fore, leg_hind, gland (no neck, ear, jaw or far legs) | head, torso_front, gland | fallback: LLAMA-01 `down` + `wool_tuft_*` + `tooth` + molten drops + dust |

- **Preset 1** keeps every silhouette and the species feedback: the crush still flattens under the stone with the geyser, the decapitation still loses the head (cut hidden) with the glowing duct, the slam still flips the llama with the coughed glob. No blood sprite is drawn. See the [preset sheet](../evidence/llama-02-death/keys_x4_presets.png).
- **Preset 3 vs 4 must read clearly different** (owner ask). Only 4 has the wall spatters, the third pool stage, the head-roll smear, bone and rib gibs, the full jet emission and the full explosion breakup.

## 6. Key poses (as authored)

All keys are rasterized by the LLAMA-01 rig (imported, not copied) from new pose dicts in `scripts/author_llama02_death.py`. A cut is a rig clip (a rectangle or a half-plane) applied before the outline pass, so the stump is closed by the rig's own 1 px outline; gore and molten are rules on their own layers.

- **0 `crush_contact` (`CRUSH`).** The landing tick. The ENV-02 falling block's anchor sits on frame (42, 52); its opaque footprint is x 13 to 76. Under it the llama is pressed into a wide wool mat (rows 52 to 71): the barrel is squashed wide (radius 8 to 8.5) and clipped flat at row 53, so the rig outlines a straight `K` underside on row 52 (at least 30 px, tested; nothing of the llama above it inside the footprint, tested). The legs splay flat on the floor, hinds back and fores forward, hooves showing. The tail sticks up out from under the left edge; the neck comes out under the right edge and the head lies on the floor in front, jaw sprung wide (both rows of buck teeth, `gape_up`/`gape_lo` stamps), the near eye popped (`bulge` stamp), the ears flung back flat. **The signature:** a molten geyser blows straight up out of the mouth (rows 31 to 60, a `Y` core in an `O`/`R`/`r` column, `K` outline) and breaks into two globs at the top: the squeezed lava gland. Gore: low squeeze-out fans on the floor at both ends of the stone with droplet pairs, a split seam of meat on the front flank under the stone's edge, blood down the jaw. **Draw order (proposal, open question 1):** on the contact ticks the stone draws **over** the llama; the boulder's rounded underside then covers the pressed barrel and only the head, legs and tail stick out, which is the read. Behind the llama (the goblin rule) the boulder floats over a lying llama at 1x.
- **2 `crushed_remains` (`REMAINS`).** The hold, drawn in front of the foot of `loose_block_landed` (actors layer, the goblin ruling). A low pressed mat (rows 56 to 71 without the leg), not a sheet: the ragged wool outline gives it a broken top. The near hind leg is snapped up and bent back with the hoof sole showing (the 1x cue that it was crushed, not lying down); the other legs lie flat. Neck and head flat on the floor to the right: X eye, buck teeth, tongue lolled onto the floor. Gore: **the hide burst open along the pressed back like a sausage** (a 23 px meat line along the top, three rib ends `e` poking up, two wet glints), one continuous seep along row 71 under the whole mat, blood out of the mouth. Molten: the gland drains out of the mouth along the floor line and crusts at the tip (`O` `R` `r` `a`).
- **3 `decap_contact` (`DECAP`).** The blade tick. LLAMA-01 frame 11 (`hurt`: neck flung up, fore hoof lifted, tail tucked) with the head removed by a blade plane through the upper neck at (54, 22.5), rising slightly forward. Below row 30 the body is LLAMA-01 frame 11 pixel for pixel (tested); no eye or tooth pixel remains on `body` (tested). On `gore`: the cut face (meat `b`, clotted `B` ends, the spine end `e e` at the mane side, a wet glint) and **the jet**: a 7 to 3 px column straight up out of the long stump to the top of the frame, leaning back the way the head flew, `B` on the shadow side, a `P` core near the base, breaking into gobbets. On `molten`: the severed spit duct glows in the front of the cut (`Y`, `O`) and a hot dribble runs down the throat edge (rows 24 to 32), plus one molten gobbet inside the spray. The long headless neck with a jet is the clearest 1x beheading read of the whole set.
- **5 `headless_collapse` (`HEADLESS`).** The hold. LLAMA-01 frame 14 (`down`: flat on the belly) with the head removed and **the neck stump propped clearly up off the chest** at about 60 degrees (neck chain (57, 61), (62, 52.5), (67.5, 42.5), radii 4.5 to 3.4, cut by a plane at (66.5, 44.5), normal (0.4, -0.92); stump top row 42, about 12 px above the back line). Redrawn in the design follow-up (gate note N1): the first version lay nearly level with the back and read as a generic headless carcass at 1x on lit brick; upright, the empty neck with its red cap is the long-neck cue, and it also continues the buckle (frame 4, stump at row 36) into the hold. Behind x 48 the body is LLAMA-01 frame 14 pixel for pixel (tested). Gore: the cut face, a run of blood down the front (throat side) of the propped neck to the floor and a 30 px pool on row 71. Molten: the duct still glowing at the front of the cut, one ember puddle crusting on the floor. The spreading pool is a decal (section 7), not this frame.
- **6 `slam_contact` (`SLAM`).** A heavy impact throws the llama onto its back. The barrel lies belly up (the mange streaks follow), the four legs flung up and out (fores up-right, hinds up-left, hooves at rows 34 to 41), the neck whipped into a kink and the head on its cheek on the floor, eye squeezed shut, mouth open. Gore: blood coughed out of the mouth in a rising arc that breaks into droplet pairs, a first splat under the head, and **a bone end through the hide at the neck kink**. Molten: one hot glob coughed up beside the mouth. The `tooth` part launches from `mouth` on this tick (it is not in the frame).
- **7 `broken_heap` (`BROKEN`).** The hold. Still on its back, the four legs **stiff in the air** (near-vertical, slightly splayed): the dead-farm-animal gag, which is the darkly funny half; the snapped neck with the bone through the hide and the head lying on its cheek with X eye, buck teeth and the tongue on the floor is the brutal half. Gore: the bone at the kink, a 24 px pool under the head, a stain under the back. Molten: the gland drains out of the mouth along the floor line. Tallest hold of the set (rows 31 to 71), still well below the idle llama's 64 px, so it never hides Carl's head.

**Part keys (parts atlas):**

- **0 `head`.** The LLAMA-01 idle head through the rig (skull, muzzle, jaw, open mouth with both rows of teeth, nostril, both ears pricked) with a new `shock` eye (wide white, small pupil) and a 4 px neck stub cut flat at the bottom. Gore: the cut face (meat, `e` spine end at the back, wet glint). Molten: the duct dot at the front of the cut. 23 x 23. Anchor `neck` (11, 24).
- **1 `neck`.** A neck segment from the LLAMA-01 idle neck (mane on the back edge), cut at both ends. Gore: both cut faces. Molten: the duct glowing at both ends. Only the explosive breakup spawns it (the decapitation's head carries its own stub). 14 x 16. Anchors `top` (20, 10), `bottom` (16, 23).
- **2 `leg_fore`.** The LLAMA-01 idle near fore leg (shin, knee knob, hoof) cut flat at the shoulder. Gore: the cut face. The rigid stick leg with a hoof is the most legible llama piece after the head. 10 x 25. Anchors `shoulder` (16, 5), `hoof` (19, 27).

## 7. Parts list and metadata schema

Keys 0 to 2 are frozen. The executor appends 3 to 31 in this order (indices are part of the contract). Classes: the GOB-02 classes `wound`, `clean`, `gib`, `blood`, plus **`molten`** (gland pieces and lava drops and decals; shown at every preset, section 5). Blood decal ids, sizes and nesting rules are the GOB-02 ones so the spawner's decal code is shared.

| # | id | Class | Size (about) | Source and drawing rule | Anchors (cell) | Settle |
| --- | --- | --- | --- | --- | --- | --- |
| 0 | `head` | wound | 23 x 23 | key | `neck` | roll |
| 1 | `neck` | wound | 14 x 16 | key (presets `[4]` only: gate ruling N2) | `top`, `bottom` | flop |
| 2 | `leg_fore` | wound | 10 x 25 | key | `shoulder`, `hoof` | clatter |
| 3 | `head_dead` | wound | as 0 | part 0 with the eye stamp `dead` (LLAMA-01 X eye) and the tongue stamp of key 2 (`gK` / `ggK`) out of the mouth corner | as 0 | (swap target) |
| 4 | `torso_front` | wound | 24 x 20 | rig, LLAMA-01 `IDLE` with `include={"barrel"}`, clipped by a plane at x 36 (normal (-1, 0); decision D-W1) and by a plane through the neck base (54, 37), normal (0.5, -0.87); keeps chest, shoulder, belly front and withers mane | `waist` (back cut centre), `neck` | flop |
| 5 | `torso_rump` | wound | 24 x 20 | rig, `IDLE` with `{"barrel", "tail"}` clipped by a plane at x 36 (normal (1, 0)); keeps rump, haunch, mange streaks and the plume tail | `waist`, `tail` | flop |
| 6 | `leg_hind` | wound | 10 x 26 | rig, `IDLE` near hind leg (`{"near_hind"}`), clipped flat at y 47.5 (normal (0, -1)) | `hip`, `hoof` | clatter |
| 7 | `leg_fore_far` | wound | as 2 | part 2 with the fur ramp one step darker (`L`→`l`, `l`→`m`, `m`→`s`, `s`→`a`) | as 2 | clatter |
| 8 | `leg_hind_far` | wound | as 6 | part 6, same ramp shift | as 6 | clatter |
| 9 | `ear` | wound | 5 x 9 | rig, `IDLE` near ear chain alone (`poly(ear_near)`), torn base on `gore` (2 px `b` with a `B` end) | `base` | flutter |
| 10 | `jaw` | wound | 10 x 6 | rig, `IDLE` jaw capsule plus the `lower` teeth stamp, hinge cut on `gore` | `hinge` | spin, clatter |
| 11 | `tooth` | clean | 3 x 5 | one buck tooth: `KTTK` / `KTtK` / `KTtK` / `.KK.` style, `T` face, `t` shade, `K` outline | centre | clatter (spins fast) |
| 12 | `gland` | molten | 9 x 11 | the ruptured throat sac: an ellipse (rx 3.4, ry 5) with LLAMA-01 frame 8's sac ramp (`Y` core, `O`, `R`) on `molten`, a torn `K`-outlined fur rim on `body` (`m`/`s`), two `b` tears on `gore` | centre | bounce, leaves `decal_molten` |
| 13 | `wool_tuft_a` | clean | 6 x 4 | matted tuft: `l`/`m` with an `s` underside and one `p` strand, `K` outline, ragged edge (the `TUFT` rhythm) | centre | flutter |
| 14 | `wool_tuft_b` | clean | 5 x 5 | as 13, a mangy `s`/`p` clump | centre | flutter |
| 15 | `gib_flesh_a` | gib | 5 x 4 | `l`/`m` wool chunk with `K` outline; 2 px meat side on `gore` | centre | bounce |
| 16 | `gib_flesh_b` | gib | 4 x 3 | `s`/`p` mangy hide over meat; meat side on `gore` | centre | bounce |
| 17 | `gib_bone` | gib | 4 x 2 | `e` bone with `K` outline (body layer); preset 4 only | centre | clatter |
| 18 | `gib_rib` | gib | 7 x 3 | a curved rib: 1 px `e` arc with `K` outline, a `b` end on `gore`; preset 4 only | centre | clatter |
| 19 | `drop_blood` | blood | 2 x 2 | GOB-02 part 16 bitmap, key `R`→`b` (same hex) | centre | dies on contact → `decal_splat_s` |
| 20 | `drop_molten` | molten | 2 x 2 | `Y O` / `R r` | centre | dies on contact → `decal_molten` |
| 21 to 29 | `decal_splat_s`, `decal_splat_m`, `decal_pool_1`, `decal_pool_2`, `decal_pool_3`, `decal_pool_3_r`, `decal_wall_spatter`, `decal_wall_spatter_b`, `decal_smear` | blood | GOB-02 sizes | the GOB-02 parts 17 to 25 bitmaps, key `R`→`b`; `o` maps to the llama `o` (section 3). Same nesting, pair offset and `floor_decal` rules | GOB-02 anchors | static |
| 30 | `decal_molten` | molten | 10 x 2 | flat lava puddle: `r` rim, `R` body, 2 `O`, 1 `Y`; floor decal (4 px height cap) | centre-bottom | static; swaps to 31 after its ttl |
| 31 | `decal_molten_crust` | molten | 10 x 2 | 30 cooled: `a` crust with 2 `r` cracks | centre-bottom | static |

**Manifest metadata (per part).** Same schema as `assets/manifests/goblin_a_death_parts.json` (`death_parts.parts[]`: `id`, `frame`, `cell`, `pivot`, `anchors`, `frame0_origin`, `class`, `material`, `presets`, `gore_layer`, `mass`, `spin`, `settle`, `bounce`, optional `swap_on_first_contact`, `on_contact`, `floor_decal`, `span`, `pair`, `pair_offset`, `stain`, `trail`, `ttl`). Additive fields: `molten_layer` (bool) and, on `decal_molten`, `swap_on_ttl` (`decal_molten_crust`, after 240 ticks). The manifests also carry `preset_layer: "gore"`, `always_shown_layers: ["body", "molten"]` (both manifests) and, on the parts manifest, `filter_by` (the spawner filters by each part's `presets` list, never by class name). Materials: `llama_flesh`, `wool`, `bone`, `tooth`, `molten`, `blood`. `frame0_origin` is the cell pivot's position on LLAMA-01 frame 0 (the keys: head (64, 19), neck (56, 30), leg_fore (52, 59)). The design stage writes the three key entries (`scripts/author_llama02_death.py keys`); the executor appends the rest.

**Never hide Carl (renderer rules, unchanged from GOB-02):** launched parts on the debris layer behind player actors; floor decals under every actor; wall spatter on `backwall_decal` only; no launched part larger than 28 x 28 (tested on the keys); floor decals up to 40 x 4; at rest a part lies on the walking line, never stacked higher than 12 px; blood and molten never land on actors. The slam and broken-heap legs (up to 40 px tall) are the pose itself, not debris; they never overlap a player because a defeated enemy keeps its own position.

## 8. Timing hints for the cosmetic spawner (60 Hz, preset 4, scale by section 5)

Presentation hints for PR C/D, measured from the authoritative `EnemyDefeated` tick. Speeds in px/s at 1x; gravity 1050 px/s². "Away" means away from `hit_origin`.

**Crush (D-04, landing tick L):**

| Tick | Event |
| --- | --- |
| L | pose 0 (3 ticks), ENV-02 block impact dust; **10 to 14 `drop_molten` from `gland`**: up 220 to 320, spread ±15°, each landing becomes `decal_molten` (at most 6); `tooth` from `mouth`: away 60 to 120, up 180 to 240, spin 900 °/s |
| L | 18 to 24 `drop_blood` from the two squeeze points (8 to 10 each) and `mouth` (2 to 4), low angles (10 to 35°), 80 to 160; landed drops become `decal_splat_s` (at most 8 per death) |
| L | 2 `wool_tuft_*` (flutter), 2 `gib_flesh_a/b` and 1 `gib_bone` squeezed out sideways from the squeeze points, 60 to 120, low |
| L+3 | pose 1 (3 ticks) |
| L+4 | `decal_splat_m` at both squeeze points; `decal_pool_1` at the pose 2 `pool_origin` |
| L+6 | pose 2, hold |
| L+8 | `decal_wall_spatter` (a or b, seeded, randomly mirrored) at block-base height when a wall face is within 48 px |
| L+20 / L+60 | pool stage 2 / stage 3 |
| L+240 | every `decal_molten` of this death swaps to `decal_molten_crust` |

**Decapitation (PR D, blade tick D):**

| Tick | Event |
| --- | --- |
| D | pose 3 (4 ticks); `head` at the pose's `neck` (head anchor `neck` on it), start rotation -25° (muzzle up, matching the `hurt` throw), away 50 to 90, up 240 to 300, spin 600 °/s backward, 2 bounces (0.3), rolls 4 to 10 px; swap to `head_dead` on first floor contact; each bounce `decal_splat_s`; the roll leaves `decal_smear` |
| D+4 | pose 4 (6 ticks); `drop_blood` from `jet_base`, 3 every 3 ticks for 36 ticks, rising 170 to 240, spread ±12°; `drop_molten` from `gland`, 1 every 6 ticks for 24 ticks |
| D+4 | `decal_wall_spatter` behind the jet when a wall face is within 48 px |
| D+10 | pose 5, hold; trickle: 1 `drop_blood` every 8 ticks from `neck` for 40 ticks |
| D+14 / D+40 / D+100 | pool stages 1 / 2 / 3 at the pose 5 `pool_origin` |

**Slam (heavy lethal hit, tick S):**

| Tick | Event |
| --- | --- |
| S | pose 6 (6 ticks); impact dust at `hit_origin`; `tooth` from `mouth` (up 200 to 260, away 60 to 100, spin 900 °/s); 8 to 12 `drop_blood` from `mouth` rising 120 to 200; 1 `drop_molten` from `gland`; 3 `wool_tuft_*` from `hit_origin` |
| S+6 | pose 7, hold |
| S+8 / S+30 | pool stages 1 / 2 at the pose 7 `pool_origin` (no stage 3: the slam is less bloody than the crush, by design) |

**Explosive breakup (PR D, earned explosives only):** from `impact_position`, every part in the preset's list at its `frame0_origin`; radial speed 160 to 300 (heavy parts −30 %), up-bias 60 to 120. Then 10 gibs, 2 `gib_rib`, 28 blood drops, 8 molten drops, both wall-spatter variants and pools under the two heaviest pieces; `gland` leaves a `decal_molten` where it settles. Settled pieces lie irregularly (seeded rotations, piled and overlapping), never in an even row.

Seeded by `hash(run_seed, event_id)` outside the domain. No visual element feeds back into combat.

## 9. Executor checklist (no design decisions needed)

General rules: draw through the rig in `scripts/author_llama02_death.py` (`beast()` with `include` and `clips`, `cut_face`, `stroke`, `splat_row`) or literal rows; keep the palette and the layer rule; keys stay byte-identical (the fixtures); do not resample or rotate.

1. **Pose frame 1 (`crush_contact` flatten).** From `CRUSH`: rump, chest, haunch, shoulder and belly y +2, `barrel_rr` (8.5, 9.0); the stone clip rect down to row 55 and the `K` underside on row 55 (`STONE_ROW` 55 for this frame only); the near eye stamp's `K` pupil becomes `e` (popped); the geyser stroke 4 px taller (top point (91.5, 35)) and 0.6 px wider, its globs moved up 4 px; each squeeze fan 2 px further out with one more droplet pair. Anchors: `hit_origin` (42, 55); the others unchanged.
2. **Pose frame 4 (`headless_collapse` buckle).** LLAMA-01 in-between `POSE_12` (fore knees folded) with `include` minus `head`, cut by the plane (64.5, 36.5), normal (0.6, -0.8); `cut_face` on it; the jet becomes a 10 px column bent back (stroke from (64, 35) toward (-0.6, -0.8), widths (4.5, 2)) with two gobbets; the duct dot on `molten` at the front of the cut. Anchors `neck` (64, 36), `jet_base` (64, 35), `gland` (66, 36).
3. **Parts 3 to 31** per the section 7 table, in that order.
4. **Manifests:** extend `PART_META` and `part_entries` for parts 3 to 31 (anchors measured from the drawn pixels; `frame0_origin` for parts that exist on LLAMA-01 frame 0; `floor_decal`, `pair`, `pair_offset`, `span` as GOB-02); add `death_poses.spawner_hints` (section 8, GOB-02 shape plus a `slam` block); anchors for frames 1 and 4.
5. **Configs and fixtures:** add the new part tags to `llama_death_parts.pixelart.json` and extend the part-cap rule to them (floor decals 40 x 4); set `unique_frames: true` on the poses config once frames 1 and 4 are drawn. Do not re-freeze the keys.
6. **Export** both atlases (`bash scripts/export_assets.sh`).
7. **Evidence:** per-preset sheets for every pose and part (presets 4, 3, 2, 1), a parts anchor sheet at x4, a frame-anchor-timing diagram, storyboards of the crush, decapitation and slam timelines (section 8) as stills on lit Floor 1, a crush-sequence GIF at x1 and x3 (poses 0, 1, 2, pool stages, molten crusting), an irregular explosion settle, Carl standing in the debris (cast drawn last), `SHA256SUMS`.
8. **Tests:** extend `tests/test_art_llama02_death.py` (new parts: sizes, layer rule, `head_dead` = `head` outside the eye and tongue, far legs = ramp-shifted near legs, molten parts carry no blood key, decal nesting).

Design problems found while executing go back to the design stage. Do not re-decide them.

## 10. Acceptance targets and validators

Automated (`python3 -m scripts.pixelart validate` on both configs, plus `tests/test_art_llama02_death.py`):

- **D1 Crush contact.** Nothing of the llama above row 52 inside the stone footprint (x 14 to 75); a straight `K` underside of at least 30 px on row 52; the geyser rises above row 40 with at least 8 `Y`; at least 5 `e` (the popped eye); at least 4 `T`.
- **D2 Remains.** Top row 44 or lower, at least 14 px tall; at least 4 `T`.
- **D3 Decap.** The jet reaches row 4 or higher with at least 40 `b`; LLAMA-01 frame 11's body unchanged from row 30 down; no `e`, `T`, `t` on `body`.
- **D5 Headless.** The propped stump: top row between 38 and 48 (validator `D5-headless-stump-propped-body-low`), width at most 88; LLAMA-01 frame 14 unchanged for x ≤ 48.
- **D6 Slam / broken heap.** Top row between 28 and 36 (legs in the air); at least 4 `T`.
- **Layer rule.** `B`/`b`/`P` on `gore` only; `gore` ⊆ {`o`, `B`, `b`, `P`, `e`}; `molten` ⊆ {`K`, `a`, `r`, `R`, `O`, `Y`}; `body` alone is closed; 0 isolated pixels, at most 9 orphans per frame.
- **Presets.** Preset 1 drops `gore` and keeps `molten` unchanged; preset 2 has no `b` or `P`.
- **Parts.** Launched parts at most 28 x 28 inside the cell with a 1 px margin; the parts manifest carries every GOB-02 `death_parts` field.
- **Palette.** 20 colours; the 17 llama keys and `B`/`b`/`P` match their source hex.
- **Frozen keys.** Poses 0, 2, 3, 5, 6, 7 and parts 0, 1, 2 (`tests/fixtures/llama-02-death_keys.json`, `llama-02-death-parts_keys.json`).

Review (human eye, at 1x on Floor 1 brick, [scale sheet](../evidence/llama-02-death/silhouette_scale_1x.png) and [scene](../evidence/llama-02-death/scene_640x360.png)):

- Every pose reads as *that llama* at 1x next to Carl (52 px), Donut (40 px) and the goblin (36 px): neck, ears, teeth or glow visible.
- The crush reads as **squashed under the stone** (only head, legs and tail out) with the geyser as the brightest thing on screen for 6 ticks; the beheading reads as **beheaded** (long empty neck, jet); the slam reads as **broken** (on its back, neck snapped), not as asleep.
- Gore reads as a few clean shapes, not noise, at 1x and at 1080p (3x); molten never reads as blood and blood never reads as molten.
- Preset 1 reads as complete sprites with the same silhouettes and the molten signature.
- No part or decal reads as a Carl or Donut colour shape; nothing lands on a player.

## 11. Evidence (design stage)

All in [docs/evidence/llama-02-death/](../evidence/llama-02-death/README.md), written by `python3 scripts/author_llama02_death.py scale` and the toolkit:

- `keys_x4_presets.png`: every key at 4x at presets 4, 2 and 1 (rows), plus a 1x strip at presets 4 and 1.
- `silhouette_scale_1x.png` / `silhouette_scale_4x.png`: two lit Floor 1 views at 1x (and their walking-line bands at 4x): Donut, Carl, GOB-01 A idle, LLAMA-01 idle, the crush under the stone, the remains at the landed stone, the decapitation; then the headless hold, slam, broken heap and the three part keys.
- `scene_640x360.png` / `scene_1920x1080.png`: the in-scene composition as the runtime draws it (enemies mirrored to face Carl), Carl and Donut unobstructed, at 1x and at 3x (1080p).
- `poses/`, `parts/`: toolkit contact sheets at x1 and x4; the per-tag pose GIFs at x1 and x3 are rewritten with an exact palette.
- Executor pass (written by [`scripts/llama02_death_evidence.py`](../../scripts/llama02_death_evidence.py), see the evidence README): `preset_poses_x4.png`, `preset_parts_x3.png`, `parts_anchors_x4.png`, `frame_anchor_timing.png`, the crush, decapitation and slam storyboards, `preset_strip.png`, `scenes_1x.png`, `scene_carl_in_debris.png`, `scene_headless_x3.png`, three full-sequence GIFs at x1 and x3, `review-pack/`.

## 12. Files

| File | What |
| --- | --- |
| `scripts/author_llama02_death.py` | authoring record: palette, pose dicts on the LLAMA-01 rig, cut and gore rules, the 32 parts, manifests, `keys [--check]`, `scale` |
| `scripts/llama02_death_evidence.py` | executor evidence: preset sheets, anchor and timing sheets, storyboards, scenes, exact-palette GIFs |
| `assets/source/characters/llama/death/llama_death_h64.grid.json` / `.ase` | poses source |
| `assets/source/characters/llama/death/llama_death_parts.grid.json` / `.ase` | parts source |
| `assets/source/characters/llama/death/*.pixelart.json` | toolkit configs (validators, frozen keys) |
| `tests/fixtures/llama-02-death_keys.json`, `llama-02-death-parts_keys.json` | frozen keys |
| `assets/manifests/llama_death_h64.json`, `llama_death_parts.json` | manifests: `death_poses` (anchors per frame), `death_parts` (key parts, GOB-02 schema) |
| `tests/test_art_llama02_death.py` | spec rules beyond the toolkit |
| `docs/evidence/llama-02-death/` | evidence (design stage and executor pass) |

## 13. Gate rulings (resolved) and remaining open items

The silhouette gate (an independent top-tier check of the frozen keys) passed. Its rulings, recorded here and applied in the data:

1. **Stone over the crushed llama on the contact ticks: accepted, driven by data.** The llama is 96 px long and the stone's footprint 64 px; behind the llama the stone floats, over it the stone reads as a crush. DEATH-C must:
   - (a) read `hazard_draw: "over"` from the `crush_contact` frames 0 and 1 (`llama_death_h64.json`, also the `death_poses.hazard_draw` block), never species code; when the flag is absent keep the goblin order (stone behind);
   - (b) draw the stone directly above the crushed enemy and still below every player actor, never over Carl or Donut;
   - (c) place the block's anchor on the mirrored `hit_origin` of each contact frame; frame 1 moves it to (42, 55), a 3 px sink that is a **render-only offset, never in the domain block position or the replay hash**;
   - (d) from `crushed_remains` on, return to the goblin order (the remains in front of the foot of `loose_block_landed` on the actors layer);
   - (e) mirror the footprint with the pose (x' = 88 - x).
2. **Molten at preset 1: stays visible** (it is lava, the species and cause feedback, the same class of pixel as the visible spit projectile). The data-only switch to hide it stays available for a later accessibility review (`always_shown_layers`), never species code.
3. **Slam pair: kept as art pending owner gameplay signoff.** It is not wired by default: routine lethal hits keep LLAMA-01 `down`, and no spawner path selects the slam until the owner approves the `force_tier: Heavy` rule. The spawner hints include a `slam` block.
4. **Darker blood accepted.** The deepest blood reuses the llama mouth `o` (`#4a1020`), one step darker than the goblin's `#5c1426`; the decals share bitmaps and keys and take the hex from each species' palette. Identical hex across species is not required.

Non-blocking gate notes, resolved or carried:

- **N1 (headless hold identity)** resolved in the design follow-up below: key 5 redrawn with the stump propped up, re-frozen.
- **N2** fixed: `neck` presets are `[4]`, and the section 5 preset-3 explosion list matches the data (head, torso_front, torso_rump, leg_fore, leg_hind, gland).
- **N3** fixed: `preset_layer` and `always_shown_layers` are in both manifests.
- **N4 (geyser brightness)** checked in the crush storyboard on lit brick: the 10 to 14 `drop_molten` keep the burst brightest for the contact ticks.
- **N5** the spawner filters parts by the per-part `presets` list (`filter_by` in the parts manifest); blood stains and the head trail are absent from preset 1 (`head` stain and trail are blood decals, which the preset 1 class filter drops).
- **N6** the slam and broken-heap GIFs show the change from flung to stiff legs reads over the 6-tick contact.

## Execution record

Executed by the medium tier per section 9 (no design decision was changed; the keys are byte-identical, `python3 -m scripts.pixelart validate` V09 passes on both configs).

- **Poses.** Frame 1 (`crush_flatten`, `crush_contact(flat=True)`): barrel, haunch, shoulder and belly 2 px lower, `barrel_rr` (8.5, 9.0), stone clip and `K` underside on row 55, the popped eye with a bone-white pupil, the geyser 4 px taller and 0.6 px wider with its globs 4 px higher, each squeeze fan pushed out with one more droplet pair (the back fan already touches the canvas edge, so its extra pair goes higher). Frame 4 (`headless_buckle`): LLAMA-01 `POSE_12` without the head, cut by the plane (64.5, 36.5), a 10 px jet bent back with two gobbets, the duct dot on `molten`. Both frames stay on row 71.
- **Parts 3 to 31** are built in the section 7 order: rig parts through `beast()` with clips and `cut_face`, the far legs as the ramp-shifted near legs, literal bitmaps for tooth, tufts, gibs and drops, the sac as a ragged fur rim around a molten ellipse, the blood decals as the GOB-02 bitmaps with `R` mapped to `b`, and the two molten decals. Frame-0 origins come from the cell shifts; anchors are measured from drawn pixels or cut points; every anchor lands on a drawn pixel (tested, except the cell-pivot `centre` anchors, which lie inside the bitmap).
- **Data.** `PART_META` presets follow the section 5 table (neck `[4]`, far legs `[4]`, ear and jaw `[4]`, bone and rib gibs `[4]`); `hazard_draw`, `spawner_hints` (crush, decapitation, slam, explosive), tick windows and anchors for every frame are in the manifests.
- **One data adjustment to a section 7 number.** The waist cut plane is at x 36, not x 41. With the plane at 41 the rump (plume tail to the cut) is 33 px wide, which cannot fit a 32 px cell, let alone the 28 px launched-part cap; at 36 the rump is 28 px and `torso_front` is about 24 px (the section 7 size). It is one constant (`WAIST_X`). *Ratified in the design follow-up (decision D-W1).*
- **Design observation (gate note N1), resolved by the design follow-up.** `headless_collapse` on lit brick at 1x ([`scene_headless_x3.png`](../evidence/llama-02-death/scene_headless_x3.png), [`silhouette_scale_1x.png`](../evidence/llama-02-death/silhouette_scale_1x.png)) still reads as a headless carcass with a stump and a pool, but the long-neck cue is weak because the stump lies nearly level with the back. The decapitation sequence gives it context. If the review judges it "generic carcass", the design-stage remedy is to prop the stump higher (top row 48 or lower) before frame 5 is redrawn; it was not changed here because frame 5 is a frozen key.
- **Evidence** is regenerated (see the evidence README), the full-sequence GIFs use an exact palette (no quantisation), and a test checks that every GIF frame keeps the wool colour and the blood red.

### Design follow-up (2026-11-19)

Two items came back to the design stage after the executor pass.

- **D-N1, `headless_collapse` re-keyed.** Key 5 is redrawn through the rig with the neck stump propped up (section 6): neck chain (57, 61), (62, 52.5), (67.5, 42.5), cut plane (66.5, 44.5) / (0.4, -0.92), stump top row 42. The body behind x 48 stays LLAMA-01 frame 14 (tested); the throat-side blood run, the 30 px pool and the molten duct dot follow the new stump. Anchors are on drawn pixels: `neck` (66, 44), `gland` (69, 46), `pool_origin` (76, 71). The bbox rule becomes `D5-headless-stump-propped-body-low` (top 38 to 48). Only key 5 changed; it was re-frozen with `python3 -m scripts.pixelart freeze --force` (the fixture's other five pose hashes and the three part hashes are unchanged). Frame 4 (buckle) is unchanged: its stump at row 36 now leads into the propped stump of the hold instead of dropping to the back line.
- **D-W1, waist split ratified at x 36.** The executor's `WAIST_X` = 36 stays. `torso_rump` (rump, haunch, mange and the plume tail) is 28 px, exactly the launched-part cap; `torso_front` (chest, shoulder, withers mane, belly front) is about 24 px. An alternative at x 38 would give a 30 px rump and break the cap; a split nearer the shoulder would leave the front torso a thin slice with no barrel, which reads as a generic chunk. The 28/24 split keeps both halves recognisable (the plume tail on one, the withers and neck base on the other). Section 7 rows 4 and 5 now say x 36.

Affected evidence regenerated through the scripts (design stills, toolkit contact sheets and GIFs, executor stills and sequence GIFs, review pack, `SHA256SUMS`). Design follow-up by the design tier (Rights/Provenance below).

## 14. Rights and provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope; no public reuse licence; no game or asset-pack distribution right. Every body pixel is rasterized by the LLAMA-01 parametric rig from pose dicts and stamps in `scripts/author_llama02_death.py` (or `scripts/author_llama01.py`); gore and molten pixels come from the shape rules and literal rows in the same script. Nothing was traced or sampled from concept boards, the Death Lab or any image-model output.

### Rights/Provenance

- Design, spec and hero keys (poses 0, 2, 3, 5, 6, 7; parts 0, 1, 2), manifests for the keys and the design-stage evidence: Claude Opus.
- Design follow-up (key 5 re-keyed, waist split ratified, evidence regenerated): Claude Opus.
- Execution (section 9): in-between frames 1 and 4, parts 3 to 31, full manifest metadata and spawner hints, configs, tests, export and evidence: Claude Sonnet.
- Independent review: pending, separate top-tier agent.
