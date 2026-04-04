use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde_json::Value;

use crate::models::{NormalizedSnapshot, ProviderKind, SnapshotStatus};
use crate::providers::parse_unix_timestamp_ms;

const DEFAULT_BALANCE_URL: &str = "https://api.moonshot.cn/v1/users/me/balance";

pub async fn fetch_snapshot(
    client: &Client,
    endpoint_override: Option<&str>,
    api_key: &str,
) -> Result<NormalizedSnapshot> {
    let url = endpoint_override
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_BALANCE_URL);

    let response = client
        .get(url)
        .bearer_auth(api_key)
        .header("Content-Type", "application/json")
        .send()
        .await
        .context("failed to request Kimi balance")?;

    let status = response.status();
    let payload: Value = response
        .json()
        .await
        .context("failed to decode Kimi balance response")?;

    if status.as_u16() == 401 {
        return Err(anyhow!("Kimi API Key 无效或已过期"));
    }

    parse_balance_response(&payload)
}

pub fn parse_balance_response(payload: &Value) -> Result<NormalizedSnapshot> {
    let balance = payload
        .get("data")
        .and_then(|value| value.get("available_balance"))
        .and_then(|value| match value {
            Value::String(raw) => raw.parse::<f64>().ok(),
            Value::Number(raw) => raw.as_f64(),
            _ => None,
        })
        .or_else(|| payload.get("available_balance").and_then(Value::as_f64))
        .ok_or_else(|| anyhow!("Kimi 余额接口缺少 available_balance"))?;

    let currency = payload
        .get("data")
        .and_then(|value| value.get("currency"))
        .and_then(Value::as_str)
        .unwrap_or("CNY");

    Ok(NormalizedSnapshot {
        provider: ProviderKind::Kimi,
        status: status_from_balance(balance),
        headline_value: Some(format_currency(balance, currency)),
        numeric_value: Some(balance),
        reset_at_unix_ms: extract_reset_at(payload),
        note: Some("账户余额".to_string()),
    })
}

fn extract_reset_at(payload: &Value) -> Option<i64> {
    let data = payload.get("data");
    [
        data.and_then(|value| value.get("nextResetTime")),
        data.and_then(|value| value.get("next_reset_time")),
        data.and_then(|value| value.get("resetAt")),
        data.and_then(|value| value.get("reset_at")),
        data.and_then(|value| value.get("end_time")),
        payload.get("nextResetTime"),
        payload.get("next_reset_time"),
        payload.get("resetAt"),
        payload.get("reset_at"),
        payload.get("end_time"),
    ]
    .into_iter()
    .find_map(parse_unix_timestamp_ms)
}

fn status_from_balance(balance: f64) -> SnapshotStatus {
    if balance <= 0.0 {
        SnapshotStatus::Danger
    } else if balance < 10.0 {
        SnapshotStatus::Warning
    } else {
        SnapshotStatus::Healthy
    }
}

fn format_currency(balance: f64, currency: &str) -> String {
    let symbol = if currency.eq_ignore_ascii_case("CNY") {
        "¥"
    } else {
        ""
    };

    format!("{symbol}{balance:.2}")
}
