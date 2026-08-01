//! Data-owned cat skill catalog for LAI.55.
//!
//! Port target: `docs/leader-ai-overhaul/final-integrated-overhaul-plan.md`
//! section 3, "Cat capability model" and "Officer and succession
//! cross-training". This leaf is exported for focused integration, but remains
//! outside `world_tick` until the single hot-root integration card.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const XP_CENTI_PER_PRIMARY_HOUR: u64 = 100;
pub const SECONDARY_XP_PERCENT: u64 = 25;
pub const SUPERVISED_XP_PERCENT: u64 = 10;
pub const HAUL_LEG_XP_CENTI: u64 = 25;
pub const AMBIENT_CLEANING_INTERVAL_MINUTES: u32 = 10;
pub const AMBIENT_CLEANING_XP_CENTI: u64 = 1;
pub const AMBIENT_DISCOVERY_CHANCE_PERCENT: u64 = 5;
pub const AMBIENT_DISCOVERY_XP_CENTI: u64 = 5;
pub const LEVEL_EFFECT_CAP: u16 = 100;
pub const LEVEL_100_XP_CENTI: u64 =
    (LEVEL_EFFECT_CAP as u64) * (LEVEL_EFFECT_CAP as u64) * XP_CENTI_PER_PRIMARY_HOUR;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficeKind {
    Steward,
    Accountant,
    Forester,
    Farmer,
    Captain,
    Loremaster,
    ClothLeader,
}

impl OfficeKind {
    pub const ALL: [Self; 7] = [
        Self::Steward,
        Self::Accountant,
        Self::Forester,
        Self::Farmer,
        Self::Captain,
        Self::Loremaster,
        Self::ClothLeader,
    ];

    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Steward => "steward",
            Self::Accountant => "accountant",
            Self::Forester => "forester",
            Self::Farmer => "farmer",
            Self::Captain => "captain",
            Self::Loremaster => "loremaster",
            Self::ClothLeader => "cloth_leader",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCategoryDefinition {
    pub id: &'static str,
    pub display_key: &'static str,
    pub stable_order: u16,
}

pub const SKILL_CATEGORIES: [SkillCategoryDefinition; 8] = [
    SkillCategoryDefinition {
        id: "gathering",
        display_key: "skill.category.gathering",
        stable_order: 10,
    },
    SkillCategoryDefinition {
        id: "construction_logistics",
        display_key: "skill.category.construction_logistics",
        stable_order: 20,
    },
    SkillCategoryDefinition {
        id: "food",
        display_key: "skill.category.food",
        stable_order: 30,
    },
    SkillCategoryDefinition {
        id: "industry",
        display_key: "skill.category.industry",
        stable_order: 40,
    },
    SkillCategoryDefinition {
        id: "care_service",
        display_key: "skill.category.care_service",
        stable_order: 50,
    },
    SkillCategoryDefinition {
        id: "martial_spiritual",
        display_key: "skill.category.martial_spiritual",
        stable_order: 60,
    },
    SkillCategoryDefinition {
        id: "civic",
        display_key: "skill.category.civic",
        stable_order: 70,
    },
    SkillCategoryDefinition {
        id: "office",
        display_key: "skill.category.office",
        stable_order: 80,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDefinition {
    pub id: &'static str,
    pub display_key: &'static str,
    pub category_id: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub office: Option<OfficeKind>,
}

pub const SKILL_DEFINITIONS: [SkillDefinition; 41] = [
    skill("hunting", "skill.hunting", "gathering"),
    skill("fishing", "skill.fishing", "gathering"),
    skill("foraging", "skill.foraging", "gathering"),
    skill("farming", "skill.farming", "gathering"),
    skill("waterwork", "skill.waterwork", "gathering"),
    skill("woodcutting", "skill.woodcutting", "gathering"),
    skill("quarrying", "skill.quarrying", "gathering"),
    skill("scouting", "skill.scouting", "gathering"),
    skill(
        "construction",
        "skill.construction",
        "construction_logistics",
    ),
    skill("roadwork", "skill.roadwork", "construction_logistics"),
    skill("hauling", "skill.hauling", "construction_logistics"),
    skill("milling", "skill.milling", "food"),
    skill("cooking", "skill.cooking", "food"),
    skill("preservation", "skill.preservation", "food"),
    skill("brewing", "skill.brewing", "food"),
    skill("woodworking", "skill.woodworking", "industry"),
    skill("crafting", "skill.crafting", "industry"),
    skill("textiles", "skill.textiles", "industry"),
    skill("tanning", "skill.tanning", "industry"),
    skill("metalworking", "skill.metalworking", "industry"),
    skill("gemwork", "skill.gemwork", "industry"),
    skill("medicine", "skill.medicine", "care_service"),
    skill("cleaning", "skill.cleaning", "care_service"),
    skill("teaching", "skill.teaching", "care_service"),
    skill("influence", "skill.influence", "care_service"),
    skill("fighting", "skill.fighting", "martial_spiritual"),
    skill("training", "skill.training", "martial_spiritual"),
    skill("ritual", "skill.ritual", "martial_spiritual"),
    skill("command", "skill.command", "martial_spiritual"),
    skill("research", "skill.research", "civic"),
    skill("trade", "skill.trade", "civic"),
    skill("diplomacy", "skill.diplomacy", "civic"),
    skill("governance", "skill.governance", "civic"),
    skill("administration", "skill.administration", "civic"),
    office_skill(
        "office_steward",
        "skill.office.steward",
        OfficeKind::Steward,
    ),
    office_skill(
        "office_accountant",
        "skill.office.accountant",
        OfficeKind::Accountant,
    ),
    office_skill(
        "office_forester",
        "skill.office.forester",
        OfficeKind::Forester,
    ),
    office_skill("office_farmer", "skill.office.farmer", OfficeKind::Farmer),
    office_skill(
        "office_captain",
        "skill.office.captain",
        OfficeKind::Captain,
    ),
    office_skill(
        "office_loremaster",
        "skill.office.loremaster",
        OfficeKind::Loremaster,
    ),
    office_skill(
        "office_cloth_leader",
        "skill.office.cloth_leader",
        OfficeKind::ClothLeader,
    ),
];

const fn skill(
    id: &'static str,
    display_key: &'static str,
    category_id: &'static str,
) -> SkillDefinition {
    SkillDefinition {
        id,
        display_key,
        category_id,
        office: None,
    }
}

const fn office_skill(
    id: &'static str,
    display_key: &'static str,
    office: OfficeKind,
) -> SkillDefinition {
    SkillDefinition {
        id,
        display_key,
        category_id: "office",
        office: Some(office),
    }
}

#[must_use]
pub fn skill_definition(id: &str) -> Option<&'static SkillDefinition> {
    SKILL_DEFINITIONS.iter().find(|skill| skill.id == id)
}

#[must_use]
pub fn skills_in_category(category_id: &str) -> Vec<&'static str> {
    SKILL_DEFINITIONS
        .iter()
        .filter(|skill| skill.category_id == category_id)
        .map(|skill| skill.id)
        .collect()
}

#[must_use]
pub fn office_proficiency_skill_id(office: OfficeKind) -> &'static str {
    SKILL_DEFINITIONS
        .iter()
        .find(|skill| skill.office == Some(office))
        .map(|skill| skill.id)
        .expect("every office has a proficiency skill")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityCompletion {
    Productive,
    Blocked,
    Waiting,
    InvalidRoute,
    FailedFabrication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum XpGrantSource {
    Primary,
    Secondary,
    Office,
    Supervised,
    HaulLeg,
    AmbientCleaning,
    AmbientDiscovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillXpGrant {
    pub skill_id: String,
    pub xp_centi: u64,
    pub source: XpGrantSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_clearance_office: Option<OfficeKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityXpDeclaration<'a> {
    pub primary_skill_id: &'a str,
    pub secondary_skill_ids: &'a [&'a str],
    pub office_skill_id: Option<&'a str>,
    pub supervised_skill_ids: &'a [&'a str],
    pub haul_legs: u16,
}

#[must_use]
pub fn productive_xp_centi_for_minutes(productive_minutes: u32) -> u64 {
    u64::from(productive_minutes) * XP_CENTI_PER_PRIMARY_HOUR / 60
}

#[must_use]
pub fn activity_xp_grants(
    declaration: ActivityXpDeclaration<'_>,
    productive_minutes: u32,
    completion: ActivityCompletion,
) -> Vec<SkillXpGrant> {
    if completion != ActivityCompletion::Productive {
        return Vec::new();
    }
    let primary = productive_xp_centi_for_minutes(productive_minutes);
    let mut grants = Vec::new();
    push_grant(
        &mut grants,
        declaration.primary_skill_id,
        primary,
        XpGrantSource::Primary,
        None,
    );
    for skill_id in declaration.secondary_skill_ids {
        push_grant(
            &mut grants,
            skill_id,
            primary * SECONDARY_XP_PERCENT / 100,
            XpGrantSource::Secondary,
            None,
        );
    }
    if let Some(skill_id) = declaration.office_skill_id {
        push_grant(&mut grants, skill_id, primary, XpGrantSource::Office, None);
    }
    for skill_id in declaration.supervised_skill_ids {
        push_grant(
            &mut grants,
            skill_id,
            primary * SUPERVISED_XP_PERCENT / 100,
            XpGrantSource::Supervised,
            None,
        );
    }
    if declaration.haul_legs > 0 {
        push_grant(
            &mut grants,
            "hauling",
            u64::from(declaration.haul_legs) * HAUL_LEG_XP_CENTI,
            XpGrantSource::HaulLeg,
            None,
        );
    }
    consolidate_grants(grants)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderDutyDomain {
    Diplomacy,
    Trade,
    Research,
    Command,
    Influence,
}

impl LeaderDutyDomain {
    #[must_use]
    pub const fn secondary_skill_id(self) -> &'static str {
        match self {
            Self::Diplomacy => "diplomacy",
            Self::Trade => "trade",
            Self::Research => "research",
            Self::Command => "command",
            Self::Influence => "influence",
        }
    }
}

#[must_use]
pub fn leader_duty_xp_grants(
    domain: LeaderDutyDomain,
    productive_minutes: u32,
) -> Vec<SkillXpGrant> {
    let primary = productive_xp_centi_for_minutes(productive_minutes);
    consolidate_grants(vec![
        grant("governance", primary, XpGrantSource::Primary, None),
        grant(
            domain.secondary_skill_id(),
            primary * SECONDARY_XP_PERCENT / 100,
            XpGrantSource::Secondary,
            None,
        ),
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfficeDutyProfile {
    pub office: OfficeKind,
    pub office_skill_id: &'static str,
    pub cross_training_skill_ids: &'static [&'static str],
    pub supervised_skill_ids: &'static [&'static str],
}

const STEWARD_CROSS: &[&str] = &["construction", "roadwork", "hauling"];
const ACCOUNTANT_CROSS: &[&str] = &["trade", "administration"];
const FORESTER_CROSS: &[&str] = &["woodcutting", "quarrying", "foraging"];
const FARMER_CROSS: &[&str] = &["farming", "cooking", "preservation"];
const CAPTAIN_CROSS: &[&str] = &["fighting", "training"];
const CAPTAIN_SUPERVISED: &[&str] = &["command"];
const LOREMASTER_CROSS: &[&str] = &["research", "teaching", "ritual"];
const CLOTH_LEADER_CROSS: &[&str] = &["textiles", "tanning", "crafting"];

#[must_use]
pub fn office_duty_profile(office: OfficeKind) -> OfficeDutyProfile {
    match office {
        OfficeKind::Steward => OfficeDutyProfile {
            office,
            office_skill_id: "office_steward",
            cross_training_skill_ids: STEWARD_CROSS,
            supervised_skill_ids: &[],
        },
        OfficeKind::Accountant => OfficeDutyProfile {
            office,
            office_skill_id: "office_accountant",
            cross_training_skill_ids: ACCOUNTANT_CROSS,
            supervised_skill_ids: &[],
        },
        OfficeKind::Forester => OfficeDutyProfile {
            office,
            office_skill_id: "office_forester",
            cross_training_skill_ids: FORESTER_CROSS,
            supervised_skill_ids: &[],
        },
        OfficeKind::Farmer => OfficeDutyProfile {
            office,
            office_skill_id: "office_farmer",
            cross_training_skill_ids: FARMER_CROSS,
            supervised_skill_ids: &[],
        },
        OfficeKind::Captain => OfficeDutyProfile {
            office,
            office_skill_id: "office_captain",
            cross_training_skill_ids: CAPTAIN_CROSS,
            supervised_skill_ids: CAPTAIN_SUPERVISED,
        },
        OfficeKind::Loremaster => OfficeDutyProfile {
            office,
            office_skill_id: "office_loremaster",
            cross_training_skill_ids: LOREMASTER_CROSS,
            supervised_skill_ids: &[],
        },
        OfficeKind::ClothLeader => OfficeDutyProfile {
            office,
            office_skill_id: "office_cloth_leader",
            cross_training_skill_ids: CLOTH_LEADER_CROSS,
            supervised_skill_ids: &[],
        },
    }
}

#[must_use]
pub fn office_duty_xp_grants(office: OfficeKind, productive_minutes: u32) -> Vec<SkillXpGrant> {
    let profile = office_duty_profile(office);
    let primary = productive_xp_centi_for_minutes(productive_minutes);
    let mut grants = vec![
        grant(
            profile.office_skill_id,
            primary,
            XpGrantSource::Office,
            Some(office),
        ),
        grant(
            "governance",
            primary * SECONDARY_XP_PERCENT / 100,
            XpGrantSource::Secondary,
            None,
        ),
    ];
    for skill_id in profile.cross_training_skill_ids {
        grants.push(grant(
            skill_id,
            primary * SECONDARY_XP_PERCENT / 100,
            XpGrantSource::Secondary,
            None,
        ));
    }
    consolidate_grants(grants)
}

#[must_use]
pub fn supervised_officer_xp_grants(
    office: OfficeKind,
    productive_minutes: u32,
) -> Vec<SkillXpGrant> {
    let profile = office_duty_profile(office);
    let primary = productive_xp_centi_for_minutes(productive_minutes);
    let mut grants = Vec::new();
    for skill_id in profile
        .cross_training_skill_ids
        .iter()
        .chain(profile.supervised_skill_ids.iter())
    {
        grants.push(grant(
            skill_id,
            primary * SUPERVISED_XP_PERCENT / 100,
            XpGrantSource::Supervised,
            None,
        ));
    }
    consolidate_grants(grants)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillProgress {
    pub total_xp_centi: u64,
}

impl SkillProgress {
    #[must_use]
    pub const fn new(total_xp_centi: u64) -> Self {
        Self { total_xp_centi }
    }

    #[must_use]
    pub fn level(self) -> u16 {
        floor_level_from_xp_centi(self.total_xp_centi).min(LEVEL_EFFECT_CAP)
    }

    #[must_use]
    pub fn output_effect_level(self) -> u16 {
        self.level().min(LEVEL_EFFECT_CAP)
    }

    #[must_use]
    pub fn mastery_xp_centi(self) -> u64 {
        self.total_xp_centi.saturating_sub(LEVEL_100_XP_CENTI)
    }
}

#[must_use]
pub fn floor_level_from_xp_centi(total_xp_centi: u64) -> u16 {
    let mut low = 0_u16;
    let mut high = LEVEL_EFFECT_CAP;
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        let threshold = u64::from(mid) * u64::from(mid) * XP_CENTI_PER_PRIMARY_HOUR;
        if threshold <= total_xp_centi {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    low
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmbientSkillCandidate<'a> {
    pub skill_id: &'a str,
    pub compatible: bool,
    pub refused: bool,
}

#[must_use]
pub fn ambient_cleaning_xp_grants(
    cat_stable_id: &str,
    interval_index: u64,
    candidates: &[AmbientSkillCandidate<'_>],
) -> Vec<SkillXpGrant> {
    let mut grants = vec![grant(
        "cleaning",
        AMBIENT_CLEANING_XP_CENTI,
        XpGrantSource::AmbientCleaning,
        None,
    )];
    let roll = keyed_hash(cat_stable_id, interval_index, "ambient_cleaning") % 100;
    if roll >= AMBIENT_DISCOVERY_CHANCE_PERCENT {
        return grants;
    }
    let eligible = candidates
        .iter()
        .filter(|candidate| candidate.compatible && !candidate.refused)
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        return grants;
    }
    let pick =
        (keyed_hash(cat_stable_id, interval_index, "ambient_pick") as usize) % eligible.len();
    grants.push(grant(
        eligible[pick].skill_id,
        AMBIENT_DISCOVERY_XP_CENTI,
        XpGrantSource::AmbientDiscovery,
        None,
    ));
    grants
}

#[must_use]
pub fn keyed_hash(cat_stable_id: &str, interval_index: u64, salt: &str) -> u64 {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in cat_stable_id
        .bytes()
        .chain([b'|'])
        .chain(interval_index.to_le_bytes())
        .chain([b'|'])
        .chain(salt.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

fn push_grant(
    grants: &mut Vec<SkillXpGrant>,
    skill_id: &str,
    xp_centi: u64,
    source: XpGrantSource,
    report_clearance_office: Option<OfficeKind>,
) {
    if xp_centi > 0 {
        grants.push(grant(skill_id, xp_centi, source, report_clearance_office));
    }
}

fn grant(
    skill_id: &str,
    xp_centi: u64,
    source: XpGrantSource,
    report_clearance_office: Option<OfficeKind>,
) -> SkillXpGrant {
    SkillXpGrant {
        skill_id: skill_id.to_owned(),
        xp_centi,
        source,
        report_clearance_office,
    }
}

fn consolidate_grants(grants: Vec<SkillXpGrant>) -> Vec<SkillXpGrant> {
    let mut by_key: BTreeMap<(String, XpGrantSource, Option<OfficeKind>), u64> = BTreeMap::new();
    for grant in grants {
        *by_key
            .entry((grant.skill_id, grant.source, grant.report_clearance_office))
            .or_default() += grant.xp_centi;
    }
    by_key
        .into_iter()
        .map(
            |((skill_id, source, report_clearance_office), xp_centi)| SkillXpGrant {
                skill_id,
                xp_centi,
                source,
                report_clearance_office,
            },
        )
        .collect()
}
