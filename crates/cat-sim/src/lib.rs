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
pub mod noise;
pub mod terrain_gen;
pub mod world_gen;

// P3 cat-AI modules (filled by P3.3–P3.9).
pub mod cat_ai;
pub mod leader_ai;
pub mod leader_director;
pub mod movement;
pub mod pathfinding;
pub mod policy;
pub mod tasks;
