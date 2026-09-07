import { animate, useMotionValue } from 'motion/react';
import { useEffect, useRef } from 'react';

import { cn } from '@/lib/cn';
import { ease } from '@/lib/motion';

interface NumberTickerProps {
  value: number;
  decimals?: number;
  locale?: string;
  className?: string;
}

/**
 * Counts to the new value instead of snapping. Writes to the DOM node directly
 * so a 10 Hz status stream never re-renders React (§3).
 */
export function NumberTicker({ value, decimals = 0, locale = 'en', className }: NumberTickerProps) {
  const node = useRef<HTMLSpanElement>(null);
  const motionValue = useMotionValue(0);

  useEffect(() => {
    const format = new Intl.NumberFormat(locale, {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    });

    const unsubscribe = motionValue.on('change', (latest) => {
      if (node.current) node.current.textContent = format.format(latest);
    });

    const controls = animate(motionValue, value, { duration: 0.45, ease });
    return () => {
      controls.stop();
      unsubscribe();
    };
  }, [value, decimals, locale, motionValue]);

  return (
    <span ref={node} className={cn('tabular-nums', className)}>
      0
    </span>
  );
}
