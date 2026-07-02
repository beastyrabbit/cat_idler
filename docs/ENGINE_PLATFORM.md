# Engine & Platform Evaluation

_Is the current platform right for this game long-term?_ Written 2026-07-03.

## TL;DR — recommendation

**Stay web-native and keep the TypeScript simulation. Swap the _renderer_, never the
_engine_.** The right long-term shape is a **hybrid**: the ~90-module, ~16.9k-line
pure-TS `lib/game` core (guarded by ~2,000 tests) stays exactly where it is; we add a
real 2D renderer (Canvas/WebGL via PixiJS) on the client, and we only touch the backend
(dedicated tick service, Postgres, Redis fan-out) when concrete concurrency triggers fire.

Do **not** rewrite the sim into a game engine (Unity/Godot). Porting TS → C#/GDScript
throws away the single biggest asset this project has — a deterministic, unit-tested
simulation — and buys us rendering we can get on the web for a fraction of the cost.

## Where we are today

```
Browser (Next.js 16 + React 19, DOM renderer)
  ↕ SSE (/api/game/stream) pushes full dashboard 1×/s   +   POST /api/game/actions
Next.js route handlers  →  server/*  →  lib/game/* (pure)  ←  worker/index.ts (1 Hz tick)
                                              ↕
                                    SQLite (WAL, better-sqlite3)
```

Two facts dominate every decision below:

1. **The simulation is portable and already excellent.** `lib/game/` is ~90 pure
   modules with **no DB imports and no side effects**, exercised by **~2,033 `it()`
   tests**. That is the crown jewel. It runs unchanged in Node, Bun, a Web Worker, or a
   serverless function.
2. **The simulation is authoritative and server-ticked, not lockstep or
   client-predicted.** One worker process advances a **shared world**; clients are
   read-mostly viewers that occasionally POST an action. This is an _idle god-sim_, not a
   twitch shooter — latency budgets are seconds, not milliseconds.

Those two facts mean the renderer and the backend are **independently swappable**, and
almost every worthwhile improvement lives on one side without disturbing the other.

## Option A — Stay web-native (current stack + WebGL renderer)

**Verdict: yes, this is the path.** Cost is low, the sim is untouched, load stays instant,
and the tab-friendly / zero-install nature that idle games depend on is preserved.

Known scaling limits and where they actually bite:

- **SSE fan-out is the _first_ bottleneck, and it's self-inflicted, not a protocol
  limit.** `app/api/game/stream/route.ts` currently calls `getGlobalDashboard(db)` **once
  per client per second** — N SQLite reads + N JSON serializations per tick for a world
  that only changes once per tick. Fix: compute the dashboard **once per tick** and
  broadcast the shared buffer. SSE itself scales well — ~2–5 KiB of server state per
  connection vs ~50 KiB for WebSocket, edge-cacheable through Cloudflare/Fastly, and no
  sticky sessions thanks to `Last-Event-ID`. ([SSE scaling](https://blog.codercops.com/blog/server-sent-events-vs-websockets-2026), [Ably](https://ably.com/blog/websockets-vs-sse))
- **Single-writer SQLite is not the near-term problem people assume.** WAL mode sustains
  ~10k–50k writes/s on modern hardware, and our _only_ hot writer is the worker (player
  actions are rare, short transactions). One shared colony at 1 Hz is nowhere near the
  ceiling. ([SQLite WAL](https://sqlite.org/wal.html), [powersync](https://powersync.com/blog/sqlite-optimizations-for-ultra-high-performance))
- **One-process tick is the real long-term ceiling for _multi-colony_.** The worker ticks
  serially and already warns when a tick exceeds its interval. Cost scales with
  `#colonies × per-colony-tick-cost`. One shared world is trivial; hundreds of colonies
  eventually blow the 1 s budget and force worker sharding (see Stage 3).

## Option B — Game engines (Unity WebGL, Godot 4)

**Verdict: no, for the whole game. Reconsider only the renderer, and even then the web
tooling wins.**

What they'd _buy_ us: a mature scene graph, particles, tilemap tooling, and a WebGPU path
(Godot 4.7, June 2026). What they _cost_ us:

- **The sim would have to be rewritten in C#/GDScript, deleting the 2,000-test suite.**
  This is disqualifying on its own. The test suite is our correctness guarantee for a
  simulation with breeding, genetics, economy, and combat interacting — irreplaceable.
- **Payload and load time fight the genre.** Idle games live or die on instant load in a
  background tab. A Godot web build is ~5 MB Brotli-compressed at best (≈40 MB
  uncompressed WASM before stripping); Unity WebGL routinely ships tens to hundreds of MB
  and long init screens, "the number one reason players abandon web games before they
  start." ([Godot web export](https://docs.godotengine.org/en/stable/tutorials/export/exporting_for_web.html), [Godot size](https://amann.dev/blog/2025/godot_web_size/), [Unity WebGL](https://cricsnapp.com/2026/02/05/ultimate-guide-to-optimizing-unity-webgl-build-sizes/))
- **Per-tab memory ceilings and browser fragility.** Unity WebGL is memory-bound per tab
  and notoriously brittle across browser releases — a bad fit for a game meant to sit
  open all day. ([Unity memory](https://backtrace.io/blog/memory-and-performance-issues-in-unity-webgl-builds))
- **A full engine is overkill for a 2D top-down/isometric colony view.** We render tiles
  and sprites, not 3D scenes.

If we ever want richer visuals, **PixiJS (WebGL/WebGPU 2D)** or a Canvas renderer gives us
90% of the payoff, loads in a fraction of the bytes, and leaves the TS sim and test suite
intact. That is Option D.

## Option C — Dedicated multiplayer servers (Colyseus / Nakama) or a proper tick service + Postgres

**Verdict: warranted _later_, and even then keep the TS sim. Colyseus/Nakama are a
partial fit; a plain Node tick service + Postgres + Redis is the more aligned target.**

- **Colyseus** is TypeScript/Node and room-based, which _sounds_ ideal — one room per
  colony. But its value is real-time **state sync + client authority for many small
  rooms** (matchmaking, anti-cheat, interpolation). Our model is a small number of
  large, long-lived, server-authoritative worlds pushed at 1 Hz. We'd use ~10% of
  Colyseus and inherit its room lifecycle. ([Colyseus](https://ably.com/blog/websockets-vs-sse), [comparison](https://www.saashub.com/compare-nakama-vs-colyseus))
- **Nakama** (Go) bundles chat, leaderboards, matchmaking, and social — useful someday,
  but it's a steeper platform to adopt and again assumes many small sessions. ([Nakama vs Colyseus](https://forum.heroiclabs.com/t/nakama-vs-colyseus/1632))
- **The clean path** when we outgrow one process: split the always-on **tick service**
  (Node/Bun running the same `lib/game` unchanged) from the web tier, move persistence to
  **Postgres** (many concurrent writers, MVCC, horizontal read scaling), and fan SSE out
  via **Redis pub/sub**. Postgres is the documented move once write throughput or colony
  count outgrows a single SQLite writer. ([SQLite→Postgres](https://daily.dev/blog/sqlite-production-guide-when-how-to-use-beyond-prototyping/))

Crucially, none of this requires touching `lib/game`. The sim doesn't care whether state
lands in SQLite or Postgres — `server/*` is the only seam that changes.

## Option D — Hybrid: keep TS sim, swap the renderer

**Verdict: this _is_ the recommendation.** It's Option A executed deliberately: the sim,
tests, tick loop, and SSE contract stay; the client's DOM view is replaced with a
WebGL/Canvas renderer fed by the same dashboard stream. Zero backend risk, no lost tests,
instant load preserved.

## Staged path & triggers

| Stage | Trigger to start | Work | Sim / test impact |
|---|---|---|---|
| **0 — Harden current stack** | now | Cache dashboard **once per tick**, broadcast to all SSE clients (kill per-client DB reads); add delta payloads if frames get large | none |
| **1 — Renderer swap (Option D)** | want richer visuals / the isometric terrain overhaul lands | PixiJS or Canvas renderer driven by existing SSE state | none |
| **2 — Fan-out scaling** | ~1,000+ concurrent viewers, or egress/CPU on the web tier climbs | Split web tier from tick worker; Redis pub/sub for SSE fan-out; keep SQLite | none |
| **3 — Multi-colony / write pressure** | tick budget exceeded (`#colonies × cost > 1 s`), or sustained writes approach SQLite's ceiling, or many colonies need concurrent writers | Shard workers by colony; migrate SQLite → **Postgres**; `server/*` only | none — `lib/game` unchanged |
| **4 — Social/meta platform** | leaderboards, cross-colony trade, accounts at scale become product priorities | Evaluate **Nakama** for social/meta layer _beside_ the tick service, not replacing it | none |

**Solo-dev note:** every stage is additive and independently shippable. Stage 0 is a small
refactor with immediate payoff. Nothing on this path ever asks one person to rewrite a
tested 16.9k-line simulation, and nothing sacrifices the instant-load property the genre
requires.

## Sources

- SSE scaling & fan-out: [codercops](https://blog.codercops.com/blog/server-sent-events-vs-websockets-2026), [Ably](https://ably.com/blog/websockets-vs-sse)
- SQLite WAL / single-writer limits & Postgres migration: [sqlite.org](https://sqlite.org/wal.html), [powersync](https://powersync.com/blog/sqlite-optimizations-for-ultra-high-performance), [daily.dev](https://daily.dev/blog/sqlite-production-guide-when-how-to-use-beyond-prototyping/)
- Godot web export size/load: [Godot docs](https://docs.godotengine.org/en/stable/tutorials/export/exporting_for_web.html), [amann.dev](https://amann.dev/blog/2025/godot_web_size/)
- Unity WebGL drawbacks: [SnappGames](https://cricsnapp.com/2026/02/05/ultimate-guide-to-optimizing-unity-webgl-build-sizes/), [Backtrace](https://backtrace.io/blog/memory-and-performance-issues-in-unity-webgl-builds)
- Colyseus vs Nakama: [saashub](https://www.saashub.com/compare-nakama-vs-colyseus), [Heroic Labs forum](https://forum.heroiclabs.com/t/nakama-vs-colyseus/1632)
