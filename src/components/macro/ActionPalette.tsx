import { motion } from 'motion/react';
import { useTranslation } from 'react-i18next';

import { ACTION_ICONS } from '@/lib/actions';
import { listItemVariants, transition } from '@/lib/motion';
import { ACTION_TYPES, type ActionType } from '@/types';

interface ActionPaletteProps {
  onAdd: (type: ActionType) => void;
}

export function ActionPalette({ onAdd }: ActionPaletteProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-2">
      <h3 className="text-[13px] font-semibold uppercase tracking-wide text-muted-foreground">
        {t('editor.palette')}
      </h3>

      <div className="flex flex-col gap-1">
        {ACTION_TYPES.map((type, index) => {
          const Icon = ACTION_ICONS[type];
          return (
            <motion.button
              key={type}
              type="button"
              variants={listItemVariants}
              initial="initial"
              animate="animate"
              transition={{ ...transition.fast, delay: index * 0.02 }}
              whileHover={{ x: 2 }}
              onClick={() => onAdd(type)}
              className="flex items-center gap-2.5 rounded-[var(--radius-sm)] border border-[var(--border)] bg-card px-3 py-2 text-left text-[13px] transition-colors hover:border-accent hover:bg-[var(--card-muted)]"
            >
              <span className="flex size-6 items-center justify-center rounded-[7px] bg-[var(--accent-soft)] text-accent">
                <Icon className="size-3.5" />
              </span>
              {t(`action.${type}`)}
            </motion.button>
          );
        })}
      </div>
    </div>
  );
}
