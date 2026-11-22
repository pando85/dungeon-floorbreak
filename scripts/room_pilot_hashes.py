#!/usr/bin/env python3
"""EVID-ROOM: the final state of every room pilot, headless, one line per plan:

  PLAN tick=T hp=H deaths=D complete=B hash=X

  room_pilot_hashes.py ROOM [PLAN ...]      (default plans: direct valve chain, solo and :2p)

Used to write docs/evidence/<room>/pilot_hashes.txt and by
`scripts/ci_floor01_combat.sh headless` to compare against it. Exits non-zero when
a plan is not identical on two runs. BIN selects the binary."""
import os
import re
import subprocess
import sys

BIN = os.environ.get("BIN", "./target/debug/floor01_scene")
PLANS = ["direct", "valve", "chain", "direct:2p", "valve:2p", "chain:2p"]
TICKS = 1500


def run(room: str, plan: str) -> str:
    out = subprocess.run(
        [BIN, "--headless-floor01-smoke", "--scene", "f01_late", "--f01-room", room,
         "--room-pilot", plan, "--capture-tick", str(TICKS)],
        check=True, capture_output=True, text=True).stdout
    state = [l for l in out.splitlines() if l.startswith("COMBAT_STATE ")][-1]
    ok = [l for l in out.splitlines() if "_SMOKE_OK" in l][-1]
    tick = re.search(r" tick=(\d+) ", state).group(1)
    hp = re.search(r"hero_hp=(\d+)", state).group(1)
    deaths = re.search(r"deaths=(\d+)", state).group(1)
    complete = re.search(r"complete=(\w+)", ok)
    digest = re.search(r" hash=([0-9a-f]+) ", state).group(1)
    return (f"{plan} tick={tick} hp={hp} deaths={deaths} "
            f"complete={complete.group(1) if complete else 'n/a'} hash={digest}")


def main() -> None:
    room = sys.argv[1]
    for plan in sys.argv[2:] or PLANS:
        first, second = run(room, plan), run(room, plan)
        if first != second:
            sys.exit(f"{room} {plan}: not identical on two runs: {first!r} vs {second!r}")
        print(first)


if __name__ == "__main__":
    main()
