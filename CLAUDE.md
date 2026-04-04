# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

BurnRate is a macOS menu-bar app built with **Tauri 2 + Rust + React 19**. It monitors usage/burn rates for three AI provider quotas: 智谱清言 (Zhipu), MiniMax, and Kimi. It runs as a tray icon with a popover overlay and a separate settings window.

## Commands

### Dev server
```bash
. ./.cargo/env          # activate local Rust toolchain (skip if Rust is global)
npm install
npm run tauri dev        # starts Vite (port 1420) + Tauri dev build
```

### Build
```bash
. ./.cargo/env
npm run tauri build -- --debug
# output: src-tauri/target/debug/bundle/macos/BurnRate.app
```

### Tests
```bash
# Frontend (Vitest + React Testing Library + jsdom)
npm test                 # vitest run
npm run test:watch       # vitest --watch

# Backend
. ./.cargo/env
cd src-tauri && cargo test
```

### Type checking
```bash
npx tsc --noEmit
```

## Architecture

### Frontend (`src/`)

Single-page React app shared between two Tauri windows. `App.tsx` detects which window it's in via `getCurrentWindow().label` and renders either the popover dashboard or the settings surface.

| Path | Purpose |
|---|---|
| `src/App.tsx` | Root component; `PopoverSurface`, `SettingsSurface`, `ProviderCard`, `ProviderSettingsCard` all live here |
| `src/lib/burnrate.ts` | Type definitions (`ProviderKind`, `DashboardState`, `SettingsState`, etc.) and thin wrappers around `invoke()` for all 8 Tauri commands |
| `src/store/useBurnRateStore.ts` | Zustand store — bridges frontend state to Tauri `invoke` calls; handles loading/error state |
| `src/features/dashboard/summary.ts` | Pure function `buildStatusSummary()` — aggregates provider snapshots into a compact pill summary (tone + text) for the menu bar |
| `src/App.css` | All styles (glass-card, popover-layout, settings-grid) |

**Tauri commands callable from frontend** (via `@tauri-apps/api/core` `invoke`):
`get_dashboard_state`, `refresh_now`, `get_settings_state`, `save_provider_settings`, `save_runtime_preferences`, `set_launch_at_login`, `toggle_provider`, `quit_app`

**Real-time updates**: Backend emits `dashboard://updated` events; frontend listens via `@tauri-apps/api/event` `listen()`.

### Backend (`src-tauri/src/`)

| Path | Purpose |
|---|---|
| `lib.rs` | App bootstrap — registers plugins (autostart, positioner, opener), initializes `AppState`, sets up tray, registers command handlers |
| `main.rs` | Entry point, calls `tauri_app_lib::run()` |
| `app_state.rs` | Core orchestrator — owns `reqwest::Client` and DB path. Handles background refresh loop, API key management via SQLite, dashboard/settings state building, provider toggle, and provider refresh dispatch |
| `commands.rs` | 8 `#[tauri::command]` handlers — thin delegations to `AppState` methods |
| `models.rs` | Shared types: `ProviderKind`, `SnapshotStatus`, `NormalizedSnapshot`, `StoredSnapshot`, `DashboardState`, `SettingsState`, view/input types |
| `tray.rs` | Menu-bar tray setup — left-click toggles popover, double-click opens settings, right-click shows context menu. Tooltip shows compact provider status (e.g. `智谱清言: 45% \| MiniMax: ok \| Kimi: ¥12.50`), updated on every refresh via `update_tray_tooltip()` |
| `providers/zhipu.rs` | Fetches Zhipu 5-hour rolling window quota (`/api/monitor/usage/quota/limit`) |
| `providers/minimax.rs` | Fetches MiniMax coding plan usage (`/v1/api/openplatform/coding_plan/remains`); supports `model_hint` to select a specific model |
| `providers/kimi.rs` | Fetches Kimi account balance (`/v1/users/me/balance`) |
| `storage/db.rs` | SQLite schema init + CRUD for `app_settings`, `provider_settings`, `snapshots`, `api_keys` tables. Opens a fresh `Connection` per call (no connection pooling) |
| `storage/rollup.rs` | `compute_rollup()` — aggregates snapshot metrics into 7-day / 30-day summaries (latest%, peak%, avg%) |

### Key data flow

1. `AppState::spawn_background_refresh()` runs a `tokio::time::sleep` loop (default 60s, min 15s).
2. Each cycle calls `refresh_all()` which fans out to enabled providers concurrently via `futures::future::join_all`.
3. Provider modules normalize API responses into `NormalizedSnapshot`, converted to `StoredSnapshot` and written to SQLite.
4. `DashboardState` is built by reading latest snapshot + 7/30-day rollups from SQLite.
5. Emitted as `dashboard://updated` event; frontend Zustand store calls `applyDashboard()`.
6. API keys are stored in the **SQLite** `api_keys` table (provider → api_key). Keys are never stored in the OS keyring. The `mask_secret()` helper shows `****last4` in settings UI.

### Tauri window layout

- **popover** (label: `"popover"`) — 400×590, transparent, undecorated, always-on-top, hidden by default. Positioned via `tauri-plugin-positioner` at tray center. Auto-hides on focus loss.
- **settings** (label: `"settings"`) — 840×700, resizable, hidden by default.

### SQLite schema

Four tables: `app_settings` (key/value), `provider_settings` (per-provider config with `enabled` toggle), `snapshots` (time-series with index on `provider, observed_at_unix_ms DESC`), `api_keys` (provider → encrypted api_key).

### Dashboard toggle

Each `ProviderSnapshotView` includes `is_enabled`. The main dashboard `ProviderCard` renders an iOS-style CSS switch (`.switch` + `.switch-knob`) on the right side. Clicking calls `toggle_provider` → `db::toggle_provider_enabled()` which flips the `enabled` column. Toggling returns an updated `DashboardState` without a full API refresh. The tray tooltip is also updated after each toggle.

### Tray tooltip

`AppState::build_tooltip_text()` reads provider settings + latest snapshots from SQLite and formats a compact summary string. Set initially via `TrayIconBuilder::tooltip()` and updated after every background refresh and manual refresh via `update_tray_tooltip()` → `tray.set_tooltip()`. macOS requires hovering ~1-2s over the tray icon for the tooltip to appear.

### Settings API key display

Settings page `ProviderSettingsCard` shows a key-status indicator: green dot + "已保存密钥" if a key exists, grey dot + "未配置密钥" otherwise. The input placeholder shows "已保存密钥，留空则保持不变" when a key exists. The actual masked key value is never shown in the UI to avoid layout issues with long keys.

## Conventions

- Provider modules follow a consistent pattern: `fetch_snapshot(client, endpoint_override, api_key, ...) -> Result<NormalizedSnapshot>` + `parse_*_response(payload) -> Result<NormalizedSnapshot>`.
- All Tauri command errors are stringified via `.map_err(|e| e.to_string())`.
- Frontend types mirror Rust `serde(rename_all = "camelCase")` output exactly — `src/lib/burnrate.ts` is the TypeScript type authority.
- Tests use `vi.hoisted()` + `vi.mock()` to mock Tauri APIs and the Zustand store.
- The local Rust toolchain is in `.cargo/` and activated via `. ./.cargo/env`. If the user has a global Rust install this step can be skipped.
