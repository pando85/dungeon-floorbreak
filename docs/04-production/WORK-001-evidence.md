# WORK-001 — Implementation evidence

**Status:** implementation proposed in a draft pull request. Do not mark accepted until CI and Linux/manual evidence are captured.

## Scope

- Rust 2024, Bevy pinned to 0.20.1 (version confirmed against the released crate).
- Original geometric 2D test scene, 480×270 world-pixel view and nearest-neighbor sampling; 960×540 initial window (2×).
- Fixed-tick keyboard/gamepad adapter normalized into a Bevy-independent action snapshot.
- Synthetic, headless replay digest and Rust unit tests.
- GitHub Actions gate for Linux compilation, formatting, Clippy, unit tests and CLI replay.

## Commands / evidence checklist

- [ ] `cargo fmt --all -- --check` — CI result required
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — CI result required
- [ ] `cargo test --workspace` — CI result required
- [ ] `cargo run -- --headless-action-test` — CI result required
- [ ] `cargo run` — manual Linux window/screenshot required
- [ ] Physical controller connect/disconnect — manual Linux test required
- [ ] Pixel scaling at 960×540, 1920×1080 and 3840×2160 — human review required
- [ ] Commit generated `Cargo.lock` after CI generates it

## Known limitations

- No player movement, combat, terrain, co-op, or game art.
- Fixed-vertical camera is integer-scaled for 16:9 multiples of 480×270; unusual aspect ratios and 1280×800 letterboxing need follow-up before pixel scaling is accepted.
- Gamepad device ownership is intentionally single-slot and not persistent across multiple devices.
- Button edge capture is sampled in FixedUpdate; timing when render and fixed ticks diverge needs scenario validation in a later work order.
- No screenshot or hardware test was performed by the authoring assistant. The local execution environment does not include Rust/Cargo or a Linux graphics session.

## Provenance

Geometric rectangles only, no franchise assets or copyrighted sprite content. This document is the evidence ledger; fill in actual CI and manual results rather than inferred success.
