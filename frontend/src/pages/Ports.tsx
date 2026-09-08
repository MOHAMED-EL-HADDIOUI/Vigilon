import { useMemo, useState, useRef, useEffect, Fragment } from "react";
import { useSearchParams } from "react-router-dom";
import {
  Radio,
  Search,
  ChevronRight,
  ChevronDown,
  History,
  Sparkles,
} from "lucide-react";
import type { Live } from "../useLive";
import { getJson } from "../lib";
import type { PortTransition } from "../types";
import PageHeader from "../components/PageHeader";
import ExportButton from "../components/ExportButton";

export default function Ports({ live }: { live: Live }) {
  const [params] = useSearchParams();
  const [q, setQ] = useState(() => params.get("port") ?? "");
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [history, setHistory] = useState<Record<string, PortTransition[]>>({});

  const knownPortsRef = useRef<Set<string>>(new Set());
  const newPortsRef = useRef<Set<string>>(new Set());

  useEffect(() => {
    const current = new Set(live.ports.map((p) => `${p.protocol}-${p.address}-${p.port}-${p.pid}`));
    for (const p of current) {
      if (!knownPortsRef.current.has(p)) {
        if (knownPortsRef.current.size > 0) {
          newPortsRef.current.add(p);
        }
        knownPortsRef.current.add(p);
      }
    }
  }, [live.ports]);

  const toggleExpand = async (key: string, port: number) => {
    const willExpand = !expanded[key];
    setExpanded((prev) => ({ ...prev, [key]: willExpand }));
    if (willExpand && !history[key]) {
      try {
        const data = await getJson<PortTransition[]>(`/ports/history?port=${port}`);
        setHistory((prev) => ({ ...prev, [key]: data }));
      } catch (err) {
        console.error("Failed to fetch port history", err);
      }
    }
  };

  const rows = useMemo(() => {
    const n = q.toLowerCase();
    return live.ports.filter(
      (p) =>
        !n ||
        String(p.port).includes(n) ||
        (p.process_name ?? "").toLowerCase().includes(n) ||
        p.protocol.toLowerCase().includes(n),
    );
  }, [live.ports, q]);

  const tcpCount = live.ports.filter((p) => p.protocol === "TCP").length;
  const udpCount = live.ports.filter((p) => p.protocol === "UDP").length;

  // Deep link: /ports?port=8080 expands the first matching row once its
  // history is available.
  const deepPort = params.get("port");
  useEffect(() => {
    if (!deepPort || rows.length === 0) return;
    const first = rows[0];
    const key = `${first.protocol}-${first.address}-${first.port}-${first.pid}`;
    setExpanded((prev) => (prev[key] ? prev : { ...prev, [key]: true }));
    if (!history[key]) {
      void getJson<PortTransition[]>(`/ports/history?port=${first.port}`)
        .then((data) => setHistory((prev) => ({ ...prev, [key]: data })))
        .catch(() => {});
    }
    // Runs when the port list hydrates; intentionally not on every keystroke.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [deepPort, live.ports]);

  return (
    <div className="space-y-6">
      {/* Top Header with Brand Logo & Page Icon */}
      <PageHeader
        icon={Radio}
        iconColor="text-amber-400"
        iconBg="bg-amber-500/15"
        iconBorder="border-amber-500/25"
        category="PORTS & SOCKETS"
        title="Listening Port Auditor"
        subtitle="Active socket bindings, owning binaries & historical transitions"
        actions={
          <div className="flex items-center gap-2 text-xs font-mono">
            <span className="rounded-full bg-emerald-500/10 px-3 py-1 text-emerald-400 border border-emerald-500/20">
              {tcpCount} TCP Ports
            </span>
            <span className="rounded-full bg-blue-500/10 px-3 py-1 text-blue-400 border border-blue-500/20">
              {udpCount} UDP Ports
            </span>
            <ExportButton endpoint="ports" label="Export Ports" />
          </div>
        }
      >
        {/* Search Field */}
        <div className="relative">
          <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 h-4 w-4 text-slate-400" />
          <input
            className="w-full rounded-2xl border border-white/[0.08] bg-black/40 pl-10 pr-4 py-2.5 text-xs text-white placeholder-slate-500 backdrop-blur-md focus:border-amber-500/50 focus:outline-none focus:ring-2 focus:ring-amber-500/20 transition-all"
            placeholder="Search port number, protocol, or owning process..."
            aria-label="Search listening ports"
            value={q}
            onChange={(e) => setQ(e.target.value)}
          />
        </div>
      </PageHeader>

      {/* Main Ports Table */}
      <div className="glass-card rounded-3xl overflow-hidden border border-white/[0.08]">
        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="bg-[#0c101a]/90 backdrop-blur-xl border-b border-white/[0.08]">
              <tr>
                <th scope="col" className="w-10 px-4 py-3"></th>
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  Port
                </th>
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  Protocol
                </th>
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  Bind Interface
                </th>
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  Process Owner
                </th>
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  PID
                </th>
                <th scope="col" className="px-4 py-3 font-semibold text-[11px] uppercase tracking-wider text-slate-400">
                  Path
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-white/[0.04]">
              {rows.map((p) => {
                const rowKey = `${p.protocol}-${p.address}-${p.port}-${p.pid}`;
                const isExpanded = !!expanded[rowKey];
                const isNew = newPortsRef.current.has(rowKey);
                const hist = history[rowKey];

                return (
                  <Fragment key={rowKey}>
                    <tr
                      className="hover:bg-white/[0.03] transition-colors cursor-pointer focus-visible:outline-2 focus-visible:outline-amber-400"
                      onClick={() => toggleExpand(rowKey, p.port)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          void toggleExpand(rowKey, p.port);
                        }
                      }}
                      tabIndex={0}
                      role="button"
                      aria-expanded={isExpanded}
                      aria-label={`${isExpanded ? "Collapse" : "Expand"} history for port ${p.port}`}
                    >
                      <td className="px-4 py-3 text-slate-500">
                        {isExpanded ? (
                          <ChevronDown className="h-4 w-4 text-amber-400" />
                        ) : (
                          <ChevronRight className="h-4 w-4 text-slate-500 hover:text-slate-300" />
                        )}
                      </td>
                      <td className="px-4 py-3">
                        <div className="flex items-center gap-2">
                          <span className="font-mono font-bold text-amber-400 text-sm">
                            :{p.port}
                          </span>
                          {isNew && (
                            <span className="flex items-center gap-1 rounded-full bg-emerald-500/15 px-2 py-0.5 text-[9px] font-bold text-emerald-400 border border-emerald-500/30 animate-pulse">
                              <Sparkles className="h-2.5 w-2.5" />
                              NEW
                            </span>
                          )}
                        </div>
                      </td>
                      <td className="px-4 py-3">
                        <span className="rounded-lg bg-white/[0.05] px-2 py-0.5 font-mono text-[10px] text-slate-300 border border-white/[0.06]">
                          {p.protocol}
                        </span>
                      </td>
                      <td className="px-4 py-3 font-mono text-slate-300">{p.address}</td>
                      <td className="px-4 py-3 font-semibold text-white">
                        {p.process_name ?? "system daemon"}
                      </td>
                      <td className="px-4 py-3 font-mono text-slate-400">{p.pid ?? "—"}</td>
                      <td className="px-4 py-3 max-w-sm truncate font-mono text-[11px] text-slate-400">
                        {p.process_path ?? "—"}
                      </td>
                    </tr>

                    {/* Expandable Port History Timeline */}
                    {isExpanded && (
                      <tr className="bg-black/40 border-t border-b border-white/[0.06]">
                        <td colSpan={7} className="p-4 pl-12">
                          <div className="rounded-2xl border border-white/[0.06] bg-[#0c101a]/70 p-4">
                            <div className="flex items-center gap-2 text-xs font-semibold text-slate-300 mb-3">
                              <History className="h-3.5 w-3.5 text-amber-400" />
                              <span>Transition History for Port :{p.port}</span>
                            </div>

                            {hist ? (
                              hist.length > 0 ? (
                                <ul className="relative border-l border-white/[0.08] ml-2 pl-4 space-y-2 text-xs">
                                  {hist.map((item, idx) => (
                                    <li key={idx} className="relative">
                                      <span
                                        className={`absolute -left-[21px] top-1 h-2.5 w-2.5 rounded-full border-2 border-[#0c101a] ${
                                          item.event_type.includes("OPEN")
                                            ? "bg-emerald-400"
                                            : "bg-rose-400"
                                        }`}
                                      />
                                      <div className="flex items-center gap-3">
                                        <span className="font-mono text-slate-400 text-[11px]">
                                          {item.timestamp.replace("T", " ").slice(0, 19)}
                                        </span>
                                        <span
                                          className={`font-semibold font-mono text-[11px] ${
                                            item.event_type.includes("OPEN")
                                              ? "text-emerald-400"
                                              : "text-rose-400"
                                          }`}
                                        >
                                          {item.event_type}
                                        </span>
                                        {item.process_name && (
                                          <span className="text-slate-300">
                                            ({item.process_name})
                                          </span>
                                        )}
                                      </div>
                                    </li>
                                  ))}
                                </ul>
                              ) : (
                                <p className="text-xs text-slate-500">
                                  No prior recorded state transitions for this port.
                                </p>
                              )
                            ) : (
                              <p className="text-xs text-slate-400 flex items-center gap-2">
                                <span className="inline-block h-3 w-3 animate-spin rounded-full border-2 border-amber-400 border-t-transparent" />
                                <span>Loading port event history...</span>
                              </p>
                            )}
                          </div>
                        </td>
                      </tr>
                    )}
                  </Fragment>
                );
              })}
              {rows.length === 0 && (
                <tr>
                  <td colSpan={7} className="text-center py-10 text-slate-500">
                    No matching open ports found.
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
