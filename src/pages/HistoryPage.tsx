import { Clock, Trash2 } from 'lucide-react';
import { motion } from 'motion/react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { ConfirmDialog } from '@/components/common/ConfirmDialog';
import { EmptyState } from '@/components/common/EmptyState';
import { PageShell } from '@/components/layout/PageShell';
import { Badge } from '@/components/ui/misc';
import { Button } from '@/components/ui/button';
import { formatDateTime, formatDuration } from '@/lib/format';
import { listItemVariants, transition } from '@/lib/motion';
import { events, history as api } from '@/services/tauri';
import { useSettingsStore } from '@/stores/settingsStore';
import type { HistoryEntry } from '@/types';

const TONE: Record<HistoryEntry['event'], 'neutral' | 'success' | 'warning' | 'danger'> = {
  started: 'neutral',
  completed: 'success',
  stopped: 'warning',
  failed: 'danger',
};

export function HistoryPage() {
  const { t } = useTranslation();
  const language = useSettingsStore((store) => store.settings.language);
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [confirmClear, setConfirmClear] = useState(false);

  const load = useCallback(async () => {
    setEntries(await api.list());
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // A finished run writes a history line in Rust; refresh when that happens.
  useEffect(() => {
    let dispose: (() => void) | undefined;
    void events.onFinished(() => void load()).then((off) => {
      dispose = off;
    });
    return () => dispose?.();
  }, [load]);

  return (
    <PageShell
      title={t('history.title')}
      description={t('nav.history')}
      actions={
        entries.length > 0 ? (
          <Button variant="ghost" onClick={() => setConfirmClear(true)}>
            <Trash2 />
            {t('history.clear')}
          </Button>
        ) : null
      }
    >
      {entries.length === 0 ? (
        <EmptyState icon={Clock} title={t('history.empty')} />
      ) : (
        <ol className="flex flex-col gap-1.5">
          {entries.map((entry, index) => (
            <motion.li
              key={entry.id}
              variants={listItemVariants}
              initial="initial"
              animate="animate"
              transition={{ ...transition.fast, delay: Math.min(index, 12) * 0.02 }}
              className="flex items-center gap-4 rounded-[var(--radius-sm)] border border-[var(--border)] bg-card px-4 py-2.5"
            >
              <Badge tone={TONE[entry.event]}>{entry.event}</Badge>
              <span className="min-w-0 flex-1 truncate text-[13px] font-medium">
                {entry.macroName}
              </span>

              {entry.iterations > 0 ? (
                <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
                  {entry.iterations} × · {formatDuration(entry.durationMs)}
                </span>
              ) : null}

              <span className="shrink-0 text-xs text-muted-foreground">
                {formatDateTime(entry.at, language)}
              </span>
            </motion.li>
          ))}
        </ol>
      )}

      <ConfirmDialog
        open={confirmClear}
        onOpenChange={setConfirmClear}
        title={t('history.clear')}
        confirmLabel={t('history.clear')}
        onConfirm={() => {
          void api.clear().then(load);
        }}
      />
    </PageShell>
  );
}
