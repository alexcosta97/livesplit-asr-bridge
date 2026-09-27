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
  2. **Auto splitter card**: file name (e.g. `gta_sa_de_autosplitter.wasm`), associated game
     ("GTA San Andreas — Definitive Edition", with a small "Change" link), buttons **Open…**
     and **Reload**.
  3. **Game** status card: ATTACHED (green) / WAITING FOR GAME… (neutral, this is normal, not
     an error). Detail line: process name, e.g. `SanAndreas.exe`.
  4. **Timer** status card: CONNECTED · 1 TIMER (green) / NOT CONNECTED (neutral) with a
     "How do I connect?" link. Always shows the connection URL `ws://10.0.177.167:16834`
     in mono with a **Copy** button (one row per non-loopback IPv4 address).
  5. **Last action** card: the most recent timer action, big and readable, e.g.
     "SPLIT" + "Los Santos — Gym Moves" + time `19:31:01`. Other actions: START, RESET,
     GAME TIME 1:23:45.600, PAUSE GAME TIME.
  6. **Error card** (only when there is an error): red, the message, **Show in log** and
     **Dismiss**.
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
