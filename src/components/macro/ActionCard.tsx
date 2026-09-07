import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { ChevronDown, GripVertical, Play, Trash2 } from 'lucide-react';
import { motion } from 'motion/react';
import { useTranslation } from 'react-i18next';

import { ActionFields } from '@/components/macro/ActionFields';
import { Button } from '@/components/ui/button';
import { NumberInput } from '@/components/ui/input';
import { cn } from '@/lib/cn';
import { ACTION_ICONS, describeAction } from '@/lib/actions';
import { transition } from '@/lib/motion';
import type { MacroAction } from '@/types';

interface ActionCardProps {
  action: MacroAction;
  index: number;
  expanded: boolean;
  running: boolean;
  onToggleExpanded: () => void;
  onChange: (next: MacroAction) => void;
  onRemove: () => void;
  onTest: () => void;
}

export function ActionCard({
  action,
  index,
  expanded,
  running,
  onToggleExpanded,
  onChange,
  onRemove,
  onTest,
}: ActionCardProps) {
  const { t } = useTranslation();
  const { attributes, listeners, setNodeRef, transform, transition: dndTransition, isDragging } =
    useSortable({ id: action.id });
  const Icon = ACTION_ICONS[action.type];

  return (
    <div
      ref={setNodeRef}
      style={{ transform: CSS.Translate.toString(transform), transition: dndTransition }}
      className={cn(
        'rounded-[var(--radius-md)] border bg-card transition-shadow',
        isDragging
          ? 'z-10 border-accent shadow-[var(--shadow-pop)]'
          : 'border-[var(--border)] shadow-[var(--shadow-soft)]',
        running && 'ring-2 ring-[var(--ring)]',
      )}
    >
      <div className="flex items-center gap-2 p-2.5">
        <button
          type="button"
          {...attributes}
          {...listeners}
          aria-label={t('common.edit')}
          className="cursor-grab touch-none rounded-[8px] p-1 text-muted-foreground transition-colors hover:bg-secondary active:cursor-grabbing"
        >
          <GripVertical className="size-4" />
        </button>

        <span className="w-6 shrink-0 text-center text-xs tabular-nums text-muted-foreground">
          {index + 1}
        </span>

        <span className="flex size-7 shrink-0 items-center justify-center rounded-[8px] bg-[var(--accent-soft)] text-accent">
          <Icon className="size-3.5" />
        </span>

        <button
          type="button"
          onClick={onToggleExpanded}
          className="flex min-w-0 flex-1 items-baseline gap-2 text-left"
        >
          <span className="shrink-0 text-[13px] font-medium">{t(`action.${action.type}`)}</span>
          <span className="truncate text-xs text-muted-foreground">{describeAction(action, t)}</span>
        </button>

        <Button variant="ghost" size="iconSm" onClick={onTest} aria-label={t('common.test')}>
          <Play />
        </Button>
        <Button
          variant="ghost"
          size="iconSm"
          onClick={onRemove}
          aria-label={t('common.delete')}
          className="hover:text-danger"
        >
          <Trash2 />
        </Button>
        <Button variant="ghost" size="iconSm" onClick={onToggleExpanded} aria-label={t('common.edit')}>
          <motion.span animate={{ rotate: expanded ? 180 : 0 }} transition={transition.fast}>
            <ChevronDown className="size-4" />
          </motion.span>
        </Button>
      </div>

      <motion.div
        initial={false}
        animate={{ height: expanded ? 'auto' : 0, opacity: expanded ? 1 : 0 }}
        transition={transition.normal}
        className="overflow-hidden"
      >
        <div className="flex flex-col gap-4 border-t border-[var(--border)] p-4">
          <ActionFields action={action} onChange={onChange} />

          <div className="grid grid-cols-2 gap-4 border-t border-[var(--border)] pt-4">
            <label className="flex flex-col gap-1.5">
              <span className="text-[13px] font-medium">{t('action.delayBefore')}</span>
              <NumberInput
                value={action.delayBeforeMs}
                min={0}
                suffix="ms"
                onValueChange={(delayBeforeMs) => onChange({ ...action, delayBeforeMs })}
              />
            </label>
            <label className="flex flex-col gap-1.5">
              <span className="text-[13px] font-medium">{t('action.delayAfter')}</span>
              <NumberInput
                value={action.delayAfterMs}
                min={0}
                suffix="ms"
                onValueChange={(delayAfterMs) => onChange({ ...action, delayAfterMs })}
              />
            </label>
          </div>
        </div>
      </motion.div>
    </div>
  );
}
