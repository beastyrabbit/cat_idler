//! Stable report-safe UI identifiers plus the production AccessKit bridge.

use accesskit::{Action, Live, Node, Role};
use bevy::a11y::AccessibilityNode;
use bevy::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StableUiId(String);

impl StableUiId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UiSection {
    Plans,
    Cats,
    Progression,
    StandingOrders,
    Officers,
    Tasks,
    Diplomacy,
    Trade,
    Connection,
}

impl UiSection {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Plans => "plans",
            Self::Cats => "cats",
            Self::Progression => "progression",
            Self::StandingOrders => "standing-orders",
            Self::Officers => "officers",
            Self::Tasks => "tasks",
            Self::Diplomacy => "diplomacy",
            Self::Trade => "trade",
            Self::Connection => "connection",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Plan,
    Cat,
    ShrineOffering,
    Research,
    StandingOrder,
    OfficerReport,
    Task,
    FavorEvent,
    ScholarPreparation,
    Boost,
    DiplomacyRelationship,
    TradeContract,
    Feedback,
}

impl EntityKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Plan => "plan",
            Self::Cat => "cat",
            Self::ShrineOffering => "shrine-offering",
            Self::Research => "research",
            Self::StandingOrder => "standing-order",
            Self::OfficerReport => "officer-report",
            Self::Task => "task",
            Self::FavorEvent => "favor-event",
            Self::ScholarPreparation => "scholar-preparation",
            Self::Boost => "boost",
            Self::DiplomacyRelationship => "diplomacy-relationship",
            Self::TradeContract => "trade-contract",
            Self::Feedback => "feedback",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ControlKind {
    MoveUp,
    MoveDown,
    Dismiss,
    Treat,
    Consent,
    Fit,
    Remove,
    Repair,
    Activate,
    Accept,
    Reject,
    Create,
    Edit,
    Delete,
    Purchase,
    Propose,
    Select,
    Reload,
}

impl ControlKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::MoveUp => "move-up",
            Self::MoveDown => "move-down",
            Self::Dismiss => "dismiss",
            Self::Treat => "treat",
            Self::Consent => "consent",
            Self::Fit => "fit",
            Self::Remove => "remove",
            Self::Repair => "repair",
            Self::Activate => "activate",
            Self::Accept => "accept",
            Self::Reject => "reject",
            Self::Create => "create",
            Self::Edit => "edit",
            Self::Delete => "delete",
            Self::Purchase => "purchase",
            Self::Propose => "propose",
            Self::Select => "select",
            Self::Reload => "reload",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TaskMarkerRole {
    Objective,
    WorkSlot,
    Endpoint,
    Cell { row_major_index: u8 },
}

impl TaskMarkerRole {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Objective => "objective",
            Self::WorkSlot => "work-slot",
            Self::Endpoint => "endpoint",
            Self::Cell { .. } => "cell",
        }
    }

    pub fn slug(self) -> String {
        match self {
            Self::Cell { row_major_index } => format!("cell-{row_major_index}"),
            role => role.as_str().to_owned(),
        }
    }
}

pub struct TestIdBuilder;

impl TestIdBuilder {
    pub fn panel(section: UiSection) -> StableUiId {
        StableUiId(format!("lai-ui:{}:panel", section.as_str()))
    }
    pub fn row(section: UiSection, kind: EntityKind, subject: &str) -> StableUiId {
        StableUiId(format!(
            "lai-ui:{}:{}:{}",
            section.as_str(),
            kind.as_str(),
            slug(subject)
        ))
    }
    pub fn control(section: UiSection, kind: ControlKind, subject: &str) -> StableUiId {
        StableUiId(format!(
            "lai-ui:{}:control:{}:{}",
            section.as_str(),
            kind.as_str(),
            slug(subject)
        ))
    }
    pub fn task_marker(task: &str, site: &str, role: TaskMarkerRole) -> StableUiId {
        let role = match role {
            TaskMarkerRole::Cell { row_major_index } => format!("cell-{row_major_index}"),
            role => role.as_str().to_owned(),
        };
        StableUiId(format!(
            "lai-ui:tasks:task:{}:site:{}:{role}",
            slug(task),
            slug(site)
        ))
    }

    pub fn named_panel(section: &str) -> StableUiId {
        StableUiId(format!("lai-ui:{}:panel", slug(section)))
    }

    pub fn named_row(section: &str, kind: EntityKind, subject: &str) -> StableUiId {
        StableUiId(format!(
            "lai-ui:{}:{}:{}",
            slug(section),
            kind.as_str(),
            slug(subject)
        ))
    }

    pub fn named_control(section: &str, kind: ControlKind, subject: &str) -> StableUiId {
        StableUiId(format!(
            "lai-ui:{}:control:{}:{}",
            slug(section),
            kind.as_str(),
            slug(subject)
        ))
    }

    pub fn status(subject: &str) -> StableUiId {
        StableUiId(format!("lai-ui:connection:status:{}", slug(subject)))
    }
}

fn slug(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b':' {
            out.push(byte.to_ascii_lowercase() as char);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    const MAX_SLUG_BYTES: usize = 48;
    if out.len() <= MAX_SLUG_BYTES {
        return out;
    }
    let hash = value.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        hash.wrapping_mul(0x0000_0100_0000_01b3) ^ u64::from(byte)
    });
    out.truncate(MAX_SLUG_BYTES - 17);
    format!("{out}-{hash:016x}")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessibleLabel(String);

impl AccessibleLabel {
    pub fn panel(section: UiSection) -> Self {
        Self(
            match section {
                UiSection::Plans => "Plans panel",
                UiSection::Cats => "Cat care panel",
                UiSection::Progression => "Progression panel",
                UiSection::StandingOrders => "Standing orders panel",
                UiSection::Officers => "Officer reports panel",
                UiSection::Tasks => "Visible tasks panel",
                UiSection::Diplomacy => "Diplomacy panel",
                UiSection::Trade => "Trade contracts panel",
                UiSection::Connection => "Connection status",
            }
            .to_owned(),
        )
    }
    pub fn control(kind: ControlKind, subject: &str) -> Self {
        let verb = match kind {
            ControlKind::MoveUp => "Move plan up",
            ControlKind::MoveDown => "Move plan down",
            ControlKind::Dismiss => "Dismiss plan",
            ControlKind::Treat => "Treat",
            ControlKind::Consent => "Give consent for",
            ControlKind::Fit => "Fit prosthetic for",
            ControlKind::Remove => "Remove prosthetic from",
            ControlKind::Repair => "Repair prosthetic for",
            ControlKind::Activate => "Activate",
            ControlKind::Accept => "Accept",
            ControlKind::Reject => "Reject",
            ControlKind::Create => "Create",
            ControlKind::Edit => "Edit",
            ControlKind::Delete => "Delete",
            ControlKind::Purchase => "Purchase",
            ControlKind::Propose => "Propose",
            ControlKind::Select => "Select",
            ControlKind::Reload => "Reload",
        };
        Self(format!("{verb} {subject}"))
    }
    pub fn task_marker(category: &str, role: TaskMarkerRole, site: &str) -> Self {
        let role = match role {
            TaskMarkerRole::Objective => "objective",
            TaskMarkerRole::WorkSlot => "work slot",
            TaskMarkerRole::Endpoint => "delivery endpoint",
            TaskMarkerRole::Cell { row_major_index } => {
                return Self(format!(
                    "{category} objective cell {row_major_index}, {site}"
                ));
            }
        };
        Self(format!("{category} {role}, {site}"))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct LeaderAiSemanticNode {
    pub semantic_id: String,
    pub focus_order: u32,
    pub enabled: bool,
}

/// Fixed browser contract used only when Chromium cannot expose AccessKit's
/// native canvas tree. These points are part of the production layout, not a
/// hidden test fixture or a state-injection API.
pub const LEADER_AI_BROWSER_VIEWPORT: (u32, u32) = (1280, 720);
pub const LEADER_AI_CANVAS_CHECKPOINTS: [(&str, (u32, u32)); 10] = [
    ("colony", (32, 32)),
    ("plans", (168, 48)),
    ("plans-action", (168, 82)),
    ("standing-orders", (528, 48)),
    ("progression", (888, 160)),
    ("care", (168, 286)),
    ("diplomacy", (528, 286)),
    ("hunt", (888, 340)),
    ("workshop", (888, 360)),
    ("reconnect", (32, 688)),
];

pub const LEADER_AI_CANVAS_ACTION_CHECKPOINTS: [(&str, (u32, u32)); 6] = [
    ("standing-order-edit", (528, 96)),
    ("standing-order-delete", (632, 96)),
    ("research-purchase", (888, 160)),
    ("boost-activate", (888, 268)),
    ("care-prosthetic", (300, 286)),
    ("trade-response", (528, 464)),
];

pub fn semantic_node(
    role: Role,
    semantic_id: impl Into<String>,
    label: impl Into<String>,
    enabled: bool,
) -> AccessibilityNode {
    let semantic_id = semantic_id.into();
    let mut node = Node::new(role);
    node.set_label(label.into());
    node.set_description(format!("test-id:{semantic_id}"));
    if matches!(role, Role::Button | Role::DefaultButton) {
        node.set_keyboard_shortcut("Enter or Space");
        node.add_action(Action::Focus);
        node.add_action(Action::Click);
    }
    if !enabled {
        node.set_disabled();
    }
    AccessibilityNode::from(node)
}

pub fn semantic_status_node(
    semantic_id: impl Into<String>,
    label: impl Into<String>,
    assertive: bool,
) -> AccessibilityNode {
    let semantic_id = semantic_id.into();
    let mut node = Node::new(if assertive { Role::Alert } else { Role::Status });
    node.set_label(label.into());
    node.set_description(format!("test-id:{semantic_id}"));
    node.set_live(if assertive {
        Live::Assertive
    } else {
        Live::Polite
    });
    node.set_live_atomic();
    AccessibilityNode::from(node)
}

pub fn report_safe_semantic_id(raw: &str) -> bool {
    !raw.is_empty()
        && raw.len() <= 160
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'-' | b'_' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stable_builder_keeps_report_safe_ids() {
        assert_eq!(
            TestIdBuilder::panel(UiSection::Plans).as_str(),
            "lai-ui:plans:panel"
        );
        assert_eq!(
            TestIdBuilder::row(UiSection::Cats, EntityKind::Cat, "Cat 42 / Left Paw").as_str(),
            "lai-ui:cats:cat:cat-42-left-paw"
        );
    }
    #[test]
    fn accesskit_button_is_named_and_actionable() {
        let node = semantic_node(Role::Button, "lai-ui:plan:p:move-up", "Move plan up", true);
        assert_eq!(node.role(), Role::Button);
        assert_eq!(node.description(), Some("test-id:lai-ui:plan:p:move-up"));
        assert!(node.supports_action(Action::Focus));
        assert!(node.supports_action(Action::Click));
    }

    #[test]
    fn connection_status_is_a_live_report_safe_node() {
        let node = semantic_status_node(
            TestIdBuilder::status("update-required").as_str(),
            "Update required",
            true,
        );
        assert_eq!(node.role(), Role::Alert);
        assert_eq!(node.live(), Some(Live::Assertive));
        assert_eq!(
            node.description(),
            Some("test-id:lai-ui:connection:status:update-required")
        );
    }

    #[test]
    fn long_authoritative_ids_are_bounded_and_collision_resistant() {
        let left = TestIdBuilder::task_marker(
            &format!("task:{}", "a".repeat(128)),
            &format!("site:{}", "b".repeat(128)),
            TaskMarkerRole::Objective,
        );
        let right = TestIdBuilder::task_marker(
            &format!("task:{}", "a".repeat(127) + "c"),
            &format!("site:{}", "b".repeat(128)),
            TaskMarkerRole::Objective,
        );
        assert!(report_safe_semantic_id(left.as_str()));
        assert!(report_safe_semantic_id(right.as_str()));
        assert_ne!(left, right);
    }
}
