#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewportSize {
    pub width_px: u16,
    pub height_px: u16,
    pub is_wasm: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponsiveClass {
    Compact,
    Wide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResponsiveDecision {
    pub class: ResponsiveClass,
    pub council_width_px: u16,
    pub council_max_height_px: u16,
    pub right_inspector_width_px: u16,
    pub bottom_dock_height_px: u16,
    pub keeps_world_primary: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResponsivePolicy {
    pub compact_max_width_px: u16,
    pub compact_council_margin_px: u16,
    pub wide_council_width_px: u16,
    pub compact_sheet_max_height_px: u16,
    pub right_inspector_width_px: u16,
    pub bottom_dock_height_px: u16,
    pub min_world_short_side_px: u16,
}

impl Default for ResponsivePolicy {
    fn default() -> Self {
        Self {
            compact_max_width_px: 1099,
            compact_council_margin_px: 16,
            wide_council_width_px: 820,
            compact_sheet_max_height_px: 360,
            right_inspector_width_px: 320,
            bottom_dock_height_px: 96,
            min_world_short_side_px: 300,
        }
    }
}

impl ResponsivePolicy {
    pub fn classify(self, viewport: ViewportSize) -> ResponsiveClass {
        if viewport.width_px <= self.compact_max_width_px {
            ResponsiveClass::Compact
        } else {
            ResponsiveClass::Wide
        }
    }

    pub fn decide(self, viewport: ViewportSize) -> ResponsiveDecision {
        match self.classify(viewport) {
            ResponsiveClass::Compact => {
                let margins = self.compact_council_margin_px.saturating_mul(2);
                ResponsiveDecision {
                    class: ResponsiveClass::Compact,
                    council_width_px: viewport.width_px.saturating_sub(margins),
                    council_max_height_px: self.compact_sheet_max_height_px.min(
                        viewport
                            .height_px
                            .saturating_sub(self.bottom_dock_height_px)
                            .saturating_sub(self.compact_council_margin_px),
                    ),
                    right_inspector_width_px: 0,
                    bottom_dock_height_px: self.bottom_dock_height_px,
                    keeps_world_primary: viewport.height_px >= self.min_world_short_side_px,
                }
            }
            ResponsiveClass::Wide => ResponsiveDecision {
                class: ResponsiveClass::Wide,
                council_width_px: self.wide_council_width_px,
                council_max_height_px: viewport
                    .height_px
                    .saturating_sub(self.bottom_dock_height_px)
                    .saturating_sub(32),
                right_inspector_width_px: self.right_inspector_width_px,
                bottom_dock_height_px: self.bottom_dock_height_px,
                keeps_world_primary: viewport.width_px
                    >= self
                        .wide_council_width_px
                        .saturating_add(self.right_inspector_width_px)
                        .saturating_add(self.min_world_short_side_px),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum OverlayLayer {
    WorldMarkers,
    WorldTooltip,
    LeftStatus,
    Council,
    RightInspector,
    Modal,
    Toast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayBand {
    World,
    Interface,
    Blocking,
}

impl OverlayLayer {
    pub const fn z_index(self) -> i32 {
        match self {
            Self::WorldMarkers => 30,
            Self::WorldTooltip => 50,
            Self::LeftStatus => 70,
            Self::Council => 80,
            Self::RightInspector => 90,
            Self::Modal => 120,
            Self::Toast => 140,
        }
    }

    pub const fn band(self) -> OverlayBand {
        match self {
            Self::WorldMarkers | Self::WorldTooltip => OverlayBand::World,
            Self::LeftStatus | Self::Council | Self::RightInspector | Self::Toast => {
                OverlayBand::Interface
            }
            Self::Modal => OverlayBand::Blocking,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusKey {
    pub test_id: String,
}

impl FocusKey {
    pub fn new(test_id: impl Into<String>) -> Self {
        Self {
            test_id: test_id.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusRetention {
    Preserved,
    Cleared,
    UnchangedEmpty,
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct FocusMemory {
    pub active: Option<FocusKey>,
}

impl FocusMemory {
    pub fn remember(&mut self, key: FocusKey) {
        self.active = Some(key);
    }

    pub fn clear(&mut self) {
        self.active = None;
    }

    pub fn preserve_after_refresh<'a>(
        &mut self,
        visible_test_ids: impl IntoIterator<Item = &'a str>,
    ) -> FocusRetention {
        let Some(active) = &self.active else {
            return FocusRetention::UnchangedEmpty;
        };
        if visible_test_ids
            .into_iter()
            .any(|test_id| test_id == active.test_id)
        {
            FocusRetention::Preserved
        } else {
            self.active = None;
            FocusRetention::Cleared
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputBlockerState {
    pub modal_open: bool,
    pub text_input_active: bool,
    pub pointer_over_blocking_ui: bool,
    pub dragging_ui: bool,
}

impl InputBlockerState {
    pub const fn blocks_world_input(self) -> bool {
        self.modal_open
            || self.text_input_active
            || self.pointer_over_blocking_ui
            || self.dragging_ui
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldInputPolicy {
    pub preserve_focus_on_stale_refresh: bool,
    pub block_world_when_ui_captures_input: bool,
}

impl Default for WorldInputPolicy {
    fn default() -> Self {
        Self {
            preserve_focus_on_stale_refresh: true,
            block_world_when_ui_captures_input: true,
        }
    }
}

impl WorldInputPolicy {
    pub const fn allows_world_input(self, blockers: InputBlockerState) -> bool {
        !(self.block_world_when_ui_captures_input && blockers.blocks_world_input())
    }
}
