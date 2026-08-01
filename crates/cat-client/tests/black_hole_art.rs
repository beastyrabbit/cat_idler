use std::path::{Path, PathBuf};

use image::RgbaImage;

const CANVAS_PIXELS: u32 = 80;
const ROAD_RING_PIXELS: u32 = 16;

fn asset_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("public/images/game/buildings/black-hole")
}

fn load(name: &str) -> RgbaImage {
    image::open(asset_root().join(name))
        .unwrap_or_else(|error| panic!("failed to load {name}: {error}"))
        .into_rgba8()
}

#[test]
fn black_hole_manifest_has_base_and_ten_cumulative_layers_per_axis() {
    assert_eq!(
        load("base.png").dimensions(),
        (CANVAS_PIXELS, CANVAS_PIXELS)
    );
    for axis in ["width", "depth", "darkness"] {
        let mut prior_visible = Vec::new();
        for level in 1..=10 {
            let image = load(&format!("{axis}-{level:02}.png"));
            assert_eq!(image.dimensions(), (CANVAS_PIXELS, CANVAS_PIXELS));
            let visible = visible_pixels(&image);
            assert!(
                prior_visible.iter().all(|pixel| visible.contains(pixel)),
                "{axis} level {level} must contain every earlier pixel"
            );
            assert!(
                visible.len() > prior_visible.len(),
                "{axis} level {level} must add visible construction"
            );
            prior_visible = visible;
        }
    }
}

#[test]
fn every_black_hole_asset_leaves_the_one_tile_road_ring_transparent() {
    let mut names = vec!["base.png".to_owned()];
    for axis in ["width", "depth", "darkness"] {
        names.extend((1..=10).map(|level| format!("{axis}-{level:02}.png")));
    }
    for name in names {
        let image = load(&name);
        for (x, y, pixel) in image.enumerate_pixels() {
            let road_ring = x < ROAD_RING_PIXELS
                || y < ROAD_RING_PIXELS
                || x >= CANVAS_PIXELS - ROAD_RING_PIXELS
                || y >= CANVAS_PIXELS - ROAD_RING_PIXELS;
            if road_ring {
                assert_eq!(pixel.0[3], 0, "{name} covers road-ring pixel ({x}, {y})");
            }
        }
    }
}

fn visible_pixels(image: &RgbaImage) -> Vec<(u32, u32)> {
    image
        .enumerate_pixels()
        .filter_map(|(x, y, pixel)| (pixel.0[3] > 0).then_some((x, y)))
        .collect()
}
