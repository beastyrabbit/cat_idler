//! Seeded noise utilities ported from `lib/game/noise.ts`.

const MODULUS: f64 = 4_294_967_296.0;
const MULTIPLIER: f64 = 1_664_525.0;
const INCREMENT: f64 = 1_013_904_223.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeededRandom {
    seed: f64,
}

impl SeededRandom {
    pub fn new(seed: f64) -> Self {
        Self { seed }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f64 {
        self.seed = (self.seed * MULTIPLIER + INCREMENT) % MODULUS;
        f64::from(to_uint32(self.seed)) / MODULUS
    }

    pub fn int(&mut self, min: i32, max: i32) -> i32 {
        let width = f64::from(max) - f64::from(min) + 1.0;
        (self.next() * width).floor() as i32 + min
    }

    pub fn float(&mut self, min: f64, max: f64) -> f64 {
        self.next() * (max - min) + min
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HashSeedPart<'a> {
    Number(f64),
    Text(&'a str),
}

impl From<f64> for HashSeedPart<'_> {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<i32> for HashSeedPart<'_> {
    fn from(value: i32) -> Self {
        Self::Number(f64::from(value))
    }
}

impl From<u32> for HashSeedPart<'_> {
    fn from(value: u32) -> Self {
        Self::Number(f64::from(value))
    }
}

impl<'a> From<&'a str> for HashSeedPart<'a> {
    fn from(value: &'a str) -> Self {
        Self::Text(value)
    }
}

impl<'a> From<&'a String> for HashSeedPart<'a> {
    fn from(value: &'a String) -> Self {
        Self::Text(value.as_str())
    }
}

pub fn create_seeded_random(seed: f64) -> SeededRandom {
    SeededRandom::new(seed)
}

pub fn hash_seed(values: &[HashSeedPart<'_>]) -> u32 {
    let mut hash = 0_i32;

    for value in values {
        let value_string = value.to_js_string();
        for code_unit in value_string.encode_utf16() {
            hash = hash
                .wrapping_shl(5)
                .wrapping_sub(hash)
                .wrapping_add(i32::from(code_unit));
        }
    }

    hash.unsigned_abs()
}

pub fn noise_2d(x: f64, y: f64, seed: f64, scale: f64) -> f64 {
    let fx = (x * scale).floor();
    let fy = (y * scale).floor();

    let mut rng = create_seeded_random(f64::from(hash_seed(&[
        HashSeedPart::Number(seed),
        HashSeedPart::Number(fx),
        HashSeedPart::Number(fy),
    ])));
    let n00 = rng.next();

    let mut rng = create_seeded_random(f64::from(hash_seed(&[
        HashSeedPart::Number(seed),
        HashSeedPart::Number(fx + 1.0),
        HashSeedPart::Number(fy),
    ])));
    let n10 = rng.next();

    let mut rng = create_seeded_random(f64::from(hash_seed(&[
        HashSeedPart::Number(seed),
        HashSeedPart::Number(fx),
        HashSeedPart::Number(fy + 1.0),
    ])));
    let n01 = rng.next();

    let mut rng = create_seeded_random(f64::from(hash_seed(&[
        HashSeedPart::Number(seed),
        HashSeedPart::Number(fx + 1.0),
        HashSeedPart::Number(fy + 1.0),
    ])));
    let n11 = rng.next();

    let dx = x * scale - fx;
    let dy = y * scale - fy;

    let nx0 = n00 * (1.0 - dx) + n10 * dx;
    let nx1 = n01 * (1.0 - dx) + n11 * dx;

    nx0 * (1.0 - dy) + nx1 * dy
}

pub fn fractal_noise_2d(
    x: f64,
    y: f64,
    seed: f64,
    octaves: u32,
    persistence: f64,
    scale: f64,
) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = scale;
    let mut max_value = 0.0;

    for i in 0..octaves {
        value += noise_2d(x, y, seed + f64::from(i), frequency) * amplitude;
        max_value += amplitude;
        amplitude *= persistence;
        frequency *= 2.0;
    }

    value / max_value
}

impl HashSeedPart<'_> {
    fn to_js_string(self) -> String {
        match self {
            Self::Number(value) => number_to_js_string(value),
            Self::Text(value) => value.to_owned(),
        }
    }
}

fn number_to_js_string(value: f64) -> String {
    let mut buffer = ryu_js::Buffer::new();
    buffer.format(value).to_owned()
}

fn to_uint32(value: f64) -> u32 {
    if !value.is_finite() || value == 0.0 {
        return 0;
    }

    value.trunc().rem_euclid(MODULUS) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::Value;

    const EPSILON: f64 = 1e-12;

    #[derive(Debug, Deserialize)]
    struct Fixture {
        counts: Counts,
        hash: Vec<HashCase>,
        random: Vec<RandomCase>,
        noise: Vec<NoiseCase>,
        fractal: Vec<FractalCase>,
    }

    #[derive(Debug, Deserialize)]
    struct Counts {
        hash: usize,
        random: usize,
        noise: usize,
        fractal: usize,
        total: usize,
    }

    #[derive(Debug, Deserialize)]
    struct HashCase {
        inputs: Vec<Value>,
        value: u32,
    }

    #[derive(Debug, Deserialize)]
    struct RandomCase {
        seed: f64,
        operations: Vec<RandomOperation>,
    }

    #[derive(Debug, Deserialize)]
    struct RandomOperation {
        kind: String,
        min: Option<f64>,
        max: Option<f64>,
        value: Value,
    }

    #[derive(Debug, Deserialize)]
    struct NoiseCase {
        x: f64,
        y: f64,
        seed: f64,
        scale: f64,
        value: f64,
    }

    #[derive(Debug, Deserialize)]
    struct FractalCase {
        x: f64,
        y: f64,
        seed: f64,
        octaves: u32,
        persistence: f64,
        scale: f64,
        value: f64,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../../docs/migration/fixtures/p2/noise_vectors.json"
        ))
        .expect("noise fixture deserializes")
    }

    fn assert_js_float_eq(actual: f64, expected: f64) {
        if actual.to_bits() == expected.to_bits() {
            return;
        }

        assert!(
            (actual - expected).abs() <= EPSILON,
            "actual {actual:?} expected {expected:?}"
        );
    }

    fn hash_input_parts(inputs: &[Value]) -> Vec<HashSeedPart<'_>> {
        inputs
            .iter()
            .map(|value| match value {
                Value::Number(number) => {
                    HashSeedPart::Number(number.as_f64().expect("fixture number is f64"))
                }
                Value::String(text) => HashSeedPart::Text(text.as_str()),
                other => panic!("unsupported hash input {other:?}"),
            })
            .collect()
    }

    #[test]
    fn fixture_counts_match_generated_vectors() {
        let fixture = fixture();

        assert_eq!(fixture.counts.hash, fixture.hash.len());
        assert_eq!(fixture.counts.random, fixture.random.len());
        assert_eq!(fixture.counts.noise, fixture.noise.len());
        assert_eq!(fixture.counts.fractal, fixture.fractal.len());
        assert_eq!(
            fixture.counts.total,
            fixture.hash.len() + fixture.random.len() + fixture.noise.len() + fixture.fractal.len()
        );
    }

    #[test]
    fn hash_seed_matches_ts_vectors() {
        for case in fixture().hash {
            let parts = hash_input_parts(&case.inputs);

            assert_eq!(hash_seed(&parts), case.value, "inputs {:?}", case.inputs);
        }
    }

    #[test]
    fn hash_seed_matches_ts_number_stringification_edges() {
        assert_eq!(hash_seed(&[HashSeedPart::Number(1e21)]), 48_304_342);
        assert_eq!(hash_seed(&[HashSeedPart::Number(1e-7)]), 1_558_270);
    }

    #[test]
    fn seeded_random_matches_ts_vectors() {
        for case in fixture().random {
            let mut rng = create_seeded_random(case.seed);

            for operation in case.operations {
                match operation.kind.as_str() {
                    "next" => assert_js_float_eq(
                        rng.next(),
                        operation.value.as_f64().expect("next value is f64"),
                    ),
                    "int" => {
                        let min = operation.min.expect("int min") as i32;
                        let max = operation.max.expect("int max") as i32;
                        let expected = operation.value.as_i64().expect("int value") as i32;

                        assert_eq!(rng.int(min, max), expected);
                    }
                    "float" => assert_js_float_eq(
                        rng.float(
                            operation.min.expect("float min"),
                            operation.max.expect("float max"),
                        ),
                        operation.value.as_f64().expect("float value is f64"),
                    ),
                    other => panic!("unsupported operation {other}"),
                }
            }
        }
    }

    #[test]
    fn noise_2d_matches_ts_vectors() {
        for case in fixture().noise {
            assert_js_float_eq(noise_2d(case.x, case.y, case.seed, case.scale), case.value);
        }
    }

    #[test]
    fn fractal_noise_2d_matches_ts_vectors() {
        for case in fixture().fractal {
            assert_js_float_eq(
                fractal_noise_2d(
                    case.x,
                    case.y,
                    case.seed,
                    case.octaves,
                    case.persistence,
                    case.scale,
                ),
                case.value,
            );
        }
    }

    #[test]
    fn noise_functions_are_deterministic_for_same_seed_and_coordinates() {
        let first = noise_2d(-12.25, 8.75, 987_654_321.0, 0.625);
        let second = noise_2d(-12.25, 8.75, 987_654_321.0, 0.625);
        assert_eq!(first.to_bits(), second.to_bits());

        let first = fractal_noise_2d(-12.25, 8.75, 987_654_321.0, 5, 0.55, 0.625);
        let second = fractal_noise_2d(-12.25, 8.75, 987_654_321.0, 5, 0.55, 0.625);
        assert_eq!(first.to_bits(), second.to_bits());
    }
}
