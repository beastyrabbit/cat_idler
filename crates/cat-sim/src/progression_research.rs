//! LAI.44 pure progression authority.
//!
//! The embedded LAI.36 content manifest is the sole capability catalog. This
//! leaf owns currency separation, capability gates, physical scholar work,
//! player preparation, and typed hooks for the later two-lane coordinator.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    content_manifest::{
        CapabilityId, CapabilityRequirement, ContentId, ContentManifest, ContentOperation,
        REQUIRED_FOUNDING_CAPABILITIES, RecipeId,
    },
    planner_core::PlannerId,
};

pub const PROGRESSION_SCHEMA_VERSION: u32 = 1;
pub const NOTES_LEDGER_SCHEMA_VERSION: u32 = 1;
pub const VOID_LEDGER_SCHEMA_VERSION: u32 = 1;
pub const MICRO_UNITS_PER_WHOLE: u64 = 1_000_000;
pub const PLAYER_PREPARATION_DISCOUNT_BASIS_POINTS: u16 = 2_500;
pub const MAX_STUDIES: usize = 256;
pub const MAX_ACTIVE_SCHOLAR_WORK: usize = 128;
pub const MAX_CURRENCY_RECEIPTS: usize = 512;
pub const MAX_LANE_CLAIMS: usize = 128;
pub const MAX_DRAIN_BATCH: usize = 64;
pub const HOLE_AXIS_STUDY_COUNT: usize = 30;
pub const BOOST_UNLOCK_STUDY_COUNT: usize = 4;
pub const BOOST_RESEARCH_STAGE_COUNT: u8 = 11;
pub const NOTES_MICRO_PER_WORK_MINUTE: u64 = 1_000;
pub const OOPSIE_PERCENT_BY_LEVEL: [u8; 5] = [25, 12, 5, 1, 0];

macro_rules! fixed_amount {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            pub const ZERO: Self = Self(0);
            pub const ONE: Self = Self(MICRO_UNITS_PER_WHOLE);

            #[must_use]
            pub const fn from_micro(value: u64) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn from_whole(value: u64) -> Option<Self> {
                match value.checked_mul(MICRO_UNITS_PER_WHOLE) {
                    Some(value) => Some(Self(value)),
                    None => None,
                }
            }

            #[must_use]
            pub const fn micro(self) -> u64 {
                self.0
            }

            pub fn checked_add(self, other: Self) -> Result<Self, ProgressionError> {
                self.0
                    .checked_add(other.0)
                    .map(Self)
                    .ok_or(ProgressionError::ArithmeticOverflow)
            }

            pub fn checked_sub(self, other: Self) -> Result<Self, ProgressionError> {
                self.0
                    .checked_sub(other.0)
                    .map(Self)
                    .ok_or(ProgressionError::InsufficientCurrency)
            }
        }
    };
}

fixed_amount!(ResearchNotes);
fixed_amount!(VoidInsight);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StudyId(String);

impl StudyId {
    pub fn new(value: impl Into<String>) -> Result<Self, ProgressionError> {
        let value = value.into();
        if crate::content_manifest::is_valid_stable_id(&value) {
            Ok(Self(value))
        } else {
            Err(ProgressionError::MalformedId)
        }
    }

    #[must_use]
    pub fn from_capability(value: &CapabilityId) -> Self {
        Self(value.as_str().to_owned())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StudyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CurrencyEventId(PlannerId);

impl CurrencyEventId {
    #[must_use]
    pub fn derive(namespace: &str, colony_id: &PlannerId, action: &str) -> Self {
        Self(PlannerId::derive(
            "progression_currency_event",
            [namespace, colony_id.as_str(), action],
        ))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ColonyPartitionKey {
    pub colony_id: PlannerId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerPartitionKey {
    pub colony_id: PlannerId,
    pub player_id: PlannerId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StudyCurrency {
    Notes,
    Void,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoleAxis {
    Width,
    Depth,
    Darkness,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecializedBoost {
    BountifulLabor,
    FleetPaws,
    InspiredWork,
    RestorativeGrace,
}

impl SpecializedBoost {
    pub const ALL: [Self; BOOST_UNLOCK_STUDY_COUNT] = [
        Self::BountifulLabor,
        Self::FleetPaws,
        Self::InspiredWork,
        Self::RestorativeGrace,
    ];

    #[must_use]
    pub const fn study_suffix(self) -> &'static str {
        match self {
            Self::BountifulLabor => "bountiful_labor",
            Self::FleetPaws => "fleet_paws",
            Self::InspiredWork => "inspired_work",
            Self::RestorativeGrace => "restorative_grace",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StudyKind {
    OrdinaryCapability { capability_id: CapabilityId },
    HoleAxis { axis: HoleAxis, level: u8 },
    BoostUnlock { boost: SpecializedBoost },
    BoostDuration { stage: u8 },
    BoostEconomy { stage: u8 },
}

impl StudyKind {
    #[must_use]
    pub const fn currency(&self) -> StudyCurrency {
        match self {
            Self::OrdinaryCapability { .. } => StudyCurrency::Notes,
            Self::HoleAxis { .. }
            | Self::BoostUnlock { .. }
            | Self::BoostDuration { .. }
            | Self::BoostEconomy { .. } => StudyCurrency::Void,
        }
    }

    #[must_use]
    pub const fn is_ordinary(&self) -> bool {
        matches!(self, Self::OrdinaryCapability { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StudyDefinition {
    pub id: StudyId,
    pub display_name: String,
    pub order: u32,
    pub kind: StudyKind,
    pub prerequisites: BTreeSet<StudyId>,
    pub cost_micro: u64,
    pub required_work_minutes: u64,
}

impl StudyDefinition {
    #[must_use]
    pub const fn currency(&self) -> StudyCurrency {
        self.kind.currency()
    }

    #[must_use]
    pub const fn notes_cost(&self) -> Option<ResearchNotes> {
        match self.currency() {
            StudyCurrency::Notes => Some(ResearchNotes::from_micro(self.cost_micro)),
            StudyCurrency::Void => None,
        }
    }

    #[must_use]
    pub const fn void_cost(&self) -> Option<VoidInsight> {
        match self.currency() {
            StudyCurrency::Notes => None,
            StudyCurrency::Void => Some(VoidInsight::from_micro(self.cost_micro)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgressionCatalog {
    studies: BTreeMap<StudyId, StudyDefinition>,
    founding_capabilities: BTreeSet<CapabilityId>,
    capability_to_study: BTreeMap<CapabilityId, StudyId>,
}

impl ProgressionCatalog {
    pub fn from_embedded() -> Result<Self, ProgressionError> {
        Self::from_manifest(ContentManifest::embedded())
    }

    pub fn from_manifest(manifest: &ContentManifest) -> Result<Self, ProgressionError> {
        manifest
            .validate()
            .map_err(|_| ProgressionError::ManifestInvalid)?;
        if manifest.capabilities.len() > MAX_STUDIES {
            return Err(ProgressionError::CapacityExceeded);
        }

        let founding_capabilities = manifest
            .founding_capabilities
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let expected_founding = REQUIRED_FOUNDING_CAPABILITIES
            .into_iter()
            .map(CapabilityId::new)
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(|_| ProgressionError::ManifestInvalid)?;
        if founding_capabilities != expected_founding {
            return Err(ProgressionError::ManifestInvalid);
        }
        for resource_id in ["logs", "stone"] {
            let founding = manifest.resources.iter().any(|resource| {
                resource.id.as_str() == resource_id
                    && resource.acquisition.founding_available
                    && resource.canonical_capability == CapabilityRequirement::Free
            });
            if !founding {
                return Err(ProgressionError::ManifestInvalid);
            }
        }

        let mut studies = BTreeMap::new();
        let mut capability_to_study = BTreeMap::new();
        for capability in &manifest.capabilities {
            if capability.founding_owned {
                if !founding_capabilities.contains(&capability.id) {
                    return Err(ProgressionError::ManifestInvalid);
                }
                continue;
            }
            let id = StudyId::from_capability(&capability.id);
            let kind = parse_hole_axis(&id).map_or_else(
                || StudyKind::OrdinaryCapability {
                    capability_id: capability.id.clone(),
                },
                |(axis, level)| StudyKind::HoleAxis { axis, level },
            );
            let level = match &kind {
                StudyKind::HoleAxis { level, .. } => u64::from(*level),
                _ => 0,
            };
            let cost_micro = if level == 0 {
                u64::from(capability.order)
                    .checked_add(1)
                    .and_then(|value| value.checked_mul(10_000))
                    .ok_or(ProgressionError::ArithmeticOverflow)?
            } else {
                level
                    .checked_mul(MICRO_UNITS_PER_WHOLE)
                    .ok_or(ProgressionError::ArithmeticOverflow)?
            };
            let required_work_minutes = u64::from(capability.order)
                .checked_add(60)
                .ok_or(ProgressionError::ArithmeticOverflow)?;
            let prerequisites = capability
                .prerequisites
                .iter()
                .filter(|prerequisite| !founding_capabilities.contains(*prerequisite))
                .map(StudyId::from_capability)
                .collect::<BTreeSet<_>>();
            let definition = StudyDefinition {
                id: id.clone(),
                display_name: capability.display_name.clone(),
                order: capability.order,
                kind,
                prerequisites,
                cost_micro,
                required_work_minutes,
            };
            if studies.insert(id.clone(), definition).is_some()
                || capability_to_study
                    .insert(capability.id.clone(), id)
                    .is_some()
            {
                return Err(ProgressionError::ManifestInvalid);
            }
        }

        add_boost_studies(&mut studies)?;
        let catalog = Self {
            studies,
            founding_capabilities,
            capability_to_study,
        };
        catalog.validate(manifest)?;
        Ok(catalog)
    }

    fn validate(&self, manifest: &ContentManifest) -> Result<(), ProgressionError> {
        if self.studies.len() > MAX_STUDIES
            || self.hole_axis_studies().len() != HOLE_AXIS_STUDY_COUNT
            || self.boost_unlock_studies().len() != BOOST_UNLOCK_STUDY_COUNT
        {
            return Err(ProgressionError::ManifestInvalid);
        }
        for axis in [HoleAxis::Width, HoleAxis::Depth, HoleAxis::Darkness] {
            let levels = self
                .studies
                .values()
                .filter_map(|study| match study.kind {
                    StudyKind::HoleAxis {
                        axis: candidate,
                        level,
                    } if candidate == axis => Some(level),
                    _ => None,
                })
                .collect::<BTreeSet<_>>();
            if levels != (1..=10).collect::<BTreeSet<_>>() {
                return Err(ProgressionError::ManifestInvalid);
            }
        }
        if self
            .studies
            .values()
            .any(|study| study.cost_micro == 0 || study.required_work_minutes == 0)
        {
            return Err(ProgressionError::ManifestInvalid);
        }
        for study in self.studies.values() {
            if study
                .prerequisites
                .iter()
                .any(|id| !self.studies.contains_key(id))
            {
                return Err(ProgressionError::ManifestInvalid);
            }
        }
        let plank =
            CapabilityId::new("plank_processing").map_err(|_| ProgressionError::ManifestInvalid)?;
        let plank_study = self
            .capability_to_study
            .get(&plank)
            .ok_or(ProgressionError::ManifestInvalid)?;
        if plank_study.as_str() != "plank_processing"
            || self
                .capability_to_study
                .keys()
                .filter(|id| id.as_str().contains("plank"))
                .count()
                != 1
        {
            return Err(ProgressionError::ManifestInvalid);
        }
        if manifest.derived_capability_total()
            != self.capability_to_study.len() + self.founding_capabilities.len()
        {
            return Err(ProgressionError::ManifestInvalid);
        }
        Ok(())
    }

    #[must_use]
    pub fn studies(&self) -> &BTreeMap<StudyId, StudyDefinition> {
        &self.studies
    }

    #[must_use]
    pub fn study(&self, id: &StudyId) -> Option<&StudyDefinition> {
        self.studies.get(id)
    }

    #[must_use]
    pub fn derived_study_total(&self) -> usize {
        self.studies.len()
    }

    #[must_use]
    pub fn derived_capability_study_total(&self) -> usize {
        self.capability_to_study.len()
    }

    #[must_use]
    pub fn founding_capabilities(&self) -> &BTreeSet<CapabilityId> {
        &self.founding_capabilities
    }

    #[must_use]
    pub fn hole_axis_studies(&self) -> Vec<&StudyDefinition> {
        self.studies
            .values()
            .filter(|study| matches!(study.kind, StudyKind::HoleAxis { .. }))
            .collect()
    }

    #[must_use]
    pub fn boost_unlock_studies(&self) -> Vec<&StudyDefinition> {
        self.studies
            .values()
            .filter(|study| matches!(study.kind, StudyKind::BoostUnlock { .. }))
            .collect()
    }
}

fn parse_hole_axis(id: &StudyId) -> Option<(HoleAxis, u8)> {
    for (prefix, axis) in [
        ("black_hole_width_", HoleAxis::Width),
        ("black_hole_depth_", HoleAxis::Depth),
        ("black_hole_darkness_", HoleAxis::Darkness),
    ] {
        let Some(level) = id.as_str().strip_prefix(prefix) else {
            continue;
        };
        let level_text = level;
        let Ok(level) = level_text.parse::<u8>() else {
            return None;
        };
        if (1..=10).contains(&level) && level_text.len() == 2 {
            return Some((axis, level));
        }
    }
    None
}

fn add_boost_studies(
    studies: &mut BTreeMap<StudyId, StudyDefinition>,
) -> Result<(), ProgressionError> {
    let mut order = 10_000_u32;
    for boost in SpecializedBoost::ALL {
        let id = StudyId::new(format!("divine_boost_{}", boost.study_suffix()))?;
        studies.insert(
            id.clone(),
            StudyDefinition {
                id,
                display_name: format!("{boost:?}"),
                order,
                kind: StudyKind::BoostUnlock { boost },
                prerequisites: BTreeSet::new(),
                cost_micro: 10_u64
                    .checked_mul(MICRO_UNITS_PER_WHOLE)
                    .ok_or(ProgressionError::ArithmeticOverflow)?,
                required_work_minutes: 60,
            },
        );
        order = order
            .checked_add(1)
            .ok_or(ProgressionError::ArithmeticOverflow)?;
    }
    for stage in 1..=BOOST_RESEARCH_STAGE_COUNT {
        for (prefix, multiplier, kind) in [
            ("divine_duration", 2_u64, StudyKind::BoostDuration { stage }),
            ("divine_economy", 3_u64, StudyKind::BoostEconomy { stage }),
        ] {
            let id = StudyId::new(format!("{prefix}_{stage:02}"))?;
            let prerequisite = (stage > 1)
                .then(|| StudyId::new(format!("{prefix}_{:02}", stage - 1)))
                .transpose()?
                .into_iter()
                .collect();
            let cost_micro = u64::from(stage)
                .checked_mul(multiplier)
                .and_then(|value| value.checked_mul(MICRO_UNITS_PER_WHOLE))
                .ok_or(ProgressionError::ArithmeticOverflow)?;
            studies.insert(
                id.clone(),
                StudyDefinition {
                    id,
                    display_name: format!("{prefix} {stage}"),
                    order,
                    kind,
                    prerequisites: prerequisite,
                    cost_micro,
                    required_work_minutes: u64::from(stage) * 60,
                },
            );
            order = order
                .checked_add(1)
                .ok_or(ProgressionError::ArithmeticOverflow)?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CurrencyCommitOutcome {
    Committed,
    AlreadyCommitted,
    RetiredReplay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoidDebitPurpose {
    HoleStudy,
    BoostStudy,
    BoostActivation,
    ConstructionMiracle,
    EmergencyRescue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurrencyDebitReceipt {
    pub id: CurrencyEventId,
    pub amount_micro: u64,
    pub fingerprint: u64,
    pub void_purpose: Option<VoidDebitPurpose>,
    pub committed_version: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchNotesSpendRequest {
    pub id: CurrencyEventId,
    pub amount: ResearchNotes,
    pub expected_version: u64,
    pub fingerprint: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchNotesLedger {
    pub schema_version: u32,
    pub partition: ColonyPartitionKey,
    pub version: u64,
    pub balance: ResearchNotes,
    pub retired_work_through: u64,
    pub spends: BTreeMap<CurrencyEventId, CurrencyDebitReceipt>,
}

impl ResearchNotesLedger {
    #[must_use]
    pub fn new(colony_id: PlannerId) -> Self {
        Self {
            schema_version: NOTES_LEDGER_SCHEMA_VERSION,
            partition: ColonyPartitionKey { colony_id },
            version: 0,
            balance: ResearchNotes::ZERO,
            retired_work_through: 0,
            spends: BTreeMap::new(),
        }
    }

    fn advance_scholar_work(
        &mut self,
        sequence: u64,
        credit: Option<ResearchNotes>,
    ) -> Result<CurrencyCommitOutcome, ProgressionError> {
        if sequence <= self.retired_work_through {
            return Ok(CurrencyCommitOutcome::RetiredReplay);
        }
        if sequence
            != self
                .retired_work_through
                .checked_add(1)
                .ok_or(ProgressionError::ArithmeticOverflow)?
        {
            return Err(ProgressionError::NonCanonicalSequence);
        }
        if let Some(credit) = credit {
            if credit == ResearchNotes::ZERO {
                return Err(ProgressionError::MalformedRequest);
            }
            self.balance = self.balance.checked_add(credit)?;
        }
        self.retired_work_through = sequence;
        self.version = self
            .version
            .checked_add(1)
            .ok_or(ProgressionError::ArithmeticOverflow)?;
        Ok(CurrencyCommitOutcome::Committed)
    }

    pub fn debit(
        &mut self,
        request: ResearchNotesSpendRequest,
    ) -> Result<CurrencyCommitOutcome, ProgressionError> {
        let mut next = self.clone();
        let outcome = next.debit_inner(request)?;
        next.validate()?;
        *self = next;
        Ok(outcome)
    }

    fn debit_inner(
        &mut self,
        request: ResearchNotesSpendRequest,
    ) -> Result<CurrencyCommitOutcome, ProgressionError> {
        if request.amount == ResearchNotes::ZERO {
            return Err(ProgressionError::MalformedRequest);
        }
        if let Some(receipt) = self.spends.get(&request.id) {
            return if receipt.amount_micro == request.amount.micro()
                && receipt.fingerprint == request.fingerprint
            {
                Ok(CurrencyCommitOutcome::AlreadyCommitted)
            } else {
                Err(ProgressionError::IdempotencyConflict)
            };
        }
        if request.expected_version != self.version {
            return Err(ProgressionError::StaleVersion);
        }
        if self.spends.len() >= MAX_CURRENCY_RECEIPTS {
            return Err(ProgressionError::Backpressure);
        }
        let committed_version = self
            .version
            .checked_add(1)
            .ok_or(ProgressionError::ArithmeticOverflow)?;
        self.balance = self.balance.checked_sub(request.amount)?;
        self.spends.insert(
            request.id.clone(),
            CurrencyDebitReceipt {
                id: request.id,
                amount_micro: request.amount.micro(),
                fingerprint: request.fingerprint,
                void_purpose: None,
                committed_version,
            },
        );
        self.version = committed_version;
        Ok(CurrencyCommitOutcome::Committed)
    }

    fn validate(&self) -> Result<(), ProgressionError> {
        if self.schema_version != NOTES_LEDGER_SCHEMA_VERSION
            || self.spends.len() > MAX_CURRENCY_RECEIPTS
            || self.spends.iter().any(|(id, receipt)| {
                id != &receipt.id || receipt.amount_micro == 0 || receipt.void_purpose.is_some()
            })
            || self.spends.values().any(|receipt| {
                receipt.committed_version == 0 || receipt.committed_version > self.version
            })
        {
            return Err(ProgressionError::MalformedPersistence);
        }
        if self
            .spends
            .values()
            .map(|receipt| receipt.committed_version)
            .collect::<BTreeSet<_>>()
            .len()
            != self.spends.len()
        {
            return Err(ProgressionError::MalformedPersistence);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedResearchNotesLedger {
    schema_version: u32,
    partition: ColonyPartitionKey,
    version: u64,
    balance: ResearchNotes,
    retired_work_through: u64,
    spends: BTreeMap<CurrencyEventId, CurrencyDebitReceipt>,
}

impl<'de> Deserialize<'de> for ResearchNotesLedger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UncheckedResearchNotesLedger::deserialize(deserializer)?;
        let ledger = Self {
            schema_version: raw.schema_version,
            partition: raw.partition,
            version: raw.version,
            balance: raw.balance,
            retired_work_through: raw.retired_work_through,
            spends: raw.spends,
        };
        ledger.validate().map_err(serde::de::Error::custom)?;
        Ok(ledger)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HoleVoidCreditPayload {
    pub partition: ColonyPartitionKey,
    pub feed_sequence: u64,
    pub amount: VoidInsight,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VoidSpendRequest {
    pub id: CurrencyEventId,
    pub amount: VoidInsight,
    pub purpose: VoidDebitPurpose,
    pub expected_version: u64,
    pub fingerprint: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidInsightLedger {
    pub schema_version: u32,
    pub partition: ColonyPartitionKey,
    pub version: u64,
    pub balance: VoidInsight,
    pub credited_feed_through: u64,
    pub spends: BTreeMap<CurrencyEventId, CurrencyDebitReceipt>,
}

impl VoidInsightLedger {
    #[must_use]
    pub fn new(colony_id: PlannerId) -> Self {
        Self {
            schema_version: VOID_LEDGER_SCHEMA_VERSION,
            partition: ColonyPartitionKey { colony_id },
            version: 0,
            balance: VoidInsight::ZERO,
            credited_feed_through: 0,
            spends: BTreeMap::new(),
        }
    }

    pub fn credit_hole_feed(
        &mut self,
        payload: HoleVoidCreditPayload,
    ) -> Result<CurrencyCommitOutcome, ProgressionError> {
        let mut next = self.clone();
        let outcome = next.credit_hole_feed_inner(payload)?;
        next.validate()?;
        *self = next;
        Ok(outcome)
    }

    fn credit_hole_feed_inner(
        &mut self,
        payload: HoleVoidCreditPayload,
    ) -> Result<CurrencyCommitOutcome, ProgressionError> {
        if payload.partition != self.partition || payload.amount == VoidInsight::ZERO {
            return Err(ProgressionError::PartitionMismatch);
        }
        if payload.feed_sequence <= self.credited_feed_through {
            return Ok(CurrencyCommitOutcome::RetiredReplay);
        }
        if payload.feed_sequence
            != self
                .credited_feed_through
                .checked_add(1)
                .ok_or(ProgressionError::ArithmeticOverflow)?
        {
            return Err(ProgressionError::NonCanonicalSequence);
        }
        self.balance = self.balance.checked_add(payload.amount)?;
        self.credited_feed_through = payload.feed_sequence;
        self.version = self
            .version
            .checked_add(1)
            .ok_or(ProgressionError::ArithmeticOverflow)?;
        Ok(CurrencyCommitOutcome::Committed)
    }

    pub fn debit(
        &mut self,
        request: VoidSpendRequest,
    ) -> Result<CurrencyCommitOutcome, ProgressionError> {
        let mut next = self.clone();
        let outcome = next.debit_inner(request)?;
        next.validate()?;
        *self = next;
        Ok(outcome)
    }

    fn debit_inner(
        &mut self,
        request: VoidSpendRequest,
    ) -> Result<CurrencyCommitOutcome, ProgressionError> {
        if request.amount == VoidInsight::ZERO {
            return Err(ProgressionError::MalformedRequest);
        }
        if let Some(receipt) = self.spends.get(&request.id) {
            return if receipt.amount_micro == request.amount.micro()
                && receipt.fingerprint == request.fingerprint
            {
                Ok(CurrencyCommitOutcome::AlreadyCommitted)
            } else {
                Err(ProgressionError::IdempotencyConflict)
            };
        }
        if request.expected_version != self.version {
            return Err(ProgressionError::StaleVersion);
        }
        if self.spends.len() >= MAX_CURRENCY_RECEIPTS {
            return Err(ProgressionError::Backpressure);
        }
        let committed_version = self
            .version
            .checked_add(1)
            .ok_or(ProgressionError::ArithmeticOverflow)?;
        self.balance = self.balance.checked_sub(request.amount)?;
        self.spends.insert(
            request.id.clone(),
            CurrencyDebitReceipt {
                id: request.id,
                amount_micro: request.amount.micro(),
                fingerprint: request.fingerprint,
                void_purpose: Some(request.purpose),
                committed_version,
            },
        );
        self.version = committed_version;
        Ok(CurrencyCommitOutcome::Committed)
    }

    pub(crate) fn drain_spend_receipts(
        &mut self,
        ids: &BTreeSet<CurrencyEventId>,
    ) -> Result<usize, ProgressionError> {
        if ids.len() > MAX_DRAIN_BATCH {
            return Err(ProgressionError::CapacityExceeded);
        }
        let mut next = self.clone();
        let mut drained = 0;
        for id in ids {
            if next.spends.get(id).is_some_and(|receipt| {
                matches!(
                    receipt.void_purpose,
                    Some(
                        VoidDebitPurpose::BoostActivation
                            | VoidDebitPurpose::ConstructionMiracle
                            | VoidDebitPurpose::EmergencyRescue
                    )
                )
            }) {
                next.spends.remove(id);
                drained += 1;
            }
        }
        next.validate()?;
        *self = next;
        Ok(drained)
    }

    fn validate(&self) -> Result<(), ProgressionError> {
        if self.schema_version != VOID_LEDGER_SCHEMA_VERSION
            || self.spends.len() > MAX_CURRENCY_RECEIPTS
            || self.spends.iter().any(|(id, receipt)| {
                id != &receipt.id || receipt.amount_micro == 0 || receipt.void_purpose.is_none()
            })
            || self.spends.values().any(|receipt| {
                receipt.committed_version == 0 || receipt.committed_version > self.version
            })
        {
            return Err(ProgressionError::MalformedPersistence);
        }
        if self
            .spends
            .values()
            .map(|receipt| receipt.committed_version)
            .collect::<BTreeSet<_>>()
            .len()
            != self.spends.len()
        {
            return Err(ProgressionError::MalformedPersistence);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedVoidInsightLedger {
    schema_version: u32,
    partition: ColonyPartitionKey,
    version: u64,
    balance: VoidInsight,
    credited_feed_through: u64,
    spends: BTreeMap<CurrencyEventId, CurrencyDebitReceipt>,
}

impl<'de> Deserialize<'de> for VoidInsightLedger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UncheckedVoidInsightLedger::deserialize(deserializer)?;
        let ledger = Self {
            schema_version: raw.schema_version,
            partition: raw.partition,
            version: raw.version,
            balance: raw.balance,
            credited_feed_through: raw.credited_feed_through,
            spends: raw.spends,
        };
        ledger.validate().map_err(serde::de::Error::custom)?;
        Ok(ledger)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScholarWorkId {
    pub colony_id: PlannerId,
    pub sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScholarTaskId(PlannerId);

impl ScholarTaskId {
    #[must_use]
    pub fn derive(work_id: &ScholarWorkId) -> Self {
        Self(PlannerId::derive(
            "progression_scholar_task",
            [work_id.colony_id.as_str(), &work_id.sequence.to_string()],
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScholarOrderId(PlannerId);

impl ScholarOrderId {
    #[must_use]
    pub fn derive(work_id: &ScholarWorkId) -> Self {
        Self(PlannerId::derive(
            "progression_scholar_order",
            [work_id.colony_id.as_str(), &work_id.sequence.to_string()],
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScholarAssignment {
    pub scholar_id: PlannerId,
    pub tool_id: PlannerId,
    pub station_id: PlannerId,
    pub location_id: PlannerId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScholarWorkPurpose {
    ProduceNotes {
        credit: ResearchNotes,
    },
    PrepareStudy {
        study_id: StudyId,
        player_id: PlannerId,
        frozen_base_cost: ResearchNotes,
        frozen_discount: ResearchNotes,
        frozen_payable: ResearchNotes,
    },
    CompleteGodStudy {
        study_id: StudyId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScholarWorkStage {
    Queued,
    Reserved,
    Working,
    Completed,
    Cancelled,
}

impl ScholarWorkStage {
    #[must_use]
    pub const fn terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScholarWorkOrder {
    pub id: ScholarWorkId,
    pub order_id: ScholarOrderId,
    pub task_id: ScholarTaskId,
    pub assignment: ScholarAssignment,
    pub purpose: ScholarWorkPurpose,
    pub required_work_minutes: u64,
    pub progress_work_minutes: u64,
    pub stage: ScholarWorkStage,
    pub request_fingerprint: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkRecoveryReason {
    CancelledByPlayer,
    ScholarDied,
    RouteLost,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkRelease {
    pub work_id: ScholarWorkId,
    pub assignment: ScholarAssignment,
    pub reason: WorkRecoveryReason,
    pub released_reservation: bool,
    pub lost_progress_work_minutes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchNotesCreditPayload {
    pub partition: ColonyPartitionKey,
    pub work_id: ScholarWorkId,
    pub amount: ResearchNotes,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparationRecord {
    pub study_id: StudyId,
    pub player_id: PlannerId,
    pub completed_work_id: ScholarWorkId,
    pub frozen_base_cost: ResearchNotes,
    pub frozen_discount: ResearchNotes,
    pub frozen_payable: ResearchNotes,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GodQueueClaim {
    pub study_id: StudyId,
    pub player_id: PlannerId,
    pub prepared_terms: Option<PreparationRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FundedStudy {
    pub study_id: StudyId,
    pub player_id: PlannerId,
    pub currency: StudyCurrency,
    pub paid_micro: u64,
    pub preparation_consumed: bool,
    pub funding_event_id: CurrencyEventId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderDuplicateKind {
    VillageCritical,
    KeyedOopsie,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaderTargetClaim {
    pub study_id: StudyId,
    pub duplicate_kind: Option<LeaderDuplicateKind>,
    pub decision_key: Option<PlannerId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaderDuplicateHook {
    None,
    VillageCritical {
        report_marks_critical: bool,
        needed_before_tick: u64,
        estimated_god_completion_tick: u64,
    },
    KeyedOopsie {
        decision_key: PlannerId,
        effective_level: u8,
        keyed_roll_percent: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeaderTargetDecision {
    Allowed,
    ChooseAnother,
    VillageCriticalOverride,
    KeyedOopsieOverride,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeCapabilityCheck {
    pub recipe_id: RecipeId,
    pub station_exists: bool,
    pub station_tier: u8,
    pub physical_ingredients_ready: bool,
    pub tools_ready: bool,
    pub capacity_ready: bool,
    pub workers_ready: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ManifestContentClass {
    Resource,
    Food,
    Item,
    Material,
    Creature,
    Station,
    Recipe,
    Augmentation,
    Fixture,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionAuthority {
    pub schema_version: u32,
    pub partition: ColonyPartitionKey,
    pub version: u64,
    pub owned_studies: BTreeSet<StudyId>,
    pub notes: ResearchNotesLedger,
    pub void: VoidInsightLedger,
    pub next_work_sequence: u64,
    pub retired_work_through: u64,
    pub work_orders: BTreeMap<u64, ScholarWorkOrder>,
    pub prepared: BTreeMap<StudyId, PreparationRecord>,
    pub god_queued: BTreeMap<StudyId, GodQueueClaim>,
    pub funded: BTreeMap<StudyId, FundedStudy>,
    pub leader_targets: BTreeMap<StudyId, LeaderTargetClaim>,
}

impl ProgressionAuthority {
    pub fn new(colony_id: PlannerId) -> Result<Self, ProgressionError> {
        ProgressionCatalog::from_embedded()?;
        Ok(Self {
            schema_version: PROGRESSION_SCHEMA_VERSION,
            partition: ColonyPartitionKey {
                colony_id: colony_id.clone(),
            },
            version: 0,
            owned_studies: BTreeSet::new(),
            notes: ResearchNotesLedger::new(colony_id.clone()),
            void: VoidInsightLedger::new(colony_id),
            next_work_sequence: 1,
            retired_work_through: 0,
            work_orders: BTreeMap::new(),
            prepared: BTreeMap::new(),
            god_queued: BTreeMap::new(),
            funded: BTreeMap::new(),
            leader_targets: BTreeMap::new(),
        })
    }

    #[must_use]
    pub fn owned_capabilities(&self, catalog: &ProgressionCatalog) -> BTreeSet<CapabilityId> {
        catalog
            .founding_capabilities
            .iter()
            .cloned()
            .chain(self.owned_studies.iter().filter_map(|study_id| {
                let study = catalog.study(study_id)?;
                match &study.kind {
                    StudyKind::OrdinaryCapability { capability_id } => Some(capability_id.clone()),
                    StudyKind::HoleAxis { .. }
                    | StudyKind::BoostUnlock { .. }
                    | StudyKind::BoostDuration { .. }
                    | StudyKind::BoostEconomy { .. } => None,
                }
            }))
            .collect()
    }

    pub fn guard_operation(
        &self,
        catalog: &ProgressionCatalog,
        content_id: &ContentId,
        operation: ContentOperation,
    ) -> Result<(), ProgressionError> {
        let manifest = ContentManifest::embedded();
        let class =
            manifest_content_class(manifest, content_id).ok_or(ProgressionError::UnknownContent)?;
        if !operation_accepts_class(operation, class) {
            return Err(ProgressionError::InvalidOperationClass);
        }
        let owned = self.owned_capabilities(catalog);
        match manifest.is_operation_permitted(content_id, operation, &owned) {
            Ok(true) => Ok(()),
            Ok(false) => Err(ProgressionError::CapabilityLocked),
            Err(_) => Err(ProgressionError::UnknownContent),
        }
    }

    pub fn guard_recipe(
        &self,
        catalog: &ProgressionCatalog,
        check: &RecipeCapabilityCheck,
    ) -> Result<(), ProgressionError> {
        let manifest = ContentManifest::embedded();
        let recipe = manifest
            .recipes
            .iter()
            .find(|recipe| recipe.id == check.recipe_id)
            .ok_or(ProgressionError::UnknownContent)?;
        if !check.station_exists
            || check.station_tier < recipe.station_tier
            || !check.physical_ingredients_ready
            || !check.tools_ready
            || !check.capacity_ready
            || !check.workers_ready
        {
            return Err(ProgressionError::PhysicalPrerequisiteMissing);
        }
        let owned = self.owned_capabilities(catalog);
        if !owned.contains(&recipe.bundle_capability) {
            return Err(ProgressionError::CapabilityLocked);
        }
        let station = manifest
            .stations
            .iter()
            .find(|station| station.id == recipe.station)
            .ok_or(ProgressionError::ManifestInvalid)?;
        if station
            .canonical_capability
            .required_id()
            .is_some_and(|required| !owned.contains(required))
        {
            return Err(ProgressionError::CapabilityLocked);
        }
        for ingredient in &recipe.ingredients {
            if !manifest
                .is_operation_permitted(&ingredient.content_id, ContentOperation::Craft, &owned)
                .map_err(|_| ProgressionError::ManifestInvalid)?
            {
                return Err(ProgressionError::CapabilityLocked);
            }
        }
        Ok(())
    }

    pub fn queue_notes_work(
        &mut self,
        partition: ColonyPartitionKey,
        assignment: ScholarAssignment,
        required_work_minutes: u64,
        expected_version: u64,
    ) -> Result<ScholarWorkId, ProgressionError> {
        let credit = required_work_minutes
            .checked_mul(NOTES_MICRO_PER_WORK_MINUTE)
            .map(ResearchNotes::from_micro)
            .ok_or(ProgressionError::ArithmeticOverflow)?;
        self.queue_work(
            partition,
            assignment,
            ScholarWorkPurpose::ProduceNotes { credit },
            required_work_minutes,
            expected_version,
        )
    }

    pub fn queue_preparation_work(
        &mut self,
        catalog: &ProgressionCatalog,
        partition: PlayerPartitionKey,
        assignment: ScholarAssignment,
        study_id: StudyId,
        expected_version: u64,
    ) -> Result<ScholarWorkId, ProgressionError> {
        self.ensure_player_partition(&partition)?;
        self.ensure_lane_available(&study_id, false)?;
        let study = catalog
            .study(&study_id)
            .ok_or(ProgressionError::UnknownStudy)?;
        if !study.kind.is_ordinary() || self.owned_studies.contains(&study_id) {
            return Err(ProgressionError::PreparationIneligible);
        }
        let base = study
            .notes_cost()
            .ok_or(ProgressionError::PreparationIneligible)?;
        let discount = ResearchNotes::from_micro(base.micro() / 4);
        let payable = base.checked_sub(discount)?;
        let required_work_minutes = study.required_work_minutes.div_ceil(4);
        self.queue_work(
            ColonyPartitionKey {
                colony_id: partition.colony_id,
            },
            assignment,
            ScholarWorkPurpose::PrepareStudy {
                study_id,
                player_id: partition.player_id,
                frozen_base_cost: base,
                frozen_discount: discount,
                frozen_payable: payable,
            },
            required_work_minutes,
            expected_version,
        )
    }

    pub fn queue_funded_study_work(
        &mut self,
        catalog: &ProgressionCatalog,
        partition: PlayerPartitionKey,
        assignment: ScholarAssignment,
        study_id: StudyId,
        expected_version: u64,
    ) -> Result<ScholarWorkId, ProgressionError> {
        self.ensure_player_partition(&partition)?;
        let funded = self
            .funded
            .get(&study_id)
            .ok_or(ProgressionError::StudyNotFunded)?;
        if funded.player_id != partition.player_id {
            return Err(ProgressionError::PartitionMismatch);
        }
        if self.work_orders.values().any(|work| {
            matches!(
                &work.purpose,
                ScholarWorkPurpose::CompleteGodStudy { study_id: candidate }
                    if candidate == &study_id
            )
        }) {
            return Err(ProgressionError::DuplicateLaneClaim);
        }
        let required = catalog
            .study(&study_id)
            .ok_or(ProgressionError::UnknownStudy)?
            .required_work_minutes;
        self.queue_work(
            ColonyPartitionKey {
                colony_id: partition.colony_id,
            },
            assignment,
            ScholarWorkPurpose::CompleteGodStudy { study_id },
            required,
            expected_version,
        )
    }

    fn queue_work(
        &mut self,
        partition: ColonyPartitionKey,
        assignment: ScholarAssignment,
        purpose: ScholarWorkPurpose,
        required_work_minutes: u64,
        expected_version: u64,
    ) -> Result<ScholarWorkId, ProgressionError> {
        if partition != self.partition {
            return Err(ProgressionError::PartitionMismatch);
        }
        if expected_version != self.version {
            return Err(ProgressionError::StaleVersion);
        }
        if required_work_minutes == 0 {
            return Err(ProgressionError::MalformedRequest);
        }
        if self.work_orders.len() >= MAX_ACTIVE_SCHOLAR_WORK {
            return Err(ProgressionError::Backpressure);
        }
        let mut next = self.clone();
        let id = ScholarWorkId {
            colony_id: partition.colony_id,
            sequence: next.next_work_sequence,
        };
        let fingerprint = work_fingerprint(&id, &assignment, &purpose, required_work_minutes);
        next.work_orders.insert(
            id.sequence,
            ScholarWorkOrder {
                id: id.clone(),
                order_id: ScholarOrderId::derive(&id),
                task_id: ScholarTaskId::derive(&id),
                assignment,
                purpose,
                required_work_minutes,
                progress_work_minutes: 0,
                stage: ScholarWorkStage::Queued,
                request_fingerprint: fingerprint,
            },
        );
        next.next_work_sequence = next
            .next_work_sequence
            .checked_add(1)
            .ok_or(ProgressionError::ArithmeticOverflow)?;
        next.bump_version()?;
        next.validate()?;
        *self = next;
        Ok(id)
    }

    pub fn reserve_work(
        &mut self,
        id: &ScholarWorkId,
        expected_version: u64,
    ) -> Result<(), ProgressionError> {
        self.transition_work(
            id,
            expected_version,
            ScholarWorkStage::Queued,
            ScholarWorkStage::Reserved,
        )
    }

    pub fn start_work(
        &mut self,
        id: &ScholarWorkId,
        expected_version: u64,
    ) -> Result<(), ProgressionError> {
        self.transition_work(
            id,
            expected_version,
            ScholarWorkStage::Reserved,
            ScholarWorkStage::Working,
        )
    }

    fn transition_work(
        &mut self,
        id: &ScholarWorkId,
        expected_version: u64,
        from: ScholarWorkStage,
        to: ScholarWorkStage,
    ) -> Result<(), ProgressionError> {
        self.ensure_work_partition(id)?;
        if expected_version != self.version {
            return Err(ProgressionError::StaleVersion);
        }
        let mut next = self.clone();
        let work = next
            .work_orders
            .get_mut(&id.sequence)
            .ok_or_else(|| self.retired_or_unknown(id))?;
        if work.id != *id || work.stage != from {
            return Err(ProgressionError::InvalidWorkStage);
        }
        work.stage = to;
        next.bump_version()?;
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn progress_work(
        &mut self,
        id: &ScholarWorkId,
        completed_minutes: u64,
        expected_version: u64,
    ) -> Result<ScholarWorkStage, ProgressionError> {
        self.ensure_work_partition(id)?;
        if expected_version != self.version {
            return Err(ProgressionError::StaleVersion);
        }
        if completed_minutes == 0 {
            return Err(ProgressionError::MalformedRequest);
        }
        let mut next = self.clone();
        let work = next
            .work_orders
            .get_mut(&id.sequence)
            .ok_or_else(|| self.retired_or_unknown(id))?;
        if work.id != *id || work.stage != ScholarWorkStage::Working {
            return Err(ProgressionError::InvalidWorkStage);
        }
        work.progress_work_minutes = work
            .progress_work_minutes
            .checked_add(completed_minutes)
            .ok_or(ProgressionError::ArithmeticOverflow)?
            .min(work.required_work_minutes);
        if work.progress_work_minutes == work.required_work_minutes {
            work.stage = ScholarWorkStage::Completed;
            let purpose = work.purpose.clone();
            next.complete_work_purpose(id, purpose)?;
        }
        let stage = next
            .work_orders
            .get(&id.sequence)
            .ok_or(ProgressionError::MalformedPersistence)?
            .stage;
        next.bump_version()?;
        next.validate()?;
        *self = next;
        Ok(stage)
    }

    fn complete_work_purpose(
        &mut self,
        id: &ScholarWorkId,
        purpose: ScholarWorkPurpose,
    ) -> Result<(), ProgressionError> {
        match purpose {
            ScholarWorkPurpose::ProduceNotes { .. } => {}
            ScholarWorkPurpose::PrepareStudy {
                study_id,
                player_id,
                frozen_base_cost,
                frozen_discount,
                frozen_payable,
            } => {
                self.ensure_lane_available(&study_id, false)?;
                self.prepared.insert(
                    study_id.clone(),
                    PreparationRecord {
                        study_id,
                        player_id,
                        completed_work_id: id.clone(),
                        frozen_base_cost,
                        frozen_discount,
                        frozen_payable,
                    },
                );
            }
            ScholarWorkPurpose::CompleteGodStudy { study_id } => {
                if self.leader_targets.contains_key(&study_id) {
                    return Err(ProgressionError::DuplicateResolutionPending);
                }
                self.funded
                    .remove(&study_id)
                    .ok_or(ProgressionError::StudyNotFunded)?;
                self.owned_studies.insert(study_id);
            }
        }
        Ok(())
    }

    pub fn recover_work(
        &mut self,
        id: &ScholarWorkId,
        reason: WorkRecoveryReason,
        expected_version: u64,
    ) -> Result<WorkRelease, ProgressionError> {
        self.ensure_work_partition(id)?;
        if expected_version != self.version {
            return Err(ProgressionError::StaleVersion);
        }
        let mut next = self.clone();
        let work = next
            .work_orders
            .get_mut(&id.sequence)
            .ok_or_else(|| self.retired_or_unknown(id))?;
        if work.id != *id || work.stage.terminal() {
            return Err(ProgressionError::InvalidWorkStage);
        }
        let release = WorkRelease {
            work_id: id.clone(),
            assignment: work.assignment.clone(),
            reason,
            released_reservation: matches!(
                work.stage,
                ScholarWorkStage::Reserved | ScholarWorkStage::Working
            ),
            lost_progress_work_minutes: work.progress_work_minutes,
        };
        work.stage = ScholarWorkStage::Cancelled;
        next.bump_version()?;
        next.validate()?;
        *self = next;
        Ok(release)
    }

    pub fn drain_terminal_work(
        &mut self,
        limit: usize,
    ) -> Result<Vec<ResearchNotesCreditPayload>, ProgressionError> {
        if limit == 0 || limit > MAX_DRAIN_BATCH {
            return Err(ProgressionError::CapacityExceeded);
        }
        let mut next = self.clone();
        let mut credits = Vec::new();
        for _ in 0..limit {
            let sequence = next
                .retired_work_through
                .checked_add(1)
                .ok_or(ProgressionError::ArithmeticOverflow)?;
            let Some(work) = next.work_orders.get(&sequence).cloned() else {
                break;
            };
            if !work.stage.terminal() {
                break;
            }
            let credit = match (&work.stage, &work.purpose) {
                (ScholarWorkStage::Completed, ScholarWorkPurpose::ProduceNotes { credit }) => {
                    Some(*credit)
                }
                _ => None,
            };
            next.notes.advance_scholar_work(sequence, credit)?;
            if let Some(amount) = credit {
                credits.push(ResearchNotesCreditPayload {
                    partition: next.partition.clone(),
                    work_id: work.id.clone(),
                    amount,
                });
            }
            next.work_orders.remove(&sequence);
            next.retired_work_through = sequence;
        }
        if credits.is_empty() && next.retired_work_through == self.retired_work_through {
            return Ok(credits);
        }
        next.bump_version()?;
        next.validate()?;
        *self = next;
        Ok(credits)
    }

    pub fn claim_god_queue(
        &mut self,
        catalog: &ProgressionCatalog,
        partition: PlayerPartitionKey,
        study_id: StudyId,
        expected_version: u64,
    ) -> Result<(), ProgressionError> {
        self.ensure_player_partition(&partition)?;
        if expected_version != self.version {
            return Err(ProgressionError::StaleVersion);
        }
        if self.owned_studies.contains(&study_id)
            || self.funded.contains_key(&study_id)
            || self.god_queued.contains_key(&study_id)
            || self.leader_targets.contains_key(&study_id)
        {
            return Err(ProgressionError::DuplicateLaneClaim);
        }
        let study = catalog
            .study(&study_id)
            .ok_or(ProgressionError::UnknownStudy)?;
        if study
            .prerequisites
            .iter()
            .any(|prerequisite| !self.owned_studies.contains(prerequisite))
        {
            return Err(ProgressionError::PrerequisiteLocked);
        }
        let mut next = self.clone();
        let prepared_terms = next.prepared.remove(&study_id);
        if prepared_terms
            .as_ref()
            .is_some_and(|prepared| prepared.player_id != partition.player_id)
        {
            return Err(ProgressionError::PartitionMismatch);
        }
        next.god_queued.insert(
            study_id.clone(),
            GodQueueClaim {
                study_id,
                player_id: partition.player_id,
                prepared_terms,
            },
        );
        next.bump_version()?;
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn fund_god_study(
        &mut self,
        catalog: &ProgressionCatalog,
        partition: PlayerPartitionKey,
        study_id: StudyId,
        funding_event_id: CurrencyEventId,
        expected_version: u64,
    ) -> Result<(), ProgressionError> {
        self.ensure_player_partition(&partition)?;
        if expected_version != self.version {
            return Err(ProgressionError::StaleVersion);
        }
        let study = catalog
            .study(&study_id)
            .ok_or(ProgressionError::UnknownStudy)?;
        let claim = self
            .god_queued
            .get(&study_id)
            .ok_or(ProgressionError::StudyNotQueued)?;
        if claim.player_id != partition.player_id {
            return Err(ProgressionError::PartitionMismatch);
        }
        if claim.prepared_terms.is_some() && !study.kind.is_ordinary() {
            return Err(ProgressionError::PreparationIneligible);
        }
        let mut next = self.clone();
        let claim = next
            .god_queued
            .remove(&study_id)
            .ok_or(ProgressionError::StudyNotQueued)?;
        let fingerprint = funding_fingerprint(
            &partition,
            &study_id,
            &funding_event_id,
            claim.prepared_terms.as_ref(),
        );
        let (currency, paid_micro) = match study.currency() {
            StudyCurrency::Notes => {
                let amount = claim.prepared_terms.as_ref().map_or_else(
                    || study.notes_cost().expect("currency checked"),
                    |record| record.frozen_payable,
                );
                let expected_notes_version = next.notes.version;
                next.notes.debit_inner(ResearchNotesSpendRequest {
                    id: funding_event_id.clone(),
                    amount,
                    expected_version: expected_notes_version,
                    fingerprint,
                })?;
                (StudyCurrency::Notes, amount.micro())
            }
            StudyCurrency::Void => {
                let amount = study.void_cost().expect("currency checked");
                let expected_void_version = next.void.version;
                let purpose = if matches!(study.kind, StudyKind::HoleAxis { .. }) {
                    VoidDebitPurpose::HoleStudy
                } else {
                    VoidDebitPurpose::BoostStudy
                };
                next.void.debit_inner(VoidSpendRequest {
                    id: funding_event_id.clone(),
                    amount,
                    purpose,
                    expected_version: expected_void_version,
                    fingerprint,
                })?;
                (StudyCurrency::Void, amount.micro())
            }
        };
        next.funded.insert(
            study_id.clone(),
            FundedStudy {
                study_id,
                player_id: partition.player_id,
                currency,
                paid_micro,
                preparation_consumed: claim.prepared_terms.is_some(),
                funding_event_id,
            },
        );
        next.bump_version()?;
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn leader_target_decision(
        &self,
        study_id: &StudyId,
        hook: &LeaderDuplicateHook,
    ) -> Result<LeaderTargetDecision, ProgressionError> {
        let duplicate = self.prepared.contains_key(study_id)
            || self.god_queued.contains_key(study_id)
            || self.funded.contains_key(study_id);
        if self.owned_studies.contains(study_id) || self.leader_targets.contains_key(study_id) {
            return Ok(LeaderTargetDecision::ChooseAnother);
        }
        if !duplicate {
            return Ok(LeaderTargetDecision::Allowed);
        }
        match hook {
            LeaderDuplicateHook::None => Ok(LeaderTargetDecision::ChooseAnother),
            LeaderDuplicateHook::VillageCritical {
                report_marks_critical,
                needed_before_tick,
                estimated_god_completion_tick,
            } if *report_marks_critical && needed_before_tick < estimated_god_completion_tick => {
                Ok(LeaderTargetDecision::VillageCriticalOverride)
            }
            LeaderDuplicateHook::KeyedOopsie {
                effective_level,
                keyed_roll_percent,
                ..
            } if usize::from(*effective_level) < OOPSIE_PERCENT_BY_LEVEL.len()
                && *keyed_roll_percent < OOPSIE_PERCENT_BY_LEVEL[usize::from(*effective_level)] =>
            {
                Ok(LeaderTargetDecision::KeyedOopsieOverride)
            }
            LeaderDuplicateHook::VillageCritical { .. }
            | LeaderDuplicateHook::KeyedOopsie { .. } => Ok(LeaderTargetDecision::ChooseAnother),
        }
    }

    pub fn select_leader_target(
        &self,
        candidates: &[StudyId],
        hooks: &BTreeMap<StudyId, LeaderDuplicateHook>,
    ) -> Result<Option<(StudyId, LeaderTargetDecision)>, ProgressionError> {
        for candidate in candidates {
            let hook = hooks.get(candidate).unwrap_or(&LeaderDuplicateHook::None);
            let decision = self.leader_target_decision(candidate, hook)?;
            if decision != LeaderTargetDecision::ChooseAnother {
                return Ok(Some((candidate.clone(), decision)));
            }
        }
        Ok(None)
    }

    pub fn mark_leader_target(
        &mut self,
        catalog: &ProgressionCatalog,
        study_id: StudyId,
        hook: LeaderDuplicateHook,
        expected_version: u64,
    ) -> Result<LeaderTargetDecision, ProgressionError> {
        if expected_version != self.version {
            return Err(ProgressionError::StaleVersion);
        }
        let study = catalog
            .study(&study_id)
            .ok_or(ProgressionError::UnknownStudy)?;
        if study
            .prerequisites
            .iter()
            .any(|prerequisite| !self.owned_studies.contains(prerequisite))
        {
            return Err(ProgressionError::PrerequisiteLocked);
        }
        let decision = self.leader_target_decision(&study_id, &hook)?;
        if decision == LeaderTargetDecision::ChooseAnother {
            return Err(ProgressionError::DuplicateLaneClaim);
        }
        let (duplicate_kind, decision_key) = match hook {
            LeaderDuplicateHook::None => (None, None),
            LeaderDuplicateHook::VillageCritical { .. } => {
                (Some(LeaderDuplicateKind::VillageCritical), None)
            }
            LeaderDuplicateHook::KeyedOopsie { decision_key, .. } => {
                (Some(LeaderDuplicateKind::KeyedOopsie), Some(decision_key))
            }
        };
        let mut next = self.clone();
        next.leader_targets.insert(
            study_id.clone(),
            LeaderTargetClaim {
                study_id,
                duplicate_kind,
                decision_key,
            },
        );
        next.bump_version()?;
        next.validate()?;
        *self = next;
        Ok(decision)
    }

    pub fn complete_free_leader_target(
        &mut self,
        study_id: &StudyId,
        expected_version: u64,
    ) -> Result<(), ProgressionError> {
        if expected_version != self.version {
            return Err(ProgressionError::StaleVersion);
        }
        let claim = self
            .leader_targets
            .get(study_id)
            .ok_or(ProgressionError::StudyNotLeaderTargeted)?;
        if claim.duplicate_kind.is_some() {
            return Err(ProgressionError::DuplicateResolutionPending);
        }
        let mut next = self.clone();
        next.leader_targets.remove(study_id);
        next.owned_studies.insert(study_id.clone());
        next.bump_version()?;
        next.validate()?;
        *self = next;
        Ok(())
    }

    fn ensure_lane_available(
        &self,
        study_id: &StudyId,
        allow_prepared: bool,
    ) -> Result<(), ProgressionError> {
        if self.owned_studies.contains(study_id)
            || (!allow_prepared && self.prepared.contains_key(study_id))
            || self.god_queued.contains_key(study_id)
            || self.funded.contains_key(study_id)
            || self.leader_targets.contains_key(study_id)
        {
            return Err(ProgressionError::DuplicateLaneClaim);
        }
        Ok(())
    }

    fn ensure_player_partition(
        &self,
        partition: &PlayerPartitionKey,
    ) -> Result<(), ProgressionError> {
        if partition.colony_id != self.partition.colony_id {
            Err(ProgressionError::PartitionMismatch)
        } else {
            Ok(())
        }
    }

    fn ensure_work_partition(&self, id: &ScholarWorkId) -> Result<(), ProgressionError> {
        if id.colony_id != self.partition.colony_id {
            Err(ProgressionError::PartitionMismatch)
        } else {
            Ok(())
        }
    }

    fn retired_or_unknown(&self, id: &ScholarWorkId) -> ProgressionError {
        if id.sequence <= self.retired_work_through {
            ProgressionError::RetiredReplay
        } else {
            ProgressionError::UnknownWork
        }
    }

    fn bump_version(&mut self) -> Result<(), ProgressionError> {
        self.version = self
            .version
            .checked_add(1)
            .ok_or(ProgressionError::ArithmeticOverflow)?;
        Ok(())
    }

    fn validate(&self) -> Result<(), ProgressionError> {
        let catalog = ProgressionCatalog::from_embedded()?;
        if self.schema_version != PROGRESSION_SCHEMA_VERSION
            || self.partition != self.notes.partition
            || self.partition != self.void.partition
            || self.work_orders.len() > MAX_ACTIVE_SCHOLAR_WORK
            || self.prepared.len() > MAX_LANE_CLAIMS
            || self.god_queued.len() > MAX_LANE_CLAIMS
            || self.funded.len() > MAX_LANE_CLAIMS
            || self.leader_targets.len() > MAX_LANE_CLAIMS
            || self.next_work_sequence == 0
            || self.retired_work_through >= self.next_work_sequence
            || self.notes.retired_work_through != self.retired_work_through
        {
            return Err(ProgressionError::MalformedPersistence);
        }
        self.notes.validate()?;
        self.void.validate()?;
        let first_live_sequence = self
            .retired_work_through
            .checked_add(1)
            .ok_or(ProgressionError::MalformedPersistence)?;
        let last_live_sequence = self
            .next_work_sequence
            .checked_sub(1)
            .ok_or(ProgressionError::MalformedPersistence)?;
        let expected_sequences = if first_live_sequence > last_live_sequence {
            BTreeSet::new()
        } else {
            (first_live_sequence..=last_live_sequence).collect()
        };
        if self.work_orders.keys().copied().collect::<BTreeSet<_>>() != expected_sequences {
            return Err(ProgressionError::MalformedPersistence);
        }
        if self.owned_studies.iter().any(|id| {
            catalog.study(id).is_none_or(|study| {
                study
                    .prerequisites
                    .iter()
                    .any(|prerequisite| !self.owned_studies.contains(prerequisite))
            })
        }) {
            return Err(ProgressionError::MalformedPersistence);
        }
        for (sequence, work) in &self.work_orders {
            if *sequence != work.id.sequence
                || work.id.colony_id != self.partition.colony_id
                || *sequence <= self.retired_work_through
                || *sequence >= self.next_work_sequence
                || work.required_work_minutes == 0
                || work.progress_work_minutes > work.required_work_minutes
                || work.stage == ScholarWorkStage::Completed
                    && work.progress_work_minutes != work.required_work_minutes
                || work.stage != ScholarWorkStage::Completed
                    && work.progress_work_minutes == work.required_work_minutes
                || work.order_id != ScholarOrderId::derive(&work.id)
                || work.task_id != ScholarTaskId::derive(&work.id)
                || work.request_fingerprint
                    != work_fingerprint(
                        &work.id,
                        &work.assignment,
                        &work.purpose,
                        work.required_work_minutes,
                    )
            {
                return Err(ProgressionError::MalformedPersistence);
            }
        }
        for (id, record) in &self.prepared {
            if id != &record.study_id
                || record.frozen_base_cost == ResearchNotes::ZERO
                || record.frozen_discount.micro() != record.frozen_base_cost.micro() / 4
                || record.frozen_payable.micro()
                    != record.frozen_base_cost.micro() - record.frozen_discount.micro()
                || !catalog
                    .study(id)
                    .is_some_and(|study| study.kind.is_ordinary())
            {
                return Err(ProgressionError::MalformedPersistence);
            }
        }
        for (id, claim) in &self.god_queued {
            if id != &claim.study_id || catalog.study(id).is_none() {
                return Err(ProgressionError::MalformedPersistence);
            }
        }
        for (id, funded) in &self.funded {
            if id != &funded.study_id
                || funded.paid_micro == 0
                || catalog
                    .study(id)
                    .is_none_or(|study| study.currency() != funded.currency)
            {
                return Err(ProgressionError::MalformedPersistence);
            }
        }
        for (id, claim) in &self.leader_targets {
            if id != &claim.study_id
                || catalog.study(id).is_none()
                || claim.duplicate_kind == Some(LeaderDuplicateKind::KeyedOopsie)
                    && claim.decision_key.is_none()
            {
                return Err(ProgressionError::MalformedPersistence);
            }
        }
        for id in catalog.studies.keys() {
            let statuses = usize::from(self.owned_studies.contains(id))
                + usize::from(self.prepared.contains_key(id))
                + usize::from(self.god_queued.contains_key(id))
                + usize::from(self.funded.contains_key(id))
                + usize::from(self.leader_targets.contains_key(id));
            let override_present = self
                .leader_targets
                .get(id)
                .is_some_and(|claim| claim.duplicate_kind.is_some());
            if statuses > 1 && !(statuses == 2 && override_present) {
                return Err(ProgressionError::MalformedPersistence);
            }
        }
        Ok(())
    }
}

fn manifest_content_class(
    manifest: &ContentManifest,
    content_id: &ContentId,
) -> Option<ManifestContentClass> {
    [
        (
            ManifestContentClass::Resource,
            manifest
                .resources
                .iter()
                .any(|entry| &entry.content_id == content_id),
        ),
        (
            ManifestContentClass::Food,
            manifest
                .foods
                .iter()
                .any(|entry| &entry.content_id == content_id),
        ),
        (
            ManifestContentClass::Item,
            manifest
                .item_definitions
                .iter()
                .any(|entry| &entry.content_id == content_id),
        ),
        (
            ManifestContentClass::Material,
            manifest
                .materials
                .iter()
                .any(|entry| &entry.content_id == content_id),
        ),
        (
            ManifestContentClass::Creature,
            manifest
                .creatures
                .iter()
                .any(|entry| &entry.content_id == content_id),
        ),
        (
            ManifestContentClass::Station,
            manifest
                .stations
                .iter()
                .any(|entry| &entry.content_id == content_id),
        ),
        (
            ManifestContentClass::Recipe,
            manifest
                .recipes
                .iter()
                .any(|entry| &entry.content_id == content_id),
        ),
        (
            ManifestContentClass::Augmentation,
            manifest
                .augmentations
                .iter()
                .any(|entry| &entry.content_id == content_id),
        ),
        (
            ManifestContentClass::Fixture,
            manifest
                .fixtures
                .iter()
                .any(|entry| &entry.content_id == content_id),
        ),
    ]
    .into_iter()
    .find_map(|(class, present)| present.then_some(class))
}

const fn operation_accepts_class(operation: ContentOperation, class: ManifestContentClass) -> bool {
    match operation {
        ContentOperation::Discover | ContentOperation::Store | ContentOperation::Trade => true,
        ContentOperation::Process => matches!(
            class,
            ManifestContentClass::Resource
                | ManifestContentClass::Food
                | ManifestContentClass::Material
        ),
        ContentOperation::Craft => matches!(
            class,
            ManifestContentClass::Resource
                | ManifestContentClass::Food
                | ManifestContentClass::Item
                | ManifestContentClass::Material
                | ManifestContentClass::Recipe
                | ManifestContentClass::Augmentation
                | ManifestContentClass::Fixture
        ),
        ContentOperation::InstallFixture => matches!(class, ManifestContentClass::Fixture),
        ContentOperation::Augment => matches!(
            class,
            ManifestContentClass::Item | ManifestContentClass::Augmentation
        ),
        ContentOperation::FeedHole => matches!(
            class,
            ManifestContentClass::Resource
                | ManifestContentClass::Food
                | ManifestContentClass::Item
                | ManifestContentClass::Material
                | ManifestContentClass::Augmentation
                | ManifestContentClass::Fixture
        ),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UncheckedProgressionAuthority {
    schema_version: u32,
    partition: ColonyPartitionKey,
    version: u64,
    owned_studies: BTreeSet<StudyId>,
    notes: ResearchNotesLedger,
    void: VoidInsightLedger,
    next_work_sequence: u64,
    retired_work_through: u64,
    work_orders: BTreeMap<u64, ScholarWorkOrder>,
    prepared: BTreeMap<StudyId, PreparationRecord>,
    god_queued: BTreeMap<StudyId, GodQueueClaim>,
    funded: BTreeMap<StudyId, FundedStudy>,
    leader_targets: BTreeMap<StudyId, LeaderTargetClaim>,
}

impl<'de> Deserialize<'de> for ProgressionAuthority {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = UncheckedProgressionAuthority::deserialize(deserializer)?;
        let state = Self {
            schema_version: raw.schema_version,
            partition: raw.partition,
            version: raw.version,
            owned_studies: raw.owned_studies,
            notes: raw.notes,
            void: raw.void,
            next_work_sequence: raw.next_work_sequence,
            retired_work_through: raw.retired_work_through,
            work_orders: raw.work_orders,
            prepared: raw.prepared,
            god_queued: raw.god_queued,
            funded: raw.funded,
            leader_targets: raw.leader_targets,
        };
        state.validate().map_err(serde::de::Error::custom)?;
        Ok(state)
    }
}

fn work_fingerprint(
    id: &ScholarWorkId,
    assignment: &ScholarAssignment,
    purpose: &ScholarWorkPurpose,
    required_work_minutes: u64,
) -> u64 {
    let mut fingerprint = StableFingerprint::new();
    fingerprint.write(id.colony_id.as_str());
    fingerprint.write_u64(id.sequence);
    fingerprint.write(assignment.scholar_id.as_str());
    fingerprint.write(assignment.tool_id.as_str());
    fingerprint.write(assignment.station_id.as_str());
    fingerprint.write(assignment.location_id.as_str());
    fingerprint.write_u64(required_work_minutes);
    match purpose {
        ScholarWorkPurpose::ProduceNotes { credit } => {
            fingerprint.write("notes");
            fingerprint.write_u64(credit.micro());
        }
        ScholarWorkPurpose::PrepareStudy {
            study_id,
            player_id,
            frozen_base_cost,
            frozen_discount,
            frozen_payable,
        } => {
            fingerprint.write("prepare");
            fingerprint.write(study_id.as_str());
            fingerprint.write(player_id.as_str());
            fingerprint.write_u64(frozen_base_cost.micro());
            fingerprint.write_u64(frozen_discount.micro());
            fingerprint.write_u64(frozen_payable.micro());
        }
        ScholarWorkPurpose::CompleteGodStudy { study_id } => {
            fingerprint.write("god_study");
            fingerprint.write(study_id.as_str());
        }
    }
    fingerprint.finish()
}

fn funding_fingerprint(
    partition: &PlayerPartitionKey,
    study_id: &StudyId,
    event_id: &CurrencyEventId,
    preparation: Option<&PreparationRecord>,
) -> u64 {
    let mut fingerprint = StableFingerprint::new();
    fingerprint.write(partition.colony_id.as_str());
    fingerprint.write(partition.player_id.as_str());
    fingerprint.write(study_id.as_str());
    fingerprint.write(event_id.as_str());
    if let Some(preparation) = preparation {
        fingerprint.write_u64(preparation.completed_work_id.sequence);
        fingerprint.write_u64(preparation.frozen_payable.micro());
    }
    fingerprint.finish()
}

struct StableFingerprint(u64);

impl StableFingerprint {
    const fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn write(&mut self, value: &str) {
        for byte in value.as_bytes() {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
        self.0 ^= 0xff;
        self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
    }

    fn write_u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    const fn finish(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressionError {
    ManifestInvalid,
    MalformedId,
    MalformedRequest,
    MalformedPersistence,
    UnknownContent,
    UnknownStudy,
    UnknownWork,
    CapabilityLocked,
    InvalidOperationClass,
    PhysicalPrerequisiteMissing,
    PrerequisiteLocked,
    PreparationIneligible,
    DuplicateLaneClaim,
    DuplicateResolutionPending,
    StudyNotQueued,
    StudyNotFunded,
    StudyNotLeaderTargeted,
    InvalidWorkStage,
    PartitionMismatch,
    StaleVersion,
    NonCanonicalSequence,
    IdempotencyConflict,
    RetiredReplay,
    InsufficientCurrency,
    Backpressure,
    CapacityExceeded,
    ArithmeticOverflow,
}

impl fmt::Display for ProgressionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "progression request rejected ({self:?})")
    }
}

impl std::error::Error for ProgressionError {}
