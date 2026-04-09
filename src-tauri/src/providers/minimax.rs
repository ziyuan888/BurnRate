use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::models::{NormalizedSnapshot, ProviderKind, SnapshotStatus};
use crate::providers::parse_unix_timestamp_ms;

const DEFAULT_QUOTA_URL: &str =
    "https://api.minimaxi.com/v1/api/openplatform/coding_plan/remains";

pub async fn fetch_snapshot(
    client: &Client,
    endpoint_override: Option<&str>,
    api_key: &str,
    model_hint: Option<&str>,
) -> Result<NormalizedSnapshot> {
    let url = endpoint_override
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_QUOTA_URL);

    let response = client
        .get(url)
        .bearer_auth(api_key)
        .header("Content-Type", "application/json")
        .send()
        .await
        .context("failed to request MiniMax quota")?;

    let status = response.status();
    let payload: Value = response
        .json()
        .await
        .context("failed to decode MiniMax response")?;

    if status.as_u16() == 401 {
        return Err(anyhow!("MiniMax API Key 无效或已过期"));
    }

    parse_quota_response(&payload, model_hint)
}

pub fn parse_quota_response(payload: &Value, model_hint: Option<&str>) -> Result<NormalizedSnapshot> {
    if payload
        .get("base_resp")
        .and_then(|value| value.get("status_code"))
        .and_then(Value::as_i64)
        != Some(0)
    {
        return Err(anyhow!("MiniMax 套餐接口返回失败"));
    }

    let entries = payload
        .get("model_remains")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("MiniMax 套餐接口缺少 model_remains"))?;

    let preferred = model_hint.unwrap_or_default();
    let entry = entries
        .iter()
        .find(|item| item.get("model_name").and_then(Value::as_str) == Some(preferred))
        .or_else(|| {
            if preferred.is_empty() {
                None
            } else {
                entries.iter().find(|item| {
                    item.get("model_name")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .starts_with(preferred)
                })
            }
        })
        .or_else(|| {
            entries.iter().find(|item| {
                item.get("model_name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .starts_with("MiniMax-M")
            })
        })
        .or_else(|| entries.first())
        .ok_or_else(|| anyhow!("MiniMax 套餐接口没有可用模型记录"))?;

    // Primary: current interval usage
    let total = entry
        .get("current_interval_total_count")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("MiniMax 套餐接口缺少 current_interval_total_count"))?;
    let remaining = entry
        .get("current_interval_usage_count")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("MiniMax 套餐接口缺少 current_interval_usage_count"))?;

    let used_ratio = if total <= 0.0 {
        0.0
    } else {
        ((total - remaining) / total).clamp(0.0, 1.0)
    };
    let base_resp = payload.get("base_resp");

    let primary_reset_at = [
        entry.get("end_time"),
        entry.get("current_interval_end_time"),
        entry.get("nextResetTime"),
        entry.get("next_reset_time"),
        entry.get("resetAt"),
        entry.get("reset_at"),
        base_resp.and_then(|value| value.get("end_time")),
    ]
    .into_iter()
    .find_map(parse_unix_timestamp_ms);

    // Secondary: try to find daily/quota limit from total_quota_remains or other fields
    let secondary = entry
        .get("total_quota_remains")
        .and_then(Value::as_f64)
        .map(|remains| {
            let total_quota = entry
                .get("total_quota")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if total_quota > 0.0 {
                let ratio = ((total_quota - remains) / total_quota).clamp(0.0, 1.0);
                (Some(format_percent(ratio)), Some(ratio))
            } else {
                (Some("0%".to_string()), Some(0.0))
            }
        })
        .unwrap_or((Some("0%".to_string()), Some(0.0)));

    Ok(NormalizedSnapshot {
        provider: ProviderKind::Minimax,
        status: status_from_ratio(used_ratio),
        headline_value: Some(format_percent(used_ratio)),
        numeric_value: Some(used_ratio),
        reset_at_unix_ms: primary_reset_at,
        note: entry
            .get("model_name")
            .and_then(Value::as_str)
            .map(|value| format!("{} 当前窗口", value)),
        secondary_value: secondary.0,
        secondary_numeric: secondary.1,
        secondary_reset_at_unix_ms: None,
        mcp_value: None,
        mcp_numeric: None,
        mcp_limit: None,
        mcp_reset_at_unix_ms: None,
    })
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
