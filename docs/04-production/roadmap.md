# Roadmap — initial 90 days (relative milestones)

**Scheduling note:** Estimates are planning hypotheses for an AI-assisted/AI-authored project, not promises. Coding speed is not the bottleneck if visual assets or game-feel review are poor. Work sequentially through gates and document actual time spent.

## Day 1–7: feasibility spikes

- Set up Rust workspace / Bevy Linux app with fixed timestep and normalized action layer.
- Prototype one controllable placeholder character, pixel camera and a bomb that removes test tiles.
- Establish one deterministic headless scenario and screenshot evidence routine.
- **Gate G1:** repeatable build, two controller actions, and a reproducible terrain edit demonstration.

## Day 8–21: destruction and local co-op

- Implement material grid, dirty chunk updates, autotile edge, collision rebuild and tests.
- Add second placeholder, solo switch / follower experiment and two-player local co-op.
- Author single LDtk map with path invariants.
- **Gate G2:** co-op works; explosion updates collision without softlock, including chunk seam tests.

## Day 22–35: animation and art feasibility

- Lock canonical art direction with first character master and small palette.
- Build Aseprite source + Lua export pipeline and validation; generate two animation cycles per character.
- Measure pixel jitter, animation readability and asset iteration costs.
- **Gate G3:** character consistency and 'safe to expand assets' human art signoff.

## Day 36–60: vertical-slice production

- Complete two enemies, Carl bomb, Donut control spell, chain reaction prop, checkpoint, forward-only encounter flow with a mandatory vertical bypass, tutorial UI and audio prototypes.
- Add replay and CI screenshot artifact generation; baseline frame-time metrics on suitable Linux hardware.
- **Gate G4:** independently playable 5-minute forward-scrolling combat corridor with meaningful destruction and a forced climb over reinforced terrain.

## Day 61–75: playtest and correction

- Minimum 3 human sessions with controller (solo and 2-player included) using a standardized questionnaire.
- Fix top 5 experience issues: pacing, camera, action clarity, feedback, companion/coop behavior.
- Reduce readability failures after large explosions.
- **Gate G5:** at least 2 of 3 sessions can identify and use a destruction-based shortcut without verbal coaching.

## Day 76–90: decision on next stage

- Measure and improve Rust + Bevy change-to-test time; the engine selection is final per ADR-0001.
- Document multiplayer/Steam integration readiness, asset rights inventory, costs and technical debt.
- Draft 3–6 new rooms only after G5; define full production budget and original-IP / license choice.
- **Gate G6:** go/no-go record grounded in recorded playtest, perf and AI iteration evidence.

## Risk register

| Risk | Likelihood | Severity | Mitigation / signal |
| --- | --- | --- | --- |
| Pixel-art frame style drift | High | High | Canonical source, shared palette, contact sheets, art gate |
| Terrain collision spikes | Medium | High | Chunk dirty sets, no per-cell rigid bodies, perf fixtures |
| AI agents alter contracts inconsistently | High | High | AGENTS.md, schema tests, small PRs, explicit ADRs |
| Solo companion pathfinding frustration | Medium | High | Prototype switch vs assist early, teleport recovery |
| Scope expands into full RPG | High | High | Strict slice exclusions and gated expansion |
| IP cannot be licensed | Unknown | Critical for Steam | Original-IP layer from day one; no public franchise assets |
| Steam Deck controller/UI friction | Medium | Medium | Gamepad-first tests and device benchmark |

## Decision cadence

Every two weeks: review build/evidence, compare actual vs assumed throughput, prune backlog and accept/reject risky gameplay changes. Make technical choices only after a measured spike; avoid turning every feature request into engine work.
