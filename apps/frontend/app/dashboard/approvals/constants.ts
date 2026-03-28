// Approvals page constants — extracted to reduce chunk size

export const ENTITY_TYPE_LABELS: Record<string, string> = {
  TRANSACTION: "Transaction",
  MEMBER_ADD: "New Member",
  MEMBER_EDIT: "Member Edit",
  MEMBER_DELETE: "Member Delete",
  MEMBERSHIP: "Membership",
};

export const APPROVAL_TYPE_LABELS: Record<string, string> = {
  MEMBERSHIP_APPROVAL: "Membership Approval",
  MEMBERSHIP_PAYMENT_APPROVAL: "Membership Payment Approval",
  TRANSACTION_APPROVAL: "Transaction Approval",
};

export const DIRECTION_LABELS: Record<string, string> = {
  INCOMING: "Incoming",
  OUTGOING: "Outgoing",
};

export const ACTION_LABELS: Record<string, string> = {
  add_member: "Add Member",
  edit_member: "Edit Member",
  delete_member: "Delete Member",
  add_sub_member: "Add Sub-member",
  edit_sub_member: "Edit Sub-member",
  remove_sub_member: "Remove Sub-member",
  add_transaction: "Add Transaction",
  edit_transaction: "Edit Transaction",
  delete_transaction: "Delete Transaction",
  approve_transaction: "Transaction Approval",
  add_membership: "Add Membership",
  create_membership: "Create Membership",
  approve_membership: "Membership Approval",
  approve: "Approve",
};

export const STATUS_CLASSES: Record<string, string> = {
  PENDING: "bg-amber-100 text-amber-800",
  APPROVED: "bg-emerald-100 text-emerald-800",
  REJECTED: "bg-rose-100 text-rose-800",
};

export const ENTITY_TYPE_COLORS: Record<string, string> = {
  TRANSACTION: "bg-sky-100 text-sky-800",
  MEMBER_ADD: "bg-emerald-100 text-emerald-800",
  MEMBER_EDIT: "bg-amber-100 text-amber-800",
  MEMBER_DELETE: "bg-rose-100 text-rose-800",
  MEMBERSHIP: "bg-indigo-100 text-indigo-800",
};

export const FIELD_LABELS: Record<string, string> = {
  type: "Transaction Type",
  category: "Category",
  amount: "Amount",
  paymentMode: "Payment Mode",
  description: "Description",
  sponsorPurpose: "Sponsor Purpose",
  senderName: "Received By",
  senderPhone: "Contact",
  remark: "Remark",
  approvalStatus: "Approval Status",
  transactionId: "Transaction ID",
  deleted: "Deleted",
  name: "Full Name",
  email: "Email Address",
  phone: "Phone Number",
  address: "Address",
  relation: "Relation to Member",
  plan: "Plan",
  startDate: "Start Date",
  endDate: "End Date",
  fee: "Fee",
  memberId: "Member ID",
  parentMemberId: "Parent Member ID",
  status: "Status",
  notes: "Notes",
  canLogin: "Can Login",
};

export const ENUM_LABELS: Record<string, Record<string, string>> = {
  type: {
    CASH_IN: "Cash In",
    CASH_OUT: "Cash Out",
    ANNUAL: "Annual",
    HONORARY: "Honorary",
    LIFE: "Life",
    ASSOCIATE: "Associate",
  },
  approvalStatus: {
    PENDING: "Pending",
    APPROVED: "Approved",
    REJECTED: "Rejected",
  },
  category: {
    MEMBERSHIP: "Membership",
    SPONSORSHIP: "Sponsorship",
    EXPENSE: "Expense",
    OTHER: "Other",
  },
  paymentMode: { UPI: "UPI", BANK_TRANSFER: "Bank Transfer", CASH: "Cash" },
  sponsorPurpose: {
    TITLE_SPONSOR: "Title Sponsor",
    GOLD_SPONSOR: "Gold Sponsor",
    SILVER_SPONSOR: "Silver Sponsor",
    FOOD_PARTNER: "Food Partner",
    MEDIA_PARTNER: "Media Partner",
    STALL_VENDOR: "Stall Vendor",
    MARKETING_PARTNER: "Marketing Partner",
  },
};

export const SKIP_KEYS = new Set(["parentUserId", "sponsorId", "approvalStatus", "id", "membershipId"]);

export const PLACEHOLDER_ID = "00000000-0000-0000-0000-000000000000";

export interface ApprovalRecord {
  id: string;
  entityType: string;
  entityId: string;
  action: string;
  approvalType: string;
  approvalTypeLabel: string;
  direction: string | null;
  previousData: Record<string, unknown> | null;
  newData: Record<string, unknown> | null;
  status: string;
  notes: string | null;
  reviewedAt: string | null;
  createdAt: string;
  requestedBy: {
    id: string;
    name: string;
    email: string;
    role: string;
  };
  reviewedBy: {
    id: string;
    name: string;
    email: string;
  } | null;
}

export interface ApprovalsResponse {
  data: ApprovalRecord[];
  total: number;
  page: number;
  limit: number;
  totalPages: number;
  pendingCount: number;
}

export function entityApiUrl(entityType: string, entityId: string, action: string): string | null {
  if (entityId === PLACEHOLDER_ID) return null;
  if (entityType === "TRANSACTION") return `/api/transactions/${entityId}`;
  if (entityType === "MEMBER_ADD") {
    if (action === "add_sub_member") return null;
    return `/api/members/${entityId}`;
  }
  if (entityType === "MEMBER_EDIT") {
    if (action === "edit_sub_member") return null;
    return `/api/members/${entityId}`;
  }
  if (entityType === "MEMBER_DELETE") {
    if (action === "remove_sub_member") return null;
    return `/api/members/${entityId}`;
  }
  if (entityType === "MEMBERSHIP") {
    if (action === "approve_membership") return `/api/members/${entityId}`;
    return `/api/memberships/${entityId}`;
  }
  return null;
}

export function formatFieldValue(
  key: string,
  val: unknown,
  txType?: unknown
): { text: string; className?: string } {
  if (val === null || val === undefined || val === "") return { text: "—" };
  const str = String(val);

  if (ENUM_LABELS[key]?.[str]) return { text: ENUM_LABELS[key][str] };

  if (key === "amount") {
    const n = parseFloat(str);
    const formatted = isNaN(n)
      ? str
      : `₹${n.toLocaleString("en-IN", { minimumFractionDigits: 0, maximumFractionDigits: 0 })}`;
    const className =
      txType === "CASH_IN"
        ? "text-emerald-700 font-semibold"
        : txType === "CASH_OUT"
        ? "text-rose-700 font-semibold"
        : "font-semibold";
    return { text: formatted, className };
  }

  if (key === "startDate" || key === "endDate") {
    try {
      return {
        text: new Date(str).toLocaleDateString("en-IN", {
          day: "2-digit",
          month: "2-digit",
          year: "numeric",
        }),
      };
    } catch {
      // fall through
    }
  }

  if (key === "memberId") {
    return { text: str.slice(0, 8) + "…", className: "font-mono text-xs" };
  }

  return { text: str };
}

export function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString("en-IN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}
