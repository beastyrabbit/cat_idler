//! Pure LAI.40 fishing profiles, Hut geometry, and physical catch transitions.

use std::{collections::BTreeSet, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    food_ecology::{
        EcologyReport, FishTask, FoodEcology, FoodEcologyError, HandFishingRequest, ReportAudience,
        ReportLevel, Tile,
    },
    quality_lots::{
        ItemInstance, LotLocation, QualityBand, QualityLotError, QualityLotLedger, RecoveryReason,
    },
    rng,
    spatial_tasks::{Rect, TaskFootprint, TilePoint},
};

pub const FISHING_SCHEMA_VERSION: u32 = 1;
pub const HAND_CATCH_UNITS: u32 = 12;
pub const HAND_CYCLE_GAME_MINUTES: u64 = 45;
pub const MAX_FISHING_RECEIPTS: usize = 256;
pub const MAX_FISHING_ID_BYTES: usize = 64;
pub const MAX_FISHING_FINGERPRINT_BYTES: usize = 1_024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FishingError {
    Schema(u32),
    InvalidOrientation,
    InvalidHutPlacement,
    OverlappingHut,
    UnreachableHut,
    InvalidRod,
    BrokenRod,
    DuplicateAttempt,
    AttemptConflict,
    AttemptIndex { expected: u64, actual: u64 },
    MalformedPersistence,
    ArithmeticOverflow,
    Ecology(FoodEcologyError),
    Ledger(QualityLotError),
}
impl fmt::Display for FishingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid fishing state: {self:?}")
    }
}
impl std::error::Error for FishingError {}
impl From<FoodEcologyError> for FishingError {
    fn from(value: FoodEcologyError) -> Self {
        Self::Ecology(value)
    }
}
impl From<QualityLotError> for FishingError {
    fn from(value: QualityLotError) -> Self {
        Self::Ledger(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockOrientation {
    North,
    East,
    South,
    West,
}
impl DockOrientation {
    const fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FishingProfile {
    pub catch_units: u32,
    pub cycle_game_minutes: u64,
    pub reliability_percent: u8,
    pub rod_reliability_contribution_percent: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FishingMode {
    Hand,
    RodOnly,
    StaffedHut,
    RodAndStaffedHut,
}

/// Exact P1-C02 profile. Rod quality changes only its reliability contribution.
pub fn fishing_profile(rod_quality: Option<QualityBand>, staffed_hut: bool) -> FishingProfile {
    match (rod_quality, staffed_hut) {
        (None, false) => FishingProfile {
            catch_units: 12,
            cycle_game_minutes: 45,
            reliability_percent: 75,
            rod_reliability_contribution_percent: 0,
        },
        (None, true) => FishingProfile {
            catch_units: 18,
            cycle_game_minutes: 30,
            reliability_percent: 95,
            rod_reliability_contribution_percent: 0,
        },
        (Some(quality), false) => {
            let rod = u8::try_from((15_u16 * quality.item_effect_durability_percent()) / 100)
                .expect("quality contribution fits u8");
            FishingProfile {
                catch_units: 15,
                cycle_game_minutes: 36,
                reliability_percent: 75 + rod,
                rod_reliability_contribution_percent: rod,
            }
        }
        (Some(quality), true) => {
            let rod = u8::try_from((15_u16 * quality.item_effect_durability_percent()) / 100)
                .expect("quality contribution fits u8");
            FishingProfile {
                catch_units: 24,
                cycle_game_minutes: 24,
                reliability_percent: (95 + rod).min(100),
                rod_reliability_contribution_percent: rod,
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FishingHutFootprint {
    pub land: TaskFootprint,
    pub dock_land: TilePoint,
    pub reserved_water: TilePoint,
    pub orientation: DockOrientation,
}

pub fn fishing_hut_footprint(
    anchor: TilePoint,
    orientation: DockOrientation,
) -> Result<FishingHutFootprint, FishingError> {
    let rect = Rect::try_new(anchor, 3, 3).map_err(|_| FishingError::InvalidHutPlacement)?;
    let (dx, dy) = orientation.delta();
    let center = TilePoint {
        x: anchor
            .x
            .checked_add(1)
            .ok_or(FishingError::ArithmeticOverflow)?,
        y: anchor
            .y
            .checked_add(1)
            .ok_or(FishingError::ArithmeticOverflow)?,
    };
    let dock_land = TilePoint {
        x: center
            .x
            .checked_add(dx)
            .ok_or(FishingError::ArithmeticOverflow)?,
        y: center
            .y
            .checked_add(dy)
            .ok_or(FishingError::ArithmeticOverflow)?,
    };
    let reserved_water = TilePoint {
        x: dock_land
            .x
            .checked_add(dx)
            .ok_or(FishingError::ArithmeticOverflow)?,
        y: dock_land
            .y
            .checked_add(dy)
            .ok_or(FishingError::ArithmeticOverflow)?,
    };
    Ok(FishingHutFootprint {
        land: TaskFootprint::rectangular(rect),
        dock_land,
        reserved_water,
        orientation,
    })
}

pub fn validate_hut_placement(
    footprint: &FishingHutFootprint,
    land: &BTreeSet<TilePoint>,
    water: &BTreeSet<TilePoint>,
    reachable: &BTreeSet<TilePoint>,
    occupied: &BTreeSet<TilePoint>,
) -> Result<(), FishingError> {
    let canonical = fishing_hut_footprint(footprint.land.anchor, footprint.orientation)?;
    if footprint != &canonical {
        return Err(FishingError::InvalidOrientation);
    }
    if !water.contains(&footprint.reserved_water) {
        return Err(FishingError::InvalidHutPlacement);
    }
    if footprint
        .land
        .tiles
        .as_slice()
        .iter()
        .any(|tile| !land.contains(tile) || !reachable.contains(tile))
    {
        return Err(FishingError::UnreachableHut);
    }
    if footprint
        .land
        .tiles
        .as_slice()
        .iter()
        .any(|tile| occupied.contains(tile))
        || occupied.contains(&footprint.reserved_water)
    {
        return Err(FishingError::OverlappingHut);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FishingAttempt {
    pub command_id: String,
    pub habitat_id: String,
    pub attempt_index: u64,
    pub world_seed: u32,
    pub now_game_minute: u64,
    pub source_quality: QualityBand,
    pub worker_skill: u8,
    pub staffed_hut: bool,
    pub cargo_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FishingOutcome {
    pub mode: FishingMode,
    pub profile: FishingProfile,
    pub succeeded: bool,
    pub caught_lot_id: Option<crate::content_manifest::PhysicalLotId>,
    pub caught_units: u32,
    pub shoreline_task: TaskFootprint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Receipt {
    id: String,
    fingerprint: String,
    outcome: FishingOutcomeWire,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FishingAuthority {
    next_attempt_index: u64,
    receipts: Vec<Receipt>,
}
impl Default for FishingAuthority {
    fn default() -> Self {
        Self {
            next_attempt_index: 0,
            receipts: Vec::new(),
        }
    }
}

impl FishingAuthority {
    #[must_use]
    pub const fn next_attempt_index(&self) -> u64 {
        self.next_attempt_index
    }

    pub fn fish(
        &mut self,
        ecology: &mut FoodEcology,
        ledger: &mut QualityLotLedger,
        rod: Option<&mut ItemInstance>,
        attempt: FishingAttempt,
    ) -> Result<FishingOutcome, FishingError> {
        let fingerprint = format!("{attempt:?}");
        if let Some(receipt) = self
            .receipts
            .iter()
            .find(|receipt| receipt.id == attempt.command_id)
        {
            return if receipt.fingerprint == fingerprint {
                receipt.outcome.clone().into_outcome()
            } else {
                Err(FishingError::AttemptConflict)
            };
        }
        if !valid_id(&attempt.command_id)
            || !valid_id(&attempt.habitat_id)
            || !valid_id(&attempt.cargo_id)
        {
            return Err(FishingError::InvalidHutPlacement);
        }
        if attempt.attempt_index != self.next_attempt_index {
            return Err(FishingError::AttemptIndex {
                expected: self.next_attempt_index,
                actual: attempt.attempt_index,
            });
        }
        let shore = ecology.fish_habitat().shoreline_task_tile;
        let shoreline_task = shoreline_task(shore)?;
        let rod_quality = rod.as_ref().map(|value| value.quality);
        if let Some(value) = rod.as_ref() {
            if value.definition_id.as_str() != "fishing_rod" {
                return Err(FishingError::InvalidRod);
            }
            if value.durability == 0 {
                return Err(FishingError::BrokenRod);
            }
        }
        let profile = fishing_profile(rod_quality, attempt.staffed_hut);
        let mode = match (rod_quality.is_some(), attempt.staffed_hut) {
            (false, false) => FishingMode::Hand,
            (true, false) => FishingMode::RodOnly,
            (false, true) => FishingMode::StaffedHut,
            (true, true) => FishingMode::RodAndStaffedHut,
        };
        let roll = keyed_success(
            attempt.world_seed,
            &attempt.habitat_id,
            shore,
            attempt.attempt_index,
            profile.reliability_percent,
        );
        let mut next_self = self.clone();
        next_self.next_attempt_index = next_self
            .next_attempt_index
            .checked_add(1)
            .ok_or(FishingError::ArithmeticOverflow)?;
        let mut next_ecology = ecology.clone();
        let mut next_ledger = ledger.clone();
        let mut worn_rod = rod.as_ref().map(|value| (**value).clone());
        if let Some(value) = worn_rod.as_mut() {
            value.durability = value
                .durability
                .checked_sub(1)
                .ok_or(FishingError::BrokenRod)?;
        }
        let available_units = next_ecology.fish_habitat().stock;
        let caught = roll && available_units > 0;
        let (caught_lot_id, caught_units) = if caught {
            let units = profile.catch_units.min(next_ecology.fish_habitat().stock);
            let mut lot = next_ecology.catch_fish_units(
                HandFishingRequest {
                    task: FishTask { task_tile: shore },
                    source_quality: attempt.source_quality,
                    worker_skill: attempt.worker_skill,
                    tool_quality: rod_quality,
                    fixture_quality: None,
                    world_seed: attempt.world_seed,
                    catch_index: next_ecology.next_catch_index(),
                    now_tick: attempt.now_game_minute,
                },
                units,
            )?;
            lot.location = LotLocation::Source(attempt.habitat_id.clone());
            next_ledger.insert_lot(lot.clone())?;
            next_ledger.move_lot(&lot.id, LotLocation::Cargo(attempt.cargo_id.clone()))?;
            (Some(lot.id), units)
        } else {
            (None, 0)
        };
        let outcome = FishingOutcome {
            mode,
            profile,
            succeeded: caught,
            caught_lot_id,
            caught_units,
            shoreline_task,
        };
        next_self.receipts.push(Receipt {
            id: attempt.command_id,
            fingerprint,
            outcome: FishingOutcomeWire::from(&outcome),
        });
        if next_self.receipts.len() > MAX_FISHING_RECEIPTS {
            next_self.receipts.remove(0);
        }
        *self = next_self;
        *ecology = next_ecology;
        *ledger = next_ledger;
        if let (Some(destination), Some(worn)) = (rod, worn_rod) {
            *destination = worn;
        }
        Ok(outcome)
    }
    pub fn recover_cargo(
        &mut self,
        ledger: &mut QualityLotLedger,
        lot_id: &crate::content_manifest::PhysicalLotId,
        reason: RecoveryReason,
        destination: LotLocation,
    ) -> Result<(), FishingError> {
        let mut next = ledger.clone();
        next.recover_lot(lot_id, reason, destination)?;
        *ledger = next;
        Ok(())
    }
    #[must_use]
    pub fn habitat_report(
        &self,
        ecology: &FoodEcology,
        audience: ReportAudience,
        level: ReportLevel,
    ) -> EcologyReport {
        ecology.fish_report(audience, level)
    }
}

fn shoreline_task(tile: Tile) -> Result<TaskFootprint, FishingError> {
    let point = TilePoint {
        x: tile.x,
        y: tile.y,
    };
    Ok(TaskFootprint::rectangular(
        Rect::try_new(point, 1, 1).map_err(|_| FishingError::InvalidHutPlacement)?,
    ))
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_FISHING_ID_BYTES
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':'))
}

fn keyed_success(seed: u32, habitat: &str, tile: Tile, index: u64, percent: u8) -> bool {
    const FNV_OFFSET: u32 = 2_166_136_261;
    const FNV_PRIME: u32 = 16_777_619;
    let mut key_seed = seed ^ FNV_OFFSET;
    for bytes in [
        habitat.as_bytes(),
        &tile.x.to_le_bytes(),
        &tile.y.to_le_bytes(),
        &index.to_le_bytes(),
    ] {
        for byte in (bytes.len() as u64).to_le_bytes().iter().chain(bytes) {
            key_seed ^= u32::from(*byte);
            key_seed = key_seed.wrapping_mul(FNV_PRIME);
        }
    }
    let roll = rng::roll_seeded(f64::from(key_seed.max(1))).next_seed % 100;
    roll < u32::from(percent)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutcomeWire {
    mode: FishingMode,
    catch_units: u32,
    cycle_game_minutes: u64,
    reliability_percent: u8,
    rod_reliability_contribution_percent: u8,
    succeeded: bool,
    caught_lot_id: Option<crate::content_manifest::PhysicalLotId>,
    caught_units: u32,
    shoreline: TileWire,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TileWire {
    x: i32,
    y: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct FishingOutcomeWire(OutcomeWire);
impl From<&FishingOutcome> for FishingOutcomeWire {
    fn from(value: &FishingOutcome) -> Self {
        Self(OutcomeWire {
            mode: value.mode,
            catch_units: value.profile.catch_units,
            cycle_game_minutes: value.profile.cycle_game_minutes,
            reliability_percent: value.profile.reliability_percent,
            rod_reliability_contribution_percent: value
                .profile
                .rod_reliability_contribution_percent,
            succeeded: value.succeeded,
            caught_lot_id: value.caught_lot_id.clone(),
            caught_units: value.caught_units,
            shoreline: TileWire {
                x: value.shoreline_task.anchor.x,
                y: value.shoreline_task.anchor.y,
            },
        })
    }
}
impl FishingOutcomeWire {
    fn into_outcome(self) -> Result<FishingOutcome, FishingError> {
        let profile = FishingProfile {
            catch_units: self.0.catch_units,
            cycle_game_minutes: self.0.cycle_game_minutes,
            reliability_percent: self.0.reliability_percent,
            rod_reliability_contribution_percent: self.0.rod_reliability_contribution_percent,
        };
        validate_outcome_wire(&self.0, profile)?;
        Ok(FishingOutcome {
            mode: self.0.mode,
            profile,
            succeeded: self.0.succeeded,
            caught_lot_id: self.0.caught_lot_id,
            caught_units: self.0.caught_units,
            shoreline_task: shoreline_task(Tile {
                x: self.0.shoreline.x,
                y: self.0.shoreline.y,
            })?,
        })
    }
}

fn validate_outcome_wire(wire: &OutcomeWire, profile: FishingProfile) -> Result<(), FishingError> {
    let rod_contributions = [12, 15, 17, 20, 24];
    let profile_valid = match wire.mode {
        FishingMode::Hand => {
            profile.catch_units == 12
                && profile.cycle_game_minutes == 45
                && profile.reliability_percent == 75
                && profile.rod_reliability_contribution_percent == 0
        }
        FishingMode::RodOnly => {
            profile.catch_units == 15
                && profile.cycle_game_minutes == 36
                && rod_contributions.contains(&profile.rod_reliability_contribution_percent)
                && profile.reliability_percent == 75 + profile.rod_reliability_contribution_percent
        }
        FishingMode::StaffedHut => {
            profile.catch_units == 18
                && profile.cycle_game_minutes == 30
                && profile.reliability_percent == 95
                && profile.rod_reliability_contribution_percent == 0
        }
        FishingMode::RodAndStaffedHut => {
            profile.catch_units == 24
                && profile.cycle_game_minutes == 24
                && profile.reliability_percent == 100
                && rod_contributions.contains(&profile.rod_reliability_contribution_percent)
        }
    };
    let result_valid = if wire.succeeded {
        wire.caught_lot_id.is_some() && (1..=profile.catch_units).contains(&wire.caught_units)
    } else {
        wire.caught_lot_id.is_none() && wire.caught_units == 0
    };
    if profile_valid && result_valid {
        Ok(())
    } else {
        Err(FishingError::MalformedPersistence)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StateWire {
    schema_version: u32,
    next_attempt_index: u64,
    receipts: Vec<ReceiptWire>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReceiptWire {
    id: String,
    fingerprint: String,
    outcome: OutcomeWire,
}
impl Serialize for FishingAuthority {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        StateWire {
            schema_version: FISHING_SCHEMA_VERSION,
            next_attempt_index: self.next_attempt_index,
            receipts: self
                .receipts
                .iter()
                .map(|value| ReceiptWire {
                    id: value.id.clone(),
                    fingerprint: value.fingerprint.clone(),
                    outcome: value.outcome.0.clone(),
                })
                .collect(),
        }
        .serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for FishingAuthority {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = StateWire::deserialize(deserializer)?;
        if wire.schema_version != FISHING_SCHEMA_VERSION {
            return Err(de::Error::custom(FishingError::Schema(wire.schema_version)));
        }
        if wire.receipts.len() > MAX_FISHING_RECEIPTS
            || wire.next_attempt_index
                < u64::try_from(wire.receipts.len())
                    .map_err(|_| de::Error::custom(FishingError::MalformedPersistence))?
        {
            return Err(de::Error::custom(FishingError::MalformedPersistence));
        }
        let mut ids = BTreeSet::new();
        let mut receipts = Vec::new();
        for receipt in wire.receipts {
            if !valid_id(&receipt.id)
                || receipt.fingerprint.is_empty()
                || receipt.fingerprint.len() > MAX_FISHING_FINGERPRINT_BYTES
                || !ids.insert(receipt.id.clone())
                || FishingOutcomeWire(receipt.outcome.clone())
                    .into_outcome()
                    .is_err()
            {
                return Err(de::Error::custom(FishingError::DuplicateAttempt));
            }
            receipts.push(Receipt {
                id: receipt.id,
                fingerprint: receipt.fingerprint,
                outcome: FishingOutcomeWire(receipt.outcome),
            });
        }
        Ok(Self {
            next_attempt_index: wire.next_attempt_index,
            receipts,
        })
    }
}
