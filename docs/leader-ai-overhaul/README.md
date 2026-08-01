# Leader Intelligence and Colony Progression Overhaul

This directory is the authoritative design and delivery record for the approved post-migration
Leader intelligence redesign. It replaces the current reactive utility director with one local,
deterministic, explainable planner that can run every founded colony while making believable
mistakes from imperfect knowledge. The additive Hole/Hunting/content integration approved after
the first cutover is specified in
[hole-hunting-content-integration.md](hole-hunting-content-integration.md); it treats every recorded
design explanation as required behavior rather than optional context. The second stored plan adds
the `bug-gui-design` systems and GUI contract without shrinking the first.

This is an explicit exception to the normal parity rule. Untouched game behavior still follows the
Rust/Bevy product contract and, where applicable, the frozen TypeScript predecessor. Behavior named
in this directory follows this specification after the atomic cutover. That implementation
baseline cutover completed on 2026-07-23; the LAI.35–LAI.70 two-plan integration is still in
progress. Older per-card “target” and “remaining work” statements are historical chronology, while
the final additive ledger in [BOARD.md](BOARD.md) records the current integrated state and gates.

## Precedence

For this overhaul, use the following order when two documents disagree:

1. The approved design recorded in this directory.
2. A completed card's implementation, tests, and evidence in [BOARD.md](BOARD.md), provided the same
   change updated the relevant design section.
3. Maintained product documents such as `docs/GAME_VISION.md` and `docs/ARCHITECTURE.md`, which
   LAI.34 synchronized with the cutover.
4. Pre-cutover Rust and historical evidence, which describe the legacy baseline rather than the
   current authority.
5. Historical material, including `docs/LEADER_AI_DESIGN.md`, `docs/TASKS.md`, and
   `docs/migration/BOARD.md`.

No implementation card may silently override this design. A discovered ambiguity or changed
constant must be resolved here and recorded on the card before that card can reach `done`.

## Lasting objectives

The planner's authority order is:

1. Active defense and immediate self-preservation.
2. Crisis survival, recovery, establishment, stabilization, sustainable growth, then prosperity.
3. Already committed urgent physical work, households, housing, storage, construction, production,
   food security, and institutional continuity.
4. Endless Hole feeding and its sustainable Apple/Fish/Hunt/Farm/Cookhouse dependencies when higher
   priorities allow.
5. Ordinary Notes research, Hole/Void research, village growth, comfort, diplomacy, and physical
   barter.

The Hole is never finished. Missing a feed has no supernatural penalty, but it gives up Void
Insight, research progress, boosts, miracles, and player power. Gods, Leaders, and officers are not
omniscient: planning, resource presentation, valuation, ecology/regeneration visibility, and
explanations use the same persisted reports and beliefs.

## Document map

| Document | Authority |
|---|---|
| [final-hole-hunting-content-plan.md](final-hole-hunting-content-plan.md) | Stored self-contained 2026-07-24 final plan for the first branch integration; later branch reconciliation must preserve its requirements explicitly |
| [final-integrated-overhaul-plan.md](final-integrated-overhaul-plan.md) | Stored self-contained 2026-07-25 second plan integrating `bug-gui-design`, colony-life systems, two research lanes, staged construction, storage, barter, and the complete GUI/visual contract |
| [bug-gui-design-BOARD.md](../branch-plan-merge/bug-gui-design-BOARD.md) | Exhaustive implementation checklist, source/conflict/visual/requirement register, and one-heavy-process policy for LAI.53–LAI.70 |
| [thread-qa-audit.md](../branch-plan-merge/thread-qa-audit.md) | Complete audit of 139 planning questions, attached user notes, direct design inputs, later supersessions, retained look-and-feel intent, and board destinations |
| [source-transfer-manifest.md](../branch-plan-merge/source-transfer-manifest.md) | Frozen commit/dirty/asset inventories and mandatory per-file semantic-transfer receipts for `the-shrine-upgrade` and `bug-gui-design` |
| [hole-hunting-content-integration.md](hole-hunting-content-integration.md) | Additive post-cutover authority for The Hole, Hunting Lairs, twenty creatures, typed food, apples, Fishing Hut/rods, cooking, unified content catalogs, rare-material crafting, art, reset, diagnostics, and future additions |
| [planner-and-beliefs.md](planner-and-beliefs.md) | Planner phases, intents, scoring, cadence, beliefs, officers, succession, RNG, and authority |
| [cats-and-care.md](cats-and-care.md) | Attributes, personality, stress, matching, injuries, treatment, and prosthetics |
| [hole-research-progression.md](hole-research-progression.md) | Hole feeds, Notes/Void, canonical graph, physical God queue, free Leader lane, preparation, boosts, miracles, persistence, UI, tests, and extension rules |
| [integrated-implementation-map.md](integrated-implementation-map.md) | Current LAI.35–70 cross-layer ownership, runtime, spatial, protocol, action, fresh persistence, server, five-screen client, diagnostics, verification, and deletion map |
| [authority-consolidation-audit.md](authority-consolidation-audit.md) | Maintained LAI.55–63 foundation-versus-live-authority audit, exact shadow-authority removals, bounded integration slices, and dependency-safe cutover order |
| [shrine-favor-research.md](shrine-favor-research.md) | Historical discarded Shrine/Favor branch proposal; never current authority |
| [diplomacy-barter.md](diplomacy-barter.md) | Personal Alliance/Neutral/Enemy, report-safe possible-now valuation, moneyless physical barter, escrow/routes/cargo/recovery, persistence, UI, tests, and extension |
| [diplomacy-trade.md](diplomacy-trade.md) | Historical LAI.0–34 Friendly/Allied/Blocked proposal; never current authority |
| [spatial-task-contract.md](spatial-task-contract.md) | Every visible task's objective, work position, delivery endpoint, footprint, route, and reservation |
| [wire-persistence-ui.md](wire-persistence-ui.md) | Historical LAI.24–31 wire/persistence/UI baseline; retained as earlier cutover evidence |
| [testing-cutover.md](testing-cutover.md) | Focused, campaign, restart, Playwright plus visible-browser/Portless, rendering, confidentiality, and atomic-cutover gates |
| [diagnostics-and-debugging.md](diagnostics-and-debugging.md) | Opt-in phase tracing, bounded campaign probes, server/action logs, browser wire evidence, and one-heavy-command debugging |
| [extending-the-system.md](extending-the-system.md) | End-to-end contributor recipes, Playwright and visible-browser QA workflows, extension invariants, current module touchpoints, and checklists |
| [content-authoring-guide.md](content-authoring-guide.md) | Historical LAI.0–34 authoring workflow; current copyable procedures are consolidated in `extending-the-system.md` |
| [browser-playtests/](browser-playtests/) | LAI.33A Playwright scenario manifest, evidence schema, and browser-checkpoint extension rules |
| [BOARD.md](BOARD.md) | Append-only Leader-AI delivery ledger through LAI.70: the completed baseline plus both stored-plan integration waves, with dependencies, status, evidence, and links to exhaustive sub-boards |

## Glossary

- **Leader**: the colony-wide strategic authority. A founding Leader can propose every essential
  domain imperfectly so a colony cannot deadlock before specialist offices exist.
- **Officer**: one of Steward, Accountant, Forester, Farmer, Captain, Loremaster, or Cloth Leader.
  Officers plan inside a domain and submit typed cross-domain requests.
- **God/player**: an authenticated human actor. The god sees the same report projection used by
  leadership, may nudge policy, buy research and boosts, and approve diplomacy, but cannot bypass
  knowledge, eligibility, reservations, refusal, or authority.
- **Observation**: physically obtained evidence about a subject at a simulation tick.
- **Belief**: a persisted estimate or categorical state derived from observations and reports,
  including confidence, bounds, provenance, and expiry.
- **Report**: the authorized projection of beliefs used by planners, protocol, and UI. It never
  contains hidden authoritative quantities merely for client-side hiding.
- **Intent**: a persistent strategic or domain commitment with evidence, dependencies, score,
  reservations, lifecycle state, rationale, and retry policy.
- **Task**: an executable, visible, multi-stage unit expanded from an approved intent.
- **Objective**: the authoritative resource, structure, plot, route, or complete footprint that a
  task concerns.
- **Work position**: the reachable tile or reserved slot where a cat performs the current stage.
- **Delivery endpoint**: the pinned structure, stockpile, Hole apron/work edge, or recipient that
  accepts output.
- **Reservation**: a world-scoped exclusive or capacity claim committed atomically with source,
  route, endpoint capacity, cargo, and worker assignment.
- **Void Insight**: exact divine currency credited only by accepted Hole intake and spent on Hole
  axis research and player-only divine boosts. It is never physical inventory or trade cargo.
- **Research Notes**: colony-owned output of completed scholar work, spent on ordinary studies.
- **Preparation**: labor-only scholar work that gives one player purchase a 25% Research Notes
  discount; it is not a currency debit.
- **Food kind**: one concrete typed bulk food with stable identity, nutrition, spoilage, value,
  source, recipes, and art; generic scalar Food is not a stored resource after the additive cutover.
- **Content catalog**: validated embedded stable-ID definitions for resources, foods, items,
  materials, creatures, recipes, augmentations, fixtures, research, and art.
- **Study**: one stable node in the validated research manifest. The interim 531/556 totals are
  historical; the catalog validator derives the current exact total from canonical content.

## Subsystem ownership and phase order

New behavior belongs in focused leaf modules. Merge-sensitive roots only orchestrate calls:

- `cat-sim` owns truth, beliefs, planner state, tasks, reservations, cats, families, governance,
  physical storage/construction, the Hole/Void ledger, Notes/two-lane research, divine effects,
  diplomacy, and barter contracts.
- `cat-protocol` owns versioned snapshots, actions, errors, and report-safe wire types.
- `cat-server` owns authentication, authorization, ordering, action dedupe, SQLite transactions,
  migration, and incompatible-client rejection.
- `cat-client` renders snapshots and sends actions; it never reconstructs hidden truth or invents
  task sites.

The integrated authoritative simulation order is:

1. Advance authoritative ecology, needs, hazards, and active emergencies.
2. Apply due observations, reports, expiry, and contradictions.
3. Advance family obligations, housing pressure, elections, succession, and due office duty.
4. Run crossed officer and Leader review boundaries chronologically.
5. Select posture; generate, deduplicate, and score goals/intents, including food permissions,
   research lanes, construction/storage/village works, Hole demand, and barter.
6. Expand dependencies; resolve authoritative complete objectives, footprints, work positions,
   delivery endpoints, and routes.
7. Atomically reserve physical source identity/quantity, sites, route segments, destination
   capacity, cargo, tools, containers, construction stages, and work slots.
8. Run the urgency-first deterministic colony-wide matcher with family enterprise, affinity,
   refusal, skill, anatomy, continuity, and route tie-breaks.
9. Advance visible task stages, construction/crops/roads/walls, movement, cargo, storage, scholar
   work, Hole feeds/upgrades, and barter caravans.
10. Resolve completion, declared XP/Mastery, teaching, injury, refusal, cancellation, preemption,
    retry, salvage, and exact physical/Notes/Void ledger effects.
11. Advance God-lane research, Leader-lane due decisions, boosts, Inspiration, contribution aid,
    miracles, cooldowns, and expiry without bypassing physical/report constraints.
12. Publish report-safe snapshots, bounded explanations, deduplicated events, and opt-in developer
    diagnostics.

LAI.46/LAI.63 have one final `world_tick` integration owner. The legacy director, Shrine/Favor
runtime, duplicate research, instant construction, aggregate storage, coin trade, and new planner
never mutate production state together.

## Existing names and target contracts

Legacy names such as `LeaderDecision`, `leader_director`, `global_upgrade_points`, Favor,
`ShrineOfferingState`, generic Food/Fish/Preserves, coin settlement, direct building upgrades, and
`research_ui.rs` are deletion targets. The integrated contracts are versioned `ColonyAiState`,
`PlannerState`, `Intent`, `BeliefStore`, `OfficerRequest`, `SiteRef`,
`WorldReservationLedger`, `VisibleTask`, `QualityLotLedger`, `BlackHoleState`,
`ProgressionResearchState`, `Family/Household/Enterprise`, `Governance/Election`, staged
construction/storage, `DivinePolicyState`, `DiplomacyState`, and `BarterContract`. They live in
focused subsystem leaves; hot roots only orchestrate them.

The canonical current footprint helper is `cat_sim::world_tick::footprint_for`. It already returns
`3 × 3` for `BuildingType::Workshop`. Spatial refactoring must leave one exported canonical
footprint authority; no second Workshop size constant is permitted.

## Cutover documentation synchronization

LAI.34 updated these maintained project surfaces in the same atomic cutover:

- `README.md`
- `docs/GAME_VISION.md`
- `docs/ARCHITECTURE.md`
- `docs/HANDOFF.md`
- `docs/IMPLEMENTATION_AUDIT.md`
- `docs/FIX_LOG.md` for defects actually corrected by the cutover
- `CLAUDE.md`
- `docs/TESTING.md`
- `docs/LEADER_AI_DESIGN.md` with a clear superseded-by-this-directory notice
- `docs/migration/BOARD.md` with at most a short link; no LAI cards are copied there

The former precedence contradiction is resolved: `docs/TESTING.md` is maintained because it
defines the enforced smoke/full-suite gates. Historical text that once called it superseded no
longer governs the current workflow.

## Locked defaults and non-goals

- Every cadence, duration, expiry, retry, and deadline uses simulation game time.
- The in-game AI is fully local and deterministic; it never calls an external model.
- Automated tests never call live AI providers.
- The AI may manage existing or founded colonies but never silently creates a player-owned village.
- Player nudges are temporary; standing orders persist.
- Hunt uses a real revealed reachable hunting source; Fetch Water uses actual water and a reachable
  dry bank; Workshop work owns the canonical complete 3 × 3/nine-tile footprint.
- Missing spatial truth blocks an intent without a worker, destination, or false world marker.
- Cross-colony exclusive reservations are world-scoped and persisted.
- Save migration is transactional and fail-closed; downgrade is unsupported.
- Release is a direct single-path cutover. There is no runtime feature flag, shadow planner,
  optional legacy mode, or dual mutation. Offline comparison fixtures are allowed only in tests.
