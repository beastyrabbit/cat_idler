# P10 — WebAssembly / browser build

Status: **DONE — runs in a browser, verified end-to-end.** `scripts/build-web.sh`
(→ `trunk build --release`) produces a working `dist/`, and the game renders +
streams live in headless Chromium (Playwright): WebGL2, the sprite-atlas cats,
the current **"cozy ledger" UI kit** (top command bar, Colony HUD, Upgrade Tree,
cat inspector, Dispatches, minimap, bottom command bar), the top-down
cutaway-interior buildings, and the WebSocket snapshot stream (live HUD: online
count, pop, jobs, event feed). Native `cargo dev` is unaffected (wasm is a
separate target).

## Reproduce

- Build: `scripts/build-web.sh` (or `cd crates/cat-web && trunk build --release`).
  A release build leaves `CAT_SERVER_URL` unset by default and derives
  `ws(s)://<host>/ws` from the page location. Set the variable only to bake an
  intentionally separate server origin. `scripts/build-web.sh --serve` defaults to the
  local development server at `ws://127.0.0.1:8787/ws` because Trunk uses port 8080.
- Serve: `scripts/build-web.sh --serve` (trunk serve on `:8080`), or any static
  host over `dist/` — no COOP/COEP needed (single-threaded wasm, no
  SharedArrayBuffer). Run `cargo run -p cat-server` on `:8787` for a live colony.

## Verified running build (Playwright smoke test)

Re-verified end-to-end after the UI redesign (the new kit renders identically in
the browser). Method: `trunk serve --release` over the release bundle + a local
`cat-server`, loaded in Chromium at 1280×800 via Playwright.

- Canvas comes up at full size on WebGL2; the full scene renders (terrain, the
  founded village with cutaway-interior buildings, sprite-atlas cats, roads) and
  the new-kit HUD shows live snapshot data (pop 5/16, "thriving", dispatches
  streaming) — i.e. the WS stream is connected and driving the render.
- **0 console errors.** The former `*.png.meta` 404 flood is gone
  (`AssetMetaCheck::Never` on the client `AssetPlugin`; the game ships no `.meta`
  sidecars) and the favicon 404 is gone (a pixel-cat `favicon.png` is bundled +
  linked). No WebGL/wgpu errors, no wasm panic.
- **Bundle size: wasm ~28 MB raw / ~8 MB gzipped** + 104 KB JS glue + ~2.5 MB
  assets. `index.html` requests `data-wasm-opt="z"`; the `-Oz` pass on the ~28 MB
  module runs in ~2–3 min on this machine (needs `wasm-opt`/binaryen on `PATH`;
  trunk silently skips it otherwise). 8 MB gzipped is the transfer cost — heavy
  but workable; atlasing / an asset manifest would cut the ~40-PNG fetch count for
  a real deploy.

## Native packaging

The native client ships as `cargo build --release -p cat-desktop` (a single
binary). It needs `BEVY_ASSET_ROOT` pointed at the workspace root so
`public/images/...` resolves, and `CAT_SERVER_URL` for the server to connect to
(`cargo dev` sets both for local dev). A platform installer/bundle is out of
scope — the release binary + those two env vars is the packaging contract.

## Earlier scouting (compile feasibility)

The full Bevy client (`cat-client`) and the browser entry bin (`cat-web`) link
to wasm with zero errors.

## What was attempted / result

- `rustup target add wasm32-unknown-unknown` — installed.
- `cargo build -p cat-web --target wasm32-unknown-unknown` — **succeeds** (0
  errors). Produces `target/wasm32-unknown-unknown/debug/cat-web.wasm` (~800 MB
  debug; a release + `wasm-opt` build is far smaller — see risks).
- `cargo build -p cat-client --target wasm32-unknown-unknown` — **succeeds**
  (the lib rlib links; bevy_winit, wgpu, ewebsock, console_error_panic_hook all
  compile for wasm).
- Native (`cargo build -p cat-client -p cat-desktop`) is **unchanged** — the
  bevy feature split (below) is target-gated.

## Dependency picture (clean)

`cat-client`'s tree is wasm-safe: `bevy`, `cat-protocol`, `cat-sim`,
`ewebsock`, `serde_json`. No `tokio` / `rusqlite` / `mio` — those live only in
`cat-server`, which is not in the client's tree. `ewebsock` provides a browser
WebSocket backend, so the SSE-less WS transport works unchanged in the browser.

## Build config already committed

- `cat-web/Cargo.toml` — depends on `cat-client`; adds
  `console_error_panic_hook` for the wasm target only.
- `cat-web/src/main.rs` — sets the panic hook (wasm) and calls
  `cat_client::run()`. Bevy's `App::run()` doesn't block on wasm (winit drives
  it from `requestAnimationFrame`), so calling it from `main` is correct.
- `cat-client/Cargo.toml` — bevy features split three ways (Cargo unions them):
  - common: renderer/UI/text/sprite/winit/png,
  - `cfg(not(wasm32))`: `multi_threaded`, `x11`, `wayland`,
  - `cfg(wasm32)`: `webgl2` (render on the near-universal WebGL2 rather than
    requiring WebGPU).
- `cat-web/index.html` — trunk entry: builds the bin, copies `public/images`
  into the dist so the client's `public/images/...` asset loads resolve.

## Resolved during P10 (previously the "risks")

- **WebGL2 parity** — verified in-browser: the sprite atlas and the whole UI kit
  render unchanged on WebGL2. (No `RenderAssetUsages`/texture-format tweaks
  needed.)
- **Bundle size** — measured: ~28 MB raw / ~8 MB gzipped with `-Oz`. Workable.
- **WS URL** — `cat-web` derives `ws(s)://<host>/ws` from the page location when
  `CAT_SERVER_URL` isn't baked, so a same-origin deploy needs no code change.
- **Assets** — the `copy-dir` link resolves `public/images/...` at the served
  paths; 0 asset 404s in the smoke test.

## Production hosting

The repository now includes a non-root multi-stage `Dockerfile` that builds the optimized
Trunk bundle and native server, serves the SPA and `/images` from `cat-server`, persists
SQLite on `/data`, and exposes `/health` plus the stateful `/ready` probe. Static responses
use Brotli/gzip, correct MIME types, cache policy, and `nosniff`; WebSocket origins can be
restricted exactly. See `docs/DEPLOYMENT.md` for the build/run/reverse-proxy recipe.

## Remaining optional optimization

- **Transfer weight** — 8 MB gzipped is heavy for a game; atlasing or an asset
  manifest would cut the ~40-PNG fetch count. Optional optimization, not a
  blocker.
- **Threads / atomics** — we run single-threaded on wasm (no `multi_threaded`);
  perf on a large colony is unmeasured. The SharedArrayBuffer + COOP/COEP path is
  a separate, larger effort only if profiling demands it.
