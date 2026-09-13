//! Cosmetic cat genetics ported from `lib/game/genetics.ts`.
//!
//! The TypeScript source uses raw `Math.random()` throughout this module. These
//! visual genetics affect sprite appearance, so the Rust port preserves the TS
//! threshold/order semantics but takes an injected roll source instead of using
//! process randomness. This gives deterministic, testable behavioural parity
//! without claiming bit-exact parity with the original unseeded calls.

use serde::{Deserialize, Serialize};

use crate::rng::roll_seeded;

const PELTS: &[&str] = &[
    "Tabby", "Ticked", "Mackerel", "Classic", "Sokoke", "Speckled", "Rosette", "Smoke",
];
const COLOURS: &[&str] = &[
    "BLACK", "WHITE", "GINGER", "GRAY", "BROWN", "CREAM", "ORANGE", "DARKGRAY",
];
const TORTIE_COLOURS: &[&str] = &[
    "BLACK", "WHITE", "GINGER", "GRAY", "BROWN", "CREAM", "ORANGE",
];
const EYE_COLOURS: &[&str] = &[
    "YELLOW", "AMBER", "HAZEL", "GREEN", "BLUE", "DARKBLUE", "GRAY",
];
const SKIN_COLOURS: &[&str] = &["BLACK", "PINK", "DARKBROWN", "BROWN", "LIGHTBROWN"];
const TINTS: &[&str] = &["none", "REDYELLOW", "BLUE", "PURPLE", "GREEN"];
const WHITE_PATCHES: &[&str] = &[
    "LITTLE",
    "LIGHTTUXEDO",
    "TUXEDO",
    "FANCY",
    "EXTRA",
    "POINTMARK",
];
const POINTS: &[&str] = &["POINTED", "MINK", "SEPIA"];
const VALID_SPRITES: &[u8] = &[3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18];
const TORTIE_PELTS: &[&str] = &["Tabby", "Ticked", "Mackerel", "Classic", "Sokoke"];
const MUTATION_CHANCE: f64 = 0.1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneticTraits {
    pub pelt_name: String,
    pub colour: String,
    pub eye_colour: String,
    pub skin_colour: String,
    pub is_tortie: bool,
    pub tint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub white_patches: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatSpriteParams {
    pub sprite_number: u8,
    pub pelt_name: String,
    pub colour: String,
    pub tint: String,
    pub skin_colour: String,
    pub eye_colour: String,
    pub shading: bool,
    pub reverse: bool,
    pub is_tortie: bool,
    #[serde(default)]
    pub accessories: Vec<String>,
    #[serde(default)]
    pub scars: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eye_colour2: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub white_patches: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub points: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tortie: Option<Vec<TortiePatch>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tortie_mask: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tortie_pattern: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tortie_colour: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TortiePatch {
    pub mask: String,
    pub pattern: String,
    pub colour: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteBaseParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sprite_number: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shading: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accessories: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scars: Option<Vec<String>>,
}

pub trait RollSource {
    fn roll(&mut self) -> f64;
}

#[derive(Debug, Clone)]
pub struct SliceRollSource<'a> {
    rolls: &'a [f64],
    index: usize,
}

impl<'a> SliceRollSource<'a> {
    #[must_use]
    pub fn new(rolls: &'a [f64]) -> Self {
        Self { rolls, index: 0 }
    }

    #[must_use]
    pub fn consumed(&self) -> usize {
        self.index
    }

    #[must_use]
    pub fn is_exhausted(&self) -> bool {
        self.index == self.rolls.len()
    }
}

impl RollSource for SliceRollSource<'_> {
    fn roll(&mut self) -> f64 {
        let roll = *self
            .rolls
            .get(self.index)
            .expect("genetics roll source exhausted");
        self.index += 1;
        roll
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SeededRollSource {
    seed: u32,
}

impl SeededRollSource {
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }

    #[must_use]
    pub fn seed(&self) -> u32 {
        self.seed
    }
}

impl RollSource for SeededRollSource {
    fn roll(&mut self) -> f64 {
        let roll = roll_seeded(f64::from(self.seed));
        self.seed = roll.next_seed;
        roll.value
    }
}

#[must_use]
pub fn extract_genetic_traits(params: Option<&CatSpriteParams>) -> Option<GeneticTraits> {
    params.map(|params| GeneticTraits {
        pelt_name: params.pelt_name.clone(),
        colour: params.colour.clone(),
        eye_colour: params.eye_colour.clone(),
        skin_colour: params.skin_colour.clone(),
        is_tortie: params.is_tortie,
        tint: params.tint.clone(),
        white_patches: params.white_patches.clone(),
        points: params.points.clone(),
    })
}

pub fn inherit_traits<R: RollSource>(
    parent1_traits: Option<&GeneticTraits>,
    parent2_traits: Option<&GeneticTraits>,
    rolls: &mut R,
) -> GeneticTraits {
    match (parent1_traits, parent2_traits) {
        (None, None) => generate_random_traits(rolls),
        (None, Some(parent)) | (Some(parent), None) => inherit_with_mutation(parent, rolls),
        (Some(parent1), Some(parent2)) => GeneticTraits {
            pelt_name: pick_parent_string(&parent1.pelt_name, &parent2.pelt_name, rolls),
            colour: pick_parent_string(&parent1.colour, &parent2.colour, rolls),
            eye_colour: pick_parent_string(&parent1.eye_colour, &parent2.eye_colour, rolls),
            skin_colour: pick_parent_string(&parent1.skin_colour, &parent2.skin_colour, rolls),
            is_tortie: determine_tortie_inheritance(parent1.is_tortie, parent2.is_tortie, rolls),
            tint: pick_parent_string(&parent1.tint, &parent2.tint, rolls),
            white_patches: inherit_white_patches(
                parent1.white_patches.as_deref(),
                parent2.white_patches.as_deref(),
                rolls,
            ),
            points: inherit_points(parent1.points.as_deref(), parent2.points.as_deref(), rolls),
        },
    }
}

pub fn generate_random_traits<R: RollSource>(rolls: &mut R) -> GeneticTraits {
    GeneticTraits {
        pelt_name: choose_string(PELTS, rolls),
        colour: choose_string(COLOURS, rolls),
        eye_colour: choose_string(EYE_COLOURS, rolls),
        skin_colour: choose_string(SKIN_COLOURS, rolls),
        is_tortie: rolls.roll() < 0.3,
        tint: choose_string(TINTS, rolls),
        white_patches: if rolls.roll() < 0.3 {
            Some("LITTLE".to_owned())
        } else {
            None
        },
        points: if rolls.roll() < 0.1 {
            Some("POINTED".to_owned())
        } else {
            None
        },
    }
}

pub fn inherit_with_mutation<R: RollSource>(
    parent_traits: &GeneticTraits,
    rolls: &mut R,
) -> GeneticTraits {
    GeneticTraits {
        pelt_name: mutate_string(&parent_traits.pelt_name, PELTS, rolls),
        colour: mutate_string(&parent_traits.colour, COLOURS, rolls),
        eye_colour: mutate_string(&parent_traits.eye_colour, EYE_COLOURS, rolls),
        skin_colour: mutate_string(&parent_traits.skin_colour, SKIN_COLOURS, rolls),
        is_tortie: if rolls.roll() < MUTATION_CHANCE {
            !parent_traits.is_tortie
        } else {
            parent_traits.is_tortie
        },
        tint: mutate_string(&parent_traits.tint, TINTS, rolls),
        white_patches: inherit_white_patches(parent_traits.white_patches.as_deref(), None, rolls),
        points: inherit_points(parent_traits.points.as_deref(), None, rolls),
    }
}

pub fn determine_tortie_inheritance<R: RollSource>(
    parent1_is_tortie: bool,
    parent2_is_tortie: bool,
    rolls: &mut R,
) -> bool {
    if parent1_is_tortie && parent2_is_tortie {
        return rolls.roll() < 0.7;
    }
    if parent1_is_tortie || parent2_is_tortie {
        return rolls.roll() < 0.4;
    }
    rolls.roll() < 0.1
}

pub fn inherit_white_patches<R: RollSource>(
    parent1_patches: Option<&str>,
    parent2_patches: Option<&str>,
    rolls: &mut R,
) -> Option<String> {
    let roll = rolls.roll();

    if roll < 0.4 {
        return pick_parent_option(parent1_patches, parent2_patches, rolls);
    }

    if roll < 0.7 {
        return Some(choose_string(WHITE_PATCHES, rolls));
    }

    None
}

pub fn inherit_points<R: RollSource>(
    parent1_points: Option<&str>,
    parent2_points: Option<&str>,
    rolls: &mut R,
) -> Option<String> {
    let roll = rolls.roll();

    if roll < 0.4 {
        return pick_parent_option(parent1_points, parent2_points, rolls);
    }

    if roll < 0.5 {
        return Some(choose_string(POINTS, rolls));
    }

    None
}

pub fn traits_to_sprite_params<R: RollSource>(
    traits: &GeneticTraits,
    base_params: Option<&SpriteBaseParams>,
    rolls: &mut R,
) -> CatSpriteParams {
    let sprite_number = base_params
        .and_then(|params| params.sprite_number)
        .unwrap_or_else(|| choose_sprite(rolls));
    let shading = base_params
        .and_then(|params| params.shading)
        .unwrap_or_else(|| rolls.roll() < 0.5);
    let reverse = base_params
        .and_then(|params| params.reverse)
        .unwrap_or_else(|| rolls.roll() < 0.1);
    let accessories = base_params
        .and_then(|params| params.accessories.clone())
        .unwrap_or_default();
    let scars = base_params
        .and_then(|params| params.scars.clone())
        .unwrap_or_default();

    let mut params = CatSpriteParams {
        sprite_number,
        pelt_name: traits.pelt_name.clone(),
        colour: traits.colour.clone(),
        tint: traits.tint.clone(),
        skin_colour: traits.skin_colour.clone(),
        eye_colour: traits.eye_colour.clone(),
        shading,
        reverse,
        is_tortie: traits.is_tortie,
        accessories,
        scars,
        eye_colour2: None,
        white_patches: traits.white_patches.clone(),
        points: traits.points.clone(),
        tortie: None,
        tortie_mask: None,
        tortie_pattern: None,
        tortie_colour: None,
    };

    if rolls.roll() < 0.05 {
        params.eye_colour2 = Some(choose_string(EYE_COLOURS, rolls));
    }

    if traits.is_tortie {
        let patch = TortiePatch {
            mask: "ONE".to_owned(),
            pattern: choose_string(TORTIE_PELTS, rolls),
            colour: choose_string(TORTIE_COLOURS, rolls),
        };
        params.tortie_mask = Some("ONE".to_owned());
        params.tortie_pattern = Some(patch.pattern.clone());
        params.tortie_colour = Some(patch.colour.clone());
        params.tortie = Some(vec![patch]);
    }

    params
}

fn pick_parent_string<R: RollSource>(parent1: &str, parent2: &str, rolls: &mut R) -> String {
    if rolls.roll() < 0.5 {
        parent1.to_owned()
    } else {
        parent2.to_owned()
    }
}

fn pick_parent_option<R: RollSource>(
    parent1: Option<&str>,
    parent2: Option<&str>,
    rolls: &mut R,
) -> Option<String> {
    if rolls.roll() < 0.5 {
        parent1.map(str::to_owned)
    } else {
        parent2.map(str::to_owned)
    }
}

fn mutate_string<R: RollSource>(parent_value: &str, variants: &[&str], rolls: &mut R) -> String {
    if rolls.roll() < MUTATION_CHANCE {
        choose_string(variants, rolls)
    } else {
        parent_value.to_owned()
    }
}

fn choose_string<R: RollSource>(variants: &[&str], rolls: &mut R) -> String {
    variants[random_index(rolls.roll(), variants.len())].to_owned()
}

fn choose_sprite<R: RollSource>(rolls: &mut R) -> u8 {
    VALID_SPRITES[random_index(rolls.roll(), VALID_SPRITES.len())]
}

fn random_index(roll: f64, len: usize) -> usize {
    assert!(roll >= 0.0, "genetics roll must be >= 0.0, got {roll}");
    assert!(roll < 1.0, "genetics roll must be < 1.0, got {roll}");
    (roll * len as f64).floor() as usize
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::{
        CatSpriteParams, GeneticTraits, SliceRollSource, SpriteBaseParams, extract_genetic_traits,
        generate_random_traits, inherit_traits, traits_to_sprite_params,
    };

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        source: String,
        traits_to_sprite_params: Vec<SpriteMappingCase>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct SpriteMappingCase {
        traits: GeneticTraits,
        #[serde(default)]
        base_params: Option<SpriteBaseParams>,
        rolls: Vec<f64>,
        params: CatSpriteParams,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p4/genetics.json"
        ))
        .expect("genetics fixture parses")
    }

    fn parent1() -> GeneticTraits {
        GeneticTraits {
            pelt_name: "Tabby".to_owned(),
            colour: "BLACK".to_owned(),
            eye_colour: "YELLOW".to_owned(),
            skin_colour: "PINK".to_owned(),
            is_tortie: true,
            tint: "none".to_owned(),
            white_patches: Some("LITTLE".to_owned()),
            points: Some("POINTED".to_owned()),
        }
    }

    fn parent2() -> GeneticTraits {
        GeneticTraits {
            pelt_name: "Smoke".to_owned(),
            colour: "GINGER".to_owned(),
            eye_colour: "BLUE".to_owned(),
            skin_colour: "BLACK".to_owned(),
            is_tortie: false,
            tint: "GREEN".to_owned(),
            white_patches: Some("TUXEDO".to_owned()),
            points: None,
        }
    }

    #[test]
    fn fixture_is_generated_from_genetics_ts() {
        assert_eq!(fixture().source, "lib/game/genetics.ts");
    }

    #[test]
    fn traits_to_sprite_params_matches_ts_fixture_with_injected_rolls() {
        for case in fixture().traits_to_sprite_params {
            let mut rolls = SliceRollSource::new(&case.rolls);

            assert_eq!(
                traits_to_sprite_params(&case.traits, case.base_params.as_ref(), &mut rolls),
                case.params
            );
            assert!(
                rolls.is_exhausted(),
                "fixture rolls should be fully consumed"
            );
        }
    }

    #[test]
    fn inherit_traits_with_two_parents_consumes_rolls_in_ts_order() {
        let first = parent1();
        let second = parent2();
        let rolls = [0.49, 0.5, 0.2, 0.9, 0.39, 0.6, 0.2, 0.6, 0.45, 0.8];
        let mut source = SliceRollSource::new(&rolls);

        let traits = inherit_traits(Some(&first), Some(&second), &mut source);

        assert_eq!(
            traits,
            GeneticTraits {
                pelt_name: "Tabby".to_owned(),
                colour: "GINGER".to_owned(),
                eye_colour: "YELLOW".to_owned(),
                skin_colour: "BLACK".to_owned(),
                is_tortie: true,
                tint: "GREEN".to_owned(),
                white_patches: Some("TUXEDO".to_owned()),
                points: Some("SEPIA".to_owned()),
            }
        );
        assert!(source.is_exhausted());
    }

    #[test]
    fn inherit_traits_from_one_parent_matches_mutation_thresholds() {
        let parent = parent1();
        let rolls = [
            0.09, 0.99, 0.1, 0.5, 0.0, 0.81, 0.09, 0.09, 0.42, 0.65, 0.99, 0.3, 0.4,
        ];
        let mut source = SliceRollSource::new(&rolls);

        let traits = inherit_traits(Some(&parent), None, &mut source);

        assert_eq!(
            traits,
            GeneticTraits {
                pelt_name: "Smoke".to_owned(),
                colour: "BLACK".to_owned(),
                eye_colour: "YELLOW".to_owned(),
                skin_colour: "LIGHTBROWN".to_owned(),
                is_tortie: false,
                tint: "BLUE".to_owned(),
                white_patches: Some("POINTMARK".to_owned()),
                points: Some("POINTED".to_owned()),
            }
        );
        assert!(source.is_exhausted());
    }

    #[test]
    fn generate_random_traits_matches_founder_roll_order() {
        let rolls = [0.0, 0.999, 0.5, 0.2, 0.29, 0.8, 0.29, 0.09];
        let mut source = SliceRollSource::new(&rolls);

        let traits = generate_random_traits(&mut source);

        assert_eq!(
            traits,
            GeneticTraits {
                pelt_name: "Tabby".to_owned(),
                colour: "DARKGRAY".to_owned(),
                eye_colour: "GREEN".to_owned(),
                skin_colour: "PINK".to_owned(),
                is_tortie: true,
                tint: "GREEN".to_owned(),
                white_patches: Some("LITTLE".to_owned()),
                points: Some("POINTED".to_owned()),
            }
        );
        assert!(source.is_exhausted());
    }

    #[test]
    fn extract_genetic_traits_copies_sprite_trait_fields() {
        let params = CatSpriteParams {
            sprite_number: 3,
            pelt_name: "Classic".to_owned(),
            colour: "CREAM".to_owned(),
            tint: "PURPLE".to_owned(),
            skin_colour: "BROWN".to_owned(),
            eye_colour: "AMBER".to_owned(),
            shading: false,
            reverse: false,
            is_tortie: true,
            accessories: vec![],
            scars: vec![],
            eye_colour2: Some("BLUE".to_owned()),
            white_patches: Some("FANCY".to_owned()),
            points: Some("MINK".to_owned()),
            tortie: None,
            tortie_mask: None,
            tortie_pattern: None,
            tortie_colour: None,
        };

        assert_eq!(
            extract_genetic_traits(Some(&params)),
            Some(GeneticTraits {
                pelt_name: "Classic".to_owned(),
                colour: "CREAM".to_owned(),
                eye_colour: "AMBER".to_owned(),
                skin_colour: "BROWN".to_owned(),
                is_tortie: true,
                tint: "PURPLE".to_owned(),
                white_patches: Some("FANCY".to_owned()),
                points: Some("MINK".to_owned()),
            })
        );
        assert_eq!(extract_genetic_traits(None), None);
    }
}
