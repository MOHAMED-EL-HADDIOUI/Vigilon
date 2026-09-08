import { Suspense, lazy } from "react";
import { NavLink, Route, Routes } from "react-router-dom";
import {
  ShieldCheck,
  LayoutDashboard,
  Network,
  Radio,
  Cpu,
  Layers,
  ShieldAlert,
  Clock,
  History as HistoryIcon,
  Settings2,
  Sun,
  Moon,
  Globe,
  Monitor,
  PackageSearch,
} from "lucide-react";
import { useLive } from "./useLive";
import { useTheme } from "./hooks/useTheme";
import { timeAgo } from "./lib";
import VigilonLogo from "./components/VigilonLogo";
// Heavy chart/graph pages load on demand so the first paint stays light.
const Dashboard = lazy(() => import("./pages/Dashboard"));
const NetworkMap = lazy(() => import("./pages/NetworkMap"));
const DnsLog = lazy(() => import("./pages/DnsLog"));
import Processes from "./pages/Processes";
import Ports from "./pages/Ports";
import Security from "./pages/Security";
import Timeline from "./pages/Timeline";
import Settings from "./pages/Settings";
import Services from "./pages/Services";
import History from "./pages/History";
import Fleet from "./pages/Fleet";
import ConnectionInspector from "./pages/ConnectionInspector";
import NotFound from "./pages/NotFound";

const navGroups = [
  {
    name: "Monitor",
    items: [
      { to: "/", label: "Overview", icon: LayoutDashboard },
      { to: "/map", label: "Network", icon: Network },
      { to: "/inspector", label: "Inspector", icon: PackageSearch },
    ],
  },
  {
    name: "Inventory",
    items: [
      { to: "/ports", label: "Ports", icon: Radio },
      { to: "/processes", label: "Processes", icon: Cpu },
      { to: "/services", label: "Services", icon: Layers },
      { to: "/dns", label: "DNS", icon: Globe },
    ],
  },
  {
    name: "Detect",
    items: [
      { to: "/security", label: "Security", icon: ShieldAlert },
      { to: "/timeline", label: "Timeline (Live)", icon: Clock },
      { to: "/history", label: "Changes (Deltas)", icon: HistoryIcon },
    ],
  },
  {
    name: "Manage",
    items: [
      { to: "/fleet", label: "Fleet", icon: Monitor },
      { to: "/settings", label: "Settings", icon: Settings2 },
    ],
  },
] as const;

export default function App() {
  const live = useLive();
  const { theme, toggle: toggleTheme } = useTheme();
  const fw = live.snapshot?.firewall;
  const isProtected = fw?.enabled ?? false;
  // Freshness: WS "connected" alone is not liveness — the snapshot can be
  // stale while the socket is open. Flag data older than ~2 metrics ticks.
  const snapshotAgeMs = live.snapshot
    ? Date.now() - new Date(live.snapshot.timestamp).getTime()
    : Number.POSITIVE_INFINITY;
  const isStale =
    live.hydrated && (!live.snapshot || Number.isNaN(snapshotAgeMs) || snapshotAgeMs > 15000);

  return (
    <div className="relative flex min-h-screen flex-col bg-ink text-text selection:bg-blue-500/30 selection:text-blue-200">
      {/* Ambient background glows for Apple-grade visual depth */}
      <div className="pointer-events-none fixed inset-0 z-0 overflow-hidden">
        <div className="absolute -top-[20%] left-1/4 h-[500px] w-[600px] rounded-full bg-blue-600/10 blur-[130px]" />
        <div className="absolute top-[35%] -right-[10%] h-[450px] w-[500px] rounded-full bg-emerald-600/5 blur-[120px]" />
        <div className="absolute -bottom-[10%] left-1/3 h-[500px] w-[600px] rounded-full bg-indigo-600/10 blur-[140px]" />
      </div>

      {/* Floating Apple macOS Header */}
      <header className="sticky top-0 z-30 flex flex-wrap items-center justify-between gap-4 border-b border-white/[0.08] bg-panel px-6 py-3.5 backdrop-blur-2xl">
        <div className="flex items-center gap-3.5">
          <div className="flex h-10 w-10 items-center justify-center rounded-2xl bg-white/[0.04] border border-white/[0.08] shadow-[0_0_20px_rgba(59,130,246,0.3)] backdrop-blur-md">
            <VigilonLogo size={24} />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h1 className="text-sm font-semibold tracking-[0.2em] text-white">VIGILON</h1>
              <span className="rounded-full bg-blue-500/10 px-2 py-0.5 text-[10px] font-medium tracking-wide text-blue-400 border border-blue-500/20">
                LOCAL-FIRST
              </span>
            </div>
            <p className="text-[11px] font-normal tracking-wide text-slate-400">
              Observability & Security Core
            </p>
          </div>
        </div>

        {/* Live System Status Badges */}
        <div className="flex items-center gap-2.5">
          <div
            className={`flex items-center gap-2 rounded-full px-3 py-1 text-xs font-medium backdrop-blur-md border transition-all duration-300 ${
              live.connected
                ? "bg-emerald-500/10 text-emerald-400 border-emerald-500/20 shadow-[0_0_12px_rgba(16,185,129,0.15)]"
                : "bg-rose-500/10 text-rose-400 border-rose-500/20 animate-pulse"
            }`}
          >
            <span className="relative flex h-2 w-2">
              {live.connected && (
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75" />
              )}
              <span
                className={`relative inline-flex h-2 w-2 rounded-full ${
                  live.connected ? "bg-emerald-500" : "bg-rose-500"
                }`}
              />
            </span>
            <span>{live.connected ? "Live Stream" : "Reconnecting"}</span>
          </div>

          <div
            className={`flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium font-mono backdrop-blur-md border ${
              !live.hydrated
                ? "bg-white/[0.04] text-slate-400 border-white/[0.08]"
                : isStale
                ? "bg-amber-500/10 text-amber-400 border-amber-500/20"
                : "bg-white/[0.04] text-slate-300 border-white/[0.08]"
            }`}
            title={
              live.snapshot
                ? `Last telemetry sample: ${live.snapshot.timestamp}`
                : "No telemetry sample received yet"
            }
          >
            <Clock className="h-3.5 w-3.5" />
            <span>
              {!live.hydrated
                ? "Connecting…"
                : live.snapshot
                ? `Updated ${timeAgo(live.snapshot.timestamp)}${isStale ? " · Stale" : ""}`
                : "No data"}
            </span>
          </div>

          <div
            className={`flex items-center gap-1.5 rounded-full px-3 py-1 text-xs font-medium backdrop-blur-md border ${
              isProtected
                ? "bg-blue-500/10 text-blue-300 border-blue-500/20"
                : "bg-amber-500/10 text-amber-400 border-amber-500/20"
            }`}
          >
            <ShieldCheck className="h-3.5 w-3.5" />
            <span>{isProtected ? "System Protected" : "Observing"}</span>
            {fw?.profile && (
              <span className="text-[10px] text-slate-400 font-mono">({fw.profile})</span>
            )}
          </div>

          <button
            type="button"
            onClick={toggleTheme}
            className="flex h-8 w-8 items-center justify-center rounded-full bg-white/[0.04] border border-white/[0.08] text-slate-300 hover:text-white hover:bg-white/[0.1] transition-all cursor-pointer"
            title={`Switch to ${theme === "dark" ? "Light" : "Dark"} mode`}
            aria-label={`Activate ${theme === "dark" ? "light" : "dark"} mode (currently ${theme})`}
          >
            {theme === "dark" ? <Sun className="h-4 w-4 text-amber-400" /> : <Moon className="h-4 w-4 text-sky-400" />}
          </button>
        </div>
      </header>

      {/* Apple-style Segmented Dock Navigation */}
      <nav
        className="sticky top-[61px] z-20 flex justify-center border-b border-white/[0.06] bg-ink/70 px-4 py-2 backdrop-blur-xl"
        aria-label="Primary"
      >
        <div className="flex max-w-full items-stretch gap-1 overflow-x-auto rounded-2xl bg-white/[0.03] p-1 border border-white/[0.06] shadow-[inset_0_1px_1px_rgba(255,255,255,0.05)]">
          {navGroups.map((group, gi) => (
            <div key={group.name} className="flex items-center gap-1" role="group" aria-label={group.name}>
              {gi > 0 && <span className="mx-1 h-6 w-px bg-white/[0.08]" aria-hidden="true" />}
              <span className="hidden px-1 text-[9px] font-semibold uppercase tracking-[0.14em] text-slate-500 lg:inline">
                {group.name}
              </span>
              {group.items.map(({ to, label, icon: Icon }) => (
                <NavLink
                  key={to}
                  to={to}
                  end={to === "/"}
                  className={({ isActive }) =>
                    `flex items-center gap-2 rounded-xl px-3.5 py-1.5 text-xs font-medium transition-all duration-200 ${
                      isActive
                        ? "bg-white/[0.12] text-white shadow-[0_2px_8px_rgba(0,0,0,0.4)] border border-white/[0.12]"
                        : "text-slate-400 hover:text-slate-200 hover:bg-white/[0.04]"
                    }`
                  }
                >
                  <Icon className="h-3.5 w-3.5" aria-hidden="true" />
                  <span>{label}</span>
                </NavLink>
              ))}
            </div>
          ))}
        </div>
      </nav>

      {/* Daemon-unreachable banner: never reassure with zeroes on failure */}
      {live.backendDown && (
        <div
          role="alert"
          className="relative z-10 mx-auto mt-4 flex w-full max-w-7xl items-center gap-3 rounded-2xl border border-rose-500/30 bg-rose-500/10 px-5 py-3 text-sm text-rose-200"
        >
          <ShieldAlert className="h-5 w-5 shrink-0 text-rose-400" aria-hidden="true" />
          <div>
            <p className="font-semibold">Vigilon daemon unreachable</p>
            <p className="text-xs text-rose-200/80">
              The API at <span className="font-mono">/api</span> did not answer. Start the daemon
              (<span className="font-mono">vigilon</span> on 127.0.0.1:8745) and reload — empty
              tables below mean "no data," not "all clear."
            </p>
          </div>
        </div>
      )}

      {/* Main Canvas */}
      <main className="relative z-10 flex-1 px-5 py-6 max-w-7xl mx-auto w-full">
        <Suspense
          fallback={
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5" aria-label="Loading page">
              {Array.from({ length: 5 }).map((_, i) => (
                <div
                  key={i}
                  className="glass-card h-44 animate-pulse rounded-3xl"
                  aria-hidden="true"
                />
              ))}
            </div>
          }
        >
          <Routes>
            <Route path="/" element={<Dashboard live={live} />} />
            <Route path="/map" element={<NetworkMap live={live} />} />
            <Route path="/ports" element={<Ports live={live} />} />
            <Route path="/processes" element={<Processes live={live} />} />
            <Route path="/security" element={<Security live={live} />} />
            <Route path="/dns" element={<DnsLog live={live} />} />
            <Route path="/inspector" element={<ConnectionInspector live={live} />} />
            <Route path="/fleet" element={<Fleet live={live} />} />
            <Route path="/timeline" element={<Timeline live={live} />} />
            <Route path="/history" element={<History />} />
            <Route path="/services" element={<Services live={live} />} />
            <Route path="/settings" element={<Settings live={live} />} />
            <Route path="*" element={<NotFound />} />
          </Routes>
        </Suspense>
      </main>

      {/* Footer Branding */}
      <footer className="relative z-10 border-t border-white/[0.06] py-5 text-center text-xs text-slate-500">
        <div className="flex flex-col sm:flex-row items-center justify-center gap-2 sm:gap-3">
          <div className="flex items-center gap-1.5 text-slate-400">
            <VigilonLogo size={16} showGlow={false} />
            <span className="font-semibold text-slate-300">Vigilon Engine</span>
          </div>
          <span className="hidden sm:inline">•</span>
          <span>Local-First System Observability</span>
          <span className="hidden sm:inline">•</span>
          <span className="font-mono text-[11px] text-slate-400">Zero Cloud Phone-Home</span>
        </div>
      </footer>
    </div>
  );
}
