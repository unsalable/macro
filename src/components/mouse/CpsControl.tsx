import { useTranslation } from 'react-i18next';

import { NumberInput } from '@/components/ui/input';
import { Slider } from '@/components/ui/slider';
import {
  MAX_INTERVAL_MS,
  MIN_INTERVAL_MS,
  clampInterval,
  cpsToInterval,
  formatCps,
  intervalToCps,
} from '@/lib/cps';

interface CpsControlProps {
  intervalMs: number;
  onIntervalChange: (intervalMs: number) => void;
  maxCps?: number;
}

/**
 * CPS and interval are two views of one value (§11). `intervalMs` stays the
 * single source of truth; the slider works in CPS because that is how people
 * think about click speed.
 */
export function CpsControl({ intervalMs, onIntervalChange, maxCps = 100 }: CpsControlProps) {
  const { t } = useTranslation();
  const cps = intervalToCps(intervalMs);

  return (
    <div className="flex flex-col gap-3 rounded-[var(--radius-sm)] bg-[var(--card-muted)] p-3">
      <div className="flex items-baseline justify-between">
        <span className="text-[13px] font-medium">{t('mouse.cps')}</span>
        <span className="text-lg font-semibold tabular-nums text-accent">{formatCps(cps)}</span>
      </div>

      <Slider
        value={[Math.min(cps, maxCps)]}
        min={1}
        max={maxCps}
        step={1}
        onValueChange={([value]) => onIntervalChange(cpsToInterval(value ?? 1))}
      />

      <div className="flex items-center gap-3">
        <span className="shrink-0 text-xs text-muted-foreground">{t('mouse.interval')}</span>
        <NumberInput
          value={Math.round(intervalMs)}
          min={MIN_INTERVAL_MS}
          max={MAX_INTERVAL_MS}
          suffix="ms"
          onValueChange={(value) => onIntervalChange(clampInterval(value))}
          className="h-8"
        />
      </div>
    </div>
  );
}
