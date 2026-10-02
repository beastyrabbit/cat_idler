# World generation

A saved world stores its generation version, seed, terrain heights and founding
terrain profiles. Loading a played save never replaces its terrain.

| Version | Created by | Used for |
|---|---|---|
| 0 | older saves without a version | their original unexplored biome and resource rules |
| 1 | `World.Create(seed)` | classic worlds: the shared-world server, the tooling and the mechanics scenarios |
| 2 | `World.CreateColony(seed, name, profile, nowUnixMs)` | every new single-player colony, including the demo |

Version 1 is frozen. The founded layout of `World.Create(41)` and a grid of
version 1 samples for seeds 7, 41 and 4242 are pinned by SHA-256 hashes in
`founding.classic_layout_hash_unchanged` and
`regression.world_generation_version_1_samples_unchanged`, so every classic
coordinate in the scenarios stays valid.

## Shared rules (versions 1 and 2)

The surface generator samples coordinates without consuming the actor random
stream. Integer coordinate hashes and interpolated noise produce the same tile
regardless of exploration order. Noise uses floor coordinates on both sides of
zero. Version 0 retains its original truncating division for compatibility.

Elevation combines broad hills with smaller variations. Moisture and temperature
select forest, pine forest, meadow, highland and wetland regions. Adjacent tiles
usually share a biome; a five-tile block no longer chooses an unrelated climate.
The height field has gentle dry slopes suitable for authoritative grid movement.
Version 1 keeps only sparse high peaks as mountains. The figures below come from
one sample of seed 41: every second tile of the 513-tile square around the
origin. On it, version 1 has no mountain tile at all.

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

## Version 2 terrain

Version 2 keeps the version 1 climate, rivers and lakes and changes four things.

- **No film water.** Water shallower than 0.15 units becomes a dry river or lake
  bank (wetland). Fish stocks are `24 + floor(depth * 4)` from the final depth.
  Version 1 leaves 303 such tiles on the seed 41 sample; version 2 leaves none.
- **Landmarks.** A warped, two-octave ridged noise lifts high ground into winding
  ridges whose crests become rock spines (`Landform = "ridge"`, mountain).
  Clustered outcrops on a 20-tile cell grid (55 percent of cells) add a rock core
  (`"outcrop"`, mountain) with a walkable scree skirt that holds stone. Both fade
  out within eight tiles of any river or lake bank zone, so banks stay walkable.
  On the seed 41 sample 1.54 percent of tiles are rock: 0.67 percent outcrops and
  0.86 percent ridges. Climate still follows the smooth height, so rock never
  changes a biome.
- **Slightly more forest.** Forest reaches into moisture 0.40 instead of 0.43:
  39.6 percent forest and 16.3 percent trees on the sample, against 36.0 and 15.2
  percent in version 1.
- **A surface datum.** `World.SurfaceDatum` is subtracted from every generated
  height. A colony sets it to the mean dry height around its site, so the founding
  plateau sits at 0 and the land around it rises and falls naturally.

No two dry, walkable neighbours ever differ by more than the 0.65-unit climbing
limit in the sampled worlds, and rock never stands inside a founding's graded
shoulder.

## Founding

A new settlement registers a saved plateau profile before generating its founding
tiles. Profiles apply lazily, so preparing one does not generate hidden tiles;
existing explored geometry receives the same profile without replacing finite
deposits or built roads. The complete profile area must be clear of foreign
claims and physical buildings, farms or stockpiles. Overlapping profiles use the
lowest blend weight, independent of registration order. Grading rejects an
existing dungeon entrance and its surface stair opening, and refounding the same
saved plateau preserves its dungeon site. Unsupported generation versions fail
save validation before play.

**Version 1** levels a core six tiles beyond the settlement radius and blends a
twenty-tile shoulder back to natural terrain, draining water inside it (fish are
removed only where a tile drains). It reveals the claimed square plus two tiles,
then places a one-tile pond and eight fixed single-tile deposits with straight
dirt approaches, the fixed blueprint and founders in rows on the roads.

**Version 2** is built for a stranger's first look:

- The plateau core is only the radius plus two tiles (the walls and the first
  expansion ring). Its shoulder blends dry ground and water surfaces toward the
  datum; water keeps its depth, so a river beside the village stays a river. The
  shoulder is 8 tiles unless the ground beside it stands unusually high or low:
  it widens to at most 20 tiles so that the blend never exceeds a climbable step
  (six shoulder tiles per unit of height difference). Across the 150 test seeds
  the median shoulder is 8 tiles (97 of 150 sites) and the widest is 20.
- The whole natural ring to Chebyshev radius 34 (`World.ColonyRevealRadius`,
  4,761 tiles) is known at founding, nearest rings first, so nearest-first scans
  over `Village.Known` find the closest source.
- Inside the walls the classic walls, gates and roads stand unchanged. The
  blueprint gains one spare den beside the north road, is mirrored and transposed
  by seed (eight variants) and every footprint is nudged by up to one tile while
  each station still reaches a road. The thirty founders start on shuffled free
  ground around the shrine, open ground before roads.
- Outside, only a one-tile footpath ring is cleared, worn to dirt only where the
  gate roads cross it. Each starter resource upgrades the nearest natural tile of
  its kind to 200 units, three food tiles on ring radius + 3 (within 12 tiles of
  the centre). Deposits sit on rings radius + 3, + 5, + 7 or further out, never on
  a future expansion wall ring, and keep clear of the Root Hollow trail north of
  the north gate. Only when the reveal lacks a resource does a free tile on one of
  those rings receive it; ore and gem therefore often fall back outside highland.
  A pond is dug on ring radius + 3 only if no natural water lies within 8 tiles of
  the fence.

### Colony site search

`World.CreateColony` chooses its centre with a deterministic scored search. A
coarse two-tile grid of the unfounded landscape ranks every even centre within 64
tiles of the origin by trees, landmarks and highland in the reveal, nearby water
and distance from the origin. Candidates must have no water within the plateau
core, water within 16 tiles and a dry dungeon entrance. The best 64 candidates are
then generated exactly, with their datum and plateau, until one meets every
first-look guarantee:

- a river or lake 3 to 8 tiles outside the fence;
- at least 300 forest tiles with trees in the reveal after founding clears its
  square and the dungeon trail;
- at least one outcrop or ridge landmark in the reveal;
- no water within 2 tiles of the Root Hollow entrance;
- the ground beside the dungeon trail within 0.6 units of the plateau, because
  the trail is laid at plateau height;
- no unclimbable step between standable revealed tiles.

On the 150 test seeds `CreateColony` takes 25.5 ms on average and 235 ms at most.
The reveal holds 411 to 1,408 trees (median 1,047) and 4 to 250 rock landmark
tiles (median 25); the nearest water lies 3 to 7 tiles beyond the fence (median
4), and is a river for 82 of the 150 seeds. Three hundred further seeds (1000 to
1299) also all meet the guarantees.

**Demo seed.** `World.DemoSeed` is 16: a river runs 3 tiles past the east fence,
dense forest closes in from the west and south (1,128 trees), and a ridged
highland with rock outcrops rises to the north-west (374 highland tiles, 40 rock
tiles, 4.7 units of relief). Seed 4242, the shipped classic default, also passes
with a lake 7 tiles to the north-west and wide highland to the east.

## Scenarios

`WorldGenerationScenarios.cs` holds 17 cases: coherent slopes and biomes,
coordinate order and entropy, connected rivers and fords, lake shelves,
ecological deposits, version 0 examples, saved height reads, founding clearance,
lazy version 1 grading, foreign and dungeon protection, unsupported versions,
the pinned version 1 samples, and for version 2 the bank conversion, outcrops and
ridges, coordinate purity with a datum and the version 2 grade.

`FoundingScenarios.cs` holds 9 cases: the pinned classic layout, colony naming
and profile, every first-look guarantee over 150 seeds (1 to 149 and 4242, in
three parallel cases that print the best seeds), the demo seed, a completed first
expansion, blueprint variation and connectivity, and refounding after
extinction. `SpatialScenarios.cs` adds the reveal order, the founders' plaza
placement and the colony site bounds.

Four authority regressions exercise real saved files and authenticated actions.
They cover unchanged terrain and dungeon data through two saves, generation of
the same unexplored frontier after restart, and manual dungeon combat resumed
between 50-millisecond steps. The combat fixture keeps the same cat, enemy,
2.25 carried ore and one exact carried mug. Another case rejects anonymous and
foreign expedition actions before accepting the owner's dispatch and recall.
The projection case checks that an entrance rumor exposes no underground floor,
chest or creature, then reveals only discovered coordinates and remaining goods.
It also checks private rumors and founding profiles.
