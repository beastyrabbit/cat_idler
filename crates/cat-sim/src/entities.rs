//! Cat and colony entity state ported from `types/game.ts` plus persisted fields
//! from `db/schema.ts`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::skills::Labor;
use crate::types::{CatSpecialization, TaskType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ColonyStatus {
    #[serde(rename = "starting")]
    #[default]
    Starting,
    #[serde(rename = "thriving")]
    Thriving,
    #[serde(rename = "struggling")]
    Struggling,
    #[serde(rename = "dead")]
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum MapType {
    #[serde(rename = "colony")]
    #[default]
    Colony,
    #[serde(rename = "world")]
    World,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum CatActivity {
    #[serde(rename = "idle")]
    #[default]
    Idle,
    #[serde(rename = "traveling")]
    Traveling,
    #[serde(rename = "working")]
    Working,
    #[serde(rename = "returning")]
    Returning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CarryingKind {
    #[serde(rename = "food")]
    Food,
    /// Fresh fish caught from a finite shoreline habitat. It remains a distinct
    /// raw food material in storage instead of becoming generic food in transit.
    #[serde(rename = "fish")]
    Fish,
    #[serde(rename = "blessings")]
    Blessings,
    #[serde(rename = "materials")]
    Materials,
    /// Undressed rock carried home from a quarry.
    #[serde(rename = "stone")]
    Stone,
    /// Finished generic workshop goods moving from the station to storage.
    #[serde(rename = "refined")]
    Refined,
    #[serde(rename = "logs")]
    Logs,
    #[serde(rename = "lumber")]
    Lumber,
    #[serde(rename = "planks")]
    Planks,
    #[serde(rename = "blocks")]
    Blocks,
    #[serde(rename = "tools")]
    Tools,
    #[serde(rename = "water")]
    Water,
    #[serde(rename = "catnip")]
    Catnip,
    #[serde(rename = "grain")]
    Grain,
    /// Milled grain moving between a Mill and a physical stockpile.
    #[serde(rename = "flour")]
    Flour,
    #[serde(rename = "herbs")]
    Herbs,
    /// Raw hide carried home alongside a hunt's food.
    #[serde(rename = "hide")]
    Hide,
    /// Finished leather moving from a Tannery to physical storage.
    #[serde(rename = "leather")]
    Leather,
    /// Raw bone carried home alongside a hunt's food and hide.
    #[serde(rename = "bone")]
    Bone,
    /// Mountain ore moving into a Smelter's local input ledger.
    #[serde(rename = "ore")]
    Ore,
    /// Smelted bars moving from a Smelter to physical storage.
    #[serde(rename = "metal")]
    Metal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Resources {
    pub food: f64,
    /// Fresh fish from finite shoreline habitats. Cats eat this before generic
    /// stored food; keeping it distinct makes ecological depletion visible.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub fish: f64,
    pub water: f64,
    pub herbs: f64,
    /// Harvested catnip from visible farm plots.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub catnip: f64,
    /// Harvested grain awaiting mill processing.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub grain: f64,
    /// Milled flour awaiting conversion into food.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub flour: f64,
    pub materials: f64,
    /// Raw quarried stone. This additive field deliberately does not alias the
    /// stable generic `materials` (Supplies) field on legacy saves.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub stone: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub refined: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub weapons: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub armor: f64,
    /// Refined lumber from the wood-cutter (P12.4b: materials → planks).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub planks: f64,
    /// Newly gathered raw timber. Unlike legacy `materials`, this is only consumed by
    /// a sawmill and is never credited by the compatibility wood-cutter chain.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub logs: f64,
    /// Construction timber made exclusively from `logs` by a sawmill. Legacy `planks`
    /// remain a fallback construction stock for existing saves.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub lumber: f64,
    /// Dressed stone from the stone-prep workshop (P19.C1: raw Stone → blocks).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub blocks: f64,
    /// Finished tools from the woodworking shop (P12.4b: planks + blocks → tools).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub tools: f64,
    /// Raw plant fibre — a small passive forage trickle (P16/P19 clothing chain slice).
    /// Feeds the clothier's fibre → cloth refine, mirroring how `materials` feeds
    /// planks/blocks.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub fibre: f64,
    /// Raw hide, a byproduct credited alongside food on hunt completion (P16/P19
    /// clothing chain slice). Feeds the tannery's hide → leather refine.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub hide: f64,
    /// Raw bone recovered from hunts. This is a distinct physical stock and never
    /// aliases legacy `materials` (Supplies) or raw `stone`.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bone: f64,
    /// Woven cloth from the clothier (P16/P19 clothing chain slice: fibre → cloth).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub cloth: f64,
    /// Tanned leather from the tannery (P16/P19 clothing chain slice: hide → leather).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub leather: f64,
    /// Raw ore, a mountain-only byproduct of quarrying (P17/P19 ore→metal chain).
    /// Feeds the smelter's ore → metal refine, mirroring how `materials` feeds
    /// planks/blocks. Mountain quarry workers return it as a separate physical haul.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub ore: f64,
    /// Refined metal bars from the smelter (P17/P19 ore→metal chain: ore → metal).
    /// Feeds the smithy's bonus metal-forge cycle (`smithy::advance_metal_forge`).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub metal: f64,
    pub blessings: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CatStats {
    pub attack: f64,
    pub defense: f64,
    pub hunting: f64,
    pub medicine: f64,
    pub cleaning: f64,
    pub building: f64,
    pub leadership: f64,
    pub vision: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CatNeeds {
    pub hunger: f64,
    pub thirst: f64,
    pub rest: f64,
    pub health: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Position {
    pub map: MapType,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Carrying {
    pub kind: CarryingKind,
    pub amount: f64,
    pub job_ended_at: i64,
    /// The gather-spot stockpile id this cargo was picked up from (P16 gather spots).
    /// `None` for every normal gathering haul (hunt/quarry/fetch-water) — those credit
    /// freshly produced yield into `resources` on arrival. When `Some`, the resource was
    /// already counted in `resources` the moment the gatherer first dropped it into the
    /// gather spot, so a P16 mover's arrival only *transfers* it between piles and must
    /// never re-add it to `resources` (see `world_tick::credit_carrying`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_gather_spot: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RoleXp {
    pub hunter: f64,
    pub architect: f64,
    pub ritualist: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub warrior: f64,
}

impl RoleXp {
    #[must_use]
    pub fn is_zero(&self) -> bool {
        is_zero(&self.hunter)
            && is_zero(&self.architect)
            && is_zero(&self.ritualist)
            && is_zero(&self.warrior)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Colony {
    #[serde(rename = "_id", alias = "id")]
    pub id: String,
    pub name: String,
    pub leader_id: Option<String>,
    pub status: ColonyStatus,
    pub resources: Resources,
    pub grid_size: u32,
    pub created_at: i64,
    pub last_tick: i64,
    pub last_attack: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_seed: Option<i64>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub threat_pressure: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cat {
    #[serde(rename = "_id", alias = "id")]
    pub id: String,
    pub colony_id: String,
    pub name: String,
    pub parent_ids: Vec<Option<String>>,
    pub birth_time: i64,
    pub death_time: Option<i64>,
    pub stats: CatStats,
    pub needs: CatNeeds,
    pub current_task: Option<TaskType>,
    pub position: Position,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<Position>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carrying: Option<Carrying>,
    #[serde(default)]
    pub activity: CatActivity,
    pub is_pregnant: bool,
    pub pregnancy_due_time: Option<i64>,
    #[serde(default)]
    pub age_hours: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pregnancy_due_age_hours: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pregnancy_mate_id: Option<String>,
    pub sprite_params: Option<BTreeMap<String, Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specialization: Option<CatSpecialization>,
    #[serde(
        default,
        deserialize_with = "deserialize_role_xp_null_default",
        skip_serializing_if = "RoleXp::is_zero"
    )]
    pub role_xp: RoleXp,
    /// Continuous per-labor proficiency (P12.1). Absent/`null` in legacy rows →
    /// empty map (mirrors the `role_xp` null-default back-compat).
    #[serde(
        default,
        deserialize_with = "deserialize_skills_null_default",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub skills: BTreeMap<Labor, f64>,
    /// Player-set priority flag (P15 "cat booster"). Biases the leader director's
    /// matcher toward this cat for job/role slots — a persistent preference, not a
    /// timed effect. Absent in legacy rows → `false` (mirrors `specialization`'s
    /// null-default back-compat).
    #[serde(default)]
    pub boosted: bool,
    /// Player-maintained labor preferences. Matching treats these as a strong,
    /// bounded preference among otherwise eligible cats; they never confer
    /// eligibility or suppress emergency work.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub preferred_labors: BTreeSet<Labor>,
}

impl Cat {
    /// This cat's proficiency in `labor` (0.0 if never performed).
    #[must_use]
    pub fn skill(&self, labor: Labor) -> f64 {
        self.skills.get(&labor).copied().unwrap_or(0.0)
    }

    /// Accrue `amount` proficiency in `labor`.
    pub fn gain_skill(&mut self, labor: Labor, amount: f64) {
        *self.skills.entry(labor).or_insert(0.0) += amount;
    }
}

fn deserialize_skills_null_default<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<Labor, f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<BTreeMap<Labor, f64>>::deserialize(deserializer)?.unwrap_or_default())
}

fn deserialize_role_xp_null_default<'de, D>(deserializer: D) -> Result<RoleXp, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<RoleXp>::deserialize(deserializer)?.unwrap_or_default())
}

fn is_zero(value: &f64) -> bool {
    value.classify() == std::num::FpCategory::Zero
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        entities::{
            Carrying, CarryingKind, Cat, CatActivity, CatNeeds, CatStats, Colony, ColonyStatus,
            MapType, Position, Resources, RoleXp,
        },
        skills::Labor,
        types::{CatSpecialization, TaskType},
    };

    #[test]
    fn constructs_colony_and_cat_with_every_field() {
        let colony = Colony {
            id: "colony-1".to_owned(),
            name: "MossClan".to_owned(),
            leader_id: Some("cat-1".to_owned()),
            status: ColonyStatus::Thriving,
            resources: Resources {
                food: 10.0,
                fish: 17.0,
                water: 11.0,
                herbs: 12.0,
                catnip: 27.0,
                grain: 28.0,
                flour: 29.0,
                materials: 13.0,
                stone: 23.5,
                refined: 14.0,
                weapons: 15.0,
                armor: 16.0,
                planks: 18.0,
                logs: 30.0,
                lumber: 31.0,
                blocks: 19.0,
                tools: 20.0,
                fibre: 21.0,
                hide: 22.0,
                bone: 22.5,
                cloth: 23.0,
                leather: 24.0,
                ore: 25.0,
                metal: 26.0,
                blessings: 17.0,
            },
            grid_size: 9,
            created_at: 1000,
            last_tick: 2000,
            last_attack: 3000,
            world_seed: Some(42),
            threat_pressure: 8.5,
        };

        let cat = Cat {
            id: "cat-1".to_owned(),
            colony_id: "colony-1".to_owned(),
            name: "Poppy".to_owned(),
            parent_ids: vec![Some("cat-0".to_owned()), None],
            birth_time: 900,
            death_time: None,
            stats: CatStats {
                attack: 1.0,
                defense: 2.0,
                hunting: 3.0,
                medicine: 4.0,
                cleaning: 5.0,
                building: 6.0,
                leadership: 7.0,
                vision: 8.0,
            },
            needs: CatNeeds {
                hunger: 9.0,
                thirst: 10.0,
                rest: 11.0,
                health: 12.0,
            },
            current_task: Some(TaskType::Hunt),
            position: Position {
                map: MapType::World,
                x: 5.0,
                y: 6.0,
            },
            destination: Some(Position {
                map: MapType::Colony,
                x: 1.0,
                y: 2.0,
            }),
            carrying: Some(Carrying {
                kind: CarryingKind::Food,
                amount: 3.5,
                job_ended_at: 4000,
                source_gather_spot: None,
            }),
            activity: CatActivity::Returning,
            is_pregnant: true,
            pregnancy_due_time: Some(5000),
            age_hours: 24.5,
            pregnancy_due_age_hours: Some(30.0),
            pregnancy_mate_id: Some("cat-2".to_owned()),
            sprite_params: Some(BTreeMap::from([(
                "coat".to_owned(),
                serde_json::json!("tabby"),
            )])),
            specialization: Some(CatSpecialization::Warrior),
            role_xp: RoleXp {
                hunter: 1.0,
                architect: 2.0,
                ritualist: 3.0,
                warrior: 4.0,
            },
            skills: BTreeMap::from([(Labor::Hunt, 5.0), (Labor::Haul, 2.0)]),
            boosted: true,
            preferred_labors: Default::default(),
        };

        assert_eq!(colony.world_seed, Some(42));
        assert_eq!(colony.threat_pressure, 8.5);
        assert_eq!(cat.activity, CatActivity::Returning);
        assert_eq!(cat.role_xp.warrior, 4.0);
        assert_eq!(cat.skill(Labor::Hunt), 5.0);
        assert_eq!(cat.skill(Labor::Fight), 0.0);
        assert!(cat.boosted);
    }

    #[test]
    fn farm_and_forestry_resources_default_in_legacy_rows_and_round_trip() {
        let legacy = serde_json::json!({
            "food": 10.0,
            "water": 11.0,
            "herbs": 12.0,
            "materials": 13.0,
            "blessings": 14.0
        });
        let decoded: Resources = serde_json::from_value(legacy).expect("legacy resources decode");
        assert_eq!(decoded.catnip, 0.0);
        assert_eq!(decoded.grain, 0.0);
        assert_eq!(decoded.flour, 0.0);
        assert_eq!(decoded.logs, 0.0);
        assert_eq!(decoded.lumber, 0.0);
        assert_eq!(decoded.stone, 0.0);
        assert_eq!(decoded.bone, 0.0);
        assert_eq!(decoded.materials, 13.0);

        let populated = Resources {
            catnip: 1.0,
            grain: 2.0,
            flour: 3.0,
            logs: 4.0,
            lumber: 5.0,
            stone: 6.0,
            ..Resources::default()
        };
        let json = serde_json::to_value(&populated).expect("new resources encode");
        assert_eq!(json["catnip"], serde_json::json!(1.0));
        assert_eq!(json["grain"], serde_json::json!(2.0));
        assert_eq!(json["flour"], serde_json::json!(3.0));
        assert_eq!(json["logs"], serde_json::json!(4.0));
        assert_eq!(json["lumber"], serde_json::json!(5.0));
        assert_eq!(json["stone"], serde_json::json!(6.0));
        assert_eq!(
            serde_json::from_value::<Resources>(json).expect("new resources decode"),
            populated
        );
    }

    #[test]
    fn legacy_row_absence_uses_ts_compatible_defaults() {
        let colony: Colony = serde_json::from_value(serde_json::json!({
            "id": "colony-1",
            "name": "MossClan",
            "leaderId": null,
            "status": "starting",
            "resources": {
                "food": 1.0,
                "water": 2.0,
                "herbs": 3.0,
                "materials": 4.0,
                "blessings": 5.0
            },
            "gridSize": 7,
            "createdAt": 1000,
            "lastTick": 2000,
            "lastAttack": 3000
        }))
        .expect("legacy colony row deserializes");

        let cat: Cat = serde_json::from_value(serde_json::json!({
            "id": "cat-1",
            "colonyId": "colony-1",
            "name": "Poppy",
            "parentIds": [null, null],
            "birthTime": 900,
            "deathTime": null,
            "stats": {
                "attack": 1.0,
                "defense": 2.0,
                "hunting": 3.0,
                "medicine": 4.0,
                "cleaning": 5.0,
                "building": 6.0,
                "leadership": 7.0,
                "vision": 8.0
            },
            "needs": {
                "hunger": 9.0,
                "thirst": 10.0,
                "rest": 11.0,
                "health": 12.0
            },
            "currentTask": null,
            "position": {
                "map": "colony",
                "x": 0,
                "y": 0
            },
            "isPregnant": false,
            "pregnancyDueTime": null,
            "spriteParams": null
        }))
        .expect("legacy cat row deserializes");

        assert_eq!(colony.world_seed, None);
        assert_eq!(colony.threat_pressure, 0.0);
        assert_eq!(colony.resources.refined, 0.0);
        assert_eq!(colony.resources.weapons, 0.0);
        assert_eq!(colony.resources.armor, 0.0);
        assert_eq!(colony.resources.planks, 0.0);
        assert_eq!(colony.resources.blocks, 0.0);
        assert_eq!(colony.resources.tools, 0.0);
        assert_eq!(colony.resources.fibre, 0.0);
        assert_eq!(colony.resources.hide, 0.0);
        assert_eq!(colony.resources.cloth, 0.0);
        assert_eq!(colony.resources.leather, 0.0);

        assert_eq!(cat.destination, None);
        assert_eq!(cat.carrying, None);
        assert_eq!(cat.activity, CatActivity::Idle);
        assert_eq!(cat.age_hours, 0.0);
        assert_eq!(cat.pregnancy_due_age_hours, None);
        assert_eq!(cat.pregnancy_mate_id, None);
        assert_eq!(cat.specialization, None);
        assert_eq!(cat.role_xp.warrior, 0.0);
        assert!(cat.skills.is_empty());
        assert_eq!(cat.skill(Labor::Hunt), 0.0);
        assert!(!cat.boosted);
    }

    #[test]
    fn skills_survive_serde_round_trip_and_null_defaults_to_empty() {
        // Explicit `null` skills (legacy-shaped) load as an empty map.
        let mut json = minimal_cat_json();
        json["skills"] = serde_json::Value::Null;
        let cat: Cat = serde_json::from_value(json).expect("null skills deserializes");
        assert!(cat.skills.is_empty());

        // A populated map round-trips exactly through the wire format.
        let mut cat = cat;
        cat.gain_skill(Labor::Hunt, 3.0);
        cat.gain_skill(Labor::FetchWater, 1.5);
        cat.gain_skill(Labor::Metalwork, 4.0);
        cat.gain_skill(Labor::Scout, 2.0);
        let wire = serde_json::to_value(&cat).expect("serialize");
        assert_eq!(wire["skills"]["hunt"], serde_json::json!(3.0));
        assert_eq!(wire["skills"]["fetch_water"], serde_json::json!(1.5));
        assert_eq!(wire["skills"]["metalwork"], serde_json::json!(4.0));
        assert_eq!(wire["skills"]["scout"], serde_json::json!(2.0));
        let back: Cat = serde_json::from_value(wire).expect("round-trip");
        assert_eq!(back.skills, cat.skills);
    }

    #[test]
    fn refinement_tier_resources_round_trip_and_omit_when_zero() {
        // Zero planks/blocks/tools are omitted from the wire (skip_serializing_if).
        let zero = Resources::default();
        let wire = serde_json::to_value(&zero).expect("serialize");
        assert!(wire.get("planks").is_none());
        assert!(wire.get("blocks").is_none());
        assert!(wire.get("tools").is_none());
        assert!(wire.get("fibre").is_none());
        assert!(wire.get("hide").is_none());
        assert!(wire.get("bone").is_none());
        assert!(wire.get("cloth").is_none());
        assert!(wire.get("leather").is_none());

        // Non-zero values survive a serialize → deserialize round trip.
        let stocked = Resources {
            planks: 7.0,
            blocks: 3.5,
            tools: 2.0,
            fibre: 6.0,
            hide: 4.5,
            bone: 2.5,
            cloth: 1.5,
            leather: 0.5,
            ..Resources::default()
        };
        let wire = serde_json::to_value(&stocked).expect("serialize");
        assert_eq!(wire["planks"], serde_json::json!(7.0));
        assert_eq!(wire["blocks"], serde_json::json!(3.5));
        assert_eq!(wire["tools"], serde_json::json!(2.0));
        assert_eq!(wire["fibre"], serde_json::json!(6.0));
        assert_eq!(wire["hide"], serde_json::json!(4.5));
        assert_eq!(wire["bone"], serde_json::json!(2.5));
        assert_eq!(wire["cloth"], serde_json::json!(1.5));
        assert_eq!(wire["leather"], serde_json::json!(0.5));
        let back: Resources = serde_json::from_value(wire).expect("round-trip");
        assert_eq!(back, stocked);
    }

    #[test]
    fn empty_skills_are_omitted_from_the_wire() {
        let cat: Cat = serde_json::from_value(minimal_cat_json()).expect("deserialize");
        let wire = serde_json::to_value(&cat).expect("serialize");
        assert!(wire.get("skills").is_none());
    }

    fn minimal_colony_json() -> serde_json::Value {
        serde_json::json!({
            "id": "colony-1",
            "name": "MossClan",
            "leaderId": null,
            "status": "starting",
            "resources": {
                "food": 1.0,
                "water": 2.0,
                "herbs": 3.0,
                "materials": 4.0,
                "blessings": 5.0
            },
            "gridSize": 7,
            "createdAt": 1_781_313_000_000_i64,
            "lastTick": 1_781_313_000_000_i64,
            "lastAttack": 1_781_313_000_000_i64
        })
    }

    fn minimal_cat_json() -> serde_json::Value {
        serde_json::json!({
            "id": "cat-1",
            "colonyId": "colony-1",
            "name": "Poppy",
            "parentIds": [null, null],
            "birthTime": 1_781_313_000_000_i64,
            "deathTime": null,
            "stats": {
                "attack": 1.0,
                "defense": 2.0,
                "hunting": 3.0,
                "medicine": 4.0,
                "cleaning": 5.0,
                "building": 6.0,
                "leadership": 7.0,
                "vision": 8.0
            },
            "needs": {
                "hunger": 9.0,
                "thirst": 10.0,
                "rest": 11.0,
                "health": 12.0
            },
            "currentTask": null,
            "position": {
                "map": "world",
                "x": 0,
                "y": 0
            },
            "isPregnant": false,
            "pregnancyDueTime": null,
            "spriteParams": null
        })
    }

    #[test]
    fn current_world_seed_rows_accept_date_now_sized_seeds() {
        let mut colony = minimal_colony_json();
        colony["worldSeed"] = serde_json::json!(1_781_313_000_000_i64);

        let colony: Colony =
            serde_json::from_value(colony).expect("TS stores worldSeed as Date.now()");
        let serialized = serde_json::to_value(colony).expect("serializes colony");

        assert_eq!(
            serialized["worldSeed"],
            serde_json::json!(1_781_313_000_000_i64)
        );
    }

    #[test]
    fn role_xp_null_rows_default_like_ts_nullish_coalescing() {
        let mut cat = minimal_cat_json();
        cat["roleXp"] = serde_json::Value::Null;

        let cat: Cat = serde_json::from_value(cat).expect("TS reads roleXp ?? DEFAULT_ROLE_XP");

        assert_eq!(cat.role_xp, RoleXp::default());
    }

    #[test]
    fn moving_cat_positions_accept_fractional_ts_coordinates() {
        let mut cat = minimal_cat_json();
        cat["position"]["x"] = serde_json::json!(0.5);

        serde_json::from_value::<Cat>(cat)
            .expect("TS movement persists fractional number coordinates while traveling");
    }

    #[test]
    fn sprite_params_accept_ts_record_unknown_values() {
        let mut cat = minimal_cat_json();
        cat["spriteParams"] = serde_json::json!({
            "spriteNumber": 3,
            "peltName": "CLASSIC",
            "shading": true,
            "accessories": []
        });

        let cat: Cat =
            serde_json::from_value(cat).expect("spriteParams is Record<string, unknown>");
        let serialized = serde_json::to_value(cat).expect("serializes cat");

        assert_eq!(
            serialized["spriteParams"]["spriteNumber"],
            serde_json::json!(3)
        );
        assert_eq!(
            serialized["spriteParams"]["shading"],
            serde_json::json!(true)
        );
        assert_eq!(
            serialized["spriteParams"]["accessories"],
            serde_json::json!([])
        );
    }

    #[test]
    fn carrying_preserves_job_ended_at_for_deposit_grace() {
        let mut cat = minimal_cat_json();
        cat["carrying"] = serde_json::json!({
            "kind": "food",
            "amount": 8.0,
            "jobEndedAt": 1_781_313_000_000_i64
        });

        let cat: Cat = serde_json::from_value(cat).expect("TS carrying row deserializes");
        let serialized = serde_json::to_value(cat).expect("serializes cat");

        assert_eq!(
            serialized["carrying"]["jobEndedAt"],
            serde_json::json!(1_781_313_000_000_i64)
        );
    }
}
