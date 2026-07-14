//! Spatial stockpiles (P12.3) — goods physically live in on-map containers.
//!
//! **Physical-store model.** `ColonyRuntime.resources` is a compatibility aggregate;
//! stockpiles plus station input/transit stores are the physical source of truth and sum
//! back to it (finished station output is excluded until delivered):
//!
//! > INVARIANT: `sum(stockpile.contents) == colony.resources` for every resource, every tick.
//!
//! Exactly one finite seeded **general storehouse** ([`GENERAL_STOREHOUSE_ID`], accepts every
//! resource, located at the founding FoodStorage) absorbs the ordinary balance. Deposits
//! arrivals) route their carried goods to the nearest accepting player pile with headroom
//! (falling back to the storehouse) while still crediting `colony.resources` exactly as
//! before. Every *other* resource change (consumption, spoilage, production, caps, tithe,
//! upgrades) keeps mutating `colony.resources` untouched; [`reconcile`] then folds the whole
//! net change into the storehouse at end of tick. Legacy shrine reservoirs migrate
//! deterministically to this finite store on load/reconcile.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{entities::Resources, zones::ZoneRect};

/// Legacy id migrated to [`GENERAL_STOREHOUSE_ID`] at reconciliation.
pub const SHRINE_STOCKPILE_ID: &str = "stockpile-shrine";
pub const GENERAL_STOREHOUSE_ID: &str = "stockpile-storehouse";
pub const GENERAL_STOREHOUSE_CAPACITY: f64 = 360.0;

/// Reserved, persisted station-local stores. They use the existing stockpile JSON so
/// saves can resume mid-production without a parallel persistence format, but they are
/// never player deposit targets. Input stores are part of the aggregate resource ledger;
/// output stores are work-in-progress and are credited only after an outbound physical
/// haul reaches a general pile.
pub const STATION_INPUT_PREFIX: &str = "station-input:";
pub const STATION_OUTPUT_PREFIX: &str = "station-output:";
pub const STATION_TRANSIT_PREFIX: &str = "station-transit:";
pub const STATION_LOCAL_CAPACITY: f64 = 10.0;

/// Per-tile capacity of a *designated* (player) stockpile, per resource.
pub const STOCKPILE_TILE_CAPACITY: f64 = 40.0;

/// Largest square edge (in tiles) a designated stockpile may span — reuses the zone cap.
pub const STOCKPILE_MAX_EDGE: i32 = crate::zones::ZONE_MAX_EDGE;

/// Most designated (non-shrine) stockpiles a colony may hold at once.
pub const MAX_DESIGNATED_STOCKPILES: usize = 8;

/// A **gather spot** (P16): a temporary, single-resource drop point placeable outside
/// the claimed village, unlike a general designated [`Stockpile`]. Reuses the ordinary
/// `Stockpile` machinery unchanged (deposit routing/`reconcile`/capacity all apply
/// exactly as for any other pile with a single-element `accepts` set) — this record is
/// purely the P16 bookkeeping layered on top: which resource it exists for, and when it
/// expires. Held in `ColonyRuntime.gather_spots`, one entry per gather-spot stockpile
/// (matched by [`Self::stockpile_id`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatherSpot {
    /// Id of the underlying [`Stockpile`] this record annotates.
    pub stockpile_id: String,
    /// The single resource this gather spot collects. Restricted to the resources a
    /// gatherer job can actually carry (see `entities::CarryingKind`): food, water,
    /// materials.
    pub kind: ResourceKind,
    /// Game-tick ms after which this gather spot expires and is cleared, folding
    /// whatever it still holds back into the shrine reservoir (same as a manual
    /// [`crate::stockpiles`] removal — never lost, just re-routed).
    pub expires_at_ms: i64,
    /// Ordinary resource drop point or a designated shoreline fishing workplace.
    /// Default preserves every pre-fishing JSON save.
    #[serde(default)]
    pub purpose: GatherSpotPurpose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatherSpotPurpose {
    #[default]
    General,
    Fishing,
}

impl GatherSpot {
    /// Whether this gather spot's TTL has elapsed by `now_ms`.
    #[must_use]
    pub fn is_expired(&self, now_ms: i64) -> bool {
        now_ms >= self.expires_at_ms
    }
}

/// Default lifetime of a gather spot from designation (P16: "temporary" — expires on
/// its own so an abandoned or exhausted spot doesn't permanently squat on the map).
pub const GATHER_SPOT_TTL_MS: i64 = 30 * 60 * 1000;

/// Largest square edge (in tiles) a gather spot may span. Deliberately smaller than a
/// general stockpile's [`STOCKPILE_MAX_EDGE`] — it is a small temp drop point next to a
/// worked resource, not a warehouse.
pub const GATHER_SPOT_MAX_EDGE: i32 = 3;

/// Most gather spots a colony may have designated at once, independent of the general
/// [`MAX_DESIGNATED_STOCKPILES`] pool (gather spots are cheap and short-lived, so they
/// get their own, slightly larger, budget).
pub const MAX_GATHER_SPOTS: usize = 6;

/// A resource kind — the eight fields of [`Resources`], usable as a set element / map key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Food,
    Water,
    Herbs,
    Catnip,
    Grain,
    Flour,
    Materials,
    Refined,
    Weapons,
    Armor,
    Logs,
    Lumber,
    Planks,
    Blocks,
    Tools,
    Fibre,
    Hide,
    Cloth,
    Leather,
    Ore,
    Metal,
    Blessings,
}

impl ResourceKind {
    /// Every resource kind, in a stable order (deterministic reconcile / iteration).
    pub const ALL: &'static [Self] = &[
        Self::Food,
        Self::Water,
        Self::Herbs,
        Self::Catnip,
        Self::Grain,
        Self::Flour,
        Self::Materials,
        Self::Refined,
        Self::Weapons,
        Self::Armor,
        Self::Logs,
        Self::Lumber,
        Self::Planks,
        Self::Blocks,
        Self::Tools,
        Self::Fibre,
        Self::Hide,
        Self::Cloth,
        Self::Leather,
        Self::Ore,
        Self::Metal,
        Self::Blessings,
    ];
}

/// Read a resource amount by kind.
#[must_use]
pub fn resource_amount(resources: &Resources, kind: ResourceKind) -> f64 {
    match kind {
        ResourceKind::Food => resources.food,
        ResourceKind::Water => resources.water,
        ResourceKind::Herbs => resources.herbs,
        ResourceKind::Catnip => resources.catnip,
        ResourceKind::Grain => resources.grain,
        ResourceKind::Flour => resources.flour,
        ResourceKind::Materials => resources.materials,
        ResourceKind::Refined => resources.refined,
        ResourceKind::Weapons => resources.weapons,
        ResourceKind::Armor => resources.armor,
        ResourceKind::Logs => resources.logs,
        ResourceKind::Lumber => resources.lumber,
        ResourceKind::Planks => resources.planks,
        ResourceKind::Blocks => resources.blocks,
        ResourceKind::Tools => resources.tools,
        ResourceKind::Fibre => resources.fibre,
        ResourceKind::Hide => resources.hide,
        ResourceKind::Cloth => resources.cloth,
        ResourceKind::Leather => resources.leather,
        ResourceKind::Ore => resources.ore,
        ResourceKind::Metal => resources.metal,
        ResourceKind::Blessings => resources.blessings,
    }
}

/// Overwrite a resource amount by kind.
pub fn set_resource(resources: &mut Resources, kind: ResourceKind, value: f64) {
    match kind {
        ResourceKind::Food => resources.food = value,
        ResourceKind::Water => resources.water = value,
        ResourceKind::Herbs => resources.herbs = value,
        ResourceKind::Catnip => resources.catnip = value,
        ResourceKind::Grain => resources.grain = value,
        ResourceKind::Flour => resources.flour = value,
        ResourceKind::Materials => resources.materials = value,
        ResourceKind::Refined => resources.refined = value,
        ResourceKind::Weapons => resources.weapons = value,
        ResourceKind::Armor => resources.armor = value,
        ResourceKind::Logs => resources.logs = value,
        ResourceKind::Lumber => resources.lumber = value,
        ResourceKind::Planks => resources.planks = value,
        ResourceKind::Blocks => resources.blocks = value,
        ResourceKind::Tools => resources.tools = value,
        ResourceKind::Fibre => resources.fibre = value,
        ResourceKind::Hide => resources.hide = value,
        ResourceKind::Cloth => resources.cloth = value,
        ResourceKind::Leather => resources.leather = value,
        ResourceKind::Ore => resources.ore = value,
        ResourceKind::Metal => resources.metal = value,
        ResourceKind::Blessings => resources.blessings = value,
    }
}

/// Add to a resource amount by kind.
pub fn add_resource(resources: &mut Resources, kind: ResourceKind, delta: f64) {
    set_resource(resources, kind, resource_amount(resources, kind) + delta);
}

/// A designatable, on-map container holding real resources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stockpile {
    pub id: String,
    pub rect: ZoneRect,
    pub accepts: BTreeSet<ResourceKind>,
    pub contents: Resources,
}

impl Stockpile {
    #[must_use]
    pub fn is_shrine(&self) -> bool {
        self.id == SHRINE_STOCKPILE_ID || self.id == GENERAL_STOREHOUSE_ID
    }

    #[must_use]
    pub fn is_general_storehouse(&self) -> bool {
        self.id == GENERAL_STOREHOUSE_ID
    }

    #[must_use]
    pub fn is_station_input(&self) -> bool {
        self.id.starts_with(STATION_INPUT_PREFIX)
    }

    #[must_use]
    pub fn is_station_output(&self) -> bool {
        self.id.starts_with(STATION_OUTPUT_PREFIX)
    }

    #[must_use]
    pub fn is_station_transit(&self) -> bool {
        self.id.starts_with(STATION_TRANSIT_PREFIX)
    }

    #[must_use]
    pub fn is_station_local(&self) -> bool {
        self.is_station_input() || self.is_station_output() || self.is_station_transit()
    }

    /// Tile count of the (inclusive-edge) footprint.
    #[must_use]
    pub fn tiles(&self) -> f64 {
        let w = (self.rect.x2 - self.rect.x1 + 1).max(0);
        let h = (self.rect.y2 - self.rect.y1 + 1).max(0);
        f64::from(w) * f64::from(h)
    }

    /// Per-resource capacity: finite for every current pile; only a not-yet-migrated
    /// legacy shrine row temporarily reports unbounded capacity.
    #[must_use]
    pub fn capacity(&self) -> Option<f64> {
        if self.id == SHRINE_STOCKPILE_ID {
            None
        } else if self.is_general_storehouse() {
            Some(GENERAL_STOREHOUSE_CAPACITY)
        } else if self.is_station_local() {
            Some(STATION_LOCAL_CAPACITY)
        } else {
            Some(self.tiles() * STOCKPILE_TILE_CAPACITY)
        }
    }

    /// Center of this pile's footprint — the same point deposit routing measures distance
    /// from, so hauling destinations and [`deposit_index`] agree on where a pile "is".
    #[must_use]
    pub fn center(&self) -> (f64, f64) {
        rect_center(self.rect)
    }

    /// Whether this pile will accept more of `kind` right now.
    #[must_use]
    pub fn has_headroom(&self, kind: ResourceKind) -> bool {
        if !self.accepts.contains(&kind) {
            return false;
        }
        self.capacity()
            .is_none_or(|cap| resource_amount(&self.contents, kind) < cap)
    }

    #[must_use]
    pub fn headroom(&self, kind: ResourceKind) -> f64 {
        if !self.accepts.contains(&kind) {
            return 0.0;
        }
        self.capacity().map_or(f64::INFINITY, |capacity| {
            (capacity - resource_amount(&self.contents, kind)).max(0.0)
        })
    }
}

#[must_use]
pub fn station_input_id(building_id: &str) -> String {
    format!("{STATION_INPUT_PREFIX}{building_id}")
}

#[must_use]
pub fn station_output_id(building_id: &str) -> String {
    format!("{STATION_OUTPUT_PREFIX}{building_id}")
}

#[must_use]
pub fn station_transit_id(building_id: &str) -> String {
    format!("{STATION_TRANSIT_PREFIX}{building_id}")
}

#[must_use]
pub fn make_station_store(
    id: String,
    rect: ZoneRect,
    accepts: impl IntoIterator<Item = ResourceKind>,
) -> Stockpile {
    Stockpile {
        id,
        rect,
        accepts: accepts.into_iter().collect(),
        contents: Resources::default(),
    }
}

/// Safely return an inclusive rectangle's `(width, height)`.
///
/// `ZoneRect` normally comes from `zones::normalize_rect`, but persisted legacy
/// rows and untrusted action coordinates still deserve a total validator.  The
/// `i64` arithmetic avoids debug-build overflow for the full `i32` range.
#[must_use]
pub fn rect_dimensions(rect: ZoneRect) -> Option<(i64, i64)> {
    let width = i64::from(rect.x2) - i64::from(rect.x1) + 1;
    let height = i64::from(rect.y2) - i64::from(rect.y1) + 1;
    (width > 0 && height > 0).then_some((width, height))
}

/// Whether two inclusive rectangles share at least one tile.
#[must_use]
pub const fn rects_overlap(left: ZoneRect, right: ZoneRect) -> bool {
    left.x1 <= right.x2 && left.x2 >= right.x1 && left.y1 <= right.y2 && left.y2 >= right.y1
}

/// Whether an inclusive rectangle contains `(x, y)`.
#[must_use]
pub const fn rect_contains(rect: ZoneRect, x: i32, y: i32) -> bool {
    x >= rect.x1 && x <= rect.x2 && y >= rect.y1 && y <= rect.y2
}

/// The shrine reservoir's footprint, centered on the village anchor tile.
#[must_use]
pub fn shrine_rect(anchor_x: i32, anchor_y: i32) -> ZoneRect {
    ZoneRect {
        x1: anchor_x,
        y1: anchor_y,
        x2: anchor_x,
        y2: anchor_y,
    }
}

/// A fresh finite seeded storehouse (legacy function name retained for call-site stability).
#[must_use]
pub fn make_shrine(rect: ZoneRect) -> Stockpile {
    Stockpile {
        id: GENERAL_STOREHOUSE_ID.to_owned(),
        rect,
        accepts: ResourceKind::ALL.iter().copied().collect(),
        contents: Resources::default(),
    }
}

fn shrine_index(stockpiles: &mut Vec<Stockpile>, shrine_rect: ZoneRect) -> usize {
    if stockpiles
        .iter()
        .any(|pile| pile.id == GENERAL_STOREHOUSE_ID)
    {
        stockpiles.retain(|pile| pile.id != SHRINE_STOCKPILE_ID);
        let idx = stockpiles
            .iter()
            .position(|pile| pile.id == GENERAL_STOREHOUSE_ID)
            .expect("storehouse retained");
        stockpiles[idx].rect = shrine_rect;
        return idx;
    }
    if let Some(idx) = stockpiles
        .iter()
        .position(|pile| pile.id == SHRINE_STOCKPILE_ID)
    {
        stockpiles[idx].id = GENERAL_STOREHOUSE_ID.to_owned();
        stockpiles[idx].rect = shrine_rect;
        stockpiles[idx].accepts = ResourceKind::ALL.iter().copied().collect();
        return idx;
    }
    stockpiles.push(make_shrine(shrine_rect));
    stockpiles.len() - 1
}

fn rect_center(rect: ZoneRect) -> (f64, f64) {
    (
        f64::from(rect.x1 + rect.x2) / 2.0,
        f64::from(rect.y1 + rect.y2) / 2.0,
    )
}

/// Choose the stockpile a deposit of `kind` arriving at `(from_x, from_y)` should land in:
/// the nearest accepting pile with headroom, tie-broken by id; the seeded storehouse is the
/// normal fallback. Returns `None` only if no pile can hold it.
#[must_use]
pub fn deposit_index(
    stockpiles: &[Stockpile],
    kind: ResourceKind,
    from_x: f64,
    from_y: f64,
) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (idx, pile) in stockpiles.iter().enumerate() {
        if pile.is_station_local() || !pile.has_headroom(kind) {
            continue;
        }
        let (cx, cy) = rect_center(pile.rect);
        let dist = (cx - from_x).powi(2) + (cy - from_y).powi(2);
        let better = match best {
            None => true,
            Some((best_idx, best_dist)) => {
                dist < best_dist || (dist == best_dist && pile.id < stockpiles[best_idx].id)
            }
        };
        if better {
            best = Some((idx, dist));
        }
    }
    best.map(|(idx, _)| idx)
        .or_else(|| stockpiles.iter().position(Stockpile::is_shrine))
}

/// Like [`deposit_index`], but skips every pile whose id is in `gather_spot_ids` — used
/// when a P16 mover hauls a gather spot's contents onward: it must land in a genuine
/// village stockpile/shrine, never back into the same (or another) gather spot, which
/// would just shuffle the goods sideways instead of moving them toward the village.
#[must_use]
pub fn village_deposit_index(
    stockpiles: &[Stockpile],
    gather_spot_ids: &[String],
    kind: ResourceKind,
    from_x: f64,
    from_y: f64,
) -> Option<usize> {
    let mut best: Option<(usize, f64)> = None;
    for (idx, pile) in stockpiles.iter().enumerate() {
        if pile.is_station_local() || gather_spot_ids.contains(&pile.id) || !pile.has_headroom(kind)
        {
            continue;
        }
        let (cx, cy) = rect_center(pile.rect);
        let dist = (cx - from_x).powi(2) + (cy - from_y).powi(2);
        let better = match best {
            None => true,
            Some((best_idx, best_dist)) => {
                dist < best_dist || (dist == best_dist && pile.id < stockpiles[best_idx].id)
            }
        };
        if better {
            best = Some((idx, dist));
        }
    }
    best.map(|(idx, _)| idx)
        .or_else(|| stockpiles.iter().position(Stockpile::is_shrine))
}

/// Restore the invariant: set the finite storehouse to `resources − sum(other physical piles)` per
/// resource, draining player piles (deterministically, by id) when they hold more than the
/// current total. Never mutates `resources`, so the economy stays byte-identical. Creates
/// the storehouse if absent and migrates any legacy shrine row.
pub fn reconcile(
    stockpiles: &mut Vec<Stockpile>,
    resources: &mut Resources,
    shrine_rect: ZoneRect,
) {
    let shrine_idx = shrine_index(stockpiles, shrine_rect);

    let mut player: Vec<usize> = (0..stockpiles.len())
        .filter(|&idx| idx != shrine_idx && !stockpiles[idx].is_station_output())
        .collect();
    player.sort_by(|&a, &b| stockpiles[a].id.cmp(&stockpiles[b].id));

    for &kind in ResourceKind::ALL {
        let total = resource_amount(resources, kind);
        let player_sum: f64 = player
            .iter()
            .map(|&idx| resource_amount(&stockpiles[idx].contents, kind))
            .sum();

        if player_sum > total {
            // Player piles hold more than the world now has (consumption ate into it):
            // drain the shortfall in id order and zero the reservoir.
            let mut overflow = player_sum - total;
            for &idx in &player {
                if overflow <= 0.0 {
                    break;
                }
                let have = resource_amount(&stockpiles[idx].contents, kind);
                let take = have.min(overflow);
                set_resource(&mut stockpiles[idx].contents, kind, have - take);
                overflow -= take;
            }
            set_resource(&mut stockpiles[shrine_idx].contents, kind, 0.0);
        } else {
            let residual = total - player_sum;
            let stored = stockpiles[shrine_idx]
                .capacity()
                .map_or(residual, |capacity| residual.min(capacity));
            set_resource(&mut stockpiles[shrine_idx].contents, kind, stored);
            // Physical capacity is authoritative. Any aggregate amount that cannot fit
            // in the finite storehouse or a designated pile is deterministic overflow,
            // not an invisible unbounded shrine balance.
            set_resource(resources, kind, player_sum + stored);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res(food: f64, water: f64, materials: f64) -> Resources {
        Resources {
            food,
            water,
            materials,
            ..Resources::default()
        }
    }

    fn player_pile(id: &str, rect: ZoneRect, accepts: &[ResourceKind]) -> Stockpile {
        Stockpile {
            id: id.to_owned(),
            rect,
            accepts: accepts.iter().copied().collect(),
            contents: Resources::default(),
        }
    }

    fn small_rect(x: i32, y: i32) -> ZoneRect {
        ZoneRect {
            x1: x,
            y1: y,
            x2: x,
            y2: y,
        }
    }

    fn pile_sum(stockpiles: &[Stockpile], kind: ResourceKind) -> f64 {
        stockpiles
            .iter()
            .map(|pile| resource_amount(&pile.contents, kind))
            .sum()
    }

    #[test]
    fn reconcile_seeds_shrine_and_holds_the_whole_total() {
        let mut piles = Vec::new();
        let mut resources = res(150.0, 100.0, 24.0);
        reconcile(&mut piles, &mut resources, small_rect(6, 6));

        assert_eq!(piles.len(), 1);
        assert!(piles[0].is_shrine());
        for &kind in ResourceKind::ALL {
            assert_eq!(
                pile_sum(&piles, kind).to_bits(),
                resource_amount(&resources, kind).to_bits()
            );
        }
    }

    #[test]
    fn reconcile_migrates_legacy_unbounded_shrine_to_finite_storehouse() {
        let rect = small_rect(2, 2);
        let mut legacy = make_shrine(rect);
        legacy.id = SHRINE_STOCKPILE_ID.to_owned();
        legacy.contents.food = 25.0;
        let mut piles = vec![legacy];
        let mut resources = res(25.0, 10.0, 5.0);
        let store_rect = ZoneRect {
            x1: 8,
            y1: 8,
            x2: 10,
            y2: 10,
        };

        reconcile(&mut piles, &mut resources, store_rect);

        assert_eq!(piles.len(), 1);
        assert_eq!(piles[0].id, GENERAL_STOREHOUSE_ID);
        assert_eq!(piles[0].rect, store_rect);
        assert_eq!(piles[0].capacity(), Some(GENERAL_STOREHOUSE_CAPACITY));
        assert_eq!(piles[0].contents.food, 25.0);
    }

    #[test]
    fn finite_storehouse_clamps_invisible_overflow_instead_of_hiding_it() {
        let mut piles = Vec::new();
        let mut resources = res(GENERAL_STOREHOUSE_CAPACITY + 40.0, 0.0, 0.0);

        reconcile(&mut piles, &mut resources, small_rect(6, 6));

        assert_eq!(resources.food, GENERAL_STOREHOUSE_CAPACITY);
        assert_eq!(piles[0].contents.food, GENERAL_STOREHOUSE_CAPACITY);
        assert_eq!(pile_sum(&piles, ResourceKind::Food), resources.food);
    }

    #[test]
    fn reconcile_with_a_player_pile_keeps_the_invariant_exactly() {
        let mut piles = vec![
            make_shrine(small_rect(6, 6)),
            player_pile("stockpile-a", small_rect(8, 8), &[ResourceKind::Food]),
        ];
        // A deposit already filled the player pile with 30 food.
        piles[1].contents.food = 30.0;
        let resources = res(150.0, 100.0, 24.0);
        let mut resources = resources;
        reconcile(&mut piles, &mut resources, small_rect(6, 6));

        assert_eq!(pile_sum(&piles, ResourceKind::Food), 150.0);
        assert_eq!(piles[1].contents.food, 30.0, "player pile retained");
        // Shrine holds the balance.
        let shrine = piles.iter().find(|p| p.is_shrine()).unwrap();
        assert_eq!(shrine.contents.food, 120.0);
    }

    #[test]
    fn reconcile_drains_player_piles_when_total_falls_below_their_holdings() {
        let mut piles = vec![
            make_shrine(small_rect(6, 6)),
            player_pile("stockpile-a", small_rect(8, 8), &[ResourceKind::Food]),
            player_pile("stockpile-b", small_rect(9, 9), &[ResourceKind::Food]),
        ];
        piles[1].contents.food = 40.0;
        piles[2].contents.food = 40.0;
        // Consumption dropped the world total below the 80 held in piles.
        let resources = res(50.0, 0.0, 0.0);
        let mut resources = resources;
        reconcile(&mut piles, &mut resources, small_rect(6, 6));

        assert_eq!(pile_sum(&piles, ResourceKind::Food), 50.0);
        // Drained in id order: "stockpile-a" first (loses 30 → 10), "stockpile-b" untouched.
        assert_eq!(piles[1].contents.food, 10.0);
        assert_eq!(piles[2].contents.food, 40.0);
        assert_eq!(piles[0].contents.food, 0.0, "shrine zeroed");
    }

    #[test]
    fn deposit_routes_to_the_nearest_accepting_pile_then_shrine() {
        let piles = vec![
            make_shrine(small_rect(6, 6)),
            player_pile("stockpile-far", small_rect(20, 20), &[ResourceKind::Food]),
            player_pile("stockpile-near", small_rect(8, 8), &[ResourceKind::Food]),
        ];
        // Depositing food near (8,8) picks the near food pile.
        assert_eq!(deposit_index(&piles, ResourceKind::Food, 8.0, 8.0), Some(2));
        // Water is accepted only by the shrine → routes there.
        assert_eq!(
            deposit_index(&piles, ResourceKind::Water, 8.0, 8.0),
            Some(0)
        );
    }

    #[test]
    fn deposit_skips_a_full_pile_and_falls_back() {
        let mut piles = vec![
            make_shrine(small_rect(6, 6)),
            player_pile("stockpile-near", small_rect(8, 8), &[ResourceKind::Food]),
        ];
        // Fill the 1-tile pile to capacity (40) → no headroom → shrine fallback.
        piles[1].contents.food = STOCKPILE_TILE_CAPACITY;
        assert_eq!(deposit_index(&piles, ResourceKind::Food, 8.0, 8.0), Some(0));
    }

    #[test]
    fn stockpile_round_trips_through_serde() {
        let pile = player_pile(
            "stockpile-a",
            small_rect(3, 4),
            &[ResourceKind::Food, ResourceKind::Water],
        );
        let json = serde_json::to_value(&pile).unwrap();
        assert_eq!(json["accepts"], serde_json::json!(["food", "water"]));
        let back: Stockpile = serde_json::from_value(json).unwrap();
        assert_eq!(back, pile);
    }

    #[test]
    fn gather_spot_is_expired_uses_inclusive_ttl_boundary() {
        let spot = GatherSpot {
            stockpile_id: "gather-1".to_owned(),
            kind: ResourceKind::Food,
            expires_at_ms: 10_000,
            purpose: GatherSpotPurpose::General,
        };
        assert!(!spot.is_expired(9_999));
        assert!(spot.is_expired(10_000));
        assert!(spot.is_expired(10_001));
    }

    #[test]
    fn gather_spot_round_trips_through_serde() {
        let spot = GatherSpot {
            stockpile_id: "gather-1".to_owned(),
            kind: ResourceKind::Materials,
            expires_at_ms: 42,
            purpose: GatherSpotPurpose::General,
        };
        let json = serde_json::to_value(&spot).unwrap();
        assert_eq!(json["stockpileId"], serde_json::json!("gather-1"));
        assert_eq!(json["kind"], serde_json::json!("materials"));
        let back: GatherSpot = serde_json::from_value(json).unwrap();
        assert_eq!(back, spot);
    }

    #[test]
    fn village_deposit_skips_gather_spots_even_when_nearest() {
        let piles = vec![
            // Shrine at (6,6): distance² 8 from (8,8).
            make_shrine(small_rect(6, 6)),
            // Gather spot exactly at (8,8): distance² 0 — nearest of all three.
            player_pile("gather-near", small_rect(8, 8), &[ResourceKind::Food]),
            // A genuine village pile at (9,9): distance² 2 — closer than the shrine,
            // but still farther than the (excluded) gather spot.
            player_pile("stockpile-village", small_rect(9, 9), &[ResourceKind::Food]),
        ];
        let gather_ids = vec!["gather-near".to_owned()];
        // The plain deposit_index would pick the near gather spot...
        assert_eq!(deposit_index(&piles, ResourceKind::Food, 8.0, 8.0), Some(1));
        // ...but village_deposit_index skips it, routing to the next-nearest real
        // village pile instead of either the gather spot or the farther-away shrine.
        assert_eq!(
            village_deposit_index(&piles, &gather_ids, ResourceKind::Food, 8.0, 8.0),
            Some(2)
        );
    }

    #[test]
    fn village_deposit_falls_back_to_shrine_when_only_gather_spots_accept() {
        let piles = vec![
            make_shrine(small_rect(6, 6)),
            player_pile("gather-1", small_rect(8, 8), &[ResourceKind::Water]),
        ];
        let gather_ids = vec!["gather-1".to_owned()];
        assert_eq!(
            village_deposit_index(&piles, &gather_ids, ResourceKind::Water, 8.0, 8.0),
            Some(0)
        );
    }
}
