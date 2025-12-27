"use client";

import type { Building, Cat, BuildingType } from "@/types/game";
import { Id } from "@/convex/_generated/dataModel";
import { useMutation, useQuery } from "convex/react";
import { api } from "@/convex/_generated/api";
import { BUILDING_COSTS } from "@/types/game";
import { CatSprite } from "@/components/colony/CatSprite";
import { MapViewport } from "@/components/ui/MapViewport";
import { Card } from "@/components/ui/card";

interface ColonyGridProps {
  colonyId: Id<"colonies">;
  gridSize: number;
  buildings: Building[];
  cats: Cat[];
  selectedBuildingType?: BuildingType | null;
  onBuildingPlaced?: () => void;
}

export function ColonyGrid({
  colonyId,
  gridSize,
  buildings,
  cats,
  selectedBuildingType,
  onBuildingPlaced,
}: ColonyGridProps) {
  const TILE_SIZE = 150;
  const placeBuilding = useMutation(api.buildings.placeBuilding);
  const updateResources = useMutation(api.colonies.updateColonyResources);
  const colony = useQuery(api.colonies.getColony, { colonyId });
  const getBuildingAt = (x: number, y: number) => {
    return buildings.find(
      (b) => b.position.x === x && b.position.y === y
    );
  };

  const getCatsAt = (x: number, y: number) => {
    return cats.filter(
      (c) =>
        c.position.map === "colony" &&
        c.position.x === x &&
        c.position.y === y
    );
  };

  const getBuildingImage = (type: Building["type"]) => {
    return `/images/buildings/${type}.png`;
  };

  const handleCellClick = async (x: number, y: number) => {
    if (!selectedBuildingType) return;

    // Check if position is occupied
    const existing = buildings.find(
      (b) => b.position.x === x && b.position.y === y
    );
    if (existing) {
      alert("Position already occupied!");
      return;
    }

    // Colony data needed to check materials
    if (!colony) return;

    if (colony.resources.materials < BUILDING_COSTS[selectedBuildingType]) {
      alert("Not enough materials!");
      return;
    }

    try {
      await placeBuilding({
        colonyId,
        type: selectedBuildingType,
        position: { x, y },
      });

      await updateResources({
        colonyId,
        resources: {
          ...colony.resources,
          materials: colony.resources.materials - BUILDING_COSTS[selectedBuildingType],
        },
      });

      onBuildingPlaced?.();
    } catch (error) {
      console.error("Failed to place building:", error);
    }
  };

  return (
    <div>
      <div className="mb-2 flex items-end justify-between gap-4">
        <div>
          <h2 className="text-lg font-black text-[var(--fg)]">Colony</h2>
          <p className="text-sm text-[var(--muted)]">Drag to pan • scroll to zoom • tiles are {TILE_SIZE}px</p>
        </div>
      </div>

      <Card className="p-3">
        <MapViewport
          contentSize={{ width: gridSize * TILE_SIZE, height: gridSize * TILE_SIZE }}
          height={560}
          initialScale={0.9}
          minScale={0.45}
          maxScale={2}
        >
        <div
          className="relative"
          style={{ width: gridSize * TILE_SIZE, height: gridSize * TILE_SIZE }}
        >
          {Array.from({ length: gridSize * gridSize }).map((_, index) => {
            const x = index % gridSize;
            const y = Math.floor(index / gridSize);
            const building = getBuildingAt(x, y);
            const catsAt = getCatsAt(x, y);
            const isSelected = selectedBuildingType !== null && selectedBuildingType !== undefined;

            return (
              <button
                key={index}
                type="button"
                onClick={() => handleCellClick(x, y)}
                className={`absolute flex items-center justify-center rounded-[22px] border text-left transition-all ${
                  isSelected && !building
                    ? "border-sage-500 bg-sage-50/90 hover:bg-sage-50 dark:bg-sage-900/20 dark:hover:bg-sage-900/30 ring-2 ring-[var(--ring)]"
                    : building
                    ? "border-[var(--card-border)] bg-white/70 hover:bg-white dark:bg-slate-950/25 dark:hover:bg-slate-950/35 shadow-sm"
                    : "border-[var(--card-border)] bg-gradient-to-br from-sage-50/80 to-cream-50/80 hover:from-sage-50 hover:to-cream-50 dark:from-slate-950/25 dark:to-slate-950/35 shadow-sm"
                } hover:shadow-md hover:-translate-y-0.5`}
                style={{
                  width: TILE_SIZE,
                  height: TILE_SIZE,
                  left: x * TILE_SIZE,
                  top: y * TILE_SIZE,
                  padding: 10,
                }}
              >
                <div className="absolute left-2 top-2 text-xs font-bold text-[var(--muted)]">
                  {x},{y}
                </div>

                {building ? (
                  <div className="flex flex-col items-center justify-center gap-1 w-full h-full p-2">
                    <img 
                      src={getBuildingImage(building.type)} 
                      alt={building.type}
                      className="w-full h-full object-contain drop-shadow-[0_10px_14px_rgba(43,33,23,0.18)]"
                    />
                    {building.constructionProgress < 100 && (
                      <div className="w-full rounded-full bg-black/10 dark:bg-white/10 h-2 overflow-hidden">
                        <div
                          className="h-full bg-amber-500"
                          style={{ width: `${Math.round(building.constructionProgress)}%` }}
                        />
                      </div>
                    )}
                  </div>
                ) : isSelected ? (
                  <div className="w-full text-center">
                    <div className="text-sm font-black text-sage-700 dark:text-sage-200">Place here</div>
                    <div className="text-[var(--muted)] text-xs mt-1 font-semibold">Click to build</div>
                  </div>
                ) : (
                  <div className="w-full text-center text-[var(--muted)] text-xs font-semibold">Grass</div>
                )}

                {catsAt.length > 0 && (
                  <div className="absolute bottom-2 left-2 flex items-center gap-1">
                    {catsAt.slice(0, 2).map((cat) => (
                      <CatSprite key={cat._id} cat={cat} size="small" />
                    ))}
                    {catsAt.length > 2 && (
                      <span className="text-xs bg-sage-600 text-white px-2 py-0.5 rounded-full font-black shadow-sm">
                        +{catsAt.length - 2}
                      </span>
                    )}
                  </div>
                )}
              </button>
            );
          })}
        </div>
        </MapViewport>
      </Card>
    </div>
  );
}



