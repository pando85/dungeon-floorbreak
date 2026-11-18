"""Lit Floor 1 composition with Carl and Donut at scale.

Reuses the existing code paths: the Floor 1 kit contract and layout loaders of ``scripts/env_kit.py`` and the
A1 lighting pass ``render_lit`` of ``scripts/env_kit_light.py`` (the same calls the per-asset review scripts
make). Nothing about the lighting is re-implemented here. The kit atlases under ``generated/art`` must exist
(``bash scripts/export_assets.sh``).
"""
from __future__ import annotations

from PIL import Image

from .common import (CARL_X, DONUT_X, LAYOUT, REFERENCES, SCENE_X, ToolkitError, flip, grid_images, load_grid_file,
                     load_json, repo_path)
from .config import AssetConfig


class Floor:
    def __init__(self) -> None:
        import env_kit as ek
        import env_kit_light as ekl
        self.ek, self.ekl = ek, ekl
        try:
            self.layout = ek.load_json(LAYOUT)
            self.kit = ek.load_kit(self.layout["kit_contract"])
            ek.validate_contract(self.kit)
            ek.validate_layout(self.kit, self.layout)
        except FileNotFoundError as exc:
            raise ToolkitError("the Floor 1 kit atlases are missing: run scripts/export_assets.sh first") from exc
        cols, rows = self.layout["grid_size"]
        self.tops = ek.collider_tops(self.kit, self.layout["tile_layers"]["collision"], cols, rows)
        self.view = self.layout["views"]["abc_640x360"]

    def ground(self, x: int) -> int:
        top = self.tops[x // self.ek.TILE]
        if not isinstance(top, int):
            raise ToolkitError(f"no walking line at x={x}")
        return top - 1

    def render(self, entries, k: int, lit: bool = True) -> Image.Image:
        """entries: ``(sprite, pivot, x, lift[, shadow])``; the pivot row sits on the walking line (minus lift).

        ``shadow`` (default True) draws the ground contact shadow; wall or ceiling creatures opt out.
        """
        layer = Image.new("RGBA", tuple(self.layout["pixel_size"]), (0, 0, 0, 0))
        boxes = []
        for sprite, piv, x, lift, *rest in entries:
            pos = (x - piv[0], self.ground(x) - piv[1] - lift)
            layer.alpha_composite(sprite, pos)
            if not rest or rest[0]:
                boxes.append((pos[0], pos[1] + lift, pos[0] + sprite.width, pos[1] + lift + sprite.height))
        return self.ekl.render_lit(self.kit, self.layout, self.view, k, layer, boxes=boxes, lit=lit).convert("RGB")


def reference_idle(name: str):
    """(image, pivot) of an approved reference's idle frame 0."""
    path, _height = REFERENCES[name]
    grid = load_grid_file(repo_path(path))
    return grid_images(grid)[0], tuple(grid["pivot"])


def default_actors(cfg: AssetConfig) -> list[dict]:
    actors = cfg.get("scene", {}).get("actors")
    if actors:
        return actors
    out = []
    for role, x in zip(("idle", "tell", "attack", "hurt", "move", "defeat"), SCENE_X):
        frame = cfg.pose(role)
        if frame is not None:
            out.append({"frame": frame, "x": x})
    if not out:
        out = [{"frame": 0, "x": SCENE_X[0]}]
    return out


def entries_for(cfg: AssetConfig, imgs: list[Image.Image]) -> list[tuple]:
    carl, carl_piv = reference_idle("carl")
    donut, donut_piv = reference_idle("donut")
    entries = [(donut, donut_piv, DONUT_X, 0), (carl, carl_piv, CARL_X, 0)]
    pivot = tuple(cfg.pivot)
    for actor in default_actors(cfg):
        img, piv = imgs[actor["frame"]], pivot
        # authored facing right; enemies face Carl on their left, so mirror them (the runtime rule)
        if actor.get("flip", cfg.get("facing", "right") == "right"):
            img, piv = flip(img, pivot)
        shadow = actor.get("shadow", cfg.get("scene", {}).get("shadow", True))  # False: wall/ceiling creature
        entry = (img, piv, actor["x"], actor.get("lift", 0), shadow)
        if actor.get("behind"):       # drawn before Carl and Donut (effects sit behind the actors)
            entries.insert(0, entry)
        else:
            entries.append(entry)
    for ref in cfg.get("scene", {}).get("references", []):
        # extra approved sprites for direct comparison (e.g. GOB-01 A/B next to a later goblin type)
        grid = load_grid_file(repo_path(ref["grid"]))
        img, piv = grid_images(grid)[ref.get("frame", 0)], tuple(grid["pivot"])
        if ref.get("flip", True):
            img, piv = flip(img, piv)
        entries.append((img, piv, ref["x"], ref.get("lift", 0), ref.get("shadow", True)))
    return entries


def render_scene(cfg: AssetConfig, imgs: list[Image.Image], k: int, floor: Floor | None = None,
                 lit: bool = True) -> Image.Image:
    """The asset's poses facing Carl and Donut on lit Floor 1; ``k`` 1 is 640x360, 3 is 1920x1080."""
    return (floor or Floor()).render(entries_for(cfg, imgs), k, lit)
