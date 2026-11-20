# ADR-0010: Combat-first progression without a conventional inventory

- **Status:** Accepted product direction (owner, 2026-11-20); implementation and numerical tuning pending playtest
- **Date:** 2026-11-20
- **Decision scope:** Game loop, rewards, progression, narrative items and sponsored gifts
- **Implementation owner:** REWARDS-01 (staged; not implemented by this ADR)
- **Related:** [REWARDS-01 design](../01-design/combat-rewards-and-no-inventory-v1.md), [roadmap](../04-production/rewards-roadmap.md), [agent handoff](../07-agents/rewards-01-workorder.md), [progression horizon](../01-design/character-progression-horizon.md), [MVP controls](../01-design/mvp-controller-contract.md)

## Context

THE CRAWL is a combat-first, physical, environmental and cooperative 2D action campaign. It aims for a small, reliable verb set and expressive encounters. A traditional equipment/loot progression system would introduce inventory management, gear stat comparisons, art combinations, multiple UI surfaces, equipment balancing, progression exploits and split attention in local co-op. The owner wants the combat-focused simplicity of Hades-like stage boons **without** converting the game into a full roguelite or erasing narrative equipment that makes characters distinct.

A companion per hero, viewer counters, special cinematic finishes and combat-performance scoring already have independent responsibilities. A new reward system must not make the HUD a simulation authority, create a second defeat source or provide a free permanent scaling loop.

## Decision

1. **No conventional inventory RPG.** No bag/weight system, common-to-legendary gear treadmill, random weapon/armor comparison, material crafting, vendor loop, consumable hotbar, permanent grind currency or mandatory equipment screen. The Pause/Menu button remains; its historical 'inventory' label is not a product requirement.
2. **Permanent abilities are mainly story-earned.** Canon-verified or human-approved exceptional objects are auto-integrated into character capability flags and existing combat verbs; they are not equip-slot instances. Preserve original-IP-neutral IDs.
3. **Four explicit reward lifetimes:** (a) hero-owned short combat momentum, (b) team-owned capped floor modifiers selected at safe stage transitions, (c) future story-gated sponsor gifts, (d) authored permanent campaign capabilities. Only curated sponsor gifts may exceptionally become permanent; never random generic stat loot.
4. **Reward skill, not a mere death animation.** Deterministic source combat facts drive an opt-in spectacle evaluator. Actual variety, precision, meaningful enemy/prop impacts, environmental causality and verified teamwork qualify; blood count, cinematic camera speed and displayed viewer integers do not.
5. **Audience is not a gameplay wallet.** AUDIENCE-01 remains an observer. Sponsorship requires the broadcast story unlock and deterministic eligibility/narrative opportunities, not an event fired by the displayed HUD. If exact numeric viewership gates are eventually required, introduce a canonical domain projection and parity-migrate the display before applying rewards.
6. **Minimum controls / UI.** Preserve approved four face-button verbs, one equipped special slot per hero, no additional active-combat controls and no permanent style meter. Safe-stage CRT cards are a future UI, not a new gear system. Default Floor 01 opening stays without viewers and sponsors.
7. **Fair co-op and solos.** Actor identity is stable across human-vs-AI control and pad swaps. A hero can own momentum; stage modifiers belong to the team. A shared action never duplicates payout for 2P. AI-only play cannot farm an unlimited reward loop.
8. **Deterministic progression and failure.** Story, stage, floor, encounter and attempt lifetimes are distinct. Idempotent grant receipts and checkpoint snapshots prevent death/replay duplication. Reward-enabled changes live behind opt-in flags and have own versioned replay hashes. Old demo baseline is unaffected with flag off.
9. **Build the smallest value first.** Evaluate two small combat momentum effects and A/B skill outcomes before stage cards. Stage choices require authoritative stage/floor boundaries. Sponsors require unlock, source-owned audience eligibility and curated story context; lasting objects also require campaign persistence.
10. **Balance is reversible.** Duration, score weights, thresholds, 2-modifier cap, 3-card offer, selected item examples, co-op chooser method and content catalogue are hypotheses requiring automated metrics and the owner's controller playtest, not final shipping commitments.

## Alternatives considered

| Alternative | Decision | Rationale |
| --- | --- | --- |
| Full loot and equipment RPG | Rejected | Distracts from combat, disproportionate integration/asset/UI cost, difficult to coordinate in same-screen co-op |
| Pure Hades-style roguelite with all progress reset on death | Rejected | Conflicts with authored story and persistent earned capabilities |
| No upgrades of any kind | Rejected | Misses opportunities to reward expressive combat, increase player motivation and introduce tactical choices between stages |
| All bonuses permanently stacked from high combo scores | Rejected | Snowballing undermines difficulty and encourages score grinding |
| Use displayed viewer count directly to award gameplay buffs | Rejected | HUD observer must not become authoritative; viewers include narrative anticipation and embarrassing failures |
| Curated multi-lifetime rewards with no inventory | Accepted | Preserves narrative signature items, makes combat the primary interface and confines balance/UI scope |

## Consequences

**Positive:** small gameplay surface, low controller complexity, less art/content multiplication, fewer inventory bugs, consistent single-player/co-op pacing, clear story unlocks and testable combat-first reward loops.

**Negative:** fewer collecting/building choices; some franchise-related equipment fantasy must be represented as authored capability or cosmetic transformation; stage/floor lifecycle and campaign save become explicit dependencies for later features. A small card system still needs deliberate balance, UX, data and persistence work.

**Risk controls:** feature flags, hard limits, deterministic idempotent receipts, anti-farming, unknown-credit fail-closed policy, no loot granted before narrative unlock, no separate inventory UI, real-pad playtests and replay/held-out AUTOPLAY checks.

## Compatibility and supersession

This ADR **narrows** the non-binding [character progression horizon](../01-design/character-progression-horizon.md) where it suggests inventory/equippable gear; it does not supersede accepted story capability order or the [MVP special-slot grammar](../01-design/mvp-controller-contract.md). Existing names like 'inventory' in older controller descriptions refer only to menu access until textual cleanup. A future introduction of equipment management requires a new ADR based on evidence of a concrete missing combat decision. Existing HUD, CINE, companion and observer contracts retain authority over their own domains.

## Rollout

Follow [the dependency roadmap](../04-production/rewards-roadmap.md): M0 policy; M1 moment-to-moment momentum; M2 bounded floor choices; M3 sponsor beats; M4 exceptional lasting story gifts. This ADR alone unlocks no item, skill, sponsor event or new HUD element. Franchise-based visual/story content remains gated by rights and book chapter verification.
