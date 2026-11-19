# Test strategy — reproducible AI-only development

## Hierarchy

1. **Unit:** materials, damage, inventory, cooldowns, actions, movement math, reachability.
2. **Deterministic integration:** fixture level + seed + timestamped normalized input commands -> state/grid hashes + events.
3. **Headless simulation:** run fixed ticks without window/audio; assert no panic, no softlock and bounded resource counts.
4. **Rendered visual:** same seed/camera/test scene -> screenshot, atlas-contact-sheet and reference comparison. Note that pixel diffs need tolerances and review; strict GPU bit-for-bit reproducibility is not assumed.
5. **Human feel:** gamepad on Linux/Steam Deck, movement, coop camera, readability, humor; cannot be replaced by unit tests.

## Scenario format

See `examples/scenario.example.json` for proposed fields. Every gameplay PR affecting a mechanic should include a new fixture or update an existing one. The scenario runner is **not implemented yet**. Agents must not claim it exists.

## Determinism contract

Fixed `tick_rate_hz`, explicit RNG seed, stable iteration of damage events, no particle-generated gameplay damage, no wall-clock decisions. Record game version, content version, scenario ID, state hashes, and failed assertions. Use event IDs to deduplicate repeated explosion triggers.

## Performance methodology

Measure on development Linux and a Steam Deck-class machine (device name and build profile in report); capture mean, p95/p99 frame time, spikes during terrain rebuild, memory and number of colliders. Frame-rate target 60, not evidence. Always compare the previous known-good build using the same fixture/map/seed.

The floor 01 scene has a tick-locked probe (`floor01_scene --fixture <name> --perf-from-tick <tick> --perf-frames <n>`, driven by `scripts/measure_death_e1.py`) that prints GPU pass time, CPU time of the fixture tick, resident memory and entity counts, so a run covers the same ticks whatever the frame time. Under Xvfb the wall-clock frame interval is bound by the virtual display and only shows missed frames; see [DEATH-E1](../evidence/death-e/README.md) for what it does and does not show.

## Visual CI

Run CLI export, asset contracts, spritesheet contact sheets, optional xvfb/virtual-display gameplay capture and screenshot analysis. Upload CI artifacts for PR review. If no renderer available in CI, run headless tests and mark the render job skipped with an explicit reason. Human review is required for any player-visible quality-of-feel change.

## Acceptance and evidence template

```text
Work order: WORK-###
Commit / branch:
Platform / GPU / resolution:
Tool versions:
Commands run:
Unit/integration results:
Replay seed / fixture:
Visual evidence path or URL:
Performance deltas (if relevant):
Known issues:
Human playtest requested? yes/no
```

## Minimum checks before any code merges

`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, asset validation, map/scenario validation, generated artifact preview. Until game crates exist, repository CI intentionally runs **documentation-only validation**.
