//! Family housing, partnerships, residence moves, elder support, and teaching obligations for LAI.56.
//!
//! Port target: `docs/leader-ai-overhaul/final-integrated-overhaul-plan.md`
//! section 3, "Partnerships, mentoring, and housing". This leaf is pure and
//! authority-free; later LAI.63+ owners attach it to world tasks, reservations,
//! persistence, protocol, and UI.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const FAMILY_HOUSING_SCHEMA_VERSION: u16 = 1;
pub const DEN_BEDS: u8 = 5;
pub const FAMILY_HOME_PARTNERED_ADULT_BEDS: u8 = 2;
pub const FAMILY_HOME_DEPENDENT_BEDS: u8 = 4;
pub const ELDER_LODGE_BEDS: u8 = 8;
pub const NURSERY_PERMANENT_BEDS: u8 = 0;
pub const TEACHING_OBLIGATION_REAL_TASKS: u8 = 3;
pub const ELDER_LODGE_MIN_OLD_AGE_HAZARD_BASIS_POINTS: u16 = 100;
pub const ELDER_LODGE_BASE_HAZARD_REDUCTION_BASIS_POINTS: u16 = 1_500;
pub const ELDER_LODGE_LEVEL_HAZARD_REDUCTION_BASIS_POINTS: u16 = 250;
pub const ELDER_LODGE_SOCIAL_RECOVERY_BASIS_POINTS: u16 = 1_100;
pub const ELDER_LODGE_MENTORING_BASIS_POINTS: u16 = 1_150;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifeStage {
    Kitten,
    Young,
    Adult,
    Elder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HousingKind {
    Den,
    FamilyHome,
    ElderLodge,
    Nursery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HousingCapacity {
    pub kind: HousingKind,
    pub flexible_beds: u8,
    pub partnered_adult_beds: u8,
    pub dependent_beds: u8,
    pub elder_beds: u8,
    pub permanent_beds: u8,
}

#[must_use]
pub const fn housing_capacity(kind: HousingKind) -> HousingCapacity {
    match kind {
        HousingKind::Den => HousingCapacity {
            kind,
            flexible_beds: DEN_BEDS,
            partnered_adult_beds: 0,
            dependent_beds: 0,
            elder_beds: 0,
            permanent_beds: DEN_BEDS,
        },
        HousingKind::FamilyHome => HousingCapacity {
            kind,
            flexible_beds: 0,
            partnered_adult_beds: FAMILY_HOME_PARTNERED_ADULT_BEDS,
            dependent_beds: FAMILY_HOME_DEPENDENT_BEDS,
            elder_beds: 0,
            permanent_beds: FAMILY_HOME_PARTNERED_ADULT_BEDS + FAMILY_HOME_DEPENDENT_BEDS,
        },
        HousingKind::ElderLodge => HousingCapacity {
            kind,
            flexible_beds: 0,
            partnered_adult_beds: 0,
            dependent_beds: 0,
            elder_beds: ELDER_LODGE_BEDS,
            permanent_beds: ELDER_LODGE_BEDS,
        },
        HousingKind::Nursery => HousingCapacity {
            kind,
            flexible_beds: 0,
            partnered_adult_beds: 0,
            dependent_beds: 0,
            elder_beds: 0,
            permanent_beds: NURSERY_PERMANENT_BEDS,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HouseholdProfile {
    pub household_id: String,
    pub adult_cat_ids: Vec<String>,
    pub dependent_cat_ids: Vec<String>,
    pub elder_cat_ids: Vec<String>,
    pub pregnant_or_parenting: bool,
    pub empty_nest: bool,
    pub current_housing: HousingKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HousingMoveReason {
    ParentingFamilyHomePriority,
    ElderLodgeFreesFamilyHome,
    EmptyNestPressureReturnToDen,
    StayPut,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HousingMoveKey {
    pub reason: HousingMoveReason,
    pub priority_rank: u8,
    pub household_id: String,
}

#[must_use]
pub fn housing_move_recommendation(
    household: &HouseholdProfile,
    family_home_has_capacity: bool,
    elder_lodge_has_capacity: bool,
    pressure_requires_den_return: bool,
) -> HousingMoveKey {
    let (reason, priority_rank) = if !household.elder_cat_ids.is_empty()
        && household.current_housing != HousingKind::ElderLodge
        && elder_lodge_has_capacity
    {
        (HousingMoveReason::ElderLodgeFreesFamilyHome, 0)
    } else if household.pregnant_or_parenting
        && household.current_housing != HousingKind::FamilyHome
        && family_home_has_capacity
    {
        (HousingMoveReason::ParentingFamilyHomePriority, 1)
    } else if household.empty_nest
        && household.current_housing == HousingKind::FamilyHome
        && pressure_requires_den_return
    {
        (HousingMoveReason::EmptyNestPressureReturnToDen, 2)
    } else {
        (HousingMoveReason::StayPut, 9)
    };
    HousingMoveKey {
        reason,
        priority_rank,
        household_id: household.household_id.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartnershipCandidate {
    pub cat_id: String,
    pub close_ancestor_or_descendant_ids: BTreeSet<String>,
    pub close_sibling_ids: BTreeSet<String>,
    pub inherited_attribute_score: u16,
    pub profession_skill_level: u16,
    pub personality_compatibility_basis_points: u16,
    pub relational_analytical: i16,
    pub tradition_ids: BTreeSet<String>,
    pub housing_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartnershipScoreKey {
    pub compatibility_rank: u32,
    pub housing_rank: u8,
    pub stable_pair_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartnershipAuthority {
    Autonomous,
    GodArranged,
}

#[must_use]
pub const fn partnership_authority_allowed(authority: PartnershipAuthority) -> bool {
    matches!(authority, PartnershipAuthority::Autonomous)
}

#[must_use]
pub fn partnership_score(
    first: &PartnershipCandidate,
    second: &PartnershipCandidate,
    colony_seed: u64,
    authority: PartnershipAuthority,
) -> Option<PartnershipScoreKey> {
    if !partnership_authority_allowed(authority) || close_kin_excluded(first, second) {
        return None;
    }
    let shared_traditions = first
        .tradition_ids
        .intersection(&second.tradition_ids)
        .count() as u32;
    let axis_distance = first
        .relational_analytical
        .saturating_sub(second.relational_analytical)
        .unsigned_abs() as u32;
    let deterministic_preference =
        keyed_pair_hash(&first.cat_id, &second.cat_id, colony_seed, "partnership") % 1_000;
    let score = u32::from(first.inherited_attribute_score)
        .saturating_add(u32::from(second.inherited_attribute_score))
        .saturating_mul(20)
        .saturating_add(
            u32::from(first.profession_skill_level)
                .saturating_add(u32::from(second.profession_skill_level))
                .saturating_mul(10),
        )
        .saturating_add(u32::from(
            first.personality_compatibility_basis_points / 100,
        ))
        .saturating_add(u32::from(
            second.personality_compatibility_basis_points / 100,
        ))
        .saturating_add(shared_traditions * 250)
        .saturating_sub(axis_distance)
        .saturating_add(deterministic_preference as u32);
    Some(PartnershipScoreKey {
        compatibility_rank: u32::MAX - score,
        housing_rank: if first.housing_ready && second.housing_ready {
            0
        } else {
            1
        },
        stable_pair_id: stable_pair_id(&first.cat_id, &second.cat_id),
    })
}

#[must_use]
pub fn close_kin_excluded(first: &PartnershipCandidate, second: &PartnershipCandidate) -> bool {
    first
        .close_ancestor_or_descendant_ids
        .contains(&second.cat_id)
        || second
            .close_ancestor_or_descendant_ids
            .contains(&first.cat_id)
        || first.close_sibling_ids.contains(&second.cat_id)
        || second.close_sibling_ids.contains(&first.cat_id)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingObligation {
    pub parent_cat_id: String,
    pub dependent_cat_id: String,
    pub completed_real_tasks_since_teach: u8,
    pub due: bool,
    pub deferred_by_emergency: bool,
}

impl TeachingObligation {
    #[must_use]
    pub fn new(parent_cat_id: impl Into<String>, dependent_cat_id: impl Into<String>) -> Self {
        Self {
            parent_cat_id: parent_cat_id.into(),
            dependent_cat_id: dependent_cat_id.into(),
            completed_real_tasks_since_teach: 0,
            due: false,
            deferred_by_emergency: false,
        }
    }
}

#[must_use]
pub fn record_parent_real_task(mut obligation: TeachingObligation) -> TeachingObligation {
    if !obligation.due {
        obligation.completed_real_tasks_since_teach = obligation
            .completed_real_tasks_since_teach
            .saturating_add(1)
            .min(TEACHING_OBLIGATION_REAL_TASKS);
        if obligation.completed_real_tasks_since_teach >= TEACHING_OBLIGATION_REAL_TASKS {
            obligation.due = true;
        }
    }
    obligation
}

#[must_use]
pub fn defer_for_emergency(mut obligation: TeachingObligation) -> TeachingObligation {
    if obligation.due {
        obligation.deferred_by_emergency = true;
    }
    obligation
}

#[must_use]
pub fn complete_teaching(mut obligation: TeachingObligation) -> TeachingObligation {
    obligation.completed_real_tasks_since_teach = 0;
    obligation.due = false;
    obligation.deferred_by_emergency = false;
    obligation
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CareActivityChoice {
    PhysicalTeaching {
        site: TeachingSite,
        mentor_cat_id: String,
    },
    EmergencyWork,
    AmbientCleaning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeachingSite {
    FamilyHome,
    Nursery,
    School,
    Office,
    Enterprise,
    Den,
}

#[must_use]
pub const fn teaching_site_allowed(site: TeachingSite) -> bool {
    matches!(
        site,
        TeachingSite::FamilyHome
            | TeachingSite::Nursery
            | TeachingSite::School
            | TeachingSite::Office
            | TeachingSite::Enterprise
    )
}

#[must_use]
pub fn choose_care_activity(
    obligation: &TeachingObligation,
    assigned_non_parent_mentor_id: Option<&str>,
    site: TeachingSite,
) -> CareActivityChoice {
    if obligation.due && obligation.deferred_by_emergency {
        return CareActivityChoice::EmergencyWork;
    }
    if obligation.due && !obligation.deferred_by_emergency && teaching_site_allowed(site) {
        let mentor = assigned_non_parent_mentor_id
            .unwrap_or(&obligation.parent_cat_id)
            .to_owned();
        return CareActivityChoice::PhysicalTeaching {
            site,
            mentor_cat_id: mentor,
        };
    }
    CareActivityChoice::AmbientCleaning
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ElderLodgeEffects {
    pub old_age_hazard_basis_points: u16,
    pub social_recovery_basis_points: u16,
    pub mentoring_basis_points: u16,
    pub immortality: bool,
}

#[must_use]
pub fn elder_lodge_effects(
    base_old_age_hazard_basis_points: u16,
    building_level: u8,
) -> ElderLodgeEffects {
    let reduction = u32::from(ELDER_LODGE_BASE_HAZARD_REDUCTION_BASIS_POINTS).saturating_add(
        u32::from(building_level.saturating_sub(1))
            .saturating_mul(u32::from(ELDER_LODGE_LEVEL_HAZARD_REDUCTION_BASIS_POINTS)),
    );
    let reduced = u32::from(base_old_age_hazard_basis_points).saturating_sub(reduction);
    ElderLodgeEffects {
        old_age_hazard_basis_points: reduced
            .max(u32::from(ELDER_LODGE_MIN_OLD_AGE_HAZARD_BASIS_POINTS))
            .min(u32::from(u16::MAX)) as u16,
        social_recovery_basis_points: ELDER_LODGE_SOCIAL_RECOVERY_BASIS_POINTS,
        mentoring_basis_points: ELDER_LODGE_MENTORING_BASIS_POINTS,
        immortality: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionedFamilyHousingState {
    pub schema_version: u16,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub households: BTreeMap<String, HouseholdProfile>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub teaching_obligations: BTreeMap<String, TeachingObligation>,
}

impl VersionedFamilyHousingState {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            schema_version: FAMILY_HOUSING_SCHEMA_VERSION,
            households: BTreeMap::new(),
            teaching_obligations: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), FamilyHousingError> {
        if self.schema_version != FAMILY_HOUSING_SCHEMA_VERSION {
            return Err(FamilyHousingError::UnsupportedVersion(self.schema_version));
        }
        for id in self
            .households
            .keys()
            .chain(self.teaching_obligations.keys())
        {
            if !is_stable_id(id) {
                return Err(FamilyHousingError::InvalidStableId(id.clone()));
            }
        }
        for (key, household) in &self.households {
            if key != &household.household_id {
                return Err(FamilyHousingError::IdentityMismatch {
                    map_key: key.clone(),
                    embedded_id: household.household_id.clone(),
                });
            }
            for id in household
                .adult_cat_ids
                .iter()
                .chain(&household.dependent_cat_ids)
                .chain(&household.elder_cat_ids)
            {
                if !is_stable_id(id) {
                    return Err(FamilyHousingError::InvalidStableId(id.clone()));
                }
            }
            let has_duplicate_resident = household
                .adult_cat_ids
                .iter()
                .chain(&household.dependent_cat_ids)
                .chain(&household.elder_cat_ids)
                .collect::<BTreeSet<_>>()
                .len()
                != household.adult_cat_ids.len()
                    + household.dependent_cat_ids.len()
                    + household.elder_cat_ids.len();
            if has_duplicate_resident {
                return Err(FamilyHousingError::DuplicateHouseholdResident(key.clone()));
            }
        }
        for (key, obligation) in &self.teaching_obligations {
            if !is_stable_id(&obligation.parent_cat_id)
                || !is_stable_id(&obligation.dependent_cat_id)
            {
                let bad = if !is_stable_id(&obligation.parent_cat_id) {
                    &obligation.parent_cat_id
                } else {
                    &obligation.dependent_cat_id
                };
                return Err(FamilyHousingError::InvalidStableId(bad.clone()));
            }
            if obligation.completed_real_tasks_since_teach > TEACHING_OBLIGATION_REAL_TASKS
                || (obligation.deferred_by_emergency && !obligation.due)
            {
                return Err(FamilyHousingError::InvalidTeachingObligation(key.clone()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FamilyHousingError {
    UnsupportedVersion(u16),
    InvalidStableId(String),
    IdentityMismatch {
        map_key: String,
        embedded_id: String,
    },
    DuplicateHouseholdResident(String),
    InvalidTeachingObligation(String),
}

#[must_use]
pub fn keyed_pair_hash(first_id: &str, second_id: &str, colony_seed: u64, salt: &str) -> u64 {
    let pair_id = stable_pair_id(first_id, second_id);
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in pair_id
        .bytes()
        .chain([b'|'])
        .chain(colony_seed.to_le_bytes())
        .chain([b'|'])
        .chain(salt.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

#[must_use]
pub fn stable_pair_id(first_id: &str, second_id: &str) -> String {
    if first_id <= second_id {
        format!("{first_id}+{second_id}")
    } else {
        format!("{second_id}+{first_id}")
    }
}

#[must_use]
pub fn is_stable_id(value: &str) -> bool {
    crate::family_specialization::is_stable_id(value)
}
