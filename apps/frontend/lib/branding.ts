export const OPS_APP_NAME = "Quorum";

function readDataBrandCode(): string {
  const value = process.env.NEXT_PUBLIC_DATA_BRAND_CODE ?? process.env.DATA_BRAND_CODE;
  const normalized = value?.trim().toUpperCase();
  return normalized || "BSDS";
}

export const DATA_BRAND_CODE = readDataBrandCode();
export const UI_DASHBOARD_NAME = `${DATA_BRAND_CODE} Dashboard`;
export const SYSTEM_EMAIL_DOMAIN = `${DATA_BRAND_CODE.toLowerCase()}-dashboard.internal`;
export const SYSTEM_EMAIL = `system@${SYSTEM_EMAIL_DOMAIN}`;
export const SYSTEM_EMAIL_LOOKUP = SYSTEM_EMAIL.toUpperCase();
export const SYSTEM_MEMBER_ID = `${DATA_BRAND_CODE}-SYSTEM-0000-00`;

export function memberIdPrefix(year: number): string {
  return `${DATA_BRAND_CODE}-${year}-`;
}

export function receiptNumberPrefix(year: number): string {
  return `${DATA_BRAND_CODE}-REC-${year}-`;
}

export function buildMemberId(year: number, sequence: number, suffix = 0): string {
  return `${memberIdPrefix(year)}${String(sequence).padStart(4, "0")}-${String(suffix).padStart(2, "0")}`;
}

export function buildMemberOrderReceiptReference(memberId: string, timestamp: number): string {
  return `${DATA_BRAND_CODE}-${memberId.substring(0, 8)}-${timestamp}`.substring(0, 40);
}

export function buildSponsorOrderReceiptReference(token: string, timestamp: number): string {
  return `${DATA_BRAND_CODE}-SP-${token.substring(0, 8)}-${timestamp}`.substring(0, 40);
}

export function buildSponsorPaymentFallbackReceipt(paymentId: string): string {
  return `${DATA_BRAND_CODE}-PAY-${paymentId.substring(4, 12).toUpperCase()}`;
}
