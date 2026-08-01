//! LAI.42 deterministic Hunting Lair authority.
//!
//! This pure leaf consumes LAI.36 creature/material manifest records and
//! LAI.37 quality/lot primitives. It deliberately does not own world-tick
//! routing, protocol projection, persistence adapters, rendering, injuries, or
//! item/material catalogs.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    content_manifest::{
        ContentId, ContentManifest, CreatureDescriptor, CreatureId, CreatureTier,
        LairBandDescriptor, MaterialId, MaterialInstanceId, PLAN1_CREATURE_IDS,
        PLAN1_RARE_MATERIAL_IDS, PhysicalLotId,
    },
    quality_lots::{
        BulkLotKey, LotLocation, LotProvenance, PhysicalLot, ProductionComplexity,
        ProductionQualityInput, QualityBand, gathering_quality_score, quality_from_score,
    },
    rng,
};

pub const HUNTING_LAIR_SCHEMA_VERSION: u32 = 1;
pub const SOLO_PARTY_CAP: usize = 1;
pub const HUNTING_BULK_PARTY_CAP: usize = 3;
pub const GAME_MINUTES_PER_HOUR: u64 = 60;

const AUTONOMOUS_SUCCESS_THRESHOLD: u8 = 70;
const AUTONOMOUS_HEALTH_THRESHOLD: u8 = 70;
const NUDGE_SUCCESS_THRESHOLD: u8 = 45;
const NUDGE_HEALTH_THRESHOLD: u8 = 80;
const ELDER_DRAGON_ID: &str = "elder_dragon";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HuntingError {
    ManifestInvalid(String),
    CreatureCatalogMismatch,
    MaterialCatalogMismatch,
    LairBandMismatch,
    MissingCreature(CreatureId),
    MissingBand(u8),
    MissingEligibleCreature { level: u8, tier: &'static str },
    InvalidLevel(u8),
    InvalidSchemaVersion(u32),
    InvalidSiteKind(HuntingSiteKind),
    EmptySiteId,
    EmptyRoster,
    EmptyParty,
    DuplicateCatId(String),
    InvalidPartyCap(usize),
    PartyTooLarge { supplied: usize, cap: usize },
    InvalidHunter(String),
    UnauthorizedAttempt,
    ArithmeticOverflow,
    InvalidStorage,
    RespawnNotReady,
    MalformedState,
}

impl fmt::Display for HuntingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid hunting-lair operation: {self:?}")
    }
}

impl std::error::Error for HuntingError {}

#[derive(Debug, Clone)]
pub struct HuntingCatalog<'a> {
    manifest: &'a ContentManifest,
    creature_index: BTreeMap<CreatureId, usize>,
}

impl<'a> HuntingCatalog<'a> {
    pub fn from_manifest(manifest: &'a ContentManifest) -> Result<Self, HuntingError> {
        if manifest.creatures.len() != PLAN1_CREATURE_IDS.len()
            || !PLAN1_CREATURE_IDS
                .iter()
                .zip(&manifest.creatures)
                .all(|(expected, actual)| *expected == actual.id.as_str())
        {
            return Err(HuntingError::CreatureCatalogMismatch);
        }
        if manifest.materials.len() != PLAN1_RARE_MATERIAL_IDS.len()
            || !PLAN1_RARE_MATERIAL_IDS.iter().all(|expected| {
                manifest
                    .materials
                    .iter()
                    .any(|material| material.id.as_str() == *expected)
            })
        {
            return Err(HuntingError::MaterialCatalogMismatch);
        }
        validate_lair_bands(&manifest.lair_bands)?;
        manifest
            .validate()
            .map_err(|errors| HuntingError::ManifestInvalid(format!("{errors:?}")))?;

        let material_ids = manifest
            .materials
            .iter()
            .map(|material| material.id.clone())
            .collect::<BTreeSet<_>>();
        for creature in &manifest.creatures {
            if !material_ids.contains(&creature.primary_material) {
                return Err(HuntingError::MaterialCatalogMismatch);
            }
            if creature
                .common_loot
                .iter()
                .any(|loot| loot.content_id.as_str() == "food")
            {
                return Err(HuntingError::CreatureCatalogMismatch);
            }
        }

        let creature_index = manifest
            .creatures
            .iter()
            .enumerate()
            .map(|(index, creature)| (creature.id.clone(), index))
            .collect();

        Ok(Self {
            manifest,
            creature_index,
        })
    }

    #[must_use]
    pub fn embedded() -> Self {
        Self::from_manifest(ContentManifest::embedded())
            .expect("embedded content manifest is the LAI.42 creature authority")
    }

    #[must_use]
    pub fn creatures(&self) -> &[CreatureDescriptor] {
        &self.manifest.creatures
    }

    pub fn creature(&self, id: &CreatureId) -> Result<&CreatureDescriptor, HuntingError> {
        self.creature_index
            .get(id)
            .and_then(|index| self.manifest.creatures.get(*index))
            .ok_or_else(|| HuntingError::MissingCreature(id.clone()))
    }

    fn band_for_level(&self, level: u8) -> Result<&LairBandDescriptor, HuntingError> {
        self.manifest
            .lair_bands
            .iter()
            .find(|band| (band.band_min..=band.band_max).contains(&level))
            .ok_or(HuntingError::MissingBand(level))
    }

    fn eligible_creatures(
        &self,
        level: u8,
        filter: impl Fn(CreatureTier) -> bool,
    ) -> Vec<&CreatureDescriptor> {
        self.manifest
            .creatures
            .iter()
            .filter(|creature| creature.level_min <= level && filter(creature.tier))
            .collect()
    }
}

fn validate_lair_bands(bands: &[LairBandDescriptor]) -> Result<(), HuntingError> {
    let expected = [
        (1, 19, None, true, 1, 1),
        (20, 39, None, true, 1, 2),
        (40, 59, None, true, 2, 2),
        (60, 79, Some(61), true, 2, 3),
        (80, 94, Some(80), true, 3, 3),
        (95, 100, Some(95), true, 3, 3),
    ];
    if bands.len() != expected.len()
        || !bands.iter().zip(expected).all(|(band, expected)| {
            (
                band.band_min,
                band.band_max,
                band.mystic_required_from_level,
                band.normal_allowed,
                band.min_roster_size,
                band.max_roster_size,
            ) == expected
        })
    {
        return Err(HuntingError::LairBandMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuntingSiteKind {
    EnemyLair,
    CaveEntrance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TileCoord {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RosterEntry {
    pub slot: u8,
    pub creature_id: CreatureId,
    pub actual_level: u8,
    pub health: u16,
}

impl RosterEntry {
    #[must_use]
    pub const fn is_alive(&self) -> bool {
        self.health > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuntingLairState {
    pub site_id: String,
    pub site_kind: HuntingSiteKind,
    pub tile: TileCoord,
    pub level: u8,
    pub generation: u32,
    pub clear_index: u32,
    pub roster: Vec<RosterEntry>,
    pub first_clear_claimed: bool,
    pub respawn_ready_game_minute: Option<u64>,
    pub cache_lot_ids: Vec<PhysicalLotId>,
    pub cache_material_instance_ids: Vec<MaterialInstanceId>,
}

impl HuntingLairState {
    pub fn new_enemy_lair(
        catalog: &HuntingCatalog<'_>,
        world_seed: u32,
        site_id: impl Into<String>,
        tile: TileCoord,
        level: u8,
    ) -> Result<Self, HuntingError> {
        let site_id = site_id.into();
        validate_site(&site_id, HuntingSiteKind::EnemyLair)?;
        validate_level(level)?;
        let roster = generate_roster(catalog, world_seed, &site_id, 0, level)?;
        Ok(Self {
            site_id,
            site_kind: HuntingSiteKind::EnemyLair,
            tile,
            level,
            generation: 0,
            clear_index: 0,
            roster,
            first_clear_claimed: false,
            respawn_ready_game_minute: None,
            cache_lot_ids: Vec::new(),
            cache_material_instance_ids: Vec::new(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        site_id: impl Into<String>,
        site_kind: HuntingSiteKind,
        tile: TileCoord,
        level: u8,
        generation: u32,
        clear_index: u32,
        roster: Vec<RosterEntry>,
        first_clear_claimed: bool,
        respawn_ready_game_minute: Option<u64>,
        cache_lot_ids: Vec<PhysicalLotId>,
        cache_material_instance_ids: Vec<MaterialInstanceId>,
    ) -> Result<Self, HuntingError> {
        let site_id = site_id.into();
        validate_site(&site_id, site_kind)?;
        validate_level(level)?;
        let state = Self {
            site_id,
            site_kind,
            tile,
            level,
            generation,
            clear_index,
            roster,
            first_clear_claimed,
            respawn_ready_game_minute,
            cache_lot_ids,
            cache_material_instance_ids,
        };
        state.validate_persisted_state(HuntingCatalog::embedded())?;
        Ok(state)
    }

    pub fn living_danger(&self, catalog: &HuntingCatalog<'_>) -> Result<u16, HuntingError> {
        self.roster
            .iter()
            .filter(|entry| entry.is_alive())
            .try_fold(0_u16, |danger, entry| {
                let creature = catalog.creature(&entry.creature_id)?;
                danger
                    .checked_add(creature.stats.danger)
                    .ok_or(HuntingError::ArithmeticOverflow)
            })
    }

    pub fn strongest_living_entry(
        &self,
        catalog: &HuntingCatalog<'_>,
    ) -> Result<Option<&RosterEntry>, HuntingError> {
        let mut strongest: Option<(&RosterEntry, (u16, u8, &str))> = None;
        for entry in self.roster.iter().filter(|entry| entry.is_alive()) {
            let creature = catalog.creature(&entry.creature_id)?;
            let candidate = (
                creature.stats.danger,
                entry.actual_level,
                entry.creature_id.as_str(),
            );
            let replace = strongest
                .map(|(_, current)| candidate > current)
                .unwrap_or(true);
            if replace {
                strongest = Some((entry, candidate));
            }
        }
        Ok(strongest.map(|(entry, _)| entry))
    }

    pub fn respawn_if_ready(
        &self,
        catalog: &HuntingCatalog<'_>,
        world_seed: u32,
        now_game_minute: u64,
    ) -> Result<Option<Self>, HuntingError> {
        let Some(ready_at) = self.respawn_ready_game_minute else {
            return Ok(None);
        };
        if !self.roster.is_empty() || now_game_minute < ready_at {
            return Ok(None);
        }
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(HuntingError::ArithmeticOverflow)?;
        let mut respawned = self.clone();
        respawned.generation = generation;
        respawned.roster =
            generate_roster(catalog, world_seed, &self.site_id, generation, self.level)?;
        respawned.respawn_ready_game_minute = None;
        Ok(Some(respawned))
    }

    fn validate_persisted_state(&self, catalog: HuntingCatalog<'_>) -> Result<(), HuntingError> {
        if self.roster.is_empty() != self.respawn_ready_game_minute.is_some()
            || (self.roster.is_empty() && !self.first_clear_claimed)
            || self.roster.len() > HUNTING_BULK_PARTY_CAP
        {
            return Err(HuntingError::MalformedState);
        }

        let mut slots = BTreeSet::new();
        for entry in &self.roster {
            let creature = catalog.creature(&entry.creature_id)?;
            if !slots.insert(entry.slot)
                || usize::from(entry.slot) >= self.roster.len()
                || entry.actual_level != self.level.clamp(creature.level_min, creature.level_max)
                || entry.health == 0
                || entry.health > creature.stats.danger
            {
                return Err(HuntingError::MalformedState);
            }
        }
        if slots.iter().copied().ne(0..self.roster.len() as u8)
            || has_duplicates(&self.cache_lot_ids)
            || has_duplicates(&self.cache_material_instance_ids)
        {
            return Err(HuntingError::MalformedState);
        }
        Ok(())
    }
}

fn has_duplicates<T: Ord + Clone>(values: &[T]) -> bool {
    let mut seen = BTreeSet::new();
    values.iter().any(|value| !seen.insert(value.clone()))
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HuntingLairStateWire {
    schema_version: u32,
    site_id: String,
    site_kind: HuntingSiteKind,
    tile: TileCoord,
    level: u8,
    generation: u32,
    clear_index: u32,
    roster: Vec<RosterEntry>,
    first_clear_claimed: bool,
    respawn_ready_game_minute: Option<u64>,
    cache_lot_ids: Vec<PhysicalLotId>,
    cache_material_instance_ids: Vec<MaterialInstanceId>,
}

impl Serialize for HuntingLairState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        HuntingLairStateWire {
            schema_version: HUNTING_LAIR_SCHEMA_VERSION,
            site_id: self.site_id.clone(),
            site_kind: self.site_kind,
            tile: self.tile,
            level: self.level,
            generation: self.generation,
            clear_index: self.clear_index,
            roster: self.roster.clone(),
            first_clear_claimed: self.first_clear_claimed,
            respawn_ready_game_minute: self.respawn_ready_game_minute,
            cache_lot_ids: self.cache_lot_ids.clone(),
            cache_material_instance_ids: self.cache_material_instance_ids.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HuntingLairState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = HuntingLairStateWire::deserialize(deserializer)?;
        if wire.schema_version != HUNTING_LAIR_SCHEMA_VERSION {
            return Err(de::Error::custom(HuntingError::InvalidSchemaVersion(
                wire.schema_version,
            )));
        }
        Self::from_parts(
            wire.site_id,
            wire.site_kind,
            wire.tile,
            wire.level,
            wire.generation,
            wire.clear_index,
            wire.roster,
            wire.first_clear_claimed,
            wire.respawn_ready_game_minute,
            wire.cache_lot_ids,
            wire.cache_material_instance_ids,
        )
        .map_err(de::Error::custom)
    }
}

fn validate_site(site_id: &str, site_kind: HuntingSiteKind) -> Result<(), HuntingError> {
    if site_id.trim().is_empty() {
        return Err(HuntingError::EmptySiteId);
    }
    if site_kind != HuntingSiteKind::EnemyLair {
        return Err(HuntingError::InvalidSiteKind(site_kind));
    }
    Ok(())
}

fn validate_level(level: u8) -> Result<(), HuntingError> {
    if (1..=100).contains(&level) {
        Ok(())
    } else {
        Err(HuntingError::InvalidLevel(level))
    }
}

pub fn generate_roster(
    catalog: &HuntingCatalog<'_>,
    world_seed: u32,
    site_id: &str,
    generation: u32,
    level: u8,
) -> Result<Vec<RosterEntry>, HuntingError> {
    validate_level(level)?;
    let band = catalog.band_for_level(level)?;
    let size = if band.min_roster_size == band.max_roster_size {
        band.min_roster_size
    } else {
        let span = band.max_roster_size - band.min_roster_size + 1;
        band.min_roster_size
            + keyed_index(
                world_seed,
                site_id,
                generation,
                level,
                "roster_size",
                0,
                span as usize,
            ) as u8
    };

    if level >= 95 {
        return generate_boss_roster(catalog, world_seed, site_id, generation, level);
    }

    let mut roster = Vec::with_capacity(size as usize);
    for slot in 0..size {
        let pool = if level <= 39 {
            catalog.eligible_creatures(level, |tier| tier == CreatureTier::Normal)
        } else {
            catalog.eligible_creatures(level, |tier| tier != CreatureTier::Boss)
        };
        let creature = choose_creature(
            &pool,
            world_seed,
            site_id,
            generation,
            level,
            "roster_creature",
            slot,
        )?;
        roster.push(roster_entry(slot, level, creature));
    }

    if band
        .mystic_required_from_level
        .is_some_and(|threshold| level >= threshold)
        && !roster_has_tier(catalog, &roster, CreatureTier::Mystic)?
    {
        let mystics = catalog.eligible_creatures(level, |tier| tier == CreatureTier::Mystic);
        let replacement = choose_creature(
            &mystics,
            world_seed,
            site_id,
            generation,
            level,
            "mandatory_mystic",
            size.saturating_sub(1),
        )?;
        if let Some(last) = roster.last_mut() {
            *last = roster_entry(last.slot, level, replacement);
        }
    }

    Ok(roster)
}

fn generate_boss_roster(
    catalog: &HuntingCatalog<'_>,
    world_seed: u32,
    site_id: &str,
    generation: u32,
    level: u8,
) -> Result<Vec<RosterEntry>, HuntingError> {
    let boss_id =
        CreatureId::new(ELDER_DRAGON_ID).map_err(|_| HuntingError::CreatureCatalogMismatch)?;
    let boss = catalog.creature(&boss_id)?;
    let supporters = catalog.eligible_creatures(level, |tier| tier != CreatureTier::Boss);
    if supporters.is_empty() {
        return Err(HuntingError::MissingEligibleCreature {
            level,
            tier: "non_boss",
        });
    }
    let mut roster = vec![roster_entry(0, level, boss)];
    for slot in 1..=2 {
        let supporter = choose_creature(
            &supporters,
            world_seed,
            site_id,
            generation,
            level,
            "boss_supporter",
            slot,
        )?;
        roster.push(roster_entry(slot, level, supporter));
    }
    Ok(roster)
}

fn roster_has_tier(
    catalog: &HuntingCatalog<'_>,
    roster: &[RosterEntry],
    tier: CreatureTier,
) -> Result<bool, HuntingError> {
    for entry in roster {
        if catalog.creature(&entry.creature_id)?.tier == tier {
            return Ok(true);
        }
    }
    Ok(false)
}

fn choose_creature<'a>(
    pool: &[&'a CreatureDescriptor],
    world_seed: u32,
    site_id: &str,
    generation: u32,
    level: u8,
    suffix: &'static str,
    slot: u8,
) -> Result<&'a CreatureDescriptor, HuntingError> {
    if pool.is_empty() {
        return Err(HuntingError::MissingEligibleCreature {
            level,
            tier: suffix,
        });
    }
    let index = keyed_index(
        world_seed,
        site_id,
        generation,
        level,
        suffix,
        slot,
        pool.len(),
    );
    Ok(pool[index])
}

fn roster_entry(slot: u8, lair_level: u8, creature: &CreatureDescriptor) -> RosterEntry {
    RosterEntry {
        slot,
        creature_id: creature.id.clone(),
        actual_level: lair_level.clamp(creature.level_min, creature.level_max),
        health: creature.stats.danger,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptAuthority {
    AutonomousLeader,
    PlayerNudge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptGate {
    Denied,
    CombatAuthorized,
    ReviewAuthorized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuntAdvice {
    Reckless,
    Risky,
    Favored,
    Safe,
}

impl HuntAdvice {
    #[must_use]
    pub const fn from_success_percent(success_percent: u8) -> Self {
        match success_percent {
            0..=49 => Self::Reckless,
            50..=69 => Self::Risky,
            70..=89 => Self::Favored,
            _ => Self::Safe,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquippedItemKind {
    Weapon,
    Armor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "id")]
pub enum EquipmentLocation {
    Equipped(String),
    Stockpile(String),
    Cargo(String),
    Cache(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EquippedItem {
    pub item_instance_id: MaterialInstanceId,
    pub kind: EquippedItemKind,
    pub resolved_effect: u16,
    pub durability: u32,
    pub reserved: bool,
    pub location: EquipmentLocation,
    pub usable: bool,
}

impl EquippedItem {
    #[must_use]
    pub fn is_eligible_for(&self, cat_id: &str, expected_kind: EquippedItemKind) -> bool {
        self.kind == expected_kind
            && self.usable
            && self.durability > 0
            && !self.reserved
            && matches!(&self.location, EquipmentLocation::Equipped(owner) if owner == cat_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HunterInput {
    pub cat_id: String,
    pub combat_power: u16,
    pub health_percent: u8,
    pub weapon: Option<EquippedItem>,
    pub armor: Option<EquippedItem>,
}

impl HunterInput {
    fn validate(&self) -> Result<(), HuntingError> {
        if self.cat_id.trim().is_empty() || self.health_percent > 100 {
            return Err(HuntingError::InvalidHunter(self.cat_id.clone()));
        }
        Ok(())
    }

    #[must_use]
    pub fn weapon_effect(&self) -> u16 {
        self.weapon
            .as_ref()
            .filter(|item| item.is_eligible_for(&self.cat_id, EquippedItemKind::Weapon))
            .map_or(0, |item| item.resolved_effect)
    }

    #[must_use]
    pub fn armor_effect(&self) -> u16 {
        self.armor
            .as_ref()
            .filter(|item| item.is_eligible_for(&self.cat_id, EquippedItemKind::Armor))
            .map_or(0, |item| item.resolved_effect)
    }

    #[must_use]
    pub fn effective_power(&self) -> u16 {
        self.combat_power
            .saturating_add(self.weapon_effect())
            .saturating_add(self.armor_effect() / 2)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WearIntent {
    pub item_instance_id: MaterialInstanceId,
    pub from_durability: u32,
    pub to_durability: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParticipantResult {
    pub cat_id: String,
    pub damage: u8,
    pub died: bool,
    pub hunting_xp: u16,
    pub fight_xp: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamedDropInstance {
    pub instance_id: MaterialInstanceId,
    pub material_id: MaterialId,
    pub quality: QualityBand,
    pub provenance: LotProvenance,
    pub location: LotLocation,
    pub reservation: Option<String>,
    pub creature_id: CreatureId,
    pub clear_index: u32,
    pub guaranteed_first_clear: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HuntOutputs {
    pub common_lots: Vec<PhysicalLot>,
    pub named_drops: Vec<NamedDropInstance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HuntResolution {
    pub lair: HuntingLairState,
    pub cleared: bool,
    pub predicted_success_percent: u8,
    pub advice: HuntAdvice,
    pub combat_roll_percent: u8,
    pub participants: Vec<ParticipantResult>,
    pub wear_intents: Vec<WearIntent>,
    pub outputs: HuntOutputs,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GatheringQualityRequest {
    pub source_quality: QualityBand,
    pub lead_skill: u8,
    pub tool_quality: Option<QualityBand>,
    pub fixture_quality: Option<QualityBand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StoragePlacement {
    pub stockpile_id: String,
    pub capacity_units: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HuntAttemptRequest {
    pub world_seed: u32,
    pub now_game_minute: u64,
    pub has_hunting_bulk: bool,
    pub party: Vec<HunterInput>,
    pub quality: GatheringQualityRequest,
    pub storage: StoragePlacement,
}

#[must_use]
pub const fn party_cap(has_hunting_bulk: bool) -> usize {
    if has_hunting_bulk {
        HUNTING_BULK_PARTY_CAP
    } else {
        SOLO_PARTY_CAP
    }
}

pub fn sorted_party(mut party: Vec<HunterInput>) -> Result<Vec<HunterInput>, HuntingError> {
    for hunter in &party {
        hunter.validate()?;
    }
    party.sort_by(|left, right| left.cat_id.cmp(&right.cat_id));
    for pair in party.windows(2) {
        if pair[0].cat_id == pair[1].cat_id {
            return Err(HuntingError::DuplicateCatId(pair[0].cat_id.clone()));
        }
    }
    Ok(party)
}

pub fn predicted_success_percent(
    catalog: &HuntingCatalog<'_>,
    lair: &HuntingLairState,
    party: &[HunterInput],
) -> Result<u8, HuntingError> {
    let party_power = party.iter().try_fold(0_i32, |sum, hunter| {
        sum.checked_add(i32::from(hunter.effective_power()))
            .ok_or(HuntingError::ArithmeticOverflow)
    })?;
    let danger = i32::from(lair.living_danger(catalog)?);
    Ok((50_i32 + party_power - danger).clamp(5, 95) as u8)
}

pub fn attempt_gate(
    authority: AttemptAuthority,
    predicted_success_percent: u8,
    party: &[HunterInput],
) -> Result<AttemptGate, HuntingError> {
    let party = sorted_party(party.to_vec())?;
    if party.is_empty() {
        return Ok(AttemptGate::Denied);
    }
    let (minimum_success, minimum_health) = match authority {
        AttemptAuthority::AutonomousLeader => {
            (AUTONOMOUS_SUCCESS_THRESHOLD, AUTONOMOUS_HEALTH_THRESHOLD)
        }
        AttemptAuthority::PlayerNudge => (NUDGE_SUCCESS_THRESHOLD, NUDGE_HEALTH_THRESHOLD),
    };
    if predicted_success_percent < minimum_success
        || party
            .iter()
            .any(|hunter| hunter.health_percent < minimum_health)
    {
        return Ok(AttemptGate::Denied);
    }
    Ok(match authority {
        AttemptAuthority::AutonomousLeader => AttemptGate::CombatAuthorized,
        AttemptAuthority::PlayerNudge => AttemptGate::ReviewAuthorized,
    })
}

pub fn resolve_attempt(
    catalog: &HuntingCatalog<'_>,
    lair: &HuntingLairState,
    request: HuntAttemptRequest,
) -> Result<HuntResolution, HuntingError> {
    validate_site(&lair.site_id, lair.site_kind)?;
    let cap = party_cap(request.has_hunting_bulk);
    if !matches!(cap, SOLO_PARTY_CAP | HUNTING_BULK_PARTY_CAP) {
        return Err(HuntingError::InvalidPartyCap(cap));
    }
    let party = sorted_party(request.party)?;
    if party.is_empty() {
        return Err(HuntingError::EmptyParty);
    }
    if party.len() > cap {
        return Err(HuntingError::PartyTooLarge {
            supplied: party.len(),
            cap,
        });
    }
    if request.storage.stockpile_id.trim().is_empty() {
        return Err(HuntingError::InvalidStorage);
    }
    let danger = lair.living_danger(catalog)?;
    if danger == 0 {
        return Err(HuntingError::EmptyRoster);
    }

    let success_percent = predicted_success_percent(catalog, lair, &party)?;
    let roll_percent = keyed_percent(
        request.world_seed,
        &lair.site_id,
        lair.generation,
        lair.level,
        lair.clear_index,
        &sorted_party_key(&party),
        "combat",
    );
    let won = roll_percent < success_percent;
    let wear_intents = wear_intents(&party);
    let participants = participant_results(won, danger, &party)?;

    if !won {
        return Ok(HuntResolution {
            lair: lair.clone(),
            cleared: false,
            predicted_success_percent: success_percent,
            advice: HuntAdvice::from_success_percent(success_percent),
            combat_roll_percent: roll_percent,
            participants,
            wear_intents,
            outputs: HuntOutputs::default(),
        });
    }

    let mut capacity = request.storage.capacity_units;
    let mut outputs = HuntOutputs::default();
    let mut cache_lot_ids = lair.cache_lot_ids.clone();
    let mut cache_material_instance_ids = lair.cache_material_instance_ids.clone();
    let mut output_index = 0_u32;

    for entry in lair.roster.iter().filter(|entry| entry.is_alive()) {
        let creature = catalog.creature(&entry.creature_id)?;
        for loot in &creature.common_loot {
            let quality = common_loot_quality(
                request.world_seed,
                lair,
                entry,
                &loot.content_id,
                &request.quality,
                output_index,
            )?;
            place_common_lot(
                &mut outputs.common_lots,
                &mut cache_lot_ids,
                &mut capacity,
                &request.storage.stockpile_id,
                &lair.site_id,
                lair.generation,
                lair.clear_index,
                output_index,
                loot.content_id.clone(),
                quality,
                loot.units,
                request.now_game_minute,
            )?;
            output_index = output_index
                .checked_add(1)
                .ok_or(HuntingError::ArithmeticOverflow)?;
        }
    }

    let ordinary_named_count = add_ordinary_named_drops(
        catalog,
        lair,
        request.world_seed,
        request.now_game_minute,
        &request.storage.stockpile_id,
        &mut capacity,
        &mut cache_material_instance_ids,
        &mut outputs.named_drops,
    )?;
    if ordinary_named_count == 0 && !lair.first_clear_claimed {
        let strongest = lair
            .strongest_living_entry(catalog)?
            .ok_or(HuntingError::EmptyRoster)?;
        let creature = catalog.creature(&strongest.creature_id)?;
        let quality = rare_quality_floor(lair.level);
        let location = take_location(
            &request.storage.stockpile_id,
            &lair.site_id,
            &mut capacity,
            &mut cache_material_instance_ids,
            true,
            make_material_instance_id(
                &lair.site_id,
                lair.generation,
                lair.clear_index,
                strongest.slot,
                "first_clear",
            )?,
        );
        outputs.named_drops.push(NamedDropInstance {
            instance_id: location.id,
            material_id: creature.primary_material.clone(),
            quality,
            provenance: LotProvenance {
                origin: provenance_origin(&lair.site_id, &creature.id, lair.clear_index),
                created_tick: request.now_game_minute,
            },
            location: location.location,
            reservation: None,
            creature_id: creature.id.clone(),
            clear_index: lair.clear_index,
            guaranteed_first_clear: true,
        });
    }

    let mut cleared = lair.clone();
    cleared.roster.clear();
    cleared.clear_index = cleared
        .clear_index
        .checked_add(1)
        .ok_or(HuntingError::ArithmeticOverflow)?;
    cleared.first_clear_claimed = true;
    let respawn_minutes = respawn_hours(lair.level)
        .checked_mul(GAME_MINUTES_PER_HOUR)
        .ok_or(HuntingError::ArithmeticOverflow)?;
    cleared.respawn_ready_game_minute = Some(
        request
            .now_game_minute
            .checked_add(respawn_minutes)
            .ok_or(HuntingError::ArithmeticOverflow)?,
    );
    cleared.cache_lot_ids = cache_lot_ids;
    cleared.cache_material_instance_ids = cache_material_instance_ids;

    Ok(HuntResolution {
        lair: cleared,
        cleared: true,
        predicted_success_percent: success_percent,
        advice: HuntAdvice::from_success_percent(success_percent),
        combat_roll_percent: roll_percent,
        participants,
        wear_intents,
        outputs,
    })
}

fn participant_results(
    won: bool,
    danger: u16,
    party: &[HunterInput],
) -> Result<Vec<ParticipantResult>, HuntingError> {
    let hunting_xp = if won {
        4 + danger / 20
    } else {
        1 + danger / 50
    };
    let fight_xp = if won {
        3 + danger / 25
    } else {
        1 + danger / 50
    };
    let total_power = party.iter().try_fold(0_i64, |sum, hunter| {
        sum.checked_add(i64::from(hunter.effective_power()))
            .ok_or(HuntingError::ArithmeticOverflow)
    })?;
    let party_len = i64::try_from(party.len()).map_err(|_| HuntingError::ArithmeticOverflow)?;
    party
        .iter()
        .map(|hunter| {
            let damage = if won {
                0
            } else {
                // Fixed-point equivalent of the protected source formula:
                // round(20 + max(danger - average party power, 0) - armor).
                let danger_gap_numerator = (i64::from(danger) * party_len - total_power).max(0);
                let damage_numerator =
                    (20_i64 - i64::from(hunter.armor_effect())) * party_len + danger_gap_numerator;
                if damage_numerator <= 10 * party_len {
                    10
                } else if damage_numerator >= 90 * party_len {
                    90
                } else {
                    u8::try_from((damage_numerator + party_len / 2) / party_len)
                        .map_err(|_| HuntingError::ArithmeticOverflow)?
                }
            };
            Ok(ParticipantResult {
                cat_id: hunter.cat_id.clone(),
                damage,
                died: damage >= hunter.health_percent,
                hunting_xp,
                fight_xp,
            })
        })
        .collect()
}

fn wear_intents(party: &[HunterInput]) -> Vec<WearIntent> {
    let mut intents = Vec::new();
    for hunter in party {
        for item in [&hunter.weapon, &hunter.armor].into_iter().flatten() {
            if item.is_eligible_for(&hunter.cat_id, item.kind) {
                intents.push(WearIntent {
                    item_instance_id: item.item_instance_id.clone(),
                    from_durability: item.durability,
                    to_durability: item.durability.saturating_sub(1),
                });
            }
        }
    }
    intents.sort_by(|left, right| left.item_instance_id.cmp(&right.item_instance_id));
    intents
}

fn common_loot_quality(
    world_seed: u32,
    lair: &HuntingLairState,
    entry: &RosterEntry,
    content_id: &ContentId,
    quality: &GatheringQualityRequest,
    output_index: u32,
) -> Result<QualityBand, HuntingError> {
    let variation = keyed_variation(
        world_seed,
        &lair.site_id,
        lair.generation,
        lair.level,
        lair.clear_index,
        content_id.as_str(),
        "common_quality",
        entry.slot,
        output_index,
    );
    let score = gathering_quality_score(
        ProductionQualityInput {
            weighted_input_quality_milli: 0,
            worker_skill: quality.lead_skill,
            tool_quality: quality.tool_quality,
            fixture_quality: quality.fixture_quality,
            station_tier: 1,
            complexity: ProductionComplexity::Raw,
            keyed_variation: variation,
        },
        quality.source_quality,
    )
    .map_err(|_| HuntingError::ArithmeticOverflow)?;
    Ok(quality_from_score(score))
}

#[allow(clippy::too_many_arguments)]
fn place_common_lot(
    lots: &mut Vec<PhysicalLot>,
    cache_lot_ids: &mut Vec<PhysicalLotId>,
    capacity: &mut u32,
    stockpile_id: &str,
    site_id: &str,
    generation: u32,
    clear_index: u32,
    output_index: u32,
    content_id: ContentId,
    quality: QualityBand,
    quantity: u32,
    now_game_minute: u64,
) -> Result<(), HuntingError> {
    if quantity == 0 {
        return Ok(());
    }
    let stock_quantity = quantity.min(*capacity);
    let cache_quantity = quantity - stock_quantity;
    if stock_quantity > 0 {
        let id = make_lot_id(site_id, generation, clear_index, output_index, "stock")?;
        lots.push(PhysicalLot {
            id,
            key: BulkLotKey::new(content_id.clone(), quality),
            provenance: LotProvenance {
                origin: format!("enemy_lair:{site_id}:clear:{clear_index}"),
                created_tick: now_game_minute,
            },
            quantity: stock_quantity,
            location: LotLocation::Stockpile(stockpile_id.to_owned()),
            reservation: None,
        });
        *capacity -= stock_quantity;
    }
    if cache_quantity > 0 {
        let id = make_lot_id(site_id, generation, clear_index, output_index, "cache")?;
        cache_lot_ids.push(id.clone());
        lots.push(PhysicalLot {
            id,
            key: BulkLotKey::new(content_id, quality),
            provenance: LotProvenance {
                origin: format!("enemy_lair:{site_id}:clear:{clear_index}"),
                created_tick: now_game_minute,
            },
            quantity: cache_quantity,
            location: LotLocation::Cache(site_id.to_owned()),
            reservation: None,
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_ordinary_named_drops(
    catalog: &HuntingCatalog<'_>,
    lair: &HuntingLairState,
    world_seed: u32,
    now_game_minute: u64,
    stockpile_id: &str,
    capacity: &mut u32,
    cache_material_instance_ids: &mut Vec<MaterialInstanceId>,
    named_drops: &mut Vec<NamedDropInstance>,
) -> Result<usize, HuntingError> {
    let mut count = 0;
    for entry in lair.roster.iter().filter(|entry| entry.is_alive()) {
        let creature = catalog.creature(&entry.creature_id)?;
        let roll = named_drop_roll_percent(
            world_seed,
            &lair.site_id,
            lair.generation,
            &creature.id,
            lair.clear_index,
        );
        if roll >= named_drop_percent(lair.level) {
            continue;
        }
        let quality = rare_quality_roll(world_seed, lair, creature)?;
        let instance_id = make_material_instance_id(
            &lair.site_id,
            lair.generation,
            lair.clear_index,
            entry.slot,
            "named",
        )?;
        let location = take_location(
            stockpile_id,
            &lair.site_id,
            capacity,
            cache_material_instance_ids,
            true,
            instance_id,
        );
        named_drops.push(NamedDropInstance {
            instance_id: location.id,
            material_id: creature.primary_material.clone(),
            quality,
            provenance: LotProvenance {
                origin: provenance_origin(&lair.site_id, &creature.id, lair.clear_index),
                created_tick: now_game_minute,
            },
            location: location.location,
            reservation: None,
            creature_id: creature.id.clone(),
            clear_index: lair.clear_index,
            guaranteed_first_clear: false,
        });
        count += 1;
    }
    Ok(count)
}

struct PlacedInstance {
    id: MaterialInstanceId,
    location: LotLocation,
}

fn take_location(
    stockpile_id: &str,
    site_id: &str,
    capacity: &mut u32,
    cache_material_instance_ids: &mut Vec<MaterialInstanceId>,
    use_capacity: bool,
    id: MaterialInstanceId,
) -> PlacedInstance {
    let location = if use_capacity && *capacity > 0 {
        *capacity -= 1;
        LotLocation::Stockpile(stockpile_id.to_owned())
    } else {
        cache_material_instance_ids.push(id.clone());
        LotLocation::Cache(site_id.to_owned())
    };
    PlacedInstance { id, location }
}

fn rare_quality_roll(
    world_seed: u32,
    lair: &HuntingLairState,
    creature: &CreatureDescriptor,
) -> Result<QualityBand, HuntingError> {
    let (floor, ceiling) = rare_quality_range(lair.level);
    if floor == ceiling {
        return Ok(floor);
    }
    let span = ceiling.ordinal() - floor.ordinal() + 1;
    let offset = (lcg_roll(
        world_seed,
        &[&lair.site_id, creature.id.as_str(), "named_drop_quality"],
        &[lair.generation, lair.clear_index],
    ) % u32::from(span)) as u8;
    QualityBand::from_ordinal(floor.ordinal() + offset)
        .map_err(|_| HuntingError::ArithmeticOverflow)
}

#[must_use]
pub const fn rare_quality_floor(level: u8) -> QualityBand {
    rare_quality_range(level).0
}

#[must_use]
pub const fn rare_quality_range(level: u8) -> (QualityBand, QualityBand) {
    match level {
        0..=24 => (QualityBand::Crude, QualityBand::Crude),
        25..=49 => (QualityBand::Crude, QualityBand::Common),
        50..=69 => (QualityBand::Common, QualityBand::Fine),
        70..=84 => (QualityBand::Fine, QualityBand::Superior),
        85..=94 => (QualityBand::Superior, QualityBand::Masterwork),
        _ => (QualityBand::Masterwork, QualityBand::Masterwork),
    }
}

#[must_use]
pub const fn named_drop_percent(level: u8) -> u8 {
    match level {
        0..=24 => 10,
        25..=49 => 15,
        50..=69 => 20,
        70..=84 => 25,
        85..=94 => 30,
        _ => 40,
    }
}

/// Exact P1.28 named-drop key:
/// world seed + Lair ID + generation + creature ID + clear index.
///
/// The fixed semantic suffix separates this roll from the independently keyed
/// quality roll without admitting level, time, roster slot, or runtime nonce.
#[must_use]
pub fn named_drop_roll_percent(
    world_seed: u32,
    site_id: &str,
    generation: u32,
    creature_id: &CreatureId,
    clear_index: u32,
) -> u8 {
    (lcg_roll(
        world_seed,
        &[site_id, creature_id.as_str(), "named_drop"],
        &[generation, clear_index],
    ) % 100) as u8
}

#[must_use]
pub const fn respawn_hours(level: u8) -> u64 {
    match level {
        0..=19 => 6,
        20..=39 => 8,
        40..=59 => 12,
        60..=79 => 14,
        80..=94 => 18,
        _ => 24,
    }
}

pub fn recover_outputs(mut outputs: HuntOutputs, destination: LotLocation) -> HuntOutputs {
    for lot in &mut outputs.common_lots {
        lot.location = destination.clone();
        lot.reservation = None;
    }
    for drop in &mut outputs.named_drops {
        drop.location = destination.clone();
        drop.reservation = None;
    }
    outputs
}

#[must_use]
pub fn release_equipment_reservations(party: &[HunterInput]) -> Vec<MaterialInstanceId> {
    let mut ids = party
        .iter()
        .flat_map(|hunter| [&hunter.weapon, &hunter.armor])
        .flatten()
        .filter(|item| item.reserved)
        .map(|item| item.item_instance_id.clone())
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn sorted_party_key(party: &[HunterInput]) -> String {
    party
        .iter()
        .map(|hunter| hunter.cat_id.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

fn provenance_origin(site_id: &str, creature_id: &CreatureId, clear_index: u32) -> String {
    format!(
        "enemy_lair:{site_id}:creature:{}:clear:{clear_index}",
        creature_id.as_str()
    )
}

fn make_lot_id(
    site_id: &str,
    generation: u32,
    clear_index: u32,
    output_index: u32,
    suffix: &str,
) -> Result<PhysicalLotId, HuntingError> {
    let hash = stable_hash(
        0,
        &[site_id, suffix],
        &[generation, clear_index, output_index],
    );
    PhysicalLotId::new(format!("hunt_lot_{hash:08x}_{output_index:02}"))
        .map_err(|_| HuntingError::ArithmeticOverflow)
}

fn make_material_instance_id(
    site_id: &str,
    generation: u32,
    clear_index: u32,
    slot: u8,
    suffix: &str,
) -> Result<MaterialInstanceId, HuntingError> {
    let hash = stable_hash(
        0,
        &[site_id, suffix],
        &[generation, clear_index, u32::from(slot)],
    );
    MaterialInstanceId::new(format!("hunt_mat_{hash:08x}_{slot:02}"))
        .map_err(|_| HuntingError::ArithmeticOverflow)
}

fn keyed_percent(
    world_seed: u32,
    site_id: &str,
    generation: u32,
    level: u8,
    clear_index: u32,
    semantic_id: &str,
    suffix: &str,
) -> u8 {
    (lcg_roll(
        world_seed,
        &[site_id, semantic_id, suffix],
        &[generation, u32::from(level), clear_index],
    ) % 100) as u8
}

fn keyed_index(
    world_seed: u32,
    site_id: &str,
    generation: u32,
    level: u8,
    suffix: &str,
    slot: u8,
    len: usize,
) -> usize {
    debug_assert!(len > 0);
    (lcg_roll(
        world_seed,
        &[site_id, suffix],
        &[generation, u32::from(level), u32::from(slot)],
    ) as usize)
        % len
}

#[allow(clippy::too_many_arguments)]
fn keyed_variation(
    world_seed: u32,
    site_id: &str,
    generation: u32,
    level: u8,
    clear_index: u32,
    content_id: &str,
    suffix: &str,
    slot: u8,
    output_index: u32,
) -> i16 {
    let roll = lcg_roll(
        world_seed,
        &[site_id, content_id, suffix],
        &[
            generation,
            u32::from(level),
            clear_index,
            u32::from(slot),
            output_index,
        ],
    ) % 501;
    roll as i16 - 250
}

fn lcg_roll(seed: u32, strings: &[&str], numbers: &[u32]) -> u32 {
    rng::roll_seeded(f64::from(stable_hash(seed, strings, numbers))).next_seed
}

fn stable_hash(seed: u32, strings: &[&str], numbers: &[u32]) -> u32 {
    const FNV_OFFSET: u32 = 2_166_136_261;
    const FNV_PRIME: u32 = 16_777_619;
    let mut hash = seed ^ FNV_OFFSET;
    for string in strings {
        for byte in (string.len() as u64)
            .to_le_bytes()
            .iter()
            .chain(string.as_bytes())
        {
            hash ^= u32::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    for number in numbers {
        for byte in number.to_le_bytes() {
            hash ^= u32::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    hash.max(1)
}
