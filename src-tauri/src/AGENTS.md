# Rust Backend (src-tauri/src/) Guidelines

## OVERVIEW
Tauri 2 Rust backend orchestrating background refresh, SQLite persistence, and provider API integrations for BurnRate.

## STRUCTURE
```
src-tauri/src/
├── lib.rs              # App bootstrap, plugin registration, tray setup
├── main.rs             # Entry point
├── app_state.rs        # Core orchestrator (705 lines): refresh loop, state building
├── commands.rs         # 8 #[tauri::command] handlers → thin AppState delegations
├── models.rs           # Shared types: ProviderKind, NormalizedSnapshot, etc.
├── tray.rs             # Menu-bar tray wiring + tooltip updates
├── providers/
│   ├── mod.rs          # ProviderKind enum with display/severity helpers
│   ├── zhipu.rs        # Zhipu 5-hour quota fetcher
│   ├── minimax.rs      # MiniMax coding plan fetcher
│   └── kimi.rs         # Kimi balance fetcher
└── storage/
    ├── db.rs           # SQLite CRUD: snapshots, settings, api_keys
    └── rollup.rs       # 7-day / 30-day metric aggregation
```

## WHERE TO LOOK
- **Orchestration**: `app_state.rs` — owns `reqwest::Client`, spawns `spawn_background_refresh()` tokio loop, builds `DashboardState` from DB
- **Commands**: `commands.rs` — all handlers stringify errors via `.map_err(|e| e.to_string())`
- **Provider pattern**: Each provider has `fetch_snapshot() -> Result<NormalizedSnapshot>` + `parse_*_response()` for normalization
- **Models**: `models.rs` — `serde(rename_all = "camelCase")` for frontend compatibility; `StoredSnapshot` (DB) vs `NormalizedSnapshot` (API) vs `ProviderSnapshotView` (UI)
- **Toggle flow**: `toggle_provider` → `db::toggle_provider_enabled()` → flips `enabled` column; returns updated state without API refresh

## CONVENTIONS
- **Naming**: `snake_case` for modules/functions, `PascalCase` for types/traits
- **Errors**: Use `anyhow::Context` for chainable errors; `.context("location")` for debugging
- **Events**: Emit `dashboard://updated` after every refresh/toggle; frontend listens via Tauri event API
- **DB**: Open fresh `Connection` per call (no pooling); tables: `app_settings`, `provider_settings`, `snapshots` (indexed), `api_keys`
- **API keys**: Stored in SQLite `api_keys` table (plain-text, local only); never in OS keyring currently

## ANTI-PATTERNS
- **Do not** panic in provider fetchers — always return `Err` with context
- **Do not** store API keys in code or logs — `mask_secret()` shows `****last4` only
- **Do not** block the async runtime — all I/O uses `reqwest` + `tokio::time::sleep`
- **Do not** modify `app_state.rs` without updating `commands.rs` delegation
