import { useEffect, useState } from "react";
import {
  Monitor,
  Server,
  Wifi,
  WifiOff,
  RefreshCw,
  Layers,
  Cpu,
  MemoryStick,
  ShieldCheck,
} from "lucide-react";
import type { Live } from "../useLive";
import type { FleetAgent, Snapshot } from "../types";
import { getJson, formatBytes } from "../lib";
import PageHeader from "../components/PageHeader";
import { EmptyState, StatusBadge, StatTile } from "../components/ui";

export default function Fleet({ live }: { live: Live }) {
  const [agents, setAgents] = useState<FleetAgent[]>([]);
  const [selectedAgent, setSelectedAgent] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchFleet = () => {
    setLoading(true);
    void getJson<FleetAgent[]>("/fleet/agents")
      .then(setAgents)
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    fetchFleet();
    const interval = setInterval(fetchFleet, 15000);
    return () => clearInterval(interval);
  }, []);

  const inspectAgent = (address: string) => {
    setSelectedAgent(address);
    setSnapshot(null);
    void getJson<Snapshot>(`/fleet/${address}/snapshot`)
      .then(setSnapshot)
      .catch(() => {});
  };

  const localHost = live.snapshot?.device.hostname ?? "Local Node";

  return (
    <div className="space-y-6">
      <PageHeader
        icon={Monitor}
        iconColor="text-indigo-400"
        iconBg="bg-indigo-500/15"
        iconBorder="border-indigo-500/25"
        category="DEVICES"
        title="Fleet Mesh & Remote Nodes"
        subtitle="Federated multi-machine telemetry — inspect remote Vigilon instances across your private subnet"
        actions={
          <button
            type="button"
            onClick={fetchFleet}
            disabled={loading}
            className="flex items-center gap-1.5 rounded-full bg-white/[0.04] px-3 py-1.5 text-xs font-medium text-slate-300 border border-white/[0.08] hover:bg-white/[0.08] cursor-pointer"
          >
            <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin text-indigo-400" : ""}`} />
            <span>Refresh Mesh</span>
          </button>
        }
      />

      {/* Local Node Card */}
      <div className="glass-card rounded-3xl p-6 border-l-4 border-l-blue-500">
        <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
          <div className="flex items-center gap-3">
            <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-blue-500/20 border border-blue-500/30 text-blue-400">
              <Server className="h-6 w-6" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-base font-bold text-white">{localHost}</h3>
                <span className="rounded-full bg-emerald-500/10 px-2 py-0.5 text-[10px] font-semibold text-emerald-400 border border-emerald-500/20">
                  PRIMARY // LOCALHOST
                </span>
              </div>
              <p className="text-xs text-slate-400 mt-0.5">
                {live.snapshot?.device.os_name} • {live.snapshot?.device.cpu_model}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-6 text-xs font-mono">
            <div>
              <p className="text-[10px] uppercase text-slate-400">CPU Usage</p>
              <p className="text-emerald-400 font-semibold mt-0.5">
                {live.snapshot ? `${live.snapshot.cpu.usage_percent.toFixed(0)}%` : "—"}
              </p>
            </div>
            <div>
              <p className="text-[10px] uppercase text-slate-400">RAM Used</p>
              <p className="text-sky-400 font-semibold mt-0.5">
                {live.snapshot ? formatBytes(live.snapshot.memory.used_bytes) : "—"}
              </p>
            </div>
            <div>
              <p className="text-[10px] uppercase text-slate-400">Connections</p>
              <p className="text-purple-400 font-semibold mt-0.5">
                {live.connections.length} active
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Remote Nodes Grid */}
      <div className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-slate-400 flex items-center gap-2">
          <Layers className="h-4 w-4 text-slate-400" />
          Remote Fleet Nodes ({agents.length})
        </h3>

        {agents.length === 0 ? (
          <EmptyState
            icon={Monitor}
            title="No remote agents configured"
            hint="Add comma-separated remote node addresses to VIGILON_REMOTES or in Settings."
          />
        ) : (
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {agents.map((ag) => {
              const online = ag.status === "online";
              const isSelected = selectedAgent === ag.address;
              return (
                <button
                  key={ag.address}
                  type="button"
                  onClick={() => inspectAgent(ag.address)}
                  className={`glass-card rounded-3xl p-5 text-left border transition-all cursor-pointer ${
                    isSelected
                      ? "border-blue-500/50 shadow-[0_0_24px_rgba(59,130,246,0.25)]"
                      : "hover:border-white/[0.12]"
                  }`}
                >
                  <div className="flex items-center justify-between mb-3">
                    <span className="font-mono text-xs font-semibold text-white truncate">
                      {ag.address}
                    </span>
                    <StatusBadge tone={online ? "emerald" : "slate"}>
                      {online ? <Wifi className="h-3 w-3" aria-hidden="true" /> : <WifiOff className="h-3 w-3" aria-hidden="true" />}
                      {online ? "ONLINE" : "OFFLINE"}
                    </StatusBadge>
                  </div>

                  <p className="text-xs text-slate-400">
                    {online ? "Click to view live hardware telemetry" : "Agent unreachable"}
                  </p>
                </button>
              );
            })}
          </div>
        )}
      </div>

      {/* Remote Snapshot Detail Inspector */}
      {selectedAgent && snapshot && (
        <div className="glass-card rounded-3xl p-6 animate-in fade-in duration-200">
          <div className="flex items-center justify-between mb-5 border-b border-white/[0.08] pb-4">
            <div>
              <h3 className="text-base font-bold text-white">
                Remote Node: {snapshot.device.hostname}
              </h3>
              <p className="text-xs text-slate-400 font-mono">
                {selectedAgent} • {snapshot.device.os_name} {snapshot.device.os_version}
              </p>
            </div>
            <button
              type="button"
              onClick={() => setSelectedAgent(null)}
              className="text-xs text-slate-400 hover:text-white px-3 py-1 rounded-full bg-white/[0.04] border border-white/[0.08]"
            >
              Close Inspector
            </button>
          </div>

          <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
            <StatTile
              icon={Cpu}
              tint="emerald"
              label="CPU Usage"
              value={`${snapshot.cpu.usage_percent.toFixed(0)}%`}
              sub={snapshot.device.cpu_model}
            />
            <StatTile
              icon={MemoryStick}
              tint="sky"
              label="RAM Utilization"
              value={`${((snapshot.memory.used_bytes / snapshot.memory.total_bytes) * 100).toFixed(0)}%`}
              sub={`${formatBytes(snapshot.memory.used_bytes)} / ${formatBytes(snapshot.memory.total_bytes)}`}
            />
            <StatTile
              icon={Server}
              tint="purple"
              label="Disks"
              value={`${snapshot.disks.length} Drives`}
              sub="Healthy Volumes"
            />
            <StatTile
              icon={ShieldCheck}
              tint="blue"
              label="Firewall State"
              value={snapshot.firewall?.enabled ? "PROTECTED" : "STANDBY"}
              sub={snapshot.firewall?.profile ?? "Standard"}
            />
          </div>
        </div>
      )}
    </div>
  );
}
