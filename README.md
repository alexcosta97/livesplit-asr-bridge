# livesplit-asr-bridge

Run a LiveSplit auto splitter on the machine your game runs on, and control
**LiveSplit One** on another machine.

LiveSplit One ASR Bridge loads a `.wasm` auto splitter (made for LiveSplit's Auto
Splitting Runtime) and runs it against your game. LiveSplit One connects to the
bridge with its built-in **Connect to Server** option, and the bridge sends it
start, split, reset and game time commands as the auto splitter decides. It is
made for two-PC setups, for example a gaming PC running the game and a
streaming PC showing the timer.

> **Status: in early development.** Releases so far are pre-releases for
> testing. The design is in
> [the design spec](docs/superpowers/specs/2026-09-27-livesplit-asr-bridge-design.md),
> and planned work is tracked in the
> [issues](https://github.com/alexcosta97/livesplit-asr-bridge/issues).

## What it does

- Load any local `.wasm` auto splitter, for any game.
- Show every kind of setting an auto splitter can have (built with the `asr`
  crate, SplitScript or by hand), saved per game, including values the auto
  splitter stores itself, as LiveSplit does.
- A developer mode that shows the auto splitter's whole settings map and its
  log messages, for writing and debugging auto splitters.
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

Get the latest version from
[the latest release](https://github.com/alexcosta97/livesplit-asr-bridge/releases/latest).
[Installing](https://github.com/alexcosta97/livesplit-asr-bridge/wiki/Installing)
covers each OS and opening the app the first time.

## Quick start

1. Download and open the app
   ([Installing](https://github.com/alexcosta97/livesplit-asr-bridge/wiki/Installing)).
2. Click **Open…**, pick your `.wasm` auto splitter and name its game.
3. Copy an address from the Timer card.
4. In LiveSplit One, open **Settings → Connect to Server** and paste it.
5. When Chrome asks, allow local network access.

## Documentation

The [wiki](https://github.com/alexcosta97/livesplit-asr-bridge/wiki) has the
full guide. If something doesn't work, start with
[Troubleshooting](https://github.com/alexcosta97/livesplit-asr-bridge/wiki/Troubleshooting).

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
