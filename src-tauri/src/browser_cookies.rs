use std::fs;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use rusqlite::{Connection, OpenFlags};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Browser {
    Arc,
    Chrome,
    Edge,
    Brave,
    Chromium,
}

impl Browser {
    pub fn display_name(self) -> &'static str {
        match self {
            Browser::Arc => "Arc",
            Browser::Chrome => "Chrome",
            Browser::Edge => "Edge",
            Browser::Brave => "Brave",
            Browser::Chromium => "Chromium",
        }
    }

    fn keychain_service(self) -> &'static str {
        match self {
            Browser::Arc => "Arc Safe Storage",
            Browser::Chrome => "Chrome Safe Storage",
            Browser::Edge => "Microsoft Edge Safe Storage",
            Browser::Brave => "Brave Safe Storage",
            Browser::Chromium => "Chromium Safe Storage",
        }
    }

    fn keychain_account(self) -> &'static str {
        match self {
            Browser::Arc => "Arc",
            Browser::Chrome => "Chrome",
            Browser::Edge => "Microsoft Edge",
            Browser::Brave => "Brave",
            Browser::Chromium => "Chromium",
        }
    }

    fn base_dir(self) -> Option<PathBuf> {
        let home = PathBuf::from(std::env::var("HOME").ok()?);
        let path = match self {
            Browser::Arc => home
                .join("Library")
                .join("Application Support")
                .join("Arc")
                .join("User Data"),
            Browser::Chrome => home
                .join("Library")
                .join("Application Support")
                .join("Google")
                .join("Chrome"),
            Browser::Edge => home
                .join("Library")
                .join("Application Support")
                .join("Microsoft Edge"),
            Browser::Brave => home
                .join("Library")
                .join("Application Support")
                .join("BraveSoftware")
                .join("Brave-Browser"),
            Browser::Chromium => home
                .join("Library")
                .join("Application Support")
                .join("Chromium"),
        };
        Some(path)
    }
}

fn all_browsers() -> [Browser; 5] {
    [
        Browser::Arc,
        Browser::Chrome,
        Browser::Edge,
        Browser::Brave,
        Browser::Chromium,
    ]
}

/// Returns candidate cookie DB paths in priority order.
pub fn cookie_db_paths() -> Vec<(Browser, String, PathBuf)> {
    let mut result = Vec::new();
    for browser in all_browsers() {
        let Some(base) = browser.base_dir() else {
            continue;
        };
        if !base.is_dir() {
            continue;
        }
        let mut profiles: Vec<(String, PathBuf)> = Vec::new();
        let default_path = base.join("Default");
        if default_path.is_dir() {
            profiles.push(("Default".to_string(), default_path));
        }
        for i in 1..=10 {
            let profile_path = base.join(format!("Profile {}", i));
            if profile_path.is_dir() {
                profiles.push((format!("Profile {}", i), profile_path));
            }
        }
        for (profile_name, profile_path) in profiles {
            let candidates = [
                profile_path.join("Cookies"),
                profile_path.join("Network").join("Cookies"),
            ];
            for cookie_path in candidates {
                if cookie_path.is_file() {
                    result.push((browser, profile_name.clone(), cookie_path));
                }
            }
        }
    }
    result
}

/// Try to open a SQLite connection read-only.
/// If the DB is locked, copy it to a temp file and open that copy.
fn open_cookie_db(path: &Path) -> Result<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    match Connection::open_with_flags(path, flags) {
        Ok(conn) => return Ok(conn),
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("locked") || msg.contains("database is locked") {
                let temp_dir = std::env::temp_dir();
                let file_name = path
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("cookies"));
                let temp_path = temp_dir.join(format!(
                    "burnrate_{}_{}",
                    std::process::id(),
                    file_name.to_string_lossy()
                ));
                fs::copy(path, &temp_path)
                    .with_context(|| "failed to copy locked cookie DB to temp path")?;
                let conn = Connection::open_with_flags(&temp_path, flags)
                    .with_context(|| "failed to open copied cookie DB")?;
                let _ = fs::remove_file(&temp_path);
                return Ok(conn);
            }
            Err(anyhow!("failed to open browser cookie DB: {}", e))
        }
    }
}

/// Convert Chromium's expires_utc (microseconds since 1601-01-01) to Unix milliseconds.
fn expires_utc_to_unix_ms(expires_utc: i64) -> i64 {
    let unix_us = expires_utc.saturating_sub(11644473600000000);
    unix_us / 1000
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Read the browser's Safe Storage password from macOS Keychain.
/// Uses `security -q find-generic-password -w` to get raw bytes.
fn get_safe_storage_password(browser: Browser) -> Result<Vec<u8>> {
    let mut services = vec![(browser.keychain_service(), browser.keychain_account())];
    // Fallback to Chrome's key for any Chromium browser.
    if !matches!(browser, Browser::Chrome) {
        services.push(("Chrome Safe Storage", "Chrome"));
    }

    for (service, account) in services {
        let output = std::process::Command::new("security")
            .args(["-q", "find-generic-password", "-w", "-a", account, "-s", service])
            .output()
            .with_context(|| "failed to run security command")?;

        if output.status.success() {
            // Strip trailing whitespace/newlines from the raw bytes.
            let mut end = output.stdout.len();
            while end > 0 {
                match output.stdout[end - 1] {
                    b' ' | b'\t' | b'\n' | b'\r' => end -= 1,
                    _ => break,
                }
            }
            let trimmed = &output.stdout[..end];
            if !trimmed.is_empty() {
                return Ok(trimmed.to_vec());
            }
        }
    }

    Err(anyhow!(
        "无法从钥匙串读取 {} 的 Safe Storage 密码",
        browser.display_name()
    ))
}

/// Derive the AES-128 key from the Safe Storage password.
fn derive_chromium_key(password: &[u8]) -> [u8; 16] {
    let mut key = [0u8; 16];
    ring::pbkdf2::derive(
        ring::pbkdf2::PBKDF2_HMAC_SHA1,
        NonZeroU32::new(1003).unwrap(),
        b"saltysalt",
        password,
        &mut key,
    );
    key
}

/// Decrypt a Chromium `encrypted_value` blob on macOS.
/// macOS Chromium uses AES-128-CBC with PKCS#7 padding, not GCM.
fn decrypt_chromium_cookie(encrypted_value: &[u8], key: &[u8; 16], db_version: i64) -> Result<String> {
    if encrypted_value.len() < 3 {
        return Err(anyhow!("encrypted_value too short"));
    }

    let version = &encrypted_value[0..3];
    if version != b"v10" && version != b"v11" {
        return Err(anyhow!("unsupported encrypted_value version: {:?}", version));
    }

    let ciphertext = &encrypted_value[3..];
    if ciphertext.len() % 16 != 0 {
        return Err(anyhow!(
            "ciphertext length {} is not a multiple of 16",
            ciphertext.len()
        ));
    }

    let iv = [b' '; 16];

    use aes::Aes128;
    use cbc::cipher::{BlockDecryptMut, KeyIvInit};
    type Aes128CbcDec = cbc::Decryptor<Aes128>;

    let decryptor = Aes128CbcDec::new(key.into(), &iv.into());
    let mut buf = ciphertext.to_vec();
    let plaintext = decryptor
        .decrypt_padded_mut::<cbc::cipher::block_padding::Pkcs7>(&mut buf)
        .map_err(|_| anyhow!("AES-128-CBC decryption or PKCS#7 unpadding failed"))?;

    // Starting from Chromium cookie DB version 24, a 32-byte domain integrity
    // hash is prepended to the decrypted value.
    let cookie_value = if db_version >= 24 {
        if plaintext.len() < 32 {
            return Err(anyhow!("decrypted value too short for domain integrity check"));
        }
        &plaintext[32..]
    } else {
        plaintext
    };

    String::from_utf8(cookie_value.to_vec())
        .map_err(|_| anyhow!("decrypted cookie value is not valid UTF-8"))
}

fn pkcs7_unpad(data: &[u8]) -> Option<&[u8]> {
    if data.is_empty() {
        return None;
    }
    let pad_len = *data.last()? as usize;
    if pad_len == 0 || pad_len > 16 {
        return None;
    }
    if pad_len > data.len() {
        return None;
    }
    let (content, padding) = data.split_at(data.len() - pad_len);
    if padding.iter().all(|&b| b == pad_len as u8) {
        Some(content)
    } else {
        None
    }
}

fn get_cookie_db_version(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT value FROM meta WHERE key = 'version'",
        [],
        |row| row.get::<_, String>(0),
    )
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(0)
}

/// Search all supported Chromium browsers for a `kimi-auth` cookie under `kimi.com`.
pub fn find_kimi_auth_cookie() -> Result<(String, String)> {
    let candidates = cookie_db_paths();
    if candidates.is_empty() {
        return Err(anyhow!("未安装支持的浏览器（Chrome / Edge / Brave / Arc / Chromium）"));
    }

    let mut diagnostics = Vec::new();

    for (browser, profile_name, db_path) in candidates {
        let source_label = format!("{} {}", browser.display_name(), profile_name);
        match query_kimi_auth_cookie(&db_path, browser) {
            Ok(Some(token)) => {
                return Ok((token, source_label));
            }
            Ok(None) => {
                match diagnose_kimi_cookies(&db_path) {
                    Ok(info) => diagnostics.push(format!("{}: {}", source_label, info)),
                    Err(e) => diagnostics.push(format!("{}: 无法查询 ({e})", source_label)),
                }
                continue;
            }
            Err(e) => {
                diagnostics.push(format!("{}: 查询失败 ({e})", source_label));
                continue;
            }
        }
    }

    Err(anyhow!(
        "未找到 Kimi 登录会话。\n\n排查信息：\n{}",
        diagnostics.join("\n")
    ))
}

fn query_kimi_auth_cookie(db_path: &Path, browser: Browser) -> Result<Option<String>> {
    let conn = open_cookie_db(db_path)?;
    let now_ms = now_unix_ms();
    let db_version = get_cookie_db_version(&conn);

    let mut stmt = conn.prepare(
        "SELECT value, encrypted_value, expires_utc FROM cookies \
         WHERE host_key LIKE '%kimi.com%' AND name = 'kimi-auth' \
         ORDER BY expires_utc DESC LIMIT 1",
    )?;

    let row = stmt.query_row([], |row| {
        let value: Option<String> = row.get(0)?;
        let encrypted_value: Option<Vec<u8>> = row.get(1)?;
        let expires_utc: i64 = row.get(2)?;
        Ok((value, encrypted_value, expires_utc))
    });

    match row {
        Ok((value, encrypted_value, expires_utc)) => {
            if expires_utc > 0 && expires_utc_to_unix_ms(expires_utc) < now_ms {
                return Ok(None); // expired
            }
            if let Some(v) = value {
                if !v.trim().is_empty() {
                    return Ok(Some(v));
                }
            }
            if let Some(enc) = encrypted_value {
                if !enc.is_empty() {
                    let password = get_safe_storage_password(browser)?;
                    let key = derive_chromium_key(&password);
                    return decrypt_chromium_cookie(&enc, &key, db_version).map(Some);
                }
            }
            Ok(None)
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(anyhow!("query failed: {}", e)),
    }
}

fn diagnose_kimi_cookies(db_path: &Path) -> Result<String> {
    let conn = open_cookie_db(db_path)?;

    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cookies WHERE host_key LIKE '%kimi.com%'",
        [],
        |row| row.get(0),
    )?;

    if total == 0 {
        return Ok("无 kimi.com cookie".to_string());
    }

    let mut stmt = conn.prepare(
        "SELECT DISTINCT name FROM cookies WHERE host_key LIKE '%kimi.com%'"
    )?;
    let names: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();

    let encrypted_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cookies WHERE host_key LIKE '%kimi.com%' AND name = 'kimi-auth' AND (value IS NULL OR value = '') AND LENGTH(encrypted_value) > 0",
        [],
        |row| row.get(0),
    )?;

    let mut parts = vec![format!("共 {total} 条 kimi.com cookie，名称: {}", names.join(", "))];
    if encrypted_count > 0 {
        parts.push(format!("其中 {encrypted_count} 条 kimi-auth 已加密"));
    }
    Ok(parts.join("；"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;

    #[test]
    fn test_expires_utc_conversion() {
        let expected_unix_ms: i64 = 1735689600000;
        let expires_utc = expected_unix_ms * 1000 + 11644473600000000;
        assert_eq!(expires_utc_to_unix_ms(expires_utc), expected_unix_ms);
    }

    #[test]
    fn test_pkcs7_unpad() {
        assert_eq!(pkcs7_unpad(b"hello\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b\x0b"), Some(b"hello".as_slice()));
        assert_eq!(pkcs7_unpad(b"hello world\x05\x05\x05\x05\x05"), Some(b"hello world".as_slice()));
        assert_eq!(pkcs7_unpad(b"0123456789abcdef\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10\x10"), Some(b"0123456789abcdef".as_slice()));
        assert_eq!(pkcs7_unpad(b""), None);
    }

    #[test]
    fn test_query_kimi_auth_cookie_found() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("burnrate_test_cookies_{}.db", std::process::id()));
        let _ = fs::remove_file(&db_path);

        let conn = Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE cookies (
                host_key TEXT,
                name TEXT,
                value TEXT,
                encrypted_value BLOB,
                path TEXT,
                expires_utc INTEGER
            )",
            [],
        )
        .unwrap();

        let far_future = (now_unix_ms() * 1000) + 11644473600000000 + 1_000_000_000;
        conn.execute(
            "INSERT INTO cookies (host_key, name, value, encrypted_value, path, expires_utc) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![".kimi.com", "kimi-auth", "test.jwt.token", Vec::<u8>::new(), "/", far_future],
        )
        .unwrap();

        let result = query_kimi_auth_cookie(&db_path, Browser::Chrome).unwrap();
        assert_eq!(result, Some("test.jwt.token".to_string()));

        let _ = fs::remove_file(&db_path);
    }

    #[test]
    fn test_query_kimi_auth_cookie_expired() {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("burnrate_test_cookies_expired_{}.db", std::process::id()));
        let _ = fs::remove_file(&db_path);

        let conn = Connection::open(&db_path).unwrap();
        conn.execute(
            "CREATE TABLE cookies (
                host_key TEXT,
                name TEXT,
                value TEXT,
                encrypted_value BLOB,
                path TEXT,
                expires_utc INTEGER
            )",
            [],
        )
        .unwrap();

        let past = 11644473600000001_i64;
        conn.execute(
            "INSERT INTO cookies (host_key, name, value, encrypted_value, path, expires_utc) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![".kimi.com", "kimi-auth", "expired.token", Vec::<u8>::new(), "/", past],
        )
        .unwrap();

        let result = query_kimi_auth_cookie(&db_path, Browser::Chrome).unwrap();
        assert_eq!(result, None);

        let _ = fs::remove_file(&db_path);
    }
}
