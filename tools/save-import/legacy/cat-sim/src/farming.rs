//! Visible, staffed farm plots for the top-down colony (P12.4/P17).
//!
//! A plot advances on game time only while farmer work is available. Climate-biome
//! fertility scales that work, and a full harvest remains flowering until one whole
//! deterministic basket can fit in storage. This keeps offline acceleration, manual
//! farming, and future Farmer-officer automation on the same pure path.

use serde::{Deserialize, Serialize};

use crate::{pathfinding::TilePos, zones::ZoneRect};

/// Maximum edge of a designated farm rectangle.
pub const FARM_MAX_EDGE: i32 = 8;
/// Maximum persistent plots per colony.
pub const MAX_FARM_PLOTS: usize = 16;

/// Effective growth hours at each inclusive stage boundary.
pub const SPROUT_AT_HOURS: f64 = 2.0;
pub const GROWING_AT_HOURS: f64 = 6.0;
pub const MATURE_AT_HOURS: f64 = 12.0;
pub const FLOWERING_AT_HOURS: f64 = 18.0;
pub const HARVEST_AT_HOURS: f64 = 24.0;
/// Maximum crop units one farmer can carry from a plot in one physical basket.
/// Large plots therefore require repeated short trips and cannot teleport an entire
/// 8x8 harvest through one cat.
pub const FARM_BASKET_CAPACITY: f64 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CropKind {
    Catnip,
    Grain,
    Herb,
}

impl CropKind {
    /// Whole-resource units harvested per farm tile and growth cycle.
    #[must_use]
    pub const fn yield_per_tile(self) -> f64 {
        match self {
            Self::Catnip | Self::Herb => 1.0,
            Self::Grain => 2.0,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmStage {
    #[default]
    Soil,
    Sprout,
    Growing,
    Mature,
    Flowering,
}

/// Persisted physical work state for one designated plot.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmWorkPhase {
    #[default]
    WaitingForWorker,
    Traveling,
    Planting,
    Tending,
    Harvesting,
    Hauling,
    OutputBlocked,
}

/// A persistent, visible farm designation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FarmPlot {
    pub id: String,
    pub rect: ZoneRect,
    pub crop: CropKind,
    /// Game timestamp at which the current crop cycle was planted.
    pub planted_at: i64,
    pub stage: FarmStage,
    /// Fertility-scaled work accrued in the current crop cycle. Additive save field;
    /// older saves have no plots, while hand-authored plot fixtures safely start at 0.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub growth_hours: f64,
    /// Mean climate fertility validated for this footprint. Legacy rows use 0 and
    /// deterministically populate it on their next farming tick.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub fertility: f64,
    /// Living Field worker currently responsible for this plot. A worker is bound to
    /// at most one plot at a time and physically visits it before work advances.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    #[serde(default)]
    pub work_phase: FarmWorkPhase,
    /// Harvested produce still lying on the plot. It is intentionally absent from the
    /// colony aggregate until a cat carries it into finite compatible storage.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pending_output: f64,
}

impl FarmPlot {
    #[must_use]
    pub fn tiles(&self) -> u32 {
        rect_area(self.rect)
    }

    #[must_use]
    pub fn harvest_amount(&self) -> f64 {
        f64::from(self.tiles()) * self.crop.yield_per_tile()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FarmAdvance {
    pub next_growth_hours: f64,
    pub next_stage: FarmStage,
    pub harvests: u32,
    pub amount_produced: f64,
}

/// Advance one crop with no clock, RNG, or world dependency.
///
/// `output_headroom` is headroom across the selected destination and colony-wide
/// resource capacity. Harvests are whole baskets: if one does not fit, growth banks at
/// the flowering boundary instead of deleting surplus produce.
#[must_use]
pub fn advance_farm(
    plot: &FarmPlot,
    elapsed_sec: f64,
    fertility: f64,
    has_farmer_work: bool,
    yield_multiplier: f64,
    output_headroom: f64,
) -> FarmAdvance {
    let old_progress = non_negative(plot.growth_hours).min(HARVEST_AT_HOURS);
    if !has_farmer_work || elapsed_sec <= 0.0 || fertility <= 0.0 {
        return FarmAdvance {
            next_growth_hours: old_progress,
            next_stage: stage_at(old_progress),
            harvests: 0,
            amount_produced: 0.0,
        };
    }

    let basket = plot.harvest_amount() * non_negative(yield_multiplier);
    let mut progress = old_progress + non_negative(elapsed_sec) / 3600.0 * fertility;
    let cycles_by_time = (progress / HARVEST_AT_HOURS).floor();
    let cycles_by_capacity = if basket > 0.0 {
        (non_negative(output_headroom) / basket).floor()
    } else {
        0.0
    };
    let cycles = cycles_by_time.min(cycles_by_capacity).max(0.0);
    progress = (progress - cycles * HARVEST_AT_HOURS).min(HARVEST_AT_HOURS);
    let harvests = cycles.min(f64::from(u32::MAX)) as u32;

    FarmAdvance {
        next_growth_hours: progress,
        next_stage: stage_at(progress),
        harvests,
        amount_produced: f64::from(harvests) * basket,
    }
}

#[must_use]
pub const fn stage_at(growth_hours: f64) -> FarmStage {
    if growth_hours >= FLOWERING_AT_HOURS {
        FarmStage::Flowering
    } else if growth_hours >= MATURE_AT_HOURS {
        FarmStage::Mature
    } else if growth_hours >= GROWING_AT_HOURS {
        FarmStage::Growing
    } else if growth_hours >= SPROUT_AT_HOURS {
        FarmStage::Sprout
    } else {
        FarmStage::Soil
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FarmPlacementError {
    InvalidRect,
    TooLarge,
    LimitReached,
    OutsideClaim,
    VillageInterior,
    Occupied,
    Overlap,
    Barren,
}

/// Validate a plot and return its mean climate fertility.
///
/// Every tile must be claimed, clear, and farmable (`fertility > 0.0`). The closure
/// arguments keep this rule usable by both action validation and deterministic tests.
pub fn validate_placement(
    rect: ZoneRect,
    existing: &[FarmPlot],
    is_claimed: impl Fn(TilePos) -> bool,
    is_village_interior: impl Fn(TilePos) -> bool,
    is_occupied: impl Fn(TilePos) -> bool,
    fertility_at: impl Fn(TilePos) -> f64,
) -> Result<f64, FarmPlacementError> {
    if rect.x1 > rect.x2 || rect.y1 > rect.y2 {
        return Err(FarmPlacementError::InvalidRect);
    }
    if rect.x2 - rect.x1 + 1 > FARM_MAX_EDGE || rect.y2 - rect.y1 + 1 > FARM_MAX_EDGE {
        return Err(FarmPlacementError::TooLarge);
    }
    if existing.len() >= MAX_FARM_PLOTS {
        return Err(FarmPlacementError::LimitReached);
    }
    if existing.iter().any(|plot| rects_overlap(rect, plot.rect)) {
        return Err(FarmPlacementError::Overlap);
    }

    let mut total_fertility = 0.0;
    let mut count = 0_u32;
    for tile in rect_tiles(rect) {
        if !is_claimed(tile) {
            return Err(FarmPlacementError::OutsideClaim);
        }
        if is_village_interior(tile) {
            return Err(FarmPlacementError::VillageInterior);
        }
        if is_occupied(tile) {
            return Err(FarmPlacementError::Occupied);
        }
        let fertility = fertility_at(tile);
        if !fertility.is_finite() || fertility <= 0.0 {
            return Err(FarmPlacementError::Barren);
        }
        total_fertility += fertility;
        count += 1;
    }
    Ok(total_fertility / f64::from(count))
}

#[must_use]
pub const fn rects_overlap(a: ZoneRect, b: ZoneRect) -> bool {
    a.x1 <= b.x2 && a.x2 >= b.x1 && a.y1 <= b.y2 && a.y2 >= b.y1
}

pub fn rect_tiles(rect: ZoneRect) -> impl Iterator<Item = TilePos> {
    (rect.y1..=rect.y2).flat_map(move |y| (rect.x1..=rect.x2).map(move |x| TilePos { x, y }))
}

#[must_use]
pub const fn rect_area(rect: ZoneRect) -> u32 {
    let width = rect.x2 - rect.x1 + 1;
    let height = rect.y2 - rect.y1 + 1;
    if width <= 0 || height <= 0 {
        0
    } else {
        (width * height) as u32
    }
}

fn non_negative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn is_zero(value: &f64) -> bool {
    *value == 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x1: i32, y1: i32, x2: i32, y2: i32) -> ZoneRect {
        ZoneRect { x1, y1, x2, y2 }
    }

    fn plot(crop: CropKind) -> FarmPlot {
        FarmPlot {
            id: "farm-1".to_owned(),
            rect: rect(1, 1, 2, 2),
            crop,
            planted_at: 1_000,
            stage: FarmStage::Soil,
            growth_hours: 0.0,
            fertility: 1.0,
            worker_id: None,
            work_phase: FarmWorkPhase::WaitingForWorker,
            pending_output: 0.0,
        }
    }

    #[test]
    fn crop_and_stage_wire_literals_are_exact() {
        assert_eq!(
            serde_json::to_string(&CropKind::Catnip).unwrap(),
            "\"catnip\""
        );
        assert_eq!(
            serde_json::to_string(&CropKind::Grain).unwrap(),
            "\"grain\""
        );
        assert_eq!(serde_json::to_string(&CropKind::Herb).unwrap(), "\"herb\"");
        assert_eq!(serde_json::to_string(&FarmStage::Soil).unwrap(), "\"soil\"");
        assert_eq!(
            serde_json::to_string(&FarmStage::Sprout).unwrap(),
            "\"sprout\""
        );
        assert_eq!(
            serde_json::to_string(&FarmStage::Growing).unwrap(),
            "\"growing\""
        );
        assert_eq!(
            serde_json::to_string(&FarmStage::Mature).unwrap(),
            "\"mature\""
        );
        assert_eq!(
            serde_json::to_string(&FarmStage::Flowering).unwrap(),
            "\"flowering\""
        );
    }

    #[test]
    fn accelerated_stage_boundaries_are_inclusive_and_exact() {
        let plot = plot(CropKind::Herb);
        for (hours, stage) in [
            (1.999, FarmStage::Soil),
            (2.0, FarmStage::Sprout),
            (6.0, FarmStage::Growing),
            (12.0, FarmStage::Mature),
            (18.0, FarmStage::Flowering),
        ] {
            let step = advance_farm(&plot, hours * 3600.0, 1.0, true, 1.0, 100.0);
            assert_eq!(step.next_stage, stage, "boundary {hours}");
        }
    }

    #[test]
    fn fertility_scales_growth_and_barren_ground_cannot_advance() {
        let plot = plot(CropKind::Catnip);
        let rich = advance_farm(&plot, 12.0 * 3600.0, 1.5, true, 1.0, 100.0);
        let poor = advance_farm(&plot, 12.0 * 3600.0, 0.5, true, 1.0, 100.0);
        let barren = advance_farm(&plot, 12.0 * 3600.0, 0.0, true, 1.0, 100.0);
        assert_eq!(rich.next_stage, FarmStage::Flowering);
        assert_eq!(poor.next_stage, FarmStage::Growing);
        assert_eq!(barren.next_stage, FarmStage::Soil);
    }

    #[test]
    fn farming_requires_work_and_harvests_whole_baskets() {
        let plot = plot(CropKind::Grain); // 4 tiles * 2 grain = 8
        let idle = advance_farm(&plot, 48.0 * 3600.0, 1.0, false, 1.0, 100.0);
        assert_eq!(idle.next_growth_hours, 0.0);
        assert_eq!(idle.harvests, 0);

        let worked = advance_farm(&plot, 48.0 * 3600.0, 1.0, true, 1.0, 100.0);
        assert_eq!(worked.harvests, 2);
        assert_eq!(worked.amount_produced, 16.0);
        assert_eq!(worked.next_growth_hours, 0.0);
    }

    #[test]
    fn full_storage_banks_a_flowering_harvest_without_loss() {
        let plot = plot(CropKind::Grain);
        let stalled = advance_farm(&plot, 24.0 * 3600.0, 1.0, true, 1.0, 7.99);
        assert_eq!(stalled.harvests, 0);
        assert_eq!(stalled.next_growth_hours, HARVEST_AT_HOURS);
        assert_eq!(stalled.next_stage, FarmStage::Flowering);

        let resumed = FarmPlot {
            growth_hours: stalled.next_growth_hours,
            stage: stalled.next_stage,
            ..plot
        };
        let harvest = advance_farm(&resumed, 1.0, 1.0, true, 1.0, 8.0);
        assert_eq!(harvest.harvests, 1);
        assert!(harvest.next_growth_hours > 0.0);
    }

    #[test]
    fn placement_rejects_overlap_occupation_unclaimed_and_barren_tiles() {
        let existing = vec![FarmPlot {
            rect: rect(2, 2, 3, 3),
            ..plot(CropKind::Herb)
        }];
        assert_eq!(
            validate_placement(
                rect(1, 1, 2, 2),
                &existing,
                |_| true,
                |_| false,
                |_| false,
                |_| 1.0,
            ),
            Err(FarmPlacementError::Overlap)
        );
        assert_eq!(
            validate_placement(
                rect(5, 5, 5, 5),
                &[],
                |_| false,
                |_| false,
                |_| false,
                |_| 1.0,
            ),
            Err(FarmPlacementError::OutsideClaim)
        );
        assert_eq!(
            validate_placement(
                rect(5, 5, 5, 5),
                &[],
                |_| true,
                |_| false,
                |_| true,
                |_| 1.0,
            ),
            Err(FarmPlacementError::Occupied)
        );
        assert_eq!(
            validate_placement(
                rect(5, 5, 5, 5),
                &[],
                |_| true,
                |_| false,
                |_| false,
                |_| 0.0,
            ),
            Err(FarmPlacementError::Barren)
        );
        assert_eq!(
            validate_placement(
                rect(5, 5, 5, 5),
                &[],
                |_| true,
                |_| true,
                |_| false,
                |_| 1.0,
            ),
            Err(FarmPlacementError::VillageInterior)
        );
    }

    #[test]
    fn placement_uses_mean_fertility_and_rejects_bad_rects() {
        let fertility = validate_placement(
            rect(1, 1, 2, 1),
            &[],
            |_| true,
            |_| false,
            |_| false,
            |tile| if tile.x == 1 { 0.5 } else { 1.5 },
        )
        .unwrap();
        assert_eq!(fertility, 1.0);
        assert_eq!(
            validate_placement(
                rect(2, 1, 1, 1),
                &[],
                |_| true,
                |_| false,
                |_| false,
                |_| 1.0,
            ),
            Err(FarmPlacementError::InvalidRect)
        );
        assert_eq!(
            validate_placement(
                rect(0, 0, FARM_MAX_EDGE, 0),
                &[],
                |_| true,
                |_| false,
                |_| false,
                |_| 1.0
            ),
            Err(FarmPlacementError::TooLarge)
        );
    }

    #[test]
    fn identical_inputs_are_bit_deterministic() {
        let plot = FarmPlot {
            growth_hours: 11.25,
            ..plot(CropKind::Catnip)
        };
        let first = advance_farm(&plot, 45_678.0, 1.1, true, 1.0, 80.0);
        let second = advance_farm(&plot, 45_678.0, 1.1, true, 1.0, 80.0);
        assert_eq!(first, second);
        assert_eq!(
            first.next_growth_hours.to_bits(),
            second.next_growth_hours.to_bits()
        );
    }

    #[test]
    fn yield_multiplier_scales_whole_harvest_and_capacity_gate() {
        let plot = plot(CropKind::Catnip); // 4 tiles * 1.5 = 6 catnip
        let blocked = advance_farm(&plot, 24.0 * 3600.0, 1.0, true, 1.5, 5.99);
        assert_eq!(blocked.harvests, 0);
        assert_eq!(blocked.next_stage, FarmStage::Flowering);

        let harvested = advance_farm(&plot, 24.0 * 3600.0, 1.0, true, 1.5, 6.0);
        assert_eq!(harvested.harvests, 1);
        assert_eq!(harvested.amount_produced, 6.0);
    }
}
