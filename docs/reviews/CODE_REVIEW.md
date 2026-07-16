# Code Review — cat-sim, cat-server, cat-protocol, cat-client

> Post-review status: findings have been dispositioned and implemented where required. See
> [`RESOLUTION.md`](RESOLUTION.md); this file preserves the pre-fix evidence.

Date: 2026-07-16
Scope: deep code review of the simulation core (`cat-sim`), the authoritative server and wire
protocol (`cat-server`, `cat-protocol`, briefly `cat-dev`), and the client's logic-level code
(`cat-client`; UX-level findings live in `UI_UX_REVIEW.md`). All findings were confirmed against
the current code; file:line citations are to this tree.

## Overall verdict

The architecture is sound and the dangerous parts are done carefully: the tick/lock/save
concurrency model is correct, authorization is defense-in-depth, persistence is transactional,
and SQL is uniformly parameterized. The significant issues cluster at the **network edge** —
resource-exhaustion vectors on an always-on public server (unbounded village creation via
freely-mintable sessions, no message-size or per-IP connection caps) and **wire forward
compatibility** (no unknown-enum fallback, non-finite float hazard), where a single unexpected
value drops the client's entire snapshot frame. Nothing found is a memory-safety bug.

---

## Part 1 — cat-server / cat-protocol / cat-dev

### Verdict

Well-architected transport layer. The doc claim "save ticks clone state and release the lock
before disk I/O" is accurate and verified (`cat-server/src/main.rs:404-426`). The problems are
at the edges: DoS surface and protocol brittleness.

### High

- **H1 — Unbounded village creation + freely-mintable sessions = world-state exhaustion.**
  `main.rs:897-913`, `cat-sim/src/actions.rs:3294-3355`. `found_village` caps one personal
  village per `player_id`, but `player_id` derives deterministically from `session_id`
  (`identity.rs:94-99`) and any sessionless client that sends `Presence` is issued a fresh signed
  session (`main.rs:900-901`) — no IP binding, no cap. One machine can mint unlimited sessions →
  unlimited villages; there is no global cap on `world.colonies.len()`. Every added 15-cat colony
  is simulated every tick, serialized into the full-world snapshot broadcast to every client
  every second (~80–150 KB per mature colony), and rewritten on every 5-tick save. The
  30-action/10 s limiter is per session/connection, so it does not throttle this.
  *Fix:* wire `into_make_service_with_connect_info::<SocketAddr>()` + `ConnectInfo`, bind session
  issuance and founding to client IP, cap total colonies and personal villages per source IP.
- **H2 — Rate limiter (and absent connection cap) keys on connection id, not IP.**
  `main.rs:810-815`, `main.rs:492-497`. The pre-auth limiter key is `format!("ip:{}", ..)` but
  the value is `ws-{connection_id}` — a fresh id per socket, so reconnecting resets the budget
  and the "ip:" bucket never groups by client. No cap on concurrent sockets; each connection
  clones + projects the whole world per tick (`main.rs:514-521`).
  *Fix:* thread the real peer IP (see H1) into the limiter and a max-connections-per-IP gate;
  handle X-Forwarded-For when behind a proxy.
- **H3 — WebSocket accepts unbounded message sizes.** `main.rs:221-224`. No
  `.max_message_size`/`.max_frame_size` (axum defaults: 64 MiB message / 16 MiB frame), and
  `handle_client_text` runs `serde_json::from_str` on the full text (`main.rs:882`). With H2's
  missing connection cap, repeated multi-MB frames are a memory-pressure vector.
  *Fix:* `ws.max_message_size(64*1024).max_frame_size(64*1024)` before `on_upgrade`.
- **H4 — Wire enums have no unknown-variant fallback → a newer server freezes older clients.**
  `cat-protocol/src/lib.rs`: `ResourceKind` (:596), `BuildingType` (:1595), `JobKind` (:1095),
  `CarryingKind` (:996), `Labor` (:897), `OfficerRole` (:1083), `ColonyStatus` (:672), etc. —
  none carry `#[serde(other)]`. `WorldSnapshot` deserializes as one nested unit, so one unknown
  variant string anywhere (one new building type in one colony) makes the whole snapshot `Err`;
  the client drops every frame and blanks until reload. Live hazard: the project actively adds
  resources/buildings/recipes, and the server updates before connected wasm bundles do.
  *Fix:* add `#[serde(other)] Unknown` to client-facing unit enums with defensive handling at
  use sites, and/or stamp a protocol version.

### Medium

- **M1 — Non-finite floats serialize to `null`, then fail to parse the whole snapshot.**
  `cat-protocol/src/lib.rs` `HousingSnapshot.pressure` (:1183), `ThreatSnapshot.pressure`
  (:1287), `CatNeeds` (:1047), resource/haul amounts. serde_json emits `null` for NaN/Inf; a
  plain `f64` fails to parse `null`, collapsing the whole `WorldSnapshot` client-side. `pressure`
  is a population/capacity ratio and capacity can be 0 in the zero-Den founding window → Inf/NaN.
  The existing zero-Den round-trip test hardcodes `pressure: 15.0`, so it never exercises the
  division. *Fix:* sanitize non-finite floats at the snapshot-build boundary (or guarantee
  finiteness in-sim) and add a NaN/Infinity round-trip test.
- **M2 — Dev-fallback session secret gated only on `NODE_ENV=production`.**
  `identity.rs:22-37`. Exposed (`BIND_ADDR=0.0.0.0`) without `SESSION_HMAC_SECRET` and without
  `NODE_ENV=production`, the server silently signs sessions with the in-source constant —
  anyone can forge any session/player. The guard keys on the wrong signal (exposure is about the
  bind address). *Fix:* refuse the fallback for non-loopback binds, or require an explicit
  insecure opt-in.
- **M3 — Sessions never expire.** `identity.rs:40-72`. `verify_session` checks only
  `HMAC(session_id) == sig` — no TTL/timestamp/nonce; a captured credential is a permanent
  bearer token with no rotation or revocation. *Fix:* embed issued-at in the signed payload and
  enforce a max age, or document the permanent-bearer model as deliberate.
- **M4 — Default WebSocket Origin policy is unrestricted.** `hosting.rs:120-122`,
  `main.rs:209-219`. `AllowedOrigins::allows` returns true when the list is empty (the default),
  so without `CAT_SERVER_ALLOWED_ORIGINS` any web origin can open an authenticated WS
  (cross-site WebSocket hijacking). *Fix:* fail closed (require an allowlist) when binding
  publicly.
- **M5 — Full-world delete + row-by-row re-insert every save; no `prepare_cached`; missing
  `colonyId` indexes.** `persistence.rs:633` plus per-row `conn.execute` in the save fns. Every
  5 ticks and on shutdown the save deletes all 11 tables and re-inserts the whole world, each
  row compiling a fresh INSERT; cats/jobs/events/zones/elections/votes/raiders have unindexed
  `colonyId` (`persistence.rs:1209,1315,1569,1611,1665,1700,1742`), making boot load O(N·rows)
  per colony. H1 amplifies this. *Fix:* `prepare_cached` on hot writers, dirty-track/upsert
  colonies, add `CREATE INDEX IF NOT EXISTS ... (colonyId)`.
- **M6 — A single corrupt JSON blob aborts server boot with no isolation.**
  `persistence.rs:953` and every `.map_err(from_sql_json)?`, propagated by `main.rs:283`.
  Fail-closed is the right default, but one bad blob bricks an always-on server; no per-colony
  quarantine, no log of which colony/column failed, and no test pins the contract.
- **M7 — Persistent save failures are logged and otherwise swallowed.** `main.rs:423-425`.
  Disk-full or invariant-guard failures (`persistence.rs:599`) are logged and discarded; `/ready`
  doesn't reflect them, so the world can silently stop persisting while play continues.
  *Fix:* surface repeated save failures into `/ready` or a metric.

### Low

- **L1 — Snapshot bloat:** `ColonySnapshot.claimed_tiles`/`revealed_tiles`/`road_tiles`/
  `dirt_road_tiles` (`cat-protocol/src/lib.rs:189,197,208,224`) rarely change but re-encode every
  tick (~0.4–0.9 MB/s/client with several colonies). Consider on-change deltas +
  permessage-deflate. (The 487-study catalog is *not* embedded — only `owned_node_ids` — so no
  per-frame ledger bloat.)
- **L2 — `usize` in wire structs** (`AccountingRoundSnapshot.remaining_piles`/`unreachable_piles`
  :491-492; `ProductionQueueEdit` indices :1505,1507,1511) is non-portable across
  wasm32/x86_64; use `u32`.
- **L3 — Hand-rolled JSON for `TileResources`/`MaxResources`** (`persistence.rs:1896,1919`) —
  adding a field without editing both builder and parser silently drops it on restart; the
  persistence audit test doesn't cover tile-resource fields. Derive serde or extend the audit.
- **L4 — `TraderSnapshot.buy_offers`/`sell_offers` lack `#[serde(default)]`** unlike siblings
  (`cat-protocol/src/lib.rs:309,312`).
- **L5 — cat-dev stale-port check is TOCTOU** (`cat-dev/src/main.rs:61-83`) — dev tool,
  acceptable.
- **L6 — `/ready` locks world+db with `.await`** (`main.rs:193-200`); a heavy tick can block the
  probe. Consider `try_lock`.

### Strengths

- Concurrency: tick in `spawn_blocking` + `blocking_lock`; snapshot built before cache publish;
  persistence clones before releasing the world lock; no lock held across `.await` or disk I/O.
- Broadcast sheds load on slow clients (`RecvError::Lagged` → warn + continue, `main.rs:526-528`).
- Authorization defense-in-depth: every colony-scoped mutation routes through `with_colony`
  (`actions.rs:433-459`), re-checking `can_control_village`; trade re-checks both ends; the
  exhaustive `action_authentication` match (`main.rs:1007`) makes a new action a compile error
  until its auth class is chosen.
- Timing-safe HMAC compare (`identity.rs:67-71`); fully transactional save with tested rollback;
  genuinely idempotent migrations (PRAGMA `column_exists` guard, not error-swallowing); no SQL
  injection (all data queries parameterized); graceful shutdown save on SIGINT and SIGTERM;
  round-trip fidelity guarded by a persistence-audit test; disciplined additive
  `#[serde(default)]` conventions with legacy-payload tests.

### Test coverage gaps (server/protocol)

1. Unknown enum variants (H4) and non-finite floats (M1) — both hard-fail the whole frame,
   both untested.
2. Corrupt/non-JSON blob boot behavior (M6) untested and undocumented.
3. `TileResources`/`MaxResources` field completeness unguarded (L3).
4. `save_world` invariant-guard failures and the save-failure path (M7) untested.
5. No cap/DoS tests for total colonies or session issuance (H1/H2); no snapshot-size upper-bound
   guard (L1).
6. Never-populated in round-trip tests: `ProductionWorkSlotSnapshot`/`work_slots`, most
   `ItemLocation` variants, `VillageTradeOfferSnapshot`, `TransportDockSnapshot`.

---

## Part 2 — cat-sim

### Verdict

**Strong health.** The determinism discipline the project depends on is real and consistently
enforced: the seeded LCG is bit-exact and golden-tested; subsystem seed forks (movement
`+1_000_003`, life `+2_000_003`, raids `+3_000_003`) are correctly threaded and proven isolated
by a twin test (`raid_rolls_do_not_advance_base_test_rng_seed`); decision-driving containers are
`BTreeMap`/`BTreeSet`/`Vec` — no `HashMap`/`HashSet` iteration order feeds a path, target,
election winner, or scarce-resource allocation. Float comparisons in hot sorts are NaN-safe,
float→int casts saturate, divisions are guarded. The confirmed problems concentrate in
**`actions.rs`** — the one surface running on untrusted client input — where two reachable
denial-of-service vectors exist.

### Critical

- **C1 — `ProductionQueueEdit::Move` swaps an unchecked source index → tick-killing panic.**
  `cat-sim/src/actions.rs:1491-1503`. The `Move` arm bounds-checks the swap *target* but never
  the *source* `*index` before `queue.swap(*index, target)`; `Vec::swap` bounds-checks in
  release too — a hard panic. The sibling `Remove` arm (:1483) checks correctly; `Move` doesn't.
  *Failure scenario:* on any colony with a completed production building (the shared global
  colony is controllable by any session), send `EditProductionQueue { Add }` once (queue length
  1), then `Move { index: 1, direction: Up }` — `target = 0` passes `0 < 1`, `queue.swap(1, 0)`
  on a length-1 vec panics and kills the world tick.
  *Fix:* mirror `Remove`'s guard: `if *index >= queue.len() { return fail(...); }`.

### High

- **H1 — `designate_rail` materializes the whole path before the length cap → OOM/hang.**
  `cat-sim/src/actions.rs:2932-2937` → `cat-sim/src/transport.rs:154-170`. The 128-tile cap is
  checked only *after* `transport::cardinal_line` has fully built the path `Vec` from raw client
  coords; `TilePoint.x/y` are unbounded `i32` on the wire, and the internal `(b.x - a.x)` is
  unchecked `i32` subtraction (opposite-sign extremes overflow → wrong `signum` → runaway loop
  in release). Contrast `build_road` (~:2738), which rejects `|coord| > 1000` and
  `distance > 24` *before* expanding.
  *Failure scenario:* with `rail` researched (a normal progression unlock on the shared colony),
  `DesignateRail { a:{0,0}, b:{0, 2_000_000_000} }` loops ~2e9 iterations pushing ~16 GB of
  `TilePos` → allocator OOM kills the server.
  *Fix:* bound endpoints/Manhattan distance before calling `cardinal_line` (mirror
  `build_road`), and give `cardinal_line` checked `i64` deltas plus an in-loop length bail.

### Medium

- **M1 — A* working arrays sized to the start↔goal bounding box, not bounded by
  `max_expansions`.** `cat-sim/src/pathfinding.rs:278-296`. `find_path` allocates and fills
  three `width*height` arrays from the Manhattan span before searching; `DEFAULT_MAX_EXPANSIONS`
  (6000) caps search *work*, not this allocation, so a far goal costs O(dist²) even when the
  search immediately bails. A cat with an ~800-tile goal allocates/zeroes ~11 MB, expands 6000
  nodes, returns `None`, frees it — every second, per such cat. In-colony movers are cheap
  (goals near); this bites long-range scout/hunt/inter-colony goals.
  *Fix:* clamp the search window to a budget derived from `max_expansions`, or back the
  closed/g-score/came-from stores with maps keyed by visited tile.

### Low

- **L1 — `find_path` bounds arithmetic can panic on extreme finite coordinates.**
  `pathfinding.rs:278-294`. `js_round_to_i32` saturates a huge finite `WorldPos` to `i32::MAX`,
  then `+ margin` overflows → negative width → `usize::try_from(...).expect(...)` panic (release
  too), aborting `world_tick`. *Fix:* clamp inputs or use saturating bounds + early `None`.
- **L2 — Employment fill pass can dispatch quarriers to a full stone store.**
  `leader_director.rs:598-618`. `fill_order` gates Quarry only on `has_quarry_site` — no
  `stone_r < 1.0` guard, though the Quarry *goal* is vetoed at `stone_r >= 1.0` (:866) and the
  module's own design comment says the fill pass must not dispatch to a saturated store.
  Deterministic wasted labor, non-crashing. *Fix:* add the stone-ratio gate to the fill entry.
- **L3 — Test-acceleration actions mutate the shared world with no ownership gate in cat-sim.**
  `actions.rs:249-274`. `SetTestAcceleration`, `AdvanceTime` (up to 86,400 s of tick), and
  `SetTestRngSeed` (reseeds *every* colony) bypass `with_colony`. The server shell disables them
  in release builds (`cat-server/src/main.rs:336-347`), so this is a belt-and-suspenders note:
  cat-sim itself embeds no gate. *Fix:* optionally add a config gate inside cat-sim.
- **L4 — `combat::d20_from_random` is unclamped (dormant).** `combat.rs:122` — a roll of exactly
  `1.0` yields 21; the live raid path (`threat::resolve_raid`) clamps, so only tests reach it.
- **L5 — NaN-stat comparators are deterministic-but-unspecified.** `warriors.rs:311`,
  `elections.rs:227` — a NaN diff sorts `Equal`; a NaN-stat cat lands in a stable but
  unspecified slot. Only relevant if upstream ever produces NaN stats.

Reviewed-and-dismissed (guarded elsewhere): `genetics::random_index`'s assert (production roll
source only yields `[0,1)`); `Stockpile::tiles()` raw rect math (unreachable — `designate_stockpile`
validates via i64 `rect_dimensions` first).

### Strengths

- Airtight determinism where it counts (golden-tested LCG, twin-tested seed forks, monotonic A*
  tie-break `seq`, platform-stable FNV-1a gait hash).
- Uniform NaN/panic-safe numeric style (JS-parity `js_min`/`js_max`/`js_round`, saturating
  casts, guarded divisions).
- Conserved economy (cycle math floors on the scarcer of time-vs-inputs; capacity clamps;
  `stockpiles::reconcile` enforces the physical invariant; drain/deposit ties break on unique id).
- Centralized authorization in `actions.rs` via `with_colony` on the server-authoritative
  `ctx.colony_id` with `can_control_village` re-checks; amounts validated
  `is_finite() && > 0` with caps.
- Overflow-safe phase gating (`saturating_sub`/`saturating_mul`); life/raid forks derive from
  the post-policy-roll base seed each tick and never persist back.

### Test coverage gaps (cat-sim)

1. **Highest value:** boundary test for `ProductionQueueEdit::Move` with `index == queue.len()`
   (catches C1); adversarial far-coordinate tests for `DesignateRail`/`BuildRoad` asserting a
   clean `fail` (catches H1).
2. No far start/goal `find_path` test (M1's allocation cliff unmeasured); no
   degenerate-coordinate or negative-margin test (L1).
3. No empty/zero-population guardrail test for `leader_director::direct_colony`/`allocate_labor`;
   nothing pins the L2 quarry overdispatch.
4. No NaN-input tests for `rank_goals` sort stability, `stockpiles::reconcile` (a NaN resource
   would silently corrupt the conservation invariant), or muster/candidacy selection (L5).
5. No double-tie election test (equal votes and equal leadership) documenting input-order
   dependence; no combat boundary test at `random == 1.0` (L4).

---

## Part 3 — cat-client (logic-level)

Player-facing UX findings are in `UI_UX_REVIEW.md`. This section covers engine/perf/architecture.

### Verdict

Strong engineering hygiene. Entities are reconciled in place (no per-frame despawn/respawn
churn), nearly every HUD system early-outs on `is_changed()`, the camera obeys the Z=1000 rule,
reconnect uses capped exponential backoff, and there are ~285 logic/UI-shape tests. The live run
rendered correctly on a real GPU with no clipping or black-screen. One perf item is worth
addressing before scaling world size.

### Medium

- **CM1 — Snapshot is double-deserialized on the main render thread every second.**
  `parse_server_message` does `serde_json::from_str::<Value>` over the entire snapshot, then
  `serde_json::from_value::<WorldSnapshot>` (`cat-client/src/lib.rs:5500-5504`), inside `poll_ws`
  — an exclusive `&mut World` system in `Update` (`:5389`, scheduled `:3685`). That is a full
  intermediate `Value` DOM allocation plus a second walk, on the frame thread, for the whole
  shared world (all colonies/cats/items) every second. Frame-hitch risk on native, worse on
  wasm; amplified by the snapshot-bloat issue (`L1` in Part 1).
  *Fix:* deserialize once — peek the discriminator (tagged envelope) and go straight
  `from_str::<Typed>`, avoiding the `Value` round-trip; longer term, offload parsing to a task
  pool and hand the render thread the finished struct.

### Low

- **CL1 — `LatestSnapshot` never cleared on transport loss** (root of UX finding H1). Render
  state should reflect "no live data" when disconnected; leaving the last `Some` conflates stale
  with live. Set at `lib.rs:5429`; nothing clears it in `schedule_reconnect`.
- **CL2 — Reconnect banner TTL (8 s) < backoff cap (30 s)** — the status can expire before the
  action completes (`lib.rs:5640` vs `:80`).

### Strengths

- **No entity churn.** Cats, buildings, fog, roads, and research cards are reconciled in place
  against keyed maps and despawned only on removal; a test
  (`cat_body_reconciliation_repairs_stale_cache_and_despawns_duplicates`, `:14670`) guards it,
  and the research UI explicitly documents the no-`Commands`-access guard (`research_ui.rs:1113`).
- **Aggressive change-gating** — ~25 update systems early-out on `is_changed()`, so the 1 s
  cadence doesn't cause per-frame reformatting.
- **Camera Z rule honored** — `CAMERA_Z = 1000.0`, sprites Y-sorted below (`lib.rs:134`,
  `:4232-4237`).
- **Clean wasm/native split** for session persistence (localStorage vs config file,
  `:434-477`) and window config (`:3838`), with a test asserting the 1024×768 browser resolution.
- **Substantial tests for a UI crate** (~134 in `lib.rs`, 16 in `research_ui.rs`, 6 in
  `station_layout.rs`), all pure logic/UI-shape per the contract.

---

## Consolidated fix priority (all crates)

1. **C1** (cat-sim) — `ProductionQueueEdit::Move` panic: bounds-check the source index. One-line
   fix, kills the world tick, reachable by any client on the shared colony.
2. **H1** (server) — bind sessions/founding to client IP + cap colonies per IP (DoS).
3. **H1** (cat-sim) — bound `designate_rail` coordinates before expanding the path (OOM).
4. **H2/H3** (server) — real per-IP rate-limit key + WebSocket message-size cap.
5. **H4 + M1** (protocol) — `#[serde(other)]` fallback on wire enums and non-finite-float
   sanitization; both currently drop the client's entire snapshot frame.
6. **CM1** (client) — single-pass snapshot deserialization off the `Value` round-trip.
7. Remaining Medium/Low items and the test-coverage gaps listed per part.
