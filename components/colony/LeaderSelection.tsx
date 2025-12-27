"use client";

import { useQuery, useMutation } from "convex/react";
import { api } from "@/convex/_generated/api";
import { Id } from "@/convex/_generated/dataModel";
import { useMemo, useState } from "react";
import { ResourceBar } from "@/components/ui/ResourceBar";
import type { Cat } from "@/types/game";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface LeaderSelectionProps {
  colonyId: Id<"colonies">;
  onLeaderSelected: () => void;
}

// Stable random seed for consistent shuffling
function shuffleArray<T>(array: T[], seed: number): T[] {
  const shuffled = [...array];
  let currentIndex = shuffled.length;
  let randomIndex: number;
  
  // Simple seeded random
  let seedValue = seed;
  const seededRandom = () => {
    seedValue = (seedValue * 9301 + 49297) % 233280;
    return seedValue / 233280;
  };
  
  while (currentIndex !== 0) {
    randomIndex = Math.floor(seededRandom() * currentIndex);
    currentIndex--;
    [shuffled[currentIndex], shuffled[randomIndex]] = [shuffled[randomIndex], shuffled[currentIndex]];
  }
  
  return shuffled;
}

function hashStringToSeed(str: string): number {
  // FNV-1a-ish, deterministic and pure (no Math.random).
  let h = 2166136261;
  for (let i = 0; i < str.length; i++) {
    h ^= str.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  return h >>> 0;
}

export function LeaderSelection({ colonyId, onLeaderSelected }: LeaderSelectionProps) {
  const cats = useQuery(api.cats.getAliveCats, { colonyId });
  const setLeader = useMutation(api.colonies.setColonyLeader);
  const [selectedCatId, setSelectedCatId] = useState<Id<"cats"> | null>(null);
  const seed = useMemo(() => hashStringToSeed(String(colonyId)), [colonyId]);

  // Get 5 random cats or all if less than 5
  const candidates = useMemo(() => {
    if (!cats) return [];
    return shuffleArray(cats, seed).slice(0, Math.min(5, cats.length));
  }, [cats, seed]);

  if (cats === undefined) {
    return <div>Loading candidates...</div>;
  }

  const handleSelectLeader = async () => {
    if (!selectedCatId) return;

    await setLeader({
      colonyId,
      catId: selectedCatId,
    });

    onLeaderSelected();
  };

  return (
    <Dialog open>
      <DialogContent className="max-w-2xl" hideClose>
        <DialogHeader>
          <div className="flex items-start justify-between gap-3">
            <div>
              <DialogTitle className="flex items-center gap-2">
                <span aria-hidden="true">👑</span> Select Colony Leader
              </DialogTitle>
              <DialogDescription>
                Choose a cat to lead your colony. Higher leadership means faster, smarter task assignments.
              </DialogDescription>
            </div>
            <Badge variant="secondary">pick 1</Badge>
          </div>
        </DialogHeader>

        <div className="mt-4 grid grid-cols-1 gap-3 md:grid-cols-2">
          {candidates.map((cat: Cat) => {
            const selected = selectedCatId === cat._id;
            const leadershipColor =
              cat.stats.leadership > 50 ? "green" : cat.stats.leadership > 30 ? "yellow" : "red";

            return (
              <button
                key={cat._id}
                type="button"
                onClick={() => setSelectedCatId(cat._id)}
                className={cn(
                  "w-full rounded-[22px] border p-4 text-left transition-all shadow-sm hover:shadow-md hover:-translate-y-0.5",
                  "bg-white/70 dark:bg-slate-950/25 border-[var(--card-border)]",
                  selected && "border-sage-500 ring-2 ring-[var(--ring)]"
                )}
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="truncate text-lg font-black text-[var(--fg)]">{cat.name}</div>
                    <div className="mt-0.5 text-xs font-semibold text-[var(--muted)]">
                      {cat.stats.leadership > 30
                        ? "Great leader"
                        : cat.stats.leadership > 20
                        ? "Good leader"
                        : cat.stats.leadership > 10
                        ? "Okay leader"
                        : "Poor leader"}
                    </div>
                  </div>
                  {selected ? <Badge variant="success">✓ selected</Badge> : <Badge variant="secondary">candidate</Badge>}
                </div>

                <div className="mt-3">
                  <div className="mb-1 text-xs font-bold text-[var(--muted)]">Leadership</div>
                  <ResourceBar value={cat.stats.leadership} max={100} color={leadershipColor} />
                </div>

                <div className="mt-3 grid grid-cols-2 gap-2 text-xs font-semibold text-[var(--fg)]">
                  <div>
                    <span className="text-[var(--muted)]">Attack:</span> {cat.stats.attack}
                  </div>
                  <div>
                    <span className="text-[var(--muted)]">Defense:</span> {cat.stats.defense}
                  </div>
                  <div>
                    <span className="text-[var(--muted)]">Hunting:</span> {cat.stats.hunting}
                  </div>
                  <div>
                    <span className="text-[var(--muted)]">Medicine:</span> {cat.stats.medicine}
                  </div>
                </div>
              </button>
            );
          })}
        </div>

        <DialogFooter className="mt-4">
          <Button onClick={handleSelectLeader} disabled={!selectedCatId} className="w-full">
            Select Leader
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}



