# ANIM-DONUT-03: Princess Donut Magic Missile cast and VFX, design spec

**Status:** design keys frozen; in-betweens (section 9) not drawn; independent art review pending; not wired into any Bevy binary. Nothing here is accepted.

Scope: [MVP controller contract](../01-design/mvp-controller-contract.md#character-actions-and-progression). After the Tutorial Guild biscuit, Donut's primary attack (Square/X) becomes Magic Missile, on the same button, and it **stays movement-compatible**: she casts while standing, running or in the air. Her melee swat and pounce are ANIM-DONUT-02 (separate work order, not touched here). Gear after the biscuit is unchanged at Floor 1 ([gear timeline](donut-gear-timeline.md)): crown/tiara with the magenta gem, collar and charm; no new item is drawn.

## 1. Acting brief

A cat casting, not a human wizard: no staff, no wand, no raised paw, no glowing hands. Following the bible's behaviour order (neutral cute, expression change, spell anticipation, payoff, recovery to cute), the whole spell is acted by the **head**:

- **Gather (anticipation):** the ears flatten out sideways (airplane ears), the upper lids drop one row into a focused stare, and the tiara gem charges with a small four-point twinkle. Her magic is tied to the crown she already wears.
- **Release (payoff):** the head jabs 1 px forward, the lids drop into a half-lidded, inner-corner-down glare, and she hisses: a 4 x 3 open mouth with two fang pixels and a pink tongue. The bolt leaves from her right cheek at mouth height.
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
| 32-47 | `cast_follow` | follow head on frame `n` | in-betweens, currently copies of 16-31 |

Cast timeline (cast-local ticks, inclusive): **gather 0-3** (4 ticks, 67 ms), **release tick 4** (projectile spawns), **release 4-9** (6 ticks), **follow 10-15** (6 ticks), back to locomotion at tick 16. **Re-fire placeholder: tick 16** (3.75 casts/s); a press during follow buffers. The grid's `duration_ms` per frame is the locomotion frame's duration rounded to whole 60 Hz ticks (preview only; the runtime timing is the locomotion clock plus the phase table above).

Projectile spawn: bolt pivot at **(+23, -19)** from Donut's pivot on idle (right cheek, mouth row), plus the locomotion frame's head offset; the manifest lists all 16 (`projectile_spawn_offset_by_locomotion_frame`). On a run frame the bolt flies 15-16 px above the floor, on idle 19 px, which hits a 36 px goblin's torso.

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
- Glow is **baked** (VFX convention of [VFX-01](vfx-01-spec.md) section 5): the A1 pass does not bloom actor-group emitters, so a small hot core and a darker outer ramp carry the glow; white is capped (bolt 2, flash 4, peak 2, fade 0, no core `c` in the fade). Read checked lit and with the A1 pass off (`keys-scale.png`, bottom strip, `Floor.render(..., lit=False)`).

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
5. Validators pass (`python3 -m scripts.pixelart validate` on both configs). VFX orphan limit is **12** for the scratch peak (thin anti-aliased slashes); the executor should bring it to 9 or justify 12.
6. White and core caps as in section 5; no frame covers more than 35 % of the VFX cell (test).

## 8. Per-key executor notes

| Key | Notes |
| --- | --- |
| Cast 0 (gather, idle) | Ears: tips removed, inner ear shown one row lower and 2 px out. Lids: face row 16 cleared, row 17 lid. Twinkle: 5 px plus above the gem, gem centre white. Do not add motes (isolated pixels). |
| Cast 16 (release, idle) | Head, face and tiara +1 px x (collar does not move). Lids at row 18 with the inner two pixels down at row 19; catchlight 1 px at row 19. Mouth rows 23-25: `KKKK` / `WEEW` / `.cc.`. Gem back to plain with a `P` centre (spent). |
| Cast 24 (release on run frame 8) | Composition key: head offset (+1, +4) plus thrust; check the chin against the near foreleg. |
| Cast 28 (release on jump rise 12) | Composition key: offset (+1, +2); check the head against the tail streaming behind. |
| VFX 0 (bolt) | Rear rim `d` joins the streaks; keep three separate streak rows with empty rows between. |
| VFX 4 (flash) | Round, not spiky; 3 spray chunks only on the caster side. |
| VFX 5 (peak) | Slashes 5 px apart, steep down-back rake; 2 px white at the centre only. |
| VFX 7 (fade) | Each slash broken in the middle; no `c` or `w`. |

## 9. In-between table (no design decisions needed)

| Frames | Currently | Target |
| --- | --- | --- |
| 32-47 `cast_follow` | copies of 16-31 | Follow head drawn once at frame-0 position and composited by the same rule: thrust kept (+1, 0), mouth closed to the master `cc` line, lids back to one row lowered (gather lids), ears halfway up (tips back, 1 px lower than master), no twinkle, gem normal. Then turn on `unique_frames`. |
| VFX 1-3 | copies of 0 | Streak lengths cycle (8/12/8, 10/10/7, 7/12/10, 9/11/9), head core shifts 1 px fore/aft, white stays 1-2 px; clean loop seam. |
| VFX 6 | copy of 5 | Between peak and fade: slashes 1 px thinner, centre `c` gone, white gone, motes 2 px further out. |
| VFX 8 | copy of 7 | Last motes and two short `d`/`v` slash remnants; nothing hot. |

Derived cast frames 1-15 and 17-31 come from the rule; the executor only fixes local collisions (none seen at review scale) and must not edit the frozen keys.

## 10. Evidence

| File | Shows |
| --- | --- |
| `docs/evidence/anim-donut-03/keys-scale.png` | Top: lit Floor 1 at 1x: cast keys 0, 16, 24, 28 (the air key lifted 14 px), a bolt in flight, the scratch peak on a GOB-01 goblin. Bottom: the same with the A1 pass off. |
| `docs/evidence/anim-donut-03/keys-scale-4x.png` | The lit strip at 4x. |

Regenerate: `bash scripts/export_assets.sh` once, then `python3 scripts/author_donut_missile.py keys` and `python3 scripts/author_donut_missile.py key-scale`; rebuild the `.ase` files with `python3 scripts/pixel_grid_to_ase.py build <grid> <ase>` and refreeze with `python3 -m scripts.pixelart freeze --force <config>` only on a design change.

## 11. Files

| File | Role |
| --- | --- |
| `scripts/author_donut_missile.py` | authoring record: cast heads, composition rule, VFX rules, manifests, evidence |
| `assets/source/characters/donut/donut_h40_missile.grid.json`, `.ase`, `.pixelart.json` | cast source, Aseprite file, config |
| `assets/source/vfx/magic_missile/vfx_magic_missile.grid.json`, `.ase`, `.pixelart.json` | VFX source, Aseprite file, config |
| `assets/manifests/donut_h40_missile.json`, `assets/manifests/vfx_magic_missile.json` | phase ticks, spawn offsets, placeholders, provenance |
| `tests/fixtures/anim-donut-03_keys.json`, `tests/fixtures/anim-donut-03-vfx_keys.json`, `tests/test_art_donut_missile.py` | frozen keys and spec tests |

## 12. Open questions

- Tail: a bottle-brush tail on release would be a strong cat read but breaks the head-only swap (the tail differs per locomotion frame). Left out; reviewer to decide whether it is worth a per-frame tail layer.
- On the spawn tick the bolt's streaks overlap her right cheek for about 2 ticks; acceptable as "leaving the mouth" or move the spawn 4 px forward.
- Gather is only 4 ticks; at 1x the lid change is subtle and the twinkle carries the tell. Gameplay may want 6 ticks.

## 13. Rights and provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope (see `docs/04-production/rights-and-release.md`). Spec + keys: Claude Opus. In-betweens and execution: pending. Review: pending. The cast heads are literal pixel edits of the approved master head; the VFX keys are rasterized from rules in `scripts/author_donut_missile.py`. No tracing, no image-model raster output, no third-party missile design used as reference.
