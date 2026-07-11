# P10 — WebAssembly / browser build

Status: **RUNS IN A BROWSER — verified end-to-end.** `trunk build --release`
produces a working `dist/`, and the game renders + streams live in headless
Chromium (Playwright): WebGL2, the sprite atlas, the 9-patch parchment UI, all
assets, and the WebSocket snapshot stream (live HUD: online count, pop, jobs,
event feed) — including the current top-down cutaway-interior look. Native
`cargo dev` is unaffected (wasm is a separate target).

## Verified running build (Playwright smoke test)

- Build: `cd crates/cat-web && CAT_SERVER_URL=ws://127.0.0.1:8787/ws trunk build
  --release` (bakes the dev WS URL via `option_env!`; omit for a same-origin
  deploy that derives `ws(s)://<host>/ws` from `web_sys` location).
- Serve: any static host over `dist/` (e.g. `python3 -m http.server 8090`) — no
  COOP/COEP needed (single-threaded wasm, no SharedArrayBuffer). Run `cat-server`
  on `:8787`.
- **Real release bundle size: wasm 27 MB raw / 8.4 MB gzipped** + 104 KB JS glue
  + ~2.5 MB assets (~31 MB `dist/` total). 8.4 MB gzipped is the transfer cost —
  heavy but workable. `index.html` requests `data-wasm-opt="z"`, but that only
  fires when a `wasm-opt` binary is on `PATH` (trunk silently skips it otherwise);
  on a lightly-loaded machine it trims the module, but a full `-Oz` pass on this
  27 MB module is minutes-to-hours of CPU, so it's opt-in, not on the default loop.
- Result in-browser: full scene renders on WebGL2 (top-down cutaway interiors,
  sprite-atlas cats, 9-patch UI); the HUD shows live snapshot data and the event
  feed streams (WS connected). **0 console errors** — the former `*.png.meta` 404
  flood is gone (`AssetMetaCheck::Never` on the client `AssetPlugin`; the game
  ships no `.meta` sidecars) and the favicon 404 is gone (a pixel-cat
  `favicon.png` is bundled + linked). No WebGL/wgpu errors, no wasm panic.

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

## Concrete path to a running browser build (proposed slices)

1. **Bundle** — `cargo install trunk`, then `cd crates/cat-web && trunk build
   --release`. Trunk runs wasm-bindgen + wasm-opt and emits `dist/` (wasm + JS
   glue + copied assets). `trunk serve` for local dev.
2. **Assets** — already handled by the `copy-dir` link; verify the served paths
   match the `public/images/...` load strings (they do today).
3. **WS URL** — `std::env::var("CAT_SERVER_URL")` returns `Err` in the browser,
   so `connect_ws` falls back to `ws://127.0.0.1:8787/ws`. For a real deploy,
   add a `cfg(target_arch = "wasm32")` branch that derives the URL from
   `web_sys::window().location()` (or bake it with `option_env!`). Small, local
   change in `connect_ws`.
4. **Canvas / sizing** — decide between Bevy's default body-appended canvas and
   a fixed `#bevy-canvas` with `fit_canvas_to_parent`. Cosmetic.
5. **Serve + smoke test** — point the browser at `trunk serve` with a local
   `cat-server` running; confirm the snapshot stream renders (sprites + 9-patch
   UI) on WebGL2.

## Biggest risks / unknowns

- **Bundle size.** Debug wasm is ~800 MB; release + wasm-opt `-Oz` + gzip
  typically lands a Bevy app in the ~15–40 MB range — still heavy for a browser
  game. Needs measurement; may want `opt-level="z"`, LTO, and stripping.
- **WebGL2 parity.** The sprite atlas + 9-patch (`ImageNode` sliced) UI *should*
  render unchanged on WebGL2, but this is unverified in-browser — the one item
  most likely to need tweaks (texture formats, `RenderAssetUsages`).
- **Threads / atomics.** We run single-threaded on wasm (no `multi_threaded`);
  fine, but perf on a big colony is unmeasured. The threaded/atomics path
  (SharedArrayBuffer + COOP/COEP headers) is a separate, larger effort if needed.
- **WS from a non-localhost host.** Item 3 above — the current localhost
  fallback only works when the server is co-located; needs the location-derived
  URL before any real deploy.
- **Asset count / latency.** ~40 individual PNG fetches on load; fine locally,
  but a deploy may want atlasing or an asset manifest to cut request count.

## Recommendation

Getting it to **compile** was quick and clean, and that config is committed
(wasm is a separate target — it does not affect the native `cargo dev`). A
running browser build is a bounded but non-trivial follow-up (trunk bundle + the
small WS-URL cfg + an in-browser WebGL2 smoke test). No red-flag blockers
surfaced; bundle size and WebGL2 UI parity are the two things to verify before
committing to a shippable web build.
