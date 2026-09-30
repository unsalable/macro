import type { Hotkey } from './macro';
import type { Language } from '@/i18n';
import type { AnimationQuality } from '@/lib/motion';

export type ThemeMode = 'light' | 'dark' | 'system';

export interface HotkeyMap {
  start: Hotkey | null;
  stop: Hotkey | null;
  pause: Hotkey | null;
  emergencyStop: Hotkey | null;
}

export interface AppSettings {
  schemaVersion: number;
  language: Language;
  theme: ThemeMode;
  animationQuality: AnimationQuality;
  startWithWindows: boolean;
  startMinimized: boolean;
  closeToTray: boolean;
  showTrayNotifications: boolean;
  confirmBeforeDelete: boolean;
  /** Keeps the 3x-Escape panic stop wired even if hotkeys are misconfigured (§61). */
  panicEscapeEnabled: boolean;
  hotkeys: HotkeyMap;
  historyLimit: number;
  onboardingDone: boolean;
  /** Looks for a new GitHub release on startup. */
  autoUpdateCheck: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  schemaVersion: 1,
  language: 'en',
  theme: 'system',
  animationQuality: 'balanced',
  startWithWindows: false,
  startMinimized: false,
  closeToTray: true,
  showTrayNotifications: true,
  confirmBeforeDelete: true,
  panicEscapeEnabled: true,
  hotkeys: {
    start: { code: 'F6', modifiers: [] },
    stop: { code: 'F7', modifiers: [] },
    pause: { code: 'F8', modifiers: [] },
    emergencyStop: { code: 'F12', modifiers: [] },
  },
  historyLimit: 500,
  onboardingDone: false,
  autoUpdateCheck: true,
};
