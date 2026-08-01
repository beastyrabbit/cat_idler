//! Canonical, versioned family/lifecycle aggregate for LAI.56.
//!
//! This composes `family_specialization` and `family_housing`; it deliberately
//! owns neither inventory nor task execution.  A later lifecycle/world-tick
//! authority supplies real births, completions, sites, and deaths through the
//! idempotent commands below.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    family_housing::{
        CareActivityChoice, HouseholdProfile, HousingKind, LifeStage, PartnershipAuthority,
        PartnershipCandidate, TeachingObligation, TeachingSite, choose_care_activity,
        complete_teaching, defer_for_emergency, housing_capacity, housing_move_recommendation,
        partnership_score, record_parent_real_task, stable_pair_id, teaching_site_allowed,
    },
    family_specialization::{
        BirthSeedOutcome, EnterpriseGoodsOwnership, FamilyBranchRule, FamilyEnterprise,
        GenerationProfessionRecord, ParentProfessionSeed, birth_seed_grant, formal_teaching_xp,
        keyed_birth_seed_outcome, occupational_surname_key, tradition_maturity,
    },
    skill_catalog::{SkillProgress, floor_level_from_xp_centi},
};

pub const FAMILY_AUTHORITY_SCHEMA_VERSION: u16 = 1;
pub const FAMILY_AUTHORITY_MAX_CATS: usize = 4_096;
pub const FAMILY_AUTHORITY_MAX_BUILDINGS: usize = 512;
pub const FAMILY_AUTHORITY_MAX_RECEIPTS: usize = 512;
pub const FAMILY_AUTHORITY_MAX_COMPLETED_TASKS: usize = 16_384;
pub const FAMILY_AUTHORITY_MAX_RELATIONS_PER_CAT: usize = 256;

/// Stable, report-safe identity reference to the separate cat capability
/// authority. It proves which authoritative fields were inherited without
/// copying acquired traits or office clearance into family state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InheritedIdentityReference {
    pub attribute_authority_ref: String,
    pub relational_analytical_authority_ref: String,
    pub inherited_parent_ids: Vec<String>,
    pub acquired_trait_ids: Vec<String>,
    pub inherited_office_clearance: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyCat {
    pub cat_id: String,
    pub life_stage: LifeStage,
    pub alive: bool,
    pub parent_cat_ids: Vec<String>,
    pub generation_index: u16,
    pub lineage_ids: BTreeSet<String>,
    pub surname_key: Option<String>,
    pub tradition_ids: BTreeSet<String>,
    pub profession_skill_xp_centi: BTreeMap<String, u64>,
    pub successful_units_by_profession: BTreeMap<String, u32>,
    pub profession_enterprise_ids: BTreeMap<String, String>,
    pub identity_reference: InheritedIdentityReference,
    pub assigned_mentor_cat_id: Option<String>,
}

impl FamilyCat {
    fn parent_seed(&self) -> ParentProfessionSeed {
        ParentProfessionSeed::new(
            self.cat_id.clone(),
            self.lineage_ids
                .iter()
                .next()
                .cloned()
                .unwrap_or_else(|| format!("lineage_{}", self.cat_id)),
            self.tradition_ids
                .iter()
                .next()
                .cloned()
                .unwrap_or_else(|| "tradition_none".to_owned()),
            self.profession_skill_xp_centi.clone(),
        )
    }

    fn dependency(&self) -> bool {
        matches!(self.life_stage, LifeStage::Kitten | LifeStage::Young)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyBuilding {
    pub building_id: String,
    pub housing_kind: Option<HousingKind>,
    pub teaching_site: Option<TeachingSite>,
    pub completed: bool,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Partnership {
    pub partnership_id: String,
    pub first_cat_id: String,
    pub second_cat_id: String,
    pub formed_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyHousehold {
    pub household_id: String,
    pub adult_cat_ids: BTreeSet<String>,
    pub dependent_cat_ids: BTreeSet<String>,
    pub residence_building_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingAssignment {
    pub obligation_id: String,
    pub site_building_id: String,
    pub learner_skill_id: String,
    pub productive_minutes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyEnterpriseRecord {
    pub enterprise: FamilyEnterprise,
    pub branch: FamilyBranchRule,
}

/// A report-safe physical residence reference.  Family ownership ends at the
/// stable building ID: a spatial authority must resolve any footprint or
/// route, rather than this leaf inventing world geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilyResidenceReport<'a> {
    pub resident_cat_id: &'a str,
    pub building_id: &'a str,
    pub housing_kind: Option<HousingKind>,
}

/// Stable household membership plus its authored physical residence reference.
/// The resident sets borrow the authority's ordered storage; no parallel
/// household membership list is materialized for reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilyHouseholdPhysicalReport<'a> {
    pub household_id: &'a str,
    pub adult_cat_ids: &'a BTreeSet<String>,
    pub dependent_cat_ids: &'a BTreeSet<String>,
    pub residence_building_id: Option<&'a str>,
}

/// A deliberately small enterprise summary.  It confirms the visible family
/// identity and exact physical site without exposing a private inventory or
/// manufacturing an enterprise-owned goods balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilyEnterpriseReport<'a> {
    pub enterprise_id: &'a str,
    pub tradition_id: &'a str,
    pub profession_id: &'a str,
    pub site_building_id: &'a str,
    pub signage_key: &'a str,
    pub worker_preference: bool,
    pub mentoring_identity: bool,
    pub goods_ownership: EnterpriseGoodsOwnership,
}

/// An effective mentor is either the explicit stored assignment or the
/// persisted parent fallback that the teaching authority will use.  Missing
/// foreign/corrupt references remain unavailable instead of being guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyMentorReference<'a> {
    Assigned(&'a str),
    ParentFallback(&'a str),
    Unavailable,
}

/// One persisted teaching obligation in canonical obligation-ID order.  The
/// summary is sufficient to render a real teaching task/status while keeping
/// skill XP, task history, and hidden capability state in their owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilyMentorshipReport<'a> {
    pub obligation_id: &'a str,
    pub parent_cat_id: &'a str,
    pub dependent_cat_id: &'a str,
    pub mentor: FamilyMentorReference<'a>,
    pub completed_real_tasks_since_teach: u8,
    pub due: bool,
    pub deferred_by_emergency: bool,
}

/// Explicit mentor assignments with no teaching obligation are still useful
/// to the Village/Cats report.  The BTree cat order is canonical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamilyAssignedMentorReport<'a> {
    pub dependent_cat_id: &'a str,
    pub mentor_cat_id: &'a str,
}

/// A typed point-query result avoids treating an unassigned cat as a hidden
/// residence or inventing a physical building reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyResidenceReportAccess<'a> {
    Report(FamilyResidenceReport<'a>),
    Unavailable(FamilyResidenceUnavailable),
}

impl<'a> FamilyResidenceReportAccess<'a> {
    #[must_use]
    pub const fn into_report(self) -> Option<FamilyResidenceReport<'a>> {
        match self {
            Self::Report(report) => Some(report),
            Self::Unavailable(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyResidenceUnavailable {
    UnknownCat,
    NoAssignedResidence,
    PhysicalBuildingUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyAuthorityState {
    pub schema_version: u16,
    pub colony_id: String,
    pub colony_seed: u64,
    pub revision: u64,
    pub cats: BTreeMap<String, FamilyCat>,
    pub buildings: BTreeMap<String, FamilyBuilding>,
    pub partnerships: BTreeMap<String, Partnership>,
    pub households: BTreeMap<String, FamilyHousehold>,
    pub residences: BTreeMap<String, String>,
    pub teaching_obligations: BTreeMap<String, TeachingObligation>,
    pub enterprises: BTreeMap<String, FamilyEnterpriseRecord>,
    pub completed_task_ids: BTreeSet<String>,
    pub receipts: BTreeMap<String, FamilyCommandReceipt>,
}

impl FamilyAuthorityState {
    #[must_use]
    pub fn empty(colony_id: impl Into<String>, colony_seed: u64) -> Self {
        Self {
            schema_version: FAMILY_AUTHORITY_SCHEMA_VERSION,
            colony_id: colony_id.into(),
            colony_seed,
            revision: 0,
            cats: BTreeMap::new(),
            buildings: BTreeMap::new(),
            partnerships: BTreeMap::new(),
            households: BTreeMap::new(),
            residences: BTreeMap::new(),
            teaching_obligations: BTreeMap::new(),
            enterprises: BTreeMap::new(),
            completed_task_ids: BTreeSet::new(),
            receipts: BTreeMap::new(),
        }
    }

    /// Strict restart boundary: callers must validate a decoded state before it
    /// can become authoritative.
    pub fn decode_json(json: &str) -> Result<Self, FamilyAuthorityError> {
        let state: Self = serde_json::from_str(json)
            .map_err(|error| FamilyAuthorityError::InvalidSnapshot(error.to_string()))?;
        state.validate()?;
        Ok(state)
    }

    /// Returns residence references in stable resident-cat ID order.  These
    /// are references to real stored buildings only; the family authority has
    /// no footprint/location authority to leak or synthesize.
    #[must_use]
    pub fn report_residences(&self) -> impl ExactSizeIterator<Item = FamilyResidenceReport<'_>> {
        self.residences
            .iter()
            .map(|(cat_id, building_id)| FamilyResidenceReport {
                resident_cat_id: cat_id,
                building_id,
                housing_kind: self
                    .buildings
                    .get(building_id)
                    .and_then(|building| building.housing_kind),
            })
    }

    /// Reads one residence without converting a missing cat, an unassigned
    /// resident, or a missing physical building into an ambiguous `None`.
    #[must_use]
    pub fn report_residence_for(&self, cat_id: &str) -> FamilyResidenceReportAccess<'_> {
        if !self.cats.contains_key(cat_id) {
            return FamilyResidenceReportAccess::Unavailable(
                FamilyResidenceUnavailable::UnknownCat,
            );
        }
        let Some((resident_cat_id, building_id)) = self.residences.get_key_value(cat_id) else {
            return FamilyResidenceReportAccess::Unavailable(
                FamilyResidenceUnavailable::NoAssignedResidence,
            );
        };
        let Some(building) = self.buildings.get(building_id) else {
            return FamilyResidenceReportAccess::Unavailable(
                FamilyResidenceUnavailable::PhysicalBuildingUnavailable,
            );
        };
        FamilyResidenceReportAccess::Report(FamilyResidenceReport {
            resident_cat_id,
            building_id,
            housing_kind: building.housing_kind,
        })
    }

    /// Returns households in stable household-ID order with only their stored
    /// member IDs and residence reference.
    #[must_use]
    pub fn report_households(
        &self,
    ) -> impl ExactSizeIterator<Item = FamilyHouseholdPhysicalReport<'_>> {
        self.households
            .values()
            .map(|household| FamilyHouseholdPhysicalReport {
                household_id: &household.household_id,
                adult_cat_ids: &household.adult_cat_ids,
                dependent_cat_ids: &household.dependent_cat_ids,
                residence_building_id: household.residence_building_id.as_deref(),
            })
    }

    /// Returns mature enterprises in stable enterprise-ID order.  No colony
    /// goods, private balances, or inferred ownership are copied into the
    /// report; `goods_ownership` is the authority's stored enum.
    #[must_use]
    pub fn report_enterprises(&self) -> impl ExactSizeIterator<Item = FamilyEnterpriseReport<'_>> {
        self.enterprises
            .values()
            .map(|record| FamilyEnterpriseReport {
                enterprise_id: &record.enterprise.enterprise_id,
                tradition_id: &record.enterprise.tradition_id,
                profession_id: &record.enterprise.profession_id,
                site_building_id: &record.enterprise.site_id,
                signage_key: &record.enterprise.signage_key,
                worker_preference: record.enterprise.worker_preference,
                mentoring_identity: record.enterprise.mentoring_identity,
                goods_ownership: record.enterprise.goods_ownership,
            })
    }

    /// Returns persisted parent/mentor teaching state in stable obligation-ID
    /// order.  It never exposes the separate skill/capability authority.
    #[must_use]
    pub fn report_mentorships(&self) -> impl ExactSizeIterator<Item = FamilyMentorshipReport<'_>> {
        self.teaching_obligations
            .iter()
            .map(|(obligation_id, obligation)| {
                let mentor = self
                    .cats
                    .get(&obligation.dependent_cat_id)
                    .and_then(|cat| cat.assigned_mentor_cat_id.as_deref())
                    .map(FamilyMentorReference::Assigned)
                    .or_else(|| {
                        self.cats.contains_key(&obligation.parent_cat_id).then_some(
                            FamilyMentorReference::ParentFallback(
                                obligation.parent_cat_id.as_str(),
                            ),
                        )
                    })
                    .unwrap_or(FamilyMentorReference::Unavailable);
                FamilyMentorshipReport {
                    obligation_id,
                    parent_cat_id: &obligation.parent_cat_id,
                    dependent_cat_id: &obligation.dependent_cat_id,
                    mentor,
                    completed_real_tasks_since_teach: obligation.completed_real_tasks_since_teach,
                    due: obligation.due,
                    deferred_by_emergency: obligation.deferred_by_emergency,
                }
            })
    }

    /// Returns direct mentor assignments even before an after-three-task
    /// obligation exists, in stable dependent-cat ID order.
    #[must_use]
    pub fn report_assigned_mentors(&self) -> Vec<FamilyAssignedMentorReport<'_>> {
        self.cats
            .values()
            .filter_map(|cat| {
                cat.assigned_mentor_cat_id.as_deref().map(|mentor_cat_id| {
                    FamilyAssignedMentorReport {
                        dependent_cat_id: &cat.cat_id,
                        mentor_cat_id,
                    }
                })
            })
            .collect()
    }

    pub fn validate(&self) -> Result<(), FamilyAuthorityError> {
        if self.schema_version != FAMILY_AUTHORITY_SCHEMA_VERSION {
            return Err(FamilyAuthorityError::UnsupportedVersion(
                self.schema_version,
            ));
        }
        validate_id(&self.colony_id)?;
        if self.cats.len() > FAMILY_AUTHORITY_MAX_CATS
            || self.buildings.len() > FAMILY_AUTHORITY_MAX_BUILDINGS
            || self.receipts.len() > FAMILY_AUTHORITY_MAX_RECEIPTS
            || self.completed_task_ids.len() > FAMILY_AUTHORITY_MAX_COMPLETED_TASKS
        {
            return Err(FamilyAuthorityError::BoundExceeded);
        }
        for (id, cat) in &self.cats {
            validate_id(id)?;
            if id != &cat.cat_id || cat.parent_cat_ids.len() > 2 {
                return Err(FamilyAuthorityError::InvalidCat(id.clone()));
            }
            validate_ids(&cat.parent_cat_ids)?;
            validate_ids(&cat.identity_reference.inherited_parent_ids)?;
            if cat.identity_reference.inherited_parent_ids != cat.parent_cat_ids
                || !cat.identity_reference.acquired_trait_ids.is_empty()
                || cat.identity_reference.inherited_office_clearance
            {
                return Err(FamilyAuthorityError::InvalidInheritedIdentity(id.clone()));
            }
            validate_id(&cat.identity_reference.attribute_authority_ref)?;
            validate_id(&cat.identity_reference.relational_analytical_authority_ref)?;
            validate_set(&cat.lineage_ids)?;
            validate_set(&cat.tradition_ids)?;
            validate_map_ids(&cat.profession_skill_xp_centi)?;
            validate_map_ids(&cat.successful_units_by_profession)?;
            validate_map_ids(&cat.profession_enterprise_ids)?;
            if let Some(mentor) = &cat.assigned_mentor_cat_id {
                validate_id(mentor)?;
            }
        }
        for (id, building) in &self.buildings {
            validate_id(id)?;
            if id != &building.building_id || building.level == 0 {
                return Err(FamilyAuthorityError::InvalidBuilding(id.clone()));
            }
        }
        for (id, partnership) in &self.partnerships {
            validate_id(id)?;
            if id != &partnership.partnership_id
                || partnership.partnership_id
                    != partnership_id(&partnership.first_cat_id, &partnership.second_cat_id)
                || partnership.first_cat_id == partnership.second_cat_id
                || !self.cats.contains_key(&partnership.first_cat_id)
                || !self.cats.contains_key(&partnership.second_cat_id)
            {
                return Err(FamilyAuthorityError::InvalidPartnership(id.clone()));
            }
        }
        for (household_id, household) in &self.households {
            validate_id(household_id)?;
            if household_id != &household.household_id || household.adult_cat_ids.len() > 2 {
                return Err(FamilyAuthorityError::InvalidHousehold(household_id.clone()));
            }
            for cat_id in household
                .adult_cat_ids
                .iter()
                .chain(&household.dependent_cat_ids)
            {
                if !self.cats.contains_key(cat_id) {
                    return Err(FamilyAuthorityError::UnknownCat(cat_id.clone()));
                }
            }
            if let Some(building_id) = &household.residence_building_id {
                if self
                    .buildings
                    .get(building_id)
                    .is_none_or(|building| !building.completed)
                {
                    return Err(FamilyAuthorityError::UnknownBuilding(building_id.clone()));
                }
            }
        }
        let mut occupancy = BTreeMap::<String, (usize, usize, usize)>::new();
        for (cat_id, building_id) in &self.residences {
            let Some(cat) = self.cats.get(cat_id) else {
                return Err(FamilyAuthorityError::UnknownCat(cat_id.clone()));
            };
            let Some(building) = self.buildings.get(building_id) else {
                return Err(FamilyAuthorityError::UnknownBuilding(building_id.clone()));
            };
            if !cat.alive || !building.completed || building.housing_kind.is_none() {
                return Err(FamilyAuthorityError::InvalidResidence(cat_id.clone()));
            }
            let counts = occupancy.entry(building_id.clone()).or_default();
            match cat.life_stage {
                LifeStage::Kitten | LifeStage::Young => counts.1 += 1,
                LifeStage::Adult => counts.0 += 1,
                LifeStage::Elder => counts.2 += 1,
            }
        }
        for (building_id, (adults, dependents, elders)) in occupancy {
            let building = self
                .buildings
                .get(&building_id)
                .expect("residence building validated");
            let kind = building
                .housing_kind
                .expect("residence housing kind validated");
            let capacity = housing_capacity(kind);
            let valid = match kind {
                HousingKind::Den => {
                    adults + dependents + elders <= usize::from(capacity.flexible_beds)
                }
                HousingKind::FamilyHome => {
                    adults <= usize::from(capacity.partnered_adult_beds)
                        && dependents <= usize::from(capacity.dependent_beds)
                        && elders == 0
                }
                HousingKind::ElderLodge => {
                    adults == 0 && dependents == 0 && elders <= usize::from(capacity.elder_beds)
                }
                HousingKind::Nursery => {
                    adults + dependents + elders <= usize::from(capacity.permanent_beds)
                }
            };
            if !valid {
                return Err(FamilyAuthorityError::InvalidResidence(building_id));
            }
        }
        for (id, obligation) in &self.teaching_obligations {
            validate_id(id)?;
            if id != &obligation_id(&obligation.parent_cat_id, &obligation.dependent_cat_id)
                || obligation.completed_real_tasks_since_teach > 3
                || (obligation.deferred_by_emergency && !obligation.due)
            {
                return Err(FamilyAuthorityError::InvalidTeachingObligation(id.clone()));
            }
        }
        for (id, record) in &self.enterprises {
            validate_id(id)?;
            let enterprise = &record.enterprise;
            if id != &enterprise.enterprise_id
                || enterprise.goods_ownership != EnterpriseGoodsOwnership::ColonyOwned
                || !self.buildings.contains_key(&enterprise.site_id)
            {
                return Err(FamilyAuthorityError::InvalidEnterprise(id.clone()));
            }
        }
        for task_id in &self.completed_task_ids {
            validate_id(task_id)?;
        }
        for (receipt_id, receipt) in &self.receipts {
            validate_id(receipt_id)?;
            if receipt_id != &receipt.receipt_id || receipt.applied_revision > self.revision {
                return Err(FamilyAuthorityError::InvalidReceipt(receipt_id.clone()));
            }
        }
        Ok(())
    }

    pub fn apply(
        &mut self,
        command: FamilyCommand,
    ) -> Result<FamilyCommandReceipt, FamilyAuthorityError> {
        self.validate()?;
        validate_id(&command.receipt_id)?;
        let fingerprint = command_fingerprint(&command.operation)?;
        if let Some(receipt) = self.receipts.get(&command.receipt_id) {
            return if receipt.operation_fingerprint == fingerprint {
                Ok(receipt.clone())
            } else {
                Err(FamilyAuthorityError::ReceiptConflict(command.receipt_id))
            };
        }
        if command.expected_revision != self.revision {
            return Err(FamilyAuthorityError::VersionConflict {
                expected: command.expected_revision,
                actual: self.revision,
            });
        }
        // Every operation is applied to an isolated aggregate and committed
        // only after all postconditions validate.  Lifecycle callers may
        // therefore safely retry a rejected command without compensating
        // partially-mutated lineage, housing, or teaching state.
        let mut next = self.clone();
        let result = next.apply_operation(command.operation)?;
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or(FamilyAuthorityError::Overflow)?;
        let receipt = FamilyCommandReceipt {
            receipt_id: command.receipt_id.clone(),
            operation_fingerprint: fingerprint,
            applied_revision: next.revision,
            result,
        };
        next.receipts.insert(command.receipt_id, receipt.clone());
        while next.receipts.len() > FAMILY_AUTHORITY_MAX_RECEIPTS {
            let Some(oldest) = next.receipts.keys().next().cloned() else {
                break;
            };
            next.receipts.remove(&oldest);
        }
        next.validate()?;
        *self = next;
        Ok(receipt)
    }

    fn apply_operation(
        &mut self,
        operation: FamilyOperation,
    ) -> Result<FamilyCommandResult, FamilyAuthorityError> {
        match operation {
            FamilyOperation::RegisterBirth(birth) => self.register_birth(birth),
            FamilyOperation::RegisterBuilding(building) => {
                validate_id(&building.building_id)?;
                if building.level == 0 || self.buildings.contains_key(&building.building_id) {
                    return Err(FamilyAuthorityError::InvalidBuilding(building.building_id));
                }
                self.buildings
                    .insert(building.building_id.clone(), building.clone());
                Ok(FamilyCommandResult::BuildingRegistered {
                    building_id: building.building_id,
                })
            }
            FamilyOperation::ReconcileBuildings { mut buildings } => {
                if buildings.len() > FAMILY_AUTHORITY_MAX_BUILDINGS {
                    return Err(FamilyAuthorityError::BoundExceeded);
                }
                buildings.sort_by(|left, right| left.building_id.cmp(&right.building_id));
                let mut next = BTreeMap::new();
                for building in buildings {
                    validate_id(&building.building_id)?;
                    let building_id = building.building_id.clone();
                    if building.level == 0 || !building.completed {
                        return Err(FamilyAuthorityError::InvalidBuilding(building_id));
                    }
                    if next.insert(building_id.clone(), building).is_some() {
                        return Err(FamilyAuthorityError::InvalidBuilding(building_id));
                    }
                }
                self.buildings = next;
                self.residences
                    .retain(|_, building_id| self.buildings.contains_key(building_id));
                for household in self.households.values_mut() {
                    if household
                        .residence_building_id
                        .as_ref()
                        .is_some_and(|building_id| !self.buildings.contains_key(building_id))
                    {
                        household.residence_building_id = None;
                    }
                }
                self.enterprises
                    .retain(|_, record| self.buildings.contains_key(&record.enterprise.site_id));
                Ok(FamilyCommandResult::BuildingsReconciled {
                    completed_buildings: u16::try_from(self.buildings.len())
                        .map_err(|_| FamilyAuthorityError::BoundExceeded)?,
                })
            }
            FamilyOperation::ReconcileLifeStages { life_stages } => {
                if life_stages.len() > FAMILY_AUTHORITY_MAX_CATS {
                    return Err(FamilyAuthorityError::BoundExceeded);
                }
                let mut changed = 0_u16;
                for (cat_id, life_stage) in life_stages {
                    let cat = self
                        .cats
                        .get_mut(&cat_id)
                        .ok_or_else(|| FamilyAuthorityError::UnknownCat(cat_id.clone()))?;
                    if cat.life_stage != life_stage {
                        cat.life_stage = life_stage;
                        changed = changed
                            .checked_add(1)
                            .ok_or(FamilyAuthorityError::Overflow)?;
                    }
                }
                if changed > 0 {
                    // A kitten becoming Young or an Adult becoming Elder can
                    // invalidate the old capacity class. Release assignments;
                    // the immediately following canonical housing pass selects
                    // a valid physical residence.
                    self.residences.clear();
                    for household in self.households.values_mut() {
                        household.residence_building_id = None;
                    }
                }
                Ok(FamilyCommandResult::LifeStagesReconciled {
                    changed_cats: changed,
                })
            }
            FamilyOperation::ReviewAutonomousPartnerships => self.review_autonomous_partnerships(),
            FamilyOperation::ReconcileHousing {
                pressure_requires_den_return,
            } => {
                self.reconcile_housing(pressure_requires_den_return)?;
                Ok(FamilyCommandResult::HousingReconciled {
                    residents: self.residences.len() as u16,
                })
            }
            FamilyOperation::RecordProfessionalCompletion(completion) => {
                self.record_professional_completion(completion)
            }
            FamilyOperation::DeferTeachingForEmergency {
                parent_cat_id,
                dependent_cat_id,
            } => {
                let id = obligation_id(&parent_cat_id, &dependent_cat_id);
                let obligation = self
                    .teaching_obligations
                    .remove(&id)
                    .ok_or(FamilyAuthorityError::UnknownTeachingObligation(id.clone()))?;
                self.teaching_obligations
                    .insert(id.clone(), defer_for_emergency(obligation));
                Ok(FamilyCommandResult::TeachingDeferred { obligation_id: id })
            }
            FamilyOperation::ResumeDeferredTeaching {
                parent_cat_id,
                dependent_cat_id,
            } => {
                let id = obligation_id(&parent_cat_id, &dependent_cat_id);
                let obligation = self
                    .teaching_obligations
                    .get_mut(&id)
                    .ok_or(FamilyAuthorityError::UnknownTeachingObligation(id.clone()))?;
                if obligation.due {
                    obligation.deferred_by_emergency = false;
                }
                Ok(FamilyCommandResult::TeachingResumed { obligation_id: id })
            }
            FamilyOperation::CompleteTeaching(assignment) => self.complete_teaching(assignment),
            FamilyOperation::AssignMentor {
                dependent_cat_id,
                mentor_cat_id,
            } => {
                let mentor_is_alive = self.cats.get(&mentor_cat_id).is_some_and(|cat| cat.alive);
                let learner = self
                    .cats
                    .get_mut(&dependent_cat_id)
                    .ok_or(FamilyAuthorityError::UnknownCat(dependent_cat_id.clone()))?;
                if !learner.dependency() || !mentor_is_alive {
                    return Err(FamilyAuthorityError::InvalidMentorAssignment);
                }
                learner.assigned_mentor_cat_id = Some(mentor_cat_id);
                Ok(FamilyCommandResult::MentorAssigned { dependent_cat_id })
            }
            FamilyOperation::CreateMatureEnterprise(request) => self.create_enterprise(request),
            FamilyOperation::RecordDeath { cat_id } => self.record_death(&cat_id),
        }
    }

    fn register_birth(
        &mut self,
        birth: BirthRegistration,
    ) -> Result<FamilyCommandResult, FamilyAuthorityError> {
        validate_id(&birth.newborn_cat_id)?;
        validate_id(&birth.attribute_authority_ref)?;
        validate_id(&birth.relational_analytical_authority_ref)?;
        if self.cats.contains_key(&birth.newborn_cat_id)
            || self.cats.len() >= FAMILY_AUTHORITY_MAX_CATS
        {
            return Err(FamilyAuthorityError::InvalidCat(birth.newborn_cat_id));
        }
        let parent_ids = ordered_parents(birth.first_parent_id, birth.second_parent_id)?;
        let parents = parent_ids
            .iter()
            .map(|id| {
                self.cats
                    .get(id)
                    .ok_or_else(|| FamilyAuthorityError::UnknownCat(id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if parents.iter().any(|parent| !parent.alive) {
            return Err(FamilyAuthorityError::DeadParent);
        }
        let (outcome, grant) = if parents.len() == 2 {
            let outcome =
                keyed_birth_seed_outcome(&birth.newborn_cat_id, &parent_ids[0], &parent_ids[1]);
            (
                outcome,
                birth_seed_grant(
                    outcome,
                    &parents[0].parent_seed(),
                    &parents[1].parent_seed(),
                ),
            )
        } else if parents.len() == 1 {
            // A sole authoritative parent is a founder/surrogacy edge, not a
            // synthetic two-parent roll: it receives the existing 5% transfer.
            let seed = parents[0].parent_seed();
            (
                BirthSeedOutcome::FirstParent,
                birth_seed_grant(BirthSeedOutcome::FirstParent, &seed, &seed),
            )
        } else {
            let seed = ParentProfessionSeed::new(
                "founder",
                "lineage_founder",
                "tradition_none",
                BTreeMap::new(),
            );
            (
                BirthSeedOutcome::None,
                birth_seed_grant(BirthSeedOutcome::None, &seed, &seed),
            )
        };
        let generation_index = parents
            .iter()
            .map(|parent| parent.generation_index)
            .max()
            .unwrap_or(0)
            .saturating_add(if parents.is_empty() { 0 } else { 1 });
        let mut lineage_ids = BTreeSet::new();
        for parent in &parents {
            lineage_ids.extend(parent.lineage_ids.iter().cloned());
        }
        if lineage_ids.is_empty() {
            lineage_ids.insert(format!("lineage_{}", birth.newborn_cat_id));
        }
        let surname_key = parents
            .iter()
            .filter_map(|parent| parent.surname_key.clone())
            .min();
        let mut tradition_ids = BTreeSet::new();
        for parent in &parents {
            tradition_ids.extend(parent.tradition_ids.iter().cloned());
        }
        let cat = FamilyCat {
            cat_id: birth.newborn_cat_id.clone(),
            life_stage: birth.life_stage,
            alive: true,
            parent_cat_ids: parent_ids.clone(),
            generation_index,
            lineage_ids,
            surname_key,
            tradition_ids,
            profession_skill_xp_centi: grant.inherited_skill_xp_centi.clone(),
            successful_units_by_profession: BTreeMap::new(),
            profession_enterprise_ids: BTreeMap::new(),
            identity_reference: InheritedIdentityReference {
                attribute_authority_ref: birth.attribute_authority_ref,
                relational_analytical_authority_ref: birth.relational_analytical_authority_ref,
                inherited_parent_ids: parent_ids.clone(),
                acquired_trait_ids: Vec::new(),
                inherited_office_clearance: false,
            },
            assigned_mentor_cat_id: None,
        };
        self.cats.insert(birth.newborn_cat_id.clone(), cat);
        self.attach_dependent_to_parent_household(&birth.newborn_cat_id, &parent_ids);
        Ok(FamilyCommandResult::BirthRegistered {
            newborn_cat_id: birth.newborn_cat_id,
            outcome,
            inherited_skill_xp_centi: grant.inherited_skill_xp_centi,
        })
    }

    fn attach_dependent_to_parent_household(&mut self, child_id: &str, parent_ids: &[String]) {
        let household_id = self
            .households
            .iter()
            .find(|(_, household)| {
                parent_ids
                    .iter()
                    .all(|id| household.adult_cat_ids.contains(id))
            })
            .map(|(household_id, _)| household_id.clone());
        if let Some(household_id) = household_id {
            if let Some(household) = self.households.get_mut(&household_id) {
                household.dependent_cat_ids.insert(child_id.to_owned());
            }
        }
    }

    fn review_autonomous_partnerships(
        &mut self,
    ) -> Result<FamilyCommandResult, FamilyAuthorityError> {
        let candidates = self.partnership_candidates();
        let mut pairs = Vec::new();
        for (index, first) in candidates.iter().enumerate() {
            for second in candidates.iter().skip(index + 1) {
                if self.has_active_partnership(&first.cat_id)
                    || self.has_active_partnership(&second.cat_id)
                {
                    continue;
                }
                if let Some(score) = partnership_score(
                    first,
                    second,
                    self.colony_seed,
                    PartnershipAuthority::Autonomous,
                ) {
                    pairs.push((score, first.cat_id.clone(), second.cat_id.clone()));
                }
            }
        }
        pairs.sort_by(|left, right| left.0.cmp(&right.0));
        let mut formed = Vec::new();
        for (_, first, second) in pairs {
            if self.has_active_partnership(&first) || self.has_active_partnership(&second) {
                continue;
            }
            let id = partnership_id(&first, &second);
            let household_id = household_id(&first, &second);
            self.partnerships.insert(
                id.clone(),
                Partnership {
                    partnership_id: id.clone(),
                    first_cat_id: first.clone(),
                    second_cat_id: second.clone(),
                    formed_revision: self.revision.saturating_add(1),
                },
            );
            self.households.insert(
                household_id.clone(),
                FamilyHousehold {
                    household_id,
                    adult_cat_ids: [first, second].into_iter().collect(),
                    dependent_cat_ids: BTreeSet::new(),
                    residence_building_id: None,
                },
            );
            formed.push(id);
        }
        Ok(FamilyCommandResult::PartnershipsReviewed {
            formed_partnership_ids: formed,
        })
    }

    fn partnership_candidates(&self) -> Vec<PartnershipCandidate> {
        self.cats
            .values()
            .filter(|cat| {
                cat.alive
                    && cat.life_stage == LifeStage::Adult
                    && !self.has_active_partnership(&cat.cat_id)
            })
            .map(|cat| {
                let ancestors = self.close_ancestors(&cat.cat_id);
                let siblings = self.close_siblings(&cat.cat_id);
                PartnershipCandidate {
                    cat_id: cat.cat_id.clone(),
                    close_ancestor_or_descendant_ids: ancestors,
                    close_sibling_ids: siblings,
                    inherited_attribute_score: 10,
                    profession_skill_level: cat
                        .profession_skill_xp_centi
                        .values()
                        .copied()
                        .map(floor_level_from_xp_centi)
                        .max()
                        .unwrap_or(0),
                    personality_compatibility_basis_points: 10_000,
                    relational_analytical: 0,
                    tradition_ids: cat.tradition_ids.clone(),
                    housing_ready: self.has_completed_housing(),
                }
            })
            .collect()
    }

    fn close_ancestors(&self, cat_id: &str) -> BTreeSet<String> {
        let mut result = BTreeSet::new();
        let mut frontier = vec![cat_id.to_owned()];
        for _ in 0..2 {
            let current = frontier;
            frontier = Vec::new();
            for id in current {
                if let Some(cat) = self.cats.get(&id) {
                    for parent in &cat.parent_cat_ids {
                        if result.insert(parent.clone()) {
                            frontier.push(parent.clone());
                        }
                    }
                }
            }
        }
        for (other_id, other) in &self.cats {
            if other.parent_cat_ids.iter().any(|parent| parent == cat_id) {
                result.insert(other_id.clone());
            }
        }
        result
    }

    fn close_siblings(&self, cat_id: &str) -> BTreeSet<String> {
        let Some(cat) = self.cats.get(cat_id) else {
            return BTreeSet::new();
        };
        self.cats
            .values()
            .filter(|other| {
                other.cat_id != cat_id
                    && !cat.parent_cat_ids.is_empty()
                    && other
                        .parent_cat_ids
                        .iter()
                        .any(|parent| cat.parent_cat_ids.contains(parent))
            })
            .map(|other| other.cat_id.clone())
            .collect()
    }

    fn has_active_partnership(&self, cat_id: &str) -> bool {
        self.partnerships.values().any(|partnership| {
            partnership.first_cat_id == cat_id || partnership.second_cat_id == cat_id
        })
    }

    fn has_completed_housing(&self) -> bool {
        self.buildings
            .values()
            .any(|building| building.completed && building.housing_kind.is_some())
    }

    fn reconcile_housing(
        &mut self,
        pressure_requires_den_return: bool,
    ) -> Result<(), FamilyAuthorityError> {
        self.residences.clear();
        for household in self.households.values_mut() {
            household.residence_building_id = None;
        }
        let mut remaining = self.completed_housing_capacities();
        let mut households = self
            .households
            .values()
            .filter(|household| {
                household
                    .adult_cat_ids
                    .iter()
                    .any(|id| self.cats.get(id).is_some_and(|cat| cat.alive))
            })
            .cloned()
            .collect::<Vec<_>>();
        households.sort_by_key(|household| {
            let elders = household
                .adult_cat_ids
                .iter()
                .filter(|id| {
                    self.cats
                        .get(*id)
                        .is_some_and(|cat| cat.life_stage == LifeStage::Elder)
                })
                .cloned()
                .collect();
            let profile = HouseholdProfile {
                household_id: household.household_id.clone(),
                adult_cat_ids: household.adult_cat_ids.iter().cloned().collect(),
                dependent_cat_ids: household.dependent_cat_ids.iter().cloned().collect(),
                elder_cat_ids: elders,
                pregnant_or_parenting: !household.dependent_cat_ids.is_empty(),
                empty_nest: household.dependent_cat_ids.is_empty(),
                current_housing: HousingKind::Den,
            };
            housing_move_recommendation(
                &profile,
                remaining.values().any(|slot| {
                    slot.kind == HousingKind::FamilyHome
                        && slot.adults >= 2
                        && slot.dependents >= household.dependent_cat_ids.len()
                }),
                remaining
                    .values()
                    .any(|slot| slot.kind == HousingKind::ElderLodge && slot.elders > 0),
                pressure_requires_den_return,
            )
        });
        for household in households {
            let living_adults = household
                .adult_cat_ids
                .iter()
                .filter(|id| self.cats.get(*id).is_some_and(|cat| cat.alive))
                .cloned()
                .collect::<Vec<_>>();
            let living_dependents = household
                .dependent_cat_ids
                .iter()
                .filter(|id| {
                    self.cats
                        .get(*id)
                        .is_some_and(|cat| cat.alive && cat.dependency())
                })
                .cloned()
                .collect::<Vec<_>>();
            let elders = living_adults
                .iter()
                .filter(|id| {
                    self.cats
                        .get(*id)
                        .is_some_and(|cat| cat.life_stage == LifeStage::Elder)
                })
                .cloned()
                .collect::<Vec<_>>();
            for elder in elders {
                if let Some(building_id) = remaining
                    .iter_mut()
                    .find(|(_, slot)| slot.kind == HousingKind::ElderLodge && slot.elders > 0)
                    .map(|(id, slot)| {
                        slot.elders -= 1;
                        id.clone()
                    })
                {
                    self.residences.insert(elder, building_id);
                }
            }
            let non_lodge_adults = living_adults
                .into_iter()
                .filter(|id| !self.residences.contains_key(id))
                .collect::<Vec<_>>();
            // Parenting households always receive Home priority. Empty-nest
            // partners keep their Home unless actual colony bed pressure asks
            // them to release it back to flexible Den capacity.
            if non_lodge_adults.len() == 2
                && (!living_dependents.is_empty() || !pressure_requires_den_return)
            {
                if let Some(building_id) = remaining
                    .iter_mut()
                    .find(|(_, slot)| {
                        slot.kind == HousingKind::FamilyHome
                            && slot.adults >= 2
                            && slot.dependents >= living_dependents.len()
                    })
                    .map(|(id, slot)| {
                        slot.adults -= 2;
                        slot.dependents -= living_dependents.len();
                        id.clone()
                    })
                {
                    for cat_id in non_lodge_adults.iter().chain(living_dependents.iter()) {
                        self.residences.insert(cat_id.clone(), building_id.clone());
                    }
                    if let Some(record) = self.households.get_mut(&household.household_id) {
                        record.residence_building_id = Some(building_id);
                    }
                }
            }
        }
        // Elders do not need a partnership household to qualify for a real
        // lodge bed. Place every still-unhoused living elder before the
        // flexible Den fallback so single and widowed elders are not
        // accidentally displaced by household-only reconciliation.
        let unassigned_elders = self
            .cats
            .values()
            .filter(|cat| {
                cat.alive
                    && cat.life_stage == LifeStage::Elder
                    && !self.residences.contains_key(&cat.cat_id)
            })
            .map(|cat| cat.cat_id.clone())
            .collect::<Vec<_>>();
        for elder_id in unassigned_elders {
            if let Some(building_id) = remaining
                .iter_mut()
                .find(|(_, slot)| slot.kind == HousingKind::ElderLodge && slot.elders > 0)
                .map(|(id, slot)| {
                    slot.elders -= 1;
                    id.clone()
                })
            {
                self.residences.insert(elder_id, building_id);
            }
        }
        for cat in self.cats.values().filter(|cat| cat.alive) {
            if self.residences.contains_key(&cat.cat_id) {
                continue;
            }
            if let Some(building_id) = remaining
                .iter_mut()
                .find(|(_, slot)| slot.kind == HousingKind::Den && slot.flexible > 0)
                .map(|(id, slot)| {
                    slot.flexible -= 1;
                    id.clone()
                })
            {
                self.residences.insert(cat.cat_id.clone(), building_id);
            }
        }
        Ok(())
    }

    fn completed_housing_capacities(&self) -> BTreeMap<String, RemainingHousing> {
        self.buildings
            .values()
            .filter(|building| building.completed)
            .filter_map(|building| {
                building
                    .housing_kind
                    .map(|kind| (building.building_id.clone(), (kind, housing_capacity(kind))))
            })
            .map(|(id, (kind, capacity))| {
                (
                    id,
                    RemainingHousing {
                        kind,
                        flexible: usize::from(capacity.flexible_beds),
                        adults: usize::from(capacity.partnered_adult_beds),
                        dependents: usize::from(capacity.dependent_beds),
                        elders: usize::from(capacity.elder_beds),
                    },
                )
            })
            .collect()
    }

    fn record_professional_completion(
        &mut self,
        completion: ProfessionalCompletion,
    ) -> Result<FamilyCommandResult, FamilyAuthorityError> {
        validate_id(&completion.task_id)?;
        validate_id(&completion.cat_id)?;
        validate_id(&completion.profession_id)?;
        validate_id(&completion.skill_id)?;
        if completion.skill_xp_centi == 0 || self.completed_task_ids.contains(&completion.task_id) {
            return Err(FamilyAuthorityError::DuplicateOrEmptyTask(
                completion.task_id,
            ));
        }
        if self.completed_task_ids.len() >= FAMILY_AUTHORITY_MAX_COMPLETED_TASKS {
            return Err(FamilyAuthorityError::BoundExceeded);
        }
        if let Some(enterprise_id) = &completion.enterprise_id {
            if !self.enterprises.contains_key(enterprise_id) {
                return Err(FamilyAuthorityError::UnknownEnterprise(
                    enterprise_id.clone(),
                ));
            }
        }
        {
            let cat = self
                .cats
                .get_mut(&completion.cat_id)
                .ok_or(FamilyAuthorityError::UnknownCat(completion.cat_id.clone()))?;
            if !cat.alive {
                return Err(FamilyAuthorityError::DeadCat(completion.cat_id));
            }
            *cat.profession_skill_xp_centi
                .entry(completion.skill_id.clone())
                .or_default() = cat
                .profession_skill_xp_centi
                .get(&completion.skill_id)
                .copied()
                .unwrap_or(0)
                .saturating_add(completion.skill_xp_centi);
            *cat.successful_units_by_profession
                .entry(completion.profession_id.clone())
                .or_default() = cat
                .successful_units_by_profession
                .get(&completion.profession_id)
                .copied()
                .unwrap_or(0)
                .saturating_add(1);
            if let Some(enterprise_id) = completion.enterprise_id {
                cat.profession_enterprise_ids
                    .insert(completion.profession_id.clone(), enterprise_id);
            }
        }
        self.completed_task_ids.insert(completion.task_id);
        let dependent_ids = self
            .cats
            .values()
            .filter(|child| {
                child.alive
                    && child.dependency()
                    && child.parent_cat_ids.contains(&completion.cat_id)
            })
            .map(|child| child.cat_id.clone())
            .collect::<Vec<_>>();
        for dependent_id in dependent_ids {
            let id = obligation_id(&completion.cat_id, &dependent_id);
            let obligation = self.teaching_obligations.remove(&id).unwrap_or_else(|| {
                TeachingObligation::new(completion.cat_id.clone(), dependent_id)
            });
            self.teaching_obligations
                .insert(id, record_parent_real_task(obligation));
        }
        Ok(FamilyCommandResult::ProfessionalCompletionRecorded {
            cat_id: completion.cat_id,
        })
    }

    fn complete_teaching(
        &mut self,
        assignment: TeachingAssignment,
    ) -> Result<FamilyCommandResult, FamilyAuthorityError> {
        validate_id(&assignment.obligation_id)?;
        validate_id(&assignment.site_building_id)?;
        validate_id(&assignment.learner_skill_id)?;
        let obligation = self
            .teaching_obligations
            .remove(&assignment.obligation_id)
            .ok_or(FamilyAuthorityError::UnknownTeachingObligation(
                assignment.obligation_id.clone(),
            ))?;
        if !obligation.due || obligation.deferred_by_emergency || assignment.productive_minutes == 0
        {
            self.teaching_obligations
                .insert(assignment.obligation_id, obligation);
            return Err(FamilyAuthorityError::TeachingNotDue);
        }
        let building = self.buildings.get(&assignment.site_building_id).ok_or(
            FamilyAuthorityError::UnknownBuilding(assignment.site_building_id.clone()),
        )?;
        if !building.completed
            || building
                .teaching_site
                .is_none_or(|site| !teaching_site_allowed(site))
        {
            self.teaching_obligations
                .insert(assignment.obligation_id, obligation);
            return Err(FamilyAuthorityError::InvalidTeachingSite);
        }
        let mentor_id = self
            .cats
            .get(&obligation.dependent_cat_id)
            .and_then(|cat| cat.assigned_mentor_cat_id.clone())
            .unwrap_or_else(|| obligation.parent_cat_id.clone());
        if !self.cats.get(&mentor_id).is_some_and(|cat| cat.alive)
            || !self
                .cats
                .get(&obligation.dependent_cat_id)
                .is_some_and(|cat| cat.alive && cat.dependency())
        {
            self.teaching_obligations
                .insert(assignment.obligation_id, obligation);
            return Err(FamilyAuthorityError::InvalidMentorAssignment);
        }
        let teacher_progress = self
            .cats
            .get(&mentor_id)
            .and_then(|cat| {
                cat.profession_skill_xp_centi
                    .get(&assignment.learner_skill_id)
            })
            .copied()
            .unwrap_or(0);
        let grant = formal_teaching_xp(
            &assignment.learner_skill_id,
            SkillProgress::new(teacher_progress),
            assignment.productive_minutes,
        );
        let learner = self
            .cats
            .get_mut(&obligation.dependent_cat_id)
            .expect("validated learner");
        *learner
            .profession_skill_xp_centi
            .entry(assignment.learner_skill_id)
            .or_default() = learner
            .profession_skill_xp_centi
            .get(&assignment.learner_skill_id)
            .copied()
            .unwrap_or(0)
            .saturating_add(grant.learner_xp_centi);
        let teacher = self.cats.get_mut(&mentor_id).expect("validated mentor");
        for teacher_grant in grant.teacher_grants {
            *teacher
                .profession_skill_xp_centi
                .entry(teacher_grant.skill_id)
                .or_default() = teacher
                .profession_skill_xp_centi
                .get(&teacher_grant.skill_id)
                .copied()
                .unwrap_or(0)
                .saturating_add(teacher_grant.xp_centi);
        }
        self.teaching_obligations.insert(
            assignment.obligation_id.clone(),
            complete_teaching(obligation),
        );
        Ok(FamilyCommandResult::TeachingCompleted {
            obligation_id: assignment.obligation_id,
            mentor_cat_id: mentor_id,
        })
    }

    fn create_enterprise(
        &mut self,
        request: EnterpriseRequest,
    ) -> Result<FamilyCommandResult, FamilyAuthorityError> {
        validate_id(&request.enterprise_id)?;
        validate_id(&request.tradition_id)?;
        validate_id(&request.profession_id)?;
        validate_id(&request.site_id)?;
        if self.enterprises.contains_key(&request.enterprise_id)
            || !self
                .buildings
                .get(&request.site_id)
                .is_some_and(|building| building.completed)
        {
            return Err(FamilyAuthorityError::InvalidEnterprise(
                request.enterprise_id,
            ));
        }
        let maturity = self.tradition_maturity(&request.profession_id, request.station_profession);
        if !maturity.mature {
            return Err(FamilyAuthorityError::TraditionNotMature);
        }
        let surname_key = occupational_surname_key(&request.profession_id)
            .unwrap_or("surname.professional")
            .to_owned();
        let enterprise = FamilyEnterprise {
            enterprise_id: request.enterprise_id.clone(),
            tradition_id: request.tradition_id.clone(),
            profession_id: request.profession_id.clone(),
            site_id: request.site_id,
            signage_key: format!("enterprise.{}", request.profession_id),
            worker_preference: true,
            mentoring_identity: true,
            history_identity: true,
            ui_identity: true,
            goods_ownership: EnterpriseGoodsOwnership::ColonyOwned,
        };
        let branch = FamilyBranchRule {
            lineage_id: format!("branch_{}", request.enterprise_id),
            profession_id: request.profession_id,
            adult_surname_key: surname_key.clone(),
            child_surname_key: surname_key,
            follows_profession: true,
            ancestry_lineage_ids: maturity
                .older_cat_id
                .into_iter()
                .chain(maturity.younger_cat_id)
                .collect(),
        };
        self.enterprises.insert(
            request.enterprise_id.clone(),
            FamilyEnterpriseRecord { enterprise, branch },
        );
        Ok(FamilyCommandResult::EnterpriseCreated {
            enterprise_id: request.enterprise_id,
        })
    }

    fn tradition_maturity(
        &self,
        profession_id: &str,
        station_profession: bool,
    ) -> crate::family_specialization::TraditionMaturity {
        let records = self
            .cats
            .values()
            .map(|cat| GenerationProfessionRecord {
                cat_id: cat.cat_id.clone(),
                generation_index: cat.generation_index,
                parent_cat_ids: cat.parent_cat_ids.clone(),
                profession_id: profession_id.to_owned(),
                skill_level: cat
                    .profession_skill_xp_centi
                    .values()
                    .copied()
                    .map(floor_level_from_xp_centi)
                    .max()
                    .unwrap_or(0),
                successful_units: cat
                    .successful_units_by_profession
                    .get(profession_id)
                    .copied()
                    .unwrap_or(0),
                physical_enterprise_id: cat.profession_enterprise_ids.get(profession_id).cloned(),
            })
            .collect::<Vec<_>>();
        tradition_maturity(profession_id, &records, station_profession)
    }

    fn record_death(&mut self, cat_id: &str) -> Result<FamilyCommandResult, FamilyAuthorityError> {
        let cat = self
            .cats
            .get_mut(cat_id)
            .ok_or_else(|| FamilyAuthorityError::UnknownCat(cat_id.to_owned()))?;
        if !cat.alive {
            return Err(FamilyAuthorityError::DeadCat(cat_id.to_owned()));
        }
        cat.alive = false;
        cat.assigned_mentor_cat_id = None;
        self.residences.remove(cat_id);
        self.partnerships.retain(|_, partnership| {
            partnership.first_cat_id != cat_id && partnership.second_cat_id != cat_id
        });
        self.teaching_obligations.retain(|_, obligation| {
            obligation.parent_cat_id != cat_id && obligation.dependent_cat_id != cat_id
        });
        for household in self.households.values_mut() {
            household.adult_cat_ids.remove(cat_id);
            household.dependent_cat_ids.remove(cat_id);
            if household.residence_building_id.is_some()
                && !household
                    .adult_cat_ids
                    .iter()
                    .any(|id| self.cats.get(id).is_some_and(|cat| cat.alive))
            {
                household.residence_building_id = None;
            }
        }
        self.households.retain(|_, household| {
            !household.adult_cat_ids.is_empty() || !household.dependent_cat_ids.is_empty()
        });
        for other in self.cats.values_mut() {
            if other.assigned_mentor_cat_id.as_deref() == Some(cat_id) {
                other.assigned_mentor_cat_id = None;
            }
        }
        Ok(FamilyCommandResult::DeathRecorded {
            cat_id: cat_id.to_owned(),
        })
    }

    #[must_use]
    pub fn report(&self) -> FamilyReport {
        FamilyReport {
            revision: self.revision,
            cats: self
                .cats
                .values()
                .map(|cat| FamilyCatSummary {
                    cat_id: cat.cat_id.clone(),
                    alive: cat.alive,
                    life_stage: cat.life_stage,
                    parent_cat_ids: cat.parent_cat_ids.iter().cloned().collect(),
                    lineage_ids: cat.lineage_ids.iter().cloned().collect(),
                    surname_key: cat.surname_key.clone(),
                    partnered: self.has_active_partnership(&cat.cat_id),
                    partnership_id: self
                        .partnerships
                        .values()
                        .find(|partnership| {
                            partnership.first_cat_id == cat.cat_id
                                || partnership.second_cat_id == cat.cat_id
                        })
                        .map(|partnership| partnership.partnership_id.clone()),
                    residence_building_id: self.residences.get(&cat.cat_id).cloned(),
                    tradition_ids: cat.tradition_ids.iter().cloned().collect(),
                    assigned_mentor_cat_id: cat.assigned_mentor_cat_id.clone(),
                    enterprise_id: cat.profession_enterprise_ids.values().next().cloned(),
                })
                .collect(),
            households: self
                .households
                .values()
                .map(|household| FamilyHouseholdSummary {
                    household_id: household.household_id.clone(),
                    residents: household
                        .adult_cat_ids
                        .iter()
                        .chain(&household.dependent_cat_ids)
                        .cloned()
                        .collect(),
                    residence_building_id: household.residence_building_id.clone(),
                })
                .collect(),
            enterprises: self
                .enterprises
                .values()
                .map(|record| FamilyEnterpriseSummary {
                    enterprise_id: record.enterprise.enterprise_id.clone(),
                    profession_id: record.enterprise.profession_id.clone(),
                    site_id: record.enterprise.site_id.clone(),
                    signage_key: record.enterprise.signage_key.clone(),
                    colony_goods_owned: true,
                })
                .collect(),
        }
    }

    /// Assigned mentors are considered before an otherwise-idle dependent can
    /// fall through to ambient cleaning.  The caller still owns emergency
    /// scheduling and visible task creation.
    #[must_use]
    pub fn preferred_care_activity(
        &self,
        parent_cat_id: &str,
        dependent_cat_id: &str,
        site_building_id: &str,
    ) -> Option<CareActivityChoice> {
        let obligation = self
            .teaching_obligations
            .get(&obligation_id(parent_cat_id, dependent_cat_id))?;
        let site = self.buildings.get(site_building_id)?.teaching_site?;
        let mentor = self
            .cats
            .get(dependent_cat_id)?
            .assigned_mentor_cat_id
            .as_deref();
        Some(choose_care_activity(obligation, mentor, site))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RemainingHousing {
    kind: HousingKind,
    flexible: usize,
    adults: usize,
    dependents: usize,
    elders: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BirthRegistration {
    pub newborn_cat_id: String,
    pub life_stage: LifeStage,
    pub first_parent_id: Option<String>,
    pub second_parent_id: Option<String>,
    pub attribute_authority_ref: String,
    pub relational_analytical_authority_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfessionalCompletion {
    pub task_id: String,
    pub cat_id: String,
    pub profession_id: String,
    pub skill_id: String,
    pub skill_xp_centi: u64,
    pub enterprise_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnterpriseRequest {
    pub enterprise_id: String,
    pub tradition_id: String,
    pub profession_id: String,
    pub site_id: String,
    pub station_profession: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "kind",
    content = "data",
    deny_unknown_fields
)]
pub enum FamilyOperation {
    RegisterBirth(BirthRegistration),
    RegisterBuilding(FamilyBuilding),
    /// Replace the physical family-building projection with the exact current
    /// completed set. Removed or repurposed sites release residence and
    /// enterprise bindings before the next housing pass; no phantom building
    /// survives a demolition or load reconciliation.
    ReconcileBuildings {
        buildings: Vec<FamilyBuilding>,
    },
    /// Advance real cat lifecycle stages without manufacturing a second birth
    /// clock inside the family aggregate.
    ReconcileLifeStages {
        life_stages: BTreeMap<String, LifeStage>,
    },
    ReviewAutonomousPartnerships,
    ReconcileHousing {
        pressure_requires_den_return: bool,
    },
    RecordProfessionalCompletion(ProfessionalCompletion),
    DeferTeachingForEmergency {
        parent_cat_id: String,
        dependent_cat_id: String,
    },
    CompleteTeaching(TeachingAssignment),
    ResumeDeferredTeaching {
        parent_cat_id: String,
        dependent_cat_id: String,
    },
    AssignMentor {
        dependent_cat_id: String,
        mentor_cat_id: String,
    },
    CreateMatureEnterprise(EnterpriseRequest),
    RecordDeath {
        cat_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyCommand {
    pub receipt_id: String,
    pub expected_revision: u64,
    pub operation: FamilyOperation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "kind",
    content = "data",
    deny_unknown_fields
)]
pub enum FamilyCommandResult {
    BirthRegistered {
        newborn_cat_id: String,
        outcome: BirthSeedOutcome,
        inherited_skill_xp_centi: BTreeMap<String, u64>,
    },
    BuildingRegistered {
        building_id: String,
    },
    BuildingsReconciled {
        completed_buildings: u16,
    },
    LifeStagesReconciled {
        changed_cats: u16,
    },
    PartnershipsReviewed {
        formed_partnership_ids: Vec<String>,
    },
    HousingReconciled {
        residents: u16,
    },
    ProfessionalCompletionRecorded {
        cat_id: String,
    },
    TeachingDeferred {
        obligation_id: String,
    },
    TeachingCompleted {
        obligation_id: String,
        mentor_cat_id: String,
    },
    TeachingResumed {
        obligation_id: String,
    },
    MentorAssigned {
        dependent_cat_id: String,
    },
    EnterpriseCreated {
        enterprise_id: String,
    },
    DeathRecorded {
        cat_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyCommandReceipt {
    pub receipt_id: String,
    pub operation_fingerprint: String,
    pub applied_revision: u64,
    pub result: FamilyCommandResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyReport {
    pub revision: u64,
    pub cats: Vec<FamilyCatSummary>,
    pub households: Vec<FamilyHouseholdSummary>,
    pub enterprises: Vec<FamilyEnterpriseSummary>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyCatSummary {
    pub cat_id: String,
    pub alive: bool,
    pub life_stage: LifeStage,
    pub parent_cat_ids: Vec<String>,
    pub lineage_ids: Vec<String>,
    pub surname_key: Option<String>,
    pub partnered: bool,
    pub partnership_id: Option<String>,
    pub residence_building_id: Option<String>,
    pub tradition_ids: Vec<String>,
    pub assigned_mentor_cat_id: Option<String>,
    pub enterprise_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyHouseholdSummary {
    pub household_id: String,
    pub residents: Vec<String>,
    pub residence_building_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyEnterpriseSummary {
    pub enterprise_id: String,
    pub profession_id: String,
    pub site_id: String,
    pub signage_key: String,
    pub colony_goods_owned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FamilyAuthorityError {
    UnsupportedVersion(u16),
    InvalidSnapshot(String),
    InvalidStableId(String),
    BoundExceeded,
    InvalidCat(String),
    InvalidBuilding(String),
    InvalidPartnership(String),
    InvalidHousehold(String),
    InvalidResidence(String),
    InvalidTeachingObligation(String),
    InvalidEnterprise(String),
    InvalidReceipt(String),
    InvalidInheritedIdentity(String),
    UnknownCat(String),
    UnknownBuilding(String),
    UnknownEnterprise(String),
    UnknownTeachingObligation(String),
    VersionConflict { expected: u64, actual: u64 },
    ReceiptConflict(String),
    DeadParent,
    DeadCat(String),
    DuplicateOrEmptyTask(String),
    TeachingNotDue,
    InvalidTeachingSite,
    InvalidMentorAssignment,
    TraditionNotMature,
    Overflow,
}
impl std::fmt::Display for FamilyAuthorityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "family authority error: {self:?}")
    }
}
impl std::error::Error for FamilyAuthorityError {}

fn ordered_parents(
    first: Option<String>,
    second: Option<String>,
) -> Result<Vec<String>, FamilyAuthorityError> {
    let parents = [first, second].into_iter().flatten().collect::<Vec<_>>();
    validate_ids(&parents)?;
    if parents.len() == 2 && parents[0] == parents[1] {
        return Err(FamilyAuthorityError::InvalidCat(
            "duplicate_parent".to_owned(),
        ));
    }
    Ok(parents)
}
fn partnership_id(first: &str, second: &str) -> String {
    derived_id("partnership", &[&stable_pair_id(first, second)])
}
fn household_id(first: &str, second: &str) -> String {
    derived_id("household", &[&stable_pair_id(first, second)])
}
fn obligation_id(parent: &str, dependent: &str) -> String {
    derived_id("obligation", &[parent, dependent])
}
fn derived_id(prefix: &str, parts: &[&str]) -> String {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for part in parts {
        for byte in part.bytes().chain([b'|']) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
    }
    format!("{prefix}_{hash:016x}")
}
fn command_fingerprint(operation: &FamilyOperation) -> Result<String, FamilyAuthorityError> {
    let encoded = serde_json::to_vec(operation)
        .map_err(|error| FamilyAuthorityError::InvalidSnapshot(error.to_string()))?;
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in encoded {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    Ok(format!("op_{hash:016x}"))
}
fn validate_id(value: &str) -> Result<(), FamilyAuthorityError> {
    if crate::family_specialization::is_stable_id(value) {
        Ok(())
    } else {
        Err(FamilyAuthorityError::InvalidStableId(value.to_owned()))
    }
}
fn validate_ids(values: &[String]) -> Result<(), FamilyAuthorityError> {
    if values.len() > FAMILY_AUTHORITY_MAX_RELATIONS_PER_CAT {
        return Err(FamilyAuthorityError::BoundExceeded);
    }
    for value in values {
        validate_id(value)?;
    }
    Ok(())
}
fn validate_set(values: &BTreeSet<String>) -> Result<(), FamilyAuthorityError> {
    if values.len() > FAMILY_AUTHORITY_MAX_RELATIONS_PER_CAT {
        return Err(FamilyAuthorityError::BoundExceeded);
    }
    for value in values {
        validate_id(value)?;
    }
    Ok(())
}
fn validate_map_ids<T>(values: &BTreeMap<String, T>) -> Result<(), FamilyAuthorityError> {
    if values.len() > FAMILY_AUTHORITY_MAX_RELATIONS_PER_CAT {
        return Err(FamilyAuthorityError::BoundExceeded);
    }
    for value in values.keys() {
        validate_id(value)?;
    }
    Ok(())
}
