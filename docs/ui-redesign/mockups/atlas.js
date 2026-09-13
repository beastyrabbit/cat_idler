/* Injects the common Field Atlas chrome. Call after icons.js + world.js. */
window.atlasChrome = function (o) {
  o = Object.assign({ tab: 'Overview', ledger: '', spread: '', foot: null, top: true, index: true, compass: true, hint: true }, o || {});
  const top = `<div class="top">
  <div class="title"><h1>The Commons</h1><p>A village of 31 cats · 12 at work · 8 studies underway · second day</p></div>
  <div class="tally">
    <div><b>184</b><span>Food</span><i>counted ¼ h ago</i></div>
    <div><b>72</b><span>Water</span><i>counted ¼ h ago</i></div>
    <div><b>96</b><span>Logs</span><i>counted ¼ h ago</i></div>
    <div><b class="est">≈44</b><span>Stone</span><i class="est">estimate · 6 h old</i></div>
    <div><b>320</b><span>Research</span><i class="est">exact</i></div>
    <div><b>18</b><span>Blessings</span><i class="est">exact</i></div>
  </div>
  <div class="clock"><b>Day 2, half past one</b><small>hour 37.5 · running <span class="speed"><span>‖</span><span class="on">1×</span><span>4×</span><span>8×</span></span></small></div>
</div>`;
  const index = `<div class="index">${NAV.map(n => `<div class="${n === o.tab ? 'on' : ''}">${n}${n == 'Cats' ? '<em>3</em>' : n == 'Work' ? '<em>1</em>' : ''}</div>`).join('')}</div>`;
  const foot = o.foot === null ? `<div class="foot"><div class="eyebrow">Needs attention · 4</div><div class="notes">
    <div class="note"><span class="wax">${icon('alert')}</span>Mallow and Hazel are thirsty and no water is reachable from the village.<a>Designate a well · send a water run to the river</a></div>
    <div class="note"><span class="wax">${icon('alert')}</span>The Sawmill is blocked: no logs have been delivered. Two haul jobs are waiting for a free hauler.<a>Assign a hauler · open Work</a></div>
    <div class="note"><span class="wax brass">${icon('pin')}</span>Two cats have no bed. Every Den is full.<a>Plan a Den — 24 logs, 8 stone</a></div>
    <div class="note"><span class="wax brass">${icon('pin')}</span>A fox is at the north gate. The Captain's office is vacant, so defense is yours.<a>Rally Wren</a></div>
  </div></div>` : o.foot;
  const compass = `<div class="compass"><canvas id="mini" width="176" height="120"></canvas><div class="vp"></div><div class="n">N</div></div>`;
  const hint = `<div class="hint">WASD or right-drag to pan · wheel to zoom · click to inspect · Tab to take the reins · saved 12 s ago</div>`;
  document.body.insertAdjacentHTML('beforeend', (o.top ? top : '') + (o.spread ? `<section class="spread">${o.spread}</section>` : '') + (o.index ? index : '') + (o.ledger ? `<aside class="ledger">${o.ledger}</aside>` : '') + foot + (o.compass ? compass : '') + (o.hint ? hint : ''));
  if (o.compass) renderWorld(document.getElementById('mini'), { scale: 0.27, ox: 0, oy: 8, annotate: false });
};
