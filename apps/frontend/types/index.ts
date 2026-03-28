// Shared frontend types for the BSDS dashboard.

/**
 * User roles in the system.
 * ADMIN > OPERATOR > MEMBER in terms of permissions.
 */
export type Role = "ADMIN" | "OPERATOR" | "ORGANISER" | "MEMBER";

/**
 * Membership lifecycle status.
 * Note: Different from Membership.status (which is approval status of a payment period).
 */
export type MembershipStatus =
  | "PENDING_APPROVAL"
  | "PENDING_PAYMENT"
  | "ACTIVE"
  | "EXPIRED"
  | "SUSPENDED";

/**
 * Membership payment period types.
 * Fees: Monthly ₹250, Half-yearly ₹1,500, Annual ₹3,000.
 */
export type MembershipType = "MONTHLY" | "HALF_YEARLY" | "ANNUAL";

/**
 * Fee type — annual one-time fee vs recurring subscription.
 */
export type FeeType = "ANNUAL_FEE" | "SUBSCRIPTION";

/**
 * Transaction types for cash in/out.
 */
export type TransactionType = "CASH_IN" | "CASH_OUT";

/**
 * Transaction categories.
 * MEMBERSHIP covers all membership fee types (subscription, annual, application).
 * The specific fee components are stored as boolean flags on the Transaction record.
 * SPONSORSHIP requires sponsorPurpose to be set.
 * Refunds are recorded as CASH_OUT / EXPENSE — no separate REFUND category.
 */
export type TransactionCategory =
  | "MEMBERSHIP"
  | "SPONSORSHIP"
  | "EXPENSE"
  | "OTHER";

/**
 * Payment modes accepted by the system.
 */
export type PaymentMode = "UPI" | "BANK_TRANSFER" | "CASH";

/**
 * Sponsor purpose types for SPONSORSHIP transactions.
 */
export type SponsorPurpose =
  | "TITLE_SPONSOR"
  | "GOLD_SPONSOR"
  | "SILVER_SPONSOR"
  | "FOOD_PARTNER"
  | "MEDIA_PARTNER"
  | "STALL_VENDOR"
  | "MARKETING_PARTNER"
  | "OTHER";

/**
 * Expense purpose types for CASH_OUT transactions (manual cash entries).
 */
export type ExpensePurpose =
  | "DECORATION_PANDAL"
  | "IDOL_MURTI"
  | "LIGHTING_SOUND"
  | "FOOD_BHOG_PRASAD"
  | "PRIEST_PUROHIT"
  | "TRANSPORT_LOGISTICS"
  | "PRINTING_PUBLICITY"
  | "CULTURAL_PROGRAM"
  | "CLEANING_SANITATION"
  | "ELECTRICITY_GENERATOR"
  | "SECURITY"
  | "OTHER";

/**
 * Universal approval status (used for Approval records, Membership status, Transaction approval).
 */
export type ApprovalStatus = "PENDING" | "APPROVED" | "REJECTED";

/**
 * Approval entity types — what the approval is for.
 */
export type ApprovalEntityType =
  | "TRANSACTION"
  | "MEMBER_ADD"
  | "MEMBER_EDIT"
  | "MEMBER_DELETE"
  | "MEMBERSHIP";

/**
 * How a transaction was recorded.
 * MANUAL = operator entered manually. RAZORPAY_WEBHOOK = auto-detected from Razorpay.
 */
export type ApprovalSource = "MANUAL" | "RAZORPAY_WEBHOOK";

// ---------------------------------------------------------------------------
// Business constants
// ---------------------------------------------------------------------------

/** Subscription fee constants (INR) — recurring plan-based fees. */
export const MEMBERSHIP_FEES: Record<MembershipType, number> = {
  MONTHLY: 250,
  HALF_YEARLY: 1500,
  ANNUAL: 3000,
};

/**
 * Annual membership fee (one-time per year, all members).
 */
export const ANNUAL_MEMBERSHIP_FEE = 5000;

/**
 * Application fee (one-time, first membership only).
 */
export const APPLICATION_FEE = 10000;

/**
 * Maximum sub-members per primary member.
 */
export const MAX_SUB_MEMBERS = 3;

/**
 * Days before expiry to send reminder notification.
 */
export const EXPIRY_REMINDER_DAYS = 15;
