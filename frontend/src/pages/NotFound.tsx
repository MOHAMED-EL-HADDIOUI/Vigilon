import { Link } from "react-router-dom";
import { Compass, LayoutDashboard } from "lucide-react";
import PageHeader from "../components/PageHeader";

export default function NotFound() {
  return (
    <div className="mx-auto max-w-xl space-y-6 py-10 text-center">
      <PageHeader
        icon={Compass}
        iconColor="text-slate-400"
        iconBg="bg-white/[0.05]"
        iconBorder="border-white/[0.08]"
        category="ROUTING"
        title="Page not found"
        subtitle="This address doesn't match any Vigilon view — the URL may be mistyped or outdated."
      />
      <Link
        to="/"
        className="inline-flex items-center gap-2 rounded-2xl bg-white/[0.08] px-5 py-2.5 text-sm font-medium text-white border border-white/[0.1] hover:bg-white/[0.14] transition-colors"
      >
        <LayoutDashboard className="h-4 w-4" aria-hidden="true" />
        Back to Overview
      </Link>
    </div>
  );
}
