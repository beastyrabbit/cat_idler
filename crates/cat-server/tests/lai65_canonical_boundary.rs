use std::collections::BTreeMap;

use cat_protocol::PROTOCOL_VERSION;
use cat_protocol::lai64::{
    ActionOutcome, ActionReceipt, CANONICAL_ACTION_SCHEMA_VERSION, CanonicalActionEnvelope,
    CanonicalGodAction, PersonalStance, ReportText, StableId, VersionExpectation, VersionLane,
};
use cat_server::lai65::{
    CanonicalBoundaryError, CanonicalColonyAccess, CanonicalColonyDirectory, CanonicalGodDispatch,
    CanonicalIngress, CanonicalReplayStore, CanonicalServerBuild, CanonicalVersionSource,
    HoleClickRateLimiter, SignedTestResetGate, TestResetSignatureVerifier, TrustedCanonicalSession,
    TwoStepSignedTestResetGate, admit_canonical_action, authorize_canonical_action,
};

fn id(value: &str) -> StableId {
    StableId::new(value).expect("valid fixture identifier")
}

fn text(value: &str) -> ReportText {
    ReportText::new(value).expect("valid fixture report")
}

#[derive(Default)]
struct Directory(BTreeMap<StableId, CanonicalColonyAccess>);

impl CanonicalColonyDirectory for Directory {
    fn selected_colony_access(&self, colony_id: &StableId) -> Option<CanonicalColonyAccess> {
        self.0.get(colony_id).cloned()
    }
}

#[derive(Default)]
struct Versions(BTreeMap<(StableId, VersionLane), u64>);

impl CanonicalVersionSource for Versions {
    fn current_version(&self, colony_id: &StableId, lane: VersionLane) -> Option<u64> {
        self.0.get(&(colony_id.clone(), lane)).copied()
    }
}

struct ResetVerifier;
impl TestResetSignatureVerifier for ResetVerifier {
    fn verify_first_step(
        &self,
        _session: &TrustedCanonicalSession,
        _nonce: &StableId,
        signature: &ReportText,
    ) -> bool {
        signature.as_str() == "fixture_signature"
    }
}

struct RejectingGate;
impl SignedTestResetGate for RejectingGate {
    fn consume_second_confirmation(
        &mut self,
        _session: &TrustedCanonicalSession,
        _nonce: &StableId,
        _signature: &ReportText,
        _confirmation: &ReportText,
        _now_ms: i64,
    ) -> Result<(), CanonicalBoundaryError> {
        Err(CanonicalBoundaryError::ResetNotStaged)
    }
}

fn session() -> TrustedCanonicalSession {
    TrustedCanonicalSession::new(id("session:fixture"), id("player:fixture"), 10_000)
        .expect("trusted session")
}

fn directory() -> Directory {
    Directory(BTreeMap::from([(
        id("colony:home"),
        CanonicalColonyAccess::PersonalVillage {
            owner_player_id: id("player:fixture"),
        },
    )]))
}

fn envelope(action: CanonicalGodAction, lanes: Vec<VersionExpectation>) -> CanonicalActionEnvelope {
    CanonicalActionEnvelope {
        protocol_version: PROTOCOL_VERSION,
        action_schema_version: CANONICAL_ACTION_SCHEMA_VERSION,
        authenticated_player_id: id("player:fixture"),
        selected_colony_id: id("colony:home"),
        idempotency_id: id("action:fixture"),
        expected_versions: lanes,
        payload: action,
    }
}

#[test]
fn trusted_session_identity_and_personal_village_authority_are_both_required() {
    let mut wrong_player = envelope(
        CanonicalGodAction::Inspiration,
        vec![VersionExpectation {
            lane: VersionLane::Divine,
            expected_version: 7,
        }],
    );
    wrong_player.authenticated_player_id = id("player:forged");
    assert_eq!(
        authorize_canonical_action(session(), wrong_player, &directory(), 1_000),
        Err(CanonicalBoundaryError::PayloadPlayerMismatch)
    );

    let mut foreign = directory();
    foreign.0.insert(
        id("colony:home"),
        CanonicalColonyAccess::PersonalVillage {
            owner_player_id: id("player:other"),
        },
    );
    assert_eq!(
        authorize_canonical_action(
            session(),
            envelope(
                CanonicalGodAction::Inspiration,
                vec![VersionExpectation {
                    lane: VersionLane::Divine,
                    expected_version: 7,
                }],
            ),
            &foreign,
            1_000,
        ),
        Err(CanonicalBoundaryError::SelectedColonyDenied)
    );

    let global = Directory(BTreeMap::from([(
        id("colony:home"),
        CanonicalColonyAccess::GlobalVillage,
    )]));
    assert_eq!(
        authorize_canonical_action(
            session(),
            envelope(
                CanonicalGodAction::PersonalStance {
                    other_colony_id: id("colony:other"),
                    stance: PersonalStance::Enemy,
                },
                vec![VersionExpectation {
                    lane: VersionLane::Diplomacy,
                    expected_version: 1,
                }],
            ),
            &global,
            1_000,
        ),
        Err(CanonicalBoundaryError::SelectedColonyDenied)
    );
}

#[test]
fn replay_is_exact_and_does_not_consume_more_hole_clicks() {
    let action = envelope(
        CanonicalGodAction::HoleClickBatch {
            target_id: id("hole:home"),
            requested_clicks: 20,
            client_batch_window_ms: 100,
        },
        vec![
            VersionExpectation {
                lane: VersionLane::Hole,
                expected_version: 3,
            },
            VersionExpectation {
                lane: VersionLane::Divine,
                expected_version: 2,
            },
            VersionExpectation {
                lane: VersionLane::Reservations,
                expected_version: 1,
            },
        ],
    );
    let authorized = authorize_canonical_action(session(), action.clone(), &directory(), 1_000)
        .expect("authorized");
    let versions = Versions(BTreeMap::from([
        ((id("colony:home"), VersionLane::Hole), 3),
        ((id("colony:home"), VersionLane::Divine), 2),
        ((id("colony:home"), VersionLane::Reservations), 1),
    ]));
    let mut store = CanonicalReplayStore::default();
    let mut limiter = HoleClickRateLimiter::default();
    let mut gate = RejectingGate;
    let admitted = admit_canonical_action(
        authorized.clone(),
        &versions,
        CanonicalServerBuild::Production,
        &mut gate,
        &store,
        &mut limiter,
        1_000,
    )
    .expect("first request dispatches");
    assert!(matches!(
        admitted,
        CanonicalIngress::Dispatch(action)
            if matches!(
                action.dispatch(),
                CanonicalGodDispatch::HoleClickBatch {
                    requested_clicks: 20,
                    accepted_clicks: 20,
                    ..
                }
            )
    ));
    store
        .record(
            &authorized,
            ActionReceipt {
                idempotency_id: id("action:fixture"),
                selected_colony_id: id("colony:home"),
                outcome: ActionOutcome::Accepted,
                changed_ids: vec![id("hole:home")],
                reason: None,
                committed_versions: vec![
                    VersionExpectation {
                        lane: VersionLane::Hole,
                        expected_version: 4,
                    },
                    VersionExpectation {
                        lane: VersionLane::Divine,
                        expected_version: 3,
                    },
                    VersionExpectation {
                        lane: VersionLane::Reservations,
                        expected_version: 2,
                    },
                ],
            },
            1_000,
        )
        .expect("recorded");
    let replay = admit_canonical_action(
        authorized,
        &versions,
        CanonicalServerBuild::Production,
        &mut gate,
        &store,
        &mut limiter,
        1_001,
    )
    .expect("replay succeeds despite the already full Hole bucket");
    assert!(
        matches!(replay, CanonicalIngress::Replay(receipt) if receipt.outcome == ActionOutcome::Replayed)
    );

    let mut conflicting = action;
    conflicting.payload = CanonicalGodAction::HoleClickBatch {
        target_id: id("hole:home"),
        requested_clicks: 1,
        client_batch_window_ms: 100,
    };
    let conflicting = authorize_canonical_action(session(), conflicting, &directory(), 1_001)
        .expect("same identity can reach replay guard");
    assert_eq!(
        admit_canonical_action(
            conflicting,
            &versions,
            CanonicalServerBuild::Production,
            &mut gate,
            &store,
            &mut limiter,
            1_001,
        ),
        Err(CanonicalBoundaryError::ReplayConflict)
    );
}

#[test]
fn hole_limiter_is_per_player_and_target_and_limits_click_count_not_action_count() {
    let mut limiter = HoleClickRateLimiter::default();
    let player = id("player:fixture");
    let hole = id("hole:home");
    assert_eq!(
        limiter
            .admit(&player, &hole, 64, 1_000)
            .expect("a large DTO batch is partially admitted"),
        20
    );
    assert!(matches!(
        limiter.admit(&player, &hole, 1, 1_001),
        Err(CanonicalBoundaryError::RateLimited { .. })
    ));
    assert_eq!(
        limiter
            .admit(&player, &id("hole:other"), 20, 1_001)
            .expect("other Hole has an independent target bucket"),
        20
    );
    assert_eq!(
        limiter
            .admit(&id("player:other"), &hole, 20, 1_001)
            .expect("other player has an independent bucket"),
        20
    );
    assert_eq!(
        limiter
            .admit(&player, &hole, 1, 2_000)
            .expect("one-second window expires exactly at its boundary"),
        1
    );
}

#[test]
fn signed_reset_needs_test_build_and_consumable_two_step_challenge() {
    let action = envelope(
        CanonicalGodAction::SignedTestReset {
            nonce: id("reset:nonce"),
            signature: text("fixture_signature"),
            confirmation: text("test_reset_confirmed"),
        },
        vec![],
    );
    let authorized = authorize_canonical_action(session(), action, &directory(), 1_000)
        .expect("canonical reset DTO is valid");
    let versions = Versions::default();
    let store = CanonicalReplayStore::default();
    let mut limiter = HoleClickRateLimiter::default();
    let mut gate = TwoStepSignedTestResetGate::default();
    assert_eq!(
        admit_canonical_action(
            authorized.clone(),
            &versions,
            CanonicalServerBuild::Production,
            &mut gate,
            &store,
            &mut limiter,
            1_000,
        ),
        Err(CanonicalBoundaryError::SignedTestResetDisabled)
    );
    gate.stage_first_step(
        &session(),
        id("reset:nonce"),
        text("fixture_signature"),
        1_000,
        &ResetVerifier,
    )
    .expect("signed first step");
    assert!(matches!(
        admit_canonical_action(
            authorized.clone(),
            &versions,
            CanonicalServerBuild::TestBuild,
            &mut gate,
            &store,
            &mut limiter,
            1_001,
        ),
        Ok(CanonicalIngress::Dispatch(_))
    ));
    assert_eq!(
        admit_canonical_action(
            authorized,
            &versions,
            CanonicalServerBuild::TestBuild,
            &mut gate,
            &store,
            &mut limiter,
            1_002,
        ),
        Err(CanonicalBoundaryError::ResetNotStaged)
    );
}

#[test]
fn strict_rows_round_trip_without_server_secret_or_unknown_fields() {
    let row = cat_server::lai65::CanonicalHoleClickRateRow {
        row_schema_version: 1,
        authenticated_player_id: id("player:fixture"),
        target_id: id("hole:home"),
        hit_timestamps_ms: vec![1_000, 1_000],
    };
    let encoded = row.encode_json().expect("encode row");
    assert_eq!(
        cat_server::lai65::CanonicalHoleClickRateRow::decode_json(&encoded),
        Ok(row)
    );
    let unknown = encoded.replacen('}', ",\"secret\":\"no\"}", 1);
    assert_eq!(
        cat_server::lai65::CanonicalHoleClickRateRow::decode_json(&unknown),
        Err(CanonicalBoundaryError::PersistenceCodec)
    );

    let rescue = envelope(
        CanonicalGodAction::EmergencyRescue {
            witness_id: id("rescue:witness:water"),
        },
        vec![
            VersionExpectation {
                lane: VersionLane::Storage,
                expected_version: 1,
            },
            VersionExpectation {
                lane: VersionLane::Divine,
                expected_version: 1,
            },
            VersionExpectation {
                lane: VersionLane::Reservations,
                expected_version: 1,
            },
        ],
    );
    assert!(rescue.validate().is_ok());
}
