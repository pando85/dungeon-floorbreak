# Evidence: Floor 1 corridor scene (COMP-01)

Real Bevy 0.20.1 renders of `floor01_scene` (see [floor01-scene.md](../../03-tech/floor01-scene.md)). They show the **pixel layers only, with no lighting, bloom or haze**: the tiles carry baked light and the pool decals, so the frames are flatter than the A1 look-dev preview. COMP-02 adds the lighting pass.

Captured on the author's Linux machine under Xvfb (`xvfb-run`) with `scripts/ci_floor01_render_smoke.sh`, from the commit recorded in the PR. The full set (including logs) is also the CI artifact `the-crawl-floor01-scene`. Checksums: [SHA256SUMS](SHA256SUMS).

| File | Run | What to look at |
| --- | --- | --- |
| `floor01-scene-640x360.png` | default start, 1280x720 window (2x) | A1 start: end wall, door, first torch, 1-tile platform, Carl and Donut |
| `floor01-scene-640x400.png` | `--deck`, 1280x800 window (2x) | same start with 20 px more headroom and floor depth |
| `floor01-scene-advanced-640x360.png` | `--scenario walk_right --capture-tick 250` | camera across the end of B1 and the start of C1: arch, parallax gallery, copper run |
| `floor01-scene-advanced-640x400.png` | same with `--deck` | 16:10 framing |
| `floor01-scene-collision-640x360.png` | `--scenario walk_right --capture-tick 80 --debug-collision` | merged solids (red), walking lines (cyan), Carl's 24 x 52 box (green) on the 1-tile platform |
| `floor01-scene-turn-around-1920x1080.png` | `--scenario turn_around --capture-tick 185 --scale 3` | 1080p at 3x with Carl flipped (facing left) and Donut behind him |

The two scripted captures print the same `FLOOR1_SCENE_STATE` line in the headless smoke and in the render; CI compares them. A passing render proves integration, not quality: **no claim of game feel, readability or performance is made, and no performance data was collected.**

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
