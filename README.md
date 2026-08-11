# Tunnel Mate

Tunnel Mate is a native, cross-platform SSH tunnel manager built with Rust and
GPUI. Its compact main window focuses on tunnel status and one-click
start/stop. Editing and diagnostics stay next to each tunnel, while infrequent
options are kept in Advanced settings.

[简体中文](README.zh-CN.md)

## Features

- Local (`-L`), remote (`-R`), and SOCKS5 (`-D`) forwarding
- Native tunnel and group CRUD with SSH config host import
- Passwords stored in the operating-system keyring, never in config backups
- Jump hosts, host-key trust prompts, encrypted-key passphrase prompts, and
  automatic reconnect
- System tray controls, start-with-app tunnels, launch at login, start
  minimized, and close to tray
- Connection diagnostics, activity history, live logs, and config backup/import
- Chinese and English UI selected from the operating-system language

## Using Tunnel Mate

Create a tunnel with the **New tunnel** button, or import host details from
`~/.ssh/config`. Local forwarding exposes a remote service on this computer;
remote forwarding exposes a local service through the SSH server; SOCKS5 mode
creates a local dynamic proxy.

The SSH host and optional jump host can both be selected from SSH config.
Tunnel Mate fills the resolved host, port, user, and identity file while keeping
the SSH alias available for matching. New tunnels enable **Reconnect when the
app starts** and **Automatic reconnect** by default.

On first contact, Tunnel Mate shows the server fingerprint and offers **Trust
and connect**. A changed key is never overwritten with one click: verify the
new fingerprint independently, copy the provided `ssh-keygen -R` command, and
connect again. Keys marked as revoked are blocked without an in-app bypass.

Use the inline **Edit** and **Diagnose** actions on a tunnel. Saving an unchanged
running tunnel does nothing; saving actual changes asks for confirmation before
disconnecting and reconnecting it. Diagnostics understand the active tunnel and
do not report its own listening port as an unrelated conflict.

Backups are portable JSON files without passwords. Export defaults to
`~/Downloads`; importing a backup validates it, stops current sessions, replaces
the configuration, and starts tunnels marked to reconnect on app launch.

On macOS, the application provides standard Application and Window menus and
supports `Command-,` for Settings, `Command-W` to close the window, `Command-M`
to minimize, and `Command-Q` to quit. If **Close to tray** is enabled, closing
the window hides it while tunnels continue running; use the Dock icon or menu
bar item to show it again.

Forms support `Tab`/`Shift-Tab` focus navigation, `Enter` for the primary
dialog action, and `Escape` to close the topmost dialog.

On Windows, the app uses the notification-area icon: left-click restores the
window and right-click opens tunnel controls. It uses the native title bar,
Mica where supported, and `Ctrl-,`, `Ctrl-W`, `Ctrl-Q`, plus the standard
`Ctrl-A/C/V/X` editing shortcuts. Linux uses a native title bar and an
AppIndicator/status-area menu with an explicit **Open Tunnel Mate** action;
availability and placement of that icon follow the desktop environment. Linux
uses the same Ctrl shortcuts and `F11` for full screen.

## Architecture

- `apps/tunnel-mate-gpui`: native desktop application, pinned to the
  synchronized `gpui-unofficial` and `gpui-platform-gpui-unofficial` 1.14.2
  packages.
- `crates/tunnel-core`: UI-independent configuration, credential, diagnostics,
  SSH, forwarding, event, and tunnel-lifecycle implementation.
- `assets/icons`: application and tray icons shared by runtime and packaging.

```text
apps/tunnel-mate-gpui  ──uses──▶  crates/tunnel-core
          │
          └── runtime and packaging assets ──▶ assets/icons
```

The repository is a pure Rust workspace; it does not require a web frontend or
webview runtime. Tunnel Mate retains the existing `TunnelMate/config.json`,
`events.jsonl`, and system-keyring credential format, so upgrades do not require
manual data migration.

## Development

Install the stable Rust toolchain. Linux also needs GTK 3, AppIndicator,
XKBCommon, Wayland, and X11 development libraries.

```bash
cargo run -p tunnel-mate-gpui
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

CI also runs a RustSec dependency audit. Dependabot checks Cargo and workflow
dependencies monthly; workflow actions are pinned to immutable commit SHAs.

## Packaging and releases

`v0.2.6` is the final Tauri release. Native GPUI releases start at `v0.5.0`
and continue on the `v0.5.x` version line.

```bash
cargo install cargo-packager --version 0.11.8 --locked
cargo build --release -p tunnel-mate-gpui
cargo packager --manifest-path apps/tunnel-mate-gpui/Cargo.toml --release
```

The packaged macOS application is written to
`target/release/Tunnel Mate.app`; installer paths vary by platform and format.
The `.github/workflows/release.yml` workflow checks macOS, Linux, and Windows
and produces DMG, DEB/AppImage, and WiX/NSIS artifacts for manual or tagged
releases. Tagged releases include `SHA256SUMS` and GitHub build-provenance
attestations. The workflow automatically uses Apple Developer ID/notarization
and Windows Authenticode credentials when their repository secrets are
configured; otherwise it explicitly produces unsigned packages.

Unsigned macOS builds may require right-clicking the app and choosing **Open**
the first time.

## License

[MIT](LICENSE)
