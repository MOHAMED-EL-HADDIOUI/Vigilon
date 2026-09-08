import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { riskLabel } from "../lib";
import type { Risk } from "../types";

/* Shared design-system primitives. New pages should compose these instead
   of hand-rolling badges, dots, tiles, and empty states. */

export type Tone =
  | "emerald"
  | "rose"
  | "amber"
  | "blue"
  | "sky"
  | "slate"
  | "indigo"
  | "teal"
  | "purple";

const TONE_CLASSES: Record<Tone, string> = {
  emerald: "bg-emerald-500/10 text-emerald-400 border-emerald-500/20",
  rose: "bg-rose-500/10 text-rose-400 border-rose-500/20",
  amber: "bg-amber-500/10 text-amber-400 border-amber-500/20",
  blue: "bg-blue-500/10 text-blue-400 border-blue-500/20",
  sky: "bg-sky-500/10 text-sky-400 border-sky-500/20",
  slate: "bg-slate-500/10 text-slate-400 border-slate-500/20",
  indigo: "bg-indigo-500/10 text-indigo-400 border-indigo-500/20",
  teal: "bg-teal-500/10 text-teal-400 border-teal-500/20",
  purple: "bg-purple-500/10 text-purple-400 border-purple-500/20",
};

export function StatusBadge({
  tone,
  children,
  className = "",
}: {
  tone: Tone;
  children: ReactNode;
  className?: string;
}) {
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-[11px] font-medium border ${TONE_CLASSES[tone]} ${className}`}
    >
      {children}
    </span>
  );
}

const RISK_DOT: Record<Risk, string> = {
  INFO: "bg-emerald-400",
  LOW: "bg-sky-400",
  MEDIUM: "bg-amber-400 shadow-[0_0_8px_rgba(245,158,11,0.6)]",
  HIGH: "bg-rose-500 shadow-[0_0_8px_rgba(244,63,94,0.6)]",
  CRITICAL: "bg-rose-500 shadow-[0_0_8px_rgba(244,63,94,0.6)]",
};

/** Risk dot that never signals by color alone: label is exposed to AT + tooltip. */
export function RiskDot({ risk, size = "h-2.5 w-2.5" }: { risk: Risk; size?: string }) {
  const label = riskLabel(risk);
  return (
    <span
      role="img"
      aria-label={`Risk: ${label}`}
      title={label}
      className={`${size} rounded-full ${RISK_DOT[risk]}`}
    />
  );
}

export function riskTone(risk: Risk): Tone {
  if (risk === "HIGH" || risk === "CRITICAL") return "rose";
  if (risk === "MEDIUM") return "amber";
  if (risk === "LOW") return "sky";
  return "emerald";
}

export function StatTile({
  icon: Icon,
  tint,
  label,
  value,
  sub,
}: {
  icon: LucideIcon;
  tint: Tone;
  label: string;
  value: ReactNode;
  sub?: ReactNode;
}) {
  const iconTone: Record<Tone, string> = {
    emerald: "text-emerald-400",
    rose: "text-rose-400",
    amber: "text-amber-400",
    blue: "text-blue-400",
    sky: "text-sky-400",
    slate: "text-slate-400",
    indigo: "text-indigo-400",
    teal: "text-teal-400",
    purple: "text-purple-400",
  };
  return (
    <div className="rounded-2xl border border-white/[0.06] bg-white/[0.02] p-4">
      <div className="flex items-center gap-2 mb-2">
        <Icon className={`h-4 w-4 ${iconTone[tint]}`} aria-hidden="true" />
        <span className="text-[11px] font-semibold uppercase tracking-wider text-slate-400">
          {label}
        </span>
      </div>
      <div className="text-2xl font-bold text-white">{value}</div>
      {sub && <div className="text-[11px] text-slate-400 mt-0.5">{sub}</div>}
    </div>
  );
}

export function EmptyState({
  icon: Icon,
  title,
  hint,
}: {
  icon: LucideIcon;
  title: string;
  hint?: string;
}) {
  return (
    <div className="glass-card rounded-3xl p-8 text-center text-slate-400">
      <Icon className="h-10 w-10 text-slate-500 mx-auto mb-3" aria-hidden="true" />
      <p className="text-sm font-semibold text-white">{title}</p>
      {hint && <p className="text-xs text-slate-400 max-w-md mx-auto mt-1">{hint}</p>}
    </div>
  );
}
