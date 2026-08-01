//! Deterministic hazardous-work incidents specified by
//! `docs/leader-ai-overhaul/cats-and-care.md`.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::{
    anatomy::{BodyPart, BodyPartCondition, CatAnatomy, HazardousJob},
    planner_core::{PlannerRngStream, planner_roll},
};

pub const PROBABILITY_BASIS_POINTS: u16 = 10_000;
pub const MINOR_OUTCOME_BASIS_POINTS: u16 = 7_000;
pub const SEVERE_OUTCOME_BASIS_POINTS: u16 = 2_000;
pub const MISSING_OUTCOME_BASIS_POINTS: u16 = 800;
pub const FATAL_OUTCOME_BASIS_POINTS: u16 = 200;

/// One completed hazardous work unit eligible for exactly one incident evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardousWorkUnit {
    Scout,
    Hunt,
    Quarry,
    Logging,
    Construction,
    RaidVictory,
    RaidDefeat,
}

impl HazardousWorkUnit {
    pub const ALL: [Self; 7] = [
        Self::Scout,
        Self::Hunt,
        Self::Quarry,
        Self::Logging,
        Self::Construction,
        Self::RaidVictory,
        Self::RaidDefeat,
    ];

    #[must_use]
    pub const fn incident_basis_points(self) -> u16 {
        match self {
            Self::Scout => 150,
            Self::Hunt => 100,
            Self::Quarry => 80,
            Self::Logging => 50,
            Self::Construction => 30,
            Self::RaidVictory => 500,
            Self::RaidDefeat => 2_000,
        }
    }

    #[must_use]
    pub const fn anatomy_job(self) -> HazardousJob {
        match self {
            Self::Scout => HazardousJob::Scout,
            Self::Hunt => HazardousJob::Hunt,
            Self::Quarry => HazardousJob::Quarry,
            Self::Logging => HazardousJob::Logging,
            Self::Construction => HazardousJob::Construction,
            Self::RaidVictory | Self::RaidDefeat => HazardousJob::Raid,
        }
    }
}

/// Stable identity of one atomic incident opportunity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncidentIdentity {
    pub cat_id: String,
    pub task_id: String,
    pub work_unit_index: u64,
    pub completion_tick: u64,
}

impl IncidentIdentity {
    #[must_use]
    pub fn new(
        cat_id: impl Into<String>,
        task_id: impl Into<String>,
        work_unit_index: u64,
        completion_tick: u64,
    ) -> Self {
        Self {
            cat_id: cat_id.into(),
            task_id: task_id.into(),
            work_unit_index,
            completion_tick,
        }
    }

    /// Losslessly component-encoded persisted incident ID.
    #[must_use]
    pub fn stable_id(&self) -> String {
        let mut id = String::from("injury:v1");
        for component in [
            self.cat_id.as_str(),
            self.task_id.as_str(),
            &self.work_unit_index.to_string(),
            &self.completion_tick.to_string(),
        ] {
            write!(id, "|{}:{component}", component.len())
                .expect("writing into a String cannot fail");
        }
        id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InjuryRolls {
    pub incident_bucket: u16,
    pub outcome_bucket: u16,
    pub part_selector: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjuryOutcome {
    Minor,
    Severe,
    Missing,
    Fatal,
}

impl InjuryOutcome {
    const fn condition(self) -> Option<BodyPartCondition> {
        match self {
            Self::Minor => Some(BodyPartCondition::Minor),
            Self::Severe => Some(BodyPartCondition::Severe),
            Self::Missing => Some(BodyPartCondition::Missing),
            Self::Fatal => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum IncidentResolution {
    NoIncident,
    NoEligiblePart {
        incident_id: String,
        outcome: InjuryOutcome,
    },
    Injured {
        incident_id: String,
        part: BodyPart,
        outcome: InjuryOutcome,
        previous: BodyPartCondition,
        current: BodyPartCondition,
    },
    Fatal {
        incident_id: String,
    },
}

/// One draw per semantic purpose from the dedicated keyed injury stream.
#[must_use]
pub fn injury_rolls(world_seed: u32, identity: &IncidentIdentity) -> InjuryRolls {
    let incident = injury_draw(world_seed, identity, "incident");
    let outcome = injury_draw(world_seed, identity, "outcome");
    let part_selector = injury_draw(world_seed, identity, "part");
    InjuryRolls {
        incident_bucket: probability_bucket(incident),
        outcome_bucket: probability_bucket(outcome),
        part_selector,
    }
}

fn injury_draw(world_seed: u32, identity: &IncidentIdentity, purpose: &str) -> u32 {
    let work_unit = identity.work_unit_index.to_string();
    let completion_tick = identity.completion_tick.to_string();
    planner_roll(
        world_seed,
        PlannerRngStream::Injury,
        [
            identity.cat_id.as_str(),
            identity.task_id.as_str(),
            work_unit.as_str(),
            completion_tick.as_str(),
            purpose,
        ],
    )
    .next_seed
}

fn probability_bucket(seed: u32) -> u16 {
    ((u64::from(seed) * u64::from(PROBABILITY_BASIS_POINTS)) >> 32) as u16
}

#[must_use]
pub const fn incident_occurs(work: HazardousWorkUnit, bucket: u16) -> bool {
    bucket < work.incident_basis_points()
}

#[must_use]
pub const fn classify_outcome(bucket: u16) -> InjuryOutcome {
    let bucket = if bucket >= PROBABILITY_BASIS_POINTS {
        PROBABILITY_BASIS_POINTS - 1
    } else {
        bucket
    };
    if bucket < MINOR_OUTCOME_BASIS_POINTS {
        InjuryOutcome::Minor
    } else if bucket < MINOR_OUTCOME_BASIS_POINTS + SEVERE_OUTCOME_BASIS_POINTS {
        InjuryOutcome::Severe
    } else if bucket
        < MINOR_OUTCOME_BASIS_POINTS + SEVERE_OUTCOME_BASIS_POINTS + MISSING_OUTCOME_BASIS_POINTS
    {
        InjuryOutcome::Missing
    } else {
        InjuryOutcome::Fatal
    }
}

#[must_use]
pub fn evaluate_incident(
    world_seed: u32,
    anatomy: &mut CatAnatomy,
    work: HazardousWorkUnit,
    identity: &IncidentIdentity,
) -> IncidentResolution {
    resolve_incident_with_rolls(anatomy, work, identity, injury_rolls(world_seed, identity))
}

#[must_use]
pub fn resolve_incident_with_rolls(
    anatomy: &mut CatAnatomy,
    work: HazardousWorkUnit,
    identity: &IncidentIdentity,
    rolls: InjuryRolls,
) -> IncidentResolution {
    if !incident_occurs(work, rolls.incident_bucket) {
        return IncidentResolution::NoIncident;
    }
    let outcome = classify_outcome(rolls.outcome_bucket);
    let Some(condition) = outcome.condition() else {
        return IncidentResolution::Fatal {
            incident_id: identity.stable_id(),
        };
    };
    let eligible = BodyPart::ALL
        .into_iter()
        .filter(|part| anatomy.part(*part).condition < condition)
        .collect::<Vec<_>>();
    let incident_id = identity.stable_id();
    let Some(part) = eligible
        .get((rolls.part_selector as usize) % eligible.len().max(1))
        .copied()
    else {
        return IncidentResolution::NoEligiblePart {
            incident_id,
            outcome,
        };
    };
    let previous = anatomy.part(part).condition;
    anatomy.injure(part, condition);
    anatomy
        .part_mut(part)
        .record_incident(incident_id.clone(), identity.completion_tick);
    IncidentResolution::Injured {
        incident_id,
        part,
        outcome,
        previous,
        current: condition,
    }
}

const _: () = assert!(
    MINOR_OUTCOME_BASIS_POINTS
        + SEVERE_OUTCOME_BASIS_POINTS
        + MISSING_OUTCOME_BASIS_POINTS
        + FATAL_OUTCOME_BASIS_POINTS
        == PROBABILITY_BASIS_POINTS
);
