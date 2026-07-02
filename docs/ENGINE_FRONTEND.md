# Map Rendering Engine — Analysis & Recommendation

_What should render the world map? Diagnosis of today's DOM renderer, evaluation of
alternatives, and a recommended path. Scope: `components/map/*`, `lib/game/isoProjection.ts`._

## 1. What we have today

The map is a **tree of absolutely-positioned React DOM nodes** inside one CSS-transformed
content plane (`components/ui/MapViewport.tsx`). Pure projection math lives in
`lib/game/isoProjection.ts` (a 2:1 iso diamond); the renderer components are thin wrappers
that turn tile/world coordinates into `left/top/zIndex` on `<img>` and `<div>` elements.

- **Content plane** (`isoContentSize`): 300×300 tiles → **76,800 × 38,784 px** at scale 1.
- **Chunk window** (`chunkWindow`): chunks −12..12 on each axis = **625 chunks × 144 tiles = 90,000 tiles**.
- **Layers**: `TileLayer` (1–2 `<img>` per tile, 256×512 each, plus fog `<div>`s and fence
  sprites), `BuildingLayer`, `CatLayer` (animated CSS sprite-sheet `<div>`s), `ZoneLayer`,
  `RaiderLayer`.
- **Camera**: hand-rolled pointer/pinch/wheel math sets `translate(tx,ty) scale(s)` on the
  plane. `will-change: transform`. Zoom range `minScale 0.08` … `maxScale 1.4`.
- **Culling**: `visibleChunksIso` inverts the four viewport corners to a tile rect, expands
  to chunks, clamps to the window. Each visible `ChunkView` `fetch()`es `/api/game/chunks`.

## 2. Why it breaks when zoomed far out (precise diagnosis)

The failure is the **zoomed-out overview**, and it is the compound of four things — culling
is what normally saves DOM maps, and at min zoom culling stops helping:

1. **The culler returns almost the entire window.** At `minScale = 0.08` the whole plane is
   only 76,800·0.08 ≈ 6,144 px wide. A 1920-px viewport inverts to ~24,000 content px of
   visible width (≈⅓ of the map), and because the iso diamond packs both world axes into that
   span, the visible **tile rect covers the large majority of the 625-chunk window** — plus a
   6-tile `tallPad` skirt. In practice min-zoom mounts on the order of **500–625 chunks ≈
   80,000–90,000 tiles**. (Even `initialScale 0.45` already mounts ~9×9 chunks ≈ 11k tiles.)

2. **DOM node explosion.** Each explored tile is 1–2 `<img>` (plus fence/marker nodes); fog
   tiles are a `<div>`. Min zoom therefore instantiates **~90,000–150,000 absolutely-positioned
   nodes**. Browsers do style recalc, layout, and paint across all of them; this is far past
   the few-thousand-node range where DOM stays interactive, and the main thread stalls for
   seconds. React must also reconcile that many memoized components on every view change.

3. **Request storm.** ~600 `ChunkView`s each fire a `fetch('/api/game/chunks')` at once —
   hundreds of concurrent requests through one better-sqlite3-backed route.

4. **Oversized compositing layer.** `will-change: transform` asks the browser to promote the
   content plane to a GPU layer at its **natural** 76,800 × 38,784 px. That exceeds
   `MAX_TEXTURE_SIZE` — commonly 16,384 on desktop, and **only ~50% of devices exceed 4,096**
   ([webgl2fundamentals](https://webgl2fundamentals.org/webgl/lessons/webgl-cross-platform-issues.html)).
   The layer cannot be cached as one texture, so panning re-rasterizes it; mobile GPUs (4,096
   cap) fare worst.

Net: the overview is the worst case on every axis, and elevation/cliffs (task #24) plus
"hundreds of animated sprites" will push even mid-zoom over the edge. This architecture has no
headroom.

## 3. Options evaluated

**Requirements to hold against:** isometric, 90k+ tiles with elevation coming, hundreds of
animated sprites, fog, 1 Hz sim + smooth interpolation, full-map overview → close-up zoom,
desktop + mobile, and a **React/Next shell where panels stay React DOM**.

| Option | Perf at our scale | React interop | Migration effort | Pixel-art | Verdict |
|---|---|---|---|---|---|
| **PixiJS v8** (`@pixi/react` v8) | GPU-batched sprites; culling API; 90k tiles + LOD is routine | Canvas sits behind React DOM panels; `@pixi/react` v8 is React-19-native | Medium — projection math reused as-is | `nearest` scale mode = crisp | **Recommended** |
| Phaser 4 | Fine, same WebGL core | Wants to own the loop/canvas; React is second-class | High — a game framework we don't need | OK | Overkill |
| Custom canvas2d | No GPU batching; 90k tiles + animated sprites at 60fps is exactly its wall | Manual | High — reinvent batching, cull, z-sort, LOD | Manual | A worse Pixi |
| DOM + aggressive LOD | Bake chunks → cached `<img>` when zoomed out cuts 90k nodes → 625 | Native | **Low (1 day)** | CSS blurs on scale | Bridge, not destination |
| WebGL tilemap libs (`@pixi/tilemap`) | Excellent for the ground plane | via Pixi | — | crisp | Folds into Pixi |

Notes with sources:

- **PixiJS v8** has an explicit, opt-in [culling API](https://www.richardfu.net/optimizing-rendering-with-pixijs-v8-a-deep-dive-into-the-new-culling-api/)
  (`cullable`, `cullArea`, `cullableChildren`) and [performance guidance](https://pixijs.com/8.x/guides/concepts/performance-tips)
  for large scenes. [`@pixi/react` v8](https://pixijs.com/blog/pixi-react-v8-live) is a ground-up
  rewrite "designed exclusively for React 19" (we're on React 19) with an `extend` API for small
  bundles. Pixi renders to a single `<canvas>`, so [React DOM panels overlay it](https://blog.logrocket.com/getting-started-pixijs-react-create-canvas/)
  — our HUD/aside/modals stay exactly as they are.
- **`@pixi/tilemap`** batches a tile grid into few draw calls (limits: ~16k tiles/layer,
  8 textures/layer — worked around with `CompositeTilemap`) — [docs](https://api.pixijs.io/@pixi/tilemap.html).
  Iso is handled by us (we already compute per-tile screen positions), so the rectangular-tilemap
  limitation doesn't bite; we can also just pool sprites.
- **`pixi-viewport`** ([v8-compatible](https://github.com/pixijs-userland/pixi-viewport)) gives
  `drag()`, `pinch()`, `wheel()`, `decelerate()`, and `clamp()` — replacing the hand-rolled
  `MapViewport` pointer math, including built-in mobile pinch.
- **Phaser vs Pixi**: Phaser is a *framework* (scenes, physics, audio, loop); Pixi is a
  *renderer* meant to drop into React/Vue ([comparison](https://generalistprogrammer.com/tutorials/phaser-vs-pixijs-renderer-comparison)).
  Our sim already runs server-side + worker, so we want a renderer, not a framework.

## 4. Recommendation: PixiJS v8 + `@pixi/react` v8 + `pixi-viewport`

**Why it fits:** WebGL sprite batching turns 90k tiles + hundreds of animated cats into a
handful of draw calls; the single canvas is one correctly-sized GPU surface (no
max-texture-size trap); `@pixi/react` v8 is React-19-native so panels stay React DOM; and
`nearest` scale mode keeps Kenney pixel art crisp at every zoom (better than CSS, which blurs).

**The projection math is fully reusable.** `isoProjection.ts` is pure and returns content-px
`left/top` — those map 1:1 to Pixi world coordinates. `tileToIso`, `tileDiamondCenter`,
`isoToTile`, `zIndexFor`, and `visibleChunksIso` are the renderer contract regardless of backend,
so the migration is layer-by-layer, not a rewrite.

### Incremental migration plan (biggest offender first)

Run Pixi and DOM **side by side** during migration: a Pixi `<canvas>` behind, remaining DOM
layers on top, both driven by the same `tx/ty/scale`. Ship each step independently.

1. **`TileLayer` → Pixi** (the 90k-node offender). Sprite-pool or `@pixi/tilemap` per chunk;
   set `sprite.zIndex = zIndexFor(...)` on a `sortableChildren` container; fog and roads as
   tinted sprites. This alone removes ~90% of the DOM and the oversized layer.
2. **`BuildingLayer` → Pixi** (static sprites, same projection).
3. **`CatLayer` → Pixi** `AnimatedSprite` from the existing sheet; interpolate 1 Hz positions
   in the ticker for smooth glide; `eventMode: 'static'` + `hitArea` for click-to-select
   (cheaper than 90k DOM event targets).
4. **`ZoneLayer` / `RaiderLayer` → Pixi** (thin).
5. **Swap `MapViewport` → `pixi-viewport`** for drag/pinch/wheel/clamp; delete the hand-rolled
   pointer math. Panels, HUD, and modals never move — they stay React DOM over the canvas.

### LOD strategy for the zoomed-out overview (the actual fix for §2)

Three bands keyed off `scale`, so the overview never touches per-tile work:

- **Close (`scale ≳ 0.4`)** — individual tile + object sprites, culled to the viewport.
- **Mid (`~0.15 … 0.4`)** — bake each 12×12 chunk **once** into a small `RenderTexture`
  (~512×256) and draw one sprite per chunk. 625 sprites instead of 90k. Rebake a chunk only
  when its tiles change (fog reveal, roads) — mirrors today's 60 s chunk TTL.
- **Overview (`scale ≲ 0.15`)** — a single **minimap aggregate**: one small quad/pixel per
  chunk tinted by dominant biome (reuse `TILE_COLORS`), composited into one ≤2048² texture.

Critical constraint: **never bake the whole map into one texture** — 76,800×38,784 dwarfs the
16,384 (and 4,096 on half of devices) `MAX_TEXTURE_SIZE`. LOD stays a grid of small textures or
one small minimap, which is what keeps the overview cheap on mobile.

### Effort & risk

- Medium effort; the pure math (`isoProjection.ts`, `mapView.ts`) and the chunk API are
  untouched. Steps 1–5 are independently shippable and independently revertible.
- Add deps via CLI only: `bun add pixi.js @pixi/react pixi-viewport`.
- Watch: `@pixi/react` needs a client component + dynamic import (`ssr: false`) in Next; use the
  `extend` API to keep the bundle small.

## Sources

- PixiJS v8 performance tips — https://pixijs.com/8.x/guides/concepts/performance-tips
- PixiJS v8 culling API — https://www.richardfu.net/optimizing-rendering-with-pixijs-v8-a-deep-dive-into-the-new-culling-api/
- `@pixi/react` v8 (React 19) — https://pixijs.com/blog/pixi-react-v8-live
- Pixi canvas + React DOM overlay — https://blog.logrocket.com/getting-started-pixijs-react-create-canvas/
- `@pixi/tilemap` — https://api.pixijs.io/@pixi/tilemap.html
- `pixi-viewport` (v8) — https://github.com/pixijs-userland/pixi-viewport
- Phaser vs Pixi — https://generalistprogrammer.com/tutorials/phaser-vs-pixijs-renderer-comparison
- WebGL MAX_TEXTURE_SIZE across devices — https://webgl2fundamentals.org/webgl/lessons/webgl-cross-platform-issues.html
