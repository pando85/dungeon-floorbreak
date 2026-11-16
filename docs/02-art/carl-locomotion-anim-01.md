# ANIM-CARL-01: Carl locomotion candidate (52 px, Book 1)

> **Round 2 in progress:** the run (frames 4-9) was redesigned after the independent Opus review. Symmetric 24 px steps, 8 px ground travel per frame, 60 ms frames, bent-elbow arm pump. The run numbers, timing and slip figures below are from round 1 and are superseded by the [run spec](carl-locomotion-anim-01-run-spec.md) until the execution pass updates this page and the evidence.

**Status:** draft, awaiting owner art review. Not accepted art and not wired into any Bevy binary. The approved master (`carl_h52.grid.json` / `.ase`, manifest `carl_book1_h52.json`), the Character Playground and the Floor 1 scene are unchanged and keep using the 36 x 56 idle atlas. Rights basis for publishing: owner-reported `ART-PUBLICATION-ALL-V1` ([rights register](../04-production/rights-and-release.md)); no reuse licence, no game or asset-pack distribution right.

Authoring and the artistic self-review were done by Claude Opus (AGENTS.md "Model routing for art work"). The preview script and the contract tests are tooling and were written by Claude Sonnet. An independent Opus art review is a separate step.

## Files

| File | Role |
| --- | --- |
| `assets/source/characters/carl/carl_h52_locomotion.grid.json` | pixel-grid-v1 source of truth, 16 frames, 11 layers (same names and order as the master), 23-colour master palette |
| `assets/source/characters/carl/carl_h52_locomotion.ase` | editable Aseprite/LibreSprite source built deterministically from the grid, with tags and per-frame durations |
| `assets/manifests/carl_book1_h52_locomotion.json` | candidate manifest, id `carl_book1_h52_locomotion_candidate_v1`, with a `locomotion` block (grounded / airborne tags, playback mode per tag) |
| `tests/test_art_carl_locomotion.py` | contract tests (idle pixel identity, contact rows, tags, palette, hair/head identity, no clipping) plus negative tests |
| `scripts/make_carl_locomotion_previews.py` | contact sheets, GIFs, lit Floor 1 compositions, `measurements.json` |
| [`docs/evidence/carl-locomotion-anim-01/`](../evidence/carl-locomotion-anim-01/README.md) | committed review renders with SHA-256 sums |

## Frame size and pivot

- **Frame 48 x 56, pivot (25, 55).** The master idle frame is 36 x 56 with pivot (19, 55). Run strides and arm swings do not fit 36 px, so every frame is the master canvas padded 6 px left and 6 px right. The pivot keeps the same foot-relative position: x 19 + 6 = 25, still the midpoint between the idle Croc soles, and y 55 is still the contact row.
- **Grounded tags** (`idle`, `run`, `jump_start`, `land`): the lowest opaque row of every frame is row 55 (0 px foot sink, as in the Floor 1 contract).
- **Airborne tags** (`jump_rise`, `jump_fall`): the pivot stays the physics foot line (bottom of the AABB). The feet are drawn above it (lowest rows 53 and 54) because the legs tuck or reach. These frames must not be snapped to the floor.
- Opaque height: idle 52 (unchanged), run 50 to 52 (the body dips 1 to 2 px on contact and down frames), crouch 48, takeoff and rise 53 (a raised fist), land impact 46.

## Frames and timing

| # | Tag | Frame | ms | Notes |
| --- | --- | --- | --- | --- |
| 0-3 | `idle` | master idle 0-3 | 420 / 260 / 560 / 260 | pixel-identical to the approved master after padding (tested) |
| 4 | `run` | contact, near leg | 70 | heel strike, far leg trailing, far fist forward; hips 1 px lower than idle |
| 5 | `run` | down, near leg | 70 | foot flat under the hip, knee bent, far heel kicked up; hips 2 px lower |
| 6 | `run` | push / pass, near leg | 70 | near toe pushes off behind, far knee passes forward; hips back at idle height |
| 7 | `run` | contact, far leg | 70 | mirror beat of 4 (drawn, not flipped) |
| 8 | `run` | down, far leg | 70 | mirror beat of 5 |
| 9 | `run` | push / pass, far leg | 70 | mirror beat of 6 |
| 10 | `jump_start` | anticipation crouch | 50 | knees bent, both arms swung back, head dropped |
| 11 | `jump_start` | takeoff | 70 | toe-off, far arm thrown up, near arm still trailing (deliberately uncoordinated) |
| 12 | `jump_rise` | rising | 100 | hold while rising; legs tucked unevenly, far arm reaching up |
| 13 | `jump_fall` | falling | 100 | hold while falling; both arms flailing up, legs reaching for the floor, bewildered face |
| 14 | `land` | impact | 80 | deep compression, wince, arms thrown forward for balance; no hand touches the floor |
| 15 | `land` | recovery | 100 | half-way up, then hand over to `idle` or `run` |

Playback modes are in the manifest: `idle` and `run` loop, `jump_start` and `land` play once, `jump_rise` and `jump_fall` hold their single frame.

### Run speed and foot slip (measured)

- The planted Croc's ground contact moves back 5 px per frame while it is on the floor (heel x 24 to 19, toe tip 26 to 21; far foot the same). At 70 ms per frame that is slip-free at about **71 px/s**.
- The greybox `RUN_SPEED` is 185 px/s and the Character Playground uses 145 px/s. At 70 ms per frame the feet slip forward by about 5 px (145 px/s) or 8 px (185 px/s) per frame. `run_scroll_145_x2.gif` and `run_scroll_x2.gif` show it honestly.
- A slip-free run at 185 px/s would need about 27 ms per frame (a 160 ms cycle), which reads as frantic. The stride cannot get longer without crouching the whole run: the legs are about 14 px long and are almost straight at idle.
- **Recommendation for the integration agent:** scale run playback with horizontal speed (frame time = 70 ms x 71 / speed), clamped to no faster than about 45 ms per frame, and accept the remaining slip at full speed. Alternatively, lower the run speed. The choice is a game-feel decision for the owner. The art does not change.

### Jump timing against the greybox physics

With `JUMP_SPEED` 440 and `GRAVITY` 1050 the apex is 92 px after about 420 ms and a full jump lasts about 840 ms. `jump_sequence_x4.gif` plays crouch (on the ground) then takeoff, holds rise while vy > 0, holds fall until touchdown, then plays land and returns to idle. **The anticipation frame must not delay the physics impulse:** play it as the first 50 ms of visual feedback, or skip it when the jump comes from a jump buffer or coyote time. Animation must not change authoritative outcomes.

## How it was made

1. **Master reuse.** The head, hair, torso skin, jacket, smiley pin, boxer seat (with two of the three hearts) and palette are the master's own pixels, only translated per frame. Nothing is rotated or resampled. The face changes in at most 7 pixels, all taken from expressions that already exist in the master idle: an "effort" brow (the far brow lowered as in idle frame 2, mouth still open) for run and jump start, plus a closed mouth for the land wince. The fall frame keeps the master's bewildered face.
2. **Pose block-in.** The legs, sleeves and loose boxer legs were re-pixelled per pose from hip, knee and ankle (or shoulder, elbow and wrist) positions with the master's material ramps: skin `S`/`L`/`M`/`S`/`D` with the lit edge to the upper left, leather `K`/`T`/`j`/`J` with a sheen rim and a bunched `T` cuff, boxers `K`/`W`/`V` with a `U` hem. Crocs (flat, heel strike, toe push, trailing, hanging) and fists are hand-drawn stamps. The scratch block-in tooling is not committed; as with the master, the grid is the source of truth.
3. **Cleanup.** Orphan pixels on the re-pixelled limbs were merged into their neighbouring cluster. Heart pixels and their off-white notches are protected. Lint result: 0 isolated pixels in all frames, 0 to 3 orphan pixels per locomotion frame (mostly 1 px colour steps where a fist meets the boxer outline).
4. **Review rounds at 1x, 3x and 8x** (see the self-review below), each time against the master idle.

## Pipeline conflicts and resolutions

| Conflict | Resolution |
| --- | --- |
| The idle canvas (36 x 56) is too narrow for strides and arm swings, and Bevy reads one fixed frame size per atlas | A separate 48 x 56 candidate atlas, with the idle copied in pixel for pixel, so one atlas serves every tag. Pivot (25, 55) keeps the foot-relative position. The integration agent must switch the actor spec to 48 x 56 / (25, 55) when it adopts this atlas |
| `scripts/check_playground_contract.py` and `scripts/check_floor01_scene_contract.py` pin `carl_book1_h52` to 36 x 56, idle only | Untouched. The candidate has its own manifest, so both contracts and both binaries keep working until review. Updating the pinned contracts is integration work |
| `pixel_grid_to_ase.py lint --strict` requires every frame's bottom row to equal the pivot row, which airborne frames cannot meet | No script change. Non-strict lint still fails on isolated pixels. `tests/test_art_carl_locomotion.py` enforces the per-tag contact rule from the manifest's `grounded_tags` / `airborne_tags` |
| Aseprite tags carry only a direction, not loop / once / hold | A `locomotion.playback` block in the manifest (`validate_assets.py` ignores unknown fields) |
| The greybox run speed (185 px/s) is far above the slip-free speed of a 52 px stocky character | Documented with measurements above. No physics change (out of scope) |
| `make_animation_previews.py` renders every tag at 6x at its atlas durations, but has no physics-timed jump or Floor 1 staging | Added `make_carl_locomotion_previews.py` instead of changing the shared script or `export_assets.sh`. The manifest is picked up by the existing export, validation and per-tag GIF steps without any script change |

## Artistic self-review (Opus, not approval)

**Key-pose problems found at 1x and fixed:**

- The contact heel did not reach row 55: the 14 px leg cannot stretch forward from standing hip height. The hips now drop 1 px on contact and 2 px on down frames, and all grounded frames are verified on row 55.
- Takeoff floated 1 px off the floor. The body was lowered and the toe-off foot re-placed.
- The near arm raised in front of the body covered the face on takeoff and rise. It now trails low on takeoff and stays low on rise, while the far arm (behind the head) does the reaching.
- The passing foot stacked directly on top of the push foot on run frame 9, which read as one pink blob. The passing knee was raised and moved forward, and the push foot moved back.
- Hearts: an automatic cleanup filled a heart's off-white notch, which turned it into a red square. Hearts are now protected, and the third heart (which a moving boxer leg would cut in half) is left out of the moving frames.
- An all-outline 2 px hand disappeared at 1x. The fists are now drawn stamps with skin lights.

**What works:**

- Identity is stable. The face, hair, jacket, pin and palette are the master's pixels in every frame, so nothing shimmers. At 1x he is still the same dark mop, black jacket with a yellow dot, white boxers and pink Crocs.
- The run has distinct contacts, a 1 to 2 px weight drop and opposite arm swing. The forward lean is only 1 to 2 px, and the "effort" scowl with an open, panting mouth makes it a man running for his life, not an athlete.
- The jump sells effort and awkwardness: an unbalanced takeoff, one arm flung up, uneven tuck, a panicked flail on the way down, and a heavy landing with a wince and no superhero three-point pose.

**Known weaknesses (honest):**

1. **The torso and head are rigid.** The jacket, hair and head only translate. There is no jacket-hem flap, hair lag or head tilt, so the upper body reads a little stiff in the run, especially in the 4x GIF. Secondary motion on the jacket hem and a 1 px hair lag on contact are the first polish candidates.
2. **The near arm over the jacket** is dark leather on dark leather. When it swings forward it reads mainly through the fist and the cuff sheen, not the sleeve shape.
3. **The 3/4 body in a lateral run.** The master's three-quarter torso was kept so the approved design is not redrawn. The legs run in profile under a 3/4 chest, and the hips are 6 px apart, so the far foot always lands further forward than the near foot. It reads at gameplay scale but is not a true profile run.
4. **Foot slip at gameplay speed** (see timing above). At 185 px/s the feet slide forward by about 8 px per frame.
5. **The far leg has no separate value from the near leg.** Both use the same skin ramp, and the overlap reads through the outline only.
6. **Two hearts in motion instead of three**, and the watch shows as a single 1 px face on the cuff.
7. Some limb shading is procedural in origin (block-in), so a few knee and shin clusters are simpler than the master's hand-placed leg modelling.
8. No human has judged game feel, speed or personality in motion. Everything here is a candidate.

## Integration notes (for the separate integration agent; not done in this PR)

- Atlas: `generated/art/carl_book1_h52_locomotion.png/.json` from `scripts/export_assets.sh`, unpadded row-major 48 x 56 frames, 16 frames, tags as above.
- Actor spec: width 48, height 56, pivot (25, 55), opaque height 52 (idle). Mirroring for facing left works the same as for idle: flip around the pivot column.
- Pick tags from simulation state only: grounded with |vx| > 0 gives `run`, otherwise `idle`; leaving the ground gives `jump_start` frame 11 then `jump_rise` while vy > 0; vy <= 0 gives `jump_fall`; touchdown gives `land` once, then `idle` or `run`. Rendering never feeds back into the simulation.
- Airborne frames keep the pivot on the AABB bottom; do not apply ground snapping to them.
