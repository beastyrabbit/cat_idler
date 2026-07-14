//! Serde wire DTOs ported from `app/api/game/actions/route.ts`,
//! `server/game.ts` (`getGlobalDashboard`), and `hooks/useGameDashboard.ts`.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldSnapshot {
    pub now: i64,
    pub world_seed: i64,
    pub colonies: Vec<ColonySnapshot>,
    pub online_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColonySnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub name: String,
    pub status: ColonyStatus,
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
    pub vote_kick: Option<VoteKickSnapshot>,
    pub zones: Vec<ZoneSnapshot>,
    pub threat: ThreatSnapshot,
    pub raiders: Vec<RaiderSnapshot>,
    pub buildings: Vec<BuildingSnapshot>,
    pub claimed_tiles: Vec<TilePoint>,
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
    /// Traffic-formed dirt roads (`path_wear >= 70`) which have not been paved.
    /// They are distinct from authored stone roads and never form on stone ground.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dirt_road_tiles: Vec<TilePoint>,
    pub village_gate: Option<GatePlacement>,
    pub village_radius: u32,
    pub anchor: TilePoint,
    /// Appointed officers (role → cat id). P12.2; empty/absent when none appointed.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub officers: BTreeMap<OfficerRole, String>,
    /// On-map stockpiles (P12.3), including the shrine reservoir. Rendered as visible
    /// piles sized to contents. Empty/absent for pre-P12.3 snapshots.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stockpiles: Vec<StockpileSnapshot>,
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
    /// Crafted-item stacks the colony currently holds and could sell to the trader
    /// (empty while `state != trading`, since selling is only valid then). Mirrors
    /// [`ItemStackSnapshot`]'s `kind`/`material`/`quality` string convention.
    pub buy_offers: Vec<TraderBuyOffer>,
    /// Resource kinds the trader stocks and the colony could buy (constant across a
    /// visit; empty while `state != trading`).
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
}

/// One resource kind the trader will sell to the colony, and at what coin-per-unit price.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraderSellOffer {
    pub resource: ResourceKind,
    /// Coin the trader charges per unit.
    pub unit_price: f64,
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockLedgerSnapshot {
    /// Stock totals as last *reported* by the bookkeeper (may lag the true resources).
    pub reported: ResourceAmounts,
    /// Game-tick timestamp (ms) of the last recount.
    pub last_counted: i64,
    /// Whether the reported totals currently match the true resources exactly (a staffed
    /// Accounting Tent keeps this `true` every tick).
    pub accurate: bool,
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
    /// Present when this pile is a P16 gather spot: a temporary, single-resource drop
    /// point placeable outside the claimed village. Absent for the shrine reservoir and
    /// every general player stockpile, and for pre-P16 snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gather_spot: Option<GatherSpotSnapshot>,
}

/// A gather spot's P16-specific bookkeeping (see [`StockpileSnapshot::gather_spot`]):
/// the single resource it collects and when it expires.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatherSpotSnapshot {
    pub kind: ResourceKind,
    pub expires_at_ms: i64,
}

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
    Blessings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColonyStatus {
    Starting,
    Thriving,
    Struggling,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceAmounts {
    pub food: f64,
    pub water: f64,
    pub herbs: f64,
    #[serde(default)]
    pub catnip: f64,
    #[serde(default)]
    pub grain: f64,
    #[serde(default)]
    pub flour: f64,
    pub materials: f64,
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
    pub water: f64,
    pub herbs: f64,
    #[serde(default)]
    pub catnip: f64,
    #[serde(default)]
    pub grain: f64,
    #[serde(default)]
    pub flour: f64,
    pub materials: f64,
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
    pub specialization: Option<Specialization>,
    pub age_hours: f64,
    pub needs: CatNeeds,
    pub current_task: Option<String>,
    pub assigned_building_id: Option<String>,
    pub role_xp: RoleXp,
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
    /// Whether this cat is currently expecting a litter (mirrors `entities::Cat::
    /// is_pregnant`). Lets the census/inspector show an "expecting" state. Additive;
    /// defaults to `false` for older payloads.
    #[serde(default)]
    pub pregnant: bool,
    /// Stable permanent-bed status. Probationary cats are physically present and may
    /// work, but remain unhoused until a vacancy is allocated before their deadline.
    #[serde(default)]
    pub housing_status: CatHousingStatus,
    /// Remaining in-game minutes before an unhoused probationary arrival leaves.
    /// `None` for permanent residents and legacy snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probation_remaining_game_minutes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CatHousingStatus {
    #[default]
    Housed,
    Probationary,
    Unhoused,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarryingKind {
    Food,
    Blessings,
    Materials,
    Logs,
    Water,
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
    Forester,
    Farmer,
    Captain,
    Loremaster,
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
    Ritual,
    Quarry,
    /// Forest-site gathering job: fells and carries raw logs. This is the sole
    /// authoritative input source for the sawmill chain.
    GatherLogs,
    Explore,
    FetchWater,
    TrainWarrior,
    ExpandVillage,
    /// P12.6: haul-then-ritual offering — surplus materials converted to blessings
    /// at the shrine (complements, never duplicates, the leader's abstract `Tithe`).
    CarryOffering,
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
    /// 0.0..=1.0 through the current production cycle, for buildings that craft on a
    /// timer (workshop/benches/smithy). 0.0 for buildings with no active cycle,
    /// including fields (which add yield continuously rather than completing cycles)
    /// and non-producing buildings. Additive; defaults to 0.0 for older snapshots.
    #[serde(default)]
    pub production_progress: f64,
    /// Short, stable, lowercase label of what this building type makes (e.g. "plank",
    /// "refined", "weapon+armor"), or `None` if it doesn't produce a resource.
    /// Additive; empty/absent for pre-production-label snapshots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_output: Option<String>,
    /// Resource units physically in flight toward this building right now — the live sum
    /// of carried cargo whose haul destination resolves to this building's tile (see
    /// `cat_sim::world_tick::building_inbound_haul`). In practice only ever nonzero for
    /// the shrine: the sim's hauling model routes every haul to either a stockpile
    /// (player-designated pile or gather spot — plain rects, not buildings) or the shrine
    /// anchor fallback; no other building type is ever a haul target, since production
    /// buildings draw inputs straight from the colony's resource pool rather than from
    /// physically delivered cargo. Additive; defaults to 0.0 for older snapshots.
    #[serde(default)]
    pub inbound_haul: f64,
    /// Resource units physically departing this building in carried cargo. The
    /// current balancing-stockpile model normally reports zero; the field is present
    /// so a physical workshop-haul implementation can expose it without a wire break.
    #[serde(default)]
    pub outbound_haul: f64,
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
    WoodCutter,
    StonePrep,
    Woodworking,
    Clothier,
    Tannery,
    /// P17/P19 ore→metal chain: refines mountain ore into metal bars. NOTE: cat-client
    /// needs a `BuildingTexture`/sprite arm for this variant (`building_texture` and
    /// `building_label` in `crates/cat-client/src/lib.rs` are exhaustive matches and
    /// will fail to compile until one is added).
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

/// The purpose of a scout excursion. General exploration advances the nearest
/// reachable fog frontier; a resource mission picks the nearest useful,
/// unrevealed terrain target with deterministic tie-breaking.
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
    },
    UnlockNode {
        session_id: String,
        nickname: String,
        sig: String,
        node_id: String,
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
    },
    JoinVillage {
        colony_id: String,
        session_id: String,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
            buy_offers: vec![TraderBuyOffer {
                kind: "mug".to_string(),
                material: "wood".to_string(),
                quality: 1,
                available: 3,
                unit_price: 2.4,
            }],
            sell_offers: vec![TraderSellOffer {
                resource: ResourceKind::Food,
                unit_price: 1.5,
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
        };
        assert_eq!(
            serde_json::to_value(&ok).expect("serialize action result"),
            json!({ "ok": true })
        );

        let failed = ActionResult {
            ok: false,
            message: Some("Unknown action.".to_string()),
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
            colonies: vec![ColonySnapshot {
                id: "colony_1".to_string(),
                name: "Global Colony".to_string(),
                status: ColonyStatus::Thriving,
                resources: ResourceAmounts {
                    food: 50.0,
                    water: 40.0,
                    herbs: 5.0,
                    catnip: 0.0,
                    grain: 0.0,
                    flour: 0.0,
                    materials: 12.0,
                    refined: 3.0,
                    weapons: 2.0,
                    armor: 1.0,
                    planks: 0.0,
                    logs: 0.0,
                    lumber: 0.0,
                    blocks: 0.0,
                    tools: 0.0,
                    blessings: 8.0,
                },
                storage: StorageSnapshot {
                    capacities: ResourceCapacities {
                        food: 200.0,
                        water: 200.0,
                        herbs: 100.0,
                        catnip: 100.0,
                        grain: 100.0,
                        flour: 100.0,
                        materials: 100.0,
                        refined: 100.0,
                        weapons: 0.0,
                        armor: 0.0,
                        planks: 0.0,
                        logs: 100.0,
                        lumber: 100.0,
                        blocks: 0.0,
                        tools: 0.0,
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
                    }),
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
                    stats: CatStats { leadership: 9.0 },
                    death_time: None,
                    parent_ids: vec!["cat_0".to_string()],
                    parents: vec!["Ash".to_string()],
                    boosted: false,
                    pregnant: false,
                    housing_status: CatHousingStatus::Housed,
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
                }],
                claimed_tiles: vec![TilePoint { x: 6, y: 6 }],
                revealed_tiles: vec![TilePoint { x: 6, y: 6 }],
                provisional_tiles: vec![],
                road_tiles: vec![],
                dirt_road_tiles: vec![],
                village_gate: Some(GatePlacement {
                    x: 5,
                    y: 7,
                    side: GateSide::S,
                }),
                village_radius: 4,
                anchor: TilePoint { x: 6, y: 6 },
                officers: BTreeMap::new(),
                stockpiles: Vec::new(),
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
            gather_spot: None,
        });
        let encoded = serde_json::to_value(&snap).expect("serialize");
        assert_eq!(
            encoded["colonies"][0]["stockpiles"][0]["id"],
            json!("stockpile-shrine")
        );
        let round: WorldSnapshot = serde_json::from_value(encoded).expect("round-trip");
        assert_eq!(round.colonies[0].stockpiles, snap.colonies[0].stockpiles);
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
        };
        let encoded = serde_json::to_value(&stack).expect("serialize");
        assert_eq!(
            encoded,
            json!({
                "kind": "weapon",
                "material": "metal",
                "quality": 3,
                "count": 2,
                "value": 130
            })
        );
        let back: ItemStackSnapshot = serde_json::from_value(encoded).expect("deserialize");
        assert_eq!(back, stack);
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
        assert_eq!(decoded.production_progress, 0.0);
        assert_eq!(decoded.production_output, None);
        assert_eq!(decoded.inbound_haul, 0.0);
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
        cat.remove("probationRemainingGameMinutes");
        let back: WorldSnapshot = serde_json::from_value(old).expect("deserialize");
        assert_eq!(
            back.colonies[0].cats[0].housing_status,
            CatHousingStatus::Housed
        );
        assert_eq!(
            back.colonies[0].cats[0].probation_remaining_game_minutes,
            None
        );
    }
}
