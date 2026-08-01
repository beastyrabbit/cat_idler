//! Canonical immutable construction blueprints for LAI.59.
//!
//! This is deliberately only the bill/visual/permit catalog consumed later by
//! [`crate::construction_stages`]. It owns no project state, reservations,
//! hauling, workers, renderer, protocol, or persistence adapter. Hole upgrades
//! remain exclusively in [`crate::black_hole::upgrade_bill`].

use std::collections::BTreeSet;

use crate::{
    construction_stages::{
        ConstructionBills, ConstructionCargoLine, ConstructionStageBill, ConstructionTargetKind,
        ScaffoldTier, building_upgrade_duration_ms,
    },
    content_manifest::{ArtKey, ContentManifest},
    spatial_tasks::footprint_for,
    types::BuildingType,
};

/// One game-hour in milliseconds. Kept here because level-one construction has
/// a catalog-owned base duration rather than an upgrade duration.
pub const GAME_HOUR_MS: u64 = 60 * 60 * 1_000;

const BASIC_HOME_DURATION_MS: u64 = 4 * GAME_HOUR_MS;
const BASIC_SITE_DURATION_MS: u64 = 5 * GAME_HOUR_MS;
const DEVELOPED_BUILDING_DURATION_MS: u64 = 8 * GAME_HOUR_MS;

const LOGS: &str = "resource_logs";
const LUMBER: &str = "resource_lumber";
const PLANKS: &str = "resource_planks";
const STONE: &str = "resource_stone";
const BLOCKS: &str = "resource_blocks";
const CLOTH: &str = "resource_cloth";
const REFINED: &str = "resource_refined";
const METAL: &str = "resource_metal";
const GEMS: &str = "resource_gem";
const FURNITURE: &str = "item_furniture";
const TOOL: &str = "item_generic_tool";
const BOWL: &str = "item_bowl";
const STORAGE_FIXTURE: &str = "fixture_storage";
const WORKSHOP_FIXTURE: &str = "fixture_workshop";
const RESEARCH_FIXTURE: &str = "fixture_research";

/// The deliberately closed source of a construction request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueprintRequest {
    NewBuilding(BuildingType),
    BuildingUpgrade {
        building_type: BuildingType,
        target_level: u8,
    },
    /// Explicitly rejected here; `black_hole::upgrade_bill` is the only owner.
    HoleUpgrade,
}

/// A catalog-owned requirement. It is intentionally distinct from mutable
/// `ConstructionCargoLine`: there is no delivered, in-transit, or consumed
/// state in this immutable recipe shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlueprintRequirement {
    pub content_id: &'static str,
    pub units: u32,
}

/// A non-empty immutable stage recipe in exact ascending content-ID order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintStage {
    requirements: Vec<BlueprintRequirement>,
}

impl BlueprintStage {
    #[must_use]
    pub fn requirements(&self) -> &[BlueprintRequirement] {
        &self.requirements
    }
}

#[derive(Debug, Clone, Copy)]
struct BlueprintStageTemplate {
    requirements: &'static [BlueprintRequirement],
}

/// Exact rectangular footprint dimensions. Tiles are derived later from the
/// authoritative spatial table and an actual anchor; the catalog never invents
/// a fallback footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintFootprint {
    pub width: i32,
    pub height: i32,
}

/// Render data for the three persisted work phases and their inspector labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlueprintPhasePresentation {
    pub scaffold_art_key: &'static str,
    pub structure_art_key: &'static str,
    pub fit_out_art_key: &'static str,
    pub inspector_label: &'static str,
}

/// The canonical, immutable result for one building+level operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructionBlueprint {
    pub id: &'static str,
    pub target_kind: ConstructionTargetKind,
    pub building_type: BuildingType,
    pub target_level: u8,
    pub scaffold_tier: ScaffoldTier,
    pub footprint: BlueprintFootprint,
    pub scaffold: BlueprintStage,
    pub structure: BlueprintStage,
    pub fit_out: BlueprintStage,
    pub base_work_duration_ms: u64,
    pub permit_capability_id: Option<&'static str>,
    pub presentation: BlueprintPhasePresentation,
}

impl ConstructionBlueprint {
    /// Convert the immutable bill into fresh project cargo. Every mutable
    /// counter starts at zero; callers cannot accidentally reuse a prior
    /// project's delivery state.
    #[must_use]
    pub fn fresh_bills(&self) -> ConstructionBills {
        ConstructionBills {
            scaffold: stage_bill(&self.scaffold),
            structure: stage_bill(&self.structure),
            fit_out: stage_bill(&self.fit_out),
        }
    }
}

fn stage_bill(stage: &BlueprintStage) -> ConstructionStageBill {
    ConstructionStageBill::new(
        stage.requirements.iter().map(|requirement| {
            ConstructionCargoLine::new(requirement.content_id, requirement.units)
        }),
    )
}

/// An explicit disposition for every legacy `BuildingType`; no forgotten
/// variant silently receives generic construction or generic scalar food.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildingCatalogDisposition {
    Cataloged,
    /// The legacy Shrine/Favor loop is retired and must fail closed.
    Retired {
        reason: &'static str,
    },
    /// This is physical work, but owned by another canonical system.
    Delegated {
        owner: &'static str,
    },
}

#[must_use]
pub const fn building_disposition(building_type: BuildingType) -> BuildingCatalogDisposition {
    match building_type {
        BuildingType::Shrine => BuildingCatalogDisposition::Retired {
            reason: "retired_shrine_favor_authority",
        },
        BuildingType::Walls => BuildingCatalogDisposition::Delegated {
            owner: "village_infrastructure",
        },
        BuildingType::Field => BuildingCatalogDisposition::Delegated {
            owner: "village_infrastructure",
        },
        // The scalar mouse-food loop is intentionally not translated into a
        // typed-food construction recipe. Typed farms/food stay downstream.
        BuildingType::MouseFarm => BuildingCatalogDisposition::Retired {
            reason: "retired_generic_scalar_food",
        },
        BuildingType::Den
        | BuildingType::FoodStorage
        | BuildingType::WaterBowl
        | BuildingType::Beds
        | BuildingType::HerbGarden
        | BuildingType::Nursery
        | BuildingType::ElderCorner
        | BuildingType::FamilyHome
        | BuildingType::ElderLodge
        | BuildingType::Workshop
        | BuildingType::Smithy
        | BuildingType::Barracks
        | BuildingType::AccountingTent
        | BuildingType::WoodCutter
        | BuildingType::StonePrep
        | BuildingType::Woodworking
        | BuildingType::Clothier
        | BuildingType::Tannery
        | BuildingType::ResearchHut
        | BuildingType::Smelter
        | BuildingType::Mill
        | BuildingType::Sawmill
        | BuildingType::School
        // LAI.46: the two Plan 1 stations are ordinary cataloged developed
        // buildings so staged construction can actually materialize them.
        | BuildingType::Cookhouse
        | BuildingType::FishingHut => BuildingCatalogDisposition::Cataloged,
    }
}

/// Resolution errors are intentional domain outcomes, never a generic fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlueprintLookupError {
    RetiredBuilding {
        building_type: BuildingType,
        reason: &'static str,
    },
    DelegatedBuilding {
        building_type: BuildingType,
        owner: &'static str,
    },
    HoleUpgradeDelegated {
        owner: &'static str,
    },
    InvalidUpgradeLevel(u8),
    MissingBlueprint,
}

/// Resolve one deterministic blueprint. New buildings are level one only;
/// upgrades are level two through ten only.
pub fn resolve_blueprint(
    request: BlueprintRequest,
) -> Result<ConstructionBlueprint, BlueprintLookupError> {
    let (building_type, target_level, target_kind) = match request {
        BlueprintRequest::NewBuilding(building_type) => {
            (building_type, 1, ConstructionTargetKind::Building)
        }
        BlueprintRequest::BuildingUpgrade {
            building_type,
            target_level,
        } => {
            if !(2..=10).contains(&target_level) {
                return Err(BlueprintLookupError::InvalidUpgradeLevel(target_level));
            }
            (
                building_type,
                target_level,
                ConstructionTargetKind::BuildingUpgrade,
            )
        }
        BlueprintRequest::HoleUpgrade => {
            return Err(BlueprintLookupError::HoleUpgradeDelegated {
                owner: "black_hole::upgrade_bill",
            });
        }
    };
    match building_disposition(building_type) {
        BuildingCatalogDisposition::Cataloged => {}
        BuildingCatalogDisposition::Retired { reason } => {
            return Err(BlueprintLookupError::RetiredBuilding {
                building_type,
                reason,
            });
        }
        BuildingCatalogDisposition::Delegated { owner } => {
            return Err(BlueprintLookupError::DelegatedBuilding {
                building_type,
                owner,
            });
        }
    }
    blueprint_for(building_type, target_level, target_kind)
        .ok_or(BlueprintLookupError::MissingBlueprint)
}

/// Materialize every cataloged building's level-one blueprint followed by its
/// level 2–10 upgrades, in `BuildingType::ALL` order. This is the sole catalog
/// ordering and is therefore restart/order stable.
#[must_use]
pub fn canonical_blueprints() -> Vec<ConstructionBlueprint> {
    let mut blueprints = Vec::new();
    for &building_type in BuildingType::ALL {
        if !matches!(
            building_disposition(building_type),
            BuildingCatalogDisposition::Cataloged
        ) {
            continue;
        }
        blueprints.push(
            blueprint_for(building_type, 1, ConstructionTargetKind::Building)
                .expect("cataloged level-one building must have a blueprint"),
        );
        for target_level in 2..=10 {
            blueprints.push(
                blueprint_for(
                    building_type,
                    target_level,
                    ConstructionTargetKind::BuildingUpgrade,
                )
                .expect("cataloged upgrade level must have a blueprint"),
            );
        }
    }
    blueprints
}

fn blueprint_for(
    building_type: BuildingType,
    target_level: u8,
    target_kind: ConstructionTargetKind,
) -> Option<ConstructionBlueprint> {
    let profile = profile_for(building_type)?;
    let (width, height) = footprint_for(building_type);
    let (scaffold_tier, scaffold, structure, fit_out, duration) = if target_level == 1 {
        (
            profile.scaffold_tier,
            stage_from_template(profile.new_scaffold),
            stage_from_template(profile.new_structure),
            stage_from_template(profile.new_fit_out),
            profile.level_one_duration_ms,
        )
    } else {
        let scale = u32::from(target_level);
        (
            ScaffoldTier::Developed,
            upgrade_scaffold(scale),
            upgrade_structure(scale),
            upgrade_fit_out(profile, scale),
            building_upgrade_duration_ms(u32::from(target_level))?,
        )
    };
    let id = blueprint_id(building_type, target_level, target_kind)?;
    Some(ConstructionBlueprint {
        id,
        target_kind,
        building_type,
        target_level,
        scaffold_tier,
        footprint: BlueprintFootprint { width, height },
        scaffold,
        structure,
        fit_out,
        base_work_duration_ms: duration,
        permit_capability_id: profile.permit_capability_id,
        presentation: profile.presentation,
    })
}

fn stage_from_template(template: BlueprintStageTemplate) -> BlueprintStage {
    BlueprintStage {
        requirements: template.requirements.to_vec(),
    }
}

fn blueprint_id(
    building_type: BuildingType,
    target_level: u8,
    target_kind: ConstructionTargetKind,
) -> Option<&'static str> {
    let prefix = match target_kind {
        ConstructionTargetKind::Building if target_level == 1 => "new",
        ConstructionTargetKind::BuildingUpgrade if (2..=10).contains(&target_level) => "upgrade",
        _ => return None,
    };
    // IDs are a closed table so they remain &'static and do not introduce an
    // allocation-derived identity into persisted callers.
    Some(match (building_type, prefix, target_level) {
        (BuildingType::Den, "new", 1) => "construction_den_new_l01",
        (BuildingType::FoodStorage, "new", 1) => "construction_food_storage_new_l01",
        (BuildingType::WaterBowl, "new", 1) => "construction_water_bowl_new_l01",
        (BuildingType::Beds, "new", 1) => "construction_beds_new_l01",
        (BuildingType::HerbGarden, "new", 1) => "construction_herb_garden_new_l01",
        (BuildingType::Nursery, "new", 1) => "construction_nursery_new_l01",
        (BuildingType::ElderCorner, "new", 1) => "construction_elder_corner_new_l01",
        (BuildingType::FamilyHome, "new", 1) => "construction_family_home_new_l01",
        (BuildingType::ElderLodge, "new", 1) => "construction_elder_lodge_new_l01",
        (BuildingType::Workshop, "new", 1) => "construction_workshop_new_l01",
        (BuildingType::Smithy, "new", 1) => "construction_smithy_new_l01",
        (BuildingType::Barracks, "new", 1) => "construction_barracks_new_l01",
        (BuildingType::AccountingTent, "new", 1) => "construction_accounting_tent_new_l01",
        (BuildingType::WoodCutter, "new", 1) => "construction_wood_cutter_new_l01",
        (BuildingType::StonePrep, "new", 1) => "construction_stone_prep_new_l01",
        (BuildingType::Woodworking, "new", 1) => "construction_woodworking_new_l01",
        (BuildingType::Clothier, "new", 1) => "construction_clothier_new_l01",
        (BuildingType::Tannery, "new", 1) => "construction_tannery_new_l01",
        (BuildingType::ResearchHut, "new", 1) => "construction_research_hut_new_l01",
        (BuildingType::Smelter, "new", 1) => "construction_smelter_new_l01",
        (BuildingType::Mill, "new", 1) => "construction_mill_new_l01",
        (BuildingType::Sawmill, "new", 1) => "construction_sawmill_new_l01",
        (BuildingType::School, "new", 1) => "construction_school_new_l01",
        (BuildingType::Cookhouse, "new", 1) => "construction_cookhouse_new_l01",
        (BuildingType::FishingHut, "new", 1) => "construction_fishing_hut_new_l01",
        (building_type, "upgrade", level @ 2..=10) => upgrade_id(building_type, level)?,
        _ => return None,
    })
}

fn upgrade_id(building_type: BuildingType, level: u8) -> Option<&'static str> {
    let ids = match building_type {
        BuildingType::Den => DEN_UPGRADES,
        BuildingType::FoodStorage => FOOD_STORAGE_UPGRADES,
        BuildingType::WaterBowl => WATER_BOWL_UPGRADES,
        BuildingType::Beds => BEDS_UPGRADES,
        BuildingType::HerbGarden => HERB_GARDEN_UPGRADES,
        BuildingType::Nursery => NURSERY_UPGRADES,
        BuildingType::ElderCorner => ELDER_CORNER_UPGRADES,
        BuildingType::FamilyHome => FAMILY_HOME_UPGRADES,
        BuildingType::ElderLodge => ELDER_LODGE_UPGRADES,
        BuildingType::Workshop => WORKSHOP_UPGRADES,
        BuildingType::Smithy => SMITHY_UPGRADES,
        BuildingType::Barracks => BARRACKS_UPGRADES,
        BuildingType::AccountingTent => ACCOUNTING_TENT_UPGRADES,
        BuildingType::WoodCutter => WOOD_CUTTER_UPGRADES,
        BuildingType::StonePrep => STONE_PREP_UPGRADES,
        BuildingType::Woodworking => WOODWORKING_UPGRADES,
        BuildingType::Clothier => CLOTHIER_UPGRADES,
        BuildingType::Tannery => TANNERY_UPGRADES,
        BuildingType::ResearchHut => RESEARCH_HUT_UPGRADES,
        BuildingType::Smelter => SMELTER_UPGRADES,
        BuildingType::Mill => MILL_UPGRADES,
        BuildingType::Sawmill => SAWMILL_UPGRADES,
        BuildingType::School => SCHOOL_UPGRADES,
        BuildingType::Cookhouse => COOKHOUSE_UPGRADES,
        BuildingType::FishingHut => FISHING_HUT_UPGRADES,
        BuildingType::Shrine
        | BuildingType::Walls
        | BuildingType::MouseFarm
        | BuildingType::Field => {
            return None;
        }
    };
    ids.get(usize::from(level.checked_sub(2)?)).copied()
}

macro_rules! upgrade_ids {
    ($name:ident, $building:literal) => {
        const $name: [&str; 9] = [
            concat!("construction_", $building, "_upgrade_l02"),
            concat!("construction_", $building, "_upgrade_l03"),
            concat!("construction_", $building, "_upgrade_l04"),
            concat!("construction_", $building, "_upgrade_l05"),
            concat!("construction_", $building, "_upgrade_l06"),
            concat!("construction_", $building, "_upgrade_l07"),
            concat!("construction_", $building, "_upgrade_l08"),
            concat!("construction_", $building, "_upgrade_l09"),
            concat!("construction_", $building, "_upgrade_l10"),
        ];
    };
}

upgrade_ids!(DEN_UPGRADES, "den");
upgrade_ids!(FOOD_STORAGE_UPGRADES, "food_storage");
upgrade_ids!(WATER_BOWL_UPGRADES, "water_bowl");
upgrade_ids!(BEDS_UPGRADES, "beds");
upgrade_ids!(HERB_GARDEN_UPGRADES, "herb_garden");
upgrade_ids!(NURSERY_UPGRADES, "nursery");
upgrade_ids!(ELDER_CORNER_UPGRADES, "elder_corner");
upgrade_ids!(FAMILY_HOME_UPGRADES, "family_home");
upgrade_ids!(ELDER_LODGE_UPGRADES, "elder_lodge");
upgrade_ids!(WORKSHOP_UPGRADES, "workshop");
upgrade_ids!(SMITHY_UPGRADES, "smithy");
upgrade_ids!(BARRACKS_UPGRADES, "barracks");
upgrade_ids!(ACCOUNTING_TENT_UPGRADES, "accounting_tent");
upgrade_ids!(WOOD_CUTTER_UPGRADES, "wood_cutter");
upgrade_ids!(STONE_PREP_UPGRADES, "stone_prep");
upgrade_ids!(WOODWORKING_UPGRADES, "woodworking");
upgrade_ids!(CLOTHIER_UPGRADES, "clothier");
upgrade_ids!(TANNERY_UPGRADES, "tannery");
upgrade_ids!(RESEARCH_HUT_UPGRADES, "research_hut");
upgrade_ids!(SMELTER_UPGRADES, "smelter");
upgrade_ids!(MILL_UPGRADES, "mill");
upgrade_ids!(SAWMILL_UPGRADES, "sawmill");
upgrade_ids!(SCHOOL_UPGRADES, "school");
upgrade_ids!(COOKHOUSE_UPGRADES, "cookhouse");
upgrade_ids!(FISHING_HUT_UPGRADES, "fishing_hut");

#[derive(Debug, Clone, Copy)]
struct BlueprintProfile {
    scaffold_tier: ScaffoldTier,
    level_one_duration_ms: u64,
    new_scaffold: BlueprintStageTemplate,
    new_structure: BlueprintStageTemplate,
    new_fit_out: BlueprintStageTemplate,
    permit_capability_id: Option<&'static str>,
    presentation: BlueprintPhasePresentation,
    advanced: bool,
}

const BASIC_SCAFFOLD: BlueprintStageTemplate = BlueprintStageTemplate {
    requirements: &[BlueprintRequirement {
        content_id: LOGS,
        units: 4,
    }],
};
const BASIC_STRUCTURE: BlueprintStageTemplate = BlueprintStageTemplate {
    requirements: &[
        BlueprintRequirement {
            content_id: LOGS,
            units: 8,
        },
        BlueprintRequirement {
            content_id: STONE,
            units: 4,
        },
    ],
};
const BASIC_HOME_FIT_OUT: BlueprintStageTemplate = BlueprintStageTemplate {
    // Cloth is bedding; furniture is the canonical woodwork item. There is no
    // invented `wood` or `bedding` resource alias.
    requirements: &[
        BlueprintRequirement {
            content_id: FURNITURE,
            units: 1,
        },
        BlueprintRequirement {
            content_id: CLOTH,
            units: 2,
        },
    ],
};
const BASIC_SITE_FIT_OUT: BlueprintStageTemplate = BlueprintStageTemplate {
    requirements: &[
        BlueprintRequirement {
            content_id: FURNITURE,
            units: 1,
        },
        BlueprintRequirement {
            content_id: CLOTH,
            units: 1,
        },
    ],
};
const BOWL_FIT_OUT: BlueprintStageTemplate = BlueprintStageTemplate {
    requirements: &[
        BlueprintRequirement {
            content_id: BOWL,
            units: 1,
        },
        BlueprintRequirement {
            content_id: CLOTH,
            units: 1,
        },
    ],
};
const DEVELOPED_SCAFFOLD: BlueprintStageTemplate = BlueprintStageTemplate {
    requirements: &[BlueprintRequirement {
        content_id: LUMBER,
        units: 8,
    }],
};
const DEVELOPED_STRUCTURE: BlueprintStageTemplate = BlueprintStageTemplate {
    requirements: &[
        BlueprintRequirement {
            content_id: BLOCKS,
            units: 6,
        },
        BlueprintRequirement {
            content_id: PLANKS,
            units: 12,
        },
        BlueprintRequirement {
            content_id: REFINED,
            units: 4,
        },
    ],
};
const DEVELOPED_FIT_OUT: BlueprintStageTemplate = BlueprintStageTemplate {
    requirements: &[
        BlueprintRequirement {
            content_id: FURNITURE,
            units: 1,
        },
        BlueprintRequirement {
            content_id: TOOL,
            units: 1,
        },
    ],
};
const STORAGE_FIT_OUT: BlueprintStageTemplate = BlueprintStageTemplate {
    requirements: &[
        BlueprintRequirement {
            content_id: STORAGE_FIXTURE,
            units: 1,
        },
        BlueprintRequirement {
            content_id: TOOL,
            units: 1,
        },
    ],
};
const WORKSHOP_FIT_OUT: BlueprintStageTemplate = BlueprintStageTemplate {
    requirements: &[
        BlueprintRequirement {
            content_id: WORKSHOP_FIXTURE,
            units: 1,
        },
        BlueprintRequirement {
            content_id: TOOL,
            units: 1,
        },
    ],
};
const RESEARCH_FIT_OUT: BlueprintStageTemplate = BlueprintStageTemplate {
    requirements: &[
        BlueprintRequirement {
            content_id: RESEARCH_FIXTURE,
            units: 1,
        },
        BlueprintRequirement {
            content_id: TOOL,
            units: 1,
        },
    ],
};

const BASIC_PRESENTATION: BlueprintPhasePresentation = BlueprintPhasePresentation {
    scaffold_art_key: "art_resource_logs",
    structure_art_key: "art_resource_logs",
    fit_out_art_key: "art_item_furniture",
    inspector_label: "Basic shelter construction",
};
const DEVELOPED_PRESENTATION: BlueprintPhasePresentation = BlueprintPhasePresentation {
    scaffold_art_key: "art_resource_lumber",
    structure_art_key: "art_resource_refined",
    fit_out_art_key: "art_item_generic_tool",
    inspector_label: "Developed building construction",
};
const WORKSHOP_PRESENTATION: BlueprintPhasePresentation = BlueprintPhasePresentation {
    scaffold_art_key: "art_resource_lumber",
    structure_art_key: "art_resource_refined",
    fit_out_art_key: "art_fixture_workshop",
    inspector_label: "Workshop construction",
};
const RESEARCH_PRESENTATION: BlueprintPhasePresentation = BlueprintPhasePresentation {
    scaffold_art_key: "art_resource_lumber",
    structure_art_key: "art_resource_refined",
    fit_out_art_key: "art_fixture_research",
    inspector_label: "Research building construction",
};

const fn basic_profile(home: bool) -> BlueprintProfile {
    BlueprintProfile {
        scaffold_tier: ScaffoldTier::Basic,
        level_one_duration_ms: if home {
            BASIC_HOME_DURATION_MS
        } else {
            BASIC_SITE_DURATION_MS
        },
        new_scaffold: BASIC_SCAFFOLD,
        new_structure: BASIC_STRUCTURE,
        new_fit_out: if home {
            BASIC_HOME_FIT_OUT
        } else {
            BASIC_SITE_FIT_OUT
        },
        permit_capability_id: None,
        presentation: BASIC_PRESENTATION,
        advanced: false,
    }
}

const fn developed_profile(
    permit_capability_id: Option<&'static str>,
    fit_out: BlueprintStageTemplate,
    presentation: BlueprintPhasePresentation,
    advanced: bool,
) -> BlueprintProfile {
    BlueprintProfile {
        scaffold_tier: ScaffoldTier::Developed,
        level_one_duration_ms: DEVELOPED_BUILDING_DURATION_MS,
        new_scaffold: DEVELOPED_SCAFFOLD,
        new_structure: DEVELOPED_STRUCTURE,
        new_fit_out: fit_out,
        permit_capability_id,
        presentation,
        advanced,
    }
}

fn profile_for(building_type: BuildingType) -> Option<BlueprintProfile> {
    Some(match building_type {
        BuildingType::Den | BuildingType::Beds | BuildingType::ElderCorner => basic_profile(true),
        BuildingType::Nursery => BlueprintProfile {
            permit_capability_id: Some("nursery"),
            ..basic_profile(true)
        },
        BuildingType::FamilyHome => BlueprintProfile {
            permit_capability_id: Some("family_home"),
            ..basic_profile(true)
        },
        BuildingType::ElderLodge => BlueprintProfile {
            permit_capability_id: Some("elder_lodge"),
            ..basic_profile(true)
        },
        BuildingType::WaterBowl => BlueprintProfile {
            new_fit_out: BOWL_FIT_OUT,
            ..basic_profile(false)
        },
        BuildingType::HerbGarden => basic_profile(false),
        BuildingType::FoodStorage => developed_profile(
            Some("storage_containers"),
            STORAGE_FIT_OUT,
            DEVELOPED_PRESENTATION,
            true,
        ),
        BuildingType::Workshop => developed_profile(
            Some("workshop"),
            WORKSHOP_FIT_OUT,
            WORKSHOP_PRESENTATION,
            true,
        ),
        BuildingType::ResearchHut | BuildingType::School => developed_profile(
            Some(if matches!(building_type, BuildingType::School) {
                "school"
            } else {
                "research_hut"
            }),
            RESEARCH_FIT_OUT,
            RESEARCH_PRESENTATION,
            true,
        ),
        BuildingType::Smithy => developed_profile(
            Some("smithy"),
            WORKSHOP_FIT_OUT,
            WORKSHOP_PRESENTATION,
            true,
        ),
        BuildingType::WoodCutter => developed_profile(
            Some("plank_processing"),
            DEVELOPED_FIT_OUT,
            DEVELOPED_PRESENTATION,
            true,
        ),
        BuildingType::StonePrep => developed_profile(
            Some("material_processing"),
            DEVELOPED_FIT_OUT,
            DEVELOPED_PRESENTATION,
            true,
        ),
        BuildingType::Woodworking => developed_profile(
            Some("woodworking"),
            WORKSHOP_FIT_OUT,
            WORKSHOP_PRESENTATION,
            true,
        ),
        BuildingType::Clothier => developed_profile(
            Some("clothier"),
            DEVELOPED_FIT_OUT,
            DEVELOPED_PRESENTATION,
            true,
        ),
        BuildingType::Tannery => developed_profile(
            Some("tannery_processing"),
            DEVELOPED_FIT_OUT,
            DEVELOPED_PRESENTATION,
            true,
        ),
        BuildingType::Smelter => developed_profile(
            Some("metal_processing"),
            DEVELOPED_FIT_OUT,
            DEVELOPED_PRESENTATION,
            true,
        ),
        BuildingType::Mill => developed_profile(
            Some("grain_milling"),
            DEVELOPED_FIT_OUT,
            DEVELOPED_PRESENTATION,
            true,
        ),
        BuildingType::Sawmill => developed_profile(
            Some("plank_processing"),
            DEVELOPED_FIT_OUT,
            DEVELOPED_PRESENTATION,
            true,
        ),
        // LAI.46 stations. Both are developed 3x3 buildings with an explicit
        // capability permit, matching every other cataloged station; neither
        // reuses a workshop/research fixture it does not own.
        BuildingType::Cookhouse => developed_profile(
            Some("cookhouse"),
            DEVELOPED_FIT_OUT,
            DEVELOPED_PRESENTATION,
            true,
        ),
        BuildingType::FishingHut => developed_profile(
            Some("fishing_hut"),
            DEVELOPED_FIT_OUT,
            DEVELOPED_PRESENTATION,
            true,
        ),
        BuildingType::Barracks | BuildingType::AccountingTent => {
            developed_profile(None, DEVELOPED_FIT_OUT, DEVELOPED_PRESENTATION, false)
        }
        BuildingType::Shrine
        | BuildingType::Walls
        | BuildingType::MouseFarm
        | BuildingType::Field => {
            return None;
        }
    })
}

fn upgrade_scaffold(level: u32) -> BlueprintStage {
    BlueprintStage {
        requirements: vec![BlueprintRequirement {
            content_id: PLANKS,
            units: level + 2,
        }],
    }
}

fn upgrade_structure(level: u32) -> BlueprintStage {
    let mut requirements = vec![
        BlueprintRequirement {
            content_id: BLOCKS,
            units: level + 3,
        },
        BlueprintRequirement {
            content_id: PLANKS,
            units: level * 3,
        },
        BlueprintRequirement {
            content_id: REFINED,
            units: level * 2,
        },
    ];
    if level >= 4 {
        requirements.push(BlueprintRequirement {
            content_id: METAL,
            units: level - 3,
        });
    }
    if level >= 8 {
        requirements.push(BlueprintRequirement {
            content_id: GEMS,
            units: level - 7,
        });
    }
    requirements.sort_unstable();
    BlueprintStage { requirements }
}

fn upgrade_fit_out(profile: BlueprintProfile, level: u32) -> BlueprintStage {
    let mut requirements = profile.new_fit_out.requirements.to_vec();
    if profile.advanced && level >= 4 {
        requirements.push(BlueprintRequirement {
            content_id: METAL,
            units: 1,
        });
    }
    if profile.advanced && level >= 8 {
        requirements.push(BlueprintRequirement {
            content_id: GEMS,
            units: 1,
        });
    }
    requirements.sort_unstable();
    requirements.dedup_by_key(|requirement| requirement.content_id);
    BlueprintStage { requirements }
}

/// Deterministically validate the complete catalog against the embedded content
/// manifest. This is intentionally an explicit coordinator call, so a bad future
/// manifest cannot be silently accepted at project construction time.
pub fn validate_catalog(manifest: &ContentManifest) -> Result<(), CatalogValidationError> {
    let content_ids = manifest
        .canonical_content_entries()
        .into_iter()
        .map(|entry| entry.content_id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    let capabilities = manifest
        .capabilities
        .iter()
        .map(|capability| capability.id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    let blueprints = canonical_blueprints();
    let mut ids = BTreeSet::new();
    let mut covered_level_one = BTreeSet::new();
    let mut covered_upgrades = BTreeSet::new();
    let mut previous_order = None;

    for blueprint in &blueprints {
        if !is_valid_blueprint_id(blueprint.id) || !ids.insert(blueprint.id) {
            return Err(CatalogValidationError::InvalidOrDuplicateBlueprintId(
                blueprint.id,
            ));
        }
        let order = (
            building_order(blueprint.building_type),
            blueprint.target_level,
        );
        if previous_order.is_some_and(|previous| previous >= order) {
            return Err(CatalogValidationError::NonCanonicalBlueprintOrder);
        }
        previous_order = Some(order);
        if blueprint.footprint.width <= 0
            || blueprint.footprint.height <= 0
            || blueprint.footprint.width != footprint_for(blueprint.building_type).0
            || blueprint.footprint.height != footprint_for(blueprint.building_type).1
            || i64::from(blueprint.footprint.width)
                .checked_mul(i64::from(blueprint.footprint.height))
                .is_none()
        {
            return Err(CatalogValidationError::InvalidFootprint(blueprint.id));
        }
        if blueprint.base_work_duration_ms == 0 {
            return Err(CatalogValidationError::ZeroDuration(blueprint.id));
        }
        let expected_duration = if blueprint.target_level == 1 {
            profile_for(blueprint.building_type)
                .expect("cataloged blueprint has profile")
                .level_one_duration_ms
        } else {
            building_upgrade_duration_ms(u32::from(blueprint.target_level)).ok_or(
                CatalogValidationError::InvalidUpgradeLevel(blueprint.target_level),
            )?
        };
        if blueprint.base_work_duration_ms != expected_duration {
            return Err(CatalogValidationError::WrongDuration(blueprint.id));
        }
        if blueprint.presentation.inspector_label.trim().is_empty() {
            return Err(CatalogValidationError::EmptyInspectorLabel(blueprint.id));
        }
        if let Some(capability) = blueprint.permit_capability_id {
            if !capabilities.contains(capability) {
                return Err(CatalogValidationError::UnknownCapability {
                    blueprint_id: blueprint.id,
                    capability_id: capability,
                });
            }
        }
        validate_art_key(
            manifest,
            blueprint.id,
            blueprint.presentation.scaffold_art_key,
        )?;
        validate_art_key(
            manifest,
            blueprint.id,
            blueprint.presentation.structure_art_key,
        )?;
        validate_art_key(
            manifest,
            blueprint.id,
            blueprint.presentation.fit_out_art_key,
        )?;
        for (stage_name, stage) in [
            ("scaffold", &blueprint.scaffold),
            ("structure", &blueprint.structure),
            ("fit_out", &blueprint.fit_out),
        ] {
            validate_stage(&content_ids, blueprint.id, stage_name, stage)?;
        }
        match blueprint.target_level {
            1 if blueprint.target_kind == ConstructionTargetKind::Building => {
                covered_level_one.insert(blueprint.building_type.as_str());
            }
            2..=10 if blueprint.target_kind == ConstructionTargetKind::BuildingUpgrade => {
                covered_upgrades.insert((blueprint.building_type.as_str(), blueprint.target_level));
            }
            level => {
                return Err(CatalogValidationError::InvalidBlueprintLevel(
                    blueprint.id,
                    level,
                ));
            }
        }
        if blueprint.scaffold_tier == ScaffoldTier::Basic
            && blueprint
                .scaffold
                .requirements
                .iter()
                .all(|requirement| requirement.content_id != LOGS)
        {
            return Err(CatalogValidationError::BasicScaffoldMissingLogs(
                blueprint.id,
            ));
        }
        if blueprint.scaffold_tier == ScaffoldTier::Developed
            && [
                &blueprint.scaffold,
                &blueprint.structure,
                &blueprint.fit_out,
            ]
            .into_iter()
            .flat_map(|stage| stage.requirements.iter())
            .any(|requirement| requirement.content_id == LOGS)
        {
            return Err(CatalogValidationError::DevelopedBlueprintUsesLogs(
                blueprint.id,
            ));
        }
    }

    for &building_type in BuildingType::ALL {
        match building_disposition(building_type) {
            BuildingCatalogDisposition::Cataloged => {
                if !covered_level_one.contains(building_type.as_str()) {
                    return Err(CatalogValidationError::MissingLevelOneBlueprint(
                        building_type,
                    ));
                }
                for target_level in 2..=10 {
                    if !covered_upgrades.contains(&(building_type.as_str(), target_level)) {
                        return Err(CatalogValidationError::MissingUpgradeBlueprint {
                            building_type,
                            target_level,
                        });
                    }
                }
            }
            BuildingCatalogDisposition::Retired { .. }
            | BuildingCatalogDisposition::Delegated { .. } => {}
        }
    }
    Ok(())
}

fn validate_stage(
    content_ids: &BTreeSet<String>,
    blueprint_id: &'static str,
    stage_name: &'static str,
    stage: &BlueprintStage,
) -> Result<(), CatalogValidationError> {
    if stage.requirements.is_empty() {
        return Err(CatalogValidationError::EmptyStage {
            blueprint_id,
            stage_name,
        });
    }
    let mut previous = None;
    let mut total_units = 0_u32;
    for requirement in &stage.requirements {
        if requirement.units == 0 {
            return Err(CatalogValidationError::ZeroRequirement {
                blueprint_id,
                stage_name,
                content_id: requirement.content_id,
            });
        }
        if !content_ids.contains(requirement.content_id) {
            return Err(CatalogValidationError::UnknownContentId {
                blueprint_id,
                stage_name,
                content_id: requirement.content_id,
            });
        }
        total_units = total_units.checked_add(requirement.units).ok_or(
            CatalogValidationError::RequirementOverflow {
                blueprint_id,
                stage_name,
            },
        )?;
        if previous.is_some_and(|last: &str| last >= requirement.content_id) {
            return Err(
                CatalogValidationError::NonCanonicalOrDuplicateStageContent {
                    blueprint_id,
                    stage_name,
                },
            );
        }
        previous = Some(requirement.content_id);
    }
    Ok(())
}

fn validate_art_key(
    manifest: &ContentManifest,
    blueprint_id: &'static str,
    art_key_id: &'static str,
) -> Result<(), CatalogValidationError> {
    let art_key = ArtKey::new(art_key_id).map_err(|_| CatalogValidationError::UnknownArtKey {
        blueprint_id,
        art_key: art_key_id,
    })?;
    if manifest.art_asset(&art_key).is_none() {
        return Err(CatalogValidationError::UnknownArtKey {
            blueprint_id,
            art_key: art_key_id,
        });
    }
    Ok(())
}

const fn building_order(building_type: BuildingType) -> usize {
    match building_type {
        BuildingType::Den => 0,
        BuildingType::FoodStorage => 1,
        BuildingType::WaterBowl => 2,
        BuildingType::Beds => 3,
        BuildingType::HerbGarden => 4,
        BuildingType::Nursery => 5,
        BuildingType::ElderCorner => 6,
        BuildingType::Walls => 7,
        BuildingType::MouseFarm => 8,
        BuildingType::Shrine => 9,
        BuildingType::Workshop => 10,
        BuildingType::Field => 11,
        BuildingType::Smithy => 12,
        BuildingType::Barracks => 13,
        BuildingType::AccountingTent => 14,
        BuildingType::WoodCutter => 15,
        BuildingType::StonePrep => 16,
        BuildingType::Woodworking => 17,
        BuildingType::Clothier => 18,
        BuildingType::Tannery => 19,
        BuildingType::ResearchHut => 20,
        BuildingType::Smelter => 21,
        BuildingType::Mill => 22,
        BuildingType::Sawmill => 23,
        BuildingType::School => 24,
        // Plan 2 institutions are additive. Keeping them after the existing
        // stable order avoids changing serialized/catalog iteration for the
        // original 25 building variants.
        BuildingType::FamilyHome => 25,
        BuildingType::ElderLodge => 26,
    }
}

fn is_valid_blueprint_id(value: &str) -> bool {
    value.starts_with("construction_")
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// Complete catalog-validation failure modes. The catalog is immutable and
/// unversioned, so it has no stateful decode path; future persisted project
/// schema remains owned by `construction_stages` and downstream persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogValidationError {
    InvalidOrDuplicateBlueprintId(&'static str),
    NonCanonicalBlueprintOrder,
    InvalidFootprint(&'static str),
    ZeroDuration(&'static str),
    WrongDuration(&'static str),
    EmptyInspectorLabel(&'static str),
    InvalidUpgradeLevel(u8),
    InvalidBlueprintLevel(&'static str, u8),
    UnknownCapability {
        blueprint_id: &'static str,
        capability_id: &'static str,
    },
    UnknownArtKey {
        blueprint_id: &'static str,
        art_key: &'static str,
    },
    EmptyStage {
        blueprint_id: &'static str,
        stage_name: &'static str,
    },
    ZeroRequirement {
        blueprint_id: &'static str,
        stage_name: &'static str,
        content_id: &'static str,
    },
    UnknownContentId {
        blueprint_id: &'static str,
        stage_name: &'static str,
        content_id: &'static str,
    },
    NonCanonicalOrDuplicateStageContent {
        blueprint_id: &'static str,
        stage_name: &'static str,
    },
    BasicScaffoldMissingLogs(&'static str),
    DevelopedBlueprintUsesLogs(&'static str),
    RequirementOverflow {
        blueprint_id: &'static str,
        stage_name: &'static str,
    },
    MissingLevelOneBlueprint(BuildingType),
    MissingUpgradeBlueprint {
        building_type: BuildingType,
        target_level: u8,
    },
}
