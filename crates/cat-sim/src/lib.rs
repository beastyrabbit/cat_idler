//! `cat-sim` — the pure, render-free, I/O-free multi-colony simulation core.
//!
//! Ported from the TypeScript `lib/game/*` modules and `server/game.ts:workerTick`.
//! No rendering, networking, filesystem, clock, or `rand`; all randomness flows
//! through [`rng`]. See `AGENTS.md` and the migration plan for the rules.

#![forbid(unsafe_code)]

pub mod rng;
pub mod types;

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
pub mod ledger;
pub mod processing;
pub mod production;
pub mod roads;
pub mod shrine;
pub mod skills;
pub mod smithy;
pub mod spoilage;
pub mod stockpiles;
pub mod storage;
pub mod trips;
pub mod village_area;
pub mod village_layout;
pub mod village_sites;

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
