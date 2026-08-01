//! Family specialization, lineage, teaching, tradition, and enterprise contracts for LAI.56.
//!
//! Port target: `docs/leader-ai-overhaul/final-integrated-overhaul-plan.md`
//! section 3, "Family knowledge and professional dynasties". This is a pure
//! deterministic leaf; LAI.63 and later hot-root owners attach it to cats,
//! runtime tasks, persistence, protocol, and UI.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::skill_catalog::{
    SkillProgress, XP_CENTI_PER_PRIMARY_HOUR, XpGrantSource, productive_xp_centi_for_minutes,
};

pub const FAMILY_SPECIALIZATION_SCHEMA_VERSION: u16 = 1;
pub const BIRTH_FIRST_PARENT_BASIS_POINTS: u16 = 3_000;
pub const BIRTH_SECOND_PARENT_BASIS_POINTS: u16 = 3_000;
pub const BIRTH_BLEND_BASIS_POINTS: u16 = 1_250;
pub const BIRTH_BOTH_BASIS_POINTS: u16 = 1_250;
pub const BIRTH_NONE_BASIS_POINTS: u16 = 1_500;
pub const SINGLE_PARENT_TRANSFER_BASIS_POINTS: u64 = 500;
pub const BLEND_PARENT_TRANSFER_BASIS_POINTS: u64 = 250;
pub const BIRTH_SKILL_CAP_XP_CENTI: u64 = 625 * XP_CENTI_PER_PRIMARY_HOUR;
pub const TRADITION_LEARNING_BASIS_POINTS: u16 = 11_000;
pub const APPRENTICE_LEARNING_BASIS_POINTS: u16 = 12_500;
pub const TEACHER_TEACHING_XP_BASIS_POINTS: u64 = 1_000;
pub const POST_100_TEACHING_MASTERY_CAP_LEVELS: u16 = 25;
pub const POST_100_TEACHING_XP_PER_BONUS_LEVEL_CENTI: u64 = 40_000;
pub const MATURE_TRADITION_MIN_LEVEL: u16 = 50;
pub const MATURE_TRADITION_MIN_LINKED_GENERATIONS: usize = 2;
pub const MATURE_TRADITION_JOINT_SUCCESSFUL_UNITS: u32 = 200;
pub const OCCUPATIONAL_SURNAME_KEYS: [(&str, &str); 9] = [
    ("milling", "surname.miller"),
    ("metalworking", "surname.smith"),
    ("baking", "surname.baker"),
    ("textiles", "surname.weaver"),
    ("fishing", "surname.fisher"),
    ("hunting", "surname.hunter"),
    ("carpentry", "surname.carpenter"),
    ("research", "surname.scholar"),
    ("milling_de", "surname.muller"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BirthSeedOutcome {
    FirstParent,
    SecondParent,
    Blend,
    Both,
    None,
}

#[must_use]
pub const fn birth_seed_band_sum_basis_points() -> u16 {
    BIRTH_FIRST_PARENT_BASIS_POINTS
        + BIRTH_SECOND_PARENT_BASIS_POINTS
        + BIRTH_BLEND_BASIS_POINTS
        + BIRTH_BOTH_BASIS_POINTS
        + BIRTH_NONE_BASIS_POINTS
}

#[must_use]
pub fn keyed_birth_seed_outcome(
    child_key: &str,
    first_parent_id: &str,
    second_parent_id: &str,
) -> BirthSeedOutcome {
    let roll = keyed_hash3(child_key, first_parent_id, second_parent_id, "birth_seed") % 10_000;
    let first_end = u64::from(BIRTH_FIRST_PARENT_BASIS_POINTS);
    let second_end = first_end + u64::from(BIRTH_SECOND_PARENT_BASIS_POINTS);
    let blend_end = second_end + u64::from(BIRTH_BLEND_BASIS_POINTS);
    let both_end = blend_end + u64::from(BIRTH_BOTH_BASIS_POINTS);
    match roll {
        n if n < first_end => BirthSeedOutcome::FirstParent,
        n if n < second_end => BirthSeedOutcome::SecondParent,
        n if n < blend_end => BirthSeedOutcome::Blend,
        n if n < both_end => BirthSeedOutcome::Both,
        _ => BirthSeedOutcome::None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParentProfessionSeed {
    pub parent_cat_id: String,
    pub lineage_id: String,
    pub tradition_id: String,
    pub professional_skill_xp_centi: BTreeMap<String, u64>,
}

impl ParentProfessionSeed {
    #[must_use]
    pub fn new(
        parent_cat_id: impl Into<String>,
        lineage_id: impl Into<String>,
        tradition_id: impl Into<String>,
        professional_skill_xp_centi: BTreeMap<String, u64>,
    ) -> Self {
        Self {
            parent_cat_id: parent_cat_id.into(),
            lineage_id: lineage_id.into(),
            tradition_id: tradition_id.into(),
            professional_skill_xp_centi,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BirthSeedGrant {
    pub outcome: BirthSeedOutcome,
    pub inherited_skill_xp_centi: BTreeMap<String, u64>,
    pub source_lineage_ids: Vec<String>,
    pub aptitude_inheritance: AptitudeInheritance,
    pub inherited_personality_axes: Vec<PersonalityAxis>,
    pub inherited_acquired_trait_ids: Vec<String>,
}

/// Keeps inherited aptitude in the authoritative attribute system instead of
/// disguising it as a profession XP seed or an acquired life trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AptitudeInheritance {
    AuthoritativeAttributeSystem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalityAxis {
    RelationalAnalytical,
}

#[must_use]
pub fn birth_seed_grant(
    outcome: BirthSeedOutcome,
    first_parent: &ParentProfessionSeed,
    second_parent: &ParentProfessionSeed,
) -> BirthSeedGrant {
    let mut inherited_skill_xp_centi = BTreeMap::new();
    let mut source_lineage_ids = BTreeSet::new();
    match outcome {
        BirthSeedOutcome::FirstParent => {
            transfer_parent(
                &mut inherited_skill_xp_centi,
                first_parent,
                SINGLE_PARENT_TRANSFER_BASIS_POINTS,
            );
            source_lineage_ids.insert(first_parent.lineage_id.clone());
        }
        BirthSeedOutcome::SecondParent => {
            transfer_parent(
                &mut inherited_skill_xp_centi,
                second_parent,
                SINGLE_PARENT_TRANSFER_BASIS_POINTS,
            );
            source_lineage_ids.insert(second_parent.lineage_id.clone());
        }
        BirthSeedOutcome::Blend => {
            transfer_parent(
                &mut inherited_skill_xp_centi,
                first_parent,
                BLEND_PARENT_TRANSFER_BASIS_POINTS,
            );
            transfer_parent(
                &mut inherited_skill_xp_centi,
                second_parent,
                BLEND_PARENT_TRANSFER_BASIS_POINTS,
            );
            source_lineage_ids.insert(first_parent.lineage_id.clone());
            source_lineage_ids.insert(second_parent.lineage_id.clone());
        }
        BirthSeedOutcome::Both => {
            transfer_parent(
                &mut inherited_skill_xp_centi,
                first_parent,
                SINGLE_PARENT_TRANSFER_BASIS_POINTS,
            );
            transfer_parent(
                &mut inherited_skill_xp_centi,
                second_parent,
                SINGLE_PARENT_TRANSFER_BASIS_POINTS,
            );
            source_lineage_ids.insert(first_parent.lineage_id.clone());
            source_lineage_ids.insert(second_parent.lineage_id.clone());
        }
        BirthSeedOutcome::None => {}
    }
    for value in inherited_skill_xp_centi.values_mut() {
        *value = (*value).min(BIRTH_SKILL_CAP_XP_CENTI);
    }
    BirthSeedGrant {
        outcome,
        inherited_skill_xp_centi,
        source_lineage_ids: source_lineage_ids.into_iter().collect(),
        aptitude_inheritance: AptitudeInheritance::AuthoritativeAttributeSystem,
        inherited_personality_axes: vec![PersonalityAxis::RelationalAnalytical],
        inherited_acquired_trait_ids: Vec::new(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningContext {
    Ordinary,
    FamilyTradition,
    ParentApprentice,
    MentorApprentice,
    TraditionMentored,
}

#[must_use]
pub const fn learning_multiplier_basis_points(context: LearningContext) -> u16 {
    match context {
        LearningContext::Ordinary => 10_000,
        LearningContext::FamilyTradition => TRADITION_LEARNING_BASIS_POINTS,
        LearningContext::ParentApprentice | LearningContext::MentorApprentice => {
            APPRENTICE_LEARNING_BASIS_POINTS
        }
        LearningContext::TraditionMentored => 13_750,
    }
}

#[must_use]
pub fn apply_learning_context(ordinary_xp_centi: u64, context: LearningContext) -> u64 {
    ordinary_xp_centi.saturating_mul(u64::from(learning_multiplier_basis_points(context))) / 10_000
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeachingXpResult {
    pub learner_skill_id: String,
    pub learner_xp_centi: u64,
    pub teacher_grants: Vec<TeacherXpGrant>,
    pub teacher_xp_debited: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeacherXpGrant {
    pub skill_id: String,
    pub xp_centi: u64,
    pub source: XpGrantSource,
}

#[must_use]
pub fn formal_teaching_xp(
    learner_skill_id: &str,
    teacher_progress: SkillProgress,
    productive_minutes: u32,
) -> TeachingXpResult {
    let base = productive_xp_centi_for_minutes(productive_minutes);
    let effective_level = bounded_teaching_level(teacher_progress);
    let learner_xp_centi = base.saturating_mul(u64::from(effective_level)) / 100;
    let teacher_teaching_xp_centi = base.saturating_mul(TEACHER_TEACHING_XP_BASIS_POINTS) / 10_000;
    TeachingXpResult {
        learner_skill_id: learner_skill_id.to_owned(),
        learner_xp_centi,
        teacher_grants: vec![TeacherXpGrant {
            skill_id: "teaching".to_owned(),
            xp_centi: teacher_teaching_xp_centi,
            source: XpGrantSource::Primary,
        }],
        teacher_xp_debited: 0,
    }
}

#[must_use]
pub fn bounded_teaching_level(progress: SkillProgress) -> u16 {
    let mastery_bonus = (progress.mastery_xp_centi() / POST_100_TEACHING_XP_PER_BONUS_LEVEL_CENTI)
        .min(u64::from(POST_100_TEACHING_MASTERY_CAP_LEVELS)) as u16;
    progress.level().saturating_add(mastery_bonus)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerationProfessionRecord {
    pub cat_id: String,
    pub generation_index: u16,
    pub parent_cat_ids: Vec<String>,
    pub profession_id: String,
    pub skill_level: u16,
    pub successful_units: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub physical_enterprise_id: Option<String>,
}

#[must_use]
pub fn tradition_maturity(
    profession_id: &str,
    records: &[GenerationProfessionRecord],
    station_profession: bool,
) -> TraditionMaturity {
    let qualifying = records
        .iter()
        .filter(|record| record.profession_id == profession_id)
        .filter(|record| record.skill_level >= MATURE_TRADITION_MIN_LEVEL)
        .collect::<Vec<_>>();
    let mut linked_pair = None;
    'outer: for older in &qualifying {
        for younger in &qualifying {
            if older.generation_index >= younger.generation_index {
                continue;
            }
            let is_direct_genetic_link =
                younger.parent_cat_ids.iter().any(|id| id == &older.cat_id);
            let is_next_generation =
                younger.generation_index == older.generation_index.saturating_add(1);
            if !is_direct_genetic_link || !is_next_generation {
                continue;
            }
            let joint_units = older
                .successful_units
                .saturating_add(younger.successful_units);
            if joint_units < MATURE_TRADITION_JOINT_SUCCESSFUL_UNITS {
                continue;
            }
            if station_profession && older.physical_enterprise_id != younger.physical_enterprise_id
            {
                continue;
            }
            if station_profession && older.physical_enterprise_id.is_none() {
                continue;
            }
            linked_pair = Some((
                older.cat_id.clone(),
                younger.cat_id.clone(),
                joint_units,
                !station_profession
                    || older.physical_enterprise_id == younger.physical_enterprise_id,
            ));
            break 'outer;
        }
    }
    match linked_pair {
        Some((
            older_cat_id,
            younger_cat_id,
            joint_successful_units,
            station_enterprise_continuity,
        )) => TraditionMaturity {
            mature: true,
            older_cat_id: Some(older_cat_id),
            younger_cat_id: Some(younger_cat_id),
            joint_successful_units,
            station_enterprise_continuity,
        },
        None => TraditionMaturity {
            mature: false,
            older_cat_id: None,
            younger_cat_id: None,
            joint_successful_units: qualifying
                .iter()
                .map(|record| record.successful_units)
                .sum(),
            station_enterprise_continuity: false,
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TraditionMaturity {
    pub mature: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub older_cat_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub younger_cat_id: Option<String>,
    pub joint_successful_units: u32,
    pub station_enterprise_continuity: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyBranchRule {
    pub lineage_id: String,
    pub profession_id: String,
    pub adult_surname_key: String,
    pub child_surname_key: String,
    pub follows_profession: bool,
    pub ancestry_lineage_ids: Vec<String>,
}

impl FamilyBranchRule {
    #[must_use]
    pub fn retains_family_membership_when_leaving_trade(&self) -> bool {
        !self.lineage_id.is_empty() && !self.ancestry_lineage_ids.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FamilyEnterprise {
    pub enterprise_id: String,
    pub tradition_id: String,
    pub profession_id: String,
    pub site_id: String,
    pub signage_key: String,
    pub worker_preference: bool,
    pub mentoring_identity: bool,
    pub history_identity: bool,
    pub ui_identity: bool,
    pub goods_ownership: EnterpriseGoodsOwnership,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnterpriseGoodsOwnership {
    ColonyOwned,
}

#[must_use]
pub fn occupational_surname_key(profession_id: &str) -> Option<&'static str> {
    OCCUPATIONAL_SURNAME_KEYS
        .iter()
        .find(|(id, _)| *id == profession_id)
        .map(|(_, key)| *key)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionedFamilySpecializationState {
    pub schema_version: u16,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parent_seeds: BTreeMap<String, ParentProfessionSeed>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub enterprises: BTreeMap<String, FamilyEnterprise>,
}

impl VersionedFamilySpecializationState {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            schema_version: FAMILY_SPECIALIZATION_SCHEMA_VERSION,
            parent_seeds: BTreeMap::new(),
            enterprises: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), FamilySpecializationError> {
        if self.schema_version != FAMILY_SPECIALIZATION_SCHEMA_VERSION {
            return Err(FamilySpecializationError::UnsupportedVersion(
                self.schema_version,
            ));
        }
        for id in self.parent_seeds.keys().chain(self.enterprises.keys()) {
            if !is_stable_id(id) {
                return Err(FamilySpecializationError::InvalidStableId(id.clone()));
            }
        }
        for (key, seed) in &self.parent_seeds {
            if key != &seed.parent_cat_id {
                return Err(FamilySpecializationError::IdentityMismatch {
                    map_key: key.clone(),
                    embedded_id: seed.parent_cat_id.clone(),
                });
            }
            for id in [&seed.parent_cat_id, &seed.lineage_id, &seed.tradition_id] {
                if !is_stable_id(id) {
                    return Err(FamilySpecializationError::InvalidStableId(id.clone()));
                }
            }
            if let Some(skill_id) = seed
                .professional_skill_xp_centi
                .keys()
                .find(|skill_id| !is_stable_id(skill_id))
            {
                return Err(FamilySpecializationError::InvalidStableId(skill_id.clone()));
            }
        }
        for (key, enterprise) in &self.enterprises {
            if key != &enterprise.enterprise_id {
                return Err(FamilySpecializationError::IdentityMismatch {
                    map_key: key.clone(),
                    embedded_id: enterprise.enterprise_id.clone(),
                });
            }
            for id in [
                &enterprise.enterprise_id,
                &enterprise.tradition_id,
                &enterprise.profession_id,
                &enterprise.site_id,
            ] {
                if !is_stable_id(id) {
                    return Err(FamilySpecializationError::InvalidStableId(id.clone()));
                }
            }
            if enterprise.signage_key.trim().is_empty() {
                return Err(FamilySpecializationError::MissingLocalizationKey(
                    enterprise.enterprise_id.clone(),
                ));
            }
        }
        if self
            .enterprises
            .values()
            .any(|enterprise| enterprise.goods_ownership != EnterpriseGoodsOwnership::ColonyOwned)
        {
            return Err(FamilySpecializationError::NonColonyGoodsOwnership);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FamilySpecializationError {
    UnsupportedVersion(u16),
    InvalidStableId(String),
    IdentityMismatch {
        map_key: String,
        embedded_id: String,
    },
    MissingLocalizationKey(String),
    NonColonyGoodsOwnership,
}

#[must_use]
pub fn is_stable_id(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|first| first.is_ascii_lowercase())
        && value.len() <= 512
        && bytes.all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'-' | b'_' | b':' | b'.' | b'|')
        })
}

#[must_use]
pub fn keyed_hash3(a: &str, b: &str, c: &str, salt: &str) -> u64 {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in a
        .bytes()
        .chain([b'|'])
        .chain(b.bytes())
        .chain([b'|'])
        .chain(c.bytes())
        .chain([b'|'])
        .chain(salt.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

fn transfer_parent(
    inherited_skill_xp_centi: &mut BTreeMap<String, u64>,
    parent: &ParentProfessionSeed,
    basis_points: u64,
) {
    for (skill_id, xp_centi) in &parent.professional_skill_xp_centi {
        let transfer = xp_centi.saturating_mul(basis_points) / 10_000;
        if transfer > 0 {
            *inherited_skill_xp_centi
                .entry(skill_id.clone())
                .or_default() += transfer;
        }
    }
}
