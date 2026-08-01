//! Deterministic physical player-village trade specified by
//! `docs/leader-ai-overhaul/diplomacy-trade.md`.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    authority::{
        AuthorityActor, AuthorityContext, AuthorityDecision, AuthorityDenial, AuthorityDomain,
        AuthorityOperation, decide_authority,
    },
    diplomacy::{DiplomacyColonyId, DiplomacyPair, DiplomacyRelationship},
    planner_core::PlannerId,
    spatial_resolver::ResolvedSpatialTask,
    spatial_tasks::{SiteLifecycleStage, SiteRef, SiteVisibility, SpatialBlockReason},
    task_runtime::{CargoLocation, TaskCargo},
    trade_valuation::{TradePurpose, TradeValuation, TradeValuationError},
    world_reservations::{
        WorldCommitOutcome, WorldReleaseOutcome, WorldReservationError, WorldReservationId,
        WorldReservationLedger, WorldReservationTransaction, WorldReservationValidation,
    },
};

pub const TRADE_SCHEMA_VERSION: u32 = 1;
pub const MAX_TRADE_LEGS: usize = 2;

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(PlannerId);

        impl $name {
            #[must_use]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
    };
}

stable_id!(TradeProposalId);
stable_id!(TradeContractId);
stable_id!(TradeActionId);

impl TradeProposalId {
    #[must_use]
    pub fn derive(pair: &DiplomacyPair, initiator: &DiplomacyColonyId, occurrence: u32) -> Self {
        Self(PlannerId::derive(
            "trade_proposal",
            [
                pair.id().as_str(),
                initiator.as_str(),
                &occurrence.to_string(),
            ],
        ))
    }
}

impl TradeContractId {
    #[must_use]
    pub fn derive(proposal_id: &TradeProposalId) -> Self {
        Self(PlannerId::derive("trade_contract", [proposal_id.as_str()]))
    }
}

impl TradeActionId {
    #[must_use]
    pub fn derive(
        contract_id: &TradeContractId,
        acting_colony: &DiplomacyColonyId,
        occurrence: &str,
        kind: TradeActionKind,
    ) -> Self {
        Self(PlannerId::derive(
            "trade_action",
            [
                contract_id.as_str(),
                acting_colony.as_str(),
                occurrence,
                kind.as_str(),
            ],
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeColonyKind {
    PlayerFounded,
    Npc,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeParty {
    pub diplomacy_id: DiplomacyColonyId,
    pub reservation_colony_id: PlannerId,
    pub kind: TradeColonyKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeCargoLeg {
    pub owner: DiplomacyColonyId,
    pub recipient: DiplomacyColonyId,
    pub cargo: TaskCargo,
    pub spatial: ResolvedSpatialTask,
    pub escrow: WorldReservationTransaction,
    pub hauler_id: PlannerId,
}

impl TradeCargoLeg {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        owner: DiplomacyColonyId,
        recipient: DiplomacyColonyId,
        resource_id: impl Into<String>,
        quantity: u32,
        spatial: ResolvedSpatialTask,
        escrow: WorldReservationTransaction,
        hauler_id: PlannerId,
    ) -> Result<Self, TradeError> {
        let resource_id = resource_id.into();
        let cargo_id = PlannerId::derive(
            "trade_cargo",
            [
                escrow.intent_id.as_str(),
                owner.as_str(),
                resource_id.as_str(),
            ],
        )
        .to_string();
        let cargo = TaskCargo {
            cargo_id,
            resource_id,
            quantity: u64::from(quantity),
            location: CargoLocation::ReservedAtSource {
                source_id: spatial.objective().stable_id().to_owned(),
            },
        };
        let leg = Self {
            owner,
            recipient,
            cargo,
            spatial,
            escrow,
            hauler_id,
        };
        leg.validate()?;
        Ok(leg)
    }

    fn validate(&self) -> Result<(), TradeError> {
        self.spatial
            .validate()
            .map_err(TradeError::SpatialBlocked)?;
        let valid_location = match &self.cargo.location {
            CargoLocation::ReservedAtSource { source_id } => {
                source_id == self.spatial.objective().stable_id()
            }
            CargoLocation::Carried { cat_id } => cat_id == &self.hauler_id.to_string(),
            CargoLocation::DepositedAtEndpoint { endpoint_id } => {
                endpoint_id == self.spatial.delivery_endpoint().stable_id()
            }
            CargoLocation::SalvagedAtStockpile { stockpile_id } => !stockpile_id.is_empty(),
            CargoLocation::Stranded { site_id } => !site_id.is_empty(),
        };
        if self.owner == self.recipient
            || self.cargo.cargo_id.is_empty()
            || self.cargo.resource_id.is_empty()
            || self.cargo.quantity == 0
            || self.escrow.resolved != self.spatial
            || self.escrow.worker_id != self.hauler_id
            || !valid_location
        {
            return Err(TradeError::MalformedContract);
        }
        let expected_resource = PlannerId::derive("trade_resource", [&self.cargo.resource_id]);
        let quantity =
            u32::try_from(self.cargo.quantity).map_err(|_| TradeError::MalformedContract)?;
        if !self.escrow.cargo_resources.iter().any(|reservation| {
            reservation.stable_id == expected_resource && reservation.units == quantity
        }) {
            return Err(TradeError::MalformedContract);
        }
        let canonical_escrow = WorldReservationTransaction::new(
            self.escrow.colony_id.clone(),
            self.escrow.task_id.clone(),
            self.escrow.intent_id.clone(),
            self.escrow.resolved.clone(),
            self.escrow.worker_id.clone(),
            self.escrow.tool_ids.clone(),
            self.escrow.cargo_resources.clone(),
        )
        .map_err(TradeError::Escrow)?;
        if canonical_escrow != self.escrow {
            return Err(TradeError::MalformedContract);
        }
        Ok(())
    }

    fn reset_to_source(&mut self) {
        self.cargo.location = CargoLocation::ReservedAtSource {
            source_id: self.spatial.objective().stable_id().to_owned(),
        };
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeProposal {
    pub schema_version: u32,
    pub id: TradeProposalId,
    pub contract_id: TradeContractId,
    pub occurrence: u32,
    pub pair: DiplomacyPair,
    pub initiator: DiplomacyColonyId,
    #[serde(with = "trade_party_map_wire")]
    pub parties: BTreeMap<DiplomacyColonyId, TradeParty>,
    pub relationship: DiplomacyRelationship,
    pub purpose: TradePurpose,
    #[serde(with = "trade_valuation_map_wire")]
    pub valuations: BTreeMap<DiplomacyColonyId, TradeValuation>,
    pub legs: Vec<TradeCargoLeg>,
    pub created_tick: u64,
    pub expiry_tick: u64,
    pub actor: AuthorityActor,
}

/// JSON object keys cannot carry the structured, external-ID-preserving
/// `DiplomacyColonyId` wire. Persist the canonical map as ordered values whose
/// embedded diplomacy ID is the key, and reject duplicates while decoding.
mod trade_party_map_wire {
    use super::*;

    pub fn serialize<S>(
        parties: &BTreeMap<DiplomacyColonyId, TradeParty>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        parties.values().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<DiplomacyColonyId, TradeParty>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = Vec::<TradeParty>::deserialize(deserializer)?;
        let mut parties = BTreeMap::new();
        for party in entries {
            let key = party.diplomacy_id.clone();
            if parties.insert(key, party).is_some() {
                return Err(serde::de::Error::custom("duplicate trade party"));
            }
        }
        Ok(parties)
    }
}

mod trade_valuation_map_wire {
    use super::*;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ValuationEntryRef<'a> {
        colony_id: &'a DiplomacyColonyId,
        valuation: &'a TradeValuation,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct ValuationEntry {
        colony_id: DiplomacyColonyId,
        valuation: TradeValuation,
    }

    pub fn serialize<S>(
        valuations: &BTreeMap<DiplomacyColonyId, TradeValuation>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        valuations
            .iter()
            .map(|(colony_id, valuation)| ValuationEntryRef {
                colony_id,
                valuation,
            })
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<DiplomacyColonyId, TradeValuation>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = Vec::<ValuationEntry>::deserialize(deserializer)?;
        let mut valuations = BTreeMap::new();
        for entry in entries {
            if valuations
                .insert(entry.colony_id, entry.valuation)
                .is_some()
            {
                return Err(serde::de::Error::custom("duplicate trade valuation"));
            }
        }
        Ok(valuations)
    }
}

impl TradeProposal {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pair: DiplomacyPair,
        initiator: DiplomacyColonyId,
        occurrence: u32,
        parties: BTreeMap<DiplomacyColonyId, TradeParty>,
        relationship: DiplomacyRelationship,
        purpose: TradePurpose,
        valuations: BTreeMap<DiplomacyColonyId, TradeValuation>,
        mut legs: Vec<TradeCargoLeg>,
        created_tick: u64,
        expiry_tick: u64,
        actor: AuthorityActor,
    ) -> Result<Self, TradeError> {
        let id = TradeProposalId::derive(&pair, &initiator, occurrence);
        let contract_id = TradeContractId::derive(&id);
        legs.sort_by(|first, second| first.owner.cmp(&second.owner));
        let proposal = Self {
            schema_version: TRADE_SCHEMA_VERSION,
            id,
            contract_id,
            occurrence,
            pair,
            initiator,
            parties,
            relationship,
            purpose,
            valuations,
            legs,
            created_tick,
            expiry_tick,
            actor,
        };
        proposal.validate()?;
        Ok(proposal)
    }

    fn validate(&self) -> Result<(), TradeError> {
        if self.schema_version != TRADE_SCHEMA_VERSION
            || self.id != TradeProposalId::derive(&self.pair, &self.initiator, self.occurrence)
            || self.contract_id != TradeContractId::derive(&self.id)
            || !self.pair.contains(&self.initiator)
            || self.created_tick >= self.expiry_tick
            || !matches!(
                self.relationship,
                DiplomacyRelationship::Friendly | DiplomacyRelationship::Allied
            )
            || self.parties.len() != 2
            || self.valuations.len() != 2
            || self.legs.len() != MAX_TRADE_LEGS
            || self
                .legs
                .windows(2)
                .any(|pair| pair[0].owner >= pair[1].owner)
        {
            return Err(TradeError::MalformedContract);
        }
        if self
            .parties
            .values()
            .any(|party| party.kind != TradeColonyKind::PlayerFounded)
        {
            return Err(TradeError::NpcLayerSeparated);
        }
        let expected_parties =
            BTreeSet::from([self.pair.first().clone(), self.pair.second().clone()]);
        if self.parties.keys().cloned().collect::<BTreeSet<_>>() != expected_parties
            || self.valuations.keys().cloned().collect::<BTreeSet<_>>() != expected_parties
            || self
                .legs
                .iter()
                .map(|leg| leg.owner.clone())
                .collect::<BTreeSet<_>>()
                != expected_parties
        {
            return Err(TradeError::MalformedContract);
        }
        for (colony_id, party) in &self.parties {
            if colony_id != &party.diplomacy_id {
                return Err(TradeError::MalformedContract);
            }
        }
        for valuation in self.valuations.values() {
            valuation.validate().map_err(TradeError::Valuation)?;
            if valuation.relationship != self.relationship || valuation.purpose != self.purpose {
                return Err(TradeError::MalformedContract);
            }
        }
        for leg in &self.legs {
            leg.validate()?;
            if !expected_parties.contains(&leg.recipient)
                || leg.recipient == leg.owner
                || self
                    .parties
                    .get(&leg.owner)
                    .is_none_or(|party| party.reservation_colony_id != leg.escrow.colony_id)
            {
                return Err(TradeError::MalformedContract);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeStage {
    Proposed,
    Escrowed,
    InTransit,
    Returning,
    Blocked,
    Stranded,
    Complete,
    Cancelled,
}

impl TradeStage {
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Stranded | Self::Complete | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeBlockReason {
    SourceUnavailable,
    InsufficientEscrow,
    RouteBlocked,
    DestinationFull,
    DestinationRemoved,
    WorkerRefused,
    WorkerDied,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeRecoveryState {
    None,
    Returning,
    Returned,
    Salvaged,
    Stranded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeContract {
    pub proposal: TradeProposal,
    pub version: u64,
    pub stage: TradeStage,
    pub acceptances: BTreeSet<DiplomacyColonyId>,
    pub next_event_tick: Option<u64>,
    pub escrow_reservation_ids: Vec<WorldReservationId>,
    pub blocked_reason: Option<TradeBlockReason>,
    pub recovery: TradeRecoveryState,
}

impl TradeContract {
    fn proposed(proposal: TradeProposal) -> Self {
        Self {
            next_event_tick: Some(proposal.created_tick),
            proposal,
            version: 0,
            stage: TradeStage::Proposed,
            acceptances: BTreeSet::new(),
            escrow_reservation_ids: Vec::new(),
            blocked_reason: None,
            recovery: TradeRecoveryState::None,
        }
    }

    #[must_use]
    pub fn id(&self) -> &TradeContractId {
        &self.proposal.contract_id
    }

    fn validate(&self) -> Result<(), TradeError> {
        self.proposal.validate()?;
        let parties = self
            .proposal
            .parties
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        if !self.acceptances.is_subset(&parties)
            || self.escrow_reservation_ids.len() > MAX_TRADE_LEGS
            || self
                .escrow_reservation_ids
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || (self.stage == TradeStage::Proposed && !self.escrow_reservation_ids.is_empty())
            || (matches!(
                self.stage,
                TradeStage::Escrowed | TradeStage::InTransit | TradeStage::Returning
            ) && self.escrow_reservation_ids.len() != MAX_TRADE_LEGS)
            || (self.stage.is_terminal() && self.next_event_tick.is_some())
            || (self.stage == TradeStage::Blocked) != self.blocked_reason.is_some()
        {
            return Err(TradeError::MalformedContract);
        }
        let locations_match_stage = self.proposal.legs.iter().all(|leg| match self.stage {
            TradeStage::Proposed | TradeStage::Escrowed | TradeStage::Cancelled => {
                matches!(leg.cargo.location, CargoLocation::ReservedAtSource { .. })
            }
            TradeStage::InTransit | TradeStage::Returning => {
                matches!(leg.cargo.location, CargoLocation::Carried { .. })
            }
            TradeStage::Blocked => matches!(
                leg.cargo.location,
                CargoLocation::Carried { .. } | CargoLocation::SalvagedAtStockpile { .. }
            ),
            TradeStage::Stranded => matches!(
                leg.cargo.location,
                CargoLocation::Stranded { .. } | CargoLocation::SalvagedAtStockpile { .. }
            ),
            TradeStage::Complete => matches!(
                leg.cargo.location,
                CargoLocation::DepositedAtEndpoint { .. }
            ),
        });
        if !locations_match_stage {
            return Err(TradeError::MalformedContract);
        }
        for leg in &self.proposal.legs {
            leg.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeActionKind {
    Accept,
    Cancel,
}

impl TradeActionKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Cancel => "cancel",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeAction {
    pub id: TradeActionId,
    pub contract_id: TradeContractId,
    pub acting_colony: DiplomacyColonyId,
    pub expected_version: u64,
    pub kind: TradeActionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeAuthorization {
    pub actor: AuthorityActor,
    pub acting_colony: DiplomacyColonyId,
    pub owner_player_id: Option<PlannerId>,
    pub authorized_for_colony: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeReceipt {
    pub action_id: TradeActionId,
    pub contract_id: TradeContractId,
    pub acting_colony: DiplomacyColonyId,
    pub expected_version: u64,
    pub resulting_version: u64,
    pub kind: TradeActionKind,
    pub stage: TradeStage,
    pub actor: AuthorityActor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeliveryValidation {
    pub route_valid: bool,
    pub destination_exists: bool,
    pub destination_capacity_available: bool,
    pub cargo_exact: bool,
    pub haulers_available: bool,
}

impl DeliveryValidation {
    #[must_use]
    pub const fn all_valid() -> Self {
        Self {
            route_valid: true,
            destination_exists: true,
            destination_capacity_available: true,
            cargo_exact: true,
            haulers_available: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryDisposition {
    pub cargo_id: String,
    pub safe_owned_stockpile: Option<SiteRef>,
    pub last_site_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeError {
    RelationshipDenied,
    NpcLayerSeparated,
    AuthorizationDenied(AuthorityDenial),
    AuthorizationColonyMismatch,
    PlayerIdentityMismatch,
    ContractNotFound,
    ProposalIdCollision,
    ActionIdCollision,
    StaleVersion { expected: u64, actual: u64 },
    InvalidTransition,
    Expired,
    RecoveryRequired,
    Escrow(WorldReservationError),
    SpatialBlocked(SpatialBlockReason),
    Valuation(TradeValuationError),
    MalformedContract,
    MalformedPersistence,
    VersionExhausted,
}

impl fmt::Display for TradeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "autonomous trade error: {self:?}")
    }
}

impl std::error::Error for TradeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeLedger {
    version: u64,
    contracts: BTreeMap<TradeContractId, TradeContract>,
    action_receipts: BTreeMap<TradeActionId, TradeReceipt>,
}

impl TradeLedger {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            version: 0,
            contracts: BTreeMap::new(),
            action_receipts: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.contracts.is_empty()
    }

    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }

    #[must_use]
    pub fn contract(&self, id: &TradeContractId) -> Option<&TradeContract> {
        self.contracts.get(id)
    }

    pub fn contracts(&self) -> impl ExactSizeIterator<Item = &TradeContract> {
        self.contracts.values()
    }

    pub fn propose(
        &mut self,
        proposal: TradeProposal,
        authorization: &TradeAuthorization,
    ) -> Result<TradeContractId, TradeError> {
        proposal.validate()?;
        if proposal
            .parties
            .values()
            .any(|party| party.kind == TradeColonyKind::Npc)
        {
            return Err(TradeError::NpcLayerSeparated);
        }
        validate_authorization(&proposal.initiator, authorization)?;
        if proposal.actor != authorization.actor {
            return Err(TradeError::AuthorizationDenied(
                AuthorityDenial::StrategyForbidden,
            ));
        }
        let id = proposal.contract_id.clone();
        if let Some(existing) = self.contracts.get(&id) {
            return if existing.proposal == proposal {
                Ok(id)
            } else {
                Err(TradeError::ProposalIdCollision)
            };
        }
        self.version = self
            .version
            .checked_add(1)
            .ok_or(TradeError::VersionExhausted)?;
        self.contracts
            .insert(id.clone(), TradeContract::proposed(proposal));
        Ok(id)
    }

    pub fn apply_action(
        &mut self,
        action: TradeAction,
        authorization: &TradeAuthorization,
        current_relationship: DiplomacyRelationship,
        now_tick: u64,
        world: &mut WorldReservationLedger,
    ) -> Result<TradeReceipt, TradeError> {
        validate_authorization(&action.acting_colony, authorization)?;
        if let Some(receipt) = self.action_receipts.get(&action.id) {
            return if receipt.contract_id == action.contract_id
                && receipt.acting_colony == action.acting_colony
                && receipt.expected_version == action.expected_version
                && receipt.kind == action.kind
                && receipt.actor == authorization.actor
            {
                Ok(receipt.clone())
            } else {
                Err(TradeError::ActionIdCollision)
            };
        }
        let mut contract = self
            .contracts
            .get(&action.contract_id)
            .cloned()
            .ok_or(TradeError::ContractNotFound)?;
        if !contract.proposal.pair.contains(&action.acting_colony) {
            return Err(TradeError::AuthorizationColonyMismatch);
        }
        if contract.version != action.expected_version {
            return Err(TradeError::StaleVersion {
                expected: action.expected_version,
                actual: contract.version,
            });
        }
        match action.kind {
            TradeActionKind::Accept => {
                if contract.stage != TradeStage::Proposed {
                    return Err(TradeError::InvalidTransition);
                }
                if now_tick >= contract.proposal.expiry_tick {
                    return Err(TradeError::Expired);
                }
                if !matches!(
                    current_relationship,
                    DiplomacyRelationship::Friendly | DiplomacyRelationship::Allied
                ) || current_relationship != contract.proposal.relationship
                {
                    return Err(TradeError::RelationshipDenied);
                }
                contract.acceptances.insert(action.acting_colony.clone());
                if contract.acceptances.len() == contract.proposal.parties.len() {
                    let mut candidate_world = world.clone();
                    let mut escrow_ids = Vec::with_capacity(MAX_TRADE_LEGS);
                    let mut transactions = contract
                        .proposal
                        .legs
                        .iter()
                        .map(|leg| leg.escrow.clone())
                        .collect::<Vec<_>>();
                    transactions.sort_by(|first, second| first.id.cmp(&second.id));
                    for transaction in transactions {
                        let id = transaction.id.clone();
                        match candidate_world
                            .try_commit(transaction, WorldReservationValidation::all_valid())
                        {
                            Ok(WorldCommitOutcome::Committed) => {
                                escrow_ids.push(id);
                            }
                            Ok(WorldCommitOutcome::AlreadyCommitted) => {
                                return Err(TradeError::Escrow(
                                    WorldReservationError::ReservationIdConflict,
                                ));
                            }
                            Err(error) => return Err(TradeError::Escrow(error)),
                        }
                    }
                    *world = candidate_world;
                    contract.escrow_reservation_ids = escrow_ids;
                    contract.stage = TradeStage::Escrowed;
                    contract.next_event_tick = now_tick.checked_add(1);
                }
            }
            TradeActionKind::Cancel => {
                if !matches!(contract.stage, TradeStage::Proposed | TradeStage::Escrowed) {
                    return if matches!(
                        contract.stage,
                        TradeStage::InTransit | TradeStage::Returning | TradeStage::Blocked
                    ) {
                        Err(TradeError::RecoveryRequired)
                    } else {
                        Err(TradeError::InvalidTransition)
                    };
                }
                release_all_atomic(world, &contract.escrow_reservation_ids)?;
                for leg in &mut contract.proposal.legs {
                    leg.reset_to_source();
                }
                contract.escrow_reservation_ids.clear();
                contract.stage = TradeStage::Cancelled;
                contract.next_event_tick = None;
            }
        }
        contract.version = contract
            .version
            .checked_add(1)
            .ok_or(TradeError::VersionExhausted)?;
        contract.validate()?;
        self.version = self
            .version
            .checked_add(1)
            .ok_or(TradeError::VersionExhausted)?;
        let receipt = TradeReceipt {
            action_id: action.id.clone(),
            contract_id: action.contract_id.clone(),
            acting_colony: action.acting_colony,
            expected_version: action.expected_version,
            resulting_version: contract.version,
            kind: action.kind,
            stage: contract.stage,
            actor: authorization.actor.clone(),
        };
        self.contracts.insert(action.contract_id, contract);
        self.action_receipts.insert(action.id, receipt.clone());
        Ok(receipt)
    }

    pub fn depart(
        &mut self,
        id: &TradeContractId,
        expected_version: u64,
        now_tick: u64,
        world: &WorldReservationLedger,
    ) -> Result<(), TradeError> {
        self.mutate_contract(id, expected_version, |contract| {
            if contract.stage != TradeStage::Escrowed
                || contract
                    .escrow_reservation_ids
                    .iter()
                    .any(|reservation| !world.contains(reservation))
            {
                return Err(TradeError::InvalidTransition);
            }
            for leg in &mut contract.proposal.legs {
                leg.cargo.location = CargoLocation::Carried {
                    cat_id: leg.hauler_id.to_string(),
                };
            }
            contract.stage = TradeStage::InTransit;
            contract.next_event_tick = now_tick.checked_add(1);
            Ok(())
        })
    }

    /// Advance already-departed cargo without consulting relationship state.
    /// A later diplomatic block forbids new acceptance but cannot erase or
    /// teleport an in-flight contract.
    pub fn attempt_delivery(
        &mut self,
        id: &TradeContractId,
        expected_version: u64,
        validation: DeliveryValidation,
        now_tick: u64,
        world: &mut WorldReservationLedger,
    ) -> Result<(), TradeError> {
        let mut candidate = self.clone();
        let mut candidate_world = world.clone();
        candidate.mutate_contract(id, expected_version, |contract| {
            if !matches!(contract.stage, TradeStage::InTransit | TradeStage::Blocked) {
                return Err(TradeError::InvalidTransition);
            }
            let failure = if !validation.route_valid {
                Some(TradeBlockReason::RouteBlocked)
            } else if !validation.destination_exists {
                Some(TradeBlockReason::DestinationRemoved)
            } else if !validation.destination_capacity_available {
                Some(TradeBlockReason::DestinationFull)
            } else if !validation.cargo_exact {
                Some(TradeBlockReason::InsufficientEscrow)
            } else if !validation.haulers_available {
                Some(TradeBlockReason::WorkerRefused)
            } else {
                None
            };
            if let Some(reason) = failure {
                contract.stage = TradeStage::Blocked;
                contract.blocked_reason = Some(reason);
                contract.next_event_tick = now_tick.checked_add(1);
                return Ok(());
            }
            if contract
                .escrow_reservation_ids
                .iter()
                .any(|reservation| !candidate_world.contains(reservation))
            {
                return Err(TradeError::Escrow(
                    WorldReservationError::MalformedPersistence,
                ));
            }
            release_all_atomic(&mut candidate_world, &contract.escrow_reservation_ids)?;
            for leg in &mut contract.proposal.legs {
                leg.cargo.location = CargoLocation::DepositedAtEndpoint {
                    endpoint_id: leg.spatial.delivery_endpoint().stable_id().to_owned(),
                };
            }
            contract.escrow_reservation_ids.clear();
            contract.stage = TradeStage::Complete;
            contract.blocked_reason = None;
            contract.next_event_tick = None;
            Ok(())
        })?;
        *self = candidate;
        *world = candidate_world;
        Ok(())
    }

    /// Begin the persisted physical recovery contract. Relationship state is
    /// deliberately absent so a new block cannot suppress return or salvage.
    pub fn close_route(
        &mut self,
        id: &TradeContractId,
        expected_version: u64,
        valid_return_route: Option<&SiteRef>,
        last_site_by_cargo: &BTreeMap<String, String>,
        now_tick: u64,
        world: &mut WorldReservationLedger,
    ) -> Result<(), TradeError> {
        let mut candidate = self.clone();
        let mut candidate_world = world.clone();
        candidate.mutate_contract(id, expected_version, |contract| {
            if !matches!(contract.stage, TradeStage::InTransit | TradeStage::Blocked) {
                return Err(TradeError::InvalidTransition);
            }
            if valid_return_route.is_some_and(valid_route) {
                contract.stage = TradeStage::Returning;
                contract.blocked_reason = None;
                contract.recovery = TradeRecoveryState::Returning;
                contract.next_event_tick = now_tick.checked_add(1);
                return Ok(());
            }
            for leg in &mut contract.proposal.legs {
                let site_id = last_site_by_cargo
                    .get(&leg.cargo.cargo_id)
                    .filter(|site| !site.is_empty())
                    .ok_or(TradeError::MalformedContract)?;
                leg.cargo.location = CargoLocation::Stranded {
                    site_id: site_id.clone(),
                };
            }
            release_all_atomic(&mut candidate_world, &contract.escrow_reservation_ids)?;
            contract.escrow_reservation_ids.clear();
            contract.stage = TradeStage::Stranded;
            contract.blocked_reason = None;
            contract.recovery = TradeRecoveryState::Stranded;
            contract.next_event_tick = None;
            Ok(())
        })?;
        *self = candidate;
        *world = candidate_world;
        Ok(())
    }

    pub fn finish_return(
        &mut self,
        id: &TradeContractId,
        expected_version: u64,
        world: &mut WorldReservationLedger,
    ) -> Result<(), TradeError> {
        let mut candidate = self.clone();
        let mut candidate_world = world.clone();
        candidate.mutate_contract(id, expected_version, |contract| {
            if contract.stage != TradeStage::Returning {
                return Err(TradeError::InvalidTransition);
            }
            release_all_atomic(&mut candidate_world, &contract.escrow_reservation_ids)?;
            for leg in &mut contract.proposal.legs {
                leg.reset_to_source();
            }
            contract.escrow_reservation_ids.clear();
            contract.stage = TradeStage::Cancelled;
            contract.recovery = TradeRecoveryState::Returned;
            contract.next_event_tick = None;
            Ok(())
        })?;
        *self = candidate;
        *world = candidate_world;
        Ok(())
    }

    pub fn carrier_failed(
        &mut self,
        id: &TradeContractId,
        expected_version: u64,
        reason: TradeBlockReason,
        dispositions: &[RecoveryDisposition],
        now_tick: u64,
        world: &mut WorldReservationLedger,
    ) -> Result<(), TradeError> {
        if !matches!(
            reason,
            TradeBlockReason::WorkerDied | TradeBlockReason::WorkerRefused
        ) {
            return Err(TradeError::MalformedContract);
        }
        let disposition_by_cargo = dispositions
            .iter()
            .map(|disposition| (disposition.cargo_id.as_str(), disposition))
            .collect::<BTreeMap<_, _>>();
        let mut candidate = self.clone();
        let mut candidate_world = world.clone();
        candidate.mutate_contract(id, expected_version, |contract| {
            if !matches!(contract.stage, TradeStage::InTransit | TradeStage::Blocked) {
                return Err(TradeError::InvalidTransition);
            }
            let mut any_stranded = false;
            for leg in &mut contract.proposal.legs {
                let disposition = disposition_by_cargo
                    .get(leg.cargo.cargo_id.as_str())
                    .ok_or(TradeError::MalformedContract)?;
                leg.cargo.location = if let Some(stockpile) = &disposition.safe_owned_stockpile {
                    stockpile
                        .validate()
                        .map_err(|_| TradeError::MalformedContract)?;
                    if !matches!(stockpile, SiteRef::Stockpile { .. }) {
                        return Err(TradeError::MalformedContract);
                    }
                    CargoLocation::SalvagedAtStockpile {
                        stockpile_id: stockpile.stable_id().to_owned(),
                    }
                } else if disposition.last_site_id.is_empty() {
                    return Err(TradeError::MalformedContract);
                } else {
                    any_stranded = true;
                    CargoLocation::Stranded {
                        site_id: disposition.last_site_id.clone(),
                    }
                };
            }
            release_all_atomic(&mut candidate_world, &contract.escrow_reservation_ids)?;
            contract.escrow_reservation_ids.clear();
            contract.stage = if any_stranded {
                TradeStage::Stranded
            } else {
                TradeStage::Blocked
            };
            contract.blocked_reason = if any_stranded { None } else { Some(reason) };
            contract.recovery = if any_stranded {
                TradeRecoveryState::Stranded
            } else {
                TradeRecoveryState::Salvaged
            };
            contract.next_event_tick = if any_stranded {
                None
            } else {
                now_tick.checked_add(1)
            };
            Ok(())
        })?;
        *self = candidate;
        *world = candidate_world;
        Ok(())
    }

    #[must_use]
    pub fn due_contract_ids(&self, now_tick: u64) -> Vec<TradeContractId> {
        let mut due = self
            .contracts
            .values()
            .filter_map(|contract| {
                contract
                    .next_event_tick
                    .filter(|tick| *tick <= now_tick)
                    .map(|tick| (tick, contract.id().clone()))
            })
            .collect::<Vec<_>>();
        due.sort_by(|first, second| first.0.cmp(&second.0).then_with(|| first.1.cmp(&second.1)));
        due.into_iter().map(|(_, id)| id).collect()
    }

    fn mutate_contract(
        &mut self,
        id: &TradeContractId,
        expected_version: u64,
        mutation: impl FnOnce(&mut TradeContract) -> Result<(), TradeError>,
    ) -> Result<(), TradeError> {
        let mut contract = self
            .contracts
            .get(id)
            .cloned()
            .ok_or(TradeError::ContractNotFound)?;
        if contract.version != expected_version {
            return Err(TradeError::StaleVersion {
                expected: expected_version,
                actual: contract.version,
            });
        }
        mutation(&mut contract)?;
        contract.version = contract
            .version
            .checked_add(1)
            .ok_or(TradeError::VersionExhausted)?;
        contract.validate()?;
        self.version = self
            .version
            .checked_add(1)
            .ok_or(TradeError::VersionExhausted)?;
        self.contracts.insert(id.clone(), contract);
        Ok(())
    }

    fn validate(&self) -> Result<(), TradeError> {
        if (!self.contracts.is_empty() && self.version == 0)
            || self
                .contracts
                .iter()
                .any(|(id, contract)| id != contract.id() || contract.validate().is_err())
        {
            return Err(TradeError::MalformedPersistence);
        }
        for receipt in self.action_receipts.values() {
            let Some(contract) = self.contracts.get(&receipt.contract_id) else {
                return Err(TradeError::MalformedPersistence);
            };
            if !contract.proposal.pair.contains(&receipt.acting_colony)
                || receipt.resulting_version == 0
                || receipt.resulting_version > contract.version
            {
                return Err(TradeError::MalformedPersistence);
            }
        }
        Ok(())
    }
}

impl Default for TradeLedger {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_authorization(
    acting_colony: &DiplomacyColonyId,
    authorization: &TradeAuthorization,
) -> Result<(), TradeError> {
    if acting_colony != &authorization.acting_colony {
        return Err(TradeError::AuthorizationColonyMismatch);
    }
    if !authorization.authorized_for_colony {
        return Err(TradeError::AuthorizationDenied(
            AuthorityDenial::PlayerNotAuthorized,
        ));
    }
    let operation = match &authorization.actor {
        AuthorityActor::God { .. } => AuthorityOperation::ApproveDiplomacy,
        _ => AuthorityOperation::ApproveIntent,
    };
    let decision = decide_authority(
        &authorization.actor,
        operation,
        AuthorityDomain::Trade,
        AuthorityContext {
            leader_present: true,
            player_authorized: authorization.authorized_for_colony,
        },
    );
    if let AuthorityDecision::Denied(reason) = decision {
        return Err(TradeError::AuthorizationDenied(reason));
    }
    if let AuthorityActor::God { player_id } = &authorization.actor
        && authorization.owner_player_id.as_ref() != Some(player_id)
    {
        return Err(TradeError::PlayerIdentityMismatch);
    }
    Ok(())
}

fn release_all_atomic(
    world: &mut WorldReservationLedger,
    ids: &[WorldReservationId],
) -> Result<(), TradeError> {
    let mut candidate = world.clone();
    for id in ids {
        match candidate.release(id).map_err(TradeError::Escrow)? {
            WorldReleaseOutcome::Released | WorldReleaseOutcome::NotFound => {}
        }
    }
    *world = candidate;
    Ok(())
}

fn valid_route(site: &SiteRef) -> bool {
    let SiteRef::OrderedRoute { metadata, route } = site else {
        return false;
    };
    site.validate().is_ok()
        && metadata.visibility != SiteVisibility::Hidden
        && metadata.lifecycle == SiteLifecycleStage::Active
        && metadata.blocked_reason.is_none()
        && !route.is_empty()
        && route.windows(2).all(|pair| {
            i64::from(pair[0].x).abs_diff(i64::from(pair[1].x))
                + i64::from(pair[0].y).abs_diff(i64::from(pair[1].y))
                == 1
        })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedTradeLedger<'a> {
    schema_version: u32,
    version: u64,
    contracts: Vec<&'a TradeContract>,
    action_receipts: Vec<&'a TradeReceipt>,
}

impl Serialize for TradeLedger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        PersistedTradeLedger {
            schema_version: TRADE_SCHEMA_VERSION,
            version: self.version,
            contracts: self.contracts.values().collect(),
            action_receipts: self.action_receipts.values().collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TradeLedger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct PersistedTradeLedger {
            schema_version: u32,
            #[serde(default)]
            version: u64,
            #[serde(default)]
            contracts: Vec<TradeContract>,
            #[serde(default)]
            action_receipts: Vec<TradeReceipt>,
        }

        let persisted = PersistedTradeLedger::deserialize(deserializer)?;
        if persisted.schema_version != TRADE_SCHEMA_VERSION {
            return Err(serde::de::Error::custom("unsupported trade schema version"));
        }
        let mut contracts = BTreeMap::new();
        let mut previous_contract_id = None;
        for contract in persisted.contracts {
            contract.validate().map_err(serde::de::Error::custom)?;
            if previous_contract_id
                .as_ref()
                .is_some_and(|previous| previous >= contract.id())
            {
                return Err(serde::de::Error::custom(
                    "trade contracts are not in canonical ID order",
                ));
            }
            previous_contract_id = Some(contract.id().clone());
            if contracts.insert(contract.id().clone(), contract).is_some() {
                return Err(serde::de::Error::custom("duplicate trade contract"));
            }
        }
        let mut action_receipts = BTreeMap::new();
        let mut previous_action_id = None;
        for receipt in persisted.action_receipts {
            if previous_action_id
                .as_ref()
                .is_some_and(|previous| previous >= &receipt.action_id)
            {
                return Err(serde::de::Error::custom(
                    "trade action receipts are not in canonical ID order",
                ));
            }
            previous_action_id = Some(receipt.action_id.clone());
            if action_receipts
                .insert(receipt.action_id.clone(), receipt)
                .is_some()
            {
                return Err(serde::de::Error::custom("duplicate trade action receipt"));
            }
        }
        let ledger = Self {
            version: persisted.version,
            contracts,
            action_receipts,
        };
        ledger.validate().map_err(serde::de::Error::custom)?;
        Ok(ledger)
    }
}
