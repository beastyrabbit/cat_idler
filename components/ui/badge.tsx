import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const badgeVariants = cva(
  "inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-bold tracking-wide transition-colors",
  {
    variants: {
      variant: {
        default: "border-[var(--card-border)] bg-black/5 text-[var(--fg)] dark:bg-white/10",
        secondary: "border-transparent bg-[var(--secondary-bg)] text-[var(--fg)]",
        success: "border-transparent bg-emerald-500/15 text-emerald-800 dark:text-emerald-300",
        warning: "border-transparent bg-amber-500/15 text-amber-900 dark:text-amber-300",
        danger: "border-transparent bg-[var(--danger)]/15 text-[var(--danger-hover)] dark:text-[var(--danger)]",
      },
    },
    defaultVariants: {
      variant: "default",
    },
  }
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return <div className={cn(badgeVariants({ variant }), className)} {...props} />;
}

export { Badge, badgeVariants };


