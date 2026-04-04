use serde_json::Value;

pub mod kimi;
pub mod minimax;
pub mod zhipu;

pub(crate) fn parse_unix_timestamp_ms(value: Option<&Value>) -> Option<i64> {
    let raw = match value {
        Some(Value::Number(number)) => number.as_i64()?,
        Some(Value::String(text)) => text.trim().parse::<i64>().ok()?,
        _ => return None,
    };

    Some(normalize_unix_timestamp_ms(raw))
}

fn normalize_unix_timestamp_ms(value: i64) -> i64 {
    if value.abs() < 1_000_000_000_000 {
        value * 1000
    } else {
        value
    }
}
