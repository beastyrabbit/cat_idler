//! Observed-state labor-pressure telemetry for player-guided playtests.
//!
//! This is deliberately a read-only projection of authoritative runtime state. It does
//! not queue work, change resources, or treat an intentionally vacant officer as hidden
//! automation. A vacant processor counts as useful demand only when its selected recipe
//! is researched and the same physical block-reason path says a temporary worker could
//! fetch input or advance/haul existing station stock.

use std::collections::BTreeSet;

use crate::{
    entities::CatActivity,
    farming::FarmWorkPhase,
    officers::OfficerRole,
    transport::{ProjectPhase, RoutePhase},
    world_tick::{ColonyRuntime, building_production_block_reason, building_staff_cap},
};

/// One deterministic sample of useful work against the living workforce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaborPressureSample {
    pub living_cats: usize,
    /// Unique cats owning a job, station, farm, or transport state machine.
    pub assigned_cats: usize,
    /// Unique assigned cats whose current owner has reachable/sourced work rather
    /// than a paused, research-locked, missing-input, or full-output station.
    pub useful_assigned_cats: usize,
    /// Assigned cats that are visibly traveling, working, or returning right now.
    pub active_cats: usize,
    /// Living cats with no useful-work ownership and an idle physical activity.
    pub idle_cats: usize,
    pub active_job_slots: usize,
    pub staffed_station_slots: usize,
    pub staffed_farm_slots: usize,
    pub staffed_transport_slots: usize,
    /// Unstaffed processor slots whose selected recipe has reachable physical work.
    pub sourced_station_vacancies: usize,
    /// Existing exterior plots that can accept a worker and are not output-blocked.
    pub workable_farm_vacancies: usize,
    /// Specialist roles intentionally left manual; these are reported, never counted as
    /// automated work demand.
    pub manual_only_offices: usize,
}

impl LaborPressureSample {
    /// Useful occupied plus immediately workable slots visible in this sample.
    #[must_use]
    pub fn useful_demand(&self) -> usize {
        self.useful_assigned_cats + self.sourced_station_vacancies + self.workable_farm_vacancies
    }

    #[must_use]
    pub fn demand_exceeds_labor(&self) -> bool {
        self.useful_demand() > self.living_cats
    }
}

fn reason_is_sourced_work(reason: Option<&str>) -> bool {
    reason.is_none_or(|reason| {
        reason.starts_with("fetching_")
            || matches!(
                reason,
                "worker_travel" | "input_in_transit" | "output_in_transit" | "output_awaiting_haul"
            )
    })
}

fn sourced_station_vacancies(colony: &ColonyRuntime, busy: &BTreeSet<&str>) -> usize {
    let probe_cat = colony
        .cats
        .iter()
        .find(|cat| cat.death_time.is_none() && !busy.contains(cat.id.as_str()))
        .or_else(|| colony.cats.iter().find(|cat| cat.death_time.is_none()));
    let Some(probe_cat) = probe_cat else {
        return 0;
    };

    colony
        .buildings
        .iter()
        .filter(|building| building.is_complete && building.construction_progress >= 100)
        .map(|building| {
            let cap = building_staff_cap(colony, building) as usize;
            let vacancies = cap.saturating_sub(building.worker_count());
            if vacancies == 0 {
                return 0;
            }
            // The authoritative block-reason path checks worker ownership before inputs.
            // Probe a clone with one real living cat so the remaining result describes
            // physical recipe/source/headroom truth without changing the campaign.
            let mut probe = colony.clone();
            let Some(probe_building) = probe
                .buildings
                .iter_mut()
                .find(|candidate| candidate.id == building.id)
            else {
                return 0;
            };
            if probe_building.assigned_cat.is_none() {
                probe_building.assigned_cat = Some(probe_cat.id.clone());
            }
            let reason = probe
                .buildings
                .iter()
                .find(|candidate| candidate.id == building.id)
                .and_then(|candidate| building_production_block_reason(&probe, candidate));
            usize::from(reason_is_sourced_work(reason.as_deref())) * vacancies
        })
        .sum()
}

/// Sample labor ownership and immediately useful vacancies without mutating `colony`.
#[must_use]
pub fn observe_labor_pressure(colony: &ColonyRuntime) -> LaborPressureSample {
    let living = colony
        .cats
        .iter()
        .filter(|cat| cat.death_time.is_none())
        .collect::<Vec<_>>();
    let living_ids = living
        .iter()
        .map(|cat| cat.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut assigned = BTreeSet::new();
    let mut useful_assigned = BTreeSet::new();

    let active_jobs = colony
        .jobs
        .iter()
        .filter(|job| {
            matches!(
                job.status,
                crate::types::JobStatus::Queued | crate::types::JobStatus::Active
            )
        })
        .filter_map(|job| job.assigned_cat.as_deref())
        .filter(|cat_id| living_ids.contains(cat_id))
        .inspect(|cat_id| {
            assigned.insert(*cat_id);
            useful_assigned.insert(*cat_id);
        })
        .count();

    let mut staffed_stations = 0;
    for building in &colony.buildings {
        let useful =
            reason_is_sourced_work(building_production_block_reason(colony, building).as_deref());
        for cat_id in building
            .assigned_cat
            .as_deref()
            .into_iter()
            .chain(
                building
                    .additional_work_slots
                    .iter()
                    .map(|slot| slot.assigned_cat.as_str()),
            )
            .filter(|cat_id| !cat_id.is_empty() && living_ids.contains(cat_id))
        {
            staffed_stations += 1;
            assigned.insert(cat_id);
            if useful {
                useful_assigned.insert(cat_id);
            }
        }
    }

    let mut staffed_farms = 0;
    for cat_id in colony
        .farms
        .iter()
        .filter(|plot| plot.work_phase != FarmWorkPhase::OutputBlocked)
        .filter_map(|plot| plot.worker_id.as_deref())
        .filter(|cat_id| living_ids.contains(cat_id))
    {
        staffed_farms += 1;
        assigned.insert(cat_id);
        useful_assigned.insert(cat_id);
    }

    let mut staffed_transport = 0;
    for cat_id in colony
        .transport
        .projects
        .values()
        .filter(|project| {
            !matches!(
                project.phase,
                ProjectPhase::Complete | ProjectPhase::Cancelled
            )
        })
        .map(|project| project.assigned_cat_id.as_str())
        .chain(
            colony
                .transport
                .routes
                .values()
                .filter(|route| route.phase != RoutePhase::Cancelled)
                .map(|route| route.assigned_cat_id.as_str()),
        )
        .filter(|cat_id| living_ids.contains(cat_id))
    {
        staffed_transport += 1;
        assigned.insert(cat_id);
        useful_assigned.insert(cat_id);
    }

    let sourced_station_vacancies = sourced_station_vacancies(colony, &assigned);
    let workable_farm_vacancies = colony
        .farms
        .iter()
        .filter(|plot| plot.worker_id.is_none() && plot.work_phase != FarmWorkPhase::OutputBlocked)
        .count();
    let active_cats = living
        .iter()
        .filter(|cat| assigned.contains(cat.id.as_str()) && cat.activity != CatActivity::Idle)
        .count();
    let idle_cats = living
        .iter()
        .filter(|cat| !assigned.contains(cat.id.as_str()) && cat.activity == CatActivity::Idle)
        .count();

    LaborPressureSample {
        living_cats: living.len(),
        assigned_cats: assigned.len(),
        useful_assigned_cats: useful_assigned.len(),
        active_cats,
        idle_cats,
        active_job_slots: active_jobs,
        staffed_station_slots: staffed_stations,
        staffed_farm_slots: staffed_farms,
        staffed_transport_slots: staffed_transport,
        sourced_station_vacancies,
        workable_farm_vacancies,
        manual_only_offices: OfficerRole::ALL
            .iter()
            .filter(|role| !colony.officers.contains_key(role))
            .count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_tick::{found_colony, new_world};

    #[test]
    fn observation_is_read_only_and_reports_founding_manual_offices() {
        let mut world = new_world(7);
        world.colonies.push(found_colony(7, "colony-1", 0, 7));
        let before = world.colonies[0].clone();
        let sample = observe_labor_pressure(&world.colonies[0]);
        assert_eq!(sample.living_cats, 15);
        assert_eq!(sample.manual_only_offices, 7);
        assert_eq!(world.colonies[0], before);
    }
}
