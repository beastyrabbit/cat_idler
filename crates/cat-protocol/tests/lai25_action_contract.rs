//! LAI.25A red action protocol contract for post-cutover mutations.

use cat_protocol::{BoundedActionId, BoundedEntityId, BoundedPlayerId, PROTOCOL_VERSION};

const PROTOCOL: &str = concat!(
    include_str!("../src/lib.rs"),
    include_str!("../src/lai25_action.rs")
);
const WIRE_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/wire-persistence-ui.md");
const TESTING_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/testing-cutover.md");

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
        let haystack = &source[cursor..];
        let Some(offset) = haystack.find(needle) else {
            panic!("missing ordered action check step `{needle}`: {reason}");
        };
        cursor += offset + needle.len();
    }
}

#[test]
fn lai25_mutation_envelope_is_versioned_authenticated_and_idempotent() {
    assert!(WIRE_DOC.contains("## LAI.25 action protocol contract"));
    let required = [
        (
            "LeaderAiActionEnvelope",
            "missing post-cutover action envelope",
        ),
        (
            "ActionProtocolVersion",
            "missing typed action protocol version",
        ),
        ("protocol_version", "missing protocol-version field"),
        ("ActionIdempotencyId", "missing stable idempotency ID type"),
        ("idempotency_id", "missing idempotency ID field"),
        ("SelectedColonyId", "missing selected colony identity type"),
        ("colony_id", "missing colony identity field"),
        ("AuthenticatedPlayerId", "missing player identity type"),
        ("player_id", "missing player identity field"),
        (
            "ExpectedStateVersions",
            "missing aggregate expected-version block",
        ),
        (
            "expected_planner_version",
            "missing expected planner version",
        ),
        ("expected_domain_version", "missing expected domain version"),
        (
            "expected_resource_version",
            "missing expected Favor/resource version",
        ),
        (
            "#[serde(deny_unknown_fields)]",
            "mutation payloads must reject unknown fields",
        ),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        PROTOCOL_VERSION > 1 && missing.is_empty(),
        "LAI.25 mutations must use a bumped, versioned, authenticated, idempotent envelope; protocol_version={PROTOCOL_VERSION}, missing: {missing:?}"
    );
}

#[test]
fn lai25_action_payload_covers_plans_orders_officers_care_research_boosts_diplomacy_and_trade() {
    let required = [
        (
            "LeaderAiActionPayload",
            "missing tagged LAI.25 action payload",
        ),
        ("NudgePlan", "missing plan nudge action"),
        (
            "CreateStandingOrder",
            "missing standing order create action",
        ),
        (
            "UpdateStandingOrder",
            "missing standing order update action",
        ),
        (
            "DeleteStandingOrder",
            "missing standing order delete action",
        ),
        ("DismissIntent", "missing intent dismissal action"),
        ("AppointOfficer", "missing officer appointment action"),
        ("UnappointOfficer", "missing officer removal action"),
        (
            "OfficerAuthorityOverride",
            "missing officer authority action",
        ),
        ("RequestTreatment", "missing treatment action"),
        ("FitProsthetic", "missing prosthetic fitting action"),
        ("RepairProsthetic", "missing prosthetic repair action"),
        (
            "PurchaseResearchWithFavor",
            "missing Favor research purchase action",
        ),
        ("PrepareScholarStudy", "missing scholar preparation action"),
        ("ActivateDivineBoost", "missing player-only boost action"),
        ("ChangeDiplomacy", "missing diplomacy relationship action"),
        ("ApproveAlliance", "missing alliance consent action"),
        ("BlockColony", "missing immediate block action"),
        ("AcceptTradeContract", "missing consent trade accept action"),
        ("RejectTradeContract", "missing consent trade reject action"),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        missing.is_empty(),
        "LAI.25 action payload must cover every new mutation domain; missing: {missing:?}"
    );
}

#[test]
fn existing_physical_placement_actions_are_wrapped_by_the_same_contract() {
    let legacy_actions = [
        (
            "PlanBuilding",
            "existing building placement action vanished",
        ),
        ("DesignateFarm", "existing farm placement action vanished"),
        (
            "DesignateStockpile",
            "existing stockpile placement action vanished",
        ),
        (
            "DesignateGatherSpot",
            "existing gather-spot placement action vanished",
        ),
        (
            "DesignateFishingSpot",
            "existing fishing-spot placement action vanished",
        ),
        ("BuildRoad", "existing road placement action vanished"),
        ("BuildBridge", "existing bridge placement action vanished"),
        ("DesignateRail", "existing rail placement action vanished"),
        ("BuildDock", "existing dock placement action vanished"),
        (
            "CreateTransportRoute",
            "existing transport route placement action vanished",
        ),
    ];
    assert!(
        missing_required(PROTOCOL, &legacy_actions).is_empty(),
        "legacy physical placement actions should remain as domains but move under the LAI.25 envelope"
    );

    let required = [
        (
            "PhysicalPlacementActionPayload",
            "missing wrapped physical placement payload",
        ),
        (
            "expected_spatial_version",
            "placement actions need expected spatial version",
        ),
        (
            "expected_reservation_version",
            "placement actions need expected reservation version",
        ),
        (
            "PlacementBounds",
            "missing strict placement bounds validator",
        ),
        ("SiteRefActionTarget", "missing typed placement site target"),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        missing.is_empty(),
        "existing physical placement actions must be wrapped by the same LAI.25 version/idempotency/bounds contract; missing: {missing:?}"
    );
}

#[test]
fn authoritative_mutation_check_order_is_encoded_once() {
    assert!(TESTING_DOC.contains("Every action enforces protocol"));
    let required = [
        (
            "ActionValidationPipeline",
            "missing single action validation pipeline",
        ),
        (
            "check_protocol_compatibility",
            "missing protocol compatibility step",
        ),
        ("check_authentication", "missing authentication step"),
        ("check_colony_ownership", "missing colony ownership step"),
        ("check_action_authority", "missing action authority step"),
        ("check_expected_versions", "missing expected-version step"),
        ("check_duplicate_replay", "missing duplicate replay step"),
        ("check_current_preconditions", "missing precondition step"),
        ("commit_favor_or_reservation", "missing atomic commit step"),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        missing.is_empty(),
        "LAI.25 must expose the single authoritative mutation validation pipeline; missing: {missing:?}"
    );
    assert_ordered(
        PROTOCOL,
        &[
            (
                "check_protocol_compatibility",
                "protocol compatibility is first",
            ),
            ("check_authentication", "authentication follows protocol"),
            ("check_colony_ownership", "ownership follows auth"),
            ("check_action_authority", "authority follows ownership"),
            (
                "check_expected_versions",
                "expected versions follow authority",
            ),
            (
                "check_duplicate_replay",
                "duplicate replay follows versions",
            ),
            (
                "check_current_preconditions",
                "current preconditions follow replay",
            ),
            (
                "commit_favor_or_reservation",
                "Favor/reservation commit is last",
            ),
        ],
    );
}

#[test]
fn conflicts_are_typed_bounded_and_refreshable_without_hidden_truth() {
    let required = [
        ("ActionConflict", "missing typed conflict enum"),
        (
            "StaleClientRefresh",
            "missing stale-client refresh conflict",
        ),
        (
            "CurrentVersionHint",
            "missing authoritative current-version hint",
        ),
        ("CurrentStateHint", "missing bounded current-state hint"),
        ("UpdateRequired", "missing incompatible-client conflict"),
        (
            "minimum_supported_version",
            "missing minimum supported version",
        ),
        ("current_protocol_version", "missing current version hint"),
        ("Unauthorized", "missing auth/authorization conflict"),
        ("OwnershipDenied", "missing colony ownership conflict"),
        ("AuthorityDenied", "missing action authority conflict"),
        ("VersionMismatch", "missing expected-version conflict"),
        ("DuplicateReplay", "missing idempotent replay result"),
        (
            "PreconditionFailed",
            "missing current precondition conflict",
        ),
        ("InsufficientFavor", "missing Favor affordability conflict"),
        (
            "ReservationConflict",
            "missing spatial reservation conflict",
        ),
        (
            "MalformedActionId",
            "missing malformed-id fail-closed conflict",
        ),
        (
            "UnknownActionVariant",
            "missing unknown-variant fail-closed conflict",
        ),
    ];
    let forbidden = [
        ("hidden_truth", "hidden truth leaked through conflict DTO"),
        ("exact_stock", "exact stock leaked through conflict DTO"),
        (
            "exact_regeneration",
            "exact regeneration leaked through conflict DTO",
        ),
        (
            "reservation_loser",
            "private reservation loser leaked through conflict DTO",
        ),
        (
            "rejected_amount",
            "hidden rejected amount leaked through conflict DTO",
        ),
    ];

    let missing = missing_required(PROTOCOL, &required);
    let present = forbidden_present(PROTOCOL, &forbidden);
    assert!(
        missing.is_empty() && present.is_empty(),
        "LAI.25 conflicts must be typed, bounded, refreshable, and leak-safe; missing: {missing:?}; forbidden present: {present:?}"
    );
}

#[test]
fn action_payload_bounds_reject_unknown_versions_variants_and_malformed_ids() {
    assert!(WIRE_DOC.contains("fail closed before any client or server compatibility path"));
    let required = [
        (
            "validate_lai25_action_bounds",
            "missing aggregate action bounds validator",
        ),
        ("BoundedActionId", "missing bounded idempotency/action ID"),
        ("BoundedPlayerId", "missing bounded player ID"),
        ("BoundedColonyId", "missing bounded colony ID"),
        ("BoundedEntityId", "missing bounded entity ID"),
        ("BoundedBasisPointNudge", "missing bounded plan nudge"),
        (
            "BoundedStandingOrderText",
            "missing bounded standing-order text",
        ),
        ("BoundedFavorAmount", "missing bounded Favor amount"),
        ("BoundedTradeAmount", "missing bounded trade amount"),
        (
            "reject_unknown_action_version",
            "missing unknown version rejection hook",
        ),
        (
            "reject_unknown_action_variant",
            "missing unknown variant rejection hook",
        ),
        (
            "reject_malformed_idempotency_id",
            "missing malformed idempotency ID rejection hook",
        ),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        missing.is_empty(),
        "LAI.25 action payloads must enforce strict bounds and fail closed for unknown versions/variants/malformed IDs; missing: {missing:?}"
    );
}

#[test]
fn planner_component_ids_round_trip_without_weakening_principal_or_action_ids() {
    let planner_id = "planner:v1|5:study|12:research_hut";
    assert_eq!(
        BoundedEntityId::new(planner_id)
            .expect("canonical planner entity IDs are valid action targets")
            .as_str(),
        planner_id
    );
    assert!(BoundedActionId::new("action|forbidden").is_err());
    assert!(BoundedPlayerId::new("player|forbidden").is_err());
}

#[test]
fn player_only_and_authority_boundaries_are_visible_in_protocol() {
    assert!(WIRE_DOC.contains("Boost activation is player-only"));
    let required = [
        ("PlayerOnlyAction", "missing player-only action marker"),
        (
            "LeaderSimulationAuthority",
            "missing Leader simulation-authority marker",
        ),
        (
            "OfficerSimulationAuthority",
            "missing officer simulation-authority marker",
        ),
        (
            "ActivateDivineBoost",
            "boost action missing from player-only payload",
        ),
        (
            "LeaderCannotActivateBoost",
            "missing Leader boost-denial conflict",
        ),
        (
            "OfficerCannotActivateBoost",
            "missing officer boost-denial conflict",
        ),
        (
            "OfficerAppointmentAuthority",
            "missing appointment authority DTO",
        ),
        (
            "TreatmentAuthority",
            "missing treatment/prosthetic authority DTO",
        ),
        (
            "DiplomacyConsentAuthority",
            "missing diplomacy/trade consent authority DTO",
        ),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        missing.is_empty(),
        "LAI.25 must make player-only and simulation-authority boundaries explicit in protocol DTOs; missing: {missing:?}"
    );
}
