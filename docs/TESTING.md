# Testing Idle Cat Forest

Unity and C# are the maintained game. The TypeScript and Rust migration documents
describe earlier implementations. Current acceptance status is in
[unity/ACCEPTANCE.md](unity/ACCEPTANCE.md).

## Behavior changes

Start with the smallest failing regression, record its result, implement the
behavior, then verify the composed consequence. Do not weaken an assertion to
match a defect. Documentation and test infrastructure need appropriate validation
but do not require a preceding behavioral failure.

During implementation, use focused smoke tests with a combined test runtime under
ten seconds. Once the implementation is complete, run the full applicable
simulation/server checks, extended campaigns, Unity suites and native UI checks.

## Simulation and authority

```sh
dotnet build tools/scenarios/Forest.Scenarios.csproj
dotnet tools/scenarios/bin/Debug/net10.0/Forest.Scenarios.dll --list
dotnet tools/scenarios/bin/Debug/net10.0/Forest.Scenarios.dll --filter=regression.
dotnet tools/scenarios/bin/Debug/net10.0/Forest.Scenarios.dll
bash tools/forest.sh server-test
```

Scenarios compile the same source as Unity EditMode tests. They cover finite
source-to-station-to-storage chains, every recipe and study, permissions,
interruption and seeded campaigns. Scenario documentation explains fixture
supplies and the limits of each workload. Actual network tests use loopback
sockets and isolated temporary worlds. Tests must never access a live user's save
or contact a paid AI provider.

## Unity and native player

Close this project's Editor before batch tests or a build:

```sh
bash tools/forest.sh edit-test
bash tools/forest.sh play-test
bash tools/forest.sh build
```

Reports are written to ignored `artifacts/tests`. Inspect the NUnit counts,
failures and Editor/runtime logs. A successful CLI envelope or process launch is
not proof of a passed suite.

The EditMode wrapper runs the 554 noncampaign simulation tests. The nine extended
campaigns run through the standalone .NET executable, including in CI. Running
all campaigns again under the Editor's Mono runtime exceeded the ten-minute local
test limit. To select a specific Unity campaign, pass its name with `--filter`
and an appropriate `--timeout`; no campaign assertion is skipped or weakened.

Use the normal UI in both Play mode and the packaged app to inspect cats, build,
assign workers, edit queues, purchase studies, enter a cat, walk, interact and
return to management. Compare the observed state with the controls used.
Screenshots establish appearance; accounting assertions establish correctness.
Use separate temporary save paths for evidence sessions.

## Performance and release evidence

Measure founding and expanded populations on the actual machine. Record workload,
population, resolution, frame and complete simulation step percentiles. Report
whether measurements came from the Editor or packaged player, and distinguish
sustained frame cost from autosave or one-time loading.

A fresh checkout must resolve dependencies, compile, open, run and build without a
copied Library directory. Keep generated binaries, credentials and screenshots
out of source control. Visual PR evidence must follow the Schaffa publishing rule.
The independent candidate review and secret scan happen before the PR is
published; this migration does not authorize merging or deployment.
