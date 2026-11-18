# ANIM-DONUT-03: Princess Donut Magic Missile cast and VFX, design spec

**Status:** keys frozen after the silhouette gate fix (section 14); in-betweens drawn and both atlases exported, validators pass (section 15); independent art review pending; not wired into any Bevy binary. Nothing here is accepted.

Scope: [MVP controller contract](../01-design/mvp-controller-contract.md#character-actions-and-progression). After the Tutorial Guild biscuit, Donut's primary attack (Square/X) becomes Magic Missile, on the same button, and it **stays movement-compatible**: she casts while standing, running or in the air. Her melee swat and pounce are ANIM-DONUT-02 (separate work order, not touched here). Gear after the biscuit is unchanged at Floor 1 ([gear timeline](donut-gear-timeline.md)): crown/tiara with the magenta gem, collar and charm; no new item is drawn.

## 1. Acting brief

A cat casting, not a human wizard: no staff, no wand, no raised paw, no glowing hands. Following the bible's behaviour order (neutral cute, expression change, spell anticipation, payoff, recovery to cute), the whole spell is acted by the **head**:

- **Gather (anticipation):** master ears, the upper lids drop one row into a focused stare, and the tiara gem charges with a small four-point twinkle. Her magic is tied to the crown she already wears.
- **Release (payoff):** the head jabs 1 px forward, the ears flatten out sideways (airplane ears: tips gone, a dark-edged wedge two rows lower and out past the head outline), the lids drop into a half-lidded glare with a raised outer brow pixel (inward slant), and she hisses: a 3 x 2 open mouth in `K` with two white fang points. The head outline itself changes, so the cast reads on the run and air bases. The bolt appears 4 px ahead of her right cheek at mouth height.
- **Follow (recovery):** mouth closes, lids and ears come back up, the head returns; then she is back to the neutral master face.

Grounded tone: a short, irritated hiss, not a heroic pose. The bolt is small and nasty, not a beam.

## 2. Composition with locomotion (decision)

**Decision: head swap, phase-synced to locomotion.** Options weighed:

| Option | Verdict |
| --- | --- |
| Separate full-body grounded, run and air cast clips | Rejected. A full-body run-cast either stops the gallop (paws slide at 185 px/s) or needs a second hand-drawn run cycle that drifts from ANIM-DONUT-01. It also locks the body for the cast, against "movement-compatible". |
| Runtime overlay sprite for the head | Rejected for now. The locomotion atlas is flattened, so the base head would show under a pinned-ear overlay; needs a headless body atlas and a two-sprite actor. |
| **Head swap baked per locomotion frame** | **Chosen.** In every ANIM-DONUT-01 frame the `head`, `face` and `gear` cels are the master's translated by a fixed offset (`HEAD_OFFSETS`, re-measured by a test). The cast replaces only those cels with a cast head translated the same way; `tail`, `legs_far`, `body` and `legs_near` are copied unchanged. |

Runtime rule (suggestion for the integration agent): the locomotion state machine keeps its own clock and frame index `n` (0-15). While a cast phase `p` is active, draw atlas frame `16 * p + n` of `donut_h40_missile` instead of frame `n` of `donut_h40_locomotion`. Same canvas **72 x 44**, same pivot **(46, 43)**, same facing (right; mirror around the pivot). Cast while idle, running, jumping, falling or landing all come for free, and the cast never moves the feet. Cost: 48 frames, of which only 3 head drawings are design work.

## 3. Frames, tags and ticks (60 Hz)

| Atlas frames | Tag | Content | Author |
| --- | --- | --- | --- |
| 0-15 | `cast_gather` | gather head on locomotion frame `n` | 0 key; 1-15 derived by rule |
| 16-31 | `cast_release` | release head (+1 px thrust) on frame `n` | 16, 24, 28 keys; others derived |
| 32-47 | `cast_follow` | follow head on frame `n` | in-betweens: one follow head drawn, composited by the rule |

Cast timeline (cast-local ticks, inclusive): **gather 0-3** (4 ticks, 67 ms), **release tick 4** (projectile spawns), **release 4-9** (6 ticks), **follow 10-15** (6 ticks), back to locomotion at tick 16. **Re-fire placeholder: tick 16** (3.75 casts/s); a press during follow buffers. The grid's `duration_ms` per frame is the locomotion frame's duration rounded to whole 60 Hz ticks (preview only; the runtime timing is the locomotion clock plus the phase table above).

Projectile spawn: bolt pivot at **(+27, -19)** from Donut's pivot on idle (4 px ahead of the right cheek, mouth row, so the streaks never cover the hiss on release ticks 4-5), plus the locomotion frame's head offset; the manifest lists all 16 (`projectile_spawn_offset_by_locomotion_frame`). On a run frame the bolt flies 15-16 px above the floor, on idle 19 px, which hits a 36 px goblin's torso.

## 4. Projectile and impact (VFX)

One grid, `assets/source/vfx/magic_missile/vfx_magic_missile.grid.json`, **32 x 32, pivot (16, 16)**, layer `body`, facing right (mirror with Donut).

| Frames | Tag | Ticks | Content |
| --- | --- | --- | --- |
| 0-3 | `missile_flight` (loop) | 4, 4, 4, 4 | **Bolt**: hot oval head 8 x 7 (pale core, 2 px white leading the centre, pink, magenta, deep-magenta rear rim) trailing **three parallel claw-scratch streaks** (8, 12, 8 px, pink to magenta to deep to violet tip). Bbox 22 x 7. Key 0; 1-3 in-betweens. |
| 4-8 | `missile_impact` (once) | 2, 3, 4, 4, 5 | 4 **flash** (round 10 x 10 disc, 4 white px, 3 spray chunks back toward the caster); 5 **claw-scratch peak**: three parallel slashes raking down-back across the hit point, hot centre lines, 2 white px, violet/magenta/lilac 2 px motes; 6 in-between; 7 **fade**: slashes thinned, broken in the middle, cooled to magenta/deep/violet, motes drifting; 8 in-between. 18 ticks total. |

Travel read: bright leading head, streaks always behind, 6 px per tick against a 22 px sprite, so consecutive positions overlap. The three streaks plus the three-slash impact are the cat signature (a flying scratch); the look is original and IP-neutral, not taken from any game, book cover or tabletop depiction.

Placeholders (gameplay tuning owns them): **speed 6 px/tick (360 px/s)**, **range 240 px (40 ticks)**, straight horizontal along facing, hitbox 8 x 6 at (+4, 0) from the VFX pivot, damage on the contact tick, impact plays at the contact point drawn over the target; at max range the bolt fizzles by playing impact frames 7-8 only. Draw order: projectile in front of Donut and enemies.

## 5. Palette and glow

- Cast: the approved master's 22 keys, unchanged (test). The twinkle and the mouth reuse `w`, `P`, `M`, `K`, `E`, `W`.
- VFX `vfx-missile-v0`, 8 colours: `k` #2a0f2e (reserved rim), `d` #7a1a63, `m` #e2309f (= Donut's gem `M`), `p` #ff7fd0, `c` #ffd8f4, `w` #ffffff (accent), `v` #9a6ee0 (= her collar violet), `l` #dbc6fb (= her tiara lilac). Magenta magic per [art direction](art-direction.md).
- Glow is **baked** (VFX convention of [VFX-01](vfx-01-spec.md) section 5): the A1 pass does not bloom actor-group emitters, so a small hot core and a darker outer ramp carry the glow; white is capped (bolt 2, flash 4, peak 2, fade 0, no core `c` in the fade). Read checked lit and with the A1 pass off (`keys-scale.png`, middle strip, `Floor.render(..., lit=False)`).
- **Pure white `#ffffff`, recorded choice.** VFX-01 uses a warm off-white `#efe3d3` for fire. This spell is cold magenta magic, and its 1-4 px hottest points must stay neutral-to-cool; a warm white would read as fire next to torches. Same value as Donut's catchlight `w`, so the fang points and the bolt core share one white.

## 6. Difficulty score ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty))

| Point | Result | Note |
| --- | --- | --- |
| 1. New design, not a variant | yes | Cast heads are acting on the approved master; the VFX is new |
| 2. Native height above about 40 px | no | 40 px body, 32 px VFX cell |
| 3. Complex anatomy | yes | Face acting on an owner-approved hero head; must stay cute and on-model |
| 4. VFX, projectiles or special states | yes | Projectile and impact |

**Score 3 of 4: Hard.** Route: top tier designs and freezes all keys and the spec (this PR), medium tier draws the in-betweens and execution, independent top-tier review.

## 7. Acceptance targets

1. At 1x on lit Floor 1, gather, release and follow read as different expressions on idle, run and air bases; the face stays Donut (eyes, muzzle, tortoiseshell, tiara unchanged in shape).
2. Body, legs and tail of every cast frame equal the locomotion frame (test `test_cast_never_touches_body_legs_or_tail`).
3. Bolt readable as a small travelling magenta object against warm brick and cold stone, lit and unlit; streaks always trail.
4. Impact reads as three scratches, not a star burst or explosion; it does not hide the goblin's silhouette for more than the 3-tick peak.
5. Validators pass (`python3 -m scripts.pixelart validate` on both configs). VFX orphan limit **9**, the same as VFX-01 and the character sets. Measured on the frozen keys: bolt 5, flash 0, scratch peak 0, fade 0 (`merge_orphans` joins slash anti-aliasing to its stroke; motes are 2 px chunks).
6. White and core caps as in section 5; no frame covers more than 35 % of the VFX cell (test).

## 8. Per-key executor notes

| Key | Notes |
| --- | --- |
| Cast 0 (gather, idle) | Ears: master (outline differs from release on purpose). Lids: face row 16 cleared, row 17 lid. Twinkle: 5 px plus above the gem, gem centre white. Do not add motes (isolated pixels). |
| Cast 16 (release, idle) | Head, face and tiara +1 px x (collar does not move). Ears: rows 8-9 tips cleared; left wedge rows 10-11 from x 37 (`KjEEBB` / `KjEB`), right wedge rows 10-11 to x 66 (`BBEEjK` / `BEjK`). Lids at row 18 with the inner two pixels down at row 19; one `K` brow pixel at row 17 on each outer lid end (x 43, 59); catchlight 1 px at row 19. Mouth: master `cc` removed, rows 23-24 at x 51-53 `wKw` / `KKK`. Gem back to plain with a `P` centre (spent). |
| Cast 24 (release on run frame 8) | Composition key: head offset (+1, +4) plus thrust; check the chin against the near foreleg. |
| Cast 28 (release on jump rise 12) | Composition key: offset (+1, +2); check the head against the tail streaming behind. |
| VFX 0 (bolt) | Rear rim `d` joins the streaks; keep three separate streak rows with empty rows between. |
| VFX 4 (flash) | Round, not spiky; 3 spray chunks only on the caster side. |
| VFX 5 (peak) | Slashes 5 px apart, steep down-back rake; 2 px white at the centre only; every anti-aliasing pixel touches its stroke (orphans 0). |
| VFX 7 (fade) | Each slash broken in the middle; no `c` or `w`. |

## 9. In-between table (no design decisions needed)

| Frames | Was (frozen keys, copies) | Drawn (section 15) |
| --- | --- | --- |
| 32-47 `cast_follow` | copies of 16-31 | Follow head drawn once at frame-0 position and composited by the same rule: thrust kept (+1, 0), mouth closed to the master `cc` line, no fangs, no brow pixels, lids back to one row lowered (gather lids), ears halfway up (tips back at row 9, wedge 1 px out instead of 2), no twinkle, gem normal. Then turn on `unique_frames` and change rule C2 to frames 16-31 only. |
| VFX 1-3 | copies of 0 | Streak lengths cycle (8/12/8, 10/10/7, 7/12/10, 9/11/9), head core shifts 1 px fore/aft, white stays 1-2 px; clean loop seam. |
| VFX 6 | copy of 5 | Between peak and fade: slashes 1 px thinner, centre `c` gone, white gone, motes (2 px chunks) 2 px further out; run `merge_orphans`, orphans at most 9. |
| VFX 8 | copy of 7 | Last motes and two short `d`/`v` slash remnants; nothing hot. |

Derived cast frames 1-15 and 17-31 come from the rule; the executor only fixes local collisions (none seen at review scale) and must not edit the frozen keys. All rows above are done; see section 15 for the rules used and the measured results.

## 10. Evidence

All images are Pillow models composed from the committed grids and the A1 lighting model of `scripts/env_kit_light.py`; none is a Bevy capture. Carl is the approved Carl 52 px idle frame 0, the goblin is GOB-01 goblin A.

| File (under `docs/evidence/anim-donut-03/`) | Shows |
| --- | --- |
| `keys-scale.png`, `keys-scale-4x.png` | Designer's key strips, regenerated unchanged: lit 1x cast keys 0, 16, 24, 28, bolt over brick, peak on a goblin; the same with the A1 pass off; master head vs release head on the run and air bases; 4x copy. |
| `bases_x4.png` | **Enlarged frames:** locomotion base and the three phases on the idle, run-suspension and jump-rise bases at 4x, pivot and walking line marked. |
| `scene_phases_lit_640x360.png`, `_1920x1080.png` | 1x gameplay-scale: gather, release and follow on the three bases side by side on lit Floor 1 brick. |
| `scene_cast_lit_640x360.png`, `_1920x1080.png` | 1x gameplay-scale: Donut releasing next to Carl on lit Floor 1 brick, a fresh bolt at the spawn point, one in flight, the scratch peak on a goblin. |
| `scene_cast_unlit_640x360.png`, `_1920x1080.png` | The same with the A1 pass off: bolt over unlit brick. |
| `cast_sequence_x2.gif` | Cast timeline at 2x on lit Floor 1, one frame per two ticks: gather, release (bolt spawns at tick 4), follow, flight at 6 px/tick, impact on a goblin, idle. |
| `layout_pivot_timing.png` | **Diagram:** frame layout of both atlases, pivot, spawn offset and the tick timeline (cast phases and VFX). |
| `cast/` | Toolkit evidence for `donut_h40_missile`: contact sheets (1x, 4x), per-phase GIFs (1x, 3x), tell/attack strip, lit scenes, `SHA256SUMS`. |
| `vfx/` | The same for `vfx_magic_missile`: contact sheets, flight and impact GIFs, strip, scenes, `SHA256SUMS`. |
| `SHA256SUMS` | Checksums of the files in this folder (not of `cast/` and `vfx/`, which carry their own); verified by `python3 scripts/check_docs.py`. |

Regenerate: `bash scripts/export_assets.sh` once, then `python3 scripts/author_donut_missile.py keys` and `python3 scripts/author_donut_missile.py evidence`; the toolkit folders with `python3 -m scripts.pixelart evidence <config> --out docs/evidence/anim-donut-03/{cast,vfx}`; rebuild the `.ase` files with `python3 scripts/pixel_grid_to_ase.py build <grid> <ase>` and refreeze with `python3 -m scripts.pixelart freeze --force <config>` only on a design change.

## 11. Files

| File | Role |
| --- | --- |
| `scripts/author_donut_missile.py` | authoring record: cast heads (gather, release, follow), composition rule, VFX rules (keys and in-betweens), manifests, evidence (`keys`, `key-scale`, `evidence`) |
| `assets/source/characters/donut/donut_h40_missile.grid.json`, `.ase`, `.pixelart.json` | cast source, Aseprite file, config |
| `assets/source/vfx/magic_missile/vfx_magic_missile.grid.json`, `.ase`, `.pixelart.json` | VFX source, Aseprite file, config |
| `assets/manifests/donut_h40_missile.json`, `assets/manifests/vfx_magic_missile.json` | phase ticks, spawn offsets, placeholders, provenance |
| `tests/fixtures/anim-donut-03_keys.json`, `tests/fixtures/anim-donut-03-vfx_keys.json`, `tests/test_art_donut_missile.py` | frozen keys and spec tests |

## 12. Settled questions (silhouette gate)

- Bristling tail: **not in this order.** It breaks the head-only swap and would cost 48 per-frame tail edits; kept as a horizon idea for a later per-frame tail layer.
- Spawn overlap: **spawn moved 4 px forward** (+27, -19 on idle, +4 on all 16 entries).
- Gather length: **stays 4 ticks.** The twinkle carries the tell and the release ear change adds an outline cue; gameplay may still tune it to 6.

## 13. Rights and provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope (see `docs/04-production/rights-and-release.md`). Spec + keys: Claude Opus. In-betweens + export: Claude Sonnet. Review: pending. The cast heads are literal pixel edits of the approved master head; the VFX keys are rasterized from rules in `scripts/author_donut_missile.py`. No tracing, no image-model raster output, no third-party missile design used as reference.

## 14. Silhouette gate result

Independent gate verdict: **FIX-KEYS, release head only**; fidelity to the master, the bolt, the impact, the palette and the glow passed. Applied:

1. Open hiss: 3 x 2 `K` mouth under the nose, master mouth line removed.
2. Fangs: one `w` at each top corner (2 px).
3. Ears on release: tips removed, wedge 2 rows lower and 1-2 px out past the master outline with a dark outer edge; gather now keeps the master ears so the two phases differ in outline.
4. Glare: one `K` brow pixel per eye on the outer lid end (inward slant).
5. Spawn offset +4 px in x on all 16 entries.
6. Scratch peak and fade: anti-aliasing reconnected (`merge_orphans`), motes as 2 px chunks; orphan limit back to 9 (measured 5, 0, 0, 0 on the VFX keys).

Re-frozen: cast keys 0 (master ears), 16, 24, 28; VFX keys 5 and 7 (0 and 4 unchanged in pixels, fixture rewritten). Release head checked at 1x on the run and air bases against the master head (`keys-scale.png`, strip 3): the head outline drops at the ears, and the open mouth and glare read. No further gate; the blind review follows.

## 15. Executor status (in-betweens, export, evidence)

Done in `scripts/author_donut_missile.py` by literal pixel rows and float rules (no image-model output, no tracing). Frozen keys 0, 16, 24, 28 and VFX 0, 4, 5, 7 are byte-identical to the fixtures (V09).

- **Follow head (frames 32-47):** `cast_head("cast_follow")` starts from the gather head (one row lowered lids, closed master mouth, master catchlights, master tiara and gem, no fangs, no brow pixels, no twinkle) and moves the ears halfway: tips one row lower than the master (row 9, `jK` each side), a dark outline pixel one px out past the master wedge (release: two px out, tips gone). The +1 px thrust is kept. Composited per locomotion frame by the same head-offset rule as the other phases, so body, legs and tail stay the locomotion frame's.
- **Flight loop (VFX 1-3):** `bolt(lengths, core)` with streak lengths 10/10/7, 7/12/10, 9/11/9 (frame 0 stays 8/12/8) and the hot core shifted +1, -1 and 0 px; the oval rim does not move, white stays 2 px, the seam 3 to 0 changes lengths by at most 1 px.
- **Frame 6 (cooling peak):** `scratch_cooling()`: slashes width 1.1 (peak 1.2), no core `c`, no white, pink body with a magenta rim, the peak's six motes pushed 2 px further from the hit point, `merge_orphans` run.
- **Frame 8 (last):** `scratch_embers()`: two short `d`/`v` slash remnants (the outer scratches, one end each) and four 2 px motes in `v`/`d`; no `c`, `w`, `p` or `m`.
- **Validators:** both configs pass with the orphan limit still 9 and isolated 0. Measured orphans on the VFX frames 0-8: 5, 3, 4, 3, 0, 0, 0, 0, 0; white pixels 2, 2, 2, 2, 4, 2, 0, 0, 0. `unique_frames` is now on in both configs; rule C2 covers frames 16-31, C1 frames 0-15 and a new C3 the follow frames 32-47 (catchlight whites only).
- **Export:** `bash scripts/export_assets.sh` produces `generated/art/donut_h40_missile.{png,json}` (48 frames 72 x 44, three tags) and `generated/art/vfx_magic_missile.{png,json}` (9 frames 32 x 32, two tags) with the `.ase` files rebuilt from the grids; `generated/` is git-ignored as for every asset.
- **Tests:** `tests/test_art_donut_missile.py` gained checks for the follow head (mouth, lids, no fangs, master tiara, ears), unique frames and the orphan limit, the flight cycle, and the cooling and ember frames.

Open questions for the review (not decided by the executor): (1) the follow head differs from the gather head only by the ears and the missing twinkle, as the table asked; the review may want a stronger relax cue. (2) Release brow pixels use `K` on the outer lid ends only (frozen key); no change was made. (3) The tick-resolved cast GIF uses the placeholder speed and range of section 4; timing is gameplay tuning.
