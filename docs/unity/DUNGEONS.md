# Layered dungeons and raids

New worlds place a two-floor Root Hollow beside each founding village. A cleared,
walkable approach joins the founding gate. A second, three-floor Deepstone Vault
can appear in the next distance band when its natural entrance is dry, unclaimed
and clear of another dungeon. Sites do not replace an existing village's terrain;
generation version zero remains unchanged.

In the demo, Root Hollow floor 1 is the ending: a raid repelled plus floor-1 loot
brought home ends the demo (D6). Floor 2 and its relic are keep-playing content on
the same save. Deepstone Vault is hidden in the demo build.

Dungeon geometry is authoritative data, independent of Unity. A seeded layout
builds a landing room, a bent corridor, a guarded chamber and a chest on each
floor. Successive layouts alternate direction so floors share XZ space.
`Int2.Level` distinguishes their occupants. Each stair connects explicit endpoints
four metres apart horizontally and four metres apart vertically. Movement spends
the same actor time as surface walking and interpolates height along the stair.
Walls and closed endpoints block routes and combat. Paths between two surface
locations do not use underground shortcuts.

The terrain under an uncarved cell is bedrock. A site's saved base elevation gives
its floors their physical heights, including entrances above sea level. The
Blender kit supplies the visible stairs, chambers and entrances; Unity presents
the selected floor as a cutaway without changing authoritative collision or
movement.

Every dungeon cell a village learns about is also a materialised tile, so the
floor and its walls render. Explorers reveal cells within four tiles on each
planning second. `MaterialiseStairEndpoints(village, cat)` creates and reveals both
ends of a stair the cat stands beside, for direct control's MoveCat.

The entrance faces its surface approach and a nine-tile-wide clearing makes the
site visible without exposing its underground rooms. In direct control, walls
on the camera side of the cat use the lower cutaway height; rear walls keep their
full height. An explicit level choice stops following a selected explorer, and
the selection outline uses the cat's physical height.

## Risk, dispatch and return

`RiskOf(cat, floor)` estimates the fight against the floor's living guardian from
time to kill versus damage taken. It returns Safe, Risky or Deadly with the
expected health loss and seconds to win:

- Cats strike for `ExplorerStrikePerSecond` times their prowess (weapon, fighting
  skill, colony combat effects and a stored relic), divided by the guardian's
  defence.
- Guardians strike for their attack (floor threat times 4.6 per second), divided
  by the cat's guard (armour, defence effects and fighting skill).
- An expected loss that would take the cat below the fight floor (15 health) is
  Deadly. A loss up to a quarter of the cat's current health is Safe. Anything
  between is Risky.

Root Hollow's threat is 0.9, so any healthy adult can try floor 1. An untrained
cat expects to lose about 35 health there (Risky). After one Train job (+3 fight)
it expects about 10 (Safe). Floor 2 is Deadly for an untrained cat and Risky after
one training; two trainings make it Safe.

`ExploreDungeon(village, cat, site, acceptDeadly)` checks everything before it
changes anything. It needs a living, uncontrolled, adult, non-migrating cat that
is not already exploring or returning a transport, health of at least 65 and
needs of at least 45. It also needs a reachable entrance and a floor that still
holds a guardian or loot. Each refusal has its own code:

- `release_control`, `kitten`, `migrating`, `already_exploring` and
  `transport_returning` for the cat's state.
- `needs_rest` (argument `need`) when health or needs are too low.
- `dungeon_cleared` (argument `seconds` until the next restock).
- `dungeon_deadly` (arguments `floor` and `loss`) unless the `ExploreDungeon`
  action sets `AcceptDeadly`.
- `blocked_route` when the entrance cannot be reached.

The trip targets the first uncleared floor. Only then does the cat give up its
current work through the normal cancellation path.

The explorer physically descends, approaches the guardian, fights, opens the chest
and returns through the stairs. Before engaging, it rereads the odds with its
current health and turns back from a Deadly fight it did not accept. A started fight
finishes unless the cat falls below the fight floor; it then flees and the guardian
lets it go. Low needs send the cat home only between fights. After a chest it
descends further only when the next floor is Safe. `RecallExplorer` starts the same
physical return without moving the cat or its inventory directly home.

A cat on the way home whose route is blocked while its health or needs are
critical gives up the trip. On the surface it spills its goods where it stands and
returns to colony life. Underground it keeps trying the stair. Either way, one
`emergency` event warns the colony.

Guardians acquire nearby cats on their own floor and chase them at least as fast
as the cat moves. They attack only from an unblocked, physically adjacent position.
Each blow varies by up to 25 percent from a seeded hash, so runs stay deterministic.
An idle guardian walks back to its post and heals there. Dormant target searches
and failed route planning wait for the next one-second planning boundary; movement
and damage use the 50-millisecond simulation cadence. Combat consumes the cat's
remaining actor time, so movement and attacking cannot both claim a full quantum.

Only a cat on an expedition whose Deadly estimate the player accepted can be killed
by a guardian. Other cats, including directly controlled ones, are left at 1 health
and cannot strike below the fight floor.

Direct control uses the same cat and cargo. `AttackCreature` selects an adjacent
enemy for that controlled cat; walking still consumes the normal movement budget.
`TakeDungeonChest(village, cat)` opens a chest on or beside the cat's tile, with the
code `chest_guarded` while its guardian lives. When the lease expires underground,
the cat starts a return. `ContinueExpeditionAfterControl(village, cat)` lets a cat
released by the player carry on from its floor instead. Surface cats and cats on
other levels cannot attack one another through a dungeon ceiling.

## Loot, restock and persistence

Root Hollow's chests pay real rewards:

- Floor 1 holds 5 research notes and a bone weapon.
- Floor 2 holds the Rootheart Charm relic with gems and stone the first time.
  Later stocks hold 8 notes and gems.
- Other sites keep stored goods.

Opening a chest moves its contents once: notes to the cat's lore, weapons and
relics into exact items the cat carries, and goods into its cargo. On arriving
home, lore becomes research points, items go to the nearest storage pile, and one
`expedition_loot` event names the floor and the totals ("Hazel brought home 5
research, a bone weapon from Root Hollow floor 1"). `Village.DeepestLootFloor`
records the deepest floor whose loot came home; the demo ending reads it. A relic
kept in colony storage multiplies the colony's combat power by 1.1 for explorers and
defenders. Relics do not stack.

A floor is cleared when its guardian is down and its chest is empty. After
`FloorRestockSeconds` (600) and once no cat stands on it, the guardian returns to
its post at full health, the chest refills and a `dungeon` event says the hollow
stirs.

Each site, floor, restock timer, creature, health value, route, chest, expedition
phase and target has saved authoritative state. Return goods enter an accepting,
reachable storage pile only upon reaching that pile. A removed or full storage
destination is rechecked. If no pile accepts the cargo, the cat waits; critical
needs instead leave a recoverable spill at the cat's actual position after it has
returned home, then release the cat to seek care. Death leaves carried goods and
items on the actual floor through the existing spill system.

Rumored entrances can be listed before a visit. Underground knowledge is added
locally as a cat explores. A client projection must hide unvisited cells, chest
contents and creatures; authoritative saves retain the complete state. Dungeon
actions pass through the same village ownership checks as ordinary actions.

## Raids

A raid is a party of `Members` raiders sharing one health pool. Raiders fall one
by one as it drops. `Strength` drives the injury the party deals: a legacy raid of
strength 30 deals exactly `RaiderStrikePerSecond`. Every cat that can fight engages,
and so does every cat rallied against the raid. A cat at or below 15 health stops
defending. An engaged party stands and fights; otherwise it walks to the shrine,
loots food and leaves.

Natural raids need only game time and colony size: after `RaidGraceSeconds`, a
colony of at least `RaidMinCats` rolls `RaidChance` once per `RaidCheckSeconds`.
The party has one raider per eight cats (two to six).

The demo profile scripts the first raid (D15). A `raid_warning` event ("Raiders
spotted to the south") comes `RaidWarningSeconds` before `DemoRaidAtSeconds` of
colony time. A fox band of three then attacks from the trail outside the south
gate. Classic worlds set `DemoRaidAtSeconds` to 0 and never run the script.

`DefendRaid` rallies defenders through `RallyDefenders`. The four nearest free
adults drop their work and walk to the raiders, or to the south gate while the
raid is only warned. It deals no direct damage and has a 60-second cooldown, with
the codes `rallied`, `no_active_raid`, `rally_cooldown` and `no_defenders`. A
Captain rallies automatically. A driven-off raid posts `raid_repelled` and counts
in `Village.RaidsRepelled`; rallied cats then stand down.

Where `RaidCanKill` is true (classic), a cat killed by raiders dies from "raid".
In the demo profile raids never kill: injuries stop at 1 health and a lost raid
costs stored food only (D16).

## Verification

The initial missing-generation regression failed before implementation in
`artifacts/tests/dungeons-red.txt`. The blocked-storage/critical-needs regression
failed before its fix in `artifacts/tests/dungeons-storage-red.txt`.

`DungeonScenarios.Cases()` covers deterministic generation, connected multilevel
rooms, physical stairs and closure recovery, the composed combat, loot and delivery
chain, recall, low needs, closed combat edges, layer isolation, direct-control
expiry, legacy-world preservation, death cargo and unavailable storage. Its
`dungeon.*` cases cover the W2-09 behaviour:

- tiles for revealed cells and stair endpoints;
- a fresh adult on floor 1 before and after one Train job, and the losses
  untrained and trained;
- fights that finish or flee, and deaths only on accepted Deadly trips;
- Deadly refusal without state change, the guarded chest and restock;
- dispatch to the first uncleared floor, both floors' loot and the relic buff;
- the stranded explorer, release from control and the guardian's walk home.

`DefenseScenarios` (`defense.*`) covers the rally, the Captain, party strength,
the renewable trigger and the raid cause of death. `RaidBeatScenarios`
(`raid_beat.*`) covers the demo beat: the warning before a band of three at the
south gate, rallied adults winning, a lost raid costing food only, no demo raid
deaths and no script in classic worlds. Rendering, projection, save/reload and
packaged Mac checks are recorded in the wider Unity acceptance ledger.
