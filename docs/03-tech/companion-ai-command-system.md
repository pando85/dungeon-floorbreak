# THE CRAWL — Companion Autonomy and Command System

**Status:** Design proposal v0.3, reconciled with COMBAT-FUN-02 (#124) and main (AUDIENCE-01 #118, CINE-01 #122, Donut Q10 and mana #123, CI #125, AUTOPLAY-01 #126). Defaults CA-01..CA-41 adopted by the orchestrator under the owner's design-defaults directive (2026-11-19); implementation status (2026-11-20): the unwired core modules 01A-core #129 (`e6c8c3b9`), 01B-core #131 (`811dc9b4`) and 01C #130 (`5b343867`) are merged on main, game-feel review pending; no behaviour is wired into a scene yet (01A-wire, 01B-wire, 01D and 01E are not started); game feel and controller UX pending the owner's play.  
**Date:** 2026-11-20  
**Scope:** Floor 01 solo (either Carl or Donut controlled by AI) with compatibility requirements for 2P, existing party/hud rules, later narrative progression and offline policy evaluation.  
**Primary goal:** Make the AI companion useful, intelligible, characterful and tactically interesting **without turning a combat-first platform-action game into a command-management game**.

> This document proposes gameplay rules, control experiments, architecture and test gates. Numeric values are *starting hypotheses*, never tuned/approved facts. It does not modify the current implementation or supersede owner-approved controller/party/HUD specifications.

## 0. Evidence and binding constraints

Reviewed on repository `pando85/dungeon-floorbreak` (main, 2026-11-20; integration baseline `e9bea792e695207adf5d32352b80e40d035add61`; re-checked against PR #124 revision 2.1 and the PR #127 head on 2026-11-20):

- `docs/01-design/mvp-controller-contract.md`: Cross/A jump; Square/X primary; Triangle/Y physical; Circle/B special; R1/RB interact; L1/LB swap in solo; L2 and R2 reserved for future actions. **Held L1 for a companion command wheel is reserved, not approved**; no mandatory right stick or D-pad commands, as D-pad is also movement.
- `docs/01-design/party-2p-open-questions.md` and `src/party.rs`: solo swap (20-tick cooldown, 6-tick buffer, blocked while controlled hero hurt), follower bot, 1 HP floor only while AI-driven, bias against AI targeting, offscreen protection; 2P P1 Carl/P2 Donut, disconnect pauses with no automatic takeover. The bot is selected by the Q9 knob `PartyTuning::uncontrolled_hero` (`FollowAndJumpBot` today). Under `--cf02` the swap cooldown is 12 ticks (CF02-Q19) and the swap pan eases in and out (CF02-Q20).
- `src/follower.rs`: deterministic movement/jumps/basic tell avoidance; **never attacks** and currently lacks a navigation planner. Its movement is produced in `src/corridor.rs` (`SceneSim::step_live` calls `follower_input_boss`); the uncontrolled hero's slot input is `RawInput::default()` (`PartyState::hero_inputs`), and the AI Donut cannot start a move (`may_start: !bot && can_act` in `src/combat_scene/donut.rs`).
- `src/input.rs`: logical `RawInput`, tick/pressed-edge semantics; `swap_pressed` is currently generated on **press**, not release. This matters critically if a long press is added. COMBAT-FUN-02 adds `RawInput::NONE` (CF02-00) and a `down` field (CF02-A); every companion `RawInput` literal ends with `..RawInput::NONE`.
- `docs/01-design/asymmetric-traversal-and-gates.md`: a mandatory obstacle may explicitly require Carl or Donut; solo solvability through switching is intended. AI must not bypass character-specific capability gates or softlock the game. That spec is not implemented in the integrated scene on main; the only capability-tagged geometry today is the camera-lab room's `donut_only` high platforms (CA-33).
- `docs/02-art/hybrid-hud-a-native-spec.md`: native pixel HUD in Bevy with dedicated HUD Camera2d and Text2d, 640×360 / 640×400, hero plates at `(8,8,164,48)` and `(468,8,164,48)`, top boss/location strips, action lane y≥60. AI plate does not show human ability chips. Avoid adding permanent HUD clutter or `bevy_ui` implicitly.
- `docs/01-design/donut-mana-v0-proposal.md`, `docs/02-art/donut-moveset-q10.md`: swat, pounce, post-unlock Magic Missile; mana max 8, cost 1, **existing AI reserve is 2 charges remaining after casting**. A new command system must respect the reserved floor, narrative unlocks and existing move constraints.
- `docs/01-design/gameplay.md`: combat-first, readable knockback, environmental combinations; same core game in solo and 2P; controller simplicity; correct event attribution for individual audience/viewership features.
- `docs/02-art/individual-viewers-hud-v1.md` (AUDIENCE-01, #118): per-hero live viewers credited by **hero index** (`audience::credited_hero`); preview and QA only, absent at the start of Floor 01 (F15). The plate name row band +80..+158 x +3..+14 holds the readout.
- `docs/03-tech/cinematic-finishers-cine-01.md` (CINE-01, #122): credit is the hero index carried by the event, eligibility fails closed, and `CINE-DUO` must never treat a solo companion action as P2.
- `docs/03-tech/autoplay-01-automated-gameplay-evaluation.md` (AUTOPLAY-01, #126): Carl-only v1 env on `FixtureSim`; kill attribution rules AP-Q14; perception mask AP-Q07 uses the same sim scroll-camera frame as CF02-Q40.

**Concurrent work, reconciled (section 17):** COMBAT-FUN-02's restrained assistant (CF02-C2 as first drafted: Q13-Q17, the AI part of Q38, Q39, Q40) is delivered by this work as the companion profile set `cf02_assist`; CF02-A no longer builds an `AssistThreats` view, the shared read-only perception contract is `CompanionView`; CF02-C2 keeps the swap cooldown and eased pan (Q19, Q20) and Donut's bail-out (Q55). AUTOPLAY-01 v1 stays Carl-only; 01E evaluates the companion on AUTOPLAY-02a, a one-seat solo-party env owned by the AUTOPLAY lead and planned after AUTOPLAY stage B (section 17 (d)). The authoritative execution breakdown is in [the companion work order](../07-agents/companion-ai-01-workorder.md).

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
5. **Human gets meaningful action ownership.** Whether an AI hit may kill is a data knob (`ca19_kill_policy`, CA-19): `never` through 01D (an AI-started lethal hit becomes a glance), and `factual` as the candidate default that 01E tests and compares with `never` (O2a). Under either, only the actual instigator receives kill and viewer credit, and the AI should not hijack a finisher the player is setting up.
6. **Same simulation, same rules.** Human and AI actors use the same abilities, physics, collision, resources, cooldowns and attack startup/recovery. No companion-only teleport, damage or free mana except the existing explicit solo safety safeguards and the `ca19_kill_policy` glance (CA-20).
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

**Carl combo:** Donut can extend a launch or interrupt the next attacker. During a player-finisher opportunity, prefer protect/extend instead of stealing the same target unless `Focus` was explicitly commanded. Under the default `ca19_kill_policy = never` an AI-started hit cannot kill at all; this preference matters for the `factual` policy.

**After biscuit:** Magic Missile only if `mana >= missile_cost + ai_mana_reserve` (`DonutMana::ai_can_cast`: 1 + 2 = 3 charges before release), suitable target/line-of-fire and a compelling tactical advantage. No invented free casts. Avoid sniping every weakened goblin Carl is winding up to finish. Profile `cf02_assist` adds CF02-Q17's limits: only to interrupt a tell aimed at the controlled hero, 97..160 px away (since CF02-DUO-A04/A05 the profile also casts offensively within the same mana rules, section 17 (e)).

### Donut controlled, Carl AI

**Goblin rush:** Carl screens the approaching goblin, interrupts with a legal jab (push kick deferred, CA-37) or maintains the route while Donut kites. He must not use a grab (canon contract: no grabs). v0 AI Carl move set: CA-37.

**Donut airborne/perching:** Carl follows paths *he* can traverse, not Donut's jumping trajectory. If a mandatory Donut-only perch or Carl-only breach separates them, he waits safely until the human switches to operate the gate. Do not bypass the gate via teleport.

**Carl tools unlocked:** Disruptive/environment-changing explosives are not automatically used. In v0, explicit direct control is required for dangerous explosive placement/detonation. A later `ContextAct` can operate one only after authored safety predicates, a clear target cue and distinct player confirmation, to be separately designed.

### Both directions

**No orders:** both characters should independently acquire useful positions, avoid tells, and perform limited support actions. Whether routine AI kills are allowed is the `ca19_kill_policy` knob (CA-19); credit is always the real actor's, and the CF02-Q39 ceiling (AI lands at most 25 % of hits; 50 % for `cf02_assist` since CF02-DUO-A03) applies.

**Character personality:** personality changes initiative/tactical preferences, barks and idles—not the semantics or reliability of `Regroup` and `Focus`. Donut may make a grandiose comment; Carl may respond dryly; neither may refuse a valid order for a joke. Avoid incessant voice feedback; text-only subtitles toggle required if audio barks are used. No voice barks in 01A-01D (CA-38).

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

**Key input contract:** swap is provisionally detected on **release**, not on press, after checking no chord or hold mode. This is a deliberate migration of the currently adopted Q8 press behavior. Existing cooldown (20 ticks; 12 under `--cf02`, CF02-Q19), buffer (6 ticks) and hurt block remain; their precise handling must be revised and regression-tested. Do not silently switch on press and then switch back if L1 becomes a hold—that visibly breaks authority/camera/ongoing attacks.

**Mode state machine:** `Idle → L1Candidate → [CommandMode | TapSwapReady] → Idle`. Enter `CommandMode` at threshold or a face chord; first face edge is consumed exclusively as a command; releasing unused command mode cancels with no swap; no face attack, jump or special leaks from command mode. `TapSwapReady` on L1 release emits exactly one `SwapRequest`. Pause, controller disconnect, dead/respawning state and transition to 2P clear modal inputs and pending commands. Entering `CommandMode` clears the controlled hero's move buffer (`src/input_buffer.rs`, CF02-H) and the party swap buffer; a face press consumed by command mode never reaches the move buffer (CA-27).

**Crucial UX risk:** tap-to-swap on release adds perceptible latency compared to existing press-to-swap. Measure real DualSense and Steam Deck; specifically playtest swap-as-a-combat-move. If this violates accepted combat feel, reject A rather than disguising the regression.

### Alternative B: R1 hold command modifier, L1 instant swap

Pros: preserves zero-delay L1 swap. Cons: interferes with existing/required R1 interact, delayed interact, inadvertent pickup/interaction after command cancel, especially with traversal gates. A hold/tap split introduces the same ambiguity and requires input migration. **Not recommended unless A fails timing and B passes interactions.**

### Alternative C: explicit button chord using an extra modifier / remap

Potential accessibility control scheme, not default. Any mandatory right-stick/R3, touchpad, gyro, paddles or D-pad order navigation **violates existing controller requirements**. A remappable accessible toggle for command mode can be offered without changing canonical defaults. If A and B both prove poor, escalate the control conflict for owner review; never silently consume R2 (reserved cycle special). CF02-A binds Down (pad Down, keyboard S or Down arrow) for the stomp and dive; no command chord may use Down.

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

**Anti-steal heuristic:** mark a short `HumanFinishWindow` candidate when the human has initiated an attack/combo that is likely to defeat a specific target, especially a launched enemy about to hit an environmental object. Unless `Focus` was explicitly issued, prefer interrupting the other enemy, holding space, protecting the player or contributing a non-lethal assist to that target. This is *not* an invulnerability grant to enemies, and does not suppress legal collisions or a kill already committed by the AI. Measure whether the heuristic is needed before adding hidden exclusions. The heuristic is only needed under `ca19_kill_policy = factual`; under `never` it still steers positioning (protect, interrupt the other enemy).

**Fairness (CA-19, CA-20, CA-29):** credit is factual under every policy: the hero-index `instigator` of the event, as AUDIENCE-01 and CINE-01 already read it. Under `never`, an AI-started hit whose damage would reach the target's HP deals 0 on its hit tick and keeps the move's non-lethal reaction (`Hit` logged with damage 0); indirect kills (contact, wall slam, shore, hazard) are not suppressed, are credited to whoever the existing event chain names and are counted as `ai_indirect_kills`. Under `factual` the AI may kill with its own legal actions. A defeat credited to a hero that was companion-driven at the defeat tick never triggers a CINE-01 moment (fail closed: the CINE feed does not forward to `CinematicEvaluator` a `CineEvent::Defeated` whose credited hero, the `instigator` or the cause's `player` for `ShoreBlock`, `ShoreBlockEdge` or `DropHazard`, was companion-driven at that tick; `slot` alone does not gate a moment); spectacular status is never awarded for AI involvement alone, and no credit is rewritten in the player's favour.

**Resource limits:** Donut's AI preserves the existing `ai_mana_reserve=2` **remaining after cast**; Carl's dangerous tools are manual until explicitly designed. A player swap does not replenish or reset resource/cooldown state. AI must not exploit the existing AI 1-HP guard to tank damage recklessly; the guard is a last-resort fairness mechanism, not a strategy. The reserve is read from `assets/data/gameplay/companion_magic_v0.json` through `DonutMana::ai_can_cast`, never copied.

## 9. HUD, sound, personality and feedback

**Always-on:** keep existing Carl/Donut plates and `AI` ownership tag. No permanent extra command panel. The AI plate can expose *one compact status icon* (Follow / Focus / Hold / Regroup) as a future native HUD extension, with an alternative minimal semantic label if legible. Do not remove the existing L1 Swap affordance or introduce fake ability chips for AI. The single state icon must stay outside the AUDIENCE-01 readout band (plate-relative +80..+158 x +3..+14) (CA-35).

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

1. Physical device adapter → logical per-player buttons/edges; resolve L1 tap/hold/chord *before* mapping face presses to human attacks. (01D only; until D-CTRL the production input path is unchanged)
2. On single-player input: if swapping, call existing `PartyState` swap authority and clear old AI directive safely; if issuing, build `CommandIntent` for current AI recipient (never human).
3. Validate command against mode, story capabilities, target ID, camera/nav state and safety. Emit stateful acknowledgment/reason event.
4. Build the read-only `CompanionView` (section 17) from pre-tick state; update safety/urgent avoid **each tick**; recompute utility preference every **6 ticks** (10 Hz starting hypothesis), immediately on threats/orders/terrain changes. Navigation planning at a lower conditional frequency; never allow stale nav after destruction.
5. The director emits one full `RawInput` per companion-driven hero, substituted at the top of `CombatState::step_party` (hook stubbed by CF02-00); `SceneSim::step_live` keeps `humans[i]` (the injected, already gated input) for each bot hero when `matches!(drive.tuning.uncontrolled_hero, UncontrolledHero::Companion(_))`, instead of calling `follower_input_boss`; no `PartyStepCtx` change; Donut's `may_start` admits companion presses; lethal AI hits pass through the glance rule (`companion::rules::glance`). No direct writes to positions, health, mana or cooldowns (CA-24).
6. Resolve actual physics, damage, kills, interactables, attribution, ability resources; record outcomes. HUD/audio read committed events only.
7. Hash and replay all gameplay-authoritative state, including command sequence, order, timers, selected stable target ID, policy version, navgraph revisions and deterministic tie breaks.

**Separate modules:** command input grammar (UI-adapter/domain-neutral), director/authority, order validation, world sensing, candidate utility, navgraph/reachability, actor profile and presentation adapter. Keep render and human barks outside gameplay core. Integrate with existing `PartyDrive`, `PartyState`, `follower` and `CombatState` incrementally, not with a parallel combat engine. Module layout: `src/companion/{mod,view,profile,director,rules,nav,command_input}.rs` (Bevy-free), the builder `src/combat_scene/companion.rs` (child module, immutable reads), fixtures in `src/companion_fixtures.rs`.

**Important:** clear the command modal on Start/pause, controller disconnect/reconnect, stage transition, checkpoint restart, switch to 2P, or death; invalidate old target IDs after entity deletion. An AI may still finish an already committed move across a swap, as required by existing Q8 behavior.

**Config knobs** in `assets/data/gameplay/companion_ai_v0.json` (created by 01A-core): loaded, validated and FNV-1a pinned like `companion_magic_v0.json`; a load error names the field; knob names carry their question ID (`ca10_decision_cadence_ticks`, `cf02_q14_swat_tell_tick`, ...). Profile sets: `cf02_assist` (CF02-Q13..Q17 and Q39 values, AI Carl passive) and `companion_v0` (both roles), each with `ca19_kill_policy`, `cf02_q16_action_cadence_ticks` (CF02-Q16, 60) and `ca26_ai_bailout` (false). The hold threshold and command knobs arrive with 01C/01D. Canonical mana values stay in `companion_magic_v0.json`; they are not duplicated.

## 11. Command event instrumentation

Recommended event IDs for record/replay/telemetry: They form a separate `CompanionEvent` log drained like `take_new_events`, not new `CombatEvent` variants; `CompanionDecision` is a debug trace and is never hashed (CA-25).

- `OrderPreviewed { tick, actor, verb, candidate }` (presentation-only; no gameplay hash if ephemeral)
- `OrderIssued { tick, sequence, issuer, recipient, verb, target }`
- `OrderAccepted/OrderRejected { tick, sequence, reason }`
- `OrderExecutionStarted/OrderDeferred/OrderCompleted/OrderCancelled { tick, sequence, reason }`
- `CompanionDecision { tick, actor, state, chosen_move, candidate, utility, safety_override }` (debug trace, sampling/capping as needed)
- `CompanionNavFailure { tick, actor, current_anchor, target_anchor, reason }`
- `AssistContribution { tick, actor, target, kind }` and existing authoritative defeat/launch/environment event chain

Metrics: orders per minute; command acceptance and failure rates; false-positive target selection; order-to-first-action latency; human-attack interference; human-finisher stealing; useful interrupts per fight; companion incoming hits; AI resource spending; no-order solo survival; mandatory-gate reachability; time stuck; forced regroup frequency; FPS CPU time of sense/nav/planning. Attribution follows AP-Q14 (instigator 0 Carl, 1 Donut, None unknown); add CF02-Q39's assist starts and AI hit share, glances and `ai_indirect_kills`.

All gameplay metrics and event chain must work in **headless** fixed-tick fixtures with repeated seed/run hashes. Keep optional anonymized telemetry out of v0 runtime unless separately approved; local fixture metrics suffice.

## 12. Representative tests (headless first)

| ID | Scenario | Required assertion |
|---|---|---|
| C-01 | Carl human, Donut AI; 2 goblins and one tell | Donut can safely swat/protect, not cast pre-biscuit, no infinite stagger loop |
| C-02 | Donut human, Carl AI; pressure from front | Carl screens by position and interrupts with a jab (the push kick joins with the first launching-move PR, no later than 01E; CA-37); no grab, explosive or Down move |
| C-03 | L1 tap vs hold vs fast chord | Exactly one swap or one order, never both; no leaked Cross jump/Square attack; gamepad, keyboard, reconnect parity |
| C-04 | Swap in the middle of AI action or order | Previous AI order cancelled; legal started attack completes; other hero assumes autonomous role; camera transitions correctly |
| C-05 | L1 input while controlled hero hurt / swap cooldown | Existing 6-tick buffering behavior preserved or explicitly amended/tested; no phantom swap after command |
| C-06 | Focus target dies / disappears / changes room | Finishes cleanly; no automatic new target; stable ID cannot alias a newly spawned enemy |
| C-07 | Hold at high platform / unsafe ledge / camera edge | Refuse unsafe anchor or cancel safely; no softlock and no forced camera blockage |
| C-08 | Character-capability geometry (camera-lab `donut_only` platforms now; an authored gate fixture when asymmetric gates exist) | Nav refuses Carl on Donut-only edges; no AI gate bypass; gate assertions are blocked until a gate fixture exists (CA-33) |
| C-09 | Donut AI at 2 / 3 mana | Cast only when post-spend reserve ≥2, with same mana regeneration and shared ability costs |
| C-10 | Explicit ContextAct on authored prop | Only legal physical interaction, no accidental pickup/forced explosive trigger; completion on world-state event |
| C-11 | Human setup of dramatic launch/kill | Under `never`: no AI-started hit kills (glance logged), indirect kills credited factually; under `factual`: support preferred over steal where safe; credit is the deterministic instigator; no CINE moment for a companion-credited defeat (CA-29); no invented spectator points |
| C-12 | Boss / Dozer chase / elevated room | Defensive overrides, cinematic tell readability and camera reachability; cannot issue lethal suicidal order |
| C-13 | 2P hot-join while solo order pending | Removes order and AI authority; P1 Carl/P2 Donut; P2 disconnect pauses, no unintended AI takeover |
| C-14 | 640×360 vs 640×400 HUD in real Bevy | Order overlay/glyphs fit; no occlusion of boss, hero plates or critical attacks; no new `bevy_ui` dependency |
| C-15 | Replay each scenario with same input/tuning | Identical state hash, commands, target IDs, interactions and kill attribution across runs |
| C-16 | No commands, both roles | Both AI Carl and AI Donut useful without manual intervention; no indefinite idle/wait after routine obstacles |
| C-17 | The eight `cf02_assist_*` fixtures moved from CF02-C2 (profile `cf02_assist`) | CF02-Q14..Q17, Q38 (AI part), Q40 assertions as adopted in the COMBAT-FUN-02 design |
| C-18 | Rollback and camera parity | With `--companion off` every legacy golden is byte-identical; `--camera auto` and `--camera scroll` give identical companion decisions and hashes |

Existing fixture names worth extending rather than replacing: `solo_swap_continuity`, `solo_follower_dodge`, `duo_independent_moves`, `solo_guard_floors_at_one`, boss tests, Donut mana drain/regeneration/unlock, the `q10_*` party goldens and, once landed, the `cf02_*` fixtures. No character-gate fixture exists on main yet.

## 13. Human playtest experiments and proposed decision gates

**Prototype A/B (gamepad-only control UX):** one cohort of beginners and experienced action players; short repeatable encounters; counterbalance which layout is played first. Compare L1 short/hold against R1 short/hold and existing instant-swap baseline. Record: L1 press-to-swap perceived latency, failed swaps, wrong target, face-button leakage, interactions delayed, number of commands attempted and voluntarily used.

**Autonomy A/B:** same three encounters with (i) current follower (ii) defensive support (iii) full limited attack/support. Blind order optional; measure *helpfulness without instructions*, undesired kills, hit rates, player entertainment, need to rescue companion, and solo/2P difficulty drift.

**Suggested hypotheses to test—not release criteria:** median command issue time <0.5s after learning; ≥90% commands select intended target; almost no accidental swap+order events; zero mandatory-route softlocks; new AI reduces instances of player babysitting without reducing human share of spectacular moments. Escalate on *any* deterministic softlock or route bypass, not only frequency.

**Decision gates:**

- **D-CTRL:** choose L1 hold only if tap-to-swap remains subjectively immediate under real combat pressure; otherwise revisit binding/gesture without overriding contract.
- **D-AUTO:** choose safe autonomous attack frequency, target avoidance and whether AI is allowed to get routine finishes. Split in two: O2a (D-KILL) whether the AI should land real kills, answered after 01E's comparison of `never` and `factual`; O2b whether AI Carl is active (`companion_v0`) or passive (`cf02_assist`) in the arena the owner plays.
- **D-FOCUS:** evaluate whether auto candidate targeting works in dense rooms; if not, prototype low-complexity cycling.
- **D-HOLD:** determine how much Hold/defend is actually needed vs simply Regroup+Focus; drop it if it adds no meaningful play.
- **D-CONTEXT:** decide which authored environmental objects can be manipulated by AI without undermining the switch mechanic.

## 14. Delivery plan / parallelizable implementation

PR A..E are the work order's stages 01A..01E (01A splits into 01A-core and 01A-wire, 01B into 01B-core and 01B-wire); PR F is AUTOPLAY-03 territory and stays optional.

**PR A — Companion perception and deterministic support (no new binding).** Add shared AI director, sensing, legal action scoring and basic protect/interruption behavior for **both** bot roles; preserve old follow and solo safeguards; record/replay metrics. `UncontrolledHero::FollowAndJumpBot` stays the default; `--companion cf02_assist|v0` opts in (CA-22). Ensure AI Donut mana reserve and safe Q10 moves.

**PR B — Capability-aware 2D navigation.** Add platform graph, safe anchors, character-specific edges, traversal hazard checks, dynamic destruction edge invalidation, camera-safe regroup and gate coverage. Can start parallel to PR A if the interfaces (`NavQuery`, `CandidatePosition`) are agreed.

**PR C — Command intent and input UX spike.** Separately prototype L1 tap/hold + 4 face chords and R1 hold alternate on a fixture, measure on physical pads; add command status/rejection types and snapshots; do not ship a bound control until D-CTRL passes. Integration depends on director semantics, not on finished visual art.

**PR D — Order execution and compact HUD feedback.** `Assist`, `Focus`, `Hold`, `Regroup`, authored `ContextAct`, cancellation/feedback, Bevy HUD using real approved pixels/plates, glyph variants, subtitles and controller configuration. Requires PR A and UX decision; navigation support for advanced interactions from PR B.

**PR E — Cooperative combo arbitration and attribution.** Anti-finisher-steal tuning, audience/viewer credit, legal physics opportunities, richer profiles/barks, command-combo synergy; comparisons on the AUTOPLAY-02a solo-party env (CA-31). Depends on stable A/B/D APIs.

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
- No claim that this proposal is book canon, production art or playtested gameplay, or that any companion behaviour runs in the game: only the unwired core modules (01A-core, 01B-core, 01C) are merged into the repository.

## 16. Items to resolve with the owner after a prototype

1. **Binding:** does delayed L1 tap-to-swap feel good enough in real combat to earn long-press commands, or must L1 stay instant on press? (O1, D-CTRL; needs real pads, otherwise NOT RUN.)
2. **Order economy:** are four intents useful, or can `Hold` merge into `Regroup/Focus`? (O4, D-HOLD.)
3. **Targeting:** how much explicit precision is worth adding beyond facing/candidate preview? (O6, D-FOCUS.)
4. **AI initiative:** do autonomous finishing blows improve team personality or make the human less satisfied? (O2a, D-KILL: should the AI land real kills? The owner answers after 01E's comparison.) Through 01D the default is `ca19_kill_policy = never`; `factual` is the candidate default that 01E tests on the same seeds and records a recommendation for (CA-19).
5. **Puzzle authority:** which contextual interactions are fair to command, versus better solved by switching directly? (O5, D-CONTEXT.)
6. **Companion spectacle:** may a companion-credited defeat ever trigger a CINE-01 moment (default no, CA-29)? (O3.)
7. **Arena role of AI Carl:** is AI Carl active (`companion_v0`) or passive (`cf02_assist`) in the arena the owner plays? (O2b, D-AUTO.) Default: `--cf02` selects `cf02_assist`.
8. **Two-pad hot-join:** real two-pad hot-join with a pending order (C-13 live check). (O7; NOT RUN until two physical pads are available.)

These are focused human-playtest decisions. All remaining requirements can proceed as design proposals without claiming approval.

## 17. Reconciliation with COMBAT-FUN-02 and AUTOPLAY-01

Reconciled on 2026-11-20 against COMBAT-FUN-02 ([design](../01-design/combat-fun-02-post-demo01-proposals.md), [work order](../07-agents/combat-fun-02-workorder.md), PR #124 revision 2.1, merged), the AUTOPLAY-01 spec and work order on main and the PR #127 head. Decisions use CF02-Qnn where COMBAT-FUN-02 already adopted a value and CA-19.. for new COMPANION decisions; all are adopted defaults under the owner's design-defaults directive, game feel pending. The CA texts are in [the work order](../07-agents/companion-ai-01-workorder.md#provisional-design-decisions-for-local-agents). Code facts on main: the bot's movement is made in `corridor.rs` `SceneSim::step_live` by `follower_input_boss`; `hero_inputs` gives the uncontrolled hero `RawInput::default()`; AI Donut can never start a move (`may_start: !bot && can_act`, `combat_scene/donut.rs`); the Q9 knob is `PartyTuning::uncontrolled_hero: UncontrolledHero { TrailFollowNeverAttacks, FollowAndJumpBot }`; `PartyFixture` carries a full `PartyTuning`; `SceneSim::is_offscreen` is the sim scroll-camera test.

### (a) Absorb / keep

| CF02 item | Goes to | Reason |
| --- | --- | --- |
| Q13 opt-in assistant (`PartyTuning::follower_assist`) | COMPANION 01A-wire, as `UncontrolledHero::Companion(Cf02Assist)` (CA-21) | One director for every AI-driven hero (CA-01); the Q9 enum already is the "who drives the bot" knob; a bool beside it would fork the AI |
| Q14 assist swat, Q15 assist pounce, Q17 assist magic | 01A-wire, profile `cf02_assist` in `companion_ai_v0.json`, numbers unchanged | Donut profile data, not party knobs; same values the CF02 review adopted |
| Q16 one AI move per 60 ticks | 01A, profile knob `cf02_q16_action_cadence_ticks` (both sets) | Distinct from CA-10's 6-tick decision cadence; both kept (CA-36). A chain follow-up inside the running move's window (haymaker, follow-through kick) is part of the same AI-started move and is not counted |
| Q38 AI lethality re-check on the hit tick | 01A, generalised to both heroes as the `ca19_kill_policy` knob (`never` default through 01D; CA-19, CA-20) | Same rule, one implementation for AI Carl and AI Donut |
| Q38 hold (pounce pauses the down timer, any Donut) | Stays CF02-A (`goblin.rs`) | A goblin rule that also serves a human Donut in 2P |
| Q39 activity floor (>= 3 assist starts, <= 25 % hits) | Shared: knob in the companion profile, acceptance stays in CF02-F `cf02_arena_direct` | F owns the arena; the floor measures the companion |
| Q40 "in view" = sim scroll camera | Shared perception contract rule (CA-23) | Same predicate as AP-Q07 and llama fair fire |
| `AssistThreats` view (CF02-A, in `step_party`) | Replaced by COMPANION `CompanionView` (CA-23) | One perception contract; A keeps its state `pub(crate)` instead of building an AI view |
| `cf02_assist_*` fixtures (8) | 01A-wire, same names, in `src/companion_fixtures.rs` | They test the companion under the `cf02_assist` profile |
| Q19 swap cooldown 12, Q20 eased pan (`corridor.rs`) | Stay CF02-C2 (reduced: "swap and Donut bail-out") | Swap/camera feel, not AI; `cf02_swap_eased_pan` stays |
| Q55 Donut bail-out (C6) | Stays CF02-C2 | A move; the AI never bails out (CA-26) |
| C1 llama targets Q18, Q37, Q44 | Stays CF02-C1 | Move/target rules; the view lists llama tells and targets when `q18_llama_targets` is on |
| CF02-A Down input, `pad_slots.rs` Down, keyboard S/Down | Stays CF02-A | v0 companion profiles never press Down (CA-24); 01C does not touch these files (CA-34) |
| CF02-H `input_buffer.rs` | Stays CF02-H | Injected presses go through it like human presses; command mode clears it (CA-27) |

### (b) Conflicts and adopted rules

| ID | Conflict | Adopted rule |
| --- | --- | --- |
| CA-19 | CF02-C1/C2 "AI never lands a killing blow" vs CA-16 "AI may earn real spectacular kills" | Data knob `ca19_kill_policy` per profile set: `never` (default through 01D, not the long-term default) or `factual` (the candidate default, tested in 01E, which compares it with `never` on the same seeds and records a recommendation; the owner decides, O2a). Motivation: the owner asked for "genuinely capable" companions, and the DEMO-01 playtest rated companion Donut 2/5. Credit is always factual: the hero that actually caused the defeat (hero-index `instigator`, as AUDIENCE-01 and CINE-01 already require). CA-16 is restated as the `factual` hypothesis |
| CA-20 | What "killing blow" means for AI Carl, whose moves deal damage and knockback | An **AI-started** hit (move started by an injected press; flag survives a swap, CF02-Q16) whose damage >= target HP deals 0 on its hit tick, keeps the move's non-lethal reaction, logs `Hit` with damage 0. Indirect kills (contact, wall slam, shore, hazard) are not suppressed, are credited factually and are counted as `ai_indirect_kills`; under `never` the utility penalises moves predicted to cause one; this penalty is deferred to the first PR that adds a launching move for the AI, no later than 01E (work order CA-20); 01A-wire still tallies `ai_indirect_kills` |
| CA-21 | CF02 assistant is Donut-only with AI Carl passive (Q13); 01A is both-role | Profile sets: `cf02_assist` = exactly Q13-Q17/Q39 (Carl passive = follow layer only); `companion_v0` = both roles active. `--cf02` selects `cf02_assist` (Q33 unchanged); `--companion v0` selects both-role |
| CA-22 | `cf02_rules` vs a COMPANION rollback flag | Rollback = `UncontrolledHero::FollowAndJumpBot` (today's default). CLI `--companion off\|cf02_assist\|v0`; see flags below |
| CA-23 | Who builds the threat view | COMPANION owns `CompanionView`; built by `src/combat_scene/companion.rs` (child module, reads private state immutably) from pre-tick state; CF02-A no longer builds `AssistThreats` |
| CA-24 | How the AI acts | It emits one full `RawInput` per companion-driven hero through the existing step path; no direct state writes (section c) |
| CA-25 | Hash and event ownership | Director state is mixed into the state hash only while a hero is companion-driven; order/decision events go to a separate `CompanionEvent` log (drained like `take_new_events`), `CompanionDecision` traces are never hashed; no new `CombatEvent` variants |
| CA-26 | Injected Jump during recovery would fire CF02 C3/C6 bail-out | v0 profiles never bail out (CF02-Q55): the press filter drops Jump during the AI hero's own move recovery; knob `ca26_ai_bailout: false` |
| CA-27 | 01C command chords vs CF02-H move buffer and the 6-tick swap buffer | The grammar runs before the move buffer; entering command mode clears the controlled hero's move buffer and the swap buffer; a consumed face press never reaches `input_buffer.rs` |
| CA-28 | Stable target IDs (CA-15, C-06) vs index-only enemies | `StableEntityId { kind, index, spawn_serial }`; `spawn_serial` is a per-attempt counter; 01A may use `serial = 0` (no respawn in its fixtures); 01D requires it (CF02-F waves) |
| CA-29 | CINE-01 credits any hero index; an AI kill under `factual` could trigger slow motion | A defeat credited to a hero that was companion-driven at the defeat tick never triggers CINE (fail closed: the CINE feed does not forward to `CinematicEvaluator` a `CineEvent::Defeated` whose credited hero, the `instigator` or the cause's `player` for `ShoreBlock`, `ShoreBlockEdge` or `DropHazard`, was companion-driven at that tick; `slot` alone does not gate a moment); AUDIENCE stays factual per hero (preview/QA only on Floor 01, F15). Presentation only, no hash change |
| CA-32 | Q39 floor/ceiling vs both-role companion | Q39 numbers apply per profile set; `companion_v0` reports them per role |

Further defaults without a CF02 conflict: CA-30 fixture registry and names, CA-31 01A-01D metrics via fixture tallies and 01E via AUTOPLAY-02a, CA-33 01B has no character-gate fixture to reuse (asymmetric gates are unimplemented on main), CA-34 01C is a pure module plus optional lab binary, CA-35 the AI-plate state icon avoids the AUDIENCE readout band, CA-36 decision cadence (6 ticks) and action cadence (60 ticks) are separate knobs. The full texts are in the work order.

### (c) API boundary

**Perception contract `CompanionView`** (types in `src/companion/view.rs`, Bevy-free, `Clone`, owns its data, no references into `CombatState` or `SceneSim`). Owner: COMPANION (01A-core defines, 01A-wire builds). Built once per tick at the top of `CombatState::step_party` from pre-tick state, only when some hero is companion-driven (zero cost otherwise). Fields:

- `tick` (the tick about to run), `solo`, `cf02_rules`, `profile_set` (type `ProfileSetId`; `CompanionProfileSet` is a public alias of it, the name `UncontrolledHero::Companion(CompanionProfileSet)` uses), `view: { left, right }` with `in_view(x) == !scene.is_offscreen(x, 0.0)` (CF02-Q40 = AP-Q07).
- `heroes: [HeroView; 2]`: x, foot_y, vx, vy, facing, grounded, health, hurt_ticks, invulnerable_ticks, defeated, `control: Human(slot) | Companion | Bot`, `active_move: Option<{ id: MoveId, phase, ticks_left, ai_started }>`, `can_start_move`, Donut only `mana: { mana, max, unlocked, ai_can_cast, projectile_speed_px_per_tick, bolt_lead_px }` (`ai_can_cast` from `DonutMana::ai_can_cast`, reserve not duplicated; `projectile_speed_px_per_tick` is filled from `ManaTuning` and feeds the Q17 arrival-time gate; `bolt_lead_px` is 35, `BOLT_SPAWN_FORWARD + BOLT_BOX_AHEAD + BOLT_BOX_W / 2`, the distance from Donut's pivot to the bolt's hit-box front on its spawn tick; the Q17 flight model is `floor((d - ENEMY_HALF_WIDTH - bolt_lead_px) / speed) + 1` (flight 0 when `d - ENEMY_HALF_WIDTH - bolt_lead_px < 0`) and the arrival attack tick `tell_tick + MAGIC_MISSILE.startup` (= `CAST_RELEASE_TICK`, 4) `+ flight` must be `< tell_len`).
- `follow: [FollowInput; 2]`, the follow layer's per-hero input (`move_x`, `jump`), filled by 01A-wire: the builder computes `follower_input_boss` on the same arguments `step_live` uses, pre-tick, into `CompanionView.follow`, `FollowInput::default()` when `!ctx.can_move[i]`; the director starts from it, so the view stays plain data and the movement is still today's bot.
- `threats: Vec<ThreatView>` in goblin, llama, boss index order: `id: StableEntityId`, x, foot_y, facing, `tell_tick`, `tell_len`, `impact_tick`, `aimed_at: Option<HeroId>`, `guarded` (Q10 light guard), `in_view`. `tell_len` is the first non-tell attack tick (goblin `TELL_TICKS` 24, llama `LLAMA_TELL_TICKS` 36); an interrupt must hit while attack tick `< tell_len`; `tell_tick` is the pre-tick attack tick. Llamas only when `q18_llama_targets` is on.
- `targets: Vec<TargetView>`: id, x, foot_y, hp, `state: Standing | Staggered { ticks } | Lying { down_ticks_left, hold_used } | Airborne`, guarded, in_view. `boss: Option<BossThreat>` (existing type).
- Guarantees: built from `&CombatState` and `&SceneSim` only; never written back; never hashed (it is derived from hashed state); ordered by `StableEntityId`; no presentation or camera-mode input (`--camera auto` vs `scroll` parity fixture). CF02 PRs that add state (A lying/hold, C1 llama targets, G morale) keep the fields `pub(crate)`; COMPANION PRs extend the builder.

**Press injection** (no other write path):

1. `step_party` first statement (no-op stub from CF02-00): `let inputs = self.companion_inputs(scene, inputs);` replaces `inputs[i]` for each hero with `drive.is_bot(hero)` (`HeroId`) and `uncontrolled_hero == Companion(_)` by the director's `RawInput { move_x, jump, attack, physical, ability, ..RawInput::NONE }` (pause, swap and, in v0, `down` stay false).
2. `SceneSim::step_live` keeps `humans[i]` (the injected, already gated input) for each bot hero when `matches!(drive.tuning.uncontrolled_hero, UncontrolledHero::Companion(_))`, instead of calling `follower_input_boss`; no `PartyStepCtx` change (the builder computes `follower_input_boss` on the same arguments `step_live` uses, pre-tick, into `CompanionView.follow`, `FollowInput::default()` when `!ctx.can_move[i]`; the director starts from it, so the view stays plain data and the movement is still today's bot).
3. `combat_scene/donut.rs`: `may_start: (!bot || companion_driven) && can_act`.
4. Damage: `companion::rules::glance(spec, target_hp, ai_started, ca19_kill_policy) -> MoveSpec` at Carl's strike site and Donut's swat/pounce/bolt hit sites (CA-20). CF02-00 stubs the Carl site as an identity glance; the exact signatures are in [the CF02 work order](../07-agents/combat-fun-02-workorder.md#cf02-00-companion-stubs-exact).
5. Regroup, 1 HP guard, offscreen immunity stay in `corridor.rs`/`party.rs` unchanged until 01B.

**Flags** (CA-22). `cf02_rules` stays CF02's; the companion mode lives in `PartyTuning::uncontrolled_hero`.

| CLI | `cf02_rules` | `uncontrolled_hero` | Goldens |
| --- | --- | --- | --- |
| none | false | `FollowAndJumpBot` | every legacy golden byte-identical (inventory below) |
| `--cf02` | true | `Companion(Cf02Assist)` | `cf02_*` (assistant fixtures owned by 01A) |
| `--companion v0` | false | `Companion(CompanionV0)` | `companion_*` |
| `--cf02 --companion v0` | true | `Companion(CompanionV0)` | `cf02_companion_*` (after CF02-F) |
| `--cf02 --companion off` | true | `FollowAndJumpBot` | A/B of the arena without assistant; `cf02_parity_off` runs with `FollowAndJumpBot` (`--cf02 --companion off` once 01A-wire lands); 01A-wire keeps it green |

`ca19_kill_policy` is `never` in both profile sets through 01D; 01E tests `factual` as the candidate default for `companion_v0`; `cf02_assist` stays `never` (CF02-Q38) (CA-19).

**Golden inventory (byte-identical with the companion off):** 20 `EXPECTED_HASHES` (13 legacy, 2 DEATH-D, 5 llama), `GOLDEN_WALK_RIGHT_400`, the story hash `931868d6afa56498`, 7 party, 13 boss and 8 Q10 goldens, `demo01_*`, the CINE goldens, and the whole-route autopilot hash `0363078d58558e95`.

### (d) AUTOPLAY-02a dependency (01E)

No second harness. 01A-01D prove behaviour with fixtures and per-fixture event tallies that use the AP-Q14 attribution rules (instigator 0 Carl, 1 Donut, `None` unknown), so numbers carry over. 01E evaluates on AUTOPLAY-02a:

- AUTOPLAY-02a is owned by the AUTOPLAY lead. It is planned after AUTOPLAY stage B, is not part of AUTOPLAY v1, and runs in parallel with AUTOPLAY C to D. 01E depends on 02a, not on a date.
- The mode is `party_solo { seat: 0|1, companion_profile }`, driving `PartyFixtureSim::step_slots`. ActionV1 is reused with swap **reserved**, so the seat stays fixed.
- ObservationV2 adds an ally block that includes Donut's Q10 state and mana.
- Metrics come from AUTOPLAY. Kill credit by hero comes from `EnemyDefeated.instigator`, and companion damage from `Hit` `move_id`. The "assist" metric is AUTOPLAY's definition: a hit on the same enemy id within N ticks before another hero's kill. COMPANION does not define its own assist window. The value of N is **90 ticks**, set by AUTOPLAY AP-Q31.
- 02a code lives in `src/autoplay/party*`. It can start on today's uncontrolled behaviour and switch to `UncontrolledHero::Companion(profile)` once 01A-wire merges.
- Companion-only diagnostics (glances, `ai_indirect_kills`, orders, stuck ticks) still come from draining `CompanionEvent`s, offered to 02a as an extra diagnostics source.
- AUTOPLAY-02a is **no-orders only** (its action space has no command input; the held L1 command modifier is the reserved, stripped swap index), so it cannot serve 01E's ordered half or order metrics; see [AUTOPLAY spec section 11](autoplay-01-automated-gameplay-evaluation.md#11-autoplay-02a--solo-party-env-one-policy-seat-companion-active), conflicts 2 to 5. The seat is serialized by hero name (AP-Q22). All 01E arms run under one `cf02_rules` value, `false` until CF02 exposes the switch on the party fixture path, so 02a runs `cf02_assist` with `cf02_rules` off, a combination the flag table in (c) does not list.
- 01E then plays `FollowAndJumpBot`, `cf02_assist`, `companion_v0` with `never` and `companion_v0` with `factual` on the same seeds; the owner decides D-KILL (O2a) and D-AUTO (O2b) from that plus play.

### (e) CF02-DUO adopted defaults: AI Donut as a fighting partner

Owner game-feel feedback after the first CF02 playtest (2026-11-21): "I expected Donut attacking". The `cf02_assist` set had been tuned to a restrained assistant (never defeats, one action per second, 25 % hit ceiling, missile only to interrupt). That restraint was an adopted default, not an owner decision; the feedback overrides it. All values are data in `companion_ai_v0.json`; the owner may change any of them in a follow-up PR. The `companion_v0` set is unchanged (`never`, 60 ticks, 25 %, no offense), so every `companion_*` fixture hash is byte-identical. Game feel is pending the owner's play.

| ID | Knob (`cf02_assist`) | Old | New | Reason |
| --- | --- | --- | --- | --- |
| CF02-DUO-A01 | `ca19_kill_policy` (supersedes CF02-Q38's AI part; the profile validator no longer pins `cf02_assist` to `never`) | `never` | `factual` | owner playtest feedback 2026-11-21: AI Donut's hits are real and can finish an enemy; credit stays factual |
| CF02-DUO-A02 | `cf02_q16_action_cadence_ticks` (CF02-Q16) | 60 | 20 | owner playtest feedback 2026-11-21: about three actions per second, still one at a time and never during a running move |
| CF02-DUO-A03 | `cf02_q39_max_hit_share_percent` (CF02-Q39) | 25 | 50 | owner playtest feedback 2026-11-21: she carries up to half of the landed hits. The ceiling is a test assertion (`cf02_arena_direct`), not runtime behaviour |
| CF02-DUO-A04 | new `cf02_duo.cf02_duo_a04_offense` | (absent: interrupt and lying/staggered pounce only) | `true` | owner playtest feedback 2026-11-21: she also starts a swat (before the biscuit, on a standing enemy in swat reach), a pounce (on a standing goblin that is not in a tell) or a Magic Missile (after the biscuit) on her own. Off in `companion_v0` |
| CF02-DUO-A05 | new `cf02_duo.cf02_duo_a05_missile_min_px` / `_max_px` | (Q17: 97..160, interrupts only) | 64..160 | owner playtest feedback 2026-11-21: the offensive missile on a standing or staggered enemy. Never a free cast: only after the unlock and only if `DonutMana::ai_can_cast` (the reserve of 2 is kept), at the AI action cadence; the Q17 interrupt rule is unchanged |
| CF02-DUO-A06 | new `cf02_duo.cf02_duo_a06_pounce_min_px` / `_max_px` | (Q15: 30..80, lying or staggered goblin only) | 30..80 on a standing goblin too | owner playtest feedback 2026-11-21: the pounce is her damaging move before the biscuit |

Utilities: interrupt 2000 > pounce on a lying goblin 1500 > offensive missile 1300 > offensive pounce 1200 > strike 1000 > offensive swat 900 (integers, minus the distance). The offensive swat only starts on a standing enemy (a swat on a staggered enemy does nothing). Unchanged: Q13 AI Carl passive in `cf02_assist`, Q14 and Q17 interrupts, Q38 lying-hold, Q40 in-view, `ca26_ai_bailout`.

Evidence: `cf02_assist_donut_defeats_goblin` (no human input, before the biscuit, her pounce defeats the goblin) and `cf02_assist_donut_missiles_goblin` (biscuit unlocked, her missiles defeat it and keep the reserve of 2). Seven `cf02_assist_*` and three `cf02_arena_*` goldens moved on purpose (the PR body lists old and new); `cf02_arena_environment` now runs the follow-and-jump bot because the active assistant clears the goblins before they swing and trip the shore the fixture demonstrates. The pre-DUO scenarios of Q14, Q15, Q16, Q38 and Q39 are kept as regression tests under the `companion_v0` Donut (same numbers as the old `cf02_assist`). Known: a pounce on a standing goblin can whiff against a moving target; there is no vertical-reach check yet, so she can try to swat or cast at an enemy on another level.

## 18. Horizon fit

Consulted [the character progression horizon](../01-design/character-progression-horizon.md). The design still fits: it answers its open question on solo AI "without P2 spectatorship" (the companion never runs in 2P), adds no move, unlock or input, keeps Carl's grab and explosives out, keeps Donut's magic behind the biscuit and her AI mana reserve, and treats the command layer as an experiment (the horizon approves no companion command wheel). Mongo is not affected. No horizon update is needed.
