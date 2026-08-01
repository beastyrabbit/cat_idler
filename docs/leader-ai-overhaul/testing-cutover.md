# Testing and atomic cutover

Every card begins with focused failing coverage, turns it green through causal production changes,
updates its authoritative design section, and records evidence on [BOARD.md](BOARD.md). Tests are
fully local/deterministic and never call live AI providers.

Contributors extending an implemented subsystem must also follow the complete transaction and the
domain-specific recipe in [extending-the-system.md](extending-the-system.md). Its stable-ID,
deterministic ordering/RNG, authority/redaction, complete spatial contract, cross-colony
reservation, persistence/versioning, rollback, and documentation checklists are release evidence,
not optional style guidance.

## Card quality gate

Before a card reaches `done`, record:

1. Red command and the expected failing assertion/fixture.
2. Green focused command and result.
3. Relevant touched-crate or scenario tests.
4. `cargo nextest run --workspace --profile smoke`.
5. Touched-crate `cargo clippy ... -- -D warnings`.
6. `cargo fmt` and `git diff --check`.
7. Updated design link and migration/default notes.
8. Determinism, confidentiality, persistence, and UI evidence relevant to that card.

The complete workspace inventory is not routinely run locally. Before push/cutover, Forgejo must
pass all four parallel Nextest hash partitions as defined by `docs/TESTING.md`.

## Planner and belief coverage

- Identical seed, persisted state, actions, and tick sequence produce byte-equivalent outcomes.
- Equivalent large/small tick partitions process planning boundaries chronologically.
- Same beliefs with different hidden world truth yield the same plan until authorized evidence.
- No regeneration appears in planner, protocol, logs, or UI below effective report level 4.
- Level error bands, expiry at 1/6/12/24 hours, 500-basis-point decay per full expiry interval,
  zero-confidence direct invalidation, precedence, and contradiction supersession are exact.
- Bounded execution errors never reveal quantity, regeneration, undiscovered type, or unseen threat.
- Omission uses its dedicated stream and exact 25/12/5/1/0% rates; emergencies are never omitted.
- Fixed-point score, 15% hysteresis, 1-point/hour starvation aging with 25-point cap, and stable
  tie-breaks are independent of collection order.
- Retry sequence is 15m, 30m, 1h, 2h, 4h; the fifth failure terminates unless evidence materially
  changes. Permanent invalidity fails immediately.
- Dependency cycles fail and equivalent requests/intents deduplicate.
- Live/history queues remain within 128/256 through long campaigns and evict deterministically.
- Leader/officer death does not reset quota, duplicate intents, or leak reservations.
- Election completes within six game-hours where an eligible cat exists.
- Appointment candidate inspection is 3/5/8/12/all by level and uses the isolated stable-key RNG.
- Officer requests age +1 point/full hour capped +25 and expire at 6h, 48h, or 7d by domain.
- Temporary nudges expire with epoch, cannot stack/bypass eligibility, and persistent standing
  orders survive restart.

## Spatial focused tests

Required named/behavioral coverage includes:

- `hunt_without_revealed_reachable_source_is_blocked_without_objective`
- `hunt_never_uses_radial_fallback`
- `unreachable_source_never_uses_straight_line`
- Hunt resolves a real revealed reachable cave/hunting-source identity and pinned destination.
- Fetch Water keeps actual water, reachable dry bank, and endpoint distinct; no bank means no cat.
- Fish keys capacity to canonical habitat identity and uses a separate bank.
- Quarry uses an exact revealed reachable source.
- Fibre Forage uses an exact revealed source and pinned textile/storage destination.
- Logging reserves and displays the complete 2 × 3/six-tile tree footprint.
- Two colonies cannot exclusively reserve the same tree/stump/unique world slot.
- Replant retains exact stump identity; Scout retains route and report delivery to the village.
- Construction uses the complete canonical building footprint.
- Road construction preserves the full ordered route.
- Station/training/accounting/personal-need work uses exact slots/sites and endpoints.
- Workshop objective is width 3, height 3, and exactly nine row-major tiles.
- Multiple Workshop workers share the objective but reserve distinct slots.
- Stage transitions and restart preserve objective, cargo, route, and pinned endpoint.
- Route closure blocks/releases/rematches; picked-up cargo is delivered or physically salvaged.
- No-site tasks never mark cats busy or install a destination.
- Malformed site metadata fails closed as an explicitly blocked task.
- Every `SiteRef` variant round-trips through protocol.
- Client renders nine Workshop and six tree cells, deduplicates coincident markers, despawns stale
  markers, and creates zero entities for unrevealed/objective-less blocked tasks.

## Cats, matching, and care

- Attribute conversion and parental midpoint/mutation clamp exactly to 1–20.
- Personality distribution matches 80/15/5 over a deterministic seed matrix.
- Every personality axis affects only its documented weights at 5/15/30%.
- Stress boundaries, exact additions/recovery, refusal, and Burned Out recovery are exact.
- Refusal preserves carried cargo and completes already-consumed atomic recipe work once.
- `Blocked(NoWillingWorker)` is visible and never softlocks a cat.
- Maximum-weight matching beats known greedy counterexamples and is stable across input order.
- 15% preemption and emergency/incapacity exceptions are exact.
- Injury rates and 70/20/8/2 outcomes follow deterministic fixtures and batching partitions.
- Paw/eye/tail aggregation and severe-work exclusions are exact.
- Minor/severe treatment requires 12/48 effective work-hours; missing parts never regrow; existing
  skill productivity plus Caregiver/Restorative Grace supplies the only healing-rate pipeline.
- Fitting requires part/side, item, consent, fitter, and reachable site.
- Wooden/metal restoration (50/75%), 360/1,080 affected-hour durability, Rehabilitation,
  adaptation, and 90% cap are exact.
- One prosthetic item ID is conserved through fitting, refusal, breakage, repair, death, trade,
  cancellation, and restart.

## LAI.30 cat-care focused and browser tests

`LAI.30_CAT_CARE_UI_CONTRACT` is a red-first client contract. Before LAI.24 and LAI.27 production
integration exists, `cargo test -p cat-client --test lai30_cat_care_ui_contract --no-fail-fast` and
`cargo nextest run -p cat-client --test lai30_cat_care_ui_contract --no-fail-fast` must compile and
fail on missing production UI symbols, not on local shims. The focused red names are:

- `care_panel_renders_stable_report_safe_cat_identity_and_capability_breakdown`
- `stress_recovery_refusal_and_willingness_reasons_are_bounded`
- `anatomy_injury_and_treatment_state_cover_every_body_part`
- `prosthetic_state_reports_side_type_restoration_durability_and_wear`
- `active_care_tasks_sites_cargo_and_conservation_are_visible_without_leaks`
- `care_controls_send_authenticated_expected_version_idempotent_actions`
- `disabled_states_typed_feedback_and_stale_refresh_preserve_selected_cat`
- `playwright_visible_browser_ids_and_hidden_truth_guards_are_defined`

The green implementation must prove report-safe per-cat panels with stable canonical identity and
innate attribute breakdown; learned skills and office experience; personality axes; acquired traits;
stress, recovery, refusal, willingness reasons, and self-preservation override status; complete
four-paw/two-eye/tail anatomy; injury and treatment state; fitted prosthetic side, type,
restoration, durability, wear, adaptation, and cap; active care task, site, cargo, patient, fitter,
medic, and Workshop repair references; and bounded eligibility or block reasons.

Treatment, consent/refusal acknowledgement, prosthetic fit, remove, and repair controls must send
authenticated player-only action envelopes with expected cat-care version and stable idempotency ID.
Disabled states and typed feedback must be bounded and existence-safe. Stale refresh preserves the
selected cat and any safe draft context; removed cats clear selection without reusing stale controls;
duplicate replay returns the original result.

The UI may not recompute capability, willingness, treatment speed, item ownership, or regeneration
from hidden truth. It may display only authorized snapshot fields. Assertions search labels,
tooltips, logs, inspector text, and accessible names for hidden regeneration, hidden stock/source
quantities, private beliefs/plans, auth material, synthesized prosthetic IDs, another colony's
private cats, and unbounded treatment errors.

The implementation must prove item/cargo identity conservation through fitting, refusal, removal,
repair, cancellation, death recovery, trade, restart, and duplicate replay. Multi-colony fixtures
must show that a selected colony's browser cannot infer another colony's cat existence, anatomy,
injury, active care task, fitted item, cargo, private block reason, or treatment state through panel
contents, disabled-state wording, feedback, tooltips, errors, or timing-relevant client work.

Playwright and visible-browser acceptance use `PLAYWRIGHT_CAT_CARE_LOCATOR_MANIFEST` with stable
roles, labels, and IDs: `ACCESSIBLE_CAT_CARE_PANEL_LABEL`, `CAT_CARE_PANEL_TEST_ID_PREFIX`,
`CAT_CARE_BODY_PART_TEST_ID_PREFIX`, `CAT_CARE_CONTROL_TEST_ID_PREFIX`, and
`CAT_CARE_TASK_REF_TEST_ID_PREFIX`. Browser checkpoints are
`VISIBLE_BROWSER_CHECKPOINT_LAI30_CAT_PANEL`,
`VISIBLE_BROWSER_CHECKPOINT_LAI30_TREATMENT_PROSTHETIC`, and
`VISIBLE_BROWSER_CHECKPOINT_LAI30_STALE_REFRESH_PRIVACY`, each storing selected colony, cat ID,
expected version, idempotency ID, task/site/cargo/prosthetic item IDs, accessibility tree,
screenshot, console/network state, viewport, zoom where relevant, and stale-refresh outcome.

## Hole, Research Notes, Void Insight, research, and boosts

- The Hole remains an endless physical demand. It has no completion, cooldown, tithe scalar, missed-
  offering curse, or hidden resource faucet.
- The Leader selects a real reported lot by replacement cost, village need, personality, skill, and
  report quality. A weak Leader may choose scarce food or omit a review; survival, defense, and
  village staffing still preempt optional Hole work.
- Every feed reserves a real identity, travels from its exact storage site to the pinned Hole edge,
  and credits Void Insight only after deposit/consumption. Retry, restart, refusal, death recovery,
  cancellation, and replay may neither duplicate credit nor delete cargo.
- Exact regeneration remains server-only through effective officer report level 3. Level 4+ exposes
  only the typed bounded estimate carried by the canonical report. Gods receive the same report gate.
- Hole feeds create Void Insight. Completed ordinary scholar work creates Research Notes. These
  ledgers never mirror inventory, cargo, escrow, generic research points, Favor, or Blessings.
- The manifest has exactly 531 unique reachable live studies in deterministic acyclic order.
- The free Leader lane commits finite-first studies immediately at exact rolling-seven-day quotas
  1/2/2/3/4. Unused quota does not carry, rejected choices consume no quota, and a normal affordable
  30-day campaign completes at least four automatic commits.
- The physical God lane has a bounded topological queue, frozen front cost, real scholar work, Notes
  or Void funding, reassignment/labor-loss rules, and one non-expiring preparation that grants the
  player-only 25% discount. A Leader normally avoids a study already queued by the God unless the
  village has a critical duplicate need or a permitted bounded mistake occurs.
- The four separately researched Divine Boosts spend Void Insight, use their committed duration and
  effect stages, reject same-type overlap without debit, allow different types to overlap, and are
  never available to the Leader or officers. Inspiration is a distinct additive God action.

## LAI.67 Research and Council focused/browser tests

The old LAI.31 Shrine/Favor progression contract is retired. Canonical coverage is
`lai67_research_council_ui` over protocol v3/schema v2. It must prove:

- Research catalog, fixed graph, two queues/lanes, inspector, prerequisites, frozen costs,
  rolling Leader quota, Notes/Void balances, physical preparation, refund/labor-loss state, and
  separate Inspiration/Divine Boost controls.
- Council Plans/Tasks/Cats/Hole/Diplomacy/Trade tabs with officer requests, effective expertise,
  standing-order capability, exact task geometry, cat/family/election/officer detail, report-safe
  rationale, Hole food permissions, personal stance, barter valuation, escrow, routes, recovery,
  and typed unavailable/conflict/stale states.
- No client-side prerequisite, regeneration, worker-eligibility, family, election, trade-consent,
  route, stock, or authority inference. Missing canonical fields display “unavailable.”
- Every remote control emits only an allowed `CanonicalGodAction`; the authenticated transport adds
  the selected colony, player identity, idempotency ID, and exact ordered version lanes.
- Multi-colony privacy checks search visible text, tooltips, accessibility trees, screenshots,
  inspector text, feedback, console, and network logs for foreign private plans/cats/stock, exact
  regeneration below level 4, unseen sites, auth material, or rejected hidden amounts.
- Stable semantic IDs, keyboard focus, AccessKit actions, responsive scrolling, safe refresh
  selection, and despawn of removed rows at all target viewports.

Playwright and visible-browser evidence records the selected colony, canonical row IDs, expected and
committed versions, idempotency IDs, action receipt/conflict, accessibility tree, screenshot,
console/network state, viewport, simulation tick, restart identifiers, and privacy-search result.
The browser may inspect real DOM/accessibility state and screenshots but may not inject snapshots,
private endpoints, hidden hooks, or direct simulation mutations.

## LAI.29 task-marker focused and browser tests

`LAI.29_WORLD_TASK_FOOTPRINT_UI_CONTRACT` is a red-first client contract. Before LAI.24 and LAI.27
production integration exists, `cargo test -p cat-client --test lai29_world_task_footprint_contract
--no-fail-fast` and `cargo nextest run -p cat-client --test lai29_world_task_footprint_contract
--no-fail-fast` must compile and fail on missing production UI symbols, not on test shims. The
focused red names are:

- `visible_task_markers_are_snapshot_only_and_strict_siterefs`
- `hunt_and_fetch_water_render_actual_objective_work_and_endpoint_sites`
- `workshop_and_tree_footprints_render_all_canonical_cells`
- `snapshot_id_keyed_dedupe_update_and_despawn_are_authoritative`
- `redacted_blocked_missing_or_foreign_tasks_emit_no_markers`
- `route_endpoint_and_work_marker_accessibility_ids_are_stable`
- `zoom_viewport_and_visible_browser_checkpoints_are_defined`
- `tooltips_are_report_safe_and_fallbacks_are_absent`

The green implementation must prove markers/entities are derived only from authoritative
`VisibleTaskSnapshot` fields and strict `SiteRef` variants. Hunt objective markers must be the actual
revealed reachable cave or hunting-source identity. Fetch Water must render the actual water source,
separate reachable dry bank/work tile, and pinned delivery endpoint as three independently
addressable markers. Workshop must render exactly nine `CanonicalFootprintCellIndex` cells in
row-major order plus distinct work-slot and delivery markers. Tree tasks render all six canonical
cells where applicable.

The focused test suite must also prove snapshot-ID keyed dedupe, update, and despawn on removal; no
radial, generic, nearest, or cat-destination fallback; no stale or duplicate markers; no marker for
redacted, blocked, missing-site, or foreign-colony tasks; multi-colony isolation; route/contact
markers distinct from pinned delivery endpoints; supported zoom and viewport culling; and report-safe
tooltips. Tooltip and accessible text assertions search for hidden stock, exact regeneration below
effective report level four, private beliefs/plans, auth material, and another colony's private
state; any match fails.

The production UI must publish a `PLAYWRIGHT_TASK_MARKER_LOCATOR_MANIFEST` with stable IDs:
`TASK_MARKER_OBJECTIVE_TEST_ID`, `TASK_MARKER_WORK_SLOT_TEST_ID`,
`TASK_MARKER_ENDPOINT_TEST_ID`, and `TASK_MARKER_CELL_TEST_ID`. Their rendered forms are
`task-marker:{task_id}:objective:{site_id}`, `task-marker:{task_id}:work:{slot_id}`,
`task-marker:{task_id}:endpoint:{site_id}`, and `task-marker:{task_id}:cell:{index}:{site_id}`.
Labels use `ACCESSIBLE_TASK_OBJECTIVE_LABEL`, `ACCESSIBLE_TASK_WORK_SLOT_LABEL`, and
`ACCESSIBLE_TASK_ENDPOINT_LABEL`, and identify only task category, marker role, report-safe site
kind, and bounded status.

Visible-browser evidence checkpoints are
`VISIBLE_BROWSER_CHECKPOINT_LAI29_WORKSHOP_FOOTPRINT`,
`VISIBLE_BROWSER_CHECKPOINT_LAI29_HUNT_WATER`,
`VISIBLE_BROWSER_CHECKPOINT_LAI29_DESPAWN_DEDUPE`, and
`VISIBLE_BROWSER_CHECKPOINT_LAI29_REDACTION`. Each checkpoint records the Playwright locator,
accessibility tree, screenshot before and after update/removal/zoom, console/network state, selected
colony, task ID, site ID, marker IDs, viewport, zoom, and simulation tick.

## Diplomacy, trade, server, and protocol

- Friendly/Allied require mutual consent and blocking is immediate.
- Neutral/Blocked villages do not initiate autonomous trade.
- Friendly valuation stays within ±10%; Allied support within 20%; all inputs are reports.
- Escrow prevents double spending and reserves destination headroom.
- Route/pickup/delivery are physical; route failure never duplicates or deletes cargo.
- Stable next-event-tick/contract-ID ordering removes colony-order advantage.
- Restart preserves in-transit contracts and stranded cargo exactly.
- Every action enforces protocol, authentication, ownership, authority, expected version, and
  idempotency in the documented order.
- Duplicate actions return the original result; stale versions refresh rather than double mutate.
- Redaction tests enumerate every snapshot, error, log, tooltip, and inspector field.
- Old clients receive `UPDATE_REQUIRED` and cannot mutate.
- Fresh-schema loading rejects or quarantines one malformed/unknown/future required row atomically.
  No semantic legacy conversion is permitted.

## Deterministic 30-day campaigns

Run at least 100 seeds for each required population:

- fresh colony;
- established colony;
- mature research/trade colony;
- fresh-schema restart at active canonical stages;
- highly Devout, Skeptical, Mercantile, Self-sufficient, Bold, and Cautious extremes;
- Officer vacancy and Leader succession;
- multi-colony resource contention.

Fresh success target: at least 85/100 finish 30 game-days with living population, operational
food/water, no impossible busy cats, no negative inventories, and a progressing or explicitly
blocked Hole/growth plan.

Established success target: at least 97/100 finish with at least half the starting population,
functional essential chains, an operational Hole route, bounded plan/request/event state, no deadlock, and
no invalid numeric state.

Successful eligible/affordable campaigns repeatedly feed the Hole, conserve Void Insight, grow or
record an evidence-backed capacity block, fill core offices when cats/buildings permit it, and
respect the automatic Leader research quota and physical God lane. No campaign may produce `NaN`,
overflow-order behavior, duplicate
ledger events, leaked truth, orphaned reservation, infinite retry, cargo loss, or impossible busy
state.

For supported paths, compare outcomes across 1-second, 1-minute, 15-minute, and 1-hour advances.

## LAI.32 campaign manifest and red harness

`docs/leader-ai-overhaul/fixtures/lai32_campaign_manifest.json` is the deterministic campaign
matrix for LAI.32. It records 17 required 100-seed sets: fresh, established, mature research/trade,
extreme scarcity, Devout, Skeptical, Mercantile, Self-sufficient, Bold, Cautious, injury/prosthetic/
stress, multi-colony, reservation/contention, Hole omission and bad-resource choices, research
quota, diplomacy/trade, and restart/partition. Each set uses the deterministic formula
`seed = seedStart + index` for `0 <= index < 100`.

The focused red harness is `crates/cat-sim/tests/lai32_campaign_manifest.rs`. Its normal smoke
target is:

```bash
cargo test -p cat-sim --test lai32_campaign_manifest --no-fail-fast
cargo nextest run -p cat-sim --test lai32_campaign_manifest --no-fail-fast
```

The smoke target validates the manifest schema, seed counts, uniqueness, thresholds, invariant
inventory, and release-profile budget, then intentionally fails on missing LAI.32 campaign-runner
evidence hooks until the post-LAI.23 production path exists. The required red smoke names are:

- `small_smoke_campaign_entrypoint_is_red_until_runner_exists`
- `campaign_success_thresholds_are_asserted_by_runner_not_docs_only`
- `campaign_progression_spatial_privacy_and_replay_invariants_are_runner_outputs`
- `restart_partition_and_release_profile_evidence_hooks_are_present`

The harness also defines ignored release entrypoints:

- `ignored_release_profile_full_campaign_matrix_meets_lai1_budget`
- `ignored_restart_partition_matrix_is_byte_equal`

Green LAI.32 must run the full 30-day matrix from the manifest and assert at least 85/100 fresh
successes and 97/100 established successes. It must prove bounded state/queues, no starvation caused
solely by endless Hole demand, believable good/bad Leader variation, at least four normal automatic
Leader research commits in 30 days, exact Hole-credit/Void Insight conservation, Hunt/Water/Workshop
spatial invariants, hidden regeneration secrecy below effective report level 4, no duplicate
mutations or replay, tick-partition twins, and restart twins. The ignored release-profile command
uses the LAI.1 fixture as its baseline and fails if median wall time or median peak RSS regresses by
more than 25% on the identical release profile.

## Persistence, UI, and performance evidence

Signed system journeys cover fresh startup and fresh-schema restart, restart at every task/Hole-feed/trade/fitting/
boost stage, action replay, concurrent stale actions, multi-colony reservations, authorization,
redaction, and incompatible clients. Save/reload equality includes all new state and reconstructed
reservations.

### LAI.33A-SYS signed system journey red contract

`LAI.33A_SYS_SIGNED_SYSTEM_JOURNEY_CONTRACT` defines deterministic signed server journeys that must
run after LAI.24-LAI.27 production and LAI.32 campaign fixtures exist. This red contract names the
required journey surfaces and evidence; it does not execute services or weaken assertions while the
shared compile is still pre-cutover.

`LAI33_SYS_SEED_FRESH_STARTUP_0x5333A001` creates a fresh colony with one planned visible task, one
Hole feed opportunity, one Notes/Void research/boost path, one treatable cat, one diplomacy partner,
and a second colony with private inventory and reports. `LAI33_SYS_SEED_RESTART_0x5333A002` loads a
fresh-schema SQLite fixture saved at active canonical stages; it is not a legacy migration fixture
and performs no currency conversion. Malformed/future-row side fixtures must fail closed. All IDs
below are stable fixture IDs recorded before startup: `world-lai33-sys`, `colony-fresh-a`,
`colony-restart-a`, `colony-foreign-b`, `cat-care-001`, `task-visible-001`, `hole-feed-001`,
`research-study-001`, `boost-001`, `prosthetic-item-001`, `trade-contract-001`, and
`reservation-world-001`.

`LAI33_SYS_STAGE_TABLE`

| Stage ID | Journey | Seed | Action Order | Restart Tick | Expected IDs/Versions | SQLite Checkpoint |
|---|---|---|---:|---:|---|---|
| S00 | fresh startup and first signed snapshot | `0x5333A001` | 0 | 0 | world, colony, protocol, persistence, snapshot version | `fresh-before-start`, `fresh-after-snapshot` |
| S01 | fresh-schema restart and strict row validation | `0x5333A002` | 1 | 0 | aggregate schema, canonical checksum, cats/sites, malformed-row rejection | `restart-before-start`, `restart-after-snapshot` |
| S02 | visible task resolve/reserve/travel/work/deposit | `0x5333A001` | 2-6 | 15, 60, 300, 900 | task, intent, site, route, reservation, cargo, worker versions | `task-stage-*` |
| S03 | Hole source, haul, pinned-edge deposit, Void credit | `0x5333A001` | 7-10 | 120, 600, 1200 | feed, source, cargo, Hole, Void event/balance | `hole-stage-*` |
| S04 | Leader commit and physical God-lane preparation | `0x5333A001` | 11-12 | 1500, 1800 | study, quota window, Notes/Void, preparation, expected versions | `research-stage-*` |
| S05 | prosthetic fitting and Workshop repair | `0x5333A002` | 13-16 | 2100, 2400 | cat, part/side, item, fitting task, repair cargo | `prosthetic-stage-*` |
| S06 | player-only boost activation and expiry state | `0x5333A001` | 17 | 2700 | boost, committed stages, Void debit, expiry tick | `boost-stage-*` |
| S07 | diplomacy consent and trade escrow/pickup/delivery | `0x5333A001` | 18-23 | 3000, 3600, 4200 | pair, proposal, contract, escrow, cargo, route, hauler | `trade-stage-*` |
| S08 | trade route failure and recovery/salvage | `0x5333A002` | 24-26 | 4800, 5400 | recovery task, stranded cargo, bounded failure | `trade-recovery-*` |
| S09 | authenticated idempotency replay | both | 27-30 | 5700 | original action/result IDs, no duplicate effects | `replay-before`, `replay-after` |
| S10 | concurrent stale expected versions | both | 31-34 | 6000 | stale/current versions, refresh snapshot, no mutation | `stale-before`, `stale-after` |
| S11 | incompatible old-client `UPDATE_REQUIRED` | both | 35 | 6000 | minimum/current protocol versions, no auth/decode side effect | `old-client-before`, `old-client-after` |
| S12 | multi-colony reservation/site/trade isolation | both | 36-40 | 6300 | selected colony, foreign colony, reservation/site/trade IDs | `isolation-before`, `isolation-after` |
| S13 | server-side redaction and malformed-row quarantine | `0x5333A002` | 41-43 | 6600 | report level, hidden sentinel hash, quarantine ID | `redaction-before`, `quarantine-after` |
| S14 | exact aggregate save/reload equality | both | 44 | 7200 | runtime fingerprint, protocol fingerprint, SQLite checksum | `final-before`, `final-after` |

`LAI33_SYS_COMMANDS`

```bash
cargo test -p cat-server --test lai33_signed_system_journey_contract --no-fail-fast
cargo nextest run -p cat-server --test lai33_signed_system_journey_contract --no-fail-fast
cargo test -p cat-server --test lai33_signed_system_journey_contract -- --ignored --nocapture
```

The ignored full entrypoints are `lai33_full_signed_restart_journey_entrypoint` and
`lai33_full_multi_colony_journey_entrypoint`. They are not release evidence until production can
start the server against the deterministic SQLite fixtures and execute signed actions through the
documented server pipeline.

`LAI33_SYS_SQLITE_CHECKSUM_CHECKPOINTS` require SHA-256 before startup, after first snapshot, before
and after every restart stage, after replay/stale/old-client/isolation/redaction checks, and at final
shutdown. Each checkpoint records commit, dirty diff hash, seed, fixture path, server command,
protocol version, persistence version, action order number, expected tick, snapshot/resource/
reservation/Hole/Void/Notes/research/boost/diplomacy/trade versions, and the aggregate runtime/protocol
fingerprint.

The journeys must prove authenticated idempotency replay returns identical prior results; stale
expected versions refresh without partial mutation; incompatible clients receive `UPDATE_REQUIRED`
before authentication or action decode; multi-colony reservations, site IDs, and trade contracts are
isolated; server-side redaction suppresses regeneration below level 4, hidden inventory, and private
plans in snapshots/errors/logs; malformed rows roll back or quarantine atomically; and exact
save/reload equality covers aggregate runtime and protocol state. Manufactured inventory or currency,
undocumented time skips, auth bypass, private endpoint calls, client-only redaction, and partial
restart reconstruction are forbidden.

Client acceptance uses authoritative snapshots and inspected own-framebuffers at supported native
sizes plus WASM checks where affected. It proves top-eight Plans controls, uncertainty, cat care,
Hole/Notes/Void/research/boosts, diplomacy/trade, and exact spatial footprints without stale/leaked
markers. These automated/native checks do not replace either browser gate below: the complete
Playwright play-test journey and the independently observed visible-browser journey are both
required.

### LAI.28 Plans UI focused/browser contract

`LAI.28_PLANS_UI_CONTRACT` defines the focused red target before the production Plans UI exists.
The harness may inspect client source and documentation for stable future symbols, but it must not
inject DOM state, fake protocol DTOs, modify production UI, or bypass LAI.24/25/27 ownership.

The eventual green run must prove the Plans surface through shipped controls only:

- top-eight authoritative rows with stable IDs, lifecycle/status, responsible Leader/officer,
  dependencies, bounded rationale/reasons, score, confidence, ranges, age, and provenance;
- no hidden truth in visible text, tooltips, accessibility trees, screenshots, logs, or conflict
  feedback, including no regeneration below effective report level 4;
- accessible `Move Up` (+0.15), `Move Down` (-0.15), dismiss, standing-order create/edit/remove, and
  domain nudge controls with exact enabled/disabled state and typed bounded feedback;
- Administration slot limit, used count, vacancy, and full/unauthorized/stale/precondition feedback;
- LAI.25 authenticated action envelopes with protocol version, idempotency ID, colony/player
  identity, expected planner/domain/resource/reservation versions, and strict bounded payloads;
- stale-action refresh that preserves panel focus and standing-order draft while despawning
  unknown or removed plans without stale controls;
- deterministic display and action behavior for equal nudges, opposite nudges, and tied plan scores;
  and
- officer report, vacancy, authority, request reason, expiry, and bounded block display.

Playwright coverage must use stable accessibility roles, labels, and `data-testid` values owned by
the production UI. It may read DOM/accessibility state and screenshots, but it must not mutate state
through JavaScript, private action endpoints, synthetic snapshots, or hidden test hooks. The
independent visible-browser checkpoint must capture the Plans panel before and after one accepted
nudge, one stale/replayed action, one standing-order edit, one officer/vacancy view, and one
regeneration-below-L4 secrecy search.

LAI.1 records a committed deterministic release-profile runtime/memory baseline for the fixed
30-day fixture before production work. On identical hardware and profile, LAI.32/LAI.34 permit no
more than 25% regression in either median wall time or peak RSS against that baseline. Functional
determinism and bounded-state tests remain hard gates even inside the performance budget. Planner
queues are bounded and stable live ticks must not rebuild expensive spatial indexes or rerun route
planning without an invalidating event.

## Real-browser acceptance (LAI.33A)

LAI.33A runs after LAI.28–LAI.33 and before LAI.34. Its execution owner is orchestration task
`task_99e5e9fd0657`. It has two cumulative release gates:

1. a Playwright-driven browser play-test run covering all required journeys through user-visible
   controls; and
2. an independently operated visible desktop-browser run with accessibility, pixel, and DevTools
   evidence.

Neither gate substitutes for the other. Every result must be reproduced from the same recorded
commit, seed, SQLite fixture, named URL, and action sequence.

### Portless Rust/Trunk setup

Use the exact serving workflow in
[extending-the-system.md](extending-the-system.md#serve-rust-and-trunk-through-portless):

- start `cargo run -p cat-server` with stable Portless base name `leader-ai-api`; the Rust server
  must consume Portless's injected `PORT` and must not bind its raw `8787` default;
- start `trunk serve --release --address 127.0.0.1 --port "$PORT"` with stable Portless base name
  `leader-ai-browser`, baking the exact named API route as `wss://.../ws`;
- use the exact `.localhost` routes reported by `portless list`/`portless get`; a linked-worktree
  prefix is part of the recorded stable route, and no numeric localhost URL is accepted;
- fail setup if either child ignores `PORT`, either route is absent, the WASM bundle cannot connect,
  or the named route changes during restart; and
- never use Bun, `bunx`, Next.js, `scripts/portless.mjs`, `scripts/build-web.sh --serve`,
  `PORTLESS=0`, or `PORTLESS=skip` for this gate.

### Required Playwright play tests

Use the connected Playwright browser automation against the exact Portless `.localhost` route.
The run must operate the shipped UI through accessible roles, labels, text, pointer/keyboard input,
and browser navigation. It may read DOM/accessibility state, screenshots, console messages, network
failures, and downloaded evidence, but it must not call private action endpoints, inject state,
evaluate JavaScript to mutate the game, bypass authentication, or replace physical simulation time
with an undocumented test hook.

The replayable scenario contract lives in
[browser-playtests/playwright-scenario-manifest.md](browser-playtests/playwright-scenario-manifest.md)
and the immutable artifact schema lives in
[browser-playtests/evidence-schema.md](browser-playtests/evidence-schema.md). Those files are
authoritative for checkpoint IDs, screenshot names, locator manifests, forbidden values,
restart-linkage fields, cleanup, and the one-to-one Playwright to visible-browser mapping.

Run all eight journeys below in deterministic order. For each checkpoint, retain the Playwright
action transcript, locator used, before/after screenshot, accessible/DOM assertion output,
console errors and warnings, failed requests, current URL, simulation tick, and authoritative IDs.
The automated run must additionally prove that selectors survive a clean reload and that failed or
stale actions surface through the same controls a player uses. A Playwright-only success cannot
prove final pixel placement or desktop accessibility and therefore cannot satisfy the visible
browser gate.

### Required visible-browser observation

Operate the actual browser exclusively through `orca-ide computer` after discovering the browser
app/window with `list-apps` and `list-windows`. Navigation and every click, key, scroll, drag, and
value change use the newest accessibility snapshot; stale element indexes are forbidden. Raw canvas
coordinates require a current screenshot, window selector, scale conversion, and a follow-up state
capture. Direct action endpoints, injected DOM/JavaScript state changes, WebDriver, headless-only
browsers, `curl`, and BRP cannot satisfy this visible-observation portion. Playwright is required by
the preceding gate but cannot replace this independent visible-browser run.

Each checkpoint stores the full `get-app-state --json` accessibility tree plus its screenshot, then
opens the real browser DevTools Console with `CmdOrCtrl+Shift+I` and stores its tree/screenshot. The
operator checks startup, every scenario, reconnect, and final state for uncaught exceptions,
rejected promises, WASM/WebGL failures, asset 404s, repeated WebSocket errors, and warnings. Any
unclassified error fails; each warning needs a written disposition. Missing accessibility or
screenshot permission, an unobservable browser/DevTools window, or a failed capture blocks the card
instead of allowing a visual assertion to be skipped. DevTools is read-only: no JavaScript or
console command may be entered, and every temporary screenshot path must be copied into the evidence
bundle at its checkpoint.

### Required browser journeys

Run the following in deterministic order, with a fresh accessibility state and screenshot before
and after each mutation:

1. **Workshop footprint:** prove one task reports a canonical width/height 3 objective with exactly
   nine ordered cells, and visually renders all nine tiles plus distinct work-slot and delivery
   markers without duplicates or stale entities.
2. **Cave Hunt and water placement:** prove Hunt objective/marker is the revealed reachable cave or
   hunting-source identity with no radial fallback. Prove Fetch Water separately identifies and
   renders actual water, reachable dry bank/work position, and pinned delivery endpoint.
3. **Plans and officer reports:** operate the top-eight controls, verify refreshed reasons,
   confidence, standing-order/nudge feedback, officer authority/vacancy, and bounded reports.
4. **God regeneration secrecy:** run owning-god and another authorized-god browser sessions below
   effective report level four. Search visible labels, tooltips, inspectors, accessibility trees,
   screenshots, and console/errors for the fixture's regeneration field/value and hidden sentinel;
   all must be absent while permitted non-regeneration report bounds/provenance remain present.
5. **Hole/Notes/Void/research:** observe a belief-selected endless Hole feed reserve, exact haul and
   pinned-edge deposit, verify one exact Void Insight credit, inspect both research lanes and the
   Leader quota, complete physical preparation, then fund one Notes/Void study or activate one
   specialized boost with one debit and refreshed duration/effect state.
6. **Cat care:** inspect attribute/personality/stress/anatomy state and exercise an available
   treatment, refusal/consent, fitting, or repair control, proving typed feedback and conserved item/
   cargo identity.
7. **Diplomacy/trade:** with two authorized browser sessions, record mutual consent, proposal/
   acceptance, belief-based valuation, escrow, and physical pickup/delivery or explicit route block;
   neither browser may expose the other's hidden stock.
8. **Save/restart:** capture an active task/Hole-feed/research/fit/trade stage, restart the Rust server
   with the same SQLite file and named Portless route, and use the still-open actual browser to
   observe reconnect. IDs, objective/work/delivery, cargo, reservations, reports, stage, controls,
   and Notes/Void balances must survive exactly, with no duplicate effect, stale marker,
   currency alias, or console error.

### Reproducible evidence

Store the immutable run under
`docs/leader-ai-overhaul/evidence/lai33a/<commit>-seed-<seed>/`. The manifest includes full commit
SHA and dirty diff hash if applicable, seed, exact API/browser URLs, browser/version/OS/viewport,
protocol and persistence versions, SQLite fixture/checksum, server/Trunk commands, start/end ticks,
scenario/action order, the complete Playwright action/locator/assertion trace with its screenshots,
console and failed-request records, all `orca-ide computer` command JSON, accessibility trees,
visible-browser screenshot files, DevTools console captures, warning dispositions, restart
identifiers, and per-checkpoint result. Every visible-browser screenshot and console capture is
paired with the same checkpoint's accessibility state. The card fails if evidence cannot be
replayed, any required surface is omitted, either browser gate is missing, or a result relies only
on a screenshot, only on an accessibility tree, or only on automated coverage.

The required `manifest.json` schema, file inventory, PASS/FAIL fields, warning disposition format,
and per-checkpoint artifact paths are defined in
[browser-playtests/evidence-schema.md](browser-playtests/evidence-schema.md).

## Serialized liveness and browser execution policy

The Leader AI campaign and Bevy/WASM build are intentionally heavyweight. Local acceptance uses one
heavy process at a time. Do not run campaign shards, Clippy crates, Trunk builds, or Playwright
workers concurrently merely to reduce wall-clock time.

Use `CARGO_BUILD_JOBS=1`, a bounded CPU set such as `taskset -c 0-3`, and one test thread. Run the
focused diagnostic first. A probe that reaches its wall-time bound without an `AfterTick` or phase
`Exit` record is liveness-red; terminate it and inspect the final entry record instead of increasing
the horizon repeatedly. The available phase/boundary state and exact commands are documented in
[diagnostics-and-debugging.md](diagnostics-and-debugging.md).

The final browser gate uses:

1. `scripts/leader-ai-browser-fixture.sh --check`;
2. one fresh `--run` release stack through the named `leader-ai-api.localhost` and
   `leader-ai-browser.localhost` Portless routes;
3. `NODE_PATH=/usr/lib/node_modules playwright test --config=playwright.config.cjs` with the checked
   configuration's single worker; and
4. server shutdown before another Cargo gate.

The final committed fixtures use fresh seed `1395892225`, fresh-schema restart seed `1395892226`,
protocol version 3, and the canonical persistence schema. Their SQLite SHA-256 values must be
regenerated and recorded after the final schema lands; the retired protocol-v2/migration checksum is
not acceptance evidence. The launcher copies a pristine fixture before use. A browser run against a
previously mutated runtime copy is not final acceptance.

## Atomic release gate

LAI.70 may cut over only when every dependency card is accepted, all evidence
fields are populated, four Forgejo shards pass, root docs are synchronized, and a repository search
plus tests prove there is one production planner/currency/Hole/action path. The protocol and
persistence versions, server, fresh fixtures, simulation, and Bevy client land together. LAI.70 also
verifies the extension checklist against the final module/registry layout so this guide names the
shipped authorities rather than obsolete integration hotspots, and links the complete final
browser evidence manifest.

Delete conflicting legacy runtime code, types, tests, reliability controls, tithe/cooldown/scalar
paths, and spendable research currency only after replacement coverage is green. There is no
runtime shadow mode, dual mutation, feature flag, or downgrade. Offline comparison fixtures remain
test-only. Record exact deleted symbols, fresh schema/fixture version, protocol version, Forgejo run,
campaign summaries, UI captures, and root-doc changes on LAI.70.
