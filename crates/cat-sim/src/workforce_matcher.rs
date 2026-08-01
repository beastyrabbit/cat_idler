//! Deterministic colony-wide workforce matching specified by
//! `docs/leader-ai-overhaul/cats-and-care.md`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    cat_willingness::{
        AssignmentAvailability, RefusalBucket, TaskPriority, WillingnessDecision, WorkerCandidate,
        assignment_availability,
    },
    planner_core::PlannerScore,
    planner_core::{PlannerRngStream, keyed_planner_seed},
    scheduler::{HYSTERESIS_BASIS_POINTS, PreemptionCause, should_preempt},
};

pub const WORKFORCE_MATCHER_SCHEMA_VERSION: u32 = 1;
pub const ORDINARY_PREEMPTION_BASIS_POINTS: i64 = HYSTERESIS_BASIS_POINTS;

/// Every task position is a distinct node, including distinct station slots
/// that happen to share one complete building objective.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkforceSlot {
    pub task_id: String,
    pub slot_id: String,
    pub priority: TaskPriority,
}

/// A fully evaluated cat/slot edge. Capability layers calculate `score`
/// before this global pass; refusal remains explicit and auditable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkforceEdge {
    pub cat_id: String,
    pub task_id: String,
    pub slot_id: String,
    pub score: i64,
    pub eligible: bool,
    pub willingness: WillingnessDecision,
    pub current_assignment: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkforceAssignment {
    pub task_id: String,
    pub slot_id: String,
    pub cat_id: String,
    pub score: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkforceMatch {
    pub assignments: Vec<WorkforceAssignment>,
    pub availability_by_slot: BTreeMap<(String, String), AssignmentAvailability>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RematchTrigger {
    PlanningCadence,
    Emergency,
    Death,
    Refusal,
    Injury,
    Recovery,
    TaskFinished,
    TaskBlocked,
    SiteLost,
    RouteChanged,
    DestinationChanged,
    PlayerDirectionAccepted,
}

impl RematchTrigger {
    #[must_use]
    pub const fn bypasses_hysteresis(self) -> bool {
        matches!(
            self,
            Self::Emergency
                | Self::Death
                | Self::Refusal
                | Self::Injury
                | Self::TaskBlocked
                | Self::SiteLost
                | Self::RouteChanged
                | Self::DestinationChanged
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkforceError {
    EmptyStableId,
    DuplicateSlot,
    DuplicateEdge,
    MultipleCurrentAssignments,
    UnknownSlot,
}

/// Derive the exact refusal bucket from semantic assignment identity, never
/// from collection order or a shared RNG cursor.
#[must_use]
pub fn refusal_bucket(
    world_seed: u32,
    colony_id: &str,
    cat_id: &str,
    task_id: &str,
    assignment_occurrence: u64,
) -> RefusalBucket {
    let occurrence = assignment_occurrence.to_string();
    let seed = keyed_planner_seed(
        world_seed,
        PlannerRngStream::Refusal,
        [colony_id, cat_id, task_id, occurrence.as_str()],
    );
    RefusalBucket::try_from((seed % 10_000) as u16).expect("modulo 10,000 is a valid bucket")
}

/// Ordinary reassignment needs at least a 15% improvement over the current
/// edge. Invalid/incapable assignments and urgent triggers bypass the floor.
#[must_use]
pub fn preemption_allowed(
    current_score: i64,
    proposed_score: i64,
    trigger: RematchTrigger,
    current_assignment_valid: bool,
) -> bool {
    let cause = if !current_assignment_valid {
        PreemptionCause::WorkerIncapacitated
    } else {
        match trigger {
            RematchTrigger::Emergency => PreemptionCause::Emergency,
            RematchTrigger::SiteLost
            | RematchTrigger::RouteChanged
            | RematchTrigger::DestinationChanged
            | RematchTrigger::TaskBlocked => PreemptionCause::RouteInvalidated,
            RematchTrigger::Death | RematchTrigger::Refusal | RematchTrigger::Injury => {
                PreemptionCause::WorkerIncapacitated
            }
            RematchTrigger::PlanningCadence
            | RematchTrigger::Recovery
            | RematchTrigger::TaskFinished
            | RematchTrigger::PlayerDirectionAccepted => PreemptionCause::Ordinary,
        }
    };
    should_preempt(
        PlannerScore::new(current_score),
        PlannerScore::new(proposed_score),
        cause,
    )
}

/// Compute the maximum-total-score bipartite assignment for the whole colony.
/// Inputs are canonicalized first, so vector/map insertion order cannot choose
/// a winner. Only accepted, positive-score edges enter the flow graph.
pub fn match_workforce(
    slots: &[WorkforceSlot],
    edges: &[WorkforceEdge],
) -> Result<WorkforceMatch, WorkforceError> {
    match_workforce_for_trigger(slots, edges, RematchTrigger::PlanningCadence)
}

/// Event-driven variant whose trigger may bypass ordinary continuity
/// hysteresis after refusal, incapacity, or invalidation.
pub fn match_workforce_for_trigger(
    slots: &[WorkforceSlot],
    edges: &[WorkforceEdge],
    trigger: RematchTrigger,
) -> Result<WorkforceMatch, WorkforceError> {
    let mut canonical_slots = slots.to_vec();
    canonical_slots.sort();
    if canonical_slots
        .iter()
        .any(|slot| slot.task_id.is_empty() || slot.slot_id.is_empty())
    {
        return Err(WorkforceError::EmptyStableId);
    }
    if canonical_slots.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(WorkforceError::DuplicateSlot);
    }

    let slot_index = canonical_slots
        .iter()
        .enumerate()
        .map(|(index, slot)| ((slot.task_id.clone(), slot.slot_id.clone()), index))
        .collect::<BTreeMap<_, _>>();
    let mut canonical_edges = edges.to_vec();
    canonical_edges.sort_by(|left, right| {
        (&left.task_id, &left.slot_id, &left.cat_id).cmp(&(
            &right.task_id,
            &right.slot_id,
            &right.cat_id,
        ))
    });
    if canonical_edges
        .iter()
        .any(|edge| edge.cat_id.is_empty() || edge.task_id.is_empty() || edge.slot_id.is_empty())
    {
        return Err(WorkforceError::EmptyStableId);
    }
    if canonical_edges.windows(2).any(|pair| {
        (&pair[0].task_id, &pair[0].slot_id, &pair[0].cat_id)
            == (&pair[1].task_id, &pair[1].slot_id, &pair[1].cat_id)
    }) {
        return Err(WorkforceError::DuplicateEdge);
    }
    let mut current_by_cat = BTreeMap::new();
    for edge in canonical_edges
        .iter()
        .filter(|edge| edge.current_assignment)
    {
        if current_by_cat.insert(edge.cat_id.clone(), edge).is_some() {
            return Err(WorkforceError::MultipleCurrentAssignments);
        }
    }
    if canonical_edges
        .iter()
        .any(|edge| !slot_index.contains_key(&(edge.task_id.clone(), edge.slot_id.clone())))
    {
        return Err(WorkforceError::UnknownSlot);
    }

    let mut availability_by_slot = BTreeMap::new();
    for slot in &canonical_slots {
        let candidates = canonical_edges
            .iter()
            .filter(|edge| edge.task_id == slot.task_id && edge.slot_id == slot.slot_id)
            .map(|edge| WorkerCandidate {
                eligible: edge.eligible,
                decision: edge.willingness,
            })
            .collect::<Vec<_>>();
        availability_by_slot.insert(
            (slot.task_id.clone(), slot.slot_id.clone()),
            assignment_availability(&candidates),
        );
    }

    let cats = canonical_edges
        .iter()
        .map(|edge| edge.cat_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let cat_index = cats
        .iter()
        .enumerate()
        .map(|(index, id)| (id.clone(), index))
        .collect::<BTreeMap<_, _>>();

    let source = 0;
    let cat_offset = 1;
    let slot_offset = cat_offset + cats.len();
    let sink = slot_offset + canonical_slots.len();
    let mut graph = FlowGraph::new(sink + 1, canonical_slots.len());
    for cat in 0..cats.len() {
        graph.add_edge(source, cat_offset + cat, 1, 0, None);
    }
    for slot in 0..canonical_slots.len() {
        graph.add_edge(slot_offset + slot, sink, 1, 0, None);
    }
    for edge in canonical_edges.iter().filter(|edge| {
        if !edge.eligible || !edge.willingness.accepts_assignment() || edge.score <= 0 {
            return false;
        }
        let Some(current) = current_by_cat.get(&edge.cat_id) else {
            return true;
        };
        edge.current_assignment
            || preemption_allowed(
                current.score,
                edge.score,
                trigger,
                current.eligible && current.willingness.accepts_assignment() && current.score > 0,
            )
    }) {
        let cat = cat_index[&edge.cat_id];
        let slot = slot_index[&(edge.task_id.clone(), edge.slot_id.clone())];
        graph.add_edge(
            cat_offset + cat,
            slot_offset + slot,
            1,
            -i128::from(edge.score),
            Some((slot, cat as i32)),
        );
    }
    graph.augment_while_profitable(source, sink);

    let scores = canonical_edges
        .iter()
        .map(|edge| {
            (
                (
                    edge.task_id.clone(),
                    edge.slot_id.clone(),
                    edge.cat_id.clone(),
                ),
                edge.score,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut assignments = Vec::new();
    for (cat_number, cat_id) in cats.iter().enumerate() {
        for edge in &graph.edges[cat_offset + cat_number] {
            if edge.to < slot_offset || edge.to >= sink || edge.capacity != 0 {
                continue;
            }
            let slot = &canonical_slots[edge.to - slot_offset];
            assignments.push(WorkforceAssignment {
                task_id: slot.task_id.clone(),
                slot_id: slot.slot_id.clone(),
                cat_id: cat_id.clone(),
                score: scores[&(slot.task_id.clone(), slot.slot_id.clone(), cat_id.clone())],
            });
        }
    }
    assignments.sort();
    Ok(WorkforceMatch {
        assignments,
        availability_by_slot,
    })
}

#[derive(Debug, Clone, Copy)]
struct FlowEdge {
    to: usize,
    reverse: usize,
    capacity: u8,
    cost: i128,
    tie_change: Option<(usize, i32)>,
}

struct FlowGraph {
    edges: Vec<Vec<FlowEdge>>,
    tie_width: usize,
}

impl FlowGraph {
    fn new(nodes: usize, tie_width: usize) -> Self {
        Self {
            edges: vec![Vec::new(); nodes],
            tie_width,
        }
    }

    fn add_edge(
        &mut self,
        from: usize,
        to: usize,
        capacity: u8,
        cost: i128,
        tie_change: Option<(usize, i32)>,
    ) {
        let forward_reverse = self.edges[to].len();
        let backward_reverse = self.edges[from].len();
        self.edges[from].push(FlowEdge {
            to,
            reverse: forward_reverse,
            capacity,
            cost,
            tie_change,
        });
        self.edges[to].push(FlowEdge {
            to: from,
            reverse: backward_reverse,
            capacity: 0,
            cost: -cost,
            tie_change: tie_change.map(|(slot, rank)| (slot, -rank)),
        });
    }

    fn augment_while_profitable(&mut self, source: usize, sink: usize) {
        loop {
            let nodes = self.edges.len();
            let mut distance = vec![i128::MAX; nodes];
            let mut tie_distance = vec![None; nodes];
            let mut predecessor = vec![None; nodes];
            let mut queued = vec![false; nodes];
            let mut queue = VecDeque::from([source]);
            distance[source] = 0;
            tie_distance[source] = Some(vec![0; self.tie_width]);
            queued[source] = true;
            while let Some(node) = queue.pop_front() {
                queued[node] = false;
                for (edge_index, edge) in self.edges[node].iter().enumerate() {
                    if edge.capacity == 0 || distance[node] == i128::MAX {
                        continue;
                    }
                    let candidate = distance[node].saturating_add(edge.cost);
                    let mut candidate_tie = tie_distance[node]
                        .as_ref()
                        .expect("reachable node has a tie distance")
                        .clone();
                    if let Some((slot, change)) = edge.tie_change {
                        candidate_tie[slot] += change;
                    }
                    let improves = candidate < distance[edge.to]
                        || (candidate == distance[edge.to]
                            && tie_distance[edge.to]
                                .as_ref()
                                .is_none_or(|current| candidate_tie < *current));
                    if improves {
                        distance[edge.to] = candidate;
                        tie_distance[edge.to] = Some(candidate_tie);
                        predecessor[edge.to] = Some((node, edge_index));
                        if !queued[edge.to] {
                            queued[edge.to] = true;
                            queue.push_back(edge.to);
                        }
                    }
                }
            }
            if distance[sink] >= 0 || predecessor[sink].is_none() {
                break;
            }
            let mut node = sink;
            while node != source {
                let (previous, edge_index) = predecessor[node].expect("sink path is complete");
                let reverse = self.edges[previous][edge_index].reverse;
                self.edges[previous][edge_index].capacity = 0;
                self.edges[node][reverse].capacity = 1;
                node = previous;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cat_willingness::{AssignmentBlockReason, RefusalReason};

    fn slot(task: &str, slot: &str) -> WorkforceSlot {
        WorkforceSlot {
            task_id: task.to_owned(),
            slot_id: slot.to_owned(),
            priority: TaskPriority::Optional,
        }
    }

    fn edge(cat: &str, task: &str, slot: &str, score: i64) -> WorkforceEdge {
        WorkforceEdge {
            cat_id: cat.to_owned(),
            task_id: task.to_owned(),
            slot_id: slot.to_owned(),
            score,
            eligible: true,
            willingness: WillingnessDecision::Willing,
            current_assignment: false,
        }
    }

    #[test]
    fn maximum_weight_matching_beats_greedy_first_choice() {
        let slots = [slot("task-a", "one"), slot("task-b", "one")];
        let edges = [
            edge("cat-a", "task-a", "one", 100),
            edge("cat-a", "task-b", "one", 99),
            edge("cat-b", "task-a", "one", 98),
            edge("cat-b", "task-b", "one", 1),
        ];
        let result = match_workforce(&slots, &edges).unwrap();
        assert_eq!(result.assignments[0].cat_id, "cat-b");
        assert_eq!(result.assignments[1].cat_id, "cat-a");
        assert_eq!(
            result
                .assignments
                .iter()
                .map(|assignment| assignment.score)
                .sum::<i64>(),
            197
        );
    }

    #[test]
    fn input_order_and_equal_score_ties_are_stable() {
        let mut slots = vec![slot("task-b", "one"), slot("task-a", "one")];
        let mut edges = vec![
            edge("cat-b", "task-b", "one", 10),
            edge("cat-a", "task-b", "one", 10),
            edge("cat-b", "task-a", "one", 10),
            edge("cat-a", "task-a", "one", 10),
        ];
        let first = match_workforce(&slots, &edges).unwrap();
        slots.reverse();
        edges.reverse();
        let second = match_workforce(&slots, &edges).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.assignments[0].task_id, "task-a");
        assert_eq!(first.assignments[0].cat_id, "cat-a");
    }

    #[test]
    fn refusal_blocks_only_that_edge_and_reports_no_willing_worker() {
        let slots = [slot("hunt", "entrance")];
        let mut refused = edge("cat-a", "hunt", "entrance", 100);
        refused.willingness = WillingnessDecision::Refused(RefusalReason::CriticalStress);
        let result = match_workforce(&slots, &[refused]).unwrap();
        assert!(result.assignments.is_empty());
        assert_eq!(
            result.availability_by_slot[&("hunt".to_owned(), "entrance".to_owned())],
            AssignmentAvailability::Blocked(AssignmentBlockReason::NoWillingWorker)
        );
    }

    #[test]
    fn multiple_slots_never_double_assign_one_cat() {
        let result = match_workforce(
            &[slot("workshop", "bench-a"), slot("workshop", "bench-b")],
            &[
                edge("cat-a", "workshop", "bench-a", 20),
                edge("cat-a", "workshop", "bench-b", 30),
            ],
        )
        .unwrap();
        assert_eq!(result.assignments.len(), 1);
        assert_eq!(result.assignments[0].slot_id, "bench-b");
    }

    #[test]
    fn preemption_boundary_and_bypasses_are_exact() {
        assert!(!preemption_allowed(
            1_000,
            1_149,
            RematchTrigger::PlanningCadence,
            true
        ));
        assert!(preemption_allowed(
            1_000,
            1_150,
            RematchTrigger::PlanningCadence,
            true
        ));
        assert!(preemption_allowed(
            1_000,
            1_001,
            RematchTrigger::Emergency,
            true
        ));
        assert!(preemption_allowed(
            1_000,
            1_001,
            RematchTrigger::PlanningCadence,
            false
        ));
        assert!(preemption_allowed(
            1_000,
            999,
            RematchTrigger::Emergency,
            true
        ));
        assert!(preemption_allowed(
            1_000,
            1,
            RematchTrigger::PlanningCadence,
            false
        ));
    }

    #[test]
    fn ordinary_matching_keeps_continuity_until_threshold_then_rematches() {
        let slots = [slot("task-a", "one"), slot("task-b", "one")];
        let mut current = edge("cat", "task-a", "one", 1_000);
        current.current_assignment = true;
        let below = edge("cat", "task-b", "one", 1_149);
        let kept = match_workforce(&slots, &[current.clone(), below]).unwrap();
        assert_eq!(kept.assignments[0].task_id, "task-a");

        let threshold = edge("cat", "task-b", "one", 1_150);
        let changed = match_workforce(&slots, &[current.clone(), threshold]).unwrap();
        assert_eq!(changed.assignments[0].task_id, "task-b");

        current.willingness = WillingnessDecision::Refused(RefusalReason::Stress);
        let invalidated =
            match_workforce(&slots, &[current, edge("cat", "task-b", "one", 1)]).unwrap();
        assert_eq!(invalidated.assignments[0].task_id, "task-b");
    }

    #[test]
    fn refusal_bucket_is_semantic_and_bounded() {
        let first = refusal_bucket(7, "colony", "cat", "task", 3);
        assert_eq!(first, refusal_bucket(7, "colony", "cat", "task", 3));
        assert!(first.get() <= 9_999);
        assert_ne!(first, refusal_bucket(7, "colony", "cat", "task", 4));
    }

    #[test]
    fn malformed_duplicate_and_unknown_contracts_fail_closed() {
        assert_eq!(
            match_workforce(&[slot("a", "one"), slot("a", "one")], &[]),
            Err(WorkforceError::DuplicateSlot)
        );
        assert_eq!(
            match_workforce(&[slot("a", "one")], &[edge("cat", "b", "one", 1)]),
            Err(WorkforceError::UnknownSlot)
        );
    }
}
