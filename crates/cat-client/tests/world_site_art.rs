use std::{collections::BTreeSet, path::Path};

#[test]
fn quarry_and_hunting_lair_are_distinct_low_resolution_site_assets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("public/images/game/sites");
    let quarry = image::open(root.join("quarry.png"))
        .expect("quarry site sprite")
        .into_rgba8();
    let lair = image::open(root.join("lair.png"))
        .expect("hunting lair site sprite")
        .into_rgba8();
    assert_eq!(quarry.dimensions(), (32, 32));
    assert_eq!(lair.dimensions(), (32, 32));
    assert_ne!(quarry.as_raw(), lair.as_raw());
    assert!(quarry.pixels().any(|pixel| pixel.0[3] > 0));
    assert!(lair.pixels().any(|pixel| pixel.0[3] > 0));
}

#[test]
fn oak_food_overlays_grow_from_low_to_medium_to_full() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("public/images/game/nature");
    let counts = ["low", "mid", "full"].map(|band| {
        let image = image::open(root.join(format!("tree_oak_apples_{band}.png")))
            .expect("oak food overlay")
            .into_rgba8();
        assert_eq!(image.dimensions(), (16, 16));
        image.pixels().filter(|pixel| pixel.0[3] > 0).count()
    });
    assert!(counts[0] > 0);
    assert!(counts[0] < counts[1] && counts[1] < counts[2], "{counts:?}");
}

#[test]
fn each_crop_and_growth_stage_has_distinct_low_resolution_art() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("public/images/game/farm/dynamic");
    let mut images = BTreeSet::new();
    for crop in ["catnip", "grain", "herb"] {
        for stage in ["sprout", "growing", "mature", "flowering"] {
            let image = image::open(root.join(format!("{crop}-{stage}.png")))
                .expect("crop-specific stage sprite")
                .into_rgba8();
            assert_eq!(image.dimensions(), (16, 16));
            assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
            assert!(
                images.insert(image.into_raw()),
                "{crop}-{stage} aliases another crop-stage sprite"
            );
        }
    }
    assert_eq!(images.len(), 12);
}

#[test]
fn transport_placeholders_are_replaced_by_distinct_pixel_art() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("public/images/game/transport");
    let mut images = BTreeSet::new();
    for name in ["rail_cart", "boat", "dock_land", "dock_water"] {
        let image = image::open(root.join(format!("{name}.png")))
            .expect("transport sprite")
            .into_rgba8();
        assert_eq!(image.dimensions(), (16, 16));
        assert!(image.pixels().any(|pixel| pixel.0[3] > 0));
        assert!(
            images.insert(image.into_raw()),
            "{name} aliases another sprite"
        );
    }
}
