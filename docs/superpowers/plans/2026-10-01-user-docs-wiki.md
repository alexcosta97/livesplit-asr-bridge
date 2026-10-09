# User Documentation Wiki Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish the user documentation for the first release as the
repository's GitHub wiki, written in `docs/wiki/` and synced on merge to
`main` (issue #16).

**Architecture:** Pages are Markdown in `docs/wiki/`, checked on pull
requests by `scripts/wiki/check.sh` (page links, images, sidebar coverage)
and mirrored into `livesplit-asr-bridge.wiki` by `.github/workflows/wiki.yml`
on push to `main`. The release job opens an issue after a full release when
the Upcoming Changes page has entries. Agent and contributor rules require
documentation in the same branch as every change users can see.

**Tech Stack:** GitHub wiki (Gollum Markdown), Bash, GitHub Actions,
shellcheck and actionlint (pinned in `mise.toml`).

**Spec:** `docs/superpowers/specs/2026-09-27-livesplit-asr-bridge-design.md`,
section 15 (with 6, 7, 8, 9, 10 and 11 as the source of the page contents).

**Branch and worktree:** `docs/user-wiki`, at
`../livesplit-asr-bridge-wt/user-wiki`. Pull request #73.

## Global Constraints

- The app is always named **LiveSplit One ASR Bridge**; `livesplit-asr-bridge`
  only for files, folders and commands (`assets/brand/BRAND.md`).
- Voice: plain and direct, short sentences, the words the app shows, no
  marketing superlatives (`BRAND.md`, "Voice").
- Button, tab and status names in bold, spelled exactly as the app shows them.
  Check each one against `src/ui/` before writing it.
- Each fact lives on one page; other pages link to it (spec §15, §15.2).
- Page links use page names: `[Troubleshooting](Troubleshooting)`,
  `[Logs and Files](Logs-and-Files)`. Images: `images/<subject>.png`.
- There is no full release yet, so pages describe `main` and Upcoming
  Changes has no entries (spec §15.3).
- Screenshots don't exist yet. Where one belongs, write the line
  `<!-- screenshot: <ID> <file> -->` followed by a one-sentence text
  description of the screen; the image replaces the comment later (§15.4).
- Log folders (§8.2): Linux `$XDG_STATE_HOME/livesplit-asr-bridge/logs/`
  (default `~/.local/state/livesplit-asr-bridge/logs/`), macOS
  `~/Library/Logs/livesplit-asr-bridge/`, Windows
  `%LOCALAPPDATA%\livesplit-asr-bridge\logs\`.
- Config folders (`dirs::config_dir()` + `livesplit-asr-bridge`): Linux
  `$XDG_CONFIG_HOME/livesplit-asr-bridge/` (default
  `~/.config/livesplit-asr-bridge/`), macOS
  `~/Library/Application Support/livesplit-asr-bridge/`, Windows
  `%APPDATA%\livesplit-asr-bridge\`.
- Release assets: `livesplit-asr-bridge-<version>-x86_64-linux.tar.gz`,
  `-arm64-macos.zip` (Apple Silicon), `-x86_64-macos.zip` (Intel),
  `-x86_64-windows.zip`. Download link: `/releases/latest`.
- Commits: Conventional Commits, signed, ending with
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **A bold UI name that doesn't match the app** (a renamed tab, "Settings"
   instead of **Splitter settings**): readers search for it and find nothing.
   Every doc task ends with a step that greps each bold term in `src/ui/`.
2. **Links that work in `docs/wiki/` but not in the published wiki**, or the
   reverse: `check.sh` pins page and image links; Task 10 checks the
   published wiki by hand.
3. **The sync on a wiki repository that doesn't exist yet** (the first page
   was never created): the publish script must fail with a message saying
   what to do, not a bare git error. Task 2 tests it.
4. **A sync that deletes the wiki's `.git`, or pushes when nothing changed**:
   Task 2 tests both against a local bare repository.
5. **The Upcoming Changes check failing the release**: the issue step runs
   after the release is published, and an empty page opens no issue. Task 3
   tests the entry count.

---

### Task 1: Rules for documentation in every change

**Files:**
- Modify: `AGENTS.md` (section "How work is run")
- Modify: `CONTRIBUTING.md` (new section "User documentation", before
  "Commits")
- Modify: `.github/pull_request_template.md` (checklist)

- [ ] **Step 1: Add to `AGENTS.md`, after the paragraph on opening a pull
  request:**

```markdown
Every change users can see is documented in the same branch as the change.
The user documentation is the wiki, written in `docs/wiki/` (spec §15):
update the pages it affects, or add an entry to `docs/wiki/Upcoming-Changes.md`
once a full release exists (§15.3). If a screen in a screenshot changes, say
in the pull request which screenshots need retaking. A pull request with a
change users can see and no documentation is not ready for review.
```

- [ ] **Step 2: Add to `CONTRIBUTING.md`, before "## Commits":**

```markdown
## User documentation

The user documentation is the
[wiki](https://github.com/alexcosta97/livesplit-asr-bridge/wiki). Its pages
are written in `docs/wiki/` and published automatically when a pull request
merges; don't edit the wiki directly, since the next merge replaces it.

- A pull request that changes something users can see updates the
  documentation in the same pull request.
- The wiki describes the latest full release. Documentation for a change that
  isn't released yet goes on `docs/wiki/Upcoming-Changes.md`, under a heading
  naming the page it belongs on. When a full release is published, an issue
  is opened to move those entries into their pages.
- Link pages by name (`[Troubleshooting](Troubleshooting)`) and put images in
  `docs/wiki/images/`. `scripts/wiki/check.sh` checks links and images, and
  runs on pull requests that change the wiki.
- If a screen in a screenshot changes, say in the pull request which
  screenshots need retaking. The rules for screenshots are in spec §15.4.
```

- [ ] **Step 3: In the PR template, replace
  `- [ ] User-facing changes are reflected in the README or documentation.`
  with:**

```markdown
- [ ] Changes users can see are documented in `docs/wiki/` (or on Upcoming
      Changes), and any screenshots to retake are listed.
```

- [ ] **Step 4: Commit**

```bash
git add AGENTS.md CONTRIBUTING.md .github/pull_request_template.md
git commit -m "docs: require user documentation with every change users can see"
```

### Task 2: Wiki check and publish scripts, and the workflow

**Files:**
- Create: `scripts/wiki/check.sh`, `scripts/wiki/publish.sh`,
  `scripts/wiki/test-wiki-scripts.sh`
- Create: `.github/workflows/wiki.yml`

**Interfaces:**
- Produces: `scripts/wiki/check.sh [dir]` (default `docs/wiki`): prints one
  line per problem, exits 1 if any. `scripts/wiki/publish.sh <src-dir>
  <wiki-remote-url> <source-sha>`: mirrors and pushes; prints "Nothing to
  publish" and exits 0 when unchanged.

- [ ] **Step 1: Write `test-wiki-scripts.sh`**, in the style of
  `scripts/release/test-release-scripts.sh` (`ok   - …` / `FAIL - …` lines, a
  failure counter, `GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1`, temp
  dirs). Cases:
  - check: a valid tree (`Home.md` linking `[A](Page-A)`, `Page-A.md`,
    `_Sidebar.md` linking both, `images/x.png` referenced by
    `<img src="images/x.png">`) passes;
  - check: `[B](Page-B)` with no `Page-B.md` fails and names `Page-B`;
  - check: `[B](Page-A#section)` passes (anchor stripped);
  - check: `https://…`, `mailto:` and `#anchor`-only links are ignored;
  - check: `![x](images/missing.png)` fails and names the file;
  - check: a page not linked from `_Sidebar.md` fails and names it
    (`_Sidebar.md`, `_Footer.md` themselves exempt);
  - publish: into a local bare repo (`git init --bare`, seeded with one commit
    holding `Home.md` and `Old.md`), publishes the tree, removes `Old.md`,
    commit message is `Sync from <sha>`;
  - publish: running again with no change prints `Nothing to publish` and
    adds no commit;
  - publish: a remote that doesn't exist exits non-zero with a message
    containing `create the first wiki page`.

- [ ] **Step 2: Run it; every case fails** (scripts don't exist).

Run: `scripts/wiki/test-wiki-scripts.sh`

- [ ] **Step 3: Write `check.sh`.** `set -euo pipefail`. For each `*.md` in
  the dir: extract `](target)` and `src="target"` targets with `grep -o`;
  skip ones with a scheme (`*:*`) or starting with `#`; strip `#…`; targets
  starting with `images/` must exist as files; others must exist as
  `<target>.md`. Then every page other than `_*.md` must appear as a
  `](Name)` target in `_Sidebar.md` (`Home` included).

- [ ] **Step 4: Write `publish.sh`.** `git clone --depth 1 "$2" "$tmp"`; on
  failure print `The wiki repository doesn't exist yet: create the first wiki
  page on GitHub, then run this again.` and exit 1. Then
  `rsync -a --delete --exclude .git "$1"/ "$tmp"/`, `git -C "$tmp" add -A`; if
  `git -C "$tmp" diff --cached --quiet`, print `Nothing to publish` and exit
  0; else commit as `github-actions[bot]`
  (`41898282+github-actions[bot]@users.noreply.github.com`) with
  `Sync from ${3:0:7}` and push.

- [ ] **Step 5: Run the tests; all pass. Run shellcheck.**

Run: `scripts/wiki/test-wiki-scripts.sh && shellcheck scripts/wiki/*.sh`

- [ ] **Step 6: Write `.github/workflows/wiki.yml`:**

```yaml
name: Wiki

# Publishes docs/wiki/ to the repository's wiki (spec §15.1). Path-filtered,
# so like release-scripts.yml it is not a required check.
on:
  pull_request:
    types: [opened, reopened, synchronize]
    paths:
      - docs/wiki/**
      - scripts/wiki/**
      - .github/workflows/wiki.yml
  push:
    branches: [main]
    paths:
      - docs/wiki/**
  workflow_dispatch:

permissions:
  contents: read

jobs:
  check:
    name: wiki-check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: jdx/mise-action@v4
      - run: shellcheck scripts/wiki/*.sh
      - run: scripts/wiki/test-wiki-scripts.sh
      - run: scripts/wiki/check.sh docs/wiki

  publish:
    name: publish wiki
    needs: check
    if: github.event_name != 'pull_request'
    runs-on: ubuntu-latest
    permissions:
      contents: write
    concurrency:
      group: wiki
    steps:
      - uses: actions/checkout@v7
      - name: Publish
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          scripts/wiki/publish.sh docs/wiki \
            "https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY}.wiki.git" \
            "$GITHUB_SHA"
```

- [ ] **Step 7: Lint and commit**

Run: `actionlint`

```bash
git add scripts/wiki .github/workflows/wiki.yml
git commit -m "ci: check the wiki on pull requests and publish it on merge"
```

### Task 3: Open an issue for upcoming documentation after a full release

**Files:**
- Create: `scripts/wiki/upcoming.sh`
- Modify: `scripts/wiki/test-wiki-scripts.sh`
- Modify: `.github/workflows/release.yml` (job `release`)

**Interfaces:**
- Produces: `scripts/wiki/upcoming.sh [file]` (default
  `docs/wiki/Upcoming-Changes.md`): prints each `## ` heading's text, one per
  line; prints nothing for no entries or a missing file; exits 0.

- [ ] **Step 1: Add tests:** a file with `# Upcoming Changes`, a paragraph,
  and no `## ` prints nothing; with `## Connecting LiveSplit One` and
  `## Logs and Files` prints both lines in order; a `## ` inside a fenced
  code block is not counted; a missing file prints nothing and exits 0.

- [ ] **Step 2: Run; the new cases fail.**

- [ ] **Step 3: Write `upcoming.sh`** with `awk`: toggle a flag on lines
  starting with a code fence, and print `substr($0, 4)` for `^## ` lines
  outside fences.

- [ ] **Step 4: Run tests and shellcheck; all pass.**

- [ ] **Step 5: In `release.yml`, job `release`**, add `issues: write` to
  `permissions`, and after "Publish the release" add:

```yaml
      - name: Ask to move upcoming documentation into the wiki
        env:
          GH_TOKEN: ${{ github.token }}
          TAG: v${{ needs.version.outputs.version }}
        run: |
          entries=$(scripts/wiki/upcoming.sh)
          if [[ -n "$entries" ]]; then
            {
              echo "$TAG is released, so move its documentation from Upcoming Changes into the wiki pages (spec §15.3), then empty Upcoming Changes."
              echo
              echo "Entries:"
              while IFS= read -r entry; do echo "- $entry"; done <<< "$entries"
            } > body.md
            gh issue create --title "Move upcoming documentation into the wiki for $TAG" \
              --label task --body-file body.md
          fi
```

  `release-scripts.yml` stays unchanged: `wiki.yml` runs these tests.

- [ ] **Step 6: `actionlint`, commit**

```bash
git add scripts/wiki .github/workflows/release.yml
git commit -m "ci: open an issue to move upcoming documentation after a release"
```

### Task 4: Wiki skeleton: Home, sidebar, footer, Upcoming Changes, diagram

**Files:**
- Create: `docs/wiki/Home.md`, `docs/wiki/_Sidebar.md`,
  `docs/wiki/_Footer.md`, `docs/wiki/Upcoming-Changes.md`,
  `docs/wiki/images/two-pcs.svg`

- [ ] **Step 1: `_Sidebar.md`**: a list linking,
  in order: Home, Installing, Getting Started, Connecting LiveSplit One,
  Auto Splitters and Settings, The Main Window, Logs and Files, Platform
  Notes, Troubleshooting, Upcoming Changes.
- [ ] **Step 2: `_Footer.md`**: one line: questions and bugs go to the
  [issues](https://github.com/alexcosta97/livesplit-asr-bridge/issues); these
  pages describe the latest release, and what's coming is on Upcoming Changes.
- [ ] **Step 3: `Upcoming-Changes.md`**: title, one paragraph explaining the
  page (§15.3), and "Nothing yet. The documentation matches the latest
  release." No `## ` headings.
- [ ] **Step 4: `images/two-pcs.svg`**: two machine nodes joined by a link,
  as in the logo (§6.10, `assets/brand/logo/icon.svg`): left, "Game PC" with
  "LiveSplit One ASR Bridge · runs the auto splitter" in paper `#F5F5F4`;
  right, "Timer PC" with "LiveSplit One in Chrome" in orange `#FF4D00`; the
  link labelled `ws://…:16834`, with commands flowing right. Background ink
  `#0B0B0C`, 1 px borders `#2A2A2E`, 4 px corners, text as system sans-serif
  (SVG in the wiki can't load the brand fonts). About 720 × 220.
- [ ] **Step 5: `Home.md`**: the tagline, two short paragraphs from the
  README's intro and spec §1 (naming LiveSplit One, and that the original
  Windows LiveSplit isn't supported yet), `<img src="images/two-pcs.svg"
  width="720" alt="…">`, `<!-- screenshot: S1 main-window.png -->` with a
  one-line description, "Start here": Installing then Getting Started, then
  one line per other page.
- [ ] **Step 6: Run `scripts/wiki/check.sh docs/wiki`.** It fails only on the
  sidebar links to pages not written yet; that is expected until Task 9.
- [ ] **Step 7: Commit** `docs(wiki): add the home page, sidebar and upcoming changes`

### Task 5: Installing, and the slimmer README

**Files:**
- Create: `docs/wiki/Installing.md`
- Modify: `README.md`

- [ ] **Step 1: `Installing.md`**, sections:
  - **Downloading**: the four assets from Global Constraints and which to
    pick (Apple Silicon vs Intel: Apple menu → About This Mac). Full
    releases vs **Pre-release** candidates.
  - **Opening the app for the first time**: move the README's "Opening the
    app" text here unchanged in substance. macOS (**Done**, System Settings →
    Privacy & Security → **Open Anyway**) with
    `<!-- screenshot: T3 macos-open-anyway.png -->`; Windows (**More info**,
    **Run anyway**) with `<!-- screenshot: T4 windows-smartscreen.png -->`.
    Why: releases aren't code-signed yet (§11).
  - **Installing on Linux**: move the README's commands, `PATH` note and
    uninstall commands here. Link Platform Notes for `ptrace_scope`.
  - **Updating**: download the new release and replace the app; settings and
    logs are kept, because they live in the folders on Logs and Files.
- [ ] **Step 2: `README.md`**: keep the intro, "What it will do" (rename to
  "What it does"), and Requirements. Replace Download, Opening the app,
  Installing on Linux and Logs with:
  - **Download**: link to `/releases/latest`, and "Installing covers each OS
    and opening the app the first time" linking the wiki page.
  - **Quick start**: 1. Download and open the app (wiki: Installing).
    2. **Open…** your `.wasm` auto splitter and name its game. 3. Copy an
    address from the Timer card. 4. In LiveSplit One, Connect to Server and
    paste it. 5. When Chrome asks, allow local network access.
  - **Documentation**: link the wiki Home and Troubleshooting.
  - Update the status note: releases exist as pre-releases (the text says
    "There is no release yet").
  Contributing and License stay.
- [ ] **Step 3: Grep every bold term** in the two files in `src/ui/`
  (`grep -rn 'Open…' src/ui` etc.); fix any that don't match.
- [ ] **Step 4: Commit** `docs: move installing to the wiki and add a quick start to the README`

### Task 6: Getting Started and Connecting LiveSplit One

**Files:**
- Create: `docs/wiki/Getting-Started.md`, `docs/wiki/Connecting-LiveSplit-One.md`

Sources: spec §6.2 (Timer card), §6.5, §6.8 (game dialog), §6.9, §5.3, §10,
§11 (browser); `src/ui/connection_tab.rs`,
`src/ui/components/how_to_connect.rs`, `timer_card.rs`, `address_row.rs`,
`timer_row.rs`, `server_section.rs`, `game_dialog.rs`.

- [ ] **Step 1: `Getting-Started.md`**: what you need (the game PC, a timer
  PC with a Chrome-based browser, a `.wasm` auto splitter for the game, both
  on the same network or VPN). Numbered walkthrough: 1. open the app; first
  launch shows Connection with the setup steps (S2 comment). 2. **Open…** and
  pick the `.wasm`. 3. name the game in "Which game is this auto splitter
  for?" and **Use this game** (S3 comment). 4. connect LiveSplit One:
  **Copy** an address, Connect to Server, allow local network access (link
  Connecting LiveSplit One; T1 comment). 5. start the game: Game card
  **ATTACHED**, Timer card **CONNECTED**, and actions appear in **Last
  action** (S1 comment). Ends with "Next" links.
- [ ] **Step 2: `Connecting-LiveSplit-One.md`**, sections:
  - **Which address to use**: one per network address of the game PC, LAN
    and VPN labels; the timer PC must reach it; the port is part of it.
  - **Connecting**: the three How to connect steps, verbatim from the app;
    Chrome-based browsers only (Chrome, Edge, Brave…); T1 and T2 comments.
    Menu path: as written in the app's steps (flag in the PR if LiveSplit
    One's real menu differs).
  - **Several timers**: every timer gets the commands; **PRIMARY** marks the
    one the tracked state follows, and the next one takes over when it
    disconnects (§5.3). S5 comment.
  - **Changing the port**: **Port**, "Restart the server to apply",
    **Restart server**; also a first thing to try when connecting fails.
  - **Who can connect**: §10 in plain words.
- [ ] **Step 3: Grep every bold and quoted UI string** in `src/ui/`; fix
  mismatches.
- [ ] **Step 4: Commit** `docs(wiki): add getting started and connecting LiveSplit One`

### Task 7: Auto Splitters and Settings, and The Main Window

**Files:**
- Create: `docs/wiki/Auto-Splitters-and-Settings.md`, `docs/wiki/The-Main-Window.md`

Sources: spec §6.1–6.4, §6.7, §6.8, §7.2, §7.3; `src/ui/settings_tab.rs`,
`src/ui/preferences_tab.rs`, `src/ui/status.rs`, `src/ui/last_action.rs`,
components `splitter_card.rs`, `splitter_strip.rs`, `game_card.rs`,
`timer_card.rs`, `last_action_card.rs`, `error_card.rs`,
`settings_toolbar.rs`, `unsaved_dialog.rs`, `settings_map.rs`.

- [ ] **Step 1: `Auto-Splitters-and-Settings.md`**, sections:
  - **Getting an auto splitter**: a `.wasm` built for the Auto Splitting
    Runtime (not a legacy `.asl`); from the auto splitter's author or
    LiveSplit's auto splitter list; the app doesn't download them yet.
  - **Opening and reloading**: **Open…**, **Reload** (for a rebuilt file).
  - **Which game it's for**: asked once per new file; **Change**; why games
    matter (settings are saved per game). S3 comment.
  - **Changing settings**: edits are drafts until **Save**; "● Unsaved
    changes" and the tab `•`; **Revert to defaults** is also a draft; the
    "Save your settings changes?" dialog and its buttons. S4 comment.
  - **Settings shared by a game**: §7.2 in plain words (auto splitters for
    the same game share values for settings with the same name; other
    settings are left alone).
  - **Developer mode**: the Preferences note, verbatim; what it shows.
- [ ] **Step 2: `The-Main-Window.md`**, sections: the wide layout (status
  column, tabs); one subsection per card (Error, Auto splitter, Game, Timer,
  Last action) with a table of states: status word, what it means, what to
  do. Include "Not sent: no timer connected" (S9 comment). **Compact
  window**: below about 640 px, **Show details**, **← Status** (S6 comment).
  **Preferences**: **Remember window size and position** and the tiling note,
  About with its folders (link Logs and Files).
- [ ] **Step 3: Grep every bold and quoted UI string** in `src/ui/`; fix
  mismatches.
- [ ] **Step 4: Commit** `docs(wiki): add auto splitters and settings, and the main window`

### Task 8: Logs and Files, Platform Notes, Troubleshooting

**Files:**
- Create: `docs/wiki/Logs-and-Files.md`, `docs/wiki/Platform-Notes.md`,
  `docs/wiki/Troubleshooting.md`

Sources: spec §6.6, §7.1, §8, §9, §11; `src/ui/components/log_toolbar.rs`,
`log_line.rs`, `error_card.rs`, `src/logging/disk.rs`, `src/config/mod.rs`,
the README's Logs section (removed in Task 5).

- [ ] **Step 1: `Logs-and-Files.md`**: the Log tab (categories table from
  §8.1, errors always shown, filters change the view not the record,
  **Copy**, **Save log…**, **Clear**, **Open log folder**, **Show in log**;
  S7 comment); **Log files** (the folder table from Global Constraints,
  `livesplit-asr-bridge-YYYY-MM-DD.log`, 7 days, 50 MB then errors only);
  **Settings files** (the config folder table, what `app.toml`,
  `splitters.toml` and `games/` hold, deleting the folder resets the app and
  shows first launch again).
- [ ] **Step 2: `Platform-Notes.md`**: Linux: `ptrace_scope` — check with
  `cat /proc/sys/kernel/yama/ptrace_scope`; option A
  `sudo setcap cap_sys_ptrace=eip ~/.local/bin/livesplit-asr-bridge`
  (repeat after each update); option B
  `sudo sysctl kernel.yama.ptrace_scope=0` (until reboot; weakens a security
  protection for every program; how to make it permanent via
  `/etc/sysctl.d/`); games under Proton/Wine are read like any process.
  macOS: memory access needs extra permissions and may need elevated rights;
  least tested. Windows: no setup. Browser: §11.
- [ ] **Step 3: `Troubleshooting.md`**: one `##` per symptom, each "What you
  see / Why / What to do", linking the page that explains more:
  - LiveSplit One won't connect (wrong address or network, port blocked by a
    firewall, local network access blocked — T2b comment, not a Chrome-based
    browser, try **Restart server**).
  - Game stays **WAITING FOR GAME…** (game not running, wrong auto splitter,
    Linux `ptrace_scope`, macOS permissions).
  - **Last action** says "Not sent: no timer connected" (S9 comment).
  - "Port 16834 is already in use" (S8 comment).
  - "The auto splitter stopped because of an error." (**Show in log**,
    **Reload**, report it to the auto splitter's author).
  - "Couldn't load … not a valid WebAssembly module" (legacy `.asl` or a
    broken download).
  - The timer doesn't split although actions show (no run in LiveSplit One;
    commands it rejects are in the Connection log).
  - macOS won't open the app / Windows protected your PC (link Installing).
  - **Reporting a bug**: version from Preferences → About, tick all Log
    categories and reproduce, attach the day's log file, open an issue with
    the Bug report form.
- [ ] **Step 4: Grep every bold and quoted UI string** in `src/ui/` and
  `src/logging/`; fix mismatches. Check the ptrace commands' syntax.
- [ ] **Step 5: Commit** `docs(wiki): add logs and files, platform notes and troubleshooting`

### Task 9: Whole-wiki pass

- [ ] **Step 1: Run `scripts/wiki/check.sh docs/wiki`**: no problems.
- [ ] **Step 2: Each fact once**: grep `docs/wiki` and `README.md` for the
  log paths, config paths, `ptrace_scope`, `Open Anyway` and `16834`; each
  appears in full on its home page only, elsewhere as a link (the port may
  appear in examples).
- [ ] **Step 3: Screenshot list**: `grep -rn 'screenshot:' docs/wiki` shows
  every ID from the spec §15.4 list (S1–S9, T1–T4, T2b) at least once.
- [ ] **Step 4: Name and voice**: no "LiveSplit ASR Bridge" or bare
  "LiveSplit" meaning LiveSplit One; `livesplit-asr-bridge` only in paths and
  commands.
- [ ] **Step 5: Fix, commit** `docs(wiki): tidy links and wording`

### Task 10: Checks, pull request and first publish

- [ ] **Step 1: Run the CONTRIBUTING checks** (`cargo fmt --check`,
  `cargo clippy --all-targets --locked -- -D warnings`, `cargo build
  --locked`, `cargo test --locked`), `actionlint`, `shellcheck
  scripts/wiki/*.sh`, `scripts/wiki/test-wiki-scripts.sh`.
- [ ] **Step 2: Push**, mark #73 ready, and fill in its description: pages,
  scripts, the README change, the rules, the screenshot comments still to
  replace, and the manual setup (create the first wiki page; restrict wiki
  editing to collaborators).
- [ ] **Step 3: After merge** (maintainer): confirm the Wiki workflow
  published, open every page on the wiki, and check that page links, the
  sidebar and `images/two-pcs.svg` render. If relative images don't render,
  switch image paths to
  `https://github.com/alexcosta97/livesplit-asr-bridge/wiki/images/<file>`
  in a follow-up and update `check.sh` to accept that prefix.
