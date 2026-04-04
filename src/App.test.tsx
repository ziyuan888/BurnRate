import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const tauriWindowMocks = vi.hoisted(() => ({
  currentWindow: {
    label: "popover",
    hide: vi.fn(),
    setFocus: vi.fn(),
  },
  settingsWindow: {
    show: vi.fn(),
    setFocus: vi.fn(),
  },
  getByLabel: vi.fn(),
}));

const storeMocks = vi.hoisted(() => ({
  loadDashboard: vi.fn().mockResolvedValue(undefined),
  refreshDashboard: vi.fn().mockResolvedValue(undefined),
  applyDashboard: vi.fn(),
  loadSettings: vi.fn().mockResolvedValue(undefined),
  saveProvider: vi.fn().mockResolvedValue(undefined),
  saveRuntime: vi.fn().mockResolvedValue(undefined),
  updateLaunchAtLogin: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => undefined),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => tauriWindowMocks.currentWindow,
  Window: {
    getByLabel: tauriWindowMocks.getByLabel,
  },
}));

vi.mock("./store/useBurnRateStore", () => ({
  useBurnRateStore: () => ({
    dashboard: {
      providers: [],
      refreshedAt: "2026-04-03T14:00:00.000Z",
    },
    settings: {
      refreshIntervalSecs: 60,
      launchAtLogin: false,
      providers: [
        {
          provider: "zhipu",
          providerLabel: "智谱清言",
          enabled: true,
          endpointUrl: "",
          modelHint: "",
          hasApiKey: false,
          supportsModelHint: false,
          secretPlaceholder: "输入套餐 API Key",
        },
        {
          provider: "minimax",
          providerLabel: "MiniMax",
          enabled: true,
          endpointUrl: "",
          modelHint: "MiniMax-M2.5",
          hasApiKey: false,
          supportsModelHint: true,
          secretPlaceholder: "输入套餐 API Key",
        },
        {
          provider: "kimi",
          providerLabel: "Kimi",
          enabled: true,
          endpointUrl: "",
          modelHint: "",
          hasApiKey: false,
          supportsModelHint: false,
          secretPlaceholder: "输入 Kimi API Key",
        },
      ],
    },
    error: null,
    loading: false,
    ...storeMocks,
  }),
}));

vi.mock("./lib/burnrate", async () => {
  const actual = await vi.importActual<typeof import("./lib/burnrate")>("./lib/burnrate");
  return {
    ...actual,
    quitApp: vi.fn().mockResolvedValue(undefined),
  };
});

import App from "./App";

describe("App settings access", () => {
  beforeEach(() => {
    tauriWindowMocks.currentWindow.label = "popover";
    tauriWindowMocks.getByLabel.mockResolvedValue(tauriWindowMocks.settingsWindow);
    tauriWindowMocks.settingsWindow.show.mockReset();
    tauriWindowMocks.settingsWindow.setFocus.mockReset();
    storeMocks.loadDashboard.mockClear();
    storeMocks.loadSettings.mockClear();
  });

  it("shows the provider key form inside the popover after clicking settings", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "设置" }));

    expect(
      screen.getByRole("heading", { level: 1, name: "连接套餐与刷新策略" }),
    ).toBeInTheDocument();
    expect(screen.getAllByLabelText("API Key")).toHaveLength(3);
  });
});
