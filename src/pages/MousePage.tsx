import { Keyboard, Play, Save, Square, X } from 'lucide-react';
import { AnimatePresence, motion } from 'motion/react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { StatusPill } from '@/components/common/StatusPill';
import { PageShell } from '@/components/layout/PageShell';
import { CpsControl } from '@/components/mouse/CpsControl';
import { MouseVisualizer } from '@/components/mouse/MouseVisualizer';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { HotkeyInput } from '@/components/ui/hotkey-input';
import { Field, NumberInput } from '@/components/ui/input';
import { NumberTicker } from '@/components/ui/number-ticker';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { createAction, newActionId } from '@/lib/actions';
import { formatDuration } from '@/lib/format';
import { formatHotkey } from '@/lib/hotkey';
import { listItemVariants, listVariants, transition } from '@/lib/motion';
import { useEngineStore } from '@/stores/engineStore';
import { useMacroStore } from '@/stores/macroStore';
import { useSettingsStore } from '@/stores/settingsStore';
import type { ClickMode, Hotkey, Macro, MouseButton } from '@/types';

/** Stable id so tuning the quick clicker never spawns a second macro. */
const QUICK_ID = 'quick-click';

/**
 * How long to sit on a change before writing it. The hotkey only reaches the
 * global matcher through a save, so an unsaved page meant a key that did
 * nothing at all — the one thing a user must never have to guess about (§18).
 */
const AUTOSAVE_DELAY_MS = 350;
const MODES: ClickMode[] = ['single', 'double', 'hold'];

interface ClickerSettings {
  button: MouseButton;
  mode: ClickMode;
  intervalMs: number;
  holdMs: number;
  repeat: number;
  hotkey: Hotkey | null;
}

/** What "the form changed" means, in one comparable string. */
const signatureOf = (settings: ClickerSettings): string => JSON.stringify(settings);

/** The stored macro read back into form values. */
function settingsOf(existing: Macro | undefined): ClickerSettings {
  const action = existing?.actions[0];
  const seed = action?.type === 'mouse_click' ? action : null;
  return {
    button: seed?.button ?? 'left',
    mode: seed?.mode ?? 'single',
    intervalMs: seed?.intervalMs ?? 100,
    holdMs: seed?.durationMs ?? 0,
    repeat: existing?.loop.mode === 'count' ? existing.loop.count : 0,
    hotkey: existing?.hotkey ?? null,
  };
}

export function MousePage() {
  const { t } = useTranslation();
  const macros = useMacroStore((store) => store.macros);
  const save = useMacroStore((store) => store.save);
  const status = useEngineStore((store) => store.status);
  const start = useEngineStore((store) => store.start);
  const stop = useEngineStore((store) => store.stop);
  const arm = useEngineStore((store) => store.arm);
  const disarm = useEngineStore((store) => store.disarm);
  const armedMacroId = useEngineStore((store) => store.armedMacroId);
  const language = useSettingsStore((store) => store.settings.language);

  const existing = macros.find((item) => item.id === QUICK_ID);
  const existingAction = existing?.actions[0];
  const stored = settingsOf(existing);

  const [button, setButton] = useState<MouseButton>(stored.button);
  const [mode, setMode] = useState<ClickMode>(stored.mode);
  const [intervalMs, setIntervalMs] = useState(stored.intervalMs);
  const [holdMs, setHoldMs] = useState(stored.holdMs);
  const [repeat, setRepeat] = useState(stored.repeat);
  const [hotkey, setHotkey] = useState<Hotkey | null>(stored.hotkey);

  const current: ClickerSettings = { button, mode, intervalMs, holdMs, repeat, hotkey };
  const signature = signatureOf(current);
  /** The last shape that is known to be on disk. */
  const saved = useRef(signature);
  const seeded = useRef(false);

  // The macro list arrives a moment after this page mounts, so the form starts
  // on defaults and has to take the stored values once they land. Without this
  // the first edit would autosave those defaults over the real ones — hotkey
  // included (§18).
  useEffect(() => {
    if (seeded.current || !existing) return;
    seeded.current = true;
    const values = settingsOf(existing);
    setButton(values.button);
    setMode(values.mode);
    setIntervalMs(values.intervalMs);
    setHoldMs(values.holdMs);
    setRepeat(values.repeat);
    setHotkey(values.hotkey);
    saved.current = signatureOf(values);
  }, [existing]);

  const bound = useMemo(
    () =>
      macros
        .flatMap((item) => item.actions)
        .filter((action) => action.type === 'mouse_click')
        .map((action) => action.button),
    [macros],
  );

  const running = status.macroId === QUICK_ID && status.state !== 'idle';
  const armed = armedMacroId === QUICK_ID;
  const hotkeyLabel = hotkey ? formatHotkey(hotkey) : t('engine.noHotkey');

  // The illustration beats at whatever is really leaving the machine, and
  // falls back to the requested rate before the first measurement lands.
  const beatMs = running && status.actualCps > 0 ? 1000 / status.actualCps : intervalMs;

  const build = (): Macro => {
    const action = createAction('mouse_click');
    return {
      id: QUICK_ID,
      name: `${t('mouse.title')} · ${t(`mouse.${button}`)}`,
      enabled: true,
      hotkey,
      activation: 'toggle',
      actions: [
        {
          ...action,
          id: existingAction?.id ?? newActionId(),
          type: 'mouse_click',
          button,
          mode,
          count: 1,
          intervalMs,
          durationMs: mode === 'hold' ? holdMs : 0,
        },
      ],
      // One click per pass, so the click rate is the *loop* interval. Leaving
      // it at 0 would repeat a single click as fast as the machine allows and
      // ignore the CPS the user set entirely.
      loop:
        repeat > 0
          ? { mode: 'count', count: repeat, durationMs: 0, intervalMs }
          : { mode: 'infinite', count: 0, durationMs: 0, intervalMs },
      randomization: existing?.randomization ?? { enabled: false, jitterMs: 0 },
      stats: existing?.stats ?? { runCount: 0, lastRunAt: null },
      createdAt: existing?.createdAt ?? '',
      updatedAt: existing?.updatedAt ?? '',
    };
  };

  const persist = async (): Promise<Macro> => {
    saved.current = signature;
    return save(build());
  };

  // Every edit is written back on its own. Before this, a hotkey chosen in the
  // field below only reached Rust if the user also pressed Save or Start, so
  // the key read as set while doing nothing (§18).
  const buildRef = useRef(build);
  buildRef.current = build;

  useEffect(() => {
    if (signature === saved.current) return undefined;
    const timer = window.setTimeout(() => {
      saved.current = signature;
      void save(buildRef.current()).catch(() => {
        // Reported by the next explicit action instead: a page that throws a
        // toast on every keystroke is worse than a quiet retry.
      });
    }, AUTOSAVE_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [signature, save]);

  /**
   * Start hands the clicker to its hotkey instead of clicking straight away:
   * one press makes the key live, and the key itself starts and stops the run
   * (§19). With no key set there is nothing to arm, so it runs as before.
   */
  const handleStart = async () => {
    try {
      await persist();
      if (hotkey) {
        await arm(QUICK_ID);
      } else {
        await start(QUICK_ID);
      }
    } catch (error) {
      toast.error(t('error.generic'), { description: String(error) });
    }
  };

  const handleStartNow = async () => {
    try {
      await persist();
      await start(QUICK_ID);
    } catch (error) {
      toast.error(t('error.generic'), { description: String(error) });
    }
  };

  return (
    <PageShell
      title={t('mouse.title')}
      description={t('mouse.subtitle')}
      actions={
        running ? (
          <Button variant="danger" onClick={() => void stop()}>
            <Square />
            {t('engine.stop')}
          </Button>
        ) : armed ? (
          <>
            <Button variant="ghost" onClick={() => void disarm()}>
              <X />
              {t('engine.disarm')}
            </Button>
            <Button variant="accent" onClick={() => void handleStartNow()}>
              <Play />
              {t('engine.startNow')}
            </Button>
          </>
        ) : (
          <>
            <Button
              variant="ghost"
              onClick={() => void persist().then(() => toast.success(t('toast.saved')))}
            >
              <Save />
              {t('common.save')}
            </Button>
            <Button variant="accent" onClick={() => void handleStart()}>
              {hotkey ? <Keyboard /> : <Play />}
              {hotkey ? t('engine.arm') : t('engine.start')}
            </Button>
          </>
        )
      }
    >
      <AnimatePresence initial={false}>
        {armed && !running ? (
          <motion.div
            initial={{ opacity: 0, y: -8, height: 0 }}
            animate={{ opacity: 1, y: 0, height: 'auto' }}
            exit={{ opacity: 0, y: -8, height: 0 }}
            transition={transition.normal}
            className="mb-6 overflow-hidden"
          >
            <Card className="border-[var(--accent)]/40 bg-[var(--accent-soft)]">
              <CardContent className="flex items-center gap-3 p-4">
                <Keyboard className="size-4 shrink-0 text-accent" />
                <div className="flex min-w-0 flex-col gap-0.5">
                  <span className="text-[13px] font-medium text-foreground">
                    {t('engine.armedTitle', { key: hotkeyLabel })}
                  </span>
                  <span className="text-[12px] text-muted-foreground">
                    {t('engine.armedHint', { key: hotkeyLabel })}
                  </span>
                </div>
              </CardContent>
            </Card>
          </motion.div>
        ) : null}
      </AnimatePresence>

      <motion.div
        variants={listVariants}
        initial="initial"
        animate="animate"
        className="grid grid-cols-1 gap-6 lg:grid-cols-[minmax(0,380px)_minmax(0,1fr)]"
      >
        <motion.div variants={listItemVariants} className="flex flex-col gap-4">
          <Card className="overflow-hidden">
            <CardContent className="p-5">
              <MouseVisualizer
                selected={button}
                onSelect={setButton}
                bound={bound}
                active={running && status.state === 'running'}
                periodMs={beatMs}
              />
            </CardContent>
          </Card>

          {/* The run strip lives under the mouse so the numbers and the thing
              they describe move together rather than sitting pages apart. */}
          <AnimatePresence initial={false}>
            {running ? (
              <motion.div
                initial={{ opacity: 0, y: -8, height: 0 }}
                animate={{ opacity: 1, y: 0, height: 'auto' }}
                exit={{ opacity: 0, y: -8, height: 0 }}
                transition={transition.normal}
              >
                <Card className="border-[var(--accent)]/40 bg-[var(--accent-soft)]">
                  <CardContent className="flex items-center justify-between gap-4 p-4">
                    <StatusPill state={status.state} periodMs={beatMs} />
                    <Live
                      label={t('engine.actualCps')}
                      value={
                        <NumberTicker value={status.actualCps} decimals={1} locale={language} />
                      }
                    />
                    <Live
                      label={t('engine.iterations')}
                      value={<NumberTicker value={status.iterations} locale={language} />}
                    />
                    <Live label={t('engine.elapsed')} value={formatDuration(status.elapsedMs)} />
                  </CardContent>
                </Card>
              </motion.div>
            ) : null}
          </AnimatePresence>
        </motion.div>

        <div className="flex flex-col gap-5">
          <motion.div variants={listItemVariants}>
            <Card>
              <CardContent className="flex flex-col gap-5 p-5">
                <Field label={t('mouse.clickMode')}>
                  <Tabs value={mode} onValueChange={(value) => setMode(value as ClickMode)}>
                    <TabsList className="w-full">
                      {MODES.map((item) => (
                        <TabsTrigger key={item} value={item} className="flex-1">
                          {t(`mouse.${item}`)}
                        </TabsTrigger>
                      ))}
                    </TabsList>
                  </Tabs>
                </Field>

                <CpsControl intervalMs={intervalMs} onIntervalChange={setIntervalMs} />

                <AnimatePresence initial={false}>
                  {mode === 'hold' ? (
                    <motion.div
                      initial={{ opacity: 0, height: 0 }}
                      animate={{ opacity: 1, height: 'auto' }}
                      exit={{ opacity: 0, height: 0 }}
                      transition={transition.normal}
                      className="overflow-hidden"
                    >
                      <Field label={t('mouse.hold')}>
                        <NumberInput value={holdMs} min={0} suffix="ms" onValueChange={setHoldMs} />
                      </Field>
                    </motion.div>
                  ) : null}
                </AnimatePresence>
              </CardContent>
            </Card>
          </motion.div>

          <motion.div variants={listItemVariants}>
            <Card>
              <CardContent className="flex flex-col gap-5 p-5">
                <Field
                  label={t('editor.loopCount')}
                  hint={repeat === 0 ? t('mouse.repeatHint') : undefined}
                >
                  <NumberInput value={repeat} min={0} max={1_000_000} onValueChange={setRepeat} />
                </Field>

                <Field label={t('settings.hotkeys')} hint={t('mouse.hotkeyHint')}>
                  <HotkeyInput value={hotkey} onChange={setHotkey} />
                </Field>
              </CardContent>
            </Card>
          </motion.div>

          <AnimatePresence initial={false}>
            {running ? (
              <motion.p
                initial={{ opacity: 0, y: -4 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -4 }}
                transition={transition.fast}
                className="text-[12px] text-muted-foreground"
              >
                {hotkey ? t('engine.runningWithKey', { key: hotkeyLabel }) : t('mouse.liveHint')}
              </motion.p>
            ) : null}
          </AnimatePresence>
        </div>
      </motion.div>
    </PageShell>
  );
}

function Live({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-0.5">
      <span className="text-[10px] uppercase tracking-wide text-muted-foreground">{label}</span>
      <span className="text-[15px] font-semibold tabular-nums text-foreground">{value}</span>
    </div>
  );
}
