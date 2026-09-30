#!/usr/bin/env bash
# Mirrors a directory of pages into the GitHub wiki repository and pushes it to
# the branch that was cloned. Prints "Nothing to publish" when the wiki already
# matches. Needs git and rsync.
#   publish.sh <src-dir> <wiki-remote-url> <source-sha>
set -euo pipefail

if [[ $# -ne 3 ]]; then
  echo "usage: publish.sh <src-dir> <wiki-remote-url> <source-sha>" >&2
  exit 2
fi
src=$1 remote=$2 sha=$3

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

if ! git clone -q --depth 1 "$remote" "$tmp" 2>/dev/null; then
  echo "The wiki repository doesn't exist yet: create the first wiki page on GitHub, then run this again."
  exit 1
fi

rsync -a --delete --exclude .git "$src"/ "$tmp"/
git -C "$tmp" add -A

if git -C "$tmp" diff --cached --quiet; then
  echo "Nothing to publish"
  exit 0
fi

git -C "$tmp" \
  -c user.name='github-actions[bot]' \
  -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
  commit -q -m "Sync from ${sha:0:7}"
git -C "$tmp" push -q origin HEAD
