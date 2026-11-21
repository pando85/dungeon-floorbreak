"""Export verified human (and, separately, AI) transitions from sessions.

Rules (PLAYTRACE-01 PT-D, PT-Q08, PT-Q09, PT-Q16):

* Only sessions that pass ``playtrace verify`` (scope ``full``), whose own
  structural re-check passes and whose files did not change during the export.
* Human and AI actions go to separate streams (``human/`` and ``ai/``) and are
  never relabelled: the stream is decided by the recorded control source of the
  hero on that tick, which the verifier re-derives.
* ``includes_other_tester`` sessions need ``--export-consented`` and an
  explicit ``--session``; synthetic fixtures need ``--allow-synthetic`` and
  make the whole dataset synthetic (never mixed with real data).
* Policy inputs are the observation only. ``diagnostics.jsonl`` is never read.
* Every row keeps ``(session, chunk, line, t, pre/post hash, raw action bits)``
  so it maps back to its source tick. The dataset is content-addressed.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
from collections import Counter, defaultdict, deque
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import numpy as np

from . import schema
from .journal import JournalError, Session, list_session_dirs, read_manifest, read_session, session_digest
from .split import SPLITS, assign_splits, check_leakage, group_key
from .verify import Verifier

DATASET_SCHEMA = "playtrace-dataset/1"
ROW_SCHEMA = "playtrace-bc-row/1"
EXPORTER_VERSION = "playtrace-tools/1"
SYNTHETIC_FLAG = "synthetic_fixture"
SYNTHETIC_MODE = "synthetic_fixture"

HUMAN_SOURCES = ("human_p1", "human_p2")
AI_SOURCES = ("companion_ai", "scripted_bot")
UNDRIVEN_SOURCE = "none"
# Episode outcomes that end an episode in a defeat the step itself reset.
IN_STEP_RESET_OUTCOMES = ("party_defeat", "demo_defeat_respawn")
SPLIT_INDEX = {name: i for i, name in enumerate(SPLITS)}
ACCEPTED_STATUS = ("completed", "abandoned")

NOTES = [
    "Human and AI actions are separate streams and are never relabelled.",
    "Policy inputs are ObservationV2 only; privileged diagnostics are not exported.",
    "reward is not exported: no named reward profile is applied (event ids only, no event text).",
    "Splits are by session (or player pseudonym when supplied); scenario variants are "
    "reported, and a within-scenario split is not a hold-out of new geometry or enemies.",
]


class ExportError(Exception):
    """The export cannot run as asked (bad options, mixed synthetic and real data...)."""


@dataclass
class Options:
    kind: str  # "transitions" or "bc"
    session_ids: list[str] = field(default_factory=list)
    hero: str = "both"  # carl | donut | both
    actor: str = "both"  # human | ai | both (bc is always human)
    context: int = 0
    seed: int = 0
    dev_frac: float = 0.2
    holdout_frac: float = 0.2
    allow_synthetic: bool = False
    export_consented: bool = False

    def validate(self) -> None:
        if self.kind not in ("transitions", "bc"):
            raise ExportError(f"unknown export kind {self.kind!r}")
        if self.hero not in ("carl", "donut", "both"):
            raise ExportError("--hero must be carl, donut or both")
        if self.actor not in ("human", "ai", "both"):
            raise ExportError("--actor must be human, ai or both")
        if self.kind == "bc" and self.actor != "human":
            raise ExportError("export-bc is human behaviour cloning: --actor must be human")
        if self.context < 0:
            raise ExportError("--context must be >= 0")
        if self.export_consented and not self.session_ids:
            raise ExportError("--export-consented needs at least one explicit --session")

    @property
    def streams(self) -> tuple[str, ...]:
        if self.kind == "bc":
            return ("human",)
        return ("human", "ai") if self.actor == "both" else (self.actor,)

    @property
    def heroes(self) -> tuple[int, ...]:
        return {"carl": (0,), "donut": (1,), "both": (0, 1)}[self.hero]


@dataclass
class SessionRows:
    """Rows built from one session, per stream, with their tensors."""

    rows: dict[str, list[dict[str, Any]]]
    obs: dict[str, list[np.ndarray]]
    next_obs: dict[str, list[np.ndarray]]
    context: dict[str, list[np.ndarray]]
    excluded: Counter


def _variant(manifest: dict[str, Any]) -> str:
    reset = manifest["reset"]
    return (
        f"{reset['source']}:{reset['name']}|cf02={reset['cf02_rules']}"
        f"|seed={reset.get('cf02_seed')}|kit={reset['kit_hash']}"
    )


def synthetic_markers(manifest: dict[str, Any]) -> list[bool]:
    """The three independent synthetic markers: quality flag, mode, feature flag."""
    return [
        SYNTHETIC_FLAG in manifest.get("quality_flags", []),
        manifest.get("mode") == SYNTHETIC_MODE,
        manifest.get("feature_flags", {}).get("synthetic") == "true",
    ]


def is_synthetic(manifest: dict[str, Any]) -> bool:
    """Synthetic when any marker says so (dropping one flag does not make a
    fixture a human session); ``check_eligibility`` rejects a disagreement."""
    return any(synthetic_markers(manifest))


def check_eligibility(manifest: dict[str, Any], options: Options, explicit: bool) -> tuple[str, str] | None:
    """A ``(reason, detail)`` rejection from the manifest alone, or ``None``."""
    if manifest.get("status") not in ACCEPTED_STATUS or not manifest.get("capture_complete"):
        return (
            "capture_incomplete",
            f"status={manifest.get('status')} capture_complete={manifest.get('capture_complete')}",
        )
    if manifest.get("observation_is_partial", True):
        return "observation_partial", "the session has no complete per-hero observation record"
    if manifest.get("observation_schema_id") != schema.OBS_SCHEMA_ID:
        return "unsupported_obs_schema", f"{manifest.get('observation_schema_id')!r}"
    if manifest.get("action_schema_id") not in schema.ACTION_SCHEMAS:
        return "unsupported_action_schema", f"{manifest.get('action_schema_id')!r}"
    if manifest.get("participants") != "owner_only" and not (options.export_consented and explicit):
        return (
            "other_tester_not_consented",
            "participants=includes_other_tester needs --export-consented and an explicit --session",
        )
    markers = synthetic_markers(manifest)
    if any(markers) and not all(markers):
        return (
            "synthetic_marker_mismatch",
            "quality flag, mode and feature flag disagree about the session being synthetic",
        )
    if any(markers) and not options.allow_synthetic:
        return "synthetic_not_allowed", "a synthetic fixture needs --allow-synthetic"
    return None


def build_session_rows(session: Session, options: Options) -> SessionRows:
    """Rows of one session. Structural problems raise ``JournalError`` (the whole
    session is then rejected: nothing is dropped silently row by row)."""
    manifest = session.manifest
    human_run = manifest["run_kind"] in ("human", "mixed")
    cache: dict[str, np.ndarray] = {}

    def decode(text: str, hero: int) -> np.ndarray:
        key = f"{hero}|{text}"
        found = cache.get(key)
        if found is None:
            try:
                found = schema.decode_obs(text)
                schema.validate_obs(found, hero)
            except schema.SchemaError as error:
                raise JournalError("invalid_observation", str(error)) from error
            cache[key] = found
        return found

    out = SessionRows(
        rows={s: [] for s in options.streams},
        obs={s: [] for s in options.streams},
        next_obs={s: [] for s in options.streams},
        context={s: [] for s in options.streams},
        excluded=Counter(),
    )
    ticks = session.ticks
    group = group_key(manifest)
    for hero in options.heroes:
        hero_name = schema.HEROES[hero]
        segment_of: dict[int, int] = {}
        previous: tuple[int, str, int] | None = None  # (episode, source, t)
        recent: deque[list[int]] = deque(maxlen=max(options.context, 1))
        for index, tick in enumerate(ticks):
            raw = tick.raw
            source = raw["src"][hero]
            if source in HUMAN_SOURCES:
                stream = "human"
                if not human_run:
                    raise JournalError("label_mismatch", "human tick in a non-human run")
            elif source in AI_SOURCES:
                stream = "ai"
            elif source == UNDRIVEN_SOURCE:
                out.excluded[f"undriven_{hero_name}"] += 1
                previous = None
                continue
            else:
                # A new control source (a learned policy, another human seat) must not
                # vanish from the human stream: the whole session is rejected.
                raise JournalError(
                    "unsupported_label", f"tick {tick.t}: unknown control source {source!r}"
                )
            if stream not in options.streams:
                previous = None
                continue
            if raw.get("obs") is None:
                raise JournalError("observation_partial", f"tick {tick.t} has no observation")
            if stream == "human":
                action_bits = raw["hero"][hero]
                action_source = "hero_routed"
            else:
                eff = raw.get("eff")
                if eff is None:
                    out.excluded["ai_without_effective_input"] += 1
                    previous = None
                    continue
                action_bits = eff[hero]
                action_source = "effective"
            try:
                schema.check_action_legal(action_bits)
            except schema.SchemaError as error:
                raise JournalError("illegal_action", f"tick {tick.t}: {error}") from error
            obs = decode(raw["obs"][hero], hero)
            action = schema.policy_action(action_bits)
            mask = schema.action_mask(obs)
            _, buttons = schema.unpack_action(action_bits)

            episode = raw["ep"]
            same = previous is not None and previous == (episode, source, tick.t - 1)
            if not same:
                segment_of[episode] = segment_of.get(episode, -1) + 1
                recent.clear()
            previous = (episode, source, tick.t)

            # The next observation: the next tick's o_t, or the episode boundary.
            nxt = ticks[index + 1] if index + 1 < len(ticks) else None
            outcome = None
            terminal = truncated = False
            if nxt is not None and nxt.raw["ep"] == episode and nxt.t == tick.t + 1:
                next_kind = "step"
                next_obs = decode(nxt.raw["obs"][hero], hero)
            else:
                end = session.episode_ends.get(episode)
                outcome = end["outcome"] if end else None
                final = end.get("final_obs") if end else None
                if final is None:
                    next_kind, next_obs = "missing", None
                elif outcome in IN_STEP_RESET_OUTCOMES:
                    # The defeat frame is not observable: final_obs is already the
                    # first observation of the next episode (PTI-14).
                    next_kind, next_obs, terminal = "post_reset", None, True
                else:
                    next_kind, next_obs, truncated = "final_obs", decode(final[hero], hero), True
            if next_kind == "step" and nxt is not None and nxt.raw["ep"] != episode:
                raise JournalError("invalid_journal", "episode changed inside a contiguous step")

            if options.kind == "transitions" and next_kind == "missing":
                # No s_(t+1) exists (the session ended without an episode_end): the
                # row is not a transition. It stays usable for behaviour cloning.
                out.excluded["no_next_obs"] += 1
                recent.append(action)
                continue

            row: dict[str, Any] = {
                "session_id": session.session_id,
                "group": group,
                "traj_id": f"{session.session_id}-e{episode}-{hero_name}-s{segment_of[episode]}",
                "t": tick.t,
                "ep": episode,
                "et": raw["et"],
                "stage": raw["stage"],
                "hero": hero_name,
                "controlled_by": source,
                "stream": stream,
                "action_source": action_source,
                "chunk": tick.chunk,
                "line": tick.line,
                "pre": raw["pre"],
                "post": raw["post"],
                "raw_action": action_bits,
                "eff_action": raw["eff"][hero] if raw.get("eff") else None,
                "action": action,
                "mask": mask,
                "mask_violation": bool(action[4] and not mask[4]),
                "nonpolicy_bits": {"swap": buttons["swap"], "pause": buttons["pause"]},
                "next_kind": next_kind,
                "terminal": terminal,
                "truncated": truncated,
                "outcome": outcome,
            }
            if options.kind == "transitions":
                row["eh"] = raw["eh"]
                row["event_ids"] = [e["i"] for e in raw["ev"]]
            ctx = np.zeros((max(options.context, 0), schema.ACTION_LEN), dtype=np.int8)
            if options.context:
                prev = list(recent)[-options.context :]
                row["ctx_len"] = len(prev)
                if prev:
                    ctx[options.context - len(prev) :] = np.array(prev, dtype=np.int8)
            recent.append(action)
            out.rows[stream].append(row)
            out.obs[stream].append(obs)
            out.next_obs[stream].append(
                next_obs if next_obs is not None else np.zeros(schema.OBS_LEN, np.float32)
            )
            out.context[stream].append(ctx)
            row["next_valid"] = next_obs is not None
    return out


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def _histogram(rows: list[dict[str, Any]]) -> dict[str, Any]:
    actions = [r["action"] for r in rows]
    counts = {
        name: Counter(a[i] for a in actions)
        for i, name in enumerate(schema.ACTION_COMPONENTS)
    }
    return {
        "rows": len(rows),
        "horizontal": {
            "left": counts["horizontal"][0],
            "neutral": counts["horizontal"][1],
            "right": counts["horizontal"][2],
        },
        **{
            name: counts[name][1]
            for name in schema.ACTION_COMPONENTS
            if name != "horizontal"
        },
        "mask_violations": sum(1 for r in rows if r["mask_violation"]),
        "swap_bit_ticks": sum(1 for r in rows if r["nonpolicy_bits"]["swap"]),
        "intent_differs_effective": sum(
            1 for r in rows if r["eff_action"] is not None and r["eff_action"] != r["raw_action"]
        ),
    }


def _write_stream(
    directory: Path,
    rows: list[dict[str, Any]],
    obs: list[np.ndarray],
    next_obs: list[np.ndarray],
    ctx: list[np.ndarray],
    options: Options,
) -> None:
    directory.mkdir(parents=True)
    with (directory / "rows.jsonl").open("w", encoding="utf-8") as handle:
        for index, row in enumerate(rows):
            handle.write(json.dumps({"row": index, **row}, sort_keys=True, separators=(",", ":")) + "\n")
    count = len(rows)
    np.save(directory / "obs.npy", np.stack(obs) if count else np.zeros((0, schema.OBS_LEN), np.float32))
    np.save(directory / "action.npy", np.array([r["action"] for r in rows], dtype=np.int8).reshape(count, schema.ACTION_LEN))
    np.save(directory / "mask.npy", np.array([r["mask"] for r in rows], dtype=np.uint8).reshape(count, schema.ACTION_LEN))
    np.save(directory / "split.npy", np.array([SPLIT_INDEX[r["split"]] for r in rows], dtype=np.int8))
    if options.kind == "transitions":
        np.save(directory / "next_obs.npy", np.stack(next_obs) if count else np.zeros((0, schema.OBS_LEN), np.float32))
        np.save(directory / "next_valid.npy", np.array([r["next_valid"] for r in rows], dtype=np.uint8))
        np.save(directory / "terminal.npy", np.array([r["terminal"] for r in rows], dtype=np.uint8))
    if options.context:
        np.save(directory / "context.npy", np.stack(ctx) if count else np.zeros((0, options.context, schema.ACTION_LEN), np.int8))


def resolve_sessions(candidates: list[Path], session_ids: list[str]) -> list[Path]:
    """Select the sessions named by ``--session``: each id must be a full session id
    that matches exactly one session directory. An empty id, a prefix, an unknown id
    or an ambiguous id is an error, so a short id can never widen the selection (and,
    with ``--export-consented``, never widens consent). No ids selects every session."""
    if not session_ids:
        return candidates
    chosen: dict[str, Path] = {}
    for session_id in session_ids:
        if not session_id.strip():
            raise ExportError("--session needs a non-empty full session id")
        matches = [p for p in candidates if p.name == session_id]
        if not matches:
            raise ExportError(
                f"no session has the exact id {session_id!r} (--session takes full ids, not prefixes)"
            )
        if len(matches) > 1:
            raise ExportError(f"{session_id!r} matches {len(matches)} sessions")
        chosen[session_id] = matches[0]
    return sorted(chosen.values())


def export_dataset(
    base: Path, out_dir: Path, options: Options, verifier: Verifier
) -> dict[str, Any]:
    """Export a dataset into ``out_dir/dataset-<id>/`` and return its manifest."""
    options.validate()
    candidates = resolve_sessions(list_session_dirs(base), options.session_ids)
    explicit = bool(options.session_ids)

    rejected: list[dict[str, str]] = []
    accepted: list[tuple[Session, dict[str, Any], str]] = []
    built: dict[str, SessionRows] = {}

    def reject(name: str, reason: str, detail: str) -> None:
        rejected.append({"session_id": name, "reason": reason, "detail": detail})

    for path in candidates:
        name = path.name
        try:
            manifest = read_manifest(path)
            if manifest.get("session_id") != name:
                # Rows are looked up by directory name: a renamed or copied session
                # directory would collide with, or hide behind, another id.
                reject(name, "invalid_journal", "the directory name differs from the manifest session_id")
                continue
            problem = check_eligibility(manifest, options, explicit=explicit)
            if problem:
                reject(name, *problem)
                continue
            before = session_digest(path)
            session = read_session(path)
            result = verifier(path)
            if not result.passed:
                reject(name, f"verify_failed:{result.code}", result.reason or "")
                continue
            if result.scope != "full":
                reject(name, "verify_not_full_scope", f"scope={result.scope}")
                continue
            rows = build_session_rows(session, options)
            if session_digest(path) != before:
                reject(name, "modified_during_export", "the session files changed")
                continue
        except JournalError as error:
            reject(name, error.code, error.message)
            continue
        accepted.append(
            (session, {"verify": {"result": "PASS", "scope": result.scope, "build": result.build}}, before)
        )
        built[session.session_id] = rows

    synthetic = {is_synthetic(s.manifest) for s, _, _ in accepted}
    if len(synthetic) > 1:
        raise ExportError("refusing to mix synthetic fixtures with real sessions in one dataset")
    dataset_synthetic = synthetic == {True}

    groups = assign_splits(
        [group_key(s.manifest) for s, _, _ in accepted],
        seed=options.seed,
        dev_frac=options.dev_frac,
        holdout_frac=options.holdout_frac,
    )
    streams: dict[str, dict[str, Any]] = {}
    all_rows: list[dict[str, Any]] = []
    stream_data: dict[str, tuple[list, list, list, list]] = {}
    excluded: Counter = Counter()
    for stream in options.streams:
        rows: list[dict[str, Any]] = []
        obs: list[np.ndarray] = []
        nxt: list[np.ndarray] = []
        ctx: list[np.ndarray] = []
        for session, _, _ in sorted(accepted, key=lambda a: a[0].session_id):
            part = built[session.session_id]
            for i, row in enumerate(part.rows[stream]):
                row["split"] = groups[row["group"]]
                rows.append(row)
                obs.append(part.obs[stream][i])
                nxt.append(part.next_obs[stream][i])
                ctx.append(part.context[stream][i])
        order = sorted(range(len(rows)), key=lambda i: (rows[i]["session_id"], rows[i]["t"], rows[i]["hero"]))
        rows = [rows[i] for i in order]
        stream_data[stream] = (rows, [obs[i] for i in order], [nxt[i] for i in order], [ctx[i] for i in order])
        all_rows += rows
    for session, _, _ in accepted:
        excluded.update(built[session.session_id].excluded)
    leakage = check_leakage(all_rows, groups)

    out_dir.mkdir(parents=True, exist_ok=True)
    staging = out_dir / f".staging-{os.getpid()}"
    shutil.rmtree(staging, ignore_errors=True)
    staging.mkdir()
    try:
        for stream, (rows, obs, nxt, ctx) in stream_data.items():
            _write_stream(staging / stream, rows, obs, nxt, ctx, options)
            by_hero = Counter(r["hero"] for r in rows)
            streams[stream] = {
                "rows": len(rows),
                "rows_by_hero": dict(by_hero),
                "rows_by_split": dict(Counter(r["split"] for r in rows)),
                "next_kind": dict(Counter(r["next_kind"] for r in rows)),
                "terminal_rows": sum(1 for r in rows if r["terminal"]),
                "action_histogram": _histogram(rows),
            }
        files = {
            str(p.relative_to(staging)): _sha256(p)
            for p in sorted(staging.rglob("*"))
            if p.is_file()
        }
        core = {
            "schema": DATASET_SCHEMA,
            "kind": options.kind,
            "synthetic": dataset_synthetic,
            "row_schema": ROW_SCHEMA,
            "exporter_version": EXPORTER_VERSION,
            "observation": {"schema": schema.OBS_SCHEMA_ID, "length": schema.OBS_LEN},
            "action": {
                "components": list(schema.ACTION_COMPONENTS),
                "sizes": list(schema.ACTION_SIZES),
                "stick_threshold": schema.STICK_THRESHOLD,
                "mask_rule": schema.MASK_RULE,
                "human_source": "hero routed input (intent)",
                "ai_source": "effective input (the AI decision)",
            },
            "options": {
                "kind": options.kind,
                "hero": options.hero,
                "actor": options.actor,
                "context": options.context,
                "seed": options.seed,
                "dev_frac": options.dev_frac,
                "holdout_frac": options.holdout_frac,
                "export_consented": options.export_consented,
            },
            "sessions": [
                {
                    "session_id": s.session_id,
                    "status": s.manifest["status"],
                    "run_kind": s.manifest["run_kind"],
                    "participants": s.manifest["participants"],
                    "build_commit": s.manifest["build_commit"],
                    "source_fingerprint": s.manifest.get("source_fingerprint", ""),
                    "variant": _variant(s.manifest),
                    "ticks": s.manifest["tick_count"],
                    "split": groups[group_key(s.manifest)],
                    "source_digest": digest,
                    **extra,
                }
                for s, extra, digest in sorted(accepted, key=lambda a: a[0].session_id)
            ],
            "rejected": sorted(rejected, key=lambda r: r["session_id"]),
            "excluded_rows": dict(excluded),
            "streams": streams,
            "splits": {
                "groups": groups,
                "variants_by_split": {
                    split: sorted({_variant(s.manifest) for s, _, _ in accepted if groups[group_key(s.manifest)] == split})
                    for split in SPLITS
                },
            },
            "leakage": leakage,
            "notes": NOTES,
            "files": files,
        }
        dataset_id = hashlib.sha256(json.dumps(core, sort_keys=True).encode()).hexdigest()[:16]
        core["dataset_id"] = dataset_id
        (staging / "dataset.json").write_text(json.dumps(core, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        final = out_dir / f"dataset-{dataset_id}"
        if final.exists():
            shutil.rmtree(staging)
        else:
            staging.rename(final)
    except BaseException:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    core["path"] = str(final)
    return core
