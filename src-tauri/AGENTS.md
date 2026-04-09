# Tauri Backend (src-tauri/) Guidelines

## OVERVIEW
Tauri 2 application configuration and Rust backend for BurnRate. Contains Cargo workspace, Tauri configuration files, and platform-specific build settings.

## STRUCTURE
```
src-tauri/
├── src/                  # Rust source code (see src/AGENTS.md)
├── icons/               # App icons for all platforms
├── gen/                 # Generated Tauri schemas
├── tests/               # Rust integration tests
├── capabilities/        # Tauri capability configs
├── Cargo.toml          # Rust dependencies
├── tauri.conf.json     # Main Tauri configuration
├── tauri.*.conf.json   # Platform-specific configs
└── Info.plist          # macOS bundle metadata
```

## WHERE TO LOOK
- **App config**: `tauri.conf.json` — window sizes, bundle settings, security policies
- **Platform overrides**: `tauri.macos.conf.json`, `tauri.linux.conf.json`, `tauri.windows.conf.json`
- **Dependencies**: `Cargo.toml` — Rust crates, features, tauri plugins
- **Capabilities**: `capabilities/` — Tauri v2 permission sets for commands/events

## CONVENTIONS
- **Multi-crate-type lib**: `crate-type = ["staticlib", "cdylib", "rlib"]` (Windows compatibility)
- **Local toolchain**: Activate via `. ./.cargo/env` before building
- **Plugins**: Use `tauri-plugin-autostart`, `positioner`, `opener`, `store`
- **TLS**: `reqwest` with `rustls-tls` (no OpenSSL dependency)
- **SQLite**: `rusqlite` with `bundled` feature (consistent across platforms)

## ANTI-PATTERNS
- Do NOT modify window configs in `tauri.conf.json` without updating frontend window detection
- Do NOT add native dependencies without updating CI workflow (`.github/workflows/release.yml`)
- Do NOT commit `target/` directory — already in `.gitignore`

## COMMANDS
```bash
# From repo root
. ./.cargo/env
cd src-tauri

# Development
cargo check              # Fast compile check
cargo test               # Run Rust tests
cargo clippy             # Lint (if installed)

# Build
npm run tauri build -- --debug    # Debug .app bundle
npm run tauri build               # Release build
```

## NOTES
- **macOS only**: aarch64 target (Apple Silicon), no x86_64 build in CI
- **Two windows**: `popover` (400×590, frameless) + `settings` (840×700)
- **Bundle**: `app` target only (no .dmg/.msi in config)
