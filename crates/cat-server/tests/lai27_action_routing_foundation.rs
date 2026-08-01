use std::{cell::Cell, collections::BTreeMap};

use cat_protocol::{
    ActionAcceptedResult, ActionIdempotencyId, ActionProtocolVersion, AuthenticatedPlayerId,
    AutomaticResearchQuotaSnapshot, BeliefReportSnapshot, BoundedAgeMs, BoundedBasisPointNudge,
    BoundedBasisPoints, BoundedEntityId, ColonyAiSnapshot, CurrentStateHint, CurrentVersionHint,
    DiplomacySnapshot, ExpectedStateVersions, FavorLedgerSnapshot, InsightSnapshot,
    LeaderAiActionEnvelope, LeaderAiActionPayload, LeaderAiActionResponse, LeaderAiActionResult,
    LeaderAiSnapshotEnvelope, NonEmptyStableId, PlanQueueSnapshot, RegenerationReportSnapshot,
    ReportEstimateSnapshot, ReportProvenanceSnapshot, ReportSafeString, ResearchFrontierSnapshot,
    SelectedColonyId, ShrineSnapshot, SiteLifecycleStageSnapshot, SiteRefSnapshot, SiteSnapshot,
    SiteVisibilitySnapshot, SnapshotProtocolVersion, SnapshotTilePoint,
    SnapshotVillageCapabilities, StaleClientRefresh,
};
use cat_sim::{authority::AuthorityDomain, officers::OfficerRole};

#[allow(dead_code)]
#[path = "../src/identity.rs"]
mod identity;
#[allow(dead_code)]
#[path = "../src/leader_ai_action_routing.rs"]
mod leader_ai_action_routing;

use leader_ai_action_routing::{
    ActorActionAuthorityClassification, AuthorizedMutation, ColonyControlPolicy,
    ExpectedServerStateVersions, IdempotencyReceiptStore, IdempotencyReplay,
    LeaderAiServerMutationPipeline, OfficerDomainAuthorityGuard, OrderedMutationExecutor,
    SelectedColonyOwnershipSource, ServerActionConflict, ServerActionResult, ServerMutationActor,
    ServerSideSnapshotRedactor, UpdateRequiredResponse, check_actor_action_authority,
    check_expected_state_versions, classify_actor_action_authority, project_server_action_response,
};

fn now_ms() -> i64 {
    1_000_000
}

fn entity(value: &str) -> BoundedEntityId {
    BoundedEntityId::new(value).expect("valid entity id")
}

fn text(value: &str) -> ReportSafeString {
    ReportSafeString::new(value).expect("valid report-safe string")
}

fn versions() -> ExpectedStateVersions {
    ExpectedStateVersions {
        expected_planner_version: 1,
        expected_domain_version: 2,
        expected_resource_version: 3,
        expected_spatial_version: Some(4),
        expected_reservation_version: Some(5),
        expected_research_version: Some(6),
        expected_scholar_version: Some(7),
        expected_boost_version: Some(8),
        expected_diplomacy_version: Some(9),
        expected_trade_version: Some(10),
        expected_prosthetic_version: Some(11),
        expected_care_version: Some(12),
        expected_officer_version: Some(13),
        expected_standing_order_version: Some(14),
    }
}

fn envelope(player_id: &str, colony_id: &str) -> LeaderAiActionEnvelope {
    LeaderAiActionEnvelope {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: ActionIdempotencyId::new("action:routing:1").expect("valid action id"),
        colony_id: SelectedColonyId::new(colony_id).expect("valid colony id"),
        player_id: AuthenticatedPlayerId::new(player_id).expect("valid player id"),
        expected_versions: versions(),
        payload: LeaderAiActionPayload::NudgePlan {
            plan_id: entity("plan:one"),
            nudge: BoundedBasisPointNudge::new(1_500).expect("valid nudge"),
            reason_key: None,
        },
    }
}

fn boost_envelope(player_id: &str, colony_id: &str) -> LeaderAiActionEnvelope {
    let mut envelope = envelope(player_id, colony_id);
    envelope.payload = LeaderAiActionPayload::ActivateDivineBoost {
        boost_kind: entity("boost:harvest"),
        duration_hours: 24,
        displayed_price_micro_favor: None,
    };
    envelope
}

fn signed_fixture(secret: &str) -> identity::SignedSession {
    identity::signed_session("session-routing-fixture".to_owned(), secret)
}

#[derive(Default)]
struct Directory {
    policies: BTreeMap<String, ColonyControlPolicy>,
    lookups: Cell<usize>,
}

impl Directory {
    fn with(mut self, colony_id: &str, policy: ColonyControlPolicy) -> Self {
        self.policies.insert(colony_id.to_owned(), policy);
        self
    }
}

impl SelectedColonyOwnershipSource for Directory {
    fn control_policy(&self, colony_id: &str) -> Option<ColonyControlPolicy> {
        self.lookups.set(self.lookups.get() + 1);
        self.policies.get(colony_id).cloned()
    }
}

fn validate(
    envelope: &LeaderAiActionEnvelope,
    session: &identity::SignedSession,
    secret: &str,
    directory: &Directory,
) -> Result<AuthorizedMutation, ServerActionConflict> {
    LeaderAiServerMutationPipeline::validate_foundation(
        &serde_json::to_string(envelope).expect("serialize action"),
        session,
        secret,
        now_ms(),
        directory,
    )
}

#[test]
fn incompatible_protocol_rejects_before_nested_decode_auth_or_ownership() {
    let secret = "routing-secret";
    let session = signed_fixture(secret);
    let directory = Directory::default();
    let mut value = serde_json::to_value(envelope(&session.player_id, "colony:home"))
        .expect("serialize action");
    value["protocolVersion"] = serde_json::Value::from(999);
    value["payload"]["action"] = serde_json::Value::String("unknown_future_action".into());

    let error = LeaderAiServerMutationPipeline::validate_foundation(
        &serde_json::to_string(&value).expect("serialize malformed action"),
        &identity::SignedSession {
            sig: "bad".into(),
            ..session
        },
        secret,
        now_ms(),
        &directory,
    )
    .expect_err("incompatible protocol must win");
    assert_eq!(
        error,
        ServerActionConflict::UpdateRequired(UpdateRequiredResponse::current())
    );
    assert_eq!(directory.lookups.get(), 0);
}

#[test]
fn current_unknown_action_fails_closed_before_auth_and_ownership() {
    let secret = "routing-secret";
    let session = signed_fixture(secret);
    let directory = Directory::default();
    let mut value = serde_json::to_value(envelope(&session.player_id, "colony:home"))
        .expect("serialize action");
    value["payload"]["action"] = serde_json::Value::String("unknown_future_action".into());

    let error = LeaderAiServerMutationPipeline::validate_foundation(
        &serde_json::to_string(&value).expect("serialize unknown action"),
        &identity::SignedSession {
            sig: "bad".into(),
            ..session
        },
        secret,
        now_ms(),
        &directory,
    )
    .expect_err("unknown action must fail closed");
    assert_eq!(error, ServerActionConflict::UnknownActionVariant);
    assert_eq!(directory.lookups.get(), 0);
}

#[test]
fn real_hmac_session_and_envelope_player_must_both_match_before_ownership() {
    let secret = "routing-secret";
    let session = signed_fixture(secret);
    let directory = Directory::default();
    let action = envelope(&session.player_id, "colony:home");

    let mut bad_mac = session.clone();
    bad_mac.sig = "0".repeat(64);
    assert_eq!(
        validate(&action, &bad_mac, secret, &directory),
        Err(ServerActionConflict::Unauthenticated)
    );
    assert_eq!(directory.lookups.get(), 0);

    let wrong_player = envelope("player:someone-else", "colony:home");
    assert_eq!(
        validate(&wrong_player, &session, secret, &directory),
        Err(ServerActionConflict::Unauthenticated)
    );
    assert_eq!(directory.lookups.get(), 0);
}

#[test]
fn global_and_owned_colonies_pass_while_missing_and_foreign_are_opaque_twins() {
    let secret = "routing-secret";
    let session = signed_fixture(secret);
    let directory = Directory::default()
        .with("colony:global", ColonyControlPolicy::GlobalVillage)
        .with(
            "colony:owned",
            ColonyControlPolicy::PlayerOwned {
                owner_player_id: session.player_id.clone(),
            },
        )
        .with(
            "colony:foreign",
            ColonyControlPolicy::PlayerOwned {
                owner_player_id: "player_other".into(),
            },
        );

    assert!(
        validate(
            &envelope(&session.player_id, "colony:global"),
            &session,
            secret,
            &directory
        )
        .is_ok()
    );
    assert!(
        validate(
            &envelope(&session.player_id, "colony:owned"),
            &session,
            secret,
            &directory
        )
        .is_ok()
    );
    let foreign = validate(
        &envelope(&session.player_id, "colony:foreign"),
        &session,
        secret,
        &directory,
    )
    .expect_err("foreign colony denied");
    let missing = validate(
        &envelope(&session.player_id, "colony:missing"),
        &session,
        secret,
        &directory,
    )
    .expect_err("missing colony denied");
    assert_eq!(foreign, ServerActionConflict::OpaqueExistenceDenied);
    assert_eq!(missing, foreign);
}

#[test]
fn player_only_boost_and_real_officer_domain_classification_do_not_forge_player_actions() {
    let secret = "routing-secret";
    let session = signed_fixture(secret);
    let directory = Directory::default().with("colony:home", ColonyControlPolicy::GlobalVillage);
    let authorized = validate(
        &boost_envelope(&session.player_id, "colony:home"),
        &session,
        secret,
        &directory,
    )
    .expect("authenticated player may request boost");
    assert_eq!(authorized.ownership().colony_id().as_str(), "colony:home");
    assert_eq!(
        authorized.verified_session().player_id().as_str(),
        session.player_id
    );
    assert_eq!(
        authorized.verified_session().rate_limit_key(),
        format!("s:{}", session.session_id)
    );

    assert_eq!(
        check_actor_action_authority(ServerMutationActor::Leader, authorized.envelope()),
        Err(ServerActionConflict::RejectLeaderBoostActivation)
    );
    assert_eq!(
        check_actor_action_authority(
            ServerMutationActor::Officer {
                role: OfficerRole::Loremaster
            },
            authorized.envelope()
        ),
        Err(ServerActionConflict::RejectOfficerBoostActivation)
    );
    assert!(OfficerDomainAuthorityGuard::owns(
        OfficerRole::Loremaster,
        AuthorityDomain::Research
    ));
    assert!(!OfficerDomainAuthorityGuard::owns(
        OfficerRole::Accountant,
        AuthorityDomain::Research
    ));

    let research = {
        let mut action = envelope(&session.player_id, "colony:home");
        action.payload = LeaderAiActionPayload::PurchaseResearchWithFavor {
            study_id: entity("study:one"),
            use_preparation: false,
            displayed_price_micro_favor: None,
        };
        action
    };
    assert_eq!(
        classify_actor_action_authority(
            ServerMutationActor::Officer {
                role: OfficerRole::Loremaster
            },
            &research
        ),
        ActorActionAuthorityClassification::RejectOfficerPlayerEnvelope
    );
    assert_eq!(
        classify_actor_action_authority(
            ServerMutationActor::Officer {
                role: OfficerRole::Accountant
            },
            &research
        ),
        ActorActionAuthorityClassification::RejectOfficerOutOfDomainMutation
    );

    let other_session = identity::signed_session("session-routing-other".to_owned(), secret);
    let other_authorized = validate(
        &boost_envelope(&other_session.player_id, "colony:home"),
        &other_session,
        secret,
        &directory,
    )
    .expect("second player controls global colony");
    assert_eq!(
        check_actor_action_authority(
            ServerMutationActor::AuthenticatedPlayer(other_authorized.verified_session()),
            authorized.envelope()
        ),
        Err(ServerActionConflict::Unauthorized)
    );
}

fn version_hint() -> CurrentVersionHint {
    CurrentVersionHint {
        planner_version: Some(2),
        domain_version: Some(3),
        resource_version: Some(4),
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
    }
}

fn state_hint() -> CurrentStateHint {
    CurrentStateHint {
        state_code: text("refresh_required"),
        visible_entity_id: Some(entity("plan:one")),
        visible_stage: Some(text("visible")),
    }
}

fn accepted_response(authorized: &AuthorizedMutation) -> LeaderAiActionResponse {
    LeaderAiActionResponse {
        protocol_version: ActionProtocolVersion::current(),
        idempotency_id: authorized.envelope().idempotency_id.clone(),
        colony_id: authorized.envelope().colony_id.clone(),
        result: LeaderAiActionResult::Accepted {
            accepted: ActionAcceptedResult {
                result_code: text("accepted"),
                changed_ids: vec![entity("plan:one")],
                committed_versions: version_hint(),
                current_state_hint: Some(state_hint()),
            },
        },
        refresh: None,
    }
}

struct RecordingExecutor {
    steps: Vec<&'static str>,
    replay: bool,
}

impl OrderedMutationExecutor for RecordingExecutor {
    fn check_expected_state_versions(
        &mut self,
        authorized: &AuthorizedMutation,
        expected: ExpectedServerStateVersions<'_>,
    ) -> Result<(), ServerActionConflict> {
        self.steps.push("versions");
        assert_eq!(
            expected.expected().expected_planner_version,
            authorized
                .envelope()
                .expected_versions
                .expected_planner_version
        );
        Ok(())
    }

    fn check_bounded_idempotent_replay(
        &mut self,
        authorized: &AuthorizedMutation,
    ) -> Result<Option<LeaderAiActionResponse>, ServerActionConflict> {
        self.steps.push("replay");
        Ok(self.replay.then(|| accepted_response(authorized)))
    }

    fn check_current_preconditions(
        &mut self,
        _authorized: &AuthorizedMutation,
    ) -> Result<(), ServerActionConflict> {
        self.steps.push("preconditions");
        Ok(())
    }

    fn commit_atomic_favor_reservation_state(
        &mut self,
        authorized: &AuthorizedMutation,
    ) -> Result<LeaderAiActionResponse, ServerActionConflict> {
        self.steps.push("commit");
        Ok(accepted_response(authorized))
    }
}

#[test]
fn remaining_executor_interfaces_enforce_version_replay_precondition_commit_order() {
    let secret = "routing-secret";
    let session = signed_fixture(secret);
    let directory = Directory::default().with("colony:home", ColonyControlPolicy::GlobalVillage);
    let authorized = validate(
        &envelope(&session.player_id, "colony:home"),
        &session,
        secret,
        &directory,
    )
    .expect("foundation passes");

    let mut ordinary = RecordingExecutor {
        steps: Vec::new(),
        replay: false,
    };
    LeaderAiServerMutationPipeline::execute_remaining(&authorized, &mut ordinary)
        .expect("executor completes");
    assert_eq!(
        ordinary.steps,
        ["versions", "replay", "preconditions", "commit"]
    );

    let mut replay = RecordingExecutor {
        steps: Vec::new(),
        replay: true,
    };
    LeaderAiServerMutationPipeline::execute_remaining(&authorized, &mut replay)
        .expect("replay returns prior result");
    assert_eq!(replay.steps, ["versions", "replay"]);
}

#[test]
fn conflict_projection_is_bounded_report_safe_and_existence_safe() {
    let secret = "routing-secret";
    let session = signed_fixture(secret);
    let action = envelope(&session.player_id, "colony:home");
    let refresh = StaleClientRefresh {
        current_versions: version_hint(),
        current_state_hint: state_hint(),
    };
    let projected = project_server_action_response(
        &action,
        &ServerActionConflict::VersionMismatch(Box::new(refresh)),
    );
    let encoded = serde_json::to_string(match &projected {
        ServerActionResult::Action(response) => response,
        ServerActionResult::UpdateRequired(_) | ServerActionResult::ProtocolError(_) => {
            panic!("expected action response")
        }
    })
    .expect("serialize response");
    for forbidden in [
        "session",
        "signature",
        "secret",
        "exactStock",
        "regeneration",
        "reservationLoser",
        "foreignPrivate",
    ] {
        assert!(!encoded.contains(forbidden));
    }

    let missing =
        project_server_action_response(&action, &ServerActionConflict::OpaqueExistenceDenied);
    let foreign = project_server_action_response(&action, &ServerActionConflict::OwnershipDenied);
    let missing_response = match missing {
        ServerActionResult::Action(response) => response,
        ServerActionResult::UpdateRequired(_) | ServerActionResult::ProtocolError(_) => {
            panic!("expected action response")
        }
    };
    let foreign_response = match foreign {
        ServerActionResult::Action(response) => response,
        ServerActionResult::UpdateRequired(_) | ServerActionResult::ProtocolError(_) => {
            panic!("expected action response")
        }
    };
    assert_eq!(
        serde_json::to_string(&missing_response).expect("serialize missing"),
        serde_json::to_string(&foreign_response).expect("serialize foreign")
    );

    let update = serde_json::to_value(UpdateRequiredResponse::current()).expect("serialize update");
    assert_eq!(
        update,
        serde_json::json!({
            "code": "UPDATE_REQUIRED",
            "minimumSupportedVersion": cat_protocol::PROTOCOL_VERSION,
            "currentProtocolVersion": cat_protocol::PROTOCOL_VERSION
        })
    );
}

fn current_versions(expected: &ExpectedStateVersions) -> CurrentVersionHint {
    CurrentVersionHint {
        planner_version: Some(expected.expected_planner_version),
        domain_version: Some(expected.expected_domain_version),
        resource_version: Some(expected.expected_resource_version),
        spatial_version: expected.expected_spatial_version,
        reservation_version: expected.expected_reservation_version,
        research_version: expected.expected_research_version,
        scholar_version: expected.expected_scholar_version,
        boost_version: expected.expected_boost_version,
        diplomacy_version: expected.expected_diplomacy_version,
        trade_version: expected.expected_trade_version,
        prosthetic_version: expected.expected_prosthetic_version,
        care_version: expected.expected_care_version,
        officer_version: expected.expected_officer_version,
        standing_order_version: expected.expected_standing_order_version,
    }
}

#[test]
fn exact_versions_precede_bounded_accepted_replay_and_conflicting_reuse() {
    let secret = "routing-secret";
    let session = signed_fixture(secret);
    let directory = Directory::default().with("colony:home", ColonyControlPolicy::GlobalVillage);
    let action = envelope(&session.player_id, "colony:home");
    let authorized =
        validate(&action, &session, secret, &directory).expect("authorized action fixture");
    let current = current_versions(&action.expected_versions);
    check_expected_state_versions(&action.expected_versions, &current).expect("exact versions");

    let mut stale = current;
    stale.planner_version = stale.planner_version.map(|version| version + 1);
    assert!(matches!(
        check_expected_state_versions(&action.expected_versions, &stale),
        Err(ServerActionConflict::VersionMismatch(_))
    ));

    let mut receipts = IdempotencyReceiptStore::default();
    receipts
        .record(&action, accepted_response(&authorized))
        .expect("record accepted response");
    assert_eq!(receipts.len(), 1);
    assert!(matches!(
        receipts
            .check_bounded_idempotent_replay(&action)
            .expect("exact replay"),
        IdempotencyReplay::ReplayAcceptedPriorResult(LeaderAiActionResponse {
            result: LeaderAiActionResult::DuplicateReplay { .. },
            ..
        })
    ));

    let mut conflicting = action;
    conflicting.payload = LeaderAiActionPayload::DismissIntent {
        intent_id: entity("intent:other"),
        planning_epoch: 1,
        reason: cat_protocol::DismissalReason::Superseded,
    };
    assert_eq!(
        receipts.check_bounded_idempotent_replay(&conflicting),
        Err(ServerActionConflict::MalformedActionId)
    );
}

#[test]
fn atomic_candidate_commits_once_and_dropped_candidate_cannot_mutate_live_state() {
    let mut live = BTreeMap::from([("favor", 10_u64), ("reservation", 1)]);
    {
        let mut rejected = leader_ai_action_routing::AtomicLeaderAiCommit::stage(&live);
        rejected.candidate_mut().insert("favor", 5);
        rejected.candidate_mut().insert("reservation", 2);
    }
    assert_eq!(
        live,
        BTreeMap::from([("favor", 10_u64), ("reservation", 1)])
    );

    let mut accepted = leader_ai_action_routing::AtomicLeaderAiCommit::stage(&live);
    accepted.candidate_mut().insert("favor", 5);
    accepted.candidate_mut().insert("reservation", 2);
    accepted.commit_favor_debit_once(&mut live);
    assert_eq!(live, BTreeMap::from([("favor", 5_u64), ("reservation", 2)]));
}

fn stable_id(value: &str) -> NonEmptyStableId {
    NonEmptyStableId::new(value).expect("valid stable id")
}

fn minimal_colony(colony_id: &str, reports: Vec<BeliefReportSnapshot>) -> ColonyAiSnapshot {
    let shrine_id = format!("shrine:{colony_id}");
    ColonyAiSnapshot {
        colony_id: stable_id(colony_id),
        state_version: 1,
        action_versions: Default::default(),
        capabilities: SnapshotVillageCapabilities {
            can_view: true,
            can_control: true,
            is_owner: true,
        },
        reports,
        plans: PlanQueueSnapshot {
            planner_version: 1,
            planning_epoch: 1,
            plans: Vec::new(),
        },
        officer_requests: Vec::new(),
        officer_institution: None,
        standing_orders: Vec::new(),
        refresh_hints: Vec::new(),
        visible_tasks: Vec::new(),
        cats: Vec::new(),
        shrine: ShrineSnapshot {
            shrine_id: stable_id(&shrine_id),
            endpoint: SiteRefSnapshot::Tile {
                site: SiteSnapshot {
                    site_id: stable_id(&shrine_id),
                    visibility: SiteVisibilitySnapshot::Visible,
                    lifecycle_stage: SiteLifecycleStageSnapshot::Active,
                    blocked_reason: None,
                },
                tile: SnapshotTilePoint { x: 0, y: 0 },
            },
            pipeline: None,
        },
        favor: FavorLedgerSnapshot {
            ledger_version: 1,
            micro_favor: 20,
            favor_events: Vec::new(),
        },
        research: ResearchFrontierSnapshot {
            research_version: 1,
            manifest_study_count: cat_protocol::MANIFEST_STUDY_COUNT,
            owned_study_ids: Vec::new(),
            frontier: Vec::new(),
            automatic_quota: AutomaticResearchQuotaSnapshot {
                quota_used: 0,
                quota_limit: 1,
                quota_window_started_at_ms: 0,
            },
            insight: InsightSnapshot {
                insight_balance: 0,
                generated_this_week: 0,
                week_started_at_ms: Some(0),
            },
            preparations: Vec::new(),
        },
        boosts: Vec::new(),
        diplomacy: DiplomacySnapshot {
            diplomacy_version: 1,
            relationships: Vec::new(),
        },
        trade: Vec::new(),
    }
}

#[test]
fn redactor_keeps_only_selected_colony_and_hides_regeneration_below_level_four() {
    let report = BeliefReportSnapshot {
        report_id: stable_id("report:selected"),
        report_version: 1,
        subject_id: stable_id("source:selected"),
        domain: text("resources"),
        estimate: ReportEstimateSnapshot {
            minimum: 4,
            maximum: 8,
            unit: text("units"),
        },
        confidence_basis_points: BoundedBasisPoints::new(7_500).expect("valid basis points"),
        age_ms: BoundedAgeMs::new(10),
        observed_at_ms: 10,
        expires_at_ms: 20,
        report_level: 3,
        provenance: ReportProvenanceSnapshot {
            source_report_ids: Vec::new(),
            observer_id: None,
            method: text("observation"),
        },
        contradicts_report_ids: Vec::new(),
        replaces_report_id: None,
        unavailable_reason: None,
        regeneration: RegenerationReportSnapshot::Estimated {
            level_4_or_higher: true,
            estimate: ReportEstimateSnapshot {
                minimum: 1,
                maximum: 2,
                unit: text("per_hour"),
            },
            provenance: ReportProvenanceSnapshot {
                source_report_ids: Vec::new(),
                observer_id: None,
                method: text("estimate"),
            },
        },
    };
    let snapshot = LeaderAiSnapshotEnvelope {
        protocol_version: SnapshotProtocolVersion::current(),
        schema_version: cat_protocol::LAI24_SNAPSHOT_SCHEMA_VERSION,
        now_ms: 10,
        world_seed: 7,
        selected_colony_id: stable_id("colony:selected"),
        public_villages: Vec::new(),
        colonies: vec![
            minimal_colony("colony:selected", vec![report]),
            minimal_colony("colony:foreign-private", Vec::new()),
        ],
    };
    let redacted = ServerSideSnapshotRedactor::redact_snapshot_for_authenticated_colony(
        snapshot,
        "colony:selected",
    )
    .expect("redacted snapshot validates");
    assert_eq!(redacted.colonies.len(), 1);
    assert_eq!(redacted.colonies[0].colony_id.as_str(), "colony:selected");
    assert!(matches!(
        redacted.colonies[0].reports[0].regeneration,
        RegenerationReportSnapshot::UnavailableBelowLevel4
    ));
    let encoded = serde_json::to_string(&redacted).expect("serialize redacted snapshot");
    assert!(!encoded.contains("foreign-private"));
    assert!(!encoded.contains("\"sessionId\":"));
    assert!(!encoded.contains("\"sig\":"));
}
