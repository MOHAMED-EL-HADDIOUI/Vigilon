import type { Risk } from "./types";

export function apiUrl(path: string): string {
  return path.startsWith("/api") ? path : `/api${path}`;
}

export async function getJson<T>(path: string): Promise<T> {
  const res = await fetch(apiUrl(path));
  if (!res.ok) throw new Error(`${res.status} ${path}`);
  return res.json() as Promise<T>;
}

export function wsUrl(): string {
  const proto = location.protocol === "https:" ? "wss" : "ws";
  return `${proto}://${location.host}/api/ws`;
}

export function formatBps(n: number): string {
  if (n < 1024) return `${n.toFixed(0)} B/s`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB/s`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB/s`;
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 ** 2) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 ** 3) return `${(n / 1024 ** 2).toFixed(1)} MB`;
  return `${(n / 1024 ** 3).toFixed(1)} GB`;
}

export function riskLabel(risk: Risk): string {
  if (risk === "INFO") return "Observed";
  if (risk === "LOW") return "Unusual";
  if (risk === "MEDIUM") return "Suspicious";
  return "Investigate";
}

export function riskColor(risk: Risk): string {
  if (risk === "INFO") return "text-mute";
  if (risk === "LOW") return "text-info";
  if (risk === "MEDIUM") return "text-warn";
  return "text-hot";
}

export function timeAgo(iso: string): string {
  const t = new Date(iso).getTime();
  if (!iso || Number.isNaN(t)) return "—";
  const diff = Date.now() - t;
  if (diff < 0) return "just now";
  const s = Math.floor(diff / 1000);
  if (s < 60) return `${s}s ago`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h ago`;
  return `${Math.floor(h / 24)}d ago`;
}

export function indicatorText(raw: string): string {
  const map: Record<string, string> = {
    new_process: "New process — not previously observed on this system",
    first_external_destination: "First time this process connected to an external server",
    connection_shortly_after_process_start: "Network connection established shortly after process launch",
    unknown_process: "Process is not recognized from the established baseline",
    unusual_path: "Process running from an unusual filesystem location",
    high_connection_rate: "Unusually high rate of outbound connection attempts",
    port_scan_like: "Access pattern resembles a port scan",
    firewall_changed: "Firewall configuration was modified",
    dns_changed: "DNS server configuration was changed",
    gateway_changed: "Network gateway was changed",
    new_service: "A new system service was registered",
    multiple_failures: "Multiple authentication failures detected in a short window",
    persistent_connection: "Connection has been maintained for an extended period",
  };
  return map[raw] ?? raw.replaceAll("_", " ");
}
