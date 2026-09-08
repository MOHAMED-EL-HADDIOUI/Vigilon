import { useEffect, useId, useRef, useState } from "react";
import {
  ShieldAlert,
  AlertTriangle,
  Info,
  CheckCircle2,
  Terminal,
  ArrowRight,
  Clock,
  X,
  Radio,
  FileCode,
  ThumbsDown,
} from "lucide-react";
import type { SecurityEvent } from "../types";
import { apiUrl, indicatorText, riskColor, riskLabel } from "../lib";

export default function WhyPanel({
  event,
  onClose,
}: {
  event: SecurityEvent;
  onClose: () => void;
}) {
  const titleId = useId();
  const panelRef = useRef<HTMLElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  const [feedback, setFeedback] = useState<"idle" | "sent" | "error">("idle");

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handler);
    // Move focus into the dialog; restore it on unmount.
    const prev = document.activeElement as HTMLElement | null;
    closeRef.current?.focus();
    const panel = panelRef.current;
    const trap = (e: KeyboardEvent) => {
      if (e.key !== "Tab" || !panel) return;
      const items = panel.querySelectorAll<HTMLElement>(
        'button, [href], input, [tabindex]:not([tabindex="-1"])',
      );
      if (items.length === 0) return;
      const first = items[0];
      const last = items[items.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };
    document.addEventListener("keydown", trap);
    return () => {
      document.removeEventListener("keydown", handler);
      document.removeEventListener("keydown", trap);
      prev?.focus?.();
    };
  }, [onClose]);

  const sendFeedback = async () => {
    try {
      const key = event.remote ?? event.local ?? "";
      const res = await fetch(apiUrl("/feedback"), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          event_id: event.id,
          rule: event.rule,
          key,
          verdict: "false_positive",
        }),
      });
      setFeedback(res.ok ? "sent" : "error");
    } catch {
      setFeedback("error");
    }
  };

  const isCritical = event.risk === "HIGH" || event.risk === "CRITICAL";
  const isMedium = event.risk === "MEDIUM";

  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      {/* Frosted Backdrop */}
      <div
        className="fixed inset-0 bg-black/60 backdrop-blur-sm transition-opacity duration-300"
        onClick={onClose}
        aria-hidden
      />

      {/* Apple Inspector Sheet */}
      <aside
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className="relative z-10 flex h-full w-full max-w-lg flex-col border-l border-white/[0.12] bg-[#0c101a]/90 shadow-[0_0_60px_rgba(0,0,0,0.8)] backdrop-blur-3xl"
      >
        {/* Header */}
        <div className="flex items-center justify-between border-b border-white/[0.08] px-6 py-4">
          <div className="flex items-center gap-3">
            <div
              className={`flex h-9 w-9 items-center justify-center rounded-xl border ${
                isCritical
                  ? "bg-rose-500/15 border-rose-500/30 text-rose-400"
                  : isMedium
                  ? "bg-amber-500/15 border-amber-500/30 text-amber-400"
                  : "bg-blue-500/15 border-blue-500/30 text-blue-400"
              }`}
            >
              {isCritical ? (
                <AlertTriangle className="h-5 w-5" />
              ) : (
                <ShieldAlert className="h-5 w-5" />
              )}
            </div>
            <div>
              <p className="text-[10px] font-semibold uppercase tracking-widest text-slate-400">
                Heuristic assessment
              </p>
              <h2 id={titleId} className="text-base font-semibold tracking-tight text-white capitalize">
                {event.rule.replaceAll("_", " ")}
              </h2>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <kbd className="hidden sm:inline-block rounded-md border border-white/10 bg-white/[0.05] px-2 py-0.5 text-[10px] font-mono text-slate-400">
              esc
            </kbd>
            <button
              ref={closeRef}
              type="button"
              className="rounded-xl border border-white/10 p-1.5 text-slate-400 transition-colors hover:bg-white/10 hover:text-white"
              onClick={onClose}
              aria-label={`Close ${event.rule.replaceAll("_", " ")} details`}
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </div>

        {/* Content Body */}
        <div className="flex-1 space-y-6 overflow-y-auto px-6 py-5">
          {/* Assessment Banner Card */}
          <div
            className={`rounded-2xl border p-4 backdrop-blur-md ${
              isCritical
                ? "bg-rose-500/[0.08] border-rose-500/25"
                : isMedium
                ? "bg-amber-500/[0.08] border-amber-500/25"
                : "bg-blue-500/[0.08] border-blue-500/25"
            }`}
          >
            <div className="flex items-center justify-between mb-1.5">
              <span className="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
                Assessment Level
              </span>
              <span
                className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-semibold ${riskColor(
                  event.risk,
                )}`}
              >
                ● {riskLabel(event.risk)}
              </span>
            </div>
            <p className="text-xs leading-relaxed text-slate-300">
              {event.assessment ??
                "Suspicious activity observed based on baseline heuristics. No automated action has been taken."}
            </p>
          </div>

          {/* Context Details Card */}
          <div className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-4">
            <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-slate-400">
              Event Context
            </h3>
            <div className="space-y-2.5 text-xs">
              <div className="flex items-center justify-between border-b border-white/[0.04] pb-2">
                <span className="flex items-center gap-2 text-slate-400">
                  <Terminal className="h-3.5 w-3.5 text-slate-400" />
                  Process
                </span>
                <span className="font-mono text-white">
                  {event.process ? `${event.process.name} (PID ${event.process.pid})` : "System"}
                </span>
              </div>

              <div className="flex items-center justify-between border-b border-white/[0.04] pb-2">
                <span className="flex items-center gap-2 text-slate-400">
                  <Radio className="h-3.5 w-3.5 text-slate-400" />
                  Local Socket
                </span>
                <span className="font-mono text-slate-200">{event.local ?? "—"}</span>
              </div>

              <div className="flex items-center justify-between border-b border-white/[0.04] pb-2">
                <span className="flex items-center gap-2 text-slate-400">
                  <ArrowRight className="h-3.5 w-3.5 text-slate-400" />
                  Remote Socket
                </span>
                <span className="font-mono text-slate-200">{event.remote ?? "—"}</span>
              </div>

              <div className="flex items-center justify-between">
                <span className="flex items-center gap-2 text-slate-400">
                  <Clock className="h-3.5 w-3.5 text-slate-400" />
                  First Sighting
                </span>
                <span className="rounded-full bg-white/[0.06] px-2 py-0.5 text-[11px] font-medium text-slate-200">
                  {event.first_seen ? "First Observation" : "Seen in Prior Sessions"}
                </span>
              </div>
            </div>
          </div>

          {/* Explanatory Indicators */}
          <div className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-4">
            <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-slate-400">
              Contributing Indicators ({event.indicators.length})
            </h3>
            <ul className="space-y-2.5 text-xs">
              {event.indicators.map((indicator, idx) => (
                <li
                  key={idx}
                  className="flex items-start gap-2.5 rounded-xl bg-white/[0.02] p-2.5 border border-white/[0.04]"
                >
                  <span className={`mt-0.5 ${riskColor(event.risk)}`}>
                    <CheckCircle2 className="h-3.5 w-3.5" />
                  </span>
                  <span className="text-slate-200 leading-relaxed">
                    {indicatorText(indicator)}
                  </span>
                </li>
              ))}
            </ul>
          </div>

          {/* Binary Path Info */}
          {event.process?.path && (
            <div className="rounded-2xl border border-white/[0.08] bg-white/[0.025] p-4">
              <h3 className="mb-2 flex items-center gap-2 text-[11px] font-semibold uppercase tracking-wider text-slate-400">
                <FileCode className="h-3.5 w-3.5" />
                Binary Execution Path
              </h3>
              <p className="break-all rounded-xl bg-black/40 p-2.5 font-mono text-[11px] text-slate-300 border border-white/[0.04]">
                {event.process.path}
              </p>
            </div>
          )}
        </div>

        {/* Apple Non-Accusatory Disclaimer Footer */}
        <div className="border-t border-white/[0.08] bg-[#07090e]/60 px-6 py-3.5 backdrop-blur-md">
          <p className="text-[11px] leading-relaxed text-slate-400 flex items-start gap-2">
            <Info className="h-4 w-4 shrink-0 text-blue-400 mt-0.5" aria-hidden="true" />
            <span>
              Heuristic observation only. Vigilon explains patterns and correlates telemetry; it
              does not automatically convict, quarantine, or terminate processes.
            </span>
          </p>
          <button
            type="button"
            onClick={() => void sendFeedback()}
            disabled={feedback === "sent"}
            className="mt-2.5 inline-flex items-center gap-1.5 rounded-xl border border-white/[0.08] bg-white/[0.03] px-3 py-1.5 text-[11px] font-medium text-slate-300 hover:bg-white/[0.07] hover:text-white transition-colors disabled:opacity-60"
          >
            <ThumbsDown className="h-3.5 w-3.5" aria-hidden="true" />
            {feedback === "sent"
              ? "Marked — similar findings will be suppressed"
              : feedback === "error"
              ? "Couldn't save — try again"
              : "Not suspicious? Mark as false positive"}
          </button>
        </div>
      </aside>
    </div>
  );
}
