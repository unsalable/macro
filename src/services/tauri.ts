/**
 * The single gate between the UI and the native side (§38).
 * No component calls `invoke` directly.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

import type {
  AppSettings,
  EngineStatus,
  HistoryEntry,
  Macro,
  MacroAction,
  Profile,
  ProfileColor,
  ProfileStore,
  RecordedEvent,
} from '@/types';

export interface RunOutcome {
  macroId: string;
  macroName: string;
  iterations: number;
  durationMs: number;
  completed: boolean;
  error: string | null;
}

export interface RecorderStatus {
  recording: boolean;
  elapsedMs: number;
}

// ------------------------------------------------------------------ engine

export const engine = {
  start: (macroId: string) => invoke<void>('engine_start', { macroId }),
  stop: () => invoke<boolean>('engine_stop'),
  pause: () => invoke<boolean>('engine_pause'),
  resume: () => invoke<boolean>('engine_resume'),
  status: () => invoke<EngineStatus>('engine_status'),
  testAction: (action: MacroAction) => invoke<void>('engine_test_action', { action }),
  /** Hands a macro to its hotkey without running it (§19). */
  arm: (macroId: string) => invoke<void>('engine_arm', { macroId }),
  disarm: () => invoke<void>('engine_disarm'),
  armed: () => invoke<string | null>('engine_armed'),
};

// ------------------------------------------------------------------ macros

export const macros = {
  list: () => invoke<Macro[]>('macro_list'),
  get: (macroId: string) => invoke<Macro | null>('macro_get', { macroId }),
  create: (name: string) => invoke<Macro>('macro_new', { name }),
  save: (value: Macro) => invoke<Macro>('macro_save', { value }),
  remove: (macroId: string) => invoke<boolean>('macro_delete', { macroId }),
  duplicate: (macroId: string) => invoke<Macro>('macro_duplicate', { macroId }),
  exportTo: (macroId: string, path: string) => invoke<void>('macro_export', { macroId, path }),
  importFrom: (path: string) => invoke<Macro>('macro_import', { path }),
};

// ---------------------------------------------------------------- profiles

export const profiles = {
  list: () => invoke<ProfileStore>('profile_list'),
  create: (name: string, color: ProfileColor) => invoke<Profile>('profile_new', { name, color }),
  save: (value: Profile) => invoke<Profile>('profile_save', { value }),
  remove: (profileId: string) => invoke<boolean>('profile_delete', { profileId }),
  switchTo: (profileId: string) => invoke<void>('profile_switch', { profileId }),
};

// ---------------------------------------------------------------- settings

export const settings = {
  get: () => invoke<AppSettings>('settings_get'),
  set: (value: AppSettings) => invoke<AppSettings>('settings_set', { value }),
};

export const history = {
  list: (limit?: number) => invoke<HistoryEntry[]>('history_list', { limit: limit ?? null }),
  clear: () => invoke<void>('history_clear'),
};

// ---------------------------------------------------------------- recorder

export const recorder = {
  status: () => invoke<RecorderStatus>('recorder_status'),
  start: () => invoke<void>('recorder_start'),
  stop: () => invoke<RecordedEvent[]>('recorder_stop'),
  toMacro: (name: string, events: RecordedEvent[], includeMoves: boolean) =>
    invoke<Macro>('recorder_to_macro', { name, events, includeMoves }),
};

// ------------------------------------------------------------------ events

export const events = {
  onStatus: (handler: (status: EngineStatus) => void): Promise<UnlistenFn> =>
    listen<EngineStatus>('engine:status', (event) => handler(event.payload)),
  onFinished: (handler: (outcome: RunOutcome) => void): Promise<UnlistenFn> =>
    listen<RunOutcome>('engine:finished', (event) => handler(event.payload)),
  onError: (handler: (message: string) => void): Promise<UnlistenFn> =>
    listen<string>('engine:error', (event) => handler(event.payload)),
  onRecorded: (handler: (recorded: RecordedEvent) => void): Promise<UnlistenFn> =>
    listen<RecordedEvent>('recorder:event', (event) => handler(event.payload)),
  onHotkey: (handler: (label: string) => void): Promise<UnlistenFn> =>
    listen<string>('hotkey:triggered', (event) => handler(event.payload)),
  onArmed: (handler: (macroId: string | null) => void): Promise<UnlistenFn> =>
    listen<string | null>('hotkey:armed', (event) => handler(event.payload)),
  onDataChanged: (handler: () => void): Promise<UnlistenFn> =>
    listen('data:changed', () => handler()),
};

// ------------------------------------------------------------------ window

export const appWindow = {
  minimize: () => getCurrentWindow().minimize(),
  toggleMaximize: () => getCurrentWindow().toggleMaximize(),
  close: () => getCurrentWindow().close(),
  isMaximized: () => getCurrentWindow().isMaximized(),
  onResized: (handler: () => void) => getCurrentWindow().onResized(() => handler()),
};

/** True when running inside Tauri rather than a plain browser tab. */
export const isNative = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
