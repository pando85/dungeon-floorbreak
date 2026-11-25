# DONUT-MOVES-ART-01: Donut reflex cues and Puddle Jumper, design spec

**Status:** design keys and spec in review. The `evade_twist` and `blink_arrive` hero keys are drawn and passed the silhouette gate (section 12). The remaining frames, the VFX strip, export, validators and the Bevy wiring are execution work (section 10). Nothing is wired into any Bevy binary yet, and nothing here is accepted. Rights basis for publishing: owner-reported `ART-PUBLICATION-ALL-V1` ([rights register](../04-production/rights-and-release.md)); no reuse licence, no game distribution right.

This order replaces two sets of presentation stand-ins with production art:

- The DM-C reflex-evasion cues ([donut-mobility-01 §3.4](../03-tech/donut-mobility-01.md#34-presentation-and-art-stand-ins-in-v1), [§11](../03-tech/donut-mobility-01.md#11-dm-c-implementation-record-presentation-stand-ins-and-evidence)): the tail glint, the tail flick, the `jump_rise` arc with afterimages, and the white streak.
- The DP-D Puddle Jumper cues ([donut-prog-01 §5.4](../03-tech/donut-prog-01-floor01-ability-progression.md#54-presentation-art-routing-and-tone), [§17](../03-tech/donut-prog-01-floor01-ability-progression.md#17-dp-d-implementation-notes-adopted-defaults-2026-11-22)): the pounce-crouch windup, the ripple rings, the dotted streak, and the fizzle ring.

All drawing stays presentation only. It is driven by event ticks and never read by the simulation, the state hash or a golden.

Read with: [Donut bible](donut-character-bible.md), [antihero tone](../01-design/antihero-tone.md), [Donut locomotion run spec](donut-locomotion-anim-01-run-spec.md), [ANIM-DONUT-02 melee spec](donut-melee-anim-02-spec.md), [ANIM-DONUT-03 missile spec](donut-magic-missile-anim-03-spec.md) (its head-swap rule is the model for the tail swap in section 3), [CF02 acting spec](cf02-acting-spec.md) (the `smug` key that `blink_arrive` is built from), [pixel-art pipeline](../07-agents/pixel-art-pipeline.md).

| File | Role |
| --- | --- |
| `scripts/author_donut_moves_keys.py` | Authoring record of the two hero keys. It is deterministic: `--check` fails if the committed grid is stale, and `--sheets <dir>` renders the review sheets |
| `assets/source/characters/donut/moves_keys/donut_h40_moves_keys.grid.json`, `.ase` | Hero keys: frame 0 `evade_twist`, frame 1 `blink_arrive` |
| [`docs/evidence/donut-moves-art-01/`](../evidence/donut-moves-art-01/README.md) | Key sheets at 6x, the lit Floor 1 composition at 1x and 3x, and the silhouette-gate sheet |

Both key frames are `pixel-grid-v1` files with these properties:

- They use the approved Donut locomotion palette unchanged (22 colours, binary alpha).
- They use the approved canvas 72 x 44 and pivot (46, 43).
- They keep row 0 and the first and last columns empty.
- They pass `scripts/pixel_grid_to_ase.py lint --strict`: 0 isolated pixels, 0 orphans, and the lowest row on the pivot row.

`evade_twist` was assembled from the approved `jump_rise` head, face and gear cels (moved down 7 px) plus a newly drawn body, legs and tail. `blink_arrive` was assembled from the frozen CF02 `smug` key (cels moved up 2 px and cut at the waterline) plus new paws and the puddle mouth. No new face, head or costume was drawn.

## 1. Acting brief

Donut dodges like a cat that was never in danger. Every frame follows the bible's behaviour order: cute, expression, anticipation, payoff, back to cute.

- **The ready tell:** the tail tip twitches. It is a show cat's flick of pride, not a power-up.
- **The evade:** a startled-cat hop that turns into a show-off.
  - The back arches high and the head ducks under it.
  - The rear rolls toward the camera, so the cream belly and inner thigh flash.
  - The tail bottle-brushes and hooks over the hump.
  - She lands compact, and the existing `smug` overlay is the payoff.
  - It is **never a flip**: the head never rotates, never goes below the rump, and the feet never pass above the head.
- **The Puddle Jumper:** a self-satisfied hop into nothing and a pop out.
  - She looks down at a puddle that is not there and dips into it nose first. The tail tip is the last thing to go.
  - She rises out of a puddle at the destination with her chin up and her paws tucked, already smug.
  - A fizzle is the indignant version: she half dips, bonks back out, and settles with her ears flat.
  - No aura, no glow on her body, no screen shake, no hero pose (DP-Q34, [tone](../01-design/antihero-tone.md)).

## 2. Frames, pivots and anchoring

| Sheet | Canvas | Pivot | Rule |
| --- | --- | --- | --- |
| Donut moves (`donut_h40_moves`) | 72 x 44 | (46, 43) | Every frame's lowest opaque row is 43. **The airborne evade frames are lowest-row anchored**, like CF02's airborne keys. The presenter applies the cosmetic arc (`donut_cues::arc_lift_px`) as a draw offset, so the art never bakes the lift (DMA-Q01) |
| Puddle Jumper VFX (`vfx_puddle_jumper`) | 48 x 16 | (24, 11) | Row 11 is the puddle surface line. Rings are centred on the pivot column |

- All Donut frames face right. Mirror them around the pivot column for facing left.
- The evade twist is **facing-relative**. The rear always rolls toward the camera, whichever side the attack came from, and the frame mirrors only with her facing (DMA-Q02).
- Layer order, bottom to top: `puddle_back`, `tail`, `legs_far`, `body`, `legs_near`, `head`, `face`, `gear`, `puddle_front`. The two puddle layers exist only in blink frames. The puddle mouth is drawn **inside** the Donut sprite, not in the VFX strip, so that the front rim can cover her body (DMA-Q03).

Key bounding boxes, given as x relative to the pivot and canvas rows:

| Key | x from pivot | Rows | Height | Layers |
| --- | --- | --- | --- | --- |
| `evade_twist` | -43 .. +23 | 2-43 | 42 | tail, legs_far, body, legs_near, head, face, gear |
| `blink_arrive` | -25 .. +23 | 1-43 | 43 | puddle_back, tail, body, legs_near, head, face, gear, puddle_front |

## 3. `tail_ready`: the flick and the glint (replaces the DM-C flick and glint)

**Composition (DMA-Q04): a tail swap baked per locomotion frame**, the same pattern as the missile's head swap.

- Atlas index: `16 * phase + locomotion frame`, with phase 0 = `flick_up` and phase 1 = `flick_snap`. That gives 32 frames. The base locomotion frame is phase "none" and is not duplicated.
- Each frame is the locomotion frame with only the `tail` cel changed.
- The rule is applied per frame, and the executor hand-fixes the outline afterwards:
  - **Tip:** the tip is the 8 tail pixels farthest from the tail root, measured by path distance along the tail mask.
  - **`flick_up`:** the tip rises 2 px and leans 1 px back, away from the facing. Vacated pixels are erased, and the outline is re-closed with `K`.
  - **`flick_snap`:** the tip curls 1 px forward and 1 px down from the base position, and one extra fur pixel (`G`) shows on the curl's outer edge.
- Limits:
  - The root half of the tail is byte-identical to the base frame.
  - At most 24 pixels change per frame.
  - No pixel outside the `tail` layer changes.

| Event | Age (ticks) | Drawn |
| --- | --- | --- |
| `EvadeArmed` | 0-1 | `flick_up` over the current locomotion frame |
| | 2-3 | `flick_snap` |
| | 4-5 | Base locomotion frame (the flick has ended; `FLICK_TICKS` stays 6) |

**The glint (DMA-Q05)** replaces the 2 x 2 square with a 3 x 3 plus from the VFX strip:

- `w` at the centre, `c` on the arms.
- Every 4th on-tick it shrinks to the 1-pixel `twinkle_small` frame.
- The 4-on / 26-off blink and the tail-tip anchor (`analyze_atlas`, `TailTip`) are unchanged. The anchor must also be computed for the tail-swap frames.

## 4. `evade`: arch and twist (replaces the `jump_rise` arc, the afterimages and the white streak)

There are five frames over the 12 evade ticks. The age counts from the `HeroEvaded` tick, and the lift is `arc_lift_px(age)`, unchanged.

| Frame | Name | Ages | Lift (px) | Pose |
| --- | --- | --- | --- | --- |
| e0 | `evade_coil` | 0-1 | 0, 3 | Coiled crouch: the melee `p0` body, compressed 2 px. Back already rounding, ears back, tail low and fat. Head ducked 3 px |
| e1 | `evade_arch` | 2-4 | 6, 8, 9 | The back pushes up into the hump; the head is ducked as in the key. Hind legs gathered under the rump, not yet rolled. Tail straight up and bottle-brushed (no hook yet). Front paws leave the floor |
| e2 | **`evade_twist` (KEY)** | 5-7 | 10, 10, 10 | The frozen key: peak hump, rear rolled toward the camera (cream belly seam, cream inner thigh, toe bean), near hind leg kicked back and tucked, tail hooked forward over the hump |
| e3 | `evade_untwist` | 8-9 | 9, 8 | The rear rolls back: the belly seam narrows to 1-2 px and the inner thigh turns dark. Both hind legs reach down. Tail hook opens to a question mark. The hump drops 3 px |
| e4 | `evade_land` | 10-11 | 6, 3 | Compact reach-down: four paws below the body, close together. The tail is still puffed but lowered. The head comes back up to the `jump_fall` height |

After that comes the existing `smug` overlay, under the pending rule unchanged.

- **Afterimages retired (DMA-Q06).** The five frames carry the motion. Afterimages read as a superhero speed effect, which the bible and the tone rule out. `AFTERIMAGES` stops being drawn, which is presentation only.
- **Swish (DMA-Q07).** The 36 x 2 white streak is replaced by the VFX strip's `swish` pair, drawn on the attacker side at the same place for `STREAK_TICKS` (2): `swish_a` at age 0 and `swish_b` at age 1. It is a thin curved arc in `c` with a `w` head and a `k` underside, the visible path of the attack she slipped. It uses no magenta, because the evade is not magic.

## 5. Puddle Jumper: dip, vanish, arrive, fizzle (replaces the DP-D crouch, ripple, streak and fizzle ring)

The blink timeline is unchanged (DP-Q25):

- Windup: t0..t5.
- Arrival: t6 (`warp_x`; `y` and `vy` are unchanged, so an airborne arrival is possible).
- Recovery: t7..t12.

Ages count from `SpecialStarted`.

| Frame | Name | Ages | Where | Pose |
| --- | --- | --- | --- | --- |
| b0 | `blink_crouch` | 0-2 | origin | Low crouch, looking down at her front paws, ears forward, tail tip up and curled (smug, not tense). The puddle mouth opens under her front paws: `puddle_back` only, a thin `L` rim |
| b1 | `blink_dip` | 3-4 | origin | Nose-first dip: head and shoulders under the front rim, rump and tail up. The puddle is at its full width with its front rim drawn |
| b2 | `blink_vanish` | 5 | origin | Only the tail tip (about 8 px) above the rim, and the puddle. A lilac `L` meniscus where the tail enters |
| a0 | `blink_pop` | 6-7 | destination | Ears, tiara and eyes above the rim; the rest is under. The puddle is at full width |
| a1 | **`blink_arrive` (KEY)** | 8-10 | destination | The frozen key: risen to the chest, smug eyes, chin up, paws tucked, meniscus, two splash drops |
| a2 | `blink_settle` | 11-12 | destination | Out of the puddle and sitting (the `smug` body, ground contact restored). The puddle mouth shrinks to a 2-row lilac and magenta ring, half of its width |

Locomotion resumes on t13. Other events:

- **`BlinkCancelled`:** no new frame. Whatever owns her sprite (hurt, knockback) takes over at once, and the origin puddle plays the VFX `close` frames.
- **`BlinkFizzled`** (DMA-Q08). It fires at the origin; she never left. Ages count from the fizzle tick, over `FIZZLE_TICKS` (8):
  - f0 `fizzle_bonk`, ages 0-3: half dip bounced back. Head out, ears flat, the CF02 `startle_puff` face, one paw on the rim. The puddle is half width.
  - f1 `fizzle_huff`, ages 4-7: sitting, ears flat, the CF02 `startle_settle` head, tail lashed to one side. No puddle; the VFX `fizzle` frames draw the drying ring.
- **Airborne arrival (DMA-Q09).** The puddle mouth stays at her feet in the air, inside the sprite. It is a hole in space, not a floor puddle. The splash drops still fall.

The VFX strip `vfx_puddle_jumper` is drawn in the Magic Missile palette unchanged: `k d m p c w v l`, binary alpha.

| Frames | Name | Use and ticks |
| --- | --- | --- |
| 0-3 | `ring_open_0..3` | Replaces the 3-ring ripple at both ends: 3 ticks per frame from `SpecialStarted` (origin) and from `BlinkResolved` (destination). Concentric flat ellipses with half widths 6, 10, 14 and 18, ratio 1:4. The outer ring is `m` over `k`, the inner ring `p`, and the newest ring `c`. Frame 3 is the outer ring only, broken into dashes |
| 4-6 | `ring_close_0..2` | The origin after she has left (ages 6-14): the ring pulls in (half widths 14, 8, 3) with two `l` drops above it |
| 7 | `streak_dash` | A tileable 16 x 3 dash (`p` core, `d` under, `c` 2 px head) for the origin-to-destination streak. It is tiled along the path for `dpq34_streak_frames` (1 tick), replacing the dotted 2 x 1 dots |
| 8 | `streak_head` | The arrival end of the streak: a 4 x 3 `c`/`w` drop |
| 9-11 | `fizzle_0..2` | A drying ring: half widths 18, 10, 4, then a single `v` drip. 3, 3 and 2 ticks (8 in all) |
| 12, 13 | `glint`, `twinkle_small` | The ready glint (section 3) |
| 14, 15 | `swish_a`, `swish_b` | The evade swish (section 4) |

## 6. Stand-in replacement map

| Stand-in (code) | Replaced by |
| --- | --- |
| `donut_cues` glint, 2 x 2 (`GLINT_SIZE_PX`) | VFX 12 and 13 at the tail tip |
| `donut_cues` flick, tip shifted 1 px | Donut moves tail swap, phases 0 and 1 (section 3) |
| `donut_cues` evade: `jump_rise` sprite plus `arc_lift_px` | Evade frames e0-e4 plus `arc_lift_px` |
| `donut_cues` `AFTERIMAGES` | Retired (DMA-Q06) |
| `donut_cues` white streak (`STREAK_*`) | VFX 14 and 15 |
| `donut_art::blink_crouch_frame` (pounce `p0`, 6 ticks) | b0, b1, b2 (6 ticks) |
| `blink_cues` 3-ring ripple (`RING_*`) | VFX 0-3 at the origin and destination, plus the in-sprite puddle mouth |
| `blink_cues` dotted streak (`STREAK_DOT_*`) | VFX 7 and 8 |
| `blink_cues` fizzle ring (`FIZZLE_*`) | f0 and f1 plus VFX 9-11 |
| (none: she reappears in her locomotion frame) | a0, a1, a2 at the destination |

Timing constants that are DM or DP question values are not changed: `FLICK_TICKS`, `EVADE_TICKS`, `ARC_PEAK_PX`, `STREAK_TICKS`, the glint blink, the DP-Q25 phases, `dpq34_streak_frames` and `FIZZLE_TICKS`. Only drawing constants that become dead are retired.

## 7. Palette

| Sheet | Palette | Notes |
| --- | --- | --- |
| Donut moves | The Donut locomotion palette, 22 colours, unchanged | The puddle mouth uses only the Donut colours it shares with the magic VFX: `M` gem_magenta (lip), `V` collar_violet (bank), `L` tiara_lilac (rim, meniscus, splash), `D` coat_dark_shadow (water), `K` outline_plum (bottom edge). Accents (`w`, `N`, `M`, `P`) are exempt from the orphan rule, as before |
| VFX | The Magic Missile palette, 8 colours, unchanged | No new colour. No partial alpha. No additive glow in the presenter |

## 8. Difficulty score ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty))

| Point | Result | Note |
| --- | --- | --- |
| 1. New design, not a variant | yes | The arch-and-twist and the puddle mouth are new poses |
| 2. Native height above about 40 px | no | 40 px body |
| 3. Complex anatomy | yes | A twisted cat body in the air; the face is reused from approved cels |
| 4. VFX, projectiles or special states | yes | The puddle mouth and the ring strip |

**Score 3 of 4: Hard.** The two hard keys are drawn and frozen. Execution draws the remaining frames from them and from approved cels, by the rules above. A new design problem found during execution goes back to design.

## 9. Acceptance targets

1. **Lint:** every new Donut frame passes `lint --strict` (0 isolated, 0 orphans, lowest row 43, row 0 and the edge columns empty). The VFX frames have 0 isolated pixels, and orphans are allowed only on accents and mote pixels.
2. **Palette:** the Donut frames use exactly the 22-colour locomotion palette, and the VFX frames exactly the 8-colour missile palette. Alpha is binary.
3. **Frozen keys:** frames e2 and a1 of the production sheet are byte-identical to the two key frames in `moves_keys` (frozen-key fixture, SHA-256).
4. **Never a flip:** in every evade frame the head cel is an unrotated shift of an approved head, the head's lowest row is above the rump's lowest row, and no paw pixel sits above the tiara.
5. **Silhouette at 1x:**
   - In the lit Floor 1 scene, e1-e3 read as a cat arching and twisting, and differ from `jump_rise`, `pounce_air` and `hurt` (the gate sheet test).
   - a0-a2 read as coming up out of a hole, not as standing on a mat.
   - b1 and b2 read as going down into it.
6. **Size:** evade frames stay within x -44 .. +24 of the pivot and 42 rows. Blink frames stay within -26 .. +25.
7. **Tail swap:** only the `tail` layer differs from the base locomotion frame, at most 24 pixels per frame, with the root half byte-identical.
8. **Timing:** the frame-for-age mappings in sections 3 to 5 are pure functions with unit tests at every boundary age.
9. **Determinism:** no state-hash change and no golden change. All playtrace and render goldens stay byte-identical, except the render fixtures that intentionally show the new cue art, each listed in the PR.
10. **Tone:** no aura, glow, shake, afterimage or hero pose. Smug is the payoff, never a pose held during the move.

## 10. Execution scope

These frames are still to be drawn:

- Evade: e0, e1, e3 and e4.
- Blink: b0, b1, b2, a0 and a2.
- Fizzle: f0 and f1.
- The 32 tail-swap frames.
- The 16 VFX frames.

After drawing come the export, the manifests, the validators, the frozen-key fixture, the Bevy wiring that replaces the section 6 stand-ins, and the evidence:

- GIFs at tick timing.
- Lit scenes at 1x and 3x.
- A 2P scene with both players and an arrival off-screen-centre.
- The tail-swap contact sheet.

## 11. Open notes for execution (non-blocking)

- The silhouette gate suggested tucking the kicked hind paw and lowering the head. Both are applied in the frozen key: the paw is tucked 2 px up and in, and the head is ducked 7 px instead of 6.
- The gate rated the arrive read as weak, because the puddle looked like a mat at 1x. The frozen key now sinks her body behind a deeper front rim and adds a meniscus and two splash drops.
- The pop frame a0 (only the ears and eyes above the rim) carries the "coming out of a hole" read in motion. The independent review should judge the arrive on the a0 → a1 → a2 GIF, not on a1 alone.
- The stubby front paws of the evade key are gathered under the bib. If they disappear at 1x in motion, the review may ask for a 1 px `K` underline. That is an execution fix, not a design change.

## 12. Silhouette gate result

An independent gate looked only at the 1x and 3x silhouette sheet and the lit scenes in the evidence folder.

**Verdict: PASS.** At 1x, `evade_twist` reads as a cat arch-and-twist hop, not a flip and not a generic jump:

- The head stays upright with the face forward, and nothing is inverted.
- `jump_rise` and `pounce_air` are long horizontal ovals. The twist has its height at the back and a hooked tail.
- `hurt` is a compact crouch, so the two cannot be confused.

The gate's non-blocking notes (rear paw, head height, and the weak arrive read) were applied to the frozen keys, as recorded in section 11.

## 13. Rights and provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope ([rights register](../04-production/rights-and-release.md)). The Donut layer is a replaceable fan-concept layer; the engine and data contracts stay IP-neutral.

- The keys are literal pixel edits and rule rasterization in `scripts/author_donut_moves_keys.py`, built on the approved `jump_rise` cels and the frozen CF02 `smug` key.
- No tracing, no image-model raster output, no third-party sprite or animation used as reference.
- Who designed, executed and reviewed is recorded in the PR's Rights/Provenance section.
