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

Breeding starts only after 36 game-hours, requires a reserved permanent bed, and takes 18 game-hours of gestation. Migration starts after 30 game-hours, checks every 12 hours, and gives an unhoused arrival 36 hours of probation after reaching the village. Ordinary old-age risk starts at 240 hours; Leaders and healers start at 288 hours. Extinction must release transient claims and restore a complete founding state with new identities.

## Feature checks

| Feature | Acceptance |
| --- | --- |
| Survival | Cats physically fetch servings, eat or drink at a destination, and sleep in reserved beds. Fish and Preserves feed cats; finite Medicine heals them. Cargo blocked by full storage cannot prevent critical needs. |
| Work | One cat owns at most one active job or vehicle route. Skill and preferences affect eligible matching and the corresponding work rate. Idle roaming creates no resources or fake completed jobs. |
| Production | Every recipe traverses source pile, carried input, station input, work, output and final delivery. No aggregate output credit precedes delivery. Repeat queues rotate, pause preserves state, and each extra worker has a separate queue. |
| Construction | The exact bill is reserved before work. A builder carries inputs, and construction advances only after the inputs arrive. Builder death or reassignment preserves delivered inputs and paid progress. |
| Roads and walls | Roads consume delivered material per completed tile. Expansion retains the old enclosure while a builder completes outer wall segments, then cuts over the gate. Farms must remain exterior. |
| Farms | A present worker grows the plot, harvests bounded baskets and walks them to a finite adjacent handoff. A separate physical haul credits storage. Full handoffs suspend work without losing harvest. |
| Storage | Capacity and resource filters apply to actual containers, including station containers. Claims cannot overdraw a pile. Removing a pile containing exact equipment must leave a recoverable location. |
| Accountant | The assigned cat returns to the tent, visits reachable piles, dwells to count and returns. Unreachable piles remain stale. Wire projections must not reveal exact uncounted quantities. |
| Equipment | Tools, Weapons and Armor keep one ID through storage, equipment, damage, repair and trade. Broken equipment remains physical. Condition and quality affect real work or combat. |
| Exploration | Scouts search by physical observation. Notes remain provisional until shrine return. Targeted scouting must not read a hidden resource before observation. |
| Ecology | Finite source deposits deplete at pickup. Fishing uses a shared 24-fish habitat and replenishes 0.5 fish per game-hour. Replanting requires a depleted stump. Imported overlapping deposits keep separate quantities. |
| Research | Every building, recipe, resource and general modifier has a measurable runtime consumer. Retired capacity IDs receive their documented one-time migration refund. |
| Shrine | Food, Herbs and Materials offerings travel to the shrine. Tithes and offerings use one spendable Blessing balance. Fertility changes when that balance changes. |
| Defense | An undefended raid moves, harms the colony and takes finite goods. Warriors and exact equipment change combat. Each manual defense action deals one six-point hit. |
| Trade | Two authorized villages must discover one another. Barter owns finite escrow and a saved land route. Restart, cancellation and full destination storage preserve both scalar goods and exact items. |
| Visiting trader | The merchant physically reaches the shrine before trading. Finite stock, coin, wagon weight, stay deadline and departure survive restart. |
| Rail and shipping | Constructed track, docks and vehicles precede routes. A living driver owns the vehicle, cargo and explicit route. Exact equipment travels with its identity and condition. A cancelled loaded vessel returns to its source dock and retains cargo aboard while the source is full. |
| Direct control | Enter, move, interact and leave preserve the same cat. Movement advances with server time, never action count. Control heartbeat renews only the same holder. Death and handoff release jobs, routes and cargo correctly. |
| Control needs | A controlled cat may eat or drink one carried serving at the dining destination, or rest at its assigned bed. Remote, unauthorized and free-serving attempts fail. |
| Persistence | Save and restart preserve the full authoritative aggregate. The maintained SQLite converter must resume active work, not merely retain opaque unused fields. |
| Long runs | Fresh and established villages across several seeds survive without unintended resets. Population turnover, housing pressure and production remain active. Report measured tick cost and tested population. |

## Deliberate implementation choices

The Unity simulation uses explicit C# data and a single authority. Movement is kinematic on an authoritative tile route; Unity physics does not decide resource ownership or job completion. Controlled motion advances in at most 0.05-second steps while needs and economy advance at one-second boundaries. Third-person control and the management camera share this world.

Founding clears an unclaimed exterior footpath from the gate to the finite starter deposits. A generated lake or mountain therefore cannot isolate a new village's only exit. Path searches use deterministic A* with room to go around settlement walls. Regression scenarios cover the former asymmetric return route and an initially inaccessible personal founding.

Fresh Unity farms complete a staffed cycle in two game-hours before modifiers. Imported farms retain their 24-hour clock, crop-specific yield and fertility behavior, so a saved mature crop does not suddenly finish on its next tick. Imported one-tile frontier projects likewise retain their exact claimed tile and boundary geometry; new Unity expansions are staged settlement-wide projects.

Entering control aboard a vessel keeps the same cat aboard while it returns to the source dock. Foot movement becomes available after docking. If its driver dies at sea, the wreck retains its exact location and cargo. The player can build physical bridge access to it, after which salvage leaves a reachable pile for ordinary hauling. No replacement crew or cargo appears at the dock automatically.

Generated output studies on exact-item stations improve material efficiency. They still create one complete item per recipe. Scalar recipes keep their ordinary yield improvement. This makes the previously inert Smithy and Woodworking workflow studies useful without fractional equipment or duplicate IDs.

Founding access follows the maintained `building_placement_research` rule. A building without an explicit unlock declaration is available at founding. Declared unlocks still require their study. This is why constructing a Den or Workshop does not require an invented research node.

## Executable checks

`tools/scenarios` and Unity EditMode compile the same acceptance scenario source. The runner exits unsuccessfully on assertions, prints each named case and can filter a single case or group. Use `--list` for the current inventory. The shared tests cover public actions, all recipes, the full dependency-order purchase graph, building and service effect differences, resource study differences and seeded campaigns.

The scenario README states each fixture's boundaries. A funded station test proves finite production behavior. It does not prove a fresh colony can earn every input without guidance. The final release evidence must include both focused scenarios and longitudinal player journeys, plus server authorization and restart tests.

The September 6 simulation run passed 536 focused acceptance scenarios in 8.58 seconds, including all 108 recipes, 487 research purchases, runtime effects, 34 regressions and the three shipping recovery scenarios. It also passed nine extended scenarios: fresh communal colonies for 48 hours, established communal colonies for 72 hours, and shared communal plus personal villages for 48 hours, each at seeds 7, 41 and 127. Each scenario compares a twin advanced in smaller partitions, checks invariants and tracks original founding identities so migrants cannot hide deaths. The complete matrix took 170.87 seconds on this Mac. Shipping cancellation and bridge salvage were checked separately after the campaign run. The maintained import and Unity UI results are recorded by their respective test runners.
