"use client";

interface LoadingSpinnerProps {
  size?: "small" | "medium" | "large";
  text?: string;
}

export function LoadingSpinner({ size = "medium", text }: LoadingSpinnerProps) {
  const sizeClasses = {
    small: "w-4 h-4",
    medium: "w-8 h-8",
    large: "w-12 h-12",
  };

  return (
    <div className="flex flex-col items-center justify-center gap-2">
      <div
        className={`${sizeClasses[size]} relative`}
        role="status"
        aria-label="Loading"
      >
        {/* Cozy bouncing dots */}
        <div className="absolute inset-0 flex items-center justify-center gap-1">
          <span className="h-1.5 w-1.5 rounded-full bg-[var(--primary)] animate-bounce [animation-delay:-0.2s]" />
          <span className="h-1.5 w-1.5 rounded-full bg-[var(--primary)] animate-bounce [animation-delay:-0.1s]" />
          <span className="h-1.5 w-1.5 rounded-full bg-[var(--primary)] animate-bounce" />
        </div>
      </div>
      {text && <p className="text-sm text-[var(--muted)]">{text}</p>}
    </div>
  );
}



