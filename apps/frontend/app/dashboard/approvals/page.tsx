"use client";

import { useState } from "react";
import { useAuth } from "@/hooks/use-auth";
import useSWRInfinite from "swr/infinite";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useToast } from "@/components/ui/use-toast";
import { apiFetcher, revalidateApiPrefixes } from "@/lib/hooks/use-api";
import { apiFetch } from "@/lib/api-client";
import type { ReceiptData } from "@/lib/receipt-utils";
import {
  CheckCircle,
  XCircle,
  Clock,
  Eye,
} from "lucide-react";

import {
  APPROVAL_TYPE_LABELS,
  DIRECTION_LABELS,
  ENTITY_TYPE_LABELS,
  ACTION_LABELS,
  STATUS_CLASSES,
  entityApiUrl,
  formatDate,
  type ApprovalRecord,
  type ApprovalsResponse,
} from "./constants";
import { ApprovalDetail } from "./approval-detail";

// ---------------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------------

export default function ApprovalsPage() {
  const { user, loading: authLoading } = useAuth();
  const { toast } = useToast();

  // Filters
  const [approvalTypeFilter, setApprovalTypeFilter] = useState("ALL");
  const [statusFilter, setStatusFilter] = useState("PENDING");
  const limit = 20;

  // Detail modal
  const [selected, setSelected] = useState<ApprovalRecord | null>(null);
  const [detailOpen, setDetailOpen] = useState(false);
  const [actionNotes, setActionNotes] = useState("");
  const [actionLoading, setActionLoading] = useState<"approve" | "reject" | null>(null);

  // Live entity fetch for detail modal
  const [entityData, setEntityData] = useState<Record<string, unknown> | null>(null);
  const [entityLoading, setEntityLoading] = useState(false);
  const [memberData, setMemberData] = useState<Record<string, unknown> | null>(null);
  const [memberLoading, setMemberLoading] = useState(false);
  const [receiptData, setReceiptData] = useState<ReceiptData | null>(null);
  const [receiptLoading, setReceiptLoading] = useState(false);
  const [receiptError, setReceiptError] = useState<string | null>(null);
  const [parentMemberData, setParentMemberData] = useState<Record<string, unknown> | null>(null);
  const [parentMemberLoading, setParentMemberLoading] = useState(false);
  const filters = new URLSearchParams({
    limit: String(limit),
    status: statusFilter,
  });
  if (approvalTypeFilter !== "ALL") filters.set("approvalType", approvalTypeFilter);

  const {
    data: approvalsPages,
    error,
    isLoading,
    isValidating,
    setSize,
    size,
  } = useSWRInfinite<ApprovalsResponse>(
    (pageIndex, previousPageData) => {
      if (!user) {
        return null;
      }

      if (previousPageData && previousPageData.page >= previousPageData.totalPages) {
        return null;
      }

      const params = new URLSearchParams(filters);
      params.set("page", String(pageIndex + 1));
      return `/api/approvals?${params.toString()}`;
    },
    apiFetcher,
    {
      dedupingInterval: 5_000,
      revalidateOnFocus: true,
      initialSize: 1,
    }
  );

  const firstPage = approvalsPages?.[0];
  const approvals = approvalsPages?.flatMap((page) => page.data) ?? [];
  const total = firstPage?.total ?? 0;
  const totalPages = firstPage?.totalPages ?? 1;
  const pendingCount = firstPage?.pendingCount ?? 0;
  const loading = authLoading || (isLoading && !approvalsPages);
  const loadingMore = isValidating && size > (approvalsPages?.length ?? 0);

  function actionBadgeClass(action: string): string {
    if (/delete|remove/.test(action)) return "bg-rose-100 text-rose-800";
    if (/edit/.test(action)) return "bg-amber-100 text-amber-800";
    return "bg-emerald-100 text-emerald-800";
  }

  function directionBadgeClass(direction: string | null) {
    if (direction === "INCOMING") return "bg-emerald-100 text-emerald-800";
    if (direction === "OUTGOING") return "bg-rose-100 text-rose-800";
    return "bg-slate-100 text-slate-700";
  }

  function openDetail(approval: ApprovalRecord) {
    setSelected(approval);
    setActionNotes("");
    setEntityData(null);
    setMemberData(null);
    setReceiptData(null);
    setReceiptError(null);
    setParentMemberData(null);
    setDetailOpen(true);

    if (approval.action === "add_sub_member" || approval.action === "edit_sub_member") {
      const parentId = (
        approval.newData?.parentMemberId ?? approval.previousData?.parentMemberId
      ) as string | undefined;
      if (parentId) {
        setParentMemberLoading(true);
        apiFetch(`/api/members/${parentId}`)
          .then((r) => (r.ok ? r.json() : null))
          .then((data) => { if (data) setParentMemberData(data); })
          .catch(() => {})
          .finally(() => setParentMemberLoading(false));
      }
    }

    const url = entityApiUrl(approval.entityType, approval.entityId, approval.action);
    if (!url) return;

    setEntityLoading(true);
    apiFetch(url)
      .then((r) => (r.ok ? r.json() : null))
      .then((data) => {
        if (!data) return;
        if (approval.entityType === "MEMBERSHIP" && approval.action === "approve_membership") {
          setMemberData(data);
          return;
        }
        setEntityData(data);
        if (approval.entityType === "MEMBERSHIP") {
          const memberId =
            (data.member as Record<string, string> | null)?.id ??
            (data.memberId as string | null);
          if (memberId) {
            setMemberLoading(true);
            apiFetch(`/api/members/${memberId}`)
              .then((r) => (r.ok ? r.json() : null))
              .then((mData) => { if (mData) setMemberData(mData); })
              .catch(() => {})
              .finally(() => setMemberLoading(false));
          }
        }
      })
      .catch(() => {})
      .finally(() => setEntityLoading(false));

    if (approval.entityType === "TRANSACTION") {
      setReceiptLoading(true);
      apiFetch(`/api/receipts/${approval.entityId}`)
        .then(async (r) => {
          const data = await r.json().catch(() => null);
          if (!r.ok || !data) {
            throw new Error((data as { error?: string } | null)?.error ?? "Failed to load receipt");
          }
          setReceiptData(data as ReceiptData);
        })
        .catch((err: unknown) => {
          setReceiptError(err instanceof Error ? err.message : "Failed to load receipt");
        })
        .finally(() => setReceiptLoading(false));
    }
  }

  // ---------------------------------------------------------------------------
  // Actions
  // ---------------------------------------------------------------------------

  async function handleAction(type: "approve" | "reject") {
    if (!selected) return;
    setActionLoading(type);

    try {
      const res = await apiFetch(`/api/approvals/${selected.id}/${type}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ notes: actionNotes || undefined }),
      });

      const json = await res.json().catch(() => ({}));

      if (!res.ok) {
        toast({
          title: `Failed to ${type}`,
          description: json.error ?? "An unexpected error occurred",
          variant: "destructive",
        });
        return;
      }

      toast({
        title: type === "approve" ? "Approved successfully" : "Rejected successfully",
        description:
          type === "approve"
            ? "The proposed change has been applied."
            : "The proposed change has been discarded.",
      });

      setDetailOpen(false);
      setSelected(null);
      setActionNotes("");
      await setSize(1);
      await revalidateApiPrefixes(
        "/api/approvals",
        "/api/dashboard/stats",
        "/api/members",
        "/api/memberships",
        "/api/my-membership",
        "/api/transactions",
        "/api/sponsors",
        "/api/sponsor-links"
      );
    } finally {
      setActionLoading(null);
    }
  }

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  return (
    <div className="space-y-6 p-4 sm:p-6">
      {/* Header */}
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Approval Queue</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Review and act on pending approval requests from operators.
          </p>
        </div>
        {pendingCount > 0 && (
          <Badge className="text-base px-3 py-1">
            <Clock className="h-4 w-4 mr-1.5 inline-block" />
            {pendingCount} pending
          </Badge>
        )}
      </div>

      {/* Filters */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <Tabs
          value={statusFilter}
          onValueChange={(value) => {
            setStatusFilter(value);
            void setSize(1);
          }}
        >
          <TabsList>
            <TabsTrigger value="PENDING" className="gap-1.5">
              Pending
              {pendingCount > 0 && (
                <span className="ml-1 rounded-full bg-primary text-primary-foreground text-xs px-1.5 py-0.5 leading-none">
                  {pendingCount}
                </span>
              )}
            </TabsTrigger>
            <TabsTrigger value="APPROVED">Approved</TabsTrigger>
            <TabsTrigger value="REJECTED">Rejected</TabsTrigger>
            <TabsTrigger value="ALL">All</TabsTrigger>
          </TabsList>
        </Tabs>

        <div className="flex items-center gap-3">
          <Select
            value={approvalTypeFilter}
            onValueChange={(value) => {
              setApprovalTypeFilter(value);
              void setSize(1);
            }}
          >
            <SelectTrigger className="w-44">
              <SelectValue placeholder="Category" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="ALL">All Categories</SelectItem>
              {Object.entries(APPROVAL_TYPE_LABELS).map(([value, label]) => (
                <SelectItem key={value} value={value}>
                  {label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <span className="text-sm text-muted-foreground whitespace-nowrap">
            {total} result{total !== 1 ? "s" : ""}
          </span>
        </div>
      </div>

      {/* Table */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base font-semibold">
            {statusFilter === "PENDING" ? "Pending Approvals" : statusFilter === "APPROVED" ? "Approved" : statusFilter === "REJECTED" ? "Rejected" : "All Approvals"}
          </CardTitle>
        </CardHeader>
        <CardContent className="overflow-x-auto p-0">
          {loading ? (
            <div className="p-8 text-center text-muted-foreground text-sm">
              Loading approvals...
            </div>
          ) : error ? (
            <div className="p-8 text-center text-destructive text-sm">
              {error.message}
            </div>
          ) : approvals.length === 0 ? (
            <div className="p-8 text-center text-muted-foreground text-sm">
              No approvals found for the selected filters.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Type</TableHead>
                  <TableHead>Category</TableHead>
                  <TableHead>Requested By</TableHead>
                  <TableHead>Date</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {approvals.map((approval) => (
                  <TableRow
                    key={approval.id}
                    className="cursor-pointer hover:bg-muted/50"
                    onClick={() => openDetail(approval)}
                  >
                    <TableCell>
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                          STATUS_CLASSES[approval.status] ?? "bg-slate-100 text-slate-800"
                        }`}
                      >
                        {approval.approvalTypeLabel ?? APPROVAL_TYPE_LABELS[approval.approvalType] ?? ENTITY_TYPE_LABELS[approval.entityType] ?? approval.entityType}
                      </span>
                    </TableCell>
                    <TableCell className="text-sm">
                      {approval.entityType === "TRANSACTION" ? (
                        <span
                          className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                            directionBadgeClass(approval.direction)
                          }`}
                        >
                          {approval.direction ? DIRECTION_LABELS[approval.direction] ?? approval.direction : "—"}
                        </span>
                      ) : (
                        <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${actionBadgeClass(approval.action)}`}>
                          {ACTION_LABELS[approval.action] ?? approval.action}
                        </span>
                      )}
                    </TableCell>
                    <TableCell className="text-sm">
                      <div>{approval.requestedBy.name}</div>
                      <div className="text-xs text-muted-foreground">
                        {approval.requestedBy.role}
                      </div>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {formatDate(approval.createdAt)}
                    </TableCell>
                    <TableCell>
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                          STATUS_CLASSES[approval.status] ?? "bg-slate-100 text-slate-800"
                        }`}
                      >
                        {approval.status}
                      </span>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          openDetail(approval);
                        }}
                      >
                        <Eye className="h-4 w-4" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Load more */}
      {approvals.length < total && size < totalPages && (
        <div className="flex justify-center">
          <Button
            variant="outline"
            disabled={loading || loadingMore}
            onClick={() => {
              void setSize(size + 1);
            }}
          >
            {loadingMore ? "Loading…" : `Show more (${total - approvals.length} remaining)`}
          </Button>
        </div>
      )}

      {/* Detail Modal */}
      <Dialog open={detailOpen} onOpenChange={setDetailOpen}>
        <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
          {selected && (
            <>
                <DialogHeader>
                  <DialogTitle className="flex items-center gap-2">
                    <span
                      className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                        STATUS_CLASSES[selected.status] ?? "bg-slate-100 text-slate-800"
                      }`}
                    >
                    {selected.approvalTypeLabel ?? APPROVAL_TYPE_LABELS[selected.approvalType] ?? ENTITY_TYPE_LABELS[selected.entityType] ?? selected.entityType}
                    </span>
                    {selected.direction && (
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${
                          directionBadgeClass(selected.direction)
                        }`}
                      >
                        {DIRECTION_LABELS[selected.direction] ?? selected.direction}
                      </span>
                    )}
                </DialogTitle>
                <DialogDescription>
                  Submitted by{" "}
                  <strong>{selected.requestedBy.name}</strong> (
                  {selected.requestedBy.role}) on{" "}
                  {formatDate(selected.createdAt)}
                  {selected.action && ` • ${ACTION_LABELS[selected.action] ?? selected.action}`}
                </DialogDescription>
              </DialogHeader>

              <div className="mt-2">
            <ApprovalDetail
              approval={selected}
              liveEntity={entityData}
              entityLoading={entityLoading}
              memberData={memberData}
              memberLoading={memberLoading}
              receiptData={receiptData}
              receiptLoading={receiptLoading}
              receiptError={receiptError}
              parentMemberData={parentMemberData}
              parentMemberLoading={parentMemberLoading}
            />
              </div>

              {selected.status === "PENDING" && (
                <>
                  <div className="mt-4">
                    <label
                      htmlFor="approval-notes"
                      className="text-sm font-medium text-muted-foreground block mb-1"
                    >
                      Notes (optional)
                    </label>
                    <Input
                      id="approval-notes"
                      placeholder="Add a note for the operator..."
                      value={actionNotes}
                      onChange={(e) => setActionNotes(e.target.value)}
                      maxLength={1000}
                    />
                  </div>

                  <DialogFooter className="mt-4 flex gap-2 sm:flex-row flex-col">
                    <Button
                      variant="outline"
                      className="border-rose-300 text-rose-600 hover:bg-rose-50"
                      disabled={actionLoading !== null}
                      onClick={() => handleAction("reject")}
                    >
                      {actionLoading === "reject" ? (
                        "Rejecting..."
                      ) : (
                        <>
                          <XCircle className="h-4 w-4 mr-1.5" />
                          Reject
                        </>
                      )}
                    </Button>
                    <Button
                      className="bg-emerald-600 text-white hover:bg-emerald-700"
                      disabled={actionLoading !== null}
                      onClick={() => handleAction("approve")}
                    >
                      {actionLoading === "approve" ? (
                        "Approving..."
                      ) : (
                        <>
                          <CheckCircle className="h-4 w-4 mr-1.5" />
                          Approve
                        </>
                      )}
                    </Button>
                  </DialogFooter>
                </>
              )}

              {selected.status !== "PENDING" && (
                <div className="mt-4 p-3 rounded-md bg-muted text-sm">
                  <div className="font-medium mb-1">
                    {selected.status === "APPROVED" ? (
                      <span className="flex items-center gap-1 text-emerald-700">
                        <CheckCircle className="h-4 w-4" /> Approved
                      </span>
                    ) : (
                      <span className="flex items-center gap-1 text-rose-600">
                        <XCircle className="h-4 w-4" /> Rejected
                      </span>
                    )}
                  </div>
                  {selected.reviewedBy && (
                    <div className="text-muted-foreground">
                      By {selected.reviewedBy.name} on{" "}
                      {selected.reviewedAt ? formatDate(selected.reviewedAt) : "—"}
                    </div>
                  )}
                  {selected.notes && (
                    <div className="mt-1 italic text-muted-foreground">
                      &ldquo;{selected.notes}&rdquo;
                    </div>
                  )}
                </div>
              )}
            </>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
