"use client";

import { useState } from "react";
import Link from "next/link";
import { useAuth } from "@/hooks/use-auth";
import { CompleteProfileModal } from "@/components/onboarding/CompleteProfileModal";
import {
  AlertCircleIcon,
  ArrowRightIcon,
  BuildingIcon,
  CalendarIcon,
  CheckCircleIcon,
  ClockIcon,
  IndianRupeeIcon,
  LinkIcon,
  ReceiptIcon,
  RefreshCwIcon,
  TrendingDownIcon,
  TrendingUpIcon,
  UserPlusIcon,
  UsersIcon,
  WalletIcon,
} from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { useApi } from "@/lib/hooks/use-api";
import {
  formatCurrency,
  formatDate,
  formatDateTime,
  formatMembershipStatus,
  formatMembershipType,
} from "@/lib/utils";

interface AdminStats {
  members: { total: number; active: number; pending: number; expired: number };
  financial: {
    totalIncome: number;
    totalExpenses: number;
    pendingApprovals: number;
    netBalance: number;
  };
  approvals: { pending: number };
  recentActivity: Array<{
    id: string;
    action: string;
    description: string;
    createdAt: string;
    user: { id: string; name: string; role: string; memberId: string };
  }>;
  recentAudit: Array<{
    id: string;
    transactionSnapshot: Record<string, unknown>;
    createdAt: string;
    performedBy: { id: string; name: string; role: string; memberId: string };
  }>;
}

interface MemberStats {
  membership: {
    status: string;
    type: string | null;
    expiry: string | null;
    daysLeft: number | null;
  };
  payments: { total: number; lastPayment: string | null };
  subMembers: Array<{
    id: string;
    memberId: string;
    name: string;
    relation: string;
    createdAt: string;
  }>;
}

/** Category badge color — matches the audit-log page convention (emerald = income, rose = expense) */
function auditCategoryBadgeClass(snapshot: Record<string, unknown>): string {
  return snapshot.type === "CASH_IN"
    ? "border-emerald-200 bg-emerald-50 text-emerald-700"
    : "border-rose-200 bg-rose-50 text-rose-700";
}

/** Amount text color — green for income, red for expense */
function auditAmountClass(snapshot: Record<string, unknown>): string {
  return snapshot.type === "CASH_IN" ? "text-emerald-700" : "text-rose-700";
}

/** Human-readable category label */
function formatCategoryLabel(raw: string): string {
  switch (raw) {
    case "MEMBERSHIP":
      return "Membership";
    case "SPONSORSHIP":
      return "Sponsorship";
    case "EXPENSE":
      return "Expense";
    case "OTHER":
      return "Other";
    default:
      return raw
        .toLowerCase()
        .split("_")
        .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
        .join(" ");
  }
}

function membershipStatusVariant(
  status: string
): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case "ACTIVE":
      return "default";
    case "EXPIRED":
      return "destructive";
    case "PENDING_APPROVAL":
    case "PENDING_PAYMENT":
      return "secondary";
    case "SUSPENDED":
      return "outline";
    default:
      return "outline";
  }
}

function roleVariant(
  role: string
): "default" | "secondary" | "destructive" | "outline" {
  switch (role) {
    case "ADMIN":
      return "destructive";
    case "OPERATOR":
      return "secondary";
    default:
      return "outline";
  }
}

function StatCard({
  label,
  value,
  note,
  icon: Icon,
  tone = "blue",
}: {
  label: string;
  value: string;
  note: string;
  icon: React.ComponentType<{ className?: string }>;
  tone?: "blue" | "green" | "red" | "amber";
}) {
  const toneClasses = {
    blue: "bg-blue-50 text-blue-600",
    green: "bg-emerald-50 text-emerald-600",
    red: "bg-rose-50 text-rose-600",
    amber: "bg-amber-50 text-amber-600",
  }[tone];

  return (
    <Card className="overflow-hidden">
      <CardContent className="px-5 pb-5 pt-7 md:pt-8">
        <div className="flex items-start justify-between gap-4">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.2em] text-slate-400">
              {label}
            </p>
            <p className="mt-4 text-3xl font-semibold tracking-tight text-slate-900">
              {value}
            </p>
            <p className="mt-2 text-sm text-muted-foreground">{note}</p>
          </div>
          <div className={`flex h-12 w-12 items-center justify-center rounded-2xl ${toneClasses}`}>
            <Icon className="h-5 w-5" />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function ActionCard({
  href,
  title,
  description,
  icon: Icon,
}: {
  href: string;
  title: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
}) {
  return (
    <Link
      href={href}
      className="group rounded-[1.5rem] border border-white/70 bg-white/80 p-5 shadow-[0_24px_60px_-32px_rgba(15,23,42,0.35)] transition-all duration-200 hover:-translate-y-1 hover:border-sky-200 hover:bg-white"
    >
      <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-slate-950 text-white">
        <Icon className="h-5 w-5" />
      </div>
      <div className="mt-5 flex items-start justify-between gap-3">
        <div>
          <h3 className="text-base font-semibold text-slate-900">{title}</h3>
          <p className="mt-1 text-sm leading-6 text-muted-foreground">{description}</p>
        </div>
        <ArrowRightIcon className="mt-1 h-4 w-4 shrink-0 text-slate-400 transition-transform group-hover:translate-x-1 group-hover:text-slate-900" />
      </div>
    </Link>
  );
}

function PanelHeader({
  eyebrow,
  title,
  description,
  actionHref,
}: {
  eyebrow: string;
  title: string;
  description: string;
  actionHref?: string;
}) {
  return (
    <div className="mb-5 flex flex-wrap items-start justify-between gap-3">
      <div>
        <p className="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">
          {eyebrow}
        </p>
        <h2 className="mt-2 text-xl font-semibold tracking-tight text-slate-900">
          {title}
        </h2>
        <p className="mt-1 text-sm text-muted-foreground">{description}</p>
      </div>
      {actionHref ? (
        <Button asChild variant="outline" size="sm">
          <Link href={actionHref}>View all</Link>
        </Button>
      ) : null}
    </div>
  );
}

function AdminDashboard({ stats, role }: { stats: AdminStats; role: string }) {
  const isAdmin = role === "ADMIN";

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden border-none bg-[radial-gradient(circle_at_top_left,_rgba(96,165,250,0.32),_transparent_20rem),linear-gradient(135deg,#0f172a_0%,#172554_52%,#0f172a_100%)] text-white shadow-[0_32px_80px_-36px_rgba(15,23,42,0.9)]">
        <CardContent className="p-6 md:p-8">
          <div className="grid gap-8 xl:grid-cols-[1.3fr_0.7fr]">
            <div>
              <h2 className="mt-2 max-w-2xl text-xl font-semibold tracking-tight md:text-2xl">
                Deshapriya Park operations, all in one place.
              </h2>
              <p className="mt-3 max-w-2xl text-sm leading-6 text-slate-300">
                From member onboarding to payment collections and sponsorship
                tracking, your daily priorities are one click away. Use the
                quick actions below to get started.
              </p>

              <div className="mt-8 grid gap-3 sm:grid-cols-3">
                <div className="rounded-3xl border border-white/10 bg-white/10 p-4">
                  <p className="text-xs uppercase tracking-[0.22em] text-slate-300">
                    Active members
                  </p>
                  <p className="mt-3 text-2xl font-semibold">{stats.members.active}</p>
                </div>
                <div className="rounded-3xl border border-white/10 bg-white/10 p-4">
                  <p className="text-xs uppercase tracking-[0.22em] text-slate-300">
                    Pending approvals
                  </p>
                  <p className="mt-3 text-2xl font-semibold">{stats.approvals.pending}</p>
                </div>
                <div className="rounded-3xl border border-white/10 bg-white/10 p-4">
                  <p className="text-xs uppercase tracking-[0.22em] text-slate-300">
                    Net position
                  </p>
                  <p className="mt-3 text-2xl font-semibold">
                    {formatCurrency(stats.financial.netBalance)}
                  </p>
                </div>
              </div>
            </div>

            <div className="rounded-[1.75rem] border border-white/10 bg-white/10 p-5 backdrop-blur-sm">
              <p className="text-xs font-semibold uppercase tracking-[0.22em] text-slate-300">
                Operations pulse
              </p>
              <div className="mt-5 space-y-4">
                <div className="rounded-3xl bg-white/10 p-4">
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-sm text-slate-300">Collections</span>
                    <TrendingUpIcon className="h-4 w-4 text-emerald-300" />
                  </div>
                  <p className="mt-3 text-2xl font-semibold">
                    {formatCurrency(stats.financial.totalIncome)}
                  </p>
                </div>
                <div className="rounded-3xl bg-white/10 p-4">
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-sm text-slate-300">Expenses</span>
                    <TrendingDownIcon className="h-4 w-4 text-rose-300" />
                  </div>
                  <p className="mt-3 text-2xl font-semibold">
                    {formatCurrency(stats.financial.totalExpenses)}
                  </p>
                </div>
                <div className="rounded-3xl bg-white/10 p-4">
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-sm text-slate-300">
                      {isAdmin ? "Approvals waiting" : "Pending payments"}
                    </span>
                    <ClockIcon className="h-4 w-4 text-amber-300" />
                  </div>
                  <p className="mt-3 text-2xl font-semibold">
                    {isAdmin
                      ? stats.approvals.pending
                      : formatCurrency(stats.financial.pendingApprovals)}
                  </p>
                </div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      <div className="rounded-[1.75rem] border border-white/70 bg-white/80 p-5 shadow-[0_24px_60px_-32px_rgba(15,23,42,0.35)] md:p-6">
        <PanelHeader
          eyebrow="Actions"
          title="Quick actions"
          description="Jump straight into the operational flows used most often."
        />
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
          <ActionCard
            href="/dashboard/members"
            title="Add member"
            description="Create or review member profiles and update membership data."
            icon={UserPlusIcon}
          />
          <ActionCard
            href="/dashboard/cash"
            title="Record payment"
            description="Enter new payment activity and keep collections moving."
            icon={ReceiptIcon}
          />
          <ActionCard
            href="/dashboard/sponsorship"
            title="Add sponsor"
            description="Track sponsor information and manage contribution records."
            icon={BuildingIcon}
          />
          <ActionCard
            href="/dashboard/sponsorship"
            title="Generate sponsor link"
            description="Create a payment-ready sponsor link for external contributors."
            icon={LinkIcon}
          />
        </div>
      </div>

      <div className="grid gap-6 xl:grid-cols-2">
        <Card>
          <CardHeader>
            <PanelHeader
              eyebrow="Feed"
              title="Recent activity"
              description="The latest actions taken across the dashboard."
              actionHref="/dashboard/activity-log"
            />
          </CardHeader>
          <CardContent className="space-y-3">
            {stats.recentActivity.length === 0 ? (
              <p className="rounded-3xl bg-slate-50 px-4 py-6 text-sm text-muted-foreground">
                No activity yet.
              </p>
            ) : (
              stats.recentActivity.map((entry) => (
                <div
                  key={entry.id}
                  className="rounded-3xl border border-slate-100 bg-slate-50/80 p-4"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="text-sm font-medium text-slate-900">
                        {entry.description}
                      </p>
                      <div className="mt-2 flex flex-wrap items-center gap-2">
                        <span className="text-xs text-muted-foreground">{entry.user.name}</span>
                        <Badge variant={roleVariant(entry.user.role)}>{entry.user.role}</Badge>
                      </div>
                    </div>
                    <span className="shrink-0 text-xs text-muted-foreground">
                      {formatDateTime(entry.createdAt)}
                    </span>
                  </div>
                </div>
              ))
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <PanelHeader
              eyebrow="Audit"
              title="Recent audit entries"
              description="Financial actions with who, what, and when."
              actionHref="/dashboard/audit-log"
            />
          </CardHeader>
          <CardContent className="space-y-3">
            {stats.recentAudit.length === 0 ? (
              <p className="rounded-3xl bg-slate-50 px-4 py-6 text-sm text-muted-foreground">
                No audit entries yet.
              </p>
            ) : (
              stats.recentAudit.map((entry) => (
                <div
                  key={entry.id}
                  className="rounded-3xl border border-slate-100 bg-slate-50/80 p-4"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="flex flex-wrap items-center gap-2">
                        <Badge
                          variant="outline"
                          className={auditCategoryBadgeClass(entry.transactionSnapshot)}
                        >
                          {formatCategoryLabel(String(entry.transactionSnapshot.category ?? "Approved"))}
                        </Badge>
                        <span className={`text-sm font-semibold ${auditAmountClass(entry.transactionSnapshot)}`}>
                          {formatCurrency(Number(entry.transactionSnapshot.amount ?? 0))}
                        </span>
                      </div>
                      <div className="mt-2 flex flex-wrap items-center gap-2">
                        <span className="text-xs text-muted-foreground">
                          {entry.performedBy.name}
                        </span>
                        <Badge variant={roleVariant(entry.performedBy.role)}>
                          {entry.performedBy.role}
                        </Badge>
                      </div>
                    </div>
                    <span className="shrink-0 text-xs text-muted-foreground">
                      {formatDateTime(entry.createdAt)}
                    </span>
                  </div>
                </div>
              ))
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function MemberDashboard({
  stats,
  memberName,
}: {
  stats: MemberStats;
  memberName?: string | null;
}) {
  const daysLeft = stats.membership.daysLeft;

  let expiryTone = "text-emerald-600";
  if (daysLeft !== null) {
    if (daysLeft <= 0) expiryTone = "text-rose-600";
    else if (daysLeft <= 15) expiryTone = "text-amber-600";
  }

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden border-none bg-[radial-gradient(circle_at_top_left,_rgba(125,211,252,0.28),_transparent_18rem),linear-gradient(135deg,#e0f2fe_0%,#eff6ff_56%,#ffffff_100%)]">
        <CardContent className="p-6 md:p-8">
          <div className="grid gap-6 xl:grid-cols-[1.2fr_0.8fr]">
            <div>
              <Badge variant="outline" className="border-sky-200 bg-white/70 text-sky-700">
                Member area
              </Badge>
              <h2 className="mt-2 text-xl font-semibold tracking-tight text-slate-900 md:text-2xl">
                Namaskar{memberName ? `, ${memberName}` : ""}. Here is your overview.
              </h2>
              <p className="mt-3 max-w-2xl text-sm leading-6 text-muted-foreground">
                Check your membership status, view payment history, and stay on
                top of upcoming renewals — everything in one place.
              </p>
            </div>

            <div className="rounded-[1.75rem] border border-white/70 bg-white/75 p-5">
              <p className="text-xs font-semibold uppercase tracking-[0.22em] text-slate-400">
                Current status
              </p>
              <div className="mt-4 flex flex-wrap items-center gap-3">
                <Badge variant={membershipStatusVariant(stats.membership.status)}>
                  {formatMembershipStatus(stats.membership.status)}
                </Badge>
                <span className="text-sm text-muted-foreground">
                  {formatMembershipType(stats.membership.type)}
                </span>
              </div>
              <div className="mt-6 grid gap-3 sm:grid-cols-2">
                <div className="rounded-3xl bg-slate-50 p-4">
                  <p className="text-xs uppercase tracking-[0.18em] text-slate-400">
                    Expires on
                  </p>
                  <p className="mt-2 text-lg font-semibold text-slate-900">
                    {formatDate(stats.membership.expiry)}
                  </p>
                </div>
                <div className="rounded-3xl bg-slate-50 p-4">
                  <p className="text-xs uppercase tracking-[0.18em] text-slate-400">
                    Days remaining
                  </p>
                  <p className={`mt-2 text-lg font-semibold ${expiryTone}`}>
                    {daysLeft !== null ? (daysLeft <= 0 ? "Expired" : `${daysLeft} days`) : "—"}
                  </p>
                </div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {(daysLeft !== null && daysLeft <= 15) || stats.membership.status !== "ACTIVE" ? (
        <div
          className={`rounded-[1.5rem] border px-4 py-4 text-sm ${
            daysLeft !== null && daysLeft > 0
              ? "border-amber-200 bg-amber-50 text-amber-800"
              : "border-rose-200 bg-rose-50 text-rose-800"
          }`}
        >
          <div className="flex items-start gap-3">
            <AlertCircleIcon className="mt-0.5 h-4 w-4 shrink-0" />
            <p>
              {daysLeft !== null && daysLeft > 0
                ? `Your membership expires in ${daysLeft} day${daysLeft === 1 ? "" : "s"}. Renew now to avoid interruption.`
                : "Your membership has expired or is awaiting activation. Renew to regain full access."}
            </p>
          </div>
        </div>
      ) : null}

      <div className="grid gap-4 md:grid-cols-3">
        <StatCard
          label="Status"
          value={formatMembershipStatus(stats.membership.status)}
          note="Current membership state."
          icon={CheckCircleIcon}
          tone="blue"
        />
        <StatCard
          label="Total paid"
          value={formatCurrency(stats.payments.total)}
          note={
            stats.payments.lastPayment
              ? `Last payment on ${formatDate(stats.payments.lastPayment)}`
              : "No recorded payments yet."
          }
          icon={WalletIcon}
          tone="green"
        />
        <StatCard
          label="Renewal"
          value={daysLeft !== null ? (daysLeft <= 0 ? "Expired" : `${daysLeft} days`) : "—"}
          note="Time remaining on the current plan."
          icon={CalendarIcon}
          tone={daysLeft !== null && daysLeft <= 15 ? "amber" : "blue"}
        />
      </div>

      <div className="grid gap-6 xl:grid-cols-[0.95fr_1.05fr]">
        <Card>
          <CardHeader className="pt-8 md:pt-9">
            <PanelHeader
              eyebrow="Payments"
              title="Payment summary"
              description="A concise overview of what you have already paid."
            />
          </CardHeader>
          <CardContent>
            <div className="rounded-[1.5rem] bg-slate-50 p-5">
              <p className="text-xs uppercase tracking-[0.18em] text-slate-400">
                Total contributions
              </p>
              <p className="mt-3 text-3xl font-semibold tracking-tight text-emerald-600">
                {formatCurrency(stats.payments.total)}
              </p>
              {stats.payments.lastPayment ? (
                <div className="mt-4 flex items-center gap-2 text-sm text-muted-foreground">
                  <CalendarIcon className="h-4 w-4" />
                  Last payment: {formatDate(stats.payments.lastPayment)}
                </div>
              ) : null}
            </div>

            <Button asChild className="mt-5 w-full">
              <Link href="/dashboard/my-membership">
                <IndianRupeeIcon className="mr-2 h-4 w-4" />
                Pay / Renew Membership
              </Link>
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pt-8 md:pt-9">
            <PanelHeader
              eyebrow="Family"
              title="Sub-members"
              description="People currently associated with your membership."
            />
          </CardHeader>
          <CardContent className="space-y-3">
            {stats.subMembers.length === 0 ? (
              <p className="rounded-3xl bg-slate-50 px-4 py-6 text-sm text-muted-foreground">
                No sub-members have been added yet.
              </p>
            ) : (
              stats.subMembers.map((member) => (
                <div
                  key={member.id}
                  className="flex items-center justify-between gap-3 rounded-3xl border border-slate-100 bg-slate-50/80 p-4"
                >
                  <div className="min-w-0">
                    <p className="truncate text-sm font-semibold text-slate-900">
                      {member.name}
                    </p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {member.memberId} · {member.relation}
                    </p>
                  </div>
                  <Badge variant="outline">Sub-member</Badge>
                </div>
              ))
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

export default function DashboardPage() {
  const { user, loading: authLoading } = useAuth();
  const role = user?.role;
  const [profileModalOpen, setProfileModalOpen] = useState(false);

  // A member_id starting with "PENDING-" means the user hasn't completed onboarding yet
  const needsProfile = !!user?.memberId?.startsWith("PENDING-");
  const { data: stats, error, isLoading, mutate } = useApi<AdminStats | MemberStats>(
    user ? "/api/dashboard/stats" : null,
    {
      dedupingInterval: 30_000,
      revalidateOnFocus: true,
    }
  );

  if (authLoading || (isLoading && !stats)) {
    return (
      <div className="flex min-h-[50vh] items-center justify-center">
        <div className="rounded-[1.75rem] border border-white/70 bg-white/80 px-6 py-5 text-sm text-muted-foreground shadow-[0_24px_60px_-32px_rgba(15,23,42,0.35)]">
          <RefreshCwIcon className="mr-2 inline h-4 w-4 animate-spin" />
          Loading dashboard…
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex min-h-[50vh] items-center justify-center">
        <Card className="max-w-md">
          <CardContent className="p-8 text-center">
            <AlertCircleIcon className="mx-auto h-9 w-9 text-destructive" />
            <h2 className="mt-4 text-lg font-semibold text-slate-900">
              Failed to load stats
            </h2>
            <p className="mt-2 text-sm text-muted-foreground">{error.message}</p>
            <Button
              variant="outline"
              size="sm"
              className="mt-5"
              onClick={() => void mutate()}
            >
              Retry
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (!stats || !role) return null;

  return (
    <>
      <CompleteProfileModal
        open={profileModalOpen}
        prefillName={user?.username}
        onSuccess={() => {
          setProfileModalOpen(false);
          void mutate();
        }}
        onSkip={() => setProfileModalOpen(false)}
      />

      {needsProfile && (
        <div className="mb-6 rounded-2xl border-2 border-amber-300 bg-amber-50 p-5 shadow-sm">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <div className="flex items-start gap-3">
              <AlertCircleIcon className="mt-0.5 h-6 w-6 shrink-0 text-amber-600" />
              <div>
                <p className="text-base font-bold text-amber-900">
                  Your membership profile is incomplete
                </p>
                <p className="mt-1 text-sm text-amber-800">
                  You haven&apos;t filled in your personal details yet. Your membership
                  status and history won&apos;t be visible until you complete your profile.
                </p>
              </div>
            </div>
            <Button
              className="shrink-0 bg-amber-600 hover:bg-amber-700 text-white"
              onClick={() => setProfileModalOpen(true)}
            >
              Complete profile
            </Button>
          </div>
        </div>
      )}

      {role === "ADMIN" || role === "OPERATOR" || role === "ORGANISER" ? (
        <AdminDashboard stats={stats as AdminStats} role={role} />
      ) : (
        <MemberDashboard
          stats={stats as MemberStats}
          memberName={user?.username}
        />
      )}
    </>
  );
}
