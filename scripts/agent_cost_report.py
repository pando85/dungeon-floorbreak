#!/usr/bin/env python3
"""Estimate the token cost of Claude Code agents from their transcripts (PIPE-02).

Claude Code stores one JSONL transcript per session at
``~/.claude/projects/<project-slug>/<session-id>.jsonl`` and one per subagent at
``~/.claude/projects/<project-slug>/<session-id>/subagents/agent-<agent-id>.jsonl`` (with an optional
``agent-<agent-id>.meta.json`` next to it). Every API response is written as several lines (one per content
block) that repeat the same ``message.id`` and the same ``usage``, so usage is de-duplicated per message id.

Per agent the report gives the model, tool calls, input / cache-write (5 minute and 1 hour) / cache-read /
output tokens, wall-clock minutes and an estimated cost from ``scripts/agent_rates.json`` (API list prices;
see the caveats in that file).

    python3 scripts/agent_cost_report.py                       # table for every session of this project
    python3 scripts/agent_cost_report.py --session 1a2b3c4d    # one session (id prefix)
    python3 scripts/agent_cost_report.py --agent 5e6f7a8b --work-order DOZ-01 --stage review --csv

``--csv`` appends rows to ``docs/07-agents/cost-ledger.csv``. Rows carry only the columns of the ledger: no
paths, session ids or transcript text. ``--agent-label`` and a ``--mapping`` file (JSON, see
``load_mapping``) supply the human labels; the default label is ``<tier>-<n>``.
"""
from __future__ import annotations

import argparse
import csv
import json
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RATES_PATH = Path(__file__).resolve().parent / "agent_rates.json"
LEDGER_PATH = ROOT / "docs/07-agents/cost-ledger.csv"
LEDGER_COLUMNS = ["date", "work_order", "config", "stage", "tier", "model", "agent_label", "tool_calls",
                  "input", "cache_write", "cache_read", "output", "est_usd", "wall_min", "outcome"]


@dataclass
class Usage:
    input: int = 0
    cache_5m: int = 0
    cache_1h: int = 0
    cache_read: int = 0
    output: int = 0

    def merge_max(self, other: "Usage") -> None:
        """Streamed duplicates of one message: counters only grow, so keep the largest of each."""
        for name in ("input", "cache_5m", "cache_1h", "cache_read", "output"):
            setattr(self, name, max(getattr(self, name), getattr(other, name)))

    def add(self, other: "Usage") -> None:
        for name in ("input", "cache_5m", "cache_1h", "cache_read", "output"):
            setattr(self, name, getattr(self, name) + getattr(other, name))

    @property
    def cache_write(self) -> int:
        return self.cache_5m + self.cache_1h


@dataclass
class AgentReport:
    agent_id: str            # "main" for the session transcript itself
    kind: str                # "main" or "subagent"
    description: str = ""    # from the meta file; shown on stdout only, never written to the ledger
    models: Counter = field(default_factory=Counter)
    tool_calls: int = 0
    usage: Usage = field(default_factory=Usage)
    usd: float = 0.0
    unpriced_models: set = field(default_factory=set)
    first_ts: datetime | None = None
    last_ts: datetime | None = None
    messages: int = 0

    @property
    def model(self) -> str:
        return self.models.most_common(1)[0][0] if self.models else "unknown"

    @property
    def wall_min(self) -> float:
        if self.first_ts is None or self.last_ts is None:
            return 0.0
        return round((self.last_ts - self.first_ts).total_seconds() / 60, 1)

    @property
    def date(self) -> str:
        return self.first_ts.strftime("%Y-%m-%d") if self.first_ts else ""


# ------------------------------------------------------------------ rates

def load_rates(path: Path = RATES_PATH) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def rate_for(model: str, rates: dict) -> dict | None:
    name = model.lower()
    for entry in rates["models"]:
        if entry["match"] in name:
            return entry
    return None


def tier_of(model: str, rates: dict) -> str:
    entry = rate_for(model, rates)
    return entry["tier"] if entry else "unknown"


def message_usd(model: str, usage: Usage, rates: dict) -> float | None:
    entry = rate_for(model, rates)
    if entry is None:
        return None
    mult = rates["cache_write_multiplier"]
    per_token = 1e-6
    return per_token * (usage.input * entry["input"]
                        + usage.cache_5m * entry["input"] * mult["5m"]
                        + usage.cache_1h * entry["input"] * mult["1h"]
                        + usage.cache_read * entry["cache_read"]
                        + usage.output * entry["output"])


# ------------------------------------------------------------------ transcript parsing

def _int(value) -> int:
    return value if isinstance(value, int) and value > 0 else 0


def usage_from(raw: dict) -> Usage:
    """Read one ``message.usage`` object. Without a 5m/1h split all cache writes count as 5 minute writes."""
    total_write = _int(raw.get("cache_creation_input_tokens"))
    split = raw.get("cache_creation") if isinstance(raw.get("cache_creation"), dict) else None
    if split and (_int(split.get("ephemeral_5m_input_tokens")) or _int(split.get("ephemeral_1h_input_tokens"))):
        c5, c1 = _int(split.get("ephemeral_5m_input_tokens")), _int(split.get("ephemeral_1h_input_tokens"))
    else:
        c5, c1 = total_write, 0
    return Usage(input=_int(raw.get("input_tokens")), cache_5m=c5, cache_1h=c1,
                 cache_read=_int(raw.get("cache_read_input_tokens")), output=_int(raw.get("output_tokens")))


def _parse_ts(text) -> datetime | None:
    if not isinstance(text, str):
        return None
    try:
        return datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError:
        return None


def parse_transcript(path: Path, agent_id: str, kind: str, rates: dict, description: str = "") -> AgentReport:
    """Aggregate one JSONL transcript. Unreadable lines are skipped."""
    report = AgentReport(agent_id=agent_id, kind=kind, description=description)
    per_message: dict[str, tuple[str, Usage]] = {}
    tool_ids: set[str] = set()
    anonymous_tools = 0
    with path.open(encoding="utf-8") as handle:
        for line in handle:
            try:
                row = json.loads(line)
            except ValueError:
                continue
            if not isinstance(row, dict):
                continue
            ts = _parse_ts(row.get("timestamp"))
            if ts is not None:
                report.first_ts = ts if report.first_ts is None or ts < report.first_ts else report.first_ts
                report.last_ts = ts if report.last_ts is None or ts > report.last_ts else report.last_ts
            if row.get("type") != "assistant" or not isinstance(row.get("message"), dict):
                continue
            message = row["message"]
            model = message.get("model") or "unknown"
            if model.startswith("<") or not isinstance(message.get("usage"), dict):
                continue    # synthetic placeholders carry no billed usage
            for block in message.get("content") or []:
                if isinstance(block, dict) and block.get("type") == "tool_use":
                    if block.get("id"):
                        tool_ids.add(block["id"])
                    else:
                        anonymous_tools += 1
            key = message.get("id") or row.get("requestId") or row.get("uuid")
            usage = usage_from(message["usage"])
            if key in per_message:
                per_message[key][1].merge_max(usage)
            else:
                per_message[key] = (model, usage)
    report.tool_calls = len(tool_ids) + anonymous_tools
    report.messages = len(per_message)
    for model, usage in per_message.values():
        report.models[model] += 1
        report.usage.add(usage)
        usd = message_usd(model, usage, rates)
        if usd is None:
            report.unpriced_models.add(model)
        else:
            report.usd += usd
    return report


# ------------------------------------------------------------------ discovery

def project_slug(path: Path) -> str:
    """Claude Code's project directory name: every non-alphanumeric character becomes a dash."""
    text = str(path.resolve())
    text = text.split("/.claude/worktrees/")[0]
    return re.sub(r"[^A-Za-z0-9-]", "-", text)


def discover(project_dir: Path, session: str | None = None) -> list[tuple[str, str, Path, str]]:
    """Return ``(agent_id, kind, transcript path, description)``: each session's main transcript, then its subagents."""
    found = []
    for main in sorted(project_dir.glob("*.jsonl")):
        if session and not main.stem.startswith(session):
            continue
        found.append(("main", "main", main, ""))
        sub_dir = project_dir / main.stem / "subagents"
        for sub in sorted(sub_dir.glob("agent-*.jsonl")) if sub_dir.is_dir() else []:
            agent_id = sub.stem[len("agent-"):]
            description = ""
            meta = sub.with_suffix(".meta.json")
            if meta.is_file():
                try:
                    description = str(json.loads(meta.read_text(encoding="utf-8")).get("description", ""))
                except ValueError:
                    pass
            found.append((agent_id, "subagent", sub, description))
    return found


def load_mapping(path: Path | None) -> dict:
    """Mapping file: ``{"<agent id or prefix>": {"work_order": ..., "config": ..., "stage": ..., "outcome": ...,
    "agent_label": ...}, "*": {defaults}}``. ``main`` addresses the orchestrator session."""
    if path is None:
        return {}
    return json.loads(path.read_text(encoding="utf-8"))


def mapped(mapping: dict, agent_id: str) -> dict:
    out = dict(mapping.get("*", {}))
    for key, value in mapping.items():
        if key != "*" and (agent_id == key or (key != "main" and agent_id.startswith(key))):
            out.update(value)
    return out


# ------------------------------------------------------------------ output

def format_table(reports: list[AgentReport], rates: dict) -> str:
    header = ["agent", "kind", "tier", "model", "calls", "input", "cache_w_5m", "cache_w_1h", "cache_read",
              "output", "est_usd", "wall_min", "description"]
    rows = [header]
    for r in reports:
        rows.append([r.agent_id[:12], r.kind, tier_of(r.model, rates), r.model, str(r.tool_calls),
                     str(r.usage.input), str(r.usage.cache_5m), str(r.usage.cache_1h), str(r.usage.cache_read),
                     str(r.usage.output), f"{r.usd:.2f}", f"{r.wall_min:.1f}", r.description[:50]])
    widths = [max(len(row[i]) for row in rows) for i in range(len(header))]
    lines = ["  ".join(cell.ljust(w) if i in (0, 1, 2, 3, 12) else cell.rjust(w)
                       for i, (cell, w) in enumerate(zip(row, widths))).rstrip() for row in rows]
    total = sum(r.usd for r in reports)
    lines.append(f"total est_usd {total:.2f} (API list-price estimate, not a billed amount)")
    unpriced = sorted({m for r in reports for m in r.unpriced_models})
    if unpriced:
        lines.append("unpriced models (counted as 0): " + ", ".join(unpriced))
    return "\n".join(lines)


def ledger_row(report: AgentReport, rates: dict, meta: dict, ordinal: int) -> dict:
    tier = tier_of(report.model, rates)
    return {
        "date": report.date, "work_order": meta.get("work_order", ""), "config": meta.get("config", ""),
        "stage": meta.get("stage", ""), "tier": tier, "model": report.model,
        "agent_label": meta.get("agent_label") or f"{tier}-{ordinal}",
        "tool_calls": report.tool_calls, "input": report.usage.input, "cache_write": report.usage.cache_write,
        "cache_read": report.usage.cache_read, "output": report.usage.output,
        "est_usd": f"{report.usd:.2f}", "wall_min": f"{report.wall_min:.1f}", "outcome": meta.get("outcome", ""),
    }


def append_ledger(rows: list[dict], path: Path, force: bool = False) -> int:
    """Append rows (writing the header if the file is new or empty). Rows whose (date, work_order, config,
    stage, agent_label) already exist are skipped unless ``force``. Returns the number of rows written."""
    existing: set[tuple] = set()
    needs_header = not path.is_file() or path.stat().st_size == 0
    if not needs_header:
        with path.open(encoding="utf-8", newline="") as handle:
            reader = csv.DictReader(handle)
            if reader.fieldnames != LEDGER_COLUMNS:
                raise ValueError(f"{path.name}: unexpected header {reader.fieldnames}")
            existing = {(r["date"], r["work_order"], r["config"], r["stage"], r["agent_label"]) for r in reader}
    written = 0
    with path.open("a", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=LEDGER_COLUMNS, lineterminator="\n")
        if needs_header:
            writer.writeheader()
        for row in rows:
            key = (row["date"], row["work_order"], row["config"], row["stage"], row["agent_label"])
            if key in existing and not force:
                print(f"skip duplicate ledger row {key[:4]} {key[4]} (use --force)", file=sys.stderr)
                continue
            writer.writerow(row)
            written += 1
    return written


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description="Estimate Claude Code agent token cost from transcripts")
    p.add_argument("--projects-root", type=Path, default=Path.home() / ".claude" / "projects")
    p.add_argument("--project", help="project slug or repository path (default: this repository)")
    p.add_argument("--session", help="only sessions whose id starts with this prefix")
    p.add_argument("--agent", action="append", default=[], help="only agents whose id starts with this (repeatable); 'main' = orchestrator")
    p.add_argument("--no-main", action="store_true", help="leave out the session (orchestrator) transcripts")
    p.add_argument("--rates", type=Path, default=RATES_PATH)
    p.add_argument("--csv", action="store_true", help="append rows to the cost ledger")
    p.add_argument("--ledger", type=Path, default=LEDGER_PATH)
    p.add_argument("--force", action="store_true", help="append even if an identical ledger key exists")
    p.add_argument("--mapping", type=Path, help="JSON file mapping agent ids to ledger metadata")
    p.add_argument("--work-order", default="")
    p.add_argument("--config", default="")
    p.add_argument("--stage", default="")
    p.add_argument("--outcome", default="")
    p.add_argument("--agent-label", default="", help="label for the selected agent (single agent only)")
    args = p.parse_args(argv)

    rates = load_rates(args.rates)
    target = args.project or str(Path.cwd())
    slug = project_slug(Path(target)) if "/" in target else target
    project_dir = args.projects_root / slug
    if not project_dir.is_dir():
        print(f"error: no transcript directory for the project (looked under {args.projects_root})", file=sys.stderr)
        return 1
    reports = []
    for agent_id, kind, path, description in discover(project_dir, args.session):
        if args.no_main and kind == "main":
            continue
        if args.agent and not any(agent_id.startswith(a) for a in args.agent):
            continue
        reports.append(parse_transcript(path, agent_id, kind, rates, description))
    reports = [r for r in reports if r.messages]
    if not reports:
        print("no agent transcripts with usage found", file=sys.stderr)
        return 1
    print(format_table(reports, rates))
    if args.csv:
        mapping = load_mapping(args.mapping)
        defaults = {k: v for k, v in (("work_order", args.work_order), ("config", args.config),
                                      ("stage", args.stage), ("outcome", args.outcome)) if v}
        if args.agent_label:
            if len(reports) != 1:
                print("error: --agent-label needs exactly one selected agent", file=sys.stderr)
                return 1
            defaults["agent_label"] = args.agent_label
        counters: Counter = Counter()
        rows = []
        for r in reports:
            meta = {**defaults, **mapped(mapping, r.agent_id)}
            counters[tier_of(r.model, rates)] += 1
            rows.append(ledger_row(r, rates, meta, counters[tier_of(r.model, rates)]))
        written = append_ledger(rows, args.ledger, args.force)
        print(f"appended {written} row(s) to {args.ledger.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
