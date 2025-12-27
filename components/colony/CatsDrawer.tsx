"use client";

import type { Cat } from "@/types/game";
import { CatSprite } from "@/components/colony/CatSprite";
import { ResourceBar } from "@/components/ui/ResourceBar";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetDescription } from "@/components/ui/sheet";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Card } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";

interface CatsDrawerProps {
  open: boolean;
  onClose: () => void;
  cats: Cat[];
}

export function CatsDrawer({ open, onClose, cats }: CatsDrawerProps) {
  return (
    <Sheet
      open={open}
      onOpenChange={(v) => {
        if (!v) onClose();
      }}
    >
      <SheetContent side="right" className="w-[420px] max-w-[92vw] p-4">
        <SheetHeader className="pr-8">
          <div className="flex items-start justify-between gap-3">
            <div>
              <SheetTitle>Cats</SheetTitle>
              <SheetDescription>{cats.length} in colony</SheetDescription>
            </div>
            <Badge variant="secondary" className="font-black">
              🐱 {cats.length}
            </Badge>
          </div>
        </SheetHeader>

        <div className="mt-4">
          <ScrollArea className="h-[calc(100vh-10rem)] rounded-[22px] border border-[var(--card-border)] bg-white/55 p-3 dark:bg-slate-950/20">
            <div className="space-y-3 pr-2">
              {cats.length === 0 ? (
                <p className="text-sm text-[var(--muted)]">No cats found.</p>
              ) : (
                cats.map((cat) => (
                  <Card key={cat._id} className="p-3 bg-white/70 dark:bg-slate-950/20">
                    <div className="flex items-center gap-3">
                      <CatSprite cat={cat} size="medium" />
                      <div className="min-w-0 flex-1">
                        <div className="flex items-start justify-between gap-2">
                          <div className="truncate font-extrabold text-[var(--fg)]">{cat.name}</div>
                          <Badge variant="secondary" className="shrink-0">
                            {cat.currentTask ? cat.currentTask.replace("_", " ") : "idle"}
                          </Badge>
                        </div>
                        <div className="mt-3 grid grid-cols-1 gap-2">
                          <ResourceBar value={cat.needs.health} max={100} color={cat.needs.health < 30 ? "red" : "green"} />
                          <div className="grid grid-cols-2 gap-2">
                            <ResourceBar value={cat.needs.hunger} max={100} color={cat.needs.hunger < 30 ? "red" : "green"} />
                            <ResourceBar value={cat.needs.thirst} max={100} color={cat.needs.thirst < 30 ? "red" : "blue"} />
                          </div>
                        </div>
                      </div>
                    </div>
                  </Card>
                ))
              )}
            </div>
          </ScrollArea>
        </div>
      </SheetContent>
    </Sheet>
  );
}


