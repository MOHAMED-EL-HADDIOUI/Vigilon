import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import VigilonLogo from "./VigilonLogo";

export interface PageHeaderProps {
  icon: LucideIcon;
  iconColor?: string;
  iconBg?: string;
  iconBorder?: string;
  category?: string;
  title: string;
  subtitle: string;
  actions?: ReactNode;
  children?: ReactNode;
}

export default function PageHeader({
  icon: Icon,
  iconColor = "text-blue-400",
  iconBg = "bg-blue-500/15",
  iconBorder = "border-blue-500/25",
  category = "OBSERVABILITY",
  title,
  subtitle,
  actions,
  children,
}: PageHeaderProps) {
  return (
    <div className="glass-card rounded-3xl p-6 relative overflow-hidden">
      {/* Subtle ambient corner glow */}
      <div className="absolute -top-12 -right-12 h-36 w-36 rounded-full bg-blue-500/5 blur-2xl pointer-events-none" />

      {/* Top brand identity line with Vigilon logo & category */}
      <div className="flex items-center gap-2 mb-3">
        <VigilonLogo size="sm" />
        <span className="text-[10px] font-semibold tracking-[0.2em] uppercase text-blue-400/90 font-mono">
          VIGILON // {category}
        </span>
      </div>

      {/* Main header row: Page Icon + Title/Subtitle + Actions */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="flex items-center gap-4">
          <div
            className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl ${iconBg} ${iconBorder} border ${iconColor} shadow-[0_4px_24px_rgba(0,0,0,0.35)] backdrop-blur-md transition-transform hover:scale-105 duration-200`}
          >
            <Icon className="h-6 w-6" />
          </div>
          <div>
            <h1 className="text-lg font-bold tracking-tight text-text">{title}</h1>
            <p className="text-xs text-mute mt-0.5">{subtitle}</p>
          </div>
        </div>

        {actions && <div className="flex items-center gap-2.5">{actions}</div>}
      </div>

      {/* Optional embedded content (e.g. search bars, segmented controls, metrics) */}
      {children && <div className="mt-5 pt-5 border-t border-white/[0.06]">{children}</div>}
    </div>
  );
}
