use cat_sim::leader_ai_diagnostics::{
    LAI69_HEARTBEAT_TICKS, Lai69BlockerReason, Lai69Counts, Lai69DiagnosticDomain,
    Lai69DiagnosticOutcome, Lai69DiagnosticRequest, Lai69DiagnosticsConfig,
    Lai69LeaderAiDiagnostics, Lai69Phase, Lai69TraceInput, Lai69TraceKind,
};

fn diagnostics(max_records: usize) -> Lai69LeaderAiDiagnostics {
    Lai69LeaderAiDiagnostics::new(
        "colony-1".try_into().unwrap_or_else(|_| unreachable!()),
        true,
        Lai69DiagnosticsConfig {
            max_records,
            max_receipts: 4,
            max_top_blockers: 2,
        },
    )
    .unwrap_or_else(|_| unreachable!())
}

fn progress(tick: u64) -> Lai69TraceInput {
    Lai69TraceInput {
        tick,
        phase: Lai69Phase::Execution,
        domain: Lai69DiagnosticDomain::Planner,
        kind: Lai69TraceKind::Progress {
            label: "advanced".to_owned(),
        },
    }
}

#[test]
fn retains_only_the_configured_tail() {
    let mut trace = diagnostics(3);
    for tick in 1..=5 {
        trace
            .record(progress(tick))
            .unwrap_or_else(|_| unreachable!());
    }
    assert_eq!(trace.records.len(), 3);
    assert_eq!(trace.records.front().map(|record| record.event_id), Some(3));
    assert_eq!(trace.records.back().map(|record| record.event_id), Some(5));
}

#[test]
fn heartbeat_is_exactly_every_120_positive_ticks_and_never_repeats() {
    let mut trace = diagnostics(8);
    let counts = Lai69Counts {
        live_cats: 2,
        intents: 3,
        tasks: 4,
        reservations: 5,
        cargo: 6,
        blocked: 7,
    };
    assert!(
        trace
            .heartbeat(0, counts.clone())
            .unwrap_or_else(|_| unreachable!())
            .is_none()
    );
    assert!(
        trace
            .heartbeat(119, counts.clone())
            .unwrap_or_else(|_| unreachable!())
            .is_none()
    );
    let heartbeat = trace
        .heartbeat(LAI69_HEARTBEAT_TICKS, counts.clone())
        .unwrap_or_else(|_| unreachable!())
        .unwrap_or_else(|| unreachable!());
    assert_eq!(heartbeat.tick, 120);
    assert_eq!(heartbeat.counts, counts);
    assert!(
        trace
            .heartbeat(
                120,
                Lai69Counts {
                    live_cats: 0,
                    intents: 0,
                    tasks: 0,
                    reservations: 0,
                    cargo: 0,
                    blocked: 0
                }
            )
            .unwrap_or_else(|_| unreachable!())
            .is_none()
    );
    assert!(
        trace
            .heartbeat(
                240,
                Lai69Counts {
                    live_cats: 0,
                    intents: 0,
                    tasks: 0,
                    reservations: 0,
                    cargo: 0,
                    blocked: 0
                }
            )
            .unwrap_or_else(|_| unreachable!())
            .is_some()
    );
}

#[test]
fn heartbeat_localizes_stall_with_ordered_blockers_and_progress() {
    let mut trace = diagnostics(16);
    for _ in 0..3 {
        trace
            .record(Lai69TraceInput {
                tick: 7,
                phase: Lai69Phase::Matching,
                domain: Lai69DiagnosticDomain::Matcher,
                kind: Lai69TraceKind::Block {
                    reason: Lai69BlockerReason::NoEligibleWorker,
                    label: "eligible worker absent".to_owned(),
                },
            })
            .unwrap_or_else(|_| unreachable!());
    }
    trace
        .record(Lai69TraceInput {
            tick: 8,
            phase: Lai69Phase::Execution,
            domain: Lai69DiagnosticDomain::Hole,
            kind: Lai69TraceKind::Block {
                reason: Lai69BlockerReason::HoleCooldown,
                label: "cooldown".to_owned(),
            },
        })
        .unwrap_or_else(|_| unreachable!());
    trace.record(progress(9)).unwrap_or_else(|_| unreachable!());
    let heartbeat = trace
        .heartbeat(
            120,
            Lai69Counts {
                live_cats: 4,
                intents: 2,
                tasks: 2,
                reservations: 1,
                cargo: 1,
                blocked: 4,
            },
        )
        .unwrap_or_else(|_| unreachable!())
        .unwrap_or_else(|| unreachable!());
    assert_eq!(heartbeat.last_phase, Some(Lai69Phase::Execution));
    assert_eq!(heartbeat.last_domain, Some(Lai69DiagnosticDomain::Planner));
    assert_eq!(
        heartbeat.last_progress_ticks[&Lai69DiagnosticDomain::Planner],
        9
    );
    assert_eq!(
        heartbeat.top_blocker_reasons[0].reason,
        Lai69BlockerReason::NoEligibleWorker
    );
    assert_eq!(heartbeat.top_blocker_reasons[0].count, 3);
}

#[test]
fn request_replay_restart_and_conflict_are_deterministic() {
    let mut trace = diagnostics(16);
    let request = Lai69DiagnosticRequest {
        request_id: "request-1".to_owned(),
        expected_version: 0,
        event: progress(1),
    };
    let first = trace.apply(request.clone());
    assert_eq!(trace.apply(request.clone()), first);
    assert_eq!(trace.records.len(), 1);
    let json = trace.to_json().unwrap_or_else(|_| unreachable!());
    let mut restarted =
        Lai69LeaderAiDiagnostics::from_json(&json).unwrap_or_else(|_| unreachable!());
    assert_eq!(restarted.apply(request), first);
    let conflict = restarted.apply(Lai69DiagnosticRequest {
        request_id: "request-1".to_owned(),
        expected_version: 1,
        event: progress(2),
    });
    assert!(matches!(conflict, Lai69DiagnosticOutcome::Rejected { .. }));
    let stale = restarted.apply(Lai69DiagnosticRequest {
        request_id: "request-2".to_owned(),
        expected_version: 0,
        event: progress(2),
    });
    assert!(matches!(stale, Lai69DiagnosticOutcome::Rejected { .. }));
}

#[test]
fn malformed_and_unknown_persisted_data_fail_closed() {
    assert!(Lai69LeaderAiDiagnostics::from_json("{\"schema_version\":99}").is_err());
    let trace = diagnostics(4);
    let mut value: serde_json::Value =
        serde_json::from_str(&trace.to_json().unwrap_or_else(|_| unreachable!()))
            .unwrap_or_else(|_| unreachable!());
    value
        .as_object_mut()
        .unwrap_or_else(|| unreachable!())
        .insert("unexpected".to_owned(), serde_json::Value::Bool(true));
    assert!(Lai69LeaderAiDiagnostics::from_json(&value.to_string()).is_err());
}

#[test]
fn public_summary_redacts_executor_details_and_records_cannot_explode() {
    let mut trace = diagnostics(2);
    trace
        .record(Lai69TraceInput {
            tick: 1,
            phase: Lai69Phase::Planning,
            domain: Lai69DiagnosticDomain::Planner,
            kind: Lai69TraceKind::Block {
                reason: Lai69BlockerReason::MissingCargo,
                label: "secret exact cargo identity".to_owned(),
            },
        })
        .unwrap_or_else(|_| unreachable!());
    let public = serde_json::to_string(&trace.public_summary()).unwrap_or_else(|_| unreachable!());
    assert!(!public.contains("secret"));
    assert!(!public.contains("missing_cargo"));
    for tick in 2..=500 {
        trace
            .record(progress(tick))
            .unwrap_or_else(|_| unreachable!());
    }
    assert_eq!(trace.records.len(), 2);
    assert!(trace.validate().is_ok());
}
