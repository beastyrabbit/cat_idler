/* Shared world scene for the GUI redesign mockups.
   Draws the same colony state for every direction using the tracked sprites
   under public/images so chrome is compared on identical ground. */
(function () {
  const IMG = '../../../public/images/';
  const T = 16;
  function rng(seed) { let s = seed >>> 0; return () => { s = (Math.imul(s, 1664525) + 1013904223) >>> 0; return s / 4294967296; }; }

  const SPRITES = {
    grass: 'game/terrain/grass.png', grass_var: 'game/terrain/grass_var.png', dirt: 'game/terrain/dirt.png',
    farmland: 'game/terrain/farmland.png', rocky: 'game/terrain/rocky.png', highland: 'game/terrain/highland.png',
    water: 'game/terrain/water.png', water_edge: 'game/terrain/water_edge.png',
    fl_r: 'game/terrain/flowers_red.png', fl_b: 'game/terrain/flowers_blue.png', fl_w: 'game/terrain/flowers_white.png',
    oak: 'game/nature/tree_oak.png', pine: 'game/nature/tree_pine.png', stump: 'game/nature/stump.png',
    road_h: 'game/infra/road_straight_h.png', road_v: 'game/infra/road_straight_v.png', road_c: 'game/infra/road_corner.png',
    road_t: 'game/infra/road_t.png', road_x: 'game/infra/road_cross.png', bridge: 'game/infra/bridge.png',
    wall: 'game/infra/palisade_topdown.png', gate_o: 'game/infra/gate_open.png', gate_c: 'game/infra/gate_closed.png',
    log_pile: 'game/props/log_pile.png', stone_pile: 'game/props/stone_pile.png', ore_pile: 'game/props/ore_pile.png',
    sack: 'game/props/sack.png', barrel: 'game/props/barrel.png', crate: 'game/props/crate.png', well: 'game/props/well.png',
    campfire: 'game/props/campfire.png', haystack: 'game/props/haystack.png', gold: 'game/props/gold_pile.png',
    soil: 'game/farm/soil.png', crop1: 'game/farm/crop_sprout.png', crop2: 'game/farm/crop_growing.png',
    crop3: 'game/farm/crop_mature.png', crop4: 'game/farm/crop_flowering.png', scarecrow: 'game/farm/scarecrow.png',
    fox: 'game/enemies/fox.png',
    den: 'game/buildings/den.png', storehouse: 'game/buildings/storehouse.png', wood_cutter: 'game/buildings/wood_cutter.png',
    woodworking: 'game/buildings/woodworking.png', research_hut: 'game/buildings/research_hut.png', smithy: 'game/buildings/smithy.png',
    mill: 'game/buildings/mill.png', barracks: 'game/buildings/barracks.png', shrine: 'game/buildings/shrine.png',
    cats: 'cats/cat-sheet.png', hat_a: 'cats/hat-architect.png', hat_w: 'cats/hat-warrior.png', hat_h: 'cats/hat-hunter.png',
    ic_logs: 'game/icons/logs.png', ic_water: 'game/icons/water.png', ic_ore: 'game/icons/ore.png', ic_grain: 'game/icons/grain.png',
    ic_stone: 'game/icons/stone.png', ic_food: 'game/icons/food.png',
  };

  // Village geometry (tile coordinates, inclusive).
  const WALL = { x0: 7, y0: 3, x1: 32, y1: 19 };
  const ROADS_V = [8, 13, 25, 31];
  const ROADS_H = [4, 11, 18];
  const GATES = [[19, 3], [20, 3], [19, 19], [20, 19], [7, 11], [32, 11]];
  const BUILDINGS = [
    ['den', 9, 5], ['wood_cutter', 14, 5], ['research_hut', 21, 5], ['storehouse', 26, 5],
    ['den', 9, 12], ['den', 9, 15], ['woodworking', 14, 14], ['smithy', 21, 14], ['mill', 26, 12], ['barracks', 26, 15],
    ['shrine', 18, 10],
  ];
  const PROPS = [
    ['log_pile', 29, 5], ['log_pile', 29, 6], ['log_pile', 30, 5], ['stone_pile', 29, 7], ['stone_pile', 30, 7],
    ['sack', 26, 8], ['sack', 27, 8], ['barrel', 28, 8], ['crate', 29, 8], ['crate', 30, 8],
    ['well', 23, 9], ['campfire', 15, 9], ['haystack', 11, 9], ['haystack', 12, 9],
    ['ore_pile', 23, 13], ['ore_pile', 24, 13], ['stone_pile', 2, 2], ['ore_pile', 3, 1], ['stump', 5, 20], ['stump', 34, 6],
    ['scarecrow', 15, 20],
  ];

  const scene = {
    cats: [
      { name: 'Mallow', x: 28.5, y: 9.3, cell: 12, tint: 'none', carry: 'ic_logs', selected: true },
      { name: 'Hazel', x: 19.5, y: 7.5, cell: 0, tint: 'gray', hat: 'hat_a' },
      { name: 'Pip', x: 27.5, y: 7.6, cell: 8, tint: 'tux', hat: 'hat_h' },
      { name: 'Bramble', x: 16.6, y: 17.3, cell: 4, tint: 'none', wait: true },
      { name: 'Clover', x: 10.4, y: 8.3, cell: 0, tint: 'white' },
      { name: 'kitten', x: 11.3, y: 8.6, cell: 1, tint: 'white', kitten: true },
      { name: 'Sorrel', x: 33.6, y: 11.5, cell: 12, tint: 'gray', carry: 'ic_water' },
      { name: 'Juniper', x: 13.5, y: 21.4, cell: 24, tint: 'black' },
      { name: 'Moss', x: 22.4, y: 12.4, cell: 20, tint: 'none', carry: 'ic_ore' },
      { name: 'Fennel', x: 11.6, y: 16.5, cell: 0, tint: 'tux', asleep: true },
      { name: 'Wren', x: 19.5, y: 5.2, cell: 4, tint: 'black', hat: 'hat_w' },
      { name: 'Thistle', x: 23.5, y: 10.6, cell: 16, tint: 'gray' },
      { name: 'kitten', x: 10.2, y: 17.3, cell: 5, tint: 'none', kitten: true },
      { name: 'Rook', x: 31.4, y: 15.5, cell: 28, tint: 'none', carry: 'ic_grain' },
    ],
    threats: [{ kind: 'fox', x: 21.5, y: 0.7 }],
    // Mallow's authoritative route to the Sawmill entrance.
    path: [[28.5, 9.5], [28.5, 11.5], [21.5, 11.5], [21.5, 13.5], [19.5, 13.5], [19.5, 18.5], [15.5, 18.5], [15.5, 17.4]],
    destination: [15.5, 17.4],
  };

  function loadAll() {
    const out = {};
    return Promise.all(Object.entries(SPRITES).map(([k, p]) => new Promise((res) => {
      const im = new Image(); im.onload = () => { out[k] = im; res(); }; im.onerror = () => { out[k] = null; res(); }; im.src = IMG + p;
    }))).then(() => out);
  }

  function tint(t) {
    return { none: 'none', gray: 'saturate(0) brightness(0.92)', black: 'saturate(0.3) brightness(0.42)', white: 'saturate(0) brightness(1.55) contrast(0.85)', tux: 'saturate(0.35) brightness(0.62)' }[t] || 'none';
  }

  function roadSprite(x, y) {
    const v = ROADS_V.includes(x), h = ROADS_H.includes(y);
    const inV = y >= WALL.y0 + 1 && y <= WALL.y1 - 1, inH = x >= WALL.x0 + 1 && x <= WALL.x1 - 1;
    return { v: v && inV, h: h && inH };
  }

  function isPlaza(x, y) { return x >= 17 && x <= 21 && y >= 9 && y <= 13 && !(x >= 18 && x <= 20 && y >= 10 && y <= 12); }
  function isSpur(x, y) { return (x === 19 && ((y >= 5 && y <= 8) || (y >= 14 && y <= 17))) || (y === 11 && ((x >= 9 && x <= 16) || (x >= 22 && x <= 30))); }
  function isRoad(x, y) { const r = roadSprite(x, y); return r.v || r.h || isPlaza(x, y) || isSpur(x, y) || (y === 11 && x >= 33 && x <= 34); }
  function isWall(x, y) { return (x === WALL.x0 || x === WALL.x1) && y >= WALL.y0 && y <= WALL.y1 || (y === WALL.y0 || y === WALL.y1) && x >= WALL.x0 && x <= WALL.x1; }
  function isGate(x, y) { return GATES.some(([gx, gy]) => gx === x && gy === y); }
  function inside(x, y) { return x > WALL.x0 && x < WALL.x1 && y > WALL.y0 && y < WALL.y1; }
  function isFarm(x, y) { return y >= 20 && y <= 22 && x >= 10 && x <= 20; }
  function isRiver(x) { return x >= 36; }
  function isBuilding(x, y) { return BUILDINGS.some(([, bx, by]) => x >= bx && x < bx + 3 && y >= by && y < by + 3); }
  function wallDist(x, y) { const dx = Math.max(WALL.x0 - x, 0, x - WALL.x1), dy = Math.max(WALL.y0 - y, 0, y - WALL.y1); return Math.sqrt(dx * dx + dy * dy); }

  window.renderWorld = function (canvas, opts) {
    opts = Object.assign({ scale: 3, ox: 0, oy: 0, cols: 41, rows: 24, fog: true, annotate: true,
      pathColor: '#ffe9a6', selColor: '#ffe9a6', destColor: '#ffe9a6', needColor: '#ff7a5c', labels: false, labelFont: '600 12px sans-serif', labelColor: '#fff', dim: 0 }, opts || {});
    const S = opts.scale, P = T * S;
    const ctx = canvas.getContext('2d');
    return loadAll().then((im) => {
      ctx.imageSmoothingEnabled = false;
      const draw = (k, x, y, w, h) => { const s = im[k]; if (s) ctx.drawImage(s, Math.round(opts.ox + x * P), Math.round(opts.oy + y * P), (w || 1) * P, (h || 1) * P); };
      const r = rng(20260913);
      // Terrain layer
      for (let y = 0; y < opts.rows; y++) for (let x = 0; x < opts.cols; x++) {
        const q = r();
        if (isRiver(x)) { draw('water', x, y); continue; }
        if (x === 35) { draw('water_edge', x, y); continue; }
        if (x <= 5 && y <= 3) { draw('rocky', x, y); continue; }
        if (isFarm(x, y)) { draw('soil', x, y); continue; }
        draw(q < 0.18 ? 'grass_var' : 'grass', x, y);
        if (!inside(x, y) && q > 0.93) draw(['fl_r', 'fl_b', 'fl_w'][Math.floor(r() * 3)], x, y);
      }
      // Roads
      for (let y = 0; y < opts.rows; y++) for (let x = 0; x < opts.cols; x++) {
        if (!isRoad(x, y)) continue;
        const n = isRoad(x, y - 1), s = isRoad(x, y + 1), w = isRoad(x - 1, y), e = isRoad(x + 1, y);
        const c = n + s + w + e;
        if (x === 35 && y === 11) { draw('bridge', x, y); continue; }
        if (c >= 4) draw('road_x', x, y); else if (c === 3) draw('road_t', x, y); else if ((n || s) && (w || e)) draw('road_c', x, y); else if (n || s) draw('road_v', x, y); else draw('road_h', x, y);
      }
      // Farms
      const stages = ['crop1', 'crop2', 'crop3', 'crop4'];
      for (let y = 20; y <= 22; y++) for (let x = 10; x <= 20; x++) if (!(x === 15 && y === 20)) draw(stages[Math.min(3, Math.floor((x - 10) / 3))], x, y);
      // Walls & gates
      for (let y = 0; y < opts.rows; y++) for (let x = 0; x < opts.cols; x++) { if (isGate(x, y)) draw('gate_o', x, y); else if (isWall(x, y)) draw('wall', x, y); }
      // Buildings
      for (const [k, x, y] of BUILDINGS) draw(k, x, y, 3, 3);
      // Props
      for (const [k, x, y] of PROPS) { if (k === 'well' || k === 'scarecrow') draw(k, x, y - 1, 1, 2); else draw(k, x, y); }
      // Trees outside the wall
      const r2 = rng(7);
      for (let y = 0; y < opts.rows; y++) for (let x = 0; x < opts.cols; x++) {
        if (inside(x, y) || isWall(x, y) || isRoad(x, y) || isFarm(x, y) || isRiver(x) || x === 35 || (x <= 5 && y <= 3)) continue;
        if (isFarm(x, y + 1) || isFarm(x, y - 1)) continue;
        const d = wallDist(x, y), q = r2();
        const dens = d <= 1 ? 0.05 : d <= 2 ? 0.25 : d <= 3 ? 0.5 : 0.78;
        if (q < dens) draw(q < dens * 0.4 ? 'pine' : 'oak', x, y);
      }
      // Fog of war
      if (opts.fog) for (let y = 0; y < opts.rows; y++) for (let x = 0; x < opts.cols; x++) {
        const d = wallDist(x, y); if (d < 3) continue;
        const a = Math.min(0.88, (d - 2.6) * 0.14);
        ctx.fillStyle = `rgba(5,12,9,${a})`; ctx.fillRect(opts.ox + x * P, opts.oy + y * P, P, P);
      }
      // Annotations under actors: path + destination + selection
      const px = (v) => opts.ox + v * P;
      if (opts.annotate) {
        ctx.save(); ctx.lineCap = 'round'; ctx.lineJoin = 'round';
        ctx.strokeStyle = 'rgba(0,0,0,0.45)'; ctx.lineWidth = 7; ctx.setLineDash([]);
        ctx.beginPath(); scene.path.forEach(([x, y], i) => i ? ctx.lineTo(px(x), opts.oy + y * P) : ctx.moveTo(px(x), opts.oy + y * P)); ctx.stroke();
        ctx.strokeStyle = opts.pathColor; ctx.lineWidth = 3; ctx.setLineDash([10, 9]);
        ctx.beginPath(); scene.path.forEach(([x, y], i) => i ? ctx.lineTo(px(x), opts.oy + y * P) : ctx.moveTo(px(x), opts.oy + y * P)); ctx.stroke();
        ctx.setLineDash([]);
        const [dx, dy] = scene.destination;
        ctx.strokeStyle = opts.destColor; ctx.lineWidth = 3; ctx.beginPath(); ctx.arc(px(dx), opts.oy + dy * P, P * 0.55, 0, Math.PI * 2); ctx.stroke();
        ctx.beginPath(); ctx.arc(px(dx), opts.oy + dy * P, P * 0.18, 0, Math.PI * 2); ctx.fillStyle = opts.destColor; ctx.fill();
        ctx.restore();
      }
      // Threats
      for (const t of scene.threats) draw(t.kind, t.x - 0.5, t.y - 0.5);
      // Cats
      const catPx = 2 * S * 32 / 3; // 2x at scale 3 = 64px
      const sorted = scene.cats.slice().sort((a, b) => a.y - b.y);
      for (const c of sorted) {
        const size = c.kitten ? catPx * 0.62 : catPx;
        const cx = px(c.x), cy = opts.oy + c.y * P;
        if (c.selected && opts.annotate) {
          ctx.save(); ctx.strokeStyle = opts.selColor; ctx.lineWidth = 3; ctx.beginPath(); ctx.ellipse(cx, cy + 4, P * 0.62, P * 0.32, 0, 0, Math.PI * 2); ctx.stroke(); ctx.restore();
        }
        ctx.save(); ctx.filter = tint(c.tint);
        if (im.cats) ctx.drawImage(im.cats, c.cell * 32, 0, 32, 32, cx - size / 2, cy - size * 0.78, size, size);
        ctx.filter = 'none';
        if (c.hat && im[c.hat]) ctx.drawImage(im[c.hat], 0, 0, 32, 32, cx - size / 2, cy - size * 0.78, size, size);
        ctx.restore();
        if (c.asleep) { ctx.save(); ctx.font = `700 ${Math.round(P * 0.42)}px sans-serif`; ctx.fillStyle = 'rgba(255,255,255,0.85)'; ctx.fillText('z', cx + size * 0.3, cy - size * 0.7); ctx.restore(); }
        if (c.carry && im[c.carry]) {
          const bs = P * 0.5; const bx = cx + size * 0.18, by = cy - size * 0.95;
          ctx.save(); ctx.fillStyle = 'rgba(20,24,20,0.85)'; ctx.beginPath(); ctx.arc(bx + bs / 2, by + bs / 2, bs * 0.62, 0, Math.PI * 2); ctx.fill();
          ctx.drawImage(im[c.carry], bx, by, bs, bs); ctx.restore();
        }
        if (c.wait) { ctx.save(); ctx.font = `700 ${Math.round(P * 0.5)}px sans-serif`; ctx.fillStyle = '#ffd36b'; ctx.fillText('!', cx + size * 0.32, cy - size * 0.7); ctx.restore(); }
        if (c.selected && opts.annotate && opts.needColor) {
          // thirst tick above the selected cat
          ctx.save(); ctx.fillStyle = opts.needColor; ctx.beginPath(); ctx.arc(cx - size * 0.3, cy - size * 0.95, P * 0.16, 0, Math.PI * 2); ctx.fill(); ctx.restore();
        }
        if (opts.labels && !c.kitten) {
          ctx.save(); ctx.font = opts.labelFont; ctx.textAlign = 'center'; ctx.fillStyle = 'rgba(0,0,0,0.6)';
          const w = ctx.measureText(c.name).width + 10; ctx.fillRect(cx - w / 2, cy + 8, w, 16); ctx.fillStyle = opts.labelColor; ctx.fillText(c.name, cx, cy + 20); ctx.restore();
        }
      }
      if (opts.dim > 0) { ctx.fillStyle = `rgba(4,10,8,${opts.dim})`; ctx.fillRect(0, 0, canvas.width, canvas.height); }
      canvas.dispatchEvent(new Event('world-ready'));
      return { P, px, py: (v) => opts.oy + v * P, scene };
    });
  };
  window.worldScene = scene;
})();
