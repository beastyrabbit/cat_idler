//! Additive, public Hunting Lair wire types.
//!
//! These DTOs intentionally expose only information a colony can reveal or
//! observe: stable site/species ids, coarse danger and risk bands, public
//! timers, party membership, outcome conditions, and awarded loot. Exact
//! monster statistics, combat rolls, RNG state, loot probabilities, and
//! Captain scoring remain authoritative server state.

use serde::{Deserialize, Serialize};

use crate::{ResourceKind, TilePoint};

/// Intrinsic danger visible after a hunting site is revealed.
///
/// This is deliberately a band rather than the simulation's exact encounter
/// strength.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuntingDanger {
    Low,
    Moderate,
    High,
    Deadly,
    Mythic,
}

/// Public lifecycle of a monster at a revealed site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuntingMonsterStatus {
    Available,
    Engaged,
    Defeated,
    Respawning,
}

/// Public wall-clock boundary for a defeated monster's return.
///
/// Clients derive remaining time from the enclosing world snapshot's `now`
/// field, avoiding a second independently ticking countdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HuntingRespawnSnapshot {
    pub respawns_at_ms: i64,
}

/// One visible monster identity. `species_id` is a stable lowercase id; display
/// copy may change without changing that identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HuntingMonsterSnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub species_id: String,
    pub display_name: String,
    pub status: HuntingMonsterStatus,
    pub respawn: Option<HuntingRespawnSnapshot>,
}

/// Whether a site's one-time first-clear trophy is still available or who
/// publicly claimed it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HuntingTrophyStatus {
    Available,
    Claimed {
        colony_id: String,
        party_id: String,
        claimed_at_ms: i64,
    },
}

/// Public first-clear trophy attached to a revealed hunting site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HuntingFirstClearTrophySnapshot {
    pub trophy_id: String,
    pub display_name: String,
    pub status: HuntingTrophyStatus,
}

/// A stable loot identity used by a site's preview.
///
/// Item fields follow [`crate::ItemStackSnapshot`]'s lowercase item-kind and
/// material convention. Quality is a public range because the exact result is
/// not known before combat resolves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HuntingLootDescriptor {
    Resource {
        resource: ResourceKind,
    },
    Item {
        item_kind: String,
        material: String,
        minimum_quality: u8,
        maximum_quality: u8,
    },
}

/// Coarse public likelihood; exact loot-table weights never enter snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuntingLootLikelihood {
    Guaranteed,
    Likely,
    Possible,
    Rare,
}

/// Quantity and likelihood bands shown before a party is dispatched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HuntingLootPreview {
    pub loot: HuntingLootDescriptor,
    pub minimum_quantity: u32,
    pub maximum_quantity: u32,
    pub likelihood: HuntingLootLikelihood,
}

/// A site known to this colony. Unrevealed sites are omitted rather than sent
/// with redacted or hidden-state placeholders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevealedHuntingSiteSnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub display_name: String,
    pub position: TilePoint,
    pub danger: HuntingDanger,
    pub monsters: Vec<HuntingMonsterSnapshot>,
    pub first_clear_trophy: Option<HuntingFirstClearTrophySnapshot>,
    pub loot_preview: Vec<HuntingLootPreview>,
}

/// Captain's relative assessment of a revealed site for the colony's currently
/// available roster. This is intentionally not the exact internal risk score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptainRiskBand {
    Unknown,
    Favorable,
    Even,
    Risky,
    Dire,
}

/// Public Captain recommendation for one revealed site.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptainHuntingAdviceSnapshot {
    pub site_id: String,
    pub risk_band: CaptainRiskBand,
    pub summary: String,
    pub recommended_party_size: u8,
}

/// Observable stage of a dispatched hunting party.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuntingPartyStatus {
    Assembling,
    Traveling,
    Fighting,
    Returning,
    Completed,
    Failed,
    Cancelled,
}

/// Public membership and coarse progress for an active or recently completed
/// hunting party. Combat stats, gear calculations, route internals, and exact
/// progress are not included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HuntingPartySnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub site_id: String,
    pub leader_cat_id: String,
    pub member_cat_ids: Vec<String>,
    pub status: HuntingPartyStatus,
    pub departed_at_ms: i64,
    pub expected_phase_end_at_ms: Option<i64>,
}

/// Public final result of an encounter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuntingCombatResult {
    Victory,
    Withdrawn,
    Defeat,
    Aborted,
}

/// Coarse condition shown for a party member after combat. Exact health and
/// damage calculations remain in the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuntingMemberCondition {
    Safe,
    Injured,
    Incapacitated,
    Killed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HuntingMemberOutcomeSnapshot {
    pub cat_id: String,
    pub condition: HuntingMemberCondition,
}

/// Exact goods publicly awarded after an encounter has resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HuntingRewardSnapshot {
    Resource {
        resource: ResourceKind,
        quantity: u32,
    },
    Items {
        item_kind: String,
        material: String,
        quality: u8,
        count: u32,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        item_ids: Vec<String>,
    },
    /// A typed hunting trophy material tracked by the world ledger rather than
    /// pretending to be a finite crafted [`crate::ItemStackSnapshot`].
    SpeciesMaterial { material: String, count: u32 },
}

/// Public outcome record. It reports what happened, not the private rolls or
/// intermediate combat state used to decide it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HuntingCombatOutcomeSnapshot {
    #[serde(alias = "_id")]
    pub id: String,
    pub party_id: String,
    pub site_id: String,
    pub resolved_at_ms: i64,
    pub result: HuntingCombatResult,
    pub members: Vec<HuntingMemberOutcomeSnapshot>,
    pub monster_statuses: Vec<HuntingMonsterSnapshot>,
    pub rewards: Vec<HuntingRewardSnapshot>,
    pub first_clear_trophy_id: Option<String>,
}

/// Additive Hunting Lair projection for one colony.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HuntingLairSnapshot {
    pub building_id: String,
    pub revealed_sites: Vec<RevealedHuntingSiteSnapshot>,
    pub captain_advice: Vec<CaptainHuntingAdviceSnapshot>,
    pub active_parties: Vec<HuntingPartySnapshot>,
    pub recent_outcomes: Vec<HuntingCombatOutcomeSnapshot>,
    /// Optional player priority hint. `None` leaves site selection entirely to
    /// the Captain/Leader policy.
    pub nudged_site_id: Option<String>,
}

/// Additive authenticated Hunting Lair actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "action",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HuntingLairAction {
    /// Ask the Captain/Leader to review hunting soon. A supplied site id is only
    /// a priority hint; safety and eligibility remain authoritative.
    NudgeHuntingSite {
        session_id: String,
        nickname: String,
        sig: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        site_id: Option<String>,
    },
}
