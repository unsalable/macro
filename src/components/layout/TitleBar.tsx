import { Copy, Minus, Square, X } from 'lucide-react';
import { AnimatePresence, motion } from 'motion/react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { cn } from '@/lib/cn';
import { formatHotkey } from '@/lib/hotkey';
import { transition } from '@/lib/motion';
import { appWindow, isNative } from '@/services/tauri';
import { useEngineStore } from '@/stores/engineStore';
import { useMacroStore } from '@/stores/macroStore';
import { useSettingsStore } from '@/stores/settingsStore';
import type { EngineState } from '@/types';

/**
 * Custom chrome; the frame is off so the app owns the whole surface (§34).
 *
 * The bar is read as an instrument panel rather than a header: a quiet
 * wordmark on the left, the window controls on the right, and the one thing
 * the user actually needs to see — what the engine is doing — centred between
 * them. While a run is in flight the centre grows into the live readout and
 * carries a Stop, so the engine can always be halted from any page without
 * hunting for the screen that started it.
 */
export function TitleBar() {
  const { t } = useTranslation();
  const status = useEngineStore((store) => store.status);
  const stop = useEngineStore((store) => store.stop);
  const macros = useMacroStore((store) => store.macros);
  const stopHotkey = useSettingsStore((store) => store.settings.hotkeys.stop);
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!isNative()) return undefined;
    let dispose: (() => void) | undefined;

    void appWindow.isMaximized().then(setMaximized);
    void appWindow
      .onResized(() => {
        void appWindow.isMaximized().then(setMaximized);
      })
      .then((off) => {
        dispose = off;
      });

    return () => dispose?.();
  }, []);

  const live = status.state !== 'idle';
  const runningName = macros.find((item) => item.id === status.macroId)?.name;

  return (
    <header
      data-tauri-drag-region
      className="drag-region relative flex h-11 shrink-0 items-center border-b border-[var(--border)] bg-[var(--background)] pl-3.5"
    >
      <Wordmark />

      {/* Absolutely placed so the capsule stays optically centred in the
          window no matter how wide the wordmark or the controls get. */}
      <div
        data-tauri-drag-region
        className="pointer-events-none absolute inset-x-0 flex justify-center px-40"
      >
        <RunCapsule
          state={status.state}
          name={runningName}
          cps={status.actualCps}
          iterations={status.iterations}
          live={live}
          stopLabel={stopHotkey ? formatHotkey(stopHotkey) : null}
          onStop={() => void stop()}
        />
      </div>

      <div className="no-drag ml-auto flex h-full items-center">
        <WindowButton onClick={() => void appWindow.minimize()} label="Minimize">
          <Minus className="size-3.5" />
        </WindowButton>
        <WindowButton onClick={() => void appWindow.toggleMaximize()} label="Maximize">
          {maximized ? <Copy className="size-3" /> : <Square className="size-3" />}
        </WindowButton>
        <WindowButton
          onClick={() => void appWindow.close()}
          label={t('common.close')}
          danger
        >
          <X className="size-3.5" />
        </WindowButton>
      </div>
    </header>
  );
}

/* ------------------------------------------------------------------ mark */

/** Drawn rather than borrowed from the icon set: the app's own glyph is a
    pointer whose tail is the repeat it fires. */
function Wordmark() {
  return (
    <div data-tauri-drag-region className="flex select-none items-center gap-2">
      <svg viewBox="0 0 16 16" className="size-[15px] shrink-0" aria-hidden="true">
        <path
          d="M4 2.4 L12.2 7.4 L8.6 8.3 L10.4 12.1 L8.7 12.9 L6.9 9.1 L4.4 11.6 Z"
          fill="var(--foreground)"
        />
        <path
          d="M13.2 2.6 a4.4 4.4 0 0 1 0 5.2"
          fill="none"
          stroke="var(--accent)"
          strokeWidth="1.5"
          strokeLinecap="round"
        />
      </svg>
      <span className="text-[10px] font-semibold uppercase tracking-[0.2em] text-muted-foreground">
        Flow<span className="text-foreground">macro</span>
      </span>
    </div>
  );
}

/* --------------------------------------------------------------- capsule */

const DOT: Record<EngineState, string> = {
  idle: 'bg-[var(--muted-foreground)]',
  running: 'bg-success',
  paused: 'bg-warning',
  stopping: 'bg-danger',
};

interface RunCapsuleProps {
  state: EngineState;
  name: string | undefined;
  cps: number;
  iterations: number;
  live: boolean;
  stopLabel: string | null;
  onStop: () => void;
}

function RunCapsule({ state, name, cps, iterations, live, stopLabel, onStop }: RunCapsuleProps) {
  const { t } = useTranslation();

  return (
    <motion.div
      layout
      transition={transition.spring}
      className={cn(
        'no-drag pointer-events-auto flex h-7 max-w-full items-center gap-2 rounded-full border px-2.5',
        'text-[11px] leading-none transition-colors duration-[var(--fm-normal)]',
        live
          ? 'border-[var(--border-strong)] bg-[var(--card)] shadow-[var(--shadow-soft)]'
          : 'border-transparent bg-[var(--secondary)]',
      )}
    >
      <span className="relative flex size-1.5 shrink-0">
        {state === 'running' ? (
          <motion.span
            className={cn('absolute inline-flex size-full rounded-full', DOT[state])}
            animate={{ opacity: [0.9, 0.2, 0.9], scale: [1, 2.1, 1] }}
            transition={{ duration: 1.4, repeat: Infinity, ease: 'easeInOut' }}
          />
        ) : null}
        <span className={cn('relative inline-flex size-full rounded-full', DOT[state])} />
      </span>

      <span
        className={cn(
          'shrink-0 font-medium',
          state === 'running' && 'text-success',
          state === 'paused' && 'text-warning',
          state === 'stopping' && 'text-danger',
          state === 'idle' && 'text-muted-foreground',
        )}
      >
        {t(`engine.${state}`)}
      </span>

      <AnimatePresence initial={false} mode="popLayout">
        {live ? (
          <motion.div
            key="live"
            initial={{ opacity: 0, width: 0 }}
            animate={{ opacity: 1, width: 'auto' }}
            exit={{ opacity: 0, width: 0 }}
            transition={transition.fast}
            className="flex items-center gap-2 overflow-hidden whitespace-nowrap"
          >
            {name ? (
              <>
                <Rule />
                <span className="max-w-[180px] truncate text-muted-foreground">{name}</span>
              </>
            ) : null}
            <Rule />
            <Readout value={cps.toFixed(1)} unit="/s" />
            <Rule />
            <Readout value={iterations.toLocaleString()} unit="×" />
            <button
              type="button"
              onClick={onStop}
              className={cn(
                'ml-0.5 -mr-1.5 flex h-5 items-center gap-1.5 rounded-full bg-danger px-2.5',
                'text-[11px] font-medium text-white transition-[filter,transform] hover:brightness-110 active:scale-[0.97]',
              )}
            >
              {t('engine.stop')}
              {stopLabel ? (
                <span className="font-mono text-[10px] opacity-70">{stopLabel}</span>
              ) : null}
            </button>
          </motion.div>
        ) : null}
      </AnimatePresence>
    </motion.div>
  );
}

function Rule() {
  return <span className="h-3 w-px shrink-0 bg-[var(--border)]" />;
}

function Readout({ value, unit }: { value: string; unit: string }) {
  return (
    <span className="shrink-0 font-mono tabular-nums text-foreground">
      {value}
      <span className="ml-0.5 text-muted-foreground">{unit}</span>
    </span>
  );
}

/* -------------------------------------------------------- window buttons */

interface WindowButtonProps {
  onClick: () => void;
  label: string;
  danger?: boolean;
  children: React.ReactNode;
}

function WindowButton({ onClick, label, danger = false, children }: WindowButtonProps) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className={cn(
        'flex h-full w-11 items-center justify-center text-muted-foreground transition-colors',
        danger ? 'hover:bg-danger hover:text-white' : 'hover:bg-secondary hover:text-foreground',
      )}
    >
      {children}
    </button>
  );
}
