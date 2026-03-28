"use client";

/**
 * Cash Management page — /dashboard/cash
 *
 * Summary cards: Total Income, Total Expenses, Pending Amount, Net Balance
 * Transaction table with filters (type, category, payment mode, date range)
 * Add transaction dialog and stored receipt access
 * Operator sees "Submit for Approval" label; admin sees "Create"
 * Razorpay-sourced transactions shown with badge, edit/delete disabled
 */

import { useState, useRef, useEffect } from "react";
import { useAuth } from "@/hooks/use-auth";
import {
  PlusIcon,
  RefreshCwIcon,
  ArrowUpIcon,
  ArrowDownIcon,
  ClockIcon,
  WalletIcon,
  ReceiptIcon,
  ChevronDownIcon,
  XIcon,
  EyeIcon,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { ReceiptView } from "@/components/receipts/ReceiptView";
import { apiFetch } from "@/lib/api-client";
import { revalidateApiPrefixes, useApi } from "@/lib/hooks/use-api";
import type { ReceiptData } from "@/lib/receipt-utils";
import { formatCurrency as formatCurrencyUtil, formatDate, formatSponsorPurpose } from "@/lib/utils";
import { MEMBERSHIP_FEES, APPLICATION_FEE, ANNUAL_MEMBERSHIP_FEE } from "@/types";
import type { MembershipType } from "@/types";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Transaction {
  id: string;
  type: "CASH_IN" | "CASH_OUT";
  category: string;
  amount: string;
  paymentMode: string;
  purpose: string;
  remark: string | null;
  sponsorPurpose: string | null;
  memberId: string | null;
  sponsorId: string | null;
  enteredById: string;
  approvalStatus: string;
  approvalSource: "MANUAL" | "RAZORPAY_WEBHOOK";
  senderName: string | null;
  senderPhone: string | null;
  sponsorSenderName: string | null;
  sponsorSenderContact: string | null;
  createdAt: string;
  member: { id: string; name: string; email: string } | null;
  sponsor: { id: string; name: string; company: string | null } | null;
  enteredBy: { id: string; name: string; email: string; role?: string };
  approvedBy: { id: string; name: string } | null;
  receipt: { receiptNumber: string; status: "ACTIVE" | "CANCELLED" } | null;
}

interface PaginatedTransactions {
  data: Transaction[];
  total: number;
  page: number;
  limit: number;
  totalPages: number;
}

interface Summary {
  totalIncome: number;
  totalExpenses: number;
  pendingAmount: number;
  netBalance: number;
}

interface TransactionFormData {
  type: "CASH_IN" | "CASH_OUT";
  category: string;
  amount: string;
  paymentMode: string;
  purpose: string;
  remark: string;
  sponsorPurpose: string;
  customSponsorPurpose: string;
  expensePurpose: string;
  customExpensePurpose: string;
  memberId: string;
  sponsorId: string;
  senderName: string;
  senderPhone: string;
  sponsorSenderName: string;
  sponsorSenderContact: string;
}

interface EligibleMember {
  id: string;
  name: string;
  email: string;
  phone: string;
  address: string;
  subMembers: { id: string; name: string; relation: string; email: string }[];
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Cash In categories — only membership and sponsorship for manual entries */
const CASH_IN_CATEGORIES = [
  { value: "MEMBERSHIP", label: "Membership" },
  { value: "SPONSORSHIP", label: "Sponsorship" },
];

/** All categories — used for filter dropdowns and display */
const CATEGORIES = [
  { value: "MEMBERSHIP", label: "Membership" },
  { value: "SPONSORSHIP", label: "Sponsorship" },
  { value: "EXPENSE", label: "Expense" },
  { value: "OTHER", label: "Other" },
];

const PAYMENT_MODES = [
  { value: "UPI", label: "UPI" },
  { value: "BANK_TRANSFER", label: "Bank Transfer" },
  { value: "CASH", label: "Cash" },
];

const SPONSOR_PURPOSES = [
  { value: "TITLE_SPONSOR", label: "Title Sponsor" },
  { value: "GOLD_SPONSOR", label: "Gold Sponsor" },
  { value: "SILVER_SPONSOR", label: "Silver Sponsor" },
  { value: "FOOD_PARTNER", label: "Food Partner" },
  { value: "MEDIA_PARTNER", label: "Media Partner" },
  { value: "STALL_VENDOR", label: "Stall Vendor" },
  { value: "MARKETING_PARTNER", label: "Marketing Partner" },
  { value: "OTHER", label: "Other" },
];

const EXPENSE_PURPOSES = [
  { value: "DECORATION_PANDAL", label: "Decoration / Pandal" },
  { value: "IDOL_MURTI", label: "Idol / Murti" },
  { value: "LIGHTING_SOUND", label: "Lighting / Sound" },
  { value: "FOOD_BHOG_PRASAD", label: "Food / Bhog / Prasad" },
  { value: "PRIEST_PUROHIT", label: "Priest / Purohit" },
  { value: "TRANSPORT_LOGISTICS", label: "Transport / Logistics" },
  { value: "PRINTING_PUBLICITY", label: "Printing / Publicity" },
  { value: "CULTURAL_PROGRAM", label: "Cultural Program" },
  { value: "CLEANING_SANITATION", label: "Cleaning / Sanitation" },
  { value: "ELECTRICITY_GENERATOR", label: "Electricity / Generator" },
  { value: "SECURITY", label: "Security" },
  { value: "OTHER", label: "Other" },
];

const APPROVAL_STATUSES = [
  { value: "PENDING", label: "Pending" },
  { value: "APPROVED", label: "Approved" },
  { value: "REJECTED", label: "Rejected" },
];

// All membership fee transactions share the same category; flags distinguish fee types.
function calcMembershipCategory(_feeApp: boolean, _feeAnnual: boolean, _feeSubs: boolean): string {
  return "MEMBERSHIP";
}

function calcMembershipAmount(
  feeApp: boolean,
  feeAnnual: boolean,
  feeSubs: boolean,
  subType: MembershipType
): number {
  let total = 0;
  if (feeApp) total += APPLICATION_FEE;
  if (feeAnnual) total += ANNUAL_MEMBERSHIP_FEE;
  if (feeSubs) total += MEMBERSHIP_FEES[subType];
  return total;
}

// ---------------------------------------------------------------------------
// Currency formatter (alias for imported utility)
// ---------------------------------------------------------------------------

const formatCurrency = formatCurrencyUtil;

// ---------------------------------------------------------------------------
// Category / mode label helpers
// ---------------------------------------------------------------------------

function getCategoryLabel(value: string): string {
  if (value === "MEMBERSHIP") return "Membership";
  return CATEGORIES.find((c) => c.value === value)?.label ?? value;
}

function getPaymentModeLabel(value: string): string {
  return PAYMENT_MODES.find((m) => m.value === value)?.label ?? value;
}

function getSponsorPurposeLabel(value: string | null): string {
  if (!value) return "—";
  return SPONSOR_PURPOSES.find((s) => s.value === value)?.label ?? value;
}

// ---------------------------------------------------------------------------
// Default form state
// ---------------------------------------------------------------------------

const emptyForm: TransactionFormData = {
  type: "CASH_IN",
  category: "",
  amount: "",
  paymentMode: "",
  purpose: "",
  remark: "",
  sponsorPurpose: "",
  customSponsorPurpose: "",
  expensePurpose: "",
  customExpensePurpose: "",
  memberId: "",
  sponsorId: "",
  senderName: "",
  senderPhone: "",
  sponsorSenderName: "",
  sponsorSenderContact: "",
};

/** Map expense purpose enum to human-readable label for storage */
function getExpensePurposeLabel(value: string): string {
  return EXPENSE_PURPOSES.find((e) => e.value === value)?.label ?? value;
}

/** Map sponsor purpose enum to human-readable label */
function getSponsorPurposeLabelForStorage(value: string): string {
  return SPONSOR_PURPOSES.find((s) => s.value === value)?.label ?? value;
}

const emptySummary: Summary = {
  totalIncome: 0,
  totalExpenses: 0,
  pendingAmount: 0,
  netBalance: 0,
};

// ---------------------------------------------------------------------------
// Page component
// ---------------------------------------------------------------------------

export default function CashPage() {
  const { user, loading: authLoading } = useAuth();
  const isAdmin = user?.role === "ADMIN";
  const isOperator = user?.role === "OPERATOR";
  const canWrite = isAdmin || isOperator;

  // Data state
  const [page, setPage] = useState(1);

  // Loading / error state
  const [actionError, setActionError] = useState<string | null>(null);

  // Filter state
  const [filterType, setFilterType] = useState("ALL");
  const [filterCategory, setFilterCategory] = useState("ALL");
  const [filterPaymentMode, setFilterPaymentMode] = useState("ALL");
  const [filterStatus, setFilterStatus] = useState("ALL");
  const [filterDateFrom, setFilterDateFrom] = useState("");
  const [filterDateTo, setFilterDateTo] = useState("");

  // Dialog state
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [showDetailDialog, setShowDetailDialog] = useState(false);
  const [detailTransaction, setDetailTransaction] = useState<Transaction | null>(null);
  const [detailMemberData, setDetailMemberData] = useState<Record<string, unknown> | null>(null);
  const [detailMemberLoading, setDetailMemberLoading] = useState(false);
  const [showReceiptDialog, setShowReceiptDialog] = useState(false);
  const [receiptData, setReceiptData] = useState<ReceiptData | null>(null);
  const [receiptLoading, setReceiptLoading] = useState(false);
  const [receiptError, setReceiptError] = useState<string | null>(null);
  const [selectedTransaction, setSelectedTransaction] =
    useState<Transaction | null>(null);
  const [formData, setFormData] = useState<TransactionFormData>(emptyForm);
  const [submitting, setSubmitting] = useState(false);

  // Member picker state (shown when category = Membership)
  const [membershipSectionActive, setMembershipSectionActive] = useState(false);
  const [eligibleMembers, setEligibleMembers] = useState<EligibleMember[]>([]);
  const [eligibleMembersLoading, setEligibleMembersLoading] = useState(false);
  const [memberSearch, setMemberSearch] = useState("");
  const [memberPickerOpen, setMemberPickerOpen] = useState(false);
  const [selectedFeeMember, setSelectedFeeMember] = useState<EligibleMember | null>(null);
  // Fee checkboxes
  const [feeAnnual, setFeeAnnual] = useState(false);
  const [feeSubs, setFeeSubs] = useState(true);
  const [feeApp, setFeeApp] = useState(false);
  const [subscriptionType, setSubscriptionType] = useState<MembershipType>("MONTHLY");
  const memberPickerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (memberPickerRef.current && !memberPickerRef.current.contains(e.target as Node)) {
        setMemberPickerOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);
  const params = new URLSearchParams({ page: String(page), limit: "20" });
  if (filterType !== "ALL") params.set("type", filterType);
  if (filterCategory !== "ALL") params.set("category", filterCategory);
  if (filterPaymentMode !== "ALL") params.set("paymentMode", filterPaymentMode);
  if (filterStatus !== "ALL") params.set("status", filterStatus);
  if (filterDateFrom) params.set("dateFrom", filterDateFrom);
  if (filterDateTo) params.set("dateTo", filterDateTo);

  const {
    data: transactionsResponse,
    error,
    isLoading: transactionsLoadingRaw,
    mutate: mutateTransactions,
  } = useApi<PaginatedTransactions>(
    user ? `/api/transactions?${params.toString()}` : null,
    {
      dedupingInterval: 10_000,
      revalidateOnFocus: true,
      keepPreviousData: true,
    }
  );
  const {
    data: summaryData,
    isLoading: summaryLoadingRaw,
    mutate: mutateSummary,
  } = useApi<Summary>(
    user ? "/api/transactions/summary" : null,
    {
      dedupingInterval: 10_000,
      revalidateOnFocus: true,
    }
  );

  const transactions = transactionsResponse?.data ?? [];
  const total = transactionsResponse?.total ?? 0;
  const totalPages = transactionsResponse?.totalPages ?? 1;
  const summary = summaryData ?? emptySummary;
  const loading = authLoading || (transactionsLoadingRaw && !transactionsResponse);
  const summaryLoading = authLoading || (summaryLoadingRaw && !summaryData);

  // ---------------------------------------------------------------------------
  // Form helpers
  // ---------------------------------------------------------------------------

  function openAddDialog() {
    setFormData(emptyForm);
    setActionError(null);
    resetMemberPicker();
    setShowAddDialog(true);
  }

  function openDetailDialog(t: Transaction) {
    setDetailTransaction(t);
    setDetailMemberData(null);
    setShowDetailDialog(true);

    if (t.category === "MEMBERSHIP" && t.memberId) {
      setDetailMemberLoading(true);
      apiFetch(`/api/members/${t.memberId}`)
        .then((r) => (r.ok ? r.json() : null))
        .then((d) => { if (d) setDetailMemberData(d as Record<string, unknown>); })
        .catch(() => {})
        .finally(() => setDetailMemberLoading(false));
    }
  }

  async function openReceiptDialog(t: Transaction) {
    setSelectedTransaction(t);
    setReceiptData(null);
    setReceiptError(null);
    setReceiptLoading(true);
    setShowReceiptDialog(true);
    try {
      const res = await apiFetch(`/api/receipts/${t.id}`);
      const data = await res.json();
      if (!res.ok) throw new Error(data.error ?? "Failed to generate receipt");
      setReceiptData(data as ReceiptData);
    } catch (err: unknown) {
      setReceiptError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      setReceiptLoading(false);
    }
  }

  function updateForm(field: keyof TransactionFormData, value: string) {
    setFormData((prev) => ({ ...prev, [field]: value }));
  }

  async function loadAllMembers() {
    setEligibleMembersLoading(true);
    try {
      const res = await apiFetch("/api/members?limit=100");
      const d = await res.json();
      setEligibleMembers(d.data ?? []);
    } finally {
      setEligibleMembersLoading(false);
    }
  }

  function resetMemberPicker() {
    setMembershipSectionActive(false);
    setMemberSearch("");
    setMemberPickerOpen(false);
    setSelectedFeeMember(null);
    setFeeAnnual(false);
    setFeeSubs(true);
    setFeeApp(false);
    setSubscriptionType("MONTHLY");
    setEligibleMembers([]);
    setFormData((prev) => ({ ...prev, memberId: "", category: "", amount: "", sponsorPurpose: "", customSponsorPurpose: "" }));
  }

  function handleCategoryChange(value: string) {
    if (value === "MEMBERSHIP") {
      setMembershipSectionActive(true);
      const cat = calcMembershipCategory(false, false, true);
      const amt = calcMembershipAmount(false, false, true, "MONTHLY");
      setFormData((prev) => ({ ...prev, category: cat, amount: String(amt) }));
      setSelectedFeeMember(null);
      setMemberSearch("");
      setFeeAnnual(false);
      setFeeSubs(true);
      setFeeApp(false);
      setSubscriptionType("MONTHLY");
      void loadAllMembers();
    } else {
      resetMemberPicker();
      setMembershipSectionActive(false);
      setFormData((prev) => ({ ...prev, category: value, memberId: "" }));
    }
  }

  function handleMemberSelect(member: EligibleMember) {
    setSelectedFeeMember(member);
    setMemberSearch(member.name);
    setMemberPickerOpen(false);
    setFormData((prev) => ({ ...prev, memberId: member.id }));
  }

  function recalcFees(
    nextAnnual: boolean,
    nextSubs: boolean,
    nextApp: boolean,
    nextSubType: MembershipType
  ) {
    const category = calcMembershipCategory(nextApp, nextAnnual, nextSubs);
    const amount = calcMembershipAmount(nextApp, nextAnnual, nextSubs, nextSubType);
    setFormData((prev) => ({
      ...prev,
      category,
      amount: amount > 0 ? String(amount) : "",
    }));
  }

  function handleFeeAnnualChange(checked: boolean) {
    setFeeAnnual(checked);
    recalcFees(checked, feeSubs, feeApp, subscriptionType);
  }

  function handleFeeSubsChange(checked: boolean) {
    setFeeSubs(checked);
    recalcFees(feeAnnual, checked, feeApp, subscriptionType);
  }

  function handleFeeAppChange(checked: boolean) {
    setFeeApp(checked);
    recalcFees(feeAnnual, feeSubs, checked, subscriptionType);
  }

  function handleSubscriptionTypeChange(type: MembershipType) {
    setSubscriptionType(type);
    recalcFees(feeAnnual, feeSubs, feeApp, type);
  }

  function validateTransactionForm(): string[] {
    const errs: string[] = [];
    if (formData.type === "CASH_IN" && !formData.category) errs.push("Category is required");
    const amt = parseFloat(formData.amount);
    if (!formData.amount || isNaN(amt) || amt <= 0) errs.push("Amount must be a positive number");
    if (formData.type === "CASH_OUT") {
      if (!formData.expensePurpose) errs.push("Purpose is required for Cash Out");
      if (formData.expensePurpose === "OTHER" && !formData.customExpensePurpose.trim())
        errs.push("Please specify the expense purpose");
    }
    if (formData.category === "SPONSORSHIP" && !formData.sponsorPurpose)
      errs.push("Sponsor purpose is required for Sponsorship transactions");
    if (formData.sponsorPurpose === "OTHER" && !formData.customSponsorPurpose.trim())
      errs.push("Please specify the sponsor purpose");
    if (membershipSectionActive && !formData.memberId)
      errs.push("A member must be selected for membership fee transactions");
    if (membershipSectionActive && !feeAnnual && !feeSubs && !feeApp)
      errs.push("Select at least one fee type (Annual, Subscription, or Application Fee)");
    if (!formData.paymentMode) errs.push("Payment mode is required");
    const categoryChosen = formData.type === "CASH_OUT"
      ? formData.expensePurpose !== ""
      : formData.category !== "";
    if (categoryChosen && !formData.senderName.trim())
      errs.push("Received By is required");
    if (formData.type === "CASH_OUT" && categoryChosen && !formData.senderPhone.trim())
      errs.push("Receiver's Contact is required");
    if (formData.category === "SPONSORSHIP" && !formData.sponsorSenderName.trim())
      errs.push("Sender's Name is required for Sponsorship");
    return errs;
  }

  // ---------------------------------------------------------------------------
  // Submit handlers
  // ---------------------------------------------------------------------------

  async function handleCreate() {
    const validationErrors = validateTransactionForm();
    if (validationErrors.length > 0) {
      setActionError(validationErrors.join(" • "));
      return;
    }
    setSubmitting(true);
    setActionError(null);
    try {
      // Build purpose string from form fields
      let purposeText = "";
      if (formData.type === "CASH_OUT") {
        // Cash Out: derive purpose from expense purpose dropdown
        purposeText = formData.expensePurpose === "OTHER"
          ? formData.customExpensePurpose.trim()
          : getExpensePurposeLabel(formData.expensePurpose);
      } else if (formData.category === "SPONSORSHIP") {
        // Sponsorship: derive from sponsor purpose
        purposeText = formData.sponsorPurpose === "OTHER"
          ? formData.customSponsorPurpose.trim()
          : getSponsorPurposeLabelForStorage(formData.sponsorPurpose);
      } else if (membershipSectionActive) {
        // Membership: auto-generate from fee selection
        const parts: string[] = [];
        if (feeAnnual) parts.push("Annual Membership Fee");
        if (feeSubs) parts.push(`${subscriptionType === "MONTHLY" ? "Monthly" : subscriptionType === "HALF_YEARLY" ? "Half-yearly" : "Annual"} Subscription`);
        if (feeApp) parts.push("Application Fee");
        purposeText = parts.join(", ") || "Membership Fee";
      }

      const payload: Record<string, unknown> = {
        type: formData.type,
        category: formData.type === "CASH_OUT" ? "EXPENSE" : formData.category,
        amount: parseFloat(formData.amount),
        paymentMode: formData.paymentMode,
        purpose: purposeText,
      };
      if (formData.remark.trim()) payload.remark = formData.remark.trim();
      if (formData.type === "CASH_OUT") {
        payload.expensePurpose = formData.expensePurpose;
        if (formData.expensePurpose === "OTHER") {
          payload.customExpensePurpose = formData.customExpensePurpose.trim();
        }
      }
      if (formData.sponsorPurpose) {
        payload.sponsorPurpose = formData.sponsorPurpose;
        if (formData.sponsorPurpose === "OTHER") {
          payload.customSponsorPurpose = formData.customSponsorPurpose.trim();
        }
      }
      if (formData.memberId) payload.memberId = formData.memberId;
      if (formData.sponsorId) payload.sponsorId = formData.sponsorId;
      if (formData.senderName) payload.senderName = formData.senderName;
      if (formData.senderPhone) payload.senderPhone = formData.senderPhone;
      if (formData.category === "SPONSORSHIP") {
        if (formData.sponsorSenderName) payload.sponsorSenderName = formData.sponsorSenderName;
        if (formData.sponsorSenderContact) payload.sponsorSenderContact = formData.sponsorSenderContact;
      }
      if (membershipSectionActive) {
        payload.membershipType = subscriptionType;
        payload.includesSubscription = feeSubs;
        payload.includesAnnualFee = feeAnnual;
        payload.includesApplicationFee = feeApp;
      }

      const res = await apiFetch("/api/transactions", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      const data = await res.json();
      if (!res.ok) {
        const fieldLabels: Record<string, string> = {
          category: "Category", amount: "Amount", paymentMode: "Payment mode",
          purpose: "Purpose", remark: "Remark", sponsorPurpose: "Sponsor purpose",
          customSponsorPurpose: "Sponsor purpose (other)", expensePurpose: "Expense purpose",
          customExpensePurpose: "Expense purpose (other)",
          senderName: "Received By", senderPhone: "Receiver's Contact", type: "Transaction type",
        };
        if (data?.details?.fieldErrors) {
          const msgs = Object.entries(data.details.fieldErrors as Record<string, string[]>)
            .map(([f, errs]) => `${fieldLabels[f] ?? f}: ${errs[0]}`)
            .join(" • ");
          throw new Error(msgs || data.error);
        }
        throw new Error(data.error ?? "Failed to create transaction");
      }

      setShowAddDialog(false);
      await Promise.all([
        mutateTransactions(),
        mutateSummary(),
        revalidateApiPrefixes(
          "/api/dashboard/stats",
          "/api/approvals",
          "/api/members",
          "/api/memberships"
        ),
      ]);
    } catch (err: unknown) {
      setActionError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      setSubmitting(false);
    }
  }

  // ---------------------------------------------------------------------------
  // Render helpers
  // ---------------------------------------------------------------------------

  function TypeBadge({ type }: { type: "CASH_IN" | "CASH_OUT" }) {
    return (
      <Badge
        variant={type === "CASH_IN" ? "default" : "destructive"}
        className={
          type === "CASH_IN"
            ? "bg-emerald-100 text-emerald-800 hover:bg-emerald-100"
            : "bg-rose-100 text-rose-800 hover:bg-rose-100"
        }
      >
        {type === "CASH_IN" ? (
          <ArrowDownIcon className="mr-1 h-3 w-3" />
        ) : (
          <ArrowUpIcon className="mr-1 h-3 w-3" />
        )}
        {type === "CASH_IN" ? "Income" : "Expense"}
      </Badge>
    );
  }

  function StatusBadge({ status }: { status: string }) {
    const variants: Record<string, string> = {
      APPROVED:
        "bg-emerald-100 text-emerald-800 hover:bg-emerald-100",
      PENDING:
        "bg-amber-100 text-amber-800 hover:bg-amber-100",
      REJECTED:
        "bg-rose-100 text-rose-800 hover:bg-rose-100",
    };
    return (
      <Badge className={variants[status] ?? ""}>{status}</Badge>
    );
  }

  // ---------------------------------------------------------------------------
  // Transaction form
  // ---------------------------------------------------------------------------

  function TransactionForm() {
    const filteredMembers = eligibleMembers.filter((m) =>
      memberSearch.trim() === ""
        ? true
        : m.name.toLowerCase().includes(memberSearch.toLowerCase())
    );

    const feeLines: { label: string; amount: number }[] = [];
    if (membershipSectionActive) {
      if (feeAnnual) feeLines.push({ label: "Annual Membership Fee", amount: ANNUAL_MEMBERSHIP_FEE });
      if (feeSubs) feeLines.push({ label: `${subscriptionType === "MONTHLY" ? "Monthly" : subscriptionType === "HALF_YEARLY" ? "Half-yearly" : "Annual"} Subscription`, amount: MEMBERSHIP_FEES[subscriptionType] });
      if (feeApp) feeLines.push({ label: "Application Fee (one-time)", amount: APPLICATION_FEE });
    }

    // Secondary fields only appear once a category / purpose has been chosen
    const categoryChosen = formData.type === "CASH_OUT"
      ? formData.expensePurpose !== ""
      : formData.category !== "";

    return (
      <div className="grid gap-4 py-2">
        {/* Type toggle */}
        <div className="flex gap-2">
          <Button
            type="button"
            variant={formData.type === "CASH_IN" ? "default" : "outline"}
            className={
              formData.type === "CASH_IN"
                ? "flex-1 bg-emerald-600 hover:bg-emerald-700"
                : "flex-1"
            }
            onClick={() => {
              setFormData((prev) => ({ ...prev, type: "CASH_IN", expensePurpose: "", customExpensePurpose: "", category: "" }));
              resetMemberPicker();
            }}
          >
            <ArrowDownIcon className="mr-2 h-4 w-4" />
            Cash In
          </Button>
          <Button
            type="button"
            variant={formData.type === "CASH_OUT" ? "destructive" : "outline"}
            className="flex-1"
            onClick={() => {
              setFormData((prev) => ({ ...prev, type: "CASH_OUT", category: "EXPENSE", sponsorPurpose: "", customSponsorPurpose: "", expensePurpose: "" }));
              resetMemberPicker();
            }}
          >
            <ArrowUpIcon className="mr-2 h-4 w-4" />
            Cash Out
          </Button>
        </div>

        {/* ── Cash In: Category (first field) ── */}
        {formData.type === "CASH_IN" && (
          <div className="grid gap-1.5">
            <Label>Category *</Label>
            <Select
              value={membershipSectionActive ? "MEMBERSHIP" : formData.category}
              onValueChange={handleCategoryChange}
            >
              <SelectTrigger>
                <SelectValue placeholder="Select category" />
              </SelectTrigger>
              <SelectContent>
                {CASH_IN_CATEGORIES.map((c) => (
                  <SelectItem key={c.value} value={c.value}>
                    {c.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        )}

        {/* ── Cash Out: Expense Purpose (first field) ── */}
        {formData.type === "CASH_OUT" && (
          <>
            <div className="grid gap-1.5">
              <Label>Purpose *</Label>
              <Select
                value={formData.expensePurpose}
                onValueChange={(v) => setFormData((prev) => ({ ...prev, expensePurpose: v, customExpensePurpose: "" }))}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select expense purpose" />
                </SelectTrigger>
                <SelectContent>
                  {EXPENSE_PURPOSES.map((e) => (
                    <SelectItem key={e.value} value={e.value}>
                      {e.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            {formData.expensePurpose === "OTHER" && (
              <div className="grid gap-1.5">
                <Label>Specify Purpose *</Label>
                <Input
                  placeholder="Enter expense purpose"
                  value={formData.customExpensePurpose}
                  onChange={(e) => updateForm("customExpensePurpose", e.target.value)}
                />
              </div>
            )}
          </>
        )}

        {/* ── All secondary fields: only shown once a category/purpose is chosen ── */}
        {categoryChosen && (
          <>
            {/* Sponsor purpose — only when category = SPONSORSHIP */}
            {formData.category === "SPONSORSHIP" && (
              <>
                <div className="grid gap-1.5">
                  <Label>Sponsor Purpose *</Label>
                  <Select
                    value={formData.sponsorPurpose}
                    onValueChange={(v) => setFormData((prev) => ({ ...prev, sponsorPurpose: v, customSponsorPurpose: "" }))}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select sponsor type" />
                    </SelectTrigger>
                    <SelectContent>
                      {SPONSOR_PURPOSES.map((s) => (
                        <SelectItem key={s.value} value={s.value}>
                          {s.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
                {formData.sponsorPurpose === "OTHER" && (
                  <div className="grid gap-1.5">
                    <Label>Specify Sponsor Purpose *</Label>
                    <Input
                      placeholder="Enter sponsor purpose"
                      value={formData.customSponsorPurpose}
                      onChange={(e) => updateForm("customSponsorPurpose", e.target.value)}
                    />
                  </div>
                )}
                <div className="grid gap-1.5">
                  <Label>Sender&apos;s Name *</Label>
                  <Input
                    placeholder="Name of person or company who sent the sponsorship"
                    value={formData.sponsorSenderName}
                    onChange={(e) => updateForm("sponsorSenderName", e.target.value)}
                  />
                </div>
                <div className="grid gap-1.5">
                  <Label>Sender&apos;s Contact (optional)</Label>
                  <Input
                    placeholder="Phone, email, or UPI ID of sender"
                    value={formData.sponsorSenderContact}
                    onChange={(e) => updateForm("sponsorSenderContact", e.target.value)}
                  />
                </div>
              </>
            )}

            {/* ── Membership section ── */}
            {membershipSectionActive && (
              <>
                <div className="grid gap-1.5">
                  <Label>Member *</Label>
                  <div ref={memberPickerRef} className="relative">
                    <div className="relative">
                      <Input
                        placeholder={eligibleMembersLoading ? "Loading members…" : "Search by member name…"}
                        value={memberSearch}
                        disabled={eligibleMembersLoading}
                        onChange={(e) => {
                          setMemberSearch(e.target.value);
                          setMemberPickerOpen(true);
                          if (selectedFeeMember && e.target.value !== selectedFeeMember.name) {
                            setSelectedFeeMember(null);
                            setFormData((prev) => ({ ...prev, memberId: "" }));
                          }
                        }}
                        onFocus={() => { if (!eligibleMembersLoading) setMemberPickerOpen(true); }}
                        className="pr-16"
                      />
                      <div className="absolute inset-y-0 right-0 flex items-center pr-2 gap-1">
                        {selectedFeeMember && (
                          <button
                            type="button"
                            className="p-1 rounded hover:bg-slate-100"
                            onClick={() => {
                              setSelectedFeeMember(null);
                              setMemberSearch("");
                              setFormData((prev) => ({ ...prev, memberId: "" }));
                            }}
                          >
                            <XIcon className="h-3.5 w-3.5 text-slate-400" />
                          </button>
                        )}
                        <ChevronDownIcon className="h-4 w-4 text-slate-400 pointer-events-none" />
                      </div>
                    </div>
                    {memberPickerOpen && !eligibleMembersLoading && (
                      <div className="absolute z-50 w-full mt-1 border rounded-md bg-white shadow-lg max-h-48 overflow-y-auto">
                        {filteredMembers.length === 0 ? (
                          <p className="px-3 py-2 text-sm text-muted-foreground">No members found.</p>
                        ) : (
                          filteredMembers.map((m) => (
                            <button
                              key={m.id}
                              type="button"
                              className="w-full text-left px-3 py-2 hover:bg-slate-50 text-sm"
                              onMouseDown={(e) => e.preventDefault()}
                              onClick={() => handleMemberSelect(m)}
                            >
                              <span className="font-medium truncate block">{m.name}</span>
                            </button>
                          ))
                        )}
                      </div>
                    )}
                  </div>
                </div>

                {selectedFeeMember && (
                  <div className="rounded-lg border border-slate-200 bg-slate-50 p-3 text-sm space-y-2">
                    <span className="font-semibold text-slate-800">{selectedFeeMember.name}</span>
                    <div className="text-muted-foreground space-y-0.5">
                      <p>{selectedFeeMember.email}</p>
                      <p>{selectedFeeMember.phone}</p>
                      <p className="text-xs truncate">{selectedFeeMember.address}</p>
                    </div>
                    {selectedFeeMember.subMembers && selectedFeeMember.subMembers.length > 0 && (
                      <div className="pt-1 border-t border-slate-200">
                        <p className="text-xs font-medium text-slate-600 mb-1">Sub-members</p>
                        <div className="space-y-0.5">
                          {selectedFeeMember.subMembers.map((sm) => (
                            <div key={sm.id} className="flex items-center gap-2 text-xs text-slate-600">
                              <span className="font-medium">{sm.name}</span>
                              <span className="text-muted-foreground">({sm.relation})</span>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}

                <div className="space-y-2">
                  <Label>Select Fees *</Label>
                  <div className="flex items-center space-x-3 rounded-md border p-3">
                    <Checkbox id="fee-annual" checked={feeAnnual} onCheckedChange={(v) => handleFeeAnnualChange(v === true)} />
                    <label htmlFor="fee-annual" className="flex-1 cursor-pointer">
                      <p className="text-sm font-medium">Annual Membership Fee</p>
                      <p className="text-xs text-muted-foreground">₹5,000 — valid for 1 year</p>
                    </label>
                  </div>
                  <div className="rounded-md border">
                    <div className="flex items-center space-x-3 p-3">
                      <Checkbox id="fee-subs" checked={feeSubs} onCheckedChange={(v) => handleFeeSubsChange(v === true)} />
                      <label htmlFor="fee-subs" className="flex-1 cursor-pointer">
                        <p className="text-sm font-medium">Subscription Fee</p>
                        <p className="text-xs text-muted-foreground">Recurring plan-based fee</p>
                      </label>
                    </div>
                    {feeSubs && (
                      <div className="px-3 pb-3">
                        <Select value={subscriptionType} onValueChange={(v) => handleSubscriptionTypeChange(v as MembershipType)}>
                          <SelectTrigger><SelectValue /></SelectTrigger>
                          <SelectContent>
                            <SelectItem value="MONTHLY">Monthly — ₹250</SelectItem>
                            <SelectItem value="HALF_YEARLY">Half-yearly — ₹1,500</SelectItem>
                            <SelectItem value="ANNUAL">Annual — ₹3,000</SelectItem>
                          </SelectContent>
                        </Select>
                      </div>
                    )}
                  </div>
                  <div className="flex items-center space-x-3 rounded-md border p-3">
                    <Checkbox id="fee-app" checked={feeApp} onCheckedChange={(v) => handleFeeAppChange(v === true)} />
                    <label htmlFor="fee-app" className="flex-1 cursor-pointer">
                      <p className="text-sm font-medium">Application Fee</p>
                      <p className="text-xs text-muted-foreground">₹10,000 — one-time, first membership only</p>
                    </label>
                  </div>
                </div>

                {feeLines.length > 0 && (
                  <div className="rounded-md bg-muted p-3 space-y-1.5">
                    {feeLines.map((l) => (
                      <div key={l.label} className="flex justify-between text-sm">
                        <span>{l.label}</span>
                        <span>₹{l.amount.toLocaleString("en-IN")}</span>
                      </div>
                    ))}
                    <div className="border-t pt-1.5 flex justify-between font-bold text-sm">
                      <span>Total</span>
                      <span>₹{feeLines.reduce((s, l) => s + l.amount, 0).toLocaleString("en-IN")}</span>
                    </div>
                  </div>
                )}
              </>
            )}

            {/* Amount — hidden for membership (auto-calculated via checkboxes) */}
            {!membershipSectionActive && (
              <div className="grid gap-1.5">
                <Label>Amount (₹) *</Label>
                <Input
                  type="number"
                  min="0.01"
                  step="0.01"
                  placeholder="0"
                  value={formData.amount}
                  onChange={(e) => updateForm("amount", e.target.value)}
                />
              </div>
            )}

            {/* Payment Mode (mandatory) */}
            <div className="grid gap-1.5">
              <Label>Payment Mode *</Label>
              <Select value={formData.paymentMode} onValueChange={(v) => updateForm("paymentMode", v)}>
                <SelectTrigger>
                  <SelectValue placeholder="Select payment mode" />
                </SelectTrigger>
                <SelectContent>
                  {PAYMENT_MODES.map((m) => (
                    <SelectItem key={m.value} value={m.value}>{m.label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Received By (mandatory for all categories) */}
            <div className="grid gap-1.5">
              <Label>Received By *</Label>
              <Input
                placeholder="Name of staff or external agency who received the payment"
                value={formData.senderName}
                onChange={(e) => updateForm("senderName", e.target.value)}
              />
            </div>

            {/* Receiver's Contact — mandatory for Cash Out, optional for Cash In */}
            <div className="grid gap-1.5">
              <Label>{formData.type === "CASH_OUT" ? "Receiver's Contact *" : "Receiver's Contact (optional)"}</Label>
              <Input
                placeholder="Phone number, email, or UPI ID"
                value={formData.senderPhone}
                onChange={(e) => updateForm("senderPhone", e.target.value)}
              />
            </div>

            {/* Remark (optional) */}
            <div className="grid gap-1.5">
              <Label>Remark (optional)</Label>
              <Input
                placeholder="Any additional note"
                value={formData.remark}
                onChange={(e) => updateForm("remark", e.target.value)}
              />
            </div>
          </>
        )}
      </div>
    );
  }

  // ---------------------------------------------------------------------------
  // Main render
  // ---------------------------------------------------------------------------

  return (
    <div className="space-y-6 p-4 sm:p-6">
      {/* Page header */}
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">Cash Management</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Track all income and expense transactions.
          </p>
        </div>
        <div className="flex w-full flex-wrap gap-2 sm:w-auto">
          <Button
            variant="outline"
            size="sm"
            disabled={loading || summaryLoading}
            onClick={() => {
              void Promise.all([mutateTransactions(), mutateSummary()]);
            }}
          >
            <RefreshCwIcon className="h-4 w-4 mr-1" />
            Refresh
          </Button>
          {canWrite && (
            <Button size="sm" onClick={openAddDialog}>
              <PlusIcon className="h-4 w-4 mr-1" />
              {isOperator ? "Submit for Approval" : "Add Transaction"}
            </Button>
          )}
        </div>
      </div>

      {/* Summary cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <ArrowDownIcon className="h-4 w-4 text-emerald-600" />
              Total Income
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold text-emerald-700">
              {formatCurrency(summary.totalIncome)}
            </p>
            <p className="text-xs text-muted-foreground mt-1">Approved only</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <ArrowUpIcon className="h-4 w-4 text-rose-600" />
              Total Expenses
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold text-rose-700">
              {formatCurrency(summary.totalExpenses)}
            </p>
            <p className="text-xs text-muted-foreground mt-1">Approved only</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <ClockIcon className="h-4 w-4 text-amber-600" />
              Pending Approvals
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-2xl font-bold text-amber-700">
              {formatCurrency(summary.pendingAmount)}
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              Awaiting admin review
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground flex items-center gap-2">
              <WalletIcon className="h-4 w-4 text-sky-600" />
              Net Balance
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p
              className={`text-2xl font-bold ${
                summary.netBalance >= 0 ? "text-sky-700" : "text-rose-700"
              }`}
            >
              {formatCurrency(summary.netBalance)}
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              Income minus expenses
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Filters */}
      <div className="flex flex-wrap gap-3 items-end">
        <div className="w-36">
          <Label className="text-xs mb-1 block">Type</Label>
          <Select
            value={filterType}
            onValueChange={(value) => {
              setFilterType(value);
              setPage(1);
            }}
          >
            <SelectTrigger className="h-9">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="ALL">All Types</SelectItem>
              <SelectItem value="CASH_IN">Cash In</SelectItem>
              <SelectItem value="CASH_OUT">Cash Out</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="w-40">
          <Label className="text-xs mb-1 block">Category</Label>
          <Select
            value={filterCategory}
            onValueChange={(value) => {
              setFilterCategory(value);
              setPage(1);
            }}
          >
            <SelectTrigger className="h-9">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="ALL">All Categories</SelectItem>
              {CATEGORIES.map((c) => (
                <SelectItem key={c.value} value={c.value}>
                  {c.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="w-40">
          <Label className="text-xs mb-1 block">Payment Mode</Label>
          <Select
            value={filterPaymentMode}
            onValueChange={(value) => {
              setFilterPaymentMode(value);
              setPage(1);
            }}
          >
            <SelectTrigger className="h-9">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="ALL">All Modes</SelectItem>
              {PAYMENT_MODES.map((m) => (
                <SelectItem key={m.value} value={m.value}>
                  {m.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="w-36">
          <Label className="text-xs mb-1 block">Status</Label>
          <Select
            value={filterStatus}
            onValueChange={(value) => {
              setFilterStatus(value);
              setPage(1);
            }}
          >
            <SelectTrigger className="h-9">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="ALL">All Statuses</SelectItem>
              {APPROVAL_STATUSES.map((s) => (
                <SelectItem key={s.value} value={s.value}>
                  {s.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div>
          <Label className="text-xs mb-1 block">From</Label>
          <Input
            type="date"
            className="h-9 w-36"
            value={filterDateFrom}
            onChange={(e) => {
              setFilterDateFrom(e.target.value);
              setPage(1);
            }}
          />
        </div>

        <div>
          <Label className="text-xs mb-1 block">To</Label>
          <Input
            type="date"
            className="h-9 w-36"
            value={filterDateTo}
            onChange={(e) => {
              setFilterDateTo(e.target.value);
              setPage(1);
            }}
          />
        </div>

        <Button
          variant="ghost"
          size="sm"
          className="h-9"
          onClick={() => {
            setFilterType("ALL");
            setFilterCategory("ALL");
            setFilterPaymentMode("ALL");
            setFilterStatus("ALL");
            setFilterDateFrom("");
            setFilterDateTo("");
            setPage(1);
          }}
        >
          Clear
        </Button>
      </div>

      {/* Error */}
      {error && (
        <div className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
          {error.message}
        </div>
      )}

      {/* Transaction table */}
      <div className="overflow-x-auto rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Date</TableHead>
              <TableHead>Type</TableHead>
              <TableHead>Category</TableHead>
              <TableHead className="text-right">Amount</TableHead>
              <TableHead>Mode</TableHead>
              <TableHead>Purpose</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Entered By</TableHead>
              {canWrite && <TableHead className="w-24 text-center">Receipt</TableHead>}
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading && (
              <TableRow>
                <TableCell colSpan={canWrite ? 9 : 8} className="text-center py-8 text-muted-foreground">
                  Loading...
                </TableCell>
              </TableRow>
            )}
            {!loading && transactions.length === 0 && (
              <TableRow>
                <TableCell colSpan={canWrite ? 9 : 8} className="text-center py-8 text-muted-foreground">
                  No transactions found.
                </TableCell>
              </TableRow>
            )}
            {!loading &&
              transactions.map((t) => (
                <TableRow
                  key={t.id}
                  className="cursor-pointer hover:bg-muted/60"
                  onClick={() => openDetailDialog(t)}
                >
                  <TableCell className="whitespace-nowrap text-sm">
                    {formatDate(t.createdAt)}
                  </TableCell>
                  <TableCell>
                    <TypeBadge type={t.type} />
                  </TableCell>
                  <TableCell className="text-sm">
                    {getCategoryLabel(t.category)}
                    {t.sponsorPurpose && (
                      <div className="text-xs text-muted-foreground">
                        {getSponsorPurposeLabel(t.sponsorPurpose)}
                      </div>
                    )}
                  </TableCell>
                  <TableCell className="text-right font-mono font-medium">
                    {formatCurrency(t.amount)}
                  </TableCell>
                  <TableCell className="text-sm">
                    {getPaymentModeLabel(t.paymentMode)}
                  </TableCell>
                  <TableCell className="text-sm max-w-[200px] truncate">
                    {t.purpose}
                    {t.approvalSource === "RAZORPAY_WEBHOOK" && (
                      <Badge className="ml-2 bg-indigo-100 text-xs text-indigo-800">
                        Razorpay
                      </Badge>
                    )}
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={t.approvalStatus} />
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {t.enteredBy?.name ?? "—"}
                  </TableCell>
                  {canWrite && (
                    <TableCell className="text-center">
                      <Button
                        variant="ghost"
                        size="icon"
                        className={`mx-auto h-9 w-9 ${
                          t.receipt?.receiptNumber
                            ? "text-sky-600 hover:bg-sky-50 hover:text-sky-700"
                            : "cursor-not-allowed text-slate-300"
                        }`}
                        disabled={!t.receipt?.receiptNumber}
                        title={
                          t.receipt?.receiptNumber
                            ? "View / Print Receipt"
                            : "Receipt not available for this transaction"
                        }
                        onClick={(e) => { e.stopPropagation(); openReceiptDialog(t); }}
                      >
                        <ReceiptIcon className="h-4.5 w-4.5" />
                      </Button>
                    </TableCell>
                  )}
                </TableRow>
              ))}
          </TableBody>
        </Table>
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between text-sm">
          <span className="text-muted-foreground">
            Showing {(page - 1) * 20 + 1}–{Math.min(page * 20, total)} of{" "}
            {total}
          </span>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={page === 1}
              onClick={() => setPage((p) => p - 1)}
            >
              Previous
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={page === totalPages}
              onClick={() => setPage((p) => p + 1)}
            >
              Next
            </Button>
          </div>
        </div>
      )}

      {/* Transaction Detail Dialog */}
      <Dialog open={showDetailDialog} onOpenChange={setShowDetailDialog}>
        <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <EyeIcon className="h-4 w-4 text-muted-foreground" />
              Transaction Details
            </DialogTitle>
          </DialogHeader>
          {detailTransaction && (
            <div className="space-y-4 text-sm">
              {/* ID + Date */}
              <div className="grid grid-cols-2 gap-x-6 gap-y-3 rounded-md border p-4">
                <div>
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Transaction ID</p>
                  <p className="mt-0.5 font-mono text-xs break-all">{detailTransaction.id}</p>
                </div>
                <div>
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Date &amp; Time</p>
                  <p className="mt-0.5">{new Date(detailTransaction.createdAt).toLocaleString("en-IN", { dateStyle: "medium", timeStyle: "short" })}</p>
                </div>
              </div>

              {/* Core fields */}
              <div className="grid grid-cols-2 gap-x-6 gap-y-3 rounded-md border p-4">
                <div>
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Type</p>
                  <p className="mt-0.5">{detailTransaction.type === "CASH_IN" ? "Cash In" : "Cash Out"}</p>
                </div>
                <div>
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Category</p>
                  <p className="mt-0.5">{getCategoryLabel(detailTransaction.category)}</p>
                </div>
                <div>
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Amount</p>
                  <p className="mt-0.5 font-semibold">{formatCurrency(detailTransaction.amount)}</p>
                </div>
                <div>
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Payment Mode</p>
                  <p className="mt-0.5">{getPaymentModeLabel(detailTransaction.paymentMode)}</p>
                </div>
                <div className="col-span-2">
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Purpose</p>
                  <p className="mt-0.5">{detailTransaction.purpose || "—"}</p>
                </div>
                {detailTransaction.sponsorPurpose && (
                  <div className="col-span-2">
                    <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Sponsor Purpose</p>
                    <p className="mt-0.5">{getSponsorPurposeLabel(detailTransaction.sponsorPurpose)}</p>
                  </div>
                )}
                {detailTransaction.remark && (
                  <div className="col-span-2">
                    <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Remark</p>
                    <p className="mt-0.5">{detailTransaction.remark}</p>
                  </div>
                )}
              </div>

              {/* Approval */}
              <div className="grid grid-cols-2 gap-x-6 gap-y-3 rounded-md border p-4">
                <div className="col-span-2">
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Approval Status</p>
                  <p className="mt-0.5">{detailTransaction.approvalStatus}</p>
                </div>
                {detailTransaction.approvedBy && (
                  <div className="col-span-2">
                    <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Approved By</p>
                    <p className="mt-0.5">{detailTransaction.approvedBy.name}</p>
                    <p className="text-xs text-muted-foreground font-mono">{detailTransaction.approvedBy.id}</p>
                  </div>
                )}
              </div>

              {/* People */}
              <div className="grid grid-cols-1 gap-y-3 rounded-md border p-4">
                <div>
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Entered By</p>
                  <div className="mt-0.5 flex items-center gap-2">
                    <span>{detailTransaction.enteredBy?.name ?? "—"}</span>
                    {detailTransaction.enteredBy?.role && (
                      <Badge className="text-[10px] px-1.5 py-0 bg-slate-100 text-slate-600 hover:bg-slate-100">
                        {detailTransaction.enteredBy.role}
                      </Badge>
                    )}
                  </div>
                </div>
                {detailTransaction.sponsor && (
                  <div>
                    <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Sponsor</p>
                    <p className="mt-0.5">{detailTransaction.sponsor.name}</p>
                    {detailTransaction.sponsor.company && (
                      <p className="text-xs text-muted-foreground">{detailTransaction.sponsor.company}</p>
                    )}
                    <p className="text-xs text-muted-foreground font-mono">{detailTransaction.sponsor.id}</p>
                  </div>
                )}
                {/* Sent By — always shown for all incoming (CASH_IN) membership/sponsorship */}
                {detailTransaction.type === "CASH_IN" &&
                  (detailTransaction.category === "MEMBERSHIP" || detailTransaction.category === "SPONSORSHIP") && (
                  <div>
                    <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Sent By</p>
                    {detailTransaction.category === "MEMBERSHIP" ? (
                      detailMemberLoading ? (
                        <div className="mt-1 space-y-1 animate-pulse">
                          <div className="h-4 bg-muted rounded w-40" />
                          <div className="h-3 bg-muted rounded w-28" />
                        </div>
                      ) : (
                        <>
                          <p className="mt-0.5">
                            {(detailMemberData?.name as string) || detailTransaction.member?.name || "—"}
                          </p>
                          {(detailMemberData?.phone as string) && (
                            <p className="text-xs text-muted-foreground">
                              {detailMemberData?.phone as string}
                            </p>
                          )}
                        </>
                      )
                    ) : (
                      <>
                        <p className="mt-0.5">{detailTransaction.sponsorSenderName || "—"}</p>
                        {detailTransaction.sponsorSenderContact && (
                          <p className="text-xs text-muted-foreground">{detailTransaction.sponsorSenderContact}</p>
                        )}
                      </>
                    )}
                  </div>
                )}
                {/* Received By — always shown */}
                <div>
                  <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Received By</p>
                  <p className="mt-0.5">{detailTransaction.senderName || "—"}</p>
                  {detailTransaction.senderPhone && (
                    <p className="text-xs text-muted-foreground">{detailTransaction.senderPhone}</p>
                  )}
                </div>
              </div>

              {/* Receipt */}
              {detailTransaction.receipt && (
                <div className="grid grid-cols-2 gap-x-6 gap-y-3 rounded-md border p-4">
                  <div>
                    <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Receipt Number</p>
                    <p className="mt-0.5 font-mono">{detailTransaction.receipt.receiptNumber}</p>
                  </div>
                  <div>
                    <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Receipt Status</p>
                    <p className="mt-0.5">{detailTransaction.receipt.status}</p>
                  </div>
                </div>
              )}
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowDetailDialog(false)}>
              Close
            </Button>
            {detailTransaction?.receipt?.receiptNumber && canWrite && (
              <Button
                onClick={() => {
                  setShowDetailDialog(false);
                  void openReceiptDialog(detailTransaction);
                }}
              >
                <ReceiptIcon className="h-4 w-4 mr-1" />
                View Receipt
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Add Transaction Dialog */}
      <Dialog
        open={showAddDialog}
        onOpenChange={(open) => {
          setShowAddDialog(open);
          if (!open) resetMemberPicker();
        }}
      >
        <DialogContent className="sm:max-w-md max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>
              {isOperator ? "Submit Transaction for Approval" : "Add Transaction"}
            </DialogTitle>
          </DialogHeader>
          {TransactionForm()}
          {actionError && (
            <p className="mt-1 text-sm text-rose-600">{actionError}</p>
          )}
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowAddDialog(false)}
              disabled={submitting}
            >
              Cancel
            </Button>
            <Button onClick={handleCreate} disabled={submitting}>
              {submitting
                ? "Saving..."
                : isOperator
                ? "Submit for Approval"
                : "Create Transaction"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Receipt Dialog */}
      <Dialog open={showReceiptDialog} onOpenChange={setShowReceiptDialog}>
        <DialogContent className="w-[min(96vw,1120px)] max-w-none overflow-hidden p-0">
          <DialogHeader className="border-b border-slate-200 px-5 py-4 sm:px-6">
            <DialogTitle>
              {receiptData
                ? `Receipt ${receiptData.receiptNumber}`
                : "Generating Receipt..."}
            </DialogTitle>
          </DialogHeader>
          <div className="max-h-[calc(100vh-10rem)] overflow-y-auto px-4 py-4 sm:px-6 sm:py-5">
            {receiptLoading && (
              <div className="text-center py-8 text-muted-foreground text-sm">
                Generating receipt...
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
          <DialogFooter className="border-t border-slate-200 px-5 py-4 sm:px-6">
            <Button
              variant="outline"
              onClick={() => setShowReceiptDialog(false)}
            >
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

    </div>
  );
}
