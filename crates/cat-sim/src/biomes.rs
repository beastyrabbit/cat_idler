//! Biome tables and calculators ported from `lib/game/biomes.ts`.

use std::{fmt, str::FromStr};

/// Error returned when a biome module wire literal is unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseBiomeError {
    enum_name: &'static str,
    value: String,
}

impl ParseBiomeError {
    #[must_use]
    pub fn enum_name(&self) -> &'static str {
        self.enum_name
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for ParseBiomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown {} wire literal {:?}",
            self.enum_name, self.value
        )
    }
}

impl std::error::Error for ParseBiomeError {}

/// TS `BiomeType` string-literal union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiomeType {
    OakForest,
    PineForest,
    Jungle,
    DeadForest,
    Mountains,
    Swamp,
    Desert,
    Tundra,
    Meadow,
    CaveEntrance,
    EnemyLair,
}

impl BiomeType {
    pub const ALL: &'static [Self] = &[
        Self::OakForest,
        Self::PineForest,
        Self::Jungle,
        Self::DeadForest,
        Self::Mountains,
        Self::Swamp,
        Self::Desert,
        Self::Tundra,
        Self::Meadow,
        Self::CaveEntrance,
        Self::EnemyLair,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OakForest => "oak_forest",
            Self::PineForest => "pine_forest",
            Self::Jungle => "jungle",
            Self::DeadForest => "dead_forest",
            Self::Mountains => "mountains",
            Self::Swamp => "swamp",
            Self::Desert => "desert",
            Self::Tundra => "tundra",
            Self::Meadow => "meadow",
            Self::CaveEntrance => "cave_entrance",
            Self::EnemyLair => "enemy_lair",
        }
    }
}

impl FromStr for BiomeType {
    type Err = ParseBiomeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "oak_forest" => Ok(Self::OakForest),
            "pine_forest" => Ok(Self::PineForest),
            "jungle" => Ok(Self::Jungle),
            "dead_forest" => Ok(Self::DeadForest),
            "mountains" => Ok(Self::Mountains),
            "swamp" => Ok(Self::Swamp),
            "desert" => Ok(Self::Desert),
            "tundra" => Ok(Self::Tundra),
            "meadow" => Ok(Self::Meadow),
            "cave_entrance" => Ok(Self::CaveEntrance),
            "enemy_lair" => Ok(Self::EnemyLair),
            _ => Err(ParseBiomeError {
                enum_name: "BiomeType",
                value: value.to_owned(),
            }),
        }
    }
}

impl TryFrom<&str> for BiomeType {
    type Error = ParseBiomeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// TS `Exclude<OverlayFeature, null>` string-literal union.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverlayFeature {
    River,
    AncientRoad,
    GameTrail,
    TradeRoute,
}

impl OverlayFeature {
    pub const ALL: &'static [Self] = &[
        Self::River,
        Self::AncientRoad,
        Self::GameTrail,
        Self::TradeRoute,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::River => "river",
            Self::AncientRoad => "ancient_road",
            Self::GameTrail => "game_trail",
            Self::TradeRoute => "trade_route",
        }
    }
}

impl FromStr for OverlayFeature {
    type Err = ParseBiomeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "river" => Ok(Self::River),
            "ancient_road" => Ok(Self::AncientRoad),
            "game_trail" => Ok(Self::GameTrail),
            "trade_route" => Ok(Self::TradeRoute),
            _ => Err(ParseBiomeError {
                enum_name: "OverlayFeature",
                value: value.to_owned(),
            }),
        }
    }
}

impl TryFrom<&str> for OverlayFeature {
    type Error = ParseBiomeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceRange {
    pub min: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseResources {
    pub food: ResourceRange,
    pub herbs: ResourceRange,
    pub water: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxResources {
    pub food: u32,
    pub herbs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiomeProperties {
    pub biome_type: BiomeType,
    pub base_danger: f64,
    pub base_resources: BaseResources,
    pub max_resources: MaxResources,
    pub travel_speed: f64,
    pub name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayFeatureProperties {
    pub danger_modifier: f64,
    pub speed_modifier: f64,
    pub initial_path_wear: u32,
    pub name: &'static str,
}

pub const BIOME_PROPERTIES: [BiomeProperties; 11] = [
    BiomeProperties {
        biome_type: BiomeType::OakForest,
        base_danger: 20.0,
        base_resources: BaseResources {
            food: ResourceRange { min: 15, max: 40 },
            herbs: ResourceRange { min: 0, max: 5 },
            water: 0,
        },
        max_resources: MaxResources { food: 50, herbs: 8 },
        travel_speed: 1.0,
        name: "Oak Forest",
    },
    BiomeProperties {
        biome_type: BiomeType::PineForest,
        base_danger: 30.0,
        base_resources: BaseResources {
            food: ResourceRange { min: 10, max: 30 },
            herbs: ResourceRange { min: 2, max: 10 },
            water: 0,
        },
        max_resources: MaxResources {
            food: 40,
            herbs: 15,
        },
        travel_speed: 0.9,
        name: "Pine Forest",
    },
    BiomeProperties {
        biome_type: BiomeType::Jungle,
        base_danger: 45.0,
        base_resources: BaseResources {
            food: ResourceRange { min: 30, max: 60 },
            herbs: ResourceRange { min: 10, max: 25 },
            water: 0,
        },
        max_resources: MaxResources {
            food: 80,
            herbs: 35,
        },
        travel_speed: 0.6,
        name: "Jungle",
    },
    BiomeProperties {
        biome_type: BiomeType::DeadForest,
        base_danger: 55.0,
        base_resources: BaseResources {
            food: ResourceRange { min: 0, max: 15 },
            herbs: ResourceRange { min: 15, max: 35 },
            water: 0,
        },
        max_resources: MaxResources {
            food: 20,
            herbs: 45,
        },
        travel_speed: 0.8,
        name: "Dead Forest",
    },
    BiomeProperties {
        biome_type: BiomeType::Mountains,
        base_danger: 50.0,
        base_resources: BaseResources {
            food: ResourceRange { min: 0, max: 10 },
            herbs: ResourceRange { min: 0, max: 5 },
            water: 0,
        },
        max_resources: MaxResources { food: 15, herbs: 8 },
        travel_speed: 0.5,
        name: "Mountains",
    },
    BiomeProperties {
        biome_type: BiomeType::Swamp,
        base_danger: 40.0,
        base_resources: BaseResources {
            food: ResourceRange { min: 5, max: 25 },
            herbs: ResourceRange { min: 20, max: 40 },
            water: 0,
        },
        max_resources: MaxResources {
            food: 35,
            herbs: 50,
        },
        travel_speed: 0.7,
        name: "Swamp",
    },
    BiomeProperties {
        biome_type: BiomeType::Desert,
        base_danger: 35.0,
        base_resources: BaseResources {
            food: ResourceRange { min: 0, max: 5 },
            herbs: ResourceRange { min: 0, max: 2 },
            water: 0,
        },
        max_resources: MaxResources { food: 8, herbs: 4 },
        travel_speed: 1.3,
        name: "Desert",
    },
    BiomeProperties {
        biome_type: BiomeType::Tundra,
        base_danger: 45.0,
        base_resources: BaseResources {
            food: ResourceRange { min: 0, max: 8 },
            herbs: ResourceRange { min: 0, max: 0 },
            water: 0,
        },
        max_resources: MaxResources { food: 12, herbs: 0 },
        travel_speed: 0.9,
        name: "Tundra",
    },
    BiomeProperties {
        biome_type: BiomeType::Meadow,
        base_danger: 10.0,
        base_resources: BaseResources {
            food: ResourceRange { min: 8, max: 25 },
            herbs: ResourceRange { min: 0, max: 4 },
            water: 0,
        },
        max_resources: MaxResources { food: 30, herbs: 6 },
        travel_speed: 1.2,
        name: "Meadow",
    },
    BiomeProperties {
        biome_type: BiomeType::CaveEntrance,
        base_danger: 60.0,
        base_resources: BaseResources {
            food: ResourceRange { min: 0, max: 0 },
            herbs: ResourceRange { min: 0, max: 0 },
            water: 0,
        },
        max_resources: MaxResources { food: 0, herbs: 0 },
        travel_speed: 1.0,
        name: "Cave Entrance",
    },
    BiomeProperties {
        biome_type: BiomeType::EnemyLair,
        base_danger: 80.0,
        base_resources: BaseResources {
            food: ResourceRange { min: 0, max: 0 },
            herbs: ResourceRange { min: 0, max: 0 },
            water: 0,
        },
        max_resources: MaxResources { food: 0, herbs: 0 },
        travel_speed: 0.8,
        name: "Enemy Lair",
    },
];

pub const OVERLAY_FEATURE_PROPERTIES: [OverlayFeatureProperties; 4] = [
    OverlayFeatureProperties {
        danger_modifier: 5.0,
        speed_modifier: 0.8,
        initial_path_wear: 0,
        name: "River",
    },
    OverlayFeatureProperties {
        danger_modifier: -15.0,
        speed_modifier: 0.5,
        initial_path_wear: 60,
        name: "Ancient Road",
    },
    OverlayFeatureProperties {
        danger_modifier: -5.0,
        speed_modifier: 0.2,
        initial_path_wear: 45,
        name: "Game Trail",
    },
    OverlayFeatureProperties {
        danger_modifier: -10.0,
        speed_modifier: 0.3,
        initial_path_wear: 50,
        name: "Trade Route",
    },
];

#[must_use]
pub fn biome_properties(biome_type: BiomeType) -> &'static BiomeProperties {
    &BIOME_PROPERTIES[biome_type as usize]
}

#[must_use]
pub fn overlay_feature_properties(
    overlay_feature: OverlayFeature,
) -> &'static OverlayFeatureProperties {
    &OVERLAY_FEATURE_PROPERTIES[overlay_feature as usize]
}

/// Calculate final danger level for a tile.
#[must_use]
pub fn calculate_danger_level(
    biome_type: BiomeType,
    overlay_feature: Option<OverlayFeature>,
    distance_from_colony: f64,
) -> f64 {
    let biome = biome_properties(biome_type);
    let mut danger = biome.base_danger;

    match overlay_feature {
        Some(OverlayFeature::River) => return 5.0,
        Some(feature) => {
            let feature = overlay_feature_properties(feature);
            danger = js_max(0.0, danger + feature.danger_modifier);
        }
        None => {}
    }

    danger = js_min(95.0, danger + distance_from_colony * 2.0);

    js_max(0.0, js_min(100.0, danger))
}

fn js_min(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else {
        left.min(right)
    }
}

fn js_max(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else {
        left.max(right)
    }
}

/// Calculate travel speed multiplier for a tile.
#[must_use]
pub fn calculate_travel_speed(
    biome_type: BiomeType,
    overlay_feature: Option<OverlayFeature>,
) -> f64 {
    let biome = biome_properties(biome_type);
    let mut speed = biome.travel_speed;

    if let Some(feature) = overlay_feature
        && feature != OverlayFeature::River
    {
        let feature = overlay_feature_properties(feature);
        speed *= 1.0 + feature.speed_modifier;
    }

    speed
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{
        BIOME_PROPERTIES, BiomeType, OVERLAY_FEATURE_PROPERTIES, OverlayFeature,
        calculate_danger_level, calculate_travel_speed,
    };

    const EPSILON: f64 = 1e-12;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p2/biome_vectors.json"
        ))
        .expect("biome vector fixture parses")
    }

    fn str_value<'a>(value: &'a Value, label: &str) -> &'a str {
        value
            .as_str()
            .unwrap_or_else(|| panic!("{label} is a string"))
    }

    fn u32_value(value: &Value, label: &str) -> u32 {
        value.as_u64().unwrap_or_else(|| panic!("{label} is a u64")) as u32
    }

    fn f64_value(value: &Value, label: &str) -> f64 {
        value
            .as_f64()
            .unwrap_or_else(|| panic!("{label} is an f64"))
    }

    fn assert_js_float_eq(actual: f64, expected: f64, context: &str) {
        if actual.to_bits() == expected.to_bits() {
            return;
        }

        assert!(
            (actual - expected).abs() <= EPSILON,
            "{context}: actual {actual:?} expected {expected:?}"
        );
    }

    fn option_overlay_from_value(value: &Value) -> Option<OverlayFeature> {
        value
            .as_str()
            .map(|overlay| overlay.parse().expect("fixture overlay parses"))
    }

    #[test]
    fn biome_literals_and_properties_match_ts_fixture() {
        let fixture = fixture();
        let expected_biomes = fixture["biomes"].as_array().expect("biomes array");

        assert_eq!(BiomeType::ALL.len(), expected_biomes.len());
        assert_eq!(BIOME_PROPERTIES.len(), expected_biomes.len());

        for (biome, expected) in BiomeType::ALL.iter().zip(expected_biomes) {
            let literal = str_value(expected, "biome literal");
            assert_eq!(biome.as_str(), literal);
            assert_eq!(literal.parse::<BiomeType>().expect("parses biome"), *biome);

            let fixture_props = &fixture["biomeProperties"][literal];
            let props = super::biome_properties(*biome);

            assert_eq!(props.biome_type, *biome);
            assert_eq!(
                props.biome_type.as_str(),
                str_value(&fixture_props["type"], "biome type")
            );
            assert_js_float_eq(
                props.base_danger,
                f64_value(&fixture_props["baseDanger"], "base danger"),
                literal,
            );
            assert_eq!(
                props.base_resources.food.min,
                u32_value(&fixture_props["baseResources"]["food"]["min"], "food min")
            );
            assert_eq!(
                props.base_resources.food.max,
                u32_value(&fixture_props["baseResources"]["food"]["max"], "food max")
            );
            assert_eq!(
                props.base_resources.herbs.min,
                u32_value(&fixture_props["baseResources"]["herbs"]["min"], "herbs min")
            );
            assert_eq!(
                props.base_resources.herbs.max,
                u32_value(&fixture_props["baseResources"]["herbs"]["max"], "herbs max")
            );
            assert_eq!(
                props.base_resources.water,
                u32_value(&fixture_props["baseResources"]["water"], "water")
            );
            assert_eq!(
                props.max_resources.food,
                u32_value(&fixture_props["maxResources"]["food"], "max food")
            );
            assert_eq!(
                props.max_resources.herbs,
                u32_value(&fixture_props["maxResources"]["herbs"], "max herbs")
            );
            assert_js_float_eq(
                props.travel_speed,
                f64_value(&fixture_props["travelSpeed"], "travel speed"),
                literal,
            );
            assert_eq!(props.name, str_value(&fixture_props["name"], "biome name"));
        }
    }

    #[test]
    fn overlay_literals_and_properties_match_ts_fixture() {
        let fixture = fixture();
        let expected_overlays = fixture["overlays"].as_array().expect("overlays array");
        let expected_present_overlays: Vec<&Value> = expected_overlays
            .iter()
            .filter(|value| !value.is_null())
            .collect();

        assert_eq!(OverlayFeature::ALL.len(), expected_present_overlays.len());
        assert_eq!(
            OVERLAY_FEATURE_PROPERTIES.len(),
            expected_present_overlays.len()
        );

        for (overlay, expected) in OverlayFeature::ALL.iter().zip(expected_present_overlays) {
            let literal = str_value(expected, "overlay literal");
            assert_eq!(overlay.as_str(), literal);
            assert_eq!(
                literal
                    .parse::<OverlayFeature>()
                    .expect("parses overlay feature"),
                *overlay
            );

            let fixture_props = &fixture["overlayFeatureProperties"][literal];
            let props = super::overlay_feature_properties(*overlay);

            assert_js_float_eq(
                props.danger_modifier,
                f64_value(&fixture_props["dangerModifier"], "danger modifier"),
                literal,
            );
            assert_js_float_eq(
                props.speed_modifier,
                f64_value(&fixture_props["speedModifier"], "speed modifier"),
                literal,
            );
            assert_eq!(
                props.initial_path_wear,
                u32_value(&fixture_props["initialPathWear"], "initial path wear")
            );
            assert_eq!(
                props.name,
                str_value(&fixture_props["name"], "overlay name")
            );
        }
    }

    #[test]
    fn calculator_vectors_match_ts_fixture() {
        let fixture = fixture();
        let vectors = fixture["vectors"].as_array().expect("vectors array");

        for vector in vectors {
            let biome = vector["biome"]
                .as_str()
                .expect("vector biome")
                .parse::<BiomeType>()
                .expect("parses vector biome");
            let overlay = option_overlay_from_value(&vector["overlay"]);
            let distance = vector["distanceFromColony"]
                .as_f64()
                .expect("vector distance");
            let expected_danger = f64_value(&vector["dangerLevel"], "vector danger");
            let expected_travel_speed = f64_value(&vector["travelSpeed"], "vector travel speed");
            let context = format!(
                "biome={} overlay={:?} distance={}",
                biome.as_str(),
                overlay.map(OverlayFeature::as_str),
                distance
            );

            assert_js_float_eq(
                calculate_danger_level(biome, overlay, distance),
                expected_danger,
                &context,
            );
            assert_js_float_eq(
                calculate_travel_speed(biome, overlay),
                expected_travel_speed,
                &context,
            );
        }
    }

    #[test]
    fn calculate_danger_level_matches_ts_nan_distance() {
        assert!(calculate_danger_level(BiomeType::OakForest, None, f64::NAN).is_nan());
    }

    #[test]
    fn calculators_are_deterministic() {
        for biome in BiomeType::ALL {
            for overlay in [
                None,
                Some(OverlayFeature::River),
                Some(OverlayFeature::AncientRoad),
                Some(OverlayFeature::GameTrail),
                Some(OverlayFeature::TradeRoute),
            ] {
                for distance in [-10.0, 0.0, 1.0, 2.5, 10.0, 42.5, 100.0] {
                    let first_danger = calculate_danger_level(*biome, overlay, distance);
                    let second_danger = calculate_danger_level(*biome, overlay, distance);
                    assert_eq!(first_danger, second_danger);

                    let first_speed = calculate_travel_speed(*biome, overlay);
                    let second_speed = calculate_travel_speed(*biome, overlay);
                    assert_eq!(first_speed, second_speed);
                }
            }
        }
    }
}
