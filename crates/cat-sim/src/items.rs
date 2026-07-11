//! DF-scale cat-themed item/material economy — data model only (P19 slice 1).
//!
//! Per `docs/migration/specs/p19-items-materials-trade.md`: a compact `ItemKind ×
//! Material` model gives DF-like breadth ("a wooden mug OR a stone mug") without an
//! exploding item list. This module is pure and additive: it does not touch the core
//! survival [`crate::entities::Resources`] struct (food/water/materials/planks/…),
//! which stays fast and untouched. The colony item store (`ColonyRuntime::items` in
//! [`crate::world_tick`]) is inert this slice — nothing produces or consumes items yet;
//! that lands in slice 2 (material-variant workshop recipes).

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Raw/intermediate material palette (spec: "Resource / item taxonomy"). Compact set
/// spanning the spec's raw-material list: logs/wood, stone, ore→metal, gems, plant
/// fibre, hide→leather, bone (from hunts), clay/sand. Intermediate refinement (planks,
/// blocks, bars, cloth, thread, leather, flour) stays modeled by the existing
/// [`crate::entities::Resources`] tier for now; `Material` here is what a *finished
/// good* is made of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Material {
    /// Logs/timber. The baseline (1.0x) material — common, low value.
    Wood,
    /// Quarried stone. Sturdier and pricier than wood.
    Stone,
    /// Smelted ore. The premium structural material (weapons/armor/tools).
    Metal,
    /// Bone from hunts. A step up from wood — durable, a little macabre.
    Bone,
    /// Plant fibre / cloth. Light, cheap, best for clothing.
    Fibre,
    /// Tanned hide. Pliable and mid-value — clothing, light armor.
    Leather,
    /// Mountain gems. The rarest, most valuable material.
    Gem,
    /// Clay/sand, fired or sun-dried. Cheap, common for mugs/bowls.
    Clay,
}

impl Material {
    /// Every material, in a stable order (deterministic iteration for future recipe
    /// tables and tests).
    pub const ALL: &'static [Self] = &[
        Self::Wood,
        Self::Stone,
        Self::Metal,
        Self::Bone,
        Self::Fibre,
        Self::Leather,
        Self::Gem,
        Self::Clay,
    ];

    /// Stable lowercase wire label (matches the `snake_case` serde rename).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wood => "wood",
            Self::Stone => "stone",
            Self::Metal => "metal",
            Self::Bone => "bone",
            Self::Fibre => "fibre",
            Self::Leather => "leather",
            Self::Gem => "gem",
            Self::Clay => "clay",
        }
    }

    /// Parse a wire label back into a [`Material`] (inverse of [`Self::as_str`]).
    #[must_use]
    pub fn from_str_label(label: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|m| m.as_str() == label)
    }

    /// Value multiplier in percent (100 = wood's 1.0x baseline). Drives [`item_value`]'s
    /// relative ordering: `Metal` (260%) and `Gem` (450%) outvalue `Wood` (100%);
    /// `Stone` (170%) outvalues `Wood` too, matching the spec's "metal weapon > wood
    /// weapon" / "stone mug > wood mug" examples.
    #[must_use]
    pub const fn value_multiplier(self) -> u32 {
        match self {
            Self::Fibre => 50,
            Self::Clay => 70,
            Self::Wood => 100,
            Self::Bone => 115,
            Self::Leather => 130,
            Self::Stone => 170,
            Self::Metal => 260,
            Self::Gem => 450,
        }
    }
}

/// Finished-good kinds (spec: "Finished goods (crafted, cat-themed)"). Kept compact —
/// `Furniture` covers beds/chairs/tables as one kind rather than three, per the card's
/// "keep compact" guidance; a recipe/workshop (slice 2) can still vary the *shape* via
/// metadata later without growing this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// Drinking mug.
    Mug,
    /// Eating bowl.
    Bowl,
    /// Den furniture (bed/chair/table).
    Furniture,
    /// Work tool (axe/shovel/pick/fishing-rod).
    Tool,
    /// Warrior-cat weapon (claw/blade).
    Weapon,
    /// Warrior-cat armor (mail).
    Armor,
    /// Wearable clothing.
    Clothing,
    /// Decoration/trinket.
    Trinket,
    /// Plaything for kittens.
    Toy,
}

impl ItemKind {
    /// Every kind, in a stable order (deterministic iteration for future recipe tables
    /// and tests).
    pub const ALL: &'static [Self] = &[
        Self::Mug,
        Self::Bowl,
        Self::Furniture,
        Self::Tool,
        Self::Weapon,
        Self::Armor,
        Self::Clothing,
        Self::Trinket,
        Self::Toy,
    ];

    /// Stable lowercase wire label (matches the `snake_case` serde rename).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Mug => "mug",
            Self::Bowl => "bowl",
            Self::Furniture => "furniture",
            Self::Tool => "tool",
            Self::Weapon => "weapon",
            Self::Armor => "armor",
            Self::Clothing => "clothing",
            Self::Trinket => "trinket",
            Self::Toy => "toy",
        }
    }

    /// Parse a wire label back into an [`ItemKind`] (inverse of [`Self::as_str`]).
    #[must_use]
    pub fn from_str_label(label: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|k| k.as_str() == label)
    }

    /// Base value (before material/quality multipliers). A rough complexity/utility
    /// scale, not meant to be exhaustively "realistic" — just internally consistent.
    #[must_use]
    pub const fn base_value(self) -> u32 {
        match self {
            Self::Mug => 4,
            Self::Bowl => 4,
            Self::Toy => 5,
            Self::Trinket => 6,
            Self::Clothing => 8,
            Self::Tool => 12,
            Self::Furniture => 15,
            Self::Weapon => 18,
            Self::Armor => 20,
        }
    }
}

/// Highest valid `quality` band (inclusive). Five bands: 0=crude, 1=common, 2=fine,
/// 3=superior, 4=masterwork.
pub const MAX_QUALITY: u8 = 4;

/// Quality multiplier in percent, indexed by quality band (0..=[`MAX_QUALITY`]).
/// Strictly increasing so [`item_value`] rises monotonically with quality.
const QUALITY_FACTOR_PCT: [u32; (MAX_QUALITY as usize) + 1] = [50, 100, 160, 240, 350];

/// Value = `base_value(kind) × value_multiplier(material) × quality_factor(quality) /
/// 10_000` (both multipliers are percents, so dividing by 100×100 normalizes back to
/// units). `quality` is clamped to [`MAX_QUALITY`]. Pure integer arithmetic (`u64`
/// intermediate to avoid overflow) — deterministic, no floating-point drift.
#[must_use]
pub fn item_value(kind: ItemKind, material: Material, quality: u8) -> u32 {
    let quality = quality.min(MAX_QUALITY);
    let base = u64::from(kind.base_value());
    let material_pct = u64::from(material.value_multiplier());
    let quality_pct = u64::from(QUALITY_FACTOR_PCT[quality as usize]);
    let value = (base * material_pct * quality_pct) / 10_000;
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// A single crafted item: kind × material × quality. `Ord`/`Hash` so it can key a
/// deterministic [`BTreeMap`] (see [`crate::world_tick::ColonyRuntime::items`]).
///
/// Serializes as a single compact wire string (`"kind:material:quality"`, e.g.
/// `"weapon:metal:3"`) rather than a `{kind, material, quality}` object, so a
/// `BTreeMap<Item, u32>` still round-trips as a plain JSON object (`serde_json` map
/// keys must serialize to strings; a struct key would not).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Item {
    pub kind: ItemKind,
    pub material: Material,
    pub quality: u8,
}

impl Item {
    /// Constructs an item, clamping `quality` to [`MAX_QUALITY`].
    #[must_use]
    pub fn new(kind: ItemKind, material: Material, quality: u8) -> Self {
        Self {
            kind,
            material,
            quality: quality.min(MAX_QUALITY),
        }
    }

    /// This item's unit value (see [`item_value`]).
    #[must_use]
    pub fn value(&self) -> u32 {
        item_value(self.kind, self.material, self.quality)
    }

    fn wire_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.kind.as_str(),
            self.material.as_str(),
            self.quality
        )
    }

    fn from_wire_key(key: &str) -> Option<Self> {
        let mut parts = key.split(':');
        let kind = ItemKind::from_str_label(parts.next()?)?;
        let material = Material::from_str_label(parts.next()?)?;
        let quality: u8 = parts.next()?.parse().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            kind,
            material,
            quality,
        })
    }
}

impl Serialize for Item {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.wire_key())
    }
}

impl<'de> Deserialize<'de> for Item {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::from_wire_key(&raw)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid item wire key {raw:?}")))
    }
}

/// Adds `count` of `item` to `store` (saturating; a no-op for `count == 0`).
pub fn add_item(store: &mut BTreeMap<Item, u32>, item: Item, count: u32) {
    if count == 0 {
        return;
    }
    let entry = store.entry(item).or_insert(0);
    *entry = entry.saturating_add(count);
}

/// Removes `count` of `item` from `store`. Fails (returns `false`, store left
/// untouched) if `store` holds fewer than `count`. On success, drops the entry
/// entirely once its count reaches zero (keeps the map compact for
/// `BTreeMap::is_empty` / `skip_serializing_if` checks).
pub fn remove_item(store: &mut BTreeMap<Item, u32>, item: Item, count: u32) -> bool {
    let have = store.get(&item).copied().unwrap_or(0);
    if have < count {
        return false;
    }
    let remaining = have - count;
    if remaining == 0 {
        store.remove(&item);
    } else {
        store.insert(item, remaining);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- value ordering (spec examples) -----------------------------------------

    #[test]
    fn metal_weapon_outvalues_wood_weapon() {
        let metal = item_value(ItemKind::Weapon, Material::Metal, 1);
        let wood = item_value(ItemKind::Weapon, Material::Wood, 1);
        assert!(metal > wood, "metal={metal} wood={wood}");
    }

    #[test]
    fn stone_mug_outvalues_wood_mug() {
        let stone = item_value(ItemKind::Mug, Material::Stone, 1);
        let wood = item_value(ItemKind::Mug, Material::Wood, 1);
        assert!(stone > wood, "stone={stone} wood={wood}");
    }

    #[test]
    fn quality_raises_value_monotonically() {
        let mut previous = 0;
        for quality in 0..=MAX_QUALITY {
            let value = item_value(ItemKind::Weapon, Material::Metal, quality);
            assert!(
                value > previous,
                "quality {quality} value {value} did not exceed previous {previous}"
            );
            previous = value;
        }

        // Also holds for a cheap combo (rounds down, but still strictly increases).
        let mut previous = 0;
        for quality in 0..=MAX_QUALITY {
            let value = item_value(ItemKind::Mug, Material::Fibre, quality);
            assert!(
                value > previous,
                "quality {quality} value {value} did not exceed previous {previous}"
            );
            previous = value;
        }
    }

    #[test]
    fn quality_above_max_clamps() {
        assert_eq!(
            item_value(ItemKind::Mug, Material::Wood, MAX_QUALITY),
            item_value(ItemKind::Mug, Material::Wood, u8::MAX)
        );
    }

    #[test]
    fn absolute_sanity_points() {
        // 4 (mug base) * 100 (wood 1.0x) * 100 (common) / 10_000 = 4.
        assert_eq!(item_value(ItemKind::Mug, Material::Wood, 1), 4);
        // 18 (weapon base) * 260 (metal 2.6x) * 350 (masterwork) / 10_000 = 163.
        assert_eq!(item_value(ItemKind::Weapon, Material::Metal, 4), 163);
    }

    // --- enum <-> label round trip ------------------------------------------------

    #[test]
    fn material_label_round_trips_and_matches_serde() {
        for &material in Material::ALL {
            let label = material.as_str();
            assert_eq!(Material::from_str_label(label), Some(material));
            assert_eq!(
                serde_json::to_value(material).unwrap(),
                serde_json::json!(label)
            );
        }
        assert_eq!(Material::from_str_label("unobtainium"), None);
    }

    #[test]
    fn item_kind_label_round_trips_and_matches_serde() {
        for &kind in ItemKind::ALL {
            let label = kind.as_str();
            assert_eq!(ItemKind::from_str_label(label), Some(kind));
            assert_eq!(
                serde_json::to_value(kind).unwrap(),
                serde_json::json!(label)
            );
        }
        assert_eq!(ItemKind::from_str_label("unobtainium"), None);
    }

    // --- Item wire-key serde --------------------------------------------------

    #[test]
    fn item_serializes_as_a_compact_wire_string() {
        let item = Item::new(ItemKind::Weapon, Material::Metal, 3);
        let json = serde_json::to_value(item).unwrap();
        assert_eq!(json, serde_json::json!("weapon:metal:3"));

        let back: Item = serde_json::from_value(json).unwrap();
        assert_eq!(back, item);
    }

    #[test]
    fn item_deserialize_rejects_malformed_keys() {
        for bad in [
            "weapon:metal",
            "weapon:metal:3:extra",
            "weapon:unknown:3",
            "nope",
        ] {
            let result: Result<Item, _> = serde_json::from_value(serde_json::json!(bad));
            assert!(result.is_err(), "expected {bad:?} to fail");
        }
    }

    #[test]
    fn item_new_clamps_quality() {
        let item = Item::new(ItemKind::Mug, Material::Clay, 200);
        assert_eq!(item.quality, MAX_QUALITY);
    }

    #[test]
    fn item_value_method_matches_free_fn() {
        let item = Item::new(ItemKind::Bowl, Material::Stone, 2);
        assert_eq!(item.value(), item_value(ItemKind::Bowl, Material::Stone, 2));
    }

    // --- BTreeMap<Item, u32> store: add/remove + deterministic map serialization ---

    #[test]
    fn add_item_accumulates_counts() {
        let mut store = BTreeMap::new();
        let mug = Item::new(ItemKind::Mug, Material::Wood, 1);
        add_item(&mut store, mug, 3);
        add_item(&mut store, mug, 2);
        assert_eq!(store.get(&mug), Some(&5));
    }

    #[test]
    fn add_item_zero_count_is_a_no_op() {
        let mut store: BTreeMap<Item, u32> = BTreeMap::new();
        add_item(&mut store, Item::new(ItemKind::Mug, Material::Wood, 1), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn remove_item_fails_when_insufficient_and_leaves_store_untouched() {
        let mut store = BTreeMap::new();
        let mug = Item::new(ItemKind::Mug, Material::Wood, 1);
        add_item(&mut store, mug, 2);

        assert!(!remove_item(&mut store, mug, 3));
        assert_eq!(store.get(&mug), Some(&2), "store untouched on failure");
    }

    #[test]
    fn remove_item_succeeds_and_drops_the_entry_at_zero() {
        let mut store = BTreeMap::new();
        let mug = Item::new(ItemKind::Mug, Material::Wood, 1);
        add_item(&mut store, mug, 5);

        assert!(remove_item(&mut store, mug, 2));
        assert_eq!(store.get(&mug), Some(&3));

        assert!(remove_item(&mut store, mug, 3));
        assert!(!store.contains_key(&mug));
        assert!(store.is_empty(), "entry dropped entirely at zero");
    }

    #[test]
    fn remove_item_from_a_missing_item_fails_unless_count_is_zero() {
        let mut store: BTreeMap<Item, u32> = BTreeMap::new();
        let mug = Item::new(ItemKind::Mug, Material::Wood, 1);
        assert!(!remove_item(&mut store, mug, 1));
        assert!(remove_item(&mut store, mug, 0));
    }

    #[test]
    fn btreemap_store_iterates_in_stable_deterministic_order() {
        let mut store = BTreeMap::new();
        add_item(
            &mut store,
            Item::new(ItemKind::Weapon, Material::Metal, 2),
            1,
        );
        add_item(&mut store, Item::new(ItemKind::Mug, Material::Wood, 1), 1);
        add_item(&mut store, Item::new(ItemKind::Mug, Material::Stone, 1), 1);

        let order_a: Vec<Item> = store.keys().copied().collect();
        // Rebuild from scratch in a different insertion order — BTreeMap sorts by Ord,
        // so iteration order is independent of insertion order.
        let mut store_b = BTreeMap::new();
        add_item(
            &mut store_b,
            Item::new(ItemKind::Mug, Material::Stone, 1),
            1,
        );
        add_item(
            &mut store_b,
            Item::new(ItemKind::Weapon, Material::Metal, 2),
            1,
        );
        add_item(&mut store_b, Item::new(ItemKind::Mug, Material::Wood, 1), 1);
        let order_b: Vec<Item> = store_b.keys().copied().collect();

        assert_eq!(order_a, order_b);
        // Ord is (kind, material, quality): Mug < Weapon, and within Mug, Wood < Stone
        // (declaration order of the `Material` enum variants).
        assert_eq!(
            order_a,
            vec![
                Item::new(ItemKind::Mug, Material::Wood, 1),
                Item::new(ItemKind::Mug, Material::Stone, 1),
                Item::new(ItemKind::Weapon, Material::Metal, 2),
            ]
        );
    }

    #[test]
    fn item_store_round_trips_through_json_as_a_plain_object() {
        let mut store = BTreeMap::new();
        add_item(
            &mut store,
            Item::new(ItemKind::Weapon, Material::Metal, 3),
            2,
        );
        add_item(&mut store, Item::new(ItemKind::Mug, Material::Wood, 1), 5);

        let json = serde_json::to_value(&store).unwrap();
        assert_eq!(json["weapon:metal:3"], serde_json::json!(2));
        assert_eq!(json["mug:wood:1"], serde_json::json!(5));

        let back: BTreeMap<Item, u32> = serde_json::from_value(json).unwrap();
        assert_eq!(back, store);
    }
}
