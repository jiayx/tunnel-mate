# Tunnel Mate

Tunnel Mate 是一款使用 Rust + GPUI 构建的原生跨平台 SSH 隧道管理器。紧凑的主界面专注于隧道状态和一键启停，编辑与诊断直接放在每条隧道上，低频选项统一收进“高级设置”。

[English](README.md)

## 主要功能

- 本地转发（`-L`）、远程转发（`-R`）和 SOCKS5（`-D`）
- 原生隧道与分组增删改查，可读取 SSH config 主机
- 密码只存入操作系统钥匙串，不会写入配置文件或备份
- 跳板机、主机密钥确认、加密私钥口令和自动重连
- 系统托盘、隧道随应用启动、登录时启动、最小化启动和关闭到托盘
- 连接诊断、活动记录以及配置备份与恢复
- 根据操作系统语言自动选择中文或英文界面

## 安装

所有安装包都可以从 [GitHub 最新版本](https://github.com/jiayx/tunnel-mate/releases/latest) 下载。

### macOS

推荐使用 Homebrew 安装，Homebrew 会自动选择 Apple 芯片或 Intel 版本：

```bash
brew install --cask jiayx/tap/tunnel-mate
```

已有安装可以通过下面的命令升级：

```bash
brew upgrade --cask tunnel-mate
```

也可以从 GitHub Releases 下载对应的 macOS DMG，再把 **Tunnel Mate** 拖入“应用程序”。当前版本未签名且未经 Apple 公证；如果首次启动被拦截，请右键选择“打开”，或前往“系统设置 > 隐私与安全性”允许打开。

### Windows

从 GitHub Releases 下载 x86_64 `.exe` 安装程序，也可以使用 `.msi` 或免安装的 `.zip` 便携版。当前版本未签名，SmartScreen 可能提示“未知发布者”。

### Linux

目前只提供 x86_64 版本。在 Debian 或 Ubuntu 上，下载 `.deb` 后执行：

```bash
sudo apt install ./tunnel-mate-*-linux-x86_64.deb
```

其他桌面发行版可以使用 AppImage。把文件放到固定目录，赋予执行权限后运行：

```bash
chmod +x tunnel-mate-*-linux-x86_64.AppImage
./tunnel-mate-*-linux-x86_64.AppImage
```

## 使用说明

1. 点击“新建隧道”，选择本地转发、远程转发或 SOCKS5，然后填写 SSH 连接信息；也可以从 `~/.ssh/config` 自动带入主机、端口、用户和私钥。
2. 填写转发端点。必填项带有星号，重连、超时和跳板机等低频选项放在高级设置中。
3. 保存后通过隧道开关连接。首次连接会显示服务器指纹，请独立核对无误后再选择“信任并连接”。
4. 隧道运行时可把 Tunnel Mate 留在 Dock、通知区域或状态栏。需要登录系统后自动恢复时，开启“开机启动”和“应用启动时自动重连”。

点击“新建隧道”手工创建，或从 `~/.ssh/config` 选择已有主机。本地转发会把远端服务映射到当前电脑；远程转发会通过 SSH 服务器暴露本地服务；SOCKS5 会创建本地动态代理。

SSH 主机和可选跳板机都可以从 SSH config 选择。选择后会填入解析后的主机、端口、用户和私钥，并保留按 SSH 别名匹配当前配置的能力。新建隧道默认开启“应用启动时自动重连”和“断线后自动重连”。

首次连接时会展示服务器指纹，并提供“信任并连接”。已保存的主机密钥发生变化时，弹窗会同时展示旧指纹和服务器刚返回的新指纹。通过管理员或服务器控制台独立核对后，可以选择“更新密钥并连接”；应用会再次确认、只替换对应的 `known_hosts` 记录，并验证连接时收到的指纹仍与弹窗一致后再重连。被标记为撤销的密钥仍会被直接阻止，应用内不提供绕过。

每条隧道都有行内“编辑”和“诊断”按钮。运行中的隧道没有实际修改时，保存不会弹出提示；有修改时会先确认，然后断开并按新配置重连。诊断会识别当前正在运行的隧道，不会把它自己监听的端口误报成其他进程占用。

备份是不会包含密码的可移植 JSON 文件。导出默认定位到 `~/Downloads`；导入时会先校验备份、停止当前连接、替换配置，并自动启动其中标记为随应用重连的隧道。

在 macOS 上，关闭窗口始终不会停止隧道；明确退出应用时才会停止隧道。如果开启“关闭窗口时隐藏 Dock 图标”，关闭窗口后还会移除 Dock 图标；可根据当前模式通过 Dock 图标或菜单栏图标重新显示。

在 Windows 上，应用使用任务栏通知区域图标：左键恢复窗口，右键打开隧道菜单；窗口使用原生标题栏，并在系统支持时启用 Mica。Linux 使用原生标题栏以及 AppIndicator/状态区菜单，菜单中会保留明确的“打开 Tunnel Mate”入口；图标是否显示以及显示位置由桌面环境决定。

## 项目结构

- `apps/tunnel-mate-gpui`：原生桌面应用，固定使用同步版本的
  `gpui-unofficial` 与 `gpui-platform-gpui-unofficial` 1.14.2。
- `crates/tunnel-core`：完全独立于 UI 的配置、凭据、诊断、SSH、端口转发、事件和隧道生命周期核心。
- `assets/icons`：运行时与打包流程共用的应用及托盘图标。

```text
apps/tunnel-mate-gpui  ──调用──▶  crates/tunnel-core
          │
          └── 运行及打包资源 ──▶ assets/icons
```

仓库是纯 Rust workspace，不需要 Web 前端或 WebView 运行时。运行配置和事件记录保存在操作系统的应用数据目录，凭据则保存在系统钥匙串中。

## 开发与测试

安装稳定版 Rust 工具链。Linux 还需要 GTK 3、AppIndicator、XKBCommon、
Wayland 和 X11 开发库。

```bash
cargo run -p tunnel-mate-gpui
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

CI 还会运行 RustSec 依赖安全审计。由于 russh 的 RSA-SHA2 支持依赖 RustCrypto RSA，且上游尚无修复版本，目前仅精确忽略 `RUSTSEC-2023-0071`；其他漏洞仍会阻止发布。Dependabot 每月检查 Cargo 和工作流依赖，GitHub Actions 均固定到不可变的提交 SHA。

## 打包与发布

首次打包前安装固定版本的打包工具：

```bash
cargo install cargo-packager --version 0.11.8 --locked
```

只打包当前 Mac 可直接运行的 `.app`：

```bash
./scripts/package-local-debug.sh
```

脚本使用 Cargo 增量 Debug 配置构建 `.app`，并对 GPUI 等第三方依赖启用适度优化。默认产物位于 `target/debug/Tunnel Mate.app`。

从干净的 `main` 分支准备并发布新版本：

```bash
./scripts/release.sh 0.5.2
```

发布脚本更新 workspace 版本号和锁文件、创建版本提交，并在推送 `main` 和对应 Tag 前要求确认。新 Tag 推送后由 GitHub 执行检查和正式构建。传入 `--yes` 可用于非交互式发布。

`.github/workflows/release.yml` 会检查 macOS、Linux 和 Windows，并在手工运行或推送版本标签时生成 DMG、DEB/AppImage 和 Windows WiX/NSIS 安装包。Windows 还会生成包含 `Tunnel Mate.exe` 的便携 ZIP，解压后无需安装即可运行。

正式版本会生成 `SHA256SUMS` 和 GitHub 构建来源证明。当前 macOS 和 Windows 发布包均未签名，配置证书后才会启用签名。

## 开源协议

[MIT](LICENSE)
