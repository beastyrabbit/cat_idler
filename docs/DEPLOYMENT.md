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
from the page origin, so a reverse proxy only needs to forward normal HTTP and
WebSocket upgrades to the container. Set `CAT_SERVER_ALLOWED_ORIGINS` to the
public browser origin (for example `https://cats.example`), not the internal
proxy-to-container URL. Multiple exact origins are comma-separated.

Do not put `SESSION_HMAC_SECRET` in an image or committed environment file. The
existing server guard refuses to start with `NODE_ENV=production` when the
secret is missing.

## Runtime configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `BIND_ADDR` | `127.0.0.1` (`0.0.0.0` in the image) | IP address on which the server listens. Invalid hostnames fail startup. |
| `PORT` | `8787` | HTTP and WebSocket port. |
| `GAME_DB_PATH` | `data/cat.db` (`/data/cat.db` in the image) | Persistent SQLite database path. |
| `SESSION_HMAC_SECRET` | insecure development fallback | Required when `NODE_ENV=production`. |
| `CAT_SERVER_WEB_DIST_DIR` | unset (`/app/web` in the image) | Trunk `dist/` directory. When set, it must contain `index.html`; unknown client routes use the SPA fallback. |
| `CAT_SERVER_PUBLIC_IMAGES_DIR` | unset (`/app/public-images` in the image) | Image tree served at `/public/images/`. This explicit tree takes precedence over a copy in `dist/`. |
| `CAT_SERVER_ALLOWED_ORIGINS` | unset | Optional strict, comma-separated WebSocket Origin allowlist. Unset keeps local/native clients usable. |

Static files use extension-derived MIME types and `nosniff`. HTML is revalidated,
fingerprinted files are cached immutably for one year, other static files for one
hour, and images for one day. Compressible responses negotiate Brotli or gzip.

## Probes

- `GET /health` is a process liveness probe and returns `ok`.
- `GET /ready` verifies that SQLite answers a query and the shared world contains
  a colony. It returns `ready`, or HTTP 503 when the server should not receive
  traffic.

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
