//! Ad-hoc playtest harness: found a colony and watch it live unattended, printing an
//! hourly vital-signs table and flagging anomalies a player would notice (extinction,
//! resets, starvation, idle cats, stalled research/economy, unfought raids).
//!
//! Not a test — a telescope. Run:
//!   SEED=20240712 HOURS=48 CADENCE_MS=1000 cargo run -p cat-sim --example playtest
//! CADENCE_MS=1000 is the true live server cadence (1 tick = 1 game-second); larger
//! values fast-forward (coarser, can hide live-cadence failures).

use std::collections::HashSet;

use cat_sim::actions::build_snapshot;
use cat_sim::world_tick::{found_colony, new_world, world_tick};

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn main() {
    let seed = env_u64("SEED", 20_240_712) as u32;
    let hours = env_u64("HOURS", 48);
    let cadence_ms = env_u64("CADENCE_MS", 1000) as i64;

    let mut world = new_world(seed);
    world
        .colonies
        .push(found_colony(world.world_seed, "colony-1", 1_000, seed));

    println!(
        "# playtest seed={seed} hours={hours} cadence_ms={cadence_ms}  (1 tick = {}s game-time)",
        cadence_ms / 1000
    );
    println!(
        "{:>4} {:>4} {:>5} {:>5} {:>5} {:>4} {:>4} {:>4} {:>5} {:>4} {:>3} {:>4} {:>6} {:>3} {:>3} {:>4} {:>5}",
        "hr",
        "pop",
        "food",
        "watr",
        "matl",
        "plnk",
        "blck",
        "tool",
        "bless",
        "rp",
        "nod",
        "bld*",
        "avgHlth",
        "idl",
        "war",
        "raid",
        "b/d"
    );

    let mut now_ms: i64 = 1_000;
    let ticks_per_hour = (3_600_000 / cadence_ms).max(1);
    let total_ticks = ticks_per_hour * hours as i64;

    let mut known_ids: HashSet<String> = HashSet::new();
    let mut births = 0u64;
    let mut deaths = 0u64;
    let mut resets = 0u64;
    let mut reset_reasons: Vec<String> = Vec::new();
    let mut min_pop = u32::MAX;
    let mut max_raiders = 0usize;
    let mut ever_extinct = false;
    let mut hours_food_zero = 0u64;
    let mut hours_water_zero = 0u64;

    // seed the id set from the founding roster
    {
        let snap = build_snapshot(&world, now_ms, 1);
        if let Some(c) = snap.colonies.first() {
            for cat in &c.cats {
                known_ids.insert(cat.id.clone());
            }
        }
    }

    for tick in 1..=total_ticks {
        now_ms += cadence_ms;
        let reports = world_tick(&mut world, now_ms);
        for r in &reports {
            if let Some(reason) = &r.reset_reason {
                resets += 1;
                reset_reasons.push(format!("hr~{} {:?}", tick / ticks_per_hour, reason));
            }
        }

        if tick % ticks_per_hour == 0 {
            let hr = tick / ticks_per_hour;
            let snap = build_snapshot(&world, now_ms, 1);
            let Some(c) = snap.colonies.first() else {
                continue;
            };

            // birth/death via id-set diff
            let live_ids: HashSet<String> = c
                .cats
                .iter()
                .filter(|c| c.death_time.is_none())
                .map(|c| c.id.clone())
                .collect();
            for id in &live_ids {
                if known_ids.insert(id.clone()) {
                    births += 1;
                }
            }
            // deaths: ids we knew that are no longer live
            let gone: Vec<String> = known_ids
                .iter()
                .filter(|id| !live_ids.contains(*id))
                .cloned()
                .collect();
            for id in gone {
                deaths += 1;
                known_ids.remove(&id);
            }

            let pop = live_ids.len() as u32;
            min_pop = min_pop.min(pop);
            if pop == 0 {
                ever_extinct = true;
            }
            let r = &c.resources;
            if r.food <= 0.5 {
                hours_food_zero += 1;
            }
            if r.water <= 0.5 {
                hours_water_zero += 1;
            }
            let avg_health = if pop > 0 {
                c.cats
                    .iter()
                    .filter(|c| c.death_time.is_none())
                    .map(|c| c.needs.health)
                    .sum::<f64>()
                    / pop as f64
            } else {
                0.0
            };
            let idle = c
                .cats
                .iter()
                .filter(|c| {
                    c.death_time.is_none() && matches!(c.activity, cat_protocol::CatActivity::Idle)
                })
                .count();
            let completed = c
                .buildings
                .iter()
                .filter(|b| b.construction_progress >= 100.0)
                .count();
            let total_b = c.buildings.len();
            max_raiders = max_raiders.max(c.raiders.len());

            println!(
                "{hr:>4} {pop:>4} {:>5.0} {:>5.0} {:>5.0} {:>4.0} {:>4.0} {:>4.0} {:>5.1} {:>4.1} {:>3} {:>4} {:>6.0} {idle:>3} {:>3} {:>4} {:>3}/{:<3}",
                r.food,
                r.water,
                r.materials,
                r.planks,
                r.blocks,
                r.tools,
                r.blessings,
                c.research.research_points,
                c.research.owned_node_ids.len(),
                format!("{completed}/{total_b}"),
                avg_health,
                c.threat.warriors,
                if c.threat.raid_active { "RAID" } else { "-" },
                births,
                deaths,
            );
        }
    }

    println!("\n# SUMMARY seed={seed}");
    println!("  final births={births} deaths={deaths} resets={resets}");
    println!("  min_pop={min_pop} ever_extinct={ever_extinct} max_raiders_seen={max_raiders}");
    println!(
        "  hours_with_food_near_zero={hours_food_zero} hours_with_water_near_zero={hours_water_zero}"
    );
    if !reset_reasons.is_empty() {
        println!("  RESETS: {}", reset_reasons.join(" | "));
    }

    // Anomaly flags
    let mut flags = Vec::new();
    if ever_extinct {
        flags.push("EXTINCTION (colony hit 0 cats)".to_string());
    }
    if resets > 0 {
        flags.push(format!("{resets} RUN RESET(S)"));
    }
    if hours_food_zero > hours / 4 {
        flags.push(format!(
            "FOOD near-zero {hours_food_zero}/{hours} hrs (starvation risk)"
        ));
    }
    if hours_water_zero > hours / 4 {
        flags.push(format!(
            "WATER near-zero {hours_water_zero}/{hours} hrs (dehydration risk)"
        ));
    }
    if births == 0 {
        flags.push("NO BIRTHS over the whole run (population can't grow)".to_string());
    }
    if flags.is_empty() {
        println!("  ✓ no gross anomalies flagged");
    } else {
        println!("  ⚠ ANOMALIES:");
        for f in flags {
            println!("    - {f}");
        }
    }
}
