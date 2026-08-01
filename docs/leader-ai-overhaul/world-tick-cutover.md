# LAI.23 world-tick cutover map

> Historical first-cutover map. Current LAI.46/LAI.63 ordered integration is defined in
> [`integrated-implementation-map.md`](integrated-implementation-map.md); Shrine/Favor and the
> partial LAI.23 phase ownership below are not the final runtime authority.

This document is the implementation map for the sole LAI.23 production cutover. It is additive
evidence for the red harness in `crates/cat-sim/tests/world_tick_cutover.rs`; it does not authorize
any partial runtime switch, shadow planner, or dual mutation.

## Required ordered path

`world_tick` remains the single simulation entry point, but after cutover it must orchestrate only
focused leaf modules in this order:

1. `phase_lai23_01_authoritative_ecology_needs_hazards_emergencies`
   - Advance authoritative ecology, needs, spoilage, hazards, raids, active emergencies, and the
     existing hard survival status transitions.
   - Do not publish report data from hidden truth in this phase.
2. `phase_lai23_02_beliefs_reports_expiry_contradictions`
   - Apply physical observations, officer reports, report expiry/decay, contradiction invalidation,
     and bounded feedback.
   - Regeneration remains hidden below the documented report level and appears only as a belief
     projection, never as an executor timestamp or exact replenishment field.
3. `phase_lai23_03_leader_officer_review_boundaries`
   - Run every crossed Leader and officer review boundary in chronological order.
   - Use `leader_planner`, `officer_expertise`, `officer_requests`, `intent_graph`, and authority
     leaves. Do not call `leader_ai`, `leader_director`, `automated_plan`, or policy reliability.
4. `phase_lai23_04_scheduler_workforce_spatial_reservations`
   - Convert approved intents into executable task dependencies.
   - Use `scheduler`, `reservation_transaction`, `spatial_resolver`, `world_reservations`,
     `workforce_matcher`, `cat_willingness`, and `cat_stress`.
   - The transaction owns source, route, endpoint capacity, cargo, tools, slots, and worker
     assignment together.
5. `phase_lai23_05_visible_task_runtime_movement_cargo`
   - Advance only `task_runtime` visible stages and physical movement/cargo state.
   - Hunt, Water, Workshop, station, care, and trade tasks retain objective, work position, endpoint,
     route, reservations, and cargo across restart.
6. `phase_lai23_06_shrine_favor_offerings`
   - Use `shrine_offerings` and `favor`.
   - The old immediate tithe, offering cooldown, scalar blessing, `global_upgrade_points`, and
     material-offering logistics paths must be gone.
7. `phase_lai23_07_research_scholars_boosts`
   - Use `research_manifest`, `research_purchase`, scholar/Insight/preparation leaves, and
     `divine_boosts`.
   - Favor is the only spendable research/boost currency; old research points and automatic
     upgrade-tree unlocks no longer mutate production state.
8. `phase_lai23_08_diplomacy_trade_contracts`
   - Use `diplomacy`, `trade_valuation`, and `autonomous_trade`.
   - Accepted contracts advance through physical escrow and route stages with stable next-event
     ordering.
9. `phase_lai23_09_stress_injury_prosthetic_lifecycle`
   - Apply `cat_stress`, `cat_willingness`, `injuries`, `prosthetics`, and acquired traits.
   - Refusal, injury, death, treatment, fitting, repair, and cargo salvage release or revalidate
     reservations atomically.
10. `phase_lai23_10_report_safe_snapshots_events`
    - Publish report-safe feedback and deduplicated event surfaces only after all authoritative
      mutations for the tick have completed.

The function names above are deliberate red-harness markers. LAI.23 may implement them as functions
or clearly named calls, but adding inert comments or unused shims to satisfy the harness is not a
valid cutover.

## Removal checklist

Remove or fully retire these world-tick roots during LAI.23:

- `leader_ai::{LeaderDecision, LeaderSnapshot}` runtime planning.
- `leader_director`, `automated_plan`, `direct_colony`, and all old labor/director decision
  adapters.
- Per-action reliability miss machinery, including `policy.config.action_reliability` and
  `next_base_roll(colony)` calls used to decide whether automation mutates.
- `phase_21_leader_capital_decisions_and_tithe`, `LeaderDecision::Tithe`,
  `automated_tithe_ready`, `last_tithe_at`, `last_offering_at`, `ritual_requested_at`, and
  material-offering carry/ritual metadata.
- `global_upgrade_points`, `resources.blessings` as a spendable path, and presentation balances that
  mirror Favor instead of reading `FavorLedger`.
- `phase_24_research`, `accrue_research`, `cat_auto_unlock`, `upgrade_tree.research_points`,
  `owned_node_ids` mutation, and `last_leader_research_choice_at` as automatic research state.
- Job-kind destination fallback, `destination_for_job`, `JobDestinationContext`,
  `phase_17_legacy_emergency_hunt`, `phase_17b_water_reserve_preemption`, `LeaderPlanHunt`, radial
  objective selection, and generic straight-line movement after pathfinding failure.
- Any protocol/server snapshot field that exposes exact hidden regeneration or replenishment before
  the belief/report layer authorizes it.

Legacy data can remain as migration input until LAI.26/LAI.34, but LAI.23 must ensure those fields
cannot keep mutating production truth in parallel with the new leaves.

## Spatial invariants

LAI.23 must preserve and wire these already-established spatial contracts:

- Hunt resolves an actual revealed reachable hunting source or blocks with no marker and no worker.
- Fetch Water resolves actual water, reachable dry bank, and pinned endpoint as three distinct
  facts.
- Workshop objective remains the canonical `3 x 3` footprint with nine row-major tiles.
- Missing objective, missing route, route closure, refused worker, and invalid restart state block
  before a cat is marked busy.
- Picked-up cargo is delivered to its pinned endpoint or salvaged to a validated owned stockpile
  before a task blocks.

## Restart and determinism gates

Production cutover must add explicit restart/partition guard calls equivalent to:

- `lai23_revalidate_active_tasks_after_restart`
- `lai23_assert_no_duplicate_leaf_mutations_after_restart`
- `lai23_tick_partition_equivalence`

The concrete implementation can choose the helper names, but the behavior must prove:

- saved active tasks revalidate objective, route, endpoint, cargo, worker, and reservations before
  continuing;
- no Favor, research, boost, trade, cargo, injury, prosthetic, or event mutation can apply twice
  after replay;
- one large supported advance and equivalent smaller partitions process planning boundaries,
  automatic quota windows, boost expiry, task stages, and cargo transitions identically.

## Red harness map

`world_tick_cutover.rs` currently contains these characterization tests:

- `lai23_single_ordered_phase_path_is_installed`
- `legacy_planner_director_reliability_tithe_and_research_schedules_are_removed`
- `world_tick_calls_completed_leaf_modules_instead_of_shadow_paths`
- `spatial_execution_has_no_hunt_water_or_work_movement_fallbacks`
- `duplicate_mutation_sites_are_removed_from_world_tick_root`
- `hidden_regeneration_fields_do_not_escape_report_projection`
- `workshop_footprint_contract_remains_canonical_three_by_three`
- `restart_and_partition_divergence_guards_are_present_in_cutover_root`

All failures except the Workshop footprint guard are expected before LAI.23. The harness is not a
green gate for this subtask; it is the red contract LAI.23 must turn green with causal production
changes in `world_tick` and its owned integration surfaces.
