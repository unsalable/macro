import { Check, Layers, Plus, Trash2 } from 'lucide-react';
import { motion } from 'motion/react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { ConfirmDialog } from '@/components/common/ConfirmDialog';
import { EmptyState } from '@/components/common/EmptyState';
import { PageShell } from '@/components/layout/PageShell';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { cn } from '@/lib/cn';
import { listVariants, listItemVariants, transition } from '@/lib/motion';
import { activeProfile, useProfileStore } from '@/stores/profileStore';
import { PROFILE_COLORS, type ProfileColor } from '@/types';

/** A deliberately small palette: profiles should stay easy to tell apart (§65). */
const SWATCH: Record<ProfileColor, string> = {
  sand: '#C9B79C',
  clay: '#B98B7E',
  sage: '#93A98D',
  slate: '#8C99A6',
  plum: '#A38BA6',
  ochre: '#C4A167',
};

export function ProfilesPage() {
  const { t } = useTranslation();
  const { store, create, save, remove, switchTo } = useProfileStore();
  const [name, setName] = useState('');
  const [color, setColor] = useState<ProfileColor>('sand');
  const [pendingDelete, setPendingDelete] = useState<string | null>(null);

  const current = activeProfile(store);

  const handleCreate = async () => {
    if (!name.trim()) return;
    await create(name.trim(), color);
    setName('');
    toast.success(t('toast.saved'));
  };

  return (
    <PageShell title={t('profiles.title')} description={t('nav.profiles')}>
      <div className="flex flex-col gap-6">
        <Card>
          <CardContent className="flex flex-wrap items-end gap-3 p-4">
            <div className="flex min-w-56 flex-1 flex-col gap-1.5">
              <span className="text-[13px] font-medium">{t('profiles.new')}</span>
              <Input
                value={name}
                maxLength={40}
                onChange={(event) => setName(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === 'Enter') void handleCreate();
                }}
              />
            </div>

            <div className="flex items-center gap-1.5 pb-1">
              {PROFILE_COLORS.map((item) => (
                <button
                  key={item}
                  type="button"
                  aria-label={item}
                  onClick={() => setColor(item)}
                  style={{ backgroundColor: SWATCH[item] }}
                  className={cn(
                    'size-6 rounded-full border-2 transition-transform',
                    color === item ? 'border-foreground scale-110' : 'border-transparent',
                  )}
                />
              ))}
            </div>

            <Button variant="accent" disabled={!name.trim()} onClick={() => void handleCreate()}>
              <Plus />
              {t('common.add')}
            </Button>
          </CardContent>
        </Card>

        {!store || store.profiles.length === 0 ? (
          <EmptyState icon={Layers} title={t('profiles.empty')} />
        ) : (
          <motion.div
            variants={listVariants}
            initial="initial"
            animate="animate"
            className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3"
          >
            {store.profiles.map((profile) => {
              const isActive = current?.id === profile.id;
              return (
                <motion.div key={profile.id} variants={listItemVariants} transition={transition.normal}>
                  <Card
                    className={cn(
                      'relative flex h-full flex-col gap-3 p-4 transition-shadow hover:shadow-[var(--shadow-lift)]',
                      isActive && 'border-accent',
                    )}
                  >
                    <div className="flex items-center gap-3">
                      <span
                        className="size-8 shrink-0 rounded-full"
                        style={{ backgroundColor: SWATCH[profile.color] }}
                      />
                      <div className="min-w-0 flex-1">
                        <Input
                          value={profile.name}
                          maxLength={40}
                          onChange={(event) =>
                            void save({ ...profile, name: event.target.value })
                          }
                          className="h-8 border-transparent bg-transparent px-0 text-sm font-medium"
                        />
                        <span className="text-xs text-muted-foreground">
                          {t('macros.actionsCount', { count: profile.macros.length })}
                        </span>
                      </div>
                    </div>

                    <div className="mt-auto flex items-center justify-between gap-2">
                      {isActive ? (
                        <span className="inline-flex items-center gap-1.5 text-xs font-medium text-success">
                          <Check className="size-3.5" />
                          {t('profiles.active')}
                        </span>
                      ) : (
                        <Button variant="outline" size="sm" onClick={() => void switchTo(profile.id)}>
                          {t('profiles.switchTo')}
                        </Button>
                      )}

                      <Button
                        variant="ghost"
                        size="iconSm"
                        disabled={store.profiles.length <= 1}
                        onClick={() => setPendingDelete(profile.id)}
                        aria-label={t('common.delete')}
                        className="hover:text-danger"
                      >
                        <Trash2 />
                      </Button>
                    </div>
                  </Card>
                </motion.div>
              );
            })}
          </motion.div>
        )}
      </div>

      <ConfirmDialog
        open={pendingDelete !== null}
        onOpenChange={(open) => !open && setPendingDelete(null)}
        title={t('common.delete')}
        description={store?.profiles.find((item) => item.id === pendingDelete)?.name}
        onConfirm={() => {
          if (pendingDelete) void remove(pendingDelete);
        }}
      />
    </PageShell>
  );
}
