import { useEffect, useState } from "react";
import {
  Laptop,
  ShieldCheck,
  Cpu,
  Battery,
  Server,
  CheckCircle2,
  Database,
  Palette,
  Sun,
  Moon,
  Monitor,
  Ban,
  Plus,
  Trash2,
  Gauge,
  Network,
} from "lucide-react";
import type { Live } from "../useLive";
import { apiUrl, getJson, formatBytes } from "../lib";
import type { BackendMeta, BandwidthQuotaInfo, BlocklistEntry, FleetAgent, Theme } from "../types";
import { useTheme } from "../hooks/useTheme";
import VigilonLogo from "../components/VigilonLogo";

interface AgentConfigView {
  bandwidth_quota_gb?: number | null;
  bandwidth_warning_pct: number;
  notifications_enabled: boolean;
  notification_min_risk: string;
  remote_agents: string[];
}

export default function Settings({ live }: { live: Live }) {
  const [meta, setMeta] = useState<BackendMeta | null>(null);
  const [blocklist, setBlocklist] = useState<BlocklistEntry[]>([]);
  const [newIp, setNewIp] = useState("");
  const [newComment, setNewComment] = useState("");
  const [quota, setQuota] = useState<BandwidthQuotaInfo | null>(null);
  const [agentConfig, setAgentConfig] = useState<AgentConfigView | null>(null);
  const [fleet, setFleet] = useState<FleetAgent[]>([]);

  const refreshBlocklist = () => {
    void getJson<BlocklistEntry[]>("/blocklist").then(setBlocklist).catch(() => {});
  };

  useEffect(() => {
    void getJson<BackendMeta>("/health").then(setMeta).catch(() => {});
    refreshBlocklist();
    void getJson<BandwidthQuotaInfo>("/bandwidth").then(setQuota).catch(() => {});
    void getJson<AgentConfigView>("/config").then(setAgentConfig).catch(() => {});
    void getJson<FleetAgent[]>("/fleet/agents").then(setFleet).catch(() => {});
  }, []);

  const addBlock = async (e: React.FormEvent) => {
    e.preventDefault();
    const ip = newIp.trim();
    if (!ip) return;
    try {
      await fetch(apiUrl("/blocklist"), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ip, comment: newComment.trim() || null }),
      });
      setNewIp("");
      setNewComment("");
      refreshBlocklist();
    } catch {
      /* offline — leave form intact */
    }
  };

  const removeBlock = async (ip: string) => {
    try {
      await fetch(apiUrl(`/blocklist/${encodeURIComponent(ip)}`), { method: "DELETE" });
      refreshBlocklist();
    } catch {
      /* ignore */
    }
  };

  const d = live.snapshot?.device;
  const bat = live.snapshot?.battery;

  const { theme, setTheme } = useTheme();

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      {/* Apple-grade Appearance Theme Card */}
      <div className="glass-card rounded-3xl p-6">
        <div className="flex items-center gap-2.5 mb-4 text-sky-400">
          <Palette className="h-5 w-5" />
          <h3 className="text-sm font-semibold text-white">Appearance & Theme</h3>
        </div>
        <p className="text-xs text-slate-400 mb-4">
          Choose your interface appearance preference with Apple VisionOS-grade glass aesthetics.
        </p>
        <div className="grid grid-cols-3 gap-3">
          {(
            [
              { id: "dark" as Theme, label: "Dark Space", icon: Moon, desc: "OLED contrast & neon glows" },
              { id: "light" as Theme, label: "Light Studio", icon: Sun, desc: "Bright paper & subtle frosting" },
              { id: "system" as Theme, label: "Auto System", icon: Monitor, desc: "Synchronize with OS theme" },
            ] as const
          ).map((item) => {
            const Icon = item.icon;
            const active = theme === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => setTheme(item.id)}
                className={`flex flex-col items-center sm:items-start text-center sm:text-left p-4 rounded-2xl border transition-all cursor-pointer ${
                  active
                    ? "bg-blue-500/15 border-blue-500/40 text-white shadow-[0_0_20px_rgba(59,130,246,0.2)]"
                    : "bg-white/[0.02] border-white/[0.06] text-slate-400 hover:bg-white/[0.05] hover:text-slate-200"
                }`}
              >
                <div className="flex items-center gap-2 mb-2">
                  <Icon className={`h-4 w-4 ${active ? "text-blue-400" : "text-slate-400"}`} />
                  <span className="text-xs font-semibold text-white">{item.label}</span>
                </div>
                <span className="text-[11px] text-slate-400">{item.desc}</span>
              </button>
            );
          })}
        </div>
      </div>
      {/* Apple "About This Mac" Hero Card */}
      <div className="glass-card rounded-3xl p-8 relative overflow-hidden">
        <div className="absolute -top-10 -right-10 h-44 w-44 rounded-full bg-blue-500/10 blur-3xl pointer-events-none" />

        <div className="flex flex-col sm:flex-row items-center sm:items-start gap-6 text-center sm:text-left">
          <div className="relative flex h-20 w-20 shrink-0 items-center justify-center rounded-3xl bg-gradient-to-br from-blue-500/20 via-indigo-500/10 to-purple-500/20 border border-white/[0.12] shadow-[0_8px_32px_rgba(59,130,246,0.25)]">
            <Laptop className="h-10 w-10 text-blue-400" />
            <div className="absolute -bottom-2 -right-2 flex h-8 w-8 items-center justify-center rounded-xl bg-[#0c101a] border border-white/[0.12] shadow-lg">
              <VigilonLogo size={18} showGlow={false} />
            </div>
          </div>

          <div className="space-y-2 flex-1">
            <div className="flex items-center justify-center sm:justify-start gap-2 mb-1">
              <VigilonLogo size="sm" />
              <span className="text-[10px] font-semibold tracking-[0.2em] uppercase text-blue-400/90 font-mono">
                VIGILON // SYSTEM CONFIG & SPECIFICATIONS
              </span>
            </div>

            <div className="flex flex-wrap items-center justify-center sm:justify-start gap-2.5">
              <h2 className="text-2xl font-bold tracking-tight text-white">
                {d?.hostname ?? "Local Machine"}
              </h2>
              <span className="rounded-full bg-blue-500/10 px-3 py-0.5 text-xs font-semibold text-blue-400 border border-blue-500/20">
                {d?.os_name ?? "Operating System"}
              </span>
            </div>

            <p className="text-sm text-slate-300 font-medium">
              {d?.os_name} {d?.os_version} {d?.kernel ? `(Kernel ${d.kernel})` : ""}
            </p>

            <div className="pt-2 flex flex-wrap justify-center sm:justify-start gap-4 text-xs text-slate-400">
              <div className="flex items-center gap-1.5">
                <Cpu className="h-3.5 w-3.5 text-slate-400" />
                <span>{d?.cpu_model ?? "System Processor"}</span>
              </div>
              <div className="flex items-center gap-1.5 font-mono">
                <Server className="h-3.5 w-3.5 text-slate-400" />
                <span>{d ? formatBytes(d.total_memory_bytes) : "Memory"}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="grid gap-6 md:grid-cols-2">
        {/* Hardware Specifications Bento Card */}
        <div className="glass-card rounded-3xl p-6">
          <div className="flex items-center gap-2.5 mb-4 text-blue-400">
            <Cpu className="h-5 w-5" />
            <h3 className="text-sm font-semibold text-white">Hardware Architecture</h3>
          </div>

          <dl className="space-y-3 text-xs divide-y divide-white/[0.04]">
            <div className="flex items-center justify-between pt-2">
              <dt className="text-slate-400">System Architecture</dt>
              <dd className="font-mono text-slate-200">{d?.physical_cores} Cores / {d?.logical_cores} Threads</dd>
            </div>
            <div className="flex items-center justify-between pt-2">
              <dt className="text-slate-400">Clock Frequency</dt>
              <dd className="font-mono text-slate-200">{d?.base_frequency_mhz ? `${d.base_frequency_mhz.toFixed(0)} MHz` : "Dynamic"}</dd>
            </div>
            <div className="flex items-center justify-between pt-2">
              <dt className="text-slate-400">Motherboard</dt>
              <dd className="font-mono text-slate-200">{d?.motherboard ?? "—"}</dd>
            </div>
            <div className="flex items-center justify-between pt-2">
              <dt className="text-slate-400">Firmware / BIOS</dt>
              <dd className="font-mono text-slate-200">{d?.bios_version ?? "—"}</dd>
            </div>
            <div className="flex items-center justify-between pt-2">
              <dt className="text-slate-400">Power Subsystem</dt>
              <dd className="flex items-center gap-1.5 font-mono text-slate-200">
                <Battery className="h-3.5 w-3.5 text-emerald-400" />
                <span>
                  {bat?.present
                    ? `${bat.state} · ${bat.percentage?.toFixed(0) ?? ""}% (Health: ${bat.health_percent?.toFixed(0) ?? "—"}%)`
                    : "Desktop (A/C Mains)"}
                </span>
              </dd>
            </div>
          </dl>
        </div>

        {/* Local-First Privacy Certificate Bento Card */}
        <div className="glass-card rounded-3xl p-6">
          <div className="flex items-center gap-2.5 mb-4 text-emerald-400">
            <ShieldCheck className="h-5 w-5" />
            <h3 className="text-sm font-semibold text-white">Privacy & Isolation Certificate</h3>
          </div>

          <ul className="space-y-3 text-xs">
            <li className="flex items-start gap-2.5 rounded-xl bg-white/[0.02] p-2.5 border border-white/[0.04]">
              <CheckCircle2 className="h-4 w-4 text-emerald-400 mt-0.5 shrink-0" />
              <span className="text-slate-300 leading-relaxed">
                <strong className="text-white">Strictly Local Telemetry:</strong> All samples and metrics are written directly to your local SQLite WAL database.
              </span>
            </li>
            <li className="flex items-start gap-2.5 rounded-xl bg-white/[0.02] p-2.5 border border-white/[0.04]">
              <CheckCircle2 className="h-4 w-4 text-emerald-400 mt-0.5 shrink-0" />
              <span className="text-slate-300 leading-relaxed">
                <strong className="text-white">No Cloud Dependency:</strong> Zero phone-home requests or cloud callbacks (Phone-Home: {String(meta?.phone_home ?? false)}).
              </span>
            </li>
            <li className="flex items-start gap-2.5 rounded-xl bg-white/[0.02] p-2.5 border border-white/[0.04]">
              <CheckCircle2 className="h-4 w-4 text-emerald-400 mt-0.5 shrink-0" />
              <span className="text-slate-300 leading-relaxed">
                <strong className="text-white">Explain, Don't Accuse:</strong> Security signals are heuristic patterns, never claims of attack or active compromise.
              </span>
            </li>
          </ul>
        </div>
      </div>

      {/* Storage & Daemon Configuration */}
      <div className="glass-card rounded-3xl p-6">
        <div className="flex items-center gap-2.5 mb-4 text-purple-400">
          <Database className="h-5 w-5" />
          <h3 className="text-sm font-semibold text-white">Engine Configuration & Retention</h3>
        </div>

        <div className="grid gap-4 sm:grid-cols-3 text-xs">
          <div className="rounded-2xl bg-white/[0.02] p-3.5 border border-white/[0.05]">
            <p className="text-slate-400 uppercase font-semibold text-[10px]">Daemon Listener</p>
            <p className="font-mono text-slate-200 mt-1">127.0.0.1:8745</p>
            <p className="text-[10px] text-slate-500 mt-1">Override with VIGILON_BIND</p>
          </div>

          <div className="rounded-2xl bg-white/[0.02] p-3.5 border border-white/[0.05]">
            <p className="text-slate-400 uppercase font-semibold text-[10px]">SQLite Storage Path</p>
            <p className="font-mono text-slate-200 mt-1 truncate">AppData/Local/Vigilon</p>
            <p className="text-[10px] text-slate-500 mt-1">Override with VIGILON_DATA</p>
          </div>

          <div className="rounded-2xl bg-white/[0.02] p-3.5 border border-white/[0.05]">
            <p className="text-slate-400 uppercase font-semibold text-[10px]">Retention Window</p>
            <p className="font-mono text-slate-200 mt-1">7 Days Auto-Pruning</p>
            <p className="text-[10px] text-slate-500 mt-1">Event-driven state transitions</p>
          </div>
        </div>
      </div>

      {/* Threat Intelligence — Blocklist / Allowlist Rules */}
      <div className="glass-card rounded-3xl p-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2.5 text-rose-400">
            <Ban className="h-5 w-5" />
            <h3 className="text-sm font-semibold text-white">Threat Intelligence — Blocklist Rules</h3>
          </div>
          <span className="rounded-full bg-rose-500/10 px-2.5 py-0.5 text-xs font-mono text-rose-400 border border-rose-500/20">
            {blocklist.length} Blocked
          </span>
        </div>
        <p className="text-xs text-slate-400 mb-4">
          Connections to blocklisted IPs raise a <span className="font-mono text-rose-300">BLOCKLISTED_CONNECTION</span> heuristic
          (Critical). Rules persist to <span className="font-mono">blocklist.json</span> in the data directory.
        </p>

        <form onSubmit={addBlock} className="flex flex-col sm:flex-row gap-2 mb-4">
          <input
            className="flex-1 rounded-2xl border border-white/[0.08] bg-black/40 px-4 py-2 text-xs font-mono text-white placeholder-slate-500 focus:outline-none focus:border-rose-500/50"
            placeholder="IP address, e.g. 203.0.113.45"
            aria-label="IP address to blocklist"
            value={newIp}
            onChange={(e) => setNewIp(e.target.value)}
          />
          <input
            className="flex-1 rounded-2xl border border-white/[0.08] bg-black/40 px-4 py-2 text-xs text-white placeholder-slate-500 focus:outline-none focus:border-rose-500/50"
            placeholder="Comment (optional)"
            aria-label="Blocklist entry comment"
            value={newComment}
            onChange={(e) => setNewComment(e.target.value)}
          />
          <button
            type="submit"
            className="flex items-center justify-center gap-1.5 rounded-2xl bg-rose-500/15 px-4 py-2 text-xs font-semibold text-rose-300 border border-rose-500/30 hover:bg-rose-500/25 transition-colors"
          >
            <Plus className="h-3.5 w-3.5" />
            Block IP
          </button>
        </form>

        <ul className="space-y-1.5 max-h-56 overflow-y-auto">
          {blocklist.map((entry) => (
            <li
              key={entry.ip}
              className="flex items-center justify-between rounded-xl bg-white/[0.02] p-2.5 border border-white/[0.04] text-xs"
            >
              <div className="min-w-0">
                <p className="font-mono text-slate-200">{entry.ip}</p>
                {entry.comment && <p className="text-[11px] text-slate-500 truncate">{entry.comment}</p>}
              </div>
              <button
                type="button"
                onClick={() => removeBlock(entry.ip)}
                className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-white/[0.04] text-slate-400 hover:text-rose-300 hover:bg-rose-500/10 transition-colors"
                title={`Remove ${entry.ip} from blocklist`}
                aria-label={`Remove ${entry.ip} from blocklist`}
              >
                <Trash2 className="h-3.5 w-3.5" />
              </button>
            </li>
          ))}
          {blocklist.length === 0 && (
            <li className="text-center text-xs text-slate-500 py-6">
              Blocklist is empty — no threat-intel rules active.
            </li>
          )}
        </ul>
      </div>

      {/* Bandwidth Quota + Remote Agents */}
      <div className="grid gap-6 md:grid-cols-2">
        <div className="glass-card rounded-3xl p-6">
          <div className="flex items-center gap-2.5 mb-4 text-teal-400">
            <Gauge className="h-5 w-5" />
            <h3 className="text-sm font-semibold text-white">Bandwidth Quota Policy</h3>
          </div>
          <dl className="space-y-3 text-xs divide-y divide-white/[0.04]">
            <div className="flex items-center justify-between pt-2">
              <dt className="text-slate-400">Monthly Cap</dt>
              <dd className="font-mono text-slate-200">
                {agentConfig?.bandwidth_quota_gb ?? quota?.quota_gb ?? "— (unset: VIGILON_BANDWIDTH_QUOTA_GB)"}
                {(agentConfig?.bandwidth_quota_gb ?? quota?.quota_gb) != null ? " GB" : ""}
              </dd>
            </div>
            <div className="flex items-center justify-between pt-2">
              <dt className="text-slate-400">Warning Threshold</dt>
              <dd className="font-mono text-slate-200">
                {agentConfig?.bandwidth_warning_pct ?? quota?.warning_pct ?? 80}% (VIGILON_QUOTA_WARN_PCT)
              </dd>
            </div>
            <div className="flex items-center justify-between pt-2">
              <dt className="text-slate-400">Used This Month ({quota?.month ?? "—"})</dt>
              <dd className="font-mono text-slate-200">
                ↓ {formatBytes(quota?.used_rx_bytes ?? 0)} · ↑ {formatBytes(quota?.used_tx_bytes ?? 0)}
              </dd>
            </div>
          </dl>
        </div>

        <div className="glass-card rounded-3xl p-6">
          <div className="flex items-center gap-2.5 mb-4 text-indigo-400">
            <Network className="h-5 w-5" />
            <h3 className="text-sm font-semibold text-white">Remote Agents (Fleet)</h3>
          </div>
          {(agentConfig?.remote_agents?.length ?? 0) > 0 || fleet.length > 0 ? (
            <ul className="space-y-1.5 text-xs font-mono">
              {(agentConfig?.remote_agents ?? []).map((addr) => {
                const status = fleet.find((f) => f.address === addr)?.status ?? "unknown";
                return (
                  <li
                    key={addr}
                    className="flex items-center justify-between rounded-xl bg-white/[0.02] p-2.5 border border-white/[0.04]"
                  >
                    <span className="text-slate-200">{addr}</span>
                    <span
                      className={`rounded-full px-2 py-0.5 text-[10px] border ${
                        status === "online"
                          ? "bg-emerald-500/10 text-emerald-400 border-emerald-500/20"
                          : "bg-slate-500/10 text-slate-400 border-slate-500/20"
                      }`}
                    >
                      {status.toUpperCase()}
                    </span>
                  </li>
                );
              })}
            </ul>
          ) : (
            <p className="text-xs text-slate-500">
              No remote agents configured. Set <span className="font-mono text-indigo-300">VIGILON_REMOTES</span> to a
              comma-separated list of <span className="font-mono">host:port</span> values, then open the Fleet page.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
