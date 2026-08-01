//! LAI.27A red server authorization/routing/redaction contract.
//!
//! These tests intentionally characterize missing post-cutover server APIs by
//! inspecting server source text. They must stay red until LAI.27 implements
//! the real routing pipeline; do not add production shims only for these names.

const MAIN: &str = include_str!("../src/main.rs");
const IDENTITY: &str = include_str!("../src/identity.rs");
const LEADER_AI_ROUTING: &str = include_str!("../src/leader_ai_action_routing.rs");
const RATE_LIMIT: &str = include_str!("../src/rate_limit.rs");
const PERSISTENCE: &str = include_str!("../src/persistence.rs");
const PROTOCOL: &str = include_str!("../../cat-protocol/src/lib.rs");
const WIRE_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/wire-persistence-ui.md");

fn server_source() -> String {
    [MAIN, IDENTITY, LEADER_AI_ROUTING, RATE_LIMIT, PERSISTENCE].join("\n")
}

fn missing_required<'a>(source: &str, required: &'a [(&str, &str)]) -> Vec<&'a str> {
    required
        .iter()
        .filter_map(|(needle, reason)| (!source.contains(needle)).then_some(*reason))
        .collect()
}

fn forbidden_present<'a>(source: &str, forbidden: &'a [(&str, &str)]) -> Vec<&'a str> {
    forbidden
        .iter()
        .filter_map(|(needle, reason)| source.contains(needle).then_some(*reason))
        .collect()
}

fn assert_ordered(source: &str, ordered: &[(&str, &str)]) {
    let mut cursor = 0;
    for (needle, reason) in ordered {
        let Some(offset) = source[cursor..].find(needle) else {
            panic!("missing ordered server routing step `{needle}`: {reason}");
        };
        cursor += offset + needle.len();
    }
}

#[test]
fn lai27_server_pipeline_is_single_authoritative_ordered_path() {
    assert!(WIRE_DOC.contains("## LAI.27 server authorization/routing/redaction contract"));
    let server = server_source();
    let required = [
        (
            "LeaderAiServerMutationPipeline",
            "missing single LAI.27 server mutation pipeline",
        ),
        (
            "check_protocol_compatibility",
            "missing protocol compatibility/UPDATE_REQUIRED step",
        ),
        (
            "check_hmac_session_authentication",
            "missing HMAC/session authentication step",
        ),
        (
            "check_selected_colony_ownership",
            "missing selected-colony ownership step",
        ),
        (
            "check_actor_action_authority",
            "missing actor/action authority step",
        ),
        (
            "check_expected_state_versions",
            "missing expected state-version step",
        ),
        (
            "check_bounded_idempotent_replay",
            "missing bounded idempotent replay step",
        ),
        (
            "check_current_preconditions",
            "missing current precondition step",
        ),
        (
            "commit_atomic_favor_reservation_state",
            "missing atomic Favor/reservation/state commit step",
        ),
    ];
    let missing = missing_required(&server, &required);
    assert!(
        missing.is_empty(),
        "LAI.27 must expose one authoritative ordered server routing/check pipeline; missing: {missing:?}"
    );
    assert_ordered(
        &server,
        &[
            ("check_protocol_compatibility", "compatibility is first"),
            (
                "check_hmac_session_authentication",
                "authentication follows compatibility",
            ),
            (
                "check_selected_colony_ownership",
                "ownership follows authentication",
            ),
            (
                "check_actor_action_authority",
                "authority follows ownership",
            ),
            (
                "check_expected_state_versions",
                "state versions follow authority",
            ),
            (
                "check_bounded_idempotent_replay",
                "duplicate replay follows versions",
            ),
            ("check_current_preconditions", "preconditions follow replay"),
            ("commit_atomic_favor_reservation_state", "commit is last"),
        ],
    );
}

#[test]
fn compatibility_update_required_and_hmac_auth_fail_before_route_selection() {
    let server = server_source();
    let required = [
        (
            "UpdateRequiredResponse",
            "old clients need a clear UPDATE_REQUIRED server response",
        ),
        ("UPDATE_REQUIRED", "missing stable incompatible-client code"),
        (
            "minimum_supported_action_protocol_version",
            "missing minimum supported version hint",
        ),
        (
            "current_action_protocol_version",
            "missing current version hint",
        ),
        (
            "VerifiedPlayerSession",
            "missing post-HMAC authenticated session type",
        ),
        (
            "reject_before_action_decode",
            "compatibility/auth must fail before action payload routing",
        ),
        (
            "constant_time_session_mac_check",
            "missing constant-time HMAC verification marker",
        ),
    ];
    let missing = missing_required(&server, &required);
    assert!(
        missing.is_empty(),
        "LAI.27 must reject incompatible or unauthenticated mutations before route selection; missing: {missing:?}"
    );
}

#[test]
fn ownership_and_actor_authority_cover_player_only_boosts_and_officer_domains() {
    let server = server_source();
    let required = [
        (
            "SelectedColonyOwnershipGuard",
            "missing selected-colony ownership guard",
        ),
        ("OwnsSelectedColony", "missing owner authorization outcome"),
        (
            "DenyForeignColonyMutation",
            "missing foreign-colony mutation denial",
        ),
        (
            "PlayerOnlyDivineBoostGuard",
            "missing player-only Divine Boost authority guard",
        ),
        (
            "RejectLeaderBoostActivation",
            "Leader must not activate boosts",
        ),
        (
            "RejectOfficerBoostActivation",
            "officers must not activate boosts",
        ),
        (
            "OfficerDomainAuthorityGuard",
            "missing officer domain limit guard",
        ),
        (
            "RejectOfficerOutOfDomainMutation",
            "officers must not mutate outside domain",
        ),
    ];
    let missing = missing_required(&server, &required);
    assert!(
        missing.is_empty(),
        "LAI.27 server authorization must enforce ownership, player-only boosts, and officer domain limits; missing: {missing:?}"
    );
}

#[test]
fn expected_versions_replay_preconditions_and_commit_are_atomic() {
    let server = server_source();
    let required = [
        (
            "ExpectedServerStateVersions",
            "missing aggregate expected-version guard",
        ),
        (
            "IdempotencyReceiptStore",
            "missing bounded idempotency receipt store",
        ),
        (
            "ReplayAcceptedPriorResult",
            "duplicate accepted action must return original result",
        ),
        (
            "ReplayRejectedPriorResult",
            "duplicate rejected action must return identical rejection",
        ),
        (
            "NoMutationBeforePreconditions",
            "stale/precondition failure must not partially mutate",
        ),
        (
            "AtomicLeaderAiCommit",
            "missing atomic commit wrapper for Favor/reservation/state",
        ),
        (
            "commit_favor_debit_once",
            "Favor debit must happen exactly once",
        ),
        (
            "commit_reservation_once",
            "reservation commit must happen exactly once",
        ),
        (
            "commit_runtime_state_once",
            "runtime state commit must happen exactly once",
        ),
    ];
    let missing = missing_required(&server, &required);
    assert!(
        missing.is_empty(),
        "LAI.27 must make expected-version, replay, precondition, and Favor/reservation/state commit atomic; missing: {missing:?}"
    );
}

#[test]
fn conflicts_are_typed_bounded_refreshable_and_existence_safe() {
    let server = server_source();
    let required = [
        (
            "ServerActionConflict",
            "missing typed server action conflict enum",
        ),
        ("ServerActionResult", "missing bounded server action result"),
        ("UpdateRequired", "missing update-required conflict"),
        ("Unauthenticated", "missing authentication conflict"),
        ("Unauthorized", "missing authorization conflict"),
        ("OwnershipDenied", "missing ownership conflict"),
        ("VersionMismatch", "missing version conflict"),
        ("DuplicateReplay", "missing replay result"),
        ("PreconditionFailed", "missing precondition conflict"),
        ("RateLimited", "missing rate-limit conflict"),
        (
            "RefreshSnapshotHint",
            "missing bounded refresh snapshot hint",
        ),
        (
            "OpaqueExistenceDenied",
            "malformed/unauthorized requests need indistinguishable denial",
        ),
    ];
    let forbidden = [
        (
            "ExactStockInConflict",
            "conflicts must not expose exact stock",
        ),
        (
            "ExactRegenerationInConflict",
            "conflicts must not expose exact regeneration",
        ),
        (
            "AuthMaterialInConflict",
            "conflicts must not expose auth material",
        ),
        (
            "ForeignColonyPrivateStateInConflict",
            "conflicts must not expose another colony private state",
        ),
    ];
    let missing = missing_required(&server, &required);
    let present = forbidden_present(&server, &forbidden);
    assert!(
        missing.is_empty() && present.is_empty(),
        "LAI.27 conflicts must be typed, bounded, refreshable, and existence-safe; missing: {missing:?}; forbidden present: {present:?}"
    );
}

#[test]
fn rate_limiting_runs_before_expensive_world_or_database_work() {
    let server = server_source();
    let required = [
        (
            "LeaderAiMutationRateLimit",
            "missing dedicated LAI.27 mutation rate limiter",
        ),
        (
            "check_rate_limit_before_world_lock",
            "rate limiting must run before world lock acquisition",
        ),
        (
            "check_rate_limit_before_database_transaction",
            "rate limiting must run before database work",
        ),
        (
            "check_rate_limit_before_snapshot_build",
            "rate limiting must run before expensive refresh snapshots",
        ),
    ];
    let missing = missing_required(&server, &required);
    assert!(
        missing.is_empty(),
        "LAI.27 rate limiting must remain before expensive world/database/snapshot work; missing: {missing:?}"
    );
    assert_ordered(
        &server,
        &[
            (
                "check_rate_limit_before_world_lock",
                "rate limit precedes world lock",
            ),
            ("world.write", "world write lock follows rate limit"),
        ],
    );
}

#[test]
fn multi_colony_isolation_and_server_side_snapshot_redaction_are_enforced() {
    let server = server_source();
    let required = [
        (
            "ServerSideSnapshotRedactor",
            "missing server-side redaction layer",
        ),
        (
            "redact_snapshot_for_authenticated_colony",
            "missing selected-colony snapshot redactor",
        ),
        (
            "redact_foreign_colony_private_beliefs",
            "missing private belief redaction",
        ),
        ("redact_private_plans", "missing private plan redaction"),
        ("redact_hidden_stock", "missing hidden-stock redaction"),
        (
            "redact_regeneration_below_l4",
            "missing exact regeneration report-level gate",
        ),
        ("redact_auth_material", "missing auth-material redaction"),
        (
            "server_redaction_before_websocket_send",
            "redaction must occur before WebSocket send",
        ),
        (
            "client_is_not_redaction_authority",
            "Bevy must not be the privacy boundary",
        ),
    ];
    let forbidden = [
        (
            "send_unredacted_world_snapshot",
            "server must not send unredacted snapshots",
        ),
        (
            "bevy_hides_private_state",
            "client must not be the redaction boundary",
        ),
    ];
    let missing = missing_required(&server, &required);
    let present = forbidden_present(&server, &forbidden);
    assert!(
        missing.is_empty() && present.is_empty(),
        "LAI.27 must enforce multi-colony redaction server-side; missing: {missing:?}; forbidden present: {present:?}"
    );
}

#[test]
fn protocol_contract_is_not_satisfied_by_legacy_action_result_or_snapshot_types() {
    let server = server_source();
    let required = [
        (
            "LeaderAiActionEnvelope",
            "server must route the LAI.25 action envelope",
        ),
        (
            "LeaderAiSnapshotEnvelope",
            "server must emit the LAI.24 snapshot envelope",
        ),
        (
            "LeaderAiServerMutationPipeline",
            "server must bind protocol DTOs to LAI.27 pipeline",
        ),
    ];
    let legacy_only = [
        (
            "ActionResult",
            "legacy ActionResult is still the only visible server action response",
        ),
        (
            "WorldSnapshot",
            "legacy WorldSnapshot is still the only visible server snapshot",
        ),
        (
            "ClientAction",
            "legacy ClientAction is still the only visible server mutation payload",
        ),
    ];
    let missing = missing_required(&(server.clone() + PROTOCOL), &required);
    let legacy = legacy_only
        .iter()
        .filter_map(|(needle, reason)| {
            (server.contains(needle) && !server.contains("LeaderAiActionEnvelope"))
                .then_some(*reason)
        })
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty() && legacy.is_empty(),
        "LAI.27 server must route new action/snapshot envelopes instead of satisfying tests with legacy DTOs; missing: {missing:?}; legacy-only: {legacy:?}"
    );
}
