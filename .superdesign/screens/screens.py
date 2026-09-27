#!/usr/bin/env python3
"""Every mockup screen of LiveSplit One ASR Bridge, built only from the
components in `.superdesign/components/` (see library.py there).

A screen here is just a layout (status column + tab area, or the compact
column) filled with <sd-component> placements. Styling lives in the
components, so changing a component changes every screen that uses it.

Usage (from the repository root, logged in to the Superdesign CLI):
    python3 .superdesign/screens/screens.py          # write and import all
    python3 .superdesign/screens/screens.py W1 C2    # only these
Draft ids are in `screens.json` next to this file.
"""
import json, pathlib, subprocess, sys

HERE = pathlib.Path(__file__).resolve().parent
COMPONENTS = json.loads((HERE.parent / "components" / "components.json").read_text())
DRAFTS_FILE = HERE / "screens.json"
DRAFTS = json.loads(DRAFTS_FILE.read_text()) if DRAFTS_FILE.exists() else {}
PROJECT = "bca2406a-5118-4388-9e38-b0e37fab0d9f"

_n = 0
def c(name, **props):
    """Place a component with static props."""
    global _n; _n += 1
    return (f"<sd-component componentId=\"{COMPONENTS[name]}\" name=\"{name}\" instance=\"{name.lower()}-{_n}\" "
            f"props='{json.dumps(props, ensure_ascii=False)}'></sd-component>")

HEAD = """<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>{title}</title>
<script src="https://cdn.tailwindcss.com"></script>
<link href="https://fonts.googleapis.com/css2?family=Archivo+Black&family=Inter:wght@400;600&family=Space+Mono:wght@400;700&display=swap" rel="stylesheet">
<style>sd-component{{display:block}} .col>sd-component{{flex-shrink:0}}</style>
</head><body>
"""

# ------------------------------------------------------------------ status

def column(error=None, loaded=True, game="attached", timer="connected", timers=1, last="action", **last_props):
    parts = [c("AppHeader")]
    if error:
        parts.append(c("ErrorCard", kind=error))
    parts.append(c("SplitterCard", loaded=loaded))
    parts.append(c("GameCard", state=game))
    parts.append(c("TimerCard", state=timer, timers=timers))
    parts.append(c("LastActionCard", state=last, **last_props))
    return ("<aside class=\"col w-[300px] shrink-0 h-full bg-[#151517] border-r border-[#2A2A2E] p-[16px] flex flex-col gap-[12px] overflow-y-auto\">\n  "
            + "\n  ".join(parts) + "\n</aside>")

def wide(title, status, tab, content, unsaved=False, overlay=None):
    body = f"""<div class="w-[1280px] h-[800px] flex bg-[#0B0B0C] overflow-hidden relative">
{status}
<main class="flex-1 min-w-0 flex flex-col">
  {c("TabStrip", active=tab, unsaved=unsaved)}
  <div class="flex-1 overflow-y-auto p-[24px]">
{content}
  </div>
</main>
{f'<div class="absolute inset-0 bg-black/60 flex items-center justify-center z-50">{overlay}</div>' if overlay else ''}
</div>"""
    return HEAD.format(title=title) + body + "\n</body></html>\n"

def compact(title, error=None, loaded=True, game="attached", timer="connected", last="action", **last_props):
    parts = [c("AppHeader", size="compact")]
    if error:
        parts.append(c("ErrorCard", kind=error, size="compact"))
    parts.append(c("SplitterStrip", loaded=loaded))
    parts.append(c("GameCard", state=game, size="compact"))
    parts.append(c("TimerCard", state=timer, size="compact"))
    parts.append(c("LastActionCard", state=last, size="compact", **last_props))
    body = ("<div class=\"col w-[400px] h-[800px] bg-[#151517] p-[16px] flex flex-col gap-[12px] overflow-y-auto\">\n  "
            + "\n  ".join(parts) + "\n</div>")
    return HEAD.format(title=title) + body + "\n</body></html>\n"

# ------------------------------------------------------------------ tabs

def stack(*items, gap=12):
    return f'<div class="flex flex-col gap-[{gap}px]">' + "".join(items) + "</div>"

def settings(status="unsaved", all_kinds=False):
    rows = [c("SettingsToolbar", status=status)]
    if all_kinds:
        rows += [c("SettingHeading", text="Routing"), c("SettingFile")]
    rows += [c("SettingHeading", text="Start & reset"),
             stack(c("Checkbox", label="Start on new game", checked=True), c("Checkbox", label="Reset on main menu", checked=False)),
             c("SettingHeading", text="Splits"),
             stack(c("Checkbox", label="Split on mission passed", checked=True), c("Checkbox", label="Split on 100% stat increase", checked=False),
                   c("SettingChoice"))]
    if all_kinds:
        rows += [c("SettingHeading", text="Collectibles", level=2),
                 stack(*[c("Checkbox", label=l, checked=ch, indent=True, tooltip=(l == "Tags")) for l, ch in
                         (("Tags", True), ("Snapshots", False), ("Horseshoes", False), ("Oysters", False))])]
    else:
        rows += [c("SettingHeading", text="Collectibles"),
                 stack(*[c("Checkbox", label=l, checked=ch, tooltip=(l == "Tags")) for l, ch in
                         (("Tags", True), ("Snapshots", False), ("Horseshoes", False), ("Oysters", False))])]
    rows += [c("SettingHeading", text="Timing")]
    loads = c("Checkbox", label="Remove loads (game time)", checked=True, tooltip=True)
    if all_kinds:
        loads = (f'<div class="flex items-center gap-[12px]">{loads}'
                 + c("Tooltip", text="Pauses game time during loading screens. Use with the Game Time comparison in LiveSplit One.") + "</div>")
    rows.append(loads)
    return "\n".join(rows)

def connection(port="16834", edited=False, port_error=False, rows=(), collapsed=False):
    timers = ("".join(c("TimerRow", address=a, state=s, primary=p) for a, s, p in rows) if rows
              else '<div class="font-[\'Inter\'] text-[14px] text-[#A1A1AA]">No timers connected yet.</div>')
    return stack(c("ServerSection", port=port, edited=edited, portError=port_error),
                 '<div class="h-px bg-[#2A2A2E] my-[8px]"></div>',
                 c("SectionLabel", text="Connected timers"), stack(timers, gap=8),
                 '<div class="h-px bg-[#2A2A2E] my-[8px]"></div>',
                 c("HowToConnect", collapsed=collapsed), gap=16)

LOG = [("19:25:01.870", "app", "Loaded gta_sa_de_autosplitter.wasm (game: GTA San Andreas — Definitive Edition)"),
       ("19:25:01.902", "app", "Tick rate changed to 20.0 Hz"),
       ("19:25:02.415", "app", "Attached to the game (SanAndreas.exe)"),
       ("19:25:02.430", "auto splitter", "Found script base at 0x7FF6A1C2B000"),
       ("19:25:03.004", "auto splitter", "New game detected"),
       ("19:25:03.005", "auto splitter", "Start"),
       ("19:26:40.512", "auto splitter", "Mission passed: In the Beginning"),
       ("19:26:40.513", "auto splitter", "Split"),
       ("19:27:58.090", "auto splitter", "Mission passed: Ryder"),
       ("19:27:58.091", "auto splitter", "Split"),
       ("19:28:44.117", "auto splitter", "Mission passed: Big Smoke"),
       ("19:28:44.118", "auto splitter", "Split"),
       ("19:29:30.660", "auto splitter", "Loading screen: pausing game time"),
       ("19:29:33.214", "auto splitter", "Loading done: resuming game time"),
       ("19:31:01.253", "auto splitter", "Mission passed: Los Santos — Gym Moves"),
       ("19:31:01.254", "auto splitter", "Split"),
       ("19:31:05.100", "auto splitter", "Reading tag count")]

def log():
    lines = [c("LogLine", time=t, category=k, message=m) for t, k, m in LOG]
    lines.append(c("LogLine", time="19:31:08.992", category="error", highlighted=True,
                   message="The auto splitter stopped because of an error: wasm trap: out of bounds memory access",
                   trace="    at gta_sa_de_autosplitter::update (wasm function 42)\n    at gta_sa_de_autosplitter::read_tags (wasm function 57)"))
    lines.append(c("LogLine", time="19:31:08.993", category="app", message="Game detached"))
    return c("LogToolbar") + '<div class="pt-[12px] flex flex-col">' + "".join(lines) + "</div>"

def preferences():
    return stack(c("SectionLabel", text="Window"),
                 c("Checkbox", label="Remember window size and position", checked=True,
                   note="Turn this off with a tiling window manager, so the window manager decides the size."),
                 '<div class="h-[16px]"></div>',
                 c("SectionLabel", text="About"),
                 '<div class="font-[\'Space_Mono\'] text-[13px] text-[#F5F5F4]">LiveSplit One ASR Bridge 0.1.0-rc.1</div>',
                 c("AboutRow", label="Config folder", path="~/.config/livesplit-asr-bridge"),
                 c("AboutRow", label="Log folder", path="~/.local/state/livesplit-asr-bridge/logs"), gap=16)

MID_RUN = dict(action="SPLIT", segment="Los Santos — Gym Moves", time="19:31:01",
               previous1="19:28:44  SPLIT  Big Smoke", previous2="19:25:03  START")

# ------------------------------------------------------------------ screens
# key: (title, width, height, html builder)
SCREENS = {
 "WIDE": ("Wide — Settings, unsaved", 1280, 800, lambda: wide("Wide", column(**MID_RUN), "settings", settings(), unsaved=True)),
 "W1": ("W1 First launch — Connection tab", 1280, 800, lambda: wide("W1", column(loaded=False, game="none", timer="notConnected", last="empty"), "connection", connection())),
 "W2": ("W2 Which game is this auto splitter for?", 1280, 800, lambda: wide("W2", column(**MID_RUN), "settings", settings(), unsaved=True, overlay=c("GameDialog"))),
 "W3": ("W3 Connection tab — changing the port", 1280, 800, lambda: wide("W3", column(timers=2, **MID_RUN), "connection",
        connection(port="16900", edited=True, collapsed=True, rows=(("192.168.1.42:53122", "Running · split 12", True), ("192.168.1.42:53140", "Running · split 12", False))))),
 "W4": ("W4 Port in use", 1280, 800, lambda: wide("W4", column(error="portInUse", timer="serverStopped", **MID_RUN), "connection", connection(port_error=True))),
 "W5": ("W5 Log tab — Show in log after a crash", 1280, 800, lambda: wide("W5", column(error="crashed", game="stopped", **MID_RUN), "log", log())),
 "W6": ("W6 Unsaved settings dialog", 1280, 800, lambda: wide("W6", column(**MID_RUN), "settings", settings(), unsaved=True, overlay=c("UnsavedDialog"))),
 "W7": ("W7 Settings saved, all widget kinds", 1280, 800, lambda: wide("W7", column(**MID_RUN), "settings", settings(status="saved", all_kinds=True))),
 "W8": ("W8 Settings tab — no auto splitter", 1280, 800, lambda: wide("W8", column(loaded=False, game="none", last="empty"), "settings",
        '<div class="h-full flex items-center justify-center">' + c("EmptyState") + "</div>")),
 "W9": ("W9 Preferences tab", 1280, 800, lambda: wide("W9", column(**MID_RUN), "preferences", preferences())),
 "C1": ("C1 Compact — mid-run", 400, 800, lambda: compact("C1", **MID_RUN)),
 "C1e": ("C1e Compact — auto splitter crashed", 400, 800, lambda: compact("C1e", error="crashed", game="stopped", **MID_RUN)),
 "C2": ("C2 Compact — before the run", 400, 800, lambda: compact("C2", game="waiting", timer="notConnected", last="empty")),
 "C3": ("C3 Compact — no auto splitter", 400, 800, lambda: compact("C3", loaded=False, game="none", timer="notConnected", last="empty")),
 "C4": ("C4 Compact — tab view (Show details on a tiling WM, or first launch)", 400, 800, lambda: HEAD.format(title="C4") +
        '<div class="w-[400px] h-[800px] bg-[#0B0B0C] flex flex-col overflow-hidden"><div class="p-[16px] bg-[#151517] border-b border-[#2A2A2E]">'
        + c("AppHeader", size="compact", back=True) + "</div>" + c("TabStrip", active="connection")
        + '<div class="flex-1 overflow-y-auto p-[16px]">' + stack(c("ServerSection"), '<div class="h-px bg-[#2A2A2E] my-[8px]"></div>', c("SectionLabel", text="Connected timers"),
        '<div class="font-[\'Inter\'] text-[14px] text-[#A1A1AA]">No timers connected yet.</div>', '<div class="h-px bg-[#2A2A2E] my-[8px]"></div>',
        c("HowToConnect"), gap=16) + "</div></div>\n</body></html>\n"),
}

def cli(*args):
    out = subprocess.run(["npx", "--yes", "@superdesign/cli@latest", *args, "--generated-by", "claude-opus-5-5", "--json"], capture_output=True, text=True)
    if out.returncode != 0:
        sys.exit(f"import failed: {out.stderr or out.stdout}")
    return json.loads(out.stdout)

def main(only):
    for key, (title, w, h, build) in SCREENS.items():
        path = HERE / f"{key}.html"
        path.write_text(build())
        if only and key not in only:
            continue
        if key in DRAFTS:
            r = cli("import-design-draft", "--into", DRAFTS[key], "--html-file", str(path))
        else:
            r = cli("import-design-draft", "--project-id", PROJECT, "--title", title, "--width", str(w), "--height", str(h), "--html-file", str(path))
            DRAFTS[key] = r["draftId"]
            DRAFTS_FILE.write_text(json.dumps(DRAFTS, indent=2) + "\n")
        warn = r.get("warnings") or []
        print(f"{key}: {DRAFTS[key]} v{r.get('version', '1')}" + (f" warnings: {[x['code'] for x in warn]}" if warn else ""))

if __name__ == "__main__":
    main(set(sys.argv[1:]))
