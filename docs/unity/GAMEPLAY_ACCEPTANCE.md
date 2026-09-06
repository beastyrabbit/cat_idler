# Gameplay acceptance

The migration preserves the maintained game's content and physical accounting requirements. The source inventory came from the current Rust implementation, `GAME_VISION.md`, the July 21 implementation audit, and executable catalogs. Older five-cat, 500-study, flat-2D, and TypeScript-parity documents do not define the Unity target.

This is a requirements checklist, not a declaration that every row has passed release acceptance. The final verification report must name the executed scenario groups, seed campaigns, server restart tests, Unity UI checks, and packaged macOS checks. A catalog label or successful purchase alone does not prove a mechanic works.

## Maintained content

| Catalog | Exact inventory | Source |
| --- | --- | --- |
| Research | 487 studies, comprising 165 building, 167 recipe/resource, and 155 general upgrades | `Catalog.Generated.cs`, exported from `cat-sim::research_catalog` |
| Physical recipes | 108 | Executable `station_recipes` descriptors and finite physical recipe rules |
| Stations | Mill 20 recipes, Sawmill 3, Workshop 20, Smelter 5, Wood Cutter 2, Stone Prep 9, Woodworking 10, Clothier 12, Tannery 11, Smithy 16 | `Catalog.Recipes` |
| Buildings | 25 types, including the founding shrine; 24 player-constructible types | `Catalog.Buildings` |
| Worker skills | 19 | Hunt, Fishing, Build, Ritual, Fight, Train, Quarry, Woodcut, Forage, Fetch Water, Mill, Process, Craft, Textile, Metalwork, Farm, Haul, Research, Scout |
| Resources | 32 named kinds | Blessings are currency; Tool, Weapon, and Armor totals derive from finite item identities |
| Finished goods | 10 kinds, 9 materials, 5 quality bands | Only maintained recipe combinations produce goods; the 450 representable combinations are not 450 recipes |
| Crops | Catnip, Grain, Herbs | Exterior farm plots |
| Officers | 7 specialist offices | Table below |
| Existing action protocol | 53 variants | Earlier audit prose says 52 and predates exact work-slot editing |

Run `tools/catalog-export` to regenerate the complete C# catalog from maintained Rust. The generated catalog retains stable research and recipe IDs, costs, AND prerequisites, effect operations, and map coordinates. The old recipe/resource matrix's count of 104 predates Thread and the added Mug recipes.

## Founding and progression

A personal village starts with 15 adults, three completed Dens and one each of Wood Cutter, Stone Prep and Woodworking. The communal village starts with 30 adults, six Dens, two of each production yard, a Research Hut, Barracks and Food Storage. These are the maintained seven-building and sixteen-building blueprints, including each shrine. The enclosure radius is six tiles for a personal village and nine for the communal village.

New villages place the shrine on exactly 3×3 tiles centered inside a complete road ring. Four cardinal gates connect to clear exterior paths and all eight finite starter deposits. Short founding road spurs connect every building entrance to the shrine without crossing building or stockpile footprints. New buildings retain an explicit doorway, and movement crosses their perimeter through that doorway.

Later construction requires an entrance on the existing shrine-connected road network. Disconnected paving and worn dirt do not qualify. Roads consume materials and physical work; placing a building creates no free access road. Construction cannot cover the shrine ring, existing entrances, or queued road, rail and expansion-wall paths. An entrance also avoids a planned wall, selecting another connected doorway when one is available. Automatic housing and both manual house-job aliases skip disconnected sites. Existing Unity and imported saves retain their shrine coordinates, footprints and gate convention. Compatibility reads imported `road_built` paths without relaying terrain or resetting progress.

Expansion checks the complete road route from each connected entrance back to
the shrine after the proposed walls. It permits an alternative route through a
gate and rejects a cut upstream of the doorway. New construction uses the same
future road check when expansion is pending. Fishing designations require a
free mapped shore, protecting roads, entrances, existing work sites and reserved
infrastructure while allowing both owned and unclaimed shores.
Road, rail and expansion jobs also reject overlapping reservations in either
planning order. Loaded expansion pauses before spending when it encounters a new
conflict. Expansion preserves any connection that currently reaches the shrine;
it does not attempt to repair a pre-existing disconnected entrance.

Each unupgraded Den has five permanent beds. Personal founding stock is exactly Food 50, Water 100, Herbs 16, Materials 60, Planks 10, and Blocks 10. The communal stock is twice that amount. Every other resource starts at zero. Empty station containers add capacity without adding goods.

The founding Leader retains deficit-driven hunting, water fetching, and scouting. At 15 cats the upper limits are six hunters, two water fetchers, and one scout. Specialist vacancies keep production, farming, management, research labor, rituals, defense staffing, and expansion under player control. Emergency water work must use a real source and carrier.

| Office | Completed workplace | Required research | Automated work |
| --- | --- | --- | --- |
| Steward | Workshop | `basic_tools` | Workshop staffing and hauling |
| Accountant | Accounting Tent | `basic_tools` | Physical stockpile counts |
| Forester | Sawmill | `sawmill` | Logging, replanting, quarrying, timber and tool stations |
| Farmer | Field | `irrigation` | Food, forage, farms and milling |
| Captain | Barracks | `barracks` | Defense, training, metalwork |
| Loremaster | Research Hut | `research_hut` | Research labor and offerings |
| Cloth Leader | Clothier | `textiles` | Fibre, Thread, Cloth and Leather stations |

An office requires both prerequisites and a living appointed cat. Appointment gives no free productivity bonus. Automatic slots, farm assignments and jobs retain their responsible office. Removing an officer releases those workers and claims while preserving manual orders. Automatic queues choose funded recipes by stock deficit and leave workers available for survival work. The player can buy every affordable study; the living Leader may buy at most one per rolling day.

Six legacy upgrades remain distinct from the research graph. Click Power, Supply Speed, Hunt Mastery, Build Mastery, Ritual Mastery and Resilience spend Blessings, with escalating base costs of 2, 3, 5, 5, 6 and 7. Their effects change actual boosts, work, yield and needs. Leadership elections and five-player removal petitions have separate saved ballots and deadlines.

Breeding starts only after 36 game-hours, requires a reserved permanent bed, and takes 18 game-hours of gestation. Migration starts after 30 game-hours, checks every 12 hours, and gives an unhoused arrival 36 hours of probation after reaching the village. Ordinary old-age risk starts at 240 hours; Leaders and healers start at 288 hours. Extinction releases transient claims and restores a complete founding state with new identities only when its founding footprint avoids foreign property. A conflict leaves the state intact and records a recovery-blocked event.

## Feature checks

| Feature | Acceptance |
| --- | --- |
| Survival | Cats physically fetch servings, eat or drink at a destination, and sleep in reserved beds. Fish and Preserves feed cats; finite Medicine heals them. Cargo blocked by full storage cannot prevent critical needs. |
| Work | One cat owns at most one active job or vehicle route. Starting or adopting work preserves unrelated carried goods at the cat's physical location before assigning ownership. Skill and preferences affect eligible matching and the corresponding work rate. Idle roaming creates no resources or fake completed jobs. |
| Production | Every recipe traverses source pile, carried input, station input, work, output and final delivery. No aggregate output credit precedes delivery. Imported batches deliver one scalar stack or exact item at a time, honoring each destination's filter and capacity while retaining the rest. Repeat queues rotate, pause preserves state, and each extra worker has a separate queue. |
| Construction | A free footprint and a shrine-connected entrance precede reservation of the exact bill. A builder carries inputs, and construction advances only after the inputs arrive. Losing the entrance connection pauses work without consuming inputs or releasing claims. Death or reassignment preserves delivered inputs and paid progress. Food and surplus Planks remain recoverable; construction consumes only its bill. |
| Roads and walls | Roads consume delivered material per completed tile. Pending road and rail paths reserve their footprints and recheck permanent obstacles before work. Interrupted routes retain job identity and suspended claims; loose cargo spills do not block their resumption. Expansion rejects walls crossing existing buildings, entrances or permanent stockpiles. A loaded conflict pauses before further work or spending; removing a conflicting gather pile resumes the same job. Expansion preserves paid wall segments, keeps the layout's gates open and clears obsolete interior walls. New layouts have four gates. Farms remain exterior. |
| Farms | A present worker grows the plot, harvests bounded baskets and walks them to a finite adjacent handoff. A separate physical haul credits storage. Full handoffs suspend work without losing harvest. |
| Storage | Capacity and resource filters apply to actual containers, including station containers and imported per-kind exact-item limits. Claims cannot overdraw a pile or free space before physical pickup. Removing a pile containing crafted items leaves a spill recoverable through manual or steward hauling. Cancellation, death and full storage preserve exact identity and condition. |
| Accountant | The assigned cat returns to the tent, visits reachable piles, dwells to count and returns. Unreachable piles remain stale. Wire projections must not reveal exact uncounted quantities. |
| Equipment | Tools, Weapons and Armor keep one ID through storage, equipment, damage, repair and trade. Armor production and unequipping honor Armor-only storage without scalar duplicates or changed condition. Broken equipment remains physical. Condition and quality affect real work or combat. |
| Exploration | Scouts search by physical observation. Notes remain provisional until shrine return. Targeted scouting must not read a hidden resource before observation. |
| Ecology | Finite source deposits deplete at pickup. Fishing uses a shared 24-fish habitat and replenishes 0.5 fish per game-hour. Replanting requires a depleted stump. Imported overlapping deposits keep separate quantities. |
| Research | Every building, recipe, resource and general modifier has a measurable runtime consumer. Retired capacity IDs receive their documented one-time migration refund. |
| Shrine | New founding shrines occupy exactly 3×3 tiles with a complete road ring outside. Construction and avoid zones cannot block shrine access. Food, Herbs and Materials offerings travel there. Tithes and offerings use one spendable Blessing balance. Fertility changes when that balance changes. |
| Defense | An undefended raid moves, harms the colony and takes finite goods. Warriors and exact equipment change combat. Each manual defense action deals one six-point hit. |
| Trade | Two authorized villages must discover one another. Barter owns finite escrow and a saved land route. Restart, cancellation and full destination storage preserve both scalar goods and exact items. An imported trade whose outward delivery is complete rejects cancellation so its earned payment can finish returning. |
| Visiting trader | The merchant physically reaches the shrine before trading. Cached land routes recheck new fence edges and use the existing replanning path when blocked. Finite stock, coin, wagon weight, stay deadline and departure survive restart. |
| Rail and shipping | Constructed track, docks and vehicles precede routes. A living driver owns the vehicle, cargo and explicit route. Rail obeys authoritative tile and fence-edge passability in both directions, retaining cargo and ownership while blocked and resuming when access returns. After a needs trip, the driver physically returns to the same vehicle before its route resumes. Offshore drivers retain needs drain and damage, travel to shore and meet needs there. Exact equipment travels with its identity and condition. A cancelled loaded vessel returns to its source dock and retains cargo aboard while the source is full. |
| Direct control | Enter, move, interact and leave preserve the same cat. Movement advances with server time, never action count. Control heartbeat renews only the same holder. Death and handoff release jobs, routes and cargo correctly. |
| Control needs | A controlled cat may eat or drink one carried serving at the dining destination, or rest at its assigned bed. Remote, unauthorized and free-serving attempts fail. |
| Persistence | Save and restart preserve the full authoritative aggregate. The maintained SQLite converter must resume active work, not merely retain opaque unused fields. |
| Long runs | Fresh and established villages across several seeds survive without unintended resets. Population turnover, housing pressure and production remain active. Report measured tick cost and tested population. |

## Deliberate implementation choices

The Unity simulation uses explicit C# data and a single authority. Movement is kinematic on an authoritative tile route; Unity physics does not decide resource ownership or job completion. Controlled motion advances in at most 0.05-second steps while needs and economy advance at one-second boundaries. Third-person control and the management camera share this world.

Founding clears permitted exterior paths from four gates to the finite starter deposits. The ownership check covers these approaches before any terrain changes. Authored roads and worn dirt remain distinct: roads form the building-access network, while traffic can wear ordinary ground into a dirt path. Path searches use deterministic A* with room to go around settlement walls. Existing saves keep their layout; loading or inspecting them does not widen a shrine or bulldoze property.

Fresh Unity farms complete a staffed cycle in two game-hours before modifiers. Imported farms retain their 24-hour clock, crop-specific yield and fertility behavior, so a saved mature crop does not suddenly finish on its next tick. Imported one-tile frontier projects likewise retain their exact claimed tile and boundary geometry; new Unity expansions are staged settlement-wide projects.

Entering control aboard a vessel keeps the same cat aboard while it returns to the source dock. Foot movement becomes available after docking. If its driver dies at sea, the wreck retains its exact location and cargo. The player can build physical bridge access to it, after which salvage leaves a reachable pile for ordinary hauling. No replacement crew or cargo appears at the dock automatically.

Generated output studies on exact-item stations improve material efficiency. They still create one complete item per recipe. Scalar recipes keep their ordinary yield improvement. This makes the previously inert Smithy and Woodworking workflow studies useful without fractional equipment or duplicate IDs.

Founding access follows the maintained `building_placement_research` rule. A building without an explicit unlock declaration is available at founding. Declared unlocks still require their study. This is why constructing a Den or Workshop does not require an invented research node.

## Executable checks

`tools/scenarios` and Unity EditMode compile the same acceptance scenario source. The runner exits unsuccessfully on assertions, prints each named case and can filter a single case or group. Use `--list` for the current inventory. The shared tests cover public actions, all recipes, the full dependency-order purchase graph, building and service effect differences, resource study differences and seeded campaigns.

The scenario README states each fixture's boundaries. A funded station test proves finite production behavior. It does not prove a fresh colony can earn every input without guidance. The final release evidence must include both focused scenarios and longitudinal player journeys, plus server authorization and restart tests.

The final simulation run passed 650 noncampaign scenarios in 30.6156 seconds, including all 108 recipes, 487 research purchases, runtime effects, 148 regressions and the three shipping recovery scenarios. This run overlapped the Unity and authority suites; its duration is validation evidence, not an isolated benchmark. Earlier failures led to corrections for carried Food and surplus Planks, Armor storage, new-work cargo, and resumed infrastructure. Thirty territory cases cover foreign claims and physical footprints during planning, work, founding and recovery. Five rail cases cover newly completed walls, water and fence edges, finite scalar and exact cargo, return travel and recovery on the same route. Four further transport cases cover physical return after needs, blocked reboarding, offshore sleep without stopping needs drain, and an accepted caravan interrupted by a newly completed farm boundary. The same cached route completes its exchange exactly once after public farm clearing and another expansion restore access.

Seventeen new layout cases cover both founding blueprints, the 3×3 shrine and ring, connected entrances, four exits and finite resource access, rejected disconnected construction, physical doorway crossing, paid road construction, interrupted access, preserved legacy geometry, avoid zones and queued road/rail collisions. The paid access chain consumes four Materials before a building becomes reachable. Field fixtures establish access roads before play; the rail fixture uses a site outside the shrine ring. Recipe and accounting assertions remain intact.

Five further merchant and raid cases cover blocked cached edges on arrival and departure. A public farm expansion interrupts both an automatic merchant and a seeded active raid. Their existing replanning finds lawful detours while preserving identities, finite goods, purse and loot. The merchant then completes a paid purchase, and the raid reaches exactly one shrine theft. PlayMode separately checks that stopped and resumed caravan snapshots render at their actual coordinates.

Ten exact-item hauling cases cover produced Mug recovery and sale, cancelled production output, cancellation and death before and after pickup, competing claims, full storage, steward recovery, source occupancy until pickup and transfer between existing stores. They preserve identity, condition, material and quality without scalar copies. Separate authority tests reload claimed and carried items through full-storage blocking, enforce imported Tools/Weapons/Armor limits, and finish mixed imported output batches one item at a time without bypassing filters. PlayMode verifies that exact cargo appears after pickup and follows the carrier.

Nine extended scenarios passed in 213.7647 seconds after the complete road and shore-placement corrections: fresh communal colonies for 48 hours, established communal colonies for 72 hours, and shared communal plus personal villages for 48 hours, each at seeds 7, 41 and 127. Each compares a twin advanced in smaller partitions, checks invariants and tracks original founding identities so migrants cannot hide deaths. All founding cats survived without a reset. This run used `61d29f1`, before the final farm-edge projection, crop-rendering and transport corrections. These workloads create no road, rail or expansion jobs, designated farms, transport routes, active village trades or imported batch outputs. The final 650-case run directly covers infrastructure guards, physical construction, blocked transport and interrupted resumption. Authority/import tests cover imported output delivery and restart separately. The established campaign has one staffed repeating Wood Cutter and finite initial supplies; it does not establish indefinite operation of a mature economy.
