import { useEffect, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { Toaster } from 'sonner';

import { Onboarding } from '@/components/common/Onboarding';
import { Sidebar } from '@/components/layout/Sidebar';
import { TitleBar } from '@/components/layout/TitleBar';
import { TooltipProvider } from '@/components/ui/tooltip';
import { useEngineStore } from '@/stores/engineStore';
import { useMacroStore } from '@/stores/macroStore';
import { useProfileStore } from '@/stores/profileStore';
import { events } from '@/services/tauri';

export function AppLayout({ children }: { children: ReactNode }) {
  const { i18n } = useTranslation();
  const subscribe = useEngineStore((store) => store.subscribe);
  const loadMacros = useMacroStore((store) => store.load);
  const loadProfiles = useProfileStore((store) => store.load);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    void subscribe().then((off) => {
      dispose = off;
    });
    return () => dispose?.();
  }, [subscribe]);

  useEffect(() => {
    void loadMacros();
    void loadProfiles();
  }, [loadMacros, loadProfiles]);

  // Rust is the source of truth: when it says the data moved, reload rather
  // than trying to mirror every mutation in the store.
  useEffect(() => {
    let dispose: (() => void) | undefined;
    void events
      .onDataChanged(() => {
        void loadMacros();
        void loadProfiles();
      })
      .then((off) => {
        dispose = off;
      });
    return () => dispose?.();
  }, [loadMacros, loadProfiles]);

  return (
    <TooltipProvider delayDuration={350} skipDelayDuration={200}>
      <div className="flex h-full flex-col bg-background">
        <TitleBar />
        <div className="flex min-h-0 flex-1">
          <Sidebar />
          <main className="min-w-0 flex-1">
            {children}
          </main>
        </div>
      </div>

      <Onboarding />

      <Toaster
        position="bottom-right"
        offset={16}
        toastOptions={{
          classNames: {
            toast:
              'rounded-[var(--radius-md)] border border-[var(--border)] bg-[var(--popover)] text-foreground shadow-[var(--shadow-pop)]',
            description: 'text-muted-foreground',
          },
        }}
        // Sonner reads direction from the document; keep it in step with i18n.
        dir={i18n.dir()}
      />
    </TooltipProvider>
  );
}
