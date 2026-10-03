# Tunnel Mate

Tunnel Mate 是一款使用 Rust 和 GPUI 构建的原生 SSH 隧道管理器，支持 macOS、Windows 和 Linux。

[English](README.md)

## 功能

- 本地转发（`-L`）、远程转发（`-R`）和 SOCKS5（`-D`）
- 隧道分组、搜索、连接诊断和活动记录
- 从 SSH config 选择主机，支持跳板机、SSH Agent、私钥和密码认证
- 主机密钥验证和加密私钥口令提示
- 断线自动重连、随应用连接、登录时启动和托盘控制
- 配置备份与导入，密码保存在系统钥匙串中
- 中英文界面，以及跟随系统的深色和浅色外观
- 贴边滚动条跟随系统的常驻或自动隐藏偏好

## 安装

从 [GitHub 最新版本](https://github.com/jiayx/tunnel-mate/releases/latest) 下载。发布包附有 `SHA256SUMS` 校验文件和 GitHub 构建来源证明。

### macOS

Homebrew 会自动选择 Apple 芯片或 Intel 版本：

```bash
brew install --cask jiayx/tap/tunnel-mate
```

升级已安装的版本：

```bash
brew upgrade --cask tunnel-mate
```

也可以下载对应的 DMG，把 **Tunnel Mate** 拖入“应用程序”。应用没有 Developer ID 签名且未经 Apple 公证；如果 macOS 拦截启动，可在“系统设置 > 隐私与安全性”中允许打开。

### Windows

下载 x86_64 `.exe` 或 `.msi` 安装程序。使用便携版时，解压 `.zip` 后运行 `Tunnel Mate.exe`。请把可执行文件放在固定位置；移动文件后需要重新开启“登录时启动”。

Windows 发布包未签名，SmartScreen 可能提示“未知发布者”。

### Linux

目前提供 x86_64 安装包。在 Debian 或 Ubuntu 上执行：

```bash
sudo apt install ./tunnel-mate-*-linux-x86_64.deb
```

其他桌面发行版可将 AppImage 放在固定位置，赋予执行权限后运行：

```bash
chmod +x tunnel-mate-*-linux-x86_64.AppImage
./tunnel-mate-*-linux-x86_64.AppImage
```

## 使用

1. 点击“新建隧道”，选择转发类型：

   | 类型 | 连接路径 |
   | --- | --- |
   | 本地转发 | 本地监听端口 → SSH 服务器 → 目标服务 |
   | 远程转发 | SSH 服务器上的监听端口 → 当前电脑 → 目标服务 |
   | SOCKS5 | 本地 SOCKS5 代理 → SSH 服务器 → 客户端请求的目标 |

2. 填写 SSH 主机、端口和用户，或从 `~/.ssh/config` 选择主机，自动填入这些字段及私钥路径。身份验证选项支持指定私钥或密码，也可使用 SSH Agent 和默认私钥。跳板机及重连选项位于高级设置中。
3. 填写监听地址；本地和远程转发还需填写目标地址。点击“保存并连接”，或取消勾选“保存后连接”，仅保存配置。
4. 在隧道行内连接、断开、重试、诊断或编辑。悬停诊断、编辑图标可查看操作名称。连接按钮内的圆点表示状态：灰色为未连接，绿色为已连接，黄色为连接中或重连中，红色为失败。修改运行中隧道的连接配置，需要确认后重连；修改名称、说明或分组会保持连接。

“全部”“运行中”“失败”筛选可与分组和搜索组合使用。“活动记录”按时间显示连接和配置变更事件。

新建隧道默认开启“应用启动时连接”和“断线后自动重连”。在设置中开启“登录时启动”，可以在登录系统后运行应用；也可选择在后台启动。

macOS 关闭窗口后会保持隧道运行。“关闭窗口时隐藏 Dock 图标”控制 Dock 图标是否保留，菜单栏图标可以重新打开窗口。在 Windows 和 Linux 上，开启“关闭窗口后继续运行”会将应用留在托盘中。明确退出应用会停止隧道。Linux 托盘图标是否可用取决于桌面环境。

## 数据与主机密钥

配置和活动记录保存在操作系统配置目录下的 `TunnelMate` 文件夹中。SSH 密码保存在系统钥匙串中，不写入配置文件和备份。

可在设置中导出或导入 JSON 备份。导入会替换当前配置、停止现有连接，再启动其中设置为随应用连接的隧道。导入后需要重新填写密码。

信任新主机前，请通过独立渠道核对服务器指纹。主机密钥发生变化时，需要再次确认才能替换 `~/.ssh/known_hosts` 中对应的记录；重连时还会检查服务器指纹是否与确认的一致。被标记为撤销的密钥会被阻止。

## 从源码构建

安装稳定版 Rust 工具链。Linux 还需要 GTK 3、AppIndicator、XDo、XKBCommon、Wayland、XCB、Fontconfig、FreeType 和 Vulkan 开发库；[构建工作流](.github/workflows/release.yml) 列出了 Ubuntu 软件包。

```bash
cargo run --locked -p tunnel-mate-gpui
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

- `apps/tunnel-mate-gpui`：桌面界面和操作系统集成
- `crates/tunnel-core`：配置、凭据、SSH 转发、诊断和隧道生命周期
- `assets/icons`：应用及托盘图标

构建本地 macOS 应用包：

```bash
cargo install cargo-packager --version 0.11.8 --locked
./scripts/package-local-debug.sh
```

产物位于 `target/debug/Tunnel Mate.app`。

## 开源协议

[MIT](LICENSE)
