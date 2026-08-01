# LAI.55–LAI.63 Plan 2 Simulation Audit

Recorded: 2026-07-25

This is a corrected Opus 5 read-only audit. The reviewer read both locked plans
and all boards from line 1, including P2.01–P2.36, GUI-R01–GUI-R26,
GUI-C01–GUI-C12, GUI-V01–GUI-V12, P2-G01–P2-G09, and every LAI.53–LAI.70
checklist, then traced the current simulation and its server/protocol
boundaries.

The reviewer made no repository edits and ran no Cargo, compiler, test, build,
lint, formatter, browser, image, or validation command. Every conclusion is a
static source finding, not acceptance evidence.

## Headline

The LAI.55–LAI.62 pure authorities are substantial, strict leaves. The sole
LAI.63 protected runtime transaction is called by the real world tick. However,
most leaf capabilities are not called by production mutation code, several
protected phases are no-ops, and the older world phases remain the shipped
authorities.

The remaining work is therefore not only cleanup or adapter cutover. For many
requirements it is the first production wiring of behavior that currently
exists only in leaf tests.

## Twelve zero- or near-zero-caller capabilities

1. Three-stage construction projects enter the canonical runtime only from
   focused source tests; no production planner command calls
   `insert_construction_project`.
2. The complete Hole feed/upgrade/recovery pipeline has no production caller:
   `begin_feed`, carried/delivered transitions, upgrade begin/delivery/
   completion, and recovery are test-only.
3. God-lane research can queue, reorder, fund, remove, and prepare, but no
   production code issues `PerformGodLabor`, so a God study never completes.
4. The free Leader lane has no production caller for `select_leader_target` or
   `CompleteLeader`. Duplicate avoidance, critical-village override, keyed
   oopsies, and overtake refund cannot occur in play.
5. Scheduled/snap elections, backing, resolution, Leader appointments,
   succession/death transitions, expulsion preview/cleanup/commit have no
   production mutator through `governance_authority`.
6. Trade posture, dispatch authorization, caravan departure, delivery,
   carrier-failure recovery, Enemy pre-dispatch rejection, and storage-binding
   verification have no live mutation loop.
7. `storage_authority.execute` is used for divine emergency cargo but does not
   own normal gathering, production, spoilage, construction, containers,
   Workshop links, or barter.
8. Teaching defer/resume/complete, mentor assignment, enterprise creation,
   surnames, and real enterprise continuity have no production issuer.
9. Productive, failed-productive, refused, unassigned, office, and supervised
   capability outcomes have no live issuer; only Hauling outcomes are emitted.
10. Ambient-cleaning XP has no world-tick call.
11. Leader food permissions exist but no consumption site consults Allowed,
    Reserve, Forbidden, or the lethal-starvation exception.
12. Exact construction/container/road/wall/village planner candidates do not
    exist, so their otherwise implemented authorities cannot be reached.

These gaps chain. Without productive XP, inherited family specialization is
nearly empty. Without Hole feed, live Void is zero, disabling Hole studies,
specialized boosts, construction miracles, and rescue. Without storage and
construction callers, stage cargo and container identity cannot become real.

## Protected runtime phase audit

The protected eleven-phase transaction is real and atomically staged. The
following phases perform meaningful work:

- authority/needs identity reconciliation;
- report/belief observation;
- Leader/officer review;
- exact-site reservation/materialization for the categories currently wired;
- workforce matching;
- the subset of visible physical tasks/construction already materialized;
- Hole credit into shared Void;
- report-twin and heartbeat projection.

The following phases remain effectively no-ops or partial:

- Unified Research only advances preparation and discards a report-safe
  projection; neither lane performs study labor/completion.
- Personal Stance/Physical Barter only validates trade state; it does not
  evaluate, authorize, depart, deliver, or recover.
- Stress/Injury merely borrows stress, anatomy, and prosthetic fields; it does
  not mutate the canonical authority.

Approximately sixty legacy `phase_*` functions still own elections,
construction, staffing, tithe/ritual, production, research, roads, and traders.
The never-compiled LAI.23 module does not remove those shipped authorities.

## Runtime cadence defect

The canonical transaction is gated by a value derived from the current game
minute rather than the authoritative simulation tick. At the shipped one-second
tick, planner review, exact task advancement, construction, Hole credit, and
research preparation run once per sixty legacy ticks.

Modulo cadence checks can also be skipped forever if an accelerated clock
jumps by a value that never lands on the modulo boundary. A long pause collapses
multiple elapsed minutes into one planner/task step unless that specific leaf
uses absolute-time catch-up.

The sole runtime must advance from a monotonic tick/cursor with bounded explicit
catch-up. Cadence should use `last_due + interval <= now`, not equality or
`now % interval == 0`.

## Skills, XP, matcher, and anatomy

Legacy `Cat::gain_skill(Labor, f64)` remains the live skill authority at many
world-tick production sites. Canonical cat capabilities receive only hauling
outcomes from emergency and exact-task bridges.

Consequently the shipped path does not award:

- primary productive XP;
- secondary productive XP;
- supervised XP;
- failed-work zero-grant receipts;
- office-duty proficiency/clearance;
- ambient cleaning;
- complete matcher/refusal behavior across legacy job assignments.

The canonical authority must receive a once-only outcome for every real task
completion/failure/refusal/unassignment before legacy skill mutation is
deleted.

## Families and late-game specialization

Birth, life-stage, death, completed-building housing, partnership review, and
housing reconciliation have partial production wiring. The exact birth order
allows parents to exist before children, so seed resolution itself is sound.

Teaching obligations, emergency defer/resume, mentorship, enterprise creation,
traditions, surnames, visible Teach tasks, and occupational continuity are not
driven. Because parent professional XP is mostly absent from the canonical
authority, child transfers and the two-generation maturity rule cannot become
meaningful no matter how long a campaign runs.

This must be wired to real completed-task receipts and exact Home/Nursery/
School/office/enterprise sites.

## Governance and direct officer control

The canonical governance authority registers residents but does not run the
election lifecycle. Legacy player ballots and vote-kick remain live. The
required top-five slate, Adult/Elder cat voting, fixed merit/interpolation,
keyed voter variation, stable tie order, and one authenticated +10 God backing
block are not the shipped election.

Legacy God actions still directly assign and vacate officers. The Leader has
no production call to `appoint_officer_from_reports`.

The canonical lifecycle must replace the old election phase and direct officer
actions, then route expulsion through every physical cleanup acknowledgement
and a reachable departure task.

## Research lanes

Production issues God queue/reorder/fund/remove and preparation commands. It
does not issue God labor completion or a Leader target/completion.

Therefore:

- funded God studies remain incomplete;
- the free Leader cadence never fires;
- a Leader cannot truthfully avoid the active God target;
- critical-village duplicate override never occurs;
- poor-Leader keyed duplicate oopsies never occur;
- the overtake refund path cannot run.

`research_purchase`, `scholar_research`, the legacy upgrade/research phase, and
old upgrade actions remain additional research authorities. They must be
deleted only after both canonical lanes are complete in production.

## Hole, food permissions, and divine chain

The content planner can create a Feed-Hole goal/task and resolve the apron, but
task completion never enters the Hole feed state machine. The live feed queue
is therefore empty; `advance_to` has nothing to credit; live Void remains zero.

The correction must bind the exact eligible lot, physical pickup, route,
delivery, feed receipt, score/Void credit, recovery, and endless next
eligibility through the single storage/task/Hole authority.

Food permissions must be consulted by the actual consumption chooser. Reserve
and Forbidden need report-safe reasons, and only lethal starvation may use the
documented exception.

Emergency Ration/Water is the one currently wired divine path, including
physical apron lots and staged hunger/thirst relief.

## Diplomacy, barter, and money

Personal stance storage has a live setter, but the trade transaction lifecycle
does not run from the protected phase. Legacy trader, route, autonomous-trade,
valuation, barter, and diplomacy modules form multiple parallel models.

Coins, SellGoods, and BuyResource remain live. The final authority must be one
physical offer/escrow/route/caravan/recovery ledger with Enemy rejection before
any reservation or side effect, honest Alliance/Neutral equivalence, and no
monetary settlement.

## Hidden-truth planner leak

The content planner input derives food and water days directly from exact
`colony.resources` values. Logs and Lair food use a fixed round-down-to-five
function, not the required effective-officer report ladder.

God and Leader report bytes are equal, but both can contain truth they should
not know. The twin check proves parity, not secrecy.

All planning stock/flow/regeneration inputs must use the ±40/25/12/5/2 percent
level bands, proper flow disclosures, staleness/confidence, and regeneration
withheld below effective level four. A hidden-truth twin must mutate ecology
truth while holding reports constant and prove planning bytes do not change.

## Legacy direct-control actions

The legacy action union/server authentication still exposes direct:

- exact-tile building planning;
- zone creation/removal;
- worker/building assignment;
- officer appointment/vacancy;
- labor preferences;
- production queue/slot edits;
- player votes/vote-kick;
- old upgrade/research purchases;
- Shrine/tithe/material/resource offerings;
- money buy/sell;
- exact road/bridge/rail/dock/vehicle/route authoring;
- farm/stockpile/gather/fishing designations;
- boost/train/defend actions.

The canonical sixteen-action union correctly omits these, but the old decoder
still authenticates and forwards them. A production retirement gate must
reject every superseded legacy gameplay action before `apply_action`, and the
client must stop presenting them.

## Compile-shaped contradictions

Static source inspection found:

- server tests call `legacy_action_requires_lai_v2` three times, but no
  definition exists;
- a server source test expects a literal retirement gate that the live handler
  does not contain;
- another source test expects `handle_leader_ai_client_text`, but the handler
  is named `handle_client_text`;
- twenty-six `#[cfg(any())]` server blocks hide apparently live action,
  persistence, research, boost, diplomacy, trade, and test code that references
  removed fields;
- the approximately 4,300-line retired sim module also references removed
  Shrine/Favor-era fields;
- `scholar_research` has no consumer and retains a Favor ledger;
- canonical construction advancement is callable but receives an empty project
  map in production.

These are compile-shaped findings only because the audit was prohibited from
running the compiler.

## Construction and storage

The three-stage construction/catalog state machines are real, but production
never inserts a canonical project. The shipped construction path remains the
legacy gather/construct progression.

The content planner has no construct, upgrade, zone, road, wall, or container
candidate. Storage is split across scalar resources, stockpiles,
`physical_storage`, `quality_lots`, and `storage_authority`.

The correction must:

- add exact planner candidates and real blueprint/project insertion;
- bind whole stage bills to exact quality-lot reservations;
- materialize scaffold/structure/fit-out/operational tasks and complete
  footprints;
- make `storage_authority` the sole writer for gathering, production,
  spoilage, construction, divine cargo, barter, loss, and salvage;
- keep any legacy resource summary derived/read-only until wire cutover;
- delete duplicate inventories after every consumer moves.

## Dependency-ordered production integration

0. Restore a coherent server retirement gate and test surface: define the
   legacy-action rejection contract in production scope and reconcile the
   handler/source assertions.
1. Fix canonical runtime cadence and bounded catch-up.
2. Emit complete canonical productive/failed/refused/office/supervised/haul
   outcomes, then retire legacy skill mutation.
3. Drive family teaching, mentoring, enterprises, traditions, surnames, and
   exact Teach tasks from real outcomes and exact sites.
4. Add construct/upgrade/zone/road/wall/container candidates, insert canonical
   projects, and retire legacy construction phases in the same cutover.
5. Make one storage authority own every lot/item/container/location/
   reservation/consume/salvage mutation.
6. Close the exact Hole feed/upgrade/recovery loop and its shared Void credit.
7. Issue God labor and free-Leader research decisions/completions, including
   duplicate avoidance, critical override, oopsie, and refunds.
8. Move election/backing/appointment/expulsion onto governance authority and
   reject direct officer/player-ballot actions.
9. Advance physical barter from the protected phase and delete coins/legacy
   trader settlement.
10. Replace exact/coarse truth reads with the officer report ladder and add
    hidden-truth invariance coverage.
11. After every replacement is live, delete never-compiled roots and all
    legacy authorities/actions rather than retaining them under
    `#[cfg(any())]`.

## Acceptance state

LAI.55–LAI.62 remain partial `dev` leaves; LAI.63 is partial `dev` integration,
not acceptance. P2-G01–P2-G09 all remain open. Earlier focused pass counts
remain useful leaf evidence but do not prove any capability runs in the game.
