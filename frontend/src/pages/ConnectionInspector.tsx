import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import {
  PackageSearch,
  Activity,
  Search,
  Clock,
  Laptop,
} from "lucide-react";
import type { Live } from "../useLive";
import type { Connection } from "../types";
import { getJson, timeAgo } from "../lib";
import { countryFlag } from "../lib/flags";
import PageHeader from "../components/PageHeader";

export default function ConnectionInspector({ live }: { live: Live }) {
  const [selected, setSelected] = useState<Connection | null>(null);
  const [history, setHistory] = useState<Connection[]>([]);
  const [search, setSearch] = useState("");
  const [loadingHistory, setLoadingHistory] = useState(false);
  const [params] = useSearchParams();
  const deepIp = params.get("ip");

  useEffect(() => {
    if (deepIp) {
      const match = live.connections.find((c) => c.remote_addr === deepIp);
      if (match) {
        setSelected(match);
        return;
      }
    }
    if (!selected && live.connections.length > 0) {
      setSelected(live.connections[0]);
    }
  }, [live.connections]);

  useEffect(() => {
    if (selected) {
      setLoadingHistory(true);
      void getJson<Connection[]>(`/connections/${selected.remote_addr}/history`)
        .then(setHistory)
        .catch(() => setHistory([]))
        .finally(() => setLoadingHistory(false));
    }
  }, [selected?.remote_addr]);

  const filteredConnections = useMemo(() => {
    const s = search.toLowerCase().trim();
    if (!s) return live.connections;
    return live.connections.filter(
      (c) =>
        c.remote_addr.toLowerCase().includes(s) ||
        String(c.remote_port).includes(s) ||
        (c.process_name?.toLowerCase().includes(s) ?? false),
    );
  }, [live.connections, search]);

  return (
    <div className="space-y-6">
      <PageHeader
        icon={PackageSearch}
        iconColor="text-rose-400"
        iconBg="bg-rose-500/15"
        iconBorder="border-rose-500/25"
        category="NETWORK"
        title="Deep Connection Inspector"
        subtitle="Full socket lifecycle telemetry, packet-level metadata & historical destination tracking"
        actions={
          <div className="text-xs font-mono text-slate-400">
            <span className="rounded-full bg-rose-500/10 px-3 py-1 text-rose-400 border border-rose-500/20">
              ● {live.connections.length} Active Sockets
            </span>
          </div>
        }
      />

      <div className="grid gap-6 lg:grid-cols-12">
        {/* Left: Searchable Connections List */}
        <div className="lg:col-span-5 glass-card rounded-3xl p-5 flex flex-col h-[min(650px,70vh)]">
          <div className="relative mb-4">
            <Search className="absolute left-3 top-2.5 h-3.5 w-3.5 text-slate-400" />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search IP, port, or process..."
              aria-label="Search connections"
              className="w-full rounded-2xl bg-white/[0.04] pl-9 pr-4 py-2 text-xs text-white placeholder-slate-500 border border-white/[0.08] focus:border-rose-500/50 focus:outline-none font-mono"
            />
          </div>

          <div className="flex-1 overflow-y-auto space-y-2 pr-1">
            {filteredConnections.map((c, i) => {
              const isSelected =
                selected?.remote_addr === c.remote_addr &&
                selected?.remote_port === c.remote_port &&
                selected?.local_port === c.local_port;
              const flag = countryFlag(c.geo?.country_code);

              return (
                <button
                  key={`${c.protocol}-${c.local_port}-${c.remote_addr}-${c.remote_port}-${i}`}
                  type="button"
                  onClick={() => setSelected(c)}
                  className={`flex w-full items-center justify-between p-3 rounded-2xl border text-left transition-all cursor-pointer ${
                    isSelected
                      ? "bg-rose-500/15 border-rose-500/40 text-white shadow-[0_0_20px_rgba(244,63,94,0.15)]"
                      : "bg-white/[0.02] border-white/[0.05] text-slate-300 hover:bg-white/[0.05]"
                  }`}
                >
                  <div className="flex items-center gap-2.5 truncate">
                    <span className="text-base">{flag}</span>
                    <div className="truncate">
                      <p className="font-mono text-xs font-semibold text-white truncate">
                        {c.remote_addr}:{c.remote_port}
                      </p>
                      <p className="text-[10px] text-slate-400 truncate">
                        {c.process_name ?? "system"} • PID {c.pid ?? "—"}
                      </p>
                    </div>
                  </div>

                  <span className="rounded-md bg-white/[0.06] px-1.5 py-0.5 text-[10px] font-mono text-slate-400 border border-white/[0.06] shrink-0">
                    {c.protocol}
                  </span>
                </button>
              );
            })}
            {filteredConnections.length === 0 && (
              <p className="text-center text-xs text-slate-500 py-10">No connections match</p>
            )}
          </div>
        </div>

        {/* Right: Deep Socket Inspector */}
        <div className="lg:col-span-7 glass-card rounded-3xl p-6 flex flex-col justify-between">
          {selected ? (
            <div className="space-y-6">
              {/* Header */}
              <div className="flex items-start justify-between border-b border-white/[0.08] pb-4">
                <div>
                  <div className="flex items-center gap-2">
                    <span className="text-2xl">{countryFlag(selected.geo?.country_code)}</span>
                    <h3 className="text-lg font-bold text-white font-mono">
                      {selected.remote_addr}:{selected.remote_port}
                    </h3>
                  </div>
                  <p className="text-xs text-slate-400 mt-1">
                    {selected.geo ? `${selected.geo.country_name}${selected.geo.city ? ` • ${selected.geo.city}` : ""}` : "Local / Private Network"}
                  </p>
                </div>
                <span className="rounded-full bg-emerald-500/10 px-3 py-1 text-xs font-mono text-emerald-400 border border-emerald-500/20">
                  {selected.state}
                </span>
              </div>

              {/* L4 Metadata Bento Grid */}
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-xs font-mono">
                <div className="rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05]">
                  <span className="text-[10px] font-sans uppercase text-slate-400">Protocol</span>
                  <p className="text-sm font-bold text-white mt-1">{selected.protocol}</p>
                </div>
                <div className="rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05]">
                  <span className="text-[10px] font-sans uppercase text-slate-400">Local Port</span>
                  <p className="text-sm font-bold text-sky-400 mt-1">:{selected.local_port}</p>
                </div>
                <div className="rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05]">
                  <span className="text-[10px] font-sans uppercase text-slate-400">Remote Port</span>
                  <p className="text-sm font-bold text-rose-400 mt-1">:{selected.remote_port}</p>
                </div>
                <div className="rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05]">
                  <span className="text-[10px] font-sans uppercase text-slate-400">Process PID</span>
                  <p className="text-sm font-bold text-purple-400 mt-1">{selected.pid ?? "System"}</p>
                </div>
              </div>

              {/* Process Card */}
              <div className="rounded-2xl bg-white/[0.03] p-4 border border-white/[0.06]">
                <div className="flex items-center gap-2 mb-2 text-slate-300 text-xs font-semibold">
                  <Laptop className="h-4 w-4 text-blue-400" />
                  <span>Attributed Application</span>
                </div>
                <p className="text-sm font-bold text-white">{selected.process_name ?? "System Kernel"}</p>
                {selected.process_path && (
                  <p className="text-[11px] font-mono text-slate-400 mt-1 break-all">
                    {selected.process_path}
                  </p>
                )}
              </div>

              {/* Historical Sightings Timeline */}
              <div>
                <div className="flex items-center justify-between mb-3">
                  <span className="text-xs font-semibold text-slate-300 flex items-center gap-1.5">
                    <Clock className="h-3.5 w-3.5 text-amber-400" />
                    Historical Destination Sightings
                  </span>
                  <span className="text-[10px] text-slate-500 font-mono">
                    {loadingHistory ? "Loading..." : `${history.length} sightings recorded`}
                  </span>
                </div>

                <div className="max-h-48 overflow-y-auto space-y-1.5 rounded-2xl bg-white/[0.02] p-2 border border-white/[0.04]">
                  {history.map((h, idx) => (
                    <div
                      key={idx}
                      className="flex items-center justify-between p-2 rounded-xl bg-white/[0.02] text-xs font-mono border border-white/[0.03]"
                    >
                      <span className="text-slate-300">
                        {h.local_addr}:{h.local_port} → :{h.remote_port}
                      </span>
                      <span className="text-slate-500 text-[11px]">
                        {h.last_seen ? timeAgo(h.last_seen) : "Recent"}
                      </span>
                    </div>
                  ))}
                  {history.length === 0 && !loadingHistory && (
                    <p className="text-center py-4 text-xs text-slate-500">
                      No prior historical sightings stored for this remote IP
                    </p>
                  )}
                </div>
              </div>
            </div>
          ) : (
            <div className="text-center py-24 text-slate-500">
              <Activity className="h-10 w-10 mx-auto mb-3 text-slate-600" />
              <p>Select a socket connection on the left to inspect</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
