//! Raid threat director ported from `lib/game/threat.ts`.

use serde::{Deserialize, Serialize};

use crate::entities::Resources;

/// Grace window: no pressure builds for this many game-seconds into a run.
pub const RAID_GRACE_SEC: f64 = 8.0 * 3600.0;
/// Pressure at which a raid launches.
pub const RAID_SPAWN_THRESHOLD: f64 = 100.0;
/// Largest warband the director will field at once.
pub const MAX_RAID_SIZE: f64 = 12.0;
/// Base strength of a single raider before scaling.
pub const RAIDER_BASE_STRENGTH: f64 = 30.0;
/// Wealth floor before stored value adds extra raiders.
pub const RAID_WEALTH_FLOOR: f64 = 250.0;
/// Population above this baseline adds extra raiders.
pub const RAID_POP_FLOOR: f64 = 20.0;
/// Hard ceiling on cats a single lost raid can kill.
pub const MAX_RAID_CASUALTIES: u32 = 1;

const CASUALTY_RATIO: f64 = 0.6;
const MAX_LOOT_FRACTION: f64 = 0.3;

/// Total stored value the colony presents as loot.
pub trait ThreatStores {
    fn food(&self) -> f64;
    fn water(&self) -> f64;
    fn herbs(&self) -> f64;
    fn materials(&self) -> f64;
    fn refined(&self) -> f64;
    fn weapons(&self) -> f64;
    fn armor(&self) -> f64;
}

impl<T> ThreatStores for &T
where
    T: ThreatStores + ?Sized,
{
    fn food(&self) -> f64 {
        (*self).food()
    }

    fn water(&self) -> f64 {
        (*self).water()
    }

    fn herbs(&self) -> f64 {
        (*self).herbs()
    }

    fn materials(&self) -> f64 {
        (*self).materials()
    }

    fn refined(&self) -> f64 {
        (*self).refined()
    }

    fn weapons(&self) -> f64 {
        (*self).weapons()
    }

    fn armor(&self) -> f64 {
        (*self).armor()
    }
}

impl ThreatStores for Resources {
    fn food(&self) -> f64 {
        self.food + self.fish
    }

    fn water(&self) -> f64 {
        self.water
    }

    fn herbs(&self) -> f64 {
        self.herbs
    }

    fn materials(&self) -> f64 {
        self.materials
    }

    fn refined(&self) -> f64 {
        self.refined
    }

    fn weapons(&self) -> f64 {
        self.weapons
    }

    fn armor(&self) -> f64 {
        self.armor
    }
}

/// Minimal resource shape accepted by [`colony_wealth`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ThreatResources {
    pub food: f64,
    pub water: f64,
    pub herbs: f64,
    pub materials: f64,
    pub refined: f64,
    pub weapons: f64,
    pub armor: f64,
}

impl ThreatStores for ThreatResources {
    fn food(&self) -> f64 {
        self.food
    }

    fn water(&self) -> f64 {
        self.water
    }

    fn herbs(&self) -> f64 {
        self.herbs
    }

    fn materials(&self) -> f64 {
        self.materials
    }

    fn refined(&self) -> f64 {
        self.refined
    }

    fn weapons(&self) -> f64 {
        self.weapons
    }

    fn armor(&self) -> f64 {
        self.armor
    }
}

/// Threat inputs for the raid director.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreatSnapshot {
    pub wealth: f64,
    pub population: f64,
    pub warriors: f64,
    pub colony_age_sec: f64,
}

/// Threat-indicator band for the HUD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThreatBand {
    Calm,
    Rising,
    Imminent,
}

impl ThreatBand {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Calm => "calm",
            Self::Rising => "rising",
            Self::Imminent => "imminent",
        }
    }
}

/// Number and strength of raider units in a planned warband.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaidPlan {
    pub count: f64,
    pub strength_each: f64,
}

/// Deterministic combat result for a raid.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaidOutcome {
    pub defenders_win: bool,
    pub loot_fraction: f64,
    pub defender_casualties: u32,
    pub margin: f64,
}

/// Loot value a colony presents to the threat curve.
#[must_use]
pub fn colony_wealth(resources: impl ThreatStores) -> f64 {
    resources.food()
        + resources.water()
        + resources.herbs()
        + resources.materials()
        + resources.refined() * 3.0
        + resources.weapons() * 5.0
        + resources.armor() * 5.0
}

/// Pressure gained per game-hour.
#[must_use]
pub fn threat_rate_per_hour(snapshot: ThreatSnapshot) -> f64 {
    if snapshot.colony_age_sec < RAID_GRACE_SEC {
        return 0.0;
    }

    let wealth_term = js_max(0.0, snapshot.wealth).sqrt() * 0.12;
    let pop_term = js_max(0.0, snapshot.population) * 0.25;
    let warrior_term = js_max(0.0, snapshot.warriors) * 0.5;
    let age_term = js_min(10.0, (snapshot.colony_age_sec / 3600.0) * 0.04);

    1.0 + wealth_term + pop_term + warrior_term + age_term
}

/// Add the pressure earned over `elapsed_game_sec` to the running total.
#[must_use]
pub fn accrue_threat(pressure: f64, snapshot: ThreatSnapshot, elapsed_game_sec: f64) -> f64 {
    if elapsed_game_sec <= 0.0 {
        return js_max(0.0, pressure);
    }

    let gained = threat_rate_per_hour(snapshot) * (elapsed_game_sec / 3600.0);
    js_max(0.0, pressure + gained)
}

/// A raid launches once accrued pressure reaches the threshold.
#[must_use]
pub fn should_spawn_raid(pressure: f64) -> bool {
    pressure >= RAID_SPAWN_THRESHOLD
}

/// HUD threat band from the current pressure.
#[must_use]
pub fn threat_band(pressure: f64) -> ThreatBand {
    if pressure >= RAID_SPAWN_THRESHOLD * (2.0 / 3.0) {
        return ThreatBand::Imminent;
    }
    if pressure >= RAID_SPAWN_THRESHOLD / 3.0 {
        return ThreatBand::Rising;
    }
    ThreatBand::Calm
}

/// Size the warband for a snapshot.
#[must_use]
pub fn plan_raid(snapshot: ThreatSnapshot) -> RaidPlan {
    let from_warriors = (js_max(0.0, snapshot.warriors) * 0.6).floor();
    let from_wealth = (js_max(0.0, snapshot.wealth - RAID_WEALTH_FLOOR).sqrt() / 10.0).floor();
    let from_pop = (js_max(0.0, snapshot.population - RAID_POP_FLOOR) / 15.0).floor();
    let count = js_max(
        1.0,
        js_min(MAX_RAID_SIZE, 1.0 + from_warriors + from_wealth + from_pop),
    );
    let age_bonus = js_min(40.0, (snapshot.colony_age_sec / 3600.0) * 0.5);
    let strength_each = js_round(RAIDER_BASE_STRENGTH + age_bonus);

    RaidPlan {
        count,
        strength_each,
    }
}

/// Resolve a raid using injected randomness.
#[must_use]
pub fn resolve_raid(defense_power: f64, raider_power: f64, roll: f64) -> RaidOutcome {
    let swing = 0.75 + 0.5 * js_min(0.999_999, js_max(0.0, roll));
    let effective = defense_power * swing;
    let enemy = js_max(1.0, raider_power);
    let margin = effective / enemy;
    let defenders_win = effective >= enemy;

    if defenders_win {
        return RaidOutcome {
            defenders_win: true,
            loot_fraction: 0.0,
            defender_casualties: 0,
            margin,
        };
    }

    let shortfall = 1.0 - js_min(1.0, margin);
    let loot_fraction = js_min(MAX_LOOT_FRACTION, 0.1 + shortfall * 0.3);
    let defender_casualties = if margin < CASUALTY_RATIO {
        MAX_RAID_CASUALTIES
    } else {
        0
    };

    RaidOutcome {
        defenders_win: false,
        loot_fraction,
        defender_casualties,
        margin,
    }
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

fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

#[cfg(test)]
mod tests {
    use crate::entities::Resources;

    use super::{
        MAX_RAID_CASUALTIES, MAX_RAID_SIZE, RAID_GRACE_SEC, RAID_SPAWN_THRESHOLD, RaidOutcome,
        RaidPlan, ThreatBand, ThreatResources, ThreatSnapshot, accrue_threat, colony_wealth,
        plan_raid, resolve_raid, should_spawn_raid, threat_band, threat_rate_per_hour,
    };

    fn snap() -> ThreatSnapshot {
        ThreatSnapshot {
            wealth: 500.0,
            population: 20.0,
            warriors: 2.0,
            colony_age_sec: RAID_GRACE_SEC + 3600.0,
        }
    }

    fn assert_f64_close(actual: f64, expected: f64, label: &str) {
        assert!(
            (actual - expected).abs() <= 1e-12,
            "{label}: actual {actual:?}, expected {expected:?}",
        );
    }

    #[test]
    fn colony_wealth_weights_refined_and_gear() {
        let resources = ThreatResources {
            food: 10.0,
            water: 20.0,
            herbs: 30.0,
            materials: 40.0,
            refined: 5.0,
            weapons: 2.0,
            armor: 3.0,
        };

        assert_eq!(colony_wealth(resources), 140.0);
        assert_eq!(colony_wealth(ThreatResources::default()), 0.0);
        assert_eq!(
            colony_wealth(Resources {
                food: 10.0,
                fish: 0.0,
                water: 20.0,
                herbs: 30.0,
                catnip: 0.0,
                grain: 0.0,
                flour: 0.0,
                preserves: 0.0,
                medicine: 0.0,
                brew: 0.0,
                materials: 40.0,
                stone: 0.0,
                refined: 5.0,
                weapons: 2.0,
                armor: 3.0,
                planks: 0.0,
                logs: 0.0,
                lumber: 0.0,
                blocks: 0.0,
                tools: 0.0,
                fibre: 0.0,
                hide: 0.0,
                bone: 0.0,
                cloth: 0.0,
                leather: 0.0,
                ore: 0.0,
                gem: 0.0,
                clay: 0.0,
                sand: 0.0,
                metal: 0.0,
                blessings: 999.0,
            }),
            140.0
        );
    }

    #[test]
    fn threat_rate_obeys_grace_and_terms() {
        assert_eq!(
            threat_rate_per_hour(ThreatSnapshot {
                colony_age_sec: 0.0,
                ..snap()
            }),
            0.0
        );
        assert_eq!(
            threat_rate_per_hour(ThreatSnapshot {
                colony_age_sec: RAID_GRACE_SEC - 1.0,
                ..snap()
            }),
            0.0
        );

        assert_f64_close(
            threat_rate_per_hour(ThreatSnapshot {
                colony_age_sec: RAID_GRACE_SEC,
                ..snap()
            }),
            10.003_281_572_999_748,
            "grace boundary rate",
        );
        assert_f64_close(
            threat_rate_per_hour(snap()),
            10.043_281_572_999_747,
            "default rate",
        );
    }

    #[test]
    fn accrue_threat_adds_elapsed_game_time_and_clamps_nonpositive_ticks() {
        assert_f64_close(
            accrue_threat(0.0, snap(), 3600.0),
            10.043_281_572_999_747,
            "one hour gained pressure",
        );
        assert_eq!(
            accrue_threat(
                10.0,
                ThreatSnapshot {
                    colony_age_sec: 0.0,
                    ..snap()
                },
                3600.0
            ),
            10.0
        );
        assert_eq!(accrue_threat(-5.0, snap(), 0.0), 0.0);
    }

    #[test]
    fn threshold_and_bands_match_thirds() {
        assert!(!should_spawn_raid(RAID_SPAWN_THRESHOLD - 0.01));
        assert!(should_spawn_raid(RAID_SPAWN_THRESHOLD));

        assert_eq!(threat_band(0.0), ThreatBand::Calm);
        assert_eq!(threat_band(RAID_SPAWN_THRESHOLD / 3.0), ThreatBand::Rising);
        assert_eq!(
            threat_band(RAID_SPAWN_THRESHOLD * (2.0 / 3.0)),
            ThreatBand::Imminent
        );
        assert_eq!(ThreatBand::Imminent.as_str(), "imminent");
    }

    #[test]
    fn plan_raid_matches_hand_derived_vectors() {
        let starter = plan_raid(ThreatSnapshot {
            wealth: 240.0,
            population: 20.0,
            warriors: 0.0,
            colony_age_sec: RAID_GRACE_SEC,
        });
        assert_eq!(
            starter,
            RaidPlan {
                count: 1.0,
                strength_each: 34.0,
            }
        );

        let default = plan_raid(snap());
        assert_eq!(
            default,
            RaidPlan {
                count: 3.0,
                strength_each: 35.0,
            }
        );

        let wealthy = plan_raid(ThreatSnapshot {
            wealth: 6000.0,
            population: 45.0,
            warriors: 8.0,
            colony_age_sec: RAID_GRACE_SEC + 30.0 * 3600.0,
        });
        assert_eq!(
            wealthy,
            RaidPlan {
                count: MAX_RAID_SIZE,
                strength_each: 49.0,
            }
        );
    }

    #[test]
    fn resolve_raid_matches_hand_derived_win_loss_and_caps() {
        assert_eq!(
            resolve_raid(1000.0, 100.0, 0.5),
            RaidOutcome {
                defenders_win: true,
                loot_fraction: 0.0,
                defender_casualties: 0,
                margin: 10.0,
            }
        );

        let loss = resolve_raid(50.0, 1000.0, 0.5);
        assert!(!loss.defenders_win);
        assert_eq!(loss.defender_casualties, MAX_RAID_CASUALTIES);
        assert_f64_close(loss.margin, 0.05, "loss margin");
        assert_f64_close(loss.loot_fraction, 0.3, "loss loot cap");

        let close = resolve_raid(100.0, 100.0, 0.0);
        assert!(!close.defenders_win);
        assert_eq!(close.defender_casualties, 0);
        assert_f64_close(close.margin, 0.75, "close margin");
        assert_f64_close(close.loot_fraction, 0.175, "close loot");

        assert!(resolve_raid(100.0, 100.0, 0.999).defenders_win);
    }

    #[test]
    fn nan_inputs_follow_javascript_min_max_comparisons() {
        assert!(
            threat_rate_per_hour(ThreatSnapshot {
                wealth: f64::NAN,
                ..snap()
            })
            .is_nan()
        );
        assert_eq!(threat_band(f64::NAN), ThreatBand::Calm);
        assert!(
            plan_raid(ThreatSnapshot {
                warriors: f64::NAN,
                ..snap()
            })
            .count
            .is_nan()
        );

        let outcome = resolve_raid(10.0, 100.0, f64::NAN);
        assert!(!outcome.defenders_win);
        assert!(outcome.loot_fraction.is_nan());
        assert_eq!(outcome.defender_casualties, 0);
        assert!(outcome.margin.is_nan());
    }
}
