"use client";

import { useMemo, useState } from "react";
import { useMutation, useQuery } from "convex/react";
import { api } from "@/convex/_generated/api";
import { Id } from "@/convex/_generated/dataModel";
import type { Cat, Colony, EventLog as EventLogType } from "@/types/game";
import { CatSprite } from "@/components/colony/CatSprite";

function formatDuration(ms: number) {
  const s = Math.max(0, Math.floor(ms / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${sec}s`;
  return `${sec}s`;
}

export function ColonyDeathSummary({ colonyId }: { colonyId: Id<"colonies"> }) {
  const colony = useQuery(api.colonies.getColony, { colonyId });
  const cats = useQuery(api.cats.getCatsByColony, { colonyId });
  const events = useQuery(api.events.getEventsByColony, { colonyId, limit: 1000 });
  const createColony = useMutation(api.colonies.createColony);

  const [newName, setNewName] = useState("");
  const [tab, setTab] = useState<"cats" | "events">("cats");
  const [catQuery, setCatQuery] = useState("");
  const [eventQuery, setEventQuery] = useState("");

  const derived = useMemo(() => {
    if (!colony || !cats || !events) return null;
    const c = colony as Colony;

    const deadCats = cats.filter((x: Cat) => x.deathTime != null);
    const taskComplete = events.filter((e: EventLogType) => e.type === "task_complete").length;
    const discoveries = events.filter((e: EventLogType) => e.type === "discovery").length;
    const survivedFor = formatDuration(c.lastTick - c.createdAt);

    // Map catId -> last death message (cause), if any.
    const deathByCat = new Map<string, { ts: number; msg: string }>();
    for (const e of events as any[]) {
      if (e.type !== "death") continue;
      const ids: string[] = [];
      if (e.catId) ids.push(e.catId);
      if (Array.isArray(e.involvedCatIds)) ids.push(...e.involvedCatIds);
      for (const id of ids) {
        const prev = deathByCat.get(id);
        if (!prev || e.timestamp > prev.ts) deathByCat.set(id, { ts: e.timestamp, msg: e.message });
      }
    }

    const eventsDesc = [...(events as any[])].sort((a, b) => b.timestamp - a.timestamp);

    return {
      survivedFor,
      deadCatsCount: deadCats.length,
      catsCount: cats.length,
      taskComplete,
      discoveries,
      deathByCat,
      eventsDesc,
      colony: c,
    };
  }, [cats, colony, events]);

  const handleRestart = async () => {
    const name = newName.trim();
    if (!name) return;
    const id = await createColony({ name, leaderId: null });
    window.location.href = `/colony/${id}`;
  };

  if (colony === undefined || cats === undefined || events === undefined) {
    return <div className="mx-auto max-w-6xl px-6 py-10">Loading…</div>;
  }
  if (!colony) {
    return <div className="mx-auto max-w-6xl px-6 py-10">Colony not found.</div>;
  }

  return (
    <div className="mx-auto max-w-6xl px-6 py-12 min-h-[calc(100vh-4rem)]">
      {/* Hero */}
      <div className="relative overflow-hidden rounded-[28px] bg-gradient-to-br from-bark-900/70 via-sage-900/30 to-slate-950/80 p-8 shadow-2xl ring-1 ring-white/10">
        <div className="pointer-events-none absolute -top-40 -right-40 h-[30rem] w-[30rem] rounded-full bg-peach-500/20 blur-3xl" />
        <div className="pointer-events-none absolute -bottom-40 -left-40 h-[30rem] w-[30rem] rounded-full bg-sky-500/18 blur-3xl" />
        <div className="pointer-events-none absolute inset-0 opacity-[0.10] [background-image:radial-gradient(rgba(255,255,255,0.18)_1px,transparent_1px)] [background-size:18px_18px]" />

        <div className="relative grid grid-cols-1 gap-8 lg:grid-cols-12 lg:items-start">
          <div className="lg:col-span-7">
            <div className="inline-flex items-center gap-2 rounded-full border border-white/10 bg-white/5 px-4 py-2 text-xs font-extrabold tracking-wide text-white/80">
              GAME OVER
              <span className="rounded-full bg-peach-500/25 px-2 py-0.5 text-[11px] font-black text-peach-50">
                colony lost ☠
              </span>
            </div>

            <h1 className="mt-4 text-4xl font-black tracking-tight text-white md:text-5xl">
              {colony.name}
            </h1>
            <p className="mt-3 text-base text-white/70">
              Review what happened, then start a new run.
              {derived ? <span className="font-semibold text-white/90"> • Survived {derived.survivedFor}</span> : null}
            </p>

            {derived && (
              <div className="mt-6 grid grid-cols-2 gap-3 md:grid-cols-4">
                {[
                  { label: "Cats", value: derived.catsCount },
                  { label: "Deaths", value: derived.deadCatsCount },
                  { label: "Tasks", value: derived.taskComplete },
                  { label: "Discoveries", value: derived.discoveries },
                ].map((s) => (
                  <div
                    key={s.label}
                    className="rounded-2xl bg-white/5 p-4 ring-1 ring-white/10"
                  >
                    <div className="text-xs font-bold uppercase tracking-wide text-white/55">
                      {s.label}
                    </div>
                    <div className="mt-1 text-3xl font-black text-white tabular-nums">
                      {s.value}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          <div className="lg:col-span-5">
            <div className="rounded-2xl bg-white/5 p-5 ring-1 ring-white/10">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <div className="text-sm font-black text-white">Start a new colony</div>
                  <p className="mt-1 text-sm text-white/70">New run, fresh leader, new chance.</p>
                </div>
                <button className="btn-secondary" onClick={() => (window.location.href = "/")}>
                  Home
                </button>
              </div>

              <div className="mt-4 flex gap-2">
                <input
                  className="input"
                  placeholder="New colony name"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleRestart()}
                />
                <button className="btn-primary whitespace-nowrap" onClick={handleRestart}>
                  Create
                </button>
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                {["Mossclan", "Emberden", "Snowpaws"].map((preset) => (
                  <button
                    key={preset}
                    className="btn-secondary whitespace-nowrap"
                    onClick={() => setNewName(preset)}
                    type="button"
                    title="Use suggested name"
                  >
                    {preset}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Tabs + lists */}
      <div className="mt-8 rounded-[28px] bg-white/70 p-5 shadow-xl ring-1 ring-[var(--card-border)] backdrop-blur dark:bg-slate-950/35 dark:ring-white/10">
        <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <div className="tabbar">
            <button className={"tab " + (tab === "cats" ? "tab-active" : "")} onClick={() => setTab("cats")}>
              Cats
            </button>
            <button className={"tab " + (tab === "events" ? "tab-active" : "")} onClick={() => setTab("events")}>
              Timeline
            </button>
          </div>

          {tab === "cats" ? (
            <input
              className="input md:max-w-sm"
              placeholder="Search cats…"
              value={catQuery}
              onChange={(e) => setCatQuery(e.target.value)}
            />
          ) : (
            <input
              className="input md:max-w-sm"
              placeholder="Search events…"
              value={eventQuery}
              onChange={(e) => setEventQuery(e.target.value)}
            />
          )}
        </div>

        <div className="mt-5">
          {tab === "cats" ? (
            <div className="grid grid-cols-1 gap-3 lg:grid-cols-2">
              {cats
                .slice()
                .filter((c: Cat) => c.name.toLowerCase().includes(catQuery.trim().toLowerCase()))
                .sort((a: Cat, b: Cat) => (a.deathTime ? 1 : 0) - (b.deathTime ? 1 : 0))
                .map((cat: Cat) => {
                  const status = cat.deathTime ? "dead" : "alive";
                  const cause = derived?.deathByCat.get(cat._id)?.msg;
                  return (
                    <div
                      key={cat._id}
                      className="group rounded-2xl bg-white/75 p-4 shadow-sm ring-1 ring-slate-200/60 transition-shadow hover:shadow-md dark:bg-slate-900/30 dark:ring-white/10"
                    >
                      <div className="flex items-center gap-3">
                        <div className="shrink-0">
                          <CatSprite cat={cat} size="medium" />
                        </div>
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center justify-between gap-3">
                            <div className="truncate text-base font-black text-slate-950 dark:text-slate-50">
                              {cat.name}
                            </div>
                            <span
                              className={
                                "rounded-full px-3 py-1 text-xs font-black tracking-wide " +
                                (status === "dead"
                                  ? "bg-red-500/10 text-red-700 dark:text-red-300"
                                  : "bg-emerald-500/10 text-emerald-700 dark:text-emerald-300")
                              }
                            >
                              {status}
                            </span>
                          </div>
                          <div className="mt-1 text-xs font-semibold text-slate-600 dark:text-slate-300">
                            {cat.currentTask ? cat.currentTask.replaceAll("_", " ") : "no task"}
                          </div>
                          {status === "dead" && cause ? (
                            <div className="mt-2 text-xs text-slate-700 dark:text-slate-300">
                              <span className="font-bold">Cause:</span> {cause}
                            </div>
                          ) : null}
                        </div>
                      </div>
                    </div>
                  );
                })}
            </div>
          ) : (
            <div className="max-h-[760px] overflow-y-auto">
              <div className="relative pl-6">
                <div className="absolute left-2 top-0 h-full w-px bg-slate-200 dark:bg-slate-800" />
                {(derived?.eventsDesc ?? [])
                  .filter((e: any) => {
                    const q = eventQuery.trim().toLowerCase();
                    if (!q) return true;
                    return (
                      String(e.message ?? "").toLowerCase().includes(q) ||
                      String(e.type ?? "").toLowerCase().includes(q)
                    );
                  })
                  .map((event: any) => (
                    <div key={event._id} className="relative mb-3">
                      <div className="absolute -left-1 top-4 h-3 w-3 rounded-full bg-slate-300 ring-4 ring-white dark:bg-slate-700 dark:ring-slate-950" />
                      <div className="rounded-2xl bg-white/75 p-4 shadow-sm ring-1 ring-slate-200/60 dark:bg-slate-900/30 dark:ring-white/10">
                        <div className="flex items-center justify-between gap-3">
                          <div className="text-xs font-semibold text-slate-500 dark:text-slate-400">
                            {new Date(event.timestamp).toLocaleString()}
                          </div>
                          <span className="rounded-full bg-black/10 px-3 py-1 text-xs font-black dark:bg-white/10">
                            {String(event.type).replaceAll("_", " ")}
                          </span>
                        </div>
                        <div className="mt-2 text-sm font-semibold text-slate-950 dark:text-slate-50">
                          {event.message}
                        </div>
                      </div>
                    </div>
                  ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}


