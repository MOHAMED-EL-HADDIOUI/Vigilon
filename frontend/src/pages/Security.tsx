import { useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import {
  ShieldAlert,
  AlertTriangle,
  Terminal,
  ArrowRight,
  Clock,
  ChevronRight,
  ShieldCheck,
} from "lucide-react";
import type { Live } from "../useLive";
import WhyPanel from "../components/WhyPanel";
import ExportButton from "../components/ExportButton";
import { getJson } from "../lib";
import { riskLabel, timeAgo } from "../lib";
import type { SecurityEvent, Risk } from "../types";
import PageHeader from "../components/PageHeader";

type RiskFilter = Risk | "ALL";
const RISK_LEVELS: readonly RiskFilter[] = ["ALL", "INFO", "LOW", "MEDIUM", "HIGH", "CRITICAL"];

export default function Security({ live }: { live: Live }) {
  const [selected, setSelected] = useState<SecurityEvent | null>(null);
  const [filter, setFilter] = useState<RiskFilter>("ALL");
  const [params] = useSearchParams();

  // Deep link: /security?event=<id> opens the finding directly. Falls back
  // to a backend fetch when the event isn't in the live buffer.
  useEffect(() => {
    const target = params.get("event");
    if (!target) return;
    const local = live.events.find((e) => e.id === target);
    if (local) {
      setSelected(local);
      return;
    }
    let cancelled = false;
    void getJson<SecurityEvent>(`/events/${encodeURIComponent(target)}`)
      .then((e) => { if (!cancelled) setSelected(e); })
      .catch(() => {});
    return () => { cancelled = true; };
  }, [params, live.events]);

  const filteredEvents = live.events.filter((e) => filter === "ALL" || e.risk === filter);

  const criticalCount = live.events.filter((e) => e.risk === "HIGH" || e.risk === "CRITICAL").length;
  const suspiciousCount = live.events.filter((e) => e.risk === "MEDIUM" || e.risk === "LOW").length;
  const observedCount = live.events.filter((e) => e.risk === "INFO").length;

  return (
    <div className="space-y-6">
      {/* Top Header with Brand Logo & Page Icon */}
      <PageHeader
        icon={ShieldAlert}
        iconColor="text-rose-400"
        iconBg="bg-rose-500/15"
        iconBorder="border-rose-500/25"
        category="SECURITY"
        title="Security & Threat Signals"
        subtitle="Local behavioral heuristics, socket anomalies & pattern explainability"
        actions={
          <div className="flex items-center gap-2 text-xs font-mono">
            <span className="rounded-full bg-rose-500/10 px-3 py-1 text-rose-400 border border-rose-500/20">
              {criticalCount} Elevated
            </span>
            <span className="rounded-full bg-amber-500/10 px-3 py-1 text-amber-400 border border-amber-500/20">
              {suspiciousCount} Suspicious
            </span>
            <span className="rounded-full bg-blue-500/10 px-3 py-1 text-blue-400 border border-blue-500/20">
              {observedCount} Observed
            </span>
            <ExportButton endpoint="events" label="Export Events" />
          </div>
        }
      >
        {/* Apple Segmented Control Filter Tabs */}
        <div className="flex flex-wrap items-center gap-1.5 rounded-2xl bg-white/[0.03] p-1.5 border border-white/[0.06] w-fit">
          {RISK_LEVELS.map((level) => {
            const active = filter === level;
            return (
              <button
                key={level}
                onClick={() => setFilter(level)}
                className={`rounded-xl px-3.5 py-1.5 text-xs font-medium transition-all duration-200 ${
                  active
                    ? "bg-white/[0.12] text-white shadow-[0_2px_8px_rgba(0,0,0,0.4)] border border-white/[0.12]"
                    : "text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]"
                }`}
              >
                {level}
              </button>
            );
          })}
        </div>
      </PageHeader>

      {/* Security Findings Feed */}
      <div className="glass-card rounded-3xl p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-semibold text-white">
            Observed Findings ({filteredEvents.length})
          </h3>
          <span className="text-[11px] text-slate-400">
            Click any entry to inspect contributing heuristics
          </span>
        </div>

        <div className="space-y-2.5">
          {filteredEvents.map((e) => {
            const isElevated = e.risk === "HIGH" || e.risk === "CRITICAL";
            const isMedium = e.risk === "MEDIUM";

            return (
              <div
                role="button"
                tabIndex={0}
                aria-label={`Inspect finding ${e.rule.replaceAll("_", " ")}`}
                key={e.id}
                className="rounded-2xl border border-white/[0.06] bg-white/[0.02] p-4 hover:bg-white/[0.05] hover:border-white/[0.12] transition-all duration-200 cursor-pointer flex flex-col sm:flex-row sm:items-center justify-between gap-4 group focus-visible:outline-2 focus-visible:outline-blue-400"
                onClick={() => setSelected(e)}
                onKeyDown={(ev) => {
                  if (ev.key === "Enter" || ev.key === " ") {
                    ev.preventDefault();
                    setSelected(e);
                  }
                }}
              >
                <div className="flex items-start gap-3.5">
                  <div
                    className={`mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-xl border ${
                      isElevated
                        ? "bg-rose-500/15 border-rose-500/30 text-rose-400 shadow-[0_0_12px_rgba(244,63,94,0.3)]"
                        : isMedium
                        ? "bg-amber-500/15 border-amber-500/30 text-amber-400"
                        : "bg-blue-500/15 border-blue-500/30 text-blue-400"
                    }`}
                  >
                    {isElevated ? (
                      <AlertTriangle className="h-4 w-4" />
                    ) : (
                      <ShieldAlert className="h-4 w-4" />
                    )}
                  </div>

                  <div>
                    <div className="flex items-center gap-2">
                      <p className="text-sm font-semibold text-white capitalize group-hover:text-blue-300 transition-colors">
                        {e.rule.replaceAll("_", " ")}
                      </p>
                      <span
                        className={`rounded-full px-2 py-0.5 text-[10px] font-semibold border ${
                          isElevated
                            ? "bg-rose-500/10 text-rose-400 border-rose-500/20"
                            : isMedium
                            ? "bg-amber-500/10 text-amber-400 border-amber-500/20"
                            : "bg-blue-500/10 text-blue-400 border-blue-500/20"
                        }`}
                      >
                        {riskLabel(e.risk)}
                      </span>
                    </div>

                    <div className="mt-1 flex flex-wrap items-center gap-3 text-xs text-slate-400">
                      {e.process && (
                        <span className="flex items-center gap-1 font-mono text-slate-300">
                          <Terminal className="h-3 w-3 text-slate-400" />
                          {e.process.name}
                        </span>
                      )}
                      {e.remote && (
                        <span className="flex items-center gap-1 font-mono text-slate-400">
                          <ArrowRight className="h-3 w-3 text-slate-500" />
                          {e.remote}
                        </span>
                      )}
                      <span className="flex items-center gap-1 font-mono text-[11px] text-slate-500">
                        <Clock className="h-3 w-3 text-slate-500" />
                        {timeAgo(e.timestamp)}
                      </span>
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-3 self-end sm:self-center">
                  <span className="rounded-full bg-white/[0.05] px-2.5 py-1 text-[11px] font-mono text-slate-300 border border-white/[0.08]">
                    {e.indicators.length} indicator{e.indicators.length > 1 ? "s" : ""}
                  </span>
                  <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-white/[0.05] text-slate-400 group-hover:bg-white/10 group-hover:text-white transition-colors">
                    <ChevronRight className="h-4 w-4" />
                  </div>
                </div>
              </div>
            );
          })}

          {filteredEvents.length === 0 && (
            <div className="rounded-2xl border border-white/[0.06] bg-white/[0.01] p-12 text-center">
              <ShieldCheck className="mx-auto h-10 w-10 text-emerald-400 mb-3" />
              <p className="text-sm font-semibold text-white">No Threat Signals Detected</p>
              <p className="text-xs text-slate-400 mt-1 max-w-sm mx-auto">
                No events match the current filter. No new patterns vs. the observed baseline —
                new activity will appear here with its contributing indicators.
              </p>
            </div>
          )}
        </div>
      </div>

      {/* Slide-in Sheet */}
      {selected && <WhyPanel event={selected} onClose={() => setSelected(null)} />}
    </div>
  );
}
