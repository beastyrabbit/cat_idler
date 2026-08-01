//! LAI.64 regression coverage for canonical report-safe authority reads.
//!
//! These tests deliberately exercise read-only report APIs.  They never
//! deserialize private authority state to construct a projection, and storage
//! quantities stay in `QualityLotLedger` rather than a report-side copy.

use cat_sim::{
    autonomous_trade::{TradeContractId, TradeProposalId},
    diplomacy::{DiplomacyColonyId, DiplomacyPair},
    family_authority::{
        BirthRegistration, FamilyAuthorityState, FamilyBuilding, FamilyCommand,
        FamilyEnterpriseRecord, FamilyHousehold, FamilyMentorReference, FamilyOperation,
        FamilyResidenceReportAccess,
    },
    family_housing::{HousingKind, LifeStage, TeachingObligation},
    family_specialization::{EnterpriseGoodsOwnership, FamilyBranchRule, FamilyEnterprise},
    moneyless_barter::PersonalStance,
    planner_core::PlannerId,
    progression_research::{ProgressionCatalog, ResearchNotes, StudyCurrency, VoidInsight},
    research_authority::{
        GodResearchFundingReport, ResearchAuthority, ResearchCommand, ResearchCommandId,
        ResearchCommandKind, ResearchStudyCollisionReport,
    },
    spatial_tasks::{Rect, TaskFootprint, TilePoint},
    storage_authority::{
        StorageAuthority, StorageCommand, StorageCommandEnvelope, StorageContainer, StorageZone,
        StorageZoneKind, WorkshopStorageLink,
    },
    trade_authority::{
        TradeAuthority, TradeContractReportAccess, TradePhysicalStagesReportAccess,
        TradeReportUnavailable,
    },
};

fn family_command(
    state: &FamilyAuthorityState,
    receipt_id: &str,
    operation: FamilyOperation,
) -> FamilyCommand {
    FamilyCommand {
        receipt_id: receipt_id.to_owned(),
        expected_revision: state.revision,
        operation,
    }
}

fn family_birth(
    state: &mut FamilyAuthorityState,
    receipt_id: &str,
    cat_id: &str,
    life_stage: LifeStage,
    first_parent_id: Option<&str>,
) {
    state
        .apply(family_command(
            state,
            receipt_id,
            FamilyOperation::RegisterBirth(BirthRegistration {
                newborn_cat_id: cat_id.to_owned(),
                life_stage,
                first_parent_id: first_parent_id.map(str::to_owned),
                second_parent_id: None,
                attribute_authority_ref: format!("attributes-{cat_id}"),
                relational_analytical_authority_ref: format!("axis-{cat_id}"),
            }),
        ))
        .expect("valid family birth");
}

fn storage_envelope(sequence: u64, command: StorageCommand) -> StorageCommandEnvelope {
    StorageCommandEnvelope {
        colony_id: "colony-one".to_owned(),
        command_id: format!("storage-command-{sequence}"),
        fingerprint: format!("storage-fingerprint-{sequence}"),
        sequence,
        command,
    }
}

fn footprint(x: i32, y: i32, width: i32, height: i32) -> TaskFootprint {
    TaskFootprint::rectangular(Rect::try_new(TilePoint { x, y }, width, height).unwrap())
}

#[test]
fn storage_reports_are_stable_visible_topology_without_a_quantity_copy() {
    let mut storage = StorageAuthority::new("colony-one").expect("valid colony");
    let zone = StorageZone::new(
        "zone-workshop-input",
        StorageZoneKind::WorkshopInput,
        footprint(3, 0, 1, 1),
    )
    .expect("valid zone");
    storage
        .execute(storage_envelope(1, StorageCommand::RegisterZone { zone }))
        .expect("register zone");
    storage
        .execute(storage_envelope(
            2,
            StorageCommand::RegisterContainer {
                container: StorageContainer {
                    id: "container-rack".to_owned(),
                    kind: cat_sim::physical_storage::ContainerKind::Rack,
                    zone_id: "zone-workshop-input".to_owned(),
                    tile: TilePoint { x: 3, y: 0 },
                    slot: 0,
                    contents: Default::default(),
                },
            },
        ))
        .expect("register container");
    storage
        .execute(storage_envelope(
            3,
            StorageCommand::LinkWorkshop {
                link: WorkshopStorageLink {
                    workshop_id: "workshop-one".to_owned(),
                    workshop_footprint: footprint(0, 0, 3, 3),
                    zone_id: "zone-workshop-input".to_owned(),
                },
            },
        ))
        .expect("link adjacent workshop input");

    let zones = storage.report_zones().collect::<Vec<_>>();
    assert_eq!(
        zones
            .iter()
            .map(|zone| zone.id.as_str())
            .collect::<Vec<_>>(),
        vec!["zone-workshop-input"]
    );
    assert!(zones[0].tiles.contains_key(&TilePoint { x: 3, y: 0 }));
    let containers = storage.report_containers().collect::<Vec<_>>();
    assert_eq!(containers[0].id, "container-rack");
    assert!(containers[0].contents.is_empty());
    let links = storage.report_workshop_links().collect::<Vec<_>>();
    assert_eq!(links[0].workshop_id, "workshop-one");
    assert_eq!(links[0].zone_id, "zone-workshop-input");
    assert!(storage.ledger().lots().next().is_none());
}

#[test]
fn family_reports_use_real_residence_and_mentorship_references() {
    let mut family = FamilyAuthorityState::empty("colony-one", 7);
    family
        .apply(family_command(
            &family,
            "register-home",
            FamilyOperation::RegisterBuilding(FamilyBuilding {
                building_id: "home-one".to_owned(),
                housing_kind: Some(HousingKind::FamilyHome),
                teaching_site: None,
                completed: true,
                level: 1,
            }),
        ))
        .expect("register family home");
    family_birth(
        &mut family,
        "birth-parent",
        "parent",
        LifeStage::Adult,
        None,
    );
    family_birth(
        &mut family,
        "birth-partner",
        "partner",
        LifeStage::Adult,
        None,
    );
    family_birth(
        &mut family,
        "birth-child",
        "child",
        LifeStage::Kitten,
        Some("parent"),
    );
    family.households.insert(
        "household-parent".to_owned(),
        FamilyHousehold {
            household_id: "household-parent".to_owned(),
            adult_cat_ids: ["parent".to_owned(), "partner".to_owned()]
                .into_iter()
                .collect(),
            dependent_cat_ids: ["child".to_owned()].into_iter().collect(),
            residence_building_id: None,
        },
    );
    family
        .apply(family_command(
            &family,
            "reconcile-housing",
            FamilyOperation::ReconcileHousing {
                pressure_requires_den_return: false,
            },
        ))
        .expect("real residence assignment");
    family
        .apply(family_command(
            &family,
            "assign-mentor",
            FamilyOperation::AssignMentor {
                dependent_cat_id: "child".to_owned(),
                mentor_cat_id: "parent".to_owned(),
            },
        ))
        .expect("assign visible mentor");
    family.teaching_obligations.insert(
        "obligation-parent-child".to_owned(),
        TeachingObligation {
            parent_cat_id: "parent".to_owned(),
            dependent_cat_id: "child".to_owned(),
            completed_real_tasks_since_teach: 3,
            due: true,
            deferred_by_emergency: false,
        },
    );
    family.enterprises.insert(
        "enterprise-weavers".to_owned(),
        FamilyEnterpriseRecord {
            enterprise: FamilyEnterprise {
                enterprise_id: "enterprise-weavers".to_owned(),
                tradition_id: "tradition-weaving".to_owned(),
                profession_id: "weaving".to_owned(),
                site_id: "home-one".to_owned(),
                signage_key: "enterprise.weavers".to_owned(),
                worker_preference: true,
                mentoring_identity: true,
                history_identity: true,
                ui_identity: true,
                goods_ownership: EnterpriseGoodsOwnership::ColonyOwned,
            },
            branch: FamilyBranchRule {
                lineage_id: "lineage-parent".to_owned(),
                profession_id: "weaving".to_owned(),
                adult_surname_key: "surname.weaver".to_owned(),
                child_surname_key: "surname.weaver".to_owned(),
                follows_profession: true,
                ancestry_lineage_ids: vec!["lineage-parent".to_owned()],
            },
        },
    );

    let residences = family.report_residences().collect::<Vec<_>>();
    assert_eq!(residences.len(), 3);
    assert!(
        residences
            .iter()
            .all(|entry| entry.building_id == "home-one")
    );
    assert_eq!(residences[0].housing_kind, Some(HousingKind::FamilyHome));
    assert!(matches!(
        family.report_residence_for("child"),
        FamilyResidenceReportAccess::Report(_)
    ));
    assert!(matches!(
        family.report_residence_for("unknown"),
        FamilyResidenceReportAccess::Unavailable(_)
    ));
    let households = family.report_households().collect::<Vec<_>>();
    assert_eq!(households.len(), 1);
    assert_eq!(households[0].residence_building_id, Some("home-one"));
    let enterprises = family.report_enterprises().collect::<Vec<_>>();
    assert_eq!(enterprises[0].site_building_id, "home-one");
    assert_eq!(
        enterprises[0].goods_ownership,
        EnterpriseGoodsOwnership::ColonyOwned
    );
    let mentorships = family.report_mentorships().collect::<Vec<_>>();
    assert!(matches!(
        mentorships[0].mentor,
        FamilyMentorReference::Assigned("parent")
    ));
    assert!(mentorships[0].due);
    assert_eq!(family.report_assigned_mentors()[0].mentor_cat_id, "parent");
}

fn research_colony() -> PlannerId {
    PlannerId::derive("lai64-report", ["colony-one"])
}

fn research_command(
    authority: &ResearchAuthority,
    action: &str,
    kind: ResearchCommandKind,
) -> ResearchCommand {
    ResearchCommand {
        id: ResearchCommandId::derive(&research_colony(), action),
        expected_version: authority.version,
        kind,
    }
}

#[test]
fn research_reports_preserve_lane_order_and_typed_collisions() {
    let catalog = ProgressionCatalog::from_embedded().expect("embedded catalog");
    let roots = catalog
        .studies()
        .values()
        .filter(|study| study.currency() == StudyCurrency::Notes && study.prerequisites.is_empty())
        .map(|study| study.id.clone())
        .collect::<Vec<_>>();
    let god_target = roots[0].clone();
    let leader_target = roots[1].clone();
    let mut authority = ResearchAuthority::new(
        research_colony(),
        ResearchNotes::from_micro(u64::MAX / 4),
        VoidInsight::from_micro(u64::MAX / 4),
    );
    authority
        .apply(
            &catalog,
            research_command(
                &authority,
                "queue-god",
                ResearchCommandKind::QueueGodPath {
                    target: god_target.clone(),
                },
            ),
        )
        .expect("queue god target");
    authority
        .apply(
            &catalog,
            research_command(
                &authority,
                "request-preparation",
                ResearchCommandKind::RequestPreparation {
                    study_id: god_target.clone(),
                },
            ),
        )
        .expect("request zero-progress preparation");
    authority
        .apply(
            &catalog,
            research_command(
                &authority,
                "prepare-god",
                ResearchCommandKind::PerformPreparation {
                    study_id: god_target.clone(),
                    staffed_research_station: true,
                    scholar_alive: true,
                    labor_minutes: 1,
                },
            ),
        )
        .expect("physical preparation");
    authority
        .apply(
            &catalog,
            research_command(
                &authority,
                "leader-free",
                ResearchCommandKind::CompleteLeader {
                    study_id: leader_target.clone(),
                    effective_loremaster_level: 5,
                    now_tick: 10,
                    duplicate_permit: cat_sim::research_authority::LeaderDuplicatePermit::None,
                },
            ),
        )
        .expect("independent leader lane");

    let god_queue = authority.report_god_queue().collect::<Vec<_>>();
    assert_eq!(god_queue.len(), 1);
    assert_eq!(god_queue[0].study_id, &god_target);
    assert!(matches!(
        god_queue[0].funding,
        GodResearchFundingReport::AwaitingFunding
    ));
    let preparations = authority.report_preparations();
    assert!(!preparations.truncated);
    assert_eq!(preparations.entries[0].study_id, &god_target);
    assert_eq!(
        authority.report_leader_decisions()[0].study_id,
        &leader_target
    );
    assert!(matches!(
        authority.report_study_collision(&god_target),
        ResearchStudyCollisionReport::GodQueued { .. }
    ));
    assert!(matches!(
        authority.report_study_collision(&leader_target),
        ResearchStudyCollisionReport::OwnedFinite
    ));
}

#[test]
fn trade_reports_are_directional_and_hide_foreign_or_missing_contracts() {
    let colony_a = DiplomacyColonyId::derive("colony-a");
    let colony_b = DiplomacyColonyId::derive("colony-b");
    let colony_c = DiplomacyColonyId::derive("colony-c");
    let mut trade = TradeAuthority::new();
    trade
        .set_stance(
            "stance-a-b",
            "stance-a-b-v1",
            0,
            colony_a.clone(),
            colony_b.clone(),
            PersonalStance::Alliance,
        )
        .expect("store directional stance");

    let personal = trade.report_personal_stances_for(&colony_a);
    assert_eq!(personal.entries.len(), 1);
    assert_eq!(personal.entries[0].from, &colony_a);
    assert_eq!(personal.entries[0].to, &colony_b);
    assert_eq!(personal.entries[0].stance, PersonalStance::Alliance);
    assert!(
        trade
            .report_personal_stances_for(&colony_c)
            .entries
            .is_empty()
    );
    assert!(trade.report_contracts_for(&colony_a).entries.is_empty());

    let pair = DiplomacyPair::new(colony_a.clone(), colony_b.clone()).expect("valid pair");
    let proposal = TradeProposalId::derive(&pair, &colony_a, 1);
    let contract_id = TradeContractId::derive(&proposal);
    assert!(matches!(
        trade.report_contract_for(&colony_c, &contract_id),
        TradeContractReportAccess::Unavailable(TradeReportUnavailable::NotVisible)
    ));
    assert!(matches!(
        trade.report_physical_stages_for(&colony_c, &contract_id),
        TradePhysicalStagesReportAccess::Unavailable(TradeReportUnavailable::NotVisible)
    ));
}

#[test]
fn family_report_order_is_stable_across_restart_json_without_using_it_as_projection() {
    let mut family = FamilyAuthorityState::empty("colony-one", 9);
    family_birth(&mut family, "birth-z", "zeta", LifeStage::Adult, None);
    family_birth(&mut family, "birth-a", "alpha", LifeStage::Adult, None);
    let before = family.report_assigned_mentors();
    let restarted = FamilyAuthorityState::decode_json(
        &serde_json::to_string(&family).expect("serialize valid authority"),
    )
    .expect("strict restart validation");
    let after = restarted.report_assigned_mentors();
    assert_eq!(before, after);
    assert_eq!(
        restarted
            .report_residences()
            .map(|entry| entry.resident_cat_id)
            .collect::<Vec<_>>(),
        Vec::<&str>::new()
    );
}
