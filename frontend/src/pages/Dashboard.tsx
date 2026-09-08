import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import ReactECharts from "echarts-for-react";
import {
  Cpu,
  Activity,
  HardDrive,
  Wifi,
  Radio,
  Network,
  ShieldAlert,
  Clock,
  ArrowUpRight,
  ArrowDownLeft,
  Laptop,
  Globe,
  Router,
  CheckCircle2,
  ChevronRight,
  Gauge,
  MemoryStick,
  LayoutDashboard,
  type LucideIcon,
} from "lucide-react";
import type { Live } from "../useLive";
import type { BandwidthQuotaInfo, HistoryPoint, ProcessBandwidth, SecurityEvent } from "../types";
import { formatBps, formatBytes, getJson, riskColor, riskLabel, timeAgo } from "../lib";
import { countryFlag } from "../lib/flags";
import WhyPanel from "../components/WhyPanel";
import SparkChart from "../components/SparkChart";
import PageHeader from "../components/PageHeader";
import ExportButton from "../components/ExportButton";
import { RiskDot } from "../components/ui";
import { useTheme } from "../hooks/useTheme";

/* ── Apple-style sparkline ring-buffer ── */
function useSpark(value: number | undefined, max = 100) {
  const buf = useRef<number[]>([]);
  if (value != null) {
    buf.current.push(value);
    if (buf.current.length > 30) buf.current = buf.current.slice(-30);
  }
  return { data: buf.current, max };
}

export default function Dashboard({ live }: { live: Live }) {
  const s = live.snapshot;
  const [history, setHistory] = useState<HistoryPoint[]>([]);
  const [selected, setSelected] = useState<SecurityEvent | null>(null);
  const [quota, setQuota] = useState<BandwidthQuotaInfo | null>(null);
  const [procBw, setProcBw] = useState<ProcessBandwidth[]>([]);
  const [panelError, setPanelError] = useState<string | null>(null);
  const { theme } = useTheme();
  const chartDark = theme !== "light";

  useEffect(() => {
    let cancelled = false;
    const fail = () => {
      if (!cancelled) setPanelError("Couldn't load chart data from the daemon.");
    };
    void getJson<HistoryPoint[]>("/metrics/history?hours=6")
      .then((v) => { if (!cancelled) setHistory(v); })
      .catch(fail);
    void getJson<BandwidthQuotaInfo>("/bandwidth").then((v) => { if (!cancelled) setQuota(v); }).catch(() => {});
    void getJson<ProcessBandwidth[]>("/process-bandwidth").then((v) => { if (!cancelled) setProcBw(v); }).catch(() => {});
    return () => { cancelled = true; };
  }, [s?.timestamp]);

  /* ── calculations ── */
  const ramPct = s
    ? s.memory.total_bytes
      ? (s.memory.used_bytes / s.memory.total_bytes) * 100
      : 0
    : 0;
  const swapPct = s
    ? s.memory.swap_total_bytes
      ? (s.memory.swap_used_bytes / s.memory.swap_total_bytes) * 100
      : 0
    : 0;

  const gpuAvg =
    s && s.gpus.length > 0
      ? s.gpus.reduce((acc, g) => acc + (g.usage_percent ?? 0), 0) / s.gpus.length
      : null;

  const diskTotalUsed = s ? s.disks.reduce((a, d) => a + d.used_bytes, 0) : 0;
  const diskTotalCap = s ? s.disks.reduce((a, d) => a + d.total_bytes, 0) : 1;
  const diskPct = (diskTotalUsed / diskTotalCap) * 100;
  const diskReadBps = s ? s.disks.reduce((a, d) => a + d.read_bps, 0) : 0;
  const diskWriteBps = s ? s.disks.reduce((a, d) => a + d.write_bps, 0) : 0;

  /* Sparkline buffers */
  const cpuSpark = useSpark(s?.cpu.usage_percent);
  const ramSpark = useSpark(ramPct);
  const gpuSpark = useSpark(gpuAvg ?? undefined);
  const diskSpark = useSpark(diskPct);
  const netRxSpark = useSpark(
    s?.network_rx_bps,
    s ? Math.max(s.network_rx_bps * 1.5, 1024) : 1024,
  );

  /* ── Apple Health style timeline chart ── */
  const chartOption = useMemo(
    () => ({
      backgroundColor: "transparent",
      textStyle: { color: chartDark ? "#94a3b8" : "#475569", fontFamily: "Inter, -apple-system, sans-serif" },
      tooltip: {
        trigger: "axis",
        backgroundColor: chartDark ? "rgba(15, 23, 42, 0.9)" : "rgba(255, 255, 255, 0.96)",
        borderColor: chartDark ? "rgba(255, 255, 255, 0.12)" : "rgba(15, 23, 42, 0.12)",
        textStyle: { color: chartDark ? "#f8fafc" : "#0f172a", fontSize: 12 },
        borderRadius: 12,
        padding: 12,
      },
      legend: {
        data: ["CPU", "RAM", "GPU"],
        top: 4,
        right: 16,
        textStyle: { color: chartDark ? "#94a3b8" : "#475569", fontSize: 11 },
        itemWidth: 10,
        itemHeight: 10,
        icon: "circle",
      },
      grid: { left: 45, right: 16, top: 36, bottom: 26 },
      xAxis: {
        type: "time",
        axisLine: { lineStyle: { color: chartDark ? "rgba(255, 255, 255, 0.08)" : "rgba(15, 23, 42, 0.14)" } },
        splitLine: { show: false },
        axisLabel: { color: chartDark ? "#64748b" : "#64748b", fontSize: 11 },
      },
      yAxis: {
        type: "value",
        max: 100,
        axisLine: { show: false },
        splitLine: { lineStyle: { color: chartDark ? "rgba(255, 255, 255, 0.05)" : "rgba(15, 23, 42, 0.08)" } },
        axisLabel: { color: "#64748b", fontSize: 11, formatter: "{value}%" },
      },
      series: [
        {
          name: "CPU",
          type: "line",
          showSymbol: false,
          smooth: true,
          lineStyle: { width: 2, color: "#10b981", shadowColor: "rgba(16, 185, 129, 0.4)", shadowBlur: 6 },
          areaStyle: {
            color: {
              type: "linear",
              x: 0, y: 0, x2: 0, y2: 1,
              colorStops: [{ offset: 0, color: "rgba(16, 185, 129, 0.25)" }, { offset: 1, color: "rgba(16, 185, 129, 0.0)" }],
            },
          },
          data: history.map((h) => [h.timestamp, h.cpu_usage]),
        },
        {
          name: "RAM",
          type: "line",
          showSymbol: false,
          smooth: true,
          lineStyle: { width: 2, color: "#38bdf8", shadowColor: "rgba(56, 189, 248, 0.4)", shadowBlur: 6 },
          areaStyle: {
            color: {
              type: "linear",
              x: 0, y: 0, x2: 0, y2: 1,
              colorStops: [{ offset: 0, color: "rgba(56, 189, 248, 0.2)" }, { offset: 1, color: "rgba(56, 189, 248, 0.0)" }],
            },
          },
          data: history.map((h) => [h.timestamp, h.ram_usage]),
        },
        {
          name: "GPU",
          type: "line",
          showSymbol: false,
          smooth: true,
          lineStyle: { width: 2, color: "#f59e0b", shadowColor: "rgba(245, 158, 11, 0.4)", shadowBlur: 6 },
          data: history.map((h) => [h.timestamp, h.gpu_usage ?? 0]),
        },
      ],
    }),
    [history, chartDark],
  );

  const topProcs = [...live.processes]
    .sort((a, b) => b.cpu_percent - a.cpu_percent)
    .slice(0, 8);

  const activeIfaces = live.interfaces.filter((i) => i.is_up && i.kind !== "loopback");
  const host = s?.device.hostname ?? "My Device";
  const primaryIp = activeIfaces[0]?.ips[0] ?? "127.0.0.1";
  const gateway = activeIfaces[0]?.gateway ?? "Default Gateway";

  // Pre-hydration: skeleton tiles so "loading" never looks like "clean".
  if (!live.hydrated) {
    return (
      <div className="space-y-6" aria-label="Loading dashboard">
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-5">
          {Array.from({ length: 5 }).map((_, i) => (
            <div key={i} className="glass-card h-48 animate-pulse rounded-3xl" aria-hidden="true" />
          ))}
        </div>
        <div className="glass-card h-64 animate-pulse rounded-3xl" aria-hidden="true" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Top Page Header with Brand Logo & Page Icon */}
      <PageHeader
        icon={LayoutDashboard}
        iconColor="text-blue-400"
        iconBg="bg-blue-500/15"
        iconBorder="border-blue-500/25"
        category="OVERVIEW"
        title="System Health & Telemetry"
        subtitle="Real-time multi-core processor, memory, storage, graphics & dynamic network throughput"
        actions={
          <div className="flex items-center gap-2 text-xs font-mono">
            <span className="rounded-full bg-blue-500/10 px-3 py-1 text-blue-400 border border-blue-500/20">
              {host}
            </span>
            <span className="rounded-full bg-emerald-500/10 px-3 py-1 text-emerald-400 border border-emerald-500/20">
              {primaryIp}
            </span>
            <ExportButton endpoint="metrics" label="Export" />
          </div>
        }
      />

      {/* ── 1. APPLE BENTO BOX TILES ── */}
      <section className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-5">
        {/* CPU Tile */}
        <BentoTile
          icon={Cpu}
          iconColor="text-emerald-400"
          accentGlow="from-emerald-500/10"
          label="CPU Processor"
          value={s ? `${s.cpu.usage_percent.toFixed(0)}%` : "—"}
          badge={s?.cpu.temperature_c ? `${s.cpu.temperature_c.toFixed(0)}°C` : undefined}
          sub={s?.device.cpu_model}
          spark={<SparkChart data={cpuSpark.data} color="#10b981" max={100} />}
          footer={
            s?.cpu.per_core?.length ? (
              <div className="mt-3 pt-3 border-t border-white/[0.06] flex items-center justify-between gap-1">
                <span className="text-[10px] uppercase font-semibold text-slate-400">
                  {s.cpu.per_core.length} Cores
                </span>
                <div className="flex items-end gap-1 h-4">
                  {s.cpu.per_core.slice(0, 16).map((c, i) => (
                    <div
                      key={i}
                      className="w-1 rounded-full bg-emerald-400/80 transition-all duration-300"
                      style={{ height: `${Math.max(15, c)}%` }}
                    />
                  ))}
                </div>
              </div>
            ) : null
          }
        />

        {/* RAM Tile */}
        <BentoTile
          icon={MemoryStick}
          iconColor="text-sky-400"
          accentGlow="from-sky-500/10"
          label="Unified Memory"
          value={`${ramPct.toFixed(0)}%`}
          sub={
            s
              ? `${formatBytes(s.memory.used_bytes)} of ${formatBytes(s.memory.total_bytes)}`
              : undefined
          }
          badge={swapPct > 0 ? `Swap ${swapPct.toFixed(0)}%` : undefined}
          spark={<SparkChart data={ramSpark.data} color="#38bdf8" max={100} />}
          footer={
            <div className="mt-3 pt-3 border-t border-white/[0.06]">
              <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/[0.06]">
                <div
                  className="h-full rounded-full bg-gradient-to-r from-sky-400 to-indigo-400 transition-all duration-500"
                  style={{ width: `${Math.min(100, ramPct)}%` }}
                />
              </div>
            </div>
          }
        />

        {/* GPU Tile */}
        <BentoTile
          icon={Gauge}
          iconColor="text-amber-400"
          accentGlow="from-amber-500/10"
          label="Graphics Engine"
          value={gpuAvg != null ? `${gpuAvg.toFixed(0)}%` : "Integrated"}
          sub={s?.gpus[0]?.name ?? "Direct3D / Vulkan"}
          badge={
            s?.gpus[0]?.temperature_c
              ? `${s.gpus[0].temperature_c.toFixed(0)}°C`
              : undefined
          }
          spark={gpuAvg != null ? <SparkChart data={gpuSpark.data} color="#f59e0b" max={100} /> : undefined}
          footer={
            s?.gpus[0]?.memory_total_bytes ? (
              <div className="mt-3 pt-3 border-t border-white/[0.06] flex justify-between text-[11px] text-slate-400 font-mono">
                <span>VRAM Total</span>
                <span>{formatBytes(s.gpus[0].memory_total_bytes)}</span>
              </div>
            ) : null
          }
        />

        {/* Storage Tile */}
        <BentoTile
          icon={HardDrive}
          iconColor="text-rose-400"
          accentGlow="from-rose-500/10"
          label="Storage System"
          value={`${diskPct.toFixed(0)}%`}
          sub={`↓ ${formatBps(diskReadBps)} · ↑ ${formatBps(diskWriteBps)}`}
          spark={<SparkChart data={diskSpark.data} color="#f43f5e" max={100} />}
          footer={
            s?.disks?.length ? (
              <div className="mt-3 pt-3 border-t border-white/[0.06] flex items-center justify-between text-[11px] text-slate-400">
                <span>{s.disks.length} Volumes</span>
                <span className="font-mono text-emerald-400">Healthy</span>
              </div>
            ) : null
          }
        />

        {/* Network Tile */}
        <BentoTile
          icon={Wifi}
          iconColor="text-teal-400"
          accentGlow="from-teal-500/10"
          label="Network I/O"
          value={s ? `↓ ${formatBps(s.network_rx_bps)}` : "—"}
          sub={s ? `↑ ${formatBps(s.network_tx_bps)}` : undefined}
          spark={<SparkChart data={netRxSpark.data} color="#2dd4bf" max={netRxSpark.max} />}
          footer={
            <div className="mt-3 pt-3 border-t border-white/[0.06] flex items-center justify-between text-[11px] text-slate-400 font-mono">
              <span className="flex items-center gap-1">
                <ArrowDownLeft className="h-3 w-3 text-teal-400" />
                {formatBytes(activeIfaces.reduce((a, i) => a + i.rx_total_bytes, 0))}
              </span>
              <span className="flex items-center gap-1">
                <ArrowUpRight className="h-3 w-3 text-sky-400" />
                {formatBytes(activeIfaces.reduce((a, i) => a + i.tx_total_bytes, 0))}
              </span>
            </div>
          }
        />
      </section>

      {/* ── 1b. BANDWIDTH QUOTA + TOP TALKERS ── */}
      <section className="grid gap-4 lg:grid-cols-2">
        {/* Monthly quota progress */}
        <div className="glass-card rounded-3xl p-6">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <Wifi className="h-4 w-4 text-teal-400" />
              <h3 className="text-sm font-semibold text-white">Monthly Bandwidth Quota</h3>
            </div>
            <span className="text-[11px] font-mono text-slate-400">
              {quota?.month ?? "this month"}
            </span>
          </div>
          {(() => {
            const used = (quota?.used_rx_bytes ?? 0) + (quota?.used_tx_bytes ?? 0);
            const cap = quota?.quota_gb ? quota.quota_gb * 1024 ** 3 : 0;
            const pct = cap > 0 ? Math.min(100, (used / cap) * 100) : 0;
            const warnAt = quota?.warning_pct ?? 80;
            const over = cap > 0 && pct >= 100;
            const warn = cap > 0 && !over && pct >= warnAt;
            return (
              <div>
                <div className="flex items-end justify-between mb-2">
                  <p className="text-2xl font-bold text-white">
                    {formatBytes(used)}
                    {cap > 0 && (
                      <span className="text-sm font-medium text-slate-400"> / {quota?.quota_gb} GB</span>
                    )}
                  </p>
                  <p className={`text-xs font-mono ${over ? "text-rose-400" : warn ? "text-amber-400" : "text-emerald-400"}`}>
                    {cap > 0 ? `${pct.toFixed(1)}%${over ? " — quota exceeded" : warn ? " — warning threshold" : ""}` : "no quota configured"}
                  </p>
                </div>
                <div className="h-2.5 w-full overflow-hidden rounded-full bg-white/[0.06]">
                  <div
                    className={`h-full rounded-full transition-all duration-500 ${over ? "bg-gradient-to-r from-rose-500 to-red-400" : warn ? "bg-gradient-to-r from-amber-500 to-orange-400" : "bg-gradient-to-r from-teal-400 to-emerald-400"}`}
                    style={{ width: `${cap > 0 ? pct : 0}%` }}
                  />
                </div>
                <div className="mt-2 flex justify-between font-mono text-[11px] text-slate-400">
                  <span>↓ {formatBytes(quota?.used_rx_bytes ?? 0)} received</span>
                  <span>↑ {formatBytes(quota?.used_tx_bytes ?? 0)} sent</span>
                </div>
              </div>
            );
          })()}
        </div>

        {/* Per-process bandwidth chart */}
        <div className="glass-card rounded-3xl p-6">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <Activity className="h-4 w-4 text-purple-400" />
              <h3 className="text-sm font-semibold text-white">Top Bandwidth Talkers</h3>
            </div>
            <span className="text-[11px] font-mono text-slate-400">per-process Rx/Tx share</span>
          </div>
          {procBw.length === 0 ? (
            <p className="text-xs text-slate-500 py-6 text-center">No per-process bandwidth attribution yet.</p>
          ) : (
            <ul className="space-y-2">
              {procBw.slice(0, 6).map((b) => {
                const total = b.rx_bytes + b.tx_bytes;
                const max = Math.max(...procBw.slice(0, 6).map((x) => x.rx_bytes + x.tx_bytes), 1);
                return (
                  <li key={b.pid}>
                    <div className="flex items-center justify-between text-xs mb-1">
                      <span className="font-medium text-slate-200 truncate">
                        {b.name} <span className="font-mono text-[10px] text-slate-500">PID {b.pid}</span>
                      </span>
                      <span className="font-mono text-[11px] text-slate-400">
                        ↓ {formatBytes(b.rx_bytes)} · ↑ {formatBytes(b.tx_bytes)}
                      </span>
                    </div>
                    <div className="h-1.5 w-full overflow-hidden rounded-full bg-white/[0.06]">
                      <div
                        className="h-full rounded-full bg-gradient-to-r from-purple-400 to-indigo-400"
                        style={{ width: `${Math.min(100, (total / max) * 100)}%` }}
                      />
                    </div>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </section>

      {/* ── 2. VISUAL NETWORK FLOW PIPELINE ── */}
      <section className="glass-card rounded-3xl p-6 relative overflow-hidden">
        <div className="flex items-center justify-between mb-6">
          <div className="flex items-center gap-2.5">
            <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-blue-500/15 border border-blue-500/20 text-blue-400">
              <Network className="h-4 w-4" />
            </div>
            <div>
              <h2 className="text-sm font-semibold tracking-tight text-white">Live Network Flow</h2>
              <p className="text-[11px] text-slate-400">End-to-end socket telemetry & destination correlation</p>
            </div>
          </div>
          <span className="rounded-full bg-white/[0.06] px-3 py-1 text-xs font-mono text-slate-300 border border-white/[0.08]">
            {live.connections.length} Active Sockets
          </span>
        </div>

        {/* Apple Flow Stream Visualizer */}
        <div className="relative mb-6 rounded-2xl border border-white/[0.08] bg-black/40 p-4 backdrop-blur-md">
          <div className="flex flex-col sm:flex-row items-center justify-between gap-4 py-2">
            {/* Host PC */}
            <div className="flex items-center gap-3 rounded-xl bg-white/[0.04] p-3 border border-white/[0.08] w-full sm:w-auto">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-emerald-500/20 border border-emerald-500/30 text-emerald-400">
                <Laptop className="h-5 w-5" />
              </div>
              <div>
                <p className="text-xs font-semibold text-white">{host}</p>
                <p className="font-mono text-[11px] text-slate-400">{primaryIp}</p>
              </div>
            </div>

            {/* Glowing Connector 1 */}
            <div className="hidden sm:flex flex-1 items-center justify-center px-4">
              <div className="relative w-full h-[2px] bg-gradient-to-r from-emerald-500/40 via-blue-500/40 to-indigo-500/40">
                <span className="absolute -top-1 left-1/2 -translate-x-1/2 h-2 w-2 rounded-full bg-blue-400 animate-ping" />
              </div>
            </div>

            {/* Router Gateway */}
            <div className="flex items-center gap-3 rounded-xl bg-white/[0.04] p-3 border border-white/[0.08] w-full sm:w-auto">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-blue-500/20 border border-blue-500/30 text-blue-400">
                <Router className="h-5 w-5" />
              </div>
              <div>
                <p className="text-xs font-semibold text-white">Local Gateway</p>
                <p className="font-mono text-[11px] text-slate-400">{gateway}</p>
              </div>
            </div>

            {/* Glowing Connector 2 */}
            <div className="hidden sm:flex flex-1 items-center justify-center px-4">
              <div className="relative w-full h-[2px] bg-gradient-to-r from-indigo-500/40 via-purple-500/40 to-sky-500/40">
                <span className="absolute -top-1 left-1/2 -translate-x-1/2 h-2 w-2 rounded-full bg-sky-400 animate-pulse" />
              </div>
            </div>

            {/* External World */}
            <div className="flex items-center gap-3 rounded-xl bg-white/[0.04] p-3 border border-white/[0.08] w-full sm:w-auto">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-purple-500/20 border border-purple-500/30 text-purple-400">
                <Globe className="h-5 w-5" />
              </div>
              <div>
                <p className="text-xs font-semibold text-white">External Internet</p>
                <p className="font-mono text-[11px] text-slate-400">
                  {live.connections.filter((c) => !c.remote_addr.startsWith("127.")).length} remotes
                </p>
              </div>
            </div>
          </div>
        </div>

        {/* 3-Column Bento Telemetry Grid */}
        <div className="grid gap-4 md:grid-cols-3">
          {/* Interfaces */}
          <div className="rounded-2xl border border-white/[0.06] bg-white/[0.02] p-4">
            <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-slate-400 flex items-center gap-1.5">
              <Wifi className="h-3.5 w-3.5 text-slate-400" />
              Physical Adapters ({activeIfaces.length})
            </h3>
            <ul className="space-y-2 text-xs">
              {activeIfaces.slice(0, 5).map((iface) => (
                <li
                  key={iface.name}
                  className="flex items-center justify-between rounded-xl bg-white/[0.03] p-2.5 border border-white/[0.04]"
                >
                  <div>
                    <p className="font-medium text-white">{iface.name}</p>
                    <p className="text-[10px] text-slate-400 uppercase font-mono">{iface.kind}</p>
                  </div>
                  <div className="text-right font-mono text-[11px] text-slate-300">
                    <div>↓ {formatBps(iface.rx_bps)}</div>
                    <div className="text-slate-400">↑ {formatBps(iface.tx_bps)}</div>
                    <div className="text-[10px] text-teal-400/70 mt-0.5">
                      Σ {formatBytes(iface.rx_total_bytes + iface.tx_total_bytes)}
                    </div>
                  </div>
                </li>
              ))}
              {activeIfaces.length === 0 && (
                <li className="text-slate-500 text-center py-4">No active adapters</li>
              )}
            </ul>
          </div>

          {/* Listening Ports */}
          <div className="rounded-2xl border border-white/[0.06] bg-white/[0.02] p-4">
            <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-slate-400 flex items-center gap-1.5">
              <Radio className="h-3.5 w-3.5 text-slate-400" />
              Listening Ports ({live.ports.length})
            </h3>
            <ul className="space-y-1.5 text-xs font-mono">
              {live.ports.slice(0, 6).map((p) => (
                <li
                  key={`${p.protocol}-${p.address}-${p.port}`}
                  className="flex items-center justify-between rounded-xl bg-white/[0.03] p-2 border border-white/[0.04]"
                >
                  <span className="font-semibold text-emerald-400">:{p.port}</span>
                  <span className="truncate max-w-[120px] text-slate-300 font-sans">
                    {p.process_name ?? "system"}
                  </span>
                  <span className="rounded bg-white/[0.06] px-1.5 py-0.5 text-[10px] text-slate-400">
                    {p.protocol}
                  </span>
                </li>
              ))}
              {live.ports.length > 6 && (
                <li className="text-center text-[11px] text-slate-400 pt-1">
                  +{live.ports.length - 6} more listening ports
                </li>
              )}
            </ul>
          </div>

          {/* Top Remote Destinations */}
          <div className="rounded-2xl border border-white/[0.06] bg-white/[0.02] p-4">
            <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-slate-400 flex items-center gap-1.5">
              <Globe className="h-3.5 w-3.5 text-slate-400" />
              Top Remote Endpoints
            </h3>
            <ul className="space-y-1.5 text-xs font-mono">
              {Object.entries(
                live.connections
                  .filter((c) => !c.remote_addr.startsWith("127.") && !c.remote_addr.startsWith("::1"))
                  .reduce<Record<string, number>>((acc, c) => {
                    acc[c.remote_addr] = (acc[c.remote_addr] ?? 0) + 1;
                    return acc;
                  }, {}),
              )
                .sort(([, a], [, b]) => b - a)
                .slice(0, 6)
                .map(([addr, count]) => (
                  <li
                    key={addr}
                    className="flex items-center justify-between rounded-xl bg-white/[0.03] p-2 border border-white/[0.04]"
                  >
                    <span className="truncate max-w-[170px] text-slate-300">
                      <span className="mr-1.5">
                        {countryFlag(live.connections.find((c) => c.remote_addr === addr)?.geo?.country_code)}
                      </span>
                      <Link
                        to={`/inspector?ip=${encodeURIComponent(addr)}`}
                        className="hover:text-sky-300 hover:underline"
                        title={`Inspect ${addr} in Connection Inspector`}
                      >
                        {addr}
                      </Link>
                    </span>
                    <span className="rounded-full bg-blue-500/10 px-2 py-0.5 text-[10px] font-semibold text-blue-400 border border-blue-500/20">
                      {count} socket{count > 1 ? "s" : ""}
                    </span>
                  </li>
                ))}
              {live.connections.length === 0 && (
                <li className="text-slate-500 text-center py-4">No external connections</li>
              )}
            </ul>
          </div>
        </div>
      </section>

      {/* ── 3. PERFORMANCE TIMELINE (6h) ── */}
      <section className="glass-card rounded-3xl p-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2.5">
            <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-purple-500/15 border border-purple-500/20 text-purple-400">
              <Activity className="h-4 w-4" />
            </div>
            <div>
              <h2 className="text-sm font-semibold tracking-tight text-white">System Dynamics</h2>
              <p className="text-[11px] text-slate-400">Continuous 6-hour hardware telemetry timeline</p>
            </div>
          </div>
        </div>
        {panelError ? (
          <p role="alert" className="rounded-2xl border border-amber-500/30 bg-amber-500/10 p-4 text-xs text-amber-200">
            {panelError} Live tiles above still update over the socket stream.
          </p>
        ) : (
          <ReactECharts option={chartOption} style={{ height: 230 }} />
        )}
      </section>

      {/* ── 4. 2x2 BENTO INTELLIGENCE GRID ── */}
      <div className="grid gap-6 lg:grid-cols-2">
        {/* Open Ports Panel */}
        <section className="glass-card rounded-3xl p-6 flex flex-col">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <Radio className="h-4 w-4 text-emerald-400" />
              <h3 className="text-sm font-semibold text-white">Open Ports</h3>
            </div>
            <span className="rounded-full bg-emerald-500/10 px-2.5 py-0.5 text-xs font-mono font-medium text-emerald-400 border border-emerald-500/20">
              {live.ports.length} Bound
            </span>
          </div>

          <div className="flex-1 overflow-x-auto">
            <table className="w-full text-left text-xs">
              <thead className="text-[11px] uppercase tracking-wider text-slate-400 border-b border-white/[0.06]">
                <tr>
                  <th scope="col" className="pb-2">Port</th>
                  <th scope="col" className="pb-2">Process</th>
                  <th scope="col" className="pb-2">Proto</th>
                  <th scope="col" className="pb-2">Bind Address</th>
                  <th scope="col" className="pb-2 text-right">PID</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-white/[0.04]">
                {live.ports.slice(0, 10).map((p) => (
                  <tr key={`${p.protocol}-${p.address}-${p.port}`} className="hover:bg-white/[0.02]">
                    <td className="py-2.5 font-mono font-semibold text-emerald-400">:{p.port}</td>
                    <td className="py-2.5 font-medium text-slate-200">{p.process_name ?? "—"}</td>
                    <td className="py-2.5 text-slate-400 font-mono text-[11px]">{p.protocol}</td>
                    <td className="py-2.5 font-mono text-slate-400 text-[11px]">{p.address}</td>
                    <td className="py-2.5 text-right font-mono text-slate-400">{p.pid ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        {/* Active Connections Panel */}
        <section className="glass-card rounded-3xl p-6 flex flex-col">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <Activity className="h-4 w-4 text-sky-400" />
              <h3 className="text-sm font-semibold text-white">Active Connections</h3>
            </div>
            <span className="rounded-full bg-sky-500/10 px-2.5 py-0.5 text-xs font-mono font-medium text-sky-400 border border-sky-500/20">
              {live.connections.length} Sockets
            </span>
          </div>

          <div className="flex-1 overflow-x-auto">
            <table className="w-full text-left text-xs">
              <thead className="text-[11px] uppercase tracking-wider text-slate-400 border-b border-white/[0.06]">
                <tr>
                  <th scope="col" className="pb-2">Socket Endpoint</th>
                  <th scope="col" className="pb-2">Process</th>
                  <th scope="col" className="pb-2">Proto</th>
                  <th scope="col" className="pb-2 text-right">State</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-white/[0.04]">
                {live.connections.slice(0, 10).map((c, i) => (
                  <tr key={i} className="hover:bg-white/[0.02]">
                    <td className="py-2.5 font-mono text-[11px] text-slate-200">
                      <span className="mr-1.5" title={c.geo ? `${c.geo.country_name}${c.geo.city ? ` · ${c.geo.city}` : ""}` : "Local / private"}>
                        {countryFlag(c.geo?.country_code)}
                      </span>
                      {c.local_port} →{" "}
                      <Link
                        to={`/inspector?ip=${encodeURIComponent(c.remote_addr)}`}
                        className="text-sky-300 hover:text-sky-200 hover:underline"
                        title={`Inspect ${c.remote_addr} in Connection Inspector`}
                      >
                        {c.remote_addr}:{c.remote_port}
                      </Link>
                    </td>
                    <td className="py-2.5 font-medium text-slate-300">{c.process_name ?? "—"}</td>
                    <td className="py-2.5 font-mono text-[11px] text-slate-400">{c.protocol}</td>
                    <td className="py-2.5 text-right">
                      <span className="rounded bg-white/[0.06] px-1.5 py-0.5 text-[10px] font-mono text-slate-300">
                        {c.state}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        {/* Security Signals Panel */}
        <section className="glass-card rounded-3xl p-6 flex flex-col">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <ShieldAlert className="h-4 w-4 text-amber-400" />
              <h3 className="text-sm font-semibold text-white">Security Signals</h3>
            </div>
            <span className="rounded-full bg-amber-500/10 px-2.5 py-0.5 text-xs font-mono font-medium text-amber-400 border border-amber-500/20">
              {live.events.length} Findings
            </span>
          </div>

          <ul className="space-y-2">
            {live.events.slice(0, 7).map((e) => (
              <li key={e.id}>
                <button
                  type="button"
                  className="flex w-full items-center justify-between gap-3 rounded-2xl bg-white/[0.02] p-3 border border-white/[0.05] hover:bg-white/[0.06] hover:border-white/[0.12] transition-all duration-200 text-left group"
                  onClick={() => setSelected(e)}
                >
                  <div className="flex items-center gap-3">
                    <RiskDot risk={e.risk} />
                    <div>
                      <p className="text-xs font-medium text-slate-100 group-hover:text-blue-300 transition-colors capitalize">
                        {e.rule.replaceAll("_", " ")}
                      </p>
                      <p className="text-[11px] text-slate-400">
                        {e.process?.name ? `${e.process.name} · ` : ""}
                        {timeAgo(e.timestamp)}
                      </p>
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    <span className={`text-[11px] font-semibold ${riskColor(e.risk)}`}>
                      {riskLabel(e.risk)}
                    </span>
                    <ChevronRight className="h-4 w-4 text-slate-500 group-hover:text-slate-300 transition-colors" />
                  </div>
                </button>
              </li>
            ))}
            {live.events.length === 0 && (
              <li className="text-slate-500 text-center py-6 text-xs flex items-center justify-center gap-2">
                <CheckCircle2 className="h-4 w-4 text-emerald-400" />
                <span>No heuristic findings observed. Baseline active.</span>
              </li>
            )}
          </ul>
        </section>

        {/* Top Process Monitor Panel */}
        <section className="glass-card rounded-3xl p-6 flex flex-col">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <Cpu className="h-4 w-4 text-purple-400" />
              <h3 className="text-sm font-semibold text-white">Active Processes</h3>
            </div>
            <span className="rounded-full bg-purple-500/10 px-2.5 py-0.5 text-xs font-mono font-medium text-purple-400 border border-purple-500/20">
              Ranked by CPU
            </span>
          </div>

          <div className="flex-1 overflow-x-auto">
            <table className="w-full text-left text-xs">
              <thead className="text-[11px] uppercase tracking-wider text-slate-400 border-b border-white/[0.06]">
                <tr>
                  <th scope="col" className="pb-2">Process</th>
                  <th scope="col" className="pb-2">CPU</th>
                  <th scope="col" className="pb-2">Memory</th>
                  <th scope="col" className="pb-2 text-right">Sockets</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-white/[0.04]">
                {topProcs.map((p) => (
                  <tr key={p.pid} className="hover:bg-white/[0.02]">
                    <td className="py-2.5 font-medium text-slate-200">
                      <div>{p.name}</div>
                      <div className="text-[10px] text-slate-400 font-mono">PID {p.pid}</div>
                    </td>
                    <td className="py-2.5">
                      <div className="flex items-center gap-2">
                        <span className="font-mono text-slate-200 w-10">
                          {p.cpu_percent.toFixed(1)}%
                        </span>
                        <div className="w-16 h-1 rounded-full bg-white/[0.06] overflow-hidden">
                          <div
                            className="h-full bg-purple-400 rounded-full"
                            style={{ width: `${Math.min(100, p.cpu_percent)}%` }}
                          />
                        </div>
                      </div>
                    </td>
                    <td className="py-2.5 font-mono text-slate-400 text-[11px]">
                      {formatBytes(p.memory_bytes)}
                    </td>
                    <td className="py-2.5 text-right font-mono text-slate-300">
                      {p.connection_count}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      </div>

      {/* ── 5. RECENT ACTIVITY TIMELINE WIDGET ── */}
      {live.timeline.length > 0 && (
        <section className="glass-card rounded-3xl p-6">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <Clock className="h-4 w-4 text-blue-400" />
              <h3 className="text-sm font-semibold text-white">System Events Timeline</h3>
            </div>
            <span className="text-xs text-slate-400 font-mono">Latest 8 entries</span>
          </div>

          <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
            {live.timeline.slice(0, 8).map((t) => (
              <div
                key={t.id}
                className="rounded-2xl border border-white/[0.06] bg-white/[0.02] p-3 hover:bg-white/[0.05] transition-colors"
              >
                <div className="flex items-center justify-between mb-1">
                  <span
                    className={`h-2 w-2 rounded-full ${
                      t.risk === "HIGH" || t.risk === "CRITICAL"
                        ? "bg-rose-500"
                        : t.risk === "MEDIUM"
                        ? "bg-amber-400"
                        : "bg-emerald-400"
                    }`}
                  />
                  <span className="text-[10px] font-mono text-slate-400">
                    {timeAgo(t.timestamp)}
                  </span>
                </div>
                <p className="text-xs font-semibold text-slate-100 truncate">{t.title}</p>
                <p className="text-[11px] text-slate-400 truncate mt-0.5">{t.summary}</p>
              </div>
            ))}
          </div>
        </section>
      )}

      {/* ── 6. SLIDE-IN INSPECTOR ── */}
      {selected && <WhyPanel event={selected} onClose={() => setSelected(null)} />}
    </div>
  );
}

/* ── Apple-style Bento Tile Component ── */
function BentoTile({
  icon: Icon,
  iconColor,
  accentGlow,
  label,
  value,
  sub,
  badge,
  spark,
  footer,
}: {
  icon: LucideIcon;
  iconColor: string;
  accentGlow: string;
  label: string;
  value: string;
  sub?: string;
  badge?: string;
  spark?: ReactNode;
  footer?: ReactNode;
}) {
  return (
    <div className={`glass-card rounded-3xl p-5 relative overflow-hidden flex flex-col justify-between group`}>
      {/* Background radial gradient accent */}
      <div
        className={`absolute -top-12 -right-12 h-32 w-32 rounded-full bg-gradient-to-br ${accentGlow} to-transparent blur-2xl pointer-events-none opacity-60 group-hover:opacity-100 transition-opacity duration-300`}
      />

      <div>
        {/* Header with Icon & Label */}
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-white/[0.05] border border-white/[0.08]">
              <Icon className={`h-4 w-4 ${iconColor}`} />
            </div>
            <span className="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
              {label}
            </span>
          </div>
          {badge && (
            <span className="rounded-full bg-white/[0.06] px-2 py-0.5 text-[10px] font-mono text-slate-300 border border-white/[0.08]">
              {badge}
            </span>
          )}
        </div>

        {/* Big Apple Display Stat */}
        <div className="mt-1">
          <div className="text-3xl font-bold tracking-tight text-white">{value}</div>
          {sub && <p className="truncate text-xs text-slate-400 mt-1">{sub}</p>}
        </div>
      </div>

      {/* Sparkline & Footer */}
      <div>
        {spark && <div className="mt-3">{spark}</div>}
        {footer}
      </div>
    </div>
  );
}
