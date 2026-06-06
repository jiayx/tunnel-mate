import { useState, useEffect } from "react";
import { 
  X, Settings, Shield, HardDrive, Check, Loader2, 
  Download, Upload, Trash2, AlertCircle, Sliders, ChevronDown
} from "lucide-react";
import { 
  GlobalSettings, getConfig, saveConfig, exportConfig, 
  importConfig, clearEvents 
} from "../lib/tauri";
import { useLanguage } from "../lib/i18n";
import { CompositionInput } from "./CompositionInput";

interface SettingsModalProps {
  theme: "light" | "dark" | "system";
  onThemeChange: (theme: "light" | "dark" | "system") => void;
  onClose: () => void;
  onConfigChanged: () => void;
}

type TabType = "general" | "network" | "data";

const ToggleSwitch = ({ 
  checked, 
  onChange, 
  disabled = false 
}: { 
  checked: boolean; 
  onChange: (checked: boolean) => void; 
  disabled?: boolean;
}) => {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-indigo-500/35 ${
        checked ? "bg-indigo-600" : "bg-gray-200 dark:bg-neutral-800"
      } ${disabled ? "opacity-50 cursor-not-allowed" : ""}`}
    >
      <span
        aria-hidden="true"
        className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm ring-0 transition duration-200 ease-in-out ${
          checked ? "translate-x-4" : "translate-x-0"
        }`}
      />
    </button>
  );
};

export default function SettingsModal({ 
  theme, 
  onThemeChange, 
  onClose, 
  onConfigChanged 
}: SettingsModalProps) {
  const { language, setLanguage, t } = useLanguage();
  const [activeTab, setActiveTab] = useState<TabType>("general");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  // Settings State
  const [launchOnStartup, setLaunchOnStartup] = useState(false);
  const [startMinimized, setStartMinimized] = useState(false);
  const [closeToTray, setCloseToTray] = useState(false);
  const [keepAliveInterval, setKeepAliveInterval] = useState<number | "">(30);
  const [connectTimeout, setConnectTimeout] = useState<number | "">(15);
  const [sshConfigPath, setSshConfigPath] = useState("");

  // Action feedback states
  const [clearingEvents, setClearingEvents] = useState(false);
  const [clearEventsSuccess, setClearEventsSuccess] = useState(false);
  const [importing, setImporting] = useState(false);
  const [importSuccess, setImportSuccess] = useState(false);

  useEffect(() => {
    // Load config from backend
    const loadSettings = async () => {
      try {
        setLoading(true);
        const config = await getConfig();
        const settings: GlobalSettings = config.settings || {
          launchOnStartup: false,
          startMinimized: false,
          closeToTray: false,
          keepAliveInterval: 30,
          connectTimeout: 15,
          sshConfigPath: "",
        };

        setLaunchOnStartup(settings.launchOnStartup);
        setStartMinimized(settings.startMinimized);
        setCloseToTray(settings.closeToTray);
        setKeepAliveInterval(settings.keepAliveInterval);
        setConnectTimeout(settings.connectTimeout);
        setSshConfigPath(settings.sshConfigPath || "");
      } catch (e) {
        setErrorMsg("Failed to load settings: " + e);
      } finally {
        setLoading(false);
      }
    };

    loadSettings();
  }, []);

  // Listen to Escape key to close modal
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const handleNumericChange = (
    val: string, 
    setter: (v: number | "") => void
  ) => {
    if (val === "") {
      setter("");
      return;
    }
    const num = parseInt(val, 10);
    if (!isNaN(num)) {
      setter(num);
    }
  };

  const handleSave = async () => {
    // Basic validation
    if (keepAliveInterval === "" || keepAliveInterval < 0) {
      setErrorMsg("KeepAlive Interval must be a non-negative integer");
      return;
    }
    if (connectTimeout === "" || connectTimeout < 1) {
      setErrorMsg("Connection Timeout must be a positive integer");
      return;
    }

    try {
      setSaving(true);
      setErrorMsg(null);
      
      const config = await getConfig();
      config.settings = {
        launchOnStartup,
        startMinimized,
        closeToTray,
        keepAliveInterval: Number(keepAliveInterval),
        connectTimeout: Number(connectTimeout),
        sshConfigPath: sshConfigPath.trim() || undefined,
      };

      await saveConfig(config);
      setSaveSuccess(true);
      onConfigChanged();
      
      setTimeout(() => {
        setSaveSuccess(false);
        onClose();
      }, 1000);
    } catch (e) {
      setErrorMsg("Failed to save settings: " + e);
    } finally {
      setSaving(false);
    }
  };

  const handleClearEvents = async () => {
    if (!confirm(t("btnDeleteConfirm"))) return;
    try {
      setClearingEvents(true);
      setErrorMsg(null);
      await clearEvents();
      setClearEventsSuccess(true);
      onConfigChanged();
      setTimeout(() => setClearEventsSuccess(false), 3000);
    } catch (e) {
      setErrorMsg("Failed to clear events: " + e);
    } finally {
      setClearingEvents(false);
    }
  };

  const handleExport = async () => {
    try {
      setErrorMsg(null);
      const configStr = await exportConfig();
      const blob = new Blob([configStr], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.href = url;
      link.download = "config.tunnelmate.json";
      link.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      setErrorMsg("Export failed: " + e);
    }
  };

  const handleImport = () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json";
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;

      const reader = new FileReader();
      reader.onload = async (evt) => {
        const text = evt.target?.result as string;
        try {
          setImporting(true);
          setErrorMsg(null);
          await importConfig(text);
          setImportSuccess(true);
          onConfigChanged();
          setTimeout(() => setImportSuccess(false), 3000);
        } catch (err) {
          setErrorMsg("Import failed: " + err);
        } finally {
          setImporting(false);
        }
      };
      reader.readAsText(file);
    };
    input.click();
  };

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4">
      <div className="w-full max-w-2xl bg-white dark:bg-neutral-900 rounded-xl shadow-2xl border border-gray-200 dark:border-neutral-800 overflow-hidden flex flex-col h-[480px]">
        {/* Header */}
        <div className="p-4 border-b border-gray-200 dark:border-neutral-800 flex items-center justify-between shrink-0 bg-gray-50/50 dark:bg-neutral-900/50">
          <div className="flex items-center gap-2">
            <Settings className="w-4 h-4 text-indigo-500 animate-spin-slow" />
            <h3 className="font-semibold text-sm text-gray-900 dark:text-white">{t("globalSettings")}</h3>
          </div>
          <button 
            onClick={onClose}
            className="p-1 rounded-md hover:bg-gray-100 dark:hover:bg-neutral-800 text-gray-500 hover:text-gray-900 dark:hover:text-white transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Body Container */}
        <div className="flex flex-1 overflow-hidden">
          {/* Left Tabs Sidebar */}
          <div className="w-44 border-r border-gray-200 dark:border-neutral-800 bg-gray-50/30 dark:bg-neutral-950/20 p-3 flex flex-col justify-between shrink-0">
            <div className="space-y-1">
              <button
                onClick={() => setActiveTab("general")}
                className={`w-full flex items-center gap-2 px-3 py-2 rounded-lg text-xs font-medium transition cursor-pointer ${
                  activeTab === "general"
                    ? "bg-indigo-50 dark:bg-indigo-950/30 text-indigo-600 dark:text-indigo-400 font-semibold"
                    : "text-gray-600 dark:text-neutral-400 hover:bg-gray-100/80 dark:hover:bg-neutral-800/60"
                }`}
              >
                <Sliders className="w-4 h-4" />
                <span>{t("appBehavior")}</span>
              </button>

              <button
                onClick={() => setActiveTab("network")}
                className={`w-full flex items-center gap-2 px-3 py-2 rounded-lg text-xs font-medium transition cursor-pointer ${
                  activeTab === "network"
                    ? "bg-indigo-50 dark:bg-indigo-950/30 text-indigo-600 dark:text-indigo-400 font-semibold"
                    : "text-gray-600 dark:text-neutral-400 hover:bg-gray-100/80 dark:hover:bg-neutral-800/60"
                }`}
              >
                <Shield className="w-4 h-4" />
                <span>{t("networkTimeouts")}</span>
              </button>

              <button
                onClick={() => setActiveTab("data")}
                className={`w-full flex items-center gap-2 px-3 py-2 rounded-lg text-xs font-medium transition cursor-pointer ${
                  activeTab === "data"
                    ? "bg-indigo-50 dark:bg-indigo-950/30 text-indigo-600 dark:text-indigo-400 font-semibold"
                    : "text-gray-600 dark:text-neutral-400 hover:bg-gray-100/80 dark:hover:bg-neutral-800/60"
                }`}
              >
                <HardDrive className="w-4 h-4" />
                <span>{t("dataManagement")}</span>
              </button>
            </div>

            {/* Error Indicator inside Sidebar if any */}
            {errorMsg && (
              <div className="p-2 bg-red-50 dark:bg-red-950/20 border border-red-200 dark:border-red-950/50 rounded-lg flex items-start gap-1.5 text-[10px] text-red-700 dark:text-red-300">
                <AlertCircle className="w-3.5 h-3.5 mt-0.5 shrink-0" />
                <span className="break-all">{errorMsg}</span>
              </div>
            )}
          </div>

          {/* Right Scrollable Panel */}
          <div className="flex-1 overflow-y-auto p-6 bg-white dark:bg-neutral-900">
            {loading ? (
              <div className="h-full flex flex-col items-center justify-center gap-2 text-xs text-gray-500">
                <Loader2 className="w-5 h-5 animate-spin text-indigo-500" />
                <span>Loading settings...</span>
              </div>
            ) : (
              <div className="space-y-6">
                
                {/* General Tab */}
                {activeTab === "general" && (
                  <div className="space-y-5">
                    <div className="flex items-start justify-between gap-4 pb-4 border-b border-gray-100 dark:border-neutral-800/50">
                      <div className="space-y-0.5">
                        <label className="text-xs font-semibold text-gray-900 dark:text-white block">
                          {t("launchOnStartup")}
                        </label>
                        <span className="text-[10px] text-gray-500 dark:text-neutral-400 block leading-normal max-w-sm">
                          {t("launchOnStartupDesc")}
                        </span>
                      </div>
                      <ToggleSwitch checked={launchOnStartup} onChange={setLaunchOnStartup} />
                    </div>

                    <div className="flex items-start justify-between gap-4 pb-4 border-b border-gray-100 dark:border-neutral-800/50">
                      <div className="space-y-0.5">
                        <label className="text-xs font-semibold text-gray-900 dark:text-white block">
                          {t("closeToTray")}
                        </label>
                        <span className="text-[10px] text-gray-500 dark:text-neutral-400 block leading-normal max-w-sm">
                          {t("closeToTrayDesc")}
                        </span>
                      </div>
                      <ToggleSwitch checked={closeToTray} onChange={setCloseToTray} />
                    </div>

                    <div className="flex items-start justify-between gap-4 pb-4 border-b border-gray-100 dark:border-neutral-800/50">
                      <div className="space-y-0.5">
                        <label className="text-xs font-semibold text-gray-900 dark:text-white block">
                          {t("startMinimized")}
                        </label>
                        <span className="text-[10px] text-gray-500 dark:text-neutral-400 block leading-normal max-w-sm">
                          {t("startMinimizedDesc")}
                        </span>
                      </div>
                      <ToggleSwitch checked={startMinimized} onChange={setStartMinimized} />
                    </div>

                    <div className="flex items-center justify-between gap-4 pb-4 border-b border-gray-100 dark:border-neutral-800/50">
                      <div className="space-y-0.5">
                        <label className="text-xs font-semibold text-gray-900 dark:text-white block">
                          {t("language")}
                        </label>
                        <span className="text-[10px] text-gray-500 dark:text-neutral-400 block leading-normal">
                          Choose application display language.
                        </span>
                      </div>
                      <div className="relative w-28">
                        <select
                          value={language}
                          onChange={(e) => setLanguage(e.target.value as "en" | "zh")}
                          className="w-full px-2.5 py-1.5 pr-7 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md text-gray-900 dark:text-white focus:outline-none focus:ring-1 focus:ring-indigo-500 transition cursor-pointer appearance-none"
                        >
                          <option value="en">English</option>
                          <option value="zh">简体中文</option>
                        </select>
                        <ChevronDown className="w-3.5 h-3.5 text-gray-400 dark:text-neutral-500 absolute right-2 top-1/2 -translate-y-1/2 pointer-events-none" />
                      </div>
                    </div>

                    <div className="flex items-center justify-between gap-4 pb-4">
                      <div className="space-y-0.5">
                        <label className="text-xs font-semibold text-gray-900 dark:text-white block">
                          {t("theme")}
                        </label>
                        <span className="text-[10px] text-gray-500 dark:text-neutral-400 block leading-normal">
                          Choose visual layout theme preference.
                        </span>
                      </div>
                      <div className="relative w-28">
                        <select
                          value={theme}
                          onChange={(e) => onThemeChange(e.target.value as "light" | "dark" | "system")}
                          className="w-full px-2.5 py-1.5 pr-7 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md text-gray-900 dark:text-white focus:outline-none focus:ring-1 focus:ring-indigo-500 transition cursor-pointer appearance-none"
                        >
                          <option value="light">{t("light")}</option>
                          <option value="dark">{t("dark")}</option>
                          <option value="system">{t("system")}</option>
                        </select>
                        <ChevronDown className="w-3.5 h-3.5 text-gray-400 dark:text-neutral-500 absolute right-2 top-1/2 -translate-y-1/2 pointer-events-none" />
                      </div>
                    </div>
                  </div>
                )}

                {/* SSH & Network Tab */}
                {activeTab === "network" && (
                  <div className="space-y-5">
                    <div className="space-y-2.5 pb-4 border-b border-gray-100 dark:border-neutral-800/50">
                      <div className="space-y-0.5">
                        <label className="text-xs font-semibold text-gray-900 dark:text-white block">
                          {t("keepAlive")}
                        </label>
                        <span className="text-[10px] text-gray-500 dark:text-neutral-400 block leading-normal">
                          {t("keepAliveDesc")}
                        </span>
                      </div>
                      <input
                        type="text"
                        value={keepAliveInterval}
                        onChange={(e) => handleNumericChange(e.target.value, setKeepAliveInterval)}
                        placeholder="30"
                        className="w-24 px-2.5 py-1 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md text-gray-900 dark:text-white focus:outline-none focus:ring-1 focus:ring-indigo-500 transition"
                      />
                    </div>

                    <div className="space-y-2.5 pb-4 border-b border-gray-100 dark:border-neutral-800/50">
                      <div className="space-y-0.5">
                        <label className="text-xs font-semibold text-gray-900 dark:text-white block">
                          {t("connTimeout")}
                        </label>
                        <span className="text-[10px] text-gray-500 dark:text-neutral-400 block leading-normal">
                          {t("connTimeoutDesc")}
                        </span>
                      </div>
                      <input
                        type="text"
                        value={connectTimeout}
                        onChange={(e) => handleNumericChange(e.target.value, setConnectTimeout)}
                        placeholder="15"
                        className="w-24 px-2.5 py-1 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md text-gray-900 dark:text-white focus:outline-none focus:ring-1 focus:ring-indigo-500 transition"
                      />
                    </div>

                    <div className="space-y-2">
                      <div className="space-y-0.5">
                        <label className="text-xs font-semibold text-gray-900 dark:text-white block">
                          Custom SSH Config Path
                        </label>
                        <span className="text-[10px] text-gray-500 dark:text-neutral-400 block leading-normal">
                          Override target path for local SSH Config discovery. Leave empty to use system default.
                        </span>
                      </div>
                      <CompositionInput
                        type="text"
                        value={sshConfigPath}
                        onValueChange={setSshConfigPath}
                        placeholder="~/.ssh/config"
                        className="w-full px-2.5 py-1 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md text-gray-900 dark:text-white focus:outline-none focus:ring-1 focus:ring-indigo-500 transition"
                      />
                    </div>
                  </div>
                )}

                {/* Data & Backup Tab */}
                {activeTab === "data" && (
                  <div className="space-y-6">
                    {/* Backup / Restore Config */}
                    <div className="space-y-2.5 pb-5 border-b border-gray-100 dark:border-neutral-800/50">
                      <div className="space-y-0.5">
                        <label className="text-xs font-semibold text-gray-900 dark:text-white block">
                          {t("backupRestore")}
                        </label>
                        <span className="text-[10px] text-gray-500 dark:text-neutral-400 block leading-normal">
                          {t("backupRestoreDesc")}
                        </span>
                      </div>
                      
                      <div className="flex gap-2">
                        <button
                          onClick={handleExport}
                          className="px-3 py-1.5 bg-gray-100 hover:bg-gray-200 dark:bg-neutral-800 dark:hover:bg-neutral-700 text-gray-700 dark:text-neutral-200 text-xs font-medium rounded-md flex items-center gap-1.5 transition cursor-pointer"
                        >
                          <Download className="w-3.5 h-3.5" />
                          <span>{t("btnExport")}</span>
                        </button>
                        <button
                          onClick={handleImport}
                          disabled={importing}
                          className="px-3 py-1.5 bg-gray-100 hover:bg-gray-200 dark:bg-neutral-800 dark:hover:bg-neutral-700 text-gray-700 dark:text-neutral-200 text-xs font-medium rounded-md flex items-center gap-1.5 transition cursor-pointer disabled:opacity-50"
                        >
                          {importing ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Upload className="w-3.5 h-3.5" />}
                          <span>{t("btnImport")}</span>
                        </button>
                        
                        {importSuccess && (
                          <div className="flex items-center gap-1 text-[10px] text-emerald-600 dark:text-emerald-400 animate-fade-in">
                            <Check className="w-3.5 h-3.5" />
                            <span>{t("configImported")}</span>
                          </div>
                        )}
                      </div>
                    </div>

                    {/* Clear events log */}
                    <div className="space-y-2.5">
                      <div className="space-y-0.5">
                        <label className="text-xs font-semibold text-gray-900 dark:text-white block">
                          {t("clearEvents")}
                        </label>
                        <span className="text-[10px] text-gray-500 dark:text-neutral-400 block leading-normal">
                          {t("clearEventsDesc")}
                        </span>
                      </div>
                      
                      <div className="flex items-center gap-3">
                        <button
                          onClick={handleClearEvents}
                          disabled={clearingEvents}
                          className="px-3 py-1.5 bg-red-50 hover:bg-red-100 dark:bg-red-950/20 dark:hover:bg-red-950/40 text-red-600 dark:text-red-400 border border-red-200 dark:border-red-900/55 text-xs font-medium rounded-md flex items-center gap-1.5 transition cursor-pointer disabled:opacity-50"
                        >
                          {clearingEvents ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Trash2 className="w-3.5 h-3.5" />}
                          <span>{t("btnClearEvents")}</span>
                        </button>

                        {clearEventsSuccess && (
                          <div className="flex items-center gap-1 text-[10px] text-emerald-600 dark:text-emerald-400 animate-fade-in">
                            <Check className="w-3.5 h-3.5" />
                            <span>Clear successful</span>
                          </div>
                        )}
                      </div>
                    </div>

                  </div>
                )}

              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="p-3 border-t border-gray-200 dark:border-neutral-800 flex items-center justify-end gap-2 shrink-0 bg-gray-50/50 dark:bg-neutral-900/50">
          <button
            onClick={onClose}
            disabled={saving}
            className="px-3 py-1.5 text-xs bg-white dark:bg-neutral-800 hover:bg-gray-100 dark:hover:bg-neutral-700 border border-gray-200 dark:border-neutral-700 text-gray-700 dark:text-neutral-200 rounded-md font-medium transition cursor-pointer"
          >
            {t("btnCancel")}
          </button>
          <button
            onClick={handleSave}
            disabled={loading || saving}
            className="px-4 py-1.5 text-xs bg-indigo-600 hover:bg-indigo-700 text-white rounded-md font-medium transition cursor-pointer shadow-sm shadow-indigo-600/10 flex items-center gap-1.5 disabled:opacity-50"
          >
            {saving ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
                <span>{t("btnChecking")}</span>
              </>
            ) : saveSuccess ? (
              <>
                <Check className="w-3.5 h-3.5" />
                <span>Saved!</span>
              </>
            ) : (
              <span>{t("btnSaveSettings")}</span>
            )}
          </button>
        </div>

      </div>
    </div>
  );
}
