# Unity migration acceptance

Local acceptance verified on September 6, 2026. The rows below distinguish
automated checks, normal UI observations and their limits. Final candidate review,
outgoing secret scanning and publication are recorded in the migration PR.

The migration started from `main` at
`8c5ea0f2d0871a1f12dfdafd831e6d4a78d40cec`. A clean checkout of
`27ab7138fe419e5c701e5384cf9b7329589ebdb9` passed dependency resolution, compilation,
native build, scene opening and Play mode. The later Village panel refresh passed
its focused regression, the complete 15-test PlayMode suite and a rebuilt native
app check of connection, local return and reconnect.

The maintained inventory is 25 buildings, 108 recipes, 487 studies, seven
specialist officers and 19 labors. The legacy action inventory had 53 variants,
including development controls. The Unity authority rejects development-only
time and random-state controls.

| Required capability | Observed evidence | Status |
| --- | --- | --- |
| Reproducible native game | Clean checkout resolved locked .NET dependencies, built with zero warnings/errors, passed formatting, and produced an ARM64 IL2CPP app. Unity imported its own Library, then opened Forest.unity and entered Play mode. The final UI source also rebuilt successfully. | Verified |
| Living cats | Finite meals, water, sleep, needs, skills, breeding beds, migration, death and recovery pass focused scenarios. Campaigns retain every founding cat without extinction resets. | Verified |
| Manual and automated work | Leader safety work, seven officer prerequisites and vacancies, public job actions and staffed queues pass scenarios. Normal UI logging passes a focused PlayMode check. | Verified |
| Physical economy | All 108 recipes carry inputs, perform work and deliver outputs. Native Wood Cutter production delivered nine planks; the Editor completed one batch and delivered one plank. | Verified |
| Scarcity and interruption | Contended inputs and beds, cancellation, reassignment, death, blocked routes, full storage, shipping recovery and direct-control handoffs pass conservation and ownership checks. | Verified |
| Construction and territory | Placement, material delivery, roads, bridges, expansion and farms pass scenarios. Normal UI placed and completed a Den with 16 planks, eight blocks and 300 work. | Verified |
| Knowledge and progression | All 487 studies pass public purchase traversal with separate effect scenarios. Normal UI research spent 20,000 → 19,995 points on Research Hut and displayed the dependency map. | Verified |
| Equipment, defense and trade | Exact item identity, condition, repair, finite traders, raids, warriors, rail/shipping and physical barter pass scenarios and real socket tests. | Verified |
| Persistent shared authority | Two identities, private-village denial, filtered reports, restart and physical trade pass real socket tests. The native app connected to a synthetic server, founded 15-cat Mosslight, then reconnected to the same village, ID 74. | Verified |
| Saves and migration | 26 authority tests and the real SQLite conversion pipeline pass. Jobs, cargo, claims, identities and returning trade survive restart without replay. Import preserves source bytes and refuses unknown schemas and destination overwrite. The packaged app opened the converted world and resumed both imported jobs. | Verified |
| 3D management and cat control | Blender geometry passes export/import checks. Editor and native UI inspect and control the same cat, walk, take five food and return it to AI while the colony continues. Native deposit emptied its cargo into the original store. | Verified |
| Normal UI operation | Editor and native inspect, staffing, queues, research, construction and direct-control flows were exercised with saved-state checks. Native repeat persisted as true and pause/resume returned to unpaused work. Editor queue delivery passed; its repeat click did not persist. Final native connection, return and reconnect refreshed the open Village panel and preserved the server address. | Verified within this scope |
| Extended operation | Nine campaign twins cover fresh 48-hour, established 72-hour and shared/personal 48-hour worlds at seeds 7, 41 and 127. | Verified in .NET |
| Measured performance | Final native 30/150-cat samples measured 16.67 ms median frames, 17.40/17.21 ms p95 frames and 1.91/1.32 ms p95 economy ticks at 6016×3080 on an M1 Max. Workloads, windows and limits are in [performance evidence](PERFORMANCE.md). | Verified |

## Test and build evidence

- [Simulation results](../../tools/scenarios/VALIDATION.md): 548 focused cases
  passed in 8.38 seconds and nine campaign twins passed in 173.54 seconds.
  Campaigns compare partitioned time, validate claims and retain founding IDs.
  The established fixture has finite supplies and one repeating wood-processing
  station; it does not prove indefinite operation of every mature production chain.
- Unity EditMode passed 554 noncampaign tests in 50.890 seconds. The unfiltered
  Editor run timed out after 600 seconds, so no Editor campaign pass is claimed.
  The same nine campaign scenarios passed in the .NET runner.
- Unity PlayMode passed all 15 tests in 8.139 seconds. These include ordinary UI
  logging, research, queues after reload, direct control at 8× speed, village
  selection, stockpile corners, merchant sales, work boosts, authored geometry
  and terrain categories/bounds during camera movement.
- [Authority and import checks](PERSISTENCE.md) passed 26 tests. The real Rust
  SQLite writer → normalizer → C# continuation pipeline passed three tests; the
  archival exporter passed two. All test worlds and identities are synthetic.
- [Blender verification](../../source-art/verification.json) passed for 80 models,
  86 meshes and 92,593 triangles. [Art documentation](../../source-art/README.md)
  records editable sources, orientation, units, pivots and reproduction commands.

Local reports are `artifacts/tests/editmode.xml`, `artifacts/tests/playmode.xml`
and `artifacts/fresh-checkout/27ab713-verification.txt`. The fresh-checkout report
covers the clean build; subsequent interactive scene opening and Play mode were
checked through Pipeline and Computer Use. These ignored reports are local
evidence, not published PR attachments. No Library, save or identity directory was
copied into the fresh checkout.

## Final native checks and evidence

The final build includes the Village refresh regression fix. A 150-cat local
world connected to the isolated shared server and selected the existing 15-cat
Mosslight village. Returning restored the 150-cat local world; reconnect retained
the server address and selected Mosslight again. The open panel tracked every
transition without closing and reopening it.

A separate copy of the synthetic SQLite-converted world opened with 30 cats and
16 buildings. At 35.7 simulated seconds, the imported haul had delivered 2.5 logs
and the unfinished production job had raised stored planks from 20 to 21. Both
original cat IDs remained. The normal roster selected Fixture Moss, entered the
close camera and returned that cat to AI. Runtime logs for the final founding,
expanded and imported worlds contained no exception, assertion or runtime error.

The PR includes game-only management and carrying screenshots, the research map
and an eight-second interaction video through verified Schaffa links. These use
synthetic saves. They contain no identity files, server credentials or user data.
The recording shows deposit, walking and return to management; capture overhead
makes it unsuitable for measuring frame rate. Performance was sampled separately
before capture.

Unity/C# is now the only playable application. Superseded Rust/Bevy entry points,
root Cargo configuration and obsolete checks were retired after replacement
builds and tests worked. Frozen compatibility libraries remain under
`tools/save-import/legacy` for read-only import and catalog export. Their locked
builds and the import pipeline still pass after retirement.

Independent candidate reviews found and resolved several issues. Construction
reassignment now leaves unrelated carried goods in a physical pile, while
entering direct control preserves the cat's existing cargo. Armor-only storage
accepts produced and unequipped armor with its exact identity and condition.
Fresh Accountant reports include exact equipment when deciding shared-client
visibility; stale reports and unreported piles still hide their contents.
Terrain rebuilding uses one position index instead of thousands of linear
searches. Focused failing tests preceded the behavior fixes. The terrain check
preserved mesh categories, bounds and unknown tiles across camera movement.
The rebuilt native app also panned across the expanded village and returned with
Home without missing terrain or runtime errors.

Further handoff tests cover new building placement, offerings, scouting, hauling
and adoption of pending road, rail and expansion jobs. Each new work owner leaves
unrelated cargo recoverable and consumes only its own bill. Expansion retains
every paid outer-wall segment, clears the obsolete interior walls and opens its
south gate. Imported barter whose outward goods have arrived cannot be cancelled
to reclaim payment. Scalar and exact payment cargo finish their return route,
including full-storage waits and restart, without paying either village twice.

[Gameplay decisions](GAMEPLAY_ACCEPTANCE.md), [save compatibility](PERSISTENCE.md)
and [development commands](DEVELOPMENT.md) describe the implemented behavior and
reproduction steps. No merge, deployment, purchase or production-data change has
been performed for this migration.
