import type { SVGProps } from "react";

export interface VigilonLogoProps extends SVGProps<SVGSVGElement> {
  size?: number | "sm" | "md" | "lg" | "xl";
  showGlow?: boolean;
}

const SIZE_MAP = {
  sm: 20,
  md: 28,
  lg: 36,
  xl: 48,
};

export default function VigilonLogo({
  size = "md",
  showGlow = true,
  className = "",
  ...props
}: VigilonLogoProps) {
  const dimension = typeof size === "number" ? size : SIZE_MAP[size] ?? 28;

  return (
    <div className={`relative inline-flex items-center justify-center ${className}`}>
      {showGlow && (
        <div
          className="absolute inset-0 rounded-full bg-blue-500/25 blur-md pointer-events-none"
          style={{ width: dimension, height: dimension }}
        />
      )}
      <svg
        width={dimension}
        height={dimension}
        viewBox="0 0 48 48"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        className="relative shrink-0"
        {...props}
      >
        <defs>
          <linearGradient id="vg-shield" x1="6" y1="4" x2="42" y2="44" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stopColor="#38bdf8" />
            <stop offset="50%" stopColor="#6366f1" />
            <stop offset="100%" stopColor="#a855f7" />
          </linearGradient>
          <linearGradient id="vg-core" x1="18" y1="16" x2="30" y2="32" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stopColor="#60a5fa" />
            <stop offset="100%" stopColor="#3b82f6" />
          </linearGradient>
          <linearGradient id="vg-ring" x1="12" y1="10" x2="36" y2="38" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stopColor="#38bdf8" stopOpacity="0.8" />
            <stop offset="100%" stopColor="#06b6d4" stopOpacity="0.2" />
          </linearGradient>
        </defs>

        {/* Outer Hex-Shield Perimeter */}
        <path
          d="M24 4L40 11V22C40 32.5 33.2 41.5 24 44C14.8 41.5 8 32.5 8 22V11L24 4Z"
          stroke="url(#vg-shield)"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          fill="rgba(15, 23, 42, 0.4)"
        />

        {/* Concentric Radar Wave / Dynamic Iris */}
        <circle
          cx="24"
          cy="22"
          r="10"
          stroke="url(#vg-ring)"
          strokeWidth="1.5"
          strokeDasharray="2 3"
        />

        {/* Inner Radar Crosshairs */}
        <line x1="24" y1="14" x2="24" y2="18" stroke="#38bdf8" strokeWidth="1.5" strokeLinecap="round" />
        <line x1="24" y1="26" x2="24" y2="30" stroke="#38bdf8" strokeWidth="1.5" strokeLinecap="round" />
        <line x1="16" y1="22" x2="20" y2="22" stroke="#38bdf8" strokeWidth="1.5" strokeLinecap="round" />
        <line x1="28" y1="22" x2="32" y2="22" stroke="#38bdf8" strokeWidth="1.5" strokeLinecap="round" />

        {/* Central Luminous Core / Vigil Beacon */}
        <circle cx="24" cy="22" r="3.5" fill="url(#vg-core)" />
        <circle cx="24" cy="22" r="1.5" fill="#ffffff" />
      </svg>
    </div>
  );
}
