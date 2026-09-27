# Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Set up the project scaffold, the pull request checks, the automated release pipeline and automated dependency updates, so every later feature is built, checked, released and kept up to date the same way.

**Architecture:** A single Rust binary crate using eframe/egui opens the app window and shows a version set at build time. GitHub Actions run the checks on every pull request, and on every merge to `main` a release workflow calculates the version with git-cliff, publishes a release candidate as a pre-release, and after a maintainer's approval publishes the full release from the same commit.

**Tech Stack:** Rust (stable, edition 2024), eframe/egui 0.36.2, GitHub Actions, git-cliff 2.14.2, mise, Renovate, bash, `gh`.

**Spec:** `docs/superpowers/specs/2026-09-27-livesplit-asr-bridge-design.md` (sections 3, 4, 13, 14 and 17). Issues: #3, #4, #5, #24.

## Global Constraints

- Crate and executable name: `livesplit-asr-bridge` (spec §3).
- The window title names LiveSplit One (spec §3).
- `license = "MIT OR Apache-2.0"` (spec §17).
- The build-time version variable is `LIVESPLIT_ASR_BRIDGE_VERSION`; without it, the version is the `Cargo.toml` version `0.0.0-dev`. No version-bump commits on `main` (spec §14.1).
- Full release tags match `vX.Y.Z`; release candidate tags are `vX.Y.Z-rc.N` and never count as the previous version (spec §14.1).
- Bump rules: breaking → major, except below 1.0.0 → minor; otherwise `feat` → minor; otherwise `fix`/`perf` → patch; otherwise no release. The first release is 0.1.0 (spec §14.1).
- Allowed commit and PR title types: `feat`, `fix`, `perf`, `refactor`, `docs`, `test`, `build`, `ci`, `style`, `chore` (CONTRIBUTING.md).
- Checks run on pull requests only, on `ubuntu-latest`, `macos-latest` and `windows-latest` (spec §13).
- Release targets: `x86_64-unknown-linux-gnu` (`.tar.gz`), `aarch64-apple-darwin` and `x86_64-apple-darwin` (`.app` in a `.zip`, both built on `macos-latest`), `x86_64-pc-windows-msvc` (`.zip`) (spec §14.2).
- Action versions: `actions/checkout@v7`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `amannn/action-semantic-pull-request@v6`, `wagoid/commitlint-github-action@v6`, `jdx/mise-action@v4`, `actions/upload-artifact@v7`, `actions/download-artifact@v8`.
- Tool versions (git-cliff, shellcheck, actionlint) are defined only in `mise.toml`. Workflows install them with `jdx/mise-action@v4`, which reads that file, so versions are never repeated in workflows.
- Every commit is signed and follows Conventional Commits; each task is one pull request that links its issue. The implementer stops after opening the pull request: only the maintainer merges.
- Pull request descriptions follow `.github/pull_request_template.md`: `Closes #N` for the task's issue, what changed and why, how it was tested (the commands run and their results), and the checklist ticked. Write it to a file outside the repository (`PR_BODY=$(mktemp)`) and pass it with `--body-file "$PR_BODY"`.

## Review Focus

1. **An empty `LIVESPLIT_ASR_BRIDGE_VERSION`** (set but blank): the app should show the fallback version, not an empty one. Test in Task 1.
2. **Re-running the release workflow for the same commit** (for example after a failed build): it should reuse that commit's release candidate tag, not create `rc.N+1`. Test in Task 3.
3. **A merge with only non-releasing commits before any release exists** (for example a `docs:` commit): no `0.1.0` release candidate should be created, even though git-cliff prints `initial_tag` when there are no tags. Test in Task 3.
4. **Release candidate tags between full releases**: the version and the "all changes" notes must count from the last full release, not the last candidate. Test in Task 3.
5. **A breaking change below 1.0.0** must bump minor, never jump to 1.0.0. Test in Task 3.

A sixth condition cannot be tested locally and is checked on the first real release (Task 3, Step 14): a newer merge cancels an older release candidate that is still waiting for approval.

---

### Task 1: Project scaffold (issue #3)

**Branch:** `feat/project-scaffold` · **PR title:** `feat: add project scaffold with the application window` · **PR body:** `Closes #3`, using the pull request template.

**Files:**
- Create: `Cargo.toml`, `Cargo.lock` (generated), `rust-toolchain.toml`, `.gitignore`
- Create: `src/main.rs` (entry point: window options and `run_native`)
- Create: `src/version.rs` (the version shown in the app)
- Create: `src/ui.rs` (the app window)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `version::VERSION: &'static str` — the app version.
  - `version::resolve(build: Option<&'static str>, fallback: &'static str) -> &'static str` (`const fn`).
  - `ui::APP_NAME: &str = "livesplit-asr-bridge"`.
  - `ui::window_title() -> String`.
  - `ui::BridgeApp` implementing `eframe::App`.
  - Environment variable `LIVESPLIT_ASR_BRIDGE_VERSION`, read at compile time (used by Task 3's build workflow).

eframe 0.36 changed the `App` trait: the required method is `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame)`, and `CentralPanel::show` takes `&mut Ui`. Do not use the older `update(&mut self, ctx: &egui::Context, ...)`.

- [ ] **Step 1: Create the manifest, toolchain file and ignore file**

`Cargo.toml`:

```toml
[package]
name = "livesplit-asr-bridge"
version = "0.0.0-dev"
edition = "2024"
rust-version = "1.95"
description = "Run LiveSplit auto splitters on one machine and control LiveSplit One on another over its Connect to Server WebSocket."
license = "MIT OR Apache-2.0"
repository = "https://github.com/alexcosta97/livesplit-asr-bridge"
readme = "README.md"
publish = false

[dependencies]
eframe = "0.36.2"
```

`rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

`.gitignore`:

```
/target
/dist
```

- [ ] **Step 2: Write the failing version tests**

`src/version.rs`:

```rust
//! The application version, set at build time by the release pipeline.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_build_version_when_set() {
        assert_eq!(resolve(Some("1.2.3-rc.4"), "0.0.0-dev"), "1.2.3-rc.4");
    }

    #[test]
    fn falls_back_when_unset() {
        assert_eq!(resolve(None, "0.0.0-dev"), "0.0.0-dev");
    }

    #[test]
    fn falls_back_when_set_but_empty() {
        assert_eq!(resolve(Some(""), "0.0.0-dev"), "0.0.0-dev");
    }

    #[test]
    fn version_is_never_empty() {
        assert!(!VERSION.is_empty());
    }
}
```

`src/main.rs` (temporary, so the tests compile):

```rust
mod version;

fn main() {}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test`
Expected: compile errors `cannot find function 'resolve'` and `cannot find value 'VERSION'`.

- [ ] **Step 4: Implement the version**

Add to the top of `src/version.rs`, below the module doc comment:

```rust
/// The version shown in the app. The release pipeline sets
/// `LIVESPLIT_ASR_BRIDGE_VERSION` at build time; local builds show the
/// `Cargo.toml` version, `0.0.0-dev`. Cargo rebuilds automatically when the
/// variable changes, because `option_env!` is tracked.
pub const VERSION: &str = resolve(
    option_env!("LIVESPLIT_ASR_BRIDGE_VERSION"),
    env!("CARGO_PKG_VERSION"),
);

/// Picks the build-time version when it is set and not empty, otherwise the
/// fallback.
pub const fn resolve(build: Option<&'static str>, fallback: &'static str) -> &'static str {
    match build {
        Some(version) if !version.is_empty() => version,
        _ => fallback,
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test`
Expected: 4 tests pass.

- [ ] **Step 6: Write the failing window title tests**

`src/ui.rs`:

```rust
//! The application window.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::VERSION;

    #[test]
    fn title_names_livesplit_one() {
        assert!(window_title().contains("LiveSplit One"));
    }

    #[test]
    fn title_includes_app_name_and_version() {
        let title = window_title();
        assert!(title.starts_with(APP_NAME));
        assert!(title.contains(VERSION));
    }
}
```

Update `src/main.rs`:

```rust
mod ui;
mod version;

fn main() {}
```

- [ ] **Step 7: Run the tests to verify they fail**

Run: `cargo test`
Expected: compile errors `cannot find function 'window_title'` and `cannot find value 'APP_NAME'`.

- [ ] **Step 8: Implement the window**

Add to `src/ui.rs`, below the module doc comment:

```rust
use eframe::egui;

use crate::version::VERSION;

/// The application name, used for the window and, later, the config and log
/// folders.
pub const APP_NAME: &str = "livesplit-asr-bridge";

/// The window title. It names LiveSplit One, because "LiveSplit" alone usually
/// means the original Windows LiveSplit, which this app does not control.
pub fn window_title() -> String {
    format!("{APP_NAME} {VERSION}: run auto splitters here, control LiveSplit One anywhere")
}

/// The application. Later issues add the status bar and tabs.
pub struct BridgeApp;

impl eframe::App for BridgeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading(APP_NAME);
            ui.label(format!("Version {VERSION}"));
            ui.label("Run auto splitters here, control LiveSplit One anywhere.");
        });
    }
}
```

Replace `src/main.rs`:

```rust
mod ui;
mod version;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(ui::window_title())
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        ui::APP_NAME,
        options,
        Box::new(|_cc| Ok(Box::new(ui::BridgeApp))),
    )
}
```

- [ ] **Step 9: Run all checks**

Run:

```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked
cargo test --locked
```

Expected: no formatting differences, no clippy warnings, 6 tests pass. (`--locked` needs `Cargo.lock`, created by the first build.)

- [ ] **Step 10 (coordinator or maintainer, needs a display): Check the window manually**

Run `cargo run`. Expected: a window titled `livesplit-asr-bridge 0.0.0-dev: run auto splitters here, control LiveSplit One anywhere`, showing the heading and `Version 0.0.0-dev`.

Run `LIVESPLIT_ASR_BRIDGE_VERSION=1.2.3 cargo run`. Expected: title and label show `1.2.3`.

- [ ] **Step 11: Commit and open the pull request**

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml .gitignore src
git commit -m "feat: add project scaffold with the application window"
git push -u origin feat/project-scaffold
gh pr create --title "feat: add project scaffold with the application window" --body-file "$PR_BODY"
```

Stop here. The maintainer merges.

---

### Task 2: Pull request checks (issue #4)

**Branch:** `ci/pull-request-checks` · **PR title:** `ci: add pull request checks` · **PR body:** `Closes #4`, using the pull request template.

**Files:**
- Create: `.github/workflows/ci.yml` (Rust checks on three platforms; commit message check)
- Create: `.github/workflows/pr-title.yml` (PR title check)
- Create: `commitlint.config.mjs` (restricts commit types to the allowed list)
- Modify: `CONTRIBUTING.md` (the local check commands match CI exactly)
- Modify: `.github/pull_request_template.md` (checklist commands match CI)

**Interfaces:**
- Consumes: the crate from Task 1 (`cargo` commands only).
- Produces: check runs named `check (ubuntu-latest)`, `check (macos-latest)`, `check (windows-latest)`, `commitlint` and `pr-title`. Required status checks in ruleset `24058678` use these exact names.

`pr-title.yml` runs on `pull_request_target`, which always uses the workflow file from `main`. It therefore does not run on this task's own pull request, only on later ones.

- [ ] **Step 1: Write `ci.yml`**

```yaml
name: CI

on:
  pull_request:
    types: [opened, reopened, synchronize]

permissions:
  contents: read
  pull-requests: read

concurrency:
  group: ci-${{ github.event.pull_request.number }}
  cancel-in-progress: true

jobs:
  check:
    name: check (${{ matrix.os }})
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v7
      - name: Install Linux build dependencies
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy --all-targets --locked -- -D warnings
      - run: cargo build --locked
      - run: cargo test --locked

  commitlint:
    name: commitlint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: wagoid/commitlint-github-action@v6
```

- [ ] **Step 2: Write `pr-title.yml`**

```yaml
name: PR title

on:
  pull_request_target:
    types: [opened, reopened, edited, synchronize]

permissions:
  pull-requests: read

jobs:
  pr-title:
    name: pr-title
    runs-on: ubuntu-latest
    steps:
      - uses: amannn/action-semantic-pull-request@v6
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          types: |
            feat
            fix
            perf
            refactor
            docs
            test
            build
            ci
            style
            chore
```

This workflow never checks out pull request code, which is what makes `pull_request_target` safe for pull requests from forks.

- [ ] **Step 3: Write `commitlint.config.mjs`**

```js
// Commit types allowed in this project (see CONTRIBUTING.md). The default
// conventional config also allows `revert`, which this project does not use.
export default {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'type-enum': [
      2,
      'always',
      ['feat', 'fix', 'perf', 'refactor', 'docs', 'test', 'build', 'ci', 'style', 'chore'],
    ],
  },
};
```

- [ ] **Step 4: Make the documented commands match CI**

In `CONTRIBUTING.md`, replace the block under "Before pushing, run the same checks CI runs:" with:

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo build --locked
cargo test --locked
```

In `.github/pull_request_template.md`, replace the checks checklist item with:

```markdown
- [ ] `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`,
      `cargo build --locked` and `cargo test --locked` pass locally.
```

- [ ] **Step 5: Lint the workflows**

Run: `mise x actionlint@1.7.12 shellcheck@0.11.0 -- actionlint`
Expected: no output (no problems). shellcheck must be loaded too: actionlint runs it on every `run:` step, and fails if it only finds mise's placeholder.

- [ ] **Step 6: Commit, push and open the pull request**

```bash
git add .github/workflows commitlint.config.mjs CONTRIBUTING.md .github/pull_request_template.md
git commit -m "ci: add pull request checks"
git push -u origin ci/pull-request-checks
gh pr create --title "ci: add pull request checks" --body-file "$PR_BODY"
```

- [ ] **Step 7: Verify the checks pass and fail when they should**

Wait for the checks: `gh pr checks --watch`. Expected: `check (ubuntu-latest)`, `check (macos-latest)`, `check (windows-latest)` and `commitlint` pass.

Prove `commitlint` rejects a bad commit:

```bash
git commit --allow-empty -m "Bad commit message"
git push
gh pr checks --watch
```

Expected: `commitlint` fails. Then remove the commit:

```bash
git reset --hard HEAD~1
git push --force-with-lease
gh pr checks --watch
```

Expected: all checks pass again.

- [ ] **Step 8: Confirm the check run names**

Run: `gh api repos/alexcosta97/livesplit-asr-bridge/commits/$(git rev-parse HEAD)/check-runs --jq '.check_runs[].name' | sort`
Expected exactly: `check (macos-latest)`, `check (ubuntu-latest)`, `check (windows-latest)`, `commitlint`.

Stop here. The maintainer merges.

- [ ] **Step 9 (coordinator, after the merge): Require the checks on `main`**

Rulesets are replaced as a whole, so send every existing rule plus the new one:

```bash
gh api repos/alexcosta97/livesplit-asr-bridge/rulesets/24058678 \
  | jq '{name, target, enforcement, conditions, bypass_actors,
         rules: ([.rules[] | select(.type != "required_status_checks")] + [{
           type: "required_status_checks",
           parameters: {
             strict_required_status_checks_policy: false,
             do_not_enforce_on_create: false,
             required_status_checks: [
               {context: "check (ubuntu-latest)", integration_id: 15368},
               {context: "check (macos-latest)", integration_id: 15368},
               {context: "check (windows-latest)", integration_id: 15368},
               {context: "commitlint", integration_id: 15368},
               {context: "pr-title", integration_id: 15368}
             ]}}])}' \
  | gh api -X PUT repos/alexcosta97/livesplit-asr-bridge/rulesets/24058678 --input - \
      --jq '[.rules[].type]'
```

Expected: `["deletion","non_fast_forward","required_linear_history","required_signatures","pull_request","required_status_checks"]`. `15368` is the GitHub Actions app id.

- [ ] **Step 10 (coordinator): Verify on the next pull request**

On Task 3's pull request, `gh pr checks` lists all five checks, including `pr-title`, as required.

---

### Task 3: Release pipeline (issue #5)

**Branch:** `ci/release-pipeline` · **PR title:** `ci: add the release pipeline` · **PR body:** `Closes #5`, using the pull request template.

**Files:**
- Create: `cliff.toml` (version and release note rules)
- Create: `mise.toml` (pins git-cliff, shellcheck and actionlint for local use)
- Create: `scripts/release/next-version.sh` (decides whether to release, and the version and tags)
- Create: `scripts/release/notes.sh` (release notes for candidates and full releases)
- Create: `scripts/release/test-release-scripts.sh` (tests for both scripts, using temporary repositories)
- Create: `scripts/package.sh` (packages a release build for one target)
- Create: `.github/workflows/build.yml` (reusable: builds and packages every target)
- Create: `.github/workflows/release.yml` (release candidates, approval, full release)
- Create: `.github/workflows/release-scripts.yml` (runs the script tests when release files change)
- Modify: `CONTRIBUTING.md` (mentions `mise.toml` for the release tools)

**Interfaces:**
- Consumes: `LIVESPLIT_ASR_BRIDGE_VERSION` (Task 1).
- Produces:
  - `scripts/release/next-version.sh` — no arguments, run at the commit being released. Prints `key=value` lines, and appends them to `$GITHUB_OUTPUT` when set: `release` (`true`/`false`), `version` (`X.Y.Z`), `rc_version` (`X.Y.Z-rc.N`), `rc_tag` (`vX.Y.Z-rc.N`), `previous_rc_tag` (empty if none), `last_full_tag` (empty if none). When `release` is `false`, the other keys are empty.
  - `scripts/release/notes.sh rc [previous_rc_tag]` and `scripts/release/notes.sh full` — Markdown release notes on stdout, for the commit at `HEAD`.
  - `scripts/package.sh <target> <version>` — writes one archive to `dist/`.
  - `.github/workflows/build.yml` — `workflow_call` with inputs `version`, `ref` and `artifact-name`; uploads one artifact per target, named `<artifact-name>-<target>`.
  - Workflow `Release scripts`, run only on pull requests that change release files. Not a required check: GitHub keeps path-filtered required checks pending forever on other pull requests, so it is advisory.
  - GitHub environment `release`.
  - `mise.toml` as the only source of tool versions, read by `jdx/mise-action@v4` in the workflows and by Renovate (Task 4).

Key facts, verified against git-cliff 2.14.2:
- Release candidate tags are hidden from version calculation with `tag_pattern`, not `ignore_tags`. With `ignore_tags`, `--unreleased` starts at the last candidate and misses earlier commits.
- `git cliff --bumped-version` never signals "nothing to release": it prints the current tag again and exits 0. With no tags at all, it prints `initial_tag` even for `docs`-only history. So `next-version.sh` compares with the last full tag, and with no tags checks that at least one releasing commit exists.
- Don't use `no_increment_regex`; it would let a breaking `refactor!:` through without a bump. Skip non-releasing types in `commit_parsers`, with `protect_breaking_commits = true`.

- [ ] **Step 1: Write `cliff.toml`**

```toml
# Release rules: see spec §14 and CONTRIBUTING.md.
[changelog]
# Only the groups: scripts/release/notes.sh adds the headings.
body = """
{% for group, commits in commits | group_by(attribute="group") %}
### {{ group | striptags | trim }}
{% for commit in commits %}- {% if commit.scope %}**{{ commit.scope }}:** {% endif %}{{ commit.message | upper_first }}
{% endfor %}{% endfor %}
"""
trim = true

[git]
conventional_commits = true
filter_unconventional = true     # non-conventional commits never cause a release
filter_commits = true            # commits no parser matches are dropped
protect_breaking_commits = true  # a breaking docs/chore/refactor commit still bumps
# Only full releases count as tags. Release candidates are left out here, not
# with ignore_tags, so --unreleased always starts at the last full release.
tag_pattern = "^v[0-9]+\\.[0-9]+\\.[0-9]+$"
sort_commits = "oldest"
commit_parsers = [
  # field = "breaking" catches both `type!:` and a BREAKING CHANGE footer.
  { field = "breaking", pattern = "true", group = "<!-- 0 -->Breaking changes" },
  { message = "^feat", group = "<!-- 1 -->Features" },
  { message = "^fix", group = "<!-- 2 -->Bug fixes" },
  { message = "^perf", group = "<!-- 3 -->Performance" },
  # Skipped before the bump is worked out. Not no_increment_regex, which would
  # also ignore a breaking `refactor!:`.
  { message = "^(docs|chore|ci|test|refactor|style|build)", skip = true },
]

[bump]
features_always_bump_minor = true   # feat bumps minor, also below 1.0.0
breaking_always_bump_major = false  # breaking bumps minor below 1.0.0, major from 1.0.0
initial_tag = "v0.1.0"
```

- [ ] **Step 2: Write `mise.toml`**

```toml
# Tools for the release scripts and workflow linting. Rust itself comes from
# rustup via rust-toolchain.toml.
[tools]
git-cliff = "2.14.2"
shellcheck = "0.11.0"
actionlint = "1.7.12"
```

Run: `mise install`
Expected: the three tools install.

- [ ] **Step 3: Write the failing script tests**

`scripts/release/test-release-scripts.sh`:

```bash
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
  actual=$(next | sed -n "s/^$2=//p")
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
  notes=$("$root/scripts/release/notes.sh" "$@" 2>/dev/null)
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
```

Run: `chmod +x scripts/release/test-release-scripts.sh`

- [ ] **Step 4: Run the tests to verify they fail**

Run: `mise x -- scripts/release/test-release-scripts.sh`
Expected: every test prints `FAIL` (the scripts don't exist yet), and the script exits 1.

- [ ] **Step 5: Write `next-version.sh`**

`scripts/release/next-version.sh`:

```bash
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
```

Run: `chmod +x scripts/release/next-version.sh`

- [ ] **Step 6: Write `notes.sh`**

`scripts/release/notes.sh`:

```bash
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
```

Run: `chmod +x scripts/release/notes.sh`

- [ ] **Step 7: Run the tests to verify they pass**

Run: `mise x -- scripts/release/test-release-scripts.sh`
Expected: every line starts with `ok`, the last line is `all tests passed`, exit code 0.

Run: `mise x -- shellcheck scripts/release/*.sh`
Expected: no output.

- [ ] **Step 8: Write `package.sh`**

`scripts/package.sh`:

```bash
#!/usr/bin/env bash
# Packages the release build for one target into dist/.
#   package.sh <target> <version>
set -euo pipefail

target=$1
version=$2
name=livesplit-asr-bridge
bin_dir="target/$target/release"
out="$(pwd)/dist"
stage=$(mktemp -d)
mkdir -p "$out"

case "$target" in
  x86_64-unknown-linux-gnu)
    mkdir -p "$stage/$name"
    cp "$bin_dir/$name" README.md LICENSE-MIT LICENSE-APACHE "$stage/$name/"
    tar -C "$stage" -czf "$out/$name-$version-x86_64-linux.tar.gz" "$name"
    ;;
  aarch64-apple-darwin | x86_64-apple-darwin)
    arch=${target%%-*}
    [[ "$arch" == aarch64 ]] && arch=arm64
    app="$stage/$name.app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    cp "$bin_dir/$name" "$app/Contents/MacOS/$name"
    chmod +x "$app/Contents/MacOS/$name"
    cp README.md LICENSE-MIT LICENSE-APACHE "$app/Contents/Resources/"
    # Bundle versions must be numeric, so candidates use the X.Y.Z part.
    short=${version%%-*}
    cat >"$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>$name</string>
  <key>CFBundleIdentifier</key><string>io.github.alexcosta97.livesplit-asr-bridge</string>
  <key>CFBundleName</key><string>$name</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleSignature</key><string>????</string>
  <key>CFBundleShortVersionString</key><string>$short</string>
  <key>CFBundleVersion</key><string>$short</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF
    ditto -c -k --sequesterRsrc --keepParent "$app" "$out/$name-$version-$arch-macos.zip"
    ;;
  x86_64-pc-windows-msvc)
    mkdir -p "$stage/$name"
    cp "$bin_dir/$name.exe" README.md LICENSE-MIT LICENSE-APACHE "$stage/$name/"
    (cd "$stage" && 7z a -tzip "$out/$name-$version-x86_64-windows.zip" "$name" >/dev/null)
    ;;
  *)
    echo "unsupported target: $target" >&2
    exit 1
    ;;
esac

ls "$out"
```

Run: `chmod +x scripts/package.sh && mise x -- shellcheck scripts/package.sh`
Expected: no output.

- [ ] **Step 9: Check the Linux package locally**

```bash
LIVESPLIT_ASR_BRIDGE_VERSION=0.1.0-rc.1 cargo build --release --locked --target x86_64-unknown-linux-gnu
scripts/package.sh x86_64-unknown-linux-gnu 0.1.0-rc.1
tar -tzf dist/livesplit-asr-bridge-0.1.0-rc.1-x86_64-linux.tar.gz
```

Expected listing: `livesplit-asr-bridge/`, `livesplit-asr-bridge/livesplit-asr-bridge`, `livesplit-asr-bridge/README.md`, `livesplit-asr-bridge/LICENSE-MIT`, `livesplit-asr-bridge/LICENSE-APACHE`. Then `rm -rf dist`.

- [ ] **Step 10: Write the reusable build workflow**

`.github/workflows/build.yml`:

```yaml
name: Build

on:
  workflow_call:
    inputs:
      version:
        description: Version embedded in the app, e.g. 0.4.0 or 0.4.0-rc.2
        required: true
        type: string
      ref:
        description: Commit to build
        required: true
        type: string
      artifact-name:
        description: Prefix for the uploaded artifacts
        required: true
        type: string

permissions:
  contents: read

jobs:
  build:
    name: build (${{ matrix.target }})
    strategy:
      matrix:
        include:
          - { os: ubuntu-latest, target: x86_64-unknown-linux-gnu }
          - { os: macos-latest, target: aarch64-apple-darwin }
          - { os: macos-latest, target: x86_64-apple-darwin }
          - { os: windows-latest, target: x86_64-pc-windows-msvc }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v7
        with:
          ref: ${{ inputs.ref }}
      - name: Install Linux build dependencies
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}
      - name: Build
        shell: bash
        env:
          LIVESPLIT_ASR_BRIDGE_VERSION: ${{ inputs.version }}
        run: cargo build --release --locked --target ${{ matrix.target }}
      - name: Package
        shell: bash
        run: scripts/package.sh ${{ matrix.target }} ${{ inputs.version }}
      - uses: actions/upload-artifact@v7
        with:
          name: ${{ inputs.artifact-name }}-${{ matrix.target }}
          path: dist/*
          if-no-files-found: error
```

- [ ] **Step 11: Write the release workflow**

`.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    branches: [main]

permissions:
  contents: read

# A newer merge cancels an older run, including a release candidate still
# waiting for approval, so only the latest candidate can be promoted.
concurrency:
  group: release
  cancel-in-progress: true

jobs:
  version:
    name: version
    runs-on: ubuntu-latest
    outputs:
      release: ${{ steps.next.outputs.release }}
      version: ${{ steps.next.outputs.version }}
      rc_version: ${{ steps.next.outputs.rc_version }}
      rc_tag: ${{ steps.next.outputs.rc_tag }}
      previous_rc_tag: ${{ steps.next.outputs.previous_rc_tag }}
    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0
      - uses: jdx/mise-action@v4
      - id: next
        run: scripts/release/next-version.sh

  build-candidate:
    name: build candidate
    needs: version
    if: needs.version.outputs.release == 'true'
    uses: ./.github/workflows/build.yml
    with:
      version: ${{ needs.version.outputs.rc_version }}
      ref: ${{ github.sha }}
      artifact-name: candidate

  candidate:
    name: publish release candidate
    needs: [version, build-candidate]
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0
      - uses: jdx/mise-action@v4
      - uses: actions/download-artifact@v8
        with:
          pattern: candidate-*
          path: dist
          merge-multiple: true
      - name: Publish the pre-release
        env:
          GH_TOKEN: ${{ github.token }}
          RC_TAG: ${{ needs.version.outputs.rc_tag }}
          PREVIOUS_RC_TAG: ${{ needs.version.outputs.previous_rc_tag }}
        run: |
          scripts/release/notes.sh rc "$PREVIOUS_RC_TAG" > notes.md
          if gh release view "$RC_TAG" > /dev/null 2>&1; then
            gh release upload "$RC_TAG" dist/* --clobber
          else
            gh release create "$RC_TAG" dist/* --prerelease --target "$GITHUB_SHA" \
              --title "$RC_TAG" --notes-file notes.md
          fi

  approve:
    name: approve release
    needs: [version, candidate]
    runs-on: ubuntu-latest
    environment: release
    steps:
      - run: echo "Promoting ${{ needs.version.outputs.rc_tag }} to v${{ needs.version.outputs.version }}"

  build-final:
    name: build release
    needs: [version, approve]
    uses: ./.github/workflows/build.yml
    with:
      version: ${{ needs.version.outputs.version }}
      ref: ${{ github.sha }}
      artifact-name: final

  release:
    name: publish release
    needs: [version, build-final]
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0
      - uses: jdx/mise-action@v4
      - uses: actions/download-artifact@v8
        with:
          pattern: final-*
          path: dist
          merge-multiple: true
      - name: Publish the release
        env:
          GH_TOKEN: ${{ github.token }}
          TAG: v${{ needs.version.outputs.version }}
        run: |
          scripts/release/notes.sh full > notes.md
          gh release create "$TAG" dist/* --latest --target "$GITHUB_SHA" \
            --title "$TAG" --notes-file notes.md
```

- [ ] **Step 12: Add the release scripts workflow**

`.github/workflows/release-scripts.yml`:

```yaml
name: Release scripts

# Only runs when release files change, so it never slows down or blocks normal
# CI. For the same reason it is not a required check: GitHub keeps a
# path-filtered required check pending forever on pull requests that don't
# match the paths.
on:
  pull_request:
    types: [opened, reopened, synchronize]
    paths:
      - cliff.toml
      - mise.toml
      - scripts/**
      - .github/workflows/build.yml
      - .github/workflows/release.yml
      - .github/workflows/release-scripts.yml

permissions:
  contents: read

jobs:
  release-scripts:
    name: release-scripts
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: jdx/mise-action@v4
      - run: shellcheck scripts/*.sh scripts/release/*.sh
      - run: scripts/release/test-release-scripts.sh
```

Add to `CONTRIBUTING.md`, at the end of "Development setup":

```markdown
4. For the release scripts and workflow linting, install the pinned tools with
   [mise](https://mise.jdx.dev/) (`mise install`, see `mise.toml`), then run
   `scripts/release/test-release-scripts.sh` and `actionlint`. The same tests
   run on pull requests that change release files.
```

Run: `mise x -- actionlint`
Expected: no output.

- [ ] **Step 13: Commit, push, open the pull request and create the environment**

```bash
git add cliff.toml mise.toml scripts .github/workflows CONTRIBUTING.md
git commit -m "ci: add the release pipeline"
git push -u origin ci/release-pipeline
gh pr create --title "ci: add the release pipeline" --body-file "$PR_BODY"
gh pr checks --watch
```

Expected: all five required checks from Task 2 pass, and so does the `release-scripts` check from the new workflow.

Coordinator, before the merge: create the `release` environment with the maintainer (user id `23384791`) as required reviewer:

```bash
gh api -X PUT repos/alexcosta97/livesplit-asr-bridge/environments/release --input - <<'EOF'
{"wait_timer": 0, "prevent_self_review": false,
 "reviewers": [{"type": "User", "id": 23384791}],
 "deployment_branch_policy": null}
EOF
gh api repos/alexcosta97/livesplit-asr-bridge/environments/release \
  --jq '[.protection_rules[] | select(.type == "required_reviewers") | .reviewers[].reviewer.login]'
```

Expected: `["alexcosta97"]`.

Stop here. The maintainer merges.

- [ ] **Step 14 (coordinator, after the merge): Verify the first release**

This merge is `ci:`, but `main` already contains Task 1's `feat:` commit and no release exists yet, so the first run releases `0.1.0`:

1. `gh run watch` on the Release run. Expected: `version` outputs `release=true`, `version=0.1.0`, `rc_tag=v0.1.0-rc.1`; four builds succeed; pre-release `v0.1.0-rc.1` appears with four archives and notes listing "Add project scaffold with the application window"; `approve release` waits.
2. Download the Linux archive, run the app, and check the title shows `0.1.0-rc.1`.
3. The first time another pull request is merged while a candidate is still waiting for approval, check the cancellation behaviour (Review Focus, sixth condition): the older run's `approve release` job is cancelled. If it is not, fix the workflow in a follow-up pull request and record the finding in the spec.
4. When the maintainer approves: expected full release `v0.1.0` marked Latest on the same commit as `v0.1.0-rc.1`, and the app shows `0.1.0`.

---

### Task 4: Dependency updates with Renovate (issue #24)

**Branch:** `ci/renovate` · **PR title:** `ci: add Renovate configuration` · **PR body:** `Closes #24`, using the pull request template.

**Files:**
- Create: `renovate.json` (Renovate configuration)
- Modify: `CONTRIBUTING.md` (dependency update pull requests, and their exemption from the linked-issue rule)

**Interfaces:**
- Consumes: the required checks from Task 2, which run on Renovate's pull requests; `Cargo.toml` and `Cargo.lock` (Task 1); the workflows and `mise.toml` (Task 3).
- Produces: weekly Renovate pull requests titled `fix(deps): …` (crates that ship in the app, and `Cargo.lock` maintenance), `ci(deps): …` (GitHub Actions) or `chore(deps): …` (mise tools, development-only crates); the Dependency Dashboard issue; labels `dependencies` and `livesplit`.

Key facts, verified against the Renovate documentation and source:
- The configuration is committed through a normal pull request **before** the app is installed. The app then skips onboarding. Otherwise its onboarding pull request, titled `Configure Renovate`, could fail the Conventional Commits checks.
- `config:recommended` gives `fix` only to Cargo `dependencies`, not `workspace.dependencies`, so the rules below set types explicitly.
- For git dependencies (the LiveSplit crates, by `rev`, from issue #6), Renovate's package name is the git URL, so the LiveSplit rule matches on `matchDepNames`. Each upstream commit is a new digest, so the weekly schedule prevents a pull request per commit.
- `dtolnay/rust-toolchain@stable` is not a version, so Renovate leaves it alone, which is intended: it always uses the current stable Rust.
- Automerge is off by default and stays off: only the maintainer merges.

- [ ] **Step 1 (coordinator): Create the labels Renovate uses**

```bash
gh label create dependencies --color 0366D6 --description "Dependency updates, opened by Renovate"
gh label create livesplit --color D4A72C --description "Updates to the LiveSplit crates"
```

- [ ] **Step 2: Write `renovate.json`**

```json
{
  "$schema": "https://docs.renovatebot.com/renovate-schema.json",
  "extends": ["config:recommended", ":semanticCommits", "schedule:weekly"],
  "semanticCommitType": "chore",
  "semanticCommitScope": "deps",
  "platformCommit": "enabled",
  "automerge": false,
  "labels": ["dependencies"],
  "prConcurrentLimit": 5,
  "prHourlyLimit": 2,
  "lockFileMaintenance": { "enabled": true },
  "packageRules": [
    {
      "description": "Crates that ship in the app: fix, so the update is released",
      "matchManagers": ["cargo"],
      "matchDepTypes": ["dependencies", "workspace.dependencies"],
      "semanticCommitType": "fix"
    },
    {
      "description": "Development-only crates: no release",
      "matchManagers": ["cargo"],
      "matchDepTypes": ["dev-dependencies", "build-dependencies"],
      "semanticCommitType": "chore"
    },
    {
      "description": "GitHub Actions: no release",
      "matchManagers": ["github-actions"],
      "semanticCommitType": "ci"
    },
    {
      "description": "Tools pinned in mise.toml: no release",
      "matchManagers": ["mise"],
      "semanticCommitType": "chore"
    },
    {
      "description": "Cargo.lock refreshes change what ships: fix, so the update is released",
      "matchUpdateTypes": ["lockFileMaintenance"],
      "semanticCommitType": "fix"
    },
    {
      "description": "The LiveSplit crates, by git rev: grouped, labelled and prioritised",
      "matchManagers": ["cargo"],
      "matchDepNames": ["livesplit-*"],
      "groupName": "LiveSplit crates",
      "addLabels": ["livesplit"],
      "prPriority": 10,
      "semanticCommitType": "fix"
    }
  ]
}
```

- [ ] **Step 3: Validate the configuration**

Run: `mise x node@lts -- npx --yes --package renovate -- renovate-config-validator --strict renovate.json`
Expected: `Config validated successfully` and exit code 0.

- [ ] **Step 4: Document dependency updates**

In `CONTRIBUTING.md`, under "Before you start", change the first bullet's sentence "every pull request must link one" to "every pull request must link one, except Renovate's dependency updates".

Add this section after "Pull requests":

```markdown
## Dependency updates

[Renovate](https://docs.renovatebot.com/) opens pull requests every week to
update dependencies, configured in `renovate.json`. Their titles follow the
conventions above, and the type decides whether the update is released:

- `fix(deps)`: crates that ship in the app, including the LiveSplit crates and
  `Cargo.lock` refreshes. These produce a release.
- `ci(deps)`: GitHub Actions. No release.
- `chore(deps)`: tools pinned in `mise.toml` and development-only crates. No
  release.

Renovate's pull requests are the one exception to the linked-issue rule. They
go through the same checks and are merged by a maintainer like any other pull
request. The Dependency Dashboard issue lists pending updates.
```

- [ ] **Step 5: Commit, push and open the pull request**

```bash
git add renovate.json CONTRIBUTING.md
git commit -m "ci: add Renovate configuration"
git push -u origin ci/renovate
gh pr create --title "ci: add Renovate configuration" --body-file "$PR_BODY"
gh pr checks --watch
```

Expected: all required checks pass.

Stop here. The maintainer merges.

- [ ] **Step 6 (maintainer, after the merge): Install the Renovate app**

Install the Mend Renovate app from https://github.com/apps/renovate, selecting only `alexcosta97/livesplit-asr-bridge`.

- [ ] **Step 7 (coordinator): Verify Renovate works**

1. No `Configure Renovate` onboarding pull request is opened: `gh pr list --search "Configure Renovate"` returns nothing.
2. The Dependency Dashboard issue appears: `gh issue list --search "Dependency Dashboard"`.
3. When the first Renovate pull requests open, their titles use the types from Step 2, carry the `dependencies` label, and pass all required checks.
