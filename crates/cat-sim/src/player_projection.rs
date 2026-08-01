//! Report-safe god/player projection boundary specified by
//! `docs/leader-ai-overhaul/planner-and-beliefs.md` and
//! `docs/leader-ai-overhaul/wire-persistence-ui.md`.

use serde::Serialize;

use crate::{
    beliefs::{BeliefProjection, BeliefStore, ExecutionFeedback},
    planner_core::PlannerId,
};

pub const PLAYER_PROJECTION_SCHEMA_VERSION: u32 = 1;
pub const MAX_PLAYER_FEEDBACK: usize = 32;

/// Every consumer that must use the report-safe player projection rather than
/// executor truth. Keeping this inventory in the simulation layer makes a new
/// UI or API surface an explicit leak-audit decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlayerSurface {
    Snapshot,
    Tooltip,
    Inspector,
    ValidationMessage,
    PlanExplanation,
    ResearchScreen,
    TraderHint,
    DebugOutput,
    ClientCache,
}

impl PlayerSurface {
    pub const ALL: [Self; 9] = [
        Self::Snapshot,
        Self::Tooltip,
        Self::Inspector,
        Self::ValidationMessage,
        Self::PlanExplanation,
        Self::ResearchScreen,
        Self::TraderHint,
        Self::DebugOutput,
        Self::ClientCache,
    ];

    /// Authoritative executor fields are forbidden on every player surface.
    /// Allowed estimates enter through `BeliefProjection`; exact Favor enters
    /// through the dedicated divine-ledger field on `PlayerProjection`.
    #[must_use]
    pub const fn rejects(self, field: ForbiddenExecutorField) -> bool {
        let _ = (self, field);
        true
    }
}

/// Private executor facts that must never be accepted by a public projection
/// constructor or shipped to a client and hidden only at render time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ForbiddenExecutorField {
    AuthoritativeStock,
    AuthoritativeProduction,
    AuthoritativeConsumption,
    SourceCapacity,
    Depletion,
    AuthoritativeRegeneration,
    UndiscoveredSite,
    UnseenThreat,
    OtherColonyBeliefs,
    OtherColonyInventory,
    OtherColonyPlans,
}

impl ForbiddenExecutorField {
    pub const ALL: [Self; 11] = [
        Self::AuthoritativeStock,
        Self::AuthoritativeProduction,
        Self::AuthoritativeConsumption,
        Self::SourceCapacity,
        Self::Depletion,
        Self::AuthoritativeRegeneration,
        Self::UndiscoveredSite,
        Self::UnseenThreat,
        Self::OtherColonyBeliefs,
        Self::OtherColonyInventory,
        Self::OtherColonyPlans,
    ];
}

/// The sole simulation-level payload from which protocol snapshots, UI hints,
/// bounded errors, and debug summaries may be built.
///
/// It intentionally has no executor-state argument and no catch-all metadata
/// map. Exact Favor is permitted because it is a divine ledger balance, not a
/// physical inventory measurement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProjection {
    pub schema_version: u32,
    pub colony_id: PlannerId,
    pub belief_version: u64,
    pub favor_balance: u64,
    pub beliefs: Vec<BeliefProjection>,
    pub feedback: Vec<ExecutionFeedback>,
}

impl PlayerProjection {
    #[must_use]
    pub fn from_beliefs(
        colony_id: PlannerId,
        belief_store: &BeliefStore,
        favor_balance: u64,
        now_tick: u64,
        feedback: impl IntoIterator<Item = ExecutionFeedback>,
    ) -> Self {
        let beliefs = belief_store
            .iter()
            .filter_map(|(_, record)| belief_store.project(&record.key, now_tick))
            .collect();
        let mut feedback: Vec<_> = feedback.into_iter().collect();
        feedback.sort_unstable_by_key(|entry| feedback_rank(*entry));
        feedback.dedup();
        feedback.truncate(MAX_PLAYER_FEEDBACK);

        Self {
            schema_version: PLAYER_PROJECTION_SCHEMA_VERSION,
            colony_id,
            belief_version: belief_store.version,
            favor_balance,
            beliefs,
            feedback,
        }
    }
}

const fn feedback_rank(feedback: ExecutionFeedback) -> u8 {
    match feedback {
        ExecutionFeedback::SourceUnavailable => 0,
        ExecutionFeedback::RouteBlocked => 1,
        ExecutionFeedback::DestinationFull => 2,
        ExecutionFeedback::NoWillingWorker => 3,
        ExecutionFeedback::ReservationConflict => 4,
        ExecutionFeedback::DependencyBlocked => 5,
        ExecutionFeedback::SiteInvalidated => 6,
    }
}
