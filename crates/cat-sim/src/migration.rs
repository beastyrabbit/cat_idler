//! Pure prosperity-migration policy and persistable probation state.
//!
//! This module returns decisions only. It does not create, house, or remove
//! cats, and its domain-separated hash cannot advance the breeding RNG chain.

use serde::{Deserialize, Serialize};

const MIGRATION_DOMAIN: &[u8] = b"idle-cat-forest/prosperity-migration/v1";
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

pub const DEFAULT_ESTABLISHMENT_GAME_MINUTES: u64 = 30 * 60;
pub const DEFAULT_COHORT_INTERVAL_GAME_MINUTES: u64 = 24 * 60;
pub const DEFAULT_HOUSING_DEADLINE_GAME_MINUTES: u64 = 36 * 60;
pub const DEFAULT_FOOD_PER_CAT: f64 = 5.0;
pub const DEFAULT_WATER_PER_CAT: f64 = 6.0;
pub const DEFAULT_MATERIALS_PER_CAT: f64 = 4.0;
pub const MAX_BASE_COHORT_SIZE: u32 = 4;
pub const MAX_BONUS_CAT_MODULUS: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MigrationPolicy {
    pub establishment_game_minutes: u64,
    pub cohort_interval_game_minutes: u64,
    pub housing_deadline_game_minutes: u64,
    pub food_per_cat: f64,
    pub water_per_cat: f64,
    pub materials_per_cat: f64,
    pub base_cohort_size: u32,
    /// A second cat arrives when the dedicated bucket hash is zero modulo this
    /// value. Zero disables the bonus cat.
    pub bonus_cat_modulus: u32,
}

impl Default for MigrationPolicy {
    fn default() -> Self {
        Self {
            establishment_game_minutes: DEFAULT_ESTABLISHMENT_GAME_MINUTES,
            cohort_interval_game_minutes: DEFAULT_COHORT_INTERVAL_GAME_MINUTES,
            housing_deadline_game_minutes: DEFAULT_HOUSING_DEADLINE_GAME_MINUTES,
            food_per_cat: DEFAULT_FOOD_PER_CAT,
            water_per_cat: DEFAULT_WATER_PER_CAT,
            materials_per_cat: DEFAULT_MATERIALS_PER_CAT,
            base_cohort_size: 1,
            bonus_cat_modulus: 4,
        }
    }
}

impl MigrationPolicy {
    /// Sanitize deserialized/user-authored policy data before evaluation. A zero
    /// interval or deadline becomes one minute, cohort size is bounded, negative
    /// bars clamp to zero, and non-finite bars return to documented defaults.
    #[must_use]
    pub fn normalized(self) -> Self {
        Self {
            cohort_interval_game_minutes: self.cohort_interval_game_minutes.max(1),
            housing_deadline_game_minutes: self.housing_deadline_game_minutes.max(1),
            food_per_cat: normalized_bar(self.food_per_cat, DEFAULT_FOOD_PER_CAT),
            water_per_cat: normalized_bar(self.water_per_cat, DEFAULT_WATER_PER_CAT),
            materials_per_cat: normalized_bar(self.materials_per_cat, DEFAULT_MATERIALS_PER_CAT),
            base_cohort_size: self.base_cohort_size.clamp(1, MAX_BASE_COHORT_SIZE),
            bonus_cat_modulus: self.bonus_cat_modulus.min(MAX_BONUS_CAT_MODULUS),
            ..self
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct MigrationInputs {
    pub world_seed: u32,
    pub colony_id: String,
    pub elapsed_game_minutes: u64,
    /// Established residents only. Probationary arrivals live in
    /// [`MigrationState`] until integration settles or removes them.
    pub resident_population: u32,
    pub housing_capacity: u32,
    pub food: f64,
    pub water: f64,
    pub materials: f64,
    pub in_crisis: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProbationaryMigrant {
    pub id: String,
    pub arrived_game_minute: u64,
    pub housing_deadline_game_minute: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct MigrationState {
    pub last_evaluated_cohort_bucket: Option<u64>,
    pub probationary_migrants: Vec<ProbationaryMigrant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct MigrationOutcome {
    pub arrivals: Vec<ProbationaryMigrant>,
    pub retained_migrant_ids: Vec<String>,
    pub departed_migrant_ids: Vec<String>,
}

/// Evaluate one migration boundary and update only migration-owned state. The
/// caller remains responsible for translating returned IDs into cat entities.
/// Deadline expiry deliberately runs before vacancy assignment: a bed appearing
/// exactly at the deadline is too late, while any earlier evaluation retains the
/// migrant permanently.
pub fn advance_migration(
    policy: &MigrationPolicy,
    state: &mut MigrationState,
    input: &MigrationInputs,
) -> MigrationOutcome {
    let policy = policy.normalized();
    let mut outcome = MigrationOutcome::default();
    expire_unhoused_migrants(
        state,
        input.elapsed_game_minutes,
        &mut outcome.departed_migrant_ids,
    );
    let mut remaining_vacancies = input
        .housing_capacity
        .saturating_sub(input.resident_population);
    allocate_vacancies(
        state,
        &mut remaining_vacancies,
        &mut outcome.retained_migrant_ids,
    );

    let Some(bucket) = cohort_bucket(&policy, input.elapsed_game_minutes) else {
        return outcome;
    };
    if state
        .last_evaluated_cohort_bucket
        .is_some_and(|last| bucket <= last)
    {
        return outcome;
    }
    state.last_evaluated_cohort_bucket = Some(bucket);

    let newly_retained = u32::try_from(outcome.retained_migrant_ids.len()).unwrap_or(u32::MAX);
    if !input.in_crisis && is_prosperous_with_retained(&policy, state, input, newly_retained) {
        outcome.arrivals = build_cohort(&policy, input, bucket);
        state
            .probationary_migrants
            .extend(outcome.arrivals.iter().cloned());
        allocate_vacancies(
            state,
            &mut remaining_vacancies,
            &mut outcome.retained_migrant_ids,
        );
    }
    outcome
}

#[must_use]
pub fn is_prosperous(
    policy: &MigrationPolicy,
    state: &MigrationState,
    input: &MigrationInputs,
) -> bool {
    is_prosperous_with_retained(&policy.normalized(), state, input, 0)
}

fn is_prosperous_with_retained(
    policy: &MigrationPolicy,
    state: &MigrationState,
    input: &MigrationInputs,
    newly_retained: u32,
) -> bool {
    let pending = u32::try_from(state.probationary_migrants.len()).unwrap_or(u32::MAX);
    let population = input
        .resident_population
        .saturating_add(pending)
        .saturating_add(newly_retained);
    if population == 0 {
        return false;
    }
    let population = f64::from(population);
    input.food.is_finite()
        && input.water.is_finite()
        && input.materials.is_finite()
        && input.food >= policy.food_per_cat * population
        && input.water >= policy.water_per_cat * population
        && input.materials >= policy.materials_per_cat * population
}

fn cohort_bucket(policy: &MigrationPolicy, elapsed_game_minutes: u64) -> Option<u64> {
    if elapsed_game_minutes < policy.establishment_game_minutes
        || policy.cohort_interval_game_minutes == 0
    {
        return None;
    }
    Some(
        (elapsed_game_minutes - policy.establishment_game_minutes)
            / policy.cohort_interval_game_minutes,
    )
}

fn build_cohort(
    policy: &MigrationPolicy,
    input: &MigrationInputs,
    bucket: u64,
) -> Vec<ProbationaryMigrant> {
    let hash = migration_bucket_hash(input.world_seed, &input.colony_id, bucket);
    let bonus = u32::from(
        policy.bonus_cat_modulus != 0 && hash.is_multiple_of(u64::from(policy.bonus_cat_modulus)),
    );
    let cohort_size = policy.base_cohort_size.saturating_add(bonus);
    let deadline = input
        .elapsed_game_minutes
        .saturating_add(policy.housing_deadline_game_minutes);
    (0..cohort_size)
        .map(|index| ProbationaryMigrant {
            id: format!("migrant-{hash:016x}-{bucket:016x}-{index:02x}"),
            arrived_game_minute: input.elapsed_game_minutes,
            housing_deadline_game_minute: deadline,
        })
        .collect()
}

fn expire_unhoused_migrants(state: &mut MigrationState, now: u64, departed: &mut Vec<String>) {
    state.probationary_migrants.sort_by(|left, right| {
        (
            left.housing_deadline_game_minute,
            left.arrived_game_minute,
            &left.id,
        )
            .cmp(&(
                right.housing_deadline_game_minute,
                right.arrived_game_minute,
                &right.id,
            ))
    });
    state.probationary_migrants.retain(|migrant| {
        if now >= migrant.housing_deadline_game_minute {
            departed.push(migrant.id.clone());
            false
        } else {
            true
        }
    });
}

fn allocate_vacancies(
    state: &mut MigrationState,
    remaining_vacancies: &mut u32,
    retained: &mut Vec<String>,
) {
    if *remaining_vacancies == 0 {
        return;
    }
    state.probationary_migrants.sort_by(|left, right| {
        (left.arrived_game_minute, &left.id).cmp(&(right.arrived_game_minute, &right.id))
    });
    let retained_count = usize::try_from(*remaining_vacancies)
        .unwrap_or(usize::MAX)
        .min(state.probationary_migrants.len());
    retained.extend(
        state
            .probationary_migrants
            .drain(..retained_count)
            .map(|migrant| migrant.id),
    );
    *remaining_vacancies =
        remaining_vacancies.saturating_sub(u32::try_from(retained_count).unwrap_or(u32::MAX));
}

fn normalized_bar(value: f64, fallback: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        fallback
    }
}

fn migration_bucket_hash(world_seed: u32, colony_id: &str, bucket: u64) -> u64 {
    let mut hash = FNV_OFFSET;
    for byte in MIGRATION_DOMAIN
        .iter()
        .chain(world_seed.to_le_bytes().iter())
        .chain(colony_id.as_bytes())
        .chain(bucket.to_le_bytes().iter())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prosperous(at_minute: u64) -> MigrationInputs {
        MigrationInputs {
            world_seed: 42,
            colony_id: "willow".to_owned(),
            elapsed_game_minutes: at_minute,
            resident_population: 6,
            housing_capacity: 6,
            food: 36.0,
            water: 42.0,
            materials: 30.0,
            in_crisis: false,
        }
    }

    #[test]
    fn migration_never_arrives_before_establishment_or_during_crisis() {
        let policy = MigrationPolicy::default();
        let mut state = MigrationState::default();
        let early = advance_migration(
            &policy,
            &mut state,
            &prosperous(policy.establishment_game_minutes - 1),
        );
        assert!(early.arrivals.is_empty());

        let mut crisis = prosperous(policy.establishment_game_minutes);
        crisis.in_crisis = true;
        assert!(
            advance_migration(&policy, &mut state, &crisis)
                .arrivals
                .is_empty()
        );
    }

    #[test]
    fn a_full_prosperous_village_accepts_an_over_capacity_cohort() {
        let policy = MigrationPolicy::default();
        let input = prosperous(policy.establishment_game_minutes);
        assert_eq!(input.resident_population, input.housing_capacity);
        let mut state = MigrationState::default();
        let outcome = advance_migration(&policy, &mut state, &input);
        assert!(!outcome.arrivals.is_empty());
        assert!(outcome.retained_migrant_ids.is_empty());
        assert_eq!(state.probationary_migrants.len(), outcome.arrivals.len());
    }

    #[test]
    fn prosperity_is_per_capita_and_cohorts_fire_only_once_per_slow_bucket() {
        let policy = MigrationPolicy::default();
        let mut small = prosperous(policy.establishment_game_minutes);
        small.resident_population = 2;
        small.food = 10.0;
        small.water = 12.0;
        small.materials = 8.0;
        assert!(is_prosperous(&policy, &MigrationState::default(), &small));

        let mut large = small.clone();
        large.resident_population = 3;
        assert!(!is_prosperous(&policy, &MigrationState::default(), &large));

        let mut state = MigrationState::default();
        assert!(
            !advance_migration(&policy, &mut state, &small)
                .arrivals
                .is_empty()
        );
        assert!(
            advance_migration(&policy, &mut state, &small)
                .arrivals
                .is_empty()
        );
        small.elapsed_game_minutes += policy.cohort_interval_game_minutes - 1;
        assert!(
            advance_migration(&policy, &mut state, &small)
                .arrivals
                .is_empty()
        );
        small.elapsed_game_minutes += 1;
        assert!(
            !advance_migration(&policy, &mut state, &small)
                .arrivals
                .is_empty()
        );
    }

    #[test]
    fn housing_before_the_deadline_retains_a_probationary_migrant() {
        let policy = MigrationPolicy::default();
        let mut state = MigrationState::default();
        let arrival = advance_migration(
            &policy,
            &mut state,
            &prosperous(policy.establishment_game_minutes),
        );
        let migrant = arrival.arrivals[0].clone();
        let mut housed = prosperous(migrant.housing_deadline_game_minute - 1);
        housed.housing_capacity += 1;
        let outcome = advance_migration(&policy, &mut state, &housed);
        assert!(outcome.retained_migrant_ids.contains(&migrant.id));
        assert!(!outcome.departed_migrant_ids.contains(&migrant.id));
        assert!(
            !state
                .probationary_migrants
                .iter()
                .any(|pending| pending.id == migrant.id)
        );
    }

    #[test]
    fn still_unhoused_migrants_depart_exactly_at_the_deadline() {
        let policy = MigrationPolicy::default();
        let mut before_state = MigrationState::default();
        let arrival = advance_migration(
            &policy,
            &mut before_state,
            &prosperous(policy.establishment_game_minutes),
        );
        let migrant = arrival.arrivals[0].clone();

        let before = advance_migration(
            &policy,
            &mut before_state,
            &prosperous(migrant.housing_deadline_game_minute - 1),
        );
        assert!(!before.departed_migrant_ids.contains(&migrant.id));

        let mut exact_state = MigrationState::default();
        advance_migration(
            &policy,
            &mut exact_state,
            &prosperous(policy.establishment_game_minutes),
        );
        let exact = advance_migration(
            &policy,
            &mut exact_state,
            &prosperous(migrant.housing_deadline_game_minute),
        );
        assert!(exact.departed_migrant_ids.contains(&migrant.id));
    }

    #[test]
    fn departure_decisions_only_name_probationary_migrants() {
        let policy = MigrationPolicy::default();
        let mut state = MigrationState {
            probationary_migrants: vec![ProbationaryMigrant {
                id: "migrant-only".to_owned(),
                arrived_game_minute: 1,
                housing_deadline_game_minute: 10,
            }],
            ..MigrationState::default()
        };
        let outcome = advance_migration(&policy, &mut state, &prosperous(10));
        assert_eq!(outcome.departed_migrant_ids, ["migrant-only"]);
        assert!(
            !outcome
                .departed_migrant_ids
                .iter()
                .any(|id| id == "native-cat")
        );
    }

    #[test]
    fn vacancy_allocation_is_stable_by_arrival_then_id() {
        let policy = MigrationPolicy::default();
        let pending = [
            ProbationaryMigrant {
                id: "migrant-b".to_owned(),
                arrived_game_minute: 5,
                housing_deadline_game_minute: 100,
            },
            ProbationaryMigrant {
                id: "migrant-a".to_owned(),
                arrived_game_minute: 5,
                housing_deadline_game_minute: 100,
            },
        ];
        let input = MigrationInputs {
            elapsed_game_minutes: 10,
            resident_population: 6,
            housing_capacity: 7,
            ..MigrationInputs::default()
        };
        let mut left = MigrationState {
            probationary_migrants: pending.to_vec(),
            ..MigrationState::default()
        };
        let mut right = MigrationState {
            probationary_migrants: pending.into_iter().rev().collect(),
            ..MigrationState::default()
        };
        let left_outcome = advance_migration(&policy, &mut left, &input);
        let right_outcome = advance_migration(&policy, &mut right, &input);
        assert_eq!(left_outcome, right_outcome);
        assert_eq!(left_outcome.retained_migrant_ids, ["migrant-a"]);
        assert_eq!(left, right);
    }

    #[test]
    fn one_vacancy_cannot_retain_an_old_migrant_and_a_new_arrival() {
        let policy = MigrationPolicy::default();
        let old = ProbationaryMigrant {
            id: "old-pending".to_owned(),
            arrived_game_minute: 10,
            housing_deadline_game_minute: 10_000,
        };
        let mut state = MigrationState {
            probationary_migrants: vec![old.clone()],
            ..MigrationState::default()
        };
        let mut input = prosperous(policy.establishment_game_minutes);
        input.housing_capacity = input.resident_population + 1;
        let outcome = advance_migration(&policy, &mut state, &input);
        assert_eq!(outcome.retained_migrant_ids, [old.id]);
        assert!(!outcome.arrivals.is_empty());
        assert_eq!(state.probationary_migrants, outcome.arrivals);
    }

    #[test]
    fn invalid_public_policy_values_normalize_to_bounded_safe_values() {
        let normalized = MigrationPolicy {
            cohort_interval_game_minutes: 0,
            housing_deadline_game_minutes: 0,
            food_per_cat: f64::NAN,
            water_per_cat: -5.0,
            materials_per_cat: f64::INFINITY,
            base_cohort_size: 0,
            bonus_cat_modulus: u32::MAX,
            ..MigrationPolicy::default()
        }
        .normalized();
        assert_eq!(normalized.cohort_interval_game_minutes, 1);
        assert_eq!(normalized.housing_deadline_game_minutes, 1);
        assert_eq!(normalized.food_per_cat, DEFAULT_FOOD_PER_CAT);
        assert_eq!(normalized.water_per_cat, 0.0);
        assert_eq!(normalized.materials_per_cat, DEFAULT_MATERIALS_PER_CAT);
        assert_eq!(normalized.base_cohort_size, 1);
        assert_eq!(normalized.bonus_cat_modulus, MAX_BONUS_CAT_MODULUS);
    }

    #[test]
    fn persisted_state_defaults_missing_fields_and_round_trips() {
        let old: MigrationState = serde_json::from_str("{}").unwrap();
        assert_eq!(old, MigrationState::default());
        let old_policy: MigrationPolicy = serde_json::from_str("{}").unwrap();
        assert_eq!(old_policy, MigrationPolicy::default());
        let partial: MigrationState = serde_json::from_str(
            r#"{"probationaryMigrants":[{"id":"old","arrivedGameMinute":5,"housingDeadlineGameMinute":9}]}"#,
        )
        .unwrap();
        assert_eq!(partial.last_evaluated_cohort_bucket, None);
        let encoded = serde_json::to_string(&partial).unwrap();
        assert_eq!(
            serde_json::from_str::<MigrationState>(&encoded).unwrap(),
            partial
        );
    }

    #[test]
    fn deterministic_twins_emit_identical_cohorts_without_rng_state() {
        let policy = MigrationPolicy::default();
        let input = prosperous(policy.establishment_game_minutes);
        let mut left = MigrationState::default();
        let mut right = MigrationState::default();
        assert_eq!(
            advance_migration(&policy, &mut left, &input),
            advance_migration(&policy, &mut right, &input)
        );
        assert_eq!(left, right);
    }
}
