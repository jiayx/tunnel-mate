<p align="center">
  <img src="src-tauri/icons/icon.png" alt="Tunnel Mate Logo" width="128" height="128">
</p>

<h1 align="center">Tunnel Mate</h1>

<p align="center">
  <strong>一个轻量、优雅且安全的跨平台 SSH 隧道与端口转发 GUI 管理工具。</strong>
</p>

<p align="center">
  <a href="README.md">English</a> | <a href="README.zh-CN.md">简体中文</a>
</p>

---

Tunnel Mate 基于 Tauri v2 和 React 构建。它会静默运行在您的系统托盘中，并提供高级的连接诊断功能，确保您的安全连接始终处于活跃状态。

---

## 🌟 核心特性

*   **SSH 隧道 GUI 管理**：轻松配置和控制本地转发 (`-L`)、远程转发 (`-R`) 以及动态转发/SOCKS5 (`-D`) 端口隧道。
*   **系统托盘深度整合**：
    *   启动时自动最小化到托盘。
    *   关闭主窗口时应用不在后台退出，而是保持在系统托盘运行。
    *   在托盘菜单中快速一键切换隧道状态，或者恢复/隐藏主窗口。
*   **导入本地 SSH 配置**：自动解析并从系统本地的 `~/.ssh/config` 导入已配置的 SSH 主机。
*   **连接诊断机制**：实时的连接诊断分析器，通过逐步的连接检测（解析 DNS、端口可用性、SSH 密钥认证等）来帮助您即时排查连接问题。
*   **事件日志记录**：内置全面的会话日志查看器，实时追踪 SSH 连接、断开、超时和重连事件。
*   **备份与恢复**：支持一键导出和导入隧道配置。
*   **自动适配状态栏主题**：托盘图标自动根据 macOS 系统状态栏的深色或浅色主题切换颜色（黑/白）。

---

## 🛠️ 技术栈

*   **后端 / 系统封装**：[Tauri v2](https://tauri.app/) (Rust)
*   **前端框架**：React 19
*   **开发语言**：TypeScript
*   **构建工具**：Vite
*   **样式框架**：Tailwind CSS
*   **图标库**：Lucide React

---

## 🚀 快速开始

### 准备工作

在您的开发机上安装以下依赖：
1.  **Node.js** (LTS 或更新版本)
2.  **Rust 工具链** (使用 `rustup` 安装)
3.  **pnpm** (包管理工具)
4.  *(仅 Linux)* 系统构建依赖（Webkit2GTK, GTK3, AppIndicator）。请参阅 Tauri 官方 Linux 安装指南。

### 开发环境配置

1.  克隆本仓库并进入项目根目录。
2.  安装前端依赖：
    ```bash
    pnpm install
    ```
3.  以开发模式启动应用：
    ```bash
    pnpm tauri dev
    ```

### 本地打包构建

如需为当前操作系统编译并打包客户端安装包：
```bash
pnpm tauri build
```
生成的安装包文件将存放在 `src-tauri/target/release/bundle/` 目录下。

---

## 🍏 macOS 安装说明 (绕过安全警告)

由于本应用未支付苹果开发者年费进行官方签名与公证，因此在 macOS 上首次安装（包括通过 Homebrew 安装）并打开时，系统会提示：
> **“无法验证此 App，因为开发者身份不明”** 或 **“Apple无法验证是否包含可能危害Mac安全或泄漏隐私的恶意软件”**

您可以通过以下两种方式轻松绕过该限制：

#### 方法 1：右键打开（推荐，最快捷）
1. 打开 **访达 (Finder)**，进入 **应用程序 (Applications)** 文件夹。
2. 找到 **Tunnel Mate** 图标，**右键点击**（或按住 `Control` 键点击）图标，在菜单中选择 **打开 (Open)**。
3. 此时弹出的安全警告框中会出现一个 **“打开”** 按钮，点击即可运行。
4. *注：只需如此操作一次，后续即可双击正常打开。*

#### 方法 2：在系统设置中允许
1. 尝试双击运行应用，触发安全警告后，点击“取消”或“好”关闭警告框。
2. 打开 Mac 的 **系统设置 (System Settings)** -> **隐私与安全 (Privacy & Security)**。
3. 向下滚动到 **安全性 (Security)** 栏目，您会看到提示：*已阻止使用“Tunnel Mate”，因为它不是来自已识别的开发者。*
4. 点击旁边的 **仍要打开 (Open Anyway)**，输入密码或使用 Touch ID 确认，然后在弹出的确认框中选择 **打开** 即可。

---

## 📦 CI/CD 与自动打包

本项目配置了完整的 GitHub Actions 工作流（`.github/workflows/release.yml`），能够自动为三大主流系统平台构建打包：

*   **macOS**：自动编译适用于 Apple Silicon 芯片 (`aarch64-apple-darwin`) 以及 Intel 芯片 (`x86_64-apple-darwin`) 的 macOS 客户端，并打包为 `.dmg` 格式。
*   **Windows**：生成 `.msi` (WiX) 安装包和 `.exe` (NSIS) 安装程序。
*   **Linux**：生成 Debian 安装包 (`.deb`) 以及免安装便携的 `.AppImage` 格式。

### 自动化发布机制：
*   **推送至 `main` 分支**：编译并在 GitHub Actions 运行详情页的 Artifacts 列表中上传安装包，方便日常测试下载。
*   **推送版本 Tag（例如 `v0.1.0`）**：自动构建全平台客户端并创建 GitHub Release 草稿页，将所有安装包作为版本附件发布。

---

## 📄 开源协议

本项目采用 [MIT 协议](LICENSE) 开源。
