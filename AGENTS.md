# AGENTS.md

livesplit-asr-bridge is a desktop app that runs LiveSplit auto splitters on one
machine and controls LiveSplit One on another. This file only says where to
find things; the information itself lives in the places below.

## Where to find what

| You need | Look in |
|---|---|
| What the project is for | `README.md` |
| The design, and the reasoning behind decisions | `docs/superpowers/specs/` |
| Implementation plans | `docs/superpowers/plans/` |
| The work to do, its scope and acceptance criteria | GitHub issues, `alexcosta97/livesplit-asr-bridge` |
| Work in progress | Open pull requests |
| Conventions: commits, branches, pull requests, checks, releases | `CONTRIBUTING.md` |
| What a pull request must contain | `.github/pull_request_template.md` |
| Release history | GitHub Releases |

The spec is the source of truth for design. If an issue, plan or pull request
contradicts it, raise it rather than choosing silently. A change to the design
updates the spec in the same pull request.

## Starting a session

1. Read `CONTRIBUTING.md`. Its conventions apply to every change.
2. Check open issues and pull requests with `gh`, to see what is in progress
   and what is next.
3. Work from an issue. Read it, the spec sections it references, and any plan
   for it before changing code.

## How work is run

The main session coordinates. It holds the full context, gives subagents
self-contained tasks (each with the issue, the relevant spec sections, the
files involved and how to verify the result), and reviews their output
against the spec before anything is committed or merged. Subagents do not
decide what is correct; the coordinating session does.
