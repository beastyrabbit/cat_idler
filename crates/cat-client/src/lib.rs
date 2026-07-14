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
//! - footprint buildings/workshops,
//! - a stockpile indicator near the shrine,
//! - avoid/gather zones,
//! - a HUD dashboard + event log, and clickable manual-action buttons that
//!   round-trip [`cat_protocol::ClientAction`] over the socket.

use bevy::asset::{AssetMetaCheck, RenderAssetUsages};
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::sprite::Anchor;
use bevy::sprite::{BorderRect, SliceScaleMode, TextureSlicer};
use bevy::ui::{InteractionDisabled, RelativeCursorPosition, VisualBox, widget::NodeImageMode};
use bevy::window::{CursorIcon, CustomCursor, CustomCursorImage, PrimaryWindow};
use cat_protocol::{
    ActionResult, BuildingSnapshot, BuildingType, CarryingKind, CatActivity, CatHousingStatus,
    CatNeeds, CatSnapshot, ClientAction, ColonySnapshot, CropKind, EventSnapshot, FarmSnapshot,
    FarmStage, FootprintSize, GateSide, GatherSpotPurpose, ItemStackSnapshot, JobKind, Labor,
    OfficerRole, ProductionQueueEdit, QueueMoveDirection, RaiderStatus, ResourceAmounts,
    ResourceCapacities, ResourceKind, RoleXp, ScoutMission, ScoutResource, Specialization,
    StockLedgerSnapshot, StockpileSnapshot, TilePoint, TraderBuyOffer, TraderSellOffer,
    TraderVisitState, VillageKind, VillageScale, WorldSnapshot, ZoneKind,
};
use cat_sim::climate::{Biome, ResourceHint};
use cat_sim::terrain_gen::{
    DecorationRole, RockSize, TERRAIN_CHUNK_SIZE, TREE_FOOTPRINT_HEIGHT, TREE_FOOTPRINT_WIDTH,
    TerrainTile, WORLD_TERRAIN_OPTIONS, decoration_footprint, derive_biome_decoration,
    generate_terrain_chunk, tile_climate_biome,
};
use cat_sim::village_layout::VILLAGE_ANCHOR;
use cat_sim::world_gen::tile_to_chunk;
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
#[cfg(all(not(target_arch = "wasm32"), unix))]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
#[cfg(not(target_arch = "wasm32"))]
use std::{
    io::Write,
    path::{Path, PathBuf},
};

mod research_ui;
mod station_layout;
use research_ui::UpgradeTreeUi;
use station_layout::{
    BuildingVisual, PropPlacement, ResidentialFacade, StationFloor, StationLayout, StationProp,
    building_visual,
};

/// Side length (world units) of one flat tile. Shrunk to ~1/3 of the original 28
/// so buildings read at a sensible size and more of the world fits on screen;
/// everything (terrain, footprint buildings, cats, trees, walls) scales off it.
const TILE: f32 = 10.0;
/// Chunks kept immediately around the camera. Five chunks across cover the
/// default 1280x800 view while keeping the terrain entity count bounded.
const TERRAIN_CHUNK_RADIUS: i32 = 2;
/// A one-chunk cache margin avoids unloading/reloading a full strip when the
/// camera briefly crosses a chunk edge.
const TERRAIN_RETAIN_RADIUS: i32 = TERRAIN_CHUNK_RADIUS + 1;
/// Reconnect attempts use exponential backoff, capped so a long-running idle
/// client recovers without hammering a missing server.
const MAX_RECONNECT_DELAY_SECS: f32 = 30.0;
const CLIENT_ALERT_CAP: usize = 8;
#[cfg(target_arch = "wasm32")]
const SESSION_STORAGE_KEY: &str = "idle-cat-forest/session/v1";
/// Starting (and R-reset) camera zoom, tuned to frame the village at the small
/// tile — a little zoomed in since there's now more world per screen.
const DEFAULT_ZOOM: f32 = 0.25;
/// Fixed Chebyshev radius of the founding settlement's permanent wall core.
/// This mirrors `cat_sim`'s `VILLAGE_START_RADIUS`; `snapshot.village_radius`
/// describes dynamic building-ring framing and can be smaller than this core.
const VILLAGE_INTERIOR_RADIUS: u32 = 6;
/// The permanent wall core already fits comfortably at DEFAULT_ZOOM. Larger
/// villages auto-fit once, until the player deliberately pans or zooms.
const STARTER_CAMERA_RADIUS: u32 = VILLAGE_INTERIOR_RADIUS;
/// Top command strip + bottom toolbar space kept clear by mature-village fitting.
const CAMERA_VERTICAL_UI_RESERVE: f32 = 160.0;
/// The mature view centres in the unobscured map rectangle: 332px of persistent
/// HUD on the left versus 195px of minimap on the right.
const CAMERA_SAFE_CENTER_OFFSET_X: f32 = (332.0 - 195.0) / 2.0;
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
const OFFICERS_SHORTCUT: KeyCode = KeyCode::KeyO;
const ORDERS_SHORTCUT: KeyCode = KeyCode::KeyP;
const CAMERA_RESET_SHORTCUT: KeyCode = KeyCode::KeyR;

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredSession {
    session_id: String,
    sig: String,
    selected_colony_id: Option<String>,
}

fn stored_session_json(session: &Session, selection: &VillageSelection) -> Option<String> {
    (!session.session_id.is_empty() && !session.sig.is_empty()).then(|| {
        serde_json::json!({
            "sessionId": session.session_id,
            "sig": session.sig,
            "selectedColonyId": selection.selected_id,
        })
        .to_string()
    })
}

fn parse_stored_session(raw: &str) -> Option<StoredSession> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let session_id = value.get("sessionId")?.as_str()?.trim();
    let sig = value.get("sig")?.as_str()?.trim();
    if session_id.is_empty() || sig.is_empty() {
        return None;
    }
    let selected_colony_id = value
        .get("selectedColonyId")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty() && id.len() <= 128 && !id.chars().any(char::is_control))
        .map(str::to_owned);
    Some(StoredSession {
        session_id: session_id.to_owned(),
        sig: sig.to_owned(),
        selected_colony_id,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn native_session_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("CAT_CLIENT_SESSION_PATH").filter(|path| !path.is_empty())
    {
        return Some(PathBuf::from(path));
    }
    std::env::var_os("XDG_CONFIG_HOME")
        .filter(|root| !root.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|home| !home.is_empty())
                .map(|home| PathBuf::from(home).join(".config"))
        })
        .map(|root| root.join("idle-cat-forest/session.json"))
}

#[cfg(not(target_arch = "wasm32"))]
fn load_session_from_path(path: &Path) -> Option<StoredSession> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| parse_stored_session(&raw))
}

#[cfg(not(target_arch = "wasm32"))]
fn native_session_temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("session.json");
    path.with_file_name(format!(".{file_name}.tmp-{}", std::process::id()))
}

#[cfg(not(target_arch = "wasm32"))]
fn save_session_to_path(path: &Path, raw: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let temp_path = native_session_temp_path(path);
    let mut options = std::fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    options.mode(0o600);
    let result = (|| {
        let mut file = options.open(&temp_path).map_err(|err| err.to_string())?;
        #[cfg(unix)]
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|err| err.to_string())?;
        file.write_all(raw.as_bytes())
            .map_err(|err| err.to_string())?;
        file.sync_all().map_err(|err| err.to_string())?;
        std::fs::rename(&temp_path, path).map_err(|err| err.to_string())?;
        #[cfg(unix)]
        if let Some(parent) = path.parent() {
            std::fs::File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temp_path);
    }
    result
}

fn load_persisted_session(mut session: ResMut<Session>, mut selection: ResMut<VillageSelection>) {
    #[cfg(target_arch = "wasm32")]
    let stored = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(SESSION_STORAGE_KEY).ok().flatten())
        .and_then(|raw| parse_stored_session(&raw));
    #[cfg(not(target_arch = "wasm32"))]
    let stored = native_session_path().and_then(|path| load_session_from_path(&path));

    restore_stored_session(stored, &mut session, &mut selection);
}

fn restore_stored_session(
    stored: Option<StoredSession>,
    session: &mut Session,
    selection: &mut VillageSelection,
) {
    let Some(stored) = stored else {
        return;
    };
    session.session_id = stored.session_id;
    session.sig = stored.sig;
    selection.selected_id = stored.selected_colony_id;
    selection.join_required = selection.selected_id.is_some();
}

fn persist_session(session: &Session, selection: &VillageSelection) -> Result<(), String> {
    let Some(raw) = stored_session_json(session, selection) else {
        return Err("refusing to persist an incomplete session".to_owned());
    };
    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window()
            .ok_or_else(|| "browser window unavailable".to_owned())?
            .local_storage()
            .map_err(|_| "browser storage unavailable".to_owned())?
            .ok_or_else(|| "browser storage unavailable".to_owned())?;
        storage
            .set_item(SESSION_STORAGE_KEY, &raw)
            .map_err(|_| "failed to persist browser session".to_owned())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path =
            native_session_path().ok_or_else(|| "config directory unavailable".to_owned())?;
        save_session_to_path(&path, &raw)
    }
}

/// The village this client is looking at and sending colony-scoped actions to.
///
/// The id deliberately lives outside [`LatestSnapshot`]: the server may reorder
/// its shared-world snapshot, and a reconnect creates a fresh socket whose
/// server-side selection starts at the founding colony. `join_required` records
/// that the persisted choice must be restored once Presence yields a new signed
/// session.
#[derive(Resource, Default, Debug)]
struct VillageSelection {
    selected_id: Option<String>,
    join_required: bool,
}

/// Current transport lifecycle. A failed connection waits for a capped
/// exponential delay, then tries again for as long as the idle client runs.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
enum ConnectionPhase {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    WaitingToRetry,
}

#[derive(Resource, Default)]
struct ConnectionState {
    phase: ConnectionPhase,
    retry_attempt: u32,
    retry_remaining_secs: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum FeedbackLevel {
    #[default]
    Info,
    Error,
}

/// Prominent connection/action feedback shown above the world.
#[derive(Resource, Default)]
struct ClientFeedback {
    message: Option<String>,
    level: FeedbackLevel,
    remaining_secs: f32,
}

/// Client-side dispatches that must survive the next snapshot update (notably
/// rejected actions and transport loss).
#[derive(Resource, Default)]
struct ClientAlerts(VecDeque<String>);

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
    selected_farm: Option<String>,
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

/// Legacy shrine store id plus the current finite seeded village storehouse id.
const SHRINE_STOCKPILE_ID: &str = "stockpile-shrine";
const GENERAL_STOREHOUSE_ID: &str = "stockpile-storehouse";

fn is_seeded_store(pile_id: &str) -> bool {
    matches!(pile_id, SHRINE_STOCKPILE_ID | GENERAL_STOREHOUSE_ID)
}

/// Whether the officers panel is shown (toggled by `O`). Hidden by default
/// (`visible` = false) so it can't pile up on the HUD + event-log in the left
/// column on a normal-height window; appointment is also in the cat inspector.
#[derive(Resource, Default)]
struct OfficersUi {
    visible: bool,
}

/// Responsive manual orders sheet. Vacant offices leave these controls as the
/// player's authoritative path for the same work categories.
#[derive(Resource, Default)]
struct OrdersUi {
    visible: bool,
    planned_building: usize,
}

#[derive(Resource, Default)]
struct GovernanceUi {
    candidate_index: usize,
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

/// The appointable officer roles, in display order.
const ALL_OFFICER_ROLES: [OfficerRole; 7] = [
    OfficerRole::Steward,
    OfficerRole::Accountant,
    OfficerRole::Forester,
    OfficerRole::Farmer,
    OfficerRole::Captain,
    OfficerRole::Loremaster,
    OfficerRole::ClothLeader,
];

fn officer_role_name(role: OfficerRole) -> &'static str {
    match role {
        OfficerRole::Steward => "Steward",
        OfficerRole::Accountant => "Accountant",
        OfficerRole::Forester => "Forester",
        OfficerRole::Farmer => "Farmer",
        OfficerRole::Captain => "Captain",
        OfficerRole::Loremaster => "Loremaster",
        OfficerRole::ClothLeader => "Cloth Leader",
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
#[derive(Resource)]
struct Tools {
    mode: ToolMode,
    /// `(start_tile, current_tile)` while dragging a zone rectangle.
    drag: Option<((i32, i32), (i32, i32))>,
    accept: AcceptChoice,
    crop: CropKind,
    gather_kind: ResourceKind,
}

impl Default for Tools {
    fn default() -> Self {
        Self {
            mode: ToolMode::Inspect,
            drag: None,
            accept: AcceptChoice::General,
            crop: CropKind::Grain,
            gather_kind: ResourceKind::Materials,
        }
    }
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
    Farm,
    GatherSpot,
    FishingSpot,
    Road,
    Building,
}

/// What a click-drag paints — a steering zone or a stockpile designation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PaintKind {
    Avoid,
    Gather,
    Stockpile,
    Farm,
    GatherSpot,
    FishingSpot,
    Road,
}

impl ToolMode {
    fn label(self) -> &'static str {
        match self {
            Self::Inspect => "Inspect",
            Self::AvoidZone => "Avoid zone",
            Self::GatherZone => "Gather zone",
            Self::Stockpile => "Stockpile",
            Self::Farm => "Farm",
            Self::GatherSpot => "Gather spot",
            Self::FishingSpot => "Fishing spot",
            Self::Road => "Road",
            Self::Building => "Building",
        }
    }

    /// What this mode paints on drag, if anything.
    fn paint_kind(self) -> Option<PaintKind> {
        match self {
            Self::Inspect => None,
            Self::AvoidZone => Some(PaintKind::Avoid),
            Self::GatherZone => Some(PaintKind::Gather),
            Self::Stockpile => Some(PaintKind::Stockpile),
            Self::Farm => Some(PaintKind::Farm),
            Self::GatherSpot => Some(PaintKind::GatherSpot),
            Self::FishingSpot => Some(PaintKind::FishingSpot),
            Self::Road => Some(PaintKind::Road),
            Self::Building => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct ChunkKey {
    x: i32,
    y: i32,
}

/// Camera-relative terrain cache. Terrain is deterministic per seed, but the
/// loaded chunk set streams as the camera crosses the unbounded world.
#[derive(Resource, Default)]
struct WorldRender {
    world_seed: Option<i64>,
    loaded_chunks: HashSet<ChunkKey>,
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

/// Pixel-art building and open-station handles, loaded once at startup.
#[derive(Resource, Clone, Default)]
struct BuildingArt {
    den: Handle<Image>,
    well: Handle<Image>,
    floor_wood: Handle<Image>,
    floor_stone: Handle<Image>,
    floor_soil: Handle<Image>,
    altar: Handle<Image>,
    barrel: Handle<Image>,
    bed: Handle<Image>,
    bed_green: Handle<Image>,
    bed_orange: Handle<Image>,
    bookcase: Handle<Image>,
    brazier: Handle<Image>,
    candelabra: Handle<Image>,
    crate_box: Handle<Image>,
    crop_flowering: Handle<Image>,
    crop_growing: Handle<Image>,
    crop_mature: Handle<Image>,
    crop_sprout: Handle<Image>,
    display_table: Handle<Image>,
    forge_fire: Handle<Image>,
    haystack: Handle<Image>,
    log_pile: Handle<Image>,
    map_table: Handle<Image>,
    metal_basin: Handle<Image>,
    ore_pile: Handle<Image>,
    reliquary_gold: Handle<Image>,
    sack: Handle<Image>,
    scarecrow: Handle<Image>,
    stone_pile: Handle<Image>,
    stove: Handle<Image>,
    stool: Handle<Image>,
    scroll: Handle<Image>,
    sword_block: Handle<Image>,
    weapon_stand: Handle<Image>,
    workbench: Handle<Image>,
}

impl BuildingArt {
    fn load(assets: &AssetServer) -> Self {
        Self {
            den: assets.load("public/images/game/buildings/den.png"),
            well: assets.load("public/images/game/props/well.png"),
            floor_wood: assets.load("public/images/game/interior/floor_wood.png"),
            floor_stone: assets.load("public/images/game/interior/floor_stone.png"),
            floor_soil: assets.load("public/images/game/farm/soil.png"),
            altar: assets.load("public/images/game/interior/altar.png"),
            barrel: assets.load("public/images/game/props/barrel.png"),
            bed: assets.load("public/images/game/interior/bed.png"),
            bed_green: assets.load("public/images/game/interior/bed-green.png"),
            bed_orange: assets.load("public/images/game/interior/bed-orange.png"),
            bookcase: assets.load("public/images/game/interior/bookcase.png"),
            brazier: assets.load("public/images/game/interior/brazier.png"),
            candelabra: assets.load("public/images/game/interior/candelabra.png"),
            crate_box: assets.load("public/images/game/props/crate.png"),
            crop_flowering: assets.load("public/images/game/farm/crop_flowering.png"),
            crop_growing: assets.load("public/images/game/farm/crop_growing.png"),
            crop_mature: assets.load("public/images/game/farm/crop_mature.png"),
            crop_sprout: assets.load("public/images/game/farm/crop_sprout.png"),
            display_table: assets.load("public/images/game/interior/display-table.png"),
            forge_fire: assets.load("public/images/game/interior/forge-fire.png"),
            haystack: assets.load("public/images/game/props/haystack.png"),
            log_pile: assets.load("public/images/game/props/log_pile.png"),
            map_table: assets.load("public/images/game/interior/map-table.png"),
            metal_basin: assets.load("public/images/game/interior/metal-basin.png"),
            ore_pile: assets.load("public/images/game/props/ore_pile.png"),
            reliquary_gold: assets.load("public/images/game/interior/reliquary-gold.png"),
            sack: assets.load("public/images/game/props/sack.png"),
            scarecrow: assets.load("public/images/game/farm/scarecrow.png"),
            stone_pile: assets.load("public/images/game/props/stone_pile.png"),
            stove: assets.load("public/images/game/interior/stove.png"),
            stool: assets.load("public/images/game/interior/stool-square.png"),
            scroll: assets.load("public/images/game/interior/scroll.png"),
            sword_block: assets.load("public/images/game/interior/sword-block.png"),
            weapon_stand: assets.load("public/images/game/interior/weapon-stand.png"),
            workbench: assets.load("public/images/game/interior/workbench.png"),
        }
    }

    fn floor(&self, kind: StationFloor) -> Handle<Image> {
        match kind {
            StationFloor::Wood => self.floor_wood.clone(),
            StationFloor::Stone => self.floor_stone.clone(),
            StationFloor::Soil => self.floor_soil.clone(),
        }
    }

    fn facade(&self, facade: ResidentialFacade) -> Handle<Image> {
        match facade {
            ResidentialFacade::Cottage => self.den.clone(),
        }
    }

    fn prop(&self, prop: StationProp) -> Handle<Image> {
        match prop {
            StationProp::Altar => self.altar.clone(),
            StationProp::Barrel => self.barrel.clone(),
            StationProp::Bed => self.bed.clone(),
            StationProp::BedGreen => self.bed_green.clone(),
            StationProp::BedOrange => self.bed_orange.clone(),
            StationProp::Bookcase => self.bookcase.clone(),
            StationProp::Brazier => self.brazier.clone(),
            StationProp::Candelabra => self.candelabra.clone(),
            StationProp::Crate => self.crate_box.clone(),
            StationProp::CropFlowering => self.crop_flowering.clone(),
            StationProp::CropGrowing => self.crop_growing.clone(),
            StationProp::CropMature => self.crop_mature.clone(),
            StationProp::CropSprout => self.crop_sprout.clone(),
            StationProp::DisplayTable => self.display_table.clone(),
            StationProp::ForgeFire => self.forge_fire.clone(),
            StationProp::Haystack => self.haystack.clone(),
            StationProp::LogPile => self.log_pile.clone(),
            StationProp::MapTable => self.map_table.clone(),
            StationProp::MetalBasin => self.metal_basin.clone(),
            StationProp::OrePile => self.ore_pile.clone(),
            StationProp::ReliquaryGold => self.reliquary_gold.clone(),
            StationProp::Sack => self.sack.clone(),
            StationProp::Scarecrow => self.scarecrow.clone(),
            StationProp::StonePile => self.stone_pile.clone(),
            StationProp::Stove => self.stove.clone(),
            StationProp::Stool => self.stool.clone(),
            StationProp::Scroll => self.scroll.clone(),
            StationProp::SwordBlock => self.sword_block.clone(),
            StationProp::WeaponStand => self.weapon_stand.clone(),
            StationProp::Well => self.well.clone(),
            StationProp::Workbench => self.workbench.clone(),
        }
    }
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

// ============================================================================
// UI kit — one cohesive visual language for every panel, button and label.
//
// The UI grew panel-by-panel with ad-hoc sizes, spacing and frames, so it read
// like a debug overlay. This kit uses the maintained Adventure pack as an
// actual render system: sliced parchment/wood panels, image-backed buttons,
// framed resource glyphs, image progress bars and stateful cursors. The fixed
// type scale and 4px spacing grid keep that expressive skin product-normal and
// readable instead of turning every surface into decoration.
// ============================================================================

// -- Palette: parchment ink + restrained state colours ----------------------
/// Parchment fallback while the image asset is loading.
const UI_BG: Color = Color::srgba(0.93, 0.84, 0.65, 0.98);
/// Dark-walnut fallback behind panel headings.
const UI_HEADER: Color = Color::srgb(0.26, 0.15, 0.07);
/// Faint divider inside a panel body.
const UI_DIVIDER: Color = Color::srgba(0.35, 0.20, 0.09, 0.55);
/// Primary ink on parchment.
const UI_INK: Color = Color::srgb(0.16, 0.095, 0.045);
/// Cream ink reserved for dark title bars and resource wells.
const UI_TITLE_INK: Color = Color::srgb(0.98, 0.91, 0.74);
/// Secondary / de-emphasised text.
const UI_MUTED: Color = Color::srgb(0.39, 0.31, 0.22);
/// Brick-red accent used for values and active affordances.
const UI_ACCENT: Color = Color::srgb(0.56, 0.16, 0.10);
/// Good news (births, gains) — saturated so it pops in the dispatch feed.
const UI_POSITIVE: Color = Color::srgb(0.20, 0.47, 0.18);
/// Trouble (deaths, crises, raids) — saturated red-orange.
const UI_WARNING: Color = Color::srgb(0.69, 0.12, 0.08);
const UI_BUTTON_BROWN: Color = Color::srgb(0.43, 0.26, 0.12);
const UI_BUTTON_BROWN_HOVER: Color = Color::srgb(0.58, 0.36, 0.17);
const UI_BUTTON_RED: Color = Color::srgb(0.50, 0.16, 0.10);
const UI_BUTTON_GREY: Color = Color::srgb(0.34, 0.32, 0.29);

const PANEL_SLICE_PX: f32 = 18.0;
const BUTTON_SLICE_X_PX: f32 = 16.0;
const BUTTON_SLICE_Y_PX: f32 = 12.0;
const PROGRESS_SLICE_PX: f32 = 12.0;

// -- Type scale (integer px keeps the pixel font crisp) ---------------------
const FS_TITLE: f32 = 16.0;
const FS_SECTION: f32 = 13.0;
const FS_BODY: f32 = 12.0;
const FS_SMALL: f32 = 11.0;

// -- Spacing grid (4px base) ------------------------------------------------
const UI_PAD: f32 = 12.0;
const UI_GAP: f32 = 6.0;
const UI_GAP_TIGHT: f32 = 3.0;
const UI_RADIUS: f32 = 6.0;
const UI_BORDER_W: f32 = 2.5;
const UI_BTN_H: f32 = 30.0;
/// Three button lines (tools + two wrapped action lines), their row gaps,
/// panel padding/frame, and the bar's 10px screen margin at 1024px.
const NARROW_BOTTOM_BAR_FOOTPRINT: f32 = 3.0 * UI_BTN_H + 4.0 * UI_GAP + 2.0 * UI_BORDER_W + 10.0;
/// Bottom-corner overlays clear the whole narrow toolbar plus breathing room,
/// not only its first row of controls.
const BOTTOM_OVERLAY_CLEARANCE: f32 = NARROW_BOTTOM_BAR_FOOTPRINT + UI_GAP;
/// Eight two-column resource rows remain readable at this compact height and
/// leave room for Dispatches above the narrow wrapped toolbar.
const HUD_RESOURCE_PILL_HEIGHT: f32 = 20.0;

/// Tracked Adventure-pack textures. Runtime never reaches into the ignored
/// source bundle; these semantic copies are the stable client asset contract.
#[derive(Resource, Clone, Default)]
struct AdventureUiArt {
    panel: Handle<Image>,
    panel_dark: Handle<Image>,
    panel_ornate: Handle<Image>,
    button: Handle<Image>,
    button_active: Handle<Image>,
    button_disabled: Handle<Image>,
    progress_track: Handle<Image>,
    progress_good: Handle<Image>,
    progress_mid: Handle<Image>,
    progress_low: Handle<Image>,
    banner: Handle<Image>,
    icon_frame: Handle<Image>,
    minimap_ring: Handle<Image>,
    cursor_pointer: Handle<Image>,
    cursor_interact: Handle<Image>,
    cursor_pressed: Handle<Image>,
    cursor_target: Handle<Image>,
    cursor_disabled: Handle<Image>,
}

impl AdventureUiArt {
    fn load(assets: &AssetServer) -> Self {
        Self {
            panel: assets.load("public/images/game/ui/panel.png"),
            panel_dark: assets.load("public/images/game/ui/panel-dark.png"),
            panel_ornate: assets.load("public/images/game/ui/panel-ornate.png"),
            button: assets.load("public/images/game/ui/button.png"),
            button_active: assets.load("public/images/game/ui/button-active.png"),
            button_disabled: assets.load("public/images/game/ui/button-disabled.png"),
            progress_track: assets.load("public/images/game/ui/progress-track.png"),
            progress_good: assets.load("public/images/game/ui/progress-good.png"),
            progress_mid: assets.load("public/images/game/ui/progress-mid.png"),
            progress_low: assets.load("public/images/game/ui/progress-low.png"),
            banner: assets.load("public/images/game/ui/banner.png"),
            icon_frame: assets.load("public/images/game/ui/icon-frame.png"),
            minimap_ring: assets.load("public/images/game/ui/minimap-ring.png"),
            cursor_pointer: assets.load("public/images/game/ui/cursor/pointer.png"),
            cursor_interact: assets.load("public/images/game/ui/cursor/interact.png"),
            cursor_pressed: assets.load("public/images/game/ui/cursor/pressed.png"),
            cursor_target: assets.load("public/images/game/ui/cursor/target.png"),
            cursor_disabled: assets.load("public/images/game/ui/cursor/disabled.png"),
        }
    }
}

fn sliced_image(image: Handle<Image>, border: BorderRect, max_corner_scale: f32) -> ImageNode {
    ImageNode {
        image,
        image_mode: NodeImageMode::Sliced(TextureSlicer {
            border,
            center_scale_mode: SliceScaleMode::Stretch,
            sides_scale_mode: SliceScaleMode::Stretch,
            max_corner_scale,
        }),
        visual_box: VisualBox::BorderBox,
        ..default()
    }
}

fn panel_slicer() -> TextureSlicer {
    TextureSlicer {
        border: BorderRect::all(PANEL_SLICE_PX),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    }
}

fn button_slicer() -> TextureSlicer {
    TextureSlicer {
        border: BorderRect::axes(BUTTON_SLICE_X_PX, BUTTON_SLICE_Y_PX),
        center_scale_mode: SliceScaleMode::Stretch,
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum AdventurePanel {
    Paper,
    Dark,
    Ornate,
}

/// Assign image handles after spawn so common UI builders stay usable by
/// snapshot-driven child creation (not only by the startup system).
fn skin_adventure_panels(
    art: Res<AdventureUiArt>,
    mut panels: Query<(&AdventurePanel, &mut ImageNode), Added<AdventurePanel>>,
) {
    for (style, mut image) in &mut panels {
        image.image = match style {
            AdventurePanel::Paper => art.panel.clone(),
            AdventurePanel::Dark => art.panel_dark.clone(),
            AdventurePanel::Ornate => art.panel_ornate.clone(),
        };
        image.image_mode = NodeImageMode::Sliced(panel_slicer());
        image.visual_box = VisualBox::BorderBox;
    }
}

/// A text bundle at a kit size + colour (one font everywhere, via the default).
fn ui_text(s: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(s),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

/// The base Node of a panel: an absolutely-placed, bordered, clipped column of a
/// fixed width. Callers set `left`/`top` (etc.) via struct-update syntax, e.g.
/// `Node { left: Val::Px(10.0), top: Val::Px(52.0), ..ui_panel_node(w) }`, and
/// pair it with [`ui_panel_frame`]. The title bar spans edge-to-edge as the
/// first child; the body ([`ui_panel_body`]) carries the padding so text never
/// rides the frame.
fn ui_panel_node(width: Val) -> Node {
    Node {
        position_type: PositionType::Absolute,
        width,
        border: UiRect::all(Val::Px(UI_BORDER_W)),
        flex_direction: FlexDirection::Column,
        overflow: Overflow::clip(),
        ..default()
    }
}

/// The cat inspector must not inherit the generic panel's clipped overflow.
/// Bevy 0.19 can leak that dynamically-shown scissor into the world pass,
/// blacking out everything outside a narrow central strip.
fn cat_inspector_panel_node() -> Node {
    Node {
        right: Val::Px(10.0),
        top: Val::Px(60.0),
        overflow: Overflow::visible(),
        ..ui_panel_node(Val::Px(300.0))
    }
}

/// The visual layer of a panel: a real sliced Adventure parchment frame. The
/// solid fallback is visible only while the texture is loading.
fn ui_panel_frame() -> impl Bundle {
    (
        BackgroundColor(UI_BG),
        BorderColor::all(Color::NONE),
        ImageNode::default(),
        AdventurePanel::Paper,
    )
}

/// A panel title bar (first child of [`ui_panel`]): a sliced dark-walnut well
/// with cream lettering. It is a real image-backed surface, not a color strip.
fn ui_title_bar(title: &str) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            padding: UiRect::axes(Val::Px(UI_PAD), Val::Px(7.0)),
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(UI_HEADER),
        ImageNode::default(),
        AdventurePanel::Dark,
        children![ui_text(title, FS_TITLE, UI_TITLE_INK)],
    )
}

/// The padded content column of a panel (second child of [`ui_panel`]): callers
/// add rows/labels here. Consistent inner padding + row gap.
fn ui_panel_body() -> Node {
    Node {
        width: Val::Percent(100.0),
        padding: UiRect::all(Val::Px(UI_PAD)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(UI_GAP),
        ..default()
    }
}

/// A kit button: solid fill + wood border + rounded corners, consistent height
/// and horizontal padding. Hover/press/active fills are driven by
/// [`update_kit_buttons`]; callers add a marker + a text child. Tag with
/// [`KitToggle`] for buttons that show a persistent active state.
fn ui_button() -> impl Bundle {
    (
        Button,
        Node {
            height: Val::Px(UI_BTN_H),
            flex_shrink: 0.0,
            padding: UiRect::axes(Val::Px(UI_PAD), Val::Px(0.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(UI_BUTTON_BROWN),
        BorderColor::all(Color::NONE),
        ImageNode::default(),
        KitButton,
    )
}

/// A compact kit button (chips, "x" affordances, inline row buttons): the same
/// fill/border/states as [`ui_button`] at a smaller height + tighter padding.
fn ui_button_small() -> impl Bundle {
    (
        Button,
        Node {
            height: Val::Px(22.0),
            min_width: Val::Px(22.0),
            flex_shrink: 0.0,
            padding: UiRect::axes(Val::Px(UI_GAP), Val::Px(0.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(UI_BUTTON_BROWN),
        BorderColor::all(Color::NONE),
        ImageNode::default(),
        KitButton,
    )
}

/// Marks a button styled by the kit so [`update_kit_buttons`] owns its fill.
#[derive(Component)]
struct KitButton;

/// Buttons remain present and legible while unavailable; the grey Adventure
/// frame and disabled cursor make that state explicit without hiding context.
#[derive(Component, Default)]
pub(crate) struct KitDisabled {
    disabled: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AdventureButtonTexture {
    Brown,
    Red,
    Grey,
}

#[derive(Clone, Copy, PartialEq, Debug)]
struct AdventureButtonAppearance {
    texture: AdventureButtonTexture,
    tint: Color,
    fallback: Color,
}

fn adventure_button_appearance(
    interaction: Interaction,
    active: bool,
    disabled: bool,
) -> AdventureButtonAppearance {
    if disabled {
        return AdventureButtonAppearance {
            texture: AdventureButtonTexture::Grey,
            tint: Color::srgb(0.72, 0.72, 0.72),
            fallback: UI_BUTTON_GREY,
        };
    }
    match (interaction, active) {
        (Interaction::Pressed, _) => AdventureButtonAppearance {
            texture: AdventureButtonTexture::Red,
            tint: Color::srgb(0.82, 0.72, 0.62),
            fallback: UI_BUTTON_RED,
        },
        (Interaction::Hovered, true) | (Interaction::None, true) => AdventureButtonAppearance {
            texture: AdventureButtonTexture::Red,
            tint: Color::WHITE,
            fallback: UI_BUTTON_RED,
        },
        (Interaction::Hovered, false) => AdventureButtonAppearance {
            texture: AdventureButtonTexture::Brown,
            tint: Color::srgb(1.0, 0.88, 0.72),
            fallback: UI_BUTTON_BROWN_HOVER,
        },
        (Interaction::None, false) => AdventureButtonAppearance {
            texture: AdventureButtonTexture::Brown,
            tint: Color::WHITE,
            fallback: UI_BUTTON_BROWN,
        },
    }
}

/// A kit button that stays lit while `active` (tabs, tool modes). The owning
/// system sets this each frame; [`update_kit_buttons`] paints the active fill.
#[derive(Component, Default)]
struct KitToggle {
    active: bool,
}

/// Which centre panel a top-bar tab opens — lets one system light the active tab.
#[derive(Component, Clone, Copy)]
enum TabKind {
    Log,
    Goods,
    Census,
    Tree,
}

/// Light the top-bar tab whose panel is currently open.
fn sync_tab_toggles(
    ann: Res<AnnouncementsUi>,
    goods: Res<GoodsUi>,
    census: Res<CensusUi>,
    tree: Res<UpgradeTreeUi>,
    mut tabs: Query<(&TabKind, &mut KitToggle)>,
) {
    for (kind, mut toggle) in &mut tabs {
        toggle.active = match kind {
            TabKind::Log => ann.visible,
            TabKind::Goods => goods.visible,
            TabKind::Census => census.visible,
            TabKind::Tree => tree.visible,
        };
    }
}

/// Paint every [`KitButton`] from its interaction + optional toggle state. One
/// system for all kit buttons so hover/press/active look identical everywhere.
type KitButtonQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Interaction,
        &'static mut BackgroundColor,
        &'static mut ImageNode,
        Option<&'static KitToggle>,
        Option<&'static KitDisabled>,
        Option<&'static InteractionDisabled>,
    ),
    With<KitButton>,
>;

fn update_kit_buttons(
    mut commands: Commands,
    art: Res<AdventureUiArt>,
    mut buttons: KitButtonQuery,
) {
    for (entity, interaction, mut bg, mut image, toggle, disabled, interaction_disabled) in
        &mut buttons
    {
        let active = toggle.is_some_and(|t| t.active);
        let disabled = disabled.is_some_and(|state| state.disabled);
        let appearance = adventure_button_appearance(*interaction, active, disabled);
        bg.0 = appearance.fallback;
        match (disabled, interaction_disabled.is_some()) {
            (true, false) => {
                commands.entity(entity).insert(InteractionDisabled);
            }
            (false, true) => {
                commands.entity(entity).remove::<InteractionDisabled>();
            }
            _ => {}
        }
        image.image = match appearance.texture {
            AdventureButtonTexture::Brown => art.button.clone(),
            AdventureButtonTexture::Red => art.button_active.clone(),
            AdventureButtonTexture::Grey => art.button_disabled.clone(),
        };
        image.image_mode = NodeImageMode::Sliced(button_slicer());
        image.visual_box = VisualBox::BorderBox;
        image.color = appearance.tint;
    }
}

fn sync_action_button_availability(
    session: Res<Session>,
    mut buttons: Query<&mut KitDisabled, With<ActionButton>>,
) {
    if !session.is_changed() {
        return;
    }
    for mut disabled in &mut buttons {
        disabled.disabled = !session.ready;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AdventureCursorKind {
    Pointer,
    Interact,
    Pressed,
    Target,
    Disabled,
}

#[derive(Resource, Default)]
struct AdventureCursorState(Option<AdventureCursorKind>);

fn adventure_cursor_kind(
    interactions: impl IntoIterator<Item = (Interaction, bool)>,
    targeting: bool,
    over_world_input_blocker: bool,
) -> AdventureCursorKind {
    let mut hovered = false;
    let mut hovered_disabled = false;
    for (interaction, disabled) in interactions {
        match interaction {
            Interaction::Pressed if !disabled => return AdventureCursorKind::Pressed,
            Interaction::Pressed | Interaction::Hovered if disabled => hovered_disabled = true,
            Interaction::Hovered => hovered = true,
            Interaction::Pressed | Interaction::None => {}
        }
    }
    if hovered_disabled {
        AdventureCursorKind::Disabled
    } else if hovered {
        AdventureCursorKind::Interact
    } else if targeting && !over_world_input_blocker {
        AdventureCursorKind::Target
    } else {
        AdventureCursorKind::Pointer
    }
}

fn adventure_cursor_hotspot(kind: AdventureCursorKind) -> (u16, u16) {
    match kind {
        // The pointer PNG has transparent padding; its first visible tip pixel
        // is at (8, 6), not at the image origin.
        AdventureCursorKind::Pointer => (8, 6),
        AdventureCursorKind::Interact => (7, 2),
        AdventureCursorKind::Pressed => (8, 8),
        AdventureCursorKind::Target | AdventureCursorKind::Disabled => (16, 16),
    }
}

fn adventure_cursor_icon(kind: AdventureCursorKind, art: &AdventureUiArt) -> CursorIcon {
    let handle = match kind {
        AdventureCursorKind::Pointer => art.cursor_pointer.clone(),
        AdventureCursorKind::Interact => art.cursor_interact.clone(),
        AdventureCursorKind::Pressed => art.cursor_pressed.clone(),
        AdventureCursorKind::Target => art.cursor_target.clone(),
        AdventureCursorKind::Disabled => art.cursor_disabled.clone(),
    };
    CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
        handle,
        hotspot: adventure_cursor_hotspot(kind),
        ..default()
    }))
}

#[allow(clippy::too_many_arguments)]
fn update_adventure_cursor(
    mut commands: Commands,
    art: Res<AdventureUiArt>,
    tools: Res<Tools>,
    research: Res<UpgradeTreeUi>,
    buttons: Query<(&Interaction, Option<&KitDisabled>), With<Button>>,
    blockers: WorldInputBlockerQuery,
    window: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut state: ResMut<AdventureCursorState>,
) {
    let interactions = buttons.iter().map(|(interaction, disabled)| {
        (*interaction, disabled.is_some_and(|state| state.disabled))
    });
    let Ok((window_entity, window)) = window.single() else {
        return;
    };
    let over_world_input_blocker =
        research.visible || cursor_over_world_input_blocker(window.cursor_position(), &blockers);
    let kind = adventure_cursor_kind(
        interactions,
        tools.mode.paint_kind().is_some(),
        over_world_input_blocker,
    );
    if state.0 == Some(kind) {
        return;
    }
    commands
        .entity(window_entity)
        .insert(adventure_cursor_icon(kind, &art));
    state.0 = Some(kind);
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

/// The single camera that renders and navigates the world. Keeping an explicit
/// marker prevents future UI/minimap cameras from being panned accidentally.
#[derive(Component)]
struct WorldCamera;

/// An opaque or framed UI surface that deliberately owns pointer input inside
/// its computed rectangle. Transparent alignment wrappers are never marked.
#[derive(Component)]
struct WorldInputBlocker;

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
const ALL_LABORS: [Labor; 19] = [
    Labor::Hunt,
    Labor::Fishing,
    Labor::Build,
    Labor::Ritual,
    Labor::Fight,
    Labor::Train,
    Labor::Quarry,
    Labor::Woodcut,
    Labor::Forage,
    Labor::FetchWater,
    Labor::Mill,
    Labor::Process,
    Labor::Craft,
    Labor::Textile,
    Labor::Metalwork,
    Labor::Farm,
    Labor::Haul,
    Labor::Research,
    Labor::Scout,
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
/// Container whose children are rebuilt from the snapshot's village list.
#[derive(Component)]
struct VillageSelectorRows;
/// One village selector button, keyed by stable colony id.
#[derive(Component, Clone)]
struct VillageButton(String);
/// Offer the currently configured barter to a discovered village.
#[derive(Component, Clone)]
struct VillageTradeProposalButton(String);
#[derive(Component, Clone, Copy)]
struct VillageTradeDraftButton(VillageTradeDraftField);
#[derive(Component, Clone)]
struct AcceptVillageTradeButton(String);
#[derive(Component, Clone)]
struct CancelVillageTradeButton(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VillageTradeDraftField {
    OfferedKind,
    OfferedAmount,
    RequestedKind,
    RequestedAmount,
}

#[derive(Resource, Debug, Clone, PartialEq)]
struct VillageTradeDraft {
    offered_kind: ResourceKind,
    offered_amount: f64,
    requested_kind: ResourceKind,
    requested_amount: f64,
}

impl Default for VillageTradeDraft {
    fn default() -> Self {
        Self {
            offered_kind: ResourceKind::Food,
            offered_amount: 5.0,
            requested_kind: ResourceKind::Materials,
            requested_amount: 5.0,
        }
    }
}
/// Marker for a building marker sprite.
#[derive(Component)]
struct BuildingSprite;
/// A soil or crop sprite belonging to a player-designated farm plot. These also
/// carry [`BuildingSprite`] so the snapshot redraw stays atomic with buildings.
#[derive(Component)]
struct FarmPlotSprite;
/// One repeated footprint tile beneath a building or open station.
#[derive(Component)]
struct StationFloorSprite;
/// One function-readable prop in an open station.
#[derive(Component)]
struct StationPropSprite;
/// A roofed facade, reserved for residential buildings.
#[derive(Component)]
struct RoofedBuildingSprite;
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
#[derive(Component)]
struct CycleFarmCrop;
#[derive(Component)]
struct FarmCropText;
#[derive(Component)]
struct CycleGatherKind;
#[derive(Component)]
struct GatherKindText;
/// Marker for the officers panel node (toggled with `O`).
#[derive(Component)]
struct OfficersPanel;
/// Manual orders sheet (toggled with `P`).
#[derive(Component)]
struct OrdersPanel;
/// Compact event log hidden while the wide manual-orders sheet owns its lane.
#[derive(Component)]
struct DispatchesPanel;
#[derive(Component, Clone, Copy)]
struct OrderButton(OrderAction);
#[derive(Component)]
struct CycleOrderBuilding;
#[derive(Component)]
struct PlannedBuildingText;
#[derive(Component)]
struct CycleElectionCandidate;
#[derive(Component)]
struct ElectionCandidateText;
#[derive(Component)]
struct CastElectionVoteButton;
#[derive(Component)]
struct RequestVoteKickButton;
#[derive(Component)]
struct VoteKickButtonText;
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
#[derive(Component)]
struct CycleLaborPreference;
#[derive(Component)]
struct ToggleLaborPreference;
#[derive(Component)]
struct LaborPreferenceText;
#[derive(Resource, Default)]
struct LaborPreferenceUi {
    selected: usize,
}
#[derive(Component)]
struct StationQueueControls;
#[derive(Component)]
struct StationQueueText;
#[derive(Component, Clone, Copy)]
enum StationQueueButton {
    Add,
    SelectNext,
    MoveUp,
    MoveDown,
    Remove,
    ToggleRepeat,
    TogglePause,
}
#[derive(Resource, Default)]
struct StationQueueUi {
    selected: usize,
}
/// Marker for the HUD colony header text (name / leader / pop / threat).
#[derive(Component)]
struct HudHeaderText;
/// Marker for the HUD jobs + ledger footer text.
#[derive(Component)]
struct HudFooterText;
/// Marker for one deterministic terrain/decor visual, used to unload a chunk.
#[derive(Component, Clone, Copy)]
struct TerrainVisual(ChunkKey);
/// A procedural nature/resource decoration keyed by its terrain tile. Unlike
/// the ground sprite, this is hidden inside the selected village's permanent
/// founding wall core.
#[derive(Component, Clone, Copy)]
struct TerrainDecoration {
    x: i32,
    y: i32,
    role: DecorationRole,
}
/// A fog-of-war tile sprite, keyed by tile for incremental updates.
#[derive(Component, Clone, Copy)]
struct FogTile {
    x: i32,
    y: i32,
}
/// Prominent action/connection feedback panel and its text child.
#[derive(Component)]
struct ClientFeedbackPanel;
#[derive(Component)]
struct ClientFeedbackText;
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
/// sim's `age::get_life_stage` (kitten <6h, young <24h, adult <240h, elder ≥240h);
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
        } else if age < 240.0 {
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
fn census_report_lines(c: &Census, scale: VillageScale) -> Vec<String> {
    let stage_max = c.kittens.max(c.young).max(c.adults).max(c.elders);
    let leader = c.leader.as_deref().unwrap_or("(vacant)");
    vec![
        format!(
            "{} population: {}",
            match scale {
                VillageScale::Communal => "Communal",
                VillageScale::Personal => "Personal",
            },
            c.total
        ),
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
        EventKind::Birth => UI_POSITIVE,
        EventKind::Death | EventKind::Raid => UI_WARNING,
        // A saturated amber, distinct from the calmer gold used for titles/tabs.
        EventKind::Crisis => Color::srgb(1.0, 0.66, 0.16),
        // Elections + progress read as calm "info"; a punchy blue reads clearly
        // on the dark panel and is the one hue outside the warm kit palette.
        EventKind::Election | EventKind::Progress => Color::srgb(0.40, 0.66, 1.0),
        EventKind::Neutral => UI_MUTED,
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
const STORABLE_KINDS: [ResourceKind; 18] = [
    ResourceKind::Food,
    ResourceKind::Water,
    ResourceKind::Herbs,
    ResourceKind::Catnip,
    ResourceKind::Grain,
    ResourceKind::Flour,
    ResourceKind::Materials,
    ResourceKind::Refined,
    ResourceKind::Weapons,
    ResourceKind::Armor,
    ResourceKind::Logs,
    ResourceKind::Lumber,
    ResourceKind::Fibre,
    ResourceKind::Hide,
    ResourceKind::Cloth,
    ResourceKind::Leather,
    ResourceKind::Ore,
    ResourceKind::Metal,
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
type AcceptButtonQuery<'w, 's> =
    Query<'w, 's, &'static Interaction, (Changed<Interaction>, With<AcceptButton>)>;
/// Change filter for toolbar button interactions (visuals are the kit's job).
type ButtonQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Interaction, &'static ActionButton),
    (Changed<Interaction>, With<Button>),
>;
/// Disjoint feedback label query kept named so the HUD system signature stays readable.
type FeedbackLabelQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (With<ClientFeedbackText>, Without<ClientFeedbackPanel>),
>;
/// Visible top-level UI rectangles that block world hover tooltips.
type UiRootQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static Node,
    ),
    (Without<ChildOf>, Without<TooltipPanel>),
>;
/// Deliberately marked panel rectangles that own pointer input over the world.
type WorldInputBlockerQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static ComputedNode,
        &'static UiGlobalTransform,
        &'static Node,
    ),
    With<WorldInputBlocker>,
>;

fn ui_surface_blocks_world(display: Display, contains_cursor: bool) -> bool {
    display != Display::None && contains_cursor
}

fn cursor_over_world_input_blocker(
    cursor: Option<Vec2>,
    blockers: &WorldInputBlockerQuery<'_, '_>,
) -> bool {
    cursor.is_some_and(|cursor| {
        blockers.iter().any(|(computed, transform, style)| {
            ui_surface_blocks_world(style.display, computed.contains_point(*transform, cursor))
        })
    })
}

fn world_pointer_input_allowed(
    research_visible: bool,
    over_world_input_blocker: bool,
    has_cursor: bool,
) -> bool {
    !research_visible && !over_world_input_blocker && has_cursor
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonAction {
    SupplyFood,
    SupplyWater,
    PlanHunt,
    ScoutWood,
    ScoutFood,
    ScoutWater,
    ScoutStone,
    Explore,
    FoundVillage,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum OrderAction {
    Hunt,
    Fish,
    FetchWater,
    Quarry,
    GatherLogs,
    ForageFibre,
    ExpandVillage,
    Ritual,
    OfferTithe,
    OfferMaterials,
    HaulSelected,
    PlanBuilding,
    StaffSelected,
    UnstaffSelected,
    TrainSelected,
    DefendRaid,
}

impl OrderAction {
    #[cfg(test)]
    const ALL: [Self; 16] = [
        Self::Hunt,
        Self::Fish,
        Self::FetchWater,
        Self::Quarry,
        Self::GatherLogs,
        Self::ForageFibre,
        Self::ExpandVillage,
        Self::Ritual,
        Self::OfferTithe,
        Self::OfferMaterials,
        Self::HaulSelected,
        Self::PlanBuilding,
        Self::StaffSelected,
        Self::UnstaffSelected,
        Self::TrainSelected,
        Self::DefendRaid,
    ];
    const JOBS: [Self; 8] = [
        Self::Hunt,
        Self::Fish,
        Self::FetchWater,
        Self::Quarry,
        Self::GatherLogs,
        Self::ForageFibre,
        Self::ExpandVillage,
        Self::Ritual,
    ];
    const TARGETS: [Self; 7] = [
        Self::OfferTithe,
        Self::OfferMaterials,
        Self::HaulSelected,
        Self::StaffSelected,
        Self::UnstaffSelected,
        Self::TrainSelected,
        Self::DefendRaid,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Hunt => "Hunt",
            Self::Fish => "Fish shore",
            Self::FetchWater => "Fetch water",
            Self::Quarry => "Quarry",
            Self::GatherLogs => "Gather logs",
            Self::ForageFibre => "Forage fibre",
            Self::ExpandVillage => "Expand village",
            Self::Ritual => "Request ritual",
            Self::OfferTithe => "Offer tithe",
            Self::OfferMaterials => "Offer materials",
            Self::HaulSelected => "Haul selected pile",
            Self::PlanBuilding => "Plan building",
            Self::StaffSelected => "Staff selected",
            Self::UnstaffSelected => "Unstaff cat",
            Self::TrainSelected => "Train selected",
            Self::DefendRaid => "Defend raid",
        }
    }
}

const PLANNABLE_BUILDINGS: [BuildingType; 24] = [
    BuildingType::Den,
    BuildingType::FoodStorage,
    BuildingType::WaterBowl,
    BuildingType::Beds,
    BuildingType::HerbGarden,
    BuildingType::Nursery,
    BuildingType::ElderCorner,
    BuildingType::Walls,
    BuildingType::MouseFarm,
    BuildingType::Workshop,
    BuildingType::AccountingTent,
    BuildingType::Field,
    BuildingType::ResearchHut,
    BuildingType::School,
    BuildingType::Smithy,
    BuildingType::Barracks,
    BuildingType::WoodCutter,
    BuildingType::StonePrep,
    BuildingType::Woodworking,
    BuildingType::Clothier,
    BuildingType::Tannery,
    BuildingType::Smelter,
    BuildingType::Mill,
    BuildingType::Sawmill,
];

impl ButtonAction {
    fn label(self) -> &'static str {
        match self {
            Self::SupplyFood => "Supply food",
            Self::SupplyWater => "Supply water",
            Self::PlanHunt => "Plan hunt",
            Self::ScoutWood => "Find wood",
            Self::ScoutFood => "Find food",
            Self::ScoutWater => "Find water",
            Self::ScoutStone => "Find stone",
            Self::Explore => "Explore",
            Self::FoundVillage => "Found village",
        }
    }
}

/// Dev-only plugin that turns on the Bevy Remote Protocol server (BRP, port
/// 15702) so the bevy MCP can `world_query`/introspect the running game for
/// automated playtesting. Compiled in on native (via the `bevy_remote` feature)
/// but a **no-op unless the `CAT_BRP` env var is set**, so a normal `cargo dev`
/// never opens the port. On wasm it does nothing (no TCP listener in the browser).
struct BrpDevPlugin;

impl Plugin for BrpDevPlugin {
    fn build(&self, _app: &mut App) {
        #[cfg(not(target_arch = "wasm32"))]
        if std::env::var("CAT_BRP").is_ok() {
            _app.add_plugins((
                bevy::remote::RemotePlugin::default(),
                bevy::remote::http::RemoteHttpPlugin::default(),
            ));
            // Bevy 0.19 with default-features=false does not auto-register these
            // render/UI types for reflection, so BRP `world.query` can't see them
            // out of the box. Register the ones a playtester wants to read: Text
            // (the live HUD strings), Transform/Sprite (positions of cats,
            // buildings), and Node (UI layout).
            _app.register_type::<Text>()
                .register_type::<Transform>()
                .register_type::<GlobalTransform>()
                .register_type::<Sprite>()
                .register_type::<Node>()
                .register_type::<Anchor>();
            info!("BRP enabled on port 15702 (CAT_BRP set) — bevy MCP can world_query this app");
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
                        resolution: bevy::window::WindowResolution::new(1024, 768)
                            .with_scale_factor_override(1.0),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(BrpDevPlugin)
        .insert_resource(LatestSnapshot::default())
        .insert_resource(Session::default())
        .insert_resource(VillageSelection::default())
        .insert_resource(VillageTradeDraft::default())
        .insert_resource(ConnectionState::default())
        .insert_resource(ClientFeedback::default())
        .insert_resource(ClientAlerts::default())
        .insert_resource(OutgoingActions::default())
        .insert_resource(WorldRender::default())
        .insert_resource(Selection::default())
        .insert_resource(StockpileSelection::default())
        .insert_resource(BuildingSelection::default())
        .insert_resource(LaborPreferenceUi::default())
        .insert_resource(StationQueueUi::default())
        .insert_resource(OfficersUi { visible: true })
        .insert_resource(OrdersUi::default())
        .insert_resource(GovernanceUi::default())
        .insert_resource(AnnouncementsUi::default())
        .insert_resource(GoodsUi::default())
        .insert_resource(CensusUi::default())
        .insert_resource(UpgradeTreeUi::default())
        .insert_resource(TradeUi::default())
        .insert_resource(MinimapUi::default())
        .insert_resource(CatBodies::default())
        .insert_resource(RaiderBodies::default())
        .insert_resource(Tools::default())
        .insert_resource(AdventureCursorState::default())
        // Match the unloaded world to full fog so zooming out never exposes a
        // hard rectangle around the bounded chunk cache.
        .insert_resource(ClearColor(FOG_COLOR))
        .add_systems(Startup, (load_persisted_session, setup, connect_ws).chain())
        // Grouped into sub-tuples to stay within Bevy's 20-per-tuple system arity.
        .add_systems(
            Update,
            (
                // networking + world render
                (
                    poll_ws,
                    reconnect_ws.after(poll_ws),
                    ensure_presence.after(poll_ws).after(reconnect_ws),
                    restore_village_selection
                        .after(poll_ws)
                        .after(ensure_presence),
                    spawn_terrain.after(camera_controls),
                    sync_terrain_decoration_visibility.after(spawn_terrain),
                    render_roads,
                    render_fog.after(spawn_terrain),
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
                    camera_controls.after(research_ui::toggle_upgrade_tree),
                    select_cat,
                    select_building,
                    close_inspectors_on_esc,
                    update_building_inspector,
                    update_station_queue_controls.after(update_building_inspector),
                    handle_station_queue_buttons,
                    update_remove_panel,
                    handle_remove_button,
                    update_inspector,
                    (
                        handle_tool_buttons,
                        handle_accept_button,
                        handle_farm_crop_button,
                        handle_gather_kind_button,
                        sync_action_button_availability,
                        update_kit_buttons
                            .after(sync_action_button_availability)
                            .after(handle_tool_buttons)
                            .after(sync_tab_toggles)
                            .after(research_ui::update_research_filter)
                            .after(research_ui::update_research_inspector),
                        sync_tab_toggles,
                        skin_adventure_panels,
                        update_adventure_cursor.after(update_kit_buttons),
                    ),
                    zone_paint,
                    place_building,
                    render_zone_preview,
                    (
                        update_hud,
                        update_village_selector,
                        handle_village_buttons,
                        handle_village_trade_buttons,
                    ),
                    update_event_log,
                    update_client_feedback,
                    handle_buttons,
                    (
                        toggle_officers,
                        toggle_orders,
                        update_officers_panel,
                        update_orders_panel,
                        update_dispatches_panel,
                        handle_order_buttons,
                        handle_order_building_cycle,
                        update_governance_controls,
                        handle_governance_buttons,
                        handle_appoint_buttons,
                        handle_vacate_buttons,
                    ),
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
                    update_labor_preference_controls,
                    handle_labor_preference_buttons,
                    toggle_census,
                    update_census,
                    (
                        research_ui::toggle_upgrade_tree,
                        research_ui::update_research_shell,
                        research_ui::handle_research_controls,
                        research_ui::research_keyboard_input,
                        research_ui::navigate_research_canvas,
                        research_ui::update_research_transform,
                        research_ui::update_research_filter,
                        research_ui::update_research_snapshot,
                        research_ui::update_research_inspector,
                        research_ui::handle_research_purchase,
                    ),
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
    Catnip,
    Grain,
    Flour,
    Materials,
    Refined,
    Planks,
    Blocks,
    Tools,
    Logs,
    Lumber,
    Herbs,
    Weapons,
    Armor,
    Blessings,
}

/// The HUD resources, in display order (refinement tier grouped after refined).
const HUD_RESOURCES: [HudRes; 16] = [
    HudRes::Food,
    HudRes::Water,
    HudRes::Catnip,
    HudRes::Grain,
    HudRes::Flour,
    HudRes::Materials,
    HudRes::Refined,
    HudRes::Planks,
    HudRes::Blocks,
    HudRes::Tools,
    HudRes::Logs,
    HudRes::Lumber,
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
            HudRes::Catnip => self.herbs.clone(),
            HudRes::Grain | HudRes::Flour => self.food.clone(),
            HudRes::Materials => self.materials.clone(),
            HudRes::Refined => self.refined.clone(),
            HudRes::Planks => self.planks.clone(),
            HudRes::Blocks => self.blocks.clone(),
            HudRes::Tools => self.tools.clone(),
            HudRes::Logs | HudRes::Lumber => self.planks.clone(),
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
        ResourceKind::Catnip => HudRes::Catnip,
        ResourceKind::Grain => HudRes::Grain,
        ResourceKind::Flour => HudRes::Flour,
        ResourceKind::Materials => HudRes::Materials,
        ResourceKind::Refined => HudRes::Refined,
        ResourceKind::Weapons => HudRes::Weapons,
        ResourceKind::Armor => HudRes::Armor,
        ResourceKind::Logs => HudRes::Logs,
        ResourceKind::Lumber => HudRes::Lumber,
        ResourceKind::Planks => HudRes::Lumber,
        ResourceKind::Blocks => HudRes::Materials,
        ResourceKind::Tools => HudRes::Refined,
        ResourceKind::Fibre | ResourceKind::Cloth => HudRes::Herbs,
        ResourceKind::Hide | ResourceKind::Leather => HudRes::Materials,
        ResourceKind::Ore => HudRes::Materials,
        ResourceKind::Metal => HudRes::Refined,
        ResourceKind::Blessings => HudRes::Blessings,
    }
}

/// The tint applied to a resource's white glyph so the readout reads at a glance.
fn resource_icon_tint(kind: HudRes) -> Color {
    match kind {
        HudRes::Food => Color::srgb(0.87, 0.35, 0.26),
        HudRes::Water => Color::srgb(0.36, 0.62, 0.93),
        HudRes::Catnip => Color::srgb(0.66, 0.48, 0.82),
        HudRes::Grain => Color::srgb(0.88, 0.68, 0.26),
        HudRes::Flour => Color::srgb(0.92, 0.88, 0.72),
        HudRes::Materials => Color::srgb(0.62, 0.46, 0.29),
        HudRes::Refined => Color::srgb(0.86, 0.71, 0.40),
        HudRes::Planks => Color::srgb(0.82, 0.66, 0.42),
        HudRes::Blocks => Color::srgb(0.62, 0.64, 0.66),
        HudRes::Tools => Color::srgb(0.70, 0.74, 0.80),
        HudRes::Logs => Color::srgb(0.45, 0.29, 0.17),
        HudRes::Lumber => Color::srgb(0.76, 0.55, 0.30),
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
        HudRes::Catnip => format!("{:.0} / {:.0}", r.catnip, cap.catnip),
        HudRes::Grain => format!("{:.0} / {:.0}", r.grain, cap.grain),
        HudRes::Flour => format!("{:.0} / {:.0}", r.flour, cap.flour),
        HudRes::Materials => format!("{:.0} / {:.0}", r.materials, cap.materials),
        HudRes::Refined => format!("{:.0} / {:.0}", r.refined, cap.refined),
        HudRes::Planks => format!("{:.0} / {:.0}", r.planks, cap.planks),
        HudRes::Blocks => format!("{:.0} / {:.0}", r.blocks, cap.blocks),
        HudRes::Tools => format!("{:.0} / {:.0}", r.tools, cap.tools),
        HudRes::Logs => format!("{:.0} / {:.0}", r.logs, cap.logs),
        HudRes::Lumber => format!("{:.0} / {:.0}", r.lumber, cap.lumber),
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
    let ui_art = AdventureUiArt::load(&asset_server);
    commands.insert_resource(ui_art.clone());

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
    commands.spawn((
        Camera2d,
        Transform::from_xyz(center.x, center.y, CAMERA_Z),
        WorldCamera,
    ));

    // A deliberate walnut rail backs the fixed right-hand inspector/minimap
    // lane. At the supported 1024px width the revealed world may end before
    // this reserved UI column; without a backing surface that unused camera
    // area reads as an accidental black rectangle whenever no inspector is
    // open. Spawn it before the interactive panels so they naturally layer on
    // top, and let the bottom toolbar cover its lower edge.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(0.0),
            top: Val::Px(50.0),
            bottom: Val::Px(BOTTOM_OVERLAY_CLEARANCE),
            width: Val::Px(204.0),
            border: UiRect::left(Val::Px(4.0)),
            ..default()
        },
        GlobalZIndex(-100),
        BackgroundColor(UI_HEADER),
        BorderColor::all(UI_BUTTON_BROWN),
    ));

    // Transport and rejected-action feedback must not disappear into the log.
    // Keep a compact banner above the world; it is hidden until feedback exists.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(30.0),
                top: Val::Px(58.0),
                width: Val::Percent(40.0),
                min_height: Val::Px(34.0),
                padding: UiRect::axes(Val::Px(UI_PAD), Val::Px(UI_GAP)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            GlobalZIndex(100),
            ui_panel_frame(),
            ClientFeedbackPanel,
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn((
                ui_text("", FS_BODY, UI_INK),
                TextLayout::justify(Justify::Center),
                ClientFeedbackText,
            ));
        });

    // HUD dashboard (top-left): a kit panel with a "Colony" title bar, a status
    // header, a two-column resource grid and a jobs/ledger footer.
    commands
        .spawn((
            Node {
                left: Val::Px(10.0),
                top: Val::Px(52.0),
                ..ui_panel_node(Val::Px(322.0))
            },
            ui_panel_frame(),
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(ui_title_bar("Colony"));
            panel.spawn(ui_panel_body()).with_children(|body| {
                // Status header (name / leader / pop / threat).
                body.spawn((ui_text("connecting…", FS_SECTION, UI_INK), HudHeaderText));
                // Resource readout: a tinted glyph + value per resource in
                // TWO columns (a wrapping row of fixed-width cells) so the 11
                // resources fit ~6 rows and the panel stays compact.
                body.spawn(Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: Val::Px(UI_GAP_TIGHT),
                    column_gap: Val::Px(UI_GAP),
                    margin: UiRect::vertical(Val::Px(UI_GAP_TIGHT)),
                    ..default()
                })
                .with_children(|grid| {
                    for kind in HUD_RESOURCES {
                        grid.spawn((
                            Node {
                                width: Val::Px(138.0),
                                height: Val::Px(HUD_RESOURCE_PILL_HEIGHT),
                                padding: UiRect::horizontal(Val::Px(4.0)),
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(UI_GAP),
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                            ImageNode::default(),
                            AdventurePanel::Dark,
                            children![
                                (
                                    Node {
                                        width: Val::Px(18.0),
                                        height: Val::Px(18.0),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    ImageNode::new(ui_art.icon_frame.clone()),
                                    children![(
                                        Node {
                                            width: Val::Px(11.0),
                                            height: Val::Px(11.0),
                                            ..default()
                                        },
                                        ImageNode {
                                            image: icons.get(kind),
                                            color: resource_icon_tint(kind),
                                            ..default()
                                        },
                                    )],
                                ),
                                (ui_text("-", FS_BODY, UI_TITLE_INK), HudResource(kind)),
                            ],
                        ));
                    }
                });
                // Jobs + ledger footer, set off by a faint divider.
                body.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(1.0),
                        border: UiRect::top(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor::all(UI_DIVIDER),
                ));
                body.spawn((ui_text("", FS_BODY, UI_MUTED), HudFooterText));
            });
        });

    // Event log (bottom-left): a kit panel with a scrolling text body.
    commands
        .spawn((
            Node {
                left: Val::Px(10.0),
                // Clear the two-row command bar at 768px-high windows.
                bottom: Val::Px(BOTTOM_OVERLAY_CLEARANCE),
                ..ui_panel_node(Val::Px(430.0))
            },
            ui_panel_frame(),
            DispatchesPanel,
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(ui_title_bar("Dispatches"));
            panel.spawn(ui_panel_body()).with_children(|body| {
                body.spawn((ui_text("events…", FS_BODY, UI_MUTED), EventLogText));
            });
        });

    // Corner minimap (bottom-right, clear of the inspectors + toolbars): a kit
    // panel with a "Map" title bar over the dynamic minimap texture. Toggled 'M'.
    commands
        .spawn((
            Node {
                right: Val::Px(10.0),
                // Clear the two-row command bar at 768px-high windows.
                bottom: Val::Px(BOTTOM_OVERLAY_CLEARANCE),
                ..ui_panel_node(Val::Px(168.0 + 2.0 * UI_GAP + 2.0 * UI_BORDER_W))
            },
            GlobalZIndex(70),
            ui_panel_frame(),
            MinimapPanel,
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(ui_title_bar("Map"));
            panel
                .spawn(Node {
                    padding: UiRect::all(Val::Px(UI_GAP)),
                    ..default()
                })
                .with_children(|body| {
                    body.spawn((
                        Node {
                            width: Val::Px(168.0),
                            height: Val::Px(168.0),
                            ..default()
                        },
                        ImageNode::new(minimap_handle),
                        // Button so the world-pick systems (which skip Button
                        // interactions) ignore clicks that land on the minimap.
                        Button,
                        RelativeCursorPosition::default(),
                        MinimapImageNode,
                        // Camera-viewport outline, positioned each frame.
                        children![
                            (
                                Node {
                                    position_type: PositionType::Absolute,
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.85)),
                                MinimapViewportRect,
                            ),
                            (
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(0.0),
                                    top: Val::Px(0.0),
                                    width: Val::Px(168.0),
                                    height: Val::Px(168.0),
                                    ..default()
                                },
                                ImageNode {
                                    image: ui_art.minimap_ring.clone(),
                                    image_mode: NodeImageMode::Stretch,
                                    ..default()
                                },
                            )
                        ],
                    ));
                });
        });

    // Top command bar: game title + panel tabs (Log/Goods/Census/Tree) + the
    // latest-dispatch ticker, all in ONE framed strip so the controls read as a
    // designed toolbar instead of buttons scattered at fixed pixel offsets. The
    // active tab stays lit via `sync_tab_toggles`.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                left: Val::Px(10.0),
                right: Val::Px(10.0),
                height: Val::Px(42.0),
                align_items: AlignItems::Center,
                column_gap: Val::Px(UI_GAP),
                padding: UiRect::horizontal(Val::Px(UI_PAD)),
                border: UiRect::all(Val::Px(UI_BORDER_W)),
                border_radius: BorderRadius::all(Val::Px(UI_RADIUS)),
                ..default()
            },
            GlobalZIndex(60),
            BackgroundColor(UI_BG),
            BorderColor::all(Color::NONE),
            ImageNode::default(),
            AdventurePanel::Paper,
            WorldInputBlocker,
        ))
        .with_children(|bar| {
            bar.spawn((
                Node {
                    width: Val::Px(168.0),
                    height: Val::Px(34.0),
                    margin: UiRect::right(Val::Px(UI_GAP)),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ImageNode {
                    image: ui_art.banner.clone(),
                    image_mode: NodeImageMode::Stretch,
                    ..default()
                },
                children![(
                    ui_text("Idle Cat Forest", FS_TITLE, UI_TITLE_INK),
                    TextLayout::no_wrap(),
                )],
            ));
            bar.spawn((
                ui_button(),
                AnnouncementsButton,
                TabKind::Log,
                KitToggle::default(),
                children![ui_text("Log [L]", FS_BODY, UI_INK)],
            ));
            bar.spawn((
                ui_button(),
                GoodsButton,
                TabKind::Goods,
                KitToggle::default(),
                children![ui_text("Goods [G]", FS_BODY, UI_INK)],
            ));
            bar.spawn((
                ui_button(),
                CensusButton,
                TabKind::Census,
                KitToggle::default(),
                children![ui_text("Census [C]", FS_BODY, UI_INK)],
            ));
            bar.spawn((
                ui_button(),
                TreeButton,
                TabKind::Tree,
                KitToggle::default(),
                children![ui_text("Tree [U]", FS_BODY, UI_INK)],
            ));
            // Ticker: pushed to the right edge, clipped so it never overflows.
            bar.spawn((
                Node {
                    margin: UiRect::left(Val::Auto),
                    min_width: Val::Px(0.0),
                    max_width: Val::Px(400.0),
                    flex_shrink: 1.0,
                    overflow: Overflow::clip(),
                    ..default()
                },
                ui_text("", FS_SMALL, UI_MUTED),
                TextLayout::no_wrap(),
                AnnouncementTicker,
            ));
        });

    // Shared-world village selector. It stays visible beside (not on top of)
    // the fixed HUD and inspectors, so changing the action/render target is an
    // explicit player choice rather than an invisible consequence of snapshot
    // ordering or founding another settlement.
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(54.0),
                left: Val::Px(340.0),
                right: Val::Px(310.0),
                min_height: Val::Px(62.0),
                padding: UiRect::all(Val::Px(UI_GAP)),
                align_items: AlignItems::Center,
                column_gap: Val::Px(UI_GAP),
                border: UiRect::all(Val::Px(UI_BORDER_W)),
                border_radius: BorderRadius::all(Val::Px(UI_RADIUS)),
                ..default()
            },
            GlobalZIndex(65),
            ui_panel_frame(),
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn((
                Node {
                    flex_shrink: 0.0,
                    ..default()
                },
                ui_text("Villages", FS_SECTION, UI_ACCENT),
                TextLayout::no_wrap(),
            ));
            panel.spawn((
                Node {
                    min_width: Val::Px(0.0),
                    flex_grow: 1.0,
                    flex_wrap: FlexWrap::Wrap,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(UI_GAP),
                    row_gap: Val::Px(UI_GAP),
                    ..default()
                },
                VillageSelectorRows,
            ));
        });

    // Announcements panel (centre), hidden until toggled. Lines are colour-coded
    // per event kind by `update_announcements`. Its Display is driven each frame
    // from `AnnouncementsUi.visible`, so it starts hidden without a spawn flag.
    commands
        .spawn((
            Node {
                left: Val::Px(456.0),
                top: Val::Px(60.0),
                ..ui_panel_node(Val::Px(500.0))
            },
            GlobalZIndex(80),
            ui_panel_frame(),
            AnnouncementsPanel,
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(ui_title_bar("Announcements"));
            panel.spawn(ui_panel_body()).with_children(|body| {
                for i in 0..ANNOUNCEMENT_LINES {
                    body.spawn((ui_text("", FS_BODY, UI_MUTED), AnnouncementLine(i)));
                }
            });
        });

    // Colony census / demographics panel (centre; shares the slot with the other
    // tab panels — mutually exclusive), hidden until toggled.
    commands
        .spawn((
            Node {
                left: Val::Px(456.0),
                top: Val::Px(60.0),
                ..ui_panel_node(Val::Px(360.0))
            },
            GlobalZIndex(82),
            ui_panel_frame(),
            CensusPanel,
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(ui_title_bar("Census"));
            panel.spawn(ui_panel_body()).with_children(|body| {
                for i in 0..CENSUS_LINES {
                    body.spawn((ui_text("", FS_BODY, UI_INK), CensusLine(i)));
                }
            });
        });

    // (Log/Goods/Census/Tree toggles now live in the top command bar above.)

    // Research is a full-page ledger, not another cramped centre popover. Its
    // 500 cards and dependency connectors are spawned once and updated in place.
    research_ui::spawn_research_ui(&mut commands);

    // Goods / inventory panel (centre, shares the slot with announcements — the
    // two are mutually exclusive), hidden until toggled.
    commands
        .spawn((
            Node {
                left: Val::Px(456.0),
                top: Val::Px(60.0),
                ..ui_panel_node(Val::Px(500.0))
            },
            GlobalZIndex(82),
            ui_panel_frame(),
            GoodsPanel,
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(ui_title_bar("Goods"));
            panel.spawn(ui_panel_body()).with_children(|body| {
                // Treasury total.
                body.spawn((ui_text("", FS_SECTION, UI_ACCENT), GoodsTreasury));
                for i in 0..GOODS_LINES {
                    body.spawn(Node {
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(UI_GAP),
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
                        row.spawn((ui_text("", FS_BODY, UI_INK), GoodsLine(i)));
                    });
                }
            });
        });

    // Trade menu (centre), shown only while a trader is Trading at the gate.
    let row_node = || Node {
        width: Val::Percent(100.0),
        align_items: AlignItems::Center,
        column_gap: Val::Px(UI_GAP),
        display: Display::None,
        ..default()
    };
    let label_node = || Node {
        flex_grow: 1.0,
        ..default()
    };
    let header = |text: &str| ui_text(text.to_string(), FS_SMALL, UI_MUTED);
    commands
        .spawn((
            Node {
                left: Val::Px(390.0),
                top: Val::Px(70.0),
                ..ui_panel_node(Val::Px(500.0))
            },
            GlobalZIndex(90),
            ui_panel_frame(),
            TradeMenuPanel,
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(ui_title_bar("Trader"));
            panel.spawn(ui_panel_body()).with_children(|body| {
                body.spawn((ui_text("", FS_SECTION, UI_ACCENT), TradeCoinText));
                body.spawn(header("- Sell your crafts -"));
                for i in 0..TRADE_SELL_ROWS {
                    body.spawn((row_node(), SellRow(i))).with_children(|row| {
                        row.spawn((
                            label_node(),
                            children![(ui_text("", FS_SMALL, UI_INK), SellRowText(i))],
                        ));
                        for all in [false, true] {
                            row.spawn((
                                ui_button_small(),
                                SellButton { row: i, all },
                                children![ui_text(
                                    if all { "All" } else { "Sell 1" },
                                    FS_SMALL,
                                    UI_INK
                                )],
                            ));
                        }
                    });
                }
                body.spawn(header("- Buy resources -"));
                for i in 0..TRADE_BUY_ROWS {
                    body.spawn((row_node(), BuyRow(i))).with_children(|row| {
                        row.spawn((
                            label_node(),
                            children![(ui_text("", FS_SMALL, UI_INK), BuyRowText(i))],
                        ));
                        row.spawn((
                            ui_button_small(),
                            BuyButton(i),
                            children![ui_text("Buy 1", FS_SMALL, UI_INK)],
                        ));
                    });
                }
                body.spawn((
                    ui_button(),
                    TradeCloseButton,
                    children![ui_text("Close [Esc]", FS_SMALL, UI_INK)],
                ));
            });
        });

    // Hover tooltip (small, follows the cursor), hidden until hovering an entity.
    // High GlobalZIndex keeps it above the other panels.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            max_width: Val::Px(260.0),
            padding: UiRect::all(Val::Px(UI_PAD)),
            border: UiRect::all(Val::Px(UI_BORDER_W)),
            border_radius: BorderRadius::all(Val::Px(UI_RADIUS)),
            display: Display::None,
            ..default()
        },
        ui_panel_frame(),
        GlobalZIndex(100),
        TooltipPanel,
        children![(ui_text("", FS_BODY, UI_INK), TooltipText)],
    ));

    // Cat inspector (top-right), hidden until a cat is selected. Includes a row
    // of "Appoint <role>" buttons that make the selected cat that officer.
    commands
        .spawn((
            cat_inspector_panel_node(),
            ui_panel_frame(),
            InspectorPanel,
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(ui_title_bar("Cat"));
            panel.spawn(ui_panel_body()).with_children(|body| {
                body.spawn((ui_text("", FS_BODY, UI_INK), InspectorText));
                // Needs, one labelled bar each (green/amber/red by level).
                for (kind, label) in CAT_NEEDS {
                    body.spawn(Node {
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(UI_GAP),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Node {
                                width: Val::Px(52.0),
                                ..default()
                            },
                            children![ui_text(label, FS_SMALL, UI_MUTED)],
                        ));
                        // Bar track + fill.
                        row.spawn((
                            Node {
                                flex_grow: 1.0,
                                height: Val::Px(13.0),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BackgroundColor(Color::NONE),
                            sliced_image(
                                ui_art.progress_track.clone(),
                                BorderRect::all(PROGRESS_SLICE_PX),
                                1.0,
                            ),
                            children![(
                                Node {
                                    width: Val::Percent(0.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                sliced_image(
                                    ui_art.progress_low.clone(),
                                    BorderRect::all(PROGRESS_SLICE_PX),
                                    1.0,
                                ),
                                NeedBar(kind),
                            )],
                        ));
                    });
                }
                // God-power: mark this cat a priority pick for the leader's matcher.
                body.spawn((
                    ui_button(),
                    BoostButton,
                    children![(
                        ui_text(boost_button_label(false), FS_SMALL, UI_INK),
                        BoostButtonText,
                    )],
                ));
                body.spawn(ui_text("Preferred labor:", FS_SMALL, UI_MUTED));
                body.spawn(bottom_bar_row_node()).with_children(|row| {
                    row.spawn((
                        ui_button_small(),
                        CycleLaborPreference,
                        children![(ui_text("hunt", FS_SMALL, UI_INK), LaborPreferenceText)],
                    ));
                    row.spawn((
                        ui_button_small(),
                        ToggleLaborPreference,
                        children![ui_text("enable / clear", FS_SMALL, UI_INK)],
                    ));
                });
                body.spawn(ui_text("Appoint officer:", FS_SMALL, UI_MUTED));
                body.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(UI_GAP_TIGHT),
                    row_gap: Val::Px(UI_GAP_TIGHT),
                    ..default()
                })
                .with_children(|row| {
                    for role in ALL_OFFICER_ROLES {
                        row.spawn((
                            ui_button_small(),
                            AppointButton(role),
                            children![ui_text(officer_role_name(role), FS_SMALL, UI_INK)],
                        ));
                    }
                });
            });
        });

    // Remove-stockpile affordance (right side), hidden until one is selected.
    commands
        .spawn((
            Node {
                right: Val::Px(10.0),
                top: Val::Px(170.0),
                ..ui_panel_node(Val::Px(224.0))
            },
            RemovePanel,
            ui_panel_frame(),
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(ui_title_bar("Stockpile"));
            panel.spawn(ui_panel_body()).with_children(|body| {
                body.spawn((ui_text("", FS_BODY, UI_INK), RemovePanelText));
                body.spawn((
                    ui_button(),
                    RemoveStockpileButton,
                    children![ui_text("Remove designation", FS_BODY, UI_INK)],
                ));
            });
        });

    // Building inspector (top-right), right-click a building; hidden until one
    // is selected. Cat/building selection is mutually exclusive, so both
    // inspectors can share this lane. Keeping it above y=232 prevents the
    // 1024px-floor minimap (bottom-right) from obscuring its lower fields.
    commands
        .spawn((
            Node {
                right: Val::Px(10.0),
                top: Val::Px(52.0),
                ..ui_panel_node(Val::Px(300.0))
            },
            ui_panel_frame(),
            BuildingInspectorPanel,
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(ui_title_bar("Building"));
            panel.spawn(ui_panel_body()).with_children(|body| {
                body.spawn((ui_text("", FS_BODY, UI_INK), BuildingInspectorText));
                body.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(UI_GAP_TIGHT),
                        display: Display::None,
                        ..default()
                    },
                    StationQueueControls,
                ))
                .with_children(|controls| {
                    controls.spawn((ui_text("", FS_SMALL, UI_MUTED), StationQueueText));
                    controls.spawn(bottom_bar_row_node()).with_children(|row| {
                        for (kind, label) in [
                            (StationQueueButton::Add, "+ cut logs"),
                            (StationQueueButton::SelectNext, "next"),
                            (StationQueueButton::MoveUp, "up"),
                            (StationQueueButton::MoveDown, "down"),
                            (StationQueueButton::Remove, "remove"),
                            (StationQueueButton::ToggleRepeat, "repeat"),
                            (StationQueueButton::TogglePause, "pause"),
                        ] {
                            row.spawn((
                                ui_button_small(),
                                kind,
                                children![ui_text(label, FS_SMALL, UI_INK)],
                            ));
                        }
                    });
                });
            });
        });

    // Officers panel (left, below the dashboard), toggled with `O`.
    spawn_officers_panel(&mut commands);
    spawn_orders_panel(&mut commands);

    // Bottom command bar (tool modes + player actions, one framed strip).
    spawn_bottom_bar(&mut commands);
}

fn spawn_officers_panel(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                // Keep the optional roster clear of both left-column panels and
                // the bottom command bar. At the supported 1024x768 floor this
                // centre-left sheet has enough vertical room for all seven rows;
                // wider windows retain the same compact, predictable placement.
                // At the 1024px floor the Dispatches panel ends at x=440.
                // Keep this optional sheet in the clear centre lane instead of
                // letting equal-z UI panels occlude its upper rows.
                left: Val::Px(450.0),
                top: Val::Px(128.0),
                ..ui_panel_node(Val::Px(300.0))
            },
            ui_panel_frame(),
            OfficersPanel,
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(ui_title_bar("Officers  [O]"));
            panel.spawn(ui_panel_body()).with_children(|body| {
                for role in ALL_OFFICER_ROLES {
                    body.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(UI_GAP),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Node {
                                flex_grow: 1.0,
                                ..default()
                            },
                            ui_text("", FS_BODY, UI_INK),
                            OfficerRow(role),
                        ));
                        row.spawn((
                            ui_button_small(),
                            VacateButton(role),
                            children![ui_text("x", FS_SMALL, UI_INK)],
                        ));
                    });
                }
            });
        });
}

fn spawn_orders_panel(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                left: Val::Px(342.0),
                top: Val::Px(128.0),
                ..ui_panel_node(Val::Px(660.0))
            },
            ui_panel_frame(),
            OrdersPanel,
            WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(ui_title_bar("Manual Orders  [P]"));
            panel.spawn(ui_panel_body()).with_children(|body| {
                body.spawn(ui_text("Field work", FS_SMALL, UI_MUTED));
                body.spawn(bottom_bar_row_node()).with_children(|row| {
                    for action in OrderAction::JOBS {
                        row.spawn((
                            ui_button_small(),
                            OrderButton(action),
                            children![ui_text(action.label(), FS_SMALL, UI_INK)],
                        ));
                    }
                });
                body.spawn(ui_text("Shrine, stores & cats", FS_SMALL, UI_MUTED));
                body.spawn(bottom_bar_row_node()).with_children(|row| {
                    for action in OrderAction::TARGETS {
                        row.spawn((
                            ui_button_small(),
                            OrderButton(action),
                            children![ui_text(action.label(), FS_SMALL, UI_INK)],
                        ));
                    }
                });
                body.spawn(ui_text("Construction", FS_SMALL, UI_MUTED));
                body.spawn(bottom_bar_row_node()).with_children(|row| {
                    row.spawn((
                        ui_button_small(),
                        CycleOrderBuilding,
                        children![(ui_text("", FS_SMALL, UI_INK), PlannedBuildingText)],
                    ));
                    row.spawn((
                        ui_button_small(),
                        OrderButton(OrderAction::PlanBuilding),
                        children![ui_text("Plan selected type", FS_SMALL, UI_INK)],
                    ));
                    row.spawn(ui_text(
                        "Choose Building tool, then click its north-west map tile",
                        FS_SMALL,
                        UI_MUTED,
                    ));
                });
                body.spawn(ui_text("Village election", FS_SMALL, UI_MUTED));
                body.spawn(bottom_bar_row_node()).with_children(|row| {
                    row.spawn((
                        ui_button_small(),
                        CycleElectionCandidate,
                        children![(
                            ui_text("No active election", FS_SMALL, UI_INK),
                            ElectionCandidateText
                        )],
                    ));
                    row.spawn((
                        ui_button_small(),
                        CastElectionVoteButton,
                        children![ui_text("Cast vote", FS_SMALL, UI_INK)],
                    ));
                    row.spawn((
                        ui_button_small(),
                        RequestVoteKickButton,
                        children![(
                            ui_text("Request vote-kick", FS_SMALL, UI_INK),
                            VoteKickButtonText
                        )],
                    ));
                });
            });
        });
}

/// The bottom command bar: tool modes + the stockpile accept-picker on the top
/// row, and the player action buttons below — all inside ONE framed strip so the
/// controls read as a single designed toolbar. Kit buttons; the active tool
/// stays lit via its [`KitToggle`].
fn bottom_bar_panel_node() -> Node {
    Node {
        width: Val::Percent(96.0),
        max_width: Val::Px(1180.0),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(UI_GAP),
        padding: UiRect::all(Val::Px(UI_GAP)),
        border: UiRect::all(Val::Px(UI_BORDER_W)),
        border_radius: BorderRadius::all(Val::Px(UI_RADIUS)),
        align_items: AlignItems::Center,
        ..default()
    }
}

fn bottom_bar_row_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        flex_wrap: FlexWrap::Wrap,
        justify_content: JustifyContent::Center,
        column_gap: Val::Px(UI_GAP),
        row_gap: Val::Px(UI_GAP),
        ..default()
    }
}

fn spawn_bottom_bar(commands: &mut Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(0.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|center| {
            center
                .spawn((bottom_bar_panel_node(), ui_panel_frame(), WorldInputBlocker))
                .with_children(|bar| {
                    // Tool-mode row + accept picker.
                    bar.spawn(bottom_bar_row_node()).with_children(|row| {
                        for mode in [
                            ToolMode::Inspect,
                            ToolMode::AvoidZone,
                            ToolMode::GatherZone,
                            ToolMode::Stockpile,
                            ToolMode::Farm,
                            ToolMode::GatherSpot,
                            ToolMode::FishingSpot,
                            ToolMode::Road,
                            ToolMode::Building,
                        ] {
                            row.spawn((
                                ui_button(),
                                ToolButton(mode),
                                KitToggle::default(),
                                children![ui_text(mode.label(), FS_BODY, UI_INK)],
                            ));
                        }
                        row.spawn((
                            ui_button(),
                            AcceptButton,
                            children![(
                                ui_text("Accepts: General", FS_BODY, UI_INK),
                                AcceptButtonText,
                            )],
                        ));
                        row.spawn((
                            ui_button(),
                            CycleFarmCrop,
                            children![(ui_text("Crop: grain", FS_BODY, UI_INK), FarmCropText)],
                        ));
                        row.spawn((
                            ui_button(),
                            CycleGatherKind,
                            children![(
                                ui_text("Gather: materials", FS_BODY, UI_INK),
                                GatherKindText
                            )],
                        ));
                    });
                    // Player action row.
                    bar.spawn(bottom_bar_row_node()).with_children(|row| {
                        for action in [
                            ButtonAction::SupplyFood,
                            ButtonAction::SupplyWater,
                            ButtonAction::PlanHunt,
                            ButtonAction::ScoutWood,
                            ButtonAction::ScoutFood,
                            ButtonAction::ScoutWater,
                            ButtonAction::ScoutStone,
                            ButtonAction::Explore,
                            ButtonAction::FoundVillage,
                        ] {
                            row.spawn((
                                ui_button(),
                                ActionButton(action),
                                KitDisabled { disabled: true },
                                children![ui_text(action.label(), FS_BODY, UI_INK)],
                            ));
                        }
                    });
                });
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
    begin_connection(world);
}

fn begin_connection(world: &mut World) {
    let url = server_ws_url();
    world.resource_mut::<ConnectionState>().phase = ConnectionPhase::Connecting;
    match ewebsock::connect(url.clone(), ewebsock::Options::default()) {
        Ok((sender, receiver)) => {
            info!("cat-client connecting to {url}");
            world.insert_non_send(WsConn { sender, receiver });
        }
        Err(err) => {
            error!("cat-client failed to connect to {url}: {err}");
            schedule_reconnect(world, format!("Connection failed: {err}"));
        }
    }
}

/// Drain socket messages: `WorldSnapshot`s update the render, action results
/// carry the signed session after a `Presence` handshake.
fn poll_ws(world: &mut World) {
    let Some(conn) = world.get_non_send::<WsConn>() else {
        return;
    };
    let mut events = Vec::new();
    while let Some(event) = conn.receiver.try_recv() {
        events.push(event);
    }

    for event in events {
        match event {
            WsEvent::Opened => {
                let was_retry = world.resource::<ConnectionState>().retry_attempt > 0;
                {
                    let mut state = world.resource_mut::<ConnectionState>();
                    state.phase = ConnectionPhase::Connected;
                    state.retry_attempt = 0;
                    state.retry_remaining_secs = 0.0;
                }
                set_feedback(
                    world,
                    if was_retry {
                        "Reconnected to colony server."
                    } else {
                        "Connected to colony server."
                    },
                    FeedbackLevel::Info,
                );
            }
            WsEvent::Message(WsMessage::Text(text)) => match parse_server_message(&text) {
                Ok(ServerPayload::Snapshot(mut snapshot)) => {
                    let allow_missing_fallback = world.resource::<Session>().ready;
                    let fallback = {
                        let mut selection = world.resource_mut::<VillageSelection>();
                        reconcile_village_selection(
                            &mut snapshot,
                            &mut selection,
                            allow_missing_fallback,
                        )
                    };
                    world.resource_mut::<LatestSnapshot>().0 = Some(snapshot);
                    if let Some((missing, fallback)) = fallback {
                        let persist_error = {
                            let session = world.resource::<Session>();
                            let selection = world.resource::<VillageSelection>();
                            persist_session(session, selection).err()
                        };
                        if let Some(err) = persist_error {
                            warn!("could not persist fallback village: {err}");
                        }
                        let message = format!(
                            "Village {missing} is no longer available; showing {fallback}."
                        );
                        push_client_alert(world, message.clone());
                        set_feedback(world, message, FeedbackLevel::Info);
                    }
                }
                Ok(ServerPayload::Action {
                    result,
                    signed_session,
                }) => {
                    if let Some((session_id, sig)) = signed_session {
                        {
                            let mut session = world.resource_mut::<Session>();
                            session.session_id = session_id;
                            session.sig = sig;
                            session.ready = true;
                        }
                        let session = world.resource::<Session>();
                        let selection = world.resource::<VillageSelection>();
                        let persist_error = persist_session(session, selection).err();
                        if let Some(err) = persist_error {
                            warn!("could not persist player session: {err}");
                        }
                    }
                    if !result.ok {
                        let message = result
                            .message
                            .unwrap_or_else(|| "The server rejected that action.".to_string());
                        let visible = format!("Action failed: {message}");
                        push_client_alert(world, visible.clone());
                        set_feedback(world, visible, FeedbackLevel::Error);
                    } else if let Some(message) = result.message {
                        set_feedback(world, message, FeedbackLevel::Info);
                    }
                }
                Err(err) => warn!("bad ws message: {err}"),
            },
            WsEvent::Message(_) => {}
            WsEvent::Error(err) => {
                schedule_reconnect(world, format!("Connection error: {err}"));
                break;
            }
            WsEvent::Closed => {
                schedule_reconnect(world, "Connection closed.".to_string());
                break;
            }
        }
    }
}

#[derive(Debug)]
enum ServerPayload {
    Snapshot(WorldSnapshot),
    Action {
        result: ActionResult,
        signed_session: Option<(String, String)>,
    },
}

fn parse_server_message(text: &str) -> Result<ServerPayload, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|err| err.to_string())?;
    if value.get("colonies").is_some() {
        return serde_json::from_value(value)
            .map(ServerPayload::Snapshot)
            .map_err(|err| err.to_string());
    }
    if value.get("ok").is_none() {
        return Err("message was neither a snapshot nor an action result".to_string());
    }
    let signed_session = match (
        value.get("sessionId").and_then(|field| field.as_str()),
        value.get("sig").and_then(|field| field.as_str()),
    ) {
        (Some(session_id), Some(sig)) => Some((session_id.to_string(), sig.to_string())),
        _ => None,
    };
    serde_json::from_value::<ActionResult>(value)
        .map(|result| ServerPayload::Action {
            result,
            signed_session,
        })
        .map_err(|err| err.to_string())
}

/// Keep the selected village at index zero because the existing render/UI
/// systems consume `colonies.first()`. Returns the missing/fallback ids when a
/// persisted selection disappeared from the shared world.
fn reconcile_village_selection(
    snapshot: &mut WorldSnapshot,
    selection: &mut VillageSelection,
    allow_missing_fallback: bool,
) -> Option<(String, String)> {
    let Some(first) = snapshot.colonies.first() else {
        selection.selected_id = None;
        selection.join_required = false;
        return None;
    };

    let fallback_id = first.id.clone();
    let Some(selected_id) = selection.selected_id.clone() else {
        selection.selected_id = Some(fallback_id);
        return None;
    };
    if let Some(index) = snapshot
        .colonies
        .iter()
        .position(|colony| colony.id == selected_id)
    {
        snapshot.colonies.swap(0, index);
        return None;
    }

    // The server intentionally sends a public/global-only snapshot before
    // Presence. Preserve an in-memory personal selection until the durable
    // identity has been restored and the personalized snapshot arrives.
    if !allow_missing_fallback {
        return None;
    }

    selection.selected_id = Some(fallback_id.clone());
    selection.join_required = false;
    Some((selected_id, fallback_id))
}

fn join_village_action(colony_id: &str, session: &Session) -> Option<ClientAction> {
    session.ready.then(|| ClientAction::JoinVillage {
        colony_id: colony_id.to_owned(),
        session_id: session.session_id.clone(),
        sig: Some(session.sig.clone()),
    })
}

/// Select a village immediately for local rendering and, when authenticated,
/// return the server action that moves this socket's mutation target to it.
fn choose_village(
    colony_id: &str,
    snapshot: &mut WorldSnapshot,
    selection: &mut VillageSelection,
    session: &Session,
) -> Option<ClientAction> {
    let index = snapshot
        .colonies
        .iter()
        .position(|colony| colony.id == colony_id)?;
    snapshot.colonies.swap(0, index);
    selection.selected_id = Some(colony_id.to_owned());
    let action = join_village_action(colony_id, session);
    selection.join_required = action.is_none();
    action
}

fn reconnect_delay_secs(attempt: u32) -> f32 {
    let exponent = attempt.saturating_sub(1).min(5);
    (2_u32.pow(exponent) as f32).min(MAX_RECONNECT_DELAY_SECS)
}

fn schedule_reconnect(world: &mut World, reason: String) {
    world.remove_non_send::<WsConn>();
    {
        let mut session = world.resource_mut::<Session>();
        session.presence_sent = false;
        session.ready = false;
    }
    {
        let mut selection = world.resource_mut::<VillageSelection>();
        selection.join_required = selection.selected_id.is_some();
    }
    let (attempt, delay) = {
        let mut state = world.resource_mut::<ConnectionState>();
        state.retry_attempt = state.retry_attempt.saturating_add(1);
        state.retry_remaining_secs = reconnect_delay_secs(state.retry_attempt);
        state.phase = ConnectionPhase::WaitingToRetry;
        (state.retry_attempt, state.retry_remaining_secs)
    };
    let message = format!("{reason} Retrying in {delay:.0}s (attempt {attempt}).");
    push_client_alert(world, message.clone());
    set_feedback(world, message, FeedbackLevel::Error);
}

fn reconnect_ws(world: &mut World) {
    let delta = world.resource::<Time>().delta_secs();
    let should_reconnect = {
        let mut state = world.resource_mut::<ConnectionState>();
        if state.phase != ConnectionPhase::WaitingToRetry {
            false
        } else {
            state.retry_remaining_secs = (state.retry_remaining_secs - delta).max(0.0);
            state.retry_remaining_secs == 0.0
        }
    };
    if should_reconnect {
        begin_connection(world);
    }
}

fn set_feedback(world: &mut World, message: impl Into<String>, level: FeedbackLevel) {
    let mut feedback = world.resource_mut::<ClientFeedback>();
    feedback.message = Some(message.into());
    feedback.level = level;
    feedback.remaining_secs = match level {
        FeedbackLevel::Info => 3.0,
        FeedbackLevel::Error => 8.0,
    };
}

fn push_client_alert(world: &mut World, message: String) {
    let mut alerts = world.resource_mut::<ClientAlerts>();
    alerts.0.push_front(message);
    alerts.0.truncate(CLIENT_ALERT_CAP);
}

/// Send the `Presence` handshake once so the server issues a signed session.
fn ensure_presence(
    conn: Option<NonSendMut<WsConn>>,
    state: Res<ConnectionState>,
    mut session: ResMut<Session>,
) {
    let Some(mut conn) = conn else {
        return;
    };
    if state.phase != ConnectionPhase::Connected || session.presence_sent {
        return;
    }
    let action = presence_action(&session);
    if let Ok(json) = serde_json::to_string(&action) {
        conn.sender.send(WsMessage::Text(json));
        session.presence_sent = true;
    }
}

fn presence_action(session: &Session) -> ClientAction {
    ClientAction::Presence {
        session_id: if session.session_id.is_empty() {
            "desktop".to_owned()
        } else {
            session.session_id.clone()
        },
        nickname: "Desktop Cat".to_string(),
        sig: (!session.sig.is_empty()).then(|| session.sig.clone()),
    }
}

/// Restore the persisted village on a fresh socket after Presence supplies its
/// new authenticated session. This is intentionally separate from founding:
/// creating another village never silently changes the player's selected map.
fn restore_village_selection(
    session: Res<Session>,
    mut selection: ResMut<VillageSelection>,
    latest: Res<LatestSnapshot>,
    mut outgoing: ResMut<OutgoingActions>,
) {
    if let Some(action) = pending_village_rejoin(latest.0.as_ref(), &mut selection, &session) {
        outgoing.0.push(action);
    }
}

fn pending_village_rejoin(
    snapshot: Option<&WorldSnapshot>,
    selection: &mut VillageSelection,
    session: &Session,
) -> Option<ClientAction> {
    if !selection.join_required || !session.ready {
        return None;
    }
    let Some(selected_id) = selection.selected_id.clone() else {
        selection.join_required = false;
        return None;
    };
    if !snapshot.is_some_and(|snapshot| {
        snapshot
            .colonies
            .iter()
            .any(|colony| colony.id == selected_id)
    }) {
        return None;
    }
    let action = join_village_action(&selected_id, session);
    if action.is_some() {
        selection.join_required = false;
    }
    action
}

/// Stream deterministic terrain chunks around the camera. The world is
/// unbounded: panning (including a minimap jump) swaps in the new local chunks
/// and drops chunks beyond a one-chunk retention margin.
fn spawn_terrain(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    art: Option<Res<TerrainArt>>,
    mut render: ResMut<WorldRender>,
    camera: Query<&Transform, With<WorldCamera>>,
    visuals: Query<(Entity, &TerrainVisual)>,
) {
    let (Some(world), Some(art)) = (latest.0.as_ref(), art) else {
        return;
    };
    let seed = world.world_seed;
    // `reconcile_village_selection` keeps the actively viewed village first.
    let village_anchor = world.colonies.first().map(|colony| colony.anchor);
    let Ok(camera) = camera.single() else {
        return;
    };
    let camera_tile = world_to_tile(camera.translation.truncate());
    let center = tile_to_chunk(camera_tile.0, camera_tile.1);
    let center = ChunkKey {
        x: center.chunk_x,
        y: center.chunk_y,
    };

    if render.world_seed != Some(seed) {
        for (entity, _) in &visuals {
            commands.entity(entity).despawn();
        }
        render.loaded_chunks.clear();
        render.world_seed = Some(seed);
    }

    let retained = chunks_around(center, TERRAIN_RETAIN_RADIUS);
    let expired: HashSet<ChunkKey> = render
        .loaded_chunks
        .difference(&retained)
        .copied()
        .collect();
    if !expired.is_empty() {
        for (entity, visual) in &visuals {
            if expired.contains(&visual.0) {
                commands.entity(entity).despawn();
            }
        }
        render
            .loaded_chunks
            .retain(|chunk| !expired.contains(chunk));
    }

    let desired = chunks_around(center, TERRAIN_CHUNK_RADIUS);
    let needed: HashSet<ChunkKey> = desired.difference(&render.loaded_chunks).copied().collect();
    if needed.is_empty() {
        return;
    }

    let (tiles, water) = terrain_for_chunks(seed, &needed);
    // Water coordinates (river overlay OR a water climate biome), so shore tiles
    // (a non-water orthogonal neighbour) can use the water_edge variant.
    for tile in &tiles {
        let p = grid_to_world(tile.x, tile.y);
        let chunk = chunk_for_tile(tile.x, tile.y);
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
            TerrainVisual(chunk),
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
                let width = TREE_FOOTPRINT_WIDTH as f32;
                let height = TREE_FOOTPRINT_HEIGHT as f32;
                let center = Vec2::new(
                    p.x + TILE * (width - 1.0) * 0.5,
                    p.y - TILE * (height - 1.0) * 0.5,
                );
                let role = decoration.expect("matched tree decoration");
                commands.spawn((
                    Sprite {
                        image: tree,
                        custom_size: Some(Vec2::new(TILE * width, TILE * height) * scale.min(1.0)),
                        ..default()
                    },
                    Anchor::CENTER,
                    Transform::from_xyz(center.x, center.y, ysort_z(center.y)),
                    TerrainVisual(chunk),
                    TerrainDecoration {
                        x: tile.x,
                        y: tile.y,
                        role,
                    },
                    if village_anchor.is_none_or(|anchor| {
                        procedural_decoration_visible(anchor, tile.x, tile.y, role)
                    }) {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                ));
            }
            Some(DecorationRole::Rock { size, .. }) => {
                let role = decoration.expect("matched rock decoration");
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
                    TerrainVisual(chunk),
                    TerrainDecoration {
                        x: tile.x,
                        y: tile.y,
                        role,
                    },
                    if village_anchor.is_none_or(|anchor| {
                        procedural_decoration_visible(anchor, tile.x, tile.y, role)
                    }) {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                ));
            }
            None => {}
        }
    }
    let loaded = needed.len();
    render.loaded_chunks.extend(needed);
    debug!(
        "terrain streamed (seed {seed}, {loaded} chunks, {} tiles)",
        tiles.len()
    );
}

/// Re-evaluate already-streamed decorations whenever the snapshot changes.
/// This matters both when a claim expands and when the village selector swaps a
/// reordered shared-world snapshot's active colony into slot zero.
fn sync_terrain_decoration_visibility(
    latest: Res<LatestSnapshot>,
    mut decorations: Query<(&TerrainDecoration, &mut Visibility)>,
) {
    if !latest.is_changed() {
        return;
    }
    let Some(colony) = latest.0.as_ref().and_then(|world| world.colonies.first()) else {
        return;
    };
    for (tile, mut visibility) in &mut decorations {
        *visibility = if procedural_decoration_visible(colony.anchor, tile.x, tile.y, tile.role) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Ground remains visible everywhere. Procedural nature/resource props clear
/// only inside the fixed founding wall core; expanded claimed farm/resource
/// territory beyond that core deliberately retains its wilderness props.
fn procedural_decoration_visible(anchor: TilePoint, x: i32, y: i32, role: DecorationRole) -> bool {
    let center_x = anchor.x.saturating_add(1);
    let center_y = anchor.y.saturating_add(1);
    decoration_footprint(x, y, role).into_iter().all(|tile| {
        tile.x.abs_diff(center_x).max(tile.y.abs_diff(center_y)) > VILLAGE_INTERIOR_RADIUS
    })
}

fn chunk_for_tile(x: i32, y: i32) -> ChunkKey {
    let chunk = tile_to_chunk(x, y);
    ChunkKey {
        x: chunk.chunk_x,
        y: chunk.chunk_y,
    }
}

fn chunks_around(center: ChunkKey, radius: i32) -> HashSet<ChunkKey> {
    let mut chunks = HashSet::with_capacity(((radius * 2 + 1).pow(2)) as usize);
    for y in center.y - radius..=center.y + radius {
        for x in center.x - radius..=center.x + radius {
            chunks.insert(ChunkKey { x, y });
        }
    }
    chunks
}

fn expanded_chunks(chunks: &HashSet<ChunkKey>, radius: i32) -> HashSet<ChunkKey> {
    let mut expanded = HashSet::new();
    for chunk in chunks {
        expanded.extend(chunks_around(*chunk, radius));
    }
    expanded
}

/// Generate requested chunks plus a one-chunk halo used only for correct shore
/// classification at chunk seams.
fn terrain_for_chunks(
    seed: i64,
    requested: &HashSet<ChunkKey>,
) -> (Vec<TerrainTile>, HashSet<(i32, i32)>) {
    let mut generation: Vec<ChunkKey> = expanded_chunks(requested, 1).into_iter().collect();
    generation.sort_by_key(|chunk| (chunk.y, chunk.x));

    let mut tiles = Vec::new();
    let mut water = HashSet::new();
    for chunk in generation {
        for tile in generate_terrain_chunk(chunk.x, chunk.y, seed, WORLD_TERRAIN_OPTIONS) {
            if tile.river.is_some() || is_water_biome(tile.climate_biome) {
                water.insert((tile.x, tile.y));
            }
            if requested.contains(&chunk) {
                tiles.push(tile);
            }
        }
    }
    (tiles, water)
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

/// Fog of war over the currently streamed chunks. Existing fog entities are
/// updated incrementally rather than rebuilding thousands of sprites per
/// snapshot, and follow the camera along with terrain.
fn render_fog(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    render: Res<WorldRender>,
    mut fog: Query<(Entity, &FogTile, &mut Sprite)>,
) {
    if !latest.is_changed() && !render.is_changed() {
        return;
    }
    let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) else {
        for (entity, _, _) in &mut fog {
            commands.entity(entity).despawn();
        }
        return;
    };
    let revealed = revealed_lookup(&colony.revealed_tiles);
    // Self-disabling fallback: with no revealed tiles (a pre-fog snapshot, or a
    // colony whose reveal state isn't populated yet) fogging the whole window
    // would black out the map — so show the full map until the set is non-empty.
    // Once the sim emits a non-empty revealed set, fog kicks in normally.
    if revealed.is_empty() {
        for (entity, _, _) in &mut fog {
            commands.entity(entity).despawn();
        }
        return;
    }
    let provisional = revealed_lookup(&colony.provisional_tiles);
    // One fog chunk beyond terrain hides overhanging tree/rock sprites at the
    // streaming seam; the clear colour matches full fog beyond that halo.
    let fog_chunks = expanded_chunks(&render.loaded_chunks, 1);
    let mut existing = HashSet::new();
    for (entity, tile, mut sprite) in &mut fog {
        let chunk = chunk_for_tile(tile.x, tile.y);
        let state = fog_state(&revealed, &provisional, tile.x, tile.y);
        if !fog_chunks.contains(&chunk) || state == FogState::Clear {
            commands.entity(entity).despawn();
            continue;
        }
        sprite.color = match state {
            FogState::Clear => unreachable!("clear fog entities are despawned"),
            FogState::Dim => PROVISIONAL_FOG_COLOR,
            FogState::Full => FOG_COLOR,
        };
        existing.insert((tile.x, tile.y));
    }

    for chunk in &fog_chunks {
        let x0 = chunk.x * TERRAIN_CHUNK_SIZE;
        let y0 = chunk.y * TERRAIN_CHUNK_SIZE;
        for y in y0..y0 + TERRAIN_CHUNK_SIZE {
            for x in x0..x0 + TERRAIN_CHUNK_SIZE {
                if existing.contains(&(x, y)) {
                    continue;
                }
                let color = match fog_state(&revealed, &provisional, x, y) {
                    FogState::Clear => continue,
                    FogState::Dim => PROVISIONAL_FOG_COLOR,
                    FogState::Full => FOG_COLOR,
                };
                let p = grid_to_world(x, y);
                commands.spawn((
                    Sprite::from_color(color, Vec2::splat(TILE)),
                    Transform::from_xyz(p.x, p.y, Z_FOG),
                    FogTile { x, y },
                ));
            }
        }
    }
}

/// Authored stone roads and traffic-formed dirt roads. Both use the connected
/// road grammar, but receive deliberately cool-stone and warm-earth tints so
/// their gameplay meaning stays legible even at the default camera zoom.
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
    let stone_set: HashSet<(i32, i32)> = colony.road_tiles.iter().map(|t| (t.x, t.y)).collect();
    let dirt_set: HashSet<(i32, i32)> = colony.dirt_road_tiles.iter().map(|t| (t.x, t.y)).collect();
    let road_set: HashSet<(i32, i32)> = stone_set.union(&dirt_set).copied().collect();
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
                color: if dirt_set.contains(&(x, y)) {
                    Color::srgb(0.72, 0.40, 0.16)
                } else {
                    Color::srgb(0.48, 0.58, 0.72)
                },
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
        // Walls are rendered by render_village_boundary. True residences retain
        // a roof; every workplace/civic/supply destination exposes a tiled floor
        // and depth-sorted props instead of pretending to be a house facade.
        // Names remain available through hover/click inspectors without covering
        // the settlement in persistent world-space text.
        let complete = building.construction_progress >= 100.0;
        match building_visual(building.building_type) {
            BuildingVisual::Infrastructure => {}
            BuildingVisual::Roofed(facade) => {
                spawn_station_floor(
                    &mut commands,
                    &art,
                    building.world_position,
                    building.footprint,
                    StationFloor::Wood,
                    complete,
                );
                let layout = building_render_layout(building.world_position, building.footprint);
                commands.spawn((
                    Sprite {
                        image: art.facade(facade),
                        color: building_sprite_color(building.building_type, complete),
                        custom_size: Some(layout.facade_size),
                        ..default()
                    },
                    Anchor::BOTTOM_CENTER,
                    Transform::from_xyz(
                        layout.facade_base.x,
                        layout.facade_base.y,
                        ysort_z(layout.facade_base.y) + 0.2,
                    ),
                    BuildingSprite,
                    RoofedBuildingSprite,
                ));
            }
            BuildingVisual::Open(station) => spawn_open_station(
                &mut commands,
                &art,
                building.world_position,
                building.footprint,
                station,
                complete,
            ),
        }
    }
    for farm in &colony.farms {
        spawn_farm_plot(&mut commands, &art, farm);
    }
}

/// Render every designated farm as an exposed soil rectangle with one crop per
/// tile at its live growth stage. Crop-specific tint distinguishes catnip,
/// grain, and herbs without adding persistent map labels.
fn spawn_farm_plot(commands: &mut Commands, art: &BuildingArt, farm: &cat_protocol::FarmSnapshot) {
    let (x0, x1) = (farm.x1.min(farm.x2), farm.x1.max(farm.x2));
    let (y0, y1) = (farm.y1.min(farm.y2), farm.y1.max(farm.y2));
    let crop_prop = farm_stage_prop(farm.stage);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let tile = TilePoint { x, y };
            spawn_station_floor(
                commands,
                art,
                tile,
                FootprintSize {
                    width: 1,
                    height: 1,
                },
                StationFloor::Soil,
                true,
            );
            let Some(prop) = crop_prop else {
                continue;
            };
            let geometry = station_prop_geometry(
                tile,
                FootprintSize {
                    width: 1,
                    height: 1,
                },
                PropPlacement {
                    prop,
                    x: 500,
                    y: 500,
                },
            );
            commands.spawn((
                Sprite {
                    image: art.prop(prop),
                    color: farm_crop_tint(farm.crop),
                    custom_size: Some(geometry.size * 0.82),
                    ..default()
                },
                Transform::from_xyz(
                    geometry.center.x,
                    geometry.center.y,
                    ysort_z(geometry.base_y) + 0.2,
                ),
                BuildingSprite,
                FarmPlotSprite,
            ));
        }
    }
}

fn farm_stage_prop(stage: FarmStage) -> Option<StationProp> {
    match stage {
        FarmStage::Soil => None,
        FarmStage::Sprout => Some(StationProp::CropSprout),
        FarmStage::Growing => Some(StationProp::CropGrowing),
        FarmStage::Mature => Some(StationProp::CropMature),
        FarmStage::Flowering => Some(StationProp::CropFlowering),
    }
}

fn farm_crop_tint(crop: CropKind) -> Color {
    match crop {
        CropKind::Catnip => Color::srgb(0.78, 0.60, 0.92),
        CropKind::Grain => Color::srgb(0.96, 0.78, 0.34),
        CropKind::Herb => Color::srgb(0.55, 0.88, 0.48),
    }
}

fn spawn_station_floor(
    commands: &mut Commands,
    art: &BuildingArt,
    nw: TilePoint,
    footprint: FootprintSize,
    floor: StationFloor,
    complete: bool,
) {
    let tint = construction_tint(complete);
    for dy in 0..footprint.height.max(1) {
        for dx in 0..footprint.width.max(1) {
            let center = grid_to_world(nw.x + dx, nw.y + dy);
            commands.spawn((
                Sprite {
                    image: art.floor(floor),
                    color: tint,
                    custom_size: Some(Vec2::splat(TILE)),
                    ..default()
                },
                Transform::from_xyz(center.x, center.y, Z_BUILDING_FLOOR),
                BuildingSprite,
                StationFloorSprite,
            ));
        }
    }
}

fn spawn_open_station(
    commands: &mut Commands,
    art: &BuildingArt,
    nw: TilePoint,
    footprint: FootprintSize,
    station: &StationLayout,
    complete: bool,
) {
    spawn_station_floor(commands, art, nw, footprint, station.floor, complete);
    let tint = construction_tint(complete);
    for placement in station.props {
        let geometry = station_prop_geometry(nw, footprint, *placement);
        commands.spawn((
            Sprite {
                image: art.prop(placement.prop),
                color: tint,
                custom_size: Some(geometry.size),
                ..default()
            },
            Transform::from_xyz(
                geometry.center.x,
                geometry.center.y,
                ysort_z(geometry.base_y) + 0.2,
            ),
            BuildingSprite,
            StationPropSprite,
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
    let gate_edge = colony
        .village_gate
        .map(|g| ((g.x, g.y), gate_side_to_wall(g.side)));
    let wall_segments = if colony.wall_segments.is_empty() {
        // Legacy snapshots predate authoritative staged-wall edges.
        let agricultural = colony
            .agricultural_tiles
            .iter()
            .map(|tile| (tile.x, tile.y))
            .collect::<HashSet<_>>();
        let claimed = colony
            .claimed_tiles
            .iter()
            .map(|tile| (tile.x, tile.y))
            .filter(|tile| !agricultural.contains(tile))
            .collect::<HashSet<_>>();
        wall_edges(&claimed, gate_edge)
            .into_iter()
            .map(|(tile, side)| (tile, side, false))
            .collect::<Vec<_>>()
    } else {
        colony
            .wall_segments
            .iter()
            .map(|segment| {
                (
                    (segment.x, segment.y),
                    gate_side_to_wall(segment.side),
                    segment.under_construction,
                )
            })
            .collect()
    };

    for (tile, side, newly_built) in wall_segments {
        let (pos, rot) = wall_edge_transform(tile, side);
        commands.spawn((
            Sprite {
                image: art.palisade.clone(),
                color: if newly_built {
                    Color::srgb(1.0, 0.82, 0.48)
                } else {
                    Color::WHITE
                },
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
const FISHING_FLAG_COLOR: Color = Color::srgb(0.18, 0.68, 0.88);
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
        let is_shrine = is_seeded_store(&pile.id);
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
                font_size: FontSize::Px(5.0),
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
                Sprite::from_color(
                    if gs.purpose == GatherSpotPurpose::Fishing {
                        FISHING_FLAG_COLOR
                    } else {
                        GATHER_FLAG_COLOR
                    },
                    Vec2::splat(TILE * 0.66),
                ),
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
    let colony = latest.0.as_ref().and_then(|w| w.colonies.first());
    let pile = selection
        .selected
        .as_deref()
        .and_then(|id| colony.and_then(|c| c.stockpiles.iter().find(|s| s.id == id)));
    let farm = selection
        .selected_farm
        .as_deref()
        .and_then(|id| colony.and_then(|c| c.farms.iter().find(|farm| farm.id == id)));
    match (pile, farm) {
        (Some(pile), _) => {
            node.display = Display::Flex;
            let total = resource_total(&pile.contents);
            let dominant = dominant_resource(&pile.contents).map_or("empty", resource_kind_name);
            let title = match pile.gather_spot.as_ref().map(|spot| spot.purpose) {
                Some(GatherSpotPurpose::Fishing) => "Fishing shore",
                Some(GatherSpotPurpose::General) => "Gather spot",
                None => "Stockpile",
            };
            text.0 = format!("{title}\n{dominant} {}", total.round() as i64);
        }
        (_, Some(farm)) => {
            node.display = Display::Flex;
            text.0 = format!("{} farm\n{:?}", crop_label(farm.crop), farm.stage);
        }
        (None, None) => {
            node.display = Display::None;
            if selection.selected.is_some() {
                selection.selected = None;
            }
            if selection.selected_farm.is_some() {
                selection.selected_farm = None;
            }
        }
    }
}

/// Send RemoveStockpile when the remove button is clicked.
fn handle_remove_button(
    session: Res<Session>,
    latest: Res<LatestSnapshot>,
    mut selection: ResMut<StockpileSelection>,
    mut outgoing: ResMut<OutgoingActions>,
    button: Query<&Interaction, (Changed<Interaction>, With<RemoveStockpileButton>)>,
) {
    for interaction in &button {
        if *interaction != Interaction::Pressed || !session.ready {
            continue;
        }
        if let Some(plot_id) = selection.selected_farm.take() {
            outgoing
                .0
                .push(build_remove_action(&session, Some(plot_id), None, false));
        } else if let Some(stockpile_id) = selection.selected.take() {
            let is_gather = latest
                .0
                .as_ref()
                .and_then(|world| world.colonies.first())
                .and_then(|colony| {
                    colony
                        .stockpiles
                        .iter()
                        .find(|pile| pile.id == stockpile_id)
                })
                .is_some_and(|pile| pile.gather_spot.is_some());
            // The selected snapshot is resolved in `update_remove_panel`; gather
            // spots need their dedicated action so in-flight mover jobs are freed.
            outgoing.0.push(build_remove_action(
                &session,
                None,
                Some(stockpile_id),
                is_gather,
            ));
        }
    }
}

fn build_remove_action(
    session: &Session,
    farm_id: Option<String>,
    stockpile_id: Option<String>,
    is_gather: bool,
) -> ClientAction {
    let session_id = session.session_id.clone();
    let nickname = "Desktop Cat".to_owned();
    let sig = session.sig.clone();
    if let Some(plot_id) = farm_id {
        ClientAction::ClearFarm {
            session_id,
            nickname,
            sig,
            plot_id,
        }
    } else if is_gather {
        ClientAction::RemoveGatherSpot {
            session_id,
            nickname,
            sig,
            stockpile_id: stockpile_id.expect("selected gather spot has an id"),
        }
    } else {
        ClientAction::RemoveStockpile {
            session_id,
            nickname,
            sig,
            stockpile_id: stockpile_id.expect("selected stockpile has an id"),
        }
    }
}

/// Toggle the officers panel with the `O` key.
fn toggle_officers(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui: ResMut<OfficersUi>,
    mut orders: ResMut<OrdersUi>,
) {
    if keys.just_pressed(OFFICERS_SHORTCUT) {
        ui.visible = !ui.visible;
        if ui.visible {
            orders.visible = false;
        }
    }
}

fn toggle_orders(
    keys: Res<ButtonInput<KeyCode>>,
    mut ui: ResMut<OrdersUi>,
    mut officers: ResMut<OfficersUi>,
) {
    if keys.just_pressed(ORDERS_SHORTCUT) {
        ui.visible = !ui.visible;
        if ui.visible {
            officers.visible = false;
        }
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

fn update_orders_panel(
    ui: Res<OrdersUi>,
    mut panel: Query<&mut Node, With<OrdersPanel>>,
    mut planned: Query<&mut Text, With<PlannedBuildingText>>,
) {
    if let Ok(mut node) = panel.single_mut() {
        node.display = if ui.visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if ui.is_changed()
        && let Ok(mut text) = planned.single_mut()
    {
        let building = PLANNABLE_BUILDINGS[ui.planned_building % PLANNABLE_BUILDINGS.len()];
        text.0 = format!("Type: {}  [cycle]", building_label(building));
    }
}

fn update_dispatches_panel(
    orders: Res<OrdersUi>,
    mut panel: Query<&mut Node, With<DispatchesPanel>>,
) {
    if !orders.is_changed() {
        return;
    }
    if let Ok(mut node) = panel.single_mut() {
        node.display = if orders.visible {
            Display::None
        } else {
            Display::Flex
        };
    }
}

fn handle_order_building_cycle(
    mut ui: ResMut<OrdersUi>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<CycleOrderBuilding>)>,
) {
    if buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        ui.planned_building = (ui.planned_building + 1) % PLANNABLE_BUILDINGS.len();
    }
}

fn update_governance_controls(
    latest: Res<LatestSnapshot>,
    mut governance: ResMut<GovernanceUi>,
    mut candidate_text: Query<&mut Text, With<ElectionCandidateText>>,
    mut kick_text: Query<&mut Text, (With<VoteKickButtonText>, Without<ElectionCandidateText>)>,
) {
    if !latest.is_changed() && !governance.is_changed() {
        return;
    }
    let colony = latest.0.as_ref().and_then(|world| world.colonies.first());
    if let Ok(mut text) = candidate_text.single_mut() {
        text.0 = colony
            .and_then(|colony| colony.election.as_ref())
            .and_then(|election| {
                if election.candidates.is_empty() {
                    return None;
                }
                governance.candidate_index %= election.candidates.len();
                let candidate = &election.candidates[governance.candidate_index];
                let votes = election.tally.get(&candidate.id).copied().unwrap_or(0);
                Some(format!("{}  ({votes} votes) [cycle]", candidate.name))
            })
            .unwrap_or_else(|| "No active election".to_owned());
    }
    if let Ok(mut text) = kick_text.single_mut() {
        text.0 = colony
            .and_then(|colony| colony.vote_kick.as_ref())
            .map(|kick| format!("Sign vote-kick {}/{}", kick.signatures, kick.needed))
            .unwrap_or_else(|| "Request vote-kick".to_owned());
    }
}

fn handle_governance_buttons(
    session: Res<Session>,
    latest: Res<LatestSnapshot>,
    mut governance: ResMut<GovernanceUi>,
    mut outgoing: ResMut<OutgoingActions>,
    cycle: Query<&Interaction, (Changed<Interaction>, With<CycleElectionCandidate>)>,
    cast: Query<&Interaction, (Changed<Interaction>, With<CastElectionVoteButton>)>,
    kick: Query<&Interaction, (Changed<Interaction>, With<RequestVoteKickButton>)>,
) {
    let Some(colony) = latest.0.as_ref().and_then(|world| world.colonies.first()) else {
        return;
    };
    if cycle
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
        && let Some(election) = &colony.election
        && !election.candidates.is_empty()
    {
        governance.candidate_index = (governance.candidate_index + 1) % election.candidates.len();
    }
    if !session.ready {
        return;
    }
    if cast
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
        && let Some(election) = &colony.election
        && let Some(candidate) = election
            .candidates
            .get(governance.candidate_index % election.candidates.len().max(1))
    {
        outgoing.0.push(ClientAction::CastVote {
            session_id: session.session_id.clone(),
            nickname: "Desktop Cat".to_owned(),
            sig: session.sig.clone(),
            election_id: election.id.clone(),
            cat_id: candidate.id.clone(),
        });
    }
    if kick
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
        && colony.leader.is_some()
    {
        outgoing.0.push(ClientAction::RequestVoteKick {
            session_id: session.session_id.clone(),
            nickname: "Desktop Cat".to_owned(),
            sig: session.sig.clone(),
        });
    }
}

fn build_order_action(
    action: OrderAction,
    session: &Session,
    selected_cat: Option<&str>,
    selected_building: Option<&str>,
    selected_pile: Option<&str>,
    planned_building: BuildingType,
) -> Result<ClientAction, &'static str> {
    if !session.ready {
        return Err("The server session is not ready.");
    }
    let session_id = session.session_id.clone();
    let sig = session.sig.clone();
    let nickname = "Desktop Cat".to_owned();
    let request = |kind| ClientAction::RequestJob {
        session_id: session_id.clone(),
        nickname: nickname.clone(),
        sig: sig.clone(),
        kind,
    };
    Ok(match action {
        OrderAction::Hunt => request(JobKind::HuntExpedition),
        OrderAction::Fish => request(JobKind::Fish),
        OrderAction::FetchWater => request(JobKind::FetchWater),
        OrderAction::Quarry => request(JobKind::Quarry),
        OrderAction::GatherLogs => request(JobKind::GatherLogs),
        OrderAction::ForageFibre => request(JobKind::ForageFibre),
        OrderAction::ExpandVillage => request(JobKind::ExpandVillage),
        OrderAction::Ritual => request(JobKind::Ritual),
        OrderAction::OfferTithe => ClientAction::OfferTithe {
            session_id,
            nickname,
            sig,
        },
        OrderAction::OfferMaterials => ClientAction::OfferMaterials {
            session_id,
            nickname,
            sig,
        },
        OrderAction::HaulSelected => ClientAction::HaulGatherSpot {
            session_id,
            nickname,
            sig,
            stockpile_id: selected_pile
                .ok_or("Select a gather spot first.")?
                .to_owned(),
            cat_id: selected_cat.map(str::to_owned),
        },
        OrderAction::PlanBuilding => ClientAction::PlanBuilding {
            session_id,
            nickname,
            sig,
            building_type: planned_building,
            site: None,
        },
        OrderAction::StaffSelected => ClientAction::AssignWorker {
            session_id,
            nickname,
            sig,
            cat_id: selected_cat.ok_or("Select a cat first.")?.to_owned(),
            building_id: Some(
                selected_building
                    .ok_or("Select a building first.")?
                    .to_owned(),
            ),
        },
        OrderAction::UnstaffSelected => ClientAction::AssignWorker {
            session_id,
            nickname,
            sig,
            cat_id: selected_cat.ok_or("Select a cat first.")?.to_owned(),
            building_id: None,
        },
        OrderAction::TrainSelected => ClientAction::TrainWarrior {
            session_id,
            nickname,
            sig,
            cat_id: Some(selected_cat.ok_or("Select a cat first.")?.to_owned()),
        },
        OrderAction::DefendRaid => ClientAction::DefendRaid {
            session_id,
            nickname,
            sig,
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn handle_order_buttons(
    session: Res<Session>,
    ui: Res<OrdersUi>,
    cat: Res<Selection>,
    building: Res<BuildingSelection>,
    pile: Res<StockpileSelection>,
    mut outgoing: ResMut<OutgoingActions>,
    mut feedback: ResMut<ClientFeedback>,
    mut tools: ResMut<Tools>,
    buttons: Query<(&Interaction, &OrderButton), Changed<Interaction>>,
) {
    if !ui.visible {
        return;
    }
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if button.0 == OrderAction::PlanBuilding {
            tools.mode = ToolMode::Building;
            tools.drag = None;
            feedback.message = Some("Building tool active: click an exact map tile.".to_owned());
            feedback.level = FeedbackLevel::Info;
            feedback.remaining_secs = 8.0;
            continue;
        }
        let planned = PLANNABLE_BUILDINGS[ui.planned_building % PLANNABLE_BUILDINGS.len()];
        match build_order_action(
            button.0,
            &session,
            cat.selected.as_deref(),
            building.selected.as_deref(),
            pile.selected.as_deref(),
            planned,
        ) {
            Ok(action) => outgoing.0.push(action),
            Err(message) => {
                feedback.message = Some(message.to_owned());
                feedback.level = FeedbackLevel::Error;
                feedback.remaining_secs = 8.0;
            }
        }
    }
}

/// Appoint the selected cat to a role when an "Appoint <role>" button is clicked.
fn handle_appoint_buttons(
    session: Res<Session>,
    selection: Res<Selection>,
    mut outgoing: ResMut<OutgoingActions>,
    buttons: Query<(&Interaction, &AppointButton), Changed<Interaction>>,
) {
    for (interaction, appoint) in &buttons {
        if *interaction == Interaction::Pressed
            && let (Some(cat), true) = (selection.selected.clone(), session.ready)
        {
            outgoing.0.push(ClientAction::AssignOfficer {
                session_id: session.session_id.clone(),
                nickname: "Desktop Cat".to_string(),
                sig: session.sig.clone(),
                role: appoint.0,
                cat_id: cat,
            });
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
fn handle_boost_button(
    session: Res<Session>,
    selection: Res<Selection>,
    latest: Res<LatestSnapshot>,
    mut outgoing: ResMut<OutgoingActions>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<BoostButton>)>,
) {
    for interaction in &buttons {
        if *interaction == Interaction::Pressed
            && let (Some(cat_id), true) = (selection.selected.clone(), session.ready)
        {
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
}

fn labor_name(labor: Labor) -> &'static str {
    match labor {
        Labor::Hunt => "hunt",
        Labor::Fishing => "fishing",
        Labor::Build => "build",
        Labor::Ritual => "ritual",
        Labor::Fight => "fight",
        Labor::Train => "train",
        Labor::Quarry => "quarry",
        Labor::Woodcut => "woodcut",
        Labor::Forage => "forage",
        Labor::FetchWater => "fetch water",
        Labor::Mill => "mill",
        Labor::Process => "process",
        Labor::Craft => "craft",
        Labor::Textile => "textile",
        Labor::Metalwork => "metalwork",
        Labor::Farm => "farm",
        Labor::Haul => "haul",
        Labor::Research => "research",
        Labor::Scout => "scout",
    }
}

fn update_labor_preference_controls(
    latest: Res<LatestSnapshot>,
    selection: Res<Selection>,
    ui: Res<LaborPreferenceUi>,
    mut text: Query<&mut Text, With<LaborPreferenceText>>,
) {
    if !latest.is_changed() && !selection.is_changed() && !ui.is_changed() {
        return;
    }
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let labor = ALL_LABORS[ui.selected % ALL_LABORS.len()];
    let enabled =
        selected_cat(&latest, &selection).is_some_and(|cat| cat.preferred_labors.contains(&labor));
    text.0 = format!(
        "{} [{}]",
        labor_name(labor),
        if enabled { "on" } else { "off" }
    );
}

fn handle_labor_preference_buttons(
    session: Res<Session>,
    selection: Res<Selection>,
    latest: Res<LatestSnapshot>,
    mut ui: ResMut<LaborPreferenceUi>,
    mut outgoing: ResMut<OutgoingActions>,
    cycle: Query<&Interaction, (Changed<Interaction>, With<CycleLaborPreference>)>,
    toggle: Query<&Interaction, (Changed<Interaction>, With<ToggleLaborPreference>)>,
) {
    if cycle
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        ui.selected = (ui.selected + 1) % ALL_LABORS.len();
    }
    if !toggle
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
        || !session.ready
    {
        return;
    }
    let Some(cat) = selected_cat(&latest, &selection) else {
        return;
    };
    let labor = ALL_LABORS[ui.selected % ALL_LABORS.len()];
    if let Some(action) = labor_preference_action(&session, cat, labor) {
        outgoing.0.push(action);
    }
}

fn labor_preference_action(
    session: &Session,
    cat: &CatSnapshot,
    labor: Labor,
) -> Option<ClientAction> {
    session.ready.then(|| ClientAction::SetCatLaborPreference {
        session_id: session.session_id.clone(),
        nickname: "Desktop Cat".to_owned(),
        sig: session.sig.clone(),
        cat_id: cat.id.clone(),
        labor,
        enabled: !cat.preferred_labors.contains(&labor),
    })
}

fn update_station_queue_controls(
    latest: Res<LatestSnapshot>,
    selection: Res<BuildingSelection>,
    mut ui: ResMut<StationQueueUi>,
    mut panel: Query<&mut Node, With<StationQueueControls>>,
    mut text: Query<&mut Text, With<StationQueueText>>,
) {
    if !latest.is_changed() && !selection.is_changed() && !ui.is_changed() {
        return;
    }
    let (Ok(mut panel), Ok(mut text)) = (panel.single_mut(), text.single_mut()) else {
        return;
    };
    let building = selection.selected.as_deref().and_then(|id| {
        latest
            .0
            .as_ref()
            .and_then(|world| world.colonies.first())
            .and_then(|colony| colony.buildings.iter().find(|building| building.id == id))
    });
    let Some(building) =
        building.filter(|building| building.building_type == BuildingType::Sawmill)
    else {
        panel.display = Display::None;
        ui.selected = 0;
        return;
    };
    panel.display = Display::Flex;
    if building.production_queue.is_empty() {
        ui.selected = 0;
        text.0 = format!(
            "queue empty — add cut logs | {}",
            if building.production_paused {
                "paused"
            } else {
                "running"
            }
        );
    } else {
        ui.selected = ui.selected.min(building.production_queue.len() - 1);
        let entry = &building.production_queue[ui.selected];
        text.0 = format!(
            "queue {}/{}: {}{} | {}",
            ui.selected + 1,
            building.production_queue.len(),
            entry.recipe_id.replace('_', " "),
            if entry.repeat { " (repeat)" } else { " (once)" },
            if building.production_paused {
                "paused"
            } else {
                "running"
            },
        );
    }
}

fn handle_station_queue_buttons(
    session: Res<Session>,
    latest: Res<LatestSnapshot>,
    selection: Res<BuildingSelection>,
    mut ui: ResMut<StationQueueUi>,
    mut outgoing: ResMut<OutgoingActions>,
    buttons: Query<(&Interaction, &StationQueueButton), Changed<Interaction>>,
) {
    let Some(building) = selection.selected.as_deref().and_then(|id| {
        latest
            .0
            .as_ref()
            .and_then(|world| world.colonies.first())
            .and_then(|colony| colony.buildings.iter().find(|building| building.id == id))
    }) else {
        return;
    };
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if matches!(button, StationQueueButton::SelectNext) {
            if !building.production_queue.is_empty() {
                ui.selected = (ui.selected + 1) % building.production_queue.len();
            }
            continue;
        }
        if !session.ready {
            continue;
        }
        let Some(action) = station_queue_action(&session, building, ui.selected, *button) else {
            continue;
        };
        outgoing.0.push(action);
    }
}

fn station_queue_action(
    session: &Session,
    building: &BuildingSnapshot,
    selected: usize,
    button: StationQueueButton,
) -> Option<ClientAction> {
    if !session.ready || building.building_type != BuildingType::Sawmill {
        return None;
    }
    let edit = match button {
        StationQueueButton::Add => ProductionQueueEdit::Add {
            recipe_id: "logs_to_lumber".to_owned(),
            repeat: true,
        },
        StationQueueButton::MoveUp => ProductionQueueEdit::Move {
            index: selected,
            direction: QueueMoveDirection::Up,
        },
        StationQueueButton::MoveDown => ProductionQueueEdit::Move {
            index: selected,
            direction: QueueMoveDirection::Down,
        },
        StationQueueButton::Remove => ProductionQueueEdit::Remove { index: selected },
        StationQueueButton::ToggleRepeat => {
            let entry = building.production_queue.get(selected)?;
            ProductionQueueEdit::SetRepeat {
                index: selected,
                repeat: !entry.repeat,
            }
        }
        StationQueueButton::TogglePause => ProductionQueueEdit::SetPaused {
            paused: !building.production_paused,
        },
        StationQueueButton::SelectNext => return None,
    };
    Some(ClientAction::EditProductionQueue {
        session_id: session.session_id.clone(),
        nickname: "Desktop Cat".to_owned(),
        sig: session.sig.clone(),
        building_id: building.id.clone(),
        edit,
    })
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
    buttons: Query<(&Interaction, &VacateButton), Changed<Interaction>>,
) {
    for (interaction, vacate) in &buttons {
        if *interaction == Interaction::Pressed && session.ready {
            outgoing.0.push(ClientAction::UnassignOfficer {
                session_id: session.session_id.clone(),
                nickname: "Desktop Cat".to_string(),
                sig: session.sig.clone(),
                role: vacate.0,
            });
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
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    blockers: WorldInputBlockerQuery,
    research: Res<UpgradeTreeUi>,
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
    let cursor = windows
        .single()
        .ok()
        .and_then(|window| window.cursor_position());
    let over_world_input_blocker = cursor_over_world_input_blocker(cursor, &blockers);
    if !world_pointer_input_allowed(research.visible, over_world_input_blocker, cursor.is_some()) {
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
        stockpile_selection.selected_farm = None;
        selection.selected = toggle_selection(selection.selected.as_deref(), picked);
        return;
    }
    // Otherwise, clicking a non-shrine stockpile selects it (for removal).
    let tile = world_to_tile(world);
    let pile = colony
        .stockpiles
        .iter()
        .find(|s| !is_seeded_store(&s.id) && point_in_stockpile(tile, s));
    selection.selected = None;
    if let Some(pile) = pile {
        stockpile_selection.selected_farm = None;
        stockpile_selection.selected = toggle_selection(
            stockpile_selection.selected.as_deref(),
            Some(pile.id.clone()),
        );
    } else {
        let farm = colony.farms.iter().find(|farm| point_in_farm(tile, farm));
        stockpile_selection.selected = None;
        stockpile_selection.selected_farm = toggle_selection(
            stockpile_selection.selected_farm.as_deref(),
            farm.map(|farm| farm.id.clone()),
        );
    }
}

/// Right-click a building to inspect it; right-click empty ground or the same
/// building again to deselect. Shift+right-click is handled by
/// `cycle_stacked_selection` instead, so bail when shift is held.
#[allow(clippy::too_many_arguments)]
fn select_building(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    blockers: WorldInputBlockerQuery,
    research: Res<UpgradeTreeUi>,
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
    let cursor = windows
        .single()
        .ok()
        .and_then(|window| window.cursor_position());
    let over_world_input_blocker = cursor_over_world_input_blocker(cursor, &blockers);
    if !world_pointer_input_allowed(research.visible, over_world_input_blocker, cursor.is_some()) {
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
        .filter(|b| building_visual(b.building_type).is_map_building())
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
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    blockers: WorldInputBlockerQuery,
    research: Res<UpgradeTreeUi>,
    latest: Res<LatestSnapshot>,
    mut cat_sel: ResMut<Selection>,
    mut building_sel: ResMut<BuildingSelection>,
    mut pile_sel: ResMut<StockpileSelection>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if !shift || !buttons.just_pressed(MouseButton::Right) {
        return;
    }
    let cursor = windows
        .single()
        .ok()
        .and_then(|window| window.cursor_position());
    let over_world_input_blocker = cursor_over_world_input_blocker(cursor, &blockers);
    if !world_pointer_input_allowed(research.visible, over_world_input_blocker, cursor.is_some()) {
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
        .filter(|b| building_visual(b.building_type).is_map_building())
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
        .filter(|s| !is_seeded_store(&s.id) && point_in_stockpile(tile, s))
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
    pile_sel.selected_farm = None;
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
    art: Res<AdventureUiArt>,
    mut selection: ResMut<Selection>,
    mut panel: Query<&mut Node, (With<InspectorPanel>, Without<NeedBar>)>,
    mut text: Query<&mut Text, With<InspectorText>>,
    mut bars: Query<(&mut Node, &mut ImageNode, &NeedBar), Without<InspectorPanel>>,
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
            for (mut bar, mut image, need) in &mut bars {
                let value = cat_need_value(&cat.needs, need.0);
                bar.width = Val::Percent(value.clamp(0.0, 100.0) as f32);
                image.image = match need_bar_band(value) {
                    NeedBarBand::Comfortable => art.progress_good.clone(),
                    NeedBarBand::Low => art.progress_mid.clone(),
                    NeedBarBand::Critical => art.progress_low.clone(),
                };
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
        stockpile.selected_farm = None;
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
    camera: &Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
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
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    ui: Query<&Interaction, With<Button>>,
    ui_roots: UiRootQuery,
    latest: Res<LatestSnapshot>,
    mut panel: Query<&mut Node, With<TooltipPanel>>,
    mut text: Query<&mut Text, With<TooltipText>>,
) {
    let (Ok(mut node), Ok(mut text)) = (panel.single_mut(), text.single_mut()) else {
        return;
    };
    let cursor = windows.single().ok().and_then(|w| w.cursor_position());
    // Suppress over every visible UI root, not only interactive buttons. Without
    // this hit test, pointing at a title bar or HUD still projected through to
    // the terrain and painted an unrelated biome card on top of the panel.
    let over_button = ui.iter().any(|i| !matches!(i, Interaction::None));
    let over_ui = cursor.is_some_and(|cursor| {
        ui_roots.iter().any(|(computed, transform, style)| {
            style.display != Display::None && computed.contains_point(*transform, cursor)
        })
    });
    let hovered = world_tooltip_allowed(over_button, over_ui, cursor.is_some())
        .then_some(cursor)
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

fn world_tooltip_allowed(over_button: bool, over_ui: bool, has_cursor: bool) -> bool {
    has_cursor && !over_button && !over_ui
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
        .filter(|b| building_visual(b.building_type).is_map_building())
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
    let title = if pile.id == GENERAL_STOREHOUSE_ID {
        "Village storehouse"
    } else if pile.id == SHRINE_STOCKPILE_ID {
        "Legacy shrine store"
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
    mut buttons: Query<(&Interaction, &ToolButton, &mut KitToggle)>,
) {
    for (interaction, button, mut toggle) in &mut buttons {
        if *interaction == Interaction::Pressed && tools.mode != button.0 {
            tools.mode = button.0;
            tools.drag = None;
        }
        // The kit paints hover/press/active; we just flag which tool is active.
        toggle.active = tools.mode == button.0;
    }
}

/// Cycle the stockpile accept-type when its picker is clicked, and keep the
/// button label in sync with the current choice.
fn handle_accept_button(
    mut tools: ResMut<Tools>,
    mut button: AcceptButtonQuery,
    mut text: Query<&mut Text, With<AcceptButtonText>>,
) {
    for interaction in &mut button {
        if *interaction == Interaction::Pressed {
            tools.accept = tools.accept.next();
        }
    }
    if tools.is_changed()
        && let Ok(mut text) = text.single_mut()
    {
        text.0 = format!("Accepts: {}", tools.accept.label());
    }
}

fn crop_label(crop: CropKind) -> &'static str {
    match crop {
        CropKind::Catnip => "catnip",
        CropKind::Grain => "grain",
        CropKind::Herb => "herbs",
    }
}

fn next_crop(crop: CropKind) -> CropKind {
    match crop {
        CropKind::Catnip => CropKind::Grain,
        CropKind::Grain => CropKind::Herb,
        CropKind::Herb => CropKind::Catnip,
    }
}

fn handle_farm_crop_button(
    mut tools: ResMut<Tools>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<CycleFarmCrop>)>,
    mut text: Query<&mut Text, With<FarmCropText>>,
) {
    if buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        tools.crop = next_crop(tools.crop);
    }
    if tools.is_changed()
        && let Ok(mut text) = text.single_mut()
    {
        text.0 = format!("Crop: {}", crop_label(tools.crop));
    }
}

const GATHER_KINDS: [ResourceKind; 4] = [
    ResourceKind::Food,
    ResourceKind::Water,
    ResourceKind::Materials,
    ResourceKind::Logs,
];

fn next_gather_kind(kind: ResourceKind) -> ResourceKind {
    let index = GATHER_KINDS
        .iter()
        .position(|candidate| *candidate == kind)
        .unwrap_or(0);
    GATHER_KINDS[(index + 1) % GATHER_KINDS.len()]
}

fn handle_gather_kind_button(
    mut tools: ResMut<Tools>,
    buttons: Query<&Interaction, (Changed<Interaction>, With<CycleGatherKind>)>,
    mut text: Query<&mut Text, With<GatherKindText>>,
) {
    if buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
    {
        tools.gather_kind = next_gather_kind(tools.gather_kind);
    }
    if tools.is_changed()
        && let Ok(mut text) = text.single_mut()
    {
        text.0 = format!("Gather: {}", resource_kind_name(tools.gather_kind));
    }
}

/// Click-drag a rectangle in a paint mode to designate an avoid/gather zone or a
/// stockpile; release sends the matching action. Esc cancels an in-progress drag.
#[allow(clippy::too_many_arguments)]
fn zone_paint(
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    blockers: WorldInputBlockerQuery,
    research: Res<UpgradeTreeUi>,
    session: Res<Session>,
    mut tools: ResMut<Tools>,
    mut outgoing: ResMut<OutgoingActions>,
) {
    let Some(kind) = tools.mode.paint_kind() else {
        tools.drag = None;
        return;
    };
    let accept = tools.accept;
    let crop = tools.crop;
    let gather_kind = tools.gather_kind;
    if keys.just_pressed(KeyCode::Escape) {
        tools.drag = None;
        return;
    }
    let cursor = windows
        .single()
        .ok()
        .and_then(|window| window.cursor_position());
    let over_world_input_blocker = cursor_over_world_input_blocker(cursor, &blockers);
    if !world_pointer_input_allowed(research.visible, over_world_input_blocker, cursor.is_some()) {
        // A drag that ends over a panel is cancelled instead of committing a
        // rectangle to the obscured world beneath it.
        if buttons.just_pressed(MouseButton::Left) || buttons.just_released(MouseButton::Left) {
            tools.drag = None;
        }
        return;
    }
    let tile = cursor_world(&windows, &camera).map(world_to_tile);

    if buttons.just_pressed(MouseButton::Left)
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
            PaintKind::Farm => ClientAction::DesignateFarm {
                session_id: session.session_id.clone(),
                nickname: "Desktop Cat".to_string(),
                sig: session.sig.clone(),
                a,
                b,
                crop,
            },
            PaintKind::GatherSpot => ClientAction::DesignateGatherSpot {
                session_id: session.session_id.clone(),
                nickname: "Desktop Cat".to_string(),
                sig: session.sig.clone(),
                a,
                b,
                kind: gather_kind,
            },
            PaintKind::FishingSpot => ClientAction::DesignateFishingSpot {
                session_id: session.session_id.clone(),
                nickname: "Desktop Cat".to_string(),
                sig: session.sig.clone(),
                at: a,
            },
            PaintKind::Road => ClientAction::BuildRoad {
                session_id: session.session_id.clone(),
                nickname: "Desktop Cat".to_string(),
                sig: session.sig.clone(),
                a,
                b,
            },
        });
    }
}

/// Building tool: one left click selects the exact north-west footprint anchor.
/// The authoritative sim repeats collision, access and affordability checks before
/// atomically reserving the scaffold, so the preview need not predict server state.
#[allow(clippy::too_many_arguments)]
fn place_building(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera: Query<(&Camera, &GlobalTransform), With<WorldCamera>>,
    blockers: WorldInputBlockerQuery,
    research: Res<UpgradeTreeUi>,
    session: Res<Session>,
    tools: Res<Tools>,
    orders: Res<OrdersUi>,
    mut outgoing: ResMut<OutgoingActions>,
) {
    if tools.mode != ToolMode::Building || !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let cursor = windows
        .single()
        .ok()
        .and_then(|window| window.cursor_position());
    let over_world_input_blocker = cursor_over_world_input_blocker(cursor, &blockers);
    if !session.ready
        || !world_pointer_input_allowed(
            research.visible,
            over_world_input_blocker,
            cursor.is_some(),
        )
    {
        return;
    }
    let Some(site) = cursor_world(&windows, &camera).map(world_to_tile) else {
        return;
    };
    let building_type = PLANNABLE_BUILDINGS[orders.planned_building % PLANNABLE_BUILDINGS.len()];
    outgoing.0.push(build_exact_building_action(
        &session,
        building_type,
        TilePoint {
            x: site.0,
            y: site.1,
        },
    ));
}

fn build_exact_building_action(
    session: &Session,
    building_type: BuildingType,
    site: TilePoint,
) -> ClientAction {
    ClientAction::PlanBuilding {
        session_id: session.session_id.clone(),
        nickname: "Desktop Cat".to_owned(),
        sig: session.sig.clone(),
        building_type,
        site: Some(site),
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

#[allow(clippy::too_many_arguments)]
fn camera_controls(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
    latest: Res<LatestSnapshot>,
    research: Res<UpgradeTreeUi>,
    windows: Query<&Window>,
    blockers: WorldInputBlockerQuery,
    mut inited: Local<bool>,
    mut last_auto_radius: Local<u32>,
    mut last_colony_id: Local<Option<String>>,
    mut last_window_size: Local<Vec2>,
    mut user_adjusted: Local<bool>,
    mut camera: Query<(&mut Transform, &mut Projection), With<WorldCamera>>,
) {
    if research.visible {
        // The ledger owns WASD/arrows and wheel while open. Consume pointer
        // deltas so closing it cannot replay stale map-camera input.
        motion.clear();
        wheel.clear();
        return;
    }
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
    let window_size = windows
        .single()
        .map(|window| Vec2::new(window.width(), window.height()))
        .unwrap_or(Vec2::new(1280.0, 800.0));
    let cursor = windows
        .single()
        .ok()
        .and_then(|window| window.cursor_position());
    let pointer_allowed = world_pointer_input_allowed(
        false,
        cursor_over_world_input_blocker(cursor, &blockers),
        cursor.is_some(),
    );
    if let Some(colony) = latest
        .0
        .as_ref()
        .and_then(|snapshot| snapshot.colonies.first())
    {
        let village_changed = last_colony_id.as_deref() != Some(colony.id.as_str());
        if village_changed {
            // A selected village can be arbitrarily far from the prior one.
            // Always move to the newly selected map, even when the player had
            // panned or zoomed the old village by hand.
            *user_adjusted = false;
            *last_auto_radius = colony.village_radius;
            projection.scale =
                village_fit_zoom(colony.village_radius, window_size.x, window_size.y);
            let center =
                village_camera_center(colony.anchor, colony.village_radius, projection.scale);
            transform.translation.x = center.x;
            transform.translation.y = center.y;
            *last_colony_id = Some(colony.id.clone());
        }
        let radius_grew = colony.village_radius > *last_auto_radius;
        let window_changed = window_size != *last_window_size;
        if !*user_adjusted && (radius_grew || window_changed) {
            projection.scale =
                village_fit_zoom(colony.village_radius, window_size.x, window_size.y);
            let center =
                village_camera_center(colony.anchor, colony.village_radius, projection.scale);
            transform.translation.x = center.x;
            transform.translation.y = center.y;
        }
        *last_auto_radius = (*last_auto_radius).max(colony.village_radius);
        *last_window_size = window_size;
    }
    let speed = 620.0 * time.delta_secs() * projection.scale;
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        *user_adjusted = true;
        transform.translation.x -= speed;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        *user_adjusted = true;
        transform.translation.x += speed;
    }
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        *user_adjusted = true;
        transform.translation.y += speed;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        *user_adjusted = true;
        transform.translation.y -= speed;
    }
    if keys.just_pressed(CAMERA_RESET_SHORTCUT) {
        *user_adjusted = false;
        let colony = latest
            .0
            .as_ref()
            .and_then(|snapshot| snapshot.colonies.first());
        let center = colony.map_or_else(
            || grid_to_world(VILLAGE_ANCHOR.x, VILLAGE_ANCHOR.y),
            |colony| village_camera_center(colony.anchor, colony.village_radius, projection.scale),
        );
        transform.translation.x = center.x;
        transform.translation.y = center.y;
        projection.scale = colony.map_or(DEFAULT_ZOOM, |colony| {
            village_fit_zoom(colony.village_radius, window_size.x, window_size.y)
        });
    }
    // Middle-button drag pans the map (left = select cat, right = select
    // building).
    if buttons.pressed(MouseButton::Middle) && pointer_allowed {
        *user_adjusted = true;
        for ev in motion.read() {
            transform.translation.x -= ev.delta.x * projection.scale;
            transform.translation.y += ev.delta.y * projection.scale;
        }
    } else {
        motion.clear();
    }
    for ev in wheel.read() {
        if pointer_allowed {
            *user_adjusted = true;
            projection.scale =
                (projection.scale * if ev.y > 0.0 { 0.9 } else { 1.1 }).clamp(MIN_ZOOM, MAX_ZOOM);
        }
    }
}

/// Orthographic scale that preserves the close founding view, then fits an
/// expanded village between the fixed top and bottom HUD strips.
fn village_fit_zoom(radius: u32, window_width: f32, window_height: f32) -> f32 {
    if radius <= STARTER_CAMERA_RADIUS {
        return DEFAULT_ZOOM;
    }
    let diameter_tiles = radius.saturating_mul(2).saturating_add(1) as f32 + 4.0;
    let world_span = diameter_tiles * TILE;
    let usable_height = (window_height - CAMERA_VERTICAL_UI_RESERVE).max(window_height * 0.5);
    DEFAULT_ZOOM
        .max(world_span / window_width.max(1.0))
        .max(world_span / usable_height.max(1.0))
        .clamp(MIN_ZOOM, MAX_ZOOM)
}

fn village_camera_center(anchor: TilePoint, radius: u32, zoom: f32) -> Vec2 {
    let mut center = grid_to_world(anchor.x + 1, anchor.y + 1);
    if radius > STARTER_CAMERA_RADIUS {
        // Moving the camera left shifts the village right into the safe map
        // rectangle. Scale converts the fixed screen-space inset to world units.
        center.x -= CAMERA_SAFE_CENTER_OFFSET_X * zoom;
    }
    center
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

/// Rebuild the compact selector only when village identity/summary/selection
/// changes (not on every one-second resource update).
fn is_discovered_trade_target(
    snapshot: &WorldSnapshot,
    selected_id: Option<&str>,
    target_id: &str,
) -> bool {
    selected_id.is_some_and(|selected_id| selected_id != target_id)
        && snapshot
            .known_villages
            .iter()
            .any(|village| village.id == target_id)
}

const VILLAGE_TRADE_KINDS: [ResourceKind; 19] = [
    ResourceKind::Food,
    ResourceKind::Water,
    ResourceKind::Herbs,
    ResourceKind::Catnip,
    ResourceKind::Grain,
    ResourceKind::Flour,
    ResourceKind::Materials,
    ResourceKind::Refined,
    ResourceKind::Weapons,
    ResourceKind::Armor,
    ResourceKind::Logs,
    ResourceKind::Lumber,
    ResourceKind::Fibre,
    ResourceKind::Hide,
    ResourceKind::Cloth,
    ResourceKind::Leather,
    ResourceKind::Ore,
    ResourceKind::Metal,
    ResourceKind::Blessings,
];
const VILLAGE_TRADE_AMOUNTS: [f64; 6] = [1.0, 5.0, 10.0, 25.0, 50.0, 100.0];

fn trade_resource_label(kind: ResourceKind) -> String {
    format!("{kind:?}").to_lowercase()
}

fn trade_resource_short_label(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::Food => "food",
        ResourceKind::Water => "water",
        ResourceKind::Herbs => "herbs",
        ResourceKind::Catnip => "nip",
        ResourceKind::Grain => "grain",
        ResourceKind::Flour => "flour",
        ResourceKind::Materials => "mats",
        ResourceKind::Refined => "refined",
        ResourceKind::Weapons => "weapons",
        ResourceKind::Armor => "armor",
        ResourceKind::Logs => "logs",
        ResourceKind::Lumber => "lumber",
        ResourceKind::Planks => "planks",
        ResourceKind::Blocks => "blocks",
        ResourceKind::Tools => "tools",
        ResourceKind::Fibre => "fibre",
        ResourceKind::Hide => "hide",
        ResourceKind::Cloth => "cloth",
        ResourceKind::Leather => "leather",
        ResourceKind::Ore => "ore",
        ResourceKind::Metal => "metal",
        ResourceKind::Blessings => "bless",
    }
}

fn cycle_village_trade_draft(draft: &mut VillageTradeDraft, field: VillageTradeDraftField) {
    let next_kind = |current: ResourceKind, excluded: ResourceKind| {
        let index = VILLAGE_TRADE_KINDS
            .iter()
            .position(|kind| *kind == current)
            .unwrap_or(0);
        (1..=VILLAGE_TRADE_KINDS.len())
            .map(|step| VILLAGE_TRADE_KINDS[(index + step) % VILLAGE_TRADE_KINDS.len()])
            .find(|kind| *kind != excluded)
            .unwrap_or(current)
    };
    let next_amount = |current: f64| {
        let index = VILLAGE_TRADE_AMOUNTS
            .iter()
            .position(|amount| *amount == current)
            .unwrap_or(0);
        VILLAGE_TRADE_AMOUNTS[(index + 1) % VILLAGE_TRADE_AMOUNTS.len()]
    };
    match field {
        VillageTradeDraftField::OfferedKind => {
            draft.offered_kind = next_kind(draft.offered_kind, draft.requested_kind);
        }
        VillageTradeDraftField::OfferedAmount => {
            draft.offered_amount = next_amount(draft.offered_amount);
        }
        VillageTradeDraftField::RequestedKind => {
            draft.requested_kind = next_kind(draft.requested_kind, draft.offered_kind);
        }
        VillageTradeDraftField::RequestedAmount => {
            draft.requested_amount = next_amount(draft.requested_amount);
        }
    }
}

fn village_trade_draft_label(draft: &VillageTradeDraft, field: VillageTradeDraftField) -> String {
    match field {
        VillageTradeDraftField::OfferedKind => {
            format!(
                "Give resource: {}",
                trade_resource_label(draft.offered_kind)
            )
        }
        VillageTradeDraftField::OfferedAmount => {
            format!("Give amount: {:.0}", draft.offered_amount)
        }
        VillageTradeDraftField::RequestedKind => format!(
            "Ask resource: {}",
            trade_resource_label(draft.requested_kind)
        ),
        VillageTradeDraftField::RequestedAmount => {
            format!("Ask amount: {:.0}", draft.requested_amount)
        }
    }
}

fn village_trade_target_label(name: &str, draft: &VillageTradeDraft) -> String {
    format!(
        "Offer to {name}\n{:.0} {} ↔ {:.0} {}",
        draft.offered_amount,
        trade_resource_short_label(draft.offered_kind),
        draft.requested_amount,
        trade_resource_short_label(draft.requested_kind),
    )
}

fn update_village_selector(
    mut commands: Commands,
    latest: Res<LatestSnapshot>,
    selection: Res<VillageSelection>,
    trade_draft: Res<VillageTradeDraft>,
    rows: Query<Entity, With<VillageSelectorRows>>,
    mut last_signature: Local<Vec<String>>,
) {
    let Some(snapshot) = latest.0.as_ref() else {
        return;
    };
    let mut signature: Vec<String> = snapshot
        .colonies
        .iter()
        .map(|colony| {
            format!(
                "{}|{}|{}|{:?}|{:?}|{:?}|{}",
                colony.id,
                colony.name,
                colony.housing.population,
                colony.status,
                colony.kind,
                colony.capabilities,
                selection.selected_id.as_deref() == Some(colony.id.as_str())
            )
        })
        .collect();
    signature.extend(snapshot.known_villages.iter().map(|village| {
        format!(
            "known|{}|{}|{:?}|{:?}",
            village.id, village.name, village.kind, village.capabilities
        )
    }));
    signature.extend(snapshot.village_trade_offers.iter().map(|offer| {
        format!(
            "offer|{}|{}|{}|{:?}|{}|{:?}|{}",
            offer.id,
            offer.from_colony_id,
            offer.to_colony_id,
            offer.offered_kind,
            offer.offered_amount,
            offer.requested_kind,
            offer.requested_amount,
        )
    }));
    signature.push(format!("draft|{trade_draft:?}"));
    if *last_signature == signature {
        return;
    }
    *last_signature = signature;

    let Ok(rows) = rows.single() else {
        return;
    };
    commands
        .entity(rows)
        .despawn_children()
        .with_children(|row| {
            if !snapshot.known_villages.is_empty() {
                for field in [
                    VillageTradeDraftField::OfferedKind,
                    VillageTradeDraftField::OfferedAmount,
                    VillageTradeDraftField::RequestedKind,
                    VillageTradeDraftField::RequestedAmount,
                ] {
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(150.0),
                            height: Val::Px(30.0),
                            padding: UiRect::axes(Val::Px(UI_GAP), Val::Px(UI_GAP_TIGHT)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(UI_RADIUS - 2.0)),
                            ..default()
                        },
                        BackgroundColor(UI_BUTTON_GREY),
                        BorderColor::all(Color::NONE),
                        ImageNode::default(),
                        KitButton,
                        VillageTradeDraftButton(field),
                        children![(
                            ui_text(
                                village_trade_draft_label(&trade_draft, field),
                                FS_SMALL,
                                UI_INK
                            ),
                            TextLayout::justify(Justify::Center),
                        )],
                    ));
                }
            }
            for colony in &snapshot.colonies {
                let active = selection.selected_id.as_deref() == Some(colony.id.as_str());
                let status = format!("{:?}", colony.status).to_lowercase();
                let marker = if active { "●" } else { "○" };
                let group = village_group_label(colony.kind, colony.capabilities.is_owner);
                let label = format!(
                    "{marker} {group} · {name}\n{pop} cats · {status}",
                    name = colony.name,
                    pop = colony.housing.population,
                );
                row.spawn((
                    Button,
                    Node {
                        width: Val::Px(240.0),
                        height: Val::Px(48.0),
                        padding: UiRect::axes(Val::Px(UI_GAP), Val::Px(UI_GAP_TIGHT)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(UI_RADIUS - 2.0)),
                        ..default()
                    },
                    BackgroundColor(UI_BUTTON_BROWN),
                    BorderColor::all(Color::NONE),
                    ImageNode::default(),
                    KitButton,
                    KitToggle { active },
                    VillageButton(colony.id.clone()),
                    children![(
                        ui_text(label, FS_SMALL, UI_INK),
                        TextLayout::justify(Justify::Center),
                    )],
                ));
                if is_discovered_trade_target(
                    snapshot,
                    selection.selected_id.as_deref(),
                    &colony.id,
                ) {
                    let label = village_trade_target_label(&colony.name, &trade_draft);
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(240.0),
                            height: Val::Px(38.0),
                            padding: UiRect::axes(Val::Px(UI_GAP), Val::Px(UI_GAP_TIGHT)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(UI_RADIUS - 2.0)),
                            ..default()
                        },
                        BackgroundColor(UI_BUTTON_BROWN),
                        BorderColor::all(Color::NONE),
                        ImageNode::default(),
                        KitButton,
                        VillageTradeProposalButton(colony.id.clone()),
                        children![(
                            ui_text(label, FS_SMALL, UI_INK),
                            TextLayout::justify(Justify::Center),
                        )],
                    ));
                }
            }
            for village in snapshot.known_villages.iter().filter(|village| {
                !snapshot
                    .colonies
                    .iter()
                    .any(|colony| colony.id == village.id)
            }) {
                let label = format!(
                    "◇ {} @ {},{}\nOffer {:.0} {} ↔ {:.0} {}",
                    village.name,
                    village.anchor.x,
                    village.anchor.y,
                    trade_draft.offered_amount,
                    trade_resource_short_label(trade_draft.offered_kind),
                    trade_draft.requested_amount,
                    trade_resource_short_label(trade_draft.requested_kind),
                );
                row.spawn((
                    Button,
                    Node {
                        width: Val::Px(240.0),
                        height: Val::Px(48.0),
                        padding: UiRect::axes(Val::Px(UI_GAP), Val::Px(UI_GAP_TIGHT)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(UI_RADIUS - 2.0)),
                        ..default()
                    },
                    BackgroundColor(UI_BUTTON_BROWN),
                    BorderColor::all(Color::NONE),
                    ImageNode::default(),
                    KitButton,
                    VillageTradeProposalButton(village.id.clone()),
                    children![(
                        ui_text(label, FS_SMALL, UI_INK),
                        TextLayout::justify(Justify::Center),
                    )],
                ));
            }
            if let Some(selected_id) = selection.selected_id.as_deref() {
                for offer in snapshot.village_trade_offers.iter().filter(|offer| {
                    offer.from_colony_id == selected_id || offer.to_colony_id == selected_id
                }) {
                    let incoming = offer.to_colony_id == selected_id;
                    let label = format!(
                        "{} {:.0} {} for {:.0} {}",
                        if incoming { "Accept" } else { "Cancel offer:" },
                        offer.offered_amount,
                        format!("{:?}", offer.offered_kind).to_lowercase(),
                        offer.requested_amount,
                        format!("{:?}", offer.requested_kind).to_lowercase(),
                    );
                    let mut entity = row.spawn((
                        Button,
                        Node {
                            width: Val::Px(210.0),
                            height: Val::Px(38.0),
                            padding: UiRect::axes(Val::Px(UI_GAP), Val::Px(UI_GAP_TIGHT)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(UI_RADIUS - 2.0)),
                            ..default()
                        },
                        BackgroundColor(if incoming {
                            UI_BUTTON_BROWN
                        } else {
                            UI_BUTTON_GREY
                        }),
                        BorderColor::all(Color::NONE),
                        ImageNode::default(),
                        KitButton,
                        children![(
                            ui_text(label, FS_SMALL, UI_INK),
                            TextLayout::justify(Justify::Center),
                        )],
                    ));
                    if incoming {
                        entity.insert(AcceptVillageTradeButton(offer.id.clone()));
                    } else {
                        entity.insert(CancelVillageTradeButton(offer.id.clone()));
                    }
                }
            }
        });
}

fn village_group_label(kind: VillageKind, is_owner: bool) -> &'static str {
    match (kind, is_owner) {
        (VillageKind::Global, _) => "Grand Commons",
        (VillageKind::Personal, true) => "My Village",
        (VillageKind::Personal, false) => "Known",
    }
}

fn handle_village_buttons(
    mut buttons: Query<(&Interaction, &VillageButton), Changed<Interaction>>,
    mut latest: ResMut<LatestSnapshot>,
    mut selection: ResMut<VillageSelection>,
    session: Res<Session>,
    mut outgoing: ResMut<OutgoingActions>,
) {
    for (interaction, button) in &mut buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(snapshot) = latest.0.as_mut() else {
            continue;
        };
        let previous = selection.selected_id.clone();
        let action = choose_village(&button.0, snapshot, &mut selection, &session);
        if selection.selected_id != previous
            && session.ready
            && let Err(err) = persist_session(&session, &selection)
        {
            warn!("could not persist selected village: {err}");
        }
        if let Some(action) = action {
            outgoing.0.push(action);
        }
    }
}

fn handle_village_trade_buttons(
    draft_buttons: Query<(&Interaction, &VillageTradeDraftButton), Changed<Interaction>>,
    proposals: Query<(&Interaction, &VillageTradeProposalButton), Changed<Interaction>>,
    accepts: Query<(&Interaction, &AcceptVillageTradeButton), Changed<Interaction>>,
    cancels: Query<(&Interaction, &CancelVillageTradeButton), Changed<Interaction>>,
    session: Res<Session>,
    mut draft: ResMut<VillageTradeDraft>,
    mut outgoing: ResMut<OutgoingActions>,
) {
    for (interaction, button) in &draft_buttons {
        if *interaction == Interaction::Pressed {
            cycle_village_trade_draft(&mut draft, button.0);
        }
    }
    if !session.ready {
        return;
    }
    for (interaction, button) in &proposals {
        if *interaction == Interaction::Pressed
            && let Some(action) = village_trade_proposal_action(&button.0, &draft, &session)
        {
            outgoing.0.push(action);
        }
    }
    for (interaction, button) in &accepts {
        if *interaction == Interaction::Pressed {
            outgoing
                .0
                .push(village_trade_reply_action(&button.0, true, &session));
        }
    }
    for (interaction, button) in &cancels {
        if *interaction == Interaction::Pressed {
            outgoing
                .0
                .push(village_trade_reply_action(&button.0, false, &session));
        }
    }
}

fn village_trade_proposal_action(
    target: &str,
    draft: &VillageTradeDraft,
    session: &Session,
) -> Option<ClientAction> {
    session.ready.then(|| ClientAction::OfferVillageTrade {
        session_id: session.session_id.clone(),
        nickname: "Desktop Cat".to_owned(),
        sig: session.sig.clone(),
        target_colony_id: target.to_owned(),
        offered_kind: draft.offered_kind,
        offered_amount: draft.offered_amount,
        requested_kind: draft.requested_kind,
        requested_amount: draft.requested_amount,
    })
}

fn village_trade_reply_action(offer_id: &str, accept: bool, session: &Session) -> ClientAction {
    if accept {
        ClientAction::AcceptVillageTrade {
            session_id: session.session_id.clone(),
            nickname: "Desktop Cat".to_owned(),
            sig: session.sig.clone(),
            offer_id: offer_id.to_owned(),
        }
    } else {
        ClientAction::CancelVillageTrade {
            session_id: session.session_id.clone(),
            nickname: "Desktop Cat".to_owned(),
            sig: session.sig.clone(),
            offer_id: offer_id.to_owned(),
        }
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
         Pop {pop}  Beds {housed}/{cap_house}  Village Lv {lvl}\n\
         Awaiting homes {probationary}  Unhoused {unhoused}  Left {departures}\n\
         Threat: {threat:?} ({pressure:.0})  warriors {warriors}",
        name = colony.name,
        status = colony.status,
        pop = colony.housing.population,
        housed = colony.housing.housed,
        cap_house = colony.housing.capacity,
        lvl = colony.housing.village_level,
        probationary = colony.housing.probationary,
        unhoused = colony.housing.unhoused,
        departures = colony.housing.departures,
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

/// Reachable Goods-panel summary for the production resources that do not fit
/// in the compact always-on survival HUD.
fn production_stores_text(resources: &ResourceAmounts) -> String {
    format!(
        "Production stores: fibre {:.0} · hide {:.0} · cloth {:.0} · leather {:.0}\n\
         Ore & metal: ore {:.0} · metal {:.0}",
        resources.fibre,
        resources.hide,
        resources.cloth,
        resources.leather,
        resources.ore,
        resources.metal,
    )
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
            census_report_lines(&census, c.scale)
        });
    for (line, mut text) in &mut lines {
        text.0 = report.get(line.0).cloned().unwrap_or_default();
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
    let colony = latest.0.as_ref().and_then(|w| w.colonies.first());
    let mut items = colony.map(|c| c.items.clone()).unwrap_or_default();
    // Most valuable stack first.
    items.sort_by_key(|s| std::cmp::Reverse(s.count * s.value));

    if let Ok(mut text) = treasury.single_mut() {
        text.0 = colony.map_or_else(
            || format!("Treasury: {}g", treasury_total(&items)),
            |colony| {
                format!(
                    "Treasury: {}g\n{}",
                    treasury_total(&items),
                    production_stores_text(&colony.resources)
                )
            },
        );
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
        if !building_visual(b.building_type).is_map_building() {
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
    camera: Query<(&Projection, &Transform), With<WorldCamera>>,
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
    mut camera: Query<&mut Transform, With<WorldCamera>>,
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

fn update_client_feedback(
    time: Res<Time>,
    mut feedback: ResMut<ClientFeedback>,
    mut panel: Query<
        (
            &mut Node,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut ImageNode,
        ),
        With<ClientFeedbackPanel>,
    >,
    mut label: FeedbackLabelQuery,
) {
    let (Ok((mut node, mut background, mut border, mut image)), Ok((mut text, mut color))) =
        (panel.single_mut(), label.single_mut())
    else {
        return;
    };
    let Some(message) = feedback.message.as_ref() else {
        node.display = Display::None;
        return;
    };
    node.display = Display::Flex;
    text.0.clone_from(message);
    match feedback.level {
        FeedbackLevel::Info => {
            *background = BackgroundColor(UI_BG);
            *border = BorderColor::all(UI_POSITIVE);
            image.color = Color::srgb(0.76, 0.96, 0.72);
            color.0 = UI_INK;
        }
        FeedbackLevel::Error => {
            *background = BackgroundColor(UI_BG);
            *border = BorderColor::all(UI_WARNING);
            image.color = Color::srgb(1.0, 0.72, 0.68);
            color.0 = UI_INK;
        }
    }
    feedback.remaining_secs = (feedback.remaining_secs - time.delta_secs()).max(0.0);
    if feedback.remaining_secs == 0.0 {
        feedback.message = None;
        node.display = Display::None;
    }
}

fn update_event_log(
    latest: Res<LatestSnapshot>,
    alerts: Res<ClientAlerts>,
    mut log: Query<&mut Text, With<EventLogText>>,
) {
    if !latest.is_changed() && !alerts.is_changed() {
        return;
    }
    let Ok(mut text) = log.single_mut() else {
        return;
    };
    let mut lines: Vec<String> = alerts
        .0
        .iter()
        .take(4)
        .map(|message| format!("! {message}"))
        .collect();
    if let Some(colony) = latest.0.as_ref().and_then(|w| w.colonies.first()) {
        let mut events = colony.events.clone();
        events.sort_by_key(|event| event.timestamp);
        lines.extend(
            events
                .iter()
                .rev()
                .take(4_usize.saturating_sub(lines.len()))
                .map(|event| format!("- {}", event.message)),
        );
    }
    text.0 = if lines.is_empty() {
        "no recent events".to_string()
    } else {
        lines.join("\n")
    };
}

/// React to toolbar clicks: tint the button and enqueue its action.
fn handle_buttons(
    session: Res<Session>,
    selection: Res<VillageSelection>,
    mut outgoing: ResMut<OutgoingActions>,
    mut buttons: ButtonQuery,
) {
    for (interaction, button) in &mut buttons {
        if *interaction == Interaction::Pressed {
            outgoing.0.extend(build_button_actions(
                button.0,
                &session,
                selection.selected_id.as_deref(),
            ));
        }
    }
}

fn build_button_actions(
    action: ButtonAction,
    session: &Session,
    selected_village: Option<&str>,
) -> Vec<ClientAction> {
    let Some(primary) = build_action(action, session) else {
        return Vec::new();
    };
    let mut actions = vec![primary];
    if action == ButtonAction::FoundVillage
        && let Some(join) = selected_village.and_then(|id| join_village_action(id, session))
    {
        // The server selects a newly founded village on this socket. Explicitly
        // restore the viewing/action target so founding and selecting remain
        // independent player operations.
        actions.push(join);
    }
    actions
}

fn build_action(action: ButtonAction, session: &Session) -> Option<ClientAction> {
    if !session.ready {
        warn!("session not ready; dropping action");
        return None;
    }
    let kind = match action {
        ButtonAction::SupplyFood => JobKind::SupplyFood,
        ButtonAction::SupplyWater => JobKind::SupplyWater,
        ButtonAction::PlanHunt => JobKind::LeaderPlanHunt,
        ButtonAction::ScoutWood
        | ButtonAction::ScoutFood
        | ButtonAction::ScoutWater
        | ButtonAction::ScoutStone
        | ButtonAction::Explore => {
            let mission = match action {
                ButtonAction::ScoutWood => ScoutMission::Resource(ScoutResource::Wood),
                ButtonAction::ScoutFood => ScoutMission::Resource(ScoutResource::Food),
                ButtonAction::ScoutWater => ScoutMission::Resource(ScoutResource::Water),
                ButtonAction::ScoutStone => ScoutMission::Resource(ScoutResource::Stone),
                ButtonAction::Explore => ScoutMission::Explore,
                _ => unreachable!("covered scout actions"),
            };
            return Some(ClientAction::DispatchScout {
                session_id: session.session_id.clone(),
                nickname: "Desktop Cat".to_string(),
                sig: session.sig.clone(),
                mission,
            });
        }
        ButtonAction::FoundVillage => {
            return Some(ClientAction::FoundVillage {
                name: "Forest Hollow".to_string(),
                session_id: session.session_id.clone(),
                sig: Some(session.sig.clone()),
            });
        }
    };
    Some(ClientAction::RequestJob {
        session_id: session.session_id.clone(),
        nickname: "Desktop Cat".to_string(),
        sig: session.sig.clone(),
        kind,
    })
}

/// Send any queued actions over the socket.
fn flush_outgoing(
    conn: Option<NonSendMut<WsConn>>,
    state: Res<ConnectionState>,
    mut outgoing: ResMut<OutgoingActions>,
) {
    let Some(mut conn) = conn else {
        return;
    };
    if state.phase != ConnectionPhase::Connected || outgoing.0.is_empty() {
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
        PaintKind::Farm => Color::srgba(0.55, 0.80, 0.25, 0.45),
        PaintKind::GatherSpot => Color::srgba(0.35, 0.70, 0.85, 0.45),
        PaintKind::FishingSpot => Color::srgba(0.20, 0.72, 0.90, 0.55),
        PaintKind::Road => Color::srgba(0.70, 0.68, 0.62, 0.55),
    }
}

/// Read-only inspector body for a building. Everything shown is authoritative
/// snapshot state: construction, occupied tiles, assigned cats, production and
/// cargo already in flight.
fn building_inspector_text(building: &BuildingSnapshot, colony: &ColonySnapshot) -> String {
    let mut out = format!(
        "{name}  Lv {lvl}",
        name = building_label(building.building_type),
        lvl = building.level,
    );
    if building.construction_progress < 100.0 {
        out.push_str(&format!(
            "\nunder construction {:.0}%\nprogress {} {}%",
            building.construction_progress,
            progress_bar(building.construction_progress / 100.0, 10),
            progress_pct(building.construction_progress / 100.0),
        ));
    } else {
        out.push_str("\noperational");
    }
    out.push_str(&format!(
        "\norigin: {},{}\nfootprint: {}x{} tiles",
        building.world_position.x,
        building.world_position.y,
        building.footprint.width.max(1),
        building.footprint.height.max(1),
    ));

    let workers: Vec<&str> = colony
        .cats
        .iter()
        .filter(|c| c.assigned_building_id.as_deref() == Some(building.id.as_str()))
        .map(|c| c.name.as_str())
        .collect();
    if building.staff_cap > 0 {
        out.push_str(&format!(
            "\nstaffed: {}/{}",
            building.staff_count, building.staff_cap
        ));
        out.push_str(if workers.is_empty() {
            "\nassigned: none"
        } else {
            "\nassigned: "
        });
        if !workers.is_empty() {
            out.push_str(&workers.join(", "));
        }
        if let Some(output) = &building.production_output {
            out.push_str(&format!(
                "\n{}",
                production_line(output, building.production_progress)
            ));
        } else if building.construction_progress >= 100.0 {
            out.push_str("\nproduction: waiting");
        }
    } else if !workers.is_empty() {
        out.push_str(&format!("\nassigned: {}", workers.join(", ")));
    }

    if building.inbound_haul > 0.0 {
        out.push_str(&format!(
            "\ninbound: {:.1} units en route",
            building.inbound_haul
        ));
    } else {
        out.push_str("\ninbound: none");
    }
    if !building.input_inventory.is_empty() {
        out.push_str(&format!(
            "\nlocal input: {}",
            building
                .input_inventory
                .iter()
                .map(|stack| format!("{} {:.1}", resource_kind_name(stack.kind), stack.amount))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !building.output_inventory.is_empty() {
        out.push_str(&format!(
            "\nlocal output: {}",
            building
                .output_inventory
                .iter()
                .map(|stack| format!("{} {:.1}", resource_kind_name(stack.kind), stack.amount))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !building.production_queue.is_empty() {
        out.push_str(&format!(
            "\nqueue: {}",
            building
                .production_queue
                .iter()
                .map(|entry| format!("{}{}", entry.recipe_id, if entry.repeat { "*" } else { "" }))
                .collect::<Vec<_>>()
                .join(" -> ")
        ));
    }
    if building.production_paused {
        out.push_str("\nproduction paused");
    }
    if let Some(reason) = &building.production_block_reason {
        out.push_str(&format!("\nblocked: {}", reason.replace('_', " ")));
    }
    if let Some(travel) = &building.worker_travel {
        out.push_str(&format!("\nworker: {travel}"));
    }
    if building.outbound_haul > 0.0 {
        out.push_str(&format!(
            "\noutbound: {:.1} units en route",
            building.outbound_haul
        ));
    } else {
        out.push_str("\noutbound: none");
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
    c.food
        + c.water
        + c.herbs
        + c.catnip
        + c.grain
        + c.flour
        + c.materials
        + c.refined
        + c.weapons
        + c.armor
        + c.logs
        + c.lumber
        + c.fibre
        + c.hide
        + c.cloth
        + c.leather
        + c.ore
        + c.metal
}

/// The single largest storable resource in a pile, or `None` when it's empty.
fn dominant_resource(c: &ResourceAmounts) -> Option<ResourceKind> {
    [
        (ResourceKind::Food, c.food),
        (ResourceKind::Water, c.water),
        (ResourceKind::Herbs, c.herbs),
        (ResourceKind::Catnip, c.catnip),
        (ResourceKind::Grain, c.grain),
        (ResourceKind::Flour, c.flour),
        (ResourceKind::Materials, c.materials),
        (ResourceKind::Refined, c.refined),
        (ResourceKind::Weapons, c.weapons),
        (ResourceKind::Armor, c.armor),
        (ResourceKind::Logs, c.logs),
        (ResourceKind::Lumber, c.lumber),
        (ResourceKind::Fibre, c.fibre),
        (ResourceKind::Hide, c.hide),
        (ResourceKind::Cloth, c.cloth),
        (ResourceKind::Leather, c.leather),
        (ResourceKind::Ore, c.ore),
        (ResourceKind::Metal, c.metal),
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
        ResourceKind::Catnip | ResourceKind::Grain | ResourceKind::Flour => PropTexture::Sack,
        ResourceKind::Materials => PropTexture::StonePile,
        ResourceKind::Refined => PropTexture::GoldPile,
        ResourceKind::Logs | ResourceKind::Lumber | ResourceKind::Planks => PropTexture::Crate,
        ResourceKind::Blocks => PropTexture::StonePile,
        ResourceKind::Tools => PropTexture::GoldPile,
        ResourceKind::Fibre | ResourceKind::Cloth => PropTexture::Haystack,
        ResourceKind::Hide | ResourceKind::Leather => PropTexture::Sack,
        ResourceKind::Ore => PropTexture::StonePile,
        ResourceKind::Metal => PropTexture::GoldPile,
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
        ResourceKind::Catnip => "catnip",
        ResourceKind::Grain => "grain",
        ResourceKind::Flour => "flour",
        ResourceKind::Materials => "materials",
        ResourceKind::Refined => "refined",
        ResourceKind::Weapons => "weapons",
        ResourceKind::Armor => "armor",
        ResourceKind::Logs => "logs",
        ResourceKind::Lumber => "lumber",
        ResourceKind::Planks => "planks",
        ResourceKind::Blocks => "blocks",
        ResourceKind::Tools => "tools",
        ResourceKind::Fibre => "fibre",
        ResourceKind::Hide => "hide",
        ResourceKind::Cloth => "cloth",
        ResourceKind::Leather => "leather",
        ResourceKind::Ore => "ore",
        ResourceKind::Metal => "metal",
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

fn point_in_farm(tile: (i32, i32), farm: &FarmSnapshot) -> bool {
    let (x0, x1) = (farm.x1.min(farm.x2), farm.x1.max(farm.x2));
    let (y0, y1) = (farm.y1.min(farm.y2), farm.y1.max(farm.y2));
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
/// adult 24–240, elder 240+.
fn life_stage(age_hours: f64) -> &'static str {
    match age_hours {
        a if a < 6.0 => "kitten",
        a if a < 24.0 => "young",
        a if a < 240.0 => "adult",
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

/// Adventure progress-art band for a need level: green when comfortable,
/// parchment when low, and red when critical.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum NeedBarBand {
    Comfortable,
    Low,
    Critical,
}

fn need_bar_band(value: f64) -> NeedBarBand {
    if value >= 60.0 {
        NeedBarBand::Comfortable
    } else if value >= 30.0 {
        NeedBarBand::Low
    } else {
        NeedBarBand::Critical
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
    let housing = match cat.housing_status {
        CatHousingStatus::Housed => "housing: housed".to_owned(),
        CatHousingStatus::Unhoused => "housing: unhoused — build a den".to_owned(),
        CatHousingStatus::Probationary => {
            let remaining = cat.probation_remaining_game_minutes.unwrap_or(0);
            let hours = remaining / 60;
            let minutes = remaining % 60;
            if remaining == 0 {
                "housing: awaiting home — leaves now unless a den opens".to_owned()
            } else {
                format!("housing: awaiting home — {hours}h {minutes:02}m left; build a den")
            }
        }
    };
    let preferred = if cat.preferred_labors.is_empty() {
        "none".to_owned()
    } else {
        cat.preferred_labors
            .iter()
            .copied()
            .map(labor_name)
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{name}\n\
         {spec} - {stage} ({age:.0}h)\n\
         at {x},{y} - {activity}\n\
         dest {dest}\n\
         carrying {carrying}{parents}{expecting}\n\
         {housing}\n\
         \n\
         skills: {skills}\n\
         prefers: {preferred}\n\
         leadership {lead:.0}",
        name = cat.name,
        spec = specialization_name(cat.specialization),
        stage = life_stage(cat.age_hours),
        age = cat.age_hours,
        x = cat.position.x,
        y = cat.position.y,
        activity = activity_name(cat.activity),
        skills = cat_skills_line(&cat.skills, &cat.role_xp),
        lead = cat.stats.leadership,
        housing = housing,
        preferred = preferred,
    )
}

/// One-line summary of a cat's role experience (skills).
fn cat_skills_line(skills: &BTreeMap<Labor, f64>, legacy: &RoleXp) -> String {
    if skills.is_empty() {
        return format!(
            "hunt {h:.0} build {b:.0} ritual {r:.0} war {w:.0}",
            h = legacy.hunter,
            b = legacy.architect,
            r = legacy.ritualist,
            w = legacy.warrior,
        );
    }
    skills
        .iter()
        .filter(|(_, xp)| **xp > 0.0)
        .map(|(labor, xp)| format!("{} {:.1}", labor_label(*labor), xp))
        .collect::<Vec<_>>()
        .join(" · ")
}

fn labor_label(labor: Labor) -> &'static str {
    match labor {
        Labor::Hunt => "hunt",
        Labor::Fishing => "fish",
        Labor::Build => "build",
        Labor::Ritual => "ritual",
        Labor::Fight => "fight",
        Labor::Train => "train",
        Labor::Quarry => "quarry",
        Labor::Woodcut => "woodcut",
        Labor::Forage => "forage",
        Labor::FetchWater => "water",
        Labor::Mill => "mill",
        Labor::Process => "process",
        Labor::Craft => "craft",
        Labor::Textile => "textile",
        Labor::Metalwork => "metal",
        Labor::Farm => "farm",
        Labor::Haul => "haul",
        Labor::Research => "research",
        Labor::Scout => "scout",
    }
}

// ---- pure building sprite / label helpers (unit-tested) ----

/// Pure geometry used by the renderer. Keeping this independent of ECS makes
/// footprint placement and screen-clipping guardrails cheap to test.
#[derive(Clone, Copy, PartialEq, Debug)]
struct BuildingRenderLayout {
    facade_base: Vec2,
    facade_size: Vec2,
}

fn building_render_layout(nw: TilePoint, footprint: FootprintSize) -> BuildingRenderLayout {
    let w = footprint.width.max(1) as f32;
    let h = footprint.height.max(1) as f32;
    let floor_size = Vec2::new(w * TILE, h * TILE);
    let floor_center = Vec2::new(
        (nw.x as f32 + (w - 1.0) / 2.0) * TILE,
        -(nw.y as f32 + (h - 1.0) / 2.0) * TILE,
    );
    let facade_base = Vec2::new(floor_center.x, -(nw.y as f32 + h - 1.0) * TILE - TILE / 2.0);

    // Residential cottage art is square. Cap it at one tile of north-side roof
    // overhang so a roof remains readable without covering the next block.
    let mut facade_size = Vec2::splat(floor_size.x);
    let max_height = floor_size.y + TILE;
    if facade_size.y > max_height {
        let scale = max_height / facade_size.y;
        facade_size *= scale;
    }

    BuildingRenderLayout {
        facade_base,
        facade_size,
    }
}

/// Pure sprite geometry for one normalized open-station prop.
#[derive(Clone, Copy, PartialEq, Debug)]
struct StationPropGeometry {
    center: Vec2,
    size: Vec2,
    base_y: f32,
}

fn station_prop_geometry(
    nw: TilePoint,
    footprint: FootprintSize,
    placement: PropPlacement,
) -> StationPropGeometry {
    let w = footprint.width.max(1) as f32;
    let h = footprint.height.max(1) as f32;
    let floor_size = Vec2::new(w * TILE, h * TILE);
    let (native_w, native_h) = placement.prop.native_px();
    let mut size = Vec2::new(native_w as f32 / 16.0 * TILE, native_h as f32 / 16.0 * TILE);
    let fit = (floor_size.x * 0.88 / size.x)
        .min(floor_size.y * 0.88 / size.y)
        .min(1.0);
    size *= fit;

    let left = (nw.x as f32 - 0.5) * TILE;
    let top = (-nw.y as f32 + 0.5) * TILE;
    let bottom = top - floor_size.y;
    let authored = Vec2::new(
        left + floor_size.x * placement.x as f32 / 1000.0,
        top - floor_size.y * placement.y as f32 / 1000.0,
    );
    let center = Vec2::new(
        authored
            .x
            .clamp(left + size.x / 2.0, left + floor_size.x - size.x / 2.0),
        authored.y.clamp(bottom + size.y / 2.0, top - size.y / 2.0),
    );

    StationPropGeometry {
        center,
        size,
        base_y: center.y - size.y / 2.0,
    }
}

fn construction_tint(complete: bool) -> Color {
    if complete {
        Color::WHITE
    } else {
        Color::srgba(0.72, 0.72, 0.70, 0.72)
    }
}

/// Residential aliases get restrained tints so their roles remain distinct at
/// a glance while hover/click inspection keeps the exact names available.
fn building_sprite_color(building: BuildingType, complete: bool) -> Color {
    if !complete {
        return construction_tint(false);
    }
    match building {
        BuildingType::Beds => Color::srgb(0.78, 0.88, 1.0),
        BuildingType::Nursery => Color::srgb(1.0, 0.83, 0.86),
        BuildingType::ElderCorner => Color::srgb(0.82, 0.78, 0.72),
        _ => Color::WHITE,
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
        BuildingType::AccountingTent => "accounting",
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
        BuildingType::Mill => "mill",
        BuildingType::Sawmill => "sawmill",
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
        CarryingKind::Logs => Color::srgb(0.45, 0.29, 0.17),
        CarryingKind::Lumber | CarryingKind::Planks => Color::srgb(0.70, 0.47, 0.25),
        CarryingKind::Blocks => Color::srgb(0.58, 0.60, 0.64),
        CarryingKind::Tools => Color::srgb(0.76, 0.78, 0.84),
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

    fn ready_session() -> Session {
        Session {
            session_id: "session-1".to_owned(),
            sig: "signed".to_owned(),
            presence_sent: true,
            ready: true,
        }
    }

    #[test]
    fn manual_orders_shortcut_does_not_conflict_with_camera_or_officers() {
        assert_ne!(ORDERS_SHORTCUT, CAMERA_RESET_SHORTCUT);
        assert_ne!(ORDERS_SHORTCUT, OFFICERS_SHORTCUT);
        assert_eq!(ORDERS_SHORTCUT, KeyCode::KeyP);
    }

    #[test]
    fn pressed_manual_order_button_queues_a_signed_action() {
        let mut app = App::new();
        app.insert_resource(ready_session())
            .insert_resource(OrdersUi {
                visible: true,
                ..OrdersUi::default()
            })
            .insert_resource(Selection::default())
            .insert_resource(BuildingSelection::default())
            .insert_resource(StockpileSelection::default())
            .insert_resource(OutgoingActions::default())
            .insert_resource(ClientFeedback::default())
            .insert_resource(Tools::default())
            .add_systems(Update, handle_order_buttons);
        app.world_mut()
            .spawn((Interaction::Pressed, OrderButton(OrderAction::Quarry)));

        app.update();

        let queued = &app.world().resource::<OutgoingActions>().0;
        assert!(matches!(
            queued.as_slice(),
            [ClientAction::RequestJob {
                session_id,
                sig,
                kind: JobKind::Quarry,
                ..
            }] if session_id == "session-1" && sig == "signed"
        ));
    }

    #[test]
    fn manual_order_builder_covers_every_button_and_signs_every_action() {
        let session = ready_session();
        let signed = |action| {
            build_order_action(
                action,
                &session,
                Some("cat-1"),
                Some("mill-1"),
                Some("gather-1"),
                BuildingType::Sawmill,
            )
            .unwrap()
        };
        let expected = [
            ClientAction::RequestJob {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                kind: JobKind::HuntExpedition,
            },
            ClientAction::RequestJob {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                kind: JobKind::Fish,
            },
            ClientAction::RequestJob {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                kind: JobKind::FetchWater,
            },
            ClientAction::RequestJob {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                kind: JobKind::Quarry,
            },
            ClientAction::RequestJob {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                kind: JobKind::GatherLogs,
            },
            ClientAction::RequestJob {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                kind: JobKind::ForageFibre,
            },
            ClientAction::RequestJob {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                kind: JobKind::ExpandVillage,
            },
            ClientAction::RequestJob {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                kind: JobKind::Ritual,
            },
            ClientAction::OfferTithe {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
            },
            ClientAction::OfferMaterials {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
            },
            ClientAction::HaulGatherSpot {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                stockpile_id: "gather-1".to_owned(),
                cat_id: Some("cat-1".to_owned()),
            },
            ClientAction::PlanBuilding {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                building_type: BuildingType::Sawmill,
                site: None,
            },
            ClientAction::AssignWorker {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                cat_id: "cat-1".to_owned(),
                building_id: Some("mill-1".to_owned()),
            },
            ClientAction::AssignWorker {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                cat_id: "cat-1".to_owned(),
                building_id: None,
            },
            ClientAction::TrainWarrior {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                cat_id: Some("cat-1".to_owned()),
            },
            ClientAction::DefendRaid {
                session_id: "session-1".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
            },
        ];
        for (action, expected) in OrderAction::ALL.into_iter().zip(expected) {
            assert_eq!(signed(action), expected, "{action:?}");
        }

        let mut unavailable = ready_session();
        unavailable.ready = false;
        for action in OrderAction::ALL {
            assert_eq!(
                build_order_action(
                    action,
                    &unavailable,
                    Some("cat-1"),
                    Some("mill-1"),
                    Some("gather-1"),
                    BuildingType::Mill,
                ),
                Err("The server session is not ready."),
                "{action:?}"
            );
        }

        assert_eq!(
            build_order_action(
                OrderAction::HaulSelected,
                &session,
                Some("cat-1"),
                Some("mill-1"),
                None,
                BuildingType::Mill,
            ),
            Err("Select a gather spot first.")
        );
        assert_eq!(
            build_order_action(
                OrderAction::StaffSelected,
                &session,
                None,
                Some("mill-1"),
                Some("gather-1"),
                BuildingType::Mill,
            ),
            Err("Select a cat first.")
        );
        assert_eq!(
            build_order_action(
                OrderAction::StaffSelected,
                &session,
                Some("cat-1"),
                None,
                Some("gather-1"),
                BuildingType::Mill,
            ),
            Err("Select a building first.")
        );
        assert_eq!(
            build_order_action(
                OrderAction::UnstaffSelected,
                &session,
                None,
                Some("mill-1"),
                Some("gather-1"),
                BuildingType::Mill,
            ),
            Err("Select a cat first.")
        );
        assert_eq!(
            build_order_action(
                OrderAction::TrainSelected,
                &session,
                None,
                Some("mill-1"),
                Some("gather-1"),
                BuildingType::Mill,
            ),
            Err("Select a cat first.")
        );
    }

    #[test]
    fn exact_tool_choices_cover_all_farms_and_supported_gather_resources() {
        assert_eq!(next_crop(CropKind::Catnip), CropKind::Grain);
        assert_eq!(next_crop(CropKind::Grain), CropKind::Herb);
        assert_eq!(next_crop(CropKind::Herb), CropKind::Catnip);
        let mut kind = GATHER_KINDS[0];
        let mut seen = Vec::new();
        for _ in 0..GATHER_KINDS.len() {
            seen.push(kind);
            kind = next_gather_kind(kind);
        }
        assert_eq!(seen, GATHER_KINDS);
        assert_eq!(kind, GATHER_KINDS[0]);
        assert_eq!(ToolMode::Building.paint_kind(), None);
        assert_eq!(ToolMode::Building.label(), "Building");
        assert_eq!(
            ToolMode::FishingSpot.paint_kind(),
            Some(PaintKind::FishingSpot)
        );
        assert_eq!(ToolMode::FishingSpot.label(), "Fishing spot");
        assert_eq!(labor_name(Labor::Fishing), "fishing");
        assert_eq!(labor_label(Labor::Fishing), "fish");
        assert_eq!(
            build_exact_building_action(
                &signed_session("builder-session"),
                BuildingType::Mill,
                TilePoint { x: -4, y: 9 },
            ),
            ClientAction::PlanBuilding {
                session_id: "builder-session".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                building_type: BuildingType::Mill,
                site: Some(TilePoint { x: -4, y: 9 }),
            }
        );
    }

    #[test]
    fn designation_remove_builder_emits_each_signed_lifecycle_action() {
        let session = signed_session("designation-session");
        assert!(matches!(
            build_remove_action(&session, Some("farm-1".to_owned()), None, false),
            ClientAction::ClearFarm { session_id, sig, plot_id, .. }
                if session_id == "designation-session" && sig == "signed" && plot_id == "farm-1"
        ));
        assert!(matches!(
            build_remove_action(&session, None, Some("pile-1".to_owned()), false),
            ClientAction::RemoveStockpile { stockpile_id, .. } if stockpile_id == "pile-1"
        ));
        assert!(matches!(
            build_remove_action(&session, None, Some("gather-1".to_owned()), true),
            ClientAction::RemoveGatherSpot { stockpile_id, .. } if stockpile_id == "gather-1"
        ));
    }

    #[test]
    fn live_election_controls_emit_candidate_vote_and_pending_petition_signature() {
        let mut world = village_world(&["alpha"]);
        let colony = &mut world.colonies[0];
        colony.leader = Some(cat_protocol::LeaderSnapshot {
            id: "leader-1".to_owned(),
            name: "Oak".to_owned(),
            leadership: 80.0,
        });
        colony.election = Some(cat_protocol::ElectionSnapshot {
            id: "election-1".to_owned(),
            ends_at: 99,
            tally: Default::default(),
            total_ballots: 0,
            candidates: vec![cat_protocol::ElectionCandidate {
                id: "candidate-1".to_owned(),
                name: "Moss".to_owned(),
                leadership: 72.0,
                specialization: None,
            }],
        });
        colony.vote_kick = Some(cat_protocol::VoteKickSnapshot {
            id: "kick-1".to_owned(),
            ends_at: 77,
            target_cat_id: "leader-1".to_owned(),
            target_name: "Oak".to_owned(),
            signatures: 2,
            needed: 5,
        });
        let mut app = App::new();
        app.insert_resource(signed_session("governance-session"))
            .insert_resource(LatestSnapshot(Some(world)))
            .insert_resource(GovernanceUi::default())
            .insert_resource(OutgoingActions::default())
            .add_systems(Update, handle_governance_buttons);
        app.world_mut()
            .spawn((Interaction::Pressed, CastElectionVoteButton));
        app.world_mut()
            .spawn((Interaction::Pressed, RequestVoteKickButton));
        app.update();
        let actions = &app.world().resource::<OutgoingActions>().0;
        assert!(matches!(
            &actions[0],
            ClientAction::CastVote { election_id, cat_id, .. }
                if election_id == "election-1" && cat_id == "candidate-1"
        ));
        assert!(matches!(
            &actions[1],
            ClientAction::RequestVoteKick { session_id, sig, .. }
                if session_id == "governance-session" && sig == "signed"
        ));
    }

    #[test]
    fn adventure_buttons_have_distinct_interaction_and_disabled_states() {
        assert_eq!(
            adventure_button_appearance(Interaction::None, false, false).texture,
            AdventureButtonTexture::Brown
        );
        assert_ne!(
            adventure_button_appearance(Interaction::Hovered, false, false).tint,
            adventure_button_appearance(Interaction::None, false, false).tint
        );
        assert_ne!(
            adventure_button_appearance(Interaction::Hovered, false, false).fallback,
            adventure_button_appearance(Interaction::None, false, false).fallback
        );
        assert_eq!(
            adventure_button_appearance(Interaction::Pressed, false, false).texture,
            AdventureButtonTexture::Red
        );
        assert_eq!(
            adventure_button_appearance(Interaction::None, true, false).texture,
            AdventureButtonTexture::Red
        );
        assert_eq!(
            adventure_button_appearance(Interaction::Pressed, true, true).texture,
            AdventureButtonTexture::Grey
        );
    }

    #[test]
    fn visual_disabled_state_is_mirrored_to_bevy_interaction_state() {
        let mut app = App::new();
        app.insert_resource(AdventureUiArt::default())
            .add_systems(Update, update_kit_buttons);
        let button = app
            .world_mut()
            .spawn((ui_button(), KitDisabled { disabled: true }))
            .id();

        app.update();
        assert!(app.world().entity(button).contains::<InteractionDisabled>());

        app.world_mut()
            .entity_mut(button)
            .get_mut::<KitDisabled>()
            .unwrap()
            .disabled = false;
        app.update();
        assert!(!app.world().entity(button).contains::<InteractionDisabled>());
    }

    #[test]
    fn adventure_cursor_prioritises_pressed_disabled_hover_and_map_tools() {
        assert_eq!(
            adventure_cursor_kind([(Interaction::None, false)], false, false),
            AdventureCursorKind::Pointer
        );
        assert_eq!(
            adventure_cursor_kind([(Interaction::None, false)], true, false),
            AdventureCursorKind::Target
        );
        assert_eq!(
            adventure_cursor_kind([(Interaction::None, false)], true, true),
            AdventureCursorKind::Pointer
        );
        assert_eq!(
            adventure_cursor_kind([(Interaction::Hovered, false)], true, true),
            AdventureCursorKind::Interact
        );
        assert_eq!(
            adventure_cursor_kind([(Interaction::Hovered, true)], true, true),
            AdventureCursorKind::Disabled
        );
        assert_eq!(
            adventure_cursor_kind([(Interaction::Pressed, true)], true, true),
            AdventureCursorKind::Disabled
        );
        assert_eq!(
            adventure_cursor_kind(
                [(Interaction::Hovered, true), (Interaction::Pressed, false),],
                true,
                true,
            ),
            AdventureCursorKind::Pressed
        );
        assert_eq!(
            adventure_cursor_hotspot(AdventureCursorKind::Pointer),
            (8, 6)
        );
    }

    #[test]
    fn marked_visible_ui_blocks_every_world_pointer_path() {
        assert!(ui_surface_blocks_world(Display::Flex, true));
        assert!(!ui_surface_blocks_world(Display::None, true));
        assert!(!ui_surface_blocks_world(Display::Flex, false));
        assert!(world_pointer_input_allowed(false, false, true));
        assert!(!world_pointer_input_allowed(false, true, true));
        assert!(!world_pointer_input_allowed(true, false, true));
        assert!(!world_pointer_input_allowed(false, false, false));
    }

    #[test]
    fn adventure_slicers_preserve_authored_corners() {
        assert_eq!(panel_slicer().border, BorderRect::all(PANEL_SLICE_PX));
        assert_eq!(
            button_slicer().border,
            BorderRect::axes(BUTTON_SLICE_X_PX, BUTTON_SLICE_Y_PX)
        );
        assert_eq!(panel_slicer().max_corner_scale, 1.0);
        assert_eq!(button_slicer().max_corner_scale, 1.0);
    }

    #[test]
    fn every_runtime_adventure_skin_asset_is_a_tracked_png() {
        let images: [&[u8]; 18] = [
            include_bytes!("../../../public/images/game/ui/panel.png"),
            include_bytes!("../../../public/images/game/ui/panel-dark.png"),
            include_bytes!("../../../public/images/game/ui/panel-ornate.png"),
            include_bytes!("../../../public/images/game/ui/button.png"),
            include_bytes!("../../../public/images/game/ui/button-active.png"),
            include_bytes!("../../../public/images/game/ui/button-disabled.png"),
            include_bytes!("../../../public/images/game/ui/progress-track.png"),
            include_bytes!("../../../public/images/game/ui/progress-good.png"),
            include_bytes!("../../../public/images/game/ui/progress-mid.png"),
            include_bytes!("../../../public/images/game/ui/progress-low.png"),
            include_bytes!("../../../public/images/game/ui/banner.png"),
            include_bytes!("../../../public/images/game/ui/icon-frame.png"),
            include_bytes!("../../../public/images/game/ui/minimap-ring.png"),
            include_bytes!("../../../public/images/game/ui/cursor/pointer.png"),
            include_bytes!("../../../public/images/game/ui/cursor/interact.png"),
            include_bytes!("../../../public/images/game/ui/cursor/pressed.png"),
            include_bytes!("../../../public/images/game/ui/cursor/target.png"),
            include_bytes!("../../../public/images/game/ui/cursor/disabled.png"),
        ];
        assert!(
            images
                .iter()
                .all(|image| image.starts_with(b"\x89PNG\r\n\x1a\n"))
        );
    }

    #[test]
    fn founding_a_village_waits_for_and_uses_the_signed_session() {
        assert!(build_action(ButtonAction::FoundVillage, &Session::default()).is_none());

        let session = Session {
            session_id: "signed-session".to_owned(),
            sig: "signed".to_owned(),
            presence_sent: true,
            ready: true,
        };
        assert_eq!(
            build_action(ButtonAction::FoundVillage, &session),
            Some(ClientAction::FoundVillage {
                name: "Forest Hollow".to_owned(),
                session_id: "signed-session".to_owned(),
                sig: Some("signed".to_owned()),
            })
        );
    }

    #[test]
    fn scout_toolbar_buttons_emit_typed_signed_missions() {
        let session = signed_session("scout-session");
        for (button, mission) in [
            (
                ButtonAction::ScoutWood,
                ScoutMission::Resource(ScoutResource::Wood),
            ),
            (
                ButtonAction::ScoutFood,
                ScoutMission::Resource(ScoutResource::Food),
            ),
            (
                ButtonAction::ScoutWater,
                ScoutMission::Resource(ScoutResource::Water),
            ),
            (
                ButtonAction::ScoutStone,
                ScoutMission::Resource(ScoutResource::Stone),
            ),
            (ButtonAction::Explore, ScoutMission::Explore),
        ] {
            assert_eq!(
                build_action(button, &session),
                Some(ClientAction::DispatchScout {
                    session_id: "scout-session".to_owned(),
                    nickname: "Desktop Cat".to_owned(),
                    sig: "signed".to_owned(),
                    mission,
                })
            );
        }
    }

    #[test]
    fn bottom_toolbar_rows_wrap_inside_a_viewport_bounded_panel() {
        let panel = bottom_bar_panel_node();
        assert_eq!(panel.width, Val::Percent(96.0));
        assert_eq!(panel.max_width, Val::Px(1180.0));

        let row = bottom_bar_row_node();
        assert_eq!(row.width, Val::Percent(100.0));
        assert_eq!(row.flex_wrap, FlexWrap::Wrap);
        assert_eq!(row.justify_content, JustifyContent::Center);
        assert_eq!(row.row_gap, Val::Px(UI_GAP));
        assert_eq!(NARROW_BOTTOM_BAR_FOOTPRINT, 129.0);
        assert_eq!(BOTTOM_OVERLAY_CLEARANCE, 135.0);
        const { assert!(BOTTOM_OVERLAY_CLEARANCE > NARROW_BOTTOM_BAR_FOOTPRINT) };
        const { assert!(HUD_RESOURCE_PILL_HEIGHT <= 20.0) };
    }

    fn village_colony(id: &str, name: &str, population: u32, status: &str) -> ColonySnapshot {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": name,
            "status": status,
            "resources": {
                "food": 1, "water": 1, "herbs": 0, "materials": 0,
                "refined": 0, "weapons": 0, "armor": 0, "blessings": 0
            },
            "storage": {
                "capacities": {
                    "food": 200, "water": 200, "herbs": 100,
                    "materials": 100, "refined": 100
                },
                "foodCapacity": 200,
                "titheRates": { "food": 20, "refined": 5 }
            },
            "leader": null,
            "cats": [],
            "jobs": [],
            "upgrades": [],
            "events": [],
            "housing": {
                "population": population, "capacity": 20,
                "pressure": 0.5, "villageLevel": 1
            },
            "research": {
                "ownedNodeIds": [], "researchPoints": 0,
                "researcherCount": 0, "blessings": 0, "nextTarget": null
            },
            "election": null,
            "voteKick": null,
            "zones": [],
            "threat": {
                "pressure": 0, "band": "calm", "raidActive": false,
                "warriors": 0, "weapons": 0, "armor": 0
            },
            "raiders": [],
            "buildings": [],
            "claimedTiles": [],
            "villageGate": null,
            "villageRadius": 4,
            "anchor": { "x": 6, "y": 6 }
        }))
        .expect("valid village fixture")
    }

    fn village_world(order: &[&str]) -> WorldSnapshot {
        let colonies = order
            .iter()
            .map(|id| match *id {
                "alpha" => village_colony("alpha", "Moss Hollow", 5, "thriving"),
                "beta" => village_colony("beta", "River Paws", 9, "struggling"),
                other => village_colony(other, "Unknown", 0, "starting"),
            })
            .collect();
        WorldSnapshot {
            now: 1,
            world_seed: 7,
            colonies,
            online_count: 1,
            selected_colony_id: order.first().map(|id| (*id).to_owned()),
            known_villages: Vec::new(),
            village_trade_offers: Vec::new(),
        }
    }

    fn signed_session(id: &str) -> Session {
        Session {
            session_id: id.to_owned(),
            sig: "signed".to_owned(),
            presence_sent: true,
            ready: true,
        }
    }

    #[test]
    fn durable_session_json_round_trips_and_presence_reuses_it() {
        let session = signed_session("stable-player-session");
        let selection = VillageSelection {
            selected_id: Some("my-village".to_owned()),
            join_required: false,
        };
        let raw = stored_session_json(&session, &selection).expect("complete session serializes");
        assert_eq!(
            parse_stored_session(&raw),
            Some(StoredSession {
                session_id: "stable-player-session".to_owned(),
                sig: "signed".to_owned(),
                selected_colony_id: Some("my-village".to_owned()),
            })
        );
        assert_eq!(
            presence_action(&session),
            ClientAction::Presence {
                session_id: "stable-player-session".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: Some("signed".to_owned()),
            }
        );
        assert_eq!(
            presence_action(&Session::default()),
            ClientAction::Presence {
                session_id: "desktop".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: None,
            }
        );

        assert_eq!(
            parse_stored_session(r#"{"sessionId":"legacy","sig":"signed"}"#),
            Some(StoredSession {
                session_id: "legacy".to_owned(),
                sig: "signed".to_owned(),
                selected_colony_id: None,
            }),
            "pre-selector bearer files remain valid"
        );
    }

    #[test]
    fn startup_restore_marks_the_persisted_village_for_authenticated_rejoin() {
        let mut session = Session::default();
        let mut selection = VillageSelection::default();
        restore_stored_session(
            Some(StoredSession {
                session_id: "restored-session".to_owned(),
                sig: "restored-signature".to_owned(),
                selected_colony_id: Some("restored-village".to_owned()),
            }),
            &mut session,
            &mut selection,
        );

        assert_eq!(session.session_id, "restored-session");
        assert_eq!(session.sig, "restored-signature");
        assert_eq!(selection.selected_id.as_deref(), Some("restored-village"));
        assert!(selection.join_required);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_session_file_round_trips_without_losing_the_bearer() {
        let path = std::env::temp_dir().join(format!(
            "idle-cat-forest-session-test-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let selection = VillageSelection {
            selected_id: Some("remembered-village".to_owned()),
            join_required: false,
        };
        let raw = stored_session_json(&signed_session("native-session"), &selection)
            .expect("session json");

        save_session_to_path(&path, &raw).expect("save session");
        let restored = load_session_from_path(&path);
        #[cfg(unix)]
        assert_eq!(
            std::fs::metadata(&path)
                .expect("session metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600,
            "native bearer file must remain private"
        );
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            restored,
            Some(StoredSession {
                session_id: "native-session".to_owned(),
                sig: "signed".to_owned(),
                selected_colony_id: Some("remembered-village".to_owned()),
            })
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn failed_native_session_replacement_preserves_the_previous_bearer() {
        let path = std::env::temp_dir().join(format!(
            "idle-cat-forest-session-failure-test-{}.json",
            std::process::id()
        ));
        let temp_path = native_session_temp_path(&path);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&temp_path);
        let _ = std::fs::remove_dir(&temp_path);
        let original = StoredSession {
            session_id: "original-session".to_owned(),
            sig: "original-signature".to_owned(),
            selected_colony_id: Some("original-village".to_owned()),
        };
        let raw = serde_json::json!({
            "sessionId": original.session_id.clone(),
            "sig": original.sig.clone(),
            "selectedColonyId": original.selected_colony_id.clone(),
        })
        .to_string();
        save_session_to_path(&path, &raw).expect("save original bearer");

        std::fs::create_dir(&temp_path).expect("block temporary-file creation");
        assert!(save_session_to_path(&path, r#"{"sessionId":"replacement","sig":"new"}"#).is_err());
        assert_eq!(load_session_from_path(&path), Some(original));

        std::fs::remove_dir(&temp_path).expect("remove blocker");
        std::fs::remove_file(&path).expect("remove bearer fixture");
    }

    #[test]
    fn village_selector_labels_global_owned_and_known_villages() {
        assert_eq!(
            village_group_label(VillageKind::Global, false),
            "Grand Commons"
        );
        assert_eq!(
            village_group_label(VillageKind::Personal, true),
            "My Village"
        );
        assert_eq!(village_group_label(VillageKind::Personal, false), "Known");
    }

    #[test]
    fn discovered_village_trade_controls_emit_exact_signed_actions() {
        let session = signed_session("trade-session");
        let draft = VillageTradeDraft {
            offered_kind: ResourceKind::Water,
            offered_amount: 25.0,
            requested_kind: ResourceKind::Lumber,
            requested_amount: 10.0,
        };
        assert_eq!(
            village_trade_proposal_action("reed-rest", &draft, &session),
            Some(ClientAction::OfferVillageTrade {
                session_id: "trade-session".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                target_colony_id: "reed-rest".to_owned(),
                offered_kind: ResourceKind::Water,
                offered_amount: 25.0,
                requested_kind: ResourceKind::Lumber,
                requested_amount: 10.0,
            })
        );
        assert_eq!(
            village_trade_reply_action("offer-1", true, &session),
            ClientAction::AcceptVillageTrade {
                session_id: "trade-session".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                offer_id: "offer-1".to_owned(),
            }
        );
        assert!(matches!(
            village_trade_reply_action("offer-1", false, &session),
            ClientAction::CancelVillageTrade { offer_id, .. } if offer_id == "offer-1"
        ));
        assert!(village_trade_proposal_action("reed-rest", &draft, &Session::default()).is_none());
    }

    #[test]
    fn village_trade_draft_cycles_resources_and_amounts_without_same_kind() {
        let mut draft = VillageTradeDraft::default();
        cycle_village_trade_draft(&mut draft, VillageTradeDraftField::OfferedKind);
        cycle_village_trade_draft(&mut draft, VillageTradeDraftField::OfferedAmount);
        cycle_village_trade_draft(&mut draft, VillageTradeDraftField::RequestedKind);
        cycle_village_trade_draft(&mut draft, VillageTradeDraftField::RequestedAmount);

        assert_eq!(draft.offered_kind, ResourceKind::Water);
        assert_eq!(draft.offered_amount, 10.0);
        assert_eq!(draft.requested_kind, ResourceKind::Refined);
        assert_eq!(draft.requested_amount, 10.0);
        assert_ne!(draft.offered_kind, draft.requested_kind);
    }

    #[test]
    fn discovered_full_colonies_are_available_as_trade_targets() {
        let mut snapshot = village_world(&["alpha", "beta"]);
        snapshot.known_villages.push(cat_protocol::VillageSummary {
            id: "beta".to_owned(),
            name: "River Paws".to_owned(),
            kind: VillageKind::Personal,
            scale: VillageScale::Personal,
            anchor: TilePoint { x: 100, y: 6 },
            capabilities: Default::default(),
        });

        assert!(is_discovered_trade_target(&snapshot, Some("alpha"), "beta"));
        assert!(!is_discovered_trade_target(
            &snapshot,
            Some("alpha"),
            "alpha"
        ));
        assert!(!is_discovered_trade_target(
            &snapshot,
            Some("beta"),
            "alpha"
        ));
        assert!(!is_discovered_trade_target(&snapshot, None, "beta"));
    }

    #[test]
    fn village_selection_survives_snapshot_reordering() {
        let mut selection = VillageSelection {
            selected_id: Some("beta".to_owned()),
            join_required: false,
        };
        let mut first = village_world(&["alpha", "beta"]);
        assert!(reconcile_village_selection(&mut first, &mut selection, true).is_none());
        assert_eq!(first.colonies[0].id, "beta");

        let mut reordered = village_world(&["beta", "alpha"]);
        assert!(reconcile_village_selection(&mut reordered, &mut selection, true).is_none());
        assert_eq!(reordered.colonies[0].id, "beta");
        assert_eq!(selection.selected_id.as_deref(), Some("beta"));
    }

    #[test]
    fn pre_presence_public_snapshot_does_not_forget_a_private_selection() {
        let mut selection = VillageSelection {
            selected_id: Some("my-private-village".to_owned()),
            join_required: true,
        };
        let mut public = village_world(&["alpha"]);

        assert!(
            reconcile_village_selection(&mut public, &mut selection, false).is_none(),
            "pre-auth redaction is not a missing-village deletion"
        );
        assert_eq!(selection.selected_id.as_deref(), Some("my-private-village"));
        assert!(selection.join_required);

        let mut personalized = village_world(&["alpha", "my-private-village"]);
        assert!(reconcile_village_selection(&mut personalized, &mut selection, true).is_none());
        assert_eq!(personalized.colonies[0].id, "my-private-village");
    }

    #[test]
    fn procedural_decor_is_hidden_only_inside_the_selected_village_walls() {
        let mut snapshot = village_world(&["alpha", "beta"]);
        snapshot.colonies[0].anchor = TilePoint { x: 50, y: 50 };
        snapshot.colonies[0].claimed_tiles = vec![TilePoint { x: 50, y: 50 }];
        snapshot.colonies[1].anchor = TilePoint { x: -20, y: -20 };
        // The second tile is expanded claimed territory outside the permanent
        // wall core: a farm/work site there must keep its wilderness decor.
        snapshot.colonies[1].claimed_tiles =
            vec![TilePoint { x: -14, y: -20 }, TilePoint { x: -12, y: -20 }];
        let mut selection = VillageSelection {
            selected_id: Some("beta".to_owned()),
            join_required: false,
        };

        reconcile_village_selection(&mut snapshot, &mut selection, true);
        let anchor = snapshot.colonies[0].anchor;
        let rock = DecorationRole::Rock {
            size: RockSize::Small,
            resource: false,
        };

        assert!(!procedural_decoration_visible(anchor, -20, -20, rock));
        assert!(!procedural_decoration_visible(anchor, -14, -20, rock));
        assert!(procedural_decoration_visible(anchor, -12, -20, rock));
        assert!(procedural_decoration_visible(anchor, 50, 50, rock));

        let tree = DecorationRole::Tree { species: 0 };
        assert!(
            !procedural_decoration_visible(anchor, -13, -20, tree),
            "a 2x3 canopy that overhangs the wall is cleared as one object"
        );
    }

    #[test]
    fn selecting_a_village_switches_render_order_and_builds_join_action() {
        let mut snapshot = village_world(&["alpha", "beta"]);
        let mut selection = VillageSelection::default();
        reconcile_village_selection(&mut snapshot, &mut selection, true);
        let action = choose_village(
            "beta",
            &mut snapshot,
            &mut selection,
            &signed_session("fresh-session"),
        );

        assert_eq!(snapshot.colonies[0].id, "beta");
        assert_eq!(selection.selected_id.as_deref(), Some("beta"));
        assert!(!selection.join_required);
        assert_eq!(
            action,
            Some(ClientAction::JoinVillage {
                colony_id: "beta".to_owned(),
                session_id: "fresh-session".to_owned(),
                sig: Some("signed".to_owned()),
            })
        );
    }

    #[test]
    fn missing_selected_village_falls_back_to_first_available() {
        let mut snapshot = village_world(&["alpha"]);
        let mut selection = VillageSelection {
            selected_id: Some("gone".to_owned()),
            join_required: true,
        };
        assert_eq!(
            reconcile_village_selection(&mut snapshot, &mut selection, true),
            Some(("gone".to_owned(), "alpha".to_owned()))
        );
        assert_eq!(selection.selected_id.as_deref(), Some("alpha"));
        assert!(!selection.join_required);
    }

    #[test]
    fn reconnect_rejoins_persisted_village_with_new_session() {
        let snapshot = village_world(&["alpha", "beta"]);
        let mut selection = VillageSelection {
            selected_id: Some("beta".to_owned()),
            join_required: true,
        };
        let action = pending_village_rejoin(
            Some(&snapshot),
            &mut selection,
            &signed_session("replacement-session"),
        );
        assert_eq!(
            action,
            Some(ClientAction::JoinVillage {
                colony_id: "beta".to_owned(),
                session_id: "replacement-session".to_owned(),
                sig: Some("signed".to_owned()),
            })
        );
        assert!(!selection.join_required, "rejoin is emitted only once");
        assert!(
            pending_village_rejoin(
                Some(&snapshot),
                &mut selection,
                &signed_session("replacement-session")
            )
            .is_none()
        );
    }

    #[test]
    fn founding_and_selecting_are_independent_actions() {
        let actions = build_button_actions(
            ButtonAction::FoundVillage,
            &signed_session("signed-session"),
            Some("beta"),
        );
        assert_eq!(actions.len(), 2);
        assert!(matches!(actions[0], ClientAction::FoundVillage { .. }));
        assert_eq!(
            actions[1],
            ClientAction::JoinVillage {
                colony_id: "beta".to_owned(),
                session_id: "signed-session".to_owned(),
                sig: Some("signed".to_owned()),
            }
        );
    }

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
            skills: BTreeMap::new(),
            stats: CatStats { leadership: 0.0 },
            death_time: None,
            parent_ids: Vec::new(),
            parents: Vec::new(),
            boosted,
            preferred_labors: Vec::new(),
            pregnant: false,
            housing_status: cat_protocol::CatHousingStatus::Housed,
            probation_remaining_game_minutes: None,
        }
    }

    #[test]
    fn cat_inspector_makes_probation_deadline_and_housing_action_explicit() {
        let mut cat = census_cat(30.0, None, false);
        cat.housing_status = CatHousingStatus::Probationary;
        cat.probation_remaining_game_minutes = Some(2_161);
        let waiting = inspector_text(&cat);
        assert!(waiting.contains("housing: awaiting home"));
        assert!(waiting.contains("36h 01m left; build a den"));

        cat.probation_remaining_game_minutes = Some(0);
        assert!(inspector_text(&cat).contains("leaves now unless a den opens"));

        cat.housing_status = CatHousingStatus::Housed;
        cat.probation_remaining_game_minutes = None;
        assert!(inspector_text(&cat).contains("housing: housed"));
    }

    #[test]
    fn hud_res_maps_every_resource_kind_to_its_glyph() {
        // Every wire ResourceKind a gather spot can carry maps to a HUD glyph.
        assert_eq!(hud_res_of(ResourceKind::Food), HudRes::Food);
        assert_eq!(hud_res_of(ResourceKind::Water), HudRes::Water);
        assert_eq!(hud_res_of(ResourceKind::Herbs), HudRes::Herbs);
        assert_eq!(hud_res_of(ResourceKind::Catnip), HudRes::Catnip);
        assert_eq!(hud_res_of(ResourceKind::Grain), HudRes::Grain);
        assert_eq!(hud_res_of(ResourceKind::Flour), HudRes::Flour);
        assert_eq!(hud_res_of(ResourceKind::Materials), HudRes::Materials);
        assert_eq!(hud_res_of(ResourceKind::Refined), HudRes::Refined);
        assert_eq!(hud_res_of(ResourceKind::Weapons), HudRes::Weapons);
        assert_eq!(hud_res_of(ResourceKind::Armor), HudRes::Armor);
        assert_eq!(hud_res_of(ResourceKind::Logs), HudRes::Logs);
        assert_eq!(hud_res_of(ResourceKind::Lumber), HudRes::Lumber);
        assert_eq!(hud_res_of(ResourceKind::Fibre), HudRes::Herbs);
        assert_eq!(hud_res_of(ResourceKind::Hide), HudRes::Materials);
        assert_eq!(hud_res_of(ResourceKind::Cloth), HudRes::Herbs);
        assert_eq!(hud_res_of(ResourceKind::Leather), HudRes::Materials);
        assert_eq!(hud_res_of(ResourceKind::Ore), HudRes::Materials);
        assert_eq!(hud_res_of(ResourceKind::Metal), HudRes::Refined);
        assert_eq!(hud_res_of(ResourceKind::Blessings), HudRes::Blessings);
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
            census_cat(250.0, Some(Specialization::Warrior), false), // elder
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
        assert!((c.avg_age_hours - (3.0 + 10.0 + 30.0 + 250.0 + 40.0 + 28.0) / 6.0).abs() < 1e-9);
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
            colony_census(&[census_cat(240.0, None, false)], &[], None).elders,
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
        let lines = census_report_lines(&c, VillageScale::Communal);
        assert_eq!(lines.len(), CENSUS_LINES);
        assert_eq!(lines[0], "Communal population: 2");
        assert_eq!(lines[1], "Leader: Bella");
        assert!(lines[2].contains("★ Boosted: 1"));
        assert_eq!(lines[3], "Expecting: 1");
        // A vacant seat renders a placeholder rather than dropping the line.
        let vacant = census_report_lines(&colony_census(&[], &[], None), VillageScale::Personal);
        assert_eq!(vacant.len(), CENSUS_LINES);
        assert_eq!(vacant[0], "Personal population: 0");
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
    fn cat_labor_control_builds_exact_signed_toggle_from_snapshot_state() {
        let session = signed_session("labor-session");
        let mut cat = census_cat(30.0, None, false);
        cat.id = "worker-7".to_owned();
        assert_eq!(
            labor_preference_action(&session, &cat, Labor::Process),
            Some(ClientAction::SetCatLaborPreference {
                session_id: "labor-session".to_owned(),
                nickname: "Desktop Cat".to_owned(),
                sig: "signed".to_owned(),
                cat_id: "worker-7".to_owned(),
                labor: Labor::Process,
                enabled: true,
            })
        );
        cat.preferred_labors.push(Labor::Process);
        assert!(matches!(
            labor_preference_action(&session, &cat, Labor::Process),
            Some(ClientAction::SetCatLaborPreference { enabled: false, .. })
        ));
        assert!(labor_preference_action(&Session::default(), &cat, Labor::Process).is_none());
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
    fn camera_chunk_set_is_bounded_and_handles_negative_world_space() {
        let chunks = chunks_around(ChunkKey { x: -4, y: 7 }, TERRAIN_CHUNK_RADIUS);
        assert_eq!(chunks.len(), 25);
        assert!(chunks.contains(&ChunkKey { x: -6, y: 5 }));
        assert!(chunks.contains(&ChunkKey { x: -2, y: 9 }));
        assert!(!chunks.contains(&ChunkKey { x: -7, y: 7 }));

        let fog_halo = expanded_chunks(&chunks, 1);
        assert_eq!(fog_halo.len(), 49);
        assert!(fog_halo.contains(&ChunkKey { x: -7, y: 4 }));
    }

    #[test]
    fn streamed_terrain_generates_exact_requested_chunks() {
        let requested = HashSet::from([ChunkKey { x: -2, y: 3 }, ChunkKey { x: 11, y: -8 }]);
        let (tiles, _) = terrain_for_chunks(20_240_703, &requested);
        assert_eq!(
            tiles.len(),
            requested.len() * (TERRAIN_CHUNK_SIZE * TERRAIN_CHUNK_SIZE) as usize
        );
        assert!(
            tiles
                .iter()
                .all(|tile| requested.contains(&chunk_for_tile(tile.x, tile.y)))
        );
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
    fn reconnect_backoff_is_exponential_and_bounded() {
        assert_eq!(reconnect_delay_secs(1), 1.0);
        assert_eq!(reconnect_delay_secs(2), 2.0);
        assert_eq!(reconnect_delay_secs(3), 4.0);
        assert_eq!(reconnect_delay_secs(6), MAX_RECONNECT_DELAY_SECS);
        assert_eq!(reconnect_delay_secs(100), MAX_RECONNECT_DELAY_SECS);
    }

    #[test]
    fn action_result_parser_preserves_failure_and_signed_presence() {
        let failed = parse_server_message(r#"{"ok":false,"message":"Not enough food."}"#)
            .expect("valid action result");
        let ServerPayload::Action {
            result,
            signed_session,
        } = failed
        else {
            panic!("expected action result");
        };
        assert!(!result.ok);
        assert_eq!(result.message.as_deref(), Some("Not enough food."));
        assert!(signed_session.is_none());

        let presence = parse_server_message(
            r#"{"ok":true,"sessionId":"session-1","sig":"signed-token","playerId":"p1"}"#,
        )
        .expect("valid signed presence result");
        let ServerPayload::Action {
            result,
            signed_session,
        } = presence
        else {
            panic!("expected presence action result");
        };
        assert!(result.ok);
        assert_eq!(
            signed_session,
            Some(("session-1".to_string(), "signed-token".to_string()))
        );
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

    #[test]
    fn farm_stages_have_explicit_crop_art_and_crop_tints_are_distinct() {
        assert_eq!(farm_stage_prop(FarmStage::Soil), None);
        assert_eq!(
            farm_stage_prop(FarmStage::Sprout),
            Some(StationProp::CropSprout)
        );
        assert_eq!(
            farm_stage_prop(FarmStage::Growing),
            Some(StationProp::CropGrowing)
        );
        assert_eq!(
            farm_stage_prop(FarmStage::Mature),
            Some(StationProp::CropMature)
        );
        assert_eq!(
            farm_stage_prop(FarmStage::Flowering),
            Some(StationProp::CropFlowering)
        );
        assert_ne!(
            farm_crop_tint(CropKind::Catnip),
            farm_crop_tint(CropKind::Grain)
        );
        assert_ne!(
            farm_crop_tint(CropKind::Grain),
            farm_crop_tint(CropKind::Herb)
        );
    }

    fn amounts(food: f64, materials: f64, refined: f64) -> ResourceAmounts {
        ResourceAmounts {
            food,
            water: 0.0,
            herbs: 0.0,
            catnip: 0.0,
            grain: 0.0,
            flour: 0.0,
            materials,
            refined,
            weapons: 0.0,
            armor: 0.0,
            planks: 0.0,
            logs: 0.0,
            lumber: 0.0,
            blocks: 0.0,
            tools: 0.0,
            fibre: 0.0,
            hide: 0.0,
            cloth: 0.0,
            leather: 0.0,
            ore: 0.0,
            metal: 0.0,
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
    fn each_new_single_resource_pile_has_a_visible_total_and_dominant_prop() {
        type ResourceSetter = fn(&mut ResourceAmounts);
        let cases: [(ResourceKind, ResourceSetter); 6] = [
            (ResourceKind::Fibre, |a: &mut ResourceAmounts| a.fibre = 3.0),
            (ResourceKind::Hide, |a: &mut ResourceAmounts| a.hide = 3.0),
            (ResourceKind::Cloth, |a: &mut ResourceAmounts| a.cloth = 3.0),
            (ResourceKind::Leather, |a: &mut ResourceAmounts| {
                a.leather = 3.0
            }),
            (ResourceKind::Ore, |a: &mut ResourceAmounts| a.ore = 3.0),
            (ResourceKind::Metal, |a: &mut ResourceAmounts| a.metal = 3.0),
        ];
        for (kind, set) in cases {
            let mut amounts = amounts(0.0, 0.0, 0.0);
            set(&mut amounts);
            assert_eq!(resource_total(&amounts), 3.0, "{kind:?}");
            assert_eq!(dominant_resource(&amounts), Some(kind));
            assert!(pile_scale(resource_total(&amounts)) > pile_scale(0.0));
        }
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
    fn building_render_layout_fits_footprint_without_world_text_geometry() {
        let layout = building_render_layout(
            TilePoint { x: 6, y: 6 },
            FootprintSize {
                width: 3,
                height: 2,
            },
        );
        assert!(layout.facade_size.x <= 3.0 * TILE);
        assert!(layout.facade_size.y <= 3.0 * TILE);

        let tiny = building_render_layout(
            TilePoint { x: -2, y: 4 },
            FootprintSize {
                width: 0,
                height: 0,
            },
        );
        assert!(tiny.facade_size.x <= TILE);
    }

    #[test]
    fn open_station_prop_geometry_preserves_aspect_and_stays_in_footprint() {
        let placement = PropPlacement {
            prop: StationProp::Workbench,
            x: 1000,
            y: 0,
        };
        let geometry = station_prop_geometry(
            TilePoint { x: 6, y: 6 },
            FootprintSize {
                width: 3,
                height: 3,
            },
            placement,
        );
        assert_eq!(geometry.size, Vec2::new(34.0 / 16.0 * TILE, TILE));
        assert_eq!(geometry.size.x / geometry.size.y, 34.0 / 16.0);
        let left = 5.5 * TILE;
        let right = 8.5 * TILE;
        let top = -5.5 * TILE;
        let bottom = -8.5 * TILE;
        assert!(geometry.center.x - geometry.size.x / 2.0 >= left);
        assert!(geometry.center.x + geometry.size.x / 2.0 <= right);
        assert!(geometry.center.y + geometry.size.y / 2.0 <= top);
        assert!(geometry.center.y - geometry.size.y / 2.0 >= bottom);
    }

    #[test]
    fn building_renderer_composes_open_shrine_without_persistent_name_text() {
        let mut snapshot = village_world(&["alpha"]);
        snapshot.colonies[0].buildings.push(BuildingSnapshot {
            id: "shrine".to_owned(),
            building_type: BuildingType::Shrine,
            level: 1,
            construction_progress: 100.0,
            world_position: TilePoint { x: 6, y: 6 },
            position: TilePoint { x: 6, y: 6 },
            footprint: FootprintSize {
                width: 3,
                height: 3,
            },
            ..default()
        });

        let mut app = App::new();
        app.insert_resource(LatestSnapshot(Some(snapshot)))
            .insert_resource(BuildingArt::default())
            .add_systems(Update, render_buildings);
        app.update();

        let world = app.world_mut();
        let mut floors = world.query_filtered::<Entity, With<StationFloorSprite>>();
        assert_eq!(
            floors.iter(world).count(),
            9,
            "one repeated tile per footprint cell"
        );
        let mut props = world.query_filtered::<Entity, With<StationPropSprite>>();
        assert_eq!(
            props.iter(world).count(),
            4,
            "altar, relic, candles, and brazier"
        );
        let mut roofs = world.query_filtered::<Entity, With<RoofedBuildingSprite>>();
        assert_eq!(
            roofs.iter(world).count(),
            0,
            "shrines are open return destinations"
        );
        let mut labels = world.query_filtered::<Entity, (With<BuildingSprite>, With<Text2d>)>();
        assert_eq!(labels.iter(world).count(), 0);
    }

    #[test]
    fn mature_village_camera_fit_preserves_founding_zoom_and_uses_window_space() {
        assert_eq!(
            village_fit_zoom(STARTER_CAMERA_RADIUS, 1024.0, 768.0),
            DEFAULT_ZOOM
        );

        let compact = village_fit_zoom(10, 1024.0, 768.0);
        let wide = village_fit_zoom(10, 1920.0, 1080.0);
        assert!(compact > DEFAULT_ZOOM);
        assert!(wide > DEFAULT_ZOOM);
        assert!(compact > wide, "small windows must zoom farther out");
        assert!(compact <= MAX_ZOOM);

        let anchor = TilePoint { x: 6, y: 6 };
        let founding_center = village_camera_center(anchor, STARTER_CAMERA_RADIUS, DEFAULT_ZOOM);
        assert_eq!(founding_center, grid_to_world(7, 7));
        let mature_center = village_camera_center(anchor, 10, compact);
        let screen_shift = (founding_center.x - mature_center.x) / compact;
        assert!((screen_shift - CAMERA_SAFE_CENTER_OFFSET_X).abs() < 0.001);
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
        assert_eq!(life_stage(239.9), "adult");
        assert_eq!(life_stage(240.0), "elder");
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
        assert_eq!(tints.len(), 16);
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
            catnip: 3.0,
            grain: 14.0,
            flour: 6.0,
            materials: 24.0,
            refined: 0.0,
            weapons: 3.0,
            armor: 2.0,
            planks: 12.0,
            logs: 9.0,
            lumber: 4.0,
            blocks: 7.0,
            tools: 1.0,
            fibre: 11.0,
            hide: 12.0,
            cloth: 5.0,
            leather: 6.0,
            ore: 7.0,
            metal: 8.0,
            blessings: 4.5,
        };
        let cap = ResourceCapacities {
            food: 200.0,
            water: 200.0,
            herbs: 100.0,
            catnip: 50.0,
            grain: 100.0,
            flour: 100.0,
            materials: 100.0,
            refined: 100.0,
            weapons: 0.0,
            armor: 0.0,
            planks: 100.0,
            logs: 100.0,
            lumber: 100.0,
            blocks: 100.0,
            tools: 100.0,
            fibre: 100.0,
            hide: 100.0,
            cloth: 100.0,
            leather: 100.0,
            ore: 100.0,
            metal: 100.0,
        };
        assert_eq!(hud_resource_value(HudRes::Food, &r, &cap), "150 / 200");
        assert_eq!(hud_resource_value(HudRes::Grain, &r, &cap), "14 / 100");
        assert_eq!(hud_resource_value(HudRes::Flour, &r, &cap), "6 / 100");
        assert_eq!(hud_resource_value(HudRes::Logs, &r, &cap), "9 / 100");
        assert_eq!(hud_resource_value(HudRes::Lumber, &r, &cap), "4 / 100");
        // The refinement tier shows amount / cap like the other storables.
        assert_eq!(hud_resource_value(HudRes::Planks, &r, &cap), "12 / 100");
        assert_eq!(hud_resource_value(HudRes::Blocks, &r, &cap), "7 / 100");
        assert_eq!(hud_resource_value(HudRes::Tools, &r, &cap), "1 / 100");
        assert_eq!(hud_resource_value(HudRes::Weapons, &r, &cap), "3");
        assert_eq!(hud_resource_value(HudRes::Blessings, &r, &cap), "4.5");
    }

    #[test]
    fn stockpile_tooltip_names_general_and_legacy_stores_and_reports_contents() {
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
                catnip: 0.0,
                grain: 0.0,
                flour: 0.0,
                materials: 0.0,
                refined: 0.0,
                weapons: 0.0,
                armor: 0.0,
                planks: 0.0,
                logs: 0.0,
                lumber: 0.0,
                blocks: 0.0,
                tools: 0.0,
                fibre: 0.0,
                hide: 0.0,
                cloth: 0.0,
                leather: 0.0,
                ore: 0.0,
                metal: 0.0,
                blessings: 0.0,
            },
            gather_spot: None,
        };
        let tip = stockpile_tooltip(&pile);
        assert!(tip.contains("Stockpile"));
        assert!(tip.contains("food only"));
        assert!(tip.contains("~12"));

        let storehouse = StockpileSnapshot {
            id: GENERAL_STOREHOUSE_ID.to_string(),
            ..pile
        };
        assert!(stockpile_tooltip(&storehouse).contains("Village storehouse"));
        let legacy = StockpileSnapshot {
            id: SHRINE_STOCKPILE_ID.to_string(),
            ..storehouse
        };
        assert!(stockpile_tooltip(&legacy).contains("Legacy shrine store"));
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
    fn world_tooltip_is_suppressed_over_all_ui_not_only_buttons() {
        assert!(world_tooltip_allowed(false, false, true));
        assert!(!world_tooltip_allowed(true, false, true));
        assert!(!world_tooltip_allowed(false, true, true));
        assert!(!world_tooltip_allowed(false, false, false));
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
            catnip: 0.0,
            grain: 0.0,
            flour: 0.0,
            materials: 24.0,
            refined: 0.0,
            weapons: 0.0,
            armor: 0.0,
            planks: 0.0,
            logs: 0.0,
            lumber: 0.0,
            blocks: 0.0,
            tools: 0.0,
            fibre: 1.0,
            hide: 2.0,
            cloth: 3.0,
            leather: 4.0,
            ore: 5.0,
            metal: 6.0,
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

        let production = production_stores_text(&reported);
        for expected in [
            "fibre 1",
            "hide 2",
            "cloth 3",
            "leather 4",
            "ore 5",
            "metal 6",
        ] {
            assert!(
                production.contains(expected),
                "missing {expected}: {production}"
            );
        }

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
                    {"id":"b1","type":"sawmill","level":2,"constructionProgress":100.0,"worldPosition":{"x":7,"y":6},"position":{"x":1,"y":0},"footprint":{"width":3,"height":2},"staffCount":1,"staffCap":1,"productionProgress":0.4,"productionOutput":"lumber","inboundHaul":5.0,"outboundHaul":2.0,"inputInventory":[{"kind":"logs","amount":5.0}],"outputInventory":[{"kind":"lumber","amount":2.0}],"productionQueue":["logs_to_lumber"],"productionBlockReason":"output_in_transit","workerTravel":"hauling output to storage","inboundCargo":[{"kind":"logs","amount":5.0}],"outboundCargo":[{"kind":"lumber","amount":2.0}]},
                    {"id":"b2","type":"den","level":1,"constructionProgress":40.0,"worldPosition":{"x":5,"y":6},"position":{"x":-1,"y":0},"footprint":{"width":2,"height":2}}
                ],
                "claimedTiles":[],"villageGate":null,"villageRadius":4,"anchor":{"x":6,"y":6}
            }]
        }"#;
        let snap: WorldSnapshot = serde_json::from_str(json).expect("parse snapshot");
        let colony = &snap.colonies[0];
        let workshop = &colony.buildings[0];
        let den = &colony.buildings[1];
        let ws = building_inspector_text(workshop, colony);
        assert!(ws.contains("sawmill"));
        assert!(ws.contains("Lv 2"));
        assert!(ws.contains("operational"));
        assert!(ws.contains("Moss")); // assigned worker name
        assert!(ws.contains("staffed: 1/1")); // live staff count / cap
        assert!(ws.contains("making lumber")); // live production output
        assert!(ws.contains("[####------]")); // progress bar at 0.4
        assert!(ws.contains("40%"));
        assert!(ws.contains("footprint: 3x2 tiles"));
        assert!(ws.contains("inbound: 5.0"));
        assert!(ws.contains("local input: logs 5.0"));
        assert!(ws.contains("local output: lumber 2.0"));
        assert!(ws.contains("queue: logs_to_lumber"));
        assert!(ws.contains("blocked: output in transit"));
        assert!(ws.contains("worker: hauling output to storage"));
        assert!(ws.contains("outbound: 2.0"));
        assert!(matches!(
            station_queue_action(
                &signed_session("queue-session"),
                workshop,
                0,
                StationQueueButton::ToggleRepeat,
            ),
            Some(ClientAction::EditProductionQueue {
                session_id,
                building_id,
                edit: ProductionQueueEdit::SetRepeat {
                    index: 0,
                    repeat: false,
                },
                ..
            }) if session_id == "queue-session" && building_id == "b1"
        ));
        assert!(matches!(
            station_queue_action(
                &signed_session("queue-session"),
                workshop,
                0,
                StationQueueButton::TogglePause,
            ),
            Some(ClientAction::EditProductionQueue {
                edit: ProductionQueueEdit::SetPaused { paused: true },
                ..
            })
        ));
        // A den (staff_cap 0) under construction shows neither staffing nor output.
        let den_text = building_inspector_text(den, colony);
        assert!(den_text.contains("under construction 40%"));
        assert!(den_text.contains("[####------]"));
        assert!(den_text.contains("footprint: 2x2 tiles"));
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
        assert_eq!(need_bar_band(80.0), NeedBarBand::Comfortable);
        assert_eq!(need_bar_band(45.0), NeedBarBand::Low);
        assert_eq!(need_bar_band(10.0), NeedBarBand::Critical);

        let xp = RoleXp {
            hunter: 12.0,
            architect: 3.0,
            ritualist: 0.0,
            warrior: 1.0,
        };
        let line = cat_skills_line(&BTreeMap::new(), &xp);
        assert!(line.contains("hunt 12"));
        assert!(line.contains("build 3"));

        let practiced = BTreeMap::from([
            (Labor::Haul, 2.5),
            (Labor::Metalwork, 7.0),
            (Labor::Scout, 1.0),
        ]);
        let line = cat_skills_line(&practiced, &xp);
        assert!(line.contains("haul 2.5"));
        assert!(line.contains("metal 7.0"));
        assert!(line.contains("scout 1.0"));
        assert!(
            !line.contains("hunt 12"),
            "real labor map replaces legacy summary"
        );
    }

    #[test]
    fn cat_inspector_does_not_clip_the_world_render() {
        let node = cat_inspector_panel_node();
        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!(node.width, Val::Px(300.0));
        assert_eq!(node.overflow, Overflow::visible());
    }
}
