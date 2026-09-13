/* Injects the Glass Clearing chrome. Load after icons.js + world.js. */
window.glassChrome = function (o) {
  o = Object.assign({ tab: 'Overview', sealOpen: false, sheet: '', sheetWide: false, card: null, mini: true, hint: true, res: true, dock: true, tl: true, mode: '' }, o || {});
  const tl = `<div class="tl"><div class="chip g"><div><h1>The Commons</h1><small>Day 2 · 13:30 · 31 cats, 12 at work</small></div><div class="seg"><span>‖</span><span class="on">1×</span><span>4×</span><span>8×</span></div></div>
    <div class="seal g ${o.sealOpen ? 'open' : ''}"><span class="b">${icon('events')}<i></i></span>4</div></div>`;
  const pop = o.sealOpen ? `<div class="pop g">
    <div class="hd"><b>Needs attention</b><span>4 · most urgent first</span><span class="x">Esc</span></div>
    <div class="row"><span class="dot"></span><div><div class="t">Mallow and Hazel are thirsty</div><div class="why">No water reachable inside the walls · river is 9 tiles past the east gate</div></div><div class="acts"><span class="act hot">Send a water run</span><span class="pin">${icon('pin')}</span></div></div>
    <div class="row"><span class="dot w"></span><div><div class="t">Sawmill blocked · no logs delivered</div><div class="why">2 haul jobs waiting · no free hauler · Thistle is idle</div></div><div class="acts"><span class="act">Let Thistle haul</span><span class="pin">${icon('pin')}</span></div></div>
    <div class="row"><span class="dot w"></span><div><div class="t">2 cats have no bed</div><div class="why">Every Den is full · next Den costs 24 logs, 8 stone</div></div><div class="acts"><span class="act">Plan a Den</span><span class="pin">${icon('pin')}</span></div></div>
    <div class="row"><span class="dot i"></span><div><div class="t">Fox at the north gate</div><div class="why">Captain vacant, so defense is yours · Wren is ready</div></div><div class="acts"><span class="act">Rally Wren</span><span class="pin">${icon('pin')}</span></div></div>
  </div>` : '';
  const res = `<div class="res">
    <div class="pill g"><img src="../../../public/images/game/icons/food.png"><b>184</b><i>✓</i></div>
    <div class="pill g"><img src="../../../public/images/game/icons/water.png"><b>72</b><i>✓</i></div>
    <div class="pill g"><img src="../../../public/images/game/icons/logs.png"><b>96</b><i>✓</i></div>
    <div class="pill g stale"><img src="../../../public/images/game/icons/stone.png"><b>44</b><i class="stale">6h</i></div>
    <div class="pill g"><img src="../../../public/images/game/icons/refined.png"><b>320</b><span>res</span></div>
    <div class="pill g"><img src="../../../public/images/game/icons/blessings.png"><b>18</b><span>bless</span></div></div>`;
  const groups = [['Overview', 'Cats', 'Village'], ['Build', 'Work', 'Stores'], ['Research', 'Officers', 'Shrine'], ['World', 'Defense', 'Trade', 'Routes'], ['Events']];
  const dock = `<div class="dock g">${groups.map(g => g.map(n => `<div class="dk ${n === o.tab ? 'on' : ''}" data-l="${n}">${icon(n.toLowerCase())}${n == 'Cats' ? '<em>3</em>' : ''}</div>`).join('')).join('<span class="sep"></span>')}</div>`;
  const sheet = o.sheet ? `<section class="sheet g2 ${o.sheetWide ? 'wide' : ''}">${o.sheet}</section>` : '';
  let card = '';
  if (o.card) {
    card = `<div class="card g" style="left:${o.card.x}px;top:${o.card.y}px;width:${o.card.w || 340}px">${o.card.html}</div>`;
    if (o.card.ax != null) card += `<svg class="lead" width="1920" height="1080"><path d="M${o.card.x} ${o.card.y + 60} C ${o.card.x - 40} ${o.card.y + 60}, ${o.card.ax + 40} ${o.card.ay}, ${o.card.ax} ${o.card.ay}" fill="none" stroke="#ffd166" stroke-width="2"/><circle cx="${o.card.ax}" cy="${o.card.ay}" r="4" fill="#ffd166"/></svg>`;
  }
  const mini = o.mini ? `<div class="mini g"><canvas id="mini" width="230" height="150"></canvas><div class="vp"></div></div>` : '';
  const hint = o.hint ? `<div class="hint">WASD pan · wheel zoom · click to inspect · Tab to control · saved 12 s ago</div>` : '';
  const mode = o.mode ? `<div class="mode g">${o.mode}</div>` : '';
  document.body.insertAdjacentHTML('beforeend', (o.tl ? tl : '') + (o.res ? res : '') + sheet + card + pop + (o.dock ? dock : '') + mini + hint + mode);
  if (o.mini) renderWorld(document.getElementById('mini'), { scale: 0.36, ox: -10, oy: -20, annotate: false });
};
window.needRings = function (ctx, px, py, scene, list) {
  for (const [n, v, c] of list) { const cat = scene.cats.find(k => k.name == n); const x = px(cat.x), y = py(cat.y) - 26;
    ctx.lineWidth = 4; ctx.strokeStyle = 'rgba(0,0,0,.45)'; ctx.beginPath(); ctx.arc(x, y, 34, 0, Math.PI * 2); ctx.stroke();
    ctx.strokeStyle = c; ctx.beginPath(); ctx.arc(x, y, 34, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * v); ctx.stroke(); }
};
window.mallowCard = `<div class="hd"><div class="portrait"></div><div><b>Mallow</b><small>Hauler · adult · Den 2</small></div></div>
  <div class="now">Carrying <u>3 logs</u> to the <u>Sawmill</u> · 31 tiles, 52 s<br><span class="why">because Steward Hazel ordered a haul · then rest</span></div>
  <div class="rings">
    <div class="ring"><div style="background:conic-gradient(#ffd166 58%,rgba(255,255,255,.12) 0)"><span>58</span></div>hunger</div>
    <div class="ring"><div style="background:conic-gradient(#ff6b57 24%,rgba(255,255,255,.12) 0)"><span>24</span></div>thirst</div>
    <div class="ring"><div style="background:conic-gradient(#7ed37a 71%,rgba(255,255,255,.12) 0)"><span>71</span></div>rest</div>
    <div class="ring"><div style="background:conic-gradient(#7ed37a 92%,rgba(255,255,255,.12) 0)"><span>92</span></div>health</div></div>
  <div class="acts"><span class="act hot">Water first</span><span class="act">Boost</span><span class="act">Control · Tab</span><span class="act q">More…</span></div>`;
