import { create } from 'zustand';

import { engine as api, events } from '@/services/tauri';
import { IDLE_STATUS, type EngineStatus } from '@/types';

interface EngineState {
  status: EngineStatus;
  /** The macro whose hotkey is live, set from the Start button (§19). */
  armedMacroId: string | null;
  lastError: string | null;
  subscribe: () => Promise<() => void>;
  start: (macroId: string) => Promise<void>;
  stop: () => Promise<void>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  toggle: (macroId: string) => Promise<void>;
  arm: (macroId: string) => Promise<void>;
  disarm: () => Promise<void>;
  clearError: () => void;
}

export const useEngineStore = create<EngineState>((set, get) => ({
  status: IDLE_STATUS,
  armedMacroId: null,
  lastError: null,

  /**
   * Status arrives as a 10 Hz push from Rust; the UI never polls (§2.4).
   * Returns a cleanup function for the effect that called it.
   */
  subscribe: async () => {
    const unlisteners = await Promise.all([
      events.onStatus((status) => set({ status })),
      events.onError((message) => set({ lastError: message })),
      events.onFinished(() => set({ status: IDLE_STATUS })),
      // Arming survives a run ending, so the key stays live for the next one.
      events.onArmed((macroId) => set({ armedMacroId: macroId })),
    ]);

    // Catch up once, in case a run started before this subscription.
    try {
      set({ status: await api.status(), armedMacroId: await api.armed() });
    } catch {
      // The listeners above still deliver the next push.
    }

    return () => unlisteners.forEach((off) => off());
  },

  /**
   * The state flips locally before the round trip so the button changes on the
   * same frame as the press. Rust's next 10 Hz push (or the catch-up read in
   * the error path) is still what decides the truth.
   */
  start: async (macroId) => {
    set({ lastError: null, status: { ...IDLE_STATUS, state: 'running', macroId } });
    try {
      await api.start(macroId);
    } catch (error) {
      set({ status: await api.status().catch(() => IDLE_STATUS) });
      throw error;
    }
  },

  stop: async () => {
    const { status } = get();
    if (status.state !== 'idle') set({ status: { ...status, state: 'stopping' } });
    await api.stop();
  },

  pause: async () => {
    await api.pause();
  },

  resume: async () => {
    await api.resume();
  },

  toggle: async (macroId) => {
    const { status } = get();
    if (status.state !== 'idle' && status.macroId === macroId) {
      await api.stop();
    } else if (status.state === 'idle') {
      await get().start(macroId);
    }
  },

  /**
   * Arming does not click anything: it makes the macro the one its hotkey —
   * and the global Start hotkey — will run. Rust stays the source of truth and
   * answers with an event, so a key pressed elsewhere updates this too.
   */
  arm: async (macroId) => {
    set({ lastError: null, armedMacroId: macroId });
    try {
      await api.arm(macroId);
    } catch (error) {
      set({ armedMacroId: await api.armed().catch(() => null) });
      throw error;
    }
  },

  disarm: async () => {
    set({ armedMacroId: null });
    await api.disarm();
  },

  clearError: () => set({ lastError: null }),
}));

export const selectIsBusy = (state: EngineState): boolean => state.status.state !== 'idle';

