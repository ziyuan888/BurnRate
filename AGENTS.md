# Repository Guidelines

**Generated:** 2026-04-09  
**Project:** BurnRate - macOS menu-bar AI quota monitor  
**Stack:** Tauri 2 + React 19 + TypeScript + Rust

---

## Project Overview
BurnRate is a macOS menu-bar application that monitors AI provider usage quotas for 智谱清言 (Zhipu), MiniMax, and Kimi. It displays real-time quota status via a tray icon with a glassmorphism popover dashboard and separate settings window.

## Structure
```
BurnRate/
├── src/                    # React 19 frontend
│   ├── App.tsx            # Root: popover vs settings routing
│   ├── lib/burnrate.ts    # Tauri invoke wrappers + types
│   ├── store/             # Zustand state management
│   └── test/              # Vitest setup
├── src-tauri/             # Tauri 2 + Rust backend
│   ├── src/               # Rust source (see src-tauri/src/AGENTS.md)
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # App config
├── public/                # Static assets
└── docs/plans/            # Documentation
```

## Where to Look
| Task | Location | Notes |
|------|----------|-------|
| UI state & rendering | `src/App.tsx` | 527 lines, routes popover/settings |
| Tauri commands | `src/lib/burnrate.ts` | All `invoke()` wrappers in one place |
| State management | `src/store/useBurnRateStore.ts` | Zustand store |
| Rust orchestration | `src-tauri/src/app_state.rs` | Core refresh loop, 705 lines |
| Provider APIs | `src-tauri/src/providers/` | zhipu.rs, minimax.rs, kimi.rs |
| SQLite storage | `src-tauri/src/storage/db.rs` | Schema + CRUD |

## Commands
```bash
# Development
. ./.cargo/env           # Activate local Rust toolchain (optional)
npm run tauri dev        # Vite (port 1420) + Tauri dev build

# Build
npm run build            # TypeScript check + Vite build
npm run tauri build -- --debug   # macOS .app bundle

# Testing
npm test                 # Frontend: Vitest + jsdom
npm run test:watch       # Watch mode
cd src-tauri && cargo test       # Backend: Rust tests
```

## Conventions
- **TypeScript**: Strict mode, 2-space indent, camelCase vars/PascalCase types
- **Rust**: snake_case modules/functions, PascalCase types, Edition 2021
- **Types**: Frontend mirrors Rust `serde(rename_all = "camelCase")` exactly
- **Testing**: Frontend `*.test.tsx` co-located; Rust tests in `src-tauri/tests/`
- **API Keys**: Stored in SQLite `api_keys` table (plain-text local only); never in OS keyring

## Anti-Patterns
- **Never** add `invoke()` calls outside `lib/burnrate.ts` — keep all Tauri commands centralized
- **Never** panic in provider fetchers — always return `Err` with context
- **Never** store API keys in code/logs — use `mask_secret()` showing `****last4` only
- **Never** block async runtime — all I/O uses `reqwest` + `tokio::time::sleep`
- **Never** mutate dashboard/settings directly — treat Zustand state as immutable

## Data Flow
1. `spawn_background_refresh()` runs 60s tokio loop (min 15s)
2. `refresh_all()` fans out to enabled providers via `join_all`
3. Providers normalize to `NormalizedSnapshot` → stored in SQLite
4. `DashboardState` built from latest + 7/30-day rollups
5. Emitted as `dashboard://updated` → frontend Zustand store

## Security Notes
- API keys stored in local SQLite (`burnrate.db`), not system keychain
- Current implementation: plain-text persistence (not encrypted)
- Never commit screenshots with real keys visible
- Use `mask_secret()` for UI display

## CI/CD
- **Workflow**: `.github/workflows/release.yml`
- **Trigger**: Version tags (`v*`)
- **Matrix**: macOS (aarch64), Ubuntu 22.04, Windows
- **Action**: `tauri-apps/tauri-action@v0.6.2` for releases
