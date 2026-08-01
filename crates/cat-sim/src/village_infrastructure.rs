//! Physical farm, road, wall, and gate sequences for LAI.60.
//!
//! Storage and Workshop input zones live in [`crate::physical_storage`].
//! This leaf supplies deterministic world-work contracts for the remaining
//! infrastructure that the Leader or responsible officer chooses.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::spatial_tasks::{TaskFootprint, TilePoint};

pub const VILLAGE_INFRASTRUCTURE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfrastructureActor {
    Leader,
    Steward,
    Farmer,
    God,
}

pub fn authorize_exact_infrastructure_action(
    actor: InfrastructureActor,
) -> Result<(), InfrastructureError> {
    if matches!(
        actor,
        InfrastructureActor::Leader | InfrastructureActor::Steward | InfrastructureActor::Farmer
    ) {
        Ok(())
    } else {
        Err(InfrastructureError::GodExactControlForbidden)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhysicalInputProgress {
    pub definition_id: String,
    pub required_units: u64,
    pub delivered_units: u64,
    pub in_transit_units: u64,
    pub consumed_units: u64,
}

impl PhysicalInputProgress {
    pub fn validate(&self) -> Result<(), InfrastructureError> {
        validate_id(&self.definition_id)?;
        if self.required_units == 0
            || self.delivered_units > self.required_units
            || self.in_transit_units > self.required_units
            || self.consumed_units > self.required_units
            || self
                .delivered_units
                .saturating_add(self.in_transit_units)
                .saturating_add(self.consumed_units)
                > self.required_units
        {
            return Err(InfrastructureError::InvalidPhysicalInput);
        }
        Ok(())
    }

    pub fn reserve_in_transit(&mut self, units: u64) -> Result<(), InfrastructureError> {
        self.validate()?;
        if units == 0 || units > self.missing_units() {
            return Err(InfrastructureError::InputOverfill);
        }
        self.in_transit_units = self
            .in_transit_units
            .checked_add(units)
            .ok_or(InfrastructureError::Overflow)?;
        Ok(())
    }

    pub fn deliver(&mut self, units: u64) -> Result<(), InfrastructureError> {
        self.validate()?;
        if units == 0 || units > self.in_transit_units {
            return Err(InfrastructureError::InvalidPhysicalInput);
        }
        self.in_transit_units -= units;
        self.delivered_units = self
            .delivered_units
            .checked_add(units)
            .ok_or(InfrastructureError::Overflow)?;
        Ok(())
    }

    pub fn lose_in_transit(&mut self, units: u64) -> Result<(), InfrastructureError> {
        if units == 0 || units > self.in_transit_units {
            return Err(InfrastructureError::InvalidPhysicalInput);
        }
        self.in_transit_units -= units;
        Ok(())
    }

    pub fn consume_all_delivered(&mut self) -> Result<(), InfrastructureError> {
        self.validate()?;
        if self.delivered_units != self.required_units {
            return Err(InfrastructureError::InputsIncomplete);
        }
        self.consumed_units = self.delivered_units;
        self.delivered_units = 0;
        Ok(())
    }

    #[must_use]
    pub const fn missing_units(&self) -> u64 {
        self.required_units
            .saturating_sub(self.delivered_units)
            .saturating_sub(self.in_transit_units)
            .saturating_sub(self.consumed_units)
    }

    #[must_use]
    pub const fn delivered_complete(&self) -> bool {
        self.delivered_units == self.required_units
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmStage {
    Reserved,
    Clearing,
    DeliverSeed,
    Sowing,
    Growing,
    ReadyToHarvest,
    Harvesting,
    Fallow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FarmPlotProject {
    pub schema_version: u32,
    pub plot_id: String,
    pub footprint: TaskFootprint,
    pub crop_id: String,
    pub stage: FarmStage,
    pub seed: PhysicalInputProgress,
    pub clear_required_minutes: u64,
    pub clear_completed_minutes: u64,
    pub sow_required_minutes: u64,
    pub sow_completed_minutes: u64,
    pub grow_required_minutes: u64,
    pub grow_elapsed_minutes: u64,
    pub harvest_required_minutes: u64,
    pub harvest_completed_minutes: u64,
}

impl FarmPlotProject {
    pub fn validate(&self) -> Result<(), InfrastructureError> {
        if self.schema_version != VILLAGE_INFRASTRUCTURE_SCHEMA_VERSION {
            return Err(InfrastructureError::UnsupportedVersion);
        }
        validate_id(&self.plot_id)?;
        validate_id(&self.crop_id)?;
        self.footprint
            .validate()
            .map_err(|_| InfrastructureError::InvalidFootprint)?;
        self.seed.validate()?;
        if [
            self.clear_required_minutes,
            self.sow_required_minutes,
            self.grow_required_minutes,
            self.harvest_required_minutes,
        ]
        .contains(&0)
            || self.clear_completed_minutes > self.clear_required_minutes
            || self.sow_completed_minutes > self.sow_required_minutes
            || self.grow_elapsed_minutes > self.grow_required_minutes
            || self.harvest_completed_minutes > self.harvest_required_minutes
        {
            return Err(InfrastructureError::InvalidWorkProgress);
        }
        let legal = match self.stage {
            FarmStage::Reserved => self.clear_completed_minutes == 0,
            FarmStage::Clearing => self.clear_completed_minutes < self.clear_required_minutes,
            FarmStage::DeliverSeed => {
                self.clear_completed_minutes == self.clear_required_minutes
                    && self.sow_completed_minutes == 0
            }
            FarmStage::Sowing => {
                self.seed.consumed_units == self.seed.required_units
                    && self.sow_completed_minutes < self.sow_required_minutes
            }
            FarmStage::Growing => {
                self.sow_completed_minutes == self.sow_required_minutes
                    && self.grow_elapsed_minutes < self.grow_required_minutes
            }
            FarmStage::ReadyToHarvest => {
                self.grow_elapsed_minutes == self.grow_required_minutes
                    && self.harvest_completed_minutes == 0
            }
            FarmStage::Harvesting => {
                self.grow_elapsed_minutes == self.grow_required_minutes
                    && self.harvest_completed_minutes < self.harvest_required_minutes
            }
            FarmStage::Fallow => self.harvest_completed_minutes == self.harvest_required_minutes,
        };
        if !legal {
            return Err(InfrastructureError::InvalidStage);
        }
        Ok(())
    }

    pub fn begin_clearing(&mut self) -> Result<(), InfrastructureError> {
        self.require_stage(FarmStage::Reserved)?;
        self.stage = FarmStage::Clearing;
        Ok(())
    }

    pub fn record_clearing(&mut self, minutes: u64) -> Result<(), InfrastructureError> {
        self.require_stage(FarmStage::Clearing)?;
        record_bounded_work(
            &mut self.clear_completed_minutes,
            self.clear_required_minutes,
            minutes,
        )?;
        if self.clear_completed_minutes == self.clear_required_minutes {
            self.stage = FarmStage::DeliverSeed;
        }
        Ok(())
    }

    pub fn begin_sowing(&mut self) -> Result<(), InfrastructureError> {
        self.require_stage(FarmStage::DeliverSeed)?;
        self.seed.consume_all_delivered()?;
        self.stage = FarmStage::Sowing;
        Ok(())
    }

    pub fn record_sowing(&mut self, minutes: u64) -> Result<(), InfrastructureError> {
        self.require_stage(FarmStage::Sowing)?;
        record_bounded_work(
            &mut self.sow_completed_minutes,
            self.sow_required_minutes,
            minutes,
        )?;
        if self.sow_completed_minutes == self.sow_required_minutes {
            self.stage = FarmStage::Growing;
        }
        Ok(())
    }

    pub fn record_growth(&mut self, elapsed_minutes: u64) -> Result<(), InfrastructureError> {
        self.require_stage(FarmStage::Growing)?;
        record_bounded_work(
            &mut self.grow_elapsed_minutes,
            self.grow_required_minutes,
            elapsed_minutes,
        )?;
        if self.grow_elapsed_minutes == self.grow_required_minutes {
            self.stage = FarmStage::ReadyToHarvest;
        }
        Ok(())
    }

    pub fn begin_harvest(&mut self) -> Result<(), InfrastructureError> {
        self.require_stage(FarmStage::ReadyToHarvest)?;
        self.stage = FarmStage::Harvesting;
        Ok(())
    }

    pub fn record_harvest(&mut self, minutes: u64) -> Result<(), InfrastructureError> {
        self.require_stage(FarmStage::Harvesting)?;
        record_bounded_work(
            &mut self.harvest_completed_minutes,
            self.harvest_required_minutes,
            minutes,
        )?;
        if self.harvest_completed_minutes == self.harvest_required_minutes {
            self.stage = FarmStage::Fallow;
        }
        Ok(())
    }

    fn require_stage(&self, stage: FarmStage) -> Result<(), InfrastructureError> {
        if self.stage == stage {
            Ok(())
        } else {
            Err(InfrastructureError::InvalidStage)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileWorkStage {
    Preview,
    MaterialsReserved,
    MaterialsDelivered,
    Labor,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhysicalTileWork {
    pub tile: TilePoint,
    pub stage: TileWorkStage,
    pub material: PhysicalInputProgress,
    pub labor_required_minutes: u64,
    pub labor_completed_minutes: u64,
}

impl PhysicalTileWork {
    pub fn validate(&self) -> Result<(), InfrastructureError> {
        self.material.validate()?;
        if self.labor_required_minutes == 0
            || self.labor_completed_minutes > self.labor_required_minutes
        {
            return Err(InfrastructureError::InvalidWorkProgress);
        }
        let legal = match self.stage {
            TileWorkStage::Preview => {
                self.material.delivered_units == 0
                    && self.material.in_transit_units == 0
                    && self.material.consumed_units == 0
                    && self.labor_completed_minutes == 0
            }
            TileWorkStage::MaterialsReserved => self.material.in_transit_units > 0,
            TileWorkStage::MaterialsDelivered => self.material.delivered_complete(),
            TileWorkStage::Labor => {
                self.material.consumed_units == self.material.required_units
                    && self.labor_completed_minutes < self.labor_required_minutes
            }
            TileWorkStage::Complete => {
                self.material.consumed_units == self.material.required_units
                    && self.labor_completed_minutes == self.labor_required_minutes
            }
        };
        if legal {
            Ok(())
        } else {
            Err(InfrastructureError::InvalidStage)
        }
    }

    pub fn reserve_material(&mut self, units: u64) -> Result<(), InfrastructureError> {
        if !matches!(
            self.stage,
            TileWorkStage::Preview | TileWorkStage::MaterialsReserved
        ) {
            return Err(InfrastructureError::InvalidStage);
        }
        self.material.reserve_in_transit(units)?;
        self.stage = TileWorkStage::MaterialsReserved;
        Ok(())
    }

    pub fn deliver_material(&mut self, units: u64) -> Result<(), InfrastructureError> {
        if self.stage != TileWorkStage::MaterialsReserved {
            return Err(InfrastructureError::InvalidStage);
        }
        self.material.deliver(units)?;
        if self.material.delivered_complete() {
            self.stage = TileWorkStage::MaterialsDelivered;
        }
        Ok(())
    }

    pub fn begin_labor(&mut self) -> Result<(), InfrastructureError> {
        if self.stage != TileWorkStage::MaterialsDelivered {
            return Err(InfrastructureError::InvalidStage);
        }
        self.material.consume_all_delivered()?;
        self.stage = TileWorkStage::Labor;
        Ok(())
    }

    pub fn record_labor(&mut self, minutes: u64) -> Result<(), InfrastructureError> {
        if self.stage != TileWorkStage::Labor {
            return Err(InfrastructureError::InvalidStage);
        }
        record_bounded_work(
            &mut self.labor_completed_minutes,
            self.labor_required_minutes,
            minutes,
        )?;
        if self.labor_completed_minutes == self.labor_required_minutes {
            self.stage = TileWorkStage::Complete;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoadProject {
    pub schema_version: u32,
    pub road_id: String,
    /// Authored traversal order; never canonicalized into a set.
    pub route_preview: Vec<TilePoint>,
    pub tiles: Vec<PhysicalTileWork>,
}

impl RoadProject {
    pub fn validate(&self) -> Result<(), InfrastructureError> {
        if self.schema_version != VILLAGE_INFRASTRUCTURE_SCHEMA_VERSION {
            return Err(InfrastructureError::UnsupportedVersion);
        }
        validate_id(&self.road_id)?;
        validate_route(&self.route_preview)?;
        if self.tiles.len() != self.route_preview.len()
            || self
                .tiles
                .iter()
                .zip(&self.route_preview)
                .any(|(work, preview)| &work.tile != preview || work.validate().is_err())
        {
            return Err(InfrastructureError::InvalidRoute);
        }
        Ok(())
    }

    #[must_use]
    pub fn completed_tiles(&self) -> Vec<TilePoint> {
        self.tiles
            .iter()
            .filter(|work| work.stage == TileWorkStage::Complete)
            .map(|work| work.tile)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BarrierKind {
    Wall,
    Gate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BarrierTileProject {
    pub barrier_id: String,
    pub kind: BarrierKind,
    pub work: PhysicalTileWork,
    pub gate_open: bool,
}

impl BarrierTileProject {
    pub fn validate(&self) -> Result<(), InfrastructureError> {
        validate_id(&self.barrier_id)?;
        self.work.validate()?;
        if self.kind == BarrierKind::Wall && self.gate_open {
            return Err(InfrastructureError::WallCannotOpen);
        }
        Ok(())
    }

    #[must_use]
    pub fn blocks_crossing(&self) -> bool {
        if self.work.stage != TileWorkStage::Complete {
            return false;
        }
        match self.kind {
            BarrierKind::Wall => true,
            BarrierKind::Gate => !self.gate_open,
        }
    }

    pub fn set_gate_open(&mut self, open: bool) -> Result<(), InfrastructureError> {
        if self.kind != BarrierKind::Gate || self.work.stage != TileWorkStage::Complete {
            return Err(InfrastructureError::NotOperationalGate);
        }
        self.gate_open = open;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VillageInfrastructureState {
    pub schema_version: u32,
    pub farms: BTreeMap<String, FarmPlotProject>,
    pub roads: BTreeMap<String, RoadProject>,
    pub barriers: BTreeMap<String, BarrierTileProject>,
}

impl VillageInfrastructureState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: VILLAGE_INFRASTRUCTURE_SCHEMA_VERSION,
            farms: BTreeMap::new(),
            roads: BTreeMap::new(),
            barriers: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), InfrastructureError> {
        if self.schema_version != VILLAGE_INFRASTRUCTURE_SCHEMA_VERSION
            || self
                .farms
                .iter()
                .any(|(id, farm)| id != &farm.plot_id || farm.validate().is_err())
            || self
                .roads
                .iter()
                .any(|(id, road)| id != &road.road_id || road.validate().is_err())
            || self
                .barriers
                .iter()
                .any(|(id, barrier)| id != &barrier.barrier_id || barrier.validate().is_err())
        {
            return Err(InfrastructureError::MalformedState);
        }
        let farm_tiles = self
            .farms
            .values()
            .flat_map(|farm| farm.footprint.tiles.as_slice().iter().copied())
            .collect::<BTreeSet<_>>();
        let barrier_tiles = self
            .barriers
            .values()
            .map(|barrier| barrier.work.tile)
            .collect::<BTreeSet<_>>();
        if farm_tiles.intersection(&barrier_tiles).next().is_some() {
            return Err(InfrastructureError::ConflictingTileUse);
        }
        Ok(())
    }

    #[must_use]
    pub fn tile_blocks_crossing(&self, tile: TilePoint) -> bool {
        self.barriers
            .values()
            .find(|barrier| barrier.work.tile == tile)
            .is_some_and(BarrierTileProject::blocks_crossing)
    }
}

impl Default for VillageInfrastructureState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VillagePriorityReport {
    pub survival_adequately_staffed: bool,
    pub defense_adequately_staffed: bool,
    pub active_village_plans_adequately_staffed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeLaborDestination {
    VillageDemand,
    UsefulHoleDependency,
}

#[must_use]
pub const fn free_labor_destination(report: VillagePriorityReport) -> FreeLaborDestination {
    if report.survival_adequately_staffed
        && report.defense_adequately_staffed
        && report.active_village_plans_adequately_staffed
    {
        FreeLaborDestination::UsefulHoleDependency
    } else {
        FreeLaborDestination::VillageDemand
    }
}

fn validate_route(route: &[TilePoint]) -> Result<(), InfrastructureError> {
    if route.is_empty() {
        return Err(InfrastructureError::InvalidRoute);
    }
    let unique = route.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != route.len()
        || route.windows(2).any(|pair| {
            let dx = i64::from(pair[0].x).abs_diff(i64::from(pair[1].x));
            let dy = i64::from(pair[0].y).abs_diff(i64::from(pair[1].y));
            dx + dy != 1
        })
    {
        return Err(InfrastructureError::InvalidRoute);
    }
    Ok(())
}

fn record_bounded_work(
    completed: &mut u64,
    required: u64,
    minutes: u64,
) -> Result<(), InfrastructureError> {
    if minutes == 0 || *completed >= required {
        return Err(InfrastructureError::InvalidWorkProgress);
    }
    *completed = completed
        .checked_add(minutes)
        .ok_or(InfrastructureError::Overflow)?
        .min(required);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfrastructureError {
    BlankId,
    UnsupportedVersion,
    GodExactControlForbidden,
    InvalidFootprint,
    InvalidPhysicalInput,
    InputOverfill,
    InputsIncomplete,
    InvalidWorkProgress,
    InvalidStage,
    InvalidRoute,
    ConflictingTileUse,
    WallCannotOpen,
    NotOperationalGate,
    MalformedState,
    Overflow,
}

impl std::fmt::Display for InfrastructureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "village infrastructure error: {self:?}")
    }
}

impl std::error::Error for InfrastructureError {}

fn validate_id(value: &str) -> Result<(), InfrastructureError> {
    if value.trim().is_empty() {
        Err(InfrastructureError::BlankId)
    } else {
        Ok(())
    }
}
