import { useState, useEffect } from "react";
import { 
  Play, Square, Terminal, Calendar, Settings, 
  Activity, ShieldCheck, Link, Clock, ArrowRight,
  AlertTriangle
} from "lucide-react";
import { Tunnel, Group, TunnelStatus, LogEvent } from "../lib/tauri";
import LogsViewer from "./LogsViewer";
import EventsViewer from "./EventsViewer";
import TunnelForm from "./TunnelForm";
import { useLanguage } from "../lib/i18n";

interface TunnelOverviewProps {
  tunnel: Tunnel | null;
  groups: Group[];
  tunnels: Tunnel[];
  status: TunnelStatus;
  statuses: Record<string, TunnelStatus>;
  logs: string[];
  events: LogEvent[];
  onStart: () => void;
  onStop: () => void;
  onTestConnection: () => void;
  onClearLogs: () => void;
  onRefreshEvents: () => void;
  onSaveTunnel: (t: Tunnel) => void;
  onDeleteTunnel: (id: string) => void;
}

type PanelTab = "overview" | "logs" | "events" | "settings";

export default function TunnelOverview({
  tunnel,
  groups,
  tunnels,
  status,
  statuses,
  logs,
  events,
  onStart,
  onStop,
  onTestConnection,
  onClearLogs,
  onRefreshEvents,
  onSaveTunnel,
  onDeleteTunnel,
}: TunnelOverviewProps) {
  const { t } = useLanguage();
  const [activeTab, setActiveTab] = useState<PanelTab>("overview");
  const [uptime, setUptime] = useState<number>(0);
  const [isEditing, setIsEditing] = useState(false);

  // Reset tab when tunnel changes
  useEffect(() => {
    setActiveTab("overview");
    setUptime(0);
  }, [tunnel?.id]);

  // Uptime counter
  useEffect(() => {
    let timer: ReturnType<typeof setInterval>;
    if (status === "running") {
      timer = setInterval(() => {
        setUptime(prev => prev + 1);
      }, 1000);
    } else {
      setUptime(0);
    }
    return () => clearInterval(timer);
  }, [status]);

  const formatUptime = (seconds: number) => {
    const hrs = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    return `${hrs.toString().padStart(2, "0")}:${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  };


  const getStatusBadgeStyle = (stat: TunnelStatus) => {
    switch (stat) {
      case "running": return "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-200 dark:border-emerald-800/50";
      case "connecting": return "bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-200 dark:border-amber-800/50 animate-pulse";
      case "reconnecting": return "bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-200 dark:border-blue-800/50 animate-pulse";
      case "failed": return "bg-red-500/10 text-red-600 dark:text-red-400 border-red-200 dark:border-red-800/50";
      default: return "bg-gray-100 dark:bg-neutral-800 text-gray-500 dark:text-neutral-400 border-gray-200 dark:border-neutral-700";
    }
  };

  const getStatusDotStyle = (stat: TunnelStatus) => {
    switch (stat) {
      case "running": return "bg-emerald-500 shadow-[0_0_6px_rgba(16,185,129,0.6)]";
      case "connecting": return "bg-amber-500 animate-pulse";
      case "reconnecting": return "bg-blue-500 animate-pulse";
      case "failed": return "bg-red-500";
      default: return "bg-gray-400 dark:bg-neutral-600";
    }
  };

  const getStatusLabel = (stat: TunnelStatus) => {
    switch (stat) {
      case "running": return "Running";
      case "connecting": return "Connecting";
      case "reconnecting": return "Reconnecting";
      case "failed": return "Failed";
      default: return "Stopped";
    }
  };

  // RENDER 1: EMPTY STATE / GLOBAL DASHBOARD OVERVIEW
  if (!tunnel) {
    const totalCount = tunnels.length;
    const runningTunnels = tunnels.filter(t => statuses[t.id] === "running");
    const runningCount = runningTunnels.length;
    const activePorts = runningTunnels.map(t => t.localPort);
    const failedTunnels = tunnels.filter(t => statuses[t.id] === "failed");
    const failedCount = failedTunnels.length;
    const isHealthy = failedCount === 0;

    return (
      <div className="flex-1 overflow-y-auto bg-gray-50 dark:bg-neutral-950 p-6 space-y-6 select-none">
        {/* Dashboard Header */}
        <div className="pb-4 border-b border-gray-200 dark:border-neutral-800 flex flex-col gap-1">
          <h2 className="text-base font-bold text-gray-900 dark:text-white">{t("dashboardOverview")}</h2>
          <p className="text-xs text-gray-500 dark:text-neutral-400">{t("dashboardDesc")}</p>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-3 gap-4">
          {/* Active Forwards */}
          <div className="p-4 bg-white dark:bg-neutral-900 border border-gray-200 dark:border-neutral-800 rounded-xl flex items-center gap-4">
            <span className="p-3 rounded-lg bg-indigo-50 dark:bg-indigo-950/40 text-indigo-600 dark:text-indigo-400">
              <Activity className="w-6 h-6" />
            </span>
            <div>
              <span className="text-[10px] uppercase font-bold tracking-wider text-gray-400 dark:text-neutral-500 block">{t("activeTunnels")}</span>
              <span className="text-xl font-bold text-gray-800 dark:text-neutral-100">
                {runningCount} <span className="text-xs font-normal text-gray-400 dark:text-neutral-500">/ {totalCount}</span>
              </span>
            </div>
          </div>

          {/* Local Listeners */}
          <div className="p-4 bg-white dark:bg-neutral-900 border border-gray-200 dark:border-neutral-800 rounded-xl flex items-center gap-4 min-w-0">
            <span className="p-3 rounded-lg bg-purple-50 dark:bg-purple-950/40 text-purple-600 dark:text-purple-450">
              <Link className="w-6 h-6" />
            </span>
            <div className="min-w-0 flex-1">
              <span className="text-[10px] uppercase font-bold tracking-wider text-gray-400 dark:text-neutral-500 block">{t("localPorts")}</span>
              <span className="text-xl font-bold text-gray-800 dark:text-neutral-100 block truncate">
                {activePorts.length}
              </span>
              <span className="text-[10px] text-gray-500 dark:text-neutral-400 block truncate font-mono">
                {activePorts.length > 0 ? activePorts.map(p => `:${p}`).join(", ") : t("noActivePorts")}
              </span>
            </div>
          </div>

          {/* System Health */}
          <div className="p-4 bg-white dark:bg-neutral-900 border border-gray-200 dark:border-neutral-800 rounded-xl flex items-center gap-4">
            <span className={`p-3 rounded-lg ${
              isHealthy 
                ? "bg-emerald-50 dark:bg-emerald-950/40 text-emerald-600 dark:text-emerald-400" 
                : "bg-red-50 dark:bg-red-950/40 text-red-600 dark:text-red-400"
            }`}>
              {isHealthy ? <ShieldCheck className="w-6 h-6" /> : <AlertTriangle className="w-6 h-6" />}
            </span>
            <div>
              <span className="text-[10px] uppercase font-bold tracking-wider text-gray-400 dark:text-neutral-500 block">{t("systemHealth")}</span>
              <span className={`text-xl font-bold ${isHealthy ? "text-emerald-600 dark:text-emerald-400" : "text-red-600 dark:text-red-450"}`}>
                {isHealthy ? t("healthy") : t("alertsCount", { count: failedCount })}
              </span>
              <span className="text-[10px] text-gray-450 dark:text-neutral-450 block">
                {isHealthy ? t("systemStatusDetailsHealthy") : t("systemStatusDetailsAlert")}
              </span>
            </div>
          </div>
        </div>

        {/* Getting Started Guide */}
        <div className="p-5 bg-white dark:bg-neutral-900 border border-gray-200 dark:border-neutral-800 rounded-xl">
          <h3 className="font-semibold text-sm text-gray-900 dark:text-white mb-4">{t("quickStart")}</h3>
          <div className="grid grid-cols-3 gap-6 text-xs leading-normal">
            <div className="space-y-1.5">
              <span className="w-6 h-6 rounded-full bg-indigo-50 dark:bg-indigo-950/60 text-indigo-600 dark:text-indigo-400 flex items-center justify-center font-bold">1</span>
              <h4 className="font-semibold text-gray-800 dark:text-neutral-200">{t("step1Title")}</h4>
              <p className="text-gray-500 dark:text-neutral-400">{t("step1Desc")}</p>
            </div>
            <div className="space-y-1.5">
              <span className="w-6 h-6 rounded-full bg-indigo-50 dark:bg-indigo-950/60 text-indigo-600 dark:text-indigo-400 flex items-center justify-center font-bold">2</span>
              <h4 className="font-semibold text-gray-800 dark:text-neutral-200">{t("step2Title")}</h4>
              <p className="text-gray-500 dark:text-neutral-400">{t("step2Desc")}</p>
            </div>
            <div className="space-y-1.5">
              <span className="w-6 h-6 rounded-full bg-indigo-50 dark:bg-indigo-950/60 text-indigo-600 dark:text-indigo-400 flex items-center justify-center font-bold">3</span>
              <h4 className="font-semibold text-gray-800 dark:text-neutral-200">{t("step3Title")}</h4>
              <p className="text-gray-500 dark:text-neutral-400">{t("step3Desc")}</p>
            </div>
          </div>
        </div>

        {/* Global Event Timeline */}
        <div className="space-y-2">
          <span className="text-[10px] font-semibold text-gray-400 dark:text-neutral-500 uppercase tracking-wider block">{t("recentGlobalEvents")}</span>
          <div className="max-h-60 overflow-y-auto">
            <EventsViewer events={events.slice(-5)} onRefresh={onRefreshEvents} />
          </div>
        </div>
      </div>
    );
  }

  // RENDER 2: TUNNEL SELECTED PANEL
  return (
    <div className="flex-1 flex flex-col h-screen overflow-hidden bg-gray-50 dark:bg-neutral-950">
      {/* Top Banner Control */}
      <div className="p-4 bg-white dark:bg-neutral-900 border-b border-gray-200 dark:border-neutral-800 flex items-center justify-between select-none">
        <div>
          <div className="flex items-center gap-2 mb-0.5">
            <h2 className="text-base font-bold text-gray-900 dark:text-white">{tunnel.name}</h2>
            <span className={`px-2 py-0.5 text-[10px] font-semibold tracking-wide border rounded-full flex items-center gap-1.5 ${getStatusBadgeStyle(status)}`}>
              <span className={`w-1.5 h-1.5 rounded-full ${getStatusDotStyle(status)}`} />
              {getStatusLabel(status)}
            </span>
          </div>
          <p className="text-[11px] text-gray-500 dark:text-neutral-400">
            {tunnel.tunnelType === "socks5" ? t("socks5Forward") : (tunnel.tunnelType === "local" ? t("localForward") : t("remoteForward"))} • {tunnel.sshUser}@{tunnel.sshHost}
          </p>
        </div>

        <div className="flex items-center gap-2">
          {status !== "running" && status !== "connecting" && status !== "reconnecting" ? (
            <button
              onClick={onStart}
              className="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-md text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer shadow-sm shadow-indigo-600/10"
            >
              <Play className="w-3.5 h-3.5 fill-current" /> {t("startTunnel")}
            </button>
          ) : (
            <button
              onClick={onStop}
              className="px-4 py-2 bg-red-600 hover:bg-red-750 text-white rounded-md text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer shadow-sm shadow-red-600/10"
            >
              <Square className="w-3.5 h-3.5 fill-current" /> {t("stopTunnel")}
            </button>
          )}

          <button
            onClick={onTestConnection}
            className="px-3 py-2 bg-gray-100 dark:bg-neutral-800 hover:bg-gray-200 dark:hover:bg-neutral-700 text-gray-700 dark:text-neutral-200 rounded-md text-xs font-semibold transition cursor-pointer"
          >
            {t("diagnostics")}
          </button>
        </div>
      </div>

      {/* Navigation Tab Links */}
      <div className="flex bg-white dark:bg-neutral-900 border-b border-gray-200 dark:border-neutral-800 px-4">
        {[
          { id: "overview", label: t("tabOverview"), icon: Activity },
          { id: "logs", label: t("tabLogs"), icon: Terminal },
          { id: "events", label: t("tabEvents"), icon: Calendar },
          { id: "settings", label: t("tabSettings"), icon: Settings },
        ].map(tab => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id as PanelTab)}
            className={`px-4 py-3 text-xs font-medium border-b-2 flex items-center gap-1.5 transition ${
              activeTab === tab.id
                ? "border-indigo-600 text-indigo-600 dark:text-indigo-400 dark:border-indigo-400"
                : "border-transparent text-gray-500 hover:text-gray-900 dark:hover:text-white"
            }`}
          >
            <tab.icon className="w-3.5 h-3.5" />
            <span>{tab.label}</span>
          </button>
        ))}
      </div>

      {/* Selected Tab Content Panels */}
      <div className="flex-1 overflow-y-auto p-6">
        
        {/* OVERVIEW PANEL */}
        {activeTab === "overview" && (
          <div className="space-y-6 max-w-3xl">
            {/* Status Hero Card */}
            <div className="grid grid-cols-2 gap-4">
              <div className="p-4 bg-white dark:bg-neutral-900 border border-gray-200 dark:border-neutral-800 rounded-xl">
                <span className="text-[10px] uppercase font-bold tracking-wider text-gray-400 dark:text-neutral-500 block mb-1">{t("localBind")}</span>
                <div className="flex items-center gap-2 text-xs font-semibold text-gray-800 dark:text-neutral-200">
                  <span>localhost:{tunnel.localPort}</span>
                  <ArrowRight className="w-3.5 h-3.5 text-gray-400" />
                  {tunnel.tunnelType === "socks5" ? (
                    <span className="text-indigo-500">{t("socks5Forward")}</span>
                  ) : (
                    <span>{tunnel.remoteHost}:{tunnel.remotePort}</span>
                  )}
                </div>
              </div>

              <div className="p-4 bg-white dark:bg-neutral-900 border border-gray-200 dark:border-neutral-800 rounded-xl">
                <span className="text-[10px] uppercase font-bold tracking-wider text-gray-400 dark:text-neutral-500 block mb-1">{t("uptime")}</span>
                <div className="flex items-center gap-2 text-xs font-semibold text-gray-800 dark:text-neutral-200">
                  <Clock className="w-4 h-4 text-gray-400" />
                  <span>{status === "running" ? formatUptime(uptime) : "Stopped"}</span>
                </div>
              </div>
            </div>

            {/* Jump Host pipeline representation */}
            {tunnel.jumpHostEnabled && (
              <div className="p-5 bg-white dark:bg-neutral-900 border border-gray-200 dark:border-neutral-800 rounded-xl">
                <span className="text-[10px] font-semibold text-gray-400 dark:text-neutral-500 uppercase tracking-wider block mb-3">{t("jumpHostMap")}</span>
                <div className="flex items-center justify-between text-xs text-center relative max-w-md mx-auto">
                  <div className="bg-neutral-100 dark:bg-neutral-800 border border-gray-200 dark:border-neutral-750 px-3 py-2 rounded-lg font-medium">
                    {t("localBind")}
                  </div>
                  <div className="h-0.5 bg-indigo-500/30 flex-1 mx-2 relative top-0.5" />
                  <div className="bg-indigo-50 dark:bg-indigo-950/40 border border-indigo-200 dark:border-indigo-900/60 px-3 py-2 rounded-lg font-semibold text-indigo-600 dark:text-indigo-400">
                    {t("bastion")} ({tunnel.jumpHost})
                  </div>
                  <div className="h-0.5 bg-indigo-500/30 flex-1 mx-2 relative top-0.5" />
                  <div className="bg-neutral-100 dark:bg-neutral-800 border border-gray-200 dark:border-neutral-750 px-3 py-2 rounded-lg font-medium">
                    {t("target")} ({tunnel.sshHost})
                  </div>
                </div>
              </div>
            )}

            {/* Tunnel Details Summary Card */}
            <div className="p-5 bg-white dark:bg-neutral-900 border border-gray-200 dark:border-neutral-800 rounded-xl space-y-4">
              <h3 className="font-semibold text-sm text-gray-900 dark:text-white">{t("connectionParams")}</h3>
              <div className="grid grid-cols-2 gap-4 text-xs">
                <div className="flex justify-between py-1.5 border-b border-gray-100 dark:border-neutral-850">
                  <span className="text-gray-500 dark:text-neutral-400">{t("host")}:</span>
                  <span className="font-semibold text-gray-800 dark:text-neutral-200">{tunnel.sshHost}:{tunnel.sshPort}</span>
                </div>
                <div className="flex justify-between py-1.5 border-b border-gray-100 dark:border-neutral-850">
                  <span className="text-gray-500 dark:text-neutral-400">{t("user")}:</span>
                  <span className="font-semibold text-gray-800 dark:text-neutral-200">{tunnel.sshUser}</span>
                </div>
                <div className="flex justify-between py-1.5 border-b border-gray-100 dark:border-neutral-850">
                  <span className="text-gray-500 dark:text-neutral-400">{t("localBind")}:</span>
                  <span className="font-semibold text-gray-800 dark:text-neutral-200">{tunnel.localHost || "127.0.0.1"}:{tunnel.localPort}</span>
                </div>
                <div className="flex justify-between py-1.5 border-b border-gray-100 dark:border-neutral-850">
                  <span className="text-gray-500 dark:text-neutral-400">{t("forwardingType")}:</span>
                  <span className="font-semibold text-gray-800 dark:text-neutral-200 uppercase">
                    {tunnel.tunnelType === "socks5" ? t("socks5Forward") : (tunnel.tunnelType === "local" ? t("localForward") : t("remoteForward"))}
                  </span>
                </div>
                {tunnel.sshIdentityFile && (
                  <div className="flex justify-between py-1.5 border-b border-gray-100 dark:border-neutral-850 col-span-2">
                    <span className="text-gray-500 dark:text-neutral-400">{t("privateKey")}:</span>
                    <span className="font-mono text-[10px] text-gray-700 dark:text-neutral-300 truncate max-w-sm">{tunnel.sshIdentityFile}</span>
                  </div>
                )}
              </div>
            </div>

            {/* Quick action warnings */}
            {tunnel.localPort < 1024 && (
              <div className="p-3.5 border border-amber-100 dark:border-amber-950/50 bg-amber-50/30 dark:bg-amber-950/10 rounded-lg flex gap-2.5 text-xs text-amber-800 dark:text-amber-300">
                <AlertTriangle className="w-4 h-4 shrink-0 mt-0.5" />
                <div>
                  <span className="font-semibold block mb-0.5">Privileged Port Warning</span>
                  <span>Port {tunnel.localPort} is less than 1024. Binding to privileged ports requires root privileges on Unix/macOS. You may need to change this port to &gt;= 1024 if start fails.</span>
                </div>
              </div>
            )}
          </div>
        )}

        {/* LOGS TAB PANEL */}
        {activeTab === "logs" && (
          <div className="h-[calc(100vh-210px)]">
            <LogsViewer tunnelId={tunnel.id} logs={logs} onClear={onClearLogs} />
          </div>
        )}

        {/* EVENTS TAB PANEL */}
        {activeTab === "events" && (
          <div className="h-[calc(100vh-210px)]">
            <EventsViewer events={events.filter(e => e.tunnelId === tunnel.id)} onRefresh={onRefreshEvents} />
          </div>
        )}

        {/* SETTINGS TAB PANEL (EDIT DIRECT) */}
        {activeTab === "settings" && (
          <div className="bg-white dark:bg-neutral-900 border border-gray-200 dark:border-neutral-800 rounded-xl p-6 space-y-6 max-w-2xl">
            <div className="flex items-center justify-between border-b border-gray-200 dark:border-neutral-800 pb-3">
              <div>
                <h3 className="font-semibold text-sm text-gray-900 dark:text-white">{t("tabSettings")}</h3>
                <p className="text-[10px] text-gray-400 dark:text-neutral-500">Modify connection configurations or delete the tunnel completely.</p>
              </div>
              <button
                onClick={() => setIsEditing(true)}
                className="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-700 text-white rounded-md text-xs font-semibold transition cursor-pointer"
              >
                {t("tabSettings")}
              </button>
            </div>

            <div className="space-y-4">
              <div className="p-4 border border-red-100 dark:border-red-950 bg-red-50/20 dark:bg-red-950/10 rounded-lg flex items-center justify-between">
                <div>
                  <span className="font-semibold text-xs text-red-800 dark:text-red-300 block mb-0.5">Danger Zone</span>
                  <span className="text-[10px] text-gray-500 dark:text-neutral-400">Deleting a tunnel is permanent. Running connections will be terminated immediately.</span>
                </div>
                <button
                  onClick={() => {
                    if (confirm(t("btnDeleteConfirm"))) {
                      onDeleteTunnel(tunnel.id);
                    }
                  }}
                  className="px-3 py-1.5 bg-red-600 hover:bg-red-750 text-white rounded-md text-xs font-semibold transition cursor-pointer"
                >
                  {t("btnDelete")}
                </button>
              </div>
            </div>

            {/* Launch Editing Modal */}
            {isEditing && (
              <TunnelForm
                tunnel={tunnel}
                groups={groups}
                tunnels={tunnels}
                onSave={(t) => {
                  onSaveTunnel(t);
                  setIsEditing(false);
                }}
                onCancel={() => setIsEditing(false)}
              />
            )}
          </div>
        )}

      </div>
    </div>
  );
}
