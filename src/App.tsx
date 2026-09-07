import { AnimatePresence } from 'motion/react';
import { useEffect, useRef } from 'react';
import { HashRouter, Route, Routes, useLocation } from 'react-router-dom';

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

export default function App() {
  const load = useSettingsStore((store) => store.load);
  const quality = useSettingsStore((store) => store.settings.animationQuality);

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
