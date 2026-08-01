//! Report-safe Divine Boost offers and emergency-rescue witnesses.
//!
//! This leaf turns server-owned authority snapshots into bounded, canonical
//! choices. A client selects only an opaque offer/witness ID; the server adds
//! authentication, sequence, and time before handing the resolved request to
//! the existing Hole, boost, research, and Void authorities. No balance,
//! receipt, research entitlement, boost, or cargo state is duplicated here.

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    divine_boosts::{
        DivineBoostActor, DivineBoostAuthorization, DivineBoostOutcome, DivineBoostPurchaseId,
        DivineBoostPurchaseRequest, DivineBoostResearchEntitlements, DivineBoostResearchStages,
        DivineBoostState, DivineBoostType, UnlockedBoostDurations, boost_cost,
    },
    divine_hole_authority::{
        DivineHoleAuthority, EmergencyRescueRequest, VoidAction, VoidActionEnvelope,
    },
    family_authority::FAMILY_AUTHORITY_MAX_CATS,
    food_divine_policy::EmergencySupplyKind,
    planner_core::PlannerId,
    progression_research::{
        BOOST_RESEARCH_STAGE_COUNT, PlayerPartitionKey, ProgressionCatalog, SpecializedBoost,
        StudyKind, VoidInsight, VoidInsightLedger,
    },
    research_authority::{RESEARCH_AUTHORITY_SCHEMA_VERSION, ResearchAuthority},
};

pub const DIVINE_ACTION_OFFER_SCHEMA_VERSION: u32 = 2;
pub const MAX_DIVINE_BOOST_OFFERS: usize =
    DivineBoostType::ALL.len() * crate::divine_boosts::DIVINE_BOOST_DURATION_HOURS.len();
pub const MAX_EMERGENCY_RESCUE_WITNESSES: usize = 2;
pub const MAX_REPORTED_LIVING_RESIDENTS: u64 = FAMILY_AUTHORITY_MAX_CATS as u64;
const MAX_PARTITION_ID_BYTES: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DivineBoostOfferEpoch {
    pub hole_authority_version: u64,
    pub boost_authority_version: u64,
    pub research_authority_version: u64,
    pub void_ledger_version: u64,
}

impl DivineBoostOfferEpoch {
    pub fn capture(
        partition: &PlayerPartitionKey,
        hole: &DivineHoleAuthority,
        boosts: &DivineBoostState,
        research: &ResearchAuthority,
    ) -> Result<Self, DivineActionOfferError> {
        validate_partition(partition)?;
        if research.schema_version != RESEARCH_AUTHORITY_SCHEMA_VERSION {
            return Err(DivineActionOfferError::MalformedResearchAuthority);
        }
        if hole.binding.colony_id != partition.colony_id
            || boosts.colony_id != partition.colony_id
            || research.colony_id != partition.colony_id
            || research.void.partition.colony_id != partition.colony_id
        {
            return Err(DivineActionOfferError::PartitionMismatch);
        }
        Ok(Self {
            hole_authority_version: hole.version,
            boost_authority_version: boosts.version,
            research_authority_version: research.version,
            void_ledger_version: research.void.version,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DivineBoostOfferId(PlannerId);

impl DivineBoostOfferId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn derive(
        partition: &PlayerPartitionKey,
        boost_type: DivineBoostType,
        duration_hours: u32,
        stages: DivineBoostResearchStages,
        exact_cost: VoidInsight,
        epoch: DivineBoostOfferEpoch,
        player_sequence: u64,
    ) -> Self {
        Self(PlannerId::derive(
            "divine_boost_offer_v2",
            [
                partition.colony_id.as_str(),
                partition.player_id.as_str(),
                boost_type_token(boost_type),
                &duration_hours.to_string(),
                &stages.divine_duration_stage.to_string(),
                &stages.divine_economy_stage.to_string(),
                &exact_cost.micro().to_string(),
                &epoch.hole_authority_version.to_string(),
                &epoch.boost_authority_version.to_string(),
                &epoch.research_authority_version.to_string(),
                &epoch.void_ledger_version.to_string(),
                &player_sequence.to_string(),
            ],
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DivineBoostOffer {
    pub id: DivineBoostOfferId,
    pub boost_type: DivineBoostType,
    pub duration_hours: u32,
    pub researched_stages: DivineBoostResearchStages,
    pub exact_cost: VoidInsight,
    pub player_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DivineBoostOfferCatalog {
    pub schema_version: u32,
    pub partition: PlayerPartitionKey,
    pub epoch: DivineBoostOfferEpoch,
    pub entitlements: DivineBoostResearchEntitlements,
    pub next_player_purchase_sequence: u64,
    pub offers: Vec<DivineBoostOffer>,
}

impl DivineBoostOfferCatalog {
    pub fn capture(
        partition: PlayerPartitionKey,
        hole: &DivineHoleAuthority,
        boosts: &DivineBoostState,
        research: &ResearchAuthority,
    ) -> Result<Self, DivineActionOfferError> {
        let entitlements = research_boost_entitlements(research)?;
        let epoch = DivineBoostOfferEpoch::capture(&partition, hole, boosts, research)?;
        let next_player_purchase_sequence = boosts
            .next_player_purchase_sequence(&partition.player_id)
            .map_err(DivineActionOfferError::Boost)?;
        Self::new(
            partition,
            entitlements,
            epoch,
            next_player_purchase_sequence,
        )
    }

    pub fn new(
        partition: PlayerPartitionKey,
        entitlements: DivineBoostResearchEntitlements,
        epoch: DivineBoostOfferEpoch,
        next_player_purchase_sequence: u64,
    ) -> Result<Self, DivineActionOfferError> {
        validate_partition(&partition)?;
        validate_entitlements(&entitlements)?;
        if next_player_purchase_sequence == 0 {
            return Err(DivineActionOfferError::InvalidPurchaseSequence);
        }
        let offers = canonical_boost_offers(
            &partition,
            &entitlements,
            epoch,
            next_player_purchase_sequence,
        )?;
        let catalog = Self {
            schema_version: DIVINE_ACTION_OFFER_SCHEMA_VERSION,
            partition,
            epoch,
            entitlements,
            next_player_purchase_sequence,
            offers,
        };
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn decode_strict(json: &str) -> Result<Self, DivineActionOfferError> {
        serde_json::from_str(json).map_err(|_| DivineActionOfferError::MalformedPersistence)
    }

    pub fn canonical_json(&self) -> Result<String, DivineActionOfferError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| DivineActionOfferError::MalformedPersistence)
    }

    /// Lookup by the opaque wire-safe string. The caller never supplies boost
    /// type, duration, research stages, or cost.
    #[must_use]
    pub fn offer_by_id(&self, offer_id: &str) -> Option<&DivineBoostOffer> {
        self.offers
            .iter()
            .find(|offer| offer.id.as_str() == offer_id)
    }

    /// Resolve an opaque client selection using only current server-owned
    /// authorities and server-supplied authentication/clock values.
    pub fn resolve_activation(
        &self,
        offer_id: &DivineBoostOfferId,
        hole: &DivineHoleAuthority,
        boosts: &DivineBoostState,
        research: &ResearchAuthority,
        trusted: TrustedBoostActivation,
    ) -> Result<DivineBoostPurchaseRequest, DivineActionOfferError> {
        self.resolve_activation_by_id(offer_id.as_str(), hole, boosts, research, trusted)
    }

    /// String-bound resolution for protocol adapters. Only the opaque offer ID
    /// is selected by a client; sequence, authorization, clock, and tick rate
    /// remain trusted server context.
    pub fn resolve_activation_by_id(
        &self,
        offer_id: &str,
        hole: &DivineHoleAuthority,
        boosts: &DivineBoostState,
        research: &ResearchAuthority,
        trusted: TrustedBoostActivation,
    ) -> Result<DivineBoostPurchaseRequest, DivineActionOfferError> {
        self.validate()?;
        let current_entitlements = research_boost_entitlements(research)?;
        let current_epoch =
            DivineBoostOfferEpoch::capture(&self.partition, hole, boosts, research)?;
        let offer = self
            .offer_by_id(offer_id)
            .ok_or(DivineActionOfferError::UnknownOffer)?;
        validate_trusted_activation(&self.partition, &trusted)?;
        let player_sequence = offer.player_sequence;
        let purchase_id = DivineBoostPurchaseId::derive(
            &self.partition.colony_id,
            &self.partition.player_id,
            player_sequence,
        );
        let is_replay = boosts.purchases.contains_key(&purchase_id);
        if (current_epoch != self.epoch || current_entitlements != self.entitlements) && !is_replay
        {
            return Err(DivineActionOfferError::StaleWitness);
        }
        if !is_replay
            && boosts
                .next_player_purchase_sequence(&self.partition.player_id)
                .map_err(DivineActionOfferError::Boost)?
                != player_sequence
        {
            return Err(DivineActionOfferError::StaleWitness);
        }
        Ok(DivineBoostPurchaseRequest {
            id: purchase_id,
            partition: self.partition.clone(),
            player_sequence,
            authorization: trusted.authorization,
            boost_type: offer.boost_type,
            duration_hours: offer.duration_hours,
            expected_boost_version: self.epoch.boost_authority_version,
            expected_void_version: self.epoch.void_ledger_version,
            activated_tick: trusted.activated_tick,
            ticks_per_game_hour: trusted.ticks_per_game_hour,
        })
    }

    /// Resolve and commit against the canonical LAI.58-owned Void ledger. This
    /// is atomic inside [`DivineBoostState::purchase_with_entitlements`] and
    /// never constructs a legacy or persisted `ProgressionAuthority`.
    pub fn purchase_by_id(
        &self,
        offer_id: &str,
        hole: &DivineHoleAuthority,
        boosts: &mut DivineBoostState,
        research: &mut ResearchAuthority,
        trusted: TrustedBoostActivation,
    ) -> Result<DivineBoostOutcome, DivineActionOfferError> {
        let entitlements = research_boost_entitlements(research)?;
        let request = self.resolve_activation_by_id(offer_id, hole, boosts, research, trusted)?;
        boosts
            .purchase_with_entitlements(&mut research.void, &entitlements, request)
            .map_err(DivineActionOfferError::Boost)
    }

    fn validate(&self) -> Result<(), DivineActionOfferError> {
        if self.schema_version != DIVINE_ACTION_OFFER_SCHEMA_VERSION
            || self.offers.len() > MAX_DIVINE_BOOST_OFFERS
        {
            return Err(DivineActionOfferError::MalformedPersistence);
        }
        validate_partition(&self.partition)
            .map_err(|_| DivineActionOfferError::MalformedPersistence)?;
        validate_entitlements(&self.entitlements)
            .map_err(|_| DivineActionOfferError::MalformedPersistence)?;
        if self.next_player_purchase_sequence == 0 {
            return Err(DivineActionOfferError::MalformedPersistence);
        }
        let expected = canonical_boost_offers(
            &self.partition,
            &self.entitlements,
            self.epoch,
            self.next_player_purchase_sequence,
        )
        .map_err(|_| DivineActionOfferError::MalformedPersistence)?;
        if self.offers != expected {
            return Err(DivineActionOfferError::MalformedPersistence);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedDivineBoostOfferCatalog {
    schema_version: u32,
    partition: PlayerPartitionKey,
    epoch: DivineBoostOfferEpoch,
    entitlements: DivineBoostResearchEntitlements,
    next_player_purchase_sequence: u64,
    offers: Vec<DivineBoostOffer>,
}

impl<'de> Deserialize<'de> for DivineBoostOfferCatalog {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UncheckedDivineBoostOfferCatalog::deserialize(deserializer)?;
        let catalog = Self {
            schema_version: raw.schema_version,
            partition: raw.partition,
            epoch: raw.epoch,
            entitlements: raw.entitlements,
            next_player_purchase_sequence: raw.next_player_purchase_sequence,
            offers: raw.offers,
        };
        catalog.validate().map_err(serde::de::Error::custom)?;
        Ok(catalog)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedBoostActivation {
    pub authorization: DivineBoostAuthorization,
    pub activated_tick: u64,
    pub ticks_per_game_hour: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportedResidentNeedsSummary {
    pub living_resident_count: u64,
    pub reported_dying_from_hunger: bool,
    pub reported_dying_from_thirst: bool,
}

impl ReportedResidentNeedsSummary {
    pub fn new(
        living_resident_count: u64,
        reported_dying_from_hunger: bool,
        reported_dying_from_thirst: bool,
    ) -> Result<Self, DivineActionOfferError> {
        let summary = Self {
            living_resident_count,
            reported_dying_from_hunger,
            reported_dying_from_thirst,
        };
        summary.validate()?;
        Ok(summary)
    }

    fn validate(self) -> Result<(), DivineActionOfferError> {
        if self.living_resident_count > MAX_REPORTED_LIVING_RESIDENTS
            || (self.living_resident_count == 0
                && (self.reported_dying_from_hunger || self.reported_dying_from_thirst))
        {
            return Err(DivineActionOfferError::InvalidResidentNeedsSummary);
        }
        Ok(())
    }

    #[must_use]
    pub const fn permits(self, supply: EmergencySupplyKind) -> bool {
        match supply {
            EmergencySupplyKind::DivineRation => self.reported_dying_from_hunger,
            EmergencySupplyKind::DivineWater => self.reported_dying_from_thirst,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmergencyRescueWitnessEpoch {
    pub needs_report_version: u64,
    pub hole_authority_version: u64,
    pub void_ledger_version: u64,
}

impl EmergencyRescueWitnessEpoch {
    pub fn capture(
        partition: &PlayerPartitionKey,
        needs_report_version: u64,
        hole: &DivineHoleAuthority,
        void_ledger: &VoidInsightLedger,
    ) -> Result<Self, DivineActionOfferError> {
        validate_partition(partition)?;
        if hole.binding.colony_id != partition.colony_id
            || void_ledger.partition.colony_id != partition.colony_id
        {
            return Err(DivineActionOfferError::PartitionMismatch);
        }
        Ok(Self {
            needs_report_version,
            hole_authority_version: hole.version,
            void_ledger_version: void_ledger.version,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EmergencyRescueWitnessId(PlannerId);

impl EmergencyRescueWitnessId {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn derive(
        partition: &PlayerPartitionKey,
        summary: ReportedResidentNeedsSummary,
        supply: EmergencySupplyKind,
        epoch: EmergencyRescueWitnessEpoch,
    ) -> Self {
        Self(PlannerId::derive(
            "emergency_rescue_witness_v1",
            [
                partition.colony_id.as_str(),
                partition.player_id.as_str(),
                emergency_supply_token(supply),
                &summary.living_resident_count.to_string(),
                bool_token(summary.reported_dying_from_hunger),
                bool_token(summary.reported_dying_from_thirst),
                &epoch.needs_report_version.to_string(),
                &epoch.hole_authority_version.to_string(),
                &epoch.void_ledger_version.to_string(),
            ],
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmergencyRescueWitness {
    pub id: EmergencyRescueWitnessId,
    pub supply: EmergencySupplyKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyRescueWitnessSet {
    pub schema_version: u32,
    pub partition: PlayerPartitionKey,
    pub epoch: EmergencyRescueWitnessEpoch,
    pub summary: ReportedResidentNeedsSummary,
    pub witnesses: Vec<EmergencyRescueWitness>,
}

impl EmergencyRescueWitnessSet {
    pub fn capture(
        partition: PlayerPartitionKey,
        needs_report_version: u64,
        summary: ReportedResidentNeedsSummary,
        hole: &DivineHoleAuthority,
        void_ledger: &VoidInsightLedger,
    ) -> Result<Self, DivineActionOfferError> {
        let epoch = EmergencyRescueWitnessEpoch::capture(
            &partition,
            needs_report_version,
            hole,
            void_ledger,
        )?;
        Self::new(partition, epoch, summary)
    }

    pub fn new(
        partition: PlayerPartitionKey,
        epoch: EmergencyRescueWitnessEpoch,
        summary: ReportedResidentNeedsSummary,
    ) -> Result<Self, DivineActionOfferError> {
        validate_partition(&partition)?;
        summary.validate()?;
        let witnesses = canonical_rescue_witnesses(&partition, epoch, summary);
        let set = Self {
            schema_version: DIVINE_ACTION_OFFER_SCHEMA_VERSION,
            partition,
            epoch,
            summary,
            witnesses,
        };
        set.validate()?;
        Ok(set)
    }

    pub fn decode_strict(json: &str) -> Result<Self, DivineActionOfferError> {
        serde_json::from_str(json).map_err(|_| DivineActionOfferError::MalformedPersistence)
    }

    pub fn canonical_json(&self) -> Result<String, DivineActionOfferError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| DivineActionOfferError::MalformedPersistence)
    }

    /// Resolve an opaque witness to the only matching emergency action. Supply,
    /// population, evidence flags, and expected versions all come from the
    /// witnessed server report rather than the client command.
    pub fn resolve_rescue(
        &self,
        witness_id: &EmergencyRescueWitnessId,
        current_needs_report_version: u64,
        current_summary: ReportedResidentNeedsSummary,
        hole: &DivineHoleAuthority,
        void_ledger: &VoidInsightLedger,
        trusted: TrustedEmergencyRescue,
    ) -> Result<VoidActionEnvelope, DivineActionOfferError> {
        self.validate()?;
        current_summary.validate()?;
        let current_epoch = EmergencyRescueWitnessEpoch::capture(
            &self.partition,
            current_needs_report_version,
            hole,
            void_ledger,
        )?;
        if current_epoch != self.epoch || current_summary != self.summary {
            return Err(DivineActionOfferError::StaleWitness);
        }
        let witness = self
            .witnesses
            .iter()
            .find(|witness| &witness.id == witness_id)
            .ok_or(DivineActionOfferError::UnknownWitness)?;
        if !self.summary.permits(witness.supply) {
            return Err(DivineActionOfferError::MissingReportEvidence);
        }
        VoidActionEnvelope::new(
            trusted.command_id,
            self.epoch.hole_authority_version,
            self.epoch.void_ledger_version,
            VoidAction::EmergencyRescue(EmergencyRescueRequest {
                player_id: self.partition.player_id.as_str().to_owned(),
                supply: witness.supply,
                living_resident_count: self.summary.living_resident_count,
                residents_dying_from_hunger: self.summary.reported_dying_from_hunger,
                residents_dying_from_thirst: self.summary.reported_dying_from_thirst,
                now_real_ms: trusted.now_real_ms,
            }),
        )
        .map_err(|_| DivineActionOfferError::InvalidTrustedContext)
    }

    fn validate(&self) -> Result<(), DivineActionOfferError> {
        if self.schema_version != DIVINE_ACTION_OFFER_SCHEMA_VERSION
            || self.witnesses.len() > MAX_EMERGENCY_RESCUE_WITNESSES
        {
            return Err(DivineActionOfferError::MalformedPersistence);
        }
        validate_partition(&self.partition)
            .map_err(|_| DivineActionOfferError::MalformedPersistence)?;
        self.summary
            .validate()
            .map_err(|_| DivineActionOfferError::MalformedPersistence)?;
        if self.witnesses != canonical_rescue_witnesses(&self.partition, self.epoch, self.summary) {
            return Err(DivineActionOfferError::MalformedPersistence);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedEmergencyRescueWitnessSet {
    schema_version: u32,
    partition: PlayerPartitionKey,
    epoch: EmergencyRescueWitnessEpoch,
    summary: ReportedResidentNeedsSummary,
    witnesses: Vec<EmergencyRescueWitness>,
}

impl<'de> Deserialize<'de> for EmergencyRescueWitnessSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UncheckedEmergencyRescueWitnessSet::deserialize(deserializer)?;
        let set = Self {
            schema_version: raw.schema_version,
            partition: raw.partition,
            epoch: raw.epoch,
            summary: raw.summary,
            witnesses: raw.witnesses,
        };
        set.validate().map_err(serde::de::Error::custom)?;
        Ok(set)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedEmergencyRescue {
    pub command_id: String,
    pub now_real_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivineActionOfferError {
    PartitionMismatch,
    InvalidPartition,
    InvalidResearchEntitlements,
    MalformedResearchAuthority,
    InvalidResidentNeedsSummary,
    MissingReportEvidence,
    UnknownOffer,
    UnknownWitness,
    StaleWitness,
    InvalidPurchaseSequence,
    InvalidTrustedContext,
    Boost(crate::divine_boosts::DivineBoostError),
    MalformedPersistence,
}

impl std::fmt::Display for DivineActionOfferError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "divine action offer rejected ({self:?})")
    }
}

impl std::error::Error for DivineActionOfferError {}

/// Derive the complete boost entitlement report from the canonical LAI.58
/// finite-completion set. Unknown studies and malformed stages fail closed;
/// no legacy progression state or second ownership ledger is constructed.
pub fn research_boost_entitlements(
    research: &ResearchAuthority,
) -> Result<DivineBoostResearchEntitlements, DivineActionOfferError> {
    if research.schema_version != RESEARCH_AUTHORITY_SCHEMA_VERSION
        || research.void.partition.colony_id != research.colony_id
    {
        return Err(DivineActionOfferError::MalformedResearchAuthority);
    }
    let catalog = ProgressionCatalog::from_embedded()
        .map_err(|_| DivineActionOfferError::MalformedResearchAuthority)?;
    let mut entitlements = DivineBoostResearchEntitlements {
        unlocked_boosts: std::collections::BTreeSet::new(),
        stages: DivineBoostResearchStages::default(),
    };
    for study_id in &research.owned_finite {
        let definition = catalog
            .study(study_id)
            .ok_or(DivineActionOfferError::MalformedResearchAuthority)?;
        if definition
            .prerequisites
            .iter()
            .any(|prerequisite| !research.owned_finite.contains(prerequisite))
            || matches!(
                &definition.kind,
                StudyKind::BoostDuration { stage } | StudyKind::BoostEconomy { stage }
                    if *stage == BOOST_RESEARCH_STAGE_COUNT
            )
        {
            return Err(DivineActionOfferError::MalformedResearchAuthority);
        }
        match &definition.kind {
            StudyKind::BoostUnlock { boost } => {
                entitlements
                    .unlocked_boosts
                    .insert(divine_boost_type(*boost));
            }
            StudyKind::BoostDuration { stage } => {
                entitlements.stages.divine_duration_stage =
                    entitlements.stages.divine_duration_stage.max(*stage);
            }
            StudyKind::BoostEconomy { stage } => {
                entitlements.stages.divine_economy_stage =
                    entitlements.stages.divine_economy_stage.max(*stage);
            }
            StudyKind::OrdinaryCapability { .. } | StudyKind::HoleAxis { .. } => {}
        }
    }
    validate_entitlements(&entitlements)?;
    Ok(entitlements)
}

fn canonical_boost_offers(
    partition: &PlayerPartitionKey,
    entitlements: &DivineBoostResearchEntitlements,
    epoch: DivineBoostOfferEpoch,
    player_sequence: u64,
) -> Result<Vec<DivineBoostOffer>, DivineActionOfferError> {
    validate_entitlements(entitlements)?;
    if player_sequence == 0 {
        return Err(DivineActionOfferError::InvalidPurchaseSequence);
    }
    let unlocked_durations =
        UnlockedBoostDurations::for_stage(entitlements.stages.divine_duration_stage);
    let mut offers = Vec::new();
    for boost_type in DivineBoostType::ALL {
        if !entitlements.unlocked_boosts.contains(&boost_type) {
            continue;
        }
        for duration_hours in crate::divine_boosts::DIVINE_BOOST_DURATION_HOURS {
            if !unlocked_durations.contains(duration_hours) {
                continue;
            }
            let exact_cost = boost_cost(boost_type, duration_hours, entitlements.stages)
                .map_err(|_| DivineActionOfferError::InvalidResearchEntitlements)?;
            offers.push(DivineBoostOffer {
                id: DivineBoostOfferId::derive(
                    partition,
                    boost_type,
                    duration_hours,
                    entitlements.stages,
                    exact_cost,
                    epoch,
                    player_sequence,
                ),
                boost_type,
                duration_hours,
                researched_stages: entitlements.stages,
                exact_cost,
                player_sequence,
            });
        }
    }
    if offers.len() > MAX_DIVINE_BOOST_OFFERS {
        return Err(DivineActionOfferError::InvalidResearchEntitlements);
    }
    Ok(offers)
}

fn canonical_rescue_witnesses(
    partition: &PlayerPartitionKey,
    epoch: EmergencyRescueWitnessEpoch,
    summary: ReportedResidentNeedsSummary,
) -> Vec<EmergencyRescueWitness> {
    [
        EmergencySupplyKind::DivineRation,
        EmergencySupplyKind::DivineWater,
    ]
    .into_iter()
    .filter(|supply| summary.permits(*supply))
    .map(|supply| EmergencyRescueWitness {
        id: EmergencyRescueWitnessId::derive(partition, summary, supply, epoch),
        supply,
    })
    .collect()
}

fn validate_entitlements(
    entitlements: &DivineBoostResearchEntitlements,
) -> Result<(), DivineActionOfferError> {
    if entitlements.unlocked_boosts.len() > DivineBoostType::ALL.len()
        || entitlements
            .unlocked_boosts
            .iter()
            .any(|boost| !DivineBoostType::ALL.contains(boost))
        || boost_cost(
            DivineBoostType::BountifulLabor,
            crate::divine_boosts::DIVINE_BOOST_BASE_DURATION_GAME_HOURS,
            entitlements.stages,
        )
        .is_err()
    {
        return Err(DivineActionOfferError::InvalidResearchEntitlements);
    }
    Ok(())
}

fn validate_partition(partition: &PlayerPartitionKey) -> Result<(), DivineActionOfferError> {
    if !bounded_planner_id(&partition.colony_id) || !bounded_planner_id(&partition.player_id) {
        return Err(DivineActionOfferError::InvalidPartition);
    }
    Ok(())
}

fn bounded_planner_id(id: &PlannerId) -> bool {
    !id.as_str().trim().is_empty() && id.as_str().len() <= MAX_PARTITION_ID_BYTES
}

fn validate_trusted_activation(
    partition: &PlayerPartitionKey,
    trusted: &TrustedBoostActivation,
) -> Result<(), DivineActionOfferError> {
    let DivineBoostActor::Player { player_id } = &trusted.authorization.actor else {
        return Err(DivineActionOfferError::InvalidTrustedContext);
    };
    if trusted.ticks_per_game_hour == 0
        || player_id != &partition.player_id
        || trusted.authorization.authenticated_player_id.as_ref() != Some(player_id)
        || !trusted.authorization.owns_colony
    {
        return Err(DivineActionOfferError::InvalidTrustedContext);
    }
    Ok(())
}

const fn boost_type_token(boost_type: DivineBoostType) -> &'static str {
    match boost_type {
        DivineBoostType::BountifulLabor => "bountiful_labor",
        DivineBoostType::FleetPaws => "fleet_paws",
        DivineBoostType::InspiredWork => "inspired_work",
        DivineBoostType::RestorativeGrace => "restorative_grace",
    }
}

const fn divine_boost_type(boost: SpecializedBoost) -> DivineBoostType {
    match boost {
        SpecializedBoost::BountifulLabor => DivineBoostType::BountifulLabor,
        SpecializedBoost::FleetPaws => DivineBoostType::FleetPaws,
        SpecializedBoost::InspiredWork => DivineBoostType::InspiredWork,
        SpecializedBoost::RestorativeGrace => DivineBoostType::RestorativeGrace,
    }
}

const fn emergency_supply_token(supply: EmergencySupplyKind) -> &'static str {
    match supply {
        EmergencySupplyKind::DivineRation => "divine_ration",
        EmergencySupplyKind::DivineWater => "divine_water",
    }
}

const fn bool_token(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
