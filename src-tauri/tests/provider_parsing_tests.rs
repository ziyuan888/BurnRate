use tauri_app_lib::providers::{
    kimi::parse_balance_response,
    kimi::parse_coding_usage_response,
    minimax::parse_quota_response as parse_minimax_quota_response,
    zhipu::parse_quota_response as parse_zhipu_quota_response,
};
use tauri_app_lib::storage::rollup::{compute_rollup, SnapshotMetric};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[test]
fn parses_zhipu_quota_response() {
    let payload = serde_json::json!({
        "success": true,
        "data": {
            "level": "lite",
            "limits": [
                {
                    "type": "TOKENS_LIMIT",
                    "number": 5,
                    "percentage": 37.0,
                    "nextResetTime": 1712219880000_i64
                }
            ]
        }
    });

    let parsed = parse_zhipu_quota_response(&payload).expect("zhipu payload should parse");

    assert_eq!(parsed.provider.as_str(), "zhipu");
    assert_eq!(parsed.headline_value.as_deref(), Some("37%"));
    assert_eq!(parsed.status.as_str(), "healthy");
    assert_eq!(parsed.reset_at_unix_ms, Some(1712219880000_i64));
}

#[test]
fn parses_minimax_quota_response() {
    let payload = serde_json::json!({
        "base_resp": {
            "status_code": 0,
            "status_msg": "ok"
        },
        "model_remains": [
            {
                "model_name": "MiniMax-M2.5",
                "current_interval_total_count": 800,
                "current_interval_usage_count": 504,
                "end_time": 1712219880000_i64,
                "current_weekly_total_count": 2000,
                "current_weekly_usage_count": 1550,
                "weekly_end_time": 1712400000000_i64
            }
        ]
    });

    let parsed = parse_minimax_quota_response(&payload, Some("MiniMax-M2.5"))
        .expect("minimax payload should parse");

    assert_eq!(parsed.provider.as_str(), "minimax");
    assert_eq!(parsed.headline_value.as_deref(), Some("37%"));
    assert_eq!(parsed.status.as_str(), "healthy");
    assert_eq!(parsed.reset_at_unix_ms, Some(1712219880000_i64));
}

#[test]
fn parses_zhipu_reset_time_from_string() {
    let payload = serde_json::json!({
        "success": true,
        "data": {
            "limits": [
                {
                    "type": "TOKENS_LIMIT",
                    "number": 5,
                    "percentage": 37.0,
                    "nextResetTime": "1712219880000"
                }
            ]
        }
    });

    let parsed = parse_zhipu_quota_response(&payload).expect("zhipu payload should parse");

    assert_eq!(parsed.reset_at_unix_ms, Some(1712219880000_i64));
}

#[test]
fn parses_minimax_reset_time_from_string() {
    let payload = serde_json::json!({
        "base_resp": {
            "status_code": 0
        },
        "model_remains": [
            {
                "model_name": "MiniMax-M2.5",
                "current_interval_total_count": 800,
                "current_interval_usage_count": 504,
                "end_time": "1712219880000"
            }
        ]
    });

    let parsed = parse_minimax_quota_response(&payload, Some("MiniMax-M2.5"))
        .expect("minimax payload should parse");

    assert_eq!(parsed.reset_at_unix_ms, Some(1712219880000_i64));
}

#[test]
fn parses_kimi_balance_response() {
    let payload = serde_json::json!({
        "data": {
            "available_balance": "28.88",
            "currency": "CNY"
        }
    });

    let parsed = parse_balance_response(&payload).expect("kimi payload should parse");

    assert_eq!(parsed.provider.as_str(), "kimi");
    assert_eq!(parsed.headline_value.as_deref(), Some("¥28.88"));
    assert_eq!(parsed.status.as_str(), "healthy");
}

#[test]
fn parses_kimi_reset_time_when_endpoint_exposes_it() {
    let payload = serde_json::json!({
        "data": {
            "available_balance": "28.88",
            "currency": "CNY",
            "nextResetTime": "1712219880000"
        }
    });

    let parsed = parse_balance_response(&payload).expect("kimi payload should parse");

    assert_eq!(parsed.reset_at_unix_ms, Some(1712219880000_i64));
}

#[test]
fn parses_kimi_coding_usage_response() {
    let payload = serde_json::json!({
        "usages": [
            {
                "scope": "FEATURE_CODING",
                "detail": {
                    "limit": "100",
                    "remaining": "100",
                    "resetTime": "2026-04-11T12:02:42.536858Z"
                },
                "limits": [
                    {
                        "window": {
                            "duration": 300,
                            "timeUnit": "TIME_UNIT_MINUTE"
                        },
                        "detail": {
                            "limit": "100",
                            "remaining": "100",
                            "resetTime": "2026-04-04T17:02:42.536858Z"
                        }
                    }
                ]
            }
        ]
    });

    let parsed = parse_coding_usage_response(&payload).expect("kimi coding payload should parse");
    let window_reset = OffsetDateTime::parse("2026-04-04T17:02:42.536858Z", &Rfc3339)
        .expect("time should parse")
        .unix_timestamp_nanos()
        / 1_000_000;

    assert_eq!(parsed.provider.as_str(), "kimi");
    assert_eq!(parsed.headline_value.as_deref(), Some("0%"));
    assert_eq!(parsed.status.as_str(), "healthy");
    assert_eq!(parsed.numeric_value, Some(0.0));
    assert_eq!(parsed.reset_at_unix_ms, Some(window_reset as i64));
    assert!(parsed.note.as_deref().is_some_and(|text| text.contains("本周额度 0%")));
}

#[test]
fn computes_rollup_from_recent_snapshots() {
    let snapshots = vec![
        SnapshotMetric {
            observed_at_unix_ms: 1_000,
            numeric_value: 0.37,
        },
        SnapshotMetric {
            observed_at_unix_ms: 2_000,
            numeric_value: 0.61,
        },
        SnapshotMetric {
            observed_at_unix_ms: 3_000,
            numeric_value: 0.82,
        },
    ];

    let rollup = compute_rollup(&snapshots);

    assert_eq!(rollup.latest_percent, Some(82));
    assert_eq!(rollup.peak_percent, Some(82));
    assert_eq!(rollup.average_percent, Some(60));
}
