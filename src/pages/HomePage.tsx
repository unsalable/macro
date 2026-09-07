import { Pause, Play, Plus, Square, Zap } from 'lucide-react';
import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';

import { EmptyState } from '@/components/common/EmptyState';
import { PageShell } from '@/components/layout/PageShell';
import { MacroCard } from '@/components/macro/MacroCard';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { NumberTicker } from '@/components/ui/number-ticker';
import { StatusPill } from '@/components/common/StatusPill';
import { formatDuration } from '@/lib/format';
import { useEngineStore } from '@/stores/engineStore';
import { useMacroStore } from '@/stores/macroStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUiStore } from '@/stores/uiStore';
import type { Macro } from '@/types';

export function HomePage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const macros = useMacroStore((store) => store.macros);
  const status = useEngineStore((store) => store.status);
  const start = useEngineStore((store) => store.start);
  const stop = useEngineStore((store) => store.stop);
  const pause = useEngineStore((store) => store.pause);
  const resume = useEngineStore((store) => store.resume);
  const selectedMacroId = useUiStore((store) => store.selectedMacroId);
  const selectMacro = useUiStore((store) => store.selectMacro);
  const language = useSettingsStore((store) => store.settings.language);

  const recent = useMemo(() => byRecency(macros).slice(0, 4), [macros]);
  const runnable = useMemo(
    () => macros.filter((item) => item.enabled && item.actions.length > 0),
    [macros],
  );

  const target =
    runnable.find((item) => item.id === selectedMacroId) ?? byRecency(runnable)[0] ?? null;
  const running = status.state === 'running' || status.state === 'paused';

  const handleStart = async () => {
    if (!target) return;
    try {
      await start(target.id);
    } catch (error) {
      toast.error(String(error));
    }
  };

  return (
    <PageShell
      title={t('home.greeting')}
      description={t('app.tagline')}
      actions={
        <Button variant="accent" onClick={() => void navigate('/macros')}>
          <Plus />
          {t('macros.new')}
        </Button>
      }
    >
      <div className="flex flex-col gap-6">
        <Card className="overflow-hidden">
          <CardContent className="flex flex-wrap items-center justify-between gap-6 p-6">
            <div className="flex min-w-0 flex-col gap-2">
              <StatusPill
                state={status.state}
                periodMs={status.actualCps > 0 ? 1000 / status.actualCps : undefined}
                className="self-start"
              />
              <p className="truncate text-lg font-medium">
                {target ? target.name : t('engine.noMacroSelected')}
              </p>
              <p className="text-[13px] text-muted-foreground">{t('engine.title')}</p>
            </div>

            <div className="flex items-center gap-8">
              <Metric
                label={t('engine.iterations')}
                value={<NumberTicker value={status.iterations} locale={language} />}
              />
              <Metric
                label={t('engine.actualCps')}
                value={
                  <NumberTicker value={status.actualCps} decimals={1} locale={language} />
                }
              />
              <Metric label={t('engine.elapsed')} value={formatDuration(status.elapsedMs)} />
            </div>

            <div className="flex items-center gap-2">
              {running ? (
                <>
                  <Button
                    variant="outline"
                    onClick={() => void (status.state === 'paused' ? resume() : pause())}
                  >
                    {status.state === 'paused' ? <Play /> : <Pause />}
                    {status.state === 'paused' ? t('engine.resume') : t('engine.pause')}
                  </Button>
                  <Button variant="danger" onClick={() => void stop()}>
                    <Square />
                    {t('engine.stop')}
                  </Button>
                </>
              ) : (
                <Button variant="primary" size="lg" disabled={!target} onClick={() => void handleStart()}>
                  <Play />
                  {t('engine.start')}
                </Button>
              )}
            </div>
          </CardContent>
        </Card>

        <section className="flex flex-col gap-3">
          <h2 className="text-[13px] font-semibold uppercase tracking-wide text-muted-foreground">
            {t('home.recent')}
          </h2>

          {recent.length === 0 ? (
            <EmptyState
              icon={Zap}
              title={t('macros.empty')}
              description={t('home.noRecent')}
              action={
                <Button variant="accent" onClick={() => void navigate('/macros')}>
                  <Plus />
                  {t('macros.new')}
                </Button>
              }
            />
          ) : (
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
              {recent.map((item, index) => (
                <MacroCard
                  key={item.id}
                  macro={item}
                  index={index}
                  active={status.macroId === item.id}
                  onOpen={() => void navigate(`/macros/${item.id}`)}
                  onSelect={() => selectMacro(item.id)}
                />
              ))}
            </div>
          )}
        </section>
      </div>
    </PageShell>
  );
}

function Metric({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-[11px] uppercase tracking-wide text-muted-foreground">{label}</span>
      <span className="text-xl font-semibold tabular-nums">{value}</span>
    </div>
  );
}

/** Most recently run first; never-run macros fall to the end. */
function byRecency(list: Macro[]): Macro[] {
  return [...list].sort((a, b) => {
    const left = a.stats.lastRunAt ?? '';
    const right = b.stats.lastRunAt ?? '';
    if (left === right) return a.name.localeCompare(b.name);
    return right.localeCompare(left);
  });
}
