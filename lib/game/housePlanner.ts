import type { JobKind } from "@/types/game";

export interface PlannerResources {
  water: number;
  materials: number;
}

export interface PlannerJob {
  kind: JobKind;
  metadata?: Record<string, unknown>;
}

export interface PlannedJob {
  kind: JobKind;
  metadata: Record<string, unknown>;
}

export interface HousePlanInput {
  resources: PlannerResources;
  activeOrQueuedJobs: PlannerJob[];
  waterRequired: number;
  materialsRequired: number;
}

function hasMatchingJob(
  jobs: PlannerJob[],
  kind: JobKind,
  predicate: (job: PlannerJob) => boolean,
): boolean {
  return jobs.some((job) => {
    if (job.kind !== kind) {
      return false;
    }
    return predicate(job);
  });
}

function hasMetadataValue(
  job: PlannerJob,
  key: string,
  expected: string,
): boolean {
  return typeof job.metadata?.[key] === "string" && job.metadata[key] === expected;
}

export function planHousePipeline(input: HousePlanInput): PlannedJob[] {
  const { resources, activeOrQueuedJobs, waterRequired, materialsRequired } = input;

  const planned: PlannedJob[] = [];

  const needWater = resources.water < waterRequired;
  const needMaterials = resources.materials < materialsRequired;

  const hasWaterPrep = hasMatchingJob(
    activeOrQueuedJobs,
    "supply_water",
    (job) => hasMetadataValue(job, "reason", "house_prereq"),
  );

  const hasMaterialPrep = hasMatchingJob(
    activeOrQueuedJobs,
    "build_house",
    (job) => hasMetadataValue(job, "phase", "gather_materials"),
  );

  const hasConstructJob = hasMatchingJob(
    activeOrQueuedJobs,
    "build_house",
    (job) => hasMetadataValue(job, "phase", "construct_house"),
  );

  if (needWater && !hasWaterPrep) {
    planned.push({
      kind: "supply_water",
      metadata: {
        reason: "house_prereq",
      },
    });
  }

  if (needMaterials && !hasMaterialPrep) {
    planned.push({
      kind: "build_house",
      metadata: {
        phase: "gather_materials",
        reason: "house_prereq",
      },
    });
  }

  if (!needWater && !needMaterials && !hasConstructJob) {
    planned.push({
      kind: "build_house",
      metadata: {
        phase: "construct_house",
      },
    });
  }

  return planned;
}
