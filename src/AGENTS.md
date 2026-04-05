# Frontend (src/) Guidelines

## OVERVIEW
React 19 + TypeScript frontend rendering the macOS menu-bar popover and settings surfaces via Tauri 2.

## STRUCTURE
```
src/
├── App.tsx              # Root: routes popover vs settings window
├── main.tsx             # React createRoot entry
├── lib/
│   └── burnrate.ts      # All Tauri invoke wrappers + shared types
├── store/
│   └── useBurnRateStore.ts  # Zustand store (dashboard/settings/loading/error)
├── features/
│   └── dashboard/
│       └── summary.ts   # Pure buildStatusSummary() for tray tooltip
└── test/
    └── setup.ts        # @testing-library/jest-dom/vitest import
```

## WHERE TO LOOK
- **Types + commands**: `lib/burnrate.ts` — all `invoke()` wrappers and `DashboardState`/`SettingsState` types mirror Rust `serde(rename_all = "camelCase")` payloads exactly
- **UI state**: `store/useBurnRateStore.ts` — single Zustand store owning `dashboard`, `settings`, `loading`, `error`; all async actions update loading/error atomically
- **Rendering**: `App.tsx` (527 lines) — `PopoverSurface` (dashboard) and `SettingsSurface` (inline or window); window detection via `getCurrentWindow().label === "settings"`
- **Real-time updates**: `App.tsx` listens to `dashboard://updated` event (from Tauri) to re-fetch state without manual refresh

## CONVENTIONS
- **Types**: Prefer `type` over `interface`; all Rust-derived types use exact `camelCase` field names
- **Components**: Functional only; icons as inline SVG components (e.g., `RefreshIcon`)
- **Events**: Use `dashboard://updated` event name for real-time sync; invoke commands return resolved typed payloads
- **Testing**: Vitest + jsdom + @testing-library/react; `*.test.tsx` co-located or in `src/`

## ANTI-PATTERNS
- **Do not** add new `invoke()` calls outside `lib/burnrate.ts` — keep all Tauri command invocations in one place
- **Do not** mutate `dashboard` or `settings` directly — treat state as immutable; use `applyDashboard()` only from event listeners
- **Do not** hardcode API keys or endpoints in components — use the settings flow via `saveProviderSettings`
