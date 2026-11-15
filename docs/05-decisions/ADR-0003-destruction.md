# ADR-0003: Selective tile/material destruction over per-pixel simulation

- **Status:** Proposed; spike validates collision implementation
- **Date:** 2026-11-15

## Context

Destruction must matter to pathing/combat but need not match Broforce's total destructibility or Noita's granular materials. Every-grid-pixel physics would increase CPU/GPU cost, visual cleanup and unwinnable state risk.

## Decision

Author material cells (16×16 provisional), apply event-based damage, update dirty tile regions and batched collision chunks. Dynamic physical objects use Avian2D when appropriate; debris are mostly cosmetic. Include critical invulnerable geometry and future authored supports instead of continuous generalized structural physics.

## Alternatives

- Decorative-only breakables: cheap but fails pathmaking pillar.
- Arbitrary destructible pixels: high freedom; high simulation and level-design cost.
- Pre-scripted break points only: robust but potentially feels fake and noninteractive.

## Required proof

One bomb breaches an alternate route while another material remains intact; collision matches visual terrain; a controlled scene preserves reachability; chunk seams test passes and frame-time spike measured.
