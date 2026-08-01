//! LAI.62-A canonical personal stance and physical barter authority.
//!
//! `TradeLedger` remains the one mutable route, escrow, hauler, and recovery
//! state machine.  This aggregate owns directional personal stances and the
//! canonical content-lot bindings for those contracts; it never owns a second
//! quantity ledger.  Runtime adapters belong to LAI.63.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    autonomous_trade::{
        DeliveryValidation, RecoveryDisposition, TradeAction, TradeAuthorization, TradeBlockReason,
        TradeContract, TradeContractId, TradeError, TradeLedger, TradeProposal, TradeReceipt,
    },
    diplomacy::{DiplomacyColonyId, DiplomacyRelationship},
    moneyless_barter::{BarterContract, GLOBAL_COLONY_EXTERNAL_ID, PersonalStance, StableId},
    moneyless_barter::{TradePostureDecision, TradeScoreInputs, choose_posture},
    storage_authority::{StorageAuthority, StorageIdentity},
    world_reservations::WorldReservationLedger,
};

pub const TRADE_AUTHORITY_SCHEMA_VERSION: u32 = 1;
pub const MAX_TRADE_AUTHORITY_STANCES: usize = 4_096;
pub const MAX_TRADE_AUTHORITY_RECEIPTS: usize = 1_024;
pub const MAX_CONTENT_LOT_BINDINGS_PER_CONTRACT: usize = 64;
/// A report page is deliberately smaller than the persisted trade ledger.
/// The caller receives a typed truncation marker instead of silently walking
/// unbounded historical contracts during a snapshot projection.
pub const MAX_TRADE_REPORT_CONTRACTS: usize = 1_024;

pub type TradeAuthorityCommandId = String;
pub type TradeAuthorityFingerprint = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradeAuthorityError {
    UnsupportedVersion(u32),
    EmptyStableId,
    SameVillage,
    GlobalVillageLockedNeutral,
    EnemyRejected,
    StaleVersion { expected: u64, actual: u64 },
    ReplayConflict,
    TooManyStances,
    TooManyReceipts,
    TooManyContentLots,
    MissingContentContract,
    ContentContractMismatch,
    DuplicateContentLot,
    NonCanonicalContentLots,
    VersionExhausted,
    Trade(TradeError),
    Invariant(&'static str),
}

impl fmt::Display for TradeAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid canonical trade authority state: {self:?}"
        )
    }
}

impl std::error::Error for TradeAuthorityError {}

impl From<TradeError> for TradeAuthorityError {
    fn from(value: TradeError) -> Self {
        Self::Trade(value)
    }
}

fn stable(value: &str) -> Result<(), TradeAuthorityError> {
    if value.trim().is_empty() {
        Err(TradeAuthorityError::EmptyStableId)
    } else {
        Ok(())
    }
}

fn is_global(colony: &DiplomacyColonyId) -> bool {
    colony.external_id() == GLOBAL_COLONY_EXTERNAL_ID
}

/// A directional stance: `from` can mark `to` Enemy without changing `to`'s
/// independent view.  It is a BTree key so report order and persistence order
/// are deterministic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectedStance {
    pub from: DiplomacyColonyId,
    pub to: DiplomacyColonyId,
}

impl DirectedStance {
    pub fn new(
        from: DiplomacyColonyId,
        to: DiplomacyColonyId,
    ) -> Result<Self, TradeAuthorityError> {
        if from == to {
            return Err(TradeAuthorityError::SameVillage);
        }
        Ok(Self { from, to })
    }

    fn validate(&self) -> Result<(), TradeAuthorityError> {
        if self.from == self.to {
            return Err(TradeAuthorityError::SameVillage);
        }
        Ok(())
    }
}

/// A content ID remains attached to its physical storage identity until the
/// LAI.63 adapter turns the binding into a reservation.  No amount is copied
/// here: `StorageAuthority` and its `QualityLotLedger` retain that authority.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeContentLotBinding {
    pub content_lot_id: StableId,
    pub storage_identity: StorageIdentity,
}

impl TradeContentLotBinding {
    fn validate(&self) -> Result<(), TradeAuthorityError> {
        self.content_lot_id
            .validate()
            .map_err(|_| TradeAuthorityError::ContentContractMismatch)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeAuthorityReceipt {
    pub command_id: TradeAuthorityCommandId,
    pub fingerprint: TradeAuthorityFingerprint,
    pub resulting_version: u64,
    pub contract_id: Option<TradeContractId>,
    pub outcome: TradeAuthorityOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeAuthorityOutcome {
    StanceStored,
    Proposed,
    Accepted,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeAuthoritySummary {
    pub version: u64,
    pub stance_count: usize,
    pub contract_count: usize,
    pub active_contract_count: usize,
    pub bounded_reason: Option<&'static str>,
}

/// A stable-order, read-only report page.  It is intentionally not a mutable
/// ledger view: the page carries only references to the authority's stored
/// IDs and lifecycle data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeReportPage<T> {
    pub entries: Vec<T>,
    pub truncated: bool,
}

/// The only personal stance surface a selected village may see.  A report is
/// directional, so it always exposes the selected village as `from` and never
/// reveals an unrelated village-to-village relationship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradePersonalStanceReport<'a> {
    pub from: &'a DiplomacyColonyId,
    pub to: &'a DiplomacyColonyId,
    pub stance: PersonalStance,
}

/// A report-safe contract header.  Valuation beliefs, player/auth data, and
/// copied barter quantities deliberately remain behind their owning leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradeContractReport<'a> {
    pub contract_id: &'a TradeContractId,
    pub counterpart_colony_id: &'a DiplomacyColonyId,
    pub version: u64,
    pub stage: crate::autonomous_trade::TradeStage,
    pub viewer_consented: bool,
    pub counterpart_consented: bool,
    pub blocked_reason: Option<TradeBlockReason>,
    pub recovery: crate::autonomous_trade::TradeRecoveryState,
}

/// Physical barter progress is reported by the exact content/storage identity
/// already bound to the contract.  A counterparty's storage identity is
/// deliberately filtered: the trade report can name the agreed content ID and
/// stage but must not expose another colony's private storage topology.  No
/// quantity is copied here; resolving a local quantity is the explicit
/// responsibility of `StorageAuthority` and its `QualityLotLedger` at the
/// projection boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeStorageIdentityReport<'a> {
    Owned(&'a StorageIdentity),
    ForeignFiltered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TradePhysicalStageReport<'a> {
    pub contract_id: &'a TradeContractId,
    pub content_lot_id: &'a StableId,
    pub storage_identity: TradeStorageIdentityReport<'a>,
    pub stage: crate::autonomous_trade::TradeStage,
}

/// Missing and foreign contract IDs intentionally share one result.  This
/// prevents a selected colony from probing another colony's private contract
/// existence through the report API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeReportUnavailable {
    NotVisible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeContractReportAccess<'a> {
    Visible(TradeContractReport<'a>),
    Unavailable(TradeReportUnavailable),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TradePhysicalStagesReportAccess<'a> {
    Visible(TradeReportPage<TradePhysicalStageReport<'a>>),
    Unavailable(TradeReportUnavailable),
}

/// The strict, versioned aggregate.  `contracts` is the only physical route
/// contract ledger.  `content_lots` adds exact item/lot identity to each of
/// those same contract IDs without shadowing any storage quantity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeAuthority {
    pub schema_version: u32,
    version: u64,
    stances: BTreeMap<DirectedStance, PersonalStance>,
    contracts: TradeLedger,
    content_lots: BTreeMap<TradeContractId, Vec<TradeContentLotBinding>>,
    content_contracts: BTreeMap<TradeContractId, BarterContract>,
    receipts: BTreeMap<TradeAuthorityCommandId, TradeAuthorityReceipt>,
}

impl TradeAuthority {
    #[must_use]
    pub fn new() -> Self {
        Self {
            schema_version: TRADE_AUTHORITY_SCHEMA_VERSION,
            version: 0,
            stances: BTreeMap::new(),
            contracts: TradeLedger::new(),
            content_lots: BTreeMap::new(),
            content_contracts: BTreeMap::new(),
            receipts: BTreeMap::new(),
        }
    }

    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }

    #[must_use]
    pub fn stance(&self, from: &DiplomacyColonyId, to: &DiplomacyColonyId) -> PersonalStance {
        if is_global(from) || is_global(to) {
            return PersonalStance::Neutral;
        }
        self.stances
            .get(&DirectedStance {
                from: from.clone(),
                to: to.clone(),
            })
            .copied()
            .unwrap_or(PersonalStance::Neutral)
    }

    #[must_use]
    pub fn contract(&self, id: &TradeContractId) -> Option<&TradeContract> {
        self.contracts.contract(id)
    }

    #[must_use]
    pub fn content_lots(&self, id: &TradeContractId) -> Option<&[TradeContentLotBinding]> {
        self.content_lots.get(id).map(Vec::as_slice)
    }

    #[must_use]
    pub fn receipt(&self, command_id: &str) -> Option<&TradeAuthorityReceipt> {
        self.receipts.get(command_id)
    }

    /// Returns only directional personal stances authored by `viewer`, in
    /// canonical destination-ID order.  Unstored neutral defaults and
    /// unrelated foreign stances remain absent rather than being fabricated.
    #[must_use]
    pub fn report_personal_stances_for(
        &self,
        viewer: &DiplomacyColonyId,
    ) -> TradeReportPage<TradePersonalStanceReport<'_>> {
        let mut entries = Vec::new();
        let mut truncated = false;
        for (directed, stance) in &self.stances {
            if &directed.from != viewer {
                continue;
            }
            if entries.len() == MAX_TRADE_REPORT_CONTRACTS {
                truncated = true;
                break;
            }
            entries.push(TradePersonalStanceReport {
                from: &directed.from,
                to: &directed.to,
                stance: *stance,
            });
        }
        TradeReportPage { entries, truncated }
    }

    /// Returns contracts to which `viewer` is a party, in canonical contract
    /// ID order.  A filtered report never surfaces a third-party contract.
    #[must_use]
    pub fn report_contracts_for(
        &self,
        viewer: &DiplomacyColonyId,
    ) -> TradeReportPage<TradeContractReport<'_>> {
        let mut entries = Vec::new();
        let mut truncated = false;
        for contract in self.contracts.contracts() {
            let Some(report) = self.contract_report_for(viewer, contract) else {
                continue;
            };
            if entries.len() == MAX_TRADE_REPORT_CONTRACTS {
                truncated = true;
                break;
            }
            entries.push(report);
        }
        TradeReportPage { entries, truncated }
    }

    /// Looks up one selected-colony contract without distinguishing a missing
    /// ID from a private foreign ID.
    #[must_use]
    pub fn report_contract_for(
        &self,
        viewer: &DiplomacyColonyId,
        contract_id: &TradeContractId,
    ) -> TradeContractReportAccess<'_> {
        self.contracts
            .contract(contract_id)
            .and_then(|contract| self.contract_report_for(viewer, contract))
            .map_or(
                TradeContractReportAccess::Unavailable(TradeReportUnavailable::NotVisible),
                TradeContractReportAccess::Visible,
            )
    }

    /// Returns the selected contract's exact physical content identities and
    /// current lifecycle stage.  The report intentionally has no quantity,
    /// cargo copy, route score, or private foreign valuation.
    #[must_use]
    pub fn report_physical_stages_for(
        &self,
        viewer: &DiplomacyColonyId,
        contract_id: &TradeContractId,
    ) -> TradePhysicalStagesReportAccess<'_> {
        let Some(contract) = self.contracts.contract(contract_id) else {
            return TradePhysicalStagesReportAccess::Unavailable(
                TradeReportUnavailable::NotVisible,
            );
        };
        if self.contract_report_for(viewer, contract).is_none() {
            return TradePhysicalStagesReportAccess::Unavailable(
                TradeReportUnavailable::NotVisible,
            );
        }
        let Some(bindings) = self.content_lots.get(contract_id) else {
            // A validated authority cannot reach this branch, but keep a
            // private-state failure from becoming an existence side channel.
            return TradePhysicalStagesReportAccess::Unavailable(
                TradeReportUnavailable::NotVisible,
            );
        };
        let truncated = bindings.len() > MAX_CONTENT_LOT_BINDINGS_PER_CONTRACT;
        let entries = bindings
            .iter()
            .take(MAX_CONTENT_LOT_BINDINGS_PER_CONTRACT)
            .map(|binding| TradePhysicalStageReport {
                contract_id: contract.id(),
                content_lot_id: &binding.content_lot_id,
                storage_identity: self.report_storage_identity_for(viewer, contract_id, binding),
                stage: contract.stage,
            })
            .collect();
        TradePhysicalStagesReportAccess::Visible(TradeReportPage { entries, truncated })
    }

    #[must_use]
    pub fn summary(&self) -> TradeAuthoritySummary {
        let active_contract_count = self
            .contracts
            .contracts()
            .filter(|contract| !contract.stage.is_terminal())
            .count();
        TradeAuthoritySummary {
            version: self.version,
            stance_count: self.stances.len(),
            contract_count: self.contracts.contracts().len(),
            active_contract_count,
            bounded_reason: None,
        }
    }

    fn contract_report_for<'a>(
        &self,
        viewer: &DiplomacyColonyId,
        contract: &'a TradeContract,
    ) -> Option<TradeContractReport<'a>> {
        let counterpart_colony_id = contract
            .proposal
            .parties
            .keys()
            .find(|party| *party != viewer)?;
        if !contract.proposal.parties.contains_key(viewer) {
            return None;
        }
        Some(TradeContractReport {
            contract_id: contract.id(),
            counterpart_colony_id,
            version: contract.version,
            stage: contract.stage,
            viewer_consented: contract.acceptances.contains(viewer),
            counterpart_consented: contract.acceptances.contains(counterpart_colony_id),
            blocked_reason: contract.blocked_reason,
            recovery: contract.recovery,
        })
    }

    fn report_storage_identity_for<'a>(
        &'a self,
        viewer: &DiplomacyColonyId,
        contract_id: &TradeContractId,
        binding: &'a TradeContentLotBinding,
    ) -> TradeStorageIdentityReport<'a> {
        let Some(content_contract) = self.content_contracts.get(contract_id) else {
            return TradeStorageIdentityReport::ForeignFiltered;
        };
        let local_external_id = viewer.external_id();
        let offer = &content_contract.offer;
        let owner_external_id = if offer
            .offered
            .iter()
            .any(|lot| &lot.lot_id == &binding.content_lot_id)
        {
            Some(offer.source.external_id.as_str())
        } else if offer
            .requested
            .iter()
            .any(|lot| &lot.lot_id == &binding.content_lot_id)
        {
            Some(offer.destination.external_id.as_str())
        } else {
            None
        };
        if owner_external_id == Some(local_external_id) {
            TradeStorageIdentityReport::Owned(&binding.storage_identity)
        } else {
            TradeStorageIdentityReport::ForeignFiltered
        }
    }

    /// Uses only report-safe score inputs.  The returned posture is a planner
    /// recommendation; it neither reveals hidden state nor opens a contract.
    pub fn evaluate_posture(
        &self,
        inputs: TradeScoreInputs,
    ) -> Result<TradePostureDecision, TradeAuthorityError> {
        choose_posture(inputs).map_err(|_| TradeAuthorityError::Invariant("invalid report score"))
    }

    /// Exposes the pre-reservation gate to planner/report callers without
    /// creating any route state.
    pub fn authorize_dispatch(
        &self,
        source: &DiplomacyColonyId,
        destination: &DiplomacyColonyId,
    ) -> Result<(), TradeAuthorityError> {
        self.pre_dispatch(source, destination)
    }

    pub fn set_stance(
        &mut self,
        command_id: impl Into<String>,
        fingerprint: impl Into<String>,
        expected_version: u64,
        from: DiplomacyColonyId,
        to: DiplomacyColonyId,
        stance: PersonalStance,
    ) -> Result<TradeAuthorityReceipt, TradeAuthorityError> {
        let command_id = command_id.into();
        let fingerprint = fingerprint.into();
        stable(&command_id)?;
        stable(&fingerprint)?;
        if let Some(receipt) = self.replay(&command_id, &fingerprint)? {
            return Ok(receipt);
        }
        self.expect_version(expected_version)?;
        let key = DirectedStance::new(from, to)?;
        if (is_global(&key.from) || is_global(&key.to)) && stance != PersonalStance::Neutral {
            return Err(TradeAuthorityError::GlobalVillageLockedNeutral);
        }
        if self.stances.len() >= MAX_TRADE_AUTHORITY_STANCES && !self.stances.contains_key(&key) {
            return Err(TradeAuthorityError::TooManyStances);
        }
        let mut staged = self.clone();
        staged.stances.insert(key, stance);
        staged.bump_version()?;
        let receipt = TradeAuthorityReceipt {
            command_id,
            fingerprint,
            resulting_version: staged.version,
            contract_id: None,
            outcome: TradeAuthorityOutcome::StanceStored,
        };
        staged.store_receipt(receipt.clone())?;
        staged.validate()?;
        *self = staged;
        Ok(receipt)
    }

    /// Creates no reservation, escrow, hauler, or route unless both
    /// directional stances pass first.  Alliance and Neutral both map to the
    /// same adapter relationship, deliberately making their mechanics equal.
    pub fn propose(
        &mut self,
        command_id: impl Into<String>,
        fingerprint: impl Into<String>,
        expected_version: u64,
        proposal: TradeProposal,
        content_contract: BarterContract,
        mut content_lots: Vec<TradeContentLotBinding>,
        authorization: &TradeAuthorization,
    ) -> Result<TradeAuthorityReceipt, TradeAuthorityError> {
        let command_id = command_id.into();
        let fingerprint = fingerprint.into();
        stable(&command_id)?;
        stable(&fingerprint)?;
        if let Some(receipt) = self.replay(&command_id, &fingerprint)? {
            return Ok(receipt);
        }
        self.expect_version(expected_version)?;
        let target = other_party(&proposal, &proposal.initiator)?;
        self.pre_dispatch(&proposal.initiator, &target)?;
        content_contract
            .validate()
            .map_err(|_| TradeAuthorityError::ContentContractMismatch)?;
        if content_contract.offer.source.external_id != proposal.initiator.external_id()
            || content_contract.offer.destination.external_id != target.external_id()
        {
            return Err(TradeAuthorityError::ContentContractMismatch);
        }
        // The old enum is an adapter only.  Both allowed personal labels use
        // Friendly, keeping bounds and transitions identical.
        if proposal.relationship != DiplomacyRelationship::Friendly {
            return Err(TradeAuthorityError::ContentContractMismatch);
        }
        if content_lots.len() > MAX_CONTENT_LOT_BINDINGS_PER_CONTRACT {
            return Err(TradeAuthorityError::TooManyContentLots);
        }
        content_lots.sort();
        if content_lots.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(TradeAuthorityError::DuplicateContentLot);
        }
        for binding in &content_lots {
            binding.validate()?;
        }
        let expected_content_ids = content_contract
            .offer
            .offered
            .iter()
            .chain(&content_contract.offer.requested)
            .map(|lot| lot.lot_id.clone())
            .collect::<BTreeSet<_>>();
        let bound_content_ids = content_lots
            .iter()
            .map(|binding| binding.content_lot_id.clone())
            .collect::<BTreeSet<_>>();
        if expected_content_ids != bound_content_ids {
            return Err(TradeAuthorityError::ContentContractMismatch);
        }
        let mut staged = self.clone();
        let contract_id = staged.contracts.propose(proposal, authorization)?;
        if staged.content_lots.contains_key(&contract_id) {
            return Err(TradeAuthorityError::ContentContractMismatch);
        }
        staged
            .content_lots
            .insert(contract_id.clone(), content_lots);
        staged
            .content_contracts
            .insert(contract_id.clone(), content_contract);
        staged.bump_version()?;
        let receipt = TradeAuthorityReceipt {
            command_id,
            fingerprint,
            resulting_version: staged.version,
            contract_id: Some(contract_id),
            outcome: TradeAuthorityOutcome::Proposed,
        };
        staged.store_receipt(receipt.clone())?;
        staged.validate()?;
        *self = staged;
        Ok(receipt)
    }

    /// Applies mutual consent and atomic escrow using the existing physical
    /// ledger.  An Enemy response is rejected before that ledger can change.
    pub fn apply_action(
        &mut self,
        action: TradeAction,
        authorization: &TradeAuthorization,
        now_tick: u64,
        world: &mut WorldReservationLedger,
    ) -> Result<TradeReceipt, TradeAuthorityError> {
        let contract = self
            .contracts
            .contract(&action.contract_id)
            .ok_or(TradeAuthorityError::MissingContentContract)?;
        let counterpart = other_party(&contract.proposal, &action.acting_colony)?;
        self.pre_dispatch(&action.acting_colony, &counterpart)?;
        let mut staged = self.clone();
        let mut staged_world = world.clone();
        let trade_version_before = staged.contracts.version();
        let receipt = staged.contracts.apply_action(
            action,
            authorization,
            DiplomacyRelationship::Friendly,
            now_tick,
            &mut staged_world,
        )?;
        if staged.contracts.version() != trade_version_before {
            staged.bump_version()?;
        }
        staged.validate()?;
        *self = staged;
        *world = staged_world;
        Ok(receipt)
    }

    pub fn depart(
        &mut self,
        contract_id: &TradeContractId,
        expected_version: u64,
        now_tick: u64,
        world: &WorldReservationLedger,
    ) -> Result<(), TradeAuthorityError> {
        let mut staged = self.clone();
        staged
            .contracts
            .depart(contract_id, expected_version, now_tick, world)?;
        staged.bump_version()?;
        staged.validate()?;
        *self = staged;
        Ok(())
    }

    pub fn attempt_delivery(
        &mut self,
        contract_id: &TradeContractId,
        expected_version: u64,
        validation: DeliveryValidation,
        now_tick: u64,
        world: &mut WorldReservationLedger,
    ) -> Result<(), TradeAuthorityError> {
        let mut staged = self.clone();
        let mut staged_world = world.clone();
        staged.contracts.attempt_delivery(
            contract_id,
            expected_version,
            validation,
            now_tick,
            &mut staged_world,
        )?;
        staged.bump_version()?;
        staged.validate()?;
        *self = staged;
        *world = staged_world;
        Ok(())
    }

    pub fn carrier_failed(
        &mut self,
        contract_id: &TradeContractId,
        expected_version: u64,
        reason: TradeBlockReason,
        dispositions: &[RecoveryDisposition],
        now_tick: u64,
        world: &mut WorldReservationLedger,
    ) -> Result<(), TradeAuthorityError> {
        let mut staged = self.clone();
        let mut staged_world = world.clone();
        staged.contracts.carrier_failed(
            contract_id,
            expected_version,
            reason,
            dispositions,
            now_tick,
            &mut staged_world,
        )?;
        staged.bump_version()?;
        staged.validate()?;
        *self = staged;
        *world = staged_world;
        Ok(())
    }

    /// LAI.63 calls this before emitting a physical reservation.  The storage
    /// authority remains sole owner of the selected identity and quantity.
    pub fn verify_storage_binding(
        &self,
        storage: &StorageAuthority,
        binding: &TradeContentLotBinding,
    ) -> bool {
        storage.location(&binding.storage_identity).is_some()
    }

    pub fn validate(&self) -> Result<(), TradeAuthorityError> {
        if self.schema_version != TRADE_AUTHORITY_SCHEMA_VERSION {
            return Err(TradeAuthorityError::UnsupportedVersion(self.schema_version));
        }
        if self.stances.len() > MAX_TRADE_AUTHORITY_STANCES
            || self.receipts.len() > MAX_TRADE_AUTHORITY_RECEIPTS
        {
            return Err(TradeAuthorityError::Invariant("bounded maps exceeded"));
        }
        for (key, stance) in &self.stances {
            key.validate()?;
            if (is_global(&key.from) || is_global(&key.to)) && *stance != PersonalStance::Neutral {
                return Err(TradeAuthorityError::GlobalVillageLockedNeutral);
            }
        }
        for contract in self.contracts.contracts() {
            let id = contract.id();
            let lots = self
                .content_lots
                .get(id)
                .ok_or(TradeAuthorityError::MissingContentContract)?;
            let content = self
                .content_contracts
                .get(id)
                .ok_or(TradeAuthorityError::MissingContentContract)?;
            content
                .validate()
                .map_err(|_| TradeAuthorityError::ContentContractMismatch)?;
            if lots.len() > MAX_CONTENT_LOT_BINDINGS_PER_CONTRACT
                || lots.windows(2).any(|pair| pair[0] >= pair[1])
            {
                return Err(TradeAuthorityError::NonCanonicalContentLots);
            }
            for lot in lots {
                lot.validate()?;
            }
        }
        if self.content_lots.len() != self.contracts.contracts().len()
            || self.content_contracts.len() != self.contracts.contracts().len()
        {
            return Err(TradeAuthorityError::Invariant(
                "content and route contracts diverged",
            ));
        }
        Ok(())
    }

    fn pre_dispatch(
        &self,
        source: &DiplomacyColonyId,
        destination: &DiplomacyColonyId,
    ) -> Result<(), TradeAuthorityError> {
        if self.stance(source, destination).is_enemy()
            || self.stance(destination, source).is_enemy()
        {
            return Err(TradeAuthorityError::EnemyRejected);
        }
        Ok(())
    }

    fn expect_version(&self, expected: u64) -> Result<(), TradeAuthorityError> {
        if expected == self.version {
            Ok(())
        } else {
            Err(TradeAuthorityError::StaleVersion {
                expected,
                actual: self.version,
            })
        }
    }

    fn replay(
        &self,
        command_id: &str,
        fingerprint: &str,
    ) -> Result<Option<TradeAuthorityReceipt>, TradeAuthorityError> {
        match self.receipts.get(command_id) {
            Some(receipt) if receipt.fingerprint == fingerprint => Ok(Some(receipt.clone())),
            Some(_) => Err(TradeAuthorityError::ReplayConflict),
            None => Ok(None),
        }
    }

    fn store_receipt(
        &mut self,
        receipt: TradeAuthorityReceipt,
    ) -> Result<TradeAuthorityReceipt, TradeAuthorityError> {
        if self.receipts.len() >= MAX_TRADE_AUTHORITY_RECEIPTS {
            return Err(TradeAuthorityError::TooManyReceipts);
        }
        self.receipts
            .insert(receipt.command_id.clone(), receipt.clone());
        Ok(receipt)
    }

    fn bump_version(&mut self) -> Result<(), TradeAuthorityError> {
        self.version = self
            .version
            .checked_add(1)
            .ok_or(TradeAuthorityError::VersionExhausted)?;
        Ok(())
    }
}

impl Default for TradeAuthority {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedTradeAuthority<'a> {
    schema_version: u32,
    version: u64,
    stances: Vec<PersistedStanceRef<'a>>,
    contracts: &'a TradeLedger,
    content: Vec<PersistedContentRef<'a>>,
    receipts: Vec<&'a TradeAuthorityReceipt>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedStanceRef<'a> {
    key: &'a DirectedStance,
    stance: PersonalStance,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PersistedContentRef<'a> {
    contract_id: &'a TradeContractId,
    lots: &'a [TradeContentLotBinding],
    contract: &'a BarterContract,
}

impl Serialize for TradeAuthority {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        PersistedTradeAuthority {
            schema_version: self.schema_version,
            version: self.version,
            stances: self
                .stances
                .iter()
                .map(|(key, stance)| PersistedStanceRef {
                    key,
                    stance: *stance,
                })
                .collect(),
            contracts: &self.contracts,
            content: self
                .content_lots
                .iter()
                .filter_map(|(contract_id, lots)| {
                    self.content_contracts
                        .get(contract_id)
                        .map(|contract| PersistedContentRef {
                            contract_id,
                            lots,
                            contract,
                        })
                })
                .collect(),
            receipts: self.receipts.values().collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TradeAuthority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct PersistedStance {
            key: DirectedStance,
            stance: PersonalStance,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct PersistedContent {
            contract_id: TradeContractId,
            lots: Vec<TradeContentLotBinding>,
            contract: BarterContract,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct PersistedTradeAuthority {
            schema_version: u32,
            version: u64,
            stances: Vec<PersistedStance>,
            contracts: TradeLedger,
            content: Vec<PersistedContent>,
            receipts: Vec<TradeAuthorityReceipt>,
        }
        let persisted = PersistedTradeAuthority::deserialize(deserializer)?;
        if persisted.schema_version != TRADE_AUTHORITY_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(
                "unsupported trade authority schema version",
            ));
        }
        let mut stances = BTreeMap::new();
        let mut prior_stance = None;
        for entry in persisted.stances {
            entry.key.validate().map_err(serde::de::Error::custom)?;
            if prior_stance
                .as_ref()
                .is_some_and(|prior: &DirectedStance| prior >= &entry.key)
            {
                return Err(serde::de::Error::custom(
                    "stances are not in canonical order",
                ));
            }
            prior_stance = Some(entry.key.clone());
            if stances.insert(entry.key, entry.stance).is_some() {
                return Err(serde::de::Error::custom("duplicate directional stance"));
            }
        }
        let mut content_lots = BTreeMap::new();
        let mut content_contracts = BTreeMap::new();
        let mut prior_content = None;
        for entry in persisted.content {
            if prior_content
                .as_ref()
                .is_some_and(|prior: &TradeContractId| prior >= &entry.contract_id)
            {
                return Err(serde::de::Error::custom(
                    "content contracts are not in canonical order",
                ));
            }
            prior_content = Some(entry.contract_id.clone());
            if content_lots
                .insert(entry.contract_id.clone(), entry.lots)
                .is_some()
                || content_contracts
                    .insert(entry.contract_id, entry.contract)
                    .is_some()
            {
                return Err(serde::de::Error::custom("duplicate content contract"));
            }
        }
        let mut receipts = BTreeMap::new();
        let mut prior_receipt = None;
        for receipt in persisted.receipts {
            stable(&receipt.command_id).map_err(serde::de::Error::custom)?;
            stable(&receipt.fingerprint).map_err(serde::de::Error::custom)?;
            if receipt.resulting_version > persisted.version {
                return Err(serde::de::Error::custom(
                    "receipt version exceeds authority version",
                ));
            }
            if prior_receipt
                .as_ref()
                .is_some_and(|prior: &String| prior >= &receipt.command_id)
            {
                return Err(serde::de::Error::custom(
                    "receipts are not in canonical order",
                ));
            }
            prior_receipt = Some(receipt.command_id.clone());
            if receipts
                .insert(receipt.command_id.clone(), receipt)
                .is_some()
            {
                return Err(serde::de::Error::custom(
                    "duplicate trade authority receipt",
                ));
            }
        }
        let authority = Self {
            schema_version: persisted.schema_version,
            version: persisted.version,
            stances,
            contracts: persisted.contracts,
            content_lots,
            content_contracts,
            receipts,
        };
        authority.validate().map_err(serde::de::Error::custom)?;
        Ok(authority)
    }
}

fn other_party(
    proposal: &TradeProposal,
    actor: &DiplomacyColonyId,
) -> Result<DiplomacyColonyId, TradeAuthorityError> {
    if !proposal.pair.contains(actor) {
        return Err(TradeAuthorityError::SameVillage);
    }
    Ok(if proposal.pair.first() == actor {
        proposal.pair.second().clone()
    } else {
        proposal.pair.first().clone()
    })
}

/// Runtime ownership inventory, deliberately kept beside the authority so the
/// LAI.63 and LAI.70 owners can delete every adapter rather than retain a
/// shadow path.
pub const LAI63_LAI70_ADAPTERS_TO_DELETE: &[&str] = &[
    "crates/cat-sim/src/diplomacy.rs",
    "crates/cat-sim/src/moneyless_barter.rs",
    "crates/cat-sim/src/autonomous_trade.rs",
    "crates/cat-sim/src/trade_valuation.rs",
    "crates/cat-sim/src/trader.rs",
    "crates/cat-sim/src/village_trade_routes.rs",
    "crates/cat-sim/src/world_tick.rs",
    "crates/cat-sim/src/leader_ai_runtime.rs",
    "crates/cat-protocol/src/lai24_snapshot.rs",
    "crates/cat-protocol/src/lai25_action.rs",
    "crates/cat-server/src/leader_ai_action_routing.rs",
    "crates/cat-server/src/leader_ai_journey.rs",
    "crates/cat-server/src/leader_ai_snapshot_projection.rs",
    "crates/cat-server/src/main.rs",
    "crates/cat-client/src/leader_ai_ui/accessibility.rs",
    "crates/cat-client/src/leader_ai_ui/live_render.rs",
    "crates/cat-client/src/leader_ai_ui/mod.rs",
    "crates/cat-client/src/leader_ai_ui/progression.rs",
    "crates/cat-client/src/lib.rs",
];
