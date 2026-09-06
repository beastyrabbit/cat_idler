# Native performance

The September 6, 2026 native samples held approximately 60 frames per second at
2400 × 1500 on an Apple M1 Max with 32 GB of memory. The ARM64 IL2CPP player ran
the local simulation at 1× speed in management view, with its 60 FPS target.
The measured build includes the simulation, art and normal-window UI revisions.
A subsequent layout correction applies only below 650 panel units and is inactive
at the measured window size. Later housing and expansion guards do not activate
in these samples: both worlds are younger than six simulated hours and have no
expansion jobs. Unity Editors were closed, and both samples preceded F9/F10 evidence capture. The
player's earlier native game instance remained open in the background to preserve
that session, so these are not measurements on an otherwise idle machine.

The 30-cat run included brief world-click inspection, a research purchase and
navigation through a prerequisite link. Neither run entered direct control or
captured images while sampling. The prepared worlds exercise the new building
entrances and connected roads. They contain no foreign construction or imported
station output, so the samples do not isolate the cost of those recovery paths.

These are observations of two finite review workloads. The frame cap hides spare
rendering capacity, and the results do not establish performance at arbitrary
populations, remote-server throughput, or a self-sustaining mature economy.

## Measurements

All timings below are milliseconds. Exact samples are preserved in
[performance-30.json](performance-30.json) and
[performance-150.json](performance-150.json).

| Measurement | 30 cats | 150 cats |
| --- | ---: | ---: |
| Buildings | 16 | 54 |
| Cats with an active job at sampling | 5 | 10 |
| Active reservations at sampling | 0 | 0 |
| Frame interval p50 | 16.666 | 16.666 |
| Frame interval p95 | 17.631 | 17.524 |
| Local 0.1-second advance p50 | 0.0089 | 0.0120 |
| Local 0.1-second advance p95 | 0.2367 | 0.5083 |
| Complete economy tick p50 | 0.2269 | 0.5086 |
| Complete economy tick p95 | 1.5820 | 1.0971 |
| Frame samples | 3,600 | 3,600 |
| Local advance samples | 1,801 | 1,599 |
| Complete economy tick samples | 180 | 159 |

The populations perform different work, and their simulation sampling windows
differ. The lower 150-cat economy p95 does not imply that adding cats makes the
simulation faster. Active-job counts are snapshots; they do not mean every cat
or every staffed station worked throughout the measurement.

## What the counters measure

[`ForestGame.Update`](../../unity/Assets/Forest/Presentation/ForestGame.cs) records
the unscaled interval between frames, including the frame limiter and normal
player work. This is a frame interval, not isolated GPU or rendering CPU time.

A stopwatch surrounds each `LocalAuthority.Advance(0.1)` call. Most calls advance
the clock without running the once-per-second economy. Calls that cross a whole
simulation second also enter the economy sample list. Those calls include the
complete ecology, village and trade update in
[`World.Step`](../../unity/Assets/Forest/Simulation/World.cs). Their cost is the
complete tick's elapsed execution time, not one second of wall time. Autosave
runs outside this stopwatch; its cost can appear in frame intervals.

Each list retains at most 3,600 samples. At the observed 60 FPS, the frame list
covers approximately the latest minute. The 30-cat advance and economy lists
cover approximately 3.00 minutes at 1×; the 150-cat lists cover 2.67 minutes.
Consequently, frame and simulation percentiles do not describe identical time
windows. Percentiles sort the retained samples and select index `floor(q × N)`
without interpolation. The player writes a report every ten seconds when
`--forest-evidence` is enabled.

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

Close Unity Editors. Use a separate synthetic save and record whether other
native game instances remain running; the measurements above retained an earlier
player session in the background. Run from the checkout root with the documented
Unity and .NET versions installed:

```sh
dotnet restore IdleCatForest.slnx --locked-mode
dotnet build IdleCatForest.slnx --no-restore -warnaserror
bash tools/forest.sh build
forest_perf_dir="$(mktemp -d /tmp/forest-native-performance.XXXXXX)"
dotnet tools/presentation-fixture/bin/Debug/net10.0/Forest.PresentationFixture.dll "$forest_perf_dir/founding30.json" 30
dotnet tools/presentation-fixture/bin/Debug/net10.0/Forest.PresentationFixture.dll "$forest_perf_dir/expanded150.json" 150
open -n 'artifacts/macos/Idle Cat Forest.app' --args --forest-save "$forest_perf_dir/founding30.json" --forest-evidence "$forest_perf_dir/30" -screen-fullscreen 0 -screen-width 2400 -screen-height 1500
```

Leave the player at 1× in management view. Verify the report's actual width and
height because macOS may constrain the requested window size. Observe
`$forest_perf_dir/30/performance.json` after it reaches 3,600 frame samples,
approximately 1,800 advance samples and 180 economy ticks, about three minutes.
For the observed 30-cat workload, briefly inspect a workplace, purchase an
available study and follow a prerequisite link. Record any interactions on a
repeat run. Preserve the report before pressing F9 or F10. Quit this synthetic
30-cat player before launching the expanded fixture:

```sh
open -n 'artifacts/macos/Idle Cat Forest.app' --args --forest-save "$forest_perf_dir/expanded150.json" --forest-evidence "$forest_perf_dir/150" -screen-fullscreen 0 -screen-width 2400 -screen-height 1500
```

Observe `$forest_perf_dir/150/performance.json` at 3,600 frame samples, approximately
1,600 advance samples and 160 economy ticks, about 2.7 minutes. The ten-second report
cadence can produce nearby sample counts on a repeat run. Record the actual
counts, workload and resolution rather than changing samples to match this table.
Use newly generated saves for another run. Keep the adjacent private `.identity`
directories local; only the performance JSON belongs with published measurements.
