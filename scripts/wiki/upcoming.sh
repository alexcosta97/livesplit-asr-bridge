#!/usr/bin/env bash
# Prints the text of each "## " heading in the Upcoming Changes page (default
# docs/wiki/Upcoming-Changes.md), one per line, skipping code fences. Prints
# nothing when there are no entries or the file is missing. Needs awk.
#   upcoming.sh [file]
set -euo pipefail

file=${1:-docs/wiki/Upcoming-Changes.md}
[[ -f "$file" ]] || exit 0

awk '/^```/ { fenced = !fenced; next } !fenced && /^## / { print substr($0, 4) }' "$file"
