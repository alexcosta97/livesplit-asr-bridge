# livesplit-asr-bridge

Run a LiveSplit auto splitter on the machine your game runs on, and control
**LiveSplit One** on another machine.

livesplit-asr-bridge loads a `.wasm` auto splitter (made for LiveSplit's Auto
Splitting Runtime) and runs it against your game. LiveSplit One connects to the
bridge with its built-in **Connect to Server** option, and the bridge sends it
start, split, reset and game time commands as the auto splitter decides. It is
made for two-PC setups, for example a gaming PC running the game and a
streaming PC showing the timer.

> **Status: in early development.** There is no release yet. The design is in
> [the design spec](docs/superpowers/specs/2026-09-27-livesplit-asr-bridge-design.md),
> and planned work is tracked in the
> [issues](https://github.com/alexcosta97/livesplit-asr-bridge/issues).

## What it will do

- Load any local `.wasm` auto splitter, for any game.
- Show the auto splitter's settings, saved per game.
- Show the address to paste into LiveSplit One's **Connect to Server**.
- Forward every timer action the auto splitter takes, including game time and
  load removal.
- Keep categorised logs, in the app and on disk for 7 days.
- Run on Windows, macOS and Linux.

## Requirements

- LiveSplit One running in a **Chrome-based browser**. The first time it
  connects, Chrome asks to allow **local network access**, which must be
  allowed.
- The original Windows LiveSplit is not supported yet.

## Download

Once released, the latest version will be available from
[the latest release](https://github.com/alexcosta97/livesplit-asr-bridge/releases/latest).
Releases marked **Pre-release** are release candidates for testing.

### Opening the app

Releases are not code-signed yet, so the first launch shows a warning.

- **macOS:** unzip the download and open `livesplit-asr-bridge.app`. macOS
  says it can't verify the developer. Click **Done**, then go to **System
  Settings → Privacy & Security**, scroll down and click **Open Anyway** next
  to the message about livesplit-asr-bridge. After that, the app opens
  normally.
- **Windows:** if SmartScreen says "Windows protected your PC", click **More
  info**, then **Run anyway**.

## Logs

The app keeps a log of what it does, by category: errors, the auto
splitter's messages and actions, the connection to LiveSplit One, and the app
itself. The **Log** tab shows the most recent lines; errors are always shown,
and the other categories when ticked. Every category is also written to disk,
one file per day, kept for 7 days, in:

| OS | Log folder |
|---|---|
| Linux | `$XDG_STATE_HOME/livesplit-asr-bridge/logs/` (default `~/.local/state/livesplit-asr-bridge/logs/`) |
| macOS | `~/Library/Logs/livesplit-asr-bridge/` |
| Windows | `%LOCALAPPDATA%\livesplit-asr-bridge\logs\` |

A day's file stops at 50 MB, after which only errors are added to it.
**Open log folder** in the Log tab opens it.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to set up a development
environment, the commit and pull request conventions, and how releases work.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or
  <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
