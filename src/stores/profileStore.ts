import { toast } from 'sonner';
import { create } from 'zustand';

import { profiles as api } from '@/services/tauri';
import type { Profile, ProfileColor, ProfileStore as Store } from '@/types';

interface ProfileState {
  store: Store | null;
  loading: boolean;
  load: () => Promise<void>;
  create: (name: string, color: ProfileColor) => Promise<Profile>;
  save: (value: Profile) => Promise<void>;
  remove: (profileId: string) => Promise<void>;
  switchTo: (profileId: string) => Promise<void>;
}

export const useProfileStore = create<ProfileState>((set, get) => ({
  store: null,
  loading: true,

  load: async () => {
    try {
      set({ store: await api.list(), loading: false });
    } catch (error) {
      set({ loading: false });
      toast.error(String(error));
    }
  },

  create: async (name, color) => {
    const profile = await api.create(name, color);
    await api.save(profile);
    await get().load();
    return profile;
  },

  save: async (value) => {
    await api.save(value);
    await get().load();
  },

  remove: async (profileId) => {
    await api.remove(profileId);
    await get().load();
  },

  switchTo: async (profileId) => {
    await api.switchTo(profileId);
    await get().load();
  },
}));

export function activeProfile(store: Store | null): Profile | null {
  if (!store) return null;
  return store.profiles.find((item) => item.id === store.activeProfileId) ?? null;
}
