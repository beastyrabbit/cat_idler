//! Visiting trader / caravan economy (P19 slice 3) — a periodic friendly NPC plus a
//! simple coin economy. The merchant owns a persisted physical exterior, obstacle-aware
//! route to a shrine contact, bounded stay, finite wagon manifest/purse, and a physical
//! route back out — no combat or threat interaction.
//!
//! Per `docs/migration/specs/p19-items-materials-trade.md`'s "Traders / caravans"
//! section: a trader arrives at the village (walks to the gate), stays a while, and
//! opens a trade menu — sell surplus/crafted [`crate::items::Item`] goods for coin, buy
//! [`crate::stockpiles::ResourceKind`] resources with coin. "A simple value/coin
//! economy; relations/price could deepen later" — this module is exactly that: flat,
//! documented, RNG-free price functions.
//!
//! **No RNG stream consumption.** Visit manifests are stable hashes of
//! `(world_seed, colony_id, visit_number)`, route ordering is deterministic, and pricing
//! is pure percent math. Same state and tick sequence therefore produce the same visit.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    items::ItemStore,
    items::{Item, item_value, item_weight_grams},
    stockpiles::ResourceKind,
};

/// Game-hours from the end of one trader visit (or colony founding, for the first
/// visit) until the next one's arrival is triggered. ~2 game-days: deliberately rarer
/// than the 8h raid grace window (`crate::threat::RAID_GRACE_SEC`), so a caravan reads
/// as a calm, occasional counterpart to raids rather than a constant presence.
pub const TRADER_VISIT_INTERVAL_GAME_HOURS: f64 = 48.0;

/// Game-hours a trader lingers in [`TraderState::Trading`] at the shrine before it starts
/// departing — a real window for the player to notice it and trade.
pub const TRADER_LINGER_GAME_HOURS: f64 = 6.0;

/// Tiles/sec the trader's wagon covers while arriving/departing. A touch faster than a
/// raiding warband's march (`RAIDER_SPEED_TILES_PER_SEC` = 0.4 in `world_tick.rs`) since
/// it's a lone wagon, not a formation.
pub const TRADER_SPEED_TILES_PER_SEC: f64 = 0.5;

/// Percent of an item's [`item_value`] the trader pays when it *buys* that item from the
/// colony (its resale margin — it sells on at full value elsewhere). Flat and documented
/// per the spec's "simple value/coin economy" guidance; no RNG, no per-trader haggling.
pub const TRADER_BUY_PRICE_PCT: u32 = 60;

/// Percent of a resource's [`resource_unit_price`] the trader charges when it *sells*
/// that resource to the colony (its markup).
pub const TRADER_SELL_PRICE_PCT: u32 = 150;

/// Maximum finished-goods weight accepted in one signed sale. Items do not yet
/// have a physical wagon-haul job, so the existing caravan transaction is the honest
/// load seam: a player may make multiple bounded loads while the trader is present.
pub const TRADER_ITEM_LOAD_LIMIT_GRAMS: u32 = 20_000;

/// Total physical wagon capacity. Resource units weigh one kilogram; crafted items use
/// their exact maintained item weight. The starting manifest deliberately leaves room
/// for several player sales instead of pretending the merchant has an infinite hold.
pub const TRADER_CARGO_CAPACITY_GRAMS: f64 = 100_000.0;
pub const TRADER_RESOURCE_UNIT_WEIGHT_GRAMS: f64 = 1_000.0;
/// Finite purse carried by each fresh visit. Player sales debit it; player purchases
/// replenish it. A restart resumes the same purse instead of minting more coin.
pub const TRADER_STARTING_COIN: f64 = 240.0;

#[must_use]
pub fn max_item_units_per_load(item: Item) -> u32 {
    TRADER_ITEM_LOAD_LIMIT_GRAMS / item_weight_grams(item).max(1)
}

/// A visiting trader's lifecycle state, in visit order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraderState {
    /// Walking in from a persisted reachable exterior point toward the village shrine.
    Arriving,
    /// Parked at the shrine; `SellGoods` / `BuyResource` are valid while in this state.
    Trading,
    /// Walking back to the same persisted exterior point; despawns on arrival.
    Departing,
}

/// Deterministic finite resource manifest for one visit. The visit number rotates
/// quantities by one or two units so a new caravan is visibly a fresh manifest while
/// identical histories remain byte-identical. No tick or action ever calls this again
/// for an in-progress visit.
#[must_use]
pub fn stock_for_visit(
    world_seed: u32,
    colony_id: &str,
    visit_number: u64,
) -> BTreeMap<ResourceKind, f64> {
    let mut manifest_seed = u64::from(world_seed) ^ visit_number.rotate_left(17);
    for byte in b"idle-cat-forest/trader-manifest/v1"
        .iter()
        .chain(colony_id.as_bytes())
    {
        manifest_seed ^= u64::from(*byte);
        manifest_seed = manifest_seed.wrapping_mul(1_099_511_628_211);
    }
    ResourceKind::ALL
        .iter()
        .copied()
        .filter(|kind| resource_unit_price(*kind).is_some())
        .enumerate()
        .map(|(index, kind)| {
            let rotation = manifest_seed
                .wrapping_add(index as u64 * 2)
                .rotate_left((index % 63) as u32)
                % 3;
            (kind, 3.0 + rotation as f64)
        })
        .collect()
}

#[must_use]
pub fn coin_for_visit(world_seed: u32, colony_id: &str, visit_number: u64) -> f64 {
    let manifest = stock_for_visit(world_seed, colony_id, visit_number);
    TRADER_STARTING_COIN
        + manifest
            .iter()
            .enumerate()
            .map(|(index, (_, amount))| *amount * (index % 3) as f64)
            .sum::<f64>()
}

/// Physical cargo weight of the merchant's remaining resources plus every item bought
/// from the colony. ItemStore identity/condition stays intact while aboard the wagon.
#[must_use]
pub fn cargo_weight_grams(stock: &BTreeMap<ResourceKind, f64>, items: &ItemStore) -> f64 {
    let resources = stock
        .values()
        .map(|amount| amount.max(0.0) * TRADER_RESOURCE_UNIT_WEIGHT_GRAMS)
        .sum::<f64>();
    let item_weight = items
        .iter()
        .map(|(item, count)| f64::from(item_weight_grams(*item)) * f64::from(*count))
        .sum::<f64>();
    resources + item_weight
}

/// Coin the trader pays the colony for `count` of `item` (the trader's buy price).
/// Deterministic integer-percent math over [`item_value`] — no floating drift beyond
/// what `item_value` itself already does.
#[must_use]
pub fn trader_buy_price(item: Item, count: u32) -> f64 {
    f64::from(item_value(item.kind, item.material, item.quality))
        * f64::from(count)
        * f64::from(TRADER_BUY_PRICE_PCT)
        / 100.0
}

/// Base coin-per-unit price of a raw/intermediate resource, loosely mirroring
/// [`crate::threat::colony_wealth`]'s relative resource weighting (refined outvalues raw
/// food/water/herbs/materials) for a consistent sense of "value" across the sim. `None`
/// for finite functional equipment: those units must retain their stable identity and
/// move through the item-trade path rather than being reconstructed from an aggregate.
/// Blessings are the gods' own currency, not a caravan trade good.
#[must_use]
pub const fn resource_unit_price(kind: ResourceKind) -> Option<u32> {
    match kind {
        ResourceKind::Food
        | ResourceKind::Fish
        | ResourceKind::Water
        | ResourceKind::Herbs
        | ResourceKind::Catnip
        | ResourceKind::Grain
        | ResourceKind::Logs
        | ResourceKind::Stone
        | ResourceKind::Clay
        | ResourceKind::Sand
        | ResourceKind::Materials => Some(1),
        ResourceKind::Flour
        | ResourceKind::Preserves
        | ResourceKind::Medicine
        | ResourceKind::Brew
        | ResourceKind::Lumber
        | ResourceKind::Planks
        | ResourceKind::Blocks
        | ResourceKind::Refined => Some(3),
        ResourceKind::Tools
        | ResourceKind::Weapons
        | ResourceKind::Armor
        | ResourceKind::Fibre
        | ResourceKind::Hide
        | ResourceKind::Bone
        | ResourceKind::Cloth
        | ResourceKind::Leather
        | ResourceKind::Ore
        | ResourceKind::Gem
        | ResourceKind::Metal
        | ResourceKind::Blessings => None,
    }
}

/// Coin cost to buy `amount` of `kind` from the trader (the trader's sell price to the
/// colony). `None` if the trader does not stock that resource kind.
#[must_use]
pub fn trader_sell_price(kind: ResourceKind, amount: f64) -> Option<f64> {
    let unit = resource_unit_price(kind)?;
    Some(amount * f64::from(unit) * f64::from(TRADER_SELL_PRICE_PCT) / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::{ItemKind, Material};

    #[test]
    fn trader_buy_price_is_a_flat_percent_of_item_value() {
        let item = Item::new(ItemKind::Mug, Material::Wood, 1);
        // 4 (mug/wood/common item_value) * 3 count * 60% = 7.2
        assert_eq!(trader_buy_price(item, 3), 7.2);
        assert_eq!(trader_buy_price(item, 0), 0.0);
    }

    #[test]
    fn trader_buy_price_scales_with_item_value_ordering() {
        let cheap = Item::new(ItemKind::Mug, Material::Wood, 0);
        let pricey = Item::new(ItemKind::Weapon, Material::Metal, 4);
        assert!(trader_buy_price(pricey, 1) > trader_buy_price(cheap, 1));
    }

    #[test]
    fn resource_unit_price_excludes_identity_bearing_equipment_and_blessings() {
        assert_eq!(resource_unit_price(ResourceKind::Tools), None);
        assert_eq!(resource_unit_price(ResourceKind::Weapons), None);
        assert_eq!(resource_unit_price(ResourceKind::Armor), None);
        assert_eq!(resource_unit_price(ResourceKind::Bone), None);
        assert_eq!(resource_unit_price(ResourceKind::Blessings), None);
        assert_eq!(resource_unit_price(ResourceKind::Food), Some(1));
        assert_eq!(resource_unit_price(ResourceKind::Stone), Some(1));
        assert_eq!(resource_unit_price(ResourceKind::Refined), Some(3));
    }

    #[test]
    fn trader_sell_price_is_amount_times_unit_price_times_markup() {
        // 10 food * 1 (unit) * 150% = 15.0
        assert_eq!(trader_sell_price(ResourceKind::Food, 10.0), Some(15.0));
        // 10 refined * 3 (unit) * 150% = 45.0
        assert_eq!(trader_sell_price(ResourceKind::Refined, 10.0), Some(45.0));
        assert_eq!(trader_sell_price(ResourceKind::Tools, 10.0), None);
        assert_eq!(trader_sell_price(ResourceKind::Weapons, 10.0), None);
    }

    #[test]
    fn trader_sell_price_of_zero_amount_is_zero_not_none() {
        assert_eq!(trader_sell_price(ResourceKind::Food, 0.0), Some(0.0));
    }

    #[test]
    fn every_manifest_in_a_seed_colony_visit_matrix_has_a_full_sale_load_of_headroom() {
        for world_seed in [0, 1, 99, 0xdead_beef, u32::MAX] {
            for colony_id in ["colony-1", "global", "personal-far-away"] {
                for visit in 1..=1_024 {
                    let left = stock_for_visit(world_seed, colony_id, visit);
                    let right = stock_for_visit(world_seed, colony_id, visit);
                    assert_eq!(left, right);
                    assert!(
                        left.values()
                            .all(|amount| amount.is_finite() && *amount > 0.0)
                    );
                    assert!(
                        cargo_weight_grams(&left, &crate::items::ItemStore::default())
                            + f64::from(TRADER_ITEM_LOAD_LIMIT_GRAMS)
                            <= TRADER_CARGO_CAPACITY_GRAMS,
                        "seed={world_seed} colony={colony_id} visit={visit}"
                    );
                    assert!(coin_for_visit(world_seed, colony_id, visit) > 0.0);
                }
            }
        }
    }

    #[test]
    fn next_visit_restock_is_deterministic_but_is_a_fresh_manifest() {
        let first = stock_for_visit(99, "colony-1", 1);
        let second = stock_for_visit(99, "colony-1", 2);
        assert_ne!(first, second);
        assert_eq!(second, stock_for_visit(99, "colony-1", 2));
    }
}
