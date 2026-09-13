//! Shrine deposit rules ported from `lib/game/shrine.ts`.

use std::borrow::Borrow;

use crate::entities::{Carrying, Position};
use crate::movement::WorldPos;

/// Credit no later than this long after the producing job ended.
pub const DEPOSIT_GRACE_MS: i64 = 60_000;

/// A carrier within this Chebyshev distance of the shrine deposits.
pub const DEPOSIT_RADIUS: f64 = 1.0;

pub trait ShrinePos {
    #[must_use]
    fn x(&self) -> f64;

    #[must_use]
    fn y(&self) -> f64;
}

impl ShrinePos for Position {
    fn x(&self) -> f64 {
        self.x
    }

    fn y(&self) -> f64 {
        self.y
    }
}

impl ShrinePos for WorldPos {
    fn x(&self) -> f64 {
        self.x
    }

    fn y(&self) -> f64 {
        self.y
    }
}

impl<T: ShrinePos + ?Sized> ShrinePos for &T {
    fn x(&self) -> f64 {
        (*self).x()
    }

    fn y(&self) -> f64 {
        (*self).y()
    }
}

#[must_use]
pub fn is_at_shrine(pos: impl ShrinePos, shrine: impl ShrinePos) -> bool {
    js_max((pos.x() - shrine.x()).abs(), (pos.y() - shrine.y()).abs()) <= DEPOSIT_RADIUS
}

/// True once the grace window has elapsed; credit regardless of position.
#[must_use]
pub fn should_force_deposit(carrying: impl Borrow<Carrying>, now: i64) -> bool {
    should_force_deposit_with_grace(carrying, now, DEPOSIT_GRACE_MS)
}

#[must_use]
pub fn should_force_deposit_with_grace(
    carrying: impl Borrow<Carrying>,
    now: i64,
    grace_ms: i64,
) -> bool {
    now >= carrying.borrow().job_ended_at + grace_ms
}

/// Whether a carrier deposits this tick.
#[must_use]
pub fn should_deposit(
    carrying: impl Borrow<Carrying>,
    pos: impl ShrinePos,
    shrine: impl ShrinePos,
    now: i64,
) -> bool {
    should_deposit_with_grace(carrying, pos, shrine, now, DEPOSIT_GRACE_MS)
}

#[must_use]
pub fn should_deposit_with_grace(
    carrying: impl Borrow<Carrying>,
    pos: impl ShrinePos,
    shrine: impl ShrinePos,
    now: i64,
    grace_ms: i64,
) -> bool {
    is_at_shrine(pos, shrine) || should_force_deposit_with_grace(carrying, now, grace_ms)
}

fn js_max(left: f64, right: f64) -> f64 {
    if left.is_nan() || right.is_nan() {
        f64::NAN
    } else if left >= right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use crate::entities::{Carrying, CarryingKind, MapType, Position};

    use super::{
        DEPOSIT_GRACE_MS, DEPOSIT_RADIUS, is_at_shrine, should_deposit, should_deposit_with_grace,
        should_force_deposit, should_force_deposit_with_grace,
    };

    fn carrying(job_ended_at: i64) -> Carrying {
        Carrying {
            kind: CarryingKind::Food,
            amount: 8.0,
            job_ended_at,
            source_gather_spot: None,
        }
    }

    fn pos(x: f64, y: f64) -> Position {
        Position {
            map: MapType::Colony,
            x,
            y,
        }
    }

    #[test]
    fn constants_match_ts() {
        assert_eq!(DEPOSIT_GRACE_MS, 60_000);
        assert_eq!(DEPOSIT_RADIUS, 1.0);
    }

    #[test]
    fn is_at_shrine_uses_chebyshev_radius_inclusively() {
        let shrine = pos(10.0, -4.0);

        assert!(is_at_shrine(pos(10.0, -4.0), shrine));
        assert!(is_at_shrine(pos(11.0, -5.0), shrine));
        assert!(is_at_shrine(pos(9.0, -3.0), shrine));
        assert!(!is_at_shrine(pos(12.0, -4.0), shrine));
        assert!(!is_at_shrine(pos(10.0, -6.0), shrine));
        assert!(!is_at_shrine(pos(11.25, -5.0), shrine));
    }

    #[test]
    fn force_deposit_triggers_at_grace_boundary() {
        let carrying = carrying(1_000);

        assert!(!should_force_deposit(&carrying, 60_999));
        assert!(should_force_deposit(&carrying, 61_000));
        assert!(!should_force_deposit_with_grace(&carrying, 1_499, 500));
        assert!(should_force_deposit_with_grace(&carrying, 1_500, 500));
    }

    #[test]
    fn should_deposit_when_at_shrine_or_after_grace() {
        let carrying = carrying(1_000);
        let shrine = pos(0.0, 0.0);

        assert!(should_deposit(&carrying, pos(1.0, 1.0), shrine, 1_000));
        assert!(!should_deposit(&carrying, pos(2.0, 0.0), shrine, 60_999));
        assert!(should_deposit(&carrying, pos(2.0, 0.0), shrine, 61_000));
        assert!(should_deposit_with_grace(
            &carrying,
            pos(2.0, 0.0),
            shrine,
            1_250,
            250
        ));
    }

    #[test]
    fn nan_coordinate_is_never_at_shrine_like_math_max() {
        assert!(!is_at_shrine(pos(f64::NAN, 0.0), pos(0.0, 0.0)));
        assert!(!is_at_shrine(pos(0.0, f64::NAN), pos(0.0, 0.0)));
    }
}
