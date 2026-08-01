//! Report-safe autonomous-trade valuation specified by
//! `docs/leader-ai-overhaul/diplomacy-trade.md`.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    beliefs::{BeliefProjection, Confidence, EvidenceId, ProjectedBeliefValue},
    diplomacy::DiplomacyRelationship,
    planner_core::PlannerId,
};

pub const FRIENDLY_VALUE_BOUND_BASIS_POINTS: i32 = 1_000;
pub const ALLIED_STRATEGIC_DISADVANTAGE_BASIS_POINTS: i32 = 2_000;
const BASIS_POINTS_SCALE: i128 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradePurpose {
    Ordinary,
    Survival,
    ActiveDefense,
}

impl TradePurpose {
    #[must_use]
    pub const fn is_strategic(self) -> bool {
        matches!(self, Self::Survival | Self::ActiveDefense)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradePersonality {
    Balanced,
    Mercantile,
    SelfSufficient,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeValueEvidence {
    pub belief_id: PlannerId,
    pub estimated_value: u64,
    pub lower_bound: u64,
    pub upper_bound: u64,
    pub confidence: Confidence,
    pub observed_tick: u64,
    pub expires_tick: Option<u64>,
    pub age_ticks: u64,
    pub evidence_ids: BTreeSet<EvidenceId>,
}

impl TradeValueEvidence {
    pub fn from_projection(
        projection: &BeliefProjection,
        now_tick: u64,
    ) -> Result<Self, TradeValuationError> {
        if projection.confidence == Confidence::zero()
            || projection.evidence_ids.is_empty()
            || projection.observed_tick > now_tick
            || projection
                .expires_tick
                .is_some_and(|expiry| now_tick >= expiry)
        {
            return Err(TradeValuationError::RecountRequired);
        }
        let (estimate, lower, upper) = match &projection.value {
            ProjectedBeliefValue::StockRange(range) | ProjectedBeliefValue::FlowRange(range) => {
                (range.estimate, range.lower_bound, range.upper_bound)
            }
            ProjectedBeliefValue::FlowRate(value) => (*value, *value, *value),
            ProjectedBeliefValue::RegenerationRange(_)
            | ProjectedBeliefValue::FlowTrend(_)
            | ProjectedBeliefValue::Category(_)
            | ProjectedBeliefValue::Unavailable => {
                return Err(TradeValuationError::RecountRequired);
            }
        };
        let estimated_value = u64::try_from(estimate)
            .ok()
            .filter(|value| *value > 0)
            .ok_or(TradeValuationError::InvalidValue)?;
        let lower_bound = u64::try_from(lower).map_err(|_| TradeValuationError::InvalidValue)?;
        let upper_bound = u64::try_from(upper).map_err(|_| TradeValuationError::InvalidValue)?;
        Ok(Self {
            belief_id: projection.key.stable_id(),
            estimated_value,
            lower_bound,
            upper_bound,
            confidence: projection.confidence,
            observed_tick: projection.observed_tick,
            expires_tick: projection.expires_tick,
            age_ticks: now_tick - projection.observed_tick,
            evidence_ids: projection.evidence_ids.clone(),
        })
    }

    fn validate(&self, evaluated_tick: u64) -> Result<(), TradeValuationError> {
        if self.estimated_value == 0
            || self.lower_bound > self.estimated_value
            || self.estimated_value > self.upper_bound
            || self.confidence == Confidence::zero()
            || self.evidence_ids.is_empty()
            || self.observed_tick > evaluated_tick
            || self.age_ticks != evaluated_tick - self.observed_tick
            || self
                .expires_tick
                .is_some_and(|expiry| evaluated_tick >= expiry)
        {
            return Err(TradeValuationError::MalformedEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TradeValuation {
    pub relationship: DiplomacyRelationship,
    pub purpose: TradePurpose,
    pub personality: TradePersonality,
    pub offered: TradeValueEvidence,
    pub requested: TradeValueEvidence,
    /// Positive means this party believes it gives more value than it receives.
    pub disadvantage_basis_points: i32,
    /// Ranking only. It never expands the legal valuation band.
    pub personality_preference: i32,
    pub evaluated_tick: u64,
}

impl TradeValuation {
    pub fn evaluate(
        relationship: DiplomacyRelationship,
        purpose: TradePurpose,
        personality: TradePersonality,
        offered: &BeliefProjection,
        requested: &BeliefProjection,
        now_tick: u64,
    ) -> Result<Self, TradeValuationError> {
        if !matches!(
            relationship,
            DiplomacyRelationship::Friendly | DiplomacyRelationship::Allied
        ) {
            return Err(TradeValuationError::RelationshipDenied);
        }
        let offered = TradeValueEvidence::from_projection(offered, now_tick)?;
        let requested = TradeValueEvidence::from_projection(requested, now_tick)?;
        let disadvantage_basis_points =
            signed_disadvantage_basis_points(offered.estimated_value, requested.estimated_value)?;
        let max_disadvantage =
            if relationship == DiplomacyRelationship::Allied && purpose.is_strategic() {
                ALLIED_STRATEGIC_DISADVANTAGE_BASIS_POINTS
            } else {
                FRIENDLY_VALUE_BOUND_BASIS_POINTS
            };
        if !within_relationship_bounds(
            offered.estimated_value,
            requested.estimated_value,
            max_disadvantage,
        )? {
            return Err(TradeValuationError::OutsideRelationshipBound);
        }
        let personality_preference = match personality {
            TradePersonality::Balanced => 0,
            TradePersonality::Mercantile => -disadvantage_basis_points,
            TradePersonality::SelfSufficient => -disadvantage_basis_points.abs(),
        };
        let valuation = Self {
            relationship,
            purpose,
            personality,
            offered,
            requested,
            disadvantage_basis_points,
            personality_preference,
            evaluated_tick: now_tick,
        };
        valuation.validate()?;
        Ok(valuation)
    }

    pub fn validate(&self) -> Result<(), TradeValuationError> {
        self.offered.validate(self.evaluated_tick)?;
        self.requested.validate(self.evaluated_tick)?;
        if !matches!(
            self.relationship,
            DiplomacyRelationship::Friendly | DiplomacyRelationship::Allied
        ) {
            return Err(TradeValuationError::RelationshipDenied);
        }
        let expected = signed_disadvantage_basis_points(
            self.offered.estimated_value,
            self.requested.estimated_value,
        )?;
        let max_disadvantage =
            if self.relationship == DiplomacyRelationship::Allied && self.purpose.is_strategic() {
                ALLIED_STRATEGIC_DISADVANTAGE_BASIS_POINTS
            } else {
                FRIENDLY_VALUE_BOUND_BASIS_POINTS
            };
        if self.disadvantage_basis_points != expected
            || !within_relationship_bounds(
                self.offered.estimated_value,
                self.requested.estimated_value,
                max_disadvantage,
            )?
        {
            return Err(TradeValuationError::OutsideRelationshipBound);
        }
        let expected_preference = match self.personality {
            TradePersonality::Balanced => 0,
            TradePersonality::Mercantile => -expected,
            TradePersonality::SelfSufficient => -expected.abs(),
        };
        if self.personality_preference != expected_preference {
            return Err(TradeValuationError::MalformedEvidence);
        }
        Ok(())
    }
}

fn signed_disadvantage_basis_points(
    offered: u64,
    requested: u64,
) -> Result<i32, TradeValuationError> {
    if offered == 0 {
        return Err(TradeValuationError::InvalidValue);
    }
    let difference = i128::from(offered) - i128::from(requested);
    let scaled = difference
        .checked_mul(BASIS_POINTS_SCALE)
        .ok_or(TradeValuationError::ArithmeticOverflow)?;
    let result = scaled / i128::from(offered);
    i32::try_from(result).map_err(|_| TradeValuationError::ArithmeticOverflow)
}

fn within_relationship_bounds(
    offered: u64,
    requested: u64,
    max_disadvantage_basis_points: i32,
) -> Result<bool, TradeValuationError> {
    let offered = i128::from(offered);
    let requested = i128::from(requested);
    let difference = offered - requested;
    let scaled_difference = difference
        .abs()
        .checked_mul(BASIS_POINTS_SCALE)
        .ok_or(TradeValuationError::ArithmeticOverflow)?;
    let permitted = offered
        .checked_mul(i128::from(if difference >= 0 {
            max_disadvantage_basis_points
        } else {
            FRIENDLY_VALUE_BOUND_BASIS_POINTS
        }))
        .ok_or(TradeValuationError::ArithmeticOverflow)?;
    Ok(scaled_difference <= permitted)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeValuationError {
    RelationshipDenied,
    RecountRequired,
    InvalidValue,
    OutsideRelationshipBound,
    ArithmeticOverflow,
    MalformedEvidence,
}
