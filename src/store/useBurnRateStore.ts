import { create } from "zustand";

import {
  getDashboardState,
  getSettingsState,
  refreshNow,
  saveProviderSettings,
  saveRuntimePreferences,
  setLaunchAtLogin,
  toggleProvider as toggleProviderFn,
  type DashboardState,
  type ProviderKind,
  type SaveProviderSettingsInput,
  type SettingsState,
} from "../lib/burnrate";

type BurnRateStore = {
  dashboard: DashboardState | null;
  settings: SettingsState | null;
  loading: boolean;
  error: string | null;
  loadDashboard: () => Promise<void>;
  loadSettings: () => Promise<void>;
  refreshDashboard: () => Promise<void>;
  applyDashboard: (dashboard: DashboardState) => void;
  saveProvider: (input: SaveProviderSettingsInput) => Promise<void>;
  saveRuntime: (refreshIntervalSecs: number) => Promise<void>;
  updateLaunchAtLogin: (enabled: boolean) => Promise<void>;
  toggleProvider: (provider: ProviderKind) => Promise<void>;
};

export const useBurnRateStore = create<BurnRateStore>((set, get) => ({
  dashboard: null,
  settings: null,
  loading: false,
  error: null,
  loadDashboard: async () => {
    set({ loading: true, error: null });
    try {
      const dashboard = await getDashboardState();
      set({ dashboard, loading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        loading: false,
      });
    }
  },
  loadSettings: async () => {
    set({ loading: true, error: null });
    try {
      const settings = await getSettingsState();
      set({ settings, loading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        loading: false,
      });
    }
  },
  refreshDashboard: async () => {
    set({ loading: true, error: null });
    try {
      const dashboard = await refreshNow();
      set({ dashboard, loading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        loading: false,
      });
    }
  },
  applyDashboard: (dashboard) => set({ dashboard }),
  saveProvider: async (input) => {
    set({ loading: true, error: null });
    try {
      const settings = await saveProviderSettings(input);
      set({ settings, loading: false });
      await get().refreshDashboard();
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        loading: false,
      });
    }
  },
  saveRuntime: async (refreshIntervalSecs) => {
    set({ loading: true, error: null });
    try {
      const settings = await saveRuntimePreferences(refreshIntervalSecs);
      set({ settings, loading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        loading: false,
      });
    }
  },
  updateLaunchAtLogin: async (enabled) => {
    set({ loading: true, error: null });
    try {
      const launchAtLogin = await setLaunchAtLogin(enabled);
      set({
        settings: get().settings
          ? { ...get().settings!, launchAtLogin }
          : get().settings,
        loading: false,
      });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        loading: false,
      });
    }
  },
  toggleProvider: async (provider) => {
    set({ loading: true, error: null });
    try {
      const dashboard = await toggleProviderFn(provider);
      set({ dashboard, loading: false });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        loading: false,
      });
    }
  },
}));
