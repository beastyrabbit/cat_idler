//! Interaction bridge for report-safe leader-AI controls.
//!
//! Buttons carry already validated LAI.25 envelopes. This layer only queues
//! them, records pending idempotency, and reconciles typed server feedback; it
//! never predicts a simulation result.

use bevy::a11y::ActionRequest as AccessibilityActionRequest;
use bevy::ecs::system::SystemParam;
use bevy::input::{
    ButtonState,
    keyboard::{KeyCode, KeyboardInput},
};
use bevy::prelude::*;
use bevy_input_focus::{FocusCause, InputFocus, InputFocusVisible};
use cat_protocol::LeaderAiActionEnvelope;

use super::LeaderAiSemanticNode;
use crate::{LeaderAiFeedback, LeaderAiLiveState, queue_authenticated_leader_ai_action};

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct LeaderAiActionButton {
    pub envelope: LeaderAiActionEnvelope,
    pub label: String,
    pub test_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaderAiSelectionKind {
    Task,
    Cat,
    Plan,
    ProgressionRow,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct LeaderAiSelectionButton {
    pub kind: LeaderAiSelectionKind,
    pub stable_id: String,
    pub label: String,
    pub test_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaderAiLocalAction {
    Reload,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct LeaderAiLocalButton {
    pub action: LeaderAiLocalAction,
    pub label: String,
    pub test_id: String,
}

#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct LeaderAiInteractionState {
    pub focused_test_id: Option<String>,
    pub selected_task_id: Option<String>,
    pub selected_cat_id: Option<String>,
    pub selected_plan_id: Option<String>,
    pub selected_progression_row_id: Option<String>,
    pub pending_idempotency_ids: Vec<String>,
    pub stale_refresh_required: bool,
    pub reload_requests: u32,
    pub last_error: Option<String>,
}

#[derive(SystemParam)]
struct LeaderAiControlQueries<'w, 's> {
    actions: Query<'w, 's, &'static LeaderAiActionButton>,
    selections: Query<'w, 's, &'static LeaderAiSelectionButton>,
    local_actions: Query<'w, 's, &'static LeaderAiLocalButton>,
}

type LeaderAiFocusableItem = (
    Entity,
    &'static LeaderAiSemanticNode,
    Option<&'static LeaderAiActionButton>,
    Option<&'static LeaderAiSelectionButton>,
    Option<&'static LeaderAiLocalButton>,
);

#[derive(SystemParam)]
struct LeaderAiFocusableQuery<'w, 's> {
    semantics: Query<'w, 's, LeaderAiFocusableItem>,
}

#[derive(SystemParam)]
struct LeaderAiInteractionContext<'w> {
    live: ResMut<'w, LeaderAiLiveState>,
    state: ResMut<'w, LeaderAiInteractionState>,
    focus: ResMut<'w, InputFocus>,
    focus_visible: ResMut<'w, InputFocusVisible>,
}

#[derive(Default)]
pub struct LeaderAiInteractionPlugin;

impl Plugin for LeaderAiInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LeaderAiInteractionState>()
            .init_resource::<InputFocus>()
            .init_resource::<InputFocusVisible>()
            .add_message::<AccessibilityActionRequest>()
            .add_message::<KeyboardInput>()
            .add_systems(
                Update,
                (
                    dispatch_pressed_leader_ai_actions,
                    dispatch_pressed_leader_ai_selections,
                    dispatch_pressed_local_actions,
                    dispatch_accessibility_actions,
                    dispatch_keyboard_input,
                    reconcile_leader_ai_feedback,
                )
                    .chain(),
            );
    }
}

fn dispatch_pressed_leader_ai_actions(
    mut interactions: Query<
        '_,
        '_,
        (Entity, &Interaction, &LeaderAiActionButton),
        Changed<Interaction>,
    >,
    mut live: ResMut<'_, LeaderAiLiveState>,
    mut state: ResMut<'_, LeaderAiInteractionState>,
    mut focus: ResMut<'_, InputFocus>,
) {
    for (entity, interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        focus.set(entity, FocusCause::Pressed);
        state.focused_test_id = Some(button.test_id.clone());
        dispatch_action(button, &mut live, &mut state);
    }
}

fn dispatch_accessibility_actions(
    mut requests: MessageReader<'_, '_, AccessibilityActionRequest>,
    controls: LeaderAiControlQueries<'_, '_>,
    semantics: Query<'_, '_, &LeaderAiSemanticNode>,
    mut context: LeaderAiInteractionContext<'_>,
) {
    for request in requests.read() {
        let Some(entity) = Entity::try_from_bits(request.target_node.0) else {
            continue;
        };
        match request.action {
            accesskit::Action::Focus => {
                let Ok(semantic) = semantics.get(entity) else {
                    continue;
                };
                if semantic.enabled {
                    context.focus.set(entity, FocusCause::Navigated);
                    context.state.focused_test_id = Some(semantic.semantic_id.clone());
                }
            }
            accesskit::Action::Click
                if semantics.get(entity).is_ok_and(|semantic| semantic.enabled) =>
            {
                context.focus.set(entity, FocusCause::Navigated);
                dispatch_entity(entity, &controls, &mut context.live, &mut context.state);
            }
            _ => {}
        }
    }
}

fn dispatch_keyboard_input(
    mut keyboard: MessageReader<'_, '_, KeyboardInput>,
    keys: Option<Res<'_, ButtonInput<KeyCode>>>,
    focusables: LeaderAiFocusableQuery<'_, '_>,
    controls: LeaderAiControlQueries<'_, '_>,
    mut context: LeaderAiInteractionContext<'_>,
) {
    let mut focusables = focusables
        .semantics
        .iter()
        .filter(|(_, semantic, action, selection, local)| {
            semantic.enabled && (action.is_some() || selection.is_some() || local.is_some())
        })
        .map(|(entity, semantic, _, _, _)| {
            (semantic.focus_order, semantic.semantic_id.as_str(), entity)
        })
        .collect::<Vec<_>>();
    focusables.sort_unstable_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.cmp(right.1))
            .then_with(|| left.2.to_bits().cmp(&right.2.to_bits()))
    });

    for input in keyboard
        .read()
        .filter(|input| input.state == ButtonState::Pressed)
    {
        match input.key_code {
            KeyCode::Tab => {
                let reverse = keys.as_ref().is_some_and(|keys| {
                    keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)
                });
                move_focus(
                    &focusables,
                    &mut context.focus,
                    &mut context.focus_visible,
                    &mut context.state,
                    reverse,
                    false,
                );
            }
            KeyCode::ArrowDown | KeyCode::ArrowRight => move_focus(
                &focusables,
                &mut context.focus,
                &mut context.focus_visible,
                &mut context.state,
                false,
                false,
            ),
            KeyCode::ArrowUp | KeyCode::ArrowLeft => move_focus(
                &focusables,
                &mut context.focus,
                &mut context.focus_visible,
                &mut context.state,
                true,
                false,
            ),
            KeyCode::Home => move_focus(
                &focusables,
                &mut context.focus,
                &mut context.focus_visible,
                &mut context.state,
                false,
                true,
            ),
            KeyCode::End => move_focus(
                &focusables,
                &mut context.focus,
                &mut context.focus_visible,
                &mut context.state,
                true,
                true,
            ),
            KeyCode::Escape => {
                context.focus.clear();
                context.focus_visible.0 = false;
                context.state.focused_test_id = None;
            }
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                if let Some(entity) = context.focus.get() {
                    dispatch_entity(entity, &controls, &mut context.live, &mut context.state);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
fn is_activation_key(input: &KeyboardInput) -> bool {
    input.state == ButtonState::Pressed
        && matches!(
            input.key_code,
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space
        )
}

fn dispatch_action(
    button: &LeaderAiActionButton,
    live: &mut LeaderAiLiveState,
    state: &mut LeaderAiInteractionState,
) {
    state.focused_test_id = Some(button.test_id.clone());
    match queue_authenticated_leader_ai_action(live, button.envelope.clone()) {
        Ok(()) => {
            let id = button.envelope.idempotency_id.as_str().to_owned();
            if !state.pending_idempotency_ids.contains(&id) {
                state.pending_idempotency_ids.push(id);
            }
            state.last_error = None;
        }
        Err(error) => state.last_error = Some(error.to_owned()),
    }
}

fn dispatch_pressed_leader_ai_selections(
    mut interactions: Query<
        '_,
        '_,
        (Entity, &Interaction, &LeaderAiSelectionButton),
        Changed<Interaction>,
    >,
    mut state: ResMut<'_, LeaderAiInteractionState>,
    mut focus: ResMut<'_, InputFocus>,
) {
    for (entity, interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        focus.set(entity, FocusCause::Pressed);
        dispatch_selection(button, &mut state);
    }
}

fn dispatch_pressed_local_actions(
    mut interactions: Query<
        '_,
        '_,
        (Entity, &Interaction, &LeaderAiLocalButton),
        Changed<Interaction>,
    >,
    mut state: ResMut<'_, LeaderAiInteractionState>,
    mut focus: ResMut<'_, InputFocus>,
) {
    for (entity, interaction, button) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        focus.set(entity, FocusCause::Pressed);
        dispatch_local_action(button, &mut state);
    }
}

fn dispatch_entity(
    entity: Entity,
    controls: &LeaderAiControlQueries<'_, '_>,
    live: &mut LeaderAiLiveState,
    state: &mut LeaderAiInteractionState,
) {
    if let Ok(button) = controls.actions.get(entity) {
        dispatch_action(button, live, state);
    } else if let Ok(button) = controls.selections.get(entity) {
        dispatch_selection(button, state);
    } else if let Ok(button) = controls.local_actions.get(entity) {
        dispatch_local_action(button, state);
    }
}

fn dispatch_selection(button: &LeaderAiSelectionButton, state: &mut LeaderAiInteractionState) {
    state.focused_test_id = Some(button.test_id.clone());
    match button.kind {
        LeaderAiSelectionKind::Task => state.selected_task_id = Some(button.stable_id.clone()),
        LeaderAiSelectionKind::Cat => state.selected_cat_id = Some(button.stable_id.clone()),
        LeaderAiSelectionKind::Plan => state.selected_plan_id = Some(button.stable_id.clone()),
        LeaderAiSelectionKind::ProgressionRow => {
            state.selected_progression_row_id = Some(button.stable_id.clone())
        }
    }
}

fn dispatch_local_action(button: &LeaderAiLocalButton, state: &mut LeaderAiInteractionState) {
    state.focused_test_id = Some(button.test_id.clone());
    match button.action {
        LeaderAiLocalAction::Reload => {
            state.reload_requests = state.reload_requests.saturating_add(1);
            #[cfg(target_arch = "wasm32")]
            if let Some(window) = web_sys::window() {
                let _ = window.location().reload();
            }
        }
    }
}

fn move_focus(
    focusables: &[(u32, &str, Entity)],
    focus: &mut InputFocus,
    focus_visible: &mut InputFocusVisible,
    state: &mut LeaderAiInteractionState,
    reverse: bool,
    boundary: bool,
) {
    let Some((_, semantic_id, entity)) = next_focus(focusables, focus.get(), reverse, boundary)
    else {
        return;
    };
    focus.set(entity, FocusCause::Navigated);
    focus_visible.0 = true;
    state.focused_test_id = Some(semantic_id.to_owned());
}

fn next_focus<'a>(
    focusables: &'a [(u32, &'a str, Entity)],
    current: Option<Entity>,
    reverse: bool,
    boundary: bool,
) -> Option<(u32, &'a str, Entity)> {
    if focusables.is_empty() {
        return None;
    }
    if boundary {
        return if reverse {
            focusables.last().copied()
        } else {
            focusables.first().copied()
        };
    }
    let current_index = focusables
        .iter()
        .position(|(_, _, entity)| Some(*entity) == current);
    let next_index = match (current_index, reverse) {
        (Some(index), false) => (index + 1) % focusables.len(),
        (Some(0), true) | (None, true) => focusables.len() - 1,
        (Some(index), true) => index - 1,
        (None, false) => 0,
    };
    focusables.get(next_index).copied()
}

fn reconcile_leader_ai_feedback(
    live: Res<'_, LeaderAiLiveState>,
    mut state: ResMut<'_, LeaderAiInteractionState>,
) {
    for feedback in live.feedback.iter() {
        match feedback {
            LeaderAiFeedback::Accepted { idempotency_id, .. }
            | LeaderAiFeedback::Rejected { idempotency_id, .. }
            | LeaderAiFeedback::Duplicate { idempotency_id, .. } => {
                state
                    .pending_idempotency_ids
                    .retain(|id| id != idempotency_id);
            }
            LeaderAiFeedback::UpdateRequired | LeaderAiFeedback::Reconnecting => {
                state.stale_refresh_required = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::keyboard::Key;

    fn input(key_code: KeyCode, state: ButtonState) -> KeyboardInput {
        KeyboardInput {
            key_code,
            logical_key: Key::Character("".into()),
            state,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }

    #[test]
    fn keyboard_activation_is_limited_to_enter_and_space_press() {
        assert!(is_activation_key(&input(
            KeyCode::Enter,
            ButtonState::Pressed
        )));
        assert!(is_activation_key(&input(
            KeyCode::Space,
            ButtonState::Pressed
        )));
        assert!(is_activation_key(&input(
            KeyCode::NumpadEnter,
            ButtonState::Pressed
        )));
        assert!(!is_activation_key(&input(
            KeyCode::Enter,
            ButtonState::Released
        )));
        assert!(!is_activation_key(&input(
            KeyCode::Tab,
            ButtonState::Pressed
        )));
    }

    #[test]
    fn focus_navigation_is_stable_and_wraps() {
        let first = Entity::from_bits(1);
        let second = Entity::from_bits(2);
        let focusables = [(10, "first", first), (20, "second", second)];
        assert_eq!(
            next_focus(&focusables, None, false, false),
            Some((10, "first", first))
        );
        assert_eq!(
            next_focus(&focusables, Some(first), true, false),
            Some((20, "second", second))
        );
        assert_eq!(
            next_focus(&focusables, Some(second), false, false),
            Some((10, "first", first))
        );
    }
}
