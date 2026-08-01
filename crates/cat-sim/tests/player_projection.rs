use cat_sim::{
    beliefs::{
        BeliefKey, BeliefKind, BeliefStore, BeliefValue, Confidence, EstimateRange, EvidenceId,
        ExecutionFeedback, Observation, ProjectedBeliefValue, ReportLevel,
    },
    planner_core::PlannerId,
    player_projection::{
        ForbiddenExecutorField, PLAYER_PROJECTION_SCHEMA_VERSION, PlayerProjection, PlayerSurface,
    },
};

const TICKS_PER_HOUR: u64 = 60;

fn id(namespace: &str, value: &str) -> PlannerId {
    PlannerId::derive(namespace, [value])
}

fn regeneration_key() -> BeliefKey {
    BeliefKey::new(
        id("domain", "resources"),
        id("site", "north-spring"),
        BeliefKind::Regeneration,
    )
}

fn regeneration_observation(level: ReportLevel, estimate: i64) -> Observation {
    let key = regeneration_key();
    let reporter = id("cat", "accountant");
    Observation::new(
        EvidenceId::derive("colony-1", &key, 10, &reporter, 0),
        key,
        BeliefValue::Estimate(
            EstimateRange::around(estimate, level.regeneration_error_basis_points().unwrap())
                .unwrap(),
        ),
        Confidence::new(8_000).unwrap(),
        10,
        TICKS_PER_HOUR,
        reporter,
        level,
    )
    .unwrap()
}

#[test]
fn every_player_surface_rejects_every_authoritative_executor_field() {
    for surface in PlayerSurface::ALL {
        for field in ForbiddenExecutorField::ALL {
            assert!(surface.rejects(field), "{surface:?} exposed {field:?}");
        }
    }
}

#[test]
fn projection_contains_exact_favor_but_only_report_safe_beliefs() {
    let mut beliefs = BeliefStore::new();
    beliefs.apply_observation(regeneration_observation(ReportLevel::Four, 20));

    let projection = PlayerProjection::from_beliefs(
        id("colony", "one"),
        &beliefs,
        73,
        10,
        [ExecutionFeedback::SourceUnavailable],
    );

    assert_eq!(projection.schema_version, PLAYER_PROJECTION_SCHEMA_VERSION);
    assert_eq!(projection.favor_balance, 73);
    assert_eq!(projection.belief_version, beliefs.version);
    assert_eq!(projection.beliefs.len(), 1);
    assert!(matches!(
        projection.beliefs[0].value,
        ProjectedBeliefValue::RegenerationRange(_)
    ));
    assert_eq!(
        projection.feedback,
        vec![ExecutionFeedback::SourceUnavailable]
    );

    let json = serde_json::to_value(&projection).unwrap();
    assert_eq!(json["favorBalance"], 73);
    let encoded = json.to_string();
    for forbidden in [
        "authoritativeStock",
        "authoritativeProduction",
        "authoritativeConsumption",
        "sourceCapacity",
        "depletion",
        "authoritativeRegeneration",
        "undiscoveredSite",
        "unseenThreat",
        "otherColonyBeliefs",
        "otherColonyInventory",
        "otherColonyPlans",
    ] {
        assert!(!encoded.contains(forbidden), "serialized leak: {forbidden}");
    }
}

#[test]
fn hidden_regeneration_truth_cannot_change_player_projection_without_a_report() {
    let beliefs = BeliefStore::new();
    let project_hidden_twin = |hidden_executor_regeneration: i64| {
        let _executor_only = hidden_executor_regeneration;
        PlayerProjection::from_beliefs(id("colony", "one"), &beliefs, 11, 0, std::iter::empty())
    };

    assert_eq!(project_hidden_twin(1), project_hidden_twin(1_000_000));
    assert!(project_hidden_twin(1).beliefs.is_empty());
}

#[test]
fn valid_level_four_report_is_the_first_player_visible_regeneration_shape() {
    for level in [ReportLevel::One, ReportLevel::Two, ReportLevel::Three] {
        let key = regeneration_key();
        let reporter = id("cat", "accountant");
        let attempted = Observation::new(
            EvidenceId::derive("colony-1", &key, 0, &reporter, u32::from(u8::from(level))),
            key,
            BeliefValue::Estimate(EstimateRange::around(20, 2_500).unwrap()),
            Confidence::new(8_000).unwrap(),
            0,
            TICKS_PER_HOUR,
            reporter,
            level,
        );
        assert!(attempted.is_err());
    }

    let mut beliefs = BeliefStore::new();
    beliefs.apply_observation(regeneration_observation(ReportLevel::Four, 20));
    let projection =
        PlayerProjection::from_beliefs(id("colony", "one"), &beliefs, 0, 10, std::iter::empty());
    let ProjectedBeliefValue::RegenerationRange(range) = &projection.beliefs[0].value else {
        panic!("regeneration must remain an estimate range");
    };
    assert_eq!(
        (range.lower_bound, range.estimate, range.upper_bound),
        (15, 20, 25)
    );
}

#[test]
fn debug_output_is_the_same_report_safe_projection() {
    let projection = PlayerProjection::from_beliefs(
        id("colony", "one"),
        &BeliefStore::new(),
        5,
        0,
        [ExecutionFeedback::RouteBlocked],
    );
    let debug = format!("{projection:?}");
    assert!(debug.contains("favor_balance: 5"));
    assert!(debug.contains("RouteBlocked"));
    assert!(!debug.contains("regeneration_rate"));
    assert!(!debug.contains("hidden"));
}
