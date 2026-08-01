//! Deterministic willingness, refusal, and safe refusal-boundary contracts
//! specified by `docs/leader-ai-overhaul/cats-and-care.md`.

use serde::{Deserialize, Serialize};

use crate::{
    acquired_traits::AcquiredTraitState,
    cat_stress::{StressBand, StressLevel},
    cat_traits::{CatPersonality, PersonalityAxis, PersonalityPole},
    planner_core::{BASIS_POINTS_SCALE, BasisPoints},
};

pub const REFUSAL_BUCKET_MAX: u16 = 9_999;

/// An explicit deterministic bucket derived by LAI.12 from world, colony, cat,
/// task, and assignment-occurrence IDs. It is not an RNG cursor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RefusalBucket(u16);

impl RefusalBucket {
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for RefusalBucket {
    type Error = &'static str;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value <= REFUSAL_BUCKET_MAX {
            Ok(Self(value))
        } else {
            Err("refusal bucket must be in 0..=9,999")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    SelfPreservation,
    Emergency,
    Required,
    Optional,
}

impl TaskPriority {
    #[must_use]
    pub const fn is_non_emergency(self) -> bool {
        matches!(self, Self::Required | Self::Optional)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRisk {
    Safe,
    Stressful,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WillingnessContext {
    pub refusal_bucket: RefusalBucket,
    pub stress: StressLevel,
    pub priority: TaskPriority,
    pub risk: TaskRisk,
    pub pregnant: bool,
    pub injured: bool,
    pub safer_eligible_worker_exists: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    Stress,
    CriticalStress,
    ProtectedFromHighRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WillingnessDecision {
    Willing,
    ReducedSuitability,
    Refused(RefusalReason),
}

impl WillingnessDecision {
    #[must_use]
    pub const fn accepts_assignment(self) -> bool {
        !matches!(self, Self::Refused(_))
    }
}

/// Exact unmodified refusal chance for the 80–94 band. Other stress bands do
/// not use probabilistic refusal.
#[must_use]
pub const fn base_refusal_chance(stress: StressLevel) -> BasisPoints {
    if matches!(stress.get(), 80..=94) {
        BasisPoints::new((stress.get() as i64 - 60) * 100)
    } else {
        BasisPoints::new(0)
    }
}

/// Evaluate one assignment from an explicit deterministic 0–9,999 bucket.
/// LAI.12 owns the semantic keyed derivation; this leaf consumes no shared RNG
/// state and cannot couple refusal to planning cadence or collection order.
#[must_use]
pub fn evaluate_willingness(context: WillingnessContext) -> WillingnessDecision {
    if context.priority == TaskPriority::SelfPreservation {
        return WillingnessDecision::Willing;
    }

    if context.risk == TaskRisk::High
        && context.safer_eligible_worker_exists
        && (context.pregnant || context.injured)
    {
        return WillingnessDecision::Refused(RefusalReason::ProtectedFromHighRisk);
    }

    if context.stress.band() == StressBand::Critical && context.priority == TaskPriority::Optional {
        return WillingnessDecision::Refused(RefusalReason::CriticalStress);
    }

    if context.priority.is_non_emergency()
        && context.stress.band() == StressBand::RefusalRisk
        && i64::from(context.refusal_bucket.get()) < base_refusal_chance(context.stress).get()
    {
        return WillingnessDecision::Refused(RefusalReason::Stress);
    }

    if context.stress.band() == StressBand::Reduced
        && matches!(context.risk, TaskRisk::Stressful | TaskRisk::High)
    {
        WillingnessDecision::ReducedSuitability
    } else {
        WillingnessDecision::Willing
    }
}

/// Leisurely/Diligent remains a separate fixed-point matching input; it never
/// rewrites the exact stress-band refusal probability.
#[must_use]
pub const fn personality_willingness_factor(personality: CatPersonality) -> BasisPoints {
    personality.weight_factor(
        PersonalityAxis::LeisurelyDiligent,
        PersonalityPole::Positive,
    )
}

/// Apply persistent acquired-trait willingness after personality and before
/// the caller's stress and divine-boost stages.
#[must_use]
pub fn acquired_willingness_factor(
    traits: &AcquiredTraitState,
    priority: TaskPriority,
    shrine_task: bool,
) -> BasisPoints {
    let mut factor = BASIS_POINTS_SCALE;
    if shrine_task {
        factor = multiply_factors(factor, traits.shrine_willingness_factor().get());
    }
    if priority.is_non_emergency() {
        factor = multiply_factors(factor, traits.non_emergency_willingness_factor().get());
    }
    BasisPoints::new(factor)
}

fn multiply_factors(first: i64, second: i64) -> i64 {
    let product =
        i128::from(first).saturating_mul(i128::from(second)) / i128::from(BASIS_POINTS_SCALE);
    product.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerCandidate {
    pub eligible: bool,
    pub decision: WillingnessDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentBlockReason {
    NoEligibleWorker,
    NoWillingWorker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentAvailability {
    Ready,
    Blocked(AssignmentBlockReason),
}

#[must_use]
pub fn assignment_availability(candidates: &[WorkerCandidate]) -> AssignmentAvailability {
    let mut eligible = false;
    for candidate in candidates {
        if candidate.eligible {
            eligible = true;
            if candidate.decision.accepts_assignment() {
                return AssignmentAvailability::Ready;
            }
        }
    }
    if eligible {
        AssignmentAvailability::Blocked(AssignmentBlockReason::NoWillingWorker)
    } else {
        AssignmentAvailability::Blocked(AssignmentBlockReason::NoEligibleWorker)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeStockpileEndpoint {
    pub endpoint_id: String,
    pub colony_id: String,
    pub route_cost: u64,
    pub safe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefusalCargo {
    pub cargo_id: String,
    pub quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CargoRefusalPlan {
    NoCargo,
    DeliverThenLeave {
        cargo: RefusalCargo,
        endpoint_id: String,
    },
    HoldCargoUntilSafeEndpoint {
        cargo: RefusalCargo,
    },
}

/// Select a safe owned cargo endpoint. A valid pinned endpoint wins; otherwise
/// the nearest route cost wins and equal cost resolves by stable endpoint ID.
#[must_use]
pub fn cargo_refusal_plan(
    cargo: Option<&RefusalCargo>,
    worker_colony_id: &str,
    pinned_endpoint_id: Option<&str>,
    safe_owned_stockpiles: &[SafeStockpileEndpoint],
) -> CargoRefusalPlan {
    let Some(cargo) = cargo else {
        return CargoRefusalPlan::NoCargo;
    };
    let candidates = || {
        safe_owned_stockpiles
            .iter()
            .filter(|endpoint| endpoint.safe && endpoint.colony_id == worker_colony_id)
    };
    if let Some(pinned) = pinned_endpoint_id
        && candidates().any(|endpoint| endpoint.endpoint_id == pinned)
    {
        return CargoRefusalPlan::DeliverThenLeave {
            cargo: cargo.clone(),
            endpoint_id: pinned.to_owned(),
        };
    }
    candidates()
        .min_by(|first, second| {
            first
                .route_cost
                .cmp(&second.route_cost)
                .then_with(|| first.endpoint_id.cmp(&second.endpoint_id))
        })
        .map_or_else(
            || CargoRefusalPlan::HoldCargoUntilSafeEndpoint {
                cargo: cargo.clone(),
            },
            |endpoint| CargoRefusalPlan::DeliverThenLeave {
                cargo: cargo.clone(),
                endpoint_id: endpoint.endpoint_id.clone(),
            },
        )
}

/// Minimal atomic station boundary persisted by the later task runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StationRefusalBoundary {
    pub station_id: String,
    pub cycle_id: String,
    pub inputs_consumed: bool,
    pub output_committed: bool,
    pub worker_released: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationRefusalResult {
    CompletedConsumedStepAndReleased,
    ReleasedWithoutCompleting,
    AlreadyReleased,
}

impl StationRefusalBoundary {
    /// Commit output at most once when inputs were already consumed, then
    /// release the station worker in the same boundary transition.
    pub fn resolve_refusal(&mut self) -> StationRefusalResult {
        if self.worker_released {
            return StationRefusalResult::AlreadyReleased;
        }
        self.worker_released = true;
        if self.inputs_consumed && !self.output_committed {
            self.output_committed = true;
            StationRefusalResult::CompletedConsumedStepAndReleased
        } else {
            StationRefusalResult::ReleasedWithoutCompleting
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        acquired_traits::AcquiredTrait,
        cat_traits::{PersonalityStrength, PersonalityValue},
    };

    fn context(stress: u8) -> WillingnessContext {
        WillingnessContext {
            refusal_bucket: RefusalBucket::try_from(9_999).unwrap(),
            stress: StressLevel::new_clamped(i32::from(stress)),
            priority: TaskPriority::Required,
            risk: TaskRisk::Stressful,
            pregnant: false,
            injured: false,
            safer_eligible_worker_exists: false,
        }
    }

    #[test]
    fn self_preservation_overrides_stress_and_protected_risk_refusal() {
        let mut input = context(100);
        input.priority = TaskPriority::SelfPreservation;
        input.risk = TaskRisk::High;
        input.pregnant = true;
        input.safer_eligible_worker_exists = true;
        assert_eq!(evaluate_willingness(input), WillingnessDecision::Willing);
    }

    #[test]
    fn reduced_band_only_reduces_stressful_or_risky_work() {
        let mut input = context(60);
        assert_eq!(
            evaluate_willingness(input),
            WillingnessDecision::ReducedSuitability
        );
        input.risk = TaskRisk::Safe;
        assert_eq!(evaluate_willingness(input), WillingnessDecision::Willing);
    }

    #[test]
    fn refusal_probability_and_bucket_boundaries_are_exact() {
        assert!(RefusalBucket::try_from(10_000).is_err());
        assert_eq!(base_refusal_chance(StressLevel::new_clamped(79)).get(), 0);
        assert_eq!(
            base_refusal_chance(StressLevel::new_clamped(80)).get(),
            2_000
        );
        assert_eq!(
            base_refusal_chance(StressLevel::new_clamped(94)).get(),
            3_400
        );
        assert_eq!(base_refusal_chance(StressLevel::new_clamped(95)).get(), 0);

        let mut input = context(80);
        input.refusal_bucket = RefusalBucket::try_from(1_999).unwrap();
        assert_eq!(
            evaluate_willingness(input),
            WillingnessDecision::Refused(RefusalReason::Stress)
        );
        input.refusal_bucket = RefusalBucket::try_from(2_000).unwrap();
        assert_eq!(evaluate_willingness(input), WillingnessDecision::Willing);

        input.stress = StressLevel::new_clamped(94);
        input.refusal_bucket = RefusalBucket::try_from(3_399).unwrap();
        assert_eq!(
            evaluate_willingness(input),
            WillingnessDecision::Refused(RefusalReason::Stress)
        );
        input.refusal_bucket = RefusalBucket::try_from(3_400).unwrap();
        assert_eq!(evaluate_willingness(input), WillingnessDecision::Willing);
    }

    #[test]
    fn explicit_bucket_results_are_independent_of_input_order() {
        let evaluate = |assignments: &[(&'static str, u16)]| {
            assignments
                .iter()
                .map(|(id, bucket)| {
                    let mut input = context(80);
                    input.refusal_bucket = RefusalBucket::try_from(*bucket).unwrap();
                    (*id, evaluate_willingness(input))
                })
                .collect::<BTreeMap<_, _>>()
        };
        let forward = [("assignment-b", 2_000), ("assignment-a", 1_999)];
        let reversed = [("assignment-a", 1_999), ("assignment-b", 2_000)];
        assert_eq!(evaluate(&forward), evaluate(&reversed));
    }

    #[test]
    fn personality_is_a_separate_fixed_point_willingness_input() {
        let leisurely = CatPersonality {
            leisurely_diligent: PersonalityValue::new(
                PersonalityPole::Negative,
                PersonalityStrength::Extreme,
            ),
            ..CatPersonality::default()
        };
        let diligent = CatPersonality {
            leisurely_diligent: PersonalityValue::new(
                PersonalityPole::Positive,
                PersonalityStrength::Extreme,
            ),
            ..CatPersonality::default()
        };
        assert_eq!(personality_willingness_factor(leisurely).get(), 7_000);
        assert_eq!(personality_willingness_factor(diligent).get(), 13_000);
    }

    #[test]
    fn critical_optional_work_always_stops_but_emergency_work_continues() {
        let mut input = context(95);
        input.priority = TaskPriority::Optional;
        assert_eq!(
            evaluate_willingness(input),
            WillingnessDecision::Refused(RefusalReason::CriticalStress)
        );
        input.priority = TaskPriority::Emergency;
        assert_eq!(evaluate_willingness(input), WillingnessDecision::Willing);
    }

    #[test]
    fn pregnant_or_injured_cat_rejects_high_risk_only_when_safer_worker_exists() {
        let mut input = context(0);
        input.risk = TaskRisk::High;
        input.pregnant = true;
        assert_eq!(evaluate_willingness(input), WillingnessDecision::Willing);
        input.safer_eligible_worker_exists = true;
        assert_eq!(
            evaluate_willingness(input),
            WillingnessDecision::Refused(RefusalReason::ProtectedFromHighRisk)
        );
        input.pregnant = false;
        input.injured = true;
        assert_eq!(
            evaluate_willingness(input),
            WillingnessDecision::Refused(RefusalReason::ProtectedFromHighRisk)
        );
    }

    #[test]
    fn burnout_and_devotion_compose_as_fixed_point_willingness() {
        let mut traits = AcquiredTraitState::default();
        traits.traits.insert(AcquiredTrait::BurnedOut);
        traits.traits.insert(AcquiredTrait::Devoted);
        assert_eq!(
            acquired_willingness_factor(&traits, TaskPriority::Required, true).get(),
            9_000
        );
        assert_eq!(
            acquired_willingness_factor(&traits, TaskPriority::Emergency, true).get(),
            12_000
        );
    }

    #[test]
    fn no_willing_worker_is_distinct_and_does_not_force_assignment() {
        let refused = WorkerCandidate {
            eligible: true,
            decision: WillingnessDecision::Refused(RefusalReason::Stress),
        };
        assert_eq!(
            assignment_availability(&[refused, refused]),
            AssignmentAvailability::Blocked(AssignmentBlockReason::NoWillingWorker)
        );
        assert_eq!(
            assignment_availability(&[]),
            AssignmentAvailability::Blocked(AssignmentBlockReason::NoEligibleWorker)
        );
    }

    #[test]
    fn cargo_prefers_pinned_then_nearest_with_stable_id_tie_break() {
        let cargo = RefusalCargo {
            cargo_id: "cargo-7".into(),
            quantity: 12,
        };
        let endpoints = vec![
            SafeStockpileEndpoint {
                endpoint_id: "stockpile-b".into(),
                colony_id: "colony-a".into(),
                route_cost: 3,
                safe: true,
            },
            SafeStockpileEndpoint {
                endpoint_id: "stockpile-a".into(),
                colony_id: "colony-a".into(),
                route_cost: 3,
                safe: true,
            },
            SafeStockpileEndpoint {
                endpoint_id: "pinned".into(),
                colony_id: "colony-a".into(),
                route_cost: 20,
                safe: true,
            },
            SafeStockpileEndpoint {
                endpoint_id: "foreign-nearest".into(),
                colony_id: "colony-b".into(),
                route_cost: 0,
                safe: true,
            },
            SafeStockpileEndpoint {
                endpoint_id: "unsafe-nearest".into(),
                colony_id: "colony-a".into(),
                route_cost: 0,
                safe: false,
            },
        ];
        assert_eq!(
            cargo_refusal_plan(Some(&cargo), "colony-a", Some("pinned"), &endpoints),
            CargoRefusalPlan::DeliverThenLeave {
                cargo: cargo.clone(),
                endpoint_id: "pinned".into()
            }
        );
        assert_eq!(
            cargo_refusal_plan(Some(&cargo), "colony-a", Some("lost"), &endpoints),
            CargoRefusalPlan::DeliverThenLeave {
                cargo: cargo.clone(),
                endpoint_id: "stockpile-a".into()
            }
        );
        assert_eq!(
            cargo_refusal_plan(Some(&cargo), "colony-a", None, &[]),
            CargoRefusalPlan::HoldCargoUntilSafeEndpoint {
                cargo: cargo.clone()
            }
        );
        assert_eq!(
            cargo_refusal_plan(None, "colony-a", None, &endpoints),
            CargoRefusalPlan::NoCargo
        );
    }

    #[test]
    fn consumed_station_step_commits_once_then_releases() {
        let mut boundary = StationRefusalBoundary {
            station_id: "station-a".into(),
            cycle_id: "cycle-a".into(),
            inputs_consumed: true,
            output_committed: false,
            worker_released: false,
        };
        assert_eq!(
            boundary.resolve_refusal(),
            StationRefusalResult::CompletedConsumedStepAndReleased
        );
        assert!(boundary.output_committed);
        assert!(boundary.worker_released);
        assert_eq!(
            boundary.resolve_refusal(),
            StationRefusalResult::AlreadyReleased
        );
    }

    #[test]
    fn unconsumed_station_step_releases_without_output() {
        let mut boundary = StationRefusalBoundary {
            station_id: "station-a".into(),
            cycle_id: "cycle-a".into(),
            inputs_consumed: false,
            output_committed: false,
            worker_released: false,
        };
        assert_eq!(
            boundary.resolve_refusal(),
            StationRefusalResult::ReleasedWithoutCompleting
        );
        assert!(!boundary.output_committed);
        assert!(boundary.worker_released);
    }
}
