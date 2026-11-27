# BOS-ART: the Ball of Swine's room art and presenter, spec

**Status:** executed on the draft PR; the top-tier gate of the weight key and the independent review are pending. **Owner art review pending**; nothing here is owner-approved and game feel is the owner's call. This is the BOS-ART row of [the FLOOR01-BOS work order](../07-agents/floor-01-bos-workorder.md) and section 15 of [the BOS plan](../01-design/floor-01-bos-integration.md). Room 18 (`bos`, `RoomScene::Ball`, `--scene f01_ball`) was a labelled greybox (BOS-A, #316). It is now dressed and drawn.

Everything is **presentation only**: the sim, the pins (`MANIFEST_HASH` `0xe93d_fd1c_84ba_336d`), the goldens and `bos_eval` are unchanged. The window hash equals the headless hash at every evidence tick (`docs/evidence/bos-art/capture_lines.txt`).

## 1. What is new, what is reused

| Piece | Source | Route |
|---|---|---|
| The ball's body | the existing approved `assets/source/characters/swine/swine-01.ase` ([SWINE-01 spec](swine-01-spec.md)), exported by the new manifest `assets/manifests/swine-01.json` to `generated/art/swine-01.png` (18 frames of 176 x 144, pivot (88, 143), 1-bit alpha). No pixel was changed | reuse + export (medium tier) |
| The chain weight (hung, landed) and the chain tiles | **new**, `scripts/author_bosart.py` writes `assets/source/props/floor01/bos_props.{grid.json,ase}` and its manifest | **route B** (easy prop): drafted by Claude Sonnet, gated and, if need be, frozen by Claude Opus |
| The arches, the lintels, the torches, the copper run, the UV tubes, the girder, the alcove niche | existing catacomb, opening and gym kit pieces; the posts and the lintel are **crops** of `pier_l` (declared as pieces of `floor01_bos_kit.json`, no new pixel) | reuse |
| The strut (the chain weight's post) and the shove's ripple, dust and debris | the Juicer's exported `boss-f01-strut` and `boss-f01-vfx` atlases | reuse (BOS-ART-AD7) |
| The stairwell behind the right wall | the opening kit's `guild_portal_open` drawn over the composer's closed door when the exit is released | reuse |

## 2. Placements

Layout px, y down, walking line y = 420, the gameplay arches at the boss walls 128 and 512 (BOS-AD14: abstract, no collision; posts 16 px at wall -32 and +16, lintel top 112 px over the floor). The contract is `assets/source/terrain/floor01/floor01_bos_kit.json` (extends the gym kit, which extends the opening kit and ENV-01); its `room_dressing.bos` block is applied by `src/ball_kit.rs` with `ring_kit::dress_with` on the composed arena. Gameplay solids, spawns, exit, views and the walking line are the plain arena's (tested).

| Element | Placement | Why |
|---|---|---|
| Arch posts | cap `bos_post_cap` 28 x 19 at y 308, shaft 24 x 77 at y 327, base 28 x 16 at y 404; centres 104 and 152 (left) and 488 (right) | each post covers its 16 px gameplay post and stands on the deck; the right arch's second post is the exit door's own frame (536..) |
| Lintels | `bos_lintel` 40 x 12 pairs at y 296..308: x 88..168 and x 470..550 | spans the 64 px arches (96..160, 480..544), top 124 px over the floor |
| Hero alcoves | the left alcove (28..110) keeps `alcove_niche` at (30, 324), the strut at 52..92 and the copper run at 32..48; the right alcove (530..608) is the exit door's bay | the alcoves are the safe pockets behind the arches; the dressing never narrows them |
| Back wall | the composer's central bay (arch ring, gallery, two piers) is kept; its two torches and the timber door are removed | it is the approved concentric masonry of the concept; the piers bake their own light |
| Deck | the composer's deck, unchanged | |
| Weight | `bos_weight_hung` 80 x 36 on the rack footprint (x 169..249), bottom 189 px over the floor, chain to the girder `gym_girder` x 128..288 at y 186..196 (underside = the sim ceiling, 224) | the sim's rack hang (`HANG_PX`) and footprint (80 x 24) |
| Tie | five `bos_chain_h` tiles from the weight's left edge to the post's top (197 px over the floor) while the weight hangs | reads "the post holds the weight" |
| Copper run | rule C: ceiling collar, 24 tiles of `pipe_v`, one flange, one bracket, `valve_wheel`, floor collar, at x 32..48 | the brass and iron of the concept, one run only |
| UV | two `gym_fluoro_accent` tubes (magenta, 4 lights) at (40, 290) and (572, 262) | limited magenta UV; the arena reads without them |
| Torches | two `torch_wall`, one on each lintel at (122, 240) and (510, 240) with their pools; flame bottom 164 px over the floor | clear of the alcoves, the lane, the weight and any standing spot (the ball is 134 tall, Donut's jump apex 138) |

Validator lessons applied: no flat greybox rectangle once dressed (section 4); no torch flame over a standing spot, an alcove or the roll lane; no decal under structure (the pools are `torch_pool_a`, drawn on the wall behind the bay and the lintels); no background through the piers (the bay is untouched); captions in the evidence only claim what is in frame.

## 3. The state to frame map

The ball's body is one atlas sprite per tick (`src/ball_art.rs::body_frame`, tested against `swine-01.pixelart.json`). It is **never rotated or scaled** (BOS-CUT7): the six roll frames are native cels of the readability set, the sprite is drawn 1:1 at the sim's `boss.x` with pivot (88, 143) and mirrored for a leftward facing.

| `BossMode` | Frame(s) | Tag |
|---|---|---|
| `Dormant`, `Neutral` (standing) | 0, 1, 2 by the sim clock (16 + 12 + 16 ticks) | `idle` |
| `Neutral` (walking, the recentre) | roll frames 3..8, 8 ticks each (BOS-ART-AD2) | `roll` at half rate |
| `Turn` | 0 | `idle` |
| `ChargeTell`, `PoundTell`, `SecondTell` | 9 for the first 8 ticks, then 10 (the coil hold) | `attack_tell` |
| `Charge` (the roll) | 11 for 3 ticks (launch smear), then 3..8, 4 ticks each, looping | `attack_charge`, `roll` |
| `Rebound` (the crash) | 12 (the flat face) for 6 ticks, then 13 | `attack_charge` |
| `Stun` | 13 held (mouths open, the punish window) | `attack_charge` |
| `PoundStrike`, `SecondStrike` (the shove) | 11 for the 3 smear ticks, then 12 | `attack_charge` |
| `PoundRecover`, `SecondRecover`, `DoubleStep` | 13 | `attack_charge` |
| `Roar` | 10 | `attack_tell` |
| `Flinch`, `Backstep` | 14 | `hurt` |
| `Pinned` (under the weight) | 16 for 8 ticks, then 17 (the squashed heap, 64 px, the pinned hurtbox's height) | `down` |
| `Defeated` | by the age of the defeat: 15 (8 ticks), 16 (12 ticks), 17 held; a pinned death is 17 at once | `down` |

The frames carry their own travel: frame 12 sits 25 px forward and frame 13 21 px back, which matches the sim's snap to `crash_pivot` (wall + 64 + 24): the visible front meets the arch on the impact and the rebound is drawn, not teleported.

## 4. The presenter and the greybox

- `src/bin/floor01_scene/ball_art.rs` (Bevy) draws: the body, a pool of 24 sprites for the weight, its chain and its tie (`ball_art::weight_draws`: hung 189 px up with the tie; falling with the chain paying out from the girder; landed on the floor; pinned resting on the heap, AD5), and the open door over the closed one when `boss.released()`.
- A live presenter hides the BOS-A greybox: the body box and its eye, the arch outlines, the label, the rack and strut rects and the ripple rects (`ball_art::greybox_shown`, tested). The strut and ripple stand-ins stay only if their reused Juicer sheet is missing (a dev fallback, never in a full export). With a missing SWINE-01 atlas the scene prints `BALL_ART not drawn` and the greybox stays.
- The windowed scene loads the dressed kit (`boss_scene::load_ball_arena_dressed`); the sim and the replays keep `load_ball_arena` (same solids), so no golden or replay moves.

## 5. Light budget

The composer's four torches (eight lights) are removed, two torches (four lights) and two tubes (four lights) added: the room's emitter count is unchanged against the plain arena, so the worst 640 px view stays inside `MAX_LIGHTS` (tested). The arena reads without the UV (the tubes are accents, 0.12 to 0.3 intensity, not the key light).

## 6. The chain weight key (route B)

`scripts/author_bosart.py`; pins and measurements in [`docs/evidence/bos-art/measurements.txt`](../evidence/bos-art/measurements.txt); the unlit x4 and 1x sheet is `docs/evidence/bos-art/contact_x4_weight.png`. Cool iron (`stone_cold` greys for the slab, `iron` for the chain and ring) and brass only (17 colours of `floor01-catacomb-v2`, no new colour), light from the upper left, no text, no face, no symbol.

| Piece | Size | Content |
|---|---|---|
| `bos_weight_hung` | 80 x 36 | iron slab 80 x 24 (the sim's footprint) on the contact row, chamfered top corners, two brass straps, four rivets, a brass collar (20 x 8) and an iron shackle; the shackle's top is the girder's underside (+1 px) |
| `bos_weight_landed` | 80 x 36 | the same slab; the chain lies in a heap of links on its top face and the collar rests to one side |
| `bos_chain_v` | 8 x 16 | two links (face-on, edge-on), tiles vertically at 16 px |
| `bos_chain_h` | 16 x 8 | the same chain on its side |

The tier flow for this key: drafted by Claude Sonnet (route B), gate and review by Claude Opus, which may edit and freeze it; a key change is a design change and updates `KEY_SHA` in `tests/test_art_bosart.py`.

## 7. Adopted defaults (BOS-ART-AD*)

| ID | Default | Why |
|---|---|---|
| BOS-ART-AD1 | the SWINE-01 atlas is exported from the committed `.ase` unchanged; no new boss key is drawn | the brief: reuse first |
| BOS-ART-AD2 | the walk (the recentre, `Neutral` with movement) shows the roll frames at 8 ticks each (half the roll rate) | the sheet has no walk; the `move` role is the roll |
| BOS-ART-AD3 | the roll starts with the launch smear (frame 11) for 3 ticks | the sheet's own launch frame |
| BOS-ART-AD4 | defeat uses the `down` strip (crack, slump, heap) driven by the age of the defeat, not the death presenter: `rolling_brute` has no species death profile (BOS-AD31), so the merged presenter has nothing to fit; a pinned death is the heap at once | the sheet has the frames; the profile path stays for a later work order |
| BOS-ART-AD5 | while `Pinned` the landed weight rests 40 px up on the heap (its top on the heap's 64 px) and snaps to the floor when the pin ends | reads "pinned under"; the sim rect (floor level) is unchanged |
| BOS-ART-AD6 | `Pinned` shows the slump for 8 ticks, then the heap | no other frame for a squashed ball |
| BOS-ART-AD7 | the strut (3 damage states), the shove's ripple, debris and the crash dust are the Juicer's exported sheets (generic dust and a rotten timber), the other Juicer effects (mirror crack, roar ring, boil, blood pool, rack shadow) are not drawn | reuse first; amends BOS-AD31 for those pieces only |
| BOS-ART-AD8 | the stairwell is the opening kit's open Guild door over the composer's closed one | an existing piece; its lit interior reads as the way down |
| BOS-ART-AD9 | the right alcove is the exit door's bay and the right arch's second post is the door's frame | the composer places the door at 536 |
| BOS-ART-AD10 | the four composer torches are removed and two torches stand on the lintels | no flame over an alcove or the lane; the room stays lit |
| BOS-ART-AD11 | an evidence-only room pilot `bos_weight` (`Pilot::CarlWeight`) hits the post twice from the left alcove and a third time on the crash | the pinned still needs the weight dropped; no fixture, golden or pin selects it |
| BOS-ART-AD12 | gate fix W1: the slab body and its rivets are recoloured from the warm `iron` ramp (it read as a wooden chest) to the cool `stone_cold` greys, with a lit top arris and a dark underside row; the brass straps and collar, the ring and the chain, the shape and the bbox are unchanged; the key is re-frozen and re-pinned | gate (Claude Opus) |

## 8. Open questions (BOS-ART-Q*)

| ID | Question | Default for now |
|---|---|---|
| BOS-ART-Q1 | Is the left alcove readable enough with the niche half behind the strut and the gate? | keep; the niche is a dark recess left of the post |
| BOS-ART-Q2 | Do the arches need a heavier gate or relief (the concept's hanging central gate and pig reliefs)? | no: that is BOS-C (the wide arena) |
| BOS-ART-Q3 | A dedicated descending stair for the exit instead of the open door? | the open door (AD8) |
| BOS-ART-Q4 | A dust trail while rolling and an impact shockwave of the ball's own (the Juicer's generic ripple and dust stand in)? | a later VFX work order (SWINE-01 spec section 1) |
| BOS-ART-Q5 | The magenta UV is two tubes: is that enough, or too much, against "limited"? | two tubes |

## 9. Files

| File | Role |
|---|---|
| `assets/manifests/swine-01.json` | the SWINE-01 runtime atlas manifest |
| `scripts/author_bosart.py`, `assets/source/props/floor01/bos_props.{grid.json,ase}`, `assets/manifests/floor01_bos_props.json` | the weight key |
| `assets/source/terrain/floor01/floor01_bos_kit.json` | the contract and the `bos` dressing block |
| `src/ball_kit.rs`, `src/ball_art.rs`, `src/bin/floor01_scene/ball_art.rs` | the dressing, the frame map and weight geometry, the Bevy presenter |
| `scripts/make_bosart_evidence.sh`, `docs/evidence/bos-art/` | the stills (each captured twice, byte-identical, window hash = headless hash), the contact sheet, the pins |
| `tests/test_art_bosart.py` | the key, contract, manifest and dressing checks |

Designed by Claude Sonnet within the brief (reuse first; the weight key is route B), gate and independent review by Claude Opus, pending.
