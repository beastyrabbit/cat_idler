//! Cat colony Bevy client — top-down renderer for **Idle Cat Forest**.
//!
//! Connects to `cat-server` over a WebSocket, deserializes the live
//! [`cat_protocol::WorldSnapshot`] each tick, and renders it as a **flat,
//! single-level, top-down grid** (per `docs/GAME_VISION.md` — no isometric, no
//! z-layers). A world tile `(x, y)` maps to screen `Vec2::new(x * TILE, -y *
//! TILE)`. The view centres on the starter village anchor
//! ([`cat_sim::village_layout::VILLAGE_ANCHOR`]) and draws:
//!
//! - static terrain regenerated from `snapshot.world_seed` via `cat_sim`,
//! - cats (coloured by specialization, with a carried-item glyph),
//! - labelled buildings/workshops,
//! - a stockpile indicator near the shrine,
//! - avoid/gather zones,
//! - a HUD dashboard + event log, and clickable manual-action buttons that
//!   round-trip [`cat_protocol::ClientAction`] over the socket.

use bevy::asset::{AssetMetaCheck, RenderAssetUsages};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::{Anchor, BorderRect, SliceScaleMode, TextureSlicer};
use bevy::ui::RelativeCursorPosition;
use cat_protocol::{
    BuildingSnapshot, BuildingType, CarryingKind, CatActivity, CatNeeds, CatSnapshot, ClientAction,
    ColonySnapshot, EventSnapshot, FootprintSize, GateSide, ItemStackSnapshot, JobKind,
    OfficerRole, RaiderStatus, ResearchSnapshot, ResourceAmounts, ResourceCapacities, ResourceKind,
    RoleXp, Specialization, StockLedgerSnapshot, StockpileSnapshot, TilePoint, TraderBuyOffer,
    TraderSellOffer, TraderVisitState, WorldSnapshot, ZoneKind,
};
use cat_sim::climate::{Biome, ResourceHint};
use cat_sim::terrain_gen::{
    DecorationRole, RockSize, TerrainTile, WORLD_TERRAIN_OPTIONS, derive_biome_decoration,
    generate_terrain_chunk, tile_climate_biome,
};
use cat_sim::upgrade_tree::{UPGRADE_NODES, UpgradeNode};
use cat_sim::village_layout::VILLAGE_ANCHOR;
use cat_sim::world_gen::tile_to_chunk;
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use std::collections::{HashMap, HashSet};

/// Side length (world units) of one flat tile. Shrunk to ~1/3 of the original 28
/// so buildings read at a sensible size and more of the world fits on screen;
/// everything (terrain, footprint buildings, cats, trees, walls) scales off it.
const TILE: f32 = 10.0;
/// Half-width (in tiles) of the terrain window regenerated around the anchor.
const WINDOW_RADIUS: i32 = 30;
/// Starting (and R-reset) camera zoom, tuned to frame the village at the small
/// tile — a little zoomed in since there's now more world per screen.
const DEFAULT_ZOOM: f32 = 0.4;
/// Orthographic-scale zoom bounds (smaller scale = closer). `MIN_ZOOM` lets the
/// wheel push all the way in to individual-cat level; `MAX_ZOOM` frames the
/// whole known map.
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 3.0;

// Flat ground layers (terrain + ground markings) sit below the y-sorted world
// sprites; all strictly below the camera at Z=1000 so nothing is clipped.
const Z_TERRAIN: f32 = 0.0;
// Paved roads sit just above the base terrain tile (they replace its look) but
// below zone overlays and the y-sorted buildings/cats.
const Z_ROAD: f32 = 0.5;
const Z_ZONE: f32 = 2.0;
/// Building interior floor: above roads/zones, below every y-sorted world sprite
/// (cats/props at ~300) so cats always walk on top of it.
const Z_BUILDING_FLOOR: f32 = 3.0;

// Standing world sprites (buildings, walls, trees, stockpile piles, cats,
// raiders) share ONE y-sorted depth band: a sprite lower on the map (more
// negative world y) draws in front of one higher up — the whole 2.5D trick.
const Z_YSORT_BASE: f32 = 300.0;
const Z_YSORT_SCALE: f32 = 0.01;

// Fog of war sits above every y-sorted world sprite (opaque, so it hides the
// terrain, trees, buildings and cats on undiscovered tiles) but below the
// camera + UI.
const Z_FOG: f32 = 500.0;
const FOG_COLOR: Color = Color::srgb(0.02, 0.03, 0.05);
/// Half-lifted fog for tiles a currently-out scout has *tentatively* uncovered
/// (`provisional_tiles`): the same dark, but semi-transparent so terrain shows
/// through a haze — a visible "discovering, not yet delivered" state between
/// full fog and clear. Snaps to clear when the scout commits the tile, or back
/// to full fog if the scout dies (both driven by the stream).
const PROVISIONAL_FOG_COLOR: Color = Color::srgba(0.02, 0.03, 0.05, 0.55);

const CAMERA_Z: f32 = 1000.0;

/// Flat top-down projection: tile `(x, y)` → world space. Y is negated so the
/// grid reads top-down with north up.
fn grid_to_world(x: i32, y: i32) -> Vec2 {
    Vec2::new(x as f32 * TILE, -(y as f32) * TILE)
}

/// Draw depth for a standing sprite from its base (bottom/front-edge) world y:
/// lower on the map (more negative y) → larger z → drawn in front.
fn ysort_z(base_world_y: f32) -> f32 {
    Z_YSORT_BASE - base_world_y * Z_YSORT_SCALE
}

// ---- Corner minimap (pure coordinate mapping — unit-tested) ----

/// Minimap texture side, in pixels; one pixel per `tiles_per_px` world tiles.
const MINIMAP_PX: i32 = 128;
/// Cap on how many terrain chunks the minimap samples for biome colour per
/// rebuild — a huge revealed area beyond this shows as revealed-but-uncoloured
/// (grey) rather than stalling the frame. Not a truncation of the world, just of
/// the per-frame biome-colour sampling.
const MINIMAP_CHUNK_CAP: usize = 512;

/// How the revealed world maps onto the minimap texture: the tile at
/// `(origin_x, origin_y)` is the top-left minimap pixel, and each pixel spans
/// `tiles_per_px` tiles square.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct MinimapView {
    origin_x: i32,
    origin_y: i32,
    tiles_per_px: i32,
}

/// Inclusive tile bounding box `(min_x, min_y, max_x, max_y)` of the revealed
/// set — or a small box around the village anchor when nothing is revealed.
fn minimap_bounds(revealed: &[TilePoint]) -> (i32, i32, i32, i32) {
    if revealed.is_empty() {
        return (
            VILLAGE_ANCHOR.x - 8,
            VILLAGE_ANCHOR.y - 8,
            VILLAGE_ANCHOR.x + 8,
            VILLAGE_ANCHOR.y + 8,
        );
    }
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for t in revealed {
        min_x = min_x.min(t.x);
        min_y = min_y.min(t.y);
        max_x = max_x.max(t.x);
        max_y = max_y.max(t.y);
    }
    (min_x, min_y, max_x, max_y)
}

/// The minimap view that fits the revealed bounding box, centred, at the coarsest
/// integer tiles-per-pixel needed (1 while the explored area is ≤ 128 tiles).
fn minimap_view(revealed: &[TilePoint]) -> MinimapView {
    let (min_x, min_y, max_x, max_y) = minimap_bounds(revealed);
    let span = (max_x - min_x + 1).max(max_y - min_y + 1).max(1);
    let tiles_per_px = ((span + MINIMAP_PX - 1) / MINIMAP_PX).max(1);
    let center_x = (min_x + max_x) / 2;
    let center_y = (min_y + max_y) / 2;
    let half = MINIMAP_PX * tiles_per_px / 2;
    MinimapView {
        origin_x: center_x - half,
        origin_y: center_y - half,
        tiles_per_px,
    }
}

/// The minimap pixel a world tile falls on, or `None` if it's outside the view.
fn world_to_minimap(view: MinimapView, x: i32, y: i32) -> Option<(i32, i32)> {
    let px = (x - view.origin_x).div_euclid(view.tiles_per_px);
    let py = (y - view.origin_y).div_euclid(view.tiles_per_px);
    ((0..MINIMAP_PX).contains(&px) && (0..MINIMAP_PX).contains(&py)).then_some((px, py))
}

/// The world tile at the centre of a minimap pixel (for click-to-pan).
fn minimap_to_world(view: MinimapView, px: i32, py: i32) -> (i32, i32) {
    let half = view.tiles_per_px / 2;
    (
        view.origin_x + px * view.tiles_per_px + half,
        view.origin_y + py * view.tiles_per_px + half,
    )
}

/// The camera-viewport rectangle in minimap pixels `(x0, y0, x1, y1)` — the
/// half-open pixel box `[x0,x1) × [y0,y1)` covering the visible tile range,
/// clamped to the minimap so a partly-off view still shows an edge.
fn viewport_rect(
    view: MinimapView,
    min_tx: i32,
    min_ty: i32,
    max_tx: i32,
    max_ty: i32,
) -> (i32, i32, i32, i32) {
    let clamp = |v: i32| v.clamp(0, MINIMAP_PX);
    let to_px = |t: i32, origin: i32| (t - origin).div_euclid(view.tiles_per_px);
    (
        clamp(to_px(min_tx, view.origin_x)),
        clamp(to_px(min_ty, view.origin_y)),
        clamp(to_px(max_tx, view.origin_x) + 1),
        clamp(to_px(max_ty, view.origin_y) + 1),
    )
}

/// Fog colour for undiscovered / uncoloured minimap pixels.
const MINIMAP_FOG: [u8; 4] = [10, 12, 16, 255];

/// A biome's minimap pixel colour (its palette tint), matching the main view.
fn biome_rgba(biome: Biome) -> [u8; 4] {
    let [r, g, b] = biome.properties().tint;
    [r, g, b, 255]
}

/// Set the minimap pixel at `(px, py)` in an `MINIMAP_PX`-wide RGBA buffer.
fn put_pixel(buf: &mut [u8], px: i32, py: i32, color: [u8; 4]) {
    if !(0..MINIMAP_PX).contains(&px) || !(0..MINIMAP_PX).contains(&py) {
        return;
    }
    let i = ((py * MINIMAP_PX + px) * 4) as usize;
    buf[i..i + 4].copy_from_slice(&color);
}

/// Set a 2x2 block of minimap pixels so a mark stands out over 1px terrain.
fn put_block(buf: &mut [u8], px: i32, py: i32, color: [u8; 4]) {
    for dy in 0..2 {
        for dx in 0..2 {
            put_pixel(buf, px + dx, py + dy, color);
        }
    }
}

/// Biome colour for each revealed tile, sampling terrain per chunk (capped at
/// `MINIMAP_CHUNK_CAP` chunks/rebuild). Tiles in unsampled chunks are omitted
/// (drawn grey by the caller) rather than blocking the frame.
fn revealed_biomes(seed: i64, revealed: &[TilePoint]) -> HashMap<(i32, i32), Biome> {
    let mut chunks: HashSet<(i32, i32)> = HashSet::new();
    for t in revealed {
        let c = tile_to_chunk(t.x, t.y);
        chunks.insert((c.chunk_x, c.chunk_y));
    }
    let mut map = HashMap::new();
    for (cx, cy) in chunks.into_iter().take(MINIMAP_CHUNK_CAP) {
        for tile in generate_terrain_chunk(cx, cy, seed, WORLD_TERRAIN_OPTIONS) {
            map.insert((tile.x, tile.y), tile.climate_biome);
        }
    }
    map
}

/// The bottom-anchored base position (front-edge centre) and pixel size of a
/// building spanning its footprint (anchored at its NW-corner tile). `aspect` is
/// the sprite's native width/height; height follows it so the art isn't stretched.
// Retired by the cutaway-interior render (kept for the footprint unit test + a
// possible fallback while the interior look is under review).
#[allow(dead_code)]
fn footprint_sprite(nw: TilePoint, footprint: FootprintSize, aspect: f32) -> (Vec2, Vec2) {
    let w = footprint.width.max(1);
    let h = footprint.height.max(1);
    // Centre x across the footprint width; base y at the bottom of the front row.
    let center_x = (nw.x as f32 + (w as f32 - 1.0) / 2.0) * TILE;
    let front_row_y = nw.y + h - 1;
    let base_y = -(front_row_y as f32) * TILE - TILE / 2.0;
    let width_px = w as f32 * TILE;
    let size = Vec2::new(width_px, width_px / aspect);
    (Vec2::new(center_x, base_y), size)
}

/// The most recent snapshot pushed by the server.
#[derive(Resource, Default)]
struct LatestSnapshot(Option<WorldSnapshot>);

/// Signed session issued by the server after a `Presence` handshake; required
/// to send authenticated actions.
#[derive(Resource, Default)]
struct Session {
    session_id: String,
    sig: String,
    presence_sent: bool,
    ready: bool,
}

/// Outbound action queue drained onto the socket by [`flush_outgoing`].
#[derive(Resource, Default)]
struct OutgoingActions(Vec<ClientAction>);

/// The currently inspected cat (by id), re-resolved from the snapshot each tick.
#[derive(Resource, Default)]
struct Selection {
    selected: Option<String>,
}

/// The currently selected (non-shrine) stockpile id, for the remove affordance.
#[derive(Resource, Default)]
struct StockpileSelection {
    selected: Option<String>,
}

/// The currently inspected building id (middle-click).
#[derive(Resource, Default)]
struct BuildingSelection {
    selected: Option<String>,
}

/// What a mouse button selects on the map. Left = cat, right = building; middle
/// is reserved for drag-panning.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ClickTarget {
    Cat,
    Building,
}

fn click_action(button: MouseButton) -> Option<ClickTarget> {
    match button {
        MouseButton::Left => Some(ClickTarget::Cat),
        MouseButton::Right => Some(ClickTarget::Building),
        _ => None,
    }
}

/// Kind of inspectable a stacked-pick candidate is, in cycle order (cats sit on
/// top of buildings which sit on stockpiles).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PickKind {
    Cat,
    Building,
    Stockpile,
}

/// One inspectable found under the cursor for shift+right-click cycling.
#[derive(Clone, PartialEq, Eq, Debug)]
struct PickCandidate {
    id: String,
    kind: PickKind,
}

/// Given the ordered stack of inspectables under the cursor and the currently
/// selected id (of any kind), return the next candidate to cycle to, wrapping
/// around. Falls to the first when nothing is selected or the selection isn't in
/// the stack. `None` only when the stack is empty.
fn cycle_stacked_pick<'a>(
    stack: &'a [PickCandidate],
    current: Option<&str>,
) -> Option<&'a PickCandidate> {
    if stack.is_empty() {
        return None;
    }
    let idx = current
        .and_then(|c| stack.iter().position(|p| p.id == c))
        .map_or(0, |i| (i + 1) % stack.len());
    stack.get(idx)
}

/// The shrine reservoir's stockpile id — always present, de-emphasized in render.
const SHRINE_STOCKPILE_ID: &str = "stockpile-shrine";

/// Whether the officers panel is shown (toggled by `O`). Hidden by default
/// (`visible` = false) so it can't pile up on the HUD + event-log in the left
/// column on a normal-height window; appointment is also in the cat inspector.
#[derive(Resource, Default)]
struct OfficersUi {
    visible: bool,
}

/// Whether the announcements / event-log panel is open (toggled by `L`).
#[derive(Resource, Default)]
struct AnnouncementsUi {
    visible: bool,
}

/// Whether the corner minimap is shown (toggled by `M`; on by default).
#[derive(Resource)]
struct MinimapUi {
    visible: bool,
}

impl Default for MinimapUi {
    fn default() -> Self {
        Self { visible: true }
    }
}

/// Handle to the dynamic minimap texture (rewritten each snapshot), plus the last
/// view used to draw it (so click-to-pan / the viewport rect share the mapping).
#[derive(Resource)]
struct Minimap {
    image: Handle<Image>,
    view: MinimapView,
}

/// Number of announcement lines the panel shows (newest first).
const ANNOUNCEMENT_LINES: usize = 11;

/// Whether the goods / inventory panel is open (toggled by `G`).
#[derive(Resource, Default)]
struct GoodsUi {
    visible: bool,
}

/// Number of item-stack lines the goods panel shows (most valuable first).
const GOODS_LINES: usize = 12;

/// Whether the colony census / demographics panel is open (toggled by `C`).
#[derive(Resource, Default)]
struct CensusUi {
    visible: bool,
}

/// Number of text lines the census panel renders (see `census_report_lines`).
const CENSUS_LINES: usize = 18;

/// Whether the upgrade-tree panel is open (toggled by `U`).
#[derive(Resource, Default)]
struct UpgradeTreeUi {
    visible: bool,
}

/// Trade menu (open while a trader is at the gate). `closed` lets the player
/// dismiss it during a visit; it resets when the trader leaves so the next visit
/// auto-opens.
#[derive(Resource, Default)]
struct TradeUi {
    closed: bool,
}

/// Max sell rows (crafted stacks) and buy rows (resource kinds) the menu shows.
const TRADE_SELL_ROWS: usize = 6;
const TRADE_BUY_ROWS: usize = 8;

/// The five appointable officer roles, in display order.
const ALL_OFFICER_ROLES: [OfficerRole; 5] = [
    OfficerRole::Steward,
    OfficerRole::Forester,
    OfficerRole::Farmer,
    OfficerRole::Captain,
    OfficerRole::Loremaster,
];

fn officer_role_name(role: OfficerRole) -> &'static str {
    match role {
        OfficerRole::Steward => "Steward",
        OfficerRole::Forester => "Forester",
        OfficerRole::Farmer => "Farmer",
        OfficerRole::Captain => "Captain",
        OfficerRole::Loremaster => "Loremaster",
    }
}

/// Label for the cat-inspector boost toggle, driven by the cat's current
/// `boosted` flag. Off invites the player to prioritise the cat; on shows the
/// god-power is active and how to release it. The leading star matches the
/// on-map marker so the two read as the same affordance.
fn boost_button_label(boosted: bool) -> &'static str {
    if boosted {
        "★ Boosted (click to clear)"
    } else {
        "★ Boost"
    }
}

/// Active map tool, any in-progress drag, and the accept-type the next stockpile
/// will be designated with.
#[derive(Resource, Default)]
struct Tools {
    mode: ToolMode,
    /// `(start_tile, current_tile)` while dragging a zone rectangle.
    drag: Option<((i32, i32), (i32, i32))>,
    accept: AcceptChoice,
}

/// What a newly designated stockpile accepts: everything, or one resource.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum AcceptChoice {
    #[default]
    General,
    Only(ResourceKind),
}

impl AcceptChoice {
    /// Cycle: General -> each storable kind in order -> back to General.
    fn next(self) -> Self {
        match self {
            Self::General => Self::Only(STORABLE_KINDS[0]),
            Self::Only(kind) => {
                let idx = STORABLE_KINDS.iter().position(|&k| k == kind).unwrap_or(0);
                STORABLE_KINDS
                    .get(idx + 1)
                    .map_or(Self::General, |&next| Self::Only(next))
            }
        }
    }

    /// The accept-set to send in DesignateStockpile.
    fn kinds(self) -> Vec<ResourceKind> {
        match self {
            Self::General => STORABLE_KINDS.to_vec(),
            Self::Only(kind) => vec![kind],
        }
    }

    /// Short label for the picker button.
    fn label(self) -> String {
        match self {
            Self::General => "General".to_string(),
            Self::Only(kind) => format!("{} only", resource_kind_name(kind)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum ToolMode {
    #[default]
    Inspect,
    AvoidZone,
    GatherZone,
    Stockpile,
}

/// What a click-drag paints — a steering zone or a stockpile designation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PaintKind {
    Avoid,
    Gather,
    Stockpile,
}

impl ToolMode {
    fn label(self) -> &'static str {
        match self {
            Self::Inspect => "Inspect",
            Self::AvoidZone => "Avoid zone",
            Self::GatherZone => "Gather zone",
            Self::Stockpile => "Stockpile",
        }
    }

    /// What this mode paints on drag, if anything.
    fn paint_kind(self) -> Option<PaintKind> {
        match self {
            Self::Inspect => None,
            Self::AvoidZone => Some(PaintKind::Avoid),
            Self::GatherZone => Some(PaintKind::Gather),
            Self::Stockpile => Some(PaintKind::Stockpile),
        }
    }
}

/// Tracks one-time terrain spawn (terrain is static per `world_seed`).
#[derive(Resource, Default)]
struct WorldRender {
    terrain_spawned: bool,
}

/// Pixel-art terrain + nature texture handles, loaded once at startup.
#[derive(Resource, Clone)]
struct TerrainArt {
    grass: Handle<Image>,
    grass_var: Handle<Image>,
    dirt: Handle<Image>,
    farmland: Handle<Image>,
    rocky: Handle<Image>,
    sand: Handle<Image>,
    snow: Handle<Image>,
    flowers_red: Handle<Image>,
    flowers_white: Handle<Image>,
    flowers_blue: Handle<Image>,
    water: Handle<Image>,
    water_edge: Handle<Image>,
    tree_oak: Handle<Image>,
    tree_pine: Handle<Image>,
    tree_snow_pine: Handle<Image>,
    tree_dead: Handle<Image>,
    rock: Handle<Image>,
}

impl TerrainArt {
    fn load(assets: &AssetServer) -> Self {
        Self {
            grass: assets.load("public/images/game/terrain/grass.png"),
            grass_var: assets.load("public/images/game/terrain/grass_var.png"),
            dirt: assets.load("public/images/game/terrain/dirt.png"),
            farmland: assets.load("public/images/game/terrain/farmland.png"),
            rocky: assets.load("public/images/game/terrain/rocky.png"),
            // The `highland` sheet tile is tan/sand — used for sandy biomes.
            sand: assets.load("public/images/game/terrain/highland.png"),
            snow: assets.load("public/images/game/terrain/snow.png"),
            flowers_red: assets.load("public/images/game/terrain/flowers_red.png"),
            flowers_white: assets.load("public/images/game/terrain/flowers_white.png"),
            flowers_blue: assets.load("public/images/game/terrain/flowers_blue.png"),
            water: assets.load("public/images/game/terrain/water.png"),
            water_edge: assets.load("public/images/game/terrain/water_edge.png"),
            tree_oak: assets.load("public/images/game/nature/tree_oak.png"),
            tree_pine: assets.load("public/images/game/nature/tree_pine.png"),
            tree_snow_pine: assets.load("public/images/game/nature/tree_snow_pine.png"),
            tree_dead: assets.load("public/images/game/nature/tree_dead.png"),
            rock: assets.load("public/images/game/props/stone_pile.png"),
        }
    }

    /// The nature sprite + its height in tiles for a biome's trees.
    /// A top-down canopy sprite + the square-size multiplier (× TILE) to render
    /// it at. All are 16×16 canopies-from-above (Kenney Roguelike, CC0).
    fn tree(&self, sprite: TreeSprite) -> (Handle<Image>, f32) {
        match sprite {
            TreeSprite::Oak => (self.tree_oak.clone(), 1.6),
            TreeSprite::Pine => (self.tree_pine.clone(), 1.5),
            TreeSprite::SnowPine => (self.tree_snow_pine.clone(), 1.5),
            TreeSprite::DeadTree => (self.tree_dead.clone(), 1.3),
        }
    }

    fn ground(&self, texture: GroundTexture) -> Handle<Image> {
        match texture {
            GroundTexture::Grass => self.grass.clone(),
            GroundTexture::GrassVar => self.grass_var.clone(),
            GroundTexture::Dirt => self.dirt.clone(),
            GroundTexture::Farmland => self.farmland.clone(),
            GroundTexture::Rocky => self.rocky.clone(),
            GroundTexture::Sand => self.sand.clone(),
            GroundTexture::Snow => self.snow.clone(),
            GroundTexture::FlowersRed => self.flowers_red.clone(),
            GroundTexture::FlowersWhite => self.flowers_white.clone(),
            GroundTexture::FlowersBlue => self.flowers_blue.clone(),
        }
    }
}

/// Ground texture chosen for a (non-water) tile from its biome.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum GroundTexture {
    Grass,
    GrassVar,
    Dirt,
    Farmland,
    Rocky,
    Sand,
    Snow,
    FlowersRed,
    FlowersWhite,
    FlowersBlue,
}

/// Pixel-art building sprite handles, loaded once at startup.
#[derive(Resource, Clone)]
// The building-sprite handles + handle() are retired by the cutaway-interior
// render (floor + prop); kept behind allow(dead_code) while that look is under
// review, to be removed once confirmed.
#[allow(dead_code)]
struct BuildingArt {
    shrine: Handle<Image>,
    den: Handle<Image>,
    workshop: Handle<Image>,
    smithy: Handle<Image>,
    research_hut: Handle<Image>,
    school: Handle<Image>,
    barracks: Handle<Image>,
    storehouse: Handle<Image>,
    market: Handle<Image>,
    well: Handle<Image>,
    wood_cutter: Handle<Image>,
    stone_prep: Handle<Image>,
    woodworking: Handle<Image>,
    clothier: Handle<Image>,
    tannery: Handle<Image>,
    // Cutaway top-down interior tiles: a footprint-filling floor + a centred
    // workstation prop, viewed straight from above (no roofs). CC0 Kenney
    // Roguelike.
    floor_wood: Handle<Image>,
    floor_stone: Handle<Image>,
    prop_workbench: Handle<Image>,
    prop_bed: Handle<Image>,
    prop_crate: Handle<Image>,
    prop_furnace: Handle<Image>,
    prop_altar: Handle<Image>,
}

impl BuildingArt {
    fn load(assets: &AssetServer) -> Self {
        Self {
            shrine: assets.load("public/images/game/buildings/shrine.png"),
            den: assets.load("public/images/game/buildings/den.png"),
            workshop: assets.load("public/images/game/buildings/workshop.png"),
            smithy: assets.load("public/images/game/buildings/smithy.png"),
            research_hut: assets.load("public/images/game/buildings/research_hut.png"),
            school: assets.load("public/images/game/buildings/school.png"),
            barracks: assets.load("public/images/game/buildings/barracks.png"),
            storehouse: assets.load("public/images/game/buildings/storehouse.png"),
            market: assets.load("public/images/game/buildings/market.png"),
            well: assets.load("public/images/game/props/well.png"),
            wood_cutter: assets.load("public/images/game/buildings/wood_cutter.png"),
            stone_prep: assets.load("public/images/game/buildings/stone_prep.png"),
            woodworking: assets.load("public/images/game/buildings/woodworking.png"),
            clothier: assets.load("public/images/game/buildings/clothier.png"),
            tannery: assets.load("public/images/game/buildings/tannery.png"),
            floor_wood: assets.load("public/images/game/interior/floor_wood.png"),
            floor_stone: assets.load("public/images/game/interior/floor_stone.png"),
            prop_workbench: assets.load("public/images/game/interior/workbench.png"),
            prop_bed: assets.load("public/images/game/interior/bed.png"),
            prop_crate: assets.load("public/images/game/props/crate.png"),
            prop_furnace: assets.load("public/images/game/interior/furnace.png"),
            prop_altar: assets.load("public/images/game/interior/altar.png"),
        }
    }

    fn floor(&self, kind: FloorKind) -> Handle<Image> {
        match kind {
            FloorKind::Wood => self.floor_wood.clone(),
            FloorKind::Stone => self.floor_stone.clone(),
        }
    }

    /// A prop handle + its native tile footprint (width, height) so the render can
    /// size it without stretching (workbench/bed are 2×1, crate 1×1).
    fn prop(&self, prop: InteriorProp) -> Option<(Handle<Image>, Vec2)> {
        match prop {
            InteriorProp::Workbench => Some((self.prop_workbench.clone(), Vec2::new(2.0, 1.0))),
            InteriorProp::Bed => Some((self.prop_bed.clone(), Vec2::new(2.0, 1.0))),
            InteriorProp::Crate => Some((self.prop_crate.clone(), Vec2::new(1.0, 1.0))),
            InteriorProp::Furnace => Some((self.prop_furnace.clone(), Vec2::new(1.3, 1.3))),
            InteriorProp::Altar => Some((self.prop_altar.clone(), Vec2::new(1.3, 1.3))),
            InteriorProp::None => None,
        }
    }

    #[allow(dead_code)]
    fn handle(&self, texture: BuildingTexture) -> Handle<Image> {
        match texture {
            BuildingTexture::Shrine => self.shrine.clone(),
            BuildingTexture::Den => self.den.clone(),
            BuildingTexture::Workshop => self.workshop.clone(),
            BuildingTexture::Smithy => self.smithy.clone(),
            BuildingTexture::ResearchHut => self.research_hut.clone(),
            BuildingTexture::School => self.school.clone(),
            BuildingTexture::Barracks => self.barracks.clone(),
            BuildingTexture::Storehouse => self.storehouse.clone(),
            BuildingTexture::Market => self.market.clone(),
            BuildingTexture::Well => self.well.clone(),
            BuildingTexture::WoodCutter => self.wood_cutter.clone(),
            BuildingTexture::StonePrep => self.stone_prep.clone(),
            BuildingTexture::Woodworking => self.woodworking.clone(),
            BuildingTexture::Clothier => self.clothier.clone(),
            BuildingTexture::Tannery => self.tannery.clone(),
        }
    }
}

/// The building sprite a [`BuildingType`] renders as. Sprites `mill`, `monument`,
/// `tent`, `town_hall` are reserved for future building types.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BuildingTexture {
    Shrine,
    Den,
    Workshop,
    Smithy,
    ResearchHut,
    School,
    Barracks,
    Storehouse,
    Market,
    Well,
    WoodCutter,
    StonePrep,
    Woodworking,
    Clothier,
    Tannery,
}

/// Pixel-art prop sprites used for stockpile piles, loaded once at startup.
#[derive(Resource, Clone)]
struct PropArt {
    sack: Handle<Image>,
    barrel: Handle<Image>,
    haystack: Handle<Image>,
    stone_pile: Handle<Image>,
    gold_pile: Handle<Image>,
    crate_box: Handle<Image>,
}

impl PropArt {
    fn load(assets: &AssetServer) -> Self {
        Self {
            sack: assets.load("public/images/game/props/sack.png"),
            barrel: assets.load("public/images/game/props/barrel.png"),
            haystack: assets.load("public/images/game/props/haystack.png"),
            stone_pile: assets.load("public/images/game/props/stone_pile.png"),
            gold_pile: assets.load("public/images/game/props/gold_pile.png"),
            crate_box: assets.load("public/images/game/props/crate.png"),
        }
    }

    fn pile(&self, texture: PropTexture) -> Handle<Image> {
        match texture {
            PropTexture::Sack => self.sack.clone(),
            PropTexture::Barrel => self.barrel.clone(),
            PropTexture::Haystack => self.haystack.clone(),
            PropTexture::StonePile => self.stone_pile.clone(),
            PropTexture::GoldPile => self.gold_pile.clone(),
            PropTexture::Crate => self.crate_box.clone(),
        }
    }
}

/// Infra sprites (village palisade + gate), loaded once at startup.
#[derive(Resource, Clone)]
struct InfraArt {
    palisade: Handle<Image>,
    gate: Handle<Image>,
    road_cross: Handle<Image>,
    road_h: Handle<Image>,
    road_v: Handle<Image>,
}

impl InfraArt {
    fn load(assets: &AssetServer) -> Self {
        Self {
            palisade: assets.load("public/images/game/infra/palisade.png"),
            gate: assets.load("public/images/game/infra/gate_open.png"),
            road_cross: assets.load("public/images/game/infra/road_cross.png"),
            road_h: assets.load("public/images/game/infra/road_straight_h.png"),
            road_v: assets.load("public/images/game/infra/road_straight_v.png"),
        }
    }

    fn road(&self, sprite: RoadSprite) -> Handle<Image> {
        match sprite {
            RoadSprite::Cross => self.road_cross.clone(),
            RoadSprite::StraightH => self.road_h.clone(),
            RoadSprite::StraightV => self.road_v.clone(),
        }
    }
}

/// The oriented road tile to draw, chosen from a tile's road neighbours.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RoadSprite {
    Cross,
    StraightH,
    StraightV,
}

/// Pick a road sprite from which orthogonal neighbours are also road: tiles that
/// connect on both axes (or stand alone) read as a cross; a single-axis run uses
/// the matching straight. Covers the blueprint's shrine-to-walls cross cleanly.
fn road_sprite_kind(n: bool, s: bool, e: bool, w: bool) -> RoadSprite {
    let vertical = n || s;
    let horizontal = e || w;
    match (vertical, horizontal) {
        (true, false) => RoadSprite::StraightV,
        (false, true) => RoadSprite::StraightH,
        // Both axes (the cross centre) or a lone tile default to the cross.
        _ => RoadSprite::Cross,
    }
}

/// DF-Steam UI kit (Kenney Adventure): wood/parchment 9-patch panel, hanging
/// banner header, wood button. Loaded once at startup.
#[derive(Resource, Clone)]
struct UiArt {
    panel: Handle<Image>,
    banner: Handle<Image>,
    button: Handle<Image>,
}

impl UiArt {
    fn load(assets: &AssetServer) -> Self {
        Self {
            panel: assets.load("public/images/game/ui/panel.png"),
            banner: assets.load("public/images/game/ui/banner.png"),
            button: assets.load("public/images/game/ui/button.png"),
        }
    }
}

/// Dark ink for text over the cream parchment panels.
const PARCHMENT_INK: Color = Color::srgb(0.24, 0.15, 0.07);
/// Wood-border inset (source px) for the 128px panel / 96x48 button 9-patches.
const PANEL_BORDER: f32 = 22.0;
const BUTTON_BORDER: f32 = 12.0;

// Wood-button tint states (multiply the sprite): idle / hover / pressed /
// active-toggle. Kenney ships one button sprite; states are tints.
const BTN_IDLE: Color = Color::srgb(1.0, 1.0, 1.0);
const BTN_HOVER: Color = Color::srgb(0.86, 0.86, 0.82);
const BTN_PRESS: Color = Color::srgb(0.70, 0.66, 0.58);
const BTN_ACTIVE: Color = Color::srgb(1.0, 0.82, 0.45);

/// A 9-patch panel/button background from a wood-frame sprite: the border stays
/// crisp while the parchment centre stretches to fill the node.
fn sliced_image(image: Handle<Image>, border: f32) -> ImageNode {
    ImageNode {
        image,
        image_mode: NodeImageMode::Sliced(TextureSlicer {
            border: BorderRect::all(border),
            center_scale_mode: SliceScaleMode::Stretch,
            sides_scale_mode: SliceScaleMode::Stretch,
            max_corner_scale: 1.0,
        }),
        ..default()
    }
}

/// Animated character sheets (cats + raiders) and specialization hats. The
/// sheets are 8 direction groups x 4 walk frames in 32x64 cells, one row of 32.
#[derive(Resource, Clone)]
struct SpriteSheets {
    cat: Handle<Image>,
    raider: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    hat_hunter: Handle<Image>,
    hat_architect: Handle<Image>,
    hat_ritualist: Handle<Image>,
    hat_warrior: Handle<Image>,
}

impl SpriteSheets {
    fn load(assets: &AssetServer, layouts: &mut Assets<TextureAtlasLayout>) -> Self {
        let layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::new(32, 64),
            32,
            1,
            None,
            None,
        ));
        Self {
            cat: assets.load("public/images/cats/cat-sheet.png"),
            raider: assets.load("public/images/cats/raider-sheet.png"),
            layout,
            hat_hunter: assets.load("public/images/cats/hat-hunter.png"),
            hat_architect: assets.load("public/images/cats/hat-architect.png"),
            hat_ritualist: assets.load("public/images/cats/hat-ritualist.png"),
            hat_warrior: assets.load("public/images/cats/hat-warrior.png"),
        }
    }

    fn hat(&self, spec: Specialization) -> Handle<Image> {
        match spec {
            Specialization::Hunter => self.hat_hunter.clone(),
            Specialization::Architect => self.hat_architect.clone(),
            Specialization::Ritualist => self.hat_ritualist.clone(),
            Specialization::Warrior => self.hat_warrior.clone(),
        }
    }
}

/// The prop sprite a stockpile's dominant resource renders as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PropTexture {
    Sack,
    Barrel,
    Haystack,
    StonePile,
    GoldPile,
    Crate,
}

/// The live WebSocket connection (kept off the render threads — the receiver is
/// `!Sync`).
struct WsConn {
    sender: WsSender,
    receiver: WsReceiver,
}

/// A persistent cat body sprite, keyed by cat id so it survives snapshots and
/// glides toward its target tile.
#[derive(Component)]
struct CatBody(String);
/// A persistent raider body sprite (its id lives in [`RaiderBodies`]).
#[derive(Component)]
struct RaiderBody;
/// The visiting-trader body sprite (a merchant cat), present only while
/// `ColonySnapshot.trader` is Some.
#[derive(Component)]
struct TraderBody;
/// The world-space (x, y) a body is gliding toward (its current target tile).
#[derive(Component)]
struct MoveTarget(Vec2);
/// A per-cat overlay (hat / carried item / selection ring) rebuilt each sync;
/// it tracks its cat's live sprite position each frame via [`FollowCat`].
#[derive(Component)]
struct CatOverlay;
/// Makes an overlay follow a cat body's interpolated position (+ a local offset).
#[derive(Component)]
struct FollowCat {
    id: String,
    offset: Vec3,
}
/// A sheet-animated character (cat or raider): its 8-way facing group and
/// whether it's moving (walk-cycled) or idle (frame 0).
#[derive(Component)]
struct AnimSprite {
    group: usize,
    moving: bool,
}

/// Live cat body entities keyed by cat id (persist across snapshots).
#[derive(Resource, Default)]
struct CatBodies(HashMap<String, Entity>);
/// Live raider body entities keyed by raider id.
#[derive(Resource, Default)]
struct RaiderBodies(HashMap<String, Entity>);
/// Marker for the cat-inspector panel node (shown only when a cat is selected).
#[derive(Component)]
struct InspectorPanel;
/// Marker for the cat-inspector text.
#[derive(Component)]
struct InspectorText;
/// A need shown as a bar in the cat inspector.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NeedKind {
    Hunger,
    Thirst,
    Rest,
    Health,
}
/// The four cat needs and their bar labels, in display order.
const CAT_NEEDS: [(NeedKind, &str); 4] = [
    (NeedKind::Hunger, "hunger"),
    (NeedKind::Thirst, "thirst"),
    (NeedKind::Rest, "rest"),
    (NeedKind::Health, "health"),
];
/// Tags a need-bar fill node so the inspector can resize/recolor it each tick.
#[derive(Component, Clone, Copy)]
struct NeedBar(NeedKind);
/// Marker for the building-inspector panel node (middle-click a building).
#[derive(Component)]
struct BuildingInspectorPanel;
/// Marker for the building-inspector text.
#[derive(Component)]
struct BuildingInspectorText;
/// Marker for a building marker sprite.
#[derive(Component)]
struct BuildingSprite;
/// Marker for a zone overlay tile.
#[derive(Component)]
struct ZoneSprite;
/// Marker for a village wall/gate segment sprite (redrawn when the claimed set
/// changes).
#[derive(Component)]
struct WallVis;

/// A perimeter edge side of a claimed tile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum WallSide {
    N,
    E,
    S,
    W,
}
/// Marker for stockpile visuals (overlay rect + pile prop + label), redrawn each
/// snapshot.
#[derive(Component)]
struct StockpileVis;
/// Marker for the outline behind a selected stockpile.
#[derive(Component)]
struct StockpileHighlight;
/// Marker for the "Remove stockpile" panel node (shown when one is selected).
#[derive(Component)]
struct RemovePanel;
/// Marker for the "Remove stockpile" panel's description text.
#[derive(Component)]
struct RemovePanelText;
/// Marker for the "Remove stockpile" button.
#[derive(Component)]
struct RemoveStockpileButton;
/// Marker for the officers panel node (toggled with `O`).
#[derive(Component)]
struct OfficersPanel;
/// One officer role row in the officers panel (its text holder).
#[derive(Component, Clone, Copy)]
struct OfficerRow(OfficerRole);
/// "Vacate" button for a role in the officers panel.
#[derive(Component, Clone, Copy)]
struct VacateButton(OfficerRole);
/// "Appoint <role>" button in the cat inspector.
#[derive(Component, Clone, Copy)]
struct AppointButton(OfficerRole);
/// The cat-inspector "Boost" toggle button (flips the selected cat's priority
/// flag via `BoostCat`).
#[derive(Component)]
struct BoostButton;
/// The label text inside the boost button, repainted live from `boosted`.
#[derive(Component)]
struct BoostButtonText;
/// Marker for the HUD colony header text (name / leader / pop / threat).
#[derive(Component)]
struct HudHeaderText;
/// Marker for the HUD jobs + ledger footer text.
#[derive(Component)]
struct HudFooterText;
/// Marker for a fog-of-war tile sprite.
#[derive(Component)]
struct FogTile;
/// Marker for a paved-road tile sprite.
#[derive(Component)]
struct RoadTile;
/// Marker for the cursor-following hover tooltip panel.
#[derive(Component)]
struct TooltipPanel;
/// Marker for the hover tooltip's text.
#[derive(Component)]
struct TooltipText;
/// Marker for the event-log text.
#[derive(Component)]
struct EventLogText;
/// Marker for the announcements panel node (toggled open/closed).
#[derive(Component)]
struct AnnouncementsPanel;
/// One announcement line slot (index 0 = newest at top).
#[derive(Component, Clone, Copy)]
struct AnnouncementLine(usize);
/// The HUD button that toggles the announcements panel.
#[derive(Component)]
struct AnnouncementsButton;
/// The compact "latest announcement" ticker line on the HUD.
#[derive(Component)]
struct AnnouncementTicker;
/// Marker for the goods / inventory panel node (toggled open/closed).
#[derive(Component)]
struct GoodsPanel;
/// One goods line slot (index 0 = most valuable stack at top).
#[derive(Component, Clone, Copy)]
struct GoodsLine(usize);
/// The per-kind glyph node for a goods line (tinted by material).
#[derive(Component, Clone, Copy)]
struct GoodsLineIcon(usize);
/// The treasury-total line at the top of the goods panel.
#[derive(Component)]
struct GoodsTreasury;
/// The HUD button that toggles the goods panel.
#[derive(Component)]
struct GoodsButton;
/// Marker for the colony census / demographics panel node.
#[derive(Component)]
struct CensusPanel;
/// One census text-line slot (index 0 at the top).
#[derive(Component, Clone, Copy)]
struct CensusLine(usize);
/// The HUD button that toggles the census panel.
#[derive(Component)]
struct CensusButton;
/// Marker for the upgrade-tree panel node.
#[derive(Component)]
struct TreePanel;
/// The tree header line showing both currency balances.
#[derive(Component)]
struct TreeCurrencyText;
/// The tree header line showing the next auto-unlock target.
#[derive(Component)]
struct TreeNextText;
/// A tech node's label text, carrying its node id (coloured by state).
#[derive(Component, Clone, Copy)]
struct TreeNodeText(&'static str);
/// A tech node's god-purchase button, carrying its node id.
#[derive(Component, Clone, Copy)]
struct TreeBuyButton(&'static str);
/// The HUD button that toggles the upgrade-tree panel.
#[derive(Component)]
struct TreeButton;
/// Marker for the trade-menu panel node.
#[derive(Component)]
struct TradeMenuPanel;
/// The coin readout in the trade menu.
#[derive(Component)]
struct TradeCoinText;
/// A sell row (container node, hidden when there's no offer at this index).
#[derive(Component, Clone, Copy)]
struct SellRow(usize);
/// The label text of a sell row.
#[derive(Component, Clone, Copy)]
struct SellRowText(usize);
/// A sell button: sells the offer at `row` (all of it when `all`, else one).
#[derive(Component, Clone, Copy)]
struct SellButton {
    row: usize,
    all: bool,
}
/// A buy row (container node, hidden when there's no offer at this index).
#[derive(Component, Clone, Copy)]
struct BuyRow(usize);
/// The label text of a buy row.
#[derive(Component, Clone, Copy)]
struct BuyRowText(usize);
/// A buy button: buys one unit of the resource offered at `row`.
#[derive(Component, Clone, Copy)]
struct BuyButton(usize);
/// The trade-menu close button.
#[derive(Component)]
struct TradeCloseButton;
/// Marker for the corner minimap panel node (toggled open/closed).
#[derive(Component)]
struct MinimapPanel;
/// Marker for the minimap image node (for click-to-pan hit-testing).
#[derive(Component)]
struct MinimapImageNode;
/// Marker for the camera-viewport outline drawn over the minimap.
#[derive(Component)]
struct MinimapViewportRect;

/// The kind of a colony event, inferred from its message (the snapshot carries
/// only message + timestamp), for colour + glyph coding in the announcements log.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EventKind {
    Birth,
    Death,
    Raid,
    Crisis,
    Election,
    Progress,
    Neutral,
}

/// Classify an event into a display bucket from its exact wire `kind`
/// (`EventSnapshot.kind`, snake_case categories from `cat_sim`'s `wire_kind`),
/// not by guessing at the message text. Death and raid share the red treatment;
/// crisis recoveries read as positive progress.
fn event_kind_of(kind: &str) -> EventKind {
    match kind {
        "birth" | "conception" => EventKind::Birth,
        // "death_<cause>" incl. death_raid (a defender lost in a raid).
        k if k.starts_with("death") => EventKind::Death,
        // "raid_<phase>" incl. raid_lost / raid_wipeout — all red-adjacent.
        k if k.starts_with("raid") => EventKind::Raid,
        "water_crisis" | "dehydration_crisis" => EventKind::Crisis,
        // Recoveries get the softer positive treatment, not the amber crisis one.
        "water_recovered" | "dehydration_recovery" => EventKind::Progress,
        "leader_change" => EventKind::Election,
        k if k.starts_with("election") => EventKind::Election,
        "research_unlocked" | "node_owned" | "warrior_trained" | "village_founded"
        | "village_expanded" | "road_built" | "production" | "discovery" | "forest_chopped"
        | "tithe" | "offering" | "blessing_delivered" => EventKind::Progress,
        // "trade_sell"/"trade_buy" (note the underscore — not "trader_*" lifecycle).
        k if k.starts_with("trade_") => EventKind::Progress,
        // trader lifecycle, jobs, rituals, resets, empty (pre-taxonomy) → neutral.
        _ => EventKind::Neutral,
    }
}

/// Aggregated colony demographics for the census panel — a pure function of the
/// snapshot's cats + events. Life stage is derived from `age_hours` (the snapshot
/// carries no stage field); pregnancies are intentionally absent because the wire
/// `CatSnapshot` exposes no pregnancy datum (flagged to the team-lead).
#[derive(Debug, Clone, PartialEq, Default)]
struct Census {
    total: u32,
    kittens: u32,
    young: u32,
    adults: u32,
    elders: u32,
    hunters: u32,
    architects: u32,
    ritualists: u32,
    warriors: u32,
    unspecialized: u32,
    boosted: u32,
    expecting: u32,
    avg_age_hours: f64,
    births: u32,
    deaths: u32,
    leader: Option<String>,
}

/// Recent births + deaths counted from the event feed by exact wire `kind`:
/// `birth` (so conceptions don't inflate it) and any `death_*` cause.
fn count_vital_events(events: &[EventSnapshot]) -> (u32, u32) {
    let mut births = 0;
    let mut deaths = 0;
    for event in events {
        if event.kind == "birth" {
            births += 1;
        } else if event.kind.starts_with("death") {
            deaths += 1;
        }
    }
    (births, deaths)
}

/// Tally colony demographics from the living cats, the event feed, and the leader
/// name. Cats with a `death_time` are excluded. Life-stage thresholds mirror the
/// sim's `age::get_life_stage` (kitten <6h, young <24h, adult <48h, elder ≥48h);
/// a non-finite age falls through to elder, matching the sim.
fn colony_census(cats: &[CatSnapshot], events: &[EventSnapshot], leader: Option<&str>) -> Census {
    let mut c = Census {
        leader: leader.map(str::to_string),
        ..Default::default()
    };
    let mut age_sum = 0.0;
    for cat in cats.iter().filter(|k| k.death_time.is_none()) {
        c.total += 1;
        age_sum += cat.age_hours;
        let age = cat.age_hours;
        if age < 6.0 {
            c.kittens += 1;
        } else if age < 24.0 {
            c.young += 1;
        } else if age < 48.0 {
            c.adults += 1;
        } else {
            c.elders += 1;
        }
        match cat.specialization {
            Some(Specialization::Hunter) => c.hunters += 1,
            Some(Specialization::Architect) => c.architects += 1,
            Some(Specialization::Ritualist) => c.ritualists += 1,
            Some(Specialization::Warrior) => c.warriors += 1,
            None => c.unspecialized += 1,
        }
        if cat.boosted {
            c.boosted += 1;
        }
        if cat.pregnant {
            c.expecting += 1;
        }
    }
    c.avg_age_hours = if c.total > 0 {
        age_sum / f64::from(c.total)
    } else {
        0.0
    };
    let (births, deaths) = count_vital_events(events);
    c.births = births;
    c.deaths = deaths;
    c
}

/// A proportional `#` bar `width` chars wide, scaled so the largest tally fills it.
fn census_bar(count: u32, max: u32, width: usize) -> String {
    if max == 0 {
        return String::new();
    }
    let filled = ((f64::from(count) / f64::from(max)) * width as f64).round() as usize;
    "#".repeat(filled.min(width))
}

/// Render the census as a fixed block of `CENSUS_LINES` display lines — a DF-style
/// "units" readout: population + leader + averages, a life-stage breakdown with
/// bars, a specialization breakdown, and a recent births/deaths line.
fn census_report_lines(c: &Census) -> Vec<String> {
    let stage_max = c.kittens.max(c.young).max(c.adults).max(c.elders);
    let leader = c.leader.as_deref().unwrap_or("(vacant)");
    vec![
        format!("Population: {}", c.total),
        format!("Leader: {leader}"),
        format!(
            "Avg age: {:.0}h    ★ Boosted: {}",
            c.avg_age_hours, c.boosted
        ),
        format!("Expecting: {}", c.expecting),
        String::new(),
        "- Life stages -".to_string(),
        format!(
            "Kittens {:>3}  {}",
            c.kittens,
            census_bar(c.kittens, stage_max, 12)
        ),
        format!(
            "Young   {:>3}  {}",
            c.young,
            census_bar(c.young, stage_max, 12)
        ),
        format!(
            "Adults  {:>3}  {}",
            c.adults,
            census_bar(c.adults, stage_max, 12)
        ),
        format!(
            "Elders  {:>3}  {}",
            c.elders,
            census_bar(c.elders, stage_max, 12)
        ),
        "- Specializations -".to_string(),
        format!("Hunter        {:>3}", c.hunters),
        format!("Architect     {:>3}", c.architects),
        format!("Ritualist     {:>3}", c.ritualists),
        format!("Warrior       {:>3}", c.warriors),
        format!("Unspecialized {:>3}", c.unspecialized),
        "- Recent -".to_string(),
        format!("Births {}   Deaths {}", c.births, c.deaths),
    ]
}

/// Line colour for an event kind (DF-style: birth green, death/raid red, crisis
/// amber, election/progress blue, neutral grey).
fn event_color(kind: EventKind) -> Color {
    match kind {
        EventKind::Birth => Color::srgb(0.45, 0.72, 0.36),
        EventKind::Death | EventKind::Raid => Color::srgb(0.82, 0.34, 0.30),
        EventKind::Crisis => Color::srgb(0.86, 0.66, 0.28),
        EventKind::Election | EventKind::Progress => Color::srgb(0.42, 0.60, 0.85),
        EventKind::Neutral => Color::srgb(0.42, 0.36, 0.28),
    }
}

/// A leading ASCII glyph marking an event's kind in the log.
fn event_glyph(kind: EventKind) -> char {
    match kind {
        EventKind::Birth => '+',
        EventKind::Death => 'x',
        EventKind::Raid | EventKind::Crisis => '!',
        EventKind::Election | EventKind::Progress => '*',
        EventKind::Neutral => '-',
    }
}

/// A short relative age like `5s` / `3m` / `2h` for an event timestamp.
fn relative_time(now_ms: i64, ts_ms: i64) -> String {
    let secs = (now_ms - ts_ms).max(0) / 1000;
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

/// The formatted announcement line for an event: `age glyph message`. The glyph
/// comes from the event's exact wire `kind`; the text is the human `message`.
fn announcement_line(now_ms: i64, kind: &str, message: &str, ts_ms: i64) -> String {
    format!(
        "{:>3} {} {}",
        relative_time(now_ms, ts_ms),
        event_glyph(event_kind_of(kind)),
        message
    )
}

// ---- Goods / inventory panel (pure formatting — unit-tested) ----

/// Item quality band name for a quality level (0..=4, clamped).
fn quality_band(quality: u8) -> &'static str {
    match quality {
        0 => "Crude",
        1 => "Common",
        2 => "Fine",
        3 => "Superior",
        _ => "Masterwork",
    }
}

/// Capitalize the first letter of a lowercase wire word (`wood` -> `Wood`).
fn capitalize_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// One goods line: `Fine Wood Mug x3 - 12g ea (36g)`, quality + material + kind,
/// per-unit value and the stack subtotal.
fn item_label(stack: &ItemStackSnapshot) -> String {
    format!(
        "{band} {material} {kind} x{count} - {value}g ea ({subtotal}g)",
        band = quality_band(stack.quality),
        material = capitalize_word(&stack.material),
        kind = capitalize_word(&stack.kind),
        count = stack.count,
        value = stack.value,
        subtotal = stack.count * stack.value,
    )
}

/// Colony treasury: total tradeable worth = sum of `count * value` over stacks.
fn treasury_total(items: &[ItemStackSnapshot]) -> u32 {
    items.iter().map(|s| s.count * s.value).sum()
}

// ---- Trade menu (pure formatting/affordability — unit-tested) ----

/// A sell-offer line: `Fine Wood Mug x3 - 8g ea` (the trader buys from you).
fn sell_offer_label(offer: &TraderBuyOffer) -> String {
    format!(
        "{band} {material} {kind} x{avail} - {price:.0}g ea",
        band = quality_band(offer.quality),
        material = capitalize_word(&offer.material),
        kind = capitalize_word(&offer.kind),
        avail = offer.available,
        price = offer.unit_price,
    )
}

/// A buy-offer line: `Food - 3g ea` (the trader sells to you); flags when you
/// can't afford one unit.
fn buy_offer_label(offer: &TraderSellOffer, coin: f64) -> String {
    let name = capitalize_word(resource_kind_name(offer.resource));
    if can_afford(coin, offer.unit_price) {
        format!("{name} - {:.0}g ea", offer.unit_price)
    } else {
        format!("{name} - {:.0}g ea  (low coin)", offer.unit_price)
    }
}

/// Whether `coin` covers a unit at `unit_price` (small epsilon for float slack).
fn can_afford(coin: f64, unit_price: f64) -> bool {
    coin + 1e-6 >= unit_price
}

/// The prominent coin readout in the trade menu.
fn coin_line(coin: f64) -> String {
    format!("Coin: {coin:.0}g")
}

// ---- Upgrade tree (structure from cat_sim::UPGRADE_NODES, state from the
// snapshot's ResearchSnapshot). Pure helpers unit-tested. ----

/// A tech node's progression state for the colony, derived from the owned set.
/// Mirrors the sim's `can_unlock`: a node is available once every prerequisite is
/// owned; affordability (blessings vs cost) is a further gate on the god-purchase.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NodeState {
    Owned,
    Available,
    Locked,
}

const NODE_OWNED_COLOR: Color = Color::srgb(0.46, 0.76, 0.42);
const NODE_READY_COLOR: Color = Color::srgb(0.96, 0.82, 0.36);
const NODE_UNAFFORDABLE_COLOR: Color = Color::srgb(0.66, 0.60, 0.40);
const NODE_LOCKED_COLOR: Color = Color::srgb(0.50, 0.45, 0.40);
const TREE_HEADER_COLOR: Color = Color::srgb(0.86, 0.72, 0.46);

/// Classify a node against the owned-node set (owned / prereqs-met / locked).
fn node_state(node: &UpgradeNode, owned: &HashSet<&str>) -> NodeState {
    if owned.contains(node.id) {
        NodeState::Owned
    } else if node.prerequisites.iter().all(|p| owned.contains(p)) {
        NodeState::Available
    } else {
        NodeState::Locked
    }
}

/// One node's display: its colour-coded label and whether the god-purchase
/// button should show (only for an available node the colony can afford).
fn node_line(node: &UpgradeNode, research: &ResearchSnapshot, owned: &HashSet<&str>) -> NodeLine {
    let (marker, color, show_buy) = match node_state(node, owned) {
        NodeState::Owned => ("[x]", NODE_OWNED_COLOR, false),
        NodeState::Available if can_afford(research.blessings, node.cost) => {
            ("[>]", NODE_READY_COLOR, true)
        }
        NodeState::Available => ("[ ]", NODE_UNAFFORDABLE_COLOR, false),
        NodeState::Locked => ("[-]", NODE_LOCKED_COLOR, false),
    };
    NodeLine {
        label: format!("{marker} {} ({:.0}b)", node.name, node.cost),
        color,
        show_buy,
    }
}

/// A node row's computed display (label + colour + whether to show its buy button).
struct NodeLine {
    label: String,
    color: Color,
    show_buy: bool,
}

/// The upgrade-tree header line: both currencies + who's researching.
fn tree_currency_line(research: &ResearchSnapshot) -> String {
    format!(
        "Blessings: {:.0}    Research: {:.0} pts ({} on it)",
        research.blessings, research.research_points, research.researcher_count
    )
}

/// The "next auto-unlock" header line (what the accruing research points target).
fn tree_next_line(research: &ResearchSnapshot) -> String {
    research.next_target.as_ref().map_or_else(
        || "Next auto-unlock: none".to_string(),
        |t| format!("Next auto-unlock: {} ({:.0} pts)", t.name, t.cost),
    )
}

/// A manual-action button and the action it enqueues when clicked.
#[derive(Component, Clone, Copy)]
struct ActionButton(ButtonAction);

/// A tool-mode toggle button.
#[derive(Component, Clone, Copy)]
struct ToolButton(ToolMode);

/// The stockpile accept-type picker button.
#[derive(Component)]
struct AcceptButton;
/// The accept-type picker's text.
#[derive(Component)]
struct AcceptButtonText;

/// Marker for the translucent zone-drag preview rectangle.
#[derive(Component)]
struct ZonePreview;

/// Duration a painted zone lasts (30 min; within the sim's 10min–2h window).
const ZONE_DURATION_MS: u64 = 30 * 60 * 1000;
/// Max zone side length in tiles (matches the sim's 8x8 cap).
const ZONE_MAX_TILES: i32 = 8;

/// Amber tint for stockpile overlays (distinct from avoid-red / gather-green).
const STOCKPILE_OVERLAY: Color = Color::srgba(0.85, 0.60, 0.25, 0.30);

/// Resource kinds a designated stockpile accepts by default (all storable
/// goods; blessings are not a physical pile).
const STORABLE_KINDS: [ResourceKind; 7] = [
    ResourceKind::Food,
    ResourceKind::Water,
    ResourceKind::Herbs,
    ResourceKind::Materials,
    ResourceKind::Refined,
    ResourceKind::Weapons,
    ResourceKind::Armor,
];

/// Query filter for the per-tick redraw of building sprite entities.
type BuildingEntities = With<BuildingSprite>;
/// Query filter for the per-tick redraw of stockpile visuals + highlight.
type StockpileEntities = Or<(With<StockpileVis>, With<StockpileHighlight>)>;
/// Query for the HUD resource value texts, disjoint from the header/footer texts.
type HudResourceQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Text, &'static HudResource),
    (Without<HudHeaderText>, Without<HudFooterText>),
>;
/// Change filter for the accept-type picker button.
type AcceptButtonQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static mut ImageNode),
    (Changed<Interaction>, With<AcceptButton>),
>;
/// Change filter for toolbar button interactions.
type ButtonQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        &'static ActionButton,
        &'static mut ImageNode,
    ),
    (Changed<Interaction>, With<Button>),
>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonAction {
    SupplyFood,
    SupplyWater,
    PlanHunt,
    FoundVillage,
}

impl ButtonAction {
    fn label(self) -> &'static str {
        match self {
            Self::SupplyFood => "Supply food",
            Self::SupplyWater => "Supply water",
            Self::PlanHunt => "Plan hunt",
            Self::FoundVillage => "Found village",
        }
    }
}

/// Build and run the client. `CAT_SERVER_URL` overrides the server address.
pub fn run() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: ".".to_string(),
                    // The game ships no `.meta` sidecars, so never probe for
                    // them: on the web that otherwise floods the console with a
                    // 404 per asset; on native it saves a redundant stat.
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                // Nearest-neighbour sampling keeps the 16px pixel-art crisp when
                // upscaled to TILE; the default linear filter blurs it.
                .set(bevy::image::ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Idle Cat Forest".to_string(),
                        resolution: bevy::window::WindowResolution::new(1280, 800),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(LatestSnapshot::default())
        .insert_resource(Session::default())
        .insert_resource(OutgoingActions::default())
        .insert_resource(WorldRender::default())
        .insert_resource(Selection::default())
        .insert_resource(StockpileSelection::default())
        .insert_resource(BuildingSelection::default())
        .insert_resource(OfficersUi::default())
        .insert_resource(AnnouncementsUi::default())
        .insert_resource(GoodsUi::default())
        .insert_resource(CensusUi::default())
        .insert_resource(UpgradeTreeUi::default())
        .insert_resource(TradeUi::default())
        .insert_resource(MinimapUi::default())
        .insert_resource(CatBodies::default())
        .insert_resource(RaiderBodies::default())
        .insert_resource(Tools::default())
        .insert_resource(ClearColor(Color::srgb(0.06, 0.09, 0.08)))
        .add_systems(Startup, (setup, connect_ws))
        // Grouped into sub-tuples to stay within Bevy's 20-per-tuple system arity.
        .add_systems(
            Update,
            (
                // networking + world render
                (
                    poll_ws,
                    ensure_presence,
                    spawn_terrain,
                    render_roads,
                    render_fog,
                    render_buildings,
                    render_walls,
                    render_zones,
                    render_stockpiles,
                    sync_cats,
                    sync_raiders,
                    sync_trader,
                    move_bodies,
                    lift_trader_above_fog.after(move_bodies),
                    follow_overlays,
                    animate_sprites,
                    hover_tooltip,
                ),
                // input, tools + HUD
                (
                    camera_controls,
                    select_cat,
                    select_building,
                    close_inspectors_on_esc,
                    update_building_inspector,
                    update_remove_panel,
                    handle_remove_button,
                    update_inspector,
                    handle_tool_buttons,
                    handle_accept_button,
                    zone_paint,
                    render_zone_preview,
                    update_hud,
                    update_event_log,
                    handle_buttons,
                    toggle_officers,
                    update_officers_panel,
                    handle_appoint_buttons,
                    handle_vacate_buttons,
                    flush_outgoing,
                ),
                // announcements / event log + goods + trade + boost + minimap
                (
                    cycle_stacked_selection,
                    toggle_announcements,
                    update_announcements,
                    toggle_goods,
                    update_goods,
                    update_trade_menu,
                    handle_trade_buttons,
                    update_boost_button,
                    handle_boost_button,
                    toggle_census,
                    update_census,
                    toggle_upgrade_tree,
                    update_upgrade_tree,
                    handle_tree_buy,
                    toggle_minimap,
                    update_minimap,
                    update_minimap_viewport,
                    minimap_click_to_pan,
                ),
            ),
        )
        .run();
}

/// The resources shown in the HUD readout. Its own enum (not `proto::ResourceKind`,
/// which lacks the refinement tier) so it can carry planks/blocks/tools too.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HudRes {
    Food,
    Water,
    Materials,
    Refined,
    Planks,
    Blocks,
    Tools,
    Herbs,
    Weapons,
    Armor,
    Blessings,
}

/// The HUD resources, in display order (refinement tier grouped after refined).
const HUD_RESOURCES: [HudRes; 11] = [
    HudRes::Food,
    HudRes::Water,
    HudRes::Materials,
    HudRes::Refined,
    HudRes::Planks,
    HudRes::Blocks,
    HudRes::Tools,
    HudRes::Herbs,
    HudRes::Weapons,
    HudRes::Armor,
    HudRes::Blessings,
];

/// Board-game glyph icons (white, recolorable) for the HUD resource readout.
#[derive(Resource, Clone)]
struct IconArt {
    food: Handle<Image>,
    water: Handle<Image>,
    materials: Handle<Image>,
    refined: Handle<Image>,
    planks: Handle<Image>,
    blocks: Handle<Image>,
    tools: Handle<Image>,
    herbs: Handle<Image>,
    weapons: Handle<Image>,
    armor: Handle<Image>,
    blessings: Handle<Image>,
    goods: Handle<Image>,
}

impl IconArt {
    fn load(assets: &AssetServer) -> Self {
        Self {
            food: assets.load("public/images/game/icons/food.png"),
            water: assets.load("public/images/game/icons/water.png"),
            materials: assets.load("public/images/game/icons/materials.png"),
            refined: assets.load("public/images/game/icons/refined.png"),
            planks: assets.load("public/images/game/icons/planks.png"),
            blocks: assets.load("public/images/game/icons/blocks.png"),
            tools: assets.load("public/images/game/icons/tools.png"),
            herbs: assets.load("public/images/game/icons/herbs.png"),
            weapons: assets.load("public/images/game/icons/weapons.png"),
            armor: assets.load("public/images/game/icons/armor.png"),
            blessings: assets.load("public/images/game/icons/blessings.png"),
            goods: assets.load("public/images/game/icons/goods.png"),
        }
    }

    fn get(&self, kind: HudRes) -> Handle<Image> {
        match kind {
            HudRes::Food => self.food.clone(),
            HudRes::Water => self.water.clone(),
            HudRes::Materials => self.materials.clone(),
            HudRes::Refined => self.refined.clone(),
            HudRes::Planks => self.planks.clone(),
            HudRes::Blocks => self.blocks.clone(),
            HudRes::Tools => self.tools.clone(),
            HudRes::Herbs => self.herbs.clone(),
            HudRes::Weapons => self.weapons.clone(),
            HudRes::Armor => self.armor.clone(),
            HudRes::Blessings => self.blessings.clone(),
        }
    }

    /// The glyph for a crafted-item kind: sword/shield/wrench for weapon/armor/
    /// tool, else the generic goods pouch (tinted by material at the call site).
    fn item_glyph(&self, kind: &str) -> Handle<Image> {
        match kind {
            "weapon" => self.weapons.clone(),
            "armor" => self.armor.clone(),
            "tool" => self.tools.clone(),
            _ => self.goods.clone(),
        }
    }
}

/// Tint for a crafted item's glyph by its material, so the goods list scans by
/// colour (wood brown, stone grey, metal steel, …).
fn material_tint(material: &str) -> Color {
    match material {
        "wood" => Color::srgb(0.62, 0.46, 0.29),
        "stone" => Color::srgb(0.62, 0.64, 0.66),
        "metal" => Color::srgb(0.72, 0.76, 0.82),
        "bone" => Color::srgb(0.90, 0.86, 0.74),
        "fibre" => Color::srgb(0.72, 0.78, 0.52),
        "leather" => Color::srgb(0.66, 0.45, 0.30),
        "gem" => Color::srgb(0.42, 0.80, 0.86),
        "clay" => Color::srgb(0.80, 0.52, 0.40),
        _ => Color::srgb(0.78, 0.76, 0.72),
    }
}

/// Tags a HUD value `Text` with the resource it reports, so one system can
/// refresh every readout.
#[derive(Component, Clone, Copy)]
struct HudResource(HudRes);

/// Map a wire [`ResourceKind`] to the HUD/icon glyph key, so gather-spot markers
/// (and anything else keyed by resource) reuse the resource icon set.
fn hud_res_of(kind: ResourceKind) -> HudRes {
    match kind {
        ResourceKind::Food => HudRes::Food,
        ResourceKind::Water => HudRes::Water,
        ResourceKind::Herbs => HudRes::Herbs,
        ResourceKind::Materials => HudRes::Materials,
        ResourceKind::Refined => HudRes::Refined,
        ResourceKind::Weapons => HudRes::Weapons,
        ResourceKind::Armor => HudRes::Armor,
        ResourceKind::Blessings => HudRes::Blessings,
    }
}

/// The tint applied to a resource's white glyph so the readout reads at a glance.
fn resource_icon_tint(kind: HudRes) -> Color {
    match kind {
        HudRes::Food => Color::srgb(0.87, 0.35, 0.26),
        HudRes::Water => Color::srgb(0.36, 0.62, 0.93),
        HudRes::Materials => Color::srgb(0.62, 0.46, 0.29),
        HudRes::Refined => Color::srgb(0.86, 0.71, 0.40),
        HudRes::Planks => Color::srgb(0.82, 0.66, 0.42),
        HudRes::Blocks => Color::srgb(0.62, 0.64, 0.66),
        HudRes::Tools => Color::srgb(0.70, 0.74, 0.80),
        HudRes::Herbs => Color::srgb(0.51, 0.79, 0.42),
        HudRes::Weapons => Color::srgb(0.74, 0.76, 0.82),
        HudRes::Armor => Color::srgb(0.56, 0.64, 0.76),
        HudRes::Blessings => Color::srgb(0.96, 0.80, 0.32),
    }
}

/// The HUD value text for a resource: `value / cap` for capped storables, a bare
/// value for weapons/armor, and one decimal for blessings.
fn hud_resource_value(kind: HudRes, r: &ResourceAmounts, cap: &ResourceCapacities) -> String {
    match kind {
        HudRes::Food => format!("{:.0} / {:.0}", r.food, cap.food),
        HudRes::Water => format!("{:.0} / {:.0}", r.water, cap.water),
        HudRes::Materials => format!("{:.0} / {:.0}", r.materials, cap.materials),
        HudRes::Refined => format!("{:.0} / {:.0}", r.refined, cap.refined),
        HudRes::Planks => format!("{:.0} / {:.0}", r.planks, cap.planks),
        HudRes::Blocks => format!("{:.0} / {:.0}", r.blocks, cap.blocks),
        HudRes::Tools => format!("{:.0} / {:.0}", r.tools, cap.tools),
        HudRes::Herbs => format!("{:.0} / {:.0}", r.herbs, cap.herbs),
        HudRes::Weapons => format!("{:.0}", r.weapons),
        HudRes::Armor => format!("{:.0}", r.armor),
        HudRes::Blessings => format!("{:.1}", r.blessings),
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
) {
    // Swap Bevy's embedded default sans for a crisp Kenney pixel face. Every
    // TextFont uses the default font handle, so overwriting that asset restyles
    // the whole UI with no per-call-site change. Embedded (not asset-loaded) so
    // it also works on wasm and can't 404. CC0 (Kenney).
    const UI_FONT: &[u8] = include_bytes!("../../../public/fonts/kenney-future-narrow.ttf");
    if let Err(err) = fonts.insert(
        &Handle::<Font>::default(),
        Font::from_bytes(UI_FONT.to_vec()),
    ) {
        warn!("failed to install UI font: {err:?}");
    }

    commands.insert_resource(TerrainArt::load(&asset_server));
    commands.insert_resource(BuildingArt::load(&asset_server));
    let icons = IconArt::load(&asset_server);
    commands.insert_resource(icons.clone());
    commands.insert_resource(PropArt::load(&asset_server));
    commands.insert_resource(InfraArt::load(&asset_server));
    commands.insert_resource(SpriteSheets::load(&asset_server, &mut atlas_layouts));
    let ui = UiArt::load(&asset_server);
    commands.insert_resource(ui.clone());

    // Dynamic minimap texture (rewritten each snapshot by update_minimap).
    let minimap_image = Image::new_fill(
        Extent3d {
            width: MINIMAP_PX as u32,
            height: MINIMAP_PX as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &MINIMAP_FOG,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::all(),
    );
    let minimap_handle = images.add(minimap_image);
    commands.insert_resource(Minimap {
        image: minimap_handle.clone(),
        view: minimap_view(&[]),
    });

    // Camera at Z=1000: a default Camera2d sits at Z=0 and clips sprites at
    // Z>0. Centre on the village anchor.
    let center = grid_to_world(VILLAGE_ANCHOR.x, VILLAGE_ANCHOR.y);
    commands.spawn((Camera2d, Transform::from_xyz(center.x, center.y, CAMERA_Z)));

    // HUD dashboard (top-left) on a wood/parchment panel with a hanging banner.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                top: Val::Px(8.0),
                width: Val::Px(330.0),
                padding: UiRect::all(Val::Px(26.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            sliced_image(ui.panel.clone(), PANEL_BORDER),
        ))
        .with_children(|panel| {
            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ImageNode::new(ui.banner.clone()),
                children![(
                    Text::new("Idle Cat Forest"),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.97, 0.90)),
                )],
            ));
            // Colony header (name / leader / pop / threat).
            panel.spawn((
                Text::new("connecting…"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(PARCHMENT_INK),
                HudHeaderText,
            ));
            // Resource readout: a tinted glyph + value per resource, in TWO
            // columns (a wrapping row of fixed-width cells) so the 11 resources
            // fit ~6 rows instead of 11 — keeps the dashboard short enough to
            // clear the event log on short windows.
            panel
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: Val::Px(3.0),
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|grid| {
                    for kind in HUD_RESOURCES {
                        grid.spawn((
                            Node {
                                width: Val::Px(126.0),
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(6.0),
                                ..default()
                            },
                            children![
                                (
                                    Node {
                                        width: Val::Px(16.0),
                                        height: Val::Px(16.0),
                                        ..default()
                                    },
                                    ImageNode {
                                        image: icons.get(kind),
                                        color: resource_icon_tint(kind),
                                        ..default()
                                    },
                                ),
                                (
                                    Text::new("-"),
                                    TextFont {
                                        font_size: FontSize::Px(13.0),
                                        ..default()
                                    },
                                    TextColor(PARCHMENT_INK),
                                    HudResource(kind),
                                ),
                            ],
                        ));
                    }
                });
            // Jobs + ledger footer.
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(PARCHMENT_INK),
                HudFooterText,
            ));
        });

    // Event log (bottom-left) on a parchment panel.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(8.0),
            bottom: Val::Px(70.0),
            width: Val::Px(430.0),
            padding: UiRect::all(Val::Px(26.0)),
            ..default()
        },
        sliced_image(ui.panel.clone(), PANEL_BORDER),
        children![(
            Text::new("events…"),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(PARCHMENT_INK),
            EventLogText,
        )],
    ));

    // Corner minimap (bottom-right, clear of the inspectors + toolbars), 9-patch
    // framed, showing the dynamic minimap texture. Toggled with 'M'.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(8.0),
            bottom: Val::Px(70.0),
            padding: UiRect::all(Val::Px(14.0)),
            ..default()
        },
        GlobalZIndex(70),
        sliced_image(ui.panel.clone(), PANEL_BORDER),
        MinimapPanel,
        children![(
            Node {
                width: Val::Px(168.0),
                height: Val::Px(168.0),
                ..default()
            },
            ImageNode::new(minimap_handle),
            // Button so the world-pick systems (which skip Button interactions)
            // ignore clicks that land on the minimap.
            Button,
            RelativeCursorPosition::default(),
            MinimapImageNode,
            // Camera-viewport outline, positioned each frame over the minimap.
            children![(
                Node {
                    position_type: PositionType::Absolute,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.85)),
                MinimapViewportRect,
            )],
        )],
    ));

    // "Latest announcement" ticker, to the right of the Goods/Log/Census/Tree
    // toggle buttons (which end at ~552) so it never overlaps them.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(14.0),
            left: Val::Px(568.0),
            max_width: Val::Px(360.0),
            ..default()
        },
        GlobalZIndex(60),
        Text::new(""),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(PARCHMENT_INK),
        AnnouncementTicker,
    ));
    commands.spawn((
        Button,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(300.0),
            min_width: Val::Px(52.0),
            height: Val::Px(28.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        GlobalZIndex(60),
        sliced_image(ui.button.clone(), BUTTON_BORDER),
        AnnouncementsButton,
        children![(
            Text::new("Log [L]"),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(PARCHMENT_INK),
        )],
    ));

    // Announcements / event-log panel (centre), hidden until toggled.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(430.0),
                top: Val::Px(60.0),
                // Capped so the panel clears the right-side cat inspector at the
                // 1280-wide default (inspector left edge ≈ 972).
                width: Val::Px(500.0),
                // Extra bottom padding so the last line clears the wood frame.
                padding: UiRect::axes(Val::Px(26.0), Val::Px(40.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                display: Display::None,
                ..default()
            },
            GlobalZIndex(80),
            sliced_image(ui.panel.clone(), PANEL_BORDER),
            AnnouncementsPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
                ImageNode::new(ui.banner.clone()),
                children![(
                    Text::new("Announcements"),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.97, 0.90)),
                )],
            ));
            for i in 0..ANNOUNCEMENT_LINES {
                panel.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(PARCHMENT_INK),
                    AnnouncementLine(i),
                ));
            }
        });

    // Goods toggle button (beside the Log button).
    commands.spawn((
        Button,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(200.0),
            min_width: Val::Px(52.0),
            height: Val::Px(28.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        GlobalZIndex(60),
        sliced_image(ui.button.clone(), BUTTON_BORDER),
        GoodsButton,
        children![(
            Text::new("Goods [G]"),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(PARCHMENT_INK),
        )],
    ));

    // Census toggle button (beside the Goods button).
    commands.spawn((
        Button,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(400.0),
            min_width: Val::Px(52.0),
            height: Val::Px(28.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        GlobalZIndex(60),
        sliced_image(ui.button.clone(), BUTTON_BORDER),
        CensusButton,
        children![(
            Text::new("Census [C]"),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(PARCHMENT_INK),
        )],
    ));

    // Colony census / demographics panel (centre, shares the slot with goods +
    // announcements — the three are mutually exclusive), hidden until toggled.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(430.0),
                top: Val::Px(60.0),
                width: Val::Px(360.0),
                padding: UiRect::axes(Val::Px(26.0), Val::Px(40.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                display: Display::None,
                ..default()
            },
            GlobalZIndex(82),
            sliced_image(ui.panel.clone(), PANEL_BORDER),
            CensusPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                },
                ImageNode::new(ui.banner.clone()),
                children![(
                    Text::new("Census"),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.97, 0.90)),
                )],
            ));
            for i in 0..CENSUS_LINES {
                panel.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(PARCHMENT_INK),
                    CensusLine(i),
                ));
            }
        });

    // Upgrade-tree toggle button (beside the Census button).
    commands.spawn((
        Button,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(500.0),
            min_width: Val::Px(52.0),
            height: Val::Px(28.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        GlobalZIndex(60),
        sliced_image(ui.button.clone(), BUTTON_BORDER),
        TreeButton,
        children![(
            Text::new("Tree [U]"),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(PARCHMENT_INK),
        )],
    ));

    // Upgrade-tree panel (centre, shares the slot with goods/announcements/census
    // — all mutually exclusive), hidden until toggled.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(430.0),
                top: Val::Px(60.0),
                width: Val::Px(400.0),
                padding: UiRect::axes(Val::Px(24.0), Val::Px(30.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                display: Display::None,
                ..default()
            },
            GlobalZIndex(82),
            sliced_image(ui.panel.clone(), PANEL_BORDER),
            TreePanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(6.0)),
                    ..default()
                },
                ImageNode::new(ui.banner.clone()),
                children![(
                    Text::new("Upgrade Tree"),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.97, 0.90)),
                )],
            ));
            let header = |text: &str| {
                (
                    Text::new(text.to_string()),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(PARCHMENT_INK),
                )
            };
            panel.spawn((header(""), TreeCurrencyText));
            panel.spawn((header(""), TreeNextText));
            // One era section per era, each with its nodes. Static structure from
            // UPGRADE_NODES (ordered by era); rows carry the node id so the update
            // system can colour them + wire each buy button.
            let max_era = UPGRADE_NODES.iter().map(|n| n.era).max().unwrap_or(0);
            for era in 1..=max_era {
                panel.spawn((
                    Text::new(format!("- Era {era} -")),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(TREE_HEADER_COLOR),
                    Node {
                        margin: UiRect::top(Val::Px(3.0)),
                        ..default()
                    },
                ));
                for node in UPGRADE_NODES.iter().filter(|n| n.era == era) {
                    panel
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            column_gap: Val::Px(6.0),
                            ..default()
                        })
                        .with_children(|row| {
                            row.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: FontSize::Px(12.0),
                                    ..default()
                                },
                                TextColor(PARCHMENT_INK),
                                TreeNodeText(node.id),
                            ));
                            row.spawn((
                                Button,
                                Node {
                                    min_width: Val::Px(40.0),
                                    height: Val::Px(20.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    display: Display::None,
                                    ..default()
                                },
                                sliced_image(ui.button.clone(), BUTTON_BORDER),
                                TreeBuyButton(node.id),
                                children![(
                                    Text::new("Buy"),
                                    TextFont {
                                        font_size: FontSize::Px(11.0),
                                        ..default()
                                    },
                                    TextColor(PARCHMENT_INK),
                                )],
                            ));
                        });
                }
            }
        });

    // Goods / inventory panel (centre, shares the slot with announcements — the
    // two are mutually exclusive), hidden until toggled.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(430.0),
                top: Val::Px(60.0),
                width: Val::Px(500.0),
                padding: UiRect::axes(Val::Px(26.0), Val::Px(40.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(3.0),
                display: Display::None,
                ..default()
            },
            GlobalZIndex(82),
            sliced_image(ui.panel.clone(), PANEL_BORDER),
            GoodsPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
                ImageNode::new(ui.banner.clone()),
                children![(
                    Text::new("Goods"),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.97, 0.90)),
                )],
            ));
            // Treasury total.
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.86, 0.66, 0.28)),
                GoodsTreasury,
            ));
            for i in 0..GOODS_LINES {
                panel
                    .spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(7.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Node {
                                width: Val::Px(15.0),
                                height: Val::Px(15.0),
                                display: Display::None,
                                ..default()
                            },
                            ImageNode::new(icons.goods.clone()),
                            GoodsLineIcon(i),
                        ));
                        row.spawn((
                            Text::new(""),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(PARCHMENT_INK),
                            GoodsLine(i),
                        ));
                    });
            }
        });

    // Trade menu (centre), shown only while a trader is Trading at the gate.
    let small_btn = || Node {
        min_width: Val::Px(46.0),
        height: Val::Px(24.0),
        padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    };
    let row_node = || Node {
        width: Val::Percent(100.0),
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        display: Display::None,
        ..default()
    };
    let label_node = || Node {
        width: Val::Px(330.0),
        ..default()
    };
    let header = |text: &str| {
        (
            Text::new(text.to_string()),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(Color::srgb(0.55, 0.42, 0.24)),
        )
    };
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(390.0),
                top: Val::Px(70.0),
                // Capped to clear the right-side inspector at the 1280 default.
                width: Val::Px(500.0),
                padding: UiRect::axes(Val::Px(26.0), Val::Px(34.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                display: Display::None,
                ..default()
            },
            GlobalZIndex(90),
            sliced_image(ui.panel.clone(), PANEL_BORDER),
            TradeMenuPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(4.0)),
                    ..default()
                },
                ImageNode::new(ui.banner.clone()),
                children![(
                    Text::new("Trader"),
                    TextFont {
                        font_size: FontSize::Px(15.0),
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.97, 0.90)),
                )],
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::srgb(0.86, 0.66, 0.28)),
                TradeCoinText,
            ));
            panel.spawn(header("- Sell your crafts -"));
            for i in 0..TRADE_SELL_ROWS {
                panel.spawn((row_node(), SellRow(i))).with_children(|row| {
                    row.spawn((
                        label_node(),
                        children![(
                            Text::new(""),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(PARCHMENT_INK),
                            SellRowText(i),
                        )],
                    ));
                    for all in [false, true] {
                        row.spawn((
                            Button,
                            small_btn(),
                            sliced_image(ui.button.clone(), BUTTON_BORDER),
                            SellButton { row: i, all },
                            children![(
                                Text::new(if all { "All" } else { "Sell 1" }),
                                TextFont {
                                    font_size: FontSize::Px(10.0),
                                    ..default()
                                },
                                TextColor(PARCHMENT_INK),
                            )],
                        ));
                    }
                });
            }
            panel.spawn(header("- Buy resources -"));
            for i in 0..TRADE_BUY_ROWS {
                panel.spawn((row_node(), BuyRow(i))).with_children(|row| {
                    row.spawn((
                        label_node(),
                        children![(
                            Text::new(""),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(PARCHMENT_INK),
                            BuyRowText(i),
                        )],
                    ));
                    row.spawn((
                        Button,
                        small_btn(),
                        sliced_image(ui.button.clone(), BUTTON_BORDER),
                        BuyButton(i),
                        children![(
                            Text::new("Buy 1"),
                            TextFont {
                                font_size: FontSize::Px(10.0),
                                ..default()
                            },
                            TextColor(PARCHMENT_INK),
                        )],
                    ));
                });
            }
            panel.spawn((
                Button,
                Node {
                    min_width: Val::Px(90.0),
                    height: Val::Px(26.0),
                    margin: UiRect::top(Val::Px(6.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                sliced_image(ui.button.clone(), BUTTON_BORDER),
                TradeCloseButton,
                children![(
                    Text::new("Close [Esc]"),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(PARCHMENT_INK),
                )],
            ));
        });

    // Hover tooltip (small, follows the cursor), hidden until hovering an entity.
    // High GlobalZIndex keeps it above the other panels.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            max_width: Val::Px(250.0),
            padding: UiRect::all(Val::Px(20.0)),
            display: Display::None,
            ..default()
        },
        sliced_image(ui.panel.clone(), PANEL_BORDER),
        GlobalZIndex(100),
        TooltipPanel,
        children![(
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(PARCHMENT_INK),
            TooltipText,
        )],
    ));

    // Cat inspector (top-right), hidden until a cat is selected. Includes a row
    // of "Appoint <role>" buttons that make the selected cat that officer.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(8.0),
                top: Val::Px(8.0),
                width: Val::Px(300.0),
                padding: UiRect::all(Val::Px(24.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                display: Display::None,
                ..default()
            },
            sliced_image(ui.panel.clone(), PANEL_BORDER),
            InspectorPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(PARCHMENT_INK),
                InspectorText,
            ));
            // Needs, one labelled bar each (green/amber/red by level).
            for (kind, label) in CAT_NEEDS {
                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Node {
                                width: Val::Px(52.0),
                                ..default()
                            },
                            children![(
                                Text::new(label),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(PARCHMENT_INK),
                            )],
                        ));
                        // Bar track + fill.
                        row.spawn((
                            Node {
                                flex_grow: 1.0,
                                height: Val::Px(11.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.20, 0.14, 0.08, 0.55)),
                            children![(
                                Node {
                                    width: Val::Percent(0.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(need_bar_color(0.0)),
                                NeedBar(kind),
                            )],
                        ));
                    });
            }
            // God-power: mark this cat a priority pick for the leader's matcher.
            panel.spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(26.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                sliced_image(ui.button.clone(), BUTTON_BORDER),
                BoostButton,
                children![(
                    Text::new(boost_button_label(false)),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(PARCHMENT_INK),
                    BoostButtonText,
                )],
            ));
            panel.spawn((
                Text::new("Appoint officer:"),
                TextFont {
                    font_size: FontSize::Px(11.0),
                    ..default()
                },
                TextColor(PARCHMENT_INK),
            ));
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(4.0),
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|row| {
                    for role in ALL_OFFICER_ROLES {
                        row.spawn((
                            Button,
                            Node {
                                min_width: Val::Px(64.0),
                                height: Val::Px(24.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            sliced_image(ui.button.clone(), BUTTON_BORDER),
                            AppointButton(role),
                            children![(
                                Text::new(officer_role_name(role)),
                                TextFont {
                                    font_size: FontSize::Px(10.0),
                                    ..default()
                                },
                                TextColor(PARCHMENT_INK),
                            )],
                        ));
                    }
                });
        });

    // Remove-stockpile affordance (right side), hidden until one is selected.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(8.0),
            top: Val::Px(170.0),
            width: Val::Px(210.0),
            padding: UiRect::all(Val::Px(22.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            display: Display::None,
            ..default()
        },
        sliced_image(ui.panel.clone(), PANEL_BORDER),
        RemovePanel,
        children![
            (
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(PARCHMENT_INK),
                RemovePanelText,
            ),
            (
                Button,
                wood_button_node(),
                sliced_image(ui.button.clone(), BUTTON_BORDER),
                RemoveStockpileButton,
                children![(
                    Text::new("Remove stockpile"),
                    TextFont {
                        font_size: FontSize::Px(12.0),
                        ..default()
                    },
                    TextColor(PARCHMENT_INK),
                )],
            ),
        ],
    ));

    // Building inspector (right, below the remove panel), middle-click a
    // building; hidden until one is selected.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(8.0),
            // Below the cat inspector (which fits ~300px tall at this width) but
            // high enough that a tall producer panel still clears the minimap.
            top: Val::Px(336.0),
            width: Val::Px(300.0),
            padding: UiRect::all(Val::Px(24.0)),
            display: Display::None,
            ..default()
        },
        sliced_image(ui.panel.clone(), PANEL_BORDER),
        BuildingInspectorPanel,
        children![(
            Text::new(""),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(PARCHMENT_INK),
            BuildingInspectorText,
        )],
    ));

    // Officers panel (left, below the dashboard), toggled with `O`.
    spawn_officers_panel(&mut commands, &ui);

    // Tool-mode toolbar (just above the action toolbar).
    spawn_tool_toolbar(&mut commands, &ui);
    // Action toolbar (bottom, centred).
    spawn_toolbar(&mut commands, &ui);
}

fn spawn_officers_panel(commands: &mut Commands, ui: &UiArt) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(8.0),
                // Below the HUD dashboard, which grew taller with the refinement
                // tier (planks/blocks/tools) rows.
                top: Val::Px(500.0),
                width: Val::Px(268.0),
                padding: UiRect::all(Val::Px(26.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
            sliced_image(ui.panel.clone(), PANEL_BORDER),
            OfficersPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Officers  [O]"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(PARCHMENT_INK),
            ));
            for role in ALL_OFFICER_ROLES {
                panel
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new(""),
                            TextFont {
                                font_size: FontSize::Px(12.0),
                                ..default()
                            },
                            TextColor(PARCHMENT_INK),
                            OfficerRow(role),
                        ));
                        row.spawn((
                            Button,
                            Node {
                                width: Val::Px(22.0),
                                height: Val::Px(20.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            sliced_image(ui.button.clone(), BUTTON_BORDER),
                            VacateButton(role),
                            children![(
                                Text::new("x"),
                                TextFont {
                                    font_size: FontSize::Px(11.0),
                                    ..default()
                                },
                                TextColor(PARCHMENT_INK),
                            )],
                        ));
                    });
            }
        });
}

fn spawn_tool_toolbar(commands: &mut Commands, ui: &UiArt) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(48.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|row| {
            for mode in [
                ToolMode::Inspect,
                ToolMode::AvoidZone,
                ToolMode::GatherZone,
                ToolMode::Stockpile,
            ] {
                row.spawn((
                    Button,
                    wood_button_node(),
                    sliced_image(ui.button.clone(), BUTTON_BORDER),
                    ToolButton(mode),
                    children![(
                        Text::new(mode.label()),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(PARCHMENT_INK),
                    )],
                ));
            }
            // Accept-type picker for the Stockpile mode — cycles what the next
            // designated pile will accept.
            row.spawn((
                Button,
                wood_button_node(),
                sliced_image(ui.button.clone(), BUTTON_BORDER),
                AcceptButton,
                children![(
                    Text::new("Accepts: General"),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(PARCHMENT_INK),
                    AcceptButtonText,
                )],
            ));
        });
}

/// A wood-button node (parchment sprite carries the look; text is dark ink).
fn wood_button_node() -> Node {
    Node {
        min_width: Val::Px(96.0),
        height: Val::Px(34.0),
        padding: UiRect::axes(Val::Px(12.0), Val::Px(4.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

fn spawn_toolbar(commands: &mut Commands, ui: &UiArt) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(10.0),
            ..default()
        })
        .with_children(|row| {
            for action in [
                ButtonAction::SupplyFood,
                ButtonAction::SupplyWater,
                ButtonAction::PlanHunt,
                ButtonAction::FoundVillage,
            ] {
                row.spawn((
                    Button,
                    wood_button_node(),
                    sliced_image(ui.button.clone(), BUTTON_BORDER),
                    ActionButton(action),
                    children![(
                        Text::new(action.label()),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(PARCHMENT_INK),
                    )],
                ));
            }
        });
}

/// Open the WebSocket as a non-send resource (the receiver is `!Sync`).
/// The cat-server WebSocket URL for this target.
///
/// Native: the `CAT_SERVER_URL` env var at launch, else the local default.
/// Wasm (browser): `std::env::var` always errs, so use a build-time
/// `CAT_SERVER_URL` bake if present (how the dev wasm build points at a
/// separate-port server), else derive a same-origin `ws(s)://<host>/ws` from the
/// page location (the reverse-proxied production default).
fn server_ws_url() -> String {
    const DEFAULT: &str = "ws://127.0.0.1:8787/ws";
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var("CAT_SERVER_URL").unwrap_or_else(|_| DEFAULT.to_string())
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(url) = option_env!("CAT_SERVER_URL") {
            return url.to_string();
        }
        web_sys::window()
            .and_then(|w| {
                let loc = w.location();
                let scheme = if loc.protocol().ok()?.as_str() == "https:" {
                    "wss"
                } else {
                    "ws"
                };
                Some(format!("{scheme}://{}/ws", loc.host().ok()?))
            })
            .unwrap_or_else(|| DEFAULT.to_string())
    }
}

fn connect_ws(world: &mut World) {
    let url = server_ws_url();
    match ewebsock::connect(url.clone(), ewebsock::Options::default()) {
        Ok((sender, receiver)) => {
            info!("cat-client connecting to {url}");
            world.insert_non_send(WsConn { sender, receiver });
        }
        Err(err) => error!("cat-client failed to connect to {url}: {err}"),
    }
}

/// Drain socket messages: `WorldSnapshot`s update the render, action results
/// carry the signed session after a `Presence` handshake.
fn poll_ws(
    conn: Option<NonSend<WsConn>>,
    mut latest: ResMut<LatestSnapshot>,
    mut session: ResMut<Session>,
) {
    let Some(conn) = conn else {
        return;
    };
    while let Some(event) = conn.receiver.try_recv() {
        let WsEvent::Message(WsMessage::Text(text)) = event else {
            continue;
        };
        // Snapshots carry a `colonies` array; action results carry `ok`.
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(value) if value.get("colonies").is_some() => {
                match serde_json::from_str::<WorldSnapshot>(&text) {
                    Ok(snapshot) => latest.0 = Some(snapshot),
                    Err(err) => warn!("bad snapshot: {err}"),
                }
            }
            Ok(value) => {
                if let (Some(sid), Some(sig)) = (
                    value.get("sessionId").and_then(|v| v.as_str()),
                    value.get("sig").and_then(|v| v.as_str()),
                ) {
                    session.session_id = sid.to_string();
                    session.sig = sig.to_string();
                    session.ready = true;
                }
            }
            Err(err) => warn!("bad ws message: {err}"),
        }
    }
}

/// Send the `Presence` handshake once so the server issues a signed session.
fn ensure_presence(conn: Option<NonSendMut<WsConn>>, mut session: ResMut<Session>) {
    let Some(mut conn) = conn else {
        return;
    };
    if session.presence_sent {
        return;
    }
    let action = ClientAction::Presence {
        session_id: "desktop".to_string(),
        nickname: "Desktop Cat".to_string(),
        sig: None,
    };
    if let Ok(json) = serde_json::to_string(&action) {
        conn.sender.send(WsMessage::Text(json));
        session.presence_sent = true;
    }
}

/// Spawn the flat terrain grid once, from the first snapshot's `world_seed`.
fn spawn_terrain(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    art: Option<Res<TerrainArt>>,
    mut render: ResMut<WorldRender>,
) {
    if render.terrain_spawned {
        return;
    }
    let (Some(world), Some(art)) = (latest.0.as_ref(), art) else {
        return;
    };
    let seed = world.world_seed;
    let tiles = window_terrain(seed);
    // Water coordinates (river overlay OR a water climate biome), so shore tiles
    // (a non-water orthogonal neighbour) can use the water_edge variant.
    let water: HashSet<(i32, i32)> = tiles
        .iter()
        .filter(|t| t.river.is_some() || is_water_biome(t.climate_biome))
        .map(|t| (t.x, t.y))
        .collect();

    for tile in &tiles {
        let p = grid_to_world(tile.x, tile.y);
        let is_water = tile.river.is_some() || is_water_biome(tile.climate_biome);
        let ground = if is_water {
            if is_shore(tile.x, tile.y, &water) {
                art.water_edge.clone()
            } else {
                art.water.clone()
            }
        } else {
            art.ground(ground_texture(tile))
        };
        // Multiply the base tile by the biome tint so ~26 climate biomes read as
        // distinct ground (forest darker green, desert sandy, snow pale, ocean
        // deep blue) without needing a unique sprite per biome.
        commands.spawn((
            Sprite {
                image: ground,
                custom_size: Some(Vec2::splat(TILE)),
                color: biome_tint(tile.climate_biome),
                ..default()
            },
            Transform::from_xyz(p.x, p.y, Z_TERRAIN),
        ));

        // Per-biome decoration density: forests dense with trees, plains open,
        // desert/tundra bare — driven by the biome's density table rather than
        // the coarse BiomeRole `decoration` field.
        let decoration = if is_water {
            None
        } else {
            derive_biome_decoration(tile.x, tile.y, seed, tile.climate_biome)
        };
        match decoration {
            Some(DecorationRole::Tree { .. }) => {
                // Tree species follows the biome (conifer/broadleaf/stump), not a
                // per-tile species roll. Top-down tree = a canopy seen from above:
                // a square sprite centred on the tile (not a standing side-view
                // trunk), sized a touch larger than the tile so the forest reads
                // as overlapping canopies.
                let (tree, scale) = art.tree(biome_tree(tile.climate_biome));
                commands.spawn((
                    Sprite {
                        image: tree,
                        custom_size: Some(Vec2::splat(TILE * scale)),
                        ..default()
                    },
                    Anchor::CENTER,
                    Transform::from_xyz(p.x, p.y, ysort_z(p.y)),
                ));
            }
            Some(DecorationRole::Rock { size, .. }) => {
                let scale = rock_scale(size);
                let base_y = p.y - TILE * 0.5;
                commands.spawn((
                    Sprite {
                        image: art.rock.clone(),
                        custom_size: Some(Vec2::splat(TILE * scale)),
                        ..default()
                    },
                    Anchor::BOTTOM_CENTER,
                    Transform::from_xyz(p.x, base_y, ysort_z(base_y)),
                ));
            }
            None => {}
        }
    }
    render.terrain_spawned = true;
    info!("terrain spawned (seed {seed}, {} tiles)", tiles.len());
}

/// A river tile with at least one non-water orthogonal neighbour is a shore.
fn is_shore(x: i32, y: i32, water: &HashSet<(i32, i32)>) -> bool {
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .any(|(dx, dy)| !water.contains(&(x + dx, y + dy)))
}

/// Base ground texture family for a non-water tile, keyed on the climate biome
/// so sandy biomes get the dirt tile, stony/mountain biomes the rock tile, and
/// everything vegetated a grass tile — the biome tint then colours it. A
/// deterministic `grass_var` sprinkle keeps grassland from tiling flat.
fn ground_texture(tile: &TerrainTile) -> GroundTexture {
    use Biome::*;
    match tile.climate_biome {
        // Sandy / dry biomes → the tan sand tile.
        Beach | Desert | Savanna => GroundTexture::Sand,
        // Stone / rock biomes → the grey rock tile.
        StonyShore | Mountains => GroundTexture::Rocky,
        // Red-clay badlands → bare earth.
        Badlands => GroundTexture::Dirt,
        // Cold land → the snow tile (Ice is a water biome, handled separately).
        Tundra | SnowyPlains | SnowyTaiga => GroundTexture::Snow,
        // Wet lowlands → dark tilled/mud earth.
        Swamp | Marsh => GroundTexture::Farmland,
        // Flower fields get an actual flowered tile, cycled by position so the
        // field reads mixed rather than a single colour.
        FlowerField => match (tile.x * 2 + tile.y).rem_euclid(3) {
            0 => GroundTexture::FlowersRed,
            1 => GroundTexture::FlowersWhite,
            _ => GroundTexture::FlowersBlue,
        },
        // Lusher / broken-up grass variants.
        Meadow | Hills | MushroomFields => GroundTexture::GrassVar,
        // Everything else (plains, all forests, jungle) → grass, with a
        // deterministic variant sprinkle to break up expanses.
        _ if (tile.x + tile.y).rem_euclid(5) == 0 => GroundTexture::GrassVar,
        _ => GroundTexture::Grass,
    }
}

/// Water biomes render with the water sprite even without a river overlay.
fn is_water_biome(biome: Biome) -> bool {
    matches!(
        biome,
        Biome::Ocean | Biome::Lake | Biome::River | Biome::Ice
    )
}

/// The biome's ground tint, multiplied onto its base tile.
fn biome_tint(biome: Biome) -> Color {
    let [r, g, b] = biome.properties().tint;
    Color::srgb_u8(r, g, b)
}

/// Rendered rock size (fraction of a tile) for a decoration rock.
fn rock_scale(size: RockSize) -> f32 {
    match size {
        RockSize::Small => 0.4,
        RockSize::Medium => 0.55,
        RockSize::Large => 0.75,
    }
}

/// The nature sprite a biome's trees render as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TreeSprite {
    Oak,
    Pine,
    SnowPine,
    DeadTree,
}

/// Biome-appropriate tree species: snow-capped conifers in snowy biomes, plain
/// conifers in cool/boreal, bare dead trees in hot-dry and wetland biomes, and
/// broadleaf oaks everywhere else. (No true cactus exists in the pixel packs, so
/// hot-dry biomes get the sparse dead tree instead — see the biome-render notes.)
fn biome_tree(biome: Biome) -> TreeSprite {
    use Biome::*;
    match biome {
        SnowyTaiga | SnowyPlains | Tundra => TreeSprite::SnowPine,
        PineForest | Taiga | Mountains => TreeSprite::Pine,
        Desert | Savanna | Badlands | Swamp | Marsh => TreeSprite::DeadTree,
        _ => TreeSprite::Oak,
    }
}

/// The inclusive tile bounds `(x0, y0, x1, y1)` of the render window around the
/// village anchor. Terrain and fog cover exactly this rectangle.
fn window_bounds() -> (i32, i32, i32, i32) {
    (
        VILLAGE_ANCHOR.x - WINDOW_RADIUS,
        VILLAGE_ANCHOR.y - WINDOW_RADIUS,
        VILLAGE_ANCHOR.x + WINDOW_RADIUS,
        VILLAGE_ANCHOR.y + WINDOW_RADIUS,
    )
}

/// Regenerate the terrain tiles inside the window around the village anchor.
fn window_terrain(seed: i64) -> Vec<TerrainTile> {
    let min = tile_to_chunk(
        VILLAGE_ANCHOR.x - WINDOW_RADIUS,
        VILLAGE_ANCHOR.y - WINDOW_RADIUS,
    );
    let max = tile_to_chunk(
        VILLAGE_ANCHOR.x + WINDOW_RADIUS,
        VILLAGE_ANCHOR.y + WINDOW_RADIUS,
    );
    let (x0, y0, x1, y1) = window_bounds();
    let mut tiles = Vec::new();
    for cy in min.chunk_y..=max.chunk_y {
        for cx in min.chunk_x..=max.chunk_x {
            for tile in generate_terrain_chunk(cx, cy, seed, WORLD_TERRAIN_OPTIONS) {
                if (x0..=x1).contains(&tile.x) && (y0..=y1).contains(&tile.y) {
                    tiles.push(tile);
                }
            }
        }
    }
    tiles
}

/// The set of revealed tile coordinates, for O(1) fog lookups.
fn revealed_lookup(tiles: &[TilePoint]) -> HashSet<(i32, i32)> {
    tiles.iter().map(|t| (t.x, t.y)).collect()
}

/// Three-tier fog visibility for a tile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FogState {
    /// Committed-revealed: no overlay, terrain shown fully.
    Clear,
    /// Tentatively uncovered by a scout still out — terrain under a dim haze.
    Dim,
    /// Undiscovered: full opaque fog.
    Full,
}

/// Classify a tile's fog tier. Revealed wins over provisional (a tile that has
/// committed shouldn't dim just because it's also still in the provisional set
/// mid-handoff), and provisional over full.
fn fog_state(
    revealed: &HashSet<(i32, i32)>,
    provisional: &HashSet<(i32, i32)>,
    x: i32,
    y: i32,
) -> FogState {
    if revealed.contains(&(x, y)) {
        FogState::Clear
    } else if provisional.contains(&(x, y)) {
        FogState::Dim
    } else {
        FogState::Full
    }
}

/// Fog of war: opaque dark tiles over every window tile the colony hasn't
/// revealed yet. Re-read each snapshot (the revealed set grows as cats walk), so
/// the fog visibly recedes. Drawn above the world sprites, so it also hides the
/// terrain, trees, buildings and cats sitting on undiscovered tiles.
fn render_fog(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    fog: Query<Entity, With<FogTile>>,
) {
    if !latest.is_changed() {
        return;
    }
    for entity in &fog {
        commands.entity(entity).despawn();
    }
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    let revealed = revealed_lookup(&colony.revealed_tiles);
    // Self-disabling fallback: with no revealed tiles (a pre-fog snapshot, or a
    // colony whose reveal state isn't populated yet) fogging the whole window
    // would black out the map — so show the full map until the set is non-empty.
    // Once the sim emits a non-empty revealed set, fog kicks in normally.
    if revealed.is_empty() {
        return;
    }
    let provisional = revealed_lookup(&colony.provisional_tiles);
    let (x0, y0, x1, y1) = window_bounds();
    for y in y0..=y1 {
        for x in x0..=x1 {
            let color = match fog_state(&revealed, &provisional, x, y) {
                FogState::Clear => continue,
                FogState::Dim => PROVISIONAL_FOG_COLOR,
                FogState::Full => FOG_COLOR,
            };
            let p = grid_to_world(x, y);
            commands.spawn((
                Sprite::from_color(color, Vec2::splat(TILE)),
                Transform::from_xyz(p.x, p.y, Z_FOG),
                FogTile,
            ));
        }
    }
}

/// Paved stone roads: an oriented road sprite at each `road_tiles` position (the
/// blueprint lays a cross from the shrine to the four walls; roads.rs paves more
/// as corridors wear). Re-read each snapshot; ground-level, below buildings/cats.
fn render_roads(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    art: Option<Res<InfraArt>>,
    roads: Query<Entity, With<RoadTile>>,
) {
    if !latest.is_changed() {
        return;
    }
    for entity in &roads {
        commands.entity(entity).despawn();
    }
    let (Some(colony), Some(art)) = (latest.0.as_ref().and_then(|w| w.colonies.first()), art)
    else {
        return;
    };
    let road_set: HashSet<(i32, i32)> = colony.road_tiles.iter().map(|t| (t.x, t.y)).collect();
    for &(x, y) in &road_set {
        let sprite = road_sprite_kind(
            road_set.contains(&(x, y - 1)),
            road_set.contains(&(x, y + 1)),
            road_set.contains(&(x + 1, y)),
            road_set.contains(&(x - 1, y)),
        );
        let p = grid_to_world(x, y);
        commands.spawn((
            Sprite {
                image: art.road(sprite),
                custom_size: Some(Vec2::splat(TILE)),
                ..default()
            },
            Transform::from_xyz(p.x, p.y, Z_ROAD),
            RoadTile,
        ));
    }
}

fn render_buildings(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    art: Option<Res<BuildingArt>>,
    sprites: Query<Entity, BuildingEntities>,
) {
    if !latest.is_changed() {
        return;
    }
    for entity in &sprites {
        commands.entity(entity).despawn();
    }
    let (Some(colony), Some(art)) = (latest.0.as_ref().and_then(|w| w.colonies.first()), art)
    else {
        return;
    };
    for building in &colony.buildings {
        // Cutaway top-down interior: the footprint is drawn as a FLOOR (flat on
        // the ground, no roof) with a centred workstation prop, viewed straight
        // from above. Walls render as the palisade — skip.
        let Some((floor_kind, prop)) = building_interior(building.building_type) else {
            continue;
        };
        let w = building.footprint.width.max(1) as f32;
        let h = building.footprint.height.max(1) as f32;
        let nw = building.world_position;
        let cx = (nw.x as f32 + (w - 1.0) / 2.0) * TILE;
        let cy = -((nw.y as f32 + (h - 1.0) / 2.0) * TILE);
        // Floor fills the footprint, flat on the ground (low z, not y-sorted).
        commands.spawn((
            Sprite {
                image: art.floor(floor_kind),
                custom_size: Some(Vec2::new(w * TILE, h * TILE)),
                ..default()
            },
            Anchor::CENTER,
            Transform::from_xyz(cx, cy, Z_BUILDING_FLOOR),
            BuildingSprite,
        ));
        // Centred workstation prop, y-sorted so cats pass in front of / behind it.
        if let Some((image, tiles)) = art.prop(prop) {
            commands.spawn((
                Sprite {
                    image,
                    custom_size: Some(Vec2::new(tiles.x * TILE, tiles.y * TILE)),
                    ..default()
                },
                Anchor::CENTER,
                Transform::from_xyz(cx, cy, ysort_z(cy) + 0.2),
                BuildingSprite,
            ));
        }
    }
}

/// Draw the village palisade ring (perimeter edges of the claimed-tile set) with
/// the gate sprite at the gate opening.
fn render_walls(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    art: Option<Res<InfraArt>>,
    existing: Query<Entity, With<WallVis>>,
) {
    if !latest.is_changed() {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let (Some(colony), Some(art)) = (latest.0.as_ref().and_then(|w| w.colonies.first()), art)
    else {
        return;
    };
    let claimed: HashSet<(i32, i32)> = colony.claimed_tiles.iter().map(|t| (t.x, t.y)).collect();
    let gate_edge = colony
        .village_gate
        .map(|g| ((g.x, g.y), gate_side_to_wall(g.side)));

    for (tile, side) in wall_edges(&claimed, gate_edge) {
        let (pos, rot) = wall_edge_transform(tile, side);
        commands.spawn((
            Sprite {
                image: art.palisade.clone(),
                custom_size: Some(Vec2::splat(TILE)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, ysort_z(pos.y))
                .with_rotation(Quat::from_rotation_z(rot)),
            WallVis,
        ));
    }

    // Gate sprite at the opening (if the gate edge is on the perimeter).
    if let Some((tile, side)) = gate_edge {
        let (pos, rot) = wall_edge_transform(tile, side);
        commands.spawn((
            Sprite {
                image: art.gate.clone(),
                custom_size: Some(Vec2::splat(TILE)),
                ..default()
            },
            Transform::from_xyz(pos.x, pos.y, ysort_z(pos.y) + 0.1)
                .with_rotation(Quat::from_rotation_z(rot)),
            WallVis,
        ));
    }
}

fn render_zones(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    sprites: Query<Entity, With<ZoneSprite>>,
) {
    if !latest.is_changed() {
        return;
    }
    for entity in &sprites {
        commands.entity(entity).despawn();
    }
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    for zone in &colony.zones {
        let (x0, x1) = (zone.x1.min(zone.x2), zone.x1.max(zone.x2));
        let (y0, y1) = (zone.y1.min(zone.y2), zone.y1.max(zone.y2));
        let w = (x1 - x0 + 1) as f32 * TILE;
        let h = (y1 - y0 + 1) as f32 * TILE;
        let cx = (x0 as f32 + x1 as f32) / 2.0 * TILE;
        let cy = -(y0 as f32 + y1 as f32) / 2.0 * TILE;
        commands.spawn((
            Sprite::from_color(zone_color(zone.kind), Vec2::new(w, h)),
            Transform::from_xyz(cx, cy, Z_ZONE),
            ZoneSprite,
        ));
    }
}

/// Draw each player stockpile as an amber overlay + a pile prop sized to its
/// contents + a dominant-resource label. The shrine reservoir is skipped (it's
/// always present and sits on the village).
/// Gather-spot flag: a teal banner (distinct from pile/accept tints) on a wood
/// pole, carrying the spot's resource icon — marks a temporary gather drop.
const GATHER_FLAG_COLOR: Color = Color::srgb(0.24, 0.60, 0.52);
const GATHER_POLE_COLOR: Color = Color::srgb(0.34, 0.25, 0.16);

#[allow(clippy::type_complexity)]
fn render_stockpiles(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    selection: Res<StockpileSelection>,
    art: Option<Res<PropArt>>,
    icons: Option<Res<IconArt>>,
    existing: Query<Entity, StockpileEntities>,
) {
    if !latest.is_changed() && !selection.is_changed() {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let (Some(colony), Some(art)) = (latest.0.as_ref().and_then(|w| w.colonies.first()), art)
    else {
        return;
    };
    for pile in &colony.stockpiles {
        let is_shrine = pile.id == SHRINE_STOCKPILE_ID;
        let (x0, x1) = (pile.x1.min(pile.x2), pile.x1.max(pile.x2));
        let (y0, y1) = (pile.y1.min(pile.y2), pile.y1.max(pile.y2));
        let w = (x1 - x0 + 1) as f32 * TILE;
        let h = (y1 - y0 + 1) as f32 * TILE;
        let cx = (x0 as f32 + x1 as f32) / 2.0 * TILE;
        let cy = -(y0 as f32 + y1 as f32) / 2.0 * TILE;

        let total = resource_total(&pile.contents);
        let dominant = dominant_resource(&pile.contents);

        // The shrine reservoir is de-emphasized: its pile prop floats above the
        // village buildings so the colony's stock reads as a visible pile, but
        // it gets no overlay rect / accept label / selection (always General,
        // can't be removed).
        if is_shrine {
            if let Some(dominant) = dominant {
                commands.spawn((
                    Sprite {
                        image: art.pile(pile_prop(dominant)),
                        custom_size: Some(Vec2::splat(pile_scale(total))),
                        ..default()
                    },
                    Transform::from_xyz(cx, cy, ysort_z(cy) + 2.0),
                    StockpileVis,
                ));
                commands.spawn((
                    Text2d::new(format!(
                        "{} {}",
                        resource_kind_name(dominant),
                        total.round() as i64
                    )),
                    TextFont {
                        font_size: FontSize::Px(9.0),
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 0.92, 0.72, 0.95)),
                    Transform::from_xyz(cx, cy - h / 2.0 - TILE * 0.25, ysort_z(cy) + 2.5),
                    StockpileVis,
                ));
            }
            continue;
        }

        // Player pile: selection outline, an accept-tinted overlay, an
        // accept-type label (always — so limited piles read even while empty),
        // and a pile prop when non-empty.
        if selection.selected.as_deref() == Some(pile.id.as_str()) {
            commands.spawn((
                Sprite::from_color(
                    Color::srgba(1.0, 0.85, 0.30, 0.50),
                    Vec2::new(w + 6.0, h + 6.0),
                ),
                Transform::from_xyz(cx, cy, Z_ZONE - 0.1),
                StockpileHighlight,
            ));
        }
        commands.spawn((
            Sprite::from_color(accept_overlay_color(&pile.accepts), Vec2::new(w, h)),
            Transform::from_xyz(cx, cy, Z_ZONE),
            StockpileVis,
        ));

        let mut label = accepts_label(&pile.accepts);
        if let Some(dominant) = dominant {
            label.push_str(&format!(
                "  {} {}",
                resource_kind_name(dominant),
                total.round() as i64
            ));
            commands.spawn((
                Sprite {
                    image: art.pile(pile_prop(dominant)),
                    custom_size: Some(Vec2::splat(pile_scale(total))),
                    ..default()
                },
                Transform::from_xyz(cx, cy, ysort_z(cy)),
                StockpileVis,
            ));
        }
        commands.spawn((
            Text2d::new(label),
            TextFont {
                font_size: FontSize::Px(9.0),
                ..default()
            },
            TextColor(Color::srgba(1.0, 0.92, 0.72, 0.95)),
            Transform::from_xyz(cx, cy - h / 2.0 - TILE * 0.25, ysort_z(cy) + 0.5),
            StockpileVis,
        ));

        // A gather spot (P16) is a temporary, resource-typed drop — often out
        // beyond the claimed area. Fly a little resource-icon flag above it so it
        // reads as distinct from an ordinary player stockpile, regardless of how
        // much it's currently holding.
        if let (Some(gs), Some(icons)) = (pile.gather_spot.as_ref(), icons.as_ref()) {
            let flag_y = cy + h / 2.0 + TILE * 0.5;
            commands.spawn((
                Sprite::from_color(GATHER_POLE_COLOR, Vec2::new(TILE * 0.14, TILE)),
                Transform::from_xyz(cx, flag_y, ysort_z(cy) + 3.0),
                StockpileVis,
            ));
            commands.spawn((
                Sprite::from_color(GATHER_FLAG_COLOR, Vec2::splat(TILE * 0.66)),
                Transform::from_xyz(cx, flag_y + TILE * 0.45, ysort_z(cy) + 3.1),
                StockpileVis,
            ));
            commands.spawn((
                Sprite {
                    image: icons.get(hud_res_of(gs.kind)),
                    custom_size: Some(Vec2::splat(TILE * 0.46)),
                    ..default()
                },
                Transform::from_xyz(cx, flag_y + TILE * 0.45, ysort_z(cy) + 3.2),
                StockpileVis,
            ));
        }
    }
}

/// Show/hide the remove-stockpile panel for the selected (non-shrine) pile, and
/// clear the selection if the pile is gone.
fn update_remove_panel(
    latest: Res<LatestSnapshot>,
    mut selection: ResMut<StockpileSelection>,
    mut panel: Query<&mut Node, With<RemovePanel>>,
    mut text: Query<&mut Text, With<RemovePanelText>>,
) {
    if !latest.is_changed() && !selection.is_changed() {
        return;
    }
    let (Ok(mut node), Ok(mut text)) = (panel.single_mut(), text.single_mut()) else {
        return;
    };
    let pile = selection.selected.as_deref().and_then(|id| {
        latest
            .0
            .as_ref()
            .and_then(|w| w.colonies.first())
            .and_then(|c| c.stockpiles.iter().find(|s| s.id == id))
    });
    match pile {
        Some(pile) => {
            node.display = Display::Flex;
            let total = resource_total(&pile.contents);
            let dominant = dominant_resource(&pile.contents).map_or("empty", resource_kind_name);
            text.0 = format!("Stockpile\n{dominant} {}", total.round() as i64);
        }
        None => {
            node.display = Display::None;
            if selection.selected.is_some() {
                selection.selected = None;
            }
        }
    }
}

/// Send RemoveStockpile when the remove button is clicked.
fn handle_remove_button(
    session: Res<Session>,
    mut selection: ResMut<StockpileSelection>,
    mut outgoing: ResMut<OutgoingActions>,
    mut button: Query<(&Interaction, &mut ImageNode), With<RemoveStockpileButton>>,
) {
    for (interaction, mut image) in &mut button {
        match interaction {
            Interaction::Pressed => {
                image.color = BTN_PRESS;
                if let (Some(id), true) = (selection.selected.clone(), session.ready) {
                    outgoing.0.push(ClientAction::RemoveStockpile {
                        session_id: session.session_id.clone(),
                        nickname: "Desktop Cat".to_string(),
                        sig: session.sig.clone(),
                        stockpile_id: id,
                    });
                    selection.selected = None;
                }
            }
            Interaction::Hovered => image.color = BTN_HOVER,
            Interaction::None => image.color = BTN_IDLE,
        }
    }
}

/// Toggle the officers panel with the `O` key.
fn toggle_officers(keys: Res<ButtonInput<KeyCode>>, mut ui: ResMut<OfficersUi>) {
    if keys.just_pressed(KeyCode::KeyO) {
        ui.visible = !ui.visible;
    }
}

/// Show/hide the officers panel and refresh each role row's holder name.
fn update_officers_panel(
    latest: Res<LatestSnapshot>,
    ui: Res<OfficersUi>,
    mut panel: Query<&mut Node, With<OfficersPanel>>,
    mut rows: Query<(&OfficerRow, &mut Text)>,
) {
    if let Ok(mut node) = panel.single_mut() {
        node.display = if ui.visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !latest.is_changed() && !ui.is_changed() {
        return;
    }
    let colony = latest.0.as_ref().and_then(|w| w.colonies.first());
    for (row, mut text) in &mut rows {
        let holder = colony.and_then(|c| officer_holder_name(c, row.0));
        text.0 = format!(
            "{}: {}",
            officer_role_name(row.0),
            holder.unwrap_or("vacant")
        );
    }
}

/// Appoint the selected cat to a role when an "Appoint <role>" button is clicked.
fn handle_appoint_buttons(
    session: Res<Session>,
    selection: Res<Selection>,
    mut outgoing: ResMut<OutgoingActions>,
    mut buttons: Query<(&Interaction, &AppointButton, &mut ImageNode), Changed<Interaction>>,
) {
    for (interaction, appoint, mut image) in &mut buttons {
        match interaction {
            Interaction::Pressed => {
                image.color = BTN_PRESS;
                if let (Some(cat), true) = (selection.selected.clone(), session.ready) {
                    outgoing.0.push(ClientAction::AssignOfficer {
                        session_id: session.session_id.clone(),
                        nickname: "Desktop Cat".to_string(),
                        sig: session.sig.clone(),
                        role: appoint.0,
                        cat_id: cat,
                    });
                }
            }
            Interaction::Hovered => image.color = BTN_HOVER,
            Interaction::None => image.color = BTN_IDLE,
        }
    }
}

/// Repaint the boost button's label from the selected cat's live `boosted`
/// flag, so the toggle round-trips visibly once the stream echoes the change.
fn update_boost_button(
    latest: Res<LatestSnapshot>,
    selection: Res<Selection>,
    mut text: Query<&mut Text, With<BoostButtonText>>,
) {
    if !latest.is_changed() && !selection.is_changed() {
        return;
    }
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let boosted = selected_cat(&latest, &selection).is_some_and(|c| c.boosted);
    text.0 = boost_button_label(boosted).to_string();
}

/// Toggle the selected cat's priority flag when the Boost button is clicked,
/// flipping off the cat's current `boosted` state read from the live snapshot.
#[allow(clippy::type_complexity)]
fn handle_boost_button(
    session: Res<Session>,
    selection: Res<Selection>,
    latest: Res<LatestSnapshot>,
    mut outgoing: ResMut<OutgoingActions>,
    mut buttons: Query<(&Interaction, &mut ImageNode), (Changed<Interaction>, With<BoostButton>)>,
) {
    for (interaction, mut image) in &mut buttons {
        match interaction {
            Interaction::Pressed => {
                image.color = BTN_PRESS;
                if let (Some(cat_id), true) = (selection.selected.clone(), session.ready) {
                    let current = selected_cat(&latest, &selection).is_some_and(|c| c.boosted);
                    outgoing.0.push(ClientAction::BoostCat {
                        session_id: session.session_id.clone(),
                        nickname: "Desktop Cat".to_string(),
                        sig: session.sig.clone(),
                        cat_id,
                        boosted: !current,
                    });
                }
            }
            Interaction::Hovered => image.color = BTN_HOVER,
            Interaction::None => image.color = BTN_IDLE,
        }
    }
}

/// The currently selected, still-living cat in the latest snapshot (if any).
fn selected_cat<'a>(latest: &'a LatestSnapshot, selection: &Selection) -> Option<&'a CatSnapshot> {
    let id = selection.selected.as_deref()?;
    latest
        .0
        .as_ref()
        .and_then(|w| w.colonies.first())
        .and_then(|c| c.cats.iter().find(|k| k.id == id && k.death_time.is_none()))
}

/// Vacate a role when its "x" button is clicked.
fn handle_vacate_buttons(
    session: Res<Session>,
    mut outgoing: ResMut<OutgoingActions>,
    mut buttons: Query<(&Interaction, &VacateButton, &mut ImageNode), Changed<Interaction>>,
) {
    for (interaction, vacate, mut image) in &mut buttons {
        match interaction {
            Interaction::Pressed => {
                image.color = BTN_PRESS;
                if session.ready {
                    outgoing.0.push(ClientAction::UnassignOfficer {
                        session_id: session.session_id.clone(),
                        nickname: "Desktop Cat".to_string(),
                        sig: session.sig.clone(),
                        role: vacate.0,
                    });
                }
            }
            Interaction::Hovered => image.color = BTN_HOVER,
            Interaction::None => image.color = BTN_IDLE,
        }
    }
}

/// Cat sprite size (32x64 cell → 1:2 aspect). Rendered larger than one tile so
/// cats stay readable + charming at the small tile.
const CAT_SIZE: Vec2 = Vec2::new(TILE * 1.4, TILE * 2.8);
/// Constant walk speed for body movement (world units/sec ≈ 3 tiles/sec) so
/// cats visibly stride tile-to-tile and never teleport.
const BODY_WALK_SPEED: f32 = TILE * 3.0;
/// Snap a body to its target only if it falls this absurdly far behind (well
/// off-screen) — otherwise it always walks there at [`BODY_WALK_SPEED`].
const BODY_MAX_LAG: f32 = TILE * 40.0;
/// A body is "moving" (walk-animated) while more than this far from its target.
const BODY_MOVE_EPS: f32 = 1.5;
/// Minimum world-space delta to derive a new facing from.
const FACING_EPS: f32 = TILE * 0.15;
/// World base position (bottom-anchor point) for a cat/raider on tile `(x, y)`.
fn body_base(x: i32, y: i32) -> Vec2 {
    let p = grid_to_world(x, y);
    Vec2::new(p.x, p.y - TILE * 0.5)
}

/// Reconcile persistent cat bodies with the snapshot: update each living cat's
/// glide target + facing (spawning new cats, despawning gone/dead ones), then
/// rebuild the follow-along overlays (hat / carried item / selection ring).
fn sync_cats(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    selection: Res<Selection>,
    sheets: Option<Res<SpriteSheets>>,
    mut bodies: ResMut<CatBodies>,
    mut cats: Query<(&Transform, &mut MoveTarget, &mut AnimSprite), With<CatBody>>,
    overlays: Query<Entity, With<CatOverlay>>,
) {
    if !latest.is_changed() && !selection.is_changed() {
        return;
    }
    let (Some(colony), Some(sheets)) = (latest.0.as_ref().and_then(|w| w.colonies.first()), sheets)
    else {
        return;
    };
    // Overlays are cheap and their state (hat/carry/selection) changes; rebuild.
    for entity in &overlays {
        commands.entity(entity).despawn();
    }

    let mut live = HashSet::new();
    for cat in &colony.cats {
        if cat.death_time.is_some() {
            continue;
        }
        live.insert(cat.id.clone());
        let target = body_base(cat.position.x, cat.position.y);

        if let Some(&entity) = bodies.0.get(&cat.id) {
            if let Ok((transform, mut move_target, mut anim)) = cats.get_mut(entity) {
                // Face the direction of travel; keep the last facing when idle.
                if let Some(group) = facing_from_delta(target - transform.translation.truncate()) {
                    anim.group = group;
                }
                move_target.0 = target;
            }
        } else {
            let group = cat
                .destination
                .and_then(|d| facing_from_delta(body_base(d.x, d.y) - target))
                .unwrap_or(0);
            let entity = commands
                .spawn((
                    Sprite {
                        image: sheets.cat.clone(),
                        texture_atlas: Some(TextureAtlas {
                            layout: sheets.layout.clone(),
                            index: atlas_index(group, 0),
                        }),
                        custom_size: Some(CAT_SIZE),
                        ..default()
                    },
                    Anchor::BOTTOM_CENTER,
                    Transform::from_xyz(target.x, target.y, ysort_z(target.y)),
                    CatBody(cat.id.clone()),
                    MoveTarget(target),
                    AnimSprite {
                        group,
                        moving: false,
                    },
                ))
                .id();
            bodies.0.insert(cat.id.clone(), entity);
        }

        // Overlays follow the (interpolated) body each frame via FollowCat; the
        // offset.z is a small bias relative to the cat's y-sorted depth.
        if selection.selected.as_deref() == Some(cat.id.as_str()) {
            spawn_cat_overlay(
                &mut commands,
                &cat.id,
                Vec3::new(0.0, CAT_SIZE.y * 0.35, -0.1),
                Sprite::from_color(Color::srgb(1.0, 0.93, 0.30), Vec2::splat(TILE * 0.7)),
            );
        }
        if let Some(spec) = cat.specialization {
            spawn_cat_overlay(
                &mut commands,
                &cat.id,
                Vec3::new(0.0, CAT_SIZE.y * 0.78, 0.5),
                Sprite {
                    image: sheets.hat(spec),
                    custom_size: Some(Vec2::splat(TILE * 0.55)),
                    ..default()
                },
            );
        }
        if let Some(carrying) = &cat.carrying {
            spawn_cat_overlay(
                &mut commands,
                &cat.id,
                Vec3::new(TILE * 0.3, CAT_SIZE.y * 0.55, 0.6),
                Sprite::from_color(carrying_color(carrying.kind), Vec2::splat(TILE * 0.28)),
            );
        }
        // Boosted cats wear a bright gold star above the head so a priority pick
        // reads at a glance without opening the inspector.
        if cat.boosted {
            spawn_cat_overlay(
                &mut commands,
                &cat.id,
                Vec3::new(0.0, CAT_SIZE.y * 1.05, 0.7),
                Sprite::from_color(Color::srgb(1.0, 0.90, 0.28), Vec2::splat(TILE * 0.42)),
            );
        }
    }

    // Despawn bodies for cats that died or vanished.
    bodies.0.retain(|id, entity| {
        if live.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });
}

fn spawn_cat_overlay(commands: &mut Commands, id: &str, offset: Vec3, sprite: Sprite) {
    commands.spawn((
        sprite,
        Transform::from_translation(offset),
        CatOverlay,
        FollowCat {
            id: id.to_string(),
            offset,
        },
    ));
}

/// Reconcile persistent raider bodies (same glide treatment as cats).
fn sync_raiders(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    sheets: Option<Res<SpriteSheets>>,
    mut bodies: ResMut<RaiderBodies>,
    mut raiders: Query<(&Transform, &mut MoveTarget, &mut AnimSprite), With<RaiderBody>>,
) {
    if !latest.is_changed() {
        return;
    }
    let (Some(colony), Some(sheets)) = (latest.0.as_ref().and_then(|w| w.colonies.first()), sheets)
    else {
        return;
    };
    let mut live = HashSet::new();
    for raider in &colony.raiders {
        live.insert(raider.id.clone());
        let target = body_base(raider.position.x, raider.position.y);
        // Raiders march on the village — face the anchor.
        let group =
            facing_from_delta(body_base(colony.anchor.x, colony.anchor.y) - target).unwrap_or(0);
        let moving = matches!(raider.status, RaiderStatus::Advancing);

        if let Some(&entity) = bodies.0.get(&raider.id) {
            if let Ok((_, mut move_target, mut anim)) = raiders.get_mut(entity) {
                anim.group = group;
                anim.moving = moving;
                move_target.0 = target;
            }
        } else {
            let entity = commands
                .spawn((
                    Sprite {
                        image: sheets.raider.clone(),
                        texture_atlas: Some(TextureAtlas {
                            layout: sheets.layout.clone(),
                            index: atlas_index(group, 0),
                        }),
                        custom_size: Some(CAT_SIZE),
                        ..default()
                    },
                    Anchor::BOTTOM_CENTER,
                    Transform::from_xyz(target.x, target.y, ysort_z(target.y)),
                    RaiderBody,
                    MoveTarget(target),
                    AnimSprite { group, moving },
                ))
                .id();
            bodies.0.insert(raider.id.clone(), entity);
        }
    }
    bodies.0.retain(|id, entity| {
        if live.contains(id) {
            true
        } else {
            commands.entity(*entity).despawn();
            false
        }
    });
}

/// Reconcile the visiting-trader body (a gold-tinted merchant cat with a pack):
/// present only while a trader is visiting, gliding to its position (arriving ->
/// gate, trading = idle at the gate, departing -> away).
fn sync_trader(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    sheets: Option<Res<SpriteSheets>>,
    mut trader_q: Query<(Entity, &Transform, &mut MoveTarget, &mut AnimSprite), With<TraderBody>>,
) {
    if !latest.is_changed() {
        return;
    }
    let (Some(colony), Some(sheets)) = (latest.0.as_ref().and_then(|w| w.colonies.first()), sheets)
    else {
        return;
    };
    match &colony.trader {
        Some(trader) => {
            let target = body_base(trader.position.x, trader.position.y);
            if let Ok((_, transform, mut move_target, mut anim)) = trader_q.single_mut() {
                if let Some(group) = facing_from_delta(target - transform.translation.truncate()) {
                    anim.group = group;
                }
                move_target.0 = target;
            } else {
                // Initially face the village gate/anchor.
                let group = facing_from_delta(body_base(colony.anchor.x, colony.anchor.y) - target)
                    .unwrap_or(6);
                commands.spawn((
                    Sprite {
                        image: sheets.cat.clone(),
                        texture_atlas: Some(TextureAtlas {
                            layout: sheets.layout.clone(),
                            index: atlas_index(group, 0),
                        }),
                        custom_size: Some(CAT_SIZE),
                        // Warm gold tint marks the merchant as friendly, not a raider.
                        color: Color::srgb(1.0, 0.85, 0.52),
                        ..default()
                    },
                    Anchor::BOTTOM_CENTER,
                    Transform::from_xyz(target.x, target.y, ysort_z(target.y)),
                    TraderBody,
                    MoveTarget(target),
                    AnimSprite {
                        group,
                        moving: false,
                    },
                    children![(
                        // A gold trade-pack on the merchant's back.
                        Sprite::from_color(Color::srgb(0.86, 0.66, 0.24), Vec2::splat(TILE * 0.5)),
                        Transform::from_xyz(TILE * 0.3, CAT_SIZE.y * 0.5, 0.6),
                    )],
                ));
            }
        }
        None => {
            for (entity, ..) in &trader_q {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Lift the trader body above the fog layer so the approaching merchant stays
/// visible while it walks in across still-fogged ground (run after `move_bodies`,
/// which otherwise y-sorts it back below the fog).
fn lift_trader_above_fog(mut trader: Query<&mut Transform, With<TraderBody>>) {
    for mut transform in &mut trader {
        transform.translation.z = Z_FOG + 5.0;
    }
}

/// Walk every persistent body toward its target each frame at a constant speed
/// (so it strides tile-to-tile, never teleporting), and flag it moving while
/// it's still en route.
fn move_bodies(time: Res<Time>, mut bodies: Query<(&mut Transform, &MoveTarget, &mut AnimSprite)>) {
    let step = BODY_WALK_SPEED * time.delta_secs();
    for (mut transform, target, mut anim) in &mut bodies {
        let current = transform.translation.truncate();
        let next = walk_step(current, target.0, step, BODY_MAX_LAG);
        transform.translation.x = next.x;
        transform.translation.y = next.y;
        // Re-sort by the body's live base y so it layers as it walks.
        transform.translation.z = ysort_z(next.y);
        anim.moving = is_moving(current, target.0, BODY_MOVE_EPS);
    }
}

/// Move each cat overlay onto its cat's current (interpolated) position, sharing
/// the cat's y-sorted depth (plus a small per-overlay bias).
fn follow_overlays(
    bodies: Query<(&CatBody, &Transform), Without<FollowCat>>,
    mut overlays: Query<(&FollowCat, &mut Transform)>,
) {
    let positions: HashMap<&str, Vec2> = bodies
        .iter()
        .map(|(body, transform)| (body.0.as_str(), transform.translation.truncate()))
        .collect();
    for (follow, mut transform) in &mut overlays {
        if let Some(pos) = positions.get(follow.id.as_str()) {
            transform.translation.x = pos.x + follow.offset.x;
            transform.translation.y = pos.y + follow.offset.y;
            transform.translation.z = ysort_z(pos.y) + follow.offset.z;
        }
    }
}

/// Cycle the walk frames of moving sheet-animated sprites (~8fps); idle sprites
/// hold frame 0. Client-only eye-candy — not synced to the sim.
fn animate_sprites(time: Res<Time>, mut sprites: Query<(&AnimSprite, &mut Sprite)>) {
    let frame = (time.elapsed_secs() * 8.0) as usize % 4;
    for (anim, mut sprite) in &mut sprites {
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = atlas_index(anim.group, if anim.moving { frame } else { 0 });
        }
    }
}

/// Left-click a cat marker to inspect it; click empty ground or the same cat to
/// deselect. Read-only — resolves the nearest cat within half a tile.
#[allow(clippy::too_many_arguments)]
fn select_cat(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    ui: Query<&Interaction, With<Button>>,
    tools: Res<Tools>,
    latest: Res<LatestSnapshot>,
    mut selection: ResMut<Selection>,
    mut stockpile_selection: ResMut<StockpileSelection>,
) {
    // Selection is the Inspect-mode action only.
    if tools.mode != ToolMode::Inspect {
        return;
    }
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    // Ignore clicks that land on a toolbar button.
    if ui.iter().any(|i| !matches!(i, Interaction::None)) {
        return;
    }
    let Some(world) = cursor_world(&windows, &camera) else {
        return;
    };
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    let cats: Vec<(String, Vec2)> = colony
        .cats
        .iter()
        .filter(|c| c.death_time.is_none())
        .map(|c| (c.id.clone(), grid_to_world(c.position.x, c.position.y)))
        .collect();
    let picked = nearest_id(world, &cats, TILE * 0.5);
    if picked.is_some() {
        // A cat wins the click; drop any stockpile selection.
        stockpile_selection.selected = None;
        selection.selected = toggle_selection(selection.selected.as_deref(), picked);
        return;
    }
    // Otherwise, clicking a non-shrine stockpile selects it (for removal).
    let tile = world_to_tile(world);
    let pile = colony
        .stockpiles
        .iter()
        .find(|s| s.id != SHRINE_STOCKPILE_ID && point_in_stockpile(tile, s));
    selection.selected = None;
    stockpile_selection.selected = toggle_selection(
        stockpile_selection.selected.as_deref(),
        pile.map(|s| s.id.clone()),
    );
}

/// Right-click a building to inspect it; right-click empty ground or the same
/// building again to deselect. Shift+right-click is handled by
/// `cycle_stacked_selection` instead, so bail when shift is held.
fn select_building(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    ui: Query<&Interaction, With<Button>>,
    latest: Res<LatestSnapshot>,
    mut selection: ResMut<BuildingSelection>,
) {
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        return;
    }
    if !buttons.just_pressed(MouseButton::Right)
        || click_action(MouseButton::Right) != Some(ClickTarget::Building)
    {
        return;
    }
    if ui.iter().any(|i| !matches!(i, Interaction::None)) {
        return;
    }
    let Some(world) = cursor_world(&windows, &camera) else {
        return;
    };
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    // Skip Walls (rendered as the palisade, not a point marker).
    let buildings: Vec<(String, Vec2)> = colony
        .buildings
        .iter()
        .filter(|b| building_texture(b.building_type).is_some())
        .map(|b| {
            (
                b.id.clone(),
                grid_to_world(b.world_position.x, b.world_position.y),
            )
        })
        .collect();
    let picked = nearest_id(world, &buildings, TILE * 0.9);
    selection.selected = toggle_selection(selection.selected.as_deref(), picked);
}

/// Shift+right-click cycles through every inspectable stacked under the cursor —
/// a cat standing on a workshop, a pile on a building tile — so each successive
/// click targets the next one (cat → building → stockpile → wrap). Routes the
/// pick to the matching inspector and clears the others.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn cycle_stacked_selection(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    ui: Query<&Interaction, With<Button>>,
    latest: Res<LatestSnapshot>,
    mut cat_sel: ResMut<Selection>,
    mut building_sel: ResMut<BuildingSelection>,
    mut pile_sel: ResMut<StockpileSelection>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if !shift || !buttons.just_pressed(MouseButton::Right) {
        return;
    }
    if ui.iter().any(|i| !matches!(i, Interaction::None)) {
        return;
    }
    let Some(world) = cursor_world(&windows, &camera) else {
        return;
    };
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    // Collect everything under the cursor, using the same hit radii as the
    // single-click picks so "stacked" means the same thing.
    let tile = world_to_tile(world);
    let mut stack: Vec<PickCandidate> = Vec::new();
    for cat in colony.cats.iter().filter(|c| c.death_time.is_none()) {
        let p = grid_to_world(cat.position.x, cat.position.y);
        if p.distance_squared(world) <= (TILE * 0.5).powi(2) {
            stack.push(PickCandidate {
                id: cat.id.clone(),
                kind: PickKind::Cat,
            });
        }
    }
    for b in colony
        .buildings
        .iter()
        .filter(|b| building_texture(b.building_type).is_some())
    {
        let p = grid_to_world(b.world_position.x, b.world_position.y);
        if p.distance_squared(world) <= (TILE * 0.9).powi(2) {
            stack.push(PickCandidate {
                id: b.id.clone(),
                kind: PickKind::Building,
            });
        }
    }
    for pile in colony
        .stockpiles
        .iter()
        .filter(|s| s.id != SHRINE_STOCKPILE_ID && point_in_stockpile(tile, s))
    {
        stack.push(PickCandidate {
            id: pile.id.clone(),
            kind: PickKind::Stockpile,
        });
    }
    // Stable cycle order (cats, then buildings, then stockpiles; by id within a
    // kind) so the sequence doesn't shuffle between clicks at a fixed cursor.
    stack.sort_by(|a, b| (a.kind as u8, &a.id).cmp(&(b.kind as u8, &b.id)));

    let current = cat_sel
        .selected
        .as_deref()
        .or(building_sel.selected.as_deref())
        .or(pile_sel.selected.as_deref());
    let Some(next) = cycle_stacked_pick(&stack, current).cloned() else {
        return;
    };
    cat_sel.selected = None;
    building_sel.selected = None;
    pile_sel.selected = None;
    match next.kind {
        PickKind::Cat => cat_sel.selected = Some(next.id),
        PickKind::Building => building_sel.selected = Some(next.id),
        PickKind::Stockpile => pile_sel.selected = Some(next.id),
    }
}

/// Re-resolve the selected building each tick and repaint its inspector panel;
/// hide it (and clear the selection) when the building is gone.
fn update_building_inspector(
    latest: Res<LatestSnapshot>,
    mut selection: ResMut<BuildingSelection>,
    mut panel: Query<&mut Node, With<BuildingInspectorPanel>>,
    mut text: Query<&mut Text, With<BuildingInspectorText>>,
) {
    if !latest.is_changed() && !selection.is_changed() {
        return;
    }
    let (Ok(mut node), Ok(mut text)) = (panel.single_mut(), text.single_mut()) else {
        return;
    };
    let found = selection.selected.as_deref().and_then(|id| {
        latest
            .0
            .as_ref()
            .and_then(|w| w.colonies.first())
            .and_then(|c| c.buildings.iter().find(|b| b.id == id).map(|b| (b, c)))
    });
    match found {
        Some((building, colony)) => {
            node.display = Display::Flex;
            text.0 = building_inspector_text(building, colony);
        }
        None => {
            node.display = Display::None;
            if selection.selected.is_some() {
                selection.selected = None;
            }
        }
    }
}

/// Re-resolve the selected cat by id each tick and repaint the inspector panel;
/// hide it (and clear the selection) when the cat is gone or dead.
fn update_inspector(
    latest: Res<LatestSnapshot>,
    mut selection: ResMut<Selection>,
    mut panel: Query<&mut Node, (With<InspectorPanel>, Without<NeedBar>)>,
    mut text: Query<&mut Text, With<InspectorText>>,
    mut bars: Query<(&mut Node, &mut BackgroundColor, &NeedBar), Without<InspectorPanel>>,
) {
    if !latest.is_changed() && !selection.is_changed() {
        return;
    }
    let (Ok(mut node), Ok(mut text)) = (panel.single_mut(), text.single_mut()) else {
        return;
    };
    let cat = selection.selected.as_deref().and_then(|id| {
        latest
            .0
            .as_ref()
            .and_then(|w| w.colonies.first())
            .and_then(|c| c.cats.iter().find(|k| k.id == id && k.death_time.is_none()))
    });
    match cat {
        Some(cat) => {
            node.display = Display::Flex;
            text.0 = inspector_text(cat);
            for (mut bar, mut color, need) in &mut bars {
                let value = cat_need_value(&cat.needs, need.0);
                bar.width = Val::Percent(value.clamp(0.0, 100.0) as f32);
                color.0 = need_bar_color(value);
            }
        }
        None => {
            node.display = Display::None;
            if selection.selected.is_some() {
                selection.selected = None;
            }
        }
    }
}

/// Esc closes any open inspector (cat card / building panel / stockpile remove).
/// Click-away already clears selection via the pick systems; this is the keyboard
/// escape hatch.
#[allow(clippy::too_many_arguments)]
fn close_inspectors_on_esc(
    keys: Res<ButtonInput<KeyCode>>,
    mut cat: ResMut<Selection>,
    mut building: ResMut<BuildingSelection>,
    mut stockpile: ResMut<StockpileSelection>,
    mut announcements: ResMut<AnnouncementsUi>,
    mut goods: ResMut<GoodsUi>,
    mut census: ResMut<CensusUi>,
    mut tree: ResMut<UpgradeTreeUi>,
    mut trade: ResMut<TradeUi>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        cat.selected = None;
        building.selected = None;
        stockpile.selected = None;
        announcements.visible = false;
        goods.visible = false;
        census.visible = false;
        tree.visible = false;
        trade.closed = true;
    }
}

/// Cursor position in world space, or `None` if off-window / no camera.
fn cursor_world(
    windows: &Query<&Window>,
    camera: &Query<(&Camera, &GlobalTransform), With<Camera2d>>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, cam_tf) = camera.single().ok()?;
    camera.viewport_to_world_2d(cam_tf, cursor).ok()
}

/// Small hover tooltip (P15 "small" tier): on mouse-hover over any world entity,
/// show a compact panel of its key live state near the cursor. Separate from the
/// right-click big inspector panels, which stay.
fn hover_tooltip(
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    ui: Query<&Interaction, With<Button>>,
    latest: Res<LatestSnapshot>,
    mut panel: Query<&mut Node, With<TooltipPanel>>,
    mut text: Query<&mut Text, With<TooltipText>>,
) {
    let (Ok(mut node), Ok(mut text)) = (panel.single_mut(), text.single_mut()) else {
        return;
    };
    // Don't cover the toolbar: suppress while hovering a button.
    let over_button = ui.iter().any(|i| !matches!(i, Interaction::None));
    let hovered = (!over_button)
        .then(|| windows.single().ok().and_then(|w| w.cursor_position()))
        .flatten()
        .zip(cursor_world(&windows, &camera))
        .and_then(|(cursor, world)| {
            let snapshot = latest.0.as_ref()?;
            let colony = snapshot.colonies.first()?;
            Some((cursor, hover_text(colony, snapshot.world_seed, world)?))
        });
    match hovered {
        Some((cursor, tip)) => {
            text.0 = tip;
            node.display = Display::Flex;
            node.left = Val::Px(cursor.x + 16.0);
            node.top = Val::Px(cursor.y + 16.0);
        }
        None => node.display = Display::None,
    }
}

/// The tooltip text for whatever sits under `world` — cats first, then buildings,
/// then stockpiles, and finally the terrain tile itself (biome + resource), so a
/// hover always reads something.
fn hover_text(colony: &ColonySnapshot, world_seed: i64, world: Vec2) -> Option<String> {
    let cats: Vec<(String, Vec2)> = colony
        .cats
        .iter()
        .filter(|c| c.death_time.is_none())
        .map(|c| (c.id.clone(), grid_to_world(c.position.x, c.position.y)))
        .collect();
    if let Some(id) = nearest_id(world, &cats, TILE * 0.6)
        && let Some(cat) = colony.cats.iter().find(|c| c.id == id)
    {
        return Some(cat_tooltip(cat));
    }

    let buildings: Vec<(String, Vec2)> = colony
        .buildings
        .iter()
        .filter(|b| building_texture(b.building_type).is_some())
        .map(|b| {
            (
                b.id.clone(),
                grid_to_world(b.world_position.x, b.world_position.y),
            )
        })
        .collect();
    if let Some(id) = nearest_id(world, &buildings, TILE * 0.9)
        && let Some(b) = colony.buildings.iter().find(|b| b.id == id)
    {
        return Some(building_tooltip(b));
    }

    let tile = world_to_tile(world);
    if let Some(pile) = colony
        .stockpiles
        .iter()
        .find(|s| point_in_stockpile(tile, s))
    {
        return Some(stockpile_tooltip(pile));
    }

    Some(tile_tooltip(world_seed, tile.0, tile.1))
}

/// Hover text for a bare terrain tile: its climate biome and what it offers.
fn tile_tooltip(world_seed: i64, x: i32, y: i32) -> String {
    let biome = tile_climate_biome(world_seed as u32, x, y);
    let props = biome.properties();
    let feature = match derive_biome_decoration(x, y, world_seed, biome) {
        Some(DecorationRole::Tree { .. }) => "trees".to_string(),
        Some(DecorationRole::Rock { .. }) => "rocks".to_string(),
        None => resource_hint_label(props.resource).to_string(),
    };
    format!("{name}\n{feature}", name = props.name)
}

/// A short label for what a biome primarily offers the gather loop.
fn resource_hint_label(hint: ResourceHint) -> &'static str {
    match hint {
        ResourceHint::Wood => "wood",
        ResourceHint::Stone => "stone",
        ResourceHint::Ore => "ore",
        ResourceHint::Fish => "water (fish)",
        ResourceHint::Farmland => "farmland",
        ResourceHint::None => "open ground",
    }
}

/// Whole-percent through a production cycle (0..=100).
fn progress_pct(progress: f64) -> u32 {
    (progress.clamp(0.0, 1.0) * 100.0).round() as u32
}

/// A DF-style ASCII progress bar of `cells` characters, filled to `progress`.
fn progress_bar(progress: f64, cells: usize) -> String {
    let filled = (progress.clamp(0.0, 1.0) * cells as f64).round() as usize;
    let filled = filled.min(cells);
    format!("[{}{}]", "#".repeat(filled), "-".repeat(cells - filled))
}

/// The production line for a staffed producer: `making {output} [bar] {pct}%`.
fn production_line(output: &str, progress: f64) -> String {
    format!(
        "making {output} {} {}%",
        progress_bar(progress, 10),
        progress_pct(progress)
    )
}

/// Compact hover text for a cat: name, specialization + activity, needs summary.
fn cat_tooltip(cat: &CatSnapshot) -> String {
    let n = &cat.needs;
    format!(
        "{name}\n\
         {spec} - {activity}\n\
         hunger {h:.0}  thirst {t:.0}  rest {r:.0}  health {hp:.0}",
        name = cat.name,
        spec = specialization_name(cat.specialization),
        activity = activity_name(cat.activity),
        h = n.hunger,
        t = n.thirst,
        r = n.rest,
        hp = n.health,
    )
}

/// Compact hover text for a building: name/level, operational state, and — once
/// operational — its staffing and what it's making (from the live snapshot).
fn building_tooltip(building: &BuildingSnapshot) -> String {
    let mut out = format!(
        "{name}  Lv {lvl}",
        name = building_label(building.building_type),
        lvl = building.level,
    );
    if building.construction_progress < 100.0 {
        out.push_str(&format!(
            "\nunder construction {:.0}%",
            building.construction_progress
        ));
        return out;
    }
    if building.staff_cap > 0 {
        out.push_str(&format!(
            "\n{}/{} working",
            building.staff_count, building.staff_cap
        ));
        if let Some(making) = &building.production_output {
            out.push_str(&format!(" - making {making}"));
        }
    }
    out
}

/// Compact hover text for a stockpile: what it accepts + rough contents.
fn stockpile_tooltip(pile: &StockpileSnapshot) -> String {
    let title = if pile.id == SHRINE_STOCKPILE_ID {
        "Shrine reservoir"
    } else {
        "Stockpile"
    };
    format!(
        "{title}\naccepts {accepts}\ncontents ~{total:.0}",
        accepts = accepts_label(&pile.accepts),
        total = resource_total(&pile.contents),
    )
}

/// Toolbar tool-mode toggles: set the active mode, tint buttons by state, and
/// cancel any in-progress drag when leaving a zone mode.
fn handle_tool_buttons(
    mut tools: ResMut<Tools>,
    mut buttons: Query<(&Interaction, &ToolButton, &mut ImageNode)>,
) {
    for (interaction, button, mut image) in &mut buttons {
        if *interaction == Interaction::Pressed && tools.mode != button.0 {
            tools.mode = button.0;
            tools.drag = None;
        }
        let active = tools.mode == button.0;
        image.color = match (active, interaction) {
            (true, _) => BTN_ACTIVE,
            (false, Interaction::Hovered) => BTN_HOVER,
            (false, _) => BTN_IDLE,
        };
    }
}

/// Cycle the stockpile accept-type when its picker is clicked, and keep the
/// button label in sync with the current choice.
fn handle_accept_button(
    mut tools: ResMut<Tools>,
    mut button: AcceptButtonQuery,
    mut text: Query<&mut Text, With<AcceptButtonText>>,
) {
    for (interaction, mut image) in &mut button {
        match interaction {
            Interaction::Pressed => {
                image.color = BTN_PRESS;
                tools.accept = tools.accept.next();
            }
            Interaction::Hovered => image.color = BTN_HOVER,
            Interaction::None => image.color = BTN_IDLE,
        }
    }
    if tools.is_changed()
        && let Ok(mut text) = text.single_mut()
    {
        text.0 = format!("Accepts: {}", tools.accept.label());
    }
}

/// Click-drag a rectangle in a paint mode to designate an avoid/gather zone or a
/// stockpile; release sends the matching action. Esc cancels an in-progress drag.
#[allow(clippy::too_many_arguments)]
fn zone_paint(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    ui: Query<&Interaction, With<Button>>,
    session: Res<Session>,
    mut tools: ResMut<Tools>,
    mut outgoing: ResMut<OutgoingActions>,
) {
    let Some(kind) = tools.mode.paint_kind() else {
        tools.drag = None;
        return;
    };
    let accept = tools.accept;
    if keys.just_pressed(KeyCode::Escape) {
        tools.drag = None;
        return;
    }
    let over_ui = ui.iter().any(|i| !matches!(i, Interaction::None));
    let tile = cursor_world(&windows, &camera).map(world_to_tile);

    if buttons.just_pressed(MouseButton::Left)
        && !over_ui
        && let Some(tile) = tile
    {
        tools.drag = Some((tile, tile));
    } else if buttons.pressed(MouseButton::Left)
        && let (Some((start, _)), Some(tile)) = (tools.drag, tile)
    {
        tools.drag = Some((start, tile));
    } else if buttons.just_released(MouseButton::Left)
        && let Some((start, end)) = tools.drag.take()
    {
        let (min, max) = drag_tile_rect(start, end, ZONE_MAX_TILES);
        if !session.ready {
            warn!("session not ready; dropping paint action");
            return;
        }
        let a = TilePoint { x: min.0, y: min.1 };
        let b = TilePoint { x: max.0, y: max.1 };
        outgoing.0.push(match kind {
            PaintKind::Avoid | PaintKind::Gather => ClientAction::CreateZone {
                session_id: session.session_id.clone(),
                nickname: "Desktop Cat".to_string(),
                sig: session.sig.clone(),
                kind: if kind == PaintKind::Avoid {
                    ZoneKind::Avoid
                } else {
                    ZoneKind::Gather
                },
                a,
                b,
                duration_ms: ZONE_DURATION_MS,
            },
            PaintKind::Stockpile => ClientAction::DesignateStockpile {
                session_id: session.session_id.clone(),
                nickname: "Desktop Cat".to_string(),
                sig: session.sig.clone(),
                a,
                b,
                accepts: accept.kinds(),
            },
        });
    }
}

/// Redraw the translucent drag preview rectangle each frame while dragging.
fn render_zone_preview(
    mut commands: Commands,
    tools: Res<Tools>,
    existing: Query<Entity, With<ZonePreview>>,
) {
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    let (Some(kind), Some((start, end))) = (tools.mode.paint_kind(), tools.drag) else {
        return;
    };
    let (min, max) = drag_tile_rect(start, end, ZONE_MAX_TILES);
    let w = (max.0 - min.0 + 1) as f32 * TILE;
    let h = (max.1 - min.1 + 1) as f32 * TILE;
    let cx = (min.0 as f32 + max.0 as f32) / 2.0 * TILE;
    let cy = -(min.1 as f32 + max.1 as f32) / 2.0 * TILE;
    commands.spawn((
        Sprite::from_color(paint_preview_color(kind), Vec2::new(w, h)),
        // Just above committed zones so the preview reads on top.
        Transform::from_xyz(cx, cy, Z_ZONE + 0.5),
        ZonePreview,
    ));
}

fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
    mut inited: Local<bool>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let Ok((mut transform, mut projection)) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(projection) = projection.as_mut() else {
        return;
    };
    // Frame the village at the small tile on the first frame.
    if !*inited {
        *inited = true;
        projection.scale = DEFAULT_ZOOM;
    }
    let speed = 620.0 * time.delta_secs() * projection.scale;
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        transform.translation.x -= speed;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        transform.translation.x += speed;
    }
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        transform.translation.y += speed;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        transform.translation.y -= speed;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        let center = grid_to_world(VILLAGE_ANCHOR.x, VILLAGE_ANCHOR.y);
        transform.translation.x = center.x;
        transform.translation.y = center.y;
        projection.scale = DEFAULT_ZOOM;
    }
    // Middle-button drag pans the map (left = select cat, right = select
    // building).
    if buttons.pressed(MouseButton::Middle) {
        for ev in motion.read() {
            transform.translation.x -= ev.delta.x * projection.scale;
            transform.translation.y += ev.delta.y * projection.scale;
        }
    } else {
        motion.clear();
    }
    for ev in wheel.read() {
        projection.scale =
            (projection.scale * if ev.y > 0.0 { 0.9 } else { 1.1 }).clamp(MIN_ZOOM, MAX_ZOOM);
    }
}

fn update_hud(
    latest: Res<LatestSnapshot>,
    mut header: Query<&mut Text, (With<HudHeaderText>, Without<HudFooterText>)>,
    mut footer: Query<&mut Text, (With<HudFooterText>, Without<HudHeaderText>)>,
    mut values: HudResourceQuery,
) {
    if !latest.is_changed() {
        return;
    }
    let (Ok(mut header), Ok(mut footer)) = (header.single_mut(), footer.single_mut()) else {
        return;
    };
    let colony = latest.0.as_ref().and_then(|w| w.colonies.first());
    let Some(world) = latest.0.as_ref() else {
        header.0 = "connecting…".to_string();
        footer.0 = String::new();
        for (mut text, _) in &mut values {
            text.0 = "-".to_string();
        }
        return;
    };
    let Some(colony) = colony else {
        header.0 = format!(
            "online {}\nNo colony yet - press Found village.",
            world.online_count
        );
        footer.0 = String::new();
        for (mut text, _) in &mut values {
            text.0 = "-".to_string();
        }
        return;
    };
    header.0 = dashboard_header_text(colony, world.online_count);
    footer.0 = dashboard_footer_text(colony);
    let r = &colony.resources;
    let cap = &colony.storage.capacities;
    for (mut text, res) in &mut values {
        text.0 = hud_resource_value(res.0, r, cap);
    }
}

/// The HUD colony header (name / leader / pop / threat) shown above the resource
/// icon rows.
fn dashboard_header_text(colony: &ColonySnapshot, online: u32) -> String {
    let leader = colony
        .leader
        .as_ref()
        .map_or_else(|| "none".to_string(), |l| l.name.clone());
    format!(
        "online {online}\n\
         Colony: {name}  [{status:?}]\n\
         Leader: {leader}\n\
         Pop {pop}/{cap_house}  Village Lv {lvl}\n\
         Threat: {threat:?} ({pressure:.0})  warriors {warriors}",
        name = colony.name,
        status = colony.status,
        pop = colony.housing.population,
        cap_house = colony.housing.capacity,
        lvl = colony.housing.village_level,
        threat = colony.threat.band,
        pressure = colony.threat.pressure,
        warriors = colony.threat.warriors,
    )
}

/// The HUD footer (job counts + stock ledger) shown below the resource rows.
fn dashboard_footer_text(colony: &ColonySnapshot) -> String {
    let active_jobs = colony
        .jobs
        .iter()
        .filter(|j| matches!(j.status, cat_protocol::JobStatus::Active))
        .count();
    format!(
        "Active jobs: {active_jobs}   Total jobs: {jobs}\n{treasury}{ledger}",
        jobs = colony.jobs.len(),
        treasury = hud_treasury_line(&colony.items),
        ledger = colony
            .stock_ledger
            .as_ref()
            .map_or_else(String::new, |l| format!("\n\n{}", ledger_hud_text(l))),
    )
}

/// Always-visible HUD line for the colony's tradeable wealth.
fn hud_treasury_line(items: &[ItemStackSnapshot]) -> String {
    format!("Treasury: {}g", treasury_total(items))
}

/// Compact HUD summary of the Accountant's reported stock ledger. When a staffed
/// Accounting Tent keeps it exact the totals show plainly; otherwise they lag
/// reality and are marked stale (with a `~` prefix + a hint to build the tent).
fn ledger_hud_text(ledger: &StockLedgerSnapshot) -> String {
    let r = &ledger.reported;
    if ledger.accurate {
        format!(
            "Ledger (exact): F{:.0} W{:.0} M{:.0} R{:.0}",
            r.food, r.water, r.materials, r.refined
        )
    } else {
        format!(
            "Ledger (stale - build Accounting Tent)\n\
             known ~F{:.0} ~W{:.0} ~M{:.0} ~R{:.0}",
            r.food, r.water, r.materials, r.refined
        )
    }
}

/// Toggle the announcements panel via the `L` key or the Log HUD button (closes
/// the goods panel, which shares the centre slot).
fn toggle_announcements(
    keys: Res<ButtonInput<KeyCode>>,
    button: Query<&Interaction, (Changed<Interaction>, With<AnnouncementsButton>)>,
    mut ui: ResMut<AnnouncementsUi>,
    mut goods: ResMut<GoodsUi>,
    mut census: ResMut<CensusUi>,
    mut tree: ResMut<UpgradeTreeUi>,
) {
    let clicked = button.iter().any(|i| *i == Interaction::Pressed);
    if keys.just_pressed(KeyCode::KeyL) || clicked {
        ui.visible = !ui.visible;
        if ui.visible {
            goods.visible = false;
            census.visible = false;
            tree.visible = false;
        }
    }
}

/// Toggle the goods panel via the `G` key or the Goods HUD button (closes the
/// announcements + census panels, which share the centre slot).
#[allow(clippy::too_many_arguments)]
fn toggle_goods(
    keys: Res<ButtonInput<KeyCode>>,
    button: Query<&Interaction, (Changed<Interaction>, With<GoodsButton>)>,
    mut ui: ResMut<GoodsUi>,
    mut announce: ResMut<AnnouncementsUi>,
    mut census: ResMut<CensusUi>,
    mut tree: ResMut<UpgradeTreeUi>,
) {
    let clicked = button.iter().any(|i| *i == Interaction::Pressed);
    if keys.just_pressed(KeyCode::KeyG) || clicked {
        ui.visible = !ui.visible;
        if ui.visible {
            announce.visible = false;
            census.visible = false;
            tree.visible = false;
        }
    }
}

/// Toggle the census panel via the `C` key or the Census HUD button (closes the
/// goods + announcements panels, which share the centre slot).
#[allow(clippy::too_many_arguments)]
fn toggle_census(
    keys: Res<ButtonInput<KeyCode>>,
    button: Query<&Interaction, (Changed<Interaction>, With<CensusButton>)>,
    mut ui: ResMut<CensusUi>,
    mut goods: ResMut<GoodsUi>,
    mut announce: ResMut<AnnouncementsUi>,
    mut tree: ResMut<UpgradeTreeUi>,
) {
    let clicked = button.iter().any(|i| *i == Interaction::Pressed);
    if keys.just_pressed(KeyCode::KeyC) || clicked {
        ui.visible = !ui.visible;
        if ui.visible {
            goods.visible = false;
            announce.visible = false;
            tree.visible = false;
        }
    }
}

/// Show/hide the census panel and repaint its demographic lines from the live
/// snapshot (population, life-stage + specialization breakdowns, recent vitals).
fn update_census(
    latest: Res<LatestSnapshot>,
    ui: Res<CensusUi>,
    mut panel: Query<&mut Node, With<CensusPanel>>,
    mut lines: Query<(&CensusLine, &mut Text)>,
) {
    if let Ok(mut node) = panel.single_mut() {
        node.display = if ui.visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !ui.visible || (!latest.is_changed() && !ui.is_changed()) {
        return;
    }
    let report = latest
        .0
        .as_ref()
        .and_then(|w| w.colonies.first())
        .map_or_else(Vec::new, |c| {
            let census = colony_census(
                &c.cats,
                &c.events,
                c.leader.as_ref().map(|l| l.name.as_str()),
            );
            census_report_lines(&census)
        });
    for (line, mut text) in &mut lines {
        text.0 = report.get(line.0).cloned().unwrap_or_default();
    }
}

/// Toggle the upgrade-tree panel via `U` or its HUD button (closes the other
/// centre panels it shares the slot with).
fn toggle_upgrade_tree(
    keys: Res<ButtonInput<KeyCode>>,
    button: Query<&Interaction, (Changed<Interaction>, With<TreeButton>)>,
    mut ui: ResMut<UpgradeTreeUi>,
    mut goods: ResMut<GoodsUi>,
    mut announce: ResMut<AnnouncementsUi>,
    mut census: ResMut<CensusUi>,
) {
    let clicked = button.iter().any(|i| *i == Interaction::Pressed);
    if keys.just_pressed(KeyCode::KeyU) || clicked {
        ui.visible = !ui.visible;
        if ui.visible {
            goods.visible = false;
            announce.visible = false;
            census.visible = false;
        }
    }
}

/// Show/hide the upgrade-tree panel and repaint it from the live research state:
/// the header currencies, each node's coloured label, and each node's buy button
/// visibility (shown only for an available, affordable node).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_upgrade_tree(
    latest: Res<LatestSnapshot>,
    ui: Res<UpgradeTreeUi>,
    mut panel: Query<&mut Node, (With<TreePanel>, Without<TreeBuyButton>)>,
    mut currency: Query<&mut Text, (With<TreeCurrencyText>, Without<TreeNextText>)>,
    mut next: Query<&mut Text, (With<TreeNextText>, Without<TreeCurrencyText>)>,
    mut nodes: Query<
        (&TreeNodeText, &mut Text),
        (Without<TreeCurrencyText>, Without<TreeNextText>),
    >,
    mut buys: Query<(&TreeBuyButton, &mut Node), Without<TreePanel>>,
    mut node_colors: Query<(&TreeNodeText, &mut TextColor)>,
) {
    if let Ok(mut node) = panel.single_mut() {
        node.display = if ui.visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !ui.visible || (!latest.is_changed() && !ui.is_changed()) {
        return;
    }
    let Some(research) = latest
        .0
        .as_ref()
        .and_then(|w| w.colonies.first())
        .map(|c| &c.research)
    else {
        return;
    };
    if let Ok(mut t) = currency.single_mut() {
        t.0 = tree_currency_line(research);
    }
    if let Ok(mut t) = next.single_mut() {
        t.0 = tree_next_line(research);
    }
    let owned: HashSet<&str> = research.owned_node_ids.iter().map(String::as_str).collect();
    // Text label per node.
    for (marker, mut text) in &mut nodes {
        if let Some(node) = UPGRADE_NODES.iter().find(|n| n.id == marker.0) {
            text.0 = node_line(node, research, &owned).label;
        }
    }
    // Colour per node (separate query to avoid a conflicting Text borrow above).
    for (marker, mut color) in &mut node_colors {
        if let Some(node) = UPGRADE_NODES.iter().find(|n| n.id == marker.0) {
            color.0 = node_line(node, research, &owned).color;
        }
    }
    // Buy button shows only for an available + affordable node.
    for (buy, mut node) in &mut buys {
        let show = UPGRADE_NODES
            .iter()
            .find(|n| n.id == buy.0)
            .is_some_and(|n| node_line(n, research, &owned).show_buy);
        node.display = if show { Display::Flex } else { Display::None };
    }
}

/// God-purchase a node when its Buy button is clicked: dispatch a session-signed
/// `UnlockNode`. Blessings drop and the node flips to owned once the stream echoes.
#[allow(clippy::type_complexity)]
fn handle_tree_buy(
    session: Res<Session>,
    mut outgoing: ResMut<OutgoingActions>,
    mut buttons: Query<(&Interaction, &TreeBuyButton, &mut ImageNode), Changed<Interaction>>,
) {
    for (interaction, buy, mut image) in &mut buttons {
        match interaction {
            Interaction::Pressed => {
                image.color = BTN_PRESS;
                if session.ready {
                    outgoing.0.push(ClientAction::UnlockNode {
                        session_id: session.session_id.clone(),
                        nickname: "Desktop Cat".to_string(),
                        sig: session.sig.clone(),
                        node_id: buy.0.to_string(),
                    });
                }
            }
            Interaction::Hovered => image.color = BTN_HOVER,
            Interaction::None => image.color = BTN_IDLE,
        }
    }
}

/// Show/hide the goods panel and repaint its treasury total + item lines (most
/// valuable stack first), with a tidy empty state when there are no goods yet.
#[allow(clippy::type_complexity)]
fn update_goods(
    latest: Res<LatestSnapshot>,
    ui: Res<GoodsUi>,
    icons: Res<IconArt>,
    mut panel: Query<&mut Node, (With<GoodsPanel>, Without<GoodsLineIcon>)>,
    mut treasury: Query<&mut Text, (With<GoodsTreasury>, Without<GoodsLine>)>,
    mut lines: Query<(&GoodsLine, &mut Text), Without<GoodsTreasury>>,
    mut icon_nodes: Query<(&GoodsLineIcon, &mut Node, &mut ImageNode), Without<GoodsPanel>>,
) {
    if let Ok(mut node) = panel.single_mut() {
        node.display = if ui.visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !ui.visible || (!latest.is_changed() && !ui.is_changed()) {
        return;
    }
    let mut items = latest
        .0
        .as_ref()
        .and_then(|w| w.colonies.first())
        .map(|c| c.items.clone())
        .unwrap_or_default();
    // Most valuable stack first.
    items.sort_by_key(|s| std::cmp::Reverse(s.count * s.value));

    if let Ok(mut text) = treasury.single_mut() {
        text.0 = format!("Treasury: {}g", treasury_total(&items));
    }
    for (line, mut text) in &mut lines {
        text.0 = match (line.0, items.get(line.0)) {
            (_, Some(stack)) => item_label(stack),
            // The empty-state line sits in the first slot when there are none.
            (0, None) if items.is_empty() => "No crafted goods yet".to_string(),
            _ => String::new(),
        };
    }
    // Per-kind glyph tinted by material, hidden on empty slots.
    for (icon, mut node, mut image) in &mut icon_nodes {
        if let Some(stack) = items.get(icon.0) {
            image.image = icons.item_glyph(&stack.kind);
            image.color = material_tint(&stack.material);
            node.display = Display::Flex;
        } else {
            node.display = Display::None;
        }
    }
}

/// Show the trade menu while a trader is at the gate (Trading) and repaint coin +
/// the sell/buy offer rows live from the snapshot.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_trade_menu(
    latest: Res<LatestSnapshot>,
    mut trade_ui: ResMut<TradeUi>,
    mut panel: Query<&mut Node, (With<TradeMenuPanel>, Without<SellRow>, Without<BuyRow>)>,
    mut coin: Query<
        &mut Text,
        (
            With<TradeCoinText>,
            Without<SellRowText>,
            Without<BuyRowText>,
        ),
    >,
    mut sell_rows: Query<(&SellRow, &mut Node), (Without<TradeMenuPanel>, Without<BuyRow>)>,
    mut sell_texts: Query<(&SellRowText, &mut Text), (Without<TradeCoinText>, Without<BuyRowText>)>,
    mut buy_rows: Query<(&BuyRow, &mut Node), (Without<TradeMenuPanel>, Without<SellRow>)>,
    mut buy_texts: Query<(&BuyRowText, &mut Text), (Without<TradeCoinText>, Without<SellRowText>)>,
) {
    let colony = latest.0.as_ref().and_then(|w| w.colonies.first());
    let trader = colony.and_then(|c| c.trader.as_ref());
    let trading = trader.is_some_and(|t| matches!(t.state, TraderVisitState::Trading));
    // Once the trader stops trading, clear any manual dismissal so the next visit
    // auto-opens.
    if !trading && trade_ui.closed {
        trade_ui.closed = false;
    }
    let open = trading && !trade_ui.closed;
    if let Ok(mut node) = panel.single_mut() {
        node.display = if open { Display::Flex } else { Display::None };
    }
    let (Some(colony), Some(trader)) = (colony, trader) else {
        return;
    };
    if !open {
        return;
    }
    if let Ok(mut text) = coin.single_mut() {
        text.0 = coin_line(colony.coin);
    }
    for (row, mut node) in &mut sell_rows {
        node.display = if row.0 < trader.buy_offers.len() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (label, mut text) in &mut sell_texts {
        text.0 = trader
            .buy_offers
            .get(label.0)
            .map_or_else(String::new, sell_offer_label);
    }
    for (row, mut node) in &mut buy_rows {
        node.display = if row.0 < trader.sell_offers.len() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (label, mut text) in &mut buy_texts {
        text.0 = trader
            .sell_offers
            .get(label.0)
            .map_or_else(String::new, |o| buy_offer_label(o, colony.coin));
    }
}

/// Dispatch Sell/Buy actions from the trade-menu buttons (resolved against the
/// live offers), and close the menu on the Close button. The sim denies any
/// trade that isn't currently valid, so these are best-effort.
#[allow(clippy::type_complexity)]
fn handle_trade_buttons(
    latest: Res<LatestSnapshot>,
    session: Res<Session>,
    mut outgoing: ResMut<OutgoingActions>,
    mut trade_ui: ResMut<TradeUi>,
    sell_buttons: Query<(&Interaction, &SellButton), Changed<Interaction>>,
    buy_buttons: Query<(&Interaction, &BuyButton), Changed<Interaction>>,
    close: Query<&Interaction, (Changed<Interaction>, With<TradeCloseButton>)>,
) {
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    let Some(trader) = colony.trader.as_ref() else {
        return;
    };
    for (interaction, button) in &sell_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(offer) = trader.buy_offers.get(button.row) {
            let count = if button.all { offer.available } else { 1 };
            if count > 0 {
                outgoing.0.push(ClientAction::SellGoods {
                    session_id: session.session_id.clone(),
                    nickname: "Desktop Cat".to_string(),
                    sig: session.sig.clone(),
                    kind: offer.kind.clone(),
                    material: offer.material.clone(),
                    quality: offer.quality,
                    count,
                });
            }
        }
    }
    for (interaction, button) in &buy_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if let Some(offer) = trader.sell_offers.get(button.0)
            && can_afford(colony.coin, offer.unit_price)
        {
            outgoing.0.push(ClientAction::BuyResource {
                session_id: session.session_id.clone(),
                nickname: "Desktop Cat".to_string(),
                sig: session.sig.clone(),
                resource: offer.resource,
                amount: 1.0,
            });
        }
    }
    if close.iter().any(|i| *i == Interaction::Pressed) {
        trade_ui.closed = true;
    }
}

/// Show/hide the announcements panel and repaint its colour-coded lines
/// newest-first, plus the HUD "latest announcement" ticker.
#[allow(clippy::type_complexity)]
fn update_announcements(
    latest: Res<LatestSnapshot>,
    ui: Res<AnnouncementsUi>,
    mut panel: Query<&mut Node, With<AnnouncementsPanel>>,
    mut lines: Query<(&AnnouncementLine, &mut Text, &mut TextColor), Without<AnnouncementTicker>>,
    mut ticker: Query<
        (&mut Text, &mut TextColor),
        (With<AnnouncementTicker>, Without<AnnouncementLine>),
    >,
) {
    if let Ok(mut node) = panel.single_mut() {
        node.display = if ui.visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !latest.is_changed() && !ui.is_changed() {
        return;
    }
    let now = latest.0.as_ref().map_or(0, |w| w.now);
    let mut events = latest
        .0
        .as_ref()
        .and_then(|w| w.colonies.first())
        .map(|c| c.events.clone())
        .unwrap_or_default();
    events.sort_by_key(|e| e.timestamp);
    // Newest first.
    let newest: Vec<_> = events.iter().rev().collect();

    for (line, mut text, mut color) in &mut lines {
        if let Some(e) = newest.get(line.0) {
            text.0 = announcement_line(now, &e.kind, &e.message, e.timestamp);
            color.0 = event_color(event_kind_of(&e.kind));
        } else {
            text.0 = String::new();
        }
    }
    if let Ok((mut text, mut color)) = ticker.single_mut() {
        if let Some(e) = newest.first() {
            let kind = event_kind_of(&e.kind);
            text.0 = format!("{} {}", event_glyph(kind), e.message);
            color.0 = event_color(kind);
        } else {
            text.0 = String::new();
        }
    }
}

/// Toggle the corner minimap with the `M` key.
fn toggle_minimap(keys: Res<ButtonInput<KeyCode>>, mut ui: ResMut<MinimapUi>) {
    if keys.just_pressed(KeyCode::KeyM) {
        ui.visible = !ui.visible;
    }
}

/// Redraw the minimap texture from the snapshot each tick: revealed terrain
/// coloured by biome, with village buildings, cats and any raiders marked.
fn update_minimap(
    latest: Res<LatestSnapshot>,
    ui: Res<MinimapUi>,
    mut minimap: ResMut<Minimap>,
    mut images: ResMut<Assets<Image>>,
    mut panel: Query<&mut Node, With<MinimapPanel>>,
) {
    if let Ok(mut node) = panel.single_mut() {
        node.display = if ui.visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !ui.visible || (!latest.is_changed() && !ui.is_changed()) {
        return;
    }
    let Some((seed, colony)) = latest
        .0
        .as_ref()
        .and_then(|w| w.colonies.first().map(|c| (w.world_seed, c)))
    else {
        return;
    };

    let view = minimap_view(&colony.revealed_tiles);
    minimap.view = view;
    let biomes = revealed_biomes(seed, &colony.revealed_tiles);

    let mut buf = vec![0u8; (MINIMAP_PX * MINIMAP_PX * 4) as usize];
    for px in buf.chunks_exact_mut(4) {
        px.copy_from_slice(&MINIMAP_FOG);
    }
    // Revealed terrain (biome colour; grey where the chunk cap skipped sampling).
    for t in &colony.revealed_tiles {
        if let Some((px, py)) = world_to_minimap(view, t.x, t.y) {
            let color = biomes
                .get(&(t.x, t.y))
                .map_or([72, 72, 78, 255], |b| biome_rgba(*b));
            put_pixel(&mut buf, px, py, color);
        }
    }
    // Buildings: shrine gold, others pale (2x2 so they read over terrain).
    for b in &colony.buildings {
        if building_texture(b.building_type).is_none() {
            continue;
        }
        if let Some((px, py)) = world_to_minimap(view, b.world_position.x, b.world_position.y) {
            let color = if b.building_type == BuildingType::Shrine {
                [236, 206, 92, 255]
            } else {
                [222, 222, 228, 255]
            };
            put_block(&mut buf, px, py, color);
        }
    }
    // Cats: leader gold, warriors orange, the rest light blue.
    let leader_id = colony.leader.as_ref().map(|l| l.id.as_str());
    for c in colony.cats.iter().filter(|c| c.death_time.is_none()) {
        if let Some((px, py)) = world_to_minimap(view, c.position.x, c.position.y) {
            // Boosted cats get a bright 2x2 gold block so priority picks pop out
            // of the dot field even on the tiny minimap.
            if c.boosted {
                put_block(&mut buf, px, py, [255, 236, 120, 255]);
                continue;
            }
            let color = if Some(c.id.as_str()) == leader_id {
                [236, 206, 92, 255]
            } else if c.specialization == Some(Specialization::Warrior) {
                [224, 128, 64, 255]
            } else {
                [110, 190, 236, 255]
            };
            put_pixel(&mut buf, px, py, color);
        }
    }
    // Active raid warband, if any.
    for r in &colony.raiders {
        if let Some((px, py)) = world_to_minimap(view, r.position.x, r.position.y) {
            put_block(&mut buf, px, py, [230, 60, 50, 255]);
        }
    }
    // Visiting trader — friendly gold mark.
    if let Some(trader) = &colony.trader
        && let Some((px, py)) = world_to_minimap(view, trader.position.x, trader.position.y)
    {
        put_block(&mut buf, px, py, [240, 205, 90, 255]);
    }

    if let Some(mut image) = images.get_mut(&minimap.image) {
        image.data = Some(buf);
    }
}

/// Position the camera-viewport outline over the minimap from the camera's
/// current world view, so it stays in sync while panning/zooming.
fn update_minimap_viewport(
    windows: Query<&Window>,
    camera: Query<(&Projection, &Transform), With<Camera2d>>,
    ui: Res<MinimapUi>,
    minimap: Res<Minimap>,
    mut rect: Query<&mut Node, With<MinimapViewportRect>>,
) {
    let Ok(mut node) = rect.single_mut() else {
        return;
    };
    if !ui.visible {
        node.display = Display::None;
        return;
    }
    let (Ok(window), Ok((proj, cam))) = (windows.single(), camera.single()) else {
        return;
    };
    let Projection::Orthographic(p) = proj else {
        return;
    };
    // Visible world half-extents = half the window size scaled by the zoom.
    let half = Vec2::new(window.width(), window.height()) * p.scale * 0.5;
    let c = cam.translation.truncate();
    // World corners -> tiles (y flips: world -y = +tile y).
    let (tx0, ty0) = world_to_tile(Vec2::new(c.x - half.x, c.y + half.y));
    let (tx1, ty1) = world_to_tile(Vec2::new(c.x + half.x, c.y - half.y));
    let (x0, y0, x1, y1) = viewport_rect(
        minimap.view,
        tx0.min(tx1),
        ty0.min(ty1),
        tx0.max(tx1),
        ty0.max(ty1),
    );
    let pct = |v: i32| Val::Percent(v as f32 / MINIMAP_PX as f32 * 100.0);
    node.display = Display::Flex;
    node.left = pct(x0);
    node.top = pct(y0);
    node.width = pct(x1 - x0);
    node.height = pct(y1 - y0);
}

/// Left-click on the minimap recenters the main camera on that world point.
fn minimap_click_to_pan(
    buttons: Res<ButtonInput<MouseButton>>,
    ui: Res<MinimapUi>,
    minimap: Res<Minimap>,
    image: Query<&RelativeCursorPosition, With<MinimapImageNode>>,
    mut camera: Query<&mut Transform, With<Camera2d>>,
) {
    if !ui.visible || !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(rel) = image.single() else {
        return;
    };
    let Some(n) = rel
        .normalized
        .filter(|n| (0.0..=1.0).contains(&n.x) && (0.0..=1.0).contains(&n.y))
    else {
        return;
    };
    let px = (n.x * MINIMAP_PX as f32) as i32;
    let py = (n.y * MINIMAP_PX as f32) as i32;
    let (tx, ty) = minimap_to_world(minimap.view, px, py);
    let world = grid_to_world(tx, ty);
    if let Ok(mut cam) = camera.single_mut() {
        cam.translation.x = world.x;
        cam.translation.y = world.y;
    }
}

fn update_event_log(latest: Res<LatestSnapshot>, mut log: Query<&mut Text, With<EventLogText>>) {
    if !latest.is_changed() {
        return;
    }
    let Ok(mut text) = log.single_mut() else {
        return;
    };
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        return;
    };
    let mut events = colony.events.clone();
    events.sort_by_key(|e| e.timestamp);
    let lines: Vec<String> = events
        .iter()
        .rev()
        .take(4)
        .map(|e| format!("- {}", e.message))
        .collect();
    text.0 = if lines.is_empty() {
        "no recent events".to_string()
    } else {
        lines.join("\n")
    };
}

/// React to toolbar clicks: tint the button and enqueue its action.
fn handle_buttons(
    session: Res<Session>,
    mut outgoing: ResMut<OutgoingActions>,
    mut buttons: ButtonQuery,
) {
    for (interaction, button, mut image) in &mut buttons {
        match interaction {
            Interaction::Pressed => {
                image.color = BTN_PRESS;
                if let Some(action) = build_action(button.0, &session) {
                    outgoing.0.push(action);
                }
            }
            Interaction::Hovered => image.color = BTN_HOVER,
            Interaction::None => image.color = BTN_IDLE,
        }
    }
}

fn build_action(action: ButtonAction, session: &Session) -> Option<ClientAction> {
    let kind = match action {
        ButtonAction::SupplyFood => JobKind::SupplyFood,
        ButtonAction::SupplyWater => JobKind::SupplyWater,
        ButtonAction::PlanHunt => JobKind::LeaderPlanHunt,
        ButtonAction::FoundVillage => {
            return Some(ClientAction::FoundVillage {
                name: "Forest Hollow".to_string(),
                session_id: if session.session_id.is_empty() {
                    "desktop".to_string()
                } else {
                    session.session_id.clone()
                },
            });
        }
    };
    if !session.ready {
        warn!("session not ready; dropping action");
        return None;
    }
    Some(ClientAction::RequestJob {
        session_id: session.session_id.clone(),
        nickname: "Desktop Cat".to_string(),
        sig: session.sig.clone(),
        kind,
    })
}

/// Send any queued actions over the socket.
fn flush_outgoing(conn: Option<NonSendMut<WsConn>>, mut outgoing: ResMut<OutgoingActions>) {
    let Some(mut conn) = conn else {
        return;
    };
    if outgoing.0.is_empty() {
        return;
    }
    for action in outgoing.0.drain(..) {
        if let Ok(json) = serde_json::to_string(&action) {
            conn.sender.send(WsMessage::Text(json));
        }
    }
}

// ---- pure selection / inspector / zone helpers (unit-tested) ----

/// Flat top-down inverse of [`grid_to_world`]: world space → tile coordinate.
fn world_to_tile(world: Vec2) -> (i32, i32) {
    (
        (world.x / TILE).round() as i32,
        (-world.y / TILE).round() as i32,
    )
}

/// Inclusive tile rectangle `(min, max)` for a drag from `start` to `end`,
/// clamped so neither side exceeds `max` tiles (the sim's zone cap).
fn drag_tile_rect(start: (i32, i32), end: (i32, i32), max: i32) -> ((i32, i32), (i32, i32)) {
    let span = (max - 1).max(0);
    let cx = end.0.clamp(start.0 - span, start.0 + span);
    let cy = end.1.clamp(start.1 - span, start.1 + span);
    (
        (start.0.min(cx), start.1.min(cy)),
        (start.0.max(cx), start.1.max(cy)),
    )
}

fn paint_preview_color(kind: PaintKind) -> Color {
    match kind {
        PaintKind::Avoid => Color::srgba(0.95, 0.30, 0.30, 0.45),
        PaintKind::Gather => Color::srgba(0.35, 0.90, 0.40, 0.45),
        PaintKind::Stockpile => Color::srgba(0.85, 0.60, 0.25, 0.45),
    }
}

/// Read-only inspector body for a building: type, level, complete/under-
/// construction, its assigned cats, and — for producers — live staffing +
/// production progress from the snapshot.
fn building_inspector_text(building: &BuildingSnapshot, colony: &ColonySnapshot) -> String {
    let mut out = format!(
        "{name}  Lv {lvl}",
        name = building_label(building.building_type),
        lvl = building.level,
    );
    if building.construction_progress < 100.0 {
        out.push_str(&format!(
            "\nunder construction {:.0}%\nat {},{}",
            building.construction_progress, building.world_position.x, building.world_position.y,
        ));
        return out;
    }
    out.push_str(&format!(
        "\noperational\nat {},{}",
        building.world_position.x, building.world_position.y
    ));
    // Producer buildings (staff_cap > 0) show live staffing + a progress bar.
    if building.staff_cap > 0 {
        out.push_str(&format!(
            "\nstaffed: {}/{}",
            building.staff_count, building.staff_cap
        ));
        let workers: Vec<&str> = colony
            .cats
            .iter()
            .filter(|c| c.assigned_building_id.as_deref() == Some(building.id.as_str()))
            .map(|c| c.name.as_str())
            .collect();
        if !workers.is_empty() {
            out.push_str(&format!(" - {}", workers.join(", ")));
        }
        if let Some(output) = &building.production_output {
            out.push_str(&format!(
                "\n{}",
                production_line(output, building.production_progress)
            ));
        }
    }
    out
}

/// The name of the cat holding an officer role, or `None` when vacant / the
/// appointed cat is no longer in the snapshot.
fn officer_holder_name(colony: &ColonySnapshot, role: OfficerRole) -> Option<&str> {
    let id = colony.officers.get(&role)?;
    colony
        .cats
        .iter()
        .find(|c| &c.id == id)
        .map(|c| c.name.as_str())
}

/// Sum of the storable goods held in a stockpile (blessings excluded).
fn resource_total(c: &ResourceAmounts) -> f64 {
    c.food + c.water + c.herbs + c.materials + c.refined + c.weapons + c.armor
}

/// The single largest storable resource in a pile, or `None` when it's empty.
fn dominant_resource(c: &ResourceAmounts) -> Option<ResourceKind> {
    [
        (ResourceKind::Food, c.food),
        (ResourceKind::Water, c.water),
        (ResourceKind::Herbs, c.herbs),
        (ResourceKind::Materials, c.materials),
        (ResourceKind::Refined, c.refined),
        (ResourceKind::Weapons, c.weapons),
        (ResourceKind::Armor, c.armor),
    ]
    .into_iter()
    .filter(|(_, v)| *v > 0.0)
    .max_by(|a, b| a.1.total_cmp(&b.1))
    .map(|(kind, _)| kind)
}

/// The pile prop sprite for a dominant resource.
fn pile_prop(kind: ResourceKind) -> PropTexture {
    match kind {
        ResourceKind::Food => PropTexture::Sack,
        ResourceKind::Water => PropTexture::Barrel,
        ResourceKind::Herbs => PropTexture::Haystack,
        ResourceKind::Materials => PropTexture::StonePile,
        ResourceKind::Refined => PropTexture::GoldPile,
        ResourceKind::Weapons | ResourceKind::Armor | ResourceKind::Blessings => PropTexture::Crate,
    }
}

/// Pile sprite size scaled by total contents: ~0.5 tile when nearly empty up to
/// ~1.4 tiles when full.
fn pile_scale(total: f64) -> f32 {
    let t = (total / 200.0).clamp(0.0, 1.0) as f32;
    TILE * (0.5 + t * 0.9)
}

fn resource_kind_name(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Food => "food",
        ResourceKind::Water => "water",
        ResourceKind::Herbs => "herbs",
        ResourceKind::Materials => "materials",
        ResourceKind::Refined => "refined",
        ResourceKind::Weapons => "weapons",
        ResourceKind::Armor => "armor",
        ResourceKind::Blessings => "blessings",
    }
}

/// A stockpile's accept-type label: "General" for all storable kinds, "X only"
/// for a single kind, otherwise the kinds joined.
fn accepts_label(accepts: &[ResourceKind]) -> String {
    if is_general_accepts(accepts) {
        return "General".to_string();
    }
    match accepts {
        [] => "Accepts nothing".to_string(),
        [one] => format!("{} only", resource_kind_name(*one)),
        many => many
            .iter()
            .map(|k| resource_kind_name(*k))
            .collect::<Vec<_>>()
            .join("/"),
    }
}

/// True when an accept-set covers every storable kind (order-independent).
fn is_general_accepts(accepts: &[ResourceKind]) -> bool {
    STORABLE_KINDS.iter().all(|k| accepts.contains(k))
}

/// Translucent overlay colour: amber for General, else tinted by the single
/// accepted resource (falls back to amber for multi-kind subsets).
fn accept_overlay_color(accepts: &[ResourceKind]) -> Color {
    if is_general_accepts(accepts) {
        return STOCKPILE_OVERLAY;
    }
    match accepts {
        [ResourceKind::Food] => Color::srgba(0.95, 0.55, 0.25, 0.32),
        [ResourceKind::Water] => Color::srgba(0.35, 0.65, 0.95, 0.32),
        [ResourceKind::Herbs] => Color::srgba(0.45, 0.80, 0.40, 0.32),
        [ResourceKind::Materials] => Color::srgba(0.70, 0.55, 0.35, 0.32),
        [ResourceKind::Refined] => Color::srgba(0.95, 0.82, 0.35, 0.32),
        [ResourceKind::Weapons | ResourceKind::Armor] => Color::srgba(0.70, 0.72, 0.78, 0.32),
        _ => STOCKPILE_OVERLAY,
    }
}

/// Perimeter wall edges of a claimed-tile set: for each claimed tile, every
/// orthogonal side whose neighbour is unclaimed. `exclude` drops the gate edge.
fn wall_edges(
    claimed: &HashSet<(i32, i32)>,
    exclude: Option<((i32, i32), WallSide)>,
) -> Vec<((i32, i32), WallSide)> {
    let mut edges = Vec::new();
    for &(x, y) in claimed {
        for (side, neighbour) in [
            (WallSide::N, (x, y - 1)),
            (WallSide::S, (x, y + 1)),
            (WallSide::E, (x + 1, y)),
            (WallSide::W, (x - 1, y)),
        ] {
            if !claimed.contains(&neighbour) && exclude != Some(((x, y), side)) {
                edges.push(((x, y), side));
            }
        }
    }
    edges
}

/// World position of a tile's edge midpoint, and the sprite rotation (radians) —
/// horizontal walls (N/S) unrotated, vertical walls (E/W) turned 90°.
fn wall_edge_transform(tile: (i32, i32), side: WallSide) -> (Vec2, f32) {
    let c = grid_to_world(tile.0, tile.1);
    let half = TILE / 2.0;
    match side {
        // North is smaller y, which is *larger* world y under the flat projection.
        WallSide::N => (c + Vec2::new(0.0, half), 0.0),
        WallSide::S => (c + Vec2::new(0.0, -half), 0.0),
        WallSide::E => (c + Vec2::new(half, 0.0), std::f32::consts::FRAC_PI_2),
        WallSide::W => (c + Vec2::new(-half, 0.0), std::f32::consts::FRAC_PI_2),
    }
}

fn gate_side_to_wall(side: GateSide) -> WallSide {
    match side {
        GateSide::N => WallSide::N,
        GateSide::E => WallSide::E,
        GateSide::S => WallSide::S,
        GateSide::W => WallSide::W,
    }
}

/// Whether a tile falls inside a stockpile's (unordered) rectangle.
fn point_in_stockpile(tile: (i32, i32), pile: &StockpileSnapshot) -> bool {
    let (x0, x1) = (pile.x1.min(pile.x2), pile.x1.max(pile.x2));
    let (y0, y1) = (pile.y1.min(pile.y2), pile.y1.max(pile.y2));
    (x0..=x1).contains(&tile.0) && (y0..=y1).contains(&tile.1)
}

/// Nearest id to `click` within `radius`, or `None` (used for cats + buildings).
fn nearest_id(click: Vec2, items: &[(String, Vec2)], radius: f32) -> Option<String> {
    let r2 = radius * radius;
    let mut best: Option<(&str, f32)> = None;
    for (id, pos) in items {
        let d2 = pos.distance_squared(click);
        if d2 <= r2 && best.is_none_or(|(_, bd)| d2 < bd) {
            best = Some((id, d2));
        }
    }
    best.map(|(id, _)| id.to_string())
}

/// Selection toggle: re-clicking the current cat (or empty ground) deselects;
/// clicking a different cat selects it.
fn toggle_selection(current: Option<&str>, picked: Option<String>) -> Option<String> {
    match (current, picked) {
        (Some(cur), Some(p)) if cur == p => None,
        (_, picked) => picked,
    }
}

/// Life stage from accelerated age (game-hours): kitten 0–6, young 6–24,
/// adult 24–48, elder 48+.
fn life_stage(age_hours: f64) -> &'static str {
    match age_hours {
        a if a < 6.0 => "kitten",
        a if a < 24.0 => "young",
        a if a < 48.0 => "adult",
        _ => "elder",
    }
}

fn activity_name(activity: CatActivity) -> &'static str {
    match activity {
        CatActivity::Idle => "idle",
        CatActivity::Traveling => "traveling",
        CatActivity::Working => "working",
        CatActivity::Returning => "returning",
    }
}

fn specialization_name(spec: Option<Specialization>) -> &'static str {
    match spec {
        Some(Specialization::Hunter) => "hunter",
        Some(Specialization::Architect) => "architect",
        Some(Specialization::Ritualist) => "ritualist",
        Some(Specialization::Warrior) => "warrior",
        None => "none",
    }
}

/// Multi-line read-only inspector body for a cat.
/// The value of one of a cat's four needs (0..100).
fn cat_need_value(needs: &CatNeeds, kind: NeedKind) -> f64 {
    match kind {
        NeedKind::Hunger => needs.hunger,
        NeedKind::Thirst => needs.thirst,
        NeedKind::Rest => needs.rest,
        NeedKind::Health => needs.health,
    }
}

/// Bar colour for a need level: green when comfortable, amber when low, red when
/// critical.
fn need_bar_color(value: f64) -> Color {
    if value >= 60.0 {
        Color::srgb(0.42, 0.72, 0.36)
    } else if value >= 30.0 {
        Color::srgb(0.88, 0.72, 0.30)
    } else {
        Color::srgb(0.84, 0.34, 0.28)
    }
}

/// The textual part of the cat inspector (identity, activity, skills) — the four
/// needs render as bars alongside it.
fn inspector_text(cat: &CatSnapshot) -> String {
    let dest = cat
        .destination
        .map_or_else(|| "none".to_string(), |d| format!("{},{}", d.x, d.y));
    let carrying = cat.carrying.as_ref().map_or_else(
        || "none".to_string(),
        |c| format!("{:?} x{:.0}", c.kind, c.amount),
    );
    // Lineage is only shown once breeding populates it (empty for founders/today).
    let parents = if cat.parents.is_empty() {
        String::new()
    } else {
        format!("\nparents: {}", cat.parents.join(", "))
    };
    // Pregnancy indicator (life-sim: gestating cats are due a litter soon).
    let expecting = if cat.pregnant {
        "\nexpecting a litter"
    } else {
        ""
    };
    format!(
        "{name}\n\
         {spec} - {stage} ({age:.0}h)\n\
         at {x},{y} - {activity}\n\
         dest {dest}\n\
         carrying {carrying}{parents}{expecting}\n\
         \n\
         skills: {skills}\n\
         leadership {lead:.0}",
        name = cat.name,
        spec = specialization_name(cat.specialization),
        stage = life_stage(cat.age_hours),
        age = cat.age_hours,
        x = cat.position.x,
        y = cat.position.y,
        activity = activity_name(cat.activity),
        skills = cat_skills_line(&cat.role_xp),
        lead = cat.stats.leadership,
    )
}

/// One-line summary of a cat's role experience (skills).
fn cat_skills_line(xp: &RoleXp) -> String {
    format!(
        "hunt {h:.0} build {b:.0} ritual {r:.0} war {w:.0}",
        h = xp.hunter,
        b = xp.architect,
        r = xp.ritualist,
        w = xp.warrior,
    )
}

// ---- pure building sprite / label helpers (unit-tested) ----

/// The sprite a building renders as, or `None` for `Walls` (drawn as infra, not
/// a point marker). Unmapped/utility variants fall back to the generic den.
fn building_texture(building: BuildingType) -> Option<BuildingTexture> {
    Some(match building {
        BuildingType::Shrine => BuildingTexture::Shrine,
        BuildingType::Workshop => BuildingTexture::Workshop,
        BuildingType::Smithy => BuildingTexture::Smithy,
        BuildingType::ResearchHut => BuildingTexture::ResearchHut,
        BuildingType::School => BuildingTexture::School,
        BuildingType::Barracks => BuildingTexture::Barracks,
        BuildingType::FoodStorage | BuildingType::MouseFarm => BuildingTexture::Storehouse,
        BuildingType::Field => BuildingTexture::Market,
        BuildingType::WaterBowl => BuildingTexture::Well,
        BuildingType::Den
        | BuildingType::Beds
        | BuildingType::Nursery
        | BuildingType::HerbGarden
        | BuildingType::ElderCorner => BuildingTexture::Den,
        // P16 craft-station workshops render as their dedicated sprites so the
        // function reads at a glance.
        BuildingType::WoodCutter => BuildingTexture::WoodCutter,
        BuildingType::StonePrep => BuildingTexture::StonePrep,
        BuildingType::Woodworking => BuildingTexture::Woodworking,
        // P18 clothing chain craft stations.
        BuildingType::Clothier => BuildingTexture::Clothier,
        BuildingType::Tannery => BuildingTexture::Tannery,
        // Ore/metal chain: the smelter is a forge/furnace — reuse the smithy sprite.
        BuildingType::Smelter => BuildingTexture::Smithy,
        BuildingType::Walls => return None,
    })
}

/// Interior floor material for a building's cutaway top-down render.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FloorKind {
    Wood,
    Stone,
}

/// The workstation prop placed on a building's floor, distinguishing it by
/// function rather than by any roof. (Per-building props — loom/anvil/forge —
/// land in later slices; this set covers the categories.)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InteriorProp {
    Workbench,
    Bed,
    Crate,
    Furnace,
    Altar,
    None,
}

/// The cutaway top-down interior for a building: its floor material + the
/// workstation prop that sits on it. `None` for `Walls` (drawn as the palisade).
fn building_interior(building: BuildingType) -> Option<(FloorKind, InteriorProp)> {
    use BuildingType as B;
    Some(match building {
        // Craft workshops: a wooden shop floor + a workbench.
        B::Workshop | B::Woodworking | B::WoodCutter | B::StonePrep | B::Clothier | B::Tannery => {
            (FloorKind::Wood, InteriorProp::Workbench)
        }
        // Metalworking: a stone forge floor + a furnace.
        B::Smithy | B::Smelter => (FloorKind::Stone, InteriorProp::Furnace),
        // Dwellings: a wooden floor + beds.
        B::Den | B::Beds | B::Nursery | B::ElderCorner | B::HerbGarden => {
            (FloorKind::Wood, InteriorProp::Bed)
        }
        // Storage: a wooden floor + crates.
        B::FoodStorage | B::WaterBowl | B::MouseFarm => (FloorKind::Wood, InteriorProp::Crate),
        // Shrine: a stone floor + a candelabra altar.
        B::Shrine => (FloorKind::Stone, InteriorProp::Altar),
        // Civic/support: stone floors, prop TBD.
        B::ResearchHut | B::School | B::Barracks | B::Field => {
            (FloorKind::Stone, InteriorProp::None)
        }
        B::Walls => return None,
    })
}

/// Native width/height aspect of a building sprite (48x48 square = 1.0; market
/// 48x32 wide = 1.5; well 16x32 tall = 0.5). Retired by the cutaway-interior
/// render; kept for the mapping unit test while the interior look is reviewed.
#[allow(dead_code)]
fn building_aspect(texture: BuildingTexture) -> f32 {
    match texture {
        BuildingTexture::Market => 48.0 / 32.0,
        BuildingTexture::Well => 16.0 / 32.0,
        _ => 1.0,
    }
}

fn building_label(building: BuildingType) -> &'static str {
    match building {
        BuildingType::Shrine => "shrine",
        BuildingType::Den => "den",
        BuildingType::FoodStorage => "food",
        BuildingType::WaterBowl => "water",
        BuildingType::Beds => "beds",
        BuildingType::HerbGarden => "herbs",
        BuildingType::Nursery => "nursery",
        BuildingType::ElderCorner => "elders",
        BuildingType::Walls => "walls",
        BuildingType::MouseFarm => "mousefarm",
        BuildingType::Workshop => "workshop",
        BuildingType::Field => "field",
        BuildingType::ResearchHut => "research",
        BuildingType::School => "school",
        BuildingType::Smithy => "smithy",
        BuildingType::Barracks => "barracks",
        BuildingType::WoodCutter => "woodcutter",
        BuildingType::StonePrep => "stoneprep",
        BuildingType::Woodworking => "woodworking",
        BuildingType::Clothier => "clothier",
        BuildingType::Tannery => "tannery",
        BuildingType::Smelter => "smelter",
    }
}

/// Direction group index (0..8) for a heading in tile space (+y = south),
/// ordered to match the sheet: S, SW, W, NW, N, NE, E, SE. Picks the nearest of
/// the 8 compass directions; a zero heading defaults to S.
fn direction_group_f(dx: f32, dy: f32) -> usize {
    const DIRS: [(f32, f32); 8] = [
        (0.0, 1.0),   // 0 S
        (-1.0, 1.0),  // 1 SW
        (-1.0, 0.0),  // 2 W
        (-1.0, -1.0), // 3 NW
        (0.0, -1.0),  // 4 N
        (1.0, -1.0),  // 5 NE
        (1.0, 0.0),   // 6 E
        (1.0, 1.0),   // 7 SE
    ];
    if dx == 0.0 && dy == 0.0 {
        return 0;
    }
    let mut best = 0;
    let mut best_dot = f32::MIN;
    for (i, (ux, uy)) in DIRS.iter().enumerate() {
        // Normalise so diagonals compete fairly with the axis directions.
        let len = (ux * ux + uy * uy).sqrt();
        let dot = (dx * ux + dy * uy) / len;
        if dot > best_dot {
            best_dot = dot;
            best = i;
        }
    }
    best
}

/// Facing group from a *world-space* travel delta (north is up = +world y, so
/// tile-south = -world y). `None` when the delta is too small to have a facing.
fn facing_from_delta(delta: Vec2) -> Option<usize> {
    if delta.length_squared() < FACING_EPS * FACING_EPS {
        return None;
    }
    Some(direction_group_f(delta.x, -delta.y))
}

/// One constant-speed walk step of `current` toward `target`: advance by `step`
/// world units along the straight line, arriving exactly if within a step. Snaps
/// to the target only when it's `max_lag` away (absurdly far / off-screen) so a
/// body never teleports for an ordinary multi-tile catch-up.
fn walk_step(current: Vec2, target: Vec2, step: f32, max_lag: f32) -> Vec2 {
    let to = target - current;
    let dist = to.length();
    if dist > max_lag || dist <= step || dist == 0.0 {
        return target;
    }
    current + to / dist * step
}

/// Whether a body is still translating (beyond `eps` world units from target).
fn is_moving(current: Vec2, target: Vec2, eps: f32) -> bool {
    current.distance_squared(target) > eps * eps
}

/// Atlas cell index for a direction group + walk frame (4 frames per group).
fn atlas_index(group: usize, frame: usize) -> usize {
    group * 4 + frame
}

fn carrying_color(kind: CarryingKind) -> Color {
    match kind {
        CarryingKind::Food => Color::srgb(0.95, 0.55, 0.25),
        CarryingKind::Water => Color::srgb(0.35, 0.65, 0.95),
        CarryingKind::Materials => Color::srgb(0.70, 0.55, 0.35),
        CarryingKind::Blessings => Color::srgb(0.95, 0.85, 0.40),
    }
}

fn zone_color(kind: ZoneKind) -> Color {
    match kind {
        ZoneKind::Avoid => Color::srgba(0.90, 0.25, 0.25, 0.28),
        ZoneKind::Gather => Color::srgba(0.30, 0.85, 0.35, 0.28),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cat_protocol::{CatStats, MapName, MapPosition, WorldSnapshot};
    use cat_sim::terrain_gen::BiomeRole;

    fn census_cat(age_hours: f64, spec: Option<Specialization>, boosted: bool) -> CatSnapshot {
        CatSnapshot {
            id: "c".to_string(),
            name: "C".to_string(),
            position: MapPosition {
                map: MapName::Colony,
                x: 0,
                y: 0,
            },
            activity: CatActivity::Idle,
            destination: None,
            carrying: None,
            specialization: spec,
            age_hours,
            needs: CatNeeds {
                hunger: 1.0,
                thirst: 1.0,
                rest: 1.0,
                health: 1.0,
            },
            current_task: None,
            assigned_building_id: None,
            role_xp: RoleXp {
                hunter: 0.0,
                architect: 0.0,
                ritualist: 0.0,
                warrior: 0.0,
            },
            stats: CatStats { leadership: 0.0 },
            death_time: None,
            parent_ids: Vec::new(),
            parents: Vec::new(),
            boosted,
            pregnant: false,
        }
    }

    #[test]
    fn hud_res_maps_every_resource_kind_to_its_glyph() {
        // Every wire ResourceKind a gather spot can carry maps to a HUD glyph.
        assert_eq!(hud_res_of(ResourceKind::Food), HudRes::Food);
        assert_eq!(hud_res_of(ResourceKind::Water), HudRes::Water);
        assert_eq!(hud_res_of(ResourceKind::Herbs), HudRes::Herbs);
        assert_eq!(hud_res_of(ResourceKind::Materials), HudRes::Materials);
        assert_eq!(hud_res_of(ResourceKind::Refined), HudRes::Refined);
        assert_eq!(hud_res_of(ResourceKind::Weapons), HudRes::Weapons);
        assert_eq!(hud_res_of(ResourceKind::Armor), HudRes::Armor);
        assert_eq!(hud_res_of(ResourceKind::Blessings), HudRes::Blessings);
    }

    fn research(owned: &[&str], blessings: f64) -> ResearchSnapshot {
        ResearchSnapshot {
            owned_node_ids: owned.iter().map(|s| (*s).to_string()).collect(),
            research_points: 0.0,
            researcher_count: 0,
            blessings,
            next_target: None,
        }
    }

    #[test]
    fn node_state_owned_available_locked() {
        // "research_hut" (no prereqs) and "basic_tools" (prereq research_hut).
        let hut = UPGRADE_NODES
            .iter()
            .find(|n| n.id == "research_hut")
            .unwrap();
        let tools = UPGRADE_NODES
            .iter()
            .find(|n| n.id == "basic_tools")
            .unwrap();

        // Nothing owned: the root is available, its child is locked.
        let none: HashSet<&str> = HashSet::new();
        assert_eq!(node_state(hut, &none), NodeState::Available);
        assert_eq!(node_state(tools, &none), NodeState::Locked);

        // Root owned: root reads owned, child becomes available.
        let owned: HashSet<&str> = ["research_hut"].into_iter().collect();
        assert_eq!(node_state(hut, &owned), NodeState::Owned);
        assert_eq!(node_state(tools, &owned), NodeState::Available);
    }

    #[test]
    fn node_line_classifies_colours_and_gates_the_buy_button() {
        let node_by = |id: &str| UPGRADE_NODES.iter().find(|n| n.id == id).unwrap();
        let owned_set = |ids: &[&'static str]| ids.iter().copied().collect::<HashSet<&str>>();

        // Own the root, 5 blessings (covers the 5b children but not an 8b one).
        let r = research(&["research_hut"], 5.0);
        let owned = owned_set(&["research_hut"]);

        // Owned root: green [x], no buy button.
        let hut = node_line(node_by("research_hut"), &r, &owned);
        assert!(hut.label.contains("[x]") && hut.label.contains("Research Hut"));
        assert_eq!(hut.color, NODE_OWNED_COLOR);
        assert!(!hut.show_buy);

        // Available + affordable (5b <= 5): gold [>], buy button shows.
        let tools = node_line(node_by("basic_tools"), &r, &owned);
        assert!(tools.label.contains("[>]"));
        assert_eq!(tools.color, NODE_READY_COLOR);
        assert!(tools.show_buy);

        // Available but unaffordable (water_carriers is 8b > 5): dim [ ], no buy.
        let water = node_line(node_by("water_carriers"), &r, &owned);
        assert!(water.label.contains("[ ]"));
        assert_eq!(water.color, NODE_UNAFFORDABLE_COLOR);
        assert!(!water.show_buy);

        // Locked (a node whose prereqs aren't owned): grey [-], no buy.
        let locked = UPGRADE_NODES
            .iter()
            .find(|n| !n.prerequisites.is_empty() && !owned.contains(n.prerequisites[0]))
            .unwrap();
        let line = node_line(locked, &r, &owned);
        assert!(line.label.contains("[-]"));
        assert_eq!(line.color, NODE_LOCKED_COLOR);
        assert!(!line.show_buy);

        // Header lines carry the balances + a next-target string.
        assert!(tree_currency_line(&r).contains("Blessings: 5"));
        assert!(tree_next_line(&r).starts_with("Next auto-unlock:"));
    }

    #[test]
    fn cycle_stacked_pick_advances_and_wraps() {
        let stack = vec![
            PickCandidate {
                id: "cat".to_string(),
                kind: PickKind::Cat,
            },
            PickCandidate {
                id: "workshop".to_string(),
                kind: PickKind::Building,
            },
            PickCandidate {
                id: "pile".to_string(),
                kind: PickKind::Stockpile,
            },
        ];
        // Nothing selected → first in the stack.
        assert_eq!(cycle_stacked_pick(&stack, None).unwrap().id, "cat");
        // Each click advances to the next, wrapping past the end.
        assert_eq!(
            cycle_stacked_pick(&stack, Some("cat")).unwrap().id,
            "workshop"
        );
        assert_eq!(
            cycle_stacked_pick(&stack, Some("workshop")).unwrap().id,
            "pile"
        );
        assert_eq!(cycle_stacked_pick(&stack, Some("pile")).unwrap().id, "cat");
        // A selection not in the stack falls back to the first.
        assert_eq!(cycle_stacked_pick(&stack, Some("gone")).unwrap().id, "cat");
        // An empty stack yields nothing.
        assert!(cycle_stacked_pick(&[], Some("cat")).is_none());
    }

    #[test]
    fn census_tallies_stages_specs_and_vitals() {
        let mut cats = vec![
            census_cat(3.0, None, false),                         // kitten, unspec
            census_cat(10.0, Some(Specialization::Hunter), true), // young, hunter, boosted
            census_cat(30.0, Some(Specialization::Architect), false), // adult
            census_cat(50.0, Some(Specialization::Warrior), false), // elder
            census_cat(40.0, Some(Specialization::Ritualist), false), // adult
        ];
        // A pregnant adult (counts toward Expecting).
        let mut expecting = census_cat(28.0, None, false);
        expecting.pregnant = true;
        cats.push(expecting);
        // A dead cat must be excluded from every tally — even if flagged pregnant.
        let mut dead = census_cat(35.0, Some(Specialization::Hunter), true);
        dead.death_time = Some(1);
        dead.pregnant = true;
        cats.push(dead);

        let events = vec![
            EventSnapshot {
                message: "Dawnpaw was born to the colony.".to_string(),
                timestamp: 0,
                kind: "birth".to_string(),
            },
            EventSnapshot {
                message: "Two cats are expecting a litter.".to_string(),
                timestamp: 0,
                kind: "conception".to_string(),
            },
            EventSnapshot {
                message: "Mossfur died of old age.".to_string(),
                timestamp: 0,
                kind: "death_old_age".to_string(),
            },
        ];

        let c = colony_census(&cats, &events, Some("Bella"));
        assert_eq!(c.total, 6);
        assert_eq!((c.kittens, c.young, c.adults, c.elders), (1, 1, 3, 1));
        assert_eq!(c.hunters, 1);
        assert_eq!(c.architects, 1);
        assert_eq!(c.ritualists, 1);
        assert_eq!(c.warriors, 1);
        assert_eq!(c.unspecialized, 2); // the kitten + the pregnant adult
        assert_eq!(c.boosted, 1);
        // Only living pregnant cats count; the dead pregnant cat is excluded.
        assert_eq!(c.expecting, 1);
        assert!((c.avg_age_hours - (3.0 + 10.0 + 30.0 + 50.0 + 40.0 + 28.0) / 6.0).abs() < 1e-9);
        // "born" counts as a birth; "expecting a litter" (conception) does not.
        assert_eq!(c.births, 1);
        assert_eq!(c.deaths, 1);
        assert_eq!(c.leader.as_deref(), Some("Bella"));
    }

    #[test]
    fn census_stage_boundaries_and_empty_colony() {
        // Boundary ages land in the higher stage (age < max), matching the sim.
        assert_eq!(
            colony_census(&[census_cat(6.0, None, false)], &[], None).young,
            1
        );
        assert_eq!(
            colony_census(&[census_cat(24.0, None, false)], &[], None).adults,
            1
        );
        assert_eq!(
            colony_census(&[census_cat(48.0, None, false)], &[], None).elders,
            1
        );
        // A non-finite age falls through to elder (sim parity), no NaN avg on empty.
        assert_eq!(
            colony_census(&[census_cat(f64::NAN, None, false)], &[], None).elders,
            1
        );
        let empty = colony_census(&[], &[], None);
        assert_eq!(empty.total, 0);
        assert_eq!(empty.avg_age_hours, 0.0);
        assert_eq!(empty.leader, None);
    }

    #[test]
    fn census_report_lines_are_stable_length_and_readable() {
        let mut expecting = census_cat(30.0, Some(Specialization::Hunter), true);
        expecting.pregnant = true;
        let c = colony_census(
            &[census_cat(3.0, None, false), expecting],
            &[],
            Some("Bella"),
        );
        let lines = census_report_lines(&c);
        assert_eq!(lines.len(), CENSUS_LINES);
        assert_eq!(lines[0], "Population: 2");
        assert_eq!(lines[1], "Leader: Bella");
        assert!(lines[2].contains("★ Boosted: 1"));
        assert_eq!(lines[3], "Expecting: 1");
        // A vacant seat renders a placeholder rather than dropping the line.
        let vacant = census_report_lines(&colony_census(&[], &[], None));
        assert_eq!(vacant.len(), CENSUS_LINES);
        assert_eq!(vacant[1], "Leader: (vacant)");
        assert_eq!(vacant[3], "Expecting: 0");
    }

    #[test]
    fn census_bar_scales_to_the_largest_tally() {
        assert_eq!(census_bar(0, 5, 12), "");
        assert_eq!(census_bar(5, 5, 12).len(), 12);
        assert_eq!(census_bar(3, 0, 12), ""); // max 0 → no divide-by-zero
        assert!(census_bar(1, 5, 12).len() < 12);
    }

    #[test]
    fn boost_button_label_reflects_boosted_state() {
        assert_eq!(boost_button_label(false), "★ Boost");
        assert_eq!(boost_button_label(true), "★ Boosted (click to clear)");
        // The star prefix ties the button to the on-map marker in both states.
        assert!(boost_button_label(false).starts_with('★'));
        assert!(boost_button_label(true).starts_with('★'));
    }

    #[test]
    fn grid_projection_is_flat_and_top_down() {
        assert_eq!(grid_to_world(0, 0), Vec2::ZERO);
        assert_eq!(grid_to_world(1, 0), Vec2::new(TILE, 0.0));
        // Y increases downward on the grid → negative world Y.
        assert_eq!(grid_to_world(0, 1), Vec2::new(0.0, -TILE));
        assert_eq!(grid_to_world(2, 3), Vec2::new(2.0 * TILE, -3.0 * TILE));
    }

    #[test]
    fn window_terrain_covers_the_anchor_window() {
        let tiles = window_terrain(20_240_703);
        assert!(!tiles.is_empty());
        // The anchor tile must be present and within window bounds.
        assert!(
            tiles
                .iter()
                .any(|t| t.x == VILLAGE_ANCHOR.x && t.y == VILLAGE_ANCHOR.y)
        );
        for t in &tiles {
            assert!(
                (VILLAGE_ANCHOR.x - WINDOW_RADIUS..=VILLAGE_ANCHOR.x + WINDOW_RADIUS)
                    .contains(&t.x)
            );
            assert!(
                (VILLAGE_ANCHOR.y - WINDOW_RADIUS..=VILLAGE_ANCHOR.y + WINDOW_RADIUS)
                    .contains(&t.y)
            );
        }
    }

    #[test]
    fn fog_lookup_and_three_tier_state() {
        let revealed = revealed_lookup(&[
            TilePoint { x: 6, y: 6 },
            TilePoint { x: 7, y: 6 },
            TilePoint { x: 6, y: 7 },
        ]);
        // A scout is tentatively uncovering two tiles out in the fog; one of
        // them (6,6) has already committed to revealed.
        let provisional = revealed_lookup(&[TilePoint { x: 9, y: 9 }, TilePoint { x: 6, y: 6 }]);
        assert_eq!(revealed.len(), 3);
        // Revealed tiles are clear.
        assert_eq!(fog_state(&revealed, &provisional, 6, 6), FogState::Clear);
        assert_eq!(fog_state(&revealed, &provisional, 7, 6), FogState::Clear);
        // Provisional-only tiles are dim (half-lifted).
        assert_eq!(fog_state(&revealed, &provisional, 9, 9), FogState::Dim);
        // Everything else is full fog.
        assert_eq!(fog_state(&revealed, &provisional, 6, 5), FogState::Full);
        assert_eq!(fog_state(&revealed, &provisional, 100, 100), FogState::Full);
        // Revealed wins over provisional when a tile is in both (mid-handoff).
        assert!(provisional.contains(&(6, 6)));
        assert_eq!(fog_state(&revealed, &provisional, 6, 6), FogState::Clear);
    }

    #[test]
    fn road_sprite_kind_picks_orientation_from_neighbours() {
        // Cross centre: connected on both axes.
        assert_eq!(road_sprite_kind(true, true, true, true), RoadSprite::Cross);
        // Vertical arm: only north/south neighbours.
        assert_eq!(
            road_sprite_kind(true, true, false, false),
            RoadSprite::StraightV
        );
        assert_eq!(
            road_sprite_kind(true, false, false, false),
            RoadSprite::StraightV
        );
        // Horizontal arm: only east/west neighbours.
        assert_eq!(
            road_sprite_kind(false, false, true, true),
            RoadSprite::StraightH
        );
        // A lone road tile falls back to the cross.
        assert_eq!(
            road_sprite_kind(false, false, false, false),
            RoadSprite::Cross
        );
        // A corner (one vertical + one horizontal) reads as a cross for now.
        assert_eq!(
            road_sprite_kind(true, false, true, false),
            RoadSprite::Cross
        );
    }

    #[test]
    fn minimap_coord_mapping_round_trips_and_scales() {
        // A small revealed patch fits 1 tile/pixel and centres on its bbox.
        let revealed = vec![
            TilePoint { x: 0, y: 0 },
            TilePoint { x: 9, y: 5 },
            TilePoint { x: 4, y: 4 },
        ];
        let view = minimap_view(&revealed);
        assert_eq!(view.tiles_per_px, 1);
        // Every revealed tile lands inside the minimap.
        for t in &revealed {
            let px = world_to_minimap(view, t.x, t.y);
            assert!(px.is_some(), "{t:?} should be on the minimap");
            // At 1 tile/px the pixel maps straight back to the tile.
            let (px, py) = px.unwrap();
            assert_eq!(minimap_to_world(view, px, py), (t.x, t.y));
        }
        // A tile far outside the revealed area is off the minimap.
        assert_eq!(world_to_minimap(view, 9000, 9000), None);

        // A huge revealed span coarsens to >1 tile/pixel (downsample, not truncate).
        let big = vec![TilePoint { x: -200, y: -200 }, TilePoint { x: 200, y: 200 }];
        let bview = minimap_view(&big);
        assert!(bview.tiles_per_px > 1);
        // The extreme corners still map onto the minimap.
        assert!(world_to_minimap(bview, -200, -200).is_some());
        assert!(world_to_minimap(bview, 200, 200).is_some());
    }

    #[test]
    fn minimap_viewport_rect_maps_and_clamps() {
        let view = MinimapView {
            origin_x: 0,
            origin_y: 0,
            tiles_per_px: 1,
        };
        // A viewport covering tiles 10..=19 → pixels [10,20) at 1 tile/px.
        assert_eq!(viewport_rect(view, 10, 10, 19, 19), (10, 10, 20, 20));
        // A viewport running off the top-left clamps to the minimap edge.
        assert_eq!(viewport_rect(view, -50, -50, 4, 4), (0, 0, 5, 5));
        // Off the far edge clamps to MINIMAP_PX.
        let (_, _, x1, y1) = viewport_rect(view, 100, 100, 900, 900);
        assert_eq!((x1, y1), (MINIMAP_PX, MINIMAP_PX));
        // At 4 tiles/pixel the rect coarsens.
        let coarse = MinimapView {
            origin_x: 0,
            origin_y: 0,
            tiles_per_px: 4,
        };
        assert_eq!(viewport_rect(coarse, 0, 0, 7, 7), (0, 0, 2, 2));
    }

    #[test]
    fn minimap_bounds_fall_back_to_the_anchor_when_empty() {
        let (min_x, min_y, max_x, max_y) = minimap_bounds(&[]);
        assert!(min_x < VILLAGE_ANCHOR.x && max_x > VILLAGE_ANCHOR.x);
        assert!(min_y < VILLAGE_ANCHOR.y && max_y > VILLAGE_ANCHOR.y);
    }

    #[test]
    fn window_bounds_is_square_around_the_anchor() {
        let (x0, y0, x1, y1) = window_bounds();
        assert_eq!(x0, VILLAGE_ANCHOR.x - WINDOW_RADIUS);
        assert_eq!(y0, VILLAGE_ANCHOR.y - WINDOW_RADIUS);
        assert_eq!(x1, VILLAGE_ANCHOR.x + WINDOW_RADIUS);
        assert_eq!(y1, VILLAGE_ANCHOR.y + WINDOW_RADIUS);
    }

    #[test]
    fn direction_group_maps_all_eight_sectors() {
        // Sheet order: S, SW, W, NW, N, NE, E, SE (with +y = south).
        assert_eq!(direction_group_f(0.0, 1.0), 0); // S
        assert_eq!(direction_group_f(-1.0, 1.0), 1); // SW
        assert_eq!(direction_group_f(-1.0, 0.0), 2); // W
        assert_eq!(direction_group_f(-1.0, -1.0), 3); // NW
        assert_eq!(direction_group_f(0.0, -1.0), 4); // N
        assert_eq!(direction_group_f(1.0, -1.0), 5); // NE
        assert_eq!(direction_group_f(1.0, 0.0), 6); // E
        assert_eq!(direction_group_f(1.0, 1.0), 7); // SE
        // Zero heading defaults to S; non-unit headings snap to the nearest.
        assert_eq!(direction_group_f(0.0, 0.0), 0);
        assert_eq!(direction_group_f(5.0, 1.0), 6); // mostly east -> E
        assert_eq!(direction_group_f(3.0, 4.0), 7); // down-right -> SE
    }

    #[test]
    fn facing_from_world_delta_flips_north_south() {
        // World +y is north (up), so a downward-on-screen move (-y) faces south.
        assert_eq!(facing_from_delta(Vec2::new(0.0, -TILE)), Some(0)); // S
        assert_eq!(facing_from_delta(Vec2::new(0.0, TILE)), Some(4)); // N
        assert_eq!(facing_from_delta(Vec2::new(TILE, 0.0)), Some(6)); // E
        assert_eq!(facing_from_delta(Vec2::new(-TILE, 0.0)), Some(2)); // W
        // A tiny jitter has no facing (keep the last).
        assert_eq!(facing_from_delta(Vec2::splat(0.01)), None);
    }

    #[test]
    fn walk_step_moves_at_constant_speed_and_never_overshoots() {
        let a = Vec2::ZERO;
        let b = Vec2::new(100.0, 0.0);
        // A step advances by exactly `step` along the line toward the target.
        let mid = walk_step(a, b, 10.0, 1000.0);
        assert!((mid.x - 10.0).abs() < 1e-4);
        assert_eq!(mid.y, 0.0);
        assert!(is_moving(a, b, BODY_MOVE_EPS));
        // Within one step of the target → arrive exactly (no overshoot).
        assert_eq!(walk_step(Vec2::new(95.0, 0.0), b, 10.0, 1000.0), b);
        assert!(!is_moving(b, b, BODY_MOVE_EPS));
        // Absurdly far behind (beyond max_lag) → snap to target.
        assert_eq!(
            walk_step(a, Vec2::new(5000.0, 0.0), 10.0, 1000.0),
            Vec2::new(5000.0, 0.0)
        );
    }

    #[test]
    fn atlas_index_is_group_times_four_plus_frame() {
        assert_eq!(atlas_index(0, 0), 0);
        assert_eq!(atlas_index(0, 3), 3);
        assert_eq!(atlas_index(1, 0), 4);
        assert_eq!(atlas_index(7, 3), 31); // last cell of a 32-cell sheet
    }

    fn amounts(food: f64, materials: f64, refined: f64) -> ResourceAmounts {
        ResourceAmounts {
            food,
            water: 0.0,
            herbs: 0.0,
            materials,
            refined,
            weapons: 0.0,
            armor: 0.0,
            planks: 0.0,
            blocks: 0.0,
            tools: 0.0,
            blessings: 0.0,
        }
    }

    #[test]
    fn dominant_resource_and_pile_prop() {
        assert_eq!(dominant_resource(&amounts(0.0, 0.0, 0.0)), None);
        assert_eq!(
            dominant_resource(&amounts(10.0, 4.0, 0.0)),
            Some(ResourceKind::Food)
        );
        assert_eq!(
            dominant_resource(&amounts(3.0, 20.0, 0.0)),
            Some(ResourceKind::Materials)
        );
        assert_eq!(pile_prop(ResourceKind::Food), PropTexture::Sack);
        assert_eq!(pile_prop(ResourceKind::Materials), PropTexture::StonePile);
        assert_eq!(pile_prop(ResourceKind::Refined), PropTexture::GoldPile);
    }

    #[test]
    fn pile_scale_grows_with_contents_and_clamps() {
        let empty = pile_scale(0.0);
        let some = pile_scale(100.0);
        let full = pile_scale(200.0);
        let over = pile_scale(9999.0);
        assert!(empty < some && some < full);
        assert_eq!(full, over); // clamps at the cap
        assert!(empty > 0.0);
    }

    #[test]
    fn resource_total_sums_storables_only() {
        let mut a = amounts(10.0, 5.0, 2.0);
        a.blessings = 99.0; // excluded
        assert_eq!(resource_total(&a), 17.0);
    }

    #[test]
    fn wall_edges_finds_perimeter_and_excludes_gate() {
        // A lone tile is all perimeter: 4 edges.
        let single: HashSet<(i32, i32)> = [(0, 0)].into_iter().collect();
        assert_eq!(wall_edges(&single, None).len(), 4);

        // A 3x3 block: the centre has no edges; a corner has exactly 2.
        let block: HashSet<(i32, i32)> = (0..3).flat_map(|x| (0..3).map(move |y| (x, y))).collect();
        let edges = wall_edges(&block, None);
        assert!(!edges.iter().any(|(tile, _)| *tile == (1, 1))); // interior
        let corner: Vec<_> = edges.iter().filter(|(tile, _)| *tile == (0, 0)).collect();
        assert_eq!(corner.len(), 2); // N + W for the top-left corner
        assert!(corner.iter().any(|(_, s)| *s == WallSide::N));
        assert!(corner.iter().any(|(_, s)| *s == WallSide::W));
        // A 3x3 block has 12 perimeter edges (4 sides x 3).
        assert_eq!(edges.len(), 12);

        // Excluding the gate edge drops exactly that one.
        let gated = wall_edges(&block, Some(((0, 0), WallSide::N)));
        assert_eq!(gated.len(), 11);
        assert!(!gated.contains(&((0, 0), WallSide::N)));
    }

    #[test]
    fn accept_choice_cycles_general_through_kinds() {
        // General -> first kind -> ... -> last kind -> General.
        let mut choice = AcceptChoice::General;
        assert_eq!(choice.kinds(), STORABLE_KINDS.to_vec());
        choice = choice.next();
        assert_eq!(choice, AcceptChoice::Only(ResourceKind::Food));
        assert_eq!(choice.kinds(), vec![ResourceKind::Food]);

        // Walk the whole cycle and confirm it returns to General after all 7.
        let mut seen = vec![choice];
        for _ in 0..STORABLE_KINDS.len() {
            choice = choice.next();
            seen.push(choice);
        }
        assert_eq!(*seen.last().unwrap(), AcceptChoice::General);
        // Every storable kind appears exactly once in the cycle.
        for kind in STORABLE_KINDS {
            assert!(seen.contains(&AcceptChoice::Only(kind)));
        }
    }

    #[test]
    fn accepts_label_maps_general_single_and_subset() {
        assert_eq!(accepts_label(&STORABLE_KINDS), "General");
        // Order-independent General detection.
        let mut shuffled = STORABLE_KINDS.to_vec();
        shuffled.reverse();
        assert_eq!(accepts_label(&shuffled), "General");
        assert_eq!(accepts_label(&[ResourceKind::Food]), "food only");
        assert_eq!(accepts_label(&[ResourceKind::Refined]), "refined only");
        assert_eq!(
            accepts_label(&[ResourceKind::Food, ResourceKind::Water]),
            "food/water"
        );
        assert!(!is_general_accepts(&[ResourceKind::Food]));
        assert!(is_general_accepts(&STORABLE_KINDS));
    }

    #[test]
    fn point_in_stockpile_rect_membership() {
        let pile = StockpileSnapshot {
            id: "sp".to_string(),
            x1: 2,
            y1: 5,
            x2: 4,
            y2: 3, // deliberately unordered
            accepts: vec![],
            contents: amounts(0.0, 0.0, 0.0),
            gather_spot: None,
        };
        assert!(point_in_stockpile((3, 4), &pile));
        assert!(point_in_stockpile((2, 3), &pile));
        assert!(point_in_stockpile((4, 5), &pile));
        assert!(!point_in_stockpile((1, 4), &pile));
        assert!(!point_in_stockpile((3, 6), &pile));
    }

    #[test]
    fn building_texture_mapping_and_sizes() {
        // Direct 1:1 mappings.
        assert_eq!(
            building_texture(BuildingType::Shrine),
            Some(BuildingTexture::Shrine)
        );
        assert_eq!(
            building_texture(BuildingType::Smithy),
            Some(BuildingTexture::Smithy)
        );
        // Aliased mappings.
        assert_eq!(
            building_texture(BuildingType::FoodStorage),
            Some(BuildingTexture::Storehouse)
        );
        assert_eq!(
            building_texture(BuildingType::MouseFarm),
            Some(BuildingTexture::Storehouse)
        );
        assert_eq!(
            building_texture(BuildingType::Field),
            Some(BuildingTexture::Market)
        );
        assert_eq!(
            building_texture(BuildingType::WaterBowl),
            Some(BuildingTexture::Well)
        );
        assert_eq!(
            building_texture(BuildingType::Nursery),
            Some(BuildingTexture::Den)
        );
        // P16 craft-station workshops each map to their dedicated sprite.
        assert_eq!(
            building_texture(BuildingType::WoodCutter),
            Some(BuildingTexture::WoodCutter)
        );
        assert_eq!(
            building_texture(BuildingType::StonePrep),
            Some(BuildingTexture::StonePrep)
        );
        assert_eq!(
            building_texture(BuildingType::Woodworking),
            Some(BuildingTexture::Woodworking)
        );
        // P18 clothing-chain craft stations get their own sprites.
        assert_eq!(
            building_texture(BuildingType::Clothier),
            Some(BuildingTexture::Clothier)
        );
        assert_eq!(
            building_texture(BuildingType::Tannery),
            Some(BuildingTexture::Tannery)
        );
        // The smelter reuses the smithy forge sprite.
        assert_eq!(
            building_texture(BuildingType::Smelter),
            Some(BuildingTexture::Smithy)
        );
        // Walls render as infra, not a point marker.
        assert_eq!(building_texture(BuildingType::Walls), None);

        // Square sprites keep aspect 1; the two non-square ones don't.
        assert_eq!(building_aspect(BuildingTexture::Shrine), 1.0);
        assert!(building_aspect(BuildingTexture::Market) > 1.0); // wide
        assert!(building_aspect(BuildingTexture::Well) < 1.0); // tall
    }

    #[test]
    fn footprint_sprite_spans_tiles_and_sits_on_front_edge() {
        let nw = TilePoint { x: 6, y: 6 };
        // A 3x3 square building: 3 tiles wide, 3 tall (aspect 1).
        let (base, size) = footprint_sprite(
            nw,
            FootprintSize {
                width: 3,
                height: 3,
            },
            1.0,
        );
        assert_eq!(size.x, 3.0 * TILE);
        assert_eq!(size.y, 3.0 * TILE);
        // Centre x is across the 3-tile span; base y is the bottom of the front row.
        assert_eq!(base.x, (6.0 + 1.0) * TILE); // centre of cols 6,7,8
        assert_eq!(base.y, -8.0 * TILE - TILE / 2.0); // front row y=8, bottom edge
        // A 1x1 default reduces to the old point placement.
        let (b1, s1) = footprint_sprite(
            nw,
            FootprintSize {
                width: 1,
                height: 1,
            },
            1.0,
        );
        assert_eq!(s1, Vec2::splat(TILE));
        assert_eq!(b1, Vec2::new(6.0 * TILE, -6.0 * TILE - TILE / 2.0));
        // A wide sprite (aspect 2) is half as tall as it is wide.
        let (_, sw) = footprint_sprite(
            nw,
            FootprintSize {
                width: 2,
                height: 1,
            },
            2.0,
        );
        assert_eq!(sw, Vec2::new(2.0 * TILE, TILE));
    }

    #[test]
    fn ysort_puts_lower_sprites_in_front() {
        // Lower on the map = more negative world y = larger z (drawn in front).
        let higher = ysort_z(0.0);
        let lower = ysort_z(-100.0);
        assert!(lower > higher);
        // Monotonic in screen depth.
        assert!(ysort_z(-200.0) > ysort_z(-100.0));
    }

    fn climate_tile(biome: BiomeRole, climate_biome: Biome, x: i32, y: i32) -> TerrainTile {
        TerrainTile {
            x,
            y,
            elevation: 0.0,
            moisture: 0.0,
            height: 1,
            biome,
            climate_biome,
            terrain: cat_sim::terrain_gen::TerrainRole::Flat,
            river: None,
            stairs: None,
            decoration: None,
        }
    }

    #[test]
    fn ground_texture_maps_climate_biomes_to_sprites() {
        // Sandy biomes -> sand, stony -> rocky, badlands -> dirt, wetlands ->
        // farmland (mud), flower fields -> a flowered tile.
        for b in [Biome::Desert, Biome::Beach, Biome::Savanna] {
            assert_eq!(
                ground_texture(&climate_tile(BiomeRole::Grassland, b, 0, 0)),
                GroundTexture::Sand
            );
        }
        assert_eq!(
            ground_texture(&climate_tile(BiomeRole::Grassland, Biome::Mountains, 0, 0)),
            GroundTexture::Rocky
        );
        assert_eq!(
            ground_texture(&climate_tile(BiomeRole::Grassland, Biome::Badlands, 0, 0)),
            GroundTexture::Dirt
        );
        for b in [Biome::Swamp, Biome::Marsh] {
            assert_eq!(
                ground_texture(&climate_tile(BiomeRole::Lowland, b, 0, 0)),
                GroundTexture::Farmland
            );
        }
        assert!(matches!(
            ground_texture(&climate_tile(
                BiomeRole::Grassland,
                Biome::FlowerField,
                1,
                1
            )),
            GroundTexture::FlowersRed | GroundTexture::FlowersWhite | GroundTexture::FlowersBlue
        ));
        assert_eq!(
            ground_texture(&climate_tile(BiomeRole::Grassland, Biome::Meadow, 0, 0)),
            GroundTexture::GrassVar
        );
        // Cold land now has a dedicated snow tile.
        for b in [Biome::Tundra, Biome::SnowyPlains, Biome::SnowyTaiga] {
            assert_eq!(
                ground_texture(&climate_tile(BiomeRole::Grassland, b, 0, 0)),
                GroundTexture::Snow
            );
        }
        // Forests and plains land on grass; grassland gets the variant sprite on
        // the deterministic subset only.
        assert_eq!(
            ground_texture(&climate_tile(BiomeRole::Forest, Biome::OakForest, 3, 3)),
            GroundTexture::Grass
        );
        assert_eq!(
            ground_texture(&climate_tile(BiomeRole::Grassland, Biome::Plains, 2, 3)),
            GroundTexture::GrassVar
        );
        assert_eq!(
            ground_texture(&climate_tile(BiomeRole::Grassland, Biome::Plains, 2, 4)),
            GroundTexture::Grass
        );
    }

    #[test]
    fn water_biomes_are_detected_and_tinted_distinctly() {
        for biome in [Biome::Ocean, Biome::Lake, Biome::River, Biome::Ice] {
            assert!(is_water_biome(biome), "{biome:?} should render as water");
        }
        for biome in [Biome::Plains, Biome::Desert, Biome::OakForest, Biome::Hills] {
            assert!(!is_water_biome(biome), "{biome:?} is land");
        }
        // The tint accessor reflects each biome's palette colour, so distinct
        // biomes produce distinct ground colours.
        assert_ne!(biome_tint(Biome::Plains), biome_tint(Biome::Desert));
        assert_ne!(biome_tint(Biome::OakForest), biome_tint(Biome::SnowyPlains));
        assert_eq!(biome_tint(Biome::Plains), Color::srgb_u8(124, 176, 84));
    }

    #[test]
    fn rock_scale_grows_with_size() {
        assert!(rock_scale(RockSize::Small) < rock_scale(RockSize::Medium));
        assert!(rock_scale(RockSize::Medium) < rock_scale(RockSize::Large));
    }

    #[test]
    fn biome_trees_pick_conifer_deadtree_or_broadleaf_and_shore_detection() {
        // Snowy biomes get snow-capped conifers; cool/boreal plain conifers.
        for b in [Biome::SnowyTaiga, Biome::SnowyPlains, Biome::Tundra] {
            assert_eq!(biome_tree(b), TreeSprite::SnowPine, "{b:?}");
        }
        for b in [Biome::PineForest, Biome::Taiga, Biome::Mountains] {
            assert_eq!(biome_tree(b), TreeSprite::Pine, "{b:?}");
        }
        // Hot-dry and wetland biomes get bare dead trees (no cactus in the packs).
        for b in [
            Biome::Desert,
            Biome::Savanna,
            Biome::Badlands,
            Biome::Swamp,
            Biome::Marsh,
        ] {
            assert_eq!(biome_tree(b), TreeSprite::DeadTree, "{b:?}");
        }
        for b in [
            Biome::OakForest,
            Biome::BirchForest,
            Biome::Jungle,
            Biome::Plains,
        ] {
            assert_eq!(biome_tree(b), TreeSprite::Oak, "{b:?}");
        }

        // A lone water tile is all shore; a fully surrounded one is not.
        let mut water = HashSet::new();
        water.insert((5, 5));
        assert!(is_shore(5, 5, &water));
        for d in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            water.insert((5 + d.0, 5 + d.1));
        }
        assert!(!is_shore(5, 5, &water));
    }

    #[test]
    fn nearest_pick_respects_radius_and_picks_closest() {
        let items = vec![
            ("a".to_string(), Vec2::new(0.0, 0.0)),
            ("b".to_string(), Vec2::new(10.0, 0.0)),
        ];
        // Click near b, within radius → b.
        assert_eq!(
            nearest_id(Vec2::new(9.0, 0.0), &items, TILE * 0.5),
            Some("b".to_string())
        );
        // Click near a → a.
        assert_eq!(
            nearest_id(Vec2::new(1.0, 0.0), &items, TILE * 0.5),
            Some("a".to_string())
        );
        // Click far from both → none.
        assert_eq!(
            nearest_id(Vec2::new(100.0, 100.0), &items, TILE * 0.5),
            None
        );
    }

    #[test]
    fn click_action_maps_buttons() {
        assert_eq!(click_action(MouseButton::Left), Some(ClickTarget::Cat));
        assert_eq!(
            click_action(MouseButton::Right),
            Some(ClickTarget::Building)
        );
        // Middle is drag-pan, not a selection.
        assert_eq!(click_action(MouseButton::Middle), None);
    }

    #[test]
    fn selection_toggle_state_machine() {
        // New cat selects it.
        assert_eq!(
            toggle_selection(None, Some("a".to_string())),
            Some("a".to_string())
        );
        // Re-clicking the same cat deselects.
        assert_eq!(toggle_selection(Some("a"), Some("a".to_string())), None);
        // Clicking a different cat switches.
        assert_eq!(
            toggle_selection(Some("a"), Some("b".to_string())),
            Some("b".to_string())
        );
        // Clicking empty ground deselects.
        assert_eq!(toggle_selection(Some("a"), None), None);
    }

    #[test]
    fn world_to_tile_inverts_grid_projection() {
        for (x, y) in [(0, 0), (3, 5), (-4, 2), (6, 6), (-7, -1)] {
            assert_eq!(world_to_tile(grid_to_world(x, y)), (x, y));
        }
        // Rounds to the nearest tile centre.
        assert_eq!(world_to_tile(Vec2::new(TILE * 2.1, -TILE * 2.9)), (2, 3));
    }

    #[test]
    fn drag_tile_rect_normalizes_and_clamps() {
        // Backwards drag normalises to (min, max).
        assert_eq!(drag_tile_rect((5, 5), (2, 3), 8), ((2, 3), (5, 5)));
        // Oversized drag clamps each side to `max` tiles from the start corner.
        let (min, max) = drag_tile_rect((0, 0), (20, 20), 8);
        assert_eq!(min, (0, 0));
        assert_eq!(max, (7, 7));
        assert_eq!(max.0 - min.0 + 1, 8);
        assert_eq!(max.1 - min.1 + 1, 8);
        // A single-tile click is a 1x1 rect.
        assert_eq!(drag_tile_rect((4, 4), (4, 4), 8), ((4, 4), (4, 4)));
    }

    #[test]
    fn tool_mode_paint_kind_mapping() {
        assert_eq!(ToolMode::Inspect.paint_kind(), None);
        assert_eq!(ToolMode::AvoidZone.paint_kind(), Some(PaintKind::Avoid));
        assert_eq!(ToolMode::GatherZone.paint_kind(), Some(PaintKind::Gather));
        assert_eq!(ToolMode::Stockpile.paint_kind(), Some(PaintKind::Stockpile));
    }

    #[test]
    fn life_stage_boundaries() {
        assert_eq!(life_stage(0.0), "kitten");
        assert_eq!(life_stage(5.9), "kitten");
        assert_eq!(life_stage(6.0), "young");
        assert_eq!(life_stage(23.9), "young");
        assert_eq!(life_stage(24.0), "adult");
        assert_eq!(life_stage(47.9), "adult");
        assert_eq!(life_stage(48.0), "elder");
    }

    #[test]
    fn events_classify_by_wire_kind_and_colour() {
        // Births + conceptions both read as positive Birth.
        assert_eq!(event_kind_of("birth"), EventKind::Birth);
        assert_eq!(event_kind_of("conception"), EventKind::Birth);
        // Every death cause maps to Death; a defender lost in a raid is a death.
        assert_eq!(event_kind_of("death_old_age"), EventKind::Death);
        assert_eq!(event_kind_of("death_starvation"), EventKind::Death);
        assert_eq!(event_kind_of("death_raid"), EventKind::Death);
        // Raid phases map to Raid.
        assert_eq!(event_kind_of("raid_launched"), EventKind::Raid);
        assert_eq!(event_kind_of("raid_wipeout"), EventKind::Raid);
        // Crises are amber; recoveries get the softer positive treatment.
        assert_eq!(event_kind_of("water_crisis"), EventKind::Crisis);
        assert_eq!(event_kind_of("dehydration_crisis"), EventKind::Crisis);
        assert_eq!(event_kind_of("water_recovered"), EventKind::Progress);
        // Governance.
        assert_eq!(event_kind_of("election_started"), EventKind::Election);
        assert_eq!(event_kind_of("leader_change"), EventKind::Election);
        // Progress: research, building, trade, blessings.
        assert_eq!(event_kind_of("research_unlocked"), EventKind::Progress);
        assert_eq!(event_kind_of("village_founded"), EventKind::Progress);
        assert_eq!(event_kind_of("trade_sell"), EventKind::Progress);
        assert_eq!(event_kind_of("tithe"), EventKind::Progress);
        // Trader lifecycle / jobs / unknown / empty (pre-taxonomy) → neutral.
        assert_eq!(event_kind_of("trader_arrived"), EventKind::Neutral);
        assert_eq!(event_kind_of("job_completed"), EventKind::Neutral);
        assert_eq!(event_kind_of(""), EventKind::Neutral);
        // Kinds map to distinct colours + glyphs.
        assert_ne!(event_color(EventKind::Birth), event_color(EventKind::Death));
        assert_ne!(
            event_color(EventKind::Election),
            event_color(EventKind::Crisis)
        );
        assert_eq!(event_glyph(EventKind::Birth), '+');
        assert_eq!(event_glyph(EventKind::Death), 'x');
    }

    #[test]
    fn goods_formatting_bands_labels_and_treasury() {
        assert_eq!(quality_band(0), "Crude");
        assert_eq!(quality_band(2), "Fine");
        assert_eq!(quality_band(4), "Masterwork");
        // Out-of-range quality clamps to the top band rather than panicking.
        assert_eq!(quality_band(9), "Masterwork");
        assert_eq!(capitalize_word("wood"), "Wood");
        assert_eq!(capitalize_word(""), "");

        let mug = ItemStackSnapshot {
            kind: "mug".to_string(),
            material: "wood".to_string(),
            quality: 2,
            count: 3,
            value: 12,
        };
        let label = item_label(&mug);
        assert!(label.contains("Fine Wood Mug"));
        assert!(label.contains("x3"));
        assert!(label.contains("12g"));
        assert!(label.contains("(36g)")); // count * value subtotal

        let bowl = ItemStackSnapshot {
            kind: "bowl".to_string(),
            material: "stone".to_string(),
            quality: 1,
            count: 2,
            value: 5,
        };
        // Treasury sums count*value across stacks: 3*12 + 2*5 = 46.
        assert_eq!(treasury_total(&[mug.clone(), bowl]), 46);
        assert_eq!(treasury_total(&[]), 0);

        // HUD treasury line reflects the same total.
        assert_eq!(hud_treasury_line(&[mug]), "Treasury: 36g");
        assert_eq!(hud_treasury_line(&[]), "Treasury: 0g");
    }

    #[test]
    fn trade_offer_labels_affordability_and_coin() {
        let sell = TraderBuyOffer {
            kind: "mug".to_string(),
            material: "wood".to_string(),
            quality: 2,
            available: 3,
            unit_price: 8.0,
        };
        let label = sell_offer_label(&sell);
        assert!(label.contains("Fine Wood Mug"));
        assert!(label.contains("x3"));
        assert!(label.contains("8g ea"));

        let buy = TraderSellOffer {
            resource: ResourceKind::Food,
            unit_price: 5.0,
        };
        // Affordable and unaffordable buy labels.
        assert_eq!(buy_offer_label(&buy, 20.0), "Food - 5g ea");
        assert!(buy_offer_label(&buy, 2.0).contains("low coin"));

        assert!(can_afford(5.0, 5.0));
        assert!(can_afford(10.0, 5.0));
        assert!(!can_afford(4.0, 5.0));

        assert_eq!(coin_line(42.0), "Coin: 42g");
        assert_eq!(coin_line(0.0), "Coin: 0g");
    }

    #[test]
    fn material_tints_are_distinct_by_material() {
        assert_ne!(material_tint("wood"), material_tint("metal"));
        assert_ne!(material_tint("stone"), material_tint("gem"));
        // An unknown material falls back to a neutral tint, not a panic.
        assert_eq!(material_tint("mithril"), material_tint("unknown"));
    }

    #[test]
    fn relative_time_and_announcement_line_format() {
        assert_eq!(relative_time(10_000, 5_000), "5s");
        assert_eq!(relative_time(200_000, 20_000), "3m");
        assert_eq!(relative_time(7_400_000, 200_000), "2h");
        // Never negative if a stray future timestamp arrives.
        assert_eq!(relative_time(0, 5_000), "0s");
        let line = announcement_line(60_000, "birth", "Pebble was born", 0);
        assert!(line.contains("1m"));
        assert!(line.contains('+')); // birth glyph from the "birth" kind
        assert!(line.contains("Pebble was born"));
    }

    #[test]
    fn resource_icons_map_all_kinds_to_distinct_tints() {
        // Every HUD resource has a tint, and neighbouring resources differ so the
        // readout reads at a glance.
        let tints: Vec<Color> = HUD_RESOURCES
            .iter()
            .map(|k| resource_icon_tint(*k))
            .collect();
        assert_eq!(tints.len(), 11);
        assert_ne!(
            resource_icon_tint(HudRes::Food),
            resource_icon_tint(HudRes::Water)
        );
        assert_ne!(
            resource_icon_tint(HudRes::Materials),
            resource_icon_tint(HudRes::Refined)
        );
        // The refinement tier is distinct from each other and from refined.
        assert_ne!(
            resource_icon_tint(HudRes::Planks),
            resource_icon_tint(HudRes::Blocks)
        );
        assert_ne!(
            resource_icon_tint(HudRes::Blocks),
            resource_icon_tint(HudRes::Tools)
        );
    }

    #[test]
    fn hud_resource_value_formats_caps_and_bare_values() {
        let r = ResourceAmounts {
            food: 150.0,
            water: 100.0,
            herbs: 16.0,
            materials: 24.0,
            refined: 0.0,
            weapons: 3.0,
            armor: 2.0,
            planks: 12.0,
            blocks: 7.0,
            tools: 1.0,
            blessings: 4.5,
        };
        let cap = ResourceCapacities {
            food: 200.0,
            water: 200.0,
            herbs: 100.0,
            materials: 100.0,
            refined: 100.0,
            weapons: 0.0,
            armor: 0.0,
            planks: 100.0,
            blocks: 100.0,
            tools: 100.0,
        };
        assert_eq!(hud_resource_value(HudRes::Food, &r, &cap), "150 / 200");
        // The refinement tier shows amount / cap like the other storables.
        assert_eq!(hud_resource_value(HudRes::Planks, &r, &cap), "12 / 100");
        assert_eq!(hud_resource_value(HudRes::Blocks, &r, &cap), "7 / 100");
        assert_eq!(hud_resource_value(HudRes::Tools, &r, &cap), "1 / 100");
        assert_eq!(hud_resource_value(HudRes::Weapons, &r, &cap), "3");
        assert_eq!(hud_resource_value(HudRes::Blessings, &r, &cap), "4.5");
    }

    #[test]
    fn stockpile_tooltip_names_shrine_and_reports_contents() {
        let pile = StockpileSnapshot {
            id: "stockpile-1".to_string(),
            x1: 0,
            y1: 0,
            x2: 1,
            y2: 1,
            accepts: vec![ResourceKind::Food],
            contents: ResourceAmounts {
                food: 12.0,
                water: 0.0,
                herbs: 0.0,
                materials: 0.0,
                refined: 0.0,
                weapons: 0.0,
                armor: 0.0,
                planks: 0.0,
                blocks: 0.0,
                tools: 0.0,
                blessings: 0.0,
            },
            gather_spot: None,
        };
        let tip = stockpile_tooltip(&pile);
        assert!(tip.contains("Stockpile"));
        assert!(tip.contains("food only"));
        assert!(tip.contains("~12"));

        let shrine = StockpileSnapshot {
            id: SHRINE_STOCKPILE_ID.to_string(),
            ..pile
        };
        assert!(stockpile_tooltip(&shrine).contains("Shrine reservoir"));
    }

    #[test]
    fn resource_hint_labels() {
        assert_eq!(resource_hint_label(ResourceHint::Wood), "wood");
        assert_eq!(resource_hint_label(ResourceHint::None), "open ground");
    }

    #[test]
    fn production_bar_and_pct_and_line() {
        assert_eq!(progress_pct(0.0), 0);
        assert_eq!(progress_pct(0.5), 50);
        assert_eq!(progress_pct(1.0), 100);
        // Clamped to the valid range.
        assert_eq!(progress_pct(1.5), 100);
        assert_eq!(progress_pct(-0.2), 0);
        // Bar has `cells` characters between the brackets, filled proportionally.
        assert_eq!(progress_bar(0.0, 10), "[----------]");
        assert_eq!(progress_bar(1.0, 10), "[##########]");
        assert_eq!(progress_bar(0.4, 10), "[####------]");
        let line = production_line("plank", 0.4);
        assert!(line.contains("making plank"));
        assert!(line.contains("[####------]"));
        assert!(line.contains("40%"));
    }

    #[test]
    fn tile_tooltip_names_the_biome() {
        // Deterministic: the same seed/tile always names the same biome, and the
        // text has a biome line plus a feature line.
        let tip = tile_tooltip(20_240_703, 6, 6);
        assert!(tip.lines().count() >= 2, "biome + feature lines: {tip:?}");
        assert!(!tip.is_empty());
    }

    #[test]
    fn hover_text_picks_the_cat_under_the_cursor() {
        let json = r#"{
            "now": 0, "worldSeed": 1, "onlineCount": 1,
            "colonies": [{
                "id":"c1","name":"A","status":"thriving",
                "resources":{"food":1,"water":1,"herbs":0,"materials":0,"refined":0,"weapons":0,"armor":0,"blessings":0},
                "storage":{"capacities":{"food":200,"water":200,"herbs":100,"materials":100,"refined":100,"weapons":50,"armor":50},"foodCapacity":200,"titheRates":{"food":20,"refined":5}},
                "leader":null,
                "cats":[
                    {"id":"k1","name":"Milo","position":{"map":"colony","x":1,"y":2},"activity":"idle","destination":null,"carrying":null,"specialization":null,"ageHours":30.0,"needs":{"hunger":80,"thirst":70,"rest":60,"health":90},"currentTask":null,"assignedBuildingId":null,"roleXp":{"hunter":0,"architect":0,"ritualist":0,"warrior":0},"stats":{"leadership":10},"deathTime":null}
                ],
                "jobs":[],"upgrades":[],"events":[],
                "housing":{"population":1,"capacity":4,"pressure":0.5,"villageLevel":1},
                "research":{"ownedNodeIds":[],"researchPoints":0,"researcherCount":0,"blessings":0,"nextTarget":null},
                "election":null,"voteKick":null,"zones":[],
                "threat":{"pressure":0,"band":"calm","raidActive":false,"warriors":0,"weapons":0,"armor":0},
                "raiders":[],"buildings":[],"stockpiles":[],"claimedTiles":[],"villageGate":null,"villageRadius":4,"anchor":{"x":6,"y":6}
            }]
        }"#;
        let snap: WorldSnapshot = serde_json::from_str(json).expect("parse snapshot");
        let colony = &snap.colonies[0];
        // The cat sits on tile (1,2); its world position is where the cursor picks it.
        let at_cat = grid_to_world(1, 2);
        let tip = hover_text(colony, snap.world_seed, at_cat).expect("cat tooltip");
        assert!(tip.contains("Milo"));
        assert!(tip.contains("hunger 80"));
        // Empty ground still yields a tile tooltip (biome + resource), never None.
        let tile_tip =
            hover_text(colony, snap.world_seed, Vec2::new(9000.0, 9000.0)).expect("tile tooltip");
        assert!(!tile_tip.is_empty());
    }

    #[test]
    fn all_building_types_have_labels() {
        for building in [
            BuildingType::Shrine,
            BuildingType::Den,
            BuildingType::Workshop,
            BuildingType::Field,
            BuildingType::Smithy,
            BuildingType::Barracks,
            BuildingType::Clothier,
            BuildingType::Tannery,
            BuildingType::Smelter,
        ] {
            assert!(!building_label(building).is_empty());
        }
    }

    #[test]
    fn deserializes_a_world_snapshot_and_counts_cats() {
        let json = r#"{
            "now": 0, "worldSeed": 1, "onlineCount": 2,
            "colonies": [{
                "id":"c1","name":"A","status":"thriving",
                "resources":{"food":1,"water":1,"herbs":0,"materials":0,"refined":0,"weapons":0,"armor":0,"blessings":0},
                "storage":{"capacities":{"food":200,"water":200,"herbs":100,"materials":100,"refined":100,"weapons":50,"armor":50},"foodCapacity":200,"titheRates":{"food":20,"refined":5}},
                "leader":null,
                "cats":[
                    {"id":"k1","name":"A","position":{"map":"colony","x":1,"y":2},"activity":"idle","destination":null,"carrying":null,"specialization":null,"ageHours":30.0,"needs":{"hunger":100,"thirst":100,"rest":100,"health":100},"currentTask":null,"assignedBuildingId":null,"roleXp":{"hunter":0,"architect":0,"ritualist":0,"warrior":0},"stats":{"leadership":10},"deathTime":null},
                    {"id":"k2","name":"B","position":{"map":"colony","x":3,"y":4},"activity":"idle","destination":null,"carrying":null,"specialization":null,"ageHours":30.0,"needs":{"hunger":100,"thirst":100,"rest":100,"health":100},"currentTask":null,"assignedBuildingId":null,"roleXp":{"hunter":0,"architect":0,"ritualist":0,"warrior":0},"stats":{"leadership":10},"deathTime":null}
                ],
                "jobs":[],"upgrades":[],"events":[],
                "housing":{"population":2,"capacity":4,"pressure":0.5,"villageLevel":1},
                "research":{"ownedNodeIds":[],"researchPoints":0,"researcherCount":0,"blessings":0,"nextTarget":null},
                "election":null,"voteKick":null,"zones":[],
                "threat":{"pressure":0,"band":"calm","raidActive":false,"warriors":0,"weapons":0,"armor":0},
                "raiders":[],"buildings":[],"claimedTiles":[],"villageGate":null,"villageRadius":4,"anchor":{"x":6,"y":6}
            }]
        }"#;
        let snap: WorldSnapshot = serde_json::from_str(json).expect("parse snapshot");
        assert_eq!(snap.colonies.len(), 1);
        assert_eq!(snap.colonies[0].cats.len(), 2);
        assert_eq!(snap.online_count, 2);
        assert!(dashboard_header_text(&snap.colonies[0], 2).contains("Colony: A"));
    }

    #[test]
    fn ledger_hud_text_marks_freshness() {
        let reported = ResourceAmounts {
            food: 148.0,
            water: 100.0,
            herbs: 16.0,
            materials: 24.0,
            refined: 0.0,
            weapons: 0.0,
            armor: 0.0,
            planks: 0.0,
            blocks: 0.0,
            tools: 0.0,
            blessings: 0.0,
        };
        let exact = ledger_hud_text(&StockLedgerSnapshot {
            reported,
            last_counted: 0,
            accurate: true,
        });
        assert!(exact.contains("exact"));
        assert!(exact.contains("F148"));
        assert!(!exact.contains('~'));

        let stale = ledger_hud_text(&StockLedgerSnapshot {
            reported,
            last_counted: 0,
            accurate: false,
        });
        assert!(stale.contains("stale"));
        assert!(stale.contains("Accounting Tent"));
        assert!(stale.contains("~F148"));
    }

    #[test]
    fn officer_holder_name_resolves_vacancy_and_dangling() {
        // farmer -> k1 (present, name "A"); captain -> "ghost" (not in cats).
        let json = r#"{
            "now": 0, "worldSeed": 1, "onlineCount": 1,
            "colonies": [{
                "id":"c1","name":"A","status":"thriving",
                "resources":{"food":1,"water":1,"herbs":0,"materials":0,"refined":0,"weapons":0,"armor":0,"blessings":0},
                "storage":{"capacities":{"food":200,"water":200,"herbs":100,"materials":100,"refined":100,"weapons":50,"armor":50},"foodCapacity":200,"titheRates":{"food":20,"refined":5}},
                "leader":null,
                "cats":[
                    {"id":"k1","name":"Moss","position":{"map":"colony","x":1,"y":2},"activity":"idle","destination":null,"carrying":null,"specialization":null,"ageHours":30.0,"needs":{"hunger":100,"thirst":100,"rest":100,"health":100},"currentTask":null,"assignedBuildingId":null,"roleXp":{"hunter":0,"architect":0,"ritualist":0,"warrior":0},"stats":{"leadership":10},"deathTime":null}
                ],
                "jobs":[],"upgrades":[],"events":[],
                "housing":{"population":1,"capacity":4,"pressure":0.5,"villageLevel":1},
                "research":{"ownedNodeIds":[],"researchPoints":0,"researcherCount":0,"blessings":0,"nextTarget":null},
                "election":null,"voteKick":null,"zones":[],
                "threat":{"pressure":0,"band":"calm","raidActive":false,"warriors":0,"weapons":0,"armor":0},
                "raiders":[],"buildings":[],"claimedTiles":[],"villageGate":null,"villageRadius":4,"anchor":{"x":6,"y":6},
                "officers":{"farmer":"k1","captain":"ghost"}
            }]
        }"#;
        let snap: WorldSnapshot = serde_json::from_str(json).expect("parse snapshot");
        let colony = &snap.colonies[0];
        assert_eq!(
            officer_holder_name(colony, OfficerRole::Farmer),
            Some("Moss")
        );
        assert_eq!(officer_holder_name(colony, OfficerRole::Steward), None); // vacant
        assert_eq!(officer_holder_name(colony, OfficerRole::Captain), None); // dangling id
    }

    #[test]
    fn building_inspector_text_reports_status_and_workers() {
        // A complete workshop with Moss assigned, plus an unfinished den.
        let json = r#"{
            "now": 0, "worldSeed": 1, "onlineCount": 1,
            "colonies": [{
                "id":"c1","name":"A","status":"thriving",
                "resources":{"food":1,"water":1,"herbs":0,"materials":0,"refined":0,"weapons":0,"armor":0,"blessings":0},
                "storage":{"capacities":{"food":200,"water":200,"herbs":100,"materials":100,"refined":100,"weapons":50,"armor":50},"foodCapacity":200,"titheRates":{"food":20,"refined":5}},
                "leader":null,
                "cats":[
                    {"id":"k1","name":"Moss","position":{"map":"colony","x":1,"y":2},"activity":"working","destination":null,"carrying":null,"specialization":null,"ageHours":30.0,"needs":{"hunger":100,"thirst":100,"rest":100,"health":100},"currentTask":null,"assignedBuildingId":"b1","roleXp":{"hunter":0,"architect":0,"ritualist":0,"warrior":0},"stats":{"leadership":10},"deathTime":null}
                ],
                "jobs":[],"upgrades":[],"events":[],
                "housing":{"population":1,"capacity":4,"pressure":0.5,"villageLevel":1},
                "research":{"ownedNodeIds":[],"researchPoints":0,"researcherCount":0,"blessings":0,"nextTarget":null},
                "election":null,"voteKick":null,"zones":[],
                "threat":{"pressure":0,"band":"calm","raidActive":false,"warriors":0,"weapons":0,"armor":0},
                "raiders":[],
                "buildings":[
                    {"id":"b1","type":"workshop","level":2,"constructionProgress":100.0,"worldPosition":{"x":7,"y":6},"position":{"x":1,"y":0},"staffCount":1,"staffCap":1,"productionProgress":0.4,"productionOutput":"refined"},
                    {"id":"b2","type":"den","level":1,"constructionProgress":40.0,"worldPosition":{"x":5,"y":6},"position":{"x":-1,"y":0}}
                ],
                "claimedTiles":[],"villageGate":null,"villageRadius":4,"anchor":{"x":6,"y":6}
            }]
        }"#;
        let snap: WorldSnapshot = serde_json::from_str(json).expect("parse snapshot");
        let colony = &snap.colonies[0];
        let workshop = &colony.buildings[0];
        let den = &colony.buildings[1];
        let ws = building_inspector_text(workshop, colony);
        assert!(ws.contains("workshop"));
        assert!(ws.contains("Lv 2"));
        assert!(ws.contains("operational"));
        assert!(ws.contains("Moss")); // assigned worker name
        assert!(ws.contains("staffed: 1/1")); // live staff count / cap
        assert!(ws.contains("making refined")); // live production output
        assert!(ws.contains("[####------]")); // progress bar at 0.4
        assert!(ws.contains("40%"));
        // A den (staff_cap 0) under construction shows neither staffing nor output.
        let den_text = building_inspector_text(den, colony);
        assert!(den_text.contains("under construction 40%"));
        assert!(!den_text.contains("staffed"));
        assert!(!den_text.contains("making"));
    }

    #[test]
    fn need_bars_and_skills_summarise_a_cat() {
        let needs = CatNeeds {
            hunger: 80.0,
            thirst: 25.0,
            rest: 50.0,
            health: 95.0,
        };
        assert_eq!(cat_need_value(&needs, NeedKind::Thirst), 25.0);
        // Colour bands: comfortable -> green, low -> amber, critical -> red.
        assert_eq!(need_bar_color(80.0), Color::srgb(0.42, 0.72, 0.36));
        assert_eq!(need_bar_color(45.0), Color::srgb(0.88, 0.72, 0.30));
        assert_eq!(need_bar_color(10.0), Color::srgb(0.84, 0.34, 0.28));

        let xp = RoleXp {
            hunter: 12.0,
            architect: 3.0,
            ritualist: 0.0,
            warrior: 1.0,
        };
        let line = cat_skills_line(&xp);
        assert!(line.contains("hunt 12"));
        assert!(line.contains("build 3"));
    }
}
