#!/usr/bin/env python3
"""The reusable UI components of LiveSplit One ASR Bridge, for Superdesign.

Each component is a Petite-Vue template styled with Tailwind, following
`.superdesign/design-system.md`. Components nest: cards and tabs use the small
components (Btn, Checkbox, AddressRow, ...), and every screen in
`.superdesign/screens/` is built only from components. Changing a component
here and running this script updates every screen that uses it.

Usage (from the repository root, logged in to the Superdesign CLI):
    python3 .superdesign/components/library.py            # create or update all
    python3 .superdesign/components/library.py Btn Card   # only these

Component ids are kept in `components.json` next to this file.
"""
import json, pathlib, subprocess, sys

HERE = pathlib.Path(__file__).resolve().parent
IDS_FILE = HERE / "components.json"
PROJECT = "bca2406a-5118-4388-9e38-b0e37fab0d9f"
CSS = ["https://fonts.googleapis.com/css2?family=Archivo+Black&family=Inter:wght@400;600&family=Space+Mono:wght@400;700&display=swap"]

# Design tokens (design-system.md). Tints are the status colour at 6 % over ink,
# pre-blended so they stay flat colours (egui-friendly).
INK, PANEL, RAISED = "#0B0B0C", "#151517", "#1E1E21"
BORDER, BORDER_STRONG = "#2A2A2E", "#3A3A40"
PAPER, SECONDARY, MUTED = "#F5F5F4", "#A1A1AA", "#6B6B73"
ORANGE, ORANGE_HOVER = "#FF4D00", "#FF6A26"
OK, ERROR, WARNING, INFO = "#22E07A", "#FF3B3B", "#FFC233", "#7DD3FC"
OK_TINT, ERROR_TINT = "#0C1813", "#1A0E0F"

DISPLAY = "font-['Archivo_Black'] uppercase tracking-[-0.02em]"
MONO = "font-['Space_Mono']"
BODY = "font-['Inter']"
LABEL = f"{MONO} text-[11px] uppercase tracking-[0.08em] text-[{MUTED}]"
CARD = f"rounded-[4px] border border-[{BORDER}] bg-[{INK}] p-[16px] flex flex-col"

IDS = json.loads(IDS_FILE.read_text()) if IDS_FILE.exists() else {}

def use(name, props, instance=None, vif=None):
    """A nested component. `props` is a JS object expression, e.g.
    "{label: 'Open…', variant: 'primary'}" or "{label: title}"."""
    cond = f' v-if="{vif}"' if vif else ""
    return (f'<sd-component{cond} componentId="{IDS[name]}" name="{name}" '
            f'instance="{instance or name.lower()}" :props="JSON.stringify({props})"></sd-component>')

def p(name, type_, default, description=""):
    return {"name": name, "type": type_, "defaultValue": default, "description": description}

# ---------------------------------------------------------------- primitives

def Btn():
    html = f'''<button type="button" class="w-fit inline-flex items-center justify-center rounded-[4px] border {BODY} font-semibold whitespace-nowrap"
  :class="{{
    'bg-[{ORANGE}] border-[{ORANGE}] text-[{INK}] hover:bg-[{ORANGE_HOVER}]': variant === 'primary',
    'bg-transparent border-[{BORDER_STRONG}] text-[{PAPER}] hover:bg-[{RAISED}]': variant === 'secondary',
    'h-[32px] px-[14px] text-[13px]': size === 'md',
    'h-[28px] px-[10px] text-[12px]': size === 'sm',
    'h-[24px] px-[8px] text-[11px]': size === 'xs',
    'opacity-40 pointer-events-none': disabled
  }}">{{{{ label }}}}</button>'''
    return html, [p("label", "string", "Button"), p("variant", "string", "secondary", "primary | secondary"),
                  p("size", "string", "md", "md (32px) | sm (28px) | xs (24px)"), p("disabled", "boolean", False)], \
        "Button. Primary = orange fill, secondary = outlined. previewWidth 120 previewHeight 40"

def SectionLabel():
    return f'<div class="{LABEL}">{{{{ text }}}}</div>', [p("text", "string", "SECTION")], \
        "Uppercase mono label used on cards and section headings. previewWidth 160 previewHeight 20"

def InfoMarker():
    return (f'<span title="Show help" class="inline-flex items-center justify-center w-[14px] h-[14px] rounded-full border border-[{MUTED}] '
            f'text-[{MUTED}] text-[9px] leading-none {MONO} align-middle">i</span>'), [], \
        "The small outlined 'i' that shows a tooltip on hover. previewWidth 20 previewHeight 20"

def Tooltip():
    return (f'<div class="max-w-[280px] rounded-[4px] border border-[{BORDER_STRONG}] bg-[{RAISED}] px-[12px] py-[8px] '
            f'{BODY} text-[12px] leading-[1.5] text-[{PAPER}]">{{{{ text }}}}</div>'), \
        [p("text", "string", "Tooltip text")], "Tooltip box. previewWidth 300 previewHeight 70"

def AddressRow():
    html = f'''<div class="flex items-center justify-between gap-[8px]">
  <div class="min-w-0">
    <div class="{MONO} text-[10px] text-[{MUTED}]">{{{{ tag }}}}</div>
    <div class="{MONO} whitespace-nowrap text-[{PAPER}]" :class="{{ 'text-[12px]': size === 'wide', 'text-[13px]': size === 'compact' }}">{{{{ url }}}}</div>
  </div>
  {use("Btn", "{label: 'Copy', size: 'xs'}")}
</div>'''
    return html, [p("tag", "string", "LAN"), p("url", "string", "ws://192.168.1.20:16834"), p("size", "string", "wide", "wide | compact")], \
        "A connection address with its network label and Copy. previewWidth 270 previewHeight 44"

def Checkbox():
    html = f'''<label class="flex items-center gap-[12px] cursor-pointer" :class="{{ 'pl-[16px]': indent }}">
  <span class="inline-flex items-center justify-center w-[16px] h-[16px] rounded-[3px] border"
    :class="{{ 'bg-[{ORANGE}] border-[{ORANGE}]': checked, 'bg-transparent border-[{BORDER_STRONG}]': !checked }}">
    <svg v-if="checked" viewBox="0 0 16 16" width="12" height="12"><path d="M3.5 8.5l3 3 6-7" fill="none" stroke="{INK}" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/></svg>
  </span>
  <span class="{BODY} text-[14px] text-[{PAPER}]">{{{{ label }}}}</span>
  {use("InfoMarker", "{}", vif="tooltip")}
</label>
<div v-if="note" class="{BODY} text-[13px] text-[{MUTED}] mt-[6px]" :class="{{ 'pl-[44px]': indent, 'pl-[28px]': !indent }}">{{{{ note }}}}</div>'''
    return html, [p("label", "string", "Setting"), p("checked", "boolean", True), p("tooltip", "boolean", False),
                  p("indent", "boolean", False), p("note", "string", "", "optional help line under the checkbox")], \
        "Checkbox setting, optionally indented under a level-2 heading, with tooltip marker and note. previewWidth 360 previewHeight 30"

# -------------------------------------------------------------- status cards

def AppHeader():
    html = f'''<div class="flex items-start justify-between gap-[8px]">
  <div>
    <div class="{DISPLAY} text-[{PAPER}] leading-tight" :class="{{ 'text-[14px]': size === 'wide', 'text-[13px]': size === 'compact' }}">LiveSplit One ASR Bridge</div>
    <div v-if="size === 'wide'" class="{MONO} text-[11px] text-[{MUTED}] mt-[4px]">{{{{ version }}}}</div>
  </div>
  {use("Btn", "{label: back ? '← Status' : 'Show details', size: 'xs'}", vif="size === 'compact'")}
</div>'''
    return html, [p("version", "string", "0.1.0-rc.1"), p("size", "string", "wide", "wide | compact"),
                  p("back", "boolean", False, "compact tab view: shows ← Status instead of Show details")], \
        "Wordmark and version at the top of the status column; in compact it carries Show details. previewWidth 270 previewHeight 44"

def ErrorCard():
    html = f'''<div class="rounded-[4px] border border-[{BORDER}] border-l-[3px] border-l-[{ERROR}] bg-[{ERROR_TINT}] p-[16px] flex flex-col gap-[12px]">
  <div class="flex items-center gap-[8px]">
    <span class="w-[8px] h-[8px] rounded-full bg-[{ERROR}]"></span>
    <span class="{DISPLAY} text-[{ERROR}] leading-none" :class="{{ 'text-[20px]': size === 'wide', 'text-[30px]': size === 'compact' }}">Error</span>
  </div>
  <div class="{BODY} text-[{PAPER}] leading-[1.45]" :class="{{ 'text-[13px]': size === 'wide', 'text-[14px]': size === 'compact' }}">
    <span v-if="kind === 'crashed'">The auto splitter stopped because of an error. Press Reload to start it again.</span>
    <span v-if="kind === 'loadFailed'">Couldn't load {{{{ fileName }}}}: not a valid WebAssembly module. The previous auto splitter is still running.</span>
    <span v-if="kind === 'portInUse'">Port {{{{ port }}}} is already in use. Choose another port in Connection and restart the server.</span>
  </div>
  <div class="flex flex-wrap gap-[6px]">
    {use("Btn", "{label: 'Show in log', size: 'sm'}", "showlog", vif="kind !== 'portInUse'")}
    {use("Btn", "{label: 'Reload', size: 'sm'}", "reload", vif="kind === 'crashed'")}
    {use("Btn", "{label: 'Open Connection', size: 'sm'}", "openconn", vif="kind === 'portInUse'")}
    {use("Btn", "{label: 'Dismiss', size: 'sm'}", "dismiss")}
  </div>
</div>'''
    return html, [p("kind", "string", "crashed", "crashed | loadFailed | portInUse"), p("size", "string", "wide", "wide | compact"),
                  p("fileName", "string", "foo.wasm"), p("port", "string", "16834")], \
        "The error card: only present while there is an error, always first in the status column. previewWidth 270 previewHeight 150"

def SplitterCard():
    html = f'''<div class="{CARD} gap-[12px]">
  {use("SectionLabel", "{text: 'Auto splitter'}")}
  <div v-if="loaded">
    <div class="{MONO} text-[13px] text-[{PAPER}] truncate">{{{{ fileName }}}}</div>
    <div class="{BODY} text-[13px] text-[{SECONDARY}] leading-snug mt-[4px]">{{{{ gameName }}}}</div>
    <a href="#change-game" id="change-game-link" class="{BODY} text-[12px] text-[{ORANGE}] hover:underline">Change</a>
  </div>
  <div v-if="!loaded">
    <div class="{MONO} text-[13px] text-[{MUTED}]">NO AUTO SPLITTER</div>
    <div class="{BODY} text-[13px] text-[{SECONDARY}] mt-[4px]">Open a .wasm auto splitter to start.</div>
  </div>
  <div class="flex gap-[8px]">
    {use("Btn", "{label: 'Open…', variant: 'primary'}", "open")}
    {use("Btn", "{label: 'Reload', disabled: !loaded}", "reload")}
  </div>
</div>'''
    return html, [p("loaded", "boolean", True), p("fileName", "string", "gta_sa_de_autosplitter.wasm"),
                  p("gameName", "string", "GTA San Andreas — Definitive Edition")], \
        "Auto splitter card (wide status column). previewWidth 270 previewHeight 170"

def SplitterStrip():
    html = f'''<div class="flex items-center justify-between gap-[8px]">
  <span v-if="loaded" class="{MONO} text-[12px] text-[{SECONDARY}] truncate">{{{{ fileName }}}}</span>
  <span v-if="!loaded" class="{MONO} text-[12px] text-[{MUTED}]">No auto splitter loaded</span>
  {use("Btn", "{label: 'Reload', size: 'xs'}", "reload", vif="loaded")}
  {use("Btn", "{label: 'Open…', variant: 'primary', size: 'xs'}", "open", vif="!loaded")}
</div>'''
    return html, [p("loaded", "boolean", True), p("fileName", "string", "gta_sa_de_autosplitter.wasm")], \
        "Compact replacement for the auto splitter card. previewWidth 370 previewHeight 30"

def status_card(label_expr, body):
    """Shared shell of the Game and Timer cards: ok = green edge + tint."""
    return f'''<div class="rounded-[4px] border border-[{BORDER}] p-[16px] flex flex-col gap-[4px]"
  :class="{{ 'border-l-[3px] border-l-[{OK}] bg-[{OK_TINT}]': tone === 'ok', 'bg-[{INK}]': tone !== 'ok' }}">
  <div class="mb-[4px]">{use("SectionLabel", label_expr)}</div>
{body}
</div>'''

def word(text, colour, dot):
    """A status word with its dot: dot is 'filled' (in the word's colour), 'outline' or None."""
    dots = {"filled": f'<span class="w-[8px] h-[8px] rounded-full shrink-0 bg-[{colour}]" :class="size === \'compact\' ? \'mt-[12px]\' : \'mt-[7px]\'"></span>',
            "outline": f'<span class="w-[8px] h-[8px] rounded-full shrink-0 border border-[{SECONDARY}]" :class="size === \'compact\' ? \'mt-[12px]\' : \'mt-[7px]\'"></span>', None: ""}
    return f'''<div class="flex items-start" :class="size === 'compact' ? 'gap-[10px]' : 'gap-[8px]'">
    {dots[dot]}
    <span class="{DISPLAY} leading-[1.05] text-[{colour}]" :class="size === 'compact' ? 'text-[30px]' : 'text-[20px]'">{text}</span>
  </div>'''

def GameCard():
    body = f'''  <div v-if="state === 'attached'" class="flex flex-col gap-[4px]">
    {word("Attached", OK, "filled")}
    <div class="{MONO} text-[12px] text-[{SECONDARY}]">{{{{ process }}}}<span v-if="size === 'wide'"> · {{{{ tickRate }}}}</span></div>
  </div>
  <div v-if="state === 'waiting'" class="flex flex-col gap-[4px]">
    {word("Waiting for game…", SECONDARY, "outline")}
    <div class="{BODY} text-[13px] text-[{SECONDARY}]">Start the game. This is normal before a run.</div>
  </div>
  <div v-if="state === 'stopped'" class="flex flex-col gap-[4px]">
    {word("Stopped", SECONDARY, "outline")}
    <div class="{BODY} text-[13px] text-[{SECONDARY}]">The auto splitter isn't running</div>
  </div>
  <div v-if="state === 'none'" class="flex flex-col gap-[4px]">
    <div class="{DISPLAY} leading-none text-[{MUTED}]" :class="size === 'compact' ? 'text-[30px]' : 'text-[20px]'">—</div>
    <div class="{BODY} text-[13px] text-[{SECONDARY}]">Load an auto splitter first</div>
  </div>'''
    html = status_card("{text: 'Game'}", body).replace("tone === 'ok'", "state === 'attached'").replace("tone !== 'ok'", "state !== 'attached'")
    return html, [p("state", "string", "attached", "attached | waiting | stopped | none"), p("process", "string", "SanAndreas.exe"),
                  p("tickRate", "string", "20 Hz"), p("size", "string", "wide", "wide | compact")], \
        "Game status card. previewWidth 270 previewHeight 110"

def TimerCard():
    connected_word = word("Connected", OK, "filled")
    neutral = lambda w: word(w, SECONDARY, "outline")
    body = f'''  <div v-if="state === 'connected'" class="flex flex-col gap-[4px]">
    {connected_word}
    <div class="{MONO} text-[12px] text-[{SECONDARY}]">{{{{ timers }}}} {{{{ timers === 1 ? 'timer' : 'timers' }}}} · LiveSplit One</div>
    <div v-if="size === 'wide'" class="flex items-center justify-between gap-[8px] mt-[8px]">
      <span class="{MONO} text-[12px] text-[{SECONDARY}]">Addresses: Connection tab</span>
      {use("Btn", "{label: '?', size: 'xs'}", "help")}
    </div>
  </div>
  <div v-if="state === 'notConnected'" class="flex flex-col gap-[4px]">
    {neutral("Not connected")}
    <a href="#how-to-connect" id="how-to-connect-link" class="{BODY} text-[13px] font-semibold text-[{ORANGE}] hover:underline">How do I connect?</a>
    <div class="h-px bg-[{BORDER}] my-[8px]"></div>
    <div class="flex flex-col gap-[8px]">
      {use("AddressRow", "{tag: 'LAN', url: lanUrl, size: size}", "lan")}
      {use("AddressRow", "{tag: 'VPN', url: vpnUrl, size: size}", "vpn", vif="size === 'wide' && vpnUrl")}
    </div>
  </div>
  <div v-if="state === 'serverStopped'" class="flex flex-col gap-[4px]">
    {neutral("Server stopped")}
    <div class="{BODY} text-[13px] text-[{SECONDARY}]">No timer can connect until the server restarts</div>
  </div>'''
    html = status_card("{text: 'Timer'}", body).replace("tone === 'ok'", "state === 'connected'").replace("tone !== 'ok'", "state !== 'connected'")
    return html, [p("state", "string", "connected", "connected | notConnected | serverStopped"), p("timers", "number", 1),
                  p("lanUrl", "string", "ws://192.168.1.20:16834"), p("vpnUrl", "string", "ws://100.101.7.3:16834"),
                  p("size", "string", "wide", "wide | compact (compact shows only the first address)")], \
        "Timer status card. Addresses only while not connected. previewWidth 270 previewHeight 200"

def LastActionCard():
    html = f'''<div class="rounded-[4px] border border-[{BORDER}] bg-[{INK}] p-[16px] flex flex-col gap-[4px]"
  :class="{{ 'border-l-[3px] border-l-[{ORANGE}]': state === 'action' }}">
  <div class="flex items-center justify-between mb-[4px]">
    {use("SectionLabel", "{text: 'Last action'}")}
    <span v-if="state === 'action'" class="{MONO} text-[12px] text-[{MUTED}]">{{{{ time }}}}</span>
  </div>
  <div v-if="state === 'action'" class="flex flex-col gap-[4px]">
    <div class="{DISPLAY} leading-none text-[{ORANGE}]" :class="size === 'compact' ? 'text-[30px]' : 'text-[24px]'">{{{{ action }}}}</div>
    <div v-if="segment" class="{BODY} font-semibold text-[{PAPER}] truncate" :class="size === 'compact' ? 'text-[16px]' : 'text-[14px]'">{{{{ segment }}}}</div>
    <div v-if="notSent" class="{MONO} text-[12px] text-[{WARNING}]">Not sent: no timer connected</div>
    <div class="flex flex-col gap-[4px] mt-[8px]">
      <div v-if="previous1" class="{MONO} text-[12px] text-[{MUTED}] whitespace-pre truncate">{{{{ previous1 }}}}</div>
      <div v-if="previous2" class="{MONO} text-[12px] text-[{MUTED}] whitespace-pre truncate">{{{{ previous2 }}}}</div>
    </div>
  </div>
  <div v-if="state === 'empty'" class="flex flex-col gap-[4px]">
    <div class="{DISPLAY} leading-none text-[{MUTED}]" :class="size === 'compact' ? 'text-[30px]' : 'text-[20px]'">No actions yet</div>
    <div class="{BODY} text-[13px] text-[{SECONDARY}] leading-snug mt-[4px]">Actions appear here when the auto splitter starts, splits or resets.</div>
  </div>
</div>'''
    return html, [p("state", "string", "action", "action | empty"), p("action", "string", "SPLIT"), p("segment", "string", "Los Santos — Gym Moves", "only when the timer provides it"),
                  p("time", "string", "19:31:01"), p("previous1", "string", "19:28:44  SPLIT  Big Smoke"), p("previous2", "string", "19:25:03  START"),
                  p("notSent", "boolean", False), p("size", "string", "wide", "wide | compact")], \
        "Last timer action with the two previous ones. previewWidth 270 previewHeight 160"

# ------------------------------------------------------------------ tab area

def TabStrip():
    tab = lambda key, label: (f'<a href="#{key}" id="tab-{key}" class="h-[44px] flex items-center {BODY} font-semibold text-[12px] uppercase tracking-[0.06em] border-b-2" '
                             f':class="active === \'{key}\' ? \'text-[{PAPER}] border-[{ORANGE}]\' : \'text-[{SECONDARY}] border-transparent hover:text-[{PAPER}]\'">{label}'
                             + (f'<span v-if="unsaved" class="text-[{WARNING}] ml-[4px]">•</span>' if key == "settings" else "") + '</a>')
    html = f'''<nav class="h-[44px] flex items-end gap-[24px] px-[24px] border-b border-[{BORDER}] bg-[{INK}]">
  {tab("settings", "Settings")}{tab("connection", "Connection")}{tab("log", "Log")}{tab("preferences", "Preferences")}
</nav>'''
    return html, [p("active", "string", "settings", "settings | connection | log | preferences"), p("unsaved", "boolean", False)], \
        "Tab strip; the active tab has a 2px orange underline; • marks unsaved settings. previewWidth 980 previewHeight 44"

def SettingsToolbar():
    html = f'''<div class="flex items-center gap-[12px] pb-[24px] border-b border-[{BORDER}]">
  {use("Btn", "{label: 'Save', variant: 'primary', disabled: status !== 'unsaved'}", "save")}
  {use("Btn", "{label: 'Revert to defaults'}", "revert")}
  <span v-if="status === 'unsaved'" class="{BODY} text-[13px] text-[{WARNING}]">● Unsaved changes</span>
  <span v-if="status === 'saved'" class="{BODY} text-[13px] text-[{OK}]">✓ Saved</span>
</div>'''
    return html, [p("status", "string", "unsaved", "unsaved | saved | none")], "Settings tab toolbar. previewWidth 600 previewHeight 60"

def SettingHeading():
    html = f'''<div v-if="level === 1" class="{LABEL} pt-[24px] pb-[12px]">{{{{ text }}}}</div>
<div v-if="level === 2" class="{BODY} font-semibold text-[13px] text-[{SECONDARY}] pt-[12px] pb-[8px] pl-[16px]">{{{{ text }}}}</div>'''
    return html, [p("text", "string", "Splits"), p("level", "number", 1, "1 | 2")], "Settings group heading, level 1 or 2. previewWidth 300 previewHeight 40"

def SettingChoice():
    html = f'''<div class="flex items-center gap-[12px]" :class="{{ 'pl-[16px]': indent }}">
  <span class="{BODY} text-[14px] text-[{PAPER}]">{{{{ label }}}}</span>
  <div class="w-[240px] h-[32px] rounded-[4px] border border-[{BORDER_STRONG}] bg-[{PANEL}] px-[12px] flex items-center justify-between {BODY} text-[13px] text-[{PAPER}]">
    <span>{{{{ value }}}}</span>
    <svg width="12" height="12" viewBox="0 0 12 12"><path d="M3 4.5l3 3 3-3" fill="none" stroke="{SECONDARY}" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
  </div>
</div>'''
    return html, [p("label", "string", "Collectible splits"), p("value", "string", "Every 10"), p("indent", "boolean", False)], \
        "Choice (dropdown) setting. previewWidth 420 previewHeight 36"

def SettingFile():
    html = f'''<div class="flex items-center gap-[12px]">
  <span class="{BODY} text-[14px] text-[{PAPER}] w-[140px] shrink-0">{{{{ label }}}}</span>
  <div class="flex-1 min-w-0 h-[32px] rounded-[4px] border border-[{BORDER_STRONG}] bg-[{PANEL}] px-[12px] flex items-center {MONO} text-[12px] text-[{PAPER}] truncate">{{{{ path }}}}</div>
  {use("Btn", "{label: 'Browse…'}", "browse")}
</div>'''
    return html, [p("label", "string", "Route file"), p("path", "string", "~/Documents/Speedruns/GTA SA/DE/any-route.txt")], \
        "File selection setting: path and Browse…. previewWidth 700 previewHeight 36"

def EmptyState():
    html = f'''<div class="flex flex-col items-center text-center gap-[12px] max-w-[420px] mx-auto">
  <div class="{DISPLAY} text-[20px] text-[{PAPER}]">No auto splitter loaded</div>
  <div class="{BODY} text-[14px] text-[{SECONDARY}] leading-[1.5]">Open a .wasm auto splitter to see its settings. Settings are saved per game.</div>
  {use("Btn", "{label: 'Open…', variant: 'primary'}", "open")}
  <div class="{MONO} text-[11px] text-[{MUTED}] leading-[1.6] mt-[12px]">Auto splitters for LiveSplit are .wasm files, usually linked from the game's speedrun.com resources.</div>
</div>'''
    return html, [], "Settings tab with no auto splitter loaded. previewWidth 440 previewHeight 200"

def ServerSection():
    html = f'''<div class="flex flex-col gap-[16px]">
  {use("SectionLabel", "{text: 'Server'}")}
  <div class="flex items-center gap-[12px]">
    <span class="{BODY} text-[14px] text-[{PAPER}]">Port</span>
    <div class="w-[120px] h-[32px] rounded-[4px] border bg-[{PANEL}] px-[12px] flex items-center {MONO} text-[13px] text-[{PAPER}]"
      :class="{{ 'border-[{ORANGE}]': edited, 'border-[{ERROR}]': portError, 'border-[{BORDER_STRONG}]': !edited && !portError }}">{{{{ port }}}}</div>
    {use("Btn", "{label: 'Restart server', variant: portError ? 'primary' : 'secondary'}", "restart")}
    <span v-if="edited" class="{BODY} text-[13px] text-[{WARNING}]">● Restart the server to apply</span>
  </div>
  <div v-if="portError" class="rounded-[4px] border border-[{ERROR}] bg-[{ERROR_TINT}] px-[14px] py-[10px] {BODY} text-[13px] text-[{PAPER}]">Couldn't start the server: port {{{{ port }}}} is already in use by another program. Try {{{{ suggestion }}}}.</div>
</div>'''
    return html, [p("port", "string", "16834"), p("edited", "boolean", False), p("portError", "boolean", False), p("suggestion", "string", "16835")], \
        "Connection tab: port and Restart server, with the edited note and port-in-use message. previewWidth 700 previewHeight 110"

def TimerRow():
    html = f'''<div class="flex items-center gap-[12px] h-[48px] px-[14px] rounded-[4px] border border-[{BORDER}] bg-[{PANEL}]">
  <span class="{MONO} text-[13px] text-[{PAPER}]">{{{{ address }}}}</span>
  <span class="text-[{MUTED}]">·</span>
  <span class="{BODY} text-[13px] text-[{SECONDARY}]">{{{{ state }}}}</span>
  <span v-if="primary" class="ml-auto {MONO} text-[10px] uppercase tracking-[0.08em] text-[{SECONDARY}] border border-[{BORDER_STRONG}] rounded-[3px] px-[6px] py-[2px]">Primary</span>
</div>'''
    return html, [p("address", "string", "192.168.1.42:53122"), p("state", "string", "Running · split 12"), p("primary", "boolean", False)], \
        "A connected timer in the Connection tab. previewWidth 700 previewHeight 50"

def HowToConnect():
    step = lambda n, body: (f'<div class="flex gap-[14px]"><span class="w-[24px] h-[24px] shrink-0 rounded-full bg-[{RAISED}] flex items-center justify-center {MONO} text-[12px] text-[{PAPER}]">{n}</span>'
                            f'<div class="flex-1 {BODY} text-[14px] text-[{PAPER}] pt-[2px]">{body}</div></div>')
    html = f'''<div class="flex flex-col gap-[16px]">
  <div class="flex items-center justify-between">
    {use("SectionLabel", "{text: 'How to connect'}")}
    <a v-if="collapsed" href="#show-steps" id="show-steps-link" class="{BODY} text-[13px] text-[{ORANGE}] hover:underline">Show setup steps ⌄</a>
  </div>
  <div v-if="!collapsed" class="flex flex-col gap-[18px]">
    {step(1, f'<div class="font-semibold mb-[10px]">Copy an address</div><div class="flex flex-col gap-[8px] w-[280px] rounded-[4px] border border-[{BORDER}] bg-[{PANEL}] p-[12px]">' + use("AddressRow", "{tag: 'LAN', url: lanUrl}", "lan") + use("AddressRow", "{tag: 'VPN', url: vpnUrl}", "vpn") + '</div>')}
    {step(2, 'In LiveSplit One open <b>Settings → Connect to Server</b> and paste it.')}
    {step(3, f'When Chrome asks, allow local network access.<div class="text-[13px] text-[{MUTED}] mt-[4px]">LiveSplit One must run in a Chrome-based browser.</div>')}
  </div>
</div>'''
    return html, [p("collapsed", "boolean", False), p("lanUrl", "string", "ws://192.168.1.20:16834"), p("vpnUrl", "string", "ws://100.101.7.3:16834")], \
        "Connection tab setup steps. previewWidth 700 previewHeight 280"

def LogToolbar():
    html = f'''<div class="flex flex-col gap-[12px] pb-[16px] border-b border-[{BORDER}]">
  <div class="flex items-center gap-[20px]">
    {use("SectionLabel", "{text: 'Show'}")}
    {use("Checkbox", "{label: 'Auto splitter', checked: autoSplitter}", "c1")}
    {use("Checkbox", "{label: 'Connection', checked: connection}", "c2")}
    {use("Checkbox", "{label: 'App & runtime', checked: app}", "c3")}
    <span class="{BODY} text-[13px] text-[{MUTED}]">Errors are always shown.</span>
  </div>
  <div class="flex gap-[8px]">
    {use("Btn", "{label: 'Copy'}", "copy")}{use("Btn", "{label: 'Save log…'}", "save")}{use("Btn", "{label: 'Clear'}", "clear")}{use("Btn", "{label: 'Open log folder'}", "folder")}
  </div>
</div>'''
    return html, [p("autoSplitter", "boolean", True), p("connection", "boolean", False), p("app", "boolean", True)], \
        "Log tab filters and actions. previewWidth 900 previewHeight 90"

def LogLine():
    html = f'''<div class="flex gap-[16px] {MONO} text-[12px] leading-[1.9] px-[8px] -mx-[8px] rounded-[4px] border"
  :class="highlighted ? 'border-[{ORANGE}] bg-[{RAISED}] py-[6px] my-[4px]' : 'border-transparent'">
  <span class="w-[92px] shrink-0 text-[{MUTED}]">{{{{ time }}}}</span>
  <span class="w-[110px] shrink-0 uppercase" :class="{{ 'text-[{ORANGE}]': category === 'auto splitter', 'text-[{INFO}]': category === 'connection', 'text-[{SECONDARY}]': category === 'app', 'text-[{ERROR}]': category === 'error' }}">{{{{ category }}}}</span>
  <div class="min-w-0">
    <div class="text-[{PAPER}]">{{{{ message }}}}</div>
    <div v-if="trace" class="text-[{MUTED}] whitespace-pre leading-[1.6]">{{{{ trace }}}}</div>
  </div>
</div>'''
    return html, [p("time", "string", "19:31:01.254"), p("category", "string", "auto splitter", "auto splitter | connection | app | error"),
                  p("message", "string", "Split"), p("trace", "string", "", "optional indented detail lines"), p("highlighted", "boolean", False)], \
        "A log line; highlighted when reached through Show in log. previewWidth 900 previewHeight 26"

def AboutRow():
    html = f'''<div class="flex items-center justify-between gap-[16px] max-w-[600px]">
  <div><div class="{BODY} text-[13px] text-[{SECONDARY}]">{{{{ label }}}}</div><div class="{MONO} text-[13px] text-[{PAPER}] mt-[2px]">{{{{ path }}}}</div></div>
  {use("Btn", "{label: 'Open'}", "open")}
</div>'''
    return html, [p("label", "string", "Config folder"), p("path", "string", "~/.config/livesplit-asr-bridge")], \
        "Preferences About row: a folder with Open. previewWidth 600 previewHeight 44"

def dialog_shell(title, body, buttons, width):
    return f'''<div class="w-[{width}px] rounded-[4px] border border-[{BORDER_STRONG}] bg-[{PANEL}] p-[24px] flex flex-col gap-[16px]">
  <div class="{DISPLAY} text-[16px] text-[{PAPER}]">{title}</div>
{body}
  <div class="flex justify-end gap-[8px] pt-[4px]">{buttons}</div>
</div>'''

def GameDialog():
    row = lambda name, n, key: (f'<div class="flex items-center justify-between h-[40px] px-[12px] rounded-[4px] border" :class="selected === \'{key}\' ? \'border-[{ORANGE}] bg-[{RAISED}]\' : \'border-transparent\'">'
                               f'<span class="{BODY} text-[14px] text-[{PAPER}]">{name}</span><span class="{MONO} text-[11px] text-[{SECONDARY}]">{n}</span></div>')
    body = f'''  <div class="{MONO} text-[12px] text-[{SECONDARY}] -mt-[8px]">{{{{ fileName }}}}</div>
  <div class="flex flex-col gap-[6px]">
    <span class="{BODY} font-semibold text-[13px] text-[{PAPER}]">Game</span>
    <div class="h-[36px] rounded-[4px] border border-[{ORANGE}] bg-[{INK}] px-[12px] flex items-center {BODY} text-[14px] text-[{PAPER}]">{{{{ typed }}}}</div>
  </div>
  <div class="flex flex-col gap-[4px]">
    <div class="{LABEL} mb-[4px]">Or pick a game you already set up</div>
    {row("GTA San Andreas — Definitive Edition", "2 auto splitters", "gtasa")}
    {row("GTA Vice City — Definitive Edition", "1 auto splitter", "gtavc")}
    {row("Hollow Knight", "1 auto splitter", "hk")}
  </div>
  <div class="{BODY} text-[12px] text-[{MUTED}]">Settings are saved per game, so auto splitters for the same game share them.</div>'''
    html = dialog_shell("{{ mode === 'change' ? 'Change game' : 'Which game is this auto splitter for?' }}", body,
                        use("Btn", "{label: 'Cancel'}", "cancel") + use("Btn", "{label: 'Use this game', variant: 'primary'}", "use"), 480)
    return html, [p("mode", "string", "new", "new | change"), p("fileName", "string", "gta_sa_de_autosplitter.wasm"),
                  p("typed", "string", "gta sa de"), p("selected", "string", "gtasa", "gtasa | gtavc | hk | none")], \
        "Game association dialog (new file or Change). previewWidth 480 previewHeight 420"

def UnsavedDialog():
    body = f'''  <div class="{BODY} text-[14px] text-[{PAPER}] leading-[1.5]">You changed {{{{ count }}}} settings for {{{{ game }}}}. <span v-if="trigger === 'reload'">Reloading the auto splitter</span><span v-if="trigger === 'open'">Opening another auto splitter</span><span v-if="trigger === 'close'">Closing the app</span> without saving discards them.</div>'''
    html = dialog_shell("Save your settings changes?", body,
                        use("Btn", "{label: 'Cancel'}", "cancel") + use("Btn", "{label: 'Discard'}", "discard")
                        + use("Btn", "{label: trigger === 'reload' ? 'Save and reload' : 'Save', variant: 'primary'}", "save"), 440)
    return html, [p("count", "number", 3), p("game", "string", "GTA San Andreas — Definitive Edition"), p("trigger", "string", "reload", "reload | open | close")], \
        "Unsaved settings dialog. previewWidth 440 previewHeight 200"

# Order matters: a component can only nest components defined above it.
LIBRARY = [Btn, SectionLabel, InfoMarker, Tooltip, AddressRow, Checkbox,
           AppHeader, ErrorCard, SplitterCard, SplitterStrip, GameCard, TimerCard, LastActionCard,
           TabStrip, SettingsToolbar, SettingHeading, SettingChoice, SettingFile, EmptyState,
           ServerSection, TimerRow, HowToConnect, LogToolbar, LogLine, AboutRow, GameDialog, UnsavedDialog]

def cli(*args):
    out = subprocess.run(["npx", "--yes", "@superdesign/cli@latest", *args, "--json"], capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"{args[0]} failed: {out.stderr or out.stdout}")
    return json.loads(out.stdout)

def main(only):
    for fn in LIBRARY:
        name = fn.__name__
        html, props, desc = fn()
        # The canvas preview of a component ignores --css-imports, so each
        # template loads the fonts itself (browsers honour a stylesheet link
        # in the body; repeats are served from cache).
        # The canvas preview also has a white page; the app is dark, so give the
        # preview page the window colour (screens set their own backgrounds).
        html = f'<link rel="stylesheet" href="{CSS[0]}"><style>body{{background:{INK}}}</style>\n' + html
        (HERE / f"{name}.html").write_text(html + "\n")
        if only and name not in only:
            continue
        common = ["--html-file", str(HERE / f"{name}.html"), "--description", desc,
                  "--props", json.dumps(props), "--css-imports", json.dumps(CSS)]
        if name in IDS:
            r = cli("update-component", "--component-id", IDS[name], *common)
            print(f"updated {name} v{r.get('version')}")
        else:
            r = cli("create-component", "--project-id", PROJECT, "--name", name, *common)
            IDS[name] = r["componentId"]
            IDS_FILE.write_text(json.dumps(IDS, indent=2) + "\n")
            print(f"created {name} {IDS[name]}")

if __name__ == "__main__":
    main(set(sys.argv[1:]))
