"use client";

import { useQuery } from "convex/react";
import { api } from "@/convex/_generated/api";
import { Id } from "@/convex/_generated/dataModel";
import type { BuildingType, Building } from "@/types/game";
import { BUILDING_COSTS } from "@/types/game";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface BuildMenuProps {
  colonyId: Id<"colonies">;
  selectedType: BuildingType | null;
  onSelectType: (type: BuildingType | null) => void;
}

export function BuildMenu({ colonyId, selectedType, onSelectType }: BuildMenuProps) {
  const colony = useQuery(api.colonies.getColony, { colonyId });
  const buildings = useQuery(api.buildings.getBuildingsByColony, { colonyId });

  if (colony === undefined || buildings === undefined) {
    return <div>Loading...</div>;
  }

  if (!colony) {
    return <div>Colony not found</div>;
  }

  const buildingTypes: BuildingType[] = [
    "food_storage",
    "water_bowl",
    "beds",
    "herb_garden",
    "nursery",
    "elder_corner",
    "walls",
    "mouse_farm",
  ];

  const getBuildingName = (type: BuildingType) => {
    return type.replaceAll("_", " ").replace(/\b\w/g, (l) => l.toUpperCase());
  };

  const getBuildingIcon = (type: BuildingType) => {
    const icons: Record<BuildingType, string> = {
      den: "🏠",
      food_storage: "📦",
      water_bowl: "🥣",
      beds: "🛏️",
      herb_garden: "🌱",
      nursery: "🧺",
      elder_corner: "🧶",
      walls: "🧱",
      mouse_farm: "🐭",
    };
    return icons[type] ?? "🏗️";
  };

  const canAfford = (type: BuildingType) => {
    return colony.resources.materials >= BUILDING_COSTS[type];
  };


  return (
    <Card className="bg-white/55 dark:bg-slate-950/25">
      <CardHeader className="p-4 pb-2">
        <div className="flex items-start justify-between gap-3">
          <CardTitle>Buildings</CardTitle>
          <Badge variant="secondary" className="font-black">
            <span aria-hidden="true">🪵</span> Materials: {colony.resources.materials}
          </Badge>
        </div>
      </CardHeader>

      <CardContent className="p-4 pt-0">
        <div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
          {buildingTypes.map((type) => {
            const selected = selectedType === type;
            const afford = canAfford(type);
            return (
              <button
                key={type}
                onClick={() => onSelectType(selected ? null : type)}
                disabled={!afford}
                className={cn(
                  "group w-full rounded-[20px] border p-3 text-left transition-all",
                  "bg-white/70 hover:bg-white shadow-sm hover:shadow-md hover:-translate-y-0.5",
                  "dark:bg-slate-950/25 dark:hover:bg-slate-950/35",
                  selected
                    ? "border-sage-500 ring-2 ring-[var(--ring)]"
                    : "border-[var(--card-border)]",
                  !afford && "opacity-60 hover:translate-y-0 hover:shadow-sm cursor-not-allowed"
                )}
              >
                <div className="flex items-center gap-3">
                  <div className="grid h-10 w-10 place-items-center rounded-2xl bg-black/5 text-xl dark:bg-white/10">
                    {getBuildingIcon(type)}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="truncate font-extrabold text-[var(--fg)]">{getBuildingName(type)}</div>
                    <div className="mt-0.5 text-xs font-semibold text-[var(--muted)]">
                      Cost: {BUILDING_COSTS[type]} materials
                    </div>
                  </div>
                </div>
              </button>
            );
          })}
        </div>

        {selectedType && (
          <div className="mt-4 rounded-[20px] border border-sage-500/35 bg-sage-50/70 p-3 dark:bg-sage-900/15">
            <div className="text-sm font-extrabold text-[var(--fg)]">
              Click on the colony grid to place: {getBuildingName(selectedType)}
            </div>
            <div className="mt-2">
              <Button variant="secondary" size="sm" onClick={() => onSelectType(null)}>
                Cancel
              </Button>
            </div>
          </div>
        )}

        <div className="mt-5">
          <div className="text-sm font-extrabold text-[var(--fg)]">Existing buildings</div>
          {buildings && buildings.length > 0 ? (
            <div className="mt-2 space-y-2">
              {buildings.map((building: Building) => (
                <div
                  key={building._id}
                  className="flex items-center justify-between rounded-[18px] border border-[var(--card-border)] bg-white/60 p-2 text-xs font-semibold text-[var(--fg)] dark:bg-slate-950/25"
                >
                  <span className="min-w-0 truncate">
                    {getBuildingName(building.type)} ({building.position.x}, {building.position.y})
                  </span>
                  {building.constructionProgress < 100 && (
                    <Badge variant="warning">{Math.round(building.constructionProgress)}%</Badge>
                  )}
                </div>
              ))}
            </div>
          ) : (
            <p className="mt-2 text-xs text-[var(--muted)]">No buildings yet</p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

// Export for use in ColonyGrid
export type { BuildMenuProps };



