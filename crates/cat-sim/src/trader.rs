//! Visiting trader / caravan economy (P19 slice 3) — a periodic friendly NPC plus a
//! simple coin economy. Lifecycle-and-movement-wise this is a much simpler analogue of
//! the raider warband in [`crate::world_tick`]'s threat/raid director (spawn off-map,
//! walk in on the same [`crate::movement::walk_path`] primitive, linger, walk back out,
//! despawn) — no combat, no threat interaction.
//!
//! Per `docs/migration/specs/p19-items-materials-trade.md`'s "Traders / caravans"
//! section: a trader arrives at the village (walks to the gate), stays a while, and
//! opens a trade menu — sell surplus/crafted [`crate::items::Item`] goods for coin, buy
//! [`crate::stockpiles::ResourceKind`] resources with coin. "A simple value/coin
//! economy; relations/price could deepen later" — this module is exactly that: flat,
//! documented, RNG-free price functions.
//!
//! **No RNG.** The spawn schedule is a game-time threshold, movement is the same
//! deterministic straight-line [`crate::movement::walk_path`] raiders use (no obstacle
//! rolls), and pricing is pure percent math — there is nothing here that needs a forked
//! seeded chain. (The card's "if the trader needs any roll" guidance is conditional;
//! this slice needs none, which keeps the determinism story trivial: same seed, same
//! tick sequence, same trader schedule, always.)

use crate::{
    items::{Item, item_value},
    stockpiles::ResourceKind,
};

/// Game-hours from the end of one trader visit (or colony founding, for the first
/// visit) until the next one's arrival is triggered. ~2 game-days: deliberately rarer
/// than the 8h raid grace window (`crate::threat::RAID_GRACE_SEC`), so a caravan reads
/// as a calm, occasional counterpart to raids rather than a constant presence.
pub const TRADER_VISIT_INTERVAL_GAME_HOURS: f64 = 48.0;

/// Game-hours a trader lingers in [`TraderState::Trading`] at the gate before it starts
/// departing — a real window for the player to notice it and trade.
pub const TRADER_LINGER_GAME_HOURS: f64 = 6.0;

/// Tiles/sec the trader's wagon covers while arriving/departing. A touch faster than a
/// raiding warband's march (`RAIDER_SPEED_TILES_PER_SEC` = 0.4 in `world_tick.rs`) since
/// it's a lone wagon, not a formation.
pub const TRADER_SPEED_TILES_PER_SEC: f64 = 0.5;

/// Off-map standoff distance (tiles) the trader spawns at / departs to, due west of the
/// village gate. Mirrors `world_tick::RAID_SPAWN_DISTANCE`'s role for raiders (though
/// traders always approach from the same fixed heading — see the module doc's "no RNG").
pub const TRADER_SPAWN_DISTANCE: f64 = 12.0;

/// Chebyshev range at which the trader counts as "arrived" (Arriving -> Trading) or
/// "gone" (Departing -> despawned). Mirrors `world_tick::ENGAGE_RANGE`.
pub const TRADER_ARRIVE_RANGE: f64 = 1.5;

/// Percent of an item's [`item_value`] the trader pays when it *buys* that item from the
/// colony (its resale margin — it sells on at full value elsewhere). Flat and documented
/// per the spec's "simple value/coin economy" guidance; no RNG, no per-trader haggling.
pub const TRADER_BUY_PRICE_PCT: u32 = 60;

/// Percent of a resource's [`resource_unit_price`] the trader charges when it *sells*
/// that resource to the colony (its markup).
pub const TRADER_SELL_PRICE_PCT: u32 = 150;

/// A visiting trader's lifecycle state, in visit order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraderState {
    /// Walking in from the off-map spawn point toward the village gate.
    Arriving,
    /// Parked at the gate; `SellGoods` / `BuyResource` are valid while in this state.
    Trading,
    /// Walking back out to the off-map spawn point; despawns on arrival.
    Departing,
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
/// for kinds the trader does not stock: weapons/armor are functional combat gear (the
/// smithy forges them for raid defense, never for sale), and blessings are the gods' own
/// currency, not a caravan trade good.
#[must_use]
pub const fn resource_unit_price(kind: ResourceKind) -> Option<u32> {
    match kind {
        ResourceKind::Food
        | ResourceKind::Water
        | ResourceKind::Herbs
        | ResourceKind::Catnip
        | ResourceKind::Grain
        | ResourceKind::Logs
        | ResourceKind::Materials => Some(1),
        ResourceKind::Flour | ResourceKind::Lumber | ResourceKind::Refined => Some(3),
        ResourceKind::Weapons | ResourceKind::Armor | ResourceKind::Blessings => None,
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
    fn resource_unit_price_excludes_weapons_armor_and_blessings() {
        assert_eq!(resource_unit_price(ResourceKind::Weapons), None);
        assert_eq!(resource_unit_price(ResourceKind::Armor), None);
        assert_eq!(resource_unit_price(ResourceKind::Blessings), None);
        assert_eq!(resource_unit_price(ResourceKind::Food), Some(1));
        assert_eq!(resource_unit_price(ResourceKind::Refined), Some(3));
    }

    #[test]
    fn trader_sell_price_is_amount_times_unit_price_times_markup() {
        // 10 food * 1 (unit) * 150% = 15.0
        assert_eq!(trader_sell_price(ResourceKind::Food, 10.0), Some(15.0));
        // 10 refined * 3 (unit) * 150% = 45.0
        assert_eq!(trader_sell_price(ResourceKind::Refined, 10.0), Some(45.0));
        assert_eq!(trader_sell_price(ResourceKind::Weapons, 10.0), None);
    }

    #[test]
    fn trader_sell_price_of_zero_amount_is_zero_not_none() {
        assert_eq!(trader_sell_price(ResourceKind::Food, 0.0), Some(0.0));
    }
}
