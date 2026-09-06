# Unity migration acceptance

The September 6 playtest exposed layout and presentation problems after the
initial technical acceptance below. The corrective revision adds a centered 3×3
shrine, surrounding roads, connected entrances, four gates, joined fence corners,
responsive zoom and a readable management interface. Existing played saves keep
their layout and progress; a separate new save exercises the revised founding.

The revised simulation passed 645 noncampaign scenarios, including 143 regressions.
Nine earlier campaign twins passed within the scope described below.
Unity passed 651 EditMode and 33 PlayMode tests.
The authority/import suite passed 46 tests. The real SQLite pipeline passed four
tests in 24.959 seconds. Blender reimport verified 82 models, 88 meshes and 145,611
triangles. These checks establish the tested behavior, not the player's approval
of the visual design. Native observations and current costs appear below and in
[the performance report](PERFORMANCE.md).

The remaining migration evidence records the original implementation and the
scope of its checks. Final candidate review, outgoing secret scanning and
publication are recorded in the same migration PR.

The migration started from `main` at
`8c5ea0f2d0871a1f12dfdafd831e6d4a78d40cec`. A clean checkout of
`27ab7138fe419e5c701e5384cf9b7329589ebdb9` passed dependency resolution, compilation,
native build, scene opening and Play mode. The later Village panel refresh passed
its focused regression and a rebuilt native app check of connection, local return
and reconnect.

The corrective revision at `61d29f13f6f4102a456975b2203b4966dedfead4` also passed
locked .NET restore and an ARM64 IL2CPP build in a new checkout with no existing
Unity Library. No Library, played save or identity directory was copied into it.

The maintained inventory is 25 buildings, 108 recipes, 487 studies, seven
specialist officers and 19 labors. The legacy action inventory had 53 variants,
including development controls. The Unity authority rejects development-only
time and random-state controls.

| Required capability | Observed evidence | Status |
| --- | --- | --- |
| Reproducible native game | Clean checkout resolved locked .NET dependencies, built with zero warnings/errors, passed formatting, and produced an ARM64 IL2CPP app. Unity imported its own Library, then opened Forest.unity and entered Play mode. The Village refresh source also rebuilt successfully. | Verified |
| Living cats | Finite meals, water, sleep, needs, skills, breeding beds, migration, death and recovery pass focused scenarios. Campaigns retain every founding cat without extinction resets. | Verified |
| Manual and automated work | Leader safety work, seven officer prerequisites and vacancies, public job actions and staffed queues pass scenarios. Normal UI logging passes a focused PlayMode check. | Verified |
| Physical economy | All 108 recipes carry inputs, perform work and deliver outputs. Original native Wood Cutter production delivered nine planks; the Editor completed one batch and delivered one plank. The corrective revision delivered a plank and started the next repeating batch. | Verified |
| Scarcity and interruption | Contended inputs and beds, cancellation, reassignment, death, blocked routes, full storage, shipping recovery and direct-control handoffs pass conservation and ownership checks. | Verified |
| Construction and territory | Placement, material delivery, roads, bridges, expansion and farms pass scenarios. Thirty regressions protect foreign territory during planning, work, founding and recovery. Normal UI placed and completed a Den with 16 planks, eight blocks and 300 work. | Verified |
| Knowledge and progression | All 487 studies pass public purchase traversal with separate effect scenarios. Normal UI research spent 20,000 → 19,995 points on Research Hut and displayed the dependency map. | Verified |
| Equipment, defense and trade | Exact item identity, condition, repair, finite traders, raids, warriors, rail/shipping and physical barter pass scenarios and real socket tests. | Verified |
| Persistent shared authority | Two identities, private-village denial, filtered reports, restart and physical trade pass real socket tests. The native app connected to a synthetic server, founded 15-cat Mosslight, then reconnected to the same village, ID 74. | Verified |
| Saves and migration | 46 authority/import tests and the real SQLite conversion pipeline pass. Jobs, cargo, claims, identities and returning trade survive restart without replay. Imported exact-item capacity, physical pickup, mixed output delivery and newly staffed empty queues preserve their limits and identities. Import preserves source bytes and refuses unknown schemas and destination overwrite. The packaged app opened the converted world and resumed both imported jobs. | Verified |
| 3D management and cat control | Blender geometry passes export/import checks. Editor and native UI inspect and control the same cat, walk, take five food and return it to AI while the colony continues. Native deposit emptied its cargo into the original store. | Verified |
| Normal UI operation | Editor and native inspect, staffing, queues, research, construction and direct-control flows were exercised with saved-state checks. Native repeat persisted as true and pause/resume returned to unpaused work. Editor queue delivery passed; its repeat click did not persist. Original native connection, return and reconnect refreshed the open Village panel and preserved the server address. Corrective-revision road, construction and queue observations appear below. | Verified within this scope |
| Extended operation | Nine campaign twins cover fresh 48-hour, established 72-hour and shared/personal 48-hour worlds at seeds 7, 41 and 127. | Verified in .NET |
| Measured performance | Current native 30/150-cat samples, workloads, resolution, source timing and limits are recorded in [the performance report](PERFORMANCE.md). | Verified for the recorded samples |

## Test and build evidence

- [Simulation results](../../tools/scenarios/VALIDATION.md): 645 focused cases,
  including 143 regressions, passed in 29.6758 seconds. Nine campaign twins passed
  in 213.7647 seconds after the complete road and shore-placement corrections.
  Campaigns compare partitioned time, validate claims and retain founding IDs.
  The established fixture has finite supplies and one repeating wood-processing
  station; it does not prove indefinite operation of every mature production chain.
- Unity EditMode passed 651 noncampaign tests in 90.865 seconds. The unfiltered
  Editor run timed out after 600 seconds, so no Editor campaign pass is claimed.
  The same nine campaign scenarios passed in the .NET runner.
- The complete corrective-revision PlayMode run passed 33 tests.
  These include ordinary UI logging, research, queues after reload, direct control
  at 8× speed, village selection, stockpile corners, merchant sales, work boosts, authored geometry
  and terrain categories/bounds during camera movement. Exact crafted cargo
  appears after physical pickup and follows its carrier.
  Revision tests also cover normalized wheel input and cursor anchoring, zoom
  limits, visible inspector subjects, joined fence corners/open gates, stable text
  sizing, queue deduplication, worn earth retaining resource geometry, preserved
  legacy orientation, silent successful control heartbeats, world-click framing,
  navigation that resets scroll for a new subject while preserving live refresh,
  and nonoverlapping narrow headers with usable Cats/Inspect scroll areas at
  600×360 and 550×336 panel units. A public 4×3 farm designation visibly covers
  every occupied tile, and clearing it removes the whole crop footprint.
- [Authority and import checks](PERSISTENCE.md) passed 46 tests in 31.31 seconds. The real Rust
  SQLite writer → normalizer → C# continuation pipeline passed four tests; the
  archival exporter passed two. All test worlds and identities are synthetic.
- [Blender verification](../../source-art/verification.json) passed for 82 models,
  88 meshes and 145,611 triangles. [Art documentation](../../source-art/README.md)
  records editable sources, orientation, units, pivots and reproduction commands.

The recorded focused, authority, campaign and Unity suites include the territory
and imported-output fixes. Campaign workloads use no transport routes or
imported batch outputs, so their pass provides no evidence for that delivery
path. Import tests check partial delivery, mixed item filters, full storage,
public staffing and restart directly.
The last campaign run used `61d29f1`, before the final farm-edge projection,
crop-rendering and transport corrections. Campaigns contain no road, rail or expansion
construction jobs and no designated farms; they do not exercise those changes.
The final focused regressions cover future farm boundaries, paid construction,
blocked rail transport and interrupted resumption. PlayMode checks the complete visible farm footprint.

Five rail regressions reproduce completed walls, water and fence edges blocking
loaded wagons and return journeys. Public construction and two successive
expansions demonstrate blocking and recovery with the same wagon, driver, route
and finite cargo. The authority suite also revokes a synthetic save directory's
write permission, verifies continued ticks and failed readiness, then restores
permission and checks successful saving without replacing cat identities.
Four additional transport cases verify physical reboarding after drinking,
a blocked return path, sleep at a reserved Den after an offshore journey, and
an accepted caravan blocked by a newly completed farm fence. The same routes
resume and conserve their cargo after needs or access recover. Two socket bursts
verify that throttled requests avoid full projections while correlated failures,
scheduled snapshots and another player's accepted actions still arrive.

Local reports are `artifacts/tests/territory-focused.txt`,
`artifacts/tests/revision-transport-authority.txt`, `artifacts/tests/revision-transport-editmode.xml`, `artifacts/tests/revision-transport-playmode.xml`
and `artifacts/fresh-checkout/27ab713-verification.txt`. The fresh-checkout report
covers the clean build; subsequent interactive scene opening and Play mode were
checked through Pipeline and Computer Use. These ignored reports are local
evidence, not published PR attachments. No Library, save or identity directory was
copied into the fresh checkout.

## Corrective-revision native observations

The revised packaged app used an isolated 30-cat founding fixture with finite
extra construction supplies and research currency. Food, water, cat needs and
survival automation retained their founding defaults. A checksum-verified save at
2,630.6 simulated seconds records these normal UI outcomes:

- Road job `job-62` completed at 328 seconds. Tiles `(3,1)`, `(4,1)`, `(5,1)` and
  `(6,1)` are roads, with no dirt, walls, water or mountains on the route. Materials
  fell from 220 to 216.
- Den `den-75` at `(5,-2)` completed through its entrance at `(5,0)`. It consumed
  16 Planks and eight Blocks, completed 300 work and retained no construction
  inputs. Stored Blocks fell from 120 to 112.
- Hazel staffed the Wood Cutter at `(3,3)` and completed `logs_to_planks` at
  2,317 seconds. One Plank reached station storage and the repeating queue began
  another batch. Stored Planks reconciled as `120 - 16 + 1 = 105`; of the initial
  100 Logs, five were consumed, five were in the next job and 90 remained stored.

All 30 cats remained alive and healthy. A nearby save sample at 2,594.9 seconds
had Health 100, Hunger 85.59 and Thirst 78.38 for every cat, with no blocked cats.
Actual stores held 190.456 Food and 240 Water. The header displayed dashes because
no Accountant had counted a pile; the UI shows reported inventory, not hidden
physical totals. Automatic hunting and water fetching had reached their targets
of 180 Food and 240 Water, so one working cat was expected at that moment.

The later packaged build also passed normal world-click inspection and prerequisite
navigation. A Research Hut purchase persisted as one owned study and 19,995 points.
Hazel entered direct control, moved, and returned to AI with the same ID and an
empty control owner. Wheel and button zoom were captured separately from the
performance sample. Its dedicated native log contained no exception, assertion
or runtime-error marker. After the narrow-window correction, a separate native
run at 1508×912 pixels displayed the village name and all four speed controls
above four nonoverlapping resources. The Cats list scrolled to Fern, inspection
opened at the needs bars, and scrolling, Back and Close remained usable.
The dedicated narrow-window log contained no exception, assertion or runtime-error
marker. That correction does not change the normal-window layout or simulation.

The rebuilt app after the housing and expansion corrections also opened a new
ordinary seed-41 save with 30 cats, 16 buildings and a 3×3 shrine at `(-1,-1)`.
Its save checksum verified. Cats reached the exterior, the zoom button responded,
and a world click opened the Stone Prep inspector with its workplace in view.
The dedicated runtime log contained no exception, assertion or error marker.

The final farm check used another isolated, fully researched save. Public road construction and
Field construction completed before a public 4×3 grain designation. The native
app displayed all twelve crop cells beside the east gate. Work → Clear farm
removed the whole crop rectangle while keeping the Field workplace and road.
The checksum-verified save retained 30 living cats, one completed Field and no
designated farm. Its dedicated native log contained no runtime-error marker.

The native build after the rail and host-permission fixes also opened an unused
seed-41 save. The zoom button changed the view immediately, and clicking a Den
opened its inspector with the building visible beside it. At 114.6 seconds the
checksum-verified save retained 30 living cats, 16 buildings and the centered
3×3 shrine. Its dedicated runtime log contained no error marker.

The final transport and socket-budget source also built as an ARM64 IL2CPP app.
Another unused seed-41 world opened the Cats panel and Fern's inspector, entered
direct control with Tab, accepted walking input and returned to management with
Tab. The checksum-verified save retained all 30 living cats, 16 buildings and
layout version 1. Fern kept identity `cat-26` and resumed AI with no control owner.
The dedicated runtime log contained no exception, assertion or error marker.

The save and performance files continued updating during observation. Current
measured samples are in [the performance report](PERFORMANCE.md). Captures use
synthetic worlds and contain no credentials or played user data.

## Original migration native checks and evidence

Before the corrective revision, the rebuilt app included the Village refresh
regression fix. A 150-cat local world connected to the isolated shared server and
selected the existing 15-cat Mosslight village. Returning restored the 150-cat
local world; reconnect retained
the server address and selected Mosslight again. The open panel tracked every
transition without closing and reopening it.

A separate copy of the synthetic SQLite-converted world opened with 30 cats and
16 buildings. At 35.7 simulated seconds, the imported haul had delivered 2.5 logs
and the unfinished production job had raised stored planks from 20 to 21. Both
original cat IDs remained. The normal roster selected Fixture Moss, entered the
close camera and returned that cat to AI. Runtime logs for those founding,
expanded and imported worlds contained no exception, assertion or runtime error.

The original PR evidence includes game-only management and carrying screenshots,
the research map and an eight-second interaction video through verified Schaffa
links. These use synthetic saves. They contain no identity files, server
credentials or user data.
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
Ten exact-item hauling regressions cover produced Mug recovery and sale,
cancelled production, cancellation and death before and after pickup, competing
claims, full storage, Steward recovery, source occupancy and transfer between
existing stores. Real save/reload checks retain the claim and condition through
blocked delivery. Imported equipment respects per-kind capacity, and mixed
station batches deliver each item only when its accepting store has space.
PlayMode checks that claimed cargo stays invisible before pickup and that carried
exact items follow their cat afterward.
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

Coordinate work rejects foreign claims and physical footprints before reserving
goods and again before construction advances. Founding skips conflicting sites,
including distant starter deposits. A conflicting extinction recovery reports
the block before changing reservations or trades. The signed two-client socket
test denies a foreign road without mutation, then accepts an owned road.
Imported exact outputs on empty, unstaffed queues now wait for a worker to reach
the station. One- and two-worker restart tests verify exclusive adoption and
delivery without changing identity, condition, quality or storage limits.
Four further import regressions keep outputs at a distant station for its
preassigned worker, including blocked access and restart. Actual outbound scalar
and exact cargo delivers separately; inbound work retains its station, inputs
and recipe progress. The importer-only correction passed all 43 authority
tests and the four real SQLite pipeline tests in 23.147 seconds. It does not
change the simulation or Unity sources covered by the suite results above.

The outbound-format checks compare the frozen Rust writer's markers with
actual SQLite and converted state. Destination stock is preserved, carrier-owned
exact equipment and variant goods avoid scalar copies, and station mirror
counters reconcile without consuming unrelated goods. Six focused cases cover
delivery, full storage and restart. The four-test pipeline uses the tracked
synthetic fixture to check the original writer formats directly.

[Gameplay decisions](GAMEPLAY_ACCEPTANCE.md), [save compatibility](PERSISTENCE.md)
and [development commands](DEVELOPMENT.md) describe the implemented behavior and
reproduction steps. No merge, deployment, purchase or production-data change has
been performed for this migration.
