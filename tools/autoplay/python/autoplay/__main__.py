"""``python -m autoplay`` command line: check-env, evaluate, train, benchmark."""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

from autoplay.env import AutoplayEnv, scenario_roster

DEFAULT_SCENARIO = "cp01_single_goblin"


def cmd_check_env(args: argparse.Namespace) -> int:
    from gymnasium.utils.env_checker import check_env

    scenarios = args.scenario or [sid for sid, _ in scenario_roster()]
    for scenario in scenarios:
        env = AutoplayEnv(scenario)
        check_env(env, skip_render_check=True)
        print(f"check_env ok: {scenario}")
    return 0


def cmd_evaluate(args: argparse.Namespace) -> int:
    from autoplay.bots import make_policy
    from autoplay.evaluate import evaluate, write_report
    from autoplay.provenance import assets_root, git_commit

    scenarios = args.scenario or [DEFAULT_SCENARIO]
    extra: dict = {"reward_id": args.reward_id}
    if args.checkpoint:
        from autoplay.train import load_policy

        policy, record = load_policy(Path(args.checkpoint), reward_id=args.reward_id)
        extra.update(
            {"checkpoint": str(args.checkpoint), "checkpoint_sha256": record["model_sha256"]}
        )
    else:
        policy = make_policy(args.policy)
    out = Path(args.out)
    result = evaluate(
        scenarios,
        policy,
        args.episodes,
        args.seed,
        env_kwargs={"reward_id": args.reward_id},
        replays=args.replays,
        replay_dir=out / "replays",
        commit=git_commit(assets_root()),
    )
    write_report(result, out, extra)
    print(json.dumps({k: v for k, v in result.items() if k != "rows"}, indent=2, sort_keys=True))
    return 0


def cmd_train(args: argparse.Namespace) -> int:
    from autoplay.train import train

    if args.algorithm != "ppo":
        raise SystemExit("only --algorithm ppo is supported")
    smoke = train(
        args.scenario or [DEFAULT_SCENARIO],
        args.steps,
        args.seed,
        Path(args.out),
        eval_episodes=args.eval_episodes,
    )
    print(json.dumps(smoke, indent=2, sort_keys=True))
    return 0


def cmd_benchmark(args: argparse.Namespace) -> int:
    from autoplay.benchmark import run_benchmark

    result = run_benchmark(args.scenario or DEFAULT_SCENARIO, args.steps)
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="python -m autoplay")
    sub = parser.add_subparsers(dest="command", required=True)

    check = sub.add_parser("check-env", help="run gymnasium check_env on roster scenarios")
    check.add_argument("--scenario", action="append")
    check.set_defaults(func=cmd_check_env)

    ev = sub.add_parser("evaluate", help="seeded evaluation of a baseline or a checkpoint")
    ev.add_argument("--scenario", action="append", help="repeatable; default cp01_single_goblin")
    ev.add_argument(
        "--policy",
        default="random_legal",
        help="a native Rust bot (random_legal, approach_and_attack, reactive_survivor) or noop",
    )
    ev.add_argument("--checkpoint", help="evaluate a saved PPO checkpoint directory instead")
    ev.add_argument(
        "--replays",
        choices=("none", "first", "all"),
        default="first",
        help="write native autoplay-episode/1 replays (default: first episode per scenario)",
    )
    ev.add_argument(
        "--reward-id",
        default="combat-v1",
        help="reward profile of the evaluation env; a checkpoint trained on another is refused",
    )
    ev.add_argument("--episodes", type=int, default=50)
    ev.add_argument("--seed", type=int, default=42)
    ev.add_argument("--out", required=True)
    ev.set_defaults(func=cmd_evaluate)

    tr = sub.add_parser("train", help="bounded CPU PPO smoke")
    tr.add_argument("--algorithm", default="ppo")
    tr.add_argument("--scenario", action="append")
    tr.add_argument("--steps", type=int, default=10_000)
    tr.add_argument("--seed", type=int, default=42)
    tr.add_argument("--eval-episodes", type=int, default=20)
    tr.add_argument("--out", required=True)
    tr.set_defaults(func=cmd_train)

    bench = sub.add_parser("benchmark", help="single-env steps/s: Rust loop vs binding vs Gymnasium")
    bench.add_argument("--scenario")
    bench.add_argument("--steps", type=int, default=20_000)
    bench.set_defaults(func=cmd_benchmark)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    started = time.perf_counter()
    code = args.func(args)
    print(f"done in {time.perf_counter() - started:.1f}s", file=sys.stderr)
    return code


if __name__ == "__main__":
    raise SystemExit(main())
