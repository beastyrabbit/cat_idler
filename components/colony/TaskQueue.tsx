"use client";

import { useQuery } from "convex/react";
import { api } from "@/convex/_generated/api";
import { Id } from "@/convex/_generated/dataModel";
import type { Task, Cat } from "@/types/game";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { cn } from "@/lib/utils";

interface TaskQueueProps {
  colonyId: Id<"colonies">;
  tasks: Task[];
  cats: Cat[];
}

export function TaskQueue({ colonyId, tasks, cats }: TaskQueueProps) {
  const colony = useQuery(api.colonies.getColony, { colonyId });

  const getTaskIcon = (type: Task["type"]) => {
    return `/images/ui/tasks/${type}.png`;
  };

  const getAssignedCat = (task: Task) => {
    if (!task.assignedCatId) return null;
    return cats.find((c) => c._id === task.assignedCatId) ?? null;
  };

  return (
    <Card className="bg-white/55 dark:bg-slate-950/25">
      <CardHeader className="p-4 pb-2">
        <div className="flex items-start justify-between gap-3">
          <CardTitle>Task Queue</CardTitle>
          {colony?.name ? <Badge variant="secondary">{colony.name}</Badge> : null}
        </div>
      </CardHeader>
      <CardContent className="p-4 pt-0">
        <div className="space-y-2">
          {tasks.length === 0 ? (
            <p className="text-sm text-[var(--muted)]">No tasks</p>
          ) : (
            tasks.map((task) => {
              const assignedCat = getAssignedCat(task);
              const progress = task.progress;
              const countdown = task.assignmentCountdown;

              return (
                <div
                  key={task._id}
                  className="rounded-[20px] border border-[var(--card-border)] bg-white/70 p-3 shadow-sm dark:bg-slate-950/25"
                >
                  <div className="flex items-center justify-between gap-3">
                    <div className="flex min-w-0 items-center gap-2">
                      <div className="grid h-9 w-9 place-items-center rounded-2xl bg-black/5 text-lg dark:bg-white/10 overflow-hidden">
                        <img src={getTaskIcon(task.type)} alt="" className="w-6 h-6 object-contain" />
                      </div>
                      <div className="min-w-0">
                        <div className="truncate font-extrabold text-[var(--fg)]">
                          {task.type.replace("_", " ")}
                        </div>
                        <div className="text-xs font-semibold text-[var(--muted)]">
                          {assignedCat ? `→ ${assignedCat.name}` : `Pending${countdown > 0 ? ` (${countdown}s)` : ""}`}
                        </div>
                      </div>
                    </div>

                    {!task.isOptimalAssignment && assignedCat ? (
                      <Badge variant="warning">⚠️ suboptimal</Badge>
                    ) : progress >= 100 ? (
                      <Badge variant="success">done</Badge>
                    ) : (
                      <Badge variant="secondary" className={cn(progress > 0 ? "" : "opacity-80")}>
                        {Math.round(progress)}%
                      </Badge>
                    )}
                  </div>

                  <div className="mt-3">
                    <Progress
                      value={Math.min(100, Math.max(0, progress))}
                      className="h-3"
                      indicatorClassName="bg-sky-500"
                      indicatorTestId="task-progress"
                    />
                  </div>
                </div>
              );
            })
          )}
        </div>
      </CardContent>
    </Card>
  );
}



