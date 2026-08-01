# Black Hole / Leader AI merge contract

The Black Hole is implemented on `the-shrine-upgrade` as an authoritative
`cat-sim` domain with a temporary adapter for the current Leader. The later
`feature-new-leader-ai` merge must preserve that ownership boundary.

## Keep from this branch

- `cat_sim::black_hole` is the sole authority for Width, Depth, Darkness,
  intake ordering, accepted resource/item bands, exact reward accounting, and
  physical upgrade recipes.
- Black Hole persistence is a versioned record keyed by colony and the existing
  internal `BuildingType::Shrine` building id. It stays outside the new Leader
  runtime JSON and its strict fingerprint.
- The protocol Black Hole snapshot is an additive leaf. The stable player action
  is `NudgeBlackHole`; it is a priority hint and never selects unsafe cargo.
- Existing `BuildingType::Shrine`, map anchor, footprint, and the five historic
  research ids remain stable internal compatibility identities. Player-facing
  copy calls the building “The Hole” and the currency “Void Insight”.
- The physical landmark is always 5x5: the central 3x3 is the void and the
  sixteen outer tiles are a permanent paved road ring. Axis upgrades must not
  resize this footprint.

## Replace during the AI merge

- Remove the current-Leader Black Hole candidate adapter. The new Leader may
  choose believed feed candidates, but it must not mutate Hole state. The
  temporary adapter currently orders scalar resources only; bind exact stored
  item identities when adding crafted-item orders.
- Delete the AI branch's `shrine_offerings` authority rather than wrapping it.
  Map AI `NudgePlan` handling to the stable `NudgeBlackHole` domain command.
- Resolve hot roots (`world_tick`, server main/persistence, and the
  protocol/client/sim crate roots) structurally. Do not take either side
  wholesale: retain this branch's `ColonyRuntime::black_holes`, phase-24b
  registration, feed logistics, construction completion hook, leaf modules,
  DTO, persistence record, and UI projection, while replacing only the
  current-Leader decision function.
- Expose the replacement Leader through a narrow command boundary that starts
  or resumes one validated resource/item feed order. Candidate scoring belongs
  to the AI; stock safety, physical hauling, intake, crediting, and persistence
  remain owned by the Hole/runtime layers.
- Rename the AI branch's scholar currency to Research Notes. Void Insight
  continues to use the current spendable `global_upgrade_points` balance until
  a dedicated field migration is deliberately introduced.

## In-flight conversion

At cutover, convert at most one legacy feeding pipeline per Hole:

1. Release reservations that have not been picked up.
2. Salvage carried/in-transit cargo back to its exact visible stock.
3. Preserve delivered and already credited quantities plus the opening index.
4. Recreate only the unfulfilled remainder as new AI child haul tasks.
5. Never replay events or receipts into Favor, Void Insight, or lifetime totals.

The AI protocol branch is already version 2 and strict. Adding the Hole leaf
there requires protocol version 3 and snapshot schema version 2. The standalone
branch remains additive on the current version-1 projection.

## Hunting Lair boundary

The Hunting Lair follows the same domain/AI split and must survive the Leader
rewrite intact:

- `cat_sim::hunting_lair` owns seeded roster generation, danger, combat,
  cooldowns, loot, species-material drops, first-clear trophies, and both Hunt
  and Fight XP.
- `cat_sim::hunting_runtime::attempt_hunting_lair` is the only dispatch
  boundary. The replacement Leader may supply a revealed `EnemyLair` and an
  ordered party, but the domain revalidates site type, party cap, health,
  predicted success, equipment, and authority before resolving anything.
  Equipment means credited, unbroken `ItemStore` identities equipped on each
  named cat; aggregate weapon/armor compatibility counters grant no bonus. The
  same exact weapon and armor identities wear once when the attempt resolves.
- Replace only `tick_hunting_lair_adapter` during the AI merge. It is the
  current-Leader compatibility policy that reviews lean food stores/nudges,
  chooses the safest revealed site, and reserves the recommended idle party.
  Keep `ActiveHuntingParty`, its persisted due boundary, attempt nonce, and the
  authoritative `attempt_hunting_lair` resolution path.
- Captain advice is advisory snapshot data. Autonomous Leader requests require
  at least 70% predicted success and 70% health for every hunter. A player
  nudge lowers only the success threshold to 45% and raises the health
  threshold to 80%; it never directly starts or forces combat.
- Keep `EnemyLair` and `CaveEntrance` separate. The former is a Hunting Lair;
  the latter is the Quarry. The AI branch must not collapse both into a generic
  cave objective.
- Keep the existing `hunting_bulk` research id and catalog cardinality. Its
  player-facing meaning is now **Hunting Parties** (party cap three); it does
  not reduce danger or alter monster rosters.
- Hunting state is world-scoped and persisted separately from Leader memory:
  rosters, trophies, species-material inventories, nudges, and recent public
  outcomes. Do not place any of these in the AI fingerprint or allow an AI
  migration to regenerate them.
- A successful attempt credits Food, Hide, and Bone through finite village
  stockpiles, with a visible one-tile hunting cache for overflow. Species drops
  remain typed counters and use the protocol's `SpeciesMaterial` reward variant;
  they are not fabricated finite items with empty ids.
- Respawn state persists one absolute `respawn_ready_at_ms` deadline computed
  from the attacking colony's game-hour duration at clear time. Reconciliation
  and the public snapshot consume that same deadline; replacement AI code must
  not reinterpret it using another colony's time scale.
- The stable client command is `NudgeHuntingSite`. At merge time, consume that
  hint during the Leader's normal planning review, select a party, and call the
  domain boundary once. Clearing or retaining the hint is an AI-policy choice;
  bypassing the safety gate is not.

Black Hole species-material intake unlocks are stable domain data:
Fox Pelt at Darkness 2, Badger Pelt at 3, Bear Pelt at 5, and Beast Core at 8.
The current branch projects the typed inventories but leaves them non-orderable
until the replacement Leader binds exact haul identities.
