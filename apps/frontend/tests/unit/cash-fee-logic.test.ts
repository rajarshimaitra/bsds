/**
 * Unit tests for membership fee constants and fee total calculation logic.
 *
 * Covers: MEMBERSHIP_FEES, APPLICATION_FEE, ANNUAL_MEMBERSHIP_FEE constant
 *         values; fee total composition for all membership types with and
 *         without optional fee components; formatCurrency output for each
 *         fee amount.
 * Does NOT cover: Razorpay integration, payment processing, API endpoints,
 *                 or React component rendering.
 * Protects: apps/frontend/types/index.ts (fee constants)
 *           apps/frontend/lib/utils.ts (formatCurrency)
 */

import { describe, it, expect } from "vitest";
import {
  MEMBERSHIP_FEES,
  APPLICATION_FEE,
  ANNUAL_MEMBERSHIP_FEE,
} from "@/types/index";
import { formatCurrency } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Constant values — pin the business-logic numbers so any accidental change
// breaks tests rather than silently mispricing member invoices.
// ---------------------------------------------------------------------------

describe("MEMBERSHIP_FEES — subscription amounts", () => {
  it("MONTHLY fee is ₹250", () => {
    expect(MEMBERSHIP_FEES.MONTHLY).toBe(250);
  });

  it("HALF_YEARLY fee is ₹1500", () => {
    expect(MEMBERSHIP_FEES.HALF_YEARLY).toBe(1500);
  });

  it("ANNUAL fee is ₹3000", () => {
    expect(MEMBERSHIP_FEES.ANNUAL).toBe(3000);
  });

  it("all three membership types have a defined fee", () => {
    for (const type of ["MONTHLY", "HALF_YEARLY", "ANNUAL"] as const) {
      expect(typeof MEMBERSHIP_FEES[type]).toBe("number");
      expect(MEMBERSHIP_FEES[type]).toBeGreaterThan(0);
    }
  });
});

describe("APPLICATION_FEE", () => {
  it("application fee is ₹10000", () => {
    expect(APPLICATION_FEE).toBe(10000);
  });
});

describe("ANNUAL_MEMBERSHIP_FEE", () => {
  it("annual membership fee is ₹5000", () => {
    expect(ANNUAL_MEMBERSHIP_FEE).toBe(5000);
  });
});

// ---------------------------------------------------------------------------
// Fee total calculation — tests the pure arithmetic that the cash page uses
// when building a membership payment breakdown.
// ---------------------------------------------------------------------------

describe("fee total — subscription only", () => {
  it("MONTHLY only: total equals MEMBERSHIP_FEES.MONTHLY (₹250)", () => {
    const total = MEMBERSHIP_FEES.MONTHLY;
    expect(total).toBe(250);
  });

  it("HALF_YEARLY only: total equals MEMBERSHIP_FEES.HALF_YEARLY (₹1500)", () => {
    const total = MEMBERSHIP_FEES.HALF_YEARLY;
    expect(total).toBe(1500);
  });

  it("ANNUAL only: total equals MEMBERSHIP_FEES.ANNUAL (₹3000)", () => {
    const total = MEMBERSHIP_FEES.ANNUAL;
    expect(total).toBe(3000);
  });
});

describe("fee total — subscription + application fee", () => {
  it("MONTHLY + application fee: total is ₹10250", () => {
    const total = MEMBERSHIP_FEES.MONTHLY + APPLICATION_FEE;
    expect(total).toBe(10250);
  });

  it("HALF_YEARLY + application fee: total is ₹11500", () => {
    const total = MEMBERSHIP_FEES.HALF_YEARLY + APPLICATION_FEE;
    expect(total).toBe(11500);
  });

  it("ANNUAL + application fee: total is ₹13000", () => {
    const total = MEMBERSHIP_FEES.ANNUAL + APPLICATION_FEE;
    expect(total).toBe(13000);
  });
});

describe("fee total — subscription + annual membership fee", () => {
  it("MONTHLY + annual membership fee: total is ₹5250", () => {
    const total = MEMBERSHIP_FEES.MONTHLY + ANNUAL_MEMBERSHIP_FEE;
    expect(total).toBe(5250);
  });

  it("ANNUAL + annual membership fee: total is ₹8000", () => {
    const total = MEMBERSHIP_FEES.ANNUAL + ANNUAL_MEMBERSHIP_FEE;
    expect(total).toBe(8000);
  });
});

describe("fee total — all three fee components", () => {
  it("ANNUAL + annual membership fee + application fee: total is ₹18000", () => {
    const total = MEMBERSHIP_FEES.ANNUAL + ANNUAL_MEMBERSHIP_FEE + APPLICATION_FEE;
    expect(total).toBe(18000);
  });

  it("MONTHLY + annual membership fee + application fee: total is ₹15250", () => {
    const total = MEMBERSHIP_FEES.MONTHLY + ANNUAL_MEMBERSHIP_FEE + APPLICATION_FEE;
    expect(total).toBe(15250);
  });
});

// ---------------------------------------------------------------------------
// formatCurrency applied to fee amounts — ensures the display layer formats
// each business amount correctly.
// ---------------------------------------------------------------------------

describe("formatCurrency — business fee amounts", () => {
  it("formats MONTHLY fee (₹250)", () => {
    expect(formatCurrency(MEMBERSHIP_FEES.MONTHLY)).toContain("250");
    expect(formatCurrency(MEMBERSHIP_FEES.MONTHLY)).toMatch(/₹/);
  });

  it("formats HALF_YEARLY fee (₹1500) with thousands separator", () => {
    expect(formatCurrency(MEMBERSHIP_FEES.HALF_YEARLY)).toContain("1,500");
  });

  it("formats ANNUAL fee (₹3000) with thousands separator", () => {
    expect(formatCurrency(MEMBERSHIP_FEES.ANNUAL)).toContain("3,000");
  });

  it("formats APPLICATION_FEE (₹10000) with thousands separator", () => {
    expect(formatCurrency(APPLICATION_FEE)).toContain("10,000");
  });

  it("formats ANNUAL_MEMBERSHIP_FEE (₹5000) with thousands separator", () => {
    expect(formatCurrency(ANNUAL_MEMBERSHIP_FEE)).toContain("5,000");
  });
});
