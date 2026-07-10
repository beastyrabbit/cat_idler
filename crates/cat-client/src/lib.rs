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

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::sprite::{Anchor, BorderRect, SliceScaleMode, TextureSlicer};
use cat_protocol::{
    BuildingSnapshot, BuildingType, CarryingKind, CatActivity, CatSnapshot, ClientAction,
    ColonySnapshot, FootprintSize, GateSide, JobKind, OfficerRole, RaiderStatus, ResourceAmounts,
    ResourceKind, Specialization, StockLedgerSnapshot, StockpileSnapshot, TilePoint, WorldSnapshot,
    ZoneKind,
};
use cat_sim::terrain_gen::{
    BiomeRole, DecorationRole, TerrainTile, WORLD_TERRAIN_OPTIONS, generate_terrain_chunk,
};
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

// Flat ground layers (terrain + ground markings) sit below the y-sorted world
// sprites; all strictly below the camera at Z=1000 so nothing is clipped.
const Z_TERRAIN: f32 = 0.0;
const Z_ZONE: f32 = 2.0;

// Standing world sprites (buildings, walls, trees, stockpile piles, cats,
// raiders) share ONE y-sorted depth band: a sprite lower on the map (more
// negative world y) draws in front of one higher up — the whole 2.5D trick.
const Z_YSORT_BASE: f32 = 300.0;
const Z_YSORT_SCALE: f32 = 0.01;

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

/// The bottom-anchored base position (front-edge centre) and pixel size of a
/// building spanning its footprint (anchored at its NW-corner tile). `aspect` is
/// the sprite's native width/height; height follows it so the art isn't stretched.
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

/// The shrine reservoir's stockpile id — always present, de-emphasized in render.
const SHRINE_STOCKPILE_ID: &str = "stockpile-shrine";

/// Whether the officers panel is currently shown (toggled with the `O` key).
#[derive(Resource)]
struct OfficersUi {
    visible: bool,
}

impl Default for OfficersUi {
    fn default() -> Self {
        Self { visible: true }
    }
}

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
    rocky: Handle<Image>,
    highland: Handle<Image>,
    water: Handle<Image>,
    water_edge: Handle<Image>,
    tree_oak: Handle<Image>,
    tree_pine: Handle<Image>,
}

impl TerrainArt {
    fn load(assets: &AssetServer) -> Self {
        Self {
            grass: assets.load("public/images/game/terrain/grass.png"),
            grass_var: assets.load("public/images/game/terrain/grass_var.png"),
            rocky: assets.load("public/images/game/terrain/rocky.png"),
            highland: assets.load("public/images/game/terrain/highland.png"),
            water: assets.load("public/images/game/terrain/water.png"),
            water_edge: assets.load("public/images/game/terrain/water_edge.png"),
            tree_oak: assets.load("public/images/game/nature/tree_oak.png"),
            tree_pine: assets.load("public/images/game/nature/tree_pine.png"),
        }
    }

    fn ground(&self, texture: GroundTexture) -> Handle<Image> {
        match texture {
            GroundTexture::Grass => self.grass.clone(),
            GroundTexture::GrassVar => self.grass_var.clone(),
            GroundTexture::Rocky => self.rocky.clone(),
            GroundTexture::Highland => self.highland.clone(),
        }
    }
}

/// Ground texture chosen for a (non-water) tile from its biome.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum GroundTexture {
    Grass,
    GrassVar,
    Rocky,
    Highland,
}

/// Pixel-art building sprite handles, loaded once at startup.
#[derive(Resource, Clone)]
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
        }
    }

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
        }
    }
}

/// The building sprite a [`BuildingType`] renders as. Sprites `mill`, `clothier`,
/// `monument`, `tent`, `town_hall` are reserved for future P12.4 building types.
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
}

impl InfraArt {
    fn load(assets: &AssetServer) -> Self {
        Self {
            palisade: assets.load("public/images/game/infra/palisade.png"),
            gate: assets.load("public/images/game/infra/gate_open.png"),
        }
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
/// Marker for the building-inspector panel node (middle-click a building).
#[derive(Component)]
struct BuildingInspectorPanel;
/// Marker for the building-inspector text.
#[derive(Component)]
struct BuildingInspectorText;
/// Marker for a building marker sprite.
#[derive(Component)]
struct BuildingSprite;
/// Marker for a building world-space text label.
#[derive(Component)]
struct BuildingLabel;
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
/// Marker for the HUD dashboard text.
#[derive(Component)]
struct HudText;
/// Marker for the event-log text.
#[derive(Component)]
struct EventLogText;

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

/// Query filter for the per-tick redraw of building marker + label entities.
type BuildingEntities = Or<(With<BuildingSprite>, With<BuildingLabel>)>;
/// Query filter for the per-tick redraw of stockpile visuals + highlight.
type StockpileEntities = Or<(With<StockpileVis>, With<StockpileHighlight>)>;
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
                    render_buildings,
                    render_walls,
                    render_zones,
                    render_stockpiles,
                    sync_cats,
                    sync_raiders,
                    move_bodies,
                    follow_overlays,
                    animate_sprites,
                ),
                // input, tools + HUD
                (
                    camera_controls,
                    select_cat,
                    select_building,
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
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.insert_resource(TerrainArt::load(&asset_server));
    commands.insert_resource(BuildingArt::load(&asset_server));
    commands.insert_resource(PropArt::load(&asset_server));
    commands.insert_resource(InfraArt::load(&asset_server));
    commands.insert_resource(SpriteSheets::load(&asset_server, &mut atlas_layouts));
    let ui = UiArt::load(&asset_server);
    commands.insert_resource(ui.clone());

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
                row_gap: Val::Px(6.0),
                ..default()
            },
            sliced_image(ui.panel.clone(), PANEL_BORDER),
        ))
        .with_children(|panel| {
            panel.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(52.0),
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
            panel.spawn((
                Text::new("connecting…"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(PARCHMENT_INK),
                HudText,
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

    // Cat inspector (top-right), hidden until a cat is selected. Includes a row
    // of "Appoint <role>" buttons that make the selected cat that officer.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(8.0),
                top: Val::Px(8.0),
                width: Val::Px(256.0),
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
            top: Val::Px(330.0),
            width: Val::Px(256.0),
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
                top: Val::Px(430.0),
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
fn connect_ws(world: &mut World) {
    let url =
        std::env::var("CAT_SERVER_URL").unwrap_or_else(|_| "ws://127.0.0.1:8787/ws".to_string());
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
    // River/water coordinates, so shore tiles (a non-water orthogonal
    // neighbour) can use the water_edge variant.
    let water: HashSet<(i32, i32)> = tiles
        .iter()
        .filter(|t| t.river.is_some())
        .map(|t| (t.x, t.y))
        .collect();

    for tile in &tiles {
        let p = grid_to_world(tile.x, tile.y);
        let ground = if tile.river.is_some() {
            if is_shore(tile.x, tile.y, &water) {
                art.water_edge.clone()
            } else {
                art.water.clone()
            }
        } else {
            art.ground(ground_texture(tile))
        };
        commands.spawn((
            Sprite {
                image: ground,
                custom_size: Some(Vec2::splat(TILE)),
                ..default()
            },
            Transform::from_xyz(p.x, p.y, Z_TERRAIN),
        ));

        if let Some(DecorationRole::Tree { species }) = tile.decoration {
            let tree = if tree_is_oak(species) {
                art.tree_oak.clone()
            } else {
                art.tree_pine.clone()
            };
            // 16×32 sprite, trunk anchored to the tile centre so it stands on
            // the ground; y-sorted with the rest of the world by its base.
            let base_y = p.y - TILE * 0.5;
            commands.spawn((
                Sprite {
                    image: tree,
                    custom_size: Some(Vec2::new(TILE, TILE * 2.0)),
                    ..default()
                },
                Anchor::BOTTOM_CENTER,
                Transform::from_xyz(p.x, base_y, ysort_z(base_y)),
            ));
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

/// Ground texture for a non-water tile: rocky/highland by biome, otherwise
/// grass — with a deterministic `grass_var` sprinkle on grassland.
fn ground_texture(tile: &TerrainTile) -> GroundTexture {
    match tile.biome {
        BiomeRole::Rocky => GroundTexture::Rocky,
        BiomeRole::Highland => GroundTexture::Highland,
        BiomeRole::Grassland if (tile.x + tile.y).rem_euclid(5) == 0 => GroundTexture::GrassVar,
        BiomeRole::Lowland | BiomeRole::Grassland | BiomeRole::Forest => GroundTexture::Grass,
    }
}

/// Oak for even species, pine for odd.
fn tree_is_oak(species: i32) -> bool {
    species.rem_euclid(2) == 0
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
    let (x0, y0, x1, y1) = (
        VILLAGE_ANCHOR.x - WINDOW_RADIUS,
        VILLAGE_ANCHOR.y - WINDOW_RADIUS,
        VILLAGE_ANCHOR.x + WINDOW_RADIUS,
        VILLAGE_ANCHOR.y + WINDOW_RADIUS,
    );
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
        // Walls render as infra/palisade, not a point marker — skip.
        let Some(texture) = building_texture(building.building_type) else {
            continue;
        };
        // Span the building's footprint (anchored NW), keeping the sprite aspect;
        // bottom-anchored on the footprint's front edge, y-sorted with the world.
        let (base, size) = footprint_sprite(
            building.world_position,
            building.footprint,
            building_aspect(texture),
        );
        let z = ysort_z(base.y);
        commands.spawn((
            Sprite {
                image: art.handle(texture),
                custom_size: Some(size),
                ..default()
            },
            Anchor::BOTTOM_CENTER,
            Transform::from_xyz(base.x, base.y, z),
            BuildingSprite,
        ));
        // Small label centred just under the footprint's front edge.
        commands.spawn((
            Text2d::new(building_label(building.building_type)),
            TextFont {
                font_size: FontSize::Px(8.0),
                ..default()
            },
            TextColor(Color::srgba(1.0, 0.97, 0.86, 0.90)),
            Transform::from_xyz(base.x, base.y - TILE * 0.4, z + 0.2),
            BuildingLabel,
        ));
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
fn render_stockpiles(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    selection: Res<StockpileSelection>,
    art: Option<Res<PropArt>>,
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
/// building again to deselect.
fn select_building(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    ui: Query<&Interaction, With<Button>>,
    latest: Res<LatestSnapshot>,
    mut selection: ResMut<BuildingSelection>,
) {
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
    mut panel: Query<&mut Node, With<InspectorPanel>>,
    mut text: Query<&mut Text, With<InspectorText>>,
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
        }
        None => {
            node.display = Display::None;
            if selection.selected.is_some() {
                selection.selected = None;
            }
        }
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
        projection.scale = (projection.scale * if ev.y > 0.0 { 0.9 } else { 1.1 }).clamp(0.3, 3.0);
    }
}

fn update_hud(latest: Res<LatestSnapshot>, mut hud: Query<&mut Text, With<HudText>>) {
    if !latest.is_changed() {
        return;
    }
    let Ok(mut text) = hud.single_mut() else {
        return;
    };
    let Some(world) = latest.0.as_ref() else {
        text.0 = "connecting…".to_string();
        return;
    };
    let Some(colony) = world.colonies.first() else {
        text.0 = format!(
            "Idle Cat Forest\nonline: {}\nNo colony yet - press Found village.",
            world.online_count
        );
        return;
    };
    text.0 = dashboard_text(colony, world.online_count);
}

fn dashboard_text(colony: &ColonySnapshot, online: u32) -> String {
    let r = &colony.resources;
    let cap = &colony.storage.capacities;
    let leader = colony
        .leader
        .as_ref()
        .map_or_else(|| "none".to_string(), |l| l.name.clone());
    let active_jobs = colony
        .jobs
        .iter()
        .filter(|j| matches!(j.status, cat_protocol::JobStatus::Active))
        .count();
    format!(
        "online {online}\n\
         Colony: {name}  [{status:?}]\n\
         Leader: {leader}\n\
         Pop {pop}/{cap_house}  Village Lv {lvl}\n\
         Threat: {threat:?} ({pressure:.0})  warriors {warriors}\n\
         \n\
         Food      {food:>5.0} / {food_cap:.0}\n\
         Water     {water:>5.0} / {water_cap:.0}\n\
         Materials {mat:>5.0} / {mat_cap:.0}\n\
         Refined   {ref_:>5.0} / {ref_cap:.0}\n\
         Herbs     {herbs:>5.0} / {herbs_cap:.0}\n\
         Weapons   {weap:>5.0}   Armor {armor:.0}\n\
         Blessings {bless:>5.1}\n\
         \n\
         Active jobs: {active_jobs}   Total jobs: {jobs}{ledger}",
        name = colony.name,
        status = colony.status,
        pop = colony.housing.population,
        cap_house = colony.housing.capacity,
        lvl = colony.housing.village_level,
        threat = colony.threat.band,
        pressure = colony.threat.pressure,
        warriors = colony.threat.warriors,
        food = r.food,
        food_cap = cap.food,
        water = r.water,
        water_cap = cap.water,
        mat = r.materials,
        mat_cap = cap.materials,
        ref_ = r.refined,
        ref_cap = cap.refined,
        herbs = r.herbs,
        herbs_cap = cap.herbs,
        weap = r.weapons,
        armor = r.armor,
        bless = r.blessings,
        jobs = colony.jobs.len(),
        ledger = colony
            .stock_ledger
            .as_ref()
            .map_or_else(String::new, |l| format!("\n\n{}", ledger_hud_text(l))),
    )
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
        .take(6)
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
/// construction, and the cats assigned to it.
fn building_inspector_text(building: &BuildingSnapshot, colony: &ColonySnapshot) -> String {
    let status = if building.construction_progress >= 100.0 {
        "operational".to_string()
    } else {
        format!("under construction {:.0}%", building.construction_progress)
    };
    let workers: Vec<&str> = colony
        .cats
        .iter()
        .filter(|c| c.assigned_building_id.as_deref() == Some(building.id.as_str()))
        .map(|c| c.name.as_str())
        .collect();
    let workers_line = if workers.is_empty() {
        "none".to_string()
    } else {
        workers.join(", ")
    };
    format!(
        "{name}  Lv {lvl}\n\
         {status}\n\
         at {x},{y}\n\
         workers: {workers}",
        name = building_label(building.building_type),
        lvl = building.level,
        x = building.world_position.x,
        y = building.world_position.y,
        workers = workers_line,
    )
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
fn inspector_text(cat: &CatSnapshot) -> String {
    let dest = cat
        .destination
        .map_or_else(|| "none".to_string(), |d| format!("{},{}", d.x, d.y));
    let carrying = cat.carrying.as_ref().map_or_else(
        || "none".to_string(),
        |c| format!("{:?} x{:.0}", c.kind, c.amount),
    );
    let n = &cat.needs;
    format!(
        "{name}\n\
         {spec} - {stage} ({age:.0}h)\n\
         at {x},{y} - {activity}\n\
         dest {dest}\n\
         carrying {carrying}\n\
         \n\
         hunger {hunger:>3.0}   thirst {thirst:>3.0}\n\
         rest   {rest:>3.0}   health {health:>3.0}",
        name = cat.name,
        spec = specialization_name(cat.specialization),
        stage = life_stage(cat.age_hours),
        age = cat.age_hours,
        x = cat.position.x,
        y = cat.position.y,
        activity = activity_name(cat.activity),
        hunger = n.hunger,
        thirst = n.thirst,
        rest = n.rest,
        health = n.health,
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
        BuildingType::Walls => return None,
    })
}

/// Native width/height aspect of a building sprite (48x48 square = 1.0; market
/// 48x32 wide = 1.5; well 16x32 tall = 0.5). Used to size a footprint-spanning
/// sprite without stretching the art.
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
    use cat_protocol::WorldSnapshot;

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

    fn tile_with(biome: BiomeRole, x: i32, y: i32) -> TerrainTile {
        TerrainTile {
            x,
            y,
            elevation: 0.0,
            moisture: 0.0,
            height: 1,
            biome,
            climate_biome: cat_sim::climate::Biome::Plains,
            terrain: cat_sim::terrain_gen::TerrainRole::Flat,
            river: None,
            stairs: None,
            decoration: None,
        }
    }

    #[test]
    fn ground_texture_maps_biomes_to_sprites() {
        assert_eq!(
            ground_texture(&tile_with(BiomeRole::Rocky, 0, 0)),
            GroundTexture::Rocky
        );
        assert_eq!(
            ground_texture(&tile_with(BiomeRole::Highland, 0, 0)),
            GroundTexture::Highland
        );
        assert_eq!(
            ground_texture(&tile_with(BiomeRole::Lowland, 1, 1)),
            GroundTexture::Grass
        );
        assert_eq!(
            ground_texture(&tile_with(BiomeRole::Forest, 3, 2)),
            GroundTexture::Grass
        );
        // Grassland gets the variant sprite on the deterministic subset only.
        assert_eq!(
            ground_texture(&tile_with(BiomeRole::Grassland, 2, 3)),
            GroundTexture::GrassVar
        );
        assert_eq!(
            ground_texture(&tile_with(BiomeRole::Grassland, 2, 4)),
            GroundTexture::Grass
        );
    }

    #[test]
    fn tree_species_pick_oak_or_pine_and_shore_detection() {
        assert!(tree_is_oak(0));
        assert!(tree_is_oak(2));
        assert!(!tree_is_oak(1));
        assert!(!tree_is_oak(3));

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
    fn all_building_types_have_labels() {
        for building in [
            BuildingType::Shrine,
            BuildingType::Den,
            BuildingType::Workshop,
            BuildingType::Field,
            BuildingType::Smithy,
            BuildingType::Barracks,
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
        assert!(dashboard_text(&snap.colonies[0], 2).contains("Colony: A"));
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
                    {"id":"b1","type":"workshop","level":2,"constructionProgress":100.0,"worldPosition":{"x":7,"y":6},"position":{"x":1,"y":0}},
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
        assert!(ws.contains("Moss")); // assigned worker
        let den_text = building_inspector_text(den, colony);
        assert!(den_text.contains("under construction 40%"));
        assert!(den_text.contains("workers: none"));
    }
}
