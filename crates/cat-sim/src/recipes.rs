//! P19 slice 2 — material-variant trade-good recipes for crafting benches.
//!
//! Per `docs/migration/specs/p19-items-materials-trade.md` + the orchestrator's slice-2
//! design decision: the [`crate::items`] layer is *trade/decoration* goods only (mug,
//! bowl, furniture, clothing, trinket, toy) — it never touches the functional
//! weapon/armor/tool [`crate::entities::Resources`] fields the smithy/woodworking
//! already forge for raid combat. This module is additive alongside those existing
//! chains: a compact per-bench [`CraftRecipe`] turns a **surplus** of an existing
//! intermediate resource (planks, blocks) into a rotating trade-good [`crate::items::Item`]
//! on its own cycle timer, so it never competes with or displaces the functional output.
//!
//! Mapping (spec: "Woodworking bench → Wood trade goods ... from planks", "StonePrep ...
//! → Stone trade goods ... from blocks"):
//! - [`WOOD_TRADE_RECIPE`]: Woodworking bench, [`crate::items::Material::Wood`], spends
//!   planks, kinds rotate through Mug / Bowl / Furniture / Toy.
//! - [`STONE_TRADE_RECIPE`]: StonePrep bench, [`crate::items::Material::Stone`], spends
//!   blocks, kinds rotate through Bowl / Trinket.
//!
//! Clothing (spec's third material-good line, P16/P19 deferred slice) lands via two more
//! benches once the raw fibre/hide → cloth/leather chain exists
//! ([`crate::entities::Resources::fibre`]/`hide`/`cloth`/`leather`,
//! [`crate::types::BuildingType::Clothier`]/[`Tannery`](crate::types::BuildingType::Tannery)):
//! - [`CLOTH_TRADE_RECIPE`]: Clothier bench, [`crate::items::Material::Fibre`], spends
//!   cloth, kinds: Clothing.
//! - [`LEATHER_TRADE_RECIPE`]: Tannery bench, [`crate::items::Material::Leather`], spends
//!   leather, kinds: Clothing.

use crate::items::{ItemKind, ItemStore, MAX_QUALITY, Material};
use crate::life_sim::trade_level;
use crate::production::ARCHITECT_SPEED;

/// A crafting bench's compact recipe: which [`Material`] it works, the rotating list of
/// [`ItemKind`]s it can produce, how much of its intermediate resource one cycle
/// consumes, how long a cycle takes, and the **surplus reserve** — the floor of the
/// intermediate resource that stays off-limits to trade crafting so the functional
/// economy (construction, tools, smithy) always gets first claim.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CraftRecipe {
    pub material: Material,
    pub kinds: &'static [ItemKind],
    pub intermediate_per_cycle: f64,
    pub cycle_sec: f64,
    pub surplus_reserve: f64,
}

/// Woodworking bench: planks → wood trade goods. `surplus_reserve` (20 planks) sits well
/// above a scaffold's `SCAFFOLD_PLANK_COST` (2.0) and one tools cycle's plank draw (2.0
/// per `WOODWORKING_CYCLE_SEC`), so trade crafting only ever spends planks construction
/// and tools were never going to need in the next several cycles. `cycle_sec` (900s) is
/// slower than the 600s tools cadence — a deliberate luxury tier, not a competing one.
pub const WOOD_TRADE_RECIPE: CraftRecipe = CraftRecipe {
    material: Material::Wood,
    kinds: &[
        ItemKind::Mug,
        ItemKind::Bowl,
        ItemKind::Furniture,
        ItemKind::Toy,
    ],
    intermediate_per_cycle: 1.0,
    cycle_sec: 900.0,
    surplus_reserve: 20.0,
};

/// StonePrep bench: blocks → stone trade goods. Mirrors [`WOOD_TRADE_RECIPE`]'s
/// reserve/cadence reasoning, sized against `SCAFFOLD_BLOCK_COST` (2.0) and the tools
/// recipe's block draw (2.0 per cycle).
pub const STONE_TRADE_RECIPE: CraftRecipe = CraftRecipe {
    material: Material::Stone,
    kinds: &[ItemKind::Bowl, ItemKind::Trinket],
    intermediate_per_cycle: 1.0,
    cycle_sec: 900.0,
    surplus_reserve: 20.0,
};

/// Clothier bench: cloth → clothing (P16/P19 clothing chain slice). The clothier has
/// no functional (non-trade) recipe competing for cloth — unlike planks/blocks, cloth
/// only ever feeds this trade craft — so the reserve exists purely to keep a small
/// buffer against the clothier's own refine cycle (fibre → cloth) outproducing this
/// craft cycle in a single tick. Mirrors [`WOOD_TRADE_RECIPE`]'s cadence/reserve shape.
pub const CLOTH_TRADE_RECIPE: CraftRecipe = CraftRecipe {
    material: Material::Fibre,
    kinds: &[ItemKind::Clothing],
    intermediate_per_cycle: 1.0,
    cycle_sec: 900.0,
    surplus_reserve: 20.0,
};

/// Tannery bench: leather → clothing. Mirrors [`CLOTH_TRADE_RECIPE`]'s reasoning for
/// the tannery's own hide → leather refine.
pub const LEATHER_TRADE_RECIPE: CraftRecipe = CraftRecipe {
    material: Material::Leather,
    kinds: &[ItemKind::Clothing],
    intermediate_per_cycle: 1.0,
    cycle_sec: 900.0,
    surplus_reserve: 20.0,
};

/// Inputs for one trade-craft tick: whether a worker is present/fast, and how much of
/// the recipe's intermediate resource is on hand (before the surplus reserve is
/// subtracted — [`advance_craft`] applies the reserve itself).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CraftOptions {
    pub has_worker: bool,
    pub worker_is_architect: bool,
    pub intermediate_available: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CraftStep {
    /// Carry-over cycle time in seconds after this tick.
    pub next_progress: f64,
    /// Intermediate resource (planks/blocks) consumed this tick.
    pub intermediate_used: f64,
    /// Whole trade-good items produced this tick (all the same [`ItemKind`] — the
    /// caller resolves which kind via [`next_trade_kind`] once per tick).
    pub items_produced: u32,
}

/// Advance a staffed craft bench: elapsed time and the bench's own progress timer
/// mirror [`crate::production::advance_workshop`]'s cadence, but cycles are additionally
/// floored by the *surplus* above `recipe.surplus_reserve` — never the raw available
/// amount — so trade crafting can only ever spend what the functional economy has no
/// claim on. Architects run the bench at double speed, matching every other bench.
#[must_use]
pub fn advance_craft(
    progress_sec: f64,
    elapsed_sec: f64,
    options: CraftOptions,
    recipe: &CraftRecipe,
) -> CraftStep {
    if !options.has_worker || elapsed_sec <= 0.0 {
        return CraftStep {
            next_progress: progress_sec,
            intermediate_used: 0.0,
            items_produced: 0,
        };
    }

    let speed = if options.worker_is_architect {
        ARCHITECT_SPEED
    } else {
        1.0
    };
    let mut progress = progress_sec + elapsed_sec * speed;

    // Surplus gate: only spend above the reserve floor.
    let spendable = (options.intermediate_available - recipe.surplus_reserve).max(0.0);

    let cycles_by_time = (progress / recipe.cycle_sec).floor();
    let cycles_by_material = (spendable / recipe.intermediate_per_cycle).floor();
    let cycles = cycles_by_time.min(cycles_by_material).max(0.0);

    progress -= cycles * recipe.cycle_sec;
    progress = progress.min(recipe.cycle_sec);

    CraftStep {
        next_progress: progress,
        intermediate_used: cycles * recipe.intermediate_per_cycle,
        items_produced: cycles as u32,
    }
}

/// Deterministically picks which [`ItemKind`] the next completed cycle of `recipe`
/// should produce: counts how many of `recipe`'s kinds (in `recipe.material`) the
/// colony has already crafted and rotates through `recipe.kinds` by that count. Pure —
/// depends only on the current item store, no RNG, no hidden counter. All items
/// produced by a single tick's cycles share this one kind (a tick can complete more
/// than one cycle only after a long elapsed-time jump; splitting kinds mid-tick would
/// need to simulate cycle-by-cycle, which isn't worth the complexity for a rare edge —
/// rotation still advances correctly on the next tick from the updated store count).
#[must_use]
pub fn next_trade_kind(items: &ItemStore, recipe: &CraftRecipe) -> ItemKind {
    let crafted_so_far: u32 = items
        .iter()
        .filter(|(item, _count)| {
            item.material == recipe.material && recipe.kinds.contains(&item.kind)
        })
        .map(|(_item, count)| *count)
        .sum();
    recipe.kinds[(crafted_so_far as usize) % recipe.kinds.len()]
}

/// Deterministic crafted-item quality band (0..=[`MAX_QUALITY`]) from a crafter's
/// [`crate::skills::Labor::Craft`] proficiency (`Cat::skill`). Reuses the same
/// [`trade_level`] curve (`floor(sqrt(xp))`) the rest of the sim uses for skill depth,
/// then compresses every 5 trade levels into one quality band:
///
/// | skill xp | trade level | quality band     |
/// |----------|-------------|------------------|
/// | 0        | 0           | 0 (crude)        |
/// | 25       | 5           | 1 (common)       |
/// | 100      | 10          | 2 (fine)         |
/// | 225      | 15          | 3 (superior)     |
/// | 400+     | 20+         | 4 (masterwork)   |
///
/// Pure and RNG-free: a more-skilled crafter always yields quality >= a less-skilled
/// one for the same recipe. No worker (xp 0.0) yields the lowest band (0).
#[must_use]
pub fn craft_quality_from_skill(skill_xp: f64) -> u8 {
    let level = trade_level(skill_xp.max(0.0));
    let band = (level / 5.0).floor().min(f64::from(MAX_QUALITY));
    band as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::{Item, add_item};

    fn options(
        has_worker: bool,
        worker_is_architect: bool,
        intermediate_available: f64,
    ) -> CraftOptions {
        CraftOptions {
            has_worker,
            worker_is_architect,
            intermediate_available,
        }
    }

    #[test]
    fn recipes_are_compact_and_documented_mappings() {
        assert_eq!(WOOD_TRADE_RECIPE.material, Material::Wood);
        assert_eq!(
            WOOD_TRADE_RECIPE.kinds,
            &[
                ItemKind::Mug,
                ItemKind::Bowl,
                ItemKind::Furniture,
                ItemKind::Toy
            ]
        );
        assert_eq!(STONE_TRADE_RECIPE.material, Material::Stone);
        assert_eq!(
            STONE_TRADE_RECIPE.kinds,
            &[ItemKind::Bowl, ItemKind::Trinket]
        );
        assert_eq!(CLOTH_TRADE_RECIPE.material, Material::Fibre);
        assert_eq!(CLOTH_TRADE_RECIPE.kinds, &[ItemKind::Clothing]);
        assert_eq!(LEATHER_TRADE_RECIPE.material, Material::Leather);
        assert_eq!(LEATHER_TRADE_RECIPE.kinds, &[ItemKind::Clothing]);
    }

    #[test]
    fn clothing_recipes_craft_clothing_items_from_surplus_cloth_and_leather() {
        // Cloth bench: below the reserve produces nothing, above it crafts Clothing.
        let starved = advance_craft(0.0, 900.0, options(true, false, 15.0), &CLOTH_TRADE_RECIPE);
        assert_eq!(starved.items_produced, 0);

        let step = advance_craft(0.0, 900.0, options(true, false, 21.0), &CLOTH_TRADE_RECIPE);
        assert_eq!(step.items_produced, 1);
        assert_eq!(step.intermediate_used, 1.0);
        assert_eq!(
            next_trade_kind(&ItemStore::default(), &CLOTH_TRADE_RECIPE),
            ItemKind::Clothing
        );

        // Leather bench mirrors the same shape.
        let starved = advance_craft(
            0.0,
            900.0,
            options(true, false, 15.0),
            &LEATHER_TRADE_RECIPE,
        );
        assert_eq!(starved.items_produced, 0);

        let step = advance_craft(
            0.0,
            900.0,
            options(true, false, 21.0),
            &LEATHER_TRADE_RECIPE,
        );
        assert_eq!(step.items_produced, 1);
        assert_eq!(step.intermediate_used, 1.0);
        assert_eq!(
            next_trade_kind(&ItemStore::default(), &LEATHER_TRADE_RECIPE),
            ItemKind::Clothing
        );
    }

    #[test]
    fn craft_does_not_progress_without_worker_or_positive_elapsed_time() {
        let step = advance_craft(
            100.0,
            900.0,
            options(false, false, 500.0),
            &WOOD_TRADE_RECIPE,
        );
        assert_eq!(
            step,
            CraftStep {
                next_progress: 100.0,
                intermediate_used: 0.0,
                items_produced: 0,
            }
        );

        let step = advance_craft(100.0, 0.0, options(true, false, 500.0), &WOOD_TRADE_RECIPE);
        assert_eq!(
            step,
            CraftStep {
                next_progress: 100.0,
                intermediate_used: 0.0,
                items_produced: 0,
            }
        );
    }

    #[test]
    fn craft_below_the_surplus_reserve_produces_nothing() {
        // 15 planks is below WOOD_TRADE_RECIPE's 20-plank reserve: the bench must never
        // dip into what construction/tools might still need, even with a full cycle of
        // elapsed time.
        let step = advance_craft(0.0, 900.0, options(true, false, 15.0), &WOOD_TRADE_RECIPE);
        assert_eq!(step.items_produced, 0);
        assert_eq!(step.intermediate_used, 0.0);
        assert_eq!(step.next_progress, 900.0);
    }

    #[test]
    fn craft_spends_only_the_surplus_above_the_reserve() {
        // 22 planks available, reserve 20 -> only 2.0 spendable, enough for two 1.0-plank
        // cycles; a third would need 3.0 spendable, which isn't there.
        let step = advance_craft(0.0, 2_700.0, options(true, false, 22.0), &WOOD_TRADE_RECIPE);
        assert_eq!(step.items_produced, 2);
        assert_eq!(step.intermediate_used, 2.0);
    }

    #[test]
    fn craft_accumulates_short_ticks_and_completes_a_cycle() {
        let step = advance_craft(890.0, 30.0, options(true, false, 100.0), &WOOD_TRADE_RECIPE);
        assert_eq!(
            step,
            CraftStep {
                next_progress: 20.0,
                intermediate_used: 1.0,
                items_produced: 1,
            }
        );
    }

    #[test]
    fn architect_worker_runs_the_bench_at_double_speed() {
        let step = advance_craft(0.0, 450.0, options(true, true, 100.0), &WOOD_TRADE_RECIPE);
        assert_eq!(
            step,
            CraftStep {
                next_progress: 0.0,
                intermediate_used: 1.0,
                items_produced: 1,
            }
        );
    }

    #[test]
    fn next_trade_kind_rotates_deterministically_by_crafted_count() {
        let mut store = ItemStore::default();
        assert_eq!(next_trade_kind(&store, &WOOD_TRADE_RECIPE), ItemKind::Mug);

        add_item(&mut store, Item::new(ItemKind::Mug, Material::Wood, 0), 1);
        assert_eq!(next_trade_kind(&store, &WOOD_TRADE_RECIPE), ItemKind::Bowl);

        add_item(&mut store, Item::new(ItemKind::Bowl, Material::Wood, 2), 1);
        assert_eq!(
            next_trade_kind(&store, &WOOD_TRADE_RECIPE),
            ItemKind::Furniture
        );

        add_item(
            &mut store,
            Item::new(ItemKind::Furniture, Material::Wood, 1),
            1,
        );
        assert_eq!(next_trade_kind(&store, &WOOD_TRADE_RECIPE), ItemKind::Toy);

        add_item(&mut store, Item::new(ItemKind::Toy, Material::Wood, 3), 1);
        // Wraps back around after all four kinds are represented once.
        assert_eq!(next_trade_kind(&store, &WOOD_TRADE_RECIPE), ItemKind::Mug);
    }

    #[test]
    fn next_trade_kind_ignores_other_materials_and_non_recipe_kinds() {
        let mut store = ItemStore::default();
        // Stone mugs/weapons must not shift the wood rotation.
        add_item(&mut store, Item::new(ItemKind::Mug, Material::Stone, 0), 5);
        add_item(
            &mut store,
            Item::new(ItemKind::Weapon, Material::Wood, 0),
            5,
        );
        assert_eq!(next_trade_kind(&store, &WOOD_TRADE_RECIPE), ItemKind::Mug);
    }

    #[test]
    fn quality_is_deterministic_and_monotonic_in_skill() {
        assert_eq!(craft_quality_from_skill(0.0), 0);
        assert_eq!(craft_quality_from_skill(24.0), 0);
        assert_eq!(craft_quality_from_skill(25.0), 1);
        assert_eq!(craft_quality_from_skill(100.0), 2);
        assert_eq!(craft_quality_from_skill(225.0), 3);
        assert_eq!(craft_quality_from_skill(400.0), 4);
        // Clamps at MAX_QUALITY, never panics or overflows past it.
        assert_eq!(craft_quality_from_skill(1_000_000.0), MAX_QUALITY);
        // Negative xp (shouldn't happen, but stay defensive) floors to 0.
        assert_eq!(craft_quality_from_skill(-5.0), 0);
    }

    #[test]
    fn quality_never_decreases_as_skill_increases() {
        let samples = [
            0.0, 10.0, 24.0, 25.0, 60.0, 100.0, 150.0, 225.0, 300.0, 400.0, 900.0,
        ];
        let mut previous = 0u8;
        for xp in samples {
            let quality = craft_quality_from_skill(xp);
            assert!(
                quality >= previous,
                "xp {xp} quality {quality} < previous {previous}"
            );
            previous = quality;
        }
    }

    #[test]
    fn higher_skill_crafter_never_yields_lower_quality_for_the_same_recipe() {
        let low_skill_quality = craft_quality_from_skill(5.0);
        let high_skill_quality = craft_quality_from_skill(500.0);
        assert!(high_skill_quality >= low_skill_quality);
        assert!(high_skill_quality > low_skill_quality);
    }
}
