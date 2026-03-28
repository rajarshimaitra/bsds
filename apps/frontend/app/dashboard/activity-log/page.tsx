"use client";

/**
 * System Activity Log — /dashboard/activity-log
 *
 * Read-only page. For financial activities, fetches and displays full
 * transaction details (matching the audit log modal structure).
 * Filters: user search, action dropdown, date range.
 */

import { useState } from "react";
import { useAuth } from "@/hooks/use-auth";
import { SearchIcon, RefreshCwIcon, EyeIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useApi } from "@/lib/hooks/use-api";
import { apiFetch } from "@/lib/api-client";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ActivityUser {
  id: string;
  name: string;
  role: string;
  memberId: string;
}

interface ActivityEntry {
  id: string;
  action: string;
  description: string;
  createdAt: string;
  approvalType?: string | null;
  approvalTypeLabel?: string | null;
  direction?: string | null;
  metadata?: Record<string, unknown> | null;
  user: ActivityUser | null;
}

interface ActivityLogResponse {
  data: ActivityEntry[];
  page: number;
  limit: number;
  total: number;
  totalPages: number;
}

// Transaction shape returned by /api/transactions/:id
interface TransactionDetail {
  id: string;
  type: string;
  category: string;
  amount: string | number;
  paymentMode: string;
  purpose: string;
  remark: string | null;
  sponsorPurpose: string | null;
  approvalStatus: string;
  approvalSource: string;
  memberId: string | null;
  member?: { id?: string; name: string; phone?: string } | null;
  sponsorId: string | null;
  sponsor?: { id?: string; name: string; company: string | null; phone?: string } | null;
  senderName: string | null;
  senderPhone: string | null;
  senderUpiId: string | null;
  senderBankAccount: string | null;
  senderBankName: string | null;
  sponsorSenderName?: string | null;
  sponsorSenderContact?: string | null;
  razorpayPaymentId: string | null;
  receiptNumber: string | null;
  createdAt: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatDateTime(iso: string): string {
  const d = new Date(iso);
  const dd = String(d.getDate()).padStart(2, "0");
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const yyyy = d.getFullYear();
  const hh = String(d.getHours()).padStart(2, "0");
  const min = String(d.getMinutes()).padStart(2, "0");
  return `${dd}/${mm}/${yyyy} ${hh}:${min}`;
}

function formatAmount(amount: string | number): string {
  const n = typeof amount === "string" ? parseFloat(amount) : amount;
  return `₹${n.toLocaleString("en-IN", { minimumFractionDigits: 0, maximumFractionDigits: 0 })}`;
}

function formatCurrency(amount: number): string {
  return new Intl.NumberFormat("en-IN", {
    style: "currency",
    currency: "INR",
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(amount);
}

function formatPaymentMode(mode: string): string {
  const MAP: Record<string, string> = {
    UPI: "UPI",
    BANK_TRANSFER: "Bank Transfer",
    CASH: "Cash",
  };
  return MAP[mode] ?? mode;
}

function formatCategoryLabel(category: string): string {
  const MAP: Record<string, string> = {
    MEMBERSHIP: "Membership",
    SPONSORSHIP: "Sponsorship",
    EXPENSE: "Expense",
    OTHER: "Other",
  };
  return MAP[category] ?? category.toLowerCase().split("_").map((p) => p.charAt(0).toUpperCase() + p.slice(1)).join(" ");
}

function formatSponsorPurpose(purpose: string): string {
  const MAP: Record<string, string> = {
    TITLE_SPONSOR: "Title Sponsor",
    GOLD_SPONSOR: "Gold Sponsor",
    SILVER_SPONSOR: "Silver Sponsor",
    FOOD_PARTNER: "Food Partner",
    MEDIA_PARTNER: "Media Partner",
    STALL_VENDOR: "Stall Vendor",
    MARKETING_PARTNER: "Marketing Partner",
  };
  return MAP[purpose] ?? purpose;
}

function roleBadgeVariant(role: string): "default" | "secondary" | "outline" {
  if (role === "ADMIN") return "default";
  if (role === "OPERATOR") return "secondary";
  return "outline";
}

function actionBadgeClass(action: string): string {
  if (action.includes("rejected")) return "bg-rose-100 text-rose-800 border-rose-200";
  if (action.includes("approved")) return "bg-emerald-100 text-emerald-800 border-emerald-200";
  return "bg-amber-100 text-amber-800 border-amber-200";
}

const FINANCIAL_ACTIONS = new Set([
  "transaction_created",
  "transaction_created_pending",
  "transaction_add_requested",
  "transaction_approved",
  "transaction_rejected",
  "membership_created",
  "membership_create_requested",
  "membership_approved",
  "membership_rejected",
  "sponsor_payment_received",
  "razorpay_payment_captured",
]);

function isFinancialAction(action: string): boolean {
  return FINANCIAL_ACTIONS.has(action);
}

function isMembershipApprovalType(approvalType: string | null | undefined): boolean {
  return approvalType === "MEMBERSHIP_APPROVAL" || approvalType === "MEMBERSHIP_PAYMENT_APPROVAL";
}

const ACTIVITY_ACTIONS = [
  "login_success", "login_failed", "password_changed",
  "member_created", "member_add_requested", "member_updated", "member_edit_requested",
  "member_deleted", "member_delete_requested",
  "sub_member_created", "sub_member_add_requested", "sub_member_updated",
  "sub_member_edit_requested", "sub_member_removed", "sub_member_remove_requested",
  "transaction_created", "transaction_created_pending", "transaction_add_requested",
  "transaction_updated", "transaction_edit_requested", "transaction_deleted",
  "transaction_delete_requested", "transaction_approved", "transaction_rejected",
  "membership_created", "membership_create_requested", "membership_approved",
  "membership_rejected", "membership_expiry_reminder_sent", "membership_expired",
  "approval_approved", "approval_rejected",
  "sponsor_created", "sponsor_updated", "sponsor_deleted",
  "sponsor_link_created", "sponsor_link_deactivated", "sponsor_payment_received",
  "razorpay_payment_captured", "payment_amount_mismatch", "webhook_rejected_invalid_signature",
  "receipt_generated", "whatsapp_notification_sent",
];

// ---------------------------------------------------------------------------
// Shared detail row
// ---------------------------------------------------------------------------

function DetailRow({ label, value, className }: { label: string; value: string; className?: string }) {
  return (
    <div className="grid grid-cols-1 gap-1 py-2 text-sm border-b border-muted/50 last:border-0 sm:grid-cols-[160px_1fr] sm:gap-2">
      <span className="text-muted-foreground shrink-0 text-xs font-medium sm:text-sm sm:font-normal">{label}</span>
      <span className={className ?? "font-medium break-words"}>{value || "—"}</span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Member profile section
// ---------------------------------------------------------------------------

function MemberProfileSection({ member }: { member: Record<string, unknown> }) {
  const subMembers = (member.subMembers ?? member.childMembers) as Array<Record<string, unknown>> | undefined;

  return (
    <div className="space-y-3">
      <div className="rounded-md border px-3">
        <DetailRow label="Full Name" value={(member.name as string) || "—"} />
        <DetailRow label="Phone" value={(member.phone as string) || "—"} />
        {typeof member.address === "string" && <DetailRow label="Address" value={member.address} />}
        {typeof member.email === "string" && <DetailRow label="Email" value={member.email} />}
        {typeof member.displayMembershipStatus === "string" && (
          <DetailRow label="Status" value={member.displayMembershipStatus} />
        )}
      </div>

      {subMembers && subMembers.length > 0 && (
        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
            Sub-Members ({subMembers.length})
          </p>
          <div className="space-y-2">
            {subMembers.map((sub, i) => (
              <div key={i} className="rounded-md border px-3 bg-muted/20">
                <DetailRow label="Name" value={(sub.name as string) || "—"} />
                {typeof sub.relation === "string" && <DetailRow label="Relation" value={sub.relation} />}
                {typeof sub.phone === "string" && <DetailRow label="Phone" value={sub.phone} />}
                {typeof sub.email === "string" && <DetailRow label="Email" value={sub.email} />}
                {typeof sub.memberId === "string" && (
                  <DetailRow label="Member ID" value={sub.memberId} className="font-mono text-xs font-medium" />
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Transaction details panel (same structure as audit log)
// ---------------------------------------------------------------------------

function TransactionDetailsPanel({
  txn,
  memberData,
  memberLoading,
}: {
  txn: TransactionDetail;
  memberData: Record<string, unknown> | null;
  memberLoading: boolean;
}) {
  const isMembership = txn.category === "MEMBERSHIP";
  const isSponsorship = txn.category === "SPONSORSHIP";

  return (
    <div className="space-y-4">
      {/* Core fields */}
      <div className="rounded-md border px-3">
        <div className="grid grid-cols-1 gap-1 py-2 text-sm border-b border-muted/50 sm:grid-cols-[160px_1fr] sm:gap-2">
          <span className="text-muted-foreground shrink-0 text-xs font-medium sm:text-sm sm:font-normal">Category</span>
          <Badge variant="outline" className={`text-xs w-fit ${
            txn.type === "CASH_IN"
              ? "border-emerald-200 bg-emerald-50 text-emerald-700"
              : "border-rose-200 bg-rose-50 text-rose-700"
          }`}>
            {formatCategoryLabel(txn.category)}
          </Badge>
        </div>
        <DetailRow
          label="Amount"
          value={formatAmount(txn.amount)}
          className={`font-semibold ${txn.type === "CASH_IN" ? "text-emerald-700" : txn.type === "CASH_OUT" ? "text-rose-700" : ""}`}
        />
        <DetailRow label="Payment Mode" value={formatPaymentMode(txn.paymentMode)} />
        {txn.approvalSource && (
          <DetailRow
            label="Approval Source"
            value={txn.approvalSource === "RAZORPAY_WEBHOOK" ? "Razorpay Webhook" : "Manual"}
          />
        )}
        {txn.receiptNumber && <DetailRow label="Receipt No." value={txn.receiptNumber} />}
        {txn.purpose && <DetailRow label="Purpose" value={txn.purpose} />}
        {txn.remark && <DetailRow label="Remark" value={txn.remark} />}
        {txn.razorpayPaymentId && (
          <DetailRow
            label="Razorpay Payment ID"
            value={txn.razorpayPaymentId}
            className="font-mono text-xs font-medium break-all"
          />
        )}
      </div>

      {/* Sent By (payer — inferred per category) */}
      {isMembership ? (
        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
            Sent By
          </p>
          <div className="rounded-md border px-3">
            <DetailRow
              label="Name"
              value={(memberData?.name as string) || txn.member?.name || "—"}
            />
            {((memberData?.phone as string) || txn.member?.phone) && (
              <DetailRow
                label="Contact"
                value={(memberData?.phone as string) || txn.member!.phone!}
              />
            )}
          </div>
        </div>
      ) : isSponsorship && (txn.sponsorSenderName || txn.sponsorSenderContact) ? (
        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
            Sent By
          </p>
          <div className="rounded-md border px-3">
            <DetailRow label="Name" value={txn.sponsorSenderName || "—"} />
            {txn.sponsorSenderContact && (
              <DetailRow label="Contact" value={txn.sponsorSenderContact} />
            )}
          </div>
        </div>
      ) : null}

      {/* Received By (collector — always shown) */}
      <div>
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
          Received By
        </p>
        <div className="rounded-md border px-3">
          <DetailRow label="Name" value={txn.senderName || "—"} />
          {txn.senderPhone && <DetailRow label="Contact" value={txn.senderPhone} />}
          {txn.senderUpiId && (
            <DetailRow label="UPI ID" value={txn.senderUpiId} className="font-mono text-xs font-medium" />
          )}
          {txn.senderBankAccount && (
            <DetailRow label="Bank Account" value={txn.senderBankAccount} className="font-mono text-xs font-medium" />
          )}
          {txn.senderBankName && <DetailRow label="Bank Name" value={txn.senderBankName} />}
        </div>
      </div>

      {/* Membership: primary member profile */}
      {isMembership && (
        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
            Primary Member
          </p>
          {memberLoading ? (
            <div className="rounded-md border px-3 py-4 space-y-2 animate-pulse">
              {[1, 2, 3].map((i) => <div key={i} className="h-4 bg-muted rounded w-3/4" />)}
            </div>
          ) : memberData ? (
            <MemberProfileSection member={memberData} />
          ) : txn.member ? (
            <div className="rounded-md border px-3">
              <DetailRow label="Name" value={txn.member.name} />
              {txn.member.phone && <DetailRow label="Phone" value={txn.member.phone} />}
            </div>
          ) : (
            <div className="rounded-md border px-3 py-3 text-sm text-muted-foreground">
              Member details unavailable
            </div>
          )}
        </div>
      )}

      {/* Sponsorship: sponsor details */}
      {isSponsorship && (
        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
            Sponsor Details
          </p>
          <div className="rounded-md border px-3">
            {txn.sponsor ? (
              <>
                <DetailRow label="Sponsor Name" value={txn.sponsor.name} />
                {txn.sponsor.company && <DetailRow label="Company" value={txn.sponsor.company} />}
                {txn.sponsor.phone && <DetailRow label="Phone" value={txn.sponsor.phone} />}
              </>
            ) : (
              <div className="py-3 text-sm text-muted-foreground">Sponsor details unavailable</div>
            )}
            {txn.sponsorPurpose && (
              <DetailRow label="Sponsor Purpose" value={formatSponsorPurpose(txn.sponsorPurpose)} />
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function ActivityLogPage() {
  const { user, loading: authLoading } = useAuth();

  // Filters
  const [filterAction, setFilterAction] = useState<string>("__all__");
  const [filterDateFrom, setFilterDateFrom] = useState("");
  const [filterDateTo, setFilterDateTo] = useState("");
  const [filterUserSearch, setFilterUserSearch] = useState("");
  const [filterPage, setFilterPage] = useState(1);

  // Detail modal
  const [selectedEntry, setSelectedEntry] = useState<ActivityEntry | null>(null);
  const [selectedTxData, setSelectedTxData] = useState<TransactionDetail | null>(null);
  const [selectedTxLoading, setSelectedTxLoading] = useState(false);
  const [selectedMemberData, setSelectedMemberData] = useState<Record<string, unknown> | null>(null);
  const [selectedMemberLoading, setSelectedMemberLoading] = useState(false);

  const params = new URLSearchParams();
  if (filterAction && filterAction !== "__all__") params.set("action", filterAction);
  if (filterDateFrom) params.set("dateFrom", filterDateFrom);
  if (filterDateTo) params.set("dateTo", filterDateTo);
  params.set("page", String(filterPage));
  params.set("limit", "20");

  const { data, error, isLoading, mutate } = useApi<ActivityLogResponse>(
    user ? `/api/activity-log?${params.toString()}` : null,
    { dedupingInterval: 60_000, revalidateOnFocus: false, keepPreviousData: true }
  );

  const serverEntries = data?.data ?? [];
  const entries = filterUserSearch.trim()
    ? serverEntries.filter((entry) =>
        entry.user?.name?.toLowerCase().includes(filterUserSearch.trim().toLowerCase())
      )
    : serverEntries;
  const pagination = data ?? { data: [], page: 1, limit: 20, total: 0, totalPages: 0 };
  const loading = authLoading || (isLoading && !data);

  function applyFilters() {
    if (filterPage === 1) { void mutate(); return; }
    setFilterPage(1);
  }

  function openModal(entry: ActivityEntry) {
    setSelectedEntry(entry);
    setSelectedTxData(null);
    setSelectedMemberData(null);
    setSelectedTxLoading(false);
    setSelectedMemberLoading(false);

    const transactionId = entry.metadata?.transactionId as string | undefined;
    if (isFinancialAction(entry.action) && transactionId) {
      setSelectedTxLoading(true);
      apiFetch(`/api/transactions/${transactionId}`)
        .then((r) => (r.ok ? r.json() : null))
        .then((d: TransactionDetail | null) => {
          if (!d) return;
          setSelectedTxData(d);
          if (d.category === "MEMBERSHIP" && d.memberId) {
            setSelectedMemberLoading(true);
            apiFetch(`/api/members/${d.memberId}`)
              .then((r) => (r.ok ? r.json() : null))
              .then((md) => { if (md) setSelectedMemberData(md as Record<string, unknown>); })
              .catch(() => {})
              .finally(() => setSelectedMemberLoading(false));
          }
        })
        .catch(() => {})
        .finally(() => setSelectedTxLoading(false));
    } else if (isMembershipApprovalType(entry.approvalType) && !isFinancialAction(entry.action)) {
      // member_created stores the UUID in memberRecordId (memberId is the formatted BSDS-YYYY-NNNN id)
      // sub_member_* actions store the parent UUID in parentMemberId
      // all other member actions store the UUID directly in memberId
      const memberId = (
        entry.metadata?.memberRecordId ??
        entry.metadata?.memberId ??
        entry.metadata?.parentMemberId
      ) as string | undefined;
      if (memberId) {
        setSelectedMemberLoading(true);
        apiFetch(`/api/members/${memberId}`)
          .then((r) => (r.ok ? r.json() : null))
          .then((md) => { if (md) setSelectedMemberData(md as Record<string, unknown>); })
          .catch(() => {})
          .finally(() => setSelectedMemberLoading(false));
      }
    }
  }

  function closeModal() {
    setSelectedEntry(null);
    setSelectedTxData(null);
    setSelectedMemberData(null);
  }

  // ---------------------------------------------------------------------------
  // Access check
  // ---------------------------------------------------------------------------

  const canView =
    user?.role === "ADMIN" || user?.role === "OPERATOR" || user?.role === "ORGANISER";

  if (!canView) {
    return (
      <div className="p-4 sm:p-6">
        <p className="text-destructive">Access denied. Admins and Operators only.</p>
      </div>
    );
  }

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  return (
    <div className="space-y-6 p-4 sm:p-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">System Activity Log</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Append-only record of all user and system actions. Read-only.
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={() => void mutate()} disabled={loading}>
          <RefreshCwIcon className="h-4 w-4 mr-2" />
          Refresh
        </Button>
      </div>

      {/* Filters */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base">Filters</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-3">
            <Input
              placeholder="Search by user name"
              value={filterUserSearch}
              onChange={(e) => { setFilterUserSearch(e.target.value); setFilterPage(1); }}
            />

            <Select
              value={filterAction}
              onValueChange={(v) => { setFilterAction(v); setFilterPage(1); }}
            >
              <SelectTrigger>
                <SelectValue placeholder="Action" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="__all__">All actions</SelectItem>
                {ACTIVITY_ACTIONS.map((a) => (
                  <SelectItem key={a} value={a}>{a}</SelectItem>
                ))}
              </SelectContent>
            </Select>

            <Input
              type="date"
              value={filterDateFrom}
              onChange={(e) => { setFilterDateFrom(e.target.value); setFilterPage(1); }}
            />
            <Input
              type="date"
              value={filterDateTo}
              onChange={(e) => { setFilterDateTo(e.target.value); setFilterPage(1); }}
            />
            <Button onClick={applyFilters} className="w-full">
              <SearchIcon className="h-4 w-4 mr-2" />
              Search
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Table */}
      <Card>
        <CardContent className="overflow-x-auto p-0">
          {loading ? (
            <div className="p-8 text-center text-muted-foreground">Loading...</div>
          ) : error ? (
            <div className="p-8 text-center text-destructive">{error.message}</div>
          ) : entries.length === 0 ? (
            <div className="p-8 text-center text-muted-foreground">No activity log entries found.</div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Date/Time</TableHead>
                  <TableHead>User</TableHead>
                  <TableHead>Approval Type</TableHead>
                  <TableHead>Action</TableHead>
                  <TableHead className="text-right">Details</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {entries.map((entry) => (
                  <TableRow
                    key={entry.id}
                    className="cursor-pointer hover:bg-sky-50/60"
                    onClick={() => openModal(entry)}
                  >
                    <TableCell className="whitespace-nowrap text-sm">
                      {formatDateTime(entry.createdAt)}
                    </TableCell>
                    <TableCell className="text-sm">
                      <span className="font-medium">{entry.user?.name ?? "SYSTEM"}</span>
                      {entry.user?.role && (
                        <Badge variant={roleBadgeVariant(entry.user.role)} className="ml-2 text-xs">
                          {entry.user.role}
                        </Badge>
                      )}
                    </TableCell>
                    <TableCell className="text-sm">
                      {entry.approvalTypeLabel ? (
                        <Badge variant="outline" className="text-xs">
                          {entry.approvalTypeLabel}
                        </Badge>
                      ) : "—"}
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline" className={`text-xs font-mono ${actionBadgeClass(entry.action)}`}>
                        {entry.action}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => { e.stopPropagation(); openModal(entry); }}
                      >
                        <EyeIcon className="h-4 w-4" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Pagination */}
      {pagination.totalPages > 1 && (
        <div className="flex items-center justify-between text-sm">
          <span className="text-muted-foreground">
            Page {pagination.page} of {pagination.totalPages} ({pagination.total} total)
          </span>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={filterPage <= 1}
              onClick={() => setFilterPage((p) => Math.max(1, p - 1))}
            >
              Previous
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={filterPage >= pagination.totalPages}
              onClick={() => setFilterPage((p) => p + 1)}
            >
              Next
            </Button>
          </div>
        </div>
      )}

      {/* Detail Modal */}
      <Dialog open={!!selectedEntry} onOpenChange={(open) => !open && closeModal()}>
        <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              Activity Entry Details
              {selectedEntry?.direction && (
                <Badge
                  variant="outline"
                  className={`text-xs ${
                    selectedEntry.direction === "INCOMING"
                      ? "border-emerald-200 bg-emerald-50 text-emerald-700"
                      : "border-rose-200 bg-rose-50 text-rose-700"
                  }`}
                >
                  {selectedEntry.direction === "INCOMING" ? "Incoming" : "Outgoing"}
                </Badge>
              )}
              {selectedEntry?.approvalTypeLabel && (
                <Badge variant="outline" className="text-xs">
                  {selectedEntry.approvalTypeLabel}
                </Badge>
              )}
            </DialogTitle>
            <DialogDescription>Full details of the selected activity log entry.</DialogDescription>
          </DialogHeader>

          {selectedEntry && (
            <div className="space-y-5">
              {/* Header summary */}
              <div className="rounded-md border px-3">
                <DetailRow label="Date/Time" value={formatDateTime(selectedEntry.createdAt)} />
                <div className="grid grid-cols-1 gap-1 py-2 text-sm border-b border-muted/50 sm:grid-cols-[160px_1fr] sm:gap-2">
                  <span className="text-muted-foreground shrink-0 text-xs font-medium sm:text-sm sm:font-normal">Action</span>
                  <Badge variant="outline" className={`text-xs font-mono w-fit ${actionBadgeClass(selectedEntry.action)}`}>
                    {selectedEntry.action}
                  </Badge>
                </div>
                <DetailRow label="Description" value={selectedEntry.description} />
              </div>

              {/* User details */}
              <div>
                <h3 className="font-semibold text-sm mb-2">Performed By</h3>
                <div className="rounded-md border px-3">
                  <DetailRow
                    label="Name"
                    value={`${selectedEntry.user?.name ?? "SYSTEM"}${selectedEntry.user?.role ? ` (${selectedEntry.user.role})` : ""}`}
                  />
                  {selectedEntry.user?.memberId && (
                    <DetailRow
                      label="Member ID"
                      value={selectedEntry.user.memberId}
                      className="font-mono text-xs font-medium"
                    />
                  )}
                  {selectedEntry.user?.id && (
                    <DetailRow
                      label="User ID"
                      value={selectedEntry.user.id}
                      className="font-mono text-xs font-medium break-all"
                    />
                  )}
                </div>
              </div>

              {/* Primary member (membership approval type, non-financial) */}
              {isMembershipApprovalType(selectedEntry.approvalType) && !isFinancialAction(selectedEntry.action) && (
                <div>
                  <h3 className="font-semibold text-sm mb-3">Primary Member</h3>
                  {selectedMemberLoading ? (
                    <div className="rounded-md border px-3 py-4 space-y-2 animate-pulse">
                      {[1, 2, 3].map((i) => <div key={i} className="h-4 bg-muted rounded w-3/4" />)}
                    </div>
                  ) : selectedMemberData ? (
                    <MemberProfileSection member={selectedMemberData} />
                  ) : (
                    <div className="rounded-md border px-3 py-3 text-sm text-muted-foreground">
                      Member details unavailable
                    </div>
                  )}
                </div>
              )}

              {/* Financial transaction details */}
              {isFinancialAction(selectedEntry.action) && (
                <div>
                  <h3 className="font-semibold text-sm mb-3">Transaction Details</h3>
                  {selectedTxLoading ? (
                    <div className="rounded-md border px-3 py-4 space-y-2 animate-pulse">
                      {[1, 2, 3, 4].map((i) => <div key={i} className="h-4 bg-muted rounded w-3/4" />)}
                    </div>
                  ) : selectedTxData ? (
                    <TransactionDetailsPanel
                      txn={selectedTxData}
                      memberData={selectedMemberData}
                      memberLoading={selectedMemberLoading}
                    />
                  ) : (
                    <div className="rounded-md border px-3 py-3 text-sm text-muted-foreground">
                      Transaction details unavailable
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
