# LiveSplit One ASR Bridge brand

The shared visual language for the app, its website, store listings, social
images and documentation. The app's full UI rules are in
[`.superdesign/design-system.md`](../../.superdesign/design-system.md), and the
UI section of the [design spec](../../docs/superpowers/specs/2026-09-27-livesplit-asr-bridge-design.md)
describes the screens.

## Name

- Display name: **LiveSplit One ASR Bridge**. Always name LiveSplit One:
  "LiveSplit" alone usually means the original Windows LiveSplit, which the app
  does not control.
- Technical name: `livesplit-asr-bridge`, for the executable, folders, package
  names and the repository. Don't use it as a display name.
- Tagline: "Run auto splitters here, control LiveSplit One anywhere."

## Logo

The mark shows two machines joined by one link: the game PC (white node) and
the timer (orange node), with a split riding the connection.

| File | Use |
|---|---|
| `logo/icon.svg` | App icon, 48 px and up |
| `logo/icon-small.svg` | App icon at 32 px and below (heavier drawing so the link and split survive) |
| `logo/icon-macos.svg`, `logo/icon-macos-small.svg` | macOS icon: the same icon as an 824 px body on a 1024 px canvas |
| `logo/mark-on-dark.svg`, `logo/mark-on-light.svg` | The mark without the tile, for headers and inline use |
| `logo/wordmark-on-dark.svg`, `logo/wordmark-on-light.svg` | Icon with "LIVESPLIT ONE / ASR BRIDGE", text as outlines |
| `icons/icon-{16…1024}.png` | App icon rasters; 32 px and below use the small drawing (Linux `.desktop`, website) |
| `icons/icon.ico` | Windows (16–256 px) |
| `icons/icon.icns` | macOS |

Rules:

- Keep clear space around the icon of at least a quarter of its width.
- Minimum size: 16 px for the icon (use `icon-small.svg` at 32 px and below),
  24 px tall for the mark alone, 160 px wide for the wordmark.
- Use the files as they are. Don't recolour, rotate, stretch or outline the
  mark, add effects, or swap which node is orange: orange is always the timer.
- On a photo or a busy background, use the icon (with its tile), not the bare
  mark.

## Colour

Exact values are in `tokens.json`.

| Role | Colour | Use |
|---|---|---|
| Ink | `#0B0B0C` | Backgrounds, text on light surfaces |
| Panel / raised | `#151517` / `#1E1E21` | Cards, panels, hover |
| Borders | `#2A2A2E` / `#3A3A40` | 1 px lines |
| Paper | `#F5F5F4` | Text on dark surfaces, light backgrounds |
| Secondary / muted text | `#A1A1AA` / `#6B6B73` | Details, labels |
| **Orange** | `#FF4D00` (hover `#FF6A26`) | The one accent: primary actions, the active tab, the last timer action, links, and the timer node in the logo |
| OK | `#22E07A` | Attached, connected. Status only |
| Error | `#FF3B3B` | Errors. Status only |
| Warning | `#FFC233` | Unsaved changes. Status only |

- Orange is the only accent. Don't introduce a second brand colour.
- Status colours mean status. Never use them for decoration.
- No gradients, glows or glassmorphism. The look is flat, high-contrast and
  sharp: 4 px corners, 1 px borders.

## Type

All three families are under the SIL Open Font License; the files and licences
are in `fonts/`, and the app embeds them.

| Family | Use |
|---|---|
| **Archivo Black** | Status words, headings, the wordmark. Uppercase, tracking −0.02em |
| **Space Mono** | Labels (uppercase, wide tracking), URLs, file names, times, log lines |
| **Inter** (Regular, SemiBold) | Body text, buttons, tabs, settings |

## Voice

Plain and direct, like a teammate at the next desk: short sentences, the
words the user sees in the app, and no marketing superlatives. Say what
happened and what to do next ("Port 16834 is already in use. Choose another
port in Connection and restart the server.").

## Rebuilding

`build.py` generates everything in `logo/`, `icons/` and `tokens.json` from
the drawings and colours it contains. It needs Python 3 with `fonttools` and
`Pillow`, and `rsvg-convert` (librsvg):

```sh
python3 assets/brand/build.py
```

Change the drawings or colours in `build.py`, not the generated files. Keep
the colours in step with `.superdesign/design-system.md`.
