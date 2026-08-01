//! `cat-sim` — the pure, render-free, I/O-free multi-colony simulation core.
//!
//! Ported from the TypeScript `lib/game/*` modules and `server/game.ts:workerTick`.
//! No rendering, networking, filesystem, clock, or `rand`; all randomness flows
//! through [`rng`]. See `AGENTS.md` and the migration plan for the rules.

#![forbid(unsafe_code)]

pub mod acquired_traits;
pub mod anatomy;
pub mod authority;
pub mod autonomous_trade;
pub mod beliefs;
pub mod black_hole;
pub mod campaign_runner;
pub mod cat_capabilities;
pub mod cat_capability_authority;
pub mod cat_governance;
pub mod cat_stress;
pub mod cat_traits;
pub mod cat_willingness;
pub mod construction_catalog;
pub mod construction_miracle_runtime;
pub mod construction_runtime;
pub mod construction_stages;
pub mod content_manifest;
pub mod cookhouse;
pub mod diplomacy;
pub mod divine_action_offers;
pub mod divine_boosts;
pub mod divine_hole_authority;
pub mod family_authority;
pub mod family_housing;
pub mod family_specialization;
pub mod favor;
pub mod fishing;
pub mod food_divine_policy;
pub mod food_ecology;
pub mod governance_authority;
pub mod hunting_lair;
pub mod intent_graph;
pub mod leader_ai_diagnostics;
pub mod leader_ai_runtime;
pub mod leader_planner;
pub mod material_crafting;
pub mod moneyless_barter;
pub mod officer_expertise;
pub mod officer_requests;
pub mod physical_storage;
pub mod planner_core;
pub mod player_directives;
pub mod player_projection;
pub mod progression_research;
pub mod prosthetics;
pub mod quality_lots;
pub mod research_authority;
pub mod research_manifest;
pub mod research_purchase;
pub mod reservation_transaction;
pub mod rng;
pub mod scheduler;
pub mod scholar_research;
pub mod shrine_offerings;
pub mod skill_catalog;
pub mod spatial_resolver;
pub mod storage_authority;
pub mod task_runtime;
pub mod trade_authority;
pub mod trade_valuation;
pub mod types;
pub mod village_infrastructure;
pub mod workforce_matcher;
pub mod world_reservations;

// P1 foundation modules (filled by the P1.3–P1.6 cards).
pub mod cost_constants;
pub mod entities;
pub mod needs_constants;
pub mod test_acceleration;

// P2 world-generation modules (filled by P2.2–P2.14).
pub mod biomes;
pub mod climate;
pub mod noise;
pub mod terrain_gen;
pub mod world_gen;

// P3 cat-AI modules (filled by P3.3–P3.9).
pub mod cat_ai;
pub mod leader_ai;
pub mod leader_director;
pub mod movement;
pub mod officers;
pub mod pathfinding;
pub mod policy;
pub mod tasks;

// P4 life-sim modules.
pub mod age;
pub mod breeding;
pub mod genetics;
pub mod life_sim;
pub mod migration;
pub mod needs;
pub mod survival;

// P5 economy/housing/roads modules.
pub mod depletion;
pub mod farming;
pub mod housing;
pub mod idle_engine;
pub mod idle_rules;
pub mod injuries;
pub mod labor_pressure;
pub mod ledger;
pub mod processing;
pub mod production;
pub mod productivity;
pub mod roads;
pub mod shrine;
pub mod skills;
pub mod smithy;
pub mod spatial_tasks;
pub mod spoilage;
pub mod station_recipes;
pub mod stockpiles;
pub mod storage;
pub mod transport;
pub mod trips;
pub mod village_area;
pub mod village_layout;
pub mod village_sites;
pub mod village_trade_routes;

// P6 military/governance/upgrade-tree modules.
pub mod combat;
pub mod elections;
pub mod research_catalog;
pub mod threat;
pub mod upgrade_tree;
pub mod warriors;
pub mod zones;

// P7 master loop.
pub mod world_tick;

// P8 action application + snapshot building.
pub mod actions;

// P19 slice 1: DF-scale cat-themed item/material economy data model. Additive and
// inert this slice — see `items` module docs.
pub mod items;

// P19 slice 2: material-variant trade-good crafting recipes (workshops → items).
pub mod recipes;

// P19 slice 3: visiting trader / caravan lifecycle + coin economy pricing.
pub mod trader;

pub use campaign_runner::{
    LAI32_RELEASE_PROFILE_MAX_REGRESSION_PERCENT, Lai32CampaignCategory, Lai32CampaignOutcome,
    Lai32CampaignPerformanceSample, Lai32CampaignRunner, Lai32CampaignScenario,
    assert_at_least_four_affordable_auto_research_commits,
    assert_believable_good_and_bad_leader_variation, assert_exact_void_insight_conservation,
    assert_hidden_regeneration_secrecy_below_l4, assert_hunt_water_workshop_spatial_invariants,
    assert_lai32_bounded_state_and_queues, assert_lai32_established_success_threshold_97_of_100,
    assert_lai32_fresh_success_threshold_85_of_100, assert_lai32_restart_twins,
    assert_lai32_tick_partition_twins, assert_no_duplicate_void_research_trade_cargo_mutations,
    assert_no_starvation_caused_solely_by_endless_hole_demand, compare_lai32_against_lai1_baseline,
    compare_lai32_partitioned_snapshots_byte_equal, compare_lai32_restart_snapshots_byte_equal,
    lai32_run_30_day_campaign, lai32_run_small_red_smoke_campaigns, measure_lai32_release_profile,
    record_lai32_wall_time_and_peak_rss, run_lai32_release_profile_campaign_matrix,
    run_lai32_restart_partition_campaign_matrix,
};
