# ADR-0006: Agent-first code and asset delivery with human feel gates

- **Status:** Accepted process
- **Date:** 2026-11-15

## Context

Production code, assets and documentation will be authored by AI agents (including local LLMs). Unbounded natural-language tasks and undetected style drift are material risks. Type checking alone does not establish whether a game is fun.

## Decision

Every change starts as a bounded `WORK-###` specification, ships with reproducible checks and PR evidence, and must comply with `AGENTS.md`. Use deterministic scenarios + headless unit tests for behavior, screenshots/contact sheets for visual regression, and human gamepad playtests for final quality judgments.

## Consequences

Increased initial documentation and tooling effort; reduced ambiguity, fewer silent regressions and better local-model delegation. If a model cannot render or execute tests, it reports the exact limitation rather than fabricating evidence.
