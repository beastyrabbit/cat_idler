use cat_protocol::{
    AnatomySnapshot, AutomaticResearchQuotaSnapshot, BeliefReportSnapshot, BoundedAgeMs,
    BoundedBasisPoints, CareStatusSnapshot, CatCareSnapshot, CatPersonalitySnapshot,
    CatTraitsSnapshot, ColonyAiSnapshot, ConsentSnapshot, CurrentVersionHint, DiplomacySnapshot,
    DivineBoostSnapshot, FavorEventSnapshot, FavorLedgerSnapshot, InsightSnapshot,
    LAI24_SNAPSHOT_SCHEMA_VERSION, LeaderAiSnapshotEnvelope, MANIFEST_STUDY_COUNT,
    NamedBasisPointSnapshot, NonEmptyStableId, OfferingPackageSnapshot, OfferingStageSnapshot,
    OfficerRequestSnapshot, PlanQueueSnapshot, PlanReasonSnapshot, PlanSnapshot,
    ProstheticSnapshot, ProstheticWearSnapshot, PublicVillageSnapshot, RegenerationReportSnapshot,
    RelationshipSnapshot, ReportEstimateSnapshot, ReportProvenanceSnapshot, ReportSafeString,
    ResearchFrontierSnapshot, ResearchStudySnapshot, ReservationSummarySnapshot,
    ScholarPreparationSnapshot, ShrineOfferingPipelineSnapshot, ShrineSnapshot,
    SiteLifecycleStageSnapshot, SiteRefSnapshot, SiteSnapshot, SiteVisibilitySnapshot,
    SnapshotProtocolVersion, SnapshotTilePoint as TilePoint,
    SnapshotVillageCapabilities as VillageCapabilities, StressSnapshot, TaskCargoSnapshot,
    TradeCargoSnapshot, TradeContractSnapshot, TradeEscrowSnapshot, TradeRouteSnapshot,
    TradeStageSnapshot, VisibleTaskSnapshot, WillingnessSnapshot, WorkSlotSnapshot,
    WorkshopFootprintSnapshot,
};
use serde_json::{Value, json};

fn id(value: &str) -> NonEmptyStableId {
    NonEmptyStableId::new(value).expect("valid stable id")
}

fn text(value: &str) -> ReportSafeString {
    ReportSafeString::new(value).expect("valid report-safe text")
}

fn bp(value: u16) -> BoundedBasisPoints {
    BoundedBasisPoints::new(value).expect("valid basis points")
}

fn site(site_id: &str, tile: TilePoint) -> SiteRefSnapshot {
    SiteRefSnapshot::Tile {
        site: SiteSnapshot {
            site_id: id(site_id),
            visibility: SiteVisibilitySnapshot::Visible,
            lifecycle_stage: SiteLifecycleStageSnapshot::Active,
            blocked_reason: None,
        },
        tile,
    }
}

fn estimate(minimum: i64, maximum: i64, unit: &str) -> ReportEstimateSnapshot {
    ReportEstimateSnapshot {
        minimum,
        maximum,
        unit: text(unit),
    }
}

fn workshop_tiles(anchor: TilePoint) -> Vec<TilePoint> {
    (0..3)
        .flat_map(|dy| {
            (0..3).map(move |dx| TilePoint {
                x: anchor.x + dx,
                y: anchor.y + dy,
            })
        })
        .collect()
}

fn sample_envelope() -> LeaderAiSnapshotEnvelope {
    let capabilities = VillageCapabilities {
        can_view: true,
        can_control: true,
        is_owner: true,
    };
    let anchor = TilePoint { x: 10, y: 20 };
    let workshop = SiteRefSnapshot::BuildingFootprint {
        site: SiteSnapshot {
            site_id: id("site:workshop"),
            visibility: SiteVisibilitySnapshot::Visible,
            lifecycle_stage: SiteLifecycleStageSnapshot::Active,
            blocked_reason: None,
        },
        building_id: id("building:workshop"),
        building_kind: text("workshop"),
        anchor,
        width: 3,
        height: 3,
        ordered_tiles: workshop_tiles(anchor),
    };
    let provenance = ReportProvenanceSnapshot {
        source_report_ids: vec![id("observation:food")],
        observer_id: Some(id("cat:accountant")),
        method: text("physical count"),
    };
    let reports = vec![BeliefReportSnapshot {
        report_id: id("report:food"),
        report_version: 2,
        subject_id: id("stockpile:main"),
        domain: text("food"),
        estimate: estimate(80, 100, "units"),
        confidence_basis_points: bp(8_000),
        age_ms: BoundedAgeMs::new(1_000),
        observed_at_ms: 9_000,
        expires_at_ms: 20_000,
        report_level: 4,
        provenance: provenance.clone(),
        contradicts_report_ids: Vec::new(),
        replaces_report_id: None,
        unavailable_reason: None,
        regeneration: RegenerationReportSnapshot::Estimated {
            level_4_or_higher: true,
            estimate: estimate(60_000, 90_000, "milliseconds"),
            provenance,
        },
    }];
    let plans = PlanQueueSnapshot {
        planner_version: 4,
        planning_epoch: 3,
        plans: vec![PlanSnapshot {
            plan_id: id("plan:workshop"),
            intent_id: id("intent:workshop"),
            lifecycle_state: text("active"),
            responsible_actor_id: id("cat:leader"),
            responsible_office: Some(text("steward")),
            dependency_intent_ids: Vec::new(),
            score_bucket: 12,
            rationale: text("reported tool need"),
            expected_cost: estimate(30, 40, "supplies"),
            expected_benefit: estimate(50, 70, "utility"),
            reasons: vec![PlanReasonSnapshot {
                code: id("reason:tool-shortage"),
                summary: text("reported tool shortage"),
                confidence_basis_points: bp(8_000),
                source_report_ids: vec![id("report:food")],
            }],
        }],
    };
    let officer_requests = vec![OfficerRequestSnapshot {
        request_id: id("request:workshop"),
        request_version: 2,
        office: text("steward"),
        domain: text("construction"),
        requested_action: text("build workshop"),
        budget: estimate(30, 40, "supplies"),
        priority_basis_points: bp(7_500),
        source_report_ids: vec![id("report:food")],
        expires_at_ms: 30_000,
        merged_into_request_id: None,
        supersedes_request_ids: Vec::new(),
        blocked_reason: None,
    }];
    let visible_tasks = vec![VisibleTaskSnapshot {
        task_id: id("task:workshop"),
        intent_id: id("intent:workshop"),
        category: text("construction"),
        stage: text("working"),
        assigned_cat_ids: vec![id("cat:builder")],
        objective: workshop.clone(),
        work_slots: vec![WorkSlotSnapshot {
            slot_id: id("slot:workshop:0"),
            tile: anchor,
            state: text("occupied"),
        }],
        endpoint: Some(site("stockpile:endpoint", TilePoint { x: 9, y: 20 })),
        footprint: workshop_tiles(anchor),
        progress_basis_points: bp(4_000),
        reservations: ReservationSummarySnapshot {
            reservation_ids: vec![id("reservation:workshop")],
            reservation_version: 2,
        },
        blocked_reason: None,
        cargo: TaskCargoSnapshot {
            cargo_ids: vec![id("cargo:supplies")],
            summary: text("supplies in transit"),
        },
        last_updated_at_ms: 10_000,
    }];
    let cats = vec![CatCareSnapshot {
        cat_id: id("cat:builder"),
        display_name: text("Moss"),
        active_task_id: Some(id("task:workshop")),
        traits: CatTraitsSnapshot {
            innate_attributes: vec![NamedBasisPointSnapshot {
                name: id("strength"),
                value_basis_points: bp(6_500),
            }],
            learned_skills: Vec::new(),
            office_experience: Vec::new(),
            acquired_traits: vec![id("seasoned_builder")],
        },
        personality: CatPersonalitySnapshot {
            sociability: bp(5_000),
            diligence: bp(7_000),
            courage: bp(6_000),
            empathy: bp(4_000),
            curiosity: bp(5_500),
        },
        stress: StressSnapshot {
            stress_basis_points: bp(2_000),
            recovery_basis_points: bp(8_000),
            refusing: false,
            refusal_reason: None,
        },
        willingness: WillingnessSnapshot {
            total_basis_points: bp(8_500),
            factors: Vec::new(),
            eligible: true,
            eligibility_reason: None,
        },
        anatomy: AnatomySnapshot {
            body_parts: Vec::new(),
        },
        prosthetics: vec![ProstheticSnapshot {
            prosthetic_id: id("prosthetic:left-paw"),
            body_part_id: id("left_forepaw"),
            prosthetic_kind: text("wooden paw"),
            restoration_basis_points: bp(7_500),
            wear: ProstheticWearSnapshot {
                durability_basis_points: bp(9_000),
                wear_basis_points: bp(1_000),
                repair_eligible: false,
                repair_reason: Some(text("not worn enough")),
            },
            fitting_task_id: None,
            repair_task_id: None,
        }],
        care: CareStatusSnapshot {
            care_site: None,
            treatment_task_id: None,
            fitting_task_id: None,
            repair_task_id: None,
            status: text("healthy"),
        },
    }];
    let shrine_endpoint = site("shrine:main", TilePoint { x: 0, y: 0 });
    let shrine = ShrineSnapshot {
        shrine_id: id("shrine:main"),
        endpoint: shrine_endpoint.clone(),
        pipeline: Some(ShrineOfferingPipelineSnapshot {
            offering_id: id("offering:1"),
            package: OfferingPackageSnapshot {
                package_id: id("package:food"),
                package_kind: text("food"),
                cargo_ids: vec![id("cargo:offering")],
                favor_reward_micro_favor: 1_000_000,
            },
            stage: OfferingStageSnapshot::Hauling {
                carrier_cat_id: id("cat:builder"),
            },
            source_report_ids: vec![id("report:food")],
            shrine_endpoint,
            cargo_disposition: text("in transit"),
            rationale: text("best reported replacement cost"),
            blocked_reason: None,
        }),
    };
    let favor = FavorLedgerSnapshot {
        ledger_version: 3,
        micro_favor: 2_000_000,
        favor_events: vec![FavorEventSnapshot {
            event_id: id("favor:event:1"),
            delta_micro_favor: 1_000_000,
            resulting_micro_favor: 2_000_000,
            occurred_at_ms: 8_000,
            reason: text("offering complete"),
        }],
    };
    let research = ResearchFrontierSnapshot {
        research_version: 5,
        manifest_study_count: MANIFEST_STUDY_COUNT,
        owned_study_ids: vec![id("study:duration:1")],
        frontier: vec![ResearchStudySnapshot {
            study_id: id("study:duration:2"),
            display_name: text("Divine Duration II"),
            prerequisite_ids: vec![id("study:duration:1")],
            price_micro_favor: 2_000_000,
            prepared_price_micro_favor: Some(1_500_000),
        }],
        automatic_quota: AutomaticResearchQuotaSnapshot {
            quota_used: 1,
            quota_limit: 2,
            quota_window_started_at_ms: 0,
        },
        insight: InsightSnapshot {
            insight_balance: 220,
            generated_this_week: 20,
            week_started_at_ms: Some(0),
        },
        preparations: vec![ScholarPreparationSnapshot {
            preparation_id: id("preparation:duration:2"),
            study_id: id("study:duration:2"),
            scholar_cat_id: Some(id("cat:scholar")),
            progress_basis_points: bp(10_000),
            committed_insight_cost: 200,
            player_discount_basis_points: bp(2_500),
            prepared: true,
        }],
    };
    let boosts = vec![DivineBoostSnapshot {
        boost_id: id("boost:harvest"),
        boost_kind: text("harvest"),
        effect_basis_points: bp(5_000),
        boost_price_micro_favor: 1_000_000,
        duration_ms: 60_000,
        boost_started_at_ms: 10_000,
        boost_expires_at_ms: 70_000,
        effect_stage: 1,
    }];
    let diplomacy = DiplomacySnapshot {
        diplomacy_version: 2,
        relationships: vec![RelationshipSnapshot {
            relationship_id: id("relationship:other"),
            other_colony_id: id("colony:other"),
            relationship_version: 2,
            state: text("friendly"),
            consent: ConsentSnapshot {
                local_approved: true,
                remote_approved: true,
                consent_version: 2,
            },
            updated_at_ms: 10_000,
        }],
    };
    let trade = vec![TradeContractSnapshot {
        contract_id: id("trade:1"),
        contract_version: 2,
        partner_colony_id: id("colony:other"),
        stage: TradeStageSnapshot::Outbound,
        actor_cat_ids: vec![id("cat:trader")],
        valuation_report_ids: vec![id("report:food")],
        valuation_confidence_basis_points: bp(8_000),
        escrow: TradeEscrowSnapshot {
            escrow_id: id("escrow:1"),
            cargo_ids: vec![id("cargo:trade")],
            released: false,
        },
        route: TradeRouteSnapshot {
            route_id: id("route:trade"),
            ordered_tiles: vec![TilePoint { x: 0, y: 0 }, TilePoint { x: 1, y: 0 }],
            endpoint: site("trade:endpoint", TilePoint { x: 1, y: 0 }),
        },
        cargo: vec![TradeCargoSnapshot {
            cargo_id: id("cargo:trade"),
            cargo_kind: text("food"),
            quantity: 5,
            state: text("in transit"),
        }],
        next_event_at_ms: Some(12_000),
        reservations: ReservationSummarySnapshot {
            reservation_ids: vec![id("reservation:trade")],
            reservation_version: 1,
        },
        bounded_failure: None,
        recovery_state: None,
    }];

    LeaderAiSnapshotEnvelope {
        protocol_version: SnapshotProtocolVersion::current(),
        schema_version: LAI24_SNAPSHOT_SCHEMA_VERSION,
        now_ms: 10_000,
        world_seed: 42,
        selected_colony_id: id("colony:home"),
        public_villages: vec![PublicVillageSnapshot {
            colony_id: id("colony:home"),
            display_name: text("Home"),
            capabilities,
        }],
        colonies: vec![ColonyAiSnapshot {
            colony_id: id("colony:home"),
            state_version: 7,
            action_versions: CurrentVersionHint::default(),
            capabilities,
            reports,
            plans,
            officer_requests,
            officer_institution: None,
            standing_orders: vec![],
            refresh_hints: vec![],
            visible_tasks,
            cats,
            shrine,
            favor,
            research,
            boosts,
            diplomacy,
            trade,
        }],
    }
}

#[test]
fn full_lai24_snapshot_round_trips_and_validates() {
    let original = sample_envelope();
    original.validate().expect("sample must satisfy bounds");

    let encoded = serde_json::to_string(&original).expect("serialize envelope");
    let decoded =
        LeaderAiSnapshotEnvelope::decode_json(&encoded).expect("decode versioned envelope");

    assert_eq!(decoded, original);
    decoded.validate().expect("round-trip remains valid");
}

#[test]
fn workshop_footprint_requires_exact_three_by_three_row_major_tiles() {
    let anchor = TilePoint { x: -3, y: 7 };
    let valid = WorkshopFootprintSnapshot {
        anchor,
        width: 3,
        height: 3,
        ordered_tiles: workshop_tiles(anchor),
    };
    valid
        .validate_workshop_three_by_three()
        .expect("canonical workshop");
    assert_eq!(
        serde_json::from_str::<WorkshopFootprintSnapshot>(
            &serde_json::to_string(&valid).expect("serialize workshop")
        )
        .expect("deserialize workshop"),
        valid
    );

    let mut wrong_dimensions = valid.clone();
    wrong_dimensions.width = 2;
    assert!(wrong_dimensions.validate_workshop_three_by_three().is_err());

    let mut wrong_order = valid;
    wrong_order.ordered_tiles.swap(0, 1);
    assert!(wrong_order.validate_nine_row_major_tiles().is_err());
}

#[test]
fn deserialize_unknown_lai24_variant_fails() {
    let unknown_site = json!({
        "kind": "unrevealed_source",
        "site": {
            "siteId": "site:unknown",
            "visibility": "visible",
            "lifecycleStage": "active",
            "blockedReason": null
        }
    });
    assert!(serde_json::from_value::<SiteRefSnapshot>(unknown_site).is_err());

    let mut envelope = serde_json::to_value(sample_envelope()).expect("serialize sample");
    envelope["colonies"][0]["visibleTasks"][0]["objective"]["kind"] =
        Value::String("unrevealed_source".into());
    let error = LeaderAiSnapshotEnvelope::decode_json(
        &serde_json::to_string(&envelope).expect("serialize malformed envelope"),
    )
    .expect_err("unknown nested variant must fail");
    assert!(matches!(
        error,
        cat_protocol::SnapshotDecodeError::UnknownSnapshotVariant
    ));
}

#[test]
fn all_site_ref_variants_round_trip_without_losing_ordered_footprints() {
    let metadata = || SiteSnapshot {
        site_id: id("site:round-trip"),
        visibility: SiteVisibilitySnapshot::Reported,
        lifecycle_stage: SiteLifecycleStageSnapshot::Reserved,
        blocked_reason: Some(text("awaiting worker")),
    };
    let anchor = TilePoint { x: 4, y: -2 };
    let route = vec![anchor, TilePoint { x: 5, y: -2 }];
    let variants = vec![
        SiteRefSnapshot::Tile {
            site: metadata(),
            tile: anchor,
        },
        SiteRefSnapshot::AnchoredRect {
            site: metadata(),
            anchor,
            width: 2,
            height: 1,
        },
        SiteRefSnapshot::OrderedTileSet {
            site: metadata(),
            ordered_tiles: route.clone(),
        },
        SiteRefSnapshot::BuildingFootprint {
            site: metadata(),
            building_id: id("building:house"),
            building_kind: text("house"),
            anchor,
            width: 2,
            height: 1,
            ordered_tiles: route.clone(),
        },
        SiteRefSnapshot::StockpileFootprint {
            site: metadata(),
            stockpile_id: id("stockpile:main"),
            ordered_tiles: route.clone(),
        },
        SiteRefSnapshot::ResourceSource {
            site: metadata(),
            source_id: id("source:tree"),
            resource_kind: text("wood"),
            ordered_tiles: route.clone(),
        },
        SiteRefSnapshot::HuntSource {
            site: metadata(),
            cave_id: id("cave:north"),
            source_tile: anchor,
        },
        SiteRefSnapshot::WaterSourceAndBank {
            site: metadata(),
            source_tile: anchor,
            bank_tile: TilePoint { x: 5, y: -2 },
        },
        SiteRefSnapshot::OrderedRoute {
            site: metadata(),
            route_id: id("route:north"),
            ordered_tiles: route,
        },
        SiteRefSnapshot::Shrine {
            site: metadata(),
            shrine_id: id("shrine:main"),
            endpoint: anchor,
        },
        SiteRefSnapshot::VillageEndpoint {
            site: metadata(),
            colony_id: id("colony:home"),
            endpoint: anchor,
        },
        SiteRefSnapshot::TradeEndpoint {
            site: metadata(),
            contract_id: id("trade:1"),
            colony_id: id("colony:other"),
            endpoint: anchor,
        },
    ];

    for variant in variants {
        let encoded = serde_json::to_string(&variant).expect("serialize SiteRef");
        let decoded: SiteRefSnapshot = serde_json::from_str(&encoded).expect("deserialize SiteRef");
        assert_eq!(decoded, variant);
    }
}

#[test]
fn unknown_fields_and_protocol_versions_fail_during_deserialization() {
    let mut encoded = serde_json::to_value(sample_envelope()).expect("serialize sample");
    encoded
        .as_object_mut()
        .expect("envelope object")
        .insert("unexpectedPrivateState".into(), Value::Bool(true));
    assert!(serde_json::from_value::<LeaderAiSnapshotEnvelope>(encoded).is_err());

    let mut nested_tile = serde_json::to_value(sample_envelope()).expect("serialize sample");
    nested_tile["colonies"][0]["visibleTasks"][0]["objective"]["orderedTiles"][0]
        .as_object_mut()
        .expect("objective tile object")
        .insert("unexpectedCoordinate".into(), Value::from(1));
    assert!(serde_json::from_value::<LeaderAiSnapshotEnvelope>(nested_tile).is_err());

    let mut nested_capabilities =
        serde_json::to_value(sample_envelope()).expect("serialize sample");
    nested_capabilities["colonies"][0]["capabilities"]
        .as_object_mut()
        .expect("capabilities object")
        .insert("unexpectedPermission".into(), Value::Bool(true));
    assert!(serde_json::from_value::<LeaderAiSnapshotEnvelope>(nested_capabilities).is_err());

    let mut unsupported = serde_json::to_value(sample_envelope()).expect("serialize sample");
    unsupported
        .as_object_mut()
        .expect("envelope object")
        .insert("protocolVersion".into(), Value::from(999_u64));
    unsupported["colonies"][0]["visibleTasks"][0]["objective"]["kind"] =
        Value::String("unrevealed_source".into());
    let error = LeaderAiSnapshotEnvelope::decode_json(
        &serde_json::to_string(&unsupported).expect("serialize unsupported envelope"),
    )
    .expect_err("unsupported protocol must fail before malformed nested payload");
    assert_eq!(
        error,
        cat_protocol::SnapshotDecodeError::UnsupportedProtocolVersion(999)
    );
}

#[test]
fn malformed_numeric_and_aggregate_bounds_fail_closed() {
    assert!(serde_json::from_value::<BoundedBasisPoints>(json!(10_001)).is_err());
    assert!(serde_json::from_value::<NonEmptyStableId>(json!("")).is_err());
    assert!(serde_json::from_value::<ReportSafeString>(json!("x".repeat(513))).is_err());

    let mut envelope = sample_envelope();
    envelope.colonies[0].plans.plans = vec![envelope.colonies[0].plans.plans[0].clone(); 9];
    assert!(envelope.validate().is_err());
}
