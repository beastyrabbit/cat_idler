"use client";

import { useQuery } from "convex/react";
import { api } from "@/convex/_generated/api";
import { Id } from "@/convex/_generated/dataModel";
import type { EventLog as EventLogType } from "@/types/game";
import { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";

interface EventLogProps {
  colonyId: Id<"colonies">;
}

export function EventLog({ colonyId }: EventLogProps) {
  const events = useQuery(api.events.getEventsByColony, {
    colonyId,
    limit: 50,
  });
  const [currentTime, setCurrentTime] = useState(() => Date.now());

  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentTime(Date.now());
    }, 60000); // Update every minute
    return () => clearInterval(interval);
  }, []);

  if (events === undefined) {
    return (
      <Card className="bg-white/55 dark:bg-slate-950/25">
        <CardHeader className="p-4 pb-2">
          <CardTitle>Event Log</CardTitle>
        </CardHeader>
        <CardContent className="p-4 pt-0">
          <p className="text-sm text-[var(--muted)]">Loading events...</p>
        </CardContent>
      </Card>
    );
  }

  const getEventIcon = (type: string) => {
    const icons: Record<string, string> = {
      birth: "👶",
      death: "💀",
      intruder_attack: "⚔️",
      intruder_defeated: "🛡️",
      breeding: "💕",
      leader_change: "👑",
      task_complete: "✅",
      building_complete: "🏗️",
      user_fed: "🍖",
      user_healed: "💊",
      cat_joined: "🐱",
      cat_left: "👋",
      discovery: "🗺️",
      crisis: "🚨",
      recovery: "🌊",
    };
    return icons[type] || "📋";
  };

  const formatTime = (timestamp: number) => {
    const date = new Date(timestamp);
    const diff = currentTime - timestamp;
    
    if (diff < 60000) return "Just now";
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
    return date.toLocaleDateString();
  };

  return (
    <Card className="bg-white/55 dark:bg-slate-950/25">
      <CardHeader className="p-4 pb-2">
        <CardTitle>Event Log</CardTitle>
      </CardHeader>
      <CardContent className="p-4 pt-0">
        {events.length === 0 ? (
          <p className="text-sm text-[var(--muted)]">No events yet</p>
        ) : (
          <ScrollArea className="h-96 rounded-[18px] border border-[var(--card-border)] bg-white/55 p-2 dark:bg-slate-950/25">
            <div className="space-y-2 pr-2">
              {events.map((event: EventLogType) => (
                <div
                  key={event._id}
                  className="flex items-start gap-3 rounded-[18px] border border-[var(--card-border)] bg-white/70 p-3 shadow-sm dark:bg-slate-950/20"
                >
                  <div className="grid h-9 w-9 place-items-center rounded-2xl bg-black/5 text-lg dark:bg-white/10">
                    {getEventIcon(event.type)}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center justify-between gap-3">
                      <div className="truncate text-sm font-bold text-[var(--fg)]">{event.message}</div>
                      <Badge variant="secondary" className="shrink-0">
                        {formatTime(event.timestamp)}
                      </Badge>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}


