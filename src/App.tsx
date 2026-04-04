import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import clsx from "clsx";

import {
  type DashboardState,
  type ProviderKind,
  type ProviderSettingsView,
  type ProviderSnapshotView,
  quitApp,
} from "./lib/burnrate";
import { buildStatusSummary } from "./features/dashboard/summary";
import { useBurnRateStore } from "./store/useBurnRateStore";
import "./App.css";

type PopoverView = "dashboard" | "settings";

function App() {
  const currentLabel = getCurrentWindow().label;
  const isSettingsWindow = currentLabel === "settings";
  const [popoverView, setPopoverView] = useState<PopoverView>("dashboard");

  if (isSettingsWindow) {
    return (
      <div className="shell shell-settings">
        <SettingsSurface mode="window" onClose={() => void getCurrentWindow().hide()} />
      </div>
    );
  }

  return (
    <div className="shell shell-popover">
      {popoverView === "settings" ? (
        <SettingsSurface mode="inline" onClose={() => setPopoverView("dashboard")} />
      ) : (
        <PopoverSurface onOpenSettings={() => setPopoverView("settings")} />
      )}
    </div>
  );
}

function PopoverSurface({ onOpenSettings }: { onOpenSettings: () => void }) {
  const {
    dashboard,
    error,
    loading,
    loadDashboard,
    refreshDashboard,
    applyDashboard,
  } = useBurnRateStore();

  useEffect(() => {
    void loadDashboard();

    let disposed = false;
    void listen<DashboardState>("dashboard://updated", (event) => {
      if (!disposed) {
        applyDashboard(event.payload);
      }
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      }
    });

    return () => {
      disposed = true;
    };
  }, [applyDashboard, loadDashboard]);

  const summary = buildStatusSummary(
    (dashboard?.providers ?? []).map((provider) => ({
      provider: provider.provider,
      status: provider.status,
      headlineValue: provider.headlineValue,
      fetchedAt: provider.fetchedAt,
      isStale: provider.isStale,
      message: provider.message,
    })),
  );

  return (
    <main className="popover-layout">
      <section className="glass-card hero-card">
        <div className="hero-row">
          <div>
            <p className="eyebrow">BurnRate</p>
            <h1>套餐燃烧速率</h1>
            <p className="subtle">
              右上角常驻监控 Zhipu、MiniMax 和 Kimi 的当前状态。
            </p>
          </div>
          <div className={clsx("status-pill", `status-pill-${summary.tone}`)}>
            {summary.compactText}
          </div>
        </div>

        <div className="hero-actions">
          <button className="primary-button" onClick={() => void refreshDashboard()}>
            {loading ? "刷新中..." : "立即刷新"}
          </button>
          <button className="ghost-button" onClick={onOpenSettings}>
            设置
          </button>
          <button className="ghost-button" onClick={() => void quitApp()}>
            退出
          </button>
        </div>
        {error ? <p className="inline-error">{error}</p> : null}
      </section>

      <section className="provider-stack">
        {(dashboard?.providers ?? []).map((provider) => (
          <ProviderCard key={provider.provider} provider={provider} />
        ))}
      </section>
    </main>
  );
}

function ProviderCard({ provider }: { provider: ProviderSnapshotView }) {
  const { toggleProvider } = useBurnRateStore();

  return (
    <article className="glass-card provider-card">
      <div className="provider-header">
        <div>
          <p className="provider-title">{provider.providerLabel}</p>
          <p className="provider-subtitle">{provider.headlineTitle}</p>
        </div>
        <button
          role="switch"
          aria-checked={provider.isEnabled}
          className={clsx("switch", provider.isEnabled && "switch-on")}
          onClick={() => void toggleProvider(provider.provider)}
        >
          <span className="switch-knob" />
        </button>
      </div>

      <div className="metric-row">
        <span className="metric-label">当前值</span>
        <span className="metric-value">{provider.headlineValue}</span>
      </div>

      {provider.resetAtLabel ? (
        <div className="meta-row">
          <span>重置</span>
          <span>{provider.resetAtLabel}</span>
        </div>
      ) : null}
      {provider.sevenDaySummary ? <p className="summary-line">{provider.sevenDaySummary}</p> : null}
      {provider.thirtyDaySummary ? (
        <p className="summary-line">{provider.thirtyDaySummary}</p>
      ) : null}
      {provider.message ? <p className="provider-message">{provider.message}</p> : null}
      <p className="provider-timestamp">
        更新于 {new Date(provider.fetchedAt).toLocaleString("zh-CN")}
        {provider.isStale ? " · 数据较旧" : ""}
      </p>
    </article>
  );
}

function SettingsSurface({
  mode,
  onClose,
}: {
  mode: "window" | "inline";
  onClose: () => void;
}) {
  const { settings, error, loading, loadSettings, saveProvider, saveRuntime, updateLaunchAtLogin } =
    useBurnRateStore();
  const [refreshInterval, setRefreshInterval] = useState("60");

  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  useEffect(() => {
    if (settings) {
      setRefreshInterval(String(settings.refreshIntervalSecs));
    }
  }, [settings]);

  return (
    <main className={clsx("settings-layout", mode === "inline" && "settings-layout-inline")}>
      <section className="settings-header">
        <div>
          <p className="eyebrow">BurnRate 设置</p>
          <h1>连接套餐与刷新策略</h1>
          <p className="subtle">
            API Key 仅用于本机查询，不会出现在菜单栏前端请求里。
            {mode === "inline" ? " 你可以直接在当前弹层完成套餐接入。" : ""}
          </p>
        </div>
        <button className="ghost-button" onClick={onClose}>
          {mode === "inline" ? "返回概览" : "关闭"}
        </button>
      </section>

      <section className="settings-grid">
        <article className="settings-panel">
          <h2>运行策略</h2>
          <label className="field">
            <span>自动刷新间隔（秒）</span>
            <input
              value={refreshInterval}
              type="number"
              min={15}
              step={15}
              onChange={(event) => setRefreshInterval(event.currentTarget.value)}
            />
          </label>
          <div className="toggle-row">
            <span>开机自动启动</span>
            <button
              className={clsx(
                "toggle-button",
                settings?.launchAtLogin ? "toggle-button-on" : "toggle-button-off",
              )}
              onClick={() => void updateLaunchAtLogin(!(settings?.launchAtLogin ?? false))}
            >
              {settings?.launchAtLogin ? "已开启" : "未开启"}
            </button>
          </div>
          <button
            className="primary-button"
            onClick={() => void saveRuntime(Number.parseInt(refreshInterval || "60", 10))}
          >
            保存运行设置
          </button>
        </article>

        <article className="settings-panel settings-panel-wide">
          <h2>套餐配置</h2>
          <div className="provider-settings-list">
            {(settings?.providers ?? []).map((provider) => (
              <ProviderSettingsCard
                key={provider.provider}
                provider={provider}
                onSave={(payload) => void saveProvider(payload)}
              />
            ))}
          </div>
        </article>
      </section>

      {mode === "inline" ? (
        <section className="settings-panel inline-settings-note">
          <h2>接入说明</h2>
          <p className="inline-note">
            先填入真实 API Key，再点击各套餐的“保存”。
            保存后回到概览页点“立即刷新”，就会开始真实联调。
          </p>
        </section>
      ) : null}
      {error ? <p className="inline-error">{error}</p> : null}
      {loading ? <p className="inline-note">正在同步本机状态…</p> : null}
    </main>
  );
}

function ProviderSettingsCard({
  provider,
  onSave,
}: {
  provider: ProviderSettingsView;
  onSave: (payload: {
    provider: ProviderKind;
    enabled: boolean;
    endpointUrl: string;
    modelHint: string;
    apiKey: string;
  }) => void;
}) {
  const [apiKey, setApiKey] = useState("");

  useEffect(() => {
    setApiKey("");
  }, [provider]);

  return (
    <div className="provider-settings-card">
      <div className="provider-settings-header">
        <div>
          <h3>{provider.providerLabel}</h3>
          <div className={clsx("key-status", provider.hasApiKey ? "key-status-configured" : "key-status-empty")}>
            <span className="key-status-dot" />
            {provider.hasApiKey ? "已保存密钥" : "未配置密钥"}
          </div>
        </div>
      </div>

      <div className="provider-settings-fields">
        <label className="field field-wide">
          <span>API Key</span>
          <input
            value={apiKey}
            type="password"
            onChange={(event) => setApiKey(event.currentTarget.value)}
            placeholder={provider.hasApiKey ? "已保存密钥，留空则保持不变" : provider.secretPlaceholder}
          />
        </label>
      </div>

      <button
        className="primary-button"
        onClick={() => {
          onSave({
            provider: provider.provider,
            enabled: true,
            endpointUrl: "",
            modelHint: "",
            apiKey,
          });
          setApiKey("");
        }}
      >
        保存 {provider.providerLabel}
      </button>
    </div>
  );
}

export default App;
