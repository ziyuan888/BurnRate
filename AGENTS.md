# Repository Guidelines

## Project Structure & Module Organization
BurnRate is a macOS menu-bar app built with `Tauri 2 + React 19 + TypeScript + Rust`. Frontend code lives in `src/`: `App.tsx` renders the popover and settings surfaces, `store/useBurnRateStore.ts` owns UI state, and `lib/burnrate.ts` bridges Tauri commands. Frontend tests live in `src/App.test.tsx` and `src/test/setup.ts`. Native code lives in `src-tauri/src/`; keep Tauri commands in `commands.rs`, orchestration in `app_state.rs`, and tray/window wiring in `tray.rs`. Static assets belong in `public/` or `src/assets/`. Generated output such as `dist/` and `src-tauri/target/` should not be hand-edited.

## Build, Test, and Development Commands
Use the local Rust toolchain first if needed: `. ./.cargo/env`. Main commands:

- `npm run tauri dev`: start the Vite frontend and Tauri desktop shell.
- `npm run build`: run TypeScript checks and build the web bundle into `dist/`.
- `npm test`: run frontend tests with Vitest once.
- `npm run test:watch`: run frontend tests in watch mode.
- `cd src-tauri && cargo test`: run Rust tests for provider parsing and backend behavior.
- `npm run tauri build -- --debug`: build the macOS debug app bundle.

## Coding Style & Naming Conventions
Use TypeScript with `strict` mode and 2-space indentation. Prefer functional React components, `camelCase` for variables/functions, and `PascalCase` for component/type names. Keep frontend types aligned with Rust `serde(rename_all = "camelCase")` payloads. In Rust, use `snake_case` for modules/functions. There is no lint script in this snapshot, so use `npm run build` and `cargo test` as the minimum quality gate.

## Testing Guidelines
Frontend tests use `Vitest`, `jsdom`, and `@testing-library/react`; name them `*.test.tsx` next to the feature or in `src/`. Backend tests live in `src-tauri/tests/` or inline Rust test modules. Add or update tests whenever command payloads, provider parsing, or dashboard rendering logic changes. For UI changes, verify both the popover flow and the settings window.

## Commit & Pull Request Guidelines
This workspace snapshot does not include `.git` history, so no local commit convention can be inferred. Use short Conventional Commit style messages such as `feat(tray): add stale-data indicator` or `fix(provider): handle empty minimax response`. PRs should include a clear summary, affected areas (`src/` or `src-tauri/`), test evidence (`npm test`, `cargo test`), and screenshots for popover/settings UI changes.

## Security & Configuration Tips
Do not hardcode API keys, endpoints, or sample secrets in `src/`, `src-tauri/`, or screenshots. When changing provider settings or persistence behavior, verify SQLite-backed state and masked-key UI behavior before merging.
