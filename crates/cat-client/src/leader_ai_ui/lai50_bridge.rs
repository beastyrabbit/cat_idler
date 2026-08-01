//! Small presentation-only route bridge for the three LAI.50 inspectors.
//!
//! The canonical snapshot feeds, selections, and authenticated actions remain
//! owned by their respective panels and transport. This bridge only supplies a
//! compact shell opener and keeps exactly one inspector visible at a time.

use accesskit::{Action, Role};
use bevy::a11y::ActionRequest as AccessibilityActionRequest;
use bevy::prelude::*;

use super::{
    lai50_food, lai50_hole_hunting, lai50_item_detail,
    lai54::bevy_shell::{Lai54LiveShell, Lai54PrimaryNavRoot, Lai54ShellRoot},
    lai66, lai68, semantic_node,
};

const INK: Color = Color::srgb(0.153, 0.106, 0.086);
const PARCHMENT: Color = Color::srgb(0.937, 0.886, 0.741);
const DARK_FOREST: Color = Color::srgb(0.090, 0.235, 0.180);
const WOOD: Color = Color::srgb(0.427, 0.282, 0.169);
const STONE: Color = Color::srgb(0.48, 0.46, 0.39);

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lai50PanelRoute {
    #[default]
    Hidden,
    HoleAndHunting,
    FoodAndCookhouse,
    ItemDetail,
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Lai50RouteMenuState {
    open: bool,
}

#[derive(Component)]
pub struct Lai50RouteMenuRoot;

#[derive(Component)]
pub struct Lai50RouteOpener;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lai50RouteAction {
    ToggleMenu,
    Open(Lai50PanelRoute),
    CloseAll,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
struct Lai50RouteControl {
    action: Lai50RouteAction,
}

#[derive(Default)]
pub struct Lai50RouteBridgePlugin;

impl Plugin for Lai50RouteBridgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Lai50PanelRoute>()
            .init_resource::<Lai50RouteMenuState>()
            .add_message::<AccessibilityActionRequest>()
            .add_systems(
                Update,
                (
                    attach_lai50_route_bridge,
                    handle_lai50_route_pointer,
                    handle_lai50_route_accessibility,
                    handle_lai50_route_keyboard,
                    handle_lai50_detail_events,
                    close_lai50_route_for_primary_screen,
                    sync_lai50_route_visibility,
                )
                    .chain(),
            );
    }
}

fn attach_lai50_route_bridge(
    mut commands: Commands<'_, '_>,
    navigation: Query<'_, '_, Entity, With<Lai54PrimaryNavRoot>>,
    shell: Query<'_, '_, Entity, With<Lai54ShellRoot>>,
    existing: Query<'_, '_, Entity, With<Lai50RouteOpener>>,
) {
    if !existing.is_empty() {
        return;
    }
    let (Ok(navigation), Ok(shell)) = (navigation.single(), shell.single()) else {
        return;
    };

    let opener = spawn_route_control(
        &mut commands,
        Lai50RouteAction::ToggleMenu,
        "Inspect",
        "lai50:route:open-menu",
        true,
    );
    commands.entity(opener).insert(Lai50RouteOpener);
    commands.entity(navigation).add_child(opener);

    let menu = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                top: Val::Px(72.0),
                width: Val::Px(280.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            GlobalZIndex(1_410),
            Visibility::Hidden,
            BackgroundColor(PARCHMENT),
            BorderColor::all(WOOD),
            Lai50RouteMenuRoot,
            semantic_node(
                Role::Menu,
                "lai50:route:menu",
                "Report inspector menu",
                true,
            ),
            Name::new("LAI.50 report inspector route menu"),
        ))
        .id();
    commands.entity(shell).add_child(menu);
    commands.entity(menu).with_children(|menu| {
        menu.spawn(text_bundle("Report inspectors", 16.0, INK));
    });
    for (action, label, stable_id) in [
        (
            Lai50RouteAction::Open(Lai50PanelRoute::HoleAndHunting),
            "Hole and Hunting",
            "lai50:route:hole-hunting",
        ),
        (
            Lai50RouteAction::Open(Lai50PanelRoute::FoodAndCookhouse),
            "Food and Cookhouse",
            "lai50:route:food-cookhouse",
        ),
        (
            Lai50RouteAction::Open(Lai50PanelRoute::ItemDetail),
            "Item detail",
            "lai50:route:item-detail",
        ),
        (
            Lai50RouteAction::CloseAll,
            "Close inspector",
            "lai50:route:close",
        ),
    ] {
        let control = spawn_route_control(&mut commands, action, label, stable_id, true);
        commands.entity(menu).add_child(control);
    }
}

fn spawn_route_control(
    commands: &mut Commands<'_, '_>,
    action: Lai50RouteAction,
    label: &'static str,
    stable_id: &'static str,
    enabled: bool,
) -> Entity {
    let control = commands
        .spawn((
            Button,
            Node {
                min_height: Val::Px(34.0),
                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(DARK_FOREST),
            BorderColor::all(STONE),
            semantic_node(Role::Button, stable_id, label, enabled),
            Lai50RouteControl { action },
            Name::new(format!("LAI.50 route {label}")),
        ))
        .id();
    commands.entity(control).with_children(|button| {
        button.spawn(text_bundle(label, 12.0, PARCHMENT));
    });
    control
}

fn handle_lai50_route_pointer(
    mut interactions: Query<'_, '_, (&Interaction, &Lai50RouteControl), Changed<Interaction>>,
    mut route: ResMut<'_, Lai50PanelRoute>,
    mut menu: ResMut<'_, Lai50RouteMenuState>,
    mut shell: Option<ResMut<'_, Lai54LiveShell>>,
) {
    for (interaction, control) in &mut interactions {
        if *interaction == Interaction::Pressed {
            apply_route_action(control.action, &mut route, &mut menu, shell.as_deref_mut());
        }
    }
}

fn handle_lai50_route_accessibility(
    mut requests: MessageReader<'_, '_, AccessibilityActionRequest>,
    controls: Query<'_, '_, &Lai50RouteControl>,
    mut route: ResMut<'_, Lai50PanelRoute>,
    mut menu: ResMut<'_, Lai50RouteMenuState>,
    mut shell: Option<ResMut<'_, Lai54LiveShell>>,
) {
    for request in requests.read() {
        if request.action != Action::Click {
            continue;
        }
        let Some(entity) = Entity::try_from_bits(request.target_node.0) else {
            continue;
        };
        let Ok(control) = controls.get(entity) else {
            continue;
        };
        apply_route_action(control.action, &mut route, &mut menu, shell.as_deref_mut());
    }
}

fn handle_lai50_route_keyboard(
    keys: Option<Res<'_, ButtonInput<KeyCode>>>,
    mut route: ResMut<'_, Lai50PanelRoute>,
    mut menu: ResMut<'_, Lai50RouteMenuState>,
    mut shell: Option<ResMut<'_, Lai54LiveShell>>,
) {
    let Some(keys) = keys else {
        return;
    };
    if keys.just_pressed(KeyCode::F6) {
        apply_route_action(
            Lai50RouteAction::ToggleMenu,
            &mut route,
            &mut menu,
            shell.as_deref_mut(),
        );
    }
    if menu.open {
        let action = if keys.just_pressed(KeyCode::Digit1) {
            Some(Lai50RouteAction::Open(Lai50PanelRoute::HoleAndHunting))
        } else if keys.just_pressed(KeyCode::Digit2) {
            Some(Lai50RouteAction::Open(Lai50PanelRoute::FoodAndCookhouse))
        } else if keys.just_pressed(KeyCode::Digit3) {
            Some(Lai50RouteAction::Open(Lai50PanelRoute::ItemDetail))
        } else {
            None
        };
        if let Some(action) = action {
            apply_route_action(action, &mut route, &mut menu, shell.as_deref_mut());
        }
    }
    if keys.just_pressed(KeyCode::Escape) && (menu.open || *route != Lai50PanelRoute::Hidden) {
        apply_route_action(
            Lai50RouteAction::CloseAll,
            &mut route,
            &mut menu,
            shell.as_deref_mut(),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_lai50_detail_events(
    mut store_selections: MessageReader<'_, '_, lai66::Lai66StoreDetailSelection>,
    mut world_selections: MessageReader<'_, '_, lai68::Lai68WorldDetailSelection>,
    mut close_requests: MessageReader<'_, '_, lai50_item_detail::Lai50ItemDetailCloseRequested>,
    mut view: ResMut<'_, lai50_item_detail::Lai50ViewState>,
    mut route: ResMut<'_, Lai50PanelRoute>,
    mut menu: ResMut<'_, Lai50RouteMenuState>,
    mut shell: Option<ResMut<'_, Lai54LiveShell>>,
) {
    let mut next = None;
    for selection in store_selections.read() {
        next = Some(match &selection.0 {
            lai66::Lai66StoreDetailTarget::ExactItem(id) => {
                lai50_item_detail::Lai50DetailSelection::ExactItem(id.clone())
            }
            lai66::Lai66StoreDetailTarget::BulkLot(id) => {
                lai50_item_detail::Lai50DetailSelection::BulkLot(id.clone())
            }
            lai66::Lai66StoreDetailTarget::RareMaterial(id) => {
                lai50_item_detail::Lai50DetailSelection::RareMaterial(id.clone())
            }
        });
    }
    for selection in world_selections.read() {
        next = Some(match &selection.0 {
            lai68::Lai68WorldDetailTarget::ExactItem(id) => {
                lai50_item_detail::Lai50DetailSelection::ExactItem(id.clone())
            }
            lai68::Lai68WorldDetailTarget::BulkLot(id) => {
                lai50_item_detail::Lai50DetailSelection::BulkLot(id.clone())
            }
            lai68::Lai68WorldDetailTarget::RareMaterial(id) => {
                lai50_item_detail::Lai50DetailSelection::RareMaterial(id.clone())
            }
        });
    }
    if let Some(selection) = next {
        view.selection = Some(selection);
        *route = Lai50PanelRoute::ItemDetail;
        menu.open = false;
        if let Some(shell) = shell.as_deref_mut() {
            shell.router.return_to_world();
        }
    }
    if close_requests.read().next().is_some() {
        *route = Lai50PanelRoute::Hidden;
        menu.open = false;
    }
}

fn apply_route_action(
    action: Lai50RouteAction,
    route: &mut Lai50PanelRoute,
    menu: &mut Lai50RouteMenuState,
    shell: Option<&mut Lai54LiveShell>,
) {
    match action {
        Lai50RouteAction::ToggleMenu => menu.open = !menu.open,
        Lai50RouteAction::Open(next) => {
            *route = next;
            menu.open = false;
            if let Some(shell) = shell {
                shell.router.return_to_world();
            }
        }
        Lai50RouteAction::CloseAll => {
            *route = Lai50PanelRoute::Hidden;
            menu.open = false;
        }
    }
}

fn close_lai50_route_for_primary_screen(
    shell: Option<Res<'_, Lai54LiveShell>>,
    mut route: ResMut<'_, Lai50PanelRoute>,
    mut menu: ResMut<'_, Lai50RouteMenuState>,
) {
    let Some(shell) = shell else {
        return;
    };
    if shell.router.visible_primary().is_some() && *route != Lai50PanelRoute::Hidden {
        *route = Lai50PanelRoute::Hidden;
        menu.open = false;
    }
}

#[allow(clippy::too_many_arguments)]
fn sync_lai50_route_visibility(
    route: Res<'_, Lai50PanelRoute>,
    menu: Res<'_, Lai50RouteMenuState>,
    mut menu_root: Query<'_, '_, &mut Visibility, With<Lai50RouteMenuRoot>>,
    mut hole: ResMut<'_, lai50_hole_hunting::Lai50PanelVisibility>,
    mut food: ResMut<'_, lai50_food::Lai50PanelVisibility>,
    mut item: ResMut<'_, lai50_item_detail::Lai50PanelVisibility>,
) {
    for mut visibility in &mut menu_root {
        *visibility = if menu.open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let hole_visible = *route == Lai50PanelRoute::HoleAndHunting;
    let food_visible = *route == Lai50PanelRoute::FoodAndCookhouse;
    let item_visible = *route == Lai50PanelRoute::ItemDetail;
    if hole.visible != hole_visible {
        hole.visible = hole_visible;
    }
    if food.visible != food_visible {
        food.visible = food_visible;
    }
    if item.visible != item_visible {
        item.visible = item_visible;
    }
}

fn text_bundle(value: impl Into<String>, font_size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font_size: FontSize::Px(font_size),
            ..default()
        },
        TextColor(color),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_defaults_to_all_panels_hidden() {
        assert_eq!(Lai50PanelRoute::default(), Lai50PanelRoute::Hidden);
    }
}
