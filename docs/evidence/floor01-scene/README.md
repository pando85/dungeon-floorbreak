# Evidence: Floor 1 corridor scene (COMP-01 integration, COMP-02 lighting)

Real Bevy 0.20.1 renders of `floor01_scene` (see [floor01-scene.md](../../03-tech/floor01-scene.md)). Since COMP-02 the default render includes the A1 lighting pass (ambient, torch pools, shadows, haze, glow, flame bloom; presentation only). The `nolight` files are the COMP-01 look (`--no-lighting`) for comparison. **No claim of art acceptance, game feel or readability is made**; the comparison numbers measure the Bevy pass against the Pillow reference model, nothing more.

Captured on the author's Linux machine under Xvfb (`xvfb-run`) with `scripts/ci_floor01_render_smoke.sh`, from the commit recorded in the PR. The full set (including logs) is also the CI artifact `the-crawl-floor01-scene`. Checksums: [SHA256SUMS](SHA256SUMS).

| File | Run | What to look at |
| --- | --- | --- |
| `floor01-scene-1920x1080.png` | default start, `--scale 3`, 1920x1080 (**primary target**) | lit start: end wall, door, first torch with warm pool, floor pools, Carl and Donut with contact shadows |
| `floor01-scene-640x360.png` | default start, 1280x720 window (2x) | same at 2x |
| `floor01-scene-640x400.png` | `--deck`, 1280x800 window (2x) | Deck framing: 20 px more headroom and floor depth |
| `floor01-scene-advanced-640x360.png` | `--scenario walk_right --capture-tick 250` | B and C segments: arch, hazed parallax gallery with its own torches, copper run |
| `floor01-scene-advanced-640x400.png` | same with `--deck` | 16:10 framing |
| `floor01-scene-nolight-1920x1080.png` | `--no-lighting --scale 3` | the COMP-01 look at the same view as the lit 1920x1080 file |
| `floor01-scene-nolight-advanced-640x360.png` | `--no-lighting`, same scripted capture | same state hash as the lit advanced capture (CI checks it) |
| `floor01-scene-reference-1920x1080.png` | `--reference-frame --scale 3` | camera window and actor pose of the ENV-01 lit preview |
| `floor01-lighting-bevy-vs-pillow.png` | `scripts/compare_floor01_lighting.py` | Bevy frame, Pillow frame and a dE76 heat map, labelled |
| `floor01-lighting-compare.json` | same | per-region mean L*, dE76, warm/cold share |
| `floor01-scene-collision-640x360.png` | `--scenario walk_right --capture-tick 80 --debug-collision` | merged solids (red), walking lines (cyan), Carl's 24 x 52 box (green), drawn after and not lit |
| `floor01-scene-turn-around-1920x1080.png` | `--scenario turn_around --capture-tick 185 --scale 3` | 1080p with Carl flipped (facing left) and Donut behind him |

The frozen captures print the same `FLOOR1_SCENE_STATE` line in the headless smoke and in the render process; CI compares them (cross-process determinism of the simulation state, not proof that the pixels match it). Separately, CI steps the scripted walk live for 250 ticks in the render loop with lighting active and with `--no-lighting` and requires the same final state in both and in the headless simulation. A passing render proves integration, not quality. Performance numbers are in the scene doc (frame intervals on one desktop GPU; nothing on a Steam Deck).

## Manual controller playtest checklist (human review required)

Fill in and attach to the PR. Nothing here is accepted until a person has played it.

```text
Tester:
Device (controller model / Steam Deck / keyboard):
Display and window size (e.g. 1920x1080 at 3x, 1280x800 at 2x):
Commit (git rev-parse --short HEAD):
Date:
```

Run `cargo run --locked --bin floor01_scene` (and again with `--deck`, and at `--scale 3` on a 1080p screen). Mark each line pass / fail / note.

1. **Movement, three devices:** left stick, D-pad and keyboard (A/D and arrows) each walk Carl left and right. Stick centre does not drift (deadzone 0.18); no stuck movement after release.
2. **Jump:** Space and South jump. Jump is about 92 px high and lands exactly on the walking line.
3. **Jump buffer and coyote time at platform edges:** press jump slightly before landing next to a platform (should still jump); walk off a platform edge and press jump just after (should still jump, within about 0.1 s). Report anything that feels late or early.
4. **Platforms:** the 1-tile platform (A1) and the 2-tile platform (A2, around x 1392) are climbable by jumping and walkable off the other side; no snagging on corners, no clipping into the floor or end wall.
5. **Restart:** R and North return Carl and Donut to the start and re-frame the camera without a visible jump of more than a few frames.
6. **Exit:** walking into the B2 arch ends the run (`FLOOR1_EXIT_REACHED` in the terminal); Esc and Start quit cleanly.
7. **Hot-unplug:** unplug the controller while holding a direction. Carl must stop (no stuck input); replugging works.
8. **Camera:** the camera leads Carl in the facing direction by about 24 px, does not move inside the +-40 px deadzone, does not snap on turning, stops at both corridor ends, and caps its speed. Look for shimmer: Carl and the brick rows must not shake while walking.
9. **Carl's feet on a flip:** turn left and right repeatedly on the flat floor. The feet must stay on the walking line; a 1 px sideways step on flip is a known limit (see the doc), anything larger is a bug.
10. **Donut follow:** Donut trails about 32 px behind, lands on platforms after Carl, never blocks or pushes him, passes through him when he reverses. Note whether that passing looks acceptable.
11. **Readability:**
    - at 1280x800 (Steam Deck-like, handheld distance): can Carl and Donut be told apart from the brick? Is the floor edge clear?
    - on a big 1080p screen at 3x: same questions, plus does the scene read as the owner-approved Floor 1 mood **without lighting** (expected flatter than A1)?
12. **A/B/C pacing:** walk the whole sample once. Does A (plain brick, torches) feel calm, B (arch and gallery) a clear change, C (copper run) a localized accent? Does the unlit stretch near x 1040-1632 read as intended? About how long is the walk?
13. **Parallax:** through the arches the gallery moves slower than the wall (far 0.35, mid 0.6). Note any popping, tearing, empty edge or shimmer.
14. **Torches:** the flames animate calmly; none flicker out of step in a distracting way.

Free notes (anything that surprised you, including art and tone):

```text

```
