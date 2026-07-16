//! DF-scale cat-themed item/material economy (P19).
//!
//! Per `docs/migration/specs/p19-items-materials-trade.md`: a compact `ItemKind ×
//! Material` model gives DF-like breadth ("a wooden mug OR a stone mug") without an
//! exploding item list. The finite item ledger adds stable unit identity, weight, and
//! condition alongside the fast aggregate survival [`crate::entities::Resources`]
//! store. Workshops create real units, truthful work wears functional equipment,
//! broken units remain physical, and staffed workshops can repair them.

use std::collections::{BTreeMap, BTreeSet};

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
    /// Clay, fired or sun-dried. Cheap, common for mugs/bowls.
    Clay,
    /// Sand, used for cast forms and simple glassy decorations.
    Sand,
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
        Self::Sand,
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
            Self::Sand => "sand",
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
            Self::Sand => 60,
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
        Material::Sand => 110,
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
        Material::Sand => 60,
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
        ItemKind::Tool if matches!(item.material, Material::Metal) => "smithy",
        ItemKind::Tool => "woodworking",
        ItemKind::Weapon | ItemKind::Armor => "smithy",
        _ => match item.material {
            Material::Wood => "woodworking",
            Material::Stone | Material::Clay | Material::Sand | Material::Gem | Material::Bone => {
                "stone_prep"
            }
            Material::Metal => "smithy",
            Material::Fibre => "clothier",
            Material::Leather => "tannery",
        },
    }
}

/// Exact physical compartment occupied by a finite item at a production station.
/// The four stages deliberately mirror the visible station inspector and persisted
/// cargo route rather than collapsing all workshop ownership into one abstract bin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StationCompartment {
    Inbound,
    LocalInput,
    LocalOutput,
    Outbound,
}

/// One authoritative physical location for one stable item identity.
///
/// `LegacyTreasury` is a migration quarantine for old saves whose aggregate
/// functional-equipment counters predate physical pile identity. New production
/// uses the station/carrier/stockpile states, while equipment and traders keep the
/// exact same item id as ownership changes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ItemLocation {
    #[default]
    LegacyTreasury,
    Stockpile {
        stockpile_id: String,
    },
    Station {
        building_id: String,
        compartment: StationCompartment,
    },
    Carrier {
        cat_id: String,
    },
    Equipped {
        cat_id: String,
    },
    Trader {
        trader_id: String,
    },
    /// Loaded into an inter-village caravan. The instance is serialized inside
    /// the caravan escrow rather than either colony's item ledger while moving.
    Caravan {
        caravan_id: String,
    },
}

const fn default_credited() -> bool {
    true
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
    /// Exact current physical location. Missing old-save fields migrate into the
    /// compatibility treasury and are assigned deterministically later.
    #[serde(default)]
    pub location: ItemLocation,
    /// Whether this unit has crossed its final-delivery boundary and therefore
    /// contributes to the stable scalar compatibility projection. Station output
    /// and its outbound carrier are real but deliberately uncredited.
    #[serde(default = "default_credited")]
    pub credited: bool,
    /// True only for simulation-issued equipment. Signed player equips clear this
    /// provenance so automation cannot silently reclaim a manual loadout.
    #[serde(default)]
    pub auto_issued: bool,
    /// Exact queued/active job whose duration sampled this item as its contributor.
    /// Persisting the link prevents equip timing from granting free speed or wearing
    /// a different late-equipped tool after restart.
    #[serde(default)]
    pub active_job_id: Option<String>,
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
        self.add_at(
            item,
            count,
            durability_mult,
            ItemLocation::LegacyTreasury,
            true,
        );
    }

    /// Create new stable identities at one exact physical location. Returns ids in
    /// creation order so a station can attach the same units to later cargo.
    pub fn add_at(
        &mut self,
        item: Item,
        count: u32,
        durability_mult: f64,
        location: ItemLocation,
        credited: bool,
    ) -> Vec<String> {
        let mut ids = Vec::with_capacity(count as usize);
        for _ in 0..count {
            self.next_serial = self.next_serial.saturating_add(1);
            let id = format!("item-{:016}", self.next_serial);
            let max_durability = item_max_durability(item, durability_mult);
            self.instances.insert(
                id.clone(),
                ItemInstance {
                    id: id.clone(),
                    item,
                    durability: max_durability,
                    max_durability,
                    location: location.clone(),
                    credited,
                    auto_issued: false,
                    active_job_id: None,
                },
            );
            ids.push(id);
        }
        if count > 0 {
            let entry = self.stacks.entry(item).or_insert(0);
            *entry = entry.saturating_add(count);
        }
        ids
    }

    /// Remove deterministic units regardless of condition (legacy aggregate API).
    pub fn remove(&mut self, item: Item, count: u32) -> bool {
        let ids = self
            .instances
            .iter()
            .filter(|(_, instance)| {
                instance.item == item
                    && matches!(
                        instance.location,
                        ItemLocation::LegacyTreasury | ItemLocation::Stockpile { .. }
                    )
            })
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

    /// Remove only pristine units for trader sale. Damaged/broken goods remain
    /// physical and must be repaired before a caravan accepts them.
    pub fn remove_pristine(&mut self, item: Item, count: u32) -> bool {
        let ids = self
            .instances
            .iter()
            .filter(|(_, instance)| {
                instance.item == item && instance.is_pristine() && instance.active_job_id.is_none()
            })
            .filter(|(_, instance)| {
                matches!(
                    instance.location,
                    ItemLocation::LegacyTreasury | ItemLocation::Stockpile { .. }
                )
            })
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

    /// Atomically move exact pristine unit identities into another finite ledger.
    /// This is the visiting-trader handoff: no unit is reconstructed from an aggregate
    /// count, so a signed sale cannot duplicate identity or reset condition.
    pub fn transfer_pristine_to(&mut self, destination: &mut Self, item: Item, count: u32) -> bool {
        self.transfer_pristine_to_at(destination, item, count, ItemLocation::LegacyTreasury)
    }

    /// Atomically transfer exact pristine identities and assign their new physical
    /// owner/location. Only genuinely stored units are eligible for sale.
    pub fn transfer_pristine_to_at(
        &mut self,
        destination: &mut Self,
        item: Item,
        count: u32,
        destination_location: ItemLocation,
    ) -> bool {
        if count == 0 {
            return true;
        }
        let ids = self
            .instances
            .iter()
            .filter(|(_, instance)| {
                instance.item == item
                    && instance.is_pristine()
                    && instance.active_job_id.is_none()
                    && matches!(
                        instance.location,
                        ItemLocation::LegacyTreasury | ItemLocation::Stockpile { .. }
                    )
            })
            .map(|(id, _)| id.clone())
            .take(count as usize)
            .collect::<Vec<_>>();
        if ids.len() != count as usize
            || ids.iter().any(|id| destination.instances.contains_key(id))
        {
            return false;
        }
        for id in ids {
            let mut instance = self
                .instances
                .remove(&id)
                .expect("selected source unit still exists");
            instance.location = destination_location.clone();
            instance.credited = false;
            destination.next_serial = destination.next_serial.max(serial_from_id(&id));
            destination.instances.insert(id, instance);
        }
        self.decrement_stack(item, count);
        let destination_count = destination.stacks.entry(item).or_insert(0);
        *destination_count = destination_count.saturating_add(count);
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
    pub fn instance_mut(&mut self, id: &str) -> Option<&mut ItemInstance> {
        self.instances.get_mut(id)
    }

    /// Atomically remove exact identities from this ledger without changing
    /// their condition. Callers validate physical eligibility before loading;
    /// duplicate or missing ids leave the store untouched.
    pub fn take_exact(&mut self, ids: &[String]) -> Option<Vec<ItemInstance>> {
        let unique = ids.iter().collect::<BTreeSet<_>>();
        if unique.len() != ids.len() || ids.iter().any(|id| !self.instances.contains_key(id)) {
            return None;
        }
        let instances = ids
            .iter()
            .map(|id| self.instances.remove(id).expect("preflighted exact item"))
            .collect::<Vec<_>>();
        self.rebuild_stacks();
        Some(instances)
    }

    /// Atomically insert transferred identities. A collision is rejected before
    /// any mutation, which makes a corrupt/legacy caravan safe to retry.
    pub fn insert_exact(&mut self, instances: Vec<ItemInstance>) -> Result<(), Vec<ItemInstance>> {
        let unique = instances
            .iter()
            .map(|instance| instance.id.as_str())
            .collect::<BTreeSet<_>>();
        if unique.len() != instances.len()
            || instances
                .iter()
                .any(|instance| self.instances.contains_key(&instance.id))
        {
            return Err(instances);
        }
        for instance in instances {
            self.next_serial = self.next_serial.max(serial_from_id(&instance.id));
            self.instances.insert(instance.id.clone(), instance);
        }
        self.rebuild_stacks();
        Ok(())
    }

    #[must_use]
    pub fn can_insert_exact(&self, instances: &[ItemInstance]) -> bool {
        let unique = instances
            .iter()
            .map(|instance| instance.id.as_str())
            .collect::<BTreeSet<_>>();
        unique.len() == instances.len()
            && instances
                .iter()
                .all(|instance| !self.instances.contains_key(&instance.id))
    }

    /// Move an existing identity without reconstructing it or changing condition.
    pub fn relocate(&mut self, id: &str, location: ItemLocation) -> bool {
        let Some(instance) = self.instances.get_mut(id) else {
            return false;
        };
        instance.location = location;
        true
    }

    pub fn set_auto_issued(&mut self, id: &str, auto_issued: bool) -> bool {
        let Some(instance) = self.instances.get_mut(id) else {
            return false;
        };
        instance.auto_issued = auto_issued;
        true
    }

    pub fn reserve_for_job(&mut self, id: &str, job_id: &str) -> bool {
        let Some(instance) = self.instances.get_mut(id) else {
            return false;
        };
        if instance.active_job_id.is_some() || instance.is_broken() {
            return false;
        }
        instance.active_job_id = Some(job_id.to_owned());
        true
    }

    #[must_use]
    pub fn item_id_for_job(&self, job_id: &str) -> Option<&str> {
        self.instances.values().find_map(|instance| {
            (instance.active_job_id.as_deref() == Some(job_id)).then_some(instance.id.as_str())
        })
    }

    pub fn release_job(&mut self, job_id: &str, wear: bool) -> Option<String> {
        let id = self.item_id_for_job(job_id)?.to_owned();
        let instance = self.instances.get_mut(&id)?;
        if wear && instance.durability > 0 {
            instance.durability -= 1;
        }
        instance.active_job_id = None;
        Some(id)
    }

    /// Mark final delivery without minting a second inventory entry.
    pub fn credit_at(&mut self, id: &str, location: ItemLocation) -> bool {
        let Some(instance) = self.instances.get_mut(id) else {
            return false;
        };
        instance.location = location;
        instance.credited = true;
        true
    }

    pub fn ids_at(&self, kind: ItemKind, location: &ItemLocation) -> impl Iterator<Item = &str> {
        self.instances.values().filter_map(move |instance| {
            (instance.item.kind == kind && &instance.location == location)
                .then_some(instance.id.as_str())
        })
    }

    #[must_use]
    pub fn credited_count(&self, kind: ItemKind) -> u32 {
        self.instances
            .values()
            .filter(|instance| instance.item.kind == kind && instance.credited)
            .count() as u32
    }

    #[must_use]
    pub fn equipped_id(&self, cat_id: &str, kind: ItemKind) -> Option<&str> {
        self.instances.values().find_map(|instance| {
            (instance.item.kind == kind
                && instance.location
                    == (ItemLocation::Equipped {
                        cat_id: cat_id.to_owned(),
                    }))
            .then_some(instance.id.as_str())
        })
    }

    #[must_use]
    pub fn first_stored_usable_id(&self, kind: ItemKind) -> Option<&str> {
        self.instances.values().find_map(|instance| {
            (instance.item.kind == kind
                && !instance.is_broken()
                && matches!(
                    instance.location,
                    ItemLocation::LegacyTreasury | ItemLocation::Stockpile { .. }
                ))
            .then_some(instance.id.as_str())
        })
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
            .filter(|instance| {
                instance.item == item
                    && instance.is_pristine()
                    && instance.active_job_id.is_none()
                    && matches!(
                        instance.location,
                        ItemLocation::LegacyTreasury | ItemLocation::Stockpile { .. }
                    )
            })
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

    /// Wear one exact stable identity. Returns whether this use crossed into the
    /// broken state; missing/already-broken ids are a no-op.
    pub fn wear_id(&mut self, id: &str) -> bool {
        let Some(instance) = self.instances.get_mut(id) else {
            return false;
        };
        if instance.is_broken() {
            return false;
        }
        instance.durability = instance.durability.saturating_sub(1);
        instance.is_broken()
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

    #[test]
    fn pristine_transfer_moves_exact_units_without_duplication() {
        let item = Item::new(ItemKind::Mug, Material::Wood, 2);
        let mut source = ItemStore::default();
        source.add(item, 3, 1.0);
        let original_ids = source
            .instances()
            .map(|instance| instance.id.clone())
            .collect::<Vec<_>>();
        let mut destination = ItemStore::default();

        assert!(source.transfer_pristine_to(&mut destination, item, 2));
        assert_eq!(source.get(&item), Some(&1));
        assert_eq!(destination.get(&item), Some(&2));
        let moved_ids = destination
            .instances()
            .map(|instance| instance.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(moved_ids, original_ids[..2]);
        assert!(
            source
                .instances()
                .all(|instance| !moved_ids.contains(&instance.id))
        );
    }

    #[test]
    fn old_instance_json_defaults_to_credited_legacy_treasury() {
        let old = serde_json::json!({
            "nextSerial": 1,
            "instances": [{
                "id": "item-0000000000000001",
                "item": "tool:wood:1",
                "durability": 6,
                "maxDurability": 6
            }]
        });
        let store: ItemStore = serde_json::from_value(old).unwrap();
        let instance = store.instances().next().unwrap();
        assert_eq!(instance.location, ItemLocation::LegacyTreasury);
        assert!(instance.credited);
        assert!(!instance.auto_issued);
        assert_eq!(instance.active_job_id, None);
    }

    #[test]
    fn exact_location_moves_preserve_identity_and_condition() {
        let item = Item::new(ItemKind::Weapon, Material::Metal, 1);
        let mut store = ItemStore::default();
        let id = store
            .add_at(
                item,
                1,
                1.0,
                ItemLocation::Station {
                    building_id: "smithy-1".to_owned(),
                    compartment: StationCompartment::LocalOutput,
                },
                false,
            )
            .pop()
            .unwrap();
        let durability = store.instance(&id).unwrap().durability;
        assert!(store.relocate(
            &id,
            ItemLocation::Carrier {
                cat_id: "smith".to_owned()
            }
        ));
        assert!(store.credit_at(
            &id,
            ItemLocation::Stockpile {
                stockpile_id: "armory".to_owned()
            }
        ));
        let instance = store.instance(&id).unwrap();
        assert_eq!(instance.id, id);
        assert_eq!(instance.durability, durability);
        assert!(instance.credited);
    }

    #[test]
    fn removal_and_sale_never_consume_equipped_carried_or_station_units() {
        let item = Item::new(ItemKind::Tool, Material::Wood, 1);
        let mut source = ItemStore::default();
        for location in [
            ItemLocation::Equipped {
                cat_id: "worker".to_owned(),
            },
            ItemLocation::Carrier {
                cat_id: "hauler".to_owned(),
            },
            ItemLocation::Station {
                building_id: "woodworking".to_owned(),
                compartment: StationCompartment::LocalOutput,
            },
        ] {
            source.add_at(item, 1, 1.0, location, true);
        }
        assert!(!source.remove(item, 1));
        assert!(!source.remove_pristine(item, 1));
        let before = source.clone();
        let mut trader = ItemStore::default();
        assert!(!source.transfer_pristine_to_at(
            &mut trader,
            item,
            1,
            ItemLocation::Trader {
                trader_id: "wagon".to_owned()
            }
        ));
        assert_eq!(source, before);
        assert!(trader.is_empty());
    }
}
