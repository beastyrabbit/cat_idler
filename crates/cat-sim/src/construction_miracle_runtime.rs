//! Canonical construction-miracle witness, package, and runtime bridge.
//!
//! The bridge derives every mutable fact from the persisted construction,
//! storage, Hole, and Void authorities. Generated lots are deposited directly
//! into the target project's construction cargo and remain purpose-bound.
//! Labor reductions which target a not-yet-open labor stage are persisted as a
//! bounded credit and consumed exactly once when that stage opens.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    black_hole::{HoleError, canonical_construction_input_unit_value_micros},
    construction_stages::{
        ConstructionProject, ConstructionStage, ConstructionStageBill, stage_work_durations,
    },
    content_manifest::{ConstructionMiracleInputClass, ContentId, ContentManifest},
    divine_hole_authority::{
        ConstructionMiracleRequest, DivineHoleError, MiracleInput, MiracleLaborStage, VoidAction,
        VoidActionEnvelope, VoidActionOutcome,
    },
    food_divine_policy::{BoundCargoPurpose, MIRACLE_INPUT_VALUE_MULTIPLIER},
    leader_ai_runtime::{LeaderAiRuntimeError, LeaderAiRuntimeState},
    progression_research::VoidInsight,
    storage_authority::{StorageAddress, StorageIdentity},
};

pub const CONSTRUCTION_MIRACLE_RUNTIME_VERSION: u32 = 1;
pub const MAX_CONSTRUCTION_MIRACLE_RECEIPTS: usize = 512;
pub const MAX_CONSTRUCTION_MIRACLE_VALUE_ENTRIES: usize = 64;
const MAX_COMPOSITION_STATES: usize = 32_768;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalConstructionInputValue {
    definition_id: String,
    unit_value_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanonicalConstructionInputValueTable {
    hole_feed_value_per_void_micros: u64,
    entries: Vec<CanonicalConstructionInputValue>,
}

impl CanonicalConstructionInputValueTable {
    fn from_manifest(
        missing: &[MissingConstructionInput],
    ) -> Result<Self, ConstructionMiracleRuntimeError> {
        let manifest = ContentManifest::embedded();
        let mut entries = Vec::new();
        for input in missing {
            let content_id = ContentId::new(&input.definition_id).map_err(|_| {
                ConstructionMiracleRuntimeError::MissingManifestInputClassification(
                    input.definition_id.clone(),
                )
            })?;
            let descriptor = manifest
                .construction_miracle_input(&content_id)
                .ok_or_else(|| {
                    ConstructionMiracleRuntimeError::MissingManifestInputClassification(
                        input.definition_id.clone(),
                    )
                })?;
            match descriptor.physical_class {
                ConstructionMiracleInputClass::BulkLot
                | ConstructionMiracleInputClass::ExactItem
                | ConstructionMiracleInputClass::Fixture => {
                    entries.push(CanonicalConstructionInputValue {
                        definition_id: input.definition_id.clone(),
                        unit_value_micros: canonical_construction_input_unit_value_micros(
                            manifest,
                            &content_id,
                        )?,
                    });
                }
                ConstructionMiracleInputClass::Ineligible => {}
            }
        }
        entries.sort_by(|left, right| left.definition_id.cmp(&right.definition_id));
        if entries.is_empty() {
            return Err(ConstructionMiracleRuntimeError::NoEligibleManifestInput);
        }
        let table = Self {
            hole_feed_value_per_void_micros: VoidInsight::ONE.micro(),
            entries,
        };
        table.validate()?;
        Ok(table)
    }

    pub fn validate(&self) -> Result<(), ConstructionMiracleRuntimeError> {
        if self.hole_feed_value_per_void_micros == 0
            || self.entries.is_empty()
            || self.entries.len() > MAX_CONSTRUCTION_MIRACLE_VALUE_ENTRIES
            || self
                .entries
                .iter()
                .any(|entry| entry.definition_id.trim().is_empty() || entry.unit_value_micros == 0)
            || self
                .entries
                .windows(2)
                .any(|pair| pair[0].definition_id >= pair[1].definition_id)
        {
            return Err(ConstructionMiracleRuntimeError::InvalidValueTable);
        }
        Ok(())
    }

    fn unit_value(&self, definition_id: &str) -> Option<u64> {
        self.entries
            .binary_search_by(|entry| entry.definition_id.as_str().cmp(definition_id))
            .ok()
            .map(|index| self.entries[index].unit_value_micros)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyConstructionMiracle {
    pub command_id: String,
    pub project_id: String,
    pub player_id: String,
    pub expected_authority_version: u64,
    pub expected_void_version: u64,
    pub now_real_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MissingConstructionInput {
    pub stage_index: u8,
    pub definition_id: String,
    pub missing_quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConstructionMiracleWitness {
    pub project_id: String,
    pub original_total_work_ms: u64,
    pub earliest_incomplete_stage_index: u8,
    pub ordered_remaining_labor_stages: Vec<MiracleLaborStage>,
    pub exact_missing_bound_inputs: Vec<MissingConstructionInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConstructionMiracleRuntimeOutcome {
    pub void_outcome: VoidActionOutcome,
    pub construction_storage_identities: Vec<StorageIdentity>,
    pub labor_credit_added_by_stage_ms: BTreeMap<u8, u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConstructionMiracleReplayIdentity {
    project_id: String,
    player_id: String,
    now_real_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConstructionMiracleRuntimeReceipt {
    command_id: String,
    replay_identity: ConstructionMiracleReplayIdentity,
    outcome: ConstructionMiracleRuntimeOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConstructionMiracleRuntimeState {
    pub version: u32,
    pending_labor_credit_ms: BTreeMap<String, BTreeMap<u8, u64>>,
    receipts: BTreeMap<String, ConstructionMiracleRuntimeReceipt>,
}

impl ConstructionMiracleRuntimeState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            version: CONSTRUCTION_MIRACLE_RUNTIME_VERSION,
            pending_labor_credit_ms: BTreeMap::new(),
            receipts: BTreeMap::new(),
        }
    }

    pub fn validate(
        &self,
        projects: &BTreeMap<String, ConstructionProject>,
    ) -> Result<(), ConstructionMiracleRuntimeError> {
        if self.version != CONSTRUCTION_MIRACLE_RUNTIME_VERSION
            || self.pending_labor_credit_ms.len() > projects.len()
            || self.receipts.len() > MAX_CONSTRUCTION_MIRACLE_RECEIPTS
            || self.receipts.iter().any(|(command_id, receipt)| {
                command_id.trim().is_empty()
                    || receipt.command_id != *command_id
                    || receipt.replay_identity.project_id.trim().is_empty()
                    || receipt.replay_identity.player_id.trim().is_empty()
            })
        {
            return Err(ConstructionMiracleRuntimeError::MalformedState);
        }
        for receipt in self.receipts.values() {
            let outcome = &receipt.outcome;
            let credited_total = outcome.labor_credit_added_by_stage_ms.iter().try_fold(
                0_u64,
                |total, (stage_index, credit)| {
                    if *stage_index > 2 || *credit == 0 {
                        return Err(ConstructionMiracleRuntimeError::MalformedState);
                    }
                    total
                        .checked_add(*credit)
                        .ok_or(ConstructionMiracleRuntimeError::ArithmeticOverflow)
                },
            )?;
            if outcome.void_outcome.command_id != receipt.command_id
                || outcome.void_outcome.void_event_id.trim().is_empty()
                || expected_materialized_identity_count(&outcome.void_outcome.generated_cargo)
                    .is_none_or(|expected| {
                        expected != outcome.construction_storage_identities.len()
                    })
                || outcome
                    .construction_storage_identities
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
                || credited_total != outcome.void_outcome.labor_work_removed_ms
                || outcome.void_outcome.generated_cargo.iter().any(|cargo| {
                    !matches!(
                        &cargo.purpose,
                        BoundCargoPurpose::Construction { project_id, .. }
                            if project_id == &receipt.replay_identity.project_id
                    )
                })
            {
                return Err(ConstructionMiracleRuntimeError::MalformedState);
            }
        }
        for (project_id, credits) in &self.pending_labor_credit_ms {
            let project = projects
                .get(project_id)
                .ok_or(ConstructionMiracleRuntimeError::MissingProject)?;
            let earliest = earliest_incomplete_stage_index(project)
                .ok_or(ConstructionMiracleRuntimeError::TerminalProject)?;
            let durations = labor_durations(project.original_total_work_ms);
            if credits.is_empty()
                || credits.len() > 3
                || credits.iter().any(|(stage_index, credit)| {
                    *stage_index < earliest
                        || usize::from(*stage_index) >= durations.len()
                        || *credit == 0
                        || *credit > durations[usize::from(*stage_index)]
                        || (project.stage.is_labor() && *stage_index == earliest)
                })
            {
                return Err(ConstructionMiracleRuntimeError::MalformedState);
            }
        }
        Ok(())
    }

    pub(crate) fn pending_credit(&self, project_id: &str, stage_index: u8) -> u64 {
        self.pending_labor_credit_ms
            .get(project_id)
            .and_then(|credits| credits.get(&stage_index))
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn take_pending_credit(&mut self, project_id: &str, stage_index: u8) -> u64 {
        let Some(credits) = self.pending_labor_credit_ms.get_mut(project_id) else {
            return 0;
        };
        let credit = credits.remove(&stage_index).unwrap_or(0);
        if credits.is_empty() {
            self.pending_labor_credit_ms.remove(project_id);
        }
        credit
    }

    fn add_pending_credit(
        &mut self,
        project_id: &str,
        stage_index: u8,
        credit_ms: u64,
    ) -> Result<(), ConstructionMiracleRuntimeError> {
        if credit_ms == 0 {
            return Ok(());
        }
        let credit = self
            .pending_labor_credit_ms
            .entry(project_id.to_owned())
            .or_default()
            .entry(stage_index)
            .or_default();
        *credit = credit
            .checked_add(credit_ms)
            .ok_or(ConstructionMiracleRuntimeError::ArithmeticOverflow)?;
        Ok(())
    }
}

impl Default for ConstructionMiracleRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstructionMiracleRuntimeError {
    EmptyAuthorityFact,
    InvalidValueTable,
    MissingManifestInputClassification(String),
    NoEligibleManifestInput,
    MissingProject,
    TerminalProject,
    NoMissingBoundInput,
    BoundInputAmbiguous(String),
    ExactPackageUnavailable,
    CompositionSearchBoundExceeded,
    ReceiptConflict,
    ReceiptCapacity,
    LaborWitnessMismatch,
    PurposeBindingMismatch,
    MalformedState,
    ArithmeticOverflow,
    Hole(HoleError),
    DivineHole(DivineHoleError),
    Runtime(LeaderAiRuntimeError),
}

impl std::fmt::Display for ConstructionMiracleRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "canonical construction miracle runtime error: {self:?}"
        )
    }
}

impl std::error::Error for ConstructionMiracleRuntimeError {}

impl From<DivineHoleError> for ConstructionMiracleRuntimeError {
    fn from(value: DivineHoleError) -> Self {
        Self::DivineHole(value)
    }
}

impl From<HoleError> for ConstructionMiracleRuntimeError {
    fn from(value: HoleError) -> Self {
        Self::Hole(value)
    }
}

impl From<LeaderAiRuntimeError> for ConstructionMiracleRuntimeError {
    fn from(value: LeaderAiRuntimeError) -> Self {
        Self::Runtime(value)
    }
}

pub fn derive_construction_miracle_witness(
    runtime: &LeaderAiRuntimeState,
    project_id: &str,
) -> Result<ConstructionMiracleWitness, ConstructionMiracleRuntimeError> {
    if project_id.trim().is_empty() {
        return Err(ConstructionMiracleRuntimeError::EmptyAuthorityFact);
    }
    let project = runtime
        .construction_projects
        .get(project_id)
        .ok_or(ConstructionMiracleRuntimeError::MissingProject)?;
    let earliest = earliest_incomplete_stage_index(project)
        .ok_or(ConstructionMiracleRuntimeError::TerminalProject)?;
    let durations = labor_durations(project.original_total_work_ms);
    let ordered_remaining_labor_stages = (0_u8..=2)
        .map(|stage_index| {
            let base_remaining =
                effective_stage_work_before_pending(project, stage_index, durations);
            let pending = runtime
                .construction_miracles
                .pending_credit(project_id, stage_index);
            let remaining_work_ms = base_remaining
                .checked_sub(pending)
                .ok_or(ConstructionMiracleRuntimeError::MalformedState)?;
            Ok::<MiracleLaborStage, ConstructionMiracleRuntimeError>(MiracleLaborStage {
                stage_index,
                remaining_work_ms,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let bill = stage_bill(project, earliest);
    let bound_units = bound_units_by_content(runtime, project_id, earliest)?;
    let mut exact_missing_bound_inputs = Vec::new();
    for line in &bill.lines {
        let accounted = u64::from(line.accounted_units());
        let bound = bound_units.get(&line.content_id).copied().unwrap_or(0);
        if accounted > 0 && bound != accounted {
            return Err(ConstructionMiracleRuntimeError::BoundInputAmbiguous(
                line.content_id.clone(),
            ));
        }
        let already_bound = accounted.max(bound);
        let missing_quantity = u64::from(line.required_units)
            .checked_sub(already_bound)
            .ok_or_else(|| {
                ConstructionMiracleRuntimeError::BoundInputAmbiguous(line.content_id.clone())
            })?;
        if missing_quantity > 0 {
            exact_missing_bound_inputs.push(MissingConstructionInput {
                stage_index: earliest,
                definition_id: line.content_id.clone(),
                missing_quantity,
            });
        }
    }
    Ok(ConstructionMiracleWitness {
        project_id: project_id.to_owned(),
        original_total_work_ms: project.original_total_work_ms,
        earliest_incomplete_stage_index: earliest,
        ordered_remaining_labor_stages,
        exact_missing_bound_inputs,
    })
}

pub fn apply_construction_miracle(
    runtime: &mut LeaderAiRuntimeState,
    request: ApplyConstructionMiracle,
) -> Result<ConstructionMiracleRuntimeOutcome, ConstructionMiracleRuntimeError> {
    if request.command_id.trim().is_empty()
        || request.project_id.trim().is_empty()
        || request.player_id.trim().is_empty()
    {
        return Err(ConstructionMiracleRuntimeError::EmptyAuthorityFact);
    }
    let replay_identity = ConstructionMiracleReplayIdentity {
        project_id: request.project_id.clone(),
        player_id: request.player_id.clone(),
        now_real_ms: request.now_real_ms,
    };
    if let Some(receipt) = runtime
        .construction_miracles
        .receipts
        .get(&request.command_id)
    {
        return if receipt.replay_identity == replay_identity {
            Ok(receipt.outcome.clone())
        } else {
            Err(ConstructionMiracleRuntimeError::ReceiptConflict)
        };
    }
    if runtime.construction_miracles.receipts.len() >= MAX_CONSTRUCTION_MIRACLE_RECEIPTS {
        return Err(ConstructionMiracleRuntimeError::ReceiptCapacity);
    }

    let mut staged = runtime.clone();
    let witness = derive_construction_miracle_witness(&staged, &request.project_id)?;
    if witness.exact_missing_bound_inputs.is_empty() {
        return Err(ConstructionMiracleRuntimeError::NoMissingBoundInput);
    }
    let value_table =
        CanonicalConstructionInputValueTable::from_manifest(&witness.exact_missing_bound_inputs)?;
    let required_value = value_table
        .hole_feed_value_per_void_micros
        .checked_mul(MIRACLE_INPUT_VALUE_MULTIPLIER)
        .ok_or(ConstructionMiracleRuntimeError::ArithmeticOverflow)?;
    let inputs = compose_exact_package(
        &witness.exact_missing_bound_inputs,
        &value_table,
        required_value,
    )?;
    let envelope = VoidActionEnvelope::new(
        request.command_id.clone(),
        request.expected_authority_version,
        request.expected_void_version,
        VoidAction::ConstructionMiracle(ConstructionMiracleRequest {
            project_id: request.project_id.clone(),
            player_id: request.player_id.clone(),
            hole_feed_value_per_void_micros: value_table.hole_feed_value_per_void_micros,
            original_total_work_ms: witness.original_total_work_ms,
            labor_stages: witness.ordered_remaining_labor_stages.clone(),
            inputs,
            now_real_ms: request.now_real_ms,
        }),
    )?;
    let void_outcome = staged
        .divine_hole
        .apply_void_action(&mut staged.research.void, envelope)?;
    let labor_credit_added_by_stage_ms =
        labor_credit_delta(&witness.ordered_remaining_labor_stages, &void_outcome)?;
    for (stage_index, credit_ms) in &labor_credit_added_by_stage_ms {
        staged.construction_miracles.add_pending_credit(
            &request.project_id,
            *stage_index,
            *credit_ms,
        )?;
    }

    let mut construction_storage_identities = Vec::new();
    for cargo in &void_outcome.generated_cargo {
        match &cargo.purpose {
            BoundCargoPurpose::Construction {
                project_id,
                stage_index,
            } if project_id == &request.project_id
                && *stage_index == witness.earliest_incomplete_stage_index => {}
            _ => return Err(ConstructionMiracleRuntimeError::PurposeBindingMismatch),
        }
        let content_id = ContentId::new(&cargo.definition_id).map_err(|_| {
            ConstructionMiracleRuntimeError::MissingManifestInputClassification(
                cargo.definition_id.clone(),
            )
        })?;
        if ContentManifest::embedded()
            .construction_miracle_input(&content_id)
            .is_none_or(|descriptor| {
                descriptor.physical_class == ConstructionMiracleInputClass::Ineligible
            })
        {
            return Err(
                ConstructionMiracleRuntimeError::MissingManifestInputClassification(
                    cargo.definition_id.clone(),
                ),
            );
        }
        let identities = staged.materialize_typed_construction_miracle_cargo(
            cargo,
            StorageAddress::ConstructionCargo {
                project_id: request.project_id.clone(),
            },
        )?;
        let bound = staged
            .construction_storage_identities
            .get_mut(&request.project_id)
            .ok_or(ConstructionMiracleRuntimeError::MissingProject)?;
        for identity in identities {
            bound.insert(identity.clone());
            construction_storage_identities.push(identity);
        }
    }
    construction_storage_identities.sort();
    construction_storage_identities.dedup();

    let outcome = ConstructionMiracleRuntimeOutcome {
        void_outcome,
        construction_storage_identities,
        labor_credit_added_by_stage_ms,
    };
    staged.construction_miracles.receipts.insert(
        request.command_id.clone(),
        ConstructionMiracleRuntimeReceipt {
            command_id: request.command_id,
            replay_identity,
            outcome: outcome.clone(),
        },
    );
    staged.validate()?;
    *runtime = staged;
    Ok(outcome)
}

fn expected_materialized_identity_count(
    cargo: &[crate::food_divine_policy::PurposeBoundCargo],
) -> Option<usize> {
    cargo.iter().try_fold(0_usize, |count, cargo| {
        let content_id = ContentId::new(&cargo.definition_id).ok()?;
        let descriptor = ContentManifest::embedded().construction_miracle_input(&content_id)?;
        let added = match descriptor.physical_class {
            ConstructionMiracleInputClass::BulkLot => 1,
            ConstructionMiracleInputClass::ExactItem | ConstructionMiracleInputClass::Fixture => {
                usize::try_from(cargo.quantity).ok()?
            }
            ConstructionMiracleInputClass::Ineligible => return None,
        };
        count.checked_add(added)
    })
}

fn compose_exact_package(
    missing: &[MissingConstructionInput],
    values: &CanonicalConstructionInputValueTable,
    required_value: u64,
) -> Result<Vec<MiracleInput>, ConstructionMiracleRuntimeError> {
    let candidates = missing
        .iter()
        .filter_map(|input| {
            values
                .unit_value(&input.definition_id)
                .map(|unit_value| (input, unit_value))
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Err(ConstructionMiracleRuntimeError::NoEligibleManifestInput);
    }
    let mut states = BTreeMap::<u64, Vec<u64>>::from([(0, vec![0; candidates.len()])]);
    for (index, (input, unit_value)) in candidates.iter().enumerate() {
        let prior = states.clone();
        for (current_value, quantities) in prior {
            let remaining_value = required_value.saturating_sub(current_value);
            let max_quantity = input.missing_quantity.min(remaining_value / *unit_value);
            for quantity in 1..=max_quantity {
                let next_value = current_value
                    .checked_add(
                        quantity
                            .checked_mul(*unit_value)
                            .ok_or(ConstructionMiracleRuntimeError::ArithmeticOverflow)?,
                    )
                    .ok_or(ConstructionMiracleRuntimeError::ArithmeticOverflow)?;
                let mut next_quantities = quantities.clone();
                next_quantities[index] = quantity;
                states.entry(next_value).or_insert(next_quantities);
                if states.len() > MAX_COMPOSITION_STATES {
                    return Err(ConstructionMiracleRuntimeError::CompositionSearchBoundExceeded);
                }
            }
        }
    }
    let quantities = states
        .remove(&required_value)
        .ok_or(ConstructionMiracleRuntimeError::ExactPackageUnavailable)?;
    Ok(candidates
        .into_iter()
        .zip(quantities)
        .filter_map(|((input, unit_value_micros), quantity)| {
            (quantity > 0).then(|| MiracleInput {
                stage_index: input.stage_index,
                definition_id: input.definition_id.clone(),
                quantity,
                unit_value_micros,
                missing_quantity_before: input.missing_quantity,
            })
        })
        .collect())
}

fn labor_credit_delta(
    before: &[MiracleLaborStage],
    outcome: &VoidActionOutcome,
) -> Result<BTreeMap<u8, u64>, ConstructionMiracleRuntimeError> {
    if before.len() != outcome.labor_stages_after.len()
        || before
            .iter()
            .zip(&outcome.labor_stages_after)
            .any(|(left, right)| {
                left.stage_index != right.stage_index
                    || right.remaining_work_ms > left.remaining_work_ms
            })
    {
        return Err(ConstructionMiracleRuntimeError::LaborWitnessMismatch);
    }
    let credits = before
        .iter()
        .zip(&outcome.labor_stages_after)
        .filter_map(|(left, right)| {
            let credit = left.remaining_work_ms - right.remaining_work_ms;
            (credit > 0).then_some((left.stage_index, credit))
        })
        .collect::<BTreeMap<_, _>>();
    let credited_total = credits.values().try_fold(0_u64, |total, credit| {
        total
            .checked_add(*credit)
            .ok_or(ConstructionMiracleRuntimeError::ArithmeticOverflow)
    })?;
    if credited_total != outcome.labor_work_removed_ms {
        return Err(ConstructionMiracleRuntimeError::LaborWitnessMismatch);
    }
    Ok(credits)
}

fn bound_units_by_content(
    runtime: &LeaderAiRuntimeState,
    project_id: &str,
    stage_index: u8,
) -> Result<BTreeMap<String, u64>, ConstructionMiracleRuntimeError> {
    let identities = runtime
        .construction_storage_identities
        .get(project_id)
        .ok_or(ConstructionMiracleRuntimeError::MissingProject)?;
    let mut units = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for identity in identities {
        if !seen.insert(identity) || runtime.storage.location(identity).is_none() {
            return Err(ConstructionMiracleRuntimeError::MalformedState);
        }
        if runtime
            .purpose_bound_storage
            .get(identity)
            .is_some_and(|purpose| {
                !matches!(
                    purpose,
                    BoundCargoPurpose::Construction {
                        project_id: bound_project_id,
                        stage_index: bound_stage_index,
                    } if bound_project_id == project_id && *bound_stage_index == stage_index
                )
            })
        {
            continue;
        }
        let (content_id, quantity) = match identity {
            StorageIdentity::Lot(lot_id) => {
                let lot = runtime
                    .storage
                    .ledger()
                    .lot(lot_id)
                    .ok_or(ConstructionMiracleRuntimeError::MalformedState)?;
                (lot.key.content_id.as_str(), u64::from(lot.quantity))
            }
            StorageIdentity::Item(item_id) => {
                let item = runtime
                    .storage
                    .ledger()
                    .item(item_id)
                    .ok_or(ConstructionMiracleRuntimeError::MalformedState)?;
                let manifest = ContentManifest::embedded();
                let content_id = manifest
                    .item_definitions
                    .iter()
                    .find(|definition| definition.id == item.definition_id)
                    .map(|definition| definition.content_id.as_str())
                    .or_else(|| {
                        manifest
                            .fixtures
                            .iter()
                            .find(|fixture| fixture.id == item.definition_id)
                            .map(|fixture| fixture.content_id.as_str())
                    })
                    .ok_or(ConstructionMiracleRuntimeError::MalformedState)?;
                (content_id, 1)
            }
        };
        let total = units.entry(content_id.to_owned()).or_insert(0_u64);
        *total = total
            .checked_add(quantity)
            .ok_or(ConstructionMiracleRuntimeError::ArithmeticOverflow)?;
    }
    Ok(units)
}

fn earliest_incomplete_stage_index(project: &ConstructionProject) -> Option<u8> {
    match project.stage {
        ConstructionStage::SiteReserved
        | ConstructionStage::DeliverScaffold
        | ConstructionStage::BuildScaffold => Some(0),
        ConstructionStage::DeliverStructure | ConstructionStage::BuildStructure => Some(1),
        ConstructionStage::DeliverFitOut | ConstructionStage::BuildFitOut => Some(2),
        ConstructionStage::Operational | ConstructionStage::Cancelled => None,
    }
}

fn stage_bill(project: &ConstructionProject, stage_index: u8) -> &ConstructionStageBill {
    match stage_index {
        0 => &project.bills.scaffold,
        1 => &project.bills.structure,
        2 => &project.bills.fit_out,
        _ => unreachable!("canonical construction has exactly three stages"),
    }
}

fn labor_durations(original_total_work_ms: u64) -> [u64; 3] {
    let (scaffold, structure, fit_out) = stage_work_durations(original_total_work_ms);
    [scaffold, structure, fit_out]
}

fn effective_stage_work_before_pending(
    project: &ConstructionProject,
    stage_index: u8,
    durations: [u64; 3],
) -> u64 {
    let Some(earliest) = earliest_incomplete_stage_index(project) else {
        return 0;
    };
    if stage_index < earliest {
        0
    } else if stage_index == earliest && project.stage.is_labor() {
        project.stage_work_remaining_ms
    } else {
        durations[usize::from(stage_index)]
    }
}
