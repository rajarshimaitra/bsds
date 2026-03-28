import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

/**
 * Merge Tailwind CSS classes with clsx and tailwind-merge.
 * Used by all shadcn/ui components.
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// ---------------------------------------------------------------------------
// T29 — Data formatting utilities
// ---------------------------------------------------------------------------

/**
 * Format a number as Indian Rupee currency.
 * e.g. 10000 → "₹10,000"
 */
export function formatCurrency(amount: number | string): string {
  const num = typeof amount === "string" ? parseFloat(amount) : amount;
  if (isNaN(num)) return "₹0";
  return new Intl.NumberFormat("en-IN", {
    style: "currency",
    currency: "INR",
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(num);
}

/**
 * Format a date as DD/MM/YYYY.
 * Returns "—" for null/undefined/invalid dates.
 */
export function formatDate(date: string | Date | null | undefined): string {
  if (!date) return "—";
  const d = new Date(date);
  if (isNaN(d.getTime())) return "—";
  return d.toLocaleDateString("en-IN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  });
}

/**
 * Format a date as DD/MM/YYYY HH:MM (24h).
 * Returns "—" for null/undefined/invalid dates.
 */
export function formatDateTime(date: string | Date | null | undefined): string {
  if (!date) return "—";
  const d = new Date(date);
  if (isNaN(d.getTime())) return "—";
  const datePart = formatDate(d);
  const hours = String(d.getHours()).padStart(2, "0");
  const minutes = String(d.getMinutes()).padStart(2, "0");
  return `${datePart} ${hours}:${minutes}`;
}

/**
 * Normalise a phone number to +91 format.
 * Handles: +91XXXXXXXXXX, 91XXXXXXXXXX, XXXXXXXXXX (10-digit).
 * Returns the original string if it cannot be normalised.
 */
export function formatPhone(phone: string): string {
  if (!phone) return phone;
  if (phone.startsWith("+91")) return phone;
  if (phone.startsWith("91") && phone.length === 12) return "+" + phone;
  if (phone.length === 10) return "+91" + phone;
  return phone;
}

/**
 * Format a member ID — already in BSDS-YYYY-NNNN-SS format, returned as-is.
 */
export function formatMemberId(id: string): string {
  return id;
}

/**
 * Convert an enum-style string to Title Case.
 * e.g. "TITLE_SPONSOR" → "Title Sponsor", "HALF_YEARLY" → "Half Yearly"
 */
function toTitleCase(str: string): string {
  return str
    .replace(/_/g, " ")
    .toLowerCase()
    .replace(/\b\w/g, (l) => l.toUpperCase());
}

/**
 * Format a sponsor purpose enum value for display.
 * e.g. "TITLE_SPONSOR" → "Title Sponsor"
 */
export function formatSponsorPurpose(purpose: string): string {
  return toTitleCase(purpose);
}

/**
 * Format a membership type enum value for display.
 * e.g. "HALF_YEARLY" → "Half-Yearly"
 */
export function formatMembershipType(type: string | null | undefined): string {
  if (!type) return "—";
  switch (type) {
    case "MONTHLY":
      return "Monthly";
    case "HALF_YEARLY":
      return "Half-Yearly";
    case "ANNUAL":
      return "Annual";
    default:
      return type;
  }
}

/**
 * Format a membership status enum value for display.
 * e.g. "PENDING_APPROVAL" → "Pending Approval"
 */
export function formatMembershipStatus(status: string): string {
  return toTitleCase(status);
}

/**
 * Format a fee type enum value for display.
 * e.g. "ANNUAL_FEE" → "Annual Fee", "SUBSCRIPTION" → "Subscription"
 */
export function formatFeeType(feeType: string | null | undefined): string {
  if (!feeType) return "—";
  switch (feeType) {
    case "ANNUAL_FEE": return "Annual Fee";
    case "SUBSCRIPTION": return "Subscription";
    default: return feeType;
  }
}
