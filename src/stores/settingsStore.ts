import { create } from 'zustand';

import i18n from '@/i18n';
import { settings as api } from '@/services/tauri';
import { DEFAULT_SETTINGS, type AppSettings, type ThemeMode } from '@/types';

interface SettingsState {
  settings: AppSettings;
  loaded: boolean;
  load: () => Promise<void>;
  update: (patch: Partial<AppSettings>) => Promise<void>;
}

const media = typeof window !== 'undefined' ? window.matchMedia('(prefers-color-scheme: dark)') : null;

function resolveDark(theme: ThemeMode): boolean {
  if (theme === 'dark') return true;
  if (theme === 'light') return false;
  return media?.matches ?? true;
}

export function applyTheme(theme: ThemeMode): void {
  document.documentElement.classList.toggle('dark', resolveDark(theme));
}

/** Applied side effects that live outside React: theme class and locale. */
function applySettings(value: AppSettings): void {
  applyTheme(value.theme);
  if (i18n.language !== value.language) {
    void i18n.changeLanguage(value.language);
  }
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  loaded: false,

  load: async () => {
    try {
      const value = await api.get();
      applySettings(value);
      set({ settings: value, loaded: true });
    } catch {
      // Fall back to the defaults already in the store rather than blocking
      // the whole app on a settings read. The language stays on whatever the
      // system detection picked — forcing English here would be a regression.
      applyTheme(DEFAULT_SETTINGS.theme);
      set({ loaded: true });
    }
  },

  update: async (patch) => {
    const next = { ...get().settings, ...patch } satisfies AppSettings;
    // Optimistic: the UI should not wait a round trip to flip a switch.
    applySettings(next);
    set({ settings: next });
    const saved = await api.set(next);
    applySettings(saved);
    set({ settings: saved });
  },
}));

/** Keeps `theme: 'system'` in sync when the OS switches appearance. */
export function watchSystemTheme(): () => void {
  if (!media) return () => {};
  const handler = () => {
    if (useSettingsStore.getState().settings.theme === 'system') {
      applyTheme('system');
    }
  };
  media.addEventListener('change', handler);
  return () => media.removeEventListener('change', handler);
}
