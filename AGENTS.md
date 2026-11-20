# AGENTS.md — Mandatory instructions for local LLM coding agents

You are implementing **Dungeon Floorbreak**, a 2D controller-first Linux game concept whose development is AI-driven. The repo contains **specifications, approved art references and an early Bevy greybox**. Do not confuse this prototype with a production-ready game. Never report a feature as implemented unless you have created and run it.

## Read order and authority

1. `README.md` for status and scope.
2. `docs/01-design/vision.md`, `gameplay.md`, `characters.md` for player-facing intent; **also consult `docs/01-design/character-progression-horizon.md` for a living, non-binding long-term direction** whenever designing character abilities, unlocks, floor mechanics or cooperation. Approved input semantics remain in `docs/01-design/mvp-controller-contract.md`.
3. `docs/01-design/antihero-tone.md` for the **mandatory, project-wide 1980s–1990s everyman action-antihero creative rule**, followed by `docs/02-art/art-direction.md`, `carl-character-bible.md`, `donut-character-brief.md`, `donut-character-bible.md`, `mongo-character-bible.md`, `animation.md`, `asset-pipeline.md` for art constraints (Carl's visual baseline is **approved**, not a blank brief); see Carl and Donut approved asset directories; Mongo's selected source PNGs are inventoried at `assets/concept/mongo/approved/README.md` and all three originals are committed with a SHA-256 manifest and verified Git blob identities.
4. `docs/03-tech/architecture.md`, `destruction.md`, `input-camera.md`, `level-data.md`, `testing.md` for implementation.
5. `docs/04-production/vertical-slice.md`, `roadmap.md`, `backlog.md`, `rights-and-release.md` for sequencing.
6. `docs/05-decisions/` for decision status; `docs/06-research/sources.md` for checked upstream links.
7. `docs/07-agents/agent-orchestration.md` for anyone orchestrating agents (model tiers, briefs, reports, CI waiting, reviews, top-tier budget).

**Monster-death art reference:** Before designing brutal monster deaths, dismemberment, explosive gibs or death-related rendering, read `tools/art-references/death-lab/README.md`, open its standalone `index.html` at **intensity 4**, and read `docs/03-tech/monster-death-system-proposal.md`. The HTML uses original Carl and GOB-01 base sprite pixels but conceptual effects; it is a visual reference, **not** a Bevy implementation or approved production asset. Follow the art-source/palette rules and do not mistake visual fragments for authoritative physics. The universal death language (PR #79), defeat causes and species profiles (PR #94) and the Bevy cosmetic presenter (PR #97) are merged; their game feel still needs human review.

**Living-horizon review rule:** For meaningful changes to playable moves, progression, abilities, new floor gameplay, co-op or terrain-combat interactions, compare against `docs/01-design/character-progression-horizon.md`. State in the design PR whether it still fits; update the guide if the direction has materially evolved. A horizon example is never approval to implement it; gameplay signoff still applies.

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
- Keep `docs/04-production/backlog.md` updated with the workorder and evidence status (in speed mode the orchestrator does this after merge and implementation PRs record status in the PR body, see Speed mode), and add an ADR for material architecture changes.

## Standard work cycle

1. Pick a `WORK-###` item, restate constraints and acceptance criteria.
2. Create a short implementation plan, list files to change and expected tests.
3. Make smallest useful change, keep data schemas stable or migrate explicitly.
4. Run available format/lint/unit/scenario tests. If blocked, record *actual* blocker and don't fabricate results.
5. Run screenshot or replay validation for visual changes, plus controller/manual checks when applicable.
6. Update documentation and, outside speed mode, the backlog (in speed mode the orchestrator updates the backlog after merge and the PR body carries the status), and provide a PR with: behavior, evidence, risks, testing, next step.
7. A human reviews *game feel* before mechanics are marked accepted.

## Model routing for art work

Every art work order (characters, animation, environments, props, enemies, vehicles) follows this pipeline, whatever the wording of its brief. Owner directive, 2026-11-17, refining the 2026-11-16 rule. Tiers are defined in [docs/07-agents/agent-orchestration.md](docs/07-agents/agent-orchestration.md#model-tiers); the Claude names below are the Claude mapping and any opencode model of the same tier fills the same role.

1. **Design: top-tier model (Claude Opus)** (the highest-capability tier available). It produces key poses or the design itself, silhouettes, palette, timing, and a written design spec with acceptance targets. **How much of the design the top tier does depends on the asset's difficulty: follow [routing by difficulty](docs/07-agents/pixel-art-pipeline.md#routing-by-difficulty)** (owner decision 2026-11-18). **Budget rule:** the design step stops at the keys and the spec. Execution chores (evidence regeneration, contact sheets, checksums, README rows, validators, CI fixes) go to the medium tier, see [Top-tier budget](docs/07-agents/agent-orchestration.md#top-tier-budget).
2. **Execution: medium-tier model (Claude Sonnet)** (the default tier). It does the work that follows the top-tier design: pixel production that the spec spells out, grid/`.ase` sources, manifests, export, validators, tests, previews, evidence, CI and docs. Design problems found here go back to the top tier. The medium tier does not re-decide them.
3. **Review: a separate top-tier agent (Claude Opus).** This is an independent art-director review against the design spec and the art acceptance criteria. An authoring agent's self-review does not count. Defects loop back to step 2 (or to step 1 for design defects) until the review passes.
4. **Owner review at the end only.** The owner sees the finished result as images (contact sheets, enlarged frames, GIFs, in-scene compositions at gameplay scale) plus schemas/diagrams (frame layout, pivots, timing and state flow). Agents do not ask the owner for interim approvals.
   **Merge delegation (owner, 2026-11-18):** a pixel-art PR (sprites, environment kits, VFX and their evidence/docs) may be marked ready and squash-merged by the orchestrator, without waiting for owner approval, once its independent top-tier review has passed (plus the medium-tier fix check, or a top-tier re-review after a design defect) and CI is green on the final head. The owner still receives a batch summary with images and any open non-blocking design notes, and can reopen any asset. Gameplay, Bevy integration, game-feel and non-art PRs still need owner approval.
   **Extended (owner, 2026-11-19):** gameplay, Bevy integration and other non-art PRs may also be marked ready and squash-merged by the orchestrator once an independent top-tier review has passed (plus a fix check, or a narrow re-review after design or logic fixes) and CI is green on the final head. Game feel stays the owner's call: a mechanic merged this way is recorded in the backlog as "merged, game-feel review pending" and is not marked accepted until the owner plays it; the owner can request changes in follow-up PRs. A PR the owner explicitly marked "do not merge" (for example a probe) stays unmerged.
   **Design defaults (owner, 2026-11-19):** the orchestrator does not wait for owner sign-off on gameplay design choices. It adopts the recommended options of a top-tier design analysis, records each adopted value as data with its question ID, documents genuinely open questions in the repo, and merges under the delegation above after an independent top-tier review. The owner reviews game feel after merge and may change any adopted value in a follow-up PR.
   **Speed mode (owner, 2026-11-20):** optimize for the end result, not for stable intermediate states. Implementation PRs do not edit `docs/04-production/backlog.md`; they carry status notes in the PR body and the orchestrator batch-updates the backlog after merges. An independent top-tier review is still required, but non-blocking findings may be fixed forward in a follow-up PR, and a PR may merge after a medium-tier fix check without a top-tier re-review unless the review found a design or logic defect. Related stages may be grouped into one PR to cut cycles. Speed mode does not relax the legal, rights and provenance guardrails, green CI on the final head, byte-identical goldens (unless a PR explicitly changes them), flag gating of new behavior, or the owner's game-feel review. See [Speed mode](docs/07-agents/agent-orchestration.md#speed-mode). **Autonomy (owner, 2026-11-20):** the orchestrator runs every work order already defined in the repo to completion without asking for a go-ahead, adopts its own recommendation for blockers that are not very big (recorded as an adopted default in the PR body) and escalates only for large blockers, legal/rights/provenance questions, contradictions with an accepted ADR and game-feel review.

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

## ChatGPT → Hermes mobile concept-image handoffs

When the owner requests a PR containing ChatGPT-generated concept images, follow [the mobile PR handoff protocol](docs/07-agents/chatgpt-hermes-image-pr.md). Prepare a draft PR with documented pending image slots and produce a **PR-specific ZIP** (Spanish instructions, Hermes task and JSON mapping) in the same response. Hermes receives original PNGs separately and commits them to **that PR branch**, updating provenance, manifests and validation. Never claim missing images or hashes exist, mark an incomplete image PR ready, or auto-merge.

