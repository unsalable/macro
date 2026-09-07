import { useTranslation } from 'react-i18next';

import { Field, NumberInput } from '@/components/ui/input';
import { Slider } from '@/components/ui/slider';
import { SwitchRow } from '@/components/ui/switch';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import type { LoopConfig, LoopMode, Randomization } from '@/types';

interface LoopEditorProps {
  loop: LoopConfig;
  randomization: Randomization;
  onLoopChange: (loop: LoopConfig) => void;
  onRandomizationChange: (randomization: Randomization) => void;
}

const MODES: LoopMode[] = ['none', 'infinite', 'count', 'duration'];

export function LoopEditor({
  loop,
  randomization,
  onLoopChange,
  onRandomizationChange,
}: LoopEditorProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-2">
        <h3 className="text-[13px] font-semibold uppercase tracking-wide text-muted-foreground">
          {t('editor.loop')}
        </h3>

        <Tabs
          value={loop.mode}
          onValueChange={(value) => onLoopChange({ ...loop, mode: value as LoopMode })}
        >
          <TabsList className="w-full">
            {MODES.map((mode) => (
              <TabsTrigger key={mode} value={mode} className="flex-1">
                {t(`editor.loop${mode.charAt(0).toUpperCase()}${mode.slice(1)}`)}
              </TabsTrigger>
            ))}
          </TabsList>
        </Tabs>
      </div>

      {loop.mode === 'count' ? (
        <Field label={t('editor.loopCount')}>
          <NumberInput
            value={loop.count}
            min={1}
            max={1_000_000}
            onValueChange={(count) => onLoopChange({ ...loop, count })}
          />
        </Field>
      ) : null}

      {loop.mode === 'duration' ? (
        <Field label={t('editor.loopDuration')}>
          <NumberInput
            value={loop.durationMs}
            min={100}
            suffix="ms"
            onValueChange={(durationMs) => onLoopChange({ ...loop, durationMs })}
          />
        </Field>
      ) : null}

      {/* Without a gap, a short macro repeats as fast as the machine allows —
          which is almost never what someone means by "repeat forever". */}
      {loop.mode !== 'none' ? (
        <Field label={t('editor.loopInterval')} hint={t('editor.loopIntervalHint')}>
          <NumberInput
            value={loop.intervalMs}
            min={0}
            max={3_600_000}
            suffix="ms"
            onValueChange={(intervalMs) => onLoopChange({ ...loop, intervalMs })}
          />
        </Field>
      ) : null}

      <div className="border-t border-[var(--border)] pt-1">
        <SwitchRow
          label={t('editor.randomization')}
          description={t('editor.randomizationHint')}
          checked={randomization.enabled}
          onCheckedChange={(enabled) => onRandomizationChange({ ...randomization, enabled })}
        />

        {randomization.enabled ? (
          <div className="flex items-center gap-3 pb-2">
            <span className="w-16 shrink-0 text-xs text-muted-foreground">
              {t('editor.jitter')}
            </span>
            <Slider
              value={[randomization.jitterMs]}
              min={0}
              max={500}
              step={5}
              onValueChange={([jitterMs]) =>
                onRandomizationChange({ ...randomization, jitterMs: jitterMs ?? 0 })
              }
            />
            <span className="w-14 shrink-0 text-right text-xs tabular-nums text-muted-foreground">
              ±{randomization.jitterMs} ms
            </span>
          </div>
        ) : null}
      </div>
    </div>
  );
}
