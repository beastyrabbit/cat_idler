//! Autonomous cat decisions ported from `lib/game/catAI.ts`.

use serde::{Deserialize, Serialize};

use crate::{
    entities::{Cat, CatNeeds, MapType, Position, Resources},
    types::BuildingType,
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AutonomousAction {
    Eat,
    Drink,
    Sleep { position: Position },
    ReturnToColony,
    Flee { from: Position },
}

#[must_use]
pub fn get_autonomous_action<F>(
    cat: &Cat,
    colony_resources: &Resources,
    _colony_has_building: F,
) -> Option<AutonomousAction>
where
    F: Fn(BuildingType) -> bool,
{
    if cat.position.map == MapType::World && has_needs_critical(&cat.needs, 15.0) {
        return Some(AutonomousAction::ReturnToColony);
    }

    if cat.needs.hunger < 30.0 && colony_resources.food > 0.0 {
        return Some(AutonomousAction::Eat);
    }

    if cat.needs.thirst < 40.0 && colony_resources.water > 0.0 {
        return Some(AutonomousAction::Drink);
    }

    if cat.needs.rest < 20.0 {
        return Some(AutonomousAction::Sleep {
            position: Position {
                map: MapType::Colony,
                x: 1.0,
                y: 1.0,
            },
        });
    }

    None
}

fn has_needs_critical(needs: &CatNeeds, threshold: f64) -> bool {
    needs.hunger < threshold
        || needs.thirst < threshold
        || needs.rest < threshold
        || needs.health < threshold
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        cat_ai::{AutonomousAction, get_autonomous_action},
        entities::{Cat, CatActivity, CatNeeds, CatStats, MapType, Position, Resources, RoleXp},
        types::{BuildingType, TaskType},
    };

    fn base_cat() -> Cat {
        Cat {
            id: "cat-1".to_owned(),
            colony_id: "colony-1".to_owned(),
            name: "Moss".to_owned(),
            parent_ids: vec![None, None],
            birth_time: 0,
            death_time: None,
            stats: CatStats::default(),
            needs: CatNeeds {
                hunger: 100.0,
                thirst: 100.0,
                rest: 100.0,
                health: 100.0,
            },
            current_task: None::<TaskType>,
            position: Position {
                map: MapType::Colony,
                x: 0.0,
                y: 0.0,
            },
            destination: None,
            carrying: None,
            activity: CatActivity::Idle,
            is_pregnant: false,
            pregnancy_due_time: None,
            age_hours: 0.0,
            pregnancy_due_age_hours: None,
            pregnancy_mate_id: None,
            sprite_params: Some(BTreeMap::new()),
            specialization: None,
            role_xp: RoleXp::default(),
            skills: Default::default(),
            boosted: false,
            preferred_labors: Default::default(),
        }
    }

    fn resources(food: f64, water: f64) -> Resources {
        Resources {
            food,
            fish: 0.0,
            water,
            herbs: 0.0,
            catnip: 0.0,
            grain: 0.0,
            flour: 0.0,
            preserves: 0.0,
            medicine: 0.0,
            brew: 0.0,
            materials: 0.0,
            stone: 0.0,
            refined: 0.0,
            weapons: 0.0,
            armor: 0.0,
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
            blessings: 0.0,
        }
    }

    fn no_building(_: BuildingType) -> bool {
        false
    }

    #[test]
    fn autonomous_action_decision_table_matches_cat_ai_ts() {
        struct Case {
            name: &'static str,
            needs: CatNeeds,
            position: Position,
            resources: Resources,
            expected: Option<AutonomousAction>,
        }

        let colony = Position {
            map: MapType::Colony,
            x: 0.0,
            y: 0.0,
        };
        let world = Position {
            map: MapType::World,
            x: 5.0,
            y: 5.0,
        };
        let sleep_position = Position {
            map: MapType::Colony,
            x: 1.0,
            y: 1.0,
        };

        let cases = [
            Case {
                name: "world critical hunger returns before eating",
                needs: CatNeeds {
                    hunger: 14.999,
                    thirst: 100.0,
                    rest: 100.0,
                    health: 100.0,
                },
                position: world,
                resources: resources(10.0, 10.0),
                expected: Some(AutonomousAction::ReturnToColony),
            },
            Case {
                name: "world critical thirst returns before drinking",
                needs: CatNeeds {
                    hunger: 100.0,
                    thirst: 14.999,
                    rest: 100.0,
                    health: 100.0,
                },
                position: world,
                resources: resources(10.0, 10.0),
                expected: Some(AutonomousAction::ReturnToColony),
            },
            Case {
                name: "world critical rest returns before sleeping",
                needs: CatNeeds {
                    hunger: 100.0,
                    thirst: 100.0,
                    rest: 14.999,
                    health: 100.0,
                },
                position: world,
                resources: resources(10.0, 10.0),
                expected: Some(AutonomousAction::ReturnToColony),
            },
            Case {
                name: "world critical health returns",
                needs: CatNeeds {
                    hunger: 100.0,
                    thirst: 100.0,
                    rest: 100.0,
                    health: 14.999,
                },
                position: world,
                resources: resources(10.0, 10.0),
                expected: Some(AutonomousAction::ReturnToColony),
            },
            Case {
                name: "critical boundary at fifteen is not critical",
                needs: CatNeeds {
                    hunger: 15.0,
                    thirst: 100.0,
                    rest: 100.0,
                    health: 100.0,
                },
                position: world,
                resources: resources(10.0, 10.0),
                expected: Some(AutonomousAction::Eat),
            },
            Case {
                name: "hungry below thirty eats with food",
                needs: CatNeeds {
                    hunger: 29.999,
                    thirst: 100.0,
                    rest: 100.0,
                    health: 100.0,
                },
                position: colony,
                resources: resources(1.0, 10.0),
                expected: Some(AutonomousAction::Eat),
            },
            Case {
                name: "hunger boundary at thirty does not eat",
                needs: CatNeeds {
                    hunger: 30.0,
                    thirst: 100.0,
                    rest: 100.0,
                    health: 100.0,
                },
                position: colony,
                resources: resources(10.0, 10.0),
                expected: None,
            },
            Case {
                name: "hungry without food falls through to null",
                needs: CatNeeds {
                    hunger: 29.999,
                    thirst: 100.0,
                    rest: 100.0,
                    health: 100.0,
                },
                position: colony,
                resources: resources(0.0, 10.0),
                expected: None,
            },
            Case {
                name: "eating has priority over drinking",
                needs: CatNeeds {
                    hunger: 29.999,
                    thirst: 39.999,
                    rest: 100.0,
                    health: 100.0,
                },
                position: colony,
                resources: resources(10.0, 10.0),
                expected: Some(AutonomousAction::Eat),
            },
            Case {
                name: "thirst below forty drinks with water",
                needs: CatNeeds {
                    hunger: 100.0,
                    thirst: 39.999,
                    rest: 100.0,
                    health: 100.0,
                },
                position: colony,
                resources: resources(10.0, 1.0),
                expected: Some(AutonomousAction::Drink),
            },
            Case {
                name: "thirst boundary at forty does not drink",
                needs: CatNeeds {
                    hunger: 100.0,
                    thirst: 40.0,
                    rest: 100.0,
                    health: 100.0,
                },
                position: colony,
                resources: resources(10.0, 10.0),
                expected: None,
            },
            Case {
                name: "thirsty without water falls through to null",
                needs: CatNeeds {
                    hunger: 100.0,
                    thirst: 39.999,
                    rest: 100.0,
                    health: 100.0,
                },
                position: colony,
                resources: resources(10.0, 0.0),
                expected: None,
            },
            Case {
                name: "drinking has priority over sleeping",
                needs: CatNeeds {
                    hunger: 100.0,
                    thirst: 39.999,
                    rest: 19.999,
                    health: 100.0,
                },
                position: colony,
                resources: resources(10.0, 10.0),
                expected: Some(AutonomousAction::Drink),
            },
            Case {
                name: "rest below twenty sleeps at colony center",
                needs: CatNeeds {
                    hunger: 100.0,
                    thirst: 100.0,
                    rest: 19.999,
                    health: 100.0,
                },
                position: colony,
                resources: resources(10.0, 10.0),
                expected: Some(AutonomousAction::Sleep {
                    position: sleep_position,
                }),
            },
            Case {
                name: "rest boundary at twenty does not sleep",
                needs: CatNeeds {
                    hunger: 100.0,
                    thirst: 100.0,
                    rest: 20.0,
                    health: 100.0,
                },
                position: colony,
                resources: resources(10.0, 10.0),
                expected: None,
            },
            Case {
                name: "fine needs on world map do nothing",
                needs: CatNeeds {
                    hunger: 100.0,
                    thirst: 100.0,
                    rest: 100.0,
                    health: 100.0,
                },
                position: world,
                resources: resources(10.0, 10.0),
                expected: None,
            },
        ];

        for case in cases {
            let mut cat = base_cat();
            cat.needs = case.needs;
            cat.position = case.position;

            assert_eq!(
                get_autonomous_action(&cat, &case.resources, no_building),
                case.expected,
                "{}",
                case.name
            );
        }
    }
}
