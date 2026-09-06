# Simulation acceptance scenarios

These scenarios compile the same C# simulation and scenario source into a .NET 10 executable and Unity EditMode tests. They do not launch Unity, contact a server, or call an AI provider.

The current inventory is 641 noncampaign cases and nine campaign twins. See `VALIDATION.md` and the result files for the verified source and remaining limits.

Build and list the cases:

```sh
dotnet build tools/scenarios/Forest.Scenarios.csproj
dotnet tools/scenarios/bin/Debug/net10.0/Forest.Scenarios.dll --list
```

Run a selected case or group. A failing assertion makes the process return a nonzero exit code.

```sh
dotnet tools/scenarios/bin/Debug/net10.0/Forest.Scenarios.dll --filter=regression.emergency_staffed_water
dotnet tools/scenarios/bin/Debug/net10.0/Forest.Scenarios.dll --filter=recipe.
dotnet tools/scenarios/bin/Debug/net10.0/Forest.Scenarios.dll --filter=catalog.
dotnet tools/scenarios/bin/Debug/net10.0/Forest.Scenarios.dll --filter=building_effect.
dotnet tools/scenarios/bin/Debug/net10.0/Forest.Scenarios.dll --filter=service_effect.
dotnet tools/scenarios/bin/Debug/net10.0/Forest.Scenarios.dll --filter=resource_effect.
dotnet tools/scenarios/bin/Debug/net10.0/Forest.Scenarios.dll --exclude-campaigns
dotnet tools/scenarios/bin/Debug/net10.0/Forest.Scenarios.dll --filter=campaign.fresh_48h_seed_7
```

Omit the filter to run every case. Campaigns advance fresh colonies for 48 hours and established colonies for 72 hours on seeds 7, 41, and 127. A further 48-hour campaign runs both the communal village and a player-founded personal village in the same world for each seed. Each campaign compares every public world field with a twin advanced in different time partitions, checks invariants hourly, and rejects founding deaths or extinction resets. Double comparisons retain nine decimal places. The established fixture adds a staffed, repeating wood-processing station and finite inputs before simulation begins. It is a bounded production-pressure campaign, not proof that every officer and production chain can sustain itself indefinitely.

Unity discovers the same cases through `IdleCatForest.Acceptance.Tests`. Use the Editor's EditMode test runner or the documented batch-mode test command and select that assembly.

Fixture state is explicit and finite. Individual scenarios isolate a worker by holding other cats under long control leases, seed supplies before actions begin, and use public actions for purchases, queue edits, assignments, construction, and control changes. Recipe cases assert input carrying, work at the station, finite input consumption, output arrival, exact item identity, and released claims. Food assertions account for the documented spoilage that continues during work.

Shipping cancellation checks cover physical return, full source storage, and driver death. The death case injects lethal health after loading, then builds a bridge with finite lumber and hauls the recovered cargo through public actions. Full storage is created and cleared through a second cat's public deposit and pickup actions.

Cargo handoff checks cover new buildings, offerings, scouting, hauling and resumed roads, rails and expansion. They preserve unrelated carried goods before work adoption, consume exact material bills and release suspended claims. The expansion case checks every paid outer segment, the open south gate and removal of obsolete inner walls.

The public purchase traversal covers all 487 research nodes and all 108 recipe unlocks. Separate differential scenarios exercise 75 building-modifier studies, 238 general/service modifier studies, and 64 resource-stage studies. Their paired fixtures start with prerequisite knowledge and differ by one completed study. Six older blessing-funded upgrade axes have both escalating-cost/max-level tests and gameplay comparisons. The complete building-construction sweep and Mountain/Rail/Shipping scenarios cover remaining access effects.

Capacity probes inspect the authoritative finite-capacity projection. Construction and full-storage scenarios separately test the physical container and delivery paths. No test calls a private simulation method. Purchase success alone does not prove every payload changes gameplay; use the named effect results when reporting that coverage.
