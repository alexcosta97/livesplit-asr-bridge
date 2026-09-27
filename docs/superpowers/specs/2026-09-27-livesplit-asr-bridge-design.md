# livesplit-asr-bridge: design

Date: 2026-09-27
Status: draft for review

## 1. Summary

livesplit-asr-bridge is a cross-platform desktop app that runs a LiveSplit
auto splitter (`.wasm`, for the Auto Splitting Runtime) on the machine the game
runs on, and drives **LiveSplit One** on another machine over a WebSocket.
LiveSplit One connects to the app with its built-in **Connect to Server**
option. The app sends it timer commands (start, split, reset, ...) as the auto
splitter makes decisions.

Typical use: a two-PC streaming setup where the game runs on a gaming PC (for
example Linux with Proton) and the timer shown on stream runs in LiveSplit One
in a Chrome-based browser on a streaming PC (for example a MacBook).

The app is game-agnostic. It works with any auto splitter built for the Auto
Splitting Runtime. It will be published on GitHub for the speedrunning
community.

## 2. Goals and non-goals

### Goals (first version)

- Load a local `.wasm` auto splitter and run it against the game.
- Show and edit the auto splitter's settings, saved per game.
- Run a WebSocket server that LiveSplit One connects to, and show the URL to
  connect to.
- Forward every timer action an auto splitter can take to the connected
  timer.
- Track the connected timer's state, so auto splitters that ask about it get
  correct answers.
- Categorised logs in the app and on disk, with 7 days of retention.
- Builds for Windows, macOS and Linux, released through GitHub.

### Non-goals (first version)

- Browsing or downloading from LiveSplit's official auto splitter list
  (backlog).
- Encrypted connections (`wss://`) or Safari support (backlog).
- Controlling the original Windows LiveSplit (backlog).
- Running legacy ASL scripts (bottom of the backlog).
- Being a timer itself. The app has no splits, comparisons or layout; the
  connected LiveSplit One is the timer.

### Success criteria

- A user loads an auto splitter, connects LiveSplit One on another machine
  using the URL shown in the app, starts the game, and LiveSplit One starts,
  splits and resets exactly as it would with the auto splitter running
  locally.
- A user who has never used the app can find how to connect from the main
  window without reading external documentation.
- When something goes wrong, the user is told what happened and where to find
  more detail.

## 3. Naming and positioning

- Name: **livesplit-asr-bridge** (repository, executable, config and log folder
  names).
- Display name: **LiveSplit One ASR Bridge**. The window title is the display
  name and the version, for example "LiveSplit One ASR Bridge 0.1.0". It is
  kept short so window lists and taskbars show the version.
- Each platform shows the app icon from `assets/brand/icons/` and the display
  name; the executable, bundle and app IDs keep the technical name:
  - **Linux:** the `livesplit-asr-bridge.desktop` entry names the app and its
    icon from the `hicolor` theme; the app ID matches its `StartupWMClass`.
    The window icon is also set at runtime, for X11.
  - **macOS:** the `.app` bundle's `Info.plist` sets `CFBundleDisplayName` to
    the display name, `CFBundleName` (the menu bar, 15 characters at most) to
    the short form **ASR Bridge**, and `CFBundleIconFile` to `icon.icns` in
    `Contents/Resources`. No icon is set at runtime, so the Dock keeps the
    macOS-shaped icon.
  - **Windows:** `build.rs` embeds `icon.ico` and version information
    (`FileDescription` and `ProductName` set to the display name, and the
    version) in the `.exe`, for Explorer and the file properties. The window
    icon is also set at runtime, for the title bar, taskbar and Alt+Tab.
- Window title and descriptions always name LiveSplit One. Descriptions use
  the tagline "run auto splitters here, control LiveSplit One anywhere". The
  first line of the README and the GitHub repository description say it is for
  LiveSplit One, because "LiveSplit" alone usually means the original Windows
  LiveSplit, which this version does not support.

## 4. Architecture

Four components, each with one responsibility:

```
 ┌─────────────── game machine ──────────┐                 ┌─ other machine ─┐
 │  UI (egui window)                     │                 │  LiveSplit One  │
 │   pick .wasm · settings · URL · logs  │                 │  (Chrome-based  │
 │        │                  ▲           │                 │   browser)      │
 │        ▼                  │           │  ws://host:port │                 │
 │  Runner ──timer calls──► Server ◄─────┼─────────────────┤ Connect to      │
 │  (auto splitter,          (WebSocket, │  commands ──►   │ Server          │
 │   reads the game)         timer state)│  ◄── events     │                 │
 │        ▲                              │                 └─────────────────┘
 │  Config (app, games, splitters)       │
 └───────────────────────────────────────┘
```

- **Runner.** Loads the `.wasm` into the upstream `livesplit-auto-splitting`
  runtime (from `livesplit-core`) and runs its update loop on a dedicated
  thread at the tick rate the auto splitter requests. It gives the auto
  splitter a **remote timer**: an implementation of the runtime's `Timer`
  interface that forwards actions to the Server and answers state queries from
  the Server's tracked copy of the timer state. It supports reloading and
  unloading through the runtime's interrupt mechanism, so a hung auto splitter
  cannot freeze the app.
- **Server.** A WebSocket server that LiveSplit One connects to. It encodes
  commands in LiveSplit One's JSON server protocol, sends them to connected
  timers, receives responses and events, and maintains the tracked timer state.
- **UI.** An egui (`eframe`) window: auto splitter picker, game association,
  settings editor, connection URL and status, and the log view.
- **Config.** Reads and writes the app's files (section 7) in the OS's standard
  configuration folder.

Implementation language: Rust, matching the runtime. The app ships as a single
executable per platform.

The upstream runtime is used (not a fork), so the app behaves like other
LiveSplit One hosts for any auto splitter.

## 5. Timer bridging

### 5.1 Commands

Every `Timer` call the auto splitter makes becomes one command in LiveSplit
One's server protocol (defined in `livesplit-core`'s `networking::server_protocol`),
sent to every connected timer:

| Auto splitter action | Server protocol command |
|---|---|
| start | `start` |
| split | `split` |
| reset | `reset` |
| skip split | `skipSplit` |
| undo split | `undoSplit` |
| pause game time | `pauseGameTime` |
| resume game time | `resumeGameTime` |
| set game time | `setGameTime` (with the time) |
| set variable | `setCustomVariable` (key and value) |

The exact command names, parameters and any required sequencing (for example
whether game time must be initialised before `setGameTime`) are confirmed
against `livesplit-core`'s `server_protocol` and LiveSplit One's handling of it
during implementation planning, and covered by tests.

**No connected timer:** commands are not queued. Each dropped command is logged
(Connection category) and discarded. Replaying stale commands when a timer
reconnects would be worse than missing them. The Last action card still shows
the action, marked as not sent (section 6.2).

### 5.2 Tracked timer state

Auto splitters query the timer for its state (not running, running, paused,
ended), the current split index, and whether a given segment was split or
skipped. The remote timer answers these immediately from a tracked copy kept
by the Server:

1. **On connect**, the Server asks LiveSplit One for the current state
   (`getCurrentState`), so connecting mid-run starts from the correct state.
2. **On every event** LiveSplit One sends (started, split, skipped, undone,
   reset, paused, resumed, ...), the Server updates the copy, including a
   per-segment record of whether each segment was split or skipped.
3. **Every 2 seconds**, the Server asks for the state again and corrects the
   copy. This recovers from missed events and from changes made directly on
   the timer (hotkeys, menu actions).

With no timer connected, the tracked state is "not running".

### 5.3 Multiple timers

Several LiveSplit One instances may connect (for example two browser tabs).
Commands go to all of them. The tracked state follows the first timer to
connect; if it disconnects, the next longest-connected timer takes over and
the Server re-queries its state.

## 6. User interface

The layout, states and visual style below come from the design system in
`.superdesign/design-system.md`, which holds the exact colours, sizes and
spacing. Mockups are in the
[Superdesign project](https://superdesign.dev/teams/7cefed93-e527-4a6c-96ed-1cf24495b924/projects/bca2406a-5118-4388-9e38-b0e37fab0d9f).

The window has two uses. Wide, it is for setup: loading an auto splitter,
editing its settings, connecting LiveSplit One and reading logs. Compact, it is
for glancing mid-run, for example as a narrow tiled window on Hyprland or on a
second monitor, to confirm in under a second that everything works.

### 6.1 Main window

Wide layout: a status column on the left, always visible, and the tab area on
the right.

```
┌──────────────────────────────────┬────────────────────────────────────────────┐
│ LIVESPLIT ONE ASR BRIDGE         │ SETTINGS •  CONNECTION  LOG  PREFERENCES   │
│ 0.1.0                            │ ══════════                                 │
│┌ ERROR ─────────────────────────┐├────────────────────────────────────────────┤
││ Couldn't load foo.wasm: not a  ││ (tab content)                              │
││ valid WebAssembly module. The  ││                                            │
││ previous auto splitter is      ││                                            │
││ still running.                 ││                                            │
││ [Show in log] [Dismiss]        ││                                            │
│└────────────────────────────────┘│                                            │
│┌ AUTO SPLITTER ─────────────────┐│                                            │
││ gta_sa_de_autosplitter.wasm    ││                                            │
││ GTA San Andreas — DE           ││                                            │
││ [Change]                       ││                                            │
││ [Open…] [Reload]               ││                                            │
│└────────────────────────────────┘│                                            │
│┌ GAME ──────────────────────────┐│                                            │
││ ● ATTACHED                     ││                                            │
││ SanAndreas.exe · 20 Hz         ││                                            │
│└────────────────────────────────┘│                                            │
│┌ TIMER ─────────────────────────┐│                                            │
││ ○ NOT CONNECTED                ││                                            │
││ How do I connect?              ││                                            │
││ ws://192.168.1.20:16834 [Copy] ││                                            │
││ ws://100.101.7.3:16834  [Copy] ││                                            │
││ [?]                            ││                                            │
│└────────────────────────────────┘│                                            │
│┌ LAST ACTION ───────────────────┐│                                            │
││ SPLIT                 19:31:01 ││                                            │
││ Los Santos — Gym Moves         ││                                            │
││ START                 19:02:44 ││                                            │
││ RESET                 19:02:40 ││                                            │
│└────────────────────────────────┘│                                            │
└──────────────────────────────────┴────────────────────────────────────────────┘
```

- The status column is about 300 px wide. Its cards are described in
  section 6.2.
- The tab area holds the Settings, Connection, Log and Preferences tabs
  (sections 6.4 to 6.7). The active tab is underlined in orange; a `•` on the
  Settings tab label marks unsaved settings.
- Below about 640 px of window width the window switches to the compact
  state (section 6.3).

### 6.2 Status column

Top to bottom: the wordmark "LIVESPLIT ONE ASR BRIDGE" with the version, the
error card (only while there is an error), then the Auto splitter, Game, Timer
and Last action cards.

If the cards do not fit the window height, the column scrolls, as a last
resort. Cards are never clipped.

**Error card:**

- Does not exist when there is no error: no placeholder, no empty space.
- When there is an error, it appears at the top of the column, under the
  wordmark and above the Auto splitter card, pushing every other card down.
- Shows only the most recent error, in red, until dismissed.
- Kinds and buttons:

  | Kind | Message (example) | Buttons |
  |---|---|---|
  | Auto splitter crashed | "The auto splitter stopped because of an error." | **Show in log**, **Reload**, **Dismiss** |
  | Load failed | "Couldn't load foo.wasm: not a valid WebAssembly module. The previous auto splitter is still running." | **Show in log**, **Dismiss** |
  | Port in use | "Port 16834 is already in use. Choose another port in Connection and restart the server." | **Open Connection**, **Dismiss** |

- Every error card has **Dismiss**. Section 9 covers the error situations.

**Auto splitter card:**

- The loaded file name, and the game it is associated with (section 7.3),
  with **Change**, which opens the "Change game" dialog (section 6.8).
- **Open…** (file picker for `.wasm`) and **Reload** (reloads the same file,
  for picking up a rebuilt auto splitter).
- With nothing loaded: "NO AUTO SPLITTER", the help text "Open a .wasm auto
  splitter to start.", **Open…** as the primary button, and **Reload**
  disabled.

**Game card:** based on the runtime's process attach and detach
notifications.

- **ATTACHED**: the process name and the tick rate, for example
  "SanAndreas.exe · 20 Hz".
- **WAITING FOR GAME…**: the auto splitter runs but is not attached to a game
  process. This is normal, not an error.
- **STOPPED**: "The auto splitter isn't running", after the auto splitter
  crashed, until it is reloaded.
- **—**: "Load an auto splitter first", when nothing is loaded.

**Timer card:**

- **CONNECTED**, with the number of timers below it (for example "2 timers ·
  LiveSplit One"), or **NOT CONNECTED** with "How do I connect?",
  which opens the Connection tab at its setup steps.
- **SERVER STOPPED** while the server is not listening, for example when the
  port is in use. There are no addresses to connect to, so none are shown; the
  error card explains why.
- While no timer is connected, the card lists the connection URL for each
  non-loopback IPv4 address of the machine (for example both a LAN address and
  a VPN address), each with **Copy**, and one `?` that opens the setup steps.
- Once a timer is connected, the URLs are hidden. The card shows a line
  pointing to the Connection tab, where the addresses remain, and the `?`.
- The URLs always use the port the server is actually listening on, never a
  port that was edited but not yet applied.
- When the last timer disconnects, the card goes back to NOT CONNECTED.

**Last action card:**

- The latest timer action in large type, with its time: START, SPLIT (with
  the segment name only when the connected timer provides it; the app never
  makes up a name such as "Split 12"), SKIP SPLIT, UNDO SPLIT, RESET, GAME TIME
  (for example "GAME TIME 1:23:45.600"), PAUSE GAME TIME or RESUME GAME TIME.
- The 2 previous actions below it, as faint lines. This is only the recent
  history of this session; it is not stored.
- An action taken while no timer is connected is still shown, with a faint
  "Not sent: no timer connected" note, since it was dropped (section 5.1). It
  tells the user the auto splitter works and the problem is the connection.
- Custom variables (`setCustomVariable`) are logged but not shown here.
- Whether LiveSplit One's server protocol exposes segment names is confirmed
  against `livesplit-core` during implementation planning.
- Empty state: "No actions yet", with "Actions appear here when the auto
  splitter starts, splits or resets."

### 6.3 Compact state

```
┌────────────────────────────────────┐
│ LIVESPLIT ONE ASR BRIDGE 0.1.0     │
│                    [Show details]  │
│ gta_sa_de_autosplitter.wasm        │
│                          [Reload]  │
│┌ GAME ────────────────────────────┐│
││ ● ATTACHED                       ││
││ SanAndreas.exe · 20 Hz           ││
│└──────────────────────────────────┘│
│┌ TIMER ───────────────────────────┐│
││ ● CONNECTED                      ││
││ 1 timer · LiveSplit One          ││
││ Addresses: Connection tab [?]    ││
│└──────────────────────────────────┘│
│┌ LAST ACTION ─────────────────────┐│
││ SPLIT                  19:31:01  ││
││ Los Santos — Gym Moves           ││
│└──────────────────────────────────┘│
└────────────────────────────────────┘
```

- Below about 640 px of window width, the tab area is hidden and the status
  column fills the window. The status words are bigger.
- A thin splitter strip replaces the Auto splitter card: the file name and
  **Reload**, or "No auto splitter loaded" and **Open…**.
- The error card, Game, Timer and Last action cards keep their order and
  states. While no timer is connected, the Timer card shows the first
  connection URL with **Copy**, and "How do I connect?".
- **Show details** widens the window where the window manager allows it.
  Otherwise, for example with a tiling window manager, it swaps the status
  column for the tab view in the same space, with **← Status** to come back.
- Buttons that open a tab (**Show in log**, **Open Connection**, "How do I
  connect?", `?`) show the tab view the same way.

### 6.4 Settings tab

- A fixed toolbar with **Save** and **Revert to defaults**, and a status text.
  Only the settings list below it scrolls.
- The settings list renders the auto splitter's published widgets in order:
  headings (with heading levels), checkboxes, choices, file selections (the
  path with **Browse…**), and any other widget kind the runtime exposes, with
  their tooltips.
- Edits are drafts. The running auto splitter keeps using the saved settings
  until **Save** is pressed, so a stray click mid-run cannot change behaviour.
- Unsaved changes are visible: "● Unsaved changes" in the toolbar, a `•` on
  the tab label, and **Save** enabled only when there is something to save.
  After saving, "✓ Saved" is shown briefly.
- **Revert to defaults** replaces the draft with the auto splitter's default
  values, as unsaved changes, so it still requires **Save** to take effect.
- Closing the app, opening a different `.wasm`, pressing **Reload** or
  pressing **Change** (the game) with unsaved changes opens the
  unsaved-settings dialog (section 6.8).
- With no auto splitter loaded, the tab explains how to load one, with
  **Open…**.

### 6.5 Connection tab

- **SERVER**: **Port** (default `16834`, the same default as the original
  LiveSplit server) and **Restart server**. Editing the port shows "Restart
  the server to apply". **Restart server** is always enabled: it closes
  connections, rebinds the port and starts accepting connections again. It is
  also useful for troubleshooting.
- If the port is in use, the error is also shown inline in this section.
- **CONNECTED TIMERS**: each connection with its address and the tracked
  state, for example "Running · split 12". A **PRIMARY** tag marks the timer
  the tracked state follows (section 5.3). With none: "No timers connected
  yet."
- **HOW TO CONNECT**, 3 numbered steps:
  1. Copy an address (the connection URLs, each with **Copy**).
  2. In LiveSplit One, open Settings → Connect to Server and paste it.
  3. When Chrome asks, allow local network access.

  It notes that LiveSplit One must run in a Chrome-based browser. The section
  can be collapsed while a timer is connected.

### 6.6 Log tab

```
┌──────────────────────────────────────────────────────────────┐
│ Show: [x] Auto splitter  [ ] Connection  [ ] App & runtime   │
│ Errors are always shown.  [Copy] [Save log…] [Clear] [Open log folder]
├──────────────────────────────────────────────────────────────┤
│ 19:31:01.254  auto splitter  Split: Los Santos Gym Moves     │
│ 19:31:01.260  connection     → split                         │
│ 19:32:10.002  ERROR          Auto splitter crashed: …        │
└──────────────────────────────────────────────────────────────┘
```

- Category filters (section 8.1). Errors are always shown. By default only
  errors are shown; ticking categories is how users debug.
- Filters change what is displayed, not what is recorded, so ticking a
  category later shows its earlier lines.
- **Copy** and **Save log…** export the lines currently shown. **Clear**
  empties the in-app view. **Open log folder** opens the on-disk log folder.
- Filter choices are remembered.
- The entry reached through **Show in log** is highlighted.

### 6.7 Preferences tab

- **Remember window size and position**, on by default, with a note that users
  of tiling window managers may want it off. When off, the app does not set a
  window size or position and leaves it to the window manager.
- **About**: the version, and the config folder and log folder, each with
  **Open**.

### 6.8 Dialogs

Dialogs are drawn inside the window (egui modal), centred, with the rest of
the window dimmed behind them.

- **"Which game is this auto splitter for?"**, when a new `.wasm` is loaded
  (section 7.3): a name field pre-filled from the file name, a list of games
  already set up with how many auto splitters use each, and **Use this game**
  and **Cancel**. **Change** on the Auto splitter card opens the same dialog,
  titled "Change game", with the current game selected.
- **"Save your settings changes?"**, when the app is closed, a different
  `.wasm` is opened, or **Reload** or **Change** is pressed with unsaved
  settings: text
  naming how many settings changed, and **Save and reload** (**Save** for the
  other triggers), **Discard** and **Cancel**.

### 6.9 First launch

With no existing app configuration, the app opens on the Connection tab with
the setup steps visible. If the window starts in the compact state, it opens
on the tab view (as **Show details** does, section 6.3) at the Connection
setup steps, with **← Status** to go back.

### 6.10 Visual style

- Dark surfaces: a near-black window background, slightly lighter cards and
  panels, 1 px borders.
- One accent colour, orange, for primary actions, the active tab, the Last
  action card, focus rings and links.
- Status colours are used only for status, never for decoration: green for
  attached and connected, neutral grey for waiting and not connected (normal
  states), red for errors, amber for unsaved changes.
- Fonts: **Archivo Black** for status words and titles (uppercase), **Space
  Mono** for labels, URLs, file names, times and log lines, **Inter** for body
  text and controls. All three are under the SIL Open Font License and are
  embedded in the binary.
- Everything is drawable with egui: flat fills, 1 px strokes, rounded
  rectangles and text. No gradients, blur or shadows.
- Exact colours, sizes and spacing are in `.superdesign/design-system.md`.
- The logo, app icons (`.ico`, `.icns`, PNGs), the font files with their
  licences, colour tokens and usage rules are in `assets/brand/` (see
  `assets/brand/BRAND.md`). The logo shows the two machines joined by one
  link: a white node for the game PC and an orange node for the timer, with a
  split mark on the link.

### 6.11 UI components

The screens are built from a fixed set of components. Each is implemented once
in the app (as an egui widget) and in the mockups (as a Superdesign component),
and every screen is made only of these, so a change to a component changes
every screen that uses it. The components live in the Superdesign project
(the repository keeps their ids in `.superdesign/components/components.json`),
and their exact values are in `.superdesign/design-system.md`.

| Component | Variants and states | Used in |
|---|---|---|
| **Button** | primary, secondary; sizes 32, 28 and 24 px; disabled | everywhere |
| **Section label** | uppercase mono label | cards, tab sections |
| **Info marker** and **Tooltip** | the `i` marker and the tooltip it opens | settings |
| **Address row** | network label (LAN, VPN), URL, **Copy**; wide or compact | Timer card, How to connect |
| **Checkbox** | checked or not, indented under a level-2 heading, tooltip marker, optional note | settings, log filters, preferences |
| **App header** | wordmark and version; compact with **Show details** or **← Status** | status column |
| **Error card** | crashed, load failed, port in use; wide or compact | status column |
| **Auto splitter card** | loaded or nothing loaded | status column (wide) |
| **Splitter strip** | loaded (**Reload**) or nothing loaded (**Open…**) | status column (compact) |
| **Game card** | attached, waiting for game, stopped, nothing loaded; wide or compact | status column |
| **Timer card** | connected, not connected (with addresses), server stopped; wide or compact | status column |
| **Last action card** | an action with its two previous ones, not sent, empty; wide or compact | status column |
| **Tab strip** | active tab, unsaved `•` | tab area |
| **Settings toolbar** | unsaved, saved, nothing to save | Settings tab |
| **Setting heading** | level 1 or 2 | Settings tab |
| **Setting choice** and **Setting file** | a dropdown; a path with **Browse…** | Settings tab |
| **Empty state** | no auto splitter loaded | Settings tab |
| **Server section** | port, edited, port in use | Connection tab |
| **Timer row** | address, tracked state, **PRIMARY** | Connection tab |
| **How to connect** | expanded or collapsed | Connection tab |
| **Log toolbar** and **Log line** | filter states; line categories, highlighted entry | Log tab |
| **About row** | a folder with **Open** | Preferences tab |
| **Game dialog** and **Unsaved dialog** | new file or change; reload, open or close | dialogs |

## 7. Configuration and settings storage

### 7.1 Files

All files live in one folder, the OS's standard per-user configuration folder
for the app:

```
<config folder>/
  app.toml            port, last loaded .wasm, log filters, window preference
                      and remembered geometry, first-launch flag
  splitters.toml      maps each known .wasm path to a game
  games/
    <game-slug>.toml  display name and saved auto splitter settings for a game
```

- `app.toml` is read at start-up.
- `splitters.toml` is read only when an auto splitter is loaded or changed.
- Only the current game's file is read, so the number of games does not
  affect start-up.
- The game file name is a filename-safe slug of the game name; the display
  name is stored inside the file.

### 7.2 Settings merge rules

A game's settings file holds a single map of setting keys to values, shared by
every auto splitter associated with that game.

- **Loading:** the auto splitter receives the saved values for the keys it
  recognises (keys it publishes as widgets). Other keys are ignored.
- **Saving:** only the loaded auto splitter's keys are written. Keys it does
  not recognise are kept unchanged.
- **Revert to defaults:** only affects the loaded auto splitter's keys.
- Keys with the same name in different auto splitters share one value.

### 7.3 Game association

- When a `.wasm` is loaded, the app looks it up in `splitters.toml`.
- **Known file:** the associated game's settings are loaded without asking.
- **New file:** the app asks "Which game is this auto splitter for?", with the
  field pre-filled from the file name and a list of games already set up
  (section 6.8). The answer is saved to `splitters.toml`.
- The association can be changed with **Change** on the Auto splitter card
  (section 6.2).

## 8. Logging

### 8.1 Categories

| Category | Contents | Display |
|---|---|---|
| Errors | Load failures, auto splitter crashes, server start failures, connection errors | Always shown, also in the error card (section 6.2) |
| Auto splitter | Messages printed by the auto splitter | Optional filter |
| Connection | Timers connecting and disconnecting, commands sent, events and responses received, dropped commands | Optional filter |
| App & runtime | Process attach and detach, tick rate changes, settings saved, server restarts, reloads | Optional filter |

The in-app view keeps the most recent 10,000 lines.

### 8.2 On disk

- Every category is written to disk, regardless of the display filters.
- One file per day, named `livesplit-asr-bridge-YYYY-MM-DD.log`.
- Files older than 7 days are deleted at start-up and at each daily rotation.
- A per-day size cap of 50 MB. Past it, only errors are written, and one line
  records that the cap was reached.

| OS | Log folder |
|---|---|
| Linux | `$XDG_STATE_HOME/livesplit-asr-bridge/logs/` (default `~/.local/state/livesplit-asr-bridge/logs/`) |
| macOS | `~/Library/Logs/livesplit-asr-bridge/` |
| Windows | `%LOCALAPPDATA%\livesplit-asr-bridge\logs\` |

These locations are documented in all user-facing documentation.

## 9. Error handling

| Situation | Behaviour |
|---|---|
| The `.wasm` fails to load | Error card with a short reason, **Show in log** and **Dismiss**, full detail in the log; the previously loaded auto splitter keeps running |
| The auto splitter crashes (traps) | Error card saying it stopped, with **Show in log**, **Reload** and **Dismiss**; the Game card shows STOPPED until Reload; the runtime has already stopped it |
| The auto splitter hangs | Reload, unload and closing the app interrupt it through the runtime |
| The game is not running | Game card "WAITING FOR GAME…"; not an error |
| The port is in use | Error card with **Open Connection** and **Dismiss**, the Timer card shows SERVER STOPPED, and the same error inline in the Connection tab, suggesting another port and **Restart server** |
| A timer disconnects | Logged; with no timers left, tracked state becomes "not running", the Timer card shows NOT CONNECTED and commands are dropped |
| LiveSplit One rejects a command (for example split with no run in progress) | Logged in the Connection category; not an error |

Errors are shown in the error card at the top of the status column
(section 6.2): only the most recent one, until dismissed. Errors that need
more detail than one line carry a **Show in log** button, which opens the Log
tab with the relevant category ticked, scrolled to the entry and with the
entry highlighted.

## 10. Security considerations

- The server listens on all network interfaces so another machine can connect.
  Anyone on the same network can connect and receive the timer commands. The
  commands contain no sensitive data, and connected clients can only affect
  the tracked timer state, not the machine. This is documented.
- The app reads the memory of the game process through the runtime's sandbox.
  It never writes to other processes.

## 11. Platform notes (for documentation)

- **Linux:** reading another process's memory may be blocked by
  `kernel.yama.ptrace_scope`. The documentation explains the options
  (`setcap cap_sys_ptrace=eip` on the executable, or relaxing
  `ptrace_scope`).
- **macOS:** reading a game's memory needs extra permissions and may need the
  app to run with elevated rights. macOS is supported but the least tested
  platform for the game side.
- **Windows:** no special setup expected.
- **Browser:** the connecting LiveSplit One must run in a Chrome-based browser.
  Chrome asks for **local network access** the first time the site connects to
  the app. It must be allowed. Chrome reports insecure WebSocket access from
  HTTPS pages as deprecated, so a future Chrome version may require `wss://`
  (backlog).
- Release builds are not code-signed at first, so macOS and Windows show
  unknown-developer warnings. The documentation explains how to open the app.

## 12. Testing

Automated:

- **Protocol:** each timer action produces the correct JSON command; each
  LiveSplit One event and state response updates the tracked state correctly
  (split index, skipped segments, resets, reconnects).
- **Settings storage:** merge rules in section 7.2; `splitters.toml` lookups
  for known and new files; slug generation.
- **Logging:** daily rotation, 7-day deletion and the size cap, using temporary
  folders only.
- **End to end:** a small test auto splitter built for the test suite, which
  starts, splits and logs on a schedule, runs in the Runner while a fake
  LiveSplit One client connects to the Server. The test checks the commands
  arrive in order and that the client's events update the tracked state.

Manual, before each public release:

- The real LiveSplit One website in Chrome connected to the app, with local
  network access allowed.
- A real game session with a real auto splitter.

## 13. Repository conventions and CI

- **Conventional Commits** for commit messages and pull request titles,
  checked in CI for the PR title and every commit.
- **`main` ruleset:**
  - changes only through pull requests;
  - required status checks: formatting (`cargo fmt --check`), lint
    (`cargo clippy -D warnings`) and tests on Linux, a build on Linux, macOS
    and Windows, and the Conventional Commits checks. Formatting, lint and
    tests don't depend on the OS, so they run once; the per-OS build still
    catches code that doesn't compile on one OS;
  - all review conversations resolved;
  - signed commits required;
  - squash merging only, so each pull request becomes one conventional commit
    on `main`.
- **Merge restriction:** a second ruleset lets only maintainers (the repository
  Admin role; the Maintain role if the repository moves to an organisation)
  update `main`, with the bypass limited to pull request merges. It is
  separate so that maintainers still follow every rule above.
- **Templates:** a pull request template (linked issue, what and why, testing,
  checklist) and issue forms for tasks, feature requests and bug reports.
  Blank issues are disabled. Every pull request links an issue.
- Tests run on pull requests only. They are not repeated on merge, because a
  pull request cannot merge without passing them. On merge to `main`, CI only
  compiles, to warm the Rust cache that pull requests restore; it does not run
  the tests. In both cases a `setup` job per OS compiles the dependencies
  once, and the other jobs share its cache.
- **Dependency updates:** Renovate opens weekly pull requests for Cargo
  dependencies (including the LiveSplit crates, taken from the
  `LiveSplit/livesplit-core` repository by git `rev`), GitHub Actions and the
  tools pinned in `mise.toml`. Updates to what ships in the app are titled
  `fix(deps)` and produce a release; Actions and tools are `ci(deps)` and
  `chore(deps)`. Renovate never merges; these pull requests are exempt from
  the linked-issue rule.
- **Tool versions** (git-cliff, shellcheck, actionlint) are defined only in
  `mise.toml`, used both locally and by the workflows.
- These conventions, the development setup and how releases work are
  documented for contributors in `CONTRIBUTING.md`.

## 14. Versioning and releases

### 14.1 Version calculation

- Semantic versioning, calculated automatically with `git-cliff` from the
  Conventional Commits since the last **full** release (tags matching
  `vX.Y.Z`, ignoring `-rc` tags). The bump is the largest change among those
  commits:
  - any breaking change (`!` or `BREAKING CHANGE`): major, except that below
    `1.0.0` it bumps minor;
  - otherwise any `feat`: minor;
  - otherwise any `fix` or `perf`: patch;
  - only `docs`, `chore`, `ci`, `test`, `refactor`, `style` or `build`: no
    release.
- The first release is `0.1.0`. Moving to `1.0.0` is a deliberate decision.
- The version is embedded at build time. There are no version-bump commits on
  `main`.

Example, starting from the full release `0.3.0`:

| Merge | Commits since `v0.3.0` | Bump | Release candidate |
|---|---|---|---|
| `fix: A` | fix | patch | `0.3.1-rc.1` |
| `fix: B` | fix, fix | patch | `0.3.1-rc.2` |
| `feat: C` | fix, fix, feat | minor | `0.4.0-rc.1` |
| `fix: D` | fix, fix, feat, fix | minor | `0.4.0-rc.2` |
| approve `0.4.0-rc.2` | | | `0.4.0` |

### 14.2 Pipeline on each merge to `main`

1. Calculate the version and the next release candidate number, counted from
   existing `vX.Y.Z-rc.*` tags for that version.
2. Build for Linux (x86-64, `.tar.gz`), macOS (Apple Silicon and Intel `.app`)
   and Windows (x86-64 `.exe` in a `.zip`) with that version embedded.
   The Linux `.tar.gz` also contains a `livesplit-asr-bridge.desktop` entry
   and the app icons in the `hicolor` theme layout, so the app appears in
   desktop launchers once installed (see the README).
3. Tag the commit `vX.Y.Z-rc.N` and publish a public **pre-release** with the
   builds and release notes.
4. A publish job waits for approval in a GitHub environment named `release`,
   with the maintainer as required reviewer. The waiting job is not sent to a
   runner and uses no minutes.
5. On approval: rebuild from the same commit with the final version, tag the
   commit `vX.Y.Z`, and publish a full release marked **Latest**.
6. Release candidate pre-releases are kept, so the history shows how many
   candidates each version needed.
7. A newer merge cancels any release candidate still waiting for approval, so
   only the latest candidate can be promoted.

If the commits since the last full release produce no bump, no release
candidate is created.

Pre-releases are badged as such by GitHub, and only full releases can be
marked Latest. Documentation links to `/releases/latest`, which always points
to the newest full release.

### 14.3 Release notes

- A full release lists every change since the previous full release, grouped
  as breaking changes, features and bug fixes.
- A release candidate lists what is new since the previous candidate of the
  same version, followed by all changes since the previous full release.
- The Releases page is the changelog. There is no `CHANGELOG.md` in the
  repository.

## 15. Documentation

- README: what the app does (naming LiveSplit One in the first line), download
  link to `/releases/latest`, quick start, and links to the wiki.
- CONTRIBUTING: development setup, commit, branch and pull request
  conventions, merge requirements, and how releases work.
- Wiki (its own backlog issue): setup, connecting LiveSplit One, settings and
  game association, log locations per OS, platform notes (section 11), and
  troubleshooting.

## 16. Backlog

Tracked as GitHub issues, created in this order.

Foundations:

1. Project scaffold: Cargo project, egui window skeleton, version embedded at
   build time, `license = "MIT OR Apache-2.0"`.
2. CI checks on pull requests (section 13), added as required status checks in
   the `main` ruleset.
3. Release pipeline (section 14).
4. Dependency updates with Renovate (section 13).

First version:

5. Runner: load a local `.wasm` into the upstream runtime, tick loop, Reload,
   interrupt on unload.
6. Game association (`splitters.toml`) and per-game settings storage with the
   merge rules (section 7).
7. Settings tab: drafts, Save, Revert to defaults, unsaved-changes prompts.
8. WebSocket server: connection URLs, port setting, Restart server.
9. Timer bridging: forward every timer action; drop and log commands when no
   timer is connected.
10. Tracked timer state: on-connect query, events, 2-second re-sync, multiple
   timers.
11. Log tab with categories, on-disk logs with daily rotation, 7-day retention
    and size cap.
12. Preferences tab: remember window size and position.
13. Error surfacing with Show in log.
14. First-launch experience and the "How do I connect?" link.

Later:

15. User documentation wiki.
16. Browse and download auto splitters from LiveSplit's official list.
17. Encrypted `wss://` connections (Safari support, and future Chrome
    requirements).
18. Code signing and notarisation for macOS and Windows.
19. Support the original Windows LiveSplit by connecting to its Server
    component.
20. Detect when the connected timer is already running its own auto splitter,
    and stand down (needs research: the protocol does not expose this).
21. Run legacy ASL scripts (very bottom of the backlog).

## 17. License

Dual licensed under MIT or Apache-2.0, at the user's option, as is common in
the Rust ecosystem (including `livesplit-core`). The repository contains
`LICENSE-MIT` and `LICENSE-APACHE`, and `Cargo.toml` declares
`license = "MIT OR Apache-2.0"`.
