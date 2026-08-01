pub use cat_protocol::{ResourceKind, TilePoint};
use serde_json::json;

#[path = "../src/hunting_lair.rs"]
mod hunting_lair;

use hunting_lair::{
    CaptainHuntingAdviceSnapshot, CaptainRiskBand, HuntingCombatOutcomeSnapshot,
    HuntingCombatResult, HuntingDanger, HuntingFirstClearTrophySnapshot, HuntingLairAction,
    HuntingLairSnapshot, HuntingLootDescriptor, HuntingLootLikelihood, HuntingLootPreview,
    HuntingMemberCondition, HuntingMemberOutcomeSnapshot, HuntingMonsterSnapshot,
    HuntingMonsterStatus, HuntingPartySnapshot, HuntingPartyStatus, HuntingRespawnSnapshot,
    HuntingRewardSnapshot, HuntingTrophyStatus, RevealedHuntingSiteSnapshot,
};

fn sample_snapshot() -> HuntingLairSnapshot {
    HuntingLairSnapshot {
        building_id: "hunting-lair-1".to_owned(),
        revealed_sites: vec![
            RevealedHuntingSiteSnapshot {
                id: "hunt-site-thornwood".to_owned(),
                display_name: "Thornwood Hollow".to_owned(),
                position: TilePoint { x: 18, y: -4 },
                danger: HuntingDanger::High,
                monsters: vec![HuntingMonsterSnapshot {
                    id: "monster-thorn-boar-1".to_owned(),
                    species_id: "thorn_boar".to_owned(),
                    display_name: "Thorn Boar".to_owned(),
                    status: HuntingMonsterStatus::Available,
                    respawn: None,
                }],
                first_clear_trophy: Some(HuntingFirstClearTrophySnapshot {
                    trophy_id: "trophy-thorn-tusk".to_owned(),
                    display_name: "Thorn Tusk".to_owned(),
                    status: HuntingTrophyStatus::Available,
                }),
                loot_preview: vec![
                    HuntingLootPreview {
                        loot: HuntingLootDescriptor::Resource {
                            resource: ResourceKind::Food,
                        },
                        minimum_quantity: 4,
                        maximum_quantity: 8,
                        likelihood: HuntingLootLikelihood::Guaranteed,
                    },
                    HuntingLootPreview {
                        loot: HuntingLootDescriptor::Item {
                            item_kind: "trinket".to_owned(),
                            material: "bone".to_owned(),
                            minimum_quality: 1,
                            maximum_quality: 2,
                        },
                        minimum_quantity: 1,
                        maximum_quantity: 2,
                        likelihood: HuntingLootLikelihood::Possible,
                    },
                ],
            },
            RevealedHuntingSiteSnapshot {
                id: "hunt-site-old-quarry".to_owned(),
                display_name: "Old Quarry Den".to_owned(),
                position: TilePoint { x: -9, y: 12 },
                danger: HuntingDanger::Moderate,
                monsters: vec![HuntingMonsterSnapshot {
                    id: "monster-stone-marten-1".to_owned(),
                    species_id: "stone_marten".to_owned(),
                    display_name: "Stone Marten".to_owned(),
                    status: HuntingMonsterStatus::Respawning,
                    respawn: Some(HuntingRespawnSnapshot {
                        respawns_at_ms: 98_765,
                    }),
                }],
                first_clear_trophy: Some(HuntingFirstClearTrophySnapshot {
                    trophy_id: "trophy-quarry-tail".to_owned(),
                    display_name: "Quarry Tail".to_owned(),
                    status: HuntingTrophyStatus::Claimed {
                        colony_id: "colony-1".to_owned(),
                        party_id: "hunt-party-previous".to_owned(),
                        claimed_at_ms: 12_345,
                    },
                }),
                loot_preview: Vec::new(),
            },
        ],
        captain_advice: vec![CaptainHuntingAdviceSnapshot {
            site_id: "hunt-site-thornwood".to_owned(),
            risk_band: CaptainRiskBand::Risky,
            summary: "Bring a full, equipped party.".to_owned(),
            recommended_party_size: 3,
        }],
        active_parties: vec![HuntingPartySnapshot {
            id: "hunt-party-7".to_owned(),
            site_id: "hunt-site-thornwood".to_owned(),
            leader_cat_id: "cat-maple".to_owned(),
            member_cat_ids: vec!["cat-maple".to_owned(), "cat-ash".to_owned()],
            status: HuntingPartyStatus::Traveling,
            departed_at_ms: 20_000,
            expected_phase_end_at_ms: Some(30_000),
        }],
        recent_outcomes: vec![HuntingCombatOutcomeSnapshot {
            id: "hunt-outcome-6".to_owned(),
            party_id: "hunt-party-previous".to_owned(),
            site_id: "hunt-site-old-quarry".to_owned(),
            resolved_at_ms: 12_345,
            result: HuntingCombatResult::Victory,
            members: vec![HuntingMemberOutcomeSnapshot {
                cat_id: "cat-maple".to_owned(),
                condition: HuntingMemberCondition::Injured,
            }],
            monster_statuses: vec![HuntingMonsterSnapshot {
                id: "monster-stone-marten-1".to_owned(),
                species_id: "stone_marten".to_owned(),
                display_name: "Stone Marten".to_owned(),
                status: HuntingMonsterStatus::Respawning,
                respawn: Some(HuntingRespawnSnapshot {
                    respawns_at_ms: 98_765,
                }),
            }],
            rewards: vec![
                HuntingRewardSnapshot::Resource {
                    resource: ResourceKind::Food,
                    quantity: 6,
                },
                HuntingRewardSnapshot::SpeciesMaterial {
                    material: "badger_pelt".to_owned(),
                    count: 1,
                },
            ],
            first_clear_trophy_id: Some("trophy-quarry-tail".to_owned()),
        }],
        nudged_site_id: Some("hunt-site-thornwood".to_owned()),
    }
}

#[test]
fn public_hunting_lair_snapshot_round_trips_with_stable_ids_and_bands() {
    let snapshot = sample_snapshot();
    let wire = serde_json::to_value(&snapshot).expect("serialize Hunting Lair snapshot");

    assert_eq!(wire["buildingId"], json!("hunting-lair-1"));
    assert_eq!(
        wire["revealedSites"][0]["monsters"][0]["speciesId"],
        json!("thorn_boar")
    );
    assert_eq!(wire["revealedSites"][0]["danger"], json!("high"));
    assert_eq!(
        wire["revealedSites"][0]["lootPreview"][1]["loot"]["kind"],
        json!("item")
    );
    assert_eq!(wire["captainAdvice"][0]["riskBand"], json!("risky"));
    assert_eq!(
        wire["recentOutcomes"][0]["rewards"][1]["material"],
        json!("badger_pelt")
    );
    assert_eq!(
        wire["recentOutcomes"][0]["rewards"][1]["kind"],
        json!("species_material")
    );
    assert_eq!(
        serde_json::from_value::<HuntingLairSnapshot>(wire)
            .expect("deserialize Hunting Lair snapshot"),
        snapshot
    );
}

#[test]
fn snapshot_exposes_public_bands_and_results_without_hidden_combat_state() {
    let wire = serde_json::to_value(sample_snapshot()).expect("serialize snapshot");
    let encoded = wire.to_string();

    for forbidden in [
        "serverSeed",
        "rngState",
        "combatRoll",
        "hitPoints",
        "exactDropChance",
        "internalScore",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "public snapshot leaked hidden field {forbidden}"
        );
    }

    let mut invalid = wire;
    invalid
        .as_object_mut()
        .expect("snapshot object")
        .insert("serverSeed".to_owned(), json!(42));
    assert!(
        serde_json::from_value::<HuntingLairSnapshot>(invalid).is_err(),
        "unknown hidden state must not deserialize into the public DTO"
    );
}

#[test]
fn player_nudge_may_target_one_revealed_site_or_leave_selection_to_the_captain() {
    let targeted = HuntingLairAction::NudgeHuntingSite {
        session_id: "session".to_owned(),
        nickname: "Observer".to_owned(),
        sig: "signed".to_owned(),
        site_id: Some("hunt-site-thornwood".to_owned()),
    };
    let targeted_wire = serde_json::to_value(&targeted).expect("serialize targeted nudge");
    assert_eq!(targeted_wire["action"], json!("nudgeHuntingSite"));
    assert_eq!(targeted_wire["siteId"], json!("hunt-site-thornwood"));

    let untargeted = HuntingLairAction::NudgeHuntingSite {
        session_id: "session".to_owned(),
        nickname: "Observer".to_owned(),
        sig: "signed".to_owned(),
        site_id: None,
    };
    let untargeted_wire = serde_json::to_value(&untargeted).expect("serialize general nudge");
    assert_eq!(untargeted_wire["action"], json!("nudgeHuntingSite"));
    assert!(
        untargeted_wire.get("siteId").is_none(),
        "an absent target stays absent rather than becoming an empty sentinel id"
    );
    assert_eq!(
        serde_json::from_value::<HuntingLairAction>(untargeted_wire)
            .expect("deserialize general nudge"),
        untargeted
    );
}

#[test]
fn strict_nested_dtos_reject_unknown_exact_state() {
    let monster = json!({
        "id": "monster-1",
        "speciesId": "thorn_boar",
        "displayName": "Thorn Boar",
        "status": "engaged",
        "respawn": null,
        "exactAttack": 93
    });

    assert!(serde_json::from_value::<HuntingMonsterSnapshot>(monster).is_err());
}
