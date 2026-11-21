# F01-REST-01 — Agent handoff: memorable refuge spaces, design-to-greybox

This is a **design/review handoff**, not a mandate to implement all three rooms concurrently. Main proposal: [Floor 01 Quiet Setpieces and Transitions](../01-design/floor-01-quiet-setpieces-and-transitions.md). **No new ability or camera mechanic is authorized by this PR.**

## Implementation order and ownership

1. **REST-A / Fast food FIRST**, after #181's completed M3 exit and the actual `f01_safe_room_reward_found` / `jump_reach_boost` contract owned by DONUT-PROG-01. Build a connected no-combat greybox with reward placement, real checkpoint, 128px Donut jump validation and explicit Carl/2P regroup. Do not retime #181's llama attacks or change Donut mobility; use the current shared scene loader and hot-file queue from #182.
2. **Art kit, separate from physics:** use approved blue-white 1990s restaurant concept as mood reference, not production sprites. Source/editable pixel kit includes tile, counter, booths, plastic tunnel/ballpit, windows showing solid dungeon stone, muted light. Single non-collidable rolling ball gag is optional and needs review. No franchise branding or unauthorized music.
3. **REST-B / Meadow Lark NEXT**, *after* labyrinth geometry stabilizes. One quiet practical human camp; no combat, escort, trading, story unlock or prolonged dialogue. Until specific survivor sprite approvals, silhouettes may be **plain labelled greybox markers**, not generated final characters. Camera and checkpoint must be real.
4. **REST-C / Juicer aftermath LAST**, after the boss + ledger grant and Ball of Swine scene integration exist. Test optional gauntlet strength and collision-safe 112px Donut blink, no duplicate grants, same main path. No fresh powers.
5. **Integration QA**, not parallel engine rewrite: author 1P Carl active, 1P Donut active and actual 2P fixtures; verify protected route, checkpoint retry, story flag idempotence and replay. Do real Bevy visual review at 640×360/400 and 1080p.

## Critical cross-team and design gates

- **PR #181** owns the Infernal Corral and fast-food threshold. Do not edit its encounter specification or infer that M3 is already implemented.
- **DONUT-PROG-01 #175** owns the Light on Your Feet ability and reward trigger. Use the existing story ledger when merged, not a second system. **Option A1** is the short 128px jump + existing Interact R1 to open Carl's ground passage. If this feels bad, **Option A2** (optional ledge) needs explicit owner/progression approval because the progression document currently describes an exit gate.
- **PR #182** owns integrated playtest scene routing, dependencies and shared-file queue. Follow it, noting its K1 dynamite-after-Hoarder adaptation instead of silently reintroducing an early explosive grant.
- **Juicer and Ball of Swine** have existing art and independent scene mechanics. Do not pull the boss fight or cinematic camera into this resting-space work.
- **Playtrace** should capture live dev sessions as its current contract specifies; test events/flags and reproducible replay. If any stage transition is unsupported in the merged recorder, report as a blocker; don't invent replay verification.
- No new item economy, inventory, gear shop, paid buffs, saved checkpoint menu, dialogue system, NPC AI framework, player movement verb or full-floor rebuild.
- If an implementation is blocked by an upstream merged contract, hand off a **playable greybox subset and precise block report**, not a false green status.

## Acceptance / evidence requirements

Follow REST-A-01..REST-ALL-02 in the main doc. At minimum include actual test commands and outputs, a collision/path graph or deterministic reachability fixture, native at-speed footage for **hot catacomb → absurd restaurant**, before/after Donut jump proof, clear reward grant on true box reach and its persistence on restart, no enemy stray projectile, correct solo/2P state. Report whether humans enjoyed the 128px jump at the exit; **machine tests cannot decide the pacing of a pause**.

This workorder is not a request to run agents in this chat or background. It is a document for local agents once scoped work is assigned. All new copy/assets must be original and comply with project rights/provenance rules.
