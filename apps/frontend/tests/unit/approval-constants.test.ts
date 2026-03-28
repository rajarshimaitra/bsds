/**
 * Unit tests for app/dashboard/approvals/constants.ts
 *
 * Covers: ENTITY_TYPE_LABELS, STATUS_CLASSES, ENTITY_TYPE_COLORS, ACTION_LABELS,
 *         FIELD_LABELS, ENUM_LABELS, entityApiUrl, formatFieldValue, formatDate.
 * Does NOT cover: React component rendering, network calls, SWR data fetching,
 *                 or the full approvals page UI.
 * Protects: apps/frontend/app/dashboard/approvals/constants.ts
 */

import { describe, it, expect } from "vitest";
import {
  APPROVAL_TYPE_LABELS,
  DIRECTION_LABELS,
  ENTITY_TYPE_LABELS,
  ACTION_LABELS,
  STATUS_CLASSES,
  ENTITY_TYPE_COLORS,
  FIELD_LABELS,
  ENUM_LABELS,
  PLACEHOLDER_ID,
  entityApiUrl,
  formatFieldValue,
  formatDate,
} from "@/app/dashboard/approvals/constants";

describe("APPROVAL_TYPE_LABELS", () => {
  it("covers the streamlined approval buckets", () => {
    expect(APPROVAL_TYPE_LABELS.MEMBERSHIP_APPROVAL).toBe("Membership Approval");
    expect(APPROVAL_TYPE_LABELS.MEMBERSHIP_PAYMENT_APPROVAL).toBe("Membership Payment Approval");
    expect(APPROVAL_TYPE_LABELS.TRANSACTION_APPROVAL).toBe("Transaction Approval");
  });
});

describe("DIRECTION_LABELS", () => {
  it("maps queue direction values to user-facing labels", () => {
    expect(DIRECTION_LABELS.INCOMING).toBe("Incoming");
    expect(DIRECTION_LABELS.OUTGOING).toBe("Outgoing");
  });
});

// ---------------------------------------------------------------------------
// ENTITY_TYPE_LABELS
// ---------------------------------------------------------------------------

describe("ENTITY_TYPE_LABELS", () => {
  it("has a label for TRANSACTION", () => {
    expect(ENTITY_TYPE_LABELS.TRANSACTION).toBeDefined();
    expect(ENTITY_TYPE_LABELS.TRANSACTION.length).toBeGreaterThan(0);
  });

  it("has a label for MEMBER_ADD", () => {
    expect(ENTITY_TYPE_LABELS.MEMBER_ADD).toBeDefined();
    expect(ENTITY_TYPE_LABELS.MEMBER_ADD.length).toBeGreaterThan(0);
  });

  it("has a label for MEMBER_EDIT", () => {
    expect(ENTITY_TYPE_LABELS.MEMBER_EDIT).toBeDefined();
    expect(ENTITY_TYPE_LABELS.MEMBER_EDIT.length).toBeGreaterThan(0);
  });

  it("has a label for MEMBER_DELETE", () => {
    expect(ENTITY_TYPE_LABELS.MEMBER_DELETE).toBeDefined();
    expect(ENTITY_TYPE_LABELS.MEMBER_DELETE.length).toBeGreaterThan(0);
  });

  it("has a label for MEMBERSHIP", () => {
    expect(ENTITY_TYPE_LABELS.MEMBERSHIP).toBeDefined();
    expect(ENTITY_TYPE_LABELS.MEMBERSHIP.length).toBeGreaterThan(0);
  });

  it("covers all 5 entity types that the backend sends", () => {
    const backendTypes = [
      "TRANSACTION",
      "MEMBER_ADD",
      "MEMBER_EDIT",
      "MEMBER_DELETE",
      "MEMBERSHIP",
    ];
    for (const type of backendTypes) {
      expect(ENTITY_TYPE_LABELS[type]).toBeDefined();
    }
    // No extra keys beyond the 5 known types
    expect(Object.keys(ENTITY_TYPE_LABELS)).toHaveLength(5);
  });
});

// ---------------------------------------------------------------------------
// STATUS_CLASSES (approval status colors)
// ---------------------------------------------------------------------------

describe("STATUS_CLASSES", () => {
  it("PENDING has a CSS class string", () => {
    expect(STATUS_CLASSES.PENDING).toBeDefined();
    expect(typeof STATUS_CLASSES.PENDING).toBe("string");
    expect(STATUS_CLASSES.PENDING.length).toBeGreaterThan(0);
  });

  it("APPROVED has a CSS class string", () => {
    expect(STATUS_CLASSES.APPROVED).toBeDefined();
    expect(typeof STATUS_CLASSES.APPROVED).toBe("string");
  });

  it("REJECTED has a CSS class string", () => {
    expect(STATUS_CLASSES.REJECTED).toBeDefined();
    expect(typeof STATUS_CLASSES.REJECTED).toBe("string");
  });

  it("all three approval statuses are covered", () => {
    for (const status of ["PENDING", "APPROVED", "REJECTED"]) {
      expect(STATUS_CLASSES[status]).toBeDefined();
    }
  });
});

// ---------------------------------------------------------------------------
// ENTITY_TYPE_COLORS
// ---------------------------------------------------------------------------

describe("ENTITY_TYPE_COLORS", () => {
  it("all 5 entity types have a color class", () => {
    for (const type of [
      "TRANSACTION",
      "MEMBER_ADD",
      "MEMBER_EDIT",
      "MEMBER_DELETE",
      "MEMBERSHIP",
    ]) {
      expect(ENTITY_TYPE_COLORS[type]).toBeDefined();
      expect(ENTITY_TYPE_COLORS[type].length).toBeGreaterThan(0);
    }
  });
});

// ---------------------------------------------------------------------------
// ACTION_LABELS
// ---------------------------------------------------------------------------

describe("ACTION_LABELS", () => {
  it("add_member has a label", () => {
    expect(ACTION_LABELS.add_member).toBeDefined();
  });

  it("edit_member has a label", () => {
    expect(ACTION_LABELS.edit_member).toBeDefined();
  });

  it("delete_member has a label", () => {
    expect(ACTION_LABELS.delete_member).toBeDefined();
  });

  it("add_transaction has a label", () => {
    expect(ACTION_LABELS.add_transaction).toBeDefined();
  });

  it("approve_transaction has a label", () => {
    expect(ACTION_LABELS.approve_transaction).toBeDefined();
  });

  it("add_membership has a label", () => {
    expect(ACTION_LABELS.add_membership).toBeDefined();
  });

  it("approve_membership has a label", () => {
    expect(ACTION_LABELS.approve_membership).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// FIELD_LABELS
// ---------------------------------------------------------------------------

describe("FIELD_LABELS", () => {
  it("type has a label", () => {
    expect(FIELD_LABELS.type).toBeDefined();
  });

  it("amount has a label", () => {
    expect(FIELD_LABELS.amount).toBeDefined();
  });

  it("paymentMode has a label", () => {
    expect(FIELD_LABELS.paymentMode).toBeDefined();
  });

  it("name has a label", () => {
    expect(FIELD_LABELS.name).toBeDefined();
  });

  it("status has a label", () => {
    expect(FIELD_LABELS.status).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// ENUM_LABELS
// ---------------------------------------------------------------------------

describe("ENUM_LABELS", () => {
  it("approvalStatus.PENDING maps to Pending", () => {
    expect(ENUM_LABELS.approvalStatus?.PENDING).toBe("Pending");
  });

  it("approvalStatus.APPROVED maps to Approved", () => {
    expect(ENUM_LABELS.approvalStatus?.APPROVED).toBe("Approved");
  });

  it("approvalStatus.REJECTED maps to Rejected", () => {
    expect(ENUM_LABELS.approvalStatus?.REJECTED).toBe("Rejected");
  });

  it("type.CASH_IN maps to Cash In", () => {
    expect(ENUM_LABELS.type?.CASH_IN).toBe("Cash In");
  });

  it("type.CASH_OUT maps to Cash Out", () => {
    expect(ENUM_LABELS.type?.CASH_OUT).toBe("Cash Out");
  });

  it("paymentMode.UPI maps to UPI", () => {
    expect(ENUM_LABELS.paymentMode?.UPI).toBe("UPI");
  });

  it("paymentMode.BANK_TRANSFER maps to Bank Transfer", () => {
    expect(ENUM_LABELS.paymentMode?.BANK_TRANSFER).toBe("Bank Transfer");
  });

  it("category.MEMBERSHIP maps to Membership", () => {
    expect(ENUM_LABELS.category?.MEMBERSHIP).toBe("Membership");
  });
});

// ---------------------------------------------------------------------------
// entityApiUrl
// ---------------------------------------------------------------------------

describe("entityApiUrl", () => {
  it("returns null for the placeholder entity ID", () => {
    expect(entityApiUrl("TRANSACTION", PLACEHOLDER_ID, "add_transaction")).toBeNull();
  });

  it("returns /api/transactions/:id for TRANSACTION entity", () => {
    const url = entityApiUrl("TRANSACTION", "tx-123", "add_transaction");
    expect(url).toBe("/api/transactions/tx-123");
  });

  it("returns /api/members/:id for MEMBER_EDIT (non-sub-member)", () => {
    const url = entityApiUrl("MEMBER_EDIT", "mem-1", "edit_member");
    expect(url).toBe("/api/members/mem-1");
  });

  it("returns null for MEMBER_EDIT with action edit_sub_member", () => {
    const url = entityApiUrl("MEMBER_EDIT", "sub-1", "edit_sub_member");
    expect(url).toBeNull();
  });

  it("returns /api/members/:id for MEMBER_DELETE (non-sub-member)", () => {
    const url = entityApiUrl("MEMBER_DELETE", "mem-1", "delete_member");
    expect(url).toBe("/api/members/mem-1");
  });

  it("returns null for MEMBER_DELETE with action remove_sub_member", () => {
    const url = entityApiUrl("MEMBER_DELETE", "sub-1", "remove_sub_member");
    expect(url).toBeNull();
  });

  it("returns /api/members/:id for MEMBERSHIP approve_membership action", () => {
    const url = entityApiUrl("MEMBERSHIP", "mem-1", "approve_membership");
    expect(url).toBe("/api/members/mem-1");
  });

  it("returns /api/memberships/:id for MEMBERSHIP other actions", () => {
    const url = entityApiUrl("MEMBERSHIP", "ms-1", "add_membership");
    expect(url).toBe("/api/memberships/ms-1");
  });

  it("returns null for unknown entity type", () => {
    const url = entityApiUrl("UNKNOWN_TYPE", "id-1", "action");
    expect(url).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// formatFieldValue
// ---------------------------------------------------------------------------

describe("formatFieldValue", () => {
  it("returns '—' for null value", () => {
    expect(formatFieldValue("name", null).text).toBe("—");
  });

  it("returns '—' for undefined value", () => {
    expect(formatFieldValue("name", undefined).text).toBe("—");
  });

  it("returns '—' for empty string value", () => {
    expect(formatFieldValue("name", "").text).toBe("—");
  });

  it("uses ENUM_LABELS for known enum field (approvalStatus: PENDING)", () => {
    expect(formatFieldValue("approvalStatus", "PENDING").text).toBe("Pending");
  });

  it("uses ENUM_LABELS for paymentMode: UPI", () => {
    expect(formatFieldValue("paymentMode", "UPI").text).toBe("UPI");
  });

  it("formats amount field as ₹ currency with Indian locale", () => {
    const { text } = formatFieldValue("amount", "10000");
    expect(text).toContain("10,000");
    expect(text).toMatch(/₹/);
  });

  it("formats amount with CASH_IN type returning emerald class", () => {
    const { text, className } = formatFieldValue("amount", "5000", "CASH_IN");
    expect(text).toContain("5,000");
    expect(className).toContain("emerald");
  });

  it("formats amount with CASH_OUT type returning rose class", () => {
    const { className } = formatFieldValue("amount", "5000", "CASH_OUT");
    expect(className).toContain("rose");
  });

  it("formats startDate field as a date string", () => {
    const { text } = formatFieldValue("startDate", "2026-03-15T00:00:00.000Z");
    // Should be a formatted date (contains digits and separators)
    expect(text).toMatch(/\d/);
  });

  it("formats endDate field as a date string", () => {
    const { text } = formatFieldValue("endDate", "2026-09-15T00:00:00.000Z");
    expect(text).toMatch(/\d/);
  });

  it("formats memberId as truncated mono text", () => {
    const id = "00000000-1111-2222-3333-444444444444";
    const { text, className } = formatFieldValue("memberId", id);
    expect(text).toContain("…");
    expect(className).toContain("mono");
  });

  it("returns plain string for unrecognised field", () => {
    const { text } = formatFieldValue("description", "Some text here");
    expect(text).toBe("Some text here");
  });
});

// ---------------------------------------------------------------------------
// formatDate (the local formatDate from constants, not lib/utils)
// ---------------------------------------------------------------------------

describe("formatDate (approvals constants)", () => {
  it("formats a valid ISO date string and includes numeric parts", () => {
    const result = formatDate("2026-03-15T10:30:00.000Z");
    // en-IN locale produces dd/mm/yyyy HH:MM
    expect(result).toMatch(/\d{2}\/\d{2}\/\d{4}/);
  });
});
