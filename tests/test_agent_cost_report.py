"""PIPE-02: the agent cost report on a small synthetic transcript (no real session data)."""
from __future__ import annotations

import csv
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import agent_cost_report as acr  # noqa: E402

FIXTURE = ROOT / "tests/fixtures/agent_transcript_synthetic.jsonl"


class CostReportTests(unittest.TestCase):
    def setUp(self) -> None:
        self.rates = acr.load_rates()
        self.tmp = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)

    def project(self) -> Path:
        """A fake project directory: one main transcript and one subagent with a meta file."""
        proj = self.tmp / "proj"
        (proj / "sess1" / "subagents").mkdir(parents=True)
        shutil.copy(FIXTURE, proj / "sess1.jsonl")
        shutil.copy(FIXTURE, proj / "sess1" / "subagents" / "agent-abc123.jsonl")
        (proj / "sess1" / "subagents" / "agent-abc123.meta.json").write_text(
            json.dumps({"description": "synthetic helper"}), encoding="utf-8")
        return proj

    def test_dedupes_message_ids_and_counts_tokens(self) -> None:
        r = acr.parse_transcript(FIXTURE, "abc", "subagent", self.rates)
        self.assertEqual(r.messages, 3)                      # msg_1 appears on two lines; the synthetic one is skipped
        self.assertEqual(r.tool_calls, 3)
        self.assertEqual((r.usage.input, r.usage.output), (16, 80))   # output is the largest streamed value
        self.assertEqual((r.usage.cache_5m, r.usage.cache_1h), (1200, 500))   # no split on msg_4: 5 minute
        self.assertEqual(r.usage.cache_read, 2500)
        self.assertEqual(r.model, "claude-sonnet-5-5")
        self.assertEqual(r.wall_min, 6.0)
        self.assertEqual(r.date, "2026-11-18")

    def test_estimated_cost_uses_the_rate_file(self) -> None:
        r = acr.parse_transcript(FIXTURE, "abc", "subagent", self.rates)
        # Sonnet 2 / 10, cache read 0.10, writes 1.25x (5m) and 2x (1h)
        want = (16 * 2 + 1200 * 2 * 1.25 + 500 * 2 * 2 + 2500 * 0.10 + 80 * 10) / 1e6
        self.assertAlmostEqual(r.usd, want, places=9)
        self.assertEqual(acr.tier_of("claude-opus-5-5", self.rates), "top")
        self.assertEqual(acr.tier_of("claude-haiku-5-5", self.rates), "light")
        self.assertEqual(acr.tier_of("mystery-model", self.rates), "unknown")
        self.assertIsNone(acr.message_usd("mystery-model", acr.Usage(input=1), self.rates))

    def test_rate_file_has_the_plan_values_and_caveats(self) -> None:
        by = {m["match"]: m for m in self.rates["models"]}
        self.assertEqual((by["opus"]["input"], by["opus"]["output"], by["opus"]["cache_read"]), (4.0, 20.0, 0.20))
        self.assertEqual((by["sonnet"]["input"], by["sonnet"]["output"], by["sonnet"]["cache_read"]), (2.0, 10.0, 0.10))
        self.assertEqual((by["haiku"]["input"], by["haiku"]["output"], by["haiku"]["cache_read"]), (0.10, 0.50, 0.01))
        self.assertFalse(by["haiku"]["verified"])
        self.assertEqual(self.rates["cache_write_multiplier"], {"5m": 1.25, "1h": 2.0})
        self.assertTrue(any("subscription" in c for c in self.rates["caveats"]))

    def test_unknown_model_is_reported_not_priced(self) -> None:
        path = self.tmp / "u.jsonl"
        path.write_text(json.dumps({"type": "assistant", "timestamp": "2026-11-18T10:00:00Z", "message": {
            "id": "m", "model": "other-model", "content": [], "usage": {"input_tokens": 5, "output_tokens": 5}}}) + "\n",
            encoding="utf-8")
        r = acr.parse_transcript(path, "x", "subagent", self.rates)
        self.assertEqual(r.usd, 0.0)
        self.assertEqual(r.unpriced_models, {"other-model"})

    def test_discovery_and_table(self) -> None:
        found = acr.discover(self.project())
        self.assertEqual([(a, k) for a, k, _p, _d in found], [("main", "main"), ("abc123", "subagent")])
        self.assertEqual(found[1][3], "synthetic helper")
        reports = [acr.parse_transcript(p, a, k, self.rates, d) for a, k, p, d in found]
        table = acr.format_table(reports, self.rates)
        self.assertIn("abc123", table)
        self.assertIn("total est_usd", table)

    def test_project_slug_matches_claude_code_naming(self) -> None:
        self.assertEqual(acr.project_slug(Path("/home/user/my.repo/.claude/worktrees/agent-1")), "-home-user-my-repo")

    def test_csv_ledger_rows_have_only_ledger_columns_and_no_ids_or_text(self) -> None:
        proj = self.project()
        reports = [acr.parse_transcript(p, a, k, self.rates, d) for a, k, p, d in acr.discover(proj)]
        mapping = {"main": {"stage": "orchestration"}, "abc": {"stage": "execute", "agent_label": "helper"},
                   "*": {"work_order": "TEST-01", "config": "A"}}
        rows = [acr.ledger_row(r, self.rates, {**acr.mapped(mapping, r.agent_id)}, i + 1) for i, r in enumerate(reports)]
        ledger = self.tmp / "ledger.csv"
        self.assertEqual(acr.append_ledger(rows, ledger), 2)
        self.assertEqual(acr.append_ledger(rows, ledger), 0, "identical keys are not appended twice")
        self.assertEqual(acr.append_ledger(rows, ledger, force=True), 2)
        with ledger.open(encoding="utf-8", newline="") as handle:
            data = list(csv.DictReader(handle))
        self.assertEqual(list(data[0]), acr.LEDGER_COLUMNS)
        self.assertEqual(len(data), 4)
        self.assertEqual((data[1]["agent_label"], data[1]["stage"], data[1]["work_order"]), ("helper", "execute", "TEST-01"))
        self.assertEqual(data[0]["agent_label"], "medium-1")
        text = ledger.read_text(encoding="utf-8")
        for secret in ("synthetic", "abc123", "sess1", "/home", str(self.tmp)):
            self.assertNotIn(secret, text)

    def test_committed_ledger_has_the_header(self) -> None:
        first = (ROOT / "docs/07-agents/cost-ledger.csv").read_text(encoding="utf-8").splitlines()[0]
        self.assertEqual(first, ",".join(acr.LEDGER_COLUMNS))

    def test_ledger_refuses_a_foreign_header(self) -> None:
        bad = self.tmp / "bad.csv"
        bad.write_text("a,b\n", encoding="utf-8")
        with self.assertRaises(ValueError):
            acr.append_ledger([{c: "" for c in acr.LEDGER_COLUMNS}], bad)


if __name__ == "__main__":
    unittest.main()
