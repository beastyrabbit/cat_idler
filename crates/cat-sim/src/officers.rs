//! Officer roles (P12.2).
//!
//! An officer is a cat appointed to automate one labor category. Officers are an
//! ADDITIVE layer over the single [`crate::leader_director`] utility AI: an empty
//! `officers` map produces byte-identical director output to pre-P12.2, and every
//! effect happens only when a role is FILLED. See
//! `docs/migration/specs/p12-idle-cat-forest.md` (P12.2).

use serde::{Deserialize, Serialize};

/// A colony office. Each role governs one labor category (see
/// [`crate::leader_director::officer_role_for`]); an unfilled role emits no auto
/// effect and its labors await manual player actions. `Ord`/`Hash` so it can key a
/// deterministic [`std::collections::BTreeMap`] and serialize as a JSON object key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficerRole {
    Steward,
    Forester,
    Farmer,
    Captain,
    Loremaster,
}

impl OfficerRole {
    /// All roles in a stable order (used for deterministic officer iteration).
    pub const ALL: &'static [Self] = &[
        Self::Steward,
        Self::Forester,
        Self::Farmer,
        Self::Captain,
        Self::Loremaster,
    ];
}

#[cfg(test)]
mod tests {
    use super::OfficerRole;

    #[test]
    fn officer_role_serializes_to_snake_case_wire_string() {
        assert_eq!(
            serde_json::to_string(&OfficerRole::Loremaster).unwrap(),
            "\"loremaster\""
        );
        assert_eq!(
            serde_json::from_str::<OfficerRole>("\"captain\"").unwrap(),
            OfficerRole::Captain
        );
    }

    #[test]
    fn all_lists_every_role_once() {
        assert_eq!(OfficerRole::ALL.len(), 5);
        for role in OfficerRole::ALL {
            assert_eq!(OfficerRole::ALL.iter().filter(|r| *r == role).count(), 1);
        }
    }
}
