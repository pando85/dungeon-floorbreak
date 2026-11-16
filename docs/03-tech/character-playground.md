# THE CRAWL — Character Playground (Bevy)

**Status:** functional prototype for visual and controller review; not human-approved animations or gameplay.

This scene is an **independent Bevy binary**, deliberately separate from Floor 1's combat greybox (PR #21). It reuses the existing LibreSprite/Aseprite export chain and the 2026-11-16 **approved native opaque heights**: Carl 52 px, Princess Donut 40 px, adult Mongo 76 px. The transparent atlas frames and pivot coordinates have different dimensions; no image scaling is applied to compensate.

## Launch on Linux

Install the project's art CLI and Python/Pillow prerequisites as described in [asset-pipeline.md](../02-art/asset-pipeline.md), then run from the repository root:

```sh
bash scripts/export_assets.sh
python3 scripts/check_playground_contract.py
cargo run --locked --bin character_playground
# Steam Deck-like 16:10 layout: 640x400 logical, 1280x800 window
cargo run --locked --bin character_playground -- --deck
# No GPU or generated assets required for domain sanity checks
cargo run --locked --bin character_playground -- --headless-playground-smoke
```

The image loading base path is `generated/art`; exported `*.png` files are **not committed**. Missing atlases are an export/setup failure, never silently replaced with placeholder characters.

## Interaction

- **1/2/3** select Carl/Donut/Mongo; **Tab** cycles selection.
- **A/D** or arrow keys move the selected character; **Space** jumps; **R** returns the cast to its starting positions.
- **Gamepad:** left stick/D-pad to move, South to jump, left/right triggers cycle selection, North resets. The first available gamepad owns this *single-player laboratory scene*; device-slot ownership and local co-op are outside the experiment.
- Idle is a four-frame loop for every actor. **Moving and jumping still display idle frames** because lateral run/jump/charge art has not been approved or created. Do not interpret motion as finished character animation.
- Feet anchor to source manifest pivots. The camera is fixed at **640×360** or **640×400** logical pixels, output 2× with nearest filtering; sprites stay **1:1** in virtual coordinates. The rest of the playable Floor 1 systems remain separate.

## CI and review gates

`.github/workflows/character-playground.yml` regenerates the three sprites from their original `.ase` and `pixel-grid-v1` sources, checks manifest sizes/pivots and four-frame row-major layouts, executes Cargo fmt/clippy/tests and a headless Bevy smoke, and captures actual Bevy screenshots at **1280×720** and **1280×800** using software Vulkan/Xvfb.

Evidence is the `the-crawl-character-playground` Actions artifact, including screenshots and loading logs. Pass/fail of CI validates runtime integration, **not** final pixel-art quality, camera readability, platform-specific controller feel or animation signoff. Human review is required for native-size silhouette clarity, Donut vs. Mongo occlusion, relative character size, and Steam Deck readability.

## Known limits / follow-ups

1. Replace idle-as-movement with approved left/right locomotion and jump sprite sequences, then validate animations on the *same* pivot.
2. Prototype Donut mounted on Mongo via an explicit seat socket (do not attach at sprite center).
3. Exercise overlap, depth order, selective destruction VFX, and optional diagnostic hitboxes; do not infer gameplay collision boxes from full alpha bounding rectangles.
4. Maintain separation from the Floor 1 greybox until both experiments have human review.

**Rights:** character source assets are public fan-concept art subject to [the rights register](../04-production/rights-and-release.md); their inclusion here does **not** establish rights to release or redistribute a playable game.
