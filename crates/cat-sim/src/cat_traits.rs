//! Innate cat attributes and personality specified by
//! `docs/leader-ai-overhaul/cats-and-care.md`.

use serde::{Deserialize, Deserializer, Serialize};

use crate::planner_core::{BASIS_POINTS_SCALE, BasisPoints, PlannerRngStream, planner_roll};

pub const ATTRIBUTE_MIN: u8 = 1;
pub const ATTRIBUTE_MAX: u8 = 20;
pub const ATTRIBUTE_BASELINE: u8 = 10;

/// One innate capability on the canonical 1–20 scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct AttributeValue(u8);

impl AttributeValue {
    #[must_use]
    pub const fn new_clamped(value: i32) -> Self {
        if value < ATTRIBUTE_MIN as i32 {
            Self(ATTRIBUTE_MIN)
        } else if value > ATTRIBUTE_MAX as i32 {
            Self(ATTRIBUTE_MAX)
        } else {
            Self(value as u8)
        }
    }

    /// Convert a legacy 0–100 value with
    /// `clamp(round(1 + old * 19 / 100), 1, 20)`.
    #[must_use]
    pub const fn from_legacy_0_to_100(legacy: i32) -> Self {
        let legacy = if legacy < 0 {
            0
        } else if legacy > 100 {
            100
        } else {
            legacy
        };
        let rounded = (150 + legacy * 19) / 100;
        Self::new_clamped(rounded)
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl Default for AttributeValue {
    fn default() -> Self {
        Self(ATTRIBUTE_BASELINE)
    }
}

impl<'de> Deserialize<'de> for AttributeValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        if (ATTRIBUTE_MIN..=ATTRIBUTE_MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(serde::de::Error::custom(format_args!(
                "attribute must be in {ATTRIBUTE_MIN}..={ATTRIBUTE_MAX}, got {value}"
            )))
        }
    }
}

/// Stable registry order for innate attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AttributeAxis {
    Attack,
    Defense,
    Hunting,
    Medicine,
    Cleaning,
    Building,
    Leadership,
    Vision,
}

impl AttributeAxis {
    pub const ALL: [Self; 8] = [
        Self::Attack,
        Self::Defense,
        Self::Hunting,
        Self::Medicine,
        Self::Cleaning,
        Self::Building,
        Self::Leadership,
        Self::Vision,
    ];

    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Attack => "attack",
            Self::Defense => "defense",
            Self::Hunting => "hunting",
            Self::Medicine => "medicine",
            Self::Cleaning => "cleaning",
            Self::Building => "building",
            Self::Leadership => "leadership",
            Self::Vision => "vision",
        }
    }
}

/// Unconverted persisted values from the former 0–100 cat-stat scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyCatAttributes {
    pub attack: i32,
    pub defense: i32,
    pub hunting: i32,
    pub medicine: i32,
    pub cleaning: i32,
    pub building: i32,
    pub leadership: i32,
    pub vision: i32,
}

/// The eight innate, non-learned capabilities of a cat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatAttributes {
    #[serde(default)]
    pub attack: AttributeValue,
    #[serde(default)]
    pub defense: AttributeValue,
    #[serde(default)]
    pub hunting: AttributeValue,
    #[serde(default)]
    pub medicine: AttributeValue,
    #[serde(default)]
    pub cleaning: AttributeValue,
    #[serde(default)]
    pub building: AttributeValue,
    #[serde(default)]
    pub leadership: AttributeValue,
    #[serde(default)]
    pub vision: AttributeValue,
}

impl CatAttributes {
    #[must_use]
    pub const fn all(value: i32) -> Self {
        let value = AttributeValue::new_clamped(value);
        Self {
            attack: value,
            defense: value,
            hunting: value,
            medicine: value,
            cleaning: value,
            building: value,
            leadership: value,
            vision: value,
        }
    }

    #[must_use]
    pub const fn from_legacy_0_to_100(legacy: LegacyCatAttributes) -> Self {
        Self {
            attack: AttributeValue::from_legacy_0_to_100(legacy.attack),
            defense: AttributeValue::from_legacy_0_to_100(legacy.defense),
            hunting: AttributeValue::from_legacy_0_to_100(legacy.hunting),
            medicine: AttributeValue::from_legacy_0_to_100(legacy.medicine),
            cleaning: AttributeValue::from_legacy_0_to_100(legacy.cleaning),
            building: AttributeValue::from_legacy_0_to_100(legacy.building),
            leadership: AttributeValue::from_legacy_0_to_100(legacy.leadership),
            vision: AttributeValue::from_legacy_0_to_100(legacy.vision),
        }
    }

    #[must_use]
    pub const fn value(self, axis: AttributeAxis) -> AttributeValue {
        match axis {
            AttributeAxis::Attack => self.attack,
            AttributeAxis::Defense => self.defense,
            AttributeAxis::Hunting => self.hunting,
            AttributeAxis::Medicine => self.medicine,
            AttributeAxis::Cleaning => self.cleaning,
            AttributeAxis::Building => self.building,
            AttributeAxis::Leadership => self.leadership,
            AttributeAxis::Vision => self.vision,
        }
    }

    #[must_use]
    pub const fn values(self) -> [u8; 8] {
        [
            self.attack.get(),
            self.defense.get(),
            self.hunting.get(),
            self.medicine.get(),
            self.cleaning.get(),
            self.building.get(),
            self.leadership.get(),
            self.vision.get(),
        ]
    }
}

impl Default for CatAttributes {
    fn default() -> Self {
        Self::all(ATTRIBUTE_BASELINE.into())
    }
}

/// Inherit one attribute. `mutation_bucket` maps 0..=4 to −2..=2.
#[must_use]
pub fn inherit_attribute(
    first: Option<AttributeValue>,
    second: Option<AttributeValue>,
    mutation_bucket: u8,
) -> AttributeValue {
    let first = first.unwrap_or_default().get();
    let second = second.unwrap_or_default().get();
    let rounded_midpoint = (u16::from(first) + u16::from(second)).div_ceil(2);
    let mutation = i32::from(mutation_bucket.min(4)) - 2;
    AttributeValue::new_clamped(i32::from(rounded_midpoint) + mutation)
}

/// Inherit all attributes with one keyed draw per axis. No shared cursor means
/// sibling creation and future axis iteration order cannot perturb a result.
#[must_use]
pub fn inherit_attributes_seeded(
    world_seed: u32,
    colony_id: &str,
    newborn_id: &str,
    first: Option<&CatAttributes>,
    second: Option<&CatAttributes>,
) -> CatAttributes {
    let value = |axis: AttributeAxis| {
        let roll = planner_roll(
            world_seed,
            PlannerRngStream::Personality,
            [
                colony_id,
                newborn_id,
                "newborn_attribute_mutation",
                axis.stable_id(),
            ],
        );
        inherit_attribute(
            first.map(|attributes| attributes.value(axis)),
            second.map(|attributes| attributes.value(axis)),
            (roll.next_seed % 5) as u8,
        )
    };

    CatAttributes {
        attack: value(AttributeAxis::Attack),
        defense: value(AttributeAxis::Defense),
        hunting: value(AttributeAxis::Hunting),
        medicine: value(AttributeAxis::Medicine),
        cleaning: value(AttributeAxis::Cleaning),
        building: value(AttributeAxis::Building),
        leadership: value(AttributeAxis::Leadership),
        vision: value(AttributeAxis::Vision),
    }
}

/// Direction on a named personality axis. Negative is the first documented
/// pole (for example Cautious); positive is the second (Bold).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PersonalityPole {
    Negative,
    Positive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PersonalityStrength {
    Subtle,
    Pronounced,
    Extreme,
}

impl PersonalityStrength {
    #[must_use]
    pub const fn from_percentile(percentile: u8) -> Self {
        let percentile = if percentile > 99 { 99 } else { percentile };
        match percentile {
            0..=79 => Self::Subtle,
            80..=94 => Self::Pronounced,
            _ => Self::Extreme,
        }
    }

    #[must_use]
    pub const fn modifier_basis_points(self) -> i64 {
        match self {
            Self::Subtle => 500,
            Self::Pronounced => 1_500,
            Self::Extreme => 3_000,
        }
    }

    #[must_use]
    const fn signed_level(self) -> i8 {
        match self {
            Self::Subtle => 1,
            Self::Pronounced => 2,
            Self::Extreme => 3,
        }
    }

    #[must_use]
    #[cfg(test)]
    const fn index(self) -> usize {
        match self {
            Self::Subtle => 0,
            Self::Pronounced => 1,
            Self::Extreme => 2,
        }
    }
}

/// A signed personality level: −3..=−1 points toward the first pole, 1..=3
/// toward the second, and 0 is the compatibility default for existing cats.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct PersonalityValue(i8);

impl PersonalityValue {
    pub const NEUTRAL: Self = Self(0);

    #[must_use]
    pub const fn new(pole: PersonalityPole, strength: PersonalityStrength) -> Self {
        let level = strength.signed_level();
        match pole {
            PersonalityPole::Negative => Self(-level),
            PersonalityPole::Positive => Self(level),
        }
    }

    #[must_use]
    pub const fn signed_level(self) -> i8 {
        self.0
    }

    #[must_use]
    pub const fn pole(self) -> Option<PersonalityPole> {
        if self.0 < 0 {
            Some(PersonalityPole::Negative)
        } else if self.0 > 0 {
            Some(PersonalityPole::Positive)
        } else {
            None
        }
    }

    #[must_use]
    pub const fn strength(self) -> Option<PersonalityStrength> {
        match self.0.unsigned_abs() {
            1 => Some(PersonalityStrength::Subtle),
            2 => Some(PersonalityStrength::Pronounced),
            3 => Some(PersonalityStrength::Extreme),
            _ => None,
        }
    }

    #[must_use]
    pub const fn signed_modifier(self) -> BasisPoints {
        match self.strength() {
            Some(strength) => {
                let magnitude = strength.modifier_basis_points();
                if self.0 < 0 {
                    BasisPoints::new(-magnitude)
                } else {
                    BasisPoints::new(magnitude)
                }
            }
            None => BasisPoints::new(0),
        }
    }
}

impl TryFrom<i8> for PersonalityValue {
    type Error = &'static str;

    fn try_from(value: i8) -> Result<Self, Self::Error> {
        if (-3..=3).contains(&value) {
            Ok(Self(value))
        } else {
            Err("personality level must be in -3..=3")
        }
    }
}

impl<'de> Deserialize<'de> for PersonalityValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from(i8::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

/// Stable registry order for the eight documented signed axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PersonalityAxis {
    CautiousBold,
    LeisurelyDiligent,
    TraditionalCurious,
    ContentAmbitious,
    SelfReliantCommunal,
    SolitaryGregarious,
    SelfSufficientMercantile,
    SkepticalDevout,
}

impl PersonalityAxis {
    pub const ALL: [Self; 8] = [
        Self::CautiousBold,
        Self::LeisurelyDiligent,
        Self::TraditionalCurious,
        Self::ContentAmbitious,
        Self::SelfReliantCommunal,
        Self::SolitaryGregarious,
        Self::SelfSufficientMercantile,
        Self::SkepticalDevout,
    ];

    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::CautiousBold => "cautious_bold",
            Self::LeisurelyDiligent => "leisurely_diligent",
            Self::TraditionalCurious => "traditional_curious",
            Self::ContentAmbitious => "content_ambitious",
            Self::SelfReliantCommunal => "self_reliant_communal",
            Self::SolitaryGregarious => "solitary_gregarious",
            Self::SelfSufficientMercantile => "self_sufficient_mercantile",
            Self::SkepticalDevout => "skeptical_devout",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatPersonality {
    #[serde(default)]
    pub cautious_bold: PersonalityValue,
    #[serde(default)]
    pub leisurely_diligent: PersonalityValue,
    #[serde(default)]
    pub traditional_curious: PersonalityValue,
    #[serde(default)]
    pub content_ambitious: PersonalityValue,
    #[serde(default)]
    pub self_reliant_communal: PersonalityValue,
    #[serde(default)]
    pub solitary_gregarious: PersonalityValue,
    #[serde(default)]
    pub self_sufficient_mercantile: PersonalityValue,
    #[serde(default)]
    pub skeptical_devout: PersonalityValue,
}

impl CatPersonality {
    #[must_use]
    pub const fn value(self, axis: PersonalityAxis) -> PersonalityValue {
        match axis {
            PersonalityAxis::CautiousBold => self.cautious_bold,
            PersonalityAxis::LeisurelyDiligent => self.leisurely_diligent,
            PersonalityAxis::TraditionalCurious => self.traditional_curious,
            PersonalityAxis::ContentAmbitious => self.content_ambitious,
            PersonalityAxis::SelfReliantCommunal => self.self_reliant_communal,
            PersonalityAxis::SolitaryGregarious => self.solitary_gregarious,
            PersonalityAxis::SelfSufficientMercantile => self.self_sufficient_mercantile,
            PersonalityAxis::SkepticalDevout => self.skeptical_devout,
        }
    }

    pub fn set(&mut self, axis: PersonalityAxis, value: PersonalityValue) {
        match axis {
            PersonalityAxis::CautiousBold => self.cautious_bold = value,
            PersonalityAxis::LeisurelyDiligent => self.leisurely_diligent = value,
            PersonalityAxis::TraditionalCurious => self.traditional_curious = value,
            PersonalityAxis::ContentAmbitious => self.content_ambitious = value,
            PersonalityAxis::SelfReliantCommunal => self.self_reliant_communal = value,
            PersonalityAxis::SolitaryGregarious => self.solitary_gregarious = value,
            PersonalityAxis::SelfSufficientMercantile => self.self_sufficient_mercantile = value,
            PersonalityAxis::SkepticalDevout => self.skeptical_devout = value,
        }
    }

    #[must_use]
    pub const fn signed_levels(self) -> [i8; 8] {
        [
            self.cautious_bold.signed_level(),
            self.leisurely_diligent.signed_level(),
            self.traditional_curious.signed_level(),
            self.content_ambitious.signed_level(),
            self.self_reliant_communal.signed_level(),
            self.solitary_gregarious.signed_level(),
            self.self_sufficient_mercantile.signed_level(),
            self.skeptical_devout.signed_level(),
        ]
    }

    /// Return a fixed-point multiplicative weight where 10,000 is unchanged.
    #[must_use]
    pub const fn weight_factor(
        self,
        axis: PersonalityAxis,
        favored_pole: PersonalityPole,
    ) -> BasisPoints {
        let signed = self.value(axis).signed_modifier().get();
        let aligned = match favored_pole {
            PersonalityPole::Negative => -signed,
            PersonalityPole::Positive => signed,
        };
        BasisPoints::new(BASIS_POINTS_SCALE + aligned)
    }

    /// Apply one axis factor with saturating integer fixed-point arithmetic.
    #[must_use]
    pub fn apply_weight(
        self,
        base_weight: i64,
        axis: PersonalityAxis,
        favored_pole: PersonalityPole,
    ) -> i64 {
        let factor = i128::from(self.weight_factor(axis, favored_pole).get());
        let adjusted =
            i128::from(base_weight).saturating_mul(factor) / i128::from(BASIS_POINTS_SCALE);
        adjusted.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64
    }
}

/// Generate all axes from independent semantic keys. Strength and polarity
/// also use separate keys, so adding a draw cannot shift either sibling cats
/// or unrelated axes.
#[must_use]
pub fn generate_personality(world_seed: u32, colony_id: &str, cat_id: &str) -> CatPersonality {
    let mut personality = CatPersonality::default();
    for axis in PersonalityAxis::ALL {
        let strength_roll = planner_roll(
            world_seed,
            PlannerRngStream::Personality,
            [colony_id, cat_id, "personality_strength", axis.stable_id()],
        );
        let pole_roll = planner_roll(
            world_seed,
            PlannerRngStream::Personality,
            [colony_id, cat_id, "personality_pole", axis.stable_id()],
        );
        let strength = PersonalityStrength::from_percentile((strength_roll.next_seed % 100) as u8);
        let pole = if pole_roll.next_seed & 1 == 0 {
            PersonalityPole::Negative
        } else {
            PersonalityPole::Positive
        };
        personality.set(axis, PersonalityValue::new(pole, strength));
    }
    personality
}

/// Additive cat-model leaf. Existing cats decode to centered attributes and
/// neutral personality, preserving behavior until an explicit backfill runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatTraits {
    #[serde(default)]
    pub attributes: CatAttributes,
    #[serde(default)]
    pub personality: CatPersonality,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_values_clamp_and_compatibility_defaults_center_on_ten() {
        assert_eq!(AttributeValue::new_clamped(-50).get(), 1);
        assert_eq!(AttributeValue::new_clamped(1).get(), 1);
        assert_eq!(AttributeValue::new_clamped(20).get(), 20);
        assert_eq!(AttributeValue::new_clamped(50).get(), 20);
        assert_eq!(AttributeValue::default().get(), 10);
        assert_eq!(CatAttributes::default(), CatAttributes::all(10));
    }

    #[test]
    fn legacy_zero_to_one_hundred_conversion_uses_the_exact_formula() {
        let cases = [
            (-100, 1),
            (0, 1),
            (2, 1),
            (3, 2),
            (47, 10),
            (50, 11),
            (97, 19),
            (98, 20),
            (100, 20),
            (200, 20),
        ];

        for (legacy, expected) in cases {
            assert_eq!(
                AttributeValue::from_legacy_0_to_100(legacy).get(),
                expected,
                "legacy value {legacy}"
            );
        }
    }

    #[test]
    fn attribute_set_conversion_preserves_the_canonical_field_order() {
        let converted = CatAttributes::from_legacy_0_to_100(LegacyCatAttributes {
            attack: 0,
            defense: 3,
            hunting: 10,
            medicine: 25,
            cleaning: 50,
            building: 75,
            leadership: 97,
            vision: 100,
        });

        assert_eq!(converted.values(), [1, 2, 3, 6, 11, 15, 19, 20]);
    }

    #[test]
    fn newborn_midpoint_mutation_rounds_up_and_clamps_at_both_bounds() {
        let low = AttributeValue::new_clamped(1);
        let high = AttributeValue::new_clamped(20);

        let baseline = AttributeValue::default();
        assert_eq!(
            (0..=4)
                .map(|bucket| inherit_attribute(Some(baseline), Some(baseline), bucket).get())
                .collect::<Vec<_>>(),
            [8, 9, 10, 11, 12]
        );
        assert_eq!(inherit_attribute(Some(low), Some(low), 0).get(), 1);
        assert_eq!(inherit_attribute(Some(high), Some(high), 4).get(), 20);
        assert_eq!(inherit_attribute(Some(low), Some(high), 2).get(), 11);
        assert_eq!(inherit_attribute(Some(high), None, 2).get(), 15);
        assert_eq!(inherit_attribute(None, None, 2).get(), 10);
    }

    #[test]
    fn seeded_inheritance_is_stable_per_newborn_and_independent_of_sibling_order() {
        let first = CatAttributes::all(1);
        let second = CatAttributes::all(20);

        let kitten_a_first =
            inherit_attributes_seeded(7, "moss-colony", "kitten-a", Some(&first), Some(&second));
        let kitten_b_second =
            inherit_attributes_seeded(7, "moss-colony", "kitten-b", Some(&first), Some(&second));
        let kitten_b_first =
            inherit_attributes_seeded(7, "moss-colony", "kitten-b", Some(&first), Some(&second));
        let kitten_a_second =
            inherit_attributes_seeded(7, "moss-colony", "kitten-a", Some(&first), Some(&second));

        assert_eq!(kitten_a_first, kitten_a_second);
        assert_eq!(kitten_b_first, kitten_b_second);
        assert_ne!(kitten_a_first, kitten_b_first);
    }

    #[test]
    fn personality_bucket_boundaries_are_exactly_eighty_fifteen_five() {
        for bucket in 0..80 {
            assert_eq!(
                PersonalityStrength::from_percentile(bucket),
                PersonalityStrength::Subtle
            );
        }
        for bucket in 80..95 {
            assert_eq!(
                PersonalityStrength::from_percentile(bucket),
                PersonalityStrength::Pronounced
            );
        }
        for bucket in 95..100 {
            assert_eq!(
                PersonalityStrength::from_percentile(bucket),
                PersonalityStrength::Extreme
            );
        }
    }

    #[test]
    fn signed_personality_values_produce_exact_fixed_point_modifiers() {
        let cases = [
            (PersonalityValue::NEUTRAL, 0),
            (
                PersonalityValue::new(PersonalityPole::Negative, PersonalityStrength::Subtle),
                -500,
            ),
            (
                PersonalityValue::new(PersonalityPole::Positive, PersonalityStrength::Subtle),
                500,
            ),
            (
                PersonalityValue::new(PersonalityPole::Negative, PersonalityStrength::Pronounced),
                -1_500,
            ),
            (
                PersonalityValue::new(PersonalityPole::Positive, PersonalityStrength::Pronounced),
                1_500,
            ),
            (
                PersonalityValue::new(PersonalityPole::Negative, PersonalityStrength::Extreme),
                -3_000,
            ),
            (
                PersonalityValue::new(PersonalityPole::Positive, PersonalityStrength::Extreme),
                3_000,
            ),
        ];

        for (value, expected) in cases {
            assert_eq!(value.signed_modifier().get(), expected);
        }
    }

    #[test]
    fn generated_personality_is_stable_per_cat_and_has_a_seed_matrix_snapshot() {
        let first = generate_personality(91, "oak-colony", "cat-17");
        assert_eq!(first, generate_personality(91, "oak-colony", "cat-17"));
        assert_ne!(first, generate_personality(91, "oak-colony", "cat-18"));
        assert_eq!(first.signed_levels(), [-1, 1, 1, -2, -1, 1, -1, -1]);

        let mut counts = [0_u32; 3];
        for cat_number in 0..1_000 {
            let personality = generate_personality(91, "oak-colony", &format!("cat-{cat_number}"));
            for axis in PersonalityAxis::ALL {
                let strength = personality
                    .value(axis)
                    .strength()
                    .expect("generated values are non-neutral");
                counts[strength.index()] += 1;
            }
        }
        assert_eq!(counts, [6_357, 1_256, 387]);
    }

    #[test]
    fn each_axis_only_changes_its_requested_weight() {
        let selected = PersonalityAxis::CautiousBold;
        let mut personality = CatPersonality::default();
        personality.set(
            selected,
            PersonalityValue::new(PersonalityPole::Positive, PersonalityStrength::Extreme),
        );

        for other in PersonalityAxis::ALL {
            if other != selected {
                personality.set(
                    other,
                    PersonalityValue::new(
                        PersonalityPole::Negative,
                        PersonalityStrength::Pronounced,
                    ),
                );
            }
        }

        assert_eq!(
            personality
                .weight_factor(selected, PersonalityPole::Positive)
                .get(),
            13_000
        );
        assert_eq!(
            personality
                .weight_factor(selected, PersonalityPole::Negative)
                .get(),
            7_000
        );
        assert_eq!(
            personality
                .weight_factor(
                    PersonalityAxis::LeisurelyDiligent,
                    PersonalityPole::Negative,
                )
                .get(),
            11_500
        );
    }

    #[test]
    fn compatibility_traits_default_when_legacy_fields_are_absent() {
        let decoded: CatTraits = serde_json::from_value(serde_json::json!({}))
            .expect("legacy trait state should default");
        assert_eq!(decoded.attributes, CatAttributes::all(10));
        assert_eq!(decoded.personality, CatPersonality::default());

        let partial: CatTraits = serde_json::from_value(serde_json::json!({
            "attributes": { "attack": 20 },
            "personality": { "cautiousBold": 3 }
        }))
        .expect("partial compatibility state should default field-by-field");
        assert_eq!(partial.attributes.attack.get(), 20);
        assert_eq!(partial.attributes.defense.get(), 10);
        assert_eq!(partial.personality.cautious_bold.signed_level(), 3);
        assert_eq!(
            partial.personality.leisurely_diligent,
            PersonalityValue::NEUTRAL
        );
    }

    #[test]
    fn personality_deserialization_rejects_out_of_range_signed_levels() {
        assert!(serde_json::from_value::<PersonalityValue>(serde_json::json!(4)).is_err());
        assert!(serde_json::from_value::<PersonalityValue>(serde_json::json!(-4)).is_err());
    }
}
