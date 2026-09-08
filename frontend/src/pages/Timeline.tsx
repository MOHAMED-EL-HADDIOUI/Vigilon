import { useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import {
  Clock,
  Radio,
  Activity,
  Layers,
} from "lucide-react";
import type { Live } from "../useLive";
import WhyPanel from "../components/WhyPanel";
import type { SecurityEvent, CorrelatedActivity } from "../types";
import { getJson, timeAgo } from "../lib";
import PageHeader from "../components/PageHeader";
import { RiskDot } from "../components/ui";

const CATEGORIES = ["All", "Info", "Suspicious", "Investigate"] as const;

export default function Timeline({ live }: { live: Live }) {
  const [id, setId] = useState<string | null>(null);
  const [corr, setCorr] = useState<CorrelatedActivity | null>(null);
  const [filter, setFilter] = useState<string>("All");
  const [params] = useSearchParams();
  const query = (params.get("q") ?? "").toLowerCase().trim();

  const selected: SecurityEvent | null = useMemo(() => {
    if (!id) return null;
    return live.events.find((e) => e.id === id) ?? corr?.events.find((e) => e.id === id) ?? null;
  }, [id, live.events, corr]);

  const filteredTimeline = useMemo(() => {
    return live.timeline.filter((t) => {
      if (filter === "Info" && t.risk !== "INFO") return false;
      if (filter === "Suspicious" && t.risk !== "LOW" && t.risk !== "MEDIUM") return false;
      if (filter === "Investigate" && t.risk !== "HIGH" && t.risk !== "CRITICAL") return false;
      if (query) {
        const hay = `${t.title} ${t.summary} ${t.kind}`.toLowerCase();
        if (!hay.includes(query)) return false;
      }
      return true;
    });
  }, [live.timeline, filter, query]);

  return (
    <div className="space-y-6">
      {/* Top Header with Brand Logo & Page Icon */}
      <PageHeader
        icon={Clock}
        iconColor="text-blue-400"
        iconBg="bg-blue-500/15"
        iconBorder="border-blue-500/25"
        category="TIMELINE"
        title="Event Timeline & Correlation"
        subtitle="Time-series audit trail with cross-domain socket correlation"
        actions={
          <span className="rounded-full bg-blue-500/10 px-3 py-1 text-xs font-mono text-blue-400 border border-blue-500/20">
            {live.timeline.length} Logged Moments
          </span>
        }
      >
        {/* Segmented Control */}
        <div className="flex flex-wrap items-center gap-1.5 rounded-2xl bg-white/[0.03] p-1.5 border border-white/[0.06] w-fit">
          {CATEGORIES.map((cat) => {
            const active = filter === cat;
            return (
              <button
                key={cat}
                onClick={() => setFilter(cat)}
                aria-pressed={active}
                className={`rounded-xl px-3.5 py-1.5 text-xs font-medium transition-all duration-200 ${
                  active
                    ? "bg-white/[0.12] text-white shadow-[0_2px_8px_rgba(0,0,0,0.4)] border border-white/[0.12]"
                    : "text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]"
                }`}
              >
                {cat}
              </button>
            );
          })}
        </div>
        {query && (
          <p className="mt-3 text-xs text-slate-400" role="status">
            Filtered by linked search <span className="font-mono text-sky-300">“{params.get("q")}”</span> —{" "}
            <a href="/timeline" className="text-slate-200 underline underline-offset-2">clear</a>
          </p>
        )}
      </PageHeader>

      {/* 2-Column Layout: Timeline Rail + Correlation Inspector */}
      <div className="grid gap-6 lg:grid-cols-[1fr_360px]">
        {/* Visual Timeline Rail */}
        <div className="glass-card rounded-3xl p-6">
          <h3 className="mb-4 text-sm font-semibold text-white">
            Chronological Activity Stream
          </h3>

          <div className="relative pl-6 border-l border-white/[0.08] ml-3 space-y-6">
            {filteredTimeline.map((t) => {
              return (
                <div key={t.id} className="relative group">
                  {/* Timeline Glowing Node */}
                  <div className="absolute -left-[31px] top-1.5 transition-transform duration-200 group-hover:scale-125">
                    <RiskDot risk={t.risk} size="h-3 w-3" />
                  </div>

                  {/* Event Card */}
                  <div
                    role="button"
                    tabIndex={0}
                    aria-label={`Correlate activity around ${t.title}`}
                    className="rounded-2xl border border-white/[0.06] bg-white/[0.02] p-4 hover:bg-white/[0.05] hover:border-white/[0.12] transition-all duration-200 cursor-pointer focus-visible:outline-2 focus-visible:outline-blue-400"
                    onClick={() => {
                      setId(t.event_id ?? null);
                      void getJson<CorrelatedActivity>(
                        `/correlate?timestamp=${encodeURIComponent(t.timestamp)}`,
                      )
                        .then(setCorr)
                        .catch(() => setCorr(null));
                    }}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        setId(t.event_id ?? null);
                        void getJson<CorrelatedActivity>(
                          `/correlate?timestamp=${encodeURIComponent(t.timestamp)}`,
                        )
                          .then(setCorr)
                          .catch(() => setCorr(null));
                      }
                    }}
                  >
                    <div className="flex items-center justify-between gap-2 mb-1.5">
                      <span className="text-xs font-semibold text-white group-hover:text-blue-300 transition-colors">
                        {t.title}
                      </span>
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-[11px] text-slate-400">
                          {t.timestamp.replace("T", " ").slice(11, 19)}
                        </span>
                        <span className="text-[10px] text-slate-500">
                          ({timeAgo(t.timestamp)})
                        </span>
                      </div>
                    </div>

                    <p className="text-xs text-slate-400 leading-relaxed">{t.summary}</p>
                  </div>
                </div>
              );
            })}

            {filteredTimeline.length === 0 && (
              <div className="py-8 text-center text-xs text-slate-500">
                No events recorded matching this filter.
              </div>
            )}
          </div>
        </div>

        {/* Right-hand Correlation Inspector */}
        <div className="space-y-4">
          <div className="glass-card rounded-3xl p-6">
            <h3 className="mb-2 text-sm font-semibold text-white flex items-center gap-2">
              <Layers className="h-4 w-4 text-blue-400" />
              <span>Temporal Correlation</span>
            </h3>
            <p className="text-xs text-slate-400 mb-4">
              Cross-domain socket & process snapshot within ±10 minutes of selected timestamp.
            </p>

            {corr ? (
              <div className="space-y-4">
                {/* Stats Summary */}
                <div className="grid grid-cols-3 gap-2 text-center">
                  <div className="rounded-xl bg-white/[0.03] p-2.5 border border-white/[0.05]">
                    <p className="font-bold text-sm text-white">{corr.events.length}</p>
                    <p className="text-[10px] text-slate-400 uppercase">Signals</p>
                  </div>
                  <div className="rounded-xl bg-white/[0.03] p-2.5 border border-white/[0.05]">
                    <p className="font-bold text-sm text-amber-400">{corr.ports.length}</p>
                    <p className="text-[10px] text-slate-400 uppercase">Port Diffs</p>
                  </div>
                  <div className="rounded-xl bg-white/[0.03] p-2.5 border border-white/[0.05]">
                    <p className="font-bold text-sm text-sky-400">{corr.connections.length}</p>
                    <p className="text-[10px] text-slate-400 uppercase">Sockets</p>
                  </div>
                </div>

                {/* Correlated Ports */}
                {corr.ports.length > 0 && (
                  <div>
                    <h4 className="text-[11px] uppercase tracking-wider text-slate-400 font-semibold mb-2 flex items-center gap-1.5">
                      <Radio className="h-3 w-3 text-amber-400" />
                      Port Transitions
                    </h4>
                    <ul className="space-y-1 text-xs font-mono">
                      {corr.ports.slice(0, 5).map((p, idx) => (
                        <li
                          key={idx}
                          className="flex items-center justify-between rounded-lg bg-white/[0.02] p-2 border border-white/[0.04]"
                        >
                          <span className="text-amber-400 font-semibold">:{p.port}</span>
                          <span className="text-slate-300 font-sans">{p.process_name ?? "system"}</span>
                          <span className="text-[10px] text-slate-500">{p.event_type}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {/* Correlated Connections */}
                {corr.connections.length > 0 && (
                  <div>
                    <h4 className="text-[11px] uppercase tracking-wider text-slate-400 font-semibold mb-2 flex items-center gap-1.5">
                      <Activity className="h-3 w-3 text-sky-400" />
                      Active Sockets
                    </h4>
                    <ul className="space-y-1 text-xs font-mono">
                      {corr.connections.slice(0, 5).map((c, idx) => (
                        <li
                          key={idx}
                          className="flex items-center justify-between rounded-lg bg-white/[0.02] p-2 border border-white/[0.04]"
                        >
                          <span className="text-slate-300 truncate max-w-[140px] text-[11px]">
                            {c.local} → {c.remote}
                          </span>
                          <span className="text-[10px] text-slate-400 font-sans">
                            {c.process_name ?? ""}
                          </span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            ) : (
              <div className="rounded-2xl border border-white/[0.06] bg-white/[0.01] p-8 text-center text-xs text-slate-500">
                Click any timeline event on the left to hydrate correlated system activity.
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Slide-in WhyPanel if an event is selected */}
      {selected && <WhyPanel event={selected} onClose={() => setId(null)} />}
    </div>
  );
}
