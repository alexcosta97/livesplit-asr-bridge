#!/usr/bin/env bash
# Works out whether the commits since the last full release need a release,
# and if so its version and release candidate tag. Run at the commit being
# released, with all tags fetched. Prints key=value lines, also appended to
# $GITHUB_OUTPUT when set.
set -euo pipefail

emit() {
  echo "$1=$2"
  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    echo "$1=$2" >>"$GITHUB_OUTPUT"
  fi
}

last_full_tag=$(git describe --tags --abbrev=0 --match 'v[0-9]*.[0-9]*.[0-9]*' --exclude '*-*' 2>/dev/null || true)
next=$(git cliff --bumped-version 2>/dev/null)

# git-cliff prints the current tag when nothing needs a release, and
# initial_tag when there are no tags, so check both cases explicitly.
if [[ -n "$last_full_tag" ]]; then
  [[ "$next" != "$last_full_tag" ]] && release=true || release=false
else
  releasing=$(git cliff --unreleased --context 2>/dev/null | jq '[.[].commits | length] | add // 0')
  ((releasing > 0)) && release=true || release=false
fi

version="" rc_version="" rc_tag="" previous_rc_tag=""
if [[ "$release" == true ]]; then
  version=${next#v}
  existing=$(git tag --points-at HEAD --list "v${version}-rc.*" | sort -V | tail -n1)
  last_rc=$(git tag --list "v${version}-rc.*" | sed -E 's/.*-rc\.([0-9]+)$/\1/' | sort -n | tail -n1)
  if [[ -n "$existing" ]]; then
    # A re-run for a commit that already has a candidate reuses it.
    rc_tag=$existing
    number=${existing##*-rc.}
    previous=$(git tag --list "v${version}-rc.*" | sed -E 's/.*-rc\.([0-9]+)$/\1/' | sort -n | awk -v n="$number" '$1 < n' | tail -n1)
  else
    number=$((${last_rc:-0} + 1))
    rc_tag="v${version}-rc.${number}"
    previous=$last_rc
  fi
  rc_version="${version}-rc.${number}"
  [[ -n "$previous" ]] && previous_rc_tag="v${version}-rc.${previous}"
fi

emit release "$release"
emit version "$version"
emit rc_version "$rc_version"
emit rc_tag "$rc_tag"
emit previous_rc_tag "$previous_rc_tag"
emit last_full_tag "$last_full_tag"
