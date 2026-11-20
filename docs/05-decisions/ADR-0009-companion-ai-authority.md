# ADR-0009: Symmetric companion AI authority and intent-based commands

- **Status:** Proposed (design-only PR; no runtime behavior adopted by this file)
- **Date:** 2026-11-20
- **Scope:** Floor 01 solo companion behavior and future command grammar; preserves [ADR-0005](ADR-0005-coop.md) and the [approved controller contract](../01-design/mvp-controller-contract.md).
- **Detailed specification:** [Companion autonomy and command system](../03-tech/companion-ai-command-system.md).
- **Local agent delivery:** [COMPANION-AI-01 work order](../07-agents/companion-ai-01-workorder.md).

## Context

The existing solo prototype permits switching between Carl and Donut. A modest deterministic follow-and-jump bot drives the inactive actor, avoiding some enemy tells, but it never attacks or navigates authored platform graphs. The intended product, however, is combat-first and relies on two asymmetric protagonists supporting one another. Merely adding commands to an ineffective follower would turn normal play into babysitting. Separate AI code for Carl and Donut would duplicate authority and make behavior diverge.

The user should control one hero directly, while the other is autonomously useful. Commands should express tactical intent without obliging a second stick, permanent menu, strategic pause, or cancellation of meaningful direct combat input. The local two-player mode must never cede a human-owned hero to AI.

## Proposed decision

1. Define **one Bevy-free companion director**, applicable to the currently uncontrolled solo hero, with character-specific capability and utility profiles. All AI actions enter the same authoritative movement, attack, resource and collision systems as human actions.
2. Evaluate safety, input ownership, progression unlocks, real reachability and committed move rules **before** carrying out orders. Use a small deterministic top-level state machine plus weighted legal action selection rather than introducing a full third-party behavior framework or runtime model.
3. Supply useful autonomous defense, positioning, traversal and limited attacks **without any commands**. Optional commands express a small set of intentions: Focus, Hold, Regroup, and Assist / authored ContextAct, with no required multi-step scripts.
4. Treat order authoring and gamepad bindings as **separate contracts**. A held-L1 command modifier is an experiment only: the current L1 press-edge swap must stay untouched until gamepad playtests prove the alternative preserves combat feel. R1 interact, D-pad locomotion and reserved future triggers keep their approved roles.
5. Implement capability-aware 2D platform navigation, invalidation after destruction and safe refusal when a goal is unreachable. Existing offscreen regroup must not bypass a story-critical character gate.
6. Keep orders and target identifiers in deterministic domain state, with explicit issued/accepted/deferred/executing/completed/rejected/cancelled events. Rendering, audio and expressive barks observe the results and never alter game authority.
7. On a solo swap, cancel the old inactive-hero order and transfer autonomous responsibility to the newly uncontrolled actor. Previously committed legal moves finish. On 2P join, disable all companion AI orders; on controller disconnect, preserve the existing pause rule.
8. Preserve actual combat attribution: AI-generated kills and individual viewers belong to their real actor; anti-finish-stealing is a soft preference, not invented invulnerability or credit transfer. Donut AI observes its existing minimum post-spend mana reserve and Carl AI does not autonomously use dangerous explosive tools in v0.
9. Build each subsystem as a small PR with deterministic fixtures and human-visible evidence. Integrate with AUTOPLAY-01 only after its interface exists, without extending its intentional Carl-only v1 scope.

## Alternatives considered

- **Scripted follower only:** cheapest but insufficient combat support and impossible navigation at asymmetric story gates.
- **Commands as required routine controls:** makes the player micromanage a second actor and conflicts with combat-first immediacy.
- **Separate Carl-AI and Donut-AI implementations:** duplicates state and drifts in command semantics, safety and tests.
- **Unrestricted planner, learned policy or LLM in the runtime:** introduces nondeterminism, unclear authority, runtime budget and debugging complexity before there is a trustworthy baseline.
- **A comprehensive tactical wheel:** expands the input/UI burden at 640×360 and can obscure critical tells. A minimal command layer can be expanded only after measured need.

## Consequences and migration

The near-term implementation has more work than simply binding four buttons: perception, legal movement/action selection, 2D navigation, order validation, feedback, stable IDs and replay tests are all necessary. A profile-driven director reduces future duplication and makes automated evaluation practical. Both AI roles should be testable with no orders before binding commands.

The design is **reversible**: preserve current follower mode as a feature-flagged baseline; keep controller command grammar configurable until real-pad validation; do not replace existing party, combat or HUD systems. Unresolved tuning and player feel must be measured, with status recorded as unapproved hypotheses rather than asserted gameplay success.

**No code or control mapping is implemented by this ADR.**
