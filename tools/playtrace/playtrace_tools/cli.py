"""``python -m playtrace_tools``: export-transitions, export-bc, check-dataset, bc-smoke.

Run from ``tools/playtrace`` (or with ``PYTHONPATH=tools/playtrace``). Sessions are
read from ``--dir`` (default ``$XDG_DATA_HOME/dungeon-floorbreak/playtrace``) and
datasets are written outside the repository (default
``$XDG_DATA_HOME/dungeon-floorbreak/datasets``). ``list``, ``inspect``, ``verify``
and ``stats`` are the Rust ``playtrace`` binary's.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

from .bc import TrainConfig, train_bc
from .dataset import DatasetError, check_against_sources, load_dataset
from .export import ExportError, Options, export_dataset
from .split import LeakageError
from .verify import VerifyResult, find_verifier, make_cli_verifier, repo_root


def _data_dir(leaf: str) -> Path:
    xdg = os.environ.get("XDG_DATA_HOME")
    base = Path(xdg) if xdg and os.path.isabs(xdg) else Path.home() / ".local/share"
    return base / "dungeon-floorbreak" / leaf


def _inside_repo(path: Path) -> bool:
    root = repo_root().resolve()
    resolved = path.resolve()
    return resolved == root or root in resolved.parents


def _add_export_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--dir", type=Path, default=None, help="session base directory")
    parser.add_argument(
        "--out", type=Path, default=None, help="dataset output directory (outside the repository)"
    )
    parser.add_argument(
        "--session", action="append", default=[], help="full session id (repeatable; no prefixes)"
    )
    parser.add_argument("--hero", choices=("carl", "donut", "both"), default="both")
    parser.add_argument(
        "--context", type=int, default=0, help="previous actions kept as trajectory context"
    )
    parser.add_argument("--seed", type=int, default=0, help="split seed")
    parser.add_argument("--dev-frac", type=float, default=0.2)
    parser.add_argument("--holdout-frac", type=float, default=0.2)
    parser.add_argument("--allow-synthetic", action="store_true")
    parser.add_argument(
        "--export-consented",
        action="store_true",
        help="include includes_other_tester sessions named by --session",
    )
    parser.add_argument(
        "--verifier",
        default=None,
        help="playtrace binary built from the recording's source (or PLAYTRACE_BIN)",
    )
    parser.add_argument(
        "--root", type=Path, default=None, help="repository checkout the replay loads assets from"
    )


def _verifier(args: argparse.Namespace):
    binary = find_verifier(args.verifier)
    if binary is None:
        print(
            "playtrace_tools: no playtrace verifier binary found; every session will be rejected",
            file=sys.stderr,
        )
        return lambda _path: VerifyResult(
            False, code="verifier_unavailable", reason="no playtrace binary"
        )
    return make_cli_verifier(binary, (args.root or repo_root()).resolve())


def cmd_export(args: argparse.Namespace, kind: str) -> int:
    base = args.dir or _data_dir("playtrace")
    out = args.out or _data_dir("datasets")
    if _inside_repo(out):
        print(
            "playtrace_tools: refusing to write a dataset inside the repository checkout",
            file=sys.stderr,
        )
        return 2
    options = Options(
        kind=kind,
        session_ids=args.session,
        hero=args.hero,
        actor="human" if kind == "bc" else getattr(args, "actor", "both"),
        context=args.context,
        seed=args.seed,
        dev_frac=args.dev_frac,
        holdout_frac=args.holdout_frac,
        allow_synthetic=args.allow_synthetic,
        export_consented=args.export_consented,
    )
    try:
        meta = export_dataset(base, out, options, _verifier(args))
    except ExportError as error:
        print(f"playtrace_tools: {error}", file=sys.stderr)
        return 2
    except LeakageError as error:
        print(f"PLAYTRACE_EXPORT result=FAIL reason={str(error)!r}", file=sys.stderr)
        return 2
    summary = {
        "dataset_id": meta["dataset_id"],
        "kind": meta["kind"],
        "synthetic": meta["synthetic"],
        "sessions_exported": len(meta["sessions"]),
        "sessions_rejected": [
            {"reason": r["reason"], "detail": r["detail"]} for r in meta["rejected"]
        ],
        "excluded_rows": meta["excluded_rows"],
        "streams": meta["streams"],
        "leakage": {k: v for k, v in meta["leakage"].items() if k != "groups_by_split"},
    }
    print("PLAYTRACE_EXPORT " + json.dumps(summary, sort_keys=True))
    print(f"PLAYTRACE_EXPORT_DIR {meta['path']}")
    return 0 if meta["sessions"] else 1


def cmd_check(args: argparse.Namespace) -> int:
    try:
        dataset = load_dataset(args.dataset)
        report = check_against_sources(dataset, args.dir or _data_dir("playtrace"))
    except DatasetError as error:
        print(f"PLAYTRACE_CHECK result=FAIL reason={str(error)!r}")
        return 1
    print(
        "PLAYTRACE_CHECK result=PASS "
        + json.dumps({k: v for k, v in report.items() if k != "leakage"})
    )
    return 0


def cmd_bc(args: argparse.Namespace) -> int:
    try:
        dataset = load_dataset(args.dataset)
        config = TrainConfig(
            epochs=args.epochs, history=args.history, seed=args.seed, hidden=args.hidden
        )
        report = train_bc(dataset, config)
    except DatasetError as error:
        print(f"playtrace_tools: {error}", file=sys.stderr)
        return 1
    text = json.dumps(report, indent=2, sort_keys=True)
    if args.report:
        args.report.write_text(text + "\n", encoding="utf-8")
    print(report["caveat"])
    print(text)
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="playtrace_tools", description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)

    transitions = sub.add_parser(
        "export-transitions",
        help="verified (obs, action, next_obs, terminal) rows, human and AI streams apart",
    )
    _add_export_args(transitions)
    transitions.add_argument("--actor", choices=("human", "ai", "both"), default="both")
    transitions.set_defaults(func=lambda a: cmd_export(a, "transitions"))

    bc = sub.add_parser(
        "export-bc", help="verified human (observation, action) rows for behaviour cloning"
    )
    _add_export_args(bc)
    bc.set_defaults(func=lambda a: cmd_export(a, "bc"))

    check = sub.add_parser(
        "check-dataset", help="re-read the sources and prove every row maps to its tick"
    )
    check.add_argument("dataset", type=Path)
    check.add_argument("--dir", type=Path, default=None, help="session base directory")
    check.set_defaults(func=cmd_check)

    smoke = sub.add_parser(
        "bc-smoke", help="tiny numpy behaviour-cloning pipeline smoke (not a policy)"
    )
    smoke.add_argument("dataset", type=Path)
    smoke.add_argument("--epochs", type=int, default=40)
    smoke.add_argument(
        "--history", type=int, default=0, help="previous actions as input (needs export --context)"
    )
    smoke.add_argument("--hidden", type=int, default=64)
    smoke.add_argument("--seed", type=int, default=0)
    smoke.add_argument("--report", type=Path, default=None)
    smoke.set_defaults(func=cmd_bc)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    return args.func(args)
