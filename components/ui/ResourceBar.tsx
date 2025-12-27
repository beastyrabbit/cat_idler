"use client";

import type { ResourceBarProps } from "@/types/game";
import { Progress } from "@/components/ui/progress";
import { cn } from "@/lib/utils";

export function ResourceBar({
  value,
  max,
  color = "green",
  showLabel = false,
  label,
}: ResourceBarProps) {
  const percentage = Math.min(100, (value / max) * 100);
  
  const colorClasses = {
    green: "bg-green-500",
    blue: "bg-blue-500",
    red: "bg-red-500",
    yellow: "bg-yellow-500",
    gray: "bg-gray-500",
    purple: "bg-purple-500",
  };

  const icons: Record<NonNullable<ResourceBarProps["color"]>, string> = {
    green: "🍃",
    blue: "💧",
    red: "❤️",
    yellow: "🌼",
    gray: "🪵",
    purple: "✨",
  };

  const isLow = percentage > 0 && percentage <= 20;

  const resourceIconPath = label ? `/images/resources/${label.toLowerCase()}.png` : null;
  // Check if it's a known resource type, otherwise fallback to emoji logic or pass as prop later
  const isKnownResource = label && ["food", "water", "herbs", "materials", "blessings"].includes(label.toLowerCase());

  return (
    <div className="w-full">
      {showLabel && label && (
        <div className="mb-1 flex items-center justify-between gap-2 text-sm font-semibold text-[var(--fg)]">
          <span className="flex items-center gap-2">
            {isKnownResource ? (
               <img src={resourceIconPath!} alt="" className="w-5 h-5 object-contain" />
            ) : (
              <span aria-hidden="true" className="text-base">
                {icons[color]}
              </span>
            )}
            <span>{label}</span>
          </span>
          <span className="tabular-nums text-[var(--muted)]">
            {value}/{max}
          </span>
        </div>
      )}
      <Progress
        value={percentage}
        className={cn("h-4", isLow && "animate-cozy-pulse")}
        aria-label={label ?? "Resource"}
        indicatorClassName={cn(
          colorClasses[color],
          "rounded-full shadow-[inset_0_0_0_1px_rgba(255,255,255,0.30)]"
        )}
        indicatorTestId="resource-indicator"
        data-testid="resource-progress"
      >
        {/* Radix adds role="progressbar" to the root */}
      </Progress>
    </div>
  );
}



