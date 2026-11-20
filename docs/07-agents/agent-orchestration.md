# Agent orchestration rules

These rules apply to any orchestrator and subagent setup working on this repository, whether it runs in opencode, Claude Code or another harness. They are tool-agnostic: they name capability tiers, not products. They complement [AGENTS.md](../../AGENTS.md) (mandatory project rules) and [workorders.md](workorders.md) (work order protocol). Where they conflict with AGENTS.md, AGENTS.md wins.

## Model tiers

| Tier | Role | Claude | opencode |
| --- | --- | --- | --- |
| **Top tier** | Most capable and most expensive. Used sparingly. | Claude Opus | The top model configured for the session's provider. With Anthropic models, same as the Claude column. |
| **Medium tier** | Default workhorse. | Claude Sonnet | The medium model configured for the session's provider. With Anthropic models, same as the Claude column. |
| **Light tier** | Cheap and fast, for mechanical work. | Claude Haiku | The light model configured for the session's provider. With Anthropic models, same as the Claude column. |

Assignment:

- **Top tier:** design where no settled spec exists, independent art or game-feel review, and decisions that need judgment.
- **Medium tier:** all execution: implementation, tests, evidence, CI work and docs, plus checking a fix list against a review.
- **Light tier:** searches, lookups and log triage.

Pixel-art work orders are routed by [difficulty](pixel-art-pipeline.md#routing-by-difficulty).

Default to the medium tier. Using the top tier needs a stated reason in the brief. Split a task before routing it: give only its hard core to the top tier and send the surrounding chores (tests, evidence, docs, CI) to the medium tier in a separate brief.

## Standard brief

Every brief states these items, and no more than needed:

- **Goal:** one observable outcome.
- **Read:** the files and sections to read (paths and headings, not whole trees).
- **Constraints:** non-goals, relevant ADRs, files that must not change, and what is already ruled out.
- **Acceptance criteria:** verifiable bullets.
- **Report format:** see [Reports](#reports).

Boilerplate that applies to every brief (an orchestrator may keep it in one reusable file and point briefs at it instead of retyping it):

- Follow [AGENTS.md](../../AGENTS.md): English only, no attribution lines in commits or PRs, no local machine paths, tokens or private information in commits, PR bodies or evidence.
- Work in your own worktree or branch. Push with `git push origin HEAD:refs/heads/<branch>`. Never merge. Never mark a PR ready for review unless the brief says so.
- Before pushing, run the checks the change needs: `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked --all-targets`, and `python3 scripts/validate_assets.py` when art changes. Docs-only changes run `python3 scripts/check_docs.py`.
- Wait for CI as described in [CI waiting](#ci-waiting).
- Do not overwrite a PR body from a shared file. Edit only your own section with `gh pr edit`.
- Art work: record which model designed, executed and reviewed it in the PR's `Rights/Provenance` section, and update `SHA256SUMS` for evidence.
- Long detail (findings, logs, number tables) goes into a file: the PR body, a spec, or a report path named in the brief.

## Reports

A subagent's final report is about 15 lines or fewer: status or verdict, head SHA, CI state, blockers and open questions, and file paths. It contains no file dumps, no pasted logs and no restated brief. Detail goes into a file and the report points to it.

## CI waiting

Wait for CI in the foreground with a blocking command, for example `gh pr checks <n> --watch`, and report once when it finishes. Do not start background monitors or watchers: each one fires its own notification later and adds to the orchestrator's context.

## Reviews

A reviewer writes its full findings to a file and replies with only the verdict, the list of blockers and the file path. Fix lists pass between agents by file path, never pasted inline.

## Top-tier budget

Measured reason: in a recorded run, top-tier art agents used about 450-600k tokens each, and several ran twice. The cost came from long, tool-heavy runs in which every tool call re-reads the whole context. One designer made 41 tool calls and used about 518k tokens, much of it regenerating evidence, updating checksums and waiting on CI. A resumed long-lived agent pays for its entire history on every call.

Rules:

- **Designers (top tier):** produce the key frames or design and the written spec, push once, and stop. Evidence regeneration, contact sheets, checksums, README rows, validators and CI fixes go to the medium tier.
- **Reviews (top tier):** one round per review with a fixed input: the spec section plus at most about 3 images (for example a contact sheet, a scene shot and a GIF). Findings go to a file.
- **Re-review (top tier):** only after a design defect. When a review found execution items only, a medium-tier agent checks the fix list instead.
- **Agent lifetime:** start a fresh, short brief for each review round. Do not resume a long-lived top-tier agent.
- **Gameplay checks:** routine visual and feel checks for gameplay PRs go to the medium tier. The top tier is used only for real feel or design decisions.

### Temporary review gate (owner decision 2026-11-20)

- The top-tier weekly quota is exhausted until 2026-11-25.
- Until then, independent reviews are done by a separate medium-tier agent, never by the PR's author.
- PRs merge on green CI on the final head.
- Every PR merged under this gate owes a top-tier re-review after the reset. Defects found then are fixed in follow-up PRs.
- The PRs that owe a re-review are listed in [the backlog](../04-production/backlog.md#top-tier-re-review-owed-temporary-gate).

## Speed mode

Owner directive, 2026-11-20: optimize the work, merge what you are almost sure works or can be fixed later; intermediate instability is acceptable, the end result must be good. The same rule is recorded in [AGENTS.md](../../AGENTS.md#model-routing-for-art-work).

- **No backlog edits in implementation PRs.** They do not touch `docs/04-production/backlog.md`; status notes go in the PR body (behavior, evidence, adopted defaults, shared-file edits). The orchestrator batch-updates the backlog after merges.
- **Review stays, re-review shrinks.** An independent top-tier review is still required before merge. Non-blocking findings may be fixed forward in a follow-up PR. After a fix list, a medium-tier fix check is enough to merge; a top-tier re-review is needed only when the review found a design or logic defect.
- **Group related stages.** Related stages of one work order may ship as one PR when their dependencies allow it (for example CF02-A+H in [the COMBAT-FUN-02 work order](combat-fun-02-workorder.md#parallel-groups)). The grouped PR keeps the acceptance criteria and fixtures of every stage it contains.

- **What does not change.** Speed mode does not relax the legal, rights and provenance guardrails, green CI on the final head, byte-identical goldens (unless a PR explicitly changes them), flag gating of new behavior, or the owner's game-feel review.

Rationale: serial rebases of the shared backlog file and CI queueing were the bottleneck, not the implementation work. Every PR that edited the backlog conflicted with the previous one and had to rebase and re-run CI; fewer, larger-but-related PRs and no shared hot file remove those cycles. CI-SPEED-01 cuts the other half: documentation-only PRs skip the heavy workflows and the Floor 1 scene workflow compiles its binary once. Game feel is still reviewed by the owner after merge.

**Autonomy (owner, 2026-11-20):** the orchestrator runs every work order already defined in the repo (backlog, work orders, PR sequences) to completion without asking the owner for a go-ahead. When it hits a blocker that is not very big (a design ambiguity, a small scope or sequencing choice, a reversible technical decision), it picks its own recommendation, records it as an adopted default in the PR body (and as data with an ID where the design-defaults rule applies) and continues. It escalates to the owner only for large blockers, legal/rights/provenance questions, contradictions with an accepted ADR, and the owner's game-feel review.

## Reading discipline

Read docs by headings first (`grep -n "^#" <file>`), then read only the sections needed, using offsets and limits. Do not read a large file whole "just in case". Look at images sparingly: a reviewer looks at about 3 images per review, and an orchestrator spot check looks at 1. Pixel-level checks belong to the reviewers, not the orchestrator.
