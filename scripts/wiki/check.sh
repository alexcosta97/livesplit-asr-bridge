#!/usr/bin/env bash
# Checks the wiki pages in a directory (default docs/wiki): every link to a
# page or image resolves, and every page is linked from _Sidebar.md. Prints one
# line per problem and exits 1 if there are any. Needs only grep and sed.
#   check.sh [dir]
set -euo pipefail

dir=${1:-docs/wiki}
problems=0

problem() {
  echo "$1"
  problems=$((problems + 1))
}

# Link and image targets in a file, one per line: markdown `](target)` and
# HTML `src="target"`.
targets() {
  { grep -oE '\]\([^)[:space:]]+\)' "$1" | sed -E 's/^\]\((.*)\)$/\1/' || true; }
  { grep -oE 'src="[^"]+"' "$1" | sed -E 's/^src="(.*)"$/\1/' || true; }
}

shopt -s nullglob
for file in "$dir"/*.md; do
  while IFS= read -r target; do
    [[ -z "$target" || "$target" == *:* || "$target" == \#* ]] && continue
    target=${target%%#*}
    if [[ "$target" == images/* ]]; then
      [[ -f "$dir/$target" ]] || problem "$(basename "$file"): missing image $target"
    elif [[ "$target" == *.md ]]; then
      problem "$(basename "$file"): link $target should name the page without .md"
    else
      [[ -f "$dir/$target.md" ]] || problem "$(basename "$file"): link to missing page $target"
    fi
  done < <(targets "$file")
done

sidebar="$dir/_Sidebar.md"
if [[ ! -f "$sidebar" ]]; then
  problem "_Sidebar.md: missing"
else
  linked=$(targets "$sidebar" | sed 's/#.*//')
  for file in "$dir"/*.md; do
    name=$(basename "$file" .md)
    [[ "$name" == _* ]] && continue
    grep -qxF "$name" <<< "$linked" || problem "$name: page is not linked from _Sidebar.md"
  done
fi

[[ $problems -eq 0 ]]
