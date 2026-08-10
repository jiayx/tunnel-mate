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

## Architecture

- `apps/tunnel-mate-gpui`: primary native desktop application, pinned to the
  synchronized `gpui-unofficial` and `gpui-platform-gpui-unofficial` 1.14.2
  packages.
- `crates/tunnel-core`: UI-independent configuration, credential, diagnostics,
  SSH, forwarding, event, and tunnel-lifecycle implementation.
- `src-tauri` and `src`: temporary compatibility client. It consumes the same
  `tunnel-core`; no SSH implementation is duplicated there.

Existing users keep the same `TunnelMate/config.json`, `events.jsonl`, and
system-keyring credentials when moving from the compatibility client.

## Development

Install the stable Rust toolchain. Linux also needs GTK 3, AppIndicator,
XKBCommon, Wayland, and X11 development libraries.

```bash
cargo run -p tunnel-mate-gpui
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The npm aliases point to the native client:

```bash
pnpm dev
pnpm build
pnpm package:native
```

The packaged macOS application is written to
`target/release/Tunnel Mate.app`; installer paths vary by platform and format.

The compatibility frontend remains available through `pnpm dev:legacy` and
`pnpm build:legacy`.

## Packaging and releases

`pnpm package:native` builds and packages the current platform with
`cargo-packager`. The `.github/workflows/gpui.yml` workflow checks macOS,
Linux, and Windows and produces DMG, DEB/AppImage, and WiX/NSIS artifacts for
manual or tagged releases.

Unsigned macOS builds may require right-clicking the app and choosing **Open**
the first time.

## License

[MIT](LICENSE)
