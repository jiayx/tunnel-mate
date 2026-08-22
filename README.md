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
- Connection diagnostics, activity history, and config backup/import
- Chinese and English UI selected from the operating-system language

## Installation

Download packages from the [latest GitHub release](https://github.com/jiayx/tunnel-mate/releases/latest).

### macOS

Homebrew is the recommended installation method and automatically selects the
Apple Silicon or Intel build:

```bash
brew install --cask jiayx/tap/tunnel-mate
```

Upgrade an existing installation with:

```bash
brew upgrade --cask tunnel-mate
```

Alternatively, download the matching macOS DMG from GitHub Releases and drag
**Tunnel Mate** into Applications. Current releases are unsigned and not
notarized. If macOS blocks the first launch, right-click the app and choose
**Open**, or allow it from **System Settings > Privacy & Security**.

### Windows

Download the x86_64 `.exe` installer from GitHub Releases and run it. The `.msi`
package is also available for managed installation. To use Tunnel Mate without
installing it, download the Windows `.zip`, extract it to a stable directory,
and run `Tunnel Mate.exe`; moving that file later requires re-enabling launch at
login. Current Windows packages are unsigned, so SmartScreen may show an
unknown-publisher warning.

### Linux

Only x86_64 packages are currently published. On Debian or Ubuntu, download the
`.deb` package and install it together with its desktop dependencies:

```bash
sudo apt install ./tunnel-mate-*-linux-x86_64.deb
```

On other desktop distributions, download the AppImage, make it executable, and
run it from a stable location:

```bash
chmod +x tunnel-mate-*-linux-x86_64.AppImage
./tunnel-mate-*-linux-x86_64.AppImage
```

## Using Tunnel Mate

1. Select **New tunnel**, choose local, remote, or SOCKS5 forwarding, and enter
   the SSH connection details. An entry from `~/.ssh/config` can fill the host,
   port, user, and identity file automatically.
2. Enter the forwarding endpoint. Required fields are marked with an asterisk;
   Advanced settings contain optional reconnect, timeout, and jump-host options.
3. Save the tunnel and use its switch to connect. Verify the server fingerprint
   independently before choosing **Trust and connect** on first contact.
4. Keep Tunnel Mate running in the Dock, notification area, or status area while
   the tunnel is in use. Enable launch at login and reconnect-on-launch when the
   tunnel should return automatically after signing in.

Create a tunnel with the **New tunnel** button, or import host details from
`~/.ssh/config`. Local forwarding exposes a remote service on this computer;
remote forwarding exposes a local service through the SSH server; SOCKS5 mode
creates a local dynamic proxy.

The SSH host and optional jump host can both be selected from SSH config.
Tunnel Mate fills the resolved host, port, user, and identity file while keeping
the SSH alias available for matching. New tunnels enable **Reconnect when the
app starts** and **Automatic reconnect** by default.

On first contact, Tunnel Mate shows the server fingerprint and offers **Trust
and connect**. For a changed key, it shows the saved and newly received
fingerprints side by side. After independent verification, **Update key and
connect** uses a second confirmation, replaces only the matching `known_hosts`
entry, verifies that the server still presents the displayed fingerprint, and
reconnects. Keys marked as revoked remain blocked without an in-app bypass.

Use the inline **Edit** and **Diagnose** actions on a tunnel. Saving an unchanged
running tunnel does nothing; saving actual changes asks for confirmation before
disconnecting and reconnecting it. Diagnostics understand the active tunnel and
do not report its own listening port as an unrelated conflict.

Backups are portable JSON files without passwords. Export defaults to
`~/Downloads`; importing a backup validates it, stops current sessions, replaces
the configuration, and starts tunnels marked to reconnect on app launch.

On macOS, closing the window leaves tunnels running; explicitly quitting stops
them and exits. If **Hide the Dock icon when closing the window** is enabled,
closing the window also hides the Dock icon; use the Dock icon or menu bar item
to show the window again as applicable.

On Windows, the app uses the notification-area icon: left-click restores the
window and right-click opens tunnel controls. It uses the native title bar and
Mica where supported. Linux uses a native title bar and an
AppIndicator/status-area menu with an explicit **Open Tunnel Mate** action;
availability and placement of that icon follow the desktop environment.

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
webview runtime. Runtime configuration and events are stored in the operating
system application-data directory, while credentials stay in the system
keyring.

## Development

Install the stable Rust toolchain. Linux also needs GTK 3, AppIndicator,
XKBCommon, Wayland, and X11 development libraries.

```bash
cargo run -p tunnel-mate-gpui
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

CI also runs a RustSec dependency audit. `RUSTSEC-2023-0071` is narrowly ignored
because russh's RSA-SHA2 support depends on RustCrypto RSA and upstream has no
patched release yet; all other vulnerabilities still block releases. Dependabot
checks Cargo and workflow dependencies monthly; workflow actions are pinned to
immutable commit SHAs.

## Packaging and releases

Install the pinned packaging tool once:

```bash
cargo install cargo-packager --version 0.11.8 --locked
```

Build only the local macOS application bundle with:

```bash
./scripts/package-local-debug.sh
```

The script builds an `app` bundle with Cargo's incremental debug profile and
moderate optimization for third-party dependencies such as GPUI. The default
output is `target/debug/Tunnel Mate.app`.

Prepare and publish a new tagged version from a clean `main` branch with:

```bash
./scripts/release.sh 0.5.2
```

The release script updates the workspace version and lockfile, creates the
release commit, and asks before pushing `main` and the matching tag. GitHub runs
the checks and release builds after the tag is pushed. Pass `--yes` for
non-interactive publishing.

The `.github/workflows/release.yml` workflow checks macOS, Linux, and Windows
and produces DMG, DEB/AppImage, and Windows WiX/NSIS installers. Windows also
gets a portable ZIP containing `Tunnel Mate.exe`, which can run without
installation. Tagged releases include `SHA256SUMS` and GitHub build-provenance
attestations. Current macOS and Windows packages are unsigned; signing will be
enabled after the required certificates are configured.

## License

[MIT](LICENSE)
