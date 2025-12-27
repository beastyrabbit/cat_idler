"use client";

import { useQuery } from "convex/react";
import { api } from "@/convex/_generated/api";
import { Id } from "@/convex/_generated/dataModel";
import { ColonyView } from "@/components/colony/ColonyView";
import { ColonyDeathSummary } from "@/components/colony/ColonyDeathSummary";
import { use } from "react";

export default function ColonyPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = use(params);
  const colonyId = id as Id<"colonies">;
  const colony = useQuery(api.colonies.getColony, { colonyId });

  if (colony === undefined) {
    return <div className="p-8">Loading colony...</div>;
  }

  if (!colony) {
    return <div className="p-8">Colony not found</div>;
  }

  if (colony.status === "dead") {
    return <ColonyDeathSummary colonyId={colonyId} />;
  }

  return <ColonyView colonyId={colonyId} />;
}



