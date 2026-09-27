# LiveSplit One ASR Bridge — design system

## Product context

A desktop app (Rust + egui, Windows / macOS / Linux) for speedrunners. It runs a LiveSplit
auto splitter (`.wasm`) against a game on the gaming PC and drives LiveSplit One, which runs
in a browser on another PC (for example the streaming PC), over a WebSocket.

Jobs to be done:
- **Setup (roomy window):** load an auto splitter, check it is for the right game, tweak its
  settings, copy the connection URL into LiveSplit One, check the timer connected, read logs.
- **Mid-run glance (compact window, often tiled to a narrow strip on Hyprland / a second
  monitor):** confirm in under a second that everything works: game attached, timer
  connected, splits are firing, nothing is broken.

## Layout (approach A — always-visible status column)

- **Left status column**, fixed ~300 px wide, full height, always visible. Top to bottom:
  1. App wordmark "LIVESPLIT ONE ASR BRIDGE" + version (small, mono).
  2. **Error card** (only when there is an error; see "Flows and states"): red, the message,
     its buttons (e.g. **Show in log**, **Reload**) and **Dismiss**. Absent otherwise.
  3. **Auto splitter card**: file name (e.g. `gta_sa_de_autosplitter.wasm`), associated game
     ("GTA San Andreas — Definitive Edition", with a small "Change" link), buttons **Open…**
     and **Reload**.
  4. **Game** status card: ATTACHED (green) / WAITING FOR GAME… (neutral, this is normal, not
     an error). Detail line: process name and tick rate, e.g. `SanAndreas.exe · 20 Hz`.
  5. **Timer** status card: CONNECTED · 1 TIMER (green) / NOT CONNECTED (neutral) with a
     "How do I connect?" link; the connection URLs appear only while NOT CONNECTED (see
     "Flows and states").
  6. **Last action** card: the most recent timer action, big and readable, e.g.
     "SPLIT" + "Los Santos — Gym Moves" + time `19:31:01`. Other actions: START, RESET,
     GAME TIME 1:23:45.600, PAUSE GAME TIME.
- **Right content area**: a tab strip — SETTINGS, CONNECTION, LOG, PREFERENCES — and the tab
  content below it. A `•` on the SETTINGS tab label when there are unsaved settings.
- **Compact state**: when the window is narrower than ~640 px, the content area is hidden
  and the status column fills the window. The status cards grow to use the width and the
  status words get bigger. Nothing else changes: same cards, same order. A small
  "Show details" button (or widening the window) brings the tabs back.

## Visual style — "speedrun / stream deck", adapted from Kinetic Orange

Energetic, high-contrast, readable from across the room. Adapted for a dark desktop app and
for what egui can render.

### Colour

- Background `#0B0B0C` (window), panels / cards `#151517`, raised / hover `#1E1E21`,
  borders `#2A2A2E` (1 px) and `#3A3A40` for emphasis.
- Text: primary `#F5F5F4`, secondary `#A1A1AA`, muted `#6B6B73`.
- **Brand accent: electric orange `#FF4D00`** — primary buttons, active tab underline,
  the "LAST ACTION" highlight, focus rings, links. Hover `#FF6A26`.
- Status colours (only for status, never decoration):
  - OK / attached / connected: green `#22E07A`
  - Waiting / not connected (normal, not an error): neutral grey `#A1A1AA` with an outlined dot
  - Error: red `#FF3B3B`
  - Unsaved / warning: amber `#FFC233`
- A status card in the OK state gets a 3 px left edge in its status colour and a very faint
  tint of it (6 % alpha) on the card background. No gradients anywhere.

### Typography

- Display / status words: **Archivo Black**, UPPERCASE, tight tracking (-0.02em).
  Status words 20 px in the wide state, 28–32 px in the compact state.
- Labels, metadata, URLs, timestamps, file names, log lines: **Space Mono** 11–13 px;
  card labels UPPERCASE with wide tracking (+0.08em), muted colour.
- Body and controls (settings labels, buttons, tabs, help text): **Inter** 13–14 px,
  weights 400 / 600.
- Tabs: Inter 600, 12 px, UPPERCASE, +0.06em tracking.

### Shape, spacing, borders

- 8 px spacing grid. Cards: 16 px padding, 12 px gap between cards. Content area: 24 px padding.
- Sharp-ish: 4 px corner radius on cards and inputs, 4 px on buttons. No pill shapes except
  small status dots (8 px circles).
- 1 px borders; 2 px orange underline for the active tab.
- No drop shadows, no blur, no glassmorphism, no gradients, no skew, no marquees, no images.
  Everything must be buildable with egui Frames, rounded rects, strokes and text.

### Components

- **Primary button**: orange `#FF4D00` fill, black `#0B0B0C` text, Inter 600 13 px, 32 px tall.
- **Secondary button**: transparent, 1 px `#3A3A40` border, primary text; hover `#1E1E21`.
- **Disabled**: 40 % opacity.
- **Checkbox**: 16 px square, 1 px border, orange fill + black check when on.
- **Dropdown / choice**: `#151517` field, 1 px border, 32 px tall, chevron.
- **Copy button next to a URL**: small secondary button, shows "COPIED ✓" in green for 1.5 s.
- **Log line**: Space Mono 12 px: time (muted) · category tag (fixed width, coloured text:
  auto splitter = orange, connection = `#7DD3FC`, app & runtime = secondary, ERROR = red)
  · message.

### Motion

Minimal: the Last action card flashes its orange edge for ~300 ms when a new action arrives.
The Copy confirmation. Nothing else animates.

## Content to use in mockups (realistic GTA SA DE data)

- Splitter: `gta_sa_de_autosplitter.wasm`, game "GTA San Andreas — Definitive Edition",
  process `SanAndreas.exe`, tick rate 20 Hz.
- Settings (Settings tab, from the auto splitter): heading "Start & reset"
  (Start on new game ✓, Reset on main menu ✗); heading "Splits" (Split on mission passed ✓,
  Split on 100% stat increase ✗, choice "Collectible splits: Off / Every item / Every 10";
  heading "Collectibles": Tags ✓, Snapshots ✗, Horseshoes ✗, Oysters ✗); heading
  "Timing" (Remove loads (game time) ✓).
- Log lines: `19:31:01.254  AUTO SPLITTER  Split: Los Santos — Gym Moves`,
  `19:31:01.260  CONNECTION  → split`, `19:30:12.004  APP  Loaded gta_sa_de_autosplitter.wasm`.

## Flows and states (agreed inventory)

Every mockup must stay consistent with these rules.

### Status column states
- **Auto splitter card, nothing loaded:** "NO AUTO SPLITTER" (muted), help text "Open a .wasm auto
  splitter to start.", primary **Open…**; Reload disabled.
- **Game:** ATTACHED (green) · WAITING FOR GAME… (neutral grey, outlined dot; normal, not an
  error) · STOPPED (neutral, "The auto splitter isn't running") after the auto splitter crashed,
  until Reload · "—" muted with "Load an auto splitter first" when no splitter is loaded.
- **Timer:** CONNECTED · N TIMERS (green) · NOT CONNECTED (neutral) with an orange
  "How do I connect?" link · SERVER STOPPED (neutral, no addresses) while the server isn't
  listening, e.g. port in use (the error card explains why). The connection URL rows are always in the Timer card in the wide
  state, one row per non-loopback IPv4 address (e.g. `ws://192.168.1.20:16834` LAN and
  `ws://100.101.7.3:16834` VPN), each with **Copy**, plus one small "?" that opens the setup
  steps. The URL rows are shown ONLY while NOT CONNECTED (same rule as the compact state); once
  a timer is connected the card shows just the status, the timer count and the "?" (the
  addresses stay on the Connection tab). The URLs always use the port the server is actually
  listening on, never an edited-but-not-applied port. A timer disconnecting is logged; with no
  timers left the card goes back to NOT CONNECTED.
- **Column overflow:** if the cards still don't fit the window height, the column scrolls
  (last resort); cards are never clipped.
- **Last action:** the latest action big, plus the 2 previous ones as faint mono lines (this
  run's recent history only). Actions: START, SPLIT (+ segment name when known), SKIP SPLIT,
  UNDO SPLIT, RESET, GAME TIME 1:23:45.600, PAUSE GAME TIME, RESUME GAME TIME. SPLIT shows
  the segment name only when the timer provides it; otherwise just "SPLIT" (never an
  invented name like "Split 12"). An action taken with no
  timer connected is still shown, with a faint mono note "Not sent: no timer connected". Custom
  variables are NOT shown here (log only). Empty state: "NO ACTIONS YET" muted, "Actions appear
  here when the auto splitter starts, splits or resets."
- **Error card**: does NOT exist when there is no error (no placeholder, no empty space).
  When there is an error it appears at the TOP of the column, directly under the wordmark and
  above the Auto splitter card, pushing every other card down. Only one, the most recent: red. Kinds: auto splitter crashed ("The auto
  splitter stopped because of an error." + **Show in log** + **Reload**), load failed
  ("Couldn't load foo.wasm: not a valid WebAssembly module. The previous auto splitter is still
  running." + **Show in log**), port in use ("Port 16834 is already in use. Choose another port
  in Connection and restart the server." + **Open Connection**). Always has **Dismiss**.

### Compact state (window narrower than ~640 px)
- Only the status column, full width, bigger status words. The URL rows are hidden while a timer
  is connected; while NOT CONNECTED the card shows the first URL with Copy and
  "How do I connect?".
- **First launch in compact:** opens on the tab view at the Connection setup steps, with
  **← Status**.
- **Show details** (top right): widens the window where the window manager allows it;
  otherwise it swaps the status column for the tab view in the same narrow space, with a
  **← Status** button to come back.

### Dialogs
- Drawn inside the window (egui modal), centred over the content area, with the rest of the
  window dimmed by a flat 60 % black overlay (no blur). 4 px radius, #151517, 1px #3A3A40
  border, 24 px padding, title in Archivo Black 16 px uppercase.
- **"Which game is this auto splitter for?"** (new .wasm, or **Change**): text field pre-filled
  from the file name ("gta sa de"), a list of games already set up to pick from (each with how
  many splitters use it), buttons **Use this game** (primary) and **Cancel**. When changing,
  the title is "Change game" and the current game is selected.
- **Unsaved settings** (Reload / Open another file / closing the app with unsaved settings):
  "Save your settings changes?", text naming the change count, buttons **Save and reload**
  (primary; **Save** when opening another file or closing the app), **Discard**, **Cancel**.

### Tabs
- **Settings:** fixed toolbar (Save, Revert to defaults, status text: "● Unsaved changes"
  amber / "✓ Saved" green for a few seconds / nothing). Scrollable list of the splitter's
  widgets in order: headings (level 1 in Space Mono 11 px uppercase, level 2 in Inter 600 13 px
  secondary, indented 16 px), checkboxes, choices (dropdown), **file selection** (mono path
  field + **Browse…**), and a tooltip on hover (small #1E1E21 box, Inter 12 px) shown via an
  "i" in a 14 px outlined circle drawn as a shape (not a font glyph). No splitter loaded: an
  empty state explaining how to load one, with **Open…**.
- **Connection:** sections separated by headings: "SERVER" (Port field 120 px, default 16834,
  **Restart server** secondary button always enabled; after the port is edited an amber note
  "Restart the server to apply"), "CONNECTED TIMERS" (rows: address `192.168.1.42:53122`,
  tracked state e.g. "Running · split 12", "Primary" tag on the one the tracked state follows;
  empty: "No timers connected yet."), "HOW TO CONNECT" (numbered steps: 1 Copy an address
  (with the URL rows + Copy), 2 In LiveSplit One open Settings → Connect to Server and paste it,
  3 When Chrome asks, allow local network access. Note: LiveSplit One must run in a
  Chrome-based browser).
- **Log:** toolbar: "SHOW" checkboxes Auto splitter / Connection / App & runtime, muted text
  "Errors are always shown.", buttons Copy, Save log…, Clear, Open log folder. Lines in Space
  Mono 12 px: time · category tag · message. An entry reached through **Show in log** is
  highlighted with a 1 px orange outline and #1E1E21 background.
- **Preferences:** checkbox "Remember window size and position" (on) with the note "Turn this
  off with a tiling window manager, so the window manager decides the size."
