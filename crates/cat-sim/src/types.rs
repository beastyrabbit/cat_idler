//! Closed string-literal game types ported from `types/game.ts`.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

/// Error returned when a wire literal does not match a closed game enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseEnumError {
    enum_name: &'static str,
    value: String,
}

impl ParseEnumError {
    #[must_use]
    pub fn enum_name(&self) -> &'static str {
        self.enum_name
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for ParseEnumError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown {} wire literal {:?}",
            self.enum_name, self.value
        )
    }
}

impl std::error::Error for ParseEnumError {}

macro_rules! define_wire_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $($variant:ident => $wire:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name {
            $(
                #[serde(rename = $wire)]
                $variant,
            )+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[
                $(Self::$variant,)+
            ];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire,)+
                }
            }
        }

        impl FromStr for $name {
            type Err = ParseEnumError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($wire => Ok(Self::$variant),)+
                    _ => Err(ParseEnumError {
                        enum_name: stringify!($name),
                        value: value.to_owned(),
                    }),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ParseEnumError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                value.parse()
            }
        }
    };
}

define_wire_enum! {
    pub enum LifeStage {
        Kitten => "kitten",
        Young => "young",
        Adult => "adult",
        Elder => "elder",
    }
}

define_wire_enum! {
    pub enum BuildingType {
        Den => "den",
        FoodStorage => "food_storage",
        WaterBowl => "water_bowl",
        Beds => "beds",
        HerbGarden => "herb_garden",
        Nursery => "nursery",
        ElderCorner => "elder_corner",
        Walls => "walls",
        MouseFarm => "mouse_farm",
        Shrine => "shrine",
        Workshop => "workshop",
        Field => "field",
        Smithy => "smithy",
        Barracks => "barracks",
        AccountingTent => "accounting_tent",
        WoodCutter => "wood_cutter",
        StonePrep => "stone_prep",
        Woodworking => "woodworking",
        Clothier => "clothier",
        Tannery => "tannery",
        // Cat-research building: a staffed research hut is the autonomous faucet for
        // upgrade-tree research points (see world_tick::research_workforce /
        // phase_24_research). It is buildable at founding (ungated by the tree) because it
        // is the *entry* to the research path — the tree's root node it would otherwise
        // gate cannot be earned by cats until a hut already exists to staff.
        ResearchHut => "research_hut",
        // P17/P19 ore -> metal chain: refines mountain ore into metal bars, mirroring
        // StonePrep's materials -> blocks refine. See production::advance_workshop's
        // Smelter arm in world_tick.rs and the "smelting" upgrade node.
        Smelter => "smelter",
        Mill => "mill",
        Sawmill => "sawmill",
        // Second staffed research building, unlocked by the "school" upgrade node
        // (era 2, prereq den_insulation). A completed, staffed School contributes to
        // world_tick::research_workforce exactly like a ResearchHut; the node's own
        // ResearchRateMult effect (applied in phase_24_research) then scales the total.
        School => "school",
    }
}

define_wire_enum! {
    pub enum TileType {
        Field => "field",
        Forest => "forest",
        DenseWoods => "dense_woods",
        River => "river",
        EnemyTerritory => "enemy_territory",
        OakForest => "oak_forest",
        PineForest => "pine_forest",
        Jungle => "jungle",
        DeadForest => "dead_forest",
        Mountains => "mountains",
        Swamp => "swamp",
        Desert => "desert",
        Tundra => "tundra",
        Meadow => "meadow",
        CaveEntrance => "cave_entrance",
        EnemyLair => "enemy_lair",
    }
}

define_wire_enum! {
    pub enum TaskType {
        Hunt => "hunt",
        GatherHerbs => "gather_herbs",
        FetchWater => "fetch_water",
        Clean => "clean",
        Build => "build",
        Guard => "guard",
        Heal => "heal",
        Kitsit => "kitsit",
        Explore => "explore",
        Patrol => "patrol",
        Teach => "teach",
        Rest => "rest",
    }
}

define_wire_enum! {
    pub enum EnemyType {
        Fox => "fox",
        Hawk => "hawk",
        Badger => "badger",
        Bear => "bear",
        RivalCat => "rival_cat",
    }
}

define_wire_enum! {
    pub enum JobKind {
        SupplyFood => "supply_food",
        SupplyWater => "supply_water",
        LeaderPlanHunt => "leader_plan_hunt",
        HuntExpedition => "hunt_expedition",
        LeaderPlanHouse => "leader_plan_house",
        BuildHouse => "build_house",
        Ritual => "ritual",
        Quarry => "quarry",
        GatherLogs => "gather_logs",
        ForageFibre => "forage_fibre",
        Explore => "explore",
        FetchWater => "fetch_water",
        TrainWarrior => "train_warrior",
        ExpandVillage => "expand_village",
        CarryOffering => "carry_offering",
        HaulGatherSpot => "haul_gather_spot",
    }
}

define_wire_enum! {
    pub enum JobStatus {
        Queued => "queued",
        Active => "active",
        Completed => "completed",
        Failed => "failed",
        Cancelled => "cancelled",
    }
}

define_wire_enum! {
    pub enum CatSpecialization {
        Hunter => "hunter",
        Architect => "architect",
        Ritualist => "ritualist",
        Warrior => "warrior",
    }
}

define_wire_enum! {
    pub enum UpgradeKey {
        ClickPower => "click_power",
        SupplySpeed => "supply_speed",
        HuntMastery => "hunt_mastery",
        BuildMastery => "build_mastery",
        RitualMastery => "ritual_mastery",
        Resilience => "resilience",
    }
}

define_wire_enum! {
    pub enum PolicyTier {
        Simple => "simple",
        Normal => "normal",
        Excellent => "excellent",
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt::Debug, str::FromStr};

    use serde::{Serialize, de::DeserializeOwned};

    use super::{
        BuildingType, CatSpecialization, EnemyType, JobKind, JobStatus, LifeStage, PolicyTier,
        TaskType, TileType, UpgradeKey,
    };

    fn assert_wire_round_trip<T>(cases: &[(T, &str)], as_str: fn(T) -> &'static str)
    where
        T: Copy + Debug + Eq + FromStr<Err = super::ParseEnumError> + Serialize + DeserializeOwned,
    {
        for (variant, wire) in cases {
            assert_eq!(as_str(*variant), *wire);
            assert_eq!(wire.parse::<T>().expect("parses wire literal"), *variant);

            let serialized = serde_json::to_string(variant).expect("serializes enum variant");
            assert_eq!(serialized, format!("\"{wire}\""));

            let deserialized: T = serde_json::from_str(&serialized).expect("deserializes variant");
            assert_eq!(deserialized, *variant);
        }
    }

    #[test]
    fn life_stage_wire_literals_round_trip() {
        let cases = [
            (LifeStage::Kitten, "kitten"),
            (LifeStage::Young, "young"),
            (LifeStage::Adult, "adult"),
            (LifeStage::Elder, "elder"),
        ];

        assert_wire_round_trip(&cases, LifeStage::as_str);
        assert_eq!(LifeStage::ALL.len(), cases.len());
    }

    #[test]
    fn building_type_wire_literals_round_trip() {
        let cases = [
            (BuildingType::Den, "den"),
            (BuildingType::FoodStorage, "food_storage"),
            (BuildingType::WaterBowl, "water_bowl"),
            (BuildingType::Beds, "beds"),
            (BuildingType::HerbGarden, "herb_garden"),
            (BuildingType::Nursery, "nursery"),
            (BuildingType::ElderCorner, "elder_corner"),
            (BuildingType::Walls, "walls"),
            (BuildingType::MouseFarm, "mouse_farm"),
            (BuildingType::Shrine, "shrine"),
            (BuildingType::Workshop, "workshop"),
            (BuildingType::Field, "field"),
            (BuildingType::Smithy, "smithy"),
            (BuildingType::Barracks, "barracks"),
            (BuildingType::AccountingTent, "accounting_tent"),
            (BuildingType::WoodCutter, "wood_cutter"),
            (BuildingType::StonePrep, "stone_prep"),
            (BuildingType::Woodworking, "woodworking"),
            (BuildingType::Clothier, "clothier"),
            (BuildingType::Tannery, "tannery"),
            (BuildingType::ResearchHut, "research_hut"),
            (BuildingType::Smelter, "smelter"),
            (BuildingType::Mill, "mill"),
            (BuildingType::Sawmill, "sawmill"),
            (BuildingType::School, "school"),
        ];

        assert_wire_round_trip(&cases, BuildingType::as_str);
        assert_eq!(BuildingType::ALL.len(), cases.len());
    }

    #[test]
    fn tile_type_wire_literals_round_trip() {
        let cases = [
            (TileType::Field, "field"),
            (TileType::Forest, "forest"),
            (TileType::DenseWoods, "dense_woods"),
            (TileType::River, "river"),
            (TileType::EnemyTerritory, "enemy_territory"),
            (TileType::OakForest, "oak_forest"),
            (TileType::PineForest, "pine_forest"),
            (TileType::Jungle, "jungle"),
            (TileType::DeadForest, "dead_forest"),
            (TileType::Mountains, "mountains"),
            (TileType::Swamp, "swamp"),
            (TileType::Desert, "desert"),
            (TileType::Tundra, "tundra"),
            (TileType::Meadow, "meadow"),
            (TileType::CaveEntrance, "cave_entrance"),
            (TileType::EnemyLair, "enemy_lair"),
        ];

        assert_wire_round_trip(&cases, TileType::as_str);
        assert_eq!(TileType::ALL.len(), cases.len());
    }

    #[test]
    fn task_type_wire_literals_round_trip() {
        let cases = [
            (TaskType::Hunt, "hunt"),
            (TaskType::GatherHerbs, "gather_herbs"),
            (TaskType::FetchWater, "fetch_water"),
            (TaskType::Clean, "clean"),
            (TaskType::Build, "build"),
            (TaskType::Guard, "guard"),
            (TaskType::Heal, "heal"),
            (TaskType::Kitsit, "kitsit"),
            (TaskType::Explore, "explore"),
            (TaskType::Patrol, "patrol"),
            (TaskType::Teach, "teach"),
            (TaskType::Rest, "rest"),
        ];

        assert_wire_round_trip(&cases, TaskType::as_str);
        assert_eq!(TaskType::ALL.len(), cases.len());
    }

    #[test]
    fn enemy_type_wire_literals_round_trip() {
        let cases = [
            (EnemyType::Fox, "fox"),
            (EnemyType::Hawk, "hawk"),
            (EnemyType::Badger, "badger"),
            (EnemyType::Bear, "bear"),
            (EnemyType::RivalCat, "rival_cat"),
        ];

        assert_wire_round_trip(&cases, EnemyType::as_str);
        assert_eq!(EnemyType::ALL.len(), cases.len());
    }

    #[test]
    fn job_kind_wire_literals_round_trip() {
        let cases = [
            (JobKind::SupplyFood, "supply_food"),
            (JobKind::SupplyWater, "supply_water"),
            (JobKind::LeaderPlanHunt, "leader_plan_hunt"),
            (JobKind::HuntExpedition, "hunt_expedition"),
            (JobKind::LeaderPlanHouse, "leader_plan_house"),
            (JobKind::BuildHouse, "build_house"),
            (JobKind::Ritual, "ritual"),
            (JobKind::Quarry, "quarry"),
            (JobKind::GatherLogs, "gather_logs"),
            (JobKind::ForageFibre, "forage_fibre"),
            (JobKind::Explore, "explore"),
            (JobKind::FetchWater, "fetch_water"),
            (JobKind::TrainWarrior, "train_warrior"),
            (JobKind::ExpandVillage, "expand_village"),
            (JobKind::CarryOffering, "carry_offering"),
            (JobKind::HaulGatherSpot, "haul_gather_spot"),
        ];

        assert_wire_round_trip(&cases, JobKind::as_str);
        assert_eq!(JobKind::ALL.len(), cases.len());
    }

    #[test]
    fn job_status_wire_literals_round_trip() {
        let cases = [
            (JobStatus::Queued, "queued"),
            (JobStatus::Active, "active"),
            (JobStatus::Completed, "completed"),
            (JobStatus::Failed, "failed"),
            (JobStatus::Cancelled, "cancelled"),
        ];

        assert_wire_round_trip(&cases, JobStatus::as_str);
        assert_eq!(JobStatus::ALL.len(), cases.len());
    }

    #[test]
    fn cat_specialization_wire_literals_round_trip() {
        let cases = [
            (CatSpecialization::Hunter, "hunter"),
            (CatSpecialization::Architect, "architect"),
            (CatSpecialization::Ritualist, "ritualist"),
            (CatSpecialization::Warrior, "warrior"),
        ];

        assert_wire_round_trip(&cases, CatSpecialization::as_str);
        assert_eq!(CatSpecialization::ALL.len(), cases.len());
    }

    #[test]
    fn cat_specialization_none_uses_json_null_wire_value() {
        let specialized = Some(CatSpecialization::Hunter);
        let serialized_some =
            serde_json::to_string(&specialized).expect("serializes specialization option");
        assert_eq!(serialized_some, "\"hunter\"");

        let deserialized_some: Option<CatSpecialization> =
            serde_json::from_str("\"hunter\"").expect("deserializes specialization string");
        assert_eq!(deserialized_some, specialized);

        let absent: Option<CatSpecialization> = None;
        let serialized_none =
            serde_json::to_string(&absent).expect("serializes absent specialization");
        assert_eq!(serialized_none, "null");

        let deserialized_none: Option<CatSpecialization> =
            serde_json::from_str("null").expect("deserializes JSON null");
        assert_eq!(deserialized_none, absent);
    }

    #[test]
    fn upgrade_key_wire_literals_round_trip() {
        let cases = [
            (UpgradeKey::ClickPower, "click_power"),
            (UpgradeKey::SupplySpeed, "supply_speed"),
            (UpgradeKey::HuntMastery, "hunt_mastery"),
            (UpgradeKey::BuildMastery, "build_mastery"),
            (UpgradeKey::RitualMastery, "ritual_mastery"),
            (UpgradeKey::Resilience, "resilience"),
        ];

        assert_wire_round_trip(&cases, UpgradeKey::as_str);
        assert_eq!(UpgradeKey::ALL.len(), cases.len());
    }

    #[test]
    fn policy_tier_wire_literals_round_trip() {
        let cases = [
            (PolicyTier::Simple, "simple"),
            (PolicyTier::Normal, "normal"),
            (PolicyTier::Excellent, "excellent"),
        ];

        assert_wire_round_trip(&cases, PolicyTier::as_str);
        assert_eq!(PolicyTier::ALL.len(), cases.len());
    }
}
