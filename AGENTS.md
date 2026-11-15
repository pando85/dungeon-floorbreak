# AGENTS.md — Mandatory instructions for local LLM coding agents

You are implementing **Dungeon Floorbreak**, a 2D controller-first Linux game concept whose development is AI-driven. The repo currently contains **specifications, not a working game**. Never report a feature as implemented unless you have created and run it.

## Read order and authority

1. `README.md` for status and scope.
2. `docs/01-design/vision.md`, `gameplay.md`, `characters.md` for player-facing intent.
3. `docs/01-design/antihero-tone.md` for the **mandatory, project-wide 1980s–1990s everyman action-antihero creative rule**, followed by `docs/02-art/art-direction.md`, `carl-character-bible.md`, `donut-character-brief.md`, `animation.md`, `asset-pipeline.md` for art constraints (Carl's visual baseline is **approved**, not a blank brief); see `assets/concept/carl/approved/README.md` for the approved image path, integrity check and provenance.
4. `docs/03-tech/architecture.md`, `destruction.md`, `input-camera.md`, `level-data.md`, `testing.md` for implementation.
5. `docs/04-production/vertical-slice.md`, `roadmap.md`, `backlog.md`, `rights-and-release.md` for sequencing.
6. `docs/05-decisions/` for decision status; `docs/06-research/sources.md` for checked upstream links.

**Priority on conflict:** legal/rights guardrails and platform safety > accepted ADRs > acceptance criteria > design specifications > implementation convenience. Flag contradictions in a PR, do not silently rewrite design intent.

## Rules

- **English only** in source code, comments, documentation, PR descriptions, issues, commit messages, tests and generated prompts.
- **Creative canon for every contributor:** apply `docs/01-design/antihero-tone.md` across characters, writing, enemies, VFX, animation, level art, UX and audio. Carl is a believable, somewhat plain-looking everyman whose practical strength and charisma grow **gradually through hardship**. Reject superhero anatomy, handsome action-star faces, effortless bravado and gratuitous armor/capes. Use 1980s–1990s action cinema for *attitude and tone*, never copied designs or dialogue.
- Prefer the smallest vertical change delivering a verifiable behavior; no giant speculative rewrites.
- Use Rust/Bevy as the **provisional** production direction; do not claim this choice is irrevocable. Verify compatible crate versions against primary sources before adding dependencies.
- Game logic must be deterministic under a fixed update and seeded RNG wherever possible. Rendering and particles must not change authoritative outcomes.
- Separate `gameplay` domain logic from the Bevy rendering/adapters layer so it can be unit-tested headlessly.
- All significant mechanics need at least one automated test, a reproducible scenario, and an evidence artifact (screenshot/GIF/short recording) once the runtime exists.
- Generate pixel-art **source files**, not only opaque final PNG files. Use Aseprite `.aseprite` + Lua for repeatable edits, export atlas PNG + metadata, and validate bounds, pivots, transparency and palette constraints.
- Do not invent art, voices, copyrighted quotes, ripped sprites, music, recognizable third-party game assets, or GitHub releases. Approved independently created franchise **concept illustrations** can enter this public repository only under the recorded owner-confirmed concept-art permission and with provenance/checksum, as documented in `docs/04-production/rights-and-release.md`. Do not infer game/asset-pack distribution rights.
- Treat Carl/Donut/Mongo names and franchise-derived abilities as a **replaceable fan-concept layer**; engine and data contracts must be IP-neutral.
- Do not post tokens, local machine paths, user-specific private information, secret configuration or unauthorized screenshots.
- Never declare 'Steam Deck Verified', legal clearance, or successful benchmarks without evidence.
- Keep `docs/04-production/backlog.md` updated with the workorder and evidence status, and add an ADR for material architecture changes.

## Standard work cycle

1. Pick a `WORK-###` item, restate constraints and acceptance criteria.
2. Create a short implementation plan, list files to change and expected tests.
3. Make smallest useful change, keep data schemas stable or migrate explicitly.
4. Run available format/lint/unit/scenario tests. If blocked, record *actual* blocker and don't fabricate results.
5. Run screenshot or replay validation for visual changes, plus controller/manual checks when applicable.
6. Update documentation/backlog and provide a PR with: behavior, evidence, risks, testing, next step.
7. A human reviews *game feel* before mechanics are marked accepted.

## Agent roles (logical; one model may fulfill several)

- `gameplay`: movement, damage, combat, progression.
- `terrain`: chunking, collision update, materials, structural rules.
- `art`: style bible, source sprites, animation, atlas validation.
- `levels`: LDtk schema, navigation constraints, encounter scripting.
- `qa`: deterministic replay, snapshots, controller tests, performance.
- `integrator`: contract ownership, dependencies, CI, changelog/ADR maintenance.

## Definition of done for a game feature

- Implements the stated behavior and preserves controller usability.
- Unit/scenario tests pass; regression evidence retained.
- No licensing uncertainty introduced by copied/generated assets.
- Frame-time and memory regressions reported for performance-sensitive work.
- Acceptance criterion checked and human review request included if appropriate.

## First work order

Start with `WORK-001` in `docs/04-production/backlog.md`. Do not begin with character illustrations, online networking, Steam integration or a complete loot system.
