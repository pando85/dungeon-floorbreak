# AGENTS.md — Mandatory instructions for local LLM coding agents

You are implementing **Dungeon Floorbreak**, a 2D controller-first Linux game concept whose development is AI-driven. The repo contains **specifications, approved art references and an early Bevy greybox**. Do not confuse this prototype with a production-ready game. Never report a feature as implemented unless you have created and run it.

## Read order and authority

1. `README.md` for status and scope.
2. `docs/01-design/vision.md`, `gameplay.md`, `characters.md` for player-facing intent.
3. `docs/01-design/antihero-tone.md` for the **mandatory, project-wide 1980s–1990s everyman action-antihero creative rule**, followed by `docs/02-art/art-direction.md`, `carl-character-bible.md`, `donut-character-brief.md`, `donut-character-bible.md`, `mongo-character-bible.md`, `animation.md`, `asset-pipeline.md` for art constraints (Carl's visual baseline is **approved**, not a blank brief); see Carl and Donut approved asset directories; Mongo's selected source PNGs are inventoried at `assets/concept/mongo/approved/README.md` and all three originals are committed with a SHA-256 manifest and verified Git blob identities.
4. `docs/03-tech/architecture.md`, `destruction.md`, `input-camera.md`, `level-data.md`, `testing.md` for implementation.
5. `docs/04-production/vertical-slice.md`, `roadmap.md`, `backlog.md`, `rights-and-release.md` for sequencing.
6. `docs/05-decisions/` for decision status; `docs/06-research/sources.md` for checked upstream links.
7. `docs/07-agents/agent-orchestration.md` for anyone orchestrating agents (model tiers, briefs, reports, CI waiting, reviews, top-tier budget).

**Priority on conflict:** legal/rights guardrails and platform safety > accepted ADRs > acceptance criteria > design specifications > implementation convenience. Flag contradictions in a PR, do not silently rewrite design intent.

## Rules

- **No private agent memory.** Never use an agent tool's persistent memory (for example Claude Code memory) for this repository. Owner rules, decisions and process changes are recorded in the repository (AGENTS.md, docs, ADRs, backlog) so every agent and harness sees them.
- **English only** in source code, comments, documentation, PR descriptions, issues, commit messages, tests and generated prompts.
- **Creative canon for every contributor:** apply `docs/01-design/antihero-tone.md` across characters, writing, enemies, VFX, animation, level art, UX and audio. Carl is a believable, somewhat plain-looking everyman whose practical strength and charisma grow **gradually through hardship**. Reject superhero anatomy, handsome action-star faces, effortless bravado and gratuitous armor/capes. Use 1980s–1990s action cinema for *attitude and tone*, never copied designs or dialogue.
- Prefer the smallest vertical change delivering a verifiable behavior; no giant speculative rewrites.
- Use **Rust + Bevy only** as the accepted engine decision (ADR-0001). Do not propose Godot migration or make a Godot prototype. Verify compatible crate versions against primary sources before adding dependencies.
- Game logic must be deterministic under a fixed update and seeded RNG wherever possible. Rendering and particles must not change authoritative outcomes.
- Separate `gameplay` domain logic from the Bevy rendering/adapters layer so it can be unit-tested headlessly.
- All significant mechanics need at least one automated test, a reproducible scenario, and an evidence artifact (screenshot/GIF/short recording) once the runtime exists.
- Generate pixel-art **source files**, not only opaque final PNG files. Use Aseprite `.aseprite` + Lua for repeatable edits, export atlas PNG + metadata, and validate bounds, pivots, transparency and palette constraints.
- Do not invent art, voices, copyrighted quotes, ripped sprites, music, recognizable third-party game assets, or GitHub releases. Independently created art of the fan-concept characters (concept images **and** production art: pixel sources, spritesheets, animation data, GIF previews, gameplay-scale renders, CI art artifacts) can enter this public repository under the recorded owner-reported `ART-PUBLICATION-ALL-V1` scope and with provenance/manifest/checksum, as documented in `docs/04-production/rights-and-release.md`. This is not independently verified consent and not a public reuse licence; do not infer rights to distribute a playable game, Steam build, trailer, merchandise or standalone asset pack.
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

## Model routing for art work

Every art work order (characters, animation, environments, props, enemies, vehicles) follows this pipeline, whatever the wording of its brief. Owner directive, 2026-11-17, refining the 2026-11-16 rule. Tiers are defined in [docs/07-agents/agent-orchestration.md](docs/07-agents/agent-orchestration.md#model-tiers); the Claude names below are the Claude mapping and any opencode model of the same tier fills the same role.

1. **Design: top-tier model (Claude Opus)** (the highest-capability tier available). It produces key poses or the design itself, silhouettes, palette, timing, and a written design spec with acceptance targets. **How much of the design the top tier does depends on the asset's difficulty: follow [routing by difficulty](docs/07-agents/pixel-art-pipeline.md#routing-by-difficulty)** (owner decision 2026-11-18). **Budget rule:** the design step stops at the keys and the spec. Execution chores (evidence regeneration, contact sheets, checksums, README rows, validators, CI fixes) go to the medium tier, see [Top-tier budget](docs/07-agents/agent-orchestration.md#top-tier-budget).
2. **Execution: medium-tier model (Claude Sonnet)** (the default tier). It does the work that follows the top-tier design: pixel production that the spec spells out, grid/`.ase` sources, manifests, export, validators, tests, previews, evidence, CI and docs. Design problems found here go back to the top tier. The medium tier does not re-decide them.
3. **Review: a separate top-tier agent (Claude Opus).** This is an independent art-director review against the design spec and the art acceptance criteria. An authoring agent's self-review does not count. Defects loop back to step 2 (or to step 1 for design defects) until the review passes.
4. **Owner review at the end only.** The owner sees the finished result as images (contact sheets, enlarged frames, GIFs, in-scene compositions at gameplay scale) plus schemas/diagrams (frame layout, pivots, timing and state flow). Agents do not ask the owner for interim approvals.
   **Merge delegation (owner, 2026-11-18):** a pixel-art PR (sprites, environment kits, VFX and their evidence/docs) may be marked ready and squash-merged by the orchestrator, without waiting for owner approval, once its independent top-tier review has passed (plus the medium-tier fix check, or a top-tier re-review after a design defect) and CI is green on the final head. The owner still receives a batch summary with images and any open non-blocking design notes, and can reopen any asset. Gameplay, Bevy integration, game-feel and non-art PRs still need owner approval.

- Bevy/gameplay integration of art stays on the medium tier unless the PR records a reason.
- Record which model designed, executed and reviewed the art in the PR's `Rights/Provenance` section.

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

Continue from the right-scrolling combat WORK-021 greybox and the current top-priority work in `docs/04-production/backlog.md`. Do not begin with character illustrations, online networking, Steam integration or a complete loot system.
