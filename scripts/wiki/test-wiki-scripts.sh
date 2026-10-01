#!/usr/bin/env bash
# Tests check.sh, publish.sh and upcoming.sh against temporary directories and a local bare
# repository standing in for the wiki. Needs git and rsync.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
failures=0

# Isolate from the user's git config (signing, hooks, default branch).
export GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

pass() { echo "ok   - $1"; }
fail() {
  echo "FAIL - $1"
  failures=$((failures + 1))
}

# A tree that passes every check.
valid_tree() { # directory
  mkdir -p "$1/images"
  printf '# Home\n\n[A](Page-A)\n\n<img src="images/x.png">\n' > "$1/Home.md"
  printf '# A\n' > "$1/Page-A.md"
  printf '[Home](Home)\n[A](Page-A)\n' > "$1/_Sidebar.md"
  printf 'png' > "$1/images/x.png"
}

expect_check() { # description, directory, "pass" or a substring the output must contain
  local description=$1 dir=$2 expected=$3 output status=0
  # `|| status=$?` so a failing script reports FAIL instead of ending the run.
  output=$("$root/scripts/wiki/check.sh" "$dir" 2>&1) || status=$?
  if [[ "$expected" == pass ]]; then
    if [[ $status -eq 0 ]]; then pass "$description"; else fail "$description: exited $status: $output"; fi
  elif [[ $status -ne 0 && "$output" == *"$expected"* ]]; then
    pass "$description"
  else
    fail "$description: exit $status, output '$output', expected a failure naming '$expected'"
  fi
}

# check.sh
d="$tmp/valid"
valid_tree "$d"
expect_check "check: a valid tree passes" "$d" pass

d="$tmp/missing-page"
valid_tree "$d"
echo '[B](Page-B)' >> "$d/Home.md"
expect_check "check: a link to a missing page fails and names it" "$d" Page-B

d="$tmp/anchor"
valid_tree "$d"
echo '[B](Page-A#section)' >> "$d/Home.md"
expect_check "check: an anchor is stripped from a page link" "$d" pass

d="$tmp/md-suffix"
valid_tree "$d"
echo '[B](Page-A.md)' >> "$d/Home.md"
expect_check "check: a link with a .md suffix fails" "$d" Page-A.md

d="$tmp/external"
valid_tree "$d"
printf '[w](https://example.com/x)\n[m](mailto:a@example.com)\n[s](#section)\n' >> "$d/Home.md"
expect_check "check: external, mailto and anchor-only links are ignored" "$d" pass

d="$tmp/missing-image"
valid_tree "$d"
echo '![x](images/missing.png)' >> "$d/Home.md"
expect_check "check: a missing markdown image fails and names it" "$d" images/missing.png

d="$tmp/missing-img-tag"
valid_tree "$d"
echo '<img src="images/gone.png">' >> "$d/Home.md"
expect_check "check: a missing img tag image fails and names it" "$d" images/gone.png

d="$tmp/orphan"
valid_tree "$d"
printf '# Orphan\n' > "$d/Orphan.md"
expect_check "check: a page not linked from the sidebar fails and names it" "$d" Orphan

d="$tmp/footer"
valid_tree "$d"
printf 'footer\n' > "$d/_Footer.md"
expect_check "check: _Footer.md needs no sidebar link" "$d" pass

# publish.sh
remote="$tmp/wiki.git"
git init -q --bare -b master "$remote"
seed="$tmp/seed"
git clone -q "$remote" "$seed" 2>/dev/null
printf 'old home\n' > "$seed/Home.md"
printf 'old\n' > "$seed/Old.md"
git -C "$seed" add -A
git -C "$seed" -c user.name=seed -c user.email=seed@example.com commit -q -m seed
git -C "$seed" push -q origin HEAD

src="$tmp/src"
valid_tree "$src"
sha=0123456789abcdef0123456789abcdef01234567

output=$("$root/scripts/wiki/publish.sh" "$src" "$remote" "$sha" 2>&1) || fail "publish: exited non-zero: $output"
check="$tmp/check"
git clone -q "$remote" "$check" 2>/dev/null
if [[ -f "$check/Page-A.md" && -f "$check/images/x.png" && "$(cat "$check/Home.md")" == "$(cat "$src/Home.md")" ]]; then
  pass "publish: publishes the tree"
else
  fail "publish: the tree was not published"
fi
if [[ ! -e "$check/Old.md" ]]; then pass "publish: removes a page that is gone"; else fail "publish: Old.md is still there"; fi
subject=$(git -C "$check" log -1 --format=%s)
if [[ "$subject" == "Sync from 0123456" ]]; then pass "publish: commit message names the source"; else fail "publish: commit message is '$subject'"; fi

before=$(git -C "$remote" rev-list --count HEAD)
output=$("$root/scripts/wiki/publish.sh" "$src" "$remote" "$sha" 2>&1) || fail "publish: second run exited non-zero: $output"
after=$(git -C "$remote" rev-list --count HEAD)
if [[ "$output" == *"Nothing to publish"* && "$before" == "$after" ]]; then
  pass "publish: an unchanged tree publishes nothing"
else
  fail "publish: unchanged run printed '$output', commits $before -> $after"
fi

status=0
output=$("$root/scripts/wiki/publish.sh" "$src" "$tmp/nope.git" "$sha" 2>&1) || status=$?
if [[ $status -ne 0 && "$output" == *"create the first wiki page"* ]]; then
  pass "publish: a missing wiki repository fails with guidance"
else
  fail "publish: missing remote: exit $status, output '$output'"
fi

# upcoming.sh
expect_upcoming() { # description, expected output, file
  local description=$1 expected=$2 file=$3 output status=0
  output=$("$root/scripts/wiki/upcoming.sh" "$file" 2>&1) || status=$?
  if [[ $status -eq 0 && "$output" == "$expected" ]]; then
    pass "$description"
  else
    fail "$description: exit $status, output '$output', expected '$expected'"
  fi
}

printf '# Upcoming Changes\n\nNothing yet.\n' > "$tmp/none.md"
expect_upcoming "upcoming: no entries prints nothing" "" "$tmp/none.md"

printf '# Upcoming Changes\n\n## Connecting LiveSplit One\n\ntext\n\n## Logs and Files\n\ntext\n' > "$tmp/two.md"
expect_upcoming "upcoming: prints each heading in order" $'Connecting LiveSplit One\nLogs and Files' "$tmp/two.md"

# shellcheck disable=SC2016 # the backticks are literal markdown
printf '# Upcoming Changes\n\n```\n## Not a heading\n```\n\n## Real\n' > "$tmp/fenced.md"
expect_upcoming "upcoming: a heading inside a code fence is not counted" "Real" "$tmp/fenced.md"

expect_upcoming "upcoming: a missing file prints nothing" "" "$tmp/absent.md"

if [[ $failures -gt 0 ]]; then
  echo "$failures failure(s)"
  exit 1
fi
