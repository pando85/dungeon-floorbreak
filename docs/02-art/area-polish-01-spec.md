# AREA-POLISH-01: dressing for mastery, the Dozer's room shell, the corral's grate and partition burst, and the Juicer's exit lintel

**Status:** executed (medium tier, reuse only); **owner art review pending**, independent top-tier review pending. Nothing here is owner-approved and game feel is the owner's call. This order closes the four "stand-in or missing" cells that the Floor 1 per-room art status (backlog batch 12/13) still lists for rooms 4, 8, 15 and 17.

**Route:** pure reuse, so **no new key and no new pixel** (the medium tier may design a pure reuse, [routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)). Every piece is an approved one: the ENV-01 catacomb kit, the ENV-02 opening kit, the heat (corral) kit, the room 5 gate grate `dfx_gate` (#263), the ENV-02 `timber_splinter_burst` and the gym kit's `gym_gate_lintel`. A gap that truly needed a new key would stop the work as an open question (POLISH-Q*); none did. Designed and executed by Claude Sonnet; the independent review is a separate Claude Opus agent.

**Authorities:** [AREA-ACT1-01](area-act1-01-spec.md) (the act-1 style: bricked bays, no sill under them, torch removals, `room_dressing` blocks applied by `ring_kit::dress_with`), [AREA-TROG-01](area-trog-01-spec.md), [AREA-HEAT-01](area-heat-01-spec.md) (HEAT-OQ1, CD-AD9), [AREA-GYM-01](area-gym-01-spec.md) (GYM-AD2, the #263 review's F3), the [danger FX spec](floor01-danger-fx-spec.md) (`dfx_gate`), the [antihero tone](../01-design/antihero-tone.md), [art direction](art-direction.md). Horizon check: this is presentation of existing rooms, with no move, ability, floor mechanic or co-op change, so [the progression horizon](../01-design/character-progression-horizon.md) is unaffected.

## 1. Contract

Presentation only. The window `COMBAT_STATE` hash equals the headless hash on every captured tick; gameplay solids, `MANIFEST_HASH` (`0xe93d_fd1c_84ba_336d`), the room, sidecar and encounter pins and every golden are unchanged. No collider, trigger, spawn, exit, view or walking line moves (tests POLISH-A1, A2). Palette `floor01-catacomb-v2`, no new colour, no atlas, no tile, nothing collidable.

## 2. Per-room placements

Room px, y down, top-left corners. The walking line is y 580, a view is 640 x 360 with its top at y 292. `bw` = `backwall_decal`, `st` = `structure`, `pr` = `prop`. The blocks are data in `scripts/author_polish01.py`, written to `assets/source/terrain/floor01/floor01_polish_kit.json` (the contract extends the opening kit and re-declares nine heat pieces by the same definitions, the act-1 precedent ACT1-ART-AD13) and applied by `src/polish_kit.rs`.

### 2.1 `mastery` (room 17, the CF02-F microarena, 1152 px)

The arena composes one bay (x 32..336) and a plain brick wall after it. The dressing reprises the floor's arcade as two **blind bays** and the corral and opening-lane language, with the exit gate (#274) and the perches, parapet and goblin lane untouched.

| Layer | Piece | At | Note |
| --- | --- | --- | --- |
| st | `pier_l`, `arch_ring_a`, `pier_r` | (416, 304), (432, 304), (640, 304) | bay 2 on the composer's bay geometry; the backwall under it is brick, so nothing shows through; no `arch_threshold`, no `arch_chain` (no blue sill under a bricked bay) |
| st | `pier_l`, `arch_ring_b`, `pier_r` | (784, 304), (800, 304), (1008, 304) | bay 3, ending at the exit side |
| pr | `torch_wall` x 3 | (440, 448), (696, 448), (800, 448) | clear of the perches (x 544..672, 896..992) and the parapet (736..768) by at least 8 px; test A6 |
| bw | `torch_pool_a_m` x 3 | (336, 364), (592, 364), (696, 364) | the composer's pool rule (torch x - 104, y - 84) |
| bw | `heat_scorch_wall_a`, `heat_scorch_wall_b` | (576, 432), (944, 448) | the corral's wall scorch behind perch 1 and the exit side |
| st | `heat_scorch_floor_a`, `heat_scorch_floor_b`, `heat_ash_floor_b`, `heat_trace_spit_a` | (552, 561), (936, 562), (880, 564), (500, 564) | floor decals between the piers (none touches a pier footprint, test A11); the llama spawns are x 616 and 1000 |
| bw | `wall_cavity` x 2 | (328, 392), (872, 400) | the ENV-02 opening lane's cavity, on plain brick |

The room has **no flat greybox rectangle**: the exit is the #274 gate art and the rest is composed pieces. The **spent drip** is not a stand-in any more: the room holds one Rot Sticker, so the scene spawns the AREA-HAZARDS-01B decal presenter and its approved `dfx_rot_drip` art (#285) replaces the code-drawn drip (test A10; the batch 12 table row was stale).

### 2.2 `dozer` (room 8, the shell around the DOZ-01 rig, 1344 px)

The rig, the rear gate, the lintel tiles, the buffer, the stop frame and the crew are the presenter's and are **not touched**. The shell is dressed in the act-1 chase's vocabulary:

| Layer | Piece | At | Note |
| --- | --- | --- | --- |
| backwall | `backwall_fill` | {384, 352, 208, 208} | bay 2 (x 384..592) is bricked up: its open cells are filled, so no background shows (test A5) |
| st | remove `arch_threshold`, `arch_chain` | (416, 544), (432, 384) | no blue sill and no chain under or over a bricked bay (the upper-row twins at y 272 and 112 stay, out of view) |
| bw | `wall_cavity` | (456, 424) | ENV-02, in the bricked bay |
| st | `heat_scorch_floor_a` | (128, 561) | the corral's floor scorch where the machine comes out of the rear gate, between the piers |

**Out of scope, recorded:** DOZ-02-FU F4 to F7 (faint hull damage at 1x) stays a follow-up of the rig and is not touched here.

### 2.3 `corral` (room 4): the grate and the partition burst

- **M3 gate grate (HEAT-OQ1).** The labelled greybox bars are replaced by the approved room 5 gate grate: the `dfx_gate` closed frame (28 x 224), **centred** on the 16 px collider (2048..2064) and top-aligned on its top (y 352), cropped at the dock's top (y 548, the rows below stand inside the stone), so it spans the lane from the ceiling to the dock. It draws exactly while the sim keeps the block standing (`!exit_gate_open()`), and nothing once it is open. The greybox bars remain only as the missing-atlas fallback; with the art loaded no flat greybox is drawn (test `no_greybox_bar_draws_while_the_grate_art_is_loaded`). The pure half is `heat_kit::gate_art_draw` and `gate_dock_top`, the Bevy half `corral_art.rs`.
- **Splinter burst on the partition's break (CD-AD9, reversed).** The ENV-02 `timber_splinter_burst` (48 x 48, 4 frames of 4 ticks) plays **two staggered bursts** (the second 4 ticks after the first) on the partition's centre line, 14 and 36 px above its foot, from the attempt's `PropPhase::Broken` event, in front of the actors like the Dozer's bursts (z `actor_z + 0.305`). A restart clears the log scan, so an earlier attempt's burst never replays. Presentation only (a function of the event log and the tick); the break keeps its wall-break cue. The pure half is `heat_kit::partition_bursts`, the Bevy half `corral_art::spawn_partition_burst` and `sync_partition_burst`.

### 2.4 `juicer` (room 15): the exit lintel

A second `gym_gate_lintel` (48 x 18, the steel header of the entrance gate, GYM-AD4) caps the exit door at **(592, 306)**: its bottom row is the door's top row (y 324), it overhangs the 20 px door by 28 px to the left and ends at the room's right edge (x 640), and it lies above the right mirror bay (y 326) and the right crack (y 362..395). It is a `structure` piece appended **after** the gate's lintel, which stays the first placement. This closes the #263 follow-up F3 for the exit side.

## 3. Light budget

No new emitter kind. The mastery adds three `torch_wall` (their composer lights), the dozer none. The worst 640 px view of each room, sampled every 32 px along the room, gathers no dropped light and at most `MAX_LIGHTS` (test A7). The corral grate, the burst and the lintel add no light.

## 4. Adopted defaults (POLISH-AD*) and open questions (POLISH-Q*)

Adopted under the owner's design-defaults authorization; the owner may change any of them in a follow-up PR.

| Id | Adopted default | Reason |
| --- | --- | --- |
| POLISH-AD1 | Reuse only, no new key, medium-tier design, independent top-tier review | the brief and the routing rule for pure reuse |
| POLISH-AD2 | The mastery arena's reprise is two blind catacomb bays (pier, ring, pier, no sill, no chain) over the plain wall | it is the floor's arcade, walled off like the chase's S2 bay, and needs no new piece |
| POLISH-AD3 | "The corral (wave 2, llama yard) and the earlier lanes (wave 1, brawl lane)" is read as the heat kit's scorch and ash pieces plus the ENV-02 opening lane's `wall_cavity` | those are the approved static decals of the two lanes; the other kits' props carry state (doors, boxes, hearths) or belong to one room |
| POLISH-AD4 | The M3 grate is `dfx_gate` frame 0, cropped at the dock; an open gate draws nothing | the raised frame hangs from room 5's lintel, and the corral has none, so it would float under the ceiling (seen in a first capture) |
| POLISH-AD5 | The partition burst is two staggered bursts, 14 and 36 px above the foot, in front of the actors | the act-1 barricade pattern (`BARRIER_BURST_Y`, `GATE_BURST_STAGGER`) scaled to a 48 px screen |
| POLISH-AD6 | The Dozer's shell is one bricked bay (with a cavity) and the machine's scorch at the rear gate; the torches are left as composed | the chase's S2 vocabulary on the machine's own room; the stop-frame torches light the weight's read (POLISH-Q2) |
| POLISH-AD7 | The Juicer's exit cap is the gate's own `gym_gate_lintel`, not a new header | reuse; the piece reads as the same steel header |
| POLISH-AD8 | The mastery's spent drip needs no change: the HZB art presenter already covers it (test A10) | the stand-in label only shows where the drip atlas is missing |
| POLISH-AD9 | Floor decals keep clear of the pier footprints | no decal hidden under, or drawn across, structure (test A11) |

| Id | Open question | Default meanwhile |
| --- | --- | --- |
| POLISH-Q1 | Should the mastery reprise other rooms' set pieces (a Boiler weight rig, a hazards acid pocket, a gym mirror bay) as static decor? They are state-bearing or room-specific, so this order did not reuse them | the arcade and the corral scorch only |
| POLISH-Q2 | The Dozer's stop-frame torches at (664, 432) and (704, 448) stand over the frame's hazard rect (x 640..720) in x. They are the composer's, unchanged, and light the weight's telegraph; removing them would dim the hazard read. Keep, or remove with their pools? | keep |
| POLISH-Q3 | The Juicer's cap carries the gate lintel's "gate slot shadow" on its bottom rows, which reads as a header shadow over a door. Does the owner want a dedicated exit header (a new key, a top-tier job)? | the reused cap |
| POLISH-Q4 | The grate art is 28 px wide over a 16 px collider (6 px of jamb on each side), and its foot is a flat cut at the dock. Does the owner want a corral-specific portcullis with a lintel (a new key)? | the reused grate |
| POLISH-Q5 | DOZ-02-FU F4 to F7 (faint hull damage at 1x) | stays recorded, out of scope |

## 5. Acceptance targets

| Id | Target | Check |
| --- | --- | --- |
| POLISH-A1 | Solids, spawns, exits, views, walking line, blink barriers, collision tiles and the room and sidecar content are identical with and without the dressing | `polish_kit` test |
| POLISH-A2 | The state hash is equal on every tick of a 500-tick run, plain and dressed, in both rooms; the window hash equals the headless hash at every captured tick | `polish_kit` test; `scripts/make_polish01_evidence.sh` |
| POLISH-A3 | Every `remove_at` matches exactly one composer piece | `polish_kit` test |
| POLISH-A4 | Every placement is inside the room, on its own piece's layer, and a placement in the floor strip is a `structure` decal | `polish_kit` test |
| POLISH-A5 | No background shows through a bricked bay (the dozer's fill, the mastery's brick under every arcade piece) | `polish_kit` test |
| POLISH-A6 | No torch flame over a standing spot | `polish_kit` test |
| POLISH-A7 | The worst view stays inside the light budget | `polish_kit` test |
| POLISH-A8 | The contract adds no tile, no atlas of its own and nothing collidable | `polish_kit` test |
| POLISH-A10 | The mastery's spent drip is art | `polish_kit` test |
| POLISH-A11 | No floor decal touches a pier | `polish_kit` test |
| POLISH-A12 | The corral grate art covers the collider, stands on the dock and draws nothing once open; no greybox bar draws with the art loaded | `heat_kit` tests |
| POLISH-A13 | The partition break plays two staggered bursts, none before the break or after 24 ticks, none after a restart | `heat_kit` test |
| POLISH-A14 | The Juicer's cap sits on the door's top row, ends at the room edge and clears the mirror and the crack; the gate's lintel stays first | `gym_kit` test |

## 6. Evidence

[`docs/evidence/area-polish-01/`](../evidence/area-polish-01/README.md): window stills at 640 x 360, each captured twice (byte-identical) with the hash-equality line in `capture_lines.txt`, and `SHA256SUMS`. Not run: a lit-read measurement of the new pieces on the stills (the dressing is judged by eye at 1x and by the geometry tests), human recognition and the owner's art review, real-display frame time.

## 7. Files

| Path | Content |
| --- | --- |
| `docs/02-art/area-polish-01-spec.md` | this spec |
| `scripts/author_polish01.py` | the contract and the two `room_dressing` blocks (`check` verifies the file) |
| `assets/source/terrain/floor01/floor01_polish_kit.json` | the contract |
| `assets/source/terrain/floor01/floor01_gym_kit.json`, `scripts/author_gym01.py` | the exit lintel placement |
| `src/polish_kit.rs`, `src/polish_kit/tests.rs` | the composer and its tests |
| `src/cf02.rs` (`load_arena_hooked`), `src/f01_mastery.rs`, `src/f01_dozer.rs` (`launch_with`) | the hooks; `launch` is dressed |
| `src/heat_kit.rs`, `src/bin/floor01_scene/corral_art.rs`, `src/bin/floor01_scene.rs` | the grate art and the partition burst |
| `scripts/make_polish01_evidence.sh`, `docs/evidence/area-polish-01/` | the evidence |
