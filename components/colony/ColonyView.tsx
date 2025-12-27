"use client";

import { useQuery } from "convex/react";
import { api } from "@/convex/_generated/api";
import { Id } from "@/convex/_generated/dataModel";
import { ResourceBar } from "@/components/ui/ResourceBar";
import { TaskQueue } from "@/components/colony/TaskQueue";
import { ColonyGrid } from "@/components/colony/ColonyGrid";
import { UserActions } from "@/components/colony/UserActions";
import { EventLog } from "@/components/colony/EventLog";
import { BuildMenu } from "@/components/colony/BuildMenu";
import { EncounterPopup } from "@/components/colony/EncounterPopup";
import { WorldMap } from "@/components/colony/WorldMap";
import { LeaderSelection } from "@/components/colony/LeaderSelection";
import { CatsDrawer } from "@/components/colony/CatsDrawer";
import { LoadingSpinner } from "@/components/ui/LoadingSpinner";
import { ErrorDisplay } from "@/components/ui/ErrorDisplay";
import { useState, useEffect } from "react";
import { useMutation } from "convex/react";
import type { BuildingType, Encounter, Cat } from "@/types/game";
import { Card } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { PawPrint, Map, RotateCcw } from "lucide-react";

interface ColonyViewProps {
  colonyId: Id<"colonies">;
}

export function ColonyView({ colonyId }: ColonyViewProps) {
  const colony = useQuery(api.colonies.getColony, { colonyId });
  const cats = useQuery(api.cats.getAliveCats, { colonyId });
  const tasks = useQuery(api.tasks.getTasksByColony, { colonyId });
  const buildings = useQuery(api.buildings.getBuildingsByColony, { colonyId });
  const encounters = useQuery(api.encounters.getActiveEncounters, { colonyId });
  const resetWorldMap = useMutation(api.worldMap.resetWorldMap);
  const [viewMode, setViewMode] = useState<"colony" | "world">("colony");
  const [sidebarTab, setSidebarTab] = useState<"build" | "tasks" | "actions" | "events">("build");
  const [selectedBuildingType, setSelectedBuildingType] = useState<BuildingType | null>(null);
  const [closedEncounters, setClosedEncounters] = useState<Set<string>>(new Set());
  const [showLeaderSelection, setShowLeaderSelection] = useState(false);
  const [catsOpen, setCatsOpen] = useState(false);
  const [mapExpansionSize, setMapExpansionSize] = useState(3);

  const handleResetWorld = async () => {
    if (confirm("Reset world map? This will regenerate all tiles with a new seed.")) {
      await resetWorldMap({ colonyId });
    }
  };

  // Show leader selection if colony has no leader
  useEffect(() => {
    if (colony && !colony.leaderId && cats && cats.length > 0) {
      // Use setTimeout to avoid setState in effect
      const timer = setTimeout(() => {
        setShowLeaderSelection(true);
      }, 0);
      return () => clearTimeout(timer);
    }
  }, [colony, cats]);

  if (colony === undefined || cats === undefined || tasks === undefined) {
    return (
      <div className="p-8 flex items-center justify-center min-h-screen">
        <LoadingSpinner size="large" text="Loading colony..." />
      </div>
    );
  }

  if (!colony) {
    return (
      <div className="p-8">
        <ErrorDisplay error="Colony not found" />
      </div>
    );
  }


  return (
    <div className="mx-auto max-w-7xl px-6 py-8">
      {/* Header */}
      <div className="mb-5 flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
        <div>
          <div className="inline-flex items-center gap-2 rounded-full border border-[var(--card-border)] bg-white/45 px-3 py-1 text-xs font-black tracking-wide text-[var(--muted)] dark:bg-white/10">
            <PawPrint className="h-4 w-4" />
            COZY COLONY
          </div>
          <h1 className="mt-2 text-3xl font-black tracking-tight text-[var(--fg)]">{colony.name}</h1>
          <p className="mt-1 text-sm text-[var(--muted)]">
            Status: <span className="font-bold capitalize text-[var(--fg)]">{colony.status}</span>
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-3">
          <Button variant="secondary" onClick={() => setCatsOpen(true)} className="gap-2">
            Cats
            <Badge variant="secondary" className="font-black">
              {(cats ?? []).length}
            </Badge>
          </Button>

          <Tabs
            value={viewMode}
            onValueChange={(v) => {
              const next = v as "colony" | "world";
              setViewMode(next);
              if (next === "colony") {
                setSidebarTab("build");
              } else {
                setSelectedBuildingType(null);
                setSidebarTab("tasks");
              }
            }}
          >
            <TabsList>
              <TabsTrigger value="colony">Colony</TabsTrigger>
              <TabsTrigger value="world">
                <Map className="h-4 w-4" />
                World
              </TabsTrigger>
            </TabsList>
          </Tabs>
        </div>
      </div>

      {/* Resources HUD */}
      <Card className="mb-6 p-4">
        <div className="space-y-4">
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-3 lg:grid-cols-5">
            <ResourceBar value={colony.resources.food} max={100} color="green" showLabel label="Food" />
            <ResourceBar value={colony.resources.water} max={100} color="blue" showLabel label="Water" />
            <ResourceBar value={colony.resources.herbs} max={50} color="yellow" showLabel label="Herbs" />
            <ResourceBar value={colony.resources.materials} max={100} color="gray" showLabel label="Materials" />
            <ResourceBar value={colony.resources.blessings} max={100} color="purple" showLabel label="Blessings" />
          </div>
          <div className="flex gap-2 border-t border-[var(--card-border)] pt-4">
            <Button
              variant="secondary"
              size="default"
              onClick={handleResetWorld}
              className="gap-2"
              title="Reset world map (generates new seed)"
            >
              <RotateCcw className="h-4 w-4" />
              Reset World Map
            </Button>
          </div>
        </div>
      </Card>

      {/* Main Content */}
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-3">
        <div className="space-y-4 lg:col-span-2">
          {viewMode === "colony" ? (
            <ColonyGrid
              colonyId={colonyId}
              gridSize={colony.gridSize}
              buildings={buildings ?? []}
              cats={cats ?? []}
              selectedBuildingType={selectedBuildingType}
              onBuildingPlaced={() => setSelectedBuildingType(null)}
            />
          ) : (
            <WorldMap
              colonyId={colonyId}
              cats={cats ?? []}
              expansionSize={mapExpansionSize}
              onExpansionChange={setMapExpansionSize}
            />
          )}
        </div>

        <Card className="p-4">
          <Tabs
            value={sidebarTab}
            onValueChange={(v) => setSidebarTab(v as typeof sidebarTab)}
          >
            <TabsList className="w-full justify-between">
              <TabsTrigger
                value="build"
                disabled={viewMode !== "colony"}
                title={viewMode !== "colony" ? "Building is only available in colony view" : undefined}
              >
                Build
              </TabsTrigger>
              <TabsTrigger value="tasks">Tasks</TabsTrigger>
              <TabsTrigger value="actions">Actions</TabsTrigger>
              <TabsTrigger value="events">Events</TabsTrigger>
            </TabsList>
          </Tabs>

          <div className="mt-4">
            {sidebarTab === "build" && viewMode === "colony" && (
              <BuildMenu colonyId={colonyId} selectedType={selectedBuildingType} onSelectType={setSelectedBuildingType} />
            )}
            {sidebarTab === "tasks" && <TaskQueue colonyId={colonyId} tasks={tasks ?? []} cats={cats ?? []} />}
            {sidebarTab === "actions" && <UserActions colonyId={colonyId} colony={colony} />}
            {sidebarTab === "events" && <EventLog colonyId={colonyId} />}
          </div>
        </Card>
      </div>

      <CatsDrawer open={catsOpen} onClose={() => setCatsOpen(false)} cats={cats ?? []} />

      {/* Encounter Popups */}
      {encounters
        ?.filter((e: Encounter) => !e.resolved && !closedEncounters.has(e._id))
        .map((encounter: Encounter) => {
          const encounterCat = cats?.find((c: Cat) => c._id === encounter.catId);
          if (!encounterCat) return null;

          return (
            <EncounterPopup
              key={encounter._id}
              encounter={encounter}
              cat={encounterCat}
              onClose={() => setClosedEncounters((prev) => new Set(prev).add(encounter._id))}
            />
          );
        })}

      {/* Leader Selection */}
      {showLeaderSelection && (
        <LeaderSelection
          colonyId={colonyId}
          onLeaderSelected={() => setShowLeaderSelection(false)}
        />
      )}
    </div>
  );
}



