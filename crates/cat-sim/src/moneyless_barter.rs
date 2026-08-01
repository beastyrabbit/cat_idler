//! Bounded LAI.62 diplomacy and physical barter contracts.
//!
//! This is an integration leaf for the report-safe diplomacy/trade design in
//! `docs/leader-ai-overhaul/diplomacy-trade.md`.  It deliberately does not
//! mutate `diplomacy`, `autonomous_trade`, or any reservation/ledger authority:
//! callers translate its commands to those existing authorities.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const MONEYLESS_BARTER_SCHEMA_VERSION: u32 = 1;
pub const STABLE_ID_CONTRACT_VERSION: u32 = 1;
pub const REPORT_SCALE: u64 = 1_000_000;
pub const GLOBAL_COLONY_EXTERNAL_ID: &str = "global-village";

/// Explicit LAI.70 deletion inventory.  These are migration/audit roots only;
/// none is a field of an offer, contract, lot, or physical command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyCutoverDisposition {
    Delete,
    RetireNpcRoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyCutoverRoot {
    pub root_id: &'static str,
    pub disposition: LegacyCutoverDisposition,
}

/// LAI.70 audit inventory for the old coin/NPC-trade authority roots.
pub const LAI70_LEGACY_CUTOVER_INVENTORY: &[LegacyCutoverRoot] = &[
    LegacyCutoverRoot {
        root_id: "coin",
        disposition: LegacyCutoverDisposition::Delete,
    },
    LegacyCutoverRoot {
        root_id: "purse",
        disposition: LegacyCutoverDisposition::Delete,
    },
    LegacyCutoverRoot {
        root_id: "monetary-price",
        disposition: LegacyCutoverDisposition::Delete,
    },
    LegacyCutoverRoot {
        root_id: "currency-settlement",
        disposition: LegacyCutoverDisposition::Delete,
    },
    LegacyCutoverRoot {
        root_id: "old-npc-trade-root",
        disposition: LegacyCutoverDisposition::RetireNpcRoot,
    },
    LegacyCutoverRoot {
        root_id: "trader-coin-economy",
        disposition: LegacyCutoverDisposition::RetireNpcRoot,
    },
];

/// Versioned deterministic IDs use FNV-1a over a namespace and ordered parts.
/// This is intentionally local to the additive leaf because `lib.rs` remains
/// owned by the world-tick/protocol cutover owners.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StableId(String);

impl StableId {
    #[must_use]
    pub fn derive(namespace: &str, parts: &[&str]) -> Self {
        let mut hash = 14_695_981_039_346_656_037_u64;
        for part in std::iter::once(namespace).chain(parts.iter().copied()) {
            for byte in part.as_bytes() {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(1_099_511_628_211);
            }
            hash ^= 0xff;
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
        Self(format!(
            "{namespace}/v{STABLE_ID_CONTRACT_VERSION}/{hash:016x}"
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate(&self) -> Result<(), BarterError> {
        let mut parts = self.0.split('/');
        let namespace = parts.next().unwrap_or_default();
        let version = parts.next().unwrap_or_default();
        let digest = parts.next().unwrap_or_default();
        if namespace.is_empty()
            || version != "v1"
            || digest.len() != 16
            || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            || parts.next().is_some()
        {
            return Err(BarterError::MalformedStableId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ColonyId {
    pub stable_id: StableId,
    pub external_id: String,
    pub is_global: bool,
}

impl ColonyId {
    #[must_use]
    pub fn derive(external_id: &str) -> Self {
        Self {
            stable_id: StableId::derive("barter-colony", &[external_id]),
            external_id: external_id.to_owned(),
            is_global: false,
        }
    }

    #[must_use]
    pub fn global() -> Self {
        Self {
            stable_id: StableId::derive("barter-global-colony", &[GLOBAL_COLONY_EXTERNAL_ID]),
            external_id: GLOBAL_COLONY_EXTERNAL_ID.to_owned(),
            is_global: true,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.stable_id.as_str()
    }

    fn validate(&self) -> Result<(), BarterError> {
        self.stable_id.validate()?;
        if self.external_id.is_empty()
            || (self.is_global && self.external_id != GLOBAL_COLONY_EXTERNAL_ID)
            || (!self.is_global && self.external_id == GLOBAL_COLONY_EXTERNAL_ID)
        {
            return Err(BarterError::MalformedColony);
        }
        let expected = if self.is_global {
            Self::global().stable_id
        } else {
            Self::derive(&self.external_id).stable_id
        };
        if self.stable_id != expected {
            return Err(BarterError::MalformedStableId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonalStance {
    Alliance,
    Neutral,
    Enemy,
}

impl PersonalStance {
    #[must_use]
    pub const fn trade_allowed(self) -> bool {
        matches!(self, Self::Alliance | Self::Neutral)
    }

    #[must_use]
    pub const fn is_enemy(self) -> bool {
        matches!(self, Self::Enemy)
    }

    /// Alliance is retained as a future-facing label, but is honest about
    /// having exactly Neutral trade behavior in this release.
    #[must_use]
    pub const fn trade_label(self) -> &'static str {
        match self {
            Self::Alliance => "Alliance (trade-equivalent to Neutral)",
            Self::Neutral => "Neutral",
            Self::Enemy => "Enemy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StanceRecord {
    pub source: ColonyId,
    pub destination: ColonyId,
    pub stance: PersonalStance,
}

impl StanceRecord {
    pub fn new(
        source: ColonyId,
        destination: ColonyId,
        stance: PersonalStance,
    ) -> Result<Self, BarterError> {
        let record = Self {
            source,
            destination,
            stance,
        };
        record.validate()?;
        Ok(record)
    }

    fn validate(&self) -> Result<(), BarterError> {
        self.source.validate()?;
        self.destination.validate()?;
        if self.source == self.destination {
            return Err(BarterError::SameColony);
        }
        if (self.source.is_global || self.destination.is_global)
            && self.stance != PersonalStance::Neutral
        {
            return Err(BarterError::GlobalVillageLockedNeutral);
        }
        Ok(())
    }
}

/// A canonical, immutable view.  Existing diplomacy state remains the sole
/// mutation authority; this view only supplies deterministic reads and gates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StanceSnapshot {
    pub schema_version: u32,
    pub records: Vec<StanceRecord>,
}

impl StanceSnapshot {
    pub fn new(mut records: Vec<StanceRecord>) -> Result<Self, BarterError> {
        records.sort_by(|a, b| {
            a.source
                .cmp(&b.source)
                .then_with(|| a.destination.cmp(&b.destination))
        });
        let snapshot = Self {
            schema_version: MONEYLESS_BARTER_SCHEMA_VERSION,
            records,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<(), BarterError> {
        if self.schema_version != MONEYLESS_BARTER_SCHEMA_VERSION {
            return Err(BarterError::UnsupportedVersion);
        }
        let mut seen = BTreeSet::new();
        for record in &self.records {
            record.validate()?;
            if !seen.insert((record.source.clone(), record.destination.clone())) {
                return Err(BarterError::DuplicateStance);
            }
        }
        if self.records.windows(2).any(|pair| {
            pair[0].source > pair[1].source
                || (pair[0].source == pair[1].source && pair[0].destination > pair[1].destination)
        }) {
            return Err(BarterError::NonCanonicalOrder);
        }
        Ok(())
    }

    #[must_use]
    pub fn stance(&self, source: &ColonyId, destination: &ColonyId) -> PersonalStance {
        self.records
            .iter()
            .find(|record| &record.source == source && &record.destination == destination)
            .map_or(PersonalStance::Neutral, |record| record.stance)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundCandidate {
    pub destination: ColonyId,
    pub source_stance: PersonalStance,
    pub destination_stance: PersonalStance,
}

/// Filters before proposal/escrow creation.  Both local Enemy and a target's
/// Enemy mark are excluded; Alliance and Neutral intentionally share the path.
#[must_use]
pub fn outbound_candidates(
    source: &ColonyId,
    snapshot: &StanceSnapshot,
    destinations: impl IntoIterator<Item = ColonyId>,
) -> Vec<OutboundCandidate> {
    let mut selected = destinations
        .into_iter()
        .filter_map(|destination| {
            if destination == *source {
                return None;
            }
            let source_stance = snapshot.stance(source, &destination);
            let destination_stance = snapshot.stance(&destination, source);
            if source_stance.is_enemy() || destination_stance.is_enemy() {
                return None;
            }
            Some(OutboundCandidate {
                destination,
                source_stance,
                destination_stance,
            })
        })
        .collect::<Vec<_>>();
    selected.sort_by(|a, b| a.destination.cmp(&b.destination));
    selected
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchRequest {
    pub source: ColonyId,
    pub destination: ColonyId,
    pub source_stance: PersonalStance,
    pub destination_stance: PersonalStance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchPermit {
    pub source: ColonyId,
    pub destination: ColonyId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchRejection {
    SameColony,
    SourceMarksEnemy,
    DestinationMarksSenderEnemy,
    GlobalVillageLockedNeutral,
}

/// The only pre-dispatch gate.  A rejection returns no permit, so callers
/// cannot create a caravan, escrow, reservation, or exchange from it.
pub fn pre_dispatch_gate(request: DispatchRequest) -> Result<DispatchPermit, DispatchRejection> {
    if request.source == request.destination {
        return Err(DispatchRejection::SameColony);
    }
    if (request.source.is_global || request.destination.is_global)
        && (request.source_stance != PersonalStance::Neutral
            || request.destination_stance != PersonalStance::Neutral)
    {
        return Err(DispatchRejection::GlobalVillageLockedNeutral);
    }
    if request.source_stance.is_enemy() {
        return Err(DispatchRejection::SourceMarksEnemy);
    }
    if request.destination_stance.is_enemy() {
        return Err(DispatchRejection::DestinationMarksSenderEnemy);
    }
    Ok(DispatchPermit {
        source: request.source,
        destination: request.destination,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalLotKind {
    Material { resource_id: String },
    TypedFood { food_id: String },
    Item { item_id: String },
}

impl PhysicalLotKind {
    fn key(&self) -> &str {
        match self {
            Self::Material { resource_id }
            | Self::TypedFood {
                food_id: resource_id,
            }
            | Self::Item {
                item_id: resource_id,
            } => resource_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhysicalLot {
    pub lot_id: StableId,
    pub kind: PhysicalLotKind,
    pub quantity: u64,
    pub quality_bps: u32,
}

impl PhysicalLot {
    pub fn new(
        lot_id: StableId,
        kind: PhysicalLotKind,
        quantity: u64,
        quality_bps: u32,
    ) -> Result<Self, BarterError> {
        let lot = Self {
            lot_id,
            kind,
            quantity,
            quality_bps,
        };
        lot.validate()?;
        Ok(lot)
    }

    fn validate(&self) -> Result<(), BarterError> {
        self.lot_id.validate()?;
        if self.quantity == 0
            || self.quality_bps > REPORT_SCALE as u32
            || !is_content_id(self.kind.key())
        {
            return Err(BarterError::MalformedPhysicalLot);
        }
        Ok(())
    }
}

/// An offer contains physical lots only.  Comparison/scoring evidence is
/// deliberately separate and cannot become a spendable contract field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BarterOffer {
    pub schema_version: u32,
    pub offer_id: StableId,
    pub source: ColonyId,
    pub destination: ColonyId,
    pub offered: Vec<PhysicalLot>,
    pub requested: Vec<PhysicalLot>,
}

impl BarterOffer {
    pub fn new(
        permit: &DispatchPermit,
        offer_id: StableId,
        mut offered: Vec<PhysicalLot>,
        mut requested: Vec<PhysicalLot>,
    ) -> Result<Self, BarterError> {
        offered.sort_by(|a, b| a.lot_id.cmp(&b.lot_id));
        requested.sort_by(|a, b| a.lot_id.cmp(&b.lot_id));
        let offer = Self {
            schema_version: MONEYLESS_BARTER_SCHEMA_VERSION,
            offer_id,
            source: permit.source.clone(),
            destination: permit.destination.clone(),
            offered,
            requested,
        };
        offer.validate()?;
        Ok(offer)
    }

    pub fn validate(&self) -> Result<(), BarterError> {
        if self.schema_version != MONEYLESS_BARTER_SCHEMA_VERSION
            || self.source == self.destination
            || self.offered.is_empty()
            || self.requested.is_empty()
        {
            return Err(BarterError::MalformedOffer);
        }
        self.source.validate()?;
        self.destination.validate()?;
        self.offer_id.validate()?;
        let mut ids = BTreeSet::new();
        for lot in self.offered.iter().chain(&self.requested) {
            lot.validate()?;
            if !ids.insert(lot.lot_id.clone()) {
                return Err(BarterError::DuplicateLot);
            }
        }
        if self
            .offered
            .windows(2)
            .any(|pair| pair[0].lot_id >= pair[1].lot_id)
            || self
                .requested
                .windows(2)
                .any(|pair| pair[0].lot_id >= pair[1].lot_id)
        {
            return Err(BarterError::NonCanonicalOrder);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractConsent {
    pub required: BTreeSet<ColonyId>,
    pub accepted: BTreeSet<ColonyId>,
}

impl ContractConsent {
    pub fn new(source: ColonyId, destination: ColonyId) -> Self {
        let required = BTreeSet::from([source, destination]);
        Self {
            required,
            accepted: BTreeSet::new(),
        }
    }

    fn validate(&self) -> Result<(), BarterError> {
        if self.required.is_empty() || !self.accepted.is_subset(&self.required) {
            return Err(BarterError::MalformedConsent);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractStage {
    Proposed,
    Accepted,
    Reserved,
    InTransit,
    Delivered,
    Returning,
    Stranded,
    Recovered,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryReason {
    RouteClosed,
    DestinationUnavailable,
    WorkerDied,
    WorkerRefused,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalLocation {
    Source,
    Escrow,
    AssignedToHauler,
    InTransit,
    Destination,
    Returning,
    Stranded,
    Salvaged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhysicalTransferLeg {
    pub lot_id: StableId,
    pub owner: ColonyId,
    pub recipient: ColonyId,
    pub source_endpoint_id: StableId,
    pub destination_endpoint_id: StableId,
    pub reservation_id: StableId,
    pub escrow_id: StableId,
    pub hauler_id: Option<StableId>,
    pub route_id: Option<StableId>,
    pub location: PhysicalLocation,
    pub quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BarterContract {
    pub schema_version: u32,
    pub contract_id: StableId,
    pub offer: BarterOffer,
    pub consent: ContractConsent,
    pub stage: ContractStage,
    pub recovery: Option<RecoveryReason>,
    pub legs: Vec<PhysicalTransferLeg>,
}

impl BarterContract {
    pub fn propose(
        permit: &DispatchPermit,
        offer: BarterOffer,
        contract_id: StableId,
    ) -> Result<Self, BarterError> {
        if offer.source != permit.source || offer.destination != permit.destination {
            return Err(BarterError::PermitMismatch);
        }
        offer.validate()?;
        let contract = Self {
            schema_version: MONEYLESS_BARTER_SCHEMA_VERSION,
            contract_id,
            consent: ContractConsent::new(permit.source.clone(), permit.destination.clone()),
            stage: ContractStage::Proposed,
            recovery: None,
            legs: Vec::new(),
            offer,
        };
        contract.validate()?;
        Ok(contract)
    }

    pub fn validate(&self) -> Result<(), BarterError> {
        if self.schema_version != MONEYLESS_BARTER_SCHEMA_VERSION {
            return Err(BarterError::UnsupportedVersion);
        }
        self.contract_id.validate()?;
        self.offer.validate()?;
        self.consent.validate()?;
        let required = BTreeSet::from([self.offer.source.clone(), self.offer.destination.clone()]);
        if self.consent.required != required {
            return Err(BarterError::MalformedConsent);
        }
        if self.recovery.is_some()
            && !matches!(
                self.stage,
                ContractStage::Returning
                    | ContractStage::Stranded
                    | ContractStage::Recovered
                    | ContractStage::Cancelled
            )
        {
            return Err(BarterError::MalformedTransfer);
        }
        let offer_lots = self
            .offer
            .offered
            .iter()
            .chain(&self.offer.requested)
            .map(|lot| (&lot.lot_id, lot.quantity))
            .collect::<std::collections::BTreeMap<_, _>>();
        let mut seen_legs = BTreeSet::new();
        for leg in &self.legs {
            for id in [
                &leg.lot_id,
                &leg.source_endpoint_id,
                &leg.destination_endpoint_id,
                &leg.reservation_id,
                &leg.escrow_id,
            ] {
                id.validate()?;
            }
            if let Some(id) = &leg.hauler_id {
                id.validate()?;
            }
            if let Some(id) = &leg.route_id {
                id.validate()?;
            }
            let direction_is_valid = (leg.owner == self.offer.source
                && leg.recipient == self.offer.destination)
                || (leg.owner == self.offer.destination && leg.recipient == self.offer.source);
            if leg.quantity == 0
                || !direction_is_valid
                || offer_lots.get(&leg.lot_id).copied() != Some(leg.quantity)
                || !seen_legs.insert(leg.lot_id.clone())
            {
                return Err(BarterError::MalformedTransfer);
            }
        }
        Ok(())
    }
}

/// Commands are adapters for the existing physical trade/reservation
/// authorities.  Constructing or inspecting them never mutates a ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicalBarterCommand {
    Reserve {
        contract_id: StableId,
        lot_id: StableId,
        quantity: u64,
        source_endpoint_id: StableId,
        destination_endpoint_id: StableId,
    },
    Escrow {
        contract_id: StableId,
        lot_id: StableId,
        reservation_id: StableId,
        quantity: u64,
    },
    AssignHauler {
        contract_id: StableId,
        lot_id: StableId,
        hauler_id: StableId,
    },
    Deliver {
        contract_id: StableId,
        lot_id: StableId,
        route_id: StableId,
        destination_endpoint_id: StableId,
    },
    Return {
        contract_id: StableId,
        lot_id: StableId,
        route_id: Option<StableId>,
    },
    Strand {
        contract_id: StableId,
        lot_id: StableId,
        reason: RecoveryReason,
    },
    Salvage {
        contract_id: StableId,
        lot_id: StableId,
        stockpile_id: StableId,
    },
    Release {
        contract_id: StableId,
        lot_id: StableId,
    },
}

pub fn recovery_commands(
    contract_id: StableId,
    leg: &PhysicalTransferLeg,
    reason: RecoveryReason,
    safe_stockpile: Option<StableId>,
) -> Vec<PhysicalBarterCommand> {
    let mut commands = vec![PhysicalBarterCommand::Return {
        contract_id: contract_id.clone(),
        lot_id: leg.lot_id.clone(),
        route_id: leg.route_id.clone(),
    }];
    if let Some(stockpile_id) = safe_stockpile {
        commands.push(PhysicalBarterCommand::Salvage {
            contract_id,
            lot_id: leg.lot_id.clone(),
            stockpile_id,
        });
    } else {
        commands.push(PhysicalBarterCommand::Strand {
            contract_id,
            lot_id: leg.lot_id.clone(),
            reason,
        });
    }
    commands
}

/// Restart snapshots carry identity, quantity, and physical location only.
/// The adapter can compare these snapshots after persistence/restart without
/// handing mutation authority to this leaf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestartConservationSnapshot {
    pub contract_id: StableId,
    pub lots: Vec<RestartLotSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestartLotSnapshot {
    pub lot_id: StableId,
    pub quantity: u64,
    pub location: PhysicalLocation,
}

/// Exact lot identity and quantity must survive restart.  Location may move
/// only through an already-authorized physical lifecycle command.
pub fn validate_restart_conservation(
    before: &RestartConservationSnapshot,
    after: &RestartConservationSnapshot,
) -> Result<(), BarterError> {
    if before.contract_id != after.contract_id {
        return Err(BarterError::RestartContractMismatch);
    }
    before.contract_id.validate()?;
    let mut before_lots = before.lots.clone();
    let mut after_lots = after.lots.clone();
    for lot in before_lots.iter().chain(&after_lots) {
        lot.lot_id.validate()?;
    }
    before_lots.sort_by(|a, b| a.lot_id.cmp(&b.lot_id));
    after_lots.sort_by(|a, b| a.lot_id.cmp(&b.lot_id));
    if before_lots.len() != after_lots.len()
        || before_lots
            .iter()
            .zip(&after_lots)
            .any(|(old, new)| old.lot_id != new.lot_id || old.quantity != new.quantity)
    {
        return Err(BarterError::RestartLotMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReportMetric {
    pub estimate: u64,
    pub lower_bound: u64,
    pub upper_bound: u64,
    pub confidence_bps: u32,
    pub observed_tick: u64,
    pub age_ticks: u64,
}

impl ReportMetric {
    pub fn new(estimate: u64) -> Result<Self, BarterError> {
        if estimate > REPORT_SCALE {
            return Err(BarterError::MetricOutOfRange);
        }
        Ok(Self {
            estimate,
            lower_bound: estimate,
            upper_bound: estimate,
            confidence_bps: REPORT_SCALE as u32,
            observed_tick: 0,
            age_ticks: 0,
        })
    }

    fn validate(&self) -> Result<(), BarterError> {
        if self.lower_bound > self.estimate
            || self.estimate > self.upper_bound
            || self.upper_bound > REPORT_SCALE
            || u64::from(self.confidence_bps) > REPORT_SCALE
        {
            return Err(BarterError::MetricOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeScoreInputs {
    pub source_need: ReportMetric,
    pub destination_offerings: ReportMetric,
    pub quality: ReportMetric,
    pub utility: ReportMetric,
    pub exchange_value: ReportMetric,
    pub distance_premium: ReportMetric,
    pub travel_time: ReportMetric,
    pub route_risk: ReportMetric,
    pub carrying_cost: ReportMetric,
    pub carrying_capacity: ReportMetric,
    pub opportunity_cost: ReportMetric,
}

impl TradeScoreInputs {
    fn validate(&self) -> Result<(), BarterError> {
        self.source_need.validate()?;
        self.destination_offerings.validate()?;
        self.quality.validate()?;
        self.utility.validate()?;
        self.exchange_value.validate()?;
        self.distance_premium.validate()?;
        self.travel_time.validate()?;
        self.route_risk.validate()?;
        self.carrying_cost.validate()?;
        self.carrying_capacity.validate()?;
        self.opportunity_cost.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeScore {
    pub benefit: i64,
    pub cost: i64,
    pub net: i64,
}

fn metric_product(a: u64, b: u64) -> i64 {
    ((u128::from(a) * u128::from(b)) / u128::from(REPORT_SCALE)) as i64
}

pub fn score_trade(inputs: TradeScoreInputs) -> Result<TradeScore, BarterError> {
    inputs.validate()?;
    let benefit = metric_product(
        inputs.source_need.estimate,
        inputs.destination_offerings.estimate,
    )
    .saturating_add(i64::from(inputs.quality.estimate as u32))
    .saturating_add(i64::from(inputs.utility.estimate as u32))
    .saturating_add(i64::from(inputs.exchange_value.estimate as u32))
    .saturating_add(i64::from(inputs.carrying_capacity.estimate as u32));
    let cost = i64::from(inputs.distance_premium.estimate as u32)
        .saturating_add(i64::from(inputs.travel_time.estimate as u32))
        .saturating_add(i64::from(inputs.route_risk.estimate as u32))
        .saturating_add(i64::from(inputs.carrying_cost.estimate as u32))
        .saturating_add(i64::from(inputs.opportunity_cost.estimate as u32));
    Ok(TradeScore {
        benefit,
        cost,
        net: benefit.saturating_sub(cost),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradePosture {
    PossibleNow,
    BetterTrade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradePostureDecision {
    pub posture: Option<TradePosture>,
    pub score: TradeScore,
}

pub fn choose_posture(inputs: TradeScoreInputs) -> Result<TradePostureDecision, BarterError> {
    let score = score_trade(inputs)?;
    // Close/fast/safe is the now posture.  Better-trade tolerates those costs
    // when physical utility/value is strong, but still requires non-negative
    // fixed-point net benefit.  These thresholds are report-space constants.
    let possible_now = score.net >= 0
        && inputs.distance_premium.estimate <= 350_000
        && inputs.travel_time.estimate <= 300_000
        && inputs.route_risk.estimate <= 200_000;
    let better_trade = score.net >= 0
        && (inputs.exchange_value.estimate >= 600_000
            || inputs.quality.estimate >= 700_000
            || inputs.utility.estimate >= 700_000
            || inputs.destination_offerings.estimate >= inputs.source_need.estimate);
    Ok(TradePostureDecision {
        posture: if possible_now {
            Some(TradePosture::PossibleNow)
        } else if better_trade {
            Some(TradePosture::BetterTrade)
        } else {
            None
        },
        score,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedTradeCandidate {
    pub candidate_id: StableId,
    pub destination: ColonyId,
    pub posture: TradePosture,
    pub score: TradeScore,
}

pub fn rank_trade_candidates(
    mut candidates: Vec<RankedTradeCandidate>,
) -> Vec<RankedTradeCandidate> {
    candidates.sort_by(|a, b| {
        b.score
            .net
            .cmp(&a.score.net)
            .then_with(|| posture_rank(a.posture).cmp(&posture_rank(b.posture)))
            .then_with(|| a.destination.cmp(&b.destination))
            .then_with(|| a.candidate_id.cmp(&b.candidate_id))
    });
    candidates
}

const fn posture_rank(posture: TradePosture) -> u8 {
    match posture {
        TradePosture::PossibleNow => 0,
        TradePosture::BetterTrade => 1,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalValueInput {
    pub base_units: u64,
    pub quantity: u64,
    pub quality_bps: u32,
    pub utility_bps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CanonicalComparisonValue(u64);

impl CanonicalComparisonValue {
    #[must_use]
    pub const fn units(self) -> u64 {
        self.0
    }
}

/// Canonical value is comparison/scoring data only; it is never carried by an
/// offer, reserved as a lot, or settled as a balance.
pub fn canonical_comparison_value(
    input: CanonicalValueInput,
) -> Result<CanonicalComparisonValue, BarterError> {
    if input.quantity == 0
        || input.base_units == 0
        || input.quality_bps > REPORT_SCALE as u32
        || input.utility_bps > REPORT_SCALE as u32
    {
        return Err(BarterError::InvalidComparisonValue);
    }
    let adjusted = u128::from(input.base_units)
        .checked_mul(u128::from(input.quantity))
        .and_then(|v| v.checked_mul(u128::from(input.quality_bps)))
        .and_then(|v| v.checked_mul(u128::from(input.utility_bps)))
        .ok_or(BarterError::ArithmeticOverflow)?
        / u128::from(REPORT_SCALE)
        / u128::from(REPORT_SCALE);
    Ok(CanonicalComparisonValue(
        u64::try_from(adjusted).map_err(|_| BarterError::ArithmeticOverflow)?,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarterError {
    UnsupportedVersion,
    MalformedStableId,
    MalformedColony,
    SameColony,
    GlobalVillageLockedNeutral,
    DuplicateStance,
    NonCanonicalOrder,
    MalformedPhysicalLot,
    DuplicateLot,
    MalformedOffer,
    MalformedConsent,
    PermitMismatch,
    MalformedTransfer,
    MetricOutOfRange,
    InvalidComparisonValue,
    ArithmeticOverflow,
    RestartContractMismatch,
    RestartLotMismatch,
}

fn is_content_id(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && value.len() <= 64
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
