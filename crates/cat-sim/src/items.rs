//! DF-scale cat-themed item/material economy (P19).
//!
//! Per `docs/migration/specs/p19-items-materials-trade.md`: a compact `ItemKind ×
//! Material` model gives DF-like breadth ("a wooden mug OR a stone mug") without an
//! exploding item list. The finite item ledger adds stable unit identity, weight, and
//! condition alongside the fast aggregate survival [`crate::entities::Resources`]
//! store. Workshops create real units, truthful work wears functional equipment,
//! broken units remain physical, and staffed workshops can repair them.

use std::collections::BTreeMap;

use std::ops::Deref;

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
/// Legacy `BTreeMap<Item, u32>` saves used these compact strings as JSON object keys;
/// [`ItemStore`] still accepts that representation during migration.
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

/// Stable unit weight in grams. The three factors are all integer percentages, so
/// snapshots, trader limits, and deterministic twins never depend on floating-point
/// rounding. Better workmanship trims waste without changing the material identity.
#[must_use]
pub fn item_weight_grams(item: Item) -> u32 {
    let kind_grams = match item.kind {
        ItemKind::Mug => 400_u64,
        ItemKind::Bowl => 500,
        ItemKind::Furniture => 8_000,
        ItemKind::Tool => 2_500,
        ItemKind::Weapon => 3_000,
        ItemKind::Armor => 6_000,
        ItemKind::Clothing => 800,
        ItemKind::Trinket => 300,
        ItemKind::Toy => 500,
    };
    let material_pct = match item.material {
        Material::Fibre => 40_u64,
        Material::Leather => 70,
        Material::Gem => 80,
        Material::Bone => 90,
        Material::Wood => 100,
        Material::Clay => 120,
        Material::Metal => 150,
        Material::Stone => 180,
    };
    let quality_pct = [110_u64, 105, 100, 95, 90][item.quality.min(MAX_QUALITY) as usize];
    ((kind_grams * material_pct * quality_pct) / 10_000)
        .max(1)
        .try_into()
        .unwrap_or(u32::MAX)
}

/// Base maximum durability before workshop research. Material and quality both
/// matter: a fine metal tool survives far more real work than a crude wooden one.
#[must_use]
pub fn item_base_max_durability(item: Item) -> u32 {
    let kind = match item.kind {
        ItemKind::Mug | ItemKind::Bowl => 8_u64,
        ItemKind::Furniture => 18,
        ItemKind::Tool => 6,
        ItemKind::Weapon => 8,
        ItemKind::Armor => 10,
        ItemKind::Clothing => 7,
        ItemKind::Trinket => 6,
        ItemKind::Toy => 5,
    };
    let material_pct = match item.material {
        Material::Fibre => 55_u64,
        Material::Clay => 70,
        Material::Wood => 100,
        Material::Leather => 120,
        Material::Bone => 130,
        Material::Stone => 155,
        Material::Metal => 220,
        Material::Gem => 180,
    };
    let quality_pct = [75_u64, 100, 125, 150, 200][item.quality.min(MAX_QUALITY) as usize];
    ((kind * material_pct * quality_pct) / 10_000)
        .max(1)
        .try_into()
        .unwrap_or(u32::MAX)
}

/// Apply the owning workshop's resolved durability research to one item's stable
/// base. Flooring is deliberate: research must cross a whole-use boundary before it
/// grants another use, and identical ownership always produces identical equipment.
#[must_use]
pub fn item_max_durability(item: Item, durability_mult: f64) -> u32 {
    let multiplier = if durability_mult.is_finite() {
        durability_mult.max(0.0)
    } else {
        1.0
    };
    (f64::from(item_base_max_durability(item)) * multiplier)
        .floor()
        .max(1.0) as u32
}

/// Research/repair owner for an item. This is the exact existing production bench
/// that makes or maintains the modeled material; no separate maintenance building is
/// invented for this slice.
#[must_use]
pub const fn item_workshop_id(item: Item) -> &'static str {
    match item.kind {
        ItemKind::Tool => "woodworking",
        ItemKind::Weapon | ItemKind::Armor => "smithy",
        _ => match item.material {
            Material::Wood => "woodworking",
            Material::Stone | Material::Clay | Material::Gem | Material::Bone => "stone_prep",
            Material::Metal => "smithy",
            Material::Fibre => "clothier",
            Material::Leather => "tannery",
        },
    }
}

/// One physical item with stable colony-local identity and independently persisted
/// condition. Broken items remain in this ledger with `durability == 0`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemInstance {
    pub id: String,
    pub item: Item,
    pub durability: u32,
    pub max_durability: u32,
}

impl ItemInstance {
    #[must_use]
    pub const fn is_broken(&self) -> bool {
        self.durability == 0
    }

    #[must_use]
    pub const fn is_pristine(&self) -> bool {
        self.durability == self.max_durability
    }
}

/// Per-colony finite item inventory. `stacks` remains the deterministic aggregate
/// view used by existing recipes and tests; `instances` is the authoritative unit
/// ledger. Deref preserves the established read-only `BTreeMap<Item, u32>` API.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ItemStore {
    stacks: BTreeMap<Item, u32>,
    instances: BTreeMap<String, ItemInstance>,
    next_serial: u64,
}

impl Deref for ItemStore {
    type Target = BTreeMap<Item, u32>;

    fn deref(&self) -> &Self::Target {
        &self.stacks
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemStoreRef<'a> {
    next_serial: u64,
    instances: Vec<&'a ItemInstance>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ItemStoreOwned {
    #[serde(default)]
    next_serial: u64,
    #[serde(default)]
    instances: Vec<ItemInstance>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ItemStoreWire {
    Current(ItemStoreOwned),
    Legacy(BTreeMap<Item, u32>),
}

impl Serialize for ItemStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ItemStoreRef {
            next_serial: self.next_serial,
            instances: self.instances.values().collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ItemStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match ItemStoreWire::deserialize(deserializer)? {
            ItemStoreWire::Current(state) => {
                let mut store = Self {
                    next_serial: state.next_serial,
                    ..Self::default()
                };
                for mut instance in state.instances {
                    instance.max_durability = instance.max_durability.max(1);
                    instance.durability = instance.durability.min(instance.max_durability);
                    store.next_serial = store.next_serial.max(serial_from_id(&instance.id));
                    store.instances.insert(instance.id.clone(), instance);
                }
                store.rebuild_stacks();
                Ok(store)
            }
            ItemStoreWire::Legacy(stacks) => {
                let mut store = Self::default();
                for (item, count) in stacks {
                    store.add(item, count, 1.0);
                }
                Ok(store)
            }
        }
    }
}

fn serial_from_id(id: &str) -> u64 {
    id.strip_prefix("item-")
        .and_then(|serial| serial.parse().ok())
        .unwrap_or(0)
}

impl ItemStore {
    fn rebuild_stacks(&mut self) {
        self.stacks.clear();
        for instance in self.instances.values() {
            let entry = self.stacks.entry(instance.item).or_insert(0);
            *entry = entry.saturating_add(1);
        }
    }

    /// Add newly crafted units at full condition, with the workshop's resolved
    /// durability research captured in their maximum durability.
    pub fn add(&mut self, item: Item, count: u32, durability_mult: f64) {
        for _ in 0..count {
            self.next_serial = self.next_serial.saturating_add(1);
            let id = format!("item-{:016}", self.next_serial);
            let max_durability = item_max_durability(item, durability_mult);
            self.instances.insert(
                id.clone(),
                ItemInstance {
                    id,
                    item,
                    durability: max_durability,
                    max_durability,
                },
            );
        }
        if count > 0 {
            let entry = self.stacks.entry(item).or_insert(0);
            *entry = entry.saturating_add(count);
        }
    }

    /// Remove deterministic units regardless of condition (legacy aggregate API).
    pub fn remove(&mut self, item: Item, count: u32) -> bool {
        if self.stacks.get(&item).copied().unwrap_or(0) < count {
            return false;
        }
        let ids = self
            .instances
            .iter()
            .filter(|(_, instance)| instance.item == item)
            .map(|(id, _)| id.clone())
            .take(count as usize)
            .collect::<Vec<_>>();
        for id in ids {
            self.instances.remove(&id);
        }
        self.decrement_stack(item, count);
        true
    }

    /// Remove only pristine units for trader sale. Damaged/broken goods remain
    /// physical and must be repaired before a caravan accepts them.
    pub fn remove_pristine(&mut self, item: Item, count: u32) -> bool {
        let ids = self
            .instances
            .iter()
            .filter(|(_, instance)| instance.item == item && instance.is_pristine())
            .map(|(id, _)| id.clone())
            .take(count as usize)
            .collect::<Vec<_>>();
        if ids.len() != count as usize {
            return false;
        }
        for id in ids {
            self.instances.remove(&id);
        }
        self.decrement_stack(item, count);
        true
    }

    fn decrement_stack(&mut self, item: Item, count: u32) {
        let remaining = self
            .stacks
            .get(&item)
            .copied()
            .unwrap_or(0)
            .saturating_sub(count);
        if remaining == 0 {
            self.stacks.remove(&item);
        } else {
            self.stacks.insert(item, remaining);
        }
    }

    pub fn instances(&self) -> impl Iterator<Item = &ItemInstance> {
        self.instances.values()
    }

    #[must_use]
    pub fn instance(&self, id: &str) -> Option<&ItemInstance> {
        self.instances.get(id)
    }

    #[must_use]
    pub fn count_kind(&self, kind: ItemKind) -> u32 {
        self.instances
            .values()
            .filter(|instance| instance.item.kind == kind)
            .count() as u32
    }

    #[must_use]
    pub fn usable_count(&self, kind: ItemKind) -> u32 {
        self.instances
            .values()
            .filter(|instance| instance.item.kind == kind && !instance.is_broken())
            .count() as u32
    }

    #[must_use]
    pub fn pristine_count(&self, item: Item) -> u32 {
        self.instances
            .values()
            .filter(|instance| instance.item == item && instance.is_pristine())
            .count() as u32
    }

    /// Apply one point of truthful use to up to `count` intact units, in stable id
    /// order. Returns the ids that crossed the broken boundary during this use.
    pub fn wear(&mut self, kind: ItemKind, count: u32) -> Vec<String> {
        let ids = self
            .instances
            .iter()
            .filter(|(_, instance)| instance.item.kind == kind && !instance.is_broken())
            .map(|(id, _)| id.clone())
            .take(count as usize)
            .collect::<Vec<_>>();
        let mut broken = Vec::new();
        for id in ids {
            let instance = self.instances.get_mut(&id).expect("selected item exists");
            instance.durability = instance.durability.saturating_sub(1);
            if instance.is_broken() {
                broken.push(id);
            }
        }
        broken
    }

    /// Restore a damaged unit, updating its maximum to current workshop research.
    pub fn repair(&mut self, id: &str, durability_mult: f64) -> bool {
        let Some(instance) = self.instances.get_mut(id) else {
            return false;
        };
        if instance.is_pristine() {
            return false;
        }
        instance.max_durability = item_max_durability(instance.item, durability_mult);
        instance.durability = instance.max_durability;
        true
    }
}

/// Adds `count` of `item` to `store` (saturating; a no-op for `count == 0`).
pub fn add_item(store: &mut ItemStore, item: Item, count: u32) {
    store.add(item, count, 1.0);
}

/// Removes `count` of `item` from `store`. Fails (returns `false`, store left
/// untouched) if `store` holds fewer than `count`. On success, drops the entry
/// entirely once its count reaches zero (keeps the map compact for
/// `BTreeMap::is_empty` / `skip_serializing_if` checks).
pub fn remove_item(store: &mut ItemStore, item: Item, count: u32) -> bool {
    store.remove(item, count)
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
        let mut store = ItemStore::default();
        let mug = Item::new(ItemKind::Mug, Material::Wood, 1);
        add_item(&mut store, mug, 3);
        add_item(&mut store, mug, 2);
        assert_eq!(store.get(&mug), Some(&5));
    }

    #[test]
    fn add_item_zero_count_is_a_no_op() {
        let mut store = ItemStore::default();
        add_item(&mut store, Item::new(ItemKind::Mug, Material::Wood, 1), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn remove_item_fails_when_insufficient_and_leaves_store_untouched() {
        let mut store = ItemStore::default();
        let mug = Item::new(ItemKind::Mug, Material::Wood, 1);
        add_item(&mut store, mug, 2);

        assert!(!remove_item(&mut store, mug, 3));
        assert_eq!(store.get(&mug), Some(&2), "store untouched on failure");
    }

    #[test]
    fn remove_item_succeeds_and_drops_the_entry_at_zero() {
        let mut store = ItemStore::default();
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
        let mut store = ItemStore::default();
        let mug = Item::new(ItemKind::Mug, Material::Wood, 1);
        assert!(!remove_item(&mut store, mug, 1));
        assert!(remove_item(&mut store, mug, 0));
    }

    #[test]
    fn btreemap_store_iterates_in_stable_deterministic_order() {
        let mut store = ItemStore::default();
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
        let mut store_b = ItemStore::default();
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
    fn item_store_round_trips_unit_identity_and_accepts_legacy_stack_json() {
        let mut store = ItemStore::default();
        add_item(
            &mut store,
            Item::new(ItemKind::Weapon, Material::Metal, 3),
            2,
        );
        add_item(&mut store, Item::new(ItemKind::Mug, Material::Wood, 1), 5);

        let json = serde_json::to_value(&store).unwrap();
        assert_eq!(json["nextSerial"], serde_json::json!(7));
        assert_eq!(json["instances"].as_array().unwrap().len(), 7);

        let back: ItemStore = serde_json::from_value(json).unwrap();
        assert_eq!(back, store);

        let legacy = serde_json::json!({"weapon:metal:3": 2, "mug:wood:1": 5});
        let migrated: ItemStore = serde_json::from_value(legacy).unwrap();
        assert_eq!(
            migrated.get(&Item::new(ItemKind::Weapon, Material::Metal, 3)),
            Some(&2)
        );
        assert_eq!(migrated.instances().count(), 7);
    }

    #[test]
    fn weight_durability_wear_break_and_repair_are_stable_per_unit() {
        let crude_wood = Item::new(ItemKind::Tool, Material::Wood, 0);
        let fine_metal = Item::new(ItemKind::Tool, Material::Metal, 2);
        assert!(item_weight_grams(fine_metal) > item_weight_grams(crude_wood));
        assert!(item_base_max_durability(fine_metal) > item_base_max_durability(crude_wood));

        let mut store = ItemStore::default();
        store.add(crude_wood, 1, 1.5);
        let id = store.instances().next().unwrap().id.clone();
        let researched_max = store.instance(&id).unwrap().max_durability;
        assert_eq!(
            researched_max,
            item_max_durability(crude_wood, 1.5),
            "research is captured in the unit maximum"
        );
        for _ in 0..researched_max {
            store.wear(ItemKind::Tool, 1);
        }
        assert!(store.instance(&id).unwrap().is_broken());
        assert_eq!(store.usable_count(ItemKind::Tool), 0);
        assert_eq!(store.get(&crude_wood), Some(&1), "broken remains physical");
        assert!(store.repair(&id, 1.5));
        assert!(store.instance(&id).unwrap().is_pristine());
    }

    #[test]
    fn every_kind_material_quality_has_nonzero_stable_weight_and_durability() {
        for &kind in ItemKind::ALL {
            for &material in Material::ALL {
                let mut previous_durability = 0;
                let mut previous_weight = u32::MAX;
                for quality in 0..=MAX_QUALITY {
                    let item = Item::new(kind, material, quality);
                    let weight = item_weight_grams(item);
                    let durability = item_base_max_durability(item);
                    assert!(weight > 0, "{item:?}");
                    assert!(durability > 0, "{item:?}");
                    assert!(durability >= previous_durability, "{item:?}");
                    assert!(weight <= previous_weight, "{item:?}");
                    previous_durability = durability;
                    previous_weight = weight;
                }
            }
        }
    }
}
