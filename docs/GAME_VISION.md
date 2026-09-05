# Idle Cat Forest game vision

**"An idle version of Dwarf Fortress, played by cats, in a forest."**

Idle Cat Forest is developed and distributed solely as a non-commercial game project.

## Pillars
1. **Readable 3D places.** An orthographic management camera shows the forest's
   tiles, workshops, stockpiles and cats. Blender-authored geometry also works in
   a closer third-person camera when the player controls an existing cat. The
   simulation uses a single walkable ground level; the presentation has real height.
2. **Manual → automated via roles.** Early game you direct the colony by hand (what to
   build, who hauls what, where the stockpiles go). As the colony grows you unlock and
   assign **leadership roles / officers**, each of which *automates a whole category of
   work* — turning the game idle. The current single **Leader** (the utility-AI director)
   is role #1; the evolution is **more specialized roles**:
   - **Steward** — hauling + stockpile management (what goes where).
   - **Accountant** — physical stockpile rounds and reported inventory freshness.
   - **Forester** — wood: felling, replanting, lumber.
   - **Farmer** — fields, foraging, food.
   - **Captain** — warriors, defense, raids.
   - **Loremaster/Ritualist** — research labor/building automation + shrine/rituals.
   - **Cloth Leader** — fibre, thread, hide, cloth, leather, and clothing stations.
   The always-present founding Leader retains a narrow, deficit-scaled safety floor: at the
   15-cat founding population, at most six primitive hunters, two emergency water fetchers, and
   one scout; those ceilings scale proportionally as the population changes. Specialist
   vacancies still make farming, production,
   hauling policy, research labor, rituals, and defense manual; a fresh idle village must not repeatedly
   reset merely because its first officer buildings do not exist yet.
   Each unlocked role hands its slice of decisions to the AI; unfilled roles stay manual.
3. **A living, visible workplace (DF readability).**
   - **More workshops** placed around the forest; cats walk to them to work. Houses may
     retain roofs, but workshops are open-top/cutaway stations whose function reads from
     the art itself, without persistent map-name plaques.
   - **Cats visibly haul items** between workshops and stockpiles (not just to the shrine).
   - **Stockpiles are real places in the world** — visible piles of wood / food / stone /
     refined goods that grow and shrink, that the player designates.
4. **A shared world with private footholds.** One large global village is available to
   everyone. A player may also found a personal village at a distant deterministic
   location on the same world map. Villages can eventually discover one another and trade.
5. **Knowledge must come home.** A founding village knows only its viable interior and a
   roughly two-tile halo. Resource-targeted and general scouts tentatively lift fog while
   away; discoveries become permanent only when the scout returns to and touches the shrine.
   Baseline deficit-driven scout dispatch belongs to the founding Leader (especially the fast
   first wood search) and is not gated behind the later Loremaster office; research labor/building
   automation and rituals remain Loremaster-owned, while the Leader retains the daily strategic
   study choice described below.

## Implementation context
The Unity migration implements these systems in pure C# with an authoritative
shared-world host and a native Apple Silicon client. The earlier Rust/Bevy and
TypeScript games provide behavioral and save-format evidence. Retaining their
architecture is not a design goal. See [ARCHITECTURE.md](ARCHITECTURE.md) and
[unity/ACCEPTANCE.md](unity/ACCEPTANCE.md) for current implementation and verification.

## What this vision adds (roadmap deltas)
- **Client: orthographic 3D renderer** with third-person cat control, cats + carried
  items, readable open workshops/buildings without map plaques, **visible stockpiles**,
  camera, dashboard, and manual action tools.
- **Sim: role/officer system** — split the monolithic leader director into assignable roles,
  each gating a category of automation; a "manual" default when a role is unfilled.
- **Sim: spatial stockpiles** — designate stockpile zones that physically hold items; hauling
  routes goods workshop↔stockpile↔workshop (extends the existing trip/haul + storage systems).
- **Sim: more workshops** + the crafting/hauling chains between them.

## Core loop & mechanics (the DF texture)

### More jobs than cats (the central tension)
Like DF, there is always **more work than labor**. You never have enough paws. This is
what makes prioritization — and eventually roles — matter, and what makes it a satisfying
idle-management game rather than a solved economy.

### Cats get better at what they do (skills)
Cats **improve at labors over time** (extends the existing `roleXp`/specialization). A cat
that hauls a lot becomes a faster hauler; one that mills becomes a better miller. Skilled
cats are faster / higher-yield, so *who* does *what* accumulates value — you grow experts.

### Manual → role-automation, one building at a time
Each production/management category starts **manual** (a god/player triggers it), then you
unlock a **role-officer** that automates it. Roles live in **role-buildings**, which are
**unlocked by the upgrade tree** and cost **escalating resources** to build:
- **Accountant** (build an *Accounting Tent*) — a cat walks the **stockpiles "counting stuff"**;
  only physically visited piles receive a fresh exact report (à la DF bookkeeper).
- **Cloth Leader** (build a *Clothier's Workshop*) — cats can first be told **manually** to
  produce **cloth / armour / clothing**; once the Cloth Leader is unlocked, it runs automatically.
- **Steward** (hauling + stockpiles), **Forester** (wood), **Farmer** (fields/foraging),
  **Captain** (defense/warriors), **Loremaster** (research labor/rituals) — same manual→auto pattern.
Unlocking a role hands its slice of decisions to the AI (the existing leader director,
specialized per role); unfilled roles stay manual.

### Production chains + farming (lots to do, always slightly cat)
- **Farms are in the overworld outside the walled settlement interior**: designate farm
  plots and **see the tiles being farmed** (DF farm plots). Grow **catnip**, **grain**,
  herbs, etc. Trees, stone/deposits, and fields do not occupy the village interior.
- **Processing chains**: grain → **Mill** → flour → food; fibre → **Clothier** → thread → cloth/clothing;
  hide → **Tannery** → leather; ore → **Smelter** → metal → **Smithy** → weapons/armor;
  planks + blocks → **Woodworking** → tools;
  logs → **Wood Cutter** → fine planks or **Sawmill** → structural lumber. Stone is dressed into
  blocks before construction. Each step is an open-top workshop cats walk to and haul between;
  one worker advances one selected recipe rather than several invisible parallel cycles.
- Everything is lightly cat-flavoured (catnip, mouse-farms, naps) but there's a deep to-do list.

### Visible stockpiles
Stockpiles are **real places in the world** — piles of wood/food/stone/cloth that grow and
shrink and that the player designates. The **Accountant** physically visits and counts them;
unvisited or unreachable piles remain visibly stale.

### Buildings & upgrade tree
- **Role-buildings** and later workshops are gated behind the **upgrade tree** and cost
  **escalating resources**, so expansion is the long-game economy: build → unlock a role →
  automate → free paws → build the next thing. Wood Cutter, Stone Prep, Woodworking, and
  Research Hut are explicit founding-placement exceptions that establish the first physical chains.
- Research uses a full-page dependency tree with about **500 data-driven nodes**: at least
  one third building-related unlocks and one third recipes/new resources, with the rest
  covering movement, labor, storage, defense, and similar upgrades. A player may buy any
  affordable nodes. The Leader may autonomously choose at most one node per rolling real-life day.

### Founding, housing, breeding, and migration
- A new village starts with **15 adult cats and three early Dens**; each Den provides exactly
  five permanent beds. The founding blueprint therefore starts full, not with spare breeding
  capacity.
- Breeding is deliberately slow: a pregnancy may begin only after the establishment window and
  only when a permanent bed can be reserved for the future kitten. Gestation takes 18 game-hours,
  and the reservation prevents a migrant or another pregnancy from silently overbooking it.
- A prosperous village attracts migrants after a 30-game-hour establishment window. Arrivals
  are real cats who work, consume resources, and may temporarily exceed housing capacity. An
  unhoused arrival remains on 36 game-hours of probation, then physically leaves unless a
  permanent bed becomes available first.
- Extinction recovery is deterministic and atomic: a reset clears the failed run's transient
  work and restores the complete 15-adult/three-Den founding state without ghost cats, stale
  housing reservations, or reused migrant identities.
- Water recovery remains physical even in an emergency. The director may pre-empt a worker and
  dispatch a real fetch from a known water source, but the simulation never conjures an
  emergency bucket directly into storage.
- Cat lives are paced for an idle game. Ordinary old-age mortality begins at **240 game-hours**;
  leaders and healers receive the same 20% extension and begin at **288 game-hours**. This is a
  deliberate current-design replacement for the archived web prototype's 48/57.6-hour values.

## Post-cutover completion status

The Rust/Bevy cutover and maintained P12–P19 design are complete. Skills, seven specialist roles,
spatial stockpiles and physical Accountant rounds, visible exterior farms, all ten processor types,
108 physical recipes, shrine-return scouting, authored/traffic roads, global and personal villages,
physical trade, the 487-study research graph, housing/migration pressure, and native/WASM player
paths are implemented. The focused and generalized acceptance evidence lives in
`docs/IMPLEMENTATION_AUDIT.md` and `docs/FIX_LOG.md`; the integrated correction gate is verified.

P9–P19 remain useful historical delivery groupings, not an active backlog. The integrated
correction set in `docs/FIX_LOG.md` passed its generalized passive, player-guided, persistence,
and multi-frame visual gate. The first pushed
Forgejo quality run and optional WASM transfer tuning remain external follow-ups. New design work
should be promoted here deliberately rather than inferred from old phase prose.

## Fidelity note
The Rust sim is a "same idea" port of the TS game; this vision *extends* it, so new systems
(roles, spatial stockpiles) are designed fresh in Rust, not ported. The web game on
`archive/web-game` is the frozen predecessor, not the target.
