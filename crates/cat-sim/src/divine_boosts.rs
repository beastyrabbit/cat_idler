//! LAI.44 player-only specialized Divine Boost authority.
//!
//! Research and activation use the LAI.44 Void ledger. The leaf is bounded,
//! versioned, deterministic, and has no automated purchase path.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    planner_core::PlannerId,
    progression_research::{
        CurrencyCommitOutcome, CurrencyEventId, PlayerPartitionKey, ProgressionAuthority,
        VoidDebitPurpose, VoidInsight, VoidInsightLedger, VoidSpendRequest,
    },
    research_manifest::{
        ADDITIVE_TRACK_STAGE_COUNT, DIVINE_DURATION_STAGE_MAX_GAME_HOURS,
        DIVINE_ECONOMY_STAGE_DISCOUNT_BASIS_POINTS, ManifestEffect,
    },
};

pub const DIVINE_BOOST_SCHEMA_VERSION: u32 = 2;
pub const DIVINE_BOOST_EFFECT_BASIS_POINTS: i64 = 15_000;
pub const DIVINE_BOOST_BASE_DURATION_GAME_HOURS: u32 = 1;
pub const DIVINE_BOOST_DURATION_HOURS: [u32; 12] = [1, 2, 3, 4, 6, 8, 10, 12, 16, 18, 21, 24];
pub const DIVINE_ECONOMY_REDUCTION_PER_STAGE_PERCENT: u8 = 3;
pub const DIVINE_ECONOMY_MAX_REDUCTION_PERCENT: u8 = 33;
pub const MAX_DIVINE_BOOST_PURCHASES: usize = 512;
pub const MAX_DIVINE_BOOST_DRAIN_BATCH: usize = 64;
pub const MAX_DIVINE_BOOST_PLAYERS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivineBoostType {
    BountifulLabor,
    FleetPaws,
    InspiredWork,
    RestorativeGrace,
}

impl DivineBoostType {
    pub const ALL: [Self; 4] = [
        Self::BountifulLabor,
        Self::FleetPaws,
        Self::InspiredWork,
        Self::RestorativeGrace,
    ];

    #[must_use]
    pub const fn base_cost_per_hour(self) -> VoidInsight {
        match self {
            Self::BountifulLabor | Self::InspiredWork | Self::RestorativeGrace => {
                VoidInsight::from_micro(2_000_000)
            }
            Self::FleetPaws => VoidInsight::ONE,
        }
    }

    #[must_use]
    pub const fn effect_domains(self) -> &'static [&'static str] {
        match self {
            Self::BountifulLabor => &["raw_gathering", "carrying", "harvesting"],
            Self::FleetPaws => &["movement"],
            Self::InspiredWork => &["construction", "production"],
            Self::RestorativeGrace => &["healing"],
        }
    }
}

#[must_use]
pub const fn active_effect_factor(_boost_type: DivineBoostType) -> i64 {
    DIVINE_BOOST_EFFECT_BASIS_POINTS
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DivineBoostResearchStages {
    pub divine_duration_stage: u8,
    pub divine_economy_stage: u8,
}

impl DivineBoostResearchStages {
    pub fn from_manifest_effects<'a>(
        effects: impl IntoIterator<Item = &'a ManifestEffect>,
    ) -> Result<Self, DivineBoostError> {
        let mut stages = Self::default();
        for effect in effects {
            match effect {
                ManifestEffect::DivineDuration {
                    stage,
                    max_duration_game_hours,
                } => {
                    validate_manifest_duration_effect(*stage, *max_duration_game_hours)?;
                    stages.divine_duration_stage = stages.divine_duration_stage.max(*stage);
                }
                ManifestEffect::DivineEconomy {
                    stage,
                    discount_basis_points,
                } => {
                    validate_manifest_economy_effect(*stage, *discount_basis_points)?;
                    stages.divine_economy_stage = stages.divine_economy_stage.max(*stage);
                }
                ManifestEffect::CatalogPayload { .. }
                | ManifestEffect::Rehabilitation { .. }
                | ManifestEffect::Administration { .. } => {}
            }
        }
        Ok(stages)
    }

    #[must_use]
    pub const fn economy_reduction_percent(self) -> u8 {
        let reduction = self
            .divine_economy_stage
            .saturating_mul(DIVINE_ECONOMY_REDUCTION_PER_STAGE_PERCENT);
        if reduction > DIVINE_ECONOMY_MAX_REDUCTION_PERCENT {
            DIVINE_ECONOMY_MAX_REDUCTION_PERCENT
        } else {
            reduction
        }
    }

    fn validate(self) -> Result<(), DivineBoostError> {
        if usize::from(self.divine_duration_stage) > ADDITIVE_TRACK_STAGE_COUNT
            || usize::from(self.divine_economy_stage) > ADDITIVE_TRACK_STAGE_COUNT
        {
            return Err(DivineBoostError::InvalidResearchStages);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DivineBoostResearchEntitlements {
    pub unlocked_boosts: BTreeSet<DivineBoostType>,
    pub stages: DivineBoostResearchStages,
}

impl DivineBoostResearchEntitlements {
    fn validate(&self) -> Result<(), DivineBoostError> {
        self.stages.validate()?;
        if self
            .unlocked_boosts
            .iter()
            .any(|boost| !DivineBoostType::ALL.contains(boost))
        {
            return Err(DivineBoostError::MalformedResearchEntitlements);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnlockedBoostDurations {
    max_index: usize,
}

impl UnlockedBoostDurations {
    #[must_use]
    pub const fn for_stage(divine_duration_stage: u8) -> Self {
        let max_index = if divine_duration_stage as usize >= DIVINE_BOOST_DURATION_HOURS.len() {
            DIVINE_BOOST_DURATION_HOURS.len() - 1
        } else {
            divine_duration_stage as usize
        };
        Self { max_index }
    }

    #[must_use]
    pub const fn durations_hours(self) -> [u32; 12] {
        DIVINE_BOOST_DURATION_HOURS
    }

    #[must_use]
    pub fn contains(self, duration_hours: u32) -> bool {
        DIVINE_BOOST_DURATION_HOURS
            .iter()
            .position(|duration| *duration == duration_hours)
            .is_some_and(|index| index <= self.max_index)
    }
}

pub fn boost_cost(
    boost_type: DivineBoostType,
    duration_hours: u32,
    stages: DivineBoostResearchStages,
) -> Result<VoidInsight, DivineBoostError> {
    stages.validate()?;
    if !UnlockedBoostDurations::for_stage(stages.divine_duration_stage).contains(duration_hours) {
        return Err(DivineBoostError::DurationLocked);
    }
    let base = u128::from(boost_type.base_cost_per_hour().micro());
    let multiplier_percent = u128::from(100 - stages.economy_reduction_percent());
    let numerator = base
        .checked_mul(u128::from(duration_hours))
        .and_then(|value| value.checked_mul(multiplier_percent))
        .ok_or(DivineBoostError::ArithmeticOverflow)?;
    let charged = numerator.div_ceil(100);
    let charged = u64::try_from(charged).map_err(|_| DivineBoostError::ArithmeticOverflow)?;
    Ok(VoidInsight::from_micro(charged))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DivineBoostPurchaseId(PlannerId);

impl DivineBoostPurchaseId {
    #[must_use]
    pub fn derive(colony_id: &PlannerId, player_id: &PlannerId, player_sequence: u64) -> Self {
        Self(PlannerId::derive(
            "divine_boost_purchase",
            [
                colony_id.as_str(),
                player_id.as_str(),
                &player_sequence.to_string(),
            ],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DivineBoostActor {
    Player { player_id: PlannerId },
    Automated { actor_id: PlannerId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivineBoostAuthorization {
    pub actor: DivineBoostActor,
    pub authenticated_player_id: Option<PlannerId>,
    pub owns_colony: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActiveDivineBoost {
    pub boost_type: DivineBoostType,
    pub partition: PlayerPartitionKey,
    pub player_sequence: u64,
    pub activated_tick: u64,
    pub expires_tick: u64,
    pub duration_hours: u32,
    pub ticks_per_game_hour: u64,
    pub paid_cost: VoidInsight,
    pub committed_research_stages: DivineBoostResearchStages,
    pub purchase_id: DivineBoostPurchaseId,
    pub void_event_id: CurrencyEventId,
    pub committed_boost_version: u64,
}

impl ActiveDivineBoost {
    fn validate(&self) -> Result<(), DivineBoostError> {
        let expected_cost = boost_cost(
            self.boost_type,
            self.duration_hours,
            self.committed_research_stages,
        )
        .map_err(|_| DivineBoostError::MalformedPersistence)?;
        let expected_expiry = exact_expiry_tick(
            self.activated_tick,
            self.duration_hours,
            self.ticks_per_game_hour,
        )
        .map_err(|_| DivineBoostError::MalformedPersistence)?;
        if self.player_sequence == 0
            || self.paid_cost != expected_cost
            || self.expires_tick != expected_expiry
            || self.purchase_id
                != DivineBoostPurchaseId::derive(
                    &self.partition.colony_id,
                    &self.partition.player_id,
                    self.player_sequence,
                )
        {
            return Err(DivineBoostError::MalformedPersistence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DivineBoostPurchaseEvent {
    pub id: DivineBoostPurchaseId,
    pub boost_type: DivineBoostType,
    pub partition: PlayerPartitionKey,
    pub player_sequence: u64,
    pub activated_tick: u64,
    pub expires_tick: u64,
    pub duration_hours: u32,
    pub ticks_per_game_hour: u64,
    pub paid_cost: VoidInsight,
    pub committed_research_stages: DivineBoostResearchStages,
    pub void_event_id: CurrencyEventId,
    pub request_fingerprint: u64,
    pub committed_boost_version: u64,
    pub committed_void_version: u64,
}

impl DivineBoostPurchaseEvent {
    fn validate(&self) -> Result<(), DivineBoostError> {
        let active_shape = ActiveDivineBoost {
            boost_type: self.boost_type,
            partition: self.partition.clone(),
            player_sequence: self.player_sequence,
            activated_tick: self.activated_tick,
            expires_tick: self.expires_tick,
            duration_hours: self.duration_hours,
            ticks_per_game_hour: self.ticks_per_game_hour,
            paid_cost: self.paid_cost,
            committed_research_stages: self.committed_research_stages,
            purchase_id: self.id.clone(),
            void_event_id: self.void_event_id.clone(),
            committed_boost_version: self.committed_boost_version,
        };
        active_shape.validate()?;
        if self.request_fingerprint != event_fingerprint(self)
            || self.committed_boost_version == 0
            || self.committed_void_version == 0
        {
            return Err(DivineBoostError::MalformedPersistence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DivineBoostState {
    pub schema_version: u32,
    pub colony_id: PlannerId,
    pub version: u64,
    pub active: BTreeMap<DivineBoostType, ActiveDivineBoost>,
    pub purchases: BTreeMap<DivineBoostPurchaseId, DivineBoostPurchaseEvent>,
    pub retired_purchase_through: BTreeMap<PlannerId, u64>,
}

impl DivineBoostState {
    #[must_use]
    pub fn new(colony_id: PlannerId) -> Self {
        Self {
            schema_version: DIVINE_BOOST_SCHEMA_VERSION,
            colony_id,
            version: 0,
            active: BTreeMap::new(),
            purchases: BTreeMap::new(),
            retired_purchase_through: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn active_boosts(&self) -> &BTreeMap<DivineBoostType, ActiveDivineBoost> {
        &self.active
    }

    #[must_use]
    pub fn active(&self, boost_type: DivineBoostType) -> Option<&ActiveDivineBoost> {
        self.active.get(&boost_type)
    }

    pub fn purchase(
        &mut self,
        ledger: &mut VoidInsightLedger,
        progression: &ProgressionAuthority,
        request: DivineBoostPurchaseRequest,
    ) -> Result<DivineBoostOutcome, DivineBoostError> {
        let player_id = self.validate_purchase_boundary(
            ledger,
            &request,
            Some(&progression.partition.colony_id),
        )?;
        let research = research_entitlements(progression)?;
        self.purchase_validated(ledger, &research, request, &player_id)
    }

    /// Purchase against entitlements derived by another canonical research
    /// authority. This keeps the legacy [`ProgressionAuthority`] entry point
    /// intact while allowing the LAI.58 authority to share this exact boost
    /// state and Void ledger without constructing a shadow progression state.
    pub fn purchase_with_entitlements(
        &mut self,
        ledger: &mut VoidInsightLedger,
        entitlements: &DivineBoostResearchEntitlements,
        request: DivineBoostPurchaseRequest,
    ) -> Result<DivineBoostOutcome, DivineBoostError> {
        let player_id = self.validate_purchase_boundary(ledger, &request, None)?;
        entitlements.validate()?;
        self.purchase_validated(ledger, entitlements, request, &player_id)
    }

    /// Return the sole canonical next purchase sequence for this player. The
    /// result accounts for drained receipts and every retained purchase.
    pub fn next_player_purchase_sequence(
        &self,
        player_id: &PlannerId,
    ) -> Result<u64, DivineBoostError> {
        self.validate()?;
        canonical_next_player_sequence(self, player_id)
    }

    fn validate_purchase_boundary(
        &self,
        ledger: &VoidInsightLedger,
        request: &DivineBoostPurchaseRequest,
        additional_colony_id: Option<&PlannerId>,
    ) -> Result<PlannerId, DivineBoostError> {
        self.validate()?;
        request.validate()?;
        let player_id = authorized_player(request)?;
        if request.partition.colony_id != self.colony_id
            || request.partition.colony_id != ledger.partition.colony_id
            || additional_colony_id
                .is_some_and(|colony_id| &request.partition.colony_id != colony_id)
            || player_id != &request.partition.player_id
        {
            return Err(DivineBoostError::PartitionMismatch);
        }
        Ok(player_id.clone())
    }

    fn purchase_validated(
        &mut self,
        ledger: &mut VoidInsightLedger,
        research: &DivineBoostResearchEntitlements,
        request: DivineBoostPurchaseRequest,
        player_id: &PlannerId,
    ) -> Result<DivineBoostOutcome, DivineBoostError> {
        let retired = self
            .retired_purchase_through
            .get(player_id)
            .copied()
            .unwrap_or(0);
        if request.player_sequence <= retired {
            return Ok(DivineBoostOutcome::RetiredReplay);
        }
        if let Some(existing) = self.purchases.get(&request.id) {
            return if event_matches_request(existing, &request) {
                Ok(DivineBoostOutcome::AlreadyCommitted)
            } else {
                Err(DivineBoostError::PurchaseIdConflict)
            };
        }
        let expected_sequence = canonical_next_player_sequence(self, player_id)?;
        if request.player_sequence != expected_sequence {
            return Err(DivineBoostError::NonCanonicalSequence);
        }
        if request.expected_boost_version != self.version {
            return Err(DivineBoostError::StaleBoostVersion);
        }
        if self.purchases.len() >= MAX_DIVINE_BOOST_PURCHASES {
            return Err(DivineBoostError::Backpressure);
        }
        if self
            .active
            .get(&request.boost_type)
            .is_some_and(|active| request.activated_tick < active.expires_tick)
        {
            return Err(DivineBoostError::ActiveSameType);
        }
        if !research.unlocked_boosts.contains(&request.boost_type) {
            return Err(DivineBoostError::BoostLocked);
        }

        let paid_cost = boost_cost(request.boost_type, request.duration_hours, research.stages)?;
        let expires_tick = exact_expiry_tick(
            request.activated_tick,
            request.duration_hours,
            request.ticks_per_game_hour,
        )?;
        let committed_boost_version = self
            .version
            .checked_add(1)
            .ok_or(DivineBoostError::ArithmeticOverflow)?;
        let void_event_id = CurrencyEventId::derive(
            "divine_boost_activation",
            &request.partition.colony_id,
            request.id.as_str(),
        );
        let mut next_state = self.clone();
        let mut next_ledger = ledger.clone();
        let event_seed = DivineBoostPurchaseEvent {
            id: request.id.clone(),
            boost_type: request.boost_type,
            partition: request.partition.clone(),
            player_sequence: request.player_sequence,
            activated_tick: request.activated_tick,
            expires_tick,
            duration_hours: request.duration_hours,
            ticks_per_game_hour: request.ticks_per_game_hour,
            paid_cost,
            committed_research_stages: research.stages,
            void_event_id: void_event_id.clone(),
            request_fingerprint: 0,
            committed_boost_version,
            committed_void_version: 0,
        };
        let fingerprint = event_fingerprint(&event_seed);
        let debit_outcome = next_ledger
            .debit(VoidSpendRequest {
                id: void_event_id.clone(),
                amount: paid_cost,
                purpose: VoidDebitPurpose::BoostActivation,
                expected_version: request.expected_void_version,
                fingerprint,
            })
            .map_err(|_| DivineBoostError::CurrencyRejected)?;
        let committed_void_version = next_ledger.version;
        let event = DivineBoostPurchaseEvent {
            request_fingerprint: fingerprint,
            committed_void_version,
            ..event_seed
        };
        let active = ActiveDivineBoost {
            boost_type: request.boost_type,
            partition: request.partition,
            player_sequence: request.player_sequence,
            activated_tick: request.activated_tick,
            expires_tick,
            duration_hours: request.duration_hours,
            ticks_per_game_hour: request.ticks_per_game_hour,
            paid_cost,
            committed_research_stages: research.stages,
            purchase_id: request.id.clone(),
            void_event_id,
            committed_boost_version,
        };
        next_state.purchases.insert(request.id, event);
        next_state.active.insert(request.boost_type, active);
        next_state.version = committed_boost_version;
        next_state.validate()?;
        *self = next_state;
        *ledger = next_ledger;
        Ok(match debit_outcome {
            CurrencyCommitOutcome::Committed => DivineBoostOutcome::Committed,
            CurrencyCommitOutcome::AlreadyCommitted => DivineBoostOutcome::AlreadyCommitted,
            CurrencyCommitOutcome::RetiredReplay => DivineBoostOutcome::RetiredReplay,
        })
    }

    pub fn expire_due(
        &mut self,
        now_tick: u64,
    ) -> Result<Vec<ExpiredDivineBoost>, DivineBoostError> {
        let mut next = self.clone();
        let expired_types = next
            .active
            .iter()
            .filter_map(|(boost_type, active)| {
                (now_tick >= active.expires_tick).then_some(*boost_type)
            })
            .collect::<Vec<_>>();
        let mut expired = Vec::new();
        for boost_type in expired_types {
            let active = next
                .active
                .remove(&boost_type)
                .ok_or(DivineBoostError::MalformedPersistence)?;
            next.version = next
                .version
                .checked_add(1)
                .ok_or(DivineBoostError::ArithmeticOverflow)?;
            expired.push(ExpiredDivineBoost {
                boost_type,
                purchase_id: active.purchase_id,
                expired_at_tick: active.expires_tick,
            });
        }
        next.validate()?;
        *self = next;
        Ok(expired)
    }

    pub fn drain_expired_purchase_receipts(
        &mut self,
        ledger: &mut VoidInsightLedger,
        player_id: &PlannerId,
        limit: usize,
    ) -> Result<usize, DivineBoostError> {
        if limit == 0 || limit > MAX_DIVINE_BOOST_DRAIN_BATCH {
            return Err(DivineBoostError::CapacityExceeded);
        }
        let mut next_state = self.clone();
        let mut next_ledger = ledger.clone();
        let mut drained = 0;
        for _ in 0..limit {
            let sequence = next_state
                .retired_purchase_through
                .get(player_id)
                .copied()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(DivineBoostError::ArithmeticOverflow)?;
            let Some(event) = next_state
                .purchases
                .values()
                .find(|event| {
                    &event.partition.player_id == player_id && event.player_sequence == sequence
                })
                .cloned()
            else {
                break;
            };
            if next_state
                .active
                .values()
                .any(|active| active.purchase_id == event.id)
            {
                break;
            }
            next_state.purchases.remove(&event.id);
            next_ledger
                .drain_spend_receipts(&BTreeSet::from([event.void_event_id]))
                .map_err(|_| DivineBoostError::CurrencyRejected)?;
            next_state
                .retired_purchase_through
                .insert(player_id.clone(), sequence);
            drained += 1;
        }
        next_state.validate()?;
        *self = next_state;
        *ledger = next_ledger;
        Ok(drained)
    }

    fn validate(&self) -> Result<(), DivineBoostError> {
        if self.schema_version != DIVINE_BOOST_SCHEMA_VERSION
            || self.active.len() > DivineBoostType::ALL.len()
            || self.purchases.len() > MAX_DIVINE_BOOST_PURCHASES
            || self.retired_purchase_through.len() > MAX_DIVINE_BOOST_PLAYERS
        {
            return Err(DivineBoostError::MalformedPersistence);
        }
        for (id, event) in &self.purchases {
            event.validate()?;
            if id != &event.id
                || event.partition.colony_id != self.colony_id
                || event.player_sequence
                    <= self
                        .retired_purchase_through
                        .get(&event.partition.player_id)
                        .copied()
                        .unwrap_or(0)
            {
                return Err(DivineBoostError::MalformedPersistence);
            }
        }
        if self
            .purchases
            .values()
            .any(|event| event.committed_boost_version > self.version)
        {
            return Err(DivineBoostError::MalformedPersistence);
        }
        for (boost_type, active) in &self.active {
            active.validate()?;
            let event = self
                .purchases
                .get(&active.purchase_id)
                .ok_or(DivineBoostError::MalformedPersistence)?;
            if *boost_type != active.boost_type
                || event.boost_type != active.boost_type
                || event.partition != active.partition
                || event.player_sequence != active.player_sequence
                || event.activated_tick != active.activated_tick
                || event.expires_tick != active.expires_tick
                || event.duration_hours != active.duration_hours
                || event.ticks_per_game_hour != active.ticks_per_game_hour
                || event.paid_cost != active.paid_cost
                || event.committed_research_stages != active.committed_research_stages
                || event.void_event_id != active.void_event_id
                || event.committed_boost_version != active.committed_boost_version
            {
                return Err(DivineBoostError::MalformedPersistence);
            }
        }
        let mut sequences = BTreeMap::<&PlannerId, BTreeSet<u64>>::new();
        for event in self.purchases.values() {
            sequences
                .entry(&event.partition.player_id)
                .or_default()
                .insert(event.player_sequence);
        }
        for (player_id, sequences) in sequences {
            let retired = self
                .retired_purchase_through
                .get(player_id)
                .copied()
                .unwrap_or(0);
            let sequence_count = u64::try_from(sequences.len())
                .map_err(|_| DivineBoostError::MalformedPersistence)?;
            let first = retired
                .checked_add(1)
                .ok_or(DivineBoostError::MalformedPersistence)?;
            let last = retired
                .checked_add(sequence_count)
                .ok_or(DivineBoostError::MalformedPersistence)?;
            let expected = (first..=last).collect::<BTreeSet<_>>();
            if sequences != expected {
                return Err(DivineBoostError::MalformedPersistence);
            }
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedDivineBoostState {
    schema_version: u32,
    colony_id: PlannerId,
    version: u64,
    active: BTreeMap<DivineBoostType, ActiveDivineBoost>,
    purchases: BTreeMap<DivineBoostPurchaseId, DivineBoostPurchaseEvent>,
    retired_purchase_through: BTreeMap<PlannerId, u64>,
}

impl<'de> Deserialize<'de> for DivineBoostState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UncheckedDivineBoostState::deserialize(deserializer)?;
        let state = Self {
            schema_version: raw.schema_version,
            colony_id: raw.colony_id,
            version: raw.version,
            active: raw.active,
            purchases: raw.purchases,
            retired_purchase_through: raw.retired_purchase_through,
        };
        state.validate().map_err(serde::de::Error::custom)?;
        Ok(state)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivineBoostPurchaseRequest {
    pub id: DivineBoostPurchaseId,
    pub partition: PlayerPartitionKey,
    pub player_sequence: u64,
    pub authorization: DivineBoostAuthorization,
    pub boost_type: DivineBoostType,
    pub duration_hours: u32,
    pub expected_boost_version: u64,
    pub expected_void_version: u64,
    pub activated_tick: u64,
    pub ticks_per_game_hour: u64,
}

impl DivineBoostPurchaseRequest {
    fn validate(&self) -> Result<(), DivineBoostError> {
        if self.player_sequence == 0
            || self.ticks_per_game_hour == 0
            || self.id
                != DivineBoostPurchaseId::derive(
                    &self.partition.colony_id,
                    &self.partition.player_id,
                    self.player_sequence,
                )
        {
            return Err(DivineBoostError::MalformedRequest);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiredDivineBoost {
    pub boost_type: DivineBoostType,
    pub purchase_id: DivineBoostPurchaseId,
    pub expired_at_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivineBoostOutcome {
    Committed,
    AlreadyCommitted,
    RetiredReplay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivineBoostError {
    Unauthorized,
    PartitionMismatch,
    BoostLocked,
    DurationLocked,
    InvalidResearchStages,
    MalformedResearchEffect,
    MalformedResearchEntitlements,
    ActiveSameType,
    StaleBoostVersion,
    PurchaseIdConflict,
    NonCanonicalSequence,
    MalformedRequest,
    MalformedPersistence,
    CurrencyRejected,
    Backpressure,
    CapacityExceeded,
    TickOverflow,
    ArithmeticOverflow,
}

impl std::fmt::Display for DivineBoostError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Divine Boost request rejected ({self:?})")
    }
}

impl std::error::Error for DivineBoostError {}

fn authorized_player(request: &DivineBoostPurchaseRequest) -> Result<&PlannerId, DivineBoostError> {
    let DivineBoostActor::Player { player_id } = &request.authorization.actor else {
        return Err(DivineBoostError::Unauthorized);
    };
    if !request.authorization.owns_colony
        || request.authorization.authenticated_player_id.as_ref() != Some(player_id)
    {
        return Err(DivineBoostError::Unauthorized);
    }
    Ok(player_id)
}

fn canonical_next_player_sequence(
    state: &DivineBoostState,
    player_id: &PlannerId,
) -> Result<u64, DivineBoostError> {
    let retired = state
        .retired_purchase_through
        .get(player_id)
        .copied()
        .unwrap_or(0);
    retired
        .checked_add(
            u64::try_from(
                state
                    .purchases
                    .values()
                    .filter(|event| &event.partition.player_id == player_id)
                    .count(),
            )
            .map_err(|_| DivineBoostError::ArithmeticOverflow)?,
        )
        .and_then(|value| value.checked_add(1))
        .ok_or(DivineBoostError::ArithmeticOverflow)
}

fn validate_manifest_stage(stage: u8) -> Result<usize, DivineBoostError> {
    if stage == 0 || usize::from(stage) > ADDITIVE_TRACK_STAGE_COUNT {
        return Err(DivineBoostError::MalformedResearchEffect);
    }
    Ok(usize::from(stage - 1))
}

fn validate_manifest_duration_effect(
    stage: u8,
    max_duration_game_hours: u8,
) -> Result<(), DivineBoostError> {
    let index = validate_manifest_stage(stage)?;
    if max_duration_game_hours != DIVINE_DURATION_STAGE_MAX_GAME_HOURS[index]
        || u32::from(max_duration_game_hours) != DIVINE_BOOST_DURATION_HOURS[usize::from(stage)]
    {
        return Err(DivineBoostError::MalformedResearchEffect);
    }
    Ok(())
}

fn validate_manifest_economy_effect(
    stage: u8,
    discount_basis_points: u16,
) -> Result<(), DivineBoostError> {
    let index = validate_manifest_stage(stage)?;
    if discount_basis_points != DIVINE_ECONOMY_STAGE_DISCOUNT_BASIS_POINTS[index] {
        return Err(DivineBoostError::MalformedResearchEffect);
    }
    Ok(())
}

fn event_matches_request(
    event: &DivineBoostPurchaseEvent,
    request: &DivineBoostPurchaseRequest,
) -> bool {
    event.id == request.id
        && event.boost_type == request.boost_type
        && event.partition == request.partition
        && event.player_sequence == request.player_sequence
        && event.activated_tick == request.activated_tick
        && event.duration_hours == request.duration_hours
        && event.ticks_per_game_hour == request.ticks_per_game_hour
}

fn research_entitlements(
    progression: &ProgressionAuthority,
) -> Result<DivineBoostResearchEntitlements, DivineBoostError> {
    let mut entitlements = DivineBoostResearchEntitlements {
        unlocked_boosts: BTreeSet::new(),
        stages: DivineBoostResearchStages::default(),
    };
    for study_id in &progression.owned_studies {
        let id = study_id.as_str();
        let unlocked = match id {
            "divine_boost_bountiful_labor" => Some(DivineBoostType::BountifulLabor),
            "divine_boost_fleet_paws" => Some(DivineBoostType::FleetPaws),
            "divine_boost_inspired_work" => Some(DivineBoostType::InspiredWork),
            "divine_boost_restorative_grace" => Some(DivineBoostType::RestorativeGrace),
            _ => None,
        };
        if let Some(boost) = unlocked {
            entitlements.unlocked_boosts.insert(boost);
        }
        if let Some(stage) = id
            .strip_prefix("divine_duration_")
            .and_then(|stage| stage.parse::<u8>().ok())
        {
            entitlements.stages.divine_duration_stage =
                entitlements.stages.divine_duration_stage.max(stage);
        }
        if let Some(stage) = id
            .strip_prefix("divine_economy_")
            .and_then(|stage| stage.parse::<u8>().ok())
        {
            entitlements.stages.divine_economy_stage =
                entitlements.stages.divine_economy_stage.max(stage);
        }
    }
    entitlements.validate()?;
    Ok(entitlements)
}

fn event_fingerprint(event: &DivineBoostPurchaseEvent) -> u64 {
    let mut value = 0xcbf2_9ce4_8422_2325_u64;
    for text in [
        event.id.as_str(),
        event.partition.colony_id.as_str(),
        event.partition.player_id.as_str(),
        match event.boost_type {
            DivineBoostType::BountifulLabor => "bountiful_labor",
            DivineBoostType::FleetPaws => "fleet_paws",
            DivineBoostType::InspiredWork => "inspired_work",
            DivineBoostType::RestorativeGrace => "restorative_grace",
        },
    ] {
        for byte in text.as_bytes() {
            value ^= u64::from(*byte);
            value = value.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    for number in [
        event.player_sequence,
        event.activated_tick,
        event.expires_tick,
        u64::from(event.duration_hours),
        event.ticks_per_game_hour,
        event.paid_cost.micro(),
        u64::from(event.committed_research_stages.divine_duration_stage),
        u64::from(event.committed_research_stages.divine_economy_stage),
    ] {
        for byte in number.to_le_bytes() {
            value ^= u64::from(byte);
            value = value.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    value
}

fn exact_expiry_tick(
    activated_tick: u64,
    duration_hours: u32,
    ticks_per_game_hour: u64,
) -> Result<u64, DivineBoostError> {
    let duration_ticks = u64::from(duration_hours)
        .checked_mul(ticks_per_game_hour)
        .ok_or(DivineBoostError::TickOverflow)?;
    activated_tick
        .checked_add(duration_ticks)
        .ok_or(DivineBoostError::TickOverflow)
}
