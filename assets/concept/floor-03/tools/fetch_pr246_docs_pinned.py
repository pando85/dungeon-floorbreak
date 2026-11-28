#!/usr/bin/env python3
"""Fetch exact reviewed PR #246 Markdown documents from a pinned public commit.

Run this script from anywhere. The destination defaults to the root of a Git
checkout detected by walking up from this file, or the current working dir.
Refuses to overwrite differing files unless --force is supplied.
"""
from __future__ import annotations
import argparse
from pathlib import Path
from urllib.request import Request, urlopen
import subprocess

REPO = "pando85/dungeon-floorbreak"
COMMIT = "3e0073df6f6e4fc2003245496d6952f64815d822"
FILES = [
    'docs/01-design/floor-03-over-city-design-bible.md',
    'docs/01-design/floor-03-circus-encounters-and-stagecraft.md',
    'docs/01-design/floor-03-grimaldi-resolution-and-coop.md',
    'docs/01-design/floor-03-skyfowl-city-and-quill-encounters.md',
    'docs/01-design/floor-03-doomsday-scenario-final-act.md',
    'docs/02-art/floor-03-over-city-art-direction.md',
    'docs/02-art/floor-03-circus-visual-storytelling.md',
    'docs/02-art/floor-03-skyfowl-and-doomsday-art-direction.md',
    'docs/03-tech/floor-03-campaign-bridge-acceptance.md',
    'docs/07-agents/floor-03-design-to-production-roadmap.md',
]

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--repo-root', type=Path, help='Git repository root, defaults to current working directory or nearest parent checkout')
    ap.add_argument('--force', action='store_true', help='Overwrite different existing files')
    a = ap.parse_args()
    if a.repo_root:
        root = a.repo_root.resolve()
    else:
        here = Path.cwd()
        try:
            result = subprocess.check_output(['git','rev-parse','--show-toplevel'], cwd=here, text=True, stderr=subprocess.DEVNULL).strip()
            root = Path(result)
        except Exception:
            root = here
    count = 0
    for name in FILES:
        dst = root/name
        url = f'https://raw.githubusercontent.com/{REPO}/{COMMIT}/{name}'
        req = Request(url, headers={'User-Agent':'PR246-design-import/1.0'})
        with urlopen(req,timeout=30) as r:
            data = r.read()
        if dst.exists() and dst.read_bytes() != data and not a.force:
            print('SKIP differs (use --force):',dst)
            continue
        dst.parent.mkdir(parents=True,exist_ok=True)
        dst.write_bytes(data)
        print('OK',dst)
        count += 1
    print(f'Fetched/wrote {count}/{len(FILES)} files from {COMMIT}. Review git diff before committing.')

if __name__ == '__main__':
    main()
