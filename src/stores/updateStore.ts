import { relaunch } from '@tauri-apps/plugin-process';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { create } from 'zustand';

export type UpdateStatus =
  | 'idle'
  | 'checking'
  | 'available'
  | 'downloading'
  | 'ready'
  | 'current'
  | 'error';

interface UpdateState {
  status: UpdateStatus;
  /** Version offered by the release feed, once one is known. */
  version: string | null;
  notes: string | null;
  /** 0..1 while downloading, null when the server sends no length. */
  progress: number | null;
  error: string | null;
  /** The handle from the last check; kept so the UI can install on demand. */
  update: Update | null;
  check: (options?: { silent?: boolean }) => Promise<boolean>;
  install: () => Promise<void>;
}

/**
 * Update checks talk to the GitHub release feed and are signed with the
 * project's minisign key, so a tampered feed cannot install anything (§2.7).
 *
 * Every failure here is non-fatal by design: an offline machine, a rate-limited
 * GitHub or a webview without the plugin must never stop the app from starting,
 * so a failed silent check leaves the status on `idle` and says nothing.
 */
export const useUpdateStore = create<UpdateState>((set, get) => ({
  status: 'idle',
  version: null,
  notes: null,
  progress: null,
  error: null,
  update: null,

  check: async ({ silent = false } = {}) => {
    if (get().status === 'downloading') return false;
    set({ status: 'checking', error: null });
    try {
      const found = await check();
      if (!found) {
        set({ status: 'current', update: null, version: null, notes: null });
        return false;
      }
      set({
        status: 'available',
        update: found,
        version: found.version,
        notes: found.body ?? null,
      });
      return true;
    } catch (error) {
      set({
        status: silent ? 'idle' : 'error',
        error: String(error),
        update: null,
      });
      return false;
    }
  },

  install: async () => {
    const update = get().update;
    if (!update) return;
    set({ status: 'downloading', progress: null, error: null });
    try {
      let total = 0;
      let received = 0;
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            total = event.data.contentLength ?? 0;
            received = 0;
            set({ progress: total ? 0 : null });
            break;
          case 'Progress':
            received += event.data.chunkLength;
            if (total) set({ progress: Math.min(received / total, 1) });
            break;
          case 'Finished':
            set({ progress: 1 });
            break;
        }
      });
      set({ status: 'ready' });
      // The installer has already replaced the files on disk; restarting is
      // what puts the user on the new version.
      await relaunch();
    } catch (error) {
      set({ status: 'error', error: String(error), progress: null });
    }
  },
}));
