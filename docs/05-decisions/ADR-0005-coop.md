# ADR-0005: Local cooperative play before online multiplayer

- **Status:** Proposed target. **Implementation note (2026-11-19, PR for WORK-005, WORK-006, HUD-DEBT-01, HUD-DEBT-04):** local two-player (P1 Carl, P2 Donut, Start joins P2, any pad disconnect pauses), the solo swap with the follower bot, Donut's damage with party defeat and the shared camera are implemented as a deterministic, replay-tested prototype. The ADR itself stays a proposed target: the values are adopted without a playtest (game feel pending, see [party-2p-open-questions](../01-design/party-2p-open-questions.md)), real pads are not CI-verified, and drop-in/out, a third player and netcode are still not promised.
- **Date:** 2026-11-15

## Context

Gamepad/controller play is central; Carl and Donut have complementary abilities. Networking complexity could delay gameplay validation, and the game benefits from shared-comedy moments on one screen.

## Decision

Build solo + two-player same-screen local co-op first, with normalized input and explicit player slots. In solo, prototype direct switching and AI companion variants; evaluate via gamepad playtest. Reserve third-character Mongo for a later milestone (initially allied AI/ability). Consider Steam Remote Play Together for remote access to local co-op when product and Steam setup are ready; do not implement netcode in v1.

## Consequences

Shared camera and co-op reachability demand early testing. Drop-in/out, third-player support, peer networking and rollback are not promised. Tests must cover two independent controllers, reconnect, camera separation and synchronized destruction state.
