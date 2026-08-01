//! Expanded cat capability, affinity, matching, and anatomy eligibility for LAI.55.
//!
//! This leaf is deliberately data-only and deterministic so later integration can
//! attach it to the authoritative cat records without editing hot roots here.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::skill_catalog::{self, SkillProgress};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InheritedAttribute {
    Attack,
    Defense,
    Hunting,
    Medicine,
    Cleaning,
    Building,
    Leadership,
    Vision,
    Charisma,
    Intelligence,
}

impl InheritedAttribute {
    pub const ALL: [Self; 10] = [
        Self::Attack,
        Self::Defense,
        Self::Hunting,
        Self::Medicine,
        Self::Cleaning,
        Self::Building,
        Self::Leadership,
        Self::Vision,
        Self::Charisma,
        Self::Intelligence,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityAttributes {
    pub attack: u8,
    pub defense: u8,
    pub hunting: u8,
    pub medicine: u8,
    pub cleaning: u8,
    pub building: u8,
    pub leadership: u8,
    pub vision: u8,
    pub charisma: u8,
    pub intelligence: u8,
}

impl CapabilityAttributes {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        attack: u8,
        defense: u8,
        hunting: u8,
        medicine: u8,
        cleaning: u8,
        building: u8,
        leadership: u8,
        vision: u8,
        charisma: u8,
        intelligence: u8,
    ) -> Result<Self, AttributeError> {
        let value = Self {
            attack,
            defense,
            hunting,
            medicine,
            cleaning,
            building,
            leadership,
            vision,
            charisma,
            intelligence,
        };
        for attribute in InheritedAttribute::ALL {
            let score = value.get(attribute);
            if !(1..=20).contains(&score) {
                return Err(AttributeError {
                    attribute,
                    value: score,
                });
            }
        }
        Ok(value)
    }

    #[must_use]
    pub const fn get(self, attribute: InheritedAttribute) -> u8 {
        match attribute {
            InheritedAttribute::Attack => self.attack,
            InheritedAttribute::Defense => self.defense,
            InheritedAttribute::Hunting => self.hunting,
            InheritedAttribute::Medicine => self.medicine,
            InheritedAttribute::Cleaning => self.cleaning,
            InheritedAttribute::Building => self.building,
            InheritedAttribute::Leadership => self.leadership,
            InheritedAttribute::Vision => self.vision,
            InheritedAttribute::Charisma => self.charisma,
            InheritedAttribute::Intelligence => self.intelligence,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttributeError {
    pub attribute: InheritedAttribute,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillJudgmentUse {
    Learning,
    TechnicalJudgment,
    ResearchSelection,
    AppointmentSelection,
    Planning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityProfile {
    pub attributes: CapabilityAttributes,
    pub influence_xp_centi: u64,
}

impl CapabilityProfile {
    #[must_use]
    pub fn effective_charisma_basis_points(self) -> u16 {
        let learned = SkillProgress::new(self.influence_xp_centi).level().min(100);
        u16::from(self.attributes.charisma) * 1_000 + learned * 25
    }

    #[must_use]
    pub fn intelligence_modifier_basis_points(self, use_case: SkillJudgmentUse) -> u16 {
        let base = 10_000_i32 + (i32::from(self.attributes.intelligence) - 10) * 250;
        let weighted = match use_case {
            SkillJudgmentUse::Learning => base,
            SkillJudgmentUse::TechnicalJudgment => base + 250,
            SkillJudgmentUse::ResearchSelection => base + 500,
            SkillJudgmentUse::AppointmentSelection => base + 500,
            SkillJudgmentUse::Planning => base + 750,
        };
        weighted.clamp(7_500, 13_000) as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaborAffinity {
    Loved,
    Preferred,
    Neutral,
    Disliked,
    Refused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LaborAffinityProfile {
    pub affinities: BTreeMap<String, LaborAffinity>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub family_enterprise_skill_ids: BTreeSet<String>,
}

impl LaborAffinityProfile {
    #[must_use]
    pub fn affinity_for(&self, skill_id: &str) -> LaborAffinity {
        self.affinities
            .get(skill_id)
            .copied()
            .unwrap_or(LaborAffinity::Neutral)
    }

    #[must_use]
    pub fn is_family_enterprise(&self, skill_id: &str) -> bool {
        self.family_enterprise_skill_ids.contains(skill_id)
    }

    #[must_use]
    pub fn eligible_for_village_labor(&self, skill_id: &str) -> bool {
        self.affinity_for(skill_id) != LaborAffinity::Refused
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignmentTier {
    Emergency,
    LeaderPriority(u8),
    Background,
}

impl AssignmentTier {
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Emergency => 0,
            Self::LeaderPriority(priority) => priority,
            Self::Background => 6,
        }
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        match self {
            Self::Emergency | Self::Background => true,
            Self::LeaderPriority(priority) => priority >= 1 && priority <= 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentInput<'a> {
    pub cat_stable_id: &'a str,
    pub skill_id: &'a str,
    pub tier: AssignmentTier,
    pub affinity: LaborAffinity,
    pub family_enterprise: bool,
    pub skill_level: u16,
    pub attribute_score: u16,
    pub continuity_minutes: u32,
    pub route_cost: u32,
    pub anatomy: &'a EffectiveAnatomy,
    pub self_preservation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AssignmentCandidateKey {
    pub urgency_rank: u8,
    pub affinity_rank: u8,
    pub skill_rank: u16,
    pub attribute_rank: u16,
    pub continuity_rank: u32,
    pub route_rank: u32,
    pub stable_id: String,
}

#[must_use]
pub fn assignment_candidate_key(input: AssignmentInput<'_>) -> Option<AssignmentCandidateKey> {
    if !input.tier.is_valid() || input.affinity == LaborAffinity::Refused {
        return None;
    }
    if input.self_preservation {
        return None;
    }
    if anatomy_eligibility(input.skill_id, input.anatomy, true).is_err() {
        return None;
    }
    Some(AssignmentCandidateKey {
        urgency_rank: input.tier.rank(),
        affinity_rank: affinity_rank(input.family_enterprise, input.affinity),
        skill_rank: u16::MAX - input.skill_level.min(u16::MAX - 1),
        attribute_rank: u16::MAX - input.attribute_score.min(u16::MAX - 1),
        continuity_rank: u32::MAX - input.continuity_minutes.min(u32::MAX - 1),
        route_rank: input.route_cost,
        stable_id: input.cat_stable_id.to_owned(),
    })
}

#[must_use]
pub const fn affinity_rank(family_enterprise: bool, affinity: LaborAffinity) -> u8 {
    if family_enterprise {
        return 0;
    }
    match affinity {
        LaborAffinity::Loved => 1,
        LaborAffinity::Preferred => 2,
        LaborAffinity::Neutral => 3,
        LaborAffinity::Disliked => 4,
        LaborAffinity::Refused => 255,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityBodyPart {
    FrontLeftPaw,
    FrontRightPaw,
    HindLeftPaw,
    HindRightPaw,
    LeftEye,
    RightEye,
    Tail,
}

impl CapabilityBodyPart {
    pub const ALL: [Self; 7] = [
        Self::FrontLeftPaw,
        Self::FrontRightPaw,
        Self::HindLeftPaw,
        Self::HindRightPaw,
        Self::LeftEye,
        Self::RightEye,
        Self::Tail,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PartFunction {
    pub natural_basis_points: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prosthetic_basis_points: Option<u16>,
}

impl PartFunction {
    #[must_use]
    pub fn effective_basis_points(self, allow_prosthetic: bool) -> u16 {
        if self.natural_basis_points > 0 {
            return self.natural_basis_points.min(10_000);
        }
        if allow_prosthetic {
            self.prosthetic_basis_points.unwrap_or(0).min(10_000)
        } else {
            0
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveAnatomy {
    pub parts: BTreeMap<CapabilityBodyPart, PartFunction>,
}

impl EffectiveAnatomy {
    #[must_use]
    pub fn healthy() -> Self {
        let mut parts = BTreeMap::new();
        for part in CapabilityBodyPart::ALL {
            parts.insert(
                part,
                PartFunction {
                    natural_basis_points: 10_000,
                    prosthetic_basis_points: None,
                },
            );
        }
        Self { parts }
    }

    #[must_use]
    pub fn with_part(mut self, part: CapabilityBodyPart, state: PartFunction) -> Self {
        self.parts.insert(part, state);
        self
    }

    #[must_use]
    pub fn part_function(&self, part: CapabilityBodyPart, allow_prosthetic: bool) -> u16 {
        self.parts
            .get(&part)
            .copied()
            .unwrap_or(PartFunction {
                natural_basis_points: 0,
                prosthetic_basis_points: None,
            })
            .effective_basis_points(allow_prosthetic)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyRequirementKind {
    None,
    Movement,
    PhysicalLabor,
    Vision,
    Combat,
    RangedCombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnatomyBlock {
    Paw(CapabilityBodyPart),
    Eye(CapabilityBodyPart),
    Tail,
}

pub const BODY_FUNCTION_MINIMUM_BASIS_POINTS: u16 = 5_000;

#[must_use]
pub fn body_requirement_for_skill(skill_id: &str) -> BodyRequirementKind {
    match skill_id {
        "hunting" | "scouting" => BodyRequirementKind::Vision,
        "woodcutting" | "quarrying" | "construction" | "roadwork" | "hauling" | "farming"
        | "milling" | "cooking" | "preservation" | "brewing" | "woodworking" | "crafting"
        | "textiles" | "tanning" | "metalworking" | "gemwork" | "medicine" | "cleaning"
        | "teaching" | "ritual" => BodyRequirementKind::PhysicalLabor,
        "fighting" | "training" | "command" => BodyRequirementKind::Combat,
        "fishing" | "foraging" | "waterwork" | "research" | "trade" | "diplomacy"
        | "governance" | "administration" | "influence" => BodyRequirementKind::Movement,
        _ if skill_catalog::skill_definition(skill_id).is_some() => BodyRequirementKind::Movement,
        _ => BodyRequirementKind::None,
    }
}

pub fn anatomy_eligibility(
    skill_id: &str,
    anatomy: &EffectiveAnatomy,
    allow_prosthetic: bool,
) -> Result<(), AnatomyBlock> {
    match body_requirement_for_skill(skill_id) {
        BodyRequirementKind::None => Ok(()),
        BodyRequirementKind::Movement => require_movement(anatomy, allow_prosthetic),
        BodyRequirementKind::PhysicalLabor => {
            require_movement(anatomy, allow_prosthetic)?;
            require_paws(anatomy, allow_prosthetic)
        }
        BodyRequirementKind::Vision => {
            require_movement(anatomy, allow_prosthetic)?;
            require_eyes(anatomy, allow_prosthetic)
        }
        BodyRequirementKind::Combat => {
            require_movement(anatomy, allow_prosthetic)?;
            require_paws(anatomy, allow_prosthetic)?;
            require_tail(anatomy, allow_prosthetic)
        }
        BodyRequirementKind::RangedCombat => {
            require_movement(anatomy, allow_prosthetic)?;
            require_eyes(anatomy, allow_prosthetic)?;
            require_tail(anatomy, allow_prosthetic)
        }
    }
}

fn require_movement(
    anatomy: &EffectiveAnatomy,
    allow_prosthetic: bool,
) -> Result<(), AnatomyBlock> {
    require_paws(anatomy, allow_prosthetic)?;
    require_tail(anatomy, allow_prosthetic)
}

fn require_paws(anatomy: &EffectiveAnatomy, allow_prosthetic: bool) -> Result<(), AnatomyBlock> {
    for part in [
        CapabilityBodyPart::FrontLeftPaw,
        CapabilityBodyPart::FrontRightPaw,
        CapabilityBodyPart::HindLeftPaw,
        CapabilityBodyPart::HindRightPaw,
    ] {
        if anatomy.part_function(part, allow_prosthetic) < BODY_FUNCTION_MINIMUM_BASIS_POINTS {
            return Err(AnatomyBlock::Paw(part));
        }
    }
    Ok(())
}

fn require_eyes(anatomy: &EffectiveAnatomy, allow_prosthetic: bool) -> Result<(), AnatomyBlock> {
    for part in [CapabilityBodyPart::LeftEye, CapabilityBodyPart::RightEye] {
        if anatomy.part_function(part, allow_prosthetic) < BODY_FUNCTION_MINIMUM_BASIS_POINTS {
            return Err(AnatomyBlock::Eye(part));
        }
    }
    Ok(())
}

fn require_tail(anatomy: &EffectiveAnatomy, allow_prosthetic: bool) -> Result<(), AnatomyBlock> {
    if anatomy.part_function(CapabilityBodyPart::Tail, allow_prosthetic)
        < BODY_FUNCTION_MINIMUM_BASIS_POINTS
    {
        return Err(AnatomyBlock::Tail);
    }
    Ok(())
}
