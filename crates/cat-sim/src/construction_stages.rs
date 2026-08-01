//! Versioned three-stage physical construction contracts.
//!
//! This additive Leader-AI leaf replaces the one-timer mental model with a
//! persisted site → scaffold → structure → fit-out lifecycle. World scheduling,
//! hauling, and rendering integrate this state separately; this module owns the
//! deterministic state machine and cargo-conservation invariants only.

use std::collections::BTreeSet;

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    content_manifest::ContentManifest,
    spatial_tasks::{OrderedTiles, TaskFootprint, TilePoint, footprint_for},
    types::BuildingType,
};

pub const CURRENT_CONSTRUCTION_PROJECT_VERSION: u32 = 1;
/// Plan 2's raw “Wood” scaffold input is the unified Plan 1 manifest's
/// canonical `resource_logs` content ID; no second `wood` content ID or compatibility
/// alias is created.
pub const BASIC_SCAFFOLD_CONTENT_ID: &str = "resource_logs";
pub const DEVELOPED_SCAFFOLD_CONTENT_IDS: &[&str] = &["resource_lumber", "resource_planks"];
pub const GAME_HOUR_MS: u64 = 60 * 60 * 1_000;

/// `ceil(8 game-hours × (target_level - 1)^1.25)` for levels 2 through 10.
///
/// The exact table keeps the simulation independent of platform floating-point
/// `powf` behavior while preserving the settled design formula.
const BUILDING_UPGRADE_DURATION_MS: [u64; 9] = [
    28_800_000,
    68_498_330,
    113_708_795,
    162_917_403,
    215_330_225,
    270_446_616,
    327_917_835,
    387_485_069,
    448_947_570,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionTargetKind {
    Building,
    BuildingUpgrade,
    HoleUpgrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScaffoldTier {
    Basic,
    Developed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionStage {
    SiteReserved,
    DeliverScaffold,
    BuildScaffold,
    DeliverStructure,
    BuildStructure,
    DeliverFitOut,
    BuildFitOut,
    Operational,
    Cancelled,
}

impl ConstructionStage {
    #[must_use]
    pub const fn is_delivery(self) -> bool {
        matches!(
            self,
            Self::DeliverScaffold | Self::DeliverStructure | Self::DeliverFitOut
        )
    }

    #[must_use]
    pub const fn is_labor(self) -> bool {
        matches!(
            self,
            Self::BuildScaffold | Self::BuildStructure | Self::BuildFitOut
        )
    }

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Operational | Self::Cancelled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConstructionCargoLine {
    pub content_id: String,
    pub required_units: u32,
    pub delivered_units: u32,
    pub in_transit_units: u32,
    pub consumed_units: u32,
}

impl ConstructionCargoLine {
    #[must_use]
    pub fn new(content_id: impl Into<String>, required_units: u32) -> Self {
        Self {
            content_id: content_id.into(),
            required_units,
            delivered_units: 0,
            in_transit_units: 0,
            consumed_units: 0,
        }
    }

    #[must_use]
    pub const fn accounted_units(&self) -> u32 {
        self.delivered_units
            .saturating_add(self.in_transit_units)
            .saturating_add(self.consumed_units)
    }

    #[must_use]
    pub const fn missing_units(&self) -> u32 {
        self.required_units.saturating_sub(self.accounted_units())
    }

    #[must_use]
    pub const fn is_fully_delivered(&self) -> bool {
        self.in_transit_units == 0
            && self.consumed_units == 0
            && self.delivered_units == self.required_units
    }

    fn validate(&self) -> Result<(), ConstructionInvariantError> {
        if self.content_id.trim().is_empty() {
            return Err(ConstructionInvariantError::EmptyContentId);
        }
        if self.required_units == 0 {
            return Err(ConstructionInvariantError::EmptyCargoLine);
        }
        let accounted_units = u64::from(self.delivered_units)
            + u64::from(self.in_transit_units)
            + u64::from(self.consumed_units);
        if accounted_units > u64::from(self.required_units) {
            return Err(ConstructionInvariantError::CargoOverfilled {
                content_id: self.content_id.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConstructionStageBill {
    pub lines: Vec<ConstructionCargoLine>,
}

impl ConstructionStageBill {
    #[must_use]
    pub fn new(lines: impl IntoIterator<Item = ConstructionCargoLine>) -> Self {
        let mut lines = lines.into_iter().collect::<Vec<_>>();
        lines.sort_by(|left, right| left.content_id.cmp(&right.content_id));
        Self { lines }
    }

    #[must_use]
    pub fn is_fully_delivered(&self) -> bool {
        !self.lines.is_empty()
            && self
                .lines
                .iter()
                .all(ConstructionCargoLine::is_fully_delivered)
    }

    #[must_use]
    pub fn missing_units(&self) -> u64 {
        self.lines
            .iter()
            .map(|line| u64::from(line.missing_units()))
            .sum()
    }

    #[must_use]
    pub fn is_fully_consumed(&self) -> bool {
        self.lines.iter().all(|line| {
            line.delivered_units == 0
                && line.in_transit_units == 0
                && line.consumed_units == line.required_units
        })
    }

    #[must_use]
    pub fn has_unconsumed_cargo(&self) -> bool {
        self.lines
            .iter()
            .any(|line| line.delivered_units > 0 || line.in_transit_units > 0)
    }

    fn line_mut(
        &mut self,
        content_id: &str,
    ) -> Result<&mut ConstructionCargoLine, ConstructionMutationError> {
        self.lines
            .iter_mut()
            .find(|line| line.content_id == content_id)
            .ok_or_else(|| ConstructionMutationError::UnexpectedContent {
                content_id: content_id.to_owned(),
            })
    }

    fn validate(&self) -> Result<(), ConstructionInvariantError> {
        if self.lines.is_empty() {
            return Err(ConstructionInvariantError::EmptyStageBill);
        }
        let mut ids = BTreeSet::new();
        for line in &self.lines {
            line.validate()?;
            if !ids.insert(line.content_id.as_str()) {
                return Err(ConstructionInvariantError::DuplicateContentId {
                    content_id: line.content_id.clone(),
                });
            }
        }
        if !self
            .lines
            .windows(2)
            .all(|pair| pair[0].content_id < pair[1].content_id)
        {
            return Err(ConstructionInvariantError::NonCanonicalCargoOrder);
        }
        Ok(())
    }

    fn consume_all(&mut self) {
        for line in &mut self.lines {
            line.consumed_units = line.required_units;
            line.delivered_units = 0;
        }
    }

    fn drain_unconsumed(&mut self) -> Vec<SalvagedCargo> {
        let salvage = self
            .lines
            .iter()
            .filter_map(|line| {
                (line.delivered_units > 0 || line.in_transit_units > 0).then(|| SalvagedCargo {
                    content_id: line.content_id.clone(),
                    delivered_units: line.delivered_units,
                    in_transit_units: line.in_transit_units,
                })
            })
            .collect();
        for line in &mut self.lines {
            line.delivered_units = 0;
            line.in_transit_units = 0;
        }
        salvage
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConstructionBills {
    pub scaffold: ConstructionStageBill,
    pub structure: ConstructionStageBill,
    pub fit_out: ConstructionStageBill,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConstructionProject {
    pub version: u32,
    pub project_id: String,
    pub target_kind: ConstructionTargetKind,
    pub building_type: Option<BuildingType>,
    pub target_level: u32,
    pub scaffold_tier: ScaffoldTier,
    #[serde(deserialize_with = "deserialize_strict_task_footprint")]
    pub footprint: TaskFootprint,
    pub stage: ConstructionStage,
    pub bills: ConstructionBills,
    pub original_total_work_ms: u64,
    pub stage_work_remaining_ms: u64,
    pub accepted_click_work_ms: u64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

fn deserialize_strict_task_footprint<'de, D>(deserializer: D) -> Result<TaskFootprint, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct StrictTilePoint {
        x: i32,
        y: i32,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct StrictTaskFootprint {
        anchor: StrictTilePoint,
        width: i32,
        height: i32,
        tiles: Vec<StrictTilePoint>,
    }

    let wire = StrictTaskFootprint::deserialize(deserializer)?;
    Ok(TaskFootprint {
        anchor: TilePoint {
            x: wire.anchor.x,
            y: wire.anchor.y,
        },
        width: wire.width,
        height: wire.height,
        tiles: OrderedTiles::canonical(wire.tiles.into_iter().map(|tile| TilePoint {
            x: tile.x,
            y: tile.y,
        })),
    })
}

impl ConstructionProject {
    pub fn new(
        project_id: impl Into<String>,
        target_kind: ConstructionTargetKind,
        building_type: Option<BuildingType>,
        target_level: u32,
        scaffold_tier: ScaffoldTier,
        footprint: TaskFootprint,
        bills: ConstructionBills,
        original_total_work_ms: u64,
        now_ms: i64,
    ) -> Result<Self, ConstructionInvariantError> {
        let project = Self {
            version: CURRENT_CONSTRUCTION_PROJECT_VERSION,
            project_id: project_id.into(),
            target_kind,
            building_type,
            target_level,
            scaffold_tier,
            footprint,
            stage: ConstructionStage::SiteReserved,
            bills,
            original_total_work_ms,
            stage_work_remaining_ms: 0,
            accepted_click_work_ms: 0,
            created_at_ms: now_ms,
            updated_at_ms: now_ms,
        };
        project.validate()?;
        Ok(project)
    }

    pub fn validate(&self) -> Result<(), ConstructionInvariantError> {
        if self.version != CURRENT_CONSTRUCTION_PROJECT_VERSION {
            return Err(ConstructionInvariantError::UnsupportedVersion {
                found: self.version,
            });
        }
        if self.project_id.trim().is_empty() {
            return Err(ConstructionInvariantError::EmptyProjectId);
        }
        match self.target_kind {
            ConstructionTargetKind::Building if self.target_level != 1 => {
                return Err(ConstructionInvariantError::InvalidTargetLevel);
            }
            ConstructionTargetKind::BuildingUpgrade if !(2..=10).contains(&self.target_level) => {
                return Err(ConstructionInvariantError::InvalidTargetLevel);
            }
            ConstructionTargetKind::HoleUpgrade if !(1..=10).contains(&self.target_level) => {
                return Err(ConstructionInvariantError::InvalidTargetLevel);
            }
            _ => {}
        }
        if self.original_total_work_ms == 0 {
            return Err(ConstructionInvariantError::ZeroDuration);
        }
        if {
            let (scaffold, structure, fit_out) = stage_work_durations(self.original_total_work_ms);
            scaffold == 0 || structure == 0 || fit_out == 0
        } {
            return Err(ConstructionInvariantError::ZeroStageDuration);
        }
        if self.accepted_click_work_ms > self.original_total_work_ms {
            return Err(ConstructionInvariantError::InvalidClickWork);
        }
        match self.target_kind {
            ConstructionTargetKind::Building | ConstructionTargetKind::BuildingUpgrade
                if self.building_type.is_none() =>
            {
                return Err(ConstructionInvariantError::MissingBuildingType);
            }
            ConstructionTargetKind::HoleUpgrade if self.building_type.is_some() => {
                return Err(ConstructionInvariantError::UnexpectedBuildingType);
            }
            _ => {}
        }
        if self.target_kind == ConstructionTargetKind::BuildingUpgrade {
            if self.scaffold_tier != ScaffoldTier::Developed {
                return Err(ConstructionInvariantError::UpgradeRequiresDevelopedScaffold);
            }
            if building_upgrade_duration_ms(self.target_level) != Some(self.original_total_work_ms)
            {
                return Err(ConstructionInvariantError::InvalidUpgradeDuration);
            }
        }
        self.footprint
            .validate()
            .map_err(|_| ConstructionInvariantError::InvalidFootprint)?;
        if let Some(building_type) = self.building_type {
            let (expected_width, expected_height) = footprint_for(building_type);
            let expected_tiles = usize::try_from(expected_width)
                .ok()
                .and_then(|width| {
                    usize::try_from(expected_height)
                        .ok()
                        .and_then(|height| width.checked_mul(height))
                })
                .ok_or(ConstructionInvariantError::InvalidFootprint)?;
            if self.footprint.width != expected_width
                || self.footprint.height != expected_height
                || self.footprint.tiles.len() != expected_tiles
            {
                return Err(ConstructionInvariantError::NonCanonicalBuildingFootprint);
            }
        } else if self.target_kind == ConstructionTargetKind::HoleUpgrade
            && (self.footprint.width != 3
                || self.footprint.height != 3
                || self.footprint.tiles.len() != 9)
        {
            return Err(ConstructionInvariantError::InvalidHoleWorkFootprint);
        }
        self.bills.scaffold.validate()?;
        self.bills.structure.validate()?;
        self.bills.fit_out.validate()?;
        self.validate_manifest_content_ids()?;
        self.validate_scaffold_material()?;
        self.validate_stage_state()?;
        Ok(())
    }

    fn validate_manifest_content_ids(&self) -> Result<(), ConstructionInvariantError> {
        let content_entries = ContentManifest::embedded().canonical_content_entries();
        for line in self
            .bills
            .scaffold
            .lines
            .iter()
            .chain(&self.bills.structure.lines)
            .chain(&self.bills.fit_out.lines)
        {
            if !content_entries
                .iter()
                .any(|entry| entry.content_id.as_str() == line.content_id)
            {
                return Err(ConstructionInvariantError::UnknownContentId {
                    content_id: line.content_id.clone(),
                });
            }
        }
        Ok(())
    }

    fn validate_stage_state(&self) -> Result<(), ConstructionInvariantError> {
        let (scaffold_duration, structure_duration, fit_out_duration) =
            stage_work_durations(self.original_total_work_ms);
        if (self.stage.is_delivery()
            || matches!(self.stage, ConstructionStage::SiteReserved)
            || self.stage.is_terminal())
            && self.stage_work_remaining_ms != 0
        {
            return Err(if self.stage.is_terminal() {
                ConstructionInvariantError::TerminalHasRemainingWork
            } else {
                ConstructionInvariantError::DeliveryHasLaborProgress
            });
        }
        let valid_work = match self.stage {
            ConstructionStage::BuildScaffold => {
                (1..=scaffold_duration).contains(&self.stage_work_remaining_ms)
            }
            ConstructionStage::BuildStructure => {
                (1..=structure_duration).contains(&self.stage_work_remaining_ms)
            }
            ConstructionStage::BuildFitOut => {
                (1..=fit_out_duration).contains(&self.stage_work_remaining_ms)
            }
            _ => true,
        };
        if !valid_work {
            return Err(ConstructionInvariantError::InvalidStageProgress);
        }
        let pristine = |bill: &ConstructionStageBill| {
            bill.lines.iter().all(|line| {
                line.delivered_units == 0 && line.in_transit_units == 0 && line.consumed_units == 0
            })
        };
        let stage_valid = match self.stage {
            ConstructionStage::SiteReserved => {
                pristine(&self.bills.scaffold)
                    && pristine(&self.bills.structure)
                    && pristine(&self.bills.fit_out)
            }
            ConstructionStage::DeliverScaffold => {
                self.bills
                    .scaffold
                    .lines
                    .iter()
                    .all(|line| line.consumed_units == 0)
                    && pristine(&self.bills.structure)
                    && pristine(&self.bills.fit_out)
            }
            ConstructionStage::BuildScaffold => {
                self.bills.scaffold.is_fully_consumed()
                    && pristine(&self.bills.structure)
                    && pristine(&self.bills.fit_out)
            }
            ConstructionStage::DeliverStructure => {
                self.bills.scaffold.is_fully_consumed()
                    && self
                        .bills
                        .structure
                        .lines
                        .iter()
                        .all(|line| line.consumed_units == 0)
                    && pristine(&self.bills.fit_out)
            }
            ConstructionStage::BuildStructure => {
                self.bills.scaffold.is_fully_consumed()
                    && self.bills.structure.is_fully_consumed()
                    && pristine(&self.bills.fit_out)
            }
            ConstructionStage::DeliverFitOut => {
                self.bills.scaffold.is_fully_consumed()
                    && self.bills.structure.is_fully_consumed()
                    && self
                        .bills
                        .fit_out
                        .lines
                        .iter()
                        .all(|line| line.consumed_units == 0)
            }
            ConstructionStage::BuildFitOut | ConstructionStage::Operational => {
                self.bills.scaffold.is_fully_consumed()
                    && self.bills.structure.is_fully_consumed()
                    && self.bills.fit_out.is_fully_consumed()
            }
            ConstructionStage::Cancelled => {
                !self.bills.scaffold.has_unconsumed_cargo()
                    && !self.bills.structure.has_unconsumed_cargo()
                    && !self.bills.fit_out.has_unconsumed_cargo()
            }
        };
        if !stage_valid {
            return Err(ConstructionInvariantError::InvalidStageCargoState);
        }
        Ok(())
    }

    fn validate_scaffold_material(&self) -> Result<(), ConstructionInvariantError> {
        let ids = self
            .bills
            .scaffold
            .lines
            .iter()
            .map(|line| line.content_id.as_str())
            .collect::<BTreeSet<_>>();
        let valid = match self.scaffold_tier {
            ScaffoldTier::Basic => ids.len() == 1 && ids.contains(BASIC_SCAFFOLD_CONTENT_ID),
            ScaffoldTier::Developed => {
                !ids.is_empty()
                    && ids
                        .iter()
                        .all(|id| DEVELOPED_SCAFFOLD_CONTENT_IDS.contains(id))
            }
        };
        if valid {
            Ok(())
        } else {
            Err(ConstructionInvariantError::InvalidScaffoldMaterial)
        }
    }

    #[must_use]
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).expect("construction project serialization is infallible")
    }

    pub fn decode_strict(json: &str) -> Result<Self, ConstructionDecodeError> {
        let project =
            serde_json::from_str::<Self>(json).map_err(ConstructionDecodeError::Decode)?;
        project
            .validate()
            .map_err(ConstructionDecodeError::Invariant)?;
        Ok(project)
    }

    pub fn reserve_site(&mut self, now_ms: i64) -> Result<(), ConstructionMutationError> {
        self.require_stage(ConstructionStage::SiteReserved)?;
        self.stage = ConstructionStage::DeliverScaffold;
        self.updated_at_ms = now_ms;
        Ok(())
    }

    pub fn begin_transit(
        &mut self,
        content_id: &str,
        units: u32,
        now_ms: i64,
    ) -> Result<(), ConstructionMutationError> {
        if units == 0 {
            return Err(ConstructionMutationError::ZeroUnits);
        }
        let line = self.active_delivery_bill_mut()?.line_mut(content_id)?;
        if units > line.missing_units() {
            return Err(ConstructionMutationError::CargoOverfill);
        }
        line.in_transit_units += units;
        self.updated_at_ms = now_ms;
        Ok(())
    }

    pub fn deliver_transit(
        &mut self,
        content_id: &str,
        units: u32,
        now_ms: i64,
    ) -> Result<(), ConstructionMutationError> {
        if units == 0 {
            return Err(ConstructionMutationError::ZeroUnits);
        }
        let line = self.active_delivery_bill_mut()?.line_mut(content_id)?;
        if units > line.in_transit_units {
            return Err(ConstructionMutationError::CargoNotInTransit);
        }
        line.in_transit_units -= units;
        line.delivered_units += units;
        self.updated_at_ms = now_ms;
        Ok(())
    }

    pub fn interrupt_transit(
        &mut self,
        content_id: &str,
        units: u32,
        cause: ConstructionRecoveryCause,
        now_ms: i64,
    ) -> Result<ConstructionRecovery, ConstructionMutationError> {
        if units == 0 {
            return Err(ConstructionMutationError::ZeroUnits);
        }
        let line = self.active_delivery_bill_mut()?.line_mut(content_id)?;
        if units > line.in_transit_units {
            return Err(ConstructionMutationError::CargoNotInTransit);
        }
        line.in_transit_units -= units;
        self.updated_at_ms = now_ms;
        Ok(ConstructionRecovery {
            content_id: content_id.to_owned(),
            units,
            cause,
        })
    }

    pub fn begin_stage_work(&mut self, now_ms: i64) -> Result<(), ConstructionMutationError> {
        let duration = match self.stage {
            ConstructionStage::DeliverScaffold => {
                stage_work_durations(self.original_total_work_ms).0
            }
            ConstructionStage::DeliverStructure => {
                stage_work_durations(self.original_total_work_ms).1
            }
            ConstructionStage::DeliverFitOut => stage_work_durations(self.original_total_work_ms).2,
            _ => return Err(ConstructionMutationError::NotDeliveryStage),
        };
        if !self.active_delivery_bill()?.is_fully_delivered() {
            return Err(ConstructionMutationError::CargoIncomplete);
        }
        self.active_delivery_bill_mut()?.consume_all();
        self.stage = match self.stage {
            ConstructionStage::DeliverScaffold => ConstructionStage::BuildScaffold,
            ConstructionStage::DeliverStructure => ConstructionStage::BuildStructure,
            ConstructionStage::DeliverFitOut => ConstructionStage::BuildFitOut,
            _ => unreachable!("delivery stage checked above"),
        };
        self.stage_work_remaining_ms = duration;
        self.updated_at_ms = now_ms;
        Ok(())
    }

    pub fn advance_work(
        &mut self,
        elapsed_ms: u64,
        now_ms: i64,
    ) -> Result<WorkAdvance, ConstructionMutationError> {
        if !self.stage.is_labor() {
            return Err(ConstructionMutationError::NotLaborStage);
        }
        let applied_ms = elapsed_ms.min(self.stage_work_remaining_ms);
        self.stage_work_remaining_ms -= applied_ms;
        let completed_stage = self.stage_work_remaining_ms == 0;
        if completed_stage {
            self.stage = match self.stage {
                ConstructionStage::BuildScaffold => ConstructionStage::DeliverStructure,
                ConstructionStage::BuildStructure => ConstructionStage::DeliverFitOut,
                ConstructionStage::BuildFitOut => ConstructionStage::Operational,
                _ => unreachable!("labor stage checked above"),
            };
        }
        self.updated_at_ms = now_ms;
        Ok(WorkAdvance {
            applied_ms,
            completed_stage,
            next_stage: self.stage,
        })
    }

    pub fn apply_accepted_clicks(
        &mut self,
        accepted_clicks: u32,
        now_ms: i64,
    ) -> Result<WorkAdvance, ConstructionMutationError> {
        let requested_ms = u64::from(accepted_clicks)
            .checked_mul(1_000)
            .ok_or(ConstructionMutationError::ArithmeticOverflow)?;
        let result = self.advance_work(requested_ms, now_ms)?;
        self.accepted_click_work_ms = self
            .accepted_click_work_ms
            .checked_add(result.applied_ms)
            .ok_or(ConstructionMutationError::ArithmeticOverflow)?;
        Ok(result)
    }

    pub fn cancel(
        &mut self,
        now_ms: i64,
    ) -> Result<ConstructionSalvage, ConstructionMutationError> {
        if self.stage.is_terminal() {
            return Err(ConstructionMutationError::TerminalProject);
        }
        let salvage = ConstructionSalvage {
            scaffold: self.bills.scaffold.drain_unconsumed(),
            structure: self.bills.structure.drain_unconsumed(),
            fit_out: self.bills.fit_out.drain_unconsumed(),
        };
        self.stage = ConstructionStage::Cancelled;
        self.stage_work_remaining_ms = 0;
        self.updated_at_ms = now_ms;
        Ok(salvage)
    }

    #[must_use]
    pub fn active_delivery_bill(
        &self,
    ) -> Result<&ConstructionStageBill, ConstructionMutationError> {
        match self.stage {
            ConstructionStage::DeliverScaffold => Ok(&self.bills.scaffold),
            ConstructionStage::DeliverStructure => Ok(&self.bills.structure),
            ConstructionStage::DeliverFitOut => Ok(&self.bills.fit_out),
            _ => Err(ConstructionMutationError::NotDeliveryStage),
        }
    }

    fn active_delivery_bill_mut(
        &mut self,
    ) -> Result<&mut ConstructionStageBill, ConstructionMutationError> {
        match self.stage {
            ConstructionStage::DeliverScaffold => Ok(&mut self.bills.scaffold),
            ConstructionStage::DeliverStructure => Ok(&mut self.bills.structure),
            ConstructionStage::DeliverFitOut => Ok(&mut self.bills.fit_out),
            _ => Err(ConstructionMutationError::NotDeliveryStage),
        }
    }

    fn require_stage(&self, expected: ConstructionStage) -> Result<(), ConstructionMutationError> {
        if self.stage == expected {
            Ok(())
        } else {
            Err(ConstructionMutationError::WrongStage {
                expected,
                actual: self.stage,
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkAdvance {
    pub applied_ms: u64,
    pub completed_stage: bool,
    pub next_stage: ConstructionStage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructionSalvage {
    pub scaffold: Vec<SalvagedCargo>,
    pub structure: Vec<SalvagedCargo>,
    pub fit_out: Vec<SalvagedCargo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionRecoveryCause {
    CarrierDeath,
    RouteLoss,
    Refusal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConstructionRecovery {
    pub content_id: String,
    pub units: u32,
    pub cause: ConstructionRecoveryCause,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SalvagedCargo {
    pub content_id: String,
    pub delivered_units: u32,
    pub in_transit_units: u32,
}

#[must_use]
pub const fn stage_work_durations(total_ms: u64) -> (u64, u64, u64) {
    let scaffold = total_ms / 5;
    let structure = ((total_ms as u128 * 3) / 5) as u64;
    let fit_out = total_ms - scaffold - structure;
    (scaffold, structure, fit_out)
}

/// Exact plan formula, rounded up to the next millisecond so an upgrade never
/// completes faster than its specified duration.
#[must_use]
pub fn building_upgrade_duration_ms(target_level: u32) -> Option<u64> {
    let index = usize::try_from(target_level.checked_sub(2)?).ok()?;
    BUILDING_UPGRADE_DURATION_MS.get(index).copied()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstructionInvariantError {
    UnsupportedVersion { found: u32 },
    EmptyProjectId,
    InvalidTargetLevel,
    ZeroDuration,
    ZeroStageDuration,
    InvalidClickWork,
    MissingBuildingType,
    UnexpectedBuildingType,
    InvalidFootprint,
    NonCanonicalBuildingFootprint,
    EmptyStageBill,
    EmptyCargoLine,
    EmptyContentId,
    DuplicateContentId { content_id: String },
    NonCanonicalCargoOrder,
    CargoOverfilled { content_id: String },
    UnknownContentId { content_id: String },
    InvalidScaffoldMaterial,
    UpgradeRequiresDevelopedScaffold,
    InvalidUpgradeDuration,
    InvalidHoleWorkFootprint,
    DeliveryHasLaborProgress,
    TerminalHasRemainingWork,
    InvalidStageProgress,
    InvalidStageCargoState,
}

#[derive(Debug)]
pub enum ConstructionDecodeError {
    Decode(serde_json::Error),
    Invariant(ConstructionInvariantError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstructionMutationError {
    WrongStage {
        expected: ConstructionStage,
        actual: ConstructionStage,
    },
    NotDeliveryStage,
    NotLaborStage,
    UnexpectedContent {
        content_id: String,
    },
    ZeroUnits,
    CargoOverfill,
    CargoNotInTransit,
    CargoIncomplete,
    ArithmeticOverflow,
    TerminalProject,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial_tasks::{Rect, TilePoint};

    fn bill(content_id: &str, units: u32) -> ConstructionStageBill {
        ConstructionStageBill::new([ConstructionCargoLine::new(content_id, units)])
    }

    fn project(building_type: BuildingType) -> ConstructionProject {
        ConstructionProject::new(
            "construction:test",
            ConstructionTargetKind::Building,
            Some(building_type),
            1,
            ScaffoldTier::Basic,
            TaskFootprint::rectangular(
                Rect::try_new(TilePoint { x: 4, y: 7 }, 3, 3).expect("valid rectangle"),
            ),
            ConstructionBills {
                scaffold: bill("resource_logs", 4),
                structure: bill("resource_stone", 6),
                fit_out: bill("resource_cloth", 2),
            },
            100_000,
            10,
        )
        .expect("valid project")
    }

    fn deliver_active(
        project: &mut ConstructionProject,
        content_id: &str,
        units: u32,
        now_ms: i64,
    ) {
        project
            .begin_transit(content_id, units, now_ms)
            .expect("transit begins");
        project
            .deliver_transit(content_id, units, now_ms + 1)
            .expect("cargo arrives");
    }

    #[test]
    fn workshop_project_preserves_complete_three_by_three_footprint() {
        let project = project(BuildingType::Workshop);
        assert_eq!(project.footprint.width, 3);
        assert_eq!(project.footprint.height, 3);
        assert_eq!(project.footprint.tiles.len(), 9);
    }

    #[test]
    fn stages_require_their_own_cargo_and_split_work_twenty_sixty_twenty() {
        let mut project = project(BuildingType::Workshop);
        project.reserve_site(20).expect("site reserves");
        assert_eq!(project.stage, ConstructionStage::DeliverScaffold);
        assert_eq!(
            project.begin_stage_work(21),
            Err(ConstructionMutationError::CargoIncomplete)
        );

        deliver_active(&mut project, "resource_logs", 4, 30);
        project.begin_stage_work(40).expect("scaffold begins");
        assert_eq!(project.stage_work_remaining_ms, 20_000);
        let advance = project
            .advance_work(20_000, 50)
            .expect("scaffold completes");
        assert!(advance.completed_stage);
        assert_eq!(advance.next_stage, ConstructionStage::DeliverStructure);

        deliver_active(&mut project, "resource_stone", 6, 60);
        project.begin_stage_work(70).expect("structure begins");
        assert_eq!(project.stage_work_remaining_ms, 60_000);
        project
            .advance_work(60_000, 80)
            .expect("structure completes");

        deliver_active(&mut project, "resource_cloth", 2, 90);
        project.begin_stage_work(100).expect("fit-out begins");
        assert_eq!(project.stage_work_remaining_ms, 20_000);
        project
            .advance_work(20_000, 110)
            .expect("fit-out completes");
        assert_eq!(project.stage, ConstructionStage::Operational);
    }

    #[test]
    fn accepted_clicks_remove_one_second_each_without_overfill() {
        let mut project = project(BuildingType::Workshop);
        project.reserve_site(20).expect("site reserves");
        deliver_active(&mut project, "resource_logs", 4, 30);
        project.begin_stage_work(40).expect("scaffold begins");
        let advance = project
            .apply_accepted_clicks(25, 50)
            .expect("clicks apply to active labor");
        assert_eq!(advance.applied_ms, 20_000);
        assert!(advance.completed_stage);
        assert_eq!(project.accepted_click_work_ms, 20_000);
    }

    #[test]
    fn cancellation_reports_only_unconsumed_physical_cargo() {
        let mut project = project(BuildingType::Workshop);
        project.reserve_site(20).expect("site reserves");
        project
            .begin_transit("resource_logs", 3, 30)
            .expect("transit begins");
        project
            .deliver_transit("resource_logs", 2, 31)
            .expect("partial cargo arrives");
        let salvage = project.cancel(40).expect("cancellation succeeds once");
        assert_eq!(
            salvage.scaffold,
            vec![SalvagedCargo {
                content_id: "resource_logs".to_owned(),
                delivered_units: 2,
                in_transit_units: 1,
            }]
        );
        assert_eq!(project.stage, ConstructionStage::Cancelled);
    }

    #[test]
    fn scaffold_tier_rejects_wrong_material_family() {
        let mut project = project(BuildingType::Workshop);
        project.scaffold_tier = ScaffoldTier::Developed;
        assert_eq!(
            project.validate(),
            Err(ConstructionInvariantError::InvalidScaffoldMaterial)
        );
    }

    #[test]
    fn upgrade_duration_uses_locked_formula_and_stage_sum_is_exact() {
        let duration = building_upgrade_duration_ms(2).expect("level two upgrade");
        assert_eq!(duration, 8 * GAME_HOUR_MS);
        let (scaffold, structure, fit_out) = stage_work_durations(duration);
        assert_eq!(scaffold + structure + fit_out, duration);
    }
}
