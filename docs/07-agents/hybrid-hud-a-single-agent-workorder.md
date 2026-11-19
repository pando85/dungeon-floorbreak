# HUD-HYBRID-A-01 — one local agent implementation workorder

> **Note (2026-11-19):** the owner authorized orchestrated multi-agent execution of this workorder under `docs/07-agents/agent-orchestration.md` and the `AGENTS.md` art routing (top-tier design, medium-tier execution, independent top-tier review). This supersedes the single-agent / no-subagent instruction below. The PR is **not auto-merged**: the owner gives the verdict on the in-engine look. The rest of this workorder stands.

**Goal:** extend the existing integrated Floor 01 Bevy experience with the **owner-approved hybrid A HUD**. One agent owns all code, UI art production, tests, screenshots and PR updates; do not spin up subagents unless the owner separately requests them.

## Start
1. Checkout existing PR branch `feat/hybrid-hud-a-floor01` against latest `origin/main` carefully (avoid discarding commits). Read `AGENTS.md`, `docs/02-art/hybrid-hud-a-design.md`, `docs/01-design/mvp-controller-contract.md`, `docs/03-tech/floor01-scene.md`, `docs/03-tech/input-camera.md`, and `docs/02-art/asset-pipeline.md`.
2. The four reference PNGs are already committed bit-exact under `assets/references/hud/hybrid-a-v1/` as **reference-only** with provenance; re-verify the SHA-256 manifest before relying on them.
3. Inspect actual main/branch scene code before changing interfaces. Keep source of truth for health, progression, camera, event ownership, and replay in the game domain; use presentation adapters.
4. Implement one reusable UI layer in `floor01_scene`: Carl upper left, Donut upper right, small location strip, combat notifications with finite lifetime, optional boss strip and P1/P2 states. Make preview-only fixtures for missing live states, not independent demos.
5. Add Donut mana bar, thinner than HP, beneath HP and above action row, violet/cyan, **post-unlock only**. No Carl mana. Pre-biscuit Donut has no mana bar. All numbers in supplied PNGs are illustrative, NOT balancing decisions.
6. Correct mockup inaccuracies: **Square/X primary, Triangle/Y physical (Carl kick), Circle/B story-unlocked special, R1/RB interact, L1/LB solo swap**. Do not implement LT punch / RT dodge / LB spells / RB cat step or invented boss/combo mechanics.
7. Create editable HUD assets if needed; do **not** use a concept screenshot as UI texture. Record sources, derivation, SHA and export reproducibility.
8. Add tests and a deterministic capture recipe (pre-unlock, post-unlock full mana, combat depleted mana, boss preview, co-op preview, 640×360 / 640×400, zoom-out). Run repo checks and capture **actual Bevy** screenshots. For unsupported states label preview.
9. Update relevant docs and backlog, then push **this branch**, update the **same PR**, and report real tests, evidence, open design choices, runtime limitations and blockers. Do not merge before independent review / owner visual acceptance.

## Strict acceptance
- Legible at 1× virtual and Deck display, does not cover actors or hazard tells.
- Mana appears only for Donut when magic is genuinely available or in explicitly labeled post-unlock preview.
- UI is independent of world camera zoom; gamepad navigation remains intact.
- No phantom resources, fake co-op ownership or changes to combat rules.
- No per-frame allocations that cause avoidable frame-time spikes; measure and document, don't guess.
- Do not claim design mockups are native pixel art or canonical character sprites.

**Deliverable:** updated PR with code, editable UI source, staged reference art, docs, reproducible tests and four real engine evidence frames (or documented blockers). End with a short concise report; human plays to accept look/feel.
