//! Stable authority actors, domains, operations, and deterministic decisions specified by
//! `docs/leader-ai-overhaul/planner-and-beliefs.md`.

use serde::{Deserialize, Serialize};

use crate::{officers::OfficerRole, planner_core::PlannerId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityDomain {
    Survival,
    Evacuation,
    Stewardship,
    Accounting,
    Forestry,
    Farming,
    Defense,
    Research,
    Textiles,
    Building,
    Diplomacy,
    Trade,
    ColonyWide,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthorityActor {
    God {
        player_id: PlannerId,
    },
    Leader {
        cat_id: PlannerId,
    },
    Officer {
        cat_id: PlannerId,
        role: OfficerRole,
    },
    ActingSteward {
        cat_id: PlannerId,
    },
    Cat {
        cat_id: PlannerId,
    },
    Scheduler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityOperation {
    ProposeIntent,
    ApproveIntent,
    ReprioritizeIntent,
    CancelIntent,
    RetryIntent,
    SubmitOfficerRequest,
    DecideOfficerRequest,
    ExecuteApprovedIntent,
    AcceptWork,
    RefuseWork,
    PlayerNudge,
    MaintainStandingOrder,
    PurchaseResearch,
    ActivateBoost,
    ApproveDiplomacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthorityContext {
    pub leader_present: bool,
    pub player_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityDecision {
    Allowed,
    Denied(AuthorityDenial),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityDenial {
    OutsideDomain,
    LeaderRequired,
    LeaderStillPresent,
    PlayerNotAuthorized,
    StrategyForbidden,
}

#[must_use]
pub fn decide_authority(
    actor: &AuthorityActor,
    operation: AuthorityOperation,
    domain: AuthorityDomain,
    context: AuthorityContext,
) -> AuthorityDecision {
    use AuthorityDecision::{Allowed, Denied};
    use AuthorityDenial::{
        LeaderRequired, LeaderStillPresent, OutsideDomain, PlayerNotAuthorized, StrategyForbidden,
    };
    use AuthorityOperation::*;

    match actor {
        AuthorityActor::Leader { .. } => match operation {
            ProposeIntent | ApproveIntent | ReprioritizeIntent | CancelIntent | RetryIntent
            | DecideOfficerRequest => Allowed,
            _ => Denied(StrategyForbidden),
        },
        AuthorityActor::Officer { role, .. } => match operation {
            ProposeIntent if officer_owns_domain(*role, domain) => Allowed,
            SubmitOfficerRequest if officer_owns_domain(*role, domain) => Allowed,
            DecideOfficerRequest if officer_owns_domain(*role, domain) => Allowed,
            _ => Denied(OutsideDomain),
        },
        AuthorityActor::ActingSteward { .. } => {
            if context.leader_present {
                Denied(LeaderStillPresent)
            } else if matches!(
                domain,
                AuthorityDomain::Survival | AuthorityDomain::Evacuation
            ) && matches!(
                operation,
                ProposeIntent | ApproveIntent | CancelIntent | RetryIntent
            ) {
                Allowed
            } else if matches!(
                operation,
                ProposeIntent | ApproveIntent | CancelIntent | RetryIntent
            ) {
                Denied(OutsideDomain)
            } else {
                Denied(StrategyForbidden)
            }
        }
        AuthorityActor::Scheduler => match operation {
            ExecuteApprovedIntent => Allowed,
            _ => Denied(StrategyForbidden),
        },
        AuthorityActor::Cat { .. } => match operation {
            AcceptWork | RefuseWork => Allowed,
            _ => Denied(StrategyForbidden),
        },
        AuthorityActor::God { .. } => {
            if !context.player_authorized {
                Denied(PlayerNotAuthorized)
            } else if matches!(
                operation,
                PlayerNudge
                    | MaintainStandingOrder
                    | PurchaseResearch
                    | ActivateBoost
                    | ApproveDiplomacy
            ) {
                Allowed
            } else if matches!(operation, ApproveIntent | CancelIntent | RetryIntent) {
                Denied(LeaderRequired)
            } else {
                Denied(StrategyForbidden)
            }
        }
    }
}

#[must_use]
pub fn officer_owns_domain(role: OfficerRole, domain: AuthorityDomain) -> bool {
    match role {
        OfficerRole::Steward => matches!(
            domain,
            AuthorityDomain::Survival
                | AuthorityDomain::Evacuation
                | AuthorityDomain::Stewardship
                | AuthorityDomain::Building
        ),
        OfficerRole::Accountant => domain == AuthorityDomain::Accounting,
        OfficerRole::Forester => domain == AuthorityDomain::Forestry,
        OfficerRole::Farmer => {
            matches!(domain, AuthorityDomain::Farming | AuthorityDomain::Survival)
        }
        OfficerRole::Captain => domain == AuthorityDomain::Defense,
        OfficerRole::Loremaster => domain == AuthorityDomain::Research,
        OfficerRole::ClothLeader => domain == AuthorityDomain::Textiles,
    }
}
