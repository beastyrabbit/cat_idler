//! Pure prosperity-migration policy and persistable probation state.
//!
//! This module returns decisions only. It does not create, house, or remove
//! cats, and its domain-separated hash cannot advance the breeding RNG chain.

use serde::{Deserialize, Serialize};

use crate::production::WORKSHOP_MATERIALS_PER_CYCLE;

const MIGRATION_DOMAIN: &[u8] = b"idle-cat-forest/prosperity-migration/v1";
const MIGRATION_ID_DOMAIN: &[u8] = b"idle-cat-forest/prosperity-migration-id/v2";
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

pub const DEFAULT_ESTABLISHMENT_GAME_MINUTES: u64 = 30 * 60;
pub const DEFAULT_COHORT_INTERVAL_GAME_MINUTES: u64 = 12 * 60;
pub const DEFAULT_HOUSING_DEADLINE_GAME_MINUTES: u64 = 36 * 60;
pub const DEFAULT_FOOD_PER_CAT: f64 = 4.0;
pub const DEFAULT_WATER_PER_CAT: f64 = 5.0;
/// Construction readiness is intentionally a modest reachable reserve, not a
/// storage-capacity ratio: the founding economy continuously converts raw material
/// into directly buildable timber and blocks.
pub const DEFAULT_MATERIALS_PER_CAT: f64 = 0.5;
pub const DEFAULT_MATERIALS_FLOOR: f64 = 8.0;
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
    pub materials_floor: f64,
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
            materials_floor: DEFAULT_MATERIALS_FLOOR,
            base_cohort_size: 1,
            bonus_cat_modulus: 0,
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
            materials_floor: normalized_bar(self.materials_floor, DEFAULT_MATERIALS_FLOOR),
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
    /// Colony run is part of the migrant identity domain. A post-collapse cohort
    /// at the same game-minute bucket must never reuse ids from the graveyard.
    pub run_number: u32,
    pub elapsed_game_minutes: u64,
    /// Established residents only. Probationary arrivals live in
    /// [`MigrationState`] until integration settles or removes them.
    pub resident_population: u32,
    /// Beds already promised to living pregnancies. Reservations reduce migration
    /// vacancies but do not consume food/water/construction prosperity bars yet.
    pub housing_reservations: u32,
    pub housing_capacity: u32,
    pub food: f64,
    pub water: f64,
    /// Raw-equivalent value of materials already held as raw stock or directly
    /// buildable planks, blocks, and lumber.
    #[serde(alias = "materials")]
    pub construction_wealth: f64,
    pub in_crisis: bool,
}

/// Raw-equivalent value of the colony's directly buildable construction stock.
///
/// One plank or block consumes [`WORKSHOP_MATERIALS_PER_CYCLE`] raw materials in the
/// actual refinement recipe. `allocate_construction_timber` spends lumber and legacy
/// planks one-for-one, so lumber carries the same raw-equivalent value as a plank.
/// Logs, tools, refined trade goods, and other inventory are intentionally excluded:
/// they cannot pay a scaffold cost directly.
#[must_use]
pub fn migration_construction_wealth(materials: f64, planks: f64, blocks: f64, lumber: f64) -> f64 {
    if !materials.is_finite() || !planks.is_finite() || !blocks.is_finite() || !lumber.is_finite() {
        return f64::NAN;
    }
    materials.max(0.0)
        + (planks.max(0.0) + blocks.max(0.0) + lumber.max(0.0)) * WORKSHOP_MATERIALS_PER_CYCLE
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ProbationaryMigrant {
    pub id: String,
    pub arrived_game_minute: u64,
    pub housing_deadline_game_minute: u64,
    /// Physical journey through the village gate. Legacy saves predate spatial
    /// migration and therefore deserialize as an already-present probationer.
    pub phase: MigrantSpatialPhase,
    /// Deterministic exterior landing/exit tile. Persisting the authored south
    /// approach lets an arriving cat visibly wait on a blocked route and later
    /// reuse the same physical origin when leaving.
    #[serde(default)]
    pub route_exterior: Option<[i32; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MigrantSpatialPhase {
    Arriving,
    #[default]
    Probationary,
    Departing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct MigrationState {
    /// Historical field name retained for save compatibility. The cursor records
    /// the newest cohort bucket that actually produced an arrival, not a failed
    /// prosperity sample: a healthy colony gets the whole interval to qualify.
    pub last_evaluated_cohort_bucket: Option<u64>,
    /// Cohort whose physical arrival is currently blocked at the village gate.
    /// This scopes the diagnostic dedup to one episode instead of event-log history.
    pub deferred_cohort_bucket: Option<u64>,
    pub probationary_migrants: Vec<ProbationaryMigrant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct MigrationOutcome {
    pub arrivals: Vec<ProbationaryMigrant>,
    pub retained_migrant_ids: Vec<String>,
    pub departed_migrant_ids: Vec<String>,
}

#[must_use]
pub fn migrant_phase(state: &MigrationState, id: &str) -> Option<MigrantSpatialPhase> {
    state
        .probationary_migrants
        .iter()
        .find(|migrant| migrant.id == id)
        .map(|migrant| migrant.phase)
}

/// Begin the housing clock only once the arriving cat physically crosses the
/// gate. A blocked route therefore cannot consume the promised 36-hour stay.
pub fn mark_migrant_arrived(state: &mut MigrationState, id: &str, now: u64) -> bool {
    let Some(migrant) = state
        .probationary_migrants
        .iter_mut()
        .find(|migrant| migrant.id == id && migrant.phase == MigrantSpatialPhase::Arriving)
    else {
        return false;
    };
    let probation_duration = migrant
        .housing_deadline_game_minute
        .saturating_sub(migrant.arrived_game_minute);
    migrant.arrived_game_minute = now;
    migrant.housing_deadline_game_minute = now.saturating_add(probation_duration);
    migrant.phase = MigrantSpatialPhase::Probationary;
    true
}

pub fn finish_migrant_departure(state: &mut MigrationState, id: &str) -> bool {
    let before = state.probationary_migrants.len();
    state
        .probationary_migrants
        .retain(|migrant| migrant.id != id || migrant.phase != MigrantSpatialPhase::Departing);
    state.probationary_migrants.len() != before
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
        .saturating_sub(input.resident_population)
        .saturating_sub(input.housing_reservations);
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
    let newly_retained = u32::try_from(outcome.retained_migrant_ids.len()).unwrap_or(u32::MAX);
    if !input.in_crisis && is_prosperous_with_retained(&policy, state, input, newly_retained) {
        state.last_evaluated_cohort_bucket = Some(bucket);
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
    let pending = u32::try_from(
        state
            .probationary_migrants
            .iter()
            .filter(|migrant| migrant.phase != MigrantSpatialPhase::Departing)
            .count(),
    )
    .unwrap_or(u32::MAX);
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
        && input.construction_wealth.is_finite()
        && input.food >= policy.food_per_cat * population
        && input.water >= policy.water_per_cat * population
        && input.construction_wealth
            >= (policy.materials_per_cat * population).max(policy.materials_floor)
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
    // Cohort sizing keeps its established run-independent schedule so adding a
    // run namespace cannot silently rebalance a live colony. Only identity is
    // run-scoped. Run 1 retains its legacy id spelling for save compatibility.
    let hash = migration_bucket_hash(input.world_seed, &input.colony_id, bucket);
    let identity_hash = (input.run_number > 1).then(|| {
        migration_identity_hash(input.world_seed, &input.colony_id, input.run_number, bucket)
    });
    let bonus = u32::from(
        policy.bonus_cat_modulus != 0 && hash.is_multiple_of(u64::from(policy.bonus_cat_modulus)),
    );
    let cohort_size = policy.base_cohort_size.saturating_add(bonus);
    let deadline = input
        .elapsed_game_minutes
        .saturating_add(policy.housing_deadline_game_minutes);
    (0..cohort_size)
        .map(|index| ProbationaryMigrant {
            id: identity_hash.map_or_else(
                || format!("migrant-{hash:016x}-{bucket:016x}-{index:02x}"),
                |identity_hash| {
                    format!(
                        "migrant-{identity_hash:016x}-r{:08x}-{bucket:016x}-{index:02x}",
                        input.run_number
                    )
                },
            ),
            arrived_game_minute: input.elapsed_game_minutes,
            housing_deadline_game_minute: deadline,
            phase: MigrantSpatialPhase::Arriving,
            route_exterior: None,
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
    for migrant in &mut state.probationary_migrants {
        if migrant.phase == MigrantSpatialPhase::Probationary
            && now >= migrant.housing_deadline_game_minute
        {
            departed.push(migrant.id.clone());
            migrant.phase = MigrantSpatialPhase::Departing;
        }
    }
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
    let retained_ids = state
        .probationary_migrants
        .iter()
        .filter(|migrant| migrant.phase == MigrantSpatialPhase::Probationary)
        .take(usize::try_from(*remaining_vacancies).unwrap_or(usize::MAX))
        .map(|migrant| migrant.id.clone())
        .collect::<Vec<_>>();
    let retained_count = retained_ids.len();
    retained.extend(retained_ids.iter().cloned());
    state
        .probationary_migrants
        .retain(|migrant| !retained_ids.contains(&migrant.id));
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

fn migration_identity_hash(world_seed: u32, colony_id: &str, run_number: u32, bucket: u64) -> u64 {
    let mut hash = FNV_OFFSET;
    for byte in MIGRATION_ID_DOMAIN
        .iter()
        .chain(world_seed.to_le_bytes().iter())
        .chain(colony_id.as_bytes())
        .chain(run_number.to_le_bytes().iter())
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
            run_number: 1,
            elapsed_game_minutes: at_minute,
            resident_population: 6,
            housing_reservations: 0,
            housing_capacity: 6,
            food: 36.0,
            water: 42.0,
            construction_wealth: 30.0,
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
        small.construction_wealth = 8.0;
        assert!(is_prosperous(&policy, &MigrationState::default(), &small));

        let mut large = small.clone();
        large.resident_population = 3;
        assert!(!is_prosperous(&policy, &MigrationState::default(), &large));

        let mut state = MigrationState::default();
        let first = advance_migration(&policy, &mut state, &small);
        assert!(!first.arrivals.is_empty());
        assert!(mark_migrant_arrived(
            &mut state,
            &first.arrivals[0].id,
            small.elapsed_game_minutes
        ));
        let retained = advance_migration(&policy, &mut state, &small);
        assert_eq!(
            retained.retained_migrant_ids,
            [first.arrivals[0].id.clone()]
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
    fn processed_build_stock_counts_toward_prosperity_but_raw_scraps_do_not() {
        let policy = MigrationPolicy::default();
        let mut input = prosperous(policy.establishment_game_minutes);
        input.resident_population = 15;
        input.housing_capacity = 30;
        input.food = 60.0;
        input.water = 75.0;

        input.construction_wealth = migration_construction_wealth(1.0, 2.0, 1.0, 0.0);
        assert_eq!(input.construction_wealth, 16.0);
        assert!(
            is_prosperous(&policy, &MigrationState::default(), &input),
            "processed build stock is real construction wealth for prosperity"
        );

        input.construction_wealth = migration_construction_wealth(1.0, 0.0, 0.0, 0.0);
        assert!(!is_prosperous(&policy, &MigrationState::default(), &input));
    }

    #[test]
    fn scaffold_spending_reduces_construction_wealth_by_its_raw_cost() {
        let raw = 1.0;
        let planks = 3.0;
        let blocks = 2.0;
        let lumber = 1.0;
        let before = migration_construction_wealth(raw, planks, blocks, lumber);
        let timber = crate::processing::allocate_construction_timber(2.0, lumber, planks);
        assert!(timber.covered);
        let after = migration_construction_wealth(
            raw,
            planks - timber.legacy_planks_used,
            blocks - 2.0,
            lumber - timber.lumber_used,
        );

        assert_eq!(before - after, 4.0 * WORKSHOP_MATERIALS_PER_CYCLE);
        assert_eq!(
            migration_construction_wealth(raw, planks, blocks, lumber).to_bits(),
            before.to_bits(),
            "the pure wealth projection is deterministic"
        );
    }

    #[test]
    fn a_cohort_window_stays_open_until_prosperity_then_fires_only_once() {
        let policy = MigrationPolicy::default();
        let mut state = MigrationState::default();
        let mut input = prosperous(policy.establishment_game_minutes);
        input.food = 0.0;

        let lean = advance_migration(&policy, &mut state, &input);
        assert!(lean.arrivals.is_empty());
        assert_eq!(state.last_evaluated_cohort_bucket, None);

        input.elapsed_game_minutes += policy.cohort_interval_game_minutes / 2;
        input.food = 100.0;
        let recovered = advance_migration(&policy, &mut state, &input);
        assert!(!recovered.arrivals.is_empty());
        assert_eq!(state.last_evaluated_cohort_bucket, Some(0));

        let duplicate = advance_migration(&policy, &mut state, &input);
        assert!(duplicate.arrivals.is_empty());
        assert_eq!(state.last_evaluated_cohort_bucket, Some(0));
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
        assert!(mark_migrant_arrived(
            &mut state,
            &migrant.id,
            policy.establishment_game_minutes
        ));
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
        assert!(mark_migrant_arrived(
            &mut before_state,
            &migrant.id,
            policy.establishment_game_minutes
        ));

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
        assert!(mark_migrant_arrived(
            &mut exact_state,
            &migrant.id,
            policy.establishment_game_minutes
        ));
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
                phase: MigrantSpatialPhase::Probationary,
                route_exterior: None,
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
                phase: MigrantSpatialPhase::Probationary,
                route_exterior: None,
            },
            ProbationaryMigrant {
                id: "migrant-a".to_owned(),
                arrived_game_minute: 5,
                housing_deadline_game_minute: 100,
                phase: MigrantSpatialPhase::Probationary,
                route_exterior: None,
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
            phase: MigrantSpatialPhase::Probationary,
            route_exterior: None,
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
    fn pregnancy_reservation_and_migrant_never_claim_the_same_last_bed() {
        let policy = MigrationPolicy::default();
        let mut input = prosperous(policy.establishment_game_minutes);
        input.housing_capacity = input.resident_population + 1;
        input.housing_reservations = 1;

        let mut state = MigrationState::default();
        let outcome = advance_migration(&policy, &mut state, &input);

        assert!(!outcome.arrivals.is_empty());
        assert!(outcome.retained_migrant_ids.is_empty());
        assert_eq!(state.probationary_migrants, outcome.arrivals);
    }

    #[test]
    fn reset_run_domain_never_reuses_a_prior_cohort_id() {
        let policy = MigrationPolicy::default();
        let mut first_input = prosperous(policy.establishment_game_minutes);
        first_input.run_number = 1;
        let mut first_state = MigrationState::default();
        let first = advance_migration(&policy, &mut first_state, &first_input);

        let mut next_input = first_input;
        next_input.run_number = 2;
        let mut next_state = MigrationState::default();
        let next = advance_migration(&policy, &mut next_state, &next_input);

        assert!(!first.arrivals.is_empty());
        assert!(!next.arrivals.is_empty());
        assert!(
            first
                .arrivals
                .iter()
                .all(|old| next.arrivals.iter().all(|new| old.id != new.id))
        );
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
        assert_eq!(
            partial.probationary_migrants[0].phase,
            MigrantSpatialPhase::Probationary
        );
        assert_eq!(partial.probationary_migrants[0].route_exterior, None);
        let encoded = serde_json::to_string(&partial).unwrap();
        assert_eq!(
            serde_json::from_str::<MigrationState>(&encoded).unwrap(),
            partial
        );
    }

    #[test]
    fn arriving_migrant_cannot_take_a_bed_or_expire_before_crossing_the_gate() {
        let policy = MigrationPolicy::default();
        let mut state = MigrationState::default();
        let mut input = prosperous(policy.establishment_game_minutes);
        input.housing_capacity += 1;
        let arrival = advance_migration(&policy, &mut state, &input);
        let migrant = &arrival.arrivals[0];
        assert_eq!(migrant.phase, MigrantSpatialPhase::Arriving);
        assert!(arrival.retained_migrant_ids.is_empty());

        input.elapsed_game_minutes = migrant.housing_deadline_game_minute + 100;
        let blocked = advance_migration(&policy, &mut state, &input);
        assert!(blocked.departed_migrant_ids.is_empty());
        assert_eq!(
            migrant_phase(&state, &migrant.id),
            Some(MigrantSpatialPhase::Arriving)
        );
    }

    #[test]
    fn departure_record_is_durable_until_the_physical_exit_finishes() {
        let mut state = MigrationState {
            probationary_migrants: vec![ProbationaryMigrant {
                id: "leaving".to_owned(),
                arrived_game_minute: 1,
                housing_deadline_game_minute: 2,
                phase: MigrantSpatialPhase::Probationary,
                route_exterior: None,
            }],
            ..MigrationState::default()
        };
        let outcome = advance_migration(&MigrationPolicy::default(), &mut state, &prosperous(2));
        assert_eq!(outcome.departed_migrant_ids, ["leaving"]);
        assert_eq!(
            migrant_phase(&state, "leaving"),
            Some(MigrantSpatialPhase::Departing)
        );
        assert!(finish_migrant_departure(&mut state, "leaving"));
        assert_eq!(migrant_phase(&state, "leaving"), None);
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
