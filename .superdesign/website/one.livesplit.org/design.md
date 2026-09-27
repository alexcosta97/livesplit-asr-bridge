---
version: "superdesign-alpha"
name: "Brushed-panel timer chrome"
description: "Near-white brushed-metal control panel wrapped around a near-black data timer surface, with sunken glossy button gradients and a single amber measurement accent."
colors:
  background: "#E4E8EF"
  surface: "#E5E8ED"
  surface-alt: "#DDE3EC"
  timer-surface: "#181818"
  text-primary: "#202733"
  text-secondary: "#8A9097"
  border: "#B8BCC2"
  border-strong: "#2C3444"
  accent: "#D29400"
typography:
  display-lg:
    fontFamily: "fira"
    fontSize: "24px"
    fontWeight: 700
  headline-md:
    fontFamily: "fira"
    fontSize: "22px"
    fontWeight: 700
  body-md:
    fontFamily: "fira"
    fontSize: "16px"
    fontWeight: 400
  body-char-weighted:
    fontFamily: "fira"
    fontSize: "20px"
    fontWeight: 400
    lineHeight: "1.4"
  accent-serif:
    fontFamily: "Times New Roman"
    fontStyle: "normal"
spacing:
  base: "8px"
  tight: "5px"
  gap: "24px"
  block: "12px"
rounded:
  control: "5px"
  split-segment: "5px"
  pill: "5px"
components:
  button-primary:
    background: "linear-gradient(rgb(245, 247, 251) 0%, rgb(219, 226, 236) 100%)"
    text-color: "#202733"
    radius: "5px"
    height: "40px"
    padding: "5px 8px"
    border: "1px solid rgb(184, 188, 194)"
  button-split-left:
    background: "linear-gradient(rgb(207, 213, 221) 0%, rgb(180, 188, 200) 100%)"
    text-color: "#202733"
    radius: "5px 0px 0px 5px"
    height: "40px"
    padding: "5px 8px"
    border: "1px solid rgb(184, 188, 194)"
  button-split-right:
    background: "linear-gradient(rgb(245, 247, 251) 0%, rgb(219, 226, 236) 100%)"
    text-color: "#202733"
    radius: "0px 5px 5px 0px"
    height: "40px"
    padding: "5px 8px"
    border: "1px solid rgb(184, 188, 194)"
  button-disabled:
    background: "#E5E8ED"
    text-color: "#8A9097"
    radius: "5px"
    height: "40px"
    padding: "5px 8px"
    border: "1px solid rgb(184, 188, 194)"
  nav-panel-card:
    background: "transparent"
    radius: "0px"
    padding: "0px"
---
# Brushed-panel timer chrome
Source: https://one.livesplit.org/

## Overview
This is a utilitarian control-panel aesthetic — a light, brushed-metal side panel of stacked rectangular buttons sitting beside a sunken, near-black data readout. It is closer to Fluent/skeuomorphic-revival control chrome than to any marketing aesthetic: every surface signals a physical control (glossy gradient buttons with hairline borders and a soft drop shadow) rather than a flat digital tile. The timer well is the one deliberately dark region in an otherwise pale, neutral system, and its content — dense rows of tabular data culminating in one oversized numeral — is the sole focal point.

## Composition
The page is two zones side by side, not stacked sections: a fixed-width light control rail on the left and a fixed-width dark data panel on the right, with open, uncomposed page background (the dominant pixel field) filling the remainder of the viewport. The rail runs top to bottom as a stack — brand mark, a primary navigational stack of two buttons, a labeled comparison group of one full-width button plus a two-up split-button row, a divider, then a second stack of three utility buttons. The dark panel is a tall list of thin horizontal data rows collapsing into one dominant large numeral near its base, capped by a footer strip. Below both, a compact 2×2 icon-button cluster sits directly under the panel. The deliberate choice is restraint: fixed small panels anchored top-left on a vast pale field, rejecting a full-bleed app-shell that would stretch the rail and panel to fill the viewport — density is concentrated, not distributed.

## Colors
The pixel field is dominated by a near-white cool gray (~89%, matching `#E4E8EF`/`#E5E8ED`), establishing the page background and the button/panel surfaces as one continuous light system — buttons are distinguished from that field only by gradient sheen and a 1px `#B8BCC2` border, not by hue. The dark timer well (`#181818`, ~6% of pixels) is the only surface with real weight; it reads as the product's "engine room" against the light chrome. Text ink is a near-black navy-gray (`#202733`) for primary labels and numerals, stepping down to a muted `#8A9097` for disabled/secondary labels and inactive tab text. A small warm accent (`--best-segment-color: #D29400`, amber/gold) exists as a semantic token reserved strictly for a best-segment measurement highlight — it is not used decoratively anywhere else, keeping the rest of the panel monochrome. Borders split between the pale `#B8BCC2` hairline on buttons and a darker `#2C3444` structural line, likely separating the dark panel's rows. Toast tokens (`#3498DB` info, `#07BC0C` success, `#F1C40F` warning, `#E74D3C` error) exist as a full semantic notification ramp but are not visible on this screen — reserve them for transient alerts only.

## Typography
Everything on screen runs on one family, fira, with hierarchy built from size and weight rather than family-switching: a 24px/700 display size for the topmost title-row figure, 22px/700 for secondary headline-weight numerals, and 16px/400 for standing body/list text. A char-weighted body reading of 20px/400 in `#202733` indicates the dominant in-row data text actually renders slightly larger than the nominal 16px body token — treat data-row text as ~20px in the rebuild. A secondary serif (Times New Roman) is present in the family list as an unstyled fallback/accent face — use it sparingly, if at all, as it carries no distinct emphasis role here, unlike a true signature italic.

## Layout
The content column is narrow and fixed — a 604px max-width dark panel plus a similarly compact light rail — not a fluid or centered marketing grid; this is a docked utility widget, not a page layout. Spacing is tight and mechanical: an 8px base unit with a 5px micro-increment for internal button padding, 24px for rail block separation, and 12px for intermediate grouping. Corner radius is uniform at 5px across every button and control — no pill shapes, no large card radii — reinforcing a hardware-panel identity over a soft consumer-app one. The one true grid is the 2×2 icon-button cluster beneath the data panel: 2 columns, 8px gap, 4 equal items at 49%/49% width per row (rows: [2][2]) — a compact, uniform icon-toolbar grid, not bento or masonry.

## Components
- **Side navigation rail (card family, transparent, 0px radius/padding)**: fixed-width light panel, top-to-bottom stack — brand mark/wordmark at top, a primary two-button vertical stack, a labeled "compare against" divider group holding one full-width selector button above a two-segment split-button row, a hairline divider, then three more full-width utility buttons stacked vertically. All buttons in this rail are full rail-width, one per row.
- **Button — primary/full gradient** (×6 measured): fill `linear-gradient(rgb(245, 247, 251) 0%, rgb(219, 226, 236) 100%)`, text `#202733`, radius 5px (slightly-rounded), height 40px, padding 5px 8px, border 1px solid `rgb(184, 188, 194)`. Used for the rail's standalone stacked actions and utility buttons — glossy top-lit metal-button read.
- **Button — split-left segment** (×1): fill `linear-gradient(rgb(207, 213, 221) 0%, rgb(180, 188, 200) 100%)`, text `#202733`, radius 5px 0px 0px 5px (left corners only, slightly-rounded), height 40px, padding 5px 8px, border 1px solid `rgb(184, 188, 194)`. Darker gradient signals the active/selected half of a two-way toggle.
- **Button — split-right segment** (×1): fill `linear-gradient(rgb(245, 247, 251) 0%, rgb(219, 226, 236) 100%)`, text `#202733`, radius 0px 5px 5px 0px (right corners only), height 40px, padding 5px 8px, border 1px solid `rgb(184, 188, 194)`. Paired directly against the split-left button forming one pill-length control with an internal seam, not a rounded pill overall.
- **Button — disabled/muted** (×3): flat fill `#E5E8ED`, text `#8A9097`, radius 5px, height 40px, padding 5px 8px, border 1px solid `rgb(184, 188, 194)`. Appears in the 2×2 icon cluster for currently-inactive controls — flat fill (no gradient) is the visual signal of disabled state versus the glossy gradient of active buttons.
- **Timer/data panel**: the dark well itself, `#181818`, holding a title row with a right-aligned counter, a labeled section header row, a tall stack of thin repeating list rows (empty/placeholder state shown as uniform dark bands), and a bottom band presenting one oversized numeral (measured display-lg 24px/700 scale, rendered far larger in practice as the focal metric) with a smaller decimal suffix beside it, capped by a labeled footer row.
- **Icon-button toolbar (2×2 grid)**: 4 square icon buttons in 2 columns × 2 rows, 8px gap, each ~49% width — mixes an active gradient button (top-left, play glyph) with three disabled flat buttons, directly beneath the timer panel.

## Graphics & Effects
Two gradients are measured, both small-surface button fills, never backgrounds: `linear-gradient(rgb(245, 247, 251) 0%, rgb(219, 226, 236) 100%)` covers roughly 0.5% of the page across the primary buttons, and `linear-gradient(rgb(207, 213, 221) 0%, rgb(180, 188, 200) 100%)` covers about 0.2% across the split-left/active segment — both read as a soft top-to-bottom glossy sheen on an otherwise flat panel, not a page wash. Buttons carry a tight drop shadow, `rgba(0, 0, 0, 0.15) 2px 2px 4px 0px`, giving each control a slight raised-hardware lift off the pale field. A `blur(5px)` backdrop filter is present in the system (likely for an overlay/toast layer) though not visible on this idle screen — reserve it for popover or notification surfaces. The timer panel itself is built on live canvas elements; stand in with a static dark rectangle plus rendered text rows when rebuilding, since no image asset drives its content.

## Motion
Motion is entirely reserved for transient notification choreography: `Toastify__bounceInRight` / `Toastify__bounceOutRight` and their left/up counterparts drive toast entrances and exits with a bounce/overshoot character (spring-like, not linear ease), while `Toastify__trackProgress` animates a linear countdown bar beneath each toast. No hover/press easing values or button-transition durations are otherwise exposed — treat all button and panel state changes as instant, mechanical toggles (gradient swap on active/disabled), reserving springy bounce motion strictly for the toast layer.

## Guardrails
- Do not stretch the rail or timer panel to full viewport width — both are fixed, compact, dock-style panels on a vast pale field, never a full-bleed app shell.
- Do not round any button beyond 5px or turn the split-button pair into a single pill — the seam between segments must stay visible with square outer-paired corners.
- Do not recolor the timer well anything but near-black (`#181818`) — it is the one deliberately dark surface and must stay isolated from the light chrome.
- Do not apply the amber accent (`#D29400`) decoratively — it is a single reserved measurement highlight, not a brand color.
- Do not substitute a flat fill for the active-button gradients, or a gradient for the disabled buttons — fill type itself is the state signal here.
- Do not invent a serif display headline — Times New Roman appears only as an unstyled fallback, not a designed accent.