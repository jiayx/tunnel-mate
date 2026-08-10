# Tunnel Mate GPUI

Primary native Rust UI for Tunnel Mate. It shares the UI-independent
`tunnel-core` crate with the compatibility Tauri client.

## Visual direction

The interface uses a restrained graphite palette, native dark window chrome,
hairline separators, translucent surfaces, and the icon's blue accent color.
The status-first tunnel list keeps start/stop, edit, and diagnostics visible
without a permanent detail sidebar. Configuration opens in a focused sheet,
with infrequent fields in the collapsed **Advanced settings** section.

- Generated design reference: [`docs/design/gpui-premium-concept.png`](../../docs/design/gpui-premium-concept.png)
- Implemented GPUI window: [`docs/design/gpui-premium-implemented.png`](../../docs/design/gpui-premium-implemented.png)

## Current milestone

- Reads and writes the existing configuration and system-keyring credentials.
- Creates and edits Local, Remote, and SOCKS5 tunnels with native IME-aware
  text input and validation.
- Starts, stops, monitors, and automatically reconnects real SSH tunnels.
- Shows group/running filters, persisted activity events, live session logs,
  and step-by-step connection diagnostics.
- Imports primary and jump hosts from SSH config with matched-host selection.
- Provides native application/window menus, keyboard shortcuts, tray behavior,
  system-language localization, and portable backup import/export.
- Keeps the primary workflow compact; authentication details, retry timing, and
  passwords live under **Advanced settings**.

## Run

```bash
cargo run -p tunnel-mate-gpui
```

The native client pins the synchronized `gpui-unofficial` and
`gpui-platform-gpui-unofficial` packages. Their versions mirror Zed release
tags, and the platform package selects the native backend for each target OS.

Build a distributable application with:

```bash
cargo build --release -p tunnel-mate-gpui
cargo packager --manifest-path apps/tunnel-mate-gpui/Cargo.toml --release
```

The old React/Tauri application remains only as a compatibility shell and calls
the same core manager/event API as this client.
