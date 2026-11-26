# AUDIENCE-01 — Individual live viewers in the Hybrid-A HUD

**Status:** owner-approved direction; design contract amended by the 2026-11-19 top-tier review (see "Adopted design decisions"). The UI art (part A), the Bevy-free domain (part B, `src/audience.rs`) and the HUD integration (part C) are implemented on PR #118; independent review and game feel are pending.
**Authority:** `AGENTS.md`, `docs/02-art/hybrid-hud-a-design.md`, `docs/02-art/hybrid-hud-a-native-spec.md`, `docs/01-design/floor-01-gameplay-progression-v0.md`, `docs/01-design/character-progression-horizon.md`. Where this proposal conflicts with current gameplay/input/unlocks, those contracts prevail.

## Decision and scope

- Live **viewers are the visible arcade score**, measured **separately for Carl and Donut**, identified by stable **hero ID**, never by controller slot. Solo companion actions score for that hero; two-player mode does not merge their totals.
- Viewers track *entertainment*, not merely competence: combat, creativity, teamwork, embarrassing failures, and *authored* anticipation of looming trouble can attract attention.
- Keep an internal, deterministic spectacle/event scoring model; no second on-screen spectacle bar.
- **No counters at Floor 01 start**: neither an OFFLINE box nor zero placeholders. An explicit later narrative broadcast unlock makes both available together. The precise story beat is **TBD with canon/progression review**, candidate after introductory System/Tutorial Guild material. It is separate from Donut's biscuit/mana unlock.
- No subscriber economy, sponsors, achievements/buffs from score, stage-end results implementation, or persistent cross-run economy in v1. Stage-end summary design is separate.
- Preserve the existing two Hybrid-A plates, avatars, HP bars, Donut-only post-biscuit mana, owner tags, ability chips, centered location/boss strip, System CRT and all controller semantics. No new permanent panels.

## Native HUD layout (640x360 and 640x400; pixel units)

Both hero plates stay at the exact existing positions/sizes: Carl `(8,8,164,48)`, Donut `(468,8,164,48)`. Place the audience readout **inside the existing name row**, *not* in the middle gap or beneath the plates.

| UI element | Plate-relative bounds | Carl absolute | Donut absolute |
| --- | --- | --- | --- |
| Eye icon | 7x7 at (+92,+4) | (100,12) | (560,12) |
| Number field | x=+106..+150, y=+4..+14; right-aligned | x=114..158 | x=574..618 |
| Trend glyph | 5x7 at (+153,+6), reserved even when hidden | (161,14) | (621,14) |

These are **proposed** pixel rects; the executor must verify actual atlas/font metrics at 1x and may adjust within the name row **without shifting another existing field**. The current names are 30px (`CARL`) and 40px (`DONUT`) at +40,+4, leaving room to the right. The HP bar begins at +50,+16, so the readout must not descend into that row. The 3px frame ring and action lane y>=60 remain untouched.

Styling: reuse `hud_ink`, `plate_fill`, `brass`, `amber`, `amber_dim`, `magenta_hi`, and the native font pipeline. **No additional frame or permanent VIEWERS label**; eye glyph + compact number + optional up/down indicator only. Existing `Jersey 10` is first choice for numbers if 1x fit checks succeed; otherwise a new original tiny bitmap digit glyph set (editable pixel-grid source). Create original editable art: never crop concept PNGs into the runtime HUD. Extend the existing HUD atlas/source/export pipeline; do not introduce `bevy_ui` or a separate render camera.

## Number formatting

Format the **display only**; model values are untruncated integers:
- `0..999`: full integer.
- `1,000..99,999`: one decimal K, **truncate**: `1.0K`, `12.4K`, `99.9K`.
- `100,000..999,999`: integer K, truncate: `159K`, `999K`.
- `1,000,000..9,999,999`: one decimal M: `1.2M`.
- `10,000,000..999,999,999`: integer M: `12M`, `999M`.
- `>=1,000,000,000`: `999M+` as a bounded display sentinel until a wider scale is designed.
- ASCII digits and invariant `.` suffix, no locale separator; right-aligned fixed field. Test at native scale with arrows and `999M+`.

## Visual state

| State | Rendering | Animation (60Hz fixed tick) |
| --- | --- | --- |
| Locked | no eye, number, trend or placeholder | none |
| Stable | brass eye + amber count | none |
| Small gain | new count, amber | none |
| Spectacular gain | magenta number and up arrow | bright highlight 12 ticks, up arrow up to 48 ticks |
| Material fall | amber number and dim-amber down arrow | arrow up to 48 ticks, cooldown suggested 600 ticks |
| Suspense / imminent event | same as spectacular gain | *no distinct warning icon or explanatory text* |

Suggested large gain threshold: `delta >= max(300, current_before * 15 / 100)`. Suggested meaningful decline: cumulative fall `>= max(250, reference * 10 / 100)`; routine small decay remains quiet. Count transition may interpolate over <=30 ticks in whole pixels/integers; authoritative values update instantly. Normal display refresh <=5Hz except on a meaningful event. New spikes retarget; no unbounded queue. No camera shake, HUD shake, particles or interference with danger tells. Respect `--hud-reduced-motion` (no flash/interpolation) and `--hud-high-contrast` (direction never color-only). Boss warnings and health always outrank spectator effects.

## Domain model and candidate tuning (not final balance)

Implement a Bevy-free deterministic domain, e.g. `src/audience.rs`, driven by real fixed-tick combat/encounter events. Keep `current_viewers`, `peak_viewers`, `last_interesting_tick` and anti-duplication/anti-farming state **per stable hero**. Rendering may read but must never affect the simulation or replay hash. Suggested unlock baseline is **250 each**.

| Distinct event | Candidate award for actor |
| --- | ---: |
| Routine enemy defeat | +120 |
| Heavy / launched defeat | +500 |
| Catastrophic / creative defeat | +1,800 |
| Separately identified environmental creativity | +900 |
| Chain combo after second meaningful action | +250 to +1,000, capped |
| Humiliating accident (unique/cooldown) | +400 |
| Scripted anticipation with targeted hero(s) | +4,000 to +15,000 |

Defeat force tiers **replace**, not stack onto, each other. An environmental bonus only stacks when it represents an independently identified action, not the same death counted twice. An actual contributing assistant may get **30%** of the actor award (integer truncation), but mere proximity does not qualify. A spectacular AI-controlled Donut action still belongs to Donut.

**Anticipation must be explicitly authored** by encounter/system data with an event ID and set of targeted heroes; it is not automatically triggered by every off-screen enemy. It may target one or both heroes and does **not** disclose the imminent threat in the UI. This preserves suspense and prevents spoiler reliability.

**Decay proposal:** begin 8s after a hero's last interesting event; once per simulation second reduce the **excess over 250** by ~2.3% (about 30s half-life). Use fixed-point/integer accumulator for small deltas, never wall time, never below baseline. Reset only that hero's inactivity after a real interesting event. Deduplicate event IDs across restarts; checkpoint/attempt restore must respect the game's existing deterministic restart semantics, without farming via deaths.

## Unlock, preview and end-of-stage extensibility

- `locked -> unlocked` is one explicit game/story event, separate from `mana: Option<ManaView>`. Persist/restore according to current story checkpoint contract (not by a new standalone save system).
- `HudSnapshot.plates[i].audience: Option<AudienceView>` is `None` until real unlock or explicit debug preview. `Some` holds the integer count and presentation trend/cue age. Avoid deriving hero identity from P1/P2.
- Provide a **clearly labelled PREVIEW VIEWERS** fixture with locked/normal/rising/falling/anticipation; preview must never change simulation, RNG, input or replay hashes.
- Future stage-end UI may consume `peak_viewers`, ending `current_viewers`, per-hero spectacle events and contributor IDs, but its presentation and winner metric are **not designed/approved in this PR**.

## Integration boundaries

Update the existing HUD model/draw/adapter only after the domain contracts exist: `src/hud.rs`, `src/hud_draw.rs`, `src/hud_adapter.rs`; the native pixel grid `assets/source/hud/` and `assets/manifests/hud_hybrid_a_v1.json`; existing export script, evidence and tests. Retain one HUD screen-space camera and current fonts and atlas rather than creating a new separate HUD. If current code has moved since this spec, adapt to the *live* main interfaces rather than duplicating state.

## Acceptance gates

1. Floor 01 starts with **zero visible audience decoration**; unlock reveals exactly one count per hero, no other HUD layout shift.
2. Source/atlas pixels, native captures, controller rows and HP/mana bars remain correct at 640x360 and 640x400, 1x and integer-scaled 3x.
3. `999`, `1.0K`, `12.4K`, `159K`, `1.2M`, `999M+` and trends fit without text/name/HP/frame collisions; test `DONUT` worst-case name width.
4. Solo, swap and local 2P attribute viewership to stable hero identities. Assist and repeat-event logic is tested; no scoreboard cloning at control switch.
5. Real authored anticipation raises selected heroes' counts *before* on-screen action; no warning text, no universal hidden-enemy detector.
6. Deterministic fixed-tick decay, no underflow, no double-award, no RNG/non-deterministic rendering feedback, checkpoint-safe reset.
7. Boss strip, System notices, warning tells, high contrast and reduced motion work together; compare against real capture in `docs/evidence/hud-hybrid-a/`.
8. `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `python3 scripts/check_docs.py`, `python3 scripts/validate_assets.py`, native render smoke checks pass **after implementation**; report actual evidence, not assumed passes.

**Open decisions:** exact canonical unlock beat; final scoring/restart balance; stage-end result ranking. The owner accepted the compact individual HUD direction, **not** those untested tuning constants.

## Adopted design decisions (2026-11-19)

The top-tier review of PR #118 (verdict PASS-WITH-FIXES) was resolved under the AGENTS.md "Design defaults" rule: each defect adopts the recommended option. **Where this table differs from the sections above, this table wins.** Every number is a hypothesis in one tuning table (`audience::TUNING`); the owner reviews game feel after merge.

| ID | Decision |
| --- | --- |
| F1 | Reference hierarchy with corrected geometry. Original monospace **5x7** digits (`0-9 K M +`, advance 6, `.` 1x1 on row 6 with advance 2), **left-aligned** from plate +104, authored as `.grid.json` in `assets/source/hud/`. Jersey 10 is not used for the count. Stable count `text_white`; **amber** number plus a **`magenta_hi` up arrow** on a spike; `brass` eye (9x7 at +92,+5, `hud_ink` pupil, `brass_hi` iris); trend glyph 5x7 at +153,+5, slot always reserved. Identical at 640x360 and 640x400. Carl eye rect (100,13,9,7), number x=112, trend (161,13,5,7); Donut x+460. |
| F2 | Name ink ends at or before plate +88 (width at most 49 px from +40). A longer replaceable name falls back to `HERO` (Carl) / `ALLY` (Donut). `DONUT` (40 px) leaves a 12 px gap. Domain helper: `audience::bounded_name`. |
| F3 | Every award row maps to a real event or is future work that awards 0 in v1: humiliating accident (no event; `HeroHurt` is never awarded), launched-goblin contact (needs a launch-credit bridge), `BossHurt`, `GoblinTell`, `BossTell` (not awarded). |
| F7 | Credit goes by **hero index** only (0 Carl, 1 Donut, anything else no award). `DefeatCause::{ShoreBlock,ShoreBlockEdge,DropHazard,Explosion}.player` and `EnemyDefeated.instigator` hold hero indices. `Control` and `PadSlots` are never read. Tests cover 2P with P1 driving Donut and solo after a swap. |
| F8 | The chain bonus comes from the CINE-01 feat signal (`cine::CinematicMoment { credited, target, score }`), `min(1000, 250 * (score - 4))`, mapped into the plain-data `audience::ChainFeat`. Audience never detects chains; CINE never reads audience data. Until CINE-01 merges the award is 0. |
| F9 | `Restart` restores the attempt-start snapshot (counts, peak, decay remainder, dedupe set, fired cue ids). A `Respawn` (DEMO-01) keeps counts and the dedupe set and never costs viewers. Decay uses a domain-owned monotonic step counter, never event ticks. v1 keeps state in memory only; persistence waits for a real save contract. |
| F10 | Audience is an **observer**, never mixed into `COMBAT_STATE` or the `COMBAT_EVENT` logs. While locked the state does not exist and no step runs, so legacy goldens stay byte-identical. When unlocked it prints its own `AUDIENCE_STATE` digest line only in audience fixtures. |
| F11 | Decay uses a milli-remainder: `acc = excess * 23 + rem; dec = acc / 1000; rem = acc % 1000`, once every 60 steps from the 480th inactive step. Floor 250. `u64` with saturating arithmetic. |
| F12 | Assist: for an environmental cause with `instigator = Some(h)`, `h != actor`, `h` gets `actor_award * 30 / 100`. Environmental creativity (+900): environmental cause whose `instigator == actor`. Melee has no assist (`Hit` has no attacker; fail closed). |
| F13 | No separate flash. Fall reference is the highest shown count since the last cue; a fall cue fires when `ref - shown >= max(250, ref / 10)`, then `ref = shown`, with a 600-step cooldown. Spike: a single-step award `>= max(300, before * 15 / 100)`; the up arrow lasts 48 steps and a new spike restarts it (no queue). Shown count refreshes at most every 12 steps (a spike at once); count-up takes at most 30 steps. |
| F14 | No new setting: reuse `HudSettings { reduced_motion, high_contrast }`. Reduced motion snaps the number (no count-up); colour states and arrows stay. High contrast turns the down arrow `amber` (`amber_dim` is 4.2:1). Direction is carried by arrow shape. |
| F15 | **No in-game unlock in v1.** Floor 01 authors no unlock and no cue; the state is `None`/locked in every Floor 01 path. Reachable only through preview (`--hud-preview viewers | viewers-rising | viewers-falling | viewers-max`, PREVIEW tag) and a QA fixture (unlock at step 1, both heroes at 250, authored test cue). The unlock is its own story flag (`broadcast_live`), never the biscuit or mana unlock. **Amended, owner-adopted 2026-11-25 ([AUDIENCE-RUN](../01-design/audience-run-design.md), implemented (AUDIENCE-RUN PR), game feel pending):** the `--f01-run` of `f01_late` sets `broadcast_live` at the corral exit, and the audience is live in every later run room, with counts carried across relaunches. All other Floor 01 paths stay locked, Floor 01 still authors no cue, and the preview and QA probe paths are kept unchanged. Opt-out: `--audience=off`. |
| F16 | Fit checked against the real captures: the plate region +80..+158 x +3..+14 is pure `plate_fill` on both plates; boss strip, HP, mana, ability row and ring are clear. |

Open questions (not blocking): **AUD-Q1** the canonical unlock beat (candidate: the first descent after the Tutorial Guild / System introduction; adopted for the `f01_late` run as the corral exit by AUDIENCE-RUN AR-AD1, the full-game beat stays open as AR-Q1); **AUD-Q2** final award and decay balance after a playtest; **AUD-Q3** whether a death costs viewers or becomes a humiliation spike once an authored accident event exists; **AUD-Q4** stage-end ranking (out of scope).

## Implementation status

| Part | State |
| --- | --- |
| A, UI art (eye, arrows, digits `.grid.json`, atlas regions) | **done** (branch `art/audience-01`): 16 `viewer_*` atlas regions (47 frames in total), pixel-grid sources, mocks in `docs/evidence/audience-01/` |
| **B, domain** `src/audience.rs` | **done** (branch `feat/audience-01-domain`): per-hero state, `AudienceInput` plain-data events mapped from `CombatEvent` (`AudienceInput::from_combat`), the DEATH-D causes (`credited_hero`) and the CINE feat (`ChainFeat`), unique-target awards and anti-farming, assist and environmental bonuses, authored `AnticipationCue` schema with `validate_cues`, fixed-step decay with the milli-remainder, trend up/down/flat with cooldown, display refresh and count-up, `format_count` (5 glyphs, `999M+` cap) and `text_ink_width`, `bounded_name`, snapshot restore on `Restart`, locked/unlocked, `digest`. Tuning is one table (`TUNING`), all hypotheses. Fixed-step unit tests cover every rule. Replay goldens unchanged. |
| **C, HUD integration** | **done** on PR #118 (details and evidence below); #120 and CINE-01 are merged |

### Part C integration (done)

The list below was the part C TODO. Each item is implemented; see "Part C as built" for where it differs from the plan.

- `PlateView.audience: Option<AudienceView>` in `src/hud.rs`, filled by `HeroId::index()` (never by `Control`), `None` while locked. `HudRuntime` fills it from `AudienceState::view` (no `src/hud_adapter.rs` mapping layer was built), and drawing uses `AudienceView::display_count(reduced_motion)`.
- Draw list in `src/hud_draw.rs` at the section 3.1 coordinates: eye, 5x7 digits via `format_count`, trend glyph (amber/`magenta_hi` up, `amber_dim`/`amber` in high contrast down); `FrameId::ALL` grows with part A's regions. Apply `bounded_name` with the real Jersey name width.
- Event mapping sites (one explicit fan-out, never a second `take_new_events`): map each tick's new combat events with `AudienceInput::from_combat`, map CINE moments into `ChainFeat` once CINE-01 merges, report hero x for authored cues, and call `AudienceState::step` once per fixed tick only when unlocked. Add the `DropHazard` arm and the boss-target test after #120 merges (the exhaustive `credited_hero` match will force it).
- Preview and QA: `--hud-preview viewers*` (presentation-only, PREVIEW tag) and an `audience-probe` fixture that unlocks at step 1 and prints `AUDIENCE_STATE`.
- Tests: overlap and format tests at 640x360 and 640x400, a test that locked Floor 01 hashes and captures are unchanged.
- Captures: locked, stable, spectacular, declining, anticipation, `viewers-max` (`999M+` on Carl, `99.9K` on Donut), boss and post-mana combinations at 360 and 400, 1x and 3x, with high contrast and reduced motion, compared with `docs/evidence/hud-hybrid-a/`.

### Part C as built

- **Model:** `PlateView.audience: Option<AudienceView>` (`src/hud.rs`), indexed by `HeroId::index()`, `None` while locked; `RealHudInputs.audience` is filled by `HudRuntime` from `AudienceState::view`. Drawing uses `AudienceView::display_count(reduced_motion)`. There is no `hud_adapter` mapping layer: the draw list (`src/hud_draw.rs`, `viewer_readout`) places `FrameId` frames, and `FrameId::ALL` grew from 31 to 47.
- **Where the state lives:** in `HudRuntime`, as an observer beside the notice queue (not in `CombatState`, not in a hash). It reads the events through the HUD's own `EventCursor`, so there is no second `take_new_events`. `print_combat_events` returns the CINE moments as `ChainFeat`, which `fixed_step` forwards (`push_chain_feat`); the pre-simulated runs (every headless replay and the fast-forward of every capture) route the same feats through `AudienceFeatFeed` (its own `CineAdapter` and cursor, no printing), so headless, fast-forward and live end in the same audience state for the same inputs (test `a_cine_chain_award_reaches_the_audience_on_the_headless_and_live_paths`); `step_hud` reports both heroes' x for authored cues. It steps once per fixed tick, only while unlocked.
- **Mapping:** `AudienceInput::from_combat` covers `EnemyDefeated` (melee, `ShoreBlock`, `ShoreBlockEdge`, `Explosion`, `DropHazard`, including the boss target), `Restart` and `Respawn`. Donut's swat, pounce and Magic Missile kills already log `instigator = Some(MoveId::hero()) = Some(1)` (`src/combat_scene/donut.rs`), so they credit hero 1 and never hero 0; unit tests pin it. `credited_hero` gained its `DropHazard` arm.
- **Preview and QA:** `--hud-preview viewers | viewers-rising | viewers-falling | viewers-max` (presentation only, they keep the real plates) and `--audience-probe [--audience-cue-x x]` (unlock at step 1, both at 250, one authored cue, `AUDIENCE_STATE` line). Floor 01 stays locked (F15).
- **Colours (deviation from the part A note):** the amber spike number is the `text_white` mask times a GPU tint, which renders (244,194,74) rather than `amber` (255,194,74): the mask red 244 cannot be raised. The code-drawn dot uses the rendered colour so it matches. The down arrow's tint is tuned so `amber_dim` is exact (168,116,42); the older warning-icon tint lands 2 to 4 levels low on this art. `scripts/check_hud_capture.py` allows only the amber rendered colour and the dim arrow, within one level per channel and only inside the two viewer readout rects (everywhere else the palette is exact; the CI runner's software Vulkan rounds the linear multiply one level off a hardware GPU), and it still proves flat 2x2 blocks. The pixel comparison against the part A mocks (`docs/evidence/audience-01/compare-mock-*.txt`) differs only in those red values on Carl's amber number.
- **Evidence:** [`docs/evidence/audience-01`](../evidence/audience-01/README.md).
- **Not done (unchanged):** the in-game unlock beat (AUD-Q1), authored Floor 01 cues, persistence, stage-end ranking. The run unlock and the per-room carry are designed in [AUDIENCE-RUN](../01-design/audience-run-design.md) (implemented in the AUDIENCE-RUN PR, game feel pending). Anticipation is shown in captures only through the QA cue; it uses the same cue as a spectacular gain, as designed.

## Rights and provenance

Original design direction and HUD interaction specification developed in a 2026-11-19 ChatGPT project conversation with owner review. UI-only drawings and reference overlays are conceptual; no third-party sprite extraction or raster concept reuse as runtime assets. Fan-concept art follows owner-reported `ART-PUBLICATION-ALL-V1` rights scope in `docs/04-production/rights-and-release.md` (no general redistributable license). The independent review of PR #118 passed with fixes (the fix round is applied). Real performance measurements and human game-feel acceptance are pending.
