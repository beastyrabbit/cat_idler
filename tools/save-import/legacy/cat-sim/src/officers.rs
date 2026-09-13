//! Officer roles (P12.2).
//!
//! An officer is a living cat appointed to automate one labor category. Vacant
//! offices are manual-only; holding office enables exactly that category and grants
//! no additive productivity bonus.

use serde::{Deserialize, Serialize};

use crate::types::BuildingType;

/// A colony office. Each role governs one labor category (see
/// [`crate::leader_director::officer_role_for`]); an unfilled role emits no auto
/// effect and its labors await manual player actions. `Ord`/`Hash` so it can key a
/// deterministic [`std::collections::BTreeMap`] and serialize as a JSON object key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfficerRole {
    Steward,
    Accountant,
    Forester,
    Farmer,
    Captain,
    Loremaster,
    ClothLeader,
}

impl OfficerRole {
    /// All roles in a stable order (used for deterministic officer iteration).
    pub const ALL: &'static [Self] = &[
        Self::Steward,
        Self::Accountant,
        Self::Forester,
        Self::Farmer,
        Self::Captain,
        Self::Loremaster,
        Self::ClothLeader,
    ];
}

/// Infrastructure and knowledge required before an office can be appointed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OfficerPrerequisite {
    pub building: BuildingType,
    pub upgrade_node: &'static str,
}

/// Return the completed workplace and researched practice that establish a role.
#[must_use]
pub const fn prerequisite_for(role: OfficerRole) -> OfficerPrerequisite {
    match role {
        OfficerRole::Steward => OfficerPrerequisite {
            building: BuildingType::Workshop,
            upgrade_node: "basic_tools",
        },
        OfficerRole::Accountant => OfficerPrerequisite {
            building: BuildingType::AccountingTent,
            upgrade_node: "basic_tools",
        },
        OfficerRole::Forester => OfficerPrerequisite {
            building: BuildingType::Sawmill,
            upgrade_node: "sawmill",
        },
        OfficerRole::Farmer => OfficerPrerequisite {
            building: BuildingType::Field,
            upgrade_node: "irrigation",
        },
        OfficerRole::Captain => OfficerPrerequisite {
            building: BuildingType::Barracks,
            upgrade_node: "barracks",
        },
        OfficerRole::Loremaster => OfficerPrerequisite {
            building: BuildingType::ResearchHut,
            upgrade_node: "research_hut",
        },
        OfficerRole::ClothLeader => OfficerPrerequisite {
            building: BuildingType::Clothier,
            upgrade_node: "textiles",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{OfficerRole, prerequisite_for};
    use crate::types::BuildingType;

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
        assert_eq!(OfficerRole::ALL.len(), 7);
        for role in OfficerRole::ALL {
            assert_eq!(OfficerRole::ALL.iter().filter(|r| *r == role).count(), 1);
        }
    }

    #[test]
    fn every_role_has_a_distinct_sensible_workplace_gate() {
        let expected = [
            (OfficerRole::Steward, BuildingType::Workshop, "basic_tools"),
            (
                OfficerRole::Accountant,
                BuildingType::AccountingTent,
                "basic_tools",
            ),
            (OfficerRole::Forester, BuildingType::Sawmill, "sawmill"),
            (OfficerRole::Farmer, BuildingType::Field, "irrigation"),
            (OfficerRole::Captain, BuildingType::Barracks, "barracks"),
            (
                OfficerRole::Loremaster,
                BuildingType::ResearchHut,
                "research_hut",
            ),
            (OfficerRole::ClothLeader, BuildingType::Clothier, "textiles"),
        ];
        for (role, building, upgrade_node) in expected {
            let prerequisite = prerequisite_for(role);
            assert_eq!(prerequisite.building, building);
            assert_eq!(prerequisite.upgrade_node, upgrade_node);
        }
    }
}
