# THE CRAWL — Floor 01 Camera & Encounter Laboratory

**Status:** Creative design and browser greybox, **not** a production-ready level or Rust/Bevy playtest.

## What this package contains

- `index.html`: a self-contained, offline interactive browser prototype with six test rooms, a variable zoom camera, 16:9 / 16:10 layouts, schematic actor movement, basic contact attacks, destructible stand-ins, player switching, 2-keyboard-player controls and a mini map showing the current viewport.
- `scenarios.json`: versioned room definitions with world-space geometry and per-room test goals. Geometry and roles are **proposals**, not accepted canonical game data.
- `BEVY_HANDOFF.md`: the experiment specification and the requirements for a proper production-engine validation.
- `preview_room01.png`, `preview_boss.png`, `preview_mobile.png`: screenshots of the HTML lab; they are **not** in-engine acceptance evidence.

## Open the lab

Open `index.html` in a modern browser. It includes all room definitions directly, so there is no server, internet, or build system requirement.

On mobile, use the directional and attack touch buttons below the viewport. Switch between Carl and Donut with **SWITCH**. To stress shared camera bounds, tap **Split duo**. On desktop, use the following test-only controls:

| Mode | Move | Jump | Normal hit | Physical hit | Switch |
|---|---|---|---|---|---|
| Solo (active actor) | Left/Right arrows | Up | J | K | Tab |
| 2P Carl | A / D | W | F | G | n/a |
| 2P Donut | Left/Right arrows | Up | M | N | n/a |

**Important:** These browser keyboard bindings are only for this prototype. The actual game must preserve the existing owner-approved controller contract, including Square/X for primary, Triangle/Y for physical, L1/LB for solo switch and story-gated specials.

## Six room experiments

| ID | Room | Design question | Primary camera challenge |
|---|---|---|---|
| ROOM-01 | The First Rematch | Direct kill vs environmental kill | How much advance warning can a wider view provide without making melee tiny? |
| ROOM-02 | Hunters From Above | Bad Llamas, threat angles and Donut elevation | Can a wide view show ground and upper telegraphs simultaneously? |
| ROOM-03 | Break A Breach | Workshop, breakable wall and falling support | Can the player see *cause → effect* of destructive kills? |
| ROOM-04 | The Hoarder's Lair | Crowd pressure and tiny threats | Does a zoomed-out view obscure small enemy tells? |
| ROOM-05 | The Juicer's Gym | Co-op vertical and horizontal separation | When do players stop reading their own characters? |
| ROOM-06 | Ball of Swine | Rolling boss, central gate, long run-up | Can the boss and impending impact both fit without hiding players? |

## Zoom experiment

The approved art/render baseline stays **640×360 virtual pixels** (Deck 640×400) with **Carl 52 px / Donut 40 px** and a full-resolution A1 lighting pass. The camera here changes its **world coverage**, not the approved sprite source sizes or native render resolution.

| Camera world coverage | 16:9 world view | 16:10 world view | Carl virtual height | Donut virtual height |
|---|---:|---:|---:|---:|
| 1.00× | 640×360 | 640×400 | 52.0 px | 40.0 px |
| 1.25× | 800×450 | 800×500 | 41.6 px | 32.0 px |
| 1.50× | 960×540 | 960×600 | 34.7 px | 26.7 px |
| 1.75× | 1120×630 | 1120×700 | 29.7 px | 22.9 px |

At 1080p the virtual 640×360 frame is scaled 3×. Therefore at 1.50×, Carl is about 104 output pixels high and Donut about 80; at 1.75× they are about 89 and 69. **4K offers more physical pixels, not automatically more camera coverage**. Handheld legibility must be separately accepted because the output scale is only 2× on 1280×800.

### What to look for

1. **Upper art buffer:** Every camera position must be covered by actual environmental layers; no backdrop void or unplanned scene bounds. A hidden ceiling is acceptable as long as the visible area contains designed architecture.
2. **Threat readability:** Is the next danger genuinely visible far enough in advance? Are small attacks and knockback contacts still readable at 1.5× and 1.75×?
3. **Action priority:** A wide shot must not destroy the feeling of a punch or kick. Test holding zoom during an impact vs subtly returning toward 1.0×.
4. **Co-op agency:** Both playable characters stay recognizable; neither is forced offscreen by the other's actions; camera separation and regroup never require irreversible teleport without a fair warning.
5. **Architectural composition:** More world coverage should reveal layered vaults, distant galleries, rafters and lamps—never uniform empty negative space.
6. **Route protection:** Breakable props create choices and kills without deleting the only safe route to the exit.

## Boundaries

- The canvas character art is **schematic** and is **not** a production replacement for approved 52px Carl and 40px Donut art.
- The toy motion, collision and combat are deliberately crude. No real enemy AI, difficulty balancing, correct attack timings, final physics, hazards or checkpoint system should be inferred.
- Visual debris uses display randomness. The actual Rust/Bevy simulation must maintain deterministic authoritative events/replay.
- `ROOM-01` to `ROOM-06` are stand-alone test rooms, **not** a statement that the game's canon has this topology or sequence.
- Some attacks and props are original game adaptation hypotheses; they are not assertions about the book.
- This browser toy does **not** validate Bevy sprite sampling, render passes, lighting, frame pacing, handheld readability, DualSense input or game feel.

## Project references

- [ADR-0007](https://github.com/pando85/dungeon-floorbreak/blob/main/docs/05-decisions/ADR-0007-rendering-resolution-lighting.md)
- [Input/camera contract](https://github.com/pando85/dungeon-floorbreak/blob/main/docs/03-tech/input-camera.md)
- [Combat-first encounter design](https://github.com/pando85/dungeon-floorbreak/blob/main/docs/01-design/combat-first-encounter-design.md)
- [Floor 01 Ball of Swine arena and camera](https://github.com/pando85/dungeon-floorbreak/blob/main/docs/01-design/floor-01-ball-of-swine-arena-gameplay.md)
- [Floor 01 art atlas](https://github.com/pando85/dungeon-floorbreak/blob/main/docs/02-art/floor-01-art-atlas-v1.md)

**No repository PR was created in this step.** This is a portable design package for creative review and handoff.
