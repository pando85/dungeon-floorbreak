# Floor 01 — Ball of Swine: combat space, movement and camera proposal

**Status (2026-11-18):** conceptual encounter design, to be greyboxed and playtested. Owner prefers the new wider annotated layout and supports exploring camera pullback. **Not** approved geometry, physics values, exact camera zoom, production art or implementation.

## Encounter fantasy and mechanics

The Ball of Swine is a rolling composite. It accelerates, **commits to a readable direction**, collides with fixed or moving authored structures, rebounds and sometimes loses a bounded number of Tusklings. Players can anticipate the path, bait the charge, change the local geometry using actions they already possess, and exploit the resulting stagger. Direct fighting remains a valid, more dangerous strategy. No mandatory waiting-for-three-identical-phases loop.

The pleasure is causal spectacle: **telegraph -> provoke/position -> impact -> detached creatures or breakage -> earned follow-up**. Each impact should visibly explain what the player caused; avoid randomness that makes the same setup behave differently for no discernible reason.

### First layout measurements — PROTOTYPE HYPOTHESES, not approval

| Property | Proposed starting value / behavior | Validation needed |
| --- | --- | --- |
| Side-on room width | About **1,200 world px** | Travel downtime, charge speed, duo separation and camera limits |
| Standard view | Virtual 640×360 (Deck alternative 640×400) | Approved art/render convention |
| Medium camera coverage | About **800×450** world px at 16:9 (1.25x) | Pixel stepping, gameplay-legible actors |
| Maximum camera coverage | About **960×540** world px at 16:9 (1.5x) | Only if handheld/screen tests remain legible |
| Main floor | Mostly uninterrupted, approximately 70% of the usable run-up | Actual collision and enemy velocity |
| Ball diameter | Start testing around **100–110 world px** | Boss art/collider scale and attack distance |
| Side routes | Left/right elevated platforms and slower Carl access | Solo/2P traversal, no untouchable perch |
| Mechanics | One central hanging gate plus limited side gates | Telegraph, swept contact and boss rebound |
| Destruction | Selected side supports and props, protected essential floor | Reachability after every authored state |

Changing camera world coverage does **not** redefine or upscale the approved character source sprites. A pullback shrinks their onscreen silhouette, so the apparent quality must be validated, not assumed.

## Arena traversal roles

**Carl:** Ground-level, fast access to targets, physical strikes, **kicks (not grabs)** and selected destruction/earned bombs. Can punt a detached live Tuskling into another foe or the vulnerable composite **only if** the collision implementation is proven deterministic. Direct melee remains satisfying. Slower authored access to both elevated side regions avoids character softlocks.

**Donut:** Faster vertical/flanking mobility. High/far jumps let her seek elevated firing angles, then return to the floor. Her attacks are limited to powers actually unlocked by this late-floor story point. No spells before their canonical unlock; no automatic shoulder ability. Platforms must never create a permanent unattackable safe zone.

**The ball:** Mostly rolls on the central line, with predictable collision/rebounds and occasional arcs. Committed charges must display a pre-charge direction and a recovery. A gate strike may interrupt momentum, stagger the ball and separate a bounded number of Tusklings. Do not use unbounded body physics, hidden homing or instant off-screen hits.

**Solo:** Instant Carl/Donut switch and reliable companion support; no simultaneous two-human input required. **Local 2P:** both players have useful actions, and no platformer waiting-room segment.

## Three environment systems

1. **Rolling lane:** provide enough horizontal run-up to create suspense but preserve openings for melee and kick follow-ups.
2. **Moving mechanical gates:** timed, visible changes that offer impact opportunities; interaction, if needed, must reuse an approved input verb instead of adding a new mandatory button. Show sweep volume, warning and safe escape before closure.
3. **Selective breakage:** terrain changes can open new angles and expose an exit but must never destroy the final safe floor, essential checkpoint or only reunion route.

Progression beats: **intact/learn** -> **damaged/recontextualize** -> **final chaos/finish**. Treat these as authored visual/combat states, not locked HP percentages. Preserve readability when UV lamps, sparks and debris activate.

## Camera proposal and accepted-policy conflict

The owner suggested allowing the camera to pull away rather than fitting the arena within one viewport. This is sensible for a rolling boss and shared-screen co-op **but is not yet an accepted camera-system amendment**.

The current [ADR-0007](../05-decisions/ADR-0007-rendering-resolution-lighting.md) retains **640×360 / 640×400 source/virtual pixels and integer output scaling**, and [camera design](../03-tech/input-camera.md) allows only a modest zoom-out subject to legibility. A wide view means **more world units per virtual pixel**, so nearest-neighbor pixel snapping and edge readability require special attention; a source-height 52px Carl would appear ~42px at 1.25x and ~35px at 1.5x, while Donut 40px becomes ~32px and ~27px. The generated image's camera box labels are illustrative, not a validated projection specification.

### Camera behavior to prototype

- **Normal (1x):** biased midpoint of two players, includes imminent boss tells; close-up detail for kicks.
- **Wide (up to ~1.25x):** ease outward when player separation, acceleration run-up, or a gate strike demands extra context. Prefer smooth, anticipated movement with hysteresis to repeated zoom pumping.
- **Extreme (~1.5x, optional):** brief view for huge transformations; only if 1080p TV and 1280×800 handheld evaluations confirm player/attack legibility. Do not pull away during precision hit windows just for spectacle.
- **Return:** damped/smooth return to normal; no arbitrary hard snaps, forced teleport or sudden split-screen.
- **Framing constraints:** keep both players and the imminent charge visible when possible; camera lead on the telegraphed travel direction. Visible hazard warnings may supplement, not replace, honest framing. Do not make gameplay outcome depend on presentation camera state.

**Technical gate:** compare actual Bevy screenshots/replay at 1x/1.25x/1.5x, 1080p and Deck, assessing 1-pixel shimmer, nearest-neighbor edge behavior, actors' effective pixel heights, UI readability, visible telegraphs, co-op split/regroup, and determinism. If production requires changing the accepted rendering/pixel sampling convention, obtain owner approval and an ADR before implementing that change.

## Greybox probes (separate follow-up PRs)

| ID | Probe | Acceptance questions |
| --- | --- | --- |
| BSW-SPACE-01 | 1,200 px flat main lane and two elevated flanks | Do solo and 2P traverse/rejoin without a dead end? Is the space mostly useful? |
| BSW-BALL-01 | Committed charges, side bounce and central gate strike | Can players bait the ball? Is collision physically satisfying and reproducible? |
| BSW-TUSK-01 | Few detached Tusklings, limited kick chain | Are consequences readable and deterministic without runaway chaos? |
| BSW-GATE-01 | Vertical gate sweep and one breakable support | Safety tell, protected exit, states after destruction |
| BSW-CAM-01 | Normal / 1.25x / 1.5x during co-op and boss charges | Pixel/handheld readability, no camera whiplash, clear boss threat |
| BSW-END-01 | Normal damage path and environmental defeat alternative | Is the boss winnable without mandatory authored puzzles? |

These are **test designs only**. Implementing collision, levels, camera or boss behavior requires independent Rust + Bevy work, deterministic/headless scenario evidence, and human game-feel review.

## Controls, narrative and long-term horizon

This concept uses the [approved MVP control contract](mvp-controller-contract.md): Carl's physical action is a kick; no extra buttons, grab action, swords or mandatory precision aiming. Donut uses only earned magic and movement. The [character progression horizon](character-progression-horizon.md) was reviewed and still fits: Floor 01 ends as a **mastery test with existing actions**, rather than granting a new skill mid-boss. Narrative ability availability must be verified before labeling it book canon.

See [standalone art concept](../02-art/floor-01-ball-of-swine-arena-concept.md), [combat-first encounter direction](combat-first-encounter-design.md) and [hybrid physics](combat-feel-hybrid-physics.md).
