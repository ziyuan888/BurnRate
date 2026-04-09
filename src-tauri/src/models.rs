use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Zhipu,
    Minimax,
    Kimi,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zhipu => "zhipu",
            Self::Minimax => "minimax",
            Self::Kimi => "kimi",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Zhipu => "智谱清言",
            Self::Minimax => "MiniMax",
            Self::Kimi => "Kimi",
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::Zhipu, Self::Minimax, Self::Kimi]
    }
}

impl TryFrom<&str> for ProviderKind {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "zhipu" => Ok(Self::Zhipu),
            "minimax" => Ok(Self::Minimax),
            "kimi" => Ok(Self::Kimi),
            _ => Err(format!("unknown provider: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotStatus {
    Healthy,
    Warning,
    Danger,
    NeedsSetup,
    Error,
    Stale,
}

impl SnapshotStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Warning => "warning",
            Self::Danger => "danger",
            Self::NeedsSetup => "needs_setup",
            Self::Error => "error",
            Self::Stale => "stale",
        }
    }

    pub fn severity(self) -> u8 {
        match self {
            Self::Danger => 5,
            Self::Error => 4,
            Self::Warning => 3,
            Self::Stale => 2,
            Self::Healthy => 1,
            Self::NeedsSetup => 0,
        }
    }
}

impl TryFrom<&str> for SnapshotStatus {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, String> {
        match value {
            "healthy" => Ok(Self::Healthy),
            "warning" => Ok(Self::Warning),
            "danger" => Ok(Self::Danger),
            "needs_setup" => Ok(Self::NeedsSetup),
            "error" => Ok(Self::Error),
            "stale" => Ok(Self::Stale),
            _ => Err(format!("unknown status: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedSnapshot {
    pub provider: ProviderKind,
    pub status: SnapshotStatus,
    pub headline_value: Option<String>,
    pub numeric_value: Option<f64>,
    pub reset_at_unix_ms: Option<i64>,
    pub note: Option<String>,
    // Secondary quota for dual-quota providers (e.g., Kimi 7-day weekly quota)
    pub secondary_value: Option<String>,
    pub secondary_numeric: Option<f64>,
    pub secondary_reset_at_unix_ms: Option<i64>,
    // MCP quota for Zhipu (monthly MCP calls limit)
    pub mcp_value: Option<String>,
    pub mcp_numeric: Option<f64>,
    pub mcp_limit: Option<i64>,
    pub mcp_reset_at_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSnapshotView {
    pub provider: ProviderKind,
    pub provider_label: String,
    pub is_enabled: bool,
    pub status: SnapshotStatus,
    pub headline_title: String,
    pub headline_value: String,
    pub reset_at_label: Option<String>,
    pub fetched_at: String,
    pub is_stale: bool,
    pub message: Option<String>,
    pub seven_day_summary: Option<String>,
    pub thirty_day_summary: Option<String>,
    // Secondary progress bar for dual-quota providers (e.g., Kimi: 5-hour + 7-day)
    pub secondary_title: Option<String>,
    pub secondary_value: Option<String>,
    pub secondary_percent: Option<f64>,
    pub secondary_reset_at_label: Option<String>,
    // MCP quota for Zhipu (monthly MCP calls: current/limit with percentage)
    pub mcp_title: Option<String>,
    pub mcp_value: Option<String>,
    pub mcp_percent: Option<f64>,
    pub mcp_limit: Option<i64>,
    pub mcp_reset_at_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub total_tokens: String,
    pub total_messages: i64,
    pub total_tool_calls: i64,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardState {
    pub providers: Vec<ProviderSnapshotView>,
    pub refreshed_at: String,
    pub usage_stats: UsageStats,
}

#[derive(Debug, Clone)]
pub struct UsageMetrics {
    pub tokens: i64,
    pub messages: i64,
    pub tool_calls: i64,
    pub tool_breakdown: Vec<(String, i64)>,
}

#[derive(Debug, Clone)]
pub struct ProviderSettingsRecord {
    pub provider: ProviderKind,
    pub enabled: bool,
    pub endpoint_url: Option<String>,
    pub model_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettingsView {
    pub provider: ProviderKind,
    pub provider_label: String,
    pub enabled: bool,
    pub endpoint_url: String,
    pub model_hint: String,
    pub has_api_key: bool,
    pub masked_api_key: Option<String>,
    pub supports_model_hint: bool,
    pub secret_placeholder: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsState {
    pub refresh_interval_secs: u64,
    pub launch_at_login: bool,
    pub providers: Vec<ProviderSettingsView>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProviderSettingsInput {
    pub provider: ProviderKind,
    pub enabled: bool,
    pub endpoint_url: String,
    pub model_hint: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRuntimePreferencesInput {
    pub refresh_interval_secs: u64,
}

#[derive(Debug, Clone)]
pub struct StoredSnapshot {
    pub provider: ProviderKind,
    pub status: SnapshotStatus,
    pub headline_value: Option<String>,
    pub numeric_value: Option<f64>,
    pub reset_at_unix_ms: Option<i64>,
    pub message: Option<String>,
    pub observed_at_unix_ms: i64,
}
