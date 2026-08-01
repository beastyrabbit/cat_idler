use cat_sim::{
    divine_boosts::{
        DIVINE_BOOST_BASE_DURATION_GAME_HOURS, DIVINE_BOOST_DURATION_HOURS, DivineBoostActor,
        DivineBoostAuthorization, DivineBoostError, DivineBoostOutcome, DivineBoostPurchaseId,
        DivineBoostPurchaseRequest, DivineBoostResearchStages, DivineBoostState, DivineBoostType,
        UnlockedBoostDurations, active_effect_factor, boost_cost,
    },
    planner_core::PlannerId,
    progression_research::{
        ColonyPartitionKey, HoleVoidCreditPayload, PlayerPartitionKey, ProgressionAuthority,
        StudyId, VoidInsight, VoidInsightLedger,
    },
    research_manifest::{ManifestEffect, ManifestTrack, research_manifest},
};

fn colony() -> PlannerId {
    PlannerId::derive("lai44_boost_colony", ["one"])
}

fn player() -> PlannerId {
    PlannerId::derive("lai44_boost_player", ["owner"])
}

fn automated() -> PlannerId {
    PlannerId::derive("lai44_boost_actor", ["automated"])
}

fn funded_ledger(whole: u64) -> VoidInsightLedger {
    let mut ledger = VoidInsightLedger::new(colony());
    ledger
        .credit_hole_feed(HoleVoidCreditPayload {
            partition: ColonyPartitionKey {
                colony_id: colony(),
            },
            feed_sequence: 1,
            amount: VoidInsight::from_whole(whole).unwrap(),
        })
        .unwrap();
    ledger
}

fn progression(
    duration_stage: u8,
    economy_stage: u8,
    unlocked: &[DivineBoostType],
) -> ProgressionAuthority {
    let mut state = ProgressionAuthority::new(colony()).unwrap();
    for boost in unlocked {
        let suffix = match boost {
            DivineBoostType::BountifulLabor => "bountiful_labor",
            DivineBoostType::FleetPaws => "fleet_paws",
            DivineBoostType::InspiredWork => "inspired_work",
            DivineBoostType::RestorativeGrace => "restorative_grace",
        };
        state
            .owned_studies
            .insert(StudyId::new(format!("divine_boost_{suffix}")).unwrap());
    }
    for stage in 1..=duration_stage {
        state
            .owned_studies
            .insert(StudyId::new(format!("divine_duration_{stage:02}")).unwrap());
    }
    for stage in 1..=economy_stage {
        state
            .owned_studies
            .insert(StudyId::new(format!("divine_economy_{stage:02}")).unwrap());
    }
    state
}

fn request(
    state: &DivineBoostState,
    ledger: &VoidInsightLedger,
    player_sequence: u64,
    boost_type: DivineBoostType,
    duration_hours: u32,
    activated_tick: u64,
) -> DivineBoostPurchaseRequest {
    DivineBoostPurchaseRequest {
        id: DivineBoostPurchaseId::derive(&colony(), &player(), player_sequence),
        partition: PlayerPartitionKey {
            colony_id: colony(),
            player_id: player(),
        },
        player_sequence,
        authorization: DivineBoostAuthorization {
            actor: DivineBoostActor::Player {
                player_id: player(),
            },
            authenticated_player_id: Some(player()),
            owns_colony: true,
        },
        boost_type,
        duration_hours,
        expected_boost_version: state.version,
        expected_void_version: ledger.version,
        activated_tick,
        ticks_per_game_hour: 60,
    }
}

fn manifest_boost_stages(duration_stage: u8, economy_stage: u8) -> DivineBoostResearchStages {
    let manifest = research_manifest();
    let duration_studies = manifest.track_studies(ManifestTrack::DivineDuration);
    let economy_studies = manifest.track_studies(ManifestTrack::DivineEconomy);
    let mut effects = Vec::new();
    if duration_stage > 0 {
        effects.extend(
            duration_studies[usize::from(duration_stage - 1)]
                .effects
                .iter(),
        );
    }
    if economy_stage > 0 {
        effects.extend(
            economy_studies[usize::from(economy_stage - 1)]
                .effects
                .iter(),
        );
    }
    DivineBoostResearchStages::from_manifest_effects(effects).unwrap()
}

#[test]
fn exactly_four_specialized_definitions_have_one_hour_base_and_preserved_domains() {
    assert_eq!(DivineBoostType::ALL.len(), 4);
    assert_eq!(DIVINE_BOOST_BASE_DURATION_GAME_HOURS, 1);
    assert_eq!(
        DivineBoostType::ALL
            .into_iter()
            .map(DivineBoostType::base_cost_per_hour)
            .collect::<Vec<_>>(),
        [
            VoidInsight::from_whole(2).unwrap(),
            VoidInsight::ONE,
            VoidInsight::from_whole(2).unwrap(),
            VoidInsight::from_whole(2).unwrap(),
        ]
    );
    assert_eq!(
        DivineBoostType::BountifulLabor.effect_domains(),
        ["raw_gathering", "carrying", "harvesting"]
    );
    assert_eq!(DivineBoostType::FleetPaws.effect_domains(), ["movement"]);
    assert_eq!(
        DivineBoostType::InspiredWork.effect_domains(),
        ["construction", "production"]
    );
    assert_eq!(
        DivineBoostType::RestorativeGrace.effect_domains(),
        ["healing"]
    );
    assert_eq!(active_effect_factor(DivineBoostType::InspiredWork), 15_000);
}

#[test]
fn duration_choices_and_economy_reduction_have_exact_checked_void_cost() {
    assert_eq!(
        UnlockedBoostDurations::for_stage(11).durations_hours(),
        [1, 2, 3, 4, 6, 8, 10, 12, 16, 18, 21, 24]
    );
    assert_eq!(DIVINE_BOOST_DURATION_HOURS[0], 1);
    assert_eq!(
        boost_cost(
            DivineBoostType::FleetPaws,
            21,
            DivineBoostResearchStages {
                divine_duration_stage: 10,
                divine_economy_stage: 11,
            },
        )
        .unwrap(),
        VoidInsight::from_micro(14_070_000)
    );
    assert_eq!(
        boost_cost(
            DivineBoostType::BountifulLabor,
            1,
            DivineBoostResearchStages {
                divine_duration_stage: 0,
                divine_economy_stage: 1,
            },
        )
        .unwrap(),
        VoidInsight::from_micro(1_940_000)
    );
    assert_eq!(
        boost_cost(
            DivineBoostType::FleetPaws,
            u32::MAX,
            DivineBoostResearchStages::default(),
        ),
        Err(DivineBoostError::DurationLocked)
    );
}

#[test]
fn manifest_effects_drive_duration_and_economy_without_shadow_stage_ids() {
    assert_eq!(
        DivineBoostResearchStages::from_manifest_effects(std::iter::empty::<&ManifestEffect>())
            .unwrap(),
        DivineBoostResearchStages::default()
    );
    let manifest = research_manifest();
    for (index, study) in manifest
        .track_studies(ManifestTrack::DivineDuration)
        .into_iter()
        .enumerate()
    {
        let expected_stage = u8::try_from(index + 1).unwrap();
        let [effect] = study.effects.as_slice() else {
            panic!("duration stage has one effect");
        };
        let ManifestEffect::DivineDuration {
            stage,
            max_duration_game_hours,
        } = effect
        else {
            panic!("typed duration effect");
        };
        assert_eq!(*stage, expected_stage);
        assert_eq!(
            u32::from(*max_duration_game_hours),
            DIVINE_BOOST_DURATION_HOURS[usize::from(expected_stage)]
        );
    }
    for (index, study) in manifest
        .track_studies(ManifestTrack::DivineEconomy)
        .into_iter()
        .enumerate()
    {
        let expected_stage = u8::try_from(index + 1).unwrap();
        let [effect] = study.effects.as_slice() else {
            panic!("economy stage has one effect");
        };
        let ManifestEffect::DivineEconomy {
            stage,
            discount_basis_points,
        } = effect
        else {
            panic!("typed economy effect");
        };
        assert_eq!(*stage, expected_stage);
        assert_eq!(*discount_basis_points, u16::from(expected_stage) * 300);
    }
    assert_eq!(
        manifest_boost_stages(11, 11),
        DivineBoostResearchStages {
            divine_duration_stage: 11,
            divine_economy_stage: 11,
        }
    );
    assert_eq!(
        DivineBoostResearchStages::from_manifest_effects([&ManifestEffect::DivineDuration {
            stage: 1,
            max_duration_game_hours: 24,
        }]),
        Err(DivineBoostError::MalformedResearchEffect)
    );
}

#[test]
fn only_authenticated_owning_players_with_the_unlock_can_activate() {
    let mut state = DivineBoostState::new(colony());
    let mut ledger = funded_ledger(10);
    let unlocked = progression(0, 0, &[DivineBoostType::FleetPaws]);
    let base = request(&state, &ledger, 1, DivineBoostType::FleetPaws, 1, 100);
    for authorization in [
        DivineBoostAuthorization {
            actor: DivineBoostActor::Automated {
                actor_id: automated(),
            },
            authenticated_player_id: None,
            owns_colony: true,
        },
        DivineBoostAuthorization {
            actor: DivineBoostActor::Player {
                player_id: player(),
            },
            authenticated_player_id: None,
            owns_colony: true,
        },
        DivineBoostAuthorization {
            actor: DivineBoostActor::Player {
                player_id: player(),
            },
            authenticated_player_id: Some(player()),
            owns_colony: false,
        },
    ] {
        let rejected = DivineBoostPurchaseRequest {
            authorization,
            ..base.clone()
        };
        assert_eq!(
            state.purchase(&mut ledger, &unlocked, rejected),
            Err(DivineBoostError::Unauthorized)
        );
    }
    let locked = base;
    let no_unlocks = progression(0, 0, &[]);
    assert_eq!(
        state.purchase(&mut ledger, &no_unlocks, locked),
        Err(DivineBoostError::BoostLocked)
    );
    assert_eq!(ledger.balance, VoidInsight::from_whole(10).unwrap());
    assert!(state.active_boosts().is_empty());
}

#[test]
fn purchase_is_atomic_idempotent_conflict_safe_and_same_type_cannot_reset() {
    let mut state = DivineBoostState::new(colony());
    let mut ledger = funded_ledger(10);
    let progression = progression(3, 0, &[DivineBoostType::FleetPaws]);
    let first = request(&state, &ledger, 1, DivineBoostType::FleetPaws, 4, 100);
    assert_eq!(
        state
            .purchase(&mut ledger, &progression, first.clone())
            .unwrap(),
        DivineBoostOutcome::Committed
    );
    assert_eq!(ledger.balance, VoidInsight::from_whole(6).unwrap());
    let active = state.active(DivineBoostType::FleetPaws).unwrap().clone();
    assert_eq!(active.expires_tick, 340);

    let replay = DivineBoostPurchaseRequest {
        expected_boost_version: 0,
        expected_void_version: 1,
        ..first.clone()
    };
    assert_eq!(
        state.purchase(&mut ledger, &progression, replay).unwrap(),
        DivineBoostOutcome::AlreadyCommitted
    );
    let before_state = state.clone();
    let before_ledger = ledger.clone();
    let conflict = DivineBoostPurchaseRequest {
        duration_hours: 1,
        ..first
    };
    assert_eq!(
        state.purchase(&mut ledger, &progression, conflict),
        Err(DivineBoostError::PurchaseIdConflict)
    );
    assert_eq!(state, before_state);
    assert_eq!(ledger, before_ledger);

    let same_type = request(&state, &ledger, 2, DivineBoostType::FleetPaws, 1, 120);
    assert_eq!(
        state.purchase(&mut ledger, &progression, same_type),
        Err(DivineBoostError::ActiveSameType)
    );
    assert_eq!(state.active(DivineBoostType::FleetPaws), Some(&active));
}

#[test]
fn expiry_restart_partition_and_different_type_overlap_are_exact() {
    let mut state = DivineBoostState::new(colony());
    let mut ledger = funded_ledger(20);
    let mut progression_state = progression(1, 0, &[DivineBoostType::FleetPaws]);
    let fleet = request(&state, &ledger, 1, DivineBoostType::FleetPaws, 2, 10);
    state
        .purchase(&mut ledger, &progression_state, fleet)
        .unwrap();
    progression_state = progression(
        5,
        11,
        &[DivineBoostType::FleetPaws, DivineBoostType::BountifulLabor],
    );
    let labor = request(&state, &ledger, 2, DivineBoostType::BountifulLabor, 8, 20);
    state
        .purchase(&mut ledger, &progression_state, labor)
        .unwrap();
    assert_eq!(state.active_boosts().len(), 2);
    assert!(state.expire_due(129).unwrap().is_empty());
    assert_eq!(state.expire_due(130).unwrap().len(), 1);
    assert!(state.active(DivineBoostType::FleetPaws).is_none());
    assert!(state.active(DivineBoostType::BountifulLabor).is_some());

    let restarted: DivineBoostState =
        serde_json::from_value(serde_json::to_value(&state).unwrap()).unwrap();
    assert_eq!(restarted, state);

    let other_colony = PlannerId::derive("lai44_boost_colony", ["other"]);
    let wrong_partition = DivineBoostPurchaseRequest {
        id: DivineBoostPurchaseId::derive(&other_colony, &player(), 3),
        partition: PlayerPartitionKey {
            colony_id: other_colony,
            player_id: player(),
        },
        player_sequence: 3,
        authorization: DivineBoostAuthorization {
            actor: DivineBoostActor::Player {
                player_id: player(),
            },
            authenticated_player_id: Some(player()),
            owns_colony: true,
        },
        boost_type: DivineBoostType::InspiredWork,
        duration_hours: 1,
        expected_boost_version: state.version,
        expected_void_version: ledger.version,
        activated_tick: 200,
        ticks_per_game_hour: 60,
    };
    assert_eq!(
        state.purchase(&mut ledger, &progression_state, wrong_partition),
        Err(DivineBoostError::PartitionMismatch)
    );
}

#[test]
fn expired_receipt_drain_reopens_bounded_history_and_old_replay_is_safe() {
    let mut state = DivineBoostState::new(colony());
    let mut ledger = funded_ledger(10);
    let progression = progression(0, 0, &[DivineBoostType::FleetPaws]);
    let first = request(&state, &ledger, 1, DivineBoostType::FleetPaws, 1, 0);
    state
        .purchase(&mut ledger, &progression, first.clone())
        .unwrap();
    state.expire_due(60).unwrap();
    assert_eq!(
        state
            .drain_expired_purchase_receipts(&mut ledger, &player(), 1)
            .unwrap(),
        1
    );
    assert!(state.purchases.is_empty());
    assert!(ledger.spends.is_empty());
    assert_eq!(
        state.purchase(&mut ledger, &progression, first).unwrap(),
        DivineBoostOutcome::RetiredReplay
    );
    let next = request(&state, &ledger, 2, DivineBoostType::FleetPaws, 1, 60);
    state
        .purchase(&mut ledger, &progression, next)
        .expect("new sequence continues after drain");
}

#[test]
fn strict_future_unknown_malformed_bounds_and_tick_overflow_fail_closed() {
    let state = DivineBoostState::new(colony());
    let mut future = serde_json::to_value(&state).unwrap();
    future["schemaVersion"] = serde_json::json!(99);
    assert!(serde_json::from_value::<DivineBoostState>(future).is_err());

    let mut unknown = serde_json::to_value(&state).unwrap();
    unknown["unknown"] = serde_json::json!(true);
    assert!(serde_json::from_value::<DivineBoostState>(unknown).is_err());

    let mut ledger = funded_ledger(10);
    let progression = progression(0, 0, &[DivineBoostType::FleetPaws]);
    let overflow = request(&state, &ledger, 1, DivineBoostType::FleetPaws, 1, u64::MAX);
    assert_eq!(
        state.clone().purchase(&mut ledger, &progression, overflow),
        Err(DivineBoostError::TickOverflow)
    );
    assert_eq!(
        state
            .clone()
            .drain_expired_purchase_receipts(&mut ledger, &player(), 0),
        Err(DivineBoostError::CapacityExceeded)
    );
}
