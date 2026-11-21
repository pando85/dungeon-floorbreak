# F01-E2E-01 — Local orchestration handoff: first complete Floor 01 run

**Read first:** [the full production review / gates](../04-production/floor-01-end-to-end-production-plan.md). This is a **new layer of release planning**, not a replacement for [the merged late-floor playtest integration plan](f01-playtest-integration-plan.md) (#182) or its draft revisions #190/#193. **No code or playable floor is implemented by merging these documents.**

## One-sentence mission

Take the existing combat foundations and approved Floor 01 scene designs from **isolated demos + independent boss** to **one genuine start-to-finish playthrough with story-correct abilities, fair solo/co-op, actual checkpoint continuity, a winnable Ball of Swine and a protected exit**, then add optional physical spectacle. Do not claim `f01_late_mastery` (a debug ability/wave arena) is the story finale.

## Owner-facing priority order

1. **Keep the active P0 late-floor work moving:** DM-A (#186), CP-A already merged (#183), other Carl/Donut/companion stages, BOIL core (#176), ENEMY-SLOT, PT-SCENE, Hoarder or hazards plus Juicer/mastery. The owners and hot-file queue already exist in #182/#190/#193. Do not fork them.
2. **New independent work now:** F01-E2E-ROUTE-0 (a data-only route/edge/capability/checkpoint contract and validator), and F01-BSW-A/B (a **new owned-module** Ball direct-only arena + deterministic movement/collision boss fixture). Both should avoid `src/bin/floor01_scene.rs` and `src/combat_scene.rs` until integration owner assigns the hook.
3. **Prepare Act I concurrently without hot-file edits:** Dozer/ENV-02 1P+2P geometry/camera charge-feasibility greybox, #181 Infernal Corral authorable room & adversarial evaluator, #187 restaurant visual and reward geometry. Do not merge new power grants here.
4. **After ENEMY-SLOT/prop core stabilizes:** wire Hoarder, Slime/Rot, trogs and Tusklings through one engine pathway. Moving gates, optional Hoarder collapse, gym bench and Ball detachment follow **real** source-attributed collision.
5. **Only then perform F01-CONTINUITY wiring:** fresh story start → actual rooms → Hoarder/restaurant/Juicer grant → Ball actual win → true Floor 01 exit, preserving standalone late mastery as a test scene.

## Required F01-E2E-ROUTE-0 artifact contract

Choose existing repo naming, schema and room-loader conventions **after inspecting the actual current branch**; the names below are abstract.

- A route manifest with stable **stage IDs, room IDs, upstream/downstream edges, normal/optional labels, enter/exit checkpoint ids, party-friendly protected entrance/exit coordinates, expected pre/post story capability flags**, source encounter PR and state `implemented_in_main` vs `placeholder_validated` vs `blocked`.
- Ordered mandatory path covers: **opening Dozer → first goblin + Guild/biscuit → #181 corral → fast-food / Light on Your Feet → Hoarder/dynamite → Boiler → at least one authentic Slime/Rot threat → safe Meadow transition → Juicer/gauntlet+blink → Ball of Swine → final protected exit**. Broken Ring and troglodyte training rooms have legal short placeholder routes at P1, then full authored encounters by P2. Dozer revenge optional.
- **No test-only `all` loadout** and no silently fabricated grant to skip a missing story interaction. For isolated greybox test entries, mark debug/fixture loadout explicitly and **do not** count them as a completed story route.
- A pure **validator** for graph connectivity, both-player path potential (using current height/jump contracts, not magic arbitrary teleport), impossible grant prerequisites, duplicate story grants, unreachable post-destruction exit and boundary flags. In addition, a deterministic test-fixture route that uses the real level/party sim once wired.
- At first, allow stage entries whose gameplay is **not implemented** to be tracked without accidentally loading as if shipped. The manifest must **not** report these as playable. The eventual E2E greybox gate requires a real playable implementation of each mandatory room, even if some art/optional combat is provisional.
- Keep `f01_late` and `f01_late_mastery` test IDs distinct from the **new Ball narrative boss ID**. There must be **one** canonical floor-complete/exit receipt and actual story-ledger carry across room scene relaunches.
- Add a focused README with the current valid launch/test commands **observed on the actual branch** and a clear list of unimplemented commands. Do not copy a hypothetical CLI command from the integration plan and claim it runs now.

**Proposed first PR gate:** graph/capability/test-state data validated with actual unit commands, no edit to shared hot gameplay files, a short source-truth table for each scene, and clear next integration API/owner.

## Required F01-BSW-A/B first implementation contract

- Native SWINE-01 art size **128px** authoritative, six author-rasterized rolling frames (no runtime sprite rotation or rescale); initial collider is a tested data profile near 60px radius, not guessed from sprite alpha.
- One isolated true arena fixture, ~1200px initial width and **actual wall-protected safe pocket(s)**. No "jump over 128px" for Carl. Shared camera/2P fairness, no offscreen early charge.
- Boss state `readable 36tick tell → committed movement → real swept collision/stop/rebound → punishable recovery`, plus ordinary direct HP damage and **legitimate defeat** with source attribution; real data-backed movement and no invulnerability gimmick.
- **First playable P1 milestone is direct-only.** No central gate, detachments, UV show lights or dramatic final results required for that milestone. If the fight is dull or impossible, fix before adding features.
- Actual Rust headless fixture with solo Carl, solo Donut, 2P positions and reproducible same-input replay, Bevy-native 640×360/400 evidence and real safe response to at least one committed charge.
- Tuskling domain can start separately only if its native actor/ENEMY-SLOT owner is known; max 3 active, finite detachment budget and actual 30px assets are a **later BSW-D** gate. No 30-body simulation.

**Source:** [BSW final encounter](../01-design/floor-01-ball-of-swine-finale-encounter.md) and [BSW work order](floor-01-ball-of-swine-finale-workorder.md) on #192. The first implementation should reference #192, not duplicate the entire design in a competing new spec.

## Real adversarial and manual checks before declaring stages playable

- Main Carl can reach/beat every required encounter with legitimate kit (without using environment where a direct route is promised); solo human Donut and real P1+P2 have a fair route.
- **STATIONARY-DONUT** natural position sweep in #181's corral, gym lead-in and Ball; real legal press edges and mana use; no false success from Carl AI dealing all damage.
- P2 delayed by 0/96/224/448px under Dozer charge or Ball commit, no camera-hiding unavoidable attacks. Playtrace state hashes and event ordering with fixed 60Hz inputs.
- Crash/reset test during environmental impact, boss hit, Tuskling detach, checkpoint and true scene transition. No duplicated or missing story unlock.
- Direct-vs-environment win A/B for Hoarder, Boiler and Ball; boss clear time and non-action time. **Optional** spectacle must not be the only valid way to play.
- One real **start-to-end story run** with a human Carl-active, and one with real two pads (not stitched videos), at native resolution and later 1080p/handheld-like output. Owner game-feel verdict is an independent, unsynthesizable acceptance condition.

## Status labeling and evidence discipline

Use these four axes in PR descriptions and the standard repo backlog, **not a new contradictory tracker**:

```text
design:     draft | merged
runtime:    not_started | branch_only | merged
tests:      not_run | headless | live_solo | live_2p | live_e2e
owner_feel: pending | accepted | changes_requested
```

Every promotion cites the actual merged SHA + runnable commands + traces/screenshots. A green docs check is **not** evidence of a playable scene. A local bot test is **not** evidence of co-op feel.

## Suggested reporting packet for the next user playtest

- **Build:** main SHA / launch flags / scene IDs / two controller models / native resolution / output resolution / FX profile.
- **Completeness:** stages reachable from normal start, missing placeholder rooms, correct grants, checkpoint/exit.
- **Gameplay:** direct vs environmental clear times, deaths, max idle wait, AI contribution, stationary exploit outcomes, visual warnings missed.
- **Quality:** real Playtrace session links/hashes, native actual footage per significant encounter, perf spikes under full Boiler and Ball Tuskling count.
- **Questions:** could you anticipate danger, do both heroes matter, does each stage play differently, does the first half remain fun after five replays, was Ball spectacular *because of you*, would you replay?
- **Status:** PASS / FAIL / BLOCKED / NOT RUN per room and per gate, plus next narrow PR.

## Do not expand scope

No new bosses/enemies/skills beyond the already authored roster, no third controller, new inventory/equipment system, Mongo, network rollback, reward/sponsor economy, procedural exploration, 30 independent pigs, camera sampling rewrite, or new Rust game engine. No art cut/paste from concept PNGs. No assertion of commercial rights to a franchise-derived executable.

## Next orchestrator action

1. Read and review this PR plus #192; coordinate with #182/#190/#193 leads.
2. Create a focused implementation PR for **F01-E2E-ROUTE-0** (data + graph validator) and a separate focused PR for **BSW-A/B**, when ownership is assigned.
3. In parallel keep the existing P0 late-floor path moving and arrange an **actual quick owner CF02/Donut co-op playtest** before adding fancy encounter effects.
4. Once those two new tracks have runnable evidence, plan F01-CONTINUITY hot-file wiring through the existing integration queue.

These are **concrete future local-agent tasks**, not statements that work has started or an asynchronous commitment by this ChatGPT session.
