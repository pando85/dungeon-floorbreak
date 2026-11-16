# ANIM-DONUT-01: Princess Donut locomotion candidate (40 px)

**Status:** draft candidate, awaiting owner art review. Nothing here is accepted. The approved master [`donut_h40.grid.json`](../../assets/source/characters/donut/donut_h40.grid.json) and its manifest are unchanged; the candidate lives in its own source and manifest until a human approves it. Runtime integration (state machine, Bevy atlas swap) is a separate work order.

Authoring and the self-review below were done by Claude Opus (art-tier model, see AGENTS.md "Model routing for art work"). An independent Opus art review is a separate step.

## Files

| File | Role |
| --- | --- |
| [`assets/source/characters/donut/donut_h40_locomotion.grid.json`](../../assets/source/characters/donut/donut_h40_locomotion.grid.json) | editable source (pixel-grid-v1), **source of truth** |
| `assets/source/characters/donut/donut_h40_locomotion.ase` | Aseprite/LibreSprite file rebuilt from the grid by the export (layers, tags, frame timings) |
| [`assets/manifests/donut_h40_locomotion.json`](../../assets/manifests/donut_h40_locomotion.json) | candidate manifest `donut_h40_locomotion_candidate_v0`, `prototype_fixture_unapproved` |
| [`scripts/author_donut_locomotion.py`](../../scripts/author_donut_locomotion.py) | authoring record: parts, pose data and cleanup patches that produced the grid (`--check`, `--force`) |
| [`scripts/make_donut_locomotion_review.py`](../../scripts/make_donut_locomotion_review.py) | contact sheets, GIFs, Floor 1 lit compositions with Carl, measurements |
| [`tests/test_art_donut_locomotion.py`](../../tests/test_art_donut_locomotion.py) | idle preserved, face/head are master pixels, palette, contact rows, clean clusters |
| [`docs/evidence/anim-donut-01/`](../evidence/anim-donut-01/README.md) | committed review renders with SHA-256 |

## Frames, tags and timings

Canvas **72×44**, pivot **(46, 43)**, one atlas row of 16 frames. Layers bottom to top: `tail, legs_far, body, legs_near, head, face, gear` (the master's five plus two leg layers, empty in the idle frames). Palette: the master's 22 keys, unchanged.

| Frame | Tag | ms | Pose | Lowest row |
| --- | --- | --- | --- | --- |
| 0–3 | `idle` | 320/260/180/260 | approved master idle, pixel-identical, re-padded (+24 px) | 43 |
| 4 | `run` | 80 | F0 near fore contact, hinds trailing | 43 |
| 5 | `run` | 70 | F1 far fore contact, near fore toe-off, hinds swing forward, body lowest | 43 |
| 6 | `run` | 80 | F2 gathered: back arched, near hind lands under the belly | 43 |
| 7 | `run` | 70 | F3 far hind contact, near hind push-off, fores swing forward | 43 |
| 8 | `run` | 90 | F4 extended suspension: body long, fores reach, hinds trail | 40 (airborne) |
| 9 | `run` | 70 | F5 descending, fores reach down | 42 (airborne) |
| 10 | `jump_start` | 90 | crouch: chest low, rear up, hinds bunched, tail lifts | 43 |
| 11 | `jump_start` | 70 | push-off: hinds extended, front rising, fores lift forward | 43 |
| 12 | `jump_rise` | 120 | rising: long tilted body, fores forward, hinds trailing, tail streaming | 43 |
| 13 | `jump_fall` | 120 | falling: front slightly down, fores reach for the floor, tail up for balance | 43 |
| 14 | `land` | 80 | impact: squashed, eyes squeezed shut (master blink curve), tail flicks | 43 |
| 15 | `land` | 110 | recovery: chin up first while the body is still low | 43 |

Run cycle 460 ms. Opaque heights: idle 40–41, run 35–38, jump 37–42, land 35–37 (full table in `measurements.json`).

## Notes for the integration agent

- **Frame size changes from 46×44 to 72×44.** A cat stretches horizontally; the extended run and rise frames are 63–64 px wide. All 16 frames, including idle, share one canvas, so `actor_sprite::DONUT` needs `width: 72, pivot_x: 46` (height, `pivot_y: 43` and the 40 px idle opaque height stay) when this atlas replaces `donut_scale_h40.png`. The approved 46×44 atlas is untouched until then.
- **Pivot convention kept:** the pivot is on the contact row (row 43) and every grounded frame's lowest opaque row is row 43 (0 px sink). Horizontally the pivot sits 6 px behind the nose in every frame, exactly as in the idle (pivot 22, nose 28), so the face stays within 1 px horizontally when switching idle ↔ run ↔ jump. The pivot is therefore under the chest in profile poses, not under the body centre; flipping mirrors around the face.
- **Airborne frames:** run F4/F5 are a gallop suspension; their lowest row is 3/1 px above the pivot row on purpose. Keep the physics foot on the walking line; do not snap these frames down. `jump_rise` and `jump_fall` have their lowest pixel (trailing hind paws / reaching fore paws) on row 43, so the sprite sits on the physics foot position through the arc and `land` frame 14 meets the floor without a pop.
- **Playback suggestions (not decisions):** `run` loops; at `RUN_SPEED` 185 px/s one 460 ms cycle covers ~85 px (≈14 px per frame), which suits a short-legged gallop. Scale run frame time with ground speed for slower movement; there is no walk yet. `jump_start` plays once on take-off (160 ms, may be shortened to frame 11 only if input latency matters), `jump_rise` holds while rising, `jump_fall` holds while falling, `land` plays once on touchdown, then `idle` or `run`.
- **Trail:** Donut trails Carl by 32 px and draws behind him; her profile head reaches 22 px ahead of her pivot, so it overlaps Carl's back leg by a few pixels (see `floor01_trail_run_*`). This is the owner-accepted pass-through behaviour.
- No events or hitboxes are authored; `footstep` markers could be F0, F1, F2, F3 if audio needs them.

## How it was made (provenance)

- **Copied, not redrawn:** head, face, tiara, chest ruff, collar, butterfly charm and the tail plume are the master's pixels, translated only. The tests prove the face and head layers of every locomotion frame are the master's pixel sets (the landing impact frame uses the master's own blink face). Coat map, eyes and accessories are therefore identical in every frame.
- **New, hand-placed:** a side-view torso (24×19) with the master's near-side coat map (ginger saddle behind the shoulder, ginger haunch blotch with a dark brindle bite, H rim light, tufted dark skirt), a rounded tufted bottom for the chest ruff, seven 1 px shadow-side tufts on the tail edge that the seated idle hides, and 14 leg poses (cream fore socks; hind legs continue the ginger haunch blotch; far legs in shadow values).
- **Pose data:** whole-pixel offsets plus pixel-safe torso edits (repeat/drop one column for extension/compression, lift a column run for the arched back, per-column offsets for the tilted leap, per-row tail shear for sway). No rotation, scaling or resampling.
- **Cleanup:** 23 per-pixel fixes after the lint orphan report (mostly where the head outline or a tilt broke a two-pixel cluster). Final lint: 0 orphan and 0 isolated pixels in all 16 frames.
- Concept board use: the turnaround and the board's tiny "gameplay sprite concept" run/jump mocks were viewed as enlarged crops for silhouette only (profile body, 3/4 head, plume up). No pixel was copied from the board.

## Gait design

- **Feline, not human or generic quadruped:** a short-legged half-bound gallop with alternating contacts near fore → far fore → near hind → far hind, a gathered phase (back arched, hind paws landing under the belly) and one extended suspension. A planted paw only ever moves backwards relative to the body between frames, so there is no skating.
- **Weight:** the body bobs 3 px over the cycle (lowest at F1, highest at F4); the head bobs 1 px and lags the body by a frame, so she runs with her head steady and level, chin up. The tail follows the bob one frame late and sways 0–7 px at the tip.
- **Personality:** the proud, level head and raised plume over tiny scurrying cream paws is the joke; the landing squeezes her eyes shut ("oof") and the recovery lifts the chin before the body (dignity restored).

## Key-pose review log (before the full sequence)

Contact, gathered and extended were built and inspected first at 6–12× and at 2–3× (`key_pose_review_round1_vs_final_5x.png`). Problems found and fixed before the in-betweens:

1. Round 1 torso was 38 px long and read as a dachshund; shortened to 24 px and rounded.
2. The haunch blotch with a dark centre read as a ring ("donut"); the brindle bite now enters from the edge.
3. A 2 px cream shoulder fleck read as a second eye; it is now cream-shadow and smaller.
4. Stick legs on a long body; legs became short stubs under the skirt, hind legs ginger so they read against the dark body.
5. The chest ruff was a square cream slab; it now has a rounded, tufted bottom and the master's chin line.
6. The trailing hind leg read as a detached "carrot"; it now attaches under the haunch at a diagonal.
7. The exposed tail base read as a flag on a pole; the plume sits 3 px further into the rump and its shadow edge got tufts.
8. Reaching forelegs drawn over the cream chest became dirty outline dots; airborne forelegs now reach out below the chest.

## Artistic self-review (Opus, not an acceptance)

What works: identity is locked (same face, eyes, tiara, collar, charm, coat map in every frame); the silhouette at 1× reads as a fluffy Persian with a plume, close to the board's run mock; the run has a readable gather/extend rhythm; landing has a clear personality beat; the idle is untouched and the face stays within 1 px horizontally across state changes.

Known weaknesses:

- **Static head.** The 3/4 head is the master's exact pixels in every frame, so it never turns, tilts or changes expression while running. This is the price of guaranteed face consistency; a later pass could add ear or eye acting variants.
- **3/4 head on a profile body.** Readable and matches the concept mocks, but anatomically it is a stylisation: her face looks at the camera, not where she runs.
- **Small legs.** Legs are 3–6 px; the gait reads mostly through paw positions and body bob. At 1× (640×360) the alternating contacts are hard to see individually; at 1080p (3×) they read.
- **Dark body on dark floors.** The tortoiseshell torso is low-contrast in cold shadow; the tail, face and ginger patches carry the silhouette.
- **Pixel-edit artefacts.** Column stretch lengthens the saddle patch in F4; the arch and tilt leave small steps on the back line. All clusters are clean, but a hand polish pass on F2, F4 and the jump frames would smooth them.
- **Jump forelegs** in `jump_start`/`jump_rise` are short paws peeking out under the chest rather than a full reach.
- **No in-engine check.** Floor 1 compositions use the Pillow A1 model; motion was judged frame by frame and in GIFs, not in Bevy, and no human has judged game feel.

## Regenerate

```bash
python3 scripts/author_donut_locomotion.py --check    # the grid still matches its authoring record
ART_CLI=/path/to/aseprite-or-libresprite bash scripts/export_assets.sh
python3 scripts/make_donut_locomotion_review.py --out docs/evidence/anim-donut-01
python3 -m unittest discover -s tests -p 'test_art_donut*.py'
```

`--force` rewrites the grid from the authoring record and overwrites any later hand edits.
