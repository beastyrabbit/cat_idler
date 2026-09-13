#!/usr/bin/env python3
"""Build a self-contained Schaffa page for the Glass Clearing UI guide.
Reads: schaffa-uploads.tsv (name<TAB>json with publicUrl), schaffa-assets.json (data URLs),
mockups/glass.css, mockups/icons.js. Writes: schaffa-guide.html (no scripts, inline CSS/SVG)."""
import json, re, os, html
D = os.path.dirname(os.path.abspath(__file__))
urls = {}
tsv = os.path.join(D, 'schaffa-uploads.tsv')
if os.path.exists(tsv):
    for line in open(tsv):
        if '\t' not in line: continue
        n, js = line.rstrip('\n').split('\t', 1)
        try: u = json.loads(js).get('publicUrl', '')
        except Exception: u = ''
        if u.startswith('https://schaffa.dev/'): urls[n] = u
A = json.load(open(os.path.join(D, 'schaffa-assets.json')))
def img(n): return urls.get(n, '')
# icons
src = open(os.path.join(D, 'mockups/icons.js')).read()
paths = dict(re.findall(r"^\s*(\w+): '(.*)',\s*$", src, re.M))
def ic(name, cls=''):
    return f'<svg class="ic {cls}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{paths[name]}</svg>'
# css from the mockup stylesheet, minus body/canvas rules and local image paths
css = open(os.path.join(D, 'mockups/glass.css')).read()
css = re.sub(r'\nbody\{[^}]*\}', '', css)
css = re.sub(r'\ncanvas\{[^}]*\}', '', css)
css = css.replace("url(../../../public/images/cats/cat-sheet.png) -6px 4px/2048px 128px no-repeat", f"url({A['mallow']}) 0 0/52px 52px no-repeat")
css = css.replace('svg.lead{position:absolute;left:0;top:0;z-index:5;pointer-events:none}', 'svg.lead{position:absolute;left:0;top:0;z-index:5;pointer-events:none}')
page_css = css + '''
html{color-scheme:dark}body{margin:0;background:#0f1713;color:var(--ink);font:15px/1.55 Manrope,"Segoe UI",system-ui,-apple-system,sans-serif;padding:0 0 80px}
main{max-width:1180px;margin:0 auto;padding:0 18px}
header.h{max-width:1180px;margin:0 auto;padding:36px 18px 18px;border-bottom:1px solid var(--line)}
header.h h1{margin:0;font:italic 400 44px/1.05 "Instrument Serif",Georgia,serif}
header.h p.lede{max-width:80ch;font-size:16px;color:var(--dim);margin:10px 0 0}
.meta{display:flex;flex-wrap:wrap;gap:6px;margin:14px 0 0}.meta span{border:1px solid var(--edge);border-radius:999px;padding:3px 10px;font-size:12px;font-weight:700;color:var(--dim)}
nav.toc{border-bottom:1px solid var(--line)}nav.toc div{max-width:1180px;margin:0 auto;padding:10px 18px;display:flex;flex-wrap:wrap;gap:4px 12px;font-size:12px;font-weight:700}nav.toc a{color:var(--dim);text-decoration:none}
h2{font:800 26px/1.15 Manrope,sans-serif;margin:56px 0 10px}h2 small{display:block;font:700 11px Manrope,sans-serif;letter-spacing:.16em;text-transform:uppercase;color:var(--sun);margin-bottom:6px}
h3{font:800 18px/1.2 Manrope,sans-serif;margin:32px 0 8px}h4{font:800 12px Manrope,sans-serif;letter-spacing:.1em;text-transform:uppercase;color:var(--dim);margin:20px 0 6px}
p{max-width:86ch;margin:8px 0}p.note{border-left:3px solid var(--sun);padding:8px 14px;background:rgba(255,209,102,.07);border-radius:0 10px 10px 0;max-width:none}
ul,ol{max-width:86ch;padding-left:22px}li{margin:4px 0}
code{font:600 12.5px/1.4 ui-monospace,Menlo,monospace;background:rgba(255,255,255,.07);padding:1px 6px;border-radius:5px;color:#ffe9a6}
pre{background:#0a110d;border:1px solid var(--line);border-radius:12px;padding:14px 16px;overflow:auto;font:500 12.5px/1.5 ui-monospace,Menlo,monospace;color:#d9e2d5}
.tw{overflow-x:auto;margin:10px 0 18px}table.spec{width:100%;min-width:640px;border-collapse:collapse;font-size:13px}
table.spec th{text-align:left;font:800 10.5px Manrope,sans-serif;letter-spacing:.12em;text-transform:uppercase;color:var(--dim);padding:8px 10px;border-bottom:1px solid var(--edge)}
table.spec td{padding:8px 10px;border-bottom:1px solid var(--line);vertical-align:top}table.spec td:first-child{font-weight:800;white-space:nowrap}
.sw{display:inline-block;width:18px;height:18px;border-radius:5px;vertical-align:-4px;margin-right:8px;border:1px solid rgba(255,255,255,.15)}
.sw-wrap{overflow-x:auto;margin:14px 0 8px}.stage{position:relative;width:1140px;height:300px;border-radius:14px;overflow:hidden;background:#4f7f3a;border:1px solid var(--line)}
.stage>img.bg{position:absolute;left:-120px;top:-60px;width:1920px;height:auto;max-width:none}
.stage.tall{height:640px}.stage.mid{height:420px}.stage.short{height:200px}
.cap{font-size:12.5px;color:var(--dim);margin:0 0 18px}.cap b{color:var(--ink)}
.two{display:grid;grid-template-columns:1fr 1fr;gap:22px}@media(max-width:800px){.two{grid-template-columns:1fr}}
figure{margin:14px 0 26px}figure img{width:100%;height:auto;display:block;border-radius:12px;border:1px solid var(--line)}figcaption{font-size:12.5px;color:var(--dim);margin-top:8px}
.anat{position:relative}.anat img{width:100%;height:auto;display:block;border-radius:12px;border:1px solid var(--line)}
.anat .mk{position:absolute;width:24px;height:24px;border-radius:50%;background:var(--sun);color:#1a1608;font:800 12px/24px Manrope,sans-serif;text-align:center;box-shadow:0 0 0 3px rgba(0,0,0,.5);transform:translate(-50%,-50%)}
.legend{display:grid;grid-template-columns:repeat(2,1fr);gap:4px 24px;font-size:13px;margin:12px 0 0}@media(max-width:700px){.legend{grid-template-columns:1fr}}.legend b{display:inline-block;width:22px;height:22px;border-radius:50%;background:var(--sun);color:#1a1608;font:800 12px/22px Manrope,sans-serif;text-align:center;margin-right:8px}
.type-row{display:grid;grid-template-columns:220px 1fr 200px;gap:16px;align-items:baseline;padding:10px 0;border-bottom:1px solid var(--line);font-size:13px}.type-row .s{color:var(--dim)}@media(max-width:700px){.type-row{grid-template-columns:1fr}}
.kbd{display:inline-block;border:1px solid var(--edge);border-radius:6px;padding:0 7px;font:700 11.5px Manrope,sans-serif;background:var(--fill);margin:0 2px}
.do-dont{display:grid;grid-template-columns:1fr 1fr;gap:18px}@media(max-width:800px){.do-dont{grid-template-columns:1fr}}.do-dont div{border:1px solid var(--line);border-radius:12px;padding:12px 16px;font-size:13.5px}.do-dont .do{border-color:rgba(126,211,122,.4)}.do-dont .dont{border-color:rgba(255,107,87,.4)}.do-dont h5{margin:0 0 6px;font:800 11px Manrope,sans-serif;letter-spacing:.12em;text-transform:uppercase}.do h5{color:var(--ok)}.dont h5{color:var(--bad)}
.gallery{display:grid;grid-template-columns:1fr 1fr;gap:18px}@media(max-width:800px){.gallery{grid-template-columns:1fr}}
.chk li{list-style:none;position:relative;padding-left:26px}.chk li:before{content:"";position:absolute;left:0;top:6px;width:14px;height:14px;border:1.5px solid var(--dim);border-radius:4px}
footer{max-width:1180px;margin:60px auto 0;padding:20px 18px;border-top:1px solid var(--line);color:var(--dim);font-size:12.5px}
.dock{transform:none;left:auto;right:auto;position:relative;display:inline-flex;margin:120px 0 0 300px}
.card .portrait{background-size:52px 52px}
'''
groups = [['Overview','Cats','Village'],['Build','Work','Stores'],['Research','Officers','Shrine'],['World','Defense','Trade','Routes'],['Events']]
dock = '<div class="dock g">' + '<span class="sep"></span>'.join(''.join(f'<div class="dk {"on" if n=="Overview" else ""} {"hov" if n=="Work" else ""}" data-l="{n}">{ic(n.lower())}{"<em>3</em>" if n=="Cats" else ""}</div>' for n in g) for g in groups) + '</div>'
chip = '<div class="chip g"><div><h1>The Commons</h1><small>Day 2 · 13:30 · 31 cats, 12 at work</small></div><div class="seg"><span>‖</span><span class="on">1×</span><span>4×</span><span>8×</span></div></div>'
def seal(state=''):
    dot = '' if state=='empty' else '<i></i>'
    return f'<div class="seal g {"open" if state=="open" else ""}" {"style=color:var(--dim)" if state=="empty" else ""}><span class="b">{ic("events")}{dot}</span>{"0" if state=="empty" else "4"}</div>'
pop = f'''<div class="pop g" style="top:76px">
 <div class="hd"><b>Needs attention</b><span>4 · most urgent first</span><span class="x">Esc</span></div>
 <div class="row"><span class="dot"></span><div><div class="t">Mallow and Hazel are thirsty</div><div class="why">No water reachable inside the walls · river is 9 tiles past the east gate</div></div><div class="acts"><span class="act hot">Send a water run</span><span class="pin">{ic("pin")}</span></div></div>
 <div class="row"><span class="dot w"></span><div><div class="t">Sawmill blocked · no logs delivered</div><div class="why">2 haul jobs waiting · no free hauler · Thistle is idle</div></div><div class="acts"><span class="act">Let Thistle haul</span><span class="pin">{ic("pin")}</span></div></div>
 <div class="row"><span class="dot i"></span><div><div class="t">Fox at the north gate</div><div class="why">Captain vacant, so defense is yours · Wren is ready</div></div><div class="acts"><span class="act">Rally Wren</span><span class="pin">{ic("pin")}</span></div></div></div>'''
pills = f'''<div class="res"><div class="pill g"><img src="{A['ic_food']}" alt=""><b>184</b><i>✓</i></div><div class="pill g"><img src="{A['ic_water']}" alt=""><b>72</b><i>✓</i></div><div class="pill g"><img src="{A['ic_logs']}" alt=""><b>96</b><i>✓</i></div><div class="pill g stale"><img src="{A['ic_stone']}" alt=""><b>44</b><i class="stale">6h</i></div><div class="pill g"><img src="{A['ic_refined']}" alt=""><b>320</b><span>res</span></div><div class="pill g"><img src="{A['ic_blessings']}" alt=""><b>18</b><span>bless</span></div></div>'''
mallow = '''<div class="hd"><div class="portrait"></div><div><b>Mallow</b><small>Hauler · adult · Den 2</small></div></div>
  <div class="now">Carrying <u>3 logs</u> to the <u>Sawmill</u> · 31 tiles, 52 s<br><span class="why">because Steward Hazel ordered a haul · then rest</span></div>
  <div class="rings"><div class="ring"><div style="background:conic-gradient(#ffd166 58%,rgba(255,255,255,.12) 0)"><span>58</span></div>hunger</div><div class="ring"><div style="background:conic-gradient(#ff6b57 24%,rgba(255,255,255,.12) 0)"><span>24</span></div>thirst</div><div class="ring"><div style="background:conic-gradient(#7ed37a 71%,rgba(255,255,255,.12) 0)"><span>71</span></div>rest</div><div class="ring"><div style="background:conic-gradient(#7ed37a 92%,rgba(255,255,255,.12) 0)"><span>92</span></div>health</div></div>
  <div class="acts"><span class="act hot">Water first</span><span class="act">Boost</span><span class="act">Control · Tab</span><span class="act q">More…</span></div>'''
sawmill = f'''<div class="hd"><div class="portrait bld"><img src="{A['b_woodworking']}" alt=""></div><div><b>Sawmill</b><small>Open station · 1 crew slot · built hour 19</small></div></div>
  <div class="now"><u>Bramble</u> is waiting for logs nobody is free to carry<br><span class="why">queue wants 6 lumber · 2 logs → 1 lumber</span></div>
  <div class="kv"><em>On site</em><b class="bad">0 of 4 logs</b><em>Output</em><b>Storehouse</b><em>Job 41</em><b>Mallow · 52 s</b><em>Job 42, 43</em><b class="bad">unclaimed</b></div>
  <div class="acts"><span class="act hot">Let Thistle haul</span><span class="act">Boost</span><span class="act q">Edit queue</span></div>'''
bg = lambda: f'<img class="bg" src="{img("world-bare")}" alt="">'
def fig(n, cap): return f'<figure><img src="{img(n)}" alt="{html.escape(cap)}"><figcaption>{cap}</figcaption></figure>'
def tw(t): return f'<div class="tw">{t}</div>'
tpl = open(os.path.join(D, 'schaffa-guide.template.html')).read()
out = tpl
for k, v in {'CSS': page_css, 'DOCK': dock, 'CHIP': chip, 'SEAL': seal(), 'SEAL_OPEN': seal('open'), 'SEAL_EMPTY': seal('empty'), 'POP': pop, 'PILLS': pills, 'MALLOW': mallow, 'SAWMILL': sawmill, 'BG': bg(), 'STONE_IMG': A['p_stone'], 'DEN_IMG': A['b_den']}.items():
    out = out.replace('{{' + k + '}}', v)
out = re.sub(r'\{\{IMG:([\w-]+)\}\}', lambda m: img(m.group(1)), out)
out = re.sub(r'\{\{IC:(\w+)\}\}', lambda m: ic(m.group(1)), out)
missing = re.findall(r'\{\{[^}]+\}\}', out)
dest = os.path.join(D, 'schaffa-guide.html'); open(dest, 'w').write(out)
print('wrote', dest, len(out.encode()) // 1024, 'KB', 'missing placeholders:', missing[:5], 'urls:', len(urls))
