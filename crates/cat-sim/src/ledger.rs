//! Physical colony stock accounting (P12.4a).
//!
//! [`crate::entities::Resources`] remains the authoritative economy.  This module stores
//! only what the village has *reported*: an unattended colony receives an occasional slow
//! background recount, while a staffed Accounting Tent updates one visible pile only after
//! its bookkeeper has walked there and completed a count.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{entities::Resources, stockpiles::Stockpile};

/// How long (ms of game time) an *unstaffed* ledger may go before its background recount.
pub const UNSTAFFED_RECOUNT_INTERVAL_MS: i64 = 30_000;

/// Persisted work required while standing at one pile.
pub const PILE_COUNT_DWELL_MS: i64 = 5_000;

/// The last report made for one player-visible physical pile.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PileReport {
    pub reported: Resources,
    pub last_counted: i64,
}

/// Durable phase of a physical bookkeeper round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountingPhase {
    #[default]
    TravelingToTent,
    TravelingToPile,
    Counting,
    ReturningToTent,
    WaitingAtTent,
}

/// One persisted tent → piles → tent round.  The ordered queue is planned from one
/// reachability component, so a restart never changes targets or pays per-pile A* again.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountingRound {
    pub worker_id: String,
    pub tent_id: String,
    pub phase: AccountingPhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_stockpile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_stockpile_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unreachable_stockpile_ids: Vec<String>,
    #[serde(default)]
    pub dwell_elapsed_ms: i64,
    #[serde(default)]
    pub topology_signature: u64,
}

/// The colony's reported totals, independently fresh pile reports, and any physical round.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StockLedger {
    pub reported: Resources,
    pub last_counted: i64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pile_reports: BTreeMap<String, PileReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_round: Option<AccountingRound>,
}

impl StockLedger {
    /// A legacy-compatible aggregate count. New colonies should call
    /// [`Self::counted_with_piles`] after their physical stores are reconciled.
    #[must_use]
    pub fn counted(resources: &Resources, now_ms: i64) -> Self {
        Self {
            reported: resources.clone(),
            last_counted: now_ms,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn counted_with_piles(resources: &Resources, piles: &[Stockpile], now_ms: i64) -> Self {
        let mut ledger = Self::counted(resources, now_ms);
        ledger.replace_pile_reports(piles, now_ms);
        ledger
    }

    #[must_use]
    pub fn is_accurate(&self, resources: &Resources) -> bool {
        &self.reported == resources
    }

    #[must_use]
    pub fn visible_piles_are_accurate(&self, piles: &[Stockpile]) -> bool {
        let visible = visible_piles(piles).collect::<Vec<_>>();
        self.pile_reports.len() == visible.len()
            && visible.iter().all(|pile| {
                self.pile_reports
                    .get(&pile.id)
                    .is_some_and(|report| report.reported == pile.contents)
            })
    }

    /// One-time migration for JSON written before per-pile reports existed. The aggregate's
    /// old timestamp is retained: this makes the newly exposed breakdown no fresher than the
    /// already persisted count.
    pub fn migrate_pile_reports(&mut self, piles: &[Stockpile]) {
        if self.pile_reports.is_empty() {
            // Legacy ledgers knew only one aggregate. Attribute that old report to the seeded
            // general store (the historical reservoir) and leave newer designated piles
            // uncounted. This preserves the aggregate exactly and lets later physical visits
            // apply honest deltas instead of sampling current contents during migration.
            self.pile_reports = visible_piles(piles)
                .map(|pile| {
                    let reported = if pile.is_general_storehouse() {
                        self.reported.clone()
                    } else {
                        Resources::default()
                    };
                    (
                        pile.id.clone(),
                        PileReport {
                            reported,
                            last_counted: self.last_counted,
                        },
                    )
                })
                .collect();
        }
        self.retain_visible_piles(piles);
    }

    pub fn replace_pile_reports(&mut self, piles: &[Stockpile], now_ms: i64) {
        self.pile_reports = visible_piles(piles)
            .map(|pile| {
                (
                    pile.id.clone(),
                    PileReport {
                        reported: pile.contents.clone(),
                        last_counted: now_ms,
                    },
                )
            })
            .collect();
    }

    pub fn retain_visible_piles(&mut self, piles: &[Stockpile]) {
        self.pile_reports.retain(|id, _| {
            piles
                .iter()
                .any(|pile| pile.id == *id && !pile.is_station_local())
        });
    }

    /// Apply a completed physical count. Only this pile's report changes. The aggregate is
    /// adjusted by the same delta, leaving every unvisited pile and station-local residue at
    /// its prior reported value.
    pub fn count_pile(&mut self, pile: &Stockpile, now_ms: i64) {
        if pile.is_station_local() {
            return;
        }
        let old = self
            .pile_reports
            .get(&pile.id)
            .map_or_else(Resources::default, |report| report.reported.clone());
        for kind in crate::stockpiles::ResourceKind::ALL {
            let delta = crate::stockpiles::resource_amount(&pile.contents, *kind)
                - crate::stockpiles::resource_amount(&old, *kind);
            crate::stockpiles::add_resource(&mut self.reported, *kind, delta);
        }
        self.pile_reports.insert(
            pile.id.clone(),
            PileReport {
                reported: pile.contents.clone(),
                last_counted: now_ms,
            },
        );
        self.last_counted = now_ms;
    }
}

fn visible_piles(piles: &[Stockpile]) -> impl Iterator<Item = &Stockpile> {
    piles.iter().filter(|pile| !pile.is_station_local())
}

/// Preserve the explicitly designed slow no-input fallback. A staffed tent suppresses the
/// background recount: its only fast updates come from physical pile visits.
pub fn refresh_ledger(
    ledger: &mut StockLedger,
    resources: &Resources,
    piles: &[Stockpile],
    staffed: bool,
    now_ms: i64,
) -> bool {
    ledger.migrate_pile_reports(piles);
    let interval_elapsed =
        now_ms.saturating_sub(ledger.last_counted) >= UNSTAFFED_RECOUNT_INTERVAL_MS;
    if !staffed && interval_elapsed {
        ledger.reported = resources.clone();
        ledger.last_counted = now_ms;
        ledger.replace_pile_reports(piles, now_ms);
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{StockLedger, UNSTAFFED_RECOUNT_INTERVAL_MS, refresh_ledger};
    use crate::{
        entities::Resources,
        stockpiles::{GENERAL_STOREHOUSE_ID, ResourceKind, Stockpile},
        zones::ZoneRect,
    };
    use std::collections::BTreeSet;

    fn stock(food: f64) -> Resources {
        Resources {
            food,
            ..Resources::default()
        }
    }

    fn pile(id: &str, food: f64) -> Stockpile {
        Stockpile {
            id: id.to_owned(),
            rect: ZoneRect {
                x1: 0,
                y1: 0,
                x2: 0,
                y2: 0,
            },
            accepts: BTreeSet::from([ResourceKind::Food]),
            contents: stock(food),
        }
    }

    #[test]
    fn staffed_tent_does_not_instantly_recount() {
        let piles = [pile(GENERAL_STOREHOUSE_ID, 175.0)];
        let mut ledger = StockLedger::counted_with_piles(&stock(100.0), &piles, 1_000);
        let truth = stock(175.0);

        assert!(!refresh_ledger(&mut ledger, &truth, &piles, true, 40_000));
        assert_eq!(ledger.reported, stock(100.0));
    }

    #[test]
    fn counting_one_pile_updates_only_its_report_and_never_truth() {
        let original = [pile("a", 10.0), pile("b", 20.0)];
        let mut ledger = StockLedger::counted_with_piles(&stock(30.0), &original, 1_000);
        let changed_a = pile("a", 15.0);
        let truth = stock(35.0);

        ledger.count_pile(&changed_a, 2_000);

        assert_eq!(ledger.pile_reports["a"].reported.food, 15.0);
        assert_eq!(ledger.pile_reports["b"].reported.food, 20.0);
        assert_eq!(ledger.reported.food, 35.0);
        assert_eq!(truth, stock(35.0));
    }

    #[test]
    fn offsetting_pile_changes_do_not_fabricate_freshness() {
        let original = [pile("a", 10.0), pile("b", 20.0)];
        let ledger = StockLedger::counted_with_piles(&stock(30.0), &original, 1_000);
        let changed = [pile("a", 15.0), pile("b", 15.0)];

        assert!(
            ledger.is_accurate(&stock(30.0)),
            "aggregate happens to match"
        );
        assert!(!ledger.visible_piles_are_accurate(&changed));
    }

    #[test]
    fn unstaffed_ledger_lags_then_background_recounts_every_pile() {
        let piles = [pile("a", 175.0)];
        let mut ledger = StockLedger::counted(&stock(100.0), 1_000);
        assert!(!refresh_ledger(
            &mut ledger,
            &stock(175.0),
            &piles,
            false,
            6_000
        ));
        assert!(refresh_ledger(
            &mut ledger,
            &stock(175.0),
            &piles,
            false,
            1_000 + UNSTAFFED_RECOUNT_INTERVAL_MS
        ));
        assert_eq!(ledger.reported.food, 175.0);
        assert_eq!(ledger.pile_reports["a"].reported.food, 175.0);
    }

    #[test]
    fn old_json_defaults_new_physical_state() {
        let ledger: StockLedger = serde_json::from_str(
            r#"{"reported":{"food":3.0,"water":0.0,"herbs":0.0,"materials":0.0,"refined":0.0,"weapons":0.0,"armor":0.0,"blessings":0.0},"lastCounted":7}"#,
        )
        .expect("legacy ledger");
        assert!(ledger.pile_reports.is_empty());
        assert!(ledger.active_round.is_none());
    }
}
