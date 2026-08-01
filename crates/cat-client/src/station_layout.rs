//! Pure visual grammar for top-down buildings.
//!
//! Houses keep a roofed silhouette. Workplaces, civic destinations, and supply
//! stations instead expose their floor and function-readable props, following
//! the open/cutaway direction in `docs/GAME_VISION.md` and the compositions in
//! `docs/sprite-review.html`. Rendering stays in the Bevy-facing crate root;
//! this module only decides what a building means visually.

use cat_protocol::BuildingType;

/// Visual treatment for a protocol building.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuildingVisual {
    /// A true residential room may keep a roofed cottage silhouette.
    Roofed(ResidentialFacade),
    /// An exposed, walkable floor composed from individual prop sprites.
    Open(&'static StationLayout),
    /// The Hole has dedicated level-scaled anomaly art rather than station props.
    BlackHole,
    /// Infrastructure rendered by a dedicated system rather than as a point building.
    Infrastructure,
}

impl BuildingVisual {
    pub(crate) const fn is_map_building(self) -> bool {
        !matches!(self, Self::Infrastructure)
    }
}

/// Exterior silhouettes reserved for residential uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResidentialFacade {
    Cottage,
}

/// Repeated floor tile beneath an open station.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StationFloor {
    Wood,
    Stone,
    Soil,
}

/// One runtime sprite used as a readable piece of an open station.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StationProp {
    Barrel,
    Bed,
    BedGreen,
    BedOrange,
    Bookcase,
    Crate,
    CropFlowering,
    CropGrowing,
    CropMature,
    CropSprout,
    DisplayTable,
    ForgeFire,
    Haystack,
    LogPile,
    MapTable,
    MetalBasin,
    OrePile,
    Sack,
    Scarecrow,
    StonePile,
    Stove,
    Stool,
    Scroll,
    SwordBlock,
    WeaponStand,
    Well,
    Workbench,
}

impl StationProp {
    /// Native dimensions of the tracked source sprite. Runtime geometry keeps
    /// this aspect ratio while fitting the sprite within its building footprint.
    pub(crate) const fn native_px(self) -> (u16, u16) {
        match self {
            Self::Bed
            | Self::BedGreen
            | Self::BedOrange
            | Self::Bookcase
            | Self::MapTable
            | Self::Workbench => (34, 16),
            Self::DisplayTable => (51, 16),
            Self::Scarecrow | Self::Well => (16, 32),
            Self::Barrel
            | Self::Crate
            | Self::CropFlowering
            | Self::CropGrowing
            | Self::CropMature
            | Self::CropSprout
            | Self::ForgeFire
            | Self::Haystack
            | Self::LogPile
            | Self::MetalBasin
            | Self::OrePile
            | Self::Sack
            | Self::Scroll
            | Self::StonePile
            | Self::Stove
            | Self::Stool
            | Self::SwordBlock
            | Self::WeaponStand => (16, 16),
        }
    }
}

/// A prop's centre in thousandths of the building footprint.
///
/// Normalized coordinates let the same authored composition fit the 2x3 civic
/// plots and the 3x3 workshop yards without encoding Bevy/world geometry here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PropPlacement {
    pub(crate) prop: StationProp,
    pub(crate) x: u16,
    pub(crate) y: u16,
}

const fn prop(prop: StationProp, x: u16, y: u16) -> PropPlacement {
    PropPlacement { prop, x, y }
}

/// Complete authored composition for one open station.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StationLayout {
    pub(crate) floor: StationFloor,
    pub(crate) props: &'static [PropPlacement],
}

const FOOD_STORAGE: StationLayout = StationLayout {
    floor: StationFloor::Wood,
    props: &[
        prop(StationProp::Crate, 230, 260),
        prop(StationProp::Sack, 720, 280),
        prop(StationProp::Barrel, 500, 730),
    ],
};
const WATER_COURT: StationLayout = StationLayout {
    floor: StationFloor::Stone,
    // Water bowls occupy a single tile in the simulation. One unmistakable
    // well sprite reads more cleanly there than three overlapping miniatures.
    props: &[prop(StationProp::Well, 500, 500)],
};
const HERB_GARDEN: StationLayout = StationLayout {
    floor: StationFloor::Soil,
    props: &[
        prop(StationProp::CropFlowering, 250, 260),
        prop(StationProp::CropGrowing, 740, 260),
        prop(StationProp::CropMature, 250, 740),
        prop(StationProp::Sack, 740, 740),
    ],
};
const MOUSE_FARM: StationLayout = StationLayout {
    floor: StationFloor::Soil,
    props: &[
        prop(StationProp::Haystack, 240, 240),
        prop(StationProp::Sack, 730, 260),
        prop(StationProp::Crate, 500, 740),
    ],
};
const WORKSHOP: StationLayout = StationLayout {
    floor: StationFloor::Wood,
    props: &[
        prop(StationProp::Workbench, 500, 270),
        prop(StationProp::Crate, 250, 760),
        prop(StationProp::Barrel, 760, 760),
    ],
};
const ACCOUNTING_TENT: StationLayout = StationLayout {
    floor: StationFloor::Wood,
    props: &[
        // An open ledger/map table anchors the counting station. The loose
        // scroll and paired sack/crate make recorded incoming stores legible
        // from above without relying on a floating text label.
        prop(StationProp::MapTable, 500, 260),
        prop(StationProp::Scroll, 190, 710),
        prop(StationProp::Sack, 500, 760),
        prop(StationProp::Crate, 810, 710),
    ],
};
const FIELD: StationLayout = StationLayout {
    floor: StationFloor::Soil,
    props: &[
        prop(StationProp::CropSprout, 220, 220),
        prop(StationProp::CropGrowing, 720, 220),
        prop(StationProp::CropFlowering, 220, 730),
        prop(StationProp::CropMature, 720, 730),
        prop(StationProp::Scarecrow, 500, 490),
    ],
};
const RESEARCH_HUT: StationLayout = StationLayout {
    floor: StationFloor::Wood,
    props: &[
        prop(StationProp::MapTable, 500, 270),
        prop(StationProp::Bookcase, 500, 760),
        prop(StationProp::Scroll, 160, 500),
    ],
};
const SCHOOL: StationLayout = StationLayout {
    floor: StationFloor::Wood,
    props: &[
        prop(StationProp::MapTable, 500, 230),
        prop(StationProp::Bookcase, 500, 780),
        prop(StationProp::Stool, 170, 520),
    ],
};
const SMITHY: StationLayout = StationLayout {
    floor: StationFloor::Stone,
    props: &[
        prop(StationProp::Workbench, 560, 250),
        prop(StationProp::ForgeFire, 210, 720),
        prop(StationProp::Stove, 500, 720),
        prop(StationProp::MetalBasin, 800, 720),
    ],
};
const BARRACKS: StationLayout = StationLayout {
    floor: StationFloor::Stone,
    props: &[
        prop(StationProp::Bed, 500, 250),
        prop(StationProp::WeaponStand, 220, 740),
        prop(StationProp::SwordBlock, 780, 740),
    ],
};
const WOOD_CUTTER: StationLayout = StationLayout {
    floor: StationFloor::Wood,
    props: &[
        prop(StationProp::Workbench, 500, 270),
        prop(StationProp::LogPile, 230, 750),
        prop(StationProp::Crate, 770, 750),
    ],
};
const STONE_PREP: StationLayout = StationLayout {
    floor: StationFloor::Stone,
    props: &[
        prop(StationProp::Workbench, 500, 270),
        prop(StationProp::StonePile, 230, 750),
        prop(StationProp::OrePile, 770, 750),
    ],
};
const WOODWORKING: StationLayout = StationLayout {
    floor: StationFloor::Wood,
    props: &[
        prop(StationProp::Workbench, 500, 270),
        prop(StationProp::Crate, 230, 750),
        prop(StationProp::LogPile, 770, 750),
    ],
};
const CLOTHIER: StationLayout = StationLayout {
    floor: StationFloor::Wood,
    props: &[
        prop(StationProp::DisplayTable, 500, 280),
        prop(StationProp::BedGreen, 300, 750),
        prop(StationProp::BedOrange, 700, 750),
    ],
};
const TANNERY: StationLayout = StationLayout {
    floor: StationFloor::Wood,
    props: &[
        prop(StationProp::Workbench, 500, 270),
        prop(StationProp::Barrel, 230, 750),
        prop(StationProp::MetalBasin, 770, 750),
    ],
};
const SMELTER: StationLayout = StationLayout {
    floor: StationFloor::Stone,
    props: &[
        prop(StationProp::ForgeFire, 500, 320),
        prop(StationProp::OrePile, 220, 760),
        prop(StationProp::OrePile, 500, 760),
        prop(StationProp::MetalBasin, 790, 760),
    ],
};
const MILL: StationLayout = StationLayout {
    floor: StationFloor::Stone,
    props: &[
        // The workbench reads as the milling surface while the three distinct
        // containers make the grain-in/flour-out flow legible from above.
        prop(StationProp::Workbench, 500, 270),
        prop(StationProp::Sack, 190, 750),
        prop(StationProp::Sack, 500, 750),
        prop(StationProp::Barrel, 810, 750),
    ],
};
const SAWMILL: StationLayout = StationLayout {
    floor: StationFloor::Wood,
    props: &[
        // The long reviewed display-table prop becomes a saw bed, flanked by
        // unmistakable raw-log input and a separate finished-goods crate.
        prop(StationProp::DisplayTable, 500, 300),
        prop(StationProp::LogPile, 210, 760),
        prop(StationProp::Crate, 790, 760),
    ],
};

/// Exhaustive visual decision for every building currently present on the wire.
///
/// Future protocol variants such as Mill and Sawmill will make this match fail
/// to compile, forcing an explicit layout rather than silently gaining a house.
pub(crate) const fn building_visual(building: BuildingType) -> BuildingVisual {
    match building {
        BuildingType::Den => BuildingVisual::Roofed(ResidentialFacade::Cottage),
        BuildingType::FoodStorage => BuildingVisual::Open(&FOOD_STORAGE),
        BuildingType::WaterBowl => BuildingVisual::Open(&WATER_COURT),
        BuildingType::Beds => BuildingVisual::Roofed(ResidentialFacade::Cottage),
        BuildingType::HerbGarden => BuildingVisual::Open(&HERB_GARDEN),
        BuildingType::Nursery => BuildingVisual::Roofed(ResidentialFacade::Cottage),
        BuildingType::ElderCorner => BuildingVisual::Roofed(ResidentialFacade::Cottage),
        BuildingType::Walls => BuildingVisual::Infrastructure,
        BuildingType::MouseFarm => BuildingVisual::Open(&MOUSE_FARM),
        BuildingType::Shrine => BuildingVisual::BlackHole,
        BuildingType::Workshop => BuildingVisual::Open(&WORKSHOP),
        BuildingType::AccountingTent => BuildingVisual::Open(&ACCOUNTING_TENT),
        BuildingType::Field => BuildingVisual::Open(&FIELD),
        BuildingType::ResearchHut => BuildingVisual::Open(&RESEARCH_HUT),
        BuildingType::School => BuildingVisual::Open(&SCHOOL),
        BuildingType::Smithy => BuildingVisual::Open(&SMITHY),
        BuildingType::Barracks => BuildingVisual::Open(&BARRACKS),
        BuildingType::WoodCutter => BuildingVisual::Open(&WOOD_CUTTER),
        BuildingType::StonePrep => BuildingVisual::Open(&STONE_PREP),
        BuildingType::Woodworking => BuildingVisual::Open(&WOODWORKING),
        BuildingType::Clothier => BuildingVisual::Open(&CLOTHIER),
        BuildingType::Tannery => BuildingVisual::Open(&TANNERY),
        BuildingType::Smelter => BuildingVisual::Open(&SMELTER),
        BuildingType::Mill => BuildingVisual::Open(&MILL),
        BuildingType::Sawmill => BuildingVisual::Open(&SAWMILL),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_BUILDINGS: [BuildingType; 25] = [
        BuildingType::Den,
        BuildingType::FoodStorage,
        BuildingType::WaterBowl,
        BuildingType::Beds,
        BuildingType::HerbGarden,
        BuildingType::Nursery,
        BuildingType::ElderCorner,
        BuildingType::Walls,
        BuildingType::MouseFarm,
        BuildingType::Shrine,
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

    #[test]
    fn every_protocol_building_has_one_explicit_visual_treatment() {
        assert_eq!(ALL_BUILDINGS.len(), 25);
        for building in ALL_BUILDINGS {
            match building_visual(building) {
                BuildingVisual::Open(layout) => {
                    assert!(
                        !layout.props.is_empty(),
                        "{building:?} needs readable props"
                    );
                    assert!(layout.props.iter().all(|p| p.x <= 1000 && p.y <= 1000));
                }
                BuildingVisual::Roofed(_)
                | BuildingVisual::BlackHole
                | BuildingVisual::Infrastructure => {}
            }
        }
    }

    #[test]
    fn all_craft_workshops_are_open_and_functionally_distinct() {
        let workshops = [
            BuildingType::Workshop,
            BuildingType::Smithy,
            BuildingType::WoodCutter,
            BuildingType::StonePrep,
            BuildingType::Woodworking,
            BuildingType::Clothier,
            BuildingType::Tannery,
            BuildingType::Smelter,
            BuildingType::Mill,
            BuildingType::Sawmill,
        ];
        let mut signatures = Vec::new();
        for building in workshops {
            let BuildingVisual::Open(layout) = building_visual(building) else {
                panic!("{building:?} must be an open station");
            };
            signatures.push(layout.props);
        }
        for (index, left) in signatures.iter().enumerate() {
            assert!(
                signatures[index + 1..].iter().all(|right| left != right),
                "workshop compositions must remain distinct"
            );
        }
    }

    #[test]
    fn accounting_tent_is_a_distinct_open_ledger_station() {
        let BuildingVisual::Open(accounting) = building_visual(BuildingType::AccountingTent) else {
            panic!("accounting tent must be an open station");
        };
        let BuildingVisual::Open(workshop) = building_visual(BuildingType::Workshop) else {
            panic!("workshop must be an open station");
        };
        let BuildingVisual::Open(research) = building_visual(BuildingType::ResearchHut) else {
            panic!("research hut must be an open station");
        };

        assert_eq!(accounting.floor, StationFloor::Wood);
        assert_ne!(accounting.props, workshop.props);
        assert_ne!(accounting.props, research.props);
        for expected in [
            StationProp::MapTable,
            StationProp::Scroll,
            StationProp::Sack,
            StationProp::Crate,
        ] {
            assert!(
                accounting
                    .props
                    .iter()
                    .any(|placed| placed.prop == expected),
                "accounting tent is missing {expected:?}"
            );
        }
    }

    #[test]
    fn only_residential_rooms_keep_roofs() {
        let roofed = ALL_BUILDINGS
            .into_iter()
            .filter(|building| matches!(building_visual(*building), BuildingVisual::Roofed(_)))
            .collect::<Vec<_>>();
        assert_eq!(
            roofed,
            [
                BuildingType::Den,
                BuildingType::Beds,
                BuildingType::Nursery,
                BuildingType::ElderCorner,
            ]
        );
    }

    #[test]
    fn field_and_black_hole_cannot_regress_to_facades() {
        let BuildingVisual::Open(field) = building_visual(BuildingType::Field) else {
            panic!("field must be an open crop plot");
        };
        assert_eq!(field.floor, StationFloor::Soil);
        assert!(
            field
                .props
                .iter()
                .any(|p| p.prop == StationProp::CropMature)
        );

        assert_eq!(
            building_visual(BuildingType::Shrine),
            BuildingVisual::BlackHole
        );
    }

    #[test]
    fn mill_and_sawmill_are_distinct_roofless_production_stations() {
        let BuildingVisual::Open(mill) = building_visual(BuildingType::Mill) else {
            panic!("mill must be an open station");
        };
        let BuildingVisual::Open(sawmill) = building_visual(BuildingType::Sawmill) else {
            panic!("sawmill must be an open station");
        };
        assert_eq!(mill.floor, StationFloor::Stone);
        assert_eq!(sawmill.floor, StationFloor::Wood);
        assert_ne!(mill.props, sawmill.props);
        assert!(mill.props.iter().any(|p| p.prop == StationProp::Sack));
        assert!(sawmill.props.iter().any(|p| p.prop == StationProp::LogPile));
        assert!(
            sawmill
                .props
                .iter()
                .any(|p| p.prop == StationProp::DisplayTable)
        );
    }
}
