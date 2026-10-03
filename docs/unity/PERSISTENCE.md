# Shared-world authority and persistence

The Unity client and the .NET host compile the same C# simulation and authority sources. The host owns time, resource accounting, village permissions, cat control and action results. A client sends input and receives a filtered projection. It cannot send a replacement world.

## Run the shared host

Install .NET SDK 10.0.400 and run from the repository root:

```sh
dotnet restore server/Forest.Server/Forest.Server.csproj --locked-mode
dotnet run --project server/Forest.Server/Forest.Server.csproj --no-restore
```

The default listener is `http://127.0.0.1:8788`, with WebSocket endpoint `/ws`, liveness `/health` and readiness `/ready`. The Unity connection field uses `ws://127.0.0.1:8788/ws`. Remote client connections require `wss://`. Put TLS termination in front of an intentionally public host; the local host does not provision certificates or change infrastructure.

`FOREST_LISTEN` changes the listener. `FOREST_SAVE_PATH` selects the complete versioned world file. The default shared save is under the platform local application data directory at `IdleCatForest/shared/world-v1.json`. Embedded Unity saves use the separate path selected by the client under `Application.persistentDataPath/unity/`.

A public listener requires `SESSION_HMAC_SECRET` through the deployment's secret mechanism. A loopback host without it creates a random private authority key beside the selected save, under `<save>.identity/authority.key`. Keep that directory with the shared world. Replacing the key prevents existing identities from proving ownership. Never paste the key or bearer JSON into logs, issue reports, evidence or commits. The embedded game uses neither the key nor the secret; see [Local identity](#local-identity).

## Embedded save location

The packaged game saves to `Application.persistentDataPath/unity/world-v1.json`. On macOS that is `~/Library/Application Support/Idle Cat Forest/Idle Cat Forest/unity/world-v1.json`. Play mode in the Unity Editor uses the sibling folder `unity-editor/`, so pressing Play never writes the packaged game's world. `--forest-save <path>` opens any other world directly. Tests and captures use temporary paths only.

A save path `world-v1.json` has these neighbours:

| File | Meaning |
|---|---|
| `world-v1.json.lock` | Writer lease, held while the world is open. |
| `world-v1.json.previous` | The file that the latest write replaced. |
| `world-v1.backup.json` | The save as it was when the current session opened it. |
| `world-v1.invalid-<utc>.json` | A world that broke simulation invariants when it was saved; kept for diagnosis. |
| `world-v1.json.identity/session.json` | Optional imported native bearer that names the local owner. |

The game never deletes, rotates or restores these files automatically. A sync tool or Steam Cloud should carry `*.json`, `*.previous` and the `.identity` folder, and skip `*.lock` and `*.tmp-*`.

## State and recovery

The save envelope is `idle-cat-forest-unity`, version 1. It contains the entire authoritative `World` aggregate as JSON plus its SHA-256 checksum. Public model fields include identities, explicit time, random state, needs, cargo, reservations, in-progress jobs, production queues, research, villages and trade. A render snapshot is not a save.

Exact crafted hauling uses the existing job identity, item IDs, source position and phase. A claimed item stays physically at its source during `item_fetch` and continues occupying that source's capacity. After pickup, `output_delivery` retains the item while storage is full. Save/reload preserves both stages, condition and exclusive ownership; completed delivery cannot replay the item. Cancellation before pickup releases the claim at the source, while interruption after pickup leaves cargo at the carrier's actual position.

The continuous simulation persists its last consumed 50-millisecond step separately from requested elapsed time. A fractional save retains the unused portion and resumes without replay. Older saves without this cursor initialize it at their existing clock. New fractional mover coordinates retain a separate last-reached grid position for collision checks. These additions preserve existing layouts, identities, jobs and goods.

Older merchants and raiders stored a whole-tile position plus cadence credit rather than a physical intermediate position. On first advance, they continue from that saved tile and discard the old cadence credit, including credit accumulated while blocked. This prevents a burst of catch-up movement. Existing fractional caravan coordinates remain intact.

Each write creates a private temporary file, flushes it and atomically replaces the destination. The previous complete file remains at `<save>.previous`. The directory is synced on Unix. Opening a world deletes temporary files that a crash left behind for that save, its backup or an invalid copy, and nothing else.

A single-writer lease prevents two authorities from opening the same save: `<save>.lock` is opened with `FileShare.None`, and where the runtime supports it also locked with `FileStream.Lock`, which the IL2CPP player enforces across processes. There is no `flock` and no P/Invoke for it. A second writer fails with `already_running`. Within one process a held lease is refused before the lock file is opened again, because closing any descriptor of that file would drop the process's byte-range lock.

Opening never writes. The first write of a session over a loaded world first copies the save as it was opened to `<name>.backup.json`; a world created in this session never writes a backup, so a fresh world cannot replace the backup of an old one. A missing main save next to an existing `.backup.json` or `.previous` fails with `missing_save_with_backup` instead of creating a new world, also for an explicit new world at that path. Creating a new world at an existing path fails with `already_exists`.

Simulation invariants (`World.Validate`) never block a save or an open. A world that breaks them is still saved, its first such save in a session is also kept as `<name>.invalid-<utc>.json`, and an open lists the failures as warnings for the log. Only the ownership structure (exactly one communal village, unique village identities, one owner per personal village) refuses to open, with `invalid_ownership`. For the same reason a world whose structure broke during play is never written over the save: the write fails with `invalid_ownership`, the invalid copy is still kept, and the save keeps its last good contents.

Every failure to open or create is a `SaveException`, an `IOException` with a stable `Code` from `SaveFailure`: `damaged` (not a save, invalid JSON or checksum mismatch), `made_by_newer_version`, `unsupported_version`, `too_large`, `already_running`, `already_exists`, `missing_save_with_backup` and `invalid_ownership`. The original file is never changed or reset.

To inspect a failed save, stop its authority and work on a copy. To recover, explicitly copy the selected `.backup.json` or `.previous` file to a new save path and open that path. No code automatically rolls back player progress or overwrites the suspected source.

### Loading and migrations

A save is read in three steps: the envelope's `Format`, `Version` and checksum; `SaveMigrations.Migrate(payload, version)` on the payload as JSON; then the typed decode. Saves tolerate members this build does not know: their values are skipped and listed as `Type.Member` in `SaveLoadReport.DroppedMembers` and in the authority's `LoadWarnings`. Wire input from sockets keeps rejecting unknown members.

Rules for every change to the serialized model:

- Add a field only in a new partial file `Simulation/Model/<Type>.<Feature>.cs`; old saves decode it as its initial value. No migration is needed.
- Removing a field loads old saves, but silently loses that data. Move the data first if it still matters.
- Renaming a field, or changing its meaning or type, bumps `SaveStore.Version`, adds the step `Steps[old version]` in `Authority/SaveMigrations.cs` that rewrites the old JSON, and adds a test that loads `server/Forest.Tests/Fixtures/world-v1.main-5ec7278.json.gz` through it. A build that finds a higher version refuses with `made_by_newer_version` instead of guessing.

The fixture is an embedded world written by `main` at `5ec7278`, the build players had before the demo push, after 120 simulated seconds. It is gzip-compressed and synthetic; it holds no identity files.

### Embedded save cadence

The embedded game saves on wall-clock time, never on simulated time: 30 seconds after the last save while the world changed, 2 seconds after a successful player action (also while paused), on focus loss and on quit. It never saves while `ForestGame.Faulted` is set. The world is validated and encoded on the main thread; the envelope, hash, backup and atomic write run on a background task with at most one save in flight. `ForestGame` exposes `SaveState`, `LastSaveUtc`, `LastSaveError`, `SaveWarnings`, `ConsecutiveSaveFailures`, `SaveMilliseconds` and its main-thread part `SaveMainThreadMilliseconds`. `LocalAuthority.RestoreLastGood()` replaces the live world with the last payload the session loaded or wrote.

### Shared host cadence

The server advances explicit simulation time on a 50-millisecond timer. It publishes snapshots at a 100-millisecond target during both autonomous play and direct control. Slow clients cannot delay the simulation clock. Saves run every five seconds; the world is encoded under the simulation lock and written outside it. I/O and write-permission failures leave simulation running and retry at the next save interval. `/ready` fails after three consecutive save failures and recovers after a successful save. Graceful shutdown attempts a final save. An abrupt stop can lose work since the last complete save, but must not expose a partial replacement.

## Identities and permissions

Presence establishes one signed identity for a socket. Every action must use that socket's valid bearer. A server-owned context supplies the player and selected village; values in a client action cannot replace it. A signed player controls the communal village and their own personal village. A foreign village remains inaccessible, with only discovered summary information exposed.

On the shared host, bearer sessions last 30 days and renew within a further seven days without changing the player identity. Authentic historical Rust bearer tokens can upgrade while retaining ownership. A rejected stored bearer is preserved, and the client reports that authentication failed. It does not silently create a replacement owner.

Coordinate construction checks foreign tile claims and physical building, farm and stockpile footprints before reserving inputs and again before work advances. Expansion checks its complete future area. Founding chooses a site whose enclosure, access paths and starter deposits avoid foreign property. Blocked work retains its paid progress and owned goods; traversal across shared land remains available.

The snapshot projection removes private village state, owner identifiers, other player records, undiscovered tiles and authoritative reservations. Stockpile quantities come from Accountant reports. Exact functional equipment is visible only to the selected village's authorized controller while those reports match the physical ledger. Direct cat control remains a simulation action on the same identity and inventory.

The native identity file uses the maintained Rust shape `sessionId`, `sig`, `selectedColonyId`, `nickname`, `textScale`. `CredentialStore.ImportLegacy(source, destination)` copies an explicitly selected source to a new protected destination. It never prints credentials, changes the source or overwrites a destination. Token renewal and village selection preserve nickname and text scale. The old native default was `~/.config/idle-cat-forest/session.json`, with `XDG_CONFIG_HOME` and `CAT_CLIENT_SESSION_PATH` overrides. Do not automatically scan home directories for identity files.

The protocol version is 3. It uses request IDs for action results and complete authorized snapshots. Each projection receives an increasing sequence under the authority lock. Clients discard older or duplicate projections while still completing their correlated action requests, so delayed broadcasts cannot undo a newer village selection or cat-control handoff. Rate-limited requests receive their correlated rejection without constructing or sending another world projection. The client retains its latest frame and continues receiving scheduled snapshots. Sequences last for one authority process; reconnecting creates a fresh client receiver. Invalid JSON, duplicate JSON properties, non-finite numbers, oversized requests and development-only actions are rejected. Native clients have no Origin header; browser clients require an exact entry in `FOREST_ALLOWED_ORIGINS`. Forwarded identity or IP headers are not trusted.

Ordinary actions are limited to 30 per socket and signed player per ten seconds, and 120 per address. Signed movement has a separate allowance of 120 per socket and player, and 800 per address, over the same period. Reconnecting does not reset the player allowance. A connection can establish only one identity, with eight sockets per address and eight new identities per address per hour. Creating villages stops at 256. Control heartbeats run every real second, independently of input focus. They use the ordinary action budget and only renew the current controller's lease.

### Local identity

The embedded game has no bearer session, so nothing expires. Its local player id is stored in the world as the first entry of `World.Players`. A long break, a clock moved forward or back, `SESSION_HMAC_SECRET` in the environment or a missing `.identity` folder therefore never changes who owns the world. When `<save>.identity/session.json` exists, for example an imported native bearer, the player id it names becomes the local owner; the embedded game reads only that id and checks neither signature nor age. An unreadable identity file is ignored with a warning. The embedded game writes no identity files and creates no authority key.

## Import a legacy SQLite world

The maintained Rust save is SQLite, normally `data/cat.db`, selected by `GAME_DB_PATH`. Its supported tables and migrations are frozen in `tools/save-import/legacy/persistence.rs`. In-flight state also lives inside JSON columns. The compatibility directory contains the original library sources and has no runnable game server or client.

Select an explicit backup copy and an existing private destination directory. The normalizer requires Rust 1.96 or newer and a native C compiler for bundled SQLite; the verified toolchain is Rust 1.98.0. Build the compatibility normalizer and C# converter, then write two new files:

```sh
cargo build --locked --manifest-path tools/save-import/normalize-legacy/Cargo.toml
dotnet restore tools/save-import/Forest.SaveImport/Forest.SaveImport.csproj --locked-mode
dotnet build tools/save-import/Forest.SaveImport/Forest.SaveImport.csproj --no-restore
tools/save-import/normalize-legacy/target/debug/forest-normalize-legacy /explicit/copy/cat.db /new/private/normalized.json
dotnet tools/save-import/Forest.SaveImport/bin/Debug/net10.0/Forest.SaveImport.dll /new/private/normalized.json /new/private/world-v1.json
```

The normalizer opens the source read-only, takes an SQLite online backup into memory, checks integrity and the supported schema, and applies the maintained migrations to the in-memory copy. Unknown tables or columns fail closed. Its derived data includes exact palisade edges and physical per-resource storage capacities. The C# converter checks references, ownership and simulation invariants before creating a private versioned Unity world. Existing destinations are refused.

The conversion keeps village and cat identities, needs, ages, lineage, pregnancy, stats, skill experience, assigned work, queues, finite item condition and location, cargo, construction contracts, reservations, research, farm progress, migration deadlines, accounting reports and rounds, traders, transport, elections and inter-village escrow. Colony-local cat coordinates are converted to world coordinates. Stations and carried cargo are moved out of the legacy mirror ledgers once, without creating a second copy. A caravan already returning does not redeliver its outward cargo.

Imported per-resource limits count exact Tools, Weapons and Armor as physical items, including claimed items awaiting pickup. Mixed station output batches deliver one item or scalar stack at a time after checking that output's filter and receiving capacity. Remaining outputs stay owned by the unfinished job across blocking and restart. Freeing a slot through a public equipment action permits the next delivery without duplicate scalar counters or changed item condition.

Imported exact outputs also survive when their station has no worker or queued recipe. Public staffing sends a worker to the station before adopting the output into one exclusive delivery job. Additional workers cannot claim it again. Full receiving storage retains the remaining item identities and condition through restart.

The same pickup requirement applies to preassigned workers. A blocked station cannot transfer its outputs into a distant worker's job. Cargo already leaving the station continues to storage separately, while leftover outputs remain at the station. Incoming recipe cargo retains its station destination, delivered inputs and work progress even when finished goods are waiting there.

Legacy `station-out` markers name a destination, so conversion never subtracts their cargo from existing destination stock. Ordinary Tools, Weapons and Armor use unsuffixed markers with exact carrier-owned items. Variant goods use an `item:` suffix and a Refined compatibility counter. The converter retains the exact identities and removes only their scalar representation. Station-resident functional output counters receive the same reconciliation; unrelated goods, equipped items and station leftovers stay with their existing owners.

Imported farms retain their 24-hour biological work cycle, fertility effect and crop yields, so a mature saved field cannot immediately harvest under the shorter cycle used by newly planted Unity farms. A saved frontier expansion completes only its original tile and derived boundary. Concurrent leadership elections and vote-kick petitions retain their separate ballots and deadlines.

Thirteen removed building-capacity studies receive their exact original research-point cost as a refund, recorded in the event log. Legacy recipe-entitlement version zero retains its grandfathered recipe access. A funded scaffold without a historical material contract stays funded. Site-less construction keeps its type and queued work until a valid site can be resolved. An orphan transit ledger becomes a finite recovery pile at its saved position with an event explaining the recovery. Unsupported identities or system shapes are refused before the Unity destination is installed.

Start the shared host with `FOREST_SAVE_PATH` selecting the new world, or launch the packaged app with `--forest-save /new/private/world-v1.json`. To retain a historical private-village owner on the shared host, provide the original authority HMAC secret through the same approved environment injection used by the Rust deployment; only the shared host reads `SESSION_HMAC_SECRET`. The embedded game needs only the imported bearer below. World conversion never exports, reads or replaces that secret.

Import the explicitly selected native bearer to the new installation's credential path separately:

```sh
dotnet tools/save-import/Forest.SaveImport/bin/Debug/net10.0/Forest.SaveImport.dll --credential /explicit/old/session.json /new/private/world-v1.json.identity/session.json
```

That example is the embedded authority's credential path; the embedded game takes the owner's player id from it. Shared-client credentials live at `Application.persistentDataPath/unity/servers/<key>.json`, where `<key>` is the lowercase SHA-256 hex digest of the server URI's `GetLeftPart(UriPartial.Authority)`. Import to that new destination before connecting. On the shared host, the original signer and valid bearer are both needed to preserve ownership. A missing signer cannot safely be inferred from a world or nickname. Keep the original SQLite file, signer configuration and bearer backup until the converted world has been inspected and saved successfully.

The importer transfers durable state into the Unity simulation. Future simulation ticks follow the C# game rules and are not a bit-for-bit replay of the old engine's random event history.

The read-only archive command is:

```sh
python3 tools/save-import/legacy_archive.py /explicit/copy/cat.db /new/private/legacy-world.json
```

The archive keeps every table, column, row, SQLite row ID and SQL schema declaration. JSON columns remain unchanged strings. It reads one SQLite transaction, validates the recognized table inventory, writes a protected new destination and refuses overwrite. It does not read the separate bearer or authority key.

This optional lossless archive is marked `PlayableUnitySave: false`. Use the typed conversion commands above to create a playable Unity save. The archive can preserve additional columns in recognized tables, but rejects unknown tables; it is not a general exporter for arbitrary future schemas.

## Verification

```sh
cargo build --locked --manifest-path tools/save-import/normalize-legacy/Cargo.toml
dotnet restore server/Forest.Tests/Forest.Tests.csproj --locked-mode
dotnet run --project server/Forest.Tests/Forest.Tests.csproj --no-restore
python3 tools/save-import/test_legacy_archive.py
python3 tools/save-import/test_playable_import.py
```

On 2026-10-03 the live-simulation server executable (`bash tools/forest.sh server-test`) passed 79 authority/import tests with zero failures. Eighteen `persistence:` cases cover the embedded world: credentials issued 90 days ago or a day in the future, a missing `.identity` folder, `SESSION_HMAC_SECRET` on and off, unknown members, the `main` fixture, invariant failures, broken ownership during play, a missing main save beside a backup, the second writer, stale temporary files, no write on open, the session backup, `RestoreLastGood`, every failure code, migrations and a background write racing the simulation. The other cases exercise signed identity expiry and renewal, corruption and overwrite refusal, protected native-bearer import, complete aggregate reload, the writer lease and real loopback clients with two identities, private villages, cat control and restart. The signed socket test rejects foreign coordinate construction without changing terrain, jobs or reservations, then accepts construction on owned land. An automatically ticking host also runs through a test relay that deliberately delivers an older broadcast after a newer action projection, checking selection and both directions of cat-control handoff. Typed conversion scenarios continue inbound station work, outbound exact equipment, construction reservations, migration, farms, accounting, weighted elections, loaded transport and returning barter after a save/reload boundary.

The Mac permission regression removes write access from a synthetic host's save directory. It first reproduced the automatic tick task stopping after its first failed save. After the correction, ticks continue, three failed saves make `/ready` return 503, and the last durable file remains byte-identical. Restoring permission allows the next save to persist the advanced world with the same founding cat identities and restores readiness. Temporary permissions and files are restored and removed after the test.

Two real socket bursts exceed the ordinary action budget. With scheduled snapshots disabled, each rejection preserves the exact client frame, and the next public projection sequence proves that rejected requests did not construct hidden projections. With automatic ticks enabled, the throttled client continues receiving new simulation snapshots. A second identity can still perform accepted actions. The first case failed before the early rejection response was added.

The scalar-haul restart test saves a carrier with eight Logs in `input_delivery` after a public avoid zone blocks its original destination. Loading preserves that saved target, job and cat. The same carrier then physically delivers to another reachable store while the obstruction remains. A second reload preserves the completed transfer without replay or stale claims.

The exact crafted-haul test saves and reloads before pickup, after pickup, while receiving storage is full and after delivery, checking one unchanged identity throughout. Three imported equipment cases enforce separate one-item limits for Tools, Weapons and Armor. A mixed three-item station-output case checks partial delivery, per-kind capacity, filter rejection, blocked restart, public capacity release and eventual completion. Two further cases assign one or two workers to an imported station with exact outputs and an empty queue. They verify physical pickup, exclusive ownership, full storage, public equipment use to free capacity and final delivery through restart. Four preassigned-worker cases cover blocked station access, reopening the route, actual outbound scalar and exact cargo, and inbound recipe continuation beside pending output. The completed log is `artifacts/tests/outbound-formats-authority.txt`; it records all case results and zero failures. The real SQLite conversion pipeline also passed all four cases after this importer correction in 23.147 seconds.

The pipeline test invokes the real Rust writer to create a synthetic 30-cat SQLite world. It tests the current schema and an older schema with missing additive columns, verifies exact station cargo and derived geometry, checks source-byte preservation and private destination permissions, and proves that unknown schemas and existing destinations are refused. All fixtures are temporary; no test searches for or reads player data. Broader gameplay acceptance is tracked separately from these persistence scenarios.

Six focused outbound cases additionally cover destination-stock conservation, multiple exact Tools/Weapons/Armor, and Mug/Trinket compatibility cargo through full storage, public capacity release and restart. The Rust fixture's `station-output` variant uses the actual persisted marker formats for Planks, all three functional item kinds and a Trinket. The pipeline compares raw SQLite contents with normalized and saved state, then runs deterministic continuation. The fixture source is tracked at `normalize-legacy/src/bin/synthetic-fixture.rs` and builds with the documented Cargo command.

Fractional clock tests compare complete authoritative state through partitions and restart without replaying old work. A failed-path test also persists the blocked start, destination and retry deadline at 0.075 seconds. Clearing the obstacle preserves the deadline; the same cat resumes physical movement at the next second and keeps its job and eight Logs through a second restart.

Waiting caravan unloading checks receiving storage only on planning boundaries. Two real save tests fence off storage with free capacity, verify one search per planning interval, and compare full state across fractional partitions and restart. Reopening access delivers scalar or exact-item payment once, retaining item identity, quality, condition and ownership through a second restart.
