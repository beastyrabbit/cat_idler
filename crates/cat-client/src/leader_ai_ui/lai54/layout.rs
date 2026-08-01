//! Native/WASM layout contracts for the LAI.54 shell.
//!
//! The logical layout is derived only from viewport, platform, and one of the
//! three supported user scales. Phones are intentionally outside this model.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClientPlatform {
    Native,
    Wasm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Viewport {
    pub width_px: u16,
    pub height_px: u16,
}

impl Viewport {
    pub const fn new(width_px: u16, height_px: u16) -> Self {
        Self {
            width_px,
            height_px,
        }
    }
}

pub const SUPPORTED_VIEWPORTS: [Viewport; 5] = [
    Viewport::new(1_024, 768),
    Viewport::new(1_280, 800),
    Viewport::new(1_920, 1_080),
    Viewport::new(2_560, 1_440),
    Viewport::new(3_840, 2_160),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UiScale {
    Percent100,
    Percent115,
    Percent130,
}

impl UiScale {
    pub const ALL: [Self; 3] = [Self::Percent100, Self::Percent115, Self::Percent130];

    pub const fn percent(self) -> u16 {
        match self {
            Self::Percent100 => 100,
            Self::Percent115 => 115,
            Self::Percent130 => 130,
        }
    }

    pub const fn factor(self) -> f32 {
        self.percent() as f32 / 100.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutMode {
    Compact,
    Wide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharterPlacement {
    CenteredOverShowcase,
    BesideShowcase,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellLayout {
    pub platform: ClientPlatform,
    pub viewport: Viewport,
    pub ui_scale: UiScale,
    pub logical_width_px: f32,
    pub logical_height_px: f32,
    pub mode: LayoutMode,
    pub charter_placement: CharterPlacement,
    pub top_bar_height_px: u16,
    pub primary_nav_height_px: u16,
    pub content_gutter_px: u16,
    pub content_max_width_px: u16,
    pub minimum_scroll_viewport_height_px: u16,
}

impl ShellLayout {
    pub const fn preserves_vertical_scroll(self) -> bool {
        self.minimum_scroll_viewport_height_px >= 160
    }

    pub const fn keeps_top_bar_visible(self) -> bool {
        self.top_bar_height_px > 0 && self.primary_nav_height_px > 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    PhoneViewportOutOfScope,
}

pub fn shell_layout(
    platform: ClientPlatform,
    viewport: Viewport,
    ui_scale: UiScale,
) -> Result<ShellLayout, LayoutError> {
    if viewport.width_px < 1_024 || viewport.height_px < 768 {
        return Err(LayoutError::PhoneViewportOutOfScope);
    }

    let logical_width_px = f32::from(viewport.width_px) / ui_scale.factor();
    let logical_height_px = f32::from(viewport.height_px) / ui_scale.factor();
    let mode = if logical_width_px >= 1_200.0 && logical_height_px >= 680.0 {
        LayoutMode::Wide
    } else {
        LayoutMode::Compact
    };
    let charter_placement = match mode {
        LayoutMode::Compact => CharterPlacement::CenteredOverShowcase,
        LayoutMode::Wide => CharterPlacement::BesideShowcase,
    };

    Ok(ShellLayout {
        platform,
        viewport,
        ui_scale,
        logical_width_px,
        logical_height_px,
        mode,
        charter_placement,
        top_bar_height_px: 56,
        primary_nav_height_px: 48,
        content_gutter_px: if mode == LayoutMode::Wide { 24 } else { 12 },
        content_max_width_px: 1_920,
        minimum_scroll_viewport_height_px: 160,
    })
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FocusState {
    pub stable_control_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusRetention {
    Preserved,
    MovedToParent,
    Empty,
}

impl FocusState {
    pub fn focus(&mut self, stable_control_id: impl Into<String>) {
        self.stable_control_id = Some(stable_control_id.into());
    }

    pub fn retain_or_parent<'a>(
        &mut self,
        visible_control_ids: impl IntoIterator<Item = &'a str>,
        parent_control_id: &str,
    ) -> FocusRetention {
        let Some(active) = self.stable_control_id.as_deref() else {
            return FocusRetention::Empty;
        };
        if visible_control_ids
            .into_iter()
            .any(|candidate| candidate == active)
        {
            FocusRetention::Preserved
        } else {
            self.stable_control_id = Some(parent_control_id.to_owned());
            FocusRetention::MovedToParent
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollState {
    pub offset_px: f32,
    pub maximum_px: f32,
}

impl ScrollState {
    pub fn set_extent(&mut self, content_height_px: f32, viewport_height_px: f32) {
        self.maximum_px = (content_height_px - viewport_height_px).max(0.0);
        self.offset_px = self.offset_px.clamp(0.0, self.maximum_px);
    }

    pub fn scroll_by(&mut self, delta_px: f32) {
        self.offset_px = (self.offset_px + delta_px).clamp(0.0, self.maximum_px);
    }

    pub fn page_by(&mut self, direction: i8, viewport_height_px: f32) {
        self.scroll_by(f32::from(direction) * viewport_height_px * 0.9);
    }
}
