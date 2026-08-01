//! LAI.61 canonical Hole, edible-policy, and divine-action coordinator.
//!
//! This leaf deliberately composes the existing physical Hole, Void ledger,
//! specialized-boost, construction, storage, and edible-policy authorities.
//! It does not keep a second resource balance, item inventory, construction
//! bill, or Hole-axis copy.  LAI.63 is the sole owner of live world-tick
//! routing into those authorities.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Serialize};

use crate::{
    black_hole::BlackHoleState,
    divine_boosts::{
        DivineBoostError, DivineBoostOutcome, DivineBoostPurchaseRequest, DivineBoostState,
        DivineBoostType,
    },
    food_divine_policy::{
        BoundCargoPurpose, CLICK_BATCH_INTERVAL_MS, CLICK_BURST_CAPACITY,
        CLICKS_PER_SECOND_PER_PLAYER, ClickLimiter, ConservationNudge, ContributionKind,
        EmergencySupplyKind, FOOD_DIVINE_SCHEMA_VERSION, FoodConsumptionDecision, FoodDivineError,
        FoodPermission, FoodPolicyActor, HOLE_DELIVERY_APRON_SITE_ID,
        INSPIRATION_EFFECT_BASIS_POINTS, InspirationState, LeaderFoodPolicy,
        MIRACLE_INPUT_VALUE_MULTIPLIER, MIRACLE_LABOR_REDUCTION_BASIS_POINTS, PurposeBoundCargo,
        RESCUE_UNITS_PER_RESIDENT, VOID_INSIGHT_PER_MIRACLE, clicks_required_for_unit,
    },
    planner_core::PlannerId,
    progression_research::{
        CurrencyCommitOutcome, CurrencyEventId, ProgressionAuthority, VoidDebitPurpose,
        VoidInsight, VoidInsightLedger, VoidSpendRequest,
    },
};

pub const DIVINE_HOLE_AUTHORITY_SCHEMA_VERSION: u32 = 1;
pub const MAX_DIVINE_HOLE_COMMAND_RECEIPTS: usize = 512;
pub const MAX_DIVINE_HOLE_TARGETS: usize = 128;
pub const MAX_DIVINE_HOLE_INPUTS: usize = 64;

/// Explicit downstream handoff inventory.  This is source-audit evidence, not
/// an integration claim: all of these paths remain owned by LAI.63–LAI.70.
pub const RUNTIME_CUTOVER_AUDIT: &[&str] = &[
    "LAI.63: bind `BlackHoleState`, this coordinator, `VoidInsightLedger`, construction projects, and `StorageAuthority` in one world-tick transaction.",
    "LAI.63: materialize every `PurposeBoundCargo` at the Hole delivery apron, reserve it, haul it, and reject barter/Hole intake/general-stock routing.",
    "LAI.64/65: expose authenticated batched clicks, Inspiration, boosts, miracles, rescues, versions, receipts, and report-safe errors; persist this aggregate and its receipts.",
    "LAI.70: remove `leader_ai_runtime::ShrineFavorRuntimeAggregate`, `favor.rs`, `shrine_offerings.rs`, and `world_tick` shrine-offering/research mutations.",
    "LAI.70: remove old `BuildingType::Shrine`/spatial shrine-site roots and old coin/blessing mutation paths after the Hole landmark and Void routes are live.",
    "LAI.63: preserve the typed construction-miracle and emergency-rescue Void debit labels through live routing and persistence.",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DivineHoleError {
    BlankId,
    UnsupportedVersion(u32),
    WrongPartition,
    StaleVersion { expected: u64, actual: u64 },
    ReceiptConflict,
    TooManyReceipts,
    TooManyTargets,
    UnknownTarget(String),
    InvalidTarget,
    InvalidBatchWindow,
    IneligibleContribution,
    RateLimited,
    TargetComplete,
    Policy(FoodDivineError),
    LedgerRejected,
    Boost(DivineBoostError),
    InvalidMiracle,
    MissingReportEvidence,
    NoActiveLabor,
    ArithmeticOverflow,
    MalformedState,
}

impl fmt::Display for DivineHoleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "divine Hole authority rejected command: {self:?}")
    }
}

impl std::error::Error for DivineHoleError {}

impl From<FoodDivineError> for DivineHoleError {
    fn from(value: FoodDivineError) -> Self {
        Self::Policy(value)
    }
}

impl From<DivineBoostError> for DivineHoleError {
    fn from(value: DivineBoostError) -> Self {
        Self::Boost(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HoleAuthorityBinding {
    pub colony_id: PlannerId,
    pub hole_id: String,
}

impl HoleAuthorityBinding {
    pub fn new(colony_id: PlannerId, hole_id: impl Into<String>) -> Result<Self, DivineHoleError> {
        let hole_id = hole_id.into();
        stable(&hole_id)?;
        Ok(Self { colony_id, hole_id })
    }

    /// Confirm that this coordinator is attached to the one existing Hole
    /// aggregate.  It never mirrors axes, feed state, upgrades, or rewards.
    pub fn validate_hole(&self, hole: &BlackHoleState) -> Result<(), DivineHoleError> {
        if self.hole_id == hole.hole_id {
            Ok(())
        } else {
            Err(DivineHoleError::WrongPartition)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhysicalEdibleLot {
    pub lot_id: String,
    pub definition_id: String,
}

impl PhysicalEdibleLot {
    fn validate(&self) -> Result<(), DivineHoleError> {
        stable(&self.lot_id)?;
        stable(&self.definition_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalEdibleDecision {
    Unavailable,
    Permitted,
    Protected,
    LethalStarvationOverride,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhysicalEdibleSelection {
    pub lot_id: String,
    pub definition_id: String,
    pub decision: PhysicalEdibleDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClickTargetSpec {
    pub target_id: String,
    pub definition_id: String,
    pub contribution_kind: ContributionKind,
    pub purpose: BoundCargoPurpose,
    pub required_units: u64,
    pub unit_value_micros: u64,
    pub log_value_micros: u64,
    pub active_labor_remaining_seconds: u64,
}

impl ClickTargetSpec {
    pub fn validate(&self) -> Result<(), DivineHoleError> {
        stable(&self.target_id)?;
        stable(&self.definition_id)?;
        if !self.contribution_kind.is_eligible() {
            return Err(DivineHoleError::IneligibleContribution);
        }
        if self.required_units == 0
            || self.unit_value_micros == 0
            || self.log_value_micros == 0
            || self.active_labor_remaining_seconds == 0
        {
            return Err(DivineHoleError::InvalidTarget);
        }
        validate_bound_purpose(&self.purpose)
    }

    pub fn clicks_per_unit(&self) -> Result<u64, DivineHoleError> {
        clicks_required_for_unit(self.unit_value_micros, self.log_value_micros)
            .map_err(DivineHoleError::Policy)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClickTargetState {
    pub spec: ClickTargetSpec,
    pub created_units: u64,
    pub clicks_toward_next_unit: u64,
    pub labor_seconds_removed: u64,
    pub next_cargo_serial: u64,
    pub player_limiters: BTreeMap<String, ClickLimiter>,
}

impl ClickTargetState {
    pub fn from_spec(spec: ClickTargetSpec) -> Result<Self, DivineHoleError> {
        spec.validate()?;
        Ok(Self {
            spec,
            created_units: 0,
            clicks_toward_next_unit: 0,
            labor_seconds_removed: 0,
            next_cargo_serial: 0,
            player_limiters: BTreeMap::new(),
        })
    }

    pub fn remaining_click_capacity(&self) -> Result<u64, DivineHoleError> {
        let clicks_per_unit = self.spec.clicks_per_unit()?;
        self.spec
            .required_units
            .checked_sub(self.created_units)
            .ok_or(DivineHoleError::MalformedState)?
            .checked_mul(clicks_per_unit)
            .and_then(|value| value.checked_sub(self.clicks_toward_next_unit))
            .map(|value| {
                value.min(
                    self.spec
                        .active_labor_remaining_seconds
                        .saturating_sub(self.labor_seconds_removed),
                )
            })
            .ok_or(DivineHoleError::MalformedState)
    }

    fn validate(&self) -> Result<(), DivineHoleError> {
        self.spec.validate()?;
        let clicks_per_unit = self.spec.clicks_per_unit()?;
        if self.created_units > self.spec.required_units
            || self.clicks_toward_next_unit >= clicks_per_unit
            || self.labor_seconds_removed > self.spec.active_labor_remaining_seconds
            || self.player_limiters.len() > MAX_DIVINE_HOLE_TARGETS
            || self
                .player_limiters
                .values()
                .any(|limiter| limiter.tokens_milli > CLICK_BURST_CAPACITY.saturating_mul(1_000))
        {
            return Err(DivineHoleError::MalformedState);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClickBatchRequest {
    pub target_id: String,
    pub player_id: String,
    pub requested_clicks: u32,
    pub client_batch_window_ms: u64,
    pub now_real_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClickBatchOutcome {
    pub accepted_clicks: u32,
    pub rate_limited_clicks: u32,
    pub labor_seconds_removed: u64,
    pub generated_cargo: Vec<PurposeBoundCargo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MiracleLaborStage {
    pub stage_index: u8,
    pub remaining_work_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MiracleInput {
    pub stage_index: u8,
    pub definition_id: String,
    pub quantity: u64,
    pub unit_value_micros: u64,
    pub missing_quantity_before: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConstructionMiracleRequest {
    pub project_id: String,
    pub player_id: String,
    pub hole_feed_value_per_void_micros: u64,
    pub original_total_work_ms: u64,
    pub labor_stages: Vec<MiracleLaborStage>,
    pub inputs: Vec<MiracleInput>,
    pub now_real_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmergencyRescueRequest {
    pub player_id: String,
    pub supply: EmergencySupplyKind,
    pub living_resident_count: u64,
    pub residents_dying_from_hunger: bool,
    pub residents_dying_from_thirst: bool,
    pub now_real_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VoidAction {
    ConstructionMiracle(ConstructionMiracleRequest),
    EmergencyRescue(EmergencyRescueRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VoidActionEnvelope {
    pub command_id: String,
    pub expected_authority_version: u64,
    pub expected_void_version: u64,
    pub action: VoidAction,
}

impl VoidActionEnvelope {
    pub fn new(
        command_id: impl Into<String>,
        expected_authority_version: u64,
        expected_void_version: u64,
        action: VoidAction,
    ) -> Result<Self, DivineHoleError> {
        let command_id = command_id.into();
        stable(&command_id)?;
        Ok(Self {
            command_id,
            expected_authority_version,
            expected_void_version,
            action,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VoidActionOutcome {
    pub command_id: String,
    pub void_event_id: String,
    pub void_debit_micro: u64,
    pub labor_work_removed_ms: u64,
    pub labor_stages_after: Vec<MiracleLaborStage>,
    pub generated_cargo: Vec<PurposeBoundCargo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DivineHoleCommand {
    RegisterEdible {
        edible_id: String,
        is_divine_ration: bool,
        now_tick: u64,
    },
    SetPermission {
        edible_id: String,
        permission: FoodPermission,
        now_tick: u64,
    },
    SetConservationNudge {
        nudge: ConservationNudge,
    },
    RegisterClickTarget {
        target: ClickTargetSpec,
    },
    AcceptClickBatch {
        batch: ClickBatchRequest,
    },
    ActivateInspiration {
        player_id: String,
        now_real_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DivineHoleCommandEnvelope {
    pub command_id: String,
    pub expected_version: u64,
    pub command: DivineHoleCommand,
}

impl DivineHoleCommandEnvelope {
    pub fn new(
        command_id: impl Into<String>,
        expected_version: u64,
        command: DivineHoleCommand,
    ) -> Result<Self, DivineHoleError> {
        let command_id = command_id.into();
        stable(&command_id)?;
        Ok(Self {
            command_id,
            expected_version,
            command,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DivineHoleCommandOutcome {
    Applied,
    ClickBatch(ClickBatchOutcome),
    Inspiration {
        active_until_real_ms: u64,
        cooldown_until_real_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CommandReceipt {
    command_id: String,
    fingerprint: u64,
    sequence: u64,
    outcome: DivineHoleCommandOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VoidActionReceipt {
    command_id: String,
    fingerprint: u64,
    sequence: u64,
    outcome: VoidActionOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DivineHoleAuthority {
    pub schema_version: u32,
    pub binding: HoleAuthorityBinding,
    pub version: u64,
    pub edible_policy: LeaderFoodPolicy,
    pub inspiration: InspirationState,
    pub click_targets: BTreeMap<String, ClickTargetState>,
    receipts: BTreeMap<String, CommandReceipt>,
    void_action_receipts: BTreeMap<String, VoidActionReceipt>,
    next_receipt_sequence: u64,
}

impl DivineHoleAuthority {
    pub fn new(binding: HoleAuthorityBinding) -> Self {
        Self {
            schema_version: DIVINE_HOLE_AUTHORITY_SCHEMA_VERSION,
            binding,
            version: 0,
            edible_policy: LeaderFoodPolicy::new(),
            inspiration: InspirationState::new(),
            click_targets: BTreeMap::new(),
            receipts: BTreeMap::new(),
            void_action_receipts: BTreeMap::new(),
            next_receipt_sequence: 0,
        }
    }

    pub fn decode_strict(json: &str) -> Result<Self, DivineHoleError> {
        let state: Self =
            serde_json::from_str(json).map_err(|_| DivineHoleError::MalformedState)?;
        state.validate()?;
        Ok(state)
    }

    pub fn canonical_json(&self) -> Result<String, DivineHoleError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| DivineHoleError::MalformedState)
    }

    pub fn validate(&self) -> Result<(), DivineHoleError> {
        let receipt_sequences = self
            .receipts
            .values()
            .map(|receipt| receipt.sequence)
            .chain(
                self.void_action_receipts
                    .values()
                    .map(|receipt| receipt.sequence),
            )
            .collect::<BTreeSet<_>>();
        let receipt_count = self
            .receipts
            .len()
            .saturating_add(self.void_action_receipts.len());
        if self.schema_version != DIVINE_HOLE_AUTHORITY_SCHEMA_VERSION
            || self.edible_policy.schema_version != FOOD_DIVINE_SCHEMA_VERSION
            || self.inspiration.schema_version != FOOD_DIVINE_SCHEMA_VERSION
            || self
                .receipts
                .len()
                .saturating_add(self.void_action_receipts.len())
                > MAX_DIVINE_HOLE_COMMAND_RECEIPTS
            || self.click_targets.len() > MAX_DIVINE_HOLE_TARGETS
            || self
                .click_targets
                .iter()
                .any(|(id, target)| id != &target.spec.target_id || target.validate().is_err())
            || self
                .receipts
                .iter()
                .any(|(id, receipt)| id != &receipt.command_id)
            || self
                .void_action_receipts
                .iter()
                .any(|(id, receipt)| id != &receipt.command_id)
            || self
                .receipts
                .keys()
                .any(|id| self.void_action_receipts.contains_key(id))
            || receipt_sequences.len() != receipt_count
            || receipt_sequences
                .iter()
                .any(|sequence| *sequence >= self.next_receipt_sequence)
            || self
                .edible_policy
                .entries
                .iter()
                .any(|(id, entry)| id != &entry.edible_id || stable(id).is_err())
            || self.inspiration.by_player.iter().any(|(id, window)| {
                id != &window.player_id
                    || stable(id).is_err()
                    || window.active_until_real_ms < window.activated_at_real_ms
                    || window.cooldown_until_real_ms < window.active_until_real_ms
            })
        {
            return Err(DivineHoleError::MalformedState);
        }
        stable(&self.binding.hole_id)?;
        Ok(())
    }

    /// Select against actual lot identities supplied by the storage authority.
    /// The policy can be stale or poor: it is intentionally not replaced by a
    /// hidden stock veto.  Only lethal starvation may bypass Forbidden.
    pub fn decide_physical_edible(
        &self,
        requested_lot_id: &str,
        available_lots: &[PhysicalEdibleLot],
        ordinary_nutrition_inadequate: bool,
        lethal_starvation: bool,
    ) -> Result<PhysicalEdibleSelection, DivineHoleError> {
        stable(requested_lot_id)?;
        let mut definitions = BTreeSet::new();
        let mut requested = None;
        for lot in available_lots {
            lot.validate()?;
            definitions.insert(lot.definition_id.clone());
            if lot.lot_id == requested_lot_id {
                requested = Some(lot);
            }
        }
        let Some(lot) = requested else {
            return Ok(PhysicalEdibleSelection {
                lot_id: requested_lot_id.to_owned(),
                definition_id: String::new(),
                decision: PhysicalEdibleDecision::Unavailable,
            });
        };
        let decision = match self.edible_policy.consumption_decision(
            &lot.definition_id,
            &definitions,
            ordinary_nutrition_inadequate,
            lethal_starvation,
        ) {
            FoodConsumptionDecision::Unavailable => PhysicalEdibleDecision::Unavailable,
            FoodConsumptionDecision::Permitted => PhysicalEdibleDecision::Permitted,
            FoodConsumptionDecision::Protected => PhysicalEdibleDecision::Protected,
            FoodConsumptionDecision::LethalEmergencyOverride => {
                PhysicalEdibleDecision::LethalStarvationOverride
            }
        };
        Ok(PhysicalEdibleSelection {
            lot_id: lot.lot_id.clone(),
            definition_id: lot.definition_id.clone(),
            decision,
        })
    }

    pub fn apply(
        &mut self,
        envelope: DivineHoleCommandEnvelope,
    ) -> Result<DivineHoleCommandOutcome, DivineHoleError> {
        self.validate()?;
        let fingerprint = command_fingerprint(&envelope.command);
        if let Some(receipt) = self.receipts.get(&envelope.command_id) {
            return if receipt.fingerprint == fingerprint {
                Ok(receipt.outcome.clone())
            } else {
                Err(DivineHoleError::ReceiptConflict)
            };
        }
        if self.void_action_receipts.contains_key(&envelope.command_id) {
            return Err(DivineHoleError::ReceiptConflict);
        }
        if envelope.expected_version != self.version {
            return Err(DivineHoleError::StaleVersion {
                expected: envelope.expected_version,
                actual: self.version,
            });
        }
        if self
            .receipts
            .len()
            .saturating_add(self.void_action_receipts.len())
            >= MAX_DIVINE_HOLE_COMMAND_RECEIPTS
        {
            return Err(DivineHoleError::TooManyReceipts);
        }
        let mut next = self.clone();
        let outcome = next.apply_inner(envelope.command)?;
        next.version = next
            .version
            .checked_add(1)
            .ok_or(DivineHoleError::ArithmeticOverflow)?;
        let sequence = next.next_receipt_sequence;
        next.next_receipt_sequence = next
            .next_receipt_sequence
            .checked_add(1)
            .ok_or(DivineHoleError::ArithmeticOverflow)?;
        next.receipts.insert(
            envelope.command_id.clone(),
            CommandReceipt {
                command_id: envelope.command_id,
                fingerprint,
                sequence,
                outcome: outcome.clone(),
            },
        );
        next.validate()?;
        *self = next;
        Ok(outcome)
    }

    fn apply_inner(
        &mut self,
        command: DivineHoleCommand,
    ) -> Result<DivineHoleCommandOutcome, DivineHoleError> {
        match command {
            DivineHoleCommand::RegisterEdible {
                edible_id,
                is_divine_ration,
                now_tick,
            } => {
                self.edible_policy
                    .register_edible(edible_id, is_divine_ration, now_tick)?;
                Ok(DivineHoleCommandOutcome::Applied)
            }
            DivineHoleCommand::SetPermission {
                edible_id,
                permission,
                now_tick,
            } => {
                self.edible_policy.set_permission(
                    FoodPolicyActor::Leader,
                    &edible_id,
                    permission,
                    now_tick,
                )?;
                Ok(DivineHoleCommandOutcome::Applied)
            }
            DivineHoleCommand::SetConservationNudge { nudge } => {
                self.edible_policy.set_conservation_nudge(nudge)?;
                Ok(DivineHoleCommandOutcome::Applied)
            }
            DivineHoleCommand::RegisterClickTarget { target } => {
                target.validate()?;
                if self.click_targets.len() >= MAX_DIVINE_HOLE_TARGETS {
                    return Err(DivineHoleError::TooManyTargets);
                }
                if self.click_targets.contains_key(&target.target_id) {
                    return Err(DivineHoleError::InvalidTarget);
                }
                self.click_targets.insert(
                    target.target_id.clone(),
                    ClickTargetState::from_spec(target)?,
                );
                Ok(DivineHoleCommandOutcome::Applied)
            }
            DivineHoleCommand::AcceptClickBatch { batch } => Ok(
                DivineHoleCommandOutcome::ClickBatch(self.accept_click_batch(batch)?),
            ),
            DivineHoleCommand::ActivateInspiration {
                player_id,
                now_real_ms,
            } => {
                self.inspiration.activate(player_id.clone(), now_real_ms)?;
                let window = self
                    .inspiration
                    .by_player
                    .get(&player_id)
                    .ok_or(DivineHoleError::MalformedState)?;
                Ok(DivineHoleCommandOutcome::Inspiration {
                    active_until_real_ms: window.active_until_real_ms,
                    cooldown_until_real_ms: window.cooldown_until_real_ms,
                })
            }
        }
    }

    fn accept_click_batch(
        &mut self,
        batch: ClickBatchRequest,
    ) -> Result<ClickBatchOutcome, DivineHoleError> {
        stable(&batch.target_id)?;
        stable(&batch.player_id)?;
        if batch.client_batch_window_ms != CLICK_BATCH_INTERVAL_MS {
            return Err(DivineHoleError::InvalidBatchWindow);
        }
        let target = self
            .click_targets
            .get_mut(&batch.target_id)
            .ok_or_else(|| DivineHoleError::UnknownTarget(batch.target_id.clone()))?;
        target.validate()?;
        let limiter = target
            .player_limiters
            .entry(batch.player_id.clone())
            .or_insert_with(|| ClickLimiter::full(batch.now_real_ms));
        let rate_accepted = accept_limited(limiter, batch.requested_clicks, batch.now_real_ms);
        let capacity = target.remaining_click_capacity()?.min(u64::from(u32::MAX)) as u32;
        let accepted = rate_accepted.min(capacity);
        let clicks_per_unit = target.spec.clicks_per_unit()?;
        let mut generated_cargo = Vec::new();
        for _ in 0..accepted {
            target.labor_seconds_removed = target
                .labor_seconds_removed
                .checked_add(1)
                .ok_or(DivineHoleError::ArithmeticOverflow)?;
            target.clicks_toward_next_unit = target
                .clicks_toward_next_unit
                .checked_add(1)
                .ok_or(DivineHoleError::ArithmeticOverflow)?;
            if target.clicks_toward_next_unit == clicks_per_unit {
                target.clicks_toward_next_unit = 0;
                target.created_units = target
                    .created_units
                    .checked_add(1)
                    .ok_or(DivineHoleError::ArithmeticOverflow)?;
                let cargo_id = format!(
                    "divine-click:{}:{}",
                    target.spec.target_id, target.next_cargo_serial
                );
                target.next_cargo_serial = target
                    .next_cargo_serial
                    .checked_add(1)
                    .ok_or(DivineHoleError::ArithmeticOverflow)?;
                generated_cargo.push(PurposeBoundCargo {
                    cargo_id,
                    definition_id: target.spec.definition_id.clone(),
                    quantity: 1,
                    canonical_value_micros: target.spec.unit_value_micros,
                    purpose: target.spec.purpose.clone(),
                    provenance_player_id: batch.player_id.clone(),
                    created_at_real_ms: batch.now_real_ms,
                    site_id: HOLE_DELIVERY_APRON_SITE_ID.to_owned(),
                });
            }
        }
        Ok(ClickBatchOutcome {
            accepted_clicks: accepted,
            rate_limited_clicks: batch.requested_clicks.saturating_sub(rate_accepted),
            labor_seconds_removed: u64::from(accepted),
            generated_cargo,
        })
    }

    /// Commits a one-Void action against the same external ledger used by the
    /// four specialized boosts.  The output is only a purpose-bound handoff;
    /// storage/construction mutation remains the LAI.63 transaction owner.
    pub fn apply_void_action(
        &mut self,
        ledger: &mut VoidInsightLedger,
        envelope: VoidActionEnvelope,
    ) -> Result<VoidActionOutcome, DivineHoleError> {
        self.validate()?;
        if ledger.partition.colony_id != self.binding.colony_id {
            return Err(DivineHoleError::WrongPartition);
        }
        let fingerprint = void_action_fingerprint(&envelope.action);
        if let Some(receipt) = self.void_action_receipts.get(&envelope.command_id) {
            return if receipt.fingerprint == fingerprint {
                Ok(receipt.outcome.clone())
            } else {
                Err(DivineHoleError::ReceiptConflict)
            };
        }
        if self.receipts.contains_key(&envelope.command_id) {
            return Err(DivineHoleError::ReceiptConflict);
        }
        if envelope.expected_authority_version != self.version {
            return Err(DivineHoleError::StaleVersion {
                expected: envelope.expected_authority_version,
                actual: self.version,
            });
        }
        if envelope.expected_void_version != ledger.version {
            return Err(DivineHoleError::StaleVersion {
                expected: envelope.expected_void_version,
                actual: ledger.version,
            });
        }
        if self
            .receipts
            .len()
            .saturating_add(self.void_action_receipts.len())
            >= MAX_DIVINE_HOLE_COMMAND_RECEIPTS
        {
            return Err(DivineHoleError::TooManyReceipts);
        }
        let mut next = self.clone();
        let mut next_ledger = ledger.clone();
        let outcome = next.apply_void_action_inner(&mut next_ledger, &envelope, fingerprint)?;
        next.version = next
            .version
            .checked_add(1)
            .ok_or(DivineHoleError::ArithmeticOverflow)?;
        let sequence = next.next_receipt_sequence;
        next.next_receipt_sequence = next
            .next_receipt_sequence
            .checked_add(1)
            .ok_or(DivineHoleError::ArithmeticOverflow)?;
        next.void_action_receipts.insert(
            envelope.command_id.clone(),
            VoidActionReceipt {
                command_id: envelope.command_id,
                fingerprint,
                sequence,
                outcome: outcome.clone(),
            },
        );
        next.validate()?;
        *self = next;
        *ledger = next_ledger;
        Ok(outcome)
    }

    fn apply_void_action_inner(
        &self,
        ledger: &mut VoidInsightLedger,
        envelope: &VoidActionEnvelope,
        fingerprint: u64,
    ) -> Result<VoidActionOutcome, DivineHoleError> {
        let (labor_work_removed_ms, labor_stages_after, generated_cargo) = match &envelope.action {
            VoidAction::ConstructionMiracle(request) => {
                self.prepare_construction_miracle(&envelope.command_id, request)?
            }
            VoidAction::EmergencyRescue(request) => (
                0,
                Vec::new(),
                self.prepare_rescue(&envelope.command_id, request)?,
            ),
        };
        let event_id = CurrencyEventId::derive(
            "lai61_divine_hole",
            &self.binding.colony_id,
            &envelope.command_id,
        );
        let purpose = match &envelope.action {
            VoidAction::ConstructionMiracle(_) => VoidDebitPurpose::ConstructionMiracle,
            VoidAction::EmergencyRescue(_) => VoidDebitPurpose::EmergencyRescue,
        };
        let debit = ledger
            .debit(VoidSpendRequest {
                id: event_id.clone(),
                amount: VoidInsight::from_whole(VOID_INSIGHT_PER_MIRACLE)
                    .ok_or(DivineHoleError::ArithmeticOverflow)?,
                purpose,
                expected_version: envelope.expected_void_version,
                fingerprint,
            })
            .map_err(|_| DivineHoleError::LedgerRejected)?;
        if !matches!(
            debit,
            CurrencyCommitOutcome::Committed | CurrencyCommitOutcome::AlreadyCommitted
        ) {
            return Err(DivineHoleError::LedgerRejected);
        }
        Ok(VoidActionOutcome {
            command_id: envelope.command_id.clone(),
            void_event_id: event_id.as_str().to_owned(),
            void_debit_micro: VoidInsight::ONE.micro(),
            labor_work_removed_ms,
            labor_stages_after,
            generated_cargo,
        })
    }

    fn prepare_construction_miracle(
        &self,
        command_id: &str,
        request: &ConstructionMiracleRequest,
    ) -> Result<(u64, Vec<MiracleLaborStage>, Vec<PurposeBoundCargo>), DivineHoleError> {
        stable(command_id)?;
        stable(&request.project_id)?;
        stable(&request.player_id)?;
        if request.hole_feed_value_per_void_micros == 0
            || request.original_total_work_ms == 0
            || request.labor_stages.is_empty()
            || request.labor_stages.len() > 3
            || request.inputs.is_empty()
            || request.inputs.len() > MAX_DIVINE_HOLE_INPUTS
        {
            return Err(DivineHoleError::InvalidMiracle);
        }
        if request
            .labor_stages
            .windows(2)
            .any(|pair| pair[0].stage_index >= pair[1].stage_index)
            || request
                .labor_stages
                .iter()
                .all(|stage| stage.remaining_work_ms == 0)
        {
            return Err(DivineHoleError::NoActiveLabor);
        }
        let earliest_stage = request
            .labor_stages
            .iter()
            .find(|stage| stage.remaining_work_ms > 0)
            .ok_or(DivineHoleError::NoActiveLabor)?
            .stage_index;
        if request.inputs.iter().any(|input| {
            input.stage_index != earliest_stage
                || input.quantity == 0
                || input.quantity > input.missing_quantity_before
                || input.unit_value_micros == 0
                || input.definition_id.trim().is_empty()
        }) {
            return Err(DivineHoleError::InvalidMiracle);
        }
        let input_value = request.inputs.iter().try_fold(0_u64, |total, input| {
            input
                .quantity
                .checked_mul(input.unit_value_micros)
                .and_then(|value| total.checked_add(value))
                .ok_or(DivineHoleError::ArithmeticOverflow)
        })?;
        let required_value = request
            .hole_feed_value_per_void_micros
            .checked_mul(MIRACLE_INPUT_VALUE_MULTIPLIER)
            .ok_or(DivineHoleError::ArithmeticOverflow)?;
        if input_value != required_value {
            return Err(DivineHoleError::InvalidMiracle);
        }
        let removal = request
            .original_total_work_ms
            .checked_mul(MIRACLE_LABOR_REDUCTION_BASIS_POINTS)
            .ok_or(DivineHoleError::ArithmeticOverflow)?
            / 10_000;
        let available_work = request
            .labor_stages
            .iter()
            .try_fold(0_u64, |total, stage| {
                total
                    .checked_add(stage.remaining_work_ms)
                    .ok_or(DivineHoleError::ArithmeticOverflow)
            })?;
        if removal == 0 || available_work < removal {
            return Err(DivineHoleError::NoActiveLabor);
        }
        let mut labor_stages_after = request.labor_stages.clone();
        let mut remaining_reduction = removal;
        for stage in &mut labor_stages_after {
            let removed = stage.remaining_work_ms.min(remaining_reduction);
            stage.remaining_work_ms -= removed;
            remaining_reduction -= removed;
            if remaining_reduction == 0 {
                break;
            }
        }
        if remaining_reduction != 0 {
            return Err(DivineHoleError::NoActiveLabor);
        }
        let cargo = request
            .inputs
            .iter()
            .enumerate()
            .map(|(serial, input)| PurposeBoundCargo {
                cargo_id: format!("miracle:{command_id}:{serial}"),
                definition_id: input.definition_id.clone(),
                quantity: input.quantity,
                canonical_value_micros: input.quantity.saturating_mul(input.unit_value_micros),
                purpose: BoundCargoPurpose::Construction {
                    project_id: request.project_id.clone(),
                    stage_index: input.stage_index,
                },
                provenance_player_id: request.player_id.clone(),
                created_at_real_ms: request.now_real_ms,
                site_id: request.project_id.clone(),
            })
            .collect();
        Ok((removal, labor_stages_after, cargo))
    }

    fn prepare_rescue(
        &self,
        command_id: &str,
        request: &EmergencyRescueRequest,
    ) -> Result<Vec<PurposeBoundCargo>, DivineHoleError> {
        stable(command_id)?;
        stable(&request.player_id)?;
        let visible = match request.supply {
            EmergencySupplyKind::DivineRation => request.residents_dying_from_hunger,
            EmergencySupplyKind::DivineWater => request.residents_dying_from_thirst,
        };
        if !visible {
            return Err(DivineHoleError::MissingReportEvidence);
        }
        let quantity = request
            .living_resident_count
            .checked_mul(RESCUE_UNITS_PER_RESIDENT)
            .ok_or(DivineHoleError::ArithmeticOverflow)?;
        Ok(vec![PurposeBoundCargo {
            cargo_id: format!("void-rescue:{command_id}"),
            definition_id: request.supply.definition_id().to_owned(),
            quantity,
            canonical_value_micros: 0,
            purpose: BoundCargoPurpose::Emergency {
                supply: request.supply,
            },
            provenance_player_id: request.player_id.clone(),
            created_at_real_ms: request.now_real_ms,
            site_id: HOLE_DELIVERY_APRON_SITE_ID.to_owned(),
        }])
    }

    /// Delegates purchase to the existing four-boost authority, sharing the
    /// same ledger passed to construction/rescue above rather than copying it.
    pub fn purchase_specialized_boost(
        &self,
        boosts: &mut DivineBoostState,
        ledger: &mut VoidInsightLedger,
        progression: &ProgressionAuthority,
        request: DivineBoostPurchaseRequest,
    ) -> Result<DivineBoostOutcome, DivineHoleError> {
        if ledger.partition.colony_id != self.binding.colony_id
            || progression.partition.colony_id != self.binding.colony_id
        {
            return Err(DivineHoleError::WrongPartition);
        }
        boosts
            .purchase(ledger, progression, request)
            .map_err(DivineHoleError::Boost)
    }

    #[must_use]
    pub const fn specialized_boosts() -> [DivineBoostType; 4] {
        DivineBoostType::ALL
    }

    #[must_use]
    pub fn report_safe_summary(&self, now_real_ms: u64) -> DivineHoleReport {
        DivineHoleReport {
            hole_id: self.binding.hole_id.clone(),
            leader_policy_entries: self.edible_policy.entries.len(),
            click_targets_open: self
                .click_targets
                .values()
                .filter(|target| target.remaining_click_capacity().unwrap_or(0) > 0)
                .count(),
            active_inspiration_stacks: self.inspiration.additive_effect_basis_points(now_real_ms)
                / u64::from(INSPIRATION_EFFECT_BASIS_POINTS),
            inspiration_effect_basis_points: self
                .inspiration
                .additive_effect_basis_points(now_real_ms),
            batch_interval_ms: CLICK_BATCH_INTERVAL_MS,
            accepted_clicks_per_player_per_second: CLICKS_PER_SECOND_PER_PLAYER,
            rescue_controls_report_gated: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DivineHoleReport {
    pub hole_id: String,
    pub leader_policy_entries: usize,
    pub click_targets_open: usize,
    pub active_inspiration_stacks: u64,
    pub inspiration_effect_basis_points: u64,
    pub batch_interval_ms: u64,
    pub accepted_clicks_per_player_per_second: u32,
    pub rescue_controls_report_gated: bool,
}

fn accept_limited(limiter: &mut ClickLimiter, requested: u32, now_real_ms: u64) -> u32 {
    let elapsed = now_real_ms.saturating_sub(limiter.last_refill_real_ms);
    let refill = elapsed.saturating_mul(u64::from(CLICKS_PER_SECOND_PER_PLAYER));
    limiter.tokens_milli = limiter
        .tokens_milli
        .saturating_add(refill.min(u64::from(u32::MAX)) as u32)
        .min(CLICK_BURST_CAPACITY * 1_000);
    limiter.last_refill_real_ms = now_real_ms;
    let accepted = requested.min(limiter.tokens_milli / 1_000);
    limiter.tokens_milli -= accepted * 1_000;
    accepted
}

fn validate_bound_purpose(purpose: &BoundCargoPurpose) -> Result<(), DivineHoleError> {
    match purpose {
        BoundCargoPurpose::Construction { project_id, .. } => stable(project_id),
        BoundCargoPurpose::Emergency { .. } => Ok(()),
    }
}

fn stable(value: &str) -> Result<(), DivineHoleError> {
    if value.trim().is_empty() {
        Err(DivineHoleError::BlankId)
    } else {
        Ok(())
    }
}

fn command_fingerprint(command: &DivineHoleCommand) -> u64 {
    stable_hash(&serde_json::to_string(command).expect("command serialization is infallible"))
}

fn void_action_fingerprint(action: &VoidAction) -> u64 {
    stable_hash(&serde_json::to_string(action).expect("action serialization is infallible"))
}

fn stable_hash(value: &str) -> u64 {
    value
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
}
