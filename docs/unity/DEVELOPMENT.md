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
or the entry scene, close the Editor on this project and run `bash tools/forest.sh setup`
first (a batch run that needs a Unity slot). The setup is idempotent: it creates the scene
only when it is missing, adds the ForestGame object only when none exists, saves only when
it changed something and logs a `forest_setup:` line. It never copies a Library cache.

The initial local shared world has the communal village. Use **Village** to found
your personal village or connect to an explicit shared server. Local state lives
under Unity's application data directory in `unity/world-v1.json`, separate from
the former game's SQLite data. Corrupt or unsupported saves fail visibly.

Use WASD/arrows or right/middle drag to pan. The wheel zooms around the pointer;
+/− keys and the visible zoom buttons work from the management view. One normalized
wheel step changes the scale by about 11%, including fractional trackpad input.
Click a cat or workplace to inspect it. The camera keeps the subject beside the
open drawer. Tab enters or leaves the selected cat; WASD walks, right drag turns the close
camera, and E interacts with nearby storage or the shrine. Colony simulation
continues during direct control. The management categories expose construction,
manual jobs, queues, staffing, study purchases, stores, officers, defense and trade.

In new worlds, **World** lists known dungeon entrances and the selected cat's
readiness. Dispatch an equipped or trained adult, follow its expedition, or recall
it. Page Up / Page Down and the World panel switch discovered levels. Selecting
an explorer follows its next descent; explicitly choosing Surface stops that
follow. In direct control, walk toward a staircase to descend or climb. E attacks
an adjacent guardian or collects a chest at the cat's position. Releasing control
underground starts a physical return to the village.

The generator creates coherent biomes, surface slopes, shallow river crossings,
deep lake shelves and two- or three-floor dungeon sites. Existing saves retain
their older terrain. Use a new save path to test this generation. The system has
no swimming, fluid simulation or arbitrary excavation. Using 3D geometry does
not itself guarantee a lower rendering cost; measured workloads are recorded in
[PERFORMANCE.md](PERFORMANCE.md).

Autonomous movement, work, needs and transport run in 50-millisecond simulation
steps at 1× speed. Pause stops simulated time; 4× and 8× advance more of the same
steps. The inspector updates needs and work progress between its slower content
refreshes. Existing saves acquire the finer clock without resetting their world.

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
close this project's Editor and hold a Unity slot (see below) before running batch
Unity tests:

```sh
bash tools/forest.sh edit-test
bash tools/forest.sh play-test
```

`edit-test` runs the `IdleCatForest.Acceptance.Tests` and
`IdleCatForest.Simulation.Tests` assemblies and excludes the `Campaign` category
(plus, until campaign cases carry that category, names containing `.campaign.`).
`--filter <regex>` narrows it further. Unity test reports go to
`$FOREST_ARTIFACTS/tests/$FOREST_RUN_ID/` (`editmode.xml`, `editmode.log`,
`playmode.xml`). A process exit alone is not a pass: inspect test totals,
failures, runtime errors and generated reports. Tests never contact paid AI
services or production worlds.

## Scenario runner

The rendering-free scenarios compile every `.cs` file under
`unity/Assets/Forest/Tests/EditMode/Acceptance/` except the NUnit adapter
`AcceptanceTests.cs`, together with the simulation sources.

```sh
dotnet run --project tools/scenarios -c Release -- --jobs=0
dotnet run --project tools/scenarios -c Release -- --jobs=0 --filter=regression.
dotnet run --project tools/scenarios -c Release -- --jobs=0 --include-campaigns --filter=campaign.
```

| Option | Meaning |
|---|---|
| `--jobs=N` | Run N cases at once; `0` uses every processor. Default `1` (serial). |
| `--filter=TEXT`, `--filter TEXT` | Case-insensitive substring of the case name. |
| `--include-campaigns` | Also select names starting with `campaign.` (about 31 min serially). |
| `--include-demo-timing` | Also select names starting with `demo_timing.`. |
| `--exclude-campaigns` | Accepted for old scripts; campaigns are excluded by default. |
| `--list` | Print the selected names and exit. |
| `--junit=PATH` | Also write a JUnit XML report. |

Output is buffered per case and printed sorted by name, followed by
`selected=N passed=N failed=N elapsed_ms=N`. A failure prints the exception type,
message and top five stack frames. Exit codes: `0` all passed, `1` a case failed,
`2` unknown or malformed argument, `3` nothing matched the selection.

## Parallel work: compile gate, worktrees and Unity slots

### Compile gate without Unity

```sh
bash tools/forest.sh check
```

`check` builds `tools/unity-check/UnityCheck.slnx`: SDK projects that mirror the
asmdefs (`IdleCatForest.Simulation`, `IdleCatForest.Authority`,
`IdleCatForest.Presentation`, `IdleCatForest.Editor` and
`IdleCatForest.PlayModeTests`) and compile the same sources with C# 9 against
Unity's reference assemblies. It prints `check: ok` or the CS errors and exits
nonzero on failure. Run it before taking a Unity slot. It needs the installed
editor and an imported `unity/Library` (for the Input System, Pipeline, test
runner, NUnit and Newtonsoft assemblies), but never starts Unity:

- `FOREST_UNITY_ROOT` overrides the editor folder
  (default `/Applications/Unity/Hub/Editor/6000.6.0f1`).
- `FOREST_UNITY_LIBRARY` overrides the Library folder (default `unity/Library`).

Presentation is checked as the player compiles it (no `UNITY_EDITOR`); Editor and
PlayMode tests are checked with the editor defines, and Editor against Unity's
`unity-4.8-api` profile. These projects are not in `IdleCatForest.slnx`, because
the solution build must stay green on machines and CI runners without Unity.

### Package worktrees

```sh
bash tools/forest.sh worktree-init <package-id> [base=demo-polish]
```

This creates (or reuses) the git worktree
`<main checkout>-worktrees/<package-id>` on branch `demo/<package-id>` from
`base`, then APFS-clones the main checkout's `unity/Library` into it with
`cp -c -R`, so `check` and batch Unity commands work without a cold import. The
main checkout is resolved from `git rev-parse --git-common-dir`; `FOREST_WORKTREES`
overrides the parent folder.

### Unity slots

At most three batch Unity instances run at once on this machine. Each slot is a
`mkdir` lock in `<main checkout>/artifacts/unity-slots/slot-{1,2,3}` with an
`owner` file recording the package id, worktree path, acquiring pid and time.

```sh
bash tools/forest.sh unity-slot acquire <package-id>   # waits until a slot is free
bash tools/forest.sh unity-slot status
bash tools/forest.sh unity-slot release <package-id>
```

`acquire` polls every 15 s (`FOREST_SLOT_POLL`) for up to 2 h
(`FOREST_SLOT_TIMEOUT`) and is a no-op when the id already holds a slot. `status`
reports slots held for more than 3 h as stale and notes when the acquiring shell
has exited; stale slots are never taken over automatically. Release a slot as soon
as the Unity run ends and hold it at most 45 minutes at a time.

`setup`, `edit-test`, `play-test`, `build`, `capture`, `inspect` and `performance`
refuse to run unless this worktree holds a slot. In the main checkout they also
require `FOREST_INTEGRATOR=1`: that project belongs to the wave integrator.

### Output locations

`FOREST_ARTIFACTS` defaults to `<main checkout>/artifacts`, and `FOREST_RUN_ID`
to `<yyyymmdd-hhmmss>-<pid>`. Test reports go to
`$FOREST_ARTIFACTS/tests/$FOREST_RUN_ID/`, and players to
`$FOREST_ARTIFACTS/macos/<worktree name>/$FOREST_RUN_ID/Idle Cat Forest.app`.

## Scripted captures

```sh
bash tools/forest.sh capture tools/capture/overview.json --out <dir> \
  [--fixture fresh|founding30|expanded150|demo35] [--seed N] [--size 2400x1500] [--rebuild]
```

`capture` needs a held slot. It reuses this worktree's newest player (or builds
one when none exists or `--rebuild` is given), creates the fixture save in a fresh
temporary directory and runs the player windowed at `--size` with
`--forest-save <temp>/world.json --forest-capture-script <script> --forest-capture-out <dir>`.
It waits up to 10 minutes for the player to quit, then deletes the temporary save.

- `fresh` (default) creates a new world from `--seed` (default 4242).
- `founding30` and `expanded150` come from `tools/presentation-fixture` (fixed seed 41).
- `demo35` is reserved; it fails with a message until W4-04 adds it.

The output folder receives one PNG per `shot`, `capture-log.txt` (one line per
step with real and simulated time and the result) and `player.log`. The player
exits `0` when every step succeeded, `1` when a step failed and `2` when the
script is unreadable or contains an unknown step.

A script is a JSON array. A step is either a bare string or an object with one
property:

| Step | Effect |
|---|---|
| `{"wait": 2.5}` | Wait real seconds; the simulation keeps running at the current speed. |
| `{"speed": 4}` | `ForestGame.SetSpeed`. |
| `"focusVillage"` | Frame the selected village (`ForestView.FocusVillage`). |
| `{"zoom": -3}` | `ForestView.ZoomBy(steps)`; positive zooms in. `100` reaches the closest zoom (3.5), `-100` the farthest (36). |
| `{"level": -1}` | `ForestView.ViewLevel(n)`; `0` is the surface. |
| `{"selectCat": "firstWorking"}` | Select a cat by id, `first` (first living cat) or `firstWorking` (falls back to the first living cat). |
| `{"selectBuilding": "research_hut"}` | Select a building by id or by kind. |
| `{"open": "Cats"}` | `ForestUI.OpenPanel(section)`, for example `Cats`, `Research` or `Inspect`. |
| `"close"` | `ForestUI.ClosePanel()`. |
| `{"act": {"Kind": "EnterCatControl", "CatId": "$selectedCat"}}` | Send a `GameAction` through `ForestGame.Act`. The string values `$selectedCat` and `$selectedBuilding` stand for the driver's last selection. |
| `{"shot": "name"}` | Save the composited frame, UI included, as `name.png`. |
| `"quit"` | Stop the script and quit the player. The player also quits at the end of the script. |

Standard scripts live in `tools/capture/`: `overview.json`, `cats-drawer.json`,
`inspector.json`, `direct-control.json`, `underground.json`, `zoom-range.json`
(zoom 3.5, 14 and 36) and `edge.json` (farthest zoom). The driver is
`unity/Assets/Forest/Presentation/Capture/CaptureDriver.cs`; it is inert unless
`--forest-capture-script` is on the command line and uses only public game members.

## Build native macOS

With this project's Editor closed and a Unity slot held:

```sh
bash tools/forest.sh build
```

The command prints the player path,
`$FOREST_ARTIFACTS/macos/<worktree name>/$FOREST_RUN_ID/Idle Cat Forest.app`;
open that path.

The build targets Apple Silicon with IL2CPP. Build products and compiler caches
are ignored. The packaged app accepts `--forest-save <new-or-existing-path>`,
`--forest-seed <integer>` for a newly created world, and
`--forest-server <ws-or-wss-address>`. Existing saves determine their own seed.
Other platforms have no build/test claim in this migration.

New saves use the revised founding layout: a centered 3×3 shrine, surrounding
road, four gates and connected building entrances. Older Unity and imported saves
keep their existing coordinates, footprints, jobs and inventories. Newly planned
buildings in those saves still require a road connection to the actual shrine.
Use a separate unused path to try the new layout without replacing progress:

```sh
open -n "<player path printed by build>" --args --forest-save "$PWD/artifacts/playtest/world.json" --forest-seed 41
```

The same path resumes the same world on subsequent launches. On Retina Macs,
`-screen-width 2400 -screen-height 1500` gives a comfortable management window;
smaller windows keep the text size and scroll their panels.

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

The performance report includes up to 3,600 frames and 50-millisecond simulation
steps, plus steps that also cross a whole-second planning boundary. It records
population, active jobs, resolution, machine and local/remote mode. These are
observations of the current workload. A remote client cannot measure the server's
simulation cost.

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
