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
