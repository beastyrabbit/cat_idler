//! Leader snapshot and decision contract ported from `lib/game/leaderAI.ts`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::officers::OfficerRole;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LeaderResources {
    pub food: f64,
    pub refined: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LeaderHousing {
    pub capacity: u32,
    pub committed: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThreatBand {
    #[serde(rename = "calm")]
    Calm,
    #[serde(rename = "rising")]
    Rising,
    #[serde(rename = "imminent")]
    Imminent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaderSnapshot {
    /// Living cats in the colony (raw head count, including kittens).
    pub population: u32,
    /// Stage-weighted count of work-capable cats; falls back to `population`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workforce: Option<f64>,
    /// Cats free to take on a new job right now.
    pub idle_cats: u32,
    /// Cats currently occupied by any job or workplace.
    pub employed_cats: u32,
    pub resources: LeaderResources,
    pub food_capacity: f64,
    /// Net food drained per game-hour; omitted means no projected scarcity.
    #[serde(
        default,
        alias = "foodDrainPerTick",
        skip_serializing_if = "Option::is_none"
    )]
    pub food_drain_per_hour: Option<f64>,
    /// Stable generic Supplies in store and the cap they are clamped to.
    pub materials: f64,
    pub materials_capacity: f64,
    /// Raw Stone in store and the cap it is clamped to. Additive for pre-P19 snapshots;
    /// absence is an empty raw-stone store, never an alias of legacy `materials`.
    #[serde(default)]
    pub stone: f64,
    #[serde(default)]
    pub stone_capacity: f64,
    /// Water in store and the cap it is clamped to.
    pub water: f64,
    pub water_capacity: f64,
    /// Net water drained per game-hour; omitted means no projected scarcity.
    #[serde(
        default,
        alias = "waterDrainPerTick",
        skip_serializing_if = "Option::is_none"
    )]
    pub water_drain_per_hour: Option<f64>,
    pub housing: LeaderHousing,
    /// `hunt_expedition` jobs in flight (active or queued).
    pub active_hunts: u32,
    /// `quarry` jobs in flight (active or queued).
    pub active_quarries: u32,
    /// `explore` jobs in flight (active or queued).
    pub active_scouts: u32,
    /// `fetch_water` jobs in flight (active or queued).
    pub active_water_fetchers: u32,
    /// An explored mountains/cave tile exists to quarry.
    pub has_quarry_site: bool,
    /// An explored water tile the colony can draw from.
    pub has_water_site: bool,
    /// An unexplored tile still sits on the reachable frontier.
    pub has_frontier: bool,
    /// Den plans in flight: `leader_plan_house` or a `build_house` den.
    pub den_plans_in_flight: u32,
    /// Storehouse builds in flight: `build_house` with a food-storage target.
    pub storage_plans_in_flight: u32,
    /// Finished granary storehouses currently standing.
    pub storehouse_count: u32,
    /// Cap on total storehouses (scales with population).
    pub storehouse_cap: u32,
    /// Completed workshops that have no assigned worker — the TS-ported general
    /// refinement workshop (`materials` -> `refined`) PLUS the Rust-only P16 raw-material
    /// craft benches (wood-cutter/stone-prep/woodworking) that have no TS equivalent.
    /// A pre-P16 caller that only ever had the general workshop still gets the same
    /// count it always did.
    pub workshops_needing_workers: u32,
    /// Completed research huts that have no assigned researcher.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub research_huts_needing_workers: Option<u32>,
    /// Completed smithies that have no assigned smith.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smithies_needing_workers: Option<u32>,
    /// A finished barracks stands, so cats can be trained into warriors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_barracks: Option<bool>,
    /// Trained warriors currently standing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warrior_count: Option<u32>,
    /// `train_warrior` jobs already in flight.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub training_in_flight: Option<u32>,
    /// `carry_offering` jobs already in flight (P12.6). Mirrors
    /// `storage_plans_in_flight`/`den_plans_in_flight`: stops the director from
    /// stacking a fresh offering dispatch on top of one already carrying the same
    /// materials surplus to the shrine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offering_in_flight: Option<u32>,
    /// Current HUD threat band, used to scale the warrior target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threat_band: Option<ThreatBand>,
    /// The larder is nearly empty; the leader stops staffing/training.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starving: Option<bool>,
    /// Appointed officers (role → cat id). ADDITIVE (P12.2): empty means no officer
    /// effect and byte-identical director output. Legacy rows without it load empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub officers: BTreeMap<OfficerRole, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LeaderDecision {
    Hunt {
        count: u32,
    },
    CancelHunts,
    FetchWater {
        count: u32,
    },
    Quarry {
        count: u32,
    },
    Scout {
        count: u32,
    },
    BuildDen,
    BuildStorage,
    AssignWorkshop {
        count: u32,
    },
    AssignResearch {
        count: u32,
    },
    AssignSmithy {
        count: u32,
    },
    /// Evergreen low-priority work used only after concrete village needs are met.
    MaintainVillage {
        count: u32,
    },
    TrainWarrior {
        count: u32,
    },
    CancelTraining,
    Tithe {
        food: u32,
        refined: u32,
        blessings: u32,
    },
    /// P12.6: dispatch a `carry_offering` job — a cat-driven, spatial blessing
    /// source that consumes a genuine *materials* surplus (draws from a resource
    /// disjoint from `Tithe`'s food/refined draw, so the two never double-count the
    /// same surplus pool). See `leader_director::OFFERING_MATERIALS_AMOUNT`.
    Offering,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{LeaderDecision, LeaderHousing, LeaderResources, LeaderSnapshot, ThreatBand};

    fn full_snapshot() -> LeaderSnapshot {
        LeaderSnapshot {
            population: 24,
            workforce: Some(21.5),
            idle_cats: 7,
            employed_cats: 12,
            resources: LeaderResources {
                food: 175.25,
                refined: 6.0,
            },
            food_capacity: 400.0,
            food_drain_per_hour: Some(3.25),
            materials: 42.5,
            materials_capacity: 100.0,
            stone: 17.0,
            stone_capacity: 100.0,
            water: 88.75,
            water_capacity: 200.0,
            water_drain_per_hour: Some(2.5),
            housing: LeaderHousing {
                capacity: 28,
                committed: 4,
            },
            active_hunts: 1,
            active_quarries: 2,
            active_scouts: 3,
            active_water_fetchers: 4,
            has_quarry_site: true,
            has_water_site: true,
            has_frontier: false,
            den_plans_in_flight: 1,
            storage_plans_in_flight: 0,
            storehouse_count: 2,
            storehouse_cap: 5,
            workshops_needing_workers: 1,
            research_huts_needing_workers: Some(2),
            smithies_needing_workers: Some(3),
            has_barracks: Some(true),
            warrior_count: Some(4),
            training_in_flight: Some(1),
            offering_in_flight: Some(0),
            threat_band: Some(ThreatBand::Rising),
            starving: Some(false),
            officers: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn leader_snapshot_round_trips_with_all_fields() {
        let snapshot = full_snapshot();
        let value = serde_json::to_value(&snapshot).expect("serializes snapshot");

        assert_eq!(
            value,
            json!({
                "population": 24,
                "workforce": 21.5,
                "idleCats": 7,
                "employedCats": 12,
                "resources": { "food": 175.25, "refined": 6.0 },
                "foodCapacity": 400.0,
                "foodDrainPerHour": 3.25,
                "materials": 42.5,
                "materialsCapacity": 100.0,
                "stone": 17.0,
                "stoneCapacity": 100.0,
                "water": 88.75,
                "waterCapacity": 200.0,
                "waterDrainPerHour": 2.5,
                "housing": { "capacity": 28, "committed": 4 },
                "activeHunts": 1,
                "activeQuarries": 2,
                "activeScouts": 3,
                "activeWaterFetchers": 4,
                "hasQuarrySite": true,
                "hasWaterSite": true,
                "hasFrontier": false,
                "denPlansInFlight": 1,
                "storagePlansInFlight": 0,
                "storehouseCount": 2,
                "storehouseCap": 5,
                "workshopsNeedingWorkers": 1,
                "researchHutsNeedingWorkers": 2,
                "smithiesNeedingWorkers": 3,
                "hasBarracks": true,
                "warriorCount": 4,
                "trainingInFlight": 1,
                "offeringInFlight": 0,
                "threatBand": "rising",
                "starving": false
            })
        );

        let round_tripped: LeaderSnapshot =
            serde_json::from_value(value).expect("deserializes snapshot");
        assert_eq!(round_tripped, snapshot);
    }

    #[test]
    fn optional_snapshot_fields_can_be_absent() {
        let mut value = serde_json::to_value(full_snapshot()).expect("serializes snapshot");
        let object = value.as_object_mut().expect("snapshot is an object");
        for key in [
            "workforce",
            "foodDrainPerHour",
            "waterDrainPerHour",
            "researchHutsNeedingWorkers",
            "smithiesNeedingWorkers",
            "hasBarracks",
            "warriorCount",
            "trainingInFlight",
            "offeringInFlight",
            "threatBand",
            "starving",
            "stone",
            "stoneCapacity",
        ] {
            object.remove(key);
        }

        let snapshot: LeaderSnapshot =
            serde_json::from_value(value).expect("deserializes without optional fields");

        assert_eq!(snapshot.workforce, None);
        assert_eq!(snapshot.food_drain_per_hour, None);
        assert_eq!(snapshot.water_drain_per_hour, None);
        assert_eq!(snapshot.research_huts_needing_workers, None);
        assert_eq!(snapshot.smithies_needing_workers, None);
        assert_eq!(snapshot.has_barracks, None);
        assert_eq!(snapshot.warrior_count, None);
        assert_eq!(snapshot.training_in_flight, None);
        assert_eq!(snapshot.offering_in_flight, None);
        assert_eq!(snapshot.threat_band, None);
        assert_eq!(snapshot.starving, None);
        assert_eq!(snapshot.stone, 0.0);
        assert_eq!(snapshot.stone_capacity, 0.0);
    }

    #[test]
    fn legacy_tick_named_drain_fields_remain_read_compatible() {
        let mut value = serde_json::to_value(full_snapshot()).expect("serializes snapshot");
        let object = value.as_object_mut().expect("snapshot is an object");
        let food = object
            .remove("foodDrainPerHour")
            .expect("hourly food drain exists");
        let water = object
            .remove("waterDrainPerHour")
            .expect("hourly water drain exists");
        object.insert("foodDrainPerTick".to_owned(), food);
        object.insert("waterDrainPerTick".to_owned(), water);

        let snapshot: LeaderSnapshot =
            serde_json::from_value(value).expect("legacy drain names deserialize");
        assert_eq!(snapshot.food_drain_per_hour, Some(3.25));
        assert_eq!(snapshot.water_drain_per_hour, Some(2.5));
    }

    #[test]
    fn threat_band_round_trips_wire_literals() {
        let cases = [
            (ThreatBand::Calm, "calm"),
            (ThreatBand::Rising, "rising"),
            (ThreatBand::Imminent, "imminent"),
        ];

        for (variant, wire) in cases {
            let serialized = serde_json::to_string(&variant).expect("serializes threat band");
            assert_eq!(serialized, format!("\"{wire}\""));

            let deserialized: ThreatBand =
                serde_json::from_str(&serialized).expect("deserializes threat band");
            assert_eq!(deserialized, variant);
        }
    }

    #[test]
    fn leader_decision_variants_round_trip() {
        let cases = [
            (
                LeaderDecision::Hunt { count: 2 },
                json!({ "kind": "hunt", "count": 2 }),
            ),
            (
                LeaderDecision::CancelHunts,
                json!({ "kind": "cancel_hunts" }),
            ),
            (
                LeaderDecision::FetchWater { count: 3 },
                json!({ "kind": "fetch_water", "count": 3 }),
            ),
            (
                LeaderDecision::Quarry { count: 4 },
                json!({ "kind": "quarry", "count": 4 }),
            ),
            (
                LeaderDecision::Scout { count: 5 },
                json!({ "kind": "scout", "count": 5 }),
            ),
            (LeaderDecision::BuildDen, json!({ "kind": "build_den" })),
            (
                LeaderDecision::BuildStorage,
                json!({ "kind": "build_storage" }),
            ),
            (
                LeaderDecision::AssignWorkshop { count: 6 },
                json!({ "kind": "assign_workshop", "count": 6 }),
            ),
            (
                LeaderDecision::AssignResearch { count: 7 },
                json!({ "kind": "assign_research", "count": 7 }),
            ),
            (
                LeaderDecision::AssignSmithy { count: 8 },
                json!({ "kind": "assign_smithy", "count": 8 }),
            ),
            (
                LeaderDecision::TrainWarrior { count: 9 },
                json!({ "kind": "train_warrior", "count": 9 }),
            ),
            (
                LeaderDecision::CancelTraining,
                json!({ "kind": "cancel_training" }),
            ),
            (
                LeaderDecision::Tithe {
                    food: 20,
                    refined: 5,
                    blessings: 2,
                },
                json!({ "kind": "tithe", "food": 20, "refined": 5, "blessings": 2 }),
            ),
            (LeaderDecision::Offering, json!({ "kind": "offering" })),
        ];

        for (decision, expected) in cases {
            let value = serde_json::to_value(&decision).expect("serializes decision");
            assert_eq!(value, expected);

            let round_tripped: LeaderDecision =
                serde_json::from_value(value).expect("deserializes decision");
            assert_eq!(round_tripped, decision);
        }
    }
}
