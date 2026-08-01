//! Canonical schema-v2 selected-colony projection.
//!
//! The server projects the single persisted `LeaderAiRuntimeState` aggregate
//! into `cat_protocol::lai64`; it does not reconstruct Shrine/Favor state,
//! mirror legacy cats/officers, or synthesize diplomacy from the retired
//! colony runtime.  Every visible task carries the exact resolved site
//! footprint.  The Hole is always its fixed 5×5 landmark with its central 3×3
//! work footprint, and regeneration remains absent until an officer report is
//! actually available.

use std::collections::BTreeMap;

use cat_protocol::{PROTOCOL_VERSION, lai64::*};
use cat_sim::{
    autonomous_trade::{TradeBlockReason, TradeRecoveryState, TradeStage as AuthorityTradeStage},
    beliefs::{BeliefKey, BeliefKind, ProjectedBeliefValue},
    black_hole::BlackHoleState,
    cat_capabilities::{InheritedAttribute, LaborAffinity},
    construction_catalog::{BlueprintRequest, resolve_blueprint},
    construction_miracle_runtime::{
        ApplyConstructionMiracle, apply_construction_miracle, derive_construction_miracle_witness,
    },
    construction_stages::{ConstructionProject, ConstructionStage, ConstructionTargetKind},
    content_manifest::{CapabilityRequirement, ContentManifest},
    diplomacy::DiplomacyColonyId,
    divine_action_offers::{DivineBoostOfferCatalog, EmergencyRescueWitnessSet},
    divine_boosts::{DivineBoostType, active_effect_factor},
    family_housing::{HousingKind, LifeStage, housing_capacity},
    food_divine_policy::{
        EmergencySupplyKind, FoodPermission as AuthorityFoodPermission,
        MIRACLE_INPUT_VALUE_MULTIPLIER, RESCUE_UNITS_PER_RESIDENT, VOID_INSIGHT_PER_MIRACLE,
    },
    leader_content_planner::{GoalLifecycle, KeepStockOrder, PersistentGoal},
    moneyless_barter::PersonalStance as AuthorityPersonalStance,
    officer_expertise::ExpertiseBonuses,
    officers::OfficerRole,
    physical_storage::ContainerKind,
    planner_core::PlannerId,
    progression_research::{PlayerPartitionKey, VoidInsight},
    quality_lots::{LotLocation, PhysicalLot, QualityBand},
    research_authority::{DuplicateAuthorization, GodResearchFundingReport},
    spatial_tasks::{ResourceSourceKind, SiteRef, TaskFootprint, TilePoint},
    storage_authority::{StorageContainer, StorageIdentity, StorageZone, VisibleStorageSlot},
    task_runtime::{RuntimeBlockReason, TaskCategory, TaskStage, VisibleTaskRuntime},
    world_tick::{ColonyRuntime, WorldState},
};
use serde::Serialize;

const GAME_MINUTE_MS: i64 = 60_000;

/// Build exactly one detailed, authenticated colony.  Public summaries are
/// already authorization-filtered by the server session/directory boundary;
/// this function only sorts and validates them before transmission.
pub fn project_selected_colony(
    world: &WorldState,
    selected_colony_id: &str,
    mut public_colonies: Vec<PublicColonySummaryV2>,
    authenticated_player_id: &str,
    now_ms: i64,
) -> Result<CanonicalSnapshotEnvelope, CanonicalWireError> {
    let colony = world
        .colonies
        .iter()
        .find(|candidate| candidate.id == selected_colony_id)
        .ok_or(CanonicalWireError::WrongPartition)?;
    public_colonies.sort_by(|left, right| left.colony_id.cmp(&right.colony_id));
    let selected_id = stable(selected_colony_id)?;
    let snapshot = CanonicalSnapshotEnvelope {
        protocol_version: PROTOCOL_VERSION,
        snapshot_schema_version: CANONICAL_SNAPSHOT_SCHEMA_VERSION,
        now_ms,
        selected_colony_id: selected_id.clone(),
        public_colonies,
        colonies: vec![project_colony(
            colony,
            selected_id,
            authenticated_player_id,
            now_ms,
        )?],
    };
    snapshot.validate()?;
    Ok(snapshot)
}

fn project_colony(
    colony: &ColonyRuntime,
    colony_id: StableId,
    authenticated_player_id: &str,
    now_ms: i64,
) -> Result<CanonicalColonySnapshot, CanonicalWireError> {
    let runtime = &colony.leader_ai_runtime;
    let officer_report_level = hole_officer_report_level(colony);
    Ok(CanonicalColonySnapshot {
        colony_id,
        state_version: selected_state_version(colony),
        versions: projected_versions(colony),
        plans: project_plans(colony)?,
        officer_requests: project_officer_requests(colony)?,
        standing_order_capabilities: project_standing_order_capabilities(colony)?,
        standing_orders: project_standing_orders(colony)?,
        tasks: project_tasks(colony)?,
        cats: project_cats(colony)?,
        job_assignments: project_job_assignments(colony)?,
        residences: project_residences(colony)?,
        governance: project_governance(colony)?,
        research: project_research(colony)?,
        construction: project_construction(colony)?,
        storage_zones: project_storage_zones(colony, now_ms)?,
        hole: project_hole(colony, officer_report_level, now_ms)?,
        divine: project_divine(colony, authenticated_player_id, now_ms)?,
        diplomacy: project_diplomacy(colony)?,
        content_manifest: Some(project_content_manifest()?),
        quality_lots: runtime
            .storage
            .ledger()
            .lots()
            .map(|lot| project_quality_lot(lot, now_ms))
            .collect::<Result<Vec<_>, _>>()?,
        exact_items: runtime
            .storage
            .ledger()
            .items()
            .map(project_exact_item)
            .collect::<Result<Vec<_>, _>>()?,
        // The storage report exposes food lot identity, quantity, quality,
        // provenance, age, and location, but no authoritative per-lot spoilage
        // state. `FoodStockSnapshotV2` requires that state, so do not derive it
        // from age or the catalog lifetime on the server projection boundary.
        food_stocks: Vec::new(),
        // `HuntingLairState` is still a standalone leaf and is not retained by
        // `LeaderAiRuntimeState`; consequently there is no report-safe Lair
        // iterator from which to project roster, public band, or respawn text.
        hunting_sites: Vec::new(),
        // Named Hunting drops remain transient `HuntOutputs::named_drops` and
        // have no persisted/report-safe authority collection in this runtime.
        rare_materials: Vec::new(),
        augmentations: project_augmentations(colony)?,
        // Installed fixtures live only in the standalone material-crafting
        // authority. The canonical runtime does not retain that authority or a
        // report-safe fixture-target iterator.
        fixtures: Vec::new(),
        // Cookhouse queues/batches are standalone leaves and are not retained
        // by the canonical runtime, so no batch stage or exact state art key is
        // available at this boundary.
        cookhouse_batches: Vec::new(),
        // Fishing placement/operation is likewise not retained by the
        // canonical runtime. In particular, no report-safe 3x3 Hut footprint,
        // dock pair, worker, rod, habitat report, or exact state art key exists.
        fishing_huts: Vec::new(),
        visual_states: project_visual_states(colony)?,
        // The old `colony.events` is not a canonical leader-AI event source.
        event_log: Vec::new(),
        diagnostics: project_diagnostics(colony, now_ms)?,
    })
}

fn project_content_manifest() -> Result<ContentManifestSnapshot, CanonicalWireError> {
    let manifest = ContentManifest::embedded();
    let mut entries = Vec::new();
    for value in &manifest.resources {
        let mut capabilities = required_capability_ids(&value.canonical_capability)?;
        if let Some(capability) = &value.processing_capability {
            capabilities.push(stable(capability.as_str())?);
        }
        capabilities.sort();
        capabilities.dedup();
        entries.push(manifest_entry(
            value.content_id.as_str(),
            "resource",
            &value.display_name,
            value.art_key.as_str(),
            capabilities,
        )?);
    }
    for value in &manifest.foods {
        entries.push(manifest_entry(
            value.content_id.as_str(),
            "food",
            &value.display_name,
            value.art_key.as_str(),
            required_capability_ids(&value.canonical_capability)?,
        )?);
    }
    for value in &manifest.item_definitions {
        entries.push(manifest_entry(
            value.content_id.as_str(),
            "item_definition",
            &value.display_name,
            value.art_key.as_str(),
            required_capability_ids(&value.canonical_capability)?,
        )?);
    }
    for value in &manifest.materials {
        entries.push(manifest_entry(
            value.content_id.as_str(),
            "material",
            &value.display_name,
            value.art_key.as_str(),
            required_capability_ids(&value.canonical_capability)?,
        )?);
    }
    for value in &manifest.creatures {
        entries.push(manifest_entry(
            value.content_id.as_str(),
            "creature",
            &value.display_name,
            value.portrait.as_str(),
            required_capability_ids(&value.canonical_capability)?,
        )?);
    }
    for value in &manifest.stations {
        entries.push(manifest_entry(
            value.content_id.as_str(),
            "station",
            &value.display_name,
            value.art_key.as_str(),
            required_capability_ids(&value.canonical_capability)?,
        )?);
    }
    for value in &manifest.recipes {
        let mut capabilities = required_capability_ids(&value.canonical_capability)?;
        capabilities.push(stable(value.bundle_capability.as_str())?);
        capabilities.sort();
        capabilities.dedup();
        entries.push(manifest_entry(
            value.content_id.as_str(),
            "recipe",
            &value.display_name,
            value.art_key.as_str(),
            capabilities,
        )?);
    }
    for value in &manifest.augmentations {
        entries.push(manifest_entry(
            value.content_id.as_str(),
            "augmentation",
            &value.display_name,
            value.art_key.as_str(),
            required_capability_ids(&value.canonical_capability)?,
        )?);
    }
    for value in &manifest.fixtures {
        entries.push(manifest_entry(
            value.content_id.as_str(),
            "fixture",
            &value.display_name,
            value.art_key.as_str(),
            required_capability_ids(&value.canonical_capability)?,
        )?);
    }
    entries.sort_by(|left, right| left.content_id.cmp(&right.content_id));
    Ok(ContentManifestSnapshot {
        manifest_version: manifest.version,
        checksum_id: stable(&format!(
            "manifest_{:016x}",
            stable_serialized_version(&manifest.to_canonical_json())
        ))?,
        entries,
    })
}

fn required_capability_ids(
    requirement: &CapabilityRequirement,
) -> Result<Vec<StableId>, CanonicalWireError> {
    requirement
        .required_id()
        .map(|capability| stable(capability.as_str()).map(|id| vec![id]))
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn manifest_entry(
    content_id: &str,
    content_kind_id: &str,
    display_name: &str,
    art_key: &str,
    capability_ids: Vec<StableId>,
) -> Result<ContentManifestEntrySnapshot, CanonicalWireError> {
    Ok(ContentManifestEntrySnapshot {
        content_id: stable(content_id)?,
        content_kind_id: stable(content_kind_id)?,
        display_name: report(display_name)?,
        art_key: stable(art_key)?,
        accessibility_label: report(display_name)?,
        capability_ids,
    })
}

fn projected_versions(colony: &ColonyRuntime) -> Vec<VersionExpectation> {
    let runtime = &colony.leader_ai_runtime;
    [
        VersionLane::Planner,
        VersionLane::Officers,
        VersionLane::Research,
        VersionLane::Construction,
        VersionLane::Storage,
        VersionLane::FoodPolicy,
        VersionLane::Hole,
        VersionLane::Divine,
        VersionLane::Governance,
        VersionLane::Diplomacy,
        VersionLane::Trade,
        VersionLane::Tasks,
        VersionLane::Reservations,
    ]
    .into_iter()
    .map(|lane| version(lane, canonical_lane_version(runtime, lane)))
    .collect()
}

fn version(lane: VersionLane, expected_version: u64) -> VersionExpectation {
    VersionExpectation {
        lane,
        expected_version,
    }
}

/// The single version-lane projection used both in snapshots and canonical
/// action admission. Keeping this in one place prevents clients from receiving
/// a version token that the server can never accept.
#[must_use]
pub fn canonical_lane_version(
    runtime: &cat_sim::leader_ai_runtime::LeaderAiRuntimeState,
    lane: VersionLane,
) -> u64 {
    match lane {
        VersionLane::Planner => stable_serialized_version(&(
            &runtime.planner,
            &runtime.beliefs,
            &runtime.player_directives,
        )),
        VersionLane::Officers => {
            stable_serialized_version(&(&runtime.governance, &runtime.officer_requests))
        }
        VersionLane::Research => stable_serialized_version(&runtime.research),
        VersionLane::Construction => stable_serialized_version(&runtime.construction_projects),
        VersionLane::Storage => runtime.storage.version(),
        VersionLane::FoodPolicy => runtime.divine_hole.edible_policy.version,
        VersionLane::Hole => stable_serialized_version(&runtime.hole),
        VersionLane::Divine => stable_serialized_version(&(
            &runtime.divine_hole,
            &runtime.boosts,
            runtime.resident_needs_report_version,
            runtime.resident_needs_summary,
        )),
        VersionLane::Governance => runtime.governance.version(),
        VersionLane::Diplomacy | VersionLane::Trade => runtime.trade.version(),
        VersionLane::Tasks => stable_serialized_version(&(
            &runtime.scheduling.visible_tasks,
            &runtime.scheduling.resolved_spatial_tasks,
        )),
        VersionLane::Reservations => stable_serialized_version(&(
            &runtime.scheduling.reservations,
            &runtime.scheduling.world_reservations,
        )),
    }
}

fn selected_state_version(colony: &ColonyRuntime) -> u64 {
    projected_versions(colony)
        .into_iter()
        .fold(0_u64, |state, item| {
            state.rotate_left(7) ^ item.expected_version.wrapping_mul(0x9E37_79B9)
        })
}

/// One deterministic fingerprint implementation feeds both the projection and
/// the canonical optimistic-concurrency lanes.
pub(crate) fn stable_serialized_version(value: &impl Serialize) -> u64 {
    use sha2::Digest as _;

    let encoded = serde_json::to_vec(value).unwrap_or_default();
    let digest = sha2::Sha256::digest(encoded);
    u64::from_be_bytes(
        digest[..8]
            .try_into()
            .expect("SHA-256 always contains eight prefix bytes"),
    )
}

fn project_plans(colony: &ColonyRuntime) -> Result<Vec<LeaderPlanSnapshot>, CanonicalWireError> {
    colony
        .leader_ai_runtime
        .planner
        .live_goals
        .values()
        .map(project_goal)
        .collect()
}

fn project_goal(goal: &PersistentGoal) -> Result<LeaderPlanSnapshot, CanonicalWireError> {
    Ok(LeaderPlanSnapshot {
        plan_id: stable(goal.id.as_str())?,
        topic_id: stable(goal.candidate_id.as_str())?,
        phase: report(goal_lifecycle_text(goal.lifecycle))?,
        priority_basis_points: u16::try_from(goal.score.clamp(0, 10_000)).unwrap_or_default(),
        confidence: confidence_from_basis_points(goal.confidence_basis_points),
        rationale: report(goal.rationale_key.as_str())?,
        dependencies: goal
            .dependencies
            .iter()
            .map(|dependency| {
                Ok(PlanDependencySnapshot {
                    plan_id: stable(dependency.as_str())?,
                    satisfied: false,
                })
            })
            .collect::<Result<Vec<_>, CanonicalWireError>>()?,
        responsible_officer_id: None,
    })
}

fn project_officer_requests(
    colony: &ColonyRuntime,
) -> Result<Vec<OfficerRequestSnapshotV2>, CanonicalWireError> {
    colony
        .leader_ai_runtime
        .officer_requests
        .iter()
        .map(|(_, request)| {
            Ok(OfficerRequestSnapshotV2 {
                request_id: stable(request.id.as_str())?,
                officer_id: stable(request.officer_id.as_str())?,
                request_kind: stable(officer_request_kind_id(request.kind))?,
                rationale: report(request.rationale_id.as_str())?,
                confidence: confidence_from_basis_points(request.confidence.get()),
                capability_id: Some(stable(request.target_id.as_str())?),
            })
        })
        .collect()
}

fn project_standing_order_capabilities(
    colony: &ColonyRuntime,
) -> Result<Vec<StandingOrderCapabilitySnapshot>, CanonicalWireError> {
    colony
        .leader_ai_runtime
        .planner
        .standing_orders
        .values()
        .map(|order| {
            Ok(StandingOrderCapabilitySnapshot {
                capability_id: stable(order.id.as_str())?,
                office_id: stable(officer_role_id(order.officer_role))?,
                order_kind_id: stable("keep_stock")?,
                enabled: true,
                reason: report("Officer authority is present in the canonical planner.")?,
            })
        })
        .collect()
}

fn project_standing_orders(
    colony: &ColonyRuntime,
) -> Result<Vec<StandingOrderSnapshotV2>, CanonicalWireError> {
    colony
        .leader_ai_runtime
        .planner
        .standing_orders
        .values()
        .map(project_standing_order)
        .collect()
}

fn project_standing_order(
    order: &KeepStockOrder,
) -> Result<StandingOrderSnapshotV2, CanonicalWireError> {
    Ok(StandingOrderSnapshotV2 {
        order_id: stable(order.id.as_str())?,
        capability_id: stable(order.id.as_str())?,
        instruction: report(&format!(
            "Keep {} between {} and {} units.",
            order.content_id.as_str(),
            order.minimum_units,
            order.target_units
        ))?,
        expires_at_ms: None,
    })
}

fn project_tasks(colony: &ColonyRuntime) -> Result<Vec<PhysicalTaskSnapshot>, CanonicalWireError> {
    colony
        .leader_ai_runtime
        .scheduling
        .visible_tasks
        .values()
        .filter_map(|task| project_task(colony, task).transpose())
        .collect()
}

fn project_task(
    colony: &ColonyRuntime,
    task: &VisibleTaskRuntime,
) -> Result<Option<PhysicalTaskSnapshot>, CanonicalWireError> {
    let Some(site) = task.spatial.objective.as_ref() else {
        // An unresolved task intentionally has no world marker.  Emitting a
        // guessed tile would violate the task placement contract.
        return Ok(None);
    };
    let Some(footprint) = site_footprint(site) else {
        return Ok(None);
    };
    let Some(site_kind_id) = projected_site_kind_id(site) else {
        // Retired Shrine identities never cross the canonical Hole boundary.
        return Ok(None);
    };
    let route = colony
        .leader_ai_runtime
        .scheduling
        .resolved_spatial_tasks
        .get(&task.id)
        .map(|resolved| {
            route_from_sites(
                &resolved.source_to_work_route,
                &resolved.work_to_delivery_route,
            )
        })
        .unwrap_or_default();
    let blockers = task
        .blocked_reason
        .map(|reason| {
            Ok(TaskBlockerSnapshot {
                blocker_id: stable(task.id.as_str())?,
                reason: report(runtime_blocker_text(reason))?,
                recoverable: !matches!(reason, RuntimeBlockReason::WorkerDied),
            })
        })
        .transpose()?
        .into_iter()
        .collect();
    let mut work_sites = task
        .spatial
        .work_positions
        .iter()
        .filter_map(|slot| {
            site_footprint(&slot.site).map(|footprint| {
                Ok(TaskSiteSnapshotV2 {
                    site_id: stable(slot.site.stable_id())?,
                    site_kind_id: stable(
                        &projected_site_kind_id(&slot.site)
                            .ok_or(CanonicalWireError::InvalidBounds("task_work_site_kind"))?,
                    )?,
                    slot_id: Some(stable(slot.stable_id.as_str())?),
                    footprint,
                })
            })
        })
        .collect::<Result<Vec<_>, CanonicalWireError>>()?;
    work_sites.sort_by(|left, right| {
        (&left.site_id, &left.slot_id).cmp(&(&right.site_id, &right.slot_id))
    });
    let delivery_site =
        task.spatial
            .delivery_endpoint
            .as_ref()
            .and_then(|site| {
                site_footprint(site).map(|footprint| {
                    Ok(TaskSiteSnapshotV2 {
                        site_id: stable(site.stable_id())?,
                        site_kind_id: stable(&projected_site_kind_id(site).ok_or(
                            CanonicalWireError::InvalidBounds("task_delivery_site_kind"),
                        )?)?,
                        slot_id: None,
                        footprint,
                    })
                })
            })
            .transpose()?;
    Ok(Some(PhysicalTaskSnapshot {
        task_id: stable(task.id.as_str())?,
        task_kind_id: stable(task_category_id(task.category))?,
        site_id: stable(site.stable_id())?,
        site_kind_id: stable(&site_kind_id)?,
        objective: report("A canonical task is pinned to this exact world site.")?,
        state: task_state(task.stage),
        footprint,
        work_sites,
        delivery_site,
        route,
        // TaskRuntime has no canonical quality/provenance projection yet.  Do
        // not invent a cargo identity from its pre-cutover resource label.
        cargo: Vec::new(),
        reservations: Vec::new(),
        worker_cat_ids: task
            .assigned_cat_ids
            .iter()
            .map(|id| stable(id))
            .collect::<Result<Vec<_>, _>>()?,
        refusals: Vec::new(),
        anatomy_requirements: Vec::new(),
        blockers,
    }))
}

fn project_cats(colony: &ColonyRuntime) -> Result<Vec<CatSnapshotV2>, CanonicalWireError> {
    let runtime = &colony.leader_ai_runtime;
    let family_report = runtime.families.report();
    let families_by_cat = family_report
        .cats
        .iter()
        .map(|cat| (cat.cat_id.as_str(), cat))
        .collect::<BTreeMap<_, _>>();
    let family_children = family_report
        .cats
        .iter()
        .flat_map(|cat| {
            cat.parent_cat_ids
                .iter()
                .cloned()
                .map(move |parent| (parent, cat.cat_id.clone()))
        })
        .fold(
            BTreeMap::<String, Vec<String>>::new(),
            |mut map, (parent, child)| {
                map.entry(parent).or_default().push(child);
                map
            },
        );
    let household_ids = family_report
        .households
        .iter()
        .flat_map(|household| {
            household
                .residents
                .iter()
                .map(move |cat_id| (cat_id.as_str(), household.household_id.as_str()))
        })
        .collect::<BTreeMap<_, _>>();
    runtime
        .cat_capabilities
        .report()
        .cats
        .iter()
        .map(|cat| {
            let family = families_by_cat.get(cat.cat_id.as_str()).copied();
            let family_snapshot = FamilySnapshotV2 {
                household_id: household_ids
                    .get(cat.cat_id.as_str())
                    .copied()
                    .map(stable)
                    .transpose()?,
                partnership_id: family
                    .and_then(|value| value.partnership_id.as_deref())
                    .map(stable)
                    .transpose()?,
                parent_ids: family
                    .map(|value| {
                        value
                            .parent_cat_ids
                            .iter()
                            .map(|id| stable(id))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?
                    .map(|mut ids| {
                        ids.sort();
                        ids
                    })
                    .unwrap_or_default(),
                child_ids: family_children
                    .get(&cat.cat_id)
                    .into_iter()
                    .flatten()
                    .map(|id| stable(id))
                    .collect::<Result<Vec<_>, _>>()?,
                residence_id: runtime
                    .families
                    .report_residence_for(&cat.cat_id)
                    .into_report()
                    .map(|residence| residence.building_id)
                    .map(stable)
                    .transpose()?,
                mentor_id: family
                    .and_then(|value| value.assigned_mentor_cat_id.as_deref())
                    .map(stable)
                    .transpose()?,
                tradition_id: family
                    .and_then(|value| value.tradition_ids.iter().next())
                    .map(|id| stable(id))
                    .transpose()?,
                surname: family
                    .and_then(|value| value.surname_key.as_deref())
                    .map(report)
                    .transpose()?,
                enterprise_id: family
                    .and_then(|value| value.enterprise_id.as_deref())
                    .map(stable)
                    .transpose()?,
            };
            Ok(CatSnapshotV2 {
                cat_id: stable(&cat.cat_id)?,
                display_name: report(&cat.cat_id)?,
                life_stage: family.map_or(LifeStageSnapshot::Adult, |value| {
                    life_stage(value.life_stage)
                }),
                job_id: runtime
                    .governance
                    .resident(&cat.cat_id)
                    .and_then(|resident| resident.job_id.as_deref())
                    .map(stable)
                    .transpose()?,
                attributes: project_attributes(cat.attributes)?,
                skills: cat
                    .skills
                    .iter()
                    .map(|skill| {
                        Ok(SkillSnapshotV2 {
                            skill_id: stable(&skill.skill_id)?,
                            xp: skill.mastery_xp_centi,
                            level: skill.level,
                            mastery: u32::try_from(skill.mastery_xp_centi).unwrap_or(u32::MAX),
                        })
                    })
                    .collect::<Result<Vec<_>, CanonicalWireError>>()?,
                affinities: cat
                    .labor
                    .affinities
                    .iter()
                    .map(|(labor_id, affinity)| {
                        Ok(AffinitySnapshot {
                            labor_id: stable(labor_id)?,
                            disposition: report(labor_affinity_text(*affinity))?,
                            refusing: *affinity == LaborAffinity::Refused,
                            refusal_reason: (*affinity == LaborAffinity::Refused)
                                .then(|| report("This cat refuses this labor."))
                                .transpose()?,
                        })
                    })
                    .collect::<Result<Vec<_>, CanonicalWireError>>()?,
                anatomy_eligibility: Vec::new(),
                family: family_snapshot,
                office_id: runtime
                    .governance
                    .resident(&cat.cat_id)
                    .and_then(|resident| resident.office_id.as_deref())
                    .map(stable)
                    .transpose()?,
                succession_eligible: family.is_some_and(|value| {
                    value.alive && matches!(value.life_stage, LifeStage::Adult | LifeStage::Elder)
                }),
            })
        })
        .collect()
}

fn project_job_assignments(
    colony: &ColonyRuntime,
) -> Result<Vec<JobAssignmentSnapshotV2>, CanonicalWireError> {
    let runtime = &colony.leader_ai_runtime;
    runtime
        .cat_capabilities
        .report()
        .cats
        .iter()
        .filter_map(|cat| {
            let resident = runtime.governance.resident(&cat.cat_id)?;
            let job_id = resident.job_id.as_deref()?;
            (resident.alive && resident.resident).then_some((cat, resident, job_id))
        })
        .map(|(cat, resident, job_id)| {
            Ok(JobAssignmentSnapshotV2 {
                assignment_id: stable(&format!("job:{}:{job_id}", cat.cat_id))?,
                cat_id: stable(&cat.cat_id)?,
                job_kind_id: stable(job_id)?,
                station_id: resident
                    .enterprise_id
                    .as_deref()
                    .or(resident.residence_id.as_deref())
                    .map(stable)
                    .transpose()?,
                active: true,
                report_reason: report("Current assignment from the canonical resident authority.")?,
            })
        })
        .collect()
}

fn project_residences(
    colony: &ColonyRuntime,
) -> Result<Vec<ResidenceSnapshotV2>, CanonicalWireError> {
    let runtime = &colony.leader_ai_runtime;
    let mut residents = BTreeMap::<&str, Vec<&str>>::new();
    let mut kinds = BTreeMap::<&str, HousingKind>::new();
    for residence in runtime.families.report_residences() {
        residents
            .entry(residence.building_id)
            .or_default()
            .push(residence.resident_cat_id);
        if let Some(kind) = residence.housing_kind {
            kinds.insert(residence.building_id, kind);
        }
    }
    residents
        .into_iter()
        .filter_map(|(building_id, mut resident_cat_ids)| {
            let project = runtime.construction_projects.get(building_id)?;
            let housing_kind = kinds.get(building_id).copied()?;
            resident_cat_ids.sort_unstable();
            Some((building_id, resident_cat_ids, project, housing_kind))
        })
        .map(|(building_id, resident_cat_ids, project, housing_kind)| {
            let capacity = u16::from(housing_capacity(housing_kind).permanent_beds);
            let pressure = if capacity == 0 {
                10_000
            } else {
                u16::try_from(
                    u64::try_from(resident_cat_ids.len())
                        .unwrap_or(u64::MAX)
                        .saturating_mul(10_000)
                        .saturating_div(u64::from(capacity))
                        .min(10_000),
                )
                .unwrap_or(10_000)
            };
            Ok(ResidenceSnapshotV2 {
                residence_id: stable(building_id)?,
                housing_kind_id: stable(housing_kind_id(housing_kind))?,
                footprint: footprint(&project.footprint),
                capacity,
                resident_cat_ids: resident_cat_ids
                    .into_iter()
                    .map(stable)
                    .collect::<Result<Vec<_>, _>>()?,
                housing_pressure_basis_points: pressure,
            })
        })
        .collect()
}

fn project_attributes(
    attributes: cat_sim::cat_capabilities::CapabilityAttributes,
) -> Result<Vec<AttributeSnapshot>, CanonicalWireError> {
    let mut projected = InheritedAttribute::ALL
        .into_iter()
        .map(|attribute| {
            let value = u16::from(attributes.get(attribute));
            Ok(AttributeSnapshot {
                attribute_id: stable(attribute_id(attribute))?,
                inherited_value: value,
                learned_value: 0,
                total_value: value,
            })
        })
        .collect::<Result<Vec<_>, CanonicalWireError>>()?;
    projected.sort_by(|left, right| left.attribute_id.cmp(&right.attribute_id));
    Ok(projected)
}

fn project_governance(colony: &ColonyRuntime) -> Result<GovernanceSnapshotV2, CanonicalWireError> {
    let runtime = &colony.leader_ai_runtime;
    let report_view = runtime.governance.report();
    let election = report_view.elections.first();
    let mut candidates = election
        .map(|election| {
            election
                .candidate_cat_ids
                .iter()
                .map(|cat_id| {
                    Ok(ElectionCandidateSnapshotV2 {
                        cat_id: stable(cat_id)?,
                        report_reason: report(
                            "Candidate is visible through the election authority.",
                        )?,
                        backing_blocks: u8::try_from(
                            election
                                .total_votes
                                .get(cat_id)
                                .copied()
                                .unwrap_or_default(),
                        )
                        .unwrap_or(u8::MAX),
                        eligible: true,
                    })
                })
                .collect::<Result<Vec<_>, CanonicalWireError>>()
        })
        .transpose()?
        .unwrap_or_default();
    candidates.sort_by(|left, right| left.cat_id.cmp(&right.cat_id));
    let institution = runtime.governance.officer_institution();
    let mut officers = OfficerRole::ALL
        .iter()
        .map(|role| {
            let appointment = institution.appointment(*role);
            let expertise = appointment
                .map(|item| institution.personal_level(&item.cat_id, *role))
                .map(u8::from)
                .unwrap_or(1);
            Ok(OfficerSnapshotV2 {
                office_id: stable(officer_role_id(*role))?,
                cat_id: appointment
                    .map(|item| stable(item.cat_id.as_str()))
                    .transpose()?,
                report_expertise_level: expertise.clamp(1, 5),
                appointment_candidate_ids: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>, CanonicalWireError>>()?;
    officers.sort_by(|left, right| left.office_id.cmp(&right.office_id));
    Ok(GovernanceSnapshotV2 {
        election_id: election
            .map(|value| stable(&value.election_id))
            .transpose()?,
        candidates,
        officers,
        succession_summary: Some(report(
            "Succession follows the canonical governance authority.",
        )?),
    })
}

fn project_research(colony: &ColonyRuntime) -> Result<ResearchSnapshotV2, CanonicalWireError> {
    let authority = &colony.leader_ai_runtime.research;
    let balances = authority.report_safe_projection();
    Ok(ResearchSnapshotV2 {
        notes_balance: balances.notes_balance.micro(),
        void_balance: balances.void_balance.micro(),
        god_queue: authority
            .report_god_queue()
            .enumerate()
            .map(|(position, entry)| {
                Ok(ResearchQueueEntrySnapshot {
                    study_id: stable(entry.study_id.as_str())?,
                    lane: ResearchLane::God,
                    position: u8::try_from(position).unwrap_or(u8::MAX),
                    funding_state: report(match entry.funding {
                        GodResearchFundingReport::Frozen(_) => "Funded with frozen terms",
                        GodResearchFundingReport::AwaitingFunding => "Awaiting funding",
                    })?,
                    // Funding state is not research progress. The report-safe
                    // queue does not expose completed labor, so do not invent
                    // a progress marker here.
                    progress_basis_points: 0,
                    duplicate_reason: None,
                    refund_reason: None,
                })
            })
            .collect::<Result<Vec<_>, CanonicalWireError>>()?,
        leader_decisions: authority
            .report_leader_decisions()
            .into_iter()
            .enumerate()
            .map(|(position, decision)| {
                Ok(ResearchQueueEntrySnapshot {
                    study_id: stable(decision.study_id.as_str())?,
                    lane: ResearchLane::Leader,
                    position: u8::try_from(position).unwrap_or(u8::MAX),
                    funding_state: report("Completed by the Leader without player currency.")?,
                    progress_basis_points: 10_000,
                    duplicate_reason: duplicate_research_reason(decision.duplicate_authorization)
                        .map(report)
                        .transpose()?,
                    refund_reason: None,
                })
            })
            .collect::<Result<Vec<_>, CanonicalWireError>>()?,
        preparations: authority
            .report_preparations()
            .entries
            .into_iter()
            .map(|entry| {
                let progress = if entry.required_labor_minutes == 0 {
                    0
                } else {
                    entry
                        .completed_labor_minutes
                        .saturating_mul(10_000)
                        .saturating_div(entry.required_labor_minutes)
                        .min(10_000) as u16
                };
                Ok(ResearchPreparationSnapshot {
                    preparation_id: stable(entry.study_id.as_str())?,
                    study_id: stable(entry.study_id.as_str())?,
                    physical_task_id: None,
                    progress_basis_points: progress,
                    player_discount_basis_points: 0,
                })
            })
            .collect::<Result<Vec<_>, CanonicalWireError>>()?,
    })
}

fn project_construction(
    colony: &ColonyRuntime,
) -> Result<Vec<ConstructionSnapshotV2>, CanonicalWireError> {
    colony
        .leader_ai_runtime
        .construction_projects
        .values()
        .map(project_construction_project)
        .collect()
}

fn project_construction_project(
    project: &ConstructionProject,
) -> Result<ConstructionSnapshotV2, CanonicalWireError> {
    let elapsed = project
        .original_total_work_ms
        .saturating_sub(project.stage_work_remaining_ms)
        .min(project.original_total_work_ms);
    let progress = elapsed
        .saturating_mul(10_000)
        .saturating_div(project.original_total_work_ms.max(1))
        .min(10_000) as u16;
    Ok(ConstructionSnapshotV2 {
        project_id: stable(&project.project_id)?,
        building_id: stable(&project.project_id)?,
        phase: construction_phase(project.stage),
        footprint: footprint(&project.footprint),
        phase_progress_basis_points: progress,
        // Cargo lines track quantities but do not yet retain a physical cargo
        // identity per line.  Keep phase shares visible without fabricating
        // lot/item provenance.
        stage_cargo: vec![
            construction_cargo(ConstructionPhase::Scaffold, 2_000),
            construction_cargo(ConstructionPhase::Structure, 6_000),
            construction_cargo(ConstructionPhase::FitOut, 2_000),
        ],
        art_state_id: stable(construction_art_state(project.stage))?,
    })
}

fn project_visual_states(
    colony: &ColonyRuntime,
) -> Result<Vec<VisualStateSnapshotV2>, CanonicalWireError> {
    let mut states = colony
        .leader_ai_runtime
        .construction_projects
        .values()
        .map(project_construction_visual_state)
        .collect::<Result<Vec<_>, CanonicalWireError>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    states.sort_by(|left, right| left.subject_id.cmp(&right.subject_id));
    Ok(states)
}

/// Project only an art key selected by the immutable construction catalog.
///
/// Site reservation has no physical phase art yet, and cancellation removes
/// the work rather than inventing a ruin sprite. Operational station art is
/// emitted only when the content manifest has an exact station identity for
/// the project's building type. The current closed `BuildingType` has no
/// Cookhouse or Fishing Hut variants, so those new state keys cannot be
/// attached until their canonical runtime identities exist.
fn project_construction_visual_state(
    project: &ConstructionProject,
) -> Result<Option<VisualStateSnapshotV2>, CanonicalWireError> {
    let Some(building_type) = project.building_type else {
        return Ok(None);
    };
    let Ok(target_level) = u8::try_from(project.target_level) else {
        return Ok(None);
    };
    let request = match project.target_kind {
        ConstructionTargetKind::Building => BlueprintRequest::NewBuilding(building_type),
        ConstructionTargetKind::BuildingUpgrade => BlueprintRequest::BuildingUpgrade {
            building_type,
            target_level,
        },
        ConstructionTargetKind::HoleUpgrade => return Ok(None),
    };
    let Ok(blueprint) = resolve_blueprint(request) else {
        // A legacy/delegated building can still have physical construction
        // state without an LAI.59 visual blueprint. Absence is safer than a
        // guessed phase image and must not hide the construction snapshot.
        return Ok(None);
    };
    let (art_key, label) = match project.stage {
        ConstructionStage::DeliverScaffold | ConstructionStage::BuildScaffold => (
            blueprint.presentation.scaffold_art_key,
            blueprint.presentation.inspector_label,
        ),
        ConstructionStage::DeliverStructure | ConstructionStage::BuildStructure => (
            blueprint.presentation.structure_art_key,
            blueprint.presentation.inspector_label,
        ),
        ConstructionStage::DeliverFitOut | ConstructionStage::BuildFitOut => (
            blueprint.presentation.fit_out_art_key,
            blueprint.presentation.inspector_label,
        ),
        ConstructionStage::Operational => {
            let Some(station) = ContentManifest::embedded()
                .stations
                .iter()
                .find(|station| station.id.as_str() == building_type.as_str())
            else {
                return Ok(None);
            };
            (station.art_key.as_str(), station.display_name.as_str())
        }
        ConstructionStage::SiteReserved | ConstructionStage::Cancelled => return Ok(None),
    };
    let accessibility = format!("{label} — {}", construction_art_state(project.stage));
    Ok(Some(VisualStateSnapshotV2 {
        subject_id: stable(&project.project_id)?,
        art_key: stable(art_key)?,
        state_id: stable(construction_art_state(project.stage))?,
        accessibility_label: report(&accessibility)?,
        footprint: footprint(&project.footprint),
    }))
}

fn construction_cargo(
    phase: ConstructionPhase,
    work_share_basis_points: u16,
) -> ConstructionCargoSnapshot {
    ConstructionCargoSnapshot {
        phase,
        work_share_basis_points,
        delivered: Vec::new(),
        in_transit: Vec::new(),
        consumed: Vec::new(),
    }
}

fn project_storage_zones(
    colony: &ColonyRuntime,
    now_ms: i64,
) -> Result<Vec<StorageZoneSnapshotV2>, CanonicalWireError> {
    let storage = &colony.leader_ai_runtime.storage;
    let links = storage
        .report_workshop_links()
        .map(|link| (link.zone_id.as_str(), link.workshop_id.as_str()))
        .collect::<BTreeMap<_, _>>();
    storage
        .report_zones()
        .map(|zone| {
            project_storage_zone(storage, zone, links.get(zone.id.as_str()).copied(), now_ms)
        })
        .collect()
}

fn project_storage_zone(
    storage: &cat_sim::storage_authority::StorageAuthority,
    zone: &StorageZone,
    linked_workshop_id: Option<&str>,
    now_ms: i64,
) -> Result<StorageZoneSnapshotV2, CanonicalWireError> {
    let mut lot_locations = BTreeMap::<String, (TilePoint, Option<String>)>::new();
    let mut tiles = zone
        .tiles
        .iter()
        .map(|(tile_point, tile_slots)| {
            let mut slots = Vec::with_capacity(4);
            for slot_index in 0_u8..4 {
                let visible = tile_slots.slots.get(&slot_index);
                let (lot_id, item_id, container_id, fullness_basis_points) = match visible {
                    Some(VisibleStorageSlot::Loose(identity)) => {
                        if let StorageIdentity::Lot(id) = identity {
                            lot_locations.insert(id.as_str().to_owned(), (*tile_point, None));
                        }
                        match identity {
                            StorageIdentity::Lot(id) => {
                                (Some(stable(id.as_str())?), None, None, 10_000)
                            }
                            StorageIdentity::Item(id) => {
                                (None, Some(stable(id.as_str())?), None, 10_000)
                            }
                        }
                    }
                    Some(VisibleStorageSlot::Container(container_id)) => {
                        let container = storage.container(container_id);
                        if let Some(container) = container {
                            for identity in &container.contents {
                                if let StorageIdentity::Lot(id) = identity {
                                    lot_locations.insert(
                                        id.as_str().to_owned(),
                                        (*tile_point, Some(container.id.clone())),
                                    );
                                }
                            }
                        }
                        (
                            None,
                            None,
                            Some(stable(container_id)?),
                            container.map_or(0, storage_container_fullness),
                        )
                    }
                    None => (None, None, None, 0),
                };
                slots.push(StorageSlotSnapshot {
                    slot_id: stable(&format!(
                        "{}:slot:{}:{}:{slot_index}",
                        zone.id, tile_point.x, tile_point.y
                    ))?,
                    lot_id,
                    item_id,
                    container_id,
                    fullness_basis_points,
                });
            }
            Ok(StorageTileSnapshot {
                tile: tile(*tile_point),
                slots,
            })
        })
        .collect::<Result<Vec<_>, CanonicalWireError>>()?;
    tiles.sort_by_key(|entry| (entry.tile.y, entry.tile.x));

    let containers = storage
        .report_containers()
        .filter(|container| container.zone_id == zone.id)
        .map(|container| project_storage_container(storage, container))
        .collect::<Result<Vec<_>, CanonicalWireError>>()?;
    let lots = lot_locations
        .into_iter()
        .filter_map(|(lot_id, location)| {
            let id = cat_sim::content_manifest::PhysicalLotId::new(lot_id).ok()?;
            storage
                .ledger()
                .lot(&id)
                .map(|lot| project_storage_lot(lot, zone, location, now_ms))
        })
        .collect::<Result<Vec<_>, CanonicalWireError>>()?;

    Ok(StorageZoneSnapshotV2 {
        zone_id: stable(&zone.id)?,
        linked_workshop_id: linked_workshop_id.map(stable).transpose()?,
        footprint: footprint(&zone.footprint),
        tiles,
        containers,
        lots,
    })
}

fn project_storage_container(
    storage: &cat_sim::storage_authority::StorageAuthority,
    container: &StorageContainer,
) -> Result<ContainerSnapshotV2, CanonicalWireError> {
    let mut content_ids = container
        .contents
        .iter()
        .filter_map(|identity| match identity {
            StorageIdentity::Lot(id) => storage
                .ledger()
                .lot(id)
                .map(|lot| lot.key.content_id.as_str()),
            StorageIdentity::Item(_) => None,
        })
        .collect::<Vec<_>>();
    content_ids.sort_unstable();
    content_ids.dedup();
    let contained_content_id = (content_ids.len() == 1
        && container
            .contents
            .iter()
            .all(|identity| matches!(identity, StorageIdentity::Lot(_))))
    .then(|| stable(content_ids[0]))
    .transpose()?;
    Ok(ContainerSnapshotV2 {
        container_id: stable(&container.id)?,
        container_kind_id: stable(container_kind_id(container.kind))?,
        capacity_slots: u8::try_from(container.kind.lot_capacity()).unwrap_or(u8::MAX),
        contained_content_id,
        fullness_basis_points: storage_container_fullness(container),
    })
}

fn project_storage_lot(
    lot: &PhysicalLot,
    zone: &StorageZone,
    (location_tile, container_id): (TilePoint, Option<String>),
    now_ms: i64,
) -> Result<PhysicalCargoSnapshot, CanonicalWireError> {
    Ok(PhysicalCargoSnapshot {
        cargo_id: stable(lot.id.as_str())?,
        content_id: stable(lot.key.content_id.as_str())?,
        quantity: u64::from(lot.quantity),
        quality_band: lot.key.quality.ordinal(),
        provenance_id: stable(&lot.provenance.origin)?,
        created_at_ms: tick_ms(lot.provenance.created_tick).min(now_ms.max(0)),
        reservation_id: lot.reservation.as_deref().map(stable).transpose()?,
        container_id: container_id.as_deref().map(stable).transpose()?,
        location_site_id: Some(stable(&zone.id)?),
        location_tile: Some(tile(location_tile)),
    })
}

fn storage_container_fullness(container: &StorageContainer) -> u16 {
    u16::try_from(
        u64::try_from(container.contents.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(10_000)
            .saturating_div(u64::try_from(container.kind.lot_capacity()).unwrap_or(1))
            .min(10_000),
    )
    .unwrap_or(10_000)
}

fn project_hole(
    colony: &ColonyRuntime,
    officer_report_level: u8,
    _now_ms: i64,
) -> Result<HoleSnapshotV2, CanonicalWireError> {
    let runtime = &colony.leader_ai_runtime;
    let BlackHoleState { hole_id, axes, .. } = &runtime.hole;
    let geometry = runtime.hole.footprint();
    let (regeneration, officer_reported_regeneration) =
        project_regeneration(colony, officer_report_level);
    Ok(HoleSnapshotV2 {
        hole_id: stable(hole_id)?,
        width: axes.width,
        depth: axes.depth,
        darkness: axes.darkness,
        footprint: footprint(&geometry.landmark),
        work_footprint: footprint(&geometry.work),
        food_permission_summary: report(
            "Food permissions are set by the Leader; Gods can only nudge conservation.",
        )?,
        food_permissions: runtime
            .divine_hole
            .edible_policy
            .entries
            .values()
            .map(|entry| {
                Ok(FoodPermissionSnapshot {
                    content_id: stable(&entry.edible_id)?,
                    permission: food_permission(entry.permission),
                    reason: report("Leader food policy report.")?,
                    confidence: ReportConfidence::OfficerVerified,
                })
            })
            .collect::<Result<Vec<_>, CanonicalWireError>>()?,
        officer_report_level,
        regeneration,
        officer_reported_regeneration,
        contribution_receipts: Vec::new(),
    })
}

/// Project only the Forester's already-report-safe renewable-source belief.
///
/// The Hole's authoritative ecology remains private: this adapter neither
/// reads it nor derives a substitute from tiles.  Level four merely permits a
/// player to receive an extant level-four-or-five officer belief.
fn project_regeneration(
    colony: &ColonyRuntime,
    officer_report_level: u8,
) -> (RegenerationReport, Option<RegenerationEstimateSnapshot>) {
    if officer_report_level < 4 {
        return (RegenerationReport::Unavailable, None);
    }

    let runtime = &colony.leader_ai_runtime;
    let now_tick = runtime.last_processed_tick.unwrap_or_default();
    let key = renewable_food_regeneration_belief_key(&colony.id);
    let Some(record) = runtime.beliefs.get(&key) else {
        return (RegenerationReport::Unavailable, None);
    };
    if record.invalidated || record.is_expired(now_tick) {
        return (RegenerationReport::Unavailable, None);
    }
    let Some(projection) = runtime.beliefs.project(&key, now_tick) else {
        return (RegenerationReport::Unavailable, None);
    };
    if !projection.report_level.regeneration_visible() {
        return (RegenerationReport::Unavailable, None);
    }
    let ProjectedBeliefValue::RegenerationRange(range) = projection.value else {
        return (RegenerationReport::Unavailable, None);
    };

    let (lower_units_per_day, upper_units_per_day) =
        renewable_hourly_milliunits_to_daily_range(range.lower_bound, range.upper_bound);
    (
        RegenerationReport::OfficerReportedEstimate,
        Some(RegenerationEstimateSnapshot {
            lower_units_per_day,
            upper_units_per_day,
            observed_at_ms: tick_ms(projection.observed_tick),
            confidence: confidence_from_basis_points(projection.confidence.get()),
        }),
    )
}

fn renewable_food_regeneration_belief_key(colony_id: &str) -> BeliefKey {
    BeliefKey::new(
        PlannerId::derive("belief_domain", [colony_id, "forestry"]),
        PlannerId::derive("belief_subject", [colony_id, "renewable_food_sources"]),
        BeliefKind::Regeneration,
    )
}

fn renewable_hourly_milliunits_to_daily_range(
    lower_milliunits_per_hour: i64,
    upper_milliunits_per_hour: i64,
) -> (u64, u64) {
    const MAX_UNITS_PER_DAY: u64 = 1_000_000_000;
    let lower = daily_units(lower_milliunits_per_hour, false).min(MAX_UNITS_PER_DAY - 1);
    let upper = daily_units(upper_milliunits_per_hour, true).min(MAX_UNITS_PER_DAY);
    (lower, upper.max(lower.saturating_add(1)))
}

fn daily_units(milliunits_per_hour: i64, round_up: bool) -> u64 {
    let daily_milliunits = u64::try_from(milliunits_per_hour)
        .unwrap_or_default()
        .saturating_mul(24);
    let units = daily_milliunits / 1_000;
    if round_up && daily_milliunits % 1_000 != 0 {
        units.saturating_add(1)
    } else {
        units
    }
}

#[cfg(test)]
mod regeneration_projection_tests {
    use super::*;
    use cat_sim::{
        beliefs::{
            BeliefValue, Confidence, EstimateRange, EvidenceId, Observation, OfficerReport,
            ReportId, ReportLevel,
        },
        world_tick::found_colony,
    };

    #[test]
    fn level_four_without_an_officer_belief_remains_unavailable() {
        let colony = found_colony(7, "colony-alpha", 0, 11);

        assert_eq!(
            project_regeneration(&colony, 4),
            (RegenerationReport::Unavailable, None)
        );
    }

    #[test]
    fn level_four_projects_the_reported_regeneration_range() {
        let mut colony = found_colony(7, "colony-alpha", 0, 11);
        let key = renewable_food_regeneration_belief_key(&colony.id);
        let reporter = PlannerId::derive("reporter", [colony.id.as_str(), "forester"]);
        let observed_tick = 42;
        let evidence_id = EvidenceId::derive(&colony.id, &key, observed_tick, &reporter, 0);
        let observation = Observation::new(
            evidence_id.clone(),
            key,
            BeliefValue::Estimate(EstimateRange::around(1_000, 2_500).unwrap()),
            Confidence::new(8_000).unwrap(),
            observed_tick,
            60,
            reporter.clone(),
            ReportLevel::Four,
        )
        .unwrap();
        colony
            .leader_ai_runtime
            .beliefs
            .apply_report(OfficerReport {
                report_id: ReportId::derive(&evidence_id, &reporter),
                observation,
                authorized: true,
            })
            .unwrap();
        colony.leader_ai_runtime.last_processed_tick = Some(observed_tick);

        let (marker, estimate) = project_regeneration(&colony, 4);
        assert_eq!(marker, RegenerationReport::OfficerReportedEstimate);
        let estimate = estimate.expect("level-four Forester report is visible");
        assert_eq!(estimate.lower_units_per_day, 18);
        assert_eq!(estimate.upper_units_per_day, 30);
        assert_eq!(estimate.observed_at_ms, 42 * GAME_MINUTE_MS);
        assert_eq!(estimate.confidence, ReportConfidence::High);
    }
}

fn hole_officer_report_level(colony: &ColonyRuntime) -> u8 {
    let institution = colony.leader_ai_runtime.governance.officer_institution();
    institution
        .appointment(OfficerRole::Forester)
        .map(|appointment| {
            institution.effective_level(
                &appointment.cat_id,
                OfficerRole::Forester,
                ExpertiseBonuses::default(),
            )
        })
        .map(u8::from)
        .unwrap_or(1)
        .clamp(1, 5)
}

fn project_divine(
    colony: &ColonyRuntime,
    authenticated_player_id: &str,
    now_ms: i64,
) -> Result<DivineSnapshotV2, CanonicalWireError> {
    let runtime = &colony.leader_ai_runtime;
    let partition = PlayerPartitionKey {
        colony_id: runtime.colony_partition.clone(),
        player_id: PlannerId::derive("player", [authenticated_player_id]),
    };
    let inspiration_expires_at_ms = runtime
        .divine_hole
        .inspiration
        .by_player
        .values()
        .filter_map(|window| i64::try_from(window.active_until_real_ms).ok())
        .filter(|expiry| *expiry >= now_ms)
        .max();
    let mut boost_offers = DivineBoostOfferCatalog::capture(
        partition.clone(),
        &runtime.divine_hole,
        &runtime.boosts,
        &runtime.research,
    )
    .map(|catalog| {
        catalog
            .offers
            .into_iter()
            .map(|offer| {
                Ok(DivineBoostOfferSnapshotV2 {
                    offer_id: stable(offer.id.as_str())?,
                    boost_type_id: stable(divine_boost_type_id(offer.boost_type))?,
                    duration_game_hours: offer.duration_hours,
                    exact_cost_micro_void: offer.exact_cost.micro(),
                    effect_basis_points: active_effect_factor(offer.boost_type),
                })
            })
            .collect::<Result<Vec<_>, CanonicalWireError>>()
    })
    .transpose()?
    .unwrap_or_default();
    boost_offers.sort_by(|left, right| left.offer_id.cmp(&right.offer_id));

    let construction_miracle_offers =
        capture_construction_miracle_offers(runtime, authenticated_player_id)?
            .into_iter()
            .map(|offer| offer.snapshot)
            .collect();

    let (mut rescue_offers, rescue_reason) = match (
        runtime.resident_needs_report_version,
        runtime.resident_needs_summary,
    ) {
        (Some(report_version), Some(summary)) => {
            match EmergencyRescueWitnessSet::capture(
                partition,
                report_version,
                summary,
                &runtime.divine_hole,
                &runtime.research.void,
            ) {
                Ok(witnesses) if !witnesses.witnesses.is_empty() => {
                    let quantity = summary
                        .living_resident_count
                        .checked_mul(RESCUE_UNITS_PER_RESIDENT)
                        .ok_or(CanonicalWireError::InvalidBounds(
                            "emergency_rescue_quantity",
                        ))?;
                    let offers = witnesses
                        .witnesses
                        .into_iter()
                        .map(|witness| {
                            Ok(EmergencyRescueOfferSnapshotV2 {
                                witness_id: stable(witness.id.as_str())?,
                                supply: emergency_supply_snapshot(witness.supply),
                                quantity,
                                exact_cost_micro_void: VoidInsight::from_whole(
                                    VOID_INSIGHT_PER_MIRACLE,
                                )
                                .map_err(|_| CanonicalWireError::MalformedPayload)?
                                .micro(),
                            })
                        })
                        .collect::<Result<Vec<_>, CanonicalWireError>>()?;
                    (offers, None)
                }
                Ok(_) => (
                    Vec::new(),
                    Some(report(
                        "No report-safe lethal hunger or thirst is present.",
                    )?),
                ),
                Err(_) => (
                    Vec::new(),
                    Some(report(
                        "Emergency rescue evidence is stale; wait for the next village report.",
                    )?),
                ),
            }
        }
        _ => (
            Vec::new(),
            Some(report(
                "Emergency rescue requires a canonical resident-needs report.",
            )?),
        ),
    };
    rescue_offers.sort_by(|left, right| left.witness_id.cmp(&right.witness_id));

    Ok(DivineSnapshotV2 {
        inspiration_expires_at_ms,
        active_boost_ids: {
            let mut ids = runtime
                .boosts
                .active_boosts()
                .values()
                .map(|boost| stable(boost.purchase_id.as_str()))
                .collect::<Result<Vec<_>, _>>()?;
            ids.sort();
            ids
        },
        boost_offers,
        construction_miracle_offers,
        rescue_available: !rescue_offers.is_empty(),
        rescue_offers,
        rescue_reason,
    })
}

#[derive(Debug, Clone)]
pub(crate) struct CapturedConstructionMiracleOffer {
    pub offer_id: String,
    pub project_id: String,
    snapshot: ConstructionMiracleOfferSnapshotV2,
}

/// Capture only construction miracles that the canonical runtime can apply
/// against the current bill, manifest, typed storage, and shared Void ledger.
/// The opaque identifier binds the authenticated player, exact project
/// witness, and all five optimistic-concurrency lanes used by the action.
pub(crate) fn capture_construction_miracle_offers(
    runtime: &cat_sim::leader_ai_runtime::LeaderAiRuntimeState,
    authenticated_player_id: &str,
) -> Result<Vec<CapturedConstructionMiracleOffer>, CanonicalWireError> {
    let player_id = PlannerId::derive("player", [authenticated_player_id]);
    let exact_cost = VoidInsight::from_whole(VOID_INSIGHT_PER_MIRACLE)
        .map_err(|_| CanonicalWireError::MalformedPayload)?;
    if runtime.research.void.balance < exact_cost {
        return Ok(Vec::new());
    }
    let lane_versions = [
        VersionLane::Research,
        VersionLane::Construction,
        VersionLane::Storage,
        VersionLane::Divine,
        VersionLane::Reservations,
    ]
    .map(|lane| canonical_lane_version(runtime, lane));
    let mut offers = Vec::new();
    for project_id in runtime
        .construction_projects
        .keys()
        .take(MAX_CANONICAL_ITEMS)
    {
        let Ok(witness) = derive_construction_miracle_witness(runtime, project_id) else {
            continue;
        };
        if witness.exact_missing_bound_inputs.is_empty() {
            continue;
        }

        // A preview on an isolated aggregate is the eligibility check: this
        // exercises canonical manifest valuation, exact package composition,
        // typed cargo materialization, and one-Void debit without mutating the
        // authoritative runtime or exposing package contents to the client.
        let mut preview = runtime.clone();
        if apply_construction_miracle(
            &mut preview,
            ApplyConstructionMiracle {
                command_id: format!("canonical:miracle-preview:{project_id}"),
                project_id: project_id.clone(),
                player_id: player_id.as_str().to_owned(),
                expected_authority_version: runtime.divine_hole.version,
                expected_void_version: runtime.research.void.version,
                now_real_ms: 0,
            },
        )
        .is_err()
        {
            continue;
        }
        let project = runtime
            .construction_projects
            .get(project_id)
            .ok_or(CanonicalWireError::MalformedPayload)?;
        let offer_id =
            construction_miracle_offer_id(player_id.as_str(), project_id, &witness, lane_versions)?;
        let snapshot = ConstructionMiracleOfferSnapshotV2 {
            offer_id: stable(&offer_id)?,
            project_id: stable(project_id)?,
            // ConstructionProject does not carry a separate building-content
            // ID yet; the canonical project ID remains the report-safe label
            // already used by the construction projection.
            building_id: stable(project_id)?,
            phase: construction_phase(project.stage),
            footprint: footprint(&project.footprint),
            exact_cost_micro_void: exact_cost.micro(),
            labor_reduction_basis_points: 1_000,
            input_value_multiplier_basis_points: u16::try_from(
                MIRACLE_INPUT_VALUE_MULTIPLIER.saturating_mul(10_000),
            )
            .map_err(|_| CanonicalWireError::MalformedPayload)?,
        };
        offers.push(CapturedConstructionMiracleOffer {
            offer_id,
            project_id: project_id.clone(),
            snapshot,
        });
    }
    offers.sort_by(|left, right| left.offer_id.cmp(&right.offer_id));
    offers.truncate(MAX_CANONICAL_ITEMS);
    Ok(offers)
}

fn construction_miracle_offer_id(
    player_id: &str,
    project_id: &str,
    witness: &cat_sim::construction_miracle_runtime::ConstructionMiracleWitness,
    lane_versions: [u64; 5],
) -> Result<String, CanonicalWireError> {
    use sha2::Digest as _;

    let encoded = serde_json::to_vec(&(
        "canonical-construction-miracle-offer-v1",
        player_id,
        project_id,
        witness,
        lane_versions,
    ))
    .map_err(|_| CanonicalWireError::MalformedPayload)?;
    let digest = sha2::Sha256::digest(encoded);
    let mut suffix = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;

        write!(&mut suffix, "{byte:02x}").map_err(|_| CanonicalWireError::MalformedPayload)?;
    }
    Ok(format!("construction-miracle-{suffix}"))
}

const fn divine_boost_type_id(boost_type: DivineBoostType) -> &'static str {
    match boost_type {
        DivineBoostType::BountifulLabor => "bountiful_labor",
        DivineBoostType::FleetPaws => "fleet_paws",
        DivineBoostType::InspiredWork => "inspired_work",
        DivineBoostType::RestorativeGrace => "restorative_grace",
    }
}

const fn emergency_supply_snapshot(supply: EmergencySupplyKind) -> EmergencySupply {
    match supply {
        EmergencySupplyKind::DivineRation => EmergencySupply::DivineRation,
        EmergencySupplyKind::DivineWater => EmergencySupply::DivineWater,
    }
}

fn project_diplomacy(colony: &ColonyRuntime) -> Result<DiplomacySnapshotV2, CanonicalWireError> {
    let authority = &colony.leader_ai_runtime.trade;
    let viewer = DiplomacyColonyId::derive(&colony.id);
    let contracts = authority.report_contracts_for(&viewer).entries;
    let mut stances = authority
        .report_personal_stances_for(&viewer)
        .entries
        .into_iter()
        .map(|stance| {
            let consented = contracts.iter().any(|contract| {
                contract.counterpart_colony_id == stance.to
                    && contract.viewer_consented
                    && contract.counterpart_consented
            });
            Ok(PersonalStanceSnapshot {
                other_colony_id: stable(stance.to.external_id())?,
                stance: personal_stance(stance.stance),
                consented,
            })
        })
        .collect::<Result<Vec<_>, CanonicalWireError>>()?;
    stances.sort_by(|left, right| left.other_colony_id.cmp(&right.other_colony_id));

    let mut projected_contracts = contracts
        .into_iter()
        .map(|contract| {
            Ok(TradeContractSnapshotV2 {
                contract_id: stable(contract.contract_id.as_str())?,
                partner_colony_id: stable(contract.counterpart_colony_id.external_id())?,
                stage: trade_stage(contract.stage),
                // The report iterator intentionally withholds foreign route and
                // quantity topology. An empty route/escrow is an explicit
                // unavailable report, never a client-computed substitute.
                route: Route {
                    ordered_tiles: Vec::new(),
                },
                escrow: Vec::new(),
                report_reason: Some(report(trade_report_reason(
                    contract.blocked_reason,
                    contract.recovery,
                ))?),
            })
        })
        .collect::<Result<Vec<_>, CanonicalWireError>>()?;
    projected_contracts.sort_by(|left, right| left.contract_id.cmp(&right.contract_id));
    Ok(DiplomacySnapshotV2 {
        stances,
        contracts: projected_contracts,
    })
}

fn project_quality_lot(
    lot: &PhysicalLot,
    now_ms: i64,
) -> Result<QualityLotSnapshotV2, CanonicalWireError> {
    Ok(QualityLotSnapshotV2 {
        lot_id: stable(lot.id.as_str())?,
        content_id: stable(lot.key.content_id.as_str())?,
        quantity: u64::from(lot.quantity),
        quality: quality(lot.key.quality),
        provenance_id: stable(&lot.provenance.origin)?,
        age_ms: now_ms
            .max(0)
            .saturating_sub(tick_ms(lot.provenance.created_tick)) as u64,
        location_site_id: stable(lot_location_id(&lot.location))?,
        reservation_id: lot.reservation.as_deref().map(stable).transpose()?,
    })
}

fn project_exact_item(
    item: &cat_sim::quality_lots::ItemInstance,
) -> Result<ExactItemSnapshotV2, CanonicalWireError> {
    Ok(ExactItemSnapshotV2 {
        item_id: stable(item.id.as_str())?,
        definition_id: stable(item.definition_id.as_str())?,
        material_id: stable(item.material_id.as_str())?,
        quality: quality(item.quality),
        durability_basis_points: u16::try_from(item.durability.min(10_000)).unwrap_or(10_000),
        provenance_id: stable(item.id.as_str())?,
        location_site_id: stable(lot_location_id(&item.location))?,
        reservation_id: item.reservation.as_deref().map(stable).transpose()?,
        augmentation_ids: item
            .augmentation
            .as_ref()
            .map(|augmentation| stable(augmentation.item.id.as_str()))
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?,
    })
}

/// Installed augmentations are embedded in the report-safe exact-item ledger.
///
/// The consumed augmentation item retains its own physical instance,
/// definition, material, location, and durability inside `ItemAugmentation`;
/// projecting that payload therefore needs no material-crafting executor state.
fn project_augmentations(
    colony: &ColonyRuntime,
) -> Result<Vec<AugmentationSnapshotV2>, CanonicalWireError> {
    let manifest = ContentManifest::embedded();
    let mut augmentations = colony
        .leader_ai_runtime
        .storage
        .ledger()
        .items()
        .filter_map(|target| {
            target
                .augmentation
                .as_ref()
                .map(|augmentation| (target, augmentation))
        })
        .map(|(target, augmentation)| {
            let definition = manifest
                .augmentations
                .iter()
                .find(|definition| {
                    definition.id.as_str() == augmentation.item.definition_id.as_str()
                })
                .ok_or(CanonicalWireError::InvalidBounds("augmentation_definition"))?;
            Ok(AugmentationSnapshotV2 {
                augmentation_instance_id: stable(augmentation.item.id.as_str())?,
                augmentation_id: stable(augmentation.item.definition_id.as_str())?,
                target_item_id: stable(target.id.as_str())?,
                material_instance_id: stable(augmentation.item.id.as_str())?,
                installed: true,
                effect_summary: report(&definition.description)?,
            })
        })
        .collect::<Result<Vec<_>, CanonicalWireError>>()?;
    augmentations.sort_by(|left, right| {
        left.augmentation_instance_id
            .cmp(&right.augmentation_instance_id)
    });
    Ok(augmentations)
}

fn project_diagnostics(
    colony: &ColonyRuntime,
    now_ms: i64,
) -> Result<Vec<DiagnosticSnapshot>, CanonicalWireError> {
    colony
        .leader_ai_runtime
        .planner
        .diagnostics
        .iter()
        .enumerate()
        .map(|(index, diagnostic)| {
            Ok(DiagnosticSnapshot {
                diagnostic_id: stable(&format!("planner_diagnostic_{index:04}"))?,
                domain: stable("planner")?,
                message: report(diagnostic.rationale_key.as_str())?,
                occurred_at_ms: tick_ms(diagnostic.tick).min(now_ms.max(0)),
            })
        })
        .collect()
}

fn site_footprint(site: &SiteRef) -> Option<Footprint> {
    match site {
        SiteRef::Tile { tile, .. } => Some(Footprint {
            ordered_tiles: vec![tile(*tile)],
        }),
        SiteRef::OrderedTiles { tiles, .. } => Some(Footprint {
            ordered_tiles: tiles.as_slice().iter().copied().map(tile).collect(),
        }),
        SiteRef::OrderedRoute { route, .. } => Some(Footprint {
            ordered_tiles: route.iter().copied().map(tile).collect(),
        }),
        _ => site.footprint().map(footprint),
    }
}

fn projected_site_kind_id(site: &SiteRef) -> Option<String> {
    let kind = match site {
        SiteRef::Tile { .. } => "tile".to_owned(),
        SiteRef::Rect { .. } => "rect".to_owned(),
        SiteRef::OrderedTiles { .. } => "ordered_tiles".to_owned(),
        SiteRef::Building { building_type, .. } => {
            format!("building:{}", building_type.as_str())
        }
        SiteRef::Stockpile { .. } => "stockpile".to_owned(),
        SiteRef::ResourceSource { resource_kind, .. } => {
            format!(
                "resource_source:{}",
                resource_source_kind_id(*resource_kind)
            )
        }
        SiteRef::OrderedRoute { .. } => "ordered_route".to_owned(),
        // Shrine is an explicitly retired identity. A canonical Hole task
        // needs a dedicated Hole site reference before it may be projected.
        SiteRef::Shrine { .. } => return None,
        SiteRef::VillageTradeEndpoint { .. } => "village_trade_endpoint".to_owned(),
    };
    Some(kind)
}

fn resource_source_kind_id(kind: ResourceSourceKind) -> &'static str {
    match kind {
        ResourceSourceKind::Hunting => "hunting",
        ResourceSourceKind::Water => "water",
        ResourceSourceKind::FishHabitat => "fish_habitat",
        ResourceSourceKind::Quarry => "quarry",
        ResourceSourceKind::Tree => "tree",
        ResourceSourceKind::Stump => "stump",
        ResourceSourceKind::Fibre => "fibre",
        ResourceSourceKind::Food => "food",
        ResourceSourceKind::Herbs => "herbs",
    }
}

fn route_from_sites(first: &SiteRef, second: &SiteRef) -> Route {
    let mut tiles = site_route_tiles(first);
    tiles.extend(site_route_tiles(second));
    tiles.dedup();
    Route {
        ordered_tiles: tiles,
    }
}

fn site_route_tiles(site: &SiteRef) -> Vec<Tile> {
    match site {
        SiteRef::OrderedRoute { route, .. } => route.iter().copied().map(tile).collect(),
        _ => Vec::new(),
    }
}

fn footprint(value: &TaskFootprint) -> Footprint {
    Footprint {
        ordered_tiles: value.tiles.as_slice().iter().copied().map(tile).collect(),
    }
}

fn tile(value: TilePoint) -> Tile {
    Tile {
        x: value.x,
        y: value.y,
    }
}

fn confidence_from_basis_points(value: u16) -> ReportConfidence {
    match value {
        0..=2_499 => ReportConfidence::Low,
        2_500..=6_999 => ReportConfidence::Moderate,
        7_000..=9_499 => ReportConfidence::High,
        _ => ReportConfidence::OfficerVerified,
    }
}

fn food_permission(value: AuthorityFoodPermission) -> FoodPermission {
    match value {
        AuthorityFoodPermission::Allowed => FoodPermission::Allowed,
        AuthorityFoodPermission::Reserve => FoodPermission::Reserve,
        AuthorityFoodPermission::Forbidden => FoodPermission::Forbidden,
    }
}

fn duplicate_research_reason(value: DuplicateAuthorization) -> Option<&'static str> {
    match value {
        DuplicateAuthorization::None => None,
        DuplicateAuthorization::CriticalVillage => {
            Some("Leader duplicated the God lane for a reported critical village need.")
        }
        DuplicateAuthorization::KeyedOopsie => {
            Some("Leader duplicated the God lane because of a persisted keyed mistake.")
        }
    }
}

fn housing_kind_id(value: HousingKind) -> &'static str {
    match value {
        HousingKind::Den => "den",
        HousingKind::FamilyHome => "family_home",
        HousingKind::ElderLodge => "elder_lodge",
        HousingKind::Nursery => "nursery",
    }
}

fn container_kind_id(value: ContainerKind) -> &'static str {
    match value {
        ContainerKind::Basket => "basket",
        ContainerKind::Barrel => "barrel",
        ContainerKind::Crate => "crate",
        ContainerKind::Chest => "chest",
        ContainerKind::Rack => "rack",
    }
}

fn personal_stance(value: AuthorityPersonalStance) -> PersonalStance {
    match value {
        AuthorityPersonalStance::Alliance => PersonalStance::Alliance,
        AuthorityPersonalStance::Neutral => PersonalStance::Neutral,
        AuthorityPersonalStance::Enemy => PersonalStance::Enemy,
    }
}

fn trade_stage(value: AuthorityTradeStage) -> TradeStage {
    match value {
        AuthorityTradeStage::Proposed => TradeStage::Proposed,
        AuthorityTradeStage::Escrowed => TradeStage::Escrowed,
        AuthorityTradeStage::InTransit => TradeStage::EnRoute,
        AuthorityTradeStage::Returning => TradeStage::Recovering,
        AuthorityTradeStage::Blocked | AuthorityTradeStage::Stranded => TradeStage::Failed,
        AuthorityTradeStage::Complete => TradeStage::Delivered,
        AuthorityTradeStage::Cancelled => TradeStage::Cancelled,
    }
}

fn trade_report_reason(
    blocked_reason: Option<TradeBlockReason>,
    recovery: TradeRecoveryState,
) -> &'static str {
    if let Some(reason) = blocked_reason {
        return match reason {
            TradeBlockReason::SourceUnavailable => "The reported trade source is unavailable.",
            TradeBlockReason::InsufficientEscrow => "The reported escrow could not be completed.",
            TradeBlockReason::RouteBlocked => "The reported trade route is blocked.",
            TradeBlockReason::DestinationFull => "The reported trade destination is full.",
            TradeBlockReason::DestinationRemoved => "The reported trade destination was removed.",
            TradeBlockReason::WorkerRefused => "The assigned trade worker refused.",
            TradeBlockReason::WorkerDied => "The assigned trade worker died.",
        };
    }
    match recovery {
        TradeRecoveryState::None => {
            "Contract is visible; foreign route, storage, valuation, and quantity details are withheld."
        }
        TradeRecoveryState::Returning => "Trade cargo is returning through the recovery path.",
        TradeRecoveryState::Returned => "Trade cargo was returned to its owning village.",
        TradeRecoveryState::Salvaged => {
            "Trade cargo was salvaged through the physical recovery path."
        }
        TradeRecoveryState::Stranded => "Trade cargo is reported stranded.",
    }
}

fn quality(value: QualityBand) -> QualityBandSnapshot {
    match value {
        QualityBand::Crude => QualityBandSnapshot::Crude,
        QualityBand::Common => QualityBandSnapshot::Common,
        QualityBand::Fine => QualityBandSnapshot::Fine,
        QualityBand::Superior => QualityBandSnapshot::Superior,
        QualityBand::Masterwork => QualityBandSnapshot::Masterwork,
    }
}

fn lot_location_id(location: &LotLocation) -> &str {
    match location {
        LotLocation::Source(id)
        | LotLocation::Stockpile(id)
        | LotLocation::StationInput(id)
        | LotLocation::StationOutput(id)
        | LotLocation::Cargo(id)
        | LotLocation::Cache(id)
        | LotLocation::Hole(id) => id,
    }
}

fn task_state(stage: TaskStage) -> TaskState {
    match stage {
        TaskStage::Resolve => TaskState::Proposed,
        TaskStage::Reserve => TaskState::Reserved,
        TaskStage::TravelToSource | TaskStage::TravelToWork | TaskStage::TravelToEndpoint => {
            TaskState::Assigned
        }
        TaskStage::Pickup | TaskStage::Work | TaskStage::Deposit => TaskState::InProgress,
        TaskStage::Complete => TaskState::Complete,
        TaskStage::Blocked => TaskState::Blocked,
        TaskStage::Cancelled => TaskState::Recovering,
    }
}

fn construction_phase(stage: ConstructionStage) -> ConstructionPhase {
    match stage {
        ConstructionStage::SiteReserved => ConstructionPhase::Reserve,
        ConstructionStage::DeliverScaffold | ConstructionStage::BuildScaffold => {
            ConstructionPhase::Scaffold
        }
        ConstructionStage::DeliverStructure | ConstructionStage::BuildStructure => {
            ConstructionPhase::Structure
        }
        ConstructionStage::DeliverFitOut | ConstructionStage::BuildFitOut => {
            ConstructionPhase::FitOut
        }
        ConstructionStage::Operational => ConstructionPhase::Operational,
        ConstructionStage::Cancelled => ConstructionPhase::Cancelled,
    }
}

fn construction_art_state(stage: ConstructionStage) -> &'static str {
    match stage {
        ConstructionStage::SiteReserved => "construction_reserve",
        ConstructionStage::DeliverScaffold | ConstructionStage::BuildScaffold => {
            "construction_scaffold"
        }
        ConstructionStage::DeliverStructure | ConstructionStage::BuildStructure => {
            "construction_structure"
        }
        ConstructionStage::DeliverFitOut | ConstructionStage::BuildFitOut => "construction_fit_out",
        ConstructionStage::Operational => "construction_operational",
        ConstructionStage::Cancelled => "construction_cancelled",
    }
}

fn goal_lifecycle_text(value: GoalLifecycle) -> &'static str {
    match value {
        GoalLifecycle::Proposed => "Proposed",
        GoalLifecycle::Expanded => "Expanded",
        GoalLifecycle::AwaitingSite => "Awaiting site",
        GoalLifecycle::Reserving => "Reserving",
        GoalLifecycle::Assigning => "Assigning",
        GoalLifecycle::Executing => "Executing",
        GoalLifecycle::Observing => "Observing",
        GoalLifecycle::Recovering => "Recovering",
        GoalLifecycle::Succeeded => "Succeeded",
        GoalLifecycle::Blocked => "Blocked",
        GoalLifecycle::Cancelled => "Cancelled",
    }
}

fn labor_affinity_text(value: LaborAffinity) -> &'static str {
    match value {
        LaborAffinity::Loved => "Loved",
        LaborAffinity::Preferred => "Preferred",
        LaborAffinity::Neutral => "Neutral",
        LaborAffinity::Disliked => "Disliked",
        LaborAffinity::Refused => "Refused",
    }
}

fn life_stage(value: LifeStage) -> LifeStageSnapshot {
    match value {
        LifeStage::Kitten => LifeStageSnapshot::Kitten,
        LifeStage::Young => LifeStageSnapshot::Adolescent,
        LifeStage::Adult => LifeStageSnapshot::Adult,
        LifeStage::Elder => LifeStageSnapshot::Elder,
    }
}

fn runtime_blocker_text(value: RuntimeBlockReason) -> &'static str {
    match value {
        RuntimeBlockReason::Spatial(_) => "The pinned world site is currently unavailable.",
        RuntimeBlockReason::ReservationLost => "The task reservation was lost.",
        RuntimeBlockReason::RouteClosedBeforePickup => "The route closed before pickup.",
        RuntimeBlockReason::RouteClosedWithCargo => "The route closed while cargo was carried.",
        RuntimeBlockReason::SourceRemoved => "The task source was removed.",
        RuntimeBlockReason::EndpointRemoved => "The task endpoint was removed.",
        RuntimeBlockReason::WorkerRefused => "The assigned worker refused this labor.",
        RuntimeBlockReason::WorkerDied => "The assigned worker died.",
        RuntimeBlockReason::WorkerIncapacitated => "The assigned worker is incapacitated.",
        RuntimeBlockReason::CargoRecoveryRequired => "Cargo recovery is required.",
        RuntimeBlockReason::InvalidLegacySite => "The task site is invalid and requires recovery.",
    }
}

fn task_category_id(value: TaskCategory) -> &'static str {
    match value {
        TaskCategory::Hunt => "hunt",
        TaskCategory::FetchWater => "fetch_water",
        TaskCategory::Fish => "fish",
        TaskCategory::Quarry => "quarry",
        TaskCategory::Logging => "logging",
        TaskCategory::Replant => "replant",
        TaskCategory::BuildingConstruction => "building_construction",
        TaskCategory::RoadConstruction => "road_construction",
        TaskCategory::StationWork => "station_work",
        TaskCategory::WorkshopWork => "workshop_work",
        TaskCategory::FarmWork => "farm_work",
        TaskCategory::HaulDelivery => "haul_delivery",
        TaskCategory::StockpileTransfer => "stockpile_transfer",
        TaskCategory::FibreForage => "fibre_forage",
        TaskCategory::Scout => "scout",
        TaskCategory::Expansion => "expansion",
        TaskCategory::OfferingRitual => "offering_ritual",
        TaskCategory::Training => "training",
        TaskCategory::Accounting => "accounting",
        TaskCategory::Eat => "eat",
        TaskCategory::Drink => "drink",
        TaskCategory::Sleep => "sleep",
    }
}

fn officer_role_id(value: OfficerRole) -> &'static str {
    match value {
        OfficerRole::Steward => "steward",
        OfficerRole::Accountant => "accountant",
        OfficerRole::Forester => "forester",
        OfficerRole::Farmer => "farmer",
        OfficerRole::Captain => "captain",
        OfficerRole::Loremaster => "loremaster",
        OfficerRole::ClothLeader => "cloth_leader",
    }
}

fn officer_request_kind_id(value: cat_sim::officer_requests::RequestKind) -> &'static str {
    use cat_sim::officer_requests::RequestKind;
    match value {
        RequestKind::Survival => "survival",
        RequestKind::ActiveDefense => "active_defense",
        RequestKind::Operational => "operational",
        RequestKind::Research => "research",
        RequestKind::Building => "building",
        RequestKind::Diplomacy => "diplomacy",
        RequestKind::Trade => "trade",
    }
}

fn attribute_id(value: InheritedAttribute) -> &'static str {
    match value {
        InheritedAttribute::Attack => "attack",
        InheritedAttribute::Defense => "defense",
        InheritedAttribute::Hunting => "hunting",
        InheritedAttribute::Medicine => "medicine",
        InheritedAttribute::Cleaning => "cleaning",
        InheritedAttribute::Building => "building",
        InheritedAttribute::Leadership => "leadership",
        InheritedAttribute::Vision => "vision",
        InheritedAttribute::Charisma => "charisma",
        InheritedAttribute::Intelligence => "intelligence",
    }
}

fn tick_ms(tick: u64) -> i64 {
    i64::try_from(tick)
        .unwrap_or(i64::MAX / GAME_MINUTE_MS)
        .saturating_mul(GAME_MINUTE_MS)
}

fn stable(value: &str) -> Result<StableId, CanonicalWireError> {
    StableId::new(value)
}

fn report(value: &str) -> Result<ReportText, CanonicalWireError> {
    ReportText::new(value)
}
