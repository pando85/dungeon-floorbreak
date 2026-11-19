# Floor 01 — Camera pullback / high-ceiling encounter probes

## Short orchestrator brief (English)

> Treat the attached `scenarios.json` and standalone `index.html` as **creative hypotheses only**. In a new, isolated Rust + Bevy workstream, implement deterministic greybox probes for ROOM-01 to ROOM-06, prioritizing camera zoom/framing and background coverage. Preserve ADR-0007 (640×360 / 640×400 virtual pixels, integer final scaling, full-res A1 lighting, Carl 52px / Donut 40px), the approved controller mapping, solo switching/AI and 2P ownership. Test 1.0×, 1.25×, 1.5× and exploratory 1.75× camera coverage, real pixel snapping, hit-readability, enemy telegraphs, 1P/2P separation, protected exits and no empty scenery above the action. Build playable fixtures with reproducible seeds, headless state traces and real Bevy captures for 1080p and 1280×800; request a human game-feel verdict before approving any new camera policy. Do not merge into currently owned scene/integration branches without coordinating with their agents.

## Design intent

The creative director wants the camera to **pull farther back** in large encounters, using the 1080p / 4K output clarity where helpful. Floor 01 locations must be authored **above the normal viewport** as genuine masonry, high vaults, galleries and background architecture, so any expanded view remains fully composed.

This changes the **camera coverage hypothesis**, not the already approved native pixel-art density. Escalating camera world coverage makes projected characters smaller even at 4K. Evidence and acceptance decide where to cap it; do not silently amend ADR-0007.

### Initial operating hypothesis (unapproved)

- 1.00× — short brawls, precise contacts, tiny enemies and critical hit frames.
- 1.25× — default optional mid-distance in ordinary open rooms and mixed encounters.
- 1.50× — crowd pressure, vertical ambush, co-op separation and boss charges.
- 1.75× — extreme stress test / occasional short reveal; do not commit as permanent gameplay zoom without high-confidence 1080p and Deck playtests.
- Automatic framing: smoothly interpolate between candidates, bias horizontally toward an imminent charge **with readable tell**, preserve two actor safety regions, clamp to authored camera bounds, use hysteresis to prevent zoom pumping and avoid hard snaps during impacts.
- Presentation-only camera changes do **not** enter replay state hashes, alter collision, scale authoritative coordinates or change hit/damage outcomes.

## Room-to-system test matrix

| Room | Contact/camera case | Environment coverage | Coop case | Minimum sign-off |
|---|---|---|---|---|
| ROOM-01: goblins | Hit-stop and kick response at 1.0/1.25/1.50 | Ordinary masonry + occasional high arch; no arbitrary constant industrial clutter | P2 high flank pre-magic; solo swap | Readable hit and enemy telegraph, recoverable simple kill |
| ROOM-02: llamas | Ground/upper ranged warning in same frame | Scorch marks local to lane, high upper gallery actually rendered | Carl under cover, Donut elevated | Warnings visible by shape/animation, no forced P2 spectator time |
| ROOM-03: workshop | Wall break → prop impact → knockback cause and effect | Industrial detail localized to workshop | Two valid approaches, one protected shared exit | Direct option and creative elimination both readable |
| ROOM-04: hoarder | Small enemy vs crowd recognition at 1.5× | Trash concentrated in lair; upper arches still legible | Flanking without hard camera drift | Players distinguish tiny threats at requested outputs |
| ROOM-05: juicer gym | Two actors separated along 2 axes; rapid regroup | Gym mirrors/upper beams, no undrawn ceiling | Simultaneously meaningful 1P/2P roles | Both actors visible or explicitly safely recoverable |
| ROOM-06: swine | Fast 100–110px boss; moving gate and charge | Large masonry/brass vault; moving architecture remains readable | Bait-and-strike duo or direct boss damage | No unseen boss impacts, no confusing camera zooms |

## Implementation sequence

1. **World bounds/data:** define six small geometry fixtures using one common world-coordinate schema, explicit render bounds, walkable safe floor, optional upper platforms and individually tagged destructible props. The top architecture MUST be drawn beyond normal camera coverage; asset render bounds and collision bounds are separate.
2. **Camera operator:** support scripted fixed coverage modes + optional automatic duo framing; clamp to authored bounds. Test 16:9 and 16:10 at the approved virtual resolutions. Do not change the game's physical camera controls or HUD input semantics.
3. **Player/target markers:** first use actual 52/40px production sprites and relevant enemy silhouettes, rather than trusting browser stand-ins. Measure effective onscreen px and inspect 1-pixel edges and lighting.
4. **Mechanical probes:** reuse existing combat event and deterministic destruction systems rather than creating another parallel combat architecture. Only include behaviors and story unlocks that are valid for the tested act. Continue the current pre/post-Guild separation.
5. **Evidence:** screenshots/video from Bevy and headless traces. Compare identical deterministic scenario events at fixed 1.0/1.25/1.50/1.75 with rendering-only camera changes; final gameplay state and authoritative replay hashes should match.
6. **Human acceptance:** ask the creative director to compare screenshots AND play, preferably on a 1080p display and Steam Deck/1280×800; feed verdict into a dedicated camera decision proposal. Keep broad creative approval separate from CI success.

## Reproducible test runs (example structure, not executable commands)

For each room, record:

```yaml
room: ROOM-01
art_source: current-approved-master
view: 640x360 # repeat with 640x400
output: 1920x1080 # repeat at 1280x800; test 4K when available
camera_coverage: 1.50
mode: solo-switch # or 2p-local
start_state_seed: fixed-per-room
camera_behavior: fixed # or auto-shared
checks:
  - protected-exit-reachable
  - both-actors-readable
  - camera-world-coverage-within-render-bounds
  - threats-and-hit-tells-visible
  - replay-events-and-final-hash-unchanged-vs-baseline
  - no-new-pixel-shimmer-or-layer-seams
human:
  melee_feel: needs_review
  environmental_kill_causality: needs_review
  partner_agency: needs_review
  camera_comfort: needs_review
  overall_preference: needs_review
```

## Potential scope conflicts to avoid

- Current `floor01_scene` and integrated combat are owned by other local agents; create isolated probes and coordinate ownership before touching shared source.
- Do not assign Carl superhuman jumping, generic enemy grabbing or unearned explosives; do not give Donut magic before the story unlock.
- Level topology, exact coordinates, boss attack timing and stage order in the browser prototype are not accepted canon or final game-design locks.
- The actual Ball of Swine concept currently uses a ~1200 world px arena hypothesis. The larger browser room deliberately **stresses camera framing**, not a redesign of the already approved arena concepts.
- Any need to revise the pixel-art render pipeline or change the accepted virtual resolution requires separate approval and evidence.

## Proposed creative acceptance decision

Choose a zoom policy only after evidence from ROOM-01 (melee), ROOM-02 (verticality), ROOM-04 (tiny threat), ROOM-05 (separation) and ROOM-06 (boss). Favor an adaptive camera if it improves readability but don't make a wide zoom the default at the expense of the tactile feel of basic kills. Preserve tall authored environments regardless of final camera policy.
