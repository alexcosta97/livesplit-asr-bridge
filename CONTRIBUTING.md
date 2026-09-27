# Contributing to livesplit-asr-bridge

Thanks for your interest in contributing. This guide covers how to set up a
development environment and the conventions every change follows.

## Before you start

- All work is tracked in the
  [issues](https://github.com/alexcosta97/livesplit-asr-bridge/issues), and
  every pull request must link one, except Renovate's dependency updates. If
  there's no issue for what you want to do, open one first with the matching
  template: **Task** for a well-defined piece of work, **Feature request** for
  a new idea, or **Bug report** for something that doesn't work.
- For anything beyond a small fix, comment on the issue before starting, so
  the approach can be agreed first.
- The design is described in
  [the design spec](docs/superpowers/specs/2026-09-27-livesplit-asr-bridge-design.md).
  Changes that alter the design update the spec in the same pull request.

## Development setup

1. Install the stable Rust toolchain with [rustup](https://rustup.rs/).
2. Clone the repository and build:

   ```sh
   cargo build
   ```

3. Before pushing, run the same checks CI runs:

   ```sh
   cargo fmt --check
   cargo clippy --all-targets --locked -- -D warnings
   cargo build --locked
   cargo test --locked
   ```

4. For the release scripts and workflow linting, install the pinned tools with
   [mise](https://mise.jdx.dev/) (`mise install`, see `mise.toml`), then run
   `scripts/release/test-release-scripts.sh` and `actionlint`. The same tests
   run on pull requests that change release files.

## Commits

### Conventional Commits

Every commit message follows
[Conventional Commits](https://www.conventionalcommits.org/):

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

| Type | Use for | Effect on the next version |
|---|---|---|
| `feat` | A new feature for users | Minor |
| `fix` | A bug fix for users | Patch |
| `perf` | A performance improvement | Patch |
| `refactor` | A code change that neither fixes a bug nor adds a feature | None |
| `docs` | Documentation only | None |
| `test` | Adding or changing tests | None |
| `build` | Build system or dependencies | None |
| `ci` | CI configuration and workflows | None |
| `style` | Formatting only, no code change | None |
| `chore` | Anything else that doesn't affect users | None |

- The description is in the imperative mood and lowercase, with no full stop:
  `feat: add port setting`, not `Added port setting.`
- A scope is optional and names the affected area: `fix(server): …`,
  `feat(settings): …`.
- A **breaking change** is marked with `!` after the type or scope
  (`feat!: …`), or a `BREAKING CHANGE:` footer describing it. Until version
  1.0.0, breaking changes bump the minor version.

### Signed commits

All commits must be signed, and `main` rejects unsigned ones. Signing with an
SSH key is the simplest option:

```sh
git config --global gpg.format ssh
git config --global user.signingkey ~/.ssh/id_ed25519.pub
git config --global commit.gpgsign true
```

Then add the same public key to your GitHub account as a **signing key**
(Settings → SSH and GPG keys → New SSH key → Key type: Signing Key). GitHub's
documentation on
[commit signature verification](https://docs.github.com/en/authentication/managing-commit-signature-verification)
covers GPG and other options.

## Branches

Name branches `<type>/<short-description>`, using the commit types above, for
example `feat/port-setting` or `fix/reconnect-state`.

## Pull requests

- **Title:** a Conventional Commit, like a commit message. Pull requests are
  squash-merged, and the title becomes the commit on `main`, so it determines
  the next version.
- **Description:** fill in the pull request template. It asks for the issue
  the pull request resolves (`Closes #123`), what changed and why, how it was
  tested, and a short checklist.
- **Scope:** one logical change per pull request.
- **Requirements to merge:**
  - all CI checks pass: formatting, clippy and tests on Linux, a build on
    Linux, macOS and Windows, and the Conventional Commits check on the title
    and every commit;
  - all review conversations are resolved;
  - all commits are signed.
- **Merging:** only maintainers can merge into `main`, using squash merge.
  The branch is deleted after merging.

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

## Releases

Releases are automated. There is no manual version bump and no
`CHANGELOG.md`: the version and release notes come from the commits.

- The version follows [Semantic Versioning](https://semver.org/) and is
  calculated from the commits since the last full release. The largest change
  wins: a breaking change bumps major (minor before 1.0.0), otherwise a `feat`
  bumps minor, otherwise a `fix` or `perf` bumps patch. Commits of the other
  types alone don't produce a release.
- Every merge to `main` that produces a version publishes a **release
  candidate** as a GitHub pre-release, tagged `vX.Y.Z-rc.N`.
- A maintainer promotes a release candidate by approving the pending release
  job. That publishes the full release `vX.Y.Z` from the same commit, marked
  **Latest**.
- Release notes list every change since the previous full release, grouped by
  type. They are the project's changelog.

## License

By contributing, you agree that your contributions are dual licensed under
the MIT and Apache-2.0 licenses, as described in the [README](README.md#license),
without any additional terms or conditions.
