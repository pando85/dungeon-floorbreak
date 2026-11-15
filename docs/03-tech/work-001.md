# WORK-001 — Linux Bevy foundation and CI evidence

**Implementation status:** proposed in an unmerged pull request until CI proves otherwise.

## What exists in this spike

- A single Rust 2024 crate using Bevy 0.20.1 with a native Linux X11 window.
- A fixed 640×360 logical camera displayed in an initial 1280×720 window (2×). Nearest sampling is enabled.
- IP-neutral colored placeholder character and terrain. No licensed character artwork.
- One gamepad (first connected) or keyboard translated into RawInput -> Actions -> InputFrame with button edges, stick deadzone and finite/clamped axes.
- Movement-only visual test scene. Jump, attack, ability, pause, controller pairing and character switching are represented by action data, not implemented mechanics.
- Headless ECS execution and automated rendered-window smoke checks in GitHub Actions.

## Run locally

Run these commands from the repository root:

    cargo run
    cargo test --all-targets
    cargo run -- --headless-smoke

After Cargo.lock is committed, use --locked for reproducible builds.

Move with A/D, arrows, left stick or D-pad. Jump action is Space/South, attack is J/West, ability K/East, pause Escape/Start. Only movement affects the demo now.

## Required validation before merge

- cargo fmt --all -- --check
- cargo clippy --all-targets -- -D warnings
- cargo test --all-targets
- cargo build --bin dungeon-floorbreak
- ./target/debug/dungeon-floorbreak --headless-smoke
- Xvfb + software Vulkan screenshot at 1280×720 with nonblank frame.
- python3 scripts/check_docs.py

The screenshot and log artifacts do not prove real controller compatibility or actual Steam Deck frame rates. Physical controller playtesting and human review remain separate gates.

## Follow-ups

- Commit CI-generated Cargo.lock before merge.
- Implement resize behavior and true integer-scaled letterboxing for variable resolutions.
- Implement gamepad assignment, hotplug, replay, and 2-player ownership in WORK-005.
- Implement jumping, gravity, buffering and coyote time in WORK-002.
