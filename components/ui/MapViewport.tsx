"use client";

import { useCallback, useMemo, useRef, useState } from "react";

type Size = { width: number; height: number };

export interface MapViewportProps {
  /** Pixel size of the content at scale=1 (ex: gridSize * tileSize). */
  contentSize: Size;
  /** Height of the viewport in px (container is full-width). */
  height?: number;
  /** Initial zoom level (1 = 100%). */
  initialScale?: number;
  minScale?: number;
  maxScale?: number;
  children: React.ReactNode;
}

export function MapViewport({
  contentSize,
  height = 560,
  initialScale = 0.8,
  minScale = 0.4,
  maxScale = 2,
  children,
}: MapViewportProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<{ x: number; y: number; tx: number; ty: number } | null>(null);

  const [scale, setScale] = useState(initialScale);
  const [tx, setTx] = useState(24);
  const [ty, setTy] = useState(24);

  const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));

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
    [maxScale, minScale, scale, tx, ty]
  );

  const onWheel = useCallback(
    (e: React.WheelEvent) => {
      // Ctrl+wheel usually means browser zoom; but in-app zoom still feels right.
      e.preventDefault();
      const factor = e.deltaY > 0 ? 0.9 : 1.1;
      zoomTo(scale * factor, e.clientX, e.clientY);
    },
    [scale, zoomTo]
  );

  const onPointerDown = useCallback((e: React.PointerEvent) => {
    if (e.button !== 0) return;
    const el = containerRef.current;
    if (!el) return;
    el.setPointerCapture(e.pointerId);
    dragRef.current = { x: e.clientX, y: e.clientY, tx, ty };
  }, [tx, ty]);

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
    setTx(24);
    setTy(24);
  }, [initialScale]);

  const contentStyle = useMemo(
    () => ({
      width: contentSize.width,
      height: contentSize.height,
      transform: `translate(${tx}px, ${ty}px) scale(${scale})`,
      transformOrigin: "0 0",
    }),
    [contentSize.height, contentSize.width, scale, tx, ty]
  );

  return (
    <div
      ref={containerRef}
      className="relative w-full overflow-hidden rounded-xl border border-slate-200/70 bg-white/40 dark:border-slate-800/70 dark:bg-slate-950/20"
      style={{ height }}
      onWheel={onWheel}
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

      <div className="absolute left-0 top-0 will-change-transform" style={contentStyle}>
        {children}
      </div>
    </div>
  );
}


