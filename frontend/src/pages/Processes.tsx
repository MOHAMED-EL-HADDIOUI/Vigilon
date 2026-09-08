import { useEffect, useMemo, useState } from "react";
import {
  Cpu,
  Search,
  ArrowUpDown,
  ChevronUp,
  ChevronDown,
  Terminal,
} from "lucide-react";
import type { Live } from "../useLive";
import type { ProcessBandwidth, ProcessInfo, ProcessSortKey } from "../types";
import { formatBytes, getJson } from "../lib";
import PageHeader from "../components/PageHeader";

export default function Processes({ live }: { live: Live }) {
  const [q, setQ] = useState("");
  const [sortKey, setSortKey] = useState<ProcessSortKey>("cpu_percent");
  const [sortDir, setSortDir] = useState<"asc" | "desc">("desc");
  const [procBw, setProcBw] = useState<ProcessBandwidth[]>([]);

  useEffect(() => {
    void getJson<ProcessBandwidth[]>("/process-bandwidth").then(setProcBw).catch(() => {});
  }, [live.snapshot?.timestamp]);

  const bwByPid = useMemo(() => {
    const m = new Map<number, ProcessBandwidth>();
    for (const b of procBw) m.set(b.pid, b);
    return m;
  }, [procBw]);

  const rows = useMemo(() => {
    const n = q.toLowerCase();
    let filtered = live.processes.filter(
      (p: ProcessInfo) => !n || p.name.toLowerCase().includes(n) || (p.path ?? "").toLowerCase().includes(n),
    );

    filtered.sort((a: ProcessInfo, b: ProcessInfo) => {
      const valA = a[sortKey];
      const valB = b[sortKey];

      if (typeof valA === "string" && typeof valB === "string") {
        const cmp = valA.localeCompare(valB);
        return sortDir === "asc" ? cmp : -cmp;
      }
      if (typeof valA === "number" && typeof valB === "number") {
        return sortDir === "asc" ? valA - valB : valB - valA;
      }
      return 0;
    });

    return filtered;
  }, [live.processes, q, sortKey, sortDir]);

  const handleSort = (key: ProcessSortKey) => {
    if (sortKey === key) {
      setSortDir(sortDir === "asc" ? "desc" : "asc");
    } else {
      setSortKey(key);
      setSortDir("desc");
    }
  };

  const Header = ({ colKey, label }: { colKey: ProcessSortKey; label: string }) => (
    <th
      scope="col"
      className="cursor-pointer select-none px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400 hover:text-white transition-colors"
      onClick={() => handleSort(colKey)}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          handleSort(colKey);
        }
      }}
      tabIndex={0}
      aria-sort={
        sortKey === colKey ? (sortDir === "asc" ? "ascending" : "descending") : "none"
      }
    >
      <div className="flex items-center gap-1.5">
        <span>{label}</span>
        {sortKey === colKey ? (
          sortDir === "asc" ? (
            <ChevronUp className="h-3.5 w-3.5 text-blue-400" />
          ) : (
            <ChevronDown className="h-3.5 w-3.5 text-blue-400" />
          )
        ) : (
          <ArrowUpDown className="h-3 w-3 text-slate-600 opacity-50" />
        )}
      </div>
    </th>
  );

  return (
    <div className="space-y-6">
      {/* Top Apple Activity Monitor Header & Controls */}
      <PageHeader
        icon={Cpu}
        iconColor="text-purple-400"
        iconBg="bg-purple-500/15"
        iconBorder="border-purple-500/25"
        category="PROCESSES"
        title="Activity Monitor"
        subtitle="Live thread, processor & memory allocation per binary"
        actions={
          <span className="rounded-full bg-purple-500/10 px-3 py-1 text-xs font-mono text-purple-300 border border-purple-500/20">
            {rows.length} Tasks Listed
          </span>
        }
      >
        {/* Apple Style Search Field */}
        <div className="relative">
          <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 h-4 w-4 text-slate-400" />
          <input
            className="w-full rounded-2xl border border-white/[0.08] bg-black/40 pl-10 pr-4 py-2.5 text-xs text-white placeholder-slate-500 backdrop-blur-md focus:border-blue-500/50 focus:outline-none focus:ring-2 focus:ring-blue-500/20 transition-all"
            placeholder="Search processes by binary name, argument, or executable path..."
            aria-label="Search processes"
            value={q}
            onChange={(e) => setQ(e.target.value)}
          />
        </div>
      </PageHeader>

      {/* Main Process Table */}
      <div className="glass-card rounded-3xl overflow-hidden border border-white/[0.08]">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="bg-[#0c101a]/90 backdrop-blur-xl border-b border-white/[0.08]">
              <tr>
                <Header colKey="name" label="Process Name" />
                <Header colKey="pid" label="PID" />
                <Header colKey="cpu_percent" label="CPU %" />
                <Header colKey="memory_bytes" label="Memory" />
                <Header colKey="connection_count" label="Sockets" />
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  Rx ↓
                </th>
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  Tx ↑
                </th>
                <Header colKey="status" label="Status" />
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  Binary Path
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-white/[0.04]">
              {rows.map((p) => {
                const hasNet = p.connection_count > 0;
                return (
                  <tr
                    key={p.pid}
                    className={`hover:bg-white/[0.03] transition-colors ${
                      hasNet ? "bg-blue-500/[0.015]" : ""
                    }`}
                  >
                    <td className="px-4 py-3 font-medium text-white">
                      <div className="flex items-center gap-2">
                        <Terminal className="h-3.5 w-3.5 text-slate-400" />
                        <span className="font-semibold text-slate-100">{p.name}</span>
                        {hasNet && (
                          <span className="rounded-full bg-blue-500/10 px-1.5 py-0.2 text-[9px] font-mono text-blue-400 border border-blue-500/20">
                            NET
                          </span>
                        )}
                      </div>
                    </td>
                    <td className="px-4 py-3 font-mono text-slate-400">{p.pid}</td>
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-slate-200 w-12 font-medium">
                          {p.cpu_percent.toFixed(1)}%
                        </span>
                        <div className="w-16 h-1.5 rounded-full bg-white/[0.06] overflow-hidden">
                          <div
                            className="h-full bg-gradient-to-r from-purple-400 to-indigo-400 rounded-full"
                            style={{ width: `${Math.min(100, p.cpu_percent)}%` }}
                          />
                        </div>
                      </div>
                    </td>
                    <td className="px-4 py-3 font-mono text-slate-300">
                      {formatBytes(p.memory_bytes)}
                    </td>
                    <td className="px-4 py-3">
                      {p.connection_count > 0 ? (
                        <span className="rounded-full bg-blue-500/15 px-2 py-0.5 font-mono text-[11px] text-blue-300 border border-blue-500/20">
                          {p.connection_count} open
                        </span>
                      ) : (
                        <span className="text-slate-500 font-mono">—</span>
                      )}
                    </td>
                    <td className="px-4 py-3 font-mono text-[11px] text-teal-300">
                      {bwByPid.get(p.pid) ? formatBytes(bwByPid.get(p.pid)!.rx_bytes) : "—"}
                    </td>
                    <td className="px-4 py-3 font-mono text-[11px] text-sky-300">
                      {bwByPid.get(p.pid) ? formatBytes(bwByPid.get(p.pid)!.tx_bytes) : "—"}
                    </td>
                    <td className="px-4 py-3">
                      <span className="rounded bg-white/[0.05] px-2 py-0.5 text-[10px] uppercase font-mono text-slate-300">
                        {p.status}
                      </span>
                    </td>
                    <td className="px-4 py-3 max-w-sm truncate font-mono text-[11px] text-slate-400">
                      {p.path ?? "—"}
                    </td>
                  </tr>
                );
              })}
              {rows.length === 0 && (
                <tr>
                  <td colSpan={9} className="text-center py-10 text-slate-500">
                    No matching processes found.
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
