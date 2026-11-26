# HRD-CUES-01: the Hoarder's pre-wake stir and the J-B post's "not yet" (design spec)

**Status:** keys, frames, wiring, tests and evidence done. Every cue frame is authored in the room kit sources (`hoarder_mound` frames 3..5, `hoarder_post_b` frames 3 and 4), two of them frozen as keys; the presenter draws them (on by default, presentation only). Design gate: [docs/evidence/hrd-cues-01-design](../evidence/hrd-cues-01-design/README.md). Runtime evidence: [docs/evidence/hrd-cues-01](../evidence/hrd-cues-01/README.md). Independent Opus review pending; owner art review pending, game-feel review pending. Not owner-approved.

**Scope.** Two small tells in Floor 1 room 1, the Hoarder (audit row X9):

1. **Pre-wake stir (HRD-Q05).** The lair mound is visible from x 240 but nothing tells the player it is alive until the gate closes and the Reveal starts. The heap now breathes once a hero reaches x 240, and harder once a hero has crossed the engage line (x 432).
2. **J-B post "not yet" (HRDC-AD4).** Hits on the J-B post count only once she is out (`PileBoss::is_out()`). During Dormant and the Reveal a hit does nothing, and the post looked exactly as it does when a hit counts (the pale fresh split, the hittable cue). The post is now visibly shored while she is not out, and the shoring springs loose on the tick she comes out.

Both cues are **presentation only**: they are derived from state the simulation already keeps (or from the hero positions the presenter already reads), they add no simulation field, and they move no hash, golden or pin.

**Route.** Difficulty score 1 (three back-layer frames that are a rigid column shift of the frozen `sealed_back` frame, two post frames on the existing post rule), class Easy, route **C** ([routing by difficulty](../07-agents/pixel-art-pipeline.md#routing-by-difficulty)): this spec plus the frozen keys. The remaining frames follow mechanically from the same functions and were authored in the same pass. Spec and keys never name who drew them; provenance is in the manifests and the PR.

**Authorities.**

- **Design:** [enemy-gameplay-hoarder.md](../01-design/enemy-gameplay-hoarder.md) (room layout, the approach 208..352, "the lair mound is visible from x 240 (stir cue, presentation only)", HRD-Q05 knobs, HRDC-AD4).
- **Data:** `assets/data/gameplay/enemy_pile_boss_v0.json` (`hrdq05_stir_x` 240, `hrdq05_engage_x` 432, `hrdq19_home_x` 608, `hrdq41_settle_ticks` 8). `hrdq05_stir_x` was read by no code before this order.
- **Sim state read:** `src/pile_boss.rs` (`mode()`, `is_out()`, `gate()` with `Gate::Open { crossed }`, `tuning()`), `src/env_prop.rs` (`FallingMass::state`), `src/corridor.rs` (`CorridorWorld::body`, `SceneSim::party_live`).
- **Current art and presenter:** [hoarder-hoard-02.md](hoarder-hoard-02.md) section 7 (mound, post, R6 "non-solid junk sits one step darker", R7 Reveal dust), `src/hoarder_art.rs` (pure half), `src/bin/floor01_scene/hoarder_art.rs` (Bevy half).
- **Sources:** `scripts/author_hoarder_room.py` (`mound_stir`, `STIR_LIFTS`, `STIR_GRIT`, `post_shored`, `post_give`), `scripts/author_hrd_cues.py` (frozen keys, mote table, design gate).
- **Tone:** [antihero tone](../01-design/antihero-tone.md) and [art direction](art-direction.md).

**Horizon check.** Presentation only: no move, ability, unlock, hit rule or terrain-combat rule changes (HRDC-AD4 itself is unchanged), so the [living horizon](../01-design/character-progression-horizon.md) needs no update.

## 1. Tone and readability rules

- **Something under the junk, not a spell.** The heap breathes like a body under a pile of rags: the crown lifts a few pixels and the items on it ride along, grit trickles off the flanks, it settles a pixel lower than it started. No eyes in the dark, no glow, no colour pulse, no sound-wave rings.
- **The post is shored, not locked.** Two nailed iron straps and a wedge under the foot, the timber one ramp step darker like the room's dressing (R6). Plain practical carpentry that says "this holds". When she bursts out, the room shakes and the shoring springs: one strap snaps and peels out, the other slips, grit sifts out of the lashing, and the pale split is back. No icon, no lock symbol, no outline colour.
- **1x first.** At 640 x 360 the cue must read from motion and value, not detail: the stir is a 3 to 5 px lift of the visible part of the mound at the right screen edge plus pale grit motes; "not yet" is a darker post crossed by two iron bands next to the lit post it becomes.
- **No new colours, no new layers.** Kit ramps only (`floor01-catacomb-v2`), binary alpha, the existing R7 dust motes, the existing z slots.

## 2. Cue behaviour and timing

Ticks are 60 Hz scene ticks. "Lead x" is the largest x of the present heroes: Carl always, Donut while `SceneSim::party_live()`, the companion bot included (the simulation's own present set for the gate; `hoarder_art::lead_x`). The presenter does not exclude a defeated hero as the sim's gate does; in Dormant that is negligible.

### 2.1 Stir levels (HRD-Q05)

| Level | Condition (every tick) | Breath period | Back-layer frame by phase | Motes |
| --- | --- | --- | --- | --- |
| **S0** none | not S1 or S2 | none | `sealed_back` (0) | none |
| **S1** approach | `mode() == Dormant`, `gate() == Open { crossed: None }`, lead x >= `hrdq05_stir_x` (240); once active it holds until the lead x drops under 224 (HC-AD11) | 72 | phase 0..5 `stir_heave` (3), 6..9 `stir_slump` (5), 10..71 `sealed_back` (0) | 3 (S1 table) |
| **S2** engaged | `mode() == Dormant`, `gate() == Open { crossed: Some(_) }` | 36 | phase 0..7 `stir_heave_hi` (4), 8..11 `stir_slump` (5), 12..35 `sealed_back` (0) | 5 (S2 table) |

- **Phase** = (scene tick - first tick the presenter saw the current level) mod period. The anchor is render-local (`HoarderMemo`), so the first breath fires on the tick the level starts: a hero stepping past x 240 sees the heap heave at once, and the crossing of x 432 starts a hard heave that covers the 8 settle ticks before the gate drops (solo). A level change re-anchors; a tick that goes backwards (restart, retry) clears the memo, like `out_age`. **Hysteresis (HC-AD11):** once S1 is active it stays active, with the same anchor, until the lead hero is under x 224 (`STIR_HOLD_PX` = 16 under `hrdq05_stir_x`), so a hero hovering on the line at x 240 does not restart the breath; dropping under 224 ends the stir and reaching 240 again starts a fresh breath at age 0.
- **The front lip stays `sealed_front` (1)** in every stir frame. The lift's stretched rows are under it, so it hides the seam.
- **The stir stops on the Reveal's first tick** (mode leaves Dormant) and R7 takes over. On a retry she is Dormant again and the stir returns by the same rules (the retry Reveal is shorter, the stir is not).
- In 2P, S2 lasts as long as the gate waits for the second player: the heap keeps heaving every 36 ticks. In solo it lasts exactly the 8 settle ticks (`hrdq41_settle_ticks`, the Reveal starts at the crossing + 8): one heave, and solo play only ever shows `stir_heave_hi`, never the S2 slump or the S2 motes after phase 7.

### 2.2 Post states ("not yet", HRDC-AD4)

| When (every tick) | Post frame | Motes |
| --- | --- | --- |
| `!boss.is_out()` (Dormant or Reveal), machine state `Supported` | `shored` (3) | none |
| `is_out()`, `out_age` 0..7, machine state `Supported` | `give` (4) | 4 give motes |
| otherwise | the machine's frame as today: `intact` (0), `cracked` (1), `broken` (2) | as today |

- `out_age` is the existing `HoarderLook::out_age` (first tick the presenter saw her out). `GIVE_TICKS` = 8.
- The post's machine cannot leave `Supported` while she is not out (HRDC-AD4: no hit counts), so `shored` only ever replaces `intact`. A qualifying hit inside the give window (machine `Weakened`) shows `cracked` at once: the machine's state always wins over the cue.
- Bolts still pass the post while she is not out (unchanged rule); the shored look is the explanation. There is **no** reactive cue on a refused hit (HC-AD8, open question HC-OQ1).

## 3. Frames and keys

All frames are in the existing atlases, appended after the current frames, so the frames and stills already committed stay byte-identical.

| Atlas | Frame | Name | Rule | Key |
| --- | --- | --- | --- | --- |
| `hoarder_mound` (176 x 72, pivot 88, 71) | 3 | `stir_heave` | `sealed_back` with every column lifted rigidly by its band's lift (left abs(dx) <= 10: 3 px, <= 40: 2, <= 56: 1; right <= 14: 3, <= 24: 2, <= 52: 1), re-outlined, 3 grit specks | **frozen** (`tests/fixtures/hrd-cues-01-mound_keys.json`) |
| | 4 | `stir_heave_hi` | same, lifts 5/4/2/1 (left reaches 10/40/56/68, right 14/24/52/66), 4 grit specks | |
| | 5 | `stir_slump` | same, lift -1 (left abs(dx) <= 40, right <= 24): the exhale settles one pixel below rest | |
| `hoarder_post_b` (16 x 64, pivot 0, 63) | 3 | `shored` | `intact` with the split painted over by plain timber, the whole post one step darker (R6), two iron straps (rows 18..20 and 40..42, the bank's strap rule, nail heads), a wedge under the lane-side foot | **frozen** (`tests/fixtures/hrd-cues-01-post_keys.json`) |
| | 4 | `give` | `intact` at full value with the split; the upper strap's right end snapped and peeled up and out, the lower strap slipped to rows 46..51 and tilted, 4 grit specks out of the lashing | |

**The lift (`mound_stir`).** For each column x, `lip` is the first opaque row of `sealed_front`; the hinge is `lip + |lift| + 1` (at most 71). Rows under the hinge are unchanged. Above it, each row takes the pixel `lift` rows lower (clamped to the hinge row), so the column moves up rigidly and the few duplicated rows sit behind the front lip. A slump takes the pixel one row higher. Pixels are moved, never re-shaded, so the rag texture does not boil between frames. The band edges are placed so they never cut the crate (dx about -40..-11), the brass frame (-6..14) or the iron pot (25..51): those ride whole; the 1 px jogs fall on rag folds, the bent copper pipe and the broken bed rail.

**Grit specks** (in the frames): a lit stone pixel (`stone` 7) over its shadow pixel (`stone` 3), 3 to 6 px over the lifted surface; never a lone pixel.

## 4. Code-drawn motes

The motes are the existing R7 dust (`Mote`, `DUST_COLOUR` srgba 0.95, 0.82, 0.62, 0.95, 3 x 3 px, `mote_z`). No new colour or sprite.

### 4.1 Stir motes

For a breath at phase `p` and the k-th entry `dx` of the level's table, `a = p - 2 - 3k`; the mote is drawn while `0 <= a < 20` at

- x = home + dx + side x 0.3 x a (side -1 for dx < 0, else +1),
- y = floor_y + h(dx) - 1.2 x a,

where home is `hrdq19_home_x` (608), floor_y the boss setup's floor and h(dx) the start height over the floor (2 px over the `sealed_back` surface, from `python3 scripts/author_hrd_cues.py --motes`):

| Table | dx and h(dx) |
| --- | --- |
| S1 | -40: 47, -20: 64, +12: 57 |
| S2 | -52: 43, -34: 53, -16: 61, +6: 59, +28: 53 |

The motes slide down and outward and vanish behind the front lip (they are drawn under it). At most 5 at once.

### 4.2 Give motes

For `out_age` a in 0..7 and k in 0..3 with dx = 0, 2, 14, 16: x = post.x + dx + 0.5, y = floor_y + 54 - 3k - 2a (the post's left edge and floor are the sidecar's hit target, as `post_prop` places it). With R7's 12-mote burst (ages 0..3) the total is 16 = `MAX_MOTES`; the stir is Dormant-only and the Reveal motes stop before she is out, so nothing else shares the pool.

## 5. Palette and layers

- **Palette:** `floor01-catacomb-v2` ramps only. Mound: the `sealed_back` colours plus `stone` 3 and 7 (39 colours in the atlas). Post: the post's colours one step down for `shored`, `iron` 2..6 for the straps, `stone` 2 and 5 for the grit (22 colours). Binary alpha.
- **Layers (unchanged slots):** the stir frames replace the mound's `Layer::Back` sprite (`back_z`, behind everything); `sealed_front` stays at `front_z`; the post frames stay at `prop_z`; all motes at `mote_z` (in front of the back layer, behind the front lip). No new `Layer` variant.

## 6. Acceptance targets

1. **A1 keys frozen.** `hoarder_mound` frame 3 and `hoarder_post_b` frame 3 are byte-identical to their fixtures (`python3 scripts/author_hrd_cues.py --check`); frames 0..2 of both atlases are unchanged (the existing hero-key test still passes).
2. **A2 rigid stir.** In each stir frame, every pixel at or under the hinge row equals `sealed_back`; above it, each column equals `sealed_back` shifted by that column's lift, apart from outline pixels and the grit specks. The lift table is exactly section 3's.
3. **A3 lip hides the seam.** Compositing a stir frame under `sealed_front` changes no pixel inside the lip's opaque area, and no opaque pixel of a stir frame reaches row 0.
4. **A4 shored reads "not yet".** `shored` has no `wood` 5 pixel in columns 4..10, rows 14..51 (no pale split); it has the two strap rows (`iron` 4 across columns 2..13 at rows 18 and 40); its mean ramp index is lower than `intact`'s; timber width at most 14 per row and a ragged top (the existing post rules).
5. **A5 give.** `give` keeps the `intact` split; at most 14 opaque px per row; grit specks are pairs.
6. **A6 presenter rules.** Pure unit tests over a `PileBoss`: S0, S1 and S2 pick the frames of section 2.1 at phases 0, 5, 6, 9, 10 (S1) and 0, 7, 8, 11, 12 (S2); a lead x of 239 gives S0 and 240 gives S1; Donut's x counts only while the party is live; the Reveal draws `sealed_back` and no stir motes; `shored` for every not-out tick; `give` for `out_age` 0..7 only when the machine is `Supported`, then `intact`; mote counts within `MAX_MOTES` on every tick of a scripted run; every frame index below `Prop::frames()`.
7. **A7 hash neutral.** `the_presenter_does_not_change_the_state_hash` runs with a lead x that drives S1 and S2; no simulation field is added; `MANIFEST_HASH`, `TABLE_HASH`, goldens and replays are unchanged.
8. **A8 1x readability.** In window stills at 640 x 360 of the floor run (`--scene f01_late --f01-run`) and of the room's direct plan, at gameplay scale with Carl and Donut on screen: (a) the S1 heave is visible on the part of the mound at the right screen edge, (b) the grit motes read against the brick wall and the heap, (c) the shored post is darker and banded next to the lit stack, (d) the give beat shows on the out tick. Every capture ends with `window_hash == headless_hash`.
9. **A9 tone.** No glow, no colour pulse, no eyes, no symbol; kit ramps only; binary alpha (toolkit and `validate_assets.py` pass).

## 7. Adopted defaults

| ID | Value | Why |
| --- | --- | --- |
| HRD-Q05 | The stir cue starts when the lead present hero reaches `hrdq05_stir_x` (240, existing data); it is a breathing heap (lift, slump, grit), presentation only | The design names x 240 and "presentation only"; a breath reads as alive under junk without showing her |
| HRDC-AD4 | Unchanged rule; its cue is the `shored` post while she is not out and the `give` beat when she comes out | Explains the dead hits with plain carpentry and teaches "now" at the moment it becomes true |
| HC-AD1 | The stir frames are a rigid column lift of `sealed_back` with an unchanged front lip | No re-shading, so no texture boil; the lip hides the stretch; the frozen sealed key stays the source |
| HC-AD2 | Two levels: S1 period 72 (heave 6, slump 4), S2 period 36 (heave_hi 8, slump 4) | One heave on the trigger tick and usually no second one on a straight run (at `RUN_SPEED` 185 px/s the 192 px walk from 240 to 432 takes about 62 ticks, under one period); a hesitant player sees it repeat. A hard heave covers the 8 settle ticks; 2P waiting keeps escalated |
| HC-AD3 | Lead x = Carl, plus Donut while the party is live, the companion bot included (`hoarder_art::lead_x`; the stir follows the same present set that crosses the gate, so S1, S2 and the gate never disagree about who is near) | Matches the sim's present set for the gate |
| HC-AD4 | Breath phase anchored to the first tick the presenter sees the level (render-local memo, cleared when the tick goes backwards) | First breath on the trigger tick; same memo pattern as `out_age` |
| HC-AD5 | Stir and give grit reuse the R7 dust motes (colour, size, `mote_z`); 3 (S1), 5 (S2), 4 (give) | One dust language in the room; no new colour or sprite; within `MAX_MOTES` |
| HC-AD6 | `shored` on every tick she is not out, Dormant and Reveal, first attempt and retries | The rule it explains holds on all those ticks |
| HC-AD7 | `give` for 8 ticks from her first out tick, only over `intact`; then the machine's frames | A beat as long as two R7 bursts; a real hit always wins |
| HC-AD8 | No reactive cue on a refused post hit; bolts keep passing the post | A refused hit leaves no trace in the sim; detecting it needs a new observer field (HC-OQ1). The ambient cue covers the case |
| HC-AD9 | On by default wherever the Hoarder room is drawn, including `--scene f01_late --f01-run`; no flag | Presentation only: no simulation, replay, hash or input change to gate; the existing flag gating protects authoritative behaviour, and this cue changes none |
| HC-AD10 | Cue frames appended: mound 3..5, post 3..4 | Existing frames, fixtures and stills stay byte-identical |
| HC-AD11 | Hysteresis: once S1 is active it holds until the lead hero is under x 224 (16 px under `hrdq05_stir_x`); the hold lives in the render-local memo (`HoarderMemo::stir_held`, `stir_level_held`) | A hero hovering at x 240 would otherwise re-anchor the breath on every crossing and heave rapidly (design gate note N1); presentation only, no sim read changes |

## 8. Open questions (not adopted)

- **HC-OQ1.** A reactive "dead knock" (post jolt, grey grit, no white-wood flash) when a human hero's melee meets the post while she is not out. It needs an unhashed observer field (for example the last refused post-hit tick, excluded from `mix_into` like F01-SIM-TICKS ST-AD1). Owner call after playing the ambient cue.
- **HC-OQ2.** Audio (a heap creak on the heave, a strap ping on the give) is out of scope until the room has an audio pass; the "stack creak" owner question stays open.
- **HC-OQ3.** The white-wood flash and splinter spray on a qualifying post hit (HOARD-02 section 7) is not drawn today; it is the natural counterpart of the give beat and belongs to a follow-up order.

## 9. Test plan

- **Python (art):** `tests/test_art_hoarder_room.py` gains: A1 (fixtures), A2 and A3 (rigid lift against `sealed_back` and `sealed_front`, recomputed from `STIR_LIFTS`), A4 and A5 (post pixel rules; extend the existing post-width and ragged-top loops to frames 3 and 4). `python3 scripts/author_hoarder_room.py --check` and `python3 scripts/author_hrd_cues.py --check` stay clean.
- **Rust (pure half):** `src/hoarder_art/tests.rs` gains the A6 table tests, the memo anchor and reset test, and extends the state-hash test (A7).
- **Scenario:** the existing hoarder room plans (`--f01-room hoarder --room-pilot hoarder:carl_solo:0`) and the floor run reach S1, S2, `shored` and `give`; a pure scripted run over those ticks asserts the frames and mote counts.

## 10. Evidence plan

- Directory `docs/evidence/hrd-cues-01/` with README and `SHA256SUMS`, made by `scripts/make_hrd_cues_evidence.sh`, stills at 640 x 360 and 1920 x 1080. Because the cue ages are render-local, each cue is captured as a **one-tick-per-frame sequence that starts just before the cue is active** (a single-tick capture would show every age at 0), and the stills are picked from the sequence by tick:
  - direct plan (`--room-pilot hoarder:carl_solo:0`): S1 heave, slump and motes, S2 heave, `shored`, `give` ages 0, 2 and 7, `intact_after` (out tick + 10);
  - floor run (`--scene f01_late --f01-run`, the same plan): the S1 heave, S2 heave, `shored`, `give` and `intact_after` stills in the Hoarder room at gameplay scale.
- Contact strips and GIFs of one S1 breath, one S2 breath and the give beat (x3 crop of the mound and of the post; the window already renders at 3x, no resampling).
- Every capture line ends with `window_hash == headless_hash`; capture lines are kept next to the stills.
- The design gate composite stays in [docs/evidence/hrd-cues-01-design](../evidence/hrd-cues-01-design/README.md); it is not runtime evidence.
