//! Food permissions and bounded divine actions for LAI.61.
//!
//! The physical Hole intake remains in [`crate::black_hole`]. This module owns
//! the additive policy and transaction contracts around food conservation,
//! ordinary contribution clicks, Inspiration, construction miracles, and
//! emergency provisions. It is deterministic: callers supply real-time
//! timestamps and report-safe evidence.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const FOOD_DIVINE_SCHEMA_VERSION: u32 = 1;
pub const LOG_BASE_CLICKS_PER_UNIT: u64 = 100;
pub const CLICK_BATCH_INTERVAL_MS: u64 = 100;
pub const CLICKS_PER_SECOND_PER_PLAYER: u32 = 20;
pub const CLICK_BURST_SECONDS: u32 = 2;
pub const CLICK_BURST_CAPACITY: u32 = CLICKS_PER_SECOND_PER_PLAYER * CLICK_BURST_SECONDS;
pub const INSPIRATION_EFFECT_BASIS_POINTS: u32 = 1_000;
pub const INSPIRATION_DURATION_REAL_MS: u64 = 15 * 60 * 1_000;
pub const INSPIRATION_COOLDOWN_REAL_MS: u64 = 60 * 60 * 1_000;
pub const VOID_INSIGHT_PER_MIRACLE: u64 = 1;
pub const MIRACLE_INPUT_VALUE_MULTIPLIER: u64 = 2;
pub const MIRACLE_LABOR_REDUCTION_BASIS_POINTS: u64 = 1_000;
pub const BASIS_POINTS_SCALE: u64 = 10_000;
pub const RESCUE_UNITS_PER_RESIDENT: u64 = 2;
pub const EMERGENCY_HAUL_PRIORITY: u8 = u8::MAX;
pub const HOLE_DELIVERY_APRON_SITE_ID: &str = "hole_delivery_apron";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoodPermission {
    Allowed,
    Reserve,
    Forbidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoodPolicyActor {
    Leader,
    God,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConservationNudge {
    ProtectScarceFood,
    Balanced,
    FavorImmediateSurvival,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FoodPolicyEntry {
    pub edible_id: String,
    pub permission: FoodPermission,
    pub updated_at_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaderFoodPolicy {
    pub schema_version: u32,
    pub version: u64,
    pub conservation_nudge: ConservationNudge,
    pub entries: BTreeMap<String, FoodPolicyEntry>,
}

impl LeaderFoodPolicy {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: FOOD_DIVINE_SCHEMA_VERSION,
            version: 0,
            conservation_nudge: ConservationNudge::Balanced,
            entries: BTreeMap::new(),
        }
    }

    pub fn register_edible(
        &mut self,
        edible_id: impl Into<String>,
        is_divine_ration: bool,
        now_tick: u64,
    ) -> Result<(), FoodDivineError> {
        let edible_id = edible_id.into();
        validate_id(&edible_id)?;
        if self.entries.contains_key(&edible_id) {
            return Err(FoodDivineError::DuplicateId);
        }
        let permission = if is_divine_ration {
            FoodPermission::Reserve
        } else {
            FoodPermission::Allowed
        };
        self.entries.insert(
            edible_id.clone(),
            FoodPolicyEntry {
                edible_id,
                permission,
                updated_at_tick: now_tick,
            },
        );
        self.bump_version()
    }

    pub fn set_permission(
        &mut self,
        actor: FoodPolicyActor,
        edible_id: &str,
        permission: FoodPermission,
        now_tick: u64,
    ) -> Result<(), FoodDivineError> {
        if actor != FoodPolicyActor::Leader {
            return Err(FoodDivineError::GodCannotEditIndividualFood);
        }
        let entry = self
            .entries
            .get_mut(edible_id)
            .ok_or(FoodDivineError::UnknownFood)?;
        if entry.permission == permission {
            return Ok(());
        }
        entry.permission = permission;
        entry.updated_at_tick = now_tick;
        self.bump_version()
    }

    pub fn set_conservation_nudge(
        &mut self,
        nudge: ConservationNudge,
    ) -> Result<(), FoodDivineError> {
        if self.conservation_nudge == nudge {
            return Ok(());
        }
        self.conservation_nudge = nudge;
        self.bump_version()
    }

    #[must_use]
    pub fn consumption_decision(
        &self,
        edible_id: &str,
        physically_available_ids: &BTreeSet<String>,
        ordinary_nutrition_inadequate: bool,
        lethal_starvation: bool,
    ) -> FoodConsumptionDecision {
        if !physically_available_ids.contains(edible_id) {
            return FoodConsumptionDecision::Unavailable;
        }
        let permission = self
            .entries
            .get(edible_id)
            .map_or(FoodPermission::Allowed, |entry| entry.permission);
        match permission {
            FoodPermission::Allowed => FoodConsumptionDecision::Permitted,
            FoodPermission::Reserve if ordinary_nutrition_inadequate => {
                FoodConsumptionDecision::Permitted
            }
            FoodPermission::Reserve => FoodConsumptionDecision::Protected,
            FoodPermission::Forbidden => {
                let permitted_alternative_exists =
                    physically_available_ids.iter().any(|candidate| {
                        self.entries
                            .get(candidate)
                            .map_or(FoodPermission::Allowed, |entry| entry.permission)
                            != FoodPermission::Forbidden
                    });
                if lethal_starvation && !permitted_alternative_exists {
                    FoodConsumptionDecision::LethalEmergencyOverride
                } else {
                    FoodConsumptionDecision::Protected
                }
            }
        }
    }

    fn bump_version(&mut self) -> Result<(), FoodDivineError> {
        self.version = self
            .version
            .checked_add(1)
            .ok_or(FoodDivineError::Overflow)?;
        Ok(())
    }
}

impl Default for LeaderFoodPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoodConsumptionDecision {
    Unavailable,
    Permitted,
    Protected,
    LethalEmergencyOverride,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContributionKind {
    Material,
    Resource,
    TypedFood,
    SmallItem,
    RareCreatureMaterial,
    CompletedEquipment,
    Fixture,
    Augmentation,
}

impl ContributionKind {
    #[must_use]
    pub const fn is_eligible(self) -> bool {
        matches!(
            self,
            Self::Material | Self::Resource | Self::TypedFood | Self::SmallItem
        )
    }
}

#[must_use]
pub fn clicks_required_for_unit(
    unit_value_micros: u64,
    log_value_micros: u64,
) -> Result<u64, FoodDivineError> {
    if unit_value_micros == 0 || log_value_micros == 0 {
        return Err(FoodDivineError::ZeroValue);
    }
    let scaled = u128::from(LOG_BASE_CLICKS_PER_UNIT)
        .checked_mul(u128::from(unit_value_micros))
        .ok_or(FoodDivineError::Overflow)?;
    let divisor = u128::from(log_value_micros);
    let rounded = scaled
        .checked_add(divisor - 1)
        .ok_or(FoodDivineError::Overflow)?
        / divisor;
    u64::try_from(rounded).map_err(|_| FoodDivineError::Overflow)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum BoundCargoPurpose {
    Construction { project_id: String, stage_index: u8 },
    Emergency { supply: EmergencySupplyKind },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PurposeBoundCargo {
    pub cargo_id: String,
    pub definition_id: String,
    pub quantity: u64,
    pub canonical_value_micros: u64,
    pub purpose: BoundCargoPurpose,
    pub provenance_player_id: String,
    pub created_at_real_ms: u64,
    pub site_id: String,
}

impl PurposeBoundCargo {
    #[must_use]
    pub const fn can_trade(&self) -> bool {
        false
    }

    #[must_use]
    pub const fn can_feed_hole(&self) -> bool {
        false
    }

    #[must_use]
    pub const fn can_return_to_general_stock(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContributionTarget {
    pub target_id: String,
    pub definition_id: String,
    pub contribution_kind: ContributionKind,
    pub purpose: BoundCargoPurpose,
    pub required_units: u64,
    pub created_units: u64,
    pub clicks_toward_next_unit: u64,
    pub clicks_per_unit: u64,
    pub unit_value_micros: u64,
    pub active_labor_remaining_seconds: u64,
}

impl ContributionTarget {
    #[must_use]
    pub fn remaining_click_capacity(&self) -> u64 {
        self.required_units
            .saturating_sub(self.created_units)
            .saturating_mul(self.clicks_per_unit)
            .saturating_sub(self.clicks_toward_next_unit)
            .min(self.active_labor_remaining_seconds)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClickLimiter {
    pub last_refill_real_ms: u64,
    pub tokens_milli: u32,
}

impl ClickLimiter {
    #[must_use]
    pub const fn full(now_real_ms: u64) -> Self {
        Self {
            last_refill_real_ms: now_real_ms,
            tokens_milli: CLICK_BURST_CAPACITY * 1_000,
        }
    }

    fn accept(&mut self, requested: u32, now_real_ms: u64) -> u32 {
        let elapsed = now_real_ms.saturating_sub(self.last_refill_real_ms);
        let refill = elapsed
            .saturating_mul(u64::from(CLICKS_PER_SECOND_PER_PLAYER))
            .min(u64::from(u32::MAX));
        self.tokens_milli = self
            .tokens_milli
            .saturating_add(refill as u32)
            .min(CLICK_BURST_CAPACITY * 1_000);
        self.last_refill_real_ms = now_real_ms;
        let available = self.tokens_milli / 1_000;
        let accepted = requested.min(available);
        self.tokens_milli -= accepted * 1_000;
        accepted
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContributionClickState {
    pub schema_version: u32,
    pub version: u64,
    pub target: ContributionTarget,
    pub player_limiters: BTreeMap<String, ClickLimiter>,
    pub cargo: BTreeMap<String, PurposeBoundCargo>,
    pub next_cargo_serial: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributionBatch {
    pub player_id: String,
    pub requested_clicks: u32,
    pub client_batch_window_ms: u64,
    pub now_real_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributionOutcome {
    pub accepted_clicks: u32,
    pub rate_limited_clicks: u32,
    pub labor_seconds_removed: u64,
    pub created_cargo_ids: Vec<String>,
}

impl ContributionClickState {
    pub fn accept_batch(
        &mut self,
        batch: ContributionBatch,
    ) -> Result<ContributionOutcome, FoodDivineError> {
        self.validate()?;
        validate_id(&batch.player_id)?;
        if batch.client_batch_window_ms != CLICK_BATCH_INTERVAL_MS {
            return Err(FoodDivineError::InvalidClickBatchWindow);
        }
        if !self.target.contribution_kind.is_eligible() {
            return Err(FoodDivineError::IneligibleContribution);
        }
        let limiter = self
            .player_limiters
            .entry(batch.player_id.clone())
            .or_insert_with(|| ClickLimiter::full(batch.now_real_ms));
        let rate_accepted = limiter.accept(batch.requested_clicks, batch.now_real_ms);
        let capacity = self
            .target
            .remaining_click_capacity()
            .min(u64::from(u32::MAX)) as u32;
        let accepted = rate_accepted.min(capacity);
        let mut created_cargo_ids = Vec::new();
        for _ in 0..accepted {
            self.target.active_labor_remaining_seconds -= 1;
            self.target.clicks_toward_next_unit += 1;
            if self.target.clicks_toward_next_unit == self.target.clicks_per_unit {
                self.target.clicks_toward_next_unit = 0;
                self.target.created_units += 1;
                let cargo_id = format!(
                    "divine_click:{}:{}",
                    self.target.target_id, self.next_cargo_serial
                );
                self.next_cargo_serial = self
                    .next_cargo_serial
                    .checked_add(1)
                    .ok_or(FoodDivineError::Overflow)?;
                self.cargo.insert(
                    cargo_id.clone(),
                    PurposeBoundCargo {
                        cargo_id: cargo_id.clone(),
                        definition_id: self.target.definition_id.clone(),
                        quantity: 1,
                        canonical_value_micros: self.target.unit_value_micros,
                        purpose: self.target.purpose.clone(),
                        provenance_player_id: batch.player_id.clone(),
                        created_at_real_ms: batch.now_real_ms,
                        site_id: HOLE_DELIVERY_APRON_SITE_ID.to_owned(),
                    },
                );
                created_cargo_ids.push(cargo_id);
            }
        }
        if accepted > 0 {
            self.version = self
                .version
                .checked_add(1)
                .ok_or(FoodDivineError::Overflow)?;
        }
        Ok(ContributionOutcome {
            accepted_clicks: accepted,
            rate_limited_clicks: batch.requested_clicks.saturating_sub(rate_accepted),
            labor_seconds_removed: u64::from(accepted),
            created_cargo_ids,
        })
    }

    pub fn validate(&self) -> Result<(), FoodDivineError> {
        if self.schema_version != FOOD_DIVINE_SCHEMA_VERSION
            || self.target.required_units == 0
            || self.target.clicks_per_unit == 0
            || self.target.unit_value_micros == 0
            || self.target.created_units > self.target.required_units
            || self.target.clicks_toward_next_unit >= self.target.clicks_per_unit
            || self
                .player_limiters
                .values()
                .any(|limiter| limiter.tokens_milli > CLICK_BURST_CAPACITY * 1_000)
            || self
                .cargo
                .iter()
                .any(|(id, cargo)| id != &cargo.cargo_id || cargo.quantity == 0)
        {
            return Err(FoodDivineError::MalformedState);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspirationWindow {
    pub player_id: String,
    pub activated_at_real_ms: u64,
    pub active_until_real_ms: u64,
    pub cooldown_until_real_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspirationState {
    pub schema_version: u32,
    pub version: u64,
    pub by_player: BTreeMap<String, InspirationWindow>,
}

impl InspirationState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: FOOD_DIVINE_SCHEMA_VERSION,
            version: 0,
            by_player: BTreeMap::new(),
        }
    }

    pub fn activate(
        &mut self,
        player_id: impl Into<String>,
        now_real_ms: u64,
    ) -> Result<(), FoodDivineError> {
        let player_id = player_id.into();
        validate_id(&player_id)?;
        if self.by_player.get(&player_id).is_some_and(|window| {
            now_real_ms < window.active_until_real_ms || now_real_ms < window.cooldown_until_real_ms
        }) {
            return Err(FoodDivineError::InspirationUnavailable);
        }
        let active_until_real_ms = now_real_ms
            .checked_add(INSPIRATION_DURATION_REAL_MS)
            .ok_or(FoodDivineError::Overflow)?;
        let cooldown_until_real_ms = now_real_ms
            .checked_add(INSPIRATION_COOLDOWN_REAL_MS)
            .ok_or(FoodDivineError::Overflow)?;
        self.by_player.insert(
            player_id.clone(),
            InspirationWindow {
                player_id,
                activated_at_real_ms: now_real_ms,
                active_until_real_ms,
                cooldown_until_real_ms,
            },
        );
        self.version = self
            .version
            .checked_add(1)
            .ok_or(FoodDivineError::Overflow)?;
        Ok(())
    }

    #[must_use]
    pub fn additive_effect_basis_points(&self, now_real_ms: u64) -> u64 {
        self.by_player
            .values()
            .filter(|window| now_real_ms < window.active_until_real_ms)
            .count() as u64
            * u64::from(INSPIRATION_EFFECT_BASIS_POINTS)
    }

    #[must_use]
    pub fn effective_stat_basis_points(&self, now_real_ms: u64) -> u64 {
        BASIS_POINTS_SCALE.saturating_add(self.additive_effect_basis_points(now_real_ms))
    }
}

impl Default for InspirationState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConstructionLaborStage {
    pub stage_index: u8,
    pub original_labor_seconds: u64,
    pub completed_labor_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConstructionMiracleInput {
    pub stage_index: u8,
    pub definition_id: String,
    pub quantity: u64,
    pub unit_value_micros: u64,
    pub missing_quantity_before: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstructionMiracleRequest {
    pub action_id: String,
    pub player_id: String,
    pub project_id: String,
    pub hole_feed_value_per_void_micros: u64,
    pub inputs: Vec<ConstructionMiracleInput>,
    pub now_real_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConstructionMiracleEvent {
    pub action_id: String,
    pub player_id: String,
    pub project_id: String,
    pub input_value_micros: u64,
    pub labor_seconds_removed: u64,
    pub cargo_ids: Vec<String>,
    pub committed_at_real_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DivineActionState {
    pub schema_version: u32,
    pub version: u64,
    pub construction_miracles: BTreeMap<String, ConstructionMiracleEvent>,
    pub rescue_events: BTreeMap<String, RescueEvent>,
    pub cargo: BTreeMap<String, PurposeBoundCargo>,
}

impl DivineActionState {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: FOOD_DIVINE_SCHEMA_VERSION,
            version: 0,
            construction_miracles: BTreeMap::new(),
            rescue_events: BTreeMap::new(),
            cargo: BTreeMap::new(),
        }
    }

    pub fn apply_construction_miracle(
        &mut self,
        void_insight_balance: &mut u64,
        stages: &mut [ConstructionLaborStage],
        request: ConstructionMiracleRequest,
    ) -> Result<ConstructionMiracleEvent, FoodDivineError> {
        self.validate()?;
        if let Some(existing) = self.construction_miracles.get(&request.action_id) {
            return Ok(existing.clone());
        }
        validate_id(&request.action_id)?;
        validate_id(&request.player_id)?;
        validate_id(&request.project_id)?;
        if *void_insight_balance < VOID_INSIGHT_PER_MIRACLE {
            return Err(FoodDivineError::InsufficientVoidInsight);
        }
        validate_labor_stages(stages)?;
        let required_value = request
            .hole_feed_value_per_void_micros
            .checked_mul(MIRACLE_INPUT_VALUE_MULTIPLIER)
            .ok_or(FoodDivineError::Overflow)?;
        let input_value = request.inputs.iter().try_fold(0_u64, |total, input| {
            validate_id(&input.definition_id)?;
            if input.quantity == 0
                || input.quantity > input.missing_quantity_before
                || input.unit_value_micros == 0
            {
                return Err(FoodDivineError::MiracleInputMismatch);
            }
            total
                .checked_add(
                    input
                        .quantity
                        .checked_mul(input.unit_value_micros)
                        .ok_or(FoodDivineError::Overflow)?,
                )
                .ok_or(FoodDivineError::Overflow)
        })?;
        if input_value != required_value {
            return Err(FoodDivineError::MiracleInputMismatch);
        }

        let original_total = stages.iter().try_fold(0_u64, |total, stage| {
            total
                .checked_add(stage.original_labor_seconds)
                .ok_or(FoodDivineError::Overflow)
        })?;
        let labor_budget = original_total.saturating_mul(MIRACLE_LABOR_REDUCTION_BASIS_POINTS)
            / BASIS_POINTS_SCALE;
        if labor_budget == 0 {
            return Err(FoodDivineError::NoActiveConstructionLabor);
        }
        let mut next_stages = stages.to_vec();
        let mut remaining_budget = labor_budget;
        next_stages.sort_by_key(|stage| stage.stage_index);
        for stage in &mut next_stages {
            if remaining_budget == 0 {
                break;
            }
            let remaining = stage
                .original_labor_seconds
                .saturating_sub(stage.completed_labor_seconds);
            let removed = remaining.min(remaining_budget);
            stage.completed_labor_seconds = stage.completed_labor_seconds.saturating_add(removed);
            remaining_budget -= removed;
        }
        let labor_seconds_removed = labor_budget - remaining_budget;
        if labor_seconds_removed == 0 {
            return Err(FoodDivineError::NoActiveConstructionLabor);
        }

        let mut next_cargo = Vec::new();
        for (serial, input) in request.inputs.iter().enumerate() {
            let cargo_id = format!("miracle:{}:{serial}", request.action_id);
            if self.cargo.contains_key(&cargo_id) {
                return Err(FoodDivineError::DuplicateId);
            }
            next_cargo.push(PurposeBoundCargo {
                cargo_id,
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
            });
        }

        *void_insight_balance -= VOID_INSIGHT_PER_MIRACLE;
        for updated in next_stages {
            let original = stages
                .iter_mut()
                .find(|stage| stage.stage_index == updated.stage_index)
                .ok_or(FoodDivineError::MalformedState)?;
            *original = updated;
        }
        let cargo_ids = next_cargo
            .iter()
            .map(|cargo| cargo.cargo_id.clone())
            .collect::<Vec<_>>();
        for cargo in next_cargo {
            self.cargo.insert(cargo.cargo_id.clone(), cargo);
        }
        let event = ConstructionMiracleEvent {
            action_id: request.action_id.clone(),
            player_id: request.player_id,
            project_id: request.project_id,
            input_value_micros: input_value,
            labor_seconds_removed,
            cargo_ids,
            committed_at_real_ms: request.now_real_ms,
        };
        self.construction_miracles
            .insert(request.action_id, event.clone());
        self.bump_version()?;
        Ok(event)
    }

    pub fn create_void_rescue(
        &mut self,
        void_insight_balance: &mut u64,
        action_id: impl Into<String>,
        player_id: impl Into<String>,
        supply: EmergencySupplyKind,
        living_resident_count: u64,
        evidence: EmergencyReportEvidence,
        now_real_ms: u64,
    ) -> Result<RescueEvent, FoodDivineError> {
        let action_id = action_id.into();
        if let Some(existing) = self.rescue_events.get(&action_id) {
            return Ok(existing.clone());
        }
        let player_id = player_id.into();
        validate_id(&action_id)?;
        validate_id(&player_id)?;
        if !evidence.control_visible(supply) {
            return Err(FoodDivineError::MissingRescueEvidence);
        }
        if *void_insight_balance < VOID_INSIGHT_PER_MIRACLE {
            return Err(FoodDivineError::InsufficientVoidInsight);
        }
        let quantity = living_resident_count
            .checked_mul(RESCUE_UNITS_PER_RESIDENT)
            .ok_or(FoodDivineError::Overflow)?;
        let cargo_id = format!("void_rescue:{action_id}");
        let event = RescueEvent {
            action_id: action_id.clone(),
            player_id: player_id.clone(),
            supply,
            quantity,
            void_insight_spent: VOID_INSIGHT_PER_MIRACLE,
            cargo_id: cargo_id.clone(),
            created_at_real_ms: now_real_ms,
        };
        *void_insight_balance -= VOID_INSIGHT_PER_MIRACLE;
        self.cargo.insert(
            cargo_id.clone(),
            PurposeBoundCargo {
                cargo_id,
                definition_id: supply.definition_id().to_owned(),
                quantity,
                canonical_value_micros: 0,
                purpose: BoundCargoPurpose::Emergency { supply },
                provenance_player_id: player_id,
                created_at_real_ms: now_real_ms,
                site_id: HOLE_DELIVERY_APRON_SITE_ID.to_owned(),
            },
        );
        self.rescue_events.insert(action_id, event.clone());
        self.bump_version()?;
        Ok(event)
    }

    fn bump_version(&mut self) -> Result<(), FoodDivineError> {
        self.version = self
            .version
            .checked_add(1)
            .ok_or(FoodDivineError::Overflow)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), FoodDivineError> {
        if self.schema_version != FOOD_DIVINE_SCHEMA_VERSION
            || self
                .construction_miracles
                .iter()
                .any(|(id, event)| id != &event.action_id)
            || self
                .rescue_events
                .iter()
                .any(|(id, event)| id != &event.action_id)
            || self
                .cargo
                .iter()
                .any(|(id, cargo)| id != &cargo.cargo_id || cargo.quantity == 0)
        {
            return Err(FoodDivineError::MalformedState);
        }
        Ok(())
    }
}

impl Default for DivineActionState {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_labor_stages(stages: &[ConstructionLaborStage]) -> Result<(), FoodDivineError> {
    if stages.is_empty()
        || stages.iter().any(|stage| {
            stage.original_labor_seconds == 0
                || stage.completed_labor_seconds > stage.original_labor_seconds
        })
        || stages
            .windows(2)
            .any(|pair| pair[0].stage_index >= pair[1].stage_index)
    {
        return Err(FoodDivineError::MalformedState);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmergencySupplyKind {
    DivineRation,
    DivineWater,
}

impl EmergencySupplyKind {
    #[must_use]
    pub const fn definition_id(self) -> &'static str {
        match self {
            Self::DivineRation => "divine_ration",
            Self::DivineWater => "divine_water",
        }
    }

    #[must_use]
    pub const fn default_food_permission(self) -> Option<FoodPermission> {
        match self {
            Self::DivineRation => Some(FoodPermission::Reserve),
            Self::DivineWater => None,
        }
    }

    #[must_use]
    pub const fn need_restored_basis_points(self) -> u16 {
        BASIS_POINTS_SCALE as u16
    }

    #[must_use]
    pub const fn expires(self) -> bool {
        false
    }

    #[must_use]
    pub const fn haul_priority(self) -> u8 {
        EMERGENCY_HAUL_PRIORITY
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmergencyReportEvidence {
    pub residents_dying_from_hunger: bool,
    pub residents_dying_from_thirst: bool,
}

impl EmergencyReportEvidence {
    #[must_use]
    pub const fn control_visible(self, supply: EmergencySupplyKind) -> bool {
        match supply {
            EmergencySupplyKind::DivineRation => self.residents_dying_from_hunger,
            EmergencySupplyKind::DivineWater => self.residents_dying_from_thirst,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RescueEvent {
    pub action_id: String,
    pub player_id: String,
    pub supply: EmergencySupplyKind,
    pub quantity: u64,
    pub void_insight_spent: u64,
    pub cargo_id: String,
    pub created_at_real_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoodDivineError {
    BlankId,
    DuplicateId,
    UnknownFood,
    GodCannotEditIndividualFood,
    ZeroValue,
    Overflow,
    IneligibleContribution,
    InvalidClickBatchWindow,
    InspirationUnavailable,
    InsufficientVoidInsight,
    MiracleInputMismatch,
    NoActiveConstructionLabor,
    MissingRescueEvidence,
    MalformedState,
}

impl std::fmt::Display for FoodDivineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "food/divine policy error: {self:?}")
    }
}

impl std::error::Error for FoodDivineError {}

fn validate_id(value: &str) -> Result<(), FoodDivineError> {
    if value.trim().is_empty() {
        Err(FoodDivineError::BlankId)
    } else {
        Ok(())
    }
}
