#[derive(Debug, Clone)]
pub struct SnapshotMetric {
    pub observed_at_unix_ms: i64,
    pub numeric_value: f64,
}

#[derive(Debug, Clone, Default)]
pub struct RollupSummary {
    pub latest_percent: Option<i64>,
    pub peak_percent: Option<i64>,
    pub average_percent: Option<i64>,
    pub latest_value: Option<f64>,
    pub peak_value: Option<f64>,
    pub average_value: Option<f64>,
}

pub fn compute_rollup(snapshots: &[SnapshotMetric]) -> RollupSummary {
    if snapshots.is_empty() {
        return RollupSummary::default();
    }

    let latest = snapshots.last().map(|item| item.numeric_value);
    let peak = snapshots
        .iter()
        .map(|item| item.numeric_value)
        .reduce(f64::max);
    let average = Some(
        snapshots
            .iter()
            .map(|item| item.numeric_value)
            .sum::<f64>()
            / snapshots.len() as f64,
    );

    RollupSummary {
        latest_percent: latest.map(to_display_percent),
        peak_percent: peak.map(to_display_percent),
        average_percent: average.map(to_display_percent),
        latest_value: latest,
        peak_value: peak,
        average_value: average,
    }
}

fn to_display_percent(value: f64) -> i64 {
    let normalized = if value > 1.0 { value / 100.0 } else { value };
    (normalized * 100.0).round() as i64
}
