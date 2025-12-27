"use client";

import { useMutation } from "convex/react";
import { api } from "@/convex/_generated/api";
import { Id } from "@/convex/_generated/dataModel";
import type { Cat } from "@/types/game";
import { useState, useEffect } from "react";
import { ResourceBar } from "@/components/ui/ResourceBar";
import { CatSprite } from "@/components/colony/CatSprite";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface CatListProps {
  colonyId: Id<"colonies">;
  cats: Cat[];
}

export function CatList({ colonyId, cats }: CatListProps) {
  const updateCatNeeds = useMutation(api.cats.updateCatNeeds);
  const [healCooldowns, setHealCooldowns] = useState<Record<string, number>>({});
  const [currentTime, setCurrentTime] = useState(() => Date.now());

  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentTime(Date.now());
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const handleHealCat = async (catId: Id<"cats">) => {
    const lastHeal = healCooldowns[catId] || 0;
    if (currentTime - lastHeal < 5000) return; // 5 seconds per cat

    const cat = cats.find((c) => c._id === catId);
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

  const getCooldownRemaining = (catId: Id<"cats">): number => {
    const lastHeal = healCooldowns[catId] || 0;
    const remaining = 5000 - (currentTime - lastHeal);
    return remaining > 0 ? Math.ceil(remaining / 1000) : 0;
  };

  return (
    <Card className="mt-6 bg-white/55 dark:bg-slate-950/25">
      <CardHeader className="p-4 pb-2">
        <div className="flex items-end justify-between gap-4">
          <CardTitle>Cats</CardTitle>
          <Badge variant="secondary" className="font-black">
            🐱 {cats.length}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="p-4 pt-0">
        <div className="mt-2 grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {cats.map((cat) => {
          const cooldown = getCooldownRemaining(cat._id);
          const canHeal = cooldown === 0 && cat.needs.health < 100;

          return (
            <div
              key={cat._id}
              className={cn(
                "rounded-[22px] border p-3 cursor-pointer transition-all shadow-sm hover:shadow-md hover:-translate-y-0.5",
                canHeal
                  ? "border-sage-500/60 bg-sage-50/60 dark:bg-sage-900/15"
                  : "border-[var(--card-border)] bg-white/70 dark:bg-slate-950/25"
              )}
              onClick={() => handleHealCat(cat._id)}
              title={canHeal ? "Click to heal (+10 health)" : cooldown > 0 ? `Heal cooldown: ${cooldown}s` : "Health is full"}
            >
              <div className="flex items-center gap-2 mb-2">
                <CatSprite cat={cat} size="small" />
                <p className="min-w-0 truncate font-extrabold text-[var(--fg)]">{cat.name}</p>
              </div>
              <div className="mt-2 space-y-1">
                <ResourceBar
                  value={cat.needs.health}
                  max={100}
                  color={cat.needs.health < 30 ? "red" : cat.needs.health < 70 ? "yellow" : "green"}
                  showLabel
                  label="Health"
                />
                <ResourceBar
                  value={cat.needs.hunger}
                  max={100}
                  color={cat.needs.hunger < 30 ? "red" : "green"}
                  showLabel
                  label="Hunger"
                />
                <ResourceBar
                  value={cat.needs.thirst}
                  max={100}
                  color={cat.needs.thirst < 30 ? "red" : "blue"}
                  showLabel
                  label="Thirst"
                />
              </div>
              <p className="text-xs mt-2 text-[var(--muted)] font-semibold">
                Task: {cat.currentTask ? cat.currentTask.replace("_", " ") : "None"}
              </p>
              {canHeal && (
                <p className="text-xs text-sage-700 dark:text-sage-200 mt-1 font-black">Click to heal!</p>
              )}
              {cooldown > 0 && (
                <p className="text-xs text-[var(--muted)] mt-1">Heal: {cooldown}s</p>
              )}
            </div>
          );
        })}
        </div>
      </CardContent>
    </Card>
  );
}



