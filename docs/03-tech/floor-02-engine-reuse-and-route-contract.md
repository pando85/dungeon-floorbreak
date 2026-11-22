# F02-ARCH-01 — Verified engine reuse and a safe Floor 02 route seam

**Status:** technical proposal based on inspecting current main files on 2026-11-22, not implemented or locally tested. Read [Floor 02 bible](../01-design/floor-02-last-lesson-design-bible.md), [F02 workorder](../07-agents/floor-02-autonomous-workorder.md), AGENTS.md and existing F01 tests. Re-audit latest main immediately before implementation; live code can change rapidly.

## 1. What exists today (inspection findings; not guesses)

| Concern | Verified path and interface | Available | Floor 02 need |
| --- | --- | --- | --- |
| Gameplay | `src/combat_scene.rs`, `src/corridor.rs`, `src/party_scene.rs` | real fixed-step combat/party state | add authored F02 enemies/room states, don't duplicate core |
| Bevy frontend | `src/bin/floor01_scene.rs` and modules | actual renderer, input, screenshots, performance | F02 dispatch/adapter, not necessarily a new app |
| Late-room launcher | `src/f01_late.rs`, `assets/levels/f01_late/manifest.json` | room listing, static story order, data hashes, relaunch-with-ledger | floor-neutral route or compatible adapter; **NOT an E2E campaign** |
| Geometry | `src/camera_lab/room.rs`, `assets/levels/floor01/*.room.json` | `floor01-demo-hall-v1` authored geometry, two spawn points, floor, ceiling and perches | F02 room data in same validated format initially |
| Encounters | `src/encounter.rs`, `assets/levels/floor01/*.encounter.json` | `floor01-encounter-v1` sidecars: roster, checkpoints, locks, exit, hazards/props and reachability | new enemy/prop kinds need explicit compatibility changes/tests |
| Prop authority | `src/env_prop.rs`, `src/env_hazard.rs`, `src/blast.rs` | typed melee/launched-body/blast/projectile/shove/prop stimuli, source attribution, MAX_CHAIN_DEPTH=4 | jug ignites only on actual ignition; source-attributed sign/conveyor |
| Story | `src/story_unlock.rs` and `assets/data/gameplay/story_unlocks_v0.json` | once-only capability grants, story pins, snapshot/carry between F01 rooms | Floor 02 story flags/transition rules, not premature special-grant invention |
| AI and evaluation | `src/companion/*`, `src/autoplay/*`, `src/bin/autoplay.rs` | existing companion, scripted evaluators and replay | validate new families, both lead heroes, no privileged test policies |
| Human evidence | `src/playtrace/*` and `src/bin/playtrace.rs` | live session journal, reset descriptor, data pins and strict verification | add F02 scene reset and pins without rewriting old sessions |
| Art | `docs/02-art/asset-pipeline.md`, `scripts/pixelart/*` | editable source, atlas/manifest and 1x visual tests | original F02 art with provenance and silhouette gate |

**Known contract detail:** the current Floor 01 late manifest contains listed rooms boiler/hazards/juicer/mastery and a larger static story order; it is explicitly a **launcher for existing scenes**, not a complete Floor 01 story route. An agent must not announce that Floor 02 is integrated merely because it has 10 loadable standalone fixtures.

**LDtk distinction:** `docs/03-tech/level-data.md` states LDtk as intended source of truth, but the currently inspected working data uses `floor01-demo-hall-v1` JSON with strict encounter sidecars. Favor an adapter and migration plan; do not introduce a second unverified LDtk native pipeline as a Floor 02 critical-path dependency.

## 2. Proposed integration architecture (PREFERRED, subject to local code audit)

1. Keep F01 CLI defaults, story grants and pinned fixtures working unchanged. Prefer extracting a **small generic route-manifest reader / validator** from the proven F01 late manifest, and keep F01 entry points as compatible wrappers. If this extraction touches too many hot files, write a **narrow F02 reader** that reuses current room and encounter parsers, with a documented consolidation follow-up. Never duplicate simulation.
2. Keep authored F02 source under `assets/levels/floor02/`; use the existing room parser first. Add new encounter kinds through the existing strict enum/validator with a backwards-compatibility suite. Do not weaken unknown-field rejection or enemy caps.
3. Implement F02 pure gameplay as narrowly scoped `src/f02_*` or `src/floor02/*` modules selected by the local integrator. Exactly one hot-file owner modifies `src/lib.rs`, shared scene dispatch, `src/encounter.rs`, story ledger, `src/combat_scene.rs`, and replay registration. Do not stuff all behavior into the huge F01 Bevy frontend.
4. Stage/encounter data, story-event ledger and presentation state remain **separate**. The existing capability ledger is not a free-form narrative flag store; adding `rage_exiled` / `mongo_met` may require a small story transition/flag abstraction. Do not falsely claim these are current APIs.
5. Extend Playtrace reset descriptor and scoped data pins for F02 *only after confirming actual code*. Previous F01 replay, fixture hashes and recordings must retain their validity or fail explicitly with exact mismatch; never rewrite old traces just to go green.
6. Runtime IDs remain IP-neutral, with fan-concept art/story naming in the replaceable adapter, per rights register. No public playable release rights are inferred from art publication.

## 3. Proposed route data (illustration only — NOT implemented schema)

~~~json
{
  "schema": "floor-route-v1-proposal",
  "id": "floor02",
  "start": "f02_s00",
  "chapters": [
    {"id":"a","beats":["f02_s00","f02_s01"]},
    {"id":"b","beats":["f02_s02","f02_s03"]},
    {"id":"c","beats":["f02_s04","f02_s05"]},
    {"id":"d","beats":["f02_s06","f02_s07"]},
    {"id":"e","beats":["f02_s08","f02_s09"]}
  ],
  "transitions": [
    {"from":"f02_s07","to":"f02_s08","event":"rage_exiled","party_floor":2},
    {"from":"f02_s09","to":"floor03_entry","event":"mongo_met","requires":["ralph_defeated"],"party_floor":3}
  ]
}
~~~

The F02-C0 architect must map this idea to actual current types and tests, rather than compiling this JSON by rote. A single player-facing continuous chapter may contain multiple internal SXX data beats without loading screens; internal room ID is not identical to player-facing chapter.

**CRITICAL:** S07 is a **Rage-only actor expulsion into the stairwell**. Carl/Donut MUST remain on Floor 02 via the protected side corridor to S08. Only post-Ralph+Mongo S09 sets the party's true Floor 03 exit. These must be two distinct authoritative events with a negative regression fixture.

## 4. Physical and behavioral reuse details

- The real prop `Stimulus` already supports melee, launched body, projectile, blast, enemy shove and authored prop contact with cause IDs. **Projectile contact is not equivalent to ignition.** A plain Magic Missile may dislodge a sales jug but cannot automatically count as a fire source unless the actual damage profile says so. Keep existing chain-depth 4.
- Room/encounter parsers are strict. Their props/hazards/roster are typed. New F02 kinds must pass explicit serde, reachability and forward/reverse compatibility tests, using ordinary data not magical custom string dispatch.
- Existing environmental mechanics include falling mass, blast and support interaction. Prioritize these for the sales sign rather than implementing a parallel object rigid-body engine.
- Current story ledger has a per-hero equipped-special grammar and once-only story grants, but not all desired F02 narrative flags/spells. **Second Chance and new Jug O' Boom equipment are not required.** World props first.
- The current `src/f01_late.rs` comments describe scene relaunches carrying the story snapshot (`story_snapshot` / `carry_story`); this is an integration lead. Confirm the real call path and whether checkpoint rollback remains correct before generalizing.

## 5. Unit of delivery and truthfulness

**Isolation proof:** command launches S02 independently and tests physical/warning events; a bot strict replay of the room passes. Useful but **not sufficient to declare the floor done**.

**Route proof:** a regular `story` loadout enters the Floor 02 beginning, carries the actual earned Floor 01 capabilities, travels all five player-facing chapters and crosses S09 to Floor 03 with no premature grant. If Floor 01 E2E is not implemented yet (the currently open PR #194 is design-only), flag the bridge as **BLOCKED ON F01-E2E**, but allow isolated Floor 02 scene work.

**Physical 2P:** a scripted two-slot headless path is not the same as two real gamepads. Real 2P and game feel remain owner gates.

## 6. Integration task IDs and ownership

| Task | Prerequisite | Deliverable | Shared hot-file ownership |
| --- | --- | --- | --- |
| F02-ARCH-00 | none | current main SHA, actual code map, risk/hot-file ownership and F01 compatible CLI list | documentation / architect |
| F02-ARCH-01 | ARCH-00 + pretrip baseline for cross-core changes | pure floor/chapter route parser/validator or narrow compatible adapter | integrator only |
| F02-ARCH-02 | ARCH-01 | S00 starter room loaded through real room/encounter validator, both hero seeds | integrator for central loader; stage author separate |
| F02-ARCH-03 | ARCH-01 | story/party checkpoint, reset and S07 vs S09 transition event policy | integrator |
| F02-ARCH-04 | ARCH-02 | real Bevy launcher and 640x360/400 native capture | integrator, art agent supplies separately |
| F02-ARCH-05 | ARCH-03/04 | linked S00->S01 and S02->S03 route, real Playtrace pins and strict replay | integrator + QA |

Avoid opening a single massive engine rewrite PR. Prefer small locally validated PRs on separate source directories and a single integration owner. Do not edit the F01 manifest hash or a golden without a documented real change.

## 7. Acceptance and explicit stop-lines

- ARCH-T01 original F01 commands, scenes and parser checks continue to pass.
- ARCH-T02 F01 combat/party/story/replay goldens unchanged absent explained legitimate migration.
- ARCH-T03 F02 starter room loads with both hero identities and normal direct path; no new button.
- ARCH-T04 strict encounter rejects unsupported F02 enemy kind, then accepts it only once explicit mapping/tests are implemented.
- ARCH-T05 illegal S07 player-Floor03 transition rejected, legal S09 transition works after Ralph/Mongo.
- ARCH-T06 F02 recordings include appropriate data pins and replay; F01 recordings not silently corrupted.
- ARCH-T07 2-slot scripted route and solo AI run measured where supported, with human 2P status still NOT RUN.
- ARCH-T08 actual Bevy gameplay screenshot/GIF for 640x360/400, not AI concept art.
- ARCH-T09 final-head local format, clippy, tests, docs, art provenance and replay validation recorded with actual commands.
- ARCH-T10 cross-cutting combat/party/camera changes hold until owner-accepted Floor 01 gameplay baseline; rights/canon conflicts remain human stop.

**No code or tests delivered in this document.** It is a conservative architecture brief for a local implementation agent with repository checkout.
