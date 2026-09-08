import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import {
  Layers,
  Search,
  Clock,
  ChevronRight,
} from "lucide-react";
import type { Live } from "../useLive";
import { getJson } from "../lib";
import type { ServiceFirstSeen } from "../types";
import PageHeader from "../components/PageHeader";
import { StatusBadge } from "../components/ui";

export default function Services({ live }: { live: Live }) {
  const [q, setQ] = useState("");
  const [selectedService, setSelectedService] = useState<string | null>(null);
  const [firstSeen, setFirstSeen] = useState<string | null>(null);

  const rows = useMemo(() => {
    const n = q.toLowerCase();
    return live.services.filter(
      (s) =>
        !n ||
        s.name.toLowerCase().includes(n) ||
        (s.display_name ?? "").toLowerCase().includes(n),
    );
  }, [live.services, q]);

  const runningCount = live.services.filter(
    (s) => s.status.toLowerCase().includes("run") || s.status.toLowerCase().includes("active"),
  ).length;

  const inspectService = async (name: string) => {
    setSelectedService(name);
    try {
      const r = await getJson<ServiceFirstSeen>(
        `/services/first-seen?name=${encodeURIComponent(name)}`,
      );
      setFirstSeen(r.first_seen ?? "Not in historical database yet");
    } catch {
      setFirstSeen("Unable to retrieve first-seen timestamp");
    }
  };

  return (
    <div className="space-y-6">
      {/* Top Header with Brand Logo & Page Icon */}
      <PageHeader
        icon={Layers}
        iconColor="text-indigo-400"
        iconBg="bg-indigo-500/15"
        iconBorder="border-indigo-500/25"
        category="SERVICES"
        title="System Services Inventory"
        subtitle="Background daemons, launch agents & service control baseline"
        actions={
          <div className="flex items-center gap-2 text-xs font-mono">
            <span className="rounded-full bg-emerald-500/10 px-3 py-1 text-emerald-400 border border-emerald-500/20">
              {runningCount} Running Daemons
            </span>
            <span className="rounded-full bg-white/[0.05] px-3 py-1 text-slate-300 border border-white/[0.08]">
              {live.services.length} Total Services
            </span>
          </div>
        }
      >
        {/* Search Field */}
        <div className="relative">
          <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 h-4 w-4 text-slate-400" />
          <input
            className="w-full rounded-2xl border border-white/[0.08] bg-black/40 pl-10 pr-4 py-2.5 text-xs text-white placeholder-slate-500 backdrop-blur-md focus:border-indigo-500/50 focus:outline-none focus:ring-2 focus:ring-indigo-500/20 transition-all"
            placeholder="Search service system name or display label..."
            aria-label="Search services"
            value={q}
            onChange={(e) => setQ(e.target.value)}
          />
        </div>
      </PageHeader>

      {/* Selected Service First Seen Banner */}
      {selectedService && (
        <div className="glass-card rounded-2xl p-4 flex items-center justify-between border-indigo-500/30 bg-indigo-500/[0.03]">
          <div className="flex items-center gap-3 text-xs">
            <Clock className="h-4 w-4 text-indigo-400" />
            <div>
              <span className="font-semibold text-white">{selectedService}</span>
              <span className="text-slate-400 ml-2">First sighted in SQLite store:</span>
              <span className="font-mono text-indigo-300 ml-1.5">{firstSeen ?? "Loading..."}</span>
            </div>
          </div>
          <button
            type="button"
            className="text-xs text-slate-400 hover:text-white"
            onClick={() => setSelectedService(null)}
            aria-label="Dismiss service inspector"
          >
            ✕
          </button>
        </div>
      )}

      {/* Services Table */}
      <div className="glass-card rounded-3xl overflow-hidden border border-white/[0.08]">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="bg-[#0c101a]/90 backdrop-blur-xl border-b border-white/[0.08]">
              <tr>
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  Service Name
                </th>
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  Display Name
                </th>
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  Status
                </th>
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  PID
                </th>
                <th scope="col" className="px-4 py-3 text-right font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  History
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-white/[0.04]">
              {rows.map((s) => {
                const isRunning =
                  s.status.toLowerCase().includes("run") ||
                  s.status.toLowerCase().includes("active");

                return (
                  <tr
                    key={s.name}
                    className="hover:bg-white/[0.03] transition-colors cursor-pointer focus-visible:outline-2 focus-visible:outline-blue-400"
                    onClick={() => inspectService(s.name)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" || e.key === " ") {
                        e.preventDefault();
                        inspectService(s.name);
                      }
                    }}
                    tabIndex={0}
                    role="button"
                    aria-label={`Inspect service ${s.name}`}
                  >
                    <td className="px-4 py-3 font-mono font-semibold text-white">
                      <Link
                        to={`/timeline?q=${encodeURIComponent(s.name)}`}
                        className="hover:text-sky-300 hover:underline"
                        title={`Find ${s.name} in Timeline`}
                      >
                        {s.name}
                      </Link>
                    </td>
                    <td className="px-4 py-3 font-medium text-slate-300">
                      {s.display_name ?? "—"}
                    </td>
                    <td className="px-4 py-3">
                      <StatusBadge tone={isRunning ? "emerald" : "slate"}>
                        <span
                          aria-hidden="true"
                          className={`h-1.5 w-1.5 rounded-full ${
                            isRunning ? "bg-emerald-400" : "bg-slate-500"
                          }`}
                        />
                        {s.status}
                      </StatusBadge>
                    </td>
                    <td className="px-4 py-3 font-mono text-slate-400">{s.pid ?? "—"}</td>
                    <td className="px-4 py-3 text-right">
                      <ChevronRight className="inline-block h-4 w-4 text-slate-500 hover:text-slate-300 transition-colors" />
                    </td>
                  </tr>
                );
              })}
              {rows.length === 0 && (
                <tr>
                  <td colSpan={5} className="text-center py-10 text-slate-500">
                    No matching services found.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
