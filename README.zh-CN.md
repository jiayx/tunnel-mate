# Tunnel Mate

Tunnel Mate 是一款使用 Rust + GPUI 构建的原生跨平台 SSH 隧道管理器。紧凑的主界面专注于隧道状态和一键启停，编辑与诊断直接放在每条隧道上，低频选项统一收进“高级设置”。

[English](README.md)

## 主要功能

- 本地转发（`-L`）、远程转发（`-R`）和 SOCKS5（`-D`）
- 原生隧道与分组增删改查，可读取 SSH config 主机
- 密码只存入操作系统钥匙串，不会写入配置文件或备份
- 跳板机、主机密钥确认、加密私钥口令和自动重连
- 系统托盘、隧道随应用启动、登录时启动、最小化启动和关闭到托盘
- 连接诊断、活动记录、实时日志以及配置备份与恢复
- 根据操作系统语言自动选择中文或英文界面

## 使用说明

点击“新建隧道”手工创建，或从 `~/.ssh/config` 选择已有主机。本地转发会把远端服务映射到当前电脑；远程转发会通过 SSH 服务器暴露本地服务；SOCKS5 会创建本地动态代理。

SSH 主机和可选跳板机都可以从 SSH config 选择。选择后会填入解析后的主机、端口、用户和私钥，并保留按 SSH 别名匹配当前配置的能力。新建隧道默认开启“应用启动时自动重连”和“断线后自动重连”。

首次连接时会展示服务器指纹，并提供“信任并连接”。已保存的主机密钥发生变化时不会一键覆盖：请先通过管理员或服务器控制台独立核对新指纹，再复制弹窗提供的 `ssh-keygen -R` 命令清理旧记录并重新连接。被标记为撤销的密钥会被直接阻止，应用内不提供绕过。

每条隧道都有行内“编辑”和“诊断”按钮。运行中的隧道没有实际修改时，保存不会弹出提示；有修改时会先确认，然后断开并按新配置重连。诊断会识别当前正在运行的隧道，不会把它自己监听的端口误报成其他进程占用。

备份是不会包含密码的可移植 JSON 文件。导出默认定位到 `~/Downloads`；导入时会先校验备份、停止当前连接、替换配置，并自动启动其中标记为随应用重连的隧道。

在 macOS 上，应用提供标准的“应用”和“窗口”菜单，并支持 `Command-,` 打开设置、`Command-W` 关闭窗口、`Command-M` 最小化以及 `Command-Q` 退出。如果开启“关闭到托盘”，关闭窗口只会隐藏界面，隧道继续运行；可通过 Dock 图标或菜单栏图标重新显示。

在 Windows 上，应用使用任务栏通知区域图标：左键恢复窗口，右键打开隧道菜单；窗口使用原生标题栏，并在系统支持时启用 Mica。设置、关闭窗口和退出分别支持 `Ctrl-,`、`Ctrl-W`、`Ctrl-Q`，输入框支持标准的 `Ctrl-A/C/V/X`。Linux 使用原生标题栏以及 AppIndicator/状态区菜单，菜单中会保留明确的“打开 Tunnel Mate”入口；图标是否显示以及显示位置由桌面环境决定。Linux 同样使用 Ctrl 系列快捷键，并支持 `F11` 全屏。

表单支持 `Tab`/`Shift-Tab` 切换输入焦点、`Enter` 执行弹窗主操作，以及 `Escape` 关闭最上层弹窗。

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

仓库现在是纯 Rust workspace，不需要 Web 前端或 WebView 运行时。Tunnel Mate 继续兼容现有的 `TunnelMate/config.json`、`events.jsonl` 和系统钥匙串凭据格式，升级时无需手工迁移数据。

## 开发与测试

安装稳定版 Rust 工具链。Linux 还需要 GTK 3、AppIndicator、XKBCommon、
Wayland 和 X11 开发库。

```bash
cargo run -p tunnel-mate-gpui
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

CI 还会运行 RustSec 依赖安全审计。Dependabot 每月检查 Cargo 和工作流依赖，GitHub Actions 均固定到不可变的提交 SHA。

## 打包与发布

`v0.2.6` 是最后一个 Tauri 版本。原生 GPUI 版本从 `v0.5.0` 开始，后续沿用 `v0.5.x` 版本线。

首次打包前安装固定版本的打包工具：

```bash
cargo install cargo-packager --version 0.11.8 --locked
```

只打包当前 Mac 可直接运行的 `.app`：

```bash
./scripts/package-local-debug.sh
```

脚本明确使用 `app` 格式，不会创建、挂载或自动打开 DMG。默认复用 Cargo 日常开发和测试共用的增量 Debug 缓存，并跳过仅正式 Release 需要的 LTO 和符号裁剪，因此小范围 UI 修改会快很多。默认产物位于 `target/debug/Tunnel Mate.app`。

从干净的 `main` 分支准备并发布新版本：

```bash
./scripts/release.sh 0.5.2
```

发布脚本只会更新 workspace 版本号和锁文件、创建版本提交，并在推送 `main` 和对应 Tag 前要求确认；本地不会编译、测试或打包。新 Tag 推送后由 GitHub 执行检查和正式构建。传入 `--yes` 可以跳过最后的确认。

`.github/workflows/release.yml` 会检查 macOS、Linux 和 Windows，并在手工运行或推送版本标签时生成 DMG、DEB/AppImage 和 WiX/NSIS 安装包。

正式标签发布会同时生成 `SHA256SUMS` 和 GitHub 构建来源证明。仓库配置 Apple Developer ID/公证及 Windows Authenticode 密钥后，工作流会自动签名；没有相应密钥时会明确生成未签名安装包，不会伪造签名状态。

未签名的 macOS 版本首次启动时，可能需要右键应用并选择“打开”。

## 开源协议

[MIT](LICENSE)
