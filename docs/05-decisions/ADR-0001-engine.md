# ADR-0001: Rust + Bevy as provisional engine

- **Status:** Provisional / requires implementation spike
- **Date:** 2026-11-15

## Context

This game is written and animated almost entirely by AI agents, supervised by human gameplay/art review. Linux and Steam Deck are the first platforms; future Steam distribution is envisioned. We value CLI-first edits, Rust type checking, deterministic headless tests and data-driven levels. A custom engine would divert effort from game feel and destruction.

## Decision

Prototype on **Bevy 0.20.x + Rust**. Keep game logic in testable modules/crates and presentation in Bevy adapters. Use Aseprite and LDtk as external art/map editors; external tools do not change the runtime language choice.

## Alternatives considered

1. **Godot 4 + GDScript:** mature editor, short iteration loop, better out-of-box platformer tooling, fully scriptable/headless workflows. Strong contender if Bevy integration cost is high.
2. **Godot + Rust GDExtension:** editor + native modules, but dual language, FFI and ownership complexity for AI-maintained workflow.
3. **Bevy:** best unified language/test infrastructure; less editor tooling ready out of box.
4. **Custom SDL3/wgpu/physics stack:** maximum control, unacceptable tooling/integration risk for early slice.

## Consequences

+ One cohesive Rust workflow; cargo checks, testable domain, data-driven plugin ecosystem.
- Animation/debug/editor ergonomics need explicit tooling, compile times may slow agents.

## Re-evaluation gate

After WORK-001..005, compare change-to-test time and visual/terrain development friction against a small Godot prototype. Keep Bevy if controller input, art workflow and destruction are productive; switch engine if integration work routinely overwhelms features. This decision is not irreversible before assets expand.

## Sources

Bevy: https://bevy.org/news/bevy-0-20/ ; ecosystem versions: [sources](../06-research/sources.md).
