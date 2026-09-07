import { toast } from 'sonner';
import { create } from 'zustand';

import { macros as api } from '@/services/tauri';
import type { Macro } from '@/types';

interface MacroState {
  macros: Macro[];
  loading: boolean;
  query: string;
  load: () => Promise<void>;
  setQuery: (query: string) => void;
  save: (value: Macro) => Promise<Macro>;
  remove: (macroId: string) => Promise<void>;
  duplicate: (macroId: string) => Promise<Macro>;
  toggleEnabled: (macroId: string) => Promise<void>;
}

export const useMacroStore = create<MacroState>((set, get) => ({
  macros: [],
  loading: true,
  query: '',

  load: async () => {
    try {
      set({ macros: await api.list(), loading: false });
    } catch (error) {
      // A read failure must not leave the page spinning forever.
      set({ loading: false });
      toast.error(String(error));
    }
  },

  setQuery: (query) => set({ query }),

  save: async (value) => {
    const saved = await api.save(value);
    set((state) => {
      const exists = state.macros.some((item) => item.id === saved.id);
      return {
        macros: exists
          ? state.macros.map((item) => (item.id === saved.id ? saved : item))
          : [...state.macros, saved],
      };
    });
    return saved;
  },

  remove: async (macroId) => {
    await api.remove(macroId);
    set((state) => ({ macros: state.macros.filter((item) => item.id !== macroId) }));
  },

  duplicate: async (macroId) => {
    const copy = await api.duplicate(macroId);
    set((state) => ({ macros: [...state.macros, copy] }));
    return copy;
  },

  toggleEnabled: async (macroId) => {
    const target = get().macros.find((item) => item.id === macroId);
    if (!target) return;
    await get().save({ ...target, enabled: !target.enabled });
  },
}));

/** Case-insensitive name match; the list is small enough to filter in render. */
export function filterMacros(list: Macro[], query: string): Macro[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return list;
  return list.filter((item) => item.name.toLowerCase().includes(needle));
}
