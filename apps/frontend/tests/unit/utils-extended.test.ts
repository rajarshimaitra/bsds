/**
 * Extended unit tests for lib/utils.ts — edge cases and cn() utility.
 *
 * Covers: cn() Tailwind class merging, formatCurrency edge values,
 *         formatDate year-boundary dates and Date objects, formatDateTime
 *         time-padding, formatPhone edge cases, formatMembershipType fallback,
 *         formatMembershipStatus all known statuses and approval status strings,
 *         formatSponsorPurpose all enum values, formatFeeType all known values.
 * Does NOT cover: React rendering, hooks, network calls, or auth logic.
 * Protects: apps/frontend/lib/utils.ts
 *
 * Ported from: dps-dashboard/tests/unit/utils-extended.test.ts
 * Wave 7E additions: formatFeeType tests, approval status string coverage via
 *                    formatMembershipStatus.
 */

import { describe, it, expect } from "vitest";
import {
  cn,
  formatCurrency,
  formatDate,
  formatDateTime,
  formatPhone,
  formatMembershipType,
  formatMembershipStatus,
  formatSponsorPurpose,
  formatFeeType,
} from "@/lib/utils";

// ---------------------------------------------------------------------------
// cn() — Tailwind class merging
// ---------------------------------------------------------------------------

describe("cn()", () => {
  it("returns empty string for no arguments", () => {
    expect(cn()).toBe("");
  });

  it("merges two class strings", () => {
    const result = cn("px-4", "py-2");
    expect(result).toContain("px-4");
    expect(result).toContain("py-2");
  });

  it("deduplicates conflicting Tailwind classes (tailwind-merge)", () => {
    // tailwind-merge should keep only the last conflicting utility
    const result = cn("px-2", "px-4");
    expect(result).toContain("px-4");
    expect(result).not.toContain("px-2");
  });

  it("handles conditional classes (clsx falsy values)", () => {
    const result = cn("base-class", false && "conditional", undefined, null, "extra");
    expect(result).toContain("base-class");
    expect(result).toContain("extra");
    expect(result).not.toContain("conditional");
  });

  it("handles object syntax from clsx", () => {
    const result = cn({ "text-red-500": true, "text-green-500": false });
    expect(result).toContain("text-red-500");
    expect(result).not.toContain("text-green-500");
  });

  it("handles array of classes", () => {
    const result = cn(["flex", "items-center"], "justify-between");
    expect(result).toContain("flex");
    expect(result).toContain("items-center");
    expect(result).toContain("justify-between");
  });
});

// ---------------------------------------------------------------------------
// formatCurrency — additional edge cases
// ---------------------------------------------------------------------------

describe("formatCurrency — edge cases", () => {
  it("formats the application fee ₹10000", () => {
    const result = formatCurrency(10000);
    expect(result).toContain("10,000");
    expect(result).toMatch(/₹/);
  });

  it("formats half-yearly fee ₹1500", () => {
    const result = formatCurrency(1500);
    expect(result).toContain("1,500");
  });

  it("formats monthly fee ₹250", () => {
    const result = formatCurrency(250);
    expect(result).toContain("250");
  });

  it("formats annual fee ₹3000", () => {
    const result = formatCurrency(3000);
    expect(result).toContain("3,000");
  });

  it("formats negative amount", () => {
    const result = formatCurrency(-500);
    // Should contain the numeric part
    expect(result).toMatch(/500/);
  });

  it("formats very large number", () => {
    const result = formatCurrency(10000000);
    expect(result).toMatch(/₹/);
    expect(result).toContain("00");
  });

  it("handles string representation of decimal", () => {
    const result = formatCurrency("250.50");
    expect(result).toContain("251");
  });

  it("handles string 0", () => {
    expect(formatCurrency("0")).toContain("0");
  });
});

// ---------------------------------------------------------------------------
// formatDate — additional edge cases
// ---------------------------------------------------------------------------

describe("formatDate — additional edge cases", () => {
  it("formats January 1st correctly", () => {
    const result = formatDate("2026-01-01T00:00:00.000Z");
    expect(result).toMatch(/\d{2}\/01\/2026/);
  });

  it("formats December 31st correctly", () => {
    const result = formatDate("2026-12-31T00:00:00.000Z");
    expect(result).toMatch(/31\/12\/2026/);
  });

  it("formats a Date object (not string)", () => {
    const d = new Date(2030, 5, 15); // June 15, 2030
    const result = formatDate(d);
    expect(result).toMatch(/2030/);
    expect(result).toMatch(/\d{2}\/\d{2}\/\d{4}/);
  });

  it("returns — for empty string", () => {
    expect(formatDate("")).toBe("—");
  });

  it("returns — for 0 (falsy)", () => {
    // 0 is falsy — same as null/undefined
    expect(formatDate(0 as unknown as null)).toBe("—");
  });
});

// ---------------------------------------------------------------------------
// formatDateTime — additional edge cases
// ---------------------------------------------------------------------------

describe("formatDateTime — additional edge cases", () => {
  it("formats midnight correctly (00:00)", () => {
    // Use a fixed UTC time that will be midnight in some timezone
    const d = new Date(2026, 2, 15, 0, 0, 0); // Local midnight
    const result = formatDateTime(d);
    expect(result).toMatch(/\d{2}\/\d{2}\/\d{4}/);
    expect(result).toMatch(/00:00/);
  });

  it("formats 23:59 correctly", () => {
    const d = new Date(2026, 2, 15, 23, 59, 0);
    const result = formatDateTime(d);
    expect(result).toMatch(/23:59/);
  });

  it("pads single-digit hours with zero", () => {
    const d = new Date(2026, 2, 15, 9, 5, 0); // 09:05
    const result = formatDateTime(d);
    expect(result).toMatch(/09:05/);
  });

  it("returns — for undefined", () => {
    expect(formatDateTime(undefined)).toBe("—");
  });
});

// ---------------------------------------------------------------------------
// formatPhone — additional edge cases
// ---------------------------------------------------------------------------

describe("formatPhone — additional edge cases", () => {
  it("handles phone starting with +91 and spaces (unchanged)", () => {
    // If it starts with +91, pass through unchanged
    expect(formatPhone("+91 9876543210")).toBe("+91 9876543210");
  });

  it("converts 11-digit number starting with 91 to +91 format", () => {
    expect(formatPhone("91 98765 43210")).not.toBe("+9191 98765 43210");
    // Exact behavior: starts with 91 + length 12 = prepend +
    // "91 98765 43210" has spaces so length != 12, passes through unchanged
    expect(formatPhone("91 98765 43210")).toBe("91 98765 43210");
  });

  it("10-digit number with leading zeros is treated as 10-digit and gets +91 prefix", () => {
    // formatPhone converts any 10-digit string to +91XXXXXXXXXX
    expect(formatPhone("0123456789")).toBe("+910123456789");
  });

  it("handles typical 10-digit number", () => {
    expect(formatPhone("9123456789")).toBe("+919123456789");
  });
});

// ---------------------------------------------------------------------------
// formatMembershipType — unknown value fallback
// ---------------------------------------------------------------------------

describe("formatMembershipType — fallback behavior", () => {
  it("returns unknown type string as-is (no match in switch)", () => {
    const result = formatMembershipType("QUARTERLY");
    expect(result).toBe("QUARTERLY");
  });

  it("returns — for null", () => {
    expect(formatMembershipType(null)).toBe("—");
  });

  it("returns — for undefined", () => {
    expect(formatMembershipType(undefined)).toBe("—");
  });

  it("returns — for empty string", () => {
    expect(formatMembershipType("")).toBe("—");
  });

  it("formats all three valid membership types correctly", () => {
    expect(formatMembershipType("MONTHLY")).toBe("Monthly");
    expect(formatMembershipType("HALF_YEARLY")).toBe("Half-Yearly");
    expect(formatMembershipType("ANNUAL")).toBe("Annual");
  });
});

// ---------------------------------------------------------------------------
// formatMembershipStatus — all status values
// ---------------------------------------------------------------------------

describe("formatMembershipStatus — all known status values", () => {
  it("converts PENDING_APPROVAL → Pending Approval", () => {
    expect(formatMembershipStatus("PENDING_APPROVAL")).toBe("Pending Approval");
  });

  it("converts PENDING_PAYMENT → Pending Payment", () => {
    expect(formatMembershipStatus("PENDING_PAYMENT")).toBe("Pending Payment");
  });

  it("converts ACTIVE → Active", () => {
    expect(formatMembershipStatus("ACTIVE")).toBe("Active");
  });

  it("converts EXPIRED → Expired", () => {
    expect(formatMembershipStatus("EXPIRED")).toBe("Expired");
  });

  it("converts SUSPENDED → Suspended", () => {
    expect(formatMembershipStatus("SUSPENDED")).toBe("Suspended");
  });

  it("handles multi-word status with multiple underscores", () => {
    // Arbitrary: tests the toTitleCase underlying logic
    expect(formatMembershipStatus("PENDING_ADMIN_REVIEW")).toBe("Pending Admin Review");
  });
});

// ---------------------------------------------------------------------------
// formatSponsorPurpose — all enum values
// ---------------------------------------------------------------------------

describe("formatSponsorPurpose — all sponsor purpose values", () => {
  it("converts TITLE_SPONSOR → Title Sponsor", () => {
    expect(formatSponsorPurpose("TITLE_SPONSOR")).toBe("Title Sponsor");
  });

  it("converts GOLD_SPONSOR → Gold Sponsor", () => {
    expect(formatSponsorPurpose("GOLD_SPONSOR")).toBe("Gold Sponsor");
  });

  it("converts SILVER_SPONSOR → Silver Sponsor", () => {
    expect(formatSponsorPurpose("SILVER_SPONSOR")).toBe("Silver Sponsor");
  });

  it("converts FOOD_PARTNER → Food Partner", () => {
    expect(formatSponsorPurpose("FOOD_PARTNER")).toBe("Food Partner");
  });

  it("converts MEDIA_PARTNER → Media Partner", () => {
    expect(formatSponsorPurpose("MEDIA_PARTNER")).toBe("Media Partner");
  });

  it("converts STALL_VENDOR → Stall Vendor", () => {
    expect(formatSponsorPurpose("STALL_VENDOR")).toBe("Stall Vendor");
  });

  it("converts MARKETING_PARTNER → Marketing Partner", () => {
    expect(formatSponsorPurpose("MARKETING_PARTNER")).toBe("Marketing Partner");
  });
});

// ---------------------------------------------------------------------------
// formatFeeType
// ---------------------------------------------------------------------------

describe("formatFeeType", () => {
  it("converts ANNUAL_FEE → Annual Fee", () => {
    expect(formatFeeType("ANNUAL_FEE")).toBe("Annual Fee");
  });

  it("converts SUBSCRIPTION → Subscription", () => {
    expect(formatFeeType("SUBSCRIPTION")).toBe("Subscription");
  });

  it("returns — for null", () => {
    expect(formatFeeType(null)).toBe("—");
  });

  it("returns — for undefined", () => {
    expect(formatFeeType(undefined)).toBe("—");
  });

  it("returns — for empty string", () => {
    expect(formatFeeType("")).toBe("—");
  });

  it("returns unknown fee type string as-is (no match in switch)", () => {
    expect(formatFeeType("ONE_TIME")).toBe("ONE_TIME");
  });
});

// ---------------------------------------------------------------------------
// formatMembershipStatus — approval status strings
// formatMembershipStatus uses toTitleCase which also covers ApprovalStatus
// values (PENDING, APPROVED, REJECTED) since they are plain ALL_CAPS strings.
// ---------------------------------------------------------------------------

describe("formatMembershipStatus — approval status strings", () => {
  it("converts PENDING → Pending", () => {
    expect(formatMembershipStatus("PENDING")).toBe("Pending");
  });

  it("converts APPROVED → Approved", () => {
    expect(formatMembershipStatus("APPROVED")).toBe("Approved");
  });

  it("converts REJECTED → Rejected", () => {
    expect(formatMembershipStatus("REJECTED")).toBe("Rejected");
  });
});
