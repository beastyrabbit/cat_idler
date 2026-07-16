//! Serde wire DTOs ported from `app/api/game/actions/route.ts`,
//! `server/game.ts` (`getGlobalDashboard`), and `hooks/useGameDashboard.ts`.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

fn is_false(value: &bool) -> bool {
    !*value
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldSnapshot {
    pub now: i64,
    pub world_seed: i64,
    pub colonies: Vec<ColonySnapshot>,
    pub online_count: u32,
    /// The full village currently selected for this socket. Additive for legacy
    /// snapshots, whose selected village remains the first colony.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_colony_id: Option<String>,
    /// Public summaries learned only after a scout physically observes another shrine
    /// and returns that contact knowledge to its own shrine.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_villages: Vec<VillageSummary>,
    /// Open atomic barter proposals visible to this socket. The server projects
    /// this list to villages the authenticated player may control.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub village_trade_offers: Vec<VillageTradeOfferSnapshot>,
    /// Accepted trades represented by their durable actor and finite escrow.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub village_trade_caravans: Vec<VillageTradeCaravanSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VillageKind {
    /// The shared founding village. Every authenticated player may control it.
    #[default]
    Global,
    /// A private, player-founded village.
    Personal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VillageScale {
    #[default]
    Personal,
    Communal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VillageCapabilities {
    pub can_view: bool,
    pub can_control: bool,
    pub is_owner: bool,
}

impl Default for VillageCapabilities {
    fn default() -> Self {
        // Legacy snapshots predate personalized projection and contained the
        // one shared global village, so retaining control is the compatible
        // client-side interpretation.
        Self {
            can_view: true,
            can_control: true,
            is_owner: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VillageSummary {
    pub id: String,
    pub name: String,
    pub kind: VillageKind,
    #[serde(default)]
    pub scale: VillageScale,
    pub anchor: TilePoint,
    #[serde(default)]
    pub capabilities: VillageCapabilities,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VillageTradeOfferSnapshot {
    pub id: String,
    pub from_colony_id: String,
    pub to_colony_id: String,
    pub offered_kind: ResourceKind,
    pub offered_amount: f64,
    pub requested_kind: ResourceKind,
    pub requested_amount: f64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VillageTradeCaravanPhase {
    Outbound,
    WaitingAtTarget,
    Returning,
    WaitingAtSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VillageTradeCaravanSnapshot {
    pub id: String,
    pub actor_id: String,
    pub from_colony_id: String,
    pub to_colony_id: String,
    pub offered_kind: ResourceKind,
    pub offered_amount: f64,
    pub requested_kind: ResourceKind,
    pub requested_amount: f64,
    /// Stable world-global identities for finite equipment cargo. Scalar cargo
    /// leaves the corresponding list empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub offered_item_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_item_ids: Vec<String>,
    pub phase: VillageTradeCaravanPhase,
    pub position: WorldPoint,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route: Vec<WorldPoint>,
    pub accepted_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColonySnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub name: String,
    /// Global/personal classification. Missing on legacy snapshots, which only
    /// carried the shared global village.
    #[serde(default)]
    pub kind: VillageKind,
    /// Mechanical founding scale. The canonical shared hub is `communal`; ordinary
    /// player-founded villages and legacy snapshots default to `personal`.
    #[serde(default)]
    pub scale: VillageScale,
    /// Audience-specific permissions. The server overwrites this while
    /// projecting a snapshot and never serializes ownership identifiers.
    #[serde(default)]
    pub capabilities: VillageCapabilities,
    pub status: ColonyStatus,
    /// Canonical snapshots may carry authoritative resources inside the trusted server, but
    /// the player-facing socket projection replaces physical stock with the Accountant's last
    /// report. Blessings remain exact because they are a non-stockpiled divine currency.
    pub resources: ResourceAmounts,
    pub storage: StorageSnapshot,
    pub leader: Option<LeaderSnapshot>,
    pub cats: Vec<CatSnapshot>,
    pub jobs: Vec<JobSnapshot>,
    pub upgrades: Vec<UpgradeSnapshot>,
    pub events: Vec<EventSnapshot>,
    pub housing: HousingSnapshot,
    pub research: ResearchSnapshot,
    pub election: Option<ElectionSnapshot>,
    /// The authoritative timing of the next automatic leadership election. This is
    /// present between elections so clients can explain the current term instead of
    /// inferring a deadline from their own clock. Absent on legacy snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub election_schedule: Option<ElectionScheduleSnapshot>,
    pub vote_kick: Option<VoteKickSnapshot>,
    pub zones: Vec<ZoneSnapshot>,
    pub threat: ThreatSnapshot,
    pub raiders: Vec<RaiderSnapshot>,
    pub buildings: Vec<BuildingSnapshot>,
    pub claimed_tiles: Vec<TilePoint>,
    /// Exterior territory owned for agriculture but intentionally excluded from the
    /// palisaded settlement. Always a subset of `claimed_tiles`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agricultural_tiles: Vec<TilePoint>,
    /// Permanent fog-of-war knowledge: the founding village area plus discoveries that
    /// scouts have delivered at the shrine. The client fogs every tile outside this set.
    /// Additive since P15; empty/absent for pre-fog snapshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revealed_tiles: Vec<TilePoint>,
    /// Fog-of-war P15: tiles a currently-out scout has *tentatively* uncovered but
    /// not yet delivered — the client should render these dim/provisional, distinct
    /// from the solid `revealed_tiles`. A tile moves from here into `revealed_tiles`
    /// once its scout reaches the shrine; it drops out (never committing) if that
    /// scout dies first. Additive since P15; empty/absent for pre-P15 snapshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provisional_tiles: Vec<TilePoint>,
    /// Paved road tiles (`overlay_feature == "road_built"`). The client draws roads
    /// over these. Additive; empty/absent when none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub road_tiles: Vec<TilePoint>,
    /// Constructed transport truth. Empty means the corresponding research is
    /// still only a blueprint and must not affect ordinary movement.
    #[serde(default)]
    pub transport: TransportSnapshot,
    /// Persisted felled tree anchors. Generated mature canopies are hidden and a
    /// stump prop is rendered at these coordinates.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stump_tiles: Vec<TilePoint>,
    /// Persisted growing tree anchors. Generated mature canopies remain hidden
    /// until the deterministic ecological timer matures them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sapling_tiles: Vec<TilePoint>,
    /// Traffic-formed dirt roads (`path_wear >= 70`) which have not been paved.
    /// They are distinct from authored stone roads and never form on stone ground.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dirt_road_tiles: Vec<TilePoint>,
    pub village_gate: Option<GatePlacement>,
    /// Authoritative wall edges. During expansion this contains the complete old
    /// enclosure plus only those new outer segments whose work has finished.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wall_segments: Vec<WallSegment>,
    pub village_radius: u32,
    pub anchor: TilePoint,
    /// Appointed officers (role → cat id). P12.2; empty/absent when none appointed.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub officers: BTreeMap<OfficerRole, String>,
    /// On-map stockpiles, including the finite seeded village storehouse. Rendered as visible
    /// piles sized to contents. Empty/absent for pre-P12.3 snapshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stockpiles: Vec<StockpileSnapshot>,
    /// Current physical pile-to-pile balancing route, if the Steward has one in flight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_stockpile_haul: Option<StockpileHaulSnapshot>,
    /// Visible farm designations and their current crop stage (P12.5). Empty for
    /// legacy snapshots and colonies without designated plots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub farms: Vec<FarmSnapshot>,
    /// The colony's reported stock ledger (P12.4a): the last-counted totals plus how fresh
    /// they are. Lags the true `resources` unless a staffed Accounting Tent keeps it exact.
    /// Absent for pre-P12.4a snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stock_ledger: Option<StockLedgerSnapshot>,
    /// The colony's item/material economy store (P19 slice 1: DF-scale item economy —
    /// `docs/migration/specs/p19-items-materials-trade.md`). Each stack is a distinct
    /// `(kind, material, quality)` combination with its held count and per-unit value.
    /// Additive and inert this slice: empty/absent for every colony, since nothing
    /// produces items yet (that lands in slice 2's material-variant workshop recipes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<ItemStackSnapshot>,
    /// Coin balance (P19 slice 3: visiting traders + a coin economy — see
    /// `docs/migration/specs/p19-items-materials-trade.md`'s "Traders / caravans"
    /// section). Its own currency, separate from `resources.blessings` and the
    /// upgrade-tree's research points. Additive; defaults to `0.0` for pre-P19-slice-3
    /// snapshots.
    #[serde(default)]
    pub coin: f64,
    /// The currently-visiting trader, if any. `None` most of the time — a colony sees
    /// at most one trader at once. Additive; absent for pre-P19-slice-3 snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trader: Option<TraderSnapshot>,
}

/// A visiting trader (P19 slice 3): its position/lifecycle state, and what it currently
/// offers to buy from / sell to the colony, at what coin price.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderSnapshot {
    pub id: String,
    pub position: TilePoint,
    pub state: TraderVisitState,
    /// Current physical route target (shrine contact while arriving, persisted exterior
    /// while departing). `None` for legacy payloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<TilePoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_exterior: Option<TilePoint>,
    #[serde(default)]
    pub visit_number: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrived_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visit_ends_at: Option<i64>,
    #[serde(default)]
    pub coin: f64,
    #[serde(default)]
    pub cargo_weight_grams: f64,
    #[serde(default)]
    pub cargo_capacity_grams: f64,
    /// Exact item stacks already acquired by this finite visit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cargo_items: Vec<ItemStackSnapshot>,
    /// Full finite resource manifest in every phase. `sell_offers` below is the
    /// actionable at-shrine projection; this inventory remains visible while the wagon
    /// approaches or departs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stock: Vec<TraderStockSnapshot>,
    /// Crafted-item stacks the colony currently holds and could sell to the trader
    /// (empty while `state != trading`, since selling is only valid then). Mirrors
    /// [`ItemStackSnapshot`]'s `kind`/`material`/`quality` string convention.
    pub buy_offers: Vec<TraderBuyOffer>,
    /// Resource manifest with stable prices and finite quantities in every phase.
    /// Actions remain valid only while `state == trading`.
    pub sell_offers: Vec<TraderSellOffer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraderVisitState {
    Arriving,
    Trading,
    Departing,
}

/// One item stack the trader will buy from the colony, and at what coin-per-unit price.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderBuyOffer {
    pub kind: String,
    pub material: String,
    pub quality: u8,
    /// How many of this stack the colony currently holds (the max sellable count).
    pub available: u32,
    /// Coin the trader pays per unit.
    pub unit_price: f64,
    /// Physical unit weight used by the caravan's bounded per-action load.
    #[serde(default)]
    pub unit_weight_grams: u32,
    /// Why this held stack cannot currently be sold. `available` is the authoritative
    /// actionable count; a blocked row remains visible so the UI never silently hides
    /// goods merely because the purse, wagon, or physical storage seam is unavailable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
}

/// One resource kind the trader will sell to the colony, and at what coin-per-unit price.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderSellOffer {
    pub resource: ResourceKind,
    /// Coin the trader charges per unit.
    pub unit_price: f64,
    /// Exact finite amount remaining in this visit's wagon manifest.
    #[serde(default)]
    pub available: f64,
    #[serde(default)]
    pub sold_out: bool,
    /// Authoritative non-price reason a one-unit purchase cannot currently complete,
    /// such as every accepting player-visible pile being full.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderStockSnapshot {
    pub resource: ResourceKind,
    pub available: f64,
    #[serde(default)]
    pub sold_out: bool,
}

/// One distinct crafted-item stack: kind × material × quality, with how many the
/// colony holds and what one is worth. `kind`/`material` are stable lowercase labels
/// (e.g. `"weapon"` / `"metal"`) rather than enums, so the client/trade UI can render
/// them without a shared enum type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemStackSnapshot {
    pub kind: String,
    pub material: String,
    pub quality: u8,
    pub count: u32,
    pub value: u32,
    /// Stable physical unit weight; total stack weight is `count * unit_weight_grams`.
    #[serde(default)]
    pub unit_weight_grams: u32,
    /// Finite units and their independently persisted condition.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub instances: Vec<ItemInstanceSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemInstanceSnapshot {
    pub id: String,
    pub durability: u32,
    pub max_durability: u32,
    pub broken: bool,
    /// Whether this unit has completed its first delivery into colony storage
    /// and therefore contributes to the legacy scalar compatibility projection.
    /// Pre-C3 item JSON already represented credited stored goods.
    #[serde(default = "default_true")]
    pub credited: bool,
    /// Exact authoritative location of this finite unit. Older snapshots only
    /// exposed condition and therefore deserialize into the legacy treasury
    /// until the server's one-time finite-equipment migration places the unit.
    #[serde(default)]
    pub location: ItemLocation,
}

/// Station-local compartments used by the physical production routes. Keeping
/// these explicit prevents a carried or completed item from also appearing in
/// generic village storage while it is still inside a workshop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationCompartment {
    Inbound,
    LocalInput,
    LocalOutput,
    Outbound,
}

/// One and only one authoritative place for a finite item identity.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ItemLocation {
    /// Compatibility location for unit identities loaded from saves which
    /// predate physical finite-equipment placement.
    #[default]
    LegacyTreasury,
    Stockpile {
        stockpile_id: String,
    },
    Station {
        building_id: String,
        compartment: StationCompartment,
    },
    Carrier {
        cat_id: String,
    },
    Equipped {
        cat_id: String,
    },
    Trader {
        trader_id: String,
    },
    Caravan {
        caravan_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockLedgerSnapshot {
    /// Stock totals as last *reported* by the bookkeeper (may lag the true resources).
    pub reported: ResourceAmounts,
    /// Game-tick timestamp (ms) of the last recount.
    pub last_counted: i64,
    /// Trusted/internal equality attestation retained for legacy snapshot compatibility.
    /// Player-facing servers omit `false`, and must clear it before a snapshot crosses the
    /// wire so report freshness cannot become an oracle for authoritative stock changes.
    #[serde(default, skip_serializing_if = "is_false")]
    pub accurate: bool,
    /// Physical work currently being performed by the Accounting Tent. Additive for older
    /// clients/snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_round: Option<AccountingRoundSnapshot>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountingPhase {
    TravelingToTent,
    TravelingToPile,
    Counting,
    ReturningToTent,
    WaitingAtTent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountingRoundSnapshot {
    pub worker_id: String,
    pub tent_id: String,
    pub phase: AccountingPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_stockpile_id: Option<String>,
    pub remaining_piles: usize,
    pub unreachable_piles: usize,
    pub dwell_elapsed_ms: i64,
    pub dwell_required_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockpileReportSnapshot {
    pub reported: ResourceAmounts,
    pub last_counted: i64,
    /// Trusted/internal equality attestation. Player-facing projections always clear and
    /// omit it; an absent value deserializes as `false` for older clients.
    #[serde(default, skip_serializing_if = "is_false")]
    pub accurate: bool,
}

/// Why an officer-created limited pile exists. Player designations omit this field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StewardManagedPileSnapshot {
    pub station_id: String,
    pub resource: ResourceKind,
    #[serde(default)]
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockpileHaulPhase {
    TravelingToSource,
    CarryingToDestination,
    RecoveryBlocked,
}

/// The one physical balancing transfer a Steward currently coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockpileHaulSnapshot {
    pub job_id: String,
    pub worker_id: String,
    pub source_stockpile_id: String,
    pub destination_stockpile_id: String,
    pub resource: ResourceKind,
    pub amount: f64,
    pub phase: StockpileHaulPhase,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockpileSnapshot {
    pub id: String,
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub accepts: Vec<ResourceKind>,
    pub contents: ResourceAmounts,
    /// Player-facing counted values. The server replaces `contents` with this report (or
    /// zero for an uncounted pile) in the socket projection, so neither numeric UI nor pile
    /// sprite size/shape can bypass the Accountant. Absent only on legacy snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<StockpileReportSnapshot>,
    /// Present when this pile is a P16 gather spot: a temporary, single-resource drop
    /// point placeable outside the claimed village. Absent for the shrine reservoir and
    /// every general player stockpile, and for pre-P16 snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gather_spot: Option<GatherSpotSnapshot>,
    /// Explicit persisted officer ownership; absent means player/system-owned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steward_managed: Option<StewardManagedPileSnapshot>,
}

/// A gather spot's P16-specific bookkeeping (see [`StockpileSnapshot::gather_spot`]):
/// the single resource it collects and when it expires.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatherSpotSnapshot {
    pub kind: ResourceKind,
    pub expires_at_ms: i64,
    /// Why this temporary pile exists. Fishing spots are one-tile shoreline
    /// work/drop points; legacy and ordinary gather spots default to `general`.
    #[serde(default)]
    pub purpose: GatherSpotPurpose,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fish_population: Option<FishPopulationSnapshot>,
}

/// Visible finite ecology for a fishing designation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FishPopulationSnapshot {
    pub stock: f64,
    pub capacity: f64,
    pub last_replenished_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GatherSpotPurpose {
    #[default]
    General,
    Fishing,
}

macro_rules! define_resource_kinds {
    ($( $(#[$meta:meta])* $kind:ident => $physical:literal),+ $(,)?) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(rename_all = "snake_case")]
        pub enum ResourceKind {
            $(
                $(#[$meta])*
                $kind,
            )+
        }

        impl ResourceKind {
            /// Every wire resource kind in stable display and serialization-test order.
            ///
            /// This inventory and [`Self::is_physical_stockpile_good`] are generated from
            /// the same declaration so adding a resource cannot silently omit it from the
            /// General-stockpile classification.
            pub const ALL: &'static [Self] = &[$(Self::$kind),+];

            /// Whether cats can haul and store this resource in a physical stockpile.
            #[must_use]
            pub const fn is_physical_stockpile_good(self) -> bool {
                match self {
                    $(Self::$kind => $physical),+
                }
            }

            /// Every physical stockpile good in [`Self::ALL`] order.
            pub fn physical_stockpile_goods() -> impl Iterator<Item = Self> {
                Self::ALL
                    .iter()
                    .copied()
                    .filter(|kind| kind.is_physical_stockpile_good())
            }
        }
    };
}

define_resource_kinds! {
    Food => true,
    Fish => true,
    Water => true,
    Herbs => true,
    Catnip => true,
    Grain => true,
    Flour => true,
    Preserves => true,
    Medicine => true,
    Brew => true,
    Materials => true,
    Stone => true,
    Refined => true,
    Weapons => true,
    Armor => true,
    Logs => true,
    Lumber => true,
    Planks => true,
    Blocks => true,
    Tools => true,
    Fibre => true,
    Hide => true,
    Bone => true,
    Cloth => true,
    Leather => true,
    Ore => true,
    Gem => true,
    Clay => true,
    Sand => true,
    Metal => true,
    /// Spendable divine favor, not an item cats can haul or place in a pile.
    Blessings => false,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColonyStatus {
    Starting,
    Thriving,
    Struggling,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceAmounts {
    pub food: f64,
    #[serde(default)]
    pub fish: f64,
    pub water: f64,
    pub herbs: f64,
    #[serde(default)]
    pub catnip: f64,
    #[serde(default)]
    pub grain: f64,
    #[serde(default)]
    pub flour: f64,
    #[serde(default)]
    pub preserves: f64,
    #[serde(default)]
    pub medicine: f64,
    #[serde(default)]
    pub brew: f64,
    pub materials: f64,
    #[serde(default)]
    pub stone: f64,
    pub refined: f64,
    pub weapons: f64,
    pub armor: f64,
    /// P12.4b refinement tier: planks (wood-cutter), blocks (stone-prep), tools
    /// (woodworking). Defaulted so legacy wire payloads still deserialize.
    #[serde(default)]
    pub planks: f64,
    #[serde(default)]
    pub logs: f64,
    #[serde(default)]
    pub lumber: f64,
    #[serde(default)]
    pub blocks: f64,
    #[serde(default)]
    pub tools: f64,
    #[serde(default)]
    pub fibre: f64,
    #[serde(default)]
    pub hide: f64,
    #[serde(default)]
    pub bone: f64,
    #[serde(default)]
    pub cloth: f64,
    #[serde(default)]
    pub leather: f64,
    #[serde(default)]
    pub ore: f64,
    #[serde(default)]
    pub gem: f64,
    #[serde(default)]
    pub clay: f64,
    #[serde(default)]
    pub sand: f64,
    #[serde(default)]
    pub metal: f64,
    pub blessings: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSnapshot {
    pub capacities: ResourceCapacities,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub food_capacity: Option<f64>,
    pub tithe_rates: TitheRates,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapacities {
    pub food: f64,
    #[serde(default)]
    pub fish: f64,
    pub water: f64,
    pub herbs: f64,
    #[serde(default)]
    pub catnip: f64,
    #[serde(default)]
    pub grain: f64,
    #[serde(default)]
    pub flour: f64,
    #[serde(default)]
    pub preserves: f64,
    #[serde(default)]
    pub medicine: f64,
    #[serde(default)]
    pub brew: f64,
    pub materials: f64,
    #[serde(default)]
    pub stone: f64,
    pub refined: f64,
    #[serde(default)]
    pub weapons: f64,
    #[serde(default)]
    pub armor: f64,
    /// P12.4b refinement-tier caps (planks/blocks/tools). Defaulted for legacy payloads.
    #[serde(default)]
    pub planks: f64,
    #[serde(default)]
    pub logs: f64,
    #[serde(default)]
    pub lumber: f64,
    #[serde(default)]
    pub blocks: f64,
    #[serde(default)]
    pub tools: f64,
    #[serde(default)]
    pub fibre: f64,
    #[serde(default)]
    pub hide: f64,
    #[serde(default)]
    pub bone: f64,
    #[serde(default)]
    pub cloth: f64,
    #[serde(default)]
    pub leather: f64,
    #[serde(default)]
    pub ore: f64,
    #[serde(default)]
    pub gem: f64,
    #[serde(default)]
    pub clay: f64,
    #[serde(default)]
    pub sand: f64,
    #[serde(default)]
    pub metal: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TitheRates {
    pub food: f64,
    pub refined: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderSnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub name: String,
    pub leadership: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatSnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub name: String,
    pub position: MapPosition,
    pub activity: CatActivity,
    pub destination: Option<MapPosition>,
    pub carrying: Option<Carrying>,
    /// Exact functional equipment currently worn or used by this cat. The item
    /// identities remain in `ColonySnapshot::items`; these fields are a compact
    /// loadout projection for inspectors and controls.
    #[serde(default)]
    pub equipment: EquipmentLoadoutSnapshot,
    pub specialization: Option<Specialization>,
    pub age_hours: f64,
    pub needs: CatNeeds,
    pub current_task: Option<String>,
    pub assigned_building_id: Option<String>,
    pub role_xp: RoleXp,
    /// Persisted, truthful per-labor proficiency. Empty/absent for legacy
    /// snapshots and cats that have not completed labor yet.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub skills: BTreeMap<Labor, f64>,
    pub stats: CatStats,
    pub death_time: Option<i64>,
    /// Known parent cat ids (mother/father), in `Cat::parent_ids` order, omitting any
    /// parent the sim doesn't know (founding cats, or a slot that was never filled).
    /// Additive; empty/absent for pre-lineage snapshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parent_ids: Vec<String>,
    /// Parent names resolved at snapshot time from `parent_ids` (best-effort — a
    /// deceased parent still resolves by name; a parent id with no matching cat is
    /// skipped). Additive; empty/absent for pre-lineage snapshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parents: Vec<String>,
    /// Player-set priority flag (P15 "cat booster"), mirrors `entities::Cat::boosted`.
    /// Additive; defaults to `false` for older payloads (client inspector button
    /// lands in a later card).
    #[serde(default)]
    pub boosted: bool,
    /// Player-maintained labor preferences. These bias eligible job/station
    /// matching without making the cat immune to emergency work or other gates.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preferred_labors: Vec<Labor>,
    /// Whether this cat is currently expecting a litter (mirrors `entities::Cat::
    /// is_pregnant`). Lets the census/inspector show an "expecting" state. Additive;
    /// defaults to `false` for older payloads.
    #[serde(default)]
    pub pregnant: bool,
    /// Stable permanent-bed status. Probationary cats are physically present and may
    /// work, but remain unhoused until a vacancy is allocated before their deadline.
    #[serde(default)]
    pub housing_status: CatHousingStatus,
    /// Physical prosperity-migration journey. `resident` is the backward-
    /// compatible default for snapshots written before cats walked through gates.
    #[serde(default)]
    pub migration_status: CatMigrationStatus,
    /// Remaining in-game minutes before an unhoused probationary arrival leaves.
    /// `None` for permanent residents and legacy snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probation_remaining_game_minutes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Labor {
    Hunt,
    Fishing,
    Build,
    Ritual,
    Fight,
    Train,
    Quarry,
    Woodcut,
    Forage,
    FetchWater,
    Mill,
    Process,
    Craft,
    Textile,
    Metalwork,
    Farm,
    Haul,
    Research,
    Scout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CatHousingStatus {
    #[default]
    Housed,
    Probationary,
    Unhoused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CatMigrationStatus {
    #[default]
    Resident,
    Arriving,
    Probationary,
    Departing,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapPosition {
    pub map: MapName,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapName {
    Colony,
    World,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatActivity {
    Idle,
    Traveling,
    Working,
    Returning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Specialization {
    Hunter,
    Architect,
    Ritualist,
    Warrior,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Carrying {
    pub kind: CarryingKind,
    pub amount: f64,
    pub job_ended_at: i64,
    /// Stable identities physically held by this cat. Empty for ordinary
    /// resource cargo and for legacy snapshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquipmentLoadoutSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weapon_item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor_item_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarryingKind {
    Food,
    Fish,
    Blessings,
    Materials,
    Stone,
    Refined,
    Logs,
    Lumber,
    Planks,
    Blocks,
    Tools,
    Water,
    Catnip,
    Grain,
    Flour,
    Preserves,
    Medicine,
    Brew,
    Herbs,
    Hide,
    Leather,
    Fibre,
    Cloth,
    Bone,
    Ore,
    Gem,
    Clay,
    Sand,
    Metal,
    Weapons,
    Armor,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatNeeds {
    pub hunger: f64,
    pub thirst: f64,
    pub rest: f64,
    pub health: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleXp {
    pub hunter: f64,
    pub architect: f64,
    pub ritualist: f64,
    #[serde(default)]
    pub warrior: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatStats {
    pub leadership: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub kind: JobKind,
    pub status: JobStatus,
    pub ends_at: i64,
    pub started_at: i64,
    pub click_time_reduced_sec: f64,
    pub assigned_cat_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficerRole {
    Steward,
    Accountant,
    Forester,
    Farmer,
    Captain,
    Loremaster,
    ClothLeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    SupplyFood,
    SupplyWater,
    LeaderPlanHunt,
    HuntExpedition,
    LeaderPlanHouse,
    BuildHouse,
    /// Physical authored-road work: fetch one unit of Supplies per tile, walk the
    /// ordered route, and pave each tile only after on-site labor.
    BuildRoad,
    Ritual,
    Quarry,
    /// Forest-site gathering job: fells and carries raw logs. This is the sole
    /// authoritative input source for the sawmill chain.
    GatherLogs,
    /// Physical Forester work that converts one felled stump/root stock into a
    /// persisted growing sapling. The player may order it without an officer.
    ReplantTree,
    /// Physical shoreline food gathering into a designated fishing spot.
    Fish,
    /// Bounded foraging shift that gathers fibre for the textile chain.
    ForageFibre,
    Explore,
    FetchWater,
    TrainWarrior,
    ExpandVillage,
    /// P12.6: haul-then-ritual offering — surplus materials converted to blessings
    /// at the shrine. This is only the physical stockpile-to-shrine delivery stage.
    CarryOffering,
    /// P12.6: the shrine ritual that consumes physically delivered offering goods and
    /// produces blessings. Never exists before its matching carry reaches the shrine.
    PerformOffering,
    /// P16 gather spots: a mover walks to a gather spot, picks up its contents, and
    /// hauls them back to a village stockpile/shrine — see [`ClientAction::DesignateGatherSpot`].
    HaulGatherSpot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Active,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeSnapshot {
    pub key: UpgradeKey,
    pub level: u32,
    pub max_level: u32,
    pub base_cost: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpgradeKey {
    ClickPower,
    SupplySpeed,
    HuntMastery,
    BuildMastery,
    RitualMastery,
    Resilience,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventSnapshot {
    pub message: String,
    pub timestamp: i64,
    /// Stable lowercase `snake_case` event category (e.g. `"birth"`,
    /// `"death_raid"`, `"tithe"`) — see `cat_sim::world_tick::EventKind::wire_kind`.
    /// Classify on this instead of pattern-matching `message` text. Defaults to
    /// an empty string when deserializing an older payload that predates this
    /// field.
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HousingSnapshot {
    pub population: u32,
    pub capacity: u32,
    pub pressure: f64,
    pub village_level: u32,
    /// Permanent residents with a deterministic bed allocation.
    #[serde(default)]
    pub housed: u32,
    /// Living arrivals still inside their 36-game-hour housing probation.
    #[serde(default)]
    pub probationary: u32,
    /// All living cats without a permanent bed (including probationers).
    #[serde(default)]
    pub unhoused: u32,
    /// Lifetime number of probationary cats who physically left without housing.
    #[serde(default)]
    pub departures: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchSnapshot {
    pub owned_node_ids: Vec<String>,
    pub research_points: f64,
    pub researcher_count: u32,
    pub blessings: f64,
    pub next_target: Option<ResearchTarget>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchTarget {
    pub id: String,
    pub name: String,
    pub cost: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElectionSnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub ends_at: i64,
    pub tally: BTreeMap<String, u32>,
    pub total_ballots: u32,
    pub candidates: Vec<ElectionCandidate>,
}

/// Server-derived term timing for the automatic leadership-election lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElectionScheduleSnapshot {
    /// Close time of the election which began the current term, when one exists.
    pub term_started_at: Option<i64>,
    /// Exact simulation boundary at which the next scheduled election becomes due.
    pub next_election_at: i64,
    /// Effective term length after applying the authoritative time scale.
    pub term_length_ms: i64,
    /// Remaining time calculated at the enclosing [`WorldSnapshot::now`].
    pub remaining_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElectionCandidate {
    #[serde(alias = "_id")]
    pub id: String,
    pub name: String,
    pub leadership: f64,
    pub specialization: Option<Specialization>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteKickSnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub ends_at: i64,
    pub target_cat_id: String,
    pub target_name: String,
    pub signatures: u32,
    pub needed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneSnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub kind: ZoneKind,
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZoneKind {
    Avoid,
    Gather,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatSnapshot {
    pub pressure: f64,
    pub band: ThreatBand,
    pub raid_active: bool,
    pub warriors: u32,
    pub weapons: f64,
    pub armor: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreatBand {
    Calm,
    Rising,
    Imminent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaiderSnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub position: TilePoint,
    pub hp: f64,
    pub strength: f64,
    pub status: RaiderStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RaiderStatus {
    Advancing,
    Engaging,
    Retreating,
    Dead,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuildingSnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    #[serde(rename = "type")]
    pub building_type: BuildingType,
    pub level: u32,
    pub construction_progress: f64,
    pub world_position: TilePoint,
    pub position: TilePoint,
    /// Tile footprint (`width` x `height`) the building occupies, anchored at
    /// `position`/`world_position` (its north-west corner). Derived from the building
    /// type in the sim; defaults to 1x1 for back-compat with older snapshots.
    #[serde(default = "default_footprint")]
    pub footprint: FootprintSize,
    /// Cats currently assigned/working this building. The sim models a single worker
    /// slot per production building today (0 or 1), never more than `staff_cap`.
    /// Additive; defaults to 0 for pre-staffing snapshots.
    #[serde(default)]
    pub staff_count: u32,
    /// Max worker occupancy for this building type (its worker slots). 1 for the
    /// building types the sim staffs with an assigned cat (workshop and the raw-material
    /// benches, smithy); 0 for building types with no worker-slot concept, including
    /// fields, which yield passively with no assigned worker at all. Additive; defaults
    /// to 0 for pre-staffing snapshots.
    #[serde(default)]
    pub staff_cap: u32,
    /// Per-cat durable station state. Slot zero mirrors the legacy building-level
    /// queue/progress fields; researched slots follow in deterministic assignment order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub work_slots: Vec<ProductionWorkSlotSnapshot>,
    /// 0.0..=1.0 through the current production cycle, for buildings that craft on a
    /// timer (workshop/benches/smithy). 0.0 for buildings with no active cycle,
    /// including fields (which add yield continuously rather than completing cycles)
    /// and non-producing buildings. Additive; defaults to 0.0 for older snapshots.
    #[serde(default)]
    pub production_progress: f64,
    /// Short, stable, lowercase label of what this building type makes (e.g. "plank",
    /// "refined", "weapon", "armor"), or `None` if it doesn't produce a resource.
    /// Additive; empty/absent for pre-production-label snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_output: Option<String>,
    /// Resource units physically in flight toward this building right now — the live sum
    /// of carried cargo whose haul destination resolves to this building's tile (see
    /// `cat_sim::world_tick::building_inbound_haul`). Physical processor input carriers target
    /// their station work point; ordinary cargo targets a stockpile.
    /// Additive; defaults to 0.0 for older snapshots.
    #[serde(default)]
    pub inbound_haul: f64,
    /// Resource units physically departing this building in carried cargo. Physical
    /// processor outputs remain uncredited until this cargo reaches storage.
    #[serde(default)]
    pub outbound_haul: f64,
    /// True station-local inputs already delivered to this building. These are not a
    /// projection of colony totals.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_inventory: Vec<ResourceStackSnapshot>,
    /// Per-resource capacity of the station's physical input working reserve.
    /// Zero for buildings without a processor-local store.
    #[serde(default)]
    pub input_capacity: f64,
    /// Finished goods still at the station awaiting a physical outbound haul. They are
    /// deliberately absent from colony aggregate resources until deposited.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_inventory: Vec<ResourceStackSnapshot>,
    /// Per-resource capacity of the station's physical finished-output reserve.
    /// Zero for buildings without a processor-local store.
    #[serde(default)]
    pub output_capacity: f64,
    /// Durable recipes in deterministic execution order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub production_queue: Vec<ProductionQueueEntrySnapshot>,
    /// Recipes the authoritative descriptor accepts for this station. Queue controls
    /// must use this list rather than duplicating station knowledge in the client.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_recipes: Vec<String>,
    /// Catalog study required by the queued implemented recipe. `None` for
    /// legacy-grandfathered snapshots and non-production buildings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_recipe_study: Option<ResearchTarget>,
    /// Pausing prevents a new physical recipe cycle while still allowing already-
    /// finished output to be hauled away.
    #[serde(default, skip_serializing_if = "is_false")]
    pub production_paused: bool,
    /// Stable machine-readable reason that the queue cannot currently advance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_block_reason: Option<String>,
    /// Human-readable physical worker/cargo travel state for the inspector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_travel: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inbound_cargo: Vec<ResourceStackSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outbound_cargo: Vec<ResourceStackSnapshot>,
    /// Pinned construction bill owned by an incomplete scaffold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub construction_required: Vec<ResourceStackSnapshot>,
    /// Units that physically reached this scaffold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub construction_delivered: Vec<ResourceStackSnapshot>,
    /// Units removed from a source pile and currently carried toward this scaffold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub construction_in_transit: Vec<ResourceStackSnapshot>,
    /// Stable physical logistics state for the scaffold inspector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub construction_block_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionWorkSlotSnapshot {
    pub cat_id: String,
    #[serde(default)]
    pub production_progress: f64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub production_queue: Vec<ProductionQueueEntrySnapshot>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub production_paused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automated_by: Option<OfficerRole>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceStackSnapshot {
    pub kind: ResourceKind,
    pub amount: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionQueueEntrySnapshot {
    pub recipe_id: String,
    pub repeat: bool,
}

impl<'de> Deserialize<'de> for ProductionQueueEntrySnapshot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Legacy(String),
            Entry {
                #[serde(rename = "recipeId")]
                recipe_id: String,
                repeat: bool,
            },
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Legacy(recipe_id) => Self {
                recipe_id,
                repeat: true,
            },
            Wire::Entry { recipe_id, repeat } => Self { recipe_id, repeat },
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueMoveDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ProductionQueueEdit {
    Add {
        recipe_id: String,
        repeat: bool,
    },
    Remove {
        index: usize,
    },
    Move {
        index: usize,
        direction: QueueMoveDirection,
    },
    SetRepeat {
        index: usize,
        repeat: bool,
    },
    SetPaused {
        paused: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CropKind {
    Catnip,
    Grain,
    Herb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmStage {
    Soil,
    Sprout,
    Growing,
    Mature,
    Flowering,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FarmWorkPhase {
    #[default]
    WaitingForWorker,
    Traveling,
    Planting,
    Tending,
    Harvesting,
    Hauling,
    OutputBlocked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FarmSnapshot {
    pub id: String,
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub crop: CropKind,
    pub planted_at: i64,
    pub stage: FarmStage,
    /// Fertility-scaled hours worked in the current cycle, for progress UI.
    #[serde(default)]
    pub growth_hours: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    #[serde(default)]
    pub work_phase: FarmWorkPhase,
    /// Current maintained crop model has no seed item; this remains empty until a
    /// real recipe introduces one rather than fabricating an input.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_inventory: Vec<ResourceStackSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_inventory: Vec<ResourceStackSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_travel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<String>,
}

/// A building's tile footprint size, in tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FootprintSize {
    pub width: i32,
    pub height: i32,
}

fn default_footprint() -> FootprintSize {
    FootprintSize {
        width: 1,
        height: 1,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BuildingType {
    #[default]
    Den,
    FoodStorage,
    WaterBowl,
    Beds,
    HerbGarden,
    Nursery,
    ElderCorner,
    Walls,
    MouseFarm,
    Shrine,
    Workshop,
    Field,
    ResearchHut,
    School,
    Smithy,
    Barracks,
    /// Accountant office prerequisite and staffed stock-ledger workplace.
    AccountingTent,
    WoodCutter,
    StonePrep,
    Woodworking,
    Clothier,
    Tannery,
    /// P17/P19 ore→metal chain: refines mountain ore into metal bars. The client renders
    /// this as a distinct open furnace-and-basin station.
    Smelter,
    Mill,
    Sawmill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TilePoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportMode {
    Rail,
    Shipping,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransportSnapshot {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub track_tiles: Vec<TilePoint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub docks: Vec<TransportDockSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vehicles: Vec<TransportVehicleSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub routes: Vec<TransportRouteSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportDockSnapshot {
    pub id: String,
    pub land_tile: TilePoint,
    pub water_tile: TilePoint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportVehicleSnapshot {
    pub id: String,
    pub mode: TransportMode,
    pub position: TilePoint,
    pub crew_cat_id: Option<String>,
    pub cargo: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportRouteSnapshot {
    pub id: String,
    pub mode: TransportMode,
    pub resource: ResourceKind,
    pub amount: f64,
    pub phase: String,
    pub repeat: bool,
}

/// A resource the player or colony leader can ask a scout to locate.
///
/// Kept deliberately separate from [`ResourceKind`]: scouting targets terrain
/// knowledge (a tree, forage, spring, or workable stone), not an amount already
/// held in a stockpile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScoutResource {
    Wood,
    Food,
    Water,
    Stone,
}

/// The purpose of a scout excursion. Both mission shapes follow deterministic,
/// knowledge-blind wander legs. General exploration returns after surveying enough
/// new ground; a resource mission returns when it physically observes the requested
/// terrain resource or exhausts its bounded search.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "resource", rename_all = "camelCase")]
pub enum ScoutMission {
    Explore,
    Resource(ScoutResource),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatePlacement {
    pub x: i32,
    pub y: i32,
    pub side: GateSide,
}

/// One physically present palisade edge. `under_construction` distinguishes the
/// newly completed outer ring from the retained old enclosure while expansion is live.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WallSegment {
    pub x: i32,
    pub y: i32,
    pub side: GateSide,
    #[serde(default)]
    pub under_construction: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateSide {
    N,
    E,
    S,
    W,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ClientAction {
    Ensure,
    Presence {
        session_id: String,
        nickname: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sig: Option<String>,
    },
    RequestJob {
        session_id: String,
        nickname: String,
        sig: String,
        kind: JobKind,
    },
    DispatchScout {
        session_id: String,
        nickname: String,
        sig: String,
        mission: ScoutMission,
    },
    Boost {
        session_id: String,
        nickname: String,
        sig: String,
        job_id: String,
    },
    PurchaseUpgrade {
        session_id: String,
        nickname: String,
        sig: String,
        key: UpgradeKey,
    },
    CastVote {
        session_id: String,
        nickname: String,
        sig: String,
        election_id: String,
        cat_id: String,
    },
    RequestVoteKick {
        session_id: String,
        nickname: String,
        sig: String,
    },
    CreateZone {
        session_id: String,
        nickname: String,
        sig: String,
        kind: ZoneKind,
        a: TilePoint,
        b: TilePoint,
        duration_ms: u64,
    },
    RemoveZone {
        session_id: String,
        nickname: String,
        sig: String,
        zone_id: String,
    },
    PlanBuilding {
        session_id: String,
        nickname: String,
        sig: String,
        #[serde(rename = "type")]
        building_type: BuildingType,
        /// North-west footprint anchor selected on the world map. Older clients did
        /// not send this field; `None` retains deterministic automatic placement.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        site: Option<TilePoint>,
    },
    UnlockNode {
        session_id: String,
        nickname: String,
        sig: String,
        node_id: String,
    },
    /// Spend cat-generated research points on a technology node. Blessing purchases
    /// remain the separate `UnlockNode` action.
    ResearchNode {
        session_id: String,
        nickname: String,
        sig: String,
        node_id: String,
    },
    /// Convert a safe food/refined surplus into shrine blessings immediately.
    OfferTithe {
        session_id: String,
        nickname: String,
        sig: String,
    },
    /// Dispatch a cat to carry a material offering to the shrine.
    OfferMaterials {
        session_id: String,
        nickname: String,
        sig: String,
    },
    /// Dispatch one manual gather-spot haul. `cat_id = None` selects the best
    /// currently available carrier deterministically.
    HaulGatherSpot {
        session_id: String,
        nickname: String,
        sig: String,
        stockpile_id: String,
        cat_id: Option<String>,
    },
    AssignWorker {
        session_id: String,
        nickname: String,
        sig: String,
        cat_id: String,
        building_id: Option<String>,
    },
    TrainWarrior {
        session_id: String,
        nickname: String,
        sig: String,
        cat_id: Option<String>,
    },
    DefendRaid {
        session_id: String,
        nickname: String,
        sig: String,
    },
    BuildRoad {
        session_id: String,
        nickname: String,
        sig: String,
        a: TilePoint,
        b: TilePoint,
    },
    /// Designate a cardinal, revealed land alignment. A living builder must
    /// carry exact Metal to every tile before it becomes track.
    DesignateRail {
        session_id: String,
        nickname: String,
        sig: String,
        a: TilePoint,
        b: TilePoint,
        cat_id: String,
    },
    /// Designate one land/water edge as a staffed physical dock project.
    BuildDock {
        session_id: String,
        nickname: String,
        sig: String,
        land: TilePoint,
        water: TilePoint,
        cat_id: String,
    },
    /// Construct rolling stock or a vessel at existing infrastructure.
    BuildTransportVehicle {
        session_id: String,
        nickname: String,
        sig: String,
        mode: TransportMode,
        home: TilePoint,
        cat_id: String,
    },
    /// Author one finite stockpile-to-stockpile route over an explicit physical path.
    CreateTransportRoute {
        session_id: String,
        nickname: String,
        sig: String,
        mode: TransportMode,
        source_stockpile_id: String,
        destination_stockpile_id: String,
        resource: ResourceKind,
        amount: f64,
        path: Vec<TilePoint>,
        cat_id: String,
        repeat: bool,
    },
    CancelTransportRoute {
        session_id: String,
        nickname: String,
        sig: String,
        route_id: String,
    },
    SetTestAcceleration {
        preset: AccelerationPreset,
    },
    AdvanceTime {
        seconds: u64,
    },
    SetTestRngSeed {
        seed: Option<u32>,
    },
    FoundVillage {
        name: String,
        session_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sig: Option<String>,
    },
    JoinVillage {
        colony_id: String,
        session_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sig: Option<String>,
    },
    /// Propose an atomic resource barter to a mutually discovered village.
    OfferVillageTrade {
        session_id: String,
        nickname: String,
        sig: String,
        target_colony_id: String,
        offered_kind: ResourceKind,
        offered_amount: f64,
        requested_kind: ResourceKind,
        requested_amount: f64,
    },
    /// Accept an offer addressed to the currently selected village.
    AcceptVillageTrade {
        session_id: String,
        nickname: String,
        sig: String,
        offer_id: String,
    },
    /// Withdraw an offer authored by the currently selected village.
    CancelVillageTrade {
        session_id: String,
        nickname: String,
        sig: String,
        offer_id: String,
    },
    AssignOfficer {
        session_id: String,
        nickname: String,
        sig: String,
        role: OfficerRole,
        cat_id: String,
    },
    UnassignOfficer {
        session_id: String,
        nickname: String,
        sig: String,
        role: OfficerRole,
    },
    DesignateFarm {
        session_id: String,
        nickname: String,
        sig: String,
        a: TilePoint,
        b: TilePoint,
        crop: CropKind,
    },
    ClearFarm {
        session_id: String,
        nickname: String,
        sig: String,
        plot_id: String,
    },
    DesignateStockpile {
        session_id: String,
        nickname: String,
        sig: String,
        a: TilePoint,
        b: TilePoint,
        accepts: Vec<ResourceKind>,
    },
    RemoveStockpile {
        session_id: String,
        nickname: String,
        sig: String,
        stockpile_id: String,
    },
    /// Designate a **gather spot** (P16): a temporary, single-resource drop point that
    /// may be placed outside the claimed village, unlike a general [`Self::DesignateStockpile`].
    /// A nearby gatherer job (hunt/quarry/fetch-water) drops its yield here instead of
    /// walking all the way to the shrine; a separate mover then hauls the contents on to
    /// a village stockpile/shrine. Expires automatically (TTL) or when removed.
    DesignateGatherSpot {
        session_id: String,
        nickname: String,
        sig: String,
        a: TilePoint,
        b: TilePoint,
        kind: ResourceKind,
    },
    /// Designate one exact, revealed shoreline tile as a fishing workplace and
    /// finite food drop point. Clicking adjacent water is also accepted and
    /// resolves to a deterministic walkable bank tile.
    DesignateFishingSpot {
        session_id: String,
        nickname: String,
        sig: String,
        at: TilePoint,
    },
    /// Remove a gather spot by its underlying stockpile id before its TTL. Its
    /// remaining contents fold back into the shrine reservoir via reconcile, exactly
    /// like [`Self::RemoveStockpile`].
    RemoveGatherSpot {
        session_id: String,
        nickname: String,
        sig: String,
        stockpile_id: String,
    },
    /// Sell `count` of a crafted-item stack (`kind`/`material`/`quality`, matching
    /// [`ItemStackSnapshot`]'s wire fields) to the visiting trader for coin (P19 slice 3).
    /// Only valid while a trader is present and `Trading`.
    SellGoods {
        session_id: String,
        nickname: String,
        sig: String,
        kind: String,
        material: String,
        quality: u8,
        count: u32,
    },
    /// Repair one finite item at its appropriate completed, staffed workshop.
    RepairItem {
        session_id: String,
        nickname: String,
        sig: String,
        item_id: String,
    },
    /// Equip one exact functional item on one living cat. Item kind determines
    /// its single Tool/Weapon/Armor slot; the sim rejects foreign, carried,
    /// broken, already-equipped, or otherwise unavailable identities.
    EquipItem {
        session_id: String,
        nickname: String,
        sig: String,
        cat_id: String,
        item_id: String,
    },
    /// Return one exact equipped item to physical village storage. The bearer
    /// and item identity are both required so another cat's loadout cannot be
    /// mutated by an ambiguous slot-only request.
    UnequipItem {
        session_id: String,
        nickname: String,
        sig: String,
        cat_id: String,
        item_id: String,
    },
    /// Buy `amount` of `resource` from the visiting trader with coin (P19 slice 3). Only
    /// valid while a trader is present, `Trading`, and stocks that resource kind.
    BuyResource {
        session_id: String,
        nickname: String,
        sig: String,
        resource: ResourceKind,
        amount: f64,
    },
    /// Set/clear a cat's player priority flag (P15 "cat booster"). A persistent bias
    /// toward this cat in the leader director's job/role matcher — not a timed effect.
    /// `boosted` makes this a toggle so the client can set or clear from one action.
    BoostCat {
        session_id: String,
        nickname: String,
        sig: String,
        cat_id: String,
        boosted: bool,
    },
    /// Add or remove one maintained labor preference for a living cat.
    SetCatLaborPreference {
        session_id: String,
        nickname: String,
        sig: String,
        cat_id: String,
        labor: Labor,
        enabled: bool,
    },
    /// Edit the durable recipe queue of one exact production station.
    EditProductionQueue {
        session_id: String,
        nickname: String,
        sig: String,
        building_id: String,
        edit: ProductionQueueEdit,
    },
    /// Edit one exact cat-owned station slot. Additive counterpart to the legacy
    /// building-level action, which continues to address slot zero.
    EditProductionWorkSlot {
        session_id: String,
        nickname: String,
        sig: String,
        building_id: String,
        cat_id: String,
        edit: ProductionQueueEdit,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccelerationPreset {
    Off,
    Fast,
    Turbo,
    Hyper,
    Ludicrous,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Village created or selected by an idempotent village action.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colony_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_resource_payload_defaults_raw_stone_and_bone_without_aliasing_materials() {
        let restored: ResourceAmounts = serde_json::from_value(json!({
            "food": 1.0, "water": 2.0, "herbs": 3.0, "materials": 19.0,
            "refined": 0.0, "weapons": 0.0, "armor": 0.0, "blessings": 0.0
        }))
        .expect("legacy resource payload");
        assert_eq!(restored.materials, 19.0);
        assert_eq!(restored.stone, 0.0);
        assert_eq!(restored.bone, 0.0);
        assert!(ResourceKind::Stone.is_physical_stockpile_good());
        assert!(ResourceKind::Bone.is_physical_stockpile_good());

        let capacities: ResourceCapacities = serde_json::from_value(json!({
            "food": 200.0, "water": 200.0, "herbs": 100.0,
            "materials": 100.0, "refined": 100.0
        }))
        .expect("legacy capacity payload");
        assert_eq!(capacities.bone, 0.0);
        assert_eq!(capacities.gem, 0.0);
        assert_eq!(capacities.clay, 0.0);
        assert_eq!(capacities.sand, 0.0);
    }
    use serde_json::json;

    #[test]
    fn physical_stockpile_classification_is_exhaustive_for_all_resource_kinds() {
        let expected_all = [
            ResourceKind::Food,
            ResourceKind::Fish,
            ResourceKind::Water,
            ResourceKind::Herbs,
            ResourceKind::Catnip,
            ResourceKind::Grain,
            ResourceKind::Flour,
            ResourceKind::Preserves,
            ResourceKind::Medicine,
            ResourceKind::Brew,
            ResourceKind::Materials,
            ResourceKind::Stone,
            ResourceKind::Refined,
            ResourceKind::Weapons,
            ResourceKind::Armor,
            ResourceKind::Logs,
            ResourceKind::Lumber,
            ResourceKind::Planks,
            ResourceKind::Blocks,
            ResourceKind::Tools,
            ResourceKind::Fibre,
            ResourceKind::Hide,
            ResourceKind::Bone,
            ResourceKind::Cloth,
            ResourceKind::Leather,
            ResourceKind::Ore,
            ResourceKind::Gem,
            ResourceKind::Clay,
            ResourceKind::Sand,
            ResourceKind::Metal,
            ResourceKind::Blessings,
        ];
        assert_eq!(ResourceKind::ALL, expected_all);

        let physical = ResourceKind::physical_stockpile_goods().collect::<Vec<_>>();
        assert_eq!(physical.len(), 30);
        for &kind in ResourceKind::ALL {
            assert_eq!(
                physical.contains(&kind),
                kind.is_physical_stockpile_good(),
                "classification drifted for {kind:?}"
            );
        }
        assert_eq!(
            ResourceKind::ALL
                .iter()
                .copied()
                .filter(|kind| !kind.is_physical_stockpile_good())
                .collect::<Vec<_>>(),
            vec![ResourceKind::Blessings],
            "Blessings alone are nonphysical divine favor, not hauled inventory"
        );
    }

    #[test]
    fn client_action_round_trips_with_route_field_names() {
        let action = ClientAction::RequestJob {
            session_id: "session_1".to_string(),
            nickname: "Guest Cat".to_string(),
            sig: "signed".to_string(),
            kind: JobKind::SupplyFood,
        };

        let encoded = serde_json::to_value(&action).expect("serialize action");
        assert_eq!(
            encoded,
            json!({
                "action": "requestJob",
                "sessionId": "session_1",
                "nickname": "Guest Cat",
                "sig": "signed",
                "kind": "supply_food"
            })
        );

        let decoded: ClientAction = serde_json::from_value(encoded).expect("deserialize action");
        assert_eq!(decoded, action);
    }

    #[test]
    fn exact_labor_and_station_queue_actions_round_trip_typed_payloads() {
        let actions = [
            ClientAction::SetCatLaborPreference {
                session_id: "session_1".to_owned(),
                nickname: "Player".to_owned(),
                sig: "signed".to_owned(),
                cat_id: "cat_7".to_owned(),
                labor: Labor::Process,
                enabled: true,
            },
            ClientAction::EditProductionQueue {
                session_id: "session_1".to_owned(),
                nickname: "Player".to_owned(),
                sig: "signed".to_owned(),
                building_id: "sawmill_2".to_owned(),
                edit: ProductionQueueEdit::Move {
                    index: 3,
                    direction: QueueMoveDirection::Up,
                },
            },
            ClientAction::EditProductionWorkSlot {
                session_id: "session_1".to_owned(),
                nickname: "Player".to_owned(),
                sig: "signed".to_owned(),
                building_id: "sawmill_2".to_owned(),
                cat_id: "cat_8".to_owned(),
                edit: ProductionQueueEdit::SetPaused { paused: true },
            },
        ];
        for action in actions {
            let encoded = serde_json::to_value(&action).unwrap();
            let decoded: ClientAction = serde_json::from_value(encoded).unwrap();
            assert_eq!(decoded, action);
        }
    }

    #[test]
    fn legacy_station_queue_strings_deserialize_as_repeating_entries() {
        let entry: ProductionQueueEntrySnapshot =
            serde_json::from_value(json!("logs_to_lumber")).unwrap();
        assert_eq!(entry.recipe_id, "logs_to_lumber");
        assert!(entry.repeat);
    }

    #[test]
    fn scout_actions_are_typed_and_round_trip_for_both_mission_shapes() {
        let cases = [
            (
                ScoutMission::Explore,
                json!({
                    "action": "dispatchScout",
                    "sessionId": "session_1",
                    "nickname": "Guest Cat",
                    "sig": "signed",
                    "mission": { "kind": "explore" }
                }),
            ),
            (
                ScoutMission::Resource(ScoutResource::Wood),
                json!({
                    "action": "dispatchScout",
                    "sessionId": "session_1",
                    "nickname": "Guest Cat",
                    "sig": "signed",
                    "mission": { "kind": "resource", "resource": "wood" }
                }),
            ),
        ];

        for (mission, expected) in cases {
            let action = ClientAction::DispatchScout {
                session_id: "session_1".to_owned(),
                nickname: "Guest Cat".to_owned(),
                sig: "signed".to_owned(),
                mission,
            };
            let encoded = serde_json::to_value(&action).expect("serialize scout action");
            assert_eq!(encoded, expected);
            assert_eq!(
                serde_json::from_value::<ClientAction>(encoded).expect("deserialize scout action"),
                action
            );
        }
    }

    #[test]
    fn village_trade_actions_round_trip_with_exact_signed_fields() {
        let actions = [
            ClientAction::OfferVillageTrade {
                session_id: "session_1".to_owned(),
                nickname: "Guest Cat".to_owned(),
                sig: "signed".to_owned(),
                target_colony_id: "reed-rest".to_owned(),
                offered_kind: ResourceKind::Food,
                offered_amount: 8.0,
                requested_kind: ResourceKind::Materials,
                requested_amount: 4.0,
            },
            ClientAction::AcceptVillageTrade {
                session_id: "session_1".to_owned(),
                nickname: "Guest Cat".to_owned(),
                sig: "signed".to_owned(),
                offer_id: "trade-1".to_owned(),
            },
            ClientAction::CancelVillageTrade {
                session_id: "session_1".to_owned(),
                nickname: "Guest Cat".to_owned(),
                sig: "signed".to_owned(),
                offer_id: "trade-1".to_owned(),
            },
        ];
        for action in actions {
            let encoded = serde_json::to_value(&action).expect("serialize village trade");
            assert_eq!(
                serde_json::from_value::<ClientAction>(encoded).expect("deserialize village trade"),
                action
            );
        }
    }

    #[test]
    fn sell_goods_action_round_trips_with_route_field_names() {
        let action = ClientAction::SellGoods {
            session_id: "session_1".to_string(),
            nickname: "Guest Cat".to_string(),
            sig: "signed".to_string(),
            kind: "mug".to_string(),
            material: "wood".to_string(),
            quality: 2,
            count: 5,
        };

        let encoded = serde_json::to_value(&action).expect("serialize action");
        assert_eq!(
            encoded,
            json!({
                "action": "sellGoods",
                "sessionId": "session_1",
                "nickname": "Guest Cat",
                "sig": "signed",
                "kind": "mug",
                "material": "wood",
                "quality": 2,
                "count": 5
            })
        );

        let decoded: ClientAction = serde_json::from_value(encoded).expect("deserialize action");
        assert_eq!(decoded, action);
    }

    #[test]
    fn repair_item_action_round_trips_with_stable_item_identity() {
        let action = ClientAction::RepairItem {
            session_id: "session_1".to_string(),
            nickname: "Guest Cat".to_string(),
            sig: "signed".to_string(),
            item_id: "item-0000000000000042".to_string(),
        };

        let encoded = serde_json::to_value(&action).expect("serialize action");
        assert_eq!(
            encoded,
            json!({
                "action": "repairItem",
                "sessionId": "session_1",
                "nickname": "Guest Cat",
                "sig": "signed",
                "itemId": "item-0000000000000042"
            })
        );
        assert_eq!(
            serde_json::from_value::<ClientAction>(encoded).expect("deserialize action"),
            action
        );
    }

    #[test]
    fn finite_equipment_actions_bind_exact_cat_and_item_identities() {
        let actions = [
            ClientAction::EquipItem {
                session_id: "session_1".to_owned(),
                nickname: "Guest Cat".to_owned(),
                sig: "signed".to_owned(),
                cat_id: "cat-7".to_owned(),
                item_id: "item-0000000000000042".to_owned(),
            },
            ClientAction::UnequipItem {
                session_id: "session_1".to_owned(),
                nickname: "Guest Cat".to_owned(),
                sig: "signed".to_owned(),
                cat_id: "cat-7".to_owned(),
                item_id: "item-0000000000000042".to_owned(),
            },
        ];

        for (action, expected_name) in actions.into_iter().zip(["equipItem", "unequipItem"]) {
            let encoded = serde_json::to_value(&action).expect("serialize equipment action");
            assert_eq!(encoded["action"], json!(expected_name));
            assert_eq!(encoded["catId"], json!("cat-7"));
            assert_eq!(encoded["itemId"], json!("item-0000000000000042"));
            assert_eq!(
                serde_json::from_value::<ClientAction>(encoded)
                    .expect("deserialize equipment action"),
                action
            );
        }
    }

    #[test]
    fn buy_resource_action_round_trips_with_route_field_names() {
        let action = ClientAction::BuyResource {
            session_id: "session_1".to_string(),
            nickname: "Guest Cat".to_string(),
            sig: "signed".to_string(),
            resource: ResourceKind::Food,
            amount: 12.5,
        };

        let encoded = serde_json::to_value(&action).expect("serialize action");
        assert_eq!(
            encoded,
            json!({
                "action": "buyResource",
                "sessionId": "session_1",
                "nickname": "Guest Cat",
                "sig": "signed",
                "resource": "food",
                "amount": 12.5
            })
        );

        let decoded: ClientAction = serde_json::from_value(encoded).expect("deserialize action");
        assert_eq!(decoded, action);
    }

    #[test]
    fn trader_snapshot_round_trips_and_coin_defaults_to_zero_when_absent() {
        let snapshot = TraderSnapshot {
            id: "trader-1".to_string(),
            position: TilePoint { x: 6, y: 12 },
            state: TraderVisitState::Trading,
            destination: Some(TilePoint { x: 7, y: 8 }),
            route_exterior: Some(TilePoint { x: 7, y: 20 }),
            visit_number: 3,
            arrived_at: Some(100),
            visit_ends_at: Some(200),
            coin: 99.0,
            cargo_weight_grams: 25_000.0,
            cargo_capacity_grams: 100_000.0,
            cargo_items: Vec::new(),
            stock: vec![TraderStockSnapshot {
                resource: ResourceKind::Food,
                available: 4.0,
                sold_out: false,
            }],
            buy_offers: vec![TraderBuyOffer {
                kind: "mug".to_string(),
                material: "wood".to_string(),
                quality: 1,
                available: 3,
                unit_price: 2.4,
                unit_weight_grams: 420,
                blocked_reason: None,
            }],
            sell_offers: vec![TraderSellOffer {
                resource: ResourceKind::Food,
                unit_price: 1.5,
                available: 4.0,
                sold_out: false,
                blocked_reason: None,
            }],
        };

        let encoded = serde_json::to_value(&snapshot).expect("serialize trader snapshot");
        let decoded: TraderSnapshot =
            serde_json::from_value(encoded).expect("deserialize trader snapshot");
        assert_eq!(decoded, snapshot);

        // `coin`/`trader` are additive: a pre-P19-slice-3 payload lacking both fields
        // must still deserialize, with `coin` defaulting to 0.0 and `trader` to `None`.
        let legacy_json = json!({
            "id": "colony_1",
            "name": "Global Colony",
            "status": "thriving",
            "resources": {
                "food": 0.0, "water": 0.0, "herbs": 0.0, "materials": 0.0, "refined": 0.0,
                "weapons": 0.0, "armor": 0.0, "planks": 0.0, "blocks": 0.0, "tools": 0.0,
                "blessings": 0.0
            },
            "storage": {
                "capacities": {
                    "food": 0.0, "water": 0.0, "herbs": 0.0, "materials": 0.0, "refined": 0.0,
                    "weapons": 0.0, "armor": 0.0, "planks": 0.0, "blocks": 0.0, "tools": 0.0
                },
                "titheRates": { "food": 20.0, "refined": 5.0 }
            },
            "leader": null,
            "cats": [],
            "jobs": [],
            "upgrades": [],
            "events": [],
            "housing": { "population": 0, "capacity": 0, "pressure": 0.0, "villageLevel": 0 },
            "research": {
                "ownedNodeIds": [], "researchPoints": 0.0, "researcherCount": 0, "blessings": 0.0
            },
            "election": null,
            "voteKick": null,
            "zones": [],
            "threat": {
                "pressure": 0.0, "band": "calm", "raidActive": false, "warriors": 0,
                "weapons": 0.0, "armor": 0.0
            },
            "raiders": [],
            "buildings": [],
            "claimedTiles": [],
            "villageGate": null,
            "villageRadius": 4,
            "anchor": { "x": 6, "y": 6 }
        });
        let colony: ColonySnapshot =
            serde_json::from_value(legacy_json).expect("deserialize legacy colony snapshot");
        assert_eq!(colony.coin, 0.0);
        assert!(colony.trader.is_none());
        assert!(colony.election_schedule.is_none());
    }

    #[test]
    fn election_schedule_is_additive_and_round_trips_camel_case() {
        let schedule = ElectionScheduleSnapshot {
            term_started_at: Some(1_000),
            next_election_at: 87_400_000,
            term_length_ms: 86_400_000,
            remaining_ms: 43_200_000,
        };
        let encoded = serde_json::to_value(&schedule).expect("serialize election schedule");
        assert_eq!(
            encoded,
            json!({
                "termStartedAt": 1_000,
                "nextElectionAt": 87_400_000,
                "termLengthMs": 86_400_000,
                "remainingMs": 43_200_000
            })
        );
        assert_eq!(
            serde_json::from_value::<ElectionScheduleSnapshot>(encoded)
                .expect("deserialize election schedule"),
            schedule
        );
    }

    #[test]
    fn optional_action_payloads_match_ts_null_and_omitted_shapes() {
        let presence = ClientAction::Presence {
            session_id: "session_1".to_string(),
            nickname: "Guest Cat".to_string(),
            sig: None,
        };
        assert_eq!(
            serde_json::to_value(&presence).expect("serialize presence"),
            json!({
                "action": "presence",
                "sessionId": "session_1",
                "nickname": "Guest Cat"
            })
        );

        let unassign = ClientAction::AssignWorker {
            session_id: "session_1".to_string(),
            nickname: "Guest Cat".to_string(),
            sig: "signed".to_string(),
            cat_id: "cat_1".to_string(),
            building_id: None,
        };
        assert_eq!(
            serde_json::to_value(&unassign).expect("serialize assignWorker"),
            json!({
                "action": "assignWorker",
                "sessionId": "session_1",
                "nickname": "Guest Cat",
                "sig": "signed",
                "catId": "cat_1",
                "buildingId": null
            })
        );
    }

    #[test]
    fn world_snapshot_round_trips_with_dashboard_field_names() {
        let snapshot = sample_world_snapshot();
        let encoded = serde_json::to_value(&snapshot).expect("serialize snapshot");

        assert_eq!(encoded["worldSeed"], json!(123456));
        assert_eq!(encoded["onlineCount"], json!(2));
        assert_eq!(encoded["colonies"][0]["cats"][0]["ageHours"], json!(42.5));
        assert_eq!(
            encoded["colonies"][0]["cats"][0]["assignedBuildingId"],
            json!("building_1")
        );
        assert_eq!(
            encoded["colonies"][0]["jobs"][0]["clickTimeReducedSec"],
            json!(3.5)
        );
        assert_eq!(
            encoded["colonies"][0]["research"]["ownedNodeIds"],
            json!(["root"])
        );
        assert_eq!(
            encoded["colonies"][0]["voteKick"]["targetCatId"],
            json!("cat_1")
        );
        assert_eq!(
            encoded["colonies"][0]["villageGate"],
            json!({ "x": 5, "y": 7, "side": "S" })
        );

        let decoded: WorldSnapshot = serde_json::from_value(encoded).expect("deserialize snapshot");
        assert_eq!(decoded, snapshot);
    }

    #[test]
    fn populated_zero_den_housing_snapshot_is_json_round_trip_safe() {
        let mut snapshot = sample_world_snapshot();
        snapshot.colonies[0].housing = HousingSnapshot {
            population: 15,
            capacity: 0,
            pressure: 15.0,
            village_level: 1,
            housed: 0,
            probationary: 0,
            unhoused: 15,
            departures: 0,
        };

        let websocket_text = serde_json::to_string(&snapshot).expect("serialize WS snapshot");
        let wire: serde_json::Value =
            serde_json::from_str(&websocket_text).expect("inspect WS snapshot");
        assert_eq!(wire["colonies"][0]["housing"]["pressure"], 15.0);
        let decoded: WorldSnapshot =
            serde_json::from_str(&websocket_text).expect("deserialize WS snapshot");
        assert_eq!(decoded, snapshot);
        assert!(decoded.colonies[0].housing.pressure.is_finite());
    }

    #[test]
    fn village_trade_caravan_snapshot_round_trips_truthful_progress() {
        let caravan = VillageTradeCaravanSnapshot {
            id: "trade-1".to_owned(),
            actor_id: "caravan-trade-1".to_owned(),
            from_colony_id: "commons".to_owned(),
            to_colony_id: "moss".to_owned(),
            offered_kind: ResourceKind::Food,
            offered_amount: 10.0,
            requested_kind: ResourceKind::Materials,
            requested_amount: 4.0,
            offered_item_ids: vec!["world-item:7:commons:item-1".to_owned()],
            requested_item_ids: Vec::new(),
            phase: VillageTradeCaravanPhase::Returning,
            position: WorldPoint { x: 42.5, y: -8.25 },
            route: vec![WorldPoint { x: 0.0, y: 0.0 }],
            accepted_at: 1_700_000_000_000,
        };
        let encoded = serde_json::to_value(&caravan).expect("serialize caravan");
        assert_eq!(encoded["phase"], "returning");
        assert_eq!(encoded["position"]["x"], 42.5);
        assert_eq!(
            serde_json::from_value::<VillageTradeCaravanSnapshot>(encoded)
                .expect("deserialize caravan"),
            caravan
        );
    }

    #[test]
    fn legacy_world_snapshot_defaults_secure_village_metadata() {
        let mut value = serde_json::to_value(sample_world_snapshot()).expect("snapshot value");
        let object = value.as_object_mut().expect("world object");
        object.remove("selectedColonyId");
        object.remove("knownVillages");
        object.remove("villageTradeCaravans");
        let colony = object
            .get_mut("colonies")
            .and_then(serde_json::Value::as_array_mut)
            .and_then(|colonies| colonies.first_mut())
            .and_then(serde_json::Value::as_object_mut)
            .expect("colony object");
        colony.remove("kind");
        colony.remove("scale");
        colony.remove("capabilities");

        let decoded: WorldSnapshot =
            serde_json::from_value(value).expect("legacy snapshot remains compatible");

        assert_eq!(decoded.selected_colony_id, None);
        assert!(decoded.known_villages.is_empty());
        assert!(decoded.village_trade_offers.is_empty());
        assert_eq!(decoded.colonies[0].kind, VillageKind::Global);
        assert_eq!(decoded.colonies[0].scale, VillageScale::Personal);
        assert_eq!(
            decoded.colonies[0].capabilities,
            VillageCapabilities::default()
        );
    }

    #[test]
    fn ts_dashboard_id_aliases_deserialize() {
        let mut encoded =
            serde_json::to_value(sample_world_snapshot()).expect("serialize snapshot");
        let colony = encoded["colonies"][0]
            .as_object_mut()
            .expect("colony object");
        colony.remove("id");
        colony.insert("_id".to_string(), json!("colony_1"));
        let cat = colony["cats"][0].as_object_mut().expect("cat object");
        cat.remove("id");
        cat.insert("_id".to_string(), json!("cat_1"));
        let building = colony["buildings"][0]
            .as_object_mut()
            .expect("building object");
        building.remove("id");
        building.insert("_id".to_string(), json!("building_1"));
        let zone = colony["zones"][0].as_object_mut().expect("zone object");
        zone.remove("id");
        zone.insert("_id".to_string(), json!("zone_1"));
        let raider = colony["raiders"][0].as_object_mut().expect("raider object");
        raider.remove("id");
        raider.insert("_id".to_string(), json!("raider_1"));

        let decoded: WorldSnapshot =
            serde_json::from_value(encoded).expect("deserialize aliased snapshot");
        let colony = &decoded.colonies[0];
        assert_eq!(colony.id, "colony_1");
        assert_eq!(colony.cats[0].id, "cat_1");
        assert_eq!(colony.buildings[0].id, "building_1");
        assert_eq!(colony.zones[0].id, "zone_1");
        assert_eq!(colony.raiders[0].id, "raider_1");
    }

    #[test]
    fn event_snapshot_kind_round_trips_and_defaults_for_older_payloads() {
        let event = EventSnapshot {
            message: "Pebble was born to Ash and Bramble.".to_string(),
            timestamp: 1_700_000_000_500,
            kind: "birth".to_string(),
        };
        let encoded = serde_json::to_value(&event).expect("serialize event");
        assert_eq!(encoded["kind"], json!("birth"));
        let decoded: EventSnapshot = serde_json::from_value(encoded).expect("deserialize event");
        assert_eq!(decoded, event);

        // A payload from before this field existed (no `kind` key) must still
        // deserialize, defaulting `kind` to an empty string rather than failing.
        let legacy = json!({
            "message": "A quiet day in the forest",
            "timestamp": 1_700_000_000_500i64,
        });
        let decoded: EventSnapshot =
            serde_json::from_value(legacy).expect("deserialize legacy event payload");
        assert_eq!(decoded.kind, "");
        assert_eq!(decoded.message, "A quiet day in the forest");
    }

    #[test]
    fn action_result_omits_absent_message() {
        let ok = ActionResult {
            ok: true,
            message: None,
            colony_id: None,
        };
        assert_eq!(
            serde_json::to_value(&ok).expect("serialize action result"),
            json!({ "ok": true })
        );

        let failed = ActionResult {
            ok: false,
            message: Some("Unknown action.".to_string()),
            colony_id: None,
        };
        assert_eq!(
            serde_json::to_value(&failed).expect("serialize action result"),
            json!({ "ok": false, "message": "Unknown action." })
        );
    }

    fn sample_world_snapshot() -> WorldSnapshot {
        let mut tally = BTreeMap::new();
        tally.insert("cat_1".to_string(), 2);

        WorldSnapshot {
            now: 1_700_000_000_000,
            world_seed: 123456,
            online_count: 2,
            selected_colony_id: Some("colony_1".to_owned()),
            known_villages: Vec::new(),
            village_trade_offers: Vec::new(),
            village_trade_caravans: Vec::new(),
            colonies: vec![ColonySnapshot {
                id: "colony_1".to_string(),
                name: "Global Colony".to_string(),
                kind: VillageKind::Global,
                scale: VillageScale::Communal,
                capabilities: VillageCapabilities::default(),
                status: ColonyStatus::Thriving,
                resources: ResourceAmounts {
                    food: 50.0,
                    fish: 0.0,
                    water: 40.0,
                    herbs: 5.0,
                    catnip: 0.0,
                    grain: 0.0,
                    flour: 0.0,
                    preserves: 0.0,
                    medicine: 0.0,
                    brew: 0.0,
                    materials: 12.0,
                    stone: 0.0,
                    refined: 3.0,
                    weapons: 2.0,
                    armor: 1.0,
                    planks: 0.0,
                    logs: 0.0,
                    lumber: 0.0,
                    blocks: 0.0,
                    tools: 0.0,
                    fibre: 0.0,
                    hide: 0.0,
                    bone: 0.0,
                    cloth: 0.0,
                    leather: 0.0,
                    ore: 0.0,
                    gem: 0.0,
                    clay: 0.0,
                    sand: 0.0,
                    metal: 0.0,
                    blessings: 8.0,
                },
                storage: StorageSnapshot {
                    capacities: ResourceCapacities {
                        food: 200.0,
                        fish: 200.0,
                        water: 200.0,
                        herbs: 100.0,
                        catnip: 100.0,
                        grain: 100.0,
                        flour: 100.0,
                        preserves: 100.0,
                        medicine: 100.0,
                        brew: 100.0,
                        materials: 100.0,
                        stone: 100.0,
                        refined: 100.0,
                        weapons: 0.0,
                        armor: 0.0,
                        planks: 0.0,
                        logs: 100.0,
                        lumber: 100.0,
                        blocks: 0.0,
                        tools: 0.0,
                        fibre: 100.0,
                        hide: 100.0,
                        bone: 100.0,
                        cloth: 100.0,
                        leather: 100.0,
                        ore: 100.0,
                        gem: 100.0,
                        clay: 100.0,
                        sand: 100.0,
                        metal: 100.0,
                    },
                    food_capacity: Some(200.0),
                    tithe_rates: TitheRates {
                        food: 20.0,
                        refined: 5.0,
                    },
                },
                leader: Some(LeaderSnapshot {
                    id: "cat_1".to_string(),
                    name: "Moss".to_string(),
                    leadership: 9.0,
                }),
                cats: vec![CatSnapshot {
                    id: "cat_1".to_string(),
                    name: "Moss".to_string(),
                    position: MapPosition {
                        map: MapName::World,
                        x: 6,
                        y: 6,
                    },
                    activity: CatActivity::Working,
                    destination: Some(MapPosition {
                        map: MapName::World,
                        x: 7,
                        y: 6,
                    }),
                    carrying: Some(Carrying {
                        kind: CarryingKind::Food,
                        amount: 4.0,
                        job_ended_at: 1_700_000_000_100,
                        item_ids: Vec::new(),
                    }),
                    equipment: EquipmentLoadoutSnapshot::default(),
                    specialization: Some(Specialization::Hunter),
                    age_hours: 42.5,
                    needs: CatNeeds {
                        hunger: 10.0,
                        thirst: 15.0,
                        rest: 20.0,
                        health: 100.0,
                    },
                    current_task: Some("Hunting".to_string()),
                    assigned_building_id: Some("building_1".to_string()),
                    role_xp: RoleXp {
                        hunter: 1.0,
                        architect: 0.0,
                        ritualist: 0.0,
                        warrior: 0.0,
                    },
                    skills: BTreeMap::from([(Labor::Hunt, 1.0)]),
                    stats: CatStats { leadership: 9.0 },
                    death_time: None,
                    parent_ids: vec!["cat_0".to_string()],
                    parents: vec!["Ash".to_string()],
                    boosted: false,
                    preferred_labors: vec![Labor::Hunt],
                    pregnant: false,
                    housing_status: CatHousingStatus::Housed,
                    migration_status: CatMigrationStatus::Resident,
                    probation_remaining_game_minutes: None,
                }],
                jobs: vec![JobSnapshot {
                    id: "job_1".to_string(),
                    kind: JobKind::SupplyFood,
                    status: JobStatus::Active,
                    ends_at: 1_700_000_010_000,
                    started_at: 1_700_000_000_000,
                    click_time_reduced_sec: 3.5,
                    assigned_cat_name: Some("Moss".to_string()),
                }],
                upgrades: vec![UpgradeSnapshot {
                    key: UpgradeKey::ClickPower,
                    level: 1,
                    max_level: 5,
                    base_cost: 10,
                }],
                events: vec![EventSnapshot {
                    message: "Moss brought back food.".to_string(),
                    kind: "job_completed".to_string(),
                    timestamp: 1_700_000_000_500,
                }],
                housing: HousingSnapshot {
                    population: 1,
                    capacity: 4,
                    pressure: 0.25,
                    village_level: 1,
                    housed: 1,
                    probationary: 0,
                    unhoused: 0,
                    departures: 0,
                },
                research: ResearchSnapshot {
                    owned_node_ids: vec!["root".to_string()],
                    research_points: 2.5,
                    researcher_count: 1,
                    blessings: 8.0,
                    next_target: Some(ResearchTarget {
                        id: "water".to_string(),
                        name: "Water Wisdom".to_string(),
                        cost: 5.0,
                    }),
                },
                election: Some(ElectionSnapshot {
                    id: "election_1".to_string(),
                    ends_at: 1_700_000_030_000,
                    tally,
                    total_ballots: 2,
                    candidates: vec![ElectionCandidate {
                        id: "cat_1".to_string(),
                        name: "Moss".to_string(),
                        leadership: 9.0,
                        specialization: Some(Specialization::Hunter),
                    }],
                }),
                election_schedule: Some(ElectionScheduleSnapshot {
                    term_started_at: Some(1_699_913_600_000),
                    next_election_at: 1_700_000_000_000,
                    term_length_ms: 86_400_000,
                    remaining_ms: 0,
                }),
                vote_kick: Some(VoteKickSnapshot {
                    id: "kick_1".to_string(),
                    ends_at: 1_700_000_040_000,
                    target_cat_id: "cat_1".to_string(),
                    target_name: "Moss".to_string(),
                    signatures: 1,
                    needed: 3,
                }),
                zones: vec![ZoneSnapshot {
                    id: "zone_1".to_string(),
                    kind: ZoneKind::Gather,
                    x1: 1,
                    y1: 2,
                    x2: 3,
                    y2: 4,
                    expires_at: 1_700_000_060_000,
                }],
                threat: ThreatSnapshot {
                    pressure: 12.0,
                    band: ThreatBand::Rising,
                    raid_active: true,
                    warriors: 1,
                    weapons: 2.0,
                    armor: 1.0,
                },
                raiders: vec![RaiderSnapshot {
                    id: "raider_1".to_string(),
                    position: TilePoint { x: 10, y: 11 },
                    hp: 7.0,
                    strength: 10.0,
                    status: RaiderStatus::Advancing,
                }],
                buildings: vec![BuildingSnapshot {
                    id: "building_1".to_string(),
                    building_type: BuildingType::Shrine,
                    level: 1,
                    construction_progress: 100.0,
                    world_position: TilePoint { x: 6, y: 6 },
                    position: TilePoint { x: 0, y: 0 },
                    footprint: FootprintSize {
                        width: 3,
                        height: 3,
                    },
                    staff_count: 0,
                    staff_cap: 0,
                    production_progress: 0.0,
                    production_output: None,
                    inbound_haul: 0.0,
                    outbound_haul: 0.0,
                    ..BuildingSnapshot::default()
                }],
                claimed_tiles: vec![TilePoint { x: 6, y: 6 }],
                agricultural_tiles: vec![],
                revealed_tiles: vec![TilePoint { x: 6, y: 6 }],
                provisional_tiles: vec![],
                road_tiles: vec![],
                transport: TransportSnapshot::default(),
                stump_tiles: vec![],
                sapling_tiles: vec![],
                dirt_road_tiles: vec![],
                village_gate: Some(GatePlacement {
                    x: 5,
                    y: 7,
                    side: GateSide::S,
                }),
                wall_segments: vec![],
                village_radius: 4,
                anchor: TilePoint { x: 6, y: 6 },
                officers: BTreeMap::new(),
                stockpiles: Vec::new(),
                active_stockpile_haul: None,
                farms: Vec::new(),
                stock_ledger: None,
                items: Vec::new(),
                coin: 0.0,
                trader: None,
            }],
        }
    }

    #[test]
    fn assign_officer_action_round_trips_with_camel_case_fields() {
        let action = ClientAction::AssignOfficer {
            session_id: "session_1".to_string(),
            nickname: "Guest Cat".to_string(),
            sig: "signed".to_string(),
            role: OfficerRole::Captain,
            cat_id: "cat_1".to_string(),
        };
        let encoded = serde_json::to_value(&action).expect("serialize assignOfficer");
        assert_eq!(
            encoded,
            json!({
                "action": "assignOfficer",
                "sessionId": "session_1",
                "nickname": "Guest Cat",
                "sig": "signed",
                "role": "captain",
                "catId": "cat_1"
            })
        );
        assert_eq!(
            serde_json::from_value::<ClientAction>(encoded).expect("deserialize"),
            action
        );
    }

    #[test]
    fn farm_actions_and_snapshot_round_trip_with_exact_wire_literals() {
        let designate = ClientAction::DesignateFarm {
            session_id: "session_1".to_owned(),
            nickname: "Guest Cat".to_owned(),
            sig: "signed".to_owned(),
            a: TilePoint { x: 14, y: 4 },
            b: TilePoint { x: 16, y: 6 },
            crop: CropKind::Catnip,
        };
        let encoded = serde_json::to_value(&designate).expect("serialize designateFarm");
        assert_eq!(
            encoded,
            json!({
                "action": "designateFarm",
                "sessionId": "session_1",
                "nickname": "Guest Cat",
                "sig": "signed",
                "a": { "x": 14, "y": 4 },
                "b": { "x": 16, "y": 6 },
                "crop": "catnip"
            })
        );
        assert_eq!(
            serde_json::from_value::<ClientAction>(encoded).expect("deserialize designateFarm"),
            designate
        );

        let clear = ClientAction::ClearFarm {
            session_id: "session_1".to_owned(),
            nickname: "Guest Cat".to_owned(),
            sig: "signed".to_owned(),
            plot_id: "farm-1".to_owned(),
        };
        assert_eq!(
            serde_json::to_value(&clear).expect("serialize clearFarm"),
            json!({
                "action": "clearFarm",
                "sessionId": "session_1",
                "nickname": "Guest Cat",
                "sig": "signed",
                "plotId": "farm-1"
            })
        );

        let farm = FarmSnapshot {
            id: "farm-1".to_owned(),
            x1: 14,
            y1: 4,
            x2: 16,
            y2: 6,
            crop: CropKind::Herb,
            planted_at: 1_000,
            stage: FarmStage::Flowering,
            growth_hours: 23.5,
            worker_id: Some("cat-1".to_owned()),
            work_phase: FarmWorkPhase::Harvesting,
            input_inventory: Vec::new(),
            output_inventory: vec![ResourceStackSnapshot {
                kind: ResourceKind::Herbs,
                amount: 4.0,
            }],
            worker_travel: None,
            block_reason: None,
        };
        let encoded = serde_json::to_value(&farm).expect("serialize farm snapshot");
        assert_eq!(encoded["crop"], json!("herb"));
        assert_eq!(encoded["stage"], json!("flowering"));
        assert_eq!(encoded["plantedAt"], json!(1_000));
        assert_eq!(
            serde_json::from_value::<FarmSnapshot>(encoded).expect("farm round-trip"),
            farm
        );
    }

    #[test]
    fn production_chain_wire_literals_are_exact() {
        assert_eq!(
            serde_json::to_value(JobKind::GatherLogs).unwrap(),
            json!("gather_logs")
        );
        assert_eq!(
            serde_json::to_value(JobKind::ReplantTree).unwrap(),
            json!("replant_tree")
        );
        assert_eq!(
            serde_json::to_value(JobKind::ForageFibre).unwrap(),
            json!("forage_fibre")
        );
        assert_eq!(
            serde_json::to_value(BuildingType::Mill).unwrap(),
            json!("mill")
        );
        assert_eq!(
            serde_json::to_value(BuildingType::Sawmill).unwrap(),
            json!("sawmill")
        );
        assert_eq!(
            serde_json::to_value(ResourceKind::Logs).unwrap(),
            json!("logs")
        );
        assert_eq!(
            serde_json::to_value(ResourceKind::Lumber).unwrap(),
            json!("lumber")
        );
    }

    #[test]
    fn boost_cat_action_round_trips_with_camel_case_fields() {
        let action = ClientAction::BoostCat {
            session_id: "session_1".to_string(),
            nickname: "Guest Cat".to_string(),
            sig: "signed".to_string(),
            cat_id: "cat_1".to_string(),
            boosted: true,
        };
        let encoded = serde_json::to_value(&action).expect("serialize boostCat");
        assert_eq!(
            encoded,
            json!({
                "action": "boostCat",
                "sessionId": "session_1",
                "nickname": "Guest Cat",
                "sig": "signed",
                "catId": "cat_1",
                "boosted": true
            })
        );
        assert_eq!(
            serde_json::from_value::<ClientAction>(encoded).expect("deserialize"),
            action
        );
    }

    #[test]
    fn designate_stockpile_action_round_trips_with_camel_case_fields() {
        let action = ClientAction::DesignateStockpile {
            session_id: "session_1".to_string(),
            nickname: "Guest Cat".to_string(),
            sig: "signed".to_string(),
            a: TilePoint { x: 3, y: 4 },
            b: TilePoint { x: 5, y: 6 },
            accepts: vec![ResourceKind::Food, ResourceKind::Water],
        };
        let encoded = serde_json::to_value(&action).expect("serialize designateStockpile");
        assert_eq!(
            encoded,
            json!({
                "action": "designateStockpile",
                "sessionId": "session_1",
                "nickname": "Guest Cat",
                "sig": "signed",
                "a": { "x": 3, "y": 4 },
                "b": { "x": 5, "y": 6 },
                "accepts": ["food", "water"]
            })
        );
        assert_eq!(
            serde_json::from_value::<ClientAction>(encoded).expect("deserialize"),
            action
        );
    }

    #[test]
    fn colony_snapshot_stockpiles_round_trip_and_default_empty() {
        // Absent `stockpiles` deserializes to empty (back-compat).
        let mut value = serde_json::to_value(sample_world_snapshot()).expect("serialize");
        assert!(value["colonies"][0].get("stockpiles").is_none());
        value["colonies"][0]
            .as_object_mut()
            .unwrap()
            .remove("stockpiles");
        let back: WorldSnapshot = serde_json::from_value(value).expect("deserialize");
        assert!(back.colonies[0].stockpiles.is_empty());
        assert!(back.colonies[0].active_stockpile_haul.is_none());

        // A populated stockpile round-trips.
        let mut snap = sample_world_snapshot();
        let contents = snap.colonies[0].resources;
        snap.colonies[0].stockpiles.push(StockpileSnapshot {
            id: "stockpile-shrine".to_string(),
            x1: 6,
            y1: 6,
            x2: 6,
            y2: 6,
            accepts: vec![ResourceKind::Food],
            contents,
            report: Some(StockpileReportSnapshot {
                reported: contents,
                last_counted: 1_700_000_030_000,
                accurate: false,
            }),
            gather_spot: None,
            steward_managed: Some(StewardManagedPileSnapshot {
                station_id: "mill-a".to_owned(),
                resource: ResourceKind::Food,
                active: true,
            }),
        });
        snap.colonies[0].active_stockpile_haul = Some(StockpileHaulSnapshot {
            job_id: "job-balance".to_owned(),
            worker_id: "cat-1".to_owned(),
            source_stockpile_id: "stockpile-storehouse".to_owned(),
            destination_stockpile_id: "stockpile-shrine".to_owned(),
            resource: ResourceKind::Food,
            amount: 7.5,
            phase: StockpileHaulPhase::CarryingToDestination,
        });
        let encoded = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(
            encoded["colonies"][0]["stockpiles"][0]["id"],
            json!("stockpile-shrine")
        );
        assert_eq!(
            encoded["colonies"][0]["stockpiles"][0]["stewardManaged"]["stationId"],
            json!("mill-a")
        );
        assert!(
            encoded["colonies"][0]["stockpiles"][0]["report"]
                .get("accurate")
                .is_none()
        );
        assert_eq!(
            encoded["colonies"][0]["activeStockpileHaul"]["phase"],
            json!("carrying_to_destination")
        );
        let round: WorldSnapshot = serde_json::from_value(encoded).expect("round-trip");
        assert_eq!(round.colonies[0].stockpiles, snap.colonies[0].stockpiles);
        assert_eq!(
            round.colonies[0].active_stockpile_haul,
            snap.colonies[0].active_stockpile_haul
        );
    }

    #[test]
    fn colony_snapshot_stock_ledger_round_trips_and_defaults_absent() {
        // Absent `stockLedger` deserializes to None (back-compat).
        let value = serde_json::to_value(sample_world_snapshot()).expect("serialize");
        assert!(value["colonies"][0].get("stockLedger").is_none());
        let back: WorldSnapshot = serde_json::from_value(value).expect("deserialize");
        assert!(back.colonies[0].stock_ledger.is_none());

        // A populated ledger round-trips with camelCase fields.
        let mut snap = sample_world_snapshot();
        let reported = snap.colonies[0].resources;
        snap.colonies[0].stock_ledger = Some(StockLedgerSnapshot {
            reported,
            last_counted: 1_700_000_030_000,
            accurate: true,
            active_round: None,
        });
        let encoded = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(
            encoded["colonies"][0]["stockLedger"]["lastCounted"],
            json!(1_700_000_030_000_i64)
        );
        assert_eq!(
            encoded["colonies"][0]["stockLedger"]["accurate"],
            json!(true)
        );
        let round: WorldSnapshot = serde_json::from_value(encoded).expect("round-trip");
        assert_eq!(
            round.colonies[0].stock_ledger,
            snap.colonies[0].stock_ledger
        );

        let mut player_safe = snap;
        player_safe.colonies[0]
            .stock_ledger
            .as_mut()
            .expect("ledger")
            .accurate = false;
        let encoded = serde_json::to_value(&player_safe).expect("serialize player-safe ledger");
        assert!(
            encoded["colonies"][0]["stockLedger"]
                .get("accurate")
                .is_none()
        );
        let round: WorldSnapshot = serde_json::from_value(encoded).expect("default accuracy");
        assert!(
            !round.colonies[0]
                .stock_ledger
                .as_ref()
                .expect("ledger")
                .accurate
        );
    }

    #[test]
    fn colony_snapshot_officers_map_serializes_by_role_and_defaults_empty() {
        // Absent `officers` deserializes to an empty map (back-compat).
        let mut value = serde_json::to_value(sample_world_snapshot()).expect("serialize");
        assert!(value["colonies"][0].get("officers").is_none());
        value["colonies"][0]
            .as_object_mut()
            .unwrap()
            .remove("officers");
        let back: WorldSnapshot = serde_json::from_value(value).expect("deserialize");
        assert!(back.colonies[0].officers.is_empty());

        // A populated map round-trips keyed by the role wire string.
        let mut snap = sample_world_snapshot();
        snap.colonies[0]
            .officers
            .insert(OfficerRole::Farmer, "cat_1".to_string());
        let encoded = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(encoded["colonies"][0]["officers"]["farmer"], json!("cat_1"));
        let round: WorldSnapshot = serde_json::from_value(encoded).expect("round-trip");
        assert_eq!(round.colonies[0].officers, snap.colonies[0].officers);
    }

    #[test]
    fn item_stack_snapshot_round_trips_with_camel_case_fields() {
        let stack = ItemStackSnapshot {
            kind: "weapon".to_string(),
            material: "metal".to_string(),
            quality: 3,
            count: 2,
            value: 130,
            unit_weight_grams: 4_275,
            instances: vec![ItemInstanceSnapshot {
                id: "item-1".to_owned(),
                durability: 0,
                max_durability: 26,
                broken: true,
                credited: false,
                location: ItemLocation::Station {
                    building_id: "smithy-a".to_owned(),
                    compartment: StationCompartment::LocalOutput,
                },
            }],
        };
        let encoded = serde_json::to_value(&stack).expect("serialize");
        assert_eq!(
            encoded,
            json!({
                "kind": "weapon",
                "material": "metal",
                "quality": 3,
                "count": 2,
                "value": 130,
                "unitWeightGrams": 4275,
                "instances": [{
                    "id": "item-1",
                    "durability": 0,
                    "maxDurability": 26,
                    "broken": true,
                    "credited": false,
                    "location": {
                        "kind": "station",
                        "buildingId": "smithy-a",
                        "compartment": "local_output"
                    }
                }]
            })
        );
        let back: ItemStackSnapshot = serde_json::from_value(encoded).expect("deserialize");
        assert_eq!(back, stack);
    }

    #[test]
    fn legacy_item_cat_and_carrying_payloads_default_to_no_duplicate_loadout() {
        let item: ItemInstanceSnapshot = serde_json::from_value(json!({
            "id": "item-1",
            "durability": 4,
            "maxDurability": 6,
            "broken": false
        }))
        .expect("legacy item instance");
        assert_eq!(item.location, ItemLocation::LegacyTreasury);
        assert!(item.credited);

        let mut snapshot = serde_json::to_value(sample_world_snapshot()).expect("snapshot");
        let cat = snapshot["colonies"][0]["cats"][0]
            .as_object_mut()
            .expect("cat object");
        cat.remove("equipment");
        cat.get_mut("carrying")
            .and_then(serde_json::Value::as_object_mut)
            .expect("carrying object")
            .remove("itemIds");
        let decoded: WorldSnapshot = serde_json::from_value(snapshot).expect("legacy snapshot");
        assert_eq!(
            decoded.colonies[0].cats[0].equipment,
            EquipmentLoadoutSnapshot::default()
        );
        assert!(
            decoded.colonies[0].cats[0]
                .carrying
                .as_ref()
                .expect("carrying")
                .item_ids
                .is_empty()
        );
    }

    #[test]
    fn colony_snapshot_items_default_empty_and_omitted_when_absent() {
        // An empty item store omits `items` from the wire payload entirely (P19 slice 1
        // is inert for every colony, so this is the common case).
        let encoded = serde_json::to_value(sample_world_snapshot()).expect("serialize");
        assert!(encoded["colonies"][0].get("items").is_none());

        // Old payloads with no `items` key at all still deserialize (back-compat).
        let mut value = encoded.clone();
        value["colonies"][0]
            .as_object_mut()
            .unwrap()
            .remove("items");
        let back: WorldSnapshot = serde_json::from_value(value).expect("deserialize");
        assert!(back.colonies[0].items.is_empty());

        // A populated store round-trips.
        let mut snap = sample_world_snapshot();
        snap.colonies[0].items.push(ItemStackSnapshot {
            kind: "mug".to_string(),
            material: "wood".to_string(),
            quality: 1,
            count: 5,
            value: 4,
            unit_weight_grams: 420,
            instances: Vec::new(),
        });
        let encoded = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(encoded["colonies"][0]["items"][0]["kind"], json!("mug"));
        let round: WorldSnapshot = serde_json::from_value(encoded).expect("round-trip");
        assert_eq!(round.colonies[0].items, snap.colonies[0].items);
    }

    #[test]
    fn building_snapshot_production_fields_round_trip_with_camel_case_names() {
        let building = BuildingSnapshot {
            id: "building_2".to_string(),
            building_type: BuildingType::Workshop,
            level: 2,
            construction_progress: 100.0,
            world_position: TilePoint { x: 8, y: 6 },
            position: TilePoint { x: 8, y: 6 },
            footprint: FootprintSize {
                width: 3,
                height: 3,
            },
            staff_count: 1,
            staff_cap: 1,
            production_progress: 0.4,
            production_output: Some("refined".to_string()),
            inbound_haul: 0.0,
            outbound_haul: 0.0,
            input_capacity: 12.0,
            output_capacity: 12.0,
            ..BuildingSnapshot::default()
        };

        let encoded = serde_json::to_value(&building).expect("serialize building");
        assert_eq!(
            encoded,
            json!({
                "id": "building_2",
                "type": "workshop",
                "level": 2,
                "constructionProgress": 100.0,
                "worldPosition": { "x": 8, "y": 6 },
                "position": { "x": 8, "y": 6 },
                "footprint": { "width": 3, "height": 3 },
                "staffCount": 1,
                "staffCap": 1,
                "productionProgress": 0.4,
                "productionOutput": "refined",
                "inboundHaul": 0.0,
                "outboundHaul": 0.0,
                "inputCapacity": 12.0,
                "outputCapacity": 12.0,
            })
        );

        let decoded: BuildingSnapshot = serde_json::from_value(encoded).expect("deserialize");
        assert_eq!(decoded, building);
    }

    #[test]
    fn building_snapshot_new_production_fields_default_for_old_payloads() {
        // A payload from before staffing/production fields existed (only the
        // original fields present) must still deserialize, defaulting the new
        // fields to their back-compat values.
        let old_payload = json!({
            "id": "building_1",
            "type": "shrine",
            "level": 1,
            "constructionProgress": 100.0,
            "worldPosition": { "x": 6, "y": 6 },
            "position": { "x": 6, "y": 6 },
        });

        let decoded: BuildingSnapshot =
            serde_json::from_value(old_payload).expect("deserialize old payload");
        assert_eq!(decoded.staff_count, 0);
        assert_eq!(decoded.staff_cap, 0);
        assert!(decoded.work_slots.is_empty());
        assert_eq!(decoded.production_progress, 0.0);
        assert_eq!(decoded.production_output, None);
        assert_eq!(decoded.inbound_haul, 0.0);
        assert_eq!(decoded.input_capacity, 0.0);
        assert_eq!(decoded.output_capacity, 0.0);
        assert!(decoded.available_recipes.is_empty());
        assert_eq!(decoded.required_recipe_study, None);
        assert!(decoded.construction_required.is_empty());
        assert!(decoded.construction_delivered.is_empty());
        assert!(decoded.construction_in_transit.is_empty());
        assert_eq!(decoded.construction_block_reason, None);
        // The pre-existing footprint back-compat default is untouched by this change.
        assert_eq!(
            decoded.footprint,
            FootprintSize {
                width: 1,
                height: 1
            }
        );
    }

    #[test]
    fn scaffold_physical_inputs_round_trip_with_additive_wire_fields() {
        let building = BuildingSnapshot {
            id: "workshop-scaffold".to_owned(),
            building_type: BuildingType::Workshop,
            construction_required: vec![ResourceStackSnapshot {
                kind: ResourceKind::Lumber,
                amount: 2.0,
            }],
            construction_delivered: vec![ResourceStackSnapshot {
                kind: ResourceKind::Lumber,
                amount: 1.0,
            }],
            construction_in_transit: vec![ResourceStackSnapshot {
                kind: ResourceKind::Blocks,
                amount: 2.0,
            }],
            construction_block_reason: Some("materials_in_transit".to_owned()),
            ..BuildingSnapshot::default()
        };

        let encoded = serde_json::to_value(&building).expect("serialize scaffold");
        assert_eq!(encoded["constructionRequired"][0]["kind"], json!("lumber"));
        assert_eq!(encoded["constructionDelivered"][0]["amount"], json!(1.0));
        assert_eq!(encoded["constructionInTransit"][0]["kind"], json!("blocks"));
        assert_eq!(
            encoded["constructionBlockReason"],
            json!("materials_in_transit")
        );
        let decoded: BuildingSnapshot = serde_json::from_value(encoded).expect("round trip");
        assert_eq!(decoded, building);
    }

    #[test]
    fn station_available_recipes_use_authoritative_camel_case_wire_field() {
        let building = BuildingSnapshot {
            id: "mill-1".to_owned(),
            building_type: BuildingType::Mill,
            available_recipes: vec!["grain_to_flour".to_owned(), "flour_to_food".to_owned()],
            ..BuildingSnapshot::default()
        };
        let encoded = serde_json::to_value(&building).expect("serialize Mill");
        assert_eq!(
            encoded["availableRecipes"],
            json!(["grain_to_flour", "flour_to_food"])
        );
        let decoded: BuildingSnapshot = serde_json::from_value(encoded).expect("round trip");
        assert_eq!(decoded.available_recipes, building.available_recipes);
        assert_eq!(
            serde_json::to_value(CarryingKind::Flour).unwrap(),
            json!("flour")
        );
    }

    #[test]
    fn multi_recipe_physical_station_snapshot_round_trips_queue_truth() {
        let building = BuildingSnapshot {
            id: "smithy-1".to_owned(),
            building_type: BuildingType::Smithy,
            production_queue: vec![
                ProductionQueueEntrySnapshot {
                    recipe_id: "smithy_weapon".to_owned(),
                    repeat: true,
                },
                ProductionQueueEntrySnapshot {
                    recipe_id: "smithy_armor".to_owned(),
                    repeat: true,
                },
            ],
            available_recipes: vec!["smithy_weapon".to_owned()],
            production_block_reason: Some("missing_metal".to_owned()),
            ..BuildingSnapshot::default()
        };
        let encoded = serde_json::to_value(&building).expect("serialize Smithy descriptor");
        assert_eq!(
            encoded["productionQueue"][1]["recipeId"],
            json!("smithy_armor")
        );
        assert_eq!(encoded["availableRecipes"], json!(["smithy_weapon"]));
        assert_eq!(encoded["productionBlockReason"], json!("missing_metal"));
        assert_eq!(
            serde_json::from_value::<BuildingSnapshot>(encoded).unwrap(),
            building
        );
    }

    #[test]
    fn locked_station_required_study_is_additive_and_round_trips() {
        let building = BuildingSnapshot {
            id: "sawmill-locked".to_owned(),
            building_type: BuildingType::Sawmill,
            required_recipe_study: Some(ResearchTarget {
                id: "carpentry_preparation".to_owned(),
                name: "Carpentry Preparation".to_owned(),
                cost: 19.5,
            }),
            ..BuildingSnapshot::default()
        };
        let encoded = serde_json::to_value(&building).expect("serialize locked station");
        assert_eq!(
            encoded["requiredRecipeStudy"]["id"],
            json!("carpentry_preparation")
        );
        let decoded: BuildingSnapshot = serde_json::from_value(encoded).expect("round trip");
        assert_eq!(decoded, building);
    }

    #[test]
    fn physical_refiner_carrying_kinds_round_trip_with_additive_wire_literals() {
        for (kind, literal) in [
            (CarryingKind::Refined, "refined"),
            (CarryingKind::Bone, "bone"),
            (CarryingKind::Hide, "hide"),
            (CarryingKind::Leather, "leather"),
            (CarryingKind::Fibre, "fibre"),
            (CarryingKind::Cloth, "cloth"),
            (CarryingKind::Ore, "ore"),
            (CarryingKind::Metal, "metal"),
            (CarryingKind::Weapons, "weapons"),
            (CarryingKind::Armor, "armor"),
        ] {
            let encoded = serde_json::to_value(kind).expect("serialize carrying kind");
            assert_eq!(encoded, json!(literal));
            assert_eq!(
                serde_json::from_value::<CarryingKind>(encoded).expect("round trip"),
                kind
            );
        }

        // Existing carrying literals remain unchanged when the additive refiner
        // variants are introduced.
        assert_eq!(
            serde_json::from_value::<CarryingKind>(json!("materials")).unwrap(),
            CarryingKind::Materials
        );
    }

    #[test]
    fn cat_snapshot_lineage_round_trips_and_defaults_empty_for_old_payloads() {
        let mut snap = sample_world_snapshot();
        snap.colonies[0].cats[0].parent_ids = vec!["cat_0".to_string(), "cat_-1".to_string()];
        snap.colonies[0].cats[0].parents = vec!["Ash".to_string(), "Bramble".to_string()];
        let encoded = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(
            encoded["colonies"][0]["cats"][0]["parentIds"],
            json!(["cat_0", "cat_-1"])
        );
        assert_eq!(
            encoded["colonies"][0]["cats"][0]["parents"],
            json!(["Ash", "Bramble"])
        );
        let round: WorldSnapshot = serde_json::from_value(encoded).expect("round-trip");
        assert_eq!(round, snap);

        // Absent lineage fields (pre-lineage payload) default to empty vecs.
        let mut old = serde_json::to_value(sample_world_snapshot()).expect("serialize");
        let cat = old["colonies"][0]["cats"][0]
            .as_object_mut()
            .expect("cat object");
        cat.remove("parentIds");
        cat.remove("parents");
        let back: WorldSnapshot = serde_json::from_value(old).expect("deserialize");
        assert!(back.colonies[0].cats[0].parent_ids.is_empty());
        assert!(back.colonies[0].cats[0].parents.is_empty());
    }

    #[test]
    fn cat_snapshot_boosted_round_trips_and_defaults_false_for_old_payloads() {
        let mut snap = sample_world_snapshot();
        snap.colonies[0].cats[0].boosted = true;
        let encoded = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(encoded["colonies"][0]["cats"][0]["boosted"], json!(true));
        let round: WorldSnapshot = serde_json::from_value(encoded).expect("round-trip");
        assert_eq!(round, snap);

        // Absent `boosted` (pre-P15 payload) defaults to false.
        let mut old = serde_json::to_value(sample_world_snapshot()).expect("serialize");
        old["colonies"][0]["cats"][0]
            .as_object_mut()
            .expect("cat object")
            .remove("boosted");
        let back: WorldSnapshot = serde_json::from_value(old).expect("deserialize");
        assert!(!back.colonies[0].cats[0].boosted);
    }

    #[test]
    fn cat_snapshot_pregnant_round_trips_and_defaults_false_for_old_payloads() {
        let mut snap = sample_world_snapshot();
        snap.colonies[0].cats[0].pregnant = true;
        let encoded = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(encoded["colonies"][0]["cats"][0]["pregnant"], json!(true));
        let round: WorldSnapshot = serde_json::from_value(encoded).expect("round-trip");
        assert_eq!(round, snap);

        // Absent `pregnant` (pre-life-sim-UI payload) defaults to false.
        let mut old = serde_json::to_value(sample_world_snapshot()).expect("serialize");
        old["colonies"][0]["cats"][0]
            .as_object_mut()
            .expect("cat object")
            .remove("pregnant");
        let back: WorldSnapshot = serde_json::from_value(old).expect("deserialize");
        assert!(!back.colonies[0].cats[0].pregnant);
    }

    #[test]
    fn cat_snapshot_probation_countdown_round_trips_and_defaults_absent() {
        let mut snap = sample_world_snapshot();
        snap.colonies[0].cats[0].housing_status = CatHousingStatus::Probationary;
        snap.colonies[0].cats[0].migration_status = CatMigrationStatus::Departing;
        snap.colonies[0].cats[0].probation_remaining_game_minutes = Some(2_160);
        let encoded = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(
            encoded["colonies"][0]["cats"][0]["probationRemainingGameMinutes"],
            json!(2_160)
        );
        let round: WorldSnapshot = serde_json::from_value(encoded).expect("round-trip");
        assert_eq!(round, snap);

        let mut old = serde_json::to_value(sample_world_snapshot()).expect("serialize");
        let cat = old["colonies"][0]["cats"][0]
            .as_object_mut()
            .expect("cat object");
        cat.remove("housingStatus");
        cat.remove("migrationStatus");
        cat.remove("probationRemainingGameMinutes");
        let back: WorldSnapshot = serde_json::from_value(old).expect("deserialize");
        assert_eq!(
            back.colonies[0].cats[0].housing_status,
            CatHousingStatus::Housed
        );
        assert_eq!(
            back.colonies[0].cats[0].migration_status,
            CatMigrationStatus::Resident
        );
        assert_eq!(
            back.colonies[0].cats[0].probation_remaining_game_minutes,
            None
        );
    }

    #[test]
    fn manual_officer_domain_actions_round_trip_with_typed_payloads() {
        let actions = [
            ClientAction::ResearchNode {
                session_id: "s".to_owned(),
                nickname: "n".to_owned(),
                sig: "x".to_owned(),
                node_id: "basic_tools".to_owned(),
            },
            ClientAction::OfferTithe {
                session_id: "s".to_owned(),
                nickname: "n".to_owned(),
                sig: "x".to_owned(),
            },
            ClientAction::OfferMaterials {
                session_id: "s".to_owned(),
                nickname: "n".to_owned(),
                sig: "x".to_owned(),
            },
            ClientAction::HaulGatherSpot {
                session_id: "s".to_owned(),
                nickname: "n".to_owned(),
                sig: "x".to_owned(),
                stockpile_id: "gather-1".to_owned(),
                cat_id: None,
            },
        ];

        for action in actions {
            let value = serde_json::to_value(&action).expect("serialize");
            let decoded: ClientAction = serde_json::from_value(value).expect("deserialize");
            assert_eq!(decoded, action);
        }
        assert_eq!(
            serde_json::to_value(BuildingType::AccountingTent).unwrap(),
            json!("accounting_tent")
        );
    }

    #[test]
    fn plan_building_site_round_trips_and_old_payloads_default_to_automatic() {
        let exact = ClientAction::PlanBuilding {
            session_id: "s".to_owned(),
            nickname: "Builder".to_owned(),
            sig: "signed".to_owned(),
            building_type: BuildingType::Sawmill,
            site: Some(TilePoint { x: -17, y: 23 }),
        };
        let encoded = serde_json::to_value(&exact).expect("serialize exact placement");
        assert_eq!(encoded["site"], json!({ "x": -17, "y": 23 }));
        assert_eq!(
            serde_json::from_value::<ClientAction>(encoded).expect("round trip"),
            exact
        );

        let legacy = json!({
            "action": "planBuilding",
            "sessionId": "old",
            "nickname": "Old client",
            "sig": "signed",
            "type": "den"
        });
        assert!(matches!(
            serde_json::from_value::<ClientAction>(legacy).expect("legacy payload"),
            ClientAction::PlanBuilding { site: None, .. }
        ));
    }

    #[test]
    fn labor_skill_map_is_typed_and_legacy_optional() {
        let skills = BTreeMap::from([
            (Labor::Haul, 2.5),
            (Labor::Metalwork, 7.0),
            (Labor::Scout, 1.0),
        ]);
        let encoded = serde_json::to_value(&skills).expect("skills serialize");
        assert_eq!(encoded["haul"], json!(2.5));
        assert_eq!(encoded["metalwork"], json!(7.0));
        assert_eq!(encoded["scout"], json!(1.0));
        assert_eq!(
            serde_json::from_value::<BTreeMap<Labor, f64>>(encoded).unwrap(),
            skills
        );

        let mut legacy = serde_json::to_value(CatSnapshot {
            id: "legacy".to_owned(),
            name: "Legacy".to_owned(),
            position: MapPosition {
                map: MapName::Colony,
                x: 0,
                y: 0,
            },
            activity: CatActivity::Idle,
            destination: None,
            carrying: None,
            equipment: EquipmentLoadoutSnapshot::default(),
            specialization: None,
            age_hours: 10.0,
            needs: CatNeeds {
                hunger: 0.0,
                thirst: 0.0,
                rest: 0.0,
                health: 100.0,
            },
            current_task: None,
            assigned_building_id: None,
            role_xp: RoleXp {
                hunter: 0.0,
                architect: 0.0,
                ritualist: 0.0,
                warrior: 0.0,
            },
            skills: BTreeMap::new(),
            stats: CatStats { leadership: 0.0 },
            death_time: None,
            parent_ids: Vec::new(),
            parents: Vec::new(),
            boosted: false,
            preferred_labors: Vec::new(),
            pregnant: false,
            housing_status: CatHousingStatus::Housed,
            migration_status: CatMigrationStatus::Resident,
            probation_remaining_game_minutes: None,
        })
        .unwrap();
        legacy.as_object_mut().unwrap().remove("skills");
        let decoded: CatSnapshot = serde_json::from_value(legacy).unwrap();
        assert!(decoded.skills.is_empty());
    }

    #[test]
    fn fishing_wire_types_round_trip_and_legacy_gather_spots_default_to_general() {
        let action = ClientAction::DesignateFishingSpot {
            session_id: "session".to_owned(),
            nickname: "Angler".to_owned(),
            sig: "signed".to_owned(),
            at: TilePoint { x: -4, y: 19 },
        };
        let encoded = serde_json::to_value(&action).expect("serialize fishing designation");
        assert_eq!(encoded["action"], json!("designateFishingSpot"));
        assert_eq!(encoded["at"], json!({ "x": -4, "y": 19 }));
        assert_eq!(
            serde_json::from_value::<ClientAction>(encoded).expect("round trip"),
            action
        );
        assert_eq!(serde_json::to_value(JobKind::Fish).unwrap(), json!("fish"));
        assert_eq!(
            serde_json::to_value(Labor::Fishing).unwrap(),
            json!("fishing")
        );

        let old = json!({ "kind": "food", "expiresAtMs": 99 });
        let spot: GatherSpotSnapshot = serde_json::from_value(old).expect("legacy gather spot");
        assert_eq!(spot.purpose, GatherSpotPurpose::General);
        assert_eq!(spot.fish_population, None);

        let fishery = GatherSpotSnapshot {
            kind: ResourceKind::Fish,
            expires_at_ms: i64::MAX,
            purpose: GatherSpotPurpose::Fishing,
            fish_population: Some(FishPopulationSnapshot {
                stock: 7.5,
                capacity: 24.0,
                last_replenished_at_ms: 123,
            }),
        };
        let wire = serde_json::to_value(fishery).unwrap();
        assert_eq!(wire["kind"], json!("fish"));
        assert_eq!(wire["fishPopulation"]["stock"], json!(7.5));
        assert_eq!(
            serde_json::from_value::<GatherSpotSnapshot>(wire).unwrap(),
            fishery
        );
        assert_eq!(
            serde_json::to_value(CarryingKind::Fish).unwrap(),
            json!("fish")
        );
    }

    #[test]
    fn physical_transport_actions_and_snapshot_round_trip_typed_state() {
        let action = ClientAction::CreateTransportRoute {
            session_id: "session".to_owned(),
            nickname: "Conductor".to_owned(),
            sig: "signed".to_owned(),
            mode: TransportMode::Rail,
            source_stockpile_id: "source".to_owned(),
            destination_stockpile_id: "destination".to_owned(),
            resource: ResourceKind::Food,
            amount: 4.0,
            path: vec![TilePoint { x: 1, y: 2 }, TilePoint { x: 2, y: 2 }],
            cat_id: "cat-1".to_owned(),
            repeat: true,
        };
        let wire = serde_json::to_value(&action).unwrap();
        assert_eq!(wire["action"], json!("createTransportRoute"));
        assert_eq!(wire["sourceStockpileId"], json!("source"));
        assert_eq!(wire["mode"], json!("rail"));
        assert_eq!(
            serde_json::from_value::<ClientAction>(wire).unwrap(),
            action
        );

        let snapshot = TransportSnapshot {
            track_tiles: vec![TilePoint { x: 1, y: 2 }],
            docks: Vec::new(),
            vehicles: vec![TransportVehicleSnapshot {
                id: "wagon".to_owned(),
                mode: TransportMode::Rail,
                position: TilePoint { x: 1, y: 2 },
                crew_cat_id: Some("cat-1".to_owned()),
                cargo: 4.0,
            }],
            routes: vec![TransportRouteSnapshot {
                id: "route".to_owned(),
                mode: TransportMode::Rail,
                resource: ResourceKind::Food,
                amount: 4.0,
                phase: "outbound".to_owned(),
                repeat: true,
            }],
        };
        let wire = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(
            serde_json::from_value::<TransportSnapshot>(wire).unwrap(),
            snapshot
        );
    }
}
