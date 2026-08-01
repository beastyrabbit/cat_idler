//! Executable LAI.33 signed journey foundation.

use cat_protocol::{
    ActionConflict, ActionProtocolVersion, ActionValidationPipeline, BoundedBasisPoints,
    BoundedColonyId, BoundedEntityId, BoundedFavorAmount, BoundedStandingOrderText,
    CurrentStateHint, CurrentVersionHint, DiplomacyRelationshipTarget, DismissalReason,
    LeaderAiActionPayload, LeaderAiActionResponse, LeaderAiActionResult, OfficerAuthorityMode,
    PublicVillageSnapshot, ReportSafeString, SiteRefActionTarget, SnapshotTilePoint,
    SnapshotVillageCapabilities, StaleClientRefresh, StandingOrderPatch, TradeRejectionReason,
};
use cat_server::{
    leader_ai_action_routing::{
        AtomicLeaderAiCommit, ExpectedServerStateVersions, IdempotencyReplay,
        LeaderAiServerMutationPipeline, OrderedMutationExecutor, ServerActionConflict,
        check_protocol_compatibility, constant_time_session_mac_check,
    },
    leader_ai_journey::{
        Lai33SignedSystemJourneyHarness, lai33_fixture_manifest, lai33_fixture_world,
    },
    leader_ai_snapshot_projection::project_selected_colony,
    persistence,
};
use rusqlite::Connection;
use sha2::{Digest, Sha256};

#[test]
fn deterministic_fresh_and_migrated_startup_use_production_setup() {
    let fresh_a = Lai33SignedSystemJourneyHarness::fresh();
    let fresh_b = Lai33SignedSystemJourneyHarness::fresh();
    assert_eq!(fresh_a.session, fresh_b.session);
    assert_eq!(fresh_a.secret, fresh_b.secret);
    assert_eq!(
        Lai33SignedSystemJourneyHarness::migrated().secret,
        "lai33-secret-5333a002"
    );
    assert_eq!(lai33_fixture_manifest()["freshSeed"], 0x5333_A001u32);
    assert_eq!(lai33_fixture_manifest()["migratedSeed"], 0x5333_A002u32);
}

#[test]
fn authenticated_signed_action_order_and_foreign_isolation_are_executable() {
    let harness = Lai33SignedSystemJourneyHarness::fresh();
    assert!(constant_time_session_mac_check(
        &harness.session,
        &harness.secret,
        0
    ));
    assert_eq!(
        ActionValidationPipeline::ordered_steps().len(),
        8,
        "protocol/auth/ownership/version/replay/precondition/commit order is fixed"
    );
    let action = harness.signed_action("colony-fresh-a", "action-plan-001");
    let authorized = harness
        .authenticate(action, 0)
        .expect("signed route authorization");
    assert_eq!(
        authorized.ownership().colony_id().as_str(),
        "colony-fresh-a"
    );
    let foreign = harness.signed_action("colony-foreign-b", "action-plan-foreign");
    assert!(matches!(
        harness.authenticate(foreign, 0),
        Err(ServerActionConflict::OpaqueExistenceDenied)
    ));
}

#[test]
fn every_lai25_domain_payload_reaches_the_authenticated_production_route() {
    let harness = Lai33SignedSystemJourneyHarness::fresh();
    let entity = |value: &str| BoundedEntityId::new(value).unwrap();
    let payloads = vec![
        LeaderAiActionPayload::CreateStandingOrder {
            order_kind: entity("guard"),
            domain: entity("water"),
            target_id: Some(entity("site-001")),
            instruction: BoundedStandingOrderText::new("maintain reserve").unwrap(),
            priority_basis_points: BoundedBasisPoints::new(5_000).unwrap(),
            expires_at_ms: Some(10_000),
        },
        LeaderAiActionPayload::UpdateStandingOrder {
            standing_order_id: entity("order-001"),
            patch: StandingOrderPatch {
                instruction: Some(BoundedStandingOrderText::new("maintain reserve").unwrap()),
                priority_basis_points: None,
                target_id: None,
                clear_target: false,
                expires_at_ms: None,
                clear_expiry: false,
            },
        },
        LeaderAiActionPayload::DeleteStandingOrder {
            standing_order_id: entity("order-001"),
        },
        LeaderAiActionPayload::DismissIntent {
            intent_id: entity("intent-001"),
            planning_epoch: 0,
            reason: DismissalReason::PlayerPriority,
        },
        LeaderAiActionPayload::AppointOfficer {
            role: cat_protocol::OfficerRole::Steward,
            cat_id: entity("cat-001"),
        },
        LeaderAiActionPayload::UnappointOfficer {
            role: cat_protocol::OfficerRole::Steward,
        },
        LeaderAiActionPayload::OfficerAuthorityOverride {
            role: cat_protocol::OfficerRole::Steward,
            domain: entity("water"),
            request_id: Some(entity("request-001")),
            mode: OfficerAuthorityMode::Grant,
        },
        LeaderAiActionPayload::RequestTreatment {
            cat_id: entity("cat-001"),
            injury_id: entity("injury-001"),
            treatment_kind: entity("bandage"),
        },
        LeaderAiActionPayload::FitProsthetic {
            cat_id: entity("cat-001"),
            prosthetic_id: entity("prosthetic-001"),
            body_part_id: entity("front-left-paw"),
            fitting_site: SiteRefActionTarget::ExactTile {
                tile: SnapshotTilePoint { x: 0, y: 0 },
            },
            fitter_cat_id: None,
        },
        LeaderAiActionPayload::RepairProsthetic {
            prosthetic_id: entity("prosthetic-001"),
            workshop_id: entity("workshop-001"),
            input_reservation_id: entity("reservation-001"),
        },
        LeaderAiActionPayload::PurchaseResearchWithFavor {
            study_id: entity("study-001"),
            use_preparation: false,
            displayed_price_micro_favor: Some(BoundedFavorAmount::new(1).unwrap()),
        },
        LeaderAiActionPayload::PrepareScholarStudy {
            study_id: entity("study-001"),
            scholar_cat_id: entity("cat-001"),
        },
        LeaderAiActionPayload::ActivateDivineBoost {
            boost_kind: entity("harvest"),
            duration_hours: 1,
            displayed_price_micro_favor: Some(BoundedFavorAmount::new(1).unwrap()),
        },
        LeaderAiActionPayload::ChangeDiplomacy {
            target_colony_id: BoundedColonyId::new("colony-foreign-b").unwrap(),
            relationship: DiplomacyRelationshipTarget::Friendly,
        },
        LeaderAiActionPayload::ApproveAlliance {
            target_colony_id: BoundedColonyId::new("colony-foreign-b").unwrap(),
            proposal_id: entity("proposal-001"),
        },
        LeaderAiActionPayload::BlockColony {
            target_colony_id: BoundedColonyId::new("colony-foreign-b").unwrap(),
            public_reason: Some(ReportSafeString::new("terms_declined").unwrap()),
        },
        LeaderAiActionPayload::AcceptTradeContract {
            contract_id: entity("contract-001"),
        },
        LeaderAiActionPayload::RejectTradeContract {
            contract_id: entity("contract-001"),
            reason: TradeRejectionReason::TermsDeclined,
        },
    ];
    for (index, payload) in payloads.into_iter().enumerate() {
        let action = harness.signed_payload_action(
            "colony-fresh-a",
            &format!("action-domain-{index:03}"),
            payload,
        );
        harness
            .authenticate(action, 0)
            .unwrap_or_else(|error| panic!("domain payload {index} was rejected: {error:?}"));
    }
}

#[test]
fn idempotent_replay_is_identical_and_old_clients_fail_before_decode() {
    let mut harness = Lai33SignedSystemJourneyHarness::fresh();
    let action = harness.signed_action("colony-fresh-a", "action-replay-001");
    let response = LeaderAiActionResponse {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: action.idempotency_id.clone(),
        colony_id: action.colony_id.clone(),
        result: LeaderAiActionResult::Rejected {
            conflict: ActionConflict::PreconditionFailed {
                reason: cat_protocol::ReportSafeString::new("fixture_precondition").unwrap(),
            },
        },
        refresh: None,
    };
    let encoded = serde_json::to_string(&action).unwrap();
    harness.receipts.record(&action, response.clone()).unwrap();
    let replay = harness
        .receipts
        .check_bounded_idempotent_replay(&action)
        .unwrap();
    match replay {
        IdempotencyReplay::ReplayRejectedPriorResult(value) => {
            assert_eq!(value.idempotency_id, response.idempotency_id);
            assert_eq!(value.colony_id, response.colony_id);
            assert!(matches!(
                value.result,
                LeaderAiActionResult::DuplicateReplay { replay } if !replay.original_accepted
            ));
        }
        other => panic!("expected deterministic rejected replay, got {other:?}"),
    }
    let old = serde_json::json!({
        "protocolVersion": ActionProtocolVersion::current().get().saturating_sub(1),
        "payload": {"action": "nudge_plan"}
    })
    .to_string();
    assert!(matches!(
        check_protocol_compatibility(&old),
        Err(ServerActionConflict::UpdateRequired(_))
    ));
    assert!(
        LeaderAiServerMutationPipeline::validate_foundation(
            &encoded,
            &harness.session,
            &harness.secret,
            0,
            &harness,
        )
        .is_ok()
    );
}

#[test]
fn domain_replay_does_not_duplicate_favor_reservation_or_cargo_effects() {
    let mut harness = Lai33SignedSystemJourneyHarness::fresh();
    let action = harness.signed_payload_action(
        "colony-fresh-a",
        "action-favor-replay-001",
        LeaderAiActionPayload::PurchaseResearchWithFavor {
            study_id: BoundedEntityId::new("study-001").unwrap(),
            use_preparation: false,
            displayed_price_micro_favor: Some(BoundedFavorAmount::new(1).unwrap()),
        },
    );
    let response = LeaderAiActionResponse {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: action.idempotency_id.clone(),
        colony_id: action.colony_id.clone(),
        result: LeaderAiActionResult::Rejected {
            conflict: ActionConflict::PreconditionFailed {
                reason: ReportSafeString::new("fixture_precondition").unwrap(),
            },
        },
        refresh: None,
    };
    harness.receipts.record(&action, response).unwrap();
    let mut effects = std::collections::BTreeMap::from([
        ("favor", 10_u64),
        ("reservation", 1_u64),
        ("cargo", 1_u64),
    ]);
    let before = effects.clone();
    let replay = harness
        .receipts
        .check_bounded_idempotent_replay(&action)
        .unwrap();
    assert!(matches!(
        replay,
        IdempotencyReplay::ReplayRejectedPriorResult(_)
    ));
    assert_eq!(effects, before);
    let mut committed = AtomicLeaderAiCommit::stage(&effects);
    committed.candidate_mut().insert("favor", 9);
    committed.candidate_mut().insert("reservation", 2);
    committed.candidate_mut().insert("cargo", 2);
    committed.commit_favor_debit_once(&mut effects);
    assert_eq!(effects["favor"], 9);
    assert_eq!(effects["reservation"], 2);
    assert_eq!(effects["cargo"], 2);
    let replay_again = harness
        .receipts
        .check_bounded_idempotent_replay(&action)
        .unwrap();
    assert!(matches!(
        replay_again,
        IdempotencyReplay::ReplayRejectedPriorResult(_)
    ));
    assert_eq!(effects["favor"], 9);
    assert_eq!(effects["reservation"], 2);
    assert_eq!(effects["cargo"], 2);
}

#[test]
fn fixture_manifest_is_bounded_and_route_only() {
    let manifest = lai33_fixture_manifest();
    assert_eq!(manifest["schema"], "lai33-signed-system-journey-v1");
    assert_eq!(
        manifest["colonies"],
        serde_json::json!([
            "global",
            "colony-fresh-a",
            "colony-migrated-a",
            "colony-foreign-b"
        ])
    );
    assert!(
        manifest["authoritativeRoutes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|route| matches!(
                route.as_str(),
                Some("snapshot" | "lai25_action" | "sqlite_migration")
            ))
    );
}

#[test]
fn exact_runtime_save_reload_checksum_and_quarantine_journey() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!(
            "lai33-journey-{}-{}.sqlite3",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
    let _ = std::fs::remove_file(&path);
    let connection = Connection::open(&path).unwrap();
    persistence::init_schema(&connection).unwrap();
    let world = lai33_fixture_world();
    persistence::save_world(&connection, &world).unwrap();
    let before = Sha256::digest(std::fs::read(&path).unwrap());
    let loaded = persistence::load_world(&connection).unwrap().unwrap();
    assert_eq!(loaded.world_seed, world.world_seed);
    assert_eq!(loaded.colonies.len(), world.colonies.len());
    for (loaded_colony, original_colony) in loaded.colonies.iter().zip(&world.colonies) {
        assert_eq!(loaded_colony.id, original_colony.id);
        assert_eq!(
            loaded_colony.leader_ai_runtime,
            original_colony.leader_ai_runtime
        );
    }
    let after = Sha256::digest(std::fs::read(&path).unwrap());
    assert_eq!(before.as_slice(), after.as_slice());
    connection
        .execute(
            "UPDATE leader_ai_colony_runtime SET runtimeJson = ?1 WHERE colonyId = 'global'",
            [r#"{"schemaVersion":1,"hiddenExactValue":99}"#],
        )
        .unwrap();
    assert!(persistence::load_world(&connection).is_err());
    let quarantine_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM leader_ai_quarantine", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(quarantine_count, 1);
    let _ = std::fs::remove_file(path);
}

struct StaleExecutor {
    commits: usize,
}

impl OrderedMutationExecutor for StaleExecutor {
    fn check_expected_state_versions(
        &mut self,
        _authorized: &cat_server::leader_ai_action_routing::AuthorizedMutation,
        _expected: ExpectedServerStateVersions<'_>,
    ) -> Result<(), ServerActionConflict> {
        Err(ServerActionConflict::VersionMismatch(Box::new(
            StaleClientRefresh {
                current_versions: CurrentVersionHint {
                    planner_version: Some(7),
                    domain_version: Some(3),
                    resource_version: Some(11),
                    spatial_version: None,
                    reservation_version: None,
                    research_version: None,
                    scholar_version: None,
                    boost_version: None,
                    diplomacy_version: None,
                    trade_version: None,
                    prosthetic_version: None,
                    care_version: None,
                    officer_version: None,
                    standing_order_version: None,
                },
                current_state_hint: CurrentStateHint {
                    state_code: ReportSafeString::new("refresh_required").unwrap(),
                    visible_entity_id: None,
                    visible_stage: None,
                },
            },
        )))
    }

    fn check_bounded_idempotent_replay(
        &mut self,
        _authorized: &cat_server::leader_ai_action_routing::AuthorizedMutation,
    ) -> Result<Option<LeaderAiActionResponse>, ServerActionConflict> {
        panic!("replay must not run before stale-version rejection")
    }

    fn check_current_preconditions(
        &mut self,
        _authorized: &cat_server::leader_ai_action_routing::AuthorizedMutation,
    ) -> Result<(), ServerActionConflict> {
        panic!("preconditions must not run before stale-version rejection")
    }

    fn commit_atomic_favor_reservation_state(
        &mut self,
        _authorized: &cat_server::leader_ai_action_routing::AuthorizedMutation,
    ) -> Result<LeaderAiActionResponse, ServerActionConflict> {
        self.commits += 1;
        panic!("commit must not run after stale-version rejection")
    }
}

#[test]
fn stale_expected_versions_return_refresh_without_mutation_or_partial_commit() {
    let harness = Lai33SignedSystemJourneyHarness::fresh();
    let authorized = harness
        .authenticate(
            harness.signed_action("colony-fresh-a", "action-stale-001"),
            0,
        )
        .expect("foundation authorization");
    let mut executor = StaleExecutor { commits: 0 };
    let error = LeaderAiServerMutationPipeline::execute_remaining(&authorized, &mut executor)
        .expect_err("stale versions must reject before replay or commit");
    assert!(matches!(error, ServerActionConflict::VersionMismatch(_)));
    let refresh = error
        .refresh_hint()
        .expect("stale response carries refresh");
    assert_eq!(refresh.current_versions.planner_version, Some(7));
    assert_eq!(
        refresh.current_state_hint.state_code.as_str(),
        "refresh_required"
    );
    assert_eq!(executor.commits, 0);
}

#[test]
fn selected_colony_projection_redacts_foreign_state_and_low_level_regeneration() {
    let world = lai33_fixture_world();
    let public_villages = world
        .colonies
        .iter()
        .map(|colony| PublicVillageSnapshot {
            colony_id: cat_protocol::NonEmptyStableId::new(colony.id.clone()).unwrap(),
            display_name: ReportSafeString::new(colony.id.clone()).unwrap(),
            capabilities: SnapshotVillageCapabilities {
                can_view: true,
                can_control: colony.id == "colony-fresh-a",
                is_owner: colony.id == "colony-fresh-a",
            },
        })
        .collect();
    let snapshot = project_selected_colony(
        &world,
        "colony-fresh-a",
        SnapshotVillageCapabilities {
            can_view: true,
            can_control: true,
            is_owner: true,
        },
        public_villages,
        0,
    )
    .expect("live projection is valid");
    let redacted = cat_server::leader_ai_action_routing::server_redaction_before_websocket_send(
        snapshot,
        "colony-fresh-a",
    )
    .expect("server redaction succeeds");
    let decoded = cat_protocol::LeaderAiSnapshotEnvelope::decode_json(&redacted).unwrap();
    assert_eq!(decoded.colonies.len(), 1);
    assert_eq!(decoded.colonies[0].colony_id.as_str(), "colony-fresh-a");
    assert!(
        decoded.colonies[0]
            .reports
            .iter()
            .all(|report| report.report_level >= 4
                || matches!(
                    report.regeneration,
                    cat_protocol::RegenerationReportSnapshot::UnavailableBelowLevel4
                ))
    );
    assert!(!redacted.contains("hiddenExactValue"));
}

#[test]
fn authored_browser_fixture_projects_every_visible_acceptance_scenario() {
    let world = lai33_fixture_world();
    let snapshot = project_selected_colony(
        &world,
        "global",
        SnapshotVillageCapabilities {
            can_view: true,
            can_control: true,
            is_owner: false,
        },
        Vec::new(),
        0,
    )
    .expect("authored browser fixture projects through the production boundary");
    let colony = &snapshot.colonies[0];
    let categories = colony
        .visible_tasks
        .iter()
        .map(|task| task.category.as_str())
        .collect::<Vec<_>>();
    assert!(categories.contains(&"hunt"));
    assert!(categories.contains(&"fetch_water"));
    assert!(categories.contains(&"workshop_work"));
    let workshop = colony
        .visible_tasks
        .iter()
        .find(|task| task.category.as_str() == "workshop_work")
        .expect("fixture workshop task");
    assert_eq!(workshop.footprint.len(), 9);
    let water = colony
        .visible_tasks
        .iter()
        .find(|task| task.category.as_str() == "fetch_water")
        .expect("fixture water task");
    assert_eq!(water.footprint.len(), 3);
    assert!(
        colony
            .cats
            .iter()
            .flat_map(|cat| &cat.anatomy.body_parts)
            .any(|part| part
                .injury
                .as_ref()
                .is_some_and(|injury| injury.injury_kind.as_str() == "severe"))
    );
    assert!(
        colony
            .cats
            .iter()
            .any(|cat| !cat.prosthetics.is_empty() && cat.care.care_site.is_some())
    );
    assert!(colony.shrine.pipeline.is_some());
    assert_eq!(colony.favor.favor_events.len(), 1);
    assert!(!colony.research.frontier.is_empty());
    assert_eq!(colony.diplomacy.relationships.len(), 1);
    assert_eq!(colony.trade.len(), 1);
}
