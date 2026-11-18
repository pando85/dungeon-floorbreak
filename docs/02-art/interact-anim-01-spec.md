# ANIM-INTERACT-01: Carl and Princess Donut interact animations, design spec

**Status:** execution phase done: all 12 Carl and 8 Donut frames drawn (six frozen keys untouched, in-betweens authored from the table in section 5), grids, `.ase` sources, manifests, atlas export, tests and evidence delivered (section 12); independent review and owner review pending. Nothing is wired into a Bevy binary. Rights basis for publishing: owner-reported `ART-PUBLICATION-ALL-V1` ([rights register](../04-production/rights-and-release.md)). No reuse licence, no game or asset-pack distribution right.

Read with: [MVP controller contract](../01-design/mvp-controller-contract.md) (Interact row, R1/RB), [asymmetric traversal and gates](../01-design/asymmetric-traversal-and-gates.md), [antihero tone](../01-design/antihero-tone.md), [Carl bible](carl-character-bible.md), [Donut bible](donut-character-bible.md), [ANIM-CARL-02 melee spec](carl-melee-anim-02-spec.md) (structure, helpers and identity rules this spec inherits), [pixel-art pipeline](../07-agents/pixel-art-pipeline.md).

| File | Role |
| --- | --- |
| `assets/source/characters/carl/carl_h52_interact.grid.json` | pixel-grid-v1 source, 12 frames (keys at 1, 2, 5, 9; the rest drawn) |
| `assets/source/characters/donut/donut_h40_interact.grid.json` | pixel-grid-v1 source, 8 frames (keys at 1, 5) |
| `assets/source/characters/{carl/carl_h52,donut/donut_h40}_interact.ase` | built from the grids by `scripts/pixel_grid_to_ase.py` |
| `assets/source/characters/{carl/carl_h52,donut/donut_h40}_interact.pixelart.json` | toolkit configs: the **machine-readable tick contract** (`tags.*.frames` = `[frame, ticks]`), frozen-key rule |
| `assets/manifests/{carl_book1_h52,donut_h40}_interact.json` | export manifests: atlas and metadata paths, tick contract (`interact.moves`), stand-off distances, contact boxes, carried-item hand anchors |
| `scripts/author_interact01.py` | deterministic authoring record (`--check`, `--evidence`) |
| `tests/test_art_interact01.py`, `tests/fixtures/anim-interact-01-{carl,donut}_keys.json` | contract tests and the frozen keys |
| `docs/evidence/anim-interact-01/` | contact sheets, GIFs per verb, Floor 1 scene composites at 1× and 3×, timing diagram, keys at 1× (see its `README.md`, `SHA256SUMS`) |

## 1. Scope and acting brief

Interact is **R1/RB**. It is never an enemy grab (the contract keeps grabs out of interact). The set is a small group of **reusable verbs** that any prop, door, switch or pickup can use. The verb is chosen by the prop's height class (section 4), not by the prop. Props are separate art; these frames contain no prop pixels. The runtime attaches a carried item to the hand anchor from the contact tick on.

- **Carl** is a plain guy doing chores in a dungeon. He grabs a handle a bit too eagerly, hauls it down with his whole weight and sags back on his heels. He squats to pick something up the way a man with a sore back does: heels up, one hand braced on his knee, a grunt. He shoves heavy things with his fists and his weight, back leg straining on its toe. No heroic stance, no flourish, no posing for the camera.
- **Princess Donut** does only what a cat plausibly does: she **pats** a low switch or small item with a front paw, and she **nudges** a small thing with her muzzle and forehead. She never grips, pulls a lever, turns a handle or opens a door with a "hand". Anything above her low class is Carl's job or a gate for her jump (asymmetric traversal: Donut conquers space, Carl transforms it).

## 2. Frame size and pivot (integration contract)

- **Carl: 48 × 56, pivot (25, 55)**, Donut: **72 × 44, pivot (46, 43)**: identical to each character's approved locomotion atlas (and Carl's melee atlas). Same palette (23 / 22 colours) and the same layers in the same order. The atlases can be loaded next to locomotion with no pivot math.
- All tags are **grounded**: the lowest opaque row is the pivot row in every frame. Columns 0 and W-1 and row 0 stay empty.
- **Facing:** all frames face right; mirror around the pivot column for left.
- **Root motion:** none. Velocity is zero during every interact verb; the feet stay planted. Gameplay positions the actor at the verb's **stand-off distance** (section 4) before the verb starts.

## 3. Tick windows (60 Hz)

Tick 0 is the fixed tick on which R1 is accepted for a valid target. Frame selection is a pure function of the verb clock. The **contact tick** is the first tick of the contact frame: the runtime applies the effect (switch toggles, item attaches, prop impulse) on that tick, never on the press. `duration_ms` in the grid is only for previews (`round(ticks × 1000 / 60)`).

### Carl

| Frame | Tag | Name | Ticks | Window | Role |
| --- | --- | --- | --- | --- | --- |
| 0 | `interact_hand` | u0 | 5 | t0-4 | reach start (in-between) |
| 1 | `interact_hand` | **u1** | 8 | **t5**-12 | **KEY, contact**: grip on the handle |
| 2 | `interact_hand` | **u2** | 10 | t13-22 | **KEY**: haul down, sag back |
| 3 | `interact_hand` | u3 | 8 | t23-30 | let go, settle (in-between) |
| 4 | `interact_floor` | p0 | 8 | t0-7 | bend start (in-between) |
| 5 | `interact_floor` | **p1** | 10 | **t8**-17 | **KEY, contact**: hand on the item |
| 6 | `interact_floor` | p2 | 10 | t18-27 | push up off the knee (in-between) |
| 7 | `interact_floor` | p3 | 8 | t28-35 | stand, item at the hip (in-between) |
| 8 | `interact_push` | s0 | 6 | t0-5 | set: lean in (in-between) |
| 9 | `interact_push` | **s1** | 8 | **t6**-13 | **KEY, contact**: fists on the face, back leg driving |
| 10 | `interact_push` | s2 | 12 | t14-25 | strain, the prop gives (in-between) |
| 11 | `interact_push` | s3 | 8 | t26-33 | step back, shake it off (in-between) |

Totals: `interact_hand` 31 ticks (517 ms), contact t5; `interact_floor` 36 (600 ms), contact t8; `interact_push` 34 (567 ms), contact t6.

### Donut

| Frame | Tag | Name | Ticks | Window | Role |
| --- | --- | --- | --- | --- | --- |
| 0 | `interact_paw` | d0 | 6 | t0-5 | paw lifts, weight back (in-between) |
| 1 | `interact_paw` | **d1** | 6 | **t6**-11 | **KEY, contact**: paw pats the target |
| 2 | `interact_paw` | d2 | 8 | t12-19 | paw lifts off, head stays on the target (in-between) |
| 3 | `interact_paw` | d3 | 6 | t20-25 | paw down, back to idle (in-between) |
| 4 | `interact_nudge` | n0 | 8 | t0-7 | head lowers (in-between) |
| 5 | `interact_nudge` | **n1** | 8 | **t8**-15 | **KEY, contact**: muzzle on the item |
| 6 | `interact_nudge` | n2 | 10 | t16-25 | push through: head forward 1-2 px, tail flicks (in-between) |
| 7 | `interact_nudge` | n3 | 8 | t26-33 | head up, back to idle (in-between) |

Totals: `interact_paw` 26 ticks (433 ms), contact t6; `interact_nudge` 34 (567 ms), contact t8.

Interrupts: a hit during any verb cancels it into hurt (no armour); if the cancel comes before the contact tick, the effect does not happen. Whether an interrupted push still applies its impulse after t6 is a gameplay decision (proposal: yes, the effect already happened).

## 4. Object-height classes and contact boxes

Pivot-relative, x forward, y up from the floor, inclusive, measured from the keys (`tests/test_art_interact01.py` checks them).

| Class | Prop contact band (y) | Carl verb | Donut verb | Key contact part | x | y |
| --- | --- | --- | --- | --- | --- | --- |
| **floor** | 0-8 | `interact_floor` (pickups, floor levers) | `interact_nudge` (small item, floor plate) | Carl p1 hand | +14..+19 | 4..7 (hand), sleeve up to 13 |
| | | | | Donut n1 muzzle and cheek | +19..+24 | 7..25 (head front) |
| **low** | 4-12 | (uses `interact_floor`) | `interact_paw` (low switch, small item on a step) | Donut d1 paw (bottom on the switch top) | +16..+21 | 5..10 |
| **waist** | 14-26 | `interact_push` (crates and heavy props at least 26 px tall; face at +20) | none | Carl s1 near fist (belly) | +14..+19 | 19..23; far fist (chest) +15..+20, 30..34 |
| **hand** | 24-34 | `interact_hand` (switch, lever, door handle, chain) | none (a cat cannot use it) | Carl u1 grip | +12..+17 | 26..31 |
| | | | | Carl u2 fist (lever thrown) | +11..+16 | 19..24 |

**Stand-off distances** (pivot to the prop's near face): `interact_hand` +17 to +18 (the handle at +17..+22), `interact_floor` +15 (item 4-6 px wide), `interact_push` +20, `interact_paw` +18 (switch 5 px wide), `interact_nudge` +21. Tolerance: ±3 px for Carl's verbs, **±2 px for Donut's** (her contact pixels are only 1-2 px deep). A prop above Donut's low class has no Donut verb: the prompt is not shown to her.

**Hand anchor for carried items (Carl p1 onwards):** the far hand centre, (+16, 5) at p1; the execution phase records it per frame for p2 and p3 in the config (`anchors`).

## 5. Keys and executor notes

**Keys are final for pose, silhouette and contact pixels**; they are frozen (`tests/fixtures/anim-interact-01-*_keys.json`) and the execution phase keeps them pixel-identical. Design problems found while drawing in-betweens go back to the designer.

Construction (same rules as ANIM-CARL-02 section 9): Carl's hair, head, torso, jacket, pin and boxer seat are the master's pixels translated by **U(dx, dy)** from locomotion idle 0 (`author_carl_melee.upper`), heads from the existing variants (v0 neutral, v3 effort closed mouth), head offset at most 2 px. Arms, legs and boxer thighs are rasterised tubes with the master ramps; Crocs and fists are stamps (`CROC_FLAT`, `CROC_TOE`, `FIST_BIG`, `FIST_SMALL`). Donut's tail, body, head, face and tiara are her idle 0 pixels translated; the tapping foreleg is a rasterised cream tube; collar pixels hidden under her lowered head are dropped.

| Key | Construction | Executor notes |
| --- | --- | --- |
| **u1** reach contact | U(+2, +1), head v0 (dumb "huh" mouth), b3 standing legs, idle near arm hanging, far arm straight forward-down, `FIST_BIG` grip at x37-42, rows 24-29 | Reads as reaching, not a jab: no lunge, no back-swung arm, the fist sits lower than the jab fist (rows 21-26). Keep the near arm slack. |
| **u2** haul down | U(-1, +3) (sagging back on the heels), head v3 offset (0, +1), short bent legs (hems on row 42), far arm a taut diagonal from the shoulder (28, 23) to `FIST_BIG` at x36-41, rows 31-36, on the thrown handle | The everyman beat: he uses his body weight, not his arm. Don't add a cheer or a fist pump on release (u3). |
| **p1** pickup contact | U(+1, +11), head v3 offset (+1, 0); deep squat, boxer seat bottom on row 49 (5 px of clear floor), back Croc on its toe directly under the seat (`CROC_TOE` at x21, rows 50-55, heel lifted), front `CROC_FLAT` at x30; near arm across the chest, `FIST_SMALL` braced on the front knee (x31-36, rows 38-41), watch on its cuff; far arm down past the knee, `FIST_SMALL` on the item at x39-44, rows 48-51 | The hand on the knee is the joke and the tone. The squat must not read as sitting: in p0, p1 and p2 the seat stays at least 4 px off the floor (tested for p1). |
| **s1** shove contact | U(+8, +3), head v3 offset (+1, +1); back leg a long diagonal to `CROC_TOE` at x12, front leg planted under the hips (`CROC_FLAT` at x33); **two contact heights** on the prop face: far `FIST_BIG` at chest (x40-45, rows 20-25), near `FIST_BIG` at belly (x39-44, rows 31-36) on its own diagonal forearm | The torso cannot rotate (identity lock), so the lean is carried by the U offset and the back leg. Never re-stack the fists: two heights are what separates it from a punch. s2 strains (head offset +1 more, back heel 1 px lower); s3 steps the back foot in and drops the arms. |
| **d1** paw tap contact | head, face and tiara (0, +1); front foreleg removed from the body and closed with a `K` belly line; a short new foreleg starts at the shoulder (57, 33) and slopes down to a round paw (disc, x63-67, rows 34-38) whose bottom rests on the switch top | One paw only; the contact is the bottom of the paw on the switch top, never its side, and the paw stays round (no square or stick end). d0 lifts the same paw 2-3 px higher and tucks it (a cat's wind-up); d2 lifts it off 2 px. The other paws do not move. |
| **n1** nose nudge contact | head, face and tiara (+2, +10); body and tail unchanged; collar hidden under the chin | Rump and tail stay up so the silhouette is "head down, nose in", not lying down. n2 pushes the head +1..+2 px forward, the tail tip flicks. |

### In-between table (execution phase)

| Frame | From key | Change |
| --- | --- | --- |
| u0 | idle → u1 | U(+1, 0), far arm half raised (elbow bent, fist at rows 30-33, x33-37), head v0 |
| u3 | u2 → idle | U(0, +2), far arm lets go and drops to the idle far-arm position, head v1 glance (checks if it worked) |
| p0 | idle → p1 | U(+1, +5): half squat (the land frame 14 legs are a good start), far arm hanging forward, near arm going to the knee |
| p2 | p1 → p3 | U(+1, +6), near hand still on the knee pushing up, far hand at the hip (rows 38-41) |
| p3 | p2 → idle | U(+1, +1), standing, far hand at the hip (the item rides there), head v3 |
| s0 | idle → s1 | U(+4, +2), back foot steps back 4 px, both arms bent forward, fists already at two heights (chest and belly) |
| s2 | s1 hold | head offset (+1, +2), back heel 1 px lower, fists unchanged (the contact holds) |
| s3 | s2 → idle | U(+3, +1), back foot steps in, arms drop to idle, head v1 |
| d0 | idle → d1 | head (0, +1), the front paw lifted and tucked, round, at rows 30-34, x58-62 (above the d1 paw, so d1 comes down onto the switch) |
| d2 | d1 → d3 | paw lifted 2 px straight up off the switch top, head still lowered |
| d3 | d2 → idle | paw back on the floor (idle front leg), head at idle |
| n0 | idle → n1 | head, face and tiara (+1, +5) |
| n2 | n1 hold | head (+3, +10), tail tip 1 px up |
| n3 | n2 → idle | head (+1, +4) |

Done in the execution phase (section 12): `unique_frames` is `true` in both configs, the design-phase "in-betweens are key copies" test is replaced by per-frame checks of this table, GIFs and contact sheets come from `python3 -m scripts.pixelart evidence`, and the evidence folder has a contact sheet and `SHA256SUMS`.

## 6. State flow

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Interact: R1 with a valid target in range (velocity set to 0)
    Run --> Interact: R1, target in range (stop, then verb)
    Interact --> Idle: last tick of the verb
    Interact --> Hurt: hit (effect only if past the contact tick)
```

The verb is chosen by the target's class and the active character (section 4). Target priority, prompt and the no-accidental-pickup rule during combat stay with the controller contract.

## 7. Integration notes and open decisions (gameplay and input owners)

1. **Effect on the contact tick**, not on the press (section 3).
2. **Stand-off snap:** the actor walks to the stand-off distance or the prompt only shows inside a small tolerance: ±3 px for Carl, ±2 px for Donut (decided at the silhouette gate). The art assumes no slide.
3. **Push is a single shove** (an impulse at t6), not a sustained push-walk. A held push with a walking loop would need new frames.
4. **Doors:** doors at hand height use `interact_hand`; no separate door-opening verb is drawn.
5. **Donut and high props:** no verb. A Donut-only route uses the low class (G1 in the gates doc: a small credible interaction that opens a shared route).

## 8. Acceptance targets

1. Frame size, pivot, palette and layers identical to each locomotion atlas; all frames grounded on the pivot row; edge columns and row 0 empty.
2. The six keys pixel-identical to the frozen fixtures in every later phase.
3. Tick windows in the configs equal section 3; contact ticks u1 t5, p1 t8, s1 t6, d1 t6, n1 t8.
4. Contact boxes inside the section 4 bands (`tests/test_art_interact01.py`).
5. At 1× on lit Floor 1 next to a placeholder prop: each verb reads without the prop (silhouettes distinct from idle, from each other and from the melee keys; u1 is not a jab, p1 is not sitting, s1 is not a punch).
6. Tone gate: Carl plain and a bit clumsy, never heroic; Donut does only cat actions (no grip, no pull, no handle).
7. Identity: hair, head, torso, jacket, pin and boxers (Carl); tail, head, face and tiara (Donut) are translated master pixels in every frame.
8. Toolkit validators pass for both configs; `python3 scripts/author_interact01.py --check` passes.

## 9. Difficulty score and route

Per [routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty): Carl scores **1** (native height above 40 px; not a new design, the approved master; no complex construction; no VFX). Donut scores **0** (40 px, approved master). Both are **Easy, route C**: the design phase wrote this spec and drew every key (more than the one hero key the route needs, because the keys define the verbs); the execution phase draws the eight Carl and six Donut in-betweens; an independent review checks the keys first. No silhouette gate is needed (not a new design), but the reviewer should apply acceptance target 5 to the keys before in-betweens are accepted.

**Living-horizon fit:** the verbs are generic and reusable and add no ability; they fit [character-progression-horizon.md](../01-design/character-progression-horizon.md) (Carl transforms space, Donut acts on small things) and the gates doc. No horizon update needed.

## 10. Rights and provenance

Derived from the owner-approved Carl 52 px master (via the ANIM-CARL-01 locomotion canvas) and the owner-approved Donut 40 px master (via her locomotion canvas); master parts translated only, new parts hand-placed or rasterised by `scripts/author_interact01.py`. No image-model raster output, no tracing, no third-party assets. The placeholder lever, switch, crate and item rectangles in the evidence are context only, not art. Spec and keys: Claude Opus. In-betweens, export, tests and evidence: Claude Sonnet (executor).

## 11. Silhouette gate (key-fix round)

Independent gate on the six keys: verdict **FIX-KEYS**. Tone and fidelity to the masters passed; u1 and n1 passed. Fixed in one round (sections 4 and 5 now give the measured pixels):

1. **p1** read as sitting: hips raised 2 px (U(+1, +11)), seat bottom 5 px above the floor, back `CROC_TOE` directly under the seat with the heel lifted.
2. **s1** read as a punch (stacked fists at shoulder height): the near fist dropped 5 px to belly height on its own forearm, the far fist dropped 1 px; two contact heights.
3. **d1** read as a stick or human arm: foreleg shortened, sloping down from the shoulder, round paw whose bottom rests on the switch top.
4. **u2** was close to idle: U(-1, +3) sag and a taut diagonal arm to the thrown handle.
5. n1 left unchanged.
6. Stand-off tolerance for Donut set to ±2 px; the other section 7 proposals stand.

No further gate before execution; the blind review re-checks the keys.

## 12. Execution phase result

**Delivered.** Carl u0, u3, p0, p2, p3, s0, s2, s3 and Donut d0, d2, d3, n0, n2, n3 are authored in `scripts/author_interact01.py` from master pixels (translated), the ANIM-CARL-02 helpers and rasterised tubes; no prop pixels, no image-model output, no tracing. The six keys are pixel-identical to the fixtures (V09). Both toolkit configs pass all configured validators (`unique_frames` true, `canvas_margin` `[1, 1, 1, 0]`, frozen keys, grounded rows, orphan-free); limits were not raised. The atlases export with `scripts/export_assets.sh` (`generated/art/{carl_book1_h52,donut_h40}_interact.{png,json}`) and `scripts/validate_assets.py` accepts both manifests.

**Decisions taken while drawing (all inside the frozen keys and the table; none changes a key pixel):**

1. **Donut n2 head is (+2, +11), not the table's (+3, +10).** At +3 the head outline sits on column 71, the frame's right edge, which section 2 and acceptance target 1 require empty. The push through reads as a 1 px lower, firmer press plus the tail flick. If the designer wants the forward 1 px back, the Donut canvas would have to grow by one column in every Donut interact frame (a pivot change versus locomotion), so it was not done. **Open question for the designer.**
2. **n2 tail flick:** the tail tip is stretched 1 px up (rows 14 and above move up one row, row 14 is repeated). Everything from row 15 down is the master tail.
3. **s2 back heel 1 px lower:** the back Croc toe stamp loses its top (heel) row and the leg ends 1 px lower; the bottom row stays on the pivot row. s2 skips the orphan cleanup pass so its arm pixels are s1's exactly ("fists unchanged").
4. **Feet.** No root motion. The small foot shuffles that remain: u0 front Croc x28 (idle 27, u1 29); u3 returns to the idle x15/x27; p0 back Croc flat x17 and front x29, p2 back toe x20, p3 x16/x28; s0 back Croc flat x11 (4 px behind idle), front x31; s3 x14/x30. A test limits any frame-to-frame jump of a Croc's left edge inside a verb to 4 px. p3 and s3 are 1 and 3 px from the idle feet, so the return to idle pops by that much unless gameplay holds the idle foot offset (a gameplay or integration note, not an art defect).
5. **Hand anchors for a carried item** (pivot-relative, y up, far hand centre): p1 (+16, 5), p2 (+12, 15), p3 (+11, 19). Recorded in `carl_h52_interact.pixelart.json` (`anchors.carried_item_hand`, keyed by frame index 5, 6, 7) and in the manifest; a test measures them from the grid. p3 stands up, so the item rides at the standing hip, 4 px higher than at p2.
6. **d3 is the master idle frame 0** exactly (the table: paw on the floor, head at idle); it is unique within the interact atlas, so V12 passes.
7. **n0, n2, n3 collar:** the collar pixels covered by the lowered head are dropped as in n1.

**Not done / still open:** no review yet; nothing is wired into Bevy; whether an interrupted push still applies its impulse (section 3) stays a gameplay decision. The p0 half squat keeps the near hand mostly behind the jacket edge (the reach to the knee is told by the arm angle); the reviewer may want it more visible.
