"""The per-asset config (``pixelart-asset-v1``): what the validators, evidence and review pack read.

A config mirrors the design spec of one asset. It names the grid source, the frame/pivot contract, the tag
table with 60 Hz tick windows, and the parameters of every validator rule. Example (abridged)::

    {"format": "pixelart-asset-v1", "asset_id": "gob-01-goblin-a",
     "grid": "assets/source/characters/goblin/goblin_a_h36.grid.json",
     "manifest": "assets/manifests/goblin_a_h36.json",
     "frame_size": [64, 56], "pivot": [28, 55], "contact_row": 55, "tick_hz": 60, "hold_ms": 100,
     "tags": {"idle": {"playback": "loop", "contact": "grounded", "frames": [[0, 12], [1, 8]]},
              "knockback": {"playback": "hold", "contact": "airborne", "frames": [[18, 4], [19, null]]}},
     "roles": {"idle": "idle", "move": "walk", "tell": "attack_tell", "attack": "attack_swing",
               "hurt": "hurt", "defeat": "down"},
     "rules": {"max_colors": 20, "max_isolated_per_frame": 0, "max_orphans_per_frame": 9, ...}}

``frames`` rows are ``[atlas frame, ticks]``; ``null`` ticks is a state-driven hold (preview duration
``hold_ms``). ``contact`` is ``grounded`` (lowest opaque row on ``contact_row``), ``airborne`` (no pixel on it)
or ``free`` (not checked).
"""
from __future__ import annotations

from pathlib import Path

from .common import TICK_HZ, ToolkitError, load_json, pg, repo_path

FORMAT = "pixelart-asset-v1"
ROLES = ("idle", "move", "tell", "attack", "hurt", "defeat")   # the readability set
CONTACTS = ("grounded", "airborne", "free")
PLAYBACKS = ("loop", "once", "hold")


class AssetConfig:
    def __init__(self, data: dict, path: Path | None = None) -> None:
        self.data = data
        self.path = path
        self.validate()

    # ---- raw access
    def __getitem__(self, key):
        return self.data[key]

    def get(self, key, default=None):
        return self.data.get(key, default)

    @property
    def asset_id(self) -> str:
        return self.data["asset_id"]

    @property
    def rules(self) -> dict:
        return self.data.get("rules", {})

    @property
    def tick_hz(self) -> int:
        return int(self.data.get("tick_hz", TICK_HZ))

    @property
    def hold_ms(self) -> int:
        return int(self.data.get("hold_ms", 100))

    @property
    def pivot(self) -> list[int]:
        return list(self.data["pivot"])

    @property
    def frame_size(self) -> list[int]:
        return list(self.data["frame_size"])

    @property
    def contact_row(self) -> int | None:
        value = self.data.get("contact_row", self.data["pivot"][1])
        return None if value is None else int(value)

    # ---- tag table
    @property
    def tags(self) -> dict:
        return self.data["tags"]

    @property
    def frame_count(self) -> int:
        return 1 + max(f for t in self.tags.values() for f, _ in t["frames"])

    def tag_of(self, frame: int) -> str:
        for name, tag in self.tags.items():
            if any(f == frame for f, _ in tag["frames"]):
                return name
        raise ToolkitError(f"frame {frame} is in no tag")

    def ticks_of(self, frame: int):
        for tag in self.tags.values():
            for f, t in tag["frames"]:
                if f == frame:
                    return t
        raise ToolkitError(f"frame {frame} is in no tag")

    def expected_ms(self, ticks) -> int:
        return self.hold_ms if ticks is None else round(ticks * 1000 / self.tick_hz)

    def role_tags(self, role: str) -> list[str]:
        value = self.data.get("roles", {}).get(role)
        if value is None:
            return []
        return [value] if isinstance(value, str) else list(value)

    def pose(self, role: str) -> int | None:
        """The frame that best shows a role in a still: the last frame of tell/attack/defeat, else the first."""
        tags = self.role_tags(role)
        if not tags:
            return None
        frames = [f for f, _ in self.tags[tags[0]]["frames"]]
        return frames[-1] if role in ("tell", "attack", "defeat") else frames[0]

    # ---- inputs
    def grid_path(self) -> Path:
        return repo_path(self.data["grid"])

    def load_grid(self) -> dict:
        return pg.load_grid(self.grid_path())

    def load_manifest(self) -> dict | None:
        raw = self.data.get("manifest")
        return load_json(repo_path(raw)) if raw else None

    # ---- schema
    def validate(self) -> None:
        d = self.data

        def need(ok: bool, message: str) -> None:
            if not ok:
                raise ToolkitError(f"config {self.path.name if self.path else ''}: {message}")

        need(d.get("format") == FORMAT, f"format must be {FORMAT!r}")
        for key in ("asset_id", "grid", "frame_size", "pivot", "tags"):
            need(key in d, f"missing {key!r}")
        need(isinstance(d["asset_id"], str) and d["asset_id"], "asset_id must be a non-empty string")
        need(len(d["frame_size"]) == 2 and len(d["pivot"]) == 2, "frame_size and pivot are [x, y]")
        need(isinstance(d["tags"], dict) and d["tags"], "tags must be a non-empty object")
        seen: set[int] = set()
        for name, tag in d["tags"].items():
            need(tag.get("playback", "once") in PLAYBACKS, f"tag {name}: playback must be one of {PLAYBACKS}")
            need(tag.get("contact", "free") in CONTACTS, f"tag {name}: contact must be one of {CONTACTS}")
            need(bool(tag.get("frames")), f"tag {name}: frames is empty")
            frames = [f for f, _ in tag["frames"]]
            need(frames == list(range(frames[0], frames[0] + len(frames))), f"tag {name}: frames must be contiguous")
            need(not seen & set(frames), f"tag {name}: a frame belongs to two tags")
            seen |= set(frames)
            for f, t in tag["frames"]:
                need(t is None or (isinstance(t, int) and t > 0), f"tag {name}: frame {f} ticks must be a positive int or null")
        need(sorted(seen) == list(range(len(seen))), "tag frames must cover 0..N-1 without gaps")
        for role, value in d.get("roles", {}).items():
            need(role in ROLES, f"unknown role {role!r} (readability set: {', '.join(ROLES)})")
            for tag in ([value] if isinstance(value, str) else value):
                need(tag in d["tags"], f"role {role}: unknown tag {tag!r}")


def load_config(path: Path | str) -> AssetConfig:
    path = Path(path)
    return AssetConfig(load_json(path), path)


def default_config(grid: dict, grid_path: str, asset_id: str, manifest: str | None = None,
                   tick_hz: int = TICK_HZ) -> AssetConfig:
    """A minimal config for a bare grid or manifest: tags and ticks from the grid, structural rules only."""
    tags: dict = {}
    for tag in grid["tags"]:
        rows = []
        for i in range(tag["from"], tag["to"] + 1):
            ms = grid["frames"][i]["duration_ms"]
            rows.append([i, max(1, round(ms * tick_hz / 1000))])
        tags[tag["name"]] = {"playback": "loop" if tag["from"] != tag["to"] else "once", "contact": "free", "frames": rows}
    covered = {f for t in tags.values() for f, _ in t["frames"]}
    if covered != set(range(len(grid["frames"]))):
        tags = {"all": {"playback": "loop", "contact": "free",
                        "frames": [[i, max(1, round(fr["duration_ms"] * tick_hz / 1000))]
                                   for i, fr in enumerate(grid["frames"])]}}
    data = {"format": FORMAT, "asset_id": asset_id, "grid": grid_path, "frame_size": list(grid["canvas"]),
            "pivot": list(grid["pivot"]), "tick_hz": tick_hz, "tags": tags,
            "rules": {"max_colors": 24, "max_isolated_per_frame": 0, "max_orphans_per_frame": 9}}
    if manifest:
        data["manifest"] = manifest
    return AssetConfig(data)
