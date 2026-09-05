//! Deterministic, player-keyed personal-village site selection.
//!
//! This additive module owns no world state and consumes no simulation RNG. It
//! is ready for a later founding action to call, but does not alter the current
//! colony placement path.

use std::collections::BTreeSet;

use crate::{
    terrain_gen::{BiomeRole, tile_biome, tile_climate_biome, tile_has_tree},
    world_tick::TilePos,
};

/// Matches the live world's established village-separation guarantee.
pub const MIN_PERSONAL_VILLAGE_SEPARATION: i32 = 48;
/// Probes before selection reports that it entered its collision fallback.
pub const PRIMARY_PROBE_COUNT: usize = 512;

const SITE_GRID_WIDTH: u64 = 257;
const SITE_GRID_SLOTS: u64 = SITE_GRID_WIDTH * SITE_GRID_WIDTH;
const SITE_GRID_RADIUS: i64 = (SITE_GRID_WIDTH as i64 - 1) / 2;
const SITE_ORIGIN: TilePos = TilePos { x: 6, y: 6 };
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
const SITE_DOMAIN: &[u8] = b"idle-cat-forest/personal-village-site/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VillageSiteSelection {
    pub anchor: TilePos,
    /// One-based number of candidates considered.
    pub probes: usize,
    pub used_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerVillageSite {
    pub player_id: String,
    pub anchor: TilePos,
}

/// Select one player's stable preferred site around the canonical world origin.
/// Existing anchors are sorted and deduplicated before collision checks, making
/// their input order irrelevant. A player's preferred sequence depends only on
/// the world seed and stable player identifier.
#[must_use]
pub fn select_personal_village_site(
    world_seed: u32,
    stable_player_id: &str,
    existing_anchors: &[TilePos],
) -> Option<VillageSiteSelection> {
    let mut occupied = existing_anchors.to_vec();
    occupied.sort_unstable();
    occupied.dedup();

    for probe in 0..usize::try_from(SITE_GRID_SLOTS).ok()? {
        let candidate = open_address_candidate(world_seed, stable_player_id, probe)?;
        if is_separated_from_all(candidate, &occupied)
            && is_buildable_personal_village_site(world_seed, candidate)
        {
            return Some(VillageSiteSelection {
                anchor: candidate,
                probes: probe + 1,
                used_fallback: probe >= PRIMARY_PROBE_COUNT,
            });
        }
    }
    None
}

/// Allocate several personal villages in stable player-ID order. This is the
/// order-independent entry point for creating a batch: caller order and repeated
/// IDs cannot change the result. Collisions are resolved by each player's open-
/// address sequence against the already allocated, sorted anchors.
#[must_use]
pub fn allocate_personal_village_sites(
    world_seed: u32,
    stable_player_ids: &[String],
    existing_anchors: &[TilePos],
) -> Option<Vec<PlayerVillageSite>> {
    let players: BTreeSet<&str> = stable_player_ids.iter().map(String::as_str).collect();
    let mut occupied = existing_anchors.to_vec();
    occupied.sort_unstable();
    occupied.dedup();
    let mut sites = Vec::with_capacity(players.len());

    for player_id in players {
        let selection = select_personal_village_site(world_seed, player_id, &occupied)?;
        occupied.push(selection.anchor);
        occupied.sort_unstable();
        sites.push(PlayerVillageSite {
            player_id: player_id.to_owned(),
            anchor: selection.anchor,
        });
    }
    Some(sites)
}

/// A founding anchor must put the shrine centre on undecorated grass/lowland.
/// The later founding stamp may flatten its local blueprint, but selection does
/// not knowingly choose water, cliffs, mountains, or a standing tree.
#[must_use]
pub fn is_buildable_personal_village_site(world_seed: u32, anchor: TilePos) -> bool {
    let Some(center_x) = anchor.x.checked_add(1) else {
        return false;
    };
    let Some(center_y) = anchor.y.checked_add(1) else {
        return false;
    };
    matches!(
        tile_biome(world_seed, center_x, center_y),
        BiomeRole::Grassland | BiomeRole::Lowland
    ) && tile_climate_biome(world_seed, center_x, center_y)
        .properties()
        .passable
        && !tile_has_tree(world_seed, center_x, center_y)
}

#[must_use]
pub fn chebyshev_distance(left: TilePos, right: TilePos) -> i64 {
    let dx = (i64::from(left.x) - i64::from(right.x)).abs();
    let dy = (i64::from(left.y) - i64::from(right.y)).abs();
    dx.max(dy)
}

fn is_separated_from_all(candidate: TilePos, existing_anchors: &[TilePos]) -> bool {
    existing_anchors.iter().all(|anchor| {
        chebyshev_distance(candidate, *anchor) >= i64::from(MIN_PERSONAL_VILLAGE_SEPARATION)
    })
}

fn open_address_candidate(
    world_seed: u32,
    stable_player_id: &str,
    probe: usize,
) -> Option<TilePos> {
    let probe = u64::try_from(probe).ok()?;
    if probe >= SITE_GRID_SLOTS {
        return None;
    }
    let hash = stable_site_hash(world_seed, stable_player_id);
    let start = mix64(hash) % SITE_GRID_SLOTS;
    let mut stride = mix64(hash ^ 0x9e37_79b9_7f4a_7c15) % (SITE_GRID_SLOTS - 1) + 1;
    if stride.is_multiple_of(SITE_GRID_WIDTH) {
        stride = if stride + 1 == SITE_GRID_SLOTS {
            1
        } else {
            stride + 1
        };
    }
    let slot = (start + probe * stride % SITE_GRID_SLOTS) % SITE_GRID_SLOTS;
    let grid_x = i64::try_from(slot % SITE_GRID_WIDTH).ok()? - SITE_GRID_RADIUS;
    let grid_y = i64::try_from(slot / SITE_GRID_WIDTH).ok()? - SITE_GRID_RADIUS;
    let x = i64::from(SITE_ORIGIN.x) + grid_x * i64::from(MIN_PERSONAL_VILLAGE_SEPARATION);
    let y = i64::from(SITE_ORIGIN.y) + grid_y * i64::from(MIN_PERSONAL_VILLAGE_SEPARATION);
    Some(TilePos {
        x: i32::try_from(x).ok()?,
        y: i32::try_from(y).ok()?,
    })
}

fn stable_site_hash(world_seed: u32, stable_player_id: &str) -> u64 {
    let mut hash = FNV_OFFSET;
    for byte in SITE_DOMAIN
        .iter()
        .chain(world_seed.to_le_bytes().iter())
        .chain(stable_player_id.as_bytes())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

const fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_tick::TilePos;

    #[test]
    fn player_sites_are_deterministic_buildable_and_well_separated() {
        for seed in 0..12 {
            let players: Vec<String> = (0..10).map(|id| format!("player-{id}")).collect();
            let first = allocate_personal_village_sites(seed, &players, &[]).unwrap();
            let second = allocate_personal_village_sites(seed, &players, &[]).unwrap();
            assert_eq!(first, second);
            for (index, site) in first.iter().enumerate() {
                assert!(is_buildable_personal_village_site(seed, site.anchor));
                for other in &first[..index] {
                    assert!(
                        chebyshev_distance(site.anchor, other.anchor)
                            >= i64::from(MIN_PERSONAL_VILLAGE_SEPARATION)
                    );
                }
            }
        }
    }

    #[test]
    fn player_and_anchor_input_order_do_not_change_allocation() {
        let mut players = vec!["moss", "ember", "reed", "cloud"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let existing = [TilePos { x: 6, y: 6 }, TilePos { x: 600, y: -300 }];
        let expected = allocate_personal_village_sites(91, &players, &existing).unwrap();
        players.reverse();
        let reversed_existing = [existing[1], existing[0]];
        assert_eq!(
            allocate_personal_village_sites(91, &players, &reversed_existing).unwrap(),
            expected
        );
    }

    #[test]
    fn a_players_preferred_site_is_independent_of_unrelated_founding_order() {
        let empty = select_personal_village_site(44, "constant-player", &[]).unwrap();
        let far = TilePos {
            x: 20_000,
            y: 20_000,
        };
        let with_non_colliding_anchor =
            select_personal_village_site(44, "constant-player", &[far]).unwrap();
        assert_eq!(empty.anchor, with_non_colliding_anchor.anchor);
    }

    #[test]
    fn collision_exhaustion_uses_the_deterministic_fallback_probe_range() {
        let seed = 17;
        let player = "crowded-player";
        let occupied: Vec<TilePos> = (0..PRIMARY_PROBE_COUNT)
            .map(|probe| open_address_candidate(seed, player, probe).unwrap())
            .collect();
        let left = select_personal_village_site(seed, player, &occupied).unwrap();
        let right = select_personal_village_site(seed, player, &occupied).unwrap();
        assert_eq!(left, right);
        assert!(left.used_fallback);
        assert!(left.probes > PRIMARY_PROBE_COUNT);
        assert!(is_separated_from_all(left.anchor, &occupied));
        assert!(is_buildable_personal_village_site(seed, left.anchor));
    }

    #[test]
    fn separation_math_is_overflow_safe_at_coordinate_extremes() {
        let extremes = [
            TilePos {
                x: i32::MIN,
                y: i32::MIN,
            },
            TilePos {
                x: i32::MAX,
                y: i32::MAX,
            },
        ];
        assert_eq!(
            chebyshev_distance(extremes[0], extremes[1]),
            u32::MAX as i64
        );
        let site = select_personal_village_site(u32::MAX, "extreme", &extremes).unwrap();
        assert!(is_separated_from_all(site.anchor, &extremes));
    }
}
