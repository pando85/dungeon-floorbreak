# ADR-0005: Local cooperative play before online multiplayer

- **Status:** Proposed target
- **Date:** 2026-11-15

## Context

Gamepad/controller play is central; Carl and Donut have complementary abilities. Networking complexity could delay gameplay validation, and the game benefits from shared-comedy moments on one screen.

## Decision

Build solo + two-player same-screen local co-op first, with normalized input and explicit player slots. In solo, prototype direct switching and AI companion variants; evaluate via gamepad playtest. Reserve third-character Mongo for a later milestone (initially allied AI/ability). Consider Steam Remote Play Together for remote access to local co-op when product and Steam setup are ready; do not implement netcode in v1.

## Consequences

Shared camera and co-op reachability demand early testing. Drop-in/out, third-player support, peer networking and rollback are not promised. Tests must cover two independent controllers, reconnect, camera separation and synchronized destruction state.
