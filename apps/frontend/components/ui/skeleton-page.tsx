import { cn } from "@/lib/utils";

type SkeletonVariant =
  | "dashboard"
  | "members"
  | "cash"
  | "approvals"
  | "audit-log"
  | "activity-log"
  | "sponsorship"
  | "my-membership";

interface SkeletonPageProps {
  variant: SkeletonVariant;
}

function SkeletonBlock({ className }: { className?: string }) {
  return <div aria-hidden className={cn("animate-pulse rounded-2xl bg-slate-200/80", className)} />;
}

function PageHeader({ actionCount = 1 }: { actionCount?: number }) {
  return (
    <div className="flex flex-wrap items-start justify-between gap-3">
      <div className="space-y-3">
        <SkeletonBlock className="h-8 w-56" />
        <SkeletonBlock className="h-4 w-72 max-w-[80vw]" />
      </div>
      <div className="flex w-full flex-wrap gap-2 sm:w-auto">
        {Array.from({ length: actionCount }).map((_, index) => (
          <SkeletonBlock key={index} className="h-10 w-full sm:w-32" />
        ))}
      </div>
    </div>
  );
}

function StatGrid({ count }: { count: number }) {
  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
      {Array.from({ length: count }).map((_, index) => (
        <div key={index} className="rounded-3xl border border-slate-200/80 bg-white/80 p-5 shadow-sm">
          <div className="flex items-start justify-between gap-4">
            <div className="space-y-3">
              <SkeletonBlock className="h-3 w-24" />
              <SkeletonBlock className="h-8 w-32" />
              <SkeletonBlock className="h-4 w-40" />
            </div>
            <SkeletonBlock className="h-12 w-12 rounded-2xl" />
          </div>
        </div>
      ))}
    </div>
  );
}

function FilterCard({ count = 3 }: { count?: number }) {
  return (
    <div className="rounded-3xl border border-slate-200/80 bg-white/80 p-5 shadow-sm">
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {Array.from({ length: count }).map((_, index) => (
          <SkeletonBlock key={index} className="h-10 w-full" />
        ))}
      </div>
    </div>
  );
}

function TableCard({
  rows = 6,
  columns = 5,
  showHeader = true,
}: {
  rows?: number;
  columns?: number;
  showHeader?: boolean;
}) {
  return (
    <div className="overflow-hidden rounded-3xl border border-slate-200/80 bg-white/80 shadow-sm">
      {showHeader ? (
        <div className="border-b border-slate-100 px-5 py-4">
          <SkeletonBlock className="h-5 w-40" />
        </div>
      ) : null}
      <div className="space-y-4 px-5 py-4">
        <div className="grid gap-3" style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}>
          {Array.from({ length: columns }).map((_, index) => (
            <SkeletonBlock key={index} className="h-4 w-full" />
          ))}
        </div>
        {Array.from({ length: rows }).map((_, rowIndex) => (
          <div
            key={rowIndex}
            className="grid gap-3 border-t border-slate-100 pt-4"
            style={{ gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}
          >
            {Array.from({ length: columns }).map((_, columnIndex) => (
              <SkeletonBlock key={columnIndex} className="h-4 w-full" />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

function SideListCard({ items = 4 }: { items?: number }) {
  return (
    <div className="rounded-3xl border border-slate-200/80 bg-white/80 p-5 shadow-sm">
      <div className="space-y-4">
        <SkeletonBlock className="h-5 w-36" />
        {Array.from({ length: items }).map((_, index) => (
          <div key={index} className="rounded-2xl border border-slate-100 bg-slate-50/70 p-4">
            <SkeletonBlock className="h-4 w-32" />
            <SkeletonBlock className="mt-3 h-3 w-24" />
          </div>
        ))}
      </div>
    </div>
  );
}

function BannerCard() {
  return (
    <div className="rounded-3xl border border-slate-200/80 bg-slate-50/80 p-4 shadow-sm">
      <SkeletonBlock className="h-4 w-full max-w-2xl" />
      <SkeletonBlock className="mt-3 h-4 w-2/3" />
    </div>
  );
}

export function SkeletonPage({ variant }: SkeletonPageProps) {
  if (variant === "dashboard") {
    return (
      <div className="space-y-6 p-4 sm:p-6">
        <div className="rounded-[1.75rem] border border-slate-200/80 bg-[linear-gradient(135deg,#dbeafe_0%,#eff6ff_45%,#f8fafc_100%)] p-6 shadow-sm md:p-8">
          <div className="grid gap-8 xl:grid-cols-[1.3fr_0.7fr]">
            <div className="space-y-4">
              <SkeletonBlock className="h-8 w-64" />
              <SkeletonBlock className="h-4 w-full max-w-2xl" />
              <SkeletonBlock className="h-4 w-5/6 max-w-2xl" />
            </div>
            <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-1">
              {Array.from({ length: 2 }).map((_, index) => (
                <div key={index} className="rounded-3xl border border-white/80 bg-white/75 p-5">
                  <SkeletonBlock className="h-3 w-24" />
                  <SkeletonBlock className="mt-4 h-8 w-28" />
                  <SkeletonBlock className="mt-3 h-4 w-40" />
                </div>
              ))}
            </div>
          </div>
        </div>

        <div className="grid gap-4 md:grid-cols-3">
          {Array.from({ length: 3 }).map((_, index) => (
            <div key={index} className="rounded-[1.5rem] border border-slate-200/80 bg-white/80 p-5 shadow-sm">
              <SkeletonBlock className="h-12 w-12 rounded-2xl" />
              <SkeletonBlock className="mt-5 h-5 w-32" />
              <SkeletonBlock className="mt-3 h-4 w-full" />
              <SkeletonBlock className="mt-2 h-4 w-2/3" />
            </div>
          ))}
        </div>

        <div className="grid gap-6 xl:grid-cols-2">
          <SideListCard items={5} />
          <SideListCard items={5} />
        </div>
      </div>
    );
  }

  if (variant === "cash") {
    return (
      <div className="space-y-6 p-4 sm:p-6">
        <PageHeader actionCount={2} />
        <StatGrid count={4} />
        <FilterCard count={6} />
        <TableCard rows={7} columns={7} />
      </div>
    );
  }

  if (variant === "my-membership") {
    return (
      <div className="space-y-6 p-4 sm:p-6">
        <PageHeader />
        <BannerCard />
        <div className="grid gap-6 xl:grid-cols-[1.2fr_0.8fr]">
          <div className="space-y-6">
            <div className="rounded-3xl border border-slate-200/80 bg-white/80 p-5 shadow-sm">
              <SkeletonBlock className="h-6 w-40" />
              <div className="mt-5 flex flex-wrap gap-3">
                <SkeletonBlock className="h-6 w-24 rounded-full" />
                <SkeletonBlock className="h-6 w-32 rounded-full" />
              </div>
              <div className="mt-6 grid gap-4 sm:grid-cols-2">
                {Array.from({ length: 4 }).map((_, index) => (
                  <div key={index}>
                    <SkeletonBlock className="h-3 w-24" />
                    <SkeletonBlock className="mt-3 h-5 w-32" />
                  </div>
                ))}
              </div>
            </div>
            <TableCard rows={5} columns={4} />
          </div>
          <SideListCard items={4} />
        </div>
      </div>
    );
  }

  if (variant === "sponsorship") {
    return (
      <div className="space-y-6 p-4 sm:p-6">
        <PageHeader actionCount={2} />
        <StatGrid count={3} />
        <FilterCard count={3} />
        <TableCard rows={6} columns={6} />
      </div>
    );
  }

  if (variant === "members") {
    return (
      <div className="space-y-6 p-4 sm:p-6">
        <PageHeader />
        <div className="grid gap-6 lg:grid-cols-[1.5fr_0.9fr]">
          <div className="space-y-4">
            <FilterCard count={2} />
            <TableCard rows={7} columns={5} showHeader={false} />
          </div>
          <SideListCard items={4} />
        </div>
      </div>
    );
  }

  if (variant === "approvals") {
    return (
      <div className="space-y-6 p-4 sm:p-6">
        <PageHeader />
        <FilterCard count={3} />
        <TableCard rows={6} columns={6} />
      </div>
    );
  }

  return (
    <div className="space-y-6 p-4 sm:p-6">
      <PageHeader />
      <FilterCard count={variant === "audit-log" ? 4 : 5} />
      <TableCard rows={6} columns={variant === "audit-log" ? 5 : 4} />
    </div>
  );
}
