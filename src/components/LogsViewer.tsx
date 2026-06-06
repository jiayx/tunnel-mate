import { useState, useEffect, useRef } from "react";
import { Copy, Trash2, Download, Search, AlertCircle } from "lucide-react";
import { useLanguage } from "../lib/i18n";

interface LogsViewerProps {
  tunnelId: string;
  logs: string[];
  onClear: () => void;
}

export default function LogsViewer({
  tunnelId,
  logs,
  onClear,
}: LogsViewerProps) {
  const { t } = useLanguage();
  const [filterText, setFilterText] = useState("");
  const [levelFilter, setLevelFilter] = useState<"all" | "info" | "error">("all");
  const terminalEndRef = useRef<HTMLDivElement>(null);

  // Auto scroll to bottom when new logs arrive
  useEffect(() => {
    terminalEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  const handleCopy = () => {
    const text = logs.join("\n");
    navigator.clipboard.writeText(text);
  };

  const handleExport = () => {
    const blob = new Blob([logs.join("\n")], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `tunnel-${tunnelId}-logs.txt`;
    link.click();
    URL.revokeObjectURL(url);
  };

  const getLogLineStyle = (line: string) => {
    if (line.includes("[ERROR]")) return "text-red-400 dark:text-red-400 font-semibold";
    if (line.includes("[WARNING]")) return "text-amber-400 dark:text-amber-400";
    if (line.includes("[INFO]")) return "text-blue-300 dark:text-blue-400";
    return "text-gray-300 dark:text-neutral-300";
  };

  // Filter logs
  const filteredLogs = logs.filter(log => {
    const matchesQuery = log.toLowerCase().includes(filterText.toLowerCase());
    
    if (levelFilter === "all") return matchesQuery;
    if (levelFilter === "info") return matchesQuery && log.includes("[INFO]");
    if (levelFilter === "error") return matchesQuery && log.includes("[ERROR]");
    
    return matchesQuery;
  });

  return (
    <div className="flex-1 flex flex-col h-full bg-neutral-950 rounded-xl border border-neutral-900 overflow-hidden font-mono text-[11px] text-gray-300 shadow-2xl">
      {/* Control Bar */}
      <div className="px-4 py-3 bg-neutral-900 border-b border-neutral-950 flex flex-wrap items-center justify-between gap-3 select-none">
        <div className="flex items-center gap-3">
          {/* Level Filter */}
          <div className="flex bg-neutral-950 p-0.5 rounded border border-neutral-800">
            <button
              onClick={() => setLevelFilter("all")}
              className={`px-2.5 py-1 rounded transition text-[10px] font-semibold ${
                levelFilter === "all" ? "bg-neutral-800 text-white" : "text-gray-400 hover:text-gray-200"
              }`}
            >
              ALL
            </button>
            <button
              onClick={() => setLevelFilter("info")}
              className={`px-2.5 py-1 rounded transition text-[10px] font-semibold ${
                levelFilter === "info" ? "bg-neutral-800 text-blue-400" : "text-gray-400 hover:text-gray-200"
              }`}
            >
              INFO
            </button>
            <button
              onClick={() => setLevelFilter("error")}
              className={`px-2.5 py-1 rounded transition text-[10px] font-semibold ${
                levelFilter === "error" ? "bg-neutral-800 text-red-400" : "text-gray-400 hover:text-gray-200"
              }`}
            >
              ERROR
            </button>
          </div>

          {/* Search */}
          <div className="relative">
            <Search className="w-3.5 h-3.5 absolute left-2 top-2 text-gray-500" />
            <input
              type="text"
              placeholder={t("logsSearchPlaceholder")}
              value={filterText}
              onChange={(e) => setFilterText(e.target.value)}
              className="pl-7 pr-3 py-1 bg-neutral-950 border border-neutral-800 rounded text-[10px] focus:outline-none focus:ring-1 focus:ring-indigo-600 w-36 text-white"
            />
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-1.5">
          <button
            onClick={handleCopy}
            disabled={logs.length === 0}
            className="p-1.5 bg-neutral-950 hover:bg-neutral-800 disabled:opacity-50 text-gray-400 hover:text-white rounded border border-neutral-800 transition cursor-pointer"
            title={t("copyLogs")}
          >
            <Copy className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={handleExport}
            disabled={logs.length === 0}
            className="p-1.5 bg-neutral-950 hover:bg-neutral-800 disabled:opacity-50 text-gray-400 hover:text-white rounded border border-neutral-800 transition cursor-pointer"
            title={t("exportLogs")}
          >
            <Download className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={onClear}
            disabled={logs.length === 0}
            className="p-1.5 bg-neutral-950 hover:bg-neutral-800 disabled:opacity-50 text-red-400 hover:text-red-300 rounded border border-neutral-800 transition cursor-pointer"
            title={t("clearLogs")}
          >
            <Trash2 className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Log Console Output */}
      <div className="flex-1 overflow-y-auto p-4 space-y-1.5 leading-relaxed selection:bg-neutral-800">
        {filteredLogs.map((log, index) => (
          <div key={index} className="flex items-start gap-2">
            <span className="text-neutral-600 select-none text-[10px] mt-0.5">{(index + 1).toString().padStart(3, "0")}</span>
            <span className={getLogLineStyle(log)}>{log}</span>
          </div>
        ))}

        {filteredLogs.length === 0 && (
          <div className="h-full flex flex-col items-center justify-center text-neutral-500 py-10 select-none">
            <AlertCircle className="w-8 h-8 mb-2 text-neutral-600" />
            <span>{logs.length === 0 ? t("terminalWaiting") : t("noMatchingLogs")}</span>
          </div>
        )}
        <div ref={terminalEndRef} />
      </div>
    </div>
  );
}
