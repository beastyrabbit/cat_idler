//! LAI.69 contract for the pure developer-only diagnostic trace leaf.

#[allow(dead_code)]
#[path = "../src/leader_ai_diagnostics.rs"]
mod leader_ai_diagnostics;

use std::collections::BTreeMap;

use leader_ai_diagnostics::{
    ActionOutcome, ConstructionDiagnostic, DiagnosticCounts, DiagnosticDomain, DiagnosticError,
    DiagnosticEvent, DiagnosticId, DiagnosticReasons, DiagnosticScores, DiagnosticText,
    DiagnosticVisibility, DivineDiagnostic, DivineDiagnosticKind, ElectionDiagnostic,
    HEARTBEAT_TICK_INTERVAL, HeartbeatStatus, HoleFeedDiagnostic, LeaderAiDiagnosticTrace,
    MAX_DIAGNOSTIC_CAPACITY, MAX_DIAGNOSTIC_MAP_ENTRIES, MAX_DIAGNOSTIC_TEXT_BYTES,
    MatcherDiagnostic, PersistenceActionDiagnostic, PersistenceActionKind, PhaseBoundary,
    PhaseDiagnostic, PlannerDiagnostic, ResearchDiagnostic, ResearchLane, SkillFamilyDiagnostic,
    SkillFamilyTransitionKind, StorageDiagnostic, TerminalCause, TradeCaravanDiagnostic,
    TradePosture, TransitionDiagnostic, UiActionDiagnostic,
};

fn id(value: &str) -> DiagnosticId {
    DiagnosticId::new(value).unwrap()
}

fn text(value: &str) -> DiagnosticText {
    DiagnosticText::new(value).unwrap()
}

fn counts(values: &[(&str, u64)]) -> DiagnosticCounts {
    DiagnosticCounts::try_from_map(
        values
            .iter()
            .map(|(key, value)| (id(key), *value))
            .collect(),
    )
    .unwrap()
}

fn scores(values: &[(&str, i64)]) -> DiagnosticScores {
    DiagnosticScores::try_from_map(
        values
            .iter()
            .map(|(key, value)| (id(key), *value))
            .collect(),
    )
    .unwrap()
}

fn reasons(values: &[(&str, &str)]) -> DiagnosticReasons {
    DiagnosticReasons::try_from_map(
        values
            .iter()
            .map(|(key, value)| (id(key), text(value)))
            .collect(),
    )
    .unwrap()
}

fn transition() -> TransitionDiagnostic {
    TransitionDiagnostic {
        domain: id("task"),
        from: id("reserved"),
        to: id("in_transit"),
        cause: text("carrier assigned"),
    }
}

fn phase_event() -> DiagnosticEvent {
    DiagnosticEvent::Phase(PhaseDiagnostic {
        phase: id("planner"),
        boundary: PhaseBoundary::Exit,
        elapsed_micros: 42,
    })
}

fn all_domain_events() -> Vec<(u64, DiagnosticEvent)> {
    vec![
        (1, phase_event()),
        (
            2,
            DiagnosticEvent::Planner(PlannerDiagnostic {
                candidate_scores: scores(&[("food", 900), ("hole", 300)]),
                omissions: reasons(&[("research", "officer vacancy")]),
            }),
        ),
        (
            3,
            DiagnosticEvent::Matcher(MatcherDiagnostic {
                priorities: scores(&[("food", 900)]),
                matches: counts(&[("worker_1", 1)]),
                rejections: reasons(&[("worker_2", "refused")]),
                task_count: 4,
                blockers: counts(&[("route", 1)]),
                reservation_count: 3,
            }),
        ),
        (
            4,
            DiagnosticEvent::SkillFamily(SkillFamilyDiagnostic {
                kind: SkillFamilyTransitionKind::Teaching,
                subject_id: id("cat_1"),
                related_id: Some(id("cat_2")),
                delta: 25,
                detail: text("third task teaching"),
            }),
        ),
        (
            5,
            DiagnosticEvent::Election(ElectionDiagnostic {
                candidate_scores: scores(&[("cat_1", 75), ("cat_2", 70)]),
                ballot_counts: counts(&[("cat_1", 8), ("cat_2", 7)]),
                selected_candidate: Some(id("cat_1")),
            }),
        ),
        (
            6,
            DiagnosticEvent::Research(ResearchDiagnostic {
                lane: ResearchLane::Leader,
                selected: Some(id("grain_milling")),
                collision: true,
                refunds: counts(&[("notes", 25)]),
            }),
        ),
        (
            7,
            DiagnosticEvent::Construction(ConstructionDiagnostic {
                project_id: id("workshop_1"),
                stage: id("structure"),
                cargo: counts(&[("resource_planks", 8)]),
                blockers: counts(&[("missing_fixture", 1)]),
            }),
        ),
        (
            8,
            DiagnosticEvent::Storage(StorageDiagnostic {
                zone_id: id("workshop_store_1"),
                used_slots: 15,
                capacity_slots: 16,
                pressure: counts(&[("food", 12)]),
            }),
        ),
        (
            9,
            DiagnosticEvent::HoleFeed(HoleFeedDiagnostic {
                operation_id: id("hole_feed_1"),
                stage: id("delivery"),
                cargo: counts(&[("food_apple", 2)]),
                blockers: reasons(&[("route", "bridge unavailable")]),
            }),
        ),
        (
            10,
            DiagnosticEvent::TradeCaravan(TradeCaravanDiagnostic {
                contract_id: id("contract_1"),
                posture: TradePosture::PossibleNow,
                caravan_stage: id("escrow"),
                cargo: counts(&[("resource_logs", 10)]),
                rejection: None,
            }),
        ),
        (
            11,
            DiagnosticEvent::PersistenceAction(PersistenceActionDiagnostic {
                kind: PersistenceActionKind::Action,
                action_id: id("action_1"),
                outcome: ActionOutcome::Rejected,
                counts: counts(&[("expected_version", 8)]),
                rejection: Some(text("stale version")),
            }),
        ),
        (
            12,
            DiagnosticEvent::Divine(DivineDiagnostic {
                kind: DivineDiagnosticKind::Contribution,
                action_id: id("click_batch_1"),
                counts: counts(&[("clicks", 20), ("generated_cargo", 1)]),
                contribution_numerator: Some(1),
                contribution_denominator: Some(4),
                rejection: None,
            }),
        ),
        (
            13,
            DiagnosticEvent::UiAction(UiActionDiagnostic {
                envelope_id: id("envelope_1"),
                action_id: id("research_queue"),
                outcome: ActionOutcome::Rejected,
                rejection: Some(text("version conflict")),
            }),
        ),
        (14, DiagnosticEvent::LastTransition(transition())),
    ]
}

#[test]
fn lai69_disabled_default_is_a_complete_no_op_and_developer_only() {
    let mut trace = LeaderAiDiagnosticTrace::default();
    let before = trace.to_canonical_json();
    assert!(!trace.is_enabled());
    assert_eq!(trace.visibility(), DiagnosticVisibility::DeveloperOnly);
    assert_eq!(trace.record(1, phase_event()).unwrap(), None);
    assert_eq!(
        trace
            .maybe_record_heartbeat(
                HEARTBEAT_TICK_INTERVAL,
                id("planner"),
                1,
                2,
                Some(transition()),
                HeartbeatStatus::Running,
            )
            .unwrap(),
        None
    );
    assert!(trace.records().is_empty());
    assert_eq!(trace.to_canonical_json(), before);

    trace.enable();
    assert!(trace.is_enabled());
    trace.record(1, phase_event()).unwrap();
    trace.disable();
    assert!(!trace.is_enabled());
    assert!(trace.records().is_empty());
    assert_eq!(
        LeaderAiDiagnosticTrace::decode_strict(&trace.to_canonical_json()).unwrap(),
        trace
    );
}

#[test]
fn lai69_every_required_domain_is_typed_and_representable() {
    let mut trace = LeaderAiDiagnosticTrace::enabled(32).unwrap();
    for (tick, event) in all_domain_events() {
        trace.record(tick, event).unwrap();
    }
    trace
        .maybe_record_heartbeat(
            HEARTBEAT_TICK_INTERVAL,
            id("outcomes"),
            4,
            3,
            Some(transition()),
            HeartbeatStatus::Terminal(TerminalCause::Completed),
        )
        .unwrap();
    assert_eq!(
        trace
            .records()
            .iter()
            .map(|record| record.event.domain())
            .collect::<Vec<_>>(),
        vec![
            DiagnosticDomain::Phase,
            DiagnosticDomain::Planner,
            DiagnosticDomain::Matcher,
            DiagnosticDomain::SkillFamily,
            DiagnosticDomain::Election,
            DiagnosticDomain::Research,
            DiagnosticDomain::Construction,
            DiagnosticDomain::Storage,
            DiagnosticDomain::HoleFeed,
            DiagnosticDomain::TradeCaravan,
            DiagnosticDomain::PersistenceAction,
            DiagnosticDomain::Divine,
            DiagnosticDomain::UiAction,
            DiagnosticDomain::LastTransition,
            DiagnosticDomain::Heartbeat,
        ]
    );
    let json = trace.to_canonical_json();
    for required in [
        "elapsedMicros",
        "candidateScores",
        "omissions",
        "priorities",
        "rejections",
        "taskCount",
        "blockers",
        "reservationCount",
        "ballotCounts",
        "refunds",
        "cargo",
        "pressure",
        "contributionNumerator",
        "envelopeId",
        "lastTransition",
    ] {
        assert!(json.contains(required), "missing {required}");
    }
    let enum_shapes = serde_json::to_string(&(
        [PhaseBoundary::Enter, PhaseBoundary::Exit],
        [
            SkillFamilyTransitionKind::Skill,
            SkillFamilyTransitionKind::Teaching,
            SkillFamilyTransitionKind::Family,
        ],
        [ResearchLane::Leader, ResearchLane::God],
        [
            PersistenceActionKind::Load,
            PersistenceActionKind::Save,
            PersistenceActionKind::Reset,
            PersistenceActionKind::Action,
        ],
        [
            DivineDiagnosticKind::ClickBatch,
            DivineDiagnosticKind::Contribution,
            DivineDiagnosticKind::RateRejection,
            DivineDiagnosticKind::Inspiration,
            DivineDiagnosticKind::Boost,
            DivineDiagnosticKind::Miracle,
            DivineDiagnosticKind::Rescue,
        ],
    ))
    .unwrap();
    for required in [
        "enter",
        "exit",
        "skill",
        "teaching",
        "family",
        "leader",
        "god",
        "load",
        "save",
        "reset",
        "action",
        "click_batch",
        "contribution",
        "rate_rejection",
        "inspiration",
        "boost",
        "miracle",
        "rescue",
    ] {
        assert!(enum_shapes.contains(required), "missing {required}");
    }
}

#[test]
fn lai69_bounds_stable_strings_and_oldest_eviction_are_exact() {
    assert_eq!(
        LeaderAiDiagnosticTrace::enabled(0),
        Err(DiagnosticError::InvalidCapacity(0))
    );
    assert_eq!(
        LeaderAiDiagnosticTrace::enabled(MAX_DIAGNOSTIC_CAPACITY + 1),
        Err(DiagnosticError::InvalidCapacity(
            MAX_DIAGNOSTIC_CAPACITY + 1
        ))
    );
    assert_eq!(
        DiagnosticId::new("Bad-ID"),
        Err(DiagnosticError::InvalidStableId)
    );
    assert_eq!(
        DiagnosticId::new(format!("a{}", "x".repeat(64))),
        Err(DiagnosticError::InvalidStableId)
    );
    assert_eq!(
        DiagnosticText::new("x".repeat(MAX_DIAGNOSTIC_TEXT_BYTES + 1)),
        Err(DiagnosticError::InvalidText)
    );

    let too_many = (0..=MAX_DIAGNOSTIC_MAP_ENTRIES)
        .map(|index| (id(&format!("key_{index}")), index as u64))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        DiagnosticCounts::try_from_map(too_many),
        Err(DiagnosticError::TooManyMapEntries)
    );

    let mut trace = LeaderAiDiagnosticTrace::enabled(2).unwrap();
    trace.record(1, phase_event()).unwrap();
    trace.record(2, phase_event()).unwrap();
    trace.record(3, phase_event()).unwrap();
    assert_eq!(trace.records().len(), 2);
    assert_eq!(
        trace
            .records()
            .iter()
            .map(|record| (record.sequence, record.tick))
            .collect::<Vec<_>>(),
        vec![(1, 2), (2, 3)]
    );
}

#[test]
fn lai69_canonical_order_and_same_input_twins_are_identical() {
    let left_counts = counts(&[("zeta", 2), ("alpha", 1)]);
    let right_counts = counts(&[("alpha", 1), ("zeta", 2)]);
    assert_eq!(left_counts, right_counts);
    assert_eq!(
        serde_json::to_string(&left_counts).unwrap(),
        r#"{"alpha":1,"zeta":2}"#
    );

    let mut left = LeaderAiDiagnosticTrace::enabled(8).unwrap();
    let mut right = LeaderAiDiagnosticTrace::enabled(8).unwrap();
    for (tick, event) in all_domain_events().into_iter().take(4) {
        left.record(tick, event.clone()).unwrap();
        right.record(tick, event).unwrap();
    }
    assert_eq!(left, right);
    assert_eq!(left.to_canonical_json(), right.to_canonical_json());
}

#[test]
fn lai69_strict_decode_rejects_unknown_future_and_malformed_state() {
    let mut trace = LeaderAiDiagnosticTrace::enabled(4).unwrap();
    trace.record(1, phase_event()).unwrap();

    let mut unknown: serde_json::Value =
        serde_json::from_str(&trace.to_canonical_json()).unwrap();
    unknown["unknown"] = serde_json::json!(true);
    assert_eq!(
        LeaderAiDiagnosticTrace::decode_strict(&unknown.to_string()),
        Err(DiagnosticError::MalformedState)
    );

    let mut future: serde_json::Value =
        serde_json::from_str(&trace.to_canonical_json()).unwrap();
    future["schemaVersion"] = serde_json::json!(2);
    assert_eq!(
        LeaderAiDiagnosticTrace::decode_strict(&future.to_string()),
        Err(DiagnosticError::InvalidSchemaVersion(2))
    );

    let mut nested_unknown: serde_json::Value =
        serde_json::from_str(&trace.to_canonical_json()).unwrap();
    nested_unknown["records"][0]["event"]["payload"]["unknown"] = serde_json::json!(true);
    assert_eq!(
        LeaderAiDiagnosticTrace::decode_strict(&nested_unknown.to_string()),
        Err(DiagnosticError::MalformedState)
    );

    let mut malformed_sequence: serde_json::Value =
        serde_json::from_str(&trace.to_canonical_json()).unwrap();
    malformed_sequence["records"][0]["sequence"] = serde_json::json!(9);
    assert_eq!(
        LeaderAiDiagnosticTrace::decode_strict(&malformed_sequence.to_string()),
        Err(DiagnosticError::InvalidPersistedState)
    );
    assert_eq!(
        LeaderAiDiagnosticTrace::decode_strict("{"),
        Err(DiagnosticError::MalformedState)
    );
}

#[test]
fn lai69_restart_round_trip_preserves_ring_sequence_and_last_transition() {
    let mut trace = LeaderAiDiagnosticTrace::enabled(4).unwrap();
    for (tick, event) in all_domain_events().into_iter().take(4) {
        trace.record(tick, event).unwrap();
    }
    trace.record(5, DiagnosticEvent::LastTransition(transition())).unwrap();
    let encoded = trace.to_canonical_json();
    let restarted = LeaderAiDiagnosticTrace::decode_strict(&encoded).unwrap();
    assert_eq!(restarted, trace);
    assert_eq!(restarted.to_canonical_json(), encoded);
    assert_eq!(
        restarted
            .records()
            .iter()
            .map(|record| record.sequence)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
}

fn heartbeat_trace(partitioned: bool) -> LeaderAiDiagnosticTrace {
    let mut trace = LeaderAiDiagnosticTrace::enabled(8).unwrap();
    let ranges = if partitioned {
        vec![(1, 119), (120, 239), (240, 240)]
    } else {
        vec![(1, 240)]
    };
    for (start, end) in ranges {
        for tick in start..=end {
            trace
                .maybe_record_heartbeat(
                    tick,
                    id("world_tick"),
                    7,
                    5,
                    Some(transition()),
                    if tick == 240 {
                        HeartbeatStatus::Terminal(TerminalCause::Completed)
                    } else {
                        HeartbeatStatus::Running
                    },
                )
                .unwrap();
        }
    }
    trace
}

#[test]
fn lai69_heartbeat_cadence_is_exact_across_partition_boundaries() {
    let one_range = heartbeat_trace(false);
    let partitioned = heartbeat_trace(true);
    assert_eq!(one_range, partitioned);
    assert_eq!(
        one_range
            .records()
            .iter()
            .map(|record| record.tick)
            .collect::<Vec<_>>(),
        vec![120, 240]
    );
    let heartbeat = match &one_range.records()[1].event {
        DiagnosticEvent::Heartbeat(heartbeat) => heartbeat,
        other => panic!("expected heartbeat, got {other:?}"),
    };
    assert_eq!(heartbeat.current_phase, id("world_tick"));
    assert_eq!(heartbeat.task_count, 7);
    assert_eq!(heartbeat.reservation_count, 5);
    assert_eq!(heartbeat.last_transition, Some(transition()));
    assert_eq!(
        heartbeat.status,
        HeartbeatStatus::Terminal(TerminalCause::Completed)
    );

    let mut duplicate = one_range;
    assert_eq!(
        duplicate
            .maybe_record_heartbeat(
                240,
                id("world_tick"),
                7,
                5,
                Some(transition()),
                HeartbeatStatus::Running,
            )
            .unwrap(),
        None
    );
    let before = duplicate.clone();
    assert_eq!(
        duplicate.record(
            241,
            DiagnosticEvent::Heartbeat(leader_ai_diagnostics::HeartbeatDiagnostic {
                current_phase: id("world_tick"),
                task_count: 0,
                reservation_count: 0,
                last_transition: None,
                status: HeartbeatStatus::Running,
            }),
        ),
        Err(DiagnosticError::InvalidHeartbeatTick(241))
    );
    assert_eq!(duplicate, before);
}

#[test]
fn lai69_count_overflow_and_invalid_events_reject_without_mutation() {
    let key = id("tasks");
    let mut values = BTreeMap::new();
    values.insert(key.clone(), u64::MAX);
    let mut counts = DiagnosticCounts::try_from_map(values).unwrap();
    let before_counts = counts.clone();
    assert_eq!(
        counts.checked_increment(key, 1),
        Err(DiagnosticError::CountOverflow)
    );
    assert_eq!(counts, before_counts);

    let mut trace = LeaderAiDiagnosticTrace::enabled(4).unwrap();
    let before_trace = trace.clone();
    assert_eq!(
        trace.record(
            1,
            DiagnosticEvent::UiAction(UiActionDiagnostic {
                envelope_id: id("envelope_1"),
                action_id: id("action_1"),
                outcome: ActionOutcome::Accepted,
                rejection: Some(text("should not exist")),
            }),
        ),
        Err(DiagnosticError::InvalidEvent)
    );
    assert_eq!(trace, before_trace);
    assert_eq!(
        trace.record(
            1,
            DiagnosticEvent::Divine(DivineDiagnostic {
                kind: DivineDiagnosticKind::RateRejection,
                action_id: id("click_1"),
                counts: DiagnosticCounts::default(),
                contribution_numerator: Some(1),
                contribution_denominator: Some(0),
                rejection: None,
            }),
        ),
        Err(DiagnosticError::InvalidEvent)
    );
    assert_eq!(trace, before_trace);
    trace.record(2, phase_event()).unwrap();
    let after_tick_two = trace.clone();
    assert_eq!(
        trace.record(1, phase_event()),
        Err(DiagnosticError::NonMonotonicTick {
            previous: 2,
            next: 1,
        })
    );
    assert_eq!(trace, after_tick_two);

    let mut near_overflow_json: serde_json::Value =
        serde_json::from_str(&after_tick_two.to_canonical_json()).unwrap();
    near_overflow_json["nextSequence"] = serde_json::json!(u64::MAX);
    near_overflow_json["records"][0]["sequence"] = serde_json::json!(u64::MAX - 1);
    let mut near_overflow =
        LeaderAiDiagnosticTrace::decode_strict(&near_overflow_json.to_string()).unwrap();
    let before_overflow = near_overflow.clone();
    assert_eq!(
        near_overflow.record(3, phase_event()),
        Err(DiagnosticError::SequenceOverflow)
    );
    assert_eq!(near_overflow, before_overflow);
}

#[test]
fn lai69_only_completed_is_a_pass_and_silence_has_no_success_state() {
    assert!(TerminalCause::Completed.is_pass());
    for failure in [
        TerminalCause::Timeout,
        TerminalCause::Stalled,
        TerminalCause::SimulationFailure,
        TerminalCause::Panic,
    ] {
        assert!(!failure.is_pass());
        assert!(!HeartbeatStatus::Terminal(failure).is_pass());
    }
    assert!(!HeartbeatStatus::Running.is_pass());
    assert!(HeartbeatStatus::Terminal(TerminalCause::Completed).is_pass());
}

#[test]
fn lai69_has_no_player_log_protocol_or_automatic_output_surface() {
    let source = include_str!("../src/leader_ai_diagnostics.rs");
    for forbidden in [
        "cat_protocol",
        "PlayerLog",
        "player_log",
        "println!",
        "eprintln!",
        "stdout",
        "stderr",
        "std::fs",
        "std::time",
        "std::thread",
        "TcpStream",
        "UdpSocket",
    ] {
        assert!(!source.contains(forbidden), "found forbidden surface {forbidden}");
    }
    assert!(source.contains("DeveloperOnly"));
    assert!(!source.contains("impl From<LeaderAiDiagnosticTrace"));
    assert!(!source.contains("impl Into<"));
}
