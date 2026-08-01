mod rng {
    pub use cat_sim::rng::*;
}

#[path = "../src/hunting_lair.rs"]
mod hunting_lair;

use hunting_lair::{
    AttemptAuthority, HuntAdvice, HuntError, Hunter, HuntingLair, MonsterSpecies, SpeciesMaterial,
    attempt_is_authorized, generate_roster, resolve_attempt,
};

fn hunter(id: &str, power: f64, health_percent: f64) -> Hunter {
    Hunter {
        cat_id: id.to_owned(),
        combat_power: power,
        health_percent,
        weapon_bonus: 0.0,
        armor_bonus: 0.0,
    }
}

#[test]
fn danger_thresholds_control_roster_size_and_species_pool() {
    for seed in 1..=64 {
        let low = generate_roster(84, seed, 0);
        assert_eq!(low.monsters.len(), 1);
        assert!(
            low.monsters
                .iter()
                .all(|monster| monster.species == MonsterSpecies::Fox)
        );

        let medium = generate_roster(85, seed, 0);
        assert_eq!(medium.monsters.len(), 2);
        assert!(medium.monsters.iter().all(|monster| matches!(
            monster.species,
            MonsterSpecies::Fox | MonsterSpecies::Badger
        )));

        let hard = generate_roster(90, seed, 0);
        assert_eq!(hard.monsters.len(), 2);
        assert!(hard.monsters.iter().all(|monster| matches!(
            monster.species,
            MonsterSpecies::Fox | MonsterSpecies::Badger | MonsterSpecies::Bear
        )));

        let deadly = generate_roster(95, seed, 0);
        assert_eq!(deadly.monsters.len(), 3);
        assert!(deadly.monsters.iter().all(|monster| matches!(
            monster.species,
            MonsterSpecies::Fox
                | MonsterSpecies::Badger
                | MonsterSpecies::Bear
                | MonsterSpecies::RivalBeast
        )));
    }
}

#[test]
fn generation_is_seeded_stable_and_generation_sensitive() {
    let first = generate_roster(100, 42, 7);
    let replay = generate_roster(100, 42, 7);
    let next_generation = generate_roster(100, 42, 8);

    assert_eq!(first, replay);
    assert_ne!(first.monsters, next_generation.monsters);
}

#[test]
fn current_danger_sums_only_living_threat_and_caps_at_one_hundred() {
    let mut lair = HuntingLair::from_species(
        100,
        [
            MonsterSpecies::RivalBeast,
            MonsterSpecies::Bear,
            MonsterSpecies::Fox,
        ],
    );
    assert_eq!(lair.current_danger(), 100);

    lair.monsters[0].health = 0;
    assert_eq!(lair.current_danger(), 80);
}

#[test]
fn advice_and_authority_gates_match_captain_risk_bands() {
    assert_eq!(HuntAdvice::from_success_percent(49), HuntAdvice::Reckless);
    assert_eq!(HuntAdvice::from_success_percent(50), HuntAdvice::Risky);
    assert_eq!(HuntAdvice::from_success_percent(69), HuntAdvice::Risky);
    assert_eq!(HuntAdvice::from_success_percent(70), HuntAdvice::Favored);
    assert_eq!(HuntAdvice::from_success_percent(89), HuntAdvice::Favored);
    assert_eq!(HuntAdvice::from_success_percent(90), HuntAdvice::Safe);

    assert!(attempt_is_authorized(
        AttemptAuthority::AutonomousLeader,
        70,
        &[hunter("a", 70.0, 70.0)]
    ));
    assert!(!attempt_is_authorized(
        AttemptAuthority::AutonomousLeader,
        69,
        &[hunter("a", 70.0, 100.0)]
    ));
    assert!(attempt_is_authorized(
        AttemptAuthority::PlayerNudge,
        45,
        &[hunter("a", 45.0, 80.0)]
    ));
    assert!(!attempt_is_authorized(
        AttemptAuthority::PlayerNudge,
        45,
        &[hunter("a", 45.0, 79.0)]
    ));
}

#[test]
fn party_cap_is_validated_without_silently_dropping_hunters() {
    let lair = HuntingLair::from_species(84, [MonsterSpecies::Fox]);
    let party = [hunter("a", 100.0, 100.0), hunter("b", 100.0, 100.0)];

    assert_eq!(
        resolve_attempt(&lair, &party, 1, 1, 10_000, 1_000),
        Err(HuntError::PartyTooLarge {
            supplied: 2,
            cap: 1
        })
    );
    assert_eq!(
        resolve_attempt(&lair, &[], 3, 1, 10_000, 1_000),
        Err(HuntError::EmptyParty)
    );
    assert_eq!(
        resolve_attempt(&lair, &party[..1], 1, 1, 10_000, 0),
        Err(HuntError::InvalidGameHourDuration(0))
    );
}

#[test]
fn victorious_clear_awards_base_loot_both_xp_tracks_and_first_clear_trophy() {
    let lair = HuntingLair::from_species(
        100,
        [
            MonsterSpecies::Fox,
            MonsterSpecies::Badger,
            MonsterSpecies::Bear,
            MonsterSpecies::RivalBeast,
        ],
    );
    let party = [hunter("a", 500.0, 100.0), hunter("b", 500.0, 100.0)];

    let result = resolve_attempt(&lair, &party, 3, 123, 50_000, 1_000).unwrap();

    assert!(result.cleared);
    assert_eq!(result.lair.current_danger(), 0);
    assert_eq!(result.loot.food, 105);
    assert_eq!(result.loot.hide, 19);
    assert_eq!(result.loot.bone, 13);
    assert!(result.first_clear_trophy.is_some());
    assert!(result.lair.first_clear_claimed);
    assert_eq!(result.lair.respawn_ready_at_ms, Some(68_000));
    assert!(result.participants.iter().all(|entry| {
        entry.hunting_xp > 0 && entry.fight_xp > 0 && !entry.died && entry.damage == 0
    }));
}

#[test]
fn first_clear_guarantees_strongest_species_material_when_natural_drops_miss() {
    let lair = HuntingLair::from_species(84, [MonsterSpecies::Fox]);
    let result = resolve_attempt(&lair, &[hunter("a", 500.0, 100.0)], 1, 1, 0, 1_000).unwrap();

    assert!(result.cleared);
    assert_eq!(
        result.loot.species_materials,
        vec![SpeciesMaterial::FoxPelt]
    );
    assert_eq!(
        result.first_clear_trophy.unwrap().species,
        MonsterSpecies::Fox
    );
}

#[test]
fn a_later_clear_does_not_repeat_trophy_or_guaranteed_drop() {
    let mut lair = HuntingLair::from_species(84, [MonsterSpecies::Fox]);
    lair.first_clear_claimed = true;

    let result = resolve_attempt(&lair, &[hunter("a", 500.0, 100.0)], 1, 1, 0, 1_000).unwrap();

    assert!(result.cleared);
    assert!(result.first_clear_trophy.is_none());
    assert!(result.loot.species_materials.is_empty());
}

#[test]
fn respawn_waits_for_cooldown_then_uses_next_deterministic_generation() {
    let lair = HuntingLair::from_species(90, [MonsterSpecies::Bear]);
    let cleared =
        resolve_attempt(&lair, &[hunter("a", 500.0, 100.0)], 1, 99, 100_000, 1_000).unwrap();

    assert_eq!(cleared.lair.respawn_ready_at_ms, Some(112_000));
    assert_eq!(cleared.lair.respawn_if_ready(111_999, 99), None);

    let respawned = cleared.lair.respawn_if_ready(112_000, 99).unwrap();
    assert_eq!(respawned.generation, 1);
    assert_eq!(respawned.monsters.len(), 2);
    assert!(respawned.monsters.iter().all(|monster| monster.is_alive()));
    assert_eq!(respawned.respawn_ready_at_ms, None);
}

#[test]
fn failed_hunt_reports_damage_death_and_both_xp_tracks_without_loot() {
    let lair = HuntingLair::from_species(100, [MonsterSpecies::RivalBeast]);
    let result = resolve_attempt(&lair, &[hunter("fragile", 0.0, 10.0)], 1, 123, 0, 1_000).unwrap();

    assert!(!result.cleared);
    assert_eq!(result.lair, lair);
    assert_eq!(result.loot.food, 0);
    assert_eq!(result.loot.hide, 0);
    assert_eq!(result.loot.bone, 0);
    assert_eq!(result.participants.len(), 1);
    assert!(result.participants[0].damage > 0);
    assert!(result.participants[0].died);
    assert!(result.participants[0].hunting_xp > 0);
    assert!(result.participants[0].fight_xp > 0);
}
