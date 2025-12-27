"use client";

import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";

interface ErrorDisplayProps {
  error: Error | string;
  onRetry?: () => void;
}

export function ErrorDisplay({ error, onRetry }: ErrorDisplayProps) {
  const errorMessage = typeof error === "string" ? error : error.message;

  return (
    <Card className="border-[rgba(234,123,60,0.35)] bg-[rgba(255,245,240,0.70)] dark:bg-[rgba(234,123,60,0.12)]">
      <CardContent className="p-5">
        <div className="flex items-start gap-3">
          <div className="mt-0.5 text-2xl" aria-hidden="true">
            ⚠️
          </div>
          <div className="flex-1">
            <div className="font-black text-[var(--fg)]">Error</div>
            <p className="mt-1 text-sm text-[var(--muted)]">{errorMessage}</p>
            {onRetry && (
              <div className="mt-4">
                <Button onClick={onRetry} variant="destructive">
                  Retry
                </Button>
              </div>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}



