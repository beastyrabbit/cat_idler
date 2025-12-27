"use client";

import { useQuery, useMutation } from "convex/react";
import { api } from "@/convex/_generated/api";
import { Id } from "@/convex/_generated/dataModel";
import type { WorldTile, Cat, Encounter } from "@/types/game";
import { MapViewport } from "@/components/ui/MapViewport";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useMemo } from "react";
import { Maximize2 } from "lucide-react";

interface WorldMapProps {
  colonyId: Id<"colonies">;
  cats: Cat[];
  expansionSize: number;
  onExpansionChange: (size: number) => void;
}

const CHUNK_SIZE = 12;
const TILE_SIZE = 150;

/**
 * Convert tile coordinates to chunk coordinates
 */
function tileToChunk(x: number, y: number) {
  return {
    chunkX: Math.floor(x / CHUNK_SIZE),
    chunkY: Math.floor(y / CHUNK_SIZE),
  };
}

/**
 * Get all chunks that should be visible based on viewport
 */
function getVisibleChunks(
  viewportX: number,
  viewportY: number,
  viewportWidth: number,
  viewportHeight: number,
  scale: number
): Array<{ chunkX: number; chunkY: number }> {
  const tileViewportX = viewportX / scale;
  const tileViewportY = viewportY / scale;
  const tileViewportWidth = viewportWidth / scale;
  const tileViewportHeight = viewportHeight / scale;

  const minTileX = tileViewportX - TILE_SIZE;
  const maxTileX = tileViewportX + tileViewportWidth + TILE_SIZE;
  const minTileY = tileViewportY - TILE_SIZE;
  const maxTileY = tileViewportY + tileViewportHeight + TILE_SIZE;

  const minChunkX = Math.floor(minTileX / (CHUNK_SIZE * TILE_SIZE));
  const maxChunkX = Math.ceil(maxTileX / (CHUNK_SIZE * TILE_SIZE));
  const minChunkY = Math.floor(minTileY / (CHUNK_SIZE * TILE_SIZE));
  const maxChunkY = Math.ceil(maxTileY / (CHUNK_SIZE * TILE_SIZE));

  const chunks: Array<{ chunkX: number; chunkY: number }> = [];
  for (let chunkY = minChunkY; chunkY <= maxChunkY; chunkY++) {
    for (let chunkX = minChunkX; chunkX <= maxChunkX; chunkX++) {
      chunks.push({ chunkX, chunkY });
    }
  }

  return chunks;
}

export function WorldMap({ colonyId, cats, expansionSize, onExpansionChange }: WorldMapProps) {
  // Get all tiles (for now, load all explored chunks)
  // In the future, we could optimize to only load visible chunks
  const tiles = useQuery(api.worldMap.getWorldTiles, { colonyId });
  const encounters = useQuery(api.encounters.getActiveEncounters, { colonyId });
  const expandRevealedArea = useMutation(api.worldMap.expandRevealedArea);

  const handleExpand = async () => {
    const newSize = expansionSize + 1;
    await expandRevealedArea({ colonyId, size: newSize });
    onExpansionChange(newSize);
  };

  // Create a map of tiles by coordinates for fast lookup
  const tileMap = useMemo(() => {
    const map = new Map<string, WorldTile>();
    tiles?.forEach((tile) => {
      map.set(`${tile.x},${tile.y}`, tile);
    });
    return map;
  }, [tiles]);

  // Calculate bounds of all tiles
  const bounds = useMemo(() => {
    if (!tiles || tiles.length === 0) {
      return { minX: 0, maxX: CHUNK_SIZE * 3, minY: 0, maxY: CHUNK_SIZE * 3 };
    }

    let minX = Infinity;
    let maxX = -Infinity;
    let minY = Infinity;
    let maxY = -Infinity;

    tiles.forEach((tile) => {
      minX = Math.min(minX, tile.x);
      maxX = Math.max(maxX, tile.x);
      minY = Math.min(minY, tile.y);
      maxY = Math.max(maxY, tile.y);
    });

    // Add padding
    return {
      minX: minX - CHUNK_SIZE,
      maxX: maxX + CHUNK_SIZE,
      minY: minY - CHUNK_SIZE,
      maxY: maxY + CHUNK_SIZE,
    };
  }, [tiles]);

  if (tiles === undefined) {
    return <div className="p-8">Loading world map...</div>;
  }

  const getCatsAt = (x: number, y: number) => {
    return cats.filter(
      (c) =>
        c.position.map === "world" &&
        c.position.x === x &&
        c.position.y === y
    );
  };

  const getEncounterAt = (x: number, y: number): Encounter | undefined => {
    return encounters?.find(
      (e: Encounter) => e.position.x === x && e.position.y === y && !e.resolved
    );
  };

  // Fog-of-war: tiles revealed when cats walk onto them (pathWear > 0)
  const isExplored = (x: number, y: number) => {
    const tile = tileMap.get(`${x},${y}`);
    if (tile && tile.pathWear > 0) return true;

    // Reveal adjacent to visited tiles so exploration feels less "spotty"
    const neighbors = [
      [x - 1, y],
      [x + 1, y],
      [x, y - 1],
      [x, y + 1],
    ];
    for (const [nx, ny] of neighbors) {
      const t = tileMap.get(`${nx},${ny}`);
      if (t && t.pathWear > 0) return true;
    }

    // Also reveal initial area around colony (center at 6, 6)
    const dx = x - 6;
    const dy = y - 6;
    const dist = Math.sqrt(dx * dx + dy * dy);
    if (dist < 3) return true;

    return false;
  };

  // Generate grid of tiles to render
  const gridTiles: Array<{ x: number; y: number; tile: WorldTile | null }> = [];
  for (let y = bounds.minY; y <= bounds.maxY; y++) {
    for (let x = bounds.minX; x <= bounds.maxX; x++) {
      const tile = tileMap.get(`${x},${y}`) || null;
      gridTiles.push({ x, y, tile });
    }
  }

  const contentWidth = (bounds.maxX - bounds.minX + 1) * TILE_SIZE;
  const contentHeight = (bounds.maxY - bounds.minY + 1) * TILE_SIZE;

  return (
    <div>
      <div className="mb-4 space-y-2">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-black text-[var(--fg)]">World map</h2>
        <Button
          variant="secondary"
          size="default"
          onClick={handleExpand}
          className="gap-2"
          title={`Expand revealed area to ${expansionSize + 1}x${expansionSize + 1} chunks`}
        >
          <Maximize2 className="h-4 w-4" />
          Expand Map ({expansionSize}x{expansionSize} → {expansionSize + 1}x{expansionSize + 1})
        </Button>
        </div>
        <p className="text-sm text-[var(--muted)]">
          Drag to pan • scroll to zoom • tiles are {TILE_SIZE}px • Infinite world
        </p>
      </div>

      <Card className="p-3">
        <MapViewport
          contentSize={{ width: contentWidth, height: contentHeight }}
          height={560}
          initialScale={0.7}
          minScale={0.35}
          maxScale={2}
        >
          <div
            className="relative"
            style={{ width: contentWidth, height: contentHeight }}
          >
            {gridTiles.map(({ x, y, tile }) => {
              const left = (x - bounds.minX) * TILE_SIZE;
              const top = (y - bounds.minY) * TILE_SIZE;

              const explored = isExplored(x, y);
              const catsAt = getCatsAt(x, y);
              const encounter = getEncounterAt(x, y);

              if (!tile && !explored) {
                // Unexplored and not generated - show as fog
                return (
                  <div
                    key={`${x}-${y}`}
                    className="absolute rounded-[22px] border border-white/10 bg-slate-900/60"
                    style={{ width: TILE_SIZE, height: TILE_SIZE, left, top }}
                  >
                    <div className="h-full w-full flex items-center justify-center text-slate-300 text-3xl">
                      ?
                    </div>
                  </div>
                );
              }

              if (!tile) {
                // Explored but tile not loaded - show placeholder
                return (
                  <div
                    key={`${x}-${y}`}
                    className="absolute rounded-[22px] border border-[var(--card-border)] bg-cream-50 dark:bg-slate-800"
                    style={{ width: TILE_SIZE, height: TILE_SIZE, left, top }}
                  />
                );
              }

              return (
                <div
                  key={tile._id}
                  className={`absolute rounded-[22px] border p-3 shadow-sm overflow-hidden ${
                    explored
                      ? "border-[var(--card-border)] bg-cream-50"
                      : "bg-slate-900/60 border-white/10"
                  }`}
                  style={{ width: TILE_SIZE, height: TILE_SIZE, left, top }}
                  title={
                    explored
                      ? `${tile.type} (${tile.resources.food} food, ${tile.resources.herbs} herbs)`
                      : "Unexplored"
                  }
                >
                  {explored ? (
                    <>
                      {/* Background Image */}
                      <img
                        src={`/images/tiles/${tile.type}.png`}
                        alt={tile.type}
                        className="absolute inset-0 w-full h-full object-cover opacity-80"
                        onError={(e) => {
                          // Fallback if image doesn't exist
                          (e.target as HTMLImageElement).style.display = "none";
                        }}
                      />

                      <div className="relative z-10 flex items-start justify-between">
                        <div className="text-5xl leading-none opacity-0">.</div>{" "}
                        {/* Spacer */}
                        {encounter && (
                          <div
                            className="text-2xl animate-cozy-pulse drop-shadow-md"
                            aria-label="Encounter"
                          >
                            ⚠️
                          </div>
                        )}
                      </div>

                      <div className="mt-3 flex items-center justify-between text-xs font-black">
                        <span className="text-[var(--muted)]">
                          {x},{y}
                        </span>
                        {tile.pathWear > 30 && (
                          <Badge variant="warning" className="gap-1">
                            🛤️ <span className="font-black">trail</span>
                          </Badge>
                        )}
                      </div>

                      <div className="mt-2 flex items-center justify-between text-xs">
                        {tile.resources.food > 0 ? (
                          <span className="rounded-full bg-black/10 px-2 py-0.5 font-bold dark:bg-white/10">
                            🍖 {tile.resources.food}
                          </span>
                        ) : (
                          <span />
                        )}
                        {tile.resources.herbs > 0 ? (
                          <span className="rounded-full bg-black/10 px-2 py-0.5 font-bold dark:bg-white/10">
                            🌿 {tile.resources.herbs}
                          </span>
                        ) : (
                          <span />
                        )}
                      </div>

                      {catsAt.length > 0 && (
                        <Badge className="absolute bottom-2 left-2 bg-sage-600 text-white border-transparent">
                          🐱 {catsAt.length}
                        </Badge>
                      )}
                    </>
                  ) : (
                    <div className="h-full w-full flex items-center justify-center text-slate-300 text-3xl">
                      ?
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </MapViewport>
      </Card>
    </div>
  );
}
