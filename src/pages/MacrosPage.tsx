import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import { Plus, Search, Upload, Zap } from 'lucide-react';
import { motion } from 'motion/react';
import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';

import { ConfirmDialog } from '@/components/common/ConfirmDialog';
import { EmptyState } from '@/components/common/EmptyState';
import { PageShell } from '@/components/layout/PageShell';
import { MacroCard } from '@/components/macro/MacroCard';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { macros as api } from '@/services/tauri';
import { listVariants } from '@/lib/motion';
import { useEngineStore } from '@/stores/engineStore';
import { filterMacros, useMacroStore } from '@/stores/macroStore';
import { useSettingsStore } from '@/stores/settingsStore';
import { useUiStore } from '@/stores/uiStore';

export function MacrosPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { macros, query, setQuery, save, remove, duplicate, toggleEnabled, load } = useMacroStore();
  const status = useEngineStore((store) => store.status);
  const toggleRun = useEngineStore((store) => store.toggle);
  const confirmBeforeDelete = useSettingsStore((store) => store.settings.confirmBeforeDelete);
  const selectMacro = useUiStore((store) => store.selectMacro);
  const [pendingDelete, setPendingDelete] = useState<string | null>(null);

  const visible = useMemo(() => filterMacros(macros, query), [macros, query]);
  const pendingName = macros.find((item) => item.id === pendingDelete)?.name ?? '';

  const handleCreate = async () => {
    const draft = await api.create(t('macros.new'));
    const saved = await save(draft);
    selectMacro(saved.id);
    void navigate(`/macros/${saved.id}`);
  };

  const handleDelete = async (macroId: string) => {
    await remove(macroId);
    toast.success(t('toast.deleted'));
  };

  const handleExport = async (macroId: string) => {
    const name = macros.find((item) => item.id === macroId)?.name ?? 'macro';
    const path = await saveDialog({
      defaultPath: `${name}.flowmacro.json`,
      filters: [{ name: 'FlowMacro', extensions: ['json'] }],
    });
    if (!path) return;
    await api.exportTo(macroId, path);
    toast.success(t('toast.exported'));
  };

  const handleImport = async () => {
    const path = await openDialog({
      multiple: false,
      filters: [{ name: 'FlowMacro', extensions: ['json'] }],
    });
    if (typeof path !== 'string') return;
    try {
      await api.importFrom(path);
      await load();
      toast.success(t('toast.imported'));
    } catch (error) {
      toast.error(t('error.invalidFile'), { description: String(error) });
    }
  };

  return (
    <PageShell
      title={t('macros.title')}
      description={t('macros.subtitle')}
      actions={
        <>
          <Button variant="ghost" onClick={() => void handleImport()}>
            <Upload />
            {t('common.import')}
          </Button>
          <Button variant="accent" onClick={() => void handleCreate()}>
            <Plus />
            {t('macros.new')}
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-5">
        <div className="relative max-w-sm">
          <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t('macros.searchPlaceholder')}
            className="pl-9"
          />
        </div>

        {visible.length === 0 ? (
          <EmptyState
            icon={Zap}
            title={query ? t('common.search') : t('macros.empty')}
            description={t('macros.emptyHint')}
            action={
              <Button variant="accent" onClick={() => void handleCreate()}>
                <Plus />
                {t('macros.new')}
              </Button>
            }
          />
        ) : (
          <motion.div
            variants={listVariants}
            initial="initial"
            animate="animate"
            className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3"
          >
            {visible.map((item, index) => (
              <MacroCard
                key={item.id}
                macro={item}
                index={index}
                active={status.macroId === item.id && status.state !== 'idle'}
                onOpen={() => void navigate(`/macros/${item.id}`)}
                onSelect={() => selectMacro(item.id)}
                onRun={() => void toggleRun(item.id)}
                onStop={() => void toggleRun(item.id)}
                onToggle={() => void toggleEnabled(item.id)}
                onDuplicate={() => void duplicate(item.id)}
                onExport={() => void handleExport(item.id)}
                onDelete={() =>
                  confirmBeforeDelete ? setPendingDelete(item.id) : void handleDelete(item.id)
                }
              />
            ))}
          </motion.div>
        )}

      </div>

      <ConfirmDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => !open && setPendingDelete(null)}
        title={t('common.delete')}
        description={pendingName}
        onConfirm={() => {
          if (pendingDelete) void handleDelete(pendingDelete);
        }}
      />
    </PageShell>
  );
}
