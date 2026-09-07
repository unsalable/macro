import { cva, type VariantProps } from 'class-variance-authority';
import type { HTMLAttributes } from 'react';

import { cn } from '@/lib/cn';

const badgeVariants = cva(
  'inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-[11px] font-medium leading-5',
  {
    variants: {
      tone: {
        neutral: 'bg-secondary text-muted-foreground',
        accent: 'bg-[var(--accent-soft)] text-accent',
        success: 'bg-[var(--success-soft)] text-success',
        warning: 'bg-[var(--warning-soft)] text-warning',
        danger: 'bg-[var(--danger-soft)] text-danger',
      },
    },
    defaultVariants: { tone: 'neutral' },
  },
);

export interface BadgeProps
  extends HTMLAttributes<HTMLSpanElement>,
    VariantProps<typeof badgeVariants> {}

export function Badge({ className, tone, ...props }: BadgeProps) {
  return <span className={cn(badgeVariants({ tone }), className)} {...props} />;
}

export function Skeleton({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn('animate-pulse rounded-[var(--radius-sm)] bg-[var(--muted)]', className)}
      {...props}
    />
  );
}

export function Separator({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div role="separator" className={cn('h-px w-full bg-[var(--border)]', className)} {...props} />;
}

/** Keyboard cap used to render hotkeys inline (§14). */
export function Kbd({ className, ...props }: HTMLAttributes<HTMLElement>) {
  return (
    <kbd
      className={cn(
        'inline-flex h-[22px] min-w-[22px] items-center justify-center rounded-[6px] border border-[var(--border-strong)]',
        'bg-[var(--card-muted)] px-1.5 font-sans text-[11px] font-medium text-muted-foreground',
        className,
      )}
      {...props}
    />
  );
}
