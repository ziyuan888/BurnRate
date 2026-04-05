use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use reqwest::Client;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use time::{Duration as TimeDuration, format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

use crate::models::{
    DashboardState, NormalizedSnapshot, ProviderKind, ProviderSettingsRecord, ProviderSettingsView,
    ProviderSnapshotView, SettingsState, SnapshotStatus, StoredSnapshot, UsageStats,
};
use crate::providers::{kimi, minimax, zhipu};
use crate::storage::db;
use crate::storage::rollup::compute_rollup;

#[derive(Clone)]
pub struct AppState {
    client: Client,
    db_path: PathBuf,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Result<Self> {
        let data_dir = app
            .path()
            .app_local_data_dir()
            .context("failed to resolve app local data directory")?;
        let db_path = data_dir.join("burnrate.db");
        db::init_database(&db_path)?;

        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(8))
                .build()
                .context("failed to build HTTP client")?,
            db_path,
        })
    }

    pub fn spawn_background_refresh(&self, app: AppHandle) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Ok(dashboard) = state.refresh_all().await {
                let _ = app.emit("dashboard://updated", dashboard);
            }
            crate::tray::update_tray_tooltip(&app, &state);

            loop {
                let interval_secs = state.refresh_interval_secs().unwrap_or(60).max(15);
                tokio::time::sleep(Duration::from_secs(interval_secs)).await;
                if let Ok(dashboard) = state.refresh_all().await {
                    let _ = app.emit("dashboard://updated", dashboard);
                }
                crate::tray::update_tray_tooltip(&app, &state);
            }
        });
    }

    pub async fn refresh_all(&self) -> Result<DashboardState> {
        let configs = db::load_provider_settings(&self.db_path)?;

        let refresh_jobs = configs.into_iter().map(|config| {
            let state = self.clone();
            async move {
                let normalized = state.fetch_provider_normalized(config.clone()).await;
                (config.provider, normalized)
            }
        });

        let mut normalized_snapshots = std::collections::HashMap::new();
        for (provider, normalized_result) in futures::future::join_all(refresh_jobs).await {
            if let Ok(snapshot) = normalized_result {
                db::insert_snapshot(&self.db_path, &to_stored_snapshot(snapshot.clone()))?;
                normalized_snapshots.insert(provider, snapshot);
            }
        }

        self.build_dashboard_state_with_normalized(normalized_snapshots)
    }

    pub fn build_dashboard_state(&self) -> Result<DashboardState> {
        self.build_dashboard_state_with_normalized(std::collections::HashMap::new())
    }

    fn build_dashboard_state_with_normalized(
        &self,
        normalized_snapshots: std::collections::HashMap<ProviderKind, NormalizedSnapshot>,
    ) -> Result<DashboardState> {
        let mut providers = Vec::new();
        let now_ms = now_unix_ms();

        for record in db::load_provider_settings(&self.db_path)? {
            let normalized = normalized_snapshots.get(&record.provider);
            let view = self.build_provider_view(record, now_ms, normalized)?;
            providers.push(view);
        }

        // Build usage stats from provider data
        let usage_stats = self.build_usage_stats(&providers)?;

        Ok(DashboardState {
            providers,
            refreshed_at: format_iso(now_ms)?,
            usage_stats,
        })
    }

    fn build_usage_stats(&self, providers: &[ProviderSnapshotView]) -> Result<UsageStats> {
        // Aggregate token usage from all providers
        let mut total_tokens: f64 = 0.0;
        let mut total_messages: i64 = 0;
        let mut total_tool_calls: i64 = 0;
        let mut tool_calls: Vec<crate::models::ToolCall> = Vec::new();

        // For each provider, try to get detailed usage from snapshots
        for provider in providers {
            // Get additional metrics from database if available
            if let Ok(metrics) = db::load_latest_metrics(&self.db_path, provider.provider) {
                total_tokens += metrics.tokens as f64;
                total_messages += metrics.messages;
                total_tool_calls += metrics.tool_calls;
                
                // Add tool-specific calls
                for (name, count) in metrics.tool_breakdown {
                    if let Some(existing) = tool_calls.iter_mut().find(|t: &&mut crate::models::ToolCall| t.name == name) {
                        existing.count += count;
                    } else {
                        tool_calls.push(crate::models::ToolCall { name, count });
                    }
                }
            }
        }

        // Sort tool calls by count descending
        tool_calls.sort_by(|a, b| b.count.cmp(&a.count));

        // Format total tokens (convert to M if > 1M)
        let tokens_str = if total_tokens >= 1_000_000.0 {
            format!("{:.1}M", total_tokens / 1_000_000.0)
        } else if total_tokens >= 1000.0 {
            format!("{:.1}K", total_tokens / 1000.0)
        } else {
            format!("{}", total_tokens as i64)
        };

        Ok(UsageStats {
            total_tokens: tokens_str,
            total_messages,
            total_tool_calls,
            tool_calls,
        })
    }

    pub fn load_settings_state(&self, app: &AppHandle) -> Result<SettingsState> {
        let launch_at_login = app
            .autolaunch()
            .is_enabled()
            .unwrap_or(false);

        let providers = db::load_provider_settings(&self.db_path)?
            .into_iter()
            .map(|record| {
                let masked_api_key = self.load_api_key(record.provider).ok().map(|key| mask_secret(&key));
                ProviderSettingsView {
                    provider: record.provider,
                    provider_label: record.provider.display_name().to_string(),
                    enabled: record.enabled,
                    endpoint_url: record.endpoint_url.unwrap_or_default(),
                    model_hint: record.model_hint.unwrap_or_default(),
                    has_api_key: masked_api_key.is_some(),
                    masked_api_key,
                    supports_model_hint: matches!(record.provider, ProviderKind::Minimax),
                    secret_placeholder: if matches!(record.provider, ProviderKind::Kimi) {
                        "输入 Kimi API Key，或控制台 Bearer Token".to_string()
                    } else {
                        "输入套餐 API Key".to_string()
                    },
                }
            })
            .collect::<Vec<_>>();

        Ok(SettingsState {
            refresh_interval_secs: self.refresh_interval_secs()?,
            launch_at_login,
            providers,
        })
    }

    pub fn save_provider_settings(
        &self,
        input: crate::models::SaveProviderSettingsInput,
    ) -> Result<()> {
        let endpoint_url = normalize_optional_text(&input.endpoint_url);
        let model_hint = normalize_optional_text(&input.model_hint);
        db::save_provider_settings(
            &self.db_path,
            &ProviderSettingsRecord {
                provider: input.provider,
                enabled: input.enabled,
                endpoint_url,
                model_hint,
            },
        )?;

        if let Some(api_key) = input.api_key {
            let trimmed = api_key.trim();
            if trimmed.is_empty() {
                self.delete_api_key(input.provider)?;
            } else {
                self.save_api_key(input.provider, trimmed)?;
            }
        }

        Ok(())
    }

    pub fn save_runtime_preferences(&self, refresh_interval_secs: u64) -> Result<()> {
        db::save_refresh_interval_secs(&self.db_path, refresh_interval_secs.max(15))
    }

    fn refresh_interval_secs(&self) -> Result<u64> {
        db::load_refresh_interval_secs(&self.db_path)
    }

    async fn fetch_provider_normalized(&self, config: ProviderSettingsRecord) -> Result<NormalizedSnapshot> {
        if !config.enabled {
            return Ok(NormalizedSnapshot {
                provider: config.provider,
                status: SnapshotStatus::NeedsSetup,
                headline_value: Some("已暂停".to_string()),
                numeric_value: None,
                reset_at_unix_ms: None,
                note: Some("该套餐已在设置页暂停自动刷新".to_string()),
                secondary_value: None,
                secondary_numeric: None,
                secondary_reset_at_unix_ms: None,
            });
        }

        let api_key = self
            .load_api_key(config.provider)
            .map_err(|_| anyhow::anyhow!("未配置 API Key"))?;

        match config.provider {
            ProviderKind::Zhipu => {
                zhipu::fetch_snapshot(&self.client, config.endpoint_url.as_deref(), &api_key).await
            }
            ProviderKind::Minimax => {
                minimax::fetch_snapshot(
                    &self.client,
                    config.endpoint_url.as_deref(),
                    &api_key,
                    config.model_hint.as_deref(),
                )
                .await
            }
            ProviderKind::Kimi => {
                kimi::fetch_snapshot(&self.client, config.endpoint_url.as_deref(), &api_key).await
            }
        }
    }

    fn build_provider_view(
        &self,
        record: ProviderSettingsRecord,
        now_ms: i64,
        normalized: Option<&NormalizedSnapshot>,
    ) -> Result<ProviderSnapshotView> {
        let latest = db::latest_snapshot(&self.db_path, record.provider)?;
        let seven_day_metrics =
            db::load_snapshot_metrics_since(&self.db_path, record.provider, now_ms - days_to_ms(7))?;
        let thirty_day_metrics = db::load_snapshot_metrics_since(
            &self.db_path,
            record.provider,
            now_ms - days_to_ms(30),
        )?;
        let recent_reset_history =
            db::load_recent_reset_timestamps(&self.db_path, record.provider, 8)?;

        let latest_message = latest.as_ref().and_then(|snapshot| snapshot.message.as_deref());
        let seven_day_summary = format_rollup(
            record.provider,
            latest_message,
            &compute_rollup(&seven_day_metrics),
            "7 天",
        );
        let thirty_day_summary = format_rollup(
            record.provider,
            latest_message,
            &compute_rollup(&thirty_day_metrics),
            "30 天",
        );

        if self.load_api_key(record.provider).is_err() {
            return Ok(ProviderSnapshotView {
                provider: record.provider,
                provider_label: record.provider.display_name().to_string(),
                is_enabled: record.enabled,
                status: SnapshotStatus::NeedsSetup,
                headline_title: default_headline_title(record.provider),
                headline_value: "--".to_string(),
                reset_at_label: None,
                fetched_at: format_iso(now_ms)?,
                is_stale: false,
                message: Some("尚未配置 API Key".to_string()),
                seven_day_summary,
                thirty_day_summary,
                secondary_title: resolve_secondary_title(record.provider),
                secondary_value: Some("0%".to_string()),
                secondary_percent: Some(0.0),
                secondary_reset_at_label: None,
            });
        }

        if let Some(snapshot) = latest {
            let is_stale = now_ms - snapshot.observed_at_unix_ms > 3 * self.refresh_interval_secs()? as i64 * 1000;
            let status = if is_stale {
                SnapshotStatus::Stale
            } else {
                snapshot.status
            };

            // Use normalized snapshot for secondary data if available (fresh from API)
            let (secondary_title, secondary_value, secondary_percent, secondary_reset) = normalized
                .map(|n| {
                    (
                        resolve_secondary_title(record.provider),
                        n.secondary_value.clone(),
                        n.secondary_numeric,
                        n.secondary_reset_at_unix_ms,
                    )
                })
                .unwrap_or_else(|| {
                    (
                        resolve_secondary_title(record.provider),
                        Some("0%".to_string()),
                        Some(0.0),
                        None,
                    )
                });

            return Ok(ProviderSnapshotView {
                provider: record.provider,
                provider_label: record.provider.display_name().to_string(),
                is_enabled: record.enabled,
                status,
                headline_title: resolve_headline_title(record.provider, snapshot.message.as_deref()),
                headline_value: snapshot.headline_value.unwrap_or_else(|| "--".to_string()),
                reset_at_label: resolve_reset_at_unix_ms(
                    record.provider,
                    snapshot.reset_at_unix_ms,
                    &recent_reset_history,
                    now_ms,
                )
                    .and_then(|value| format_reset_label(value, now_ms).ok()),
                fetched_at: format_iso(snapshot.observed_at_unix_ms)?,
                is_stale,
                message: snapshot.message,
                seven_day_summary,
                thirty_day_summary,
                secondary_title,
                secondary_value,
                secondary_percent,
                secondary_reset_at_label: secondary_reset
                    .and_then(|value| format_reset_label(value, now_ms).ok()),
            });
        }

        Ok(ProviderSnapshotView {
            provider: record.provider,
            provider_label: record.provider.display_name().to_string(),
            is_enabled: record.enabled,
            status: SnapshotStatus::NeedsSetup,
            headline_title: default_headline_title(record.provider),
            headline_value: "--".to_string(),
            reset_at_label: None,
            fetched_at: format_iso(now_ms)?,
            is_stale: false,
            message: Some("尚无历史数据，请手动刷新".to_string()),
            seven_day_summary,
            thirty_day_summary,
            secondary_title: resolve_secondary_title(record.provider),
            secondary_value: Some("0%".to_string()),
            secondary_percent: Some(0.0),
            secondary_reset_at_label: None,
        })
    }

    pub fn toggle_provider(&self, provider: ProviderKind) -> Result<bool> {
        db::toggle_provider_enabled(&self.db_path, provider)
    }

    pub fn build_tooltip_text(&self) -> String {
        let records = match db::load_provider_settings(&self.db_path) {
            Ok(r) => r,
            Err(_) => return "BurnRate".to_string(),
        };

        let mut parts = Vec::new();
        for record in records {
            if !record.enabled {
                parts.push(format!("{}: 已暂停", record.provider.display_name()));
                continue;
            }
            let snapshot = db::latest_snapshot(&self.db_path, record.provider)
                .ok()
                .flatten();
            let value = snapshot
                .and_then(|s| s.headline_value)
                .unwrap_or_else(|| "--".to_string());
            parts.push(format!("{}: {}", record.provider.display_name(), value));
        }

        if parts.is_empty() {
            "BurnRate".to_string()
        } else {
            parts.join(" | ")
        }
    }

    fn save_api_key(&self, provider: ProviderKind, value: &str) -> Result<()> {
        db::save_api_key(&self.db_path, provider, value)
    }

    fn load_api_key(&self, provider: ProviderKind) -> Result<String> {
        db::load_api_key(&self.db_path, provider)
    }

    fn delete_api_key(&self, provider: ProviderKind) -> Result<()> {
        db::delete_api_key(&self.db_path, provider)
    }
}

fn to_stored_snapshot(snapshot: NormalizedSnapshot) -> StoredSnapshot {
    StoredSnapshot {
        provider: snapshot.provider,
        status: snapshot.status,
        headline_value: snapshot.headline_value,
        numeric_value: snapshot.numeric_value,
        reset_at_unix_ms: snapshot.reset_at_unix_ms,
        message: snapshot.note,
        observed_at_unix_ms: now_unix_ms(),
    }
}

fn default_headline_title(provider: ProviderKind) -> String {
    match provider {
        ProviderKind::Zhipu => "5 小时窗口".to_string(),
        ProviderKind::Minimax => "当前周期".to_string(),
        ProviderKind::Kimi => "账户余额".to_string(),
    }
}

fn resolve_headline_title(provider: ProviderKind, message: Option<&str>) -> String {
    if matches!(provider, ProviderKind::Kimi)
        && message.is_some_and(|value| value.contains("本周额度"))
    {
        return "5 小时窗口".to_string();
    }

    default_headline_title(provider)
}

fn resolve_secondary_title(provider: ProviderKind) -> Option<String> {
    match provider {
        ProviderKind::Kimi => Some("7 天额度".to_string()),
        ProviderKind::Zhipu => Some("7 天额度".to_string()),
        ProviderKind::Minimax => Some("总配额".to_string()),
    }
}

fn format_rollup(
    provider: ProviderKind,
    message: Option<&str>,
    rollup: &crate::storage::rollup::RollupSummary,
    range_label: &str,
) -> Option<String> {
    match provider {
        ProviderKind::Kimi => {
            if message.is_some_and(|value| value.contains("本周额度")) {
                return Some(format!(
                    "{range_label} 最新 {}% / 峰值 {}% / 均值 {}%",
                    rollup.latest_percent?,
                    rollup.peak_percent?,
                    rollup.average_percent?
                ));
            }

            let latest = rollup.latest_value?;
            let average = rollup.average_value?;
            Some(format!(
                "{range_label} 最新 ¥{latest:.2} / 均值 ¥{average:.2}"
            ))
        }
        ProviderKind::Zhipu | ProviderKind::Minimax => Some(format!(
            "{range_label} 最新 {}% / 峰值 {}% / 均值 {}%",
            rollup.latest_percent?,
            rollup.peak_percent?,
            rollup.average_percent?
        )),
    }
}

fn format_iso(unix_ms: i64) -> Result<String> {
    let seconds = normalize_unix_timestamp_ms(unix_ms) / 1000;
    let datetime = OffsetDateTime::from_unix_timestamp(seconds)?;
    Ok(datetime.format(&Rfc3339)?)
}

fn format_reset_label(reset_at_unix_ms: i64, now_unix_ms: i64) -> Result<String> {
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    format_reset_label_with_offset(reset_at_unix_ms, now_unix_ms, offset)
}

fn resolve_reset_at_unix_ms(
    provider: ProviderKind,
    latest_reset_at_unix_ms: Option<i64>,
    reset_history: &[i64],
    now_unix_ms: i64,
) -> Option<i64> {
    let mut unique_resets = Vec::new();

    for value in latest_reset_at_unix_ms.into_iter().chain(reset_history.iter().copied()) {
        let normalized = normalize_unix_timestamp_ms(value);
        if !unique_resets.contains(&normalized) {
            unique_resets.push(normalized);
        }
    }

    unique_resets.sort_unstable_by(|left, right| right.cmp(left));

    let anchor = unique_resets.first().copied()?;
    if anchor > now_unix_ms {
        return Some(anchor);
    }

    let cycle_ms = infer_cycle_duration_ms(provider, &unique_resets)?;
    let mut next_reset = anchor;
    while next_reset <= now_unix_ms {
        next_reset += cycle_ms;
    }

    Some(next_reset)
}

fn infer_cycle_duration_ms(provider: ProviderKind, reset_history: &[i64]) -> Option<i64> {
    for window in reset_history.windows(2) {
        let newer = window[0];
        let older = window[1];
        let interval = newer - older;
        if interval > 60_000 && interval <= days_to_ms(30) {
            return Some(interval);
        }
    }

    match provider {
        ProviderKind::Zhipu => Some(5 * 60 * 60 * 1000),
        ProviderKind::Minimax | ProviderKind::Kimi => None,
    }
}

fn format_reset_label_with_offset(
    reset_at_unix_ms: i64,
    now_unix_ms: i64,
    offset: UtcOffset,
) -> Result<String> {
    let reset_at = OffsetDateTime::from_unix_timestamp(normalize_unix_timestamp_ms(reset_at_unix_ms) / 1000)?
        .to_offset(offset);
    let now = OffsetDateTime::from_unix_timestamp(normalize_unix_timestamp_ms(now_unix_ms) / 1000)?
        .to_offset(offset);
    let time_text = reset_at.format(&time::macros::format_description!("[hour]:[minute]"))?;

    if reset_at.date() == now.date() {
        return Ok(format!("今天 {time_text}"));
    }

    if reset_at.date() == (now + TimeDuration::days(1)).date() {
        return Ok(format!("明天 {time_text}"));
    }

    let date_text = reset_at.format(&time::macros::format_description!("[month]-[day]"))?;
    Ok(format!("{date_text} {time_text}"))
}

fn normalize_unix_timestamp_ms(value: i64) -> i64 {
    if value.abs() < 1_000_000_000_000 {
        value * 1000
    } else {
        value
    }
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn days_to_ms(days: i64) -> i64 {
    days * 24 * 60 * 60 * 1000
}

fn normalize_optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn mask_secret(secret: &str) -> String {
    if secret.len() <= 8 {
        return "*".repeat(secret.len());
    }
    let visible = &secret[secret.len() - 4..];
    format!("{}{}", "*".repeat(secret.len() - 4), visible)
}

#[cfg(test)]
mod tests {
    use super::{
        format_reset_label_with_offset, format_rollup, normalize_unix_timestamp_ms,
        resolve_headline_title, resolve_reset_at_unix_ms,
    };
    use crate::models::ProviderKind;
    use crate::storage::rollup::RollupSummary;
    use time::{UtcOffset, macros::datetime};

    #[test]
    fn formats_same_day_reset_label() {
        let offset = UtcOffset::from_hms(8, 0, 0).expect("offset should build");
        let now = datetime!(2026-04-04 10:00:00 +8).unix_timestamp() * 1000;
        let reset_at = datetime!(2026-04-04 15:30:00 +8).unix_timestamp() * 1000;

        let label =
            format_reset_label_with_offset(reset_at, now, offset).expect("label should format");

        assert_eq!(label, "今天 15:30");
    }

    #[test]
    fn normalizes_second_precision_reset_timestamp() {
        let offset = UtcOffset::from_hms(8, 0, 0).expect("offset should build");
        let now = datetime!(2026-04-04 23:30:00 +8).unix_timestamp() * 1000;
        let reset_at_seconds = datetime!(2026-04-05 00:15:00 +8).unix_timestamp();

        let label = format_reset_label_with_offset(reset_at_seconds, now, offset)
            .expect("label should format");

        assert_eq!(normalize_unix_timestamp_ms(reset_at_seconds) % 1000, 0);
        assert_eq!(label, "明天 00:15");
    }

    #[test]
    fn rolls_zhipu_reset_forward_when_latest_snapshot_is_missing_reset() {
        let now = datetime!(2026-04-04 19:43:00 +8).unix_timestamp() * 1000;
        let last_known_reset = datetime!(2026-04-04 18:05:26 +8).unix_timestamp() * 1000;

        let derived = resolve_reset_at_unix_ms(
            ProviderKind::Zhipu,
            None,
            &[last_known_reset],
            now,
        );

        assert_eq!(
            derived,
            Some(datetime!(2026-04-04 23:05:26 +8).unix_timestamp() * 1000)
        );
    }

    #[test]
    fn infers_minimax_reset_interval_from_history_when_latest_snapshot_is_missing_reset() {
        let now = datetime!(2026-04-04 20:10:00 +8).unix_timestamp() * 1000;
        let history = [
            datetime!(2026-04-04 15:00:00 +8).unix_timestamp() * 1000,
            datetime!(2026-04-04 20:00:00 +8).unix_timestamp() * 1000,
        ];

        let derived = resolve_reset_at_unix_ms(
            ProviderKind::Minimax,
            None,
            &history,
            now,
        );

        assert_eq!(
            derived,
            Some(datetime!(2026-04-05 01:00:00 +8).unix_timestamp() * 1000)
        );
    }

    #[test]
    fn uses_window_title_for_kimi_coding_usage() {
        let title = resolve_headline_title(ProviderKind::Kimi, Some("本周额度 81% · 04-11 20:00 重置"));

        assert_eq!(title, "5 小时窗口");
    }

    #[test]
    fn formats_kimi_coding_rollup_as_percentages() {
        let rollup = RollupSummary {
            latest_percent: Some(72),
            peak_percent: Some(83),
            average_percent: Some(51),
            latest_value: Some(0.72),
            peak_value: Some(0.83),
            average_value: Some(0.51),
        };

        let summary = format_rollup(
            ProviderKind::Kimi,
            Some("本周额度 81% · 04-11 20:00 重置"),
            &rollup,
            "7 天",
        );

        assert_eq!(summary.as_deref(), Some("7 天 最新 72% / 峰值 83% / 均值 51%"));
    }
}
