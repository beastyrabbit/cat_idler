#!/usr/bin/env bash
# Build (and optionally serve) the browser/WASM client — P10.
#
# The Bevy client compiles to wasm and runs in a browser on WebGL2, streaming the
# same WorldSnapshot the native client does. This is the one reproducible entry
# point for that build; the in-browser result is verified by loading the served
# bundle in headless Chromium (Playwright) and asserting the canvas renders with
# zero console errors — see docs/migration/WASM.md for the smoke-test procedure.
#
# Usage:
#   scripts/build-web.sh                 # release bundle -> crates/cat-web/dist/
#   scripts/build-web.sh --serve         # bundle + live-serve on http://127.0.0.1:8080
#   CAT_SERVER_URL=ws://host:8787/ws scripts/build-web.sh   # bake a non-default WS URL
#
# Requires: `trunk` (cargo install trunk), the wasm32 target
# (rustup target add wasm32-unknown-unknown), and — for the -Oz size pass that
# index.html requests via data-wasm-opt="z" — `wasm-opt` on PATH (binaryen).
# On this machine the -Oz pass on the ~28 MB module takes ~2-3 min; the bundle
# lands at ~8 MB gzipped.
set -euo pipefail

cd "$(dirname "$0")/../crates/cat-web"

# Default the baked WS URL to the local dev server; omit CAT_SERVER_URL entirely
# for a same-origin deploy where the client derives ws(s)://<host>/ws from the
# page location.
: "${CAT_SERVER_URL:=ws://127.0.0.1:8787/ws}"
export CAT_SERVER_URL

if [[ "${1:-}" == "--serve" ]]; then
  echo "Serving browser client on http://127.0.0.1:8080 (CAT_SERVER_URL=$CAT_SERVER_URL)"
  echo "Run 'cargo run -p cat-server' in another terminal for a live colony."
  exec trunk serve --release --port 8080
fi

echo "Building release WASM bundle (CAT_SERVER_URL=$CAT_SERVER_URL) ..."
trunk build --release
echo "Bundle written to crates/cat-web/dist/ :"
ls -lh dist/
