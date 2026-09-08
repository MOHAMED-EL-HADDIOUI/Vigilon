import { useEffect, useState } from "react";
import {
  History as HistoryIcon,
  Radio,
  Globe,
  ShieldAlert,
  Layers,
  Network,
} from "lucide-react";
import { getJson } from "../lib";
import type {
  ChangesSummary,
  RemoteIpSighting,
  DnsCacheEntry,
  PortTransition,
} from "../types";
import PageHeader from "../components/PageHeader";
import ExportButton from "../components/ExportButton";
import { StatTile } from "../components/ui";

const HOUR_PRESETS = [
  { label: "1 Hour", value: 1 },
  { label: "6 Hours", value: 6 },
  { label: "24 Hours", value: 24 },
  { label: "7 Days", value: 168 },
];

export default function History() {
  const [hours, setHours] = useState(24);
  const [changes, setChanges] = useState<ChangesSummary | null>(null);
  const [remotes, setRemotes] = useState<RemoteIpSighting[]>([]);
  const [dns, setDns] = useState<DnsCacheEntry[]>([]);
  const [port, setPort] = useState("8080");
  const [portHist, setPortHist] = useState<PortTransition[]>([]);

  useEffect(() => {
    void getJson<ChangesSummary>(`/changes?hours=${hours}`).then(setChanges).catch(() => {});
    void getJson<RemoteIpSighting[]>("/remotes").then(setRemotes).catch(() => {});
    void getJson<DnsCacheEntry[]>("/dns").then(setDns).catch(() => {});
  }, [hours]);

  const handlePortLookup = (e: React.FormEvent) => {
    e.preventDefault();
    void getJson<PortTransition[]>(`/ports/history?port=${port}`)
      .then(setPortHist)
      .catch(() => setPortHist([]));
  };

  return (
    <div className="space-y-6">
      {/* Top Header with Brand Logo & Page Icon */}
      <PageHeader
        icon={HistoryIcon}
        iconColor="text-teal-400"
        iconBg="bg-teal-500/15"
        iconBorder="border-teal-500/25"
        category="AUDIT & DELTAS"
        title="System Changes Differential"
        subtitle="Time-window delta: newly opened ports, services & external remotes"
        actions={
          <div className="flex items-center gap-2">
            <ExportButton endpoint="events" label="Export" />
            <div className="flex items-center gap-1.5 rounded-2xl bg-white/[0.03] p-1.5 border border-white/[0.06]">
              {HOUR_PRESETS.map((p) => (
              <button
                key={p.value}
                type="button"
                onClick={() => setHours(p.value)}
                className={`rounded-xl px-3 py-1.5 text-xs font-medium transition-all ${
                  hours === p.value
                    ? "bg-white/[0.12] text-white shadow-[0_2px_8px_rgba(0,0,0,0.4)] border border-white/[0.12]"
                    : "text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]"
                }`}
              >
                {p.label}
              </button>
            ))}
            </div>
          </div>
        }
      >
        {/* 4 Bento Delta Cards */}
        {changes ? (
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-4 pt-1">
            <StatTile icon={Radio} tint="amber" label="Port Diffs" value={changes.port_transitions} sub="Bound / Closed" />
            <StatTile icon={ShieldAlert} tint="rose" label="Security Signals" value={changes.security_events} sub="Observed findings" />
            <StatTile icon={Layers} tint="indigo" label="Service Events" value={changes.service_events} sub="Daemon state shifts" />
            <StatTile icon={Globe} tint="sky" label="Remote IPs" value={changes.distinct_remotes} sub="Distinct hosts connected" />
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-4 sm:grid-cols-4 pt-1" aria-label="Loading deltas">
            {Array.from({ length: 4 }).map((_, i) => (
              <div key={i} className="h-24 animate-pulse rounded-2xl border border-white/[0.06] bg-white/[0.02]" aria-hidden="true" />
            ))}
          </div>
        )}
      </PageHeader>

      <div className="grid gap-6 lg:grid-cols-2">
        {/* Remote IPs (First / Last Seen) */}
        <div className="glass-card rounded-3xl p-6">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <Globe className="h-4 w-4 text-sky-400" />
              <h3 className="text-sm font-semibold text-white">External Remote IPs</h3>
            </div>
            <span className="rounded-full bg-sky-500/10 px-2.5 py-0.5 text-xs font-mono text-sky-400 border border-sky-500/20">
              {remotes.length} Hosts
            </span>
          </div>

          <div className="max-h-80 overflow-y-auto">
            <table className="w-full text-left text-xs">
              <thead className="sticky top-0 bg-[#0c101a] text-[11px] uppercase tracking-wider text-slate-400 border-b border-white/[0.08]">
                <tr>
                  <th scope="col" className="pb-2">IP Address</th>
                  <th scope="col" className="pb-2">First Observed</th>
                  <th scope="col" className="pb-2 text-right">Latest Sighting</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-white/[0.04] font-mono text-[11px]">
                {remotes.map((r) => (
                  <tr key={r.ip} className="hover:bg-white/[0.02]">
                    <td className="py-2 text-slate-200">{r.ip}</td>
                    <td className="py-2 text-slate-400">
                      {r.first_seen.replace("T", " ").slice(0, 19)}
                    </td>
                    <td className="py-2 text-right text-slate-300">
                      {r.last_seen.replace("T", " ").slice(0, 19)}
                    </td>
                  </tr>
                ))}
                {remotes.length === 0 && (
                  <tr>
                    <td colSpan={3} className="text-center py-8 text-slate-500">
                      No remote host records in selected interval.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>

        {/* Port Investigator & DNS Cache */}
        <div className="space-y-6">
          {/* Quick Port Investigator */}
          <div className="glass-card rounded-3xl p-6">
            <div className="flex items-center gap-2 mb-3">
              <Radio className="h-4 w-4 text-amber-400" />
              <h3 className="text-sm font-semibold text-white">Port Transition Investigator</h3>
            </div>
            <p className="text-xs text-slate-400 mb-4">
              Query historical records to see which binary originally bound a specific port number.
            </p>

            <form onSubmit={handlePortLookup} className="flex gap-2">
              <input
                className="flex-1 rounded-2xl border border-white/[0.08] bg-black/40 px-4 py-2 text-xs font-mono text-white placeholder-slate-500 focus:outline-none focus:border-amber-500/50"
                placeholder="e.g. 8080 or 3000"
                aria-label="Port number to investigate"
                value={port}
                onChange={(e) => setPort(e.target.value)}
              />
              <button
                type="submit"
                className="rounded-2xl bg-white/[0.1] px-4 py-2 text-xs font-semibold text-white hover:bg-white/[0.16] transition-colors border border-white/[0.08]"
              >
                Inspect
              </button>
            </form>

            {portHist.length > 0 && (
              <ul className="mt-4 space-y-1.5 font-mono text-xs max-h-40 overflow-y-auto">
                {portHist.map((row, i) => (
                  <li
                    key={i}
                    className="flex items-center justify-between rounded-xl bg-white/[0.02] p-2 border border-white/[0.04]"
                  >
                    <span className="text-slate-400 text-[11px]">
                      {row.timestamp.replace("T", " ").slice(0, 19)}
                    </span>
                    <span className="text-amber-400 font-semibold">{row.event_type}</span>
                    <span className="text-slate-200 font-sans">{row.process_name ?? "—"}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>

          {/* DNS Resolver Cache */}
          <div className="glass-card rounded-3xl p-6">
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <Network className="h-4 w-4 text-emerald-400" />
                <h3 className="text-sm font-semibold text-white">Local DNS Resolver Cache</h3>
              </div>
              <span className="rounded-full bg-emerald-500/10 px-2 py-0.5 text-[10px] font-mono text-emerald-400">
                {dns.length} Entries
              </span>
            </div>

            <ul className="max-h-44 overflow-y-auto space-y-1 font-mono text-xs">
              {dns.slice(0, 50).map((d, i) => (
                <li
                  key={i}
                  className="flex items-center justify-between rounded-lg bg-white/[0.02] p-1.5 border border-white/[0.03]"
                >
                  <span className="text-slate-300 truncate max-w-[240px]">{d.name}</span>
                  <span className="rounded bg-white/[0.06] px-1.5 py-0.2 text-[9px] text-slate-400">
                    {d.record_type}
                  </span>
                </li>
              ))}
              {dns.length === 0 && (
                <li className="text-slate-500 text-center py-6">
                  No cached resolver records (platform or permission constrained).
                </li>
              )}
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
