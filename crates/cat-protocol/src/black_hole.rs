//! Additive, player-neutral Black Hole wire types.

use serde::{Deserialize, Serialize};

use crate::ResourceKind;

pub const BLACK_HOLE_LEVEL_MAX: u8 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct BlackHoleLevel(u8);

impl BlackHoleLevel {
    pub const fn new(value: u8) -> Result<Self, &'static str> {
        if value <= BLACK_HOLE_LEVEL_MAX {
            Ok(Self(value))
        } else {
            Err("Black Hole level must be in 0..=10")
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl<'de> Deserialize<'de> for BlackHoleLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlackHoleAxis {
    Width,
    Depth,
    Darkness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleAxisState {
    pub axis: BlackHoleAxis,
    pub physical_level: BlackHoleLevel,
    pub researched_level: BlackHoleLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleIntakeTiming {
    pub opening_index: u64,
    pub next_opens_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleFeedLine {
    pub resource: ResourceKind,
    pub planned_units: u32,
    pub delivered_units: u32,
    pub credited_units: u32,
    pub credited_value_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleFeedOrder {
    pub id: String,
    pub opening_index: u64,
    pub line: BlackHoleFeedLine,
    pub carrier_cat_id: Option<String>,
    pub waiting_for_opening: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleUpgradeRequirement {
    pub descriptor_id: String,
    pub required_units: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleUpgradeProject {
    pub job_id: String,
    pub axis: BlackHoleAxis,
    pub current_level: BlackHoleLevel,
    pub target_level: BlackHoleLevel,
    pub requirements: Vec<BlackHoleUpgradeRequirement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleResourceDescriptor {
    pub resource: ResourceKind,
    pub darkness_required: BlackHoleLevel,
    pub reward_micros_per_unit: u64,
    pub visible_units: u32,
    pub orderable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleItemDescriptor {
    pub kind_id: String,
    pub darkness_required: BlackHoleLevel,
    pub maximum_quality: BlackHoleLevel,
    pub stored_count: u32,
    pub orderable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleLifetimeTotals {
    pub credited_units: u64,
    pub credited_value_micros: u64,
    pub opening_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlackHoleSnapshot {
    pub building_id: String,
    pub axes: Vec<BlackHoleAxisState>,
    pub intake: BlackHoleIntakeTiming,
    pub active_feed_order: Option<BlackHoleFeedOrder>,
    pub active_project: Option<BlackHoleUpgradeProject>,
    pub accepted_resources: Vec<BlackHoleResourceDescriptor>,
    pub accepted_items: Vec<BlackHoleItemDescriptor>,
    pub lifetime_totals: BlackHoleLifetimeTotals,
    pub next_review_at_ms: Option<i64>,
    pub urged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum BlackHoleAction {
    NudgeBlackHole {
        session_id: String,
        nickname: String,
        sig: String,
    },
}
