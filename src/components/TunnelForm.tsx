import { useState, useEffect } from "react";
import { X, Download, ChevronDown, Info, Server, Shuffle, GitBranch, RotateCcw, FolderOpen } from "lucide-react";
import { Group, Tunnel, ForwardKind, SshHostConfig, importSshConfig, selectPrivateKeyFile } from "../lib/tauri";
import { useLanguage } from "../lib/i18n";
import { CompositionInput, CompositionTextarea } from "./CompositionInput";
import { useEscapeLayer } from "../hooks/useEscapeLayer";

interface TunnelFormProps {
  tunnel: Tunnel | null; // null means creating new tunnel
  groups: Group[];
  tunnels: Tunnel[]; // for validation and jump hosts selection
  onSave: (tunnel: Tunnel) => void;
  onCancel: () => void;
  defaultGroupId?: string;
}

type TabType = "general" | "ssh" | "forward" | "jump" | "behavior";

export default function TunnelForm({
  tunnel,
  groups,
  tunnels,
  onSave,
  onCancel,
  defaultGroupId,
}: TunnelFormProps) {
  const { t } = useLanguage();
  const [activeTab, setActiveTab] = useState<TabType>("general");
  const [sshConfigs, setSshConfigs] = useState<SshHostConfig[]>([]);
  const [showSshConfigList, setShowSshConfigList] = useState(false);

  // Form states
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [groupId, setGroupId] = useState(defaultGroupId || "");
  const [forwardKind, setForwardKind] = useState<ForwardKind>("local");

  const [sshHost, setSshHost] = useState("");
  const [sshPort, setSshPort] = useState<number | "">(22);
  const [sshUser, setSshUser] = useState("");
  const [sshIdentityFile, setSshIdentityFile] = useState("");
  const [sshPassword, setSshPassword] = useState("");

  const [jumpHostEnabled, setJumpHostEnabled] = useState(false);
  const [jumpHostId, setJumpHostId] = useState("");
  const [jumpHost, setJumpHost] = useState("");
  const [jumpPort, setJumpPort] = useState<number | "">(22);
  const [jumpUser, setJumpUser] = useState("");
  const [jumpIdentityFile, setJumpIdentityFile] = useState("");
  const [jumpPassword, setJumpPassword] = useState("");

  const [listenHost, setListenHost] = useState("127.0.0.1");
  const [listenPort, setListenPort] = useState<number | "">("");
  const [targetHost, setTargetHost] = useState("");
  const [targetPort, setTargetPort] = useState<number | "">("");

  const [startWithApp, setStartWithApp] = useState(false);
  const [autoReconnect, setAutoReconnect] = useState(true);
  const [retryCount, setRetryCount] = useState<number | "">(3);
  const [retryInterval, setRetryInterval] = useState<number | "">(5);

  const [errors, setErrors] = useState<Record<string, string>>({});

  // Load SSH configs on mount for auto-fill helper
  useEffect(() => {
    importSshConfig()
      .then(configs => setSshConfigs(configs))
      .catch(e => console.error("Failed to load ssh config:", e));
  }, []);

  useEscapeLayer(true, onCancel);
  useEscapeLayer(showSshConfigList, () => setShowSshConfigList(false));

  // Initialize form if editing
  useEffect(() => {
    if (tunnel) {
      setName(tunnel.name);
      setDescription(tunnel.description || "");
      setGroupId(tunnel.groupId || "");
      setForwardKind(tunnel.forward.kind);
      setSshHost(tunnel.sshHost);
      setSshPort(tunnel.sshPort);
      setSshUser(tunnel.sshUser);
      setSshIdentityFile(tunnel.sshIdentityFile || "");
      setSshPassword(tunnel.sshPassword || "");
      setJumpHostEnabled(tunnel.jumpHostEnabled);
      setJumpHostId(tunnel.jumpHostId || "");
      setJumpHost(tunnel.jumpHost || "");
      setJumpPort(tunnel.jumpPort || 22);
      setJumpUser(tunnel.jumpUser || "");
      setJumpIdentityFile(tunnel.jumpIdentityFile || "");
      setJumpPassword(tunnel.jumpPassword || "");
      setListenHost(tunnel.forward.listen.host);
      setListenPort(tunnel.forward.listen.port);
      if (tunnel.forward.kind === "socks5") {
        setTargetHost("");
        setTargetPort("");
      } else {
        setTargetHost(tunnel.forward.target.host);
        setTargetPort(tunnel.forward.target.port);
      }
      setStartWithApp(tunnel.startWithApp);
      setAutoReconnect(tunnel.autoReconnect);
      setRetryCount(tunnel.retryCount);
      setRetryInterval(tunnel.retryInterval);
    } else {
      // Default local port generator
      setListenPort(Math.floor(Math.random() * 10000) + 10000);
    }
  }, [tunnel]);

  // Clear errors reactively as user types/edits fields
  useEffect(() => {
    setErrors(prev => {
      const next = { ...prev };
      let changed = false;

      if (next.name && name.trim()) {
        delete next.name;
        changed = true;
      }
      if (next.sshHost && sshHost.trim()) {
        delete next.sshHost;
        changed = true;
      }
      if (next.sshUser && sshUser.trim()) {
        delete next.sshUser;
        changed = true;
      }
      if (next.sshPort && typeof sshPort === "number" && sshPort >= 1 && sshPort <= 65535 && Number.isInteger(sshPort)) {
        delete next.sshPort;
        changed = true;
      }
      if (next.listenPort && typeof listenPort === "number" && listenPort >= 1 && listenPort <= 65535 && Number.isInteger(listenPort)) {
        delete next.listenPort;
        changed = true;
      }
      if (next.targetHost && targetHost.trim()) {
        delete next.targetHost;
        changed = true;
      }
      if (next.targetPort && typeof targetPort === "number" && targetPort >= 1 && targetPort <= 65535 && Number.isInteger(targetPort)) {
        delete next.targetPort;
        changed = true;
      }
      if (next.jumpHost && (!jumpHostEnabled || jumpHostId || jumpHost)) {
        delete next.jumpHost;
        changed = true;
      }
      if (next.jumpPort && (!jumpHostEnabled || (typeof jumpPort === "number" && jumpPort >= 1 && jumpPort <= 65535 && Number.isInteger(jumpPort)))) {
        delete next.jumpPort;
        changed = true;
      }
      if (next.retryCount && (!autoReconnect || (typeof retryCount === "number" && retryCount >= 0 && retryCount <= 100 && Number.isInteger(retryCount)))) {
        delete next.retryCount;
        changed = true;
      }
      if (next.retryInterval && (!autoReconnect || (typeof retryInterval === "number" && retryInterval >= 1 && retryInterval <= 3600 && Number.isInteger(retryInterval)))) {
        delete next.retryInterval;
        changed = true;
      }

      return changed ? next : prev;
    });
  }, [name, sshHost, sshUser, sshPort, listenPort, targetHost, targetPort, jumpHostId, jumpHost, jumpPort, jumpHostEnabled, autoReconnect, retryCount, retryInterval]);

  const handleAutoFill = (cfg: SshHostConfig) => {
    if (!name) setName(cfg.host);
    setSshHost(cfg.hostName || cfg.host);
    setSshUser(cfg.user || "");
    setSshPort(cfg.port || 22);
    setSshIdentityFile(cfg.identityFile || "");
    setSshPassword("");
    setShowSshConfigList(false);
  };

  const handleSelectPrivateKeyFile = async (setPath: (path: string) => void) => {
    try {
      const path = await selectPrivateKeyFile();
      if (path) {
        setPath(path);
      }
    } catch (error) {
      console.error("Failed to select private key file:", error);
    }
  };

  const getTabForErrorKey = (key: string): TabType => {
    if (key === "name") return "general";
    if (key === "sshHost" || key === "sshUser" || key === "sshPort") return "ssh";
    if (key === "listenPort" || key === "targetHost" || key === "targetPort") return "forward";
    if (key === "jumpHost" || key === "jumpPort" || key === "jumpUser") return "jump";
    if (key === "retryCount" || key === "retryInterval") return "behavior";
    return "general";
  };

  const validate = (): boolean => {
    const errs: Record<string, string> = {};

    // 1. General Tab
    if (!name.trim()) errs.name = t("errNameRequired");

    // 2. SSH Tab
    if (!sshHost.trim()) errs.sshHost = t("errHostRequired");
    if (!sshUser.trim()) errs.sshUser = t("errUserRequired");
    const sPort = Number(sshPort);
    if (isNaN(sPort) || sPort < 1 || sPort > 65535 || !Number.isInteger(sPort)) {
      errs.sshPort = t("errInvalidPort");
    }

    // 3. Port Forwarding Tab
    const lPort = Number(listenPort);
    if (!listenPort) {
      errs.listenPort = t("errListenPortRequired");
    } else if (isNaN(lPort) || lPort < 1 || lPort > 65535 || !Number.isInteger(lPort)) {
      errs.listenPort = t("errInvalidPort");
    }

    if (forwardKind === "local" || forwardKind === "remote") {
      if (!targetHost.trim()) {
        errs.targetHost = t("errTargetHostRequired");
      }
      const rPort = Number(targetPort);
      if (!targetPort) {
        errs.targetPort = t("errTargetPortRequired");
      } else if (isNaN(rPort) || rPort < 1 || rPort > 65535 || !Number.isInteger(rPort)) {
        errs.targetPort = t("errInvalidPort");
      }
    }

    // 4. Jump Host Tab
    if (jumpHostEnabled) {
      if (!jumpHostId && !jumpHost) {
        errs.jumpHost = t("errJumpHostRequired");
      }
      if (!jumpHostId && !jumpUser.trim()) {
        errs.jumpUser = t("errUserRequired");
      }
      const jPort = Number(jumpPort);
      if (!jumpHostId && (isNaN(jPort) || jPort < 1 || jPort > 65535 || !Number.isInteger(jPort))) {
        errs.jumpPort = t("errInvalidPort");
      }
    }

    // 5. Behavior Tab
    if (autoReconnect) {
      const rCount = Number(retryCount);
      if (isNaN(rCount) || rCount < 0 || rCount > 100 || !Number.isInteger(rCount)) {
        errs.retryCount = t("errInvalidRetries");
      }
      const rInterval = Number(retryInterval);
      if (isNaN(rInterval) || rInterval < 1 || rInterval > 3600 || !Number.isInteger(rInterval)) {
        errs.retryInterval = t("errInvalidInterval");
      }
    }

    setErrors(errs);

    const errKeys = Object.keys(errs);
    if (errKeys.length > 0) {
      const firstErrorKey = errKeys[0];
      const targetTab = getTabForErrorKey(firstErrorKey);
      setActiveTab(targetTab);
      return false;
    }

    return true;
  };

  const handleSave = () => {
    if (!validate()) return;

    const normalizedListenHost = listenHost.trim() || "127.0.0.1";
    const normalizedTargetHost = targetHost.trim();
    const forward = forwardKind === "socks5"
      ? {
          kind: "socks5" as const,
          listen: { host: normalizedListenHost, port: Number(listenPort) },
        }
      : {
          kind: forwardKind,
          listen: { host: normalizedListenHost, port: Number(listenPort) },
          target: { host: normalizedTargetHost, port: Number(targetPort) },
        };

    const data: Tunnel = {
      id: tunnel?.id || crypto.randomUUID(),
      name: name.trim(),
      description: description.trim() || undefined,
      groupId: groupId || undefined,
      sshHost: sshHost.trim(),
      sshPort: Number(sshPort),
      sshUser: sshUser.trim(),
      sshIdentityFile: sshIdentityFile.trim() || undefined,
      sshPassword: sshPassword || undefined,
      jumpHostEnabled,
      jumpHostId: jumpHostEnabled && jumpHostId ? jumpHostId : undefined,
      jumpHost: jumpHostEnabled && !jumpHostId ? jumpHost.trim() : undefined,
      jumpPort: jumpHostEnabled && !jumpHostId ? Number(jumpPort) : undefined,
      jumpUser: jumpHostEnabled && !jumpHostId ? jumpUser.trim() : undefined,
      jumpIdentityFile: jumpHostEnabled && !jumpHostId && jumpIdentityFile.trim() ? jumpIdentityFile.trim() : undefined,
      jumpPassword: jumpHostEnabled && !jumpHostId && jumpPassword ? jumpPassword : undefined,
      forward,
      startWithApp,
      autoReconnect,
      retryCount: Number(retryCount),
      retryInterval: Number(retryInterval),
    };

    onSave(data);
  };

  const tabs: { id: TabType; label: string }[] = [
    { id: "general", label: t("tabGeneral") },
    { id: "ssh", label: t("tabSsh") },
    { id: "forward", label: t("tabForward") },
    { id: "jump", label: t("tabJump") },
    { id: "behavior", label: t("tabBehavior") },
  ];
  const tabIcons = {
    general: Info,
    ssh: Server,
    forward: Shuffle,
    jump: GitBranch,
    behavior: RotateCcw,
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4">
      <div className="w-full max-w-4xl h-[85vh] bg-white dark:bg-neutral-900 rounded-xl shadow-2xl border border-gray-200 dark:border-neutral-800 flex flex-col overflow-hidden">
        {/* Form Header */}
        <div className="p-4 border-b border-gray-200 dark:border-neutral-800 flex items-center justify-between">
          <h2 className="text-base font-semibold text-gray-900 dark:text-white">
            {tunnel ? t("titleEdit", { name: tunnel.name }) : t("titleCreate")}
          </h2>
          <button 
            onClick={onCancel}
            className="p-1 rounded-md hover:bg-gray-100 dark:hover:bg-neutral-800 text-gray-500 hover:text-gray-900 dark:hover:text-white transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        <div className="flex flex-1 min-h-0 overflow-hidden">
          {/* Left Tabs Sidebar */}
          <div className="w-44 border-r border-gray-200 dark:border-neutral-800 bg-gray-50/30 dark:bg-neutral-950/20 p-3 shrink-0">
            <div className="space-y-1">
              {tabs.map(tab => {
                const hasError = Object.keys(errors).some(key => getTabForErrorKey(key) === tab.id);
                const Icon = tabIcons[tab.id];
                return (
                  <button
                    key={tab.id}
                    onClick={() => setActiveTab(tab.id)}
                    className={`w-full flex items-center gap-2 px-3 py-2 rounded-lg text-xs font-medium transition cursor-pointer ${
                      activeTab === tab.id
                        ? "bg-indigo-50 dark:bg-indigo-950/30 text-indigo-600 dark:text-indigo-400 font-semibold"
                        : "text-gray-600 dark:text-neutral-400 hover:bg-gray-100/80 dark:hover:bg-neutral-800/60"
                    }`}
                  >
                    <Icon className="w-4 h-4 shrink-0" />
                    <span className="truncate">{tab.label}</span>
                    {hasError && (
                      <span className="ml-auto w-1.5 h-1.5 rounded-full bg-red-500 animate-pulse shrink-0" />
                    )}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Right Scrollable Panel */}
          <div className="flex-1 overflow-y-auto p-5 bg-white dark:bg-neutral-900">
          
          {/* GENERAL TAB */}
          {activeTab === "general" && (
            <div className="space-y-4">
              <div>
                <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("tunnelName")}</label>
                <CompositionInput
                  type="text"
                  placeholder="e.g., Production DB"
                  value={name}
                  onValueChange={setName}
                  className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                />
                {errors.name && <p className="text-[10px] text-red-500 mt-1">{errors.name}</p>}
              </div>

              <div>
                <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("descriptionOpt")}</label>
                <CompositionTextarea
                  placeholder="Notes about this tunnel connection..."
                  value={description}
                  onValueChange={setDescription}
                  rows={2}
                  className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                />
              </div>

              <div>
                <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("groupEnv")}</label>
                <div className="relative">
                  <select
                    value={groupId}
                    onChange={(e) => setGroupId(e.target.value)}
                    className="w-full px-3 py-2 pr-8 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500 appearance-none text-gray-900 dark:text-white cursor-pointer"
                  >
                    <option value="">{t("noGroup")}</option>
                    {groups.map(g => (
                      <option key={g.id} value={g.id}>{g.name}</option>
                    ))}
                  </select>
                  <div className="absolute inset-y-0 right-2.5 flex items-center pointer-events-none">
                    <ChevronDown className="w-3.5 h-3.5 text-gray-400 dark:text-neutral-500" />
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* SSH TAB */}
          {activeTab === "ssh" && (
            <div className="space-y-4">
              {sshConfigs.length > 0 && (
                <div className="flex items-center justify-between gap-3 rounded-lg border border-indigo-100 dark:border-indigo-950/60 bg-indigo-50/50 dark:bg-indigo-950/20 px-3 py-2">
                  <div className="min-w-0">
                    <div className="text-xs font-semibold text-gray-900 dark:text-white">{t("sshConfigImport")}</div>
                    <div className="text-[10px] text-gray-500 dark:text-neutral-400 truncate">{t("sshConfigImportHint")}</div>
                  </div>
                  <button
                    type="button"
                    onClick={() => setShowSshConfigList(true)}
                    className="shrink-0 px-3 py-1.5 bg-indigo-600 hover:bg-indigo-700 text-white rounded-md text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer shadow-sm shadow-indigo-600/10"
                  >
                    <Download className="w-3.5 h-3.5" />
                    {t("sshConfigImport")}
                  </button>
                </div>
              )}

              <div className="grid grid-cols-4 gap-3">
                <div className="col-span-3">
                  <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("sshHost")}</label>
                  <CompositionInput
                    type="text"
                    placeholder="ssh.example.com or 192.168.1.1"
                    value={sshHost}
                    onValueChange={setSshHost}
                    className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                  />
                  {errors.sshHost && <p className="text-[10px] text-red-500 mt-1">{errors.sshHost}</p>}
                </div>
                <div>
                  <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("sshPort")}</label>
                  <input
                    type="number"
                    value={sshPort}
                    onChange={(e) => setSshPort(e.target.value === "" ? "" : Number(e.target.value))}
                    className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                  />
                  {errors.sshPort && <p className="text-[10px] text-red-500 mt-1">{errors.sshPort}</p>}
                </div>
              </div>

              <div>
                <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("sshUser")}</label>
                <CompositionInput
                  type="text"
                  placeholder="e.g., root, ubuntu"
                  value={sshUser}
                  onValueChange={setSshUser}
                  className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                />
                {errors.sshUser && <p className="text-[10px] text-red-500 mt-1">{errors.sshUser}</p>}
              </div>

              <div>
                <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("sshPassword")}</label>
                <CompositionInput
                  type="password"
                  placeholder="Optional password authentication"
                  value={sshPassword}
                  onValueChange={setSshPassword}
                  className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                />
                <p className="text-[10px] text-gray-400 dark:text-neutral-500 mt-1">
                  {t("sshPasswordDesc")}
                </p>
              </div>

              <div>
                <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("privateKeyPath")}</label>
                <div className="relative">
                  <CompositionInput
                    type="text"
                    placeholder="e.g., /Users/username/.ssh/id_rsa"
                    value={sshIdentityFile}
                    onValueChange={setSshIdentityFile}
                    className="w-full pl-3 pr-10 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                  />
                  <button
                    type="button"
                    onClick={() => handleSelectPrivateKeyFile(setSshIdentityFile)}
                    title={t("selectPrivateKeyFile")}
                    className="absolute inset-y-1 right-1 w-8 rounded-md flex items-center justify-center text-gray-400 hover:text-indigo-600 hover:bg-gray-100 dark:text-neutral-500 dark:hover:text-indigo-400 dark:hover:bg-neutral-800 transition cursor-pointer"
                  >
                    <FolderOpen className="w-3.5 h-3.5" />
                  </button>
                </div>
                <p className="text-[10px] text-gray-400 dark:text-neutral-500 mt-1">
                  {t("privateKeyDesc")}
                </p>
              </div>
            </div>
          )}

          {/* FORWARD TAB */}
          {activeTab === "forward" && (
            <div className="space-y-4">
              <div>
                <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("forwardingType")}</label>
                <div className="relative">
                  <select
                    value={forwardKind}
                    onChange={(e) => setForwardKind(e.target.value as ForwardKind)}
                    className="w-full px-3 py-2 pr-8 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500 appearance-none text-gray-900 dark:text-white cursor-pointer"
                  >
                    <option value="local">{t("localForward")}</option>
                    <option value="remote">{t("remoteForward")}</option>
                    <option value="socks5">{t("socks5Forward")}</option>
                  </select>
                  <div className="absolute inset-y-0 right-2.5 flex items-center pointer-events-none">
                    <ChevronDown className="w-3.5 h-3.5 text-gray-400 dark:text-neutral-500" />
                  </div>
                </div>
              </div>

              {/* ── LOCAL PORT FORWARDING ── */}
              {forwardKind === "local" && (
                <div className="space-y-3">
                  {/* Visual flow */}
                  <div className="flex items-center gap-1.5 text-[10px] font-medium select-none px-0.5">
                    <span className="px-2 py-0.5 rounded bg-indigo-50 dark:bg-indigo-950/40 text-indigo-600 dark:text-indigo-400 font-semibold border border-indigo-100 dark:border-indigo-900/40">本机</span>
                    <span className="text-gray-400">───▶</span>
                    <span className="px-2 py-0.5 rounded bg-gray-100 dark:bg-neutral-800 text-gray-500 dark:text-neutral-400 border border-gray-200 dark:border-neutral-700">SSH 服务器</span>
                    <span className="text-gray-400">───▶</span>
                    <span className="px-2 py-0.5 rounded bg-emerald-50 dark:bg-emerald-950/40 text-emerald-700 dark:text-emerald-400 font-semibold border border-emerald-100 dark:border-emerald-900/40">目标服务</span>
                  </div>

                  {/* 本机监听 */}
                  <div className="p-3 rounded-lg border border-indigo-100 dark:border-indigo-900/40 bg-indigo-50/30 dark:bg-indigo-950/10 space-y-2.5">
                    <div className="text-[10px] font-bold text-indigo-600 dark:text-indigo-400 uppercase tracking-wider">① 本机监听</div>
                    <div className="grid grid-cols-2 gap-3">
                      <div>
                        <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">监听地址</label>
                        <CompositionInput type="text" value={listenHost} onValueChange={setListenHost}
                          className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500" />
                        <p className="text-[10px] text-gray-400 dark:text-neutral-500 mt-0.5">通常保持 127.0.0.1 不变</p>
                      </div>
                      <div>
                        <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">本机端口 <span className="text-red-400">*</span></label>
                        <input type="number" placeholder="如 13306" value={listenPort || ""}
                          onChange={(e) => setListenPort(e.target.value === "" ? "" : Number(e.target.value))}
                          className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500" />
                        <p className="text-[10px] text-gray-400 dark:text-neutral-500 mt-0.5">你在本机访问的端口号</p>
                        {errors.listenPort && <p className="text-[10px] text-red-500 mt-1">{errors.listenPort}</p>}
                      </div>
                    </div>
                  </div>

                  {/* 目标服务 */}
                  <div className="p-3 rounded-lg border border-emerald-100 dark:border-emerald-900/40 bg-emerald-50/30 dark:bg-emerald-950/10 space-y-2.5">
                    <div className="text-[10px] font-bold text-emerald-700 dark:text-emerald-400 uppercase tracking-wider">② 目标服务（从 SSH 服务器视角填写）</div>
                    <div className="grid grid-cols-2 gap-3">
                      <div>
                        <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">目标主机 <span className="text-red-400">*</span></label>
                        <CompositionInput type="text" placeholder="如 localhost 或 db.internal" value={targetHost}
                          onValueChange={setTargetHost}
                          className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500" />
                        <p className="text-[10px] text-gray-400 dark:text-neutral-500 mt-0.5">SSH 服务器能访问到的主机</p>
                        {errors.targetHost && <p className="text-[10px] text-red-500 mt-1">{errors.targetHost}</p>}
                      </div>
                      <div>
                        <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">目标端口 <span className="text-red-400">*</span></label>
                        <input type="number" placeholder="如 3306" value={targetPort || ""}
                          onChange={(e) => setTargetPort(e.target.value === "" ? "" : Number(e.target.value))}
                          className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500" />
                        <p className="text-[10px] text-gray-400 dark:text-neutral-500 mt-0.5">目标服务监听的端口</p>
                        {errors.targetPort && <p className="text-[10px] text-red-500 mt-1">{errors.targetPort}</p>}
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {/* ── REMOTE PORT FORWARDING ── */}
              {forwardKind === "remote" && (
                <div className="space-y-3">
                  <div className="flex items-center gap-1.5 text-[10px] font-medium select-none px-0.5">
                    <span className="px-2 py-0.5 rounded bg-amber-50 dark:bg-amber-950/40 text-amber-700 dark:text-amber-400 font-semibold border border-amber-100 dark:border-amber-900/40">外部访问者</span>
                    <span className="text-gray-400">───▶</span>
                    <span className="px-2 py-0.5 rounded bg-gray-100 dark:bg-neutral-800 text-gray-500 dark:text-neutral-400 border border-gray-200 dark:border-neutral-700">SSH 服务器端口</span>
                    <span className="text-gray-400">───▶</span>
                    <span className="px-2 py-0.5 rounded bg-indigo-50 dark:bg-indigo-950/40 text-indigo-600 dark:text-indigo-400 font-semibold border border-indigo-100 dark:border-indigo-900/40">本机服务</span>
                  </div>

                  {/* SSH 服务器监听 */}
                  <div className="p-3 rounded-lg border border-gray-200 dark:border-neutral-800 bg-gray-50 dark:bg-neutral-950/20 space-y-2.5">
                    <div className="text-[10px] font-bold text-gray-500 dark:text-neutral-400 uppercase tracking-wider">① SSH 服务器对外暴露的端口</div>
                    <div className="grid grid-cols-2 gap-3">
                      <div>
                        <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">服务器绑定地址</label>
                        <CompositionInput type="text" value={listenHost} onValueChange={setListenHost}
                          className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500" />
                        <p className="text-[10px] text-gray-400 dark:text-neutral-500 mt-0.5">SSH 服务器上绑定的地址</p>
                      </div>
                      <div>
                        <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">服务器端口 <span className="text-red-400">*</span></label>
                        <input type="number" placeholder="如 8080" value={listenPort || ""}
                          onChange={(e) => setListenPort(e.target.value === "" ? "" : Number(e.target.value))}
                          className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500" />
                        <p className="text-[10px] text-gray-400 dark:text-neutral-500 mt-0.5">外部通过此端口访问</p>
                        {errors.listenPort && <p className="text-[10px] text-red-500 mt-1">{errors.listenPort}</p>}
                      </div>
                    </div>
                  </div>

                  {/* 本机服务 */}
                  <div className="p-3 rounded-lg border border-indigo-100 dark:border-indigo-900/40 bg-indigo-50/30 dark:bg-indigo-950/10 space-y-2.5">
                    <div className="text-[10px] font-bold text-indigo-600 dark:text-indigo-400 uppercase tracking-wider">② 本机目标服务（流量最终转到这里）</div>
                    <div className="grid grid-cols-2 gap-3">
                      <div>
                        <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">本机地址</label>
                        <CompositionInput type="text" placeholder="如 localhost" value={targetHost}
                          onValueChange={setTargetHost}
                          className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500" />
                        <p className="text-[10px] text-gray-400 dark:text-neutral-500 mt-0.5">你本机上的服务地址</p>
                        {errors.targetHost && <p className="text-[10px] text-red-500 mt-1">{errors.targetHost}</p>}
                      </div>
                      <div>
                        <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">本机端口 <span className="text-red-400">*</span></label>
                        <input type="number" placeholder="如 3000" value={targetPort || ""}
                          onChange={(e) => setTargetPort(e.target.value === "" ? "" : Number(e.target.value))}
                          className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500" />
                        <p className="text-[10px] text-gray-400 dark:text-neutral-500 mt-0.5">你本机服务的端口号</p>
                        {errors.targetPort && <p className="text-[10px] text-red-500 mt-1">{errors.targetPort}</p>}
                      </div>
                    </div>
                  </div>
                </div>
              )}

              {/* ── SOCKS5 PROXY ── */}
              {forwardKind === "socks5" && (
                <div className="space-y-3">
                  <div className="flex items-center gap-1.5 text-[10px] font-medium select-none px-0.5">
                    <span className="px-2 py-0.5 rounded bg-indigo-50 dark:bg-indigo-950/40 text-indigo-600 dark:text-indigo-400 font-semibold border border-indigo-100 dark:border-indigo-900/40">本机代理端口</span>
                    <span className="text-gray-400">───▶</span>
                    <span className="px-2 py-0.5 rounded bg-gray-100 dark:bg-neutral-800 text-gray-500 dark:text-neutral-400 border border-gray-200 dark:border-neutral-700">SSH 服务器</span>
                    <span className="text-gray-400">───▶</span>
                    <span className="px-2 py-0.5 rounded bg-emerald-50 dark:bg-emerald-950/40 text-emerald-700 dark:text-emerald-400 font-semibold border border-emerald-100 dark:border-emerald-900/40">任意目标</span>
                  </div>
                  <div className="p-3 rounded-lg border border-indigo-100 dark:border-indigo-900/40 bg-indigo-50/30 dark:bg-indigo-950/10 space-y-2.5">
                    <div className="text-[10px] font-bold text-indigo-600 dark:text-indigo-400 uppercase tracking-wider">本机 SOCKS5 代理监听</div>
                    <div className="grid grid-cols-2 gap-3">
                      <div>
                        <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">监听地址</label>
                        <CompositionInput type="text" value={listenHost} onValueChange={setListenHost}
                          className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500" />
                        <p className="text-[10px] text-gray-400 dark:text-neutral-500 mt-0.5">通常保持 127.0.0.1 不变</p>
                      </div>
                      <div>
                        <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">代理端口 <span className="text-red-400">*</span></label>
                        <input type="number" placeholder="如 1080" value={listenPort || ""}
                          onChange={(e) => setListenPort(e.target.value === "" ? "" : Number(e.target.value))}
                          className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500" />
                        <p className="text-[10px] text-gray-400 dark:text-neutral-500 mt-0.5">在系统或浏览器中配置此端口为代理</p>
                        {errors.listenPort && <p className="text-[10px] text-red-500 mt-1">{errors.listenPort}</p>}
                      </div>
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}

          {/* JUMP HOST TAB */}
          {activeTab === "jump" && (
            <div className="space-y-4">
              <div className="flex items-center justify-between p-3 bg-gray-50 dark:bg-neutral-950/20 border border-gray-200 dark:border-neutral-800 rounded-lg">
                <div>
                  <span className="font-semibold text-xs text-gray-800 dark:text-neutral-200 block">{t("enableJumpHost")}</span>
                  <span className="text-[10px] text-gray-500 dark:text-neutral-400">{t("jumpHostDesc")}</span>
                </div>
                <button
                  type="button"
                  onClick={() => setJumpHostEnabled(!jumpHostEnabled)}
                  className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-1 focus:ring-indigo-500 ${
                    jumpHostEnabled ? 'bg-indigo-600' : 'bg-gray-200 dark:bg-neutral-800'
                  }`}
                >
                  <span
                    className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out ${
                      jumpHostEnabled ? 'translate-x-4' : 'translate-x-0'
                    }`}
                  />
                </button>
              </div>

              {jumpHostEnabled && (
                <div className="space-y-4 p-4 border border-gray-200 dark:border-neutral-800 rounded-lg bg-gray-50/50 dark:bg-neutral-950/10">
                  <div>
                    <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("selectBastion")}</label>
                    <div className="relative">
                      <select
                        value={jumpHostId}
                        onChange={(e) => {
                          const selected = tunnels.find(t => t.id === e.target.value);
                          if (selected) {
                            setJumpHostId(selected.id);
                            setJumpHost("");
                            setJumpPort(22);
                            setJumpUser("");
                            setJumpIdentityFile("");
                            setJumpPassword("");
                          } else {
                            setJumpHostId("");
                          }
                        }}
                        className="w-full px-3 py-2 pr-8 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500 appearance-none text-gray-900 dark:text-white cursor-pointer"
                      >
                        <option value="">-- Manual Configuration --</option>
                        {tunnels.filter(t => t.id !== tunnel?.id).map(t => (
                          <option key={t.id} value={t.id}>{t.name} ({t.sshHost})</option>
                        ))}
                      </select>
                      <div className="absolute inset-y-0 right-2.5 flex items-center pointer-events-none">
                        <ChevronDown className="w-3.5 h-3.5 text-gray-400 dark:text-neutral-500" />
                      </div>
                    </div>
                  </div>

                  <div className="grid grid-cols-4 gap-3">
                    <div className="col-span-3">
                      <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("bastion")}</label>
                      <CompositionInput
                        type="text"
                        placeholder="bastion.example.com"
                        value={jumpHost}
                        onValueChange={(value) => {
                          setJumpHostId("");
                          setJumpHost(value);
                        }}
                        disabled={Boolean(jumpHostId)}
                        className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                      />
                      {errors.jumpHost && <p className="text-[10px] text-red-500 mt-1">{errors.jumpHost}</p>}
                    </div>
                    <div>
                      <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("port")}</label>
                      <input
                        type="number"
                        value={jumpPort}
                        onChange={(e) => setJumpPort(e.target.value === "" ? "" : Number(e.target.value))}
                        className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                      />
                      {errors.jumpPort && <p className="text-[10px] text-red-500 mt-1">{errors.jumpPort}</p>}
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("user")}</label>
                      <CompositionInput
                        type="text"
                        placeholder="e.g., ec2-user"
                        value={jumpUser}
                        onValueChange={setJumpUser}
                        className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                      />
                      {errors.jumpUser && <p className="text-[10px] text-red-500 mt-1">{errors.jumpUser}</p>}
                    </div>
                    <div>
                      <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("sshPassword")}</label>
                      <CompositionInput
                        type="password"
                        placeholder="Optional password authentication"
                        value={jumpPassword}
                        onValueChange={setJumpPassword}
                        className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                      />
                    </div>
                  </div>

                  <div>
                    <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("privateKey")}</label>
                    <div className="relative">
                      <CompositionInput
                        type="text"
                        placeholder="e.g., /path/to/bastion_key"
                        value={jumpIdentityFile}
                        onValueChange={setJumpIdentityFile}
                        className="w-full px-3 pr-10 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                      />
                      <button
                        type="button"
                        onClick={() => handleSelectPrivateKeyFile(setJumpIdentityFile)}
                        title={t("selectPrivateKeyFile")}
                        className="absolute inset-y-1 right-1 w-8 rounded-md flex items-center justify-center text-gray-400 hover:text-indigo-600 hover:bg-gray-100 dark:text-neutral-500 dark:hover:text-indigo-400 dark:hover:bg-neutral-800 transition cursor-pointer"
                      >
                        <FolderOpen className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}

          {/* BEHAVIOR TAB */}
          {activeTab === "behavior" && (
            <div className="space-y-4">
              <div className="flex items-center justify-between p-3 bg-white dark:bg-neutral-900 border border-gray-200 dark:border-neutral-800 rounded-lg">
                <div>
                  <span className="font-semibold text-xs text-gray-800 dark:text-neutral-200 block">{t("startWithApp")}</span>
                  <span className="text-[10px] text-gray-500 dark:text-neutral-400">Launch this tunnel automatically when the application starts.</span>
                </div>
                <button
                  type="button"
                  onClick={() => setStartWithApp(!startWithApp)}
                  className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-1 focus:ring-indigo-500 ${
                    startWithApp ? 'bg-indigo-600' : 'bg-gray-200 dark:bg-neutral-800'
                  }`}
                >
                  <span
                    className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out ${
                      startWithApp ? 'translate-x-4' : 'translate-x-0'
                    }`}
                  />
                </button>
              </div>

              <div className="flex items-center justify-between p-3 bg-white dark:bg-neutral-900 border border-gray-200 dark:border-neutral-800 rounded-lg">
                <div>
                  <span className="font-semibold text-xs text-gray-800 dark:text-neutral-200 block">{t("autoReconnect")}</span>
                  <span className="text-[10px] text-gray-500 dark:text-neutral-400">Automatically try to re-establish connection if it drops.</span>
                </div>
                <button
                  type="button"
                  onClick={() => setAutoReconnect(!autoReconnect)}
                  className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-1 focus:ring-indigo-500 ${
                    autoReconnect ? 'bg-indigo-600' : 'bg-gray-200 dark:bg-neutral-800'
                  }`}
                >
                  <span
                    className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out ${
                      autoReconnect ? 'translate-x-4' : 'translate-x-0'
                    }`}
                  />
                </button>
              </div>

              {autoReconnect && (
                <div className="grid grid-cols-2 gap-4 p-4 border border-gray-200 dark:border-neutral-800 rounded-lg bg-gray-50/50 dark:bg-neutral-950/10">
                  <div>
                    <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("maxRetries")}</label>
                    <input
                      type="number"
                      value={retryCount}
                      onChange={(e) => setRetryCount(e.target.value === "" ? "" : Number(e.target.value))}
                      className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                    />
                    {errors.retryCount && <p className="text-[10px] text-red-500 mt-1">{errors.retryCount}</p>}
                  </div>
                  <div>
                    <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1">{t("retryInterval")}</label>
                    <input
                      type="number"
                      value={retryInterval}
                      onChange={(e) => setRetryInterval(e.target.value === "" ? "" : Number(e.target.value))}
                      className="w-full px-3 py-2 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500"
                    />
                    {errors.retryInterval && <p className="text-[10px] text-red-500 mt-1">{errors.retryInterval}</p>}
                  </div>
                </div>
              )}
            </div>
          )}

          </div>
        </div>

        {showSshConfigList && (
          <div className="fixed inset-0 z-[60] bg-black/35 backdrop-blur-sm flex items-center justify-center p-4" onClick={() => setShowSshConfigList(false)}>
            <div
              className="w-full max-w-md bg-white dark:bg-neutral-900 border border-gray-200 dark:border-neutral-800 rounded-xl shadow-2xl overflow-hidden"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="p-4 border-b border-gray-200 dark:border-neutral-800 flex items-center justify-between">
                <div>
                  <h3 className="text-sm font-semibold text-gray-900 dark:text-white">{t("sshConfigImportTitle")}</h3>
                  <p className="text-[10px] text-gray-500 dark:text-neutral-400 mt-0.5">{t("sshConfigImportHint")}</p>
                </div>
                <button
                  type="button"
                  onClick={() => setShowSshConfigList(false)}
                  className="p-1 rounded-md hover:bg-gray-100 dark:hover:bg-neutral-800 text-gray-500 hover:text-gray-900 dark:hover:text-white transition cursor-pointer"
                >
                  <X className="w-4 h-4" />
                </button>
              </div>
              <div className="max-h-80 overflow-y-auto p-2 text-xs">
                {sshConfigs.map(cfg => (
                  <button
                    key={cfg.host}
                    type="button"
                    onClick={() => handleAutoFill(cfg)}
                    className="w-full text-left px-3 py-2.5 rounded-md hover:bg-gray-100 dark:hover:bg-neutral-800 flex flex-col transition cursor-pointer"
                  >
                    <span className="font-medium text-gray-900 dark:text-neutral-100">{cfg.host}</span>
                    <span className="text-[10px] text-gray-500 dark:text-neutral-400 truncate w-full">{cfg.hostName || t("sshConfigNoHostName")}</span>
                  </button>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* Form Footer */}
        <div className="p-4 border-t border-gray-200 dark:border-neutral-800 bg-gray-50 dark:bg-neutral-900/50 flex justify-end gap-2 shrink-0">
          <button
            onClick={onCancel}
            className="px-4 py-2 bg-white dark:bg-neutral-800 border border-gray-200 dark:border-neutral-700 hover:bg-gray-100 dark:hover:bg-neutral-700 text-gray-700 dark:text-neutral-200 rounded-md text-xs font-semibold transition cursor-pointer"
          >
            {t("btnCancel")}
          </button>
          <button
            onClick={handleSave}
            className="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-md text-xs font-semibold transition cursor-pointer shadow-sm shadow-indigo-600/10"
          >
            {t("btnSaveTunnel")}
          </button>
        </div>
      </div>
    </div>
  );
}
