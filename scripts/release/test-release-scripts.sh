#!/usr/bin/env bash
# Tests next-version.sh and notes.sh against temporary git repositories.
# Needs git, git-cliff and jq.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
failures=0

# Isolate from the user's git config (signing, hooks, default branch).
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1

new_repo() {
  repo=$(mktemp -d)
  cd "$repo"
  git init -q -b main
  git config user.name test
  git config user.email test@example.com
  cp "$root/cliff.toml" .
}

commit() { git commit -q --allow-empty -m "$1"; }
tag() { git tag "$1"; }

next() { "$root/scripts/release/next-version.sh" 2>/dev/null; }

expect() { # description, key, expected value
  local actual
  # `|| true` so a script that fails reports FAIL instead of ending the run.
  actual=$(next | sed -n "s/^$2=//p") || true
  if [[ "$actual" == "$3" ]]; then
    echo "ok   - $1"
  else
    echo "FAIL - $1: $2 is '$actual', expected '$3'"
    failures=$((failures + 1))
  fi
}

expect_notes() { # description, expected substring, notes.sh args...
  local description=$1 needle=$2 notes
  shift 2
  notes=$("$root/scripts/release/notes.sh" "$@" 2>/dev/null) || true
  if [[ "$notes" == *"$needle"* ]]; then
    echo "ok   - $description"
  else
    echo "FAIL - $description: notes do not contain '$needle':"
    echo "$notes"
    failures=$((failures + 1))
  fi
}

# No releases yet.
new_repo
commit "chore: initial commit"
commit "docs: add readme"
expect "docs-only history before any release: no release" release false

commit "feat: add window"
expect "first releasing commit: release" release true
expect "first release is 0.1.0" version 0.1.0
expect "first release candidate is rc.1" rc_tag v0.1.0-rc.1
expect "no previous candidate" previous_rc_tag ""
tag v0.1.0-rc.1
expect "re-run on a tagged commit reuses its tag" rc_tag v0.1.0-rc.1

commit "fix: correct title"
expect "second candidate keeps the version" version 0.1.0
expect "second candidate is rc.2" rc_tag v0.1.0-rc.2
expect "previous candidate is rc.1" previous_rc_tag v0.1.0-rc.1
expect_notes "candidate notes list what is new since rc.1" "Correct title" rc v0.1.0-rc.1
expect_notes "candidate notes include all changes" "Add window" rc v0.1.0-rc.1
tag v0.1.0-rc.2

# First full release.
tag v0.1.0
commit "docs: explain setup"
expect "docs-only since a full release: no release" release false

commit "fix: handle reconnects"
expect "fix bumps patch" version 0.1.1
expect "count restarts for a new version" rc_tag v0.1.1-rc.1
expect "last full release found" last_full_tag v0.1.0
tag v0.1.1-rc.1

commit "feat: add port setting"
expect "feat after a fix bumps minor, counting from the last full release" version 0.2.0
expect "count restarts when the version changes" rc_tag v0.2.0-rc.1
expect_notes "full notes count from the last full release, not the candidate" "Handle reconnects" full

commit "feat!: change settings format"
expect "breaking change below 1.0.0 bumps minor" version 0.2.0

# From 1.0.0, breaking changes bump major.
tag v1.0.0
commit "refactor!: drop old config"
expect "breaking refactor from 1.0.0 bumps major" version 2.0.0

if ((failures > 0)); then
  echo "$failures test(s) failed"
  exit 1
fi
echo "all tests passed"
