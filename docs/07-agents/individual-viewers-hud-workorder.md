# AUDIENCE-01 — Agent work order: individual viewer counters

**Status:** implementation/evidence work order for the **draft** AUDIENCE-01 PR. Owner approved the compact per-character direction, not final gameplay tuning. Read \`AGENTS.md\`, \`docs/07-agents/chatgpt-hermes-image-pr.md\`, \`docs/02-art/individual-viewers-hud-v1.md\`, \`docs/02-art/hybrid-hud-a-native-spec.md\`, \`docs/03-tech/floor01-scene.md\`, \`docs/02-art/asset-pipeline.md\`, \`docs/01-design/floor-01-gameplay-progression-v0.md\`.

## Intake (Hermes: same PR, no second PR)

1. Confirm the PR's actual head branch and checkout that exact branch. Do not create another PR, commit to main, or overwrite recent work.
2. The owner provides a **PR-specific ZIP** with \`README-ES.md\`, \`hermes-task.md\`, \`handoff.json\`, five *reference-only* transparent PNG overlays at 640x360 and an editable **candidate key** (\`pixel-grid-v1\`) for eye/up/down. The ZIP is not itself in GitHub. Verify each file's actual SHA-256 and PNG dimensions before copying; mark missing binaries pending rather than forging files or hashes.
3. Put original reference PNGs under \`assets/references/hud/viewers-v1/\` following the exact mapping in \`handoff.json\` and their byte identities. Copy bytes without resaving. The **candidate key** is not a production asset: medium-tier art execution must validate/re-author it against the project's pixel-grid lint/export/atlas conventions before placing it in \`assets/source/hud/\`.
4. Add \`manifest.json\`, \`SHA256SUMS\` and amend the directory README with the verified file sizes, hashes, original names, PNG dimensions and provenance, matching existing HUD reference conventions. Update \`docs/04-production/backlog.md\` with AUDIENCE-01 intake + actual implementation/evidence state. Keep branch/PR as draft while reference PNGs or code/evidence are pending.
5. The baseline capture to inspect is \`docs/evidence/hud-hybrid-a/hud-REAL-normal-640x360-1x.png\`. **Never** substitute a drawn alternate game scene or the image-generator concept sheets for the real Bevy HUD.

## Implementation split (recommended)

- **Top-tier design review:** confirm in-plate x/y fit against current actual atlas names and DONUT glyphs; ensure no overlap with hit points, post-biscuit mana or boss strip. Resolve only genuine design defects and document decisions.
- **Medium-tier execution A (UI art):** convert/validate original eye, up and down pixel keys into the current HUD atlas generator and manifest with reproducible export. Do not paste references as a HUD texture. No new dependency or camera.
- **Medium-tier execution B (domain):** Bevy-free \`audience\` state, authored per-hero events, unique-event awards, decay, checkpoint restore and fixed-tick tests. Preserve control ownership independence (solo AI/2P).
- **Medium-tier execution C (HUD integration):** \`PlateView\` optional audience, adapters/draw-list, locked/unlocked preview, 1x/Deck/high-contrast/reduced-motion captures, overlap and formatting tests. Changes to runtime require real checks; no screenshot-only implementation.
- **Independent top-tier review + medium-tier fix check:** inspect short spec, max three evidence images and test summary; report verdict. Merge only under \`AGENTS.md\` delegated review/green-CI policy, with game-feel review pending.

## Fixed design constraints

- Carl and Donut each own their own count, never one global count.
- No counter before an actual later narrative unlock; no offline/zero placeholder; the unlock is **not** automatically the mana/biscuit unlock.
- Live HUD uses **one tiny inline readout per existing hero plate**, not the enlarged HUD panels, the center gap or the stage-end scorecard. Eye + compact value + optional trend.
- Anticipation can create an unexplained spike before danger appears; do not add explanatory danger telemetry.
- Existing Hybrid-A plate dimensions, health/mana, ability keys, location/boss strip, System face and safe action lane are inviolable.
- Domain logic deterministic, no presentation-to-gameplay feedback, anti-farming, author events not generated from an invisible-enemy scanner.
- Large stage-end results, subscribers and sponsors are out of scope.

## Acceptance and handoff report

After intake: \`python3 scripts/check_docs.py\`; for new sourced art: \`python3 scripts/validate_assets.py\`, generator \`--check\`, source lint as applicable; for code: \`cargo fmt --all -- --check\`, \`cargo clippy --locked --all-targets -- -D warnings\`, \`cargo test --locked --all-targets\`, render smoke in real Bevy at 640x360 and 640x400. Capture locked, stable, spectacular, declining, anticipation, boss and post-mana unlocked states, clearly labelling **PREVIEW** if unsupported by actual gameplay.

Report back on the **same PR**: head SHA, files imported, exact hashes, tests actually executed, CI status, real-vs-preview evidence paths, game-feel gaps and independent review verdict. Do not pretend the ZIP's reference PNGs demonstrate a running game.
