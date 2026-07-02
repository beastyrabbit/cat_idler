"use client";

import { tileToIso, zIndexFor } from "@/lib/game/isoProjection";
import { DIAMOND_CLIP, ISO } from "./constants";

export interface MapZone {
	_id: string;
	kind: "avoid" | "gather";
	x1: number;
	y1: number;
	x2: number;
	y2: number;
	expiresAt: number;
}

interface ZoneLayerProps {
	zones: MapZone[];
	now: number;
	onRemove: (zoneId: string) => void;
	/** Pending first corner while drawing a new zone. */
	draftCorner?: { x: number; y: number } | null;
}

const ZONE_STYLE: Record<MapZone["kind"], string> = {
	avoid: "rgba(220, 60, 60, 0.35)",
	gather: "rgba(60, 180, 90, 0.35)",
};

function ZoneTiles({
	zone,
	now,
	onRemove,
}: {
	zone: MapZone;
	now: number;
	onRemove: (zoneId: string) => void;
}) {
	const minutesLeft = Math.max(0, Math.ceil((zone.expiresAt - now) / 60_000));
	const title = `${zone.kind === "avoid" ? "🚫 Avoid" : "📍 Gather"} zone — ${minutesLeft}m left (click to remove your own)`;
	const tiles = [];
	for (let y = zone.y1; y <= zone.y2; y++) {
		for (let x = zone.x1; x <= zone.x2; x++) {
			const { left, top } = tileToIso(x, y, ISO);
			tiles.push(
				<button
					key={`${zone._id}:${x},${y}`}
					type="button"
					aria-label={title}
					title={title}
					onClick={() => onRemove(zone._id)}
					className="absolute cursor-pointer border-0 p-0"
					style={{
						left,
						top,
						width: ISO.tileWidth,
						height: ISO.tileHeight,
						zIndex: zIndexFor(x, y, "object", ISO),
						clipPath: DIAMOND_CLIP,
						background: ZONE_STYLE[zone.kind],
					}}
				/>,
			);
		}
	}
	return <>{tiles}</>;
}

export function ZoneLayer({
	zones,
	now,
	onRemove,
	draftCorner,
}: ZoneLayerProps) {
	return (
		<>
			{zones.map((zone) => (
				<ZoneTiles key={zone._id} zone={zone} now={now} onRemove={onRemove} />
			))}
			{draftCorner && (
				<div
					className="pointer-events-none absolute animate-pulse"
					style={{
						left: tileToIso(draftCorner.x, draftCorner.y, ISO).left,
						top: tileToIso(draftCorner.x, draftCorner.y, ISO).top,
						width: ISO.tileWidth,
						height: ISO.tileHeight,
						zIndex: zIndexFor(draftCorner.x, draftCorner.y, "object", ISO) + 2,
						clipPath: DIAMOND_CLIP,
						background: "rgba(250, 200, 60, 0.6)",
					}}
				/>
			)}
		</>
	);
}
