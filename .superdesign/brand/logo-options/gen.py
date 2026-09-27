ORANGE="#FF4D00"; INK="#0B0B0C"; PAPER="#F5F5F4"; LINE="#2A2A2E"
TILE=f'<rect width="1024" height="1024" rx="224" fill="{INK}"/><rect x="8" y="8" width="1008" height="1008" rx="216" fill="none" stroke="{LINE}" stroke-width="16"/>'

# A. Split marker: a rising step (the split) crossed by the split line
def mark_a(fg=PAPER):
    return (f'<line x1="456" y1="232" x2="456" y2="792" stroke="{fg}" stroke-width="56"/>'
            f'<path d="M168 656 H456 V392 H856" fill="none" stroke="{ORANGE}" stroke-width="120" stroke-linejoin="miter"/>')
# B. Bridge: game node -> timer node, arc crossed by a split mark
def mark_b(fg=PAPER):
    return (f'<path d="M256 600 Q512 176 768 600" fill="none" stroke="{fg}" stroke-width="72" stroke-linecap="round"/>'
            f'<rect x="484" y="276" width="56" height="200" rx="8" fill="{ORANGE}"/>'
            f'<circle cx="256" cy="688" r="104" fill="{fg}"/>'
            f'<circle cx="768" cy="688" r="104" fill="{ORANGE}"/>')
# C. Stream-deck key: ASR on an orange key
def icon_c(small=False):
    txt = "A" if small else "ASR"
    size = 640 if small else 330
    y = 740 if small else 628
    return (f'<rect width="1024" height="1024" rx="224" fill="{ORANGE}"/>'
            f'<rect x="64" y="64" width="896" height="856" rx="176" fill="#FF6A26"/>'
            f'<text x="512" y="{y}" text-anchor="middle" font-family="Archivo Black" font-size="{size}" letter-spacing="-12" fill="{INK}">{txt}</text>')

def svg(inner, px, vb="0 0 1024 1024"):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb}" width="{px}" height="{px}">{inner}</svg>'

D={
 "a":dict(name="A · Split marker", idea="A rising step: the moment a split lands. The white line is the split marker, the orange step is the run moving on.",
          icon=lambda s=False: TILE+mark_a(), mark=lambda fg: mark_a(fg)),
 "b":dict(name="B · Bridge", idea="Two machines joined by one link: the game PC (white) and the timer (orange), with a split mark riding the connection.",
          icon=lambda s=False: TILE+mark_b(), mark=lambda fg: mark_b(fg)),
 "c":dict(name="C · Stream-deck key", idea="The app as a key on a stream deck: ASR in the UI's display type. Below 32 px the key keeps only the A.",
          icon=lambda s=False: icon_c(s), mark=lambda fg: icon_c(False)),
}
HEAD='''<!doctype html><html><head><meta charset="utf-8"><title>{t}</title>
<link href="https://fonts.googleapis.com/css2?family=Archivo+Black&family=Inter:wght@400;600&family=Space+Mono&display=swap" rel="stylesheet">
<style>
*{{box-sizing:border-box;margin:0}} body{{background:#0B0B0C}} .root{{width:1440px;height:900px;background:#0B0B0C;color:#F5F5F4;font-family:Inter,sans-serif;display:grid;grid-template-columns:1fr 1fr}}
.half{{padding:48px;display:flex;flex-direction:column;gap:28px}} .light{{background:#F5F5F4;color:#0B0B0C}}
.label{{font-family:'Space Mono',monospace;font-size:11px;letter-spacing:.08em;text-transform:uppercase;color:#6B6B73}}
h1{{font-family:'Archivo Black';font-size:28px;text-transform:uppercase;letter-spacing:-.02em}}
.idea{{font-size:14px;color:#A1A1AA;max-width:560px;line-height:1.5}} .light .idea{{color:#52525B}}
.row{{display:flex;align-items:flex-end;gap:28px}} .sz{{display:flex;flex-direction:column;align-items:center;gap:8px}}
.wm{{display:flex;align-items:center;gap:20px}} .wm .t{{font-family:'Archivo Black';font-size:30px;line-height:1.02;text-transform:uppercase;letter-spacing:-.02em}}
.wm .t span{{color:#FF4D00}} .bar{{display:flex;gap:10px;align-items:center;background:#151517;border:1px solid #2A2A2E;border-radius:4px;padding:6px 10px;width:max-content}}
.light .bar{{background:#E4E4E7;border-color:#D4D4D8}} .dot{{width:16px;height:16px;border-radius:3px;background:#3A3A40}}
</style></head><body><div class="root">'''
def half(k,light):
    d=D[k]; fg = INK if light else PAPER
    cls="half light" if light else "half"
    sizes="".join(f'<div class="sz">{svg(d["icon"](px<32),px)}<div class="label">{px}</div></div>' for px in (128,64,32,16))
    top = (f'<div><div class="label">Direction</div><h1>{d["name"]}</h1></div><div class="idea">{d["idea"]}</div>' if not light
           else f'<div><div class="label">On light backgrounds</div><h1 style="visibility:hidden">x</h1></div><div class="idea">Same icon tile; the bare mark switches its white parts to ink.</div>')
    mark = svg(d["mark"](fg),96)
    return f'''<div class="{cls}">{top}
<div class="row">{svg(d["icon"](),256)}<div style="display:flex;flex-direction:column;gap:12px"><div class="label">Mark without the tile</div>{mark}</div></div>
<div><div class="label" style="margin-bottom:12px">App icon sizes (real pixels)</div><div class="row">{sizes}</div></div>
<div><div class="label" style="margin-bottom:12px">Taskbar</div><div class="bar"><div class="dot"></div>{svg(d["icon"](True),16)}<div class="dot"></div><div class="dot"></div></div></div>
<div><div class="label" style="margin-bottom:12px">Wordmark lockup</div><div class="wm">{svg(d["icon"](),64)}<div class="t">LiveSplit One<br><span>ASR</span> Bridge</div></div></div>
</div>'''
for k in D:
    open(f"logo-{k}.html","w").write(HEAD.format(t=f"Logo {D[k]['name']}")+half(k,False)+half(k,True)+"</div></body></html>")
    open(f"icon-{k}.svg","w").write(svg(D[k]["icon"](),1024))
print("done")
