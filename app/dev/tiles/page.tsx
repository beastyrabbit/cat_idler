"use client";

/**
 * Isometric Nature tile explorer (dev-only, no game imports).
 *
 * Renders every object group in the Kenney "Isometric Nature" pack as a
 * labeled row of its four rotations so the opaque `naturePack_NNN_R` numbering
 * can be identified by eye. Each group has a free-text note (localStorage-
 * persisted) for annotations like "cliff-N-edge" or "stairs-up-east", and the
 * whole annotation set exports to JSON for handoff into worldgen/tile config.
 *
 * Route: /dev/tiles  (not linked from the game; safe to delete)
 */

import {
	useCallback,
	useMemo,
	useRef,
	useState,
	useSyncExternalStore,
} from "react";

/** Public path to the Isometric Nature PNGs (spaces are URL-encoded on use). */
const NATURE_BASE =
	"/Kenney Game Assets All-in-1 3.5.0/2D assets/Isometric Nature/PNG";

/** localStorage key for the annotation map ({ groupId: note }). */
const STORAGE_KEY = "catidler.tileNotes.v1";
const NOTES_CHANGED_EVENT = "catidler.tileNotes.changed";

/** Source canvas is 220x379; the ground diamond is 180x115 near the bottom. */
const CANVAS_W = 220;
const CANVAS_H = 379;

const ROTATIONS = [0, 1, 2, 3] as const;

function pad3(n: number): string {
	return String(n).padStart(3, "0");
}

/** Flat ground tiles first (most useful for worldgen), then the numeric groups. */
const FLAT_GROUPS = Array.from(
	{ length: 13 },
	(_, i) => `naturePack_flat_${pad3(i + 1)}`,
);
const NUMERIC_GROUPS = Array.from(
	{ length: 175 },
	(_, i) => `naturePack_${pad3(i + 1)}`,
);
const ALL_GROUPS = [...FLAT_GROUPS, ...NUMERIC_GROUPS];

function spriteSrc(group: string, rotation: number): string {
	return encodeURI(`${NATURE_BASE}/${group}_${rotation}.png`);
}

type Notes = Record<string, string>;

let memoryNotesSnapshot = "{}";

function parseNotesSnapshot(snapshot: string): Notes {
	try {
		const parsed = JSON.parse(snapshot);
		return parsed && typeof parsed === "object" && !Array.isArray(parsed)
			? (parsed as Notes)
			: {};
	} catch {
		return {};
	}
}

function notesSnapshotFromStorage(): string {
	if (typeof window === "undefined") return "{}";
	try {
		const raw = window.localStorage.getItem(STORAGE_KEY);
		return raw ?? "{}";
	} catch {
		return memoryNotesSnapshot;
	}
}

function serverNotesSnapshot(): string {
	return "{}";
}

function loadNotes(): Notes {
	return parseNotesSnapshot(notesSnapshotFromStorage());
}

function saveNotes(notes: Notes): void {
	const snapshot = JSON.stringify(notes);
	memoryNotesSnapshot = snapshot;
	if (typeof window === "undefined") return;
	try {
		window.localStorage.setItem(STORAGE_KEY, snapshot);
	} catch {
		// Storage full or blocked — annotations stay in-memory for the session.
	}
	window.dispatchEvent(new Event(NOTES_CHANGED_EVENT));
}

function subscribeNotes(onStoreChange: () => void): () => void {
	if (typeof window === "undefined") return () => {};

	const onStorage = (event: StorageEvent) => {
		if (event.key === STORAGE_KEY) {
			onStoreChange();
		}
	};
	window.addEventListener("storage", onStorage);
	window.addEventListener(NOTES_CHANGED_EVENT, onStoreChange);

	return () => {
		window.removeEventListener("storage", onStorage);
		window.removeEventListener(NOTES_CHANGED_EVENT, onStoreChange);
	};
}

/** Thumbnail scale multiplier applied to the 220x379 source canvas. */
type Scale = 0.4 | 0.55 | 0.7;

const GroupRow = function GroupRow({
	group,
	scale,
	note,
	onNoteChange,
}: {
	group: string;
	scale: Scale;
	note: string;
	onNoteChange: (group: string, value: string) => void;
}) {
	const w = Math.round(CANVAS_W * scale);
	const h = Math.round(CANVAS_H * scale);
	return (
		<div
			id={group}
			className="grid grid-cols-[minmax(0,1fr)_320px] gap-4 border-b border-neutral-800 py-4"
		>
			<div className="flex flex-wrap items-end gap-4">
				{ROTATIONS.map((r) => (
					<figure key={r} className="flex flex-col items-center gap-1">
						{/* Checkerboard backdrop so white cliffs on transparent PNGs stay visible. */}
						<div
							className="flex items-end justify-center rounded"
							style={{
								width: w,
								height: h,
								backgroundColor: "#3a3a3a",
								backgroundImage:
									"linear-gradient(45deg, #444 25%, transparent 25%, transparent 75%, #444 75%, #444), linear-gradient(45deg, #444 25%, transparent 25%, transparent 75%, #444 75%, #444)",
								backgroundSize: "16px 16px",
								backgroundPosition: "0 0, 8px 8px",
							}}
						>
							{/* biome-ignore lint/performance/noImgElement: dev tool renders hundreds of raw pack PNGs; next/image optimization is unwanted here. */}
							{/* eslint-disable-next-line @next/next/no-img-element */}
							<img
								src={spriteSrc(group, r)}
								alt={`${group} rotation ${r}`}
								width={w}
								height={h}
								loading="lazy"
								draggable={false}
								className="select-none"
							/>
						</div>
						<figcaption className="font-mono text-[11px] text-neutral-400">
							{group.replace("naturePack_", "")}_{r}
						</figcaption>
					</figure>
				))}
			</div>
			<div className="flex flex-col gap-1">
				<label
					htmlFor={`note-${group}`}
					className="font-mono text-xs text-neutral-500"
				>
					{group}
				</label>
				<textarea
					id={`note-${group}`}
					value={note}
					onChange={(e) => onNoteChange(group, e.target.value)}
					placeholder="e.g. cliff-N-edge, stairs-up-east, river-start…"
					spellCheck={false}
					className="h-24 w-full resize-y rounded border border-neutral-700 bg-neutral-900 p-2 font-mono text-sm text-neutral-100 outline-none focus:border-emerald-500"
				/>
			</div>
		</div>
	);
};

export default function TileExplorerPage() {
	const notesSnapshot = useSyncExternalStore(
		subscribeNotes,
		notesSnapshotFromStorage,
		serverNotesSnapshot,
	);
	const notes = useMemo(
		() => parseNotesSnapshot(notesSnapshot),
		[notesSnapshot],
	);
	const [query, setQuery] = useState("");
	const [scale, setScale] = useState<Scale>(0.55);
	const [onlyAnnotated, setOnlyAnnotated] = useState(false);
	const [exportOpen, setExportOpen] = useState(false);
	const [exportText, setExportText] = useState("");
	const [status, setStatus] = useState("");
	const statusTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

	const flash = useCallback((message: string) => {
		setStatus(message);
		if (statusTimer.current) clearTimeout(statusTimer.current);
		statusTimer.current = setTimeout(() => setStatus(""), 2500);
	}, []);

	const onNoteChange = useCallback((group: string, value: string) => {
		const next = { ...loadNotes() };
		if (value.trim() === "") {
			delete next[group];
		} else {
			next[group] = value;
		}
		saveNotes(next);
	}, []);

	const annotatedCount = Object.keys(notes).length;

	const visibleGroups = useMemo(() => {
		const q = query.trim().toLowerCase();
		return ALL_GROUPS.filter((group) => {
			if (onlyAnnotated && !notes[group]) return false;
			if (!q) return true;
			if (group.toLowerCase().includes(q)) return true;
			const note = notes[group];
			return note ? note.toLowerCase().includes(q) : false;
		});
	}, [query, onlyAnnotated, notes]);

	const buildExport = useCallback(() => {
		return JSON.stringify(
			{
				pack: "Isometric Nature",
				generatedAt: new Date().toISOString(),
				count: Object.keys(notes).length,
				notes,
			},
			null,
			2,
		);
	}, [notes]);

	const openExport = useCallback(async () => {
		const json = buildExport();
		setExportText(json);
		setExportOpen(true);
		try {
			await navigator.clipboard.writeText(json);
			flash("Annotations copied to clipboard");
		} catch {
			flash("Clipboard blocked — copy from the box below");
		}
	}, [buildExport, flash]);

	const loadFromText = useCallback(() => {
		try {
			const parsed = JSON.parse(exportText);
			const incoming =
				parsed && typeof parsed === "object" && parsed.notes
					? (parsed.notes as Notes)
					: (parsed as Notes);
			if (!incoming || typeof incoming !== "object") {
				flash("No usable notes object found");
				return;
			}
			saveNotes(incoming);
			flash(`Loaded ${Object.keys(incoming).length} annotations`);
		} catch {
			flash("Could not parse JSON");
		}
	}, [exportText, flash]);

	return (
		<div className="min-h-screen bg-neutral-950 text-neutral-100">
			<header className="sticky top-0 z-10 border-b border-neutral-800 bg-neutral-950/95 px-4 py-3 backdrop-blur">
				<div className="flex flex-wrap items-center gap-3">
					<h1 className="text-lg font-semibold">
						Isometric Nature — Tile Explorer
					</h1>
					<span className="text-sm text-neutral-500">
						{ALL_GROUPS.length} groups · {annotatedCount} annotated
					</span>
					<div className="ml-auto flex flex-wrap items-center gap-2">
						<input
							type="search"
							value={query}
							onChange={(e) => setQuery(e.target.value)}
							placeholder="Jump / search (e.g. 042, flat, cliff)"
							className="w-64 rounded border border-neutral-700 bg-neutral-900 px-3 py-1.5 text-sm outline-none focus:border-emerald-500"
						/>
						<label className="flex items-center gap-1.5 text-sm text-neutral-400">
							<input
								type="checkbox"
								checked={onlyAnnotated}
								onChange={(e) => setOnlyAnnotated(e.target.checked)}
							/>
							Annotated only
						</label>
						<select
							value={scale}
							onChange={(e) => setScale(Number(e.target.value) as Scale)}
							className="rounded border border-neutral-700 bg-neutral-900 px-2 py-1.5 text-sm"
							aria-label="Thumbnail size"
						>
							<option value={0.4}>Small</option>
							<option value={0.55}>Medium</option>
							<option value={0.7}>Large</option>
						</select>
						<button
							type="button"
							onClick={openExport}
							className="rounded bg-emerald-600 px-3 py-1.5 text-sm font-medium hover:bg-emerald-500"
						>
							Export annotations
						</button>
					</div>
				</div>
				{status && (
					<div className="mt-2 text-sm text-emerald-400">{status}</div>
				)}
				<p className="mt-2 text-xs text-neutral-500">
					Type a note under any group to tag it (autosaved locally). Rotations
					0–3 are the same object turned N/E/S/W. Filename shown under each
					thumbnail matches{" "}
					<code className="text-neutral-400">naturePack_*.png</code>.
				</p>
			</header>

			{exportOpen && (
				<div className="border-b border-neutral-800 bg-neutral-900 px-4 py-3">
					<div className="mb-2 flex items-center gap-2">
						<span className="text-sm font-medium">Annotations JSON</span>
						<button
							type="button"
							onClick={() => {
								navigator.clipboard
									.writeText(exportText)
									.then(() => flash("Copied"))
									.catch(() => flash("Clipboard blocked"));
							}}
							className="rounded bg-neutral-700 px-2 py-1 text-xs hover:bg-neutral-600"
						>
							Copy
						</button>
						<button
							type="button"
							onClick={loadFromText}
							className="rounded bg-neutral-700 px-2 py-1 text-xs hover:bg-neutral-600"
						>
							Load from box
						</button>
						<button
							type="button"
							onClick={() => setExportOpen(false)}
							className="ml-auto rounded bg-neutral-700 px-2 py-1 text-xs hover:bg-neutral-600"
						>
							Close
						</button>
					</div>
					<textarea
						value={exportText}
						onChange={(e) => setExportText(e.target.value)}
						spellCheck={false}
						className="h-40 w-full resize-y rounded border border-neutral-700 bg-neutral-950 p-2 font-mono text-xs outline-none"
					/>
					<p className="mt-1 text-xs text-neutral-500">
						Paste an exported set here and press “Load from box” to restore
						annotations on another machine.
					</p>
				</div>
			)}

			<main className="px-4 pb-24">
				{visibleGroups.length === 0 ? (
					<p className="py-12 text-center text-neutral-500">
						No groups match “{query}”.
					</p>
				) : (
					visibleGroups.map((group) => (
						<GroupRow
							key={group}
							group={group}
							scale={scale}
							note={notes[group] ?? ""}
							onNoteChange={onNoteChange}
						/>
					))
				)}
			</main>
		</div>
	);
}
