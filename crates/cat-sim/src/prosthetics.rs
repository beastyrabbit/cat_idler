//! Deterministic prosthetic item lifecycle specified by
//! `docs/leader-ai-overhaul/cats-and-care.md`.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    acquired_traits::AcquiredTraitState,
    anatomy::{BodyPart, BodyPartCondition, CatAnatomy},
    planner_core::PlannerId,
};

pub const PROSTHETIC_SCHEMA_VERSION: u32 = 1;
pub const WOODEN_DURABILITY_MINUTES: u64 = 360 * 60;
pub const METAL_DURABILITY_MINUTES: u64 = 1_080 * 60;
pub const REHABILITATION_BONUS_BASIS_POINTS: u16 = 200;
pub const PROSTHETIC_FUNCTION_CAP_BASIS_POINTS: u16 = 9_000;

/// One stable, losslessly encoded identity retained across every lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProstheticId(PlannerId);

impl ProstheticId {
    #[must_use]
    pub fn derive(
        colony_id: &str,
        part: BodyPart,
        material: ProstheticMaterial,
        production_serial: u64,
    ) -> Self {
        let serial = production_serial.to_string();
        Self(PlannerId::derive(
            "prosthetic",
            [colony_id, part.stable_id(), material.stable_id(), &serial],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn is_valid(&self) -> bool {
        self.as_str().starts_with("planner:v1|10:prosthetic|")
    }
}

impl fmt::Display for ProstheticId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProstheticMaterial {
    Wooden,
    Metal,
}

impl ProstheticMaterial {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Wooden => "wooden",
            Self::Metal => "metal",
        }
    }

    #[must_use]
    pub const fn restoration_basis_points(self) -> u16 {
        match self {
            Self::Wooden => 5_000,
            Self::Metal => 7_500,
        }
    }

    #[must_use]
    pub const fn max_durability_minutes(self) -> u64 {
        match self {
            Self::Wooden => WOODEN_DURABILITY_MINUTES,
            Self::Metal => METAL_DURABILITY_MINUTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FitSiteKind {
    Treatment,
    Workshop,
}

/// Authoritative physical state of one finite prosthetic item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ProstheticLocation {
    Inventory {
        colony_id: String,
    },
    FittingReservation {
        colony_id: String,
        reservation_id: String,
        cat_id: String,
        fitter_id: String,
        site_id: String,
        site_kind: FitSiteKind,
    },
    Fitted {
        colony_id: String,
        cat_id: String,
    },
    RepairReservation {
        colony_id: String,
        reservation_id: String,
        workshop_id: String,
    },
    TradeCargo {
        colony_id: String,
        caravan_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProstheticItem {
    pub id: ProstheticId,
    pub material: ProstheticMaterial,
    pub part: BodyPart,
    pub durability_minutes: u64,
    #[serde(default)]
    pub rehabilitation_stages: u8,
    pub location: ProstheticLocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FittedSlot {
    cat_id: String,
    part: BodyPart,
    item_id: ProstheticId,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProstheticLedger {
    items: BTreeMap<ProstheticId, ProstheticItem>,
    fitted_slots: BTreeMap<(String, BodyPart), ProstheticId>,
}

fn default_schema_version() -> u32 {
    PROSTHETIC_SCHEMA_VERSION
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LedgerRef<'a> {
    schema_version: u32,
    items: Vec<&'a ProstheticItem>,
    fitted_slots: Vec<FittedSlot>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LedgerOwned {
    #[serde(default = "default_schema_version")]
    schema_version: u32,
    #[serde(default)]
    items: Vec<ProstheticItem>,
    #[serde(default)]
    fitted_slots: Vec<FittedSlot>,
}

impl Serialize for ProstheticLedger {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut items = self.items.values().collect::<Vec<_>>();
        items.sort_by(|left, right| {
            left.part
                .cmp(&right.part)
                .then_with(|| left.id.cmp(&right.id))
        });
        LedgerRef {
            schema_version: PROSTHETIC_SCHEMA_VERSION,
            items,
            fitted_slots: self
                .fitted_slots
                .iter()
                .map(|((cat_id, part), item_id)| FittedSlot {
                    cat_id: cat_id.clone(),
                    part: *part,
                    item_id: item_id.clone(),
                })
                .collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProstheticLedger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = LedgerOwned::deserialize(deserializer)?;
        if wire.schema_version != PROSTHETIC_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(
                "unsupported prosthetic schema version",
            ));
        }
        let mut ledger = Self::default();
        for item in wire.items {
            let id = item.id.clone();
            if ledger.items.insert(id, item).is_some() {
                return Err(serde::de::Error::custom("duplicate prosthetic item id"));
            }
        }
        for slot in wire.fitted_slots {
            if ledger
                .fitted_slots
                .insert((slot.cat_id, slot.part), slot.item_id)
                .is_some()
            {
                return Err(serde::de::Error::custom("duplicate fitted anatomy slot"));
            }
        }
        ledger.validate().map_err(serde::de::Error::custom)?;
        Ok(ledger)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FitAuthorization<'a> {
    pub colony_id: &'a str,
    pub cat_id: &'a str,
    pub part: BodyPart,
    pub reservation_id: &'a str,
    pub fitter_id: &'a str,
    pub fitter_capable: bool,
    pub patient_consents: bool,
    pub site_id: &'a str,
    pub site_kind: FitSiteKind,
    pub site_reachable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairAuthorization<'a> {
    pub colony_id: &'a str,
    pub reservation_id: &'a str,
    pub workshop_id: &'a str,
    pub workshop_reachable: bool,
    pub finite_inputs_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProstheticError {
    DuplicateId,
    InvalidStableId,
    UnknownItem,
    WrongOwner,
    WrongPart,
    PartNotMissing,
    Broken,
    SlotOccupied,
    PatientRefused,
    FitterIncapable,
    SiteUnreachable,
    InvalidAuthorization,
    NotInInventory,
    NotReserved,
    NotFitted,
    NotBroken,
    FiniteInputsNotAuthorized,
}

impl ProstheticLedger {
    pub fn register(
        &mut self,
        id: ProstheticId,
        material: ProstheticMaterial,
        part: BodyPart,
        colony_id: impl Into<String>,
    ) -> Result<(), ProstheticError> {
        if !id.is_valid() {
            return Err(ProstheticError::InvalidStableId);
        }
        if self.items.contains_key(&id) {
            return Err(ProstheticError::DuplicateId);
        }
        let colony_id = colony_id.into();
        if colony_id.is_empty() {
            return Err(ProstheticError::InvalidAuthorization);
        }
        self.items.insert(
            id.clone(),
            ProstheticItem {
                id,
                material,
                part,
                durability_minutes: material.max_durability_minutes(),
                rehabilitation_stages: 0,
                location: ProstheticLocation::Inventory { colony_id },
            },
        );
        Ok(())
    }

    pub fn begin_fitting(
        &mut self,
        id: &ProstheticId,
        anatomy: &CatAnatomy,
        authorization: FitAuthorization<'_>,
    ) -> Result<(), ProstheticError> {
        if !authorization.patient_consents {
            return Err(ProstheticError::PatientRefused);
        }
        if !authorization.fitter_capable {
            return Err(ProstheticError::FitterIncapable);
        }
        if !authorization.site_reachable {
            return Err(ProstheticError::SiteUnreachable);
        }
        if [
            authorization.colony_id,
            authorization.cat_id,
            authorization.reservation_id,
            authorization.fitter_id,
            authorization.site_id,
        ]
        .contains(&"")
        {
            return Err(ProstheticError::InvalidAuthorization);
        }
        let item = self.items.get(id).ok_or(ProstheticError::UnknownItem)?;
        if item.part != authorization.part {
            return Err(ProstheticError::WrongPart);
        }
        if anatomy.part(item.part).condition != BodyPartCondition::Missing {
            return Err(ProstheticError::PartNotMissing);
        }
        if item.durability_minutes == 0 {
            return Err(ProstheticError::Broken);
        }
        if self
            .fitted_slots
            .contains_key(&(authorization.cat_id.to_owned(), item.part))
        {
            return Err(ProstheticError::SlotOccupied);
        }
        match &item.location {
            ProstheticLocation::Inventory { colony_id } if colony_id == authorization.colony_id => {
            }
            ProstheticLocation::Inventory { .. } => return Err(ProstheticError::WrongOwner),
            _ => return Err(ProstheticError::NotInInventory),
        }
        self.items.get_mut(id).expect("item preflighted").location =
            ProstheticLocation::FittingReservation {
                colony_id: authorization.colony_id.to_owned(),
                reservation_id: authorization.reservation_id.to_owned(),
                cat_id: authorization.cat_id.to_owned(),
                fitter_id: authorization.fitter_id.to_owned(),
                site_id: authorization.site_id.to_owned(),
                site_kind: authorization.site_kind,
            };
        Ok(())
    }

    pub fn complete_fitting(&mut self, id: &ProstheticId) -> Result<(), ProstheticError> {
        let (colony_id, cat_id, part) = {
            let item = self.items.get(id).ok_or(ProstheticError::UnknownItem)?;
            let ProstheticLocation::FittingReservation {
                colony_id, cat_id, ..
            } = &item.location
            else {
                return Err(ProstheticError::NotReserved);
            };
            (colony_id.clone(), cat_id.clone(), item.part)
        };
        if self.fitted_slots.contains_key(&(cat_id.clone(), part)) {
            return Err(ProstheticError::SlotOccupied);
        }
        self.items.get_mut(id).expect("item preflighted").location = ProstheticLocation::Fitted {
            colony_id,
            cat_id: cat_id.clone(),
        };
        self.fitted_slots.insert((cat_id, part), id.clone());
        Ok(())
    }

    pub fn cancel_reservation(&mut self, id: &ProstheticId) -> Result<(), ProstheticError> {
        let colony_id = match &self
            .items
            .get(id)
            .ok_or(ProstheticError::UnknownItem)?
            .location
        {
            ProstheticLocation::FittingReservation { colony_id, .. }
            | ProstheticLocation::RepairReservation { colony_id, .. } => colony_id.clone(),
            _ => return Err(ProstheticError::NotReserved),
        };
        self.items.get_mut(id).expect("item preflighted").location =
            ProstheticLocation::Inventory { colony_id };
        Ok(())
    }

    pub fn unfit(&mut self, id: &ProstheticId) -> Result<(), ProstheticError> {
        let (colony_id, cat_id, part) = {
            let item = self.items.get(id).ok_or(ProstheticError::UnknownItem)?;
            let ProstheticLocation::Fitted { colony_id, cat_id } = &item.location else {
                return Err(ProstheticError::NotFitted);
            };
            (colony_id.clone(), cat_id.clone(), item.part)
        };
        if self.fitted_slots.get(&(cat_id.clone(), part)) != Some(id) {
            return Err(ProstheticError::NotFitted);
        }
        self.fitted_slots.remove(&(cat_id, part));
        self.items.get_mut(id).expect("item preflighted").location =
            ProstheticLocation::Inventory { colony_id };
        Ok(())
    }

    pub fn complete_rehabilitation_stage(
        &mut self,
        id: &ProstheticId,
    ) -> Result<(), ProstheticError> {
        let item = self.items.get_mut(id).ok_or(ProstheticError::UnknownItem)?;
        if !matches!(item.location, ProstheticLocation::Fitted { .. }) {
            return Err(ProstheticError::NotFitted);
        }
        if item.durability_minutes == 0 {
            return Err(ProstheticError::Broken);
        }
        item.rehabilitation_stages = item.rehabilitation_stages.saturating_add(1);
        Ok(())
    }

    /// Apply only work performed using the exact affected fitted body part.
    /// Returns the productive minutes before breakage for audit/adaptation accounting.
    pub fn record_affected_work(
        &mut self,
        cat_id: &str,
        part: BodyPart,
        minutes: u64,
        acquired_traits: &mut AcquiredTraitState,
    ) -> u64 {
        let Some(id) = self.fitted_slots.get(&(cat_id.to_owned(), part)).cloned() else {
            return 0;
        };
        let Some(item) = self.items.get_mut(&id) else {
            return 0;
        };
        let productive = minutes.min(item.durability_minutes);
        item.durability_minutes -= productive;
        acquired_traits.record_productive_prosthetic_minutes(productive);
        productive
    }

    #[must_use]
    pub fn effective_part_function_basis_points(
        &self,
        anatomy: &CatAnatomy,
        cat_id: &str,
        part: BodyPart,
        acquired_traits: &AcquiredTraitState,
    ) -> u16 {
        let natural = anatomy.part(part).condition.function_basis_points();
        if anatomy.part(part).condition != BodyPartCondition::Missing {
            return natural;
        }
        let Some(id) = self.fitted_slots.get(&(cat_id.to_owned(), part)) else {
            return natural;
        };
        let Some(item) = self
            .items
            .get(id)
            .filter(|item| item.durability_minutes > 0)
        else {
            return natural;
        };
        let rehab =
            u16::from(item.rehabilitation_stages).saturating_mul(REHABILITATION_BONUS_BASIS_POINTS);
        let adapted = acquired_traits.prosthetic_restoration_bonus().get().max(0) as u16;
        item.material
            .restoration_basis_points()
            .saturating_add(rehab)
            .saturating_add(adapted)
            .min(PROSTHETIC_FUNCTION_CAP_BASIS_POINTS)
    }

    pub fn begin_repair(
        &mut self,
        id: &ProstheticId,
        authorization: RepairAuthorization<'_>,
    ) -> Result<(), ProstheticError> {
        if !authorization.workshop_reachable {
            return Err(ProstheticError::SiteUnreachable);
        }
        if !authorization.finite_inputs_authorized {
            return Err(ProstheticError::FiniteInputsNotAuthorized);
        }
        if [
            authorization.colony_id,
            authorization.reservation_id,
            authorization.workshop_id,
        ]
        .contains(&"")
        {
            return Err(ProstheticError::InvalidAuthorization);
        }
        let item = self.items.get(id).ok_or(ProstheticError::UnknownItem)?;
        if item.durability_minutes != 0 {
            return Err(ProstheticError::NotBroken);
        }
        match &item.location {
            ProstheticLocation::Inventory { colony_id } if colony_id == authorization.colony_id => {
            }
            ProstheticLocation::Inventory { .. } => return Err(ProstheticError::WrongOwner),
            _ => return Err(ProstheticError::NotInInventory),
        }
        self.items.get_mut(id).expect("item preflighted").location =
            ProstheticLocation::RepairReservation {
                colony_id: authorization.colony_id.to_owned(),
                reservation_id: authorization.reservation_id.to_owned(),
                workshop_id: authorization.workshop_id.to_owned(),
            };
        Ok(())
    }

    pub fn complete_repair(&mut self, id: &ProstheticId) -> Result<(), ProstheticError> {
        let (colony_id, max_durability) = {
            let item = self.items.get(id).ok_or(ProstheticError::UnknownItem)?;
            let ProstheticLocation::RepairReservation { colony_id, .. } = &item.location else {
                return Err(ProstheticError::NotReserved);
            };
            (colony_id.clone(), item.material.max_durability_minutes())
        };
        let item = self.items.get_mut(id).expect("item preflighted");
        item.durability_minutes = max_durability;
        item.location = ProstheticLocation::Inventory { colony_id };
        Ok(())
    }

    pub fn recover_from_death(&mut self, cat_id: &str) -> Vec<ProstheticId> {
        let slots = self
            .fitted_slots
            .keys()
            .filter(|(fitted_cat_id, _)| fitted_cat_id == cat_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut recovered = Vec::with_capacity(slots.len());
        for slot in slots {
            let Some(id) = self.fitted_slots.remove(&slot) else {
                continue;
            };
            let Some(item) = self.items.get_mut(&id) else {
                continue;
            };
            let ProstheticLocation::Fitted { colony_id, .. } = &item.location else {
                continue;
            };
            item.location = ProstheticLocation::Inventory {
                colony_id: colony_id.clone(),
            };
            recovered.push(id);
        }
        recovered
    }

    #[must_use]
    pub fn trade_eligible(&self, id: &ProstheticId) -> bool {
        self.items
            .get(id)
            .is_some_and(|item| matches!(item.location, ProstheticLocation::Inventory { .. }))
    }

    pub fn begin_trade(
        &mut self,
        id: &ProstheticId,
        caravan_id: &str,
    ) -> Result<(), ProstheticError> {
        if caravan_id.is_empty() {
            return Err(ProstheticError::InvalidAuthorization);
        }
        let colony_id = match &self
            .items
            .get(id)
            .ok_or(ProstheticError::UnknownItem)?
            .location
        {
            ProstheticLocation::Inventory { colony_id } => colony_id.clone(),
            _ => return Err(ProstheticError::NotInInventory),
        };
        self.items.get_mut(id).expect("item preflighted").location =
            ProstheticLocation::TradeCargo {
                colony_id,
                caravan_id: caravan_id.to_owned(),
            };
        Ok(())
    }

    pub fn cancel_trade(&mut self, id: &ProstheticId) -> Result<(), ProstheticError> {
        let colony_id = match &self
            .items
            .get(id)
            .ok_or(ProstheticError::UnknownItem)?
            .location
        {
            ProstheticLocation::TradeCargo { colony_id, .. } => colony_id.clone(),
            _ => return Err(ProstheticError::NotReserved),
        };
        self.items.get_mut(id).expect("item preflighted").location =
            ProstheticLocation::Inventory { colony_id };
        Ok(())
    }

    pub fn complete_trade(
        &mut self,
        id: &ProstheticId,
        destination_colony_id: &str,
    ) -> Result<(), ProstheticError> {
        if destination_colony_id.is_empty() {
            return Err(ProstheticError::InvalidAuthorization);
        }
        if !matches!(
            self.items
                .get(id)
                .ok_or(ProstheticError::UnknownItem)?
                .location,
            ProstheticLocation::TradeCargo { .. }
        ) {
            return Err(ProstheticError::NotReserved);
        }
        self.items.get_mut(id).expect("item preflighted").location =
            ProstheticLocation::Inventory {
                colony_id: destination_colony_id.to_owned(),
            };
        Ok(())
    }

    #[must_use]
    pub fn fitted_item(&self, cat_id: &str, part: BodyPart) -> Option<&ProstheticId> {
        self.fitted_slots.get(&(cat_id.to_owned(), part))
    }

    #[must_use]
    pub fn location(&self, id: &ProstheticId) -> Option<&ProstheticLocation> {
        self.items.get(id).map(|item| &item.location)
    }

    #[must_use]
    pub fn remaining_durability_minutes(&self, id: &ProstheticId) -> Option<u64> {
        self.items.get(id).map(|item| item.durability_minutes)
    }

    #[must_use]
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub fn item(&self, id: &ProstheticId) -> Option<&ProstheticItem> {
        self.items.get(id)
    }

    /// Iterate all durable prosthetic items in stable item-ID order.
    pub fn items(&self) -> impl ExactSizeIterator<Item = &ProstheticItem> {
        self.items.values()
    }

    pub fn item_ids(&self) -> impl ExactSizeIterator<Item = &ProstheticId> {
        self.items.keys()
    }

    fn validate(&self) -> Result<(), &'static str> {
        let mut fitted_ids = BTreeSet::new();
        for (id, item) in &self.items {
            if id != &item.id || !id.is_valid() {
                return Err("invalid or mismatched prosthetic item id");
            }
            if item.durability_minutes > item.material.max_durability_minutes() {
                return Err("prosthetic durability exceeds canonical material maximum");
            }
            if !location_fields_valid(&item.location) {
                return Err("prosthetic location contains an empty stable id");
            }
            if matches!(item.location, ProstheticLocation::FittingReservation { .. })
                && item.durability_minutes == 0
            {
                return Err("broken prosthetic cannot retain a fitting reservation");
            }
            if matches!(item.location, ProstheticLocation::RepairReservation { .. })
                && item.durability_minutes != 0
            {
                return Err("serviceable prosthetic cannot retain a repair reservation");
            }
            let fitted_slot = self
                .fitted_slots
                .iter()
                .find(|(_, fitted_id)| *fitted_id == id);
            match (&item.location, fitted_slot) {
                (
                    ProstheticLocation::Fitted { cat_id, .. },
                    Some(((slot_cat_id, slot_part), _)),
                ) if cat_id == slot_cat_id && item.part == *slot_part => {}
                (ProstheticLocation::Fitted { .. }, _) => {
                    return Err("fitted item has no matching anatomy slot");
                }
                (_, Some(_)) => return Err("unfitted item is referenced by an anatomy slot"),
                (_, None) => {}
            }
        }
        for ((cat_id, part), id) in &self.fitted_slots {
            if cat_id.is_empty() {
                return Err("empty fitted cat id");
            }
            let Some(item) = self.items.get(id) else {
                return Err("fitted slot references an unknown item");
            };
            if item.part != *part {
                return Err("fitted slot part does not match prosthetic side");
            }
            if !fitted_ids.insert(id) {
                return Err("one prosthetic item cannot occupy two anatomy slots");
            }
            if !matches!(
                &item.location,
                ProstheticLocation::Fitted {
                    cat_id: fitted_cat_id,
                    ..
                } if fitted_cat_id == cat_id
            ) {
                return Err("fitted slot cat does not match prosthetic location");
            }
        }
        Ok(())
    }
}

fn location_fields_valid(location: &ProstheticLocation) -> bool {
    match location {
        ProstheticLocation::Inventory { colony_id } => !colony_id.is_empty(),
        ProstheticLocation::FittingReservation {
            colony_id,
            reservation_id,
            cat_id,
            fitter_id,
            site_id,
            ..
        } => [colony_id, reservation_id, cat_id, fitter_id, site_id]
            .into_iter()
            .all(|value| !value.is_empty()),
        ProstheticLocation::Fitted { colony_id, cat_id } => {
            !colony_id.is_empty() && !cat_id.is_empty()
        }
        ProstheticLocation::RepairReservation {
            colony_id,
            reservation_id,
            workshop_id,
        } => [colony_id, reservation_id, workshop_id]
            .into_iter()
            .all(|value| !value.is_empty()),
        ProstheticLocation::TradeCargo {
            colony_id,
            caravan_id,
        } => !colony_id.is_empty() && !caravan_id.is_empty(),
    }
}
