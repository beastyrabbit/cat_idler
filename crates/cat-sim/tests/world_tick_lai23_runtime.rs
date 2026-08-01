use cat_sim::{
    leader_ai_runtime::LeaderAiRuntimeState,
    officers::OfficerRole,
    types::{JobKind, JobStatus, TaskType},
    world_tick::{JobRequester, JobRuntime, WorldState, found_colony, new_world, world_tick},
};
use serde_json::Value;

fn first_json_differences(left: &Value, right: &Value, path: &str, output: &mut Vec<String>) {
    if output.len() >= 24 {
        return;
    }
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            let keys = left
                .keys()
                .chain(right.keys())
                .collect::<std::collections::BTreeSet<_>>();
            for key in keys {
                let child_path = format!("{path}.{key}");
                match (left.get(key), right.get(key)) {
                    (Some(left), Some(right)) => {
                        first_json_differences(left, right, &child_path, output)
                    }
                    (left, right) => output.push(format!("{child_path}: {left:?} != {right:?}")),
                }
            }
        }
        (Value::Array(left), Value::Array(right)) => {
            for index in 0..left.len().max(right.len()) {
                let child_path = format!("{path}[{index}]");
                match (left.get(index), right.get(index)) {
                    (Some(left), Some(right)) => {
                        first_json_differences(left, right, &child_path, output)
                    }
                    (left, right) => output.push(format!("{child_path}: {left:?} != {right:?}")),
                }
            }
        }
        _ if left != right => output.push(format!("{path}: {left:?} != {right:?}")),
        _ => {}
    }
}

fn assert_runtime_eq(left: &LeaderAiRuntimeState, right: &LeaderAiRuntimeState) {
    if left == right {
        return;
    }
    let left = serde_json::to_value(left).expect("left runtime JSON");
    let right = serde_json::to_value(right).expect("right runtime JSON");
    let mut differences = Vec::new();
    first_json_differences(&left, &right, "$", &mut differences);
    panic!(
        "leader runtime divergence (first {} fields):\n{}",
        differences.len(),
        differences.join("\n")
    );
}

fn founding_world() -> WorldState {
    let mut world = new_world(42);
    world.colonies.push(found_colony(42, "colony-1", 0, 7));
    world
}

#[test]
fn first_authoritative_tick_binds_runtime_cats_leader_and_exact_offices() {
    let mut world = founding_world();
    let report = world_tick(&mut world, 1_000);
    let colony = &world.colonies[0];

    assert_eq!(report.len(), 1);
    assert!(!report[0].skipped);
    assert!(colony.leader_ai_restart_validated);
    assert_eq!(colony.leader_ai_runtime.cats.len(), colony.cats.len());
    assert_eq!(OfficerRole::ALL.len(), 7);
    assert_eq!(
        colony
            .leader_ai_runtime
            .officers
            .institution
            .leader()
            .map(|leader| leader.as_str()),
        colony
            .leader_id
            .as_deref()
            .map(|leader| { cat_sim::planner_core::PlannerId::derive("cat", [leader]) })
            .as_ref()
            .map(|leader| leader.as_str()),
    );
    colony.leader_ai_runtime.validate().unwrap();
}

#[test]
fn restart_revalidation_and_tick_partition_keep_runtime_identical() {
    let mut uninterrupted = founding_world();
    let _ = world_tick(&mut uninterrupted, 1_000);

    let mut restarted = uninterrupted.clone();
    let encoded = serde_json::to_string(&restarted.colonies[0].leader_ai_runtime).unwrap();
    restarted.colonies[0].leader_ai_runtime =
        serde_json::from_str::<LeaderAiRuntimeState>(&encoded).unwrap();
    restarted.colonies[0].leader_ai_restart_validated = false;

    let _ = world_tick(&mut uninterrupted, 61_000);
    for now_ms in 2_000..=61_000 {
        let _ = world_tick(&mut restarted, now_ms);
    }

    assert_runtime_eq(
        &restarted.colonies[0].leader_ai_runtime,
        &uninterrupted.colonies[0].leader_ai_runtime,
    );
    assert!(restarted.colonies[0].leader_ai_restart_validated);
}

#[test]
fn cat_input_permutation_keeps_emitted_runtime_identical() {
    let mut ordered = founding_world();
    let mut permuted = ordered.clone();
    permuted.colonies[0].cats.reverse();

    let _ = world_tick(&mut ordered, 1_000);
    let _ = world_tick(&mut permuted, 1_000);

    assert_eq!(
        permuted.colonies[0].leader_ai_runtime,
        ordered.colonies[0].leader_ai_runtime
    );
}

#[test]
fn leader_death_opens_durable_succession_without_immediate_replacement() {
    let mut world = founding_world();
    let _ = world_tick(&mut world, 1_000);
    let leader_id = world.colonies[0]
        .leader_id
        .clone()
        .expect("founding leader");
    world.colonies[0]
        .cats
        .iter_mut()
        .find(|cat| cat.id == leader_id)
        .expect("leader cat")
        .death_time = Some(1_500);

    let _ = world_tick(&mut world, 2_000);
    let colony = &world.colonies[0];
    assert_eq!(colony.leader_id, None);
    assert_eq!(colony.leader_ai_runtime.officers.institution.leader(), None);
    assert!(
        !colony
            .leader_ai_runtime
            .officers
            .institution
            .leader_succession_due(359)
    );
    assert!(
        colony
            .leader_ai_runtime
            .officers
            .institution
            .leader_succession_due(360)
    );
}

#[test]
fn review_boundary_is_idempotent_and_legacy_planner_jobs_retire_once() {
    let mut world = founding_world();
    let assigned_cat = world.colonies[0].cats[0].id.clone();
    world.colonies[0].cats[0].current_task = Some(TaskType::Hunt);
    world.colonies[0].jobs.extend([
        JobRuntime {
            id: "legacy-hunt".to_owned(),
            kind: JobKind::LeaderPlanHunt,
            status: JobStatus::Active,
            requested_by: JobRequester::Leader,
            assigned_cat: Some(assigned_cat),
            ..JobRuntime::default()
        },
        JobRuntime {
            id: "legacy-offering".to_owned(),
            kind: JobKind::CarryOffering,
            status: JobStatus::Queued,
            requested_by: JobRequester::Leader,
            ..JobRuntime::default()
        },
    ]);

    let _ = world_tick(&mut world, 1_000);
    let first_epoch = world.colonies[0].leader_ai_runtime.planner.planning_epoch;
    let first_favor = world.colonies[0]
        .leader_ai_runtime
        .shrine_favor
        .favor
        .clone();
    assert!(
        world.colonies[0]
            .jobs
            .iter()
            .filter(|job| job.id.starts_with("legacy-"))
            .all(|job| job.status == JobStatus::Cancelled)
    );

    let _ = world_tick(&mut world, 2_000);
    assert_eq!(
        world.colonies[0].leader_ai_runtime.planner.planning_epoch,
        first_epoch
    );
    assert_eq!(
        world.colonies[0].leader_ai_runtime.shrine_favor.favor,
        first_favor
    );
    assert!(
        world.colonies[0]
            .jobs
            .iter()
            .filter(|job| job.id.starts_with("legacy-"))
            .all(|job| job.status == JobStatus::Cancelled)
    );
}
