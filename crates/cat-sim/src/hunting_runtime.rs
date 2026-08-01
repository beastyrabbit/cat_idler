//! World-scoped Hunting Lair authority.
//!
//! `EnemyLair` sites own deterministic monster rosters and first-clear state.
//! `CaveEntrance` remains the quarry site. Leader implementations may request an
//! attempt through this module, but roster, safety, combat, rewards, trophies,
//! respawns, and XP remain simulation-owned.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    age::get_life_stage,
    entities::CatActivity,
    hunting_lair::{
        AttemptAuthority, FirstClearTrophy, HuntAdvice, HuntResolution, Hunter, MonsterSpecies,
        SpeciesMaterial, attempt_is_authorized, generate_roster, predicted_success_percent,
        resolve_attempt,
    },
    items::ItemKind,
    skills::Labor,
    types::{TaskType, TileType},
    warriors::combat_stage_factor,
    world_tick::{DeathCause, EventKind, SharedSpatialState, TilePos, WorldState, append_event},
};

const HUNTING_PARTIES_RESEARCH_ID: &str = "hunting_bulk";
const WEAPON_BONUS: f64 = 25.0;
const ARMOR_BONUS: f64 = 25.0;
const HUNTING_REVIEW_GAME_MS: i64 = 6 * 60 * 60 * 1_000;
const MINIMUM_TRAVEL_GAME_MS: i64 = 10 * 60 * 1_000;
const TRAVEL_GAME_MS_PER_TILE: i64 = 2 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveHuntingParty {
    pub id: String,
    pub colony_id: String,
    pub site: TilePos,
    pub member_cat_ids: Vec<String>,
    pub authority: AttemptAuthority,
    pub departed_at_ms: i64,
    pub resolves_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntingTrophyClaim {
    pub colony_id: String,
    pub species: MonsterSpecies,
    pub claimed_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntingOutcomeRecord {
    pub colony_id: String,
    pub site: TilePos,
    pub resolved_at_ms: i64,
    pub resolution: HuntResolution,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptainRecommendation {
    pub predicted_success_percent: u8,
    pub advice: HuntAdvice,
    pub party_size: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HuntingAttemptReport {
    pub site: TilePos,
    pub resolution: HuntResolution,
    pub trophy: Option<HuntingTrophyClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HuntingAttemptError {
    UnknownColony,
    UnrevealedSite,
    NotEnemyLair,
    MissingRoster,
    UnknownOrDeadCat(String),
    UnavailableCat(String),
    DuplicateCat(String),
    EmptyParty,
    PartyTooLarge {
        supplied: usize,
        cap: usize,
    },
    UnsafeAttempt {
        predicted_success_percent: u8,
        advice: HuntAdvice,
    },
    EmptyLair,
}

/// Materialize deterministic rosters for mapped Enemy Lairs, and advance a
/// cleared roster only when its strongest monster's cooldown has elapsed.
pub fn reconcile_hunting_lairs(shared: &mut SharedSpatialState, world_seed: u32, now_ms: i64) {
    let sites = shared
        .tiles
        .iter()
        .filter_map(|(&site, tile)| {
            (tile.tile_type == TileType::EnemyLair)
                .then_some((site, tile.danger_level.round().clamp(0.0, 100.0) as u8))
        })
        .collect::<Vec<_>>();
    for (site, environmental_danger) in sites {
        let site_seed = hunting_site_seed(world_seed, site);
        let lair = shared
            .hunting_lairs
            .entry(site)
            .or_insert_with(|| generate_roster(environmental_danger, site_seed, 0));
        if let Some(respawned) = lair.respawn_if_ready(now_ms, site_seed) {
            *lair = respawned;
        }
    }
    shared.hunting_lairs.retain(|site, _| {
        shared
            .tiles
            .get(site)
            .is_some_and(|tile| tile.tile_type == TileType::EnemyLair)
    });
}

pub fn captain_recommendation(
    world: &WorldState,
    colony_id: &str,
    site: TilePos,
    party_ids: &[String],
) -> Result<CaptainRecommendation, HuntingAttemptError> {
    let colony = world
        .colonies
        .iter()
        .find(|colony| colony.id == colony_id)
        .ok_or(HuntingAttemptError::UnknownColony)?;
    validate_site(world, colony_id, site)?;
    let cap = hunting_party_cap(
        colony
            .upgrade_tree
            .owned_node_ids
            .iter()
            .any(|node_id| node_id == HUNTING_PARTIES_RESEARCH_ID),
    );
    if party_ids.is_empty() {
        return Err(HuntingAttemptError::EmptyParty);
    }
    if party_ids.len() > cap {
        return Err(HuntingAttemptError::PartyTooLarge {
            supplied: party_ids.len(),
            cap,
        });
    }
    let party = assemble_party(colony, party_ids)?;
    let lair = world
        .shared_spatial
        .hunting_lairs
        .get(&site)
        .ok_or(HuntingAttemptError::MissingRoster)?;
    let predicted = predicted_success_percent(lair, &party);
    Ok(CaptainRecommendation {
        predicted_success_percent: predicted,
        advice: HuntAdvice::from_success_percent(predicted),
        party_size: party.len(),
    })
}

/// Deterministic strongest currently living party for Captain advice. This is
/// advisory selection only; dispatch still goes through [`attempt_hunting_lair`].
pub fn recommended_party_ids(
    world: &WorldState,
    colony_id: &str,
    site: TilePos,
) -> Result<Vec<String>, HuntingAttemptError> {
    validate_site(world, colony_id, site)?;
    let colony = world
        .colonies
        .iter()
        .find(|colony| colony.id == colony_id)
        .ok_or(HuntingAttemptError::UnknownColony)?;
    let lair = world
        .shared_spatial
        .hunting_lairs
        .get(&site)
        .ok_or(HuntingAttemptError::MissingRoster)?;
    let cap = hunting_party_cap(
        colony
            .upgrade_tree
            .owned_node_ids
            .iter()
            .any(|node_id| node_id == HUNTING_PARTIES_RESEARCH_ID),
    );
    let mut scored = colony
        .cats
        .iter()
        .filter(|cat| {
            cat.death_time.is_none()
                && combat_stage_factor(get_life_stage(cat.age_hours)) > 0.0
                && cat.activity == CatActivity::Idle
                && cat.current_task.is_none()
                && cat.carrying.is_none()
                && !colony
                    .jobs
                    .iter()
                    .any(|job| job.assigned_cat.as_deref() == Some(cat.id.as_str()))
                && !colony
                    .buildings
                    .iter()
                    .any(|building| building.assigned_cat.as_deref() == Some(cat.id.as_str()))
        })
        .filter_map(|cat| {
            let ids = vec![cat.id.clone()];
            assemble_party(colony, &ids).ok().map(|party| {
                (
                    predicted_success_percent(lair, &party),
                    cat.needs.health.round() as i32,
                    cat.id.clone(),
                )
            })
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| right.1.cmp(&left.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    Ok(scored
        .into_iter()
        .take(cap)
        .map(|(_, _, cat_id)| cat_id)
        .collect())
}

/// Resolve one authoritative encounter. The caller is the Leader adapter or an
/// authenticated player-nudge adapter; neither can override the domain gates.
pub fn attempt_hunting_lair(
    world: &mut WorldState,
    colony_id: &str,
    site: TilePos,
    party_ids: &[String],
    authority: AttemptAuthority,
    now_ms: i64,
) -> Result<HuntingAttemptReport, HuntingAttemptError> {
    validate_site(world, colony_id, site)?;
    let colony_index = world
        .colonies
        .iter()
        .position(|colony| colony.id == colony_id)
        .ok_or(HuntingAttemptError::UnknownColony)?;
    let cap = hunting_party_cap(
        world.colonies[colony_index]
            .upgrade_tree
            .owned_node_ids
            .iter()
            .any(|node_id| node_id == HUNTING_PARTIES_RESEARCH_ID),
    );
    if party_ids.is_empty() {
        return Err(HuntingAttemptError::EmptyParty);
    }
    if party_ids.len() > cap {
        return Err(HuntingAttemptError::PartyTooLarge {
            supplied: party_ids.len(),
            cap,
        });
    }
    let party = assemble_party(&world.colonies[colony_index], party_ids)?;
    let lair = world
        .shared_spatial
        .hunting_lairs
        .get(&site)
        .cloned()
        .ok_or(HuntingAttemptError::MissingRoster)?;
    let predicted = predicted_success_percent(&lair, &party);
    let advice = HuntAdvice::from_success_percent(predicted);
    if !attempt_is_authorized(authority, predicted, &party) {
        return Err(HuntingAttemptError::UnsafeAttempt {
            predicted_success_percent: predicted,
            advice,
        });
    }
    let game_hour_ms = hunting_game_hour_ms(&world.colonies[colony_index]);
    let current_game_hour =
        u64::try_from(now_ms.max(0).saturating_div(game_hour_ms)).unwrap_or(u64::MAX);
    let attempt_nonce = *world
        .shared_spatial
        .hunting_attempt_nonces
        .entry(site)
        .or_default();
    world
        .shared_spatial
        .hunting_attempt_nonces
        .insert(site, attempt_nonce.saturating_add(1));
    let seed = hunting_attempt_seed(
        world.world_seed,
        site,
        lair.generation,
        current_game_hour,
        attempt_nonce,
    );
    let resolution = resolve_attempt(&lair, &party, cap, seed, now_ms, game_hour_ms)
        .map_err(|_| HuntingAttemptError::EmptyLair)?;

    let colony = &mut world.colonies[colony_index];
    let mut deaths = Vec::new();
    for result in &resolution.participants {
        let cat = colony
            .cats
            .iter_mut()
            .find(|cat| cat.id == result.cat_id)
            .expect("validated party remains present during atomic resolution");
        cat.gain_skill(Labor::Hunt, f64::from(result.hunting_xp));
        cat.gain_skill(Labor::Fight, f64::from(result.fight_xp));
        cat.role_xp.hunter += f64::from(result.hunting_xp);
        cat.role_xp.warrior += f64::from(result.fight_xp);
        cat.needs.health = (cat.needs.health - f64::from(result.damage)).max(0.0);
        cat.current_task = None;
        cat.activity = CatActivity::Idle;
        cat.destination = None;
        if result.died {
            deaths.push((cat.id.clone(), cat.name.clone()));
        }
    }
    for result in &resolution.participants {
        crate::world_tick::wear_equipped_item(colony, &result.cat_id, ItemKind::Weapon, now_ms);
        crate::world_tick::wear_equipped_item(colony, &result.cat_id, ItemKind::Armor, now_ms);
    }
    crate::world_tick::credit_hunting_loot(
        colony,
        resolution.loot.food,
        resolution.loot.hide,
        resolution.loot.bone,
        now_ms,
    );
    for (cat_id, name) in deaths {
        crate::world_tick::mark_cat_dead(colony, &cat_id, now_ms);
        append_event(
            colony,
            now_ms,
            EventKind::Death(DeathCause::Hunt),
            format!("{name} was killed while clearing a Hunting Lair."),
        );
    }

    for material in &resolution.loot.species_materials {
        *world
            .shared_spatial
            .hunting_materials
            .entry(colony_id.to_owned())
            .or_default()
            .entry(*material)
            .or_default() += 1;
    }
    let trophy = resolution
        .first_clear_trophy
        .map(|FirstClearTrophy { species }| HuntingTrophyClaim {
            colony_id: colony_id.to_owned(),
            species,
            claimed_at_ms: now_ms,
        });
    if let Some(claim) = &trophy {
        world
            .shared_spatial
            .hunting_trophies
            .entry(site)
            .or_insert_with(|| claim.clone());
    }
    world
        .shared_spatial
        .hunting_lairs
        .insert(site, resolution.lair.clone());
    world
        .shared_spatial
        .recent_hunt_outcomes
        .push(HuntingOutcomeRecord {
            colony_id: colony_id.to_owned(),
            site,
            resolved_at_ms: now_ms,
            resolution: resolution.clone(),
        });
    if world.shared_spatial.recent_hunt_outcomes.len() > 24 {
        let excess = world.shared_spatial.recent_hunt_outcomes.len() - 24;
        world.shared_spatial.recent_hunt_outcomes.drain(0..excess);
    }

    Ok(HuntingAttemptReport {
        site,
        resolution,
        trophy,
    })
}

#[must_use]
pub const fn species_material_darkness_required(material: SpeciesMaterial) -> u8 {
    match material {
        SpeciesMaterial::FoxPelt => 2,
        SpeciesMaterial::BadgerPelt => 3,
        SpeciesMaterial::BearPelt => 5,
        SpeciesMaterial::BeastCore => 8,
    }
}

/// Temporary current-Leader adapter. It turns a safe Leader decision (or a
/// player priority hint) into a reserved, delayed party, while all validation
/// and resolution remain in this module for the replacement AI to call later.
pub fn tick_hunting_lair_adapter(world: &mut WorldState, now_ms: i64) {
    let due_party_ids = world
        .shared_spatial
        .active_hunting_parties
        .iter()
        .filter_map(|(id, party)| (party.resolves_at_ms <= now_ms).then_some(id.clone()))
        .collect::<Vec<_>>();
    for party_id in due_party_ids {
        let Some(party) = world
            .shared_spatial
            .active_hunting_parties
            .remove(&party_id)
        else {
            continue;
        };
        let result = attempt_hunting_lair(
            world,
            &party.colony_id,
            party.site,
            &party.member_cat_ids,
            party.authority,
            party.resolves_at_ms,
        );
        if result.is_err()
            && let Some(colony) = world
                .colonies
                .iter_mut()
                .find(|colony| colony.id == party.colony_id)
        {
            release_party_reservations(colony, &party.member_cat_ids);
        }
    }

    let mut colony_ids = world
        .colonies
        .iter()
        .map(|colony| colony.id.clone())
        .collect::<Vec<_>>();
    colony_ids.sort();
    for colony_id in colony_ids {
        try_start_hunting_party(world, &colony_id, now_ms);
    }
}

fn try_start_hunting_party(world: &mut WorldState, colony_id: &str, now_ms: i64) {
    if world
        .shared_spatial
        .active_hunting_parties
        .values()
        .any(|party| party.colony_id == colony_id)
    {
        return;
    }
    let Some(colony_index) = world
        .colonies
        .iter()
        .position(|colony| colony.id == colony_id)
    else {
        return;
    };
    let colony = &world.colonies[colony_index];
    let has_living_leader = colony.leader_id.as_ref().is_some_and(|leader_id| {
        colony
            .cats
            .iter()
            .any(|cat| cat.id == *leader_id && cat.death_time.is_none())
    });
    if !has_living_leader {
        return;
    }
    let targeted = world.shared_spatial.hunting_nudges.get(colony_id).copied();
    let generally_nudged = world
        .shared_spatial
        .hunting_general_nudges
        .contains(colony_id);
    let player_nudged = targeted.is_some() || generally_nudged;
    let living = colony
        .cats
        .iter()
        .filter(|cat| cat.death_time.is_none())
        .count() as f64;
    let food_is_lean = colony.resources.food + colony.resources.fish < living.max(1.0) * 4.0;
    if !player_nudged && !food_is_lean {
        return;
    }
    if !player_nudged
        && world
            .shared_spatial
            .hunting_next_review_at
            .get(colony_id)
            .is_some_and(|next| *next > now_ms)
    {
        return;
    }

    let mut candidates = world
        .shared_spatial
        .hunting_lairs
        .iter()
        .filter(|(site, lair)| {
            colony.revealed_tiles.contains(site)
                && lair.current_danger() > 0
                && targeted.is_none_or(|target| target == **site)
        })
        .map(|(site, lair)| (*site, lair.current_danger()))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(site, danger)| (*danger, site.x, site.y));
    let Some((site, _)) = candidates.first().copied() else {
        return;
    };
    let Ok(party_ids) = recommended_party_ids(world, colony_id, site) else {
        return;
    };
    if party_ids.is_empty() {
        return;
    }
    let authority = if player_nudged {
        AttemptAuthority::PlayerNudge
    } else {
        AttemptAuthority::AutonomousLeader
    };
    let Ok(recommendation) = captain_recommendation(world, colony_id, site, &party_ids) else {
        return;
    };
    let minimum_health = match authority {
        AttemptAuthority::AutonomousLeader => 70.0,
        AttemptAuthority::PlayerNudge => 80.0,
    };
    let minimum_success = match authority {
        AttemptAuthority::AutonomousLeader => 70,
        AttemptAuthority::PlayerNudge => 45,
    };
    if recommendation.predicted_success_percent < minimum_success
        || party_ids.iter().any(|cat_id| {
            colony
                .cats
                .iter()
                .find(|cat| cat.id == *cat_id)
                .is_none_or(|cat| cat.needs.health < minimum_health)
        })
    {
        return;
    }

    let distance = (site.x - colony.anchor.x)
        .abs()
        .max((site.y - colony.anchor.y).abs());
    let travel_game_ms =
        MINIMUM_TRAVEL_GAME_MS.max(i64::from(distance).saturating_mul(TRAVEL_GAME_MS_PER_TILE));
    let scale = normalized_time_scale(colony);
    let travel_ms = ((travel_game_ms as f64 / scale).ceil().max(1.0)) as i64;
    let resolves_at_ms = now_ms.saturating_add(travel_ms);
    let party_id = format!("hunt:{colony_id}:{}:{}:{now_ms}", site.x, site.y);
    let colony = &mut world.colonies[colony_index];
    for cat_id in &party_ids {
        if let Some(cat) = colony.cats.iter_mut().find(|cat| cat.id == *cat_id) {
            cat.current_task = Some(TaskType::Hunt);
            cat.activity = CatActivity::Traveling;
            cat.destination = None;
        }
    }
    world.shared_spatial.active_hunting_parties.insert(
        party_id.clone(),
        ActiveHuntingParty {
            id: party_id,
            colony_id: colony_id.to_owned(),
            site,
            member_cat_ids: party_ids,
            authority,
            departed_at_ms: now_ms,
            resolves_at_ms,
        },
    );
    world.shared_spatial.hunting_nudges.remove(colony_id);
    world
        .shared_spatial
        .hunting_general_nudges
        .remove(colony_id);
    let review_ms = ((HUNTING_REVIEW_GAME_MS as f64 / normalized_time_scale(colony))
        .ceil()
        .max(1.0)) as i64;
    world
        .shared_spatial
        .hunting_next_review_at
        .insert(colony_id.to_owned(), now_ms.saturating_add(review_ms));
}

fn release_party_reservations(
    colony: &mut crate::world_tick::ColonyRuntime,
    member_cat_ids: &[String],
) {
    for cat_id in member_cat_ids {
        if let Some(cat) = colony.cats.iter_mut().find(|cat| {
            cat.id == *cat_id
                && cat.death_time.is_none()
                && cat.current_task == Some(TaskType::Hunt)
        }) {
            cat.current_task = None;
            cat.activity = CatActivity::Idle;
            cat.destination = None;
        }
    }
}

fn normalized_time_scale(colony: &crate::world_tick::ColonyRuntime) -> f64 {
    if colony.test_time_scale.is_finite() {
        colony.test_time_scale.max(1.0)
    } else {
        1.0
    }
}

fn hunting_game_hour_ms(colony: &crate::world_tick::ColonyRuntime) -> i64 {
    (3_600_000.0 / normalized_time_scale(colony))
        .round()
        .clamp(1.0, i64::MAX as f64) as i64
}

fn validate_site(
    world: &WorldState,
    colony_id: &str,
    site: TilePos,
) -> Result<(), HuntingAttemptError> {
    let colony = world
        .colonies
        .iter()
        .find(|colony| colony.id == colony_id)
        .ok_or(HuntingAttemptError::UnknownColony)?;
    if !colony.revealed_tiles.contains(&site) {
        return Err(HuntingAttemptError::UnrevealedSite);
    }
    if !world
        .shared_spatial
        .tiles
        .get(&site)
        .is_some_and(|tile| tile.tile_type == TileType::EnemyLair)
    {
        return Err(HuntingAttemptError::NotEnemyLair);
    }
    Ok(())
}

fn assemble_party(
    colony: &crate::world_tick::ColonyRuntime,
    party_ids: &[String],
) -> Result<Vec<Hunter>, HuntingAttemptError> {
    let mut seen = BTreeSet::new();
    let mut party = Vec::with_capacity(party_ids.len());
    for cat_id in party_ids {
        if !seen.insert(cat_id.as_str()) {
            return Err(HuntingAttemptError::DuplicateCat(cat_id.clone()));
        }
        let cat = colony
            .cats
            .iter()
            .find(|cat| cat.id == *cat_id && cat.death_time.is_none())
            .ok_or_else(|| HuntingAttemptError::UnknownOrDeadCat(cat_id.clone()))?;
        let reserved_for_this_hunt = cat.current_task == Some(TaskType::Hunt);
        if combat_stage_factor(get_life_stage(cat.age_hours)) <= 0.0
            || (!reserved_for_this_hunt
                && (cat.activity != CatActivity::Idle
                    || cat.current_task.is_some()
                    || cat.carrying.is_some()))
        {
            return Err(HuntingAttemptError::UnavailableCat(cat_id.clone()));
        }
        let has_weapon = has_usable_equipped_item(colony, cat_id, ItemKind::Weapon);
        let has_armor = has_usable_equipped_item(colony, cat_id, ItemKind::Armor);
        let stage = combat_stage_factor(get_life_stage(cat.age_hours));
        let combat_power = (cat.stats.attack * 0.35
            + cat.stats.defense * 0.25
            + cat.stats.hunting * 0.40
            + cat.skill(Labor::Fight) * 0.10
            + cat.skill(Labor::Hunt) * 0.10)
            * stage;
        party.push(Hunter {
            cat_id: cat.id.clone(),
            combat_power,
            health_percent: cat.needs.health.clamp(0.0, 100.0),
            weapon_bonus: if has_weapon { WEAPON_BONUS } else { 0.0 },
            armor_bonus: if has_armor { ARMOR_BONUS } else { 0.0 },
        });
    }
    Ok(party)
}

fn has_usable_equipped_item(
    colony: &crate::world_tick::ColonyRuntime,
    cat_id: &str,
    kind: ItemKind,
) -> bool {
    colony
        .items
        .equipped_id(cat_id, kind)
        .and_then(|id| colony.items.instance(id))
        .is_some_and(|instance| instance.credited && !instance.is_broken())
}

const fn hunting_party_cap(group_researched: bool) -> usize {
    if group_researched { 3 } else { 1 }
}

fn hunting_site_seed(world_seed: u32, site: TilePos) -> u32 {
    world_seed
        ^ (site.x as u32).wrapping_mul(0x9e37_79b9)
        ^ (site.y as u32).rotate_left(16).wrapping_mul(0x85eb_ca6b)
}

fn hunting_attempt_seed(
    world_seed: u32,
    site: TilePos,
    generation: u32,
    current_game_hour: u64,
    attempt_nonce: u64,
) -> u32 {
    hunting_site_seed(world_seed, site)
        .wrapping_add(generation.wrapping_mul(3_000_003))
        .wrapping_add(current_game_hour as u32)
        .wrapping_add((attempt_nonce as u32).wrapping_mul(5_000_011))
}

#[must_use]
pub fn material_inventory(
    shared: &SharedSpatialState,
    colony_id: &str,
) -> BTreeMap<SpeciesMaterial, u32> {
    shared
        .hunting_materials
        .get(colony_id)
        .cloned()
        .unwrap_or_default()
}
