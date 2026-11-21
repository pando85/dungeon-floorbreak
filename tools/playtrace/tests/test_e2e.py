"""End to end with the real Rust ``playtrace`` binary (``PLAYTRACE_BIN``).

The sessions are written by ``playtrace synth-fixture``: fixture-scripted, no
live input, no human, labelled synthetic, but recorded by the real recorder and
verified by the real fresh-process verifier. Skipped when the binary is not
built (CI sets ``PLAYTRACE_REQUIRE_E2E=1`` so a missing binary fails instead).
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
from collections import Counter
from pathlib import Path

import pytest

from playtrace_tools import schema
from playtrace_tools.cli import main
from playtrace_tools.dataset import check_against_sources, load_dataset
from playtrace_tools.export import Options, export_dataset
from playtrace_tools.journal import fnv1a64, read_session
from playtrace_tools.verify import find_verifier, make_cli_verifier, repo_root


@pytest.fixture
def verifier(rust_sessions):
    binary = find_verifier(os.environ.get("PLAYTRACE_BIN"))
    assert binary is not None
    return make_cli_verifier(binary, repo_root())


def test_the_real_verifier_passes_fixture_sessions_and_the_export_checks_out(rust_sessions, verifier, tmp_path):
    options = Options(kind="transitions", allow_synthetic=True, context=2)
    meta = export_dataset(rust_sessions, tmp_path / "ds", options, verifier)
    assert meta["synthetic"] is True and not meta["rejected"]
    assert len(meta["sessions"]) == 2
    assert all(s["verify"] == {"result": "PASS", "scope": "full", "build": "identical"} for s in meta["sessions"])
    dataset = load_dataset(Path(meta["path"]))
    report = check_against_sources(dataset, rust_sessions)
    assert report["rows_checked"] > 600
    # both episodes (a restart in the middle) and a final observation at the boundary
    kinds = Counter(r["next_kind"] for s in dataset.streams.values() for r in s.rows)
    assert kinds["final_obs"] >= 2 and kinds["step"] > 500
    # the human and the AI stream both exist (the solo-swap fixture has an AI hero)
    assert dataset.streams["human"].rows and dataset.streams["ai"].rows
    assert meta["leakage"]["units_in_several_splits"] == 0


def test_without_allow_synthetic_the_fixture_is_refused(rust_sessions, verifier, tmp_path):
    meta = export_dataset(rust_sessions, tmp_path / "ds", Options(kind="bc", actor="human"), verifier)
    assert not meta["sessions"]
    assert {r["reason"] for r in meta["rejected"]} == {"synthetic_not_allowed"}


def test_rust_stats_agree_with_the_exported_histogram(rust_sessions, verifier, tmp_path):
    meta = export_dataset(
        rust_sessions, tmp_path / "ds", Options(kind="bc", actor="human", allow_synthetic=True), verifier
    )
    binary = os.environ["PLAYTRACE_BIN"]
    done = subprocess.run(
        [binary, "stats", "--dir", str(rust_sessions)], check=True, capture_output=True, text=True
    )
    totals: Counter = Counter()
    for line in done.stdout.splitlines():
        if "class=human" in line:
            for key, value in re.findall(r"(\w+)=(\d+)", line):
                if key in ("ticks", "left", "neutral", "right", "jump", "attack", "physical", "ability", "down"):
                    totals[key] += int(value)
    hist = meta["streams"]["human"]["action_histogram"]
    assert totals["ticks"] == hist["rows"]
    assert (totals["left"], totals["neutral"], totals["right"]) == (
        hist["horizontal"]["left"],
        hist["horizontal"]["neutral"],
        hist["horizontal"]["right"],
    )
    assert totals["jump"] == hist["jump"] and totals["attack"] == hist["primary"]
    assert totals["physical"] == hist["physical"] and totals["down"] == hist["down"]


def _reseal_with_edit(session: Path, edit) -> None:
    """Edit one tick and re-seal chunk trailers and the manifest, so only the
    replay (not a checksum) can notice."""
    loaded = read_session(session)
    manifest = json.loads((session / "manifest.json").read_text())
    ticks = [t.raw for t in loaded.ticks]
    edit(ticks)
    checksums = []
    for index in range(len(manifest["chunk_checksums"])):
        chunk = ticks[index * 600 : (index + 1) * 600]
        lines = [json.dumps(t, separators=(",", ":")) for t in chunk]
        body = "".join(line + "\n" for line in lines).encode()
        digest = f"{fnv1a64(body):016x}"
        trailer = {
            "end": True,
            "chunk": index,
            "ticks": len(chunk),
            "first_t": chunk[0]["t"],
            "last_t": chunk[-1]["t"],
            "fnv": digest,
        }
        (session / f"ticks-{index:06d}.jsonl").write_bytes(
            body + json.dumps(trailer, separators=(",", ":")).encode() + b"\n"
        )
        checksums.append({"chunk": index, "ticks": len(chunk), "fnv": digest})
    manifest["chunk_checksums"] = checksums
    (session / "manifest.json").write_text(json.dumps(manifest))


def test_a_resealed_tampered_session_is_rejected_by_the_real_verifier(rust_sessions, verifier, tmp_path):
    store = tmp_path / "store"
    store.mkdir()
    victim = sorted(rust_sessions.iterdir())[0]
    shutil.copytree(victim, store / victim.name)

    def flip(ticks):
        ticks[50]["hero"][0][1] ^= 1  # change a recorded input bit after the fact

    _reseal_with_edit(store / victim.name, flip)
    # The Python re-check passes (the file is internally consistent)...
    assert read_session(store / victim.name).ticks
    # ...but the replay does not, and the exporter says why.
    meta = export_dataset(store, tmp_path / "ds", Options(kind="bc", actor="human", allow_synthetic=True), verifier)
    assert not meta["sessions"]
    (rejection,) = meta["rejected"]
    assert rejection["reason"].startswith("verify_failed:")


def test_cli_export_check_and_bc_smoke_end_to_end(rust_sessions, tmp_path, capsys):
    out = tmp_path / "ds"
    binary = os.environ["PLAYTRACE_BIN"]
    code = main(
        [
            "export-bc",
            "--dir",
            str(rust_sessions),
            "--out",
            str(out),
            "--allow-synthetic",
            "--verifier",
            binary,
        ]
    )
    text = capsys.readouterr().out
    assert code == 0 and "PLAYTRACE_EXPORT " in text
    dataset_dir = Path(re.search(r"PLAYTRACE_EXPORT_DIR (.+)", text).group(1))
    assert main(["check-dataset", str(dataset_dir), "--dir", str(rust_sessions)]) == 0
    assert "result=PASS" in capsys.readouterr().out
    report_path = tmp_path / "bc.json"
    assert main(["bc-smoke", str(dataset_dir), "--epochs", "3", "--report", str(report_path)]) == 0
    report = json.loads(report_path.read_text())
    assert report["synthetic"] is True
    assert report["dev"]["rows"] > 0
    assert schema.OBS_LEN == report["feature_width"]
