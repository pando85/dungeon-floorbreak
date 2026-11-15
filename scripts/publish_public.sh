#!/usr/bin/env bash
set -euo pipefail

REPO_NAME="${REPO_NAME:-pando85/dungeon-floorbreak}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if ! command -v gh >/dev/null 2>&1; then
  echo "GitHub CLI (gh) is required: https://cli.github.com/" >&2
  exit 1
fi
if ! command -v git >/dev/null 2>&1; then
  echo "git is required" >&2
  exit 1
fi
if ! gh auth status >/dev/null 2>&1; then
  echo "Authenticate first with: gh auth login" >&2
  exit 1
fi
python3 scripts/check_docs.py

if gh repo view "$REPO_NAME" >/dev/null 2>&1; then
  echo "Repository $REPO_NAME already exists. Nothing was pushed. Inspect it before any changes." >&2
  exit 1
fi
if [[ ! -d .git ]]; then
  git init -b main
fi
if ! git config user.name >/dev/null || ! git config user.email >/dev/null; then
  echo "Configure git user.name and user.email before publishing" >&2
  exit 1
fi
if [[ -n "$(git status --porcelain 2>/dev/null)" ]]; then
  git add .
  git commit -m "docs: bootstrap AI-first pixel platformer design"
fi

echo "Will create PUBLIC GitHub repository: $REPO_NAME"
echo "Review the files and press Ctrl-C to cancel, or Enter to continue."
read -r _
gh repo create "$REPO_NAME" --public --source=. --remote=origin --push

echo "Published: https://github.com/$REPO_NAME"
