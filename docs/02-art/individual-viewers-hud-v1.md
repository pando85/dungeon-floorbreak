# AUDIENCE-01 — Individual live viewers in the Hybrid-A HUD

**Status:** owner-approved direction; proposed design/implementation contract, **not implemented**. 2026-11-19.
**Authority:** \`AGENTS.md\`, \`docs/02-art/hybrid-hud-a-design.md\`, \`docs/02-art/hybrid-hud-a-native-spec.md\`, \`docs/01-design/floor-01-gameplay-progression-v0.md\`, \`docs/01-design/character-progression-horizon.md\`. Where this proposal conflicts with current gameplay/input/unlocks, those contracts prevail.

## Decision and scope

- Live **viewers are the visible arcade score**, measured **separately for Carl and Donut**, identified by stable **hero ID**, never by controller slot. Solo companion actions score for that hero; two-player mode does not merge their totals.
- Viewers track *entertainment*, not merely competence: combat, creativity, teamwork, embarrassing failures, and *authored* anticipation of looming trouble can attract attention.
- Keep an internal, deterministic spectacle/event scoring model; no second on-screen spectacle bar.
- **No counters at Floor 01 start**: neither an OFFLINE box nor zero placeholders. An explicit later narrative broadcast unlock makes both available together. The precise story beat is **TBD with canon/progression review**, candidate after introductory System/Tutorial Guild material. It is separate from Donut's biscuit/mana unlock.
- No subscriber economy, sponsors, achievements/buffs from score, stage-end results implementation, or persistent cross-run economy in v1. Stage-end summary design is separate.
- Preserve the existing two Hybrid-A plates, avatars, HP bars, Donut-only post-biscuit mana, owner tags, ability chips, centered location/boss strip, System CRT and all controller semantics. No new permanent panels.

## Native HUD layout (640x360 and 640x400; pixel units)

Both hero plates stay at the exact existing positions/sizes: Carl \`(8,8,164,48)\`, Donut \`(468,8,164,48)\`. Place the audience readout **inside the existing name row**, *not* in the middle gap or beneath the plates.

| UI element | Plate-relative bounds | Carl absolute | Donut absolute |
| --- | --- | --- | --- |
| Eye icon | 7x7 at (+92,+4) | (100,12) | (560,12) |
| Number field | x=+106..+150, y=+4..+14; right-aligned | x=114..158 | x=574..618 |
| Trend glyph | 5x7 at (+153,+6), reserved even when hidden | (161,14) | (621,14) |

These are **proposed** pixel rects; the executor must verify actual atlas/font metrics at 1x and may adjust within the name row **without shifting another existing field**. The current names are 30px (\`CARL\`) and 40px (\`DONUT\`) at +40,+4, leaving room to the right. The HP bar begins at +50,+16, so the readout must not descend into that row. The 3px frame ring and action lane y>=60 remain untouched.

Styling: reuse \`hud_ink\`, \`plate_fill\`, \`brass\`, \`amber\`, \`amber_dim\`, \`magenta_hi\`, and the native font pipeline. **No additional frame or permanent VIEWERS label**; eye glyph + compact number + optional up/down indicator only. Existing \`Jersey 10\` is first choice for numbers if 1x fit checks succeed; otherwise a new original tiny bitmap digit glyph set (editable pixel-grid source). Create original editable art: never crop concept PNGs into the runtime HUD. Extend the existing HUD atlas/source/export pipeline; do not introduce \`bevy_ui\` or a separate render camera.

## Number formatting

Format the **display only**; model values are untruncated integers:
- \`0..999\`: full integer.
- \`1,000..99,999\`: one decimal K, **truncate**: \`1.0K\`, \`12.4K\`, \`99.9K\`.
- \`100,000..999,999\`: integer K, truncate: \`159K\`, \`999K\`.
- \`1,000,000..9,999,999\`: one decimal M: \`1.2M\`.
- \`10,000,000..999,999,999\`: integer M: \`12M\`, \`999M\`.
- \`>=1,000,000,000\`: \`999M+\` as a bounded display sentinel until a wider scale is designed.
- ASCII digits and invariant \`.\` suffix, no locale separator; right-aligned fixed field. Test at native scale with arrows and \`999M+\`.

## Visual state

| State | Rendering | Animation (60Hz fixed tick) |
| --- | --- | --- |
| Locked | no eye, number, trend or placeholder | none |
| Stable | brass eye + amber count | none |
| Small gain | new count, amber | none |
| Spectacular gain | magenta number and up arrow | bright highlight 12 ticks, up arrow up to 48 ticks |
| Material fall | amber number and dim-amber down arrow | arrow up to 48 ticks, cooldown suggested 600 ticks |
| Suspense / imminent event | same as spectacular gain | *no distinct warning icon or explanatory text* |

Suggested large gain threshold: \`delta >= max(300, current_before * 15 / 100)\`. Suggested meaningful decline: cumulative fall \`>= max(250, reference * 10 / 100)\`; routine small decay remains quiet. Count transition may interpolate over <=30 ticks in whole pixels/integers; authoritative values update instantly. Normal display refresh <=5Hz except on a meaningful event. New spikes retarget; no unbounded queue. No camera shake, HUD shake, particles or interference with danger tells. Respect \`--hud-reduced-motion\` (no flash/interpolation) and \`--hud-high-contrast\` (direction never color-only). Boss warnings and health always outrank spectator effects.

## Domain model and candidate tuning (not final balance)

Implement a Bevy-free deterministic domain, e.g. \`src/audience.rs\`, driven by real fixed-tick combat/encounter events. Keep \`current_viewers\`, \`peak_viewers\`, \`last_interesting_tick\` and anti-duplication/anti-farming state **per stable hero**. Rendering may read but must never affect the simulation or replay hash. Suggested unlock baseline is **250 each**.

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

- \`locked -> unlocked\` is one explicit game/story event, separate from \`mana: Option<ManaView>\`. Persist/restore according to current story checkpoint contract (not by a new standalone save system).
- \`HudSnapshot.plates[i].audience: Option<AudienceView>\` is \`None\` until real unlock or explicit debug preview. \`Some\` holds the integer count and presentation trend/cue age. Avoid deriving hero identity from P1/P2.
- Provide a **clearly labelled PREVIEW VIEWERS** fixture with locked/normal/rising/falling/anticipation; preview must never change simulation, RNG, input or replay hashes.
- Future stage-end UI may consume \`peak_viewers\`, ending \`current_viewers\`, per-hero spectacle events and contributor IDs, but its presentation and winner metric are **not designed/approved in this PR**.

## Integration boundaries

Update the existing HUD model/draw/adapter only after the domain contracts exist: \`src/hud.rs\`, \`src/hud_draw.rs\`, \`src/hud_adapter.rs\`; the native pixel grid \`assets/source/hud/\` and \`assets/manifests/hud_hybrid_a_v1.json\`; existing export script, evidence and tests. Retain one HUD screen-space camera and current fonts and atlas rather than creating a new separate HUD. If current code has moved since this spec, adapt to the *live* main interfaces rather than duplicating state.

## Acceptance gates

1. Floor 01 starts with **zero visible audience decoration**; unlock reveals exactly one count per hero, no other HUD layout shift.
2. Source/atlas pixels, native captures, controller rows and HP/mana bars remain correct at 640x360 and 640x400, 1x and integer-scaled 3x.
3. \`999\`, \`1.0K\`, \`12.4K\`, \`159K\`, \`1.2M\`, \`999M+\` and trends fit without text/name/HP/frame collisions; test \`DONUT\` worst-case name width.
4. Solo, swap and local 2P attribute viewership to stable hero identities. Assist and repeat-event logic is tested; no scoreboard cloning at control switch.
5. Real authored anticipation raises selected heroes' counts *before* on-screen action; no warning text, no universal hidden-enemy detector.
6. Deterministic fixed-tick decay, no underflow, no double-award, no RNG/non-deterministic rendering feedback, checkpoint-safe reset.
7. Boss strip, System notices, warning tells, high contrast and reduced motion work together; compare against real capture in \`docs/evidence/hud-hybrid-a/\`.
8. \`cargo fmt --all -- --check\`, \`cargo clippy --locked --all-targets -- -D warnings\`, \`cargo test --locked --all-targets\`, \`python3 scripts/check_docs.py\`, \`python3 scripts/validate_assets.py\`, native render smoke checks pass **after implementation**; report actual evidence, not assumed passes.

**Open decisions:** exact canonical unlock beat; final scoring/restart balance; stage-end result ranking. The owner accepted the compact individual HUD direction, **not** those untested tuning constants.

## Rights and provenance

Original design direction and HUD interaction specification developed in a 2026-11-19 ChatGPT project conversation with owner review. UI-only drawings and reference overlays are conceptual; no third-party sprite extraction or raster concept reuse as runtime assets. Fan-concept art follows owner-reported \`ART-PUBLICATION-ALL-V1\` rights scope in \`docs/04-production/rights-and-release.md\` (no general redistributable license). Implementation, independent review, real performance measurements and human game-feel acceptance are pending.
