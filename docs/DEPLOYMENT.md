# Production deployment

Idle Cat Forest can ship as one container: `cat-server` serves the browser/WASM
bundle, the public image tree, `/ws`, and the health probes from the same origin.
SQLite data lives on a persistent `/data` volume.

## Build and run the container

```bash
docker build --tag idle-cat-forest:local .

docker run --rm \
  --name idle-cat-forest \
  --publish 8787:8787 \
  --volume idle-cat-forest-data:/data \
  --env NODE_ENV=production \
  --env SESSION_HMAC_SECRET='<replace-with-a-long-random-secret>' \
  --env CAT_SERVER_ALLOWED_ORIGINS='http://localhost:8787' \
  idle-cat-forest:local
```

Open `http://localhost:8787`. The release web bundle derives its WebSocket URL
from the page origin. Set `CAT_SERVER_ALLOWED_ORIGINS` to the
public browser origin (for example `https://cats.example`), not the internal
proxy-to-container URL. Multiple exact origins are comma-separated.

A reverse proxy must forward normal HTTP/WebSocket upgrades and set exactly one
`X-Forwarded-For` client IP. Put the proxy's exact TCP peer address in
`CAT_SERVER_TRUSTED_PROXY_IPS`. Forwarding headers from every other peer are ignored; a trusted
proxy with a missing, chained, duplicate, or malformed value is rejected. This prevents clients
from choosing the IP identity used by connection and abuse limits.

Do not put `SESSION_HMAC_SECRET` in an image or committed environment file. The server refuses to
expose a public bind without it. The insecure fallback is loopback-only unless
`CAT_SERVER_ALLOW_INSECURE_SESSION_SECRET=1` is deliberately set for development.
Sessions rotate without changing the stable player identity that owns personal villages. Ordinary
action access lasts 30 days, with a seven-day authenticated renewal window; credentials beyond
that window start as a new player and do not inherit the former player's villages.

## Runtime configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `BIND_ADDR` | `127.0.0.1` (`0.0.0.0` in the image) | IP address on which the server listens. Invalid hostnames fail startup. |
| `PORT` | `8787` | HTTP and WebSocket port. |
| `GAME_DB_PATH` | `data/cat.db` (`/data/cat.db` in the image) | Persistent SQLite database path. |
| `SESSION_HMAC_SECRET` | loopback-only development fallback | Required for production and every non-loopback bind. |
| `CAT_SERVER_ALLOW_INSECURE_SESSION_SECRET` | unset | Explicitly permits the built-in development secret; never set this on a public deployment. |
| `CAT_SERVER_WEB_DIST_DIR` | unset (`/app/web` in the image) | Trunk `dist/` directory. When set, it must contain `index.html`; unknown client routes use the SPA fallback. |
| `CAT_SERVER_PUBLIC_IMAGES_DIR` | unset (`/app/public-images` in the image) | Image tree served at `/public/images/`. This explicit tree takes precedence over a copy in `dist/`. |
| `CAT_SERVER_ALLOWED_ORIGINS` | unset | Exact, comma-separated WebSocket Origin allowlist. Required for non-loopback binds; optional for local/native clients. |
| `CAT_SERVER_TRUSTED_PROXY_IPS` | unset | Exact, comma-separated TCP proxy IPs whose single `X-Forwarded-For` client IP is trusted for abuse limits. |

Static files use extension-derived MIME types and `nosniff`. HTML is revalidated,
fingerprinted files are cached immutably for one year, other static files for one
hour, and images for one day. Compressible responses negotiate Brotli or gzip.

## Probes

- `GET /health` is a process liveness probe and returns `ok`.
- `GET /ready` uses non-blocking world/SQLite checks, verifies that the shared world contains a
  colony, and fails after three consecutive periodic save errors. It returns `ready`, or HTTP 503
  when the server should not receive traffic.

The image has a Docker `HEALTHCHECK` against `/ready` and runs as UID/GID 10001.

## Run the release binary without Docker

Build the Trunk bundle first, then point the server at explicit paths:

```bash
scripts/build-web.sh
cargo build --locked --release -p cat-server

NODE_ENV=production \
SESSION_HMAC_SECRET='<replace-with-a-long-random-secret>' \
BIND_ADDR=0.0.0.0 \
PORT=8787 \
GAME_DB_PATH="$PWD/data/cat.db" \
CAT_SERVER_WEB_DIST_DIR="$PWD/crates/cat-web/dist" \
CAT_SERVER_PUBLIC_IMAGES_DIR="$PWD/public/images" \
CAT_SERVER_ALLOWED_ORIGINS='http://localhost:8787' \
./target/release/cat-server
```
