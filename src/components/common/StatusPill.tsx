import { motion } from 'motion/react';
import { useTranslation } from 'react-i18next';

import { cn } from '@/lib/cn';
import { transition } from '@/lib/motion';
import type { EngineState } from '@/types';

const TONE: Record<EngineState, { dot: string; text: string; bg: string }> = {
  idle: { dot: 'bg-[var(--muted-foreground)]', text: 'text-muted-foreground', bg: 'bg-secondary' },
  running: { dot: 'bg-success', text: 'text-success', bg: 'bg-[var(--success-soft)]' },
  paused: { dot: 'bg-warning', text: 'text-warning', bg: 'bg-[var(--warning-soft)]' },
  stopping: { dot: 'bg-danger', text: 'text-danger', bg: 'bg-[var(--danger-soft)]' },
};

interface StatusPillProps {
  state: EngineState;
  /** Measured period of one injection, so the pulse keeps time with the run. */
  periodMs?: number | undefined;
  className?: string;
}

/**
 * Clamped so a 100 CPS run does not strobe the dot, and quantised to tenths
 * so the 10 Hz measurement stream does not restart the loop on every push.
 */
function pulseSeconds(periodMs: number | undefined): number {
  if (!periodMs || !Number.isFinite(periodMs)) return 1.8;
  const seconds = Math.min(1.8, Math.max(0.28, periodMs / 1000));
  return Math.round(seconds * 10) / 10;
}

export function StatusPill({ state, periodMs, className }: StatusPillProps) {
  const { t } = useTranslation();
  const tone = TONE[state];
  const pulse = pulseSeconds(periodMs);

  return (
    <span
      className={cn(
        'inline-flex items-center gap-2 rounded-full px-3 py-1 text-[12px] font-medium',
        tone.bg,
        tone.text,
        className,
      )}
    >
      <span className="relative flex size-1.5">
        {state === 'running' ? (
          // The pulse runs at the engine's real rate, so the pill and the
          // thing it describes are visibly the same event (§56).
          <motion.span
            className={cn('absolute inline-flex size-full rounded-full', tone.dot)}
            animate={{ opacity: [0.9, 0.25, 0.9], scale: [1, 1.8, 1] }}
            transition={{ duration: pulse, repeat: Infinity, ease: 'easeInOut' }}
          />
        ) : null}
        <motion.span
          layout
          transition={transition.fast}
          className={cn('relative inline-flex size-full rounded-full', tone.dot)}
        />
      </span>
      {t(`engine.${state}`)}
    </span>
  );
}
