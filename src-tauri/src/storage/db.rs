use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{ProviderKind, ProviderSettingsRecord, SnapshotStatus, StoredSnapshot};
use crate::storage::rollup::SnapshotMetric;

pub fn init_database(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create app data directory")?;
    }

    let connection = Connection::open(path).context("failed to open SQLite database")?;
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS app_settings (
              key TEXT PRIMARY KEY,
              value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS provider_settings (
              provider TEXT PRIMARY KEY,
              enabled INTEGER NOT NULL DEFAULT 1,
              endpoint_url TEXT,
              model_hint TEXT,
              updated_at_unix_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS snapshots (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              provider TEXT NOT NULL,
              observed_at_unix_ms INTEGER NOT NULL,
              status TEXT NOT NULL,
              headline_value TEXT,
              numeric_value REAL,
              reset_at_unix_ms INTEGER,
              message TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_snapshots_provider_time
            ON snapshots(provider, observed_at_unix_ms DESC);

            CREATE TABLE IF NOT EXISTS api_keys (
              provider TEXT PRIMARY KEY,
              api_key TEXT NOT NULL
            );
            "#,
        )
        .context("failed to migrate SQLite schema")?;

    connection
        .execute(
            "INSERT OR IGNORE INTO app_settings(key, value) VALUES('refresh_interval_secs', '60')",
            [],
        )
        .context("failed to seed refresh interval")?;

    for provider in ProviderKind::all() {
        connection
            .execute(
                "INSERT OR IGNORE INTO provider_settings(provider, enabled, endpoint_url, model_hint, updated_at_unix_ms)
                 VALUES(?1, 1, '', '', strftime('%s','now') * 1000)",
                [provider.as_str()],
            )
            .with_context(|| format!("failed to seed provider settings for {}", provider.as_str()))?;
    }

    Ok(())
}

pub fn load_refresh_interval_secs(path: &Path) -> Result<u64> {
    let connection = Connection::open(path)?;
    let value: Option<String> = connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'refresh_interval_secs'",
            [],
            |row| row.get(0),
        )
        .optional()?;

    Ok(value
        .as_deref()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(60))
}

pub fn save_refresh_interval_secs(path: &Path, refresh_interval_secs: u64) -> Result<()> {
    let connection = Connection::open(path)?;
    connection.execute(
        "INSERT INTO app_settings(key, value) VALUES('refresh_interval_secs', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [refresh_interval_secs.to_string()],
    )?;
    Ok(())
}

pub fn load_provider_settings(path: &Path) -> Result<Vec<ProviderSettingsRecord>> {
    let connection = Connection::open(path)?;
    let mut statement = connection.prepare(
        "SELECT provider, enabled, endpoint_url, model_hint
         FROM provider_settings
         ORDER BY provider ASC",
    )?;

    let rows = statement.query_map([], |row| {
        let provider_raw: String = row.get(0)?;
        let provider = ProviderKind::try_from(provider_raw.as_str())
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(error.into()))?;

        Ok(ProviderSettingsRecord {
            provider,
            enabled: row.get::<_, i64>(1)? == 1,
            endpoint_url: normalize_optional_text(row.get::<_, String>(2)?),
            model_hint: normalize_optional_text(row.get::<_, String>(3)?),
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }

    Ok(result)
}

pub fn save_provider_settings(path: &Path, record: &ProviderSettingsRecord) -> Result<()> {
    let connection = Connection::open(path)?;
    connection.execute(
        "INSERT INTO provider_settings(provider, enabled, endpoint_url, model_hint, updated_at_unix_ms)
         VALUES(?1, ?2, ?3, ?4, strftime('%s','now') * 1000)
         ON CONFLICT(provider) DO UPDATE SET
           enabled = excluded.enabled,
           endpoint_url = excluded.endpoint_url,
           model_hint = excluded.model_hint,
           updated_at_unix_ms = excluded.updated_at_unix_ms",
        params![
            record.provider.as_str(),
            if record.enabled { 1 } else { 0 },
            record.endpoint_url.clone().unwrap_or_default(),
            record.model_hint.clone().unwrap_or_default(),
        ],
    )?;
    Ok(())
}

pub fn insert_snapshot(path: &Path, snapshot: &StoredSnapshot) -> Result<()> {
    let connection = Connection::open(path)?;
    connection.execute(
        "INSERT INTO snapshots(
           provider,
           observed_at_unix_ms,
           status,
           headline_value,
           numeric_value,
           reset_at_unix_ms,
           message
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            snapshot.provider.as_str(),
            snapshot.observed_at_unix_ms,
            snapshot.status.as_str(),
            snapshot.headline_value.clone(),
            snapshot.numeric_value,
            snapshot.reset_at_unix_ms,
            snapshot.message.clone(),
        ],
    )?;
    Ok(())
}

pub fn latest_snapshot(path: &Path, provider: ProviderKind) -> Result<Option<StoredSnapshot>> {
    let connection = Connection::open(path)?;
    let result = connection
        .query_row(
            "SELECT provider, status, headline_value, numeric_value, reset_at_unix_ms, message, observed_at_unix_ms
             FROM snapshots
             WHERE provider = ?1
             ORDER BY observed_at_unix_ms DESC
             LIMIT 1",
            [provider.as_str()],
            |row| {
                let provider_raw: String = row.get(0)?;
                let status_raw: String = row.get(1)?;
                Ok(StoredSnapshot {
                    provider: ProviderKind::try_from(provider_raw.as_str())
                        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(error.into()))?,
                    status: SnapshotStatus::try_from(status_raw.as_str())
                        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(error.into()))?,
                    headline_value: row.get(2)?,
                    numeric_value: row.get(3)?,
                    reset_at_unix_ms: row.get(4)?,
                    message: row.get(5)?,
                    observed_at_unix_ms: row.get(6)?,
                })
            },
        )
        .optional()?;

    Ok(result)
}

pub fn load_snapshot_metrics_since(
    path: &Path,
    provider: ProviderKind,
    since_unix_ms: i64,
) -> Result<Vec<SnapshotMetric>> {
    let connection = Connection::open(path)?;
    let mut statement = connection.prepare(
        "SELECT observed_at_unix_ms, numeric_value
         FROM snapshots
         WHERE provider = ?1
           AND observed_at_unix_ms >= ?2
           AND numeric_value IS NOT NULL
         ORDER BY observed_at_unix_ms ASC",
    )?;

    let rows = statement.query_map(params![provider.as_str(), since_unix_ms], |row| {
        Ok(SnapshotMetric {
            observed_at_unix_ms: row.get(0)?,
            numeric_value: row.get(1)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn load_recent_reset_timestamps(
    path: &Path,
    provider: ProviderKind,
    limit: usize,
) -> Result<Vec<i64>> {
    let connection = Connection::open(path)?;
    let mut statement = connection.prepare(
        "SELECT reset_at_unix_ms
         FROM snapshots
         WHERE provider = ?1
           AND reset_at_unix_ms IS NOT NULL
         ORDER BY observed_at_unix_ms DESC
         LIMIT ?2",
    )?;

    let rows = statement.query_map(params![provider.as_str(), limit as i64], |row| row.get(0))?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }

    Ok(result)
}

fn normalize_optional_text(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn save_api_key(path: &Path, provider: ProviderKind, api_key: &str) -> Result<()> {
    let connection = Connection::open(path)?;
    connection.execute(
        "INSERT INTO api_keys(provider, api_key) VALUES(?1, ?2)
         ON CONFLICT(provider) DO UPDATE SET api_key = excluded.api_key",
        params![provider.as_str(), api_key],
    )?;
    Ok(())
}

pub fn load_api_key(path: &Path, provider: ProviderKind) -> Result<String> {
    let connection = Connection::open(path)?;
    let result: Option<String> = connection
        .query_row(
            "SELECT api_key FROM api_keys WHERE provider = ?1",
            [provider.as_str()],
            |row| row.get(0),
        )
        .optional()?;
    result.context("credential not found")
}

pub fn delete_api_key(path: &Path, provider: ProviderKind) -> Result<()> {
    let connection = Connection::open(path)?;
    connection.execute("DELETE FROM api_keys WHERE provider = ?1", [provider.as_str()])?;
    Ok(())
}

pub fn toggle_provider_enabled(path: &Path, provider: ProviderKind) -> Result<bool> {
    let connection = Connection::open(path)?;
    let current: i64 = connection
        .query_row(
            "SELECT enabled FROM provider_settings WHERE provider = ?1",
            [provider.as_str()],
            |row| row.get(0),
        )
        .context("provider not found")?;
    let new_enabled: i64 = if current == 1 { 0 } else { 1 };
    connection.execute(
        "UPDATE provider_settings SET enabled = ?1, updated_at_unix_ms = strftime('%s','now') * 1000
         WHERE provider = ?2",
        params![new_enabled, provider.as_str()],
    )?;
    Ok(new_enabled == 1)
}
