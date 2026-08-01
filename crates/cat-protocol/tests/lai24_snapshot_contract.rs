//! LAI.24A red protocol contract for the post-cutover report-safe snapshot.

use cat_protocol::PROTOCOL_VERSION;

const PROTOCOL: &str = concat!(
    include_str!("../src/lib.rs"),
    include_str!("../src/lai24_snapshot.rs")
);
const WIRE_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/wire-persistence-ui.md");
const SPATIAL_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/spatial-task-contract.md");
const SHRINE_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/shrine-favor-research.md");
const CATS_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/cats-and-care.md");
const DIPLOMACY_DOC: &str = include_str!("../../../docs/leader-ai-overhaul/diplomacy-trade.md");
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

#[test]
fn lai24_snapshot_envelope_is_versioned_and_fail_closed() {
    assert!(WIRE_DOC.contains("## LAI.24 snapshot schema contract"));
    let required = [
        ("LeaderAiSnapshotEnvelope", "missing LAI.24 root envelope"),
        ("schema_version", "missing explicit snapshot schema version"),
        (
            "SnapshotProtocolVersion",
            "missing typed protocol-version field",
        ),
        (
            "SnapshotDecodeError",
            "missing fail-closed snapshot decode error",
        ),
        (
            "UnsupportedProtocolVersion",
            "unknown protocol versions must fail before nested decode",
        ),
        (
            "UnknownSnapshotVariant",
            "unknown tagged variants must fail closed",
        ),
        (
            "#[serde(deny_unknown_fields)]",
            "LAI.24 DTOs must reject unknown object fields",
        ),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        PROTOCOL_VERSION > 1 && missing.is_empty(),
        "LAI.24 must ship a bumped, versioned, fail-closed snapshot envelope; protocol_version={PROTOCOL_VERSION}, missing: {missing:?}"
    );
}

#[test]
fn beliefs_reports_are_ranges_confidence_age_and_provenance_only() {
    assert!(TESTING_DOC.contains("No regeneration appears"));
    let required = [
        ("BeliefReportSnapshot", "missing report-safe belief DTO"),
        (
            "ReportEstimateSnapshot",
            "missing bounded estimate/range DTO",
        ),
        ("minimum", "missing range lower bound"),
        ("maximum", "missing range upper bound"),
        (
            "confidence_basis_points",
            "missing confidence basis-point field",
        ),
        ("age_ms", "missing report age field"),
        ("observed_at_ms", "missing observation timestamp"),
        ("expires_at_ms", "missing report expiry timestamp"),
        ("ReportProvenanceSnapshot", "missing provenance DTO"),
        (
            "RegenerationReportSnapshot",
            "missing explicit regeneration projection DTO",
        ),
        (
            "UnavailableBelowLevel4",
            "regeneration must be unavailable below report level 4",
        ),
        (
            "level_4_or_higher",
            "missing report-level gate for regeneration ranges",
        ),
    ];
    let forbidden = [
        (
            "hidden_truth",
            "hidden truth field crossed protocol boundary",
        ),
        (
            "authoritative_quantity",
            "authoritative quantity field crossed protocol boundary",
        ),
        (
            "exact_regeneration",
            "exact regeneration field crossed protocol boundary",
        ),
    ];

    let missing = missing_required(PROTOCOL, &required);
    let present = forbidden_present(PROTOCOL, &forbidden);
    assert!(
        missing.is_empty() && present.is_empty(),
        "belief/report snapshots must be report-safe ranges with provenance and no hidden truth; missing: {missing:?}; forbidden present: {present:?}"
    );
}

#[test]
fn plans_requests_visible_tasks_and_cat_active_task_are_projected() {
    assert!(WIRE_DOC.contains("top plan queue"));
    assert!(SPATIAL_DOC.contains("`VisibleTaskSnapshot` contains"));
    let required = [
        ("PlanQueueSnapshot", "missing bounded top-plan queue DTO"),
        ("PlanSnapshot", "missing plan DTO"),
        ("plan_id", "missing stable plan ID"),
        ("intent_id", "missing intent/dependency link"),
        ("rationale", "missing plan rationale"),
        ("expected_cost", "missing expected-cost projection"),
        ("expected_benefit", "missing expected-benefit projection"),
        ("PlanReasonSnapshot", "missing bounded reason DTO"),
        ("OfficerRequestSnapshot", "missing officer request DTO"),
        ("VisibleTaskSnapshot", "missing visible task DTO"),
        ("active_task_id", "CatSnapshot missing active task ID link"),
        ("assigned_cat_ids", "visible task missing assigned cats"),
        ("objective", "visible task missing objective site"),
        ("work_slots", "visible task missing work positions/slots"),
        ("endpoint", "visible task missing pinned delivery endpoint"),
        ("progress_basis_points", "missing bounded task progress"),
        ("blocked_reason", "missing bounded task block reason"),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        missing.is_empty(),
        "LAI.24 must project plans, officer requests, visible tasks, and cat active_task_id; missing: {missing:?}"
    );
}

#[test]
fn siterefs_round_trip_all_required_physical_variants() {
    assert!(SPATIAL_DOC.contains("`SiteRef` supports"));
    let required = [
        ("SiteRefSnapshot", "missing report-safe SiteRef DTO"),
        ("Tile", "missing exact tile SiteRef variant"),
        ("AnchoredRect", "missing anchored rectangle variant"),
        (
            "OrderedTileSet",
            "missing canonically ordered tile-set variant",
        ),
        (
            "BuildingFootprint",
            "missing building ID/anchor/footprint variant",
        ),
        ("StockpileFootprint", "missing stockpile footprint variant"),
        ("ResourceSource", "missing resource-source variant"),
        ("HuntSource", "missing Hunt cave/source variant"),
        (
            "WaterSourceAndBank",
            "missing Fetch Water source/bank variant",
        ),
        ("OrderedRoute", "missing route/road segment variant"),
        ("Shrine", "missing Shrine endpoint variant"),
        ("VillageEndpoint", "missing village endpoint variant"),
        ("TradeEndpoint", "missing trade endpoint variant"),
        ("visibility", "missing SiteRef visibility"),
        ("lifecycle_stage", "missing SiteRef lifecycle stage"),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        missing.is_empty(),
        "LAI.24 SiteRef must round-trip every supported physical variant; missing: {missing:?}"
    );
}

#[test]
fn workshop_siteref_reports_three_by_three_and_all_nine_ordered_tiles() {
    assert!(WIRE_DOC.contains("`width: 3`, `height: 3`"));
    let required = [
        (
            "WorkshopFootprintSnapshot",
            "missing explicit Workshop footprint DTO",
        ),
        ("width", "Workshop footprint missing width"),
        ("height", "Workshop footprint missing height"),
        ("ordered_tiles", "Workshop footprint missing ordered tiles"),
        (
            "validate_workshop_three_by_three",
            "missing width/height 3 validator",
        ),
        (
            "validate_nine_row_major_tiles",
            "missing nine row-major tile validator",
        ),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        missing.is_empty(),
        "LAI.24 Workshop SiteRef must encode width 3, height 3, and all nine row-major tiles; missing: {missing:?}"
    );
}

#[test]
fn cat_traits_stress_anatomy_injuries_and_prosthetics_are_projected() {
    assert!(CATS_DOC.contains("Cat care UI shows"));
    let required = [
        ("CatTraitsSnapshot", "missing innate/acquired traits DTO"),
        ("CatPersonalitySnapshot", "missing personality DTO"),
        ("StressSnapshot", "missing stress/refusal DTO"),
        ("WillingnessSnapshot", "missing willingness breakdown DTO"),
        ("AnatomySnapshot", "missing anatomy DTO"),
        ("BodyPartSnapshot", "missing body-part DTO"),
        ("InjurySnapshot", "missing injury DTO"),
        ("TreatmentSnapshot", "missing treatment DTO"),
        ("ProstheticSnapshot", "missing prosthetic DTO"),
        (
            "ProstheticWearSnapshot",
            "missing durability/wear projection DTO",
        ),
        ("CareStatusSnapshot", "missing care status DTO"),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        missing.is_empty(),
        "LAI.24 cat snapshots must include traits, stress, anatomy, injuries, prosthetics, and care status; missing: {missing:?}"
    );
}

#[test]
fn shrine_favor_research_insight_preparation_and_boosts_are_exact() {
    assert!(SHRINE_DOC.contains("exact Favor ledger summary"));
    let required = [
        (
            "ShrineOfferingPipelineSnapshot",
            "missing endless Shrine pipeline DTO",
        ),
        ("OfferingStageSnapshot", "missing offering stage DTO"),
        ("OfferingPackageSnapshot", "missing offering package DTO"),
        ("FavorLedgerSnapshot", "missing exact Favor ledger DTO"),
        ("micro_favor", "Favor must use exact micro-Favor units"),
        ("favor_events", "missing Favor ledger event summary"),
        ("ResearchFrontierSnapshot", "missing research frontier DTO"),
        (
            "MANIFEST_STUDY_COUNT: usize = 531",
            "missing 531-study bound",
        ),
        (
            "AutomaticResearchQuotaSnapshot",
            "missing rolling seven-day quota DTO",
        ),
        ("quota_used", "missing used quota"),
        ("quota_limit", "missing quota limit"),
        ("quota_window_started_at_ms", "missing quota window anchor"),
        ("InsightSnapshot", "missing Insight DTO"),
        (
            "ScholarPreparationSnapshot",
            "missing scholar preparation DTO",
        ),
        ("DivineBoostSnapshot", "missing active boost DTO"),
        ("boost_price_micro_favor", "missing exact boost price"),
        ("boost_expires_at_ms", "missing exact boost expiry"),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        missing.is_empty(),
        "LAI.24 must project Shrine/Favor/research/Insight/preparation/boost state; missing: {missing:?}"
    );
}

#[test]
fn diplomacy_and_physical_trade_are_report_safe_and_multi_colony_public() {
    assert!(DIPLOMACY_DOC.contains("Snapshots expose relationship/consent state"));
    let required = [
        ("DiplomacySnapshot", "missing diplomacy DTO"),
        ("RelationshipSnapshot", "missing relationship DTO"),
        ("ConsentSnapshot", "missing mutual consent DTO"),
        (
            "TradeContractSnapshot",
            "missing physical trade contract DTO",
        ),
        ("TradeEscrowSnapshot", "missing trade escrow DTO"),
        ("TradeCargoSnapshot", "missing finite cargo DTO"),
        ("TradeRouteSnapshot", "missing physical route DTO"),
        ("TradeStageSnapshot", "missing trade stage DTO"),
        ("relationship_version", "missing relationship version"),
        ("contract_version", "missing trade contract version"),
        ("bounded_failure", "missing bounded trade failure reason"),
    ];

    let missing = missing_required(PROTOCOL, &required);
    assert!(
        missing.is_empty(),
        "LAI.24 diplomacy/trade snapshots must expose only public relationship and physical contract facts; missing: {missing:?}"
    );
}

#[test]
fn bounds_unknown_variants_and_private_colony_state_fail_closed() {
    assert!(WIRE_DOC.contains("Multi-colony snapshots/actions expose"));
    let required = [
        (
            "validate_lai24_snapshot_bounds",
            "missing aggregate snapshot bounds validator",
        ),
        (
            "BoundedBasisPoints",
            "missing shared 0..=10000 basis-point bound",
        ),
        ("BoundedAgeMs", "missing nonnegative report-age bound"),
        ("NonEmptyStableId", "missing stable non-empty ID bound"),
        ("ReportSafeString", "missing bounded display/error string"),
        (
            "reject_private_colony_state",
            "missing private multi-colony state redaction guard",
        ),
        (
            "PrivateColonyStateSnapshot",
            "private colony state must be represented only as an absent/forbidden guard",
        ),
    ];
    let forbidden = [
        ("owner_session_id", "private owner session ID leaked"),
        ("hmac", "authentication secret material leaked"),
        ("private_beliefs", "another colony's private beliefs leaked"),
        (
            "hidden_inventory",
            "another colony's hidden inventory leaked",
        ),
        ("private_plans", "another colony's private plans leaked"),
    ];

    let missing = missing_required(PROTOCOL, &required);
    let present = forbidden_present(PROTOCOL, &forbidden);
    assert!(
        missing.is_empty() && present.is_empty(),
        "LAI.24 must enforce bounds, reject unknown variants, and omit multi-colony private state; missing: {missing:?}; forbidden present: {present:?}"
    );
}
