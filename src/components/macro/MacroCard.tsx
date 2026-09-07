import { Copy, Download, MoreHorizontal, Pencil, Play, Square, Trash2 } from 'lucide-react';
import { motion } from 'motion/react';
import { useTranslation } from 'react-i18next';

import { Badge, Kbd } from '@/components/ui/misc';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Switch } from '@/components/ui/switch';
import { cn } from '@/lib/cn';
import { formatHotkey } from '@/lib/hotkey';
import { listItemVariants, transition } from '@/lib/motion';
import type { Macro } from '@/types';

interface MacroCardProps {
  macro: Macro;
  index: number;
  active: boolean;
  onOpen: () => void;
  onSelect?: () => void;
  onRun?: () => void;
  onStop?: () => void;
  onToggle?: () => void;
  onDuplicate?: () => void;
  onExport?: () => void;
  onDelete?: () => void;
}

export function MacroCard({
  macro,
  index,
  active,
  onOpen,
  onSelect,
  onRun,
  onStop,
  onToggle,
  onDuplicate,
  onExport,
  onDelete,
}: MacroCardProps) {
  const { t } = useTranslation();

  return (
    <motion.div
      variants={listItemVariants}
      initial="initial"
      animate="animate"
      transition={{ ...transition.normal, delay: index * 0.035 }}
      whileHover={{ y: -2 }}
      onClick={onSelect}
    >
      <Card
        className={cn(
          'group flex h-full flex-col gap-3 p-4 transition-shadow',
          'hover:shadow-[var(--shadow-lift)]',
          active && 'border-accent',
          !macro.enabled && 'opacity-60',
        )}
      >
        <div className="flex items-start justify-between gap-3">
          <button
            type="button"
            onClick={onOpen}
            className="min-w-0 flex-1 text-left text-sm font-medium leading-snug hover:text-accent"
          >
            <span className="line-clamp-2">{macro.name}</span>
          </button>

          <div className="flex shrink-0 items-center gap-1">
            {onToggle ? (
              <Switch
                checked={macro.enabled}
                onCheckedChange={onToggle}
                aria-label={t('common.enabled')}
              />
            ) : null}

            {onDuplicate || onDelete || onExport ? (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="iconSm" aria-label={t('common.edit')}>
                    <MoreHorizontal />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent>
                  <DropdownMenuItem onSelect={onOpen}>
                    <Pencil />
                    {t('common.edit')}
                  </DropdownMenuItem>
                  {onDuplicate ? (
                    <DropdownMenuItem onSelect={onDuplicate}>
                      <Copy />
                      {t('common.duplicate')}
                    </DropdownMenuItem>
                  ) : null}
                  {onExport ? (
                    <DropdownMenuItem onSelect={onExport}>
                      <Download />
                      {t('common.export')}
                    </DropdownMenuItem>
                  ) : null}
                  {onDelete ? (
                    <>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem destructive onSelect={onDelete}>
                        <Trash2 />
                        {t('common.delete')}
                      </DropdownMenuItem>
                    </>
                  ) : null}
                </DropdownMenuContent>
              </DropdownMenu>
            ) : null}
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Badge tone={macro.actions.length > 0 ? 'neutral' : 'warning'}>
            {t('macros.actionsCount', { count: macro.actions.length })}
          </Badge>
          {macro.hotkey ? <Kbd>{formatHotkey(macro.hotkey)}</Kbd> : null}
          {macro.loop.mode !== 'none' ? (
            <Badge tone="accent">{t(`editor.loop${capitalize(macro.loop.mode)}`)}</Badge>
          ) : null}
        </div>

        <div className="mt-auto flex items-center justify-between gap-2 pt-1">
          <span className="text-xs text-muted-foreground">
            {t('macros.runCount')} · {macro.stats.runCount}
          </span>

          {active && onStop ? (
            <Button variant="danger" size="sm" onClick={onStop}>
              <Square />
              {t('engine.stop')}
            </Button>
          ) : onRun ? (
            <Button
              variant="outline"
              size="sm"
              disabled={macro.actions.length === 0}
              onClick={onRun}
              className="opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100"
            >
              <Play />
              {t('engine.start')}
            </Button>
          ) : null}
        </div>
      </Card>
    </motion.div>
  );
}

function capitalize(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}
