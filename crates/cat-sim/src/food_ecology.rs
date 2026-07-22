//! Deterministic renewable wilderness food sites: caves, apple trees, berry bushes,
//! and fishing habitats. The world seed and tile coordinate completely determine a
//! site's profile; mutable stock remains in the authoritative world ledger.

use crate::{
    noise::{HashSeedPart, hash_seed},
    types::TileType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoodSiteKind {
    Cave,
    AppleTree,
    BerryBush,
    Fishing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoodSiteProfile {
    pub kind: FoodSiteKind,
    pub level: u8,
    pub capacity: u32,
    pub initial_stock: u32,
    pub regen_per_game_hour: f64,
}

#[must_use]
pub fn terrestrial_food_site(
    world_seed: u32,
    x: i32,
    y: i32,
    tile_type: TileType,
) -> Option<FoodSiteProfile> {
    if matches!(tile_type, TileType::Mountains | TileType::CaveEntrance)
        && site_roll(world_seed, x, y, "cave-presence") % 10_000 < 180
    {
        return Some(cave_profile(world_seed, x, y));
    }

    let forest = matches!(
        tile_type,
        TileType::Forest
            | TileType::DenseWoods
            | TileType::OakForest
            | TileType::PineForest
            | TileType::Jungle
    );
    if forest && site_roll(world_seed, x, y, "apple-presence") % 10_000 < 120 {
        return Some(fruit_profile(world_seed, x, y, FoodSiteKind::AppleTree));
    }
    let bush_ground = forest
        || matches!(
            tile_type,
            TileType::Field | TileType::Meadow | TileType::Swamp
        );
    (bush_ground && site_roll(world_seed, x, y, "berry-presence") % 10_000 < 450)
        .then(|| fruit_profile(world_seed, x, y, FoodSiteKind::BerryBush))
}

#[must_use]
pub fn cave_profile(world_seed: u32, x: i32, y: i32) -> FoodSiteProfile {
    let level = 1 + site_roll(world_seed, x, y, "cave-level") % 100;
    let capacity = 18 + site_roll(world_seed, x, y, "cave-capacity") % 143;
    let initial_percent = 25 + site_roll(world_seed, x, y, "cave-stock") % 76;
    FoodSiteProfile {
        kind: FoodSiteKind::Cave,
        level: level as u8,
        capacity,
        initial_stock: (capacity * initial_percent / 100).max(1),
        regen_per_game_hour: f64::from(8 + site_roll(world_seed, x, y, "cave-regen") % 41) / 100.0,
    }
}

#[must_use]
pub fn fishing_profile(world_seed: u32, x: i32, y: i32) -> FoodSiteProfile {
    let capacity = 16 + site_roll(world_seed, x, y, "fish-capacity") % 33;
    let initial_percent = 50 + site_roll(world_seed, x, y, "fish-stock") % 51;
    FoodSiteProfile {
        kind: FoodSiteKind::Fishing,
        level: 0,
        capacity,
        initial_stock: (capacity * initial_percent / 100).max(1),
        regen_per_game_hour: f64::from(25 + site_roll(world_seed, x, y, "fish-regen") % 76) / 100.0,
    }
}

#[must_use]
pub fn replenish_whole_units(
    stock: u32,
    capacity: u32,
    last_replenished_at_ms: i64,
    now_ms: i64,
    game_time_scale: f64,
    regen_per_game_hour: f64,
) -> (u32, i64) {
    if capacity == 0
        || stock >= capacity
        || now_ms <= last_replenished_at_ms
        || !game_time_scale.is_finite()
        || game_time_scale <= 0.0
        || !regen_per_game_hour.is_finite()
        || regen_per_game_hour <= 0.0
    {
        return (stock.min(capacity), last_replenished_at_ms.min(now_ms));
    }
    let elapsed_ms = now_ms.saturating_sub(last_replenished_at_ms) as f64;
    let generated =
        (elapsed_ms / 3_600_000.0 * game_time_scale * regen_per_game_hour).floor() as u32;
    if generated == 0 {
        return (stock, last_replenished_at_ms);
    }
    let accepted = generated.min(capacity - stock);
    let next_stock = stock + accepted;
    let next_cursor = if next_stock == capacity {
        now_ms
    } else {
        let consumed_ms = (f64::from(accepted) / (game_time_scale * regen_per_game_hour)
            * 3_600_000.0)
            .round() as i64;
        last_replenished_at_ms
            .saturating_add(consumed_ms)
            .min(now_ms)
    };
    (next_stock, next_cursor)
}

#[must_use]
pub fn cave_injury_chance(
    level: u8,
    hunting_skill: f64,
    fighting_skill: f64,
    group_size: u8,
    weapons: u8,
) -> f64 {
    let expertise = (hunting_skill.max(0.0) + fighting_skill.max(0.0)) * 0.5;
    let group_support = f64::from(group_size.saturating_sub(1)) * 8.0;
    let equipment = f64::from(weapons) * 7.0;
    let margin = f64::from(level) - expertise - group_support - equipment;
    let risk = if margin >= 0.0 {
        0.03 + margin * 0.008
    } else {
        0.03 + margin * 0.003
    };
    risk.clamp(0.01, 0.80)
}

fn fruit_profile(world_seed: u32, x: i32, y: i32, kind: FoodSiteKind) -> FoodSiteProfile {
    let (capacity_min, capacity_span, stock_min, regen_min, regen_span, tag) = match kind {
        FoodSiteKind::AppleTree => (8, 17, 40, 18, 48, "apple"),
        FoodSiteKind::BerryBush => (5, 12, 35, 35, 86, "berry"),
        FoodSiteKind::Cave | FoodSiteKind::Fishing => unreachable!("not a fruit site"),
    };
    let capacity =
        capacity_min + site_roll(world_seed, x, y, &format!("{tag}-capacity")) % capacity_span;
    let initial_percent =
        stock_min + site_roll(world_seed, x, y, &format!("{tag}-stock")) % (101 - stock_min);
    FoodSiteProfile {
        kind,
        level: 0,
        capacity,
        initial_stock: (capacity * initial_percent / 100).max(1),
        regen_per_game_hour: f64::from(
            regen_min + site_roll(world_seed, x, y, &format!("{tag}-regen")) % regen_span,
        ) / 100.0,
    }
}

fn site_roll(world_seed: u32, x: i32, y: i32, purpose: &str) -> u32 {
    hash_seed(&[
        HashSeedPart::Number(f64::from(world_seed)),
        HashSeedPart::Number(f64::from(x)),
        HashSeedPart::Number(f64::from(y)),
        HashSeedPart::Text(purpose),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cave_profiles_are_deterministic_but_vary_in_level_stock_and_regeneration() {
        let first = cave_profile(43, 20, -9);
        assert_eq!(first, cave_profile(43, 20, -9));
        assert_eq!(first.kind, FoodSiteKind::Cave);

        let profiles = (-40..40)
            .map(|x| cave_profile(43, x, x * 3 + 1))
            .collect::<Vec<_>>();
        assert!(profiles.iter().all(|site| (1..=100).contains(&site.level)));
        assert!(
            profiles
                .iter()
                .all(|site| site.initial_stock <= site.capacity)
        );
        assert!(
            profiles.iter().map(|site| site.level).min()
                < profiles.iter().map(|site| site.level).max()
        );
        assert!(
            profiles.iter().map(|site| site.capacity).min()
                < profiles.iter().map(|site| site.capacity).max()
        );
        assert!(
            profiles
                .iter()
                .map(|site| site.regen_per_game_hour.to_bits())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                > 8
        );
    }

    #[test]
    fn apple_trees_are_sparse_and_berry_bushes_are_more_common() {
        let mut apples = 0;
        let mut berries = 0;
        for y in -100..100 {
            for x in -100..100 {
                match terrestrial_food_site(77, x, y, TileType::Forest).map(|site| site.kind) {
                    Some(FoodSiteKind::AppleTree) => apples += 1,
                    Some(FoodSiteKind::BerryBush) => berries += 1,
                    _ => {}
                }
            }
        }
        assert!(apples > 0, "the world must contain some apple trees");
        assert!(
            berries > apples * 2,
            "berries should be meaningfully more common"
        );
        assert!(
            apples < 800,
            "apple trees stay rare across 40,000 forest tiles"
        );
    }

    #[test]
    fn fishing_habitats_have_site_specific_capacity_and_regeneration() {
        let sites = (0..64)
            .map(|x| fishing_profile(99, x, x * 2))
            .collect::<Vec<_>>();
        assert!(sites.iter().all(|site| site.kind == FoodSiteKind::Fishing));
        assert!(
            sites.iter().map(|site| site.capacity).min()
                < sites.iter().map(|site| site.capacity).max()
        );
        assert!(
            sites
                .iter()
                .map(|site| site.regen_per_game_hour.to_bits())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                > 8
        );
    }

    #[test]
    fn renewable_stock_preserves_fractional_time_and_never_exceeds_capacity() {
        let hour = 3_600_000;
        let (unchanged, cursor) = replenish_whole_units(0, 20, 1_000, 1_000 + hour, 1.0, 0.5);
        assert_eq!(unchanged, 0);
        assert_eq!(cursor, 1_000, "half a unit remains banked as elapsed time");

        let (one, cursor) = replenish_whole_units(0, 20, cursor, 1_000 + hour * 2, 1.0, 0.5);
        assert_eq!(one, 1);
        assert_eq!(cursor, 1_000 + hour * 2);

        let (capped, _) = replenish_whole_units(19, 20, 0, hour * 100, 1.0, 4.0);
        assert_eq!(capped, 20);
    }

    #[test]
    fn cave_injury_risk_rises_with_level_and_falls_with_skill_groups_and_weapons() {
        let novice_solo = cave_injury_chance(80, 10.0, 10.0, 1, 0);
        let veteran_solo = cave_injury_chance(80, 80.0, 80.0, 1, 0);
        let armed_group = cave_injury_chance(80, 80.0, 80.0, 4, 4);
        assert!(novice_solo > veteran_solo);
        assert!(veteran_solo > armed_group);
        assert!(
            cave_injury_chance(90, 40.0, 40.0, 1, 0) > cave_injury_chance(20, 40.0, 40.0, 1, 0)
        );
        assert!((0.0..=1.0).contains(&novice_solo));
    }
}
