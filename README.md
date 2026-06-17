<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Tunnel Mate Logo" width="128" height="128">
</p>

<h1 align="center">Tunnel Mate</h1>

<p align="center">
  <strong>A lightweight, elegant, and secure cross-platform GUI application for managing SSH tunnels and port forwarding sessions.</strong>
</p>

<p align="center">
  <a href="README.md">English</a> | <a href="README.zh-CN.md">简体中文</a>
</p>

---

Tunnel Mate is built with Tauri v2 and React. It runs silently in your system tray and offers advanced connection diagnostics to ensure your secure connections are always active.

---

## 🌟 Key Features

*   **SSH Tunnel GUI Management**: Easily configure and control Local (`-L`), Remote (`-R`), and Dynamic/SOCKS5 (`-D`) port forwarding tunnels.
*   **System Tray Integration**:
    *   Minimize to tray on startup.
    *   Keep running in the background when closing the main window.
    *   Quickly toggle tunnels or restore/hide the window from the tray menu.
*   **SSH Config Host Import**: Automatically parse and import your configured SSH hosts directly from `~/.ssh/config`.
*   **Connection Diagnostics**: Real-time diagnostic inspector that executes step-by-step connection checks (resolving DNS, checking port availability, authenticating SSH keys) to troubleshoot connection issues instantly.
*   **Event Logger**: Comprehensive session log viewer tracking SSH connect, disconnect, timeout, and reconnect events.
*   **Backup & Restore**: Easily export and import your tunnel configurations.
*   **Auto-Adapting Tray Mode**: Tray icon automatically switches color/theme (black/white) based on macOS status bar light or dark theme.

---

## 🛠️ Tech Stack

*   **Backend / System Wrapper**: [Tauri v2](https://tauri.app/) (written in Rust)
*   **Frontend Library**: React 19
*   **Language**: TypeScript
*   **Bundler**: Vite
*   **Styling**: Tailwind CSS
*   **Icons**: Lucide React

---

## 🚀 Getting Started

### Prerequisites

Ensure you have the following installed on your machine:
1.  **Node.js** (LTS or later)
2.  **Rust Toolchain** (via `rustup`)
3.  **pnpm** (Package manager)
4.  *(Linux only)* System build libraries (Webkit2GTK, GTK3, AppIndicator). See Tauri's Linux setup guide.

### Development Setup

1.  Clone the repository and navigate to the project directory.
2.  Install frontend dependencies:
    ```bash
    pnpm install
    ```
3.  Launch the application in development mode:
    ```bash
    pnpm tauri dev
    ```

### Local Build

To compile and package the application installer locally for your current OS:
```bash
pnpm tauri build
```
The output installers will be generated under `src-tauri/target/release/bundle/`.

---

## 🍏 macOS Installation Notes (Bypass Security Warning)

Since this application is not signed and notarized with a paid Apple Developer Account, macOS Gatekeeper may show a warning on first launch (including installation via Homebrew):
> **"Tunnel Mate cannot be opened because the developer cannot be verified"** or **"Apple cannot verify that this app is free from malware"**

You can easily bypass this security warning using one of the following methods:

#### Method 1: Right-Click Open (Recommended & Easiest)
1. Open **Finder** and go to your **Applications** folder.
2. Locate **Tunnel Mate**, **right-click** (or hold `Control` and click) the app icon, and select **Open**.
3. In the dialog box that appears, click the **Open** button.
4. *Note: You only need to do this once. After this, you can open the app normally by double-clicking it.*

#### Method 2: Allow in System Settings
1. Try to open the app, and close the security warning dialog when it appears.
2. Open **System Settings** -> **Privacy & Security** on your Mac.
3. Scroll down to the **Security** section. You will see a message: *"Tunnel Mate" was blocked from use because it is not from an identified developer.*
4. Click **Open Anyway**, enter your Mac password or use Touch ID, and click **Open** to confirm.

---

## 📦 CI/CD & Auto-Packaging

This project features a fully configured GitHub Actions workflow (`.github/workflows/release.yml`) that builds and packages the application for 3 major platforms:

*   **macOS**: Compiles installers for both Apple Silicon (`aarch64-apple-darwin`) and Intel (`x86_64-apple-darwin`) macOS architectures.
*   **Windows**: Generates both `.msi` and `.exe` (NSIS) installers.
*   **Linux**: Generates Debian packages (`.deb`) and portable `.AppImage` packages.

### Automated Releases:
*   On pushing to `main` branch: Builds and uploads installer binaries to the Action run page as downloadable artifacts for quick testing.
*   On pushing a version tag (e.g. `v0.1.0`): Automatically builds the application and drafts a GitHub Release draft containing all installer packages.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
