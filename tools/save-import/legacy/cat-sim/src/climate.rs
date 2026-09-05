//! Climate-driven biome palette (P17).
//!
//! A rich, Minecraft-style **2D biome layer** that sits *alongside* the coarse
//! [`BiomeRole`](crate::terrain_gen::BiomeRole) the terrain generator already
//! emits. It is deliberately **additive and non-breaking**:
//!
//! - Movement ([`crate::movement`]) consumes each fine biome's `move_factor`
//!   through a per-tick, per-chunk derived cache. Placement / P14.1 keeps the
//!   unchanged coarse [`BiomeRole`] compatibility surface.
//! - This module adds a ~26-entry [`Biome`] palette + property table (tree
//!   density, ground tint, movement factor, resource + mining rule, crop
//!   fertility, passability) and a *base* classifier over
//!   temperature × humidity × elevation. The terrain generator samples two extra
//!   low-frequency climate-noise fields and stamps [`Biome`] on every tile.
//!
//! Every [`Biome`] maps back to a **surface** [`BiomeRole`]
//! ([`BiomeClimate::surface_role`]) for legacy placement and fallback behavior;
//! the founding plateau always resolves to a grass biome
//! ([`Biome::Plains`] → grassland).

use std::{fmt, str::FromStr};

use crate::terrain_gen::BiomeRole;

/// Error returned when a [`Biome`] wire literal is unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseClimateBiomeError {
    value: String,
}

impl ParseClimateBiomeError {
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for ParseClimateBiomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown Biome wire literal {:?}", self.value)
    }
}

impl std::error::Error for ParseClimateBiomeError {}

/// The climate-driven biome palette (~26 entries, "≈25").
///
/// `River` is an overlay set by the generator when a tile carries a river; the
/// pure [`classify_climate_biome`] returns it only when `has_river` is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Biome {
    // Water tiers (impassable)
    Ocean,
    Lake,
    River,
    Ice,
    // Coast
    Beach,
    StonyShore,
    // Temperate grass
    Plains,
    Meadow,
    FlowerField,
    // Forests
    OakForest,
    BirchForest,
    DarkForest,
    PineForest,
    Taiga,
    Jungle,
    // Warm / dry
    Savanna,
    Desert,
    Badlands,
    // Wet lowland
    Swamp,
    Marsh,
    // Cold
    Tundra,
    SnowyPlains,
    SnowyTaiga,
    // Highland
    Hills,
    Mountains,
    // Odd
    MushroomFields,
}

impl Biome {
    /// Every palette entry, in declaration order (matches the property table).
    pub const ALL: &'static [Self] = &[
        Self::Ocean,
        Self::Lake,
        Self::River,
        Self::Ice,
        Self::Beach,
        Self::StonyShore,
        Self::Plains,
        Self::Meadow,
        Self::FlowerField,
        Self::OakForest,
        Self::BirchForest,
        Self::DarkForest,
        Self::PineForest,
        Self::Taiga,
        Self::Jungle,
        Self::Savanna,
        Self::Desert,
        Self::Badlands,
        Self::Swamp,
        Self::Marsh,
        Self::Tundra,
        Self::SnowyPlains,
        Self::SnowyTaiga,
        Self::Hills,
        Self::Mountains,
        Self::MushroomFields,
    ];

    /// Snake-case wire literal, for the client / serialization.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.properties().wire
    }

    /// This biome's properties (density, tint, movement, resource, fertility …).
    #[must_use]
    pub const fn properties(self) -> &'static BiomeClimate {
        &BIOME_CLIMATE[self as usize]
    }

    /// The coarse surface [`BiomeRole`] this biome sits on. Lets a future
    /// movement/placement rewire keep the existing role semantics.
    #[must_use]
    pub const fn surface_role(self) -> BiomeRole {
        self.properties().surface_role
    }

    /// `(tree_density, rock_density)` used by the density-driven decoration
    /// sampler ([`crate::terrain_gen::derive_biome_decoration`]).
    #[must_use]
    pub const fn decoration_density(self) -> (f64, f64) {
        let props = self.properties();
        (props.tree_density, props.rock_density)
    }
}

impl FromStr for Biome {
    type Err = ParseClimateBiomeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|biome| biome.as_str() == value)
            .ok_or_else(|| ParseClimateBiomeError {
                value: value.to_owned(),
            })
    }
}

impl TryFrom<&str> for Biome {
    type Error = ParseClimateBiomeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// What a biome primarily offers to the scout/gather loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceHint {
    None,
    Wood,
    Stone,
    Ore,
    Fish,
    Farmland,
}

/// Mining rule for a biome (card P17): mines only on mountains; stony biomes
/// give a trickle of stone; everything else yields nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mining {
    None,
    Trickle,
    Full,
}

impl Mining {
    /// Fraction of a quarry job's base yield this mining rule actually produces:
    /// `Full` mountains keep the whole base yield, `Trickle` stony/gravel ground
    /// gives a fifth (a real but minor source), and `None` produces nothing —
    /// wired into `world_tick::total_yield_for_job`'s `Quarry` arm (P17 mining
    /// rules) so the leftover base constant (`QUARRY_TOTAL_YIELD`) stays the
    /// "full mountain" number and every other biome scales down from there.
    #[must_use]
    pub const fn yield_multiplier(self) -> f64 {
        match self {
            Self::Full => 1.0,
            Self::Trickle => 0.2,
            Self::None => 0.0,
        }
    }
}

/// Per-biome property row (the P17 analogue of [`crate::biomes::BiomeProperties`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiomeClimate {
    pub biome: Biome,
    /// Snake-case wire literal.
    pub wire: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Coarse surface role this biome maps onto (movement/placement compat).
    pub surface_role: BiomeRole,
    /// Fraction of flat tiles that carry a tree decoration (plains sparse,
    /// forest dense, desert ~none). Drives the density-based decoration sampler.
    pub tree_density: f64,
    /// Fraction of flat tiles that carry a rock decoration.
    pub rock_density: f64,
    /// Ground tint hint (RGB) for the client to tile/tint biomes.
    pub tint: [u8; 3],
    /// Movement-speed surface factor, aligned with [`crate::movement`] surface
    /// factors (sand slow, stone fast). `0.0` for impassable water/gated peaks.
    pub move_factor: f64,
    /// Primary resource available here.
    pub resource: ResourceHint,
    /// Mining rule (full ore only on mountains; trickle on stony biomes).
    pub mining: Mining,
    /// Crop-growth multiplier (grass ~0.8, marsh ~1.5, desert/tundra 0 = barren).
    pub fertility: f64,
    /// Whether cats can walk this tile today.
    pub passable: bool,
    /// Impassable now but walkable once a tech unlock lands (mountains).
    pub mountain_gated: bool,
}

impl BiomeClimate {
    /// Fields can be sown here (any positive fertility).
    #[must_use]
    pub const fn farmable(&self) -> bool {
        self.fertility > 0.0
    }
}

// Movement surface-factor anchors mirrored from `crate::movement` so this table
// reads on the same scale (grassland/lowland 0.75, forest 0.6, sand 0.5, …).
const F_SAND: f64 = 0.5;
const F_GRASS: f64 = 0.75;
const F_FOREST: f64 = 0.6;
const F_HIGHLAND: f64 = 0.7;

/// The ~26-entry property table, indexed by `Biome as usize`.
pub const BIOME_CLIMATE: [BiomeClimate; 26] = [
    BiomeClimate {
        biome: Biome::Ocean,
        wire: "ocean",
        name: "Ocean",
        surface_role: BiomeRole::Lowland,
        tree_density: 0.0,
        rock_density: 0.0,
        tint: [38, 84, 138],
        move_factor: 0.0,
        resource: ResourceHint::Fish,
        mining: Mining::None,
        fertility: 0.0,
        passable: false,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Lake,
        wire: "lake",
        name: "Lake",
        surface_role: BiomeRole::Lowland,
        tree_density: 0.0,
        rock_density: 0.0,
        tint: [58, 110, 165],
        move_factor: 0.0,
        resource: ResourceHint::Fish,
        mining: Mining::None,
        fertility: 0.0,
        passable: false,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::River,
        wire: "river",
        name: "River",
        surface_role: BiomeRole::Lowland,
        tree_density: 0.0,
        rock_density: 0.0,
        tint: [70, 128, 180],
        move_factor: 0.0,
        resource: ResourceHint::Fish,
        mining: Mining::None,
        fertility: 0.0,
        passable: false,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Ice,
        wire: "ice",
        name: "Ice",
        surface_role: BiomeRole::Lowland,
        tree_density: 0.0,
        rock_density: 0.0,
        tint: [200, 224, 236],
        move_factor: 0.4,
        resource: ResourceHint::None,
        mining: Mining::None,
        fertility: 0.0,
        passable: false,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Beach,
        wire: "beach",
        name: "Beach",
        surface_role: BiomeRole::Lowland,
        tree_density: 0.02,
        rock_density: 0.03,
        tint: [222, 208, 150],
        move_factor: F_SAND,
        resource: ResourceHint::Fish,
        mining: Mining::None,
        fertility: 0.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::StonyShore,
        wire: "stony_shore",
        name: "Stony Shore",
        surface_role: BiomeRole::Rocky,
        tree_density: 0.0,
        rock_density: 0.25,
        tint: [140, 142, 138],
        move_factor: 0.9,
        resource: ResourceHint::Stone,
        mining: Mining::Trickle,
        fertility: 0.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Plains,
        wire: "plains",
        name: "Plains",
        surface_role: BiomeRole::Grassland,
        tree_density: 0.04,
        rock_density: 0.02,
        tint: [124, 176, 84],
        move_factor: F_GRASS,
        resource: ResourceHint::Farmland,
        mining: Mining::None,
        fertility: 0.8,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Meadow,
        wire: "meadow",
        name: "Meadow",
        surface_role: BiomeRole::Grassland,
        tree_density: 0.05,
        rock_density: 0.02,
        tint: [140, 190, 96],
        move_factor: 0.8,
        resource: ResourceHint::Farmland,
        mining: Mining::None,
        fertility: 1.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::FlowerField,
        wire: "flower_field",
        name: "Flower Field",
        surface_role: BiomeRole::Grassland,
        tree_density: 0.03,
        rock_density: 0.01,
        tint: [176, 196, 108],
        move_factor: 0.78,
        resource: ResourceHint::Farmland,
        mining: Mining::None,
        fertility: 1.1,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::OakForest,
        wire: "oak_forest",
        name: "Oak Forest",
        surface_role: BiomeRole::Forest,
        tree_density: 0.55,
        rock_density: 0.04,
        tint: [72, 128, 64],
        move_factor: F_FOREST,
        resource: ResourceHint::Wood,
        mining: Mining::None,
        fertility: 0.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::BirchForest,
        wire: "birch_forest",
        name: "Birch Forest",
        surface_role: BiomeRole::Forest,
        tree_density: 0.5,
        rock_density: 0.03,
        tint: [110, 150, 92],
        move_factor: 0.62,
        resource: ResourceHint::Wood,
        mining: Mining::None,
        fertility: 0.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::DarkForest,
        wire: "dark_forest",
        name: "Dark Forest",
        surface_role: BiomeRole::Forest,
        tree_density: 0.68,
        rock_density: 0.03,
        tint: [46, 86, 52],
        move_factor: 0.55,
        resource: ResourceHint::Wood,
        mining: Mining::None,
        fertility: 0.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::PineForest,
        wire: "pine_forest",
        name: "Pine Forest",
        surface_role: BiomeRole::Forest,
        tree_density: 0.58,
        rock_density: 0.05,
        tint: [56, 104, 76],
        move_factor: F_FOREST,
        resource: ResourceHint::Wood,
        mining: Mining::None,
        fertility: 0.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Taiga,
        wire: "taiga",
        name: "Taiga",
        surface_role: BiomeRole::Forest,
        tree_density: 0.52,
        rock_density: 0.05,
        tint: [86, 122, 96],
        move_factor: F_FOREST,
        resource: ResourceHint::Wood,
        mining: Mining::None,
        fertility: 0.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Jungle,
        wire: "jungle",
        name: "Jungle",
        surface_role: BiomeRole::Forest,
        tree_density: 0.7,
        rock_density: 0.03,
        tint: [46, 120, 48],
        move_factor: 0.45,
        resource: ResourceHint::Wood,
        mining: Mining::None,
        fertility: 0.4,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Savanna,
        wire: "savanna",
        name: "Savanna",
        surface_role: BiomeRole::Grassland,
        tree_density: 0.1,
        rock_density: 0.03,
        tint: [180, 176, 96],
        move_factor: 0.8,
        resource: ResourceHint::Farmland,
        mining: Mining::None,
        fertility: 0.5,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Desert,
        wire: "desert",
        name: "Desert",
        surface_role: BiomeRole::Rocky,
        tree_density: 0.01,
        rock_density: 0.03,
        tint: [224, 206, 140],
        move_factor: F_SAND,
        resource: ResourceHint::None,
        mining: Mining::None,
        fertility: 0.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Badlands,
        wire: "badlands",
        name: "Badlands",
        surface_role: BiomeRole::Rocky,
        tree_density: 0.0,
        rock_density: 0.3,
        tint: [178, 118, 74],
        move_factor: 0.55,
        resource: ResourceHint::Stone,
        mining: Mining::Trickle,
        fertility: 0.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Swamp,
        wire: "swamp",
        name: "Swamp",
        surface_role: BiomeRole::Lowland,
        tree_density: 0.3,
        rock_density: 0.02,
        tint: [86, 108, 78],
        move_factor: 0.45,
        resource: ResourceHint::Farmland,
        mining: Mining::None,
        fertility: 1.2,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Marsh,
        wire: "marsh",
        name: "Marsh",
        surface_role: BiomeRole::Lowland,
        tree_density: 0.12,
        rock_density: 0.02,
        tint: [104, 130, 92],
        move_factor: F_SAND,
        resource: ResourceHint::Farmland,
        mining: Mining::None,
        fertility: 1.5,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Tundra,
        wire: "tundra",
        name: "Tundra",
        surface_role: BiomeRole::Grassland,
        tree_density: 0.02,
        rock_density: 0.06,
        tint: [176, 190, 178],
        move_factor: F_HIGHLAND,
        resource: ResourceHint::None,
        mining: Mining::None,
        fertility: 0.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::SnowyPlains,
        wire: "snowy_plains",
        name: "Snowy Plains",
        surface_role: BiomeRole::Grassland,
        tree_density: 0.03,
        rock_density: 0.03,
        tint: [222, 230, 236],
        move_factor: 0.65,
        resource: ResourceHint::None,
        mining: Mining::None,
        fertility: 0.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::SnowyTaiga,
        wire: "snowy_taiga",
        name: "Snowy Taiga",
        surface_role: BiomeRole::Forest,
        tree_density: 0.45,
        rock_density: 0.05,
        tint: [150, 174, 168],
        move_factor: 0.55,
        resource: ResourceHint::Wood,
        mining: Mining::None,
        fertility: 0.0,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Hills,
        wire: "hills",
        name: "Hills",
        surface_role: BiomeRole::Highland,
        tree_density: 0.1,
        rock_density: 0.2,
        tint: [122, 150, 98],
        move_factor: F_HIGHLAND,
        resource: ResourceHint::Stone,
        mining: Mining::Trickle,
        fertility: 0.2,
        passable: true,
        mountain_gated: false,
    },
    BiomeClimate {
        biome: Biome::Mountains,
        wire: "mountains",
        name: "Mountains",
        surface_role: BiomeRole::Highland,
        tree_density: 0.03,
        rock_density: 0.45,
        tint: [130, 128, 126],
        move_factor: 0.35,
        resource: ResourceHint::Ore,
        mining: Mining::Full,
        fertility: 0.0,
        passable: false,
        mountain_gated: true,
    },
    BiomeClimate {
        biome: Biome::MushroomFields,
        wire: "mushroom_fields",
        name: "Mushroom Fields",
        surface_role: BiomeRole::Forest,
        tree_density: 0.25,
        rock_density: 0.05,
        tint: [150, 110, 150],
        move_factor: 0.7,
        resource: ResourceHint::Wood,
        mining: Mining::None,
        fertility: 0.6,
        passable: true,
        mountain_gated: false,
    },
];

/// Convenience accessor for a biome's property row.
#[must_use]
pub fn biome_climate(biome: Biome) -> &'static BiomeClimate {
    biome.properties()
}

// ---- Climate → biome lookup --------------------------------------------------
//
// Elevation bands (raw fractal elevation in [0, 1)). These key off the *same*
// continuous elevation the terrain generator already samples, so water sits at
// the low end and peaks at the high end.
const OCEAN_LEVEL: f64 = 0.24;
const LAKE_LEVEL: f64 = 0.30;
const BEACH_LEVEL: f64 = 0.34;
const LOWLAND_CEIL: f64 = 0.46; // wet lowlands below this can become marsh/swamp
const HILL_LEVEL: f64 = 0.72;
const MOUNTAIN_LEVEL: f64 = 0.86;

// Special-biome weirdness gates (a third low-frequency field carves large but
// uncommon regions of flower fields / mushroom fields).
const FLOWER_WEIRDNESS: f64 = 0.86;
const MUSHROOM_WEIRDNESS: f64 = 0.1;

/// Temperature band index `0..=4` (cold → hot).
const fn temp_band(temperature: f64) -> usize {
    if temperature < 0.30 {
        0
    } else if temperature < 0.50 {
        1
    } else if temperature < 0.70 {
        2
    } else if temperature < 0.85 {
        3
    } else {
        4
    }
}

/// Humidity band index `0..=4` (arid → wet).
const fn humidity_band(humidity: f64) -> usize {
    if humidity < 0.25 {
        0
    } else if humidity < 0.45 {
        1
    } else if humidity < 0.65 {
        2
    } else if humidity < 0.80 {
        3
    } else {
        4
    }
}

/// The temperate-land matrix: `[temperature_band][humidity_band]`.
const CLIMATE_MATRIX: [[Biome; 5]; 5] = [
    // cold
    [
        Biome::Tundra,
        Biome::SnowyPlains,
        Biome::SnowyPlains,
        Biome::SnowyTaiga,
        Biome::SnowyTaiga,
    ],
    // cool
    [
        Biome::Tundra,
        Biome::Plains,
        Biome::Meadow,
        Biome::PineForest,
        Biome::Taiga,
    ],
    // temperate
    [
        Biome::Savanna,
        Biome::Plains,
        Biome::Meadow,
        Biome::BirchForest,
        Biome::DarkForest,
    ],
    // warm
    [
        Biome::Desert,
        Biome::Savanna,
        Biome::Plains,
        Biome::OakForest,
        Biome::Swamp,
    ],
    // hot
    [
        Biome::Desert,
        Biome::Badlands,
        Biome::Savanna,
        Biome::Jungle,
        Biome::Jungle,
    ],
];

/// Deterministically pick a [`Biome`] from climate fields.
///
/// Inputs are the raw noise fields the generator samples per tile:
/// `temperature`, `humidity`, `weirdness` and `elevation` in `[0, 1)`, plus
/// whether the tile is inside the founding plateau and whether it carries a
/// river overlay. Pure and total — same inputs → same biome.
///
/// The founding plateau always resolves to [`Biome::Plains`] so the starting
/// area stays habitable grass. Water/coast come from elevation bands; mid
/// elevations use the temperature × humidity [`CLIMATE_MATRIX`]; a low-frequency
/// weirdness field carves rare flower/mushroom regions.
#[must_use]
pub fn classify_climate_biome(
    temperature: f64,
    humidity: f64,
    weirdness: f64,
    elevation: f64,
    in_plateau: bool,
    has_river: bool,
) -> Biome {
    if has_river {
        return Biome::River;
    }
    if in_plateau {
        return Biome::Plains;
    }

    let cold = temperature < 0.30;

    // Water & coast tiers by elevation.
    if elevation < OCEAN_LEVEL {
        return if cold { Biome::Ice } else { Biome::Ocean };
    }
    if elevation < LAKE_LEVEL {
        return if cold { Biome::Ice } else { Biome::Lake };
    }
    if elevation < BEACH_LEVEL {
        return if cold {
            Biome::StonyShore
        } else {
            Biome::Beach
        };
    }

    // Peaks & hills.
    if elevation >= MOUNTAIN_LEVEL {
        return Biome::Mountains;
    }
    if elevation >= HILL_LEVEL {
        return Biome::Hills;
    }

    // Low-lying, very wet land becomes marsh (temperate) or swamp (warm).
    if elevation < LOWLAND_CEIL && humidity >= 0.80 && !cold {
        return if temperature >= 0.70 {
            Biome::Swamp
        } else {
            Biome::Marsh
        };
    }

    let base = CLIMATE_MATRIX[temp_band(temperature)][humidity_band(humidity)];

    // Rare, large special regions carved by the low-frequency weirdness field.
    if weirdness >= FLOWER_WEIRDNESS && matches!(base, Biome::Plains | Biome::Meadow) {
        return Biome::FlowerField;
    }
    if weirdness <= MUSHROOM_WEIRDNESS
        && matches!(
            base,
            Biome::OakForest | Biome::BirchForest | Biome::DarkForest | Biome::Swamp | Biome::Marsh
        )
    {
        return Biome::MushroomFields;
    }

    base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_has_a_matching_property_row() {
        assert_eq!(Biome::ALL.len(), BIOME_CLIMATE.len());
        for biome in Biome::ALL {
            let props = biome.properties();
            assert_eq!(props.biome, *biome, "row for {biome:?} is out of order");
            // Round-trips through its wire literal.
            assert_eq!(props.wire.parse::<Biome>().unwrap(), *biome);
            assert_eq!(biome.as_str(), props.wire);
        }
    }

    #[test]
    fn wire_literals_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for biome in Biome::ALL {
            assert!(seen.insert(biome.as_str()), "duplicate wire {biome:?}");
        }
    }

    #[test]
    fn plateau_and_river_short_circuit() {
        // Founding plateau is always habitable grass, whatever the climate.
        for &(t, h, w, e) in &[(0.05, 0.05, 0.5, 0.9), (0.95, 0.95, 0.5, 0.1)] {
            assert_eq!(
                classify_climate_biome(t, h, w, e, true, false),
                Biome::Plains
            );
        }
        // A river overlay wins over everything (but not... it wins over plateau too here).
        assert_eq!(
            classify_climate_biome(0.5, 0.5, 0.5, 0.5, false, true),
            Biome::River
        );
    }

    #[test]
    fn founding_plateau_is_a_grass_biome() {
        let biome = classify_climate_biome(0.5, 0.5, 0.5, 0.5, true, false);
        assert!(matches!(
            biome.surface_role(),
            BiomeRole::Grassland | BiomeRole::Lowland
        ));
        assert!(biome.properties().farmable());
    }

    #[test]
    fn elevation_bands_produce_water_coast_and_peaks() {
        // Warm low elevation -> open water; cold low -> ice.
        assert_eq!(
            classify_climate_biome(0.6, 0.5, 0.5, 0.10, false, false),
            Biome::Ocean
        );
        assert_eq!(
            classify_climate_biome(0.1, 0.5, 0.5, 0.10, false, false),
            Biome::Ice
        );
        assert_eq!(
            classify_climate_biome(0.6, 0.5, 0.5, 0.32, false, false),
            Biome::Beach
        );
        assert_eq!(
            classify_climate_biome(0.1, 0.5, 0.5, 0.32, false, false),
            Biome::StonyShore
        );
        assert_eq!(
            classify_climate_biome(0.6, 0.5, 0.5, 0.90, false, false),
            Biome::Mountains
        );
        assert_eq!(
            classify_climate_biome(0.6, 0.5, 0.5, 0.75, false, false),
            Biome::Hills
        );
    }

    #[test]
    fn matrix_cells_map_to_expected_biomes() {
        // hot + arid -> desert; hot + humid -> jungle.
        assert_eq!(
            classify_climate_biome(0.95, 0.1, 0.5, 0.55, false, false),
            Biome::Desert
        );
        assert_eq!(
            classify_climate_biome(0.95, 0.9, 0.5, 0.60, false, false),
            Biome::Jungle
        );
        // cold + arid -> tundra.
        assert_eq!(
            classify_climate_biome(0.1, 0.1, 0.5, 0.55, false, false),
            Biome::Tundra
        );
        // temperate + humid -> a forest.
        assert_eq!(
            classify_climate_biome(0.6, 0.7, 0.5, 0.55, false, false).surface_role(),
            BiomeRole::Forest
        );
    }

    #[test]
    fn wet_lowlands_become_marsh_or_swamp() {
        // Temperate wet lowland -> marsh (fertile).
        let marsh = classify_climate_biome(0.5, 0.9, 0.5, 0.40, false, false);
        assert_eq!(marsh, Biome::Marsh);
        assert!(marsh.properties().fertility > 1.0);
        // Warm wet lowland -> swamp.
        assert_eq!(
            classify_climate_biome(0.8, 0.9, 0.5, 0.40, false, false),
            Biome::Swamp
        );
    }

    #[test]
    fn weirdness_carves_flower_and_mushroom_regions() {
        // A plains/meadow cell + high weirdness -> flower field.
        assert_eq!(
            classify_climate_biome(0.4, 0.35, 0.95, 0.55, false, false),
            Biome::FlowerField
        );
        // A forest cell + low weirdness -> mushroom fields.
        assert_eq!(
            classify_climate_biome(0.6, 0.7, 0.02, 0.55, false, false),
            Biome::MushroomFields
        );
    }

    #[test]
    fn mining_full_is_mountains_only() {
        for biome in Biome::ALL {
            let mining = biome.properties().mining;
            if mining == Mining::Full {
                assert_eq!(*biome, Biome::Mountains, "only mountains mine fully");
            }
        }
        assert_eq!(Biome::Mountains.properties().mining, Mining::Full);
        assert!(Biome::StonyShore.properties().mining == Mining::Trickle);
    }

    #[test]
    fn water_and_gated_peaks_are_impassable() {
        for biome in [Biome::Ocean, Biome::Lake, Biome::River, Biome::Ice] {
            assert!(!biome.properties().passable, "{biome:?} should block");
        }
        let mountains = Biome::Mountains.properties();
        assert!(!mountains.passable);
        assert!(mountains.mountain_gated);
    }

    #[test]
    fn forest_biomes_are_denser_than_grass_biomes() {
        let forest = Biome::OakForest.decoration_density().0;
        let plains = Biome::Plains.decoration_density().0;
        assert!(
            forest > plains * 5.0,
            "forest {forest} should dwarf plains {plains}"
        );
        assert_eq!(Biome::Desert.decoration_density().0.max(0.02), 0.02);
    }

    #[test]
    fn classification_is_deterministic() {
        for i in 0..64 {
            let t = f64::from(i) / 64.0;
            let a = classify_climate_biome(t, 1.0 - t, t, t, false, false);
            let b = classify_climate_biome(t, 1.0 - t, t, t, false, false);
            assert_eq!(a, b);
        }
    }

    #[test]
    fn mining_yield_multiplier_ranks_full_above_trickle_above_none() {
        assert_eq!(Mining::Full.yield_multiplier(), 1.0);
        assert!(Mining::Trickle.yield_multiplier() > 0.0);
        assert!(Mining::Trickle.yield_multiplier() < Mining::Full.yield_multiplier());
        assert_eq!(Mining::None.yield_multiplier(), 0.0);
    }

    #[test]
    fn every_biome_mining_multiplier_matches_its_rule() {
        for biome in Biome::ALL {
            let props = biome.properties();
            assert_eq!(
                props.mining.yield_multiplier(),
                props.mining.yield_multiplier()
            );
            match props.mining {
                Mining::Full => assert_eq!(*biome, Biome::Mountains),
                Mining::None => assert_eq!(props.mining.yield_multiplier(), 0.0),
                Mining::Trickle => assert!(props.mining.yield_multiplier() > 0.0),
            }
        }
    }
}
