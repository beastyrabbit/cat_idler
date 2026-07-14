//! Deterministic unattended-play campaign with explicit feature telemetry.
//!
//! Run at the live server cadence:
//! `SEED=20240712 HOURS=48 CADENCE_MS=1000 cargo run --release -p cat-sim --example playtest`.
//! Set `COMMUNAL=1` to exercise the larger shared Grand Commons blueprint instead of an exact
//! personal founding.
//! The default expectations cover systems that do not require a player-established officer.
//! Override them with a comma-separated `EXPECT_FEATURES` list (or `none`) when running an
//! established fixture, relax the idle-stall limit with `MAX_IDLE_STALL_HOURS`, or use
//! `STRICT=0` to report failures without a non-zero exit.

use std::collections::{BTreeMap, HashSet};

use cat_sim::{
    actions::build_snapshot,
    entities::CarryingKind,
    types::BuildingType,
    world_tick::{ColonyRuntime, found_colony, found_global_colony, new_world, world_tick},
};

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| !matches!(value.to_ascii_lowercase().as_str(), "0" | "false" | "no"))
        .unwrap_or(default)
}

fn default_expected_features(hours: u64) -> HashSet<String> {
    let mut expected = HashSet::new();
    if hours >= 1 {
        expected.insert("election".to_owned());
        expected.insert("fog".to_owned());
        expected.insert("scout".to_owned());
    }
    if hours >= 4 {
        expected.insert("hunt".to_owned());
        expected.insert("water".to_owned());
    }
    if hours >= 60 {
        expected.insert("trader".to_owned());
    }
    expected
}

fn expected_features(hours: u64) -> HashSet<String> {
    let Ok(raw) = std::env::var("EXPECT_FEATURES") else {
        return default_expected_features(hours);
    };
    raw.split(',')
        .map(str::trim)
        .filter(|feature| !feature.is_empty() && *feature != "none")
        .map(str::to_owned)
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
struct PlaytestSummary {
    seed: u32,
    communal: bool,
    hours: u64,
    cadence_ms: i64,
    births: u64,
    deaths: u64,
    resets: u64,
    reset_reasons: Vec<String>,
    min_population: u32,
    max_population: u32,
    ever_extinct: bool,
    hours_food_zero: u64,
    hours_water_zero: u64,
    max_idle_stall_hours: u64,
    max_raiders: usize,
    raids_spawned: u64,
    raids_resolved: u64,
    raids_resolved_without_player_defense: u64,
    active_raid_at_end: bool,
    event_counts: BTreeMap<String, u64>,
    initial_revealed_tiles: usize,
    final_revealed_tiles: usize,
    ritual_jobs_seen: u64,
    ritual_jobs_completed: u64,
    offering_jobs_seen: u64,
    offering_jobs_completed: u64,
    hunt_jobs_seen: u64,
    hunt_jobs_completed: u64,
    water_jobs_seen: u64,
    water_jobs_completed: u64,
    leader_scout_jobs_seen: u64,
    leader_scout_jobs_completed: u64,
    buildings_commissioned: BTreeMap<String, u64>,
    buildings_completed: BTreeMap<String, u64>,
    first_researcher_hour: Option<u64>,
    first_research_point_hour: Option<u64>,
    first_research_node_hour: Option<u64>,
    final_research_points: f64,
    final_research_nodes: usize,
    first_field_commissioned_hour: Option<u64>,
    first_field_completed_hour: Option<u64>,
    final_completed_buildings: usize,
    tools_produced: f64,
    peak_tools: f64,
    final_tools: f64,
    final_god_blessings: f64,
    final_stored_blessings: f64,
    peak_carried_blessings: f64,
    peak_item_count: u64,
    final_item_count: u64,
    trader_visits: u64,
    trader_trading_windows: u64,
}

impl PlaytestSummary {
    fn event_count(&self, kind: &str) -> u64 {
        self.event_counts.get(kind).copied().unwrap_or(0)
    }

    fn anomaly_flags(&self, expected: &HashSet<String>, idle_stall_limit: u64) -> Vec<String> {
        let mut flags = Vec::new();
        if self.ever_extinct {
            flags.push("EXTINCTION: colony hit zero living cats".to_owned());
        }
        if self.resets > 0 {
            flags.push(format!("{} RUN RESET(S)", self.resets));
        }
        if self.hours_food_zero > self.hours / 4 {
            flags.push(format!(
                "FOOD near zero for {}/{} sampled hours",
                self.hours_food_zero, self.hours
            ));
        }
        if self.hours_water_zero > self.hours / 4 {
            flags.push(format!(
                "WATER near zero for {}/{} sampled hours",
                self.hours_water_zero, self.hours
            ));
        }
        // Foundings intentionally start at exact housing capacity. No birth without a
        // player/Steward-built Den is the designed outcome, not a passive-run anomaly.
        if self.max_idle_stall_hours > idle_stall_limit {
            flags.push(format!(
                "IDLE STALL: every living cat was idle for {} consecutive sampled hours (limit {idle_stall_limit})",
                self.max_idle_stall_hours
            ));
        }

        let mut expectations: Vec<&str> = expected.iter().map(String::as_str).collect();
        expectations.sort_unstable();
        for feature in expectations {
            let reached = match feature {
                "election" => {
                    self.event_count("election_opened") > 0
                        && self.event_count("election_resolved") > 0
                }
                "field" => {
                    self.first_field_commissioned_hour.is_some()
                        && self.first_field_completed_hour.is_some()
                }
                "fog" => {
                    self.final_revealed_tiles > self.initial_revealed_tiles
                        && self.event_count("discovery") > 0
                }
                "hunt" => {
                    self.hunt_jobs_seen > 0 && (self.hours < 24 || self.hunt_jobs_completed > 0)
                }
                "items" => self.peak_item_count > 0,
                "offering" => {
                    self.event_count("offering") > 0 && self.event_count("blessing_delivered") > 0
                }
                "raid" => self.raids_spawned > 0 && self.raids_resolved > 0,
                "scout" => self.leader_scout_jobs_seen > 0 && self.leader_scout_jobs_completed > 0,
                "research" => {
                    self.first_researcher_hour.is_some() && self.first_research_point_hour.is_some()
                }
                "research_node" => self.first_research_node_hour.is_some(),
                "tithe" => self.event_count("tithe") > 0,
                "tools" => self.tools_produced >= 1.0,
                "water" => {
                    self.water_jobs_seen > 0 && (self.hours < 24 || self.water_jobs_completed > 0)
                }
                "trader" => self.trader_visits > 0 && self.trader_trading_windows > 0,
                unknown => {
                    flags.push(format!("UNKNOWN EXPECTATION: {unknown}"));
                    continue;
                }
            };
            if !reached {
                flags.push(format!(
                    "UNREACHED FEATURE: {feature} did not complete its observable lifecycle"
                ));
            }
        }
        flags
    }
}

fn is_research_building(building_type: BuildingType) -> bool {
    matches!(
        building_type,
        BuildingType::ResearchHut | BuildingType::School
    )
}

fn count_completed_buildings(colony: &ColonyRuntime) -> usize {
    colony
        .buildings
        .iter()
        .filter(|building| building.construction_progress >= 100)
        .count()
}

fn game_hour(tick: i64, ticks_per_hour: i64) -> u64 {
    tick.div_euclid(ticks_per_hour) as u64
}

fn run_campaign(
    seed: u32,
    hours: u64,
    cadence_ms: i64,
    communal: bool,
    print_hourly: bool,
) -> PlaytestSummary {
    assert!(cadence_ms > 0, "CADENCE_MS must be positive");
    let mut world = new_world(seed);
    let colony = if communal {
        found_global_colony(world.world_seed, "colony-1", 1_000, seed)
    } else {
        found_colony(world.world_seed, "colony-1", 1_000, seed)
    };
    world.colonies.push(colony);

    if print_hourly {
        println!(
            "# passive campaign village={} seed={seed} hours={hours} cadence_ms={cadence_ms} (1 tick = {:.3}s game-time)",
            if communal { "communal" } else { "personal" },
            cadence_ms as f64 / 1000.0
        );
        println!(
            "{:>4} {:>4} {:>5} {:>5} {:>5} {:>4} {:>4} {:>4} {:>5} {:>5} {:>4} {:>3} {:>4} {:>6} {:>3} {:>3} {:>4} {:>7}",
            "hr",
            "pop",
            "food",
            "watr",
            "matl",
            "plnk",
            "blck",
            "tool",
            "god",
            "rp",
            "node",
            "bld",
            "item",
            "health",
            "idl",
            "war",
            "raid",
            "birth/death"
        );
    }

    let mut now_ms = 1_000_i64;
    let ticks_per_hour = (3_600_000 / cadence_ms).max(1);
    let total_ticks = ticks_per_hour.saturating_mul(hours as i64);
    let initial = &world.colonies[0];
    let initial_revealed_tiles = initial.revealed_tiles.len();
    let mut known_ids: HashSet<String> = initial
        .cats
        .iter()
        .filter(|cat| cat.death_time.is_none())
        .map(|cat| cat.id.clone())
        .collect();
    let mut seen_events: HashSet<String> = HashSet::new();
    let mut event_counts = BTreeMap::new();
    let mut births = 0_u64;
    let mut deaths = 0_u64;
    let mut resets = 0_u64;
    let mut reset_reasons = Vec::new();
    let mut min_population = u32::MAX;
    let mut max_population = 0_u32;
    let mut ever_extinct = false;
    let mut hours_food_zero = 0_u64;
    let mut hours_water_zero = 0_u64;
    let mut current_idle_stall = 0_u64;
    let mut max_idle_stall_hours = 0_u64;
    let mut max_raiders = 0_usize;
    let mut raids_spawned = 0_u64;
    let mut raids_resolved = 0_u64;
    let mut raids_resolved_without_player_defense = 0_u64;
    let mut observed_raid: Option<(String, f64)> = None;
    let mut seen_ritual_jobs = HashSet::new();
    let mut completed_ritual_jobs = HashSet::new();
    let mut seen_offering_jobs = HashSet::new();
    let mut completed_offering_jobs = HashSet::new();
    let mut seen_hunt_jobs = HashSet::new();
    let mut completed_hunt_jobs = HashSet::new();
    let mut seen_water_jobs = HashSet::new();
    let mut completed_water_jobs = HashSet::new();
    let mut seen_leader_scout_jobs = HashSet::new();
    let mut completed_leader_scout_jobs = HashSet::new();
    let mut known_buildings: HashSet<String> = initial
        .buildings
        .iter()
        .map(|building| building.id.clone())
        .collect();
    let mut known_completed_buildings: HashSet<String> = initial
        .buildings
        .iter()
        .filter(|building| building.construction_progress >= 100)
        .map(|building| building.id.clone())
        .collect();
    let mut buildings_commissioned = BTreeMap::new();
    let mut buildings_completed = BTreeMap::new();
    let mut first_researcher_hour = None;
    let mut first_research_point_hour = None;
    let mut first_research_node_hour = None;
    let mut first_field_commissioned_hour = None;
    let mut first_field_completed_hour = None;
    let mut last_tools = initial.resources.tools;
    let mut tools_produced = 0.0_f64;
    let mut peak_tools = last_tools;
    let mut peak_carried_blessings = 0.0_f64;
    let mut peak_item_count: u64 = initial.items.values().map(|count| u64::from(*count)).sum();
    let mut trader_visits = 0_u64;
    let mut trader_trading_windows = 0_u64;

    for tick in 1..=total_ticks {
        now_ms += cadence_ms;
        let reports = world_tick(&mut world, now_ms);
        for report in &reports {
            if let Some(reason) = report.reset_reason {
                resets += 1;
                reset_reasons.push(format!("hr~{} {reason:?}", game_hour(tick, ticks_per_hour)));
            }
        }

        let colony = &world.colonies[0];
        max_raiders = max_raiders.max(colony.raiders.len());
        for event in &colony.events {
            let kind = event.kind.wire_kind();
            if seen_events.insert(event.id.clone()) {
                *event_counts.entry(kind.clone()).or_insert(0) += 1;
                match kind.as_str() {
                    "field_commissioned" => {
                        first_field_commissioned_hour
                            .get_or_insert(game_hour(tick, ticks_per_hour));
                    }
                    "trader_arrived" => trader_visits += 1,
                    "trader_trading" => trader_trading_windows += 1,
                    _ => {}
                }
            }
        }
        for job in &colony.jobs {
            match job.kind {
                cat_sim::types::JobKind::Ritual => {
                    seen_ritual_jobs.insert(job.id.clone());
                    if job.status == cat_sim::types::JobStatus::Completed {
                        completed_ritual_jobs.insert(job.id.clone());
                    }
                }
                cat_sim::types::JobKind::CarryOffering => {
                    seen_offering_jobs.insert(job.id.clone());
                    if job.status == cat_sim::types::JobStatus::Completed {
                        completed_offering_jobs.insert(job.id.clone());
                    }
                }
                cat_sim::types::JobKind::HuntExpedition => {
                    seen_hunt_jobs.insert(job.id.clone());
                    if job.status == cat_sim::types::JobStatus::Completed {
                        completed_hunt_jobs.insert(job.id.clone());
                    }
                }
                cat_sim::types::JobKind::FetchWater => {
                    seen_water_jobs.insert(job.id.clone());
                    if job.status == cat_sim::types::JobStatus::Completed {
                        completed_water_jobs.insert(job.id.clone());
                    }
                }
                cat_sim::types::JobKind::Explore
                    if job.requested_by == cat_sim::world_tick::JobRequester::Leader =>
                {
                    seen_leader_scout_jobs.insert(job.id.clone());
                    if job.status == cat_sim::types::JobStatus::Completed {
                        completed_leader_scout_jobs.insert(job.id.clone());
                    }
                }
                _ => {}
            }
        }
        for building in &colony.buildings {
            let kind = building.building_type.as_str().to_owned();
            if known_buildings.insert(building.id.clone()) {
                *buildings_commissioned.entry(kind.clone()).or_insert(0) += 1;
            }
            if building.construction_progress >= 100
                && known_completed_buildings.insert(building.id.clone())
            {
                *buildings_completed.entry(kind).or_insert(0) += 1;
            }
        }

        let tools = colony.resources.tools;
        if tools > last_tools {
            tools_produced += tools - last_tools;
        }
        last_tools = tools;
        peak_tools = peak_tools.max(tools);
        let carried_blessings: f64 = colony
            .cats
            .iter()
            .filter_map(|cat| cat.carrying.as_ref())
            .filter(|carrying| carrying.kind == CarryingKind::Blessings)
            .map(|carrying| carrying.amount)
            .sum();
        peak_carried_blessings = peak_carried_blessings.max(carried_blessings);
        let item_count = colony.items.values().map(|count| u64::from(*count)).sum();
        peak_item_count = peak_item_count.max(item_count);
        let research_staffed = colony.buildings.iter().any(|building| {
            building.construction_progress >= 100
                && is_research_building(building.building_type)
                && building.assigned_cat.is_some()
        });
        if research_staffed {
            first_researcher_hour.get_or_insert(game_hour(tick, ticks_per_hour));
        }
        if colony.upgrade_tree.research_points > 0.0 {
            first_research_point_hour.get_or_insert(game_hour(tick, ticks_per_hour));
        }
        if !colony.upgrade_tree.owned_node_ids.is_empty() {
            first_research_node_hour.get_or_insert(game_hour(tick, ticks_per_hour));
        }
        if colony.buildings.iter().any(|building| {
            building.building_type == BuildingType::Field && building.construction_progress >= 100
        }) {
            first_field_completed_hour.get_or_insert(game_hour(tick, ticks_per_hour));
        }

        let current_raid = colony.active_raid.clone();
        match (observed_raid.take(), current_raid) {
            (None, Some(id)) => {
                raids_spawned += 1;
                observed_raid = Some((id, colony.raid_clicks));
            }
            (Some((_id, click_start)), None) => {
                raids_resolved += 1;
                if colony.raid_clicks <= click_start {
                    raids_resolved_without_player_defense += 1;
                }
            }
            (Some((old_id, click_start)), Some(new_id)) if old_id != new_id => {
                raids_resolved += 1;
                if colony.raid_clicks <= click_start {
                    raids_resolved_without_player_defense += 1;
                }
                raids_spawned += 1;
                observed_raid = Some((new_id, colony.raid_clicks));
            }
            (Some(observed), Some(_)) => observed_raid = Some(observed),
            (None, None) => {}
        }

        if tick % ticks_per_hour != 0 {
            continue;
        }

        let snapshot = build_snapshot(&world, now_ms, 1);
        let colony_snapshot = &snapshot.colonies[0];
        let live_ids: HashSet<String> = colony_snapshot
            .cats
            .iter()
            .filter(|cat| cat.death_time.is_none())
            .map(|cat| cat.id.clone())
            .collect();
        for id in &live_ids {
            if known_ids.insert(id.clone()) {
                births += 1;
            }
        }
        let gone: Vec<String> = known_ids
            .iter()
            .filter(|id| !live_ids.contains(*id))
            .cloned()
            .collect();
        for id in gone {
            deaths += 1;
            known_ids.remove(&id);
        }

        let population = live_ids.len() as u32;
        min_population = min_population.min(population);
        max_population = max_population.max(population);
        ever_extinct |= population == 0;
        hours_food_zero += u64::from(colony_snapshot.resources.food <= 0.5);
        hours_water_zero += u64::from(colony_snapshot.resources.water <= 0.5);
        let idle = colony_snapshot
            .cats
            .iter()
            .filter(|cat| {
                cat.death_time.is_none() && matches!(cat.activity, cat_protocol::CatActivity::Idle)
            })
            .count();
        if population > 0 && idle == population as usize {
            current_idle_stall += 1;
            max_idle_stall_hours = max_idle_stall_hours.max(current_idle_stall);
        } else {
            current_idle_stall = 0;
        }
        let average_health = if population == 0 {
            0.0
        } else {
            colony_snapshot
                .cats
                .iter()
                .filter(|cat| cat.death_time.is_none())
                .map(|cat| cat.needs.health)
                .sum::<f64>()
                / f64::from(population)
        };
        if print_hourly {
            println!(
                "{:>4} {population:>4} {:>5.0} {:>5.0} {:>5.0} {:>4.0} {:>4.0} {:>4.0} {:>5.1} {:>5.2} {:>4} {:>3} {:>4} {:>6.0} {idle:>3} {:>3} {:>4} {:>3}/{:<3}",
                game_hour(tick, ticks_per_hour),
                colony_snapshot.resources.food,
                colony_snapshot.resources.water,
                colony_snapshot.resources.materials,
                colony_snapshot.resources.planks,
                colony_snapshot.resources.blocks,
                colony_snapshot.resources.tools,
                colony.global_upgrade_points,
                colony_snapshot.research.research_points,
                colony_snapshot.research.owned_node_ids.len(),
                count_completed_buildings(colony),
                item_count,
                average_health,
                colony_snapshot.threat.warriors,
                if colony_snapshot.threat.raid_active {
                    "RAID"
                } else {
                    "-"
                },
                births,
                deaths,
            );
        }
    }

    let colony = &world.colonies[0];
    PlaytestSummary {
        seed,
        communal,
        hours,
        cadence_ms,
        births,
        deaths,
        resets,
        reset_reasons,
        min_population: if min_population == u32::MAX {
            colony
                .cats
                .iter()
                .filter(|cat| cat.death_time.is_none())
                .count() as u32
        } else {
            min_population
        },
        max_population,
        ever_extinct,
        hours_food_zero,
        hours_water_zero,
        max_idle_stall_hours,
        max_raiders,
        raids_spawned,
        raids_resolved,
        raids_resolved_without_player_defense,
        active_raid_at_end: colony.active_raid.is_some(),
        event_counts,
        initial_revealed_tiles,
        final_revealed_tiles: colony.revealed_tiles.len(),
        ritual_jobs_seen: seen_ritual_jobs.len() as u64,
        ritual_jobs_completed: completed_ritual_jobs.len() as u64,
        offering_jobs_seen: seen_offering_jobs.len() as u64,
        offering_jobs_completed: completed_offering_jobs.len() as u64,
        hunt_jobs_seen: seen_hunt_jobs.len() as u64,
        hunt_jobs_completed: completed_hunt_jobs.len() as u64,
        water_jobs_seen: seen_water_jobs.len() as u64,
        water_jobs_completed: completed_water_jobs.len() as u64,
        leader_scout_jobs_seen: seen_leader_scout_jobs.len() as u64,
        leader_scout_jobs_completed: completed_leader_scout_jobs.len() as u64,
        buildings_commissioned,
        buildings_completed,
        first_researcher_hour,
        first_research_point_hour,
        first_research_node_hour,
        final_research_points: colony.upgrade_tree.research_points,
        final_research_nodes: colony.upgrade_tree.owned_node_ids.len(),
        first_field_commissioned_hour,
        first_field_completed_hour,
        final_completed_buildings: count_completed_buildings(colony),
        tools_produced,
        peak_tools,
        final_tools: colony.resources.tools,
        final_god_blessings: colony.global_upgrade_points,
        final_stored_blessings: colony.resources.blessings,
        peak_carried_blessings,
        peak_item_count,
        final_item_count: colony.items.values().map(|count| u64::from(*count)).sum(),
        trader_visits,
        trader_trading_windows,
    }
}

fn print_summary(summary: &PlaytestSummary, expected: &HashSet<String>, flags: &[String]) {
    println!(
        "\n# SUMMARY village={} seed={}",
        if summary.communal {
            "communal"
        } else {
            "personal"
        },
        summary.seed
    );
    println!(
        "  population min={} max={} births={} deaths={} resets={}",
        summary.min_population,
        summary.max_population,
        summary.births,
        summary.deaths,
        summary.resets
    );
    println!(
        "  raids spawned={} resolved={} auto/unfought={} max_raiders={} active_at_end={}",
        summary.raids_spawned,
        summary.raids_resolved,
        summary.raids_resolved_without_player_defense,
        summary.max_raiders,
        summary.active_raid_at_end
    );
    println!(
        "  research staffed_hr={:?} first_point_hr={:?} first_node_hr={:?} final_points={:.3} nodes={}",
        summary.first_researcher_hour,
        summary.first_research_point_hour,
        summary.first_research_node_hour,
        summary.final_research_points,
        summary.final_research_nodes
    );
    println!(
        "  fields commissioned_hr={:?} completed_hr={:?} completed_buildings={}",
        summary.first_field_commissioned_hour,
        summary.first_field_completed_hour,
        summary.final_completed_buildings
    );
    println!(
        "  building_lifecycle commissioned={:?} completed={:?}",
        summary.buildings_commissioned, summary.buildings_completed
    );
    println!(
        "  tools produced={:.2} peak={:.2} final={:.2}; items peak={} final={}",
        summary.tools_produced,
        summary.peak_tools,
        summary.final_tools,
        summary.peak_item_count,
        summary.final_item_count
    );
    println!(
        "  blessings god_bank={:.2} legacy_store={:.2} peak_carried={:.2}; tithe={} offering={} delivered={}",
        summary.final_god_blessings,
        summary.final_stored_blessings,
        summary.peak_carried_blessings,
        summary.event_count("tithe"),
        summary.event_count("offering"),
        summary.event_count("blessing_delivered")
    );
    println!(
        "  safety jobs hunt={}/{} completed water={}/{} completed leader_scout={}/{} completed; shrine ritual={}/{} completed offering={}/{} completed ritual_requests={}",
        summary.hunt_jobs_seen,
        summary.hunt_jobs_completed,
        summary.water_jobs_seen,
        summary.water_jobs_completed,
        summary.leader_scout_jobs_seen,
        summary.leader_scout_jobs_completed,
        summary.ritual_jobs_seen,
        summary.ritual_jobs_completed,
        summary.offering_jobs_seen,
        summary.offering_jobs_completed,
        summary.event_count("ritual_ready")
    );
    println!(
        "  elections opened={} resolved={}; traders visits={} trading_windows={}; longest_all-idle_stall={}h",
        summary.event_count("election_opened"),
        summary.event_count("election_resolved"),
        summary.trader_visits,
        summary.trader_trading_windows,
        summary.max_idle_stall_hours
    );
    println!(
        "  fog revealed={} -> {} tiles; shrine deliveries={}",
        summary.initial_revealed_tiles,
        summary.final_revealed_tiles,
        summary.event_count("discovery")
    );
    println!("  event_counts={:?}", summary.event_counts);
    if !summary.reset_reasons.is_empty() {
        println!("  reset_reasons={}", summary.reset_reasons.join(" | "));
    }
    let mut expected: Vec<&str> = expected.iter().map(String::as_str).collect();
    expected.sort_unstable();
    println!("  expectations={}", expected.join(","));
    if flags.is_empty() {
        println!("  PASS: every configured expectation was observed");
    } else {
        println!("  FAILURES:");
        for flag in flags {
            println!("    - {flag}");
        }
    }
}

fn main() {
    let seed = env_u64("SEED", 20_240_712) as u32;
    let hours = env_u64("HOURS", 48);
    let cadence_ms = env_u64("CADENCE_MS", 1_000) as i64;
    let communal = env_bool("COMMUNAL", false);
    let expected = expected_features(hours);
    let idle_stall_limit = env_u64("MAX_IDLE_STALL_HOURS", 8);
    let strict = env_bool("STRICT", true);

    let summary = run_campaign(seed, hours, cadence_ms, communal, true);
    if env_bool("VERIFY_DETERMINISM", true) {
        let repeated = run_campaign(seed, hours, cadence_ms, communal, false);
        assert_eq!(summary, repeated, "same seed and cadence diverged");
        println!("# determinism repeat: identical");
    }
    let flags = summary.anomaly_flags(&expected, idle_stall_limit);
    print_summary(&summary, &expected, &flags);
    if strict && !flags.is_empty() {
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cat_sim::world_tick::VillageScale;

    #[test]
    fn fresh_passive_expectations_do_not_require_vacant_officer_automation() {
        let expected = default_expected_features(144);
        assert!(expected.contains("election"));
        assert!(expected.contains("fog"));
        assert!(expected.contains("hunt"));
        assert!(expected.contains("scout"));
        assert!(expected.contains("water"));
        assert!(expected.contains("trader"));
        for officer_owned in [
            "field",
            "items",
            "offering",
            "raid",
            "research",
            "research_node",
            "tithe",
            "tools",
        ] {
            assert!(!expected.contains(officer_owned));
        }
    }

    #[test]
    fn scale_switch_selects_the_real_founding_blueprints() {
        let personal = found_colony(7, "personal", 1_000, 7);
        let communal = found_global_colony(7, "communal", 1_000, 7);
        assert_eq!(personal.scale, VillageScale::Personal);
        assert_eq!(communal.scale, VillageScale::Communal);
        assert_eq!(
            personal
                .cats
                .iter()
                .filter(|cat| cat.death_time.is_none())
                .count(),
            15
        );
        assert_eq!(
            communal
                .cats
                .iter()
                .filter(|cat| cat.death_time.is_none())
                .count(),
            30
        );
    }
}
