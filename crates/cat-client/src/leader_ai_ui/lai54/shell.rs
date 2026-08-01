//! Product-normal routed shell adapted from `bug-gui-design/ui_shell.rs`.
//!
//! Navigation is explicit and pointer/focus activated, with no alternate
//! opener table or secondary top-level route catalog.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LocalizationKey(pub &'static str);

impl LocalizationKey {
    pub fn is_english_key(self) -> bool {
        self.0.contains('.')
            && self
                .0
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'.' || byte == b'_')
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PrimaryScreen {
    Log,
    Stores,
    Village,
    Research,
    Council,
}

impl PrimaryScreen {
    pub const ALL: [Self; 5] = [
        Self::Log,
        Self::Stores,
        Self::Village,
        Self::Research,
        Self::Council,
    ];

    pub const fn label_key(self) -> LocalizationKey {
        match self {
            Self::Log => LocalizationKey("shell.screen.log"),
            Self::Stores => LocalizationKey("shell.screen.stores"),
            Self::Village => LocalizationKey("shell.screen.village"),
            Self::Research => LocalizationKey("shell.screen.research"),
            Self::Council => LocalizationKey("shell.screen.council"),
        }
    }

    pub const fn icon(self) -> SemanticPixelIcon {
        match self {
            Self::Log => SemanticPixelIcon::EventLog,
            Self::Stores => SemanticPixelIcon::Crate,
            Self::Village => SemanticPixelIcon::Village,
            Self::Research => SemanticPixelIcon::ResearchNotes,
            Self::Council => SemanticPixelIcon::CouncilTable,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CouncilTab {
    Plans,
    Tasks,
    Cats,
    Hole,
    Diplomacy,
    Trade,
}

impl CouncilTab {
    pub const ALL: [Self; 6] = [
        Self::Plans,
        Self::Tasks,
        Self::Cats,
        Self::Hole,
        Self::Diplomacy,
        Self::Trade,
    ];

    pub const fn label_key(self) -> LocalizationKey {
        match self {
            Self::Plans => LocalizationKey("council.tab.plans"),
            Self::Tasks => LocalizationKey("council.tab.tasks"),
            Self::Cats => LocalizationKey("council.tab.cats"),
            Self::Hole => LocalizationKey("council.tab.hole"),
            Self::Diplomacy => LocalizationKey("council.tab.diplomacy"),
            Self::Trade => LocalizationKey("council.tab.trade"),
        }
    }

    pub const fn icon(self) -> SemanticPixelIcon {
        match self {
            Self::Plans => SemanticPixelIcon::PlanLedger,
            Self::Tasks => SemanticPixelIcon::TaskMarker,
            Self::Cats => SemanticPixelIcon::CatProfile,
            Self::Hole => SemanticPixelIcon::Hole,
            Self::Diplomacy => SemanticPixelIcon::VillageRadio,
            Self::Trade => SemanticPixelIcon::CargoExchange,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellRoute {
    World,
    Primary(PrimaryScreen),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShellRouter {
    route: ShellRoute,
    council_tab: CouncilTab,
}

impl Default for ShellRouter {
    fn default() -> Self {
        Self {
            route: ShellRoute::World,
            council_tab: CouncilTab::Plans,
        }
    }
}

impl ShellRouter {
    pub const fn route(self) -> ShellRoute {
        self.route
    }

    pub const fn visible_primary(self) -> Option<PrimaryScreen> {
        match self.route {
            ShellRoute::World => None,
            ShellRoute::Primary(screen) => Some(screen),
        }
    }

    pub const fn visible_primary_count(self) -> usize {
        self.visible_primary().is_some() as usize
    }

    pub fn open(&mut self, screen: PrimaryScreen) {
        self.route = ShellRoute::Primary(screen);
    }

    pub fn open_council(&mut self, tab: CouncilTab) {
        self.council_tab = tab;
        self.route = ShellRoute::Primary(PrimaryScreen::Council);
    }

    pub const fn council_tab(self) -> CouncilTab {
        self.council_tab
    }

    pub fn return_to_world(&mut self) {
        self.route = ShellRoute::World;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticPixelIcon {
    EventLog,
    Crate,
    Village,
    ResearchNotes,
    CouncilTable,
    PlanLedger,
    TaskMarker,
    CatProfile,
    Hole,
    VillageRadio,
    CargoExchange,
    CenterVillage,
    Connected,
    Connecting,
    Disconnected,
    Warning,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    LoadingSnapshot,
    Connected,
    Reconnecting,
    UpdateRequired,
    Error,
}

impl ConnectionState {
    pub const ALL: [Self; 7] = [
        Self::Disconnected,
        Self::Connecting,
        Self::LoadingSnapshot,
        Self::Connected,
        Self::Reconnecting,
        Self::UpdateRequired,
        Self::Error,
    ];

    pub const fn label_key(self) -> LocalizationKey {
        match self {
            Self::Disconnected => LocalizationKey("session.connection.disconnected"),
            Self::Connecting => LocalizationKey("session.connection.connecting"),
            Self::LoadingSnapshot => LocalizationKey("session.connection.loading_snapshot"),
            Self::Connected => LocalizationKey("session.connection.connected"),
            Self::Reconnecting => LocalizationKey("session.connection.reconnecting"),
            Self::UpdateRequired => LocalizationKey("session.connection.update_required"),
            Self::Error => LocalizationKey("session.connection.error"),
        }
    }

    pub const fn icon(self) -> SemanticPixelIcon {
        match self {
            Self::Connected => SemanticPixelIcon::Connected,
            Self::Connecting | Self::LoadingSnapshot | Self::Reconnecting => {
                SemanticPixelIcon::Connecting
            }
            Self::Disconnected => SemanticPixelIcon::Disconnected,
            Self::UpdateRequired | Self::Error => SemanticPixelIcon::Warning,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionStatus {
    pub connection: ConnectionState,
    pub session_label_key: LocalizationKey,
    pub authenticated_player_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopBarModel {
    pub center_village_label_key: LocalizationKey,
    pub center_village_icon: SemanticPixelIcon,
    pub selected_village_name: Option<String>,
    pub session: SessionStatus,
}

impl TopBarModel {
    pub fn new(connection: ConnectionState) -> Self {
        Self {
            center_village_label_key: LocalizationKey("shell.action.center_village"),
            center_village_icon: SemanticPixelIcon::CenterVillage,
            selected_village_name: None,
            session: SessionStatus {
                connection,
                session_label_key: LocalizationKey("shell.session.status"),
                authenticated_player_label: None,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceStack {
    pub transient_menu_open: bool,
    pub modal_open: bool,
    pub nested_detail_open: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EscapeOutcome {
    ClosedTransientMenu,
    ClosedModal,
    ClosedNestedDetail,
    ClosedPrimaryToWorld,
    AlreadyAtWorld,
}

/// The single Escape contract. Callers do not independently close surfaces.
pub fn handle_escape(stack: &mut SurfaceStack, router: &mut ShellRouter) -> EscapeOutcome {
    if stack.transient_menu_open {
        stack.transient_menu_open = false;
        EscapeOutcome::ClosedTransientMenu
    } else if stack.modal_open {
        stack.modal_open = false;
        EscapeOutcome::ClosedModal
    } else if stack.nested_detail_open {
        stack.nested_detail_open = false;
        EscapeOutcome::ClosedNestedDetail
    } else if router.visible_primary().is_some() {
        router.return_to_world();
        EscapeOutcome::ClosedPrimaryToWorld
    } else {
        EscapeOutcome::AlreadyAtWorld
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationActivation {
    ExplicitControl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimaryNavigationItem {
    pub screen: PrimaryScreen,
    pub label_key: LocalizationKey,
    pub icon: SemanticPixelIcon,
    pub activation: NavigationActivation,
}

pub const PRIMARY_NAVIGATION: [PrimaryNavigationItem; 5] = [
    PrimaryNavigationItem {
        screen: PrimaryScreen::Log,
        label_key: PrimaryScreen::Log.label_key(),
        icon: PrimaryScreen::Log.icon(),
        activation: NavigationActivation::ExplicitControl,
    },
    PrimaryNavigationItem {
        screen: PrimaryScreen::Stores,
        label_key: PrimaryScreen::Stores.label_key(),
        icon: PrimaryScreen::Stores.icon(),
        activation: NavigationActivation::ExplicitControl,
    },
    PrimaryNavigationItem {
        screen: PrimaryScreen::Village,
        label_key: PrimaryScreen::Village.label_key(),
        icon: PrimaryScreen::Village.icon(),
        activation: NavigationActivation::ExplicitControl,
    },
    PrimaryNavigationItem {
        screen: PrimaryScreen::Research,
        label_key: PrimaryScreen::Research.label_key(),
        icon: PrimaryScreen::Research.icon(),
        activation: NavigationActivation::ExplicitControl,
    },
    PrimaryNavigationItem {
        screen: PrimaryScreen::Council,
        label_key: PrimaryScreen::Council.label_key(),
        icon: PrimaryScreen::Council.icon(),
        activation: NavigationActivation::ExplicitControl,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceMaterial {
    DarkForestWorktable,
    Parchment,
    Wood,
    SolidPanel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabTreatment {
    Underline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconTreatment {
    SemanticPixel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisualLanguage {
    pub worktable: SurfaceMaterial,
    pub content: SurfaceMaterial,
    pub framing: SurfaceMaterial,
    pub feedback: SurfaceMaterial,
    pub tabs: TabTreatment,
    pub icons: IconTreatment,
}

impl VisualLanguage {
    pub const AUTHORED: Self = Self {
        worktable: SurfaceMaterial::DarkForestWorktable,
        content: SurfaceMaterial::Parchment,
        framing: SurfaceMaterial::Wood,
        feedback: SurfaceMaterial::SolidPanel,
        tabs: TabTreatment::Underline,
        icons: IconTreatment::SemanticPixel,
    };
}
