//! Player-painted map zones (pure rules), ported from `lib/game/zones.ts`.
//!
//! Players steer the colony by marking rectangles: `gather` doubles a tile's
//! appeal, `avoid` excludes it — unless a critical need leaves no other option.

/// A tile position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZonePos {
    pub x: i32,
    pub y: i32,
}

/// A normalized, inclusive-edge rectangle in tile coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneRect {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

/// Zone behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneKind {
    Avoid,
    Gather,
}

/// A placed zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zone {
    pub rect: ZoneRect,
    pub kind: ZoneKind,
}

pub const ZONE_MAX_PER_PLAYER: u32 = 2;
pub const ZONE_MAX_EDGE: i32 = 8;
pub const ZONE_MIN_DURATION_MS: i64 = 10 * 60 * 1000;
pub const ZONE_MAX_DURATION_MS: i64 = 2 * 3600 * 1000;
pub const GATHER_MULTIPLIER: f64 = 2.0;

/// JS `Math.round`: half rounds toward +∞ (`Math.round(-0.5) === 0`), unlike
/// Rust's `f64::round` (half away from zero). Reproduced as `floor(x + 0.5)`.
fn js_round(x: f64) -> i32 {
    (x + 0.5).floor() as i32
}

/// Order two (possibly fractional) corners into a normalized integer rect.
#[must_use]
pub fn normalize_rect(ax: f64, ay: f64, bx: f64, by: f64) -> ZoneRect {
    let (ax, ay, bx, by) = (js_round(ax), js_round(ay), js_round(bx), js_round(by));
    ZoneRect {
        x1: ax.min(bx),
        y1: ay.min(by),
        x2: ax.max(bx),
        y2: ay.max(by),
    }
}

/// Inclusive-edge containment.
#[must_use]
pub fn is_in_zone(pos: ZonePos, rect: ZoneRect) -> bool {
    pos.x >= rect.x1 && pos.x <= rect.x2 && pos.y >= rect.y1 && pos.y <= rect.y2
}

/// Returns a user-facing error, or `None` when the zone is acceptable.
#[must_use]
pub fn validate_zone(rect: ZoneRect, duration_ms: i64, active_player_zones: u32) -> Option<String> {
    if active_player_zones >= ZONE_MAX_PER_PLAYER {
        return Some(format!(
            "You already have {ZONE_MAX_PER_PLAYER} active zones"
        ));
    }
    if rect.x2 - rect.x1 + 1 > ZONE_MAX_EDGE || rect.y2 - rect.y1 + 1 > ZONE_MAX_EDGE {
        return Some(format!(
            "Zones are limited to {ZONE_MAX_EDGE}x{ZONE_MAX_EDGE} tiles"
        ));
    }
    if !(ZONE_MIN_DURATION_MS..=ZONE_MAX_DURATION_MS).contains(&duration_ms) {
        return Some("Zone duration must be between 10 minutes and 2 hours".to_string());
    }
    None
}

/// Zone-adjusted appeal of a tile. Gather zones double the base score; avoid
/// zones zero it unless `critical` (a need leaves no choice).
#[must_use]
pub fn score_tile_with_zones(base_score: f64, pos: ZonePos, zones: &[Zone], critical: bool) -> f64 {
    let mut score = base_score;
    for zone in zones {
        if !is_in_zone(pos, zone.rect) {
            continue;
        }
        if zone.kind == ZoneKind::Avoid && !critical {
            return 0.0;
        }
        if zone.kind == ZoneKind::Gather {
            score *= GATHER_MULTIPLIER;
        }
    }
    score
}

/// Wander/journey candidates with avoid-zone tiles removed.
#[must_use]
pub fn filter_targets_by_zones(
    targets: &[ZonePos],
    zones: &[Zone],
    critical: bool,
) -> Vec<ZonePos> {
    if critical {
        return targets.to_vec();
    }
    let avoids: Vec<&Zone> = zones.iter().filter(|z| z.kind == ZoneKind::Avoid).collect();
    if avoids.is_empty() {
        return targets.to_vec();
    }
    targets
        .iter()
        .copied()
        .filter(|target| !avoids.iter().any(|z| is_in_zone(*target, z.rect)))
        .collect()
}

/// Weighted pick over candidates: gather-zone tiles appear twice, avoid tiles are
/// gone. Falls back to the raw list when zones filter out everything (critical-need
/// behaviour). `roll` is clamped to `[0, 0.999999]`.
#[must_use]
pub fn pick_target_with_zones(targets: &[ZonePos], zones: &[Zone], roll: f64) -> Option<ZonePos> {
    if targets.is_empty() {
        return None;
    }
    let allowed = filter_targets_by_zones(targets, zones, false);
    let pool: &[ZonePos] = if allowed.is_empty() {
        targets
    } else {
        &allowed
    };

    let mut weighted: Vec<ZonePos> = Vec::new();
    for target in pool {
        weighted.push(*target);
        if zones
            .iter()
            .any(|z| z.kind == ZoneKind::Gather && is_in_zone(*target, z.rect))
        {
            weighted.push(*target);
        }
    }
    let clamped = roll.clamp(0.0, 0.999_999);
    let index = (clamped * weighted.len() as f64).floor() as usize;
    weighted.get(index).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(x: i32, y: i32) -> ZonePos {
        ZonePos { x, y }
    }
    fn rect(x1: i32, y1: i32, x2: i32, y2: i32) -> ZoneRect {
        ZoneRect { x1, y1, x2, y2 }
    }
    fn gather(r: ZoneRect) -> Zone {
        Zone {
            rect: r,
            kind: ZoneKind::Gather,
        }
    }
    fn avoid(r: ZoneRect) -> Zone {
        Zone {
            rect: r,
            kind: ZoneKind::Avoid,
        }
    }

    #[test]
    fn js_round_half_towards_positive_infinity() {
        assert_eq!(js_round(0.5), 1);
        assert_eq!(js_round(-0.5), 0); // JS Math.round(-0.5) === 0
        assert_eq!(js_round(2.5), 3);
        assert_eq!(js_round(-2.5), -2);
    }

    #[test]
    fn normalize_orders_and_rounds_corners() {
        assert_eq!(normalize_rect(3.4, 1.6, 0.5, 4.4), rect(1, 2, 3, 4));
    }

    #[test]
    fn inclusive_edge_containment() {
        let r = rect(0, 0, 2, 2);
        assert!(is_in_zone(pos(0, 0), r));
        assert!(is_in_zone(pos(2, 2), r));
        assert!(!is_in_zone(pos(3, 2), r));
    }

    #[test]
    fn validate_rejects_over_limit_edge_and_duration() {
        assert!(validate_zone(rect(0, 0, 7, 0), ZONE_MIN_DURATION_MS, 0).is_none());
        assert!(validate_zone(rect(0, 0, 8, 0), ZONE_MIN_DURATION_MS, 0).is_some()); // 9 wide
        assert!(validate_zone(rect(0, 0, 0, 0), ZONE_MIN_DURATION_MS - 1, 0).is_some());
        assert!(validate_zone(rect(0, 0, 0, 0), ZONE_MAX_DURATION_MS + 1, 0).is_some());
        assert!(
            validate_zone(rect(0, 0, 0, 0), ZONE_MIN_DURATION_MS, ZONE_MAX_PER_PLAYER).is_some()
        );
    }

    #[test]
    fn gather_doubles_and_avoid_zeros_unless_critical() {
        let g = gather(rect(0, 0, 2, 2));
        let a = avoid(rect(0, 0, 2, 2));
        assert_eq!(score_tile_with_zones(10.0, pos(1, 1), &[g], false), 20.0);
        assert_eq!(score_tile_with_zones(10.0, pos(1, 1), &[a], false), 0.0);
        assert_eq!(score_tile_with_zones(10.0, pos(1, 1), &[a], true), 10.0);
        assert_eq!(score_tile_with_zones(10.0, pos(5, 5), &[g, a], false), 10.0);
    }

    #[test]
    fn filter_removes_avoid_tiles_unless_critical() {
        let a = avoid(rect(0, 0, 1, 1));
        let targets = [pos(0, 0), pos(5, 5)];
        assert_eq!(
            filter_targets_by_zones(&targets, &[a], false),
            vec![pos(5, 5)]
        );
        assert_eq!(
            filter_targets_by_zones(&targets, &[a], true),
            targets.to_vec()
        );
        assert_eq!(
            filter_targets_by_zones(&targets, &[], false),
            targets.to_vec()
        );
    }

    #[test]
    fn weighted_pick_favours_gather_and_clamps_roll() {
        let g = gather(rect(0, 0, 0, 0));
        let targets = [pos(0, 0), pos(5, 5)]; // (0,0) gathered -> appears twice
        // weighted = [ (0,0), (0,0), (5,5) ]; roll 0 -> index 0
        assert_eq!(pick_target_with_zones(&targets, &[g], 0.0), Some(pos(0, 0)));
        // roll 1 clamps to 0.999999 -> index floor(0.999999*3)=2 -> (5,5)
        assert_eq!(pick_target_with_zones(&targets, &[g], 1.0), Some(pos(5, 5)));
        assert_eq!(pick_target_with_zones(&[], &[g], 0.5), None);
    }

    #[test]
    fn pick_falls_back_to_raw_when_all_avoided() {
        let a = avoid(rect(0, 0, 9, 9));
        let targets = [pos(1, 1), pos(2, 2)];
        // all filtered out -> pool falls back to raw targets
        assert!(pick_target_with_zones(&targets, &[a], 0.0).is_some());
    }
}
