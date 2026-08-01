//! Source-derived characterization coverage for the reusable layered-sprite foundation.
//!
//! Adapted from the untracked `the-shrine-upgrade` test leaf. It intentionally
//! exercises only deterministic composition; no renderer or LAI.68 integration
//! system is registered here.

#[allow(dead_code)]
#[path = "../src/layered_sprite.rs"]
mod layered_sprite;

use bevy::prelude::{IVec2, UVec2, Vec2};
use layered_sprite::{
    CanvasSpec, LayerSlot, ReconcilePlan, SpritePart, VariantSpec, VariantState,
    VisibilityPredicate, VisualOwner,
};

fn black_hole_spec() -> VariantSpec {
    VariantSpec::new(
        CanvasSpec::new(UVec2::splat(16), UVec2::splat(80)).unwrap(),
        [
            SpritePart::new(
                LayerSlot::new(0, "base"),
                "public/images/game/buildings/black-hole/base.png",
            ),
            SpritePart::new(
                LayerSlot::new(20, "width-rail-2"),
                "public/images/game/buildings/black-hole/width-02.png",
            )
            .visible_when(VisibilityPredicate::level_at_least("width", 2)),
            SpritePart::new(
                LayerSlot::new(10, "depth-pillar-1"),
                "public/images/game/buildings/black-hole/depth-01.png",
            )
            .visible_when(VisibilityPredicate::level_at_least("depth", 1)),
            SpritePart::new(
                LayerSlot::new(30, "darkness-runes"),
                "public/images/game/buildings/black-hole/darkness-03.png",
            )
            .visible_when(VisibilityPredicate::all([
                VisibilityPredicate::level_at_least("darkness", 3),
                VisibilityPredicate::flag("powered"),
            ])),
        ],
    )
    .unwrap()
}

#[test]
fn resolves_visible_parts_in_stable_layer_order() {
    let state = VariantState::new()
        .with_level("width", 2)
        .with_level("depth", 1)
        .with_level("darkness", 3)
        .with_flag("powered");

    let resolved = black_hole_spec().resolve(&state);
    let names: Vec<_> = resolved
        .parts()
        .iter()
        .map(|part| part.slot.name.as_str())
        .collect();

    assert_eq!(
        names,
        ["base", "depth-pillar-1", "width-rail-2", "darkness-runes"]
    );
}

#[test]
fn reconciliation_skips_exact_signature_and_rebuilds_only_changed_owner() {
    let owner = VisualOwner::new("building", "black-hole@12,8");
    let spec = black_hole_spec();
    let first = spec.reconcile(&owner, None, &VariantState::new().with_level("width", 1));

    let ReconcilePlan::Rebuild {
        owner: planned_owner,
        signature,
        parts,
    } = first
    else {
        panic!("an owner without a signature must be built");
    };
    assert_eq!(planned_owner, owner);
    assert_eq!(parts.len(), 1);

    assert!(matches!(
        spec.reconcile(
            &owner,
            Some(&signature),
            &VariantState::new().with_level("width", 1)
        ),
        ReconcilePlan::Unchanged
    ));

    let ReconcilePlan::Rebuild {
        owner: changed_owner,
        signature: changed_signature,
        parts: changed_parts,
    } = spec.reconcile(
        &owner,
        Some(&signature),
        &VariantState::new().with_level("width", 2),
    )
    else {
        panic!("a newly visible layer must rebuild this owner");
    };
    assert_eq!(changed_owner, owner);
    assert_ne!(changed_signature, signature);
    assert_eq!(changed_parts.len(), 2);
}

#[test]
fn flags_participate_in_visibility_and_signature() {
    let spec = black_hole_spec();
    let unpowered = VariantState::new().with_level("darkness", 3);
    let powered = unpowered.clone().with_flag("powered");

    let a = spec.resolve(&unpowered);
    let b = spec.resolve(&powered);

    assert_eq!(a.parts().len(), 1);
    assert_eq!(b.parts().len(), 2);
    assert_ne!(a.signature(), b.signature());
}

#[test]
fn canvas_converts_pixel_dimensions_and_offsets_to_world_units() {
    let canvas = CanvasSpec::new(UVec2::splat(16), UVec2::new(80, 48)).unwrap();
    let part = SpritePart::new(
        LayerSlot::new(4, "gold-trim"),
        "public/images/game/buildings/black-hole/gold.png",
    )
    .with_draw_pixels(UVec2::new(34, 16))
    .with_offset_pixels(IVec2::new(8, -4));
    let spec = VariantSpec::new(canvas, [part]).unwrap();
    let resolved = spec.resolve(&VariantState::new());
    let geometry = resolved.parts()[0].geometry(Vec2::splat(10.0), 0.01);

    assert_eq!(
        resolved.canvas_world_size(Vec2::splat(10.0)),
        Vec2::new(50.0, 30.0)
    );
    assert_eq!(geometry.custom_size, Vec2::new(21.25, 10.0));
    assert_eq!(geometry.translation, Vec2::new(5.0, -2.5));
    assert_eq!(geometry.z, 0.04);
}

#[test]
fn equal_order_slots_use_name_then_asset_path_as_tie_breakers() {
    let canvas = CanvasSpec::new(UVec2::splat(16), UVec2::splat(16)).unwrap();
    let spec = VariantSpec::new(
        canvas,
        [
            SpritePart::new(LayerSlot::new(5, "b"), "z.png"),
            SpritePart::new(LayerSlot::new(5, "a"), "z.png"),
            SpritePart::new(LayerSlot::new(5, "a"), "a.png"),
        ],
    )
    .unwrap();

    let resolved = spec.resolve(&VariantState::new());
    let keys: Vec<_> = resolved
        .parts()
        .iter()
        .map(|part| (part.slot.name.clone(), part.asset_path.clone()))
        .collect();
    assert_eq!(
        keys,
        [
            ("a".to_owned(), "a.png".to_owned()),
            ("a".to_owned(), "z.png".to_owned()),
            ("b".to_owned(), "z.png".to_owned())
        ]
    );
}

#[test]
fn canvas_rejects_dimensions_that_cannot_map_to_whole_tiles() {
    assert!(CanvasSpec::new(UVec2::splat(16), UVec2::new(79, 80)).is_err());
    assert!(CanvasSpec::new(UVec2::new(0, 16), UVec2::splat(80)).is_err());
}
