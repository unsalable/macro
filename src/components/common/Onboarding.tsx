import { Keyboard, MousePointerClick, ShieldAlert } from 'lucide-react';
import { motion } from 'motion/react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Kbd } from '@/components/ui/misc';
import { listItemVariants, transition } from '@/lib/motion';
import { useSettingsStore } from '@/stores/settingsStore';

/** One screen, shown once (§52). Its job is the safety net, not a tour. */
export function Onboarding() {
  const { t } = useTranslation();
  const loaded = useSettingsStore((store) => store.loaded);
  const done = useSettingsStore((store) => store.settings.onboardingDone);
  const update = useSettingsStore((store) => store.update);

  if (!loaded || done) return null;

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={transition.normal}
      className="fixed inset-0 z-[60] flex items-center justify-center bg-[var(--overlay)] backdrop-blur-sm"
    >
      <motion.div
        initial={{ opacity: 0, y: 12, scale: 0.98 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        transition={transition.spring}
        className="w-full max-w-md rounded-[var(--radius-lg)] border border-[var(--border)] bg-card p-7 shadow-[var(--shadow-pop)]"
      >
        <h2 className="text-xl font-semibold tracking-tight">{t('onboarding.title')}</h2>
        <p className="mt-2 text-[13px] leading-relaxed text-muted-foreground">
          {t('onboarding.body')}
        </p>

        <ul className="mt-6 flex flex-col gap-3">
          {[
            { icon: MousePointerClick, text: t('onboarding.pointOne') },
            { icon: Keyboard, text: t('onboarding.pointTwo') },
            { icon: ShieldAlert, text: t('onboarding.pointThree') },
          ].map((item, index) => (
            <motion.li
              key={item.text}
              variants={listItemVariants}
              initial="initial"
              animate="animate"
              transition={{ ...transition.normal, delay: 0.08 + index * 0.05 }}
              className="flex items-start gap-3"
            >
              <span className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-[8px] bg-[var(--accent-soft)] text-accent">
                <item.icon className="size-3.5" />
              </span>
              <span className="text-[13px] leading-relaxed">{item.text}</span>
            </motion.li>
          ))}
        </ul>

        <div className="mt-6 flex items-center justify-between gap-4">
          <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Kbd>Esc</Kbd>
            <Kbd>Esc</Kbd>
            <Kbd>Esc</Kbd>
          </span>
          <Button variant="accent" onClick={() => void update({ onboardingDone: true })}>
            {t('onboarding.start')}
          </Button>
        </div>
      </motion.div>
    </motion.div>
  );
}
