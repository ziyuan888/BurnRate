import { invoke } from "@tauri-apps/api/core";

export type ProviderKind = "zhipu" | "minimax" | "kimi";
export type ProviderStatus =
  | "healthy"
  | "warning"
  | "danger"
  | "needs_setup"
  | "error"
  | "stale";

export type ProviderSnapshotView = {
  provider: ProviderKind;
  providerLabel: string;
  isEnabled: boolean;
  status: ProviderStatus;
  headlineTitle: string;
  headlineValue: string;
  resetAtLabel: string | null;
  fetchedAt: string;
  isStale: boolean;
  message: string | null;
  sevenDaySummary: string | null;
  thirtyDaySummary: string | null;
};

export type DashboardState = {
  providers: ProviderSnapshotView[];
  refreshedAt: string;
};

export type ProviderSettingsView = {
  provider: ProviderKind;
  providerLabel: string;
  enabled: boolean;
  endpointUrl: string;
  modelHint: string;
  hasApiKey: boolean;
  maskedApiKey: string | null;
  supportsModelHint: boolean;
  secretPlaceholder: string;
};

export type SettingsState = {
  refreshIntervalSecs: number;
  launchAtLogin: boolean;
  providers: ProviderSettingsView[];
};

export type SaveProviderSettingsInput = {
  provider: ProviderKind;
  enabled: boolean;
  endpointUrl: string;
  modelHint: string;
  apiKey?: string | null;
};

export async function getDashboardState(): Promise<DashboardState> {
  return invoke("get_dashboard_state");
}

export async function refreshNow(): Promise<DashboardState> {
  return invoke("refresh_now");
}

export async function getSettingsState(): Promise<SettingsState> {
  return invoke("get_settings_state");
}

export async function saveProviderSettings(
  input: SaveProviderSettingsInput,
): Promise<SettingsState> {
  return invoke("save_provider_settings", { input });
}

export async function saveRuntimePreferences(
  refreshIntervalSecs: number,
): Promise<SettingsState> {
  return invoke("save_runtime_preferences", {
    input: { refreshIntervalSecs },
  });
}

export async function setLaunchAtLogin(enabled: boolean): Promise<boolean> {
  return invoke("set_launch_at_login", { enabled });
}

export async function quitApp(): Promise<void> {
  return invoke("quit_app");
}

export async function toggleProvider(
  provider: ProviderKind,
): Promise<DashboardState> {
  return invoke("toggle_provider", { provider });
}
