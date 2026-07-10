//! Colony stock ledger (P12.4a) — a Dwarf-Fortress-bookkeeper-style *reported* view of
//! the colony's stock.
//!
//! True [`crate::entities::Resources`] on the colony is always exact and is never touched
//! here. The ledger is a lagging **report**: a staffed Accounting Tent recounts it to the
//! exact current stock every tick (accurate + fast, like a DF bookkeeper), while without a
//! staffed tent it only recounts once every [`UNSTAFFED_RECOUNT_INTERVAL_MS`], so the
//! reported numbers lag reality between recounts. Refreshing the ledger never mutates the
//! true resources, so the economy of record stays byte-identical.

use serde::{Deserialize, Serialize};

use crate::entities::Resources;

/// How long (ms of game time) an *unstaffed* ledger may go before it fully recounts.
/// At the default 1s worker tick this is ~30 game-ticks.
pub const UNSTAFFED_RECOUNT_INTERVAL_MS: i64 = 30_000;

/// The colony's reported stock ledger: the last counted [`Resources`] totals and the tick
/// that count was taken. Distinct from the true `colony.resources`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockLedger {
    /// The stock totals as last *reported* (may lag the true resources when unstaffed).
    pub reported: Resources,
    /// Game-tick timestamp (ms) of the last recount.
    pub last_counted: i64,
}

impl StockLedger {
    /// A ledger freshly counted against `resources` at `now_ms`.
    #[must_use]
    pub fn counted(resources: &Resources, now_ms: i64) -> Self {
        Self {
            reported: resources.clone(),
            last_counted: now_ms,
        }
    }

    /// Whether the reported totals currently match the true resources bit-for-bit.
    #[must_use]
    pub fn is_accurate(&self, resources: &Resources) -> bool {
        &self.reported == resources
    }
}

/// Refresh the reported ledger. With a `staffed` Accounting Tent it recounts to the exact
/// current `resources` every tick; otherwise it only recounts once the recount interval has
/// elapsed since `last_counted`. Returns `true` when a recount happened this tick. Never
/// mutates `resources`.
pub fn refresh_ledger(
    ledger: &mut StockLedger,
    resources: &Resources,
    staffed: bool,
    now_ms: i64,
) -> bool {
    let interval_elapsed =
        now_ms.saturating_sub(ledger.last_counted) >= UNSTAFFED_RECOUNT_INTERVAL_MS;
    if staffed || interval_elapsed {
        ledger.reported = resources.clone();
        ledger.last_counted = now_ms;
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{StockLedger, UNSTAFFED_RECOUNT_INTERVAL_MS, refresh_ledger};
    use crate::entities::Resources;

    fn stock(food: f64) -> Resources {
        Resources {
            food,
            ..Resources::default()
        }
    }

    #[test]
    fn staffed_tent_recounts_to_exact_resources_the_same_tick() {
        let mut ledger = StockLedger::counted(&stock(100.0), 1_000);
        let truth = stock(175.0);

        let recounted = refresh_ledger(&mut ledger, &truth, true, 1_500);

        assert!(recounted);
        assert_eq!(ledger.reported, truth);
        assert_eq!(ledger.last_counted, 1_500);
        assert!(ledger.is_accurate(&truth));
    }

    #[test]
    fn unstaffed_ledger_lags_within_the_recount_interval() {
        let mut ledger = StockLedger::counted(&stock(100.0), 1_000);
        let truth = stock(175.0);

        // Only a few ticks later: no recount, reported still lags.
        let recounted = refresh_ledger(&mut ledger, &truth, false, 1_000 + 5_000);

        assert!(!recounted);
        assert_eq!(ledger.reported, stock(100.0), "reported still stale");
        assert_eq!(ledger.last_counted, 1_000);
        assert!(!ledger.is_accurate(&truth));
    }

    #[test]
    fn unstaffed_ledger_recounts_after_the_interval_elapses() {
        let mut ledger = StockLedger::counted(&stock(100.0), 1_000);
        let truth = stock(175.0);

        let recounted = refresh_ledger(
            &mut ledger,
            &truth,
            false,
            1_000 + UNSTAFFED_RECOUNT_INTERVAL_MS,
        );

        assert!(recounted);
        assert_eq!(ledger.reported, truth);
        assert_eq!(ledger.last_counted, 1_000 + UNSTAFFED_RECOUNT_INTERVAL_MS);
    }

    #[test]
    fn refresh_never_mutates_the_true_resources() {
        let mut ledger = StockLedger::default();
        let truth = stock(42.0);
        let before = truth.clone();

        refresh_ledger(&mut ledger, &truth, true, 10_000);

        assert_eq!(truth, before, "true resources untouched");
    }
}
