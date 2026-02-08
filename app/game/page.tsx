"use client";

import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery } from "convex/react";

import { api } from "@/convex/_generated/api";
import { summarizeCatIdentity } from "@/lib/game/catTraits";
import { presetFromTimeScale } from "@/lib/game/testAcceleration";

const anyApi = api as any;

function createSessionId(): string {
  return `session_${Math.random().toString(36).slice(2)}_${Date.now()}`;
}

function cleanErrorMessage(err: unknown): string {
  const raw = (err as Error).message ?? String(err);
  const match = raw.match(/Uncaught Error: (.+?)(?:\s+at\s|$)/);
  return match ? match[1].trim() : raw;
}

function formatDuration(ms: number): string {
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

export default function GamePage() {
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
    const storedSession =
      localStorage.getItem("cat_idle_session") || createSessionId();
    const storedName = localStorage.getItem("cat_idle_nickname") || "Guest Cat";

    localStorage.setItem("cat_idle_session", storedSession);
    localStorage.setItem("cat_idle_nickname", storedName);

    setSessionId(storedSession);
    setNickname(storedName);
    setShowTestControls(window.location.search.includes("test=1"));
  }, []);

  useEffect(() => {
    if (!sessionId || !nickname) {
      return;
    }

    void ensureGlobalState({});
    void upsertPresence({ sessionId, nickname });

    const heartbeat = setInterval(() => {
      void upsertPresence({ sessionId, nickname });
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
    } catch (err) {
      console.error(`Action ${actionKey} failed:`, err);
      setError(cleanErrorMessage(err));
      return undefined;
    } finally {
      setBusyAction(null);
    }
  }

  const submitJob = async (
    kind:
      | "supply_food"
      | "supply_water"
      | "leader_plan_hunt"
      | "leader_plan_house"
      | "ritual",
  ) => {
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
    localStorage.setItem("cat_idle_nickname", trimmed);
    if (sessionId) {
      void upsertPresence({ sessionId, nickname: trimmed });
    }
  };

  const statusTone = useMemo(() => {
    const status = colony?.status ?? "starting";
    if (status === "thriving") return "status-thriving";
    if (status === "struggling") return "status-struggling";
    if (status === "dead") return "status-dead";
    return "status-starting";
  }, [colony?.status]);

  if (dashboard === undefined) {
    return (
      <main className="idle-shell">
        <section className="idle-hero card">
          <h1>Preparing Global Colony...</h1>
          <p className="muted">Loading shared state.</p>
        </section>
      </main>
    );
  }

  if (!dashboard || !colony) {
    return (
      <main className="idle-shell">
        <section className="idle-hero card">
          <h1>Global Colony Not Ready</h1>
          <button
            className="btn btn-primary"
            onClick={() => void ensureGlobalState({})}
          >
            Initialize Colony
          </button>
        </section>
      </main>
    );
  }

  return (
    <main className="idle-shell">
      <section className="idle-topbar card">
        <div>
          <p className="eyebrow">Shared Idle World</p>
          <h1>{colony.name}</h1>
          <p className={`status-pill ${statusTone}`}>
            Run #{colony.runNumber ?? 1} · {colony.status}
          </p>
        </div>

        <div className="presence">
          <label htmlFor="nickname" className="presence-label">
            Nickname
          </label>
          <input
            id="nickname"
            className="input"
            defaultValue={nickname}
            onBlur={(e) => updateNickname(e.target.value)}
          />
          <p className="muted">Online now: {dashboard.onlineCount}</p>
        </div>
      </section>

      <section className="resource-strip card">
        <article>
          <span>Food</span>
          <strong>{Math.floor(colony.resources.food)}</strong>
        </article>
        <article>
          <span>Water</span>
          <strong>{Math.floor(colony.resources.water)}</strong>
        </article>
        <article>
          <span>Herbs</span>
          <strong>{Math.floor(colony.resources.herbs)}</strong>
        </article>
        <article>
          <span>Materials</span>
          <strong>{Math.floor(colony.resources.materials)}</strong>
        </article>
        <article>
          <span>Ritual Points</span>
          <strong>{Math.floor(ritualPoints)}</strong>
        </article>
      </section>

      {error ? <p className="error-line">{error}</p> : null}

      <section className="idle-grid">
        <div className="left-stack">
          <section className="card panel">
            <h2>Player Actions</h2>
            <p className="muted">Short jobs for immediate support.</p>
            <div className="action-row">
              <button
                className="btn btn-primary"
                disabled={busyAction === "supply_food"}
                onClick={() => void submitJob("supply_food")}
              >
                + Supply Food (20s)
              </button>
              <button
                className="btn btn-primary"
                disabled={busyAction === "supply_water"}
                onClick={() => void submitJob("supply_water")}
              >
                + Supply Water (15s)
              </button>
            </div>

            <p className="muted">Leader-planned long jobs.</p>
            <div className="action-row">
              <button
                className="btn btn-secondary"
                disabled={busyAction === "leader_plan_hunt"}
                onClick={() => void submitJob("leader_plan_hunt")}
              >
                Request Hunt (plan + expedition)
              </button>
              <button
                className="btn btn-secondary"
                disabled={busyAction === "leader_plan_house"}
                onClick={() => void submitJob("leader_plan_house")}
              >
                Request House Build
              </button>
              <button
                className="btn btn-secondary"
                disabled={busyAction === "ritual"}
                onClick={() => void submitJob("ritual")}
              >
                Request Ritual
              </button>
            </div>

            {showTestControls ? (
              <>
                <p className="muted">Test acceleration mode (for QA).</p>
                <div className="action-row">
                  <button
                    className="btn btn-secondary"
                    disabled={busyAction === "accel:off"}
                    onClick={() => void onSetAcceleration("off")}
                  >
                    Speed: Off
                  </button>
                  <button
                    className="btn btn-secondary"
                    disabled={busyAction === "accel:fast"}
                    onClick={() => void onSetAcceleration("fast")}
                  >
                    Speed: Fast
                  </button>
                  <button
                    className="btn btn-secondary"
                    disabled={busyAction === "accel:turbo"}
                    onClick={() => void onSetAcceleration("turbo")}
                  >
                    Speed: Turbo
                  </button>
                </div>
                <p className="muted">Current mode: {accelerationPreset}</p>
              </>
            ) : null}
          </section>

          <section className="card panel">
            <h2>Active Jobs</h2>
            <div className="job-list">
              {jobs.length === 0 ? (
                <p className="muted">No queued or active jobs.</p>
              ) : null}
              {jobs.map((job: any) => {
                const totalMs = Math.max(1, job.endsAt - job.startedAt);
                const doneMs = Math.max(
                  0,
                  Math.min(totalMs, now - job.startedAt),
                );
                const pct = Math.floor((doneMs / totalMs) * 100);
                const remaining = formatDuration(job.endsAt - now);

                return (
                  <article key={job._id} className="job-item">
                    <div className="job-head">
                      <div>
                        <h3>{job.kind.replace(/_/g, " ")}</h3>
                        <p className="muted">
                          {job.status} · ETA {remaining}
                        </p>
                      </div>
                      <button
                        className="btn btn-secondary"
                        disabled={
                          busyAction === job._id || job.status === "completed"
                        }
                        onClick={() => void onBoostJob(job._id)}
                      >
                        Boost -10s
                      </button>
                    </div>
                    <div className="progress-rail">
                      <div
                        className="progress-fill"
                        style={{ width: `${pct}%` }}
                      />
                    </div>
                  </article>
                );
              })}
            </div>
          </section>
        </div>

        <div className="right-stack">
          <section className="card panel">
            <h2>Leader + Cats</h2>
            {dashboard.leader ? (
              <p className="muted">
                Leader: <strong>{dashboard.leader.name}</strong> · leadership{" "}
                {Math.floor(dashboard.leader.stats.leadership)}
              </p>
            ) : (
              <p className="muted">Leader will be auto-selected.</p>
            )}
            <div className="cat-list">
              {cats.map((cat: any) => {
                const appearance = summarizeCatIdentity(
                  cat.spriteParams as Record<string, unknown> | null,
                );
                const roleXp = cat.roleXp ?? {
                  hunter: 0,
                  architect: 0,
                  ritualist: 0,
                };

                return (
                  <article key={cat._id} className="cat-item cat-item-rich">
                    <div className="cat-main">
                      <strong>{cat.name}</strong>
                      <p className="muted">
                        Spec: {cat.specialization ?? "none"}
                      </p>
                      <p className="muted cat-species">
                        Species {appearance.species} · Sprite{" "}
                        {appearance.sprite}
                      </p>
                      <p className="muted cat-traits">
                        Lineage {appearance.lineage} · Coat {appearance.coat} ·
                        Eyes {appearance.eyes} · Marks {appearance.markings}
                      </p>
                      <p className="muted cat-appearance-extra">
                        Skin {appearance.skin} · Accessories{" "}
                        {appearance.accessories} · Scars {appearance.scars}
                      </p>
                      <p className="muted cat-role-xp">
                        Role XP H {roleXp.hunter} · A {roleXp.architect} · R{" "}
                        {roleXp.ritualist}
                      </p>
                    </div>
                    <div className="cat-stats cat-stats-grid">
                      <span>ATK {Math.floor(cat.stats.attack)}</span>
                      <span>DEF {Math.floor(cat.stats.defense)}</span>
                      <span>HUNT {Math.floor(cat.stats.hunting)}</span>
                      <span>MED {Math.floor(cat.stats.medicine)}</span>
                      <span>CLEAN {Math.floor(cat.stats.cleaning)}</span>
                      <span>BUILD {Math.floor(cat.stats.building)}</span>
                      <span>LEAD {Math.floor(cat.stats.leadership)}</span>
                      <span>VIS {Math.floor(cat.stats.vision)}</span>
                    </div>
                  </article>
                );
              })}
            </div>
          </section>

          <section className="card panel">
            <h2>Global Upgrades</h2>
            <p className="muted">
              Any player can spend ritual points for permanent run bonuses.
            </p>
            <div className="upgrade-list">
              {upgrades.map((upgrade: any) => {
                const cost = upgrade.baseCost * (upgrade.level + 1);
                const maxed = upgrade.level >= upgrade.maxLevel;
                return (
                  <article key={upgrade._id} className="upgrade-item">
                    <div>
                      <h3>{upgrade.key.replace(/_/g, " ")}</h3>
                      <p className="muted">
                        Lv {upgrade.level}/{upgrade.maxLevel} · Cost {cost}
                      </p>
                    </div>
                    <button
                      className="btn btn-primary"
                      disabled={
                        maxed ||
                        ritualPoints < cost ||
                        busyAction === `upgrade:${upgrade.key}`
                      }
                      onClick={() => void onBuyUpgrade(upgrade.key)}
                    >
                      {maxed ? "Maxed" : "Upgrade"}
                    </button>
                  </article>
                );
              })}
            </div>
          </section>

          <section className="card panel">
            <h2>Event Feed</h2>
            <div className="event-list">
              {events.length === 0 ? (
                <p className="muted">No events yet.</p>
              ) : null}
              {events.map((event: any) => (
                <article key={event._id} className="event-item">
                  <span>{new Date(event.timestamp).toLocaleTimeString()}</span>
                  <p>{event.message}</p>
                </article>
              ))}
            </div>
          </section>
        </div>
      </section>
    </main>
  );
}
