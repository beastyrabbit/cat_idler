//! LAI.59 canonical construction-blueprint contract.

use std::str::FromStr;

use cat_sim::{
    construction_catalog::{
        BlueprintLookupError, BlueprintRequest, BuildingCatalogDisposition, building_disposition,
        canonical_blueprints, resolve_blueprint, validate_catalog,
    },
    construction_stages::{ConstructionTargetKind, ScaffoldTier},
    content_manifest::ContentManifest,
    types::BuildingType,
};

fn stage_ids(blueprint: &cat_sim::construction_catalog::ConstructionBlueprint) -> Vec<&str> {
    blueprint
        .scaffold
        .requirements()
        .iter()
        .chain(blueprint.structure.requirements())
        .chain(blueprint.fit_out.requirements())
        .map(|requirement| requirement.content_id)
        .collect()
}

fn has(blueprint: &cat_sim::construction_catalog::ConstructionBlueprint, content_id: &str) -> bool {
    stage_ids(blueprint).contains(&content_id)
}

#[test]
fn lai59_every_building_type_has_an_explicit_catalog_or_retirement_delegation() {
    for &building_type in BuildingType::ALL {
        match building_disposition(building_type) {
            BuildingCatalogDisposition::Cataloged => {
                let new = resolve_blueprint(BlueprintRequest::NewBuilding(building_type)).unwrap();
                assert_eq!(new.target_level, 1);
                assert_eq!(new.target_kind, ConstructionTargetKind::Building);
                for target_level in 2..=10 {
                    let upgrade = resolve_blueprint(BlueprintRequest::BuildingUpgrade {
                        building_type,
                        target_level,
                    })
                    .unwrap();
                    assert_eq!(upgrade.target_kind, ConstructionTargetKind::BuildingUpgrade);
                    assert_eq!(upgrade.target_level, target_level);
                }
            }
            BuildingCatalogDisposition::Retired { .. } => {
                assert!(matches!(
                    resolve_blueprint(BlueprintRequest::NewBuilding(building_type)),
                    Err(BlueprintLookupError::RetiredBuilding { .. })
                ));
            }
            BuildingCatalogDisposition::Delegated { .. } => {
                assert!(matches!(
                    resolve_blueprint(BlueprintRequest::NewBuilding(building_type)),
                    Err(BlueprintLookupError::DelegatedBuilding { .. })
                ));
            }
        }
    }
}

#[test]
fn lai59_workshop_is_exactly_three_by_three_and_basic_homes_have_bedding_cloth_and_woodwork() {
    let workshop =
        resolve_blueprint(BlueprintRequest::NewBuilding(BuildingType::Workshop)).unwrap();
    assert_eq!(
        (workshop.footprint.width, workshop.footprint.height),
        (3, 3)
    );

    for building_type in [
        BuildingType::Den,
        BuildingType::Beds,
        BuildingType::Nursery,
        BuildingType::ElderCorner,
    ] {
        let home = resolve_blueprint(BlueprintRequest::NewBuilding(building_type)).unwrap();
        assert_eq!(home.scaffold_tier, ScaffoldTier::Basic);
        assert!(
            has(&home, "resource_cloth"),
            "{building_type:?} lacks bedding cloth"
        );
        assert!(
            has(&home, "item_furniture"),
            "{building_type:?} lacks woodwork"
        );
    }
}

#[test]
fn lai59_advanced_workshop_progresses_fixture_tool_metal_and_gems() {
    let level_one =
        resolve_blueprint(BlueprintRequest::NewBuilding(BuildingType::Workshop)).unwrap();
    assert!(has(&level_one, "fixture_workshop"));
    assert!(has(&level_one, "item_generic_tool"));
    assert!(has(&level_one, "resource_refined"));

    let level_four = resolve_blueprint(BlueprintRequest::BuildingUpgrade {
        building_type: BuildingType::Workshop,
        target_level: 4,
    })
    .unwrap();
    assert!(has(&level_four, "resource_metal"));

    let level_eight = resolve_blueprint(BlueprintRequest::BuildingUpgrade {
        building_type: BuildingType::Workshop,
        target_level: 8,
    })
    .unwrap();
    assert!(has(&level_eight, "resource_gem"));
}

#[test]
fn lai59_raw_logs_are_limited_to_basic_new_buildings_and_developed_work_uses_processed_timber() {
    for blueprint in canonical_blueprints() {
        let scaffold_ids = blueprint
            .scaffold
            .requirements()
            .iter()
            .map(|requirement| requirement.content_id)
            .collect::<Vec<_>>();
        if blueprint.scaffold_tier == ScaffoldTier::Basic {
            assert_eq!(blueprint.target_level, 1);
            assert!(scaffold_ids.contains(&"resource_logs"));
        } else {
            assert!(!stage_ids(&blueprint).contains(&"resource_logs"));
            assert!(!scaffold_ids.contains(&"resource_logs"));
            assert!(
                scaffold_ids.contains(&"resource_lumber")
                    || scaffold_ids.contains(&"resource_planks")
            );
        }
    }
}

#[test]
fn lai59_fresh_project_bills_have_exact_requirements_and_zero_mutable_counters() {
    let blueprint =
        resolve_blueprint(BlueprintRequest::NewBuilding(BuildingType::Workshop)).unwrap();
    let bills = blueprint.fresh_bills();
    for (recipe, bill) in [
        (blueprint.scaffold.requirements(), &bills.scaffold),
        (blueprint.structure.requirements(), &bills.structure),
        (blueprint.fit_out.requirements(), &bills.fit_out),
    ] {
        assert_eq!(bill.lines.len(), recipe.len());
        for (line, requirement) in bill.lines.iter().zip(recipe) {
            assert_eq!(line.content_id, requirement.content_id);
            assert_eq!(line.required_units, requirement.units);
            assert_eq!(line.delivered_units, 0);
            assert_eq!(line.in_transit_units, 0);
            assert_eq!(line.consumed_units, 0);
        }
    }
}

#[test]
fn lai59_manifest_validation_stable_order_and_closed_wire_inputs_hold() {
    validate_catalog(ContentManifest::embedded()).unwrap();

    let first = canonical_blueprints();
    let restarted = canonical_blueprints();
    assert_eq!(first, restarted);
    assert_eq!(first.first().unwrap().id, "construction_den_new_l01");
    assert!(first.windows(2).all(|pair| {
        pair[0].building_type != pair[1].building_type
            || pair[0].target_level + 1 == pair[1].target_level
    }));
    assert!(
        first
            .iter()
            .all(|blueprint| blueprint.id.starts_with("construction_"))
    );

    assert!(BuildingType::from_str("future_building").is_err());
    assert!(serde_json::from_str::<BuildingType>("\"future_building\"").is_err());
}

#[test]
fn lai59_shrine_and_hole_are_explicitly_rejected_and_no_alias_or_currency_leaks_into_bills() {
    assert!(matches!(
        resolve_blueprint(BlueprintRequest::NewBuilding(BuildingType::Shrine)),
        Err(BlueprintLookupError::RetiredBuilding { .. })
    ));
    assert!(matches!(
        resolve_blueprint(BlueprintRequest::HoleUpgrade),
        Err(BlueprintLookupError::HoleUpgradeDelegated {
            owner: "black_hole::upgrade_bill"
        })
    ));

    for blueprint in canonical_blueprints() {
        for content_id in stage_ids(&blueprint) {
            assert_ne!(content_id, "wood");
            assert!(!content_id.starts_with("food_"));
            assert!(!content_id.contains("coin"));
            assert!(!content_id.contains("currency"));
        }
    }
}
