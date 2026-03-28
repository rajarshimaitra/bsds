"use client";

/**
 * My Membership Page
 *
 * Displays:
 * - Two-tier fee status: Annual Membership Fee + Subscription Fee (side-by-side)
 * - Member info row (Member ID, Total Paid, Application Fee status)
 * - User details card (name, email, phone, address)
 * - Sub-members list (linked sub-members and their details)
 * - Payment history (table of membership Transactions — Type badges, Receipt column)
 * - Renew/Pay dialog with checkbox-based fee selection (single API call)
 *   - ACTIVE: expiry countdown + "Renew" button
 *   - EXPIRED / PENDING_PAYMENT: "Pay Now" button
 *   - PENDING_APPROVAL: "Awaiting Approval" message
 * - Sub-member pay-on-behalf: if logged-in user is a sub-member, show "Pay for [Parent Name]"
 * - Application fee indicator if not yet paid
 *
 * Cash payment creates a single Transaction with boolean fee flags.
 */

import { useState, useEffect } from "react";
import { useAuth } from "@/hooks/use-auth";
import { CompleteProfileModal } from "@/components/onboarding/CompleteProfileModal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
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
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { useToast } from "@/components/ui/use-toast";
import { ReceiptView } from "@/components/receipts/ReceiptView";
import { apiFetch } from "@/lib/api-client";
import { revalidateApiPrefixes, useApi } from "@/lib/hooks/use-api";
import type { ReceiptData } from "@/lib/receipt-utils";
import { MEMBERSHIP_FEES, APPLICATION_FEE, ANNUAL_MEMBERSHIP_FEE } from "@/types";
import type { MembershipType } from "@/types";
import {
  formatCurrency as formatCurrencyUtil,
  formatDate as formatDateUtil,
  formatMembershipType,
} from "@/lib/utils";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface SubMemberInfo {
  id: string;
  memberId: string;
  name: string;
  email: string;
  phone: string;
  relation: string;
}

interface TransactionRecord {
  id: string;
  amount: string;
  approvalStatus: string;
  includesSubscription: boolean;
  includesAnnualFee: boolean;
  includesApplicationFee: boolean;
  createdAt: string;
  receipt: { receiptNumber: string } | null;
}

interface MyMembershipData {
  notRegistered?: boolean;
  user: {
    id: string;
    memberId: string;
    name: string;
    email: string;
    phone: string;
    address: string;
    role: string;
    membershipStatus: string;
    membershipType: MembershipType | null;
    membershipStart: string | null;
    membershipExpiry: string | null;
    annualFeeStart: string | null;
    annualFeeExpiry: string | null;
    annualFeePaid: boolean;
    totalPaid: string;
    applicationFeePaid: boolean;
  };
  member: { id: string } | null;
  subMembers: SubMemberInfo[];
  transactionHistory: TransactionRecord[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const formatDate = formatDateUtil;
const formatCurrency = formatCurrencyUtil;

function daysUntil(dateStr: string | null): number | null {
  if (!dateStr) return null;
  const expiry = new Date(dateStr);
  expiry.setHours(0, 0, 0, 0);
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const diff = Math.ceil(
    (expiry.getTime() - today.getTime()) / (1000 * 60 * 60 * 24)
  );
  return diff;
}

function membershipStatusLabel(status: string): string {
  switch (status) {
    case "ACTIVE":
      return "Active";
    case "EXPIRED":
      return "Expired";
    case "PENDING_APPROVAL":
      return "Pending Approval";
    case "PENDING_PAYMENT":
      return "Pending Payment";
    case "SUSPENDED":
      return "Suspended";
    default:
      return status;
  }
}

function membershipStatusVariant(
  status: string
): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case "ACTIVE":
      return "default";
    case "EXPIRED":
    case "SUSPENDED":
      return "destructive";
    case "PENDING_APPROVAL":
    case "PENDING_PAYMENT":
      return "secondary";
    default:
      return "outline";
  }
}

function approvalStatusVariant(
  status: string
): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case "APPROVED":
      return "default";
    case "REJECTED":
      return "destructive";
    case "PENDING":
      return "secondary";
    default:
      return "outline";
  }
}

const membershipTypeLabel = (type: MembershipType | null): string => formatMembershipType(type);

// Fee type badges for a transaction record
function FeeBadges({ record }: { record: TransactionRecord }) {
  return (
    <div className="flex flex-col gap-1">
      {record.includesSubscription && (
        <Badge variant="outline" className="w-fit text-xs">Subscription</Badge>
      )}
      {record.includesAnnualFee && (
        <Badge variant="outline" className="w-fit text-xs">Annual</Badge>
      )}
      {record.includesApplicationFee && (
        <Badge variant="outline" className="w-fit text-xs">Application</Badge>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Page Component
// ---------------------------------------------------------------------------

export default function MyMembershipPage() {
  const { user: authUser, loading: authLoading } = useAuth();
  const { toast } = useToast();

  const [profileModalOpen, setProfileModalOpen] = useState(false);
  const [payDialogOpen, setPayDialogOpen] = useState(false);
  const [selectedType, setSelectedType] = useState<MembershipType>("MONTHLY");
  const [selectedMode, setSelectedMode] = useState<"UPI" | "BANK_TRANSFER" | "CASH">("CASH");
  const [includeAppFee, setIncludeAppFee] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [isRenewal, setIsRenewal] = useState(false);
  const [payAnnualFee, setPayAnnualFee] = useState(false);
  const [paySubscription, setPaySubscription] = useState(true);

  // Receipt dialog state
  const [showReceiptDialog, setShowReceiptDialog] = useState(false);
  const [receiptData, setReceiptData] = useState<ReceiptData | null>(null);
  const [receiptLoading, setReceiptLoading] = useState(false);
  const [receiptError, setReceiptError] = useState<string | null>(null);

  const isSubMember = (authUser as Record<string, unknown> | null)?.isSubMember === true;

  // Auto-open the profile modal when the user has no member record yet
  const {
    data,
    error,
    isLoading,
    mutate,
  } = useApi<MyMembershipData>(
    authUser ? "/api/my-membership" : null,
    {
      dedupingInterval: 30_000,
      revalidateOnFocus: true,
    }
  );

  useEffect(() => {
    if (data?.notRegistered) {
      setProfileModalOpen(true);
    }
  }, [data?.notRegistered]);

  // ---------------------------------------------------------------------------
  // Receipt dialog
  // ---------------------------------------------------------------------------

  async function openReceiptDialog(txId: string) {
    setReceiptData(null);
    setReceiptError(null);
    setReceiptLoading(true);
    setShowReceiptDialog(true);
    try {
      const res = await apiFetch(`/api/receipts/${txId}`);
      const json = await res.json();
      if (!res.ok) throw new Error(json.error ?? "Failed to generate receipt");
      setReceiptData(json as ReceiptData);
    } catch (err: unknown) {
      setReceiptError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      setReceiptLoading(false);
    }
  }

  // ---------------------------------------------------------------------------
  // Compute payment amount
  // ---------------------------------------------------------------------------

  const computeAmount = (): number => {
    let total = 0;
    if (payAnnualFee) total += ANNUAL_MEMBERSHIP_FEE;
    if (paySubscription) total += MEMBERSHIP_FEES[selectedType];
    if (includeAppFee) total += APPLICATION_FEE;
    return total;
  };

  // ---------------------------------------------------------------------------
  // Submit payment — single POST /api/memberships with fee flags
  // ---------------------------------------------------------------------------

  const handlePay = async () => {
    if (!data?.member?.id) {
      toast({
        title: "No member record found",
        description: "Your account has not been linked to a member record yet.",
        variant: "destructive",
      });
      return;
    }

    if (!payAnnualFee && !paySubscription) {
      toast({
        title: "No fees selected",
        description: "Please select at least one fee type to pay.",
        variant: "destructive",
      });
      return;
    }

    setSubmitting(true);
    try {
      const amount = computeAmount();
      const res = await apiFetch("/api/memberships", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          memberId: data.member.id,
          type: paySubscription ? selectedType : "ANNUAL",
          amount,
          paymentMode: selectedMode,
          includesSubscription: paySubscription,
          includesAnnualFee: payAnnualFee,
          includesApplicationFee: includeAppFee && needsAppFee,
          isApplicationFee: includeAppFee && needsAppFee,
        }),
      });
      const json = await res.json();
      if (!res.ok) {
        toast({
          title: "Payment failed",
          description: json.error ?? "Unknown error",
          variant: "destructive",
        });
        return;
      }

      toast({
        title: "Payment submitted for approval",
        description:
          "Your payment has been recorded and is awaiting admin approval.",
      });
      setPayDialogOpen(false);
      await Promise.all([mutate(), revalidateApiPrefixes("/api/dashboard/stats")]);
    } catch {
      toast({ title: "Network error", variant: "destructive" });
    } finally {
      setSubmitting(false);
    }
  };

  // ---------------------------------------------------------------------------
  // Derived state
  // ---------------------------------------------------------------------------

  const membershipStatus = data?.user.membershipStatus ?? "PENDING_PAYMENT";
  const isActive = membershipStatus === "ACTIVE";
  const isPendingApproval =
    membershipStatus === "PENDING_APPROVAL" ||
    (data?.transactionHistory ?? []).some((t) => t.approvalStatus === "PENDING");
  const daysLeft = daysUntil(data?.user.membershipExpiry ?? null);
  const needsAppFee = data ? !data.user.applicationFeePaid : false;

  // Annual fee derived state
  const annualFeeExpiry = data?.user.annualFeeExpiry ?? null;
  const annualFeePaid = data?.user.annualFeePaid ?? false;
  const annualFeeDaysLeft = daysUntil(annualFeeExpiry);
  const annualFeeActive = annualFeePaid && annualFeeDaysLeft !== null && annualFeeDaysLeft >= 0;
  const annualFeeExpired = annualFeeExpiry ? !annualFeeActive : false;

  // Whether each fee needs payment (for button text and dialog defaults)
  const annualFeeNeedsPayment = !annualFeePaid || annualFeeExpired;
  const subscriptionNeedsPayment =
    membershipStatus === "EXPIRED" || membershipStatus === "PENDING_PAYMENT";

  // ---------------------------------------------------------------------------
  // Dialog open helpers — pre-check the relevant boxes based on context
  // ---------------------------------------------------------------------------

  const openPayDialog = (renewal: boolean) => {
    setIsRenewal(renewal);
    setIncludeAppFee(renewal ? false : needsAppFee);

    if (renewal) {
      setPaySubscription(true);
      setPayAnnualFee(annualFeeNeedsPayment);
    } else {
      setPayAnnualFee(annualFeeNeedsPayment);
      setPaySubscription(subscriptionNeedsPayment);
      if (!annualFeeNeedsPayment && !subscriptionNeedsPayment) {
        setPaySubscription(true);
      }
    }

    setPayDialogOpen(true);
  };

  // Button text based on what needs payment
  const actionButtonText = (): string => {
    if (isActive) return "Renew Membership";
    if (annualFeeNeedsPayment && subscriptionNeedsPayment) return "Pay Membership Fees";
    if (annualFeeNeedsPayment) return "Pay Annual Fee";
    if (subscriptionNeedsPayment) return "Pay Subscription";
    return "Pay Now";
  };

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  if (authLoading || (isLoading && !data)) {
    return (
      <div className="p-8 text-muted-foreground">Loading membership data...</div>
    );
  }

  if (error && !data) {
    return (
      <div className="space-y-4 p-8">
        <p className="text-sm text-destructive">
          Failed to load membership data: {error.message}
        </p>
        <Button variant="outline" size="sm" onClick={() => void mutate()}>
          Retry
        </Button>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="p-8 text-muted-foreground">
        No membership data available. Please contact the administrator.
      </div>
    );
  }

  return (
    <div className="space-y-6 p-4 sm:p-6">
      <h1 className="text-2xl font-bold tracking-tight text-slate-900">My Membership</h1>

      {/* ------------------------------------------------------------------ */}
      {/* Profile completion modal                                             */}
      {/* ------------------------------------------------------------------ */}
      <CompleteProfileModal
        open={profileModalOpen}
        prefillName={authUser?.username}
        onSuccess={() => {
          setProfileModalOpen(false);
          void mutate();
        }}
        onSkip={() => setProfileModalOpen(false)}
      />

      {/* Not registered — modal trigger banner */}
      {data.notRegistered && (
        <Card className="border-amber-300 bg-amber-50">
          <CardContent className="flex flex-col items-start justify-between gap-3 p-5 sm:flex-row sm:items-center">
            <div>
              <p className="text-base font-semibold text-amber-800">
                Your membership profile is not set up yet.
              </p>
              <p className="mt-0.5 text-sm text-amber-700">
                Fill in your details to activate your membership record.
              </p>
            </div>
            <Button
              size="sm"
              className="shrink-0 bg-amber-600 hover:bg-amber-700 text-white"
              onClick={() => setProfileModalOpen(true)}
            >
              Complete profile
            </Button>
          </CardContent>
        </Card>
      )}

      {/* ------------------------------------------------------------------ */}
      {/* Sub-member pay-on-behalf banner                                      */}
      {/* ------------------------------------------------------------------ */}
      {isSubMember && (
        <Card className="border-amber-200 bg-amber-50/90">
          <CardContent className="flex flex-col items-start justify-between gap-3 p-4 sm:flex-row sm:items-center">
            <p className="text-sm text-amber-800">
              You are viewing the membership for{" "}
              <strong>{data.user.name}</strong>. As a sub-member, you can pay
              on their behalf.
            </p>
            <Button
              size="sm"
              variant="outline"
              className="border-amber-400 text-amber-800 hover:bg-amber-100"
              onClick={() => openPayDialog(isActive)}
            >
              Pay for {data.user.name}
            </Button>
          </CardContent>
        </Card>
      )}

      {/* ------------------------------------------------------------------ */}
      {/* Application fee alert                                               */}
      {/* ------------------------------------------------------------------ */}
      {needsAppFee && (
        <Card className="border-sky-200 bg-sky-50/90">
          <CardContent className="p-4">
            <p className="text-sm text-sky-800">
              <strong>Application fee pending:</strong> A one-time application
              fee of {formatCurrency(APPLICATION_FEE)} is required. It will be
              included with your first subscription payment.
            </p>
          </CardContent>
        </Card>
      )}

      {/* ------------------------------------------------------------------ */}
      {/* Member Info Row                                                      */}
      {/* ------------------------------------------------------------------ */}
      <div className="grid gap-3 sm:grid-cols-3">
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Member ID
          </p>
          <p className="font-mono text-sm font-semibold">{data.user.memberId}</p>
        </div>
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Total Paid
          </p>
          <p className="text-sm font-semibold">
            {formatCurrency(data.user.totalPaid)}
          </p>
        </div>
        <div>
          <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Application Fee
          </p>
          <p className="text-sm">
            {data.user.applicationFeePaid ? (
              <span className="text-emerald-600">Paid</span>
            ) : (
              <span className="text-amber-600">Not Paid</span>
            )}
          </p>
        </div>
      </div>

      {/* ------------------------------------------------------------------ */}
      {/* Two-Tier Status Cards                                                */}
      {/* ------------------------------------------------------------------ */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {/* Card 1 — Annual Membership Fee */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center justify-between">
              <span>Annual Membership Fee</span>
              {annualFeeActive ? (
                <Badge variant="default">Active</Badge>
              ) : annualFeeExpired ? (
                <Badge variant="destructive">Expired</Badge>
              ) : (
                <Badge variant="destructive">Not Paid</Badge>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">Fee</span>
              <span>{formatCurrency(ANNUAL_MEMBERSHIP_FEE)}/year</span>
            </div>
            {data.user.annualFeeStart && (
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Start Date</span>
                <span>{formatDate(data.user.annualFeeStart)}</span>
              </div>
            )}
            {annualFeeExpiry && (
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Expiry Date</span>
                <span>{formatDate(annualFeeExpiry)}</span>
              </div>
            )}
            {annualFeeActive && annualFeeDaysLeft !== null && (
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Days until expiry</span>
                <span>
                  {annualFeeDaysLeft === 0
                    ? "Expires today"
                    : `${annualFeeDaysLeft} day${annualFeeDaysLeft === 1 ? "" : "s"}`}
                </span>
              </div>
            )}
          </CardContent>
        </Card>

        {/* Card 2 — Subscription Fee */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center justify-between">
              <span>Subscription Fee</span>
              <Badge variant={membershipStatusVariant(membershipStatus)}>
                {membershipStatusLabel(membershipStatus)}
              </Badge>
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {data.user.membershipType && (
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Plan</span>
                <span>{membershipTypeLabel(data.user.membershipType)}</span>
              </div>
            )}
            {data.user.membershipType && (
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Fee</span>
                <span>{formatCurrency(MEMBERSHIP_FEES[data.user.membershipType])}</span>
              </div>
            )}
            {data.user.membershipStart && (
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Start Date</span>
                <span>{formatDate(data.user.membershipStart)}</span>
              </div>
            )}
            {data.user.membershipExpiry && (
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Expiry Date</span>
                <span>{formatDate(data.user.membershipExpiry)}</span>
              </div>
            )}
            {isActive && daysLeft !== null && daysLeft >= 0 && (
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">Days until expiry</span>
                <span>
                  {daysLeft === 0
                    ? "Expires today"
                    : `${daysLeft} day${daysLeft === 1 ? "" : "s"}`}
                </span>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* ------------------------------------------------------------------ */}
      {/* Action area                                                          */}
      {/* ------------------------------------------------------------------ */}
      <div className="flex flex-wrap items-center gap-3">
        {isPendingApproval && !isActive && (
          <Badge variant="secondary">Awaiting Approval</Badge>
        )}

        {isActive && (
          <Button onClick={() => openPayDialog(true)}>
            {actionButtonText()}
          </Button>
        )}

        {(membershipStatus === "EXPIRED" ||
          membershipStatus === "PENDING_PAYMENT") && (
          <Button onClick={() => openPayDialog(false)}>
            {actionButtonText()}
          </Button>
        )}

        {membershipStatus === "PENDING_APPROVAL" && (
          <p className="text-sm text-muted-foreground">
            Your membership application is pending admin approval.
          </p>
        )}
      </div>

      {/* ------------------------------------------------------------------ */}
      {/* User Details Card                                                   */}
      {/* ------------------------------------------------------------------ */}
      <Card>
        <CardHeader>
          <CardTitle>Personal Details</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid gap-3 sm:grid-cols-2">
            <div>
              <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Full Name
              </p>
              <p className="text-sm">{data.user.name}</p>
            </div>
            <div>
              <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                Email
              </p>
              <p className="text-sm">{data.user.email}</p>
            </div>
            {data.user.phone && (
              <div>
                <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  WhatsApp / Phone
                </p>
                <p className="text-sm">{data.user.phone}</p>
              </div>
            )}
            {data.user.address && (
              <div className="sm:col-span-2">
                <p className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  Address
                </p>
                <p className="text-sm">{data.user.address}</p>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* ------------------------------------------------------------------ */}
      {/* Sub-members List                                                    */}
      {/* ------------------------------------------------------------------ */}
      {!isSubMember && data.subMembers.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Sub-Members</CardTitle>
          </CardHeader>
          <CardContent className="overflow-x-auto p-0 pt-0">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Member ID</TableHead>
                  <TableHead>Name</TableHead>
                  <TableHead>Relation</TableHead>
                  <TableHead>Email</TableHead>
                  <TableHead>Phone</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.subMembers.map((sm) => (
                  <TableRow key={sm.id}>
                    <TableCell className="font-mono text-xs">{sm.memberId}</TableCell>
                    <TableCell>{sm.name}</TableCell>
                    <TableCell>{sm.relation}</TableCell>
                    <TableCell>{sm.email}</TableCell>
                    <TableCell>{sm.phone}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}

      {/* ------------------------------------------------------------------ */}
      {/* Payment History                                                     */}
      {/* ------------------------------------------------------------------ */}
      <Card>
        <CardHeader>
          <CardTitle>Payment History</CardTitle>
        </CardHeader>
        <CardContent className="overflow-x-auto">
          {data.transactionHistory.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No payment records found.
            </p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Date</TableHead>
                  <TableHead>Type</TableHead>
                  <TableHead>Amount</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Receipt</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.transactionHistory.map((record) => (
                  <TableRow key={record.id}>
                    <TableCell className="text-sm">
                      {formatDate(record.createdAt)}
                    </TableCell>
                    <TableCell>
                      <FeeBadges record={record} />
                    </TableCell>
                    <TableCell className="font-semibold">
                      {formatCurrency(record.amount)}
                    </TableCell>
                    <TableCell>
                      <Badge variant={approvalStatusVariant(record.approvalStatus)}>
                        {record.approvalStatus.charAt(0) +
                          record.approvalStatus.slice(1).toLowerCase()}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      {record.receipt ? (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-7 px-2 text-xs"
                          onClick={() => void openReceiptDialog(record.id)}
                        >
                          View
                        </Button>
                      ) : (
                        <span className="text-xs text-muted-foreground">—</span>
                      )}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* ------------------------------------------------------------------ */}
      {/* Receipt Dialog                                                      */}
      {/* ------------------------------------------------------------------ */}
      <Dialog open={showReceiptDialog} onOpenChange={setShowReceiptDialog}>
        <DialogContent className="max-w-2xl p-0">
          <DialogHeader className="px-6 pt-6">
            <DialogTitle>
              {receiptData
                ? `Receipt ${receiptData.receiptNumber}`
                : "Generating Receipt..."}
            </DialogTitle>
          </DialogHeader>
          <div className="max-h-[calc(100vh-10rem)] overflow-y-auto px-4 py-4 sm:px-6 sm:py-5">
            {receiptLoading && (
              <div className="text-center py-8 text-muted-foreground text-sm">
                Generating receipt…
              </div>
            )}
            {receiptError && !receiptLoading && (
              <div className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
                {receiptError}
              </div>
            )}
            {receiptData && !receiptLoading && (
              <ReceiptView receipt={receiptData} />
            )}
          </div>
        </DialogContent>
      </Dialog>

      {/* ------------------------------------------------------------------ */}
      {/* Pay / Renew Dialog                                                  */}
      {/* ------------------------------------------------------------------ */}
      <Dialog open={payDialogOpen} onOpenChange={setPayDialogOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>
              {isRenewal ? "Renew Membership" : "Pay Membership Fee"}
            </DialogTitle>
            <DialogDescription>
              {isSubMember
                ? `Paying on behalf of ${data.user.name}.`
                : "Select the fees you want to pay."}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-2">
            {/* Fee selection checkboxes */}
            <div className="space-y-3">
              <Label>Select Fees to Pay</Label>

              {/* Annual Fee checkbox */}
              <div className="flex items-center space-x-3 rounded-md border p-3">
                <Checkbox
                  id="pay-annual"
                  checked={payAnnualFee}
                  onCheckedChange={(checked) => setPayAnnualFee(checked === true)}
                />
                <label htmlFor="pay-annual" className="flex-1 cursor-pointer">
                  <p className="text-sm font-medium">Annual Membership Fee</p>
                  <p className="text-xs text-muted-foreground">
                    {formatCurrency(ANNUAL_MEMBERSHIP_FEE)} — valid for 1 year
                  </p>
                </label>
              </div>

              {/* Subscription checkbox */}
              <div className="rounded-md border">
                <div className="flex items-center space-x-3 p-3">
                  <Checkbox
                    id="pay-subscription"
                    checked={paySubscription}
                    onCheckedChange={(checked) => setPaySubscription(checked === true)}
                  />
                  <label htmlFor="pay-subscription" className="flex-1 cursor-pointer">
                    <p className="text-sm font-medium">Subscription Fee</p>
                    <p className="text-xs text-muted-foreground">Recurring plan-based fee</p>
                  </label>
                </div>
                {paySubscription && (
                  <div className="px-3 pb-3">
                    <Select
                      value={selectedType}
                      onValueChange={(v) => setSelectedType(v as MembershipType)}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="Select type" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="MONTHLY">
                          Monthly — {formatCurrency(MEMBERSHIP_FEES.MONTHLY)}
                        </SelectItem>
                        <SelectItem value="HALF_YEARLY">
                          Half-Yearly — {formatCurrency(MEMBERSHIP_FEES.HALF_YEARLY)}
                        </SelectItem>
                        <SelectItem value="ANNUAL">
                          Annual — {formatCurrency(MEMBERSHIP_FEES.ANNUAL)}
                        </SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                )}
              </div>

              {/* Application fee checkbox — only shown if unpaid */}
              {needsAppFee && (
                <div className="flex items-center space-x-3 rounded-md border p-3">
                  <Checkbox
                    id="pay-app-fee"
                    checked={includeAppFee}
                    onCheckedChange={(checked) => setIncludeAppFee(checked === true)}
                  />
                  <label htmlFor="pay-app-fee" className="flex-1 cursor-pointer">
                    <p className="text-sm font-medium">Application Fee</p>
                    <p className="text-xs text-muted-foreground">
                      {formatCurrency(APPLICATION_FEE)} — one-time, first membership only
                    </p>
                  </label>
                </div>
              )}
            </div>

            {/* Payment mode */}
            <div className="space-y-1">
              <Label>Payment Mode</Label>
              <Select
                value={selectedMode}
                onValueChange={(v) =>
                  setSelectedMode(v as "UPI" | "BANK_TRANSFER" | "CASH")
                }
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select payment mode" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="CASH">Cash (in-person)</SelectItem>
                  <SelectItem value="UPI" disabled>
                    UPI (coming soon — Razorpay)
                  </SelectItem>
                  <SelectItem value="BANK_TRANSFER" disabled>
                    Bank Transfer (coming soon — Razorpay)
                  </SelectItem>
                </SelectContent>
              </Select>
              {selectedMode === "CASH" && (
                <p className="text-xs text-muted-foreground">
                  Cash payments are recorded by the operator and require admin
                  approval before your membership is activated.
                </p>
              )}
            </div>

            {/* Amount summary with line items */}
            <div className="rounded-md bg-muted p-3 space-y-2">
              {payAnnualFee && (
                <div className="flex justify-between text-sm">
                  <span>Annual Membership Fee</span>
                  <span>{formatCurrency(ANNUAL_MEMBERSHIP_FEE)}</span>
                </div>
              )}
              {paySubscription && (
                <div className="flex justify-between text-sm">
                  <span>{membershipTypeLabel(selectedType)} Subscription</span>
                  <span>{formatCurrency(MEMBERSHIP_FEES[selectedType])}</span>
                </div>
              )}
              {includeAppFee && (
                <div className="flex justify-between text-sm">
                  <span>Application Fee (one-time)</span>
                  <span>{formatCurrency(APPLICATION_FEE)}</span>
                </div>
              )}
              <Separator />
              <div className="flex justify-between font-bold">
                <span>Total</span>
                <span>{formatCurrency(computeAmount())}</span>
              </div>
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setPayDialogOpen(false)}
              disabled={submitting}
            >
              Cancel
            </Button>
            <Button
              onClick={handlePay}
              disabled={submitting || (!payAnnualFee && !paySubscription)}
            >
              {submitting
                ? "Processing..."
                : selectedMode === "CASH"
                ? "Submit Cash Payment"
                : "Proceed to Pay"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
