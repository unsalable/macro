import { Circle, Radio, Save, Square } from 'lucide-react';
import { motion } from 'motion/react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';

import { EmptyState } from '@/components/common/EmptyState';
import { PageShell } from '@/components/layout/PageShell';
import { Timeline } from '@/components/recorder/Timeline';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Field, Input } from '@/components/ui/input';
import { SwitchRow } from '@/components/ui/switch';
import { formatDuration } from '@/lib/format';
import { events, recorder as api } from '@/services/tauri';
import { useMacroStore } from '@/stores/macroStore';
import type { RecordedEvent } from '@/types';

export function RecorderPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const save = useMacroStore((store) => store.save);

  const [recording, setRecording] = useState(false);
  const [captured, setCaptured] = useState<RecordedEvent[]>([]);
  const [live, setLive] = useState<RecordedEvent[]>([]);
  const [elapsed, setElapsed] = useState(0);
  const [name, setName] = useState('');
  const [includeMoves, setIncludeMoves] = useState(true);
  const startedAt = useRef(0);

  // Live events arrive from Rust already filtered: mouse moves are stored but
  // not streamed, so this list stays readable during a long recording.
  useEffect(() => {
    let dispose: (() => void) | undefined;
    void events
      .onRecorded((event) => setLive((current) => [...current.slice(-199), event]))
      .then((off) => {
        dispose = off;
      });
    return () => dispose?.();
  }, []);

  useEffect(() => {
    if (!recording) return undefined;
    const timer = window.setInterval(() => setElapsed(Date.now() - startedAt.current), 100);
    return () => window.clearInterval(timer);
  }, [recording]);

  const start = useCallback(async () => {
    try {
      await api.start();
      setLive([]);
      setCaptured([]);
      startedAt.current = Date.now();
      setElapsed(0);
      setRecording(true);
    } catch (error) {
      toast.error(t('error.generic'), { description: String(error) });
    }
  }, [t]);

  const stop = useCallback(async () => {
    const result = await api.stop();
    setRecording(false);
    setCaptured(result);
  }, []);

  const saveAsMacro = async () => {
    const draft = await api.toMacro(name || t('recorder.title'), captured, includeMoves);
    const saved = await save(draft);
    toast.success(t('toast.saved'));
    void navigate(`/macros/${saved.id}`);
  };

  const shown = recording ? live : captured;

  return (
    <PageShell
      title={t('recorder.title')}
      description={t('recorder.emptyHint')}
      actions={
        recording ? (
          <Button variant="danger" onClick={() => void stop()}>
            <Square />
            {t('recorder.stop')}
          </Button>
        ) : (
          <Button variant="accent" onClick={() => void start()}>
            <Circle />
            {t('recorder.start')}
          </Button>
        )
      }
    >
      <div className="grid grid-cols-1 gap-6 xl:grid-cols-[minmax(0,1fr)_300px]">
        <div className="flex flex-col gap-4">
          {recording ? (
            <Card className="border-danger/40">
              <CardContent className="flex items-center gap-3 p-4">
                <motion.span
                  className="size-2.5 rounded-full bg-danger"
                  animate={{ opacity: [1, 0.25, 1] }}
                  transition={{ duration: 1.4, repeat: Infinity, ease: 'easeInOut' }}
                />
                <span className="text-sm font-medium text-danger">{t('recorder.recording')}</span>
                <span className="ml-auto text-sm tabular-nums text-muted-foreground">
                  {formatDuration(elapsed)}
                </span>
              </CardContent>
            </Card>
          ) : null}

          {shown.length === 0 ? (
            <EmptyState
              icon={Radio}
              title={t('recorder.empty')}
              description={t('recorder.emptyHint')}
            />
          ) : (
            <div className="scrollbar-thin max-h-[60vh] overflow-y-auto rounded-[var(--radius-md)] bg-[var(--card-muted)] p-2">
              <Timeline events={shown} />
            </div>
          )}
        </div>

        <aside className="flex flex-col gap-4">
          <Card>
            <CardContent className="flex flex-col gap-4 p-4">
              <Field label={t('editor.name')} hint={t('recorder.nameHint')}>
                <Input
                  value={name}
                  maxLength={80}
                  placeholder={t('recorder.title')}
                  onChange={(event) => setName(event.target.value)}
                />
              </Field>

              <SwitchRow
                label={t('recorder.includeMoves')}
                description={t('recorder.includeMovesHint')}
                checked={includeMoves}
                onCheckedChange={setIncludeMoves}
              />

              <Button
                variant="primary"
                disabled={recording || captured.length === 0}
                onClick={() => void saveAsMacro()}
              >
                <Save />
                {t('recorder.saveAsMacro')}
              </Button>
            </CardContent>
          </Card>
        </aside>
      </div>
    </PageShell>
  );
}
