import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface UiState {
  sidebarCollapsed: boolean;
  toggleSidebar: () => void;
  /** Macro currently open in the editor, or being targeted by the dashboard. */
  selectedMacroId: string | null;
  selectMacro: (macroId: string | null) => void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set, get) => ({
      sidebarCollapsed: false,
      toggleSidebar: () => set({ sidebarCollapsed: !get().sidebarCollapsed }),
      selectedMacroId: null,
      selectMacro: (macroId) => set({ selectedMacroId: macroId }),
    }),
    {
      name: 'flowmacro-ui',
      // Only view preferences are persisted; selection is per-session.
      partialize: (state) => ({ sidebarCollapsed: state.sidebarCollapsed }),
    },
  ),
);
