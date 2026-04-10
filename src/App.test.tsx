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

const storeState = vi.hoisted(() => ({
  dashboard: {
    providers: [] as Array<{
      provider: "zhipu" | "minimax" | "kimi";
      providerLabel: string;
      isEnabled: boolean;
      status: "healthy" | "warning" | "danger" | "needs_setup" | "error" | "stale";
      headlineTitle: string;
      headlineValue: string;
      resetAtLabel: string | null;
      fetchedAt: string;
      isStale: boolean;
      message: string | null;
      sevenDaySummary: string | null;
      thirtyDaySummary: string | null;
      secondaryTitle: string | null;
      secondaryValue: string | null;
      secondaryPercent: number | null;
      secondaryResetAtLabel: string | null;
    }>,
    refreshedAt: "2026-04-03T14:00:00.000Z",
  },
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
    dashboard: storeState.dashboard,
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
          secretPlaceholder: "输入 Kimi API Key，或控制台 Bearer Token",
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
    storeState.dashboard.providers = [];
    storeMocks.loadDashboard.mockClear();
    storeMocks.loadSettings.mockClear();
  });

  it("shows the provider key form inside the popover after clicking settings", () => {
    render(<App />);

    fireEvent.click(screen.getByRole("button", { name: "设置" }));

    expect(
      screen.getByRole("heading", { level: 1, name: "连接套餐与刷新策略" }),
    ).toBeInTheDocument();
    expect(screen.getAllByLabelText("API Key")).toHaveLength(2);
    expect(screen.getByLabelText("API Key / 控制台 Token")).toBeInTheDocument();
  });

  it("shows the next reset time on the provider card", () => {
    storeState.dashboard.providers = [
      {
        provider: "minimax",
        providerLabel: "MiniMax",
        isEnabled: true,
        status: "healthy",
        headlineTitle: "当前用量",
        headlineValue: "37%",
        resetAtLabel: "今天 15:30",
        fetchedAt: "2026-04-03T14:00:00.000Z",
        isStale: false,
        message: null,
        sevenDaySummary: null,
        thirtyDaySummary: null,
        secondaryTitle: null,
        secondaryValue: null,
        secondaryPercent: null,
        secondaryResetAtLabel: null,
      },
    ];

    render(<App />);

    // Check for reset time display - "今天 15:30" is within "重置 今天 15:30"
    expect(screen.getByText(/重置.*今天 15:30/)).toBeInTheDocument();
  });

  it("shows Kimi coding usage with dual progress bars", () => {
    storeState.dashboard.providers = [
      {
        provider: "kimi",
        providerLabel: "Kimi",
        isEnabled: true,
        status: "warning",
        headlineTitle: "5 小时窗口",
        headlineValue: "72%",
        resetAtLabel: "今天 19:00",
        fetchedAt: "2026-04-03T14:00:00.000Z",
        isStale: false,
        message: "本周额度 81% · 04-11 20:00 重置",
        sevenDaySummary: "7 天 最新 72% / 峰值 83% / 均值 51%",
        thirtyDaySummary: null,
        secondaryTitle: "7 天额度",
        secondaryValue: "81%",
        secondaryPercent: 0.81,
        secondaryResetAtLabel: "04-11 20:00",
      },
    ];

    render(<App />);

    // Primary progress bar (5-hour window) - check headline text exists
    expect(screen.getByText(/5 小时窗口/)).toBeInTheDocument();
    // There are two "72%" elements (card-badge and progress-value), get all and verify at least one exists
    expect(screen.getAllByText("72%").length).toBeGreaterThanOrEqual(1);
    // "今天 19:00" is rendered within "重置 今天 19:00" span
    expect(screen.getByText(/重置.*今天 19:00/)).toBeInTheDocument();

    // Secondary progress bar (7-day quota)
    expect(screen.getByText("7 天额度")).toBeInTheDocument();
    // There are two "81%" elements (card-badge and secondary-value), get all and verify at least one exists
    expect(screen.getAllByText("81%").length).toBeGreaterThanOrEqual(1);
  });
});
