import { useEffect, useMemo, useState } from "react";
import ReactECharts from "echarts-for-react";
import {
  Search,
  Globe,
  ShieldAlert,
  ShieldCheck,
  Filter,
  RefreshCw,
} from "lucide-react";
import type { Live } from "../useLive";
import type { DnsQuery, TopDnsDomain } from "../types";
import { getJson, timeAgo } from "../lib";
import PageHeader from "../components/PageHeader";
import { useTheme } from "../hooks/useTheme";

export default function DnsLog({ live }: { live: Live }) {
  const [queries, setQueries] = useState<DnsQuery[]>([]);
  const [topDomains, setTopDomains] = useState<TopDnsDomain[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(false);
  const { theme } = useTheme();
  const chartDark = theme !== "light";

  const fetchDnsData = () => {
    setLoading(true);
    void Promise.allSettled([
      getJson<DnsQuery[]>("/dns/queries?limit=250").then(setQueries),
      getJson<TopDnsDomain[]>("/dns/top?hours=24&limit=10").then(setTopDomains),
    ]).finally(() => setLoading(false));
  };

  useEffect(() => {
    fetchDnsData();
    const interval = setInterval(fetchDnsData, 10000);
    return () => clearInterval(interval);
  }, [live.snapshot?.timestamp]);

  const filteredQueries = useMemo(() => {
    const s = search.toLowerCase().trim();
    if (!s) return queries;
    return queries.filter(
      (q) =>
        q.query_name.toLowerCase().includes(s) ||
        q.record_type.toLowerCase().includes(s),
    );
  }, [queries, search]);

  const chartOption = useMemo(
    () => ({
      backgroundColor: "transparent",
      tooltip: {
        trigger: "axis",
        axisPointer: { type: "shadow" },
        backgroundColor: chartDark ? "rgba(15, 23, 42, 0.9)" : "rgba(255, 255, 255, 0.96)",
        borderColor: chartDark ? "rgba(255, 255, 255, 0.12)" : "rgba(15, 23, 42, 0.12)",
        textStyle: { color: chartDark ? "#f8fafc" : "#0f172a", fontSize: 12 },
        borderRadius: 12,
      },
      grid: { left: "3%", right: "4%", bottom: "3%", top: "10%", containLabel: true },
      xAxis: {
        type: "category",
        data: topDomains.map((d) => d.domain.length > 18 ? `${d.domain.slice(0, 16)}…` : d.domain),
        axisLine: { lineStyle: { color: chartDark ? "rgba(255, 255, 255, 0.1)" : "rgba(15, 23, 42, 0.14)" } },
        axisLabel: { color: chartDark ? "#94a3b8" : "#475569", fontSize: 10, rotate: 25 },
      },
      yAxis: {
        type: "value",
        splitLine: { lineStyle: { color: chartDark ? "rgba(255, 255, 255, 0.05)" : "rgba(15, 23, 42, 0.08)" } },
        axisLabel: { color: "#64748b", fontSize: 11 },
      },
      series: [
        {
          name: "Queries",
          type: "bar",
          data: topDomains.map((d) => d.count),
          itemStyle: {
            color: {
              type: "linear",
              x: 0,
              y: 0,
              x2: 0,
              y2: 1,
              colorStops: [
                { offset: 0, color: "#38bdf8" },
                { offset: 1, color: "#6366f1" },
              ],
            },
            borderRadius: [6, 6, 0, 0],
          },
        },
      ],
    }),
    [topDomains, chartDark],
  );

  return (
    <div className="space-y-6">
      <PageHeader
        icon={Search}
        iconColor="text-sky-400"
        iconBg="bg-sky-500/15"
        iconBorder="border-sky-500/25"
        category="NETWORK"
        title="DNS Query Intelligence"
        subtitle="Real-time resolution telemetry, domain frequency analysis & suspicious lookup heuristic tracking"
        actions={
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={fetchDnsData}
              disabled={loading}
              className="flex items-center gap-1.5 rounded-full bg-white/[0.04] px-3 py-1.5 text-xs font-medium text-slate-300 border border-white/[0.08] hover:bg-white/[0.08] cursor-pointer"
            >
              <RefreshCw className={`h-3.5 w-3.5 ${loading ? "animate-spin text-sky-400" : ""}`} />
              <span>Refresh</span>
            </button>
          </div>
        }
      />

      {/* Top Queried Domains Chart */}
      {topDomains.length > 0 && (
        <div className="glass-card rounded-3xl p-6">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2 text-sky-400">
              <Globe className="h-4 w-4" />
              <h3 className="text-sm font-semibold text-white">Top Resolved Domains (24h)</h3>
            </div>
            <span className="text-[11px] font-mono text-slate-400">
              {topDomains.reduce((a, b) => a + b.count, 0)} Total Requests
            </span>
          </div>
          <ReactECharts option={chartOption} style={{ height: 200 }} />
        </div>
      )}

      {/* DNS Query Table */}
      <div className="glass-card rounded-3xl p-6">
        <div className="flex flex-col sm:flex-row items-center justify-between gap-4 mb-5">
          <div className="flex items-center gap-2">
            <Filter className="h-4 w-4 text-slate-400" />
            <h3 className="text-sm font-semibold text-white">Live Query Stream</h3>
            <span className="rounded-full bg-blue-500/10 px-2.5 py-0.5 text-xs font-mono text-blue-400 border border-blue-500/20">
              {filteredQueries.length} Recorded
            </span>
          </div>

          <div className="relative w-full sm:w-72">
            <Search className="absolute left-3 top-2.5 h-3.5 w-3.5 text-slate-400" />
            <input
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder="Search domain or record..."
              aria-label="Search DNS queries"
              className="w-full rounded-2xl bg-white/[0.04] pl-9 pr-4 py-1.5 text-xs text-white placeholder-slate-500 border border-white/[0.08] focus:border-blue-500/50 focus:outline-none focus:ring-1 focus:ring-blue-500/30 font-mono"
            />
          </div>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full text-left text-xs">
            <thead className="text-[11px] uppercase tracking-wider text-slate-400 border-b border-white/[0.06]">
              <tr>
                <th scope="col" className="pb-3">Domain / Query Name</th>
                <th scope="col" className="pb-3">Type</th>
                <th scope="col" className="pb-3">Status</th>
                <th scope="col" className="pb-3">Assessment</th>
                <th scope="col" className="pb-3 text-right">Observed</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-white/[0.04]">
              {filteredQueries.slice(0, 100).map((q, i) => {
                const isSuspicious =
                  q.query_name.endsWith(".tk") ||
                  q.query_name.endsWith(".xyz") ||
                  q.query_name.endsWith(".top") ||
                  q.query_name.endsWith(".buzz");
                return (
                  <tr key={i} className="hover:bg-white/[0.02] transition-colors">
                    <td className="py-3 font-mono text-slate-200 flex items-center gap-2">
                      <Globe className="h-3.5 w-3.5 text-slate-500 shrink-0" />
                      <span className="truncate max-w-sm">{q.query_name}</span>
                    </td>
                    <td className="py-3">
                      <span className="rounded-md bg-white/[0.06] px-2 py-0.5 text-[10px] font-mono text-sky-400 border border-white/[0.06]">
                        {q.record_type}
                      </span>
                    </td>
                    <td className="py-3">
                      <span className="text-[11px] text-emerald-400 font-mono">RESOLVED</span>
                    </td>
                    <td className="py-3">
                      {isSuspicious ? (
                        <span className="flex items-center gap-1 text-[11px] text-amber-400 font-medium">
                          <ShieldAlert className="h-3.5 w-3.5" />
                          Uncommon TLD
                        </span>
                      ) : (
                        <span className="flex items-center gap-1 text-[11px] text-slate-400">
                          <ShieldCheck className="h-3.5 w-3.5 text-slate-500" />
                          Standard
                        </span>
                      )}
                    </td>
                    <td className="py-3 text-right font-mono text-slate-400">
                      {timeAgo(q.timestamp)}
                    </td>
                  </tr>
                );
              })}
              {filteredQueries.length === 0 && (
                <tr>
                  <td colSpan={5} className="text-center py-8 text-slate-500">
                    No DNS queries match your filter
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
