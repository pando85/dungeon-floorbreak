# Work order protocol for local LLMs

## Operating principle

Code is created by local agents; a human sets priorities and evaluates play feel, design coherence and public-release decisions. Use one explicit work order per PR. An agent may use tools/models suited to coding or art, but the engineering contract is model-neutral; art design and art review follow the model routing rule in [AGENTS.md](../../AGENTS.md#model-routing-for-art-work).

## Prompt template

```text
You are the implementation agent for Dungeon Floorbreak.
Read AGENTS.md and all documentation linked for the selected WORK-###.
Task: [WORK-### + title]
Goal: [one observable player or tooling behavior]
Constraints: [non-goals and relevant ADRs]
Files/contracts expected to change: [...]
Acceptance criteria: [verifiable bullets copied from backlog/spec]
Evidence: [tests, logs, replay scenario, screenshot/GIF if visual]
Rules: English only; no unauthorized franchise assets; no fabricated execution;
       smallest useful PR; preserve deterministic gameplay and contracts.
Before coding, return a 5-step plan and identify contradictions.
Then implement, run checks and describe failures precisely.
```

## PR template

Include: `Why`, `Change`, `Acceptance Criteria`, `Commands & Results`, `Visual Evidence`, `Performance Data`, `Rights/Provenance`, `Risks`, `Next PR`. If work cannot finish, create a clear partial PR with failing tests and a reproducible blocker; never mark incomplete tasks as delivered.

## Agent coordination

Do not allow two agents to rewrite the same canonical sprite, combat schema or terrain grid type concurrently. Integration owner controls schema migrations and dependency changes. Parallelize on separate artifacts with stable contracts (e.g., game input, art export validation, LDtk schema), not speculative interfaces. Keep PRs reviewable: aim one feature/subsystem and tests each.

## Quality gates

**Static:** cargo fmt, clippy, unit test, docs lint, JSON schema. **Behavioral:** deterministic scenario. **Visual:** short controlled capture/contact sheet. **Human:** controller-play review, mood and humor. Agent reasoning is not an acceptable substitute for evidence.

## ADR discipline

Create a new `ADR-XXXX` when changing engine, destructive-terrain representation, camera model, network strategy, IP packaging or rendering source conventions. ADRs must state decision, alternatives, why, consequences and reversible/irreversible aspects.
