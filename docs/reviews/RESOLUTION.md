# Review resolution ledger

Date: 2026-07-16

This is the maintained disposition for every finding in this folder. The review files preserve
the evidence that produced each finding; this ledger records what changed after the review. A
finding is not marked resolved merely because the project compiles.

## Feature and documentation review

| Finding | Disposition |
| --- | --- |
| Recipe count was 104 instead of 108 | **Resolved.** README and CLAUDE now use the asserted 108-recipe runtime count. |
| About 40 phases/modules understated the implementation | **Resolved.** The docs now state 53 ordered phases and 60+ simulation modules. |
| Module and persistence maps omitted maintained systems | **Resolved.** The missing economy/world modules and `shared_world_tiles` are documented. |
| Research catalog generation was undocumented | **Resolved.** CLAUDE and ARCHITECTURE explain the legacy JSON plus named family/stage source expansion and validation. |
| Cat specialization presentation claim was inaccurate | **Resolved.** The client now has distinct shape-and-color specialization/officer badges, and the docs describe those cues precisely. |

The feature review found no missing maintained game system. Historical design documents remain
historical rather than being converted into a second live backlog.

## Simulation and protocol review

| Finding | Disposition |
| --- | --- |
| Out-of-range production-queue move could panic the shared tick | **Resolved.** The source index is validated before target calculation/swap, with a clean-failure regression. |
| Rail designation expanded attacker-sized or overflowing coordinates | **Resolved.** Endpoints/distance are bounded before expansion; the line builder uses checked `i64` deltas and a 128-tile cap. |
| A* allocated a dense start-to-goal rectangle before its work cap | **Resolved.** Search state is sparse and grows only with discovered nodes; extreme coordinates and bounds arithmetic are checked. |
| Full Stone storage still received quarry fill labor | **Resolved.** The employment floor respects Stone fill ratio; the golden fixture now records the corrected Scout/Hunt allocation. |
| Mature villages could outrun their water-fetch transit reserve | **Resolved from the generalized campaign gate.** The per-cat transit reserve is two units, which keeps the four bounded physical fetchers ahead of migration-grown consumption without changing the migration or research prosperity bars. Seeds 42 and 99 now complete the established 300-hour campaign with positive reserves. |
| Mutable non-finite values could invalidate an entire JSON snapshot | **Resolved.** Snapshot projection sanitizes every mutable float family and a NaN/infinity round trip proves the wire remains decodable. |
| Dormant d20 helper could return 21 | **Resolved.** Rolls clamp to 1–20. |
| New nested wire enums could make an older client appear frozen | **Resolved by explicit versioning.** `protocolVersion` is serialized first; breaking wire changes must bump `PROTOCOL_VERSION`, and the client retains a visibly stale frame with `UPDATE REQUIRED` before nested decode. |
| Target-width `usize` appeared on the wire | **Resolved.** Accounting counts and queue indices are `u32`, with checked native conversions. |
| Trader offer fields rejected older snapshots | **Resolved.** Both offer vectors default empty, with a legacy-payload test. |
| Test acceleration exists in the pure sim | **Accepted harness boundary.** Release server builds reject these actions; the pure sim keeps them for deterministic integration campaigns. |
| Election NaN ordering was unspecified | **Accepted parity boundary.** A golden test intentionally preserves stable TypeScript ordering, while outbound leadership is finite-sanitized. |

## Server review

| Finding | Disposition |
| --- | --- |
| Sessions could mint unbounded villages | **Resolved.** Session issuance and personal villages are capped per effective client IP, and the shared world has a 256-colony hard cap. |
| Pre-auth rate limiting used a connection id; sockets were uncapped | **Resolved.** Direct peer/effective client IP drives an independent action limit and an eight-connection cap. |
| Reverse proxies could either spoof IPs or collapse all users into one bucket | **Resolved.** Only exact configured proxy peers may supply exactly one strict `X-Forwarded-For` IP; all other forwarding headers are ignored and malformed trusted input fails closed. |
| WebSocket frames/messages were effectively unbounded | **Resolved.** Both limits are 64 KiB. |
| Public bind could use a built-in signing secret or unrestricted Origin | **Resolved.** Non-loopback startup requires a real secret and exact Origin allowlist unless an explicit insecure-development opt-in is set. |
| Sessions never expired | **Resolved without orphaning villages.** New v2 sessions carry a signed timestamp plus stable player token and expire for ordinary actions after 30 days. Authentic legacy and v1/v2 credentials upgrade or renew during a seven-day grace window without changing `playerId`; tampered, future-dated, and beyond-grace credentials cannot inherit the old player's villages. |
| Repeated save errors left readiness green | **Resolved.** Three consecutive periodic failures make `/ready` fail; a successful save resets it. |
| Readiness could wait behind world/database locks | **Resolved.** It uses non-blocking lock attempts. |
| Persistence lacked hot indexes/statement caching | **Resolved for measured hot paths.** Colony foreign-key indexes and cached cat/job/building/tile writers are installed. |
| Corrupt JSON gave no actionable location | **Resolved.** Loading stays fail-closed and reports table, row identity, and column for every loaded JSON field. An exhaustive column matrix prevents context-free loaders from returning; automatic mutation/quarantine is intentionally not performed. |
| Hand-maintained tile-resource JSON could drift | **Resolved by exhaustive audit coverage.** Every resource field is pinned across the persistence round trip. |

Two scale redesigns remain optional rather than correctness blockers: dirty-tracked SQLite upserts
instead of atomic full-world checkpoint replacement, and versioned/delta WebSocket snapshots
instead of complete snapshots. The new hard population/connection bounds prevent either path from
being an unbounded input surface. They should be driven by profiling before changing persistence
atomicity or the simple authoritative snapshot contract.

## Client and usability review

| Finding | Disposition |
| --- | --- |
| A disconnected frozen world looked live | **Resolved.** A persistent LIVE/STALE/OFFLINE/UPDATE REQUIRED chip remains visible for the entire transport state. |
| Reconnect text did not count down | **Resolved.** The chip displays the live remaining delay and attempt number. |
| No first-run guidance or shortcut reference | **Resolved.** A dismissible first-run overlay teaches the survival loop and doubles as Help, reachable through its button, H, or Shift-?. |
| Roles were not glanceable | **Resolved.** All four specializations and seven officer roles have distinct glyph/color badges. |
| Research retained dead FUTURE presentation | **Resolved.** The unreachable state and copy were removed. |
| Actions used the hardcoded actor name “Desktop Cat” | **Resolved.** The wire label is the truthful neutral `Idle Cat Forest player`. |
| Snapshot payload was deserialized through a full intermediate JSON DOM | **Resolved.** A verified top-level discriminator leads directly to typed deserialization; small action extension fields are extracted without a DOM. |
| Spending actions lacked confirmations | **No change required.** These are frequent, reversible idle-game actions with visible costs and server feedback; blanket modal confirmation would obstruct normal play. |

The booted client captured and the review inspected its own framebuffer for first-run onboarding,
the live world with role cues, and a retained stale world with a reconnect countdown. Temporary
capture code was removed after verification.

## Workspace and delivery review

| Finding | Disposition |
| --- | --- |
| Crate package metadata drifted from workspace metadata | **Resolved.** All workspace crates inherit the shared package fields. |
| CI used a moving Rust toolchain while the image pinned 1.96 | **Resolved.** `rust-version = 1.96`, both CI jobs, and the image agree. |
| No dependency advisory/source gate | **Resolved.** Forgejo runs cargo-deny advisories, bans, and source checks; local verification passes with narrow documented upstream exceptions. |
| Stale Next/Node build products and screenshots cluttered the tree | **Resolved locally.** The generated directories/files were removed and the Rust-focused ignore file no longer carries the retired Next template. |
| Justified dead-code allowances | **Retained.** Both are narrow, commented compatibility/test helpers rather than blanket lint suppression. |

The final consolidated workspace, WASM, dependency, persistence, generalized campaign, and native
framebuffer gates are recorded in `docs/FIX_LOG.md`.
