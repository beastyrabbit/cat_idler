"use client";

/**
 * /game/pixi — the PixiJS renderer spike (docs/ENGINE_FRONTEND.md verdict),
 * standing side-by-side with the DOM map at /game. Nothing about /game changes.
 *
 * The renderer is browser-only (WebGL), so it loads behind a client-side
 * `dynamic(..., { ssr:false })` boundary — Next never tries to render Pixi on
 * the server.
 */

import dynamic from "next/dynamic";

const PixiMapScreen = dynamic(() => import("@/lib/render/pixi/PixiMapScreen"), {
	ssr: false,
});

export default function GamePixiPage() {
	return <PixiMapScreen />;
}
