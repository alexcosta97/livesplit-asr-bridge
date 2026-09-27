#!/usr/bin/env bash
# Release notes for the commit at HEAD, on stdout.
#   notes.sh rc [previous_rc_tag]  notes for a release candidate
#   notes.sh full                  notes for a full release
set -euo pipefail

last_full_tag=$(git describe --tags --abbrev=0 --match 'v[0-9]*.[0-9]*.[0-9]*' --exclude '*-*' 2>/dev/null || true)
since=${last_full_tag:-the start of the project}

section() { # heading, git-cliff range arguments...
  local heading=$1 body
  shift
  body=$(git cliff "$@" --strip all 2>/dev/null)
  printf '## %s\n\n' "$heading"
  if [[ -n "${body//[[:space:]]/}" ]]; then
    printf '%s\n\n' "$body"
  else
    printf 'No user-facing changes.\n\n'
  fi
}

case "${1:-}" in
  rc)
    if [[ -n "${2:-}" ]]; then
      section "New since $2" "$2..HEAD"
    fi
    section "All changes since $since" --unreleased
    ;;
  full)
    section "Changes since $since" --unreleased
    ;;
  *)
    echo "usage: notes.sh rc [previous_rc_tag] | notes.sh full" >&2
    exit 2
    ;;
esac
