# Idle Cat Forest architecture

Unity renders the forest and provides management and third-person cat control.
The same C# simulation runs inside the native app or an independent shared-world
server. The migration acceptance ledger is [unity/ACCEPTANCE.md](unity/ACCEPTANCE.md).
The game is non-commercial.

## Components

| Path | Responsibility |
| --- | --- |
| `unity/Assets/Forest/Simulation` | Plain C# world, cats, catalogs, jobs, resource claims and player actions |
| `unity/Assets/Forest/Authority` | Signed identity, permissions, client projection, atomic persistence and WebSocket client |
| `unity/Assets/Forest/Presentation` | Unity models, cameras, input and UI Toolkit management |
| `unity/Assets/Forest/Editor` | Entry scene, native build, inspection and measured performance commands |
| `server/Forest.Server` | ASP.NET shared-world host |
| `server/Forest.Tests` | Real sockets, identity, save, restart and migration verification |
| `tools/scenarios` | Rendering-free scenarios shared with Unity EditMode tests |
| `tools/save-import` | Read-only conversion of maintained legacy SQLite saves |
| `source-art` | Editable Blender geometry and reproducible FBX export |

The .NET projects compile the simulation and authority source from Unity's Assets
directory. There is one implementation of game rules. Unity assembly definitions
keep simulation independent of the engine, rendering, network I/O and clocks.

## Time and decisions

`World.Step(seconds)` receives explicit time. Seeded random state, stable IDs,
job phases and reservations belong to the world. Autonomous and controlled movement,
needs, work, farming and transport advance in fixed 50-millisecond simulation steps.
Each cat shares one time budget between movement and work. Rates are measured per
simulated second, so shorter steps do not multiply production or consumption.
Planning and ecology retain their slower schedules. Different input partitions
must produce the same state at the same simulation boundary.

The save retains both requested elapsed time and the last consumed simulation
step. Loading a partial step continues its remaining time; older saves begin from
their existing clock without replaying elapsed work. Moving vehicles, merchants,
caravans and raiders retain fractional coordinates plus their last reached grid
position, which remains the anchor for route and boundary checks.

The Leader creates bounded primitive survival and scouting work. Specialist
officers create work in their researched categories. Jobs execute physical travel,
pickup, work and delivery; choosing a plan does not credit its expected output.
Cat needs can suspend work, and cancellation, death and direct control must release
claims or leave owned cargo at a recoverable physical location.

The catalogs contain 25 buildings, 108 recipes and 487 studies. Catalog data alone
does not implement their effects. Named scenarios test physical production,
capacity, service modifiers, resource stages and public unlocks. See
[unity/GAMEPLAY_ACCEPTANCE.md](unity/GAMEPLAY_ACCEPTANCE.md).

## Authority and persistence

A local game embeds `LocalAuthority`. A remote game sends the same actions through
`WorldClient`; the server validates the signed connection, selected village,
action limits and permissions. The communal village is shared, while personal
villages belong to stable identities. Founding and joining do not grant access to
another player's private inventory or control.

Client frames contain the permitted projection. Accountant reports remain
historical physical counts; rendering must not expose canonical inventory as a
current count. The server owns movement, control leases, goods, trades and routes.
Unity physics and camera input cannot bypass those rules.

Versioned saves retain the full authoritative world with a checksum and atomic
replacement. Credential files are separate and protected. Unsupported or damaged
data fails visibly and is never replaced with a fresh colony. Legacy import reads
SQLite through the maintained loader and writes a new destination. Detailed
format, key and continuation rules live in
[unity/PERSISTENCE.md](unity/PERSISTENCE.md).

## Rendering and operation

The management camera is orthographic over an actual 3D world. Direct control uses
a replaceable third-person camera following the same cat ID, needs, cargo and
authority. Other cats continue working. Presentation interpolates authoritative
positions and animates authored cat parts without moving simulation state.

The built-in renderer uses shared Blender-authored meshes, simple materials, one
directional light and bounded shadows. Terrain rendering reads discovered state;
it must never generate authoritative tiles. Asset import, pivots, axes and
rendering cost are checked in Blender and Unity.

[unity/DEVELOPMENT.md](unity/DEVELOPMENT.md) documents opening, running, building,
inspection and tests. Apple Silicon macOS is the required native target. Other
platforms are possible but require their own builds and verification.
