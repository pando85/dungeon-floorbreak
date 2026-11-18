# Pixel-art production pipeline and routing experiment (Floor 01)

Owner-approved plan, 2026-11-18. It refines [agent-orchestration.md](agent-orchestration.md) and the "Model routing for art work" section of [AGENTS.md](../../AGENTS.md) without changing them: Opus designs, Sonnet executes, a separate Opus reviews, the owner reviews at the end. Harness: **Claude Code** (subagents).

## 1. Production order (Floor 01, by first appearance)

| # | Batch | Characters | Environment |
| --- | --- | --- | --- |
| 0 | Close-out | DOZ-01 review (PR #47) | ENV-02 review (PR #48) |
| 1 | Opening | Goblin Engineer standalone (delta of GOB-01 / DOZ-01 rider) | — |
| 2 | Guild | — | AREA-REFUGE-01 |
| 3 | Llamas | **Bad Llama** + spit VFX, **Dungeon Rat** | AREA-HEAT-01 |
| 4 | Restaurant | — | AREA-FASTFOOD-01 |
| 5 | Infestation | **Scatterer** + Brood Guardian, The Hoarder (boss) | AREA-HOARDER-01 |
| 6 | Workshop | Bomb Bard, Copper Chopper | AREA-GOBLINWORKSHOP-01 |
| 7 | Hazards | Green Slime, Rot Sticker | AREA-HAZARDS-01 |
| 8 | Rings | — | AREA-RINGS-01 |
| 9 | Gym | Troglodyte Pygmy / Basher / Virtuoso, The Juicer | AREA-GYM-01 |
| 10 | Arena | Ball of Swine + Tusklings | AREA-ARENA-01 (geometry concept first) |

**Scope per enemy: readability set** until that enemy has a gameplay work order: `idle` (2–4), `move` (4–6), `tell` (1–2), `attack` (2–3), `hurt` (1), `defeat` (2–3); about 12–20 frames. Full sets come later with gameplay. This respects the combat-first rule (no large bestiary before direct-hit feel has human evidence) while art keeps moving.

## 2. Pipeline stages

| Stage | Tier | Output | Rule |
| --- | --- | --- | --- |
| P Packet | script (+ Haiku/Sonnet) | `pixelart packet <asset>`: concept crops, quantized palette candidates, scale lineup (Carl 52, Donut 40, goblin 36), frame/pivot template, doc excerpts (≤ 2 pages), pre-filled spec template | Designer starts without exploring the repo |
| D Design | Opus, fresh | per routing config (section 4) | ≤ 20 tool calls, one push, stop. Families are designed in one session |
| E Execute | Sonnet | in-betweens, `.ase`/grid, manifest, tests, evidence via the toolkit | no bespoke per-asset preview/validator scripts |
| G Gate | no model | validators pass + review pack of exactly 3 images | Opus review is not spawned before G passes |
| R Review | Opus, fresh, one round | verdict file; input = spec + 3 images + validator report | execution findings → Sonnet checks the fix list; only design defects return to Opus |
| Chores | Haiku | checksums, README rows, CI log triage, PR status | |
| O Owner | human | batch summary after merge | since 2026-11-18 the orchestrator merges pixel-art PRs that passed R and CI (see AGENTS.md, merge delegation); the owner can reopen any asset |

## 3. Shared toolkit and cost ledger (PIPE-01, PIPE-02)

- `scripts/pixelart/`: packet generator, spec template, parametrized validators (palette count, orphans/isolated pixels, pivot and contact rows, frozen keys, tick table), evidence (contact sheet, per-tag GIF at tick timing, lit scene composition at 640×360 and 1920×1080) and the 3-image review pack. Existing per-asset scripts stay; new assets use the toolkit.
- `scripts/agent_cost_report.py`: reads Claude Code session and subagent transcripts (`usage` per assistant message), groups by agent, prints tier, model, tool calls, input / cache-write / cache-read / output tokens, estimated USD and wall-clock. Committed results go to `docs/07-agents/cost-ledger.csv` with no local paths or session text.
- Rates (API list prices, USD per MTok, from the Claude API reference cached 2026-11-03; cache writes 1.25× input for 5 min, 2× for 1 h): Opus 5.5 4 / 20, cache read 0.20; Sonnet 5.5 2 / 10, cache read 0.10; Haiku 5.5 0.10 / 0.50, cache read 0.01 (unverified, assumed 0.1×). On a subscription these are API-equivalent estimates, not billed amounts.
- Baseline: the documented 450–600k tokens per Opus art agent (no local transcript history of earlier runs exists).

## 4. Routing experiment: three configurations on the next three monsters

No asset is produced twice. Each of the next three monsters gets one configuration:

| Monster | Config | Opus design role | Sonnet role before execution |
| --- | --- | --- | --- |
| Bad Llama | **A Lean Opus** | size, palette, all keys (one per readability tag, ~6–8) and the spec | packet only |
| Dungeon Rat | **B Opus as editor** | fresh Opus corrects Sonnet's draft keys pixel-level, finalizes and freezes them, fixes the spec | packet, size block-ins, palette and draft keys + draft spec from the template |
| Scatterer | **C Opus spec only** | spec (size, palette ramps, per-pose joint/silhouette notes, timing) + 1 hero key (`idle` 0) | packet only; after design Sonnet draws all other keys |

All three then follow E → G → R → O identically. The Bad Llama gets A because it is the largest and first combat-relevant of the three.

**Confounds:** the monsters differ in size and complexity. Compare normalized metrics, not raw totals.

**Blind review rubric** (reviewer sees spec + 3 images + validator report, never the config or PR body), each 1–5:

1. Silhouette and readability at 1× in the lit Floor 1 scene next to Carl and Donut.
2. Fidelity to the approved concept and to the antihero tone.
3. Palette discipline and fit with the A1 lighting.
4. Animation clarity: tell → attack readable, timing, weight.
5. Technical cleanliness: pivots, contact rows, orphans, consistency between frames.

**Metrics per monster:** Opus tokens and USD, total USD, USD per accepted frame, review loops to PASS, blocker count, rubric total, owner verdict (accept / rework) at the batch review.

**Decision rule:** pick the cheapest configuration whose rubric total is within 2 points of A and that passed in at most one review loop. Then route:

- New boss (Hoarder, Juicer, Ball of Swine) → A.
- New regular enemy → the winner.
- Derived asset (Brood Guardian, Bomb Bard, Tusklings, Troglodyte variants) and kit-extension environments → C, or Sonnet-only with an Opus review if C wins.

Re-check the routing with the ledger after three more assets.
