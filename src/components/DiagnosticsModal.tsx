import React, { useState } from "react";
import { X, CheckCircle2, AlertTriangle, XCircle, Loader2, ArrowRight } from "lucide-react";
import { Tunnel, DiagnosticStep, testConnection } from "../lib/tauri";
import { useLanguage } from "../lib/i18n";
import { CompositionInput } from "./CompositionInput";

interface DiagnosticsModalProps {
  tunnel: Tunnel;
  onClose: () => void;
  onSuccess: (passphrase?: string) => void; // call start tunnel on success
  initialSteps?: DiagnosticStep[];
}

export default function DiagnosticsModal({
  tunnel,
  onClose,
  onSuccess,
  initialSteps,
}: DiagnosticsModalProps) {
  const { t } = useLanguage();
  const [running, setRunning] = useState(false);
  const [steps, setSteps] = useState<DiagnosticStep[]>([]);
  const [passphrase, setPassphrase] = useState("");
  const [passphraseRequired, setPassphraseRequired] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);

  const startTest = async (pass?: string) => {
    setRunning(true);
    setErrorMsg(null);
    try {
      const results = await testConnection(tunnel, pass);
      setSteps(results);

      // Check if passphrase was requested (warning status on SSH Authentication)
      const authStep = results.find(s => s.name.includes("SSH Authentication"));
      if (authStep && authStep.status === "warning" && authStep.message.includes("passphrase")) {
        setPassphraseRequired(true);
      } else {
        setPassphraseRequired(false);
      }
    } catch (e) {
      setErrorMsg(String(e));
    } finally {
      setRunning(false);
    }
  };

  React.useEffect(() => {
    if (initialSteps && initialSteps.length > 0) {
      setSteps(initialSteps);
      // Check if passphrase was requested (warning status on SSH Authentication)
      const authStep = initialSteps.find(s => s.name.includes("SSH Authentication"));
      if (authStep && authStep.status === "warning" && authStep.message.includes("passphrase")) {
        setPassphraseRequired(true);
      } else {
        setPassphraseRequired(false);
      }
    } else {
      startTest();
    }
  }, [tunnel, initialSteps]);

  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const hasErrors = steps.some(s => s.status === "error");
  const isPassphraseOk = passphraseRequired && passphrase.trim().length > 0;
  const showProceed = steps.length > 0 && !hasErrors && !passphraseRequired;

  return (
    <div className="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4">
      <div className="w-full max-w-md bg-white dark:bg-neutral-900 rounded-xl shadow-2xl border border-gray-200 dark:border-neutral-800 overflow-hidden flex flex-col">
        {/* Header */}
        <div className="p-4 border-b border-gray-200 dark:border-neutral-800 flex items-center justify-between">
          <div>
            <h3 className="font-semibold text-sm text-gray-900 dark:text-white">{t("titleConnectionTest")}</h3>
            <p className="text-[10px] text-gray-400 dark:text-neutral-500">{t("subTitleConnectionTest", { name: tunnel.name })}</p>
          </div>
          <button 
            onClick={onClose}
            className="p-1 rounded-md hover:bg-gray-100 dark:hover:bg-neutral-800 text-gray-500 hover:text-gray-900 dark:hover:text-white transition"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content */}
        <div className="p-5 flex-1 space-y-4">
          {errorMsg && (
            <div className="p-3 bg-red-50 dark:bg-red-950/20 border border-red-155 dark:border-red-950 text-red-700 dark:text-red-300 text-xs rounded-md">
              {t("diagnosticExecutionError")}: {errorMsg}
            </div>
          )}

          {/* Diagnostic Steps Checklist */}
          <div className="space-y-3">
            {/* Mock pending check if empty */}
            {steps.length === 0 && running && (
              <div className="flex items-center gap-3 text-xs text-gray-500">
                <Loader2 className="w-4 h-4 animate-spin text-indigo-500" />
                <span>{t("runningChecks")}</span>
              </div>
            )}

            {steps.map((step, idx) => (
              <div key={idx} className="flex items-start gap-3 text-xs border-b border-gray-100 dark:border-neutral-850 pb-2.5 last:border-0 last:pb-0">
                <span className="mt-0.5 shrink-0">
                  {step.status === "success" && <CheckCircle2 className="w-4 h-4 text-emerald-500 fill-emerald-50 dark:fill-transparent" />}
                  {step.status === "warning" && <AlertTriangle className="w-4 h-4 text-amber-500 fill-amber-50 dark:fill-transparent" />}
                  {step.status === "error" && <XCircle className="w-4 h-4 text-red-500 fill-red-50 dark:fill-transparent" />}
                </span>
                <div className="space-y-0.5">
                  <h4 className="font-semibold text-gray-800 dark:text-neutral-200">{step.name}</h4>
                  <p className="text-[11px] text-gray-500 dark:text-neutral-400 leading-normal">{step.message}</p>
                </div>
              </div>
            ))}
          </div>

          {/* Passphrase Input if Required */}
          {passphraseRequired && (
            <div className="p-4 border border-amber-100 dark:border-amber-950/50 bg-amber-50/30 dark:bg-amber-950/10 rounded-lg space-y-3">
              <div>
                <span className="font-semibold text-xs text-amber-800 dark:text-amber-300 block mb-0.5">{t("passphraseRequired")}</span>
                <span className="text-[10px] text-gray-500 dark:text-neutral-400">{t("passphraseDesc")}</span>
              </div>
              <div className="flex gap-2">
                <CompositionInput
                  type="password"
                  placeholder={t("passphrasePlaceholder")}
                  value={passphrase}
                  onValueChange={setPassphrase}
                  className="flex-1 px-3 py-1.5 text-xs bg-white dark:bg-neutral-950 border border-gray-200 dark:border-neutral-800 rounded-md focus:ring-1 focus:ring-indigo-500 text-gray-900 dark:text-white"
                />
                <button
                  onClick={() => startTest(passphrase)}
                  disabled={!isPassphraseOk || running}
                  className="px-3 py-1.5 bg-amber-600 hover:bg-amber-700 disabled:bg-gray-200 dark:disabled:bg-neutral-800 disabled:text-gray-400 text-white rounded-md text-xs font-semibold transition cursor-pointer"
                >
                  {t("btnVerifyKey")}
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 bg-gray-50 dark:bg-neutral-900/50 border-t border-gray-200 dark:border-neutral-800 flex justify-end gap-2">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-white dark:bg-neutral-800 border border-gray-200 dark:border-neutral-700 hover:bg-gray-100 dark:hover:bg-neutral-700 text-gray-700 dark:text-neutral-200 rounded-md text-xs font-semibold transition cursor-pointer"
          >
            {t("btnClose")}
          </button>
          
          {showProceed && (
            <button
              onClick={() => {
                onSuccess(passphrase || undefined);
                onClose();
              }}
              className="px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white rounded-md text-xs font-semibold flex items-center gap-1.5 transition cursor-pointer shadow-sm shadow-emerald-600/10"
            >
              {t("startTunnel")} <ArrowRight className="w-3.5 h-3.5" />
            </button>
          )}

          {hasErrors && (
            <button
              onClick={() => startTest()}
              disabled={running}
              className="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-md text-xs font-semibold transition cursor-pointer"
            >
              {running ? t("btnChecking") : t("btnRetryTest")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
