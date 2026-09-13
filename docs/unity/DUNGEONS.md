# Layered dungeon exploration

New worlds place a two-floor Root Hollow beside each founding village. A cleared,
walkable approach joins the founding gate. A second, three-floor Deepstone Vault
can appear in the next distance band when its natural entrance is dry, unclaimed
and clear of another dungeon. Sites do not replace an existing village's terrain;
generation version zero remains unchanged.

Dungeon geometry is authoritative data, independent of Unity. A seeded layout
builds a landing room, a bent corridor, a guarded chamber and a finite supplies
chest on each floor. Successive layouts alternate direction so floors share XZ
space. `Int2.Level` distinguishes their occupants. Each stair connects explicit
endpoints four metres apart horizontally and four metres apart vertically.
Movement spends the same actor time as surface walking and interpolates height
along the stair. Walls and closed endpoints block routes and combat. Paths between
two surface locations do not use underground shortcuts.

The terrain under an uncarved cell is bedrock. A site's saved base elevation gives
its floors their physical heights, including entrances above sea level. The
Blender kit supplies the visible stairs, chambers and entrances; Unity presents
the selected floor as a cutaway without changing authoritative collision or
movement.

The entrance faces its surface approach and a nine-tile-wide clearing makes the
site visible without exposing its underground rooms. In direct control, walls
on the camera side of the cat use the lower cutaway height; rear walls keep their
full height. An explicit level choice stops following a selected explorer, and
the selection outline uses the cat's physical height.

## Dispatch, danger and return

`ExploreDungeon` takes a living adult cat from the controlled village and a known
site. It checks a real route to the entrance and evaluates health, hunger, thirst,
rest, fighting skill and the condition and quality of equipped weapons. Unready
cats refuse the expedition before relinquishing their existing work. Ready cats
cancel their prior claims through the normal work cancellation path.

The explorer physically descends, approaches the guardian, fights, reaches the
chest, carries its contents and returns through the stairs. Danger increases with
depth and distance from the founding site. The cat reassesses danger before a
fight or another descent and returns when injured or low on essential needs.
`RecallExplorer` starts the same physical return without moving the cat or its
inventory directly home.

Guardians acquire nearby cats on their own floor, chase through the same walkable
cells and attack only at an unblocked, physically adjacent position. Dormant target
searches and failed route planning wait for the next one-second planning boundary;
movement and damage use the 50-millisecond simulation cadence. Combat consumes the
cat's remaining actor time, so movement and attacking cannot both claim a full
quantum.

Direct control uses the same cat and cargo. `AttackCreature` selects an adjacent
enemy for that controlled cat; walking still consumes the normal movement budget.
Releasing control underground starts a return. Surface cats and cats on other
levels cannot attack one another through a dungeon ceiling.

## Goods and persistence

Each site, floor, creature, health value, route, chest, expedition phase and target
has saved authoritative state. Defeated creatures stay dead. A cleared chest stays
empty. Visiting or loading the site does not create another copy of either.

Chest goods transfer to one cat only upon reaching the chest. Return goods enter
an accepting, reachable storage pile only upon reaching that pile. A removed or
full storage destination is rechecked. If no pile accepts the cargo, the cat waits;
critical needs instead leave a recoverable spill at the cat's actual position
after it has returned home, then release the cat to seek care. Death leaves carried
goods on the actual dungeon floor through the existing spill system.

Rumored entrances can be listed before a visit. Underground knowledge is added
locally as a cat explores. A client projection must hide unvisited cells, chest
contents and creatures; authoritative saves retain the complete state. Dungeon
actions pass through the same village ownership checks as ordinary actions.

## Verification

The initial missing-generation regression failed before implementation in
`artifacts/tests/dungeons-red.txt`. The blocked-storage/critical-needs regression
failed before its fix in `artifacts/tests/dungeons-storage-red.txt`.

`DungeonScenarios.Cases()` covers deterministic generation, connected multilevel
rooms, physical stairs and closure recovery, readiness refusal, the composed
combat–loot–delivery chain, recall, low needs, closed combat edges, layer isolation,
direct-control expiry, legacy-world preservation, death cargo and unavailable
storage. Rendering, projection, save/reload and packaged Mac checks are recorded
in the wider Unity acceptance ledger.
