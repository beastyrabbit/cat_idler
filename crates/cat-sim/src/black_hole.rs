//! Pure Black Hole domain authority.
//!
//! This leaf is intentionally inert: it owns validation, deterministic intake
//! ordering, reward accounting, and upgrade recipes, but it is not wired into
//! `world_tick` yet.

use std::{cmp::Ordering, collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

use crate::{
    items::{Item, ItemKind, MAX_QUALITY},
    stockpiles::ResourceKind,
};

pub const AXIS_MIN: u8 = 0;
pub const AXIS_MAX: u8 = 10;
pub const VALUE_MICROS: u64 = 1_000_000;
pub const MAX_CREDIT_PER_OPENING: u32 = 1;
pub const BLACK_HOLE_RUNTIME_SCHEMA_VERSION: u32 = 1;
pub const INTAKE_COOLDOWN_GAME_MS: i64 = 40 * 60 * 1_000;
pub const LEADER_REVIEW_INTERVAL_MS: i64 = 12 * 60 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlackHoleError {
    AxisOutOfRange { axis: BlackHoleAxis, value: u8 },
    OrderOutOfRange { order: u32 },
    ZeroQuantity,
    UpgradeAtMaxLevel { axis: BlackHoleAxis },
}

impl fmt::Display for BlackHoleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AxisOutOfRange { axis, value } => {
                write!(f, "{axis:?} axis level {value} is outside 0..=10")
            }
            Self::OrderOutOfRange { order } => write!(f, "feed order {order} is outside 0..=110"),
            Self::ZeroQuantity => write!(f, "feed candidates must have a positive quantity"),
            Self::UpgradeAtMaxLevel { axis } => write!(f, "{axis:?} axis is already at max level"),
        }
    }
}

impl std::error::Error for BlackHoleError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlackHoleAxis {
    Width,
    Depth,
    Darkness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BlackHoleAxes {
    pub width: u8,
    pub depth: u8,
    pub darkness: u8,
}

impl Default for BlackHoleAxes {
    fn default() -> Self {
        Self {
            width: AXIS_MIN,
            depth: AXIS_MIN,
            darkness: AXIS_MIN,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BlackHoleAxesWire {
    width: u8,
    depth: u8,
    darkness: u8,
}

impl<'de> Deserialize<'de> for BlackHoleAxes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = BlackHoleAxesWire::deserialize(deserializer)?;
        Self::new(wire.width, wire.depth, wire.darkness).map_err(serde::de::Error::custom)
    }
}

impl BlackHoleAxes {
    pub fn new(width: u8, depth: u8, darkness: u8) -> Result<Self, BlackHoleError> {
        validate_axis(BlackHoleAxis::Width, width)?;
        validate_axis(BlackHoleAxis::Depth, depth)?;
        validate_axis(BlackHoleAxis::Darkness, darkness)?;
        Ok(Self {
            width,
            depth,
            darkness,
        })
    }

    #[must_use]
    pub const fn intake_width(self) -> usize {
        intake_width(self.width)
    }

    #[must_use]
    pub const fn max_order(self) -> u32 {
        max_order(self.depth)
    }

    #[must_use]
    pub const fn max_quality(self) -> u8 {
        max_quality_for_darkness(self.darkness)
    }

    #[must_use]
    pub const fn level(self, axis: BlackHoleAxis) -> u8 {
        match axis {
            BlackHoleAxis::Width => self.width,
            BlackHoleAxis::Depth => self.depth,
            BlackHoleAxis::Darkness => self.darkness,
        }
    }

    pub fn raise(&mut self, axis: BlackHoleAxis) -> Result<u8, BlackHoleError> {
        let current = self.level(axis);
        if current >= AXIS_MAX {
            return Err(BlackHoleError::UpgradeAtMaxLevel { axis });
        }
        let next = current + 1;
        match axis {
            BlackHoleAxis::Width => self.width = next,
            BlackHoleAxis::Depth => self.depth = next,
            BlackHoleAxis::Darkness => self.darkness = next,
        }
        Ok(next)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleFeedOrder {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    pub resource: ResourceKind,
    pub target_units: u32,
    pub delivered_units: u32,
    pub credited_units: u32,
    pub credited_value_micros: u64,
    pub created_at: i64,
}

impl BlackHoleFeedOrder {
    #[must_use]
    pub const fn remaining_units(&self) -> u32 {
        self.target_units.saturating_sub(self.credited_units)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleUpgradeProject {
    pub axis: BlackHoleAxis,
    pub target_level: u8,
    pub job_id: String,
    pub started_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleRuntime {
    pub schema_version: u32,
    pub building_id: String,
    #[serde(default)]
    pub axes: BlackHoleAxes,
    #[serde(default)]
    pub intake: IntakeState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_feed: Option<BlackHoleFeedOrder>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_opening_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub urged_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_upgrade: Option<BlackHoleUpgradeProject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_review_at: Option<i64>,
}

impl BlackHoleRuntime {
    #[must_use]
    pub fn for_building(building_id: impl Into<String>) -> Self {
        Self {
            schema_version: BLACK_HOLE_RUNTIME_SCHEMA_VERSION,
            building_id: building_id.into(),
            axes: BlackHoleAxes::default(),
            intake: IntakeState::default(),
            active_feed: None,
            next_opening_at: None,
            urged_at: None,
            active_upgrade: None,
            next_review_at: None,
        }
    }

    #[must_use]
    pub fn next_upgrade_axis(&self, researched: BlackHoleAxes) -> Option<BlackHoleAxis> {
        [
            BlackHoleAxis::Width,
            BlackHoleAxis::Depth,
            BlackHoleAxis::Darkness,
        ]
        .into_iter()
        .filter(|axis| self.axes.level(*axis) < researched.level(*axis))
        .min_by_key(|axis| (self.axes.level(*axis), *axis))
    }
}

pub fn validate_axis(axis: BlackHoleAxis, value: u8) -> Result<(), BlackHoleError> {
    if (AXIS_MIN..=AXIS_MAX).contains(&value) {
        Ok(())
    } else {
        Err(BlackHoleError::AxisOutOfRange { axis, value })
    }
}

#[must_use]
pub const fn intake_width(width_level: u8) -> usize {
    1 + width_level as usize
}

#[must_use]
pub const fn max_order(depth_level: u8) -> u32 {
    10 * (1 + depth_level as u32)
}

#[must_use]
pub const fn max_quality_for_darkness(darkness_level: u8) -> u8 {
    match darkness_level {
        0..=4 => 0,
        5..=6 => 1,
        7..=8 => 2,
        9 => 3,
        _ => MAX_QUALITY,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum FeedKind {
    Resource { resource: ResourceKind },
    Item { item: Item },
}

impl FeedKind {
    #[must_use]
    pub fn unit_value_micros(self) -> u64 {
        match self {
            Self::Resource { resource } => resource_unit_value_micros(resource),
            Self::Item { item } => u64::from(item.value()) * (VALUE_MICROS / 10),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedSource {
    Local,
    Child,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildLoad {
    pub child_id: u64,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedCandidate {
    pub kind: FeedKind,
    pub order: u32,
    pub quantity: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_loads: Vec<ChildLoad>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FeedCandidateWire {
    kind: FeedKind,
    order: u32,
    quantity: u32,
    #[serde(default)]
    child_loads: Vec<ChildLoad>,
}

impl<'de> Deserialize<'de> for FeedCandidate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = FeedCandidateWire::deserialize(deserializer)?;
        Self::new(wire.kind, wire.order, wire.quantity, wire.child_loads)
            .map_err(serde::de::Error::custom)
    }
}

impl FeedCandidate {
    pub fn new(
        kind: FeedKind,
        order: u32,
        quantity: u32,
        child_loads: Vec<ChildLoad>,
    ) -> Result<Self, BlackHoleError> {
        if order > max_order(AXIS_MAX) {
            return Err(BlackHoleError::OrderOutOfRange { order });
        }
        if quantity == 0 && child_loads.iter().all(|load| load.quantity == 0) {
            return Err(BlackHoleError::ZeroQuantity);
        }
        Ok(Self {
            kind,
            order,
            quantity,
            child_loads,
        })
    }

    #[must_use]
    pub fn resource(resource: ResourceKind, order: u32, quantity: u32) -> Self {
        Self::new(FeedKind::Resource { resource }, order, quantity, Vec::new())
            .expect("literal resource candidate is valid")
    }

    #[must_use]
    pub fn item(item: Item, order: u32, quantity: u32) -> Self {
        Self::new(FeedKind::Item { item }, order, quantity, Vec::new())
            .expect("literal item candidate is valid")
    }

    #[must_use]
    pub fn total_quantity(&self) -> u32 {
        self.quantity.saturating_add(
            self.child_loads
                .iter()
                .map(|load| load.quantity)
                .fold(0_u32, u32::saturating_add),
        )
    }

    #[must_use]
    pub fn is_unlocked_by(&self, axes: BlackHoleAxes) -> bool {
        self.order <= axes.max_order()
            && match self.kind {
                FeedKind::Resource { resource } => resource_darkness_requirement(resource)
                    .is_some_and(|required| axes.darkness >= required),
                FeedKind::Item { item } => {
                    item_darkness_requirement(item.kind)
                        .is_some_and(|required| axes.darkness >= required)
                        && item.quality <= axes.max_quality()
                }
            }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeState {
    pub next_opening_index: u64,
    pub lifetime: LifetimeTotals,
}

impl Default for IntakeState {
    fn default() -> Self {
        Self::new()
    }
}

impl IntakeState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next_opening_index: 0,
            lifetime: LifetimeTotals::new(),
        }
    }

    pub fn intake(
        &mut self,
        axes: BlackHoleAxes,
        candidates: &mut [FeedCandidate],
    ) -> IntakeReport {
        let opening_index = self.next_opening_index;
        let mut report = IntakeReport {
            width: axes.intake_width(),
            max_order: axes.max_order(),
            max_quality: axes.max_quality(),
            ..IntakeReport::default()
        };
        let mut openings_remaining = axes.intake_width();

        while openings_remaining > 0 {
            let Some(index) = next_candidate_index(axes, candidates) else {
                break;
            };
            let credit = self.credit_one(opening_index, index, candidates);
            openings_remaining -= 1;
            report.total_quantity = report.total_quantity.saturating_add(credit.quantity);
            report.total_value_micros = report
                .total_value_micros
                .saturating_add(credit.total_value_micros);
            report.reward_micros = report.reward_micros.saturating_add(credit.reward_micros);
            report.credits.push(credit);
        }
        if !report.credits.is_empty() {
            self.next_opening_index = self.next_opening_index.saturating_add(1);
            self.lifetime.openings = self.lifetime.openings.saturating_add(1);
        }

        report
    }

    fn credit_one(
        &mut self,
        opening_index: u64,
        index: usize,
        candidates: &mut [FeedCandidate],
    ) -> Credit {
        let candidate = candidates
            .get_mut(index)
            .expect("candidate index was selected from slice");
        let (source, child_id) = consume_one(candidate);
        let unit_value_micros = candidate.kind.unit_value_micros();
        let credit = Credit {
            opening_index,
            candidate_index: index,
            kind: candidate.kind,
            source,
            child_id,
            order: candidate.order,
            quantity: MAX_CREDIT_PER_OPENING,
            unit_value_micros,
            total_value_micros: unit_value_micros,
            reward_micros: reward_micros_for_value(unit_value_micros),
        };
        self.lifetime.apply(&credit);
        credit
    }
}

fn next_candidate_index(axes: BlackHoleAxes, candidates: &[FeedCandidate]) -> Option<usize> {
    candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.total_quantity() > 0 && candidate.is_unlocked_by(axes))
        .min_by(|(left_index, left), (right_index, right)| {
            candidate_ordering(left, *left_index, right, *right_index)
        })
        .map(|(index, _)| index)
}

fn candidate_ordering(
    left: &FeedCandidate,
    left_index: usize,
    right: &FeedCandidate,
    right_index: usize,
) -> Ordering {
    left.order
        .cmp(&right.order)
        .then_with(|| left.kind.cmp(&right.kind))
        .then_with(|| left_index.cmp(&right_index))
}

fn consume_one(candidate: &mut FeedCandidate) -> (FeedSource, Option<u64>) {
    if candidate.quantity > 0 {
        candidate.quantity -= 1;
        return (FeedSource::Local, None);
    }

    let load = candidate
        .child_loads
        .iter_mut()
        .find(|load| load.quantity > 0)
        .expect("candidate has positive total quantity");
    load.quantity -= 1;
    (FeedSource::Child, Some(load.child_id))
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeReport {
    pub width: usize,
    pub max_order: u32,
    pub max_quality: u8,
    pub total_quantity: u32,
    pub total_value_micros: u64,
    pub reward_micros: u64,
    pub credits: Vec<Credit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credit {
    pub opening_index: u64,
    pub candidate_index: usize,
    pub kind: FeedKind,
    pub source: FeedSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_id: Option<u64>,
    pub order: u32,
    pub quantity: u32,
    pub unit_value_micros: u64,
    pub total_value_micros: u64,
    pub reward_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifetimeTotals {
    pub quantity: u64,
    pub value_micros: u64,
    pub reward_micros: u64,
    pub openings: u64,
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        with = "feed_kind_totals_serde"
    )]
    pub by_kind: BTreeMap<FeedKind, u64>,
}

impl Default for LifetimeTotals {
    fn default() -> Self {
        Self::new()
    }
}

impl LifetimeTotals {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            quantity: 0,
            value_micros: 0,
            reward_micros: 0,
            openings: 0,
            by_kind: BTreeMap::new(),
        }
    }

    fn apply(&mut self, credit: &Credit) {
        self.quantity = self.quantity.saturating_add(u64::from(credit.quantity));
        self.value_micros = self.value_micros.saturating_add(credit.total_value_micros);
        self.reward_micros = self.reward_micros.saturating_add(credit.reward_micros);
        let entry = self.by_kind.entry(credit.kind).or_insert(0);
        *entry = entry.saturating_add(u64::from(credit.quantity));
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FeedKindTotal {
    kind: FeedKind,
    quantity: u64,
}

mod feed_kind_totals_serde {
    use super::{BTreeMap, FeedKind, FeedKindTotal};
    use serde::{Deserialize, Serialize};

    pub fn serialize<S>(totals: &BTreeMap<FeedKind, u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        totals
            .iter()
            .map(|(kind, quantity)| FeedKindTotal {
                kind: *kind,
                quantity: *quantity,
            })
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<BTreeMap<FeedKind, u64>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let entries = Vec::<FeedKindTotal>::deserialize(deserializer)?;
        let mut totals = BTreeMap::new();
        for entry in entries {
            if totals.insert(entry.kind, entry.quantity).is_some() {
                return Err(serde::de::Error::custom("duplicate feed kind total"));
            }
        }
        Ok(totals)
    }
}

#[must_use]
pub const fn reward_micros_for_value(value_micros: u64) -> u64 {
    value_micros
}

#[must_use]
pub const fn resource_unit_value_micros(resource: ResourceKind) -> u64 {
    match resource {
        ResourceKind::Food
        | ResourceKind::Fish
        | ResourceKind::Herbs
        | ResourceKind::Catnip
        | ResourceKind::Grain
        | ResourceKind::Materials
        | ResourceKind::Stone
        | ResourceKind::Logs
        | ResourceKind::Clay
        | ResourceKind::Sand
        | ResourceKind::Fibre
        | ResourceKind::Hide
        | ResourceKind::Bone
        | ResourceKind::Ore => VALUE_MICROS / 10,
        ResourceKind::Flour
        | ResourceKind::Preserves
        | ResourceKind::Medicine
        | ResourceKind::Brew
        | ResourceKind::Lumber
        | ResourceKind::Planks
        | ResourceKind::Blocks
        | ResourceKind::Refined
        | ResourceKind::Thread
        | ResourceKind::Cloth
        | ResourceKind::Leather
        | ResourceKind::Metal => 3 * (VALUE_MICROS / 10),
        ResourceKind::Gem => VALUE_MICROS / 2,
        ResourceKind::Water
        | ResourceKind::Tools
        | ResourceKind::Weapons
        | ResourceKind::Armor
        | ResourceKind::Blessings => 0,
    }
}

#[must_use]
pub const fn resource_darkness_requirement(resource: ResourceKind) -> Option<u8> {
    match resource {
        ResourceKind::Food | ResourceKind::Herbs | ResourceKind::Materials => Some(0),
        ResourceKind::Fish | ResourceKind::Grain | ResourceKind::Catnip => Some(1),
        ResourceKind::Logs | ResourceKind::Stone | ResourceKind::Clay | ResourceKind::Sand => {
            Some(2)
        }
        ResourceKind::Fibre | ResourceKind::Hide | ResourceKind::Bone | ResourceKind::Ore => {
            Some(3)
        }
        ResourceKind::Flour
        | ResourceKind::Lumber
        | ResourceKind::Planks
        | ResourceKind::Blocks
        | ResourceKind::Thread => Some(4),
        ResourceKind::Preserves | ResourceKind::Medicine | ResourceKind::Brew => Some(5),
        ResourceKind::Refined
        | ResourceKind::Cloth
        | ResourceKind::Leather
        | ResourceKind::Metal => Some(6),
        ResourceKind::Gem => Some(7),
        ResourceKind::Water
        | ResourceKind::Tools
        | ResourceKind::Weapons
        | ResourceKind::Armor
        | ResourceKind::Blessings => None,
    }
}

#[must_use]
pub const fn item_darkness_requirement(kind: ItemKind) -> Option<u8> {
    match kind {
        ItemKind::Mug | ItemKind::Bowl | ItemKind::Toy | ItemKind::Trinket => Some(2),
        ItemKind::Brick | ItemKind::Furniture | ItemKind::Clothing => Some(5),
        ItemKind::Tool => Some(7),
        ItemKind::Weapon => Some(8),
        ItemKind::Armor => Some(9),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeRecipe {
    pub axis: BlackHoleAxis,
    pub from_level: u8,
    pub to_level: u8,
    pub reward_cost_micros: u64,
    pub consumed_resources: Vec<ResourceRequirement>,
    pub consumed_tools: Vec<ToolRequirement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirement {
    pub resource: ResourceKind,
    pub quantity: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequirement {
    pub minimum_quality: u8,
    pub quantity: u32,
}

impl ToolRequirement {
    #[must_use]
    pub fn accepts(self, item: Item) -> bool {
        item.kind == ItemKind::Tool && item.quality >= self.minimum_quality
    }
}

pub fn upgrade_recipe(
    axes: BlackHoleAxes,
    axis: BlackHoleAxis,
) -> Result<UpgradeRecipe, BlackHoleError> {
    let from_level = match axis {
        BlackHoleAxis::Width => axes.width,
        BlackHoleAxis::Depth => axes.depth,
        BlackHoleAxis::Darkness => axes.darkness,
    };
    if from_level >= AXIS_MAX {
        return Err(BlackHoleError::UpgradeAtMaxLevel { axis });
    }

    let to_level = from_level + 1;
    let mut consumed_resources = vec![ResourceRequirement {
        resource: ResourceKind::Materials,
        quantity: 5 * u32::from(to_level),
    }];
    let axis_quantity = 2 * u32::from(to_level);
    match axis {
        BlackHoleAxis::Width => {
            consumed_resources.push(ResourceRequirement {
                resource: ResourceKind::Logs,
                quantity: axis_quantity,
            });
            if to_level >= 4 {
                consumed_resources.push(ResourceRequirement {
                    resource: ResourceKind::Planks,
                    quantity: 2 * u32::from(to_level - 3),
                });
            }
        }
        BlackHoleAxis::Depth => {
            consumed_resources.push(ResourceRequirement {
                resource: ResourceKind::Stone,
                quantity: axis_quantity,
            });
            if to_level >= 4 {
                consumed_resources.push(ResourceRequirement {
                    resource: ResourceKind::Blocks,
                    quantity: 2 * u32::from(to_level - 3),
                });
            }
        }
        BlackHoleAxis::Darkness => {
            consumed_resources.push(ResourceRequirement {
                resource: ResourceKind::Herbs,
                quantity: axis_quantity,
            });
            if to_level >= 4 {
                consumed_resources.push(ResourceRequirement {
                    resource: ResourceKind::Refined,
                    quantity: 2 * u32::from(to_level - 3),
                });
            }
        }
    }
    if to_level >= 7 {
        consumed_resources.push(ResourceRequirement {
            resource: ResourceKind::Metal,
            quantity: 2 * u32::from(to_level - 6),
        });
    }
    if to_level == 10 {
        consumed_resources.push(ResourceRequirement {
            resource: ResourceKind::Gem,
            quantity: 4,
        });
    }
    let consumed_tools = match to_level {
        1 => Vec::new(),
        2..=4 => vec![ToolRequirement {
            minimum_quality: 0,
            quantity: 1,
        }],
        5..=6 => vec![ToolRequirement {
            minimum_quality: 1,
            quantity: 1,
        }],
        7..=8 => vec![ToolRequirement {
            minimum_quality: 2,
            quantity: 2,
        }],
        9 => vec![ToolRequirement {
            minimum_quality: 3,
            quantity: 2,
        }],
        _ => vec![ToolRequirement {
            minimum_quality: 4,
            quantity: 3,
        }],
    };
    Ok(UpgradeRecipe {
        axis,
        from_level,
        to_level,
        // Research already spent Void Insight. Physical construction never
        // charges the currency a second time.
        reward_cost_micros: 0,
        consumed_resources,
        consumed_tools,
    })
}
