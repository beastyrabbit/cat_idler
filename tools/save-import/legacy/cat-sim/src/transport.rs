//! Persisted physical rail and shipping state for P17 distant-biome logistics.
//!
//! Research owns blueprints only. Every effect here requires constructed
//! infrastructure, a vehicle, a living crew cat, and explicit physical cargo.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{stockpiles::ResourceKind, world_tick::TilePos};

pub const RAIL_METAL_PER_TILE: f64 = 1.0;
/// Maximum endpoint-inclusive alignment accepted by one rail designation.
/// The line builder shares the action cap so raw coordinates cannot allocate an
/// attacker-controlled path before the caller validates it.
pub const MAX_CARDINAL_LINE_TILES: usize = 128;
pub const ROLLING_STOCK_METAL: f64 = 8.0;
pub const DOCK_LUMBER: f64 = 8.0;
pub const DOCK_BLOCKS: f64 = 4.0;
pub const VESSEL_LUMBER: f64 = 12.0;
pub const RAIL_TILES_PER_GAME_SECOND: f64 = 0.10;
pub const SHIP_TILES_PER_GAME_SECOND: f64 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportMode {
    Rail,
    Shipping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InfrastructureKind {
    Track,
    Dock,
    RollingStock,
    Vessel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectPhase {
    Fetching,
    Building,
    Complete,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CargoReservation {
    pub source_stockpile_id: String,
    pub kind: ResourceKind,
    pub amount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfrastructureProject {
    pub id: String,
    pub kind: InfrastructureKind,
    pub tiles: Vec<TilePos>,
    pub assigned_cat_id: String,
    pub phase: ProjectPhase,
    pub reservations: Vec<CargoReservation>,
    pub delivered: BTreeMap<ResourceKind, f64>,
    pub work_done_seconds: f64,
    pub required_work_seconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dock {
    pub id: String,
    pub land_tile: TilePos,
    pub water_tile: TilePos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutePhase {
    Boarding,
    Loading,
    Outbound,
    Unloading,
    Returning,
    WaitingForStorage,
    Complete,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportRoute {
    pub id: String,
    pub mode: TransportMode,
    pub source_stockpile_id: String,
    pub destination_stockpile_id: String,
    pub resource: ResourceKind,
    pub amount: f64,
    pub assigned_cat_id: String,
    pub phase: RoutePhase,
    pub path: Vec<TilePos>,
    pub path_index: usize,
    pub segment_progress: f64,
    pub cargo_loaded: f64,
    pub vehicle_id: String,
    pub position: TilePos,
    pub repeat: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vehicle {
    pub id: String,
    pub mode: TransportMode,
    pub home: TilePos,
    pub assigned_route_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TransportState {
    #[serde(default)]
    pub track_tiles: BTreeSet<TilePos>,
    #[serde(default)]
    pub docks: BTreeMap<String, Dock>,
    #[serde(default)]
    pub vehicles: BTreeMap<String, Vehicle>,
    #[serde(default)]
    pub projects: BTreeMap<String, InfrastructureProject>,
    #[serde(default)]
    pub routes: BTreeMap<String, TransportRoute>,
}

impl TransportState {
    #[must_use]
    pub fn track_connects(&self, path: &[TilePos]) -> bool {
        path.len() >= 2
            && path.iter().all(|tile| self.track_tiles.contains(tile))
            && path.windows(2).all(|pair| adjacent(pair[0], pair[1]))
    }

    #[must_use]
    pub fn idle_vehicle(&self, mode: TransportMode) -> Option<&Vehicle> {
        self.vehicles
            .values()
            .find(|vehicle| vehicle.mode == mode && vehicle.assigned_route_id.is_none())
    }
}

#[must_use]
pub const fn adjacent(a: TilePos, b: TilePos) -> bool {
    a.x.abs_diff(b.x) + a.y.abs_diff(b.y) == 1
}

#[must_use]
pub fn cardinal_line(a: TilePos, b: TilePos) -> Option<Vec<TilePos>> {
    if a.x != b.x && a.y != b.y {
        return None;
    }
    let dx = i64::from(b.x) - i64::from(a.x);
    let dy = i64::from(b.y) - i64::from(a.y);
    let distance = dx.abs().checked_add(dy.abs())?;
    let tile_count = usize::try_from(distance.checked_add(1)?).ok()?;
    if tile_count > MAX_CARDINAL_LINE_TILES {
        return None;
    }
    let step_x = i32::try_from(dx.signum()).ok()?;
    let step_y = i32::try_from(dy.signum()).ok()?;
    let mut cursor = a;
    let mut out = Vec::with_capacity(tile_count);
    out.push(a);
    for _ in 0..distance {
        cursor = TilePos {
            x: cursor.x.checked_add(step_x)?,
            y: cursor.y.checked_add(step_y)?,
        };
        out.push(cursor);
    }
    (cursor == b).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cardinal_lines_are_exact_and_diagonals_are_denied() {
        assert_eq!(
            cardinal_line(TilePos { x: 1, y: 2 }, TilePos { x: 4, y: 2 })
                .unwrap()
                .len(),
            4
        );
        assert_eq!(
            cardinal_line(TilePos { x: 0, y: 0 }, TilePos { x: 1, y: 1 }),
            None
        );
    }

    #[test]
    fn cardinal_lines_reject_unbounded_and_overflowing_coordinate_spans() {
        assert_eq!(
            cardinal_line(TilePos { x: 0, y: 0 }, TilePos { x: 0, y: i32::MAX },),
            None
        );
        assert_eq!(
            cardinal_line(TilePos { x: i32::MIN, y: 0 }, TilePos { x: i32::MAX, y: 0 },),
            None
        );
        assert_eq!(
            cardinal_line(
                TilePos { x: 0, y: 0 },
                TilePos {
                    x: i32::try_from(MAX_CARDINAL_LINE_TILES - 1).unwrap(),
                    y: 0,
                },
            )
            .unwrap()
            .len(),
            MAX_CARDINAL_LINE_TILES
        );
        assert_eq!(
            cardinal_line(
                TilePos { x: 0, y: 0 },
                TilePos {
                    x: i32::try_from(MAX_CARDINAL_LINE_TILES).unwrap(),
                    y: 0,
                },
            ),
            None
        );
    }

    #[test]
    fn a_track_route_requires_every_adjacent_constructed_tile() {
        let mut state = TransportState::default();
        state.track_tiles.extend([
            TilePos { x: 1, y: 1 },
            TilePos { x: 2, y: 1 },
            TilePos { x: 3, y: 1 },
        ]);
        assert!(state.track_connects(&[
            TilePos { x: 1, y: 1 },
            TilePos { x: 2, y: 1 },
            TilePos { x: 3, y: 1 }
        ]));
        assert!(!state.track_connects(&[TilePos { x: 1, y: 1 }, TilePos { x: 3, y: 1 }]));
    }
}
