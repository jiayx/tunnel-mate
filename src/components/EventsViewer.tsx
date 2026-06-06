import { useState } from "react";
import { Search, Calendar, Info, Clock, AlertTriangle, ShieldCheck, PlayCircle, PlusCircle, Trash2 } from "lucide-react";
import { useLanguage } from "../lib/i18n";
import { LogEvent } from "../lib/tauri";
import { CompositionInput } from "./CompositionInput";

interface EventsViewerProps {
  events: LogEvent[];
  onRefresh: () => void;
}

export default function EventsViewer({
  events,
  onRefresh,
}: EventsViewerProps) {
  const { t } = useLanguage();
  const [filterQuery, setFilterQuery] = useState("");

  const getEventIcon = (type: LogEvent["eventType"]) => {
    switch (type) {
      case "created": return <PlusCircle className="w-4 h-4 text-emerald-500" />;
      case "updated": return <Info className="w-4 h-4 text-indigo-500" />;
      case "started": return <PlayCircle className="w-4 h-4 text-sky-500" />;
      case "stopped": return <Clock className="w-4 h-4 text-neutral-500" />;
      case "reconnected": return <ShieldCheck className="w-4 h-4 text-teal-500" />;
      case "failed": return <AlertTriangle className="w-4 h-4 text-red-500" />;
      case "deleted": return <Trash2 className="w-4 h-4 text-rose-500" />;
      default: return <Info className="w-4 h-4 text-gray-500" />;
    }
  };

  const getEventTagStyle = (type: LogEvent["eventType"]) => {
    switch (type) {
      case "created": return "bg-emerald-50 text-emerald-700 dark:bg-emerald-950/20 dark:text-emerald-400 border-emerald-100 dark:border-emerald-900";
      case "started": return "bg-sky-50 text-sky-700 dark:bg-sky-950/20 dark:text-sky-400 border-sky-100 dark:border-sky-900";
      case "reconnected": return "bg-teal-50 text-teal-700 dark:bg-teal-950/20 dark:text-teal-400 border-teal-100 dark:border-teal-900";
      case "failed": return "bg-red-50 text-red-700 dark:bg-red-950/20 dark:text-red-400 border-red-100 dark:border-red-900";
      default: return "bg-gray-50 text-gray-700 dark:bg-neutral-850 dark:text-neutral-300 border-gray-100 dark:border-neutral-800";
    }
  };

  const filteredEvents = [...events]
    .reverse() // show newest first
    .filter(e => 
      e.message.toLowerCase().includes(filterQuery.toLowerCase()) ||
      (e.tunnelName && e.tunnelName.toLowerCase().includes(filterQuery.toLowerCase())) ||
      e.eventType.toLowerCase().includes(filterQuery.toLowerCase())
    );

  const formatDate = (dateStr: string) => {
    try {
      const d = new Date(dateStr);
      return d.toLocaleString();
    } catch {
      return dateStr;
    }
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-white dark:bg-neutral-900 rounded-xl border border-gray-200 dark:border-neutral-800 overflow-hidden shadow-sm">
      {/* Control Header */}
      <div className="px-4 py-3 bg-gray-50 dark:bg-neutral-900/50 border-b border-gray-200 dark:border-neutral-800 flex items-center justify-between gap-3">
        <div className="relative">
          <div className="absolute inset-y-0 left-2.5 flex items-center pointer-events-none">
            <Search className="w-4 h-4 text-gray-400 dark:text-neutral-500" />
          </div>
          <CompositionInput
            type="text"
            placeholder={t("searchEventsLog")}
            value={filterQuery}
            onValueChange={setFilterQuery}
            className="pl-9 pr-3 py-1.5 bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md text-xs focus:outline-none focus:ring-1 focus:ring-indigo-500 w-52 text-gray-900 dark:text-white"
          />
        </div>

        <button
          onClick={onRefresh}
          className="px-3 py-1.5 bg-white dark:bg-neutral-800 border border-gray-200 dark:border-neutral-700 hover:bg-gray-100 dark:hover:bg-neutral-700 text-gray-750 dark:text-neutral-200 rounded-md text-xs font-semibold transition cursor-pointer"
        >
          {t("btnRefreshLogs")}
        </button>
      </div>

      {/* Events Timeline */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {filteredEvents.map(e => (
          <div key={e.id} className="flex gap-3 items-start group relative">
            <span className="p-1.5 rounded-full bg-gray-100 dark:bg-neutral-800 border border-gray-200 dark:border-neutral-750 shrink-0">
              {getEventIcon(e.eventType)}
            </span>
            
            <div className="flex-1 min-w-0 bg-gray-50/50 dark:bg-neutral-900/40 border border-gray-100 dark:border-neutral-850 p-3 rounded-lg hover:border-gray-200 dark:hover:border-neutral-750 transition">
              <div className="flex flex-wrap items-center justify-between gap-2 mb-1.5">
                <div className="flex items-center gap-2">
                  <span className={`px-2 py-0.5 rounded text-[10px] uppercase font-bold tracking-wider border ${getEventTagStyle(e.eventType)}`}>
                    {t(`ev_${e.eventType}` as any)}
                  </span>
                  {e.tunnelName && (
                    <span className="font-semibold text-xs text-gray-800 dark:text-neutral-200 truncate">
                      {e.tunnelName}
                    </span>
                  )}
                </div>
                
                <div className="flex items-center gap-1 text-[10px] text-gray-400 dark:text-neutral-500">
                  <Clock className="w-3 h-3" />
                  <span>{formatDate(e.timestamp)}</span>
                </div>
              </div>
              <p className="text-xs text-gray-600 dark:text-neutral-400 leading-normal">
                {e.message}
              </p>
            </div>
          </div>
        ))}

        {filteredEvents.length === 0 && (
          <div className="h-full flex flex-col items-center justify-center text-gray-400 dark:text-neutral-500 py-10 select-none">
            <Calendar className="w-8 h-8 mb-2 text-gray-300 dark:text-neutral-700" />
            <span>{t("noEventsFound")}</span>
          </div>
        )}
      </div>
    </div>
  );
}
