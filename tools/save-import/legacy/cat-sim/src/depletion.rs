//! World-resource depletion and regrowth rules ported from `lib/game/depletion.ts`.

use crate::types::TileType;

/// Tile types that count as forest: choppable for lumber and excluded from
/// food regrowth.
pub const FOREST_TYPES: [TileType; 6] = [
    TileType::Forest,
    TileType::OakForest,
    TileType::PineForest,
    TileType::DenseWoods,
    TileType::Jungle,
    TileType::DeadForest,
];

/// The food cap stamped onto a field tile after chopping a forest.
pub const CHOPPED_FOREST_FOOD_CAP: f64 = 5.0;

/// Minimal tile view used by [`is_chopped_stump_tile`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepletionTile {
    pub tile_type: TileType,
    pub max_food: f64,
    pub last_depleted: f64,
}

impl From<&DepletionTile> for DepletionTile {
    fn from(tile: &DepletionTile) -> Self {
        *tile
    }
}

/// Whether a tile type is forest: choppable and not eligible for food regrowth.
#[must_use]
pub fn is_forest_type(tile_type: TileType) -> bool {
    FOREST_TYPES.contains(&tile_type)
}

/// Food regrown over `elapsed_sec` at +1 food per game hour.
#[must_use]
pub fn regrowth_amount(elapsed_sec: f64) -> f64 {
    if elapsed_sec <= 0.0 {
        return 0.0;
    }

    elapsed_sec / 3600.0
}

/// Whether a tile is the render-only signature of a chopped forest stump.
#[must_use]
pub fn is_chopped_stump_tile(tile: impl Into<DepletionTile>) -> bool {
    let tile = tile.into();

    tile.tile_type == TileType::Field
        && tile.last_depleted > 0.0
        && tile.max_food <= CHOPPED_FOREST_FOOD_CAP
}

#[cfg(test)]
mod tests {
    use super::{
        CHOPPED_FOREST_FOOD_CAP, DepletionTile, FOREST_TYPES, is_chopped_stump_tile,
        is_forest_type, regrowth_amount,
    };
    use crate::types::TileType;

    #[test]
    fn forest_type_set_matches_depletion_ts() {
        assert_eq!(
            FOREST_TYPES,
            [
                TileType::Forest,
                TileType::OakForest,
                TileType::PineForest,
                TileType::DenseWoods,
                TileType::Jungle,
                TileType::DeadForest,
            ]
        );

        for tile_type in FOREST_TYPES {
            assert!(is_forest_type(tile_type));
        }

        for tile_type in [
            TileType::Field,
            TileType::Meadow,
            TileType::River,
            TileType::EnemyTerritory,
        ] {
            assert!(!is_forest_type(tile_type));
        }
    }

    #[test]
    fn regrowth_amount_is_one_food_per_hour_and_never_negative() {
        assert_eq!(regrowth_amount(3600.0), 1.0);
        assert_eq!(regrowth_amount(1800.0), 0.5);
        assert_eq!(regrowth_amount(7200.0), 2.0);
        assert_eq!(regrowth_amount(120.0), 120.0 / 3600.0);
        assert_eq!(regrowth_amount(0.0), 0.0);
        assert_eq!(regrowth_amount(-100.0), 0.0);
    }

    #[test]
    fn chopped_stump_detection_uses_field_low_cap_and_depletion_stamp() {
        let stump = DepletionTile {
            tile_type: TileType::Field,
            max_food: CHOPPED_FOREST_FOOD_CAP,
            last_depleted: 1000.0,
        };

        assert!(is_chopped_stump_tile(stump));
        assert!(!is_chopped_stump_tile(DepletionTile {
            max_food: 40.0,
            ..stump
        }));
        assert!(!is_chopped_stump_tile(DepletionTile {
            last_depleted: 0.0,
            ..stump
        }));
        assert!(!is_chopped_stump_tile(DepletionTile {
            tile_type: TileType::Forest,
            max_food: 0.0,
            ..stump
        }));
        assert!(!is_chopped_stump_tile(DepletionTile {
            tile_type: TileType::River,
            max_food: 0.0,
            ..stump
        }));
    }
}
