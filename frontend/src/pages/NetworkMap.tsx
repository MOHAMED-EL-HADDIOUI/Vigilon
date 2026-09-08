import { useMemo, useState, type CSSProperties } from "react";
import { Link } from "react-router-dom";
import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  type Edge,
  type Node,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import {
  Network as NetworkIcon,
  Radio,
  Globe,
  Wifi,
} from "lucide-react";
import type { Live } from "../useLive";
import { formatBps, formatBytes } from "../lib";
import { countryFlag } from "../lib/flags";
import { useTheme } from "../hooks/useTheme";
import type { ListeningPort, RemoteEndpointGroup } from "../types";
import PageHeader from "../components/PageHeader";

export default function NetworkMap({ live }: { live: Live }) {
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const { theme } = useTheme();

  const host =
    live.snapshot?.device.hostname ??
    live.interfaces.find((i) => i.kind !== "loopback")?.name ??
    "Local Device";
  const ip =
    live.interfaces.find((i) => i.kind !== "loopback" && i.ips.length)?.ips[0] ?? "";
  const gw = live.interfaces.find((i) => i.gateway)?.gateway ?? "Local Gateway";

  const { nodes, edges } = useMemo(() => {
    const ns: Node[] = [
      {
        id: "internet",
        position: { x: 420, y: 10 },
        data: { label: "🌐 INTERNET\nExternal WAN", type: "internet" },
        style: nodeStyle("#38bdf8", "rgba(56, 189, 248, 0.15)"),
      },
      {
        id: "router",
        position: { x: 420, y: 110 },
        data: { label: `⚡ Gateway Router\n${gw}`, type: "router" },
        style: nodeStyle("#6366f1", "rgba(99, 102, 241, 0.15)"),
      },
      {
        id: "pc",
        position: { x: 420, y: 220 },
        data: { label: `💻 ${host}\n${ip}`, type: "pc" },
        style: nodeStyle("#10b981", "rgba(16, 185, 129, 0.15)"),
      },
    ];

    const es: Edge[] = [
      { id: "e1", source: "internet", target: "router", animated: true, style: { stroke: "#38bdf8", strokeWidth: 2 } },
      { id: "e2", source: "router", target: "pc", animated: true, style: { stroke: "#10b981", strokeWidth: 2 } },
    ];

    // Local listening ports
    live.ports.slice(0, 15).forEach((p, i) => {
      const id = `p-${p.port}-${i}`;
      const col = i % 5;
      const row = Math.floor(i / 5);
      const x = 50 + col * 140;
      const y = 370 + row * 90;
      ns.push({
        id,
        position: { x, y },
        data: {
          label: `:${p.port} (${p.protocol})\n${p.process_name ?? "system"}`,
          type: "port",
          details: p,
        },
        style: nodeStyle("#f59e0b", "rgba(245, 158, 11, 0.1)"),
      });
      es.push({
        id: `ep-${id}`,
        source: "pc",
        target: id,
        style: { stroke: "rgba(245, 158, 11, 0.5)", strokeWidth: 1.5 },
      });
    });

    // Group external remote endpoints
    const groupedRemotes = new Map<
      string,
      { count: number; processes: Set<string>; ports: Set<number>; addr: string; geo?: { country_code: string; country_name: string; city?: string | null } | null }
    >();

    live.connections
      .filter((c) => !c.remote_addr.startsWith("127.") && !c.remote_addr.startsWith("::1"))
      .forEach((c) => {
        if (!groupedRemotes.has(c.remote_addr)) {
          groupedRemotes.set(c.remote_addr, {
            count: 0,
            processes: new Set(),
            ports: new Set(),
            addr: c.remote_addr,
            geo: c.geo,
          });
        }
        const g = groupedRemotes.get(c.remote_addr)!;
        g.count++;
        if (c.process_name) g.processes.add(c.process_name);
        g.ports.add(c.remote_port);
        if (!g.geo && c.geo) g.geo = c.geo;
      });

    const remotes = Array.from(groupedRemotes.values()).slice(0, 12);
    remotes.forEach((g, i) => {
      const id = `r-${i}`;
      const col = i % 3;
      const row = Math.floor(i / 3);
      const x = 680 + col * 170;
      const y = 100 + row * 90;
      const flag = countryFlag(g.geo?.country_code);
      const geoLabel = g.geo ? ` ${g.geo.country_name}` : "";
      ns.push({
        id,
        position: { x, y },
        data: {
          label: `${flag} ${g.addr}\n${g.count} socket${g.count > 1 ? "s" : ""}${geoLabel}`,
          type: "remote",
          details: {
            addr: g.addr,
            count: g.count,
            processes: Array.from(g.processes),
            ports: Array.from(g.ports),
          },
        },
        style: nodeStyle("#f43f5e", "rgba(244, 63, 94, 0.1)"),
      });
      es.push({
        id: `er-${id}`,
        source: "pc",
        target: id,
        animated: true,
        style: { stroke: "#f43f5e", strokeWidth: 1.5 },
      });
    });

    return { nodes: ns, edges: es };
  }, [live.ports, live.connections, live.interfaces, host, ip, gw]);

  return (
    <div className="space-y-6">
      {/* Top Header with Brand Logo & Page Icon */}
      <PageHeader
        icon={NetworkIcon}
        iconColor="text-sky-400"
        iconBg="bg-sky-500/15"
        iconBorder="border-sky-500/25"
        category="TOPOLOGY"
        title="Network Topology Map"
        subtitle="Real-time socket topology, router gateway & remote peers"
        actions={
          <div className="flex items-center gap-2 text-xs font-mono text-slate-400">
            <span className="flex items-center gap-1.5 rounded-full bg-emerald-500/10 px-3 py-1 text-emerald-400 border border-emerald-500/20">
              ● {live.ports.length} Local Ports
            </span>
            <span className="flex items-center gap-1.5 rounded-full bg-rose-500/10 px-3 py-1 text-rose-400 border border-rose-500/20">
              ● {live.connections.length} Connections
            </span>
          </div>
        }
      >
        {/* Adapters Pills */}
        <div className="flex flex-wrap gap-2">
          {live.interfaces.slice(0, 8).map((i) => (
            <div
              key={i.name}
              className="flex items-center gap-2 rounded-xl bg-white/[0.03] px-3 py-1.5 text-xs font-mono border border-white/[0.05]"
            >
              <Wifi className="h-3 w-3 text-slate-400" />
              <span className="text-slate-200">{i.name}</span>
              <span className={`text-[10px] ${i.is_up ? "text-emerald-400" : "text-slate-500"}`}>
                ({i.is_up ? "UP" : "DOWN"})
              </span>
              <span className="text-slate-400">↓{formatBps(i.rx_bps)}</span>
              <span className="text-teal-400/70 text-[10px]">Σ {formatBytes(i.rx_total_bytes + i.tx_total_bytes)}</span>
            </div>
          ))}
        </div>
      </PageHeader>

      {/* Spatial Topology Flow Canvas */}
      <div
        role="application"
        aria-label="Network topology graph. Tab past the canvas for a keyboard-accessible endpoint list below."
        className="relative h-[60vh] min-h-[420px] overflow-hidden rounded-3xl border border-white/[0.08] bg-[#090d16]/80 backdrop-blur-2xl shadow-[0_12px_40px_rgba(0,0,0,0.6)]">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          fitView
          colorMode={theme === "light" ? "light" : "dark"}
          onNodeClick={(_, node) => setSelectedNode(node)}
        >
          <Background color="#1e293b" gap={24} size={1} />
          <MiniMap
            style={{
              backgroundColor: "rgba(15, 23, 42, 0.8)",
              border: "1px solid rgba(255, 255, 255, 0.1)",
              borderRadius: "12px",
            }}
            nodeColor="#38bdf8"
          />
          <Controls
            style={{
              backgroundColor: "rgba(15, 23, 42, 0.8)",
              border: "1px solid rgba(255, 255, 255, 0.1)",
              borderRadius: "12px",
              padding: "4px",
            }}
          />
        </ReactFlow>
      </div>

      {/* Keyboard-accessible endpoint list (canvas fallback) */}
      <div className="glass-card rounded-3xl p-6">
        <h3 className="mb-3 text-[11px] font-semibold uppercase tracking-wider text-slate-400">
          All endpoints (keyboard accessible)
        </h3>
        <ul className="grid gap-1.5 sm:grid-cols-2 lg:grid-cols-3 text-xs font-mono">
          {live.ports.slice(0, 15).map((p, i) => (
            <li key={`k-port-${p.port}-${i}`}>
              <span className="block truncate rounded-xl bg-white/[0.02] p-2 border border-white/[0.04] text-slate-300">
                <span className="text-amber-400 font-semibold">:{p.port}</span>
                <span className="text-slate-400"> {p.protocol} · {p.process_name ?? "system"}</span>
              </span>
            </li>
          ))}
          {live.connections.slice(0, 15).map((c, i) => (
            <li key={`k-conn-${i}`}>
              <Link
                to={`/inspector?ip=${encodeURIComponent(c.remote_addr)}`}
                className="block truncate rounded-xl bg-white/[0.02] p-2 border border-white/[0.04] text-slate-300 hover:bg-white/[0.05] hover:text-white focus-visible:outline-2 focus-visible:outline-blue-400"
              >
                <span className="mr-1">{countryFlag(c.geo?.country_code)}</span>
                {c.remote_addr}:{c.remote_port}
                <span className="text-slate-400"> · {c.process_name ?? "system"}</span>
              </Link>
            </li>
          ))}
        </ul>
      </div>
      {selectedNode && selectedNode.data.type === "port" && selectedNode.data.details != null && (() => {
        const d = selectedNode.data.details as ListeningPort;
        return (
          <div className="glass-card rounded-3xl p-6" aria-live="polite">
            <div className="flex items-center gap-2 mb-4 text-amber-400 font-semibold text-sm">
              <Radio className="h-4 w-4" />
              <span>Listening Port Inspection (:{d.port})</span>
            </div>
            <div className="grid grid-cols-2 gap-4 sm:grid-cols-4 text-xs">
              <div className="rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05]">
                <p className="text-slate-400 uppercase font-semibold text-[10px]">Protocol</p>
                <p className="font-mono text-sm text-white mt-0.5">{d.protocol}</p>
              </div>
              <div className="rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05]">
                <p className="text-slate-400 uppercase font-semibold text-[10px]">Bind Address</p>
                <p className="font-mono text-sm text-white mt-0.5">{d.address}</p>
              </div>
              <div className="rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05]">
                <p className="text-slate-400 uppercase font-semibold text-[10px]">PID</p>
                <p className="font-mono text-sm text-white mt-0.5">{d.pid ?? "N/A"}</p>
              </div>
              <div className="rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05]">
                <p className="text-slate-400 uppercase font-semibold text-[10px]">Process Name</p>
                <p className="font-semibold text-sm text-white mt-0.5">{d.process_name ?? "system"}</p>
              </div>
              {d.process_path && (
                <div className="col-span-2 sm:col-span-4 rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05]">
                  <p className="text-slate-400 uppercase font-semibold text-[10px]">Executable Path</p>
                  <p className="font-mono text-xs text-slate-300 break-all mt-0.5">{d.process_path}</p>
                </div>
              )}
            </div>
          </div>
        );
      })()}

      {selectedNode && selectedNode.data.type === "remote" && selectedNode.data.details != null && (() => {
        const d = selectedNode.data.details as RemoteEndpointGroup;
        return (
          <div className="glass-card rounded-3xl p-6" aria-live="polite">
            <div className="flex items-center justify-between gap-2 mb-4 text-rose-400 font-semibold text-sm">
              <span className="flex items-center gap-2">
                <Globe className="h-4 w-4" />
                <span>Remote Endpoint Inspection ({d.addr})</span>
              </span>
              <Link
                to={`/inspector?ip=${encodeURIComponent(d.addr)}`}
                className="rounded-full bg-rose-500/10 px-3 py-1 text-[11px] font-medium text-rose-300 border border-rose-500/25 hover:bg-rose-500/20 transition-colors"
              >
                Open in Inspector →
              </Link>
            </div>
            <div className="grid grid-cols-2 gap-4 sm:grid-cols-4 text-xs">
              <div className="rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05]">
                <p className="text-slate-400 uppercase font-semibold text-[10px]">Target Address</p>
                <p className="font-mono text-sm text-white mt-0.5">{d.addr}</p>
              </div>
              <div className="rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05]">
                <p className="text-slate-400 uppercase font-semibold text-[10px]">Active Sockets</p>
                <p className="font-mono text-sm text-rose-400 font-semibold mt-0.5">{d.count}</p>
              </div>
              <div className="rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05] col-span-2">
                <p className="text-slate-400 uppercase font-semibold text-[10px]">Destination Ports</p>
                <p className="font-mono text-xs text-slate-200 mt-0.5">{d.ports.join(", ")}</p>
              </div>
              <div className="col-span-2 sm:col-span-4 rounded-2xl bg-white/[0.03] p-3 border border-white/[0.05]">
                <p className="text-slate-400 uppercase font-semibold text-[10px]">Connecting Processes</p>
                <p className="font-medium text-xs text-slate-200 mt-0.5">
                  {d.processes.join(", ") || "Unattributed"}
                </p>
              </div>
            </div>
          </div>
        );
      })()}
    </div>
  );
}

function nodeStyle(border: string, bg: string): CSSProperties {
  return {
    background: bg,
    backdropFilter: "blur(16px)",
    WebkitBackdropFilter: "blur(16px)",
    color: "#f8fafc",
    border: `1px solid ${border}`,
    boxShadow: `0 8px 24px 0 rgba(0,0,0,0.5)`,
    borderRadius: "16px",
    padding: "10px 14px",
    fontSize: "12px",
    fontFamily: "Inter, -apple-system, sans-serif",
    fontWeight: 500,
    whiteSpace: "pre-line",
    textAlign: "center",
    cursor: "pointer",
  };
}
