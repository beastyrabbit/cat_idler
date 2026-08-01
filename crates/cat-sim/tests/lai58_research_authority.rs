use cat_sim::{
    planner_core::PlannerId,
    progression_research::{
        ColonyPartitionKey, ProgressionCatalog, ResearchNotes, StudyCurrency, StudyId, StudyKind,
        VoidInsight,
    },
    research_authority::{
        CriticalVillageAuthorization, DuplicateAuthorization, LeaderCandidate,
        LeaderDuplicatePermit, LeaderExclusionReason, MAX_GOD_QUEUE_ENTRIES,
        ROLLING_SEVEN_GAME_DAYS_MINUTES, ResearchAuthority, ResearchAuthorityError,
        ResearchCommand, ResearchCommandId, ResearchCommandKind, ResearchCommandOutcome,
    },
};

fn colony() -> PlannerId {
    PlannerId::derive("lai58-authority-test", ["colony"])
}

fn catalog() -> ProgressionCatalog {
    ProgressionCatalog::from_embedded().expect("embedded canonical progression catalog")
}

fn authority() -> ResearchAuthority {
    ResearchAuthority::new(
        colony(),
        ResearchNotes::from_micro(u64::MAX / 4),
        VoidInsight::from_micro(u64::MAX / 4),
    )
}

fn ordinary_root(catalog: &ProgressionCatalog) -> StudyId {
    catalog
        .studies()
        .values()
        .find(|study| study.currency() == StudyCurrency::Notes && study.prerequisites.is_empty())
        .expect("an ordinary root")
        .id
        .clone()
}

fn command(state: &ResearchAuthority, action: &str, kind: ResearchCommandKind) -> ResearchCommand {
    ResearchCommand {
        id: ResearchCommandId::derive(&colony(), action),
        expected_version: state.version,
        kind,
    }
}

#[test]
fn lai58_authority_leader_and_god_lanes_are_independent() {
    let catalog = catalog();
    let study_id = ordinary_root(&catalog);
    let mut state = authority();
    let before_notes = state.notes.balance;
    let before_void = state.void.balance;
    state
        .apply(
            &catalog,
            command(
                &state,
                "leader-free",
                ResearchCommandKind::CompleteLeader {
                    study_id: study_id.clone(),
                    effective_loremaster_level: 0,
                    now_tick: 0,
                    duplicate_permit: LeaderDuplicatePermit::None,
                },
            ),
        )
        .expect("Leader unlock is free and instant");
    assert_eq!(state.notes.balance, before_notes);
    assert_eq!(state.void.balance, before_void);
    assert!(state.god_queue.is_empty());
    assert!(state.owned_finite.contains(&study_id));
}

#[test]
fn lai58_authority_has_exact_rolling_leader_cadence() {
    assert_eq!(
        (0..=5)
            .map(ResearchAuthority::leader_cadence_limit)
            .collect::<Vec<_>>(),
        vec![1, 1, 2, 2, 3, 4]
    );
    let catalog = catalog();
    let mut state = authority();
    let root = ordinary_root(&catalog);
    state
        .apply(
            &catalog,
            command(
                &state,
                "first",
                ResearchCommandKind::CompleteLeader {
                    study_id: root.clone(),
                    effective_loremaster_level: 0,
                    now_tick: 0,
                    duplicate_permit: LeaderDuplicatePermit::None,
                },
            ),
        )
        .expect("first weekly free study");
    let another = catalog
        .studies()
        .values()
        .find(|study| study.id != root && study.prerequisites.is_empty())
        .expect("another root")
        .id
        .clone();
    assert_eq!(
        state
            .apply(
                &catalog,
                command(
                    &state,
                    "too-soon",
                    ResearchCommandKind::CompleteLeader {
                        study_id: another.clone(),
                        effective_loremaster_level: 0,
                        now_tick: ROLLING_SEVEN_GAME_DAYS_MINUTES - 1,
                        duplicate_permit: LeaderDuplicatePermit::None,
                    },
                ),
            )
            .expect_err("same low-Loremaster window is exhausted"),
        ResearchAuthorityError::LeaderCadenceExhausted
    );
    state
        .apply(
            &catalog,
            command(
                &state,
                "next-window",
                ResearchCommandKind::CompleteLeader {
                    study_id: another,
                    effective_loremaster_level: 0,
                    now_tick: ROLLING_SEVEN_GAME_DAYS_MINUTES,
                    duplicate_permit: LeaderDuplicatePermit::None,
                },
            ),
        )
        .expect("exact seven-day boundary releases capacity");
}

#[test]
fn lai58_authority_retires_expired_cadence_history_for_unbounded_late_progression() {
    let catalog = catalog();
    let repeatable = catalog
        .studies()
        .values()
        .find(|study| {
            matches!(
                study.kind,
                StudyKind::BoostDuration { stage: 11 } | StudyKind::BoostEconomy { stage: 11 }
            )
        })
        .expect("canonical catalog has a repeatable terminal")
        .id
        .clone();
    let mut state = authority();
    for study_id in catalog.studies().keys() {
        if study_id != &repeatable
            && !matches!(
                catalog.study(study_id).map(|study| &study.kind),
                Some(
                    StudyKind::BoostDuration { stage: 11 } | StudyKind::BoostEconomy { stage: 11 }
                )
            )
        {
            state.owned_finite.insert(study_id.clone());
        }
    }

    for cycle in 0_u64..40 {
        let now_tick = cycle * ROLLING_SEVEN_GAME_DAYS_MINUTES;
        state
            .apply(
                &catalog,
                command(
                    &state,
                    &format!("late-cycle-{cycle}"),
                    ResearchCommandKind::CompleteLeader {
                        study_id: repeatable.clone(),
                        now_tick,
                        effective_loremaster_level: 5,
                        duplicate_permit: LeaderDuplicatePermit::None,
                    },
                ),
            )
            .expect("expired cadence history must not stop repeatable progression");
        assert!(state.leader_commits.len() <= 1);
    }
    assert_eq!(state.repeatable_completions[&repeatable], 40);
}

#[test]
fn lai58_authority_finite_first_excludes_repeatable_terminals() {
    let catalog = catalog();
    let state = authority();
    let terminal = catalog
        .studies()
        .keys()
        .find(|id| {
            matches!(
                catalog.study(id).map(|study| &study.kind),
                Some(
                    StudyKind::BoostDuration { stage: 11 } | StudyKind::BoostEconomy { stage: 11 }
                )
            )
        })
        .expect("a canonical infinite terminal")
        .clone();
    let selected = state.select_leader_target(
        &catalog,
        &[LeaderCandidate {
            study_id: terminal.clone(),
            report_score: 9,
            need_score: 9,
            intelligence_score: 9,
            personality_score: 9,
            research_skill_score: 9,
        }],
        5,
        0,
        &LeaderDuplicatePermit::None,
    );
    assert_eq!(selected.selected, None);
    assert_eq!(
        selected.excluded[&terminal],
        LeaderExclusionReason::RepeatableWhileFiniteRemains
    );
}

#[test]
fn lai58_authority_excludes_queued_studies_except_typed_critical_or_oopsie() {
    let catalog = catalog();
    let target = ordinary_root(&catalog);
    let mut state = authority();
    state
        .apply(
            &catalog,
            command(
                &state,
                "queue",
                ResearchCommandKind::QueueGodPath {
                    target: target.clone(),
                },
            ),
        )
        .expect("queue target");
    let candidate = LeaderCandidate {
        study_id: target.clone(),
        report_score: 1,
        need_score: 2,
        intelligence_score: 3,
        personality_score: 4,
        research_skill_score: 5,
    };
    let normal = state.select_leader_target(
        &catalog,
        &[candidate.clone()],
        5,
        0,
        &LeaderDuplicatePermit::None,
    );
    assert_eq!(normal.selected, None);
    assert_eq!(
        normal.excluded[&target],
        LeaderExclusionReason::GodQueueDuplicateForbidden
    );
    let critical = state.select_leader_target(
        &catalog,
        &[candidate],
        5,
        0,
        &LeaderDuplicatePermit::CriticalVillage(CriticalVillageAuthorization {
            report_marks_critical: true,
            needed_before_tick: 4,
            estimated_god_completion_tick: 5,
        }),
    );
    assert_eq!(critical.selected, Some(target));
    assert_eq!(
        critical.selected_duplicate_authorization,
        DuplicateAuthorization::CriticalVillage
    );
    assert_eq!(
        (0..=4)
            .map(LeaderDuplicatePermit::oopsie_percent)
            .collect::<Vec<_>>(),
        vec![25, 12, 5, 1, 0]
    );
    let oopsie = state.select_leader_target(
        &catalog,
        &[LeaderCandidate {
            study_id: ordinary_root(&catalog),
            report_score: 1,
            need_score: 0,
            intelligence_score: 0,
            personality_score: 0,
            research_skill_score: 0,
        }],
        5,
        0,
        &LeaderDuplicatePermit::KeyedOopsie(
            cat_sim::research_authority::KeyedOopsieAuthorization {
                effective_level: 1,
                keyed_roll_percent: 11,
            },
        ),
    );
    assert_eq!(
        oopsie.selected_duplicate_authorization,
        DuplicateAuthorization::KeyedOopsie
    );
}

#[test]
fn lai58_authority_queue_is_bounded_and_front_terms_are_frozen() {
    let catalog = catalog();
    let target = ordinary_root(&catalog);
    let mut state = authority();
    state
        .apply(
            &catalog,
            command(
                &state,
                "queue",
                ResearchCommandKind::QueueGodPath { target },
            ),
        )
        .expect("queue root");
    let funded = state
        .apply(
            &catalog,
            command(
                &state,
                "fund",
                ResearchCommandKind::FundGodFront {
                    consume_preparation: false,
                },
            ),
        )
        .expect("fund only front");
    let first_payable = match funded.outcome {
        ResearchCommandOutcome::Funded { payable_cost_micro } => payable_cost_micro,
        _ => panic!("fund receipt"),
    };
    assert!(state.god_front().expect("front").frozen.is_some());
    assert_eq!(state.notes.partition.colony_id, colony());
    assert_eq!(
        state.notes.spends.len(),
        1,
        "funding is a canonical ledger debit"
    );
    assert!(state.notes.version > 0);
    let refund = state
        .apply(
            &catalog,
            command(
                &state,
                "fund-again",
                ResearchCommandKind::FundGodFront {
                    consume_preparation: false,
                },
            ),
        )
        .expect("funding is idempotent within a new action");
    assert_eq!(
        refund.outcome,
        ResearchCommandOutcome::Funded {
            payable_cost_micro: first_payable
        }
    );
}

#[test]
fn lai58_authority_rejects_the_sixty_fifth_topological_queue_entry() {
    let catalog = catalog();
    let mut state = authority();
    let targets = catalog.studies().keys().cloned().collect::<Vec<_>>();
    let mut rejected = false;
    for (index, target) in targets.into_iter().enumerate() {
        let request = command(
            &state,
            &format!("queue-cap-{index}"),
            ResearchCommandKind::QueueGodPath { target },
        );
        match state.apply(&catalog, request) {
            Ok(_) => {}
            Err(ResearchAuthorityError::QueueFull) => {
                rejected = true;
                break;
            }
            Err(error) => panic!("queue path must either fit or hit exact cap: {error}"),
        }
    }
    assert!(rejected, "canonical graph exceeds one God queue page");
    assert!(state.god_queue.len() <= MAX_GOD_QUEUE_ENTRIES);
}

#[test]
fn lai58_authority_requires_physical_station_and_staff_for_both_labor_kinds() {
    let catalog = catalog();
    let target = ordinary_root(&catalog);
    let mut state = authority();
    state
        .apply(
            &catalog,
            command(
                &state,
                "queue",
                ResearchCommandKind::QueueGodPath {
                    target: target.clone(),
                },
            ),
        )
        .expect("queue");
    state
        .apply(
            &catalog,
            command(
                &state,
                "request-preparation",
                ResearchCommandKind::RequestPreparation {
                    study_id: target.clone(),
                },
            ),
        )
        .expect("request zero-progress preparation");
    assert_eq!(
        state
            .apply(
                &catalog,
                command(
                    &state,
                    "no-station-prep",
                    ResearchCommandKind::PerformPreparation {
                        study_id: target.clone(),
                        staffed_research_station: false,
                        scholar_alive: true,
                        labor_minutes: 1
                    }
                )
            )
            .expect_err("preparation needs physical station"),
        ResearchAuthorityError::ResearchInfrastructureUnavailable
    );
    state
        .apply(
            &catalog,
            command(
                &state,
                "fund",
                ResearchCommandKind::FundGodFront {
                    consume_preparation: false,
                },
            ),
        )
        .expect("fund");
    assert_eq!(
        state
            .apply(
                &catalog,
                command(
                    &state,
                    "no-scholar",
                    ResearchCommandKind::PerformGodLabor {
                        staffed_research_station: true,
                        scholar_alive: false,
                        labor_minutes: 1
                    }
                )
            )
            .expect_err("research needs live staffed scholar"),
        ResearchAuthorityError::ResearchInfrastructureUnavailable
    );
}

#[test]
fn lai58_authority_preparation_is_one_use_floor_discount() {
    let catalog = catalog();
    let target = ordinary_root(&catalog);
    let definition = catalog.study(&target).expect("target definition");
    let mut state = authority();
    state
        .apply(
            &catalog,
            command(
                &state,
                "queue",
                ResearchCommandKind::QueueGodPath {
                    target: target.clone(),
                },
            ),
        )
        .expect("queue");
    let required = definition.required_work_minutes.div_ceil(4);
    state
        .apply(
            &catalog,
            command(
                &state,
                "request-preparation",
                ResearchCommandKind::RequestPreparation {
                    study_id: target.clone(),
                },
            ),
        )
        .expect("request zero-progress preparation");
    state
        .apply(
            &catalog,
            command(
                &state,
                "prepare",
                ResearchCommandKind::PerformPreparation {
                    study_id: target.clone(),
                    staffed_research_station: true,
                    scholar_alive: true,
                    labor_minutes: required,
                },
            ),
        )
        .expect("complete prep once");
    let receipt = state
        .apply(
            &catalog,
            command(
                &state,
                "fund-with-prep",
                ResearchCommandKind::FundGodFront {
                    consume_preparation: true,
                },
            ),
        )
        .expect("consume prep only on player God front");
    assert_eq!(
        receipt.outcome,
        ResearchCommandOutcome::Funded {
            payable_cost_micro: definition.cost_micro - definition.cost_micro / 4
        }
    );
    assert!(!state.preparations.contains_key(&target));
}

#[test]
fn lai58_authority_cancellation_refunds_frozen_currency_and_loses_labor() {
    let catalog = catalog();
    let target = ordinary_root(&catalog);
    let mut state = authority();
    let initial = state.notes.balance;
    state
        .apply(
            &catalog,
            command(
                &state,
                "queue",
                ResearchCommandKind::QueueGodPath {
                    target: target.clone(),
                },
            ),
        )
        .expect("queue");
    state
        .apply(
            &catalog,
            command(
                &state,
                "fund",
                ResearchCommandKind::FundGodFront {
                    consume_preparation: false,
                },
            ),
        )
        .expect("fund");
    state
        .apply(
            &catalog,
            command(
                &state,
                "labor",
                ResearchCommandKind::PerformGodLabor {
                    staffed_research_station: true,
                    scholar_alive: true,
                    labor_minutes: 1,
                },
            ),
        )
        .expect("work");
    let removal = state
        .apply(
            &catalog,
            command(
                &state,
                "cancel",
                ResearchCommandKind::RemoveGodTarget { study_id: target },
            ),
        )
        .expect("cancel");
    assert_eq!(state.notes.balance, initial);
    assert!(matches!(
        removal.outcome,
        ResearchCommandOutcome::GodTargetRemoved {
            lost_labor_minutes: 1,
            ..
        }
    ));
}

#[test]
fn lai58_authority_leader_overtake_refunds_and_reports_why() {
    let catalog = catalog();
    let target = ordinary_root(&catalog);
    let mut state = authority();
    let initial = state.notes.balance;
    state
        .apply(
            &catalog,
            command(
                &state,
                "queue",
                ResearchCommandKind::QueueGodPath {
                    target: target.clone(),
                },
            ),
        )
        .expect("queue");
    state
        .apply(
            &catalog,
            command(
                &state,
                "fund",
                ResearchCommandKind::FundGodFront {
                    consume_preparation: false,
                },
            ),
        )
        .expect("fund");
    let receipt = state
        .apply(
            &catalog,
            command(
                &state,
                "critical",
                ResearchCommandKind::CompleteLeader {
                    study_id: target,
                    effective_loremaster_level: 5,
                    now_tick: 0,
                    duplicate_permit: LeaderDuplicatePermit::CriticalVillage(
                        CriticalVillageAuthorization {
                            report_marks_critical: true,
                            needed_before_tick: 1,
                            estimated_god_completion_tick: 2,
                        },
                    ),
                },
            ),
        )
        .expect("critical report may overtake God");
    assert_eq!(state.notes.balance, initial);
    assert!(matches!(
        receipt.outcome,
        ResearchCommandOutcome::LeaderCompleted {
            duplicate_authorization: DuplicateAuthorization::CriticalVillage,
            ..
        }
    ));
}

#[test]
fn lai58_authority_replays_exact_receipt_conflicts_and_strictly_decodes() {
    let catalog = catalog();
    let target = ordinary_root(&catalog);
    let mut state = authority();
    let first = command(
        &state,
        "queue",
        ResearchCommandKind::QueueGodPath {
            target: target.clone(),
        },
    );
    let receipt = state.apply(&catalog, first.clone()).expect("commit queue");
    assert_eq!(
        state.apply(&catalog, first).expect("same action replay"),
        receipt
    );
    let conflict = ResearchCommand {
        id: ResearchCommandId::derive(&colony(), "queue"),
        expected_version: state.version,
        kind: ResearchCommandKind::QueueGodPath { target },
    };
    assert_eq!(
        state
            .apply(&catalog, conflict)
            .expect_err("same id different payload is conflict"),
        ResearchAuthorityError::IdempotencyConflict
    );
    state
        .apply(
            &catalog,
            command(
                &state,
                "fund-ledger-history",
                ResearchCommandKind::FundGodFront {
                    consume_preparation: false,
                },
            ),
        )
        .expect("canonical ledger debit before restart");
    assert_eq!(state.notes.spends.len(), 1);
    let serialized = serde_json::to_value(&state).expect("serialize authority");
    let restored: ResearchAuthority =
        serde_json::from_value(serialized).expect("strict restart round-trip");
    assert_eq!(restored, state);
    assert!(
        serde_json::from_str::<ResearchAuthority>(r#"{"schemaVersion":1,"unexpected":true}"#)
            .is_err()
    );
}

#[test]
fn lai58_authority_rejects_a_noncanonical_ledger_partition() {
    let catalog = catalog();
    let mut state = authority();
    state.notes.partition = ColonyPartitionKey {
        colony_id: PlannerId::derive("foreign", ["colony"]),
    };
    let target = ordinary_root(&catalog);
    let request = command(
        &state,
        "partition-rejected",
        ResearchCommandKind::QueueGodPath { target },
    );
    assert_eq!(
        state
            .apply(&catalog, request)
            .expect_err("the aggregate cannot attach a foreign notes ledger"),
        ResearchAuthorityError::MalformedPersistence
    );
}
