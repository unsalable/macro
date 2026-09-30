import { AnimatePresence } from 'motion/react';
import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { HashRouter, Route, Routes, useLocation } from 'react-router-dom';
import { toast } from 'sonner';

import { MotionProvider } from '@/components/layout/MotionProvider';
import { NavDirectionContext, navIndex } from '@/lib/nav';
import { AppLayout } from '@/layouts/AppLayout';
import { EditorPage } from '@/pages/EditorPage';
import { HistoryPage } from '@/pages/HistoryPage';
import { HomePage } from '@/pages/HomePage';
import { KeyboardPage } from '@/pages/KeyboardPage';
import { MacrosPage } from '@/pages/MacrosPage';
import { MousePage } from '@/pages/MousePage';
import { ProfilesPage } from '@/pages/ProfilesPage';
import { RecorderPage } from '@/pages/RecorderPage';
import { SettingsPage } from '@/pages/SettingsPage';
import { useSettingsStore, watchSystemTheme } from '@/stores/settingsStore';
import { useUpdateStore } from '@/stores/updateStore';

function AnimatedRoutes() {
  const location = useLocation();

  // The page travels the same way the sidebar selection does. The previous
  // index lives in a ref so the direction is known on the very render that
  // swaps the route, before any effect has run.
  const index = navIndex(location.pathname);
  const previous = useRef(index);
  const direction = index < previous.current ? -1 : 1;

  useEffect(() => {
    previous.current = index;
  }, [index]);

  return (
    <NavDirectionContext.Provider value={direction}>
      <AnimatePresence mode="wait" initial={false} custom={direction}>
        {/* An explicit location plus a key is what lets the outgoing page
            finish its exit before the next one mounts (§29). */}
        <Routes location={location} key={location.pathname}>
          <Route path="/" element={<HomePage />} />
          <Route path="/macros" element={<MacrosPage />} />
          <Route path="/macros/:macroId" element={<EditorPage />} />
          <Route path="/recorder" element={<RecorderPage />} />
          <Route path="/mouse" element={<MousePage />} />
          <Route path="/keyboard" element={<KeyboardPage />} />
          <Route path="/profiles" element={<ProfilesPage />} />
          <Route path="/history" element={<HistoryPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="*" element={<HomePage />} />
        </Routes>
      </AnimatePresence>
    </NavDirectionContext.Provider>
  );
}

/**
 * A desktop shell has no browser chrome, so the webview's built-in gestures
 * only ever misfire here: the side buttons walk the hash history (and are
 * bindable as hotkeys, so they get pressed on purpose), and dragging a nav
 * link hands the user a `tauri.localhost` URL ghost.
 */
function useShellGestureGuards() {
  useEffect(() => {
    const isSideButton = (event: MouseEvent) => event.button === 3 || event.button === 4;

    const blockNavigation = (event: MouseEvent) => {
      if (isSideButton(event)) event.preventDefault();
    };
    const blockDrag = (event: DragEvent) => event.preventDefault();

    window.addEventListener('mousedown', blockNavigation, { capture: true });
    window.addEventListener('mouseup', blockNavigation, { capture: true });
    window.addEventListener('auxclick', blockNavigation, { capture: true });
    window.addEventListener('dragstart', blockDrag, { capture: true });
    return () => {
      window.removeEventListener('mousedown', blockNavigation, { capture: true });
      window.removeEventListener('mouseup', blockNavigation, { capture: true });
      window.removeEventListener('auxclick', blockNavigation, { capture: true });
      window.removeEventListener('dragstart', blockDrag, { capture: true });
    };
  }, []);
}

/**
 * One quiet look at the release feed after startup. It never interrupts: a
 * failed check says nothing, and a found update is a toast the user can
 * ignore — the same update is waiting in Settings whenever they want it.
 */
function useStartupUpdateCheck() {
  const { t } = useTranslation();
  const loaded = useSettingsStore((store) => store.loaded);
  const enabled = useSettingsStore((store) => store.settings.autoUpdateCheck);
  const asked = useRef(false);

  useEffect(() => {
    if (!loaded || !enabled || asked.current) return undefined;
    asked.current = true;

    // Late enough that the check never competes with the first paint or with
    // the hook install that the engine depends on.
    const timer = window.setTimeout(() => {
      void useUpdateStore
        .getState()
        .check({ silent: true })
        .then((found) => {
          if (!found) return;
          const version = useUpdateStore.getState().version;
          toast(t('update.available', { version }), {
            duration: 12_000,
            action: {
              label: t('update.install'),
              onClick: () => void useUpdateStore.getState().install(),
            },
          });
        });
    }, 4_000);

    return () => window.clearTimeout(timer);
  }, [enabled, loaded, t]);
}

export default function App() {
  const load = useSettingsStore((store) => store.load);
  const quality = useSettingsStore((store) => store.settings.animationQuality);

  useShellGestureGuards();
  useStartupUpdateCheck();

  useEffect(() => {
    void load();
    return watchSystemTheme();
  }, [load]);

  return (
    <MotionProvider quality={quality}>
      <HashRouter>
        <AppLayout>
          <AnimatedRoutes />
        </AppLayout>
      </HashRouter>
    </MotionProvider>
  );
}
