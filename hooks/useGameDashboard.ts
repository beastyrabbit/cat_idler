"use client";

import { useEffect, useMemo, useRef, useState } from "react";

import { presetFromTimeScale } from "@/lib/game/testAcceleration";

function createSessionId(): string {
	return `session_${Math.random().toString(36).slice(2)}_${Date.now()}`;
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

async function postAction<T = unknown>(
	action: string,
	payload: Record<string, unknown> = {},
): Promise<T> {
	const response = await fetch("/api/game/actions", {
		method: "POST",
		headers: { "Content-Type": "application/json" },
		body: JSON.stringify({ action, ...payload }),
	});

	const result = (await response.json().catch(() => null)) as T | null;

	if (!response.ok) {
		const message =
			result && typeof result === "object" && "message" in result
				? String((result as { message: unknown }).message)
				: `Request failed (${response.status})`;
		throw new Error(message);
	}

	if (result === null) {
		// 2xx with an unparseable body — don't report success on nothing
		throw new Error("Empty response from server.");
	}

	return result;
}

export function useGameDashboard() {
	// undefined = loading, null = no colony yet, object = live data
	const [dashboard, setDashboard] = useState<any>(undefined);

	const [sessionId, setSessionId] = useState("");
	const [nickname, setNickname] = useState("");
	const [now, setNow] = useState(() => Date.now());
	const [busyAction, setBusyAction] = useState<string | null>(null);
	const [error, setError] = useState<string | null>(null);
	const [showTestControls, setShowTestControls] = useState(false);
	const lastFrameAtRef = useRef(0);

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

	// Live dashboard via SSE; EventSource reconnects automatically on drops.
	useEffect(() => {
		const source = new EventSource("/api/game/stream");

		source.addEventListener("dashboard", (event) => {
			try {
				setDashboard(JSON.parse((event as MessageEvent).data));
				lastFrameAtRef.current = Date.now();
			} catch (err) {
				console.warn("Failed to parse dashboard frame:", err);
			}
		});

		source.addEventListener("error", () => {
			// server-sent failure frame (dashboard read failed) — staleness is
			// surfaced via connectionLost while EventSource reconnects
		});

		source.onerror = () => {
			// keep last known state; EventSource retries on its own.
			// connectionLost flags staleness if this persists.
		};

		return () => source.close();
	}, []);

	useEffect(() => {
		if (!sessionId || !nickname) {
			return;
		}

		postAction("presence", { sessionId, nickname }).catch((err) =>
			console.warn("presence failed:", err),
		);

		const heartbeat = setInterval(() => {
			postAction("presence", { sessionId, nickname }).catch((err) =>
				console.warn("presence heartbeat failed:", err),
			);
		}, 30_000);

		return () => clearInterval(heartbeat);
	}, [sessionId, nickname]);

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

	// Stale-data indicator: frames normally arrive every second; if none
	// landed for a while the stream is down and the data on screen is old.
	const connectionLost =
		lastFrameAtRef.current > 0 && now - lastFrameAtRef.current > 5000;

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
				setError(err instanceof Error ? err.message : String(err));
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
		await runAction(kind, () =>
			postAction("requestJob", { sessionId, nickname, kind }),
		);
	};

	const onBoostJob = async (jobId: string) => {
		if (!sessionId || !nickname) {
			return;
		}
		await runAction(jobId, () =>
			postAction("boost", { sessionId, nickname, jobId }),
		);
	};

	const onBuyUpgrade = async (key: string) => {
		if (!sessionId || !nickname) {
			return;
		}
		await runAction(`upgrade:${key}`, () =>
			postAction("purchaseUpgrade", { sessionId, nickname, key }),
		);
	};

	const onCastVote = async (electionId: string, catId: string) => {
		if (!sessionId || !nickname) {
			return;
		}
		await runAction(`vote:${electionId}`, () =>
			postAction("castVote", { sessionId, nickname, electionId, catId }),
		);
	};

	const onRequestVoteKick = async () => {
		if (!sessionId || !nickname) {
			return;
		}
		await runAction("voteKick", () =>
			postAction("requestVoteKick", { sessionId, nickname }),
		);
	};

	const onSetAcceleration = async (preset: "off" | "fast" | "turbo") => {
		await runAction(`accel:${preset}`, () =>
			postAction("setTestAcceleration", { preset }),
		);
	};

	const onAdvanceTime = async (seconds: number) => {
		await runAction(`advance:${seconds}`, () =>
			postAction("advanceTime", { seconds }),
		);
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
			postAction("presence", { sessionId, nickname: trimmed }).catch((err) =>
				console.warn("presence failed:", err),
			);
		}
	};

	const ensureGlobalState = async () => {
		return postAction("ensure");
	};

	const statusTone = (colony?.status ?? "starting") as
		| "thriving"
		| "struggling"
		| "dead"
		| "starting";

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
		onAdvanceTime,
		updateNickname,
		ensureGlobalState,

		// Leader
		leader: dashboard?.leader ?? null,
		onlineCount: dashboard?.onlineCount ?? 0,

		// Map (Phase 1: shrine + buildings on the world map)
		buildings: dashboard?.buildings ?? [],
		anchor: dashboard?.anchor ?? { x: 6, y: 6 },
		housing: dashboard?.housing ?? null,

		// Elections (Phase 4)
		election: dashboard?.election ?? null,
		voteKick: dashboard?.voteKick ?? null,
		onCastVote,
		onRequestVoteKick,

		// Connection
		connectionLost,
	};
}
