import { useEffect, useId, useRef, useState } from "react";
import { Download, FileSpreadsheet, FileCode, Check, TriangleAlert } from "lucide-react";
import { apiUrl } from "../lib";

interface ExportButtonProps {
  endpoint: "events" | "connections" | "metrics" | "ports";
  label?: string;
  hours?: number;
}

type Status = "idle" | "working" | "done" | "error";

export default function ExportButton({ endpoint, label = "Export", hours = 24 }: ExportButtonProps) {
  const [open, setOpen] = useState(false);
  const [status, setStatus] = useState<Status>("idle");
  const [error, setError] = useState<string | null>(null);
  const menuId = useId();
  const buttonRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // Close on Escape / outside click; return focus to the toggle.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setOpen(false);
        buttonRef.current?.focus();
      }
    };
    const onClick = (e: MouseEvent) => {
      if (
        menuRef.current &&
        !menuRef.current.contains(e.target as Node) &&
        !buttonRef.current?.contains(e.target as Node)
      ) {
        setOpen(false);
      }
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("mousedown", onClick);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("mousedown", onClick);
    };
  }, [open ]);

  const download = async (format: "csv" | "json") => {
    setStatus("working");
    setError(null);
    try {
      const query = new URLSearchParams({ format, hours: String(hours) });
      const res = await fetch(apiUrl(`/export/${endpoint}?${query.toString()}`));
      if (!res.ok) {
        let detail = `Export failed (${res.status})`;
        try {
          const body = (await res.json()) as { error?: string };
          if (body.error) detail = body.error;
        } catch {
          /* keep default */
        }
        throw new Error(detail);
      }
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `vigilon-${endpoint}.${format}`;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      setTimeout(() => URL.revokeObjectURL(url), 5000);
      setStatus("done");
      setOpen(false);
      setTimeout(() => setStatus("idle"), 3000);
    } catch (e) {
      setStatus("error");
      setError(e instanceof Error ? e.message : "Export failed");
    }
  };

  return (
    <div className="relative inline-block text-left">
      <button
        ref={buttonRef}
        type="button"
        onClick={() => setOpen(!open)}
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={menuId}
        className="flex items-center gap-1.5 rounded-full bg-white/[0.04] px-3 py-1.5 text-xs font-medium text-slate-300 border border-white/[0.08] hover:bg-white/[0.08] hover:text-white transition-all cursor-pointer shadow-sm"
        title="Export data as CSV or JSON"
      >
        <Download className="h-3.5 w-3.5 text-blue-400" aria-hidden="true" />
        <span>
          {status === "working" ? "Exporting…" : status === "done" ? "Exported ✓" : label}
        </span>
      </button>

      {open && (
        <div
          ref={menuRef}
          id={menuId}
          role="menu"
          aria-label="Export format"
          className="absolute right-0 mt-2 w-44 rounded-2xl bg-[#0c101a]/95 border border-white/[0.12] p-1.5 shadow-2xl backdrop-blur-2xl z-50"
        >
          <div className="px-2.5 py-1 text-[10px] uppercase font-semibold tracking-wider text-slate-400">
            Export Format
          </div>
          <button
            type="button"
            role="menuitem"
            onClick={() => void download("csv")}
            disabled={status === "working"}
            className="flex w-full items-center justify-between rounded-xl px-2.5 py-2 text-xs text-slate-200 hover:bg-white/[0.08] transition-colors cursor-pointer disabled:opacity-50"
          >
            <span className="flex items-center gap-2">
              <FileSpreadsheet className="h-4 w-4 text-emerald-400" aria-hidden="true" />
              <span>CSV Spreadsheet</span>
            </span>
            {status === "done" && <Check className="h-3 w-3 text-emerald-400" aria-hidden="true" />}
          </button>
          <button
            type="button"
            role="menuitem"
            onClick={() => void download("json")}
            disabled={status === "working"}
            className="flex w-full items-center justify-between rounded-xl px-2.5 py-2 text-xs text-slate-200 hover:bg-white/[0.08] transition-colors cursor-pointer disabled:opacity-50"
          >
            <span className="flex items-center gap-2">
              <FileCode className="h-4 w-4 text-amber-400" aria-hidden="true" />
              <span>JSON Raw Data</span>
            </span>
          </button>
        </div>
      )}
      <span className="sr-only" aria-live="polite">
        {status === "error" && error ? error : ""}
      </span>
      {status === "error" && error && (
        <p className="absolute right-0 mt-1 flex w-52 items-center gap-1.5 rounded-xl border border-rose-500/30 bg-rose-500/10 p-2 text-[11px] text-rose-200">
          <TriangleAlert className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
          {error}
        </p>
      )}
    </div>
  );
}
