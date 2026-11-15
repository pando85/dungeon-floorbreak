# ADR-0002: Detailed dark pixel art with selective modular animation

- **Status:** Direction accepted; implementation details provisional
- **Date:** 2026-11-15

## Context

The game needs much more expressive, larger and darker character design than tiny Broforce-like sprites. AI-only asset production creates risk of frame inconsistency, palette drift and unmanageable redraw work. Mewgenics-inspired modular expressions are attractive, but full rigged pixel animation may shimmer or feel stiff.

## Decision

Use **48–56 px class character height** for the primary human-scale figure (provisional), **16 px terrain grid**, near-neighbor pixel art, dark fantasy environment with strong contrast. Primary locomotion and combat motion use curated frame-based sprites; expression, face, ears/tail/accessories can be layered. Aseprite `.aseprite` source + Lua scripts + reproducible CLI export + atlas metadata are the preferred asset pipeline.

## Alternatives

- Tiny retro pixel art: low detail/quick content but weaker expressions and room readability.
- SVG or vector rigging for all characters: editable and fluid but may visually clash with pixel terrain.
- 3D-to-2D prerender: potentially stable frames but adds modeling/rig/render integration.

## Consequences and tests

More art effort per character. Mitigate with canonical master, palette limits, tagged animations, contact sheets, preview GIFs, pivot and silhouette checks, and human approval. Aseprite is not 'AI image generation'; agents need robust scripts and a carefully chosen image workflow.
