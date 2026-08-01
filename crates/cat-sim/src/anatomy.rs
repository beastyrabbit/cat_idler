//! Cat anatomy, functional capability, and treatment contracts specified by
//! `docs/leader-ai-overhaul/cats-and-care.md`.

use serde::{Deserialize, Deserializer, Serialize};

pub const BASIS_POINTS_FULL_FUNCTION: u16 = 10_000;
pub const MINOR_TREATMENT_MINUTES: u32 = 12 * 60;
pub const SEVERE_TREATMENT_MINUTES: u32 = 48 * 60;

/// Stable semantic order for every tracked cat body part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyPart {
    FrontLeftPaw,
    FrontRightPaw,
    HindLeftPaw,
    HindRightPaw,
    LeftEye,
    RightEye,
    Tail,
}

impl BodyPart {
    pub const ALL: [Self; 7] = [
        Self::FrontLeftPaw,
        Self::FrontRightPaw,
        Self::HindLeftPaw,
        Self::HindRightPaw,
        Self::LeftEye,
        Self::RightEye,
        Self::Tail,
    ];

    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::FrontLeftPaw => "front_left_paw",
            Self::FrontRightPaw => "front_right_paw",
            Self::HindLeftPaw => "hind_left_paw",
            Self::HindRightPaw => "hind_right_paw",
            Self::LeftEye => "left_eye",
            Self::RightEye => "right_eye",
            Self::Tail => "tail",
        }
    }

    #[must_use]
    pub const fn is_paw(self) -> bool {
        matches!(
            self,
            Self::FrontLeftPaw | Self::FrontRightPaw | Self::HindLeftPaw | Self::HindRightPaw
        )
    }

    #[must_use]
    pub const fn is_eye(self) -> bool {
        matches!(self, Self::LeftEye | Self::RightEye)
    }
}

/// Natural function before the later prosthetic-restoration layer.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum BodyPartCondition {
    #[default]
    Healthy,
    Minor,
    Severe,
    Missing,
}

impl BodyPartCondition {
    #[must_use]
    pub const fn function_basis_points(self) -> u16 {
        match self {
            Self::Healthy => 10_000,
            Self::Minor => 8_500,
            Self::Severe => 5_000,
            Self::Missing => 0,
        }
    }

    #[must_use]
    pub const fn treatment_minutes_required(self) -> Option<u32> {
        match self {
            Self::Minor => Some(MINOR_TREATMENT_MINUTES),
            Self::Severe => Some(SEVERE_TREATMENT_MINUTES),
            Self::Healthy | Self::Missing => None,
        }
    }

    #[must_use]
    pub const fn blocks_hazardous_work(self) -> bool {
        matches!(self, Self::Severe | Self::Missing)
    }
}

/// Natural condition and effective treatment work for one part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BodyPartState {
    #[serde(default)]
    pub condition: BodyPartCondition,
    #[serde(default)]
    pub treatment_minutes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub injury_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub injured_at_tick: Option<u64>,
}

impl Default for BodyPartState {
    fn default() -> Self {
        Self {
            condition: BodyPartCondition::Healthy,
            treatment_minutes: 0,
            injury_id: None,
            injured_at_tick: None,
        }
    }
}

impl<'de> Deserialize<'de> for BodyPartState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Fields {
            #[serde(default)]
            condition: BodyPartCondition,
            #[serde(default)]
            treatment_minutes: u32,
            #[serde(default)]
            injury_id: Option<String>,
            #[serde(default)]
            injured_at_tick: Option<u64>,
        }

        let fields = Fields::deserialize(deserializer)?;
        match fields.condition.treatment_minutes_required() {
            Some(required) if fields.treatment_minutes < required => Ok(Self {
                condition: fields.condition,
                treatment_minutes: fields.treatment_minutes,
                injury_id: fields.injury_id,
                injured_at_tick: fields.injured_at_tick,
            }),
            Some(required) => Err(serde::de::Error::custom(format_args!(
                "completed treatment must transition the part to healthy; progress {}/{}",
                fields.treatment_minutes, required
            ))),
            None if fields.treatment_minutes == 0 => Ok(Self {
                condition: fields.condition,
                treatment_minutes: 0,
                injury_id: fields.injury_id,
                injured_at_tick: fields.injured_at_tick,
            }),
            None => Err(serde::de::Error::custom(
                "healthy and missing parts cannot retain treatment progress",
            )),
        }
    }
}

impl BodyPartState {
    /// An injury never improves a part; a same-severity reinjury resets treatment.
    pub fn injure(&mut self, condition: BodyPartCondition) {
        if condition >= self.condition && condition != BodyPartCondition::Healthy {
            self.condition = condition;
            self.treatment_minutes = 0;
        }
    }

    /// Persist the causal incident identity alongside the condition.
    pub fn record_incident(&mut self, incident_id: impl Into<String>, completed_tick: u64) {
        self.injury_id = Some(incident_id.into());
        self.injured_at_tick = Some(completed_tick);
    }

    pub fn treat(&mut self, effective_minutes: u32) -> TreatmentTransition {
        let Some(required) = self.condition.treatment_minutes_required() else {
            self.treatment_minutes = 0;
            return TreatmentTransition::NotTreatable;
        };
        self.treatment_minutes = self.treatment_minutes.saturating_add(effective_minutes);
        if self.treatment_minutes < required {
            return TreatmentTransition::InProgress;
        }
        self.condition = BodyPartCondition::Healthy;
        self.treatment_minutes = 0;
        TreatmentTransition::Healed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreatmentTransition {
    InProgress,
    Healed,
    NotTreatable,
}

/// Legacy cats decode as wholly healthy; omitted additive fields default per part.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CatAnatomy {
    pub front_left_paw: BodyPartState,
    pub front_right_paw: BodyPartState,
    pub hind_left_paw: BodyPartState,
    pub hind_right_paw: BodyPartState,
    pub left_eye: BodyPartState,
    pub right_eye: BodyPartState,
    pub tail: BodyPartState,
}

impl CatAnatomy {
    #[must_use]
    pub const fn part(&self, part: BodyPart) -> &BodyPartState {
        match part {
            BodyPart::FrontLeftPaw => &self.front_left_paw,
            BodyPart::FrontRightPaw => &self.front_right_paw,
            BodyPart::HindLeftPaw => &self.hind_left_paw,
            BodyPart::HindRightPaw => &self.hind_right_paw,
            BodyPart::LeftEye => &self.left_eye,
            BodyPart::RightEye => &self.right_eye,
            BodyPart::Tail => &self.tail,
        }
    }

    pub const fn part_mut(&mut self, part: BodyPart) -> &mut BodyPartState {
        match part {
            BodyPart::FrontLeftPaw => &mut self.front_left_paw,
            BodyPart::FrontRightPaw => &mut self.front_right_paw,
            BodyPart::HindLeftPaw => &mut self.hind_left_paw,
            BodyPart::HindRightPaw => &mut self.hind_right_paw,
            BodyPart::LeftEye => &mut self.left_eye,
            BodyPart::RightEye => &mut self.right_eye,
            BodyPart::Tail => &mut self.tail,
        }
    }

    pub fn injure(&mut self, part: BodyPart, condition: BodyPartCondition) {
        self.part_mut(part).injure(condition);
    }

    pub fn treat(&mut self, part: BodyPart, effective_minutes: u32) -> TreatmentTransition {
        self.part_mut(part).treat(effective_minutes)
    }

    #[must_use]
    pub fn paw_function_basis_points(&self) -> u16 {
        average_function(self, &BodyPart::ALL[..4])
    }

    #[must_use]
    pub fn eye_function_basis_points(&self) -> u16 {
        average_function(self, &BodyPart::ALL[4..6])
    }

    #[must_use]
    pub fn physical_labor_function_basis_points(&self) -> u16 {
        self.paw_function_basis_points()
    }

    #[must_use]
    pub fn vision_function_basis_points(&self) -> u16 {
        self.eye_function_basis_points()
    }

    #[must_use]
    pub fn scouting_function_basis_points(&self) -> u16 {
        self.eye_function_basis_points()
    }

    #[must_use]
    pub fn hunting_function_basis_points(&self) -> u16 {
        self.eye_function_basis_points()
    }

    #[must_use]
    pub const fn tail_function_basis_points(&self) -> u16 {
        self.tail.condition.function_basis_points()
    }

    /// Paws supply 90% and tail balance supplies the documented remaining 10%.
    #[must_use]
    pub fn movement_function_basis_points(&self) -> u16 {
        weighted_balance_function(
            self.paw_function_basis_points(),
            self.tail_function_basis_points(),
        )
    }

    /// Physical combat uses paw function plus the tail's documented 10% balance share.
    #[must_use]
    pub fn combat_function_basis_points(&self) -> u16 {
        weighted_balance_function(
            self.paw_function_basis_points(),
            self.tail_function_basis_points(),
        )
    }

    /// Ranged combat uses sight plus the tail's documented 10% balance share.
    #[must_use]
    pub fn ranged_combat_function_basis_points(&self) -> u16 {
        weighted_balance_function(
            self.eye_function_basis_points(),
            self.tail_function_basis_points(),
        )
    }

    #[must_use]
    pub fn job_capability(&self, job: HazardousJob) -> JobCapability {
        let movement = self.movement_function_basis_points();
        let task_function = match job {
            HazardousJob::Scout => self.scouting_function_basis_points(),
            HazardousJob::Hunt => self.hunting_function_basis_points(),
            HazardousJob::Quarry | HazardousJob::Logging | HazardousJob::Construction => {
                self.physical_labor_function_basis_points()
            }
            HazardousJob::Raid => self.combat_function_basis_points(),
        };
        JobCapability {
            movement_function_basis_points: movement,
            task_function_basis_points: task_function,
            effective_function_basis_points: movement.min(task_function),
            blocked: self.capability_block(job),
        }
    }

    fn capability_block(&self, job: HazardousJob) -> Option<CapabilityBlock> {
        if BodyPart::ALL[..4]
            .iter()
            .any(|part| self.part(*part).condition.blocks_hazardous_work())
        {
            return Some(CapabilityBlock::Paw);
        }
        if matches!(
            job,
            HazardousJob::Scout | HazardousJob::Hunt | HazardousJob::Raid
        ) && BodyPart::ALL[4..6]
            .iter()
            .any(|part| self.part(*part).condition.blocks_hazardous_work())
        {
            return Some(CapabilityBlock::Eye);
        }
        if self.tail.condition.blocks_hazardous_work() {
            return Some(CapabilityBlock::Tail);
        }
        None
    }
}

fn average_function(anatomy: &CatAnatomy, parts: &[BodyPart]) -> u16 {
    let sum = parts
        .iter()
        .map(|part| u32::from(anatomy.part(*part).condition.function_basis_points()))
        .sum::<u32>();
    (sum / parts.len() as u32) as u16
}

fn weighted_balance_function(primary: u16, tail: u16) -> u16 {
    ((u32::from(primary) * 90 + u32::from(tail) * 10) / 100) as u16
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HazardousJob {
    Scout,
    Hunt,
    Quarry,
    Logging,
    Construction,
    Raid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityBlock {
    Paw,
    Eye,
    Tail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobCapability {
    pub movement_function_basis_points: u16,
    pub task_function_basis_points: u16,
    pub effective_function_basis_points: u16,
    pub blocked: Option<CapabilityBlock>,
}
