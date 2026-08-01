//! Focused Black Hole domain tests.
//!
use cat_sim::{
    black_hole::{
        BlackHoleAxes, BlackHoleAxis, FeedCandidate, FeedKind, FeedSource, IntakeState,
        VALUE_MICROS, item_darkness_requirement, max_order, max_quality_for_darkness,
        resource_darkness_requirement, resource_unit_value_micros, upgrade_recipe,
    },
    items::{Item, ItemKind, Material},
    stockpiles::ResourceKind,
};

#[test]
fn axes_validate_zero_to_ten_and_derive_rules() {
    let axes = BlackHoleAxes::new(3, 4, 8).expect("valid axes");

    assert_eq!(axes.intake_width(), 4);
    assert_eq!(axes.max_order(), 50);
    assert_eq!(axes.max_quality(), 2);
    assert_eq!(max_order(0), 10);
    assert_eq!(max_order(10), 110);
    assert_eq!(max_quality_for_darkness(0), 0);
    assert_eq!(max_quality_for_darkness(7), 2);
    assert_eq!(max_quality_for_darkness(10), 4);
    assert!(BlackHoleAxes::new(11, 0, 0).is_err());
}

#[test]
fn ten_logs_feed_as_four_four_two_at_width_level_three() {
    let axes = BlackHoleAxes::new(3, 0, 2).expect("width four, logs unlocked");
    let mut state = IntakeState::new();
    let mut candidates = [FeedCandidate::resource(ResourceKind::Logs, 1, 10)];

    let first = state.intake(axes, &mut candidates);
    let second = state.intake(axes, &mut candidates);
    let third = state.intake(axes, &mut candidates);
    let empty = state.intake(axes, &mut candidates);

    assert_eq!(first.total_quantity, 4);
    assert_eq!(second.total_quantity, 4);
    assert_eq!(third.total_quantity, 2);
    assert_eq!(empty.total_quantity, 0);
    assert_eq!(
        first
            .credits
            .iter()
            .map(|credit| credit.opening_index)
            .collect::<Vec<_>>(),
        vec![0, 0, 0, 0]
    );
    assert_eq!(
        third
            .credits
            .iter()
            .map(|credit| credit.opening_index)
            .collect::<Vec<_>>(),
        vec![2, 2]
    );
    assert_eq!(state.lifetime.quantity, 10);
    assert_eq!(state.lifetime.openings, 3);
    assert_eq!(state.lifetime.value_micros, VALUE_MICROS);
    assert_eq!(candidates[0].quantity, 0);
}

#[test]
fn width_level_zero_consumes_ten_logs_across_ten_openings() {
    let axes = BlackHoleAxes::new(0, 0, 2).expect("width one, logs unlocked");
    let mut state = IntakeState::new();
    let mut candidates = [FeedCandidate::resource(ResourceKind::Logs, 1, 10)];

    let mut credited = Vec::new();
    for _ in 0..10 {
        let report = state.intake(axes, &mut candidates);
        assert_eq!(report.total_quantity, 1);
        credited.push(report.credits[0].opening_index);
    }

    assert_eq!(credited, (0_u64..10).collect::<Vec<_>>());
    assert_eq!(state.next_opening_index, 10);
    assert_eq!(state.lifetime.quantity, 10);
}

#[test]
fn depth_and_darkness_gate_candidate_order_and_quality() {
    let common_mug = Item::new(ItemKind::Mug, Material::Wood, 1);
    let crude_mug = Item::new(ItemKind::Mug, Material::Wood, 0);
    let mut candidates = [
        FeedCandidate::item(common_mug, 1, 1),
        FeedCandidate::resource(ResourceKind::Stone, 11, 1),
        FeedCandidate::item(crude_mug, 10, 1),
    ];

    let mut state = IntakeState::new();
    let depth_zero_darkness_two =
        BlackHoleAxes::new(2, 0, 2).expect("order 10, crude quality only");
    let first = state.intake(depth_zero_darkness_two, &mut candidates);
    assert_eq!(first.total_quantity, 1);
    assert_eq!(first.credits[0].kind, FeedKind::Item { item: crude_mug });
    assert_eq!(candidates[0].quantity, 1, "quality 1 remains locked");
    assert_eq!(candidates[1].quantity, 1, "order 11 remains locked");

    let depth_one_darkness_five = BlackHoleAxes::new(2, 1, 5).expect("order 20, quality 1");
    let second = state.intake(depth_one_darkness_five, &mut candidates);
    assert_eq!(
        second
            .credits
            .iter()
            .map(|credit| credit.kind)
            .collect::<Vec<_>>(),
        vec![
            FeedKind::Item { item: common_mug },
            FeedKind::Resource {
                resource: ResourceKind::Stone
            }
        ]
    );
}

#[test]
fn child_loads_are_credited_after_local_quantity_with_source_and_child_id() {
    let axes = BlackHoleAxes::new(2, 0, 0).expect("three openings");
    let mut state = IntakeState::new();
    let mut candidates = [FeedCandidate::new(
        FeedKind::Resource {
            resource: ResourceKind::Food,
        },
        1,
        1,
        vec![cat_sim::black_hole::ChildLoad {
            child_id: 42,
            quantity: 2,
        }],
    )
    .expect("candidate with child load")];

    let report = state.intake(axes, &mut candidates);

    assert_eq!(report.total_quantity, 3);
    assert_eq!(
        report
            .credits
            .iter()
            .map(|credit| (credit.source, credit.child_id))
            .collect::<Vec<_>>(),
        vec![
            (FeedSource::Local, None),
            (FeedSource::Child, Some(42)),
            (FeedSource::Child, Some(42))
        ]
    );
    assert_eq!(state.lifetime.quantity, 3);
}

#[test]
fn item_credit_uses_exact_item_value_and_lifetime_kind_totals() {
    let item = Item::new(ItemKind::Weapon, Material::Metal, 3);
    let mut state = IntakeState::new();
    let mut candidates = [FeedCandidate::item(item, 1, 1)];
    let report = state.intake(
        BlackHoleAxes::new(0, 0, 9).expect("superior weapons unlocked"),
        &mut candidates,
    );

    assert_eq!(report.total_quantity, 1);
    assert_eq!(report.credits[0].opening_index, 0);
    assert_eq!(
        report.credits[0].unit_value_micros,
        u64::from(item.value()) * (VALUE_MICROS / 10)
    );
    assert_eq!(
        state
            .lifetime
            .by_kind
            .get(&FeedKind::Item { item })
            .copied(),
        Some(1)
    );
}

#[test]
fn upgrade_recipe_is_deterministic_and_requires_consumed_tools() {
    let axes = BlackHoleAxes::new(2, 5, 7).expect("valid axes");

    let width = upgrade_recipe(axes, BlackHoleAxis::Width).expect("width recipe");
    assert_eq!(width.from_level, 2);
    assert_eq!(width.to_level, 3);
    assert_eq!(width.reward_cost_micros, 0);
    assert_eq!(
        width
            .consumed_resources
            .iter()
            .map(|requirement| (requirement.resource, requirement.quantity))
            .collect::<Vec<_>>(),
        vec![(ResourceKind::Materials, 15), (ResourceKind::Logs, 6)]
    );
    assert_eq!(width.consumed_tools[0].minimum_quality, 0);
    assert_eq!(width.consumed_tools[0].quantity, 1);

    let darkness = upgrade_recipe(axes, BlackHoleAxis::Darkness).expect("darkness recipe");
    assert_eq!(darkness.to_level, 8);
    assert_eq!(
        darkness
            .consumed_resources
            .iter()
            .map(|requirement| (requirement.resource, requirement.quantity))
            .collect::<Vec<_>>(),
        vec![
            (ResourceKind::Materials, 40),
            (ResourceKind::Herbs, 16),
            (ResourceKind::Refined, 10),
            (ResourceKind::Metal, 4)
        ]
    );
    assert_eq!(darkness.consumed_tools[0].minimum_quality, 2);
    assert_eq!(darkness.consumed_tools[0].quantity, 2);
    assert!(darkness.consumed_tools[0].accepts(Item::new(ItemKind::Tool, Material::Gem, 2)));
    assert!(!darkness.consumed_tools[0].accepts(Item::new(ItemKind::Tool, Material::Metal, 1)));
}

#[test]
fn darkness_table_and_void_insight_values_match_the_design() {
    assert_eq!(resource_darkness_requirement(ResourceKind::Water), None);
    assert_eq!(resource_darkness_requirement(ResourceKind::Food), Some(0));
    assert_eq!(resource_darkness_requirement(ResourceKind::Gem), Some(7));
    assert_eq!(item_darkness_requirement(ItemKind::Tool), Some(7));
    assert_eq!(item_darkness_requirement(ItemKind::Armor), Some(9));
    assert_eq!(resource_unit_value_micros(ResourceKind::Food), 100_000);
    assert_eq!(resource_unit_value_micros(ResourceKind::Refined), 300_000);
    assert_eq!(resource_unit_value_micros(ResourceKind::Gem), 500_000);
    assert_eq!(resource_unit_value_micros(ResourceKind::Blessings), 0);
}

#[test]
fn tier_ten_recipe_requires_gems_and_three_masterwork_tools() {
    let recipe = upgrade_recipe(
        BlackHoleAxes::new(9, 9, 9).expect("valid axes"),
        BlackHoleAxis::Width,
    )
    .expect("tier ten recipe");

    assert!(recipe.consumed_resources.iter().any(|requirement| {
        requirement.resource == ResourceKind::Gem && requirement.quantity == 4
    }));
    assert_eq!(recipe.consumed_tools[0].minimum_quality, 4);
    assert_eq!(recipe.consumed_tools[0].quantity, 3);
}

#[test]
fn serde_rejects_invalid_axes_and_candidates() {
    let valid: BlackHoleAxes =
        serde_json::from_str(r#"{"width":10,"depth":0,"darkness":4}"#).expect("valid axes json");
    assert_eq!(valid.width, 10);
    assert!(
        serde_json::from_str::<BlackHoleAxes>(r#"{"width":11,"depth":0,"darkness":0}"#).is_err()
    );

    assert!(
        serde_json::from_str::<FeedCandidate>(
            r#"{"kind":{"kind":"resource","resource":"logs"},"order":111,"quantity":1}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<FeedCandidate>(
            r#"{"kind":{"kind":"resource","resource":"logs"},"order":1,"quantity":0}"#
        )
        .is_err()
    );
}

#[test]
fn intake_state_lifetime_totals_round_trip_through_json() {
    let mut state = IntakeState::new();
    let mut candidates = [FeedCandidate::resource(ResourceKind::Logs, 1, 1)];
    state.intake(
        BlackHoleAxes::new(0, 0, 2).expect("logs unlocked"),
        &mut candidates,
    );

    let json = serde_json::to_string(&state).expect("state serializes");
    let restored: IntakeState = serde_json::from_str(&json).expect("state deserializes");

    assert_eq!(restored, state);
    assert!(json.contains(r#""byKind":[{"kind":{"kind":"resource","resource":"logs"}"#));
}
