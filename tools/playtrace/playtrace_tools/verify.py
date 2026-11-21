"""Call the Rust ``playtrace verify`` (the fresh-process replay) for a session.

The exporter never re-implements the replay. A session is exportable only when
the verifier exits 0 and prints ``result=PASS scope=full``. The verifier must
be built from the same source as the recording (PTI-10: ``build_mismatch``
otherwise); pass it with ``--verifier`` or ``PLAYTRACE_BIN``.
"""

from __future__ import annotations

import os
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

_RESULT = re.compile(r"\bresult=(PASS|FAIL)\b")
_FIELD = re.compile(r"\b(scope|code|build|ticks)=(\S+)")
_TIMEOUT_SECONDS = 1800


@dataclass(frozen=True)
class VerifyResult:
    passed: bool
    scope: str | None = None
    build: str | None = None
    code: str | None = None
    reason: str | None = None


Verifier = Callable[[Path], VerifyResult]


def repo_root() -> Path:
    return Path(__file__).resolve().parents[3]


def find_verifier(explicit: str | None) -> Path | None:
    candidates: list[Path] = []
    if explicit:
        candidates.append(Path(explicit))
    elif os.environ.get("PLAYTRACE_BIN"):
        candidates.append(Path(os.environ["PLAYTRACE_BIN"]))
    else:
        root = repo_root()
        candidates += [root / "target/release/playtrace", root / "target/debug/playtrace"]
    for path in candidates:
        if path.is_file() and os.access(path, os.X_OK):
            return path
    return None


def parse_verify_output(stdout: str, returncode: int, stderr: str = "") -> VerifyResult:
    """Interpret ``playtrace verify`` output. Anything but a clean PASS fails."""
    line = next((ln for ln in stdout.splitlines() if ln.startswith("PLAYTRACE_VERIFY ")), "")
    match = _RESULT.search(line)
    fields = dict(_FIELD.findall(line))
    if returncode == 0 and match and match.group(1) == "PASS":
        return VerifyResult(True, scope=fields.get("scope"), build=fields.get("build"))
    if match and match.group(1) == "FAIL":
        reason = re.search(r"reason=(.*)$", line)
        return VerifyResult(
            False,
            code=fields.get("code", "unknown"),
            reason=reason.group(1).strip('"') if reason else None,
        )
    detail = (stderr.strip().splitlines() or ["no verifier result line"])[0]
    return VerifyResult(False, code="verifier_error", reason=detail)


def make_cli_verifier(binary: Path, root: Path) -> Verifier:
    """A verifier that runs ``<binary> verify <id> --dir <base> --root <root>``."""

    def run(session_dir: Path) -> VerifyResult:
        command = [
            str(binary),
            "verify",
            session_dir.name,
            "--dir",
            str(session_dir.parent),
            "--root",
            str(root),
        ]
        try:
            done = subprocess.run(
                command,
                cwd=root,
                capture_output=True,
                text=True,
                timeout=_TIMEOUT_SECONDS,
                check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as error:
            return VerifyResult(False, code="verifier_error", reason=type(error).__name__)
        return parse_verify_output(done.stdout, done.returncode, done.stderr)

    return run
