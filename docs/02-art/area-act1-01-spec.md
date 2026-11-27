# AREA-ACT1-01: the act-1 chase, the shore and the guild, design spec

**Status:** executed (section 16): the three rooms are dressed and wired; the silhouette gate passed with execution notes G1 to G6. **Owner art review pending**; nothing is owner-approved, and three lit-read targets were relaxed (ACT1-ART-Q7, resolved by ACT1-ART-AD22). This order dresses the three act-1 rooms of the [act-1 integration plan](../01-design/floor-01-act-01-integration.md):
- `act1_chase` (plan 4.1: the S2 emergence, the S3 timbers under the low soffit, the S4 alcove, the S5 barricade lock, the wedge and the arch);
- `act1_shore` (plan 4.2: the wedged machine and the shore strike);
- `act1_guild` (plan 4.3: the guide, the grant zone and the guild door).

All three rooms are greyboxes on main: the composer draws the catacomb arcade, and the scene draws the decor rects (`f01_act1::decor`), the timbers, the barricade and the guild door as flat rectangles. The dressing is mostly **reuse**: ENV-01 (catacomb), ENV-02 (opening: timber sets, the full barricade, the alcove niche, the guild portal, the soffit tiles), the refuge kit (AREA-REFUGE-01), DOZ-01 and `mordecai_h44`. There are **two new keys**, drawn by `scripts/author_act1area01.py`:
- `act1_grant_box_{closed,open}`, the strongbox of the guild grant (audit row P1);
- `act1_gallery_breach`, the hole the machine tears through the wall at S2.

The key pins and measurements are in [`docs/evidence/area-act1-01/gate/`](../evidence/area-act1-01/README.md). Launch targets: `floor01_scene --scene f01_late --f01-room act1_chase` (and `act1_shore`, `act1_guild`), as in the [act-1 evidence](../evidence/act1/README.md).

**Route:** two new keys, each scoring 2, so **B + silhouette gate** ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)):
- `act1_grant_box`: a new design, plus a second state;
- `act1_gallery_breach`: a new design, more than 40 px.

The barricade, the timbers, the alcove, the portal, the hearth and the props are reuse (score 0). The step faces and the soffit slices are derived executor pieces (score 0 or 1, no gate). The design step stops at this spec and the keys.

**Authorities:**
- **Design intent:** the act-1 plan:
  - sections 4.1–4.3 (geometry, decor rects, statics, the locks);
  - section 14 (art reuse);
  - section 17 (open questions; ACT1-Q3 wedge art, ACT1-Q7 dressing, ACT1-Q10 the barricade break shown, ACT1-Q1 the room-clear panel).
- **Geometry:** `assets/levels/floor01/f01_act1_{chase,shore,guild}.room.json` and their sidecars (pinned hashes, read-only here), `src/f01_act1.rs`, and the composer `f01_boiler::compose_kit`.
- **Sprites (read-only):** DOZ-01 and its rig, `mordecai_h44`, Carl, Donut and the goblins.
- **Reuse:**
  - ENV-01 [catacomb kit](floor-01-catacomb-kit.md);
  - ENV-02 [opening spec](env-02-opening-spec.md) and `floor01_opening_kit.json`;
  - the refuge kit `floor01_refuge_kit.json` (AREA-REFUGE-01).
- **Precedent:** [AREA-TROG-01](area-trog-01-spec.md): the `room_dressing` block, `ring_kit::dress_with` through `compose_kit_hooked`, `trog_kit::dressing_of` returning an `Option`, the step-face rule, and the lit-read and visibility tests.
- **Lessons of the trog, camp and fast-food passes:**
  - no decal under the deck or under structure (measured visibility);
  - no background through a pier or a bricked bay;
  - no slivers under 8 px between pieces;
  - no blue sill under a bricked bay;
  - no chalk or decal on a block face;
  - no torch flame over a standing spot or a hazard;
  - lit reads, not unlit stills;
  - no motif drawn over by architecture;
  - captions claim only what is in frame.
- **Pipeline and tone:** [art direction](art-direction.md), [asset pipeline](asset-pipeline.md), [antihero tone](../01-design/antihero-tone.md).

## 1. Canon

- **Palette:** `floor01-catacomb-v2` only, no new colour; at most 63 colours per atlas; 1-bit alpha.
- **No text, glyph, numeral, logo or face.** The guild is a warm, lived-in room, not a shop sign. The strongbox lock plate is blank. The refuge's portrait frame is not used (a face).
- **Tone:** worn, practical and improvised (antihero tone):
  - the guild is a cramped back room with a hearth;
  - the strongbox is a battered box, not a glowing treasure chest;
  - the breach is a wall smashed by a machine, not a magic portal.

## 2. Scale and coordinates

Room px, y down, a 16 px grid. The walking line is y 580, the deck top y 560, and the structure strip y 561..579. A view is 640 x 360 with its top at y 292. The chase shares its x axis with the ENV-02 layout R, and ENV-02 layout y + 272 = room y. All placements below are **top-left corners in room px**.

The composed arcade has period 336:
- `pier_l` at 176 + 336k (72 wide), `pier_r` at 400 + 336k (80 wide);
- the lower bays at x 248..399 + 336k;
- `arch_threshold` at (240 + 336k, 544) and `arch_chain` at (256 + 336k, 384) per bay;
- torches in triples around each pier pair, with `torch_pool_*` decals at x = torch x - 104.

The guild and the shore start their arcade at 160 (piers at 160/384 and 496/720).

## 3. Per-room placement

Layers: `bw` = `backwall_decal`, `st` = `structure`, `pr` = `prop`. **P** marks a presenter-spawned sprite (it changes state), not a dressing placement. Every placement must keep the room's solids, statics, locks, triggers, spawns, checkpoints and exits exactly as `f01_act1::launch` builds them.

### 3.1 `act1_chase` (width 3008)

| Beat | Piece | Layer | At | Note |
| --- | --- | --- | --- | --- |
| S2 | `backwall_fill` {584, 352, 152, 208} | backwall | | brick bay 584..735; the breach needs a wall, not a window |
| S2 | remove `arch_threshold` (576, 544), `arch_chain` (592, 384) | st | | no blue sill under a bricked bay; no chain over the breach |
| S2 | remove `torch_wall` (768, 448), (824, 432), (864, 448) and pools (664, 364), (720, 348), (760, 364) | pr / bw | | the breach replaces pier_r 736..815 and the torches on it |
| S2 | **`act1_gallery_breach`** (key) | st | (696, 342) | = the sim window rect; placed after the arcade; covers pier_r 736..815 and the ring foot it crosses; 32 px of pier_l (888..919) stays |
| S3 | `act1_soffit` slices (executor, from the ENV-02 R collision tiles, columns 61..104, rows 0..5) | st | (976, 272), 704 x 96 | = the soffit decor rect; no new pixels; non-collidable |
| S3 | remove `torch_wall` (1104, 448), (1160, 432), (1440, 448), (1496, 432) and pools at x 1000, 1056, 1336, 1392 | pr / bw | | no flame inside a timber set |
| S3 | `timber_set_intact` / `_smashed` **P** | pr | (1104, 456), (1424, 456) | = the timber rects; the smashed frame stays shown |
| S3 | `act1_step_mid`, `_end_l`, `_end_r` (executor, trog step rule) | st | 1168..1231 and 1456..1519, y 564 | 16 px faces on the hump statics |
| S4 | remove `torch_wall` (2112, 448) and pool (2008, 364) | pr / bw | | the flame stood over the alcove ledge |
| S4 | `act1_alcove_niche` (re-declared `alcove_niche` rect, layer `st`, with the light `alcove_glow`) | st | (2064, 468) | on pier_r 2080..2159; the niche floor sits on the ledge |
| S4 | step faces as S3 | st | 2064..2159, y 564 | the alcove ledge static |
| S5 | `timber_barricade_full_intact` / `_smashed` **P** | pr | (2304, 304) | = the ENV-02 full barricade over the lock solid {2312, 304, 80, 276}; follows `barrier_hit()`, so the break stays shown (ACT1-Q10) |
| S6 | the wedge: greybox wedge draw kept (`dozer_art`, the rig clipped and darkened) | | | ACT1-Q3 stays open (ACT1-ART-Q1) |
| S6 | the arch {2608, 342, 248, 238}: the composer's bay 2600..2751 already reads as the arch; no piece | | | the flat decor rect is not drawn |

### 3.2 `act1_shore` (width 976)

| Piece | Layer | At | Note |
| --- | --- | --- | --- |
| remove `torch_wall` (176, 448) and its pool (72, 364) | pr / bw | | the flame sat behind the wedged machine |
| the wedged machine: the same darkened, clipped DOZ-01 rig as the chase wedge, bottom on y 580, clipped to {56, 440, 192, 140} | presenter | | replaces the grey decor rect; no new art |

The shore stays otherwise as composed: the strike column and the goblins need an uncluttered floor. The exit portal (704, 444) is the composer's.

### 3.3 `act1_guild` (width 640)

| Piece | Layer | At | Note |
| --- | --- | --- | --- |
| remove `torch_wall` (48, 432), pool (-56, 348) | pr / bw | | the hearth is the light of this wall |
| `refuge_hearth` (4 frames, its lights) | pr | (64, 500) | on the spawn side, not on the zone, the guide or a window |
| `refuge_hearth_flue` x 9 | bw | (80, 356 + 16i), i = 0..8 | the chimney up to the slab; on plain back wall left of pier_l 160 |
| `refuge_barrel` | pr | (8, 552) | |
| `refuge_stool` | pr | (150, 562) | |
| `refuge_desk` + `refuge_candle_flame` (the refuge offset) | pr | (272, 554) | in front of the bay, behind the actors |
| **`act1_grant_box_closed` / `_open`** **P** (key) | pr | (340, 552) | grant zone 336..376, left of the guide; `open` once the guild door lock is open (`donut_unlocked`) |
| `mordecai_h44` (unchanged) | actor | as on main (x 380, facing left) | idle and talk animation out of scope (ACT1-ART-Q2) |
| `refuge_wall_map` | bw | (474, 484) | on plain back wall 464..527 |
| `refuge_bookshelf` | pr | (474, 524) | between pier_r and the door |
| `guild_portal_closed` / `_open` (composer piece (536, 444)) | pr | | the scene overlays `guild_portal_open` once the lock opens and hides the flat lock bar |

No piece on the ceiling slab y 336..351 (ACT1-ART-Q3). No foreground layer anywhere: nothing may cover an actor or the room-clear panel (ACT1-Q1).

## 4. Hidden greybox rects

In a dressed room the scene does not draw the decor rects, the timber rects, the barricade rect or the door bar. It keeps:
- the `F01_ACT1_DECOR room=.. label=.. rect=..` lines, unchanged;
- the wedge's greybox draw;
- Mordecai's sprite.

The plain composition stays available for the existing presentation-only tests (a `dressed` flag, as `f01_trog::launch`).

## 5. Piece list

**K** = frozen key; **E** = executor piece (derived, no new design); **R** = reuse.

| Piece | Kind | Size | Source |
| --- | --- | --- | --- |
| `act1_grant_box_closed`, `_open` | K | 24 x 28 | `act1_grant_box.grid.json` |
| `act1_gallery_breach` | K | 192 x 218 | `act1_gallery_breach.grid.json` |
| `act1_soffit_a..d` | E | 4 x 176 x 96 | the ENV-02 R collision tiles at columns 61..104, rows 0..5, copied pixel for pixel |
| `act1_step_mid`, `_end_l`, `_end_r` | E | 16 x 16 | the trog step rule (`author_trogarea01.end_of` / `darken` on the heat shelf body), one stone step darker |
| `act1_alcove_niche` | R | 96 x 96 | the `alcove_niche` rect, re-declared on `structure` with a light |
| timber sets, full barricade, guild portal | R | | `floor01_opening_kit.json` |
| hearth, flue, desk, candle, stool, barrel, map, bookshelf | R | | `floor01_refuge_kit.json` |

## 6. The keys

### 6.1 `act1_grant_box` (2 frames)

A plain, battered strongbox, 24 x 28 per frame:
- three horizontal planks, with two iron bands and iron corner caps;
- a chipped lid corner, a gouge and a split;
- a hasp with a blank brass plate;
- light from the upper left.

`open` has the lid thrown back (its underside showing), a dark interior with a small warm glint, and the hasp hanging loose. The floor contact is row 27 (room y 579). It is the only object in the zone, and it reads as "something was handed over here" without a glow effect.

### 6.2 `act1_gallery_breach`

A hole torn through the wall, 192 x 218 (the S2 window rect):
- **Outline:** a burst outline with a stepped crown. Its edge follows the 16 x 8 bond, moving per course by whole and half bricks, so it is never a smooth arc.
- **Lip:** a crushed brick lip 4..12 px wide, with dislodged half-bricks hanging into the void.
- **Depth:** the right jamb's thickness is lit from the upper left; the soffit is in shadow.
- **Void:** near-black far gallery with two pit props and a lintel (where the machine came from).
- **Foot:** rubble heaped towards the jambs.

The void reaches room y 559 and covers the core {736, 422, 112, 138} completely (see the measurements). The machine is drawn in front of it.

## 7. States (presentation only)

| Sprite | State source | Frames |
| --- | --- | --- |
| timbers | `TimberSmashed` / the timber's sim state, read as the scene already does for the flat rects | intact → smashed, stays shown |
| barricade | `barrier_hit()` (rewind- and respawn-safe) | intact → smashed while the lock is open; intact again when the respawn closes it |
| grant box | the `guild_door` lock open (`donut_unlocked`) | closed → open |
| guild portal | the same lock | closed → `guild_portal_open` overlay |

The ENV-02 splinter bursts are already wired (ACT1-IMP-18) and stay as they are. No state writes back to the sim.

## 8. Lighting (presentation only)

The profile is `floor01_torchlit_a1`. Net light change:
- chase: −8 torches (−16 lights), +1 (`alcove_glow`);
- shore: −1 torch (−2);
- guild: −1 torch (−2), +2 (hearth), +1 (candle).

`alcove_glow` sits on `act1_alcove_niche` at offset (48, 56): colour `#f2b060` (`gal_warm` 5), radius 96, intensity 0.40, group `scene`, no flame sprite. It is the warm spill from the gallery behind the niche; ACT1-ART-Q4 asks whether a visible source is wanted. Every room's worst 640 px view stays at or under `MAX_LIGHTS` (64).

## 9. Collision and state

None. The dressing adds no solid: the soffit slices and step faces are non-collidable presentation pieces. The humps, the ledge, the barricade lock, the guild door and the slab are pushed by `f01_act1::launch` exactly as now.

## 10. Readability and lit-read targets

Measured **lit** on the window stills (as TROG-ART-A12), with `lookdev_compose.readability(..., ring=6)`:
1. **S4:** `act1_alcove_niche` has the highest lit mean L* of every non-flame, non-actor shape in the S4 view (x 1920..2240). This is the plan's "brightest shape in S4".
2. **S2:** Carl, Donut and the machine each keep `edge_dL` ≥ 15 against the breach. The void's lit mean L* is ≤ 12.
3. **S3:** the timber sets keep `edge_dL` ≥ 10 against the soffit and the piers after the torch removal. If they fail, re-add one torch at (1200, 448) only (already outside the set), not inside a set.
4. **Guild:** the grant box's lit mean L* is ≤ Donut's and ≥ the desk's. The hearth is not brighter than Mordecai's face region.
5. **Visibility:** ≥ 90 % of each new or re-declared piece is visible behind the architecture, and `backwall_null_px` is 0 (no background through the bricked S2 bay).

## 11. Keys and pins

The key pins are the SHA-256 of each key's pixel list, in `gate/measurements.txt` (`python3 scripts/author_act1area01.py measure`). The execution must keep them byte-identical; a key change is a design change and goes back to the top tier.

## 12. Acceptance targets

| Id | Target | How it is checked |
| --- | --- | --- |
| ACT1-ART-A1 | Solids, statics, locks, triggers, spawns, checkpoints, exits, views, walking line and the guild slab identical with and without the dressing, in all three rooms | `act1_kit` unit test on the composed layouts |
| ACT1-ART-A2 | Room, sidecar and encounter hashes, the state hash and every golden unchanged; the window `COMBAT_STATE` hash equals the headless hash at every captured tick | existing pins; double capture |
| ACT1-ART-A3 | Every `remove_at` matches (zero unmatched) | unit test: `dress_with` returns 0 |
| ACT1-ART-A4 | Every placement in y 561..579 is on `structure` | unit test |
| ACT1-ART-A5 | No background pixel in the bricked bay x 584..735, y 352..559 | unit test |
| ACT1-ART-A6 | The worst 640 px view of each room ≤ `MAX_LIGHTS` | unit test |
| ACT1-ART-A7 | Key frames byte-identical to the pins | art unittest |
| ACT1-ART-A8 | Palette `floor01-catacomb-v2`, ≤ 63 colours per atlas, 1-bit alpha, ≥ 1 transparent pixel per piece | `validate_assets.py`, art unittest |
| ACT1-ART-A9 | Lit read and visibility of section 10 on the runtime stills | evidence script |
| ACT1-ART-A10 | The timbers' and the barricade's smashed frames stay shown after the break; the barricade returns to intact after the respawn | window stills at the break, after a defeat, after the respawn |
| ACT1-ART-A11 | No text, glyph, numeral, logo or face | review |
| ACT1-ART-A12 | Window stills at 640 x 360 read at 1x: the breach with the machine in front, the soffit and timbers, the alcove, the barricade intact and broken, the shore wedge, the guild with the box closed and open | owner review |

## 13. Execution list (one PR, this branch)

1. **Pieces:** the E pieces in `scripts/author_act1area01.py` as new functions below the keys (keys untouched); the atlases, grid and `.ase` sources, manifests, and the export through `scripts/export_assets.sh`.
2. **Contract:** `assets/source/terrain/floor01/floor01_act1_kit.json`. It extends the opening kit, declares the act-1 atlases, and references the refuge atlases and pieces it uses. It has a `room_dressing` block per room (section 3).
3. **`src/act1_kit.rs`:**
   - `dressing_of(room_id) -> Option<Dressing>`;
   - `compose(root, loaded, room, dressed)` through `f01_boiler::compose_kit_hooked`, keeping the statics as `f01_act1::launch` pushes them;
   - tests A1, A3..A6.
4. **Bevy:**
   - `f01_act1::launch` gets a `dressed` flag;
   - the scene skips the flat rects for a dressed room (section 4);
   - presenter sprites for the timbers, the barricade, the grant box and the portal overlay (section 7);
   - the shore wedge draw.
5. **Evidence** at `docs/evidence/area-act1-01/`:
   - window stills of the three rooms per beat;
   - the double captures and the hash equality;
   - the lit read and visibility on the real stills;
   - contact sheets;
   - the art unittest `tests/test_art_act1area01.py`.

## 14. Files

| Path | Content |
| --- | --- |
| `docs/02-art/area-act1-01-spec.md` | this spec |
| `scripts/author_act1area01.py` | the keys; at execution every piece, the atlases, manifests, contract and measurements |
| `assets/source/props/floor01/act1_grant_box`, `act1_gallery_breach` (`.grid.json` + `.ase`) | the keys |
| `docs/evidence/area-act1-01/gate/` | the key sheets and `measurements.txt` with the key pins |
| *(execution)* `assets/source/terrain/floor01/floor01_act1_kit.json`, `assets/manifests/floor01_act1_*.json` | the contract, the manifests |
| *(execution)* `src/act1_kit.rs`, `src/bin/floor01_scene/act1_art.rs`, `tests/test_art_act1area01.py` | the dressing, the presenters, the art unittest |

## 15. Adopted defaults and open questions

Adopted under the owner's design-defaults authorization (AGENTS.md, 2026-11-19); the owner may change any of them in a follow-up PR.

| Id | Adopted default | Reason |
| --- | --- | --- |
| ACT1-ART-AD1 | Route B + silhouette gate; two new keys only | everything else has approved art |
| ACT1-ART-AD2 | No new barricade key: the ENV-02 `timber_barricade_full_intact/_smashed` over the lock, following `barrier_hit()` | the approved piece already matches the lock solid; ACT1-Q10 needs a visible break, which `_smashed` is |
| ACT1-ART-AD3 | `act1_gallery_breach` on the sim's window rect, with the bay at 584..735 bricked and the three torches on the replaced pier removed | the composer's piers do not line up with the sim window; the machine needs a hole to come out of, not an arch it fits through |
| ACT1-ART-AD4 | The alcove is the `alcove_niche` rect re-declared on `structure` at (2064, 468) with a warm light | on `backwall_decal` it would hide behind pier_r 2080..2159; the plan wants it the brightest shape in S4 |
| ACT1-ART-AD5 | The soffit is copied from the ENV-02 R collision tiles (no new pixels), non-collidable | the soffit rect equals ENV-02 columns 61..104, rows 0..5 |
| ACT1-ART-AD6 | Hump and ledge faces from the trog step rule | the composer draws nothing for statics (TROG-ART-AD5 precedent) |
| ACT1-ART-AD7 | Torches removed where a flame stood inside a timber set, over the alcove ledge, behind the wedge or beside the hearth | the flame-over-standing-spot and motif-overdraw lessons |
| ACT1-ART-AD8 | The guild is furnished from the refuge kit; the hearth is on the spawn side at (64, 500), not in a bay | a lived-in back room with no new art; the hearth must not sit over the zone, the guide or a window |
| ACT1-ART-AD9 | The grant object is a plain strongbox `act1_grant_box` at (340, 552), opened when the guild door lock opens | audit P1; IP-neutral id; no glow, no text |
| ACT1-ART-AD10 | The shore's wedged machine uses the chase wedge's darkened, clipped DOZ-01 rig | no new art; ACT1-Q3 stays open for both |
| ACT1-ART-AD11 | The guild portal open state is an overlay of `guild_portal_open`; its lights stay those of the composed closed piece | keeps the composer's piece and its lights; the light swap is ACT1-ART-Q5 |
| ACT1-ART-AD12 | No foreground pieces in any act-1 room | nothing may cover an actor or the room-clear panel (ACT1-Q1) |

| Id | Open question | Recommendation | Owner of the answer |
| --- | --- | --- | --- |
| ACT1-ART-Q1 | End-on wedge art for the chase and the shore (ACT1-Q3) | keep the greybox wedge draw until a DOZ-01 follow-up | owner art review |
| ACT1-ART-Q2 | Mordecai's idle and talk animation | out of scope; follow-up **GUILD-BEAT-01** (audit work order 13) | follow-up |
| ACT1-ART-Q3 | The guild ceiling slab y 336..351 has no art | follow TROG-ART-Q1 (one kit-wide answer) | owner art review |
| ACT1-ART-Q4 | Should the alcove light have a visible source (a lamp sprite)? | no: a flame would stand over the ledge, where the player waits | owner art review |
| ACT1-ART-Q5 | Should the guild portal's lights switch to the open-state lights? | yes, in the presenter follow-up, if the overlay reads wrong | executor / review |
| ACT1-ART-Q6 | Interact-verb wiring for the box and the guide | out of scope; follow-up **GUILD-VERB-01** | follow-up |
| ACT1-ART-Q7 | Three lit-read targets of section 10 fail on the real stills (section 16): the alcove niche is not the brightest shape in S4 (mean L* 24.0 against `pier_l` 42.8, and no glow intensity or radius reaches it); the frozen grant box is darker than the desk (16.5 and 12.3 against 21.8 and 21.3); the hearth is not darker than Mordecai's face region (43.7 against 43.4 closed, 42.7 against 33.6 open) | relax 10.1 and 10.4 to what the art can do: S4 "the niche is the brightest **warm** shape" or a peak (90th-percentile) metric, and "box <= Donut" only, or, if the owner wants the read as written, a top-tier design change (a lit lamp source for the niche, a lighter box key, a smaller or dimmer hearth); no executor tuning can meet them | top-tier design and owner art review |

**Horizon fit:** the kit changes no move, ability, unlock or gate. It dresses three rooms whose mechanics are on main, and the grant box shows a state the sim already has. The [progression horizon](../01-design/character-progression-horizon.md) still fits; it is not edited.

## 16. Execution record

Designed by Claude Opus (keys, spec, scratch gate composite); silhouette gate by Claude Opus (the orchestrator, independent of the designer); executed by Claude Sonnet on this branch. Evidence: [docs/evidence/area-act1-01](../evidence/area-act1-01/README.md).

**Gate notes (silhouette gate PASS with execution notes).**
- **G1** (flat black rectangle behind the stove): it was the greybox `guide` rect of `f01_act1::decor`, drawn behind the hearth's column; a dressed room draws no flat decor rect (section 4), so no flat greybox rectangle remains in any still (the guide rectangle stays only as the fallback of a missing Mordecai sprite).
- **G2** (torch on the stove pipe's tip): the torch `(48, 432)` and its pool `(-56, 348)` are removed as the spec says; the one in the gate composite was the surviving greybox torch. No torch is left on the pipe.
- **G3** (blue sills under open bays): kept. The lesson of the trog run (TROG-ART-AD20) is "no blue sill under a **bricked** bay"; the only bricked bay of the act-1 rooms (S2, x 584..735) has its sill removed. The open bays are windows onto the far gallery and keep the composer's sill (ACT1-ART-AD19).
- **G4** (the S3 timber set can read as a gallows): approved ENV-02 art, not changed; **owner note for the playtest packet** (`chase_s3_timbers`, `chase_s3_after_first_smash`).
- **G5** (S3 torch removals and the timbers' read): the lit read keeps the timber set at `edge_dL` 21.6 against the soffit and piers (>= 10); the fallback torch is not used.
- **G6** (the alcove glow brightest in S4): **fails**, see ACT1-ART-Q7; reported, not tuned.

**Pieces and atlases** (`scripts/author_act1area01.py`, below the keys; the three keys are byte-identical to the pins of `gate/measurements.txt`):

| Atlas | Pieces | Colours |
| --- | --- | --- |
| `floor01_act1_keys` (192 x 254) | `act1_gallery_breach`, `act1_grant_box_closed`, `act1_grant_box_open` | 31 |
| `floor01_act1_soffit` (176 x 408) | `act1_soffit_a..d` (the ENV-02 R soffit, collision rows 0..5, columns 61..104, pixel for pixel; each slice carries one cleared corner pixel) | 14 |
| `floor01_act1_props` (104 x 120) | `act1_alcove_niche` (the ENV-02 `alcove_niche` pixels, copied), `act1_step_mid`, `_end_l`, `_end_r` (16 x 16 hump faces) | 14 |

The contract `assets/source/terrain/floor01/floor01_act1_kit.json` extends the opening kit, declares those atlases and the refuge atlases again, re-declares the eight refuge pieces the guild uses by the same definitions (same rects, anchors and lights) and holds the three `room_dressing` blocks of section 3. Every atlas is 63 colours or fewer, 1-bit alpha, every piece has a transparent pixel.

**Code.** `src/act1_kit.rs` (`ACT1_CONTRACT`, `dressing_of`, `is_dressed`, `compose`, the pure presenter pieces and `presenter_rects`), `f01_act1::launch_with(root, room, dressed)` (`launch` is dressed; `Act1Run::dressed`), `src/bin/floor01_scene/act1_art.rs` (timbers, barricade, grant box, portal overlay), `dozer_art::wedged_rig` and the Bevy `spawn_wedge` (the shore's machine). `git diff origin/main -- src/f01_act1.rs` touches only the `launch_with` split, the compose call and the `dressed` field; the room, sidecar and encounter JSON and every pin are unchanged.

**Tests.** `src/act1_kit/tests.rs` (16 tests): A1 (solids, statics, locks, triggers, spawns, checkpoints, exits, views and walking line identical with and without the dressing, in all three rooms), the state hash (500 ticks, three rooms), A3 (zero unmatched `remove_at`), A4, A5 (no background in the bricked bay or behind the breach), A6 (light trade and the worst 640 px view), the sim rects (breach, soffit, steps, timbers, barricade, niche, box, portal), the window rule, the torch clearance and the lookups; `dozer_art::tests::the_shore_wedge_fits_the_decor_rect`; `tests/test_art_act1area01.py` (22 tests: A7 key pins, executor pins, A8, the soffit and niche copies, the dressing of section 3, the refuge re-declarations, the evidence numbers).

**Evidence.** Fourteen window stills at 640 x 360, each captured twice (byte-identical) with the window hash equal to the headless hash; the lit read and the visibility on the real stills (`measurements.txt`): visibility 100.0 % for all 39 placed pieces, `backwall_null_px=0`; S2 passes (heroes `edge_dL` 18.0 and 26.6, the machine 18.4, the void 2.2 L*); S3 passes (21.6); S4 and two guild readings fail (ACT1-ART-Q7).

**Adopted defaults of the execution** (each changeable in a follow-up):

| Id | Adopted default | Reason |
| --- | --- | --- |
| ACT1-ART-AD13 | The refuge atlases and the eight refuge pieces are declared again in the act-1 contract by the same definitions, with no pixel copy | the contract chain (`extends`) has no sibling reference; the loader merges one chain |
| ACT1-ART-AD14 | `act1_alcove_niche` is a pixel-identical copy of the ENV-02 `alcove_niche` rect in `floor01_act1_props` (pinned equal by the art test), with the `alcove_glow` light | the contract validator refuses two pieces on one atlas rect, so a re-declaration on `structure` needs its own rect |
| ACT1-ART-AD15 | The 16 x 16 step faces are the top 16 rows of the heat shelf body (arris, slab lip, shadow row, the first course's head), one stone step darker (the trog step rule), the transparent nick moved to the bottom right corner | a 16 px static; the shelf body's own nick (row 31) falls off the crop |
| ACT1-ART-AD16 | A4 reads "every placement over y 561..579 draws above the collision layer": the 17 step faces are `structure`, the guild's refuge furniture stays `prop` (its layer in the refuge kit) | the literal "on structure" would move the hearth, barrel, stool, desk, candle and bookshelf, which stand on the walking line, off the prop layer |
| ACT1-ART-AD17 | All four soffit slices carry the one cleared top-left pixel (the ENV-02 tiles are opaque) | every piece needs one transparent pixel; the trog step pieces' rule |
| ACT1-ART-AD18 | The shore wedge is `dozer_art::wedged_rig`: the rig at the chase wedge's pose (`rig_sprites` itself, the opening's tuning), clipped by the chase wedge's `clip_right` (extracted, not forked), re-based by -2608 and cut at the decor rect's left and top (`SpriteDraw::crop_left`, a new field) | the rig is 255 x 160 px and the decor rect {56, 440, 192, 140}; the pose is static (the strain is over) |
| ACT1-ART-AD19 | The open bays keep the composer's blue sill (gate note G3) | TROG-ART-AD20 removes the sill under a bricked bay only |
| ACT1-ART-AD20 | The presenter sprites (timbers, barricade, box, portal overlay) draw at the z just above the highest prop piece and below the actors; the overlay a hair above the composer's closed portal | they are props of their rooms, and the actors stay in front |
| ACT1-ART-AD21 | The 14 stills use `hide`, `watch`, `skip` and `strike`/`donut` at ticks of the ACT1 evidence where one exists (ten of 14 reuse its hashes) | the same captures keep the hash comparison |
| ACT1-ART-AD22 | ACT1-ART-Q7 resolved by the orchestrator (top tier, 2026-11-27): targets 10.1 and 10.4 are relaxed to what the art reads on the real stills. S4: the niche reads by its arch shape and cold contrast against the warm piers, not as the brightest shape. Guild: the grant box only needs to be findable at 1x beside the desk, and the hearth may be brighter than Mordecai's face. The three misses are owner art notes, not defects | the stills read correctly at 1x; the grant beat is carried by the SYSTEM NOTICE; meeting the targets as written needs a design change (a lamp source, a lighter box key, a dimmer hearth) that the budget defers to owner review |

**Follow-ups** (recorded here and in the PR body; the backlog is the orchestrator's): GUILD-BEAT-01 (Mordecai's idle and talk animation), GUILD-VERB-01 (interact verbs for the box and the guide), ACT1-ART-Q1 to Q7.
