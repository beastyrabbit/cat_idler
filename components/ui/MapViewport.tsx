"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";

type Size = { width: number; height: number };

export interface ViewportView {
	tx: number;
	ty: number;
	scale: number;
	width: number;
	height: number;
}

export interface MapViewportProps {
	/** Pixel size of the content at scale=1 (ex: gridSize * tileSize). */
	contentSize: Size;
	/** Height of the viewport (px number or CSS size, container is full-width). */
	height?: number | string;
	/** Initial zoom level (1 = 100%). */
	initialScale?: number;
	minScale?: number;
	maxScale?: number;
	/** Content-px point to center in the viewport on mount and on reset. */
	initialCenter?: { x: number; y: number };
	/** Reports pan/zoom/viewport-size changes (rAF-throttled). */
	onViewChange?: (view: ViewportView) => void;
	children: React.ReactNode;
}

export function MapViewport({
	contentSize,
	height = 560,
	initialScale = 0.8,
	minScale = 0.4,
	maxScale = 2,
	initialCenter,
	onViewChange,
	children,
}: MapViewportProps) {
	const containerRef = useRef<HTMLDivElement | null>(null);
	const dragRef = useRef<{
		x: number;
		y: number;
		tx: number;
		ty: number;
	} | null>(null);
	const rafRef = useRef<number | null>(null);

	const [scale, setScale] = useState(initialScale);
	const [tx, setTx] = useState(24);
	const [ty, setTy] = useState(24);

	const clamp = (v: number, lo: number, hi: number) =>
		Math.max(lo, Math.min(hi, v));

	const centerView = useCallback(() => {
		if (!initialCenter) return;
		const rect = containerRef.current?.getBoundingClientRect();
		if (!rect) return;
		setTx(rect.width / 2 - initialCenter.x * initialScale);
		setTy(rect.height / 2 - initialCenter.y * initialScale);
	}, [initialCenter, initialScale]);

	// Center on mount (container size is only known after layout).
	useEffect(() => {
		centerView();
		// eslint-disable-next-line react-hooks/exhaustive-deps
	}, []);

	// Report view changes, throttled to animation frames.
	useEffect(() => {
		if (!onViewChange) return;
		if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
		rafRef.current = requestAnimationFrame(() => {
			rafRef.current = null;
			const rect = containerRef.current?.getBoundingClientRect();
			if (!rect) return;
			onViewChange({ tx, ty, scale, width: rect.width, height: rect.height });
		});
		return () => {
			if (rafRef.current !== null) {
				cancelAnimationFrame(rafRef.current);
				rafRef.current = null;
			}
		};
	}, [tx, ty, scale, onViewChange]);

	// Re-report when the viewport itself resizes.
	useEffect(() => {
		if (!onViewChange) return;
		const el = containerRef.current;
		if (!el || typeof ResizeObserver === "undefined") return;
		const observer = new ResizeObserver(() => {
			const rect = el.getBoundingClientRect();
			onViewChange({ tx, ty, scale, width: rect.width, height: rect.height });
		});
		observer.observe(el);
		return () => observer.disconnect();
	}, [tx, ty, scale, onViewChange]);

	const zoomTo = useCallback(
		(nextScale: number, anchorX: number, anchorY: number) => {
			const rect = containerRef.current?.getBoundingClientRect();
			if (!rect) return;

			const s0 = scale;
			const s1 = clamp(nextScale, minScale, maxScale);
			if (s1 === s0) return;

			// Anchor in container coords.
			const mx = anchorX - rect.left;
			const my = anchorY - rect.top;

			// World coords under cursor before zoom.
			const wx = (mx - tx) / s0;
			const wy = (my - ty) / s0;

			// Keep the same world point under cursor after zoom.
			const nextTx = mx - wx * s1;
			const nextTy = my - wy * s1;

			setScale(s1);
			setTx(nextTx);
			setTy(nextTy);
		},
		[maxScale, minScale, scale, tx, ty],
	);

	// React attaches wheel listeners passively, where preventDefault is a
	// no-op (and logs an error) — attach a non-passive native listener so
	// zooming the map doesn't also scroll the page.
	const wheelHandlerRef = useRef<(e: WheelEvent) => void>(() => {});
	wheelHandlerRef.current = (e: WheelEvent) => {
		e.preventDefault();
		const factor = e.deltaY > 0 ? 0.9 : 1.1;
		zoomTo(scale * factor, e.clientX, e.clientY);
	};

	useEffect(() => {
		const el = containerRef.current;
		if (!el) return;
		const handler = (e: WheelEvent) => wheelHandlerRef.current(e);
		el.addEventListener("wheel", handler, { passive: false });
		return () => el.removeEventListener("wheel", handler);
	}, []);

	const onPointerDown = useCallback(
		(e: React.PointerEvent) => {
			if (e.button !== 0) return;
			const el = containerRef.current;
			if (!el) return;
			el.setPointerCapture(e.pointerId);
			dragRef.current = { x: e.clientX, y: e.clientY, tx, ty };
		},
		[tx, ty],
	);

	const onPointerMove = useCallback((e: React.PointerEvent) => {
		const drag = dragRef.current;
		if (!drag) return;
		const dx = e.clientX - drag.x;
		const dy = e.clientY - drag.y;
		setTx(drag.tx + dx);
		setTy(drag.ty + dy);
	}, []);

	const onPointerUp = useCallback((e: React.PointerEvent) => {
		const el = containerRef.current;
		if (el) el.releasePointerCapture(e.pointerId);
		dragRef.current = null;
	}, []);

	const reset = useCallback(() => {
		setScale(initialScale);
		if (initialCenter) {
			centerView();
		} else {
			setTx(24);
			setTy(24);
		}
	}, [initialScale, initialCenter, centerView]);

	const contentStyle = useMemo(
		() => ({
			width: contentSize.width,
			height: contentSize.height,
			transform: `translate(${tx}px, ${ty}px) scale(${scale})`,
			transformOrigin: "0 0",
		}),
		[contentSize.height, contentSize.width, scale, tx, ty],
	);

	return (
		<div
			ref={containerRef}
			className="relative w-full overflow-hidden rounded-xl border border-slate-200/70 bg-white/40 dark:border-slate-800/70 dark:bg-slate-950/20"
			style={{ height }}
			onPointerDown={onPointerDown}
			onPointerMove={onPointerMove}
			onPointerUp={onPointerUp}
		>
			<div className="absolute right-3 top-3 z-10 flex flex-col gap-2">
				<button
					type="button"
					className="btn-secondary h-10 w-10 p-0"
					onClick={() => setScale((s) => clamp(s * 1.15, minScale, maxScale))}
					aria-label="Zoom in"
					title="Zoom in"
				>
					+
				</button>
				<button
					type="button"
					className="btn-secondary h-10 w-10 p-0"
					onClick={() => setScale((s) => clamp(s / 1.15, minScale, maxScale))}
					aria-label="Zoom out"
					title="Zoom out"
				>
					−
				</button>
				<button
					type="button"
					className="btn-secondary h-10 w-10 p-0"
					onClick={reset}
					aria-label="Reset view"
					title="Reset"
				>
					⟳
				</button>
				<div className="rounded-lg bg-white/70 px-2 py-1 text-xs font-semibold text-slate-700 shadow-sm backdrop-blur dark:bg-slate-950/60 dark:text-slate-200">
					{Math.round(scale * 100)}%
				</div>
			</div>

			<div
				className="absolute left-0 top-0 will-change-transform"
				style={contentStyle}
			>
				{children}
			</div>
		</div>
	);
}
