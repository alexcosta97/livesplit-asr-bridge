# LiveSplit One ASR Bridge — ALTERNATIVE look: "like LiveSplit One" (REJECTED)

> **Rejected on 2026-09-27.** The owner prefers the approved style in `design-system.md`.
> This exploration is also inaccurate: the extraction crawled one.livesplit.org in light mode,
> but LiveSplit One is dark by default (its light values only apply under
> `prefers-color-scheme: light`). Do not use this file for new designs.

Exploration only. The approved design is `design-system.md` (speedrun / stream deck). This file
keeps EVERYTHING from `design-system.md` (product context, approach-A layout, compact state,
every flow, state and rule under "Flows and states") and replaces ONLY the visual style, so
the app looks like a sibling of LiveSplit One (style extracted from https://one.livesplit.org,
see `website/one.livesplit.org/design.md`).

## Visual style — brushed-panel chrome + dark data wells

- **Chrome is light**, like LiveSplit One's sidebar: window background `#EEF1F6`, status column
  (the "rail") `#E4E8EF`, panels/tab area `#E5E8ED`, alt surface `#DDE3EC`. Text `#202733`,
  secondary `#8A9097`. Hairline borders `#B8BCC2`; structural lines `#2C3444`.
- **Live data sits in dark wells**, like LiveSplit One's timer: `#181818` rounded 5px panels
  with `#2C3444` row separators and light text (`#F5F7FB`, secondary `#8A9097`). Used for:
  the GAME, TIMER and LAST ACTION status cards in the column, the error card, and the log list.
  Everything else (auto splitter card, tabs, settings, connection form, preferences, dialogs)
  is light chrome.
- **Status colours** = LiveSplit One's semantic set, only for status: OK / attached / connected
  `#07BC0C`, error `#E74D3C`, warning / unsaved `#F1C40F`, info / links `#3498DB`, waiting /
  not connected = `#8A9097`. The one warm accent `#D29400` (LiveSplit One's best-segment gold)
  is used only for the LAST ACTION word, echoing a split. A status card shows its colour as a
  3px left edge + the coloured status word inside the dark well. No other decoration.
- **Buttons** (LiveSplit One exactly): 5px radius, 1px `#B8BCC2` border, drop shadow
  `2px 2px 4px rgba(0,0,0,0.15)`, fill `linear-gradient(#F5F7FB, #DBE2EC)` (top-lit metal),
  text `#202733`, 32–40px tall. Primary actions (Open…, Save, Use this game, Restart server when
  needed) use the same gradient with Fira Sans 600 text — LiveSplit One has no coloured primary
  button. Active / selected / pressed (e.g. the active tab, a selected list row, a toggled
  split button): darker gradient `linear-gradient(#CFD5DD, #B4BCC8)`. Disabled: flat `#E5E8ED`,
  text `#8A9097`, no gradient, no shadow.
- **Tabs**: a split-button row (segments joined with visible seams, outer corners 5px), the
  active one uses the darker pressed gradient. The unsaved dot `•` is `#F1C40F`.
- **Inputs / dropdowns / checkboxes**: white `#F5F8FC` fields, 1px `#B8BCC2` border, 5px radius;
  checked checkbox = `#202733` fill with white check. Focus ring `#3498DB`.
- **Typography**: Fira Sans only. Status words 20px/700 UPPERCASE in the wells (28–32px in the
  compact state); card and section labels 11px/600 uppercase `#8A9097` tracking +0.06em; body
  14–16px/400; data rows (URLs, file names, times, log) in Fira Mono 12–13px. No other fonts.
- **Spacing**: 8px base, 5px micro, 12px grouping, 24px between blocks. Radius 5px everywhere.
- **Dialogs**: light `#E5E8ED` panel, 5px radius, `#B8BCC2` border, soft shadow; the window
  behind is dimmed with flat 40% `#202733`.
- Must stay buildable in egui: gradients as 2-colour vertical meshes, shadows via egui Shadow,
  no blur, no images.
