//! Cat needs helpers ported from `lib/game/needs.ts`.

use std::borrow::Borrow;

use crate::{
    entities::CatNeeds,
    needs_constants::{NEEDS_DECAY_RATES, NEEDS_RESTORE_AMOUNTS},
};

#[must_use]
pub fn decay_needs(current_needs: impl Borrow<CatNeeds>, tick_count: f64) -> CatNeeds {
    let current_needs = current_needs.borrow();

    CatNeeds {
        hunger: js_max(
            0.0,
            current_needs.hunger - NEEDS_DECAY_RATES.hunger * tick_count,
        ),
        thirst: js_max(
            0.0,
            current_needs.thirst - NEEDS_DECAY_RATES.thirst * tick_count,
        ),
        rest: js_max(
            0.0,
            current_needs.rest - NEEDS_DECAY_RATES.rest * tick_count,
        ),
        health: current_needs.health,
    }
}

#[must_use]
pub fn restore_hunger_default(needs: impl Borrow<CatNeeds>) -> CatNeeds {
    restore_hunger(needs, NEEDS_RESTORE_AMOUNTS.eating)
}

#[must_use]
pub fn restore_hunger(needs: impl Borrow<CatNeeds>, amount: f64) -> CatNeeds {
    let needs = needs.borrow();

    CatNeeds {
        hunger: js_min(100.0, needs.hunger + amount),
        ..needs.clone()
    }
}

#[must_use]
pub fn restore_thirst_default(needs: impl Borrow<CatNeeds>) -> CatNeeds {
    restore_thirst(needs, NEEDS_RESTORE_AMOUNTS.drinking)
}

#[must_use]
pub fn restore_thirst(needs: impl Borrow<CatNeeds>, amount: f64) -> CatNeeds {
    let needs = needs.borrow();

    CatNeeds {
        thirst: js_min(100.0, needs.thirst + amount),
        ..needs.clone()
    }
}

#[must_use]
pub fn restore_rest(needs: impl Borrow<CatNeeds>, amount: f64, has_beds: bool) -> CatNeeds {
    let needs = needs.borrow();
    let restore_amount = if has_beds {
        NEEDS_RESTORE_AMOUNTS.sleeping_with_beds
    } else {
        amount
    };

    CatNeeds {
        rest: js_min(100.0, needs.rest + restore_amount),
        ..needs.clone()
    }
}

#[must_use]
pub fn restore_health(needs: impl Borrow<CatNeeds>, amount: f64) -> CatNeeds {
    let needs = needs.borrow();

    CatNeeds {
        health: js_min(100.0, needs.health + amount),
        ..needs.clone()
    }
}

#[must_use]
pub fn apply_needs_damage(needs: impl Borrow<CatNeeds>) -> CatNeeds {
    let needs = needs.borrow();
    let mut damage = 0.0;

    if needs.hunger == 0.0 {
        damage += 5.0;
    }
    if needs.thirst == 0.0 {
        damage += 3.0;
    }

    CatNeeds {
        health: js_max(0.0, needs.health - damage),
        ..needs.clone()
    }
}

#[must_use]
pub fn apply_needs_damage_over_time(needs: impl Borrow<CatNeeds>, tick_count: f64) -> CatNeeds {
    let needs = needs.borrow();

    if tick_count <= 0.0 {
        return needs.clone();
    }

    let mut damage = 0.0;
    if needs.hunger == 0.0 {
        damage += 5.0 * tick_count;
    }
    if needs.thirst == 0.0 {
        damage += 3.0 * tick_count;
    }

    CatNeeds {
        health: js_max(0.0, needs.health - damage),
        ..needs.clone()
    }
}

#[must_use]
pub fn has_needs_critical_default(needs: impl Borrow<CatNeeds>) -> bool {
    has_needs_critical(needs, 15.0)
}

#[must_use]
pub fn has_needs_critical(needs: impl Borrow<CatNeeds>, threshold: f64) -> bool {
    let needs = needs.borrow();

    needs.hunger < threshold
        || needs.thirst < threshold
        || needs.rest < threshold
        || needs.health < threshold
}

#[must_use]
pub fn is_dead(needs: impl Borrow<CatNeeds>) -> bool {
    needs.borrow().health == 0.0
}

fn js_max(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left >= right {
        left
    } else {
        right
    }
}

fn js_min(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left <= right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::{
        apply_needs_damage, apply_needs_damage_over_time, decay_needs, has_needs_critical,
        has_needs_critical_default, is_dead, restore_health, restore_hunger,
        restore_hunger_default, restore_rest, restore_thirst, restore_thirst_default,
    };
    use crate::{
        entities::CatNeeds,
        needs_constants::{NEEDS_DECAY_RATES, NEEDS_RESTORE_AMOUNTS},
    };

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        source: String,
        constants: ConstantsFixture,
        counts: CountsFixture,
        decay: Vec<DecayCase>,
        restore: Vec<RestoreCase>,
        damage: Vec<NeedsCase>,
        damage_over_time: Vec<DamageOverTimeCase>,
        critical: Vec<CriticalCase>,
        dead: Vec<BoolCase>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ConstantsFixture {
        decay_rates: DecayRatesFixture,
        restore_amounts: RestoreAmountsFixture,
    }

    #[derive(Debug, Deserialize)]
    struct DecayRatesFixture {
        hunger: f64,
        thirst: f64,
        rest: f64,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RestoreAmountsFixture {
        eating: f64,
        drinking: f64,
        sleeping: f64,
        sleeping_with_beds: f64,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CountsFixture {
        decay: usize,
        restore: usize,
        damage: usize,
        damage_over_time: usize,
        critical: usize,
        dead: usize,
        total: usize,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DecayCase {
        name: String,
        needs: CatNeeds,
        tick_count: f64,
        expected_bits: NeedsBitsFixture,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct RestoreCase {
        name: String,
        kind: RestoreKind,
        needs: CatNeeds,
        amount: Option<f64>,
        has_beds: Option<bool>,
        expected_bits: NeedsBitsFixture,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "lowercase")]
    enum RestoreKind {
        Hunger,
        Thirst,
        Rest,
        Health,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct NeedsCase {
        name: String,
        needs: CatNeeds,
        expected_bits: NeedsBitsFixture,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DamageOverTimeCase {
        name: String,
        needs: CatNeeds,
        tick_count: f64,
        expected_bits: NeedsBitsFixture,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct CriticalCase {
        name: String,
        needs: CatNeeds,
        threshold: f64,
        expected: bool,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct BoolCase {
        name: String,
        needs: CatNeeds,
        expected: bool,
    }

    #[derive(Debug, Deserialize)]
    struct NeedsBitsFixture {
        hunger: String,
        thirst: String,
        rest: String,
        health: String,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p4/needs.json"
        ))
        .expect("needs fixture parses")
    }

    fn assert_f64_exact(actual: f64, expected: f64, label: &str) {
        assert_eq!(actual.to_bits(), expected.to_bits(), "{label}");
    }

    fn assert_f64_bits(actual: f64, expected_bits: &str, label: &str) {
        assert_eq!(actual.to_bits().to_string(), expected_bits, "{label}");
    }

    fn assert_needs_bits(actual: &CatNeeds, expected: &NeedsBitsFixture, name: &str) {
        assert_f64_bits(actual.hunger, &expected.hunger, &format!("{name} hunger"));
        assert_f64_bits(actual.thirst, &expected.thirst, &format!("{name} thirst"));
        assert_f64_bits(actual.rest, &expected.rest, &format!("{name} rest"));
        assert_f64_bits(actual.health, &expected.health, &format!("{name} health"));
    }

    #[test]
    fn fixture_is_generated_from_needs_ts() {
        let fixture = fixture();

        assert_eq!(fixture.source, "lib/game/needs.ts");
        assert_eq!(fixture.counts.decay, fixture.decay.len());
        assert_eq!(fixture.counts.restore, fixture.restore.len());
        assert_eq!(fixture.counts.damage, fixture.damage.len());
        assert_eq!(
            fixture.counts.damage_over_time,
            fixture.damage_over_time.len()
        );
        assert_eq!(fixture.counts.critical, fixture.critical.len());
        assert_eq!(fixture.counts.dead, fixture.dead.len());
        assert_eq!(
            fixture.counts.total,
            fixture.decay.len()
                + fixture.restore.len()
                + fixture.damage.len()
                + fixture.damage_over_time.len()
                + fixture.critical.len()
                + fixture.dead.len()
        );
    }

    #[test]
    fn constants_match_ts_fixture() {
        let constants = fixture().constants;

        assert_f64_exact(
            NEEDS_DECAY_RATES.hunger,
            constants.decay_rates.hunger,
            "hunger decay",
        );
        assert_f64_exact(
            NEEDS_DECAY_RATES.thirst,
            constants.decay_rates.thirst,
            "thirst decay",
        );
        assert_f64_exact(
            NEEDS_DECAY_RATES.rest,
            constants.decay_rates.rest,
            "rest decay",
        );
        assert_f64_exact(
            NEEDS_RESTORE_AMOUNTS.eating,
            constants.restore_amounts.eating,
            "eating restore",
        );
        assert_f64_exact(
            NEEDS_RESTORE_AMOUNTS.drinking,
            constants.restore_amounts.drinking,
            "drinking restore",
        );
        assert_f64_exact(
            NEEDS_RESTORE_AMOUNTS.sleeping,
            constants.restore_amounts.sleeping,
            "sleeping restore",
        );
        assert_f64_exact(
            NEEDS_RESTORE_AMOUNTS.sleeping_with_beds,
            constants.restore_amounts.sleeping_with_beds,
            "bed sleeping restore",
        );
    }

    #[test]
    fn decay_needs_matches_ts_fixture() {
        for case in fixture().decay {
            assert_needs_bits(
                &decay_needs(&case.needs, case.tick_count),
                &case.expected_bits,
                &case.name,
            );
        }
    }

    #[test]
    fn restore_helpers_match_ts_fixture() {
        for case in fixture().restore {
            let actual = match case.kind {
                RestoreKind::Hunger => match case.amount {
                    Some(amount) => restore_hunger(&case.needs, amount),
                    None => restore_hunger_default(&case.needs),
                },
                RestoreKind::Thirst => match case.amount {
                    Some(amount) => restore_thirst(&case.needs, amount),
                    None => restore_thirst_default(&case.needs),
                },
                RestoreKind::Rest => restore_rest(
                    &case.needs,
                    case.amount.expect("rest fixture has amount"),
                    case.has_beds.expect("rest fixture has bed flag"),
                ),
                RestoreKind::Health => {
                    restore_health(&case.needs, case.amount.expect("health fixture has amount"))
                }
            };

            assert_needs_bits(&actual, &case.expected_bits, &case.name);
        }
    }

    #[test]
    fn apply_needs_damage_matches_ts_fixture() {
        for case in fixture().damage {
            assert_needs_bits(
                &apply_needs_damage(&case.needs),
                &case.expected_bits,
                &case.name,
            );
        }
    }

    #[test]
    fn apply_needs_damage_over_time_matches_ts_fixture() {
        for case in fixture().damage_over_time {
            assert_needs_bits(
                &apply_needs_damage_over_time(&case.needs, case.tick_count),
                &case.expected_bits,
                &case.name,
            );
        }
    }

    #[test]
    fn critical_helpers_match_ts_fixture() {
        for case in fixture().critical {
            assert_eq!(
                has_needs_critical(&case.needs, case.threshold),
                case.expected,
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn default_critical_threshold_is_fifteen() {
        for case in fixture()
            .critical
            .into_iter()
            .filter(|case| case.threshold == 15.0)
        {
            assert_eq!(
                has_needs_critical_default(&case.needs),
                case.expected,
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn is_dead_matches_ts_fixture() {
        for case in fixture().dead {
            assert_eq!(is_dead(&case.needs), case.expected, "{}", case.name);
        }
    }

    #[test]
    fn nan_inputs_follow_js_math_min_and_max() {
        let needs = CatNeeds {
            hunger: f64::NAN,
            thirst: 100.0,
            rest: 100.0,
            health: f64::NAN,
        };

        assert!(decay_needs(&needs, 1.0).hunger.is_nan());
        assert!(restore_health(&needs, 1.0).health.is_nan());
        assert!(!has_needs_critical(&needs, 15.0));
    }
}
