import { motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import type { ReactNode } from 'react';

import { cn } from '@/lib/cn';
import { transition } from '@/lib/motion';

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description?: string | undefined;
  action?: ReactNode | undefined;
  className?: string | undefined;
}

/** Every list gets a real empty state, never a blank panel (§53). */
export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
}: EmptyStateProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={transition.normal}
      className={cn(
        'flex flex-col items-center justify-center gap-3 rounded-[var(--radius-md)] border border-dashed',
        'border-[var(--border-strong)] bg-[var(--card-muted)] px-8 py-14 text-center',
        className,
      )}
    >
      <span className="flex size-11 items-center justify-center rounded-full bg-[var(--accent-soft)] text-accent">
        <Icon className="size-5" />
      </span>
      <div className="flex flex-col gap-1">
        <p className="text-sm font-medium text-foreground">{title}</p>
        {description ? (
          <p className="max-w-sm text-[13px] leading-relaxed text-muted-foreground">
            {description}
          </p>
        ) : null}
      </div>
      {action ? <div className="mt-1">{action}</div> : null}
    </motion.div>
  );
}
