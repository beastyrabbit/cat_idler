use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoleColor {
    Paper,
    PaperDark,
    Wood,
    Stone,
    Olive,
    Rust,
    Ink,
    Muted,
    Danger,
    Water,
    FocusRing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorToken {
    pub name: &'static str,
    pub role: RoleColor,
    pub color: Color,
}

impl ColorToken {
    pub const fn new(name: &'static str, role: RoleColor, color: Color) -> Self {
        Self { name, role, color }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpacingScale {
    pub xxs: u16,
    pub xs: u16,
    pub sm: u16,
    pub md: u16,
    pub lg: u16,
    pub xl: u16,
}

impl Default for SpacingScale {
    fn default() -> Self {
        Self {
            xxs: 4,
            xs: 8,
            sm: 12,
            md: 16,
            lg: 24,
            xl: 32,
        }
    }
}

impl SpacingScale {
    pub const fn steps(self) -> [u16; 6] {
        [self.xxs, self.xs, self.sm, self.md, self.lg, self.xl]
    }

    pub const fn is_exact_lai_scale(self) -> bool {
        matches!(self.steps(), [4, 8, 12, 16, 24, 32])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeometryScale {
    pub panel_radius_px: u16,
    pub button_radius_px: u16,
    pub panel_border_px: u16,
    pub button_height_px: u16,
}

impl Default for GeometryScale {
    fn default() -> Self {
        Self {
            panel_radius_px: 10,
            button_radius_px: 8,
            panel_border_px: 1,
            button_height_px: 32,
        }
    }
}

impl GeometryScale {
    pub const fn uses_restrained_geometry(self) -> bool {
        self.panel_radius_px >= 8
            && self.panel_radius_px <= 12
            && self.button_radius_px >= 8
            && self.button_radius_px <= 12
            && self.panel_border_px >= 1
            && self.panel_border_px <= 2
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionDuration {
    pub fast_ms: u16,
    pub standard_ms: u16,
    pub slow_ms: u16,
}

impl Default for MotionDuration {
    fn default() -> Self {
        Self {
            fast_ms: 100,
            standard_ms: 150,
            slow_ms: 200,
        }
    }
}

impl MotionDuration {
    pub const fn all_within_lai_range(self) -> bool {
        self.fast_ms >= 100
            && self.fast_ms <= 200
            && self.standard_ms >= 100
            && self.standard_ms <= 200
            && self.slow_ms >= 100
            && self.slow_ms <= 200
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeedbackState {
    Loading,
    Empty,
    Stale,
    UpdateRequired,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateStyle {
    pub state: FeedbackState,
    pub fill: RoleColor,
    pub border: RoleColor,
    pub text: RoleColor,
    pub blocks_mutation: bool,
}

impl StateStyle {
    pub const fn new(
        state: FeedbackState,
        fill: RoleColor,
        border: RoleColor,
        text: RoleColor,
        blocks_mutation: bool,
    ) -> Self {
        Self {
            state,
            fill,
            border,
            text,
            blocks_mutation,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForbiddenPattern {
    Glass,
    Glow,
    Hero,
    KpiGrid,
    Pill,
    FakeChart,
    Gradient,
    Blur,
    DecorativeOnly,
}

impl ForbiddenPattern {
    pub const fn needle(self) -> &'static str {
        match self {
            Self::Glass => "glass",
            Self::Glow => "glow",
            Self::Hero => "hero",
            Self::KpiGrid => "kpi",
            Self::Pill => "pill",
            Self::FakeChart => "chart",
            Self::Gradient => "gradient",
            Self::Blur => "blur",
            Self::DecorativeOnly => "decorative",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StyleValidationError {
    MissingRole(RoleColor),
    ForbiddenToken {
        token: String,
        pattern: ForbiddenPattern,
    },
    InvalidSpacing([u16; 6]),
    InvalidGeometry(GeometryScale),
    InvalidMotion(MotionDuration),
    MissingState(FeedbackState),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeaderAiUiTheme {
    pub colors: Vec<ColorToken>,
    pub spacing: SpacingScale,
    pub geometry: GeometryScale,
    pub motion: MotionDuration,
    pub states: Vec<StateStyle>,
}

impl Default for LeaderAiUiTheme {
    fn default() -> Self {
        Self {
            colors: vec![
                ColorToken::new("lai-paper", RoleColor::Paper, Color::srgb(0.86, 0.79, 0.63)),
                ColorToken::new(
                    "lai-paper-dark",
                    RoleColor::PaperDark,
                    Color::srgb(0.34, 0.22, 0.13),
                ),
                ColorToken::new("lai-wood", RoleColor::Wood, Color::srgb(0.46, 0.29, 0.16)),
                ColorToken::new("lai-stone", RoleColor::Stone, Color::srgb(0.48, 0.46, 0.39)),
                ColorToken::new("lai-olive", RoleColor::Olive, Color::srgb(0.34, 0.43, 0.22)),
                ColorToken::new("lai-rust", RoleColor::Rust, Color::srgb(0.62, 0.31, 0.15)),
                ColorToken::new("lai-ink", RoleColor::Ink, Color::srgb(0.16, 0.10, 0.06)),
                ColorToken::new("lai-muted", RoleColor::Muted, Color::srgb(0.44, 0.36, 0.27)),
                ColorToken::new(
                    "lai-danger",
                    RoleColor::Danger,
                    Color::srgb(0.58, 0.20, 0.18),
                ),
                ColorToken::new("lai-water", RoleColor::Water, Color::srgb(0.24, 0.42, 0.50)),
                ColorToken::new(
                    "lai-focus-ring",
                    RoleColor::FocusRing,
                    Color::srgb(0.86, 0.60, 0.28),
                ),
            ],
            spacing: SpacingScale::default(),
            geometry: GeometryScale::default(),
            motion: MotionDuration::default(),
            states: vec![
                StateStyle::new(
                    FeedbackState::Loading,
                    RoleColor::Paper,
                    RoleColor::Stone,
                    RoleColor::Ink,
                    true,
                ),
                StateStyle::new(
                    FeedbackState::Empty,
                    RoleColor::Paper,
                    RoleColor::Wood,
                    RoleColor::Muted,
                    false,
                ),
                StateStyle::new(
                    FeedbackState::Stale,
                    RoleColor::Paper,
                    RoleColor::Rust,
                    RoleColor::Ink,
                    true,
                ),
                StateStyle::new(
                    FeedbackState::UpdateRequired,
                    RoleColor::PaperDark,
                    RoleColor::Rust,
                    RoleColor::Paper,
                    true,
                ),
                StateStyle::new(
                    FeedbackState::Error,
                    RoleColor::Paper,
                    RoleColor::Danger,
                    RoleColor::Danger,
                    true,
                ),
            ],
        }
    }
}

impl LeaderAiUiTheme {
    pub fn color_for(&self, role: RoleColor) -> Option<Color> {
        self.colors
            .iter()
            .find(|token| token.role == role)
            .map(|token| token.color)
    }

    pub fn state_style(&self, state: FeedbackState) -> Option<StateStyle> {
        self.states
            .iter()
            .copied()
            .find(|style| style.state == state)
    }
}

pub fn validate_product_normal_tokens(theme: &LeaderAiUiTheme) -> Result<(), StyleValidationError> {
    for role in [
        RoleColor::Paper,
        RoleColor::PaperDark,
        RoleColor::Wood,
        RoleColor::Stone,
        RoleColor::Olive,
        RoleColor::Rust,
        RoleColor::Ink,
        RoleColor::Muted,
        RoleColor::Danger,
        RoleColor::Water,
        RoleColor::FocusRing,
    ] {
        if theme.color_for(role).is_none() {
            return Err(StyleValidationError::MissingRole(role));
        }
    }

    for token in &theme.colors {
        let name = token.name.to_ascii_lowercase();
        for pattern in [
            ForbiddenPattern::Glass,
            ForbiddenPattern::Glow,
            ForbiddenPattern::Hero,
            ForbiddenPattern::KpiGrid,
            ForbiddenPattern::Pill,
            ForbiddenPattern::FakeChart,
            ForbiddenPattern::Gradient,
            ForbiddenPattern::Blur,
            ForbiddenPattern::DecorativeOnly,
        ] {
            if name.contains(pattern.needle()) {
                return Err(StyleValidationError::ForbiddenToken {
                    token: token.name.to_string(),
                    pattern,
                });
            }
        }
    }

    if !theme.spacing.is_exact_lai_scale() {
        return Err(StyleValidationError::InvalidSpacing(theme.spacing.steps()));
    }
    if !theme.geometry.uses_restrained_geometry() {
        return Err(StyleValidationError::InvalidGeometry(theme.geometry));
    }
    if !theme.motion.all_within_lai_range() {
        return Err(StyleValidationError::InvalidMotion(theme.motion));
    }
    for state in [
        FeedbackState::Loading,
        FeedbackState::Empty,
        FeedbackState::Stale,
        FeedbackState::UpdateRequired,
        FeedbackState::Error,
    ] {
        if theme.state_style(state).is_none() {
            return Err(StyleValidationError::MissingState(state));
        }
    }

    Ok(())
}
