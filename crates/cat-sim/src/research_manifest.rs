//! LAI.58 canonical research-manifest leaf specified by
//! `docs/branch-plan-merge/bug-gui-design-BOARD.md`.
//!
//! This leaf semantically imports the useful source catalog, removes the
//! obsolete Shrine/Favor/generic-food/coin authority, and overlays the complete
//! Plan 1 + Plan 2 capability surface. Counts are always derived from the built
//! manifest; the historical 531-node snapshot is deliberately not a live rule.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::OnceLock,
};

use serde::{Deserialize, Serialize};

use crate::research_catalog::{ResearchNode, ResearchPayload, research_catalog};

pub const RESEARCH_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const ADDITIVE_TRACK_COUNT: usize = 4;
pub const ADDITIVE_TRACK_STAGE_COUNT: usize = 11;
pub const ADDITIVE_TRACK_STUDY_COUNT: usize = ADDITIVE_TRACK_COUNT * ADDITIVE_TRACK_STAGE_COUNT;
pub const HISTORICAL_SOURCE_MANIFEST_STUDY_COUNT: usize = 531;

/// The fourteen global modifiers are presentation tracks over the canonical
/// node ledger.  A track has ten finite levels and one explicitly separate,
/// repeatable terminal; it is never an eleventh finite building level.
pub const GLOBAL_MODIFIER_TRACK_IDS: [&str; 14] = [
    "logistics",
    "construction",
    "scholarship",
    "governance",
    "welfare",
    "exploration",
    "defense_doctrine",
    "combat_doctrine",
    "storage",
    "agriculture",
    "water_management",
    "craftsmanship",
    "trade",
    "resilience",
];
pub const GLOBAL_MODIFIER_FINITE_LEVELS: u8 = 10;
pub const GLOBAL_MODIFIER_TERMINAL_LEVEL: u8 = 11;
pub const RESEARCH_GRAPH_ALLOWS_ZOOM: bool = false;
pub const RESEARCH_GRAPH_DRAG_PANNING: bool = true;
pub const RESEARCH_GRAPH_REGION_OWNS_SCROLL: bool = true;
pub const MINIMUM_AND_JUNCTIONS: usize = 24;
pub const CURATED_CONVERGENCE_JUNCTION_IDS: [&str; 8] = [
    "stone_tools",
    "metal_tools",
    "precision_tools",
    "civil_engineering",
    "preservation_science",
    "organized_provisioning",
    "public_administration",
    "combined_arms",
];

pub const DIVINE_DURATION_ALLOWED_GAME_HOURS: [u8; 12] = [1, 2, 3, 4, 6, 8, 10, 12, 16, 18, 21, 24];
pub const DIVINE_DURATION_STAGE_MAX_GAME_HOURS: [u8; ADDITIVE_TRACK_STAGE_COUNT] =
    [2, 3, 4, 6, 8, 10, 12, 16, 18, 21, 24];
pub const DIVINE_ECONOMY_STAGE_DISCOUNT_BASIS_POINTS: [u16; ADDITIVE_TRACK_STAGE_COUNT] = [
    300, 600, 900, 1_200, 1_500, 1_800, 2_100, 2_400, 2_700, 3_000, 3_300,
];
pub const REHABILITATION_STAGE_RESTORATION_PERCENTAGE_POINTS: [u8; ADDITIVE_TRACK_STAGE_COUNT] =
    [2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22];
pub const ADMINISTRATION_BASE_STANDING_ORDER_SLOTS: u8 = 3;
pub const ADMINISTRATION_BASE_STRATEGIC_INTENT_SLOTS: u8 = 4;
pub const ADMINISTRATION_STAGE_STANDING_ORDER_SLOTS: [u8; ADDITIVE_TRACK_STAGE_COUNT] =
    [4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
pub const ADMINISTRATION_STAGE_STRATEGIC_INTENT_SLOTS: [u8; ADDITIVE_TRACK_STAGE_COUNT] =
    [4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9];

pub const DEPRECATED_STUDY_IDS: &[&str] = &[
    "den_stores",
    "beds_stores",
    "herb_garden_stores",
    "nursery_stores",
    "elder_corner_stores",
    "walls_stores",
    "mouse_farm_stores",
    "shrine_stores",
    "field_stores",
    "research_hut_stores",
    "school_stores",
    "barracks_stores",
    "accounting_tent_stores",
];

const OBSOLETE_STUDY_FRAGMENTS: &[&str] = &[
    "shrine",
    "favor",
    "blessing",
    "coin",
    "purse",
    "generic_food",
    "food_storage",
];
const OBSOLETE_EXACT_IDENTITIES: &[&str] = &[
    "flour_to_food",
    "dry_food",
    "smoke_food",
    "pickle_food",
    "preserve_rations",
    "preserve_masterwork_feast",
];

/// Stable capability families added by the two accepted plans.  The third
/// field is a prerequisite CSV; two prerequisites deliberately form visible
/// AND junctions rather than hiding conjunctions in effect handlers.
const REQUIRED_CAPABILITY_FAMILIES: &[(&str, &str, &str)] = &[
    ("typed_food_handling", "Typed Food Handling", ""),
    ("apple_ecology", "Apple Ecology", "typed_food_handling"),
    ("hand_fishing", "Hand Fishing", "typed_food_handling"),
    (
        "universal_quality",
        "Universal Quality",
        "typed_food_handling",
    ),
    (
        "physical_lot_ledger",
        "Physical Lot Ledger",
        "universal_quality",
    ),
    (
        "cookhouse",
        "Cookhouse Permit",
        "typed_food_handling,physical_lot_ledger",
    ),
    (
        "fishing_hut",
        "Fishing Hut Permit",
        "hand_fishing,physical_lot_ledger",
    ),
    (
        "fishing_rods",
        "Fishing Rods",
        "hand_fishing,universal_quality",
    ),
    ("hunting_lairs", "Hunting Lairs", "universal_quality"),
    (
        "hunting_parties",
        "Hunting Parties",
        "hunting_lairs,physical_lot_ledger",
    ),
    ("plank_processing", "Global Plank Processing", ""),
    (
        "material_processing",
        "Material Processing",
        "plank_processing,universal_quality",
    ),
    (
        "augmentations",
        "Typed Augmentations",
        "material_processing,physical_lot_ledger",
    ),
    (
        "station_fixtures",
        "Typed Station Fixtures",
        "material_processing,physical_lot_ledger",
    ),
    (
        "research_instruments",
        "Research Instruments",
        "material_processing,universal_quality",
    ),
    (
        "black_hole_foundations",
        "The Hole Foundations",
        "physical_lot_ledger",
    ),
    (
        "black_hole_width",
        "Hole Width",
        "black_hole_foundations,plank_processing",
    ),
    (
        "black_hole_depth",
        "Hole Depth",
        "black_hole_foundations,material_processing",
    ),
    (
        "black_hole_darkness",
        "Hole Darkness",
        "black_hole_width,black_hole_depth",
    ),
    (
        "void_insight_studies",
        "Void Insight Studies",
        "black_hole_darkness,research_instruments",
    ),
    ("governance_foundations", "Governance Foundations", ""),
    (
        "partnerships",
        "Persistent Partnerships",
        "governance_foundations",
    ),
    (
        "family_homes",
        "Family Home Permit",
        "partnerships,plank_processing",
    ),
    (
        "elder_lodge",
        "Elder Lodge Permit",
        "family_homes,universal_quality",
    ),
    (
        "nursery",
        "Nursery Permit",
        "family_homes,governance_foundations",
    ),
    (
        "mentorship",
        "Physical Mentorship",
        "nursery,research_instruments",
    ),
    (
        "family_traditions",
        "Family Traditions",
        "mentorship,partnerships",
    ),
    (
        "professional_enterprises",
        "Professional Enterprises",
        "family_traditions,station_fixtures",
    ),
    (
        "three_stage_construction",
        "Three-stage Construction",
        "plank_processing,physical_lot_ledger",
    ),
    (
        "building_upgrade_permits",
        "Building Upgrade Permits",
        "three_stage_construction,governance_foundations",
    ),
    (
        "storage_zones",
        "Physical Storage Zones",
        "physical_lot_ledger,governance_foundations",
    ),
    (
        "basket_containers",
        "Basket Containers",
        "storage_zones,typed_food_handling",
    ),
    (
        "barrel_containers",
        "Barrel Containers",
        "storage_zones,material_processing",
    ),
    (
        "crate_containers",
        "Crate Containers",
        "storage_zones,plank_processing",
    ),
    (
        "chest_containers",
        "Chest Containers",
        "storage_zones,station_fixtures",
    ),
    (
        "rack_containers",
        "Rack Containers",
        "storage_zones,material_processing",
    ),
    (
        "linked_workshop_storage",
        "Linked Workshop Storage",
        "storage_zones,professional_enterprises",
    ),
    (
        "physical_farms",
        "Physical Farms",
        "typed_food_handling,storage_zones",
    ),
    (
        "authored_roads",
        "Authored Roads",
        "three_stage_construction,governance_foundations",
    ),
    (
        "walls_and_gates",
        "Walls and Gates",
        "authored_roads,material_processing",
    ),
    (
        "food_permissions",
        "Leader Food Permissions",
        "typed_food_handling,governance_foundations",
    ),
    (
        "material_barter",
        "Material Barter",
        "physical_lot_ledger,governance_foundations",
    ),
    (
        "village_diplomacy",
        "Village Diplomacy",
        "material_barter,professional_enterprises",
    ),
    (
        "prosthetic_rehabilitation",
        "Prosthetic Rehabilitation",
        "material_processing,mentorship",
    ),
];

const RARE_MATERIAL_CAPABILITIES: &[(&str, &str)] = &[
    ("bat_wing_processing", "Bat Wing Processing"),
    ("fox_pelt_processing", "Fox Pelt Processing"),
    ("badger_pelt_processing", "Badger Pelt Processing"),
    ("boar_tusk_processing", "Boar Tusk Processing"),
    ("wolf_pelt_processing", "Wolf Pelt Processing"),
    ("lynx_pelt_processing", "Lynx Pelt Processing"),
    ("stag_antler_processing", "Stag Antler Processing"),
    ("serpent_scale_processing", "Serpent Scale Processing"),
    ("bear_pelt_processing", "Bear Pelt Processing"),
    ("eagle_feather_processing", "Eagle Feather Processing"),
    ("moon_antler_processing", "Moon Antler Processing"),
    ("warg_fang_processing", "Warg Fang Processing"),
    ("cockatrice_eye_processing", "Cockatrice Eye Processing"),
    ("troll_hide_processing", "Troll Hide Processing"),
    ("griffin_plume_processing", "Griffin Plume Processing"),
    ("basilisk_scale_processing", "Basilisk Scale Processing"),
    ("manticore_barb_processing", "Manticore Barb Processing"),
    ("beast_core_processing", "Beast Core Processing"),
    ("wyvern_membrane_processing", "Wyvern Membrane Processing"),
    ("dragon_heart_processing", "Dragon Heart Processing"),
];

const CURATED_CONVERGENCE_SPECS: &[(&str, &str, &str)] = &[
    (
        "stone_tools",
        "Stone Tools",
        "plank_processing,material_processing",
    ),
    (
        "metal_tools",
        "Metal Tools",
        "stone_tools,material_processing",
    ),
    (
        "precision_tools",
        "Precision Tools",
        "metal_tools,research_instruments",
    ),
    (
        "civil_engineering",
        "Civil Engineering",
        "three_stage_construction,authored_roads",
    ),
    (
        "preservation_science",
        "Preservation Science",
        "typed_food_handling,barrel_containers",
    ),
    (
        "organized_provisioning",
        "Organized Provisioning",
        "storage_zones,food_permissions",
    ),
    (
        "public_administration",
        "Public Administration",
        "governance_foundations,professional_enterprises",
    ),
    (
        "combined_arms",
        "Combined Arms",
        "hunting_parties,walls_and_gates",
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestTrack {
    DivineDuration,
    DivineEconomy,
    Rehabilitation,
    Administration,
}

impl ManifestTrack {
    pub const ALL: [Self; ADDITIVE_TRACK_COUNT] = [
        Self::DivineDuration,
        Self::DivineEconomy,
        Self::Rehabilitation,
        Self::Administration,
    ];

    #[must_use]
    pub const fn stable_prefix(self) -> &'static str {
        match self {
            Self::DivineDuration => "divine_duration",
            Self::DivineEconomy => "divine_economy",
            Self::Rehabilitation => "rehabilitation",
            Self::Administration => "administration",
        }
    }

    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::DivineDuration => "Divine Duration",
            Self::DivineEconomy => "Divine Economy",
            Self::Rehabilitation => "Rehabilitation",
            Self::Administration => "Administration",
        }
    }

    #[must_use]
    pub const fn root_prerequisite(self) -> &'static str {
        match self {
            Self::DivineDuration | Self::DivineEconomy => "void_insight_studies",
            Self::Rehabilitation => "prosthetic_rehabilitation",
            Self::Administration => "governance_foundations",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestEffectHandler {
    CatalogBuildingAvailableAtFounding,
    CatalogUnlockBuilding,
    CatalogUnlockRecipe,
    CatalogUnlockResource,
    CatalogUnlockJob,
    CatalogModifyBuilding,
    CatalogModifyEffect,
    CatalogUnlockCapability,
    DivineBoostDuration,
    DivineBoostEconomy,
    ProstheticRehabilitation,
    AdministrationCapacity,
}

pub const LIVE_EFFECT_HANDLERS: &[ManifestEffectHandler] = &[
    ManifestEffectHandler::CatalogBuildingAvailableAtFounding,
    ManifestEffectHandler::CatalogUnlockBuilding,
    ManifestEffectHandler::CatalogUnlockRecipe,
    ManifestEffectHandler::CatalogUnlockResource,
    ManifestEffectHandler::CatalogUnlockJob,
    ManifestEffectHandler::CatalogModifyBuilding,
    ManifestEffectHandler::CatalogModifyEffect,
    ManifestEffectHandler::CatalogUnlockCapability,
    ManifestEffectHandler::DivineBoostDuration,
    ManifestEffectHandler::DivineBoostEconomy,
    ManifestEffectHandler::ProstheticRehabilitation,
    ManifestEffectHandler::AdministrationCapacity,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ManifestEffect {
    CatalogPayload {
        handler: ManifestEffectHandler,
        target_id: String,
    },
    DivineDuration {
        stage: u8,
        max_duration_game_hours: u8,
    },
    DivineEconomy {
        stage: u8,
        discount_basis_points: u16,
    },
    Rehabilitation {
        stage: u8,
        restoration_bonus_percentage_points: u8,
    },
    Administration {
        stage: u8,
        standing_order_slots: u8,
        strategic_intent_slots: u8,
    },
}

impl ManifestEffect {
    #[must_use]
    pub const fn handler(&self) -> ManifestEffectHandler {
        match self {
            Self::CatalogPayload { handler, .. } => *handler,
            Self::DivineDuration { .. } => ManifestEffectHandler::DivineBoostDuration,
            Self::DivineEconomy { .. } => ManifestEffectHandler::DivineBoostEconomy,
            Self::Rehabilitation { .. } => ManifestEffectHandler::ProstheticRehabilitation,
            Self::Administration { .. } => ManifestEffectHandler::AdministrationCapacity,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestStudySource {
    CurrentCatalog,
    AdditiveTrack(ManifestTrack),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestStudy {
    pub stable_id: String,
    pub display_name: String,
    pub prerequisites: Vec<String>,
    pub source: ManifestStudySource,
    pub stage: Option<u8>,
    /// The finite price for ordinary studies and the next frozen price for an
    /// infinite terminal. Repeat completions double this value.
    pub cost_units: u64,
    pub repeatable_terminal: bool,
    pub order_index: usize,
    pub effects: Vec<ManifestEffect>,
}

impl ManifestStudy {
    fn from_catalog(index: usize, node: &ResearchNode) -> Self {
        Self {
            stable_id: node.id.clone(),
            display_name: node.name.clone(),
            prerequisites: node.prerequisites.clone(),
            source: ManifestStudySource::CurrentCatalog,
            stage: None,
            cost_units: node.cost.ceil().max(1.0) as u64,
            repeatable_terminal: false,
            order_index: index,
            effects: node.payloads.iter().map(catalog_effect).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchManifest {
    pub schema_version: u32,
    studies: Vec<ManifestStudy>,
    by_id: BTreeMap<String, usize>,
}

impl ResearchManifest {
    pub fn from_studies(mut studies: Vec<ManifestStudy>) -> Result<Self, ResearchManifestError> {
        studies.sort_by(|left, right| {
            left.order_index
                .cmp(&right.order_index)
                .then_with(|| left.stable_id.cmp(&right.stable_id))
        });
        let mut manifest = Self {
            schema_version: RESEARCH_MANIFEST_SCHEMA_VERSION,
            studies,
            by_id: BTreeMap::new(),
        };
        manifest.validate()?;
        Ok(manifest)
    }

    #[must_use]
    pub fn studies(&self) -> &[ManifestStudy] {
        &self.studies
    }

    #[must_use]
    pub fn get(&self, stable_id: &str) -> Option<&ManifestStudy> {
        self.by_id.get(stable_id).map(|index| &self.studies[*index])
    }

    pub fn validate(&mut self) -> Result<(), ResearchManifestError> {
        if self.schema_version != RESEARCH_MANIFEST_SCHEMA_VERSION {
            return Err(ResearchManifestError::UnsupportedSchemaVersion);
        }
        if self.studies.is_empty() {
            return Err(ResearchManifestError::EmptyManifest);
        }

        self.by_id.clear();
        let mut display_names = BTreeSet::new();
        let mut order_indices = BTreeSet::new();
        for (index, study) in self.studies.iter().enumerate() {
            validate_study_identity(study)?;
            if self.by_id.insert(study.stable_id.clone(), index).is_some() {
                return Err(ResearchManifestError::DuplicateStableId(
                    study.stable_id.clone(),
                ));
            }
            if !display_names.insert(study.display_name.clone()) {
                return Err(ResearchManifestError::DuplicateDisplayName(
                    study.display_name.clone(),
                ));
            }
            if !order_indices.insert(study.order_index) {
                return Err(ResearchManifestError::DuplicateOrderIndex(
                    study.order_index,
                ));
            }
            if is_obsolete_stable_id(&study.stable_id) {
                return Err(ResearchManifestError::DeprecatedStudyPresent(
                    study.stable_id.clone(),
                ));
            }
            if study.cost_units == 0
                || study.repeatable_terminal && study.stage != Some(GLOBAL_MODIFIER_TERMINAL_LEVEL)
            {
                return Err(ResearchManifestError::MalformedRepeatable(
                    study.stable_id.clone(),
                ));
            }
            if study.effects.is_empty() {
                return Err(ResearchManifestError::StudyHasNoEffect(
                    study.stable_id.clone(),
                ));
            }
            for effect in &study.effects {
                if !LIVE_EFFECT_HANDLERS.contains(&effect.handler()) {
                    return Err(ResearchManifestError::UnknownEffectHandler {
                        study_id: study.stable_id.clone(),
                        handler: effect.handler(),
                    });
                }
            }
        }

        self.validate_track_shape()?;
        self.validate_topology()
    }

    #[must_use]
    pub fn starting_frontier_ids(&self) -> Vec<&str> {
        self.studies
            .iter()
            .filter(|study| study.prerequisites.is_empty())
            .map(|study| study.stable_id.as_str())
            .collect()
    }

    #[must_use]
    pub fn reachable_study_ids(&self) -> BTreeSet<&str> {
        let mut reachable = BTreeSet::new();
        let mut ready = self
            .studies
            .iter()
            .filter(|study| study.prerequisites.is_empty())
            .map(|study| study.stable_id.as_str())
            .collect::<BTreeSet<_>>();
        while let Some(id) = ready.pop_first() {
            if !reachable.insert(id) {
                continue;
            }
            for study in &self.studies {
                if !reachable.contains(study.stable_id.as_str())
                    && study
                        .prerequisites
                        .iter()
                        .all(|prerequisite| reachable.contains(prerequisite.as_str()))
                {
                    ready.insert(study.stable_id.as_str());
                }
            }
        }
        reachable
    }

    #[must_use]
    pub fn track_studies(&self, track: ManifestTrack) -> Vec<&ManifestStudy> {
        self.studies
            .iter()
            .filter(|study| study.source == ManifestStudySource::AdditiveTrack(track))
            .collect()
    }

    /// Derive the graph totals that are safe to show in a fixed-scale research
    /// graph.  This deliberately reads the canonical manifest rather than
    /// retaining old migration totals as authority.
    #[must_use]
    pub fn graph_totals(&self) -> ResearchGraphTotals {
        let track_count = GLOBAL_MODIFIER_TRACK_IDS
            .iter()
            .filter(|track_id| !self.global_modifier_track_studies(track_id).is_empty())
            .count();
        let projected_track_node_count = GLOBAL_MODIFIER_TRACK_IDS
            .iter()
            .map(|prefix| self.global_modifier_track_studies(prefix).len())
            .sum::<usize>();
        let raw_node_count = self
            .studies
            .len()
            .saturating_sub(projected_track_node_count);
        let and_junction_count = self
            .studies
            .iter()
            .filter(|study| study.prerequisites.len() >= 2)
            .count();
        let curated_junction_count = CURATED_CONVERGENCE_JUNCTION_IDS
            .iter()
            .filter(|id| {
                self.get(id)
                    .is_some_and(|study| study.prerequisites.len() >= 2)
            })
            .count();
        ResearchGraphTotals {
            raw_node_count,
            track_count,
            projected_node_count: self.studies.len(),
            and_junction_count,
            curated_junction_count,
        }
    }

    #[must_use]
    pub fn global_modifier_track_studies(&self, track_id: &str) -> Vec<&ManifestStudy> {
        let prefix = format!("{track_id}_");
        let mut studies = self
            .studies
            .iter()
            .filter(|study| study.stable_id.starts_with(&prefix) && study.stage.is_some())
            .collect::<Vec<_>>();
        studies.sort_by_key(|study| study.stage);
        studies
    }

    #[must_use]
    pub fn study_count(&self) -> usize {
        self.studies.len()
    }

    pub fn repeat_cost_units(
        &self,
        stable_id: &str,
        prior_terminal_completions: u32,
    ) -> Result<u64, ResearchManifestError> {
        let study = self
            .get(stable_id)
            .filter(|study| study.repeatable_terminal)
            .ok_or_else(|| ResearchManifestError::MalformedRepeatable(stable_id.to_owned()))?;
        let multiplier = 1_u64
            .checked_shl(prior_terminal_completions)
            .ok_or_else(|| ResearchManifestError::MalformedRepeatable(stable_id.to_owned()))?;
        study
            .cost_units
            .checked_mul(multiplier)
            .ok_or_else(|| ResearchManifestError::MalformedRepeatable(stable_id.to_owned()))
    }

    /// Canonical LAI.58 content validation.  It is intentionally separate from
    /// persistence validation: an integration owner may load an older catalog
    /// read-only, inspect the precise missing graph content, and reconcile it
    /// without making every save impossible to deserialize.
    pub fn validate_lai58_graph(&self) -> Result<ResearchGraphTotals, ResearchManifestError> {
        let totals = self.graph_totals();
        if totals.track_count != GLOBAL_MODIFIER_TRACK_IDS.len() {
            return Err(ResearchManifestError::WrongGlobalTrackCount {
                expected: GLOBAL_MODIFIER_TRACK_IDS.len(),
                actual: totals.track_count,
            });
        }
        for track_id in GLOBAL_MODIFIER_TRACK_IDS {
            let studies = self.global_modifier_track_studies(track_id);
            if studies.len() != usize::from(GLOBAL_MODIFIER_TERMINAL_LEVEL)
                || studies
                    .iter()
                    .take(usize::from(GLOBAL_MODIFIER_FINITE_LEVELS))
                    .enumerate()
                    .any(|(index, study)| {
                        study.stage != Some(u8::try_from(index + 1).expect("ten levels fit"))
                            || study.repeatable_terminal
                    })
                || !studies.last().is_some_and(|study| {
                    study.stage == Some(GLOBAL_MODIFIER_TERMINAL_LEVEL)
                        && study.repeatable_terminal
                        && study.cost_units
                            == studies[usize::from(GLOBAL_MODIFIER_FINITE_LEVELS) - 1]
                                .cost_units
                                .saturating_mul(2)
                })
            {
                return Err(ResearchManifestError::MalformedGlobalModifierTrack(
                    track_id.to_owned(),
                ));
            }
        }
        if totals.and_junction_count < MINIMUM_AND_JUNCTIONS {
            return Err(ResearchManifestError::InsufficientAndJunctions {
                minimum: MINIMUM_AND_JUNCTIONS,
                actual: totals.and_junction_count,
            });
        }
        if totals.curated_junction_count != CURATED_CONVERGENCE_JUNCTION_IDS.len() {
            return Err(ResearchManifestError::MissingCuratedJunctions {
                expected: CURATED_CONVERGENCE_JUNCTION_IDS.len(),
                actual: totals.curated_junction_count,
            });
        }
        Ok(totals)
    }

    /// Building studies grant only this permit.  They do not place, upgrade, or
    /// otherwise mutate a building; the Leader's physical construction path is
    /// the only consumer permitted to turn a permit into a site.
    #[must_use]
    pub fn building_permit_ids(&self, stable_id: &str) -> BTreeSet<&str> {
        self.get(stable_id)
            .into_iter()
            .flat_map(|study| study.effects.iter())
            .filter_map(|effect| match effect {
                ManifestEffect::CatalogPayload {
                    handler: ManifestEffectHandler::CatalogUnlockBuilding,
                    target_id,
                } => Some(target_id.as_str()),
                _ => None,
            })
            .collect()
    }

    fn validate_track_shape(&self) -> Result<(), ResearchManifestError> {
        for track in ManifestTrack::ALL {
            let studies = self.track_studies(track);
            if studies.len() != ADDITIVE_TRACK_STAGE_COUNT {
                return Err(ResearchManifestError::MalformedTrack {
                    track,
                    reason: "wrong stage count",
                });
            }
            for (index, study) in studies.iter().enumerate() {
                let expected_stage = u8::try_from(index + 1).expect("eleven stages fit u8");
                if study.stage != Some(expected_stage) {
                    return Err(ResearchManifestError::MalformedTrack {
                        track,
                        reason: "noncanonical stage order",
                    });
                }
                let expected_prerequisite = if expected_stage == 1 {
                    track.root_prerequisite().to_owned()
                } else {
                    additive_track_study_id(track, expected_stage - 1)
                };
                if study.prerequisites != [expected_prerequisite] {
                    return Err(ResearchManifestError::MalformedTrack {
                        track,
                        reason: "wrong prerequisite chain",
                    });
                }
                validate_track_effect(track, expected_stage, &study.effects)?;
            }
        }
        Ok(())
    }

    fn validate_topology(&self) -> Result<(), ResearchManifestError> {
        let mut indegree = vec![0_usize; self.studies.len()];
        let mut dependents = vec![Vec::new(); self.studies.len()];
        for (index, study) in self.studies.iter().enumerate() {
            let mut unique = BTreeSet::new();
            for prerequisite in &study.prerequisites {
                if is_obsolete_stable_id(prerequisite) {
                    return Err(ResearchManifestError::DeprecatedStudyReferenced {
                        study_id: study.stable_id.clone(),
                        deprecated_id: prerequisite.clone(),
                    });
                }
                if !unique.insert(prerequisite) {
                    return Err(ResearchManifestError::DuplicatePrerequisite {
                        study_id: study.stable_id.clone(),
                        prerequisite: prerequisite.clone(),
                    });
                }
                let Some(prerequisite_index) = self.by_id.get(prerequisite).copied() else {
                    return Err(ResearchManifestError::MissingPrerequisite {
                        study_id: study.stable_id.clone(),
                        prerequisite: prerequisite.clone(),
                    });
                };
                if prerequisite_index == index {
                    return Err(ResearchManifestError::SelfPrerequisite(
                        study.stable_id.clone(),
                    ));
                }
                indegree[index] += 1;
                dependents[prerequisite_index].push(index);
            }
        }

        let mut ready: BTreeSet<usize> = indegree
            .iter()
            .enumerate()
            .filter_map(|(index, degree)| (*degree == 0).then_some(index))
            .collect();
        let mut visited = 0;
        while let Some(index) = ready.pop_first() {
            visited += 1;
            for dependent in &dependents[index] {
                indegree[*dependent] -= 1;
                if indegree[*dependent] == 0 {
                    ready.insert(*dependent);
                }
            }
        }
        if visited != self.studies.len() {
            return Err(ResearchManifestError::CycleOrUnreachable {
                visited,
                total: self.studies.len(),
            });
        }

        let reachable = self.reachable_study_ids();
        if reachable.len() != self.studies.len() {
            return Err(ResearchManifestError::CycleOrUnreachable {
                visited: reachable.len(),
                total: self.studies.len(),
            });
        }
        Ok(())
    }
}

static RESEARCH_MANIFEST: OnceLock<ResearchManifest> = OnceLock::new();

#[must_use]
pub fn research_manifest() -> &'static ResearchManifest {
    RESEARCH_MANIFEST.get_or_init(|| {
        build_manifest()
            .unwrap_or_else(|error| panic!("embedded research manifest is invalid: {error}"))
    })
}

fn build_manifest() -> Result<ResearchManifest, ResearchManifestError> {
    let source = research_catalog();
    let by_source_id = source
        .nodes()
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let removed = source
        .nodes()
        .iter()
        .filter(|node| source_node_is_obsolete(node))
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut studies = Vec::new();
    for node in source
        .nodes()
        .iter()
        .filter(|node| !removed.contains(node.id.as_str()))
    {
        let mut study = ManifestStudy::from_catalog(studies.len(), node);
        let mut promoted = BTreeSet::new();
        for prerequisite in &node.prerequisites {
            promote_retained_prerequisites(
                prerequisite,
                &by_source_id,
                &removed,
                &mut BTreeSet::new(),
                &mut promoted,
            )?;
        }
        study.prerequisites = promoted.into_iter().collect();
        studies.push(study);
    }

    for (stable_id, display_name, prerequisites) in REQUIRED_CAPABILITY_FAMILIES {
        studies.push(capability_study(
            stable_id,
            display_name,
            split_prerequisites(prerequisites),
            studies.len(),
        ));
    }
    for (stable_id, display_name) in RARE_MATERIAL_CAPABILITIES {
        studies.push(capability_study(
            stable_id,
            display_name,
            vec!["hunting_lairs".to_owned(), "material_processing".to_owned()],
            studies.len(),
        ));
    }
    for (stable_id, display_name, prerequisites) in CURATED_CONVERGENCE_SPECS {
        if !studies.iter().any(|study| study.stable_id == *stable_id) {
            studies.push(capability_study(
                stable_id,
                display_name,
                split_prerequisites(prerequisites),
                studies.len(),
            ));
        }
    }

    for track in ManifestTrack::ALL {
        for stage in 1..=ADDITIVE_TRACK_STAGE_COUNT {
            let mut study =
                additive_track_study(track, u8::try_from(stage).expect("eleven stages fit u8"));
            study.order_index = studies.len();
            studies.push(study);
        }
    }

    mark_global_modifier_tracks(&mut studies)?;
    for junction_id in CURATED_CONVERGENCE_JUNCTION_IDS {
        let Some(junction) = studies
            .iter_mut()
            .find(|study| study.stable_id == junction_id)
        else {
            return Err(ResearchManifestError::MissingCuratedJunctions {
                expected: CURATED_CONVERGENCE_JUNCTION_IDS.len(),
                actual: 0,
            });
        };
        if junction.prerequisites.len() < 2 {
            junction.prerequisites.extend([
                "typed_food_handling".to_owned(),
                "governance_foundations".to_owned(),
            ]);
            junction.prerequisites.sort();
            junction.prerequisites.dedup();
        }
    }
    for (index, study) in studies.iter_mut().enumerate() {
        study.order_index = index;
    }
    let manifest = ResearchManifest::from_studies(studies)?;
    manifest.validate_lai58_graph()?;
    Ok(manifest)
}

fn source_node_is_obsolete(node: &ResearchNode) -> bool {
    is_obsolete_stable_id(&node.id)
        || node.payloads.iter().any(|payload| {
            let effect = catalog_effect(payload);
            matches!(
                effect,
                ManifestEffect::CatalogPayload { target_id, .. }
                    if is_obsolete_stable_id(&target_id)
            )
        })
}

fn is_obsolete_stable_id(stable_id: &str) -> bool {
    let normalized = stable_id.to_ascii_lowercase();
    DEPRECATED_STUDY_IDS.contains(&stable_id)
        || OBSOLETE_EXACT_IDENTITIES.contains(&stable_id)
        || OBSOLETE_STUDY_FRAGMENTS
            .iter()
            .any(|fragment| normalized.contains(fragment))
}

fn promote_retained_prerequisites(
    stable_id: &str,
    source: &BTreeMap<&str, &ResearchNode>,
    removed: &BTreeSet<&str>,
    visiting: &mut BTreeSet<String>,
    promoted: &mut BTreeSet<String>,
) -> Result<(), ResearchManifestError> {
    if !removed.contains(stable_id) {
        promoted.insert(stable_id.to_owned());
        return Ok(());
    }
    if !visiting.insert(stable_id.to_owned()) {
        return Err(ResearchManifestError::CycleOrUnreachable {
            visited: 0,
            total: source.len(),
        });
    }
    let node = source
        .get(stable_id)
        .ok_or_else(|| ResearchManifestError::MissingPrerequisite {
            study_id: stable_id.to_owned(),
            prerequisite: stable_id.to_owned(),
        })?;
    for prerequisite in &node.prerequisites {
        promote_retained_prerequisites(prerequisite, source, removed, visiting, promoted)?;
    }
    visiting.remove(stable_id);
    Ok(())
}

fn split_prerequisites(csv: &str) -> Vec<String> {
    csv.split(',')
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn capability_study(
    stable_id: &str,
    display_name: &str,
    prerequisites: Vec<String>,
    order_index: usize,
) -> ManifestStudy {
    let is_building_permit = matches!(
        stable_id,
        "cookhouse" | "fishing_hut" | "family_homes" | "elder_lodge" | "nursery"
    );
    let handler = if is_building_permit {
        ManifestEffectHandler::CatalogUnlockBuilding
    } else {
        ManifestEffectHandler::CatalogUnlockCapability
    };
    let target_id = if stable_id == "family_homes" {
        "family_home"
    } else {
        stable_id
    };
    ManifestStudy {
        stable_id: stable_id.to_owned(),
        display_name: display_name.to_owned(),
        prerequisites,
        source: ManifestStudySource::CurrentCatalog,
        stage: None,
        cost_units: 1,
        repeatable_terminal: false,
        order_index,
        effects: vec![ManifestEffect::CatalogPayload {
            handler,
            target_id: target_id.to_owned(),
        }],
    }
}

const GLOBAL_MODIFIER_STAGE_SUFFIXES: [&str; 11] = [
    "basics",
    "coordination",
    "standards",
    "instruments",
    "training",
    "networks",
    "specialization",
    "optimization",
    "resilience",
    "excellence",
    "mastery",
];

fn mark_global_modifier_tracks(studies: &mut [ManifestStudy]) -> Result<(), ResearchManifestError> {
    for track_id in GLOBAL_MODIFIER_TRACK_IDS {
        let mut finite_final_cost = None;
        for (index, suffix) in GLOBAL_MODIFIER_STAGE_SUFFIXES.iter().enumerate() {
            let stable_id = format!("{track_id}_{suffix}");
            let study = studies
                .iter_mut()
                .find(|study| study.stable_id == stable_id)
                .ok_or_else(|| {
                    ResearchManifestError::MalformedGlobalModifierTrack(track_id.to_owned())
                })?;
            let stage = u8::try_from(index + 1).expect("eleven levels fit u8");
            study.stage = Some(stage);
            study.repeatable_terminal = stage == GLOBAL_MODIFIER_TERMINAL_LEVEL;
            if stage == GLOBAL_MODIFIER_FINITE_LEVELS {
                finite_final_cost = Some(study.cost_units);
            } else if stage == GLOBAL_MODIFIER_TERMINAL_LEVEL {
                study.cost_units = finite_final_cost
                    .ok_or_else(|| {
                        ResearchManifestError::MalformedGlobalModifierTrack(track_id.to_owned())
                    })?
                    .saturating_mul(2);
            }
        }
    }
    Ok(())
}

fn additive_track_study(track: ManifestTrack, stage: u8) -> ManifestStudy {
    let stable_id = additive_track_study_id(track, stage);
    let prerequisites = if stage == 1 {
        vec![track.root_prerequisite().to_owned()]
    } else {
        vec![additive_track_study_id(track, stage - 1)]
    };
    ManifestStudy {
        stable_id,
        display_name: format!("{} Stage {stage:02}", track.display_name()),
        prerequisites,
        source: ManifestStudySource::AdditiveTrack(track),
        stage: Some(stage),
        cost_units: u64::from(stage),
        repeatable_terminal: false,
        order_index: 0,
        effects: vec![track_effect(track, stage)],
    }
}

fn additive_track_study_id(track: ManifestTrack, stage: u8) -> String {
    format!("{}_stage_{stage:02}", track.stable_prefix())
}

fn track_effect(track: ManifestTrack, stage: u8) -> ManifestEffect {
    let index = usize::from(stage - 1);
    match track {
        ManifestTrack::DivineDuration => ManifestEffect::DivineDuration {
            stage,
            max_duration_game_hours: DIVINE_DURATION_STAGE_MAX_GAME_HOURS[index],
        },
        ManifestTrack::DivineEconomy => ManifestEffect::DivineEconomy {
            stage,
            discount_basis_points: DIVINE_ECONOMY_STAGE_DISCOUNT_BASIS_POINTS[index],
        },
        ManifestTrack::Rehabilitation => ManifestEffect::Rehabilitation {
            stage,
            restoration_bonus_percentage_points: REHABILITATION_STAGE_RESTORATION_PERCENTAGE_POINTS
                [index],
        },
        ManifestTrack::Administration => ManifestEffect::Administration {
            stage,
            standing_order_slots: ADMINISTRATION_STAGE_STANDING_ORDER_SLOTS[index],
            strategic_intent_slots: ADMINISTRATION_STAGE_STRATEGIC_INTENT_SLOTS[index],
        },
    }
}

fn validate_track_effect(
    track: ManifestTrack,
    stage: u8,
    effects: &[ManifestEffect],
) -> Result<(), ResearchManifestError> {
    if effects != [track_effect(track, stage)] {
        return Err(ResearchManifestError::MalformedTrack {
            track,
            reason: "wrong stage effect",
        });
    }
    Ok(())
}

fn catalog_effect(payload: &ResearchPayload) -> ManifestEffect {
    let (handler, target_id) = match payload {
        ResearchPayload::BuildingAvailableAtFounding { building_id } => (
            ManifestEffectHandler::CatalogBuildingAvailableAtFounding,
            building_id.clone(),
        ),
        ResearchPayload::UnlockBuilding { building_id } => (
            ManifestEffectHandler::CatalogUnlockBuilding,
            building_id.clone(),
        ),
        ResearchPayload::UnlockRecipe { recipe_id } => (
            ManifestEffectHandler::CatalogUnlockRecipe,
            recipe_id.clone(),
        ),
        ResearchPayload::UnlockResource { resource_id } => (
            ManifestEffectHandler::CatalogUnlockResource,
            resource_id.clone(),
        ),
        ResearchPayload::UnlockJob { job_id } => {
            (ManifestEffectHandler::CatalogUnlockJob, job_id.clone())
        }
        ResearchPayload::ModifyBuilding { building_id, .. } => (
            ManifestEffectHandler::CatalogModifyBuilding,
            building_id.clone(),
        ),
        ResearchPayload::Modify { effect_id, .. } => (
            ManifestEffectHandler::CatalogModifyEffect,
            effect_id.clone(),
        ),
        ResearchPayload::UnlockCapability { capability_id } => (
            ManifestEffectHandler::CatalogUnlockCapability,
            capability_id.clone(),
        ),
    };
    ManifestEffect::CatalogPayload { handler, target_id }
}

fn validate_study_identity(study: &ManifestStudy) -> Result<(), ResearchManifestError> {
    if study.stable_id.trim().is_empty() || study.display_name.trim().is_empty() {
        return Err(ResearchManifestError::BlankStudyIdentity);
    }
    if study
        .stage
        .is_some_and(|stage| stage == 0 || usize::from(stage) > ADDITIVE_TRACK_STAGE_COUNT)
    {
        return Err(ResearchManifestError::InvalidStage {
            study_id: study.stable_id.clone(),
        });
    }
    Ok(())
}

/// Derived, presentation-safe counts for the one research graph.  Historical
/// migration counts are intentionally absent: callers must ask the loaded
/// catalog rather than accidentally treating 495/88/228/531 as live rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResearchGraphTotals {
    pub raw_node_count: usize,
    pub track_count: usize,
    pub projected_node_count: usize,
    pub and_junction_count: usize,
    pub curated_junction_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResearchManifestError {
    UnsupportedSchemaVersion,
    EmptyManifest,
    WrongGlobalTrackCount {
        expected: usize,
        actual: usize,
    },
    InsufficientAndJunctions {
        minimum: usize,
        actual: usize,
    },
    MissingCuratedJunctions {
        expected: usize,
        actual: usize,
    },
    DuplicateStableId(String),
    DuplicateDisplayName(String),
    DuplicateOrderIndex(usize),
    DuplicatePrerequisite {
        study_id: String,
        prerequisite: String,
    },
    MissingPrerequisite {
        study_id: String,
        prerequisite: String,
    },
    SelfPrerequisite(String),
    CycleOrUnreachable {
        visited: usize,
        total: usize,
    },
    BlankStudyIdentity,
    InvalidStage {
        study_id: String,
    },
    StudyHasNoEffect(String),
    UnknownEffectHandler {
        study_id: String,
        handler: ManifestEffectHandler,
    },
    DeprecatedStudyPresent(String),
    DeprecatedStudyReferenced {
        study_id: String,
        deprecated_id: String,
    },
    MalformedTrack {
        track: ManifestTrack,
        reason: &'static str,
    },
    MalformedGlobalModifierTrack(String),
    MalformedRepeatable(String),
}

impl fmt::Display for ResearchManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid research manifest: {self:?}")
    }
}

impl std::error::Error for ResearchManifestError {}
