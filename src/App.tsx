import { useState, useEffect } from "react";
import { Channel } from "@tauri-apps/api/core";
import { 
  getConfig, saveConfig, getEvents, startTunnel, stopTunnel, 
  getTunnelStatus, AppConfig, Tunnel, Group, TunnelStatus, LogEvent,
  listenToStatusChanges
} from "./lib/tauri";
import Sidebar from "./components/Sidebar";
import TunnelOverview from "./components/TunnelOverview";
import TunnelForm from "./components/TunnelForm";
import DiagnosticsModal from "./components/DiagnosticsModal";
import SettingsModal from "./components/SettingsModal";
import { Folder, X } from "lucide-react";
import { LanguageProvider, useLanguage } from "./lib/i18n";
import { CompositionInput } from "./components/CompositionInput";

export default function App() {
  return (
    <LanguageProvider>
      <AppContent />
    </LanguageProvider>
  );
}

function AppContent() {
  const { t } = useLanguage();
  const [config, setConfig] = useState<AppConfig>({ version: 1, groups: [], tunnels: [] });
  const [selectedTunnelId, setSelectedTunnelId] = useState<string | null>(null);
  const [statuses, setStatuses] = useState<Record<string, TunnelStatus>>({});
  const [logs, setLogs] = useState<Record<string, string[]>>({});
  const [events, setEvents] = useState<LogEvent[]>([]);

  // UI state
  const [showTunnelForm, setShowTunnelForm] = useState(false);
  const [showGroupForm, setShowGroupForm] = useState(false);
  const [showDiagnostics, setShowDiagnostics] = useState(false);
  const [showSettingsModal, setShowSettingsModal] = useState(false);
  const [theme, setTheme] = useState<"light" | "dark" | "system">("system");

  // Group Form state
  const [newGroupName, setNewGroupName] = useState("");
  const [newGroupDesc, setNewGroupDesc] = useState("");
  const [preselectedGroupId, setPreselectedGroupId] = useState<string | undefined>(undefined);
  const [editingGroup, setEditingGroup] = useState<Group | null>(null);
  const [editingTunnel, setEditingTunnel] = useState<Tunnel | null>(null);

  // Load config and query initial states
  const loadData = async () => {
    try {
      const cfg = await getConfig();
      setConfig(cfg);

      // Fetch status for all tunnels
      const statusMap: Record<string, TunnelStatus> = {};
      for (const t of cfg.tunnels) {
        try {
          statusMap[t.id] = await getTunnelStatus(t.id);
        } catch {
          statusMap[t.id] = "stopped";
        }
      }
      setStatuses(statusMap);

      // Fetch history events
      const evs = await getEvents();
      setEvents(evs);
    } catch (e) {
      console.error("Failed to load initial config data:", e);
    }
  };

  useEffect(() => {
    loadData();

    // 1. Listen to real-time status change events
    const unlistenStatus = listenToStatusChanges((payload) => {
      setStatuses(prev => ({ ...prev, [payload.tunnelId]: payload.status }));
      
      // Refresh events when status changes
      getEvents().then(evs => setEvents(evs));
    });

    return () => {
      unlistenStatus.then(f => f());
    };
  }, []);

  // Theme effect
  useEffect(() => {
    const root = window.document.documentElement;
    root.classList.remove("light", "dark");

    if (theme === "system") {
      const systemTheme = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
      root.classList.add(systemTheme);
    } else {
      root.classList.add(theme);
    }
  }, [theme]);

  // Close modals on Escape key
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setShowTunnelForm(false);
        setPreselectedGroupId(undefined);
        setShowGroupForm(false);
        setEditingGroup(null);
        setShowDiagnostics(false);
        setShowSettingsModal(false);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleStartTunnel = async (id: string, passphrase?: string) => {
    try {
      const logChannel = new Channel<string>();
      logChannel.onmessage = (message) => {
        setLogs(prev => {
          const tunnelLogs = prev[id] || [];
          return {
            ...prev,
            [id]: [...tunnelLogs, message],
          };
        });
      };
      await startTunnel(id, logChannel, passphrase);
    } catch (e) {
      alert("Failed to start tunnel: " + e);
    }
  };

  const handleStopTunnel = async (id: string) => {
    try {
      await stopTunnel(id);
    } catch (e) {
      alert("Failed to stop tunnel: " + e);
    }
  };

  const handleStartGroup = async (groupId: string) => {
    const groupTunnels = config.tunnels.filter(t => t.groupId === groupId);
    for (const t of groupTunnels) {
      if (statuses[t.id] !== "running") {
        handleStartTunnel(t.id);
      }
    }
  };

  const handleStopGroup = async (groupId: string) => {
    const groupTunnels = config.tunnels.filter(t => t.groupId === groupId);
    for (const t of groupTunnels) {
      if (statuses[t.id] === "running" || statuses[t.id] === "connecting" || statuses[t.id] === "reconnecting") {
        stopTunnel(t.id);
      }
    }
  };

  const handleSaveTunnel = async (t: Tunnel) => {
    let updatedTunnels = [...config.tunnels];
    const index = updatedTunnels.findIndex(item => item.id === t.id);
    
    if (index >= 0) {
      updatedTunnels[index] = t;
    } else {
      updatedTunnels.push(t);
    }

    const updatedConfig = { ...config, tunnels: updatedTunnels };
    try {
      await saveConfig(updatedConfig);
      setConfig(updatedConfig);
      setShowTunnelForm(false);
      // Log event
      const evs = await getEvents();
      setEvents(evs);
    } catch (e) {
      alert("Failed to save tunnel: " + e);
    }
  };

  const handleDeleteTunnel = async (id: string) => {
    // Stop if running
    if (statuses[id] === "running") {
      await stopTunnel(id);
    }

    const updatedTunnels = config.tunnels.filter(t => t.id !== id);
    const updatedConfig = { ...config, tunnels: updatedTunnels };

    try {
      await saveConfig(updatedConfig);
      setConfig(updatedConfig);
      setSelectedTunnelId(null);
      // Log event
      const evs = await getEvents();
      setEvents(evs);
    } catch (e) {
      alert("Failed to delete tunnel: " + e);
    }
  };

  const handleCloseGroupForm = () => {
    setShowGroupForm(false);
    setEditingGroup(null);
    setNewGroupName("");
    setNewGroupDesc("");
  };

  const handleSaveGroup = async () => {
    if (!newGroupName.trim()) return;

    let updatedConfig;

    if (editingGroup) {
      const updatedGroups = config.groups.map(g => {
        if (g.id === editingGroup.id) {
          return {
            ...g,
            name: newGroupName,
            description: newGroupDesc || undefined,
          };
        }
        return g;
      });
      updatedConfig = {
        ...config,
        groups: updatedGroups,
      };
    } else {
      const newGroup: Group = {
        id: crypto.randomUUID(),
        name: newGroupName,
        description: newGroupDesc || undefined,
      };
      updatedConfig = {
        ...config,
        groups: [...config.groups, newGroup],
      };
    }

    try {
      await saveConfig(updatedConfig);
      setConfig(updatedConfig);
      handleCloseGroupForm();
      // Log event
      const evs = await getEvents();
      setEvents(evs);
    } catch (e) {
      alert("Failed to save group: " + e);
    }
  };

  const handleRenameGroup = (group: Group) => {
    setEditingGroup(group);
    setNewGroupName(group.name);
    setNewGroupDesc(group.description || "");
    setShowGroupForm(true);
  };

  const handleDeleteGroup = async (groupId: string) => {
    if (!confirm(t("confirmDeleteGroup"))) return;

    const updatedGroups = config.groups.filter(g => g.id !== groupId);
    const updatedTunnels = config.tunnels.map(t => {
      if (t.groupId === groupId) {
        return {
          ...t,
          groupId: undefined,
        };
      }
      return t;
    });

    const updatedConfig = {
      ...config,
      groups: updatedGroups,
      tunnels: updatedTunnels,
    };

    try {
      await saveConfig(updatedConfig);
      setConfig(updatedConfig);
      // Log event
      const evs = await getEvents();
      setEvents(evs);
    } catch (e) {
      alert("Failed to delete group: " + e);
    }
  };

  const handleMoveTunnelToGroup = async (tunnelId: string, groupId: string) => {
    const tunnel = config.tunnels.find(t => t.id === tunnelId);
    const group = config.groups.find(g => g.id === groupId);
    if (!tunnel || !group || tunnel.groupId === groupId) return;

    const updatedConfig = {
      ...config,
      tunnels: config.tunnels.map(t => t.id === tunnelId ? { ...t, groupId } : t),
    };

    try {
      await saveConfig(updatedConfig);
      setConfig(updatedConfig);
      setSelectedTunnelId(tunnelId);
      const evs = await getEvents();
      setEvents(evs);
    } catch (e) {
      alert("Failed to move tunnel: " + e);
    }
  };

  const handleTestConnectionForTunnel = (id: string) => {
    setSelectedTunnelId(id);
    setShowDiagnostics(true);
  };

  const selectedTunnel = config.tunnels.find(t => t.id === selectedTunnelId) || null;
  const selectedStatus = selectedTunnelId ? statuses[selectedTunnelId] || "stopped" : "stopped";
  const selectedLogs = selectedTunnelId ? logs[selectedTunnelId] || [] : [];

  return (
    <div className="flex h-screen overflow-hidden bg-gray-100 dark:bg-neutral-950">
      
      {/* Sidebar navigation */}
      <Sidebar
        groups={config.groups}
        tunnels={config.tunnels}
        statuses={statuses}
        selectedTunnelId={selectedTunnelId}
        onSelectTunnel={setSelectedTunnelId}
        onNewTunnel={(groupId) => {
          setPreselectedGroupId(groupId);
          setEditingTunnel(null);
          setShowTunnelForm(true);
        }}
        onNewGroup={() => setShowGroupForm(true)}
        onStartGroup={handleStartGroup}
        onStopGroup={handleStopGroup}
        onOpenSettings={() => setShowSettingsModal(true)}
        onRenameGroup={handleRenameGroup}
        onDeleteGroup={handleDeleteGroup}
        onStartTunnel={handleStartTunnel}
        onStopTunnel={handleStopTunnel}
        onDeleteTunnel={handleDeleteTunnel}
        onTestConnection={handleTestConnectionForTunnel}
        onMoveTunnelToGroup={handleMoveTunnelToGroup}
        onEditTunnel={(id) => {
          const tunnel = config.tunnels.find(t => t.id === id) || null;
          setSelectedTunnelId(id);
          setEditingTunnel(tunnel);
          setPreselectedGroupId(undefined);
          setShowTunnelForm(true);
        }}
      />

      {/* Main panel */}
      <TunnelOverview
        tunnel={selectedTunnel}
        tunnels={config.tunnels}
        status={selectedStatus}
        statuses={statuses}
        logs={selectedLogs}
        events={events}
        onStart={() => {
          if (selectedTunnel) {
            handleStartTunnel(selectedTunnel.id);
          }
        }}
        onStop={() => {
          if (selectedTunnel) {
            handleStopTunnel(selectedTunnel.id);
          }
        }}
        onTestConnection={() => setShowDiagnostics(true)}
        onClearLogs={() => {
          if (selectedTunnelId) {
            setLogs(prev => ({ ...prev, [selectedTunnelId]: [] }));
          }
        }}
        onRefreshEvents={() => {
          getEvents().then(evs => setEvents(evs));
        }}
      />

      {/* NEW / EDIT TUNNEL FORM MODAL */}
      {showTunnelForm && (
        <TunnelForm
          tunnel={editingTunnel}
          groups={config.groups}
          tunnels={config.tunnels}
          onSave={(t) => {
            handleSaveTunnel(t);
            setEditingTunnel(null);
          }}
          onCancel={() => {
            setShowTunnelForm(false);
            setPreselectedGroupId(undefined);
            setEditingTunnel(null);
          }}
          defaultGroupId={preselectedGroupId}
        />
      )}

      {/* NEW / EDIT GROUP FORM MODAL */}
      {showGroupForm && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4">
          <div className="w-full max-w-sm bg-white dark:bg-neutral-900 rounded-xl shadow-2xl border border-gray-200 dark:border-neutral-800 overflow-hidden animate-fade-in">
            <div className="p-4 border-b border-gray-200 dark:border-neutral-800 flex items-center justify-between">
              <h3 className="font-semibold text-sm text-gray-900 dark:text-white flex items-center gap-1.5 select-none">
                <Folder className="w-4 h-4 text-indigo-500" /> {editingGroup ? t("renameGroup") : t("createGroup")}
              </h3>
              <button 
                onClick={handleCloseGroupForm}
                className="p-1 rounded-md hover:bg-gray-100 dark:hover:bg-neutral-800 text-gray-500 hover:text-gray-900 dark:hover:text-white transition cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            
            <div className="p-5 space-y-3">
              <div>
                <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1 select-none">{t("groupName")}</label>
                <CompositionInput
                  type="text"
                  placeholder="e.g., Production, Staging"
                  value={newGroupName}
                  onValueChange={setNewGroupName}
                  className="w-full px-3 py-1.5 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:outline-none focus:ring-1 focus:ring-indigo-500 text-gray-900 dark:text-white transition"
                />
              </div>
              <div>
                <label className="text-[11px] font-semibold text-gray-500 dark:text-neutral-400 block mb-1 select-none">{t("descOptional")}</label>
                <CompositionInput
                  type="text"
                  placeholder="e.g., Database cluster forwards"
                  value={newGroupDesc}
                  onValueChange={setNewGroupDesc}
                  className="w-full px-3 py-1.5 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:outline-none focus:ring-1 focus:ring-indigo-500 text-gray-900 dark:text-white transition"
                />
              </div>
            </div>

            <div className="p-4 bg-gray-50 dark:bg-neutral-900/50 border-t border-gray-200 dark:border-neutral-800 flex justify-end gap-2 shrink-0">
              <button
                onClick={handleCloseGroupForm}
                className="px-3 py-1.5 bg-white dark:bg-neutral-800 border border-gray-200 dark:border-neutral-700 hover:bg-gray-100 dark:hover:bg-neutral-700 text-gray-700 dark:text-neutral-200 rounded-md text-xs font-semibold transition cursor-pointer"
              >
                {t("btnCancel")}
              </button>
              <button
                onClick={handleSaveGroup}
                className="px-3.5 py-1.5 bg-indigo-600 hover:bg-indigo-700 text-white rounded-md text-xs font-semibold transition cursor-pointer shadow-sm shadow-indigo-600/10"
              >
                {t("btnSave")}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* DIAGNOSTICS TEST MODAL */}
      {showDiagnostics && selectedTunnel && (
        <DiagnosticsModal
          tunnel={selectedTunnel}
          onClose={() => setShowDiagnostics(false)}
          onSuccess={(passphrase) => handleStartTunnel(selectedTunnel.id, passphrase)}
        />
      )}

      {/* SETTINGS MODAL */}
      {showSettingsModal && (
        <SettingsModal
          theme={theme}
          onThemeChange={setTheme}
          onClose={() => setShowSettingsModal(false)}
          onConfigChanged={loadData}
        />
      )}

    </div>
  );
}
