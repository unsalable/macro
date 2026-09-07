import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import { restrictToParentElement, restrictToVerticalAxis } from '@dnd-kit/modifiers';
import {
  SortableContext,
  arrayMove,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { ArrowLeft, ListPlus, Redo2, Save, Undo2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate, useParams } from 'react-router-dom';
import { toast } from 'sonner';

import { EmptyState } from '@/components/common/EmptyState';
import { PageShell } from '@/components/layout/PageShell';
import { ActionCard } from '@/components/macro/ActionCard';
import { ActionPalette } from '@/components/macro/ActionPalette';
import { LoopEditor } from '@/components/macro/LoopEditor';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { HotkeyInput } from '@/components/ui/hotkey-input';
import { Field, Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useUndoRedo } from '@/hooks/useUndoRedo';
import { createAction, estimateDurationMs } from '@/lib/actions';
import { hotkeysEqual } from '@/lib/hotkey';
import { formatDuration } from '@/lib/format';
import { engine as engineApi } from '@/services/tauri';
import { useEngineStore } from '@/stores/engineStore';
import { useMacroStore } from '@/stores/macroStore';
import { useSettingsStore } from '@/stores/settingsStore';
import type { Activation, ActionType, Hotkey, Macro, MacroAction } from '@/types';

export function EditorPage() {
  const { macroId } = useParams<{ macroId: string }>();
  const { t } = useTranslation();
  const navigate = useNavigate();
  const macros = useMacroStore((store) => store.macros);
  const save = useMacroStore((store) => store.save);
  const status = useEngineStore((store) => store.status);
  const globalHotkeys = useSettingsStore((store) => store.settings.hotkeys);

  const source = useMemo(() => macros.find((item) => item.id === macroId), [macros, macroId]);
  const { state: draft, set, reset, undo, redo, canUndo, canRedo } = useUndoRedo<Macro | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    if (source && (!draft || draft.id !== source.id)) {
      reset(source);
      setDirty(false);
    }
  }, [source, draft, reset]);

  const update = useCallback(
    (mutate: (current: Macro) => Macro) => {
      set((current) => (current ? mutate(current) : current));
      setDirty(true);
    },
    [set],
  );

  const handleSave = useCallback(async () => {
    if (!draft) return;
    await save(draft);
    setDirty(false);
    toast.success(t('toast.saved'));
  }, [draft, save, t]);

  // Ctrl+S / Ctrl+Z / Ctrl+Shift+Z, the shortcuts people try first (§58).
  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (!event.ctrlKey) return;
      if (event.key === 's') {
        event.preventDefault();
        void handleSave();
      } else if (event.key === 'z' && !event.shiftKey) {
        event.preventDefault();
        undo();
      } else if ((event.key === 'z' && event.shiftKey) || event.key === 'y') {
        event.preventDefault();
        redo();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [handleSave, undo, redo]);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  if (!draft) {
    return (
      <PageShell title={t('editor.title')}>
        <EmptyState icon={ListPlus} title={t('common.loading')} />
      </PageShell>
    );
  }

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    update((current) => {
      const from = current.actions.findIndex((item) => item.id === active.id);
      const to = current.actions.findIndex((item) => item.id === over.id);
      if (from < 0 || to < 0) return current;
      return { ...current, actions: arrayMove(current.actions, from, to) };
    });
  };

  const addAction = (type: ActionType) => {
    const action = createAction(type);
    update((current) => ({ ...current, actions: [...current.actions, action] }));
    setExpanded(action.id);
  };

  const changeAction = (next: MacroAction) => {
    update((current) => ({
      ...current,
      actions: current.actions.map((item) => (item.id === next.id ? next : item)),
    }));
  };

  const removeAction = (actionId: string) => {
    update((current) => ({
      ...current,
      actions: current.actions.filter((item) => item.id !== actionId),
    }));
  };

  /** A shortcut can only mean one thing: reject anything already spoken for. */
  const assignHotkey = (hotkey: Hotkey | null) => {
    if (hotkey) {
      const takenByGlobal = Object.values(globalHotkeys).some((existing) =>
        hotkeysEqual(existing, hotkey),
      );
      const takenByMacro = macros.some(
        (item) => item.id !== draft.id && hotkeysEqual(item.hotkey, hotkey),
      );
      if (takenByGlobal || takenByMacro) {
        toast.error(t('error.hotkeyTaken'));
        return;
      }
    }
    update((current) => ({ ...current, hotkey }));
  };

  const testAction = async (action: MacroAction) => {
    try {
      await engineApi.testAction(action);
    } catch (error) {
      toast.error(t('error.generic'), { description: String(error) });
    }
  };

  const runningIndex =
    status.macroId === draft.id && status.state === 'running' ? status.actionIndex : -1;

  return (
    <PageShell
      title={draft.name}
      description={`${t('macros.actionsCount', { count: draft.actions.length })} · ${formatDuration(
        estimateDurationMs(draft.actions),
      )}`}
      actions={
        <>
          <Button variant="ghost" size="icon" onClick={() => void navigate('/macros')} aria-label={t('common.back')}>
            <ArrowLeft />
          </Button>
          <Button variant="ghost" size="icon" disabled={!canUndo} onClick={undo} aria-label={t('common.undo')}>
            <Undo2 />
          </Button>
          <Button variant="ghost" size="icon" disabled={!canRedo} onClick={redo} aria-label={t('common.redo')}>
            <Redo2 />
          </Button>
          <Button variant="accent" disabled={!dirty} onClick={() => void handleSave()}>
            <Save />
            {t('common.save')}
          </Button>
        </>
      }
    >
      <div className="grid grid-cols-1 gap-6 xl:grid-cols-[minmax(0,1fr)_300px]">
        <div className="flex flex-col gap-3">
          {draft.actions.length === 0 ? (
            <EmptyState
              icon={ListPlus}
              title={t('editor.emptyActions')}
              description={t('editor.emptyActionsHint')}
            />
          ) : (
            <DndContext
              sensors={sensors}
              collisionDetection={closestCenter}
              modifiers={[restrictToVerticalAxis, restrictToParentElement]}
              onDragEnd={handleDragEnd}
            >
              <SortableContext
                items={draft.actions.map((item) => item.id)}
                strategy={verticalListSortingStrategy}
              >
                <div className="flex flex-col gap-2">
                  {draft.actions.map((action, index) => (
                    <ActionCard
                      key={action.id}
                      action={action}
                      index={index}
                      expanded={expanded === action.id}
                      running={runningIndex === index}
                      onToggleExpanded={() =>
                        setExpanded((current) => (current === action.id ? null : action.id))
                      }
                      onChange={changeAction}
                      onRemove={() => removeAction(action.id)}
                      onTest={() => void testAction(action)}
                    />
                  ))}
                </div>
              </SortableContext>
            </DndContext>
          )}
        </div>

        <aside className="flex flex-col gap-5">
          <Card>
            <CardContent className="flex flex-col gap-4 p-4">
              <Field label={t('editor.name')}>
                <Input
                  value={draft.name}
                  maxLength={80}
                  onChange={(event) => update((current) => ({ ...current, name: event.target.value }))}
                />
              </Field>

              <Field label={t('settings.hotkeys')}>
                <HotkeyInput
                  value={draft.hotkey}
                  onChange={assignHotkey}
                />
              </Field>

              <Field label={t('editor.activation')}>
                <Select
                  value={draft.activation}
                  onValueChange={(value) =>
                    update((current) => ({ ...current, activation: value as Activation }))
                  }
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="toggle">{t('editor.activationToggle')}</SelectItem>
                    <SelectItem value="hold">{t('editor.activationHold')}</SelectItem>
                  </SelectContent>
                </Select>
              </Field>
            </CardContent>
          </Card>

          <Card>
            <CardContent className="p-4">
              <LoopEditor
                loop={draft.loop}
                randomization={draft.randomization}
                onLoopChange={(loop) => update((current) => ({ ...current, loop }))}
                onRandomizationChange={(randomization) =>
                  update((current) => ({ ...current, randomization }))
                }
              />
            </CardContent>
          </Card>

          <Card>
            <CardContent className="p-4">
              <ActionPalette onAdd={addAction} />
            </CardContent>
          </Card>
        </aside>
      </div>
    </PageShell>
  );
}
