//! Versioned per-cat capability authority for LAI.55.
//!
//! This leaf owns only capability facts keyed by real cat IDs.  Anatomy,
//! prosthetic inventory, rooms, and tools remain authoritative in their
//! existing systems and are passed in when eligibility or expertise is read.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    acquired_traits::AcquiredTraitState,
    anatomy::{BodyPart, CatAnatomy},
    cat_capabilities::{
        AssignmentTier, BODY_FUNCTION_MINIMUM_BASIS_POINTS, BodyRequirementKind,
        CapabilityAttributes, InheritedAttribute, LaborAffinity, LaborAffinityProfile,
        affinity_rank, body_requirement_for_skill,
    },
    officer_expertise::{ExpertiseBonuses, ExpertiseLevel, effective_level, personal_level},
    prosthetics::ProstheticLedger,
    skill_catalog::{
        self, ActivityXpDeclaration, AmbientSkillCandidate, OfficeKind, SkillProgress,
        SkillXpGrant, XpGrantSource,
    },
};

pub const CAT_CAPABILITY_AUTHORITY_SCHEMA_VERSION: u32 = 1;
pub const MAX_CAPABILITY_CATS: usize = 4_096;
pub const MAX_CAPABILITY_RECEIPTS: usize = 2_048;
pub const MAX_STABLE_CAT_ID_BYTES: usize = 128;
pub const MAX_RECEIPT_ID_BYTES: usize = 160;
pub const MAX_GRANTS_PER_CAPABILITY_RECEIPT: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatCapabilityRegistration {
    pub cat_id: String,
    pub attributes: CapabilityAttributes,
    pub labor: LaborAffinityProfile,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub skills: BTreeMap<String, SkillProgress>,
    /// Actual completed office-duty time. This is intentionally separate from
    /// learned XP: only this counter can raise report clearance.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub office_duty_minutes: BTreeMap<OfficeKind, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkActivity {
    pub primary_skill_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary_skill_ids: Vec<String>,
    #[serde(default)]
    pub haul_legs: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProductiveOutcome {
    /// A completed activity gets the catalog's full productive grant.
    Productive {
        productive_minutes: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        activity: Option<WorkActivity>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        office: Option<OfficeKind>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        supervised_by: Option<OfficeKind>,
    },
    /// Failed work retains its declared context for diagnostics but earns no
    /// skill XP and no completed office-duty time or clearance.
    FailedProductive {
        productive_minutes: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        activity: Option<WorkActivity>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        office: Option<OfficeKind>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        supervised_by: Option<OfficeKind>,
    },
    /// Transit-only hauling earns the catalog's small fixed gain per completed
    /// physical leg rather than time-based productive XP.
    Hauling {
        haul_legs: u16,
    },
    /// These outcomes deliberately have no learning side effect.
    Refused,
    Unassigned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductiveOutcomeReceipt {
    pub receipt_id: String,
    pub cat_id: String,
    pub outcome: ProductiveOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AptitudeReceipt {
    pub receipt_id: String,
    pub cat_id: String,
    pub interval_index: u64,
    /// Canonically ordered catalog skills compatible with the cat's current
    /// trait/personality context. Refusal is checked again by this authority.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub compatible_skill_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignmentEligibilityInput<'a> {
    pub cat_id: &'a str,
    pub skill_id: &'a str,
    pub tier: AssignmentTier,
    pub attribute: InheritedAttribute,
    pub continuity_minutes: u32,
    pub route_cost: u32,
    pub self_preservation: bool,
    pub anatomy: AnatomyCapabilityContext<'a>,
}

/// Lexicographically ascending: the first key is the exact preferred worker.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CapabilityAssignmentCandidateKey {
    pub urgency_rank: u8,
    pub affinity_rank: u8,
    pub skill_rank: u16,
    pub attribute_rank: u16,
    pub continuity_rank: u32,
    pub route_rank: u32,
    pub stable_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnatomyCapabilityContext<'a> {
    pub anatomy: &'a CatAnatomy,
    pub prosthetics: &'a ProstheticLedger,
    pub acquired_traits: &'a AcquiredTraitState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityEligibilityBlock {
    Paw(BodyPart),
    Eye(BodyPart),
    Tail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillCapabilityReport {
    pub skill_id: String,
    pub progress: SkillProgress,
    pub level: u16,
    pub mastery_xp_centi: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfficeCapabilityReport {
    pub office: OfficeKind,
    pub proficiency: SkillProgress,
    pub completed_duty_minutes: u64,
    pub personal_level: ExpertiseLevel,
}

/// Safe projection: it never carries receipt identities, executor outcomes,
/// or external anatomy/prosthetic state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatCapabilityReport {
    pub cat_id: String,
    pub attributes: CapabilityAttributes,
    pub labor: LaborAffinityProfile,
    pub skills: Vec<SkillCapabilityReport>,
    pub offices: Vec<OfficeCapabilityReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatCapabilityAuthorityReport {
    pub schema_version: u32,
    pub cats: Vec<CatCapabilityReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatCapabilityRecord {
    cat_id: String,
    attributes: CapabilityAttributes,
    labor: LaborAffinityProfile,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    skills: BTreeMap<String, SkillProgress>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    office_duty_minutes: BTreeMap<OfficeKind, u64>,
}

impl From<CatCapabilityRegistration> for CatCapabilityRecord {
    fn from(value: CatCapabilityRegistration) -> Self {
        Self {
            cat_id: value.cat_id,
            attributes: value.attributes,
            labor: value.labor,
            skills: value.skills,
            office_duty_minutes: value.office_duty_minutes,
        }
    }
}

impl CatCapabilityRecord {
    fn report(&self) -> CatCapabilityReport {
        let skills = self
            .skills
            .iter()
            .map(|(skill_id, progress)| SkillCapabilityReport {
                skill_id: skill_id.clone(),
                progress: *progress,
                level: progress.level(),
                mastery_xp_centi: progress.mastery_xp_centi(),
            })
            .collect();
        let offices = OfficeKind::ALL
            .into_iter()
            .map(|office| {
                let proficiency = self
                    .skills
                    .get(skill_catalog::office_proficiency_skill_id(office))
                    .copied()
                    .unwrap_or_else(|| SkillProgress::new(0));
                let completed_duty_minutes =
                    self.office_duty_minutes.get(&office).copied().unwrap_or(0);
                OfficeCapabilityReport {
                    office,
                    proficiency,
                    completed_duty_minutes,
                    personal_level: personal_level(completed_duty_minutes),
                }
            })
            .collect();
        CatCapabilityReport {
            cat_id: self.cat_id.clone(),
            attributes: self.attributes,
            labor: self.labor.clone(),
            skills,
            offices,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ReceiptFingerprint {
    Aptitude {
        cat_id: String,
        interval_index: u64,
        compatible_skill_ids: Vec<String>,
    },
    Outcome {
        cat_id: String,
        outcome: ProductiveOutcome,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AppliedReceipt {
    receipt_id: String,
    fingerprint: ReceiptFingerprint,
    grants: Vec<SkillXpGrant>,
}

/// Executor state. It is serialized as vectors so load-time validation can
/// reject duplicate IDs rather than silently accepting a map's last value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CatCapabilityAuthority {
    cats: BTreeMap<String, CatCapabilityRecord>,
    receipts: BTreeMap<String, AppliedReceipt>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthorityRef<'a> {
    schema_version: u32,
    cats: Vec<&'a CatCapabilityRecord>,
    receipts: Vec<&'a AppliedReceipt>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AuthorityOwned {
    schema_version: u32,
    cats: Vec<CatCapabilityRecord>,
    #[serde(default)]
    receipts: Vec<AppliedReceipt>,
}

impl Serialize for CatCapabilityAuthority {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        AuthorityRef {
            schema_version: CAT_CAPABILITY_AUTHORITY_SCHEMA_VERSION,
            cats: self.cats.values().collect(),
            receipts: self.receipts.values().collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CatCapabilityAuthority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AuthorityOwned::deserialize(deserializer)?;
        if wire.schema_version != CAT_CAPABILITY_AUTHORITY_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(
                "unsupported cat capability authority schema version",
            ));
        }
        let mut cats = BTreeMap::new();
        for record in wire.cats {
            let id = record.cat_id.clone();
            if cats.insert(id, record).is_some() {
                return Err(serde::de::Error::custom("duplicate capability cat id"));
            }
        }
        let mut receipts = BTreeMap::new();
        for receipt in wire.receipts {
            let id = receipt.receipt_id.clone();
            if receipts.insert(id, receipt).is_some() {
                return Err(serde::de::Error::custom("duplicate capability receipt id"));
            }
        }
        let authority = Self { cats, receipts };
        authority.validate().map_err(serde::de::Error::custom)?;
        Ok(authority)
    }
}

impl CatCapabilityAuthority {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cats: BTreeMap::new(),
            receipts: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn report(&self) -> CatCapabilityAuthorityReport {
        CatCapabilityAuthorityReport {
            schema_version: CAT_CAPABILITY_AUTHORITY_SCHEMA_VERSION,
            cats: self
                .cats
                .values()
                .map(CatCapabilityRecord::report)
                .collect(),
        }
    }

    #[must_use]
    pub fn cat_report(&self, cat_id: &str) -> Option<CatCapabilityReport> {
        self.cats.get(cat_id).map(CatCapabilityRecord::report)
    }

    pub fn register_cat(
        &mut self,
        registration: CatCapabilityRegistration,
    ) -> Result<(), CatCapabilityAuthorityError> {
        let record = CatCapabilityRecord::from(registration);
        validate_record(&record)?;
        if self.cats.len() >= MAX_CAPABILITY_CATS {
            return Err(CatCapabilityAuthorityError::CatCapacityExceeded);
        }
        if self.cats.contains_key(&record.cat_id) {
            return Err(CatCapabilityAuthorityError::DuplicateCatId);
        }
        self.cats.insert(record.cat_id.clone(), record);
        Ok(())
    }

    /// Replaces one existing cat's complete capability record after all fields
    /// validate. The stable cat ID itself cannot be changed.
    pub fn update_cat(
        &mut self,
        cat_id: &str,
        replacement: CatCapabilityRegistration,
    ) -> Result<(), CatCapabilityAuthorityError> {
        if !self.cats.contains_key(cat_id) {
            return Err(CatCapabilityAuthorityError::UnknownCatId);
        }
        let record = CatCapabilityRecord::from(replacement);
        validate_record(&record)?;
        if record.cat_id != cat_id {
            return Err(CatCapabilityAuthorityError::CatIdImmutable);
        }
        self.cats.insert(record.cat_id.clone(), record);
        Ok(())
    }

    /// Removal also discards that cat's receipts atomically. A removed real cat
    /// has no executor history to replay into this authority.
    pub fn remove_cat(
        &mut self,
        cat_id: &str,
    ) -> Result<CatCapabilityReport, CatCapabilityAuthorityError> {
        let Some(record) = self.cats.remove(cat_id) else {
            return Err(CatCapabilityAuthorityError::UnknownCatId);
        };
        self.receipts
            .retain(|_, receipt| receipt_cat_id(&receipt.fingerprint) != cat_id);
        Ok(record.report())
    }

    pub fn apply_aptitude_receipt(
        &mut self,
        receipt: AptitudeReceipt,
    ) -> Result<Vec<SkillXpGrant>, CatCapabilityAuthorityError> {
        validate_receipt_id(&receipt.receipt_id)?;
        validate_cat_id(&receipt.cat_id)?;
        validate_skill_id_sequence(&receipt.compatible_skill_ids)?;
        let fingerprint = ReceiptFingerprint::Aptitude {
            cat_id: receipt.cat_id.clone(),
            interval_index: receipt.interval_index,
            compatible_skill_ids: receipt.compatible_skill_ids.clone(),
        };
        if let Some(applied) = self.receipts.get(&receipt.receipt_id) {
            return replay_or_conflict(applied, &fingerprint);
        }
        let record = self
            .cats
            .get(&receipt.cat_id)
            .ok_or(CatCapabilityAuthorityError::UnknownCatId)?;
        let candidates = receipt
            .compatible_skill_ids
            .iter()
            .map(|skill_id| AmbientSkillCandidate {
                skill_id,
                compatible: true,
                refused: record.labor.affinity_for(skill_id) == LaborAffinity::Refused,
            })
            .collect::<Vec<_>>();
        let grants = skill_catalog::ambient_cleaning_xp_grants(
            &receipt.cat_id,
            receipt.interval_index,
            &candidates,
        );
        self.commit_grants(receipt.receipt_id, fingerprint, grants, None)
    }

    pub fn apply_productive_outcome_receipt(
        &mut self,
        receipt: ProductiveOutcomeReceipt,
    ) -> Result<Vec<SkillXpGrant>, CatCapabilityAuthorityError> {
        validate_receipt_id(&receipt.receipt_id)?;
        validate_cat_id(&receipt.cat_id)?;
        validate_outcome(&receipt.outcome)?;
        let fingerprint = ReceiptFingerprint::Outcome {
            cat_id: receipt.cat_id.clone(),
            outcome: receipt.outcome.clone(),
        };
        if let Some(applied) = self.receipts.get(&receipt.receipt_id) {
            return replay_or_conflict(applied, &fingerprint);
        }
        if !self.cats.contains_key(&receipt.cat_id) {
            return Err(CatCapabilityAuthorityError::UnknownCatId);
        }
        let (grants, completed_office_duty) = outcome_grants(&receipt.outcome)?;
        self.commit_grants(
            receipt.receipt_id,
            fingerprint,
            grants,
            completed_office_duty,
        )
    }

    /// Eligibility reads the one authoritative anatomy and prosthetic ledger;
    /// no `EffectiveAnatomy` copy is constructed or persisted here.
    pub fn skill_eligibility(
        &self,
        cat_id: &str,
        skill_id: &str,
        context: AnatomyCapabilityContext<'_>,
    ) -> Result<(), CatCapabilityAuthorityError> {
        if !self.cats.contains_key(cat_id) {
            return Err(CatCapabilityAuthorityError::UnknownCatId);
        }
        if skill_catalog::skill_definition(skill_id).is_none() {
            return Err(CatCapabilityAuthorityError::UnknownSkillId);
        }
        let part_function = |part| {
            context.prosthetics.effective_part_function_basis_points(
                context.anatomy,
                cat_id,
                part,
                context.acquired_traits,
            )
        };
        match body_requirement_for_skill(skill_id) {
            BodyRequirementKind::None => Ok(()),
            BodyRequirementKind::Movement => require_movement(&part_function),
            BodyRequirementKind::PhysicalLabor => {
                require_movement(&part_function).and_then(|()| require_paws(&part_function))
            }
            BodyRequirementKind::Vision => {
                require_movement(&part_function).and_then(|()| require_eyes(&part_function))
            }
            BodyRequirementKind::Combat => require_movement(&part_function)
                .and_then(|()| require_paws(&part_function))
                .and_then(|()| require_tail(&part_function)),
            BodyRequirementKind::RangedCombat => require_movement(&part_function)
                .and_then(|()| require_eyes(&part_function))
                .and_then(|()| require_tail(&part_function)),
        }
        .map_err(CatCapabilityAuthorityError::IneligibleAnatomy)
    }

    /// Computes the authoritative urgency-first worker ordering. Refusal,
    /// self-preservation, invalid priorities, and anatomy exclusion return no
    /// candidate; callers must never treat that as permission to force work.
    pub fn assignment_candidate_key(
        &self,
        input: AssignmentEligibilityInput<'_>,
    ) -> Result<Option<CapabilityAssignmentCandidateKey>, CatCapabilityAuthorityError> {
        let record = self
            .cats
            .get(input.cat_id)
            .ok_or(CatCapabilityAuthorityError::UnknownCatId)?;
        if !input.tier.is_valid()
            || input.self_preservation
            || record.labor.affinity_for(input.skill_id) == LaborAffinity::Refused
        {
            return Ok(None);
        }
        match self.skill_eligibility(input.cat_id, input.skill_id, input.anatomy) {
            Ok(()) => {}
            Err(CatCapabilityAuthorityError::IneligibleAnatomy(_)) => return Ok(None),
            Err(error) => return Err(error),
        }
        let skill_level = record
            .skills
            .get(input.skill_id)
            .copied()
            .unwrap_or_else(|| SkillProgress::new(0))
            .level();
        Ok(Some(CapabilityAssignmentCandidateKey {
            urgency_rank: input.tier.rank(),
            affinity_rank: affinity_rank(
                record.labor.is_family_enterprise(input.skill_id),
                record.labor.affinity_for(input.skill_id),
            ),
            skill_rank: u16::MAX - skill_level.min(u16::MAX - 1),
            attribute_rank: u16::MAX
                - u16::from(record.attributes.get(input.attribute)).min(u16::MAX - 1),
            continuity_rank: u32::MAX - input.continuity_minutes.min(u32::MAX - 1),
            route_rank: input.route_cost,
            stable_id: input.cat_id.to_owned(),
        }))
    }

    #[must_use]
    pub fn office_effective_level(
        &self,
        cat_id: &str,
        office: OfficeKind,
        context: ExpertiseBonuses,
    ) -> Result<ExpertiseLevel, CatCapabilityAuthorityError> {
        let record = self
            .cats
            .get(cat_id)
            .ok_or(CatCapabilityAuthorityError::UnknownCatId)?;
        let duty_minutes = record
            .office_duty_minutes
            .get(&office)
            .copied()
            .unwrap_or(0);
        Ok(effective_level(personal_level(duty_minutes), context))
    }

    pub fn validate(&self) -> Result<(), CatCapabilityAuthorityError> {
        if self.cats.len() > MAX_CAPABILITY_CATS {
            return Err(CatCapabilityAuthorityError::CatCapacityExceeded);
        }
        if self.receipts.len() > MAX_CAPABILITY_RECEIPTS {
            return Err(CatCapabilityAuthorityError::ReceiptCapacityExceeded);
        }
        for (cat_id, record) in &self.cats {
            if cat_id != &record.cat_id {
                return Err(CatCapabilityAuthorityError::CatIdMismatch);
            }
            validate_record(record)?;
        }
        for (receipt_id, receipt) in &self.receipts {
            if receipt_id != &receipt.receipt_id {
                return Err(CatCapabilityAuthorityError::ReceiptIdMismatch);
            }
            validate_receipt_id(receipt_id)?;
            validate_fingerprint(&receipt.fingerprint)?;
            if !self.cats.contains_key(receipt_cat_id(&receipt.fingerprint)) {
                return Err(CatCapabilityAuthorityError::ReceiptReferencesUnknownCat);
            }
            validate_grants(&receipt.grants)?;
        }
        Ok(())
    }

    fn commit_grants(
        &mut self,
        receipt_id: String,
        fingerprint: ReceiptFingerprint,
        grants: Vec<SkillXpGrant>,
        completed_office_duty: Option<(OfficeKind, u32)>,
    ) -> Result<Vec<SkillXpGrant>, CatCapabilityAuthorityError> {
        if self.receipts.len() >= MAX_CAPABILITY_RECEIPTS {
            return Err(CatCapabilityAuthorityError::ReceiptCapacityExceeded);
        }
        validate_grants(&grants)?;
        let cat_id = receipt_cat_id(&fingerprint).to_owned();
        let record = self
            .cats
            .get(&cat_id)
            .ok_or(CatCapabilityAuthorityError::UnknownCatId)?;
        let next_skills = checked_skill_update(&record.skills, &grants)?;
        let next_office_duty =
            checked_office_duty_update(&record.office_duty_minutes, completed_office_duty)?;
        let applied = AppliedReceipt {
            receipt_id: receipt_id.clone(),
            fingerprint,
            grants: grants.clone(),
        };
        // All fallible work occurred above; these two writes form the commit.
        let record = self
            .cats
            .get_mut(&cat_id)
            .expect("registered cat was preflighted");
        record.skills = next_skills;
        record.office_duty_minutes = next_office_duty;
        self.receipts.insert(receipt_id, applied);
        Ok(grants)
    }
}

fn outcome_grants(
    outcome: &ProductiveOutcome,
) -> Result<(Vec<SkillXpGrant>, Option<(OfficeKind, u32)>), CatCapabilityAuthorityError> {
    match outcome {
        ProductiveOutcome::Refused | ProductiveOutcome::Unassigned => Ok((Vec::new(), None)),
        ProductiveOutcome::Hauling { haul_legs } => Ok((
            (*haul_legs > 0)
                .then(|| SkillXpGrant {
                    skill_id: "hauling".to_owned(),
                    xp_centi: u64::from(*haul_legs) * skill_catalog::HAUL_LEG_XP_CENTI,
                    source: XpGrantSource::HaulLeg,
                    report_clearance_office: None,
                })
                .into_iter()
                .collect(),
            None,
        )),
        ProductiveOutcome::FailedProductive { .. } => Ok((Vec::new(), None)),
        ProductiveOutcome::Productive {
            productive_minutes,
            activity,
            office,
            supervised_by,
        } => {
            let mut grants = Vec::new();
            if let Some(activity) = activity {
                let secondary = activity
                    .secondary_skill_ids
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                grants.extend(skill_catalog::activity_xp_grants(
                    ActivityXpDeclaration {
                        primary_skill_id: &activity.primary_skill_id,
                        secondary_skill_ids: &secondary,
                        office_skill_id: None,
                        supervised_skill_ids: &[],
                        haul_legs: activity.haul_legs,
                    },
                    *productive_minutes,
                    skill_catalog::ActivityCompletion::Productive,
                ));
            }
            if let Some(office) = office {
                grants.extend(skill_catalog::office_duty_xp_grants(
                    *office,
                    *productive_minutes,
                ));
            }
            if let Some(office) = supervised_by {
                grants.extend(skill_catalog::supervised_officer_xp_grants(
                    *office,
                    *productive_minutes,
                ));
            }
            let grants = consolidate_grants(grants)?;
            Ok((
                grants,
                (*office).map(|office| (office, *productive_minutes)),
            ))
        }
    }
}

fn consolidate_grants(
    grants: Vec<SkillXpGrant>,
) -> Result<Vec<SkillXpGrant>, CatCapabilityAuthorityError> {
    let mut consolidated = BTreeMap::<(String, XpGrantSource, Option<OfficeKind>), u64>::new();
    for grant in grants {
        let key = (grant.skill_id, grant.source, grant.report_clearance_office);
        let entry = consolidated.entry(key).or_default();
        *entry = entry
            .checked_add(grant.xp_centi)
            .ok_or(CatCapabilityAuthorityError::XpOverflow)?;
    }
    Ok(consolidated
        .into_iter()
        .map(
            |((skill_id, source, report_clearance_office), xp_centi)| SkillXpGrant {
                skill_id,
                xp_centi,
                source,
                report_clearance_office,
            },
        )
        .collect())
}

fn checked_skill_update(
    skills: &BTreeMap<String, SkillProgress>,
    grants: &[SkillXpGrant],
) -> Result<BTreeMap<String, SkillProgress>, CatCapabilityAuthorityError> {
    let mut next = skills.clone();
    for grant in grants {
        let progress = next
            .entry(grant.skill_id.clone())
            .or_insert_with(|| SkillProgress::new(0));
        progress.total_xp_centi = progress
            .total_xp_centi
            .checked_add(grant.xp_centi)
            .ok_or(CatCapabilityAuthorityError::XpOverflow)?;
    }
    Ok(next)
}

fn checked_office_duty_update(
    duty: &BTreeMap<OfficeKind, u64>,
    increment: Option<(OfficeKind, u32)>,
) -> Result<BTreeMap<OfficeKind, u64>, CatCapabilityAuthorityError> {
    let mut next = duty.clone();
    if let Some((office, minutes)) = increment {
        let value = next.entry(office).or_default();
        *value = value
            .checked_add(u64::from(minutes))
            .ok_or(CatCapabilityAuthorityError::OfficeDutyOverflow)?;
    }
    Ok(next)
}

fn replay_or_conflict(
    applied: &AppliedReceipt,
    fingerprint: &ReceiptFingerprint,
) -> Result<Vec<SkillXpGrant>, CatCapabilityAuthorityError> {
    if &applied.fingerprint == fingerprint {
        Ok(applied.grants.clone())
    } else {
        Err(CatCapabilityAuthorityError::ReceiptConflict)
    }
}

fn validate_record(record: &CatCapabilityRecord) -> Result<(), CatCapabilityAuthorityError> {
    validate_cat_id(&record.cat_id)?;
    for attribute in InheritedAttribute::ALL {
        if !(1..=20).contains(&record.attributes.get(attribute)) {
            return Err(CatCapabilityAuthorityError::InvalidAttribute);
        }
    }
    for skill_id in record.skills.keys() {
        validate_skill_id(skill_id)?;
    }
    if record.skills.len() > skill_catalog::SKILL_DEFINITIONS.len() {
        return Err(CatCapabilityAuthorityError::TooManySkills);
    }
    for skill_id in record.labor.affinities.keys() {
        validate_skill_id(skill_id)?;
    }
    for skill_id in &record.labor.family_enterprise_skill_ids {
        validate_skill_id(skill_id)?;
    }
    if record.labor.affinities.len() > skill_catalog::SKILL_DEFINITIONS.len()
        || record.labor.family_enterprise_skill_ids.len() > skill_catalog::SKILL_DEFINITIONS.len()
    {
        return Err(CatCapabilityAuthorityError::TooManySkills);
    }
    Ok(())
}

fn validate_fingerprint(
    fingerprint: &ReceiptFingerprint,
) -> Result<(), CatCapabilityAuthorityError> {
    match fingerprint {
        ReceiptFingerprint::Aptitude {
            cat_id,
            compatible_skill_ids,
            ..
        } => {
            validate_cat_id(cat_id)?;
            validate_skill_id_sequence(compatible_skill_ids)
        }
        ReceiptFingerprint::Outcome { cat_id, outcome } => {
            validate_cat_id(cat_id)?;
            validate_outcome(outcome)
        }
    }
}

fn validate_outcome(outcome: &ProductiveOutcome) -> Result<(), CatCapabilityAuthorityError> {
    match outcome {
        ProductiveOutcome::Productive {
            activity,
            office,
            supervised_by,
            ..
        }
        | ProductiveOutcome::FailedProductive {
            activity,
            office,
            supervised_by,
            ..
        } => {
            if activity.is_none() && office.is_none() && supervised_by.is_none() {
                return Err(CatCapabilityAuthorityError::EmptyProductiveOutcome);
            }
            if let Some(activity) = activity {
                validate_skill_id(&activity.primary_skill_id)?;
                validate_skill_id_sequence(&activity.secondary_skill_ids)?;
            }
            Ok(())
        }
        ProductiveOutcome::Hauling { .. }
        | ProductiveOutcome::Refused
        | ProductiveOutcome::Unassigned => Ok(()),
    }
}

fn validate_grants(grants: &[SkillXpGrant]) -> Result<(), CatCapabilityAuthorityError> {
    if grants.len() > MAX_GRANTS_PER_CAPABILITY_RECEIPT {
        return Err(CatCapabilityAuthorityError::TooManyGrants);
    }
    for grant in grants {
        validate_skill_id(&grant.skill_id)?;
        if grant.xp_centi == 0 {
            return Err(CatCapabilityAuthorityError::ZeroXpGrant);
        }
        if let Some(office) = grant.report_clearance_office {
            if grant.skill_id != skill_catalog::office_proficiency_skill_id(office)
                || grant.source != XpGrantSource::Office
            {
                return Err(CatCapabilityAuthorityError::InvalidOfficeClearanceGrant);
            }
        }
    }
    Ok(())
}

fn validate_skill_id_sequence(ids: &[String]) -> Result<(), CatCapabilityAuthorityError> {
    if ids.len() > skill_catalog::SKILL_DEFINITIONS.len() {
        return Err(CatCapabilityAuthorityError::TooManySkills);
    }
    let mut previous = None;
    for skill_id in ids {
        validate_skill_id(skill_id)?;
        if previous.is_some_and(|previous: &str| previous >= skill_id.as_str()) {
            return Err(CatCapabilityAuthorityError::NonCanonicalSkillIds);
        }
        previous = Some(skill_id.as_str());
    }
    Ok(())
}

fn validate_skill_id(skill_id: &str) -> Result<(), CatCapabilityAuthorityError> {
    skill_catalog::skill_definition(skill_id)
        .is_some()
        .then_some(())
        .ok_or(CatCapabilityAuthorityError::UnknownSkillId)
}

fn validate_cat_id(cat_id: &str) -> Result<(), CatCapabilityAuthorityError> {
    valid_stable_id(cat_id, MAX_STABLE_CAT_ID_BYTES)
        .then_some(())
        .ok_or(CatCapabilityAuthorityError::InvalidCatId)
}

fn validate_receipt_id(receipt_id: &str) -> Result<(), CatCapabilityAuthorityError> {
    valid_stable_id(receipt_id, MAX_RECEIPT_ID_BYTES)
        .then_some(())
        .ok_or(CatCapabilityAuthorityError::InvalidReceiptId)
}

fn valid_stable_id(value: &str, max_bytes: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_bytes
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.' | b'|')
        })
}

fn receipt_cat_id(fingerprint: &ReceiptFingerprint) -> &str {
    match fingerprint {
        ReceiptFingerprint::Aptitude { cat_id, .. }
        | ReceiptFingerprint::Outcome { cat_id, .. } => cat_id,
    }
}

fn require_movement(function: &impl Fn(BodyPart) -> u16) -> Result<(), CapabilityEligibilityBlock> {
    require_paws(function)?;
    require_tail(function)
}

fn require_paws(function: &impl Fn(BodyPart) -> u16) -> Result<(), CapabilityEligibilityBlock> {
    for part in BodyPart::ALL[..4].iter().copied() {
        if function(part) < BODY_FUNCTION_MINIMUM_BASIS_POINTS {
            return Err(CapabilityEligibilityBlock::Paw(part));
        }
    }
    Ok(())
}

fn require_eyes(function: &impl Fn(BodyPart) -> u16) -> Result<(), CapabilityEligibilityBlock> {
    for part in BodyPart::ALL[4..6].iter().copied() {
        if function(part) < BODY_FUNCTION_MINIMUM_BASIS_POINTS {
            return Err(CapabilityEligibilityBlock::Eye(part));
        }
    }
    Ok(())
}

fn require_tail(function: &impl Fn(BodyPart) -> u16) -> Result<(), CapabilityEligibilityBlock> {
    if function(BodyPart::Tail) < BODY_FUNCTION_MINIMUM_BASIS_POINTS {
        Err(CapabilityEligibilityBlock::Tail)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatCapabilityAuthorityError {
    InvalidCatId,
    InvalidReceiptId,
    DuplicateCatId,
    UnknownCatId,
    CatIdImmutable,
    CatIdMismatch,
    ReceiptIdMismatch,
    ReceiptReferencesUnknownCat,
    ReceiptConflict,
    CatCapacityExceeded,
    ReceiptCapacityExceeded,
    InvalidAttribute,
    UnknownSkillId,
    NonCanonicalSkillIds,
    TooManySkills,
    EmptyProductiveOutcome,
    TooManyGrants,
    ZeroXpGrant,
    InvalidOfficeClearanceGrant,
    XpOverflow,
    OfficeDutyOverflow,
    IneligibleAnatomy(CapabilityEligibilityBlock),
}

impl std::fmt::Display for CatCapabilityAuthorityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "cat capability authority error: {self:?}")
    }
}

impl std::error::Error for CatCapabilityAuthorityError {}
