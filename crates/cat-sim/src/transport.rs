//! Persisted physical rail and shipping state for P17 distant-biome logistics.
//!
//! Research owns blueprints only. Every effect here requires constructed
//! infrastructure, a vehicle, a living crew cat, and explicit physical cargo.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{stockpiles::ResourceKind, world_tick::TilePos};

pub const RAIL_METAL_PER_TILE: f64 = 1.0;
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
    let dx = (b.x - a.x).signum();
    let dy = (b.y - a.y).signum();
    let mut cursor = a;
    let mut out = vec![a];
    while cursor != b {
        cursor = TilePos {
            x: cursor.x + dx,
            y: cursor.y + dy,
        };
        out.push(cursor);
    }
    Some(out)
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
