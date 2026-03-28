"use client";

/**
 * Financial Audit Log — /dashboard/audit-log
 *
 * Read-only page. No create/edit/delete buttons.
 * Detail modal: full transaction details, member profile (phone/address/submembers),
 * sponsor details, received-by info. No stored snapshot.
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
import { ReceiptView } from "@/components/receipts/ReceiptView";
import type { ReceiptData } from "@/lib/receipt-utils";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Performer {
  id: string;
  name: string;
  role: string;
  memberId: string;
}

interface ReceiptBreakdownItem {
  label: string;
  amount: number;
}

interface TransactionReceipt {
  type: string;
  purpose: string | null;
  breakdown: ReceiptBreakdownItem[] | null;
  memberName: string | null;
  memberCode: string | null;
  membershipStart: string | null;
  membershipEnd: string | null;
}

interface TransactionDetail {
  id: string;
  type: string;
  category: string;
  amount: string;
  paymentMode: string;
  purpose: string;
  remark: string | null;
  sponsorPurpose: string | null;
  approvalStatus: string;
  approvalSource: string;
  memberId: string | null;
  member: {
    id?: string;
    name: string;
    phone?: string;
  } | null;
  sponsorId: string | null;
  sponsor: {
    id?: string;
    name: string;
    company: string | null;
    phone?: string;
  } | null;
  senderName: string | null;
  senderPhone: string | null;
  senderUpiId: string | null;
  senderBankAccount: string | null;
  senderBankName: string | null;
  sponsorSenderName: string | null;
  sponsorSenderContact: string | null;
  razorpayPaymentId: string | null;
  razorpayOrderId: string | null;
  receiptNumber: string | null;
  receipt: TransactionReceipt | null;
  createdAt: string;
}

interface AuditEntry {
  id: string;
  eventType: string;
  approvalType: string;
  approvalTypeLabel: string;
  direction: string | null;
  transactionSnapshot: Record<string, unknown>;
  transactionId: string;
  performedById: string;
  createdAt: string;
  performedBy: Performer;
  transaction: TransactionDetail | null;
}

interface AuditLogResponse {
  data: AuditEntry[];
  page: number;
  limit: number;
  total: number;
  totalPages: number;
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

function formatDateShort(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString("en-IN", {
    day: "2-digit",
    month: "long",
    year: "numeric",
  });
}

function formatCategoryLabel(category: string): string {
  switch (category) {
    case "MEMBERSHIP": return "Membership";
    case "SPONSORSHIP": return "Sponsorship";
    case "EXPENSE": return "Expense";
    case "OTHER": return "Other";
    default:
      return category.toLowerCase().split("_").map((p) => p.charAt(0).toUpperCase() + p.slice(1)).join(" ");
  }
}

function formatPaymentMode(mode: string): string {
  switch (mode) {
    case "UPI": return "UPI";
    case "BANK_TRANSFER": return "Bank Transfer";
    case "CASH": return "Cash";
    default: return mode;
  }
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

function getEntryCategory(entry: AuditEntry): string | null {
  if (entry.transaction?.category) return entry.transaction.category;
  return typeof entry.transactionSnapshot.category === "string"
    ? entry.transactionSnapshot.category
    : null;
}

function isIncomingEntry(entry: AuditEntry): boolean {
  if (entry.transaction?.type) return entry.transaction.type === "CASH_IN";
  return entry.transactionSnapshot.type === "CASH_IN";
}

function getCategoryBadgeClass(entry: AuditEntry): string {
  return isIncomingEntry(entry)
    ? "border-emerald-200 bg-emerald-50 text-emerald-700"
    : "border-rose-200 bg-rose-50 text-rose-700";
}

function isMembershipCategory(category: string | null): boolean {
  return category === "MEMBERSHIP";
}

function isSponsorshipCategory(category: string | null): boolean {
  return category === "SPONSORSHIP";
}

function getTableCounterparty(entry: AuditEntry): string {
  const category = getEntryCategory(entry);
  if (isMembershipCategory(category)) {
    return entry.transaction?.member?.name
      ?? entry.transaction?.senderName
      ?? String(entry.transactionSnapshot.senderName ?? "—");
  }
  if (isSponsorshipCategory(category)) {
    return entry.transaction?.sponsor?.name
      ?? entry.transaction?.senderName
      ?? String(entry.transactionSnapshot.senderName ?? "—");
  }
  return entry.transaction?.senderName ?? String(entry.transactionSnapshot.senderName ?? "—");
}

const TRANSACTION_CATEGORIES = ["MEMBERSHIP", "SPONSORSHIP", "EXPENSE", "OTHER"];

// ---------------------------------------------------------------------------
// Approval status badge
// ---------------------------------------------------------------------------

function ApprovalBadge({ status }: { status: string | null | undefined }) {
  if (!status) return <span className="text-muted-foreground text-sm">—</span>;
  const styles: Record<string, string> = {
    APPROVED: "border-emerald-200 bg-emerald-50 text-emerald-700",
    PENDING: "border-amber-200 bg-amber-50 text-amber-700",
    REJECTED: "border-rose-200 bg-rose-50 text-rose-700",
  };
  const labels: Record<string, string> = {
    APPROVED: "Approved",
    PENDING: "Pending",
    REJECTED: "Rejected",
  };
  return (
    <Badge variant="outline" className={`text-xs ${styles[status] ?? ""}`}>
      {labels[status] ?? status}
    </Badge>
  );
}

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
// Member profile section (phone, address, submembers)
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
// Transaction details panel
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
  const isMembership = isMembershipCategory(txn.category);
  const isSponsorship = isSponsorshipCategory(txn.category);
  const isExpense = txn.category === "EXPENSE" || txn.type === "CASH_OUT";

  return (
    <div className="space-y-4">
      {/* Core transaction fields */}
      <div className="rounded-md border px-3">
        <DetailRow
          label="Amount"
          value={formatAmount(txn.amount)}
          className={`font-semibold ${txn.type === "CASH_IN" ? "text-emerald-700" : txn.type === "CASH_OUT" ? "text-rose-700" : ""}`}
        />
        <DetailRow label="Payment Mode" value={formatPaymentMode(txn.paymentMode)} />
        <div className="grid grid-cols-1 gap-1 py-2 text-sm border-b border-muted/50 last:border-0 sm:grid-cols-[160px_1fr] sm:gap-2">
          <span className="text-muted-foreground shrink-0 text-xs font-medium sm:text-sm sm:font-normal">Approval Status</span>
          <ApprovalBadge status={txn.approvalStatus} />
        </div>
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
      ) : isExpense && !isSponsorship && (txn.sponsorSenderName || txn.sponsorSenderContact) ? (
        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground mb-2">
            Sent By
          </p>
          <div className="rounded-md border px-3">
            <DetailRow label="Name" value={txn.sponsorSenderName || "—"} />
            {txn.sponsorSenderContact && (
              <DetailRow label="Phone" value={txn.sponsorSenderContact} />
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

      {/* Membership fee breakdown */}
      {isMembership && txn.receipt && (
        <div className="rounded-xl border border-slate-200 bg-slate-50 p-4 space-y-3">
          <p className="text-[11px] font-semibold uppercase tracking-widest text-slate-500">Fee Breakdown</p>
          {txn.receipt.purpose && (
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">Purpose</span>
              <span className="font-medium text-right max-w-[60%]">{txn.receipt.purpose}</span>
            </div>
          )}
          {txn.receipt.breakdown && txn.receipt.breakdown.length > 0 && (
            <div className="space-y-1.5">
              {txn.receipt.breakdown.map((item) => (
                <div key={`${item.label}-${item.amount}`} className="flex justify-between text-sm">
                  <span className="text-slate-700">{item.label}</span>
                  <span className="font-medium tabular-nums">{formatCurrency(item.amount)}</span>
                </div>
              ))}
              <div className="flex justify-between text-sm font-semibold border-t border-slate-200 pt-2 mt-1">
                <span>Total</span>
                <span className="tabular-nums">
                  {formatCurrency(txn.receipt.breakdown.reduce((s, i) => s + i.amount, 0))}
                </span>
              </div>
            </div>
          )}
          {txn.receipt.membershipStart && txn.receipt.membershipEnd && (
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">Period</span>
              <span className="font-medium text-right">
                {formatDateShort(txn.receipt.membershipStart)} – {formatDateShort(txn.receipt.membershipEnd)}
              </span>
            </div>
          )}
        </div>
      )}

      {/* Sponsorship: sponsor details + purpose */}
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

export default function AuditLogPage() {
  const { user, loading: authLoading } = useAuth();

  // Filters
  const [filterDateFrom, setFilterDateFrom] = useState("");
  const [filterDateTo, setFilterDateTo] = useState("");
  const [filterCategory, setFilterCategory] = useState<string>("__all__");
  const [filterPage, setFilterPage] = useState(1);

  // Detail modal
  const [selectedEntry, setSelectedEntry] = useState<AuditEntry | null>(null);
  const [selectedMemberData, setSelectedMemberData] = useState<Record<string, unknown> | null>(null);
  const [selectedMemberLoading, setSelectedMemberLoading] = useState(false);
  const [selectedReceiptData, setSelectedReceiptData] = useState<ReceiptData | null>(null);
  const [selectedReceiptLoading, setSelectedReceiptLoading] = useState(false);
  const [selectedReceiptError, setSelectedReceiptError] = useState<string | null>(null);

  const params = new URLSearchParams();
  if (filterCategory && filterCategory !== "__all__") params.set("category", filterCategory);
  if (filterDateFrom) params.set("dateFrom", filterDateFrom);
  if (filterDateTo) params.set("dateTo", filterDateTo);
  params.set("page", String(filterPage));
  params.set("limit", "20");

  const { data, error, isLoading, mutate } = useApi<AuditLogResponse>(
    user ? `/api/audit-log?${params.toString()}` : null,
    { dedupingInterval: 60_000, revalidateOnFocus: false, keepPreviousData: true }
  );

  const entries = data?.data ?? [];
  const pagination = data ?? { data: [], page: 1, limit: 20, total: 0, totalPages: 0 };
  const loading = authLoading || (isLoading && !data);

  function applyFilters() {
    if (filterPage === 1) { void mutate(); return; }
    setFilterPage(1);
  }

  function openModal(entry: AuditEntry) {
    setSelectedEntry(entry);
    setSelectedMemberData(null);
    setSelectedMemberLoading(false);
    setSelectedReceiptData(null);
    setSelectedReceiptError(null);

    const category = entry.transaction?.category
      ?? (typeof entry.transactionSnapshot.category === "string" ? entry.transactionSnapshot.category : null);
    const memberId = entry.transaction?.memberId;

    if (category === "MEMBERSHIP" && memberId) {
      setSelectedMemberLoading(true);
      apiFetch(`/api/members/${memberId}`)
        .then((r) => (r.ok ? r.json() : null))
        .then((d) => { if (d) setSelectedMemberData(d as Record<string, unknown>); })
        .catch(() => {})
        .finally(() => setSelectedMemberLoading(false));
    }

    if (entry.transaction?.receiptNumber && entry.transaction.id) {
      setSelectedReceiptLoading(true);
      apiFetch(`/api/receipts/${entry.transaction.id}`)
        .then((r) => (r.ok ? r.json() : Promise.reject(new Error("Failed to load receipt"))))
        .then((d) => setSelectedReceiptData(d as ReceiptData))
        .catch((err: unknown) => setSelectedReceiptError(err instanceof Error ? err.message : "Failed to load receipt"))
        .finally(() => setSelectedReceiptLoading(false));
    }
  }

  function closeModal() {
    setSelectedEntry(null);
    setSelectedMemberData(null);
    setSelectedReceiptData(null);
    setSelectedReceiptError(null);
  }

  // ---------------------------------------------------------------------------
  // Access check
  // ---------------------------------------------------------------------------

  const canView = user?.role === "ADMIN" || user?.role === "OPERATOR" || user?.role === "ORGANISER";

  if (!canView) {
    return (
      <div className="p-4 sm:p-6">
        <p className="text-destructive">Access denied. Admins and Operators only.</p>
      </div>
    );
  }

  return (
    <div className="space-y-6 p-4 sm:p-6">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">Financial Audit Log</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Append-only record of all financial events. Read-only.
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
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
            <Select
              value={filterCategory}
              onValueChange={(v) => { setFilterCategory(v); setFilterPage(1); }}
            >
              <SelectTrigger>
                <SelectValue placeholder="Transaction category" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="__all__">All categories</SelectItem>
                {TRANSACTION_CATEGORIES.map((category) => (
                  <SelectItem key={category} value={category}>{formatCategoryLabel(category)}</SelectItem>
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
            <div className="p-8 text-center text-muted-foreground">No audit log entries found.</div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Date/Time</TableHead>
                  <TableHead>Category</TableHead>
                  <TableHead>Sender / Receiver</TableHead>
                  <TableHead>Payment Mode</TableHead>
                  <TableHead className="text-right">Amount</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Performer</TableHead>
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
                      {getEntryCategory(entry) ? (
                        <Badge variant="outline" className={`text-xs ${getCategoryBadgeClass(entry)}`}>
                          {formatCategoryLabel(getEntryCategory(entry)!)}
                        </Badge>
                      ) : "—"}
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      <div className="flex flex-col">
                        <span>{getTableCounterparty(entry)}</span>
                        {isMembershipCategory(getEntryCategory(entry)) && (
                          <span className="text-xs text-muted-foreground">Primary member</span>
                        )}
                        {isSponsorshipCategory(getEntryCategory(entry)) && (
                          <span className="text-xs text-muted-foreground">Sponsor</span>
                        )}
                      </div>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {entry.transaction?.paymentMode
                        ? formatPaymentMode(entry.transaction.paymentMode)
                        : "—"}
                    </TableCell>
                    <TableCell className="text-right font-mono font-medium text-sm">
                      {entry.transaction ? (
                        <span className={isIncomingEntry(entry) ? "text-emerald-700" : "text-rose-700"}>
                          {formatAmount(entry.transaction.amount)}
                        </span>
                      ) : "—"}
                    </TableCell>
                    <TableCell>
                      <ApprovalBadge status={entry.transaction?.approvalStatus} />
                    </TableCell>
                    <TableCell className="text-sm">
                      {entry.performedBy?.name ?? "SYSTEM"}
                      {entry.performedBy?.role && (
                        <span className="ml-1 text-xs text-muted-foreground">
                          ({entry.performedBy.role})
                        </span>
                      )}
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
        <DialogContent className={`${selectedEntry?.transaction?.receiptNumber ? "w-[min(96vw,1120px)] max-w-none" : "max-w-2xl"} max-h-[90vh] overflow-y-auto`}>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              Audit Entry Details
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
            <DialogDescription>Full details of the selected audit log entry.</DialogDescription>
          </DialogHeader>

          {selectedEntry && (
            <div className="space-y-5">
              {/* Header summary */}
              <div className="rounded-md border px-3">
                <DetailRow label="Date/Time" value={formatDateTime(selectedEntry.createdAt)} />
                {getEntryCategory(selectedEntry) && (
                  <div className="grid grid-cols-1 gap-1 py-2 text-sm border-b border-muted/50 last:border-0 sm:grid-cols-[160px_1fr] sm:gap-2">
                    <span className="text-muted-foreground shrink-0 text-xs font-medium sm:text-sm sm:font-normal">Category</span>
                    <Badge variant="outline" className={`text-xs w-fit ${getCategoryBadgeClass(selectedEntry)}`}>
                      {formatCategoryLabel(getEntryCategory(selectedEntry)!)}
                    </Badge>
                  </div>
                )}
                <DetailRow
                  label="Performed By"
                  value={`${selectedEntry.performedBy?.name ?? "SYSTEM"}${selectedEntry.performedBy?.role ? ` (${selectedEntry.performedBy.role})` : ""}`}
                />
                {selectedEntry.performedBy?.memberId && (
                  <DetailRow
                    label="Member ID"
                    value={selectedEntry.performedBy.memberId}
                    className="font-mono text-xs font-medium"
                  />
                )}
              </div>

              {/* Transaction details */}
              {selectedEntry.transaction && (
                <div>
                  <h3 className="font-semibold text-sm mb-3">Transaction Details</h3>
                  <TransactionDetailsPanel
                    txn={selectedEntry.transaction}
                    memberData={selectedMemberData}
                    memberLoading={selectedMemberLoading}
                  />
                </div>
              )}

              {/* Receipt */}
              {selectedEntry.transaction?.receiptNumber && (
                <div>
                  <h3 className="font-semibold text-sm mb-3">Receipt</h3>
                  {selectedReceiptLoading && (
                    <div className="rounded-md border px-4 py-6 animate-pulse space-y-3">
                      {[1, 2, 3].map((i) => (
                        <div key={i} className="h-4 bg-muted rounded w-3/4" />
                      ))}
                    </div>
                  )}
                  {selectedReceiptError && !selectedReceiptLoading && (
                    <div className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
                      {selectedReceiptError}
                    </div>
                  )}
                  {selectedReceiptData && !selectedReceiptLoading && (
                    <ReceiptView receipt={selectedReceiptData} />
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
