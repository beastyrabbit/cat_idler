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

Use WASD/arrows or right/middle drag to pan. The wheel zooms around the pointer;
+/− keys and the visible zoom buttons work from the management view. One normalized
wheel step changes the scale by about 11%, including fractional trackpad input.
Click a cat or workplace to inspect it. The camera keeps the subject beside the
open drawer. In management view, click farm soil to open that exact plot's Work
card with its current worker, assignment and clearing controls. Choose a named
worker beside the plot, then Assign selected cat. This choice stays synchronized
with Work's top chooser and its other farm cards. The empty choice refuses farm
assignment; an ordinary busy worker can be reassigned through the same control.
Compact farm cards keep short status text and actions together. Long text or a
smaller window can still require scrolling; the jump leaves ordinary scrolling
free. Cats and buildings take priority when they occupy
the clicked tile. Tab enters or leaves the selected cat; WASD walks, right drag turns the close
camera, and E interacts with nearby storage or the shrine. Colony simulation
continues during direct control. The management categories expose construction,
manual jobs, queues, staffing, study purchases, stores, officers, defense and trade.

Stores has fixed **Inventory** and **Equipment** destinations above its scrolling
contents. Inventory keeps **Designate general stockpile** beside those tabs while
the resource ledger scrolls. It starts the existing two-corner tool; choose
opposite corners inside the village or press Esc to cancel. Equipment hides
this action. Resource-filtered designation remains below Inventory's ledger.
The cat inspector's **Choose equipment for this cat** opens Equipment
with that exact cat selected. Worn gear and return controls appear before work
preferences. Defense opens Equipment; workshop reports and hauling open Inventory.

Inventory exposes the carrier choice used in Work. **Haul from pile** honors
that named cat; a busy cat refuses instead of receiving an automatic substitute.
Use **Choose available cat** for ranked automatic assignment. Hauling can
pick up from a reachable tile of the source footprint when its anchor is blocked.
Closing every source tile holds the same claim until access reopens; pickup
still requires the cat's physical arrival. Hauling and ordinary work delivery
also use reachable tiles of the destination footprint. A closed destination
holds the load and retries; reopening a tile permits delivery. Filters and
capacity still apply at arrival, with another accepting store used when needed.
Workshop output preflight uses the same footprint access. Its storage warning
asks for space and a reachable route before a batch or during delivery.
Preflight checks researched output up to one carrying load. A 4.4-Food batch
needs room for 4.4 before its ingredients are claimed; an 8.8-Flour batch can
start with room for its first eight-unit load and deliver the remainder on a
later trip. Capacity and filters are checked again on physical delivery.
After output delivery physically resumes, Work and the worker controls clear
that old warning; retry waits and genuinely blocked routes retain their feedback.
Equipment retrieval and return also use reachable stockpile footprint tiles.
Accountant rounds visit a reachable footprint tile while retaining a valid chosen
approach and the tile used for counting. A fresh report still requires physical
arrival and five units of counting work. Removed nonempty storage keeps its goods
and historical report as a spill, but receives no new storage count.
Food, water and care pickup also use a reachable stockpile tile. Pickup carries
the finite serving; the need improves only after the cat returns to the shrine.
A valid approach keeps its chosen tile when the anchor reopens. If every supply
route closes, the cat retries at whole-second boundaries and can continue its
original work between attempts. Trade and transport retain their endpoint rules.

Cancelled shipping keeps its loaded vessel and driver until physical return.
An obsolete return-route warning clears when sailing along a valid return edge
actually resumes. Genuine obstruction, reboarding failure and full home storage
retain their feedback. Routes updates phase, warnings and completed-route removal
while open, preserving the dispatch form's choices and keyboard focus.

Routes retains a valid destination when you leave and reopen the panel or act
on another route. Removing that pile visibly selects the first current pile;
changing village starts from that village's piles. Check the source and
destination before dispatch. Each route card identifies its driver, stable cat
ID, endpoint pile IDs and coordinates beside **Cancel route**. These details
update while open without replacing the dispatch form.

Workshop recipe choices contain unlocked recipes. **Study recipe unlocks** opens
recipe studies with prior search filters cleared. The selected bill includes
research modifiers; an unstaffed workshop also previews historical storage
reports and supplier links before assignment. While a batch collects inputs or
works, its queue entry stays first. Future entries, repeat choices and pause
remain editable. The remaining queue becomes fully editable once that batch's
output is ready for delivery. Rejected edits preserve officer control.
An otherwise idle recipe position with an empty queue, owned by its living
appointed officer, names that officer as the next work chooser. Its inspector,
queue and building hover explain that Add takes manual control. Needs, direct
control, pauses, active delivery and genuine blockers keep their current
feedback. Queue emptiness alone does not identify a shortage or establish that
output targets have been met.
The bill's **View inventory & hauling** action opens Inventory with the existing
carrier choice. Workers can collect directly from reachable external piles;
hauling into accepting storage is optional consolidation. The bill's historical
storage reports exclude external piles, so a zero report can coexist with a
funded batch. Pile reports can still list goods after they move. Only storage
reports refresh through Accountant visits.
Workplace rows show coordinates when several buildings share a kind, so the
player can choose the intended station before opening its inspector.
Station and per-worker Add controls retain keyboard focus through live inspector
refreshes. Revealing the first order's supply report keeps the focused Add at
its viewport position. Explicit Stores destination changes still start at the top.
In local worlds, a future Repeat control also retains its focus and position
through live refreshes and newly revealed guidance. Queue-entry controls follow
their exact local order when it moves; an old control for a removed order does
nothing. Remote commands retain their existing index-based behavior. Focus
continuity across server snapshot replacement is not established.

Completed workplaces have a fixed **Go to** chooser above their scrolling
inspector. Jump to staffing, a numbered position's work and ownership controls,
its queue and pause control, or historical recipe inputs. This moves within the
same inspector. Progress and worker controls appear before ingredient reports.
Selecting a destination changes no assignment, order or pause. The chooser stays
available while scrolling, and the same destination can be chosen again.
The popup fits its destination text. A fixed **Jumped to** label retains the
numbered worker and section from the last deliberate jump through refresh and
manual scrolling. It clears when that destination or subject disappears.
Paused production exposes **Resume work** beside officer handback below
progress. Resume takes manual control; returning the same worker to its officer
keeps the current batch and manual orders. Resuming while global time stays
paused clears the obsolete pause warning from the inspector without advancing
or rewriting the authoritative simulation.

Cat inspector, census, selected-cat hover and Defense selection derive current
production feedback from the cat's matching active job and workplace. Pause and
Resume are visible there even while global time is paused. Needs and genuine
blockers keep their meaning. A paused future queue still permits finished goods
to be delivered. These displays leave stored goals, job markers and progress
unchanged. The Work active-job list discards an obsolete production-pause
warning only when the displayed job belongs to that cat's current matching
workplace. Other jobs and genuine warnings retain their recorded feedback.

In Stores, choose a free adult under Equipment and use **Retrieve for selected
cat** on an exact item. The cat walks to its source before wearing it. Stores
and the cat inspector show the source and **Cancel equipment order**. Returning
worn gear takes a trip to accepting storage; it stays equipped until delivery.
Closing every footprint tile holds the same exact item claim and retries once
per whole second. Reopening a tile allows the physical trip to resume. The
source and destination IDs remain stable even when the cat uses another tile.
Needs suspend these trips. Busy cats keep their current work, so release that
work before ordering equipment. A full return destination requires space in
that same pile, or cancellation followed by a new return to another pile.

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

The renewable Grain acceptance case is
`acceptance.renewable_grain_two_cycles_steward_mill`. It runs two natural crops
and three finite manual Mill batches; do not use it as an implementation smoke.
The server counterpart uses the same chain with 43 strict whole-authority
save/reload checkpoints. Its fixture starts with no Grain, Flour or growth
credit, retains ordinary needs, and uses finite support and accepting storage.
It does not establish sustained food security. October 23 execution and native
scope are recorded in ACCEPTANCE.md and DEVELOPMENT_LOOP.md.
That native continuation exposed a primary queue restart discrepancy. The
focused server filter is `FOREST_TEST='hourly primary queue'`; shared writer
coverage uses `--filter=primary_queue` with forty cases. The server's
`FOREST_TEST='primary queue'` also runs fourteen automatic-production durable
twin checkpoints and five divergent-history boundaries. It compares the
untouched live branch with complete saved/restored authority. The scoped repair
updates the compatibility mirror only at relevant primary queue mutations;
strict loading retains historical values. Packaged normal quit/reopen on the
same isolated save passes full authority replay through physical Flour storage
and subsequent automatic work. The retained native protocol continuation binds
the failed opening checkpoint exactly; it does not normalize queues. See the
current acceptance entry for timing, performance and remaining limits.
Never erase queue fields in an audit or infer a
renewable automatic restart proof from the separate 43 finite-order checkpoints.

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

New saves use the revised founding layout: a centered 3×3 shrine, surrounding
road, four gates and connected building entrances. Older Unity and imported saves
keep their existing coordinates, footprints, jobs and inventories. Newly planned
buildings in those saves still require a road connection to the actual shrine.
Use a separate unused path to try the new layout without replacing progress:

```sh
open -n 'artifacts/macos/Idle Cat Forest.app' --args --forest-save "$PWD/artifacts/playtest/world.json" --forest-seed 41
```

The same path resumes the same world on subsequent launches. On Retina Macs,
`-screen-width 2400 -screen-height 1500` gives a comfortable management window;
smaller windows keep the text size and scroll their panels.

For automated native reviews, verify quit with the exact launched PID and the
collector's exit result. Do not query a native app binding after CmdQ: the
October 22 interaction review observed an unscoped replacement launch in the
same second as its post-quit accessibility query. Keep that incident separate
from fixture validation and leave default saves and identity stores unread.

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

For frame-pacing diagnosis, also pass `--forest-profile-frames` alongside
`--forest-evidence`. The same report then includes up to 3,600 completed frame
records. Each record pairs elapsed main-thread scopes from one frame with the
interval observed at the following frame. Game, View and UI include nested
scopes and must not be added to their children. Host preemption contributes to
elapsed scope time; Unity layout, rendering, GPU work and frame-limiter waits
are not isolated. The current unfinished frame is omitted, and a report write's
own timing appears in a later report. Profiling adds allocations, serialization
and disk work, so use separate untouched fixtures for ordinary timing and
profiled diagnosis. The flag alone enables no recording or file output.

For local save diagnosis, pass `--forest-profile-saves` together with
`--forest-evidence`. This also enables the adjacent-frame records above. The
report retains at most 128 save calls, each with a sequence, Unity frame index,
success or failed stage, UTF-8 sizes, total time and residual time. Its ten
nonoverlapping stages cover lock wait, validation, payload encoding, payload
UTF-8 conversion and checksum, envelope encoding, private temporary-file setup,
envelope UTF-8 conversion and write, durable flush and close, replacement, and
directory sync. Ordinary saves do not create a trace or read its timestamps.

Stages measure elapsed time, including host preemption. A failed stage may
include stream unwinding. Unattempted stages are not completed work. Authority
total excludes Presentation's retention of the result; the outer frame Save
scope includes it. Join a save's frame index only to an available completed
frame record to inspect the following interval. Initialization saves and calls
after the last periodic report are absent. Pause and quit may have no retained
following interval. These bounded reports add overhead and do not establish a
speedup. They preserve the save lock, validation, v1 bytes and durability steps.

Save and load checksums convert the payload in chunks of up to 4,096 UTF-16
characters using a per-call UTF-8 scratch buffer of about 12 KiB at most.
Chunk boundaries retain valid surrogate pairs and preserve the existing
replacement behavior for isolated surrogates. JSON encoding, the envelope,
strict reads and durable installation are unchanged. This bounds checksum
scratch storage; it does not bound the complete save's allocations.

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
