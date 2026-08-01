//! Reusable LAI.33 signed journey setup over the production routing guards.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use cat_protocol::{
    ActionIdempotencyId, ActionProtocolVersion, AuthenticatedPlayerId, BoundedBasisPointNudge,
    BoundedEntityId, ExpectedStateVersions, LeaderAiActionEnvelope, LeaderAiActionPayload,
    SelectedColonyId,
};
use cat_sim::{
    anatomy::{BodyPart, BodyPartCondition},
    authority::{AuthorityActor, AuthorityDomain},
    autonomous_trade::{
        TradeAuthorization, TradeCargoLeg, TradeColonyKind, TradeLedger, TradeParty, TradeProposal,
    },
    beliefs::{
        BeliefKey, BeliefKind, BeliefProjection, Confidence, EstimateRange, EvidenceId,
        EvidenceSource, ProjectedBeliefValue, ReportLevel,
    },
    diplomacy::{
        DiplomacyAction, DiplomacyActionId, DiplomacyActionKind, DiplomacyAuthorization,
        DiplomacyColonyId, DiplomacyPair, DiplomacyRelationship, ProposedRelationship,
    },
    favor::{Favor, FavorEventId, FavorEventKind},
    intent_graph::Intent,
    planner_core::{IntentId, PlannerId},
    prosthetics::{ProstheticId, ProstheticMaterial},
    shrine_offerings::{OfferingChoice, OfferingPackage, ShrineOfferingState},
    spatial_resolver::{ResolvedSpatialTask, SpatialTaskCategory},
    spatial_tasks::{
        Rect, ResourceSourceKind, SiteMetadata, SiteRef, SpatialObjective, TaskFootprint,
        TilePoint, WorkSlot,
    },
    task_runtime::{TaskCategory, VisibleTaskRuntime},
    trade_valuation::{TradePersonality, TradePurpose, TradeValuation},
    types::BuildingType,
    world_reservations::{CapacityReservation, WorldReservationTransaction},
    world_tick::ColonyRuntime,
};
use sha2::Digest;

use crate::{
    identity::{SignedSession, signed_session},
    leader_ai_action_routing::{
        ColonyControlPolicy, IdempotencyReceiptStore, LeaderAiServerMutationPipeline,
        SelectedColonyOwnershipSource, ServerActionConflict,
    },
    persistence,
};

pub const LAI33_FRESH_SEED: u32 = 0x5333_A001;
pub const LAI33_MIGRATED_SEED: u32 = 0x5333_A002;
const LAI33_GLOBAL_ID: &str = "global";

#[derive(Debug, Clone)]
pub struct Lai33SignedSystemJourneyHarness {
    pub secret: String,
    pub session: SignedSession,
    ownership: BTreeMap<String, ColonyControlPolicy>,
    pub receipts: IdempotencyReceiptStore,
}

#[must_use]
pub fn lai33_fixture_world() -> cat_sim::world_tick::WorldState {
    let mut world = cat_sim::world_tick::new_world(LAI33_FRESH_SEED);
    let mut global =
        cat_sim::world_tick::found_global_colony(world.world_seed, LAI33_GLOBAL_ID, 0, 11);
    let mut fresh = cat_sim::world_tick::found_colony(world.world_seed, "colony-fresh-a", 0, 12);
    fresh.kind = cat_sim::world_tick::VillageKind::Personal;
    fresh.owner_player_id = Some("player-fresh".to_owned());
    let mut migrated =
        cat_sim::world_tick::found_colony(world.world_seed, "colony-migrated-a", 0, 22);
    migrated.kind = cat_sim::world_tick::VillageKind::Personal;
    migrated.owner_player_id = Some("player-migrated".to_owned());
    let mut foreign =
        cat_sim::world_tick::found_colony(world.world_seed, "colony-foreign-b", 0, 23);
    foreign.kind = cat_sim::world_tick::VillageKind::Personal;
    foreign.owner_player_id = Some("player-foreign".to_owned());
    seed_global_browser_acceptance(&mut global, world.world_seed);
    for colony_id in [fresh.id.as_str(), migrated.id.as_str(), foreign.id.as_str()] {
        global.known_village_ids.insert(colony_id.to_owned());
    }
    for colony in [&mut fresh, &mut migrated, &mut foreign] {
        colony.known_village_ids.insert(LAI33_GLOBAL_ID.to_owned());
    }
    world.colonies.extend([global, fresh, migrated, foreign]);
    world
}

fn point(x: i32, y: i32) -> TilePoint {
    TilePoint { x, y }
}

fn one_tile_footprint(tile: TilePoint) -> TaskFootprint {
    TaskFootprint::rectangular(Rect::new(tile, 1, 1).expect("fixture tile is representable"))
}

fn stockpile_site(id: &str, tile: TilePoint) -> SiteRef {
    SiteRef::Stockpile {
        metadata: SiteMetadata::revealed(id),
        stockpile_id: id.to_owned(),
        footprint: one_tile_footprint(tile),
    }
}

fn insert_visible_fixture_task(
    colony: &mut ColonyRuntime,
    occurrence: u32,
    label: &str,
    category: TaskCategory,
    spatial: SpatialObjective,
) {
    let intent_id = IntentId::derive(&colony.id, 0, "browser_acceptance", label, occurrence);
    let leader_id = PlannerId::derive("cat", ["browser-fixture-leader"]);
    let mut intent = Intent::proposed(
        intent_id.clone(),
        PlannerId::derive("colony", [colony.id.as_str()]),
        AuthorityActor::Leader {
            cat_id: leader_id.clone(),
        },
        Some(leader_id),
        AuthorityDomain::ColonyWide,
        PlannerId::derive("fixture_task_kind", [label]),
        PlannerId::derive("fixture_task_target", [label]),
        PlannerId::derive("fixture_task_rationale", [label]),
        0,
    );
    intent.spatial_objective = Some(spatial.clone());
    colony
        .leader_ai_runtime
        .intents
        .insert_or_merge(intent)
        .expect("fixture intent must be valid");
    let task = VisibleTaskRuntime::resolved(
        colony.id.clone(),
        intent_id,
        occurrence,
        category,
        spatial,
        Vec::new(),
        0,
    )
    .expect("fixture task must be valid");
    colony
        .leader_ai_runtime
        .scheduling
        .visible_tasks
        .insert(task.id.clone(), task);
}

fn seed_spatial_browser_tasks(colony: &mut ColonyRuntime) {
    let hunt_cave = SiteRef::ResourceSource {
        metadata: SiteMetadata::revealed("fixture-hunt-cave"),
        source_id: "fixture-hunt-cave".to_owned(),
        resource_kind: ResourceSourceKind::Hunting,
        footprint: one_tile_footprint(point(12, 8)),
    };
    let hunt_spatial = SpatialObjective::resolved(
        hunt_cave,
        vec![WorkSlot::exclusive(
            "fixture-hunt-cave-slot",
            SiteRef::Tile {
                metadata: SiteMetadata::revealed("fixture-hunt-cave-work"),
                tile: point(12, 8),
            },
        )],
        Some(stockpile_site("fixture-hunt-endpoint", point(6, 6))),
    );
    insert_visible_fixture_task(colony, 0, "hunt-at-cave", TaskCategory::Hunt, hunt_spatial);

    let water_spatial = SpatialObjective::resolved(
        SiteRef::ResourceSource {
            metadata: SiteMetadata::revealed("fixture-water-source"),
            source_id: "fixture-water-source".to_owned(),
            resource_kind: ResourceSourceKind::Water,
            footprint: one_tile_footprint(point(15, 11)),
        },
        vec![WorkSlot::exclusive(
            "fixture-water-dry-bank-slot",
            SiteRef::Tile {
                metadata: SiteMetadata::revealed("fixture-water-dry-bank"),
                tile: point(15, 10),
            },
        )],
        Some(stockpile_site("fixture-water-endpoint", point(7, 6))),
    );
    insert_visible_fixture_task(
        colony,
        1,
        "water-source-bank-endpoint",
        TaskCategory::FetchWater,
        water_spatial,
    );

    let workshop = SiteRef::building("fixture-workshop", BuildingType::Workshop, point(18, 6));
    let workshop_spatial = SpatialObjective::resolved(
        workshop.clone(),
        vec![WorkSlot::exclusive(
            "fixture-workshop-slot",
            SiteRef::Tile {
                metadata: SiteMetadata::revealed("fixture-workshop-work-cell"),
                tile: point(19, 7),
            },
        )],
        Some(workshop),
    );
    insert_visible_fixture_task(
        colony,
        2,
        "workshop-exact-footprint",
        TaskCategory::WorkshopWork,
        workshop_spatial,
    );
}

fn seed_care_browser_scenarios(colony: &mut ColonyRuntime, world_seed: u32) {
    colony
        .leader_ai_runtime
        .reconcile_legacy_cats(world_seed, &colony.id, &colony.cats);
    let treatment_cat = colony
        .cats
        .first()
        .expect("fixture colony has founding cats")
        .id
        .clone();
    let fitting_cat = colony
        .cats
        .get(1)
        .expect("fixture colony has at least two founding cats")
        .id
        .clone();
    colony
        .leader_ai_runtime
        .cats
        .get_mut(&treatment_cat)
        .expect("reconciled treatment cat")
        .anatomy
        .injure(BodyPart::FrontLeftPaw, BodyPartCondition::Severe);
    colony
        .leader_ai_runtime
        .cats
        .get_mut(&fitting_cat)
        .expect("reconciled fitting cat")
        .anatomy
        .injure(BodyPart::FrontRightPaw, BodyPartCondition::Missing);
    let prosthetic_id = ProstheticId::derive(
        &colony.id,
        BodyPart::FrontRightPaw,
        ProstheticMaterial::Wooden,
        1,
    );
    colony
        .leader_ai_runtime
        .prosthetics
        .register(
            prosthetic_id,
            ProstheticMaterial::Wooden,
            BodyPart::FrontRightPaw,
            colony.id.clone(),
        )
        .expect("fixture prosthetic must be valid");
}

fn seed_shrine_and_favor_browser_scenarios(colony: &mut ColonyRuntime) {
    let shrine_id = format!("{}:shrine", colony.id);
    colony
        .leader_ai_runtime
        .shrine_favor
        .favor
        .credit(
            FavorEventId::derive("browser_fixture", &colony.id, "opening-favor"),
            FavorEventKind::LegacyMigrationCredit,
            Favor::from_whole(24).expect("fixture favor is bounded"),
            0,
            0,
        )
        .expect("fixture favor credit must commit");
    let mut state = ShrineOfferingState::new(&shrine_id);
    state
        .start(
            OfferingChoice {
                package: OfferingPackage::Materials,
                utility_micro_favor: 1_000_000,
                evidence_ids: vec!["fixture-materials-report".to_owned()],
            },
            0,
        )
        .expect("fixture offering pipeline must start");
    colony
        .leader_ai_runtime
        .shrine_favor
        .shrine_offerings
        .insert(shrine_id, state);
}

fn seed_diplomacy_browser_scenario(colony: &mut ColonyRuntime) {
    let acting = DiplomacyColonyId::derive(&colony.id);
    let target = DiplomacyColonyId::derive("colony-foreign-b");
    let pair = DiplomacyPair::new(acting.clone(), target.clone()).expect("distinct fixture pair");
    let actor = PlannerId::derive("player", ["browser-fixture-global"]);
    let target_actor = PlannerId::derive("player", ["browser-fixture-foreign"]);
    let mut apply = |acting_colony: &DiplomacyColonyId,
                     player_id: &PlannerId,
                     occurrence: &str,
                     expected_version: u64,
                     kind: DiplomacyActionKind| {
        colony
            .leader_ai_runtime
            .diplomacy
            .apply(
                DiplomacyAction {
                    id: DiplomacyActionId::derive(pair.id(), acting_colony, occurrence),
                    pair: pair.clone(),
                    acting_colony_id: acting_colony.clone(),
                    expected_version,
                    kind,
                },
                DiplomacyAuthorization {
                    actor: AuthorityActor::God {
                        player_id: player_id.clone(),
                    },
                    acting_colony_id: acting_colony.clone(),
                    owner_player_id: player_id.clone(),
                    player_authorized: true,
                },
            )
            .expect("fixture diplomacy transition must commit");
    };
    apply(
        &acting,
        &actor,
        "propose-friendly",
        0,
        DiplomacyActionKind::Propose(ProposedRelationship::Friendly),
    );
    apply(
        &acting,
        &actor,
        "approve-global",
        1,
        DiplomacyActionKind::Approve,
    );
    apply(
        &target,
        &target_actor,
        "approve-foreign",
        2,
        DiplomacyActionKind::Approve,
    );
}

fn trade_projection(label: &str) -> BeliefProjection {
    let key = BeliefKey::new(
        PlannerId::derive("fixture_trade_domain", [label]),
        PlannerId::derive("fixture_trade_subject", [label]),
        BeliefKind::Stock,
    );
    let reporter = PlannerId::derive("cat", [format!("fixture-reporter-{label}").as_str()]);
    let evidence = EvidenceId::derive(label, &key, 0, &reporter, 0);
    BeliefProjection {
        key,
        value: ProjectedBeliefValue::StockRange(
            EstimateRange::new(100, 100, 100).expect("fixture valuation range"),
        ),
        confidence: Confidence::new(8_000).expect("fixture valuation confidence"),
        observed_tick: 0,
        expires_tick: Some(100),
        source: EvidenceSource::AuthorizedOfficerReport,
        reporter_id: reporter,
        evidence_ids: BTreeSet::from([evidence]),
        report_level: ReportLevel::Three,
    }
}

fn trade_route(id: &str, start: TilePoint) -> SiteRef {
    SiteRef::OrderedRoute {
        metadata: SiteMetadata::revealed(id),
        route: vec![
            start,
            point(start.x.saturating_add(1), start.y),
            point(start.x.saturating_add(2), start.y),
        ],
    }
}

fn trade_spatial(source_id: &str, recipient: &DiplomacyColonyId, x: i32) -> ResolvedSpatialTask {
    let source = SiteRef::ResourceSource {
        metadata: SiteMetadata::revealed(source_id),
        source_id: source_id.to_owned(),
        resource_kind: ResourceSourceKind::FishHabitat,
        footprint: one_tile_footprint(point(x, 20)),
    };
    let work = SiteRef::Tile {
        metadata: SiteMetadata::revealed(format!("{source_id}:bank")),
        tile: point(x, 21),
    };
    let endpoint = SiteRef::VillageTradeEndpoint {
        metadata: SiteMetadata::revealed(format!("trade-endpoint:{}", recipient.external_id())),
        colony_id: recipient.external_id().to_owned(),
        footprint: one_tile_footprint(point(x.saturating_add(8), 21)),
    };
    let spatial = SpatialObjective::resolved(
        source,
        vec![WorkSlot::exclusive(format!("{source_id}:slot"), work)],
        Some(endpoint),
    );
    let resolved = ResolvedSpatialTask {
        category: SpatialTaskCategory::Fish,
        spatial,
        source_to_work_route: trade_route(&format!("{source_id}:pickup-route"), point(x, 20)),
        work_to_delivery_route: trade_route(&format!("{source_id}:delivery-route"), point(x, 21)),
        source_units: 1,
        source_capacity: 1,
        delivery_units: 1,
        delivery_capacity: 1,
        source_to_work_route_capacity: 1,
        work_to_delivery_route_capacity: 1,
    };
    resolved
        .validate()
        .expect("fixture trade route must resolve");
    resolved
}

fn trade_leg(
    owner: &DiplomacyColonyId,
    recipient: &DiplomacyColonyId,
    source_id: &str,
    resource_id: &str,
    occurrence: u32,
    x: i32,
) -> TradeCargoLeg {
    let spatial = trade_spatial(source_id, recipient, x);
    let hauler_id = PlannerId::derive("cat", [format!("fixture-hauler-{source_id}").as_str()]);
    let task_id = PlannerId::derive("trade_task", [source_id]);
    let intent_id = IntentId::derive(
        owner.external_id(),
        0,
        "browser_trade",
        source_id,
        occurrence,
    );
    let resource_stable_id = PlannerId::derive("trade_resource", [resource_id]);
    let escrow = WorldReservationTransaction::new(
        PlannerId::derive("colony", [owner.external_id()]),
        task_id,
        intent_id,
        spatial.clone(),
        hauler_id.clone(),
        Vec::new(),
        vec![CapacityReservation {
            stable_id: resource_stable_id,
            units: 1,
            capacity: 1,
        }],
    )
    .expect("fixture trade escrow must be valid");
    TradeCargoLeg::new(
        owner.clone(),
        recipient.clone(),
        resource_id,
        1,
        spatial,
        escrow,
        hauler_id,
    )
    .expect("fixture trade leg must be valid")
}

fn seed_trade_browser_scenario(colony: &mut ColonyRuntime) {
    let global = DiplomacyColonyId::derive(&colony.id);
    let foreign = DiplomacyColonyId::derive("colony-foreign-b");
    let pair = DiplomacyPair::new(global.clone(), foreign.clone()).expect("fixture trade pair");
    let relationship = DiplomacyRelationship::Friendly;
    let purpose = TradePurpose::Ordinary;
    let valuations = BTreeMap::from([
        (
            global.clone(),
            TradeValuation::evaluate(
                relationship,
                purpose,
                TradePersonality::Balanced,
                &trade_projection("global-offered"),
                &trade_projection("global-requested"),
                0,
            )
            .expect("fixture global trade valuation"),
        ),
        (
            foreign.clone(),
            TradeValuation::evaluate(
                relationship,
                purpose,
                TradePersonality::Balanced,
                &trade_projection("foreign-offered"),
                &trade_projection("foreign-requested"),
                0,
            )
            .expect("fixture foreign trade valuation"),
        ),
    ]);
    let parties = BTreeMap::from([
        (
            global.clone(),
            TradeParty {
                diplomacy_id: global.clone(),
                reservation_colony_id: PlannerId::derive("colony", [global.external_id()]),
                kind: TradeColonyKind::PlayerFounded,
            },
        ),
        (
            foreign.clone(),
            TradeParty {
                diplomacy_id: foreign.clone(),
                reservation_colony_id: PlannerId::derive("colony", [foreign.external_id()]),
                kind: TradeColonyKind::PlayerFounded,
            },
        ),
    ]);
    let actor = AuthorityActor::Leader {
        cat_id: PlannerId::derive("cat", ["fixture-trade-leader"]),
    };
    let proposal = TradeProposal::new(
        pair,
        global.clone(),
        0,
        parties,
        relationship,
        purpose,
        valuations,
        vec![
            trade_leg(&global, &foreign, "fixture-fish-source", "fish", 0, 30),
            trade_leg(&foreign, &global, "fixture-herb-source", "herbs", 0, 50),
        ],
        0,
        1_000_000_000,
        actor.clone(),
    )
    .expect("fixture trade proposal must be valid");
    colony
        .leader_ai_runtime
        .trade
        .propose(
            proposal,
            &TradeAuthorization {
                actor,
                acting_colony: global,
                owner_player_id: None,
                authorized_for_colony: true,
            },
        )
        .expect("fixture trade proposal must register");
    let persisted = serde_json::to_string(&colony.leader_ai_runtime.trade)
        .expect("fixture trade ledger must encode");
    let _: TradeLedger =
        serde_json::from_str(&persisted).expect("fixture trade ledger must round-trip");
}

fn seed_global_browser_acceptance(colony: &mut ColonyRuntime, world_seed: u32) {
    seed_spatial_browser_tasks(colony);
    seed_care_browser_scenarios(colony, world_seed);
    seed_shrine_and_favor_browser_scenarios(colony);
    seed_diplomacy_browser_scenario(colony);
    seed_trade_browser_scenario(colony);
    colony
        .leader_ai_runtime
        .validate()
        .expect("authored browser fixture must be persistence-valid");
}

impl Lai33SignedSystemJourneyHarness {
    #[must_use]
    pub fn fresh() -> Self {
        Self::new(LAI33_FRESH_SEED)
    }

    #[must_use]
    pub fn migrated() -> Self {
        Self::new(LAI33_MIGRATED_SEED)
    }

    #[must_use]
    pub fn new(seed: u32) -> Self {
        let secret = format!("lai33-secret-{seed:08x}");
        let session = signed_session(
            format!("session_v2_0_{:016x}_journey-{seed:08x}", u64::from(seed)),
            &secret,
        );
        let mut ownership = BTreeMap::new();
        ownership.insert(
            "colony-fresh-a".to_owned(),
            ColonyControlPolicy::PlayerOwned {
                owner_player_id: session.player_id.clone(),
            },
        );
        ownership.insert(
            "colony-migrated-a".to_owned(),
            ColonyControlPolicy::PlayerOwned {
                owner_player_id: session.player_id.clone(),
            },
        );
        ownership.insert(
            "colony-foreign-b".to_owned(),
            ColonyControlPolicy::PlayerOwned {
                owner_player_id: "player-foreign".to_owned(),
            },
        );
        Self {
            secret,
            session,
            ownership,
            receipts: IdempotencyReceiptStore::default(),
        }
    }

    #[must_use]
    pub fn signed_action(&self, colony_id: &str, id: &str) -> LeaderAiActionEnvelope {
        self.signed_payload_action(
            colony_id,
            id,
            LeaderAiActionPayload::NudgePlan {
                plan_id: BoundedEntityId::new("plan-visible-001").expect("fixture plan id"),
                nudge: BoundedBasisPointNudge::new(1_500).expect("fixture nudge"),
                reason_key: None,
            },
        )
    }

    #[must_use]
    pub fn signed_payload_action(
        &self,
        colony_id: &str,
        id: &str,
        payload: LeaderAiActionPayload,
    ) -> LeaderAiActionEnvelope {
        let mut expected_versions = ExpectedStateVersions {
            expected_planner_version: 0,
            expected_domain_version: 0,
            expected_resource_version: 0,
            expected_spatial_version: None,
            expected_reservation_version: None,
            expected_research_version: None,
            expected_scholar_version: None,
            expected_boost_version: None,
            expected_diplomacy_version: None,
            expected_trade_version: None,
            expected_prosthetic_version: None,
            expected_care_version: None,
            expected_officer_version: None,
            expected_standing_order_version: None,
        };
        match &payload {
            LeaderAiActionPayload::CreateStandingOrder { .. }
            | LeaderAiActionPayload::UpdateStandingOrder { .. }
            | LeaderAiActionPayload::DeleteStandingOrder { .. } => {
                expected_versions.expected_standing_order_version = Some(0)
            }
            LeaderAiActionPayload::AppointOfficer { .. }
            | LeaderAiActionPayload::UnappointOfficer { .. }
            | LeaderAiActionPayload::OfficerAuthorityOverride { .. } => {
                expected_versions.expected_officer_version = Some(0)
            }
            LeaderAiActionPayload::RequestTreatment { .. } => {
                expected_versions.expected_care_version = Some(0)
            }
            LeaderAiActionPayload::FitProsthetic { .. }
            | LeaderAiActionPayload::RepairProsthetic { .. } => {
                expected_versions.expected_prosthetic_version = Some(0);
                expected_versions.expected_spatial_version = Some(0);
                expected_versions.expected_reservation_version = Some(0);
            }
            LeaderAiActionPayload::PurchaseResearchWithFavor { .. } => {
                expected_versions.expected_research_version = Some(0)
            }
            LeaderAiActionPayload::PrepareScholarStudy { .. } => {
                expected_versions.expected_research_version = Some(0);
                expected_versions.expected_scholar_version = Some(0);
            }
            LeaderAiActionPayload::ActivateDivineBoost { .. } => {
                expected_versions.expected_research_version = Some(0);
                expected_versions.expected_boost_version = Some(0);
            }
            LeaderAiActionPayload::ChangeDiplomacy { .. }
            | LeaderAiActionPayload::ApproveAlliance { .. }
            | LeaderAiActionPayload::BlockColony { .. } => {
                expected_versions.expected_diplomacy_version = Some(0)
            }
            LeaderAiActionPayload::AcceptTradeContract { .. } => {
                expected_versions.expected_trade_version = Some(0);
                expected_versions.expected_diplomacy_version = Some(0);
                expected_versions.expected_reservation_version = Some(0);
            }
            LeaderAiActionPayload::RejectTradeContract { .. } => {
                expected_versions.expected_trade_version = Some(0)
            }
            LeaderAiActionPayload::PhysicalPlacement { .. }
            | LeaderAiActionPayload::SelectColony { .. }
            | LeaderAiActionPayload::FoundVillage { .. }
            | LeaderAiActionPayload::NudgePlan { .. }
            | LeaderAiActionPayload::DismissIntent { .. } => {}
        }
        LeaderAiActionEnvelope {
            protocol_version: ActionProtocolVersion::current(),
            idempotency_id: ActionIdempotencyId::new(id).expect("fixture action id"),
            colony_id: SelectedColonyId::new(colony_id).expect("fixture colony id"),
            player_id: AuthenticatedPlayerId::new(self.session.player_id.clone())
                .expect("fixture player id"),
            expected_versions,
            payload,
        }
    }

    pub fn authenticate(
        &self,
        envelope: LeaderAiActionEnvelope,
        now_ms: i64,
    ) -> Result<crate::leader_ai_action_routing::AuthorizedMutation, ServerActionConflict> {
        let encoded = serde_json::to_string(&envelope).expect("fixture action encoding");
        LeaderAiServerMutationPipeline::validate_foundation(
            &encoded,
            &self.session,
            &self.secret,
            now_ms,
            self,
        )
    }
}

impl SelectedColonyOwnershipSource for Lai33SignedSystemJourneyHarness {
    fn control_policy(&self, colony_id: &str) -> Option<ColonyControlPolicy> {
        self.ownership.get(colony_id).cloned()
    }
}

pub fn lai33_fixture_manifest() -> serde_json::Value {
    serde_json::json!({
        "schema": "lai33-signed-system-journey-v1",
        "freshSeed": LAI33_FRESH_SEED,
        "migratedSeed": LAI33_MIGRATED_SEED,
        "colonies": ["global", "colony-fresh-a", "colony-migrated-a", "colony-foreign-b"],
        "authoritativeRoutes": ["snapshot", "lai25_action", "sqlite_migration"],
        "browserScenarios": {
            "spatialTasks": ["hunt", "fetch_water", "workshop_work"],
            "workshopFootprintCells": 9,
            "care": ["treatment", "prosthetic_fitting"],
            "shrinePipeline": true,
            "favorEvents": 1,
            "researchFrontier": true,
            "diplomacyRelationships": 1,
            "tradeContracts": 1
        }
    })
}

/// Write the deterministic pre-start SQLite fixture through the normal schema
/// and world persistence APIs. Browser actions still enter through WebSocket
/// authentication after startup.
pub fn write_lai33_authoritative_fixture(
    database_path: &Path,
    manifest_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let connection = rusqlite::Connection::open(database_path)?;
    persistence::init_schema(&connection)?;
    let world = lai33_fixture_world();
    persistence::save_world(&connection, &world)?;
    let mut manifest = lai33_fixture_manifest();
    let checksum = sha2::Sha256::digest(std::fs::read(database_path)?);
    let checksum = checksum
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    manifest["fixture"] = serde_json::json!({
        "sqlitePath": database_path,
        "sqliteSha256Before": checksum.clone(),
        "sqliteSha256After": checksum
    });
    manifest["expected"] = serde_json::json!({
        "worldId": "world-lai33-sys",
        "colonyIds": ["global", "colony-fresh-a", "colony-migrated-a", "colony-foreign-b"],
        "ticks": [0, 15, 60, 300, 900, 7200],
        "versions": {"protocol": cat_protocol::PROTOCOL_VERSION, "persistence": 1}
    });
    std::fs::write(manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(())
}
