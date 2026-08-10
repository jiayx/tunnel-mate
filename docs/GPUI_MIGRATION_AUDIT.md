# GPUI migration audit

Date: 2026-08-11
Status: native GPUI client is the primary application; the Tauri client is a
compatibility shell.

## Product surface

| Area | Native GPUI implementation | Verification |
| --- | --- | --- |
| Simple main workflow | Status-first tunnel list, filters, inline edit/diagnose, one-click start/stop | Packaged app launched on macOS |
| Advanced configuration | Keys, passwords, jump host, retry policy, startup behavior | Workspace compile and model validation tests |
| Tunnel types | Local, remote, SOCKS5 | Core diagnostics and forwarding tests |
| Tunnel lifecycle | Start, stop, stop-all, status, retry/reconnect | Manager/worker tests and runtime event wiring |
| Credentials | OS keyring; password fields excluded from JSON and backups | Serialization and plaintext-secret rejection tests |
| SSH trust/auth | Host-key confirmation and encrypted-key passphrase retry | Parser and SSH authentication tests |
| Organization | Group CRUD and all/running/group filters | Native UI wiring and config validation |
| Observability | Activity, diagnostics, capped live logs, copy/export/clear | Logger round-trip test and native UI wiring |
| Import/export | Validated portable JSON, Downloads default, and native file dialogs | Backup round-trip and rejection tests |
| Desktop integration | Native menus/shortcuts, tray, close-to-tray, graceful quit, autostart, centered startup | macOS runtime smoke test and compile checks |
| Localization | Separate Chinese and English copy selected from the system locale | Locale-selection tests and native UI wiring |
| Compatibility | Existing config/event paths, keyring service, and schema retained | Isolated store round-trip and legacy client check |

## Automated evidence

- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace`: 27 passed, 0 failed
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `CI=true pnpm typecheck`
- `CI=true pnpm build:legacy`
- GitHub Actions YAML parsed successfully

## Packaging evidence

- Release binary built with `cargo build --release -p tunnel-mate-gpui`
- macOS app created with `cargo-packager 0.11.8`
- `Info.plist` passed `plutil -lint`
- Packaged executable identified as a 64-bit Mach-O binary
- Packaged app launched successfully with an isolated
  `TUNNEL_MATE_CONFIG_DIR`

## Release boundary

Local runtime and packaging were verified on the current Intel macOS host.
Linux and Windows are covered by the build/test/package matrix in
`.github/workflows/gpui.yml`; their installers require running that workflow
on the corresponding GitHub-hosted runners. Code signing/notarization
credentials are intentionally not embedded in the repository.
