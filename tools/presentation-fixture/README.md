# Private native review saves

Create a new isolated save for normal native UI and performance checks:

```sh
dotnet run --project tools/presentation-fixture/Forest.PresentationFixture.csproj -- /absolute/new/founding.json 30
dotnet run --project tools/presentation-fixture/Forest.PresentationFixture.csproj -- /absolute/new/expanded.json 150
dotnet run --project tools/presentation-fixture/Forest.PresentationFixture.csproj -- /absolute/new/defense.json 150 --defense
dotnet run --project tools/presentation-fixture/Forest.PresentationFixture.csproj -- /absolute/new/logistics.json 30 --logistics
dotnet run --project tools/presentation-fixture/Forest.PresentationFixture.csproj -- /absolute/new/officer-materials.json 30 --officer-materials
dotnet run --project tools/presentation-fixture/Forest.PresentationFixture.csproj -- /absolute/new/workshop-controls.json 30 --workshop-controls
dotnet run --project tools/presentation-fixture/Forest.PresentationFixture.csproj -- /absolute/new/equipment-controls.json 30 --equipment-controls
dotnet run --project tools/presentation-fixture/Forest.PresentationFixture.csproj -- /absolute/new/open-slot-controls.json 30 --open-slot-controls
dotnet run --project tools/presentation-fixture/Forest.PresentationFixture.csproj -- /absolute/new/chain-controls.json 30 --chain-controls
```

Pass the created path to the native app with `--forest-save`. The tool refuses an existing save or identity directory, uses `LocalAuthority.CreateNew`, validates, saves, and reloads the result. It never prints credentials. Its adjacent `.identity` directory is private local authentication state and must not be published.

The 30-cat mode retains the maintained founding blueprint and adds finite review supplies, 20,000 research points, and 200 blessings before play. The 150-cat mode creates 30 Dens, all building types, finite goods, a known radius-23 enclosure, exterior resource sources, and staffed repeating production chains. It advances 15 seconds so the saved workers already have real jobs. Enough research remains unowned for ordinary research purchases.

Both modes use the centered 3 × 3 shrine, its complete surrounding road ring, and four cardinal gates with walkable exits. Each house and workplace has a persisted entrance on the shrine-connected road network. The expanded fixture authors deterministic shortest road spurs around buildings and its stockpile, including the exterior field's route through a gate. It keeps the original finite inventory and workload: 150 cats, 30 Dens, 54 buildings, 13 staffed work positions and 12 queued recipes. The stockpile sits clear of the shrine ring, and building placement skips its footprint.

Before saving and after reloading, the generator checks the ring, all four gates, nonoverlapping footprints, connected entrances and actual authoritative paths through those entrances. This makes the expanded performance workload exercise entrance routing rather than unrestricted movement through workshop walls. Creating a fixture is authoring test data; it does not simulate the time or resources spent on constructing that prepared village.

This is a test-only command. It adds no developer bypass to the game or server and does not migrate or overwrite an existing save.

For generation version 1, the expanded fixture authors a level radius-23 plateau
before placing its roads and buildings. It creates new dungeon sites outside
that expanded settlement, rather than retaining the smaller founding world's
stair openings inside the prepared town. Its finite pond has an explicit bed
and water depth. The 30-cat fixture retains the normal communal founding terrain.

The optional `--defense` mode prepares the expanded village for physical defense
and Captain training checks. It keeps the same finite supplies, workshops and
other labor skills. The Captain office starts vacant. North Defender waits on
the north road at `0,18` with fight skill 2; South Defender waits at `0,-18` with
fight skill 3. Each has a raider three tiles farther along its road, at `0,21`
and `0,-21`. Both raids have one health, allowing ordinary combat to finish in
roughly 10 to 15 seconds after contact. They use normal movement and can breach
the shrine or escape if left alone. Pause immediately after opening the save
to inspect and issue orders before advancing time.

Captain Candidate waits at `0,4`, and Training Candidate at `0,5`. These four
cats are free adults with ordinary needs and no carried cargo. Other cats have
no fight skill or fight preference, so readiness begins at one trained cat
against a target of ten. After testing exact manual targets and cancellation,
appoint Captain Candidate through Officers and repel the raids. The reachable,
unpaused Barracks and Training Candidate's fight preference let normal Captain
automation demonstrate the remaining training shortfall. No Captain, defense
order or training job is injected by this mode.

The optional `--logistics` mode retains the 30-cat founding layout for a smaller
workshop review. It removes the added building materials, retains 1,000 finite
Food and Water, and places three one-tile piles containing five Logs each. The
lexically first pile is blocked by a wall; both later piles are reachable. The
generator prints their coordinates. These prepared piles and the wall are test
data, not changes to the runtime construction rules.

The earlier Wood Cutter at `3,-8` starts deliberately paused and empty. The
later one at `3,3` is unpaused and empty. Logistics Worker waits at its entrance
with processing experience; Forester Candidate is free and the office is vacant.
No production, staffing or officer order is injected. Pause on opening, assign
Logistics Worker to the later cutter and add one nonrepeating Logs To Planks
order through the normal UI. It should collect from a reachable pile and deliver
one Plank, leaving the five inaccessible Logs intact. Source-closure recovery
is covered by the deterministic and authority regressions. This mode does not
inject a later route change.

The Forester requires the Sawmill study and a completed Sawmill. This founding
fixture does not grant either. Build them through normal play or use the
prepared 150-cat fixture for an officer check. There, appoint a free adult as
Forester, pause the Wood Cutter and release its worker. The paused position
should remain empty while the world advances; Resume should permit automatic
staffing again. The deterministic regressions cover two-station competition and
the last-worker reserve boundary. Neither native fixture establishes a
sustainable economy.

The optional `--officer-materials` mode reviews a Cloth Leader's choice between
blocked Thread and reachable Fibre in the 30-cat founding village. It converts
the duplicate Woodworking at `-8,3` into a completed Clothier, retaining its
footprint, entrance and road connection. Its managed store accepts the ordinary
Clothier inputs and outputs. The mode owns Textiles and its prerequisites, but
leaves later recipe studies unowned. It retains 1,000 finite Food and Water and
removes the other added materials. A one-tile pile has five Thread behind a
wall, and a separate reachable pile has five Fibre. The generator prints both
coordinates. All inventory reports remain uncounted.

Pause immediately after opening this save. In Officers, select Cloth Leader
Candidate and appoint it as Cloth Leader. Open Work, inspect the Clothier and
resume time. Textile Worker waits at its entrance with textile experience and
a preference for textile work. Normal officer automation should staff that cat
and choose Fibre To Thread, consuming the five reachable Fibre and producing
five Thread. The inaccessible pile's five Thread must remain untouched. The
other recipe is displayed as Fibre To Cloth, although its actual input is five
Thread and its output is one Cloth. Its initial output deficit is 32, versus
27 for Fibre To Thread, so aggregate inventory alone would select that blocked
recipe. The expected first job demonstrates reachable input selection.

Use 8× after inspecting the first recipe to complete the ordinary 600-work
batch sooner. Once the first output is delivered, the officer may choose Fibre
To Cloth using that newly reachable Thread. The initial fixture contains no
officer, worker slot, queue, reservation or production job. Appointment,
staffing, claims, needs and production all run through the normal game systems.
This mode authors finite test data and does not establish sustainable production.

The optional `--workshop-controls` mode prepares the 30-cat founding village for
manual work and an in-place return to Forester control. It converts the duplicate
Carpentry Hall at `-8,3` into a completed Sawmill, retaining its connected entrance,
and owns the Sawmill study and prerequisites. The Forester office stays vacant.
Every Forester workplace starts paused and empty, so appointment cannot race the
player's manual setup. Workshop Worker waits at the later Wood Cutter's entrance
with processing experience. Forester Candidate is a free adult.

Three reachable one-tile piles contain ten finite Logs, ten Stone and two Blocks.
The main store retains 1,000 Food and Water. There are no live Planks. One authored
historical storage report records three Logs and one Plank at `0h`, with zero
Blocks and Stone. All other storage reports remain uncounted. This report is
prepared test data, not evidence of an Accountant visit, and intentionally differs
from actual goods. It becomes older as ordinary play advances the world.

Pause on opening. Assign Workshop Worker to the later Wood Cutter, then add a
finite Logs To Planks order and a second repeating order through its queue. The
input panel should show Logs, a bill of five, and reported stock of three with
the count timestamp and uncounted-pile count. Appoint Forester Candidate through
Officers. Resume the exact worker, then use Return to Forester. The same cat and
manual queue should remain. Pause and Resume work should retain its current
batch; Resume followed by Return to Forester restores delegation. The repeating
manual order continues until the player changes it, even under the officer.
Ten Logs can fund two ordinary batches. They do not support indefinite work.

Inspect the remaining Carpentry Hall at `-8,-8`, assign another free cat and add
Planks And Blocks To Tools to its queue while it remains paused. This shows both
ingredient names, bills of two, historical reported amounts and navigation to
the existing Wood Cutter and Stone Yard suppliers. The report
must remain unchanged as hidden goods move. View inventory & hauling opens the
ordinary Stores panel. No officer, worker slot, queue, reservation or production
job is injected by this mode. Normal controls own all appointment, staffing,
claims and production observed afterward.

The optional `--equipment-controls` mode prepares four finite exact items in the
30-cat founding village. Equipment Runner is a free adult on the shrine's north
road at `0,3`. A reachable one-tile store at least six tiles away holds
`equipment-review-tool` (wood, quality 3, condition 17/42),
`equipment-review-weapon` (iron, quality 2, condition 31/60), and
`equipment-review-armor` (leather, quality 4, condition 23/50). A separate walled
one-tile store holds `equipment-review-blocked-weapon` (stone, quality 1,
condition 9/20). The generator prints the two source coordinates and routes.
The main store retains 1,000 finite Food and Water; other material stocks are
empty. Every storage report is authored at `0h` with those initial exact counts.
These reports are setup data, not evidence of an Accountant visit.

Pause on opening. In Stores, select Equipment Runner and request a reachable
exact item. The order should reserve that item and send the same cat to its
physical source. Cancel the exact order during travel, then request it again
and let the cat arrive. Returning worn gear should require physical travel to
accepting storage and preserve its identity, quality and condition. Test the
blocked weapon's refusal separately. No equipment order, worn item, job, claim,
officer, queue or worker slot is injected. Captain remains vacant, and the
player controls all retrieval, cancellation and return actions.

The optional `--open-slot-controls` mode prepares a Clothier with two empty
positions in the 30-cat founding village. It converts the duplicate Carpentry
Hall at `-8,3`, keeping its footprint, connected entrance and managed storage.
The generator buys Clothier Crews and its prerequisites through public research
actions using the fixture's prepared research currency. Public staffing, queue
and release actions leave position 1 paused with a manual repeating Fibre To
Thread order. Position 2 stays unpaused with two finite manual orders, Fibre To
Thread followed by Fibre To Cloth. Neither position has a worker or active job
when saved. This prepared history is test setup, not a claim that a player made
these edits during review.

Pause on opening. Appoint Cloth Leader Candidate through Officers, then resume
and inspect the Clothier. Normal automation should staff only position 2 with
Textile Worker, who waits at the connected entrance with textile experience.
Position 1 must remain empty and paused with its repeating manual order. The
later position must keep both finite manual orders. Five finite Fibre in a
reachable pile fund the chain. The researched 12% output modifier produces 5.6
Thread, then consumes five Thread to produce 1.12 Cloth, leaving 0.6 Thread.
No officer replenishes Fibre, and the fixture has no other production goods.
Use 8x after inspecting the initial staffing and physical pickup to complete
both ordinary batches sooner. This finite chain does not establish a
sustainable mature economy.

The same mode includes the four exact unworn items and two sources described
under `--equipment-controls`. Equipment Runner is free on the north road at
`0,3`. Use the inspector's Equipment control to reach Stores, then check exact
retrieval, cancellation and physical return. This navigation setup contains no
equipment orders or item claims. Food and Water start at 1,000 each and are
finite. All storage counts at `0h` are authored reports, not Accountant visits.
The generator validates the exact slots, finite goods, routes and empty claims
before saving and again after strict reload. It never advances the prepared
world, modifies played saves or reads identity credentials.

The optional `--chain-controls` mode prepares the 30-cat founding village for a
finite chain across three workshops. It starts with no staffing, queues,
officers, jobs, claims or items. All three founding recipes retain their exact
baseline bills, outputs and 600-work durations; no research yield or efficiency
modifier is prepared. Food and Water start at 1,000 each. Two separate reachable
off-road piles hold ten Logs and ten Stone. Planks, Blocks and Tools start at
zero. Reports at `0h` are authored fixture counts, not Accountant visits.

Pause on opening and preview the recipes and their ingredient reports before
staffing. Use normal controls to assign **Plank Worker** to the later Wood
Cutter, **Block Worker** to the northern Stone Prep and **Tool Worker** to the northwest
Woodworking station. Each named free adult waits at that workplace's entrance
with 40 skill in its labor. That skill shortens work; it does not change yields.
Queue two finite **Logs To Planks** orders, two finite **Stone To Blocks**
orders and one finite **Planks And Blocks To Tools** order. Leave Repeat off and
avoid buying output or efficiency studies during the conservation check.

The upstream bills consume five raw goods for one processed good per batch.
Both pairs of finite orders therefore deliver two Planks and two Blocks, which
fund one exact wooden Tool at Woodworking. Workers must pick up, work on and
deliver each material through physical routes. The tool is an identity with
material, quality and condition, rather than a scalar stock entry. An initial
missing-input status at Woodworking is expected until upstream deliveries fund
its whole bill. Inspect Events and Stores after completion, then quit normally
before any closed-save conservation audit. Survival work may consume the finite
provisions. This setup exercises a composed finite chain and does not establish
sustainable production. The generator validates exact supplies, unmodified
recipes, empty ownership and physical routes before saving and after reload;
it never advances the prepared world or reads identity credentials.

For a supply-report check, pause and remove a raw-material stockpile before
staffing. Its goods remain in an external spill. A workshop worker can collect
from that reachable spill directly, although the bill's historical storage
total excludes it. Alternatively, use **View inventory & hauling**, choose a
free carrier and **Haul from pile** to consolidate into accepting storage.
Selecting the staffed worker retains that busy carrier and refuses the haul;
**Choose available cat** explicitly enables automatic assignment. Reports can
retain moved goods, so verify finished output separately from reported counts.
