//! Typed spatial task contracts for the leader-AI overhaul.
//!
//! Derived from `docs/leader-ai-overhaul/spatial-task-contract.md`. This leaf owns
//! canonical tile ordering and building footprints; task resolution and assignment
//! remain separate later slices.

use std::{fmt, num::NonZeroU32};

use serde::{Deserialize, Serialize, de::Error as _};

use crate::types::BuildingType;

/// Stable identity of an authoritative site. IDs outlive selected contact tiles.
pub type SiteId = String;
/// Stable identity of a reservable work slot.
pub type WorkSlotId = String;

/// Defensive persistence bound, not a gameplay placement limit.
///
/// Routes have their own ordered representation; no rectangular objective needs more
/// than a 1024 x 1024 safety envelope.
pub const MAX_RECT_TILES: usize = 1024 * 1024;

/// A malformed spatial value rejected by constructors or persistence validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialInvariantError {
    InvalidRectangle,
    FootprintOutsideBounds,
    FootprintBoundsMismatch,
    RectFootprintMismatch,
    BuildingFootprintMismatch,
    EmptyStableId,
    ZeroWorkSlotCapacity,
}

impl fmt::Display for SpatialInvariantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidRectangle => "rectangle dimensions, area, or coordinates are invalid",
            Self::FootprintOutsideBounds => "footprint contains a tile outside its bounds",
            Self::FootprintBoundsMismatch => "footprint bounds do not match its canonical tiles",
            Self::RectFootprintMismatch => "rectangle and redundant footprint disagree",
            Self::BuildingFootprintMismatch => {
                "building footprint does not match the canonical type and anchor"
            }
            Self::EmptyStableId => "stable spatial ID cannot be empty",
            Self::ZeroWorkSlotCapacity => "work-slot capacity must be non-zero",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SpatialInvariantError {}

/// An authoritative world-tile coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TilePoint {
    pub x: i32,
    pub y: i32,
}

/// A non-empty rectangle anchored at its north-west (minimum) tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Rect {
    anchor: TilePoint,
    width: i32,
    height: i32,
}

impl Rect {
    /// Construct a non-empty rectangle whose final coordinate is representable.
    #[must_use]
    pub fn new(anchor: TilePoint, width: i32, height: i32) -> Option<Self> {
        Self::try_new(anchor, width, height).ok()
    }

    pub fn try_new(
        anchor: TilePoint,
        width: i32,
        height: i32,
    ) -> Result<Self, SpatialInvariantError> {
        let width_usize = usize::try_from(width)
            .ok()
            .filter(|width| *width > 0)
            .ok_or(SpatialInvariantError::InvalidRectangle)?;
        let height_usize = usize::try_from(height)
            .ok()
            .filter(|height| *height > 0)
            .ok_or(SpatialInvariantError::InvalidRectangle)?;
        let area = width_usize
            .checked_mul(height_usize)
            .filter(|area| *area <= MAX_RECT_TILES)
            .ok_or(SpatialInvariantError::InvalidRectangle)?;
        debug_assert!(area > 0);
        anchor
            .x
            .checked_add(width - 1)
            .ok_or(SpatialInvariantError::InvalidRectangle)?;
        anchor
            .y
            .checked_add(height - 1)
            .ok_or(SpatialInvariantError::InvalidRectangle)?;
        Ok(Self {
            anchor,
            width,
            height,
        })
    }

    #[must_use]
    pub const fn anchor(self) -> TilePoint {
        self.anchor
    }

    #[must_use]
    pub const fn width(self) -> i32 {
        self.width
    }

    #[must_use]
    pub const fn height(self) -> i32 {
        self.height
    }

    #[must_use]
    pub fn tile_count(self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    #[must_use]
    pub fn ordered_tiles(self) -> OrderedTiles {
        OrderedTiles::row_major(self)
    }
}

impl<'de> Deserialize<'de> for Rect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RectFields {
            anchor: TilePoint,
            width: i32,
            height: i32,
        }

        let fields = RectFields::deserialize(deserializer)?;
        Self::try_new(fields.anchor, fields.width, fields.height).map_err(D::Error::custom)
    }
}

/// A canonical row-major tile set: ascending `y`, then ascending `x`, without duplicates.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(transparent)]
pub struct OrderedTiles(Vec<TilePoint>);

impl OrderedTiles {
    /// Canonicalize an arbitrary set of tiles into deterministic row-major order.
    #[must_use]
    pub fn canonical(tiles: impl IntoIterator<Item = TilePoint>) -> Self {
        let mut tiles = tiles.into_iter().collect::<Vec<_>>();
        tiles.sort_unstable_by_key(|tile| (tile.y, tile.x));
        tiles.dedup();
        Self(tiles)
    }

    /// Enumerate every tile in `rect` in deterministic row-major order.
    #[must_use]
    pub fn row_major(rect: Rect) -> Self {
        let mut tiles = Vec::with_capacity(rect.tile_count());
        for dy in 0..rect.height() {
            for dx in 0..rect.width() {
                tiles.push(TilePoint {
                    x: rect.anchor().x + dx,
                    y: rect.anchor().y + dy,
                });
            }
        }
        Self(tiles)
    }

    #[must_use]
    pub fn as_slice(&self) -> &[TilePoint] {
        &self.0
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[must_use]
    pub fn into_vec(self) -> Vec<TilePoint> {
        self.0
    }
}

impl AsRef<[TilePoint]> for OrderedTiles {
    fn as_ref(&self) -> &[TilePoint] {
        self.as_slice()
    }
}

impl<'de> Deserialize<'de> for OrderedTiles {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Vec::<TilePoint>::deserialize(deserializer).map(Self::canonical)
    }
}

impl IntoIterator for OrderedTiles {
    type Item = TilePoint;
    type IntoIter = std::vec::IntoIter<TilePoint>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Complete authoritative footprint and its rectangular bounds.
///
/// `tiles` may be sparse for irregular sources; rectangular constructors enumerate
/// every cell and therefore keep width/height and tiles consistent by construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskFootprint {
    pub anchor: TilePoint,
    pub width: i32,
    pub height: i32,
    pub tiles: OrderedTiles,
}

impl TaskFootprint {
    #[must_use]
    pub fn rectangular(rect: Rect) -> Self {
        Self {
            anchor: rect.anchor(),
            width: rect.width(),
            height: rect.height(),
            tiles: rect.ordered_tiles(),
        }
    }

    /// Construct an irregular footprint with canonical tiles and minimal bounds.
    #[must_use]
    pub fn from_tiles(tiles: impl IntoIterator<Item = TilePoint>) -> Option<Self> {
        let tiles = OrderedTiles::canonical(tiles);
        let first = *tiles.as_slice().first()?;
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (first.x, first.x, first.y, first.y);
        for tile in tiles.as_slice().iter().copied().skip(1) {
            min_x = min_x.min(tile.x);
            max_x = max_x.max(tile.x);
            min_y = min_y.min(tile.y);
            max_y = max_y.max(tile.y);
        }
        Some(Self {
            anchor: TilePoint { x: min_x, y: min_y },
            width: max_x.checked_sub(min_x)?.checked_add(1)?,
            height: max_y.checked_sub(min_y)?.checked_add(1)?,
            tiles,
        })
    }

    #[must_use]
    pub fn rect(&self) -> Rect {
        Rect::try_new(self.anchor, self.width, self.height)
            .expect("validated task footprint always has valid bounds")
    }

    /// Validate a redundant persisted footprint without allocating its full rectangle.
    pub fn validate(&self) -> Result<(), SpatialInvariantError> {
        let rect = Rect::try_new(self.anchor, self.width, self.height)?;
        let Some(first) = self.tiles.as_slice().first().copied() else {
            return Err(SpatialInvariantError::FootprintBoundsMismatch);
        };
        let max_x = rect.anchor().x + rect.width() - 1;
        let max_y = rect.anchor().y + rect.height() - 1;
        if self.tiles.as_slice().iter().any(|tile| {
            tile.x < rect.anchor().x || tile.x > max_x || tile.y < rect.anchor().y || tile.y > max_y
        }) {
            return Err(SpatialInvariantError::FootprintOutsideBounds);
        }
        let (mut min_x, mut max_tile_x, mut min_y, mut max_tile_y) =
            (first.x, first.x, first.y, first.y);
        for tile in self.tiles.as_slice().iter().copied().skip(1) {
            min_x = min_x.min(tile.x);
            max_tile_x = max_tile_x.max(tile.x);
            min_y = min_y.min(tile.y);
            max_tile_y = max_tile_y.max(tile.y);
        }
        if min_x != rect.anchor().x
            || min_y != rect.anchor().y
            || max_tile_x != max_x
            || max_tile_y != max_y
        {
            return Err(SpatialInvariantError::FootprintBoundsMismatch);
        }
        Ok(())
    }
}

/// Persisted lifecycle of a referenced world site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteLifecycleStage {
    Planned,
    #[default]
    Active,
    Depleted,
    Destroyed,
    Removed,
}

/// Whether the site may be projected beyond authoritative simulation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteVisibility {
    Hidden,
    #[default]
    Revealed,
    Public,
}

/// Bounded reasons a spatial contract can be visible but unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpatialBlockReason {
    SourceUnavailable,
    SourceDepleted,
    ObjectiveDestroyed,
    WorkPositionUnavailable,
    DeliveryEndpointUnavailable,
    RouteUnavailable,
    CapacityUnavailable,
    ReservationConflict,
    UnrevealedObjective,
    NoWillingWorker,
    InvalidLegacySite,
}

/// Common stable state carried by every [`SiteRef`] variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteMetadata {
    pub stable_id: SiteId,
    pub lifecycle: SiteLifecycleStage,
    pub visibility: SiteVisibility,
    pub blocked_reason: Option<SpatialBlockReason>,
}

impl SiteMetadata {
    #[must_use]
    pub fn revealed(stable_id: impl Into<SiteId>) -> Self {
        Self {
            stable_id: stable_id.into(),
            lifecycle: SiteLifecycleStage::Active,
            visibility: SiteVisibility::Revealed,
            blocked_reason: None,
        }
    }
}

/// Stable discriminator for all supported site-reference families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteKind {
    Tile,
    Rect,
    OrderedTiles,
    Building,
    Stockpile,
    ResourceSource,
    OrderedRoute,
    Shrine,
    VillageTradeEndpoint,
}

/// Semantic kind of an authoritative natural or gathered resource source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceSourceKind {
    Hunting,
    Water,
    FishHabitat,
    Quarry,
    Tree,
    Stump,
    Fibre,
    Food,
    Herbs,
}

/// Typed reference to authoritative world state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SiteRef {
    Tile {
        metadata: SiteMetadata,
        tile: TilePoint,
    },
    Rect {
        metadata: SiteMetadata,
        rect: Rect,
        footprint: TaskFootprint,
    },
    OrderedTiles {
        metadata: SiteMetadata,
        tiles: OrderedTiles,
    },
    Building {
        metadata: SiteMetadata,
        building_id: String,
        building_type: BuildingType,
        anchor: TilePoint,
        footprint: TaskFootprint,
    },
    Stockpile {
        metadata: SiteMetadata,
        stockpile_id: String,
        footprint: TaskFootprint,
    },
    ResourceSource {
        metadata: SiteMetadata,
        source_id: String,
        resource_kind: ResourceSourceKind,
        footprint: TaskFootprint,
    },
    /// Route order is meaningful and therefore is never row-major canonicalized.
    OrderedRoute {
        metadata: SiteMetadata,
        route: Vec<TilePoint>,
    },
    Shrine {
        metadata: SiteMetadata,
        building_id: String,
        anchor: TilePoint,
        footprint: TaskFootprint,
    },
    VillageTradeEndpoint {
        metadata: SiteMetadata,
        colony_id: String,
        footprint: TaskFootprint,
    },
}

impl SiteRef {
    /// Create a building reference from the single canonical footprint authority.
    #[must_use]
    pub fn building(
        building_id: impl Into<String>,
        building_type: BuildingType,
        anchor: TilePoint,
    ) -> Self {
        let building_id = building_id.into();
        Self::Building {
            metadata: SiteMetadata::revealed(building_id.clone()),
            building_id,
            building_type,
            anchor,
            footprint: canonical_building_footprint(building_type, anchor),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> SiteKind {
        match self {
            Self::Tile { .. } => SiteKind::Tile,
            Self::Rect { .. } => SiteKind::Rect,
            Self::OrderedTiles { .. } => SiteKind::OrderedTiles,
            Self::Building { .. } => SiteKind::Building,
            Self::Stockpile { .. } => SiteKind::Stockpile,
            Self::ResourceSource { .. } => SiteKind::ResourceSource,
            Self::OrderedRoute { .. } => SiteKind::OrderedRoute,
            Self::Shrine { .. } => SiteKind::Shrine,
            Self::VillageTradeEndpoint { .. } => SiteKind::VillageTradeEndpoint,
        }
    }

    #[must_use]
    pub const fn metadata(&self) -> &SiteMetadata {
        match self {
            Self::Tile { metadata, .. }
            | Self::Rect { metadata, .. }
            | Self::OrderedTiles { metadata, .. }
            | Self::Building { metadata, .. }
            | Self::Stockpile { metadata, .. }
            | Self::ResourceSource { metadata, .. }
            | Self::OrderedRoute { metadata, .. }
            | Self::Shrine { metadata, .. }
            | Self::VillageTradeEndpoint { metadata, .. } => metadata,
        }
    }

    #[must_use]
    pub fn stable_id(&self) -> &str {
        self.metadata().stable_id.as_str()
    }

    #[must_use]
    pub const fn footprint(&self) -> Option<&TaskFootprint> {
        match self {
            Self::Rect { footprint, .. }
            | Self::Building { footprint, .. }
            | Self::Stockpile { footprint, .. }
            | Self::ResourceSource { footprint, .. }
            | Self::Shrine { footprint, .. }
            | Self::VillageTradeEndpoint { footprint, .. } => Some(footprint),
            Self::Tile { .. } | Self::OrderedTiles { .. } | Self::OrderedRoute { .. } => None,
        }
    }

    /// Validate invariants after deserializing a persisted or wire-projected reference.
    ///
    /// `SiteRef` keeps public variants for exhaustive matching, so persistence owners must
    /// call this method before installing a decoded value into authoritative state.
    pub fn validate(&self) -> Result<(), SpatialInvariantError> {
        if self.stable_id().is_empty() {
            return Err(SpatialInvariantError::EmptyStableId);
        }
        match self {
            Self::Tile { .. } | Self::OrderedTiles { .. } | Self::OrderedRoute { .. } => Ok(()),
            Self::Rect {
                rect, footprint, ..
            } => {
                footprint.validate()?;
                if *footprint != TaskFootprint::rectangular(*rect) {
                    return Err(SpatialInvariantError::RectFootprintMismatch);
                }
                Ok(())
            }
            Self::Building {
                building_type,
                anchor,
                footprint,
                ..
            } => {
                footprint.validate()?;
                if *footprint != try_canonical_building_footprint(*building_type, *anchor)? {
                    return Err(SpatialInvariantError::BuildingFootprintMismatch);
                }
                Ok(())
            }
            Self::Shrine {
                anchor, footprint, ..
            } => {
                footprint.validate()?;
                if *footprint != try_canonical_building_footprint(BuildingType::Shrine, *anchor)? {
                    return Err(SpatialInvariantError::BuildingFootprintMismatch);
                }
                Ok(())
            }
            Self::Stockpile { footprint, .. }
            | Self::ResourceSource { footprint, .. }
            | Self::VillageTradeEndpoint { footprint, .. } => footprint.validate(),
        }
    }
}

/// The semantic role a typed site plays in one task contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpatialRole {
    Objective,
    WorkPosition,
    DeliveryEndpoint,
}

/// Whether a work slot is unique or owns a bounded share of site capacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkSlotReservation {
    Exclusive,
    Capacity(NonZeroU32),
}

/// A stable, separately reservable work position for one task stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkSlot {
    pub stable_id: WorkSlotId,
    pub site: SiteRef,
    pub reservation: WorkSlotReservation,
}

impl WorkSlot {
    #[must_use]
    pub fn exclusive(stable_id: impl Into<WorkSlotId>, site: SiteRef) -> Self {
        Self {
            stable_id: stable_id.into(),
            site,
            reservation: WorkSlotReservation::Exclusive,
        }
    }

    pub fn capacity(
        stable_id: impl Into<WorkSlotId>,
        site: SiteRef,
        units: u32,
    ) -> Result<Self, SpatialInvariantError> {
        let units = NonZeroU32::new(units).ok_or(SpatialInvariantError::ZeroWorkSlotCapacity)?;
        Ok(Self {
            stable_id: stable_id.into(),
            site,
            reservation: WorkSlotReservation::Capacity(units),
        })
    }

    pub fn validate(&self) -> Result<(), SpatialInvariantError> {
        if self.stable_id.is_empty() {
            return Err(SpatialInvariantError::EmptyStableId);
        }
        self.site.validate()
    }
}

/// Complete spatial contract for a task before runtime assignment is introduced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpatialObjective {
    pub objective: Option<SiteRef>,
    pub work_positions: Vec<WorkSlot>,
    pub delivery_endpoint: Option<SiteRef>,
    pub blocked_reason: Option<SpatialBlockReason>,
}

impl SpatialObjective {
    #[must_use]
    pub fn resolved(
        objective: SiteRef,
        work_positions: Vec<WorkSlot>,
        delivery_endpoint: Option<SiteRef>,
    ) -> Self {
        Self {
            objective: Some(objective),
            work_positions,
            delivery_endpoint,
            blocked_reason: None,
        }
    }

    /// Represent a failure before objective resolution without inventing a marker.
    #[must_use]
    pub fn blocked(reason: SpatialBlockReason) -> Self {
        Self {
            objective: None,
            work_positions: Vec::new(),
            delivery_endpoint: None,
            blocked_reason: Some(reason),
        }
    }

    #[must_use]
    pub fn site_for_role(&self, role: SpatialRole, index: usize) -> Option<&SiteRef> {
        match role {
            SpatialRole::Objective => (index == 0).then_some(self.objective.as_ref()).flatten(),
            SpatialRole::WorkPosition => self.work_positions.get(index).map(|slot| &slot.site),
            SpatialRole::DeliveryEndpoint => (index == 0)
                .then_some(self.delivery_endpoint.as_ref())
                .flatten(),
        }
    }

    #[must_use]
    pub fn footprint(&self) -> Option<&TaskFootprint> {
        self.objective.as_ref()?.footprint()
    }

    pub fn validate(&self) -> Result<(), SpatialInvariantError> {
        if let Some(objective) = &self.objective {
            objective.validate()?;
        }
        for work_position in &self.work_positions {
            work_position.validate()?;
        }
        if let Some(delivery_endpoint) = &self.delivery_endpoint {
            delivery_endpoint.validate()?;
        }
        Ok(())
    }
}

/// Tile footprint `(width, height)` occupied by a building type.
///
/// This is the single size table for both legacy runtime placement and typed spatial
/// objectives. Building instances persist only their type and north-west anchor.
#[must_use]
pub const fn footprint_for(building_type: BuildingType) -> (i32, i32) {
    match building_type {
        BuildingType::Shrine
        | BuildingType::Workshop
        | BuildingType::Smithy
        | BuildingType::FoodStorage
        | BuildingType::WoodCutter
        | BuildingType::StonePrep
        | BuildingType::Woodworking
        | BuildingType::Clothier
        | BuildingType::Tannery
        | BuildingType::Smelter
        | BuildingType::Mill
        | BuildingType::Sawmill
        | BuildingType::ElderLodge
        // LAI.46 stations. The Fishing Hut's canonical footprint is its 3x3
        // *land* rectangle only; its oriented dock cell and reserved water
        // attachment are separate typed roles owned by `fishing`.
        | BuildingType::Cookhouse
        | BuildingType::FishingHut => (3, 3),
        BuildingType::Den
        | BuildingType::Beds
        | BuildingType::Nursery
        | BuildingType::FamilyHome
        | BuildingType::HerbGarden
        | BuildingType::ElderCorner
        | BuildingType::MouseFarm
        | BuildingType::Field
        | BuildingType::Barracks
        | BuildingType::ResearchHut
        | BuildingType::School
        | BuildingType::AccountingTent => (2, 3),
        BuildingType::WaterBowl | BuildingType::Walls => (1, 1),
    }
}

/// Tiles covered by a `width x height` rectangle in row-major order.
/// Returns an empty vector for a non-positive dimension.
#[must_use]
pub fn footprint_tiles(anchor: TilePoint, width: i32, height: i32) -> Vec<TilePoint> {
    let Some(rect) = Rect::new(anchor, width, height) else {
        return Vec::new();
    };
    rect.ordered_tiles().into_vec()
}

/// Complete canonical footprint for a building at its north-west anchor.
#[must_use]
pub fn canonical_building_footprint(
    building_type: BuildingType,
    anchor: TilePoint,
) -> TaskFootprint {
    try_canonical_building_footprint(building_type, anchor)
        .expect("runtime building anchor must support its canonical footprint")
}

/// Fallible canonical footprint construction for persistence validation.
pub fn try_canonical_building_footprint(
    building_type: BuildingType,
    anchor: TilePoint,
) -> Result<TaskFootprint, SpatialInvariantError> {
    let (width, height) = footprint_for(building_type);
    Rect::try_new(anchor, width, height).map(TaskFootprint::rectangular)
}
