use cat_protocol::{
    BlackHoleAction, BlackHoleAxis, BlackHoleAxisState, BlackHoleFeedLine, BlackHoleFeedOrder,
    BlackHoleIntakeTiming, BlackHoleLevel, BlackHoleLifetimeTotals, BlackHoleSnapshot,
    ResourceKind,
};
use serde_json::json;

#[test]
fn snapshot_round_trips_with_exact_opening_and_credit_totals() {
    let snapshot = BlackHoleSnapshot {
        building_id: "the-hole".to_owned(),
        axes: vec![BlackHoleAxisState {
            axis: BlackHoleAxis::Width,
            physical_level: BlackHoleLevel::new(3).unwrap(),
            researched_level: BlackHoleLevel::new(4).unwrap(),
        }],
        intake: BlackHoleIntakeTiming {
            opening_index: 42,
            next_opens_at_ms: Some(30_000),
        },
        active_feed_order: Some(BlackHoleFeedOrder {
            id: "feed-1".to_owned(),
            opening_index: 42,
            line: BlackHoleFeedLine {
                resource: ResourceKind::Food,
                planned_units: 10,
                delivered_units: 4,
                credited_units: 4,
                credited_value_micros: 400_000,
            },
            carrier_cat_id: Some("cat-1".to_owned()),
            waiting_for_opening: true,
        }),
        active_project: None,
        accepted_resources: Vec::new(),
        accepted_items: Vec::new(),
        lifetime_totals: BlackHoleLifetimeTotals {
            credited_units: 65,
            credited_value_micros: 12_300_000,
            opening_count: 11,
        },
        next_review_at_ms: Some(99_000),
        urged: false,
    };
    let wire = serde_json::to_value(&snapshot).unwrap();
    assert_eq!(wire["axes"][0]["axis"], json!("width"));
    assert_eq!(wire["intake"]["openingIndex"], json!(42));
    assert_eq!(
        serde_json::from_value::<BlackHoleSnapshot>(wire).unwrap(),
        snapshot
    );
}

#[test]
fn nudge_is_an_auth_only_additive_action() {
    let action = BlackHoleAction::NudgeBlackHole {
        session_id: "session".to_owned(),
        nickname: "Observer".to_owned(),
        sig: "signed".to_owned(),
    };
    let wire = serde_json::to_value(&action).unwrap();
    assert_eq!(wire["action"], json!("nudgeBlackHole"));
    assert_eq!(wire.as_object().unwrap().len(), 4);
}

#[test]
fn levels_above_ten_are_rejected() {
    assert!(BlackHoleLevel::new(11).is_err());
    assert!(serde_json::from_str::<BlackHoleLevel>("11").is_err());
}
