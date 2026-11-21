# F01-CHASE-DIRECTOR-01 — Local agent work order

**Task:** implement the actual Floor 01 introductory Murder Dozer chase as a cinematic **but fully player-controlled** playable set-piece, following [the authoritative choreography proposal](../01-design/floor-01-dozer-opening-choreography.md). This PR is intentionally a **design + work-order** start; the agent should continue on its branch to provide real code/tests/evidence, or split focused implementation PRs under orchestration if code ownership dictates. The initial documentation is not a completed gameplay feature.

## Read / branch coordination

Follow `AGENTS.md`, `docs/07-agents/workorders.md`, current orchestration/owners, and current merged code **before touching any shared modules**. Audit `docs/02-art/env-02-opening-spec.md`, `assets/levels/floor01/opening_route_proposal.json`, `docs/02-art/doz-01-dozer-spec.md`, `src/bin/floor01_scene.rs`, `src/corridor.rs`, `src/party_scene.rs`, camera and `src/scene_anim.rs`. Reuse original source art/animations (approved Dozer, ENV-02 timber/blocker), existing audio as *provisional assets* subject to review, exact native dimensions/markers.

Avoid conflicts with:
- PR #174 (Carl progression), #175 (Donut progression), #176 (Goblin Boiler agent work), #178 (Hoarder/Dozer revenge/Slime-Rot encounter concepts).
- ongoing companion movement/navigation work: do not implement its AI here; report missing integration and prove chase supports its interfaces.
- Playtrace / render replay, CF02 and camera changes. Do not silently rewrite their systems.

## Deliverables, in dependency order

| Subtask | Deliverable | Evidence / pass condition |
| --- | --- | --- |
| CHDIR-A | Wire current ENV-02 **option R**, authentic S1–S5 geometry and markers into an actual playable entry with two characters | At-speed real Bevy 640×360 and 640×400 screenshot, both party modes, no new independent level-loader fork |
| CHDIR-B | DOZ-01 h120 rig + authoritative positional fixed-step pursuit, S2 entry, S3 two one-shot timber break events, collision with wheel and 1-tile humps | Headless state and body/hitbox traces, real animation at 1x, death/retry from x1008 |
| CHDIR-C | S4 visible committed charge warning, shared x2064–2160 refuge, barrier collision/destruction and S5 honest wedge/overshoot | Beat order from real contact; exit and next goblin fight reachable; consistent camera |
| CHDIR-D | **Co-op fairness**: x1888 trigger **arms** the charge; evaluate safe reachable arrival for both live heroes before lethal commitment, including 448px separation. Preserve 8s barricade fail-open policy without unavoidable offscreen death | Deterministic two-character feasibility matrix, actual two-pad runs and fixed-seed repro when a case fails |
| CHDIR-E | Presentation: music intro/ducking, Dozer engine/whistle, two splinter beats, wheel close pass, crash/abrupt quiet, optional nonintrusive fail hint, native VFX | No presentation signal alters authoritative collision, event log or state hash; A/B audio/VFX off matches gameplay |
| CHDIR-F | Full test fixtures and recordings: CHDIR-01..12, progress persistence, postchase restart and actual S5 goblin recovery | `playtrace verify` if integration supports it; capture at least three at-speed playthroughs incl. first failure, co-op safe near miss and complete exit; owner game-feel signoff remains **pending** |

## Technical and art guardrails

- **Only modify the existing chase/level adapters and reusable gameplay components necessary**. A new shared chase phase enum and a small domain type are acceptable; building a general vehicle AI, new generic cutscene system, third-party movie-style overlay, or new movement verb are not.
- No HP or damage frames for the opening Dozer. Standard attacks may spark against hull but cannot defeat or stun the machine. Carl cannot leap over its wheel with existing movement physics. This is a **survival-only** moment; the future Dozer revenge is explicitly separate.
- **Never substitute a noninteractive scripted path for actual player inputs.** Trigger event timing can be authored and deterministic; player motion, safe refuge, hitboxes, wheel contact and barrier break still happen through fixed-step domain state.
- The full-height barricade variant is authoritative; the deprecated short background variant is not a lane blocker. The real blocker must vanish on the same tick of Dozer contact, never because the cutscene reaches its predetermined second.
- End-on wedge through S5 arch (at most 192px-wide visible pose) is a production art constraint. If missing, use ENV-02's declared wall-breach contingency *only after owner review* rather than distort approved art.
- A companion's autonomous pathfinding is independently owned. The chase may expose requirements (formation, regroup, guard timing), not copy/replace AI. In solo, manual character swapping must not accidentally kill the unselected companion on every run; 2P may fairly fail for bad decisions but must never be mathematically doomed by ordinary screen separation.
- No invented first-floor unlocks, special input, dynamite, mana, new enemy or viewers/sponsor elements. No conventional cinematic slow motion: exceptional kill rewards remain separate from surviving this chase.
- Every time/speed/distance exception must be data-driven and tested at 60Hz. VFX/raster/lighting/audio never alter sim outcomes. Respect Playtrace's normal live recording defaults and native art/provenance/rights constraints.

## Exact acceptance questions

1. **How many seconds does the original S1 spawn→S5 escape actually take** for novice/practiced solos and two real humans? Compare vs 25–40s hypothesis; do not falsify duration by inserting waits.
2. Can new players identify **danger from the wheel** and **the correct safe alcove** during their first attempt, without a long tutorial?
3. Does the charge **commit in a direction** rather than track a player and kill them in the alcove?
4. For two humans separated at 0, 96, 224, 448px at x1888, does each have a geometrically possible path to refuge before the charge intersects their positions? If not, fix policy/timing or call out owner approval required.
5. Do the two timber-splinter events, 104px low refuge, 96px full barricade and wedge all render at the approved 1x scale in the actual scene and respect collision?
6. Does failing restart at x1008 without replaying S1; does winning reach x2640 and continue directly into the I-03 goblin encounter?
7. Do identical captured fixed-tick inputs reproduce the same exact movement, actor ownership, damage, reset, prop states and event order even with audio/lighting disabled?
8. Does first goblin melee after the near miss feel like a satisfying emotional reversal? Ask for recorded human verdict.

**Do not claim owner approval** from an automated success metric. Keep the final PR checklist, actual commands/tests, native screenshots, source provenance, perf data, 1P/2P evidence and a short honest "not yet working" section.

**Minimum good PR outcome:** one playable S1–S5 chase with reachable refuge, fully deterministic barrier and checkpoint, correct DOZ-01 appearance and a short verified Playtrace. Polished audio, additional end-on art and final 2P feel may remain explicit follow-ups if they require separate asset/review ownership; **do not** mark the full feature done until fairness blockers are resolved.
