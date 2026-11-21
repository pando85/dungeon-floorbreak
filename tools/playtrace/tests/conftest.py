"""Shared fixtures. Every session here is SYNTHETIC (see ``synthetic_journal``).

The pure-Python sessions use a stub verifier. The ``rust_sessions`` fixture uses
the real ``playtrace`` binary (``PLAYTRACE_BIN``) to write fixture-scripted
sessions that genuinely pass ``playtrace verify``; it skips when the binary is
not built, unless ``PLAYTRACE_REQUIRE_E2E=1`` (CI), which makes that a failure.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from playtrace_tools.verify import VerifyResult, repo_root  # noqa: E402
from synthetic_journal import write_session  # noqa: E402


def pass_verifier(_path: Path) -> VerifyResult:
    return VerifyResult(True, scope="full", build="identical")


@pytest.fixture
def base(tmp_path: Path) -> Path:
    directory = tmp_path / "sessions"
    directory.mkdir()
    return directory


@pytest.fixture
def out(tmp_path: Path) -> Path:
    return tmp_path / "datasets"


@pytest.fixture
def five_sessions(base: Path) -> Path:
    """Five synthetic sessions: one 1P human Carl + AI Donut, then variants."""
    write_session(base, "a" * 32, seed=1)
    write_session(base, "b" * 32, seed=2, src=("companion_ai", "human_p1"))
    write_session(base, "c" * 32, seed=3, src=("human_p1", "human_p2"), defeat_at=90)
    write_session(base, "d" * 32, seed=4, end_open=True)
    write_session(base, "e" * 32, seed=5, status="abandoned", restart_at=None)
    return base


@pytest.fixture(scope="session")
def rust_sessions(tmp_path_factory: pytest.TempPathFactory) -> Path:
    binary = os.environ.get("PLAYTRACE_BIN")
    if not binary or not Path(binary).is_file():
        if os.environ.get("PLAYTRACE_REQUIRE_E2E") == "1":
            pytest.fail("PLAYTRACE_REQUIRE_E2E=1 but PLAYTRACE_BIN is not a built playtrace binary")
        pytest.skip("PLAYTRACE_BIN is not set: no real-verifier e2e")
    directory = tmp_path_factory.mktemp("rust_sessions")
    root = repo_root()
    for fixture, ticks in (("duo_independent_moves", "400"), ("solo_swap_continuity", "300")):
        subprocess.run(
            [binary, "synth-fixture", "--dir", str(directory), "--fixture", fixture, "--ticks", ticks, "--root", str(root)],
            check=True,
            cwd=root,
            capture_output=True,
        )
    return directory
