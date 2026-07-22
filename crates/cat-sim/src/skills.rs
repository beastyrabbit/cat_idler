//! Per-labor proficiency skills (P12.1).
//!
//! Every labor a cat performs accrues a continuous per-labor skill that scales the
//! job's speed and yield through the existing `life_sim` trade curves
//! ([`crate::life_sim::trade_speed_multiplier`] / [`crate::life_sim::trade_yield_multiplier`]).
//! This generalizes [`crate::entities::RoleXp`] (four specialization roles) to *all*
//! labors; `RoleXp` stays as the discrete specialization / auto-promotion track and
//! skills stack continuously on top of it.

use serde::{Deserialize, Serialize};

use crate::types::{BuildingType, JobKind};

/// Skill gained when a labor job completes — matches the `role_xp` +1.0 per job so
/// hunt's skill and `role_xp.hunter` stay in lock-step (no behavior change at 0).
pub const SKILL_GAIN_PER_JOB: f64 = 1.0;

/// Skill gained per hauling trip. Smaller than a full job completion: hauling is a
/// side effect of gathering jobs, not a job in its own right.
pub const HAUL_SKILL_GAIN: f64 = 0.25;

/// Station work earns one point per completed recipe cycle. Continuous work
/// (fields and research) earns the same point per game-hour, making accrual
/// independent of server tick size.
pub const SKILL_GAIN_PER_WORK_HOUR: f64 = 1.0;

/// A labor category a cat can become skilled at. `Ord`/`Hash` so it can key a
/// deterministic [`std::collections::BTreeMap`] and serialize as a JSON object key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Labor {
    Hunt,
    Fishing,
    Build,
    Ritual,
    Fight,
    Train,
    Quarry,
    Woodcut,
    Forage,
    FetchWater,
    Mill,
    Process,
    Craft,
    Textile,
    Metalwork,
    Farm,
    Haul,
    Research,
    Scout,
}

impl Labor {
    pub const ALL: &'static [Self] = &[
        Self::Hunt,
        Self::Fishing,
        Self::Build,
        Self::Ritual,
        Self::Fight,
        Self::Train,
        Self::Quarry,
        Self::Woodcut,
        Self::Forage,
        Self::FetchWater,
        Self::Mill,
        Self::Process,
        Self::Craft,
        Self::Textile,
        Self::Metalwork,
        Self::Farm,
        Self::Haul,
        Self::Research,
        Self::Scout,
    ];

    /// The labor a completed job of `kind` trains, if any. Abstract supply and
    /// leader-planning jobs train nothing because no cat performs physical work;
    /// maintained station labor is mapped separately by [`Self::for_building_type`].
    #[must_use]
    pub fn for_job_kind(kind: JobKind) -> Option<Self> {
        match kind {
            JobKind::HuntExpedition => Some(Self::Hunt),
            JobKind::GatherFood => Some(Self::Farm),
            JobKind::Fish => Some(Self::Fishing),
            JobKind::BuildHouse | JobKind::BuildRoad => Some(Self::Build),
            JobKind::Ritual => Some(Self::Ritual),
            JobKind::Quarry => Some(Self::Quarry),
            JobKind::GatherLogs | JobKind::ReplantTree => Some(Self::Woodcut),
            JobKind::ForageFibre => Some(Self::Forage),
            JobKind::FetchWater => Some(Self::FetchWater),
            JobKind::TrainWarrior => Some(Self::Train),
            // P12.6 separates the physical delivery from the shrine ceremony so each
            // stage trains the labor it actually performs.
            JobKind::CarryOffering => Some(Self::Haul),
            JobKind::PerformOffering => Some(Self::Ritual),
            // P16: a gather-spot mover is pure hauling, same labor as the mid-job haul
            // trips other gathering jobs already train (`HAUL_SKILL_GAIN`).
            JobKind::HaulGatherSpot => Some(Self::Haul),
            JobKind::VillageMaintenance => Some(Self::Haul),
            JobKind::Explore => Some(Self::Scout),
            JobKind::ExpandVillage => Some(Self::Build),
            JobKind::SupplyFood
            | JobKind::SupplyWater
            | JobKind::LeaderPlanHunt
            | JobKind::LeaderPlanHouse => None,
        }
    }

    /// Labor performed by a staffed production/research building. Housing,
    /// storage, civic shells, and role stations without a maintained production
    /// cycle intentionally map to `None`.
    #[must_use]
    pub const fn for_building_type(kind: BuildingType) -> Option<Self> {
        match kind {
            BuildingType::Field => Some(Self::Farm),
            BuildingType::Mill => Some(Self::Mill),
            BuildingType::Sawmill
            | BuildingType::Workshop
            | BuildingType::WoodCutter
            | BuildingType::StonePrep => Some(Self::Process),
            BuildingType::Woodworking => Some(Self::Craft),
            BuildingType::Clothier | BuildingType::Tannery => Some(Self::Textile),
            BuildingType::Smithy | BuildingType::Smelter => Some(Self::Metalwork),
            BuildingType::ResearchHut | BuildingType::School => Some(Self::Research),
            BuildingType::Den
            | BuildingType::FoodStorage
            | BuildingType::WaterBowl
            | BuildingType::Beds
            | BuildingType::HerbGarden
            | BuildingType::Nursery
            | BuildingType::ElderCorner
            | BuildingType::Walls
            | BuildingType::MouseFarm
            | BuildingType::Shrine
            | BuildingType::Barracks
            | BuildingType::AccountingTent => None,
        }
    }
}

/// Convert the duration multiplier used by discrete jobs into a bounded work-rate
/// multiplier for continuous/station work. This is exactly 1 at zero skill and
/// approaches 4/3, matching the existing skill curve without an unbounded economy.
#[must_use]
pub fn work_rate_multiplier(skill: f64) -> f64 {
    1.0 / crate::life_sim::trade_speed_multiplier(skill.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::Labor;
    use crate::types::{BuildingType, JobKind};

    #[test]
    fn labor_maps_every_job_kind() {
        // Exhaustive so a new JobKind forces a decision here.
        for kind in JobKind::ALL {
            let mapped = Labor::for_job_kind(*kind);
            match kind {
                JobKind::HuntExpedition => assert_eq!(mapped, Some(Labor::Hunt)),
                JobKind::GatherFood => assert_eq!(mapped, Some(Labor::Farm)),
                JobKind::Fish => assert_eq!(mapped, Some(Labor::Fishing)),
                JobKind::BuildHouse | JobKind::BuildRoad => {
                    assert_eq!(mapped, Some(Labor::Build));
                }
                JobKind::Ritual => assert_eq!(mapped, Some(Labor::Ritual)),
                JobKind::Quarry => assert_eq!(mapped, Some(Labor::Quarry)),
                JobKind::GatherLogs | JobKind::ReplantTree => {
                    assert_eq!(mapped, Some(Labor::Woodcut));
                }
                JobKind::ForageFibre => assert_eq!(mapped, Some(Labor::Forage)),
                JobKind::FetchWater => assert_eq!(mapped, Some(Labor::FetchWater)),
                JobKind::TrainWarrior => assert_eq!(mapped, Some(Labor::Train)),
                JobKind::CarryOffering => assert_eq!(mapped, Some(Labor::Haul)),
                JobKind::PerformOffering => assert_eq!(mapped, Some(Labor::Ritual)),
                JobKind::HaulGatherSpot => assert_eq!(mapped, Some(Labor::Haul)),
                JobKind::VillageMaintenance => assert_eq!(mapped, Some(Labor::Haul)),
                JobKind::Explore => assert_eq!(mapped, Some(Labor::Scout)),
                JobKind::ExpandVillage => assert_eq!(mapped, Some(Labor::Build)),
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

    #[test]
    fn station_work_rate_is_monotonic_and_bounded() {
        let zero = super::work_rate_multiplier(0.0);
        let practiced = super::work_rate_multiplier(30.0);
        let expert = super::work_rate_multiplier(1_000_000.0);
        assert_eq!(zero, 1.0);
        assert!(practiced > zero);
        assert!(expert > practiced);
        assert!(expert <= 4.0 / 3.0 + f64::EPSILON);
    }

    #[test]
    fn every_building_has_an_explicit_labor_decision() {
        for kind in BuildingType::ALL {
            let labor = Labor::for_building_type(*kind);
            match kind {
                BuildingType::Field => assert_eq!(labor, Some(Labor::Farm)),
                BuildingType::Mill => assert_eq!(labor, Some(Labor::Mill)),
                BuildingType::Sawmill
                | BuildingType::Workshop
                | BuildingType::WoodCutter
                | BuildingType::StonePrep => assert_eq!(labor, Some(Labor::Process)),
                BuildingType::Woodworking => assert_eq!(labor, Some(Labor::Craft)),
                BuildingType::Clothier | BuildingType::Tannery => {
                    assert_eq!(labor, Some(Labor::Textile));
                }
                BuildingType::Smithy | BuildingType::Smelter => {
                    assert_eq!(labor, Some(Labor::Metalwork));
                }
                BuildingType::ResearchHut | BuildingType::School => {
                    assert_eq!(labor, Some(Labor::Research));
                }
                _ => assert_eq!(labor, None),
            }
        }
    }

    #[test]
    fn every_maintained_labor_has_a_truthful_runtime_source() {
        for labor in Labor::ALL {
            let job_source = JobKind::ALL
                .iter()
                .any(|kind| Labor::for_job_kind(*kind) == Some(*labor));
            let building_source = BuildingType::ALL
                .iter()
                .any(|kind| Labor::for_building_type(*kind) == Some(*labor));
            let combat_source = *labor == Labor::Fight;
            assert!(
                job_source || building_source || combat_source,
                "{labor:?} has no maintained completion path"
            );
        }
    }
}
