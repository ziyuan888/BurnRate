use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::Client;
use serde_json::{Value, json};
use time::{
    Duration as TimeDuration, OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339,
};

use crate::models::{NormalizedSnapshot, ProviderKind, SnapshotStatus};
use crate::providers::parse_unix_timestamp_ms;

const DEFAULT_BALANCE_URL: &str = "https://api.moonshot.cn/v1/users/me/balance";
const DEFAULT_CODING_USAGE_URL: &str =
    "https://www.kimi.com/apiv2/kimi.gateway.billing.v1.BillingService/GetUsages";
const FEATURE_CODING_SCOPE: &str = "FEATURE_CODING";
const FIVE_HOUR_WINDOW_MINUTES: i64 = 300;

pub async fn fetch_snapshot(
    client: &Client,
    endpoint_override: Option<&str>,
    secret: &str,
) -> Result<NormalizedSnapshot> {
    if should_use_coding_usage(secret, endpoint_override) {
        return fetch_coding_usage_snapshot(client, endpoint_override, secret).await;
    }

    let url = endpoint_override
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_BALANCE_URL);

    let response = client
        .get(url)
        .bearer_auth(secret.trim())
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
        .and_then(parse_number)
        .or_else(|| payload.get("available_balance").and_then(parse_number))
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
        secondary_value: None,
        secondary_numeric: None,
        secondary_reset_at_unix_ms: None,
    })
}

pub fn parse_coding_usage_response(payload: &Value) -> Result<NormalizedSnapshot> {
    let usages = payload
        .get("usages")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Kimi Coding 用量接口缺少 usages"))?;

    let usage = usages
        .iter()
        .find(|item| item.get("scope").and_then(Value::as_str) == Some(FEATURE_CODING_SCOPE))
        .or_else(|| usages.first())
        .ok_or_else(|| anyhow!("Kimi Coding 用量接口没有可用记录"))?;

    let detail = usage
        .get("detail")
        .ok_or_else(|| anyhow!("Kimi Coding 用量接口缺少 detail"))?;

    let total = detail
        .get("limit")
        .and_then(parse_number)
        .ok_or_else(|| anyhow!("Kimi Coding 用量接口缺少 detail.limit"))?;
    let remaining = detail
        .get("remaining")
        .and_then(parse_number)
        .ok_or_else(|| anyhow!("Kimi Coding 用量接口缺少 detail.remaining"))?;
    let weekly_used_ratio = ratio_from_limit_and_remaining(total, remaining);
    let weekly_reset = parse_reset_at(detail.get("resetTime"));
    let current_window = extract_five_hour_window_usage(usage);
    let current_used_ratio = current_window
        .as_ref()
        .map(|item| item.used_ratio)
        .unwrap_or(weekly_used_ratio);
    let current_reset = current_window
        .as_ref()
        .and_then(|item| item.reset_at_unix_ms)
        .or(weekly_reset);

    Ok(NormalizedSnapshot {
        provider: ProviderKind::Kimi,
        status: merge_status(
            status_from_usage_ratio(current_used_ratio),
            status_from_usage_ratio(weekly_used_ratio),
        ),
        headline_value: Some(format_percent(current_used_ratio)),
        numeric_value: Some(current_used_ratio),
        reset_at_unix_ms: current_reset,
        note: build_weekly_note(weekly_used_ratio, weekly_reset),
        secondary_value: Some(format_percent(weekly_used_ratio)),
        secondary_numeric: Some(weekly_used_ratio),
        secondary_reset_at_unix_ms: weekly_reset,
    })
}

fn should_use_coding_usage(secret: &str, endpoint_override: Option<&str>) -> bool {
    endpoint_override
        .is_some_and(|value| value.contains("BillingService/GetUsages"))
        || looks_like_console_token(secret)
}

fn looks_like_console_token(secret: &str) -> bool {
    parse_console_token_claims(secret).is_ok()
}

async fn fetch_coding_usage_snapshot(
    client: &Client,
    endpoint_override: Option<&str>,
    secret: &str,
) -> Result<NormalizedSnapshot> {
    let token = normalize_console_token(secret)?;
    let claims = parse_console_token_claims(token)?;
    let url = endpoint_override
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(DEFAULT_CODING_USAGE_URL);

    let response = client
        .post(url)
        .header("Accept", "*/*")
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("Cookie", format!("kimi-auth={token}"))
        .header("Origin", "https://www.kimi.com")
        .header("Referer", "https://www.kimi.com/code/console?from=kfc_overview_topbar")
        .header("R-Timezone", "Asia/Shanghai")
        .header("X-Language", "zh-CN")
        .header("X-Msh-Device-Id", claims.device_id)
        .header("X-Msh-Platform", "web")
        .header("X-Msh-Session-Id", claims.session_id)
        .header("X-Msh-Version", "1.0.0")
        .header("X-Traffic-Id", claims.traffic_id)
        .json(&json!({
            "scope": [FEATURE_CODING_SCOPE]
        }))
        .send()
        .await
        .context("failed to request Kimi coding usage")?;

    let status = response.status();
    let payload: Value = response
        .json()
        .await
        .context("failed to decode Kimi coding usage response")?;

    if matches!(status.as_u16(), 401 | 403) {
        return Err(anyhow!("Kimi 控制台 Token 无效或已过期"));
    }

    parse_coding_usage_response(&payload)
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
    .find_map(parse_reset_at)
}

fn extract_five_hour_window_usage(usage: &Value) -> Option<QuotaWindowUsage> {
    let limit = usage
        .get("limits")
        .and_then(Value::as_array)?
        .iter()
        .find(|item| {
            duration_minutes(item.get("window"))
                .is_some_and(|duration| duration == FIVE_HOUR_WINDOW_MINUTES)
        })?;

    let detail = limit.get("detail")?;
    let total = detail.get("limit").and_then(parse_number)?;
    let remaining = detail.get("remaining").and_then(parse_number)?;
    Some(QuotaWindowUsage {
        used_ratio: ratio_from_limit_and_remaining(total, remaining),
        reset_at_unix_ms: parse_reset_at(detail.get("resetTime")),
    })
}

fn build_weekly_note(used_ratio: f64, reset_at_unix_ms: Option<i64>) -> Option<String> {
    let ratio_text = format_percent(used_ratio);
    let reset_text = reset_at_unix_ms
        .and_then(|value| format_local_reset_label(value).ok())
        .map(|value| format!(" · {value} 重置"))
        .unwrap_or_default();

    Some(format!("本周额度 {ratio_text}{reset_text}"))
}

fn duration_minutes(window: Option<&Value>) -> Option<i64> {
    let window = window?;
    let duration = window.get("duration").and_then(parse_number)? as i64;
    match window.get("timeUnit").and_then(Value::as_str)? {
        "TIME_UNIT_MINUTE" => Some(duration),
        "TIME_UNIT_HOUR" => Some(duration * 60),
        _ => None,
    }
}

fn ratio_from_limit_and_remaining(total: f64, remaining: f64) -> f64 {
    if total <= 0.0 {
        0.0
    } else {
        ((total - remaining) / total).clamp(0.0, 1.0)
    }
}

fn parse_number(value: &Value) -> Option<f64> {
    match value {
        Value::String(raw) => raw.trim().parse::<f64>().ok(),
        Value::Number(raw) => raw.as_f64(),
        _ => None,
    }
}

fn parse_reset_at(value: Option<&Value>) -> Option<i64> {
    parse_unix_timestamp_ms(value).or_else(|| {
        let raw = value?.as_str()?.trim();
        let parsed = OffsetDateTime::parse(raw, &Rfc3339).ok()?;
        Some((parsed.unix_timestamp_nanos() / 1_000_000) as i64)
    })
}

fn normalize_console_token(secret: &str) -> Result<&str> {
    let trimmed = secret.trim();
    let token = trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
        .unwrap_or(trimmed);

    if token.is_empty() {
        return Err(anyhow!("Kimi 控制台 Token 不能为空"));
    }

    Ok(token)
}

fn parse_console_token_claims(secret: &str) -> Result<ConsoleTokenClaims> {
    let token = normalize_console_token(secret)?;
    let mut parts = token.split('.');
    let _header = parts.next().ok_or_else(|| anyhow!("Kimi 控制台 Token 格式无效"))?;
    let payload = parts.next().ok_or_else(|| anyhow!("Kimi 控制台 Token 格式无效"))?;
    let _signature = parts.next().ok_or_else(|| anyhow!("Kimi 控制台 Token 格式无效"))?;

    let decoded = URL_SAFE_NO_PAD
        .decode(payload)
        .context("failed to decode Kimi console token payload")?;
    let claims: Value = serde_json::from_slice(&decoded)
        .context("failed to parse Kimi console token payload")?;

    Ok(ConsoleTokenClaims {
        traffic_id: claims
            .get("sub")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Kimi 控制台 Token 缺少 sub"))?
            .to_string(),
        session_id: claims
            .get("ssid")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Kimi 控制台 Token 缺少 ssid"))?
            .to_string(),
        device_id: claims
            .get("device_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Kimi 控制台 Token 缺少 device_id"))?
            .to_string(),
    })
}

fn format_local_reset_label(reset_at_unix_ms: i64) -> Result<String> {
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let reset_at = OffsetDateTime::from_unix_timestamp(reset_at_unix_ms / 1000)?.to_offset(offset);
    let now = OffsetDateTime::now_utc().to_offset(offset);
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

fn status_from_balance(balance: f64) -> SnapshotStatus {
    if balance <= 0.0 {
        SnapshotStatus::Danger
    } else if balance < 10.0 {
        SnapshotStatus::Warning
    } else {
        SnapshotStatus::Healthy
    }
}

fn status_from_usage_ratio(ratio: f64) -> SnapshotStatus {
    if ratio >= 0.85 {
        SnapshotStatus::Danger
    } else if ratio >= 0.70 {
        SnapshotStatus::Warning
    } else {
        SnapshotStatus::Healthy
    }
}

fn merge_status(left: SnapshotStatus, right: SnapshotStatus) -> SnapshotStatus {
    if left.severity() >= right.severity() {
        left
    } else {
        right
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

fn format_percent(ratio: f64) -> String {
    format!("{}%", (ratio * 100.0).round() as i64)
}

struct ConsoleTokenClaims {
    traffic_id: String,
    session_id: String,
    device_id: String,
}

struct QuotaWindowUsage {
    used_ratio: f64,
    reset_at_unix_ms: Option<i64>,
}
