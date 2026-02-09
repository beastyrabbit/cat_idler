"use client";

import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery } from "convex/react";

import { api } from "@/convex/_generated/api";
import { presetFromTimeScale } from "@/lib/game/testAcceleration";

const anyApi = api as any;

function createSessionId(): string {
  return `session_${Math.random().toString(36).slice(2)}_${Date.now()}`;
}

function cleanErrorMessage(err: unknown): string {
  if (err == null) return "An unknown error occurred";
  const raw = err instanceof Error ? err.message : String(err);
  const match = raw.match(/Uncaught Error: (.+?)(?:\s+at\s|$)/);
  return match ? match[1].trim() : raw;
}

export function formatDuration(ms: number): string {
  if (ms <= 0) {
    return "done";
  }

  const totalSeconds = Math.ceil(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }

  if (minutes > 0) {
    return `${minutes}m ${seconds}s`;
  }

  return `${seconds}s`;
}

/** Player-requestable job kinds only. `hunt_expedition` and `build_house` are auto-queued by the worker. */
export type JobKind =
  | "supply_food"
  | "supply_water"
  | "leader_plan_hunt"
  | "leader_plan_house"
  | "ritual";

export function useGameDashboard() {
  const dashboard = useQuery(anyApi.game.getGlobalDashboard);

  const ensureGlobalState = useMutation(anyApi.game.ensureGlobalState);
  const upsertPresence = useMutation(anyApi.players.upsertPresence);
  const requestJob = useMutation(anyApi.game.requestJob);
  const clickBoostJob = useMutation(anyApi.game.clickBoostJob);
  const purchaseUpgrade = useMutation(anyApi.game.purchaseUpgrade);
  const setTestAcceleration = useMutation(anyApi.game.setTestAcceleration);

  const [sessionId, setSessionId] = useState("");
  const [nickname, setNickname] = useState("");
  const [now, setNow] = useState(() => Date.now());
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showTestControls, setShowTestControls] = useState(false);

  useEffect(() => {
    if (!error) return;
    const t = setTimeout(() => setError(null), 4000);
    return () => clearTimeout(t);
  }, [error]);

  useEffect(() => {
    try {
      const storedSession =
        localStorage.getItem("cat_idle_session") || createSessionId();
      const storedName =
        localStorage.getItem("cat_idle_nickname") || "Guest Cat";

      localStorage.setItem("cat_idle_session", storedSession);
      localStorage.setItem("cat_idle_nickname", storedName);

      setSessionId(storedSession);
      setNickname(storedName);
    } catch {
      // localStorage unavailable (private browsing, etc.) — use ephemeral session
      setSessionId(createSessionId());
      setNickname("Guest Cat");
    }
    setShowTestControls(window.location.search.includes("test=1"));
  }, []);

  useEffect(() => {
    if (!sessionId || !nickname) {
      return;
    }

    ensureGlobalState({}).catch((err) =>
      console.warn("ensureGlobalState failed:", err),
    );
    upsertPresence({ sessionId, nickname }).catch((err) =>
      console.warn("upsertPresence failed:", err),
    );

    const heartbeat = setInterval(() => {
      upsertPresence({ sessionId, nickname }).catch((err) =>
        console.warn("upsertPresence heartbeat failed:", err),
      );
    }, 30_000);

    return () => clearInterval(heartbeat);
  }, [sessionId, nickname, ensureGlobalState, upsertPresence]);

  useEffect(() => {
    const interval = setInterval(() => {
      setNow(Date.now());
    }, 1000);

    return () => clearInterval(interval);
  }, []);

  const colony = dashboard?.colony;
  const jobs = dashboard?.jobs ?? [];
  const upgrades = dashboard?.upgrades ?? [];
  const events = dashboard?.events ?? [];
  const cats = dashboard?.cats ?? [];

  const ritualPoints = colony?.globalUpgradePoints ?? 0;
  const accelerationPreset = useMemo(() => {
    return presetFromTimeScale(colony?.testTimeScale);
  }, [colony?.testTimeScale]);

  async function runAction<T>(
    actionKey: string,
    fn: () => Promise<T>,
  ): Promise<T | undefined> {
    setError(null);
    setBusyAction(actionKey);
    try {
      const result = await fn();
      const asRecord =
        typeof result === "object" && result !== null
          ? (result as Record<string, unknown>)
          : null;
      if (asRecord && asRecord.ok === false) {
        setError(
          typeof asRecord.message === "string"
            ? (asRecord.message as string)
            : "The action failed. Please try again.",
        );
      }
      return result;
    } catch (err: unknown) {
      const isNetwork =
        err instanceof TypeError && /fetch|network/i.test(err.message);
      if (isNetwork) {
        console.warn(`Action ${actionKey} network error:`, err);
        setError("Network error — check your connection and try again.");
      } else {
        console.error(`Action ${actionKey} failed:`, err);
        setError(cleanErrorMessage(err));
      }
      return undefined;
    } finally {
      setBusyAction(null);
    }
  }

  const submitJob = async (kind: JobKind) => {
    if (!sessionId || !nickname) {
      return;
    }
    await runAction(kind, () => requestJob({ sessionId, nickname, kind }));
  };

  const onBoostJob = async (jobId: string) => {
    if (!sessionId || !nickname) {
      return;
    }
    await runAction(jobId, () => clickBoostJob({ sessionId, nickname, jobId }));
  };

  const onBuyUpgrade = async (key: string) => {
    if (!sessionId || !nickname) {
      return;
    }
    await runAction(`upgrade:${key}`, () =>
      purchaseUpgrade({ sessionId, nickname, key }),
    );
  };

  const onSetAcceleration = async (preset: "off" | "fast" | "turbo") => {
    await runAction(`accel:${preset}`, () => setTestAcceleration({ preset }));
  };

  const updateNickname = (value: string) => {
    const trimmed = value.trim() || "Guest Cat";
    setNickname(trimmed);
    try {
      localStorage.setItem("cat_idle_nickname", trimmed);
    } catch {
      // localStorage unavailable — nickname persists in memory only
    }
    if (sessionId) {
      upsertPresence({ sessionId, nickname: trimmed }).catch((err) =>
        console.warn("upsertPresence failed:", err),
      );
    }
  };

  const statusTone = useMemo(() => {
    const status = colony?.status ?? "starting";
    if (status === "thriving") return "thriving" as const;
    if (status === "struggling") return "struggling" as const;
    if (status === "dead") return "dead" as const;
    return "starting" as const;
  }, [colony?.status]);

  return {
    // Raw data
    dashboard,
    colony,
    jobs,
    upgrades,
    events,
    cats,
    ritualPoints,
    accelerationPreset,
    statusTone,
    now,

    // Session
    sessionId,
    nickname,
    showTestControls,

    // State
    busyAction,
    error,

    // Actions
    submitJob,
    onBoostJob,
    onBuyUpgrade,
    onSetAcceleration,
    updateNickname,
    ensureGlobalState,

    // Leader
    leader: dashboard?.leader ?? null,
    onlineCount: dashboard?.onlineCount ?? 0,
  };
}
