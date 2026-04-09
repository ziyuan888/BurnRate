use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::models::{NormalizedSnapshot, ProviderKind, SnapshotStatus};
use crate::providers::parse_unix_timestamp_ms;

const DEFAULT_QUOTA_URL: &str = "https://api.z.ai/api/monitor/usage/quota/limit";

pub async fn fetch_snapshot(
    client: &Client,
    endpoint_override: Option<&str>,
    api_key: &str,
) -> Result<NormalizedSnapshot> {
    let url = derive_quota_url(endpoint_override).unwrap_or_else(|| DEFAULT_QUOTA_URL.to_string());
    let response = client
        .get(url)
        .header("Accept", "application/json, text/plain, */*")
        .header("Authorization", normalize_auth_header(api_key))
        .send()
        .await
        .context("failed to request Zhipu quota")?;

    let status = response.status();
    let payload: Value = response
        .json()
        .await
        .context("failed to decode Zhipu response")?;

    if status.as_u16() == 401 {
        return Err(anyhow!("智谱凭证无效或已过期"));
    }

    parse_quota_response(&payload)
}

pub fn parse_quota_response(payload: &Value) -> Result<NormalizedSnapshot> {
    let success = payload
        .get("success")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !success {
        return Err(anyhow!("智谱套餐接口未返回 success=true"));
    }

    let limits = payload
        .get("data")
        .and_then(|data| data.get("limits"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("智谱套餐接口缺少 limits"))?;

    // Find 5-hour window (primary) - Token usage
    let rolling_limit = limits.iter().find(|item| {
        item.get("type").and_then(Value::as_str) == Some("TOKENS_LIMIT")
            && item.get("number").and_then(Value::as_i64) == Some(5)
    });

    let item = rolling_limit.ok_or_else(|| anyhow!("智谱套餐接口缺少 5 小时窗口"))?;
    let percent = item
        .get("percentage")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("智谱套餐接口缺少 percentage"))?;
    let numeric_value = if percent > 1.0 { percent / 100.0 } else { percent };
    let data = payload.get("data");

    let primary_reset_at = [
        item.get("nextResetTime"),
        item.get("next_reset_time"),
        item.get("resetAt"),
        item.get("reset_at"),
        data.and_then(|value| value.get("nextResetTime")),
        data.and_then(|value| value.get("next_reset_time")),
        payload.get("nextResetTime"),
        payload.get("next_reset_time"),
    ]
    .into_iter()
    .find_map(parse_unix_timestamp_ms);

    // Try to find 7-day window (secondary) - look for daily or weekly limits
    let daily_limit = limits.iter().find(|item| {
        item.get("type").and_then(Value::as_str) == Some("TOKENS_LIMIT")
            && item.get("number").and_then(Value::as_i64) == Some(1)
    });

    let (secondary_value, secondary_numeric, secondary_reset_at) = daily_limit
        .map(|daily| {
            let daily_percent = daily
                .get("percentage")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let daily_ratio = if daily_percent > 1.0 { daily_percent / 100.0 } else { daily_percent };
            let daily_reset = [
                daily.get("nextResetTime"),
                daily.get("next_reset_time"),
                daily.get("resetAt"),
                daily.get("reset_at"),
            ]
            .into_iter()
            .find_map(parse_unix_timestamp_ms);
            (
                Some(format_percent(daily_ratio)),
                Some(daily_ratio),
                daily_reset,
            )
        })
        .unwrap_or((Some("0%".to_string()), Some(0.0), None));

    // Find MCP limit (monthly MCP calls) - look for type "MCP_LIMIT" with 30-day window
    let mcp_limit_item = limits.iter().find(|item| {
        let item_type = item.get("type").and_then(Value::as_str);
        let number = item.get("number").and_then(Value::as_i64);
        // MCP_LIMIT with 30-day window or 1-month window
        (item_type == Some("MCP_LIMIT") || item_type == Some("CALLS_LIMIT"))
            && (number == Some(30) || number == Some(1))
    });

    let (mcp_value, mcp_numeric, mcp_limit, mcp_reset_at) = mcp_limit_item
        .map(|mcp| {
            let mcp_percent = mcp
                .get("percentage")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let mcp_ratio = if mcp_percent > 1.0 { mcp_percent / 100.0 } else { mcp_percent };
            let mcp_limit_val = mcp
                .get("limit")
                .and_then(Value::as_i64)
                .or_else(|| mcp.get("max").and_then(Value::as_i64))
                .or_else(|| mcp.get("total").and_then(Value::as_i64));
            let mcp_current = mcp
                .get("current")
                .and_then(Value::as_i64)
                .or_else(|| mcp.get("used").and_then(Value::as_i64));
            let mcp_reset = [
                mcp.get("nextResetTime"),
                mcp.get("next_reset_time"),
                mcp.get("resetAt"),
                mcp.get("reset_at"),
            ]
            .into_iter()
            .find_map(parse_unix_timestamp_ms);
            
            let value_str = match (mcp_current, mcp_limit_val) {
                (Some(cur), Some(lim)) if lim > 0 => format!("{}/{}", cur, lim),
                _ => format_percent(mcp_ratio),
            };
            
            (
                Some(value_str),
                Some(mcp_ratio),
                mcp_limit_val,
                mcp_reset,
            )
        })
        .unwrap_or((None, None, None, None));

    Ok(NormalizedSnapshot {
        provider: ProviderKind::Zhipu,
        status: status_from_ratio(numeric_value),
        headline_value: Some(format_percent(numeric_value)),
        numeric_value: Some(numeric_value),
        reset_at_unix_ms: primary_reset_at,
        note: Some("5 小时窗口".to_string()),
        secondary_value,
        secondary_numeric,
        secondary_reset_at_unix_ms: secondary_reset_at,
        mcp_value,
        mcp_numeric,
        mcp_limit,
        mcp_reset_at_unix_ms: mcp_reset_at,
    })
}

fn derive_quota_url(endpoint_override: Option<&str>) -> Option<String> {
    let override_value = endpoint_override?.trim();
    if override_value.is_empty() {
        return None;
    }

    if override_value.contains("/api/monitor/usage/quota/limit") {
        return Some(override_value.to_string());
    }

    let parsed = reqwest::Url::parse(override_value).ok()?;
    let host = parsed.host_str()?;
    if host.contains("api.z.ai")
        || host.contains("open.bigmodel.cn")
        || host.contains("dev.bigmodel.cn")
    {
        return Some(format!(
            "{}://{host}/api/monitor/usage/quota/limit",
            parsed.scheme()
        ));
    }

    None
}

fn normalize_auth_header(api_key: &str) -> String {
    if api_key.to_ascii_lowercase().starts_with("bearer ") {
        api_key.to_string()
    } else {
        format!("Bearer {api_key}")
    }
}

fn status_from_ratio(ratio: f64) -> SnapshotStatus {
    if ratio >= 0.85 {
        SnapshotStatus::Danger
    } else if ratio >= 0.70 {
        SnapshotStatus::Warning
    } else {
        SnapshotStatus::Healthy
    }
}

fn format_percent(ratio: f64) -> String {
    format!("{}%", (ratio * 100.0).round() as i64)
}
