# World generation

New worlds use generation version 1. A saved world stores its version, seed,
terrain heights and founding terrain profiles. An older save without a generation
version keeps version 0, including its original unexplored biome and resource
rules. Loading a played save does not replace its terrain.

The surface generator samples coordinates without consuming the actor random
stream. Integer coordinate hashes and interpolated noise produce the same tile
regardless of exploration order. Noise uses floor coordinates on both sides of
zero. Version 0 retains its original truncating division for compatibility.

Elevation combines broad hills with smaller variations. Moisture and temperature
select forest, pine forest, meadow, highland and wetland regions. Adjacent tiles
usually share a biome; a five-tile block no longer chooses an unrelated climate.
The height field has gentle dry slopes suitable for authoritative grid movement.
Sparse high peaks retain the mountain travel requirement.

Rivers have continuous meandering channels with shallow banks and deeper centers.
A ford appears every 43 tiles along a channel, with a gradual transition into and
out of the crossing. At the ford, the entire channel stays below the 0.45-unit
wading limit. Deep channel tiles reach about 1.65 units. Lakes have shallow
shorelines, middle shelves and basins deeper than three units. Their beds blend
between shelves, and their outer banks blend into the surrounding hills.
Water outside founding terrain profiles has one height across each lake and each
river cross-section. This is deterministic terrain generation, without erosion
or fluid simulation.

`Tile.Elevation` records the land or lake-bed height. `WaterSurface` and
`WaterDepth` record the actual water column. The presentation reads these saved
values. A bridge's walking height sits 0.08 units above the water. Looking up a
height never explores a tile or advances the simulation.

The water mesh carries a tint derived from each tile's saved depth. A dedicated
included shader blends the shallow bed into the water, darkens deeper shelves
and animates small ripples. These effects do not change water quantity or paths.

Resources follow the generated environment. Forests supply timber and forage;
meadows supply food, fibre and herbs; wetlands supply clay and sand; highlands
hold stone, ore and gems. Fish occur only in water. All deposits and fish
capacities remain finite authoritative quantities.

A new settlement registers a saved plateau profile before generating its
founding tiles. The level core extends six tiles beyond the settlement radius,
covering its initial gates and resource paths. A twenty-tile shoulder blends
back into the surrounding terrain. Profiles apply lazily, so founding does not
generate thousands of hidden tiles. Existing explored geometry receives the same
profile without replacing finite deposits or built roads. Fish are removed only
where grading drains their water tile. The complete profile area must be clear
of foreign claims and physical buildings, farms or stockpiles before registration.
Overlapping profiles use the lowest blend weight, independent of registration
order. The founding layout then places its finite pond, resource deposits,
roads and buildings.

Grading rejects an existing dungeon entrance and its surface stair opening.
Refounding the same saved plateau preserves its dungeon site. Unsupported
generation versions fail save validation before play.

Twelve focused generation scenarios cover coherent slopes and biomes, positive and
negative coordinates, extreme signed coordinates, exploration order, untouched
actor entropy, connected river rows, complete ford crossings, lake depths,
ecological deposits, fixed version-0 examples, saved height reads and lazy
founding profiles, existing dungeon protection and unsupported versions. The
initial red run took 36.3 milliseconds. The final generation and coordinate run
passed all thirteen cases in 367.2 milliseconds. Native rendering, authority
persistence and longer campaign results are recorded in the acceptance ledger.

Four authority regressions exercise real saved files and authenticated actions.
They cover unchanged terrain and dungeon data through two saves, generation of
the same unexplored frontier after restart, and manual dungeon combat resumed
between 50-millisecond steps. The combat fixture keeps the same cat, enemy,
2.25 carried ore and one exact carried mug. Another case rejects anonymous and
foreign expedition actions before accepting the owner's dispatch and recall.
The projection case checks that an entrance rumor exposes no underground floor,
chest or creature, then reveals only discovered coordinates and remaining goods.
It also checks private rumors and founding profiles.

The initial authority run took 821.8 milliseconds. Save/restart, fractional combat
and ownership passed. The projection regression failed because a rumor included
the undiscovered dungeon layout. All four cases pass after filtering the client
projection, including the later entrance-clearing revision.
