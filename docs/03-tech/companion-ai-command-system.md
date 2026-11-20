# THE CRAWL — Companion Autonomy and Command System

**Status:** Design proposal v0.2 — for owner/gameplay playtest review; **not approved or implemented**.  
**Date:** 2026-11-20  
**Scope:** Floor 01 solo (either Carl or Donut controlled by AI) with compatibility requirements for 2P, existing party/hud rules, later narrative progression and offline policy evaluation.  
**Primary goal:** Make the AI companion useful, intelligible, characterful and tactically interesting **without turning a combat-first platform-action game into a command-management game**.

> This document proposes gameplay rules, control experiments, architecture and test gates. Numeric values are *starting hypotheses*, never tuned/approved facts. It does not modify the current implementation or supersede owner-approved controller/party/HUD specifications.

## 0. Evidence and binding constraints

Reviewed on repository `pando85/dungeon-floorbreak` (main, 2026-11-20; integration baseline `e9bea792e695207adf5d32352b80e40d035add61`):

- `docs/01-design/mvp-controller-contract.md`: Cross/A jump; Square/X primary; Triangle/Y physical; Circle/B special; R1/RB interact; L1/LB swap in solo; L2 and R2 reserved for future actions. **Held L1 for a companion command wheel is reserved, not approved**; no mandatory right stick or D-pad commands, as D-pad is also movement.
- `docs/01-design/party-2p-open-questions.md` and `src/party.rs`: solo swap (20-tick cooldown, 6-tick buffer, blocked while controlled hero hurt), follower bot, 1 HP floor only while AI-driven, bias against AI targeting, offscreen protection; 2P P1 Carl/P2 Donut, disconnect pauses with no automatic takeover.
- `src/follower.rs`: deterministic movement/jumps/basic tell avoidance; **never attacks** and currently lacks a navigation planner.
- `src/input.rs`: logical `RawInput`, tick/pressed-edge semantics; `swap_pressed` is currently generated on **press**, not release. This matters critically if a long press is added.
- `docs/01-design/asymmetric-traversal-and-gates.md`: a mandatory obstacle may explicitly require Carl or Donut; solo solvability through switching is intended. AI must not bypass character-specific capability gates or softlock the game.
- `docs/02-art/hybrid-hud-a-native-spec.md`: native pixel HUD in Bevy with dedicated HUD Camera2d and Text2d, 640×360 / 640×400, hero plates at `(8,8,164,48)` and `(468,8,164,48)`, top boss/location strips, action lane y≥60. AI plate does not show human ability chips. Avoid adding permanent HUD clutter or `bevy_ui` implicitly.
- `docs/01-design/donut-mana-v0-proposal.md`, `docs/02-art/donut-moveset-q10.md`: swat, pounce, post-unlock Magic Missile; mana max 8, cost 1, **existing AI reserve is 2 charges remaining after casting**. A new command system must respect the reserved floor, narrative unlocks and existing move constraints.
- `docs/01-design/gameplay.md`: combat-first, readable knockback, environmental combinations; same core game in solo and 2P; controller simplicity; correct event attribution for individual audience/viewership features.

**Concurrent integration warning:** AUTOPLAY-01 implementation has started in [PR #127](https://github.com/pando85/dungeon-floorbreak/pull/127) and is Carl-only at its v1 boundary; do not depend on its unmerged APIs or broaden it to two-hero policies. [COMBAT-FUN-02 PR #124](https://github.com/pando85/dungeon-floorbreak/pull/124) remains separate. Rebase/coordinate with both before editing shared combat/input files. The authoritative local-agent execution breakdown is in [the companion work order](../07-agents/companion-ai-01-workorder.md).

### External research (primary sources first)

| Reference | Evidence | Transfer to THE CRAWL | Non-transfer |
|---|---|---|---|
| God of War / Ragnarök | Santa Monica describes companion AI as an extension of combat and evaluates player danger, enemy positions and kill stealing; the playable and companion versions of Atreus share the same skill tree. [Studio Q&A](https://sms.playstation.com/stories/crafting-companion-behaviors/media/1031) / [PlayStation design interview](https://www.playstation.com/en-sg/editorial/god-of-war-ragnarok-evolution-of-atreus/) / [Atreus GDC](https://www.gdcvault.com/play/1025768/Raising-Atreus-for-Battle-in) | Proactive threat support, explicit player authority, same character capability in both control modes | Cinematic choreography / high production complexity |
| God of War Ragnarök traversal | [GDC 2023 Companion Traversal](https://www.gdcvault.com/play/1029003/Companion-Traversal-in-God-of) describes traversal, navmesh and designers' affordances | Capability-aware navigation on a 2D platform graph | 3D navmesh unchanged |
| Dragon's Dogma 2 | [Official manual](https://game.capcom.com/manual/dd2/en/ps5/page/4/7): Go / To me / Wait / Help, with contextual suggestions | Small vocabulary of intents; contextual meaning | Four simultaneous autonomous pawns and broad exploration |
| Final Fantasy VII Rebirth | [Official battle design](https://www.square-enix.com/ffvii/en-gb/games/rebirth/battle/): autonomous baseline, commands, synergistic skills and time-slowing tactical mode | Independent characters plus explicit coordinated actions | Menus, ATB complexity and frequent slow time |
| Accessibility of gamepad controls | [Android controller practice](https://developer.android.com/games/sdk/game-controller/recommended-practices): remapping, hold/toggle alternatives, secondary button sets | No compulsory right stick; accessibility alternative to holds; visible focus | Android platform-specific API choices |

**Research synthesis:** companions should help without being maintained; commands should express *intent*, not micromanage an animation; high-stakes actions require authority; navigation needs to understand reachable platforms, not simply follow world coordinates.

## 1. Design pillars and player contract

1. **The human chooses an avatar, not a passive follower.** AI Carl is as developed as AI Donut; both use the same decision system with different capability/policy profiles.
2. **Default state is effective autonomy.** The player can complete standard encounters without ever opening an order menu. Commands should create tactical advantages or intentional coordination, not repair broken follower navigation.
3. **Predictable commands, flexible execution.** `Assist` means help with the current threat/opportunity; AI chooses a valid available move and shows what it intends to do.
4. **Safety over obedience; never lie.** An order can be deferred, refused or interrupted, with a legible reason. It must not result in unexplained self-destruction, softlocks or an accidental ability unlock.
5. **Human gets meaningful action ownership.** Normal AI kills are allowed, but AI should not systematically steal an enemy the player is actively setting up for a spectacular finisher; only actual instigators receive kill/viewer credit.
6. **Same simulation, same rules.** Human and AI actors use the same abilities, physics, collision, resources, cooldowns and attack startup/recovery. No companion-only teleport, damage or free mana except the existing explicit solo safety safeguards.
7. **Game story first.** Character capability and authority are story-phase aware. No early magic, premature explosives, nonphysical cat interactions, invented grabs or mandatory shoulder-mode solutions.
8. **Presentation does not drive combat.** HUD, animation and voice responses consume state/events; they cannot change authoritative outcomes.

**Success feeling:** “They understand what I'm doing, often help before I ask, and when I give a command I can predict the result.”  
**Failure feeling:** “I must babysit them”, “they killed my setup”, “the command button does the wrong thing”, “they got stuck and the level can't progress”.

## 2. Decision model: autonomy versus commands

AI proposes behavior continuously even without an order. Split arbitration into hard constraints and soft priority selection:

**Hard constraints (always first):** mode/authority; capability & story unlock; physical reachability and animation commitments; prevent known softlocks; legal ability resource check; collision/damage rules; guard critical interactables; avoid suicidal paths / untelegraphed hazards. A command cannot override these.

**Immediate interrupts:** avoid lethal/urgent telegraphed damage if physically possible; protect leader from imminent unavoidable hit if a safe legal action exists. Interrupt only actions with cancel windows or explicit defensive overrides; preserve existing hit/recovery rules.

**Soft priority evaluation:** valid player order > immediate protection/support > opportunistic combo assist > routine fight > traversal follow > idle expression. A `Hold` command is not stronger than a critical camera tether or player rescue.

**Implementation recommendation:** deterministic hierarchical finite-state controller for top-level states (`Follow`, `CombatSupport`, `ExecutingOrder`, `Regroup`, `Recover`, `Unreachable`) plus a small, data-driven *utility scoring* function for choosing legal moves and targets. Do not create a general-purpose behavior-tree framework before profiling a real need. A future ML policy may replace the **proposal/scoring** function but never the safety/authority validator.

## 3. Command vocabulary: minimal and symmetrical

Five semantic verbs; **only four fixed face-button slots** in the proposed L1 layer. `Interact` is a context-dependent interpretation of `Assist/Act`, not a sixth button.

| Verb (logical ID) | Scope / default lifetime | Carl AI | Donut AI | Invalid/defer behavior |
|---|---|---|---|---|
| `Assist` / `Act` | Single action, then resume prior safe default | Kick/jab an immediate threat, protect escape route or perform validated physical context action | Swat a tell, pounce a legal opening, cast when unlocked and reserve allows, or perform validated small-object interaction | If no safe eligible action: `Unavailable(no_opportunity)`; **never select an unrelated target** |
| `Focus` | One enemy stable ID until dead, inaccessible, cancelled, or timeout | Pursue within bounded tether; close-range pressure / stagger | Flank/perch/attack from safe reach; never walk into enemy for a ranged shot | Enemy not reachable: show `Cannot reach`, stay in defend/follow; do not silently switch target |
| `Hold` | Anchor at **companion's current safe position**; until cancelled or camera-tether expiry | Guard nearby approach / avoid unnecessary chase | Stay at safe vantage or maintain defensive zone | Unsafe anchor: cannot accept; automatically break hold for urgent safety and camera tether |
| `Regroup` | Until both within follow band | Return safely, defend retreat if necessary | Return on navigable safe path, avoid attacks | Path missing: keep current safe reachable zone, notify; request swap at asymmetric gate |
| `ContextAct` (via `Assist/Act`) | One authored capability-checked interaction | Open/break Carl-reachable object only if allowed/authorized | Reach small/perch-accessible target and interact physically if allowed/authorized | No eligible target or missing capability: refusal, never fake success |

**Note on `Hold`:** no arbitrary point-and-click placement or second-stick cursor in v0. It anchors at the AI's *existing* location; positional direction/pinning is a separate experiment, not implicit in the control contract.

### Command completion / expiration policy

- `Assist`: completes after one actual action or times out if action never becomes legal. Never repeatedly spends resources in a background loop from one press.
- `Focus`: resolves on enemy death/retreat, inaccessible world state, room transition or explicit cancel. Candidate max active duration **8s** if nothing progresses; combat engagement may refresh only with measurable progress; tune.
- `Hold`: persistent only while safe and within the solo camera tether, with **8s initial timeout** in normal rooms; authored short puzzle hold may survive longer with explicit level metadata. Never prevent story progress or regress safe regroup.
- `Regroup`: completes when the follower is in its existing stable follow band; returns to default `Follow`.
- `ContextAct`: completes only when the actual world interaction emits its success event. Cannot register success when the AI merely reaches the prop.
- A new compatible order replaces the old one. One **persistent directive** and one **ephemeral immediate task** at most; no hidden queues. `Regroup` clears persistent target/hold.
- On swap, the newly controlled hero's AI order is cancelled; the other hero enters autonomous `Follow/CombatSupport`. Preserve a human-started attack's legal motion, recovery and hitboxes; do not convert queued human presses into autonomous follow-up attacks.
- In 2P, no AI order recipient. Input ownership stays P1 Carl/P2 Donut. Future optional co-op pings are *visual requests* and require a distinct human-to-human subsystem.

## 4. Command interpretation matrix and examples

### Carl controlled, Donut AI

**Goblin tell:** Donut sees a tell reaching Carl; choose a safe swat if in range, possibly after closing a short distance. Swat remains 0 damage per Q10 and obeys the light-guard/cooldown; don't create infinite interruption loops.

**Carl combo:** Donut can extend a launch or interrupt the next attacker. During a player-finisher opportunity, prefer protect/extend instead of stealing the same target unless `Focus` was explicitly commanded.

**After biscuit:** Magic Missile only if `mana >= missile_cost + ai_mana_reserve` (existing design: 1+2 = 3 charges available **before** release), suitable target/line-of-fire and a compelling tactical advantage. No invented free casts. Avoid sniping every weakened goblin Carl is winding up to finish.

### Donut controlled, Carl AI

**Goblin rush:** Carl screens the approaching goblin, interrupts with a legal jab/kick or maintains the route while Donut kites. He must not use a grab (canon contract: no grabs).

**Donut airborne/perching:** Carl follows paths *he* can traverse, not Donut's jumping trajectory. If a mandatory Donut-only perch or Carl-only breach separates them, he waits safely until the human switches to operate the gate. Do not bypass the gate via teleport.

**Carl tools unlocked:** Disruptive/environment-changing explosives are not automatically used. In v0, explicit direct control is required for dangerous explosive placement/detonation. A later `ContextAct` can operate one only after authored safety predicates, a clear target cue and distinct player confirmation, to be separately designed.

### Both directions

**No orders:** both characters should independently acquire useful positions, avoid tells, and perform limited support actions. Routine AI kills are legitimate and credited to the real actor, but cannot dominate play.

**Character personality:** personality changes initiative/tactical preferences, barks and idles—not the semantics or reliability of `Regroup` and `Focus`. Donut may make a grandiose comment; Carl may respond dryly; neither may refuse a valid order for a joke. Avoid incessant voice feedback; text-only subtitles toggle required if audio barks are used.

## 5. Input UX: decision experiment (do not silently rebind)

Current canonical mapping: L1 tap = swap; R1 interact; D-pad or stick movement; face buttons for jump/punch/physical/special. Current code fires swap on *press* (`InputFrame::swap_pressed`). The approved design explicitly **reserves held L1 for later commands**; activation timing remains unapproved.

### Prototype A (primary candidate): L1 tap swap / L1 held command modifier

| Input | In solo | 2P |
|---|---|---|
| Short L1 press **and release** without any face chord | Swap active Carl/Donut | No character swap (optional future ping only) |
| Hold L1 ≥ **10 simulation ticks** (~167 ms; hypothesis) | Show 4-command legend and preview valid targets | No IA commands |
| L1 + Square/X | `Focus` candidate enemy | Unused |
| L1 + Triangle/Y | `Hold` at AI's current safe location | Unused |
| L1 + Circle/B | `Regroup` | Unused |
| L1 + Cross/A | Context `Assist` or safe `ContextAct` (clear exact label before commit) | Unused |
| Short/long L1 release after any chord | **Never also swap** | Unused |
| R1/RB | Normal interact (unchanged) | Normal interact |

**Fast chord:** the first face-button edge while L1 is held **immediately enters command mode**, even before the hold threshold. Do not wait 10 ticks before accepting L1+Square. This avoids making fast two-button commands feel laggy.

**Key input contract:** swap is provisionally detected on **release**, not on press, after checking no chord or hold mode. This is a deliberate migration of the currently adopted Q8 press behavior. Existing cooldown (20 ticks), buffer (6 ticks) and hurt block remain; their precise handling must be revised and regression-tested. Do not silently switch on press and then switch back if L1 becomes a hold—that visibly breaks authority/camera/ongoing attacks.

**Mode state machine:** `Idle → L1Candidate → [CommandMode | TapSwapReady] → Idle`. Enter `CommandMode` at threshold or a face chord; first face edge is consumed exclusively as a command; releasing unused command mode cancels with no swap; no face attack, jump or special leaks from command mode. `TapSwapReady` on L1 release emits exactly one `SwapRequest`. Pause, controller disconnect, dead/respawning state and transition to 2P clear modal inputs and pending commands.

**Crucial UX risk:** tap-to-swap on release adds perceptible latency compared to existing press-to-swap. Measure real DualSense and Steam Deck; specifically playtest swap-as-a-combat-move. If this violates accepted combat feel, reject A rather than disguising the regression.

### Alternative B: R1 hold command modifier, L1 instant swap

Pros: preserves zero-delay L1 swap. Cons: interferes with existing/required R1 interact, delayed interact, inadvertent pickup/interaction after command cancel, especially with traversal gates. A hold/tap split introduces the same ambiguity and requires input migration. **Not recommended unless A fails timing and B passes interactions.**

### Alternative C: explicit button chord using an extra modifier / remap

Potential accessibility control scheme, not default. Any mandatory right-stick/R3, touchpad, gyro, paddles or D-pad order navigation **violates existing controller requirements**. A remappable accessible toggle for command mode can be offered without changing canonical defaults. If A and B both prove poor, escalate the control conflict for owner review; never silently consume R2 (reserved cycle special).

### Accessibility and discoverability

- Provide hold-to-open **and toggle-to-open** command mode as remappable alternatives, both deterministic.
- Teach one contextual assist action in the first available encounter; add Focus/Hold/Regroup as a short optional tutorial. Do not dump a four-command HUD on spawn.
- No compulsory slowdown while selecting orders; combat remains at normal speed by default. If slow time is tested for solo accessibility, it must be an explicit, deterministic gameplay-time policy, not presentation-only time scaling, and **never** slow a second human's game.
- Show actual device glyphs and an always-cancellable focus state. No state depends on color alone.
- No double-tap or combo requiring three simultaneous inputs.

## 6. Targeting and the `Assist`/`ContextAct` disambiguation problem

**Selection should be deterministic, explicit enough to trust and usable with one stick.** Input is not a world-space mouse coordinate.

1. At command entry, compute valid visible/nearby candidate targets (enemies and authored companion interactions) based on camera visible bounds, facing, meaningful range, line of sight where relevant, ability compatibility, navgraph reachability and safety.
2. Sort with stable, documented scoring: on-screen and facing-aligned > currently engaged player target > near target > obstructed/far; stable target ID as final tie-breaker. Record resolved stable entity ID in `CommandIntent` when activated.
3. Highlight exactly **one previewed target** when opening/issuing `Focus` or `ContextAct`. If target selection is ambiguous or the front target changes due to movement, show a clear preview before accepting; do not silently retarget an issued order.
4. `Focus` requires an enemy; if none is unambiguously eligible, fail with `No target`. A target killed after issuance completes the order instead of selecting another enemy.
5. `Assist` operates **without selecting an enemy** when there is an imminent threat to the human or a well-defined combat opportunity. If an explicitly highlighted authored companion interaction exists and is legal, show the label `ACT: <short-object-name>` instead of `ASSIST` **before** commitment. If both compete and neither was explicitly pointed out, prefer combat support; authored progression interactions should be discoverable via human R1 and swapping.
6. `ContextAct` must be supported only for authored object capabilities (e.g. `requires=small_cat_access` or `requires=demolitionist_physical_break`) and never be a blanket 'auto-solve puzzle'. No guarantee an AI can perform actions that fundamentally require a human swap.
7. For inaccessible targets, do not choose a convenient different target. Return `unreachable` with a small indicator and remain safely autonomous.

**V0 target selection intentionally does not require a movable reticle.** If human playtests show excessive incorrect targets in crowded fights, next iteration should try a lightweight target-cycle shortcut *without* stealing the D-pad movement or demanding a second stick. No cursor/aim wheel until data demonstrates the need.

## 7. Companion navigation and spatial intelligence

**Existing follower is not enough.** Build a compact **2D platform traversal graph** based on the current level's collision and authored navigation hints:

- Nodes: safe walkable platform intervals, landing/perch zones, regroup points, interaction anchors, authored ledges.
- Edges: walk, jump, drop, temporary/destroyable passage and *authored* one-time transitions.
- Each edge has capability predicates (Carl/Donut reachability, ability unlocks), danger/risk and animation/movement constraints. No generic leap that ignores actual physics.
- Physical feasibility uses the existing deterministic movement/hitbox rules, not artist sprite canvas size. Carl must not follow Donut up a cat-only route; Donut must not spontaneously break a Carl-only wall.
- Rebuild only the affected graph edges after destructible terrain changes, not the whole graph every tick. Cache reachable safe anchors for repeated decisions.
- Combat positioning candidates include protective guard point, flank point, ranged sightline and escape anchor; score against proximity to camera and enemy tells.
- On no valid route: `WaitSafely` + clear feedback + authored safe regroup protocol. Generic teleporting across story-critical gates prohibited. Existing off-screen regroup after 120 ticks needs review against this graph; restrict to safe equivalent regions that cannot bypass mandatory gates, with debug trace and camera warning.
- Separate emergency positional corrections from real traversal. Avoid a pop-in inside the camera, during combat or just to satisfy a `Hold` directive.
- Every mandatory gate must have a switch-and-solve fallback in solo; never depend on an AI successfully operating it with frame-perfect timing.

## 8. Combat support policy and anti-kill-steal

AI chooses only valid moves already implemented/unlocked; respects per-move recovery, interrupt guard, attack token state, projectile line-of-sight, enemy stagger, physics and hero hurtboxes.

**Threat awareness:** enemy tell/impact ETA, victim, active hitbox, avoidance window, player HP (including 1-HP risk), risk of being surrounded, active player attack commitment, enemy HP/launch state and proximity to environmental hazards. Urgent protective acts can beat soft command priorities, if physically legal.

**Positioning roles:** Carl screens close threats and creates knockback lines; Donut seeks safe flank/height and interrupt/casting lanes. Both can contribute damage. Differences live in character profiles, not separate order grammars.

**Anti-steal heuristic:** mark a short `HumanFinishWindow` candidate when the human has initiated an attack/combo that is likely to defeat a specific target, especially a launched enemy about to hit an environmental object. Unless `Focus` was explicitly issued, prefer interrupting the other enemy, holding space, protecting the player or contributing a non-lethal assist to that target. This is *not* an invulnerability grant to enemies, and does not suppress legal collisions or a kill already committed by the AI. Measure whether the heuristic is needed before adding hidden exclusions.

**Fairness:** AI can score spectacular kills if its own legal actions produce them; rare slow-motion remains skill-gated and co-op-compatible per the game's death spectacle spec. Do not award automatic 'spectacular' status merely because the AI acted. Attribute kill/viewers to actual instigator, and record assist/launch/environment sequence separately so human and AI points remain explainable. Do not implement arbitrary player-favoring kill-credit rewriting.

**Resource limits:** Donut's AI preserves the existing `ai_mana_reserve=2` **remaining after cast**; Carl's dangerous tools are manual until explicitly designed. A player swap does not replenish or reset resource/cooldown state. AI must not exploit the existing AI 1-HP guard to tank damage recklessly; the guard is a last-resort fairness mechanism, not a strategy.

## 9. HUD, sound, personality and feedback

**Always-on:** keep existing Carl/Donut plates and `AI` ownership tag. No permanent extra command panel. The AI plate can expose *one compact status icon* (Follow / Focus / Hold / Regroup) as a future native HUD extension, with an alternative minimal semantic label if legible. Do not remove the existing L1 Swap affordance or introduce fake ability chips for AI.

**Command mode (temporary):** a compact four-choice overlay near a safe portion of the viewport, **not over the boss strip, hero health plates, the current telegraph or the action lane whenever possible**. Screen-space sprites/Text2d on the existing HUD layer. Highlight valid candidates in world space, with a small clear targeting marker, not a large radial over the fight. At 640×360 and 640×400, obey 8px safe margin, nearest-neighbor rendering and original glyph artwork policy. Position/occlusion and exact pixels require design-mock and in-engine captures **using existing approved art**, never inferred from stock imagery.

**Order acknowledgement:** `Accepted` visual immediately on valid command (prefer ≤6 game ticks as initial target); `Executing`, `Deferred`, `Unreachable`, `Done`, `Cancelled` get distinct shapes and brief labels when important. A refusal must explain the actual constraint (`No path`, `Not unlocked`, `No target`, `Unsafe`, `Out of mana`) without noisy repeating barks.

**Audio/personality:** distinct Carl/Donut responses, compact subtitles, cooldown on repeated acknowledgement. Character expression never changes whether the intent was actually accepted. Subtitles and non-audio indicators preserve accessibility. Visual emphasis must remain below the priority of combat telegraphs.

## 10. Domain model / deterministic integration contract

Illustrative Rust-shaped types (names are provisional; adapt to current code):

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompanionVerb { Assist, Focus, Hold, Regroup, ContextAct }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderStatus {
    Preview, Accepted, Deferred, Executing, Completed, Rejected, Cancelled,
}

pub struct CommandIntent {
    pub issued_tick: u64,
    pub sequence: u32,
    pub issuer: HeroId,        // human-controlled hero at issuance
    pub recipient: HeroId,     // AI-controlled hero at issuance
    pub verb: CompanionVerb,
    pub target: Option<StableEntityId>,
    pub anchor: Option<NavAnchorId>,
}

pub struct CompanionDirector {
    pub owner: HeroId,
    pub mode: CompanionMode,
    pub persistent_order: Option<ResolvedOrder>,
    pub immediate_task: Option<ResolvedOrder>,
    pub last_reason: Option<OrderReason>,
    pub stable_target: Option<StableEntityId>,
    // cooldowns, safe anchor, commitment, nav version and policy tuning
}
```

**Suggested pipeline, fixed simulation tick:**

1. Physical device adapter → logical per-player buttons/edges; resolve L1 tap/hold/chord *before* mapping face presses to human attacks.
2. On single-player input: if swapping, call existing `PartyState` swap authority and clear old AI directive safely; if issuing, build `CommandIntent` for current AI recipient (never human).
3. Validate command against mode, story capabilities, target ID, camera/nav state and safety. Emit stateful acknowledgment/reason event.
4. Sense world snapshot; update safety/urgent avoid **each tick**; recompute utility preference every **6 ticks** (10 Hz starting hypothesis), immediately on threats/orders/terrain changes. Navigation planning at a lower conditional frequency; never allow stale nav after destruction.
5. AI action selection emits the same `RawInput`/domain actions as a human, through the same legal move transitions. Never synthesize a guaranteed hit outside the combat sim.
6. Resolve actual physics, damage, kills, interactables, attribution, ability resources; record outcomes. HUD/audio read committed events only.
7. Hash and replay all gameplay-authoritative state, including command sequence, order, timers, selected stable target ID, policy version, navgraph revisions and deterministic tie breaks.

**Separate modules:** command input grammar (UI-adapter/domain-neutral), director/authority, order validation, world sensing, candidate utility, navgraph/reachability, actor profile and presentation adapter. Keep render and human barks outside gameplay core. Integrate with existing `PartyDrive`, `PartyState`, `follower` and `CombatState` incrementally, not with a parallel combat engine.

**Important:** clear the command modal on Start/pause, controller disconnect/reconnect, stage transition, checkpoint restart, switch to 2P, or death; invalidate old target IDs after entity deletion. An AI may still finish an already committed move across a swap, as required by existing Q8 behavior.

**Config knobs** suggested in `assets/data/gameplay/companion_ai_v0.json`: hold threshold, command timeout, target candidate range, utility reaction tick cadence, AI attack cooldown, tether distance, safe-path threshold, bark cooldown, assist budget. Use existing pinned tuning conventions and replay hash. Do **not** duplicate existing canonical mana values in the new file.

## 11. Command event instrumentation

Recommended event IDs for record/replay/telemetry:

- `OrderPreviewed { tick, actor, verb, candidate }` (presentation-only; no gameplay hash if ephemeral)
- `OrderIssued { tick, sequence, issuer, recipient, verb, target }`
- `OrderAccepted/OrderRejected { tick, sequence, reason }`
- `OrderExecutionStarted/OrderDeferred/OrderCompleted/OrderCancelled { tick, sequence, reason }`
- `CompanionDecision { tick, actor, state, chosen_move, candidate, utility, safety_override }` (debug trace, sampling/capping as needed)
- `CompanionNavFailure { tick, actor, current_anchor, target_anchor, reason }`
- `AssistContribution { tick, actor, target, kind }` and existing authoritative defeat/launch/environment event chain

Metrics: orders per minute; command acceptance and failure rates; false-positive target selection; order-to-first-action latency; human-attack interference; human-finisher stealing; useful interrupts per fight; companion incoming hits; AI resource spending; no-order solo survival; mandatory-gate reachability; time stuck; forced regroup frequency; FPS CPU time of sense/nav/planning.

All gameplay metrics and event chain must work in **headless** fixed-tick fixtures with repeated seed/run hashes. Keep optional anonymized telemetry out of v0 runtime unless separately approved; local fixture metrics suffice.

## 12. Representative tests (headless first)

| ID | Scenario | Required assertion |
|---|---|---|
| C-01 | Carl human, Donut AI; 2 goblins and one tell | Donut can safely swat/protect, not cast pre-biscuit, no infinite stagger loop |
| C-02 | Donut human, Carl AI; pressure from front | Carl uses legal melee, shields opportunistically, no made-up grab/explosive |
| C-03 | L1 tap vs hold vs fast chord | Exactly one swap or one order, never both; no leaked Cross jump/Square attack; gamepad, keyboard, reconnect parity |
| C-04 | Swap in the middle of AI action or order | Previous AI order cancelled; legal started attack completes; other hero assumes autonomous role; camera transitions correctly |
| C-05 | L1 input while controlled hero hurt / swap cooldown | Existing 6-tick buffering behavior preserved or explicitly amended/tested; no phantom swap after command |
| C-06 | Focus target dies / disappears / changes room | Finishes cleanly; no automatic new target; stable ID cannot alias a newly spawned enemy |
| C-07 | Hold at high platform / unsafe ledge / camera edge | Refuse unsafe anchor or cancel safely; no softlock and no forced camera blockage |
| C-08 | Carl-only barrier and Donut-only perch (existing E1) | Correct capability checks; no AI gate bypass or impossible solo route; swap works |
| C-09 | Donut AI at 2 / 3 mana | Cast only when post-spend reserve ≥2, with same mana regeneration and shared ability costs |
| C-10 | Explicit ContextAct on authored prop | Only legal physical interaction, no accidental pickup/forced explosive trigger; completion on world-state event |
| C-11 | Human setup of dramatic launch/kill | AI support preferred over steal where safe; actual deterministic instigator gets credit; no invented spectator points |
| C-12 | Boss / Dozer chase / elevated room | Defensive overrides, cinematic tell readability and camera reachability; cannot issue lethal suicidal order |
| C-13 | 2P hot-join while solo order pending | Removes order and AI authority; P1 Carl/P2 Donut; P2 disconnect pauses, no unintended AI takeover |
| C-14 | 640×360 vs 640×400 HUD in real Bevy | Order overlay/glyphs fit; no occlusion of boss, hero plates or critical attacks; no new `bevy_ui` dependency |
| C-15 | Replay each scenario with same input/tuning | Identical state hash, commands, target IDs, interactions and kill attribution across runs |
| C-16 | No commands, both roles | Both AI Carl and AI Donut useful without manual intervention; no indefinite idle/wait after routine obstacles |

Existing fixture names worth extending rather than replacing: `solo_swap_continuity`, `solo_follower_dodge`, `duo_independent_moves`, `solo_guard_floors_at_one`, boss tests, Donut mana drain/regeneration/unlock, Q10 moves, E1 sequential character gates.

## 13. Human playtest experiments and proposed decision gates

**Prototype A/B (gamepad-only control UX):** one cohort of beginners and experienced action players; short repeatable encounters; counterbalance which layout is played first. Compare L1 short/hold against R1 short/hold and existing instant-swap baseline. Record: L1 press-to-swap perceived latency, failed swaps, wrong target, face-button leakage, interactions delayed, number of commands attempted and voluntarily used.

**Autonomy A/B:** same three encounters with (i) current follower (ii) defensive support (iii) full limited attack/support. Blind order optional; measure *helpfulness without instructions*, undesired kills, hit rates, player entertainment, need to rescue companion, and solo/2P difficulty drift.

**Suggested hypotheses to test—not release criteria:** median command issue time <0.5s after learning; ≥90% commands select intended target; almost no accidental swap+order events; zero mandatory-route softlocks; new AI reduces instances of player babysitting without reducing human share of spectacular moments. Escalate on *any* deterministic softlock or route bypass, not only frequency.

**Decision gates:**

- **D-CTRL:** choose L1 hold only if tap-to-swap remains subjectively immediate under real combat pressure; otherwise revisit binding/gesture without overriding contract.
- **D-AUTO:** choose safe autonomous attack frequency, target avoidance and whether AI is allowed to get routine finishes.
- **D-FOCUS:** evaluate whether auto candidate targeting works in dense rooms; if not, prototype low-complexity cycling.
- **D-HOLD:** determine how much Hold/defend is actually needed vs simply Regroup+Focus; drop it if it adds no meaningful play.
- **D-CONTEXT:** decide which authored environmental objects can be manipulated by AI without undermining the switch mechanic.

## 14. Delivery plan / parallelizable implementation

**PR A — Companion perception and deterministic support (no new binding).** Add shared AI director, sensing, legal action scoring and basic protect/interruption behavior for **both** bot roles; preserve old follow and solo safeguards; record/replay metrics. Feature flag default off until playtested. Ensure AI Donut mana reserve and safe Q10 moves.

**PR B — Capability-aware 2D navigation.** Add platform graph, safe anchors, character-specific edges, traversal hazard checks, dynamic destruction edge invalidation, camera-safe regroup and gate coverage. Can start parallel to PR A if the interfaces (`NavQuery`, `CandidatePosition`) are agreed.

**PR C — Command intent and input UX spike.** Separately prototype L1 tap/hold + 4 face chords and R1 hold alternate on a fixture, measure on physical pads; add command status/rejection types and snapshots; do not ship a bound control until D-CTRL passes. Integration depends on director semantics, not on finished visual art.

**PR D — Order execution and compact HUD feedback.** `Assist`, `Focus`, `Hold`, `Regroup`, authored `ContextAct`, cancellation/feedback, Bevy HUD using real approved pixels/plates, glyph variants, subtitles and controller configuration. Requires PR A and UX decision; navigation support for advanced interactions from PR B.

**PR E — Cooperative combo arbitration and attribution.** Anti-finisher-steal tuning, audience/viewer credit, legal physics opportunities, richer profiles/barks, command-combo synergy; offline scripted comparisons. Depends on stable A/B/D APIs.

**PR F — ML/offline policy experimentation (optional later).** Expose deterministic observations, legal action masks and reward/metric adapters in headless environments; train candidate policies to propose actions. Keep validation/authority deterministic and compare against fixed baseline with playtests before adoption. No runtime LLM/inference dependency required.

### Definition of done (v0 candidate)

1. Both AI-control directions are implemented and measured; no Donut-specific special-case control system.
2. Routine play needs no orders; orders improve intentional coordination, not basic survivability.
3. One-screen same narrative route solo and 2P; no gate bypass, softlock, free-resource exploit or unauthorised interactions.
4. A single input can never yield both an attack/swap and a command; controls work on real pads and fallback keyboard.
5. Order status & reasons are deterministic; temporary presentation remains readable in actual 640×360 and 640×400 scene screenshots.
6. Every legal action shares human moveset, costs and collision physics; semantic events/kill credit are traceable and repeatable.
7. Reviewer has headless tests, replay hashes, control usability evidence, per-role gameplay captures and a short decision log for unresolved tunables.

## 15. Explicit non-goals

- No free-form natural-language order parsing, LLM at runtime or character-to-character AI negotiation.
- No programmable multi-command queue, complex radial inventory, arbitrary waypoint cursor, compulsory right stick/D-pad order navigation, or pause-on-every-command tactics.
- No automatic special-resource explosives, automatic puzzle completion, early unlocks, invented cat controls or forced shoulder mode.
- No 2P override of human inputs, no third-character Mongo control system or netcode change.
- No claim that this proposal is book canon, production art, playtested gameplay or an implementation merged into the repository.

## 16. Items to resolve with the owner after a prototype

1. **Binding:** does delayed L1 tap-to-swap feel good enough in real combat to earn long-press commands, or must L1 stay instant on press?
2. **Order economy:** are four intents useful, or can `Hold` merge into `Regroup/Focus`?
3. **Targeting:** how much explicit precision is worth adding beyond facing/candidate preview?
4. **AI initiative:** do autonomous finishing blows improve team personality or make the human less satisfied?
5. **Puzzle authority:** which contextual interactions are fair to command, versus better solved by switching directly?

These are focused human-playtest decisions. All remaining requirements can proceed as design proposals without claiming approval.