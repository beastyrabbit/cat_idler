//! Deterministic Hunting Lair domain rules.
//!
//! Hunting Lairs are the runtime interpretation of `TerrainType::EnemyLair`.
//! They intentionally remain separate from the quarry-oriented
//! `TerrainType::CaveEntrance`.

use serde::{Deserialize, Serialize};

use crate::rng::roll_seeded;

const ROSTER_GENERATION_SEED_OFFSET: u32 = 4_000_003;
const SOLO_PARTY_CAP: usize = 1;
const GROUP_PARTY_CAP: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonsterSpecies {
    Fox,
    Badger,
    Bear,
    RivalBeast,
}

impl MonsterSpecies {
    #[must_use]
    pub const fn threat(self) -> u8 {
        match self {
            Self::Fox => 20,
            Self::Badger => 35,
            Self::Bear => 60,
            Self::RivalBeast => 90,
        }
    }

    #[must_use]
    pub const fn respawn_cooldown_game_hours(self) -> u64 {
        match self {
            Self::Fox => 6,
            Self::Badger => 8,
            Self::Bear => 12,
            Self::RivalBeast => 18,
        }
    }

    #[must_use]
    pub const fn base_loot(self) -> BaseLoot {
        match self {
            Self::Fox => BaseLoot::new(12, 2, 1),
            Self::Badger => BaseLoot::new(18, 3, 2),
            Self::Bear => BaseLoot::new(30, 6, 4),
            Self::RivalBeast => BaseLoot::new(45, 8, 6),
        }
    }

    #[must_use]
    pub const fn species_drop_percent(self) -> u8 {
        match self {
            Self::Fox => 10,
            Self::Badger => 15,
            Self::Bear => 25,
            Self::RivalBeast => 40,
        }
    }

    #[must_use]
    pub const fn species_material(self) -> SpeciesMaterial {
        match self {
            Self::Fox => SpeciesMaterial::FoxPelt,
            Self::Badger => SpeciesMaterial::BadgerPelt,
            Self::Bear => SpeciesMaterial::BearPelt,
            Self::RivalBeast => SpeciesMaterial::BeastCore,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeciesMaterial {
    FoxPelt,
    BadgerPelt,
    BearPelt,
    BeastCore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseLoot {
    pub food: u32,
    pub hide: u32,
    pub bone: u32,
}

impl BaseLoot {
    const fn new(food: u32, hide: u32, bone: u32) -> Self {
        Self { food, hide, bone }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LairMonster {
    pub id: u32,
    pub species: MonsterSpecies,
    pub health: u16,
}

impl LairMonster {
    #[must_use]
    pub const fn is_alive(&self) -> bool {
        self.health > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntingLair {
    pub environmental_danger: u8,
    pub generation: u32,
    pub monsters: Vec<LairMonster>,
    pub first_clear_claimed: bool,
    pub respawn_ready_at_ms: Option<i64>,
}

impl HuntingLair {
    /// Convenience constructor for fixtures, migrations, and authored scenarios.
    #[must_use]
    pub fn from_species(
        environmental_danger: u8,
        species: impl IntoIterator<Item = MonsterSpecies>,
    ) -> Self {
        let monsters = species
            .into_iter()
            .enumerate()
            .map(|(index, species)| LairMonster {
                id: u32::try_from(index).unwrap_or(u32::MAX),
                species,
                health: u16::from(species.threat()),
            })
            .collect();

        Self {
            environmental_danger: environmental_danger.min(100),
            generation: 0,
            monsters,
            first_clear_claimed: false,
            respawn_ready_at_ms: None,
        }
    }

    #[must_use]
    pub fn current_danger(&self) -> u8 {
        self.monsters
            .iter()
            .filter(|monster| monster.is_alive())
            .map(|monster| u16::from(monster.species.threat()))
            .fold(0_u16, u16::saturating_add)
            .min(100) as u8
    }

    #[must_use]
    pub fn strongest_living_species(&self) -> Option<MonsterSpecies> {
        self.monsters
            .iter()
            .filter(|monster| monster.is_alive())
            .map(|monster| monster.species)
            .max_by_key(|species| species.threat())
    }

    /// Returns a new generation only after a fully cleared lair's cooldown.
    #[must_use]
    pub fn respawn_if_ready(&self, now_ms: i64, world_seed: u32) -> Option<Self> {
        let ready_at = self.respawn_ready_at_ms?;
        if self.current_danger() > 0 || now_ms < ready_at {
            return None;
        }

        let mut respawned = generate_roster(
            self.environmental_danger,
            world_seed,
            self.generation.wrapping_add(1),
        );
        respawned.first_clear_claimed = self.first_clear_claimed;
        Some(respawned)
    }
}

/// Generates one monster below danger 85, two from 85 through 94, and three
/// from 95 onward. Species are selected only from the danger-unlocked pool.
#[must_use]
pub fn generate_roster(environmental_danger: u8, world_seed: u32, generation: u32) -> HuntingLair {
    let environmental_danger = environmental_danger.min(100);
    let roster_size = match environmental_danger {
        0..=84 => 1,
        85..=94 => 2,
        _ => 3,
    };
    let species_pool: &[MonsterSpecies] = match environmental_danger {
        0..=84 => &[MonsterSpecies::Fox],
        85..=89 => &[MonsterSpecies::Fox, MonsterSpecies::Badger],
        90..=94 => &[
            MonsterSpecies::Fox,
            MonsterSpecies::Badger,
            MonsterSpecies::Bear,
        ],
        _ => &[
            MonsterSpecies::Fox,
            MonsterSpecies::Badger,
            MonsterSpecies::Bear,
            MonsterSpecies::RivalBeast,
        ],
    };

    let mut seed = world_seed.wrapping_add(generation.wrapping_mul(ROSTER_GENERATION_SEED_OFFSET));
    let mut monsters = Vec::with_capacity(roster_size);
    for index in 0..roster_size {
        let roll = roll_seeded(f64::from(seed));
        seed = roll.next_seed;
        let pool_index =
            ((roll.value * species_pool.len() as f64).floor() as usize).min(species_pool.len() - 1);
        let species = species_pool[pool_index];
        monsters.push(LairMonster {
            id: generation
                .wrapping_mul(GROUP_PARTY_CAP as u32)
                .wrapping_add(index as u32),
            species,
            health: u16::from(species.threat()),
        });
    }

    HuntingLair {
        environmental_danger,
        generation,
        monsters,
        first_clear_claimed: false,
        respawn_ready_at_ms: None,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunter {
    pub cat_id: String,
    pub combat_power: f64,
    pub health_percent: f64,
    pub weapon_bonus: f64,
    pub armor_bonus: f64,
}

impl Hunter {
    fn effective_power(&self) -> f64 {
        (self.combat_power + self.weapon_bonus + self.armor_bonus * 0.5).max(0.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HuntAdvice {
    Reckless,
    Risky,
    Favored,
    Safe,
}

impl HuntAdvice {
    #[must_use]
    pub const fn from_success_percent(success_percent: u8) -> Self {
        match success_percent {
            0..=49 => Self::Reckless,
            50..=69 => Self::Risky,
            70..=89 => Self::Favored,
            _ => Self::Safe,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptAuthority {
    AutonomousLeader,
    PlayerNudge,
}

#[must_use]
pub fn predicted_success_percent(lair: &HuntingLair, party: &[Hunter]) -> u8 {
    let party_power: f64 = party.iter().map(Hunter::effective_power).sum();
    (50.0 + party_power - f64::from(lair.current_danger()))
        .round()
        .clamp(5.0, 95.0) as u8
}

/// Captain advisory gate. The Leader owns autonomous dispatch; the player can
/// explicitly nudge a healthier party into a riskier attempt.
#[must_use]
pub fn attempt_is_authorized(
    authority: AttemptAuthority,
    predicted_success_percent: u8,
    party: &[Hunter],
) -> bool {
    let (minimum_success, minimum_health) = match authority {
        AttemptAuthority::AutonomousLeader => (70, 70.0),
        AttemptAuthority::PlayerNudge => (45, 80.0),
    };

    !party.is_empty()
        && predicted_success_percent >= minimum_success
        && party
            .iter()
            .all(|hunter| hunter.health_percent >= minimum_health)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstClearTrophy {
    pub species: MonsterSpecies,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntLoot {
    pub food: u32,
    pub hide: u32,
    pub bone: u32,
    pub species_materials: Vec<SpeciesMaterial>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantResult {
    pub cat_id: String,
    pub damage: u8,
    /// Integration maps this to `DeathCause::Hunt`.
    pub died: bool,
    /// Apply to the cat's Hunting skill even when the attempt fails.
    pub hunting_xp: u16,
    /// Apply to the cat's Fight labor XP even when the attempt fails.
    pub fight_xp: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntResolution {
    pub lair: HuntingLair,
    pub cleared: bool,
    pub predicted_success_percent: u8,
    pub advice: HuntAdvice,
    pub roll: f64,
    pub next_seed: u32,
    pub participants: Vec<ParticipantResult>,
    pub loot: HuntLoot,
    pub first_clear_trophy: Option<FirstClearTrophy>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HuntError {
    EmptyParty,
    InvalidPartyCap(usize),
    InvalidGameHourDuration(i64),
    PartyTooLarge { supplied: usize, cap: usize },
    LairEmpty,
}

/// Resolves one full-clear attempt. A success defeats the living roster; a
/// failure leaves it untouched. The first RNG roll decides combat, followed by
/// one independent species-material roll per defeated monster.
pub fn resolve_attempt(
    lair: &HuntingLair,
    party: &[Hunter],
    party_cap: usize,
    seed: u32,
    now_ms: i64,
    game_hour_ms: i64,
) -> Result<HuntResolution, HuntError> {
    if party.is_empty() {
        return Err(HuntError::EmptyParty);
    }
    if !matches!(party_cap, SOLO_PARTY_CAP | GROUP_PARTY_CAP) {
        return Err(HuntError::InvalidPartyCap(party_cap));
    }
    if game_hour_ms <= 0 {
        return Err(HuntError::InvalidGameHourDuration(game_hour_ms));
    }
    if party.len() > party_cap {
        return Err(HuntError::PartyTooLarge {
            supplied: party.len(),
            cap: party_cap,
        });
    }
    let strongest_species = lair
        .strongest_living_species()
        .ok_or(HuntError::LairEmpty)?;
    let danger = lair.current_danger();
    let success_percent = predicted_success_percent(lair, party);
    let combat_roll = roll_seeded(f64::from(seed));
    let won = combat_roll.value < f64::from(success_percent) / 100.0;

    let (hunting_xp, fight_xp) = if won {
        (4 + u16::from(danger) / 20, 3 + u16::from(danger) / 25)
    } else {
        (1 + u16::from(danger) / 50, 1 + u16::from(danger) / 50)
    };
    let average_power = party.iter().map(Hunter::effective_power).sum::<f64>() / party.len() as f64;
    let participants = party
        .iter()
        .map(|hunter| {
            let damage = if won {
                0
            } else {
                (20.0 + (f64::from(danger) - average_power).max(0.0) - hunter.armor_bonus)
                    .round()
                    .clamp(10.0, 90.0) as u8
            };
            ParticipantResult {
                cat_id: hunter.cat_id.clone(),
                damage,
                died: f64::from(damage) >= hunter.health_percent,
                hunting_xp,
                fight_xp,
            }
        })
        .collect();

    if !won {
        return Ok(HuntResolution {
            lair: lair.clone(),
            cleared: false,
            predicted_success_percent: success_percent,
            advice: HuntAdvice::from_success_percent(success_percent),
            roll: combat_roll.value,
            next_seed: combat_roll.next_seed,
            participants,
            loot: HuntLoot::default(),
            first_clear_trophy: None,
        });
    }

    let mut next_seed = combat_roll.next_seed;
    let mut loot = HuntLoot::default();
    for monster in lair.monsters.iter().filter(|monster| monster.is_alive()) {
        let base = monster.species.base_loot();
        loot.food = loot.food.saturating_add(base.food);
        loot.hide = loot.hide.saturating_add(base.hide);
        loot.bone = loot.bone.saturating_add(base.bone);

        let drop_roll = roll_seeded(f64::from(next_seed));
        next_seed = drop_roll.next_seed;
        if drop_roll.value < f64::from(monster.species.species_drop_percent()) / 100.0 {
            loot.species_materials
                .push(monster.species.species_material());
        }
    }

    let is_first_clear = !lair.first_clear_claimed;
    if is_first_clear && loot.species_materials.is_empty() {
        loot.species_materials
            .push(strongest_species.species_material());
    }
    let first_clear_trophy = is_first_clear.then_some(FirstClearTrophy {
        species: strongest_species,
    });

    let mut cleared_lair = lair.clone();
    for monster in &mut cleared_lair.monsters {
        monster.health = 0;
    }
    cleared_lair.first_clear_claimed = true;
    let cooldown_hours =
        i64::try_from(strongest_species.respawn_cooldown_game_hours()).unwrap_or(i64::MAX);
    let cooldown_ms = game_hour_ms.saturating_mul(cooldown_hours);
    cleared_lair.respawn_ready_at_ms = Some(now_ms.saturating_add(cooldown_ms));

    Ok(HuntResolution {
        lair: cleared_lair,
        cleared: true,
        predicted_success_percent: success_percent,
        advice: HuntAdvice::from_success_percent(success_percent),
        roll: combat_roll.value,
        next_seed,
        participants,
        loot,
        first_clear_trophy,
    })
}
