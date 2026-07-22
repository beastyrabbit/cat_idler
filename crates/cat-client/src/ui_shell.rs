//! Reusable layout, navigation, and scrolling contracts for the Bevy client UI.
//!
//! Top-level surfaces are deliberately small in number and strongly typed. Dynamic
//! content belongs in [`spawn_vertical_scroll_area`], never behind clipping that
//! makes the final rows unreachable.

use bevy::{
    ecs::{hierarchy::ChildOf, relationship::RelatedSpawnerCommands},
    picking::hover::Hovered,
    prelude::*,
    ui_widgets::{ControlOrientation, ScrollArea, Scrollbar, ScrollbarDragState, ScrollbarThumb},
};

pub(crate) const PRIMARY_SURFACE_Z: i32 = 82;
pub(crate) const CONTEXT_SURFACE_Z: i32 = 90;
pub(crate) const FEEDBACK_SURFACE_Z: i32 = 100;
pub(crate) const MODAL_SURFACE_Z: i32 = 900;
pub(crate) const START_SURFACE_Z: i32 = 1_000;

const _: () = {
    assert!(PRIMARY_SURFACE_Z < CONTEXT_SURFACE_Z);
    assert!(CONTEXT_SURFACE_Z < FEEDBACK_SURFACE_Z);
    assert!(FEEDBACK_SURFACE_Z < MODAL_SURFACE_Z);
    assert!(MODAL_SURFACE_Z < START_SURFACE_Z);
};

const SCROLLBAR_WIDTH: f32 = 10.0;
const SCROLLBAR_GAP: f32 = 3.0;
const SCROLLBAR_MIN_THUMB: f32 = 20.0;
const SCROLLBAR_TRACK: Color = Color::srgb(0.30, 0.22, 0.14);
const SCROLLBAR_THUMB: Color = Color::srgb(0.62, 0.43, 0.24);
const SCROLLBAR_THUMB_ACTIVE: Color = Color::srgb(0.78, 0.58, 0.32);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub(crate) enum PrimaryScreen {
    Log,
    Stores,
    Village,
    Research,
}

#[cfg(test)]
impl PrimaryScreen {
    pub(crate) const ALL: [Self; 4] = [Self::Log, Self::Stores, Self::Village, Self::Research];
}

#[derive(Resource, Default, Debug)]
pub(crate) struct UiRouter {
    primary: Option<PrimaryScreen>,
}

impl UiRouter {
    pub(crate) fn primary(&self) -> Option<PrimaryScreen> {
        self.primary
    }

    pub(crate) fn is_open(&self, screen: PrimaryScreen) -> bool {
        self.primary == Some(screen)
    }

    pub(crate) fn toggle(&mut self, screen: PrimaryScreen) {
        if self.is_open(screen) {
            self.close_primary();
        } else {
            self.open(screen);
        }
    }

    pub(crate) fn open(&mut self, screen: PrimaryScreen) {
        self.primary = Some(screen);
    }

    pub(crate) fn close_primary(&mut self) -> bool {
        self.primary.take().is_some()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum UiSurfaceKind {
    Hud,
    ContextPanel,
    PrimaryScreen,
    Modal,
    Tooltip,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct UiSurfaceRoot(pub(crate) UiSurfaceKind);

#[derive(Component, Debug)]
pub(crate) struct UiScrollViewport;

#[derive(Component, Debug)]
pub(crate) struct UiScrollbarTrack;

#[derive(Component, Debug, Default)]
pub(crate) struct UiScrollResetOnOpen {
    was_visible: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct UiLayoutProfile {
    pub(crate) effective_width: f32,
    pub(crate) effective_height: f32,
    pub(crate) compact: bool,
}

impl UiLayoutProfile {
    pub(crate) fn new(width: f32, height: f32, ui_scale: f32) -> Self {
        let scale = ui_scale.max(0.01);
        let effective_width = width / scale;
        let effective_height = height / scale;
        Self {
            effective_width,
            effective_height,
            compact: effective_width <= 1_100.0,
        }
    }
}

pub(crate) fn resolution_density_scale(width: f32, height: f32) -> f32 {
    if width >= 3_200.0 && height >= 1_800.0 {
        1.5
    } else if width >= 2_400.0 && height >= 1_300.0 {
        1.25
    } else {
        1.0
    }
}

pub(crate) fn effective_ui_scale(width: f32, height: f32, user_scale: f32) -> f32 {
    resolution_density_scale(width, height) * user_scale.clamp(1.0, 1.3)
}

pub(crate) fn primary_screen_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: Val::Percent(1.0),
        right: Val::Auto,
        top: Val::Px(60.0),
        bottom: Val::Px(10.0),
        width: Val::Percent(98.0),
        min_width: Val::Px(0.0),
        min_height: Val::Px(0.0),
        display: Display::None,
        border: UiRect::all(Val::Px(2.5)),
        flex_direction: FlexDirection::Column,
        overflow: Overflow::visible(),
        ..default()
    }
}

pub(crate) fn scroll_content_node(padding: f32, gap: f32) -> Node {
    Node {
        width: Val::Percent(100.0),
        min_width: Val::Px(0.0),
        min_height: Val::Px(0.0),
        padding: UiRect::all(Val::Px(padding)),
        flex_direction: FlexDirection::Column,
        flex_shrink: 0.0,
        row_gap: Val::Px(gap),
        ..default()
    }
}

fn scroll_viewport_node() -> Node {
    Node {
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        min_width: Val::Px(0.0),
        min_height: Val::Px(0.0),
        overflow: Overflow::scroll_y(),
        ..default()
    }
}

/// Spawn a vertically scrollable body with a draggable, automatically hidden
/// scrollbar. The returned entity is the actual scroll viewport.
pub(crate) fn spawn_vertical_scroll_area(
    parent: &mut RelatedSpawnerCommands<ChildOf>,
    padding: f32,
    gap: f32,
    build_content: impl FnOnce(&mut RelatedSpawnerCommands<ChildOf>),
) -> Entity {
    let mut viewport_id = Entity::PLACEHOLDER;
    parent
        .spawn(Node {
            display: Display::Grid,
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            min_width: Val::Px(0.0),
            min_height: Val::Px(0.0),
            grid_template_columns: vec![
                RepeatedGridTrack::flex(1, 1.0),
                RepeatedGridTrack::auto(1),
            ],
            column_gap: Val::Px(SCROLLBAR_GAP),
            overflow: Overflow::visible(),
            ..default()
        })
        .with_children(|frame| {
            viewport_id = frame
                .spawn((
                    scroll_viewport_node(),
                    ScrollArea,
                    ScrollPosition::default(),
                    Hovered::default(),
                    UiScrollViewport,
                    UiScrollResetOnOpen::default(),
                ))
                .with_children(|viewport| {
                    viewport
                        .spawn(scroll_content_node(padding, gap))
                        .with_children(build_content);
                })
                .id();

            frame.spawn((
                Node {
                    width: Val::Px(SCROLLBAR_WIDTH),
                    min_width: Val::Px(SCROLLBAR_WIDTH),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(SCROLLBAR_TRACK),
                Scrollbar {
                    orientation: ControlOrientation::Vertical,
                    target: viewport_id,
                    min_thumb_length: SCROLLBAR_MIN_THUMB,
                },
                UiScrollbarTrack,
                children![(
                    Hovered::default(),
                    BackgroundColor(SCROLLBAR_THUMB),
                    ScrollbarThumb {
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        border: UiRect::all(Val::Px(1.0)),
                    },
                )],
            ));
        });
    viewport_id
}

pub(crate) struct ClientUiShellPlugin;

impl Plugin for ClientUiShellPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiRouter>().add_systems(
            Update,
            (
                update_scrollbar_visibility,
                update_scrollbar_appearance,
                keyboard_scroll_areas,
                reset_scroll_on_open,
                validate_surface_contract,
            ),
        );
    }
}

/// Debug-only invariant check: primary screens must never stack. Keeping this
/// in the shared shell catches a future screen that bypasses [`UiRouter`].
fn validate_surface_contract(
    surfaces: Query<(&UiSurfaceRoot, &Node)>,
    mut reported_overlap: Local<bool>,
) {
    let visible_primary_count = surfaces
        .iter()
        .filter(|(surface, node)| {
            surface.0 == UiSurfaceKind::PrimaryScreen && node.display != Display::None
        })
        .count();
    if visible_primary_count > 1 && !*reported_overlap {
        warn!("UI surface contract violated: {visible_primary_count} primary screens are visible");
        *reported_overlap = true;
    } else if visible_primary_count <= 1 {
        *reported_overlap = false;
    }
}

fn update_scrollbar_visibility(
    viewports: Query<&ComputedNode, With<UiScrollViewport>>,
    mut tracks: Query<(&Scrollbar, &mut Node), With<UiScrollbarTrack>>,
) {
    for (scrollbar, mut track) in &mut tracks {
        let Ok(viewport) = viewports.get(scrollbar.target) else {
            continue;
        };
        let overflow = viewport.content_size().y > viewport.size().y + 0.5;
        track.display = if overflow {
            Display::Flex
        } else {
            Display::None
        };
    }
}

#[allow(clippy::type_complexity)]
fn update_scrollbar_appearance(
    mut thumbs: Query<
        (&Hovered, &ScrollbarDragState, &mut BackgroundColor),
        (
            With<ScrollbarThumb>,
            Or<(Changed<Hovered>, Changed<ScrollbarDragState>)>,
        ),
    >,
) {
    for (hovered, drag, mut color) in &mut thumbs {
        color.0 = if hovered.0 || drag.dragging {
            SCROLLBAR_THUMB_ACTIVE
        } else {
            SCROLLBAR_THUMB
        };
    }
}

fn keyboard_scroll_areas(
    keys: Res<ButtonInput<KeyCode>>,
    mut viewports: Query<(&Hovered, &ComputedNode, &mut ScrollPosition), With<UiScrollViewport>>,
) {
    if !keys.any_just_pressed([
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::Home,
        KeyCode::End,
    ]) {
        return;
    }
    let Some((_, computed, mut position)) = viewports.iter_mut().find(|(hovered, _, _)| hovered.0)
    else {
        return;
    };
    let viewport_height = computed.size().y * computed.inverse_scale_factor();
    let content_height = computed.content_size().y * computed.inverse_scale_factor();
    let max_y = (content_height - viewport_height).max(0.0);
    if keys.just_pressed(KeyCode::Home) {
        position.y = 0.0;
    } else if keys.just_pressed(KeyCode::End) {
        position.y = max_y;
    } else if keys.just_pressed(KeyCode::PageUp) {
        position.y = (position.y - viewport_height * 0.9).max(0.0);
    } else if keys.just_pressed(KeyCode::PageDown) {
        position.y = (position.y + viewport_height * 0.9).min(max_y);
    }
}

fn reset_scroll_on_open(
    mut viewports: Query<(&ComputedNode, &mut UiScrollResetOnOpen, &mut ScrollPosition)>,
) {
    for (computed, mut state, mut position) in &mut viewports {
        let visible = computed.size().x > 0.0 && computed.size().y > 0.0;
        if visible && !state.was_visible {
            position.0 = Vec2::ZERO;
        }
        state.was_visible = visible;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_test_scroll_area(mut commands: Commands) {
        commands.spawn(Node::default()).with_children(|root| {
            spawn_vertical_scroll_area(root, 12.0, 6.0, |content| {
                content.spawn(Node::default());
            });
        });
    }

    #[test]
    fn router_keeps_one_primary_screen_open() {
        let mut router = UiRouter::default();
        for screen in PrimaryScreen::ALL {
            router.open(screen);
            assert_eq!(router.primary(), Some(screen));
            assert!(router.is_open(screen));
        }
        router.toggle(PrimaryScreen::Research);
        assert_eq!(router.primary(), None);
    }

    #[test]
    fn supported_scale_matrix_uses_effective_dimensions() {
        for (width, height) in [(1_024.0, 768.0), (1_280.0, 800.0), (1_920.0, 1_080.0)] {
            for user_scale in [1.0, 1.15, 1.3] {
                let scale = effective_ui_scale(width, height, user_scale);
                let profile = UiLayoutProfile::new(width, height, scale);
                assert!(profile.effective_width >= 1_024.0 / 1.3);
                assert!(profile.effective_height >= 768.0 / 1.3);
            }
        }
        assert!((effective_ui_scale(2_560.0, 1_440.0, 1.0) - 1.25).abs() < f32::EPSILON);
        assert!((effective_ui_scale(3_840.0, 2_160.0, 1.3) - 1.95).abs() < 0.000_01);

        let compact = UiLayoutProfile::new(1_024.0, 768.0, 1.3);
        assert!(compact.compact);
        assert!(compact.effective_width > 780.0);
        assert!(compact.effective_height > 590.0);
    }

    #[test]
    fn primary_screen_is_viewport_constrained() {
        let node = primary_screen_node();
        assert_eq!(node.left, Val::Percent(1.0));
        assert_eq!(node.right, Val::Auto);
        assert_eq!(node.width, Val::Percent(98.0));
        assert_eq!(node.top, Val::Px(60.0));
        assert_eq!(node.bottom, Val::Px(10.0));
        assert_eq!(node.overflow, Overflow::visible());
        assert_eq!(node.min_height, Val::Px(0.0));
        assert_eq!(node.display, Display::None);
    }

    #[test]
    fn scroll_viewport_owns_clipping_and_can_shrink() {
        let viewport = scroll_viewport_node();
        assert_eq!(viewport.overflow, Overflow::scroll_y());
        assert_eq!(viewport.min_width, Val::Px(0.0));
        assert_eq!(viewport.min_height, Val::Px(0.0));

        let content = scroll_content_node(12.0, 6.0);
        assert_eq!(content.flex_shrink, 0.0);
        assert_eq!(content.padding, UiRect::all(Val::Px(12.0)));
    }

    #[test]
    fn scroll_helper_wires_bevy_widgets_to_the_same_viewport() {
        let mut app = App::new();
        app.add_systems(Startup, spawn_test_scroll_area);
        app.update();

        let mut viewports = app.world_mut().query_filtered::<
            (Entity, &Node, &ScrollPosition),
            (With<ScrollArea>, With<UiScrollViewport>),
        >();
        let (viewport, node, position) = viewports.single(app.world()).expect("one viewport");
        assert_eq!(node.overflow, Overflow::scroll_y());
        assert_eq!(position.0, Vec2::ZERO);

        let mut scrollbars = app
            .world_mut()
            .query_filtered::<&Scrollbar, With<UiScrollbarTrack>>();
        let scrollbar = scrollbars.single(app.world()).expect("one scrollbar");
        assert_eq!(scrollbar.target, viewport);
        assert_eq!(scrollbar.orientation, ControlOrientation::Vertical);
    }
}
