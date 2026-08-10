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

每条隧道都有行内“编辑”和“诊断”按钮。运行中的隧道没有实际修改时，保存不会弹出提示；有修改时会先确认，然后断开并按新配置重连。诊断会识别当前正在运行的隧道，不会把它自己监听的端口误报成其他进程占用。

备份是不会包含密码的可移植 JSON 文件。导出默认定位到 `~/Downloads`；导入时会先校验备份、停止当前连接、替换配置，并自动启动其中标记为随应用重连的隧道。

在 macOS 上，应用提供标准的“应用”和“窗口”菜单，并支持 `Command-,` 打开设置、`Command-W` 关闭窗口、`Command-M` 最小化以及 `Command-Q` 退出。如果开启“关闭到托盘”，关闭窗口只会隐藏界面，隧道继续运行；可通过 Dock 图标或菜单栏图标重新显示。

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

## 打包与发布

```bash
cargo install cargo-packager --version 0.11.8 --locked
cargo build --release -p tunnel-mate-gpui
cargo packager --manifest-path apps/tunnel-mate-gpui/Cargo.toml --release
```

打包后的 macOS 应用位于 `target/release/Tunnel Mate.app`；其他平台的安装包路径会随格式不同而变化。`.github/workflows/release.yml` 会检查 macOS、Linux 和 Windows，并在手工运行或推送版本标签时生成 DMG、DEB/AppImage 和 WiX/NSIS 安装包。

未签名的 macOS 版本首次启动时，可能需要右键应用并选择“打开”。

## 开源协议

[MIT](LICENSE)
