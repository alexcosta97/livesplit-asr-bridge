#!/usr/bin/env python3
"""Builds the brand kit from the drawings below: logo SVGs, the wordmark (text
converted to outlines), app icons (PNG, ICO, ICNS) and tokens.json.

Requirements: Python 3 with fonttools and Pillow, and rsvg-convert (librsvg).
Run from anywhere:  python3 assets/brand/build.py
"""
import json, pathlib, subprocess
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parent
LOGO, ICONS = ROOT / "logo", ROOT / "icons"
LOGO.mkdir(exist_ok=True); ICONS.mkdir(exist_ok=True)

# Colour tokens (the same values as .superdesign/design-system.md).
C = {
    "orange": "#FF4D00", "orange-hover": "#FF6A26",
    "ink": "#0B0B0C", "panel": "#151517", "raised": "#1E1E21",
    "border": "#2A2A2E", "border-strong": "#3A3A40",
    "paper": "#F5F5F4", "text-secondary": "#A1A1AA", "text-muted": "#6B6B73",
    "ok": "#22E07A", "error": "#FF3B3B", "warning": "#FFC233", "info": "#7DD3FC",
}

# The mark: the game PC (paper node) and the timer (orange node), joined by a
# link carrying a split. Two drawings: full detail, and a heavier one for
# 32 px and below so the link and split mark survive.
def mark(fg, small=False):
    if small:
        return (f'<path d="M232 628 Q512 132 792 628" fill="none" stroke="{fg}" stroke-width="136" stroke-linecap="round"/>'
                f'<rect x="452" y="200" width="120" height="300" rx="16" fill="{C["orange"]}"/>'
                f'<circle cx="232" cy="716" r="164" fill="{fg}"/><circle cx="792" cy="716" r="164" fill="{C["orange"]}"/>')
    return (f'<path d="M256 600 Q512 176 768 600" fill="none" stroke="{fg}" stroke-width="72" stroke-linecap="round"/>'
            f'<rect x="484" y="276" width="56" height="200" rx="8" fill="{C["orange"]}"/>'
            f'<circle cx="256" cy="688" r="104" fill="{fg}"/><circle cx="768" cy="688" r="104" fill="{C["orange"]}"/>')

def tile(small=False):
    t = f'<rect width="1024" height="1024" rx="224" fill="{C["ink"]}"/>'
    if not small:  # the hairline edge disappears at small sizes, so it's left out there
        t += f'<rect x="8" y="8" width="1008" height="1008" rx="216" fill="none" stroke="{C["border"]}" stroke-width="16"/>'
    return t

def svg(body, w=1024, h=1024, vb=None):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb or f"0 0 {w} {h}"}" width="{w}" height="{h}">{body}</svg>\n'

icon = tile() + mark(C["paper"])
icon_small = tile(True) + mark(C["paper"], True)
(LOGO / "icon.svg").write_text(svg(icon))
(LOGO / "icon-small.svg").write_text(svg(icon_small))
# The mark on its own, cropped to its drawing, for headers and inline use.
(LOGO / "mark-on-dark.svg").write_text(svg(mark(C["paper"]), 784, 580, "120 244 784 580"))
(LOGO / "mark-on-light.svg").write_text(svg(mark(C["ink"]), 784, 580, "120 244 784 580"))
# macOS draws app icons as a 824 px body on a 1024 px canvas. Its 16 and 32 px
# slots (and their @2x, up to 64 px) use the padded small drawing.
def macos(body):
    return svg(f'<g transform="translate(100 100) scale({824/1024})">{body}</g>')
(LOGO / "icon-macos.svg").write_text(macos(icon))
(LOGO / "icon-macos-small.svg").write_text(macos(icon_small))

# Wordmark: icon + "LIVESPLIT ONE / ASR BRIDGE" in Archivo Black, as outlines.
font = TTFont(ROOT / "fonts" / "ArchivoBlack-Regular.ttf")
glyphs, cmap, upm = font.getGlyphSet(), font.getBestCmap(), font["head"].unitsPerEm
def text_path(s, size, x, baseline, tracking=-0.02):
    scale, out = size / upm, []
    for ch in s:
        g = cmap[ord(ch)]
        pen = SVGPathPen(glyphs)
        glyphs[g].draw(TransformPen(pen, (scale, 0, 0, -scale, x, baseline)))
        if pen.getCommands():
            out.append(pen.getCommands())
        x += glyphs[g].width * scale + tracking * size
    return " ".join(out), x

def wordmark(text_fg):
    size, lh = 88, 92
    l1, w1 = text_path("LIVESPLIT ONE", size, 0, size * 0.72)
    a, wa = text_path("ASR ", size, 0, size * 0.72 + lh)
    b, w2 = text_path("BRIDGE", size, wa, size * 0.72 + lh)
    width = max(w1, w2)
    icon_px, gap = 216, 40
    body = (f'<g transform="scale({icon_px/1024})">{icon}</g>'
            f'<g transform="translate({icon_px + gap} 20)"><path d="{l1}" fill="{text_fg}"/>'
            f'<path d="{a}" fill="{C["orange"]}"/><path d="{b}" fill="{text_fg}"/></g>')
    return svg(body, round(icon_px + gap + width + 24), icon_px)

(LOGO / "wordmark-on-dark.svg").write_text(wordmark(C["paper"]))
(LOGO / "wordmark-on-light.svg").write_text(wordmark(C["ink"]))

# Icon exports. 32 px and below use the small drawing.
def png(src, px, dst):
    subprocess.run(["rsvg-convert", "-w", str(px), "-h", str(px), str(src), "-o", str(dst)], check=True)
    return dst
sizes = [16, 24, 32, 48, 64, 128, 256, 512, 1024]
for px in sizes:
    png(LOGO / ("icon-small.svg" if px <= 32 else "icon.svg"), px, ICONS / f"icon-{px}.png")
ico = [Image.open(ICONS / f"icon-{px}.png") for px in (16, 24, 32, 48, 64, 128, 256)]
ico[-1].save(ICONS / "icon.ico", format="ICO", sizes=[i.size for i in ico], append_images=ico[:-1])
mac_imgs = {}
for px in (16, 32, 64, 128, 256, 512, 1024):
    mac_imgs[px] = Image.open(png(LOGO / ("icon-macos-small.svg" if px <= 64 else "icon-macos.svg"), px, ROOT / f".mac-{px}.png"))
mac_imgs[1024].save(ICONS / "icon.icns", format="ICNS", append_images=[mac_imgs[p] for p in (16, 32, 64, 128, 256, 512)])
for p in ROOT.glob(".mac-*.png"):
    p.unlink()

# Design tokens, for the app theme, a website and store graphics.
tokens = {
    "$comment": "Brand tokens for LiveSplit One ASR Bridge. Source: .superdesign/design-system.md",
    "color": {
        "background": {"window": C["ink"], "panel": C["panel"], "raised": C["raised"]},
        "border": {"default": C["border"], "strong": C["border-strong"]},
        "text": {"primary": C["paper"], "secondary": C["text-secondary"], "muted": C["text-muted"]},
        "accent": {"default": C["orange"], "hover": C["orange-hover"]},
        "status": {"ok": C["ok"], "waiting": C["text-secondary"], "error": C["error"], "warning": C["warning"]},
        "log": {"auto-splitter": C["orange"], "connection": C["info"], "app": C["text-secondary"], "error": C["error"]},
    },
    "font": {
        "display": {"family": "Archivo Black", "file": "fonts/ArchivoBlack-Regular.ttf", "case": "uppercase", "tracking": "-0.02em"},
        "mono": {"family": "Space Mono", "files": ["fonts/SpaceMono-Regular.ttf", "fonts/SpaceMono-Bold.ttf"]},
        "body": {"family": "Inter", "files": ["fonts/Inter-Regular.ttf", "fonts/Inter-SemiBold.ttf"]},
    },
    "size": {"status-word": 20, "status-word-compact": 30, "label": 11, "body": 14, "control": 13, "mono": 12},
    "space": {"unit": 8, "card-padding": 16, "card-gap": 12, "content-padding": 24},
    "radius": {"card": 4, "control": 4, "icon": "22%"},
    "stroke": {"border": 1, "status-edge": 3, "active-tab": 2},
}
(ROOT / "tokens.json").write_text(json.dumps(tokens, indent=2) + "\n")
print("brand kit built")
