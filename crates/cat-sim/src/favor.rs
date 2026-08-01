//! Exact Favor currency ledger specified by
//! `docs/leader-ai-overhaul/shrine-favor-research.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

use crate::planner_core::PlannerId;

pub const FAVOR_SCHEMA_VERSION: u32 = 1;
pub const MICRO_FAVOR_PER_FAVOR: u64 = 1_000_000;

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Favor(u64);

impl Favor {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(MICRO_FAVOR_PER_FAVOR);

    #[must_use]
    pub const fn from_micro_favor(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn from_whole(value: u64) -> Option<Self> {
        match value.checked_mul(MICRO_FAVOR_PER_FAVOR) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }

    #[must_use]
    pub const fn micro_favor(self) -> u64 {
        self.0
    }

    fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FavorEventId(PlannerId);

impl FavorEventId {
    #[must_use]
    pub fn derive(namespace: &str, colony_id: &str, source_id: &str) -> Self {
        Self(PlannerId::derive(
            "favor_event",
            [namespace, colony_id, source_id],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FavorEventKind {
    OfferingCredit,
    ResearchPurchase,
    DivineBoostPurchase,
    LegacyMigrationCredit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FavorDirection {
    Credit,
    Debit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FavorEvent {
    pub id: FavorEventId,
    pub kind: FavorEventKind,
    pub direction: FavorDirection,
    pub amount: Favor,
    pub balance_after: Favor,
    pub committed_version: u64,
    pub committed_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FavorCommitOutcome {
    Committed,
    AlreadyCommitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavorLedger {
    pub schema_version: u32,
    pub version: u64,
    pub balance: Favor,
    events: BTreeMap<FavorEventId, FavorEvent>,
}

impl FavorLedger {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            schema_version: FAVOR_SCHEMA_VERSION,
            version: 0,
            balance: Favor::ZERO,
            events: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn event(&self, id: &FavorEventId) -> Option<&FavorEvent> {
        self.events.get(id)
    }

    #[must_use]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Iterate committed public ledger entries in stable event-ID order.
    pub fn events(&self) -> impl ExactSizeIterator<Item = &FavorEvent> {
        self.events.values()
    }

    pub fn credit(
        &mut self,
        id: FavorEventId,
        kind: FavorEventKind,
        amount: Favor,
        expected_version: u64,
        now_tick: u64,
    ) -> Result<FavorCommitOutcome, FavorError> {
        self.commit(
            id,
            kind,
            FavorDirection::Credit,
            amount,
            expected_version,
            now_tick,
        )
    }

    pub fn debit(
        &mut self,
        id: FavorEventId,
        kind: FavorEventKind,
        amount: Favor,
        expected_version: u64,
        now_tick: u64,
    ) -> Result<FavorCommitOutcome, FavorError> {
        self.commit(
            id,
            kind,
            FavorDirection::Debit,
            amount,
            expected_version,
            now_tick,
        )
    }

    fn commit(
        &mut self,
        id: FavorEventId,
        kind: FavorEventKind,
        direction: FavorDirection,
        amount: Favor,
        expected_version: u64,
        now_tick: u64,
    ) -> Result<FavorCommitOutcome, FavorError> {
        if amount == Favor::ZERO {
            return Err(FavorError::ZeroAmount);
        }
        if let Some(existing) = self.events.get(&id) {
            return if existing.kind == kind
                && existing.direction == direction
                && existing.amount == amount
            {
                Ok(FavorCommitOutcome::AlreadyCommitted)
            } else {
                Err(FavorError::EventIdConflict)
            };
        }
        if expected_version != self.version {
            return Err(FavorError::StaleVersion);
        }
        let balance_after = match direction {
            FavorDirection::Credit => self
                .balance
                .checked_add(amount)
                .ok_or(FavorError::Overflow)?,
            FavorDirection::Debit => self
                .balance
                .checked_sub(amount)
                .ok_or(FavorError::InsufficientFavor)?,
        };
        let committed_version = self.version.checked_add(1).ok_or(FavorError::Overflow)?;
        let event = FavorEvent {
            id: id.clone(),
            kind,
            direction,
            amount,
            balance_after,
            committed_version,
            committed_tick: now_tick,
        };
        self.events.insert(id, event);
        self.balance = balance_after;
        self.version = committed_version;
        Ok(FavorCommitOutcome::Committed)
    }

    fn validate(&self) -> Result<(), FavorError> {
        if self.schema_version != FAVOR_SCHEMA_VERSION {
            return Err(FavorError::MalformedPersistence);
        }
        let mut balance = Favor::ZERO;
        let mut version = 0;
        let mut events = self.events.values().collect::<Vec<_>>();
        events.sort_by_key(|event| event.committed_version);
        for event in events {
            if event.amount == Favor::ZERO || event.committed_version != version + 1 {
                return Err(FavorError::MalformedPersistence);
            }
            balance = match event.direction {
                FavorDirection::Credit => balance.checked_add(event.amount),
                FavorDirection::Debit => balance.checked_sub(event.amount),
            }
            .ok_or(FavorError::MalformedPersistence)?;
            if event.balance_after != balance {
                return Err(FavorError::MalformedPersistence);
            }
            version = event.committed_version;
        }
        if version != self.version || balance != self.balance {
            return Err(FavorError::MalformedPersistence);
        }
        Ok(())
    }
}

impl Default for FavorLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedFavorLedger {
    schema_version: u32,
    #[serde(default)]
    version: u64,
    #[serde(default)]
    balance: Favor,
    #[serde(default)]
    events: BTreeMap<FavorEventId, FavorEvent>,
}

impl<'de> Deserialize<'de> for FavorLedger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UncheckedFavorLedger::deserialize(deserializer)?;
        let ledger = Self {
            schema_version: raw.schema_version,
            version: raw.version,
            balance: raw.balance,
            events: raw.events,
        };
        ledger.validate().map_err(serde::de::Error::custom)?;
        Ok(ledger)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FavorError {
    ZeroAmount,
    StaleVersion,
    InsufficientFavor,
    EventIdConflict,
    Overflow,
    MalformedPersistence,
}

impl std::fmt::Display for FavorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Favor ledger error: {self:?}")
    }
}

impl std::error::Error for FavorError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(name: &str) -> FavorEventId {
        FavorEventId::derive("test", "colony", name)
    }

    #[test]
    fn credit_debit_cas_and_idempotency_are_exact() {
        let mut ledger = FavorLedger::new();
        let credit = event("offering");
        assert_eq!(
            ledger.credit(
                credit.clone(),
                FavorEventKind::OfferingCredit,
                Favor::ONE,
                0,
                10,
            ),
            Ok(FavorCommitOutcome::Committed)
        );
        assert_eq!(ledger.balance, Favor::ONE);
        assert_eq!(
            ledger.credit(credit, FavorEventKind::OfferingCredit, Favor::ONE, 0, 99,),
            Ok(FavorCommitOutcome::AlreadyCommitted)
        );
        assert_eq!(ledger.version, 1);
        assert_eq!(
            ledger.debit(
                event("research"),
                FavorEventKind::ResearchPurchase,
                Favor::from_micro_favor(500_000),
                1,
                20,
            ),
            Ok(FavorCommitOutcome::Committed)
        );
        assert_eq!(ledger.balance, Favor::from_micro_favor(500_000));
    }

    #[test]
    fn stale_unaffordable_conflicting_and_zero_mutations_fail_without_change() {
        let mut ledger = FavorLedger::new();
        let before = ledger.clone();
        assert_eq!(
            ledger.debit(
                event("debit"),
                FavorEventKind::ResearchPurchase,
                Favor::ONE,
                0,
                1,
            ),
            Err(FavorError::InsufficientFavor)
        );
        assert_eq!(ledger, before);
        assert_eq!(
            ledger.credit(
                event("zero"),
                FavorEventKind::OfferingCredit,
                Favor::ZERO,
                0,
                1,
            ),
            Err(FavorError::ZeroAmount)
        );
        ledger
            .credit(
                event("one"),
                FavorEventKind::OfferingCredit,
                Favor::ONE,
                0,
                1,
            )
            .unwrap();
        assert_eq!(
            ledger.credit(
                event("stale"),
                FavorEventKind::OfferingCredit,
                Favor::ONE,
                0,
                2,
            ),
            Err(FavorError::StaleVersion)
        );
        assert_eq!(ledger.version, 1);
    }

    #[test]
    fn restart_validates_event_chain_balance_and_version() {
        let mut ledger = FavorLedger::new();
        ledger
            .credit(
                event("offering"),
                FavorEventKind::OfferingCredit,
                Favor::from_whole(2).unwrap(),
                0,
                1,
            )
            .unwrap();
        let value = serde_json::to_value(&ledger).unwrap();
        assert_eq!(
            serde_json::from_value::<FavorLedger>(value.clone()).unwrap(),
            ledger
        );
        let mut corrupt = value;
        corrupt["balance"] = serde_json::json!(9);
        assert!(serde_json::from_value::<FavorLedger>(corrupt).is_err());
    }
}
