# LLAMA-03: Bad Llama rush (brace, scrape, run, recover), design spec

**Status:** design keys authored (frames 1, 2, 4 and 8 of the rush atlas); K1 and K2 applied at the silhouette gate. The ten-frame atlas, manifest, Bevy wiring and runtime evidence are executed (section 11); the independent review and the owner's art and game-feel review are pending. Nothing here is accepted: the owner judges the rush by playing it.

Scope: the production art for the corral rush ([CRL-Q39](../01-design/enemy-gameplay-llama-rush.md)), replacing the CORRAL-E code-drawn stand-in's **body** (CRL-Q38) on the merged rush state. The rush mechanics, data and timings (`assets/data/gameplay/enemy_llama_rush_v0.json`, [enemy-gameplay-llama-rush.md](../01-design/enemy-gameplay-llama-rush.md)) are unchanged; this is presentation only.

Sources: [`llama_rush_keys.grid.json`](../../assets/source/characters/llama/rush/llama_rush_keys.grid.json) (with its `.ase`), written by [`scripts/author_llama03_rush.py`](../../scripts/author_llama03_rush.py) with the LLAMA-01 parametric rig. Gate images: [docs/evidence/llama-03-rush/](../evidence/llama-03-rush/README.md). Tests: `tests/test_art_llama03_rush.py`.

## 1. Canon and tone

- The llama is the [LLAMA-01](llama-01-spec.md) Bad Llama, unchanged: 64 px, mangy tan wool with mange streaks, the long neck, long ears, big yellow buck teeth. The rush must not redesign it.
- **The rush is a body attack, never a spit.** The spit tells by raising the head high and lighting the throat (LLAMA-01 frames 7 to 10). The rush says the opposite with the whole body: the head goes **down and forward, level with the chest**, the throat stays plain fur, no frame carries a glow key (r, R, O, Y). A player who has learned "head up and glowing means spit" reads "head down and pawing means it is coming at me".
- **Llama aggression, not a bull costume.** Real llamas charge with the ears pinned flat back and the tail raised; that is the anger cue on every brace, scrape and run frame. The hoof scrape borrows the bull's paw only as a one-beat gesture.
- [Antihero tone](../01-design/antihero-tone.md): a dumb, mean farm animal throwing its weight around. The run is ungainly and heavy, a long low battering ram with legs flailing, never a sleek galloping predator. The comedy is in the stop: the llama skids on stiff fore legs, its head pitches forward by inertia and it stands blown, tongue out, squinting. Never heroic, never cute.
- Fan-concept layer: the llama is the replaceable fan-concept layer; the runtime contract (rush phase to frame) is IP-neutral.

**Not decided by this spec:** any rush timing, speed, box, damage or cooldown (CORRAL-B data, unchanged), sound, camera shake, a lane telegraph in the game (see LL3-AD7).

**Living horizon:** this is art for an already-merged enemy move. It adds no move, unlock or ability, so it does not change the [character progression horizon](../01-design/character-progression-horizon.md).

## 2. Frame contract

Same as LLAMA-01, so the rush atlas swaps in with no anchor work:

| Item | Value |
| --- | --- |
| Frame | 96 x 72 |
| Pivot | (44, 71): body centre column, contact row |
| Contact row | 71 on every frame (all rush frames are grounded: at least one hoof stamp on rows 68 to 71) |
| Facing | authored facing right; the runtime flips around the pivot for facing < 0 (mirrored pivot x 51) |
| Layers | one, `body` |
| Palette | `llama-floor01-v0`, the 17 LLAMA-01 keys, unchanged; no new colour |
| Rush box (reference) | x [0, 28) ahead of the pivot, heights [2, 30) above the contact row, so frame columns 44 to 71 and rows 42 to 69 facing right |

## 3. States and the tick map (60 Hz, read from `RushState`)

The rush state (`src/llama_rush.rs`) holds `phase`, `tick` (attack tick since the tell start), `active` (committed ticks done) and `left` (recover or bonk ticks still to run). The frame is a pure function of those fields; the art never feeds back into the simulation.

| Phase | Ticks (data) | Frames |
| --- | --- | --- |
| Tell | 32 (`crlq06_tell_ticks`), tell ticks 0 to 31 | 0-3 `brace_in`; 4-7 `brace`; 8-11 `scrape`; 12-15 `brace`; 16-19 `scrape`; 20-23 `brace`; 24-27 `scrape`; 28-31 `coil` |
| Committed | up to 24 (`crlq07_active_ticks`), 4 px a tick; may end early on contact | `RUN[(active / 3) % 4]` with `RUN = [6, 7, 4, 5]` (push, reach, contact, gather): 3 ticks a frame, a 12-tick stride of 48 px |
| Recover | 40 (`crlq09_recover_ticks`); elapsed = 40 - `left` | elapsed 0-9 `recover_skid`; 10-39 `recover_blown` |
| Bonk | 60 (`crlq10_bonk_ticks`); elapsed = 60 - `left` | elapsed 0-11 the LLAMA-01 `hurt` frame (11); 12-59 `recover_blown` (LL3-AD1) |

The scrape beats are exactly the stand-in's scrape half-beats (`tick >= 4 && tick % 8 < 4`), so the art paw and the dirt kick land on the same ticks. A tell interrupted by a hit leaves the rush (the llama is in `Hurt` mode) and shows the LLAMA-01 hurt frame as today.

```mermaid
stateDiagram-v2
    [*] --> brace_in: tell tick 0
    brace_in --> brace: tick 4
    brace --> scrape: ticks 8, 16, 24
    scrape --> brace: ticks 12, 20
    scrape --> coil: tick 28
    coil --> run: commit (tell tick 32)
    state run {
        push --> reach --> contact --> gather --> push
    }
    run --> recover_skid: whiff, hero hit or leash stop
    recover_skid --> recover_blown: elapsed 10
    run --> hurt11: bonk (solid or partition)
    hurt11 --> recover_blown: elapsed 12
    recover_blown --> [*]: recover or bonk ends
```

## 4. Frame list (rush atlas, 10 frames)

| # | Name | Tag | Author | Pose |
| --- | --- | --- | --- | --- |
| 0 | `brace_in` | `rush_brace` | executor | body and head halfway between the LLAMA-01 idle and the brace (hind legs already braced): the head on its way down |
| 1 | `brace` | `rush_brace` | **key** | braced, head down the lane, ears pinned, tail up |
| 2 | `scrape` | `rush_brace` | **key** | the brace with the near fore hoof pawing back |
| 3 | `coil` | `rush_brace` | executor | the brace 1 px lower, head drawn back 2 px: the gather before the launch |
| 4 | `run_contact` | `rush_run` | **key** | near fore strikes far forward, near hind flung back |
| 5 | `run_gather` | `rush_run` | executor | near fore vertical under the shoulder, near hind swinging under, far hind toeing off |
| 6 | `run_push` | `rush_run` | executor | near hind planted under the barrel, far hind swinging forward, both fores folded up |
| 7 | `run_reach` | `rush_run` | executor | far fore striking under the chest, near fore reaching, near hind kicked back |
| 8 | `recover_skid` | `rush_recover` | **key** | fore legs jammed forward, rump down, head pitched down, mouth open |
| 9 | `recover_blown` | `rush_recover` | executor | legs back under, head hanging 2 px lower, tongue out, squint |

The far legs of frames 5, 6 and 7 are the gate's correction B1: tracked in world space at 12 px per frame, the far fore strikes in frame 7 and lifts to the key's position at the contact, and the far hind lands at the contact and toes off in frame 5, so no planted hoof skates.

Tags (the manifest's `animation_tags`): `rush_brace` 0 to 3, `rush_run` 4 to 7, `rush_recover` 8 to 9. Frame durations in the grid follow the tick map for previews (67 ms on 0 to 3, 50 ms on 4 to 7, 167 ms on 8, a 500 ms review hold on 9); the runtime picks frames from the tick map, not from durations.

## 5. Key poses (as authored)

All four are rig poses (`L.shifted` over the LLAMA-01 `IDLE` or another key) in `scripts/author_llama03_rush.py`; the pose dicts are the source of truth. Measured bboxes (frame coordinates, facing right):

| Key | Atlas # | Bbox x | Bbox y | Silhouette read |
| --- | --- | --- | --- | --- |
| `brace` | 1 | 8-85 | 24-71 | a long horizontal wedge: tail up at the back, head low at the front, fore legs splayed forward stiff, hind legs coiled |
| `scrape` | 2 | 8-86 | 24-71 | the brace with a gap under the chest closed by the near fore drawn back (hoof x 50, 11 px behind its brace position), head 1 px lower, body rocked 1 px forward; the hind hooves stay on the brace footprint (K2: rows 66 to 71, columns 0 to 39 equal the brace) |
| `run_contact` | 4 | 5-89 | 27-71 | the longest, lowest shape: near fore reaching to x 71, near hind flung back to x 11, head at chest height out to x 89 |
| `recover_skid` | 8 | 11-81 | 32-71 | a hunched lump: rump low, back rising to the withers, head pitched down below the withers, fore legs braced ahead |

Details:

- **Brace:** chest down 2 px from the idle, back sloping to the front, long neck run level so the skull (72, 37.5) sits about 2 px below the withers; clenched teeth, angry eye, ears pinned along the neck (`ear_ring`), tail raised. The fore hooves are planted ahead (x 61 and 66), the hind hooves under the rump (27 and 32).
- **Scrape:** the brace with the near fore leg drawn back under the chest, knee bent, hoof dragged on the floor at x 50 (11 px behind its brace position); the far fore takes the weight. One literal stamp closes a mane pinhole on the neck.
- **Run contact:** body level and low, withers 2 px below the brace; chest and shoulder lead; neck forward and slightly down so the skull (77, 42.5) is ahead of the box front and inside the box band; tail streaming back.
- **Recover skid:** the brake. The fore legs jammed stiff far forward (hooves x 71 and 75), the rump sitting down on coiled hind legs, the head pitched forward and **down** by inertia (skull (71, 43)), mouth hanging open with a gum-coloured tongue hanging 2 px below the jaw (K1: keys `o`, `g`, never a glow key), the winded squint (a new 4 x 3 eye stamp `squint`: the lid down, no eye white), ears hanging limp, tail drooped.

## 6. Silhouette rules

1. **Head height separates the rush from the spit.** Every brace, scrape and run frame keeps its top row at 20 or lower on screen (row >= 20; the LLAMA-01 idle top is row 8, the spit tell row 4 or less). The recover top is row 28 or lower.
2. **The head points down the lane.** In brace, scrape and run every opaque pixel ahead of column 76 lies on rows 33 to 56 (chest height); the run reaches column 86 or further.
3. **Zero glow keys** in every rush frame. The throat is plain fur.
4. **The recover head never rises** above row 40 ahead of column 76.
5. **Brace and scrape alternate on the same footprint** (same hind hooves, far fore within 1 px; enforced by a test: scrape rows 66 to 71, columns 0 to 39 equal the brace) so the scrape beat reads as a paw, not as a step.
6. **Identity on every frame:** teeth (`T` >= 6), and the eye white (`e` >= 2) except the recover squint (`e` = 0).

## 7. VFX layer (what stays code-drawn)

The art replaces the stand-in's body cues only. The dust and dirt stay code-drawn presentation pieces (`src/combat_anim/rush_stand_in.rs`), changed as follows:

| Piece | Today | With LLAMA-03 |
| --- | --- | --- |
| Brace lean (`lean_px`) | 4 px back, rocking 1 px | **0**: the pose carries the brace (LL3-AD2) |
| Charge lean | 2 px into the rush | **0** (LL3-AD2) |
| Scrape gouge and the two flecks | under the hind hoof (`x - dir * 14`) | anchored to the pawing near fore hoof at `x + dir * 6` (`SCRAPE_HOOF_PX`), growing backward (LL3-AD3) |
| Scrape puff (3 x 3) | behind the hind hoof | unchanged: the dirt the paw throws out behind the llama (LL3-AD3) |
| Charge trail, recover settle | as today | unchanged |
| Bonk burst and 3 px rock back for 8 ticks | as today | unchanged (the rock sells the impact on the hurt frame) |

## 8. Readability against the cast

See the gate images (section 10). At 1x on the merged corral room the braced llama is a long low tan wedge with a dark outline that reads on both the brick wall and the dark bench trough; its head is at Carl's chest height pointing at him, which no other llama frame does. It is twice Carl's width and about the height of his shoulders, so it never hides him; a goblin standing between them stays fully readable in front of the llama's legs (green on tan). Donut (40 px) is never overlapped at the trigger distance (112 px).

## 9. Acceptance targets

| ID | Target | Check |
| --- | --- | --- |
| A1 | Frame 96 x 72, pivot (44, 71), contact row 71 on all 10 frames; palette `llama-floor01-v0` unchanged | tests, V-rules of the atlas config |
| A2 | Zero `r`, `R`, `O`, `Y` pixels on all 10 frames | `key_counts` max 0, tests |
| A3 | Tops: frames 0 (`brace_in`) >= 14; 1 to 7 >= 20; 8 and 9 >= 28 | `bbox` rules |
| A4 | Head down the lane: frames 1 to 7, pixels ahead of column 76 on rows 33 to 56; frames 4 to 7 reach column 86 | tests |
| A5 | Recover head low: frames 8 and 9, nothing ahead of column 76 above row 40 | tests |
| A6 | Teeth `T` >= 6 on all frames; eye `e` >= 2 on frames 0 to 7, `e` = 0 on 8 and 9 | `key_counts` |
| A7 | Frozen keys 1, 2, 4, 8 byte-identical to `tests/fixtures/llama-03-rush_keys.json` | V09 |
| A8 | Orphans <= 9, isolated 0 per frame; enclosed pockets only of the two LLAMA-01 kinds (tail-mane pinholes left of column 25, leg gaps of 4 px or fewer below row 63) | lint, tests |
| A9 | Unique frames; at least one hoof stamp on rows 68 to 71 in every frame | V-rules, tests |
| A10 | In the game, a rush never shows a LLAMA-01 `attack_tell` or `attack_swing` frame and never a glow key | Rust test (section 11) |
| A11 | Gameplay unchanged: headless hashes, goldens, `MANIFEST_HASH`, `TABLE_HASH`, replays identical; the window capture hash equals the headless hash (`match=yes`) | checks, `scripts/room_capture.sh` |
| A12 | At 1x in the corral the brace and the scrape read as "coming at you", distinct from the idle and the spit tell | gate images, owner game-feel review |

## 10. Evidence (design stage)

[docs/evidence/llama-03-rush/](../evidence/llama-03-rush/README.md): the silhouette sheet (the four keys next to the LLAMA-01 idle, spit tell and release, Carl, Donut and a goblin; colour and black; 1x and 4x), the corral strips (each frame mirrored to face Carl on the merged corral still, 1x and 3x), two full 640 x 360 frames, and the lane mock. The lane band is a review annotation (the box's sweep, 96 px travel plus the 28 px box, heights 2 to 30), not game art.

## 11. Execution (done; no design decisions)

1. Ten-frame atlas `assets/source/characters/llama/rush/llama_rush_h64.grid.json` (+ `.ase`, `.pixelart.json`) written by `python3 scripts/author_llama03_rush.py atlas`, manifest `assets/manifests/llama_rush_h64.json` (palette `llama-floor01-v0`, tags of section 4, the tick map of section 3 as `rush_poses`). The keys come byte-identical from the key grid; the frozen fixture is the post-gate one (K1 and K2) and is never regenerated.
2. Bevy (presentation only): `src/combat_anim/llama_rush_pose.rs` maps the rush state to a frame; while a rush pose is active the llama sprite draws the rush atlas (same frame size and pivot, so the image and layout handles swap in place on the same entity, keeping flip, flash tint and z), on by default; without the exported rush atlas the scene logs one `LLAMA_RUSH_ART fallback` line and keeps today's stand-in body. `lean_px` is 0 for tell and committed; the scrape gouge and flecks are re-anchored (section 7). The bonk's first 12 ticks keep the LLAMA-01 hurt frame on the LLAMA-01 atlas.
3. Tests: `a_rush_never_shows_the_throat_glow` walks a rush through all four phases and asserts rush-atlas frames or the hurt frame only; pose-map unit tests, a scene-level check on `corral_rush`, a manifest-equals-code test, and the atlas tests of `tests/test_art_llama03_rush_atlas.py`.
4. Evidence: contact sheet, GIF, 1x in-room stills from `scripts/room_capture.sh corral corral_rush 736:2:50 OUT --f01-run` with its `match=yes` line.

The silhouette gate applied K1 (the panting tongue on `recover_skid`, reused on `recover_blown`) and K2 (the scrape's hind hooves on the brace footprint) before the in-betweens were made.

## 12. Adopted defaults

| ID | Default | Reason |
| --- | --- | --- |
| LL3-AD1 | A bonk shows the LLAMA-01 `hurt` frame (11) for its first 12 ticks, then `recover_blown` | the hurt pose (head thrown back, eyes shut) is the right "hit a wall" read and already exists; no new bonk key |
| LL3-AD2 | `lean_px` is 0 in tell and committed when the art is on; the bonk rock stays | the poses carry the brace and the charge; a lean on top would double them |
| LL3-AD3 | The scrape gouge and flecks move to the pawing fore hoof (`x + dir * 6`); the 3 x 3 puff stays behind the hind hoof | the dirt must start where the hoof paws; the puff behind the llama is the 1x-readable part and is physically the thrown dirt |
| LL3-AD4 | The tell is brace_in, three brace/scrape beats on the stand-in's 8-tick rhythm, then coil | the beat is already tuned and reviewed (CORRAL-F); the coil gives a last 4-tick "now" before the launch |
| LL3-AD5 | Run cycle `[push, reach, contact, gather]`, 3 ticks a frame, from active tick 0 | starts on the push out of the coil; 12 ticks per stride at 4 px a tick reads as a heavy gallop |
| LL3-AD6 | Llama aggression cue: ears pinned plus tail raised on brace, scrape and run | real llama threat posture, an aggression cue. The tail is raised on the LLAMA-01 idle and spit tell as well, so it is not what separates the rush from the spit: the head height and angle are (gate S1: nobody lowers the spit's tail to "restore" the contrast) |
| LL3-AD7 | No in-game lane band | the brief asks the body to tell the lane; the band in the gate images is a review annotation only |
| LL3-AD8 | The recover head pitches down, never up | an upward head is the spit's language |
| LL3-AD9 | The rush art is on by default in `--scene f01_late --f01-run` and in the corral room | it replaces a stand-in; no flag keeps the stand-in body |
| LL3-AD10 | A separate extension atlas (`llama_rush_h64`) instead of appending to LLAMA-01 | LLAMA-01's atlas, manifest and goldens stay untouched (the LLAMA-02 death precedent) |

Open question for the owner (game feel, not art): **LL3-OQ1**, range-end readability. With no lane band (LL3-AD7) the rush's target is read from the body alone. The drawn muzzle reaches 45 px ahead of the pivot while the hit box reaches 28 px, so at the end of range the muzzle can poke up to 17 px into a hero the box never reaches (an error in the player's favour), and at the standard trigger distance the muzzle is already about 4 px past Carl's centre line on the last run frame before the hit.

Owner notes from the gate and the execution: the limp near ear of `recover_skid` and `recover_blown` reads as a collar or strap at 1x (a lighter ear fill with the outline on the lower edge only would fix it if the reviewer agrees); in black silhouette the brace and the run read as a generic shaggy quadruped (hyena or wolf) because the upright neck and tall ears are removed on purpose (accepted: raising the head would break the rush-versus-spit rule).

## 13. Files

| File | Role |
| --- | --- |
| `scripts/author_llama03_rush.py` | the key poses, key grid, fixture, gate entry point |
| `scripts/llama03_rush_gate.py` | the gate images, the contact sheet and the GIF |
| `assets/source/characters/llama/rush/llama_rush_keys.grid.json`, `.ase` | the four design keys |
| `assets/source/characters/llama/rush/llama_rush_h64.grid.json`, `.ase`, `.pixelart.json` | the ten-frame atlas and its toolkit config |
| `assets/manifests/llama_rush_h64.json` | the atlas manifest and the `rush_poses` tick map |
| `src/combat_anim/llama_rush_pose.rs` | the pose map (state to frame) |
| `src/combat_anim/rush_stand_in.rs` | the dust and dirt (art on: lean 0, gouge at the paw) |
| `tests/test_art_llama03_rush_atlas.py` | atlas, A1 to A9, manifest and exported-atlas tests |
| `tests/fixtures/llama-03-rush_keys.json` | frozen keys, keyed by rush-atlas index |
| `tests/test_art_llama03_rush.py` | contract, readability and evidence tests |
| `docs/evidence/llama-03-rush/` | gate images and checksums |

## 14. Rights and provenance

Fan-concept art under the owner-reported `ART-PUBLICATION-ALL-V1` scope; no public reuse licence; no game or asset-pack distribution right. Every pixel is rasterized by the LLAMA-01 parametric rig (`scripts/author_llama01.py`) from the pose dicts and literal stamps in `scripts/author_llama03_rush.py`. Nothing was traced or sampled from concept boards, reference footage or image-model output. The corral backdrop in the gate images is the merged CORRAL-WIRE window still.

### Rights/Provenance

Rights/Provenance: designed Opus, gate Opus, executed Sonnet, reviewed Opus (review pending).

- Design, spec, the four keys and the gate images: Claude Opus.
- Silhouette gate (K1, K2, B1 to B8): Claude Opus, a separate fresh agent.
- Execution (section 11): Claude Sonnet.
- Independent review: pending, a separate top-tier agent.
