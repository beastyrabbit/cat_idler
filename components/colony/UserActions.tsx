"use client";

import { useMutation, useQuery } from "convex/react";
import { api } from "@/convex/_generated/api";
import { Id } from "@/convex/_generated/dataModel";
import { useEffect, useState } from "react";
import type { Colony, Cat } from "@/types/game";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

interface UserActionsProps {
  colonyId: Id<"colonies">;
  colony: Colony;
}

export function UserActions({ colonyId, colony }: UserActionsProps) {
  const updateResources = useMutation(api.colonies.updateColonyResources);
  const updateCatNeeds = useMutation(api.cats.updateCatNeeds);
  const cats = useQuery(api.cats.getAliveCats, { colonyId });
  
  const [lastFeedTime, setLastFeedTime] = useState(0);
  const [lastWaterTime, setLastWaterTime] = useState(0);
  const [healCooldowns, setHealCooldowns] = useState<Record<string, number>>({});
  const [currentTime, setCurrentTime] = useState(() => Date.now());

  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentTime(Date.now());
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const canFeed = currentTime - lastFeedTime >= 10000; // 10 seconds
  const canGiveWater = currentTime - lastWaterTime >= 10000; // 10 seconds

  const handleFeed = async () => {
    if (!canFeed) return;
    
    await updateResources({
      colonyId,
      resources: {
        ...colony.resources,
        food: colony.resources.food + 1,
      },
    });
    setLastFeedTime(currentTime);
  };

  const handleGiveWater = async () => {
    if (!canGiveWater) return;
    
    await updateResources({
      colonyId,
      resources: {
        ...colony.resources,
        water: colony.resources.water + 1,
      },
    });
    setLastWaterTime(currentTime);
  };

  const handleHealCat = async (catId: Id<"cats">) => {
    const lastHeal = healCooldowns[catId] || 0;
    if (Date.now() - lastHeal < 5000) return; // 5 seconds per cat

    // Get cat first to check current health
    const cat = cats?.find((c: Cat) => c._id === catId);
    if (!cat || cat.needs.health >= 100) return;

    await updateCatNeeds({
      catId,
      needs: {
        ...cat.needs,
        health: Math.min(100, cat.needs.health + 10),
      },
    });
    
    setHealCooldowns((prev) => ({ ...prev, [catId]: currentTime }));
  };

  const getCooldownRemaining = (lastTime: number, cooldown: number): number => {
    const remaining = cooldown - (currentTime - lastTime);
    return remaining > 0 ? Math.ceil(remaining / 1000) : 0;
  };

  return (
    <Card className="bg-white/55 dark:bg-slate-950/25">
      <CardHeader className="p-4 pb-2">
        <div className="flex items-start justify-between gap-3">
          <CardTitle>Actions</CardTitle>
          <Badge variant="secondary" className="font-black">
            ⏱️ 10s
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="p-4 pt-0">
        <div className="space-y-2">
          <Button
            onClick={handleFeed}
            disabled={!canFeed}
            variant={canFeed ? "default" : "secondary"}
            className="w-full justify-between"
          >
            <span>🍖 Give Food (+1)</span>
            {!canFeed ? (
              <span className="tabular-nums opacity-80">{getCooldownRemaining(lastFeedTime, 10000)}s</span>
            ) : null}
          </Button>

          <Button
            onClick={handleGiveWater}
            disabled={!canGiveWater}
            variant={canGiveWater ? "default" : "secondary"}
            className="w-full justify-between"
          >
            <span>💧 Give Water (+1)</span>
            {!canGiveWater ? (
              <span className="tabular-nums opacity-80">{getCooldownRemaining(lastWaterTime, 10000)}s</span>
            ) : null}
          </Button>
        </div>

        <div className="mt-4">
          <p className="text-sm text-[var(--muted)]">
            Healing is available from the cats panel (per-cat cooldown).
          </p>
        </div>
      </CardContent>
    </Card>
  );
}




