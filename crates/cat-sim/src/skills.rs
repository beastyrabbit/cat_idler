//! Per-labor proficiency skills (P12.1).
//!
//! Every labor a cat performs accrues a continuous per-labor skill that scales the
//! job's speed and yield through the existing `life_sim` trade curves
//! ([`crate::life_sim::trade_speed_multiplier`] / [`crate::life_sim::trade_yield_multiplier`]).
//! This generalizes [`crate::entities::RoleXp`] (four specialization roles) to *all*
//! labors; `RoleXp` stays as the discrete specialization / auto-promotion track and
//! skills stack continuously on top of it.

use serde::{Deserialize, Serialize};

use crate::types::JobKind;

/// Skill gained when a labor job completes — matches the `role_xp` +1.0 per job so
/// hunt's skill and `role_xp.hunter` stay in lock-step (no behavior change at 0).
pub const SKILL_GAIN_PER_JOB: f64 = 1.0;

/// Skill gained per hauling trip. Smaller than a full job completion: hauling is a
/// side effect of gathering jobs, not a job in its own right.
pub const HAUL_SKILL_GAIN: f64 = 0.25;

/// A labor category a cat can become skilled at. `Ord`/`Hash` so it can key a
/// deterministic [`std::collections::BTreeMap`] and serialize as a JSON object key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Labor {
    Hunt,
    Build,
    Ritual,
    Fight,
    Quarry,
    FetchWater,
    Mill,
    Craft,
    Farm,
    Haul,
    Research,
}

impl Labor {
    /// The labor a completed job of `kind` trains, if any. Planning, supply,
    /// scouting and expansion jobs train no labor skill (they have no yield/skill
    /// curve to feed). `Mill`/`Craft`/`Farm`/`Research` have no job kinds yet — they
    /// are reserved for the later P12.4/P12.5 production chains.
    #[must_use]
    pub fn for_job_kind(kind: JobKind) -> Option<Self> {
        match kind {
            JobKind::HuntExpedition => Some(Self::Hunt),
            JobKind::BuildHouse => Some(Self::Build),
            JobKind::Ritual => Some(Self::Ritual),
            JobKind::Quarry => Some(Self::Quarry),
            JobKind::FetchWater => Some(Self::FetchWater),
            JobKind::TrainWarrior => Some(Self::Fight),
            // P12.6: an offering is a ritual act (haul + perform at the shrine), so
            // it trains the same Ritual labor as the pure-labor `ritual` job.
            JobKind::CarryOffering => Some(Self::Ritual),
            JobKind::SupplyFood
            | JobKind::SupplyWater
            | JobKind::LeaderPlanHunt
            | JobKind::LeaderPlanHouse
            | JobKind::Explore
            | JobKind::ExpandVillage => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Labor;
    use crate::types::JobKind;

    #[test]
    fn labor_maps_every_job_kind() {
        // Exhaustive so a new JobKind forces a decision here.
        for kind in JobKind::ALL {
            let mapped = Labor::for_job_kind(*kind);
            match kind {
                JobKind::HuntExpedition => assert_eq!(mapped, Some(Labor::Hunt)),
                JobKind::BuildHouse => assert_eq!(mapped, Some(Labor::Build)),
                JobKind::Ritual => assert_eq!(mapped, Some(Labor::Ritual)),
                JobKind::Quarry => assert_eq!(mapped, Some(Labor::Quarry)),
                JobKind::FetchWater => assert_eq!(mapped, Some(Labor::FetchWater)),
                JobKind::TrainWarrior => assert_eq!(mapped, Some(Labor::Fight)),
                JobKind::CarryOffering => assert_eq!(mapped, Some(Labor::Ritual)),
                _ => assert_eq!(mapped, None),
            }
        }
    }

    #[test]
    fn labor_serializes_to_snake_case_wire_string() {
        assert_eq!(
            serde_json::to_string(&Labor::FetchWater).unwrap(),
            "\"fetch_water\""
        );
        assert_eq!(
            serde_json::from_str::<Labor>("\"hunt\"").unwrap(),
            Labor::Hunt
        );
    }
}
