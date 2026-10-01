# Logs and Files

## The Log tab

The app logs what it does, in categories. The **Log** tab shows the most recent lines.

| Category | What it holds |
|---|---|
| Errors | Files that failed to load, auto splitter crashes, server start failures, connection errors. |
| **Auto splitter** | What the auto splitter prints, and which settings it stored. |
| **Connection** | Timers connecting and disconnecting, commands sent, replies from LiveSplit One, commands that were dropped. |
| **App & runtime** | The game attaching and detaching, tick rate changes, saved settings, server restarts, reloads. |

Errors are always shown. The other three are filters under **Show**, and only errors are ticked by default. Tick a category to debug. Filters change what you see, not what's recorded, so ticking one later shows its earlier lines. The app remembers your choices.

<!-- screenshot: S7 log-tab.png -->
The Log tab with Auto splitter and Connection ticked, the buttons Copy, Save log…, Clear and Open log folder, and a few log lines.

- **Copy** and **Save log…** export the lines currently shown.
- **Clear** empties the view. It doesn't delete the log files.
- **Open log folder** opens the folder the log files are in.
- **Show in log**, on an error, opens this tab at the line that explains it, highlighted.

Lines after the first line of a message, such as the details of a crash, show muted under it. The view keeps the latest 10,000 lines.

When the auto splitter sets the game time on every tick, only the first of a run of game times is logged.

## Log files

Every category is written to disk, whatever the filters show. There's one file a day, named `livesplit-asr-bridge-YYYY-MM-DD.log`. Files older than 7 days are deleted. A day's file stops at 50 MB. After that, only errors are added to it.

| OS | Log folder |
|---|---|
| Linux | `$XDG_STATE_HOME/livesplit-asr-bridge/logs/` (default `~/.local/state/livesplit-asr-bridge/logs/`) |
| macOS | `~/Library/Logs/livesplit-asr-bridge/` |
| Windows | `%LOCALAPPDATA%\livesplit-asr-bridge\logs\` |

**Open log folder** in the **Log** tab opens it, and so does **Open** next to **Log folder** in **Preferences**.

## Settings files

The app keeps its settings in one folder:

| OS | Config folder |
|---|---|
| Linux | `$XDG_CONFIG_HOME/livesplit-asr-bridge/` (default `~/.config/livesplit-asr-bridge/`) |
| macOS | `~/Library/Application Support/livesplit-asr-bridge/` |
| Windows | `%APPDATA%\livesplit-asr-bridge\` |

**Open** next to **Config folder** in **Preferences** opens it. It holds:

- `app.toml`: the port, the last auto splitter you loaded, the log filters, the window preference and size, and Developer mode.
- `splitters.toml`: which game each auto splitter file is for.
- `games/`: one file per game, with its name and the saved settings of its auto splitters.

You can back these files up or copy them to another PC. To start over, close the app and delete the folder. The next launch looks like the first one, and the app asks for your games again.

Updating the app keeps these files. See [Installing](Installing).
