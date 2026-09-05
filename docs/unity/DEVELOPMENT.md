# Unity development

`ACCEPTANCE.md` records the migration gates and observed results. Unity is the
playable application. Frozen Rust libraries support the explicit legacy import
and catalog export utilities; the former applications remain in Git history.

## Installed and pinned tools

- Unity Editor 6000.6.0f1, Apple Silicon, including Mac Build Support IL2CPP.
- Unity CLI 1.0.0-beta.6 with project package `com.unity.pipeline` 0.6.0-exp.1.
- .NET SDK 10.0.400 for the shared-world host and rendering-free scenarios.
- Blender 5.2.1 LTS for reproducible authored geometry.

Unity resolves its package graph from `unity/Packages/manifest.json` and
`packages-lock.json`. The game has no runtime AI service dependency. The built-in
renderer, a directional light and shared low-poly FBX geometry target the available
Mac. Unity's Input System handles input; authoritative movement is simulation grid
movement, not physics or NavMesh prediction. A small utility director and explicit
job executor keep resource claims outside any planner's predicted effects.

## Open and run

Open the repository's `unity` directory in Unity Hub, or run:

```sh
bash tools/forest.sh open
```

If the prerelease CLI returns without opening an Editor, open the folder through
Unity Hub. The verified direct executable fallback on this Mac is:

```sh
/Applications/Unity/Hub/Editor/6000.6.0f1/Unity.app/Contents/MacOS/Unity -projectPath "$PWD/unity"
```

For projects below `/tmp`, macOS reports the canonical `/private/tmp` path to
Pipeline. Use the project path returned by `unity status` for live commands.

Open `Assets/Scenes/Forest.unity` and press Play. If regenerating project settings
or the entry scene, stop Play mode and run `bash tools/forest.sh setup` first.
The setup uses Editor APIs and saves the scene; it never copies a Library cache.

The initial local shared world has the communal village. Use **Village** to found
your personal village or connect to an explicit shared server. Local state lives
under Unity's application data directory in `unity/world-v1.json`, separate from
the former game's SQLite data. Corrupt or unsupported saves fail visibly.

Use WASD/arrows to pan and the wheel to zoom. Click a cat or workplace to inspect
it. Tab enters or leaves the selected cat; WASD walks, right drag turns the close
camera, and E interacts with nearby storage or the shrine. Colony simulation
continues during direct control. The management categories expose construction,
manual jobs, queues, staffing, study purchases, stores, officers, defense and trade.

## Host and test

```sh
bash tools/forest.sh server
bash tools/forest.sh server-test
```

The host binds loopback by default. Use the server's documented explicit options
for another save or binding; public binding requires configured authentication.
Remote clients require `wss`, while loopback `ws` is supported for local testing.
See `PERSISTENCE.md` for identity, permissions, save and legacy-import details.

Run the smallest focused scenario during implementation. After implementation,
close this project's Editor before running batch Unity tests:

```sh
bash tools/forest.sh edit-test
bash tools/forest.sh play-test
```

Unity test reports go to ignored `artifacts/tests`. A process exit alone is not a
pass: inspect test totals, failures, runtime errors and generated reports. Tests
never contact paid AI services or production worlds.

## Build native macOS

With this project's Editor closed:

```sh
bash tools/forest.sh build
open 'artifacts/macos/Idle Cat Forest.app'
```

The build targets Apple Silicon with IL2CPP. Build products and compiler caches
are ignored. The packaged app accepts `--forest-save <new-or-existing-path>`,
`--forest-seed <integer>` for a newly created world, and
`--forest-server <ws-or-wss-address>`. Existing saves determine their own seed.
Other platforms have no build/test claim in this migration.

Install the native support module with Unity Hub, or with the pinned CLI:

```sh
unity install-modules --editor-version 6000.6.0f1 --module mac-il2cpp --architecture arm64 --yes --format json
```

For an isolated review world, use the documented finite-supply generator in
`tools/presentation-fixture/README.md`. It refuses existing save and identity
paths. Never use a production save for screenshots or scenario setup.

Pass `--forest-evidence <directory>` to enable game-only evidence capture and a
`performance.json` sample every ten seconds. F9 captures one composited PNG; F10
captures forty frames at roughly five frames per second. Capture folders use a
timestamp and do not contain credentials. Capture can briefly slow rendering, so
record performance before capturing or after its samples leave the rolling window.
Without this explicit option, the app does not write capture files.

The performance report includes up to 3,600 frames and simulation steps, complete
one-second economy tick samples, population, active jobs, resolution, machine and
local/remote mode. These are observations of the current workload. A remote
client cannot measure the server's simulation cost.

## Live agent inspection

Pipeline is project-local; no additional MCP is required. Always supply the
project path if several Editors are running. Discover command schemas with
`unity command --project-path unity --format json`.

```sh
bash tools/forest.sh inspect
bash tools/forest.sh performance
unity command forest_art_audit --project-path unity --format json
unity command recompile --project-path unity --format json
unity command recompile_status --project-path unity --format json
```

Stop Play mode before editing C# and reenter after successful compilation.
Assembly reload disposes local connections; never interpret a stale Game view as
a current test. `recompile_status` has a nested `failed` result which must be
checked even if the CLI envelope says the command itself succeeded.

Capture the composited Game view to include the UI, and inspect the actual
packaged window as well. A screenshot is visual evidence; the simulation and
authority tests establish resource accounting and permissions.

Art sources, export commands, units, pivots, licensing and triangle counts are
documented in `source-art/README.md`. Blender shader appearance is checked again
after Unity imports the standard materials.
