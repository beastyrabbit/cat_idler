"use client";

import { useMutation } from "convex/react";
import { api } from "@/convex/_generated/api";
import { Id } from "@/convex/_generated/dataModel";
import type { Encounter, Cat } from "@/types/game";
import { useState, useEffect } from "react";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";

interface EncounterPopupProps {
  encounter: Encounter;
  cat: Cat;
  onClose: () => void;
}

export function EncounterPopup({ encounter, cat, onClose }: EncounterPopupProps) {
  const addClicks = useMutation(api.encounters.addClicks);
  const [clicks, setClicks] = useState(encounter.clicksReceived);
  const [timeRemaining, setTimeRemaining] = useState(0);

  useEffect(() => {
    const updateTime = () => {
      const remaining = Math.max(0, Math.ceil((encounter.expiresAt - Date.now()) / 1000));
      setTimeRemaining(remaining);
      if (remaining === 0 && !encounter.resolved) {
        // Auto-resolve will happen on next tick
      }
    };
    
    updateTime();
    const interval = setInterval(updateTime, 1000);

    return () => clearInterval(interval);
  }, [encounter]);

  const handleClick = async () => {
    if (encounter.resolved || clicks >= encounter.clicksNeeded) return;

    await addClicks({
      encounterId: encounter._id,
      clicks: 1,
    });

    setClicks((prev) => prev + 1);
  };

  const getEnemyImage = (enemyType: Encounter["enemyType"]) => {
    return `/images/enemies/${enemyType || "fox"}.png`;
  };

  const getEnemyName = (enemyType: Encounter["enemyType"]) => {
    const names: Record<NonNullable<Encounter["enemyType"]>, string> = {
      fox: "Fox",
      hawk: "Hawk",
      badger: "Badger",
      bear: "Bear",
      rival_cat: "Rival Cat",
    };
    return names[enemyType || "fox"] || "Enemy";
  };

  const title = encounter.resolved
    ? encounter.outcome === "user_win" || encounter.outcome === "cat_win"
      ? "Victory!"
      : "Defeat…"
    : `${getEnemyName(encounter.enemyType)} attack!`;

  if (encounter.resolved) {
    return (
      <Dialog
        open
        onOpenChange={(v) => {
          if (!v) onClose();
        }}
      >
        <DialogContent className="max-w-md">
          <DialogHeader>
            <div className="flex items-center justify-between gap-3">
              <DialogTitle>
                {title}{" "}
                <span aria-hidden="true">
                  {encounter.outcome === "user_win" || encounter.outcome === "cat_win" ? "🎉" : "😿"}
                </span>
              </DialogTitle>
              <Badge variant={encounter.outcome === "user_win" || encounter.outcome === "cat_win" ? "success" : "danger"}>
                {encounter.outcome === "user_win" || encounter.outcome === "cat_win" ? "safe" : "hurt"}
              </Badge>
            </div>
            <DialogDescription>
              {encounter.outcome === "user_win" || encounter.outcome === "cat_win"
                ? `${cat.name} successfully defended against the ${getEnemyName(encounter.enemyType)}.`
                : `${cat.name} was defeated by the ${getEnemyName(encounter.enemyType)}.`}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter className="mt-4">
            <Button onClick={onClose} className="w-full">
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }

  const progress = (clicks / encounter.clicksNeeded) * 100;

  return (
    <Dialog
      open
      onOpenChange={(v) => {
        if (!v) onClose();
      }}
    >
      <DialogContent className="max-w-md">
        <DialogHeader>
          <div className="flex items-start justify-between gap-3">
            <div>
              <DialogTitle className="flex items-center gap-2">
                <img 
                  src={getEnemyImage(encounter.enemyType)} 
                  alt="" 
                  className="w-12 h-12 object-contain"
                />
                {getEnemyName(encounter.enemyType)} spotted!
              </DialogTitle>
              <DialogDescription>
                {cat.name} needs help. Click to defend before time runs out.
              </DialogDescription>
            </div>
            <Badge variant="warning">⚠️ danger</Badge>
          </div>
        </DialogHeader>

        <div className="mt-4 space-y-3">
          <div className="flex items-center justify-between text-sm font-semibold text-[var(--muted)]">
            <span>Defense progress</span>
            <span className="tabular-nums">
              {clicks} / {encounter.clicksNeeded}
            </span>
          </div>
          <Progress
            value={Math.min(100, Math.max(0, progress))}
            className="h-4"
            indicatorClassName="bg-peach-500"
            indicatorTestId="encounter-progress"
          />

          <Button
            onClick={handleClick}
            disabled={clicks >= encounter.clicksNeeded}
            variant={clicks >= encounter.clicksNeeded ? "default" : "destructive"}
            className="w-full text-base font-black"
          >
            {clicks >= encounter.clicksNeeded ? "Victory!" : "Click to Help!"}
          </Button>

          <div className="flex items-center justify-between text-sm text-[var(--muted)]">
            <span>Auto-resolve</span>
            <span className="tabular-nums">{timeRemaining}s</span>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}



