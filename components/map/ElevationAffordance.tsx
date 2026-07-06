"use client";

import { ELEVATION_DIR } from "@/lib/game/elevation";
import { DIAMOND_CLIP, ISO } from "./constants";

const EDGE_CLIP: Record<keyof typeof ELEVATION_DIR, string> = {
	N: "polygon(50% 0%, 100% 50%, 100% 58%, 50% 8%)",
	E: "polygon(100% 50%, 50% 100%, 50% 92%, 100% 42%)",
	S: "polygon(50% 100%, 0% 50%, 0% 42%, 50% 92%)",
	W: "polygon(0% 50%, 50% 0%, 50% 8%, 0% 58%)",
};

const EDGE_GRADIENT: Record<keyof typeof ELEVATION_DIR, string> = {
	N: "linear-gradient(32deg, rgba(28,20,11,0.72), rgba(255,239,190,0.42) 44%, rgba(28,20,11,0.2))",
	E: "linear-gradient(148deg, rgba(28,20,11,0.72), rgba(255,239,190,0.38) 44%, rgba(28,20,11,0.22))",
	S: "linear-gradient(32deg, rgba(255,239,190,0.35), rgba(28,20,11,0.68) 54%, rgba(28,20,11,0.28))",
	W: "linear-gradient(148deg, rgba(255,239,190,0.36), rgba(28,20,11,0.68) 54%, rgba(28,20,11,0.28))",
};

const STAIR_GRADIENT =
	"repeating-linear-gradient(90deg, rgba(246,232,190,0.9) 0 5px, rgba(77,54,30,0.85) 5px 8px, rgba(246,232,190,0.9) 8px 13px)";

function maskHas(mask: number, dir: keyof typeof ELEVATION_DIR): boolean {
	return (mask & ELEVATION_DIR[dir]) !== 0;
}

export function ElevationAffordance({
	left,
	top,
	zIndex,
	cliffMask,
	stairMask,
	dim = 1,
}: {
	left: number;
	top: number;
	zIndex: number;
	cliffMask: number;
	stairMask: number;
	dim?: number;
}) {
	if (cliffMask === 0 && stairMask === 0) {
		return null;
	}
	const opacity = dim < 1 ? 0.55 : 0.92;
	return (
		<>
			{(["N", "E", "S", "W"] as const).map((dir) =>
				maskHas(cliffMask, dir) ? (
					<div
						key={`cliff-${dir}`}
						className="pointer-events-none absolute"
						style={{
							left,
							top,
							width: ISO.tileWidth,
							height: ISO.tileHeight,
							zIndex,
							clipPath: EDGE_CLIP[dir],
							background: EDGE_GRADIENT[dir],
							mixBlendMode: "multiply",
							opacity,
						}}
					/>
				) : null,
			)}
			{(["N", "E", "S", "W"] as const).map((dir) =>
				maskHas(stairMask, dir) ? (
					<div
						key={`stair-${dir}`}
						className="pointer-events-none absolute"
						style={{
							left,
							top,
							width: ISO.tileWidth,
							height: ISO.tileHeight,
							zIndex: zIndex + 1,
							clipPath: EDGE_CLIP[dir],
							background: STAIR_GRADIENT,
							filter: "drop-shadow(0 1px 1px rgba(30,20,10,0.6))",
							opacity: dim < 1 ? 0.65 : 0.95,
						}}
					/>
				) : null,
			)}
		</>
	);
}

export function ElevationBaseTile({
	left,
	top,
	zIndex,
	title,
}: {
	left: number;
	top: number;
	zIndex: number;
	title?: string;
}) {
	return (
		<div
			title={title}
			className="absolute"
			style={{
				left,
				top,
				width: ISO.tileWidth,
				height: ISO.tileHeight,
				zIndex,
				clipPath: DIAMOND_CLIP,
				background: "#8aa37b",
			}}
		/>
	);
}
