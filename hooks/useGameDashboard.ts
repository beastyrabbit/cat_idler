"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

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
	const [sig, setSig] = useState("");
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

	// Persist the server-signed identity returned by the presence action.
	// Legacy (unsigned) sessions have no stored sig; presence upgrades them.
	const applyIdentity = useCallback((result: unknown) => {
		if (!result || typeof result !== "object") return;
		const record = result as { sessionId?: unknown; sig?: unknown };
		if (typeof record.sessionId === "string" && record.sessionId) {
			setSessionId(record.sessionId);
			try {
				localStorage.setItem("cat_idle_session", record.sessionId);
			} catch {
				// localStorage unavailable — identity persists in memory only
			}
		}
		if (typeof record.sig === "string" && record.sig) {
			setSig(record.sig);
			try {
				localStorage.setItem("cat_idle_sig", record.sig);
			} catch {
				// localStorage unavailable — identity persists in memory only
			}
		}
	}, []);

	useEffect(() => {
		try {
			const storedSession =
				localStorage.getItem("cat_idle_session") || createSessionId();
			const storedName =
				localStorage.getItem("cat_idle_nickname") || "Guest Cat";
			const storedSig = localStorage.getItem("cat_idle_sig") || "";

			localStorage.setItem("cat_idle_session", storedSession);
			localStorage.setItem("cat_idle_nickname", storedName);

			setSessionId(storedSession);
			setNickname(storedName);
			setSig(storedSig);
		} catch {
			// localStorage unavailable (private browsing, etc.) — use ephemeral session
			setSessionId(createSessionId());
			setNickname("Guest Cat");
		}
		// Visible with ?test=1, or always during local development.
		setShowTestControls(
			window.location.search.includes("test=1") ||
				process.env.NODE_ENV === "development",
		);
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
		if (!sessionId || !nickname || !sig) {
			return;
		}

		postAction("presence", { sessionId, nickname, sig })
			.then(applyIdentity)
			.catch((err) => console.warn("presence failed:", err));

		const heartbeat = setInterval(() => {
			postAction("presence", { sessionId, nickname, sig })
				.then(applyIdentity)
				.catch((err) => console.warn("presence heartbeat failed:", err));
		}, 30_000);

		return () => clearInterval(heartbeat);
	}, [sessionId, nickname, sig, applyIdentity]);

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
				const message =
					typeof asRecord.message === "string"
						? asRecord.message
						: typeof asRecord.reason === "string"
							? `Not possible right now: ${(asRecord.reason as string).replaceAll("_", " ")}.`
							: "The action failed. Please try again.";
				setError(message);
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
		if (!sessionId || !nickname || !sig) {
			return;
		}
		await runAction(kind, () =>
			postAction("requestJob", { sessionId, nickname, sig, kind }),
		);
	};

	const onBoostJob = async (jobId: string) => {
		if (!sessionId || !nickname || !sig) {
			return;
		}
		await runAction(jobId, () =>
			postAction("boost", { sessionId, nickname, sig, jobId }),
		);
	};

	const onBuyUpgrade = async (key: string) => {
		if (!sessionId || !nickname || !sig) {
			return;
		}
		await runAction(`upgrade:${key}`, () =>
			postAction("purchaseUpgrade", { sessionId, nickname, sig, key }),
		);
	};

	const onCastVote = async (electionId: string, catId: string) => {
		if (!sessionId || !nickname || !sig) {
			return;
		}
		await runAction(`vote:${electionId}`, () =>
			postAction("castVote", { sessionId, nickname, sig, electionId, catId }),
		);
	};

	const onRequestVoteKick = async () => {
		if (!sessionId || !nickname || !sig) {
			return;
		}
		await runAction("voteKick", () =>
			postAction("requestVoteKick", { sessionId, nickname, sig }),
		);
	};

	const onCreateZone = async (
		kind: "avoid" | "gather",
		a: { x: number; y: number },
		b: { x: number; y: number },
		durationMs: number,
	) => {
		if (!sessionId || !nickname || !sig) {
			return;
		}
		await runAction("createZone", () =>
			postAction("createZone", {
				sessionId,
				nickname,
				sig,
				kind,
				a,
				b,
				durationMs,
			}),
		);
	};

	const onRemoveZone = async (zoneId: string) => {
		if (!sessionId || !nickname || !sig) {
			return;
		}
		await runAction(`removeZone:${zoneId}`, () =>
			postAction("removeZone", { sessionId, nickname, sig, zoneId }),
		);
	};

	const onBuildRoad = async (
		a: { x: number; y: number },
		b: { x: number; y: number },
	) => {
		if (!sessionId || !nickname || !sig) {
			return;
		}
		await runAction("buildRoad", () =>
			postAction("buildRoad", { sessionId, nickname, sig, a, b }),
		);
	};

	const onPlanBuilding = async (
		type:
			| "workshop"
			| "field"
			| "research_hut"
			| "school"
			| "smithy"
			| "barracks",
	) => {
		if (!sessionId || !nickname || !sig) {
			return;
		}
		await runAction(`build:${type}`, () =>
			postAction("planBuilding", { sessionId, nickname, sig, type }),
		);
	};

	const onTrainWarrior = async (catId?: string | null) => {
		if (!sessionId || !nickname || !sig) {
			return;
		}
		await runAction("trainWarrior", () =>
			postAction("trainWarrior", {
				sessionId,
				nickname,
				sig,
				catId: catId ?? null,
			}),
		);
	};

	const onDefendRaid = async () => {
		if (!sessionId || !nickname || !sig) {
			return;
		}
		// Defense clicks are frequent — key by "defend" so the button stays live.
		await runAction("defend", () =>
			postAction("defendRaid", { sessionId, nickname, sig }),
		);
	};

	const onUnlockNode = async (nodeId: string) => {
		if (!sessionId || !nickname || !sig) {
			return;
		}
		await runAction(`unlock:${nodeId}`, () =>
			postAction("unlockNode", { sessionId, nickname, sig, nodeId }),
		);
	};

	const onAssignWorker = async (catId: string, buildingId: string | null) => {
		if (!sessionId || !nickname || !sig) {
			return;
		}
		await runAction(`assign:${buildingId ?? "none"}`, () =>
			postAction("assignWorker", {
				sessionId,
				nickname,
				sig,
				catId,
				buildingId,
			}),
		);
	};

	const onSetAcceleration = async (
		preset: "off" | "fast" | "turbo" | "hyper" | "ludicrous",
	) => {
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
			postAction("presence", { sessionId, nickname: trimmed, sig })
				.then(applyIdentity)
				.catch((err) => console.warn("presence failed:", err));
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

		// Zones (Phase 5)
		zones: dashboard?.zones ?? [],
		onCreateZone,
		onRemoveZone,

		// Production (Phase 7)
		onPlanBuilding,
		onAssignWorker,
		onBuildRoad,

		// Research & upgrade tree
		research: dashboard?.research ?? null,
		onUnlockNode,

		// Military (Roadmap 4)
		threat: dashboard?.threat ?? null,
		raiders: dashboard?.raiders ?? [],
		onTrainWarrior,
		onDefendRaid,

		// Connection
		connectionLost,
	};
}
