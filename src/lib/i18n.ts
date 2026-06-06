import React, { createContext, useContext, useState } from "react";

export type Language = "en" | "zh";

export const translations = {
  en: {
    // Sidebar
    searchPlaceholder: "Search tunnels...",
    btnNewTunnel: "Tunnel",
    btnNewGroup: "Group",
    dashboardOverview: "Dashboard",
    dashboardDesc: "Monitor and manage your active secure SSH tunnels and global status.",
    sshTunnels: "SSH Tunnels",
    noTunnels: "No tunnels found",
    startAll: "Start All",
    stopAll: "Stop All",
    deleteTunnel: "Delete Tunnel",
    language: "Language",
    theme: "Theme",
    light: "Light",
    dark: "Dark",
    system: "System",
    createGroup: "Create Group",
    groupName: "Group Name",
    descOptional: "Description (Optional)",
    btnCancel: "Cancel",
    btnSave: "Save Group",
    btnSaveSettings: "Save Settings",
    editTunnel: "Edit Tunnel",
    renameGroup: "Rename",
    deleteGroup: "Delete Group",
    copyGroupName: "Copy Group Name",
    newTunnelInGroup: "New Tunnel Inside",
    confirmDeleteGroup: "Are you sure you want to delete this group? (Tunnels inside will be kept ungrouped)",

    // TunnelOverview Dashboard
    welcome: "Welcome to Tunnel Mate",
    welcomeSub: "Manage, monitor, and automate secure port forwards without complex commands.",
    btnImport: "Import",
    btnExport: "Export",
    activeTunnels: "Active Forwards",
    localPorts: "Local Listeners",
    systemHealth: "System Health",
    healthy: "Healthy",
    alertsCount: "{count} Alerts",
    noActivePorts: "No active ports bound",
    systemStatusDetailsHealthy: "All services operational",
    systemStatusDetailsAlert: "Connection drops detected",
    quickStart: "Quick Start",
    step1Title: "1. Add Tunnel",
    step1Desc: "Click \"+ Tunnel\" or import SSH configuration.",
    step2Title: "2. Diagnostics",
    step2Desc: "Verify key authorization and connection paths.",
    step3Title: "3. Start Tunnel",
    step3Desc: "Start the tunnel and monitor live connection logs.",
    recentGlobalEvents: "Recent Global Events",

    // Selected Tunnel Overview
    connectionParams: "Connection Parameters",
    jumpHostMap: "Jump Host Routing Map",
    bastion: "Bastion",
    target: "Target",
    stats: "Statistics",
    uptime: "Uptime",
    host: "Host",
    port: "Port",
    user: "User",
    privateKey: "Private Key",
    localBind: "Local Bind",
    remoteDest: "Remote Destination",
    startTunnel: "Start Tunnel",
    stopTunnel: "Stop Tunnel",
    diagnostics: "Diagnostics",
    tabOverview: "Overview",
    tabLogs: "Live Logs",
    tabEvents: "History Events",
    tabSettings: "Settings",
    noEvents: "No history events for this tunnel",
    clearLogs: "Clear Logs",
    copyLogs: "Copy Logs",
    exportLogs: "Export Logs",
    logsSearchPlaceholder: "Filter log lines...",
    eventSearchPlaceholder: "Filter audit events...",
    sshConfigImport: "SSH Config Import",
    sshConfigImportHint: "Choose a host entry to prefill SSH connection fields.",
    sshConfigImportTitle: "Import SSH Host",
    sshConfigNoHostName: "No HostName",
    forwardingType: "Forwarding Type",

    // TunnelForm
    titleCreate: "Create SSH Tunnel",
    titleEdit: "Edit Tunnel: {name}",
    tabGeneral: "General",
    tabSsh: "SSH Connection",
    tabForward: "Port Forwarding",
    tabJump: "Jump Host",
    tabBehavior: "Behavior",
    tunnelName: "Tunnel Name",
    descriptionOpt: "Description (Optional)",
    groupEnv: "Group / Environment",
    noGroup: "No Group (Ungrouped)",
    localForward: "Local Port Forward (L)",
    remoteForward: "Remote Port Forward (R)",
    socks5Forward: "SOCKS5 Dynamic Proxy (D)",
    sshHost: "SSH Host / IP",
    sshPort: "Port",
    sshUser: "SSH Username",
    privateKeyPath: "Private Key Path (Optional)",
    privateKeyDesc: "Leave empty to use active ssh-agent or default key directories (`~/.ssh/id_rsa`, `~/.ssh/id_ed25519`).",
    localAddress: "Local Address",
    localPort: "Local Port",
    remoteDestHost: "Remote Destination Host",
    localDestHost: "Local Destination Host",
    destPort: "Destination Port",
    enableJumpHost: "Enable SSH Jump Host (ProxyJump)",
    jumpHostDesc: "Route connection to target host via an intermediate bastion server.",
    selectBastion: "Select Existing Tunnel as Bastion",
    startWithApp: "Start Automatically with Application",
    autoReconnect: "Auto Reconnect on Connection Loss",
    maxRetries: "Max Reconnection Retries",
    retryInterval: "Retry Interval (Seconds)",
    btnDelete: "Delete Tunnel",
    btnDeleteConfirm: "Are you sure you want to delete this tunnel?",
    btnSaveTunnel: "Save Tunnel",
    errNameRequired: "Name is required",
    errHostRequired: "SSH Host is required",
    errUserRequired: "SSH Username is required",
    errLocalPortRequired: "Local port is required",
    errDestHostRequired: "Destination host is required",
    errDestPortRequired: "Destination port is required",
    errJumpHostRequired: "Jump Host is required",
    errInvalidPort: "Port must be an integer between 1 and 65535",
    errInvalidRetries: "Retries must be between 0 and 100",
    errInvalidInterval: "Interval must be between 1 and 3600 seconds",
    descLocalTitle: "Local Port Forwarding (L)",
    descLocalBody: "Forwards traffic from a local port on your computer to a remote destination host:port over the SSH server.",
    descRemoteTitle: "Remote Port Forwarding (R)",
    descRemoteBody: "Forwards traffic from a port on the remote SSH server to a local destination host:port on your network.",
    descSocksTitle: "SOCKS5 Proxy Tunneling (D)",
    descSocksBody: "Spawns a local SOCKS5 proxy server. Any traffic sent to this port is dynamically routed to the destination via the SSH tunnel.",

    // DiagnosticsModal
    titleConnectionTest: "Connection Test",
    subTitleConnectionTest: "Diagnosing connection for \"{name}\"",
    diagnosticExecutionError: "Diagnostic Execution Error",
    runningChecks: "Running connection checks...",
    passphraseRequired: "Passphrase Required",
    passphraseDesc: "This private SSH key is passphrase-encrypted. Enter the password to connect.",
    passphrasePlaceholder: "Enter private key passphrase...",
    btnVerifyKey: "Verify Key",
    btnClose: "Close",
    btnRetryTest: "Retry Test",
    btnChecking: "Checking...",

    // EventsViewer
    searchEventsLog: "Search events log...",
    btnRefreshLogs: "Refresh Logs",
    noEventsFound: "No events found",
    ev_created: "Created",
    ev_updated: "Updated",
    ev_started: "Started",
    ev_stopped: "Stopped",
    ev_reconnected: "Reconnected",
    ev_failed: "Failed",
    ev_deleted: "Deleted",

    // LogsViewer
    terminalWaiting: "Terminal active. Waiting for tunnel connection logs...",
    noMatchingLogs: "No matching log lines",

    // SettingsModal
    globalSettings: "Global Settings",
    appBehavior: "General",
    networkTimeouts: "SSH & Network",
    dataManagement: "Data and Backup",
    launchOnStartup: "Launch on Startup",
    launchOnStartupDesc: "Start Tunnel Mate automatically when your computer boots up.",
    closeToTray: "Close Window to System Tray",
    closeToTrayDesc: "Keep the application running in the background when closing the main window.",
    startMinimized: "Start Minimized to System Tray",
    startMinimizedDesc: "Launch the application silently in the tray without showing the main window.",
    keepAlive: "SSH KeepAlive Interval (Seconds)",
    keepAliveDesc: "Send keepalive packets to prevent connection dropouts. Use 0 to disable.",
    connTimeout: "SSH Connection Timeout (Seconds)",
    connTimeoutDesc: "Maximum seconds to wait when establishing a new connection.",
    backupRestore: "Backup & Restore Configurations",
    backupRestoreDesc: "Export all your tunnels, environments, and settings to a JSON backup file, or restore them.",
    clearEvents: "Clear History Event Logs",
    clearEventsDesc: "Permanently delete all connection and configuration change events.",
    btnClearEvents: "Clear History",
    settingsSaved: "Settings saved successfully",
    configImported: "Configuration imported successfully",
  },
  zh: {
    // Sidebar
    searchPlaceholder: "搜索隧道...",
    btnNewTunnel: "添加隧道",
    btnNewGroup: "新建分组",
    dashboardOverview: "仪表盘",
    dashboardDesc: "监控并管理您的活跃 SSH 安全隧道及全局系统状态。",
    sshTunnels: "SSH 隧道",
    noTunnels: "未找到隧道",
    startAll: "全部启动",
    stopAll: "全部停止",
    deleteTunnel: "删除隧道",
    language: "语言",
    theme: "主题",
    light: "浅色模式",
    dark: "深色模式",
    system: "跟随系统",
    createGroup: "创建分组",
    groupName: "分组名称",
    descOptional: "描述（可选）",
    btnCancel: "取消",
    btnSave: "保存分组",
    btnSaveSettings: "保存设置",
    editTunnel: "编辑隧道",
    renameGroup: "重命名",
    deleteGroup: "删除分组",
    copyGroupName: "复制分组名称",
    newTunnelInGroup: "在此分组内创建",
    confirmDeleteGroup: "确定要删除该分组吗？（分组内的隧道将被保留为未分组）",

    // TunnelOverview Dashboard
    welcome: "欢迎使用 Tunnel Mate",
    welcomeSub: "无需复杂的命令，轻松管理、监控并自动化配置您的安全端口转发。",
    btnImport: "导入配置",
    btnExport: "导出配置",
    activeTunnels: "活动隧道",
    localPorts: "本地监听端口",
    systemHealth: "系统状态",
    healthy: "运行正常",
    alertsCount: "{count} 处异常",
    noActivePorts: "暂无监听端口",
    systemStatusDetailsHealthy: "所有隧道服务正常",
    systemStatusDetailsAlert: "检测到连接断开异常",
    quickStart: "快速上手",
    step1Title: "1. 添加与导入",
    step1Desc: "点击 “+ 隧道” 添加或导入 SSH 配置。",
    step2Title: "2. 连接诊断",
    step2Desc: "运行诊断，测试连接通路与密钥配置。",
    step3Title: "3. 启动隧道",
    step3Desc: "一键启动连接，并实时查看连接日志。",
    recentGlobalEvents: "最近全局事件",

    // Selected Tunnel Overview
    connectionParams: "连接参数",
    jumpHostMap: "跳板机路由映射",
    bastion: "跳板机",
    target: "目标主机",
    stats: "连接统计",
    uptime: "运行时间",
    host: "主机地址",
    port: "端口",
    user: "用户名",
    privateKey: "私钥路径",
    localBind: "本地绑定",
    remoteDest: "远程目标",
    startTunnel: "启动隧道",
    stopTunnel: "停止隧道",
    diagnostics: "连接诊断",
    tabOverview: "信息概览",
    tabLogs: "实时日志",
    tabEvents: "历史事件",
    tabSettings: "设置",
    noEvents: "该隧道暂无历史事件",
    clearLogs: "清除日志",
    copyLogs: "复制日志",
    exportLogs: "导出日志",
    logsSearchPlaceholder: "过滤日志内容...",
    eventSearchPlaceholder: "过滤审计事件...",
    sshConfigImport: "导入 SSH 配置",
    sshConfigImportHint: "选择一个 Host 条目，自动填充当前隧道的 SSH 连接信息。",
    sshConfigImportTitle: "导入 SSH Host",
    sshConfigNoHostName: "未配置 HostName",
    forwardingType: "转发类型",

    // TunnelForm
    titleCreate: "创建 SSH 隧道",
    titleEdit: "编辑隧道: {name}",
    tabGeneral: "常规",
    tabSsh: "SSH 连接",
    tabForward: "端口转发",
    tabJump: "跳板配置",
    tabBehavior: "重连行为",
    tunnelName: "隧道名称",
    descriptionOpt: "描述（可选）",
    groupEnv: "分组 / 环境",
    noGroup: "无分组（未归类）",
    localForward: "本地端口转发 (L)",
    remoteForward: "远程端口转发 (R)",
    socks5Forward: "SOCKS5 动态代理 (D)",
    sshHost: "SSH 主机地址",
    sshPort: "SSH 端口",
    sshUser: "SSH 用户名",
    privateKeyPath: "私钥路径（可选）",
    privateKeyDesc: "留空将使用当前活跃的 ssh-agent 或默认密钥路径（如 `~/.ssh/id_rsa`, `~/.ssh/id_ed25519`）。",
    localAddress: "本地监听地址",
    localPort: "本地端口",
    remoteDestHost: "远程目标主机",
    localDestHost: "本地目标主机",
    destPort: "目标端口",
    enableJumpHost: "启用 SSH 跳板机 (ProxyJump)",
    jumpHostDesc: "通过中间的堡垒机/跳板机服务器转发与目标主机的连接。",
    selectBastion: "选择已有隧道作为跳板",
    startWithApp: "随应用自动启动",
    autoReconnect: "连接断开时自动重连",
    maxRetries: "最大重连尝试次数",
    retryInterval: "重连时间间隔 (秒)",
    btnDelete: "删除隧道",
    btnDeleteConfirm: "您确定要删除该隧道吗？",
    btnSaveTunnel: "保存隧道",
    errNameRequired: "名称不能为空",
    errHostRequired: "SSH 主机地址不能为空",
    errUserRequired: "SSH 用户名不能为空",
    errLocalPortRequired: "本地端口不能为空",
    errDestHostRequired: "目标主机地址不能为空",
    errDestPortRequired: "目标端口不能为空",
    errJumpHostRequired: "必须指定跳板机隧道",
    errInvalidPort: "端口号必须是 1 到 65535 之间的整数",
    errInvalidRetries: "重连次数必须在 0 到 100 之间",
    errInvalidInterval: "重连间隔必须在 1 到 3600 秒之间",
    descLocalTitle: "本地端口转发 (L)",
    descLocalBody: "通过 SSH 服务器将你本地计算机的某个端口流量转发到远程目标主机的指定端口。",
    descRemoteTitle: "远程端口转发 (R)",
    descRemoteBody: "通过 SSH 连接将远程服务器上的某个端口流量转发到你本地网络中的某个目标主机和端口。",
    descSocksTitle: "SOCKS5 动态代理 (D)",
    descSocksBody: "在本地启动一个 SOCKS5 代理服务器，发送到该端口的所有流量都会通过 SSH 隧道动态路由至目标网络。",

    // DiagnosticsModal
    titleConnectionTest: "连接测试",
    subTitleConnectionTest: "正在诊断连接 \"{name}\"",
    diagnosticExecutionError: "诊断执行错误",
    runningChecks: "正在执行连接性检查...",
    passphraseRequired: "需要密钥密码",
    passphraseDesc: "此 SSH 私钥已被密码加密。请输入密码以继续连接。",
    passphrasePlaceholder: "请输入私钥的密码...",
    btnVerifyKey: "验证密码",
    btnClose: "关闭",
    btnRetryTest: "重新测试",
    btnChecking: "检查中...",

    // EventsViewer
    searchEventsLog: "搜索事件日志...",
    btnRefreshLogs: "刷新日志",
    noEventsFound: "未找到任何事件",
    ev_created: "已创建",
    ev_updated: "已更新",
    ev_started: "已启动",
    ev_stopped: "已停止",
    ev_reconnected: "已重连",
    ev_failed: "运行失败",
    ev_deleted: "已删除",

    // LogsViewer
    terminalWaiting: "终端已激活。正在等待隧道连接日志...",
    noMatchingLogs: "没有匹配的日志",

    // SettingsModal
    globalSettings: "全局设置",
    appBehavior: "常规",
    networkTimeouts: "SSH 与网络",
    dataManagement: "数据与文件管理",
    launchOnStartup: "开机自启动",
    launchOnStartupDesc: "在计算机启动时自动运行 Tunnel Mate。",
    closeToTray: "关闭窗口时隐藏至系统托盘",
    closeToTrayDesc: "关闭主窗口时应用不在后台退出，而是保持在系统托盘运行。",
    startMinimized: "启动时最小化到系统托盘",
    startMinimizedDesc: "应用启动时静默最小化到系统托盘，不弹出主窗口。",
    keepAlive: "SSH 心跳检测间隔 (秒)",
    keepAliveDesc: "定时发送心跳包防止连接因网络静默超时断开。设置为 0 禁用。",
    connTimeout: "SSH 连接超时时间 (秒)",
    connTimeoutDesc: "建立 TCP 与 SSH 连接时的最长等待时间。",
    backupRestore: "备份与恢复应用配置",
    backupRestoreDesc: "导出全量的隧道配置、分组以及全局设置到 JSON 备份文件中，或从备份中导入恢复。",
    clearEvents: "清空历史审计日志",
    clearEventsDesc: "永久删除当前保存在本地的历史连接性与配置变更的审计记录。",
    btnClearEvents: "清空记录",
    settingsSaved: "全局设置已成功保存",
    configImported: "配置文件导入成功",
  }
};

type TranslationKey = keyof typeof translations.en;

interface LanguageContextType {
  language: Language;
  setLanguage: (lang: Language) => void;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
}

const LanguageContext = createContext<LanguageContextType | undefined>(undefined);

export function LanguageProvider({ children }: { children: React.ReactNode }) {
  const [language, setLanguageState] = useState<Language>(() => {
    const saved = localStorage.getItem("tunnelmate_language");
    if (saved === "zh" || saved === "en") return saved;
    // Autodetect browser locale
    const locale = navigator.language.toLowerCase();
    return locale.startsWith("zh") ? "zh" : "en";
  });

  const setLanguage = (lang: Language) => {
    setLanguageState(lang);
    localStorage.setItem("tunnelmate_language", lang);
  };

  const t = (key: TranslationKey, params?: Record<string, string | number>): string => {
    const dict = translations[language] || translations.en;
    let text = dict[key] || translations.en[key] || String(key);
    
    if (params) {
      Object.entries(params).forEach(([k, v]) => {
        text = text.replace(new RegExp(`{${k}}`, "g"), String(v));
      });
    }
    return text;
  };

  return React.createElement(
    LanguageContext.Provider,
    { value: { language, setLanguage, t } },
    children
  );
}

export function useLanguage() {
  const context = useContext(LanguageContext);
  if (!context) {
    throw new Error("useLanguage must be used within a LanguageProvider");
  }
  return context;
}
