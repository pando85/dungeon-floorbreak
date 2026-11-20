"""Versions, schema stamps and content hashes written next to every artifact."""

from __future__ import annotations

import hashlib
import json
import platform
import subprocess
import sys
from importlib import metadata
from pathlib import Path
from typing import Any

from autoplay import _native
from autoplay.env import ACTION_NVEC, ACTION_VERSION, OBSERVATION_VERSION, observation_schema

#: Layout and kit contract the Floor 01 fixtures run on (mirrors the native env).
CORRIDOR_LAYOUT = "assets/levels/floor01/corridor_abc_sample.json"
REWARD_PROFILES = "assets/data/autoplay/reward_profiles_v1.json"

PACKAGES = ("gymnasium", "numpy", "stable-baselines3", "torch", "maturin", "pytest")


def assets_root(explicit: str | None = None) -> Path:
    import os

    return Path(explicit or os.environ.get(_native.ASSETS_ROOT_ENV) or _native.default_assets_root())


def package_versions() -> dict[str, str | None]:
    versions: dict[str, str | None] = {"python": platform.python_version()}
    for name in PACKAGES:
        try:
            versions[name] = metadata.version(name)
        except metadata.PackageNotFoundError:
            versions[name] = None
    return versions


def git_commit(root: Path) -> str:
    """Commit of the checkout, ``-dirty`` appended when tracked files changed.

    Same convention as the native CLI. Falls back to ``GITHUB_SHA`` and then
    ``unknown`` (an installed wheel moved away from its checkout).
    """
    import os

    def git(*args: str) -> str | None:
        try:
            out = subprocess.run(
                ["git", "-C", str(root), *args], check=True, capture_output=True, text=True
            )
            return out.stdout.strip()
        except (OSError, subprocess.CalledProcessError):
            return None

    head = git("rev-parse", "HEAD")
    if head and len(head) >= 7:
        return f"{head}-dirty" if git("status", "--porcelain", "--untracked-files=no") else head
    return os.environ.get("GITHUB_SHA", "unknown")


def content_hash(root: Path) -> str:
    """sha256 over the corridor layout, its kit contract and the reward data."""
    digest = hashlib.sha256()
    layout_path = root / CORRIDOR_LAYOUT
    layout = json.loads(layout_path.read_text())
    paths = [layout_path, root / layout["kit_contract"], root / REWARD_PROFILES]
    for path in paths:
        digest.update(path.read_bytes())
    return digest.hexdigest()


def schema_stamp(reward_id: str) -> dict[str, Any]:
    """What must match for a checkpoint to be loadable against this build."""
    schema = observation_schema()
    canonical = json.dumps(
        {k: v for k, v in schema.items()}, sort_keys=True, default=list
    ).encode()
    return {
        "observation_version": OBSERVATION_VERSION,
        "observation_schema_sha256": hashlib.sha256(canonical).hexdigest(),
        "action_version": ACTION_VERSION,
        "action_nvec": list(ACTION_NVEC),
        "reward_id": reward_id,
    }


def environment_record(reward_id: str, root: Path | None = None) -> dict[str, Any]:
    root = root or assets_root()
    return {
        "python_executable_name": Path(sys.executable).name,
        "versions": package_versions(),
        "schema": schema_stamp(reward_id),
        "commit": git_commit(root),
        "content_sha256": content_hash(root),
        "device": "cpu",
    }
