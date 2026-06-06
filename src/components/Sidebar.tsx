import { useState, useEffect } from "react";
import { 
  Search, Plus, Folder, FolderOpen, Play, Square, 
  Activity, ChevronDown, ChevronRight, Settings,
  Edit, Trash2, FolderPlus
} from "lucide-react";
import { Group, Tunnel, TunnelStatus } from "../lib/tauri";
import { useLanguage } from "../lib/i18n";
import { CompositionInput } from "./CompositionInput";

interface SidebarProps {
  groups: Group[];
  tunnels: Tunnel[];
  statuses: Record<string, TunnelStatus>;
  selectedTunnelId: string | null;
  onSelectTunnel: (id: string | null) => void;
  onNewTunnel: (groupId?: string) => void;
  onNewGroup: () => void;
  onStartGroup: (groupId: string) => void;
  onStopGroup: (groupId: string) => void;
  onOpenSettings: () => void;
  onRenameGroup: (group: Group) => void;
  onDeleteGroup: (groupId: string) => void;
  onStartTunnel: (id: string) => void;
  onStopTunnel: (id: string) => void;
  onDeleteTunnel: (id: string) => void;
  onTestConnection: (id: string) => void;
  onEditTunnel: (id: string) => void;
}

type MenuType = "group" | "tunnel" | "list";

export default function Sidebar({
  groups,
  tunnels,
  statuses,
  selectedTunnelId,
  onSelectTunnel,
  onNewTunnel,
  onNewGroup,
  onStartGroup,
  onStopGroup,
  onOpenSettings,
  onRenameGroup,
  onDeleteGroup,
  onStartTunnel,
  onStopTunnel,
  onDeleteTunnel,
  onTestConnection,
  onEditTunnel,
}: SidebarProps) {
  const { t } = useLanguage();
  const [searchQuery, setSearchQuery] = useState("");
  const [collapsedGroups, setCollapsedGroups] = useState<Record<string, boolean>>({});
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    type: MenuType;
    targetId?: string;
  } | null>(null);

  useEffect(() => {
    const handleCloseMenu = () => setContextMenu(null);
    window.addEventListener("click", handleCloseMenu);
    window.addEventListener("contextmenu", handleCloseMenu);
    return () => {
      window.removeEventListener("click", handleCloseMenu);
      window.removeEventListener("contextmenu", handleCloseMenu);
    };
  }, []);

  const handleGroupContextMenu = (e: React.MouseEvent, groupId: string) => {
    e.preventDefault();
    e.stopPropagation();
    
    const menuWidth = 176;
    const menuHeight = 220;
    let x = e.clientX;
    let y = e.clientY;
    
    const docWidth = window.innerWidth;
    const docHeight = window.innerHeight;
    
    if (x + menuWidth > docWidth) {
      x = docWidth - menuWidth - 8;
    }
    if (y + menuHeight > docHeight) {
      y = docHeight - menuHeight - 8;
    }
    
    setContextMenu({ x, y, type: "group", targetId: groupId });
  };

  const handleTunnelContextMenu = (e: React.MouseEvent, tunnelId: string) => {
    e.preventDefault();
    e.stopPropagation();
    
    const menuWidth = 176;
    const menuHeight = 160;
    let x = e.clientX;
    let y = e.clientY;
    
    const docWidth = window.innerWidth;
    const docHeight = window.innerHeight;
    
    if (x + menuWidth > docWidth) {
      x = docWidth - menuWidth - 8;
    }
    if (y + menuHeight > docHeight) {
      y = docHeight - menuHeight - 8;
    }
    
    setContextMenu({ x, y, type: "tunnel", targetId: tunnelId });
  };

  const handleListContextMenu = (e: React.MouseEvent) => {
    // Only trigger if we clicked directly on the list area, not on child items
    if ((e.target as HTMLElement).closest(".group-item") || (e.target as HTMLElement).closest(".tunnel-item")) {
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    
    const menuWidth = 176;
    const menuHeight = 180;
    let x = e.clientX;
    let y = e.clientY;
    
    const docWidth = window.innerWidth;
    const docHeight = window.innerHeight;
    
    if (x + menuWidth > docWidth) {
      x = docWidth - menuWidth - 8;
    }
    if (y + menuHeight > docHeight) {
      y = docHeight - menuHeight - 8;
    }
    
    setContextMenu({ x, y, type: "list" });
  };

  const toggleGroup = (groupId: string) => {
    setCollapsedGroups(prev => ({ ...prev, [groupId]: !prev[groupId] }));
  };

  const getStatusColor = (status?: TunnelStatus) => {
    switch (status) {
      case "running": return "bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]";
      case "connecting": return "bg-amber-500 animate-pulse shadow-[0_0_8px_rgba(245,158,11,0.5)]";
      case "reconnecting": return "bg-blue-500 animate-pulse shadow-[0_0_8px_rgba(59,130,246,0.5)]";
      case "failed": return "bg-red-500 shadow-[0_0_8px_rgba(239,68,68,0.5)]";
      default: return "bg-gray-400 dark:bg-gray-600";
    }
  };

  // Filter tunnels by query
  const filteredTunnels = tunnels.filter(t => 
    t.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
    t.sshHost.toLowerCase().includes(searchQuery.toLowerCase()) ||
    t.tunnelType.toLowerCase().includes(searchQuery.toLowerCase())
  );

  // Group tunnels
  const tunnelsByGroup: Record<string, Tunnel[]> = {};
  const ungroupedTunnels: Tunnel[] = [];

  filteredTunnels.forEach(t => {
    if (t.groupId) {
      if (!tunnelsByGroup[t.groupId]) tunnelsByGroup[t.groupId] = [];
      tunnelsByGroup[t.groupId].push(t);
    } else {
      ungroupedTunnels.push(t);
    }
  });

  return (
    <aside className="w-64 flex flex-col h-screen border-r border-gray-200 dark:border-neutral-800 bg-gray-50/80 dark:bg-neutral-900/90 backdrop-blur-md select-none">
      {/* App Header */}
      <div className="p-4 flex items-center justify-between border-b border-gray-200 dark:border-neutral-800">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-gradient-to-tr from-indigo-500 to-purple-600 flex items-center justify-center text-white font-bold text-lg shadow-md shadow-indigo-500/20">
            T
          </div>
          <div>
            <h1 className="font-semibold text-sm tracking-wide text-gray-900 dark:text-white">Tunnel Mate</h1>
            <p className="text-[10px] text-gray-500 dark:text-neutral-400">Secure SSH Manager</p>
          </div>
        </div>
      </div>

      {/* Search Bar */}
      <div className="px-3 pt-3 flex flex-col gap-2">
        <div className="relative">
          <Search className="w-4 h-4 absolute left-2.5 top-2.5 text-gray-400 dark:text-neutral-500" />
          <CompositionInput
            type="text"
            placeholder={t("searchPlaceholder")}
            value={searchQuery}
            onValueChange={setSearchQuery}
            className="w-full pl-9 pr-3 py-1.5 text-xs bg-white dark:bg-neutral-955 border border-gray-200 dark:border-neutral-800 rounded-md text-gray-900 dark:text-white focus:outline-none focus:ring-1 focus:ring-indigo-500 transition"
          />
        </div>
      </div>

      {/* Navigation List */}
      <div 
        onContextMenu={handleListContextMenu}
        className="flex-1 overflow-y-auto px-2 py-4 space-y-4"
      >
        {/* Global actions */}
        <div>
          <button
            onClick={() => onSelectTunnel(null)}
            className={`w-full flex items-center gap-2 px-3 py-1.5 rounded-md text-xs font-medium transition ${
              selectedTunnelId === null 
                ? "bg-gray-200/80 dark:bg-neutral-800 text-gray-900 dark:text-white" 
                : "text-gray-600 dark:text-neutral-400 hover:bg-gray-100 dark:hover:bg-neutral-800/50"
            }`}
          >
            <Activity className="w-4 h-4 text-indigo-500" />
            <span>{t("dashboardOverview")}</span>
          </button>
        </div>

        {/* Groups & Tunnels */}
        <div className="space-y-1">
          <span className="px-3 text-[10px] font-semibold text-gray-400 dark:text-neutral-500 uppercase tracking-wider block mb-2">
            {t("sshTunnels")}
          </span>

          {/* Render Groups */}
          {groups.map(g => {
            const isCollapsed = collapsedGroups[g.id];
            const groupTunnels = tunnelsByGroup[g.id] || [];
            
            // Only render group if it matches search query OR has children matching search query
            if (searchQuery && groupTunnels.length === 0) return null;

            return (
              <div key={g.id} className="space-y-0.5">
                <div 
                  onContextMenu={(e) => handleGroupContextMenu(e, g.id)}
                  className="group/item group-item flex items-center justify-between px-2 py-1 hover:bg-gray-200/55 dark:hover:bg-neutral-855 rounded-md transition text-xs select-none"
                >
                  <button 
                    onClick={() => toggleGroup(g.id)}
                    className="flex items-center gap-1.5 flex-1 font-medium text-gray-700 dark:text-neutral-300 text-left cursor-pointer"
                  >
                    {isCollapsed ? <ChevronRight className="w-3.5 h-3.5 text-gray-400" /> : <ChevronDown className="w-3.5 h-3.5 text-gray-400" />}
                    {isCollapsed ? <Folder className="w-3.5 h-3.5 text-yellow-500/80" /> : <FolderOpen className="w-3.5 h-3.5 text-yellow-500/80" />}
                    <span className="truncate">{g.name}</span>
                    <span className="text-[10px] text-gray-400 dark:text-neutral-500">({groupTunnels.length})</span>
                  </button>

                  {/* Group Action Buttons */}
                  <div className="opacity-0 group-hover/item:opacity-100 flex items-center gap-1.5 transition">
                    <button 
                      title={t("startAll")}
                      onClick={(e) => { e.stopPropagation(); onStartGroup(g.id); }}
                      className="p-0.5 hover:bg-gray-300 dark:hover:bg-neutral-700 rounded text-emerald-500 transition cursor-pointer"
                    >
                      <Play className="w-3 h-3 fill-current" />
                    </button>
                    <button 
                      title={t("stopAll")}
                      onClick={(e) => { e.stopPropagation(); onStopGroup(g.id); }}
                      className="p-0.5 hover:bg-gray-300 dark:hover:bg-neutral-700 rounded text-red-500 transition cursor-pointer"
                    >
                      <Square className="w-3 h-3 fill-current" />
                    </button>
                  </div>
                </div>

                {!isCollapsed && (
                  <div className="pl-4 border-l border-gray-200 dark:border-neutral-800 ml-3.5 space-y-0.5">
                    {groupTunnels.map(t => (
                      <button
                        key={t.id}
                        onClick={() => onSelectTunnel(t.id)}
                        onContextMenu={(e) => handleTunnelContextMenu(e, t.id)}
                        className={`w-full tunnel-item flex items-center justify-between px-2.5 py-1.5 rounded-md text-[11px] transition text-left ${
                          selectedTunnelId === t.id
                            ? "bg-indigo-50/80 dark:bg-indigo-950/40 text-indigo-700 dark:text-indigo-300 font-medium"
                            : "text-gray-600 dark:text-neutral-400 hover:bg-gray-150 dark:hover:bg-neutral-805/40"
                        }`}
                      >
                        <div className="flex items-center gap-2 truncate pointer-events-none">
                          <span className={`w-2 h-2 rounded-full ${getStatusColor(statuses[t.id])}`} />
                          <span className="truncate">{t.name}</span>
                        </div>
                        <span className="text-[9px] uppercase tracking-wider text-gray-400 dark:text-neutral-500 pointer-events-none">
                          {t.tunnelType === "socks5" ? "S5" : t.tunnelType}
                        </span>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            );
          })}

          {/* Render Ungrouped Tunnels */}
          {ungroupedTunnels.map(t => (
            <button
              key={t.id}
              onClick={() => onSelectTunnel(t.id)}
              onContextMenu={(e) => handleTunnelContextMenu(e, t.id)}
              className={`w-full tunnel-item flex items-center justify-between px-3 py-1.5 rounded-md text-xs transition text-left ${
                selectedTunnelId === t.id
                  ? "bg-indigo-50/80 dark:bg-indigo-950/40 text-indigo-700 dark:text-indigo-300 font-medium"
                  : "text-gray-600 dark:text-neutral-400 hover:bg-gray-100 dark:hover:bg-neutral-800/50"
              }`}
            >
              <div className="flex items-center gap-2 truncate pointer-events-none">
                <span className={`w-2 h-2 rounded-full ${getStatusColor(statuses[t.id])}`} />
                <span className="truncate">{t.name}</span>
              </div>
              <span className="text-[9px] uppercase tracking-wider text-gray-400 dark:text-neutral-500 pointer-events-none">
                {t.tunnelType === "socks5" ? "SOCKS5" : t.tunnelType}
              </span>
            </button>
          ))}

          {filteredTunnels.length === 0 && (
            <p className="text-[10px] text-gray-400 dark:text-neutral-500 px-3 py-2 italic text-center">
              {t("noTunnels")}
            </p>
          )}
        </div>
      </div>

      {/* Bottom Status / Actions Bar */}
      <div className="p-2 border-t border-gray-200 dark:border-neutral-800 flex items-center justify-between bg-gray-100/40 dark:bg-neutral-900/50">
        <div className="flex items-center gap-1">
          {/* Add Tunnel Button */}
          <button
            onClick={() => onNewTunnel()}
            className="p-1.5 rounded-md text-gray-500 hover:text-gray-900 dark:text-neutral-400 dark:hover:text-white hover:bg-gray-200 dark:hover:bg-neutral-800 transition cursor-pointer"
            title={t("btnNewTunnel")}
          >
            <Plus className="w-4 h-4" />
          </button>
          {/* New Group Button */}
          <button
            onClick={onNewGroup}
            className="p-1.5 rounded-md text-gray-500 hover:text-gray-900 dark:text-neutral-400 dark:hover:text-white hover:bg-gray-200 dark:hover:bg-neutral-800 transition cursor-pointer"
            title={t("btnNewGroup")}
          >
            <FolderPlus className="w-4 h-4" />
          </button>
        </div>

        {/* Settings Button */}
        <button
          onClick={onOpenSettings}
          className="p-1.5 rounded-md text-gray-500 hover:text-gray-900 dark:text-neutral-400 dark:hover:text-white hover:bg-gray-200 dark:hover:bg-neutral-800 transition cursor-pointer"
          title={t("globalSettings")}
        >
          <Settings className="w-4 h-4" />
        </button>
      </div>

      {/* Floating Context Menu */}
      {contextMenu && (
        <div 
          style={{ top: contextMenu.y, left: contextMenu.x }}
          className="fixed bg-white/95 dark:bg-neutral-850/95 border border-gray-200 dark:border-neutral-750 rounded-lg shadow-xl py-1.5 z-[100] text-xs w-44 font-normal text-gray-700 dark:text-neutral-200 glass animate-fade-in"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Render Group Menu */}
          {contextMenu.type === "group" && (() => {
            const currentGroup = groups.find(g => g.id === contextMenu.targetId);
            if (!currentGroup) return null;
            return (
              <>
                {/* Create Tunnel Inside */}
                <button
                  onClick={() => {
                    onNewTunnel(currentGroup.id);
                    setContextMenu(null);
                  }}
                  className="w-full text-left px-3 py-1.5 hover:bg-indigo-50 hover:text-indigo-600 dark:hover:bg-indigo-950/30 dark:hover:text-indigo-400 flex items-center gap-2 cursor-pointer transition font-medium"
                >
                  <Plus className="w-3.5 h-3.5" />
                  <span>{t("newTunnelInGroup")}</span>
                </button>

                {/* Rename */}
                <button
                  onClick={() => {
                    onRenameGroup(currentGroup);
                    setContextMenu(null);
                  }}
                  className="w-full text-left px-3 py-1.5 hover:bg-gray-100 dark:hover:bg-neutral-750/60 flex items-center gap-2 cursor-pointer transition"
                >
                  <Edit className="w-3.5 h-3.5 text-gray-400" />
                  <span>{t("renameGroup")}</span>
                </button>

                <div className="h-[1px] bg-gray-150 dark:bg-neutral-700 my-1" />

                {/* Start All */}
                <button
                  onClick={() => {
                    onStartGroup(currentGroup.id);
                    setContextMenu(null);
                  }}
                  className="w-full text-left px-3 py-1.5 hover:bg-emerald-50 hover:text-emerald-600 dark:hover:bg-emerald-950/20 dark:hover:text-emerald-450 flex items-center gap-2 cursor-pointer transition font-medium"
                >
                  <Play className="w-3.5 h-3.5 fill-current" />
                  <span>{t("startAll")}</span>
                </button>

                {/* Stop All */}
                <button
                  onClick={() => {
                    onStopGroup(currentGroup.id);
                    setContextMenu(null);
                  }}
                  className="w-full text-left px-3 py-1.5 hover:bg-red-50 hover:text-red-650 dark:hover:bg-red-950/20 dark:hover:text-red-400 flex items-center gap-2 cursor-pointer transition font-medium"
                >
                  <Square className="w-3.5 h-3.5 fill-current" />
                  <span>{t("stopAll")}</span>
                </button>

                <div className="h-[1px] bg-gray-150 dark:bg-neutral-700 my-1" />

                {/* Delete Group */}
                <button
                  onClick={() => {
                    onDeleteGroup(currentGroup.id);
                    setContextMenu(null);
                  }}
                  className="w-full text-left px-3 py-1.5 hover:bg-red-50 hover:text-red-650 dark:hover:bg-red-950/20 dark:hover:text-red-400 text-red-600 dark:text-red-405 flex items-center gap-2 cursor-pointer transition font-medium"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                  <span>{t("deleteGroup")}</span>
                </button>
              </>
            );
          })()}

          {/* Render Tunnel Menu */}
          {contextMenu.type === "tunnel" && (() => {
            const currentTunnel = tunnels.find(t => t.id === contextMenu.targetId);
            if (!currentTunnel) return null;
            const isRunning = statuses[currentTunnel.id] === "running" || statuses[currentTunnel.id] === "connecting" || statuses[currentTunnel.id] === "reconnecting";
            
            return (
              <>
                {/* Start / Stop */}
                {!isRunning ? (
                  <button
                    onClick={() => {
                      onStartTunnel(currentTunnel.id);
                      setContextMenu(null);
                    }}
                    className="w-full text-left px-3 py-1.5 hover:bg-emerald-50 hover:text-emerald-600 dark:hover:bg-emerald-950/20 dark:hover:text-emerald-450 flex items-center gap-2 cursor-pointer transition font-medium"
                  >
                    <Play className="w-3.5 h-3.5 fill-current text-emerald-500" />
                    <span>{t("startTunnel")}</span>
                  </button>
                ) : (
                  <button
                    onClick={() => {
                      onStopTunnel(currentTunnel.id);
                      setContextMenu(null);
                    }}
                    className="w-full text-left px-3 py-1.5 hover:bg-red-50 hover:text-red-650 dark:hover:bg-red-950/20 dark:hover:text-red-400 flex items-center gap-2 cursor-pointer transition font-medium"
                  >
                    <Square className="w-3.5 h-3.5 fill-current text-red-500" />
                    <span>{t("stopTunnel")}</span>
                  </button>
                )}

                {/* Edit Tunnel */}
                <button
                  onClick={() => {
                    onEditTunnel(currentTunnel.id);
                    setContextMenu(null);
                  }}
                  className="w-full text-left px-3 py-1.5 hover:bg-indigo-50 hover:text-indigo-600 dark:hover:bg-indigo-950/30 dark:hover:text-indigo-400 flex items-center gap-2 cursor-pointer transition font-medium"
                >
                  <Edit className="w-3.5 h-3.5 text-indigo-400" />
                  <span>{t("editTunnel")}</span>
                </button>

                {/* Connection Diagnostics */}
                <button
                  onClick={() => {
                    onTestConnection(currentTunnel.id);
                    setContextMenu(null);
                  }}
                  className="w-full text-left px-3 py-1.5 hover:bg-indigo-50 hover:text-indigo-600 dark:hover:bg-indigo-950/30 dark:hover:text-indigo-400 flex items-center gap-2 cursor-pointer transition font-medium"
                >
                  <Activity className="w-3.5 h-3.5 text-indigo-505" />
                  <span>{t("diagnostics")}</span>
                </button>

                <div className="h-[1px] bg-gray-150 dark:bg-neutral-700 my-1" />

                {/* Delete Tunnel */}
                <button
                  onClick={() => {
                    if (confirm(t("btnDeleteConfirm"))) {
                      onDeleteTunnel(currentTunnel.id);
                    }
                    setContextMenu(null);
                  }}
                  className="w-full text-left px-3 py-1.5 hover:bg-red-50 hover:text-red-655 dark:hover:bg-red-950/20 dark:hover:text-red-400 text-red-600 dark:text-red-400 flex items-center gap-2 cursor-pointer transition font-medium"
                >
                  <Trash2 className="w-3.5 h-3.5 text-red-500" />
                  <span>{t("deleteTunnel")}</span>
                </button>
              </>
            );
          })()}

          {/* Render List Menu */}
          {contextMenu.type === "list" && (
            <>
              {/* Add Tunnel */}
              <button
                onClick={() => {
                  onNewTunnel();
                  setContextMenu(null);
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-indigo-50 hover:text-indigo-600 dark:hover:bg-indigo-950/30 dark:hover:text-indigo-400 flex items-center gap-2 cursor-pointer transition font-medium"
              >
                <Plus className="w-3.5 h-3.5" />
                <span>{t("btnNewTunnel")}</span>
              </button>

              {/* Create Group */}
              <button
                onClick={() => {
                  onNewGroup();
                  setContextMenu(null);
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-indigo-50 hover:text-indigo-600 dark:hover:bg-indigo-950/30 dark:hover:text-indigo-400 flex items-center gap-2 cursor-pointer transition font-medium"
              >
                <FolderPlus className="w-3.5 h-3.5 text-yellow-500/80" />
                <span>{t("btnNewGroup")}</span>
              </button>

              <div className="h-[1px] bg-gray-150 dark:bg-neutral-700 my-1" />

              {/* Start All Tunnels */}
              <button
                onClick={() => {
                  tunnels.forEach(t => {
                    if (statuses[t.id] !== "running") {
                      onStartTunnel(t.id);
                    }
                  });
                  setContextMenu(null);
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-emerald-50 hover:text-emerald-600 dark:hover:bg-emerald-950/20 dark:hover:text-emerald-450 flex items-center gap-2 cursor-pointer transition font-medium"
              >
                <Play className="w-3.5 h-3.5 fill-current text-emerald-500" />
                <span>{t("startAll")}</span>
              </button>

              {/* Stop All Tunnels */}
              <button
                onClick={() => {
                  tunnels.forEach(t => {
                    const stat = statuses[t.id];
                    if (stat === "running" || stat === "connecting" || stat === "reconnecting") {
                      onStopTunnel(t.id);
                    }
                  });
                  setContextMenu(null);
                }}
                className="w-full text-left px-3 py-1.5 hover:bg-red-50 hover:text-red-650 dark:hover:bg-red-950/20 dark:hover:text-red-400 flex items-center gap-2 cursor-pointer transition font-medium"
              >
                <Square className="w-3.5 h-3.5 fill-current text-red-550" />
                <span>{t("stopAll")}</span>
              </button>
            </>
          )}
        </div>
      )}
    </aside>
  );
}
