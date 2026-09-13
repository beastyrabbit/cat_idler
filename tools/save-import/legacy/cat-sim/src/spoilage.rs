//! Resource spoilage helpers ported from `lib/game/spoilage.ts`.

use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

/// Food decay per minute for food inside storage, from `server/game.ts`.
pub const STORED_FOOD_DECAY_PER_MINUTE: f64 = 0.0005;

/// Food decay per minute for food above capacity, from `server/game.ts`.
pub const OVERFLOW_FOOD_DECAY_PER_MINUTE: f64 = 0.02;

const HOT_THRESHOLD: f64 = 30.0;
const COLD_THRESHOLD: f64 = 5.0;
const HOT_MULTIPLIER: f64 = 2.0;
const COLD_MULTIPLIER: f64 = 0.5;
const CAPACITY_PENALTY_THRESHOLD: f64 = 0.8;
const CAPACITY_PENALTY_MULTIPLIER: f64 = 1.25;

/// Resources handled by the TypeScript spoilage report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpoilableResource {
    Food,
    Herbs,
}

/// Inputs for one resource in the storage-efficiency report.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageConditions {
    pub resource: SpoilableResource,
    pub current_amount: f64,
    pub max_capacity: f64,
    pub storage_level: i32,
    pub temperature: f64,
}

/// Per-resource result from the storage-efficiency report.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSpoilageResult {
    pub resource: SpoilableResource,
    pub original_amount: f64,
    pub spoiled_amount: f64,
    pub remaining_amount: f64,
    pub spoilage_rate: f64,
    pub efficiency: f64,
}

/// Full storage-efficiency report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageReport {
    pub results: Vec<ResourceSpoilageResult>,
    pub overall_efficiency: f64,
    pub worst_resource: Option<SpoilableResource>,
}

/// Breakdown of the server tick food-spoilage formula.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FoodSpoilage {
    pub raw_food: f64,
    pub stored: f64,
    pub overflow: f64,
    pub decayed_stored: f64,
    pub decayed_overflow: f64,
    pub remaining_food: f64,
}

/// Calculate the report spoilage rate for a resource, as a percentage per tick.
#[must_use]
pub fn calculate_spoilage_rate(
    resource: SpoilableResource,
    storage_level: i32,
    temperature: f64,
    capacity_ratio: f64,
) -> f64 {
    let base_rate = base_decay_rate(resource);
    let reduction_per_level = storage_reduction_per_level(resource);

    let storage_multiplier = js_max(0.0, 1.0 - reduction_per_level * f64::from(storage_level));
    let mut rate = base_rate * storage_multiplier;

    if temperature > HOT_THRESHOLD {
        rate *= HOT_MULTIPLIER;
    } else if temperature < COLD_THRESHOLD {
        rate *= COLD_MULTIPLIER;
    }

    if capacity_ratio > CAPACITY_PENALTY_THRESHOLD {
        rate *= CAPACITY_PENALTY_MULTIPLIER;
    }

    js_max(0.0, rate)
}

/// Apply a percentage spoilage rate to a current resource amount.
#[must_use]
pub fn apply_spoilage(current_amount: f64, spoilage_rate: f64) -> f64 {
    if current_amount <= 0.0 {
        return 0.0;
    }

    let remaining = current_amount * (1.0 - spoilage_rate / 100.0);
    js_max(0.0, remaining)
}

/// Estimate resource amount after repeated ticks at the same spoilage rate.
#[must_use]
pub fn estimate_spoilage_over_time(current_amount: f64, spoilage_rate: f64, ticks: i32) -> f64 {
    if ticks <= 0 || spoilage_rate <= 0.0 {
        return current_amount;
    }

    let retention_rate = 1.0 - spoilage_rate / 100.0;
    current_amount * retention_rate.powf(f64::from(ticks))
}

/// Evaluate storage efficiency across multiple resources.
#[must_use]
pub fn evaluate_storage_efficiency(resources: impl AsRef<[StorageConditions]>) -> StorageReport {
    let resources = resources.as_ref();

    if resources.is_empty() {
        return StorageReport {
            results: Vec::new(),
            overall_efficiency: 100.0,
            worst_resource: None,
        };
    }

    let results: Vec<ResourceSpoilageResult> = resources
        .iter()
        .map(|condition| {
            let capacity_ratio = if condition.max_capacity > 0.0 {
                condition.current_amount / condition.max_capacity
            } else {
                0.0
            };
            let spoilage_rate = calculate_spoilage_rate(
                condition.resource,
                condition.storage_level,
                condition.temperature,
                capacity_ratio,
            );
            let remaining_amount = apply_spoilage(condition.current_amount, spoilage_rate);
            let spoiled_amount = condition.current_amount - remaining_amount;
            let efficiency = (1.0 - spoilage_rate / 100.0) * 100.0;

            ResourceSpoilageResult {
                resource: condition.resource,
                original_amount: condition.current_amount,
                spoiled_amount: round_to_cents(spoiled_amount),
                remaining_amount: round_to_cents(remaining_amount),
                spoilage_rate,
                efficiency,
            }
        })
        .collect();

    let overall_efficiency =
        results.iter().map(|result| result.efficiency).sum::<f64>() / results.len() as f64;

    let mut worst_resource = None;
    let mut worst_efficiency = f64::INFINITY;
    for result in &results {
        if result.efficiency < worst_efficiency {
            worst_efficiency = result.efficiency;
            worst_resource = Some(result.resource);
        }
    }

    StorageReport {
        results,
        overall_efficiency,
        worst_resource,
    }
}

/// Generate the newspaper-style "Storage & Supplies" text from a report.
#[must_use]
pub fn generate_spoilage_report(
    report: impl Borrow<StorageReport>,
    colony_name: impl AsRef<str>,
) -> String {
    let report = report.borrow();
    let colony_name = colony_name.as_ref();
    let mut lines = Vec::new();
    lines.push(format!("STORAGE & SUPPLIES — {colony_name}"));
    lines.push(String::new());

    if report.results.is_empty() {
        lines.push("Nothing currently stored in the colony warehouses.".to_owned());
        return lines.join("\n");
    }

    if report.overall_efficiency > 90.0 {
        lines.push(
            "Storage conditions are excellent across the colony. Minimal waste reported."
                .to_owned(),
        );
    } else if report.overall_efficiency >= 50.0 {
        lines.push(
            "Storage conditions are adequate, though some improvements could reduce waste."
                .to_owned(),
        );
    } else {
        lines.push(
            "Alarming waste levels reported! Poor storage conditions are causing crisis-level spoilage."
                .to_owned(),
        );
    }
    lines.push(String::new());

    for result in &report.results {
        lines.push(format!(
            "{}: {} units remaining ({} lost to spoilage, {:.1}% rate)",
            result.resource.label(),
            result.remaining_amount,
            result.spoiled_amount,
            result.spoilage_rate
        ));
    }

    if let Some(worst_resource) = report.worst_resource
        && report.results.len() > 1
    {
        lines.push(String::new());
        lines.push(format!(
            "Worst performing: {} storage needs urgent attention.",
            worst_resource.as_str()
        ));
    }

    lines.join("\n")
}

/// Apply the authoritative server tick food-spoilage formula to a raw food
/// amount after consumption has already been subtracted.
#[must_use]
pub fn apply_food_spoilage(raw_food: f64, food_capacity: f64, elapsed_sec: f64) -> f64 {
    calculate_food_spoilage(raw_food, food_capacity, elapsed_sec).remaining_food
}

/// Apply the server tick food-spoilage formula after subtracting food use.
#[must_use]
pub fn apply_food_spoilage_after_consumption(
    current_food: f64,
    food_use: f64,
    food_capacity: f64,
    elapsed_sec: f64,
) -> f64 {
    apply_food_spoilage(
        js_max(0.0, current_food - food_use),
        food_capacity,
        elapsed_sec,
    )
}

/// Return every component of the authoritative server tick food-spoilage formula.
#[must_use]
pub fn calculate_food_spoilage(
    raw_food: f64,
    food_capacity: f64,
    elapsed_sec: f64,
) -> FoodSpoilage {
    let raw_food = js_max(0.0, raw_food);
    let stored = js_min(raw_food, food_capacity);
    let overflow = js_max(0.0, raw_food - food_capacity);
    let minutes = elapsed_sec / 60.0;
    let decayed_stored = stored * (1.0 - STORED_FOOD_DECAY_PER_MINUTE * minutes);
    let decayed_overflow = overflow * (1.0 - OVERFLOW_FOOD_DECAY_PER_MINUTE * minutes);
    let remaining_food = js_max(0.0, decayed_stored + decayed_overflow);

    FoodSpoilage {
        raw_food,
        stored,
        overflow,
        decayed_stored,
        decayed_overflow,
        remaining_food,
    }
}

/// Apply only the stored-food branch of the server tick formula.
#[must_use]
pub fn apply_stored_food_decay(stored_food: f64, elapsed_sec: f64) -> f64 {
    stored_food * (1.0 - STORED_FOOD_DECAY_PER_MINUTE * (elapsed_sec / 60.0))
}

/// Apply only the overflow-food branch of the server tick formula.
#[must_use]
pub fn apply_overflow_food_decay(overflow_food: f64, elapsed_sec: f64) -> f64 {
    overflow_food * (1.0 - OVERFLOW_FOOD_DECAY_PER_MINUTE * (elapsed_sec / 60.0))
}

impl SpoilableResource {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Food => "food",
            Self::Herbs => "herbs",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Food => "Food",
            Self::Herbs => "Herbs",
        }
    }
}

const fn base_decay_rate(resource: SpoilableResource) -> f64 {
    match resource {
        SpoilableResource::Food => 2.0,
        SpoilableResource::Herbs => 1.0,
    }
}

const fn storage_reduction_per_level(resource: SpoilableResource) -> f64 {
    match resource {
        SpoilableResource::Food => 0.5,
        SpoilableResource::Herbs => 0.4,
    }
}

fn round_to_cents(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn js_max(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left > right {
        left
    } else {
        right
    }
}

fn js_min(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left < right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ResourceSpoilageResult, SpoilableResource, StorageConditions, StorageReport,
        apply_food_spoilage, apply_food_spoilage_after_consumption, apply_overflow_food_decay,
        apply_spoilage, apply_stored_food_decay, calculate_food_spoilage, calculate_spoilage_rate,
        estimate_spoilage_over_time, evaluate_storage_efficiency, generate_spoilage_report,
    };

    #[test]
    fn report_spoilage_rates_match_spoilage_ts_vectors() {
        assert_eq!(
            calculate_spoilage_rate(SpoilableResource::Food, 0, 20.0, 0.5),
            2.0
        );
        assert_eq!(
            calculate_spoilage_rate(SpoilableResource::Herbs, 0, 20.0, 0.5),
            1.0
        );
        assert_eq!(
            calculate_spoilage_rate(SpoilableResource::Food, 1, 20.0, 0.5),
            1.0
        );
        assert_eq!(
            calculate_spoilage_rate(SpoilableResource::Food, 2, 20.0, 0.5),
            0.0
        );
        assert_eq!(
            calculate_spoilage_rate(SpoilableResource::Food, 3, 20.0, 0.5),
            0.0
        );
        assert!(
            (calculate_spoilage_rate(SpoilableResource::Herbs, 1, 20.0, 0.5) - 0.6).abs() < 1e-12
        );
        assert!(
            (calculate_spoilage_rate(SpoilableResource::Herbs, 2, 20.0, 0.5) - 0.2).abs() < 1e-12
        );
        assert_eq!(
            calculate_spoilage_rate(SpoilableResource::Herbs, 3, 20.0, 0.5),
            0.0
        );
        assert_eq!(
            calculate_spoilage_rate(SpoilableResource::Food, 0, 35.0, 0.5),
            4.0
        );
        assert_eq!(
            calculate_spoilage_rate(SpoilableResource::Food, 0, 3.0, 0.5),
            1.0
        );
        assert_eq!(
            calculate_spoilage_rate(SpoilableResource::Food, 0, 20.0, 0.9),
            2.5
        );
    }

    #[test]
    fn report_helpers_apply_and_compound_spoilage() {
        assert_eq!(apply_spoilage(100.0, 2.0), 98.0);
        assert!((apply_spoilage(1.0, 99.0) - 0.01).abs() < 1e-12);
        assert_eq!(apply_spoilage(0.5, 100.0), 0.0);
        assert_eq!(apply_spoilage(0.0, 5.0), 0.0);

        assert!((estimate_spoilage_over_time(100.0, 10.0, 3) - 72.9).abs() < 1e-12);
        assert_eq!(estimate_spoilage_over_time(100.0, 10.0, 0), 100.0);
        assert_eq!(estimate_spoilage_over_time(100.0, 0.0, 5), 100.0);
    }

    #[test]
    fn storage_report_matches_spoilage_ts_vectors() {
        let report = evaluate_storage_efficiency([
            StorageConditions {
                resource: SpoilableResource::Food,
                current_amount: 80.0,
                max_capacity: 100.0,
                storage_level: 1,
                temperature: 20.0,
            },
            StorageConditions {
                resource: SpoilableResource::Herbs,
                current_amount: 40.0,
                max_capacity: 100.0,
                storage_level: 0,
                temperature: 20.0,
            },
        ]);

        assert_eq!(report.results.len(), 2);
        assert_eq!(report.results[0].resource, SpoilableResource::Food);
        assert_eq!(report.results[1].resource, SpoilableResource::Herbs);

        let empty = evaluate_storage_efficiency([]);
        assert!(empty.results.is_empty());
        assert_eq!(empty.overall_efficiency, 100.0);
        assert_eq!(empty.worst_resource, None);

        let worst = evaluate_storage_efficiency([
            StorageConditions {
                resource: SpoilableResource::Food,
                current_amount: 80.0,
                max_capacity: 100.0,
                storage_level: 2,
                temperature: 20.0,
            },
            StorageConditions {
                resource: SpoilableResource::Herbs,
                current_amount: 50.0,
                max_capacity: 100.0,
                storage_level: 0,
                temperature: 35.0,
            },
        ]);
        assert_eq!(worst.worst_resource, Some(SpoilableResource::Herbs));

        let single = evaluate_storage_efficiency([StorageConditions {
            resource: SpoilableResource::Food,
            current_amount: 50.0,
            max_capacity: 100.0,
            storage_level: 0,
            temperature: 20.0,
        }]);
        assert_eq!(single.overall_efficiency, 98.0);
    }

    #[test]
    fn report_text_matches_spoilage_ts_shape() {
        let text = generate_spoilage_report(
            &StorageReport {
                results: vec![ResourceSpoilageResult {
                    resource: SpoilableResource::Food,
                    original_amount: 100.0,
                    spoiled_amount: 2.0,
                    remaining_amount: 98.0,
                    spoilage_rate: 2.0,
                    efficiency: 98.0,
                }],
                overall_efficiency: 98.0,
                worst_resource: Some(SpoilableResource::Food),
            },
            "Whiskertown",
        );
        assert_eq!(
            text,
            "STORAGE & SUPPLIES — Whiskertown\n\nStorage conditions are excellent across the colony. Minimal waste reported.\n\nFood: 98 units remaining (2 lost to spoilage, 2.0% rate)"
        );

        let empty = generate_spoilage_report(
            &StorageReport {
                results: Vec::new(),
                overall_efficiency: 100.0,
                worst_resource: None,
            },
            "Pawville",
        );
        assert_eq!(
            empty,
            "STORAGE & SUPPLIES — Pawville\n\nNothing currently stored in the colony warehouses."
        );
    }

    #[test]
    fn server_food_spoilage_formula_uses_stored_and_overflow_decay_rates() {
        assert_eq!(apply_stored_food_decay(100.0, 60.0), 99.95);
        assert_eq!(apply_overflow_food_decay(25.0, 60.0), 24.5);

        let report = calculate_food_spoilage(150.0, 100.0, 60.0);
        assert_eq!(report.raw_food, 150.0);
        assert_eq!(report.stored, 100.0);
        assert_eq!(report.overflow, 50.0);
        assert_eq!(report.decayed_stored, 99.95);
        assert_eq!(report.decayed_overflow, 49.0);
        assert_eq!(report.remaining_food, 148.95);
        assert_eq!(apply_food_spoilage(150.0, 100.0, 60.0), 148.95);
        assert_eq!(
            apply_food_spoilage_after_consumption(155.0, 5.0, 100.0, 60.0),
            148.95
        );
        assert_eq!(apply_food_spoilage(-10.0, 100.0, 60.0), 0.0);
    }
}
