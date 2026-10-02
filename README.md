# Tunnel Mate

Tunnel Mate is a native SSH tunnel manager for macOS, Windows, and Linux, built
with Rust and GPUI.

[简体中文](README.zh-CN.md)

## Features

- Local (`-L`), remote (`-R`), and SOCKS5 (`-D`) forwarding
- Tunnel groups, search, connection diagnostics, and activity history
- SSH config host selection, jump hosts, SSH agent, private-key, and password authentication
- Host-key verification and encrypted-key passphrase prompts
- Automatic reconnect, connect on app launch, launch at login, and tray controls
- Configuration backup and import; passwords stored in the system keyring
- Chinese and English UI, with light and dark appearance matching the operating system

## Installation

Download packages from the [latest GitHub release](https://github.com/jiayx/tunnel-mate/releases/latest).
Releases include `SHA256SUMS` and GitHub build-provenance attestations.

### macOS

Homebrew selects the Apple Silicon or Intel build automatically:

```bash
brew install --cask jiayx/tap/tunnel-mate
```

To upgrade:

```bash
brew upgrade --cask tunnel-mate
```

Alternatively, download the matching DMG and drag **Tunnel Mate** into
Applications. The app is not Developer ID signed or notarized. If macOS blocks
it, allow it in **System Settings > Privacy & Security**.

### Windows

Download the x86_64 `.exe` installer or `.msi` package. For a portable setup,
extract the `.zip` and run `Tunnel Mate.exe`. Keep the executable in a fixed
location; moving it requires re-enabling launch at login.

Windows packages are unsigned, so SmartScreen may show an unknown-publisher
warning.

### Linux

Packages are available for x86_64. On Debian or Ubuntu:

```bash
sudo apt install ./tunnel-mate-*-linux-x86_64.deb
```

On other desktop distributions, keep the AppImage in a fixed location and run:

```bash
chmod +x tunnel-mate-*-linux-x86_64.AppImage
./tunnel-mate-*-linux-x86_64.AppImage
```

## Usage

1. Select **New tunnel** and choose a forwarding mode:

   | Mode | Connection path |
   | --- | --- |
   | Local | Local listening port → SSH server → target service |
   | Remote | Listening port on the SSH server → this computer → target service |
   | SOCKS5 | Local SOCKS5 proxy → SSH server → destination requested by the client |

2. Enter the SSH host, port, and user, or select a host from `~/.ssh/config`.
   The selected host fills these fields and its identity file. Authentication
   options accept a private key or password; SSH agent and default keys are
   also supported. Advanced settings contain jump-host and reconnect options.
3. Enter the listening address and, for local or remote forwarding, the target
   address. Select **Save and connect**, or clear **Connect after save** to
   save without connecting.
4. Use **Connect**, **Disconnect**, **Edit**, and **Diagnose** on each tunnel.
   Editing a running tunnel's connection settings requires confirmation before
   reconnecting. Updating its name, description, or group keeps it connected.

New tunnels enable **Connect when app starts** and **Reconnect
automatically** by default. Enable **Launch at login** in Settings to start the app
after signing in; background startup is optional.

On macOS, closing the window keeps tunnels running. The setting **Hide the
Dock icon when closing the window** controls whether the Dock icon remains
visible; the menu bar icon can reopen the window. On Windows and Linux,
**Keep running after closing the window** leaves the app in the tray.
Explicitly quitting stops tunnels. Linux tray availability depends on the
desktop environment.

## Data and host keys

Configuration and activity records are stored in the operating-system config
directory under `TunnelMate`. SSH passwords are stored in the system keyring;
they are excluded from configuration files and backups.

Settings can export or import a JSON backup. Import replaces the current
configuration and stops existing sessions, then starts tunnels configured to
connect on app launch. Passwords must be entered again after importing.

Verify the displayed server fingerprint independently before trusting a new
host. Changed keys require a second confirmation before replacing the matching
`~/.ssh/known_hosts` entry. The connection checks that the server still presents
the confirmed fingerprint. Revoked keys are blocked.

## Building from source

Install the stable Rust toolchain. Linux also needs GTK 3, AppIndicator, XDo,
XKBCommon, Wayland, XCB, Fontconfig, FreeType, and Vulkan development libraries;
the [build workflow](.github/workflows/release.yml) lists the Ubuntu packages.

```bash
cargo run --locked -p tunnel-mate-gpui
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

- `apps/tunnel-mate-gpui`: desktop UI and operating-system integration
- `crates/tunnel-core`: configuration, credentials, SSH forwarding, diagnostics,
  and tunnel lifecycle
- `assets/icons`: application and tray icons

To build a local macOS app bundle:

```bash
cargo install cargo-packager --version 0.11.8 --locked
./scripts/package-local-debug.sh
```

The bundle is written to `target/debug/Tunnel Mate.app`.

## License

[MIT](LICENSE)
