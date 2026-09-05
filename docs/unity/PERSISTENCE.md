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

A public listener requires `SESSION_HMAC_SECRET` through the deployment's secret mechanism. Local use creates a random private authority key beside the selected save, under `<save>.identity/authority.key`. Keep that directory with the world. Replacing the key prevents existing identities from proving ownership. Never paste the key or bearer JSON into logs, issue reports, evidence or commits.

## State and recovery

The save envelope is `idle-cat-forest-unity`, version 1. It contains the entire authoritative `World` aggregate as JSON plus its SHA-256 checksum. Public model fields include identities, explicit time, random state, needs, cargo, reservations, in-progress jobs, production queues, research, villages and trade. A render snapshot is not a save.

Each write creates a private temporary file, flushes it and atomically replaces the destination. The previous complete file remains at `<save>.previous`. The directory is synced on Unix. A single-writer file lease prevents two authorities from opening the same save. Creating a new world at an existing path fails. Unsupported versions, corrupt JSON, checksum mismatch and invalid ownership fail without resetting the world.

To inspect a failed save, stop its authority and work on a copy. To recover the previous file, explicitly copy the selected `.previous` file to a new save path and start the host with that path. No code automatically rolls back player progress or overwrites the suspected source.

The server advances explicit simulation time every 100 milliseconds. It publishes snapshots at a 100-millisecond target while a cat is controlled and a 500-millisecond target otherwise. Slow clients cannot delay the simulation clock. Saves run every five seconds. `/ready` fails after three consecutive save failures. Graceful shutdown attempts a final save. An abrupt stop can lose work since the last complete save, but must not expose a partial replacement.

## Identities and permissions

Presence establishes one signed identity for a socket. Every action must use that socket's valid bearer. A server-owned context supplies the player and selected village; values in a client action cannot replace it. A signed player controls the communal village and their own personal village. A foreign village remains inaccessible, with only discovered summary information exposed.

Bearer sessions last 30 days and renew within a further seven days without changing the player identity. Authentic historical Rust bearer tokens can upgrade while retaining ownership. A rejected stored bearer is preserved, and the client reports that authentication failed. It does not silently create a replacement owner.

The snapshot projection removes private village state, owner identifiers, other player records, undiscovered tiles and authoritative reservations. Stockpile quantities come from Accountant reports. Exact functional equipment is visible only to the selected village's authorized controller while those reports match the physical ledger. Direct cat control remains a simulation action on the same identity and inventory.

The native identity file uses the maintained Rust shape `sessionId`, `sig`, `selectedColonyId`, `nickname`, `textScale`. `CredentialStore.ImportLegacy(source, destination)` copies an explicitly selected source to a new protected destination. It never prints credentials, changes the source or overwrites a destination. Token renewal and village selection preserve nickname and text scale. The old native default was `~/.config/idle-cat-forest/session.json`, with `XDG_CONFIG_HOME` and `CAT_CLIENT_SESSION_PATH` overrides. Do not automatically scan home directories for identity files.

The protocol version is 3. It uses request IDs for action results and complete authorized snapshots. Each projection receives an increasing sequence under the authority lock. Clients discard older or duplicate projections while still completing their correlated action requests, so delayed broadcasts cannot undo a newer village selection or cat-control handoff. Sequences last for one authority process; reconnecting creates a fresh client receiver. Invalid JSON, duplicate JSON properties, non-finite numbers, oversized requests and development-only actions are rejected. Native clients have no Origin header; browser clients require an exact entry in `FOREST_ALLOWED_ORIGINS`. Forwarded identity or IP headers are not trusted.

Ordinary actions are limited to 30 per socket and signed player per ten seconds, and 120 per address. Signed movement has a separate allowance of 120 per socket and player, and 800 per address, over the same period. Reconnecting does not reset the player allowance. A connection can establish only one identity, with eight sockets per address and eight new identities per address per hour. Creating villages stops at 256. Control heartbeats run every real second, independently of input focus. They use the ordinary action budget and only renew the current controller's lease.

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

Imported farms retain their 24-hour biological work cycle, fertility effect and crop yields, so a mature saved field cannot immediately harvest under the shorter cycle used by newly planted Unity farms. A saved frontier expansion completes only its original tile and derived boundary. Concurrent leadership elections and vote-kick petitions retain their separate ballots and deadlines.

Thirteen removed building-capacity studies receive their exact original research-point cost as a refund, recorded in the event log. Legacy recipe-entitlement version zero retains its grandfathered recipe access. A funded scaffold without a historical material contract stays funded. Site-less construction keeps its type and queued work until a valid site can be resolved. An orphan transit ledger becomes a finite recovery pile at its saved position with an event explaining the recovery. Unsupported identities or system shapes are refused before the Unity destination is installed.

Start the shared host with `FOREST_SAVE_PATH` selecting the new world, or launch the packaged app with `--forest-save /new/private/world-v1.json`. To retain a historical private-village owner, provide the original authority HMAC secret through the same approved environment injection used by the Rust deployment. Both embedded and shared authorities accept `SESSION_HMAC_SECRET`. World conversion never exports, reads or replaces that secret.

Import the explicitly selected native bearer to the new installation's credential path separately:

```sh
dotnet tools/save-import/Forest.SaveImport/bin/Debug/net10.0/Forest.SaveImport.dll --credential /explicit/old/session.json /new/private/world-v1.json.identity/session.json
```

That example is the embedded authority's credential path. Shared-client credentials live at `Application.persistentDataPath/unity/servers/<key>.json`, where `<key>` is the lowercase SHA-256 hex digest of the server URI's `GetLeftPart(UriPartial.Authority)`. Import to that new destination before connecting. The original signer and valid bearer are both needed to preserve ownership. A missing signer cannot safely be inferred from a world or nickname. Keep the original SQLite file, signer configuration and bearer backup until the converted world has been inspected and saved successfully.

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

The server executable tests exercise signed identity expiry and renewal, corruption and overwrite refusal, protected native-bearer import, complete aggregate reload, the writer lease and real loopback clients with two identities, private villages, cat control and restart. An automatically ticking host also runs through a test relay that deliberately delivers an older broadcast after a newer action projection, checking selection and both directions of cat-control handoff. Typed conversion scenarios continue inbound station work, outbound exact equipment, construction reservations, migration, farms, accounting, weighted elections, loaded transport and returning barter after a save/reload boundary.

The pipeline test invokes the real Rust writer to create a synthetic 30-cat SQLite world. It tests the current schema and an older schema with missing additive columns, verifies exact station cargo and derived geometry, checks source-byte preservation and private destination permissions, and proves that unknown schemas and existing destinations are refused. All fixtures are temporary; no test searches for or reads player data. Broader gameplay acceptance is tracked separately from these persistence scenarios.
