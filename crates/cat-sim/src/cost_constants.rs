//! Combat, building, and task-skill constants ported from `types/game.ts`.

use crate::types::{BuildingType, EnemyType, TaskType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyStats {
    pub base_clicks: u32,
    pub damage: (u32, u32),
}

pub const ENEMY_STATS: [(EnemyType, EnemyStats); 5] = [
    (
        EnemyType::Fox,
        EnemyStats {
            base_clicks: 20,
            damage: (30, 50),
        },
    ),
    (
        EnemyType::Hawk,
        EnemyStats {
            base_clicks: 15,
            damage: (20, 40),
        },
    ),
    (
        EnemyType::Badger,
        EnemyStats {
            base_clicks: 40,
            damage: (40, 60),
        },
    ),
    (
        EnemyType::Bear,
        EnemyStats {
            base_clicks: 75,
            damage: (50, 70),
        },
    ),
    (
        EnemyType::RivalCat,
        EnemyStats {
            base_clicks: 30,
            damage: (30, 50),
        },
    ),
];

pub const BUILDING_COSTS: [(BuildingType, u32); 23] = [
    (BuildingType::Den, 0),
    (BuildingType::FoodStorage, 5),
    (BuildingType::WaterBowl, 3),
    (BuildingType::Beds, 8),
    (BuildingType::HerbGarden, 10),
    (BuildingType::Nursery, 12),
    (BuildingType::ElderCorner, 10),
    (BuildingType::Walls, 15),
    (BuildingType::MouseFarm, 25),
    (BuildingType::Shrine, 0),
    (BuildingType::Workshop, 20),
    (BuildingType::Field, 15),
    (BuildingType::Smithy, 30),
    (BuildingType::Barracks, 30),
    (BuildingType::AccountingTent, 15),
    (BuildingType::WoodCutter, 20),
    (BuildingType::StonePrep, 20),
    (BuildingType::Woodworking, 25),
    // P16/P19 clothing chain slice — same tier as the wood-cutter/stone-prep raw
    // refinement benches they mirror.
    (BuildingType::Clothier, 20),
    (BuildingType::Tannery, 20),
    // P17/P19 ore→metal chain — same era-3 tier as the smithy it feeds.
    (BuildingType::Smelter, 30),
    // Cat-research entry building (port extension, no TS BUILDING_COSTS entry) — mid-tier
    // like the field/accounting tent. The autonomous build actually pays the shared
    // plank/block scaffold cost, not this value; it only surfaces in the client inspector.
    (BuildingType::ResearchHut, 15),
    // Second research building, unlocked by the "school" upgrade node (era 2) — same
    // mid tier as the research hut it mirrors. Built via `PlanBuilding` (gated on owning
    // the "school" node) rather than autonomously commissioned, so this is purely the
    // client-inspector/cost-preview value; the actual build still pays the shared
    // plank/block scaffold cost like every other `BuildHouse` job.
    (BuildingType::School, 15),
];

pub const TASK_TO_SKILL: [(TaskType, &str); 12] = [
    (TaskType::Hunt, "hunting"),
    (TaskType::GatherHerbs, "medicine"),
    (TaskType::FetchWater, "hunting"),
    (TaskType::Clean, "cleaning"),
    (TaskType::Build, "building"),
    (TaskType::Guard, "defense"),
    (TaskType::Heal, "medicine"),
    (TaskType::Kitsit, "leadership"),
    (TaskType::Explore, "vision"),
    (TaskType::Patrol, "attack"),
    (TaskType::Teach, "leadership"),
    (TaskType::Rest, "defense"),
];

#[must_use]
pub fn enemy_stats(enemy_type: EnemyType) -> EnemyStats {
    ENEMY_STATS
        .iter()
        .find_map(|(key, stats)| (*key == enemy_type).then_some(*stats))
        .expect("ENEMY_STATS covers every EnemyType variant")
}

#[must_use]
pub fn building_cost(building_type: BuildingType) -> u32 {
    BUILDING_COSTS
        .iter()
        .find_map(|(key, cost)| (*key == building_type).then_some(*cost))
        .expect("BUILDING_COSTS covers every BuildingType variant")
}

#[must_use]
pub fn task_skill(task_type: TaskType) -> &'static str {
    TASK_TO_SKILL
        .iter()
        .find_map(|(key, skill)| (*key == task_type).then_some(*skill))
        .expect("TASK_TO_SKILL covers every TaskType variant")
}

#[cfg(test)]
mod tests {
    use crate::types::{BuildingType, EnemyType, TaskType};

    use super::{
        BUILDING_COSTS, ENEMY_STATS, TASK_TO_SKILL, building_cost, enemy_stats, task_skill,
    };

    #[test]
    fn enemy_stats_match_typescript_record_for_every_enemy_type() {
        let expected = [
            (EnemyType::Fox, 20, (30, 50)),
            (EnemyType::Hawk, 15, (20, 40)),
            (EnemyType::Badger, 40, (40, 60)),
            (EnemyType::Bear, 75, (50, 70)),
            (EnemyType::RivalCat, 30, (30, 50)),
        ];

        assert_eq!(ENEMY_STATS.len(), EnemyType::ALL.len());
        assert_eq!(expected.len(), EnemyType::ALL.len());

        for enemy_type in EnemyType::ALL {
            let entries: Vec<_> = ENEMY_STATS
                .iter()
                .filter(|(key, _)| key == enemy_type)
                .collect();
            assert_eq!(
                entries.len(),
                1,
                "{enemy_type:?} must have exactly one entry"
            );

            let expected_entry = expected
                .iter()
                .find(|(key, _, _)| key == enemy_type)
                .expect("expected TS fixture covers enum variant");
            let actual = entries[0].1;
            assert_eq!(actual.base_clicks, expected_entry.1);
            assert_eq!(actual.damage, expected_entry.2);
            assert_eq!(enemy_stats(*enemy_type), actual);
        }
    }

    #[test]
    fn building_costs_match_typescript_record_for_every_building_type() {
        let expected = [
            (BuildingType::Den, 0),
            (BuildingType::FoodStorage, 5),
            (BuildingType::WaterBowl, 3),
            (BuildingType::Beds, 8),
            (BuildingType::HerbGarden, 10),
            (BuildingType::Nursery, 12),
            (BuildingType::ElderCorner, 10),
            (BuildingType::Walls, 15),
            (BuildingType::MouseFarm, 25),
            (BuildingType::Shrine, 0),
            (BuildingType::Workshop, 20),
            (BuildingType::Field, 15),
            (BuildingType::Smithy, 30),
            (BuildingType::Barracks, 30),
            (BuildingType::AccountingTent, 15),
            (BuildingType::WoodCutter, 20),
            (BuildingType::StonePrep, 20),
            (BuildingType::Woodworking, 25),
            (BuildingType::Clothier, 20),
            (BuildingType::Tannery, 20),
            (BuildingType::Smelter, 30),
            (BuildingType::ResearchHut, 15),
            (BuildingType::School, 15),
        ];

        assert_eq!(BUILDING_COSTS.len(), BuildingType::ALL.len());
        assert_eq!(expected.len(), BuildingType::ALL.len());

        for building_type in BuildingType::ALL {
            let entries: Vec<_> = BUILDING_COSTS
                .iter()
                .filter(|(key, _)| key == building_type)
                .collect();
            assert_eq!(
                entries.len(),
                1,
                "{building_type:?} must have exactly one entry"
            );

            let expected_entry = expected
                .iter()
                .find(|(key, _)| key == building_type)
                .expect("expected TS fixture covers enum variant");
            assert_eq!(entries[0].1, expected_entry.1);
            assert_eq!(building_cost(*building_type), expected_entry.1);
        }
    }

    #[test]
    fn task_to_skill_matches_typescript_record_for_every_task_type() {
        let expected = [
            (TaskType::Hunt, "hunting"),
            (TaskType::GatherHerbs, "medicine"),
            (TaskType::FetchWater, "hunting"),
            (TaskType::Clean, "cleaning"),
            (TaskType::Build, "building"),
            (TaskType::Guard, "defense"),
            (TaskType::Heal, "medicine"),
            (TaskType::Kitsit, "leadership"),
            (TaskType::Explore, "vision"),
            (TaskType::Patrol, "attack"),
            (TaskType::Teach, "leadership"),
            (TaskType::Rest, "defense"),
        ];

        assert_eq!(TASK_TO_SKILL.len(), TaskType::ALL.len());
        assert_eq!(expected.len(), TaskType::ALL.len());

        for task_type in TaskType::ALL {
            let entries: Vec<_> = TASK_TO_SKILL
                .iter()
                .filter(|(key, _)| key == task_type)
                .collect();
            assert_eq!(
                entries.len(),
                1,
                "{task_type:?} must have exactly one entry"
            );

            let expected_entry = expected
                .iter()
                .find(|(key, _)| key == task_type)
                .expect("expected TS fixture covers enum variant");
            assert_eq!(entries[0].1, expected_entry.1);
            assert_eq!(task_skill(*task_type), expected_entry.1);
        }
    }
}
