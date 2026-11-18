# Camera lab review (Rust + Bevy pass)

Review of the browser lab (`README.md`, `BEVY_HANDOFF.md`, `scenarios.json`,
`index.html`) before and while it was implemented as a Bevy probe
(`docs/03-tech/camera-lab.md`). The browser package stays a creative hypothesis;
this file records what held up, what conflicted with the project rules, and what
was changed. Nothing here is an approval of a camera policy.

## What is sound

- **Coverage arithmetic.** 1.0x / 1.25x / 1.5x / 1.75x of 640 x 360 are exactly
  640 / 800 / 960 / 1120 px wide (16:9) and 640 x 400 ... 1120 x 700 (16:10).
  All of them are multiples of 32 px, which the probe uses as its whole-pixel zoom
  quantum. The heights in the table (Carl 52 to 29.7 px, Donut 40 to 22.9 px in
  virtual pixels) are right.
- **"4K is more pixels, not more coverage."** Correct and confirmed by
  measurement: coverage only changes how many world pixels share the screen.
- **Presentation-only camera.** The rule that camera state never enters replay
  hashes, collision or damage is enforced by construction and by a test (same
  scripted run at every coverage gives one authoritative hash).
- **Room-to-system matrix.** The six cases map cleanly onto approved readability
  sets (goblin, llama, engineer, scatterer, hoarder, troglodyte, juicer, Ball of
  Swine) and onto the existing combat, strike and destruction code.
- **Scope guard.** The handoff keeps the render pipeline, the virtual resolution
  and the controller contract out of scope. That held: no ADR-0007 change was
  needed.

## What conflicted with ADR-0007, the controller contract or canon

| # | Finding | Rule | Resolution in the probe |
|---|---|---|---|
| 1 | Room names, subtitles, purposes and the whole mobile UI (CAMBIO, Separar duo, ...) were Spanish. | English only | Translated in `scenarios.json`, the embedded level data in `index.html`, the UI strings and `README.md`. |
| 2 | Platforms sit 108 to 272 px above the floor. Carl's approved jump (440 px/s, 1050 px/s^2) peaks at 92 px. | No superhuman jumps | Platforms up to 200 px above the floor get an authored staircase of 32 px steps for Carl; the higher ones are Donut-only perches reached through a lower platform. Donut's lab jump is 171 px (a proposal for the high-jumping cat, not approved balance). Reachability is unit-tested for both actors with every prop intact and with every prop broken. |
| 3 | Weak walls 75 to 155 px tall block the floor lane; Carl cannot jump over those above 86 px. | Protected exits, no softlock | Every blocking wall is destructible by Carl in 3 hits, the floor, steps, platforms and exit are never destructible (tested through all six scripted runs), and the gate cycles open on a timer. |
| 4 | Floor and platform y values are not on the deep-floor tile grid (walking line = 16 n + 4). | ADR-0007 deep floor | Everything of a room shifts by one offset (-4 .. +4 px) and platform tops snap to whole tiles; recorded in `Room::snap_dy`. |
| 5 | The browser auto zoom is a continuous formula (`1 + (separation - 310) / 500`). | Pixel snapping | A fractional coverage cannot be drawn 1 texel per world pixel. The operator moves in whole 32 px steps (2.5 percent each) with smoothing and hysteresis. |
| 6 | Browser keys (J/K/Tab, F/G, M/N) differ from the approved mapping. | Controller contract | The Bevy adapter maps Square/X primary, Triangle/Y physical, L1/LB solo switch (ignored in 2P), Circle/B special (story-gated: logged as locked, does nothing). Keyboard fallbacks are test-only. |
| 7 | ROOM-06 is 2060 px wide; the approved arena hypothesis is about 1200 px. | Concept lock | Kept deliberately as a camera stress test (the handoff says so); not a redesign. |
| 8 | 1.25x and 1.75x are not integer ratios at 1080p (2.4 and 1.714), 1.5x is not at 1280x800 (1.333). | ADR-0007 integer scale | Measured and documented, see the doc's rendering section. Only 1.0x (and 1.5x at 1080p / 4K) keep an integer ratio. |
| 9 | The browser gate is a static prop and the heat hazard has no mechanic. | - | The gate is a timed moving gate that never closes on a body; the heat vent is a damage zone drawn with the kit's hazard grit. |
| 10 | "A goblin rebounds against another goblin" (ROOM-01) has no equivalent in the shared goblin code (no enemy-enemy collision). | Do not edit shared modules | Not implemented; knocked enemies do bump destructible props. Listed as a limitation. |

## What does not exist as approved art

Workshop, gym, hoarder lair and swine vault have no dedicated approved
environment kit. The lab recomposes existing Floor 01 kit pieces for them (copper
pipe runs for the workshop, rubble for the lair, a denser arcade for the gym and
the arena). That is a **greybox limitation**, not art. Props are drawn with the
approved raking-shore, loose-block, timber-set, rubble and door pieces, which
are larger or differently shaped than the browser stand-ins (for example a
timber set stands for a weak wall, a stone block for a crate or a weight), so
collider and art sizes differ in places.

## What was changed

- Data and text: translation (point 1).
- New: `src/camera_lab/` (Bevy-free domain: schema, layout composer, camera
  operator, simulation, scripted runs, art mapping), `src/bin/camera_lab.rs`
  (the Bevy adapter), `scripts/camera_lab_assets.sh`, `scripts/camera_lab_evidence.sh`,
  `docs/03-tech/camera-lab.md`, `docs/evidence/camera-lab/`.
- Shared modules were not edited. The probe reuses `combat`, `goblin`, `feel`,
  `input`, `env_kit`, `lighting`, `light_pass` and `actor_sprite` as they are.
