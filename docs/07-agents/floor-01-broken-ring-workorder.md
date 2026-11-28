# FLOOR01-RINGS-01 — Local agent handoff: The Broken Ring

**Scope:** implement a focused late-Floor-01 **side-view combat encounter** with enormous concentric-ring architecture, a short route fork, one optional shutter interaction and a safe glimpse of the Meadow Lark camp. This is a **design-only PR at creation**; no playable assets or code are implied. Read [the full encounter specification](../01-design/floor-01-broken-ring-labyrinth.md).

## Non-negotiable outcome

The sequence should feel like progressing through a huge, strange place rather than solving a literal circular maze. The player moves primarily right, has a real enjoyable direct fight, can optionally exploit one slowly moving **physical** gate and emerges into a small revelation of **human survivors**. There is no long wrong-path penalty, mandatory ring switch puzzle, new movement verb, magic gate, equipment reward or platforming-first detour.

## Current dependencies / ownership

- Review `AGENTS.md`, production backlog, active PR ownership and `docs/07-agents/f01-playtest-integration-plan.md` (#182, integrated schedule), and the actual current Floor 01 level material/scene schema.
- **BOIL/#176:** currently owns shared authored prop/hazard/impact core. **Reuse that interface** for shutter if and when it exists; do not implement a second generic moving-gate engine or touch BOIL's implementation branch as a parallel agent. If unavailable, ship a safe **static-gate combat greybox**, honestly mark RING-C blocked.
- **#185 / HAZ:** slime/Rot enemy behavior in flight; R2 uses only already implemented, reviewed enemies; don't invent a new labyrinth-specific monster or unapproved slime splitting.
- **#187 / REST:** Meadow Lark *camp interior* belongs to that proposal. This work stops at the **camp approach/reveal and protected handoff**. Coordinate geometry marker and transition rather than duplicate environment/NPC production.
- **DONUT-PROG:** Light on Your Feet exists as a **story grant before this area** in the approved design; its actual implementation/128px reach is a dependency. Without it, optional shelf can be omitted from the first greybox or tested via a clearly labelled fixture loadout, never automatically granted here.
- **CARL-PROG:** follow the final story-ledger merge. The currently selected integration resolution grants dynamite after Hoarder; this area is later. **Do not grant it here**.
- The Juicer boss, Puddle Jumper and the Ball of Swine moving-ring arena remain **later content**, not part of this passage.

## Sequential work slices

| Slice | Concrete deliverable | Actual evidence |
| --- | --- | --- |
| **RING-A** | Level scene with main floor, ring landmark, safe 64px bypass, optional +128px shelf and distinct entrance/exit landmarks. Exact pixels chosen from actual traversal physics. | Screenshots at 1× native virtual resolution; a path/reachability fixture for solo both hero owners and P1+P2 |
| **RING-B** | Two staggered ordinary goblins, clear direct-win path, one **static** stone shutter variant that changes real collision/LoS but never blocks mandatory route | Recorded direct clear and optional-height route; no hidden mandatory interact |
| **RING-C** | Slow shutter with distinct warning, committed bounded close/open, **actual** swept body collision and one pin/stagger event from a launched enemy | Fixed-step event/impact/hash tests; safe heroic contact and no repeated contact damage; retry mid-movement |
| **RING-D** | Approved brick/stone ring visual kit, unique entry crown / carved shield / survivor-canvas marks, audio restraint, real co-op camera shots | Native 640×360/400 and 1080p captures, no HUD overlap or misleading foreground art |
| **RING-E** | Protected transition revealing camp structures and passing to REST-B when its scene is ready | Gameplay session reaches next checkpoint, no projectile spill, story flags unchanged, replay reproducible |

**Implementation in independent small PRs/commits** respecting the orchestrator's hot-file queue. A successful greybox under RING-A/B is a useful deliverable, but never claim the dynamic shutter or late-floor route is integrated before it actually is.

## The signature scene: rules to preserve

- The shutter is **slow and readable**, distinct from the dramatic rapid Ball of Swine boss gates. Initial design hypotheses: warning 36–48 ticks, movement 24–36 ticks, dwell 90–150 ticks only if testing justifies cycling. Keep data-driven.
- Moving shutter may block a shortcut or one combat projectile lane; **both Carl and Donut can always follow a standard ground/64px bypass**. An optional +128px ledge demonstrates Donut's earned leap; her straight horizontal Magic Missile is not auto-aimed.
- At raised paths respect **DONUT-PROG DP-Q45**'s 228px lane ceiling and validate the boosted 128px ledge with actual physics and HUD, not sprite assumptions.
- Gate cannot instantly crush the player. If moving contact cannot be resolved through existing authoritative physics without trapping/teleporting players, **defer moving collision**, use static variants and label the remaining work.
- An enemy legitimately kicked/knocked into the moving gate can be **pinned/staggered once**, with true source attribution, and then punished with normal attacks; it is not a scripted instant death. Direct combat (never using gate) is equally legitimate.
- Enemy AI may not fire, spawn or land hazards offscreen because the player approaches from an optional upper line or P2 lags. No AI companion navigation implementation in this work.
- The player never presses a new switch input, walks through a fake background ring, loops around a duplicated room or teleports between floors. No new camera topology.
- Camp survivors: use already approved **concept identities** for Imani, Elle McGibbons, Brandon and Chris Andrews, but **not** concept PNGs as ready sprites. Odette isn't in this scene. No original dialogue authored in this implementation until narrative/story review.

## Automated self-test contract for agent

At minimum implement and **execute** (do not just declare) the RING-01..14 acceptance checks from the main spec. The key **anti-tedium** test sends honest legal rightward movement, jumps and ordinary attacks through the actual level, without ever activating the shutter as a puzzle switch. It must reach the protected end and produce travel/retry/forced-wait metrics. Another fixture launches one goblin into the shutter and verifies the actual collision event, not a coordinate-scripted kill.

Use Rust real sim/party fixtures and Playtrace according to their merged capabilities; no Python toy physics. If current AUTOPLAY doesn't support the new level, add small deterministic in-engine fixtures rather than misleadingly invoking unsupported party evaluate commands. Report **pass / fail / blocked / not-run**, log concrete commands, compare state hashes and actual scene screenshots. Capture at-speed **two-pad** run and one Donut active solo session. Human playtest determines whether this is memorable and unconfusing.

## Completion report required

- Actual scene entry and current branch/commit; authored 2D walkable topology and protected route proof
- Direct and optional environmental combat trace + true source/damage attribution
- Validated +64/+128 motion and same-height Magic Missile ranges
- Camera preview at two vertical bands and survivor reveal
- 1P Carl, 1P Donut, real 2P, checkpoint and retry evidence
- Native visual source provenance / approved art refs
- Exact RING-01..14 test status, blocked shared APIs, next focused implementation step
- Explicit **pending owner at-speed game-feel review** until the owner actually plays

This handoff does not alter accepted character abilities or author a full maze or camp. All novel-specific prose and rights-uncertain scene details are kept separate from original gameplay adaptations.
