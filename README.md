# Idle Cat Forest

A non-commercial forest colony game. Cats gather, carry, build, learn, rest and
raise kittens while you decide how the village develops. Specialist officers
gradually take over work that begins under manual control.

The game runs in Unity with a C# simulation and an Apple Silicon macOS application.
The [migration acceptance ledger](docs/unity/ACCEPTANCE.md) records gameplay,
save compatibility, native UI checks and measured performance.

## Play on macOS

Install Unity Editor **6000.6.0f1 for Apple Silicon** with **Mac Build Support
IL2CPP**. Open the repository's `unity` folder in Unity Hub, open
`Assets/Scenes/Forest.unity`, and press Play.

With Unity CLI **1.0.0-beta.6** installed:

```sh
bash tools/forest.sh open
```

To build the native application, close this project's Editor first:

```sh
bash tools/forest.sh build
open 'artifacts/macos/Idle Cat Forest.app'
```

The project pins Unity Pipeline **0.6.0-exp.1** and its package graph. The native
build uses ARM64 IL2CPP. Browser and other native exports are not verified targets.

## Controls

- WASD or arrows pan the management camera. The wheel zooms; middle drag pans.
- Click a cat or workplace to inspect it. The top navigation opens management tools.
- Use Build to choose a plan, then click a clear known site. Cats must carry its materials.
- Assign a worker and edit station or individual work queues in the workplace inspector.
- Research offers searchable cards and a dependency map for all 487 studies.
- Inspect a cat and press Tab to enter third-person control. WASD walks, right drag
  turns the camera, E interacts, and Tab returns to management. Inventory, needs
  and identity belong to the same authoritative cat throughout.
- Village creates a personal village, selects the Commons, saves locally or
  connects to a shared server. The other villages keep simulating.

Inventory marked with an asterisk is a physical Accountant report, which can be
stale. The game does not pretend an unvisited pile has a fresh exact count.
Action failures appear at the bottom of the window.

## Shared world and saves

The local app embeds the same authority used by the server. Install **.NET SDK
10.0.400** to run a separate shared world:

```sh
dotnet restore IdleCatForest.slnx --locked-mode
bash tools/forest.sh server
```

The default server listens on loopback at `ws://127.0.0.1:8788/ws`. The Village
panel connects to it. Personal villages belong to signed identities; the communal
village is shared. Remote connections require secure WebSockets except on loopback.

Local Unity saves are separate from the old game's data. Existing worlds are
loaded, never replaced because loading failed. See the tested
[save and identity migration procedure](docs/unity/PERSISTENCE.md) before importing
a maintained legacy SQLite world. Keep its original database, signer and identity
files intact. Import writes a new destination.

## Development and verification

```sh
dotnet build IdleCatForest.slnx --no-restore
bash tools/forest.sh server-test
dotnet run --project tools/scenarios/Forest.Scenarios.csproj
bash tools/forest.sh edit-test
bash tools/forest.sh play-test
```

Close this project's Editor before batch Unity tests. The
[development guide](docs/unity/DEVELOPMENT.md) explains setup, inspection, build
commands and isolated scenario saves. The [testing guide](docs/TESTING.md) explains
focused checks, long campaigns, normal UI verification and performance evidence.

Game rules live in `unity/Assets/Forest/Simulation`, independently of Unity.
The .NET host and scenarios compile that same source. Rendering and input live in
`Presentation`; signed identity and persistence live in `Authority`. Read the
[architecture](docs/ARCHITECTURE.md) and [game vision](docs/GAME_VISION.md) before
changing behavior.

Most custom models were authored in Blender. Editable sources and reproducible
FBX export, import and geometry checks are in [source-art](source-art/README.md).
No external AI service is required to play or test. Package and asset provenance
is recorded in [third-party sources](docs/unity/THIRD_PARTY.md).

The former TypeScript game is historical on `archive/web-game`. The Rust/Bevy
implementation at the migration base remains available in Git history. A frozen
Rust library and SQLite loader under `tools/save-import/legacy` support legacy
import and catalog export; they are not another playable application.
