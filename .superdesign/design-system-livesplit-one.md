# LiveSplit One ASR Bridge — ALTERNATIVE look: "like LiveSplit One" (back-pocket)

> **Back-pocket alternative, not the approved design.** The approved style is
> `design-system.md` (speedrun / stream deck). This file is kept in case the community asks
> for the app to look more like LiveSplit One. Don't use it for new designs unless the owner
> switches to it.
>
> **Light-mode mix-up (fixed 2026-09-27):** the first version of this file used LiveSplit
> One's LIGHT scheme, because the site extraction (`website/one.livesplit.org/`) crawled
> one.livesplit.org in light mode. LiveSplit One is DARK by default: its stylesheet defines
> the dark values on `:root` and only applies the light ones under
> `@media (prefers-color-scheme: light)` or `:root[data-theme="light"]`. The values below are
> the dark defaults, read directly from LiveSplit One's `:root` block, unless marked
> "not in the CSS".

This file keeps EVERYTHING from `design-system.md` (product context, approach-A layout,
compact state, every flow, state and rule under "Flows and states") and replaces ONLY the
visual style, so the app looks like a sibling of LiveSplit One.

## Visual style — dark LiveSplit One chrome + data wells

Values are LiveSplit One `:root` custom properties (dark default). Where LiveSplit One uses a
translucent colour, the opaque equivalent for egui is given too.

### Colour

- **Window / content area** `#171717` (`--main-background-color`).
- **Status column (the "rail")** `#1A1A1A` (`--sidebar-background-color`), like LiveSplit
  One's 250 px sidebar.
- **Panels / cards** (auto splitter card, settings panel, dialogs' inner lists) `#121212`
  (`--light-row-color`), 1 px `#404040` border (`--border-color`), shadow `0 1px 3px #00000033`
  (LiveSplit One's table / settings-panel shadow).
- **Wells** (live data) `#0B0B0B` (`--dark-row-color`); well header / separator rows `#090909`
  (`--header-row-color`). Row separators inside wells 1 px `#404040`.
- **Hover row** `#404040` (`--hover-row-color`).
- **Borders / dividers** 1 px `#404040` (`--border-color`, also used for `hr`).
- **Text** primary `#EEEEEE` (`--main-text-color`, `--button-text-color`); strong / on-selected
  `#FFFFFF` (`--selected-row-text-color`, `--overlay-input-text-color`); secondary / labels
  `#BFBFBF` (`--field-label-color`); muted / disabled `#666666` (`--button-disabled-text-color`).
  LiveSplit One also draws body text with `text-shadow: 2px 2px 2px #00000080`
  (`--main-text-shadow`); egui can't blur text, so omit it (or at most a 1 px `#00000080`
  offset copy on status words).
- **Links / "Change" / "How do I connect?"** `#56B0FF` (`--link-color`).
- **Focus / highlight** `#FFD500` (`--field-label-focus-color`, `--field-highlight-color`):
  LiveSplit One marks the focused field with this yellow. Used for focus rings (2 px).
- **Best-segment gold** `#FFD500` (`--best-segment-selected-color`, and `--best-segment-color`
  resolves to the same value). The one warm accent: used only for the LAST ACTION word and its
  3 px card edge, echoing a best split.
- **Selection** (selected list row, e.g. the chosen game in the "Which game" dialog, a log
  entry reached through Show in log): vertical gradient `#3373F4 → #153574`
  (`--selected-row-color`), hover `#6E9CF7 → #204FAC` (`--selected-row-hover-color`), text
  `#FFFFFF`.
- **Status colours** = LiveSplit One's toast set (`--toastify-color-*`), only for status:
  OK / attached / connected / "✓ Saved" `#07BC0C` (success); error `#E74D3C` (error);
  warning / unsaved `•` / "Restart the server to apply" `#F1C40F` (warning); info `#3498DB`
  (info, used only for info notes, not links). Waiting / not connected = neutral `#BFBFBF`
  with an outlined dot. A status card shows its colour as a 3 px left edge plus the coloured
  status word inside the well. No tinted fills, no other decoration.
- **Scrollbars** thumb `#303030` (`--scrollbar-thumb-color`), 10 px, fully rounded; track
  transparent.
- **Log category tags**: auto splitter `#FFD500`, connection `#56B0FF`, app & runtime
  `#BFBFBF`, ERROR `#E74D3C` (not in the CSS; chosen from the palette above).

### Buttons and tabs (LiveSplit One's button treatment)

- **Button**: 5 px radius, 1 px `#404040` border, vertical 2-colour gradient
  `#1C1C1C → #121212` (`--button-color`: faintly top-lit), text `#EEEEEE` Fira Sans 700.
  LiveSplit One's buttons are 40 px tall at 20 px text (`--button-height`); in this app 32 px
  tall at 14 px (40 px in the compact state). No drop shadow: LiveSplit One's buttons have
  none (the first version's `2px 2px 4px` shadow was not in the CSS).
- **Hover**: `#242424 → #171717` (`--button-hover-color`).
- **Pressed / active / selected** (pressed button, active tab, toggled split button):
  `#212121 → #3B3B3B` (`--button-active-color`, the gradient runs darker-to-lighter, so it
  reads as pushed in).
- **Primary actions** (Open…, Save, Use this game, Restart server when needed): same button,
  LiveSplit One has no coloured primary button. Primary is distinguished only by position
  (first) and label.
- **Flat control fill** (LiveSplit One's "middle" button, e.g. a value between two buttons): `#171717`
  (`--button-middle-color`), 1 px `#404040` border, 5 px radius.
- **Disabled**: flat `#222222` (`--button-disabled-color`), text `#666666`, no gradient.
- **Tabs**: LiveSplit One's split-button row: segments joined (inner corners square, outer
  corners 5 px, 1 px `#404040` seams), each segment the normal button gradient; the active
  one uses the pressed gradient `#212121 → #3B3B3B` and `#FFFFFF` text, inactive tabs
  `#BFBFBF` text. Fira Sans 600, 12 px, uppercase, +0.06em. The unsaved dot `•` is `#F1C40F`.

### Inputs

- **Text / number fields** (port, file path, game name): LiveSplit One style: no box,
  transparent background, 1 px bottom line `#FFFFFF40` (`--input-border-color`; opaque on
  `#121212` ≈ `#505050`), text `#EEEEEE`; focused: bottom line and label turn `#FFD500`.
  Where a boxed field reads better (dialogs), 1 px `#FFFFFF40` border, 5 px radius
  (`--overlay-input-border-color`, overlay input style).
- **Dropdowns**: `#0B0B0B` fill (`--dark-row-color`), 1 px `#404040` border, chevron `#EEEEEE`
  (`--select-arrow-color`); LiveSplit One's selects have 0 radius, keep 0 here.
- **Checkboxes**: not in the CSS (LiveSplit One uses native ones). Chosen: 16 px, 2 px
  radius, `#0B0B0B` fill with 1 px `#FFFFFF40` (≈ `#505050`) border; checked = `#EEEEEE`
  fill with a `#121212` check. LiveSplit One's toggle switches, if ever needed: track
  `#FFFFFF20`, hover `#FFFFFF40`, on `#FFFFFF60`, on+hover `#FFFFFF80`, thumb `#FFFFFF`.
- **Tooltip** "i" circle: 14 px, 1 px `#BFBFBF` outline, drawn as a shape; tooltip box
  `#1C1C1C`, 1 px `#FFFFFF40` border, 5 px radius, text `#EEEEEE` 12 px.

### Dialogs and overlays

- Panel `#1C1C1C` (`--overlay-background-color` `#1C1C1CCC` made opaque, because egui has no
  backdrop blur), 1 px `#FFFFFF40` border (`--overlay-border-color`, ≈ `#505050`), 10 px
  radius (LiveSplit One's dialog), shadow `0 5px 10px #1C1C1CCC` (`--overlay-shadow-color`),
  24 px padding, title Fira Sans 700 16 px uppercase `#EEEEEE`, muted text `#FFFFFFB3`
  (`--overlay-muted-text-color`, ≈ `#BBBBBB`).
- The window behind is dimmed with flat 60 % `#000000` (not in the CSS; LiveSplit One's
  dialogs don't dim).

### Typography

- **Fira Sans** for everything but data (LiveSplit One's `fira` face). Status words 20 px/700
  UPPERCASE in the wells (28–32 px in the compact state); card and section labels 11 px/600
  uppercase `#BFBFBF`, tracking +0.06em; body 14–16 px/400; buttons 14 px/700.
- **Fira Mono** 12–13 px for data rows (URLs, file names, times, log lines, version). Not on
  one.livesplit.org (it uses Menlo / Consolas for code); chosen as the monospace sibling of
  Fira Sans.
- No other fonts.

### Shape, spacing, rendering

- Spacing: 8 px base (`--ui-margin`), 16 px large (`--ui-large-margin`), 24 px content
  margin (`--main-content-margin`); 12 px between cards, 16 px card padding.
- Radius 5 px on buttons, cards, wells and fields; 10 px dialogs; 0 on dropdowns.
- Must stay buildable in egui: gradients as 2-colour vertical meshes, shadows via egui
  `Shadow`, no blur, no images, translucent colours flattened to the opaque values above.
