//! Persistent player directive mutations specified by
//! `docs/leader-ai-overhaul/action-implementation-map.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{officers::OfficerRole, planner_core::PlannerId};

pub const PLAYER_DIRECTIVE_SCHEMA_VERSION: u32 = 1;
pub const MAX_STANDING_ORDERS: usize = 14;
pub const MAX_AUTHORITY_OVERRIDES: usize = 128;
pub const MAX_TREATMENT_REQUESTS: usize = 256;
pub const MAX_BROAD_NUDGES: usize = 16;

/// Canonical God-facing strategic domains. These mirror the protocol's broad
/// vocabulary without giving the player an exact intent, worker, tile, route,
/// or stock command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BroadNudgeDomain {
    Survival,
    Defense,
    Hole,
    Hunting,
    Food,
    Housing,
    Construction,
    Storage,
    Research,
    Trade,
    Infrastructure,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BroadNudgeKey {
    pub domain: BroadNudgeDomain,
    pub building_kind_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BroadNudgeDirective {
    pub key: BroadNudgeKey,
    pub basis_points: i16,
    pub planning_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerDirectiveId(PlannerId);

impl PlayerDirectiveId {
    #[must_use]
    pub fn derive(namespace: &str, colony_id: &PlannerId, action_id: &str) -> Self {
        Self(PlannerId::derive(
            "player_directive",
            [namespace, colony_id.as_str(), action_id],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StandingOrder {
    pub id: PlayerDirectiveId,
    pub order_kind: String,
    pub domain: String,
    pub target_id: Option<String>,
    pub instruction: String,
    pub priority_basis_points: u16,
    pub expires_tick: Option<u64>,
    pub created_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandingOrderPatch {
    pub instruction: Option<String>,
    pub priority_basis_points: Option<u16>,
    pub target_id: Option<String>,
    pub clear_target: bool,
    pub expires_tick: Option<u64>,
    pub clear_expiry: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorityOverrideKey {
    pub role: OfficerRole,
    pub domain: String,
    pub request_id: Option<String>,
}

impl AuthorityOverrideKey {
    fn stable_key(&self) -> String {
        PlannerId::derive(
            "authority_override",
            [
                format!("{:?}", self.role).as_str(),
                self.domain.as_str(),
                self.request_id.as_deref().unwrap_or(""),
            ],
        )
        .to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TreatmentRequest {
    pub id: PlayerDirectiveId,
    pub cat_id: String,
    pub injury_id: String,
    pub treatment_kind: String,
    pub requested_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerDirectiveState {
    pub schema_version: u32,
    pub version: u64,
    pub standing_orders: BTreeMap<PlayerDirectiveId, StandingOrder>,
    pub authority_overrides: BTreeMap<String, bool>,
    pub treatment_requests: BTreeMap<PlayerDirectiveId, TreatmentRequest>,
    #[serde(default)]
    pub broad_nudges: BTreeMap<BroadNudgeKey, BroadNudgeDirective>,
}

impl PlayerDirectiveState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: PLAYER_DIRECTIVE_SCHEMA_VERSION,
            version: 0,
            standing_orders: BTreeMap::new(),
            authority_overrides: BTreeMap::new(),
            treatment_requests: BTreeMap::new(),
            broad_nudges: BTreeMap::new(),
        }
    }

    /// Install or replace one non-stacking broad influence for the current
    /// planning epoch. Advancing the epoch clears every prior influence.
    pub fn set_broad_nudge(
        &mut self,
        directive: BroadNudgeDirective,
    ) -> Result<(), PlayerDirectiveError> {
        validate_broad_nudge(&directive)?;
        let mut next = self.clone();
        next.retain_broad_nudge_epoch(directive.planning_epoch)?;
        if next.broad_nudges.get(&directive.key) == Some(&directive) {
            *self = next;
            return Ok(());
        }
        if !next.broad_nudges.contains_key(&directive.key)
            && next.broad_nudges.len() >= MAX_BROAD_NUDGES
        {
            return Err(PlayerDirectiveError::CapacityReached);
        }
        next.broad_nudges.insert(directive.key.clone(), directive);
        next.bump_version()?;
        next.validate()?;
        *self = next;
        Ok(())
    }

    /// Clear influences from older planning epochs. A future epoch is never
    /// accepted from a caller because it could outlive the report that
    /// authorized the action.
    pub fn retain_broad_nudge_epoch(
        &mut self,
        planning_epoch: u64,
    ) -> Result<(), PlayerDirectiveError> {
        if self
            .broad_nudges
            .values()
            .any(|directive| directive.planning_epoch > planning_epoch)
        {
            return Err(PlayerDirectiveError::InvalidDirective);
        }
        let previous_len = self.broad_nudges.len();
        self.broad_nudges
            .retain(|_, directive| directive.planning_epoch == planning_epoch);
        if self.broad_nudges.len() != previous_len {
            self.bump_version()?;
        }
        Ok(())
    }

    #[must_use]
    pub fn broad_nudge_basis_points(&self, key: &BroadNudgeKey, planning_epoch: u64) -> i16 {
        self.broad_nudges
            .get(key)
            .filter(|directive| directive.planning_epoch == planning_epoch)
            .map_or(0, |directive| directive.basis_points)
    }

    pub fn create_standing_order(
        &mut self,
        order: StandingOrder,
        slot_limit: usize,
    ) -> Result<(), PlayerDirectiveError> {
        self.validate()?;
        if let Some(existing) = self.standing_orders.get(&order.id) {
            return if existing == &order {
                Ok(())
            } else {
                Err(PlayerDirectiveError::IdConflict)
            };
        }
        if self.standing_orders.len() >= slot_limit.min(MAX_STANDING_ORDERS) {
            return Err(PlayerDirectiveError::CapacityReached);
        }
        validate_standing_order(&order)?;
        self.standing_orders.insert(order.id.clone(), order);
        self.bump_version()
    }

    pub fn update_standing_order(
        &mut self,
        id: &str,
        patch: StandingOrderPatch,
    ) -> Result<(), PlayerDirectiveError> {
        let key = self
            .standing_orders
            .keys()
            .find(|key| key.as_str() == id)
            .cloned()
            .ok_or(PlayerDirectiveError::UnknownDirective)?;
        let mut next = self
            .standing_orders
            .get(&key)
            .cloned()
            .ok_or(PlayerDirectiveError::UnknownDirective)?;
        if let Some(instruction) = patch.instruction {
            next.instruction = instruction;
        }
        if let Some(priority) = patch.priority_basis_points {
            next.priority_basis_points = priority;
        }
        if patch.clear_target {
            next.target_id = None;
        } else if let Some(target) = patch.target_id {
            next.target_id = Some(target);
        }
        if patch.clear_expiry {
            next.expires_tick = None;
        } else if let Some(expiry) = patch.expires_tick {
            next.expires_tick = Some(expiry);
        }
        validate_standing_order(&next)?;
        self.standing_orders.insert(key, next);
        self.bump_version()
    }

    pub fn delete_standing_order(&mut self, id: &str) -> Result<(), PlayerDirectiveError> {
        let key = self
            .standing_orders
            .keys()
            .find(|key| key.as_str() == id)
            .cloned()
            .ok_or(PlayerDirectiveError::UnknownDirective)?;
        self.standing_orders.remove(&key);
        self.bump_version()
    }

    pub fn set_authority_override(
        &mut self,
        key: AuthorityOverrideKey,
        granted: bool,
    ) -> Result<(), PlayerDirectiveError> {
        if key.domain.is_empty() || key.domain.len() > 128 {
            return Err(PlayerDirectiveError::InvalidDirective);
        }
        let stable_key = key.stable_key();
        if !self.authority_overrides.contains_key(&stable_key)
            && self.authority_overrides.len() >= MAX_AUTHORITY_OVERRIDES
        {
            return Err(PlayerDirectiveError::CapacityReached);
        }
        if self.authority_overrides.get(&stable_key) == Some(&granted) {
            return Ok(());
        }
        self.authority_overrides.insert(stable_key, granted);
        self.bump_version()
    }

    pub fn request_treatment(
        &mut self,
        request: TreatmentRequest,
    ) -> Result<(), PlayerDirectiveError> {
        if let Some(existing) = self.treatment_requests.get(&request.id) {
            return if existing == &request {
                Ok(())
            } else {
                Err(PlayerDirectiveError::IdConflict)
            };
        }
        if self.treatment_requests.len() >= MAX_TREATMENT_REQUESTS
            || request.cat_id.is_empty()
            || request.injury_id.is_empty()
            || request.treatment_kind.is_empty()
        {
            return Err(PlayerDirectiveError::CapacityReached);
        }
        self.treatment_requests.insert(request.id.clone(), request);
        self.bump_version()
    }

    pub fn validate(&self) -> Result<(), PlayerDirectiveError> {
        if self.schema_version != PLAYER_DIRECTIVE_SCHEMA_VERSION
            || self.standing_orders.len() > MAX_STANDING_ORDERS
            || self.authority_overrides.len() > MAX_AUTHORITY_OVERRIDES
            || self.treatment_requests.len() > MAX_TREATMENT_REQUESTS
            || self.broad_nudges.len() > MAX_BROAD_NUDGES
        {
            return Err(PlayerDirectiveError::MalformedPersistence);
        }
        for (id, order) in &self.standing_orders {
            if id != &order.id {
                return Err(PlayerDirectiveError::MalformedPersistence);
            }
            validate_standing_order(order)?;
        }
        for (id, request) in &self.treatment_requests {
            if id != &request.id
                || request.cat_id.is_empty()
                || request.injury_id.is_empty()
                || request.treatment_kind.is_empty()
            {
                return Err(PlayerDirectiveError::MalformedPersistence);
            }
        }
        for (key, directive) in &self.broad_nudges {
            if key != &directive.key || validate_broad_nudge(directive).is_err() {
                return Err(PlayerDirectiveError::MalformedPersistence);
            }
        }
        Ok(())
    }

    fn bump_version(&mut self) -> Result<(), PlayerDirectiveError> {
        self.version = self
            .version
            .checked_add(1)
            .ok_or(PlayerDirectiveError::VersionExhausted)?;
        Ok(())
    }
}

impl Default for PlayerDirectiveState {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_standing_order(order: &StandingOrder) -> Result<(), PlayerDirectiveError> {
    if order.order_kind.is_empty()
        || order.domain.is_empty()
        || order.instruction.trim().is_empty()
        || order.instruction.len() > 512
        || order
            .expires_tick
            .is_some_and(|expiry| expiry <= order.created_tick)
    {
        Err(PlayerDirectiveError::InvalidDirective)
    } else {
        Ok(())
    }
}

fn validate_broad_nudge(directive: &BroadNudgeDirective) -> Result<(), PlayerDirectiveError> {
    if directive.basis_points == 0 || !(-1_500..=1_500).contains(&directive.basis_points) {
        return Err(PlayerDirectiveError::InvalidDirective);
    }
    match (
        directive.key.domain,
        directive.key.building_kind_id.as_deref(),
    ) {
        (BroadNudgeDomain::Construction, Some(id)) if !id.trim().is_empty() && id.len() <= 128 => {}
        (BroadNudgeDomain::Construction, None) => {}
        (_, None) => {}
        _ => return Err(PlayerDirectiveError::InvalidDirective),
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerDirectiveError {
    UnknownDirective,
    InvalidDirective,
    CapacityReached,
    IdConflict,
    VersionExhausted,
    MalformedPersistence,
}

impl std::fmt::Display for PlayerDirectiveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "player directive error: {self:?}")
    }
}

impl std::error::Error for PlayerDirectiveError {}
