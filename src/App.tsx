import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState, useMemo } from "react";
import clsx from "clsx";

import {
  type DashboardState,
  type ProviderKind,
  type ProviderSettingsView,
  quitApp,
} from "./lib/burnrate";
import { buildStatusSummary } from "./features/dashboard/summary";
import { useBurnRateStore } from "./store/useBurnRateStore";
import "./App.css";

type PopoverView = "dashboard" | "settings";

// Icons as simple SVG components
const RefreshIcon = ({ style }: { style?: React.CSSProperties }) => (
  <svg style={{ width: '16px', height: '16px', ...style }} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" />
    <path d="M3 3v5h5" />
    <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16" />
    <path d="M16 21h5v-5" />
  </svg>
);

const SettingsIcon = ({ style }: { style?: React.CSSProperties }) => (
  <svg style={{ width: '16px', height: '16px', ...style }} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
    <circle cx="12" cy="12" r="3" />
  </svg>
);

const CloseIcon = ({ style }: { style?: React.CSSProperties }) => (
  <svg style={{ width: '16px', height: '16px', ...style }} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M18 6 6 18" />
    <path d="m6 6 12 12" />
  </svg>
);

const ClockIcon = ({ style }: { style?: React.CSSProperties }) => (
  <svg style={{ width: '14px', height: '14px', ...style }} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <circle cx="12" cy="12" r="10" />
    <polyline points="12 6 12 12 16 14" />
  </svg>
);

const MessageIcon = ({ style }: { style?: React.CSSProperties }) => (
  <svg style={{ width: '14px', height: '14px', ...style }} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
  </svg>
);

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

  const [currentTime, setCurrentTime] = useState(() => formatTime(new Date()));

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

    // Update time every minute
    const timeInterval = setInterval(() => {
      setCurrentTime(formatTime(new Date()));
    }, 60000);

    return () => {
      disposed = true;
      clearInterval(timeInterval);
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

  // Parse provider data for display
  const providerData = useMemo(() => {
    return (dashboard?.providers ?? []).map((p) => ({
      ...p,
      // Parse progress from headlineValue (e.g. "11%" or "37/100")
      progress: parseProgress(p.headlineValue),
      // Estimate remaining time from resetAtLabel
      remainingTime: p.resetAtLabel || "--",
      // Parse secondary progress for dual-quota providers (check null/undefined, not falsy)
      secondaryProgress: p.secondaryPercent != null ? { percent: Math.round(p.secondaryPercent * 100) } : null,
      secondaryRemainingTime: p.secondaryResetAtLabel || null,
      // Parse MCP progress for Zhipu
      mcpProgress: p.mcpPercent != null ? { 
        percent: Math.round(p.mcpPercent * 100),
        value: p.mcpValue,
        limit: p.mcpLimit,
      } : null,
      mcpRemainingTime: p.mcpResetAtLabel || null,
    }));
  }, [dashboard]);

  // Get usage stats from dashboard data
  const usageStats = dashboard?.usageStats;
  const totalTokens = usageStats?.totalTokens ?? "--";
  const totalMessages = usageStats?.totalMessages ?? 0;
  const totalToolCalls = usageStats?.totalToolCalls ?? 0;
  const toolCalls = usageStats?.toolCalls ?? [];

  const hideWindow = () => {
    void getCurrentWindow().hide();
  };

  return (
    <main className="popover-layout">
      {/* Header */}
      <header className="popover-header">
        <span className="popover-header-title">BurnRate 用量查询</span>
        <div className="popover-header-actions">
          <span className="popover-header-time">{currentTime}</span>
          <button 
            className={clsx("icon-button", loading && "spinning")}
            onClick={() => void refreshDashboard()}
            title="刷新"
            disabled={loading}
            type="button"
          >
            <RefreshIcon />
          </button>
          <button 
            className="icon-button" 
            onClick={onOpenSettings}
            title="设置"
            type="button"
          >
            <SettingsIcon />
          </button>
          <button 
            className="icon-button" 
            onClick={hideWindow}
            title="关闭"
            type="button"
          >
            <CloseIcon />
          </button>
        </div>
      </header>

      {/* Provider Cards - each provider has its own card */}
      {providerData.map((provider, index) => (
        <div 
          key={provider.provider} 
          className="card" 
          style={{ '--delay': `${index * 0.1}s` } as React.CSSProperties}
        >
          <div className="card-header">
            <div className="card-header-left">
              <span className="card-title">
                {provider.provider === 'zhipu' ? '智谱 coding plan 用量' : provider.providerLabel}
              </span>
              {provider.provider === 'zhipu' && (
                <span className="card-subtitle">Default Account</span>
              )}
            </div>
            <div className="card-header-right">
              {/* Circular progress indicator for Zhipu */}
              {provider.provider === 'zhipu' && (
                <div className={clsx(
                  "circular-progress",
                  provider.progress.percent > 80 && "warning",
                  provider.progress.percent > 95 && "danger"
                )}>
                  <svg viewBox="0 0 36 36" className="circular-chart">
                    <path
                      className="circle-bg"
                      d="M18 2.0845
                        a 15.9155 15.9155 0 0 1 0 31.831
                        a 15.9155 15.9155 0 0 1 0 -31.831"
                    />
                    <path
                      className="circle"
                      strokeDasharray={`${provider.progress.percent}, 100`}
                      d="M18 2.0845
                        a 15.9155 15.9155 0 0 1 0 31.831
                        a 15.9155 15.9155 0 0 1 0 -31.831"
                    />
                  </svg>
                  <span className="circular-value">{provider.progress.percent}</span>
                </div>
              )}
              <span className={clsx(
                "card-badge",
                provider.status === 'healthy' && "badge-healthy",
                provider.status === 'warning' && "badge-warning",
                provider.status === 'danger' && "badge-danger"
              )}>
                {provider.headlineValue}
              </span>
            </div>
          </div>
          <div className="card-content">
            {/* MCP progress bar for Zhipu */}
            {provider.mcpProgress && provider.mcpTitle && (
              <div className="progress-item">
                <div className="progress-header">
                  <span className="progress-label">{provider.mcpTitle}</span>
                  <span className={clsx("progress-value", provider.mcpProgress.percent > 80 && "warning", provider.mcpProgress.percent > 95 && "danger")}>
                    {provider.mcpProgress.value || `${provider.mcpProgress.percent}%`}
                  </span>
                </div>
                <div className="progress-bar-bg">
                  <div 
                    className={clsx(
                      "progress-bar-fill",
                      provider.mcpProgress.percent > 80 && "warning",
                      provider.mcpProgress.percent > 95 && "danger"
                    )}
                    style={{ width: `${Math.min(provider.mcpProgress.percent, 100)}%` }}
                  />
                </div>
                <div className="progress-meta">
                  <ClockIcon />
                  <span>{provider.mcpRemainingTime ? `${provider.mcpRemainingTime} 后重置` : '--'}</span>
                </div>
              </div>
            )}

            {/* Secondary progress bar (Token 5-hour for Zhipu) */}
            {provider.secondaryProgress && provider.secondaryTitle && (
              <div className="progress-item">
                <div className="progress-header">
                  <span className="progress-label">{provider.secondaryTitle}</span>
                  <span className={clsx("progress-value", provider.secondaryProgress.percent > 80 && "warning", provider.secondaryProgress.percent > 95 && "danger")}>
                    {provider.secondaryValue}
                  </span>
                </div>
                <div className="progress-bar-bg">
                  <div 
                    className={clsx(
                      "progress-bar-fill",
                      provider.secondaryProgress.percent > 80 && "warning",
                      provider.secondaryProgress.percent > 95 && "danger"
                    )}
                    style={{ width: `${Math.min(provider.secondaryProgress.percent, 100)}%` }}
                  />
                </div>
                <div className="progress-meta">
                  <ClockIcon />
                  <span>{provider.secondaryRemainingTime ? `${provider.secondaryRemainingTime} 后重置` : '--'}</span>
                </div>
              </div>
            )}

            {/* Primary progress bar for non-Zhipu providers */}
            {provider.provider !== 'zhipu' && (
              <>
                <div className="progress-header">
                  <span className="progress-label">{provider.headlineTitle || '当前用量'}</span>
                  <span className={clsx("progress-value", provider.progress.percent > 80 && "warning", provider.progress.percent > 95 && "danger")}>
                    {provider.headlineValue}
                  </span>
                </div>
                <div className="progress-bar-bg">
                  <div 
                    className={clsx(
                      "progress-bar-fill",
                      provider.progress.percent > 80 && "warning",
                      provider.progress.percent > 95 && "danger"
                    )}
                    style={{ width: `${Math.min(provider.progress.percent, 100)}%` }}
                  />
                </div>
                <div className="progress-meta">
                  <ClockIcon />
                  <span>重置 {provider.remainingTime}</span>
                </div>
              </>
            )}
          </div>
        </div>
      ))}

      {/* Model Usage */}
      <div className="metric-grid">
        <div className="metric-card" style={{ '--delay': '0.1s' } as React.CSSProperties}>
          <div className="metric-card-header">模型用量</div>
          <div className="metric-card-value">{totalTokens}</div>
          <div className="metric-card-sub">
            <MessageIcon />
            <span>{totalMessages.toLocaleString()}</span>
          </div>
        </div>
        <div className="metric-card" style={{ '--delay': '0.15s' } as React.CSSProperties}>
          <div className="metric-card-header">工具调用</div>
          <div className="metric-card-value">{totalToolCalls}</div>
          <div className="metric-card-sub">
            <span>次调用</span>
          </div>
        </div>
      </div>

      {/* Tool Calls */}
      <div className="card" style={{ '--delay': '0.2s' } as React.CSSProperties}>
        <div className="card-header">
          <span className="card-title">工具调用详情</span>
        </div>
        <div className="card-content">
          {toolCalls.length > 0 ? (
            <div className="tool-list">
              {toolCalls.map((tool) => (
                <div key={tool.name} className="tool-item">
                  <span className="tool-name">{tool.name}</span>
                  <span className="tool-count">{tool.count}</span>
                </div>
              ))}
            </div>
          ) : (
            <p style={{ color: 'rgba(255,255,255,0.4)', fontSize: '13px', margin: 0 }}>
              工具调用数据需通过 MCP/Coding API 获取
            </p>
          )}
        </div>
      </div>

      {/* Status Summary */}
      <div className="provider-stack" style={{ marginTop: "8px" }}>
        <div className="provider-card" style={{ '--delay': '0.3s' } as React.CSSProperties}>
          <div className="provider-header">
            <span className="provider-title">总体状态</span>
            <span className={clsx("status-pill", `status-pill-${summary.tone}`)}>
              {summary.compactText}
            </span>
          </div>
          <div className="provider-meta">
            <span>点击右上角按钮刷新数据或打开设置</span>
          </div>
        </div>
      </div>

      {/* Actions */}
      <div className="actions-bar">
        <button className="primary-button flex-1" onClick={() => void refreshDashboard()}>
          {loading ? "刷新中..." : "立即刷新"}
        </button>
        <button className="ghost-button" onClick={() => void quitApp()}>
          退出
        </button>
      </div>

      {error ? <p className="inline-error" style={{ margin: '0 16px 12px' }}>{error}</p> : null}
    </main>
  );
}

// Helper function to format time like "23:48"
function formatTime(date: Date): string {
  return date.toLocaleTimeString("zh-CN", { 
    hour: "2-digit", 
    minute: "2-digit",
    hour12: false 
  });
}

// Helper to parse progress from headline value
function parseProgress(value: string): { percent: number; current: number; total: number } {
  // Try to parse "11%" format
  const percentMatch = value.match(/(\d+)%/);
  if (percentMatch) {
    const percent = parseInt(percentMatch[1], 10);
    return { percent, current: percent, total: 100 };
  }
  
  // Try to parse "37/100" format
  const fractionMatch = value.match(/(\d+)\s*\/\s*(\d+)/);
  if (fractionMatch) {
    const current = parseInt(fractionMatch[1], 10);
    const total = parseInt(fractionMatch[2], 10);
    return { percent: Math.round((current / total) * 100), current, total };
  }
  
  // Default
  return { percent: 0, current: 0, total: 100 };
}

function SettingsSurface({
  mode,
  onClose,
}: {
  mode: "window" | "inline";
  onClose: () => void;
}) {
  const { settings, error, loading, loadSettings, saveProvider, saveRuntime, updateLaunchAtLogin, toggleProvider } =
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
          <div className="toggle-row" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: '14px' }}>
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
            style={{ marginTop: '18px' }}
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
                onToggle={() => void toggleProvider(provider.provider)}
              />
            ))}
          </div>
        </article>
      </section>

      {mode === "inline" ? (
        <section className="settings-panel inline-settings-note">
          <h2>接入说明</h2>
          <p className="inline-note">
            先填入真实 API Key，再点击各套餐的"保存"。
            Kimi 如果要显示 Coding 用量，请粘贴控制台里的 Bearer Token。
            保存后回到概览页点"立即刷新"，就会开始真实联调。
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
  onToggle,
}: {
  provider: ProviderSettingsView;
  onSave: (payload: {
    provider: ProviderKind;
    enabled: boolean;
    endpointUrl: string;
    modelHint: string;
    apiKey: string;
  }) => void;
  onToggle: () => void;
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
        {/* iOS-style Toggle Switch */}
        <button
          className={clsx("switch", provider.enabled && "switch-on")}
          onClick={onToggle}
          title={provider.enabled ? "点击禁用" : "点击启用"}
          type="button"
        >
          <span className="switch-knob" />
        </button>
      </div>

      <div className="provider-settings-fields">
        <label className="field field-wide">
          <span>{provider.provider === "kimi" ? "API Key / 控制台 Token" : "API Key"}</span>
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
