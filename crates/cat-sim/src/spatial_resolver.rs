//! Authoritative task-site resolution specified by
//! `docs/leader-ai-overhaul/spatial-task-contract.md`.

use serde::{Deserialize, Serialize};

use crate::{
    food_divine_policy::EmergencySupplyKind,
    spatial_tasks::{
        SiteLifecycleStage, SiteRef, SiteVisibility, SpatialBlockReason, SpatialObjective,
        TaskFootprint, TilePoint, WorkSlot, canonical_building_footprint,
    },
    types::BuildingType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "building_type", rename_all = "snake_case")]
pub enum SpatialTaskCategory {
    Hunt,
    FetchWater,
    Fish,
    Quarry,
    Logging,
    AppleHarvest,
    HoleWork,
    CookhouseWork,
    FishingHutWork,
    Construction(BuildingType),
    RoadConstruction,
    StationWork(BuildingType),
    WorkshopWork,
    FarmWork,
    OfferingRitual,
    /// One exact purpose-bound rescue unit moving from the Hole apron to one
    /// reported living resident. The supply discriminator prevents ration and
    /// water tasks from sharing or silently substituting cargo.
    EmergencySupply(EmergencySupplyKind),
}

/// One authoritative objective/work/route combination supplied by world
/// indexes. Resolution selects from these facts and never fabricates a tile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialResolutionCandidate {
    pub objective: SiteRef,
    pub work_slot: WorkSlot,
    pub source_to_work_route: SiteRef,
    pub work_to_delivery_route: SiteRef,
    pub objective_exists: bool,
    pub work_position_available: bool,
    pub source_available_units: u32,
    pub source_capacity: u32,
    pub source_to_work_route_capacity: u32,
    pub work_to_delivery_route_capacity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialResolutionRequest {
    pub category: SpatialTaskCategory,
    /// `None` permits deterministic initial selection. Once resolved, callers
    /// persist and supply the exact stable ID so removal never retargets.
    pub pinned_objective_id: Option<String>,
    pub pinned_delivery_endpoint: SiteRef,
    pub delivery_endpoint_exists: bool,
    pub requested_source_units: u32,
    pub requested_delivery_units: u32,
    pub delivery_capacity: u32,
    pub candidates: Vec<SpatialResolutionCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedSpatialTask {
    pub category: SpatialTaskCategory,
    pub spatial: SpatialObjective,
    pub source_to_work_route: SiteRef,
    pub work_to_delivery_route: SiteRef,
    pub source_units: u32,
    pub source_capacity: u32,
    pub delivery_units: u32,
    pub delivery_capacity: u32,
    pub source_to_work_route_capacity: u32,
    pub work_to_delivery_route_capacity: u32,
}

impl ResolvedSpatialTask {
    pub fn validate(&self) -> Result<(), SpatialBlockReason> {
        self.spatial
            .validate()
            .map_err(|_| SpatialBlockReason::SourceUnavailable)?;
        let objective = self
            .spatial
            .objective
            .as_ref()
            .ok_or(SpatialBlockReason::SourceUnavailable)?;
        let work_slot = self
            .spatial
            .work_positions
            .first()
            .ok_or(SpatialBlockReason::WorkPositionUnavailable)?;
        let endpoint = self
            .spatial
            .delivery_endpoint
            .as_ref()
            .ok_or(SpatialBlockReason::DeliveryEndpointUnavailable)?;
        if self.spatial.work_positions.len() != 1
            || self.spatial.blocked_reason.is_some()
            || !category_matches(self.category, objective)
            || objective.stable_id() == work_slot.site.stable_id()
            || self.source_units == 0
            || self.source_capacity == 0
            || self.source_units > self.source_capacity
            || self.delivery_units == 0
            || self.delivery_capacity == 0
            || self.delivery_units > self.delivery_capacity
            || self.source_to_work_route_capacity == 0
            || self.work_to_delivery_route_capacity == 0
        {
            return Err(SpatialBlockReason::SourceUnavailable);
        }
        validate_resolved_objective_metadata(self.category, objective)?;
        if work_slot.site.metadata().visibility == SiteVisibility::Hidden
            || !matches!(
                work_slot.site.metadata().lifecycle,
                SiteLifecycleStage::Active | SiteLifecycleStage::Planned
            )
            || work_slot.site.metadata().blocked_reason.is_some()
        {
            return Err(SpatialBlockReason::WorkPositionUnavailable);
        }
        if endpoint.metadata().visibility == SiteVisibility::Hidden
            || endpoint.metadata().lifecycle != SiteLifecycleStage::Active
            || endpoint.metadata().blocked_reason.is_some()
        {
            return Err(SpatialBlockReason::DeliveryEndpointUnavailable);
        }
        validate_route(&self.source_to_work_route)?;
        validate_route(&self.work_to_delivery_route)?;
        Ok(())
    }

    #[must_use]
    pub fn objective(&self) -> &SiteRef {
        self.spatial
            .objective
            .as_ref()
            .expect("validated resolved task has an objective")
    }

    #[must_use]
    pub fn work_slot(&self) -> &WorkSlot {
        self.spatial
            .work_positions
            .first()
            .expect("validated resolved task has one work slot")
    }

    #[must_use]
    pub fn delivery_endpoint(&self) -> &SiteRef {
        self.spatial
            .delivery_endpoint
            .as_ref()
            .expect("validated resolved task has a delivery endpoint")
    }
}

/// LAI.46's strict hot-root geometry check. The lower-level persisted spatial
/// types intentionally remain able to decode older contracts, but no newly
/// resolved physical task may enter reservations unless this complete role
/// geometry passes. In particular, a one-tile center/generic marker can never
/// stand in for a multi-cell objective or work area.
pub fn validate_truthful_task_geometry(
    resolved: &ResolvedSpatialTask,
) -> Result<(), SpatialBlockReason> {
    resolved.validate()?;
    for stable_id in [
        resolved.objective().stable_id(),
        resolved.work_slot().stable_id.as_str(),
        resolved.work_slot().site.stable_id(),
        resolved.delivery_endpoint().stable_id(),
    ] {
        let normalized = stable_id.to_ascii_lowercase();
        if ["generic", "fallback", "center", "reported_work"]
            .iter()
            .any(|forbidden| normalized.contains(forbidden))
        {
            return Err(SpatialBlockReason::InvalidLegacySite);
        }
    }

    match resolved.category {
        SpatialTaskCategory::Hunt
        | SpatialTaskCategory::FetchWater
        | SpatialTaskCategory::Fish
        | SpatialTaskCategory::Quarry => {
            validate_single_source_with_bank(resolved)?;
        }
        SpatialTaskCategory::WorkshopWork => {
            validate_full_rect_work(resolved, 3, 3)?;
            let SiteRef::Building {
                building_type: BuildingType::Workshop,
                ..
            } = resolved.objective()
            else {
                return Err(SpatialBlockReason::SourceUnavailable);
            };
        }
        SpatialTaskCategory::StationWork(_) | SpatialTaskCategory::FarmWork => {
            let objective = resolved
                .objective()
                .footprint()
                .ok_or(SpatialBlockReason::SourceUnavailable)?;
            validate_full_rect_work(resolved, objective.width, objective.height)?;
        }
        SpatialTaskCategory::Construction(_) => {
            let objective = resolved
                .objective()
                .footprint()
                .ok_or(SpatialBlockReason::SourceUnavailable)?;
            validate_full_rect_work(resolved, objective.width, objective.height)?;
        }
        SpatialTaskCategory::AppleHarvest => validate_full_rect_work(resolved, 3, 3)?,
        SpatialTaskCategory::HoleWork => {
            validate_exact_rect(resolved.objective(), 5, 5)?;
            validate_exact_rect(&resolved.work_slot().site, 3, 3)?;
            let objective = resolved.objective().footprint().ok_or(
                SpatialBlockReason::SourceUnavailable,
            )?;
            let work = resolved.work_slot().site.footprint().ok_or(
                SpatialBlockReason::WorkPositionUnavailable,
            )?;
            if work.anchor.x != objective.anchor.x.saturating_add(1)
                || work.anchor.y != objective.anchor.y.saturating_add(1)
                || !site_is_single_tile_on_footprint_edge(
                    resolved.delivery_endpoint(),
                    objective,
                    work,
                )
            {
                return Err(SpatialBlockReason::DeliveryEndpointUnavailable);
            }
        }
        SpatialTaskCategory::CookhouseWork => validate_full_rect_work(resolved, 3, 3)?,
        SpatialTaskCategory::FishingHutWork => validate_fishing_hut_geometry(resolved)?,
        SpatialTaskCategory::Logging => {
            let objective = resolved
                .objective()
                .footprint()
                .ok_or(SpatialBlockReason::SourceUnavailable)?;
            if objective.width == 3 && objective.height == 3 {
                validate_full_rect_work(resolved, 3, 3)?;
            }
        }
        SpatialTaskCategory::RoadConstruction => {
            if !matches!(resolved.objective(), SiteRef::OrderedRoute { .. }) {
                return Err(SpatialBlockReason::SourceUnavailable);
            }
        }
        SpatialTaskCategory::OfferingRitual
        | SpatialTaskCategory::EmergencySupply(_) => {}
    }
    validate_route_roles(resolved)
}

fn validate_single_source_with_bank(
    resolved: &ResolvedSpatialTask,
) -> Result<(), SpatialBlockReason> {
    validate_exact_rect(resolved.objective(), 1, 1)?;
    let source = resolved
        .objective()
        .footprint()
        .and_then(|footprint| footprint.tiles.as_slice().first())
        .copied()
        .ok_or(SpatialBlockReason::SourceUnavailable)?;
    let work = site_single_tile(&resolved.work_slot().site)
        .ok_or(SpatialBlockReason::WorkPositionUnavailable)?;
    if i64::from(source.x).abs_diff(i64::from(work.x))
        + i64::from(source.y).abs_diff(i64::from(work.y))
        != 1
    {
        return Err(SpatialBlockReason::WorkPositionUnavailable);
    }
    Ok(())
}

fn validate_full_rect_work(
    resolved: &ResolvedSpatialTask,
    width: i32,
    height: i32,
) -> Result<(), SpatialBlockReason> {
    validate_exact_rect(resolved.objective(), width, height)?;
    validate_exact_rect(&resolved.work_slot().site, width, height)?;
    if resolved.objective().footprint() != resolved.work_slot().site.footprint() {
        return Err(SpatialBlockReason::WorkPositionUnavailable);
    }
    Ok(())
}

fn validate_exact_rect(
    site: &SiteRef,
    width: i32,
    height: i32,
) -> Result<(), SpatialBlockReason> {
    let footprint = site
        .footprint()
        .ok_or(SpatialBlockReason::SourceUnavailable)?;
    if footprint.width != width
        || footprint.height != height
        || footprint.tiles.len()
            != usize::try_from(width.saturating_mul(height)).unwrap_or_default()
        || footprint.tiles != footprint.rect().ordered_tiles()
    {
        return Err(SpatialBlockReason::SourceUnavailable);
    }
    Ok(())
}

fn validate_fishing_hut_geometry(
    resolved: &ResolvedSpatialTask,
) -> Result<(), SpatialBlockReason> {
    let SiteRef::OrderedTiles { tiles, .. } = resolved.objective() else {
        return Err(SpatialBlockReason::SourceUnavailable);
    };
    if tiles.len() != 10 {
        return Err(SpatialBlockReason::SourceUnavailable);
    }
    let work = site_single_tile(&resolved.work_slot().site)
        .ok_or(SpatialBlockReason::WorkPositionUnavailable)?;
    let water = site_single_tile(resolved.delivery_endpoint())
        .ok_or(SpatialBlockReason::DeliveryEndpointUnavailable)?;
    if !tiles.as_slice().contains(&work)
        || !tiles.as_slice().contains(&water)
        || i64::from(work.x).abs_diff(i64::from(water.x))
            + i64::from(work.y).abs_diff(i64::from(water.y))
            != 1
    {
        return Err(SpatialBlockReason::WorkPositionUnavailable);
    }
    let land = tiles
        .as_slice()
        .iter()
        .copied()
        .filter(|tile| *tile != water)
        .collect::<Vec<_>>();
    let footprint = TaskFootprint::from_tiles(land)
        .ok_or(SpatialBlockReason::SourceUnavailable)?;
    if footprint.width != 3
        || footprint.height != 3
        || footprint.tiles != footprint.rect().ordered_tiles()
        || !footprint.tiles.as_slice().contains(&work)
    {
        return Err(SpatialBlockReason::SourceUnavailable);
    }
    Ok(())
}

fn site_is_single_tile_on_footprint_edge(
    site: &SiteRef,
    outer: &TaskFootprint,
    inner: &TaskFootprint,
) -> bool {
    let Some(tile) = site_single_tile(site) else {
        return false;
    };
    outer.tiles.as_slice().contains(&tile) && !inner.tiles.as_slice().contains(&tile)
}

fn validate_route_roles(resolved: &ResolvedSpatialTask) -> Result<(), SpatialBlockReason> {
    let SiteRef::OrderedRoute {
        route: source_route,
        ..
    } = &resolved.source_to_work_route
    else {
        return Err(SpatialBlockReason::RouteUnavailable);
    };
    let SiteRef::OrderedRoute {
        route: delivery_route,
        ..
    } = &resolved.work_to_delivery_route
    else {
        return Err(SpatialBlockReason::RouteUnavailable);
    };
    let work_site = &resolved.work_slot().site;
    if !route_endpoint_touches_site(source_route.first(), resolved.objective())
        || !route_endpoint_touches_site(source_route.last(), work_site)
        || !route_endpoint_touches_site(delivery_route.first(), work_site)
        || !route_endpoint_touches_site(delivery_route.last(), resolved.delivery_endpoint())
    {
        return Err(SpatialBlockReason::RouteUnavailable);
    }
    Ok(())
}

fn route_endpoint_touches_site(endpoint: Option<&TilePoint>, site: &SiteRef) -> bool {
    endpoint.is_some_and(|endpoint| {
        site_single_tile(site).is_some_and(|tile| tile == *endpoint)
            || site
                .footprint()
                .is_some_and(|footprint| footprint.tiles.as_slice().contains(endpoint))
            || matches!(
                site,
                SiteRef::OrderedTiles { tiles, .. } if tiles.as_slice().contains(endpoint)
            )
    })
}

fn validate_resolved_objective_metadata(
    category: SpatialTaskCategory,
    objective: &SiteRef,
) -> Result<(), SpatialBlockReason> {
    let metadata = objective.metadata();
    if metadata.visibility == SiteVisibility::Hidden {
        return Err(SpatialBlockReason::UnrevealedObjective);
    }
    if let Some(reason) = metadata.blocked_reason {
        return Err(reason);
    }
    match metadata.lifecycle {
        SiteLifecycleStage::Active => Ok(()),
        SiteLifecycleStage::Planned
            if matches!(
                category,
                SpatialTaskCategory::Construction(_) | SpatialTaskCategory::RoadConstruction
            ) =>
        {
            Ok(())
        }
        SiteLifecycleStage::Depleted => Err(SpatialBlockReason::SourceDepleted),
        SiteLifecycleStage::Destroyed | SiteLifecycleStage::Removed => {
            Err(SpatialBlockReason::ObjectiveDestroyed)
        }
        SiteLifecycleStage::Planned => Err(SpatialBlockReason::SourceUnavailable),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpatialResolutionOutcome {
    Resolved(Box<ResolvedSpatialTask>),
    Blocked(Box<SpatialObjective>),
}

impl SpatialResolutionOutcome {
    #[must_use]
    pub fn blocked_reason(&self) -> Option<SpatialBlockReason> {
        match self {
            Self::Resolved(_) => None,
            Self::Blocked(spatial) => spatial.blocked_reason,
        }
    }
}

/// Resolve only authoritative supplied facts. There is intentionally no RNG,
/// radial point, straight-line route, or nearest-endpoint code path.
#[must_use]
pub fn resolve_spatial_task(mut request: SpatialResolutionRequest) -> SpatialResolutionOutcome {
    request.candidates.sort_by(|first, second| {
        first
            .objective
            .stable_id()
            .cmp(second.objective.stable_id())
            .then_with(|| first.work_slot.stable_id.cmp(&second.work_slot.stable_id))
    });

    let mut candidates = std::mem::take(&mut request.candidates)
        .into_iter()
        .filter(|candidate| {
            request
                .pinned_objective_id
                .as_ref()
                .is_none_or(|pinned| candidate.objective.stable_id() == pinned)
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return blocked(SpatialBlockReason::SourceUnavailable);
    }

    let mut first_failure = None;
    for candidate in candidates.drain(..) {
        match resolve_candidate(&request, candidate) {
            Ok(resolved) => return SpatialResolutionOutcome::Resolved(Box::new(resolved)),
            Err(reason) => first_failure.get_or_insert(reason),
        };
    }
    blocked(first_failure.unwrap_or(SpatialBlockReason::SourceUnavailable))
}

fn resolve_candidate(
    request: &SpatialResolutionRequest,
    candidate: SpatialResolutionCandidate,
) -> Result<ResolvedSpatialTask, SpatialBlockReason> {
    validate_objective(request.category, &candidate)?;
    validate_work_position(request.category, &candidate)?;
    validate_route(&candidate.source_to_work_route)?;
    validate_endpoint(request)?;
    validate_route(&candidate.work_to_delivery_route)?;

    if request.requested_source_units == 0
        || candidate.source_capacity == 0
        || request.requested_source_units > candidate.source_available_units
        || request.requested_source_units > candidate.source_capacity
    {
        return Err(SpatialBlockReason::CapacityUnavailable);
    }
    if request.requested_delivery_units == 0
        || request.delivery_capacity == 0
        || request.requested_delivery_units > request.delivery_capacity
    {
        return Err(SpatialBlockReason::CapacityUnavailable);
    }
    if candidate.source_to_work_route_capacity == 0
        || candidate.work_to_delivery_route_capacity == 0
    {
        return Err(SpatialBlockReason::RouteUnavailable);
    }

    let resolved = ResolvedSpatialTask {
        category: request.category,
        spatial: SpatialObjective::resolved(
            candidate.objective,
            vec![candidate.work_slot],
            Some(request.pinned_delivery_endpoint.clone()),
        ),
        source_to_work_route: candidate.source_to_work_route,
        work_to_delivery_route: candidate.work_to_delivery_route,
        source_units: request.requested_source_units,
        source_capacity: candidate.source_capacity,
        delivery_units: request.requested_delivery_units,
        delivery_capacity: request.delivery_capacity,
        source_to_work_route_capacity: candidate.source_to_work_route_capacity,
        work_to_delivery_route_capacity: candidate.work_to_delivery_route_capacity,
    };
    resolved.validate()?;
    Ok(resolved)
}

fn validate_objective(
    category: SpatialTaskCategory,
    candidate: &SpatialResolutionCandidate,
) -> Result<(), SpatialBlockReason> {
    candidate
        .objective
        .validate()
        .map_err(|_| SpatialBlockReason::SourceUnavailable)?;
    if !category_matches(category, &candidate.objective) || !candidate.objective_exists {
        return Err(SpatialBlockReason::SourceUnavailable);
    }
    let metadata = candidate.objective.metadata();
    if let Some(reason) = metadata.blocked_reason {
        return Err(reason);
    }
    if metadata.visibility == SiteVisibility::Hidden {
        return Err(SpatialBlockReason::UnrevealedObjective);
    }
    match metadata.lifecycle {
        SiteLifecycleStage::Active => {}
        SiteLifecycleStage::Planned
            if matches!(
                category,
                SpatialTaskCategory::Construction(_) | SpatialTaskCategory::RoadConstruction
            ) => {}
        SiteLifecycleStage::Depleted => return Err(SpatialBlockReason::SourceDepleted),
        SiteLifecycleStage::Destroyed | SiteLifecycleStage::Removed => {
            return Err(SpatialBlockReason::ObjectiveDestroyed);
        }
        SiteLifecycleStage::Planned => return Err(SpatialBlockReason::SourceUnavailable),
    }
    Ok(())
}

fn validate_work_position(
    category: SpatialTaskCategory,
    candidate: &SpatialResolutionCandidate,
) -> Result<(), SpatialBlockReason> {
    candidate
        .work_slot
        .validate()
        .map_err(|_| SpatialBlockReason::WorkPositionUnavailable)?;
    if !candidate.work_position_available
        || candidate.work_slot.site.stable_id() == candidate.objective.stable_id()
        || candidate.work_slot.site.metadata().visibility == SiteVisibility::Hidden
        || !matches!(
            candidate.work_slot.site.metadata().lifecycle,
            SiteLifecycleStage::Active | SiteLifecycleStage::Planned
        )
    {
        return Err(SpatialBlockReason::WorkPositionUnavailable);
    }
    if category == SpatialTaskCategory::FetchWater
        && work_position_overlaps_objective(&candidate.work_slot.site, &candidate.objective)
    {
        return Err(SpatialBlockReason::WorkPositionUnavailable);
    }
    Ok(())
}

fn validate_endpoint(request: &SpatialResolutionRequest) -> Result<(), SpatialBlockReason> {
    request
        .pinned_delivery_endpoint
        .validate()
        .map_err(|_| SpatialBlockReason::DeliveryEndpointUnavailable)?;
    let metadata = request.pinned_delivery_endpoint.metadata();
    if !request.delivery_endpoint_exists
        || metadata.visibility == SiteVisibility::Hidden
        || !matches!(metadata.lifecycle, SiteLifecycleStage::Active)
        || metadata.blocked_reason.is_some()
    {
        return Err(SpatialBlockReason::DeliveryEndpointUnavailable);
    }
    Ok(())
}

fn validate_route(route: &SiteRef) -> Result<(), SpatialBlockReason> {
    route
        .validate()
        .map_err(|_| SpatialBlockReason::RouteUnavailable)?;
    let SiteRef::OrderedRoute {
        metadata,
        route: tiles,
    } = route
    else {
        return Err(SpatialBlockReason::RouteUnavailable);
    };
    if metadata.visibility == SiteVisibility::Hidden
        || metadata.lifecycle != SiteLifecycleStage::Active
        || metadata.blocked_reason.is_some()
        || tiles.is_empty()
        || tiles.windows(2).any(|pair| {
            i64::from(pair[0].x).abs_diff(i64::from(pair[1].x))
                + i64::from(pair[0].y).abs_diff(i64::from(pair[1].y))
                != 1
        })
    {
        return Err(SpatialBlockReason::RouteUnavailable);
    }
    Ok(())
}

fn category_matches(category: SpatialTaskCategory, objective: &SiteRef) -> bool {
    use crate::spatial_tasks::ResourceSourceKind;
    match (category, objective) {
        (
            SpatialTaskCategory::Hunt,
            SiteRef::ResourceSource {
                resource_kind: ResourceSourceKind::Hunting,
                ..
            },
        )
        | (
            SpatialTaskCategory::FetchWater,
            SiteRef::ResourceSource {
                resource_kind: ResourceSourceKind::Water,
                ..
            },
        )
        | (
            SpatialTaskCategory::Fish,
            SiteRef::ResourceSource {
                resource_kind: ResourceSourceKind::FishHabitat,
                ..
            },
        )
        | (
            SpatialTaskCategory::Quarry,
            SiteRef::ResourceSource {
                resource_kind: ResourceSourceKind::Quarry,
                ..
            },
        ) => true,
        (
            SpatialTaskCategory::Logging,
            SiteRef::ResourceSource {
                resource_kind: ResourceSourceKind::Tree,
                footprint,
                ..
            },
        ) => logging_footprint_is_complete(footprint),
        (
            SpatialTaskCategory::AppleHarvest,
            SiteRef::ResourceSource {
                resource_kind: ResourceSourceKind::Tree,
                footprint,
                ..
            },
        ) => {
            footprint.width == 3
                && footprint.height == 3
                && footprint.tiles == footprint.rect().ordered_tiles()
        }
        (SpatialTaskCategory::HoleWork, SiteRef::Rect { footprint, .. }) => {
            footprint.width == 5
                && footprint.height == 5
                && footprint.tiles == footprint.rect().ordered_tiles()
        }
        (SpatialTaskCategory::CookhouseWork, SiteRef::Rect { footprint, .. }) => {
            footprint.width == 3
                && footprint.height == 3
                && footprint.tiles == footprint.rect().ordered_tiles()
        }
        (SpatialTaskCategory::FishingHutWork, SiteRef::OrderedTiles { tiles, .. }) => {
            tiles.len() == 10
        }
        (SpatialTaskCategory::Construction(expected), SiteRef::Building { building_type, .. })
        | (SpatialTaskCategory::StationWork(expected), SiteRef::Building { building_type, .. }) => {
            *building_type == expected
        }
        (SpatialTaskCategory::RoadConstruction, SiteRef::OrderedRoute { route, .. }) => {
            !route.is_empty()
        }
        (
            SpatialTaskCategory::WorkshopWork,
            SiteRef::Building {
                building_type: BuildingType::Workshop,
                anchor,
                footprint,
                ..
            },
        ) => *footprint == canonical_building_footprint(BuildingType::Workshop, *anchor),
        (
            SpatialTaskCategory::FarmWork,
            SiteRef::Building {
                building_type: BuildingType::Field,
                ..
            },
        ) => true,
        (SpatialTaskCategory::OfferingRitual, SiteRef::Stockpile { .. }) => true,
        _ => false,
    }
}

fn logging_footprint_is_complete(footprint: &TaskFootprint) -> bool {
    footprint.width == 2
        && footprint.height == 3
        && footprint.tiles.len() == 6
        && footprint.tiles == footprint.rect().ordered_tiles()
}

fn work_position_overlaps_objective(work: &SiteRef, objective: &SiteRef) -> bool {
    let Some(tile) = site_single_tile(work) else {
        return false;
    };
    objective
        .footprint()
        .is_some_and(|footprint| footprint.tiles.as_slice().contains(&tile))
}

fn site_single_tile(site: &SiteRef) -> Option<TilePoint> {
    match site {
        SiteRef::Tile { tile, .. } => Some(*tile),
        _ => None,
    }
}

fn blocked(reason: SpatialBlockReason) -> SpatialResolutionOutcome {
    SpatialResolutionOutcome::Blocked(Box::new(SpatialObjective::blocked(reason)))
}
