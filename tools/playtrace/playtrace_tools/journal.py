"""Read and validate a ``playtrace-session/1`` journal (read-only).

This is an independent second check on top of ``playtrace verify``: it parses
the manifest, ``control.jsonl`` and the ``ticks-NNNNNN.jsonl`` chunks, checks
every chunk trailer checksum (FNV-1a-64), the manifest's chunk table, the
contiguity of the recorder tick ``t`` from 0 and the control-stream integrity
record. It never writes to a session and never opens ``diagnostics.jsonl``
(privileged diagnostics are not an exporter input).
"""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

SESSION_SCHEMA = "playtrace-session/1"
FNV_OFFSET = 0xCBF29CE484222325
FNV_PRIME = 0x100000001B3
MASK64 = 0xFFFFFFFFFFFFFFFF
_CHUNK_RE = re.compile(r"^ticks-(\d{6})\.jsonl$")


class JournalError(Exception):
    """A session cannot be read or fails a structural check.

    ``code`` is a stable reason code (``invalid_journal``, ``unsupported_schema``...).
    """

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message


def fnv1a64(data: bytes, state: int = FNV_OFFSET) -> int:
    for byte in data:
        state = ((state ^ byte) * FNV_PRIME) & MASK64
    return state


@dataclass
class Tick:
    """One recorded tick with where it came from in the source files."""

    raw: dict[str, Any]
    chunk: int
    line: int  # zero-based line index inside the chunk file

    @property
    def t(self) -> int:
        return self.raw["t"]


@dataclass
class Session:
    path: Path
    manifest: dict[str, Any]
    control: list[dict[str, Any]]
    ticks: list[Tick]
    episode_ends: dict[int, dict[str, Any]] = field(default_factory=dict)

    @property
    def session_id(self) -> str:
        return self.manifest["session_id"]


def _read_lines(path: Path) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise JournalError("invalid_journal", f"cannot read {path.name}: {error}") from error
    if text and not text.endswith("\n"):
        raise JournalError("invalid_journal", f"{path.name} ends with a partial line")
    return text.splitlines()


def _load_json(line: str, what: str) -> Any:
    try:
        return json.loads(line)
    except json.JSONDecodeError as error:
        raise JournalError("invalid_journal", f"{what}: bad JSON ({error.msg})") from error


def read_manifest(session_dir: Path) -> dict[str, Any]:
    path = session_dir / "manifest.json"
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise JournalError("invalid_journal", f"manifest.json unreadable: {error}") from error
    if manifest.get("schema") != SESSION_SCHEMA:
        raise JournalError(
            "unsupported_schema",
            f"session schema {manifest.get('schema')!r} is not {SESSION_SCHEMA!r}",
        )
    return manifest


def read_session(session_dir: Path) -> Session:
    """Parse and structurally validate one session; raise ``JournalError``."""
    manifest = read_manifest(session_dir)

    control_lines = _read_lines(session_dir / "control.jsonl")
    control = [_load_json(line, "control.jsonl") for line in control_lines]
    for expected, record in enumerate(control):
        if record.get("seq") != expected:
            raise JournalError(
                "invalid_journal",
                f"control.jsonl: seq {record.get('seq')!r} where {expected} was expected",
            )
    integrity = manifest.get("control_integrity")
    if integrity is not None:
        blob = "".join(line + "\n" for line in control_lines).encode()
        if integrity["lines"] != len(control_lines) or int(integrity["fnv"], 16) != fnv1a64(blob):
            raise JournalError("invalid_journal", "control.jsonl does not match its manifest record")

    chunk_files = sorted(
        (int(m.group(1)), p)
        for p in session_dir.iterdir()
        if (m := _CHUNK_RE.match(p.name))
    )
    table = {c["chunk"]: c for c in manifest.get("chunk_checksums", [])}
    ticks: list[Tick] = []
    for expected_index, (index, path) in enumerate(chunk_files):
        if index != expected_index:
            raise JournalError("invalid_journal", f"chunk {expected_index} is missing")
        lines = _read_lines(path)
        if not lines:
            raise JournalError("invalid_journal", f"{path.name} is empty")
        trailer = _load_json(lines[-1], f"{path.name} trailer")
        if not trailer.get("end"):
            raise JournalError("invalid_journal", f"{path.name} has no closing trailer")
        body = lines[:-1]
        blob = "".join(line + "\n" for line in body).encode()
        digest = fnv1a64(blob)
        if int(trailer["fnv"], 16) != digest or trailer["ticks"] != len(body):
            raise JournalError("invalid_journal", f"{path.name} fails its checksum")
        entry = table.get(index)
        if entry is None or int(entry["fnv"], 16) != digest or entry["ticks"] != len(body):
            raise JournalError("invalid_journal", f"{path.name} disagrees with the manifest table")
        for line_index, line in enumerate(body):
            raw = _load_json(line, f"{path.name}:{line_index}")
            ticks.append(Tick(raw, index, line_index))
    if len(table) != len(chunk_files):
        raise JournalError("invalid_journal", "the manifest lists chunks that are not on disk")

    for expected, tick in enumerate(ticks):
        if tick.t != expected:
            raise JournalError(
                "invalid_journal",
                f"tick {tick.t} where {expected} was expected (missing or duplicate tick)",
            )
    if manifest.get("tick_count") != len(ticks):
        raise JournalError(
            "invalid_journal",
            f"manifest tick_count {manifest.get('tick_count')} != {len(ticks)} ticks on disk",
        )

    ends: dict[int, dict[str, Any]] = {}
    for record in control:
        ev = record["ev"]
        if ev.get("k") == "episode_end":
            ends[ev["episode"]] = ev
    return Session(session_dir, manifest, control, ticks, ends)


def session_digest(session_dir: Path) -> str:
    """SHA-256 over the manifest, control stream and every tick chunk (names and
    bytes, in sorted order): detects a session edited while it was exported."""
    digest = hashlib.sha256()
    names = ["manifest.json", "control.jsonl"] + sorted(
        p.name for p in session_dir.iterdir() if _CHUNK_RE.match(p.name)
    )
    for name in names:
        digest.update(name.encode() + b"\0")
        try:
            digest.update((session_dir / name).read_bytes())
        except OSError as error:
            raise JournalError("invalid_journal", f"cannot read {name}: {error}") from error
        digest.update(b"\0")
    return digest.hexdigest()


def list_session_dirs(base: Path) -> list[Path]:
    """Every directory below ``base`` that has a ``manifest.json``."""
    if not base.is_dir():
        return []
    return sorted(p for p in base.iterdir() if (p / "manifest.json").is_file())
