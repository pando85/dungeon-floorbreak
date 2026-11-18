# I03-KILLS — Integrated goblin defeat variants, collision and rock kill

> **Owner direction update — 2026-11-18:** Review the proposed [universal monster death language](../01-design/universal-monster-death-language-v0.md) **before** locking new death sprites. The creative target now explicitly includes catastrophic body explosions and decapitation for monsters when the attack/environment supports it, not merely polite knockdowns. Preserve the existing CP-01/02/03 deterministic implementation priorities, original GOB-01 source poses and story-unlock gates. The new shared gore system and extra finishes remain **design hypotheses pending owner visual review**, not a mandate to implement all types inside I-03.

**Status:** EXECUTION HANDOFF / **DRAFT IMPLEMENTATION PR SEED**, not gameplay already written, tests already passed, or a new art approval. Local agent work should be committed to this same draft PR after rebasing current `main`, and the draft should become ready only with working Rust+Bevy code, evidence and independent review.

**Parent gameplay design:** [PR #78 Act I first encounter](https://github.com/pando85/dungeon-floorbreak/pull/78), especially `docs/01-design/floor-01-first-goblin-encounter.md`; **if it is still draft, keep any new choreography experimental**. Current death-art audit is [goblin defeat language](../01-design/floor-01-goblin-defeat-language.md). Continue existing [issue #40](https://github.com/pando85/dungeon-floorbreak/issues/40), [combat probe spec](floor-01-integrated-combat-probes.md), [ENV-02](../02-art/env-02-opening-spec.md), [GOB-01](../02-art/gob-01-goblin-spec.md), and [ART-INT-02](../evidence/art-int-02/README.md). Do not create a parallel permanent combat demo or replace the approved opening route.

## Existing starting point (verified on main 2026-11-18)

- `src/combat.rs`: common deterministic combat domain used by existing Bevy combat fixtures.
- `src/combat_anim.rs`: GOB-01 A/B native frames mapped to sim states; `knockback_contact` (26) and `getup` are mapped but **have no corresponding CP-02 simulation state yet**.
- `src/bin/floor01_scene.rs`: `--fixture cp01_single_goblin`, `cp01_kick`, `cp02_two_goblins`, etc., plus `--manual`, capture and traces. The *normal story scene* does **not** contain enemy combat yet according to its docs; do not call existing fixture screenshots a normal-run encounter.
- `docs/03-tech/floor01-scene.md`: actual fixture state and animation integration, plus the existing known 1-pixel/contact-placement constraints.
- `assets/levels/floor01/opening_route_proposal.json`: accepted **ENV-02 option R** with S5 at x2608–3296, x2640 checkpoint, shore strike at ~2975, fall footprint x3000–3080 and goblin spawns x3136 / 3176 / 3216. This is a **proposed level route**, not yet a completed in-engine chase.
- Existing GOB-01 `hurt`, `knockback`, `down`, `corpse` and Carl kick/swing art cover D-01/D-02; GOB-02-CRUSH is a small **new art scope** and should not block domain correctness tests.

## Engineering priorities and separate agent ownership

**P0A — Protect CP-01 and migrate inputs:** one gameplay owner only.
- Preserve existing fixtures, scripted seeds, replay hashes and current manual test keys while adding the **device-neutral** `PhysicalAttack` separate from `EquippedSpecial` (Triangle/Y physical, Circle/B future special). The old temporary East-button-as-kick mapping belongs only to compatibility/test adapters until tests update. No pre-Guild bombs, no Carl enemy grabbing.
- Reuse `src/combat.rs` combat/animation state, never create a new independent battle engine. Keep physics deterministic at 60Hz and art read-only.
- Do not require working duo AI, a full Dozer chase or new artwork to run the headless CP fixtures.

**P0B — CP-02 launch/contact, first no damage:** same combat owner, sequentially.
- Kick one **living** goblin into another; stable ordered contact event, one reaction per `launch_id, target_id`, cap knockback, use GOB-01 `knockback_contact` (26) and second `hurt` (16–17).
- First implementation is *interruption/stagger only*, no automatic contact damage. Damage, chaining and corpse projectiles are **feature-flagged hypotheses, OFF** until a later playtest.
- Validate wall, floor and enemy contact separately. No through-wall hits, infinite ping-pong or duplicate events. Preserve existing CP-01 animation timing and hit-stop.
- If body geometry is revised to the proposed airborne 28×18 torso box, update collision + pivot placement and tests coherently; do not silently stretch the standing 24×38 hurtbox.

**P1 — CP-03 ENV-02 rock, one authored impact:** one scene/terrain owner coordinating with gameplay owner.
- Read **existing** ENV-02 level markers for the diagonal shore and hazard footprint. Author states `intact -> damaged_1 -> damaged_2 -> broken`; one deduplicated hit per melee attack instance, three distinct hits as current *level-art hypothesis*; stone `seated -> settling -> slipped -> falling -> landed`.
- The block's fall is **authored, localized and deterministic**, not a general-purpose tile collapse solver. Resolve one impact at landing against entities *in footprint*. Death credit traces to player who broke the shore, not to a global environment owner.
- Support direct kills and rock-miss outcomes; neither must lock the Guild door. Protected exit stays navigable after **all** break/no-break/kill orderings. Don't change ENV-02's accepted art pieces, layouts, palette, ceiling option R or original asset hashes without a separate approved request.
- Use the current GOB-01 `down` / `corpse` as a **clearly labeled temporary stand-in** until the independent GOB-02-CRUSH art extension is accepted.

**P1 art — GOB-02-CRUSH additive asset:** separate *top-tier visual-design* agent followed by medium-tier original pixel-source executor and independent top-tier reviewer.
- Produce only native-size `crush_contact` / `crush_remains` keys (plus in-betweens if visual evidence needs them), in both goblin A/B or a proven shared base + B overlay; match 64×56 canvas/pivot/palette and 36px source actor scale.
- DO NOT edit GOB-01's frozen Opus keys, replace the existing GOB-01 27-frame atlas, trace the concept collage or bypass the existing .ase/grid -> exported atlas -> manifest -> validator/test pipeline.
- Deliver key art at 1×, 3× GIF, and real 640×360/640×400 Bevy context frames with block occlusion; play at real speed. If new art needs a separate PR, create one **based on main**, then merge/rebase this draft PR's integration after review rather than concurrent edits to the same atlas. Do not merge unreviewed keys into production.

**P2 — Normal story route activation:** integrate only after CP fixtures prove contact and rock rules.
- Keep one `floor01_scene` binary: attach S5 skirmish and rock to the existing opening route **when CHASE-01 / ENV-02 integration is ready**. If the chase is still unavailable, offer a clearly labeled normal-run in-scene checkpoint/scenario and report the missing dependency; do not claim the chase is playable.
- Baseline: two goblins, third only as optional scenario; player may punch/launch/crush or simply kill both directly. Clear route to Guild, short restart. No cutscene auto-kills.
- Re-run with real P1 Carl + P2 Donut and solo L1 switch only after those systems have landed; do not fake proof from the passive-follower sprite.

## Practical work order / single-writer constraints

1. **Agent 1, gameplay + tests:** on this PR branch, migrate physical input and implement CP-02 in `combat.rs`, `combat_anim.rs`, and fixture adapters (rebase if main advanced). Commit with headless tests and same-tick trace. **Single code owner** for input/combat schema.
2. **Agent 2, opening scene + deterministic terrain:** after agent 1's interface is stable, integrate S5 shore/block landing with existing ENV-02 markers and snapshot tests; same PR branch or nonconflicting sub-branch. Do not simultaneously edit shared `floor01_scene.rs` with agent 1.
3. **Agent 3, GOB-02-CRUSH art pipeline:** independently prototype original additive 2-pose crush family and show native 1×/3× evidence, then integrate only after approved independent review. Art lives in its own PR if changes touch shared frozen manifests.
4. **Agent 4, QA/independent review:** inspect real Bevy full-speed captures, verify determinism/coverage, check regressions, and request human **fun** verdict. Do not award oneself approval.
5. An orchestrator rebases and resolves deltas **sequentially**, reruns checks and changes PR from draft only when all requested implementation scopes are truly present. Record the **incomplete** items explicitly; splitting CP-03/art is preferable to pretending a full green MVP.

## Required scenarios and tests

| ID | Gameplay fact | Tests/evidence needed |
| --- | --- | --- |
| `I03-DIRECT` | One goblin defeated by ordinary melee; full frame progression | Existing CP-01 regression + real-speed video + owner critique |
| `I03-KICK` | Triangle/Y triggers distinct physical kick; none triggered by Circle/B before special unlock | Controller and keyboard action adapters + input replay |
| `I03-CONTACT` | Flying live goblin touches second; each reacts once, no contact damage | Fixed-step contact trace, stable hash, `knockback_contact` Bevy capture |
| `I03-CONTACT-WALL` | Wall collision stops flight with no through-wall target reaction | Collision ordering and camera replay |
| `I03-SHORE-3` | Three unique strikes change prop states; one landing | Test no double counting from held input / hit-stop |
| `I03-ROCK-KILL` | Goblin in marked footprint dies exactly once at rock impact with correct credit | Deterministic event/hash, 1× Bevy video |
| `I03-ROCK-MISS` | Empty footprint: no damage and no fabricated defeat | Negative test, visual recording |
| `I03-SAFE-EXIT` | All break and enemy orderings leave Guild route traversable | Reachability/property scenario tests |
| `I03-RETRY` | Combat, enemies, prop, debris and attribution reset predictably | Replayed seed with unchanged hashes |
| `I03-2P/SOLO` | Both characters participate, no input theft or fake magic | **Only mark passed when actual controller/AI systems are available** |

**CI:** run repo's documented commands: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, `bash scripts/export_assets.sh`, `bash scripts/ci_floor01_combat.sh headless`, real Bevy renders/asset validators, and `python3 scripts/check_docs.py`. Some commands may require Docker/Xvfb/Vulkan/installed LibreSprite; log actual environment and skipped checks. Compare baseline CP-01 fixture hashes and ordinary corridor behavior before altering any golden trace; justify intentional fixture changes.

## Finish-line definition

This implementation PR is **not ready for merge** until:
- At least CP-01 direct kill and CP-02 physical kick + once-per-launch living goblin collision are implemented and reproducibly validated in Rust + Bevy.
- CP-03 rock kill works with the actual ENV-02 authored support or is split to an explicit subsequent PR, **not silently marked complete**.
- Documentation truthfully distinguishes fixture proof versus normal story route, native artwork present versus temporary stand-in, 1P/2P real proof versus pending.
- Human sees the **three contrasting outcomes at actual game speed** and judges basic-kill feel before final approval.

**Do not conflate a CI green build with game-feel approval. Do not mark GOB-02-CRUSH complete without source/manifest, independent art review and a real-scale visual review.**
