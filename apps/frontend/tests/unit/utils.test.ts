/**
 * Unit tests for lib/utils.ts formatting utilities.
 *
 * Covers: formatCurrency, formatDate, formatDateTime, formatPhone,
 *         formatMemberId, formatSponsorPurpose, formatMembershipType,
 *         formatMembershipStatus — standard values and null/undefined guards.
 * Does NOT cover: cn() class-merging (see utils-extended.test.ts),
 *                 or any React component logic.
 * Protects: apps/frontend/lib/utils.ts
 *
 * Ported from: dps-dashboard/tests/unit/utils.test.ts
 * Changes: Import path unchanged (@/lib/utils). No auth dependency.
 */

import { describe, it, expect } from "vitest";
import {
  formatCurrency,
  formatDate,
  formatDateTime,
  formatPhone,
  formatMemberId,
  formatSponsorPurpose,
  formatMembershipType,
  formatMembershipStatus,
} from "@/lib/utils";

// ---------------------------------------------------------------------------
// formatCurrency
// ---------------------------------------------------------------------------

describe("formatCurrency", () => {
  it("formats a whole number without decimal places", () => {
    const result = formatCurrency(10000);
    expect(result).toContain("10,000");
    expect(result).toMatch(/₹/);
  });

  it("formats a decimal amount as an integer", () => {
    const result = formatCurrency(1500.5);
    expect(result).toContain("1,501");
  });

  it("formats a string amount", () => {
    const result = formatCurrency("3000");
    expect(result).toContain("3,000");
  });

  it("returns ₹0 for NaN string", () => {
    expect(formatCurrency("abc")).toBe("₹0");
  });

  it("formats zero", () => {
    const result = formatCurrency(0);
    expect(result).toContain("0");
  });
});

// ---------------------------------------------------------------------------
// formatDate
// ---------------------------------------------------------------------------

describe("formatDate", () => {
  it("formats a valid ISO date string as DD/MM/YYYY", () => {
    // 2026-03-15 should render in en-IN locale as 15/03/2026
    const result = formatDate("2026-03-15T00:00:00.000Z");
    expect(result).toMatch(/\d{2}\/\d{2}\/\d{4}/);
  });

  it("returns — for null", () => {
    expect(formatDate(null)).toBe("—");
  });

  it("returns — for undefined", () => {
    expect(formatDate(undefined)).toBe("—");
  });

  it("returns — for invalid date string", () => {
    expect(formatDate("not-a-date")).toBe("—");
  });

  it("accepts a Date object", () => {
    const d = new Date(2026, 2, 15); // March 15, 2026 (months are 0-based)
    const result = formatDate(d);
    expect(result).toMatch(/\d{2}\/\d{2}\/2026/);
  });
});

// ---------------------------------------------------------------------------
// formatDateTime
// ---------------------------------------------------------------------------

describe("formatDateTime", () => {
  it("includes date and time parts", () => {
    const result = formatDateTime("2026-03-15T10:30:00.000Z");
    // Date part
    expect(result).toMatch(/\d{2}\/\d{2}\/\d{4}/);
    // Time part — HH:MM
    expect(result).toMatch(/\d{2}:\d{2}/);
  });

  it("returns — for null", () => {
    expect(formatDateTime(null)).toBe("—");
  });

  it("returns — for invalid date", () => {
    expect(formatDateTime("bad")).toBe("—");
  });
});

// ---------------------------------------------------------------------------
// formatPhone
// ---------------------------------------------------------------------------

describe("formatPhone", () => {
  it("passes through +91 numbers unchanged", () => {
    expect(formatPhone("+919876543210")).toBe("+919876543210");
  });

  it("adds + prefix to 91XXXXXXXXXX", () => {
    expect(formatPhone("919876543210")).toBe("+919876543210");
  });

  it("prepends +91 to 10-digit numbers", () => {
    expect(formatPhone("9876543210")).toBe("+919876543210");
  });

  it("returns unknown format unchanged", () => {
    expect(formatPhone("12345")).toBe("12345");
  });

  it("handles empty string", () => {
    expect(formatPhone("")).toBe("");
  });
});

// ---------------------------------------------------------------------------
// formatMemberId
// ---------------------------------------------------------------------------

describe("formatMemberId", () => {
  it("returns the ID unchanged", () => {
    expect(formatMemberId("BSDS-2026-0001-00")).toBe("BSDS-2026-0001-00");
    expect(formatMemberId("BSDS-2026-0001-02")).toBe("BSDS-2026-0001-02");
  });
});

// ---------------------------------------------------------------------------
// formatSponsorPurpose
// ---------------------------------------------------------------------------

describe("formatSponsorPurpose", () => {
  it("converts TITLE_SPONSOR to Title Sponsor", () => {
    expect(formatSponsorPurpose("TITLE_SPONSOR")).toBe("Title Sponsor");
  });

  it("converts FOOD_PARTNER to Food Partner", () => {
    expect(formatSponsorPurpose("FOOD_PARTNER")).toBe("Food Partner");
  });

  it("converts MARKETING_PARTNER to Marketing Partner", () => {
    expect(formatSponsorPurpose("MARKETING_PARTNER")).toBe("Marketing Partner");
  });
});

// ---------------------------------------------------------------------------
// formatMembershipType
// ---------------------------------------------------------------------------

describe("formatMembershipType", () => {
  it("converts MONTHLY to Monthly", () => {
    expect(formatMembershipType("MONTHLY")).toBe("Monthly");
  });

  it("converts HALF_YEARLY to Half-Yearly", () => {
    expect(formatMembershipType("HALF_YEARLY")).toBe("Half-Yearly");
  });

  it("converts ANNUAL to Annual", () => {
    expect(formatMembershipType("ANNUAL")).toBe("Annual");
  });

  it("returns — for null", () => {
    expect(formatMembershipType(null)).toBe("—");
  });

  it("returns — for undefined", () => {
    expect(formatMembershipType(undefined)).toBe("—");
  });
});

// ---------------------------------------------------------------------------
// formatMembershipStatus
// ---------------------------------------------------------------------------

describe("formatMembershipStatus", () => {
  it("converts PENDING_APPROVAL to Pending Approval", () => {
    expect(formatMembershipStatus("PENDING_APPROVAL")).toBe("Pending Approval");
  });

  it("converts ACTIVE to Active", () => {
    expect(formatMembershipStatus("ACTIVE")).toBe("Active");
  });

  it("converts PENDING_PAYMENT to Pending Payment", () => {
    expect(formatMembershipStatus("PENDING_PAYMENT")).toBe("Pending Payment");
  });

  it("converts EXPIRED to Expired", () => {
    expect(formatMembershipStatus("EXPIRED")).toBe("Expired");
  });
});
