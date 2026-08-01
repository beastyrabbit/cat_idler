//! Pure start-charter and mature showcase models.
//!
//! This semantically adapts `bug-gui-design/start_screen.rs` and
//! `landing_showcase.rs` as an off-map static presentation catalog. The API has
//! no input or output type from protocol, simulation, persistence, selection,
//! networking, or authoritative ECS state.

use super::{
    layout::{CharterPlacement, ScrollState, ShellLayout},
    shell::{ConnectionState, LocalizationKey, SemanticPixelIcon},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShowcaseBuildingKind {
    Hole,
    Workshop,
    FamilyHome,
    ElderLodge,
    Cookhouse,
    FishingHut,
    Farm,
    StorageYard,
    Carpentry,
    Smithy,
    Tannery,
    Weavery,
    Pottery,
    Mill,
    Bakery,
    Infirmary,
    School,
    CouncilHall,
    Barracks,
    Watchtower,
    Nursery,
    Waterworks,
    Market,
    Orchard,
    Brewery,
    Masonry,
    Apiary,
    Gatehouse,
    Guardpost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShowcaseFootprint {
    pub x: i16,
    pub y: i16,
    pub width: u8,
    pub height: u8,
}

impl ShowcaseFootprint {
    pub const fn new(x: i16, y: i16, width: u8, height: u8) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn is_centered_five_by_five(self) -> bool {
        self.x == -2 && self.y == -2 && self.width == 5 && self.height == 5
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShowcaseLot {
    pub presentation_id: u8,
    pub kind: ShowcaseBuildingKind,
    pub footprint: ShowcaseFootprint,
}

const fn lot(
    presentation_id: u8,
    kind: ShowcaseBuildingKind,
    x: i16,
    y: i16,
    width: u8,
    height: u8,
) -> ShowcaseLot {
    ShowcaseLot {
        presentation_id,
        kind,
        footprint: ShowcaseFootprint::new(x, y, width, height),
    }
}

pub const SHOWCASE_LOTS: [ShowcaseLot; 48] = [
    lot(0, ShowcaseBuildingKind::Hole, -2, -2, 5, 5),
    lot(1, ShowcaseBuildingKind::Workshop, 7, -2, 3, 3),
    lot(2, ShowcaseBuildingKind::FamilyHome, -30, -17, 4, 3),
    lot(3, ShowcaseBuildingKind::FamilyHome, -23, -17, 4, 3),
    lot(4, ShowcaseBuildingKind::FamilyHome, -16, -17, 4, 3),
    lot(5, ShowcaseBuildingKind::FamilyHome, 14, -17, 4, 3),
    lot(6, ShowcaseBuildingKind::FamilyHome, 21, -17, 4, 3),
    lot(7, ShowcaseBuildingKind::FamilyHome, 28, -17, 4, 3),
    lot(8, ShowcaseBuildingKind::ElderLodge, -9, -17, 5, 4),
    lot(9, ShowcaseBuildingKind::Cookhouse, 5, -17, 5, 4),
    lot(10, ShowcaseBuildingKind::FishingHut, 28, 13, 4, 4),
    lot(11, ShowcaseBuildingKind::Farm, -31, 13, 7, 5),
    lot(12, ShowcaseBuildingKind::Farm, -22, 13, 7, 5),
    lot(13, ShowcaseBuildingKind::Farm, -13, 13, 7, 5),
    lot(14, ShowcaseBuildingKind::Farm, -4, 13, 7, 5),
    lot(15, ShowcaseBuildingKind::StorageYard, -30, -9, 5, 4),
    lot(16, ShowcaseBuildingKind::StorageYard, -23, -9, 5, 4),
    lot(17, ShowcaseBuildingKind::StorageYard, 24, -9, 5, 4),
    lot(18, ShowcaseBuildingKind::Carpentry, -15, -9, 5, 4),
    lot(19, ShowcaseBuildingKind::Smithy, -8, -9, 5, 4),
    lot(20, ShowcaseBuildingKind::Tannery, 6, -9, 5, 4),
    lot(21, ShowcaseBuildingKind::Weavery, 13, -9, 5, 4),
    lot(22, ShowcaseBuildingKind::Pottery, 20, -9, 4, 4),
    lot(23, ShowcaseBuildingKind::Mill, -30, 5, 5, 4),
    lot(24, ShowcaseBuildingKind::Bakery, -23, 5, 5, 4),
    lot(25, ShowcaseBuildingKind::Infirmary, -16, 5, 5, 4),
    lot(26, ShowcaseBuildingKind::School, -9, 5, 5, 4),
    lot(27, ShowcaseBuildingKind::CouncilHall, -2, 6, 5, 4),
    lot(28, ShowcaseBuildingKind::Barracks, 5, 5, 5, 4),
    lot(29, ShowcaseBuildingKind::Watchtower, -34, -20, 3, 3),
    lot(30, ShowcaseBuildingKind::Watchtower, 32, -20, 3, 3),
    lot(31, ShowcaseBuildingKind::Nursery, 12, 5, 5, 4),
    lot(32, ShowcaseBuildingKind::Waterworks, 19, 5, 5, 4),
    lot(33, ShowcaseBuildingKind::Market, 26, 5, 6, 4),
    lot(34, ShowcaseBuildingKind::Orchard, 6, 13, 6, 5),
    lot(35, ShowcaseBuildingKind::Orchard, 14, 13, 6, 5),
    lot(36, ShowcaseBuildingKind::Brewery, 12, -2, 5, 4),
    lot(37, ShowcaseBuildingKind::Masonry, 19, -2, 5, 4),
    lot(38, ShowcaseBuildingKind::Apiary, 26, -2, 4, 4),
    lot(39, ShowcaseBuildingKind::FamilyHome, -33, -2, 4, 3),
    lot(40, ShowcaseBuildingKind::FamilyHome, -26, -2, 4, 3),
    lot(41, ShowcaseBuildingKind::Farm, 22, 13, 5, 5),
    lot(42, ShowcaseBuildingKind::StorageYard, -19, -2, 5, 4),
    lot(43, ShowcaseBuildingKind::FishingHut, 33, 10, 4, 4),
    lot(44, ShowcaseBuildingKind::Carpentry, -32, 6, 5, 4),
    lot(45, ShowcaseBuildingKind::Weavery, -25, 6, 5, 4),
    lot(46, ShowcaseBuildingKind::Gatehouse, -2, -22, 5, 3),
    lot(47, ShowcaseBuildingKind::Guardpost, 32, 6, 3, 3),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShowcaseInfrastructure {
    pub road_segments: u16,
    pub wall_segments: u16,
    pub gates: u8,
    pub enterprise_signs: u8,
    pub defensive_positions: u8,
}

pub const SHOWCASE_INFRASTRUCTURE: ShowcaseInfrastructure = ShowcaseInfrastructure {
    road_segments: 412,
    wall_segments: 136,
    gates: 2,
    enterprise_signs: 9,
    defensive_positions: 6,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShowcaseCat {
    pub presentation_id: u8,
    pub route_index: u8,
    pub animation_phase_ms: u32,
    pub semantic_role_icon: SemanticPixelIcon,
}

const fn build_showcase_cats() -> [ShowcaseCat; 60] {
    let mut cats = [ShowcaseCat {
        presentation_id: 0,
        route_index: 0,
        animation_phase_ms: 0,
        semantic_role_icon: SemanticPixelIcon::CatProfile,
    }; 60];
    let mut index = 0;
    while index < cats.len() {
        cats[index] = ShowcaseCat {
            presentation_id: index as u8,
            route_index: (index % 12) as u8,
            animation_phase_ms: ((index as u32 * 9_973) + 137) % 60_000,
            semantic_role_icon: SemanticPixelIcon::CatProfile,
        };
        index += 1;
    }
    cats
}

pub const SHOWCASE_CATS: [ShowcaseCat; 60] = build_showcase_cats();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShowcaseBinding {
    OffMapStaticPresentation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatureShowcase {
    pub binding: ShowcaseBinding,
    pub maturity_days: u16,
    pub lots: &'static [ShowcaseLot],
    pub cats: &'static [ShowcaseCat],
    pub infrastructure: ShowcaseInfrastructure,
}

pub const MATURE_SHOWCASE: MatureShowcase = MatureShowcase {
    binding: ShowcaseBinding::OffMapStaticPresentation,
    maturity_days: 730,
    lots: &SHOWCASE_LOTS,
    cats: &SHOWCASE_CATS,
    infrastructure: SHOWCASE_INFRASTRUCTURE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShowcaseValidationError {
    Maturity,
    LotCount,
    BuildingVariety,
    Hole,
    RequiredBuilding,
    Infrastructure,
    CatCount,
    CatPhase,
}

impl MatureShowcase {
    pub fn validate(self) -> Result<(), ShowcaseValidationError> {
        if !(700..=760).contains(&self.maturity_days) {
            return Err(ShowcaseValidationError::Maturity);
        }
        if self.lots.len() < 42 {
            return Err(ShowcaseValidationError::LotCount);
        }

        let mut distinct_kinds = 0;
        for (index, lot) in self.lots.iter().enumerate() {
            if self.lots[..index]
                .iter()
                .all(|earlier| earlier.kind != lot.kind)
            {
                distinct_kinds += 1;
            }
        }
        if distinct_kinds < 18 {
            return Err(ShowcaseValidationError::BuildingVariety);
        }

        let mut holes = self
            .lots
            .iter()
            .filter(|lot| lot.kind == ShowcaseBuildingKind::Hole);
        let Some(hole) = holes.next() else {
            return Err(ShowcaseValidationError::Hole);
        };
        if holes.next().is_some() || !hole.footprint.is_centered_five_by_five() {
            return Err(ShowcaseValidationError::Hole);
        }
        for required in [
            ShowcaseBuildingKind::Workshop,
            ShowcaseBuildingKind::FamilyHome,
            ShowcaseBuildingKind::ElderLodge,
            ShowcaseBuildingKind::Cookhouse,
            ShowcaseBuildingKind::FishingHut,
            ShowcaseBuildingKind::Farm,
            ShowcaseBuildingKind::StorageYard,
            ShowcaseBuildingKind::Carpentry,
            ShowcaseBuildingKind::Watchtower,
            ShowcaseBuildingKind::Gatehouse,
        ] {
            if !self.lots.iter().any(|lot| lot.kind == required) {
                return Err(ShowcaseValidationError::RequiredBuilding);
            }
        }
        if self.infrastructure.road_segments == 0
            || self.infrastructure.wall_segments == 0
            || self.infrastructure.enterprise_signs == 0
            || self.infrastructure.defensive_positions == 0
        {
            return Err(ShowcaseValidationError::Infrastructure);
        }
        if self.cats.len() != 60 {
            return Err(ShowcaseValidationError::CatCount);
        }
        for (index, cat) in self.cats.iter().enumerate() {
            if self.cats[..index]
                .iter()
                .any(|earlier| earlier.animation_phase_ms == cat.animation_phase_ms)
            {
                return Err(ShowcaseValidationError::CatPhase);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestinationKind {
    Global,
    Personal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DestinationCard {
    pub kind: DestinationKind,
    pub title_key: LocalizationKey,
    pub detail_key: LocalizationKey,
    pub action_key: LocalizationKey,
    pub icon: SemanticPixelIcon,
}

pub const DESTINATION_CARDS: [DestinationCard; 2] = [
    DestinationCard {
        kind: DestinationKind::Global,
        title_key: LocalizationKey("start.destination.global.title"),
        detail_key: LocalizationKey("start.destination.global.detail"),
        action_key: LocalizationKey("start.destination.global.action"),
        icon: SemanticPixelIcon::VillageRadio,
    },
    DestinationCard {
        kind: DestinationKind::Personal,
        title_key: LocalizationKey("start.destination.personal.title"),
        detail_key: LocalizationKey("start.destination.personal.detail"),
        action_key: LocalizationKey("start.destination.personal.action"),
        icon: SemanticPixelIcon::Village,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartFocus {
    PlayerName,
    GlobalDestination,
    PersonalDestination,
    VillageName,
    ExplicitEntry,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryDisabledReason {
    Disconnected,
    Loading,
    PlayerName,
    Destination,
    VillageName,
    Pending,
    Error,
}

impl EntryDisabledReason {
    pub const fn label_key(self) -> LocalizationKey {
        match self {
            Self::Disconnected => LocalizationKey("start.disabled.disconnected"),
            Self::Loading => LocalizationKey("start.disabled.loading"),
            Self::PlayerName => LocalizationKey("start.disabled.player_name"),
            Self::Destination => LocalizationKey("start.disabled.destination"),
            Self::VillageName => LocalizationKey("start.disabled.village_name"),
            Self::Pending => LocalizationKey("start.disabled.pending"),
            Self::Error => LocalizationKey("start.disabled.error"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryControlState {
    Enabled,
    Disabled(EntryDisabledReason),
}

#[derive(Clone, Debug, PartialEq)]
pub struct StartCharterState {
    pub connection: ConnectionState,
    pub snapshot_loaded: bool,
    pub player_name: String,
    pub village_name: String,
    pub selected_destination: Option<DestinationKind>,
    pub pending: bool,
    pub error_key: Option<LocalizationKey>,
    pub focus: StartFocus,
    pub scroll: ScrollState,
}

impl Default for StartCharterState {
    fn default() -> Self {
        Self {
            connection: ConnectionState::Disconnected,
            snapshot_loaded: false,
            player_name: String::new(),
            village_name: String::new(),
            selected_destination: None,
            pending: false,
            error_key: None,
            focus: StartFocus::PlayerName,
            scroll: ScrollState::default(),
        }
    }
}

impl StartCharterState {
    pub fn select_destination(&mut self, destination: DestinationKind) {
        self.selected_destination = Some(destination);
        self.focus = match destination {
            DestinationKind::Global => StartFocus::GlobalDestination,
            DestinationKind::Personal => StartFocus::PersonalDestination,
        };
    }

    pub fn entry_control_state(&self) -> EntryControlState {
        if self.pending {
            return EntryControlState::Disabled(EntryDisabledReason::Pending);
        }
        if self.error_key.is_some() {
            return EntryControlState::Disabled(EntryDisabledReason::Error);
        }
        if matches!(
            self.connection,
            ConnectionState::Disconnected
                | ConnectionState::Reconnecting
                | ConnectionState::UpdateRequired
                | ConnectionState::Error
        ) {
            return EntryControlState::Disabled(EntryDisabledReason::Disconnected);
        }
        if self.connection != ConnectionState::Connected || !self.snapshot_loaded {
            return EntryControlState::Disabled(EntryDisabledReason::Loading);
        }
        if !valid_name(&self.player_name) {
            return EntryControlState::Disabled(EntryDisabledReason::PlayerName);
        }
        let Some(destination) = self.selected_destination else {
            return EntryControlState::Disabled(EntryDisabledReason::Destination);
        };
        if destination == DestinationKind::Personal && !valid_name(&self.village_name) {
            return EntryControlState::Disabled(EntryDisabledReason::VillageName);
        }
        EntryControlState::Enabled
    }

    /// Produces local explicit-entry intent only after the entry control is
    /// activated. Construction/default/selection alone can never produce it.
    pub fn explicit_entry_intent(&self) -> Result<ExplicitEntryIntent, EntryDisabledReason> {
        match self.entry_control_state() {
            EntryControlState::Enabled => {
                let Some(destination) = self.selected_destination else {
                    return Err(EntryDisabledReason::Destination);
                };
                Ok(ExplicitEntryIntent { destination })
            }
            EntryControlState::Disabled(reason) => Err(reason),
        }
    }
}

fn valid_name(value: &str) -> bool {
    let mut words = value.split_whitespace().peekable();
    let mut visible_len = 0;
    while let Some(word) = words.next() {
        visible_len += word.chars().count();
        if words.peek().is_some() {
            visible_len += 1;
        }
    }
    (2..=48).contains(&visible_len)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExplicitEntryIntent {
    pub destination: DestinationKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StartLayout {
    pub placement: CharterPlacement,
    pub charter_width_px: f32,
    pub charter_max_height_px: f32,
    pub destinations_stacked: bool,
}

pub fn start_layout(shell: ShellLayout) -> StartLayout {
    let compact = shell.charter_placement == CharterPlacement::CenteredOverShowcase;
    StartLayout {
        placement: shell.charter_placement,
        charter_width_px: if compact {
            (shell.logical_width_px * 0.92).min(560.0)
        } else {
            560.0
        },
        charter_max_height_px: shell.logical_height_px * 0.92,
        destinations_stacked: compact && shell.logical_width_px < 860.0,
    }
}
