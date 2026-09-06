# Native performance

The September 6, 2026 native samples held approximately 60 frames per second at
2400 × 1500 on an Apple M1 Max with 32 GB of memory. The ARM64 IL2CPP player ran
the local simulation at 1× speed in management view, with its 60 FPS target.
The clean build came from commit `90177b6c506900180ab0777df5ccdfbb2dcbe2b9`
and includes the continuous simulation. Every 50-millisecond step advances actors.
The p95 execution cost per step was 0.491 ms with 30 cats and 0.945 ms with 150.

Both runs used the default management view without interaction, direct control,
screenshots or recording during measurement. Unity Editors and test runners were
closed. The player's earlier native game instance remained open in the background
to preserve that session, so the machine was not otherwise idle. Each report was
preserved before subsequent UI checks and evidence capture.

The prepared worlds exercise connected building entrances and road routing. They
have no designated farms, transport routes, active village trades, expansion jobs,
foreign construction or imported station output. Both are younger than six
simulated hours. These samples therefore do not measure large cultivated areas,
late housing expansion or transport and import recovery paths.

These are observations of two finite review workloads. The frame cap hides spare
rendering capacity, and the results do not establish performance at arbitrary
populations, remote-server throughput, or a self-sustaining mature economy.

## Measurements

Frame and execution timings below are milliseconds. Exact samples are preserved in
[performance-30.json](performance-30.json) and
[performance-150.json](performance-150.json).

| Measurement | 30 cats | 150 cats |
| --- | ---: | ---: |
| Buildings | 16 | 54 |
| Cats with an active job at sampling | 5 | 10 |
| Active reservations at sampling | 0 | 0 |
| Frame interval p50 | 16.667 | 16.666 |
| Frame interval p95 | 17.583 | 17.394 |
| Local 50 ms simulation step p50 | 0.1110 | 0.3643 |
| Local 50 ms simulation step p95 | 0.4909 | 0.9454 |
| Step including whole-second planning p50 | 0.1661 | 0.5041 |
| Step including whole-second planning p95 | 0.9619 | 1.1083 |
| Frame samples | 3,600 | 3,600 |
| Simulation step samples | 2,606 | 3,405 |
| Steps including whole-second planning | 130 | 170 |
| Simulated time covered by step samples | 130.30 s | 170.25 s |

The populations perform different work, and their simulation sampling windows
differ. Active-job counts are snapshots; they do not mean every cat or every
staffed station worked throughout the measurement.

## What the counters measure

[`ForestGame.Update`](../../unity/Assets/Forest/Presentation/ForestGame.cs) records
the unscaled interval between frames, including the frame limiter and normal
player work. This is a frame interval, not isolated GPU or rendering CPU time.

A stopwatch surrounds each `LocalAuthority.Advance(0.05)` call. Every call runs
actor movement, needs, work and transport through
[`World.Step`](../../unity/Assets/Forest/Simulation/World.cs). Calls that cross a
whole simulation second also perform the slower planning updates and enter the
subset reported under the existing `economyTicks`, `economyP50` and `economyP95`
JSON names. Those timings include that call's actor work and planning. They do
not aggregate the preceding second's simulation steps. Autosave runs outside the
stopwatch; its cost can appear in frame intervals.

Each list retains at most 3,600 samples. At the observed 60 FPS, the frame list
covers approximately the latest minute. The 30-cat step list spans 130.30 simulated
seconds and includes 130 planning steps; the 150-cat list spans 170.25 seconds and
includes 170 planning steps. At 1×, these are approximately 2.17 and 2.84 minutes
of play. Frame and simulation percentiles therefore cover different time windows.
Percentiles sort the retained samples and select index `floor(q × N)` without
interpolation. The player writes a report every ten seconds when
`--forest-evidence` is enabled. The pinned reports record
`simulationStepSeconds: 0.05` and supersede the earlier 0.1-second measurements.

## Earlier terrain rebuild check

An independent review found that each camera chunk rebuild called a linear tile
search 5,929 times. The renderer now builds a position index once per rebuild.
On the same Mac, a PlayMode check rebuilt three camera positions over 3,025 tiles
in 160.028 ms before the change and 7.339 ms after it. Both runs checked the same
77×77 mesh bounds, water, mountain, known missing ground and fog categories, and
verified that moving the camera did not create authoritative tiles. These are
Editor CPU timings from the earlier renderer review, separate from the current
native samples. This historical comparison was not rerun for the final art and
camera revisions.

## Workloads

[`Forest.PresentationFixture`](../../tools/presentation-fixture/README.md) creates
isolated, validated saves with finite review supplies. The 30-cat fixture retains
the maintained 16-building communal blueprint, including six Dens. It adds
construction materials, 20,000 research points and 200 blessings before play.
These review credits are not production founding defaults.

The expanded fixture has 150 cats with real beds in 30 Dens, all 25 building
types across 54 buildings, a known radius-23 enclosure, and finite exterior
sources. Its storage starts with 5,000 Food, 5,000 Water and finite processing
inputs. Thirty owned studies enable representative production. Thirteen stations
are staffed and twelve recipes are queued across the production chains; research,
school and accounting staffing are included. The generator advances fifteen
seconds so the save starts with actual work in progress. The tool grants its
review supplies before play; ordinary production and consumption then change
the stocks.

Both fixtures now use a centered 3 × 3 shrine with a complete road ring and four
cardinal gates. Houses and workshops have persisted entrances connected to that
ring. The expanded fixture authors road spurs around physical building and
stockpile footprints, including the exterior field's route through a gate. Its
53 non-shrine buildings have validated connected entrances, and authoritative
paths reach each workplace through its entrance before saving and after reload.

Both samples are local, with direct control off and no remote server connection.
They demonstrate these prepared workloads on this machine. Long-term survival
and resource conservation have separate [acceptance scenarios](GAMEPLAY_ACCEPTANCE.md).

## Reproduce

Close Unity Editors and test runners. Use a separate synthetic save and record
whether other native game instances remain running; the measurements above
retained an earlier player session in the background. Run from the checkout root
with the documented Unity and .NET versions installed:

```sh
dotnet restore IdleCatForest.slnx --locked-mode
dotnet build IdleCatForest.slnx --no-restore -warnaserror
bash tools/forest.sh build
forest_perf_dir="$(mktemp -d /tmp/forest-native-performance.XXXXXX)"
dotnet tools/presentation-fixture/bin/Debug/net10.0/Forest.PresentationFixture.dll "$forest_perf_dir/founding30.json" 30
dotnet tools/presentation-fixture/bin/Debug/net10.0/Forest.PresentationFixture.dll "$forest_perf_dir/expanded150.json" 150
open -n 'artifacts/macos/Idle Cat Forest.app' --args --forest-save "$forest_perf_dir/founding30.json" --forest-evidence "$forest_perf_dir/30" -screen-fullscreen 0 -screen-width 2400 -screen-height 1500
```

Leave the player at 1× in the default management view without interaction or
capture. Verify the report's actual width and height because macOS may constrain
the requested window size. Observe
`$forest_perf_dir/30/performance.json` after it reaches 3,600 frame samples,
approximately 2,600 simulation steps and 130 planning steps, about 2.2 minutes.
Copy that report to a separate measurement file before interacting, pressing F9
or F10, or starting any recording. Quit this synthetic 30-cat player before
launching the expanded fixture:

```sh
open -n 'artifacts/macos/Idle Cat Forest.app' --args --forest-save "$forest_perf_dir/expanded150.json" --forest-evidence "$forest_perf_dir/150" -screen-fullscreen 0 -screen-width 2400 -screen-height 1500
```

Observe `$forest_perf_dir/150/performance.json` at 3,600 frame samples, approximately
3,400 simulation steps and 170 planning steps, about 2.8 minutes. Preserve this
report before any UI checks or capture too. The ten-second report cadence can
produce nearby sample counts on a repeat run. Record the actual counts, workload
and resolution rather than changing samples to match this table. Use newly
generated saves for another run. Keep the adjacent private `.identity`
directories local; only the performance JSON belongs with published measurements.
